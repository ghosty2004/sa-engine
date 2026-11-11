//! The rest of the AI subsystem's non-task code (PORT-AI-1, helper B): `CPedIntelligence`, `CPedGroup*`,
//! `CPedGroups`, `CDecisionMaker*`, `CAcquaintance`, `CPlayerRelationshipRecorder`, attractors, `CPtrList`,
//! `CPedGeometryAnalyser`, unnamed `FUN_`/`vt_` rows and the `__init_*` static initialisers.
//! Ported from the 1.0 US bytes (`docs/re/specs/AI/*.md`); the exe wins over gta-reversed.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

sa_core::globals! {
    list GLOBALS;
    /// `GetPlayerRelationshipRecorder`'s lazily allocated instance (0x80 bytes: 16 × {ped, bool}).
    pub static PLAYER_RELATIONSHIP_RECORDER @ 0xC17084: GPtr<u8> = GPtr::NULL;
    /// `CPedAttractorManager` singleton pointer (`GetPedAttractorManager`)
    pub static PED_ATTRACTOR_MANAGER @ 0xC09840: GPtr<u8> = GPtr::NULL;
    /// `CDecisionMakerTypes` singleton pointer (`CDecisionMakerTypes::GetInstance`)
    pub static DECISION_MAKER_TYPES @ 0xC0B030: GPtr<u8> = GPtr::NULL;
}

/// `CTimer::m_snTimeInMilliseconds` (owned by CTimer).
const TIME_MS: usize = 0xB7CB84;

orig! {
    /// CRT `_atexit` (0x821D1E)
    fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall)
    fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// CRT `operator new` (0x82119A)
    fn crt_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// `CColModel::CColModel` (0x40FB60)
    fn col_model_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x40FB60 => unported;
    /// `CTaskSimpleAnim::~CTaskSimpleAnim` (0x61A6F0)
    fn task_simple_anim_dtor(this: GPtr<u8>) = thiscall 0x61A6F0 => unported;
    /// `FUN_005bb9f0` (0x5BB9F0, stdcall): loads an event index table from a file.
    fn load_event_indices(indices: GPtr<u8>, file: u32) -> u32 = stdcall 0x5BB9F0 => unported;
    /// `CPedGroupPlacer::PlaceFormationGroup` (0x5FC9B0)
    fn place_formation_group(this: GPtr<u8>, a0: u32, a1: u32, a2: u32, a3: u32) -> u8 = thiscall 0x5FC9B0 => unported;
    /// `CPedGroupPlacer::PlaceChatGroup` (0x5FCE80)
    fn place_chat_group(this: GPtr<u8>, a0: u32, a1: u32, a2: u32, a3: u32) -> u8 = thiscall 0x5FCE80 => unported;
    /// `CPedGroupPlacer::PlaceRandomGroup` (0x5FD330)
    fn place_random_group(this: GPtr<u8>, a0: u32, a1: u32, a2: u32, a3: u32) -> u8 = thiscall 0x5FD330 => unported;
    /// `CCarEnterExit::ComputeTargetDoorToExit(CVehicle*, CPed*)` (0x64F110)
    fn compute_target_door_to_exit(veh: GPtr<u8>, ped: GPtr<u8>) -> u32 = cdecl 0x64F110 => unported;
    /// `CDecision::MakeDecision` (0x6040D0)
    #[allow(clippy::too_many_arguments)]
    fn decision_make_decision(this: GPtr<u8>, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32) = thiscall 0x6040D0 => unported;
    /// `CEntitySeekPosCalculator*::ComputeEntitySeekPos` (vtable +0)
    fn seek_pos_compute(calc: GPtr<u8>, ped: GPtr<u8>, target: GPtr<u8>, out: GPtr<u8>) = thiscall vslot 0x0 => unported;
    /// `CVehiclePotentialCollisionScanner::ScanForVehiclePotentialCollisionEvents` (0x603720)
    fn scan_vehicle_potential_collision(this: GPtr<u8>, ped: GPtr<u8>, vehs: GPtr<u8>, n: u32) = thiscall 0x603720 => unported;
    /// `CPointRoute::operator delete` (0x41B7D0)
    fn point_route_delete(p: GPtr<u8>) = cdecl 0x41B7D0 => unported;
    /// `FUN_0061a3d0` (0x61A3D0): `CTaskComplex` base destructor
    fn task_complex_dtor(this: GPtr<u8>) = thiscall 0x61A3D0 => unported;
}

// ---- static initialisers -----------------------------------------------------------------------

/// `__init_g_969a28` (0x849A00): `[0x969A28] = 1.0 / 50.0`.
#[hook(addr = 0x849A00, conv = cdecl, name = "__init_g_969a28", expect = "D9 05 24 86 85 00")]
pub fn init_g_969a28() {
    g::<f32>(0x969A28).write(x(1.0).fdiv_m(50.0).to_f32());
}

/// `__init_g_a9b098` (0x84BA90): `[0xA9B098] = 0.005 * 0.0625`.
#[hook(addr = 0x84BA90, conv = cdecl, name = "__init_g_a9b098", expect = "D9 05 8C B8 85 00")]
pub fn init_g_a9b098() {
    g::<f32>(0xA9B098).write(x(f32::from_bits(0x3BA3_D70A)).fmul_m(0.0625).to_f32());
}

/// `__init_g_a9b0a0` (0x84BAD0): `[0xA9B0A0] = 100.0 / 3.6` (3.6 = bits 0x40666666).
#[hook(addr = 0x84BAD0, conv = cdecl, name = "__init_g_a9b0a0", expect = "D9 05 28 86 85 00")]
pub fn init_g_a9b0a0() {
    g::<f32>(0xA9B0A0).write(x(100.0).fdiv_m(f32::from_bits(0x4066_6666)).to_f32());
}

/// `__init_g_c0afc8` (0x851380): `[0xC0AFC8] = 1.0 / 50.0`.
#[hook(addr = 0x851380, conv = cdecl, name = "__init_g_c0afc8", expect = "D9 05 24 86 85 00")]
pub fn init_g_c0afc8() {
    g::<f32>(0xC0AFC8).write(x(1.0).fdiv_m(50.0).to_f32());
}

/// `__init_g_c0bbf0` (0x8516B0): `[0xC0BBF0] = 0.005 * 0.0625`.
#[hook(addr = 0x8516B0, conv = cdecl, name = "__init_g_c0bbf0", expect = "D9 05 48 D0 86 00")]
pub fn init_g_c0bbf0() {
    g::<f32>(0xC0BBF0).write(x(f32::from_bits(0x3BA3_D70A)).fmul_m(0.0625).to_f32());
}

/// `__init_g_c0bbf4` (0x8516D0): `[0xC0BBF4] = 1.0 / 50.0`.
#[hook(addr = 0x8516D0, conv = cdecl, name = "__init_g_c0bbf4", expect = "D9 05 24 86 85 00")]
pub fn init_g_c0bbf4() {
    g::<f32>(0xC0BBF4).write(x(1.0).fdiv_m(50.0).to_f32());
}

/// `__init_g_c177bc` (0x851F50): `[0xC177BC] = 1.0 / 50.0`.
#[hook(addr = 0x851F50, conv = cdecl, name = "__init_g_c177bc", expect = "D9 05 24 86 85 00")]
pub fn init_g_c177bc() {
    g::<f32>(0xC177BC).write(x(1.0).fdiv_m(50.0).to_f32());
}

/// `__init_g_c18bf0` (0x852140): `[0xC18BF0] = 0.005 * 0.0625`.
#[hook(addr = 0x852140, conv = cdecl, name = "__init_g_c18bf0", expect = "D9 05 6C E6 86 00")]
pub fn init_g_c18bf0() {
    g::<f32>(0xC18BF0).write(x(f32::from_bits(0x3BA3_D70A)).fmul_m(0.0625).to_f32());
}

/// `__init_g_c18bfc` (0x8521A0): `[0xC18BFC] = 1.0 / (50.0 * 50.0)` (`fld; fmul; fdivr`).
#[hook(addr = 0x8521A0, conv = cdecl, name = "__init_g_c18bfc", expect = "D9 05 74 E6 86 00")]
pub fn init_g_c18bfc() {
    g::<f32>(0xC18BFC).write(x(50.0).fmul_m(50.0).fdivr_m(1.0).to_f32());
}

/// `__init_g_c09870` (0x850F60): registers `__dtor_g_c09870` (0x8567C0).
#[hook(addr = 0x850F60, conv = cdecl, name = "__init_g_c09870", expect = "68 C0 67 85 00")]
pub fn init_g_c09870() -> i32 {
    atexit(0x8567C0)
}

/// `__init_CCover__m_ListOfProcessedBuildings` (0x853170): registers 0x8569C0.
#[hook(addr = 0x853170, conv = cdecl, name = "__init_CCover__m_ListOfProcessedBuildings", expect = "68 C0 69 85 00")]
pub fn init_cover_list_of_processed_buildings() -> i32 {
    atexit(0x8569C0)
}

/// `__init_col1` (0x851FF0): `CColModel` ctor at 0xC17824, registers 0x8568F0.
#[hook(addr = 0x851FF0, conv = cdecl, name = "__init_col1", expect = "B9 24 78 C1 00")]
pub fn init_col1() -> i32 {
    col_model_ctor(g(0xC17824));
    atexit(0x8568F0)
}

/// `__init_ms_ClimbColModel` (0x852A10): `CColModel` ctor at 0xC19518, registers 0x856960.
#[hook(addr = 0x852A10, conv = cdecl, name = "__init_ms_ClimbColModel", expect = "B9 18 95 C1 00")]
pub fn init_ms_climb_col_model() -> i32 {
    col_model_ctor(g(0xC19518));
    atexit(0x856960)
}

/// `__init_ms_StandUpColModel` (0x852A30): `CColModel` ctor at 0xC19548, registers 0x856970.
#[hook(addr = 0x852A30, conv = cdecl, name = "__init_ms_StandUpColModel", expect = "B9 48 95 C1 00")]
pub fn init_ms_stand_up_col_model() -> i32 {
    col_model_ctor(g(0xC19548));
    atexit(0x856970)
}

/// `__init_ms_VaultColModel` (0x852A50): `CColModel` ctor at 0xC19578, registers 0x856980.
#[hook(addr = 0x852A50, conv = cdecl, name = "__init_ms_VaultColModel", expect = "B9 78 95 C1 00")]
pub fn init_ms_vault_col_model() -> i32 {
    col_model_ctor(g(0xC19578));
    atexit(0x856980)
}

/// `__init_ms_FindEdgeColModel` (0x852A70): `CColModel` ctor at 0xC195A8, registers 0x856990.
#[hook(addr = 0x852A70, conv = cdecl, name = "__init_ms_FindEdgeColModel", expect = "B9 A8 95 C1 00")]
pub fn init_ms_find_edge_col_model() -> i32 {
    col_model_ctor(g(0xC195A8));
    atexit(0x856990)
}

/// `__init_CInformFriendsEventQueue__ms_informFriendsEvents` (0x84BA30): 8 × 0xC at 0xA9AFB8
/// (ctor 0x4AC0D0, dtor 0x4B2950), registers 0x8561F0.
#[hook(addr = 0x84BA30, conv = cdecl, name = "__init_CInformFriendsEventQueue__ms_informFriendsEvents", expect = "68 50 29 4B 00")]
pub fn init_inform_friends_events() -> i32 {
    eh_vector_ctor(g(0xA9AFB8), 0xC, 8, 0x4AC0D0, 0x4B2950);
    atexit(0x8561F0)
}

/// `__init_CInformGroupEventQueue__ms_informGroupEvents` (0x84BA60): 8 × 0x10 at 0xA9B018
/// (ctor 0x4AC350, dtor 0x4B29A0), registers 0x856210.
#[hook(addr = 0x84BA60, conv = cdecl, name = "__init_CInformGroupEventQueue__ms_informGroupEvents", expect = "68 A0 29 4B 00")]
pub fn init_inform_group_events() -> i32 {
    eh_vector_ctor(g(0xA9B018), 0x10, 8, 0x4AC350, 0x4B29A0);
    atexit(0x856210)
}

/// `__init_CPedGroups__ms_groups` (0x851330): 8 × 0x2D4 at 0xC09920 (ctor 0x5FC150, dtor 0x5FC190),
/// registers 0x856860.
#[hook(addr = 0x851330, conv = cdecl, name = "__init_CPedGroups__ms_groups", expect = "68 90 C1 5F 00")]
pub fn init_ped_groups() -> i32 {
    eh_vector_ctor(g(0xC09920), 0x2D4, 8, 0x5FC150, 0x5FC190);
    atexit(0x856860)
}

fn zero_dwords(base: usize, n: usize) {
    for i in 0..n {
        g::<u32>(base + 4 * i).write(0);
    }
}

/// `__init_g_c1a43c` (0x853240): zero 6 dwords at 0xC1A43C, registers 0x8569D0.
#[hook(addr = 0x853240, conv = cdecl, name = "__init_g_c1a43c", expect = "57 33 C0 B9 06 00 00 00")]
pub fn init_g_c1a43c() -> i32 {
    zero_dwords(0xC1A43C, 6);
    atexit(0x8569D0)
}

/// `__init_g_c1a45c` (0x853260): zero 30 dwords at 0xC1A45C, registers 0x8569E0.
#[hook(addr = 0x853260, conv = cdecl, name = "__init_g_c1a45c", expect = "57 33 C0 B9 1E 00 00 00")]
pub fn init_g_c1a45c() -> i32 {
    zero_dwords(0xC1A45C, 0x1E);
    atexit(0x8569E0)
}

/// `__init_g_c1a4dc` (0x853280): zero 30 dwords at 0xC1A4DC, registers 0x8569F0.
#[hook(addr = 0x853280, conv = cdecl, name = "__init_g_c1a4dc", expect = "57 33 C0 B9 1E 00 00 00")]
pub fn init_g_c1a4dc() -> i32 {
    zero_dwords(0xC1A4DC, 0x1E);
    atexit(0x8569F0)
}

// ---- leaves ------------------------------------------------------------------------------------

/// `thunk_FUN_0061a6f0` (0x48DE60): `jmp CTaskSimpleAnim::~CTaskSimpleAnim`.
/// Not hookable: a bare `jmp rel32` entry is taken for a `.HOODLUM` stub by sa-hook (its target is not in `.HOODLUM`).
pub fn thunk_61a6f0_48de60(this: GPtr<u8>) {
    task_simple_anim_dtor(this);
}

/// `thunk_FUN_0061a6f0` (0x48EA90): `jmp CTaskSimpleAnim::~CTaskSimpleAnim`.
/// Not hookable: a bare `jmp rel32` entry is taken for a `.HOODLUM` stub by sa-hook (its target is not in `.HOODLUM`).
pub fn thunk_61a6f0_48ea90(this: GPtr<u8>) {
    task_simple_anim_dtor(this);
}

/// `CAcquaintance::~CAcquaintance` (0x608780): `ret`.
/// Not hookable: the body up to its `ret` is shorter than the 5-byte patch.
pub fn acquaintance_dtor_608780(_this: GPtr<u8>) {}

/// `(missed by ghidra: cat7-call)` (0x66B150): `this[+0x34] = a0` (`ret 4`).
#[hook(addr = 0x66B150, conv = thiscall, name = "(missed by ghidra: cat7-call)", expect = "8B 44 24 04 89 41 34")]
pub fn set_field_34_66b150(this: GPtr<u8>, a0: u32) {
    wr::<u32>(this, 0x34, a0);
}

/// `CDecisionMakerTypes::LoadEventIndices` (0x600840): `FUN_005bb9f0(this + 0xC034, 0x86CD44)`.
#[hook(addr = 0x600840, conv = thiscall, name = "CDecisionMakerTypes::LoadEventIndices", expect = "68 44 CD 86 00")]
pub fn decision_maker_types_load_event_indices_600840(this: GPtr<u8>) -> u32 {
    load_event_indices(this.byte_add(0xC034), 0x86CD44)
}

/// `CPedAttractor::IsAtHeadOfQueue(CPed*)` (0x5EB530): `*m_PedQueue.begin (+0x20) == ped`.
#[hook(addr = 0x5EB530, conv = thiscall, name = "CPedAttractor::IsAtHeadOfQueue", expect = "8B 41 20 8B 10")]
pub fn ped_attractor_is_at_head_of_queue_5eb530(this: GPtr<u8>, ped: u32) -> u8 {
    let begin = rd::<GPtr<u8>>(this, 0x20);
    u8::from(rd::<u32>(begin, 0) == ped)
}

/// `FUN_0065ee90` (0x65EE90): bit 2 of the byte at +0x3D = bit 0 of `a0`.
#[hook(addr = 0x65EE90, conv = thiscall, name = "FUN_0065ee90", expect = "8A 44 24 04 8A 51 3D")]
pub fn fun_0065ee90(this: GPtr<u8>, a0: u8) {
    let d = rd::<u8>(this, 0x3D);
    wr::<u8>(this, 0x3D, d ^ (((a0 << 2) ^ d) & 4));
}

/// `CAcquaintance::CAcquaintance` (0x608AE0): five zero dwords; returns `this`.
#[hook(addr = 0x608AE0, conv = thiscall, name = "CAcquaintance::CAcquaintance", expect = "8B C1 33 C9 8B D0")]
pub fn acquaintance_ctor_608ae0(this: GPtr<u8>) -> GPtr<u8> {
    for i in 0..5 {
        wr::<u32>(this, 4 * i, 0);
    }
    this
}

/// `FUN_0065e710` (0x65E710, cdecl): `0x12E` when the i16 at `p+0x22` is 0x67 or 0x69, else `0x12C`.
#[hook(addr = 0x65E710, conv = cdecl, name = "FUN_0065e710", expect = "8B 44 24 04 0F BF 40 22")]
pub fn fun_0065e710(p: GPtr<u8>) -> u32 {
    match rd::<i16>(p, 0x22) {
        0x67 | 0x69 => 0x12E,
        _ => 0x12C,
    }
}

/// `CPlayerRelationshipRecorder::ClearRelationshipWithPlayer(CPed*)` (0x61A150): clears the first of
/// the 16 `{ped, bool}` entries holding `ped`.
#[hook(addr = 0x61A150, conv = thiscall, name = "CPlayerRelationshipRecorder::ClearRelationshipWithPlayer", expect = "8B 54 24 04 33 C0")]
pub fn player_relationship_recorder_clear_61a150(this: GPtr<u8>, ped: u32) {
    for i in 0..0x10 {
        if rd::<u32>(this, i * 8) == ped {
            wr::<u32>(this, i * 8, 0);
            wr::<u8>(this, i * 8 + 4, 0);
            return;
        }
    }
}

/// `GetPlayerRelationshipRecorder` (0x61A2E0): the instance at 0xC17084, allocated (CRT `operator new`,
/// 0x80 bytes, 16 cleared entries) on first use.
#[hook(addr = 0x61A2E0, conv = cdecl, name = "GetPlayerRelationshipRecorder", expect = "A1 84 70 C1 00")]
pub fn get_player_relationship_recorder_61a2e0() -> GPtr<u8> {
    let cur = PLAYER_RELATIONSHIP_RECORDER.get();
    if !cur.is_null() {
        return cur;
    }
    let p = crt_new(0x80);
    if !p.is_null() {
        for i in 0..0x10 {
            wr::<u32>(p, i * 8, 0);
            wr::<u8>(p, i * 8 + 4, 0);
        }
    }
    PLAYER_RELATIONSHIP_RECORDER.set(p);
    p
}

/// `FUN_004bc470` (0x4BC470): when `a0 != this[+0x18]` (fcomp; unordered counts as different):
/// `+0x18 = a0`, timer `+0x28 = now`, `+0x2C = 0`, `+0x30 = 1`.
#[hook(addr = 0x4BC470, conv = thiscall, name = "FUN_004bc470", expect = "D9 44 24 04 D8 59 18")]
pub fn fun_004bc470(this: GPtr<u8>, a0: f32) {
    let c = x(a0).fcom(x(rd::<f32>(this, 0x18)));
    if !c.test_parity(0x44) {
        return;
    }
    wr::<u32>(this, 0x18, a0.to_bits());
    wr::<u32>(this, 0x28, g::<u32>(TIME_MS).read());
    wr::<u32>(this, 0x2C, 0);
    wr::<u8>(this, 0x30, 1);
}

/// `CPedGroupPlacer::PlaceGroup` (0x5FD810): group type `a3` 0/1 → `PlaceFormationGroup`, 2/3 →
/// `PlaceChatGroup`, 5 → `PlaceRandomGroup` (tail jumps); otherwise returns the low byte of `a2`.
#[hook(addr = 0x5FD810, conv = thiscall, name = "CPedGroupPlacer::PlaceGroup", expect = "8B 44 24 10 83 F8 05")]
pub fn ped_group_placer_place_group_5fd810(this: GPtr<u8>, a0: u32, a1: u32, a2: u32, a3: u32) -> u8 {
    match a3 {
        0 | 1 => place_formation_group(this, a0, a1, a2, a3),
        2 | 3 => place_chat_group(this, a0, a1, a2, a3),
        5 => place_random_group(this, a0, a1, a2, 5),
        _ => a2 as u8,
    }
}

/// `FUN_0065de60` (0x65DE60): `vtbl = 0x86F8F8`, copies the `CVector` at `a0` to +4; returns `this`.
#[hook(addr = 0x65DE60, conv = thiscall, name = "FUN_0065de60", expect = "8B 54 24 04 8B C1")]
pub fn fun_0065de60(this: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0x86F8F8);
    for i in 0..3 {
        wr::<u32>(this, 4 + 4 * i, rd::<u32>(v, 4 * i));
    }
    this
}

/// `CPtrList::IsMemberOfList(void*)` (0x5521D0): walks the `{item, next}` nodes from the head.
#[hook(addr = 0x5521D0, conv = thiscall, name = "CPtrList::IsMemberOfList", expect = "8B 01 85 C0 74 15")]
pub fn ptr_list_is_member_of_list_5521d0(this: GPtr<u8>, item: u32) -> u8 {
    let mut n = rd::<GPtr<u8>>(this, 0);
    while !n.is_null() {
        if rd::<u32>(n, 0) == item {
            return 1;
        }
        n = rd::<GPtr<u8>>(n, 4);
    }
    0
}

/// `FUN_005e9800` (0x5E9800): element count of a vector of 0x14-byte items (`begin +4`, `end +8`);
/// 0 when `begin` is null.
#[hook(addr = 0x5E9800, conv = thiscall, name = "FUN_005e9800", expect = "8B 41 04 85 C0")]
pub fn fun_005e9800(this: GPtr<u8>) -> i32 {
    let b = rd::<u32>(this, 4);
    if b == 0 {
        return 0;
    }
    (rd::<u32>(this, 8).wrapping_sub(b) as i32) / 0x14
}

/// `FUN_0063bab0` (0x63BAB0): when `+0x10` is 0, `+0x10 = ComputeTargetDoorToExit(+0xC, a0)`; returns it.
#[hook(addr = 0x63BAB0, conv = thiscall, name = "FUN_0063bab0", expect = "56 8B F1 8B 46 10")]
pub fn fun_0063bab0(this: GPtr<u8>, ped: GPtr<u8>) -> u32 {
    let v = rd::<u32>(this, 0x10);
    if v != 0 {
        return v;
    }
    let r = compute_target_door_to_exit(rd::<GPtr<u8>>(this, 0xC), ped);
    wr::<u32>(this, 0x10, r);
    r
}

/// `CPedGroups::GetGroupId(CPedGroup*)` (0x5F7EE0): index of `group` in `ms_groups` (8 × 0x2D4 at
/// 0xC09920), or -1.
#[hook(addr = 0x5F7EE0, conv = cdecl, name = "CPedGroups::GetGroupId", expect = "56 8B 74 24 08")]
pub fn ped_groups_get_group_id_5f7ee0(group: u32) -> i32 {
    for i in 0..8u32 {
        if 0xC09920 + i * 0x2D4 == group {
            return i as i32;
        }
    }
    -1
}

/// `CPedGroups::IsInPlayersGroup(CPed*)` (0x5F7F10): not a player (`+0x480` == 0) and one of the 8
/// member slots of group 0 (0xC0992C..=0xC09948) is `ped`.
#[hook(addr = 0x5F7F10, conv = cdecl, name = "CPedGroups::IsInPlayersGroup", expect = "8B 4C 24 04 8B 81 80 04 00 00")]
pub fn ped_groups_is_in_players_group_5f7f10(ped: GPtr<u8>) -> u8 {
    if rd::<u32>(ped, 0x480) != 0 {
        return 0;
    }
    let p = ped.addr() as u32;
    for i in 0..8 {
        if g::<u32>(0xC0992C + 4 * i).read() == p {
            return 1;
        }
    }
    0
}

/// `FUN_005bd3b0` (0x5BD3B0, cdecl): `1 → 0x3D, 3..=8 → 0x3F..=0x44`, anything else `0x3E`.
#[hook(addr = 0x5BD3B0, conv = cdecl, name = "FUN_005bd3b0", preserve = "ecx", expect = "8B 44 24 04 48")]
pub fn fun_005bd3b0(a: i32) -> u32 {
    match a {
        1 => 0x3D,
        3..=8 => 0x3C + a as u32,
        _ => 0x3E,
    }
}

/// `CPedGroupMembership::GetObjectForPedToHold` (0x5F6950, static): `ftol((rand() & 0xFFFF) *
/// 2^-15 * 100)`: `< 33` → the model id word at 0x8CD72C, `< 66` → the word at 0x8CD728, else -1.
/// rand site 0x5F6956.
#[hook(addr = 0x5F6950, conv = cdecl, name = "CPedGroupMembership::GetObjectForPedToHold", expect = "51 E8 C8 B1 22 00")]
pub fn ped_group_membership_get_object_for_ped_to_hold_5f6950() -> i32 {
    let r = at_site(0x5F6956, rand) & 0xFFFF;
    let v = ftol32(xi(r).fmul_m(f32::from_bits(0x3800_0000)).fmul_m(100.0));
    if v < 0x21 {
        i32::from(g::<u16>(0x8CD72C).read())
    } else if v < 0x42 {
        i32::from(g::<u16>(0x8CD728).read())
    } else {
        -1
    }
}

/// `FUN_00669fb0` (0x669FB0): `CTaskTimer::IsOutOfTime`: not started (+0x30) → false; a pending
/// restart (+0x31) sets `start (+0x28) = now`; then `now >= start + interval (+0x2C)` (unsigned).
#[hook(addr = 0x669FB0, conv = thiscall, name = "FUN_00669fb0", expect = "8A 41 30 84 C0")]
pub fn fun_00669fb0(this: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0x30) == 0 {
        return 0;
    }
    if rd::<u8>(this, 0x31) != 0 {
        wr::<u32>(this, 0x28, g::<u32>(TIME_MS).read());
        wr::<u8>(this, 0x31, 0);
    }
    let end = rd::<u32>(this, 0x28).wrapping_add(rd::<u32>(this, 0x2C));
    u8::from(g::<u32>(TIME_MS).read() >= end)
}

/// `FUN_005ec220` (0x5EC220): `std::vector<void*>::erase(it)` (end at +8): shifts the tail down one
/// slot, `end -= 4`, `*out = it`; returns `out`.
#[hook(addr = 0x5EC220, conv = thiscall, name = "FUN_005ec220", expect = "8B 51 08 53 56")]
pub fn fun_005ec220(this: GPtr<u8>, out: GPtr<u8>, it: GPtr<u8>) -> GPtr<u8> {
    let end = rd::<u32>(this, 8);
    let mut p = it.byte_add(4);
    while p.addr() as u32 != end {
        wr::<u32>(p, -4, rd::<u32>(p, 0));
        p = p.byte_add(4);
    }
    wr::<u32>(this, 8, end.wrapping_sub(4));
    wr::<u32>(out, 0, it.addr() as u32);
    out
}

/// `FUN_00606060` (0x606060): `CDecision::MakeDecision` of entry `a0` (0x3C each) with the other 8
/// arguments (`ret 0x24`).
/// Not registered (PORT-AI-1): its split-run L2 pairs differ in the task out-parameter (grove 2/72, c-bug
/// 146/146 pairs, all with the +0x64 stack shift of a Rust caller); unresolved, so the original keeps running.
#[allow(clippy::too_many_arguments)]
pub fn fun_00606060(
    this: GPtr<u8>,
    a0: i32,
    a1: u32,
    a2: u32,
    a3: u32,
    a4: u32,
    a5: u32,
    a6: u32,
    a7: u32,
    a8: u32,
) {
    decision_make_decision(
        this.byte_add(a0.wrapping_mul(0x3C) as isize),
        a1,
        a2,
        a3,
        a4,
        a5,
        a6,
        a7,
        a8,
    );
}

/// The body of 0x46FEE0 / 0x665DC0: the seek-pos calculator at +0x40 computes into a local `CVector`
/// (`ped`, target entity +0xC), copied to `out`; returns `out`.
fn seek_pos(this: GPtr<u8>, out: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let mut tmp = [0u32; 3];
    let buf: GPtr<u8> = GPtr::from_raw(&mut tmp as *mut [u32; 3]).cast();
    seek_pos_compute(this.byte_add(0x40), ped, rd::<GPtr<u8>>(this, 0xC), buf);
    for (i, v) in tmp.iter().enumerate() {
        wr::<u32>(out, 4 * i as isize, *v);
    }
    out
}

/// `FUN_0046fee0` (0x46FEE0): see [`seek_pos`].
#[hook(addr = 0x46FEE0, conv = thiscall, name = "FUN_0046fee0", expect = "83 EC 0C 56 8B C1")]
pub fn fun_0046fee0(this: GPtr<u8>, out: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    seek_pos(this, out, ped)
}

/// `FUN_00665dc0` (0x665DC0): see [`seek_pos`].
#[hook(addr = 0x665DC0, conv = thiscall, name = "FUN_00665dc0", expect = "83 EC 0C 56 8B C1")]
pub fn fun_00665dc0(this: GPtr<u8>, out: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    seek_pos(this, out, ped)
}

/// `CEventScanner::ScanForEventsNow(const CPed*, bool)` (0x6053D0): with the full dword `dont_scan`
/// == 0: restart the timer at +4 when already started (+0xC), then
/// `ScanForVehiclePotentialCollisionEvents(this + 4, ped, intel + 0xE0, 16)`.
#[hook(addr = 0x6053D0, conv = thiscall, name = "CEventScanner::ScanForEventsNow", expect = "8B 44 24 04 8B 90 7C 04 00 00")]
pub fn event_scanner_scan_for_events_now_6053d0(this: GPtr<u8>, ped: GPtr<u8>, dont_scan: u32) {
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    if dont_scan != 0 {
        return;
    }
    if rd::<u8>(this, 0xC) != 0 {
        wr::<u32>(this, 4, g::<u32>(TIME_MS).read());
        wr::<i32>(this, 8, -1);
        wr::<u8>(this, 0xC, 1);
    }
    scan_vehicle_potential_collision(this.byte_add(4), ped, intel.byte_add(0xE0), 0x10);
}

/// `FUN_00667310` (0x667310): `CCompressedVector` decompress: the i16s at +8/+A/+C × 0.125 into `out`.
#[hook(addr = 0x667310, conv = thiscall, name = "FUN_00667310", expect = "83 EC 10 0F BF 41 08")]
pub fn fun_00667310(this: GPtr<u8>, out: GPtr<u8>) {
    for i in 0..3 {
        let v = xi(i32::from(rd::<i16>(this, 8 + 2 * i))).fmul_m(0.125);
        wr::<f32>(out, 4 * i, v.to_f32());
    }
}

/// `FUN_0066e050` (0x66E050): task destructor: `vtbl = 0x870078`, `CPointRoute::operator delete(+0x20)`
/// when set, then the `CTaskComplex` base destructor.
#[hook(addr = 0x66E050, conv = thiscall, name = "FUN_0066e050", expect = "6A FF 68 28 55 84 00")]
pub fn fun_0066e050(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x870078);
    let r = rd::<GPtr<u8>>(this, 0x20);
    if !r.is_null() {
        point_route_delete(r);
    }
    task_complex_dtor(this);
}

// ---- batch 2 -------------------------------------------------------------------------------------

orig! {
    /// `FUN_005eec90` (0x5EEC90): `CPedStepAttractor` complete destructor
    fn step_attractor_dtor(this: GPtr<u8>) = thiscall 0x5EEC90 => unported;
    /// `CPedAttractorManager::CPedAttractorManager` (0x5EDF10)
    fn attractor_manager_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x5EDF10 => unported;
    /// `CPedAttractor::Constructor` (0x5EDFB0)
    fn ped_attractor_ctor(this: GPtr<u8>, a0: u32, a1: u32, a2: u32, a3: u32, c0: u32, c1: u32, c2: u32, c3: u32, c4: u32, c5: u32, c6: u32) -> GPtr<u8> = thiscall 0x5EDFB0 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CTaskUtilityLineUpPedWithCar::Constructor(const CVector&, int, int, int)` (0x64FBB0)
    fn line_up_ped_with_car_ctor(this: GPtr<u8>, offset: GPtr<u8>, a: u32, b: u32, c: u32) -> GPtr<u8> = thiscall 0x64FBB0 => unported;
    /// `CVector::Normalise` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CPedGroupMembership::RemoveMember(int)` (0x5F80D0)
    fn membership_remove_member(this: GPtr<u8>, i: i32) = thiscall 0x5F80D0 => unported;
    /// `CPedGroupIntelligence::Flush` (0x5F7350)
    fn group_intelligence_flush(this: GPtr<u8>) = thiscall 0x5F7350 => unported;
    /// `thunk_FUN_01567930` (0x4650F0): `CDecisionMakerTypes` constructor
    fn decision_maker_types_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4650F0 => unported;
    /// `CPedAttractorManager::FindAssociatedAttractor<CPedAttractor>` (0x5EB6F0, `ret 0xC`)
    fn find_associated_attractor(effect: GPtr<u8>, entity: u32, vec: GPtr<u8>) -> GPtr<u8> = stdcall 0x5EB6F0 => unported;
    /// `CPedAttractor::HasEmptySlot` (0x5EAF10)
    fn ped_attractor_has_empty_slot(this: GPtr<u8>) -> u8 = thiscall 0x5EAF10 => unported;
    /// `CPedAttractor::IsRegisteredWithPed(CPed*)` (0x5EB4C0)
    fn ped_attractor_is_registered_with_ped(this: GPtr<u8>, ped: u32) -> u8 = thiscall 0x5EB4C0 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// scalar deleting destructor (vtable +0)
    fn entity_delete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
}

/// `CPools::ms_pPedAttractorPool` (slot 0xC4)
const PED_ATTRACTOR_POOL: usize = 0xB744C4;
/// `CPedGroups::ms_groups` (8 × 0x2D4)
const GROUPS: u32 = 0xC09920;
/// `CPedGroups::ms_activeGroups` (`bool[8]`)
const ACTIVE_GROUPS: usize = 0xC098E0;

/// `CPedStepAttractor::~CPedStepAttractor` (0x5EEC40): scalar deleting; `FUN_005eec90`, then the
/// inlined delete from the ped attractor pool (slot 0xC4).
#[hook(addr = 0x5EEC40, conv = thiscall, name = "CPedStepAttractor::~CPedStepAttractor", expect = "56 57 8B F9 E8 47 00 00 00")]
pub fn ped_step_attractor_sdd_5eec40(this: GPtr<u8>, flags: u32) -> GPtr<u8> {
    step_attractor_dtor(this);
    if flags & 1 != 0 {
        crate::events::pool_delete_inline(PED_ATTRACTOR_POOL, this, 0xC4);
    }
    this
}

/// `CPedAttractorManager::GetPedAttractorManager` (0x5EE190): the singleton at 0xC09840, created
/// (CRT `operator new(0xA0)` + ctor) on first use.
#[hook(addr = 0x5EE190, conv = cdecl, name = "CPedAttractorManager::GetPedAttractorManager", expect = "6A FF 68 8B DC 83 00")]
pub fn get_ped_attractor_manager_5ee190() -> GPtr<u8> {
    let cur = PED_ATTRACTOR_MANAGER.get();
    if !cur.is_null() {
        return cur;
    }
    let p = crt_new(0xA0);
    let r = if p.is_null() {
        GPtr::NULL
    } else {
        attractor_manager_ctor(p)
    };
    PED_ATTRACTOR_MANAGER.set(r);
    r
}

/// `CDecisionMakerTypes::GetInstance` (0x4684F0): the singleton at 0xC0B030, created (CRT
/// `operator new(0xF1C0)` + ctor 0x4650F0) on first use. (Its entry jumps through a protection stub
/// that pushes the SEH handler.)
#[hook(addr = 0x4684F0, conv = cdecl, name = "CDecisionMakerTypes::GetInstance", expect = "6A FF E9 F6 9C F9 FF")]
pub fn decision_maker_types_get_instance_4684f0() -> GPtr<u8> {
    let cur = DECISION_MAKER_TYPES.get();
    if !cur.is_null() {
        return cur;
    }
    let p = crt_new(0xF1C0);
    let r = if p.is_null() {
        GPtr::NULL
    } else {
        decision_maker_types_ctor(p)
    };
    DECISION_MAKER_TYPES.set(r);
    r
}

/// `CPedStepAttractor::CPedStepAttractor(C2dEffect*, CEntity*, int)` (0x5EEBD0): the base ctor with
/// `(a0, a1, a2, 1)` and the seven dwords at 0x86C510..0x86C528, then `vtbl = 0x86C62C`.
#[hook(addr = 0x5EEBD0, conv = thiscall, name = "CPedStepAttractor::CPedStepAttractor", expect = "A1 28 C5 86 00")]
pub fn ped_step_attractor_ctor_5eebd0(this: GPtr<u8>, a0: u32, a1: u32, a2: u32) -> GPtr<u8> {
    let c = |i: usize| g::<u32>(0x86C510 + 4 * i).read();
    ped_attractor_ctor(this, a0, a1, a2, 1, c(0), c(1), c(2), c(3), c(4), c(5), c(6));
    wr::<u32>(this, 0, 0x86C62C);
    this
}

/// `FUN_00646cf0` (0x646CF0, cdecl): door `d` 8/9/10/11 → mask 4/8/1/2; sets `*a` when
/// `ped[+0x486] & mask`, `*b` when `ped[+0x487] & mask`.
#[hook(addr = 0x646CF0, conv = cdecl, name = "FUN_00646cf0", expect = "8B 44 24 08 83 C0 F8")]
pub fn fun_00646cf0(p: GPtr<u8>, d: i32, a: GPtr<u8>, b: GPtr<u8>) {
    let mask = match d {
        8 => 4,
        9 => 8,
        10 => 1,
        11 => 2,
        _ => return,
    };
    if rd::<u8>(p, 0x486) & mask != 0 {
        wr::<u8>(a, 0, 1);
    }
    if rd::<u8>(p, 0x487) & mask != 0 {
        wr::<u8>(b, 0, 1);
    }
}

/// `FUN_005bd360` (0x5BD360, cdecl): first char `H/L/G/B` → 0..3; otherwise 7 (the two-char
/// comparisons against `"LH"`, `"LL"`, `"LG"` compare a zero-extended byte and never match). `preserve = "ecx"`:
/// `CTaskSimpleFight::LoadMeleeData` keeps its table index in ECX across this call and the next one
/// (0x5BF1FD .. 0x5BF241); found by the weapon-combat all-ports run (rand divergence at frame 1541).
#[hook(addr = 0x5BD360, conv = cdecl, name = "FUN_005bd360", expect = "8B 44 24 04 8A 00", preserve = "ecx")]
pub fn fun_005bd360(s: GPtr<u8>) -> u8 {
    match rd::<u8>(s, 0) {
        0x48 => 0,
        0x4C => 1,
        0x47 => 2,
        0x42 => 3,
        _ => 7,
    }
}

/// `FUN_006550d0` (0x6550D0): task destructor: `vtbl = 0x86F364`, clean up the entity reference at
/// +0x1C, `CPointRoute::operator delete(+0x20)`, then the `CTaskComplex` base destructor.
#[hook(addr = 0x6550D0, conv = thiscall, name = "FUN_006550d0", expect = "6A FF 68 08 37 84 00")]
pub fn fun_006550d0(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86F364);
    let e = rd::<GPtr<u8>>(this, 0x1C);
    if !e.is_null() {
        entity_clean_up_old_reference(e, this.byte_add(0x1C));
    }
    let r = rd::<GPtr<u8>>(this, 0x20);
    if !r.is_null() {
        point_route_delete(r);
    }
    task_complex_dtor(this);
}

/// `FUN_0063bae0` (0x63BAE0): `+0x1C = new CTaskUtilityLineUpPedWithCar(CVector(0,0,0), 0, 0, +0x10)`
/// (CRT `operator new(0x1C)`; null when that fails); returns it.
#[hook(addr = 0x63BAE0, conv = thiscall, name = "FUN_0063bae0", expect = "6A FF 68 5B 16 84 00")]
pub fn fun_0063bae0(this: GPtr<u8>, _a0: u32) -> GPtr<u8> {
    let mut v = [0u32; 3];
    let p = crt_new(0x1C);
    let r = if p.is_null() {
        GPtr::NULL
    } else {
        let vp: GPtr<u8> = GPtr::from_raw(&mut v as *mut [u32; 3]).cast();
        line_up_ped_with_car_ctor(p, vp, 0, 0, rd::<u32>(this, 0x10))
    };
    wr::<u32>(this, 0x1C, r.addr() as u32);
    r
}

/// `FUN_004ace80` (0x4ACE80): the XY of the vector at +0x18 (z = 0), normalised, against the ped's
/// matrix forward (+0x10): `-(y*fy + z*fz + x*fx) > 0.9397`.
#[hook(addr = 0x4ACE80, conv = thiscall, name = "FUN_004ace80", expect = "83 EC 0C 8B 44 24 10")]
pub fn fun_004ace80(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let f = rd::<GPtr<u8>>(ped, 0x14).byte_add(0x10);
    let mut v = [rd::<f32>(this, 0x18), rd::<f32>(this, 0x1C), 0.0f32];
    vector_normalise(GPtr::from_raw(&mut v as *mut [f32; 3]).cast());
    let d = x(v[1])
        .fmul_m(rd::<f32>(f, 4))
        .fadd(x(v[2]).fmul_m(rd::<f32>(f, 8)));
    let d = d.fadd(x(v[0]).fmul_m(rd::<f32>(f, 0))).fchs();
    let c = d.fcom(x(f32::from_bits(0x3F70_902E)));
    u8::from(!c.test_any(0x41))
}

/// `CPedGroups::Init` (0x5FB8C0): every active group is deactivated, its members removed (slot by
/// slot), its intelligence flushed and `+0x2D0` cleared; every `ms_bGroupNeedsScriptReference`
/// word (0xC098D0) is set to 1.
#[hook(addr = 0x5FB8C0, conv = cdecl, name = "CPedGroups::Init", expect = "53 55 56 57 33 DB")]
pub fn ped_groups_init_5fb8c0() {
    for k in 0..8u32 {
        let grp = g::<u8>((GROUPS + k * 0x2D4) as usize);
        let act = g::<u8>(ACTIVE_GROUPS + k as usize);
        if act.read() != 0 {
            act.write(0);
            for i in 0..8 {
                if rd::<u32>(grp, 0xC + 4 * i as isize) != 0 {
                    membership_remove_member(grp.byte_add(8), i);
                }
            }
            group_intelligence_flush(grp.byte_add(0x30));
            wr::<u8>(grp, 0x2D0, 0);
        }
        g::<u16>(0xC098D0 + 2 * k as usize).write(1);
    }
}

/// `CPedGroups::GetPedsGroup(CPed*)` (0x5F7E80): the first active group whose membership slots
/// (+0xC..+0x28) hold `ped` (non-null), else null.
#[hook(addr = 0x5F7E80, conv = cdecl, name = "CPedGroups::GetPedsGroup", expect = "56 57 8B 7C 24 0C")]
pub fn ped_groups_get_peds_group_5f7e80(ped: u32) -> u32 {
    for k in 0..8u32 {
        let grp = g::<u8>((GROUPS + k * 0x2D4) as usize);
        if g::<u8>(ACTIVE_GROUPS + k as usize).read() == 0 || ped == 0 {
            continue;
        }
        if (0..8).any(|i| rd::<u32>(grp, 0xC + 4 * i) == ped) {
            return GROUPS + k * 0x2D4;
        }
    }
    0
}

/// `CPedGroups::AddGroup` (0x5FB800): the first inactive group is activated, its members removed,
/// its intelligence flushed and `+0x2D0` cleared; returns its index, or -1 when all 8 are active.
#[hook(addr = 0x5FB800, conv = cdecl, name = "CPedGroups::AddGroup", expect = "53 83 C8 FF 33 DB")]
pub fn ped_groups_add_group_5fb800() -> i32 {
    for k in 0..8u32 {
        let act = g::<u8>(ACTIVE_GROUPS + k as usize);
        if act.read() != 0 {
            continue;
        }
        let grp = g::<u8>((GROUPS + k * 0x2D4) as usize);
        act.write(1);
        for i in 0..8 {
            if rd::<u32>(grp, 0xC + 4 * i as isize) != 0 {
                membership_remove_member(grp.byte_add(8), i);
            }
        }
        group_intelligence_flush(grp.byte_add(0x30));
        wr::<u8>(grp, 0x2D0, 0);
        return k as i32;
    }
    -1
}

/// `CPedAttractorManager::CPedAttractorManager` (0x5EDF10): zeroes the three dwords at +4/+8/+0xC of
/// each of the ten `std::vector`s (stride 0x10); returns `this`.
#[hook(addr = 0x5EDF10, conv = thiscall, name = "CPedAttractorManager::CPedAttractorManager", expect = "64 A1 00 00 00 00")]
pub fn ped_attractor_manager_ctor_5edf10(this: GPtr<u8>) -> GPtr<u8> {
    for k in 0..10 {
        for o in [4, 8, 0xC] {
            wr::<u32>(this, k * 0x10 + o, 0);
        }
    }
    this
}

/// `CPedAttractorManager::HasEmptySlot(const C2dEffect*, const CEntity*)` (0x5EBB00): only for
/// attractor effects (`+0xC == 3`); the attractor list by type (`+0x34`: 0 → +0x10, 1 → +0,
/// k → +0x10*k, others → true); no associated attractor → true, else `CPedAttractor::HasEmptySlot`.
#[hook(addr = 0x5EBB00, conv = thiscall, name = "CPedAttractorManager::HasEmptySlot", expect = "8B 44 24 04 85 C0")]
pub fn ped_attractor_manager_has_empty_slot_5ebb00(this: GPtr<u8>, effect: GPtr<u8>, entity: u32) -> u8 {
    if effect.is_null() || rd::<u8>(effect, 0xC) != 3 {
        return 0;
    }
    let off: isize = match rd::<u8>(effect, 0x34) {
        0 => 0x10,
        1 => 0,
        t @ 2..=9 => 0x10 * t as isize,
        _ => return 1,
    };
    let a = find_associated_attractor(effect, entity, this.byte_add(off));
    if a.is_null() {
        return 1;
    }
    ped_attractor_has_empty_slot(a)
}

/// `CPedAttractorManager::IsPedRegisteredWithEffect<CPedAttractor>(ped, effect, entity, vector&)`
/// (0x5EB690, `ret 0x10`, ECX unused): any attractor of the vector with that effect (+4) and entity
/// (+8) that is registered with `ped`.
#[hook(addr = 0x5EB690, conv = stdcall, name = "CPedAttractorManager::IsPedRegisteredWithEffect<CPedAttractor>", expect = "8B 44 24 10 53")]
pub fn ped_attractor_manager_is_ped_registered_with_effect_5eb690(
    ped: u32,
    effect: u32,
    entity: u32,
    vec: GPtr<u8>,
) -> u8 {
    let begin = rd::<GPtr<u8>>(vec, 4);
    let n = if begin.is_null() {
        0
    } else {
        (rd::<u32>(vec, 8).wrapping_sub(begin.addr() as u32) as i32) >> 2
    };
    for i in 0..n.max(0) {
        let a = rd::<GPtr<u8>>(begin, 4 * i as isize);
        if rd::<u32>(a, 4) == effect
            && rd::<u32>(a, 8) == entity
            && ped_attractor_is_registered_with_ped(a, ped) != 0
        {
            return 1;
        }
    }
    0
}

/// `FUN_0061aeb0` (0x61AEB0): task destructor: `vtbl = 0x86D5B8`; for the objects at +0x1C and +0x20:
/// `CWorld::Remove`, delete (vtable +0, flag 1) when still set, `--word[0xBB4A70]`; then
/// `CTaskSimpleAnim::~CTaskSimpleAnim`.
#[hook(addr = 0x61AEB0, conv = thiscall, name = "FUN_0061aeb0", expect = "6A FF 68 68 F1 83 00")]
pub fn fun_0061aeb0(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x86D5B8);
    for off in [0x1C, 0x20] {
        let o = rd::<GPtr<u8>>(this, off);
        if o.is_null() {
            continue;
        }
        world_remove(o);
        let o = rd::<GPtr<u8>>(this, off);
        if !o.is_null() {
            entity_delete(o, 1);
        }
        let c = g::<u16>(0xBB4A70);
        c.write(c.read().wrapping_sub(1));
    }
    task_simple_anim_dtor(this);
}

orig! {
    /// `CPedGroupMembership::IsMember(CPed*)` (0x5F6A10)
    fn membership_is_member(this: GPtr<u8>, ped: u32) -> u8 = thiscall 0x5F6A10 => unported;
}

/// `CPedGroups::AreInSameGroup(CPed*, CPed*)` (0x5F7F40): both non-null and some active group holds
/// `a` in a member slot (+0xC..+0x28) and has `b` as a member (`IsMember`).
#[hook(addr = 0x5F7F40, conv = cdecl, name = "CPedGroups::AreInSameGroup", expect = "53 8B 5C 24 08")]
pub fn ped_groups_are_in_same_group_5f7f40(a: u32, b: u32) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    for k in 0..8u32 {
        if g::<u8>(ACTIVE_GROUPS + k as usize).read() == 0 {
            continue;
        }
        let grp = g::<u8>((GROUPS + k * 0x2D4) as usize);
        if (0..8).any(|i| rd::<u32>(grp, 0xC + 4 * i) == a) && membership_is_member(grp.byte_add(8), b) != 0 {
            return 1;
        }
    }
    0
}

// ---- batch 3 -------------------------------------------------------------------------------------

orig! {
    /// `CTaskManager::FindActiveTaskByType(int)` (0x681740)
    fn task_manager_find_active_task_by_type(tm: GPtr<u8>, t: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CAttractorScanner::Clear` (0x5FFF90)
    fn attractor_scanner_clear(this: GPtr<u8>) = thiscall 0x5FFF90 => unported;
    /// `CPed::IsAlive` (0x5E0170)
    fn ped_is_alive(ped: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CEntityScanner::ScanForEntitiesInRange(int, const CPed&)` (0x5FFA20)
    fn entity_scanner_scan_in_range(this: GPtr<u8>, kind: i32, ped: GPtr<u8>) = thiscall 0x5FFA20 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(cb, data)` (0x4CEBE0)
    fn assoc_set_finish_callback(a: GPtr<u8>, cb: u32, data: u32) = thiscall 0x4CEBE0 => unported;
    /// `CAnimManager::RemoveAnimBlockRef(int)` (0x4D3FD0)
    fn remove_anim_block_ref(i: i32) = cdecl 0x4D3FD0 => unported;
    /// `FUN_0061a3a0` (0x61A3A0): `CTaskSimple` base destructor
    fn task_simple_dtor(this: GPtr<u8>) = thiscall 0x61A3A0 => unported;
    /// `CDecision::From(const CDecision&)` (0x6006B0)
    fn decision_from(this: GPtr<u8>, src: GPtr<u8>) = thiscall 0x6006B0 => unported;
}

/// `(missed by ghidra: cat7-call)` (0x6822B0; the body continues through the straight protection stub
/// 0x40806B `eax = ped->m_pIntelligence`): the active task of type `+0x34` of the ped at +0x38 →
/// its signed byte at +0x59, else 0.
#[hook(addr = 0x6822B0, conv = thiscall, name = "(missed by ghidra: cat7-call)", expect = "8B 41 38 8B 49 34")]
pub fn active_task_byte_59_6822b0(this: GPtr<u8>) -> i32 {
    let intel = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x38), 0x47C);
    let t = task_manager_find_active_task_by_type(intel.byte_add(4), rd::<i32>(this, 0x34));
    if t.is_null() {
        0
    } else {
        i32::from(rd::<i8>(t, 0x59))
    }
}

/// `CEventScanner::CEventScanner` (0x605300): clears the scanners' timers (+4.., +0x1C.., +0xAC..),
/// `CAttractorScanner::Clear(this + 0x1C)`, then `+0 = now - ftol((rand() & 0xFFFF) * 2^-15 * -3000)`
/// (the time is read before the draw; rand site 0x6053A3); returns `this`.
#[hook(addr = 0x605300, conv = thiscall, name = "CEventScanner::CEventScanner", expect = "51 53 33 DB 56")]
pub fn event_scanner_ctor_605300(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 8, 0);
    wr::<u8>(this, 0xC, 0);
    wr::<u8>(this, 0xD, 0);
    wr::<u32>(this, 0x10, 0);
    wr::<u32>(this, 0x14, 0);
    wr::<u8>(this, 0x18, 0);
    wr::<u8>(this, 0x19, 0);
    let s = this.byte_add(0x1C);
    wr::<u8>(s, 0, 1);
    wr::<u32>(s, 4, 0);
    wr::<u32>(s, 8, 0);
    wr::<u8>(s, 0xC, 0);
    wr::<u8>(s, 0xD, 0);
    wr::<u32>(s, 0x10, 0);
    wr::<u32>(s, 0x14, 0);
    attractor_scanner_clear(s);
    wr::<u32>(this, 0xAC, 0);
    wr::<u32>(this, 0xB0, 0);
    wr::<u8>(this, 0xB4, 0);
    wr::<u8>(this, 0xB5, 0);
    wr::<u8>(this, 0xB8, 1);
    wr::<u8>(this, 0xB9, 0);
    wr::<u8>(this, 0xBA, 0);
    wr::<u32>(this, 0xBC, 0);
    wr::<u32>(this, 0xC0, 0);
    wr::<u8>(this, 0xC4, 0);
    wr::<u8>(this, 0xC5, 0);
    wr::<u32>(this, 0xC8, 0);
    wr::<u32>(this, 0xCC, 0);
    wr::<u8>(this, 0xD0, 0);
    wr::<u8>(this, 0xD1, 0);
    let now = g::<u32>(TIME_MS).read();
    let r = at_site(0x6053A3, rand) & 0xFFFF;
    let d = ftol32(xi(r).fmul_m(f32::from_bits(0x3800_0000)).fmul_m(-3000.0));
    wr::<u32>(this, 0, now.wrapping_sub(d as u32));
    this
}

/// `CVehicleScanner::ScanForVehiclesInRange(const CPed&)` (0x5FFE60): only for a live ped; in a
/// vehicle (`+0x58C`, flag `+0x46C` bit 8): a player (`+0x484 == 2`) scans first; then the closest
/// vehicle (+0x4C) becomes the ped's vehicle (reference moved); otherwise a plain scan.
#[hook(addr = 0x5FFE60, conv = thiscall, name = "CVehicleScanner::ScanForVehiclesInRange", expect = "56 57 8B 7C 24 0C")]
pub fn vehicle_scanner_scan_for_vehicles_in_range_5ffe60(this: GPtr<u8>, ped: GPtr<u8>) {
    if ped_is_alive(ped) == 0 {
        return;
    }
    if rd::<u32>(ped, 0x58C) == 0 || rd::<u32>(ped, 0x46C) & 0x100 == 0 {
        entity_scanner_scan_in_range(this, 0, ped);
        return;
    }
    if rd::<u8>(ped, 0x484) == 2 {
        entity_scanner_scan_in_range(this, 0, ped);
    }
    let slot = this.byte_add(0x4C);
    let old = rd::<GPtr<u8>>(slot, 0);
    if !old.is_null() {
        entity_clean_up_old_reference(old, slot);
    }
    let v = rd::<GPtr<u8>>(ped, 0x58C);
    wr::<u32>(slot, 0, v.addr() as u32);
    if !v.is_null() {
        entity_register_reference(v, slot);
    }
}

/// `FUN_00691630` (0x691630): anim task destructor: `vtbl = 0x870B2C`; unless `+0x35`, the entity at
/// +8 gets flag 0x400 cleared, its reference cleaned up, flag 0x800 set and the slot cleared; the
/// association at +0x38 gets the finish callback 0x4D1390 (data 0); the anim block at +0x2C loses a
/// ref (`(p - 0xB5D4A0) >> 5`); then the `CTaskSimple` base destructor.
#[hook(addr = 0x691630, conv = thiscall, name = "FUN_00691630", expect = "6A FF 68 A8 72 84 00")]
pub fn fun_00691630(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x870B2C);
    if rd::<u8>(this, 0x35) == 0 {
        let slot = this.byte_add(8);
        let e = rd::<GPtr<u8>>(slot, 0);
        if !e.is_null() {
            wr::<u32>(e, 0x1C, rd::<u32>(e, 0x1C) & 0xFFFF_FBFF);
            entity_clean_up_old_reference(rd::<GPtr<u8>>(slot, 0), slot);
            let e = rd::<GPtr<u8>>(slot, 0);
            wr::<u32>(e, 0x1C, rd::<u32>(e, 0x1C) | 0x800);
            wr::<u32>(slot, 0, 0);
        }
    }
    let a = rd::<GPtr<u8>>(this, 0x38);
    if !a.is_null() {
        assoc_set_finish_callback(a, 0x4D1390, 0);
    }
    let b = rd::<u32>(this, 0x2C);
    if b != 0 {
        remove_anim_block_ref((b.wrapping_sub(0xB5D4A0) as i32) >> 5);
    }
    task_simple_dtor(this);
}

/// `CDecisionSimple::MakeDecision(int event, int16* task, int* index)` (0x6007A0): `*task = 200`,
/// `*index = -1`; an exact match of `event` in the first `count (+0x30)` dwords wins (the word at
/// that slot is stored, as the bytes do); otherwise one `rand()` (site 0x6007F2) picks the first
/// slot whose probability (+0x18) is `>=` `rand * 3.0518509e-5` (stored as f32).
#[hook(addr = 0x6007A0, conv = thiscall, name = "CDecisionSimple::MakeDecision", expect = "8B 54 24 04 83 FA FF")]
pub fn decision_simple_make_decision_6007a0(this: GPtr<u8>, ev: i32, task: GPtr<u8>, index: GPtr<u8>) {
    wr::<u16>(task, 0, 200);
    wr::<i32>(index, 0, -1);
    let n = rd::<i32>(this, 0x30);
    if ev != -1 {
        for i in 0..n.max(0) {
            if rd::<i32>(this, 4 * i as isize) == ev {
                wr::<u16>(task, 0, rd::<u16>(this, 4 * i as isize));
                wr::<i32>(index, 0, i);
                break;
            }
        }
    }
    if rd::<u16>(task, 0) != 200 {
        return;
    }
    let r = at_site(0x6007F2, rand);
    let f = xi(r).fmul_m(f32::from_bits(0x3800_0100)).to_f32();
    let n = rd::<i32>(this, 0x30);
    for i in 0..n.max(0) {
        let c = x(f).fcom(x(rd::<f32>(this, 0x18 + 4 * i as isize)));
        if !c.test_parity(0x41) {
            wr::<u16>(task, 0, rd::<u16>(this, 4 * i as isize));
            wr::<i32>(index, 0, i);
            return;
        }
    }
}

/// `CDecisionMakerTypes::MakeDecision(CPedGroup*, int event, ...)` (0x606F80, `ret 0x20`): the
/// group's decision maker (`+0x2C8`; -1 → the default group table, `+0x2D0 == 1` → 0xE824 else
/// 0xDE88) entry for the event's index; `CDecision::MakeDecision(a2..a7, &task, &a7)` with
/// `task = 200`; returns `(int16) task`.
#[hook(addr = 0x606F80, conv = thiscall, name = "CDecisionMakerTypes::MakeDecision-pedgroup-ptr", expect = "8B 54 24 08 8B 94 91 34 C0 00 00")]
#[allow(clippy::too_many_arguments)]
pub fn decision_maker_types_make_decision_group_606f80(
    this: GPtr<u8>,
    group: GPtr<u8>,
    ev: i32,
    a2: u32,
    a3: u32,
    a4: u32,
    a5: u32,
    a6: u32,
    a7: u32,
) -> i32 {
    let idx = rd::<i32>(this, 0xC034 + 4 * ev as isize);
    let dm = rd::<i32>(group, 0x2C8);
    let mut task: u32 = 0xC8;
    let mut a7_slot: u32 = a7;
    let base = if dm == -1 {
        let off: isize = if rd::<u8>(group, 0x2D0) == 1 {
            0xE824
        } else {
            0xDE88
        };
        this.byte_add(off + idx.wrapping_mul(0x3C) as isize)
    } else {
        this.byte_add(4 + dm.wrapping_mul(0x29).wrapping_add(idx).wrapping_mul(0x3C) as isize)
    };
    let tp: GPtr<u8> = GPtr::from_raw(&mut task as *mut u32).cast();
    let ap: GPtr<u8> = GPtr::from_raw(&mut a7_slot as *mut u32).cast();
    decision_make_decision(base, a2, a3, a4, a5, a6, a7, tp.addr() as u32, ap.addr() as u32);
    i32::from(task as u16 as i16)
}

/// `CDecisionMakerTypes::MakeDecision(CPed*, int event, ..., bool group, int16* task, int16* facial)`
/// (0x606E70, `ret 0x2C`): `*task = 200`, `*facial = -1`; the ped's decision maker
/// (`intel +0xB8` when `a8`, else `+0xB4`): -2 → the table at +0xD4EC; -1 → `FUN_00606060` on the
/// player (+0xCB50, `+0x484 == 2`) or ped (+0xC1B4) default tables; else the maker's entry.
#[hook(addr = 0x606E70, conv = thiscall, name = "CDecisionMakerTypes::MakeDecision-ped-ptr", expect = "8A 44 24 24 84 C0")]
#[allow(clippy::too_many_arguments)]
pub fn decision_maker_types_make_decision_ped_606e70(
    this: GPtr<u8>,
    ped: GPtr<u8>,
    ev: i32,
    a2: u32,
    a3: u32,
    a4: u32,
    a5: u32,
    a6: u32,
    a7: u32,
    a8: u8,
    task: GPtr<u8>,
    facial: GPtr<u8>,
) {
    let intel = rd::<GPtr<u8>>(ped, 0x47C);
    let dm = if a8 != 0 {
        rd::<i32>(intel, 0xB8)
    } else {
        rd::<i32>(intel, 0xB4)
    };
    let idx = rd::<i32>(this, 0xC034 + 4 * ev as isize);
    wr::<u16>(task, 0, 200);
    wr::<u16>(facial, 0, 0xFFFF);
    let (t, f) = (task.addr() as u32, facial.addr() as u32);
    match dm {
        -2 => decision_make_decision(
            this.byte_add(0xD4EC + idx.wrapping_mul(0x3C) as isize),
            a2,
            a3,
            a4,
            a5,
            a6,
            a7,
            t,
            f,
        ),
        -1 => {
            let off: isize = if rd::<u8>(ped, 0x484) == 2 { 0xCB50 } else { 0xC1B4 };
            fun_00606060_call(this.byte_add(off), idx, a2, a3, a4, a5, a6, a7, t, f)
        }
        _ => decision_make_decision(
            this.byte_add(4 + dm.wrapping_mul(0x29).wrapping_add(idx).wrapping_mul(0x3C) as isize),
            a2,
            a3,
            a4,
            a5,
            a6,
            a7,
            t,
            f,
        ),
    }
}

orig! {
    /// `FUN_00606060` (0x606060), through its hook toggle
    fn fun_00606060_call(this: GPtr<u8>, a0: i32, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32) = thiscall 0x606060 => unported;
}

/// `CDecisionMakerTypes::AddDecisionMaker(CDecisionMaker* src, uint8 type, bool group)` (0x607050,
/// `ret 0xC`; the group branch sets its range through the straight protection stub 0x406A7C): the
/// first free slot (`bool[0xC0B01C]`) of 15..20 (group) or 0..15 gets active, its type (0xC0AFE0),
/// and the 0x29 decisions copied (`CDecision::From`); `++count (+0)`; returns the slot or -1.
#[hook(addr = 0x607050, conv = thiscall, name = "CDecisionMakerTypes::AddDecisionMaker", expect = "51 8A 54 24 10")]
pub fn decision_maker_types_add_decision_maker_607050(
    this: GPtr<u8>,
    src: GPtr<u8>,
    kind: u8,
    group: u8,
) -> i32 {
    let (lo, hi) = if group != 0 { (0xF, 0x14) } else { (0, 0xF) };
    for k in lo..hi {
        let act = g::<u8>(0xC0B01C + k);
        if act.read() != 0 {
            continue;
        }
        act.write(1);
        g::<u8>(0xC0AFE0 + k).write(kind);
        let dst = this.byte_add(4 + 0x99C * k as isize);
        for i in 0..0x29 {
            decision_from(dst.byte_add(0x3C * i), src.byte_add(0x3C * i));
        }
        wr::<u32>(this, 0, rd::<u32>(this, 0).wrapping_add(1));
        return k as i32;
    }
    -1
}

orig! {
    /// `CPedIntelligence::SetPedDecisionMakerType(int)` (0x600B50)
    fn intel_set_ped_decision_maker_type(intel: GPtr<u8>, t: i32) = thiscall 0x600B50 => unported;
    /// `CPedGroupIntelligence::SetGroupDecisionMakerType(int)` (0x5F7340)
    fn group_intel_set_decision_maker_type(intel: GPtr<u8>, t: i32) = thiscall 0x5F7340 => unported;
}

/// `CDecisionMakerTypes::RemoveDecisionMaker(int)` (0x6043A0): only for an active slot. A ped maker
/// (type 0): every ped of the pool (scanned from the last slot down, free slots and a null object
/// skipped) using it (`intel +0xB4`) gets its default type (`(int8) ped->m_pStats[+0x32]`). A group
/// maker: every active group using it (`+0x2C8`) gets -1. Then the slot is freed, its type cleared
/// and `count (+0)` decremented.
#[hook(addr = 0x6043A0, conv = thiscall, name = "CDecisionMakerTypes::RemoveDecisionMaker", expect = "51 53 8B 5C 24 0C")]
pub fn decision_maker_types_remove_decision_maker_6043a0(this: GPtr<u8>, idx: i32) {
    let act = g::<u8>((0xC0B01C + idx) as u32 as usize);
    if act.read() == 0 {
        return;
    }
    let kind = g::<u8>((0xC0AFE0 + idx) as u32 as usize);
    if kind.read() == 0 {
        let pool = g::<GPtr<u8>>(0xB74490).read();
        let mut i = rd::<i32>(pool, 8);
        while i != 0 {
            i -= 1;
            if (rd::<GPtr<u8>>(pool, 4).byte_add(i as isize).read() as i8) < 0 {
                continue;
            }
            let ped = rd::<u32>(pool, 0).wrapping_add((i as u32).wrapping_mul(0x7C4));
            if ped == 0 {
                continue;
            }
            let ped = GPtr::<u8>::from_addr(ped as usize);
            let intel = rd::<GPtr<u8>>(ped, 0x47C);
            if rd::<i32>(intel, 0xB4) == idx {
                let t = rd::<i8>(rd::<GPtr<u8>>(ped, 0x59C), 0x32);
                intel_set_ped_decision_maker_type(intel, i32::from(t));
            }
        }
    } else {
        for k in 0..8u32 {
            let grp = g::<u8>((GROUPS + k * 0x2D4) as usize);
            if g::<u8>(ACTIVE_GROUPS + k as usize).read() != 0 && rd::<i32>(grp, 0x2C8) == idx {
                group_intel_set_decision_maker_type(grp.byte_add(0x30), -1);
            }
        }
    }
    act.write(0);
    kind.write(0);
    wr::<i32>(this, 0, rd::<i32>(this, 0).wrapping_sub(1));
}

// ---- batch 5 -------------------------------------------------------------------------------------

orig! {
    /// `CCarEnterExit::ComputeDoorFlag(CVehicle*, int door, bool)` (0x64E550)
    fn compute_door_flag(veh: GPtr<u8>, door: u32, b: u32) -> u8 = cdecl 0x64E550 => unported;
    /// `CVehicle::SetGettingOutFlags(uint8)` (0x6D3020)
    fn vehicle_set_getting_out_flags(veh: GPtr<u8>, flags: u32) = thiscall 0x6D3020 => unported;
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player_m(ped: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CDecisionSimple::MakeDecision(int, int16*, int*)` (0x6007A0), through its hook toggle
    fn decision_simple_make_decision(this: GPtr<u8>, ev: i32, task: GPtr<u8>, index: GPtr<u8>) = thiscall 0x6007A0 => unported;
}

/// `FUN_0063ba00` (0x63BA00): car-exit start: `+0x20 = ComputeDoorFlag(veh +0xC, door +0x10, 1)`,
/// `veh->SetGettingOutFlags` (the pushed dword is that byte over the upper bytes of the saved
/// `this`), `+0x21 = 1`, `++veh[+0x485]`; a non-player driver that is `ped` with `+0x18` set clears
/// `veh[+0x3D0]`/`[+0x3BA]`; a player driver that is `ped` gets `veh[+0x36] = (x & 7) | 0x48`.
#[hook(addr = 0x63BA00, conv = thiscall, name = "FUN_0063ba00", expect = "51 56 57 8B F1")]
pub fn fun_0063ba00(this: GPtr<u8>, ped: GPtr<u8>) {
    let veh = rd::<GPtr<u8>>(this, 0xC);
    let f = compute_door_flag(veh, rd::<u32>(this, 0x10), 1);
    wr::<u8>(this, 0x20, f);
    vehicle_set_getting_out_flags(
        rd::<GPtr<u8>>(this, 0xC),
        (this.addr() as u32 & 0xFFFF_FF00) | u32::from(f),
    );
    wr::<u8>(this, 0x21, 1);
    let veh = rd::<GPtr<u8>>(this, 0xC);
    wr::<u8>(veh, 0x485, rd::<u8>(veh, 0x485).wrapping_add(1));
    let drv = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0xC), 0x460);
    if !drv.is_null() && ped_is_player_m(drv) == 0 {
        let veh = rd::<GPtr<u8>>(this, 0xC);
        if ped.addr() as u32 == rd::<u32>(veh, 0x460) && rd::<u8>(this, 0x18) != 0 {
            wr::<u8>(veh, 0x3D0, 0);
            wr::<u8>(rd::<GPtr<u8>>(this, 0xC), 0x3BA, 0);
        }
    }
    if ped_is_player_m(ped) != 0 {
        let veh = rd::<GPtr<u8>>(this, 0xC);
        if ped.addr() as u32 == rd::<u32>(veh, 0x460) {
            wr::<u8>(veh, 0x36, (rd::<u8>(veh, 0x36) & 7) | 0x48);
        }
    }
}

/// `CDecision::MakeDecision(int type, bool b, int x1, int x2, int x3, int ev, int16* task,
/// int16* facial)` (0x6040D0, `ret 0x20`): collects the slots with a non-zero probability
/// (`byte[+0x18 + type + 4i]`), a task not -1 / x1 / x2 / x3 and the flag `byte[+0x30 + b + 2i]`
/// into a local `CDecisionSimple` (unused slots task 200, probability 0), `*task = 200`; none →
/// `*facial = -1`. Otherwise all six probabilities become cumulative sums (x87 `fiadd` from 0.0,
/// each stored as f32), scaled by `1 / total`, `CDecisionSimple::MakeDecision(ev, task, &tmp)`,
/// `*facial = -1`.
#[hook(addr = 0x6040D0, conv = thiscall, name = "CDecision::MakeDecision", expect = "83 EC 60 53 55")]
#[allow(clippy::too_many_arguments)]
pub fn decision_make_decision_6040d0(
    this: GPtr<u8>,
    kind: i32,
    b: u8,
    x1: i32,
    x2: i32,
    x3: i32,
    ev: i32,
    task: GPtr<u8>,
    facial: GPtr<u8>,
) {
    let mut tasks = [0xC8i32; 6];
    let mut probs = [0u8; 6];
    let mut n = 0usize;
    for i in 0..6isize {
        let p = rd::<u8>(this, 0x18 + kind as isize + 4 * i);
        if p == 0 {
            continue;
        }
        let t = rd::<i32>(this, 4 * i);
        if t == -1 || t == x1 || t == x2 || t == x3 || rd::<u8>(this, 0x30 + isize::from(b != 0) + 2 * i) == 0
        {
            continue;
        }
        tasks[n] = t;
        probs[n] = p;
        n += 1;
    }
    wr::<u16>(task, 0, 0xC8);
    if n == 0 {
        wr::<u16>(facial, 0, 0xFFFF);
        return;
    }
    // local CDecisionSimple: tasks[6] +0, probs[6] +0x18, count +0x30
    let mut obj = [0u32; 13];
    obj[12] = 6;
    let mut sum = x(0.0);
    for e in 0..6 {
        obj[e] = tasks[e] as u32;
        sum = sum.fadd(xi(i32::from(probs[e])));
        obj[6 + e] = sum.to_f32().to_bits();
    }
    let inv = sum.fdivr_m(1.0);
    for e in 0..6 {
        obj[6 + e] = inv.fmul_m(f32::from_bits(obj[6 + e])).to_f32().to_bits();
    }
    let mut idx = 0u32;
    let op: GPtr<u8> = GPtr::from_raw(&mut obj as *mut [u32; 13]).cast();
    let ip: GPtr<u8> = GPtr::from_raw(&mut idx as *mut u32).cast();
    decision_simple_make_decision(op, ev, task, ip);
    wr::<u16>(facial, 0, 0xFFFF);
}

/// `CInterestingEvents::Constructor` (0x6023A0): flags byte `+0x12C = (x & ~1) | 0xE`, `+0x154 = 0xFF`,
/// `+0x134 = 0`, `+0x138 = 30.0`, `+0x130 = CTimer::m_FrameCounter (0xB7CB4C) - 1`; the 8 events
/// (3 dwords each) cleared; for the 29 types: `+0xB8` dwords 0, `+0x7E` delays 2000, `+0x60`
/// priorities 5; then the per-type priority bytes and delay words of the bytes; returns `this`.
#[hook(addr = 0x6023A0, conv = thiscall, name = "CInterestingEvents::Constructor", expect = "8B C1 8A 88 2C 01 00 00")]
pub fn interesting_events_ctor_6023a0(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u8>(this, 0x12C, (rd::<u8>(this, 0x12C) & 0xFE) | 0xE);
    wr::<u8>(this, 0x154, 0xFF);
    wr::<u32>(this, 0x134, 0);
    wr::<u32>(this, 0x138, 0x41F0_0000);
    wr::<u32>(this, 0x130, g::<u32>(0xB7CB4C).read().wrapping_sub(1));
    for i in 0..8 {
        for k in 0..3 {
            wr::<u32>(this, 0xC * i + 4 * k, 0);
        }
    }
    for i in 0..0x1D {
        wr::<u32>(this, 0xB8 + 4 * i, 0);
        wr::<u16>(this, 0x7E + 2 * i, 0x7D0);
        wr::<u8>(this, 0x60 + i, 5);
    }
    let bytes: [(&[isize], u8); 9] = [
        (&[0x7C, 0x7B], 10),
        (&[0x7A, 0x79, 0x78, 0x77, 0x76, 0x70, 0x6F], 9),
        (&[0x63, 0x62, 0x61], 1),
        (&[0x74, 0x72], 7),
        (&[0x73, 0x6C, 0x6D, 0x6B], 6),
        (&[0x67, 0x66, 0x65, 0x64], 2),
        (&[0x75, 0x71], 8),
        (&[0x6E, 0x6A], 5),
        (&[0x69, 0x68], 4),
    ];
    for (offs, v) in bytes {
        for &o in offs {
            wr::<u8>(this, o, v);
        }
    }
    let words: [(&[isize], u16); 5] = [
        (&[0xB6, 0xB4], 0xFA0),
        (&[0xB2, 0xA8, 0x96], 0x1F40),
        (
            &[0xB0, 0xAE, 0xAC, 0xA6, 0xA2, 0xA0, 0x9E, 0x9C, 0x9A, 0x94, 0x92],
            0x1770,
        ),
        (&[0xAA, 0xA4, 0x98, 0x84, 0x82, 0x80], 0x1388),
        (&[0x90, 0x8E, 0x8C, 0x8A, 0x88, 0x86], 0xBB8),
    ];
    for (offs, v) in words {
        for &o in offs {
            wr::<u16>(this, o, v);
        }
    }
    this
}

orig! {
    /// `CPedAttractor::GetTailOfQueue` (0x5EB5B0)
    fn ped_attractor_get_tail_of_queue(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x5EB5B0 => unported;
    /// `CPedAttractor::GetHeadOfQueue` (0x5EB590)
    fn ped_attractor_get_head_of_queue(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x5EB590 => unported;
}

/// `CPedAttractorManager::HasQueueTailArrivedAtSlot(const C2dEffect*, const CEntity*)` (0x5EBBA0):
/// non-attractor effect → false; list by type as in `HasEmptySlot` (others → true); no attractor or
/// empty queue tail → true; tail == head → it has an active task 0xEB; otherwise its active task
/// 0xEA has `+0x10 == queue size - 1` (EAX = 1/0).
#[hook(addr = 0x5EBBA0, conv = thiscall, name = "CPedAttractorManager::HasQueueTailArrivedAtSlot", expect = "8B 44 24 04 85 C0")]
pub fn ped_attractor_manager_has_queue_tail_arrived_at_slot_5ebba0(
    this: GPtr<u8>,
    effect: GPtr<u8>,
    entity: u32,
) -> u8 {
    if effect.is_null() || rd::<u8>(effect, 0xC) != 3 {
        return 0;
    }
    let off: isize = match rd::<u8>(effect, 0x34) {
        0 => 0x10,
        1 => 0,
        t @ 2..=9 => 0x10 * t as isize,
        _ => return 1,
    };
    let a = find_associated_attractor(effect, entity, this.byte_add(off));
    if a.is_null() {
        return 1;
    }
    let begin = rd::<u32>(a, 0x20);
    let n = if begin == 0 {
        0
    } else {
        (rd::<u32>(a, 0x24).wrapping_sub(begin) as i32) >> 2
    };
    let tail = ped_attractor_get_tail_of_queue(a);
    if tail.is_null() {
        return 1;
    }
    let head = ped_attractor_get_head_of_queue(a);
    let tm = rd::<GPtr<u8>>(tail, 0x47C).byte_add(4);
    if tail.addr() == head.addr() {
        return u8::from(!task_manager_find_active_task_by_type(tm, 0xEB).is_null());
    }
    let t = task_manager_find_active_task_by_type(tm, 0xEA);
    u8::from(!t.is_null() && rd::<i32>(t, 0x10) == n.wrapping_sub(1))
}

pub const HOOKS: &[HookReg] = &[
    HOOK_INIT_G_969A28,
    HOOK_INIT_G_A9B098,
    HOOK_INIT_G_A9B0A0,
    HOOK_INIT_G_C0AFC8,
    HOOK_INIT_G_C0BBF0,
    HOOK_INIT_G_C0BBF4,
    HOOK_INIT_G_C177BC,
    HOOK_INIT_G_C18BF0,
    HOOK_INIT_G_C18BFC,
    HOOK_INIT_G_C09870,
    HOOK_INIT_COVER_LIST_OF_PROCESSED_BUILDINGS,
    HOOK_INIT_COL1,
    HOOK_INIT_MS_CLIMB_COL_MODEL,
    HOOK_INIT_MS_STAND_UP_COL_MODEL,
    HOOK_INIT_MS_VAULT_COL_MODEL,
    HOOK_INIT_MS_FIND_EDGE_COL_MODEL,
    HOOK_INIT_INFORM_FRIENDS_EVENTS,
    HOOK_INIT_INFORM_GROUP_EVENTS,
    HOOK_INIT_PED_GROUPS,
    HOOK_INIT_G_C1A43C,
    HOOK_INIT_G_C1A45C,
    HOOK_INIT_G_C1A4DC,
    HOOK_SET_FIELD_34_66B150,
    HOOK_DECISION_MAKER_TYPES_LOAD_EVENT_INDICES_600840,
    HOOK_PED_ATTRACTOR_IS_AT_HEAD_OF_QUEUE_5EB530,
    HOOK_FUN_0065EE90,
    HOOK_ACQUAINTANCE_CTOR_608AE0,
    HOOK_FUN_0065E710,
    HOOK_PLAYER_RELATIONSHIP_RECORDER_CLEAR_61A150,
    HOOK_GET_PLAYER_RELATIONSHIP_RECORDER_61A2E0,
    HOOK_FUN_004BC470,
    HOOK_PED_GROUP_PLACER_PLACE_GROUP_5FD810,
    HOOK_FUN_0065DE60,
    HOOK_PTR_LIST_IS_MEMBER_OF_LIST_5521D0,
    HOOK_FUN_005E9800,
    HOOK_FUN_0063BAB0,
    HOOK_PED_GROUPS_GET_GROUP_ID_5F7EE0,
    HOOK_PED_GROUPS_IS_IN_PLAYERS_GROUP_5F7F10,
    HOOK_FUN_005BD3B0,
    HOOK_PED_GROUP_MEMBERSHIP_GET_OBJECT_FOR_PED_TO_HOLD_5F6950,
    HOOK_FUN_00669FB0,
    HOOK_FUN_005EC220,
    HOOK_FUN_0046FEE0,
    HOOK_FUN_00665DC0,
    HOOK_EVENT_SCANNER_SCAN_FOR_EVENTS_NOW_6053D0,
    HOOK_FUN_00667310,
    HOOK_FUN_0066E050,
    HOOK_PED_STEP_ATTRACTOR_SDD_5EEC40,
    HOOK_GET_PED_ATTRACTOR_MANAGER_5EE190,
    HOOK_DECISION_MAKER_TYPES_GET_INSTANCE_4684F0,
    HOOK_PED_STEP_ATTRACTOR_CTOR_5EEBD0,
    HOOK_FUN_00646CF0,
    HOOK_FUN_005BD360,
    HOOK_FUN_006550D0,
    HOOK_FUN_0063BAE0,
    HOOK_FUN_004ACE80,
    HOOK_PED_GROUPS_INIT_5FB8C0,
    HOOK_PED_GROUPS_GET_PEDS_GROUP_5F7E80,
    HOOK_PED_GROUPS_ADD_GROUP_5FB800,
    HOOK_PED_ATTRACTOR_MANAGER_CTOR_5EDF10,
    HOOK_PED_ATTRACTOR_MANAGER_HAS_EMPTY_SLOT_5EBB00,
    HOOK_PED_ATTRACTOR_MANAGER_IS_PED_REGISTERED_WITH_EFFECT_5EB690,
    HOOK_FUN_0061AEB0,
    HOOK_PED_GROUPS_ARE_IN_SAME_GROUP_5F7F40,
    HOOK_ACTIVE_TASK_BYTE_59_6822B0,
    HOOK_EVENT_SCANNER_CTOR_605300,
    HOOK_VEHICLE_SCANNER_SCAN_FOR_VEHICLES_IN_RANGE_5FFE60,
    HOOK_FUN_00691630,
    HOOK_DECISION_SIMPLE_MAKE_DECISION_6007A0,
    HOOK_DECISION_MAKER_TYPES_MAKE_DECISION_GROUP_606F80,
    HOOK_DECISION_MAKER_TYPES_MAKE_DECISION_PED_606E70,
    HOOK_DECISION_MAKER_TYPES_ADD_DECISION_MAKER_607050,
    HOOK_DECISION_MAKER_TYPES_REMOVE_DECISION_MAKER_6043A0,
    HOOK_FUN_0063BA00,
    HOOK_DECISION_MAKE_DECISION_6040D0,
    HOOK_INTERESTING_EVENTS_CTOR_6023A0,
    HOOK_PED_ATTRACTOR_MANAGER_HAS_QUEUE_TAIL_ARRIVED_AT_SLOT_5EBBA0,
];
