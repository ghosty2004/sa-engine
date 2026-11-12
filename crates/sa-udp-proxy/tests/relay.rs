//! Host integration tests: the proxy between a WebSocket client and a UDP echo server on loopback.

use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use sa_net::{DatagramTransport, LinkState, NativeWsTransport};
use sa_udp_proxy::{Config, LogLevel, Proxy, Target, connect_frame, shim};
use tokio::net::{TcpListener, UdpSocket};
use tokio_tungstenite::tungstenite::Message;

type Ws = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// A UDP echo server; records every source address it saw.
async fn echo_server() -> (SocketAddr, Arc<Mutex<HashSet<SocketAddr>>>) {
    let sock = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let addr = sock.local_addr().unwrap();
    let seen = Arc::new(Mutex::new(HashSet::new()));
    let s = Arc::clone(&seen);
    tokio::spawn(async move {
        let mut buf = vec![0u8; 65536];
        while let Ok((n, from)) = sock.recv_from(&mut buf).await {
            s.lock().unwrap().insert(from);
            let _ = sock.send_to(&buf[..n], from).await;
        }
    });
    (addr, seen)
}

async fn start_proxy(config: Config) -> (String, Arc<Proxy>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let proxy = Proxy::new(config);
    tokio::spawn(Arc::clone(&proxy).serve(listener));
    (url, proxy)
}

fn config_for(target: SocketAddr) -> Config {
    Config {
        allow: vec![Target::parse(&target.to_string()).unwrap()],
        log: LogLevel::Normal,
        ..Config::default()
    }
}

async fn ws_connect(url: &str, target: &str) -> (Ws, String) {
    let (mut ws, _) = tokio_tungstenite::connect_async(url).await.unwrap();
    ws.send(Message::text(connect_frame(target))).await.unwrap();
    let reply = next_text(&mut ws).await;
    (ws, reply)
}

async fn next_text(ws: &mut Ws) -> String {
    loop {
        match tokio::time::timeout(Duration::from_secs(3), ws.next())
            .await
            .expect("reply in time")
        {
            Some(Ok(Message::Text(t))) => return t.to_string(),
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
            other => panic!("expected a text frame, got {other:?}"),
        }
    }
}

async fn next_binary(ws: &mut Ws, wait: Duration) -> Option<Vec<u8>> {
    loop {
        match tokio::time::timeout(wait, ws.next()).await {
            Ok(Some(Ok(Message::Binary(b)))) => return Some(b.to_vec()),
            Ok(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => continue,
            _ => return None,
        }
    }
}

/// Waits for the end of the stream (close frame or EOF); true if it ended within `wait`.
async fn closes(ws: &mut Ws, wait: Duration) -> bool {
    let end = tokio::time::Instant::now() + wait;
    loop {
        match tokio::time::timeout_at(end, ws.next()).await {
            Ok(None | Some(Ok(Message::Close(_))) | Some(Err(_))) => return true,
            Ok(Some(Ok(_))) => continue,
            Err(_) => return false,
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn round_trips_datagrams() {
    let (echo, seen) = echo_server().await;
    let (url, proxy) = start_proxy(config_for(echo)).await;
    let (mut ws, reply) = ws_connect(&url, &echo.to_string()).await;
    assert_eq!(reply, format!("{{\"connected\":\"{echo}\"}}"));
    // RakNet sizes: open request (3), cookie (3), typical sync (~70), MTU (576), the proxy's maximum (1500).
    let sizes = [1usize, 3, 70, 576, 1500];
    for (i, &n) in sizes.iter().cycle().take(100).enumerate() {
        let dg: Vec<u8> = (0..n).map(|j| (i * 7 + j) as u8).collect();
        ws.send(Message::binary(dg.clone())).await.unwrap();
        assert_eq!(
            next_binary(&mut ws, Duration::from_secs(2)).await.as_deref(),
            Some(&dg[..]),
            "datagram {i}"
        );
    }
    // A second connection gets its own UDP socket (its own source port at the server).
    let (mut ws2, _) = ws_connect(&url, &echo.to_string()).await;
    ws2.send(Message::binary(vec![9u8; 10])).await.unwrap();
    assert_eq!(
        next_binary(&mut ws2, Duration::from_secs(2)).await,
        Some(vec![9u8; 10])
    );
    assert_eq!(
        seen.lock().unwrap().len(),
        2,
        "one source address per WebSocket connection"
    );
    ws.close(None).await.unwrap();
    ws2.close(None).await.unwrap();
    assert_eq!(proxy.stats.relayed.load(std::sync::atomic::Ordering::Relaxed), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refuses_targets_off_the_allowlist() {
    let (echo, seen) = echo_server().await;
    let (other, _) = echo_server().await;
    let (url, proxy) = start_proxy(config_for(echo)).await;

    // Another port on the same host.
    let (mut ws, reply) = ws_connect(&url, &other.to_string()).await;
    assert!(reply.starts_with("{\"error\":\"target not allowed"), "{reply}");
    assert!(closes(&mut ws, Duration::from_secs(2)).await);

    // The allowed target by host name resolves to the allowed address.
    let (mut ok, reply) = ws_connect(&url, &format!("localhost:{}", echo.port())).await;
    assert!(reply.starts_with("{\"connected\""), "{reply}");
    ok.send(Message::binary(vec![1, 2, 3])).await.unwrap();
    assert_eq!(
        next_binary(&mut ok, Duration::from_secs(2)).await,
        Some(vec![1, 2, 3])
    );

    // Malformed control frames.
    for bad in [
        "hello",
        "{\"connect\":5}",
        "{\"connect\":\"nohost\"}",
        "{\"connect\":\"0.0.0.0:7777\"}",
    ] {
        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
        ws.send(Message::text(bad)).await.unwrap();
        let reply = next_text(&mut ws).await;
        assert!(reply.starts_with("{\"error\""), "{bad}: {reply}");
        assert!(closes(&mut ws, Duration::from_secs(2)).await);
    }
    // A binary frame before the control frame.
    let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
    ws.send(Message::binary(vec![0x18, 0x69, 0x69])).await.unwrap();
    assert!(next_text(&mut ws).await.starts_with("{\"error\""));

    // allow_any lifts the allowlist (but never for unspecified/broadcast/multicast addresses).
    let (url_any, _) = start_proxy(Config {
        allow_any: true,
        ..Config::default()
    })
    .await;
    let (mut any, reply) = ws_connect(&url_any, &other.to_string()).await;
    assert!(reply.starts_with("{\"connected\""), "{reply}");
    any.send(Message::binary(vec![4])).await.unwrap();
    assert_eq!(next_binary(&mut any, Duration::from_secs(2)).await, Some(vec![4]));
    let (_, reply) = ws_connect(&url_any, "255.255.255.255:7777").await;
    assert!(reply.starts_with("{\"error\""), "{reply}");

    assert_eq!(
        seen.lock().unwrap().len(),
        1,
        "refused connections sent nothing to the server"
    );
    assert!(proxy.stats.refused.load(std::sync::atomic::Ordering::Relaxed) >= 6);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enforces_size_rate_and_idle_limits() {
    let (echo, _) = echo_server().await;
    let config = Config {
        max_datagram: 600,
        max_packets_per_sec: 20,
        idle_timeout: Duration::from_millis(700),
        ..config_for(echo)
    };
    let (url, _) = start_proxy(config).await;
    let (mut ws, _) = ws_connect(&url, &echo.to_string()).await;

    // Oversize: dropped, the connection stays.
    ws.send(Message::binary(vec![0u8; 601])).await.unwrap();
    assert_eq!(next_binary(&mut ws, Duration::from_millis(300)).await, None);
    ws.send(Message::binary(vec![5u8; 600])).await.unwrap();
    assert_eq!(
        next_binary(&mut ws, Duration::from_secs(2)).await,
        Some(vec![5u8; 600])
    );

    // Rate: a burst of 100 within a few ms gets about one second of budget (20) through.
    for i in 0..100u8 {
        ws.send(Message::binary(vec![i])).await.unwrap();
    }
    let mut got = 0;
    while next_binary(&mut ws, Duration::from_millis(300)).await.is_some() {
        got += 1;
    }
    assert!((15..=22).contains(&got), "{got} of 100 passed a 20/s limit");

    // Idle: silence closes the connection.
    let t = Instant::now();
    assert!(closes(&mut ws, Duration::from_secs(3)).await);
    assert!(
        t.elapsed() >= Duration::from_millis(300),
        "closed too early: {:?}",
        t.elapsed()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enforces_connection_and_origin_limits() {
    let (echo, _) = echo_server().await;
    let (url, _) = start_proxy(Config {
        max_per_ip: 1,
        ..config_for(echo)
    })
    .await;
    let (_first, reply) = ws_connect(&url, &echo.to_string()).await;
    assert!(reply.starts_with("{\"connected\""));
    assert!(
        tokio_tungstenite::connect_async(&url).await.is_err(),
        "second connection from the same IP"
    );

    let config = Config {
        origins: vec!["https://game.example".into()],
        ..config_for(echo)
    };
    let (url, _) = start_proxy(config).await;
    assert!(
        tokio_tungstenite::connect_async(&url).await.is_err(),
        "no Origin header"
    );
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let mut req = url.as_str().into_client_request().unwrap();
    req.headers_mut()
        .insert("Origin", "https://evil.example".parse().unwrap());
    assert!(
        tokio_tungstenite::connect_async(req).await.is_err(),
        "wrong Origin"
    );
    let mut req = url.as_str().into_client_request().unwrap();
    req.headers_mut()
        .insert("Origin", "https://game.example".parse().unwrap());
    assert!(
        tokio_tungstenite::connect_async(req).await.is_ok(),
        "allowed Origin"
    );
}

/// The client transport trait over the proxy: what samp-rs's RakNet update would do.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sa_net_transport_round_trips_through_the_proxy() {
    let (echo, _) = echo_server().await;
    let (url, _) = start_proxy(config_for(echo)).await;
    let refused = {
        let url = url.clone();
        tokio::task::spawn_blocking(move || {
            NativeWsTransport::open(&url, "127.0.0.1:1", Duration::from_secs(3))
        })
        .await
        .unwrap()
    };
    let e = refused.unwrap_err();
    assert_eq!(e.kind(), std::io::ErrorKind::ConnectionRefused, "{e}");

    tokio::task::spawn_blocking(move || {
        let mut t = NativeWsTransport::open(&url, &echo.to_string(), Duration::from_secs(3)).unwrap();
        assert_eq!(t.state(), LinkState::Open);
        let mut buf = [0u8; 2048];
        assert_eq!(
            t.recv(&mut buf).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        for i in 0..50u8 {
            let dg = vec![i; 1 + i as usize * 11];
            assert_eq!(t.send(&dg).unwrap(), dg.len());
            let start = Instant::now();
            let n = loop {
                match t.recv(&mut buf) {
                    Ok(n) => break n,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            start.elapsed() < Duration::from_secs(2),
                            "no echo for datagram {i}"
                        );
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Err(e) => panic!("{e}"),
                }
            };
            assert_eq!(&buf[..n], &dg[..]);
        }
    })
    .await
    .unwrap();
}

/// An unmodified UDP client through the shim and the proxy.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shim_carries_a_plain_udp_client() {
    let (echo, seen) = echo_server().await;
    let (url, _) = start_proxy(config_for(echo)).await;
    let shim_sock = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let shim_addr = shim_sock.local_addr().unwrap();
    tokio::spawn(shim::run(
        shim_sock,
        shim::ShimConfig {
            proxy_url: url,
            target: echo.to_string(),
            idle_timeout: Duration::from_secs(5),
        },
    ));
    let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    client.connect(shim_addr).await.unwrap();
    let mut buf = [0u8; 2048];
    // The first datagrams may be dropped while the shim's WebSocket opens (like early UDP loss): retry.
    let mut ok = false;
    for _ in 0..20 {
        client.send(b"\x18\x69\x69").await.unwrap();
        if let Ok(Ok(n)) = tokio::time::timeout(Duration::from_millis(200), client.recv(&mut buf)).await {
            assert_eq!(&buf[..n], b"\x18\x69\x69");
            ok = true;
            break;
        }
    }
    assert!(ok, "no echo through shim + proxy");
    // Echoes of retried requests that were queued while the WebSocket opened.
    while tokio::time::timeout(Duration::from_millis(200), client.recv(&mut buf))
        .await
        .is_ok()
    {}
    for i in 0..20u8 {
        client.send(&[i; 100]).await.unwrap();
        let n = tokio::time::timeout(Duration::from_secs(2), client.recv(&mut buf))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&buf[..n], &[i; 100]);
    }
    assert_eq!(seen.lock().unwrap().len(), 1);
}
