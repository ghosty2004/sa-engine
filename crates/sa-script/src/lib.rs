//! SCRIPT subsystem of the 1:1 rebuild of `gta_sa.exe` 1.0 US (PORT-SCRIPT-1, `docs/arch/script-port.md`):
//! the `CRunningScript` VM core ([`vm`]) and the `ProcessCommands0To99` switch ([`cmd0`]).
//!
//! Same rules as sa-core and sa-world (ADR-001): game state through `sa_core::globals!`/`GPtr`, calls
//! to other catalogued functions through `sa_core::orig!` with the original address, every port
//! registered with `#[hook]` and listed in its module's `HOOKS` ([`hooks::ALL`]).
#![no_std]
#![deny(unsafe_code)]
#![recursion_limit = "1024"]

#[cfg(test)]
extern crate std;

pub use sa_core::hook;

pub mod cleanup;
pub mod script_draw;
pub mod script_init;
pub mod script_brains;
pub mod script_brain_requests;
pub mod script_objects;
pub mod script_resources;
pub mod script_tables;
pub mod script_registries;
pub mod script_process;
pub mod script_tasks;
pub mod script_things;
pub mod the_scripts_init;
pub mod script_search_lights;
pub mod death_arrest_check;
pub mod entry_exit_flag;
pub mod script_anim_group;
pub mod script_locate;
pub mod script_thing_index;
pub mod streamed_script_load;
pub mod script_entity_cleanup;
pub mod cmd0;
pub mod cmd100;
pub mod cmd1000;
pub mod cmd1100;
pub mod cmd1200;
pub mod cmd1300;
pub mod cmd1400;
pub mod cmd1500;
pub mod cmd1600;
pub mod cmd1700;
pub mod cmd1800;
pub mod cmd1900;
pub mod cmd200;
pub mod cmd2000;
pub mod cmd2100;
pub mod cmd2200;
pub mod cmd2300;
pub mod cmd2400;
pub mod cmd2500;
pub mod cmd2600;
pub mod cmd300;
pub mod cmd400;
pub mod cmd500;
pub mod cmd600;
pub mod cmd700;
pub mod cmd800;
pub mod cmd900;
pub mod cmdlib;
pub mod hooks;
pub mod misc;
// SAVE-1 (docs/arch/save-port.md)
pub mod save;
pub mod vm;
pub mod play_anim_command;
pub mod generated;
pub mod mission_cleanup_init;
