//! samp-rs's RakNet layer on the mod's UDP sockets: one socket per connection attempt, as RakNet opens one per
//! `Connect`.

use std::io::{self, ErrorKind};
use std::net::{SocketAddr, SocketAddrV4};

use sa_mod::Error;
use sa_mod::net::{Socket, State};
use sa_net::{DatagramTransport, LinkState};
use samp_client::netclient::{NetConnector, Transport};

/// A mod socket as RakNet's datagram channel.
struct ModTransport(Socket);

fn io_error(e: Error) -> io::Error {
    match e {
        Error::Again => ErrorKind::WouldBlock.into(),
        Error::Denied => io::Error::new(ErrorKind::PermissionDenied, "the manifest's net rules do not allow it"),
        e => io::Error::other(format!("{e:?}")),
    }
}

impl DatagramTransport for ModTransport {
    fn send(&mut self, datagram: &[u8]) -> io::Result<usize> {
        self.0.send(datagram).map_err(io_error)
    }

    fn recv(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.recv(buf).map_err(io_error)
    }

    fn state(&self) -> LinkState {
        match self.0.state() {
            State::Connecting => LinkState::Connecting,
            State::Open => LinkState::Open,
            State::Closed => LinkState::Closed("closed by the host".into()),
        }
    }
}

/// Opens the mod's sockets to the server as the user named it (the host resolves the name).
pub struct ModConnector;

impl NetConnector for ModConnector {
    fn open(&self, _addr: SocketAddrV4, text: &str) -> io::Result<(Transport, Option<SocketAddr>)> {
        let socket = Socket::open(text).map_err(io_error)?;
        Ok((Box::new(ModTransport(socket)), None))
    }
}
