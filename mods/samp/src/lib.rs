//! SA:MP as a mod: samp-rs's client (`samp_client::hosted`) running inside the game's mod host. The client reaches
//! the game through [`game::ModGame`] (its game adapter over the mod API), its server through the mod's UDP sockets
//! ([`transport::ModConnector`]; in the browser the host relays them through the page's UDP proxy) and runs on the
//! host's clock.
//!
//! Once the game runs its world the mod shows its connect screen ([`connect::Screen`]): the server, the nickname and
//! the favourite servers. The manifest asks for the servers the user approves (`net = ["udp:user-approved"]`): the
//! first connection to a server makes the host ask the user, and the mod waits for the answer. Its storage holds:
//!
//! * `server`, `name`, `favourites`: the connect screen's fields and favourites;
//! * `auth_table.txt`: the servers' auth table (third-party data the user provides; without it the server refuses
//!   the join at its auth check);
//! * `auto_connect`: `1` joins `server` as `name` without the screen, `auto_spawn`: `1` presses Shift in class
//!   selection by itself (headless runs);
//! * `serial_seed`: made once, from the clock, never from user or host data.

mod connect;
mod game;
mod transport;

use std::time::Duration;

use sa_mod::input::Input;
use sa_mod::net::Socket;
use sa_mod::game::{GameOption, set_option};
use sa_mod::{Error, Event, GameState, Mod, Overlay, Rgba, event_kind, events, state, storage, time};
use sa_samp_api::hooks::{FramePhase, GameHooks};
use sa_samp_api::input::{EventResult, WindowEvent};
use samp_client::hosted::{HostedClient, HostedConfig};
use samp_client::netclient::NetClient;
use samp_proto::auth::AuthTable;
use samp_proto::time::HostTime;

const DEFAULT_SERVER: &str = "127.0.0.1:7777";
const DEFAULT_NAME: &str = "ModCJ";

fn host_time() -> HostTime {
    let now = time::now();
    HostTime { monotonic: Duration::from_micros(now.monotonic_us), wall: Duration::from_micros(now.unix_us) }
}

/// A setting from the mod's storage.
fn setting(key: &str) -> Option<String> {
    let v = storage::read(key).ok().flatten()?;
    String::from_utf8(v).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// The client serial's seed, made on the first run and kept.
fn serial_seed() -> String {
    if let Some(s) = setting("serial_seed").filter(|s| s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit())) {
        return s;
    }
    let s = samp_client::netclient::pseudo_random_hex40();
    if let Err(e) = storage::write("serial_seed", s.as_bytes()) {
        sa_mod::log(sa_mod::level::WARN, &format!("samp: cannot keep the serial seed: {e:?}"));
    }
    s
}

fn auth_table() -> Result<AuthTable, String> {
    match setting("auth_table.txt") {
        Some(t) => AuthTable::parse(&t).map_err(|e| format!("auth_table.txt: {e:?}")),
        None => {
            sa_mod::log(sa_mod::level::WARN, "samp: no auth_table.txt in the mod's storage: joins fail at auth");
            AuthTable::parse("").map_err(|e| format!("{e:?}"))
        }
    }
}

fn start(server: &str, name: &str) -> Result<HostedClient, String> {
    let (host, port) = server
        .rsplit_once(':')
        .and_then(|(h, p)| Some((h.to_string(), p.parse::<u16>().ok()?)))
        .ok_or_else(|| format!("server {server:?}: expected HOST:PORT"))?;
    let mut net = NetClient::new_unresolved(server, name, auth_table()?).map_err(|e| e.to_string())?;
    net.set_connector(Box::new(transport::ModConnector));
    net.set_serial_seed(&serial_seed());
    let auto_spawn = setting("auto_spawn").is_some_and(|v| v == "1");
    let config = HostedConfig { host, port, name: name.to_string(), auto_spawn };
    let log = Box::new(|line: &str| sa_mod::info(&format!("samp: {line}")));
    Ok(HostedClient::new(net, config, log))
}

/// Where the mod is.
enum Stage {
    /// The connect screen.
    Screen,
    /// The user chose a server; the host may be asking the user about it.
    Approval { server: String, name: String },
    /// The client runs.
    Client(Box<HostedClient>),
}

struct Samp {
    stage: Stage,
    screen: connect::Screen,
    game: game::ModGame,
}

impl Default for Samp {
    fn default() -> Samp {
        Samp { stage: Stage::Screen, screen: connect::Screen::load(DEFAULT_SERVER, DEFAULT_NAME), game: Default::default() }
    }
}

impl Samp {
    /// One of samp's frame points; the client runs only while the game runs its world (samp hooks
    /// `CGame::Process`, which the start-up and the loading never reach).
    fn frame(&mut self, phase: FramePhase) {
        if state::game_state() != Some(GameState::Playing) {
            return;
        }
        if let Stage::Client(c) = &mut self.stage {
            c.frame(phase, &mut self.game);
        }
    }

    /// Whether the server the user chose may be reached yet: a socket opens once the host has the user's yes.
    fn approval(&mut self) {
        let Stage::Approval { server, name } = &self.stage else { return };
        match Socket::open(server) {
            Ok(probe) => {
                drop(probe);
                match start(server, name) {
                    Ok(c) => self.stage = Stage::Client(Box::new(c)),
                    Err(e) => {
                        self.screen.status = Some(e);
                        self.stage = Stage::Screen;
                    }
                }
            }
            Err(Error::Again) => self.screen.status = Some(format!("Waiting for the permission to reach {server}")),
            Err(e) => {
                self.screen.status = Some(match e {
                    Error::Denied => format!("{server}: not allowed (the game's approved servers)"),
                    e => format!("{server}: {e:?}"),
                });
                self.stage = Stage::Screen;
            }
        }
    }

    /// The keyboard while the connect screen, the chat input or a dialog is up.
    fn capture(&self) {
        let keyboard = match &self.stage {
            Stage::Screen | Stage::Approval { .. } => true,
            Stage::Client(c) => c.ui.wants_keyboard(),
        };
        let _ = sa_mod::input::capture(keyboard, keyboard, false);
    }
}

/// A platform event as the window event samp's UI reads.
fn window_event(e: Input) -> WindowEvent {
    match e {
        Input::KeyDown { vk, repeat } => WindowEvent::KeyDown { vk, repeat },
        Input::KeyUp { vk } => WindowEvent::KeyUp { vk },
        Input::Char(c) => WindowEvent::Char { ch: u32::from(c), unicode: true },
        Input::MouseMove { x, y } => WindowEvent::MouseMove { x, y },
        Input::MouseButton { button, down } => WindowEvent::MouseButton { button, down },
        Input::Wheel(d) => WindowEvent::Wheel(d.clamp(-32768, 32767) as i16),
        Input::Focus(f) => WindowEvent::Focus(f),
    }
}

impl Mod for Samp {
    fn init(&mut self) -> Result<(), String> {
        samp_proto::time::set_clock(host_time);
        // What samp.dll switches off in the game; RPC 30 TogglePlayerClock turns the clock back on.
        for o in [
            GameOption::TypedCheats,
            GameOption::Clock,
            GameOption::AmbientPeds,
            GameOption::RandomTraffic,
            GameOption::CarGenerators,
            GameOption::AmbientPlanes,
            GameOption::Cutscenes,
            GameOption::Replay,
            GameOption::Plants,
            GameOption::SunAndMoon,
            GameOption::IdleCamera,
            GameOption::HelpMessages,
            GameOption::ZoneNames,
        ] {
            set_option(o, false).map_err(|e| format!("game option {o:?}: {e:?}"))?;
        }
        if setting("auto_connect").is_some_and(|v| v == "1") {
            self.stage = Stage::Approval { server: self.screen.server.clone(), name: self.screen.name.clone() };
        }
        events::subscribe(&[event_kind::FRAME_BEGIN]);
        Ok(())
    }

    fn event(&mut self, event: Event) {
        if let Event::FrameBegin(_) = event {
            if state::game_state() == Some(GameState::Playing) {
                self.approval();
            }
            self.frame(FramePhase::BeforeGameProcess);
            self.capture();
        }
    }

    fn input(&mut self, event: Input) -> bool {
        let consumed = match &mut self.stage {
            Stage::Screen => {
                if let Some(connect::Action::Connect { server, name }) = self.screen.input(event) {
                    self.screen.status = None;
                    self.stage = Stage::Approval { server, name };
                }
                !matches!(event, Input::MouseMove { .. } | Input::Focus(_))
            }
            Stage::Approval { .. } => false,
            Stage::Client(c) => c.window_event(window_event(event), &self.game) == EventResult::Consumed,
        };
        self.capture();
        consumed
    }

    fn overlay(&mut self, _frame: u32, o: &mut Overlay) {
        self.frame(FramePhase::Before2d);
        self.frame(FramePhase::After2d);
        self.frame(FramePhase::Present);
        if state::game_state() != Some(GameState::Playing) {
            return;
        }
        match &self.stage {
            Stage::Screen | Stage::Approval { .. } => self.screen.draw(o),
            Stage::Client(c) => {
                let line = format!("SA:MP {}:{} ({:?})", self.screen.server, self.screen.name, c.net().phase());
                let _ = o.text(8.0, 8.0, 12.0, Rgba::new(0xA9, 0xC4, 0xE4, 0xFF), &line);
            }
        }
    }
}

sa_mod::entry!(Samp);
