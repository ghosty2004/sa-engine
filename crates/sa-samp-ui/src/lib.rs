//! `sa-samp-ui`: the SA:MP 0.3.7 user interface drawn by the engine (docs/samp/overlay.md).
//!
//! samp-rs draws its UI through a D3D9 device proxy and D3DX fonts on the original `gta_sa.exe`. The native
//! engine (Linux, and later the browser) has no D3D9 device, so this crate holds the UI itself: chat window
//! and input line, dialogs (all six styles), nametags with health / armour bars, 3D text labels, textdraws,
//! the kill list, the scoreboard, the class-selection panel and the cursor. It keeps their state, handles keyboard and mouse input, and
//! lays everything out as a [`draw::DrawList`] (rectangles, UI-font text, game-font text). The engine side
//! (`sa-samp::overlay`) draws that list through the ported `CSprite2d`, RW Im2D and `CFont` code.
//!
//! samp-rs talks to it through `sa_samp_api::ui::SampUi` ([`ui::Ui`] implements it): it feeds RPC state in
//! and drains [`sa_samp_api::ui::UiEvent`]s (chat lines, dialog responses, clicks).
//!
//! Look and behaviour follow samp-rs's `samp-ui` (its specification of samp.dll): the same layout formulas,
//! colours, `{RRGGBB}` handling and line wrapping. The UI font is Liberation Sans Bold (metric-compatible
//! with Arial, SIL OFL 1.1, `assets/fonts/`); textdraws use the game's fonts as in samp.

pub mod chat;
pub mod classpanel;
pub mod colour;
pub mod dialog;
pub mod draw;
pub mod edit;
pub mod font;
pub mod panels;
pub mod skin;
pub mod text;
pub mod textdraw;
pub mod ui;
pub mod world;

pub use draw::{Cmd, DrawList};
pub use font::{FontAtlas, FontId, Measure};
pub use ui::Ui;
pub use world::WorldView;
