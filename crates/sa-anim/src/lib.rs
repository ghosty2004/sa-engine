//! ANIM subsystem of the 1:1 rebuild of `gta_sa.exe` 1.0 US (PLAN §2 crate `sa-anim`;
//! PORT-ANIM-INPUT, `docs/arch/anim-input-port.md`): `CAnimManager` (blocks, groups, IFP loading,
//! the uncompressed-animation cache), the blend classes (`CAnimBlendNode`, `CAnimBlendSequence`,
//! `CAnimBlendHierarchy`, `CAnimBlendAssociation`, `CAnimBlendStaticAssociation`,
//! `CAnimBlendAssocGroup`, `CAnimBlendClumpData`), bones and IK (`BoneNode_c`, `BoneNodeManager_c`,
//! `IKChain_c`, `IKChainManager_c`, `CPedIK`), vehicle anim groups, `CCutsceneMgr`/`CCutsceneObject`
//! and `CAnimatedBuilding`.
//!
//! A separate crate on top of `sa-core` (ADR-001 §7), named as PLAN §2 fixes it: ANIM is consumed by
//! PED/VEH/CAM/SCRIPT and consumes only CORE/STREAM/RW (all reached through `orig!`), so no engine
//! crate needs to depend on it; it also keeps the WORLD/PHYS worktrees' files untouched.
//!
//! Same rules as sa-core: game state through `sa_core::globals!`/`GPtr`, calls to other catalogued
//! functions (RenderWare `Rp*`/`Rt*` included) through `sa_core::orig!`, every port registered with
//! `#[hook]` and listed in its module's `HOOKS` ([`hooks::ALL`]). Memory access only through sa-core's
//! audited accessors, so this crate has no `unsafe` beyond `GameData` impls.
#![no_std]
#![deny(unsafe_code)]
// Large `orig!` blocks expand one recursive macro step per declaration.
#![recursion_limit = "1024"]

#[cfg(test)]
extern crate std;

pub use sa_core::hook;

pub mod assoc;
pub mod bones;
pub mod building;
pub mod clumpdata;
pub mod common;
pub mod cutscene;
pub mod group;
pub mod hierarchy;
pub mod hooks;
pub mod ik;
pub mod manager;
pub mod mvpb_anim;
pub mod node;
pub mod pedik;
pub mod sequence;
pub mod statics;
pub mod vehgroup;
