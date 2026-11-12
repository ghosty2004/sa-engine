//! Native UDP transport: exactly what samp-rs's `netclient.rs` does today (bind 0.0.0.0:0, connect,
//! non-blocking).

use std::io::{self, ErrorKind};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs, UdpSocket};

use crate::{Connector, DatagramTransport, LinkState};

/// A connected, non-blocking UDP socket.
#[derive(Debug)]
pub struct UdpTransport {
    sock: UdpSocket,
    server: SocketAddr,
}

impl UdpTransport {
    /// Binds an ephemeral port and connects it to `server`.
    pub fn connect(server: SocketAddr) -> io::Result<Self> {
        let bind: SocketAddr = if server.is_ipv4() {
            (Ipv4Addr::UNSPECIFIED, 0).into()
        } else {
            (Ipv6Addr::UNSPECIFIED, 0).into()
        };
        let sock = UdpSocket::bind(bind)?;
        sock.connect(server)?;
        sock.set_nonblocking(true)?;
        Ok(Self { sock, server })
    }

    /// The local address (the source address the server sees, before NAT).
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.sock.local_addr()
    }

    /// The server address.
    pub fn server(&self) -> SocketAddr {
        self.server
    }
}

impl DatagramTransport for UdpTransport {
    fn send(&mut self, datagram: &[u8]) -> io::Result<usize> {
        self.sock.send(datagram)
    }

    fn recv(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            match self.sock.recv(buf) {
                // ICMP port unreachable from an earlier datagram (server not up yet): not data, try again.
                Err(e)
                    if matches!(
                        e.kind(),
                        ErrorKind::ConnectionRefused | ErrorKind::ConnectionReset
                    ) =>
                {
                    continue;
                }
                r => return r,
            }
        }
    }

    fn state(&self) -> LinkState {
        LinkState::Open
    }
}

/// Opens [`UdpTransport`]s (IPv4 preferred: RakNet 2.x servers are IPv4 only).
#[derive(Clone, Copy, Debug, Default)]
pub struct UdpConnector;

impl Connector for UdpConnector {
    type Transport = UdpTransport;

    fn connect(&self, server: &str) -> io::Result<UdpTransport> {
        let addrs: Vec<SocketAddr> = server.to_socket_addrs()?.collect();
        let addr = addrs
            .iter()
            .find(|a| a.is_ipv4())
            .or(addrs.first())
            .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "server did not resolve"))?;
        UdpTransport::connect(*addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn round_trip_over_loopback() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        server.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        let mut t = UdpConnector
            .connect(&server.local_addr().unwrap().to_string())
            .unwrap();
        let mut buf = [0u8; 64];
        assert_eq!(t.recv(&mut buf).unwrap_err().kind(), ErrorKind::WouldBlock);
        t.send(b"\x18\x69\x69").unwrap();
        let (n, peer) = server.recv_from(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"\x18\x69\x69");
        server.send_to(b"\x1a\x34\x12", peer).unwrap();
        let start = Instant::now();
        let n = loop {
            match t.recv(&mut buf) {
                Ok(n) => break n,
                Err(e) if e.kind() == ErrorKind::WouldBlock && start.elapsed() < Duration::from_secs(2) => {
                    std::thread::sleep(Duration::from_millis(2))
                }
                Err(e) => panic!("{e}"),
            }
        };
        assert_eq!(&buf[..n], b"\x1a\x34\x12");
        assert_eq!(t.state(), LinkState::Open);
    }
}
