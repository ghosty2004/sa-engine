//! Native WebSocket transport to `sa-udp-proxy` (feature `native-ws`): the browser path from a host process.
//! Opening is blocking (handshake + control frame, bounded by a timeout); after that the socket is
//! non-blocking and polled like the UDP transport.

use std::collections::VecDeque;
use std::io::{self, ErrorKind};
use std::net::TcpStream;
use std::time::Duration;

use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

use crate::{Connector, DatagramTransport, INBOX_LIMIT, LinkState, ProxyReply, connect_frame, parse_reply};

type Ws = WebSocket<MaybeTlsStream<TcpStream>>;

fn ws_err(e: tungstenite::Error) -> io::Error {
    match e {
        tungstenite::Error::Io(e) => e,
        e => io::Error::other(e.to_string()),
    }
}

/// A WebSocket to the proxy, relaying to one server.
#[derive(Debug)]
pub struct NativeWsTransport {
    ws: Ws,
    state: LinkState,
    inbox: VecDeque<Vec<u8>>,
}

impl NativeWsTransport {
    /// Opens `proxy_url` (`ws://host:port[/path]`) and asks for `server`; blocks until the proxy answered or
    /// `timeout` passed.
    pub fn open(proxy_url: &str, server: &str, timeout: Duration) -> io::Result<Self> {
        let frame = connect_frame(server)?;
        let (mut ws, _) = tungstenite::connect(proxy_url).map_err(ws_err)?;
        let tcp = Self::tcp(&mut ws)?;
        tcp.set_nodelay(true)?;
        tcp.set_read_timeout(Some(timeout))?;
        ws.send(Message::text(frame)).map_err(ws_err)?;
        loop {
            match ws.read().map_err(ws_err)? {
                Message::Text(t) => match parse_reply(&t) {
                    ProxyReply::Connected => break,
                    ProxyReply::Error(e) => {
                        return Err(io::Error::new(
                            ErrorKind::ConnectionRefused,
                            format!("proxy: {e}"),
                        ));
                    }
                },
                Message::Ping(_) | Message::Pong(_) => {}
                m => return Err(io::Error::other(format!("proxy: unexpected {m:?}"))),
            }
        }
        Self::tcp(&mut ws)?.set_nonblocking(true)?;
        Ok(Self {
            ws,
            state: LinkState::Open,
            inbox: VecDeque::new(),
        })
    }

    fn tcp(ws: &mut Ws) -> io::Result<&mut TcpStream> {
        #[allow(unreachable_patterns)]
        match ws.get_mut() {
            MaybeTlsStream::Plain(s) => Ok(s),
            _ => Err(io::Error::new(ErrorKind::Unsupported, "TLS not built in")),
        }
    }

    fn close_with(&mut self, why: String) {
        if !matches!(self.state, LinkState::Closed(_)) {
            self.state = LinkState::Closed(why);
        }
    }

    /// Reads every complete frame the socket holds into the inbox.
    fn pump(&mut self) {
        if matches!(self.state, LinkState::Closed(_)) {
            return;
        }
        loop {
            match self.ws.read() {
                Ok(Message::Binary(b)) => {
                    if self.inbox.len() < INBOX_LIMIT {
                        self.inbox.push_back(b.to_vec());
                    }
                }
                Ok(Message::Close(_)) => return self.close_with("proxy closed".into()),
                Ok(Message::Text(t)) => return self.close_with(format!("proxy: {t}")),
                Ok(_) => {}
                Err(tungstenite::Error::Io(e)) if e.kind() == ErrorKind::WouldBlock => return,
                Err(e) => return self.close_with(e.to_string()),
            }
        }
    }
}

impl DatagramTransport for NativeWsTransport {
    fn send(&mut self, datagram: &[u8]) -> io::Result<usize> {
        if let LinkState::Closed(why) = &self.state {
            return Err(io::Error::new(ErrorKind::NotConnected, why.clone()));
        }
        match self.ws.send(Message::binary(datagram.to_vec())) {
            Ok(()) => Ok(datagram.len()),
            // Queued in tungstenite's write buffer; flushed by the next send or recv.
            Err(tungstenite::Error::Io(e)) if e.kind() == ErrorKind::WouldBlock => Ok(datagram.len()),
            Err(e) => {
                self.close_with(e.to_string());
                Err(io::Error::new(ErrorKind::NotConnected, e.to_string()))
            }
        }
    }

    fn recv(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.inbox.is_empty() {
            match self.ws.flush() {
                Ok(()) => {}
                Err(tungstenite::Error::Io(e)) if e.kind() == ErrorKind::WouldBlock => {}
                Err(e) => self.close_with(e.to_string()),
            }
            self.pump();
        }
        let Some(d) = self.inbox.pop_front() else {
            return Err(ErrorKind::WouldBlock.into());
        };
        let n = d.len().min(buf.len());
        buf[..n].copy_from_slice(&d[..n]);
        Ok(n)
    }

    fn state(&self) -> LinkState {
        self.state.clone()
    }
}

impl Drop for NativeWsTransport {
    fn drop(&mut self) {
        let _ = self.ws.close(None);
        let _ = self.ws.flush();
    }
}

/// Opens [`NativeWsTransport`]s through one proxy.
#[derive(Clone, Debug)]
pub struct NativeWsConnector {
    /// `ws://host:port[/path]` of the proxy.
    pub proxy_url: String,
    /// Bound on the opening handshake.
    pub timeout: Duration,
}

impl Connector for NativeWsConnector {
    type Transport = NativeWsTransport;

    fn connect(&self, server: &str) -> io::Result<NativeWsTransport> {
        NativeWsTransport::open(&self.proxy_url, server, self.timeout)
    }
}
