//! Datagram transports for the SA:MP client (docs/samp/web-proxy.md).
//!
//! samp-rs's RakNet layer (`samp-client` `netclient.rs`) talks to the server through one connected,
//! non-blocking UDP socket per connection attempt. [`DatagramTransport`] is that socket's surface, with the
//! same semantics (`recv` returns `ErrorKind::WouldBlock` when nothing is queued; `send` never blocks), so the
//! RakNet code can run unchanged over:
//!
//! * [`UdpTransport`] (native: a real UDP socket),
//! * `WebSocketTransport` (wasm32: a browser `WebSocket` to `sa-udp-proxy`, one binary frame per datagram),
//! * `NativeWsTransport` (host, feature `native-ws`: the same proxy protocol from a native process, for tests).
//!
//! A [`Connector`] opens one transport per connection attempt, as RakNet opens a new socket for every
//! `Connect` (a new source port at the server).
//!
//! Proxy protocol: after the WebSocket opens the client sends the text frame `{"connect":"host:port"}`; the
//! proxy answers `{"connected":"ip:port"}` (datagrams flow from then on) or `{"error":"..."}` and closes.

#![forbid(unsafe_code)]

use std::io;

#[cfg(not(target_arch = "wasm32"))]
mod udp;
#[cfg(not(target_arch = "wasm32"))]
pub use udp::{UdpConnector, UdpTransport};

#[cfg(all(not(target_arch = "wasm32"), feature = "native-ws"))]
mod native_ws;
#[cfg(all(not(target_arch = "wasm32"), feature = "native-ws"))]
pub use native_ws::{NativeWsConnector, NativeWsTransport};

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::{WebSocketConnector, WebSocketTransport};

/// Where a transport stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkState {
    /// Opening (WebSocket handshake or waiting for the proxy's `connected`). Sent datagrams are queued.
    Connecting,
    /// Datagrams flow.
    Open,
    /// Closed or refused; the string says why. Sends fail, `recv` drains what was already received.
    Closed(String),
}

/// One connected datagram channel to one server, polled from RakNet's update.
pub trait DatagramTransport {
    /// Sends one datagram without blocking. Returns its length. While [`LinkState::Connecting`] the datagram
    /// may be queued (a full queue drops it, like a lost UDP datagram); after [`LinkState::Closed`] it fails
    /// with `ErrorKind::NotConnected`.
    fn send(&mut self, datagram: &[u8]) -> io::Result<usize>;

    /// Takes one received datagram into `buf` (truncated to `buf.len()`, like UDP) without blocking. No datagram
    /// queued: `ErrorKind::WouldBlock`.
    fn recv(&mut self, buf: &mut [u8]) -> io::Result<usize>;

    /// Current state.
    fn state(&self) -> LinkState;
}

/// Opens a transport to `host:port`; called once per connection attempt.
pub trait Connector {
    /// The transport this connector makes.
    type Transport: DatagramTransport;
    /// Starts opening a transport to `server` (`host:port`). Returns at once for asynchronous transports
    /// (state [`LinkState::Connecting`]).
    fn connect(&self, server: &str) -> io::Result<Self::Transport>;
}

/// The proxy's first frame for `server`. Rejects targets that would need JSON escaping.
pub fn connect_frame(server: &str) -> io::Result<String> {
    if server.is_empty() || server.chars().any(|c| c == '"' || c == '\\' || c.is_control()) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "bad server address"));
    }
    Ok(format!("{{\"connect\":\"{server}\"}}"))
}

/// What a text frame from the proxy means.
#[derive(Debug, PartialEq, Eq)]
pub enum ProxyReply {
    /// `{"connected":...}`.
    Connected,
    /// `{"error":"..."}` (the message), or anything unexpected.
    Error(String),
}

/// Reads a proxy text frame (small hand parser: no JSON dependency in the wasm build).
pub fn parse_reply(text: &str) -> ProxyReply {
    let t = text.trim();
    if t.starts_with("{\"connected\"") {
        return ProxyReply::Connected;
    }
    if let Some(rest) = t.strip_prefix("{\"error\":\"") {
        return ProxyReply::Error(rest.trim_end_matches('}').trim_end_matches('"').to_string());
    }
    ProxyReply::Error(format!("unexpected proxy reply {t}"))
}

/// Datagrams queued while the transport opens (more are dropped).
pub const PENDING_LIMIT: usize = 64;
/// Received datagrams buffered between two polls (more are dropped).
pub const INBOX_LIMIT: usize = 4096;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_frames() {
        assert_eq!(
            connect_frame("127.0.0.1:7777").unwrap(),
            r#"{"connect":"127.0.0.1:7777"}"#
        );
        assert!(connect_frame("a\"b:1").is_err());
        assert_eq!(
            parse_reply(r#"{"connected":"1.2.3.4:7777"}"#),
            ProxyReply::Connected
        );
        assert_eq!(
            parse_reply(r#"{"error":"target not allowed: x:1"}"#),
            ProxyReply::Error("target not allowed: x:1".into())
        );
        assert!(matches!(parse_reply("hello"), ProxyReply::Error(_)));
    }
}
