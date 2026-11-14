//! `--samp-ui-demo` with `SA_SAMP_UI_SCENE=ref`: the content of `tools/e2e/samp/sampui.pwn` (the reference
//! gamemode that `tools/e2e/samp-ui-ref.sh` shows on the original client), so samp-rs's UI on the engine can be
//! compared with screenshots of samp.dll 0.3.7-R4: the client's own connect lines, the server's chat lines, the
//! remote player's chat, a 3D label, the textdraws, the game text and the kill list; then each dialog style (the
//! list styles with the third row selected, the input styles with text typed), the chat input and the
//! scoreboard. Keep both in step.
//!
//! The script advances one step every [`HOLD`] playing frames and prints the state it shows, so the offscreen
//! frames (`--present-dir`) can be matched to the reference shots.

use sa_platform::input::{InputEvent, Key};
use samp_client::ui::core::UiCore;
use samp_proto::rpcs::ui::textdraw_flags as tf;
use samp_ui::colour::rgba_to_argb;

use crate::samp_ui_demo::{connect, crgba, dialog, game_text, info, player_chat, scores, sprite, td};

/// Playing frames per step.
pub const HOLD: u32 = 4;

/// Colours of the reference players.
const LOCAL: u32 = 0xFF80_00FF;
const SMOKE: u32 = 0x2B8F_2BFF;
const RYDER: u32 = 0x9966_CCFF;

/// The reference label's text.
pub const LABEL: &str = "{FFFF00}Grove Street\n{FFFFFF}Home of the {00FF00}Grove Street Families";

/// The `scene` command of sampui.pwn (and what the client prints while connecting).
pub fn setup(core: &mut UiCore) {
    let t = samp_ui::chat::LocalTime { unix: 0, hour: 12, minute: 0, second: 0 };
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
    game_text(core, "~r~reference ~w~game text", 600_000, 3);
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
        _ => ("Shop", "Weapon\tPrice\tAmmo\nDeagle\t$5000\t100\nSawnoff\t$5000\t100\nPistol\t$1000\t50", "Buy", "Close"),
    };
    dialog(core, 10 + i16::from(style), style, title, body, b1, b2);
}

/// The step at playing frame `n` (1-based), what it shows, and the input / RPCs it starts with.
pub fn step(n: u32, w: f32, h: f32, core: &mut UiCore, input: &mut Vec<InputEvent>) {
    let key = |input: &mut Vec<InputEvent>, k: Key| {
        input.push(InputEvent::KeyDown { key: k, repeat: false });
        input.push(InputEvent::KeyUp { key: k });
    };
    // The last frame of a step closes the chat input (step 1) or answers the dialog (steps 2..7) with Esc,
    // so the next step starts clean.
    if n.is_multiple_of(HOLD) && (1..=7).contains(&(n / HOLD - 1)) {
        key(input, Key::Escape);
    }
    if n % HOLD != 1 {
        return;
    }
    let s = n / HOLD;
    let typed = |input: &mut Vec<InputEvent>, t: &str| input.extend(t.chars().map(InputEvent::Char));
    let name = match s {
        0 => "01-scene".to_string(),
        1 => {
            key(input, Key::T);
            typed(input, "thello world");
            "02-chat-input".into()
        }
        2..=7 => {
            let style = (s - 2) as u8;
            show_style(core, style);
            input.push(InputEvent::MouseMove { x: w as i32 - 60, y: h as i32 - 60 });
            match style {
                1 => typed(input, "RefPlayer"),
                3 => typed(input, "secret"),
                2 | 4 | 5 => {
                    key(input, Key::Down);
                    key(input, Key::Down);
                }
                _ => {}
            }
            format!("03-dialog-style{style}")
        }
        8 => {
            // The reference lists the local player only (NPCs are not listed).
            scores(core, &[(0, 0, 10)]);
            key(input, Key::Tab);
            input.push(InputEvent::MouseMove { x: (w / 2.0) as i32 + 40, y: (h / 2.0) as i32 - 40 });
            "04-scoreboard".into()
        }
        9 => {
            key(input, Key::Tab);
            key(input, Key::Fn(7));
            "05-chat-plain".into()
        }
        _ => return,
    };
    println!("samp-ui-scene: frame {n}: {name}");
}
