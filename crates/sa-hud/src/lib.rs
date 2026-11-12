//! HUD subsystem of the 1:1 rebuild of `gta_sa.exe` 1.0 US (PLAN §2 crate `sa-hud`, PORT-HUD,
//! `docs/arch/hud-port.md`): HUD, radar and blips, fonts, messages and game text, GXT text lookup,
//! on-screen timers and counters, script menus, and the small HUD-catalogued managers.
//!
//! Same rules as sa-core, sa-stream, sa-world and sa-pop (ADR-001): game state through
//! `sa_core::globals!`/`GPtr`, calls to other catalogued functions through `sa_core::orig!` with the
//! original address (rendering calls into RenderWare / D3D stay `orig!` calls in phase A), every port
//! registered with `#[hook]` and listed in its module's `HOOKS` ([`hooks::ALL`]). Ported from the 1.0 US
//! bytes; gta-reversed and `docs/re/specs/HUD/*.md` are reading aids. Where they differ, the bytes win.
#![no_std]
#![deny(unsafe_code)]
#![recursion_limit = "1024"]

#[cfg(test)]
extern crate std;

pub use sa_core::hook;

pub mod common;
pub mod hooks;

pub mod font;
pub mod hud;
pub mod menu;
pub mod messages;
pub mod radar;
pub mod statics;
pub mod text;
pub mod timer;

pub mod font_print;
pub mod hud_draw;
pub mod menu2;
pub mod menu_mgr;
pub mod radar_draw;
pub mod place;
pub mod radar_draw2;
pub mod statics2;
pub mod text_load;

pub mod credits;
pub mod credits_table;
pub mod hud_draw3;
pub mod menu3;
pub mod menu_ctrl;
pub mod menu_draw;
pub mod menu_input;
pub mod menu_map;
pub mod menu_controls;
pub mod menu_map_input;
pub mod menu_stats_file;
pub mod menu_system_init;
pub mod radar_draw3;
pub mod generated;
