//! SA:MP as a mod: samp-rs's client (`samp_client::hosted`) running inside the game's mod host. The client reaches
//! the game through [`game::ModGame`] (its game adapter over the mod API), its server through the mod's UDP sockets
//! ([`transport::ModConnector`]; in the browser the host relays them through the page's UDP proxy) and runs on the
//! host's clock.
//!
//! Once the game runs its world the mod covers it with its connect screen ([`connect::Screen`]), a server browser,
//! until the server accepted the join. The manifest asks for the servers the user approves
//! (`net = ["udp:user-approved"]`): the first connection to a server makes the host ask the user, and the mod waits
//! for the answer; the Internet list comes over HTTP (`http = [...]`). Its storage holds (a file of the same name in
//! the mod's folder stands in for a missing key: a page's storage starts empty):
//!
//! * `server`, `name`, `favourites`, `recent`, `server_list_url`, `query_allowed`: the connect screen's
//!   ([`connect`]);
//! * `auth_table.txt`: a servers' auth table replacing the built-in one;
//! * `auto_connect`: `1` joins `server` as `name` without the screen, `auto_spawn`: `1` presses Shift in class
//!   selection by itself (headless runs);
//! * `ui_scene`: `ref` shows the UI's fixed reference scene instead (no server; comparisons with screenshots);
//! * `serial_seed`: made once, from the clock, never from user or host data.

mod assets;
mod canvas;
mod connect;
mod game;
mod setup;
mod transport;

use std::time::Duration;

use sa_mod::input::Input;
use sa_mod::net::Socket;
use sa_mod::game::{GameOption, set_option};
use sa_mod::{Error, Event, GameState, Mod, Overlay, Vec3, event_kind, events, state, storage, time};
use sa_samp_api::hooks::{FramePhase, GameHooks};
use sa_samp_api::input::{EventResult, WindowEvent};
use sa_samp_api::world::WorldState;

use connect::Tone;
use samp_client::hosted::{HostedClient, HostedConfig};
use samp_client::ui::host::UiHost;
use samp_client::ui::render::LabelDraw;
use samp_client::ui::scene;
use samp_client::netclient::NetClient;
use samp_proto::auth::AuthTable;
use samp_proto::time::HostTime;

const DEFAULT_SERVER: &str = "127.0.0.1:7777";
const DEFAULT_NAME: &str = "ModCJ";

fn host_time() -> HostTime {
    let now = time::now();
    HostTime { monotonic: Duration::from_micros(now.monotonic_us), wall: Duration::from_micros(now.unix_us) }
}

/// A setting from the mod's storage, else from the file of that name in the mod's folder (how a page hands them
/// over: its storage starts empty).
fn setting(key: &str) -> Option<String> {
    let v = match storage::read(key).ok().flatten() {
        Some(v) => v,
        None => sa_mod::files::read(key).ok().flatten()?,
    };
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

/// The servers' auth table: the built-in one, or an `auth_table.txt` ([`setting`]) that replaces it.
fn auth_table() -> Result<AuthTable, String> {
    match setting("auth_table.txt") {
        Some(t) => AuthTable::parse(&t).map_err(|e| format!("auth_table.txt: {e:?}")),
        None => Ok(AuthTable::builtin()),
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
    /// The client runs; the screen covers the world until the server accepted the join.
    Client {
        client: Box<HostedClient>,
        server: String,
        /// When it started and when the join was accepted (host microseconds).
        since: u64,
        joined: Option<u64>,
    },
    /// The UI's fixed reference scene, without a server (`ui_scene` = `ref`).
    Scene(Box<Scene>),
}

/// samp-rs's UI showing [`samp_client::ui::scene`].
struct Scene {
    ui: UiHost,
    /// Overlay frames drawn while the game runs its world.
    frames: u32,
    /// The 3D label's place, fixed in front of the camera at the first frame.
    label: Option<Vec3>,
}

impl Scene {
    /// The scene's frame `n`: the step's content and input, the UI's passes.
    fn frame(&mut self, game: &mut game::ModGame) {
        self.frames += 1;
        let n = self.frames;
        if n == 1 {
            scene::setup(&mut self.ui.core);
            if let Some(c) = state::camera() {
                let z = state::player().filter(|p| p.handle.is_some()).map_or(c.pos.z, |p| p.ped.pos.z + 1.0);
                self.label = Some(Vec3::new(c.pos.x + c.front.x * 7.0 + 1.5, c.pos.y + c.front.y * 7.0, z));
            }
        }
        self.ui.world.labels = self
            .label
            .iter()
            .map(|l| LabelDraw { pos: [l.x, l.y, l.z], text: scene::LABEL.into(), argb: 0xFFFF_FFFF })
            .collect();
        use sa_samp_api::ui::UiBackend as _;
        let (w, h) = game.ui.video_size();
        let mut input = Vec::new();
        if let Some(name) = scene::step(n, w as f32, h as f32, &mut self.ui.core, &mut input) {
            sa_mod::info(&format!("samp: ui scene frame {n}: {name}"));
        }
        for e in input {
            if let WindowEvent::MouseMove { x, y } = e {
                game.ui.set_cursor(x, y);
            }
            self.ui.window_event(e, game);
        }
        for phase in [FramePhase::Before2d, FramePhase::After2d, FramePhase::Present] {
            self.ui.frame(phase, game);
        }
        self.ui.core.take_outgoing();
    }
}

struct Samp {
    stage: Stage,
    screen: connect::Screen,
    game: game::ModGame,
    images: canvas::Images,
}

impl Default for Samp {
    fn default() -> Samp {
        let mut game = game::ModGame::default();
        game.ui.set_projector(Box::new(|p| {
            state::world_to_screen(Vec3::new(p[0], p[1], p[2])).map(|(x, y, _)| (x, y))
        }));
        Samp {
            stage: Stage::Screen,
            screen: connect::Screen::load(DEFAULT_SERVER, DEFAULT_NAME),
            game,
            images: Default::default(),
        }
    }
}

impl Samp {
    /// One of samp's frame points; the client runs only while the game runs its world (samp hooks
    /// `CGame::Process`, which the start-up and the loading never reach).
    fn frame(&mut self, phase: FramePhase) {
        if state::game_state() != Some(GameState::Playing) {
            return;
        }
        if let Stage::Client { client, .. } = &mut self.stage {
            client.frame(phase, &mut self.game);
        }
    }

    /// Whether the server the user chose may be reached yet: a socket opens once the host has the user's yes.
    fn approval(&mut self) {
        let Stage::Approval { server, name } = &self.stage else { return };
        match Socket::open(server) {
            Ok(probe) => {
                drop(probe);
                self.screen.joining(server);
                match start(server, name) {
                    Ok(c) => {
                        let server = server.clone();
                        self.stage = Stage::Client { client: Box::new(c), server, since: time::now().monotonic_us, joined: None };
                    }
                    Err(e) => {
                        self.screen.status = Some((Tone::Error, e));
                        self.stage = Stage::Screen;
                    }
                }
            }
            Err(Error::Again) => {}
            Err(e) => {
                self.screen.status = Some((
                    Tone::Error,
                    match e {
                        Error::Denied => format!("{server}: not allowed (you denied it in the game's question)"),
                        e => format!("{server}: {e:?}"),
                    },
                ));
                self.stage = Stage::Screen;
            }
        }
    }

    /// The keyboard while the connect screen, the chat input or a dialog is up; the mouse and the platform's
    /// cursor while samp's UI shows its cursor (a dialog, the scoreboard).
    fn capture(&self) {
        let (keyboard, mouse, cursor) = match &self.stage {
            // the platform's pointer for the screen's mouse
            Stage::Screen => (true, true, true),
            Stage::Approval { .. } => (true, true, false),
            Stage::Client { client, joined: None, .. } if !client.connected() => (true, true, false),
            Stage::Client { client, .. } => {
                let (_, mouse, cursor) = self.game.ui.input_state();
                (client.ui.wants_keyboard(), mouse, cursor)
            }
            Stage::Scene(_) => (false, false, false),
        };
        // samp's own cursor (mouse.png) when the skin is there, else the platform's
        let platform_cursor = cursor && (matches!(self.stage, Stage::Screen) || !self.game.ui.draws_cursor());
        let _ = sa_mod::input::capture(keyboard, mouse || keyboard, platform_cursor);
    }
}

/// How long the screen takes to fade out once the server accepted the join.
const COVER_FADE_US: u64 = 700_000;

/// A chat line without samp's `{RRGGBB}` colour codes.
fn strip_colours(text: &[u8]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < text.len() {
        if text[i] == b'{' && text.get(i + 7) == Some(&b'}') && text[i + 1..i + 7].iter().all(u8::is_ascii_hexdigit) {
            i += 8;
            continue;
        }
        out.push(char::from(text[i]));
        i += 1;
    }
    out
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
        assets::install(&mut self.game.ui);
        // the fonts are rasterised here, where the host allows the most work (the game's default video mode and
        // the desktop's)
        let desktop = sa_mod::display::desktop();
        self.game.ui.prepare(&[800, desktop.0 as i32]);
        self.screen.prepare(&[(800, 600), desktop]);
        // the reference scene is the UI alone, over the unchanged single-player world
        if setting("ui_scene").is_some_and(|v| v == "ref") {
            self.stage = Stage::Scene(Box::new(Scene { ui: UiHost::new(None), frames: 0, label: None }));
            events::subscribe(&[event_kind::FRAME_BEGIN]);
            return Ok(());
        }
        // What samp.dll switches off in the game (its start-up skips the logos, the intro movie and the main menu);
        // RPC 30 TogglePlayerClock turns the clock back on.
        for o in [
            GameOption::StartScreens,
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
        // samp.saa's map, the server's vehicles and objects and the other players need samp's pool sizes
        setup::raise_limits()?;
        if setting("auto_connect").is_some_and(|v| v == "1") {
            self.stage = Stage::Approval { server: self.screen.server.clone(), name: self.screen.name.clone() };
        }
        events::subscribe(&[event_kind::FRAME_BEGIN]);
        Ok(())
    }

    fn event(&mut self, event: Event) {
        if let Event::FrameBegin(_) = event {
            if state::game_state() == Some(GameState::Playing) {
                // samp's own main.scm keeps the densities and the wanted level cap at 0. Here the game's story
                // script still runs and raises the cap (a locked city then gives wanted level 4), and with
                // ambient peds off the population code dereferences the NULL `AddPed` returns for every cop or
                // pedestrian it tries to create, so both are forced back every frame.
                let _ = self.game.set_densities(0.0, 0.0);
                let _ = self.game.set_max_wanted_level(0);
                self.approval();
            }
            self.frame(FramePhase::BeforeGameProcess);
            self.capture();
        }
    }

    fn input(&mut self, event: Input) -> bool {
        if let Input::MouseMove { x, y } = event {
            self.game.ui.set_cursor(x, y);
        }
        let consumed = match &mut self.stage {
            Stage::Screen => {
                if let Some(connect::Action::Connect { server, name }) = self.screen.input(event) {
                    self.screen.status = None;
                    self.stage = Stage::Approval { server, name };
                }
                !matches!(event, Input::MouseMove { .. } | Input::Focus(_))
            }
            Stage::Approval { .. } => false,
            Stage::Client { client, joined: None, .. } if !client.connected() => {
                if matches!(event, Input::KeyDown { vk: 0x1B, .. }) {
                    self.screen.status = Some((Tone::Warning, "Cancelled".into()));
                    self.stage = Stage::Screen;
                }
                !matches!(event, Input::Focus(_))
            }
            Stage::Client { client, .. } => client.window_event(window_event(event), &self.game) == EventResult::Consumed,
            Stage::Scene(_) => false,
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
        match &mut self.stage {
            Stage::Screen => {
                let _ = sa_mod::game::set_hud_visible(false);
                self.screen.draw(o);
            }
            Stage::Approval { server, .. } => {
                let _ = sa_mod::game::set_hud_visible(false);
                self.screen.draw_progress(o, &connect::Progress::Approval { server }, 1.0);
            }
            Stage::Scene(s) => {
                s.frame(&mut self.game);
                let hide = state::menu_active();
                self.game.ui.flush(&mut canvas::ModCanvas { overlay: o, images: &mut self.images }, hide);
            }
            Stage::Client { client, server, since, joined } => {
                let now = time::now().monotonic_us;
                if client.connected() && joined.is_none() {
                    *joined = Some(now);
                }
                let alpha = joined.map_or(1.0, |t| 1.0 - now.saturating_sub(t) as f32 / COVER_FADE_US as f32);
                // samp's UI only once the cover fades: under the cover it would show through the veil
                let hide = state::menu_active() || alpha >= 1.0;
                self.game.ui.flush(&mut canvas::ModCanvas { overlay: o, images: &mut self.images }, hide);
                if alpha > 0.0 {
                    let message = client
                        .ui
                        .core
                        .chat
                        .last()
                        .map(|e| strip_colours(&e.text))
                        .filter(|m| !m.ends_with("Started"));
                    let p = connect::Progress::Joining { server, since_us: *since, message };
                    self.screen.draw_progress(o, &p, alpha);
                }
            }
        }
    }
}

sa_mod::entry!(Samp);
