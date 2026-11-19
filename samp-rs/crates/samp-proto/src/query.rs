//! The server query protocol: plaintext UDP datagrams to the game port, outside the RakNet connection (what
//! server browsers send). A request is `SAMP`, the server's IPv4 address and port as the client addressed it, and an
//! opcode; the server answers with the same 11 bytes followed by the opcode's payload. Texts are the server's code
//! page bytes, kept as such.

/// The request opcodes.
pub mod op {
    /// Server information: password flag, players, hostname, gamemode, language.
    pub const INFO: u8 = b'i';
    /// The server's rules (name / value pairs: version, weather, worldtime, mapname, ...).
    pub const RULES: u8 = b'r';
    /// The players' names and scores (servers leave it empty above 100 players).
    pub const CLIENTS: u8 = b'c';
    /// Echoes four bytes (round-trip time).
    pub const PING: u8 = b'p';
}

/// The size of the header a request and its answer start with.
pub const HEADER: usize = 11;

/// A request with `opcode` for the server at `addr`:`port`; [`op::PING`] carries `payload`.
pub fn request(addr: [u8; 4], port: u16, opcode: u8, payload: [u8; 4]) -> Vec<u8> {
    let mut d = Vec::with_capacity(HEADER + 4);
    d.extend_from_slice(b"SAMP");
    d.extend_from_slice(&addr);
    d.extend_from_slice(&port.to_le_bytes());
    d.push(opcode);
    if opcode == op::PING {
        d.extend_from_slice(&payload);
    }
    d
}

/// The `i` answer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Info {
    /// The server asks for a password.
    pub password: bool,
    /// Players online.
    pub players: u16,
    /// Player slots.
    pub max_players: u16,
    /// The server's name.
    pub hostname: Vec<u8>,
    /// The game mode's name.
    pub gamemode: Vec<u8>,
    /// The language (the map name on servers before 0.3).
    pub language: Vec<u8>,
}

/// A server's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reply {
    /// [`op::INFO`].
    Info(Info),
    /// [`op::RULES`]: (name, value).
    Rules(Vec<(Vec<u8>, Vec<u8>)>),
    /// [`op::CLIENTS`]: (name, score).
    Clients(Vec<(Vec<u8>, i32)>),
    /// [`op::PING`]: the payload echoed.
    Ping([u8; 4]),
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let (a, b) = (self.0.get(..n)?, self.0.get(n..)?);
        self.0 = b;
        Some(a)
    }
    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn text8(&mut self) -> Option<Vec<u8>> {
        let n = self.u8()?;
        Some(self.take(n.into())?.to_vec())
    }
    fn text32(&mut self) -> Option<Vec<u8>> {
        let n = self.u32()?;
        Some(self.take(usize::try_from(n).ok()?)?.to_vec())
    }
}

/// Parses an answer (`None`: not a query answer, or cut short).
pub fn parse_reply(data: &[u8]) -> Option<Reply> {
    if data.len() < HEADER || &data[..4] != b"SAMP" {
        return None;
    }
    let mut r = Reader(&data[HEADER..]);
    Some(match data[HEADER - 1] {
        op::INFO => Reply::Info(Info {
            password: r.u8()? != 0,
            players: r.u16()?,
            max_players: r.u16()?,
            hostname: r.text32()?,
            gamemode: r.text32()?,
            language: r.text32()?,
        }),
        op::RULES => {
            let n = r.u16()?;
            Reply::Rules((0..n).map(|_| Some((r.text8()?, r.text8()?))).collect::<Option<_>>()?)
        }
        op::CLIENTS => {
            let n = r.u16()?;
            Reply::Clients((0..n).map(|_| Some((r.text8()?, r.u32()? as i32))).collect::<Option<_>>()?)
        }
        op::PING => Reply::Ping(r.take(4)?.try_into().ok()?),
        _ => return None,
    })
}

/// The IPv4 address bytes of `host` when it is a dotted quad; zeros otherwise (servers do not check them).
pub fn address_bytes(host: &str) -> [u8; 4] {
    host.parse::<std::net::Ipv4Addr>().map_or([0; 4], |a| a.octets())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(opcode: u8, payload: &[u8]) -> Vec<u8> {
        let mut d = request([127, 0, 0, 1], 7777, opcode, [0; 4])[..HEADER].to_vec();
        d.extend_from_slice(payload);
        d
    }

    #[test]
    fn request_layout() {
        assert_eq!(request([127, 0, 0, 1], 7777, op::INFO, [9; 4]), b"SAMP\x7f\x00\x00\x01\x61\x1ei");
        assert_eq!(request([1, 2, 3, 4], 1, op::PING, [5, 6, 7, 8]), b"SAMP\x01\x02\x03\x04\x01\x00p\x05\x06\x07\x08");
        assert_eq!(address_bytes("10.0.0.2"), [10, 0, 0, 2]);
        assert_eq!(address_bytes("example.org"), [0; 4]);
    }

    #[test]
    fn answers() {
        let mut p = vec![1, 3, 0, 50, 0];
        for s in [&b"Host"[..], b"Mode", b"EN"] {
            p.extend_from_slice(&(s.len() as u32).to_le_bytes());
            p.extend_from_slice(s);
        }
        let info = Info {
            password: true,
            players: 3,
            max_players: 50,
            hostname: b"Host".to_vec(),
            gamemode: b"Mode".to_vec(),
            language: b"EN".to_vec(),
        };
        assert_eq!(parse_reply(&answer(op::INFO, &p)), Some(Reply::Info(info)));
        assert_eq!(parse_reply(&answer(op::INFO, &p[..p.len() - 1])), None);
        let rules = answer(op::RULES, b"\x01\x00\x07weather\x0210");
        assert_eq!(parse_reply(&rules), Some(Reply::Rules(vec![(b"weather".to_vec(), b"10".to_vec())])));
        let clients = answer(op::CLIENTS, b"\x01\x00\x05ModCJ\x2a\x00\x00\x00");
        assert_eq!(parse_reply(&clients), Some(Reply::Clients(vec![(b"ModCJ".to_vec(), 42)])));
        assert_eq!(parse_reply(&answer(op::PING, &[1, 2, 3, 4])), Some(Reply::Ping([1, 2, 3, 4])));
        assert_eq!(parse_reply(b"SAMQ\0\0\0\0\0\0i"), None);
    }
}
