//! Control channel: a line protocol on 127.0.0.1 (port from `SA_HOOK_PORT`, default 47800).
//! Every command gets exactly one reply line.
//!
//! | command | reply |
//! |---|---|
//! | `ping` | `pong` |
//! | `frames` | `frames hook=N game=M state=S time=T` (boundary count, `CTimer::m_FrameCounter`, `gGameState`, `m_snTimeInMilliseconds`) |
//! | `hooks` | `hooks N; ADDR NAME MODE calls=C; …` |
//! | `hook ADDR\|NAME rust\|orig` | `ok NAME MODE frame=N` once applied at a frame boundary, `pending …` on timeout |
//! | `peek ADDR LEN` | `peek ADDR HEX` (LEN ≤ 4096) |
//! | `selftest` | parity check original vs Rust on the game thread: `selftest NAME n=… mismatches=…; …` |
//! | `hashes [FRAME]` | frame-state hashes of the latest (or given) frame: `hashes frame=F tick=T NAME=HEX …` |
//! | `det` | oracle status: `det enabled=… det=… frame=… ticks=… input=MODE … desyncs=…` |
//! | `record PATH` / `record stop` | start recording the pad input at the next tick / write it to PATH |
//! | `replay PATH` | inject a recording from the next tick on |
//! | `stack` | main thread EIP/ESP and the stack words pointing into game code (hang diagnosis) |
//! | `hook ADDR\|NAME l2` | route every call through the L2 oracle (original vs Rust at the call site) |
//! | `l2` | L2 oracle summary: per function calls, cases, mismatches, undeclared writes |
//! | `l2 flush` | flush the L2 spool and report |

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::time::Duration;

pub const DEFAULT_PORT: u16 = 47800;
pub const MAX_PEEK: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Ping,
    Frames,
    Hooks,
    Hook { key: String, rust: bool },
    Peek { addr: u32, len: usize },
    Selftest,
    Hashes(Option<u32>),
    Det,
    Record(String),
    Replay(String),
    Stack,
    HookL2(String),
    L2(Option<String>),
    Unknown(String),
}

pub fn parse_num(s: &str) -> Option<u64> {
    match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Some(h) => u64::from_str_radix(h, 16).ok(),
        None => s.parse().ok(),
    }
}

pub fn parse(line: &str) -> Option<Command> {
    let t = line.trim();
    if t.is_empty() {
        return None;
    }
    let w: Vec<&str> = t.split_whitespace().collect();
    let lower = w[0].to_ascii_lowercase();
    Some(match (lower.as_str(), &w[1..]) {
        ("ping", []) => Command::Ping,
        ("frames", []) => Command::Frames,
        ("hooks", []) => Command::Hooks,
        ("selftest", []) => Command::Selftest,
        ("hashes", []) => Command::Hashes(None),
        ("hashes", [f]) => match f.parse() {
            Ok(f) => Command::Hashes(Some(f)),
            Err(_) => Command::Unknown(t.to_string()),
        },
        ("det", []) => Command::Det,
        ("stack", []) => Command::Stack,
        ("l2", []) => Command::L2(None),
        ("l2", [a]) => Command::L2(Some(a.to_string())),
        ("hook", [key, m]) if m.eq_ignore_ascii_case("l2") => Command::HookL2(key.to_string()),
        ("record", [p]) => Command::Record(p.to_string()),
        ("replay", [p]) => Command::Replay(p.to_string()),
        ("hook", [key, m]) if m.eq_ignore_ascii_case("rust") || m.eq_ignore_ascii_case("orig") => {
            Command::Hook {
                key: key.to_string(),
                rust: m.eq_ignore_ascii_case("rust"),
            }
        }
        ("peek", [a, n]) => match (parse_num(a), parse_num(n)) {
            (Some(addr), Some(len)) if addr <= u32::MAX as u64 && (1..=MAX_PEEK as u64).contains(&len) => {
                Command::Peek {
                    addr: addr as u32,
                    len: len as usize,
                }
            }
            _ => Command::Unknown(t.to_string()),
        },
        _ => Command::Unknown(t.to_string()),
    })
}

pub fn port_from_env(v: Option<&str>) -> u16 {
    v.and_then(|s| s.trim().parse().ok())
        .filter(|&p| p != 0)
        .unwrap_or(DEFAULT_PORT)
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Accepts connections forever (one at a time); each line gets one reply line from `handle`.
pub fn serve(listener: TcpListener, mut handle: impl FnMut(&Command) -> String) {
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(60)));
        let _ = stream.set_nodelay(true);
        let Ok(mut w) = stream.try_clone() else { continue };
        let mut r = BufReader::new(stream);
        let mut line = String::new();
        loop {
            line.clear();
            match r.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
            if line.len() > 4096 {
                break;
            }
            let Some(cmd) = parse(&line) else { continue };
            let mut reply = handle(&cmd);
            reply.retain(|c| c != '\n');
            reply.push('\n');
            if w.write_all(reply.as_bytes()).is_err() {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpStream;

    #[test]
    fn parse_commands() {
        assert_eq!(parse("ping\r\n"), Some(Command::Ping));
        assert_eq!(parse(" FRAMES "), Some(Command::Frames));
        assert_eq!(parse("hooks"), Some(Command::Hooks));
        assert_eq!(parse(""), None);
        assert_eq!(
            parse("hook 0x561b10 orig"),
            Some(Command::Hook {
                key: "0x561b10".into(),
                rust: false
            })
        );
        assert_eq!(
            parse("hook CGeneral::LimitRadianAngle RUST"),
            Some(Command::Hook {
                key: "CGeneral::LimitRadianAngle".into(),
                rust: true
            })
        );
        assert!(matches!(parse("hook x maybe"), Some(Command::Unknown(_))));
        assert_eq!(
            parse("peek 0xB7CB4C 4"),
            Some(Command::Peek {
                addr: 0xB7CB4C,
                len: 4
            })
        );
        assert!(matches!(parse("peek 0x10 99999"), Some(Command::Unknown(_))));
        assert!(matches!(parse("peek 0x1ffffffff 4"), Some(Command::Unknown(_))));
        assert_eq!(parse("hashes"), Some(Command::Hashes(None)));
        assert_eq!(parse("hashes 120"), Some(Command::Hashes(Some(120))));
        assert!(matches!(parse("hashes x"), Some(Command::Unknown(_))));
        assert_eq!(parse("det"), Some(Command::Det));
        assert_eq!(
            parse("record C:\\r.sarec"),
            Some(Command::Record("C:\\r.sarec".into()))
        );
        assert_eq!(parse("record stop"), Some(Command::Record("stop".into())));
        assert_eq!(parse("replay r.sarec"), Some(Command::Replay("r.sarec".into())));
        assert_eq!(parse("l2"), Some(Command::L2(None)));
        assert_eq!(parse("l2 flush"), Some(Command::L2(Some("flush".into()))));
        assert_eq!(
            parse("hook CVector::Normalise L2"),
            Some(Command::HookL2("CVector::Normalise".into()))
        );
        assert_eq!(port_from_env(Some("47801")), 47801);
        assert_eq!(port_from_env(Some("x")), DEFAULT_PORT);
        assert_eq!(port_from_env(None), DEFAULT_PORT);
        assert_eq!(hex(&[0, 0xAB]), "00ab");
    }

    #[test]
    fn serves_lines() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        std::thread::spawn(move || {
            serve(l, |c| match c {
                Command::Ping => "pong".into(),
                _ => "error\nwith newline".into(),
            })
        });
        let mut s = TcpStream::connect(addr).unwrap();
        s.write_all(b"ping\nbogus\n").unwrap();
        let mut r = BufReader::new(s);
        let (mut a, mut b) = (String::new(), String::new());
        r.read_line(&mut a).unwrap();
        r.read_line(&mut b).unwrap();
        assert_eq!((a.as_str(), b.as_str()), ("pong\n", "errorwith newline\n"));
    }
}
