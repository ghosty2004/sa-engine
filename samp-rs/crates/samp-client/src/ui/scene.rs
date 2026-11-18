//! A fixed UI scene without a server: the content of the reference gamemode `tools/e2e/samp/sampui.pwn` (which
//! `tools/e2e/samp-ui-ref.sh` shows on the original client) fed into [`UiCore`] the way the server's RPCs would be,
//! so samp-rs's UI on any backend can be compared with screenshots: the client's own connect lines, the server's
//! chat lines, the remote player's chat, a 3D label, the textdraws, the game text and the kill list; then each
//! dialog style (the list styles with the third row selected, the input styles with text typed), the chat input
//! and the scoreboard. Keep it in step with sampui.pwn.
//!
//! [`step`] advances one step every [`HOLD`] playing frames and returns what the frame shows, so captured frames can
//! be matched to the reference shots.

use sa_samp_api::input::WindowEvent;
use samp_proto::rpcs::session::{PlayerScoreAndPing, SendPlayerScoresAndPings};
use samp_proto::rpcs::ui::{SendGameText, ShowDialog, TextDrawData, textdraw_flags as tf};
use samp_proto::rpcs::{Incoming, session, ui};
use samp_proto::session::InitGame;
use samp_proto::types::Vec2;
use samp_ui::chat::{ChatWindow, LocalTime};
use samp_ui::colour::rgba_to_argb;
use samp_ui::text::NoExpansion;

use super::core::UiCore;

/// Playing frames per step.
pub const HOLD: u32 = 4;

/// Colours of the reference players.
const LOCAL: u32 = 0xFF80_00FF;
const SMOKE: u32 = 0x2B8F_2BFF;
const RYDER: u32 = 0x9966_CCFF;

/// The reference label's text.
pub const LABEL: &str = "{FFFF00}Grove Street\n{FFFFFF}Home of the {00FF00}Grove Street Families";
/// The label's draw distance.
pub const LABEL_RANGE: f32 = 40.0;

/// Windows virtual-key codes the scene presses.
mod vk {
    pub const TAB: u16 = 0x09;
    pub const ESCAPE: u16 = 0x1B;
    pub const DOWN: u16 = 0x28;
    pub const T: u16 = 0x54;
    pub const F7: u16 = 0x76;
}

/// A server line (RPC 93's chat entry) in wire colour `rgba`.
fn info(core: &mut UiCore, text: &str, rgba: u32, time: LocalTime) {
    core.chat.add_client_message(rgba, text.as_bytes(), &NoExpansion, time);
}

/// A player's chat line (RPC 101's entries) with the name in wire colour `rgba`.
fn player_chat(core: &mut UiCore, name: &str, rgba: u32, text: &str, time: LocalTime) {
    core.chat.add_player_chat(name.as_bytes(), rgba_to_argb(rgba), text.as_bytes(), time);
}

/// samp's netgame with us as player 0 and the server's settings, the connection lines at `time`.
fn connect(core: &mut UiCore, name: &str, port: u16, server: &str, time: LocalTime) {
    core.on_netgame_start("127.0.0.1", port, name.as_bytes());
    core.on_init_game(&InitGame {
        player_id: 0,
        show_name_tags: true,
        name_tag_distance: 70.0,
        server_name: server.into(),
        ..InitGame::default()
    });
    // The lines above carry the wall clock; the frames show timestamps, so they are printed again at `time`.
    core.chat = ChatWindow::new();
    for line in [
        super::core::msg::STARTED.to_string(),
        format!("Connecting to 127.0.0.1:{port}..."),
        "Connected. Joining the game...".to_string(),
        format!("Connected to {{B9C9BF}}{server}"),
    ] {
        core.chat.add_debug(line.as_bytes(), time);
    }
}

/// `CRGBA` (R, G, B, A in memory) of a wire colour `0xRRGGBBAA`, as RPC 134 carries textdraw colours.
const fn crgba(rgba: u32) -> u32 {
    rgba.swap_bytes()
}

/// A text textdraw's RPC 134 block (`flags`: [`tf`]).
#[allow(clippy::too_many_arguments)]
fn td(
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
fn sprite(x: f32, y: f32, w: f32, h: f32, rgba: u32) -> TextDrawData {
    td(x, y, 4, (0.0, 0.0), (w, h), rgba, 0, 0, 0)
}

fn rpc(core: &mut UiCore, i: Incoming) {
    core.on_rpc(&i, &NoExpansion);
}

/// The `scene` command of sampui.pwn (and what the client prints while connecting).
pub fn setup(core: &mut UiCore) {
    let t = LocalTime { unix: 0, hour: 12, minute: 0, second: 0 };
    connect(core, "RefPlayer", 7822, "SA-MP UI reference", t);
    core.colours.set(0, LOCAL);
    info(core, "Welcome to the {FFFF00}SA-MP UI{FFFFFF} reference server", 0xFFFF_FFFF, t);
    info(core, "Server: this line is light blue", 0xA9C4_E4FF, t);
    info(core, "{FF0000}Red {00FF00}green {0000FF}blue {FFFFFF}white {FF6347}and back", 0xFF63_47FF, t);
    info(
        core,
        "A long line: the quick brown fox jumps over the lazy dog, again and again, until it runs off",
        0x88AA_62FF,
        t,
    );
    info(core, "Yellow 0123456789 !\"#$%%&'()*+,-./:;<=>?@[\\]^_`|~", 0xFFFF_00FF, t);
    player_chat(core, "Big_Smoke", SMOKE, "hello from the remote player", t);
    let p = tf::PROPORTIONAL;
    let w = (1280.0, 1280.0);
    let tds = &mut core.textdraws;
    tds.show(0, td(40.0, 140.0, 0, (0.5, 1.5), w, 0xFFFF_FFFF, 1, 0, p), b"Font 0 ~r~red~w~ white");
    tds.show(1, td(40.0, 170.0, 1, (0.4, 1.4), w, 0xFFFF_FFFF, 0, 2, p), b"Font 1 shadow 2");
    tds.show(2, td(40.0, 200.0, 2, (0.4, 1.6), w, 0xFFFF_00FF, 1, 0, p), b"FONT 2 OUTLINE");
    tds.show(3, td(40.0, 230.0, 3, (0.6, 2.0), w, 0x2B8F_2BFF, 1, 0, p), b"Font 3 Pricedown");
    let mut boxed = td(320.0, 380.0, 1, (0.35, 1.2), (0.0, 200.0), 0xFFFF_FFFF, 0, 0, p | tf::BOX | tf::ALIGN_CENTRE);
    boxed.box_colour = crgba(0x0000_0088);
    tds.show(4, boxed, b"Boxed centre text");
    tds.show(5, td(600.0, 330.0, 1, (0.35, 1.2), w, 0xA9C4_E4FF, 1, 0, p | tf::ALIGN_RIGHT), b"Right aligned");
    tds.show(6, sprite(500.0, 140.0, 40.0, 40.0, 0xFF00_0099), b"LD_SPAC:white");
    tds.show(7, sprite(560.0, 140.0, 32.0, 32.0, 0xFFFF_FFFF), b"LD_BEAT:chit");
    let text = SendGameText { style: 3, time_ms: 600_000, text: "~r~reference ~w~game text".into() };
    rpc(core, Incoming::Ui(ui::Incoming::SendGameText(text)));
    let c = rgba_to_argb;
    core.deaths.add(b"Big_Smoke", c(SMOKE), b"RefPlayer", c(LOCAL), 24);
    core.deaths.add(b"RefPlayer", c(LOCAL), b"Ryder", c(RYDER), 31);
    core.deaths.add(b"", 0, b"Big_Smoke", c(SMOKE), 200);
    core.deaths.add(b"Ryder", c(RYDER), b"Big_Smoke", c(SMOKE), 0);
}

/// The dialog of `ShowStyle(style)` in sampui.pwn.
fn show_style(core: &mut UiCore, style: u8) {
    let (title, body, b1, b2) = match style {
        0 => (
            "Message box",
            "{FFFFFF}Welcome to the server!\nThis is a {FF0000}message box{A9C4E4} with two lines.",
            "OK",
            "Cancel",
        ),
        1 => ("Input", "Enter your name:", "Send", "Cancel"),
        2 => ("Weapons", "Desert Eagle\nM4\nMP5\nShotgun\n{FF0000}Sniper Rifle", "Select", "Close"),
        3 => ("Login", "Enter your password:", "Login", "Quit"),
        4 => ("Shop", "Deagle\t$5000\t100\nSawnoff\t$5000\t100\nPistol\t$1000\t50", "Buy", "Close"),
        _ => {
            ("Shop", "Weapon\tPrice\tAmmo\nDeagle\t$5000\t100\nSawnoff\t$5000\t100\nPistol\t$1000\t50", "Buy", "Close")
        }
    };
    let d = ShowDialog {
        dialog_id: 10 + i16::from(style),
        style,
        title: title.into(),
        button1: b1.into(),
        button2: b2.into(),
        body: body.into(),
    };
    rpc(core, Incoming::Ui(ui::Incoming::ShowDialog(d)));
}

/// The step at playing frame `n` (1-based) on a `w` x `h` screen: the input it starts with goes to `input`; returns
/// the name of the step that begins at `n`.
pub fn step(n: u32, w: f32, h: f32, core: &mut UiCore, input: &mut Vec<WindowEvent>) -> Option<String> {
    let key = |input: &mut Vec<WindowEvent>, vk: u16| {
        input.push(WindowEvent::KeyDown { vk, repeat: false });
        input.push(WindowEvent::KeyUp { vk });
    };
    // The last frame of a step closes the chat input (step 1) or answers the dialog (steps 2..7) with Esc, so the
    // next step starts clean.
    if n.is_multiple_of(HOLD) && (1..=7).contains(&(n / HOLD - 1)) {
        key(input, vk::ESCAPE);
    }
    if n % HOLD != 1 {
        return None;
    }
    let s = n / HOLD;
    let typed = |input: &mut Vec<WindowEvent>, t: &str| {
        input.extend(t.chars().map(|c| WindowEvent::Char { ch: u32::from(c), unicode: true }))
    };
    let name = match s {
        0 => "01-scene".to_string(),
        1 => {
            key(input, vk::T);
            typed(input, "thello world");
            "02-chat-input".into()
        }
        2..=7 => {
            let style = (s - 2) as u8;
            show_style(core, style);
            input.push(WindowEvent::MouseMove { x: w as i32 - 60, y: h as i32 - 60 });
            match style {
                1 => typed(input, "RefPlayer"),
                3 => typed(input, "secret"),
                2 | 4 | 5 => {
                    key(input, vk::DOWN);
                    key(input, vk::DOWN);
                }
                _ => {}
            }
            format!("03-dialog-style{style}")
        }
        8 => {
            // The reference lists the local player only (NPCs are not listed).
            let players = vec![PlayerScoreAndPing { player_id: 0, score: 0, ping: 10 }];
            let s = SendPlayerScoresAndPings { players };
            rpc(core, Incoming::Session(session::Incoming::SendPlayerScoresAndPings(s)));
            key(input, vk::TAB);
            input.push(WindowEvent::MouseMove { x: (w / 2.0) as i32 + 40, y: (h / 2.0) as i32 - 40 });
            "04-scoreboard".into()
        }
        9 => {
            key(input, vk::TAB);
            key(input, vk::F7);
            "05-chat-plain".into()
        }
        _ => return None,
    };
    Some(name)
}
