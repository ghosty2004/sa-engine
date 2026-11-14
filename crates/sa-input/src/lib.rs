//! INPUT subsystem of the 1:1 rebuild of `gta_sa.exe` 1.0 US (PLAN §2 crate `sa-input`;
//! PORT-ANIM-INPUT, `docs/arch/anim-input-port.md`): `CPad` (the pad state machine and every
//! control query), `CControllerState`, `CKeyboardState`, `CMouseControllerState` and the
//! `CControllerConfigManager` (`ControlsManager`, key bindings and their effect on the pad states).
//!
//! A separate crate on top of `sa-core` (ADR-001 §7), named as PLAN §2 fixes it. Reasons for not
//! putting it into sa-core or sa-world: INPUT is a consumer of CORE (the keyboard/mouse globals and
//! the Win32/DirectInput glue that PORT-CORE already ported stay in `sa_core::input` and are used from
//! here), sa-world and later crates (sa-ped, sa-vehicle, sa-cam) call into the pad only through
//! `orig!` declarations, and a separate crate keeps the WORLD/PHYS worktrees' files untouched.
//!
//! Same rules as sa-core: game state through `sa_core::globals!`/`GPtr`, calls to other catalogued
//! functions through `sa_core::orig!`, every port registered with `#[hook]` and listed in its
//! module's `HOOKS` ([`hooks::ALL`]). Memory access only through sa-core's audited accessors, so this
//! crate has no `unsafe` beyond `GameData` impls.
#![no_std]
#![deny(unsafe_code)]
// Large `orig!` blocks expand one recursive macro step per declaration.
#![recursion_limit = "1024"]

#[cfg(test)]
extern crate std;

pub use sa_core::hook;

pub mod ccm;
pub mod common;
pub mod hooks;
pub mod pad;
pub mod statics;

pub mod std6_natives;
pub mod generated;
pub mod pad_mouse_steer;
pub mod key_char;
