//! The engine-drawn SA:MP user interface: chat window and input line, dialogs, nametags, 3D text labels,
//! textdraws, game text, the kill list, the scoreboard and the mouse cursor.
//!
//! On the original backend samp-rs draws all of this itself through its D3D9 proxy (`ui/render/*.rs`,
//! [`crate::render::Overlay`]). A backend that has no D3D9 device (the native engine, the browser build)
//! can instead offer an engine-owned UI through [`crate::GameAdapter::ui`]: samp-rs feeds it state (the
//! RPC payloads, already decoded) and drains [`UiEvent`]s (what the player typed or clicked). Layout and
//! behaviour follow samp 0.3.7-R4 (`docs/samp/overlay.md`).
//!
//! Strings are UTF-8 (samp-rs converts the wire's code page 1252). Embedded `{RRGGBB}` colour codes are
//! interpreted where samp interprets them (chat, dialogs, labels, nametags' text is plain). Colours are
//! [`Rgba`] (samp's wire colours are `0xRRGGBBAA`: see [`Rgba::from_rgba_u32`]).
//!
//! Added at API v1 (additive: a default method on `GameAdapter`, new types only).

use crate::input::{EventResult, WindowEvent};
use crate::{PedRef, Rgba, Vec3};

impl Rgba {
    /// From samp's wire format `0xRRGGBBAA`.
    pub const fn from_rgba_u32(v: u32) -> Self {
        Self {
            r: (v >> 24) as u8,
            g: (v >> 16) as u8,
            b: (v >> 8) as u8,
            a: v as u8,
        }
    }

    /// From D3D's `0xAARRGGBB`.
    pub const fn from_argb_u32(v: u32) -> Self {
        Self {
            a: (v >> 24) as u8,
            r: (v >> 16) as u8,
            g: (v >> 8) as u8,
            b: v as u8,
        }
    }

    /// To D3D's `0xAARRGGBB`.
    pub const fn to_argb_u32(self) -> u32 {
        (self.a as u32) << 24 | (self.r as u32) << 16 | (self.g as u32) << 8 | self.b as u32
    }
}

/// The kind of a chat entry (samp's chat window entry types).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChatKind {
    /// Player chat (RPC 101): `name:` in the name colour, then the text in white.
    Player,
    /// Server / client message (RPC 93) and local info lines: the text in its colour.
    Info,
    /// Debug line.
    Debug,
}

/// One chat line as samp-rs hands it over.
#[derive(Clone, Debug, PartialEq)]
pub struct ChatMessage {
    /// Entry type.
    pub kind: ChatKind,
    /// Sender name (player chat only; empty otherwise).
    pub name: String,
    /// Name colour.
    pub name_colour: Rgba,
    /// Text (may contain `{RRGGBB}` codes).
    pub text: String,
    /// Text colour.
    pub colour: Rgba,
    /// Local time the line arrived (hour, minute, second), for the chat timestamps (F9 / `/timestamp`).
    pub time: [u8; 3],
}

/// Display options samp-rs sets from `sa-mp.cfg`, its local commands and the server's `InitGame` flags.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiOptions {
    /// Chat lines shown (`/pagesize`, 10..=20).
    pub chat_page_size: u8,
    /// Timestamps before chat lines (`/timestamp`).
    pub timestamps: bool,
    /// `/fontsize` (−3..=5): every UI font grows by 2 px per step.
    pub font_size: i8,
    /// Nametags shown at all (`ShowNameTags`).
    pub nametags: bool,
    /// Nametag draw distance in metres (`SetNameTagDrawDistance`, default 70).
    pub nametag_distance: f32,
    /// Nametags hidden behind buildings and objects (`DisableNameTagLOS` turns it off).
    pub nametag_los: bool,
}

impl Default for UiOptions {
    fn default() -> Self {
        Self {
            chat_page_size: 10,
            timestamps: false,
            font_size: 0,
            nametags: true,
            nametag_distance: 70.0,
            nametag_los: true,
        }
    }
}

/// Dialog styles (`ShowPlayerDialog`'s style byte).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogStyle {
    /// 0: message box.
    MsgBox,
    /// 1: input line.
    Input,
    /// 2: list.
    List,
    /// 3: password input.
    Password,
    /// 4: tab list.
    Tablist,
    /// 5: tab list with a header row.
    TablistHeaders,
}

impl DialogStyle {
    /// From the wire byte (unknown values are message boxes, as samp treats them).
    pub const fn from_byte(b: u8) -> Self {
        match b {
            1 => Self::Input,
            2 => Self::List,
            3 => Self::Password,
            4 => Self::Tablist,
            5 => Self::TablistHeaders,
            _ => Self::MsgBox,
        }
    }
}

/// A dialog to show (RPC 61).
#[derive(Clone, Debug, PartialEq)]
pub struct DialogSpec {
    /// Dialog id (−1 hides).
    pub id: i16,
    /// Style.
    pub style: DialogStyle,
    /// Caption.
    pub title: String,
    /// Body (rows separated by `\n`, cells by `\t` for tab lists).
    pub body: String,
    /// Left button.
    pub button1: String,
    /// Right button (empty = none).
    pub button2: String,
}

/// What a nametag hangs on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NametagAnchor {
    /// A ped: the backend reads its position every frame.
    Ped(PedRef),
    /// A fixed world position (the ped's root; the tag is raised as samp raises it).
    World(Vec3),
}

/// One nametag with its health / armour bars.
#[derive(Clone, Debug, PartialEq)]
pub struct Nametag {
    /// Player id shown after the name (`Name (id)`).
    pub player_id: u16,
    /// Anchor.
    pub anchor: NametagAnchor,
    /// Name.
    pub name: String,
    /// Name colour (the player colour, alpha ignored).
    pub colour: Rgba,
    /// Health (0..=100; negative hides the bars).
    pub health: f32,
    /// Armour (0..=100).
    pub armour: f32,
    /// Paused (AFK icon).
    pub afk: bool,
}

/// A chat bubble above a player (`SetPlayerChatBubble`, RPC 59).
#[derive(Clone, Debug, PartialEq)]
pub struct ChatBubble {
    /// The player's ped (or a fixed position).
    pub anchor: NametagAnchor,
    /// Text (≤ 144 characters, `{RRGGBB}` codes; wrapped at 36 characters like samp).
    pub text: String,
    /// Colour (alpha forced opaque, as samp does).
    pub colour: Rgba,
    /// Draw distance in metres.
    pub distance: f32,
    /// Lifetime in milliseconds of game time.
    pub expire_ms: u32,
}

/// A 3D text label (RPC 36).
#[derive(Clone, Debug, PartialEq)]
pub struct TextLabel3d {
    /// Text (`{RRGGBB}` codes and `\n`).
    pub text: String,
    /// Colour.
    pub colour: Rgba,
    /// World position (or offset when attached).
    pub pos: Vec3,
    /// Draw distance in metres.
    pub draw_distance: f32,
    /// Hidden behind walls (line of sight test; the native backend does not test it yet).
    pub test_los: bool,
    /// Attached to a ped (position is then an offset from it).
    pub attached_ped: Option<PedRef>,
}

/// Textdraw alignment (samp's 1 left, 2 centre, 3 right).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TdAlign {
    /// Left.
    Left,
    /// Centred.
    Centre,
    /// Right.
    Right,
}

/// A textdraw (RPC 134), in samp's 640×448 canvas.
#[derive(Clone, Debug, PartialEq)]
pub struct TextDrawSpec {
    /// Text (GTA `~r~`-style codes are the game font's business).
    pub text: String,
    /// Position.
    pub x: f32,
    /// Position.
    pub y: f32,
    /// Letter size.
    pub letter_w: f32,
    /// Letter size.
    pub letter_h: f32,
    /// Box size (`TextSize`): wrap x / centre size.
    pub text_w: f32,
    /// Box size.
    pub text_h: f32,
    /// Letter colour.
    pub colour: Rgba,
    /// Box on.
    pub use_box: bool,
    /// Box colour.
    pub box_colour: Rgba,
    /// Shadow size.
    pub shadow: u8,
    /// Outline size.
    pub outline: u8,
    /// Shadow / outline colour.
    pub background_colour: Rgba,
    /// Game font 0..=3 (4 = sprite, 5 = model preview: not drawn by the engine UI yet).
    pub font: u8,
    /// Proportional spacing.
    pub proportional: bool,
    /// Alignment.
    pub align: TdAlign,
    /// Clickable in cursor mode.
    pub selectable: bool,
}

/// A row of the scoreboard (TAB).
#[derive(Clone, Debug, PartialEq)]
pub struct ScoreRow {
    /// Player id.
    pub id: u16,
    /// Name.
    pub name: String,
    /// Score.
    pub score: i32,
    /// Ping.
    pub ping: u32,
    /// Name colour.
    pub colour: Rgba,
}

/// The cursor mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CursorMode {
    /// No cursor; the game owns the mouse.
    #[default]
    Off,
    /// Cursor shown, the camera does not move (dialogs, scoreboard, `SelectTextDraw`).
    Locked,
    /// Cursor shown, game input passes (samp's mode 3).
    Free,
}

/// What the player did, drained by samp-rs with [`SampUi::poll_events`].
#[derive(Clone, Debug, PartialEq)]
pub enum UiEvent {
    /// The chat input was submitted: text or a `/command` (samp-rs decides between RPC 101 and 50 and
    /// runs its local commands).
    ChatSubmit(String),
    /// A dialog was answered (`OnDialogResponse`): button 1 = left, 0 = right / Esc.
    DialogResponse {
        /// Dialog id.
        id: i16,
        /// 1 or 0.
        button: u8,
        /// Selected list row or −1.
        list_item: i16,
        /// Input text, or the selected row's first cell.
        text: String,
    },
    /// A selectable textdraw was clicked (`0xFFFF` = selection cancelled with Esc).
    TextDrawClicked(u16),
    /// A scoreboard row was clicked.
    ScoreboardClicked(u16),
}

/// The engine-drawn SA:MP UI. Methods are called from frame callbacks on the game thread.
pub trait SampUi {
    /// Adds a chat line (oldest of 100 dropped).
    fn chat_add(&mut self, msg: &ChatMessage);
    /// Clears the chat.
    fn chat_clear(&mut self);
    /// Shows a dialog; `None` hides the current one.
    fn show_dialog(&mut self, dialog: Option<&DialogSpec>);
    /// Replaces the set of nametags drawn this frame and after.
    fn set_nametags(&mut self, tags: &[Nametag]);
    /// Shows / replaces (`Some`) or removes (`None`) player `player_id`'s chat bubble.
    fn set_chat_bubble(&mut self, player_id: u16, bubble: Option<&ChatBubble>);
    /// Creates / replaces (`Some`) or deletes (`None`) 3D text label `id`.
    fn set_label(&mut self, id: u16, label: Option<&TextLabel3d>);
    /// Shows / replaces (`Some`) or hides (`None`) textdraw `id` (0..2048 global, 2048.. player).
    fn set_textdraw(&mut self, id: u16, td: Option<&TextDrawSpec>);
    /// `GameTextForPlayer`: style 0..=6, time in milliseconds.
    fn game_text(&mut self, text: &str, time_ms: u32, style: u8);
    /// Adds a kill-list row (`reason` = weapon id, 200 connect, 201 disconnect; empty killer = none).
    fn kill_entry(
        &mut self,
        killer: &str,
        killer_colour: Rgba,
        victim: &str,
        victim_colour: Rgba,
        reason: u8,
    );
    /// Opens (`Some`) or closes the scoreboard with these rows and the server name.
    fn set_scoreboard(&mut self, rows: Option<(&str, &[ScoreRow])>);
    /// Sets the cursor mode.
    fn set_cursor_mode(&mut self, mode: CursorMode);
    /// `SelectTextDraw` (`Some(hover colour)`) / `CancelSelectTextDraw` (`None`).
    fn select_textdraws(&mut self, hover: Option<Rgba>);
    /// Display options (page size, timestamps, font size, nametag rules).
    fn set_options(&mut self, options: &UiOptions);
    /// Whether the chat input or a dialog has the keyboard (samp's input lock).
    fn wants_keyboard(&self) -> bool;
    /// A window event, before the game sees it. Native backends call this themselves before
    /// [`crate::hooks::GameHooks::window_event`].
    fn handle_event(&mut self, event: WindowEvent) -> EventResult;
    /// Takes the events since the last call.
    fn poll_events(&mut self) -> Vec<UiEvent>;
}
