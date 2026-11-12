//! `--samp-ui-demo` with `SA_SAMP_UI_SCENE=ref`: the content of `tools/e2e/samp/sampui.pwn` (the reference
//! gamemode that `tools/e2e/samp-ui-ref.sh` shows on the original client), so the engine-drawn UI can be
//! compared with screenshots of samp.dll 0.3.7-R4: the client's own connect lines, the server's chat lines,
//! the remote player's chat, a 3D label, the textdraws, the game text and the kill list; then each dialog
//! style (the list styles with the third row selected, the input styles with text typed), the chat input and
//! the scoreboard. Keep both in step.
//!
//! The script advances one step every [`HOLD`] playing frames and prints the state it shows, so the offscreen
//! frames (`--present-dir`) can be matched to the reference shots.

use sa_platform::input::{InputEvent, Key};
use sa_samp_api::ui::{
    ChatKind, DialogSpec, DialogStyle, SampUi, ScoreRow, TdAlign, TextDrawSpec, TextLabel3d,
};
use sa_samp_api::{Rgba, Vec3};

/// Playing frames per step.
pub const HOLD: u32 = 4;

/// Colours of the reference players.
const LOCAL: u32 = 0xFF80_00FF;
const SMOKE: u32 = 0x2B8F_2BFF;
const RYDER: u32 = 0x9966_CCFF;

fn chat(ui: &mut dyn SampUi, kind: ChatKind, name: &str, name_rgba: u32, text: &str, rgba: u32) {
    ui.chat_add(&sa_samp_api::ui::ChatMessage {
        kind,
        name: name.into(),
        name_colour: Rgba::from_rgba_u32(name_rgba),
        text: text.into(),
        colour: Rgba::from_rgba_u32(rgba),
        time: [12, 0, 0],
    });
}

#[allow(clippy::too_many_arguments)]
fn td(text: &str, x: f32, y: f32, font: u8, lw: f32, lh: f32, rgba: u32, outline: u8, shadow: u8) -> TextDrawSpec {
    TextDrawSpec {
        text: text.into(),
        x,
        y,
        letter_w: lw,
        letter_h: lh,
        text_w: 1280.0,
        text_h: 1280.0,
        colour: Rgba::from_rgba_u32(rgba),
        use_box: false,
        box_colour: Rgba::from_rgba_u32(0x8080_8080),
        shadow,
        outline,
        background_colour: Rgba::from_rgba_u32(0x0000_00FF),
        font,
        proportional: true,
        align: TdAlign::Left,
        selectable: false,
    }
}

fn sprite(text: &str, x: f32, y: f32, w: f32, h: f32, rgba: u32) -> TextDrawSpec {
    TextDrawSpec {
        text_w: w,
        text_h: h,
        proportional: false,
        ..td(text, x, y, 4, 0.48, 1.12, rgba, 0, 0)
    }
}

/// The `scene` command of sampui.pwn (and what the client prints while connecting).
pub fn setup(ui: &mut dyn SampUi, label_pos: Vec3) {
    let info = |ui: &mut dyn SampUi, t: &str, c: u32| chat(ui, ChatKind::Info, "", 0, t, c);
    info(ui, "{FFFFFF}SA-MP {B9C9BF}0.3.7-R4 {FFFFFF}Started", 0xFFFF_FFFF);
    info(ui, "Connecting to 127.0.0.1:7822...", 0xA9C4_E4FF);
    info(ui, "Connected. Joining the game...", 0xA9C4_E4FF);
    info(ui, "Connected to {B9C9BF}SA-MP UI reference", 0xA9C4_E4FF);
    info(ui, "Welcome to the {FFFF00}SA-MP UI{FFFFFF} reference server", 0xFFFF_FFFF);
    info(ui, "Server: this line is light blue", 0xA9C4_E4FF);
    info(ui, "{FF0000}Red {00FF00}green {0000FF}blue {FFFFFF}white {FF6347}and back", 0xFF63_47FF);
    info(
        ui,
        "A long line: the quick brown fox jumps over the lazy dog, again and again, until it runs off",
        0x88AA_62FF,
    );
    info(ui, "Yellow 0123456789 !\"#$%%&'()*+,-./:;<=>?@[\\]^_`|~", 0xFFFF_00FF);
    chat(ui, ChatKind::Player, "Big_Smoke", SMOKE, "hello from the remote player", 0xFFFF_FFFF);
    ui.set_label(
        1,
        Some(&TextLabel3d {
            text: "{FFFF00}Grove Street\n{FFFFFF}Home of the {00FF00}Grove Street Families".into(),
            colour: Rgba::from_rgba_u32(0xFFFF_FFFF),
            pos: label_pos,
            draw_distance: 40.0,
            test_los: false,
            attached_ped: None,
        }),
    );
    ui.set_textdraw(0, Some(&td("Font 0 ~r~red~w~ white", 40.0, 140.0, 0, 0.5, 1.5, 0xFFFF_FFFF, 1, 0)));
    ui.set_textdraw(1, Some(&td("Font 1 shadow 2", 40.0, 170.0, 1, 0.4, 1.4, 0xFFFF_FFFF, 0, 2)));
    ui.set_textdraw(2, Some(&td("FONT 2 OUTLINE", 40.0, 200.0, 2, 0.4, 1.6, 0xFFFF_00FF, 1, 0)));
    ui.set_textdraw(3, Some(&td("Font 3 Pricedown", 40.0, 230.0, 3, 0.6, 2.0, 0x2B8F_2BFF, 1, 0)));
    ui.set_textdraw(
        4,
        Some(&TextDrawSpec {
            align: TdAlign::Centre,
            use_box: true,
            box_colour: Rgba::from_rgba_u32(0x0000_0088),
            text_w: 0.0,
            text_h: 200.0,
            ..td("Boxed centre text", 320.0, 380.0, 1, 0.35, 1.2, 0xFFFF_FFFF, 0, 0)
        }),
    );
    ui.set_textdraw(
        5,
        Some(&TextDrawSpec {
            align: TdAlign::Right,
            ..td("Right aligned", 600.0, 330.0, 1, 0.35, 1.2, 0xA9C4_E4FF, 1, 0)
        }),
    );
    ui.set_textdraw(6, Some(&sprite("LD_SPAC:white", 500.0, 140.0, 40.0, 40.0, 0xFF00_0099)));
    ui.set_textdraw(7, Some(&sprite("LD_BEAT:chit", 560.0, 140.0, 32.0, 32.0, 0xFFFF_FFFF)));
    ui.game_text("~r~reference ~w~game text", 600_000, 3);
    let c = Rgba::from_rgba_u32;
    ui.kill_entry("Big_Smoke", c(SMOKE), "RefPlayer", c(LOCAL), 24);
    ui.kill_entry("RefPlayer", c(LOCAL), "Ryder", c(RYDER), 31);
    ui.kill_entry("", Rgba::default(), "Big_Smoke", c(SMOKE), 200);
    ui.kill_entry("Ryder", c(RYDER), "Big_Smoke", c(SMOKE), 0);
}

/// The dialog of `ShowStyle(style)` in sampui.pwn.
pub fn dialog(style: u8) -> DialogSpec {
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
        _ => (
            "Shop",
            "Weapon\tPrice\tAmmo\nDeagle\t$5000\t100\nSawnoff\t$5000\t100\nPistol\t$1000\t50",
            "Buy",
            "Close",
        ),
    };
    DialogSpec {
        id: 10 + i16::from(style),
        style: DialogStyle::from_byte(style),
        title: title.into(),
        body: body.into(),
        button1: b1.into(),
        button2: b2.into(),
    }
}

/// The scoreboard rows of the reference (the local player only: NPCs are not listed).
pub fn scoreboard_rows() -> Vec<ScoreRow> {
    vec![ScoreRow {
        id: 0,
        name: "RefPlayer".into(),
        score: 0,
        ping: 10,
        colour: Rgba::from_rgba_u32(LOCAL),
    }]
}

/// The step at playing frame `n` (1-based), what it shows, and the input / UI calls it starts with.
pub fn step(n: u32, w: f32, h: f32, ui: &mut dyn SampUi, input: &mut Vec<InputEvent>) {
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
            ui.show_dialog(Some(&dialog(style)));
            input.push(InputEvent::MouseMove {
                x: w as i32 - 60,
                y: h as i32 - 60,
            });
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
            ui.set_scoreboard(Some(("SA-MP UI reference", &scoreboard_rows())));
            input.push(InputEvent::MouseMove {
                x: (w / 2.0) as i32 + 40,
                y: (h / 2.0) as i32 - 40,
            });
            "04-scoreboard".into()
        }
        9 => {
            ui.set_scoreboard(None);
            key(input, Key::Fn(7));
            "05-chat-plain".into()
        }
        _ => return,
    };
    println!("samp-ui-scene: frame {n}: {name}");
}
