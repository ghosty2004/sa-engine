//! ANIM: the static initialisers / `atexit` destructors of the subsystem's globals
//! (PORT-ANIM-INPUT, fork D). Ported from the 1.0 US bytes, `docs/re/specs/ANIM/_static-init.md`.
//! See `docs/arch/anim-input-port.md`.
//!
//! The CRT `eh vector constructor/destructor iterator` and `_atexit` are called with the original
//! ctor/dtor addresses (as sa-stream's `CLoadingScreen` statics do); the initialisers return what
//! `_atexit` returned (their `pop ecx; ret` keeps EAX).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::orig;

use crate::bones::{bone_node_man, eh_vector_ctor, eh_vector_dtor};
use crate::common::{g, wr};
use crate::cutscene::cutscene_associations;
use crate::ik::ik_chain_man;
use crate::vehgroup::VEHICLE_ANIM_GROUPS;

sa_core::globals! {
    list GLOBALS;
}

/// `CAnimManager::ms_aAnimations` (2500 × `CAnimBlendHierarchy`, 0x18 each; owned by CAnimManager).
const ANIMATIONS: usize = 0xB4EA40;
/// `CAnimManager::ms_aAnimBlocks` (180 × `CAnimBlock`, 0x20 each; owned by CAnimManager).
const ANIM_BLOCKS: usize = 0xB5D4A0;

orig! {
    /// CRT `_atexit` (0x821D1E)
    pub fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
    /// `CAnimBlendAssocGroup::CAnimBlendAssocGroup()` (0x4CDE70)
    pub fn assoc_group_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4CDE70 => unported;
    /// `CAnimBlendAssocGroup::~CAnimBlendAssocGroup()` (0x4CE1D0)
    pub fn assoc_group_dtor(this: GPtr<u8>) = thiscall 0x4CE1D0 => unported;
    /// `BoneNodeManager_c::BoneNodeManager_c()` (0x617330)
    pub fn bone_node_manager_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x617330 => crate::bones::manager_constructor;
    /// `BoneNodeManager_c::~BoneNodeManager_c()` (0x617390)
    pub fn bone_node_manager_dtor(this: GPtr<u8>) = thiscall 0x617390 => crate::bones::manager_destructor;
    /// `IKChainManager_c::IKChainManager_c()` (0x617FC0)
    pub fn ik_chain_manager_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x617FC0 => crate::ik::manager_constructor;
    /// `IKChainManager_c::~IKChainManager_c()` (0x618030, catalogue FUN_00618030)
    pub fn ik_chain_manager_dtor(this: GPtr<u8>) = thiscall 0x618030 => unported;
}

/// `__init_CAnimManager__ms_aAnimations` (0x84BFD0): construct the 2500 hierarchies (ctor 0x4CF270,
/// dtor 0x4CF970), register 0x8562C0.
#[hook(addr = 0x84BFD0, conv = cdecl, name = "__init_CAnimManager__ms_aAnimations", expect = "68 70 F9 4C 00")]
pub fn init_anim_manager_animations() -> i32 {
    eh_vector_ctor(g(ANIMATIONS), 0x18, 0x9C4, 0x4CF270, 0x4CF970);
    atexit(0x8562C0)
}

/// `__init_CAnimManager__ms_aAnimBlocks` (0x84C000): the inlined `CAnimBlock` ctor of the 180 blocks:
/// `Name[0] = 0`, `RefCnt (+0x12, u16) = 0`.
#[hook(addr = 0x84C000, conv = cdecl, name = "__init_CAnimManager__ms_aAnimBlocks", expect = "B8 A0 D4 B5 00")]
pub fn init_anim_manager_anim_blocks() {
    let base = g::<u8>(ANIM_BLOCKS);
    for i in 0..0xB4 {
        let b = base.byte_add(i * 0x20);
        wr::<u8>(b, 0, 0);
        wr::<u16>(b, 0x12, 0);
    }
}

/// `__init_CCutsceneMgr__ms_cutsceneAssociations` (0x850040): the group's ctor, register 0x8566F0.
#[hook(addr = 0x850040, conv = cdecl, name = "__init_CCutsceneMgr__ms_cutsceneAssociations", expect = "B9 20 40 BC 00")]
pub fn init_cutscene_associations() -> i32 {
    assoc_group_ctor(cutscene_associations());
    atexit(0x8566F0)
}

/// `__init_g_boneNodeMan` (0x851A70): the manager's ctor, register 0x8568D0.
#[hook(addr = 0x851A70, conv = cdecl, name = "__init_g_boneNodeMan", expect = "B9 20 08 C1 00")]
pub fn init_g_bone_node_man() -> i32 {
    bone_node_manager_ctor(bone_node_man());
    atexit(0x8568D0)
}

/// `__init_g_ikChainMan` (0x851B50): the manager's ctor, register 0x8568E0.
#[hook(addr = 0x851B50, conv = cdecl, name = "__init_g_ikChainMan", expect = "B9 48 54 C1 00")]
pub fn init_g_ik_chain_man() -> i32 {
    ik_chain_manager_ctor(ik_chain_man());
    atexit(0x8568E0)
}

/// `__init_CVehicleAnimGroupData__m_vehicleAnimGroups` (0x853E20): construct the 30 groups (ctor
/// 0x5BD420, dtor 0x5B9E30 = `ret`), register 0x856A60.
#[hook(addr = 0x853E20, conv = cdecl, name = "__init_CVehicleAnimGroupData__m_vehicleAnimGroups", expect = "68 30 9E 5B 00")]
pub fn init_vehicle_anim_groups() -> i32 {
    eh_vector_ctor(VEHICLE_ANIM_GROUPS.ptr().cast(), 0x94, 0x1E, 0x5BD420, 0x5B9E30);
    atexit(0x856A60)
}

/// `__dtor_CAnimManager__ms_aAnimations` (0x8562C0): destroy the 2500 hierarchies.
#[hook(addr = 0x8562C0, conv = cdecl, name = "__dtor_CAnimManager__ms_aAnimations", expect = "68 70 F9 4C 00")]
pub fn dtor_anim_manager_animations() {
    eh_vector_dtor(g(ANIMATIONS), 0x18, 0x9C4, 0x4CF970);
}

/// `__dtor_CCutsceneMgr__ms_cutsceneAssociations` (0x8566F0): tail-jump to the group's dtor.
#[hook(addr = 0x8566F0, conv = cdecl, name = "__dtor_CCutsceneMgr__ms_cutsceneAssociations", expect = "B9 20 40 BC 00")]
pub fn dtor_cutscene_associations() {
    assoc_group_dtor(cutscene_associations());
}

/// `__dtor_g_boneNodeMan` (0x8568D0): tail-jump to the manager's dtor.
#[hook(addr = 0x8568D0, conv = cdecl, name = "__dtor_g_boneNodeMan", expect = "B9 20 08 C1 00")]
pub fn dtor_g_bone_node_man() {
    bone_node_manager_dtor(bone_node_man());
}

/// `__dtor_g_ikChainMan` (0x8568E0): tail-jump to `IKChainManager_c::~IKChainManager_c` (0x618030).
#[hook(addr = 0x8568E0, conv = cdecl, name = "__dtor_g_ikChainMan", expect = "B9 48 54 C1 00")]
pub fn dtor_g_ik_chain_man() {
    ik_chain_manager_dtor(ik_chain_man());
}

/// `__dtor_CVehicleAnimGroupData__m_vehicleAnimGroups` (0x856A60): the iterator with the `ret`
/// element dtor (no effect).
#[hook(addr = 0x856A60, conv = cdecl, name = "__dtor_CVehicleAnimGroupData__m_vehicleAnimGroups", expect = "68 30 9E 5B 00")]
pub fn dtor_vehicle_anim_groups() {
    eh_vector_dtor(VEHICLE_ANIM_GROUPS.ptr().cast(), 0x94, 0x1E, 0x5B9E30);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_INIT_ANIM_MANAGER_ANIMATIONS,
    HOOK_INIT_ANIM_MANAGER_ANIM_BLOCKS,
    HOOK_INIT_CUTSCENE_ASSOCIATIONS,
    HOOK_INIT_G_BONE_NODE_MAN,
    HOOK_INIT_G_IK_CHAIN_MAN,
    HOOK_INIT_VEHICLE_ANIM_GROUPS,
    HOOK_DTOR_ANIM_MANAGER_ANIMATIONS,
    HOOK_DTOR_CUTSCENE_ASSOCIATIONS,
    HOOK_DTOR_G_BONE_NODE_MAN,
    HOOK_DTOR_G_IK_CHAIN_MAN,
    HOOK_DTOR_VEHICLE_ANIM_GROUPS,
];
