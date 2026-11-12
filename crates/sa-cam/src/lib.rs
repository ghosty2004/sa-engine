//! CAM subsystem of the 1:1 rebuild of `gta_sa.exe` 1.0 US (PLAN §2 crate `sa-cam`, PORT-CAM,
//! `docs/arch/cam-port.md`): `CCamera` (`TheCamera` 0xB6F028), the `CCam` modes, `CHandShaker`,
//! `CIdleCam`, `CCurves` and the camera static initialisers.
//!
//! Same rules as sa-core, sa-stream, sa-world and sa-pop (ADR-001): game state through
//! `sa_core::globals!`/`GPtr`, calls to other catalogued functions through `sa_core::orig!` with the
//! original address, every port registered with `#[hook]` and listed in its module's `HOOKS`
//! ([`hooks::ALL`]). Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/CAM/*.md` are reading
//! aids. Where they differ, the bytes win. x87 sequences follow the original operand order (register vs.
//! memory operands differ on NaN inputs), with sa-world's [`Fpu`](sa_world::collision::fpu::Fpu) machine
//! where the stack use matters.
#![no_std]
#![deny(unsafe_code)]
#![recursion_limit = "1024"]

#[cfg(test)]
extern crate std;

pub use sa_core::hook;

pub mod common;
pub mod hooks;

pub mod cam;
pub mod camctl;
pub mod camera;
pub mod camfx;
pub mod curves;
pub mod handshaker;
pub mod idlecam;
pub mod statics;

pub mod aimweapon;
pub mod arrestcam;
pub mod avoidgeom;
pub mod camcol;
pub mod camcontrol;
pub mod cammodes;
pub mod camproc;
pub mod camtarget;
pub mod camtrans;
pub mod dwcams;
pub mod firstperson;
pub mod followcar;
pub mod followped;
pub mod obbe;
pub mod padlook;
pub mod pedmouse;
pub mod pedonpc;
pub mod rocket;
pub mod splines;
pub mod twoplayer;

pub mod camera_process;
pub mod generated;
pub mod idle_cam_slerp;
pub mod std6_natives;
