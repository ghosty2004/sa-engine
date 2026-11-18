//! The net-ping mod: sends a datagram to a UDP echo server once a second, shows the last round trip in the overlay
//! and keeps the number of replies across sessions in its storage. Natively the socket is a real UDP socket; in the
//! browser the page's UDP proxy relays it. Build: `examples/mods/build.sh`; the echo server:
//! `tools/mods/udp-echo.py` (docs/mods/getting-started.md).

use sa_mod::net::{Socket, State};
use sa_mod::{Error, Mod, Overlay, Rgba, storage, time};

const TARGET: &str = "127.0.0.1:7790";
const INTERVAL_US: u64 = 1_000_000;
const REOPEN_US: u64 = 5_000_000;
const KEY_REPLIES: &str = "replies";
const SIZE: f32 = 16.0;
const MARGIN: f32 = 8.0;

#[derive(Default)]
struct NetPing {
    socket: Option<Socket>,
    next_open_us: u64,
    next_send_us: u64,
    seq: u32,
    replies: u64,
    replies_before: u64,
    last_rtt_us: Option<u64>,
    error: Option<String>,
}

/// A ping datagram: `ping SEQ SENT_US`, which the echo server sends back unchanged.
fn ping(seq: u32, sent_us: u64) -> String {
    format!("ping {seq} {sent_us}")
}

/// The send time of an echoed ping.
fn sent_us(reply: &[u8]) -> Option<u64> {
    let text = std::str::from_utf8(reply).ok()?;
    text.strip_prefix("ping ")?.split(' ').nth(1)?.parse().ok()
}

impl NetPing {
    /// Opens the socket from a frame, and again a while after it closed: in the browser the socket is a WebSocket
    /// to the page's UDP proxy, which gives up on a connection the page cannot serve (while the game's boot blocks
    /// the page, before the first frame).
    fn ensure_open(&mut self, now: u64) {
        let closed = self.socket.as_ref().is_none_or(|s| s.state() == State::Closed);
        if !closed || now < self.next_open_us {
            return;
        }
        self.next_open_us = now + REOPEN_US;
        self.socket = None;
        match Socket::open(TARGET) {
            Ok(s) => self.socket = Some(s),
            Err(e) => self.error = Some(format!("open: {e:?}")),
        }
    }

    fn poll(&mut self) {
        let now = time::now().monotonic_us;
        self.ensure_open(now);
        let Some(s) = &self.socket else { return };
        let mut buf = [0u8; 64];
        loop {
            match s.recv(&mut buf) {
                Ok(n) => {
                    if let Some(sent) = sent_us(&buf[..n]) {
                        self.last_rtt_us = Some(now.saturating_sub(sent));
                        self.replies += 1;
                        if self.replies == 1 {
                            sa_mod::info(&format!("net-ping: first reply from {TARGET}"));
                        }
                    }
                }
                Err(Error::Again) => break,
                Err(e) => {
                    self.error = Some(format!("recv: {e:?}"));
                    break;
                }
            }
        }
        if now >= self.next_send_us && s.state() != State::Closed {
            self.next_send_us = now + INTERVAL_US;
            self.seq += 1;
            if let Err(e) = s.send(ping(self.seq, now).as_bytes()) {
                self.error = Some(format!("send: {e:?}"));
            }
            let _ = storage::write(KEY_REPLIES, (self.replies_before + self.replies).to_string().as_bytes());
        }
    }

    fn line(&self) -> String {
        let state = match self.socket.as_ref().map(Socket::state) {
            Some(State::Connecting) => "connecting",
            Some(State::Open) => "open",
            Some(State::Closed) | None => "closed",
        };
        let rtt = self.last_rtt_us.map_or("-".to_string(), |us| format!("{:.2} ms", us as f64 / 1000.0));
        let err = self.error.as_deref().map(|e| format!(", {e}")).unwrap_or_default();
        format!(
            "ping {TARGET}: {state}, rtt {rtt}, {} replies ({} in all){err}",
            self.replies,
            self.replies_before + self.replies
        )
    }
}

impl Mod for NetPing {
    fn init(&mut self) -> Result<(), String> {
        self.replies_before = storage::read(KEY_REPLIES)
            .ok()
            .flatten()
            .and_then(|v| String::from_utf8(v).ok()?.parse().ok())
            .unwrap_or(0);
        sa_mod::info(&format!("net-ping: pinging {TARGET}; {} replies in earlier sessions", self.replies_before));
        Ok(())
    }

    fn overlay(&mut self, _frame: u32, o: &mut Overlay) {
        self.poll();
        let text = self.line();
        let tw = text.chars().count() as f32 * SIZE;
        let _ = o.rect(MARGIN, MARGIN, tw + 12.0, SIZE + 12.0, Rgba::new(0, 0, 0, 0xA0));
        let _ = o.text(MARGIN + 6.0, MARGIN + 6.0, SIZE, Rgba::new(0x80, 0xFF, 0x80, 0xFF), &text);
    }
}

sa_mod::entry!(NetPing);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echoed_pings_carry_their_send_time() {
        assert_eq!(sent_us(ping(7, 123_456).as_bytes()), Some(123_456));
        assert_eq!(sent_us(b"pong"), None);
    }
}
