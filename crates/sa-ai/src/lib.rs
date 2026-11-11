//! AI subsystem of the 1:1 rebuild of `gta_sa.exe` 1.0 US (PLAN §2 crate `sa-ai`; PORT-AI-1,
//! `docs/arch/ai-port.md`): `CTask*` (simple and complex), `CEvent*`, the event handler/group/scanner,
//! `CPedIntelligence`, `CTaskManager`, `CPedGroup*`, `CDecisionMaker*`, `CAcquaintance`, attractors.
//!
//! Same rules as the other engine crates (ADR-001): game state through `sa_core::globals!`/`GPtr`,
//! calls to other catalogued functions through `sa_core::orig!` with the original address, every port
//! registered with `#[hook]` and listed in its module's `HOOKS` ([`hooks::ALL`]). Task and event objects
//! keep the original vtables (a ported virtual is a hook on the function, not a new vtable) and are
//! allocated from the original pools through the original `operator new`. Ports follow the original
//! bytes, including its bugs (PLAN §4), never gta-reversed's fixes.
#![no_std]
#![deny(unsafe_code)]
// Large `orig!` blocks expand one recursive macro step per declaration.
#![recursion_limit = "2048"]

#[cfg(test)]
extern crate std;

pub use sa_core::hook;

pub mod car_drive;
pub mod car_enter;
pub mod car_exit;
pub mod collscan;
pub mod common;
pub mod conversation;
pub mod decisions;
pub mod dtors;
pub mod events2;
pub mod fall;
pub mod events;
pub mod fight;
pub mod fighthelp;
pub mod gangtasks;
pub mod flee;
pub mod gang6;
mod gen_check;
pub mod gen_clones;
pub mod gen_ctors;
pub mod gen_dtors;
pub mod gen_leaves;
pub mod gen_subtasks;
pub mod gen_virtuals;
pub mod geh;
pub mod group;
pub mod groups;
pub mod handler;
pub mod hooks;
pub mod intel;
pub mod intelrest;
pub mod killtasks;
pub mod misc;
pub mod partner6;
pub mod pgi;
pub mod seek;
pub mod small;
pub mod stuck;
pub mod smallai5;
pub mod taskmgr;
pub mod tasks;
pub mod trivial;
pub mod usegun;
pub mod walk;
pub mod walkround;
pub mod wander;
