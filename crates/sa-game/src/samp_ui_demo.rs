//! `--samp-ui-demo`: shows the engine-drawn SA:MP UI (`sa-samp-ui` through `sa_samp::overlay`) in the
//! running game without a server: sample chat lines, the chat input being typed into, a nametag above CJ and
//! the peds near him, a 3D text label, a textdraw, game text, the kill list, then a list dialog with the
//! cursor and a password dialog. Input reaches the UI as window events, as the platform layer will deliver
//! them (`NativeGame::window_event`). Evidence: run with `--present-dir DIR` (offscreen PNGs).
//!
//! What a real client does instead: samp-rs's `GameHooks::frame` calls `game.ui()` with the RPC state and
//! drains `poll_events()` (docs/samp/overlay.md).

use std::cell::RefCell;

use sa_platform::input::{InputEvent, InputState, Key, MouseButton};
use sa_samp::{EngineView, NativeGame};
use sa_samp_api::camera::Camera as _;
use sa_samp_api::hooks::FramePhase;
use sa_samp_api::peds::Peds as _;
use sa_samp_api::ui::{
    ChatKind, ChatMessage, DialogSpec, DialogStyle, Nametag, NametagAnchor, SampUi, ScoreRow, TdAlign,
    TextDrawSpec, TextLabel3d, UiEvent,
};
use sa_samp_api::{GameAdapter as _, GameState, Rgba, SampHost as _, Vec3};

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
    /// Events the UI produced (logged and echoed into the chat like a server would).
    events: Vec<String>,
    /// Echo lines held back until the dialogs are done: new lines scroll the chat, and where the sky is not
    /// drawn yet (STD-9) the old frame's pixels stay, so the evidence frames keep the chat still.
    echoes: Vec<UiEvent>,
}

/// From this playing frame on the held-back echo lines are shown.
const ECHO_FRAME: u32 = 30;

thread_local! {
    static GAME: RefCell<Option<NativeGame<EngineView>>> = const { RefCell::new(None) };
    static DEMO: RefCell<Demo> = RefCell::new(Demo::default());
    static INPUT: RefCell<Vec<InputEvent>> = const { RefCell::new(Vec::new()) };
}

/// The client does nothing in the demo: the frame handler below plays samp-rs's part.
struct NoClient;
impl sa_samp_api::hooks::GameHooks for NoClient {}

fn chat(ui: &mut dyn SampUi, kind: ChatKind, name: &str, name_rgba: u32, text: &str, rgba: u32) {
    ui.chat_add(&ChatMessage {
        kind,
        name: name.into(),
        name_colour: Rgba::from_rgba_u32(name_rgba),
        text: text.into(),
        colour: Rgba::from_rgba_u32(rgba),
    });
}

/// Platform input events for the next main-loop iteration: they take the real path (`samp_input::filter`
/// → `NativeGame::window_event` → the UI), as the window's events do.
fn queue(e: InputEvent) {
    INPUT.with(|q| q.borrow_mut().push(e));
}

fn key(k: Key) {
    queue(InputEvent::KeyDown {
        key: k,
        repeat: false,
    });
    queue(InputEvent::KeyUp { key: k });
}

fn type_text(s: &str) {
    for c in s.chars() {
        queue(InputEvent::Char(c));
    }
}

/// Moves the demo's queued input into this iteration's platform input (main loop, before `samp_input`).
pub fn take_input(input: &mut InputState) {
    INPUT.with(|q| input.events.append(&mut q.borrow_mut()));
}

/// The world-anchored parts: nametags above CJ and the peds within 40 m, a label 6 m in front of the camera.
fn world_state(game: &mut NativeGame<EngineView>) {
    let Some(cj) = game.local_player() else { return };
    let Ok(s) = game.ped_snapshot(cj) else { return };
    let pos = s.matrix.pos;
    let mut tags = vec![Nametag {
        player_id: 0,
        anchor: NametagAnchor::Ped(cj),
        name: "Carl_Johnson".into(),
        colour: Rgba::from_rgba_u32(0xFF80_00FF),
        health: s.health,
        armour: 60.0,
        afk: false,
    }];
    let names = ["Big_Smoke", "Ryder", "Sweet", "OG_Loc", "Cesar"];
    let peds = game.peds_near(pos, 40.0);
    for (i, p) in peds
        .into_iter()
        .filter(|p| *p != cj)
        .take(names.len())
        .enumerate()
    {
        let health = game.ped_snapshot(p).map(|s| s.health).unwrap_or(100.0);
        tags.push(Nametag {
            player_id: 1 + i as u16,
            anchor: NametagAnchor::Ped(p),
            name: names[i].into(),
            colour: Rgba::from_rgba_u32([0x2B8F_2BFF, 0x9966_CCFF, 0x3399_FFFF, 0xFFFF_00FF, 0xFF33_33FF][i]),
            health: health.min(100.0) * 0.7,
            armour: 0.0,
            afk: i == 1,
        });
    }
    let cam = game.camera();
    let f = cam.matrix.forward;
    let p = cam.matrix.pos;
    let label_pos = Vec3::new(p.x + f.x * 6.0, p.y + f.y * 6.0, pos.z + 0.6);
    let Some(ui) = game.ui() else { return };
    ui.set_nametags(&tags);
    ui.set_label(
        1,
        Some(&TextLabel3d {
            text: "{FFFF00}Grove Street\n{FFFFFF}Home of the {00FF00}Grove Street Families".into(),
            colour: Rgba::from_rgba_u32(0xFFFF_FFFF),
            pos: label_pos,
            draw_distance: 50.0,
            test_los: false,
            attached_ped: None,
        }),
    );
}

/// Static parts, set once.
fn setup(ui: &mut dyn SampUi) {
    chat(
        ui,
        ChatKind::Info,
        "",
        0,
        "{FFFFFF}SA-MP {B9C9BF}0.3.7-R4 {FFFFFF}Started",
        0xFFFF_FFFF,
    );
    chat(
        ui,
        ChatKind::Info,
        "",
        0,
        "Connecting to 127.0.0.1:7777...",
        0xA9C4_E4FF,
    );
    chat(
        ui,
        ChatKind::Info,
        "",
        0,
        "Connected. Joining the game...",
        0xA9C4_E4FF,
    );
    chat(
        ui,
        ChatKind::Info,
        "",
        0,
        "Connected to {B9C9BF}sa-engine native test server",
        0xA9C4_E4FF,
    );
    chat(
        ui,
        ChatKind::Info,
        "",
        0,
        "{FFFF00}Welcome to the {33AA33}native {FFFF00}Rust client! Type {FFFFFF}/help",
        0xFFFF_FFFF,
    );
    chat(
        ui,
        ChatKind::Player,
        "Big_Smoke",
        0x2B8F_2BFF,
        "We should have shot that fool on the train",
        0xFFFF_FFFF,
    );
    chat(
        ui,
        ChatKind::Player,
        "Sweet",
        0x3399_FFFF,
        "Grove Street, home. At least it was before I messed everything up. This line is long enough to wrap onto a second chat entry.",
        0xFFFF_FFFF,
    );
    chat(
        ui,
        ChatKind::Info,
        "",
        0,
        "* Ryder pours a 40 for the homies",
        0xC2A2_DAFF,
    );
    ui.kill_entry(
        "Carl_Johnson",
        Rgba::from_rgba_u32(0xFF80_00FF),
        "Ballas_Member",
        Rgba::from_rgba_u32(0xC000_FFFF),
        24,
    );
    ui.kill_entry(
        "",
        Rgba::default(),
        "Ryder",
        Rgba::from_rgba_u32(0x9966_CCFF),
        200,
    );
    ui.kill_entry(
        "Sweet",
        Rgba::from_rgba_u32(0x3399_FFFF),
        "Vagos_Member",
        Rgba::from_rgba_u32(0xFFFF_00FF),
        31,
    );
    ui.set_textdraw(
        10,
        Some(&TextDrawSpec {
            text: "~r~SA~w~:~b~MP ~y~native".into(),
            x: 498.0,
            y: 100.0,
            letter_w: 0.4,
            letter_h: 1.6,
            text_w: 640.0,
            text_h: 640.0,
            colour: Rgba::from_rgba_u32(0xFFFF_FFFF),
            use_box: false,
            box_colour: Rgba::default(),
            shadow: 0,
            outline: 1,
            background_colour: Rgba::from_rgba_u32(0x0000_00FF),
            font: 1,
            proportional: true,
            align: TdAlign::Left,
            selectable: false,
        }),
    );
    ui.set_textdraw(
        11,
        Some(&TextDrawSpec {
            text: "Rendered by sa-engine through CFont".into(),
            x: 320.0,
            y: 430.0,
            letter_w: 0.25,
            letter_h: 1.0,
            text_w: 20.0,
            text_h: 400.0,
            colour: Rgba::from_rgba_u32(0xE0E0_E0FF),
            use_box: true,
            box_colour: Rgba::from_rgba_u32(0x0000_0088),
            shadow: 1,
            outline: 0,
            background_colour: Rgba::from_rgba_u32(0x0000_00FF),
            font: 2,
            proportional: true,
            align: TdAlign::Centre,
            selectable: false,
        }),
    );
    // Sprite textdraws (font 4): a TXD loaded from models/txd like samp does, and a tinted box.
    let sprite = |text: &str, x: f32, y: f32, w: f32, h: f32, rgba: u32| TextDrawSpec {
        text: text.into(),
        x,
        y,
        letter_w: 0.0,
        letter_h: 0.0,
        text_w: w,
        text_h: h,
        colour: Rgba::from_rgba_u32(rgba),
        use_box: false,
        box_colour: Rgba::default(),
        shadow: 0,
        outline: 0,
        background_colour: Rgba::default(),
        font: 4,
        proportional: false,
        align: TdAlign::Left,
        selectable: false,
    };
    ui.set_textdraw(
        12,
        Some(&sprite("LD_SPAC:white", 496.0, 116.0, 124.0, 3.0, 0xFFFF_FFC0)),
    );
    ui.set_textdraw(
        13,
        Some(&sprite("loadsc2:loadsc2", 8.0, 236.0, 84.0, 56.0, 0xFFFF_FFFF)),
    );
    ui.game_text("~y~Welcome~n~~w~to Grove Street", 6000, 3);
}

/// The UI's input path (`samp_input`): a window event to the adapter.
fn ui_event(e: sa_samp_api::input::WindowEvent) -> sa_samp_api::input::EventResult {
    GAME.with(|g| match g.borrow_mut().as_mut() {
        Some(game) => game.window_event(e),
        None => sa_samp_api::input::EventResult::PassToGame,
    })
}

/// Whether the UI has the keyboard.
fn ui_wants_keyboard() -> bool {
    GAME.with(|g| {
        g.borrow_mut()
            .as_mut()
            .and_then(|g| g.ui().map(|u| u.wants_keyboard()))
            .unwrap_or(false)
    })
}

fn on_frame(phase: u8) {
    let Some(&phase) = PHASES.get(usize::from(phase)) else {
        return;
    };
    GAME.with(|g| {
        let mut g = g.borrow_mut();
        let Some(game) = g.as_mut() else { return };
        if phase == FramePhase::After2d && game.game_state() == GameState::Playing {
            script(game);
        }
        game.dispatch_frame(phase);
    });
}

/// The demo script, one step per `After2d` frame while playing (before the overlay draws this frame).
fn script(game: &mut NativeGame<EngineView>) {
    let n = DEMO.with(|d| {
        let mut d = d.borrow_mut();
        d.frames += 1;
        d.frames
    });
    if n == 1
        && let Some(ui) = game.ui()
    {
        setup(ui);
    }
    world_state(game);
    match n {
        // Open the chat input with T and type (the T's own character is swallowed).
        3 => {
            key(Key::T);
            // The window delivers the T key and then its character, which the UI swallows.
            type_text("thello from the native client");
        }
        // Enter sends: the client gets a ChatSubmit event and echoes it like the server would.
        8 => key(Key::Enter),
        // A list dialog; Down twice selects the third row.
        10 => {
            if let Some(ui) = game.ui() {
                ui.show_dialog(Some(&DialogSpec {
                    id: 1,
                    style: DialogStyle::TablistHeaders,
                    title: "{FFFFFF}Ammu-Nation".into(),
                    body: "Weapon\tPrice\tAmmo\n{FFFFFF}Desert Eagle\t{00FF00}$1200\t21\n{FFFFFF}Shotgun\t{00FF00}$600\t15\n{FFFFFF}MP5\t{00FF00}$2000\t90\n{FFFFFF}M4\t{FF0000}$4500\t150\n{FFFFFF}Sniper Rifle\t{00FF00}$5000\t10".into(),
                    button1: "Buy".into(),
                    button2: "Close".into(),
                }));
            }
            queue(InputEvent::MouseMove { x: 300, y: 380 });
            key(Key::Down);
            key(Key::Down);
        }
        // Enter answers the list; a password dialog follows.
        15 => key(Key::Enter),
        16 => {
            if let Some(ui) = game.ui() {
                ui.show_dialog(Some(&DialogSpec {
                    id: 2,
                    style: DialogStyle::Password,
                    title: "Login".into(),
                    body: "Welcome back, {FF8000}Carl_Johnson{A9C4E4}.\nEnter your password to log in:"
                        .into(),
                    button1: "Login".into(),
                    button2: "Quit".into(),
                }));
            }
            type_text("grove4life");
        }
        20 => key(Key::Enter),
        // The scoreboard (samp-rs opens it on TAB with the score RPC's rows); the cursor hovers a row.
        24 => {
            let rows: Vec<ScoreRow> = [
                (0, "Carl_Johnson", 0xFF80_00FF, 12, 31),
                (1, "Big_Smoke", 0x2B8F_2BFF, 8, 45),
                (2, "Ryder", 0x9966_CCFF, 5, 120),
                (3, "Sweet", 0x3399_FFFF, 9, 38),
                (4, "OG_Loc", 0xFFFF_00FF, 1, 230),
                (5, "Cesar", 0xFF33_33FF, 4, 64),
            ]
            .into_iter()
            .map(|(id, name, c, score, ping)| ScoreRow {
                id,
                name: name.into(),
                score,
                ping,
                colour: Rgba::from_rgba_u32(c),
            })
            .collect();
            if let Some(ui) = game.ui() {
                ui.set_scoreboard(Some(("sa-engine native test server", &rows)));
            }
            queue(InputEvent::MouseMove { x: 420, y: 232 });
        }
        // Clicking a row answers with its player id; then the scoreboard closes.
        28 => {
            queue(InputEvent::MouseDown(MouseButton::Left));
            queue(InputEvent::MouseUp(MouseButton::Left));
        }
        29 => {
            if let Some(ui) = game.ui() {
                ui.set_scoreboard(None);
            }
        }
        _ => {}
    }
    // The client side: drain the events and answer them like a server would.
    let events = game.ui().map(|u| u.poll_events()).unwrap_or_default();
    for e in events {
        let line = format!("{e:?}");
        println!("samp-ui-demo: event {line}");
        DEMO.with(|d| {
            let mut d = d.borrow_mut();
            d.events.push(line);
            d.echoes.push(e);
        });
    }
    if n < ECHO_FRAME {
        return;
    }
    for e in DEMO.with(|d| std::mem::take(&mut d.borrow_mut().echoes)) {
        let Some(ui) = game.ui() else { continue };
        match e {
            UiEvent::ChatSubmit(t) => {
                chat(ui, ChatKind::Player, "Carl_Johnson", 0xFF80_00FF, &t, 0xFFFF_FFFF)
            }
            UiEvent::DialogResponse {
                id: 1,
                button: 1,
                text,
                ..
            } => chat(
                ui,
                ChatKind::Info,
                "",
                0,
                &format!("You bought a {{FFFFFF}}{text}{{33AA33}}."),
                0x33AA_33FF,
            ),
            UiEvent::DialogResponse { id: 2, text, .. } => chat(
                ui,
                ChatKind::Info,
                "",
                0,
                &format!("Logged in ({} characters).", text.chars().count()),
                0x33AA_33FF,
            ),
            _ => {}
        }
    }
}

/// Builds the adapter and registers the demo's frame handler.
pub fn install() -> Result<(), String> {
    let mut game = sa_samp::engine();
    game.install(
        &sa_samp_api::config::SampModeConfig::default(),
        Box::new(NoClient),
        None,
    )
    .map_err(|e| format!("samp-ui-demo: install: {e}"))?;
    GAME.with(|g| *g.borrow_mut() = Some(game));
    sa_core::samp::set_frame_handler(on_frame);
    crate::samp_input::set_target(crate::samp_input::Target {
        event: ui_event,
        wants_keyboard: ui_wants_keyboard,
    });
    println!("sa-game: samp-ui demo: the engine-drawn SA:MP UI with sample content (no server)");
    Ok(())
}

/// Summary at the end of the run.
pub fn report(md: &mut String) {
    use std::fmt::Write as _;
    sa_core::samp::clear_frame_handler();
    let (frames, uploaded, sprites) = GAME.with(|g| {
        g.borrow_mut()
            .as_mut()
            .map(|g| {
                (
                    g.overlay().frames,
                    g.overlay().atlas_uploaded(),
                    g.overlay().sprites_loaded(),
                )
            })
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
