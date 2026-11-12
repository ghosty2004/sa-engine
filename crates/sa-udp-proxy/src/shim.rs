//! UDP → WebSocket shim: the client half of the proxy for native programs.
//!
//! The shim listens on a local UDP address. Each new UDP source address (one client socket) gets its own
//! WebSocket connection to the proxy, opened with `{"connect":TARGET}`; datagrams then travel as binary frames
//! both ways. This lets an unmodified UDP client (samp-rs's host test, or samp.dll under Wine) exercise the
//! exact path the browser build takes. It is a test and debugging tool; the browser uses
//! `sa_net::WebSocketTransport` instead.
//!
//! SA:MP derives the datagram cipher key from the **server's** port, so the shim should listen on the same
//! port number as the real server (on another loopback address, e.g. 127.0.0.2), or the client must be told
//! the real server port separately.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

/// Shim configuration.
#[derive(Clone, Debug)]
pub struct ShimConfig {
    /// WebSocket URL of the proxy, e.g. `ws://127.0.0.1:7780`.
    pub proxy_url: String,
    /// Target the proxy should relay to, `host:port`.
    pub target: String,
    /// A client source address silent this long loses its WebSocket.
    pub idle_timeout: Duration,
}

/// Opens a WebSocket to the proxy and sends the control frame; returns the stream once `connected` arrived.
pub async fn open(
    proxy_url: &str,
    target: &str,
) -> Result<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    String,
> {
    let (mut ws, _) = tokio_tungstenite::connect_async(proxy_url)
        .await
        .map_err(|e| format!("connect: {e}"))?;
    ws.send(Message::text(crate::connect_frame(target)))
        .await
        .map_err(|e| format!("send: {e}"))?;
    loop {
        match ws.next().await {
            Some(Ok(Message::Text(t))) => {
                let v: serde_json::Value = serde_json::from_str(&t).map_err(|_| format!("bad reply {t}"))?;
                if v.get("connected").is_some() {
                    return Ok(ws);
                }
                return Err(v.get("error").and_then(|e| e.as_str()).unwrap_or(&t).to_string());
            }
            Some(Ok(Message::Ping(_) | Message::Pong(_))) => continue,
            Some(Ok(m)) => return Err(format!("unexpected {m:?}")),
            Some(Err(e)) => return Err(format!("websocket: {e}")),
            None => return Err("closed before reply".into()),
        }
    }
}

/// Runs the shim on `udp` forever.
pub async fn run(udp: UdpSocket, config: ShimConfig) -> std::io::Result<()> {
    let udp = Arc::new(udp);
    let mut peers: HashMap<SocketAddr, mpsc::Sender<Vec<u8>>> = HashMap::new();
    let mut buf = vec![0u8; 65536];
    loop {
        let (n, from) = udp.recv_from(&mut buf).await?;
        let dg = buf[..n].to_vec();
        peers.retain(|_, tx| !tx.is_closed());
        let tx = peers.entry(from).or_insert_with(|| {
            let (tx, rx) = mpsc::channel(256);
            tokio::spawn(peer_task(Arc::clone(&udp), from, rx, config.clone()));
            tx
        });
        // Full queue (WebSocket still opening, or stalled): drop like UDP.
        let _ = tx.try_send(dg);
    }
}

async fn peer_task(
    udp: Arc<UdpSocket>,
    from: SocketAddr,
    mut rx: mpsc::Receiver<Vec<u8>>,
    config: ShimConfig,
) {
    let mut ws = match open(&config.proxy_url, &config.target).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("shim: {from}: {e}");
            return;
        }
    };
    eprintln!("shim: {from} -> {} via {}", config.target, config.proxy_url);
    loop {
        tokio::select! {
            dg = rx.recv() => match dg {
                Some(dg) => if ws.send(Message::binary(dg)).await.is_err() { break },
                None => break,
            },
            msg = ws.next() => match msg {
                Some(Ok(Message::Binary(b))) => { let _ = udp.send_to(&b, from).await; }
                Some(Ok(Message::Ping(_) | Message::Pong(_))) => {}
                _ => break,
            },
            _ = tokio::time::sleep(config.idle_timeout) => break,
        }
    }
    eprintln!("shim: {from} closed");
    let _ = ws.close(None).await;
}
