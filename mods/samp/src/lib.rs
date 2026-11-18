//! SA:MP as a mod: samp-rs's client core (its RakNet session `NetClient` and the netgame's connection lifecycle)
//! running inside the game's mod host. The core reaches its server through the mod's UDP sockets
//! ([`transport::ModConnector`]; in the browser the host relays them through the page's UDP proxy), runs on the
//! host's clock and keeps its settings in the mod's storage:
//!
//! * `server`: `HOST:PORT` (default `127.0.0.1:7777`), which the manifest's `net` rules must allow;
//! * `name`: the player name (default `ModCJ`);
//! * `auth_table.txt`: the servers' auth table (third-party data the user provides; without it the server refuses
//!   the join at its auth check).
//!
//! It connects, follows samp's connection lifecycle and shows the session and samp's chat lines in the overlay;
//! the game side of the client (players, vehicles, the UI) comes on the game API.

mod transport;

use std::time::Duration;

use sa_mod::{Event, Mod, Overlay, Rgba, event_kind, events, storage, time};
use samp_client::lifecycle::{self, Action, NetGame};
use samp_client::netclient::{Event as NetEvent, NetClient};
use samp_proto::auth::AuthTable;
use samp_proto::time::{HostTime, Instant};

const DEFAULT_SERVER: &str = "127.0.0.1:7777";
const DEFAULT_NAME: &str = "ModCJ";
const CHAT_LINES: usize = 8;
const SIZE: f32 = 16.0;
const MARGIN: f32 = 8.0;

fn host_time() -> HostTime {
    let now = time::now();
    HostTime { monotonic: Duration::from_micros(now.monotonic_us), wall: Duration::from_micros(now.unix_us) }
}

/// A setting from the mod's storage.
fn setting(key: &str) -> Option<String> {
    let v = storage::read(key).ok().flatten()?;
    String::from_utf8(v).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

#[derive(Default)]
struct Samp {
    session: Option<Session>,
    error: Option<String>,
}

struct Session {
    net: NetClient,
    netgame: NetGame,
    start: Instant,
    server: (String, u16),
    chat: Vec<String>,
}

impl Session {
    fn open(server: &str, name: &str, auth: AuthTable) -> Result<Session, String> {
        let (host, port) = server
            .rsplit_once(':')
            .and_then(|(h, p)| Some((h.to_string(), p.parse::<u16>().ok()?)))
            .ok_or_else(|| format!("server {server:?}: expected HOST:PORT"))?;
        let mut net = NetClient::new_unresolved(server, name, auth).map_err(|e| e.to_string())?;
        net.set_connector(Box::new(transport::ModConnector));
        let start = Instant::now();
        Ok(Session { net, netgame: NetGame::new(0), start, server: (host, port), chat: Vec::new() })
    }

    fn chat_line(&mut self, line: &str) {
        sa_mod::info(&format!("samp: {line}"));
        self.chat.push(line.to_string());
        if self.chat.len() > CHAT_LINES {
            self.chat.remove(0);
        }
    }

    fn act(&mut self, actions: Vec<Action>) {
        for a in actions {
            match a {
                Action::Debug(line) | Action::Info(line) => self.chat_line(line),
                Action::Disconnect(_) => self.net.disconnect_silent(),
                Action::Reset | Action::DeletePlayers | Action::StopAudio { .. } => {}
            }
        }
    }

    /// samp's netgame tick: the connect timer, then the packets RakNet received since the last frame.
    fn tick(&mut self) {
        let now = Instant::now();
        let now_ms = now.duration_since(self.start).as_millis() as u64;
        if self.netgame.connect_due(now_ms) {
            let line = lifecycle::msg::connecting(&self.server.0, self.server.1);
            self.chat_line(&line);
            if let Err(e) = self.net.connect() {
                self.chat_line(&format!("connect: {e}"));
            }
            self.netgame.attempt_started(Instant::now().duration_since(self.start).as_millis() as u64);
        }
        for e in self.net.tick(now) {
            let actions = match e {
                NetEvent::InitGame(g) => {
                    self.netgame.on_init_game();
                    self.chat_line(&format!("Connected to {}", g.server_name));
                    Vec::new()
                }
                NetEvent::Packet(p) => self.netgame.on_packet(p),
                NetEvent::Rejected(reason) => self.netgame.on_rejected(reason),
                NetEvent::Rpc(_) | NetEvent::Sync(_) => Vec::new(),
            };
            self.act(actions);
        }
    }
}

impl Mod for Samp {
    fn init(&mut self) -> Result<(), String> {
        samp_proto::time::set_clock(host_time);
        let server = setting("server").unwrap_or_else(|| DEFAULT_SERVER.to_string());
        let name = setting("name").unwrap_or_else(|| DEFAULT_NAME.to_string());
        let auth = match setting("auth_table.txt") {
            Some(t) => AuthTable::parse(&t).map_err(|e| format!("auth_table.txt: {e:?}"))?,
            None => {
                sa_mod::log(sa_mod::level::WARN, "samp: no auth_table.txt in the mod's storage: joins fail at auth");
                AuthTable::parse("").map_err(|e| format!("{e:?}"))?
            }
        };
        match Session::open(&server, &name, auth) {
            Ok(mut s) => {
                s.chat_line(samp_client::ui::core::msg::STARTED);
                self.session = Some(s);
            }
            Err(e) => self.error = Some(e),
        }
        events::subscribe(&[event_kind::FRAME_BEGIN]);
        Ok(())
    }

    fn event(&mut self, event: Event) {
        if let (Event::FrameBegin(_), Some(s)) = (event, self.session.as_mut()) {
            s.tick();
        }
    }

    fn overlay(&mut self, _frame: u32, o: &mut Overlay) {
        let mut lines = Vec::new();
        match &self.session {
            Some(s) => {
                lines.push(format!("SA:MP {}:{} ({:?})", s.server.0, s.server.1, s.net.phase()));
                lines.extend(s.chat.iter().cloned());
            }
            None => lines.push(format!("SA:MP: {}", self.error.as_deref().unwrap_or("not started"))),
        }
        for (i, l) in lines.iter().enumerate() {
            let y = MARGIN + i as f32 * (SIZE + 2.0);
            let _ = o.text(MARGIN, y, SIZE, Rgba::new(0xA9, 0xC4, 0xE4, 0xFF), l);
        }
    }
}

sa_mod::entry!(Samp);
