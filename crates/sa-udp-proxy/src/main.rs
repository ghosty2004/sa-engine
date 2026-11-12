//! `sa-udp-proxy`: WebSocket ↔ UDP relay for the browser build (docs/samp/web-proxy.md).

#![forbid(unsafe_code)]

use std::time::Duration;

use sa_udp_proxy::{Config, DEFAULT_BIND, LogLevel, Proxy, Target, shim};

const USAGE: &str = "\
usage: sa-udp-proxy [OPTIONS] [TARGET]
       sa-udp-proxy shim --listen UDP_ADDR --proxy WS_URL --target HOST:PORT

Relays WebSocket binary frames to UDP datagrams and back. Each WebSocket connection is one UDP socket.
The client's first frame must be the text frame {\"connect\":\"host:port\"}.

TARGET (host:port) is the server the proxy relays to; it is the only allowed target unless --allow /
--allow-any add more.

options:
  --bind ADDR          WebSocket listen address (default 127.0.0.1:7780)
  --allow HOST:PORT    also allow this target (repeatable)
  --allow-any          allow every target (open relay: only on trusted networks)
  --origin ORIGIN      allowed browser Origin header (repeatable; default: any)
  --max-datagram N     largest datagram in bytes (default 1500)
  --max-pps N          datagrams per second per direction (default 400)
  --max-bps N          bytes per second per direction (default 524288)
  --idle-timeout SECS  close a connection after SECS without datagrams (default 60)
  --max-conns N        open connections in total (default 256)
  --max-per-ip N       open connections per client IP (default 16)
  -q, --quiet          log errors and refusals only
  -v, --verbose        log every datagram
  -h, --help

shim mode (testing): listens on UDP_ADDR and carries each UDP client's datagrams over its own WebSocket
connection to the proxy at WS_URL, for TARGET. SA:MP keys its cipher on the server port, so listen on the
server's port number on another loopback address (e.g. 127.0.0.2:7777).";

fn fail(msg: &str) -> ! {
    eprintln!("sa-udp-proxy: {msg}\n\n{USAGE}");
    std::process::exit(2);
}

fn value<T: std::str::FromStr>(args: &mut impl Iterator<Item = String>, flag: &str) -> T {
    let v = args
        .next()
        .unwrap_or_else(|| fail(&format!("{flag} needs a value")));
    v.parse()
        .unwrap_or_else(|_| fail(&format!("{flag}: bad value {v:?}")))
}

fn main() {
    let mut args = std::env::args().skip(1).peekable();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build();
    let rt = rt.unwrap_or_else(|e| fail(&format!("runtime: {e}")));
    if args.peek().map(String::as_str) == Some("shim") {
        args.next();
        let (mut listen, mut url, mut target) = (None::<String>, None, None);
        while let Some(a) = args.next() {
            match a.as_str() {
                "--listen" => listen = Some(value(&mut args, "--listen")),
                "--proxy" => url = Some(value(&mut args, "--proxy")),
                "--target" => target = Some(value(&mut args, "--target")),
                "-h" | "--help" => {
                    println!("{USAGE}");
                    return;
                }
                _ => fail(&format!("unknown shim argument {a:?}")),
            }
        }
        let listen = listen.unwrap_or_else(|| fail("shim needs --listen"));
        let config = shim::ShimConfig {
            proxy_url: url.unwrap_or_else(|| format!("ws://{DEFAULT_BIND}")),
            target: target.unwrap_or_else(|| fail("shim needs --target")),
            idle_timeout: Duration::from_secs(60),
        };
        let r: std::io::Result<()> = rt.block_on(async {
            let udp = tokio::net::UdpSocket::bind(&listen).await?;
            eprintln!(
                "shim: udp {} -> {} via {}",
                udp.local_addr()?,
                config.target,
                config.proxy_url
            );
            shim::run(udp, config).await
        });
        if let Err(e) = r {
            fail(&format!("shim: {e}"));
        }
        return;
    }

    let mut config = Config::default();
    let mut bind = DEFAULT_BIND.to_string();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--bind" => bind = value(&mut args, "--bind"),
            "--allow" => {
                let s: String = value(&mut args, "--allow");
                config.allow.push(Target::parse(&s).unwrap_or_else(|e| fail(&e)));
            }
            "--allow-any" => config.allow_any = true,
            "--origin" => config.origins.push(value(&mut args, "--origin")),
            "--max-datagram" => config.max_datagram = value(&mut args, "--max-datagram"),
            "--max-pps" => config.max_packets_per_sec = value(&mut args, "--max-pps"),
            "--max-bps" => config.max_bytes_per_sec = value(&mut args, "--max-bps"),
            "--idle-timeout" => config.idle_timeout = Duration::from_secs(value(&mut args, "--idle-timeout")),
            "--max-conns" => config.max_connections = value(&mut args, "--max-conns"),
            "--max-per-ip" => config.max_per_ip = value(&mut args, "--max-per-ip"),
            "-q" | "--quiet" => config.log = LogLevel::Quiet,
            "-v" | "--verbose" => config.log = LogLevel::Verbose,
            "-h" | "--help" => {
                println!("{USAGE}");
                return;
            }
            s if s.starts_with('-') => fail(&format!("unknown option {s:?}")),
            s => config
                .allow
                .insert(0, Target::parse(s).unwrap_or_else(|e| fail(&e))),
        }
    }
    if config.allow.is_empty() && !config.allow_any {
        fail("give a TARGET, --allow HOST:PORT or --allow-any");
    }
    if config.max_datagram == 0 || config.max_packets_per_sec == 0 || config.max_bytes_per_sec == 0 {
        fail("limits must be positive");
    }
    let r: std::io::Result<()> = rt.block_on(async {
        let listener = tokio::net::TcpListener::bind(&bind).await?;
        let proxy = Proxy::new(config);
        tokio::select! {
            r = proxy.serve(listener) => r,
            _ = tokio::signal::ctrl_c() => {
                eprintln!("sa-udp-proxy: interrupted");
                Ok(())
            }
        }
    });
    if let Err(e) = r {
        fail(&format!("{bind}: {e}"));
    }
}
