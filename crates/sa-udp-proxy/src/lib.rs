//! WebSocket ↔ UDP relay for the browser build (docs/samp/web-proxy.md).
//!
//! A browser cannot send UDP, so the wasm32 build of sa-game + samp-rs reaches an unmodified SA:MP 0.3.7 /
//! open.mp server through this proxy, which the user runs next to the browser (or on a host they control):
//!
//! * every WebSocket connection owns exactly one UDP socket, connected to one target server, so the server
//!   sees one stable source address per client, as with a native client;
//! * the first message on a connection is a text control frame `{"connect":"host:port"}`; the proxy answers
//!   `{"connected":"ip:port"}` or `{"error":"..."}` (and then closes with code 1008);
//! * after that, every binary frame is one raw UDP payload in each direction (no extra header);
//! * targets are checked against an allowlist (default: only the target given on the command line), so the
//!   proxy cannot be used as an open relay; `allow_any` lifts that;
//! * per-connection limits: datagram size, packets/s and bytes/s in each direction (excess datagrams are
//!   dropped, as UDP would), an idle timeout, a deadline for the control frame, and limits on the number of
//!   connections in total and per client IP; an optional `Origin` allowlist for browsers.
//!
//! [`shim`] is the reverse direction for tests and for unmodified native clients: a local UDP port whose
//! datagrams travel over a WebSocket to this proxy.

#![forbid(unsafe_code)]

pub mod shim;

use std::collections::HashMap;
use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::http::StatusCode;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, WebSocketConfig};

/// Default WebSocket bind address.
pub const DEFAULT_BIND: &str = "127.0.0.1:7780";

/// Logging verbosity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Errors and refused connections only.
    Quiet,
    /// Plus one line per connection open/close with its counters.
    Normal,
    /// Plus one line per dropped or relayed datagram.
    Verbose,
}

/// One allowed target, as given on the command line (`host:port`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    /// Host name or IP literal (IPv6 without brackets).
    pub host: String,
    /// UDP port.
    pub port: u16,
}

impl Target {
    /// Parses `host:port`, `ip:port` or `[ipv6]:port`.
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();
        let (host, port) = s
            .rsplit_once(':')
            .ok_or_else(|| format!("{s:?}: expected host:port"))?;
        let host = host
            .strip_prefix('[')
            .and_then(|h| h.strip_suffix(']'))
            .unwrap_or(host);
        if host.is_empty() || host.contains(char::is_whitespace) {
            return Err(format!("{s:?}: bad host"));
        }
        let port: u16 = port.parse().map_err(|_| format!("{s:?}: bad port"))?;
        if port == 0 {
            return Err(format!("{s:?}: port 0"));
        }
        Ok(Self {
            host: host.to_ascii_lowercase(),
            port,
        })
    }

    fn lookup_string(&self) -> String {
        if self.host.contains(':') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.lookup_string())
    }
}

/// Proxy configuration.
#[derive(Clone, Debug)]
pub struct Config {
    /// Allowed targets (ignored with `allow_any`).
    pub allow: Vec<Target>,
    /// Any target is allowed (still never unspecified, broadcast or multicast addresses).
    pub allow_any: bool,
    /// Allowed browser origins (`Origin` header); empty = any origin, including none.
    pub origins: Vec<String>,
    /// Largest datagram relayed in either direction, in bytes.
    pub max_datagram: usize,
    /// Datagrams per second per direction (token bucket, one second of burst).
    pub max_packets_per_sec: u32,
    /// Bytes per second per direction (token bucket, one second of burst).
    pub max_bytes_per_sec: u32,
    /// A connection with no datagram in either direction for this long is closed.
    pub idle_timeout: Duration,
    /// Deadline for the WebSocket handshake plus the `connect` control frame.
    pub connect_timeout: Duration,
    /// Open connections, in total.
    pub max_connections: usize,
    /// Open connections per client IP.
    pub max_per_ip: usize,
    /// Logging verbosity.
    pub log: LogLevel,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            allow: Vec::new(),
            allow_any: false,
            origins: Vec::new(),
            // RakNet 2.x / SA:MP use an MTU of 576; open.mp allows up to 1492. 1500 covers every real datagram.
            max_datagram: 1500,
            max_packets_per_sec: 400,
            max_bytes_per_sec: 512 * 1024,
            idle_timeout: Duration::from_secs(60),
            connect_timeout: Duration::from_secs(10),
            max_connections: 256,
            max_per_ip: 16,
            log: LogLevel::Normal,
        }
    }
}

/// The control frame a client sends first.
pub fn connect_frame(target: &str) -> String {
    serde_json::json!({ "connect": target }).to_string()
}

/// Why a `connect` request was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The first frame was not `{"connect":"host:port"}`.
    BadRequest(String),
    /// The target is not on the allowlist.
    NotAllowed(String),
    /// The host did not resolve, or only to forbidden addresses.
    Unresolved(String),
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::BadRequest(s) => write!(f, "bad request: {s}"),
            Refusal::NotAllowed(s) => write!(f, "target not allowed: {s}"),
            Refusal::Unresolved(s) => write!(f, "target unresolved: {s}"),
        }
    }
}

/// Addresses the proxy never sends to, whatever the allowlist says.
fn forbidden(addr: &SocketAddr) -> bool {
    let ip = addr.ip();
    addr.port() == 0
        || ip.is_unspecified()
        || ip.is_multicast()
        || matches!(ip, IpAddr::V4(v4) if v4.is_broadcast())
}

async fn resolve(t: &Target) -> Vec<SocketAddr> {
    match tokio::net::lookup_host(t.lookup_string()).await {
        Ok(it) => it.filter(|a| !forbidden(a)).collect(),
        Err(_) => Vec::new(),
    }
}

/// Parses a control frame and checks the target against the configuration; returns the address to use.
pub async fn check_target(config: &Config, frame: &str) -> Result<SocketAddr, Refusal> {
    let v: serde_json::Value =
        serde_json::from_str(frame).map_err(|_| Refusal::BadRequest("not JSON".into()))?;
    let s = v
        .get("connect")
        .and_then(|c| c.as_str())
        .ok_or_else(|| Refusal::BadRequest("expected {\"connect\":\"host:port\"}".into()))?;
    let want = Target::parse(s).map_err(Refusal::BadRequest)?;
    let addrs = resolve(&want).await;
    if addrs.is_empty() {
        return Err(Refusal::Unresolved(want.to_string()));
    }
    if config.allow_any || config.allow.contains(&want) {
        // Prefer IPv4: SA:MP / RakNet 2.x servers are IPv4 only.
        return Ok(*addrs.iter().find(|a| a.is_ipv4()).unwrap_or(&addrs[0]));
    }
    for allowed in &config.allow {
        if allowed.port != want.port {
            continue;
        }
        let ok = resolve(allowed).await;
        if let Some(a) = addrs.iter().find(|a| ok.contains(a)) {
            return Ok(*a);
        }
    }
    Err(Refusal::NotAllowed(want.to_string()))
}

/// A token bucket over packets and bytes, refilled continuously, holding at most one second of budget.
#[derive(Debug)]
pub struct RateLimit {
    pps: f64,
    bps: f64,
    packets: f64,
    bytes: f64,
    last: Instant,
}

impl RateLimit {
    /// A full bucket.
    pub fn new(pps: u32, bps: u32, now: Instant) -> Self {
        Self {
            pps: pps.into(),
            bps: bps.into(),
            packets: pps.into(),
            bytes: bps.into(),
            last: now,
        }
    }

    /// Takes one datagram of `len` bytes if the budget allows.
    pub fn allow(&mut self, len: usize, now: Instant) -> bool {
        let dt = now.saturating_duration_since(self.last).as_secs_f64();
        self.last = now;
        self.packets = (self.packets + dt * self.pps).min(self.pps);
        self.bytes = (self.bytes + dt * self.bps).min(self.bps);
        let len = len as f64;
        if self.packets >= 1.0 && self.bytes >= len {
            self.packets -= 1.0;
            self.bytes -= len;
            true
        } else {
            false
        }
    }
}

/// Process-wide counters (also read by tests).
#[derive(Debug, Default)]
pub struct Stats {
    /// Connections accepted at the TCP level.
    pub accepted: AtomicU64,
    /// Connections that reached the relay phase.
    pub relayed: AtomicU64,
    /// Connections refused (limits, origin, allowlist, bad control frame).
    pub refused: AtomicU64,
    /// Open connections right now.
    pub open: AtomicUsize,
}

/// A running proxy's shared state.
pub struct Proxy {
    config: Config,
    per_ip: Mutex<HashMap<IpAddr, usize>>,
    next_id: AtomicU64,
    started: Instant,
    /// Counters.
    pub stats: Stats,
}

macro_rules! plog {
    ($proxy:expr, $level:expr, $($arg:tt)*) => {
        if $proxy.config.log >= $level {
            let t = $proxy.started.elapsed();
            eprintln!("[{:>8}.{:03}] {}", t.as_secs(), t.subsec_millis(), format_args!($($arg)*));
        }
    };
}

/// Releases the connection slots when a connection task ends.
struct Slot {
    proxy: Arc<Proxy>,
    ip: IpAddr,
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.proxy.stats.open.fetch_sub(1, Ordering::Relaxed);
        let mut m = self.proxy.per_ip.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(n) = m.get_mut(&self.ip) {
            *n -= 1;
            if *n == 0 {
                m.remove(&self.ip);
            }
        }
    }
}

#[derive(Default)]
struct Counters {
    up_dgrams: u64,
    up_bytes: u64,
    down_dgrams: u64,
    down_bytes: u64,
    dropped_size: u64,
    dropped_rate: u64,
}

impl Proxy {
    /// New proxy state.
    pub fn new(config: Config) -> Arc<Self> {
        Arc::new(Self {
            config,
            per_ip: Mutex::new(HashMap::new()),
            next_id: AtomicU64::new(1),
            started: Instant::now(),
            stats: Stats::default(),
        })
    }

    /// The configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Accepts connections forever.
    pub async fn serve(self: Arc<Self>, listener: TcpListener) -> std::io::Result<()> {
        if let Ok(a) = listener.local_addr() {
            let targets = if self.config.allow_any {
                "any target".to_string()
            } else {
                self.config
                    .allow
                    .iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            plog!(
                self,
                LogLevel::Quiet,
                "sa-udp-proxy: listening on ws://{a}, allowed: {targets}"
            );
        }
        loop {
            let (tcp, peer) = match listener.accept().await {
                Ok(x) => x,
                Err(e) => {
                    // Out of file descriptors and the like: back off instead of spinning.
                    plog!(self, LogLevel::Quiet, "accept failed: {e}");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
            };
            self.stats.accepted.fetch_add(1, Ordering::Relaxed);
            let id = self.next_id.fetch_add(1, Ordering::Relaxed);
            let Some(slot) = self.take_slot(peer.ip()) else {
                self.stats.refused.fetch_add(1, Ordering::Relaxed);
                plog!(self, LogLevel::Quiet, "#{id} {peer}: refused, connection limit");
                drop(tcp);
                continue;
            };
            let proxy = Arc::clone(&self);
            tokio::spawn(async move {
                let _slot = slot;
                proxy.handle(id, peer, tcp).await;
            });
        }
    }

    fn take_slot(self: &Arc<Self>, ip: IpAddr) -> Option<Slot> {
        let mut m = self.per_ip.lock().unwrap_or_else(|e| e.into_inner());
        if self.stats.open.load(Ordering::Relaxed) >= self.config.max_connections {
            return None;
        }
        let n = m.entry(ip).or_insert(0);
        if *n >= self.config.max_per_ip {
            return None;
        }
        *n += 1;
        self.stats.open.fetch_add(1, Ordering::Relaxed);
        Some(Slot {
            proxy: Arc::clone(self),
            ip,
        })
    }

    fn origin_ok(&self, req: &Request) -> bool {
        if self.config.origins.is_empty() {
            return true;
        }
        let Some(o) = req.headers().get("origin").and_then(|v| v.to_str().ok()) else {
            return false;
        };
        self.config.origins.iter().any(|a| a.eq_ignore_ascii_case(o))
    }

    async fn handle(self: Arc<Self>, id: u64, peer: SocketAddr, tcp: TcpStream) {
        let _ = tcp.set_nodelay(true);
        let ws_config = WebSocketConfig::default()
            .max_message_size(Some(self.config.max_datagram.max(4096) * 4))
            .max_frame_size(Some(self.config.max_datagram.max(4096) * 4));
        let mut origin_refused = false;
        // The signature is tungstenite's handshake `Callback`.
        #[allow(clippy::result_large_err)]
        let callback = |req: &Request, resp: Response| -> Result<Response, ErrorResponse> {
            if self.origin_ok(req) {
                Ok(resp)
            } else {
                origin_refused = true;
                let mut e = ErrorResponse::new(Some("origin not allowed".into()));
                *e.status_mut() = StatusCode::FORBIDDEN;
                Err(e)
            }
        };
        let deadline = tokio::time::Instant::now() + self.config.connect_timeout;
        let accepted = tokio::time::timeout_at(
            deadline,
            tokio_tungstenite::accept_hdr_async_with_config(tcp, callback, Some(ws_config)),
        )
        .await;
        let mut ws = match accepted {
            Ok(Ok(ws)) => ws,
            Ok(Err(e)) => {
                self.stats.refused.fetch_add(1, Ordering::Relaxed);
                let why = if origin_refused {
                    "origin not allowed".to_string()
                } else {
                    e.to_string()
                };
                plog!(
                    self,
                    LogLevel::Quiet,
                    "#{id} {peer}: WebSocket handshake failed: {why}"
                );
                return;
            }
            Err(_) => {
                self.stats.refused.fetch_add(1, Ordering::Relaxed);
                plog!(
                    self,
                    LogLevel::Quiet,
                    "#{id} {peer}: WebSocket handshake timed out"
                );
                return;
            }
        };

        // Control frame.
        let first = loop {
            match tokio::time::timeout_at(deadline, ws.next()).await {
                Ok(Some(Ok(Message::Text(t)))) => break Some(t.to_string()),
                Ok(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => continue,
                Ok(Some(Ok(_))) => break None,
                _ => {
                    self.stats.refused.fetch_add(1, Ordering::Relaxed);
                    plog!(self, LogLevel::Quiet, "#{id} {peer}: no connect frame");
                    return;
                }
            }
        };
        let checked = match first {
            Some(t) => check_target(&self.config, &t).await,
            None => Err(Refusal::BadRequest(
                "first frame must be a text connect frame".into(),
            )),
        };
        let target = match checked {
            Ok(a) => a,
            Err(r) => {
                self.stats.refused.fetch_add(1, Ordering::Relaxed);
                plog!(self, LogLevel::Quiet, "#{id} {peer}: refused: {r}");
                let _ = ws
                    .send(Message::text(
                        serde_json::json!({ "error": r.to_string() }).to_string(),
                    ))
                    .await;
                let _ = ws
                    .close(Some(CloseFrame {
                        code: CloseCode::Policy,
                        reason: "refused".into(),
                    }))
                    .await;
                return;
            }
        };
        let bind: SocketAddr = if target.is_ipv4() {
            "0.0.0.0:0".parse()
        } else {
            "[::]:0".parse()
        }
        .unwrap_or_else(|_| unreachable!("literal socket address"));
        let udp = match UdpSocket::bind(bind).await {
            Ok(u) => u,
            Err(e) => {
                plog!(self, LogLevel::Quiet, "#{id} {peer}: UDP bind failed: {e}");
                let _ = ws
                    .send(Message::text(
                        serde_json::json!({ "error": "proxy: UDP bind failed" }).to_string(),
                    ))
                    .await;
                return;
            }
        };
        // A connected socket: only the target's datagrams are received.
        if let Err(e) = udp.connect(target).await {
            plog!(
                self,
                LogLevel::Quiet,
                "#{id} {peer}: UDP connect to {target} failed: {e}"
            );
            let _ = ws
                .send(Message::text(
                    serde_json::json!({ "error": "proxy: UDP connect failed" }).to_string(),
                ))
                .await;
            return;
        }
        if ws
            .send(Message::text(
                serde_json::json!({ "connected": target.to_string() }).to_string(),
            ))
            .await
            .is_err()
        {
            return;
        }
        self.stats.relayed.fetch_add(1, Ordering::Relaxed);
        let local = udp.local_addr().map(|a| a.to_string()).unwrap_or_default();
        plog!(
            self,
            LogLevel::Normal,
            "#{id} {peer}: relaying to {target} from udp {local}"
        );

        let start = Instant::now();
        let (c, reason) = self.relay(id, &mut ws, &udp).await;
        let _ = ws
            .close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: reason.clone().into(),
            }))
            .await;
        plog!(
            self,
            LogLevel::Normal,
            "#{id} {peer}: closed after {:.1} s ({reason}); up {} dg / {} B, down {} dg / {} B, dropped {} oversize, {} rate",
            start.elapsed().as_secs_f64(),
            c.up_dgrams,
            c.up_bytes,
            c.down_dgrams,
            c.down_bytes,
            c.dropped_size,
            c.dropped_rate
        );
    }

    async fn relay(
        &self,
        id: u64,
        ws: &mut tokio_tungstenite::WebSocketStream<TcpStream>,
        udp: &UdpSocket,
    ) -> (Counters, String) {
        let cfg = &self.config;
        let mut c = Counters::default();
        let now = Instant::now();
        let mut up = RateLimit::new(cfg.max_packets_per_sec, cfg.max_bytes_per_sec, now);
        let mut down = RateLimit::new(cfg.max_packets_per_sec, cfg.max_bytes_per_sec, now);
        let mut buf = vec![0u8; 65536];
        let mut last_activity = tokio::time::Instant::now();
        let reason = loop {
            let idle = tokio::time::sleep_until(last_activity + cfg.idle_timeout);
            tokio::select! {
                msg = ws.next() => match msg {
                    Some(Ok(Message::Binary(b))) => {
                        let now = Instant::now();
                        if b.len() > cfg.max_datagram || b.is_empty() {
                            c.dropped_size += 1;
                            plog!(self, LogLevel::Verbose, "#{id}: dropped {} B datagram from client (size)", b.len());
                        } else if !up.allow(b.len(), now) {
                            c.dropped_rate += 1;
                            plog!(self, LogLevel::Verbose, "#{id}: dropped {} B datagram from client (rate)", b.len());
                        } else {
                            match udp.send(&b).await {
                                Ok(n) => {
                                    c.up_dgrams += 1;
                                    c.up_bytes += n as u64;
                                    last_activity = tokio::time::Instant::now();
                                    plog!(self, LogLevel::Verbose, "#{id}: -> {n} B");
                                }
                                // ICMP unreachable from an earlier datagram (server not up): UDP semantics, go on.
                                Err(e) if matches!(e.kind(), std::io::ErrorKind::ConnectionRefused) => {}
                                Err(e) => break format!("udp send: {e}"),
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break "client closed".to_string(),
                    Some(Ok(Message::Text(_))) => break "unexpected text frame".to_string(),
                    Some(Ok(_)) => {} // ping/pong (tungstenite answers pings itself)
                    Some(Err(e)) => break format!("websocket: {e}"),
                },
                r = udp.recv(&mut buf) => match r {
                    Ok(n) => {
                        let now = Instant::now();
                        if n > cfg.max_datagram {
                            c.dropped_size += 1;
                        } else if !down.allow(n, now) {
                            c.dropped_rate += 1;
                        } else {
                            if let Err(e) = ws.send(Message::binary(buf[..n].to_vec())).await {
                                break format!("websocket send: {e}");
                            }
                            c.down_dgrams += 1;
                            c.down_bytes += n as u64;
                            last_activity = tokio::time::Instant::now();
                            plog!(self, LogLevel::Verbose, "#{id}: <- {n} B");
                        }
                    }
                    Err(e) if matches!(e.kind(), std::io::ErrorKind::ConnectionRefused) => {}
                    Err(e) => break format!("udp recv: {e}"),
                },
                _ = idle => break format!("idle for {} s", cfg.idle_timeout.as_secs()),
            }
        };
        (c, reason)
    }
}
