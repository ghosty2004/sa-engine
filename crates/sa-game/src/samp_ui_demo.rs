//! `--samp-ui-demo`: shows samp-rs's UI (`samp_client::ui`, drawn through the engine backend `sa_samp::overlay`)
//! in the running game without a server: sample chat lines, the chat input being typed into, a nametag above CJ
//! and the peds near him with a chat bubble, a 3D text label, textdraws, game text, the kill list, then a list
//! dialog with the cursor, a password dialog and the scoreboard. The content goes in the way a server's would
//! (`UiCore`'s RPC handlers and models); input reaches the UI as window events through the platform layer
//! (`samp_input` → `NativeGame::window_event` → the client's `GameHooks::window_event`). Evidence: run with
//! `--present-dir DIR` (offscreen PNGs).

use std::cell::RefCell;
use std::rc::Rc;

use sa_platform::input::{InputEvent, InputState, Key, MouseButton};
use sa_samp::{EngineView, NativeGame};
use sa_samp_api::camera::Camera as _;
use sa_samp_api::hooks::{FramePhase, GameHooks};
use sa_samp_api::input::{EventResult, WindowEvent};
use sa_samp_api::peds::Peds as _;
use sa_samp_api::{GameAdapter, GameState, PedRef, SampHost as _, Vec3};
use samp_client::ui::core::{Outgoing, UiCore};
use samp_client::ui::host::UiHost;
use samp_client::ui::render::{BubbleDraw, LabelDraw, TagDraw};
use samp_proto::rpcs::session::{PlayerScoreAndPing, SendPlayerScoresAndPings};
use samp_proto::session::InitGame;
use samp_proto::rpcs::ui::{SendGameText, ShowDialog, TextDrawData, textdraw_flags};
use samp_proto::rpcs::{Incoming, remote, session, ui};
use samp_proto::types::Vec2;
use samp_ui::chat::{ChatWindow, LocalTime};
use samp_ui::colour::rgba_to_argb;
use samp_ui::text::NoExpansion;

const PHASES: [FramePhase; 6] = [
    FramePhase::BeforeGameProcess,
    FramePhase::CameraProcess,
    FramePhase::AfterGameProcess,
    FramePhase::Before2d,
    FramePhase::After2d,
    FramePhase::Present,
];

/// Demo progress.
#[derive(Default)]
struct Demo {
    /// `After2d` frames seen while playing.
    frames: u32,
    /// What the UI sent (logged and echoed into the chat like a server would).
    events: Vec<String>,
    /// Echo lines held back until the dialogs are done: new lines scroll the chat, and where the sky is not
    /// drawn yet (STD-9) the old frame's pixels stay, so the evidence frames keep the chat still.
    echoes: Vec<Outgoing>,
}

/// From this playing frame on the held-back echo lines are shown.
const ECHO_FRAME: u32 = 30;

thread_local! {
    static GAME: RefCell<Option<NativeGame<EngineView>>> = const { RefCell::new(None) };
    static HOST: RefCell<Option<Rc<RefCell<UiHost>>>> = const { RefCell::new(None) };
    static DEMO: RefCell<Demo> = RefCell::new(Demo::default());
    static INPUT: RefCell<Vec<InputEvent>> = const { RefCell::new(Vec::new()) };
}

/// The demo's client: samp-rs's UI host, shared with the demo script that plays the server.
struct DemoClient(Rc<RefCell<UiHost>>);

impl GameHooks for DemoClient {
    fn frame(&mut self, phase: FramePhase, game: &mut dyn GameAdapter) {
        self.0.borrow_mut().frame(phase, game);
    }
    fn window_event(&mut self, event: WindowEvent, game: &dyn GameAdapter) -> EventResult {
        self.0.borrow_mut().window_event(event, game)
    }
}

/// A clock that advances with the demo (the chat stores the local time of each line).
pub fn demo_time(n: u32) -> LocalTime {
    LocalTime { unix: 0, hour: 20, minute: 15 + (n / 60) as u8, second: (n % 60) as u8 }
}

/// A server line (RPC 93's chat entry) in wire colour `rgba`.
pub fn info(core: &mut UiCore, text: &str, rgba: u32, time: LocalTime) {
    core.chat.add_client_message(rgba, text.as_bytes(), &NoExpansion, time);
}

/// A player's chat line (RPC 101's entries) with the name in wire colour `rgba`.
pub fn player_chat(core: &mut UiCore, name: &str, rgba: u32, text: &str, time: LocalTime) {
    core.chat.add_player_chat(name.as_bytes(), rgba_to_argb(rgba), text.as_bytes(), time);
}

/// A debug line (samp's connection lines).
pub fn debug(core: &mut UiCore, text: &str, time: LocalTime) {
    core.chat.add_debug(text.as_bytes(), time);
}

/// samp's netgame with us as player 0 and the server's settings: the connection lines in the demo's clock.
pub fn connect(core: &mut UiCore, name: &str, port: u16, server: &str, time: LocalTime) {
    core.on_netgame_start("127.0.0.1", port, name.as_bytes());
    core.on_init_game(&InitGame {
        player_id: 0,
        show_name_tags: true,
        name_tag_distance: 70.0,
        server_name: server.into(),
        ..InitGame::default()
    });
    // The lines above carry the wall clock; the evidence frames show timestamps, so they are printed again in
    // the demo's clock.
    core.chat = ChatWindow::new();
    debug(core, samp_client::ui::core::msg::STARTED, time);
    debug(core, &format!("Connecting to 127.0.0.1:{port}..."), time);
    debug(core, "Connected. Joining the game...", time);
    debug(core, &format!("Connected to {{B9C9BF}}{server}"), time);
}

/// `CRGBA` (R, G, B, A in memory) of a wire colour `0xRRGGBBAA`, as RPC 134 carries textdraw colours.
pub const fn crgba(rgba: u32) -> u32 {
    rgba.swap_bytes()
}

/// A text textdraw's RPC 134 block (`flags`: [`textdraw_flags`]).
#[allow(clippy::too_many_arguments)]
pub fn td(
    x: f32,
    y: f32,
    font: u8,
    letter: (f32, f32),
    size: (f32, f32),
    rgba: u32,
    outline: u8,
    shadow: u8,
    flags: u8,
) -> TextDrawData {
    TextDrawData {
        flags,
        letter_size: Vec2 { x: letter.0, y: letter.1 },
        letter_colour: crgba(rgba),
        text_size: Vec2 { x: size.0, y: size.1 },
        box_colour: crgba(0x8080_8080),
        shadow,
        outline,
        background_colour: crgba(0x0000_00FF),
        style: font,
        position: Vec2 { x, y },
        ..TextDrawData::default()
    }
}

/// A sprite textdraw (font 4, `txd:name`).
pub fn sprite(x: f32, y: f32, w: f32, h: f32, rgba: u32) -> TextDrawData {
    td(x, y, 4, (0.0, 0.0), (w, h), rgba, 0, 0, 0)
}

/// A server's RPC to the UI.
pub fn rpc(core: &mut UiCore, i: Incoming) {
    core.on_rpc(&i, &NoExpansion);
}

/// `GameTextForPlayer` (RPC 73).
pub fn game_text(core: &mut UiCore, text: &str, time_ms: u32, style: i32) {
    rpc(core, Incoming::Ui(ui::Incoming::SendGameText(SendGameText { style, time_ms, text: text.into() })));
}

/// `ShowPlayerDialog` (RPC 61).
pub fn dialog(core: &mut UiCore, id: i16, style: u8, title: &str, body: &str, b1: &str, b2: &str) {
    let d = ShowDialog {
        dialog_id: id,
        style,
        title: title.into(),
        button1: b1.into(),
        button2: b2.into(),
        body: body.into(),
    };
    rpc(core, Incoming::Ui(ui::Incoming::ShowDialog(d)));
}

/// A remote player joins (RPC 137) with a wire colour.
pub fn join(core: &mut UiCore, id: u16, name: &str, rgba: u32) {
    let j = remote::PlayerJoin { player_id: id, colour: rgba, is_npc: 0, name: name.into() };
    rpc(core, Incoming::Remote(remote::Incoming::PlayerJoin(j)));
}

/// Scores and pings (RPC 155).
pub fn scores(core: &mut UiCore, rows: &[(u16, i32, u32)]) {
    let players = rows.iter().map(|&(player_id, score, ping)| PlayerScoreAndPing { player_id, score, ping }).collect();
    let s = SendPlayerScoresAndPings { players };
    rpc(core, Incoming::Session(session::Incoming::SendPlayerScoresAndPings(s)));
}

/// Platform input events for the next main-loop iteration: they take the real path (`samp_input::filter`
/// → `NativeGame::window_event` → the client), as the window's events do.
fn queue(e: InputEvent) {
    INPUT.with(|q| q.borrow_mut().push(e));
}

fn key(k: Key) {
    queue(InputEvent::KeyDown { key: k, repeat: false });
    queue(InputEvent::KeyUp { key: k });
}

fn type_text(s: &str) {
    for c in s.chars() {
        queue(InputEvent::Char(c));
    }
}

fn click() {
    queue(InputEvent::MouseDown(MouseButton::Left));
    queue(InputEvent::MouseUp(MouseButton::Left));
}

/// Moves the demo's queued input into this iteration's platform input (main loop, before `samp_input`).
pub fn take_input(input: &mut InputState) {
    INPUT.with(|q| input.events.append(&mut q.borrow_mut()));
}

fn dist(a: Vec3, b: Vec3) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

/// Head height above a ped's root (the bone samp reads is the head; the root is at the pelvis).
const HEAD_ABOVE_ROOT: f32 = 0.7;

/// A ped's head position.
fn head(game: &NativeGame<EngineView>, ped: PedRef) -> Option<Vec3> {
    let p = game.ped_snapshot(ped).ok()?.matrix.pos;
    Some(Vec3::new(p.x, p.y, p.z + HEAD_ABOVE_ROOT))
}

/// What the client finds in the world each frame: nametags of CJ and the peds within 40 m (within the server's
/// nametag distance and in sight, as samp's tag pass), CJ's chat bubble, a label 6 m in front of the camera.
fn world_state(game: &mut NativeGame<EngineView>, host: &mut UiHost, label: Option<(Vec3, &str, f32)>) {
    let Some(cj) = game.local_player() else { return };
    let Ok(s) = game.ped_snapshot(cj) else { return };
    let pos = s.matrix.pos;
    let cam = game.camera_pos();
    let mut tags = Vec::new();
    let mut add = |game: &NativeGame<EngineView>, ped: PedRef, id: u16, name: &str, rgba: u32, health: f32, armour: f32, afk: bool| {
        let Some(h) = head(game, ped) else { return };
        let d = dist(h, cam);
        if d >= host.core.name_tag_distance || !sa_samp::overlay::render::line_clear(h, cam) {
            return;
        }
        tags.push(TagDraw {
            anchor: [h.x, h.y, h.z],
            distance: d,
            name: format!("{name} ({id})").into_bytes(),
            colour: rgba_to_argb(rgba),
            health,
            armour,
            afk,
        });
    };
    if label.is_none() {
        add(game, cj, 0, "Carl_Johnson", 0xFF80_00FF, s.health, 60.0, false);
        let names = ["Big_Smoke", "Ryder", "Sweet", "OG_Loc", "Cesar"];
        let colours = [0x2B8F_2BFF, 0x9966_CCFF, 0x3399_FFFF, 0xFFFF_00FF, 0xFF33_33FF];
        let peds = game.peds_near(pos, 40.0);
        for (i, p) in peds.into_iter().filter(|p| *p != cj).take(names.len()).enumerate() {
            let health = game.ped_snapshot(p).map(|s| s.health).unwrap_or(100.0);
            add(game, p, 1 + i as u16, names[i], colours[i], health.min(100.0) * 0.7, 0.0, i == 1);
        }
    }
    let mut bubbles = Vec::new();
    if let Some(b) = host.core.bubbles.get(0)
        && let Some(h) = head(game, cj)
        && dist(h, cam) < b.distance
    {
        let raise = b.anchor_raise(dist(h, cam));
        bubbles.push(BubbleDraw { anchor: [h.x, h.y, h.z + raise], text: b.text.clone(), colour: b.colour });
    }
    let mut labels = Vec::new();
    let (label_pos, text, range) = match label {
        Some(l) => l,
        None => {
            let c = game.camera();
            let (f, p) = (c.matrix.forward, c.matrix.pos);
            (
                Vec3::new(p.x + f.x * 6.0, p.y + f.y * 6.0, pos.z + 0.6),
                "{FFFF00}Grove Street\n{FFFFFF}Home of the {00FF00}Grove Street Families",
                50.0,
            )
        }
    };
    if dist(label_pos, cam) <= range {
        labels.push(LabelDraw { pos: [label_pos.x, label_pos.y, label_pos.z], text: text.into(), argb: 0xFFFF_FFFF });
    }
    host.world.tags = tags;
    host.world.bubbles = bubbles;
    host.world.labels = labels;
}

/// Static parts, set once (as a server would send them).
fn setup(core: &mut UiCore) {
    let t = demo_time(1);
    connect(core, "Carl_Johnson", 7777, "sa-engine native test server", t);
    core.colours.set(0, 0xFF80_00FF);
    for (id, name, rgba) in [
        (1, "Big_Smoke", 0x2B8F_2BFF),
        (2, "Ryder", 0x9966_CCFF),
        (3, "Sweet", 0x3399_FFFF),
        (4, "OG_Loc", 0xFFFF_00FF),
        (5, "Cesar", 0xFF33_33FF),
    ] {
        join(core, id, name, rgba);
    }
    info(core, "{FFFF00}Welcome to the {33AA33}native {FFFF00}Rust client! Type {FFFFFF}/help", 0xFFFF_FFFF, t);
    player_chat(core, "Big_Smoke", 0x2B8F_2BFF, "We should have shot that fool on the train", t);
    player_chat(
        core,
        "Sweet",
        0x3399_FFFF,
        "Grove Street, home. At least it was before I messed everything up. This line is long enough to wrap onto a second chat entry.",
        t,
    );
    info(core, "* Ryder pours a 40 for the homies", 0xC2A2_DAFF, t);
    core.deaths.add(b"Carl_Johnson", rgba_to_argb(0xFF80_00FF), b"Ballas_Member", rgba_to_argb(0xC000_FFFF), 24);
    core.deaths.add(b"", 0, b"Ryder", rgba_to_argb(0x9966_CCFF), 200);
    core.deaths.add(b"Sweet", rgba_to_argb(0x3399_FFFF), b"Vagos_Member", rgba_to_argb(0xFFFF_00FF), 31);
    let f = textdraw_flags::PROPORTIONAL;
    let tds = &mut core.textdraws;
    tds.show(10, td(498.0, 100.0, 1, (0.4, 1.6), (640.0, 640.0), 0xFFFF_FFFF, 1, 0, f), b"~r~SA~w~:~b~MP ~y~native");
    let mut boxed = td(320.0, 430.0, 2, (0.25, 1.0), (20.0, 400.0), 0xE0E0_E0FF, 0, 1, f);
    boxed.flags |= textdraw_flags::BOX | textdraw_flags::ALIGN_CENTRE;
    boxed.box_colour = crgba(0x0000_0088);
    tds.show(11, boxed, b"Rendered by sa-engine through CFont");
    // Sprite textdraws (font 4): a TXD loaded from models/txd like samp does, and a tinted box.
    tds.show(12, sprite(496.0, 116.0, 124.0, 3.0, 0xFFFF_FFC0), b"LD_SPAC:white");
    tds.show(13, sprite(8.0, 236.0, 84.0, 56.0, 0xFFFF_FFFF), b"loadsc2:loadsc2");
    game_text(core, "~y~Welcome~n~~w~to Grove Street", 6000, 3);
    let bubble = "Ah shit, here we go again.";
    core.bubbles.set(0, 0xFFFF_FFFF, 40.0, 120_000, bubble.as_bytes(), samp_client::ui::core::tick_ms());
}

fn on_frame(phase: u8) {
    let Some(&phase) = PHASES.get(usize::from(phase)) else { return };
    GAME.with(|g| {
        let mut g = g.borrow_mut();
        let Some(game) = g.as_mut() else { return };
        if phase == FramePhase::After2d && game.game_state() == GameState::Playing {
            script(game);
        }
        game.dispatch_frame(phase);
    });
}

fn host() -> Rc<RefCell<UiHost>> {
    HOST.with(|h| h.borrow().clone().expect("installed"))
}

/// What the UI sent this frame, logged.
fn drain(host: &mut UiHost) -> Vec<Outgoing> {
    let out = host.core.take_outgoing();
    for o in &out {
        let line = format!("{o:?}");
        println!("samp-ui-demo: event {line}");
        DEMO.with(|d| d.borrow_mut().events.push(line));
    }
    out
}

/// The demo script, one step per `After2d` frame while playing (before the client's `After2d` callback).
fn script(game: &mut NativeGame<EngineView>) {
    let n = DEMO.with(|d| {
        let mut d = d.borrow_mut();
        d.frames += 1;
        d.frames
    });
    let host = host();
    let mut host = host.borrow_mut();
    if std::env::var("SA_SAMP_UI_SCENE").is_ok_and(|s| s == "ref") {
        reference_scene(game, &mut host, n);
        return;
    }
    if n == 1 {
        setup(&mut host.core);
    }
    world_state(game, &mut host, None);
    let core = &mut host.core;
    match n {
        // Open the chat input with T and type (the T's own character opens it).
        3 => {
            key(Key::T);
            type_text("thello from the native client");
        }
        // Enter sends: the client sends RPC 101 and the demo echoes it like the server would.
        8 => key(Key::Enter),
        // A list dialog; Down twice selects the third row.
        10 => {
            dialog(
                core,
                1,
                5,
                "{FFFFFF}Ammu-Nation",
                "Weapon\tPrice\tAmmo\n{FFFFFF}Desert Eagle\t{00FF00}$1200\t21\n{FFFFFF}Shotgun\t{00FF00}$600\t15\n{FFFFFF}MP5\t{00FF00}$2000\t90\n{FFFFFF}M4\t{FF0000}$4500\t150\n{FFFFFF}Sniper Rifle\t{00FF00}$5000\t10",
                "Buy",
                "Close",
            );
            queue(InputEvent::MouseMove { x: 300, y: 380 });
            key(Key::Down);
            key(Key::Down);
        }
        // Enter answers the list; a password dialog follows.
        15 => key(Key::Enter),
        16 => {
            dialog(
                core,
                2,
                3,
                "Login",
                "Welcome back, {FF8000}Carl_Johnson{A9C4E4}.\nEnter your password to log in:",
                "Login",
                "Quit",
            );
            type_text("grove4life");
        }
        20 => key(Key::Enter),
        // The scoreboard (TAB) with the score RPC's rows; the cursor hovers a row.
        24 => {
            scores(core, &[(0, 12, 31), (1, 8, 45), (2, 5, 120), (3, 9, 38), (4, 1, 230), (5, 4, 64)]);
            key(Key::Tab);
            queue(InputEvent::MouseMove { x: 420, y: 232 });
        }
        // A double click on a row answers with its player id (RPC 23) and closes the scoreboard.
        28 => {
            click();
            click();
        }
        29 => {
            if core.scoreboard.is_open() {
                key(Key::Tab);
            }
        }
        // `/timestamp` and `/pagesize 14` as samp-rs applies them, then F6 opens the input (the scrollbar shows
        // while it is open) and Page Up scrolls the history.
        32 => {
            core.chat.set_timestamps(true);
            core.chat.set_page_size(14);
            key(Key::Fn(6));
            type_text("/pagesize 14");
        }
        34 => key(Key::PageUp),
        _ => {}
    }
    let out = drain(&mut host);
    DEMO.with(|d| d.borrow_mut().echoes.extend(out));
    if n < ECHO_FRAME {
        return;
    }
    let t = demo_time(n);
    for o in DEMO.with(|d| std::mem::take(&mut d.borrow_mut().echoes)) {
        let core = &mut host.core;
        match o {
            Outgoing::Chat(text) => {
                player_chat(core, "Carl_Johnson", 0xFF80_00FF, &String::from_utf8_lossy(&text), t);
            }
            Outgoing::DialogResponse(r) if r.dialog_id == 1 && r.response == 1 => {
                let text = String::from_utf8_lossy(&r.text);
                info(core, &format!("You bought a {{FFFFFF}}{text}{{33AA33}}."), 0x33AA_33FF, t);
            }
            Outgoing::DialogResponse(r) if r.dialog_id == 2 => {
                info(core, &format!("Logged in ({} characters).", r.text.len()), 0x33AA_33FF, t);
            }
            _ => {}
        }
    }
}

/// `SA_SAMP_UI_SCENE=ref`: the content of the reference gamemode (`crate::samp_ui_scene`) instead of the demo.
fn reference_scene(game: &mut NativeGame<EngineView>, host: &mut UiHost, n: u32) {
    if n == 1 {
        crate::samp_ui_scene::setup(&mut host.core);
    }
    let cam = game.camera();
    let (p, f) = (cam.matrix.pos, cam.matrix.forward);
    if n == 1 {
        let z = game.local_player().and_then(|cj| game.ped_snapshot(cj).ok()).map_or(p.z, |s| s.matrix.pos.z + 1.0);
        LABEL.with(|l| l.set(Some(Vec3::new(p.x + f.x * 7.0 + 1.5, p.y + f.y * 7.0, z))));
    }
    if let Some(l) = LABEL.with(|l| l.get()) {
        world_state(game, host, Some((l, crate::samp_ui_scene::LABEL, 40.0)));
    }
    let (w, h) = sa_samp::overlay::render::screen_size();
    let mut input = Vec::new();
    crate::samp_ui_scene::step(n, w, h, &mut host.core, &mut input);
    for e in input {
        queue(e);
    }
    drain(host);
}

thread_local! {
    static LABEL: std::cell::Cell<Option<Vec3>> = const { std::cell::Cell::new(None) };
}

/// Builds the adapter and registers the demo's frame handler.
pub fn install(game_dir: &std::path::Path) -> Result<(), String> {
    let host = Rc::new(RefCell::new(UiHost::new(None)));
    let mut game = sa_samp::engine();
    game.install(&sa_samp_api::config::SampModeConfig::default(), Box::new(DemoClient(host.clone())), None)
        .map_err(|e| format!("samp-ui-demo: install: {e}"))?;
    crate::samp_assets::install(game_dir, game.overlay());
    GAME.with(|g| *g.borrow_mut() = Some(game));
    HOST.with(|h| *h.borrow_mut() = Some(host));
    sa_core::samp::set_frame_handler(on_frame);
    crate::samp_input::set_target(crate::samp_input::Target { event: ui_event, lock: ui_lock });
    println!("sa-game: samp-ui demo: samp-rs's SA:MP UI on the engine backend with sample content (no server)");
    Ok(())
}

/// The UI's input path (`samp_input`): a window event to the adapter.
fn ui_event(e: WindowEvent) -> EventResult {
    GAME.with(|g| match g.borrow_mut().as_mut() {
        Some(game) => game.window_event(e),
        None => EventResult::PassToGame,
    })
}

/// samp's input lock (keyboard, mouse) from the UI.
fn ui_lock() -> (bool, bool) {
    GAME.with(|g| g.borrow().as_ref().map(|g| g.ui_input_lock()).unwrap_or_default())
}

/// Summary at the end of the run.
pub fn report(md: &mut String) {
    use std::fmt::Write as _;
    sa_core::samp::clear_frame_handler();
    let (frames, uploaded, sprites) = GAME.with(|g| {
        g.borrow_mut()
            .as_mut()
            .map(|g| (g.overlay().frames, g.overlay().atlas_uploaded(), g.overlay().sprites_loaded()))
            .unwrap_or_default()
    });
    DEMO.with(|d| {
        let d = d.borrow();
        let _ = writeln!(
            md,
            "\n## SA:MP UI demo\n\nOverlay frames drawn: {frames}; glyph atlas uploaded: {uploaded}; sprite textures: {sprites}; UI events: {}\n",
            d.events.len()
        );
        for e in &d.events {
            let _ = writeln!(md, "- {e}");
        }
        println!(
            "samp-ui-demo: {} playing frames, {frames} overlay frames, atlas uploaded: {uploaded}, sprites: {sprites}, {} events",
            d.frames,
            d.events.len()
        );
    });
}
