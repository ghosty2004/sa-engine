//! `CRunningScript::ProcessCommands1600To1699` (0x493FE0): complete port (PORT-SCRIPT-2 partial, completed in
//! PORT-SCRIPT-3). Every jump-table case is in [`PORTED`]; the other opcodes of the range reach the
//! original default (-1, 0x495DE1). Cases follow the 1.0 US bytes and
//! `docs/re/specs/SCRIPT/commands/ProcessCommands1600To1699.md`.
//!
//! Divergence notes: label/name buffers start zeroed (32-byte FX names, 40-byte debug labels, 0674's 9-byte
//! plate); the camera look-at-entity forms (067A/067B/067D/067E) pass a zeroed look-at vector where the
//! original passes a stack vector it never writes; `setne` bools read back with stale upper bytes are
//! passed as 0/1.
//!
//! Every ported case returns 0 (shared `xor al,al` tails 0x495DA9 / 0x495DDD, epilogue 0x495DE3 -> 0x4059EC).
//! Degree-to-radian products the original stores as f32 temporaries in stack slots (`fmul [0x8595EC]; fstp`)
//! are computed with `X87` (PC=24, memory operand) and rounded to f32 the same way.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x0642, 0x0643, 0x0646, 0x0647, 0x0648, 0x064A, 0x064B, 0x064C, 0x064D, 0x064E, 0x064F, 0x0650, 0x0651,
    0x0652, 0x0653, 0x0654, 0x0655, 0x0656, 0x0657, 0x0659, 0x065A, 0x065B, 0x065C, 0x065D, 0x065E, 0x065F,
    0x0660, 0x0661, 0x0662, 0x0663, 0x0664, 0x0665, 0x0666, 0x0667, 0x0668, 0x0669, 0x066A, 0x066B, 0x066C,
    0x066D, 0x066E, 0x066F, 0x0670, 0x0671, 0x0672, 0x0673, 0x0674, 0x0675, 0x0676, 0x0677, 0x0678, 0x0679,
    0x067A, 0x067B, 0x067C, 0x067D, 0x067E, 0x067F, 0x0680, 0x0681, 0x0682, 0x0683, 0x0684, 0x0685, 0x0686,
    0x0687, 0x0688, 0x0689, 0x068A, 0x068B, 0x068C, 0x068D, 0x068E, 0x068F, 0x0697, 0x0698, 0x0699, 0x069A,
    0x069B, 0x069C, 0x069D, 0x069E, 0x069F, 0x06A0, 0x06A1, 0x06A2, 0x06A3,
];

/// `.rdata` 0x8595EC: 0.017453292f (degrees to radians)
const K_DEG2RAD: u32 = 0x3C8E_FA35;
/// `.rdata` 0x858C1C: -1.0f
const K_MINUS_1: u32 = 0xBF80_0000;
/// `CTheScripts::ScriptEffectSystemArray[0].m_pFxSystem` (8-byte entries; the case reads `[i*8 + 0xA44114]`).
const SCRIPT_EFFECT_FX: usize = 0xA44114;

/// `g_fx` (`Fx_c`) 0xA9AE00 and `g_fxMan` (`FxManager_c`) 0xA9AE80.
const G_FX: usize = 0xA9AE00;
const G_FX_MAN: usize = 0xA9AE80;

orig! {
    /// `Fx_c::CreateMatFromVec(RwMatrix* out, CVector* pos, CVector* dir)` (0x49E950)
    fn create_mat_from_vec(fx: GPtr<u8>, out: GPtr<u8>, pos: GPtr<u32>, dir: GPtr<u32>) = thiscall 0x49E950 => unported;
    /// `CEntity` virtual slot 7 (+0x1C, `CreateRwObject`)
    fn entity_create_rw_object(e: GPtr<u8>) = thiscall vslot 0x1C => unported;
    /// `CEntity::GetModellingMatrix()` (0x46A2D0)
    fn get_modelling_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x46A2D0 => unported;
    /// `FxManager_c::CreateFxSystem(const char*, RwMatrix*, RwMatrix* parent, bool)` (0x4A9BB0)
    fn create_fx_system_mat(m: GPtr<u8>, name: GPtr<u8>, mat: GPtr<u8>, parent: GPtr<u8>, local: u32) -> GPtr<u8> = thiscall 0x4A9BB0 => unported;
    /// `CTheScripts::AddScriptEffectSystem(FxSystem_c*)` (0x492F90)
    fn add_script_effect_system(fx: GPtr<u8>) -> i32 = cdecl 0x492F90 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), raw dwords, ST0 (read as f64: exact)
    fn find_ground_z_raw(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CTheScripts::GetActualScriptThingIndex(int32 ref, uint8 type)` (0x4839A0)
    fn get_actual_script_thing_index(r: i32, ty: u32) -> i32 = cdecl 0x4839A0 => unported;
    /// `FxSystem_c::Play()` (0x4AA2F0)
    fn fx_play(fx: GPtr<u8>) = thiscall 0x4AA2F0 => unported;
    /// `FxSystem_c::Kill()` (0x4AA3F0)
    fn fx_kill(fx: GPtr<u8>) = thiscall 0x4AA3F0 => unported;
    /// `CTheScripts::RemoveScriptEffectSystem(int32)` (0x492FD0)
    fn remove_script_effect_system(h: i32) = cdecl 0x492FD0 => unported;
    /// `CPhysical::AttachEntityToEntity(CPhysical*, CVector offset, CVector rotation)` (0x54D570; both vectors by value)
    fn attach_entity_to_entity(e: GPtr<u8>, target: GPtr<u8>, ox: u32, oy: u32, oz: u32, rx: f32, ry: f32, rz: f32) = thiscall 0x54D570 => unported;
    /// `CPhysical::DettachEntityFromEntity(float x, float y, float z, bool useCollision)` (0x5442F0)
    fn dettach_entity_from_entity(e: GPtr<u8>, x: f32, y: f32, z: u32, use_collision: u32) = thiscall 0x5442F0 => unported;
}

orig! {
    /// `CPedIntelligence::ClearTasks(bool primary, bool secondary)` (0x601420)
    fn clear_tasks(i: GPtr<u8>, a: u32, b: u32) = thiscall 0x601420 => unported;
    /// `CAutomobile::FixTyre(eWheels)` (0x6A3580)
    fn fix_tyre(v: GPtr<u8>, w: u32) = thiscall 0x6A3580 => unported;
    /// `CStats::GetStatValue(uint16)` (0x558E40), ST0 (a stored float: exact as f64)
    fn get_stat_value(stat: u32) -> f64 = cdecl 0x558E40 => unported;
    /// `FxSystem_c::Stop()` (0x4AA390)
    fn fx_stop(fx: GPtr<u8>) = thiscall 0x4AA390 => unported;
    /// `CDamageManager::GetCarNodeIndexFromDoor(eDoors)` (0x6C26F0)
    fn node_from_door(d: u32) -> u32 = cdecl 0x6C26F0 => unported;
    /// `CAutomobile::FixDoor(int32 node, eDoors)` (0x6A35A0)
    fn fix_door(v: GPtr<u8>, node: u32, d: u32) = thiscall 0x6A35A0 => unported;
    /// `CDamageManager::GetCarNodeIndexFromPanel(ePanels)` (0x6C26A0)
    fn node_from_panel(p: u32) -> u32 = cdecl 0x6C26A0 => unported;
    /// `CAutomobile::FixPanel(int32 node, ePanels)` (0x6A3670)
    fn fix_panel(v: GPtr<u8>, node: u32, p: u32) = thiscall 0x6A3670 => unported;
    /// Unnamed thiscall at 0x492CB0 (a `.HOODLUM` stub) on `CTheScripts::ScriptAttractors`... the 36-byte
    /// entry at 0xC3A200 + i * 36, taking the ped type (ADD_PEDTYPE_AS_ATTRACTOR_USER)
    fn attractor_492cb0(e: GPtr<u8>, ped_type: u32) = thiscall 0x492CB0 => unported;
    /// `CRunningScript::GetIndexOfGlobalVariable()` (0x464700); result unused here
    fn get_index_of_global_variable(s: Scr) -> u16 = thiscall 0x464700 => crate::vm::get_index_of_global_variable;
}

orig! {
    /// `CAutomobile::PopDoor(int32 node, eDoors, bool showVisualEffect)` (0x6ADEF0)
    fn pop_door(v: GPtr<u8>, node: u32, d: u32, fx: u32) = thiscall 0x6ADEF0 => unported;
    /// `CAutomobile::PopPanel(int32 node, ePanels, bool showVisualEffect)` (0x6ADF80)
    fn pop_panel(v: GPtr<u8>, node: u32, p: u32, fx: u32) = thiscall 0x6ADF80 => unported;
    /// `FxSystem_c::PlayAndKill()` (0x4AA3D0)
    fn fx_play_and_kill(fx: GPtr<u8>) = thiscall 0x4AA3D0 => unported;
    /// `CPedScriptedTaskRecord::GetStatus(CPed*, int32 opcode)` (0x608710)
    fn scripted_task_status(ped: GPtr<u8>, opcode: u32) -> u32 = cdecl 0x608710 => unported;
    /// `CVehicle::IsDoorMissing(eDoors)` (vtable slot 0x98), AL
    fn is_door_missing(v: GPtr<u8>, d: u32) -> u8 = thiscall vslot 0x98 => unported;
    /// `CVehicle::OpenDoor(CPed*, int32 node, eDoors, float ratio, bool playSound)` (vtable slot 0x6C);
    /// the float raw
    fn open_door(v: GPtr<u8>, ped: u32, node: u32, d: u32, ratio: u32, sound: u32) = thiscall vslot 0x6C => unported;
    /// `CPickups::GetActualPickupIndex(int32)` (0x4552A0)
    fn get_actual_pickup_index(h: i32) -> i32 = cdecl 0x4552A0 => unported;
    /// `CPickup::GetPosn()` (0x4549A0): the CVector through the hidden pointer, returned in EAX
    fn pickup_get_posn(p: GPtr<u8>, ret: GPtr<u32>) -> GPtr<u32> = thiscall 0x4549A0 => unported;
}

orig! {
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u32>) = thiscall 0x59C910 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskComplexEvasiveDiveAndGetUp::CTaskComplexEvasiveDiveAndGetUp(CVehicle*, int32 time, const CVector*,
    /// bool)` (0x6536B0)
    fn dive_and_get_up_ctor(t: GPtr<u8>, veh: u32, time: u32, dir: GPtr<u32>, b: u32) -> GPtr<u8> = thiscall 0x6536B0 => unported;
    /// `CTaskComplexShuffleSeats::CTaskComplexShuffleSeats(CVehicle*)` (0x63D130)
    fn shuffle_seats_ctor(t: GPtr<u8>, veh: GPtr<u8>) -> GPtr<u8> = thiscall 0x63D130 => unported;
    /// `CRunningScript::GivePedScriptedTask(int32 pedHandle, CTask*, int32 opcode)` (0x465C20)
    fn give_ped_scripted_task(s: Scr, ped: u32, task: GPtr<u8>, command: u32) = thiscall 0x465C20 => unported;
    /// `CScriptResourceManager::RemoveFromResourceManager(int32, uint32 type, CRunningScript*)` (0x470510), AL
    fn remove_from_resource_manager(m: GPtr<u8>, h: i32, ty: u32, s: Scr) -> u8 = thiscall 0x470510 => unported;
    /// `CDecisionMakerTypesFileLoader::UnloadDecisionMaker(int32)` (0x607A70)
    fn unload_decision_maker(i: i32) = cdecl 0x607A70 => unported;
    /// `CBaseModelInfo::GetModelType()` (vtable slot 0x10), AL
    fn model_type(mi: GPtr<u8>) -> u8 = thiscall vslot 0x10 => unported;
    /// `CVehicleModelInfo::SetCustomCarPlateText(const char*)` (0x4C8980)
    fn set_custom_car_plate_text(mi: GPtr<u8>, text: GPtr<u8>) = thiscall 0x4C8980 => unported;
}

/// `CTheScripts::ScriptResourceManager` 0xA485A8; `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8.
const SCRIPT_RESOURCE_MANAGER: usize = 0xA485A8;
const MODEL_INFO_PTRS: u32 = 0xA9B0C8;

orig! {
    /// `IKChainManager_c::IsLooking(CPed*)` (0x6181A0), AL
    fn ik_is_looking(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6181A0 => unported;
    /// `IKChainManager_c::AbortLookAt(CPed*, int32 blendOutTime)` (0x618280)
    fn ik_abort_look_at(m: GPtr<u8>, ped: GPtr<u8>, t: u32) = thiscall 0x618280 => unported;
    /// `CTaskSimpleClearLookAt::CTaskSimpleClearLookAt()` (0x634670)
    fn clear_look_at_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x634670 => unported;
    /// `CTaskComplexSequence::AddTask(CTask*)` (0x632D10)
    fn sequence_add_task(seq: GPtr<u8>, t: GPtr<u8>) = thiscall 0x632D10 => unported;
    /// `CTaskComplexDestroyCar::CTaskComplexDestroyCar(CVehicle*, uint32, uint32, uint32)` (0x621C00)
    fn destroy_car_ctor(t: GPtr<u8>, v: GPtr<u8>, a: u32, b: u32, c: u32) -> GPtr<u8> = thiscall 0x621C00 => unported;
    /// `CCamera::TakeControlAttached(CEntity* target, CEntity* attached, CVector* offset, CVector* lookAt,
    /// float tilt, int16 switchType, int32 whoIsInControl)` (0x50C910); the float raw
    fn take_control_attached(cam: GPtr<u8>, target: GPtr<u8>, attached: GPtr<u8>, offset: GPtr<u32>, look: GPtr<u32>, tilt: u32, sw: u32, who: u32) = thiscall 0x50C910 => unported;
    /// `CPhysical::AttachEntityToEntity(CPhysical*, RwV3d* offset, RtQuat* rot)` (0x54D690)
    fn attach_entity_to_entity_quat(e: GPtr<u8>, target: GPtr<u8>, off: u32, rot: u32) = thiscall 0x54D690 => unported;
    /// `CPedAttractorManager* GetPedAttractorManager()` (0x5EE190)
    fn get_ped_attractor_manager() -> GPtr<u8> = cdecl 0x5EE190 => unported;
    /// `CPedAttractorManager::GetPedUsingEffect(const C2dEffect*, const CEntity*)` (0x5EBE50)
    fn get_ped_using_effect(m: GPtr<u8>, fx: GPtr<u8>, e: u32) -> GPtr<u8> = thiscall 0x5EBE50 => unported;
    /// `CTaskSimpleGunControl::CTaskSimpleGunControl(CEntity*, const CVector*, const CVector*, int16, int16,
    /// int32)` (0x61F3F0)
    fn gun_control_ctor(t: GPtr<u8>, e: u32, target: GPtr<u32>, mv: u32, a: u32, b: u32, c: u32) -> GPtr<u8> = thiscall 0x61F3F0 => unported;
    /// `FxManager_c::CreateFxSystem(const char*, CVector* pos, RwMatrix* parent, bool)` (0x4A9BE0)
    fn create_fx_system_vec(m: GPtr<u8>, name: GPtr<u8>, pos: GPtr<u32>, parent: GPtr<u8>, local: u32) -> GPtr<u8> = thiscall 0x4A9BE0 => unported;
}

orig! {
    /// `IKChainManager_c::LookAt(const char* purpose, CPed*, CEntity* target, int32 time, int32 bone, CVector*
    /// pos, bool, float speed, int32 blendTime, int32 priority, bool)` (0x618970); the float raw
    fn ik_look_at(m: GPtr<u8>, purpose: GPtr<u8>, ped: GPtr<u8>, target: GPtr<u8>, time: u32, bone: u32, pos: u32, a: u32, speed: u32, blend: u32, prio: u32, b: u32) = thiscall 0x618970 => unported;
    /// Unnamed thiscall constructor at 0x634440 (`CTaskSimpleTriggerLookAt(CEntity*, int32 time, int32 bone,
    /// CVector offset, bool, float speed, int32 blendTime, int32 priority)`, the vector by value); floats raw
    fn trigger_look_at_ctor(t: GPtr<u8>, e: GPtr<u8>, time: u32, bone: u32, x: u32, y: u32, z: u32, a: u32, speed: u32, blend: u32, prio: u32) -> GPtr<u8> = thiscall 0x634440 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    fn get_weapon_info(ty: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CTaskSimpleThrowControl::CTaskSimpleThrowControl(CEntity*, const CVector*)` (0x61F8B0)
    fn throw_control_ctor(t: GPtr<u8>, e: u32, pos: GPtr<u32>) -> GPtr<u8> = thiscall 0x61F8B0 => unported;
}

orig! {
    /// `CTaskComplexPartnerChat::CTaskComplexPartnerChat(const char*, CPed*, bool lead, float, int32, bool, bool,
    /// CVector)` (0x684290); the float and the by-value vector raw
    fn partner_chat_ctor(t: GPtr<u8>, name: GPtr<u8>, partner: GPtr<u8>, lead: u32, f: u32, n: i32, a: u32, b: u32, x: u32, y: u32, z: u32) -> GPtr<u8> = thiscall 0x684290 => unported;
    /// `CTaskSimpleTogglePedThreatScanner::CTaskSimpleTogglePedThreatScanner(bool, bool, bool)` (0x492D10)
    fn threat_scanner_ctor(t: GPtr<u8>, a: u32, b: u32, c: u32) -> GPtr<u8> = thiscall 0x492D10 => unported;
    /// `CTaskSimpleTogglePedThreatScanner::ProcessPed(CPed*)` (0x6337A0)
    fn threat_scanner_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6337A0 => unported;
    /// `CTask::~CTask()` (0x61A3A0, unnamed in the catalogue; the base destructor the inlined derived one ends in)
    fn task_base_dtor(t: GPtr<u8>) = thiscall 0x61A3A0 => unported;
    /// `CTaskComplexLeaveAnyCar::CTaskComplexLeaveAnyCar(int32 delay, bool, bool)` (0x421150)
    fn leave_any_car_ctor(t: GPtr<u8>, delay: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x421150 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn random_int_in_range(lo: i32, hi: i32) -> i32 = cdecl 0x407180 => unported;
}

/// "COMMAND_TASK_CHAT_WITH_CHAR" (`.rdata` 0x85A3B8).
const S_TASK_CHAT_WITH_CHAR: usize = 0x85A3B8;
/// `CTask` vtable 0x85A334 (written back by the inlined destructor of the stack threat-scanner task).
const CTASK_VTABLE: u32 = 0x85A334;

/// "COMMAND_TASK_LOOK_AT_OBJECT" (`.rdata` 0x85A3D4), the IK look-at purpose string.
const S_TASK_LOOK_AT_OBJECT: usize = 0x85A3D4;

/// `g_ikChainMan` 0xC15448.
const IK_CHAIN_MAN: usize = 0xC15448;
/// `CTaskSequences::ms_taskSequence[0]` 0xC178F0 (0x40 bytes each).
const TASK_SEQUENCES: u32 = 0xC178F0;
/// The 2d-effect array of the script attractors (0x40 bytes each) 0xC3AB00.
const SCRIPT_ATTRACTOR_EFFECTS: u32 = 0xC3AB00;
/// `.rdata` 0x859E2C: 360.0f; 0x85A3B4: -999.9f
const K_360: u32 = 0x43B4_0000;
const K_MINUS_999_9: u32 = 0xC479_F99A;

/// ATTACH_CAMERA_TO_* (0679..067E): `TheCamera.TakeControlAttached(target, attached, &(P1, P2, P3), lookAt, tilt,
/// (u16)switch, 1)`. `look` is `&(P4, P5, P6)` for the forms without a look-at entity; with one, the original
/// passes a stack vector it never writes (divergence note: the port passes a zeroed vector).
fn attach_camera(attached: GPtr<u8>, target: GPtr<u8>, look: [u32; 3], tilt: u32, sw: u32) {
    let off = [sp(1), sp(2), sp(3)];
    take_control_attached(
        GPtr::from_addr(THE_CAMERA),
        target,
        attached,
        GPtr::from_raw(off.as_ptr().cast_mut()),
        GPtr::from_raw(look.as_ptr().cast_mut()),
        tilt,
        sw & 0xFFFF,
        1,
    );
}

/// CREATE_FX_SYSTEM_ON_CHAR / _CAR / _OBJECT (0669 0x4949F9 / 066B 0x494B70 / 066D 0x494BEC) and the
/// _WITH_DIRECTION forms of car and object (066C 0x494BAC / 066E 0x494CB8, the 066A body from 0x494B1C):
/// label (32); `CollectParameters(5 or 8)`; the discarded `GetActualScriptThingIndex(CollectNext..., 1)`;
/// `e = lookup(P0)` (no null check); pos = (P1, P2, z) (ground z at (P1, P2) when `P3 <= -100`, ordered, else
/// the `fld`ed P3 `fstp`ed). Without direction: `CreateRwObject` when needed, `parent = GetModellingMatrix()`,
/// `g_fxMan.CreateFxSystem(name, &pos, parent, P4 != 0)`. With direction: `g_fx.CreateMatFromVec(&mat, &pos,
/// &(P4, P5, P6))` first, then `CreateFxSystem(name, &mat, parent, P7 != 0)`. A null parent or system → P0 = -1;
/// else P0 = AddScriptEffectSystem(fx) and with mission cleanup AddEntityToList(P0, 4). StoreParameters(1).
/// Divergence note: the name buffer starts zeroed.
fn fx_on_entity(s: Scr, lookup: fn(i32) -> GPtr<u8>, dir: bool) {
    let mut name = [0u8; 36];
    let np = GPtr::from_raw(name.as_mut_ptr());
    read_text_label(s, np, 32);
    collect(s, if dir { 8 } else { 5 });
    get_actual_script_thing_index(collect_next_parameter_without_increasing_pc(s), 1);
    let e = lookup(spi(0));
    let (x, y) = (sp(1), sp(2));
    let st = X87::from_f32(spf(3));
    let z = if st.fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
        st
    } else {
        X87::from_f64_exact(find_ground_z_raw(x, y))
    }
    .to_f32()
    .to_bits();
    let mut pos = [x, y, z];
    let pp = GPtr::from_raw(pos.as_mut_ptr());
    let mut mat = [0u32; 0x10];
    let mp = GPtr::from_raw(mat.as_mut_ptr()).cast::<u8>();
    if dir {
        let mut d = [sp(4), sp(5), sp(6)];
        create_mat_from_vec(GPtr::from_addr(G_FX), mp, pp, GPtr::from_raw(d.as_mut_ptr()));
    }
    if e.byte_add(0x18).cast::<u32>().read() == 0 {
        entity_create_rw_object(e);
    }
    let parent = get_modelling_matrix(e);
    let fx = if parent.is_null() {
        GPtr::NULL
    } else if dir {
        create_fx_system_mat(GPtr::from_addr(G_FX_MAN), np, mp, parent, u32::from(sp(7) != 0))
    } else {
        create_fx_system_vec(GPtr::from_addr(G_FX_MAN), np, pp, parent, u32::from(sp(4) != 0))
    };
    if fx.is_null() {
        set_sp(0, u32::MAX);
    } else {
        let h = add_script_effect_system(fx);
        set_sp(0, h as u32);
        if uses_mission_cleanup(s) {
            mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), h, 4);
        }
    }
    store(s, 1);
}

fn ped_lookup(h: i32) -> GPtr<u8> {
    ped_at_ref(pool(PED_POOL), h)
}
fn vehicle_lookup(h: i32) -> GPtr<u8> {
    vehicle_at_ref(pool(VEHICLE_POOL), h)
}
fn object_lookup(h: i32) -> GPtr<u8> {
    object_at_ref(pool(OBJECT_POOL), h)
}

/// `CPickups::aPickUps[0]` (0x20 bytes each) 0x9788C0.
const PICKUPS: u32 = 0x9788C0;
/// `.rdata` 0x858B40: 50.0f
const K_50: u32 = 0x4248_0000;

/// `TheCamera.m_nActiveCam` (byte) 0xB6F081; `TheCamera.m_aCams[0].Front` 0xB6F32C and `.Source` 0xB6F338
/// (0x238 bytes per cam).
const ACTIVE_CAM: usize = 0xB6F081;
const CAM0_FRONT: u32 = 0xB6F32C;
const CAM0_SOURCE: u32 = 0xB6F338;
/// `CTaskSequences::ms_iActiveSequence` 0x8D2E98 and the `m_bRepeatSequence` dword of sequence 0 at
/// 0xC17920 (0x40 bytes per sequence).
const ACTIVE_SEQUENCE: usize = 0x8D2E98;
const SEQ_REPEAT: u32 = 0xC17920;
/// The 36-byte script attractor entries at 0xC3A200.
const SCRIPT_ATTRACTORS: u32 = 0xC3A200;

/// `cam = TheCamera.m_aCams[m_nActiveCam]` field address.
#[inline]
fn active_cam_field(base: u32) -> GPtr<u32> {
    let i = u32::from(GPtr::<u8>::from_addr(ACTIVE_CAM).read());
    GPtr::from_addr(base.wrapping_add(i * 0x238) as usize)
}

/// The debug commands' 40-byte label operand: `ReadTextLabelFromScript(buf, 40)` (the text is not used).
#[inline]
fn skip_label40(s: Scr) {
    let mut buf = [0u8; 40];
    read_text_label(s, GPtr::from_raw(buf.as_mut_ptr()), 40);
}

/// `fld SP[i]; fmul [0x8595EC]; fstp dword`
#[inline]
fn deg2rad(v: f32) -> f32 {
    X87::from_f32(v).fmul_m(f(K_DEG2RAD)).to_f32()
}

/// The script effect system slot of handle `h` (`GetActualScriptThingIndex(h, 1)`), or `None` for a negative
/// index or a null `FxSystem_c*` (both go to the `xor al,al` tail 0x495DDD).
#[inline]
fn fx_slot(h: i32) -> Option<GPtr<u8>> {
    let i = get_actual_script_thing_index(h, 1);
    if i < 0 {
        return None;
    }
    let fx = GPtr::<GPtr<u8>>::from_addr(SCRIPT_EFFECT_FX.wrapping_add((i as u32).wrapping_mul(8) as usize))
        .read();
    if fx.is_null() { None } else { Some(fx) }
}

/// `CollectParameters(8)`; `obj = OBJ(SP0)`, `target` from the given lookup on SP1 (called after the object
/// lookup); `obj->AttachEntityToEntity(target, CVector(SP2, SP3, SP4), CVector(SP5, SP6, SP7) * deg2rad)`. No null
/// checks (0681 0x495425, 069A 0x495BAF, 069B 0x495C6A: the same code with a different target pool).
fn attach_object(s: Scr, target: fn(u32) -> GPtr<u8>) {
    collect(s, 8);
    let obj = object_at_ref(pool(OBJECT_POOL), spi(0));
    let t = target(sp(1));
    let (rx, ry, rz) = (deg2rad(spf(5)), deg2rad(spf(6)), deg2rad(spf(7)));
    attach_entity_to_entity(obj, t, sp(2), sp(3), sp(4), rx, ry, rz);
}

/// `CRunningScript::ProcessCommands1600To1699(int32 command)` (0x493FE0, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x493FE0, conv = thiscall, name = "CRunningScript::ProcessCommands1600To1699", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript")]
pub fn process_commands_1600_to_1699(s: Scr, command: u32) -> i8 {
    match command {
        // 064C PLAY_FX_SYSTEM (0x494323)
        0x064C => {
            collect(s, 1);
            if let Some(fx) = fx_slot(spi(0)) {
                fx_play(fx);
            }
        }
        // 0650 KILL_FX_SYSTEM (0x494402): the handle (EDI) is SP0 as collected; tail 0x4943D7
        0x0650 => {
            collect(s, 1);
            let h = spi(0);
            if let Some(fx) = fx_slot(h) {
                fx_kill(fx);
                remove_script_effect_system(h);
                if uses_mission_cleanup(s) {
                    mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), h, 4);
                }
            }
        }
        // 0681 ATTACH_OBJECT_TO_CAR (0x495425)
        0x0681 => attach_object(s, |h| vehicle_at_ref(pool(VEHICLE_POOL), h as i32)),
        // 0682 DETACH_OBJECT (0x4954E2)
        0x0682 => detach_object(s),
        // 0685 IS_OBJECT_ATTACHED (0x4956D3): `obj && obj->m_pAttachedTo (+0xFC)`
        0x0685 => {
            collect(s, 1);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            let b = !o.is_null() && !o.byte_add(0xFC).cast::<GPtr<u8>>().read().is_null();
            set_flag(s, b);
        }
        // 069A ATTACH_OBJECT_TO_OBJECT (0x495BAF): the object pool pointer is loaded once (ESI) for both lookups
        0x069A => {
            collect(s, 8);
            let p = pool(OBJECT_POOL);
            let obj = object_at_ref(p, spi(0));
            let t = object_at_ref(p, spi(1));
            let (rx, ry, rz) = (deg2rad(spf(5)), deg2rad(spf(6)), deg2rad(spf(7)));
            attach_entity_to_entity(obj, t, sp(2), sp(3), sp(4), rx, ry, rz);
        }
        // 069B ATTACH_OBJECT_TO_CHAR (0x495C6A)
        0x069B => attach_object(s, |h| ped_at_ref(pool(PED_POOL), h as i32)),
        // 066A CREATE_FX_SYSTEM_ON_CHAR_WITH_DIRECTION (0x494A6E)
        0x066A => create_fx_system_on_char_with_direction(s),
        // 064A, 064D, 0651, 0659, 065A, 0666, 066F, 0670, 0671, 0675, 0678, 068F, 069C..06A1 (0x495DDD): the
        // jump table goes straight to the epilogue: 0 without reading the operands.
        0x064A | 0x064D | 0x0651 | 0x0659 | 0x065A | 0x0666 | 0x066F | 0x0670 => {}
        0x0671 | 0x0675 | 0x0678 | 0x068F | 0x069C..=0x06A1 => {}
        // 065D VIEW_INTEGER_VARIABLE / 0660 WATCH_FLOAT_VARIABLE (0x4947C9), 065E VIEW_FLOAT_VARIABLE (0x4947E5),
        // 065F WATCH_INTEGER_VARIABLE (0x494801): GetIndexOfGlobalVariable() (unused), then the 40-byte label.
        0x065D..=0x0660 => {
            get_index_of_global_variable(s);
            skip_label40(s);
        }
        // 0661 BREAKPOINT (0x4947EC) / 0662 WRITE_DEBUG (0x494808): the 40-byte label only.
        0x0661 | 0x0662 => skip_label40(s),
        // 0663 WRITE_DEBUG_WITH_INT (0x49481D) / 0664 WRITE_DEBUG_WITH_FLOAT (0x49483B): label, then
        // `CollectParameters(1)`.
        0x0663 | 0x0664 => {
            skip_label40(s);
            collect(s, 1);
        }
        // 0665 GET_CHAR_MODEL (0x494844): ped not null-checked; P0 = (i16)m_nModelIndex (+0x22); StoreParameters(1).
        0x0665 => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            set_sp(0, i32::from(ped.byte_add(0x22).cast::<u16>().read() as i16) as u32);
            store(s, 1);
        }
        // 06A3 GET_CAR_MASS (0x495DAD): vehicle not null-checked; P0 = m_fMass (+0x8C) raw; StoreParameters(1).
        // gr: unregistered.
        0x06A3 => {
            collect(s, 1);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            set_sp(0, v.byte_add(0x8C).cast::<u32>().read());
            store(s, 1);
        }
        // 068C IS_PLAYER_TARGETTING_ANYTHING (0x495A51): flag = Players[P0].m_pPed->+0x71C (target entity) != 0.
        0x068C => {
            collect(s, 1);
            let ped = player_info(sp(0)).cast::<GPtr<u8>>().read();
            set_flag(s, ped.byte_add(0x71C).cast::<u32>().read() != 0);
        }
        // 068D GET_ACTIVE_CAMERA_COORDINATES (0x495A7D): no operands read; P0..P2 = the active cam's Source (raw);
        // StoreParameters(3).
        0x068D => {
            let p = active_cam_field(CAM0_SOURCE);
            let v = [p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read()];
            set_sp(0, v[0]);
            set_sp(1, v[1]);
            set_sp(2, v[2]);
            store(s, 3);
        }
        // 068E GET_ACTIVE_CAMERA_POINT_AT (0x495AA1): P[i] = `fld Front[i]; fadd Source[i]` (PC=24); StoreParameters(3).
        0x068E => {
            let fr = active_cam_field(CAM0_FRONT);
            let so = active_cam_field(CAM0_SOURCE);
            for i in 0..3 {
                let v = X87::from_f32(f(fr.wrapping_index(i).read())).fadd_m(f(so.wrapping_index(i).read()));
                set_sp(i, v.to_f32().to_bits());
            }
            store(s, 3);
        }
        // 0648 SET_FOLLOW_NODE_THRESHOLD_DISTANCE (0x4941F5): ped not null-checked; `m_pIntelligence->+0xCC` = P1 raw.
        0x0648 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            ped.byte_add(0x47C).cast::<GPtr<u8>>().read().byte_add(0xCC).cast::<u32>().write(sp(1));
        }
        // 0687 CLEAR_CHAR_TASKS (0x49575F): `m_pIntelligence->ClearTasks(true, true)` (ped not null-checked).
        0x0687 => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            clear_tasks(ped.byte_add(0x47C).cast::<GPtr<u8>>().read(), 1, 1);
        }
        // 0699 FIX_CAR_TYRE (0x495B80): CAutomobile::FixTyre(P1) (no null check). gr: unregistered.
        0x0699 => {
            collect(s, 2);
            fix_tyre(vehicle_at_ref(pool(VEHICLE_POOL), spi(0)), sp(1));
        }
        // 0652 GET_INT_STAT (0x49443C): P0 = `_ftol2(GetStatValue((u16)P0))` (ST0 read as f64, low dword).
        // 0653 GET_FLOAT_STAT (0x494470): P0 = `fstp GetStatValue((u16)P0)`. StoreParameters(1).
        0x0652 | 0x0653 => {
            collect(s, 1);
            let v = X87::from_f64_exact(get_stat_value(sp(0) & 0xFFFF));
            set_sp(0, if command == 0x0652 { v.ftol() as u32 } else { v.to_f32().to_bits() });
            store(s, 1);
        }
        // 064E STOP_FX_SYSTEM (0x494361): FxSystem_c::Stop on the script effect slot (nothing for a negative
        // index or a null system). gr: unregistered.
        0x064E => {
            collect(s, 1);
            if let Some(fx) = fx_slot(spi(0)) {
                fx_stop(fx);
            }
        }
        // 067F FORCE_CAR_LIGHTS (0x49539D): no null check; byte +0x4A8 bits 3..4 = `(u8)P1 << 3`. gr: unregistered.
        0x067F => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let p = v.byte_add(0x4A8);
            let b = p.read();
            p.write(b ^ (((sp(1) as u8) << 3 ^ b) & 0x18));
        }
        // 0643 SET_SEQUENCE_TO_REPEAT (0x4940AD): `i = GetActualScriptThingIndex(P0, 4)`; nothing unless 0 <= i
        // < 64; then the dword at `0xC17920 + ms_iActiveSequence * 0x40` (not `i`, as the exe) = P1.
        0x0643 => {
            collect(s, 2);
            let i = get_actual_script_thing_index(spi(0), 4);
            if (0..0x40).contains(&i) {
                let a = GPtr::<u32>::from_addr(ACTIVE_SEQUENCE).read();
                GPtr::<u32>::from_addr(SEQ_REPEAT.wrapping_add(a << 6) as usize).write(sp(1));
            }
        }
        // 068A FIX_CAR_DOOR (0x4958D2) / 0698 FIX_CAR_PANEL (0x495B44): no null check;
        // `FixDoor(GetCarNodeIndexFromDoor(P1), P1)` / `FixPanel(GetCarNodeIndexFromPanel(P1), P1)`.
        // gr: unregistered.
        0x068A => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let n = node_from_door(sp(1));
            fix_door(v, n, sp(1));
        }
        0x0698 => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let n = node_from_panel(sp(1));
            fix_panel(v, n, sp(1));
        }
        // 0654 SET_OBJECT_RENDER_SCORCHED (0x4944A1): no null check; dword +0x40 bit 29 = P1 != 0. gr: unregistered.
        0x0654 => {
            collect(s, 2);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            let p = o.byte_add(0x40).cast::<u32>();
            let v = p.read();
            p.write(if sp(1) != 0 { v | 0x2000_0000 } else { v & 0xDFFF_FFFF });
        }
        // 0680 ADD_PEDTYPE_AS_ATTRACTOR_USER (0x4953DD): `i = GetActualScriptThingIndex(P0, 6)`; for 0 <= i < 64:
        // 0x492CB0(&attractors[i] (0xC3A200 + i * 36), P1). gr: unregistered.
        0x0680 => {
            collect(s, 2);
            let i = get_actual_script_thing_index(spi(0), 6);
            if (0..0x40).contains(&i) {
                attractor_492cb0(GPtr::from_addr(SCRIPT_ATTRACTORS.wrapping_add(i as u32 * 36) as usize), sp(1));
            }
        }
        // 0686 IS_VEHICLE_ATTACHED (0x495719): flag = vehicle != null && m_pAttachedTo (+0xFC) != 0.
        // gr: unregistered.
        0x0686 => {
            collect(s, 1);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            set_flag(s, !v.is_null() && v.byte_add(0xFC).cast::<u32>().read() != 0);
        }
        // 0689 POP_CAR_DOOR (0x495882) / 0697 POP_CAR_PANEL (0x495AF4): no null check; PopDoor /
        // PopPanel(GetCarNodeIndexFromDoor/Panel(P1), P1, P2 != 0) (the bool pushed with stale upper bytes:
        // 0/1 here). gr: unregistered.
        0x0689 => {
            collect(s, 3);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let b = u32::from(sp(2) != 0);
            let n = node_from_door(sp(1));
            pop_door(v, n, sp(1), b);
        }
        0x0697 => {
            collect(s, 3);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let b = u32::from(sp(2) != 0);
            let n = node_from_panel(sp(1));
            pop_panel(v, n, sp(1), b);
        }
        // 064F PLAY_AND_KILL_FX_SYSTEM (0x49439F): with a slot: FxSystem_c::PlayAndKill();
        // RemoveScriptEffectSystem(P0); with mission cleanup RemoveEntityFromList(P0, 4). gr: unregistered.
        0x064F => {
            collect(s, 1);
            let h = spi(0);
            if let Some(fx) = fx_slot(h) {
                fx_play_and_kill(fx);
                remove_script_effect_system(h);
                if uses_mission_cleanup(s) {
                    mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), h, 4);
                }
            }
        }
        // 0646 GET_SEQUENCE_PROGRESS (0x4940F3): ped not null-checked; P0 = -1 when
        // `CPedScriptedTaskRecord::GetStatus(ped, 0x618)` is 0, else `[[m_pIntelligence + 0x10] + 0x10]`
        // (no null check); StoreParameters(1).
        0x0646 => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let v = if scripted_task_status(ped, 0x618) == 0 {
                u32::MAX
            } else {
                let intel = ped.byte_add(0x47C).cast::<GPtr<u8>>().read();
                intel.byte_add(0x10).cast::<GPtr<u8>>().read().byte_add(0x10).cast::<u32>().read()
            };
            set_sp(0, v);
            store(s, 1);
        }
        // 0657 OPEN_CAR_DOOR (0x49467B): no null check; `node = GetCarNodeIndexFromDoor(P1)`; nothing when
        // IsDoorMissing(P1) (virtual) or the frame `+0x648 + node * 4` is null; else virtual
        // OpenDoor(nullptr, node, P1, 1.0f, true). gr: unregistered.
        0x0657 => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let d = sp(1);
            let node = node_from_door(d);
            if is_door_missing(v, d) == 0
                && v.byte_add(0x648 + node as isize * 4).cast::<u32>().read() != 0
            {
                open_door(v, 0, node, d, 0x3F80_0000, 1);
            }
        }
        // 065B GET_PICKUP_COORDINATES (0x4946E4): `i = GetActualPickupIndex(P0)`; P0..P2 = (0, 0, -100.0f) when
        // -1, else `aPickUps[i].GetPosn()`; StoreParameters(3). gr: unregistered.
        0x065B => {
            collect(s, 1);
            let i = get_actual_pickup_index(spi(0));
            let v = if i == -1 {
                [0, 0, 0xC2C8_0000]
            } else {
                let mut tmp = [0u32; 3];
                let r = pickup_get_posn(
                    GPtr::from_addr(PICKUPS.wrapping_add((i as u32) << 5) as usize),
                    GPtr::from_raw(tmp.as_mut_ptr()),
                );
                [r.read(), r.wrapping_index(1).read(), r.wrapping_index(2).read()]
            };
            set_sp(0, v[0]);
            set_sp(1, v[1]);
            set_sp(2, v[2]);
            store(s, 3);
        }
        // 06A2 GET_CAR_SPEED_VECTOR (0x495D27): no null check; P[i] = `m_vecMoveSpeed[i] (+0x44) * 50.0f`
        // (`fld; fmul m32; fstp`); StoreParameters(3). gr: unregistered.
        0x06A2 => {
            collect(s, 1);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0)).byte_add(0x44).cast::<u32>();
            let r = [v.read(), v.wrapping_index(1).read(), v.wrapping_index(2).read()];
            for (i, c) in r.iter().enumerate() {
                set_sp(i as i32, X87::from_f32(f(*c)).fmul_m(f(K_50)).to_f32().to_bits());
            }
            store(s, 3);
        }
        // 0673 TASK_DIVE_AND_GET_UP (0x494E34): `dir = (P1, P2, 0)`, `dir.Normalise()`; `new(0x24)` (null on
        // failure) `CTaskComplexEvasiveDiveAndGetUp(nullptr, P3, &dir, true)`; GivePedScriptedTask(P0, task,
        // command). gr: unregistered. Dead stack writes not ported: the EH state slot.
        0x0673 => {
            collect(s, 4);
            let dir = [sp(1), sp(2), 0];
            let dp = GPtr::from_raw(dir.as_ptr().cast_mut());
            vector_normalise(dp);
            let mem = task_new(0x24);
            let t = if mem.is_null() { GPtr::NULL } else { dive_and_get_up_ctor(mem, 0, sp(3), dp, 1) };
            give_ped_scripted_task(s, sp(0), t, command);
        }
        // 0676 TASK_SHUFFLE_TO_NEXT_CAR_SEAT (0x494F1D): vehicle P1 looked up first; `new(0x2C)` (null on
        // failure) `CTaskComplexShuffleSeats(veh)`; GivePedScriptedTask(P0, task, command). gr: unregistered.
        0x0676 => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(1));
            let mem = task_new(0x2C);
            let t = if mem.is_null() { GPtr::NULL } else { shuffle_seats_ctor(mem, v) };
            give_ped_scripted_task(s, sp(0), t, command);
        }
        // 065C REMOVE_DECISION_MAKER (0x494767): `i = GetActualScriptThingIndex(P0, 7)`; for 0 <= i < 20 and
        // `ScriptResourceManager.RemoveFromResourceManager(P0, 3, this)`: UnloadDecisionMaker(i); with mission
        // cleanup RemoveEntityFromList(P0, 9) (regardless). gr: unregistered.
        0x065C => {
            collect(s, 1);
            let h = spi(0);
            let i = get_actual_script_thing_index(h, 7);
            if (0..0x14).contains(&i)
                && remove_from_resource_manager(GPtr::from_addr(SCRIPT_RESOURCE_MANAGER), h, 3, s) != 0
            {
                unload_decision_maker(i);
            }
            if uses_mission_cleanup(s) {
                mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), h, 9);
            }
        }
        // 0674 CUSTOM_PLATE_FOR_NEXT_CAR (0x494EA4): `CollectParameters(1)`; label (9 bytes); the first 8 bytes
        // that are '_' or NUL become ' '; byte 8 = 0; with `mi = ms_modelInfoPtrs[P0]` not null, of type 6
        // (vehicle, virtual GetModelType) and `+0x24` non-zero: `mi->SetCustomCarPlateText(buf)`.
        // gr: unregistered. Divergence note: zeroed 16-byte buffer.
        0x0674 => {
            collect(s, 1);
            let mut buf = [0u8; 16];
            read_text_label(s, GPtr::from_raw(buf.as_mut_ptr()), 9);
            for c in buf.iter_mut().take(8) {
                if *c == b'_' || *c == 0 {
                    *c = b' ';
                }
            }
            buf[8] = 0;
            let mi = GPtr::<GPtr<u8>>::from_addr(MODEL_INFO_PTRS.wrapping_add(sp(0).wrapping_mul(4)) as usize).read();
            if !mi.is_null() && model_type(mi) == 6 && mi.byte_add(0x24).cast::<u32>().read() != 0 {
                set_custom_car_plate_text(mi, GPtr::from_raw(buf.as_mut_ptr()));
            }
        }
        // 0642 IS_CHAR_AT_SCRIPTED_ATTRACTOR (0x49401E): `i = GetActualScriptThingIndex(P1, 6)`; flag 0 unless
        // 0 <= i < 64; `u = GetPedAttractorManager()->GetPedUsingEffect(&effects[i] (0xC3AB00 + i * 64), nullptr)`;
        // P0 == -1: flag = u != null; else flag = PedPool.GetAtRef(P0) == u.
        0x0642 => {
            collect(s, 2);
            let h = sp(0);
            let i = get_actual_script_thing_index(spi(1), 6);
            let mut b = false;
            if (0..0x40).contains(&i) {
                let m = get_ped_attractor_manager();
                let u = get_ped_using_effect(m, GPtr::from_addr(SCRIPT_ATTRACTOR_EFFECTS.wrapping_add((i as u32) << 6) as usize), 0);
                b = if h == u32::MAX { !u.is_null() } else { ped_at_ref(pool(PED_POOL), h as i32).addr() == u.addr() };
            }
            set_flag(s, b);
        }
        // 0647 CLEAR_LOOK_AT (0x49415B): P0 == -1: `new(8)` (null on failure) CTaskSimpleClearLookAt added to the
        // active sequence (0xC178F0 + ms_iActiveSequence * 0x40, AddTask); else, when `g_ikChainMan.IsLooking(ped)`,
        // `AbortLookAt(ped, 500)` (ped not null-checked).
        0x0647 => {
            collect(s, 1);
            if spi(0) == -1 {
                let mem = task_new(8);
                let t = if mem.is_null() { GPtr::NULL } else { clear_look_at_ctor(mem) };
                let a = GPtr::<u32>::from_addr(ACTIVE_SEQUENCE).read();
                sequence_add_task(GPtr::from_addr(TASK_SEQUENCES.wrapping_add(a << 6) as usize), t);
            } else {
                let ped = ped_at_ref(pool(PED_POOL), spi(0));
                let ik = GPtr::from_addr(IK_CHAIN_MAN);
                if ik_is_looking(ik, ped) != 0 {
                    ik_abort_look_at(ik, ped, 500);
                }
            }
        }
        // 0656 LIMIT_ANGLE (0x4945D5): x87 (PC=24): `fld P0`; while less than 0 (ordered) `fadd 360.0`; while
        // greater than 360 (ordered) `fsub 360.0`; P0 = the value as stored (`fst`/`fstp`); StoreParameters(1).
        // gr: unregistered.
        0x0656 => {
            collect(s, 1);
            let mut v = X87::from_f32(spf(0));
            let mut out = v.to_f32();
            if v.fcom(X87::zero()) == sa_math::x87::Cmp::Less {
                loop {
                    v = v.fadd_m(f(K_360));
                    if v.fcom(X87::zero()) != sa_math::x87::Cmp::Less {
                        break;
                    }
                }
                out = v.to_f32();
            }
            if v.fcom(X87::constant(K_360)) == sa_math::x87::Cmp::Greater {
                loop {
                    v = v.fsub_m(f(K_360));
                    if v.fcom(X87::constant(K_360)) != sa_math::x87::Cmp::Greater {
                        break;
                    }
                }
                out = v.to_f32();
            }
            set_sp(0, out.to_bits());
            store(s, 1);
        }
        // 0667 TASK_AIM_GUN_AT_COORD (0x494868): `new(0x3C)` (null on failure) `CTaskSimpleGunControl(nullptr,
        // &(P1, P2, P3), nullptr, 0, 500, P4)`; GivePedScriptedTask(P0, task, command). gr: unregistered.
        0x0667 => {
            collect(s, 5);
            let v = [sp(1), sp(2), sp(3)];
            let mem = task_new(0x3C);
            let t = if mem.is_null() {
                GPtr::NULL
            } else {
                gun_control_ctor(mem, 0, GPtr::from_raw(v.as_ptr().cast_mut()), 0, 0, 0x1F4, sp(4))
            };
            give_ped_scripted_task(s, sp(0), t, command);
        }
        // 0669 / 066B / 066D / 066C / 066E CREATE_FX_SYSTEM_ON_* (see `fx_on_entity`). gr: unregistered.
        0x0669 => fx_on_entity(s, ped_lookup, false),
        0x066B => fx_on_entity(s, vehicle_lookup, false),
        0x066D => fx_on_entity(s, object_lookup, false),
        0x066C => fx_on_entity(s, vehicle_lookup, true),
        0x066E => fx_on_entity(s, object_lookup, true),
        // 0672 TASK_DESTROY_CAR (0x494DC6): vehicle P1 looked up first; `new(0x20)` (null on failure)
        // `CTaskComplexDestroyCar(veh, 0, 0, 0)`; GivePedScriptedTask(P0, task, command). gr: unregistered.
        0x0672 => {
            collect(s, 2);
            let h = sp(0);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(1));
            let mem = task_new(0x20);
            let t = if mem.is_null() { GPtr::NULL } else { destroy_car_ctor(mem, v, 0, 0, 0) };
            give_ped_scripted_task(s, h, t, command);
        }
        // 0679 ATTACH_CAMERA_TO_VEHICLE (0x495066) / 067C ATTACH_CAMERA_TO_CHAR (0x49520C): `CollectParameters(9)`;
        // TakeControlAttached(nullptr, e(P0), &(P1..P3), &(P4..P6), P7, (u16)P8, 1).
        0x0679 | 0x067C => {
            collect(s, 9);
            let e = if command == 0x0679 { vehicle_lookup(spi(0)) } else { ped_lookup(spi(0)) };
            attach_camera(e, GPtr::NULL, [sp(4), sp(5), sp(6)], sp(7), sp(8));
        }
        // 067A _VEHICLE_LOOK_AT_VEHICLE (0x495104), 067B _VEHICLE_LOOK_AT_CHAR (0x495187), 067D
        // _CHAR_LOOK_AT_VEHICLE (0x4952A9), 067E _CHAR_LOOK_AT_CHAR (0x49531B): `CollectParameters(7)`; attached =
        // lookup(P0) first, target = lookup(P4); TakeControlAttached(target, attached, &(P1..P3), &unwritten, P5,
        // (u16)P6, 1). gr: 067D unimplemented.
        0x067A | 0x067B | 0x067D | 0x067E => {
            collect(s, 7);
            let a = if command <= 0x067B { vehicle_lookup(spi(0)) } else { ped_lookup(spi(0)) };
            let t = if command == 0x067A || command == 0x067D { vehicle_lookup(spi(4)) } else { ped_lookup(spi(4)) };
            attach_camera(a, t, [0; 3], sp(5), sp(6));
        }
        // 0683 ATTACH_CAR_TO_CAR (0x49557B): `CollectParameters(8)`; e = VEH(P0), target = VEH(P1) (no null checks);
        // `fld P2; fst x`; rotation = (P5, P6, P7) * deg2rad (each `fstp`ed); when `P2 <= -999.9f` (ordered):
        // `e->AttachEntityToEntity(target, nullptr, nullptr)` (quaternion form 0x54D690); else
        // `e->AttachEntityToEntity(target, (x, P3, P4), rotation)`. gr: unregistered.
        0x0683 => {
            collect(s, 8);
            let e = vehicle_lookup(spi(0));
            let t = vehicle_lookup(spi(1));
            let st = X87::from_f32(spf(2));
            let x = st.to_f32().to_bits();
            let (rx, ry, rz) = (deg2rad(spf(5)), deg2rad(spf(6)), deg2rad(spf(7)));
            if !st.fcom(X87::constant(K_MINUS_999_9)).test_parity(0x41) {
                attach_entity_to_entity_quat(e, t, 0, 0);
            } else {
                attach_entity_to_entity(e, t, x, sp(3), sp(4), rx, ry, rz);
            }
        }
        // 0684 DETACH_CAR (0x495657): as 0682 DETACH_OBJECT (it jumps into that body at 0x495554/0x495559) on
        // VEH(P0).
        0x0684 => {
            collect(s, 5);
            let e = vehicle_lookup(spi(0));
            let ny = X87::from_f32(spf(2)).fmul_m(f(K_MINUS_1)).to_f32();
            if !e.is_null() && !e.byte_add(0xFC).cast::<GPtr<u8>>().read().is_null() {
                let z = sp(3);
                let uc = u32::from(sp(4) != 0);
                let y = deg2rad(ny);
                let x = deg2rad(spf(1));
                dettach_entity_from_entity(e, x, y, z, uc);
            }
        }
        // 064B CREATE_FX_SYSTEM (0x494228): label (32); `CollectParameters(4)`; the discarded
        // `GetActualScriptThingIndex(CollectNext..., 1)`; pos = (P0, P1, z) (ground z when `P2 <= -100`, ordered,
        // else the `fld`ed P2 `fstp`ed); `fx = g_fxMan.CreateFxSystem(name, &pos, nullptr, P3 != 0)`; null → P0 = -1;
        // else P0 = AddScriptEffectSystem(fx) and with mission cleanup AddEntityToList(P0, 4); StoreParameters(1).
        // gr: unregistered. Divergence note: zeroed name buffer.
        0x064B => {
            let mut name = [0u8; 36];
            let np = GPtr::from_raw(name.as_mut_ptr());
            read_text_label(s, np, 32);
            collect(s, 4);
            get_actual_script_thing_index(collect_next_parameter_without_increasing_pc(s), 1);
            let (x, y) = (sp(0), sp(1));
            let st = X87::from_f32(spf(2));
            let z = if st.fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
                st
            } else {
                X87::from_f64_exact(find_ground_z_raw(x, y))
            }
            .to_f32()
            .to_bits();
            let mut pos = [x, y, z];
            let fx = create_fx_system_vec(
                GPtr::from_addr(G_FX_MAN),
                np,
                GPtr::from_raw(pos.as_mut_ptr()),
                GPtr::NULL,
                u32::from(sp(3) != 0),
            );
            if fx.is_null() {
                set_sp(0, u32::MAX);
            } else {
                let h = add_script_effect_system(fx);
                set_sp(0, h as u32);
                if uses_mission_cleanup(s) {
                    mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), h, 4);
                }
            }
            store(s, 1);
        }
        // 0655 TASK_LOOK_AT_OBJECT (0x4944E9): time = P2 (-1 → 20000, -2 → 0x7FFFFFFF); `obj = OBJ(P1)`; P0 != -1:
        // `g_ikChainMan.LookAt("COMMAND_TASK_LOOK_AT_OBJECT", PED(P0), obj, time, -1, nullptr, false, 0.25f, 500,
        // 6, true)`; P0 == -1: `new(0x30)` (null on failure) 0x634440(obj, time, -1, (0, 0, 0), true, 0.25f, 1000, 3)
        // added to the active sequence. gr: unregistered.
        0x0655 => {
            collect(s, 3);
            let h = sp(0);
            let t2 = sp(2);
            let time = if t2 == u32::MAX {
                20000
            } else if t2 == u32::MAX - 1 {
                0x7FFF_FFFF
            } else {
                t2
            };
            let obj = object_lookup(spi(1));
            if h != u32::MAX {
                let ped = ped_lookup(h as i32);
                ik_look_at(
                    GPtr::from_addr(IK_CHAIN_MAN),
                    GPtr::from_addr(S_TASK_LOOK_AT_OBJECT),
                    ped,
                    obj,
                    time,
                    u32::MAX,
                    0,
                    0,
                    0x3E80_0000,
                    0x1F4,
                    6,
                    1,
                );
            } else {
                let mem = task_new(0x30);
                let t = if mem.is_null() {
                    GPtr::NULL
                } else {
                    trigger_look_at_ctor(mem, obj, time, u32::MAX, 0, 0, 0, 1, 0x3E80_0000, 0x3E8, 3)
                };
                let a = GPtr::<u32>::from_addr(ACTIVE_SEQUENCE).read();
                sequence_add_task(GPtr::from_addr(TASK_SEQUENCES.wrapping_add(a << 6) as usize), t);
            }
        }
        // 0668 TASK_SHOOT_AT_COORD (0x4948EF): target = (P1, P2, P3). P0 != -1: when the ped's current weapon
        // (`+0x5A0 + (i8)+0x718 * 0x1C`) has `GetWeaponInfo(type, 1)->m_nFlags (+0x18)` bit 8 (throwable),
        // `new(0x1C)` CTaskSimpleThrowControl(nullptr, &target), given when not null. Otherwise (and when that
        // task is null): `new(0x3C)` (null on failure) `CTaskSimpleGunControl(nullptr, &target, nullptr, 3, 5,
        // P4)`. GivePedScriptedTask(P0, task, command). gr: unregistered.
        0x0668 => {
            collect(s, 5);
            let h = sp(0);
            let v = [sp(1), sp(2), sp(3)];
            let vp = GPtr::from_raw(v.as_ptr().cast_mut());
            if h != u32::MAX {
                let ped = ped_lookup(h as i32);
                let slot = i32::from(ped.byte_add(0x718).read() as i8);
                let ty = ped.byte_add(0x5A0 + slot as isize * 0x1C).cast::<u32>().read();
                let info = get_weapon_info(ty, 1);
                if (info.byte_add(0x18).cast::<u32>().read() >> 8) & 1 != 0 {
                    let mem = task_new(0x1C);
                    let t = if mem.is_null() { GPtr::NULL } else { throw_control_ctor(mem, 0, vp) };
                    if !t.is_null() {
                        give_ped_scripted_task(s, h, t, command);
                        return 0;
                    }
                }
            }
            let mem = task_new(0x3C);
            let t = if mem.is_null() { GPtr::NULL } else { gun_control_ctor(mem, 0, vp, 0, 3, 5, sp(4)) };
            give_ped_scripted_task(s, h, t, command);
        }
        // 0677 TASK_CHAT_WITH_CHAR (0x494F85): `partner = PED(P1)`; `new(0x78)` (null on failure)
        // `CTaskComplexPartnerChat("COMMAND_TASK_CHAT_WITH_CHAR", partner, P2 != 0, 0.5f, P3 != 0 ? -1 : 4, true,
        // true, (0, 0, 0))`; GivePedScriptedTask(P0, task, command). gr: unregistered. Divergence note: the
        // `setne` bool read back from a stack dword (stale upper bytes) is passed as 0/1.
        0x0677 => {
            collect(s, 4);
            let h = sp(0);
            let lead = u32::from(sp(2) != 0);
            let n = if sp(3) != 0 { -1 } else { 4 };
            let partner = ped_lookup(spi(1));
            let mem = task_new(0x78);
            let t = if mem.is_null() {
                GPtr::NULL
            } else {
                partner_chat_ctor(mem, GPtr::from_addr(S_TASK_CHAT_WITH_CHAR), partner, lead, 0x3F00_0000, n, 1, 1, 0, 0, 0)
            };
            give_ped_scripted_task(s, h, t, command);
        }
        // 0688 TASK_TOGGLE_PED_THREAT_SCANNER (0x49578F): P0 != -1: a stack (0xC bytes)
        // `CTaskSimpleTogglePedThreatScanner(P1 != 0, P2 != 0, P3 != 0)` run with `ProcessPed(PED(P0))`, then its
        // inlined destructor (vtable = CTask's 0x85A334, `CTask::~CTask` 0x61A3A0); P0 == -1: `new(0xC)` (null on
        // failure) of the same task added to the active sequence. gr: unregistered. Divergence note: the first
        // bool of the stack form has stale upper bytes in the original (passed as 0/1).
        0x0688 => {
            collect(s, 4);
            let (a, b, c) = (u32::from(sp(1) != 0), u32::from(sp(2) != 0), u32::from(sp(3) != 0));
            if sp(0) != u32::MAX {
                let ped = ped_lookup(spi(0));
                let mut buf = [0u32; 4];
                let t = GPtr::from_raw(buf.as_mut_ptr()).cast::<u8>();
                threat_scanner_ctor(t, a, b, c);
                threat_scanner_process_ped(t, ped);
                t.cast::<u32>().write(CTASK_VTABLE);
                task_base_dtor(t);
            } else {
                let mem = task_new(0xC);
                let t = if mem.is_null() { GPtr::NULL } else { threat_scanner_ctor(mem, a, b, c) };
                let seq = GPtr::<u32>::from_addr(ACTIVE_SEQUENCE).read();
                sequence_add_task(GPtr::from_addr(TASK_SEQUENCES.wrapping_add(seq << 6) as usize), t);
            }
        }
        // 068B TASK_EVERYONE_LEAVE_CAR (0x49590E): vehicle not null-checked. With a driver (+0x460): `new(0x14)`
        // (null on failure) `CTaskComplexLeaveAnyCar(0, true, false)` given to `PedPool.GetRef(driver)`. Then for
        // i < (u8)+0x488 (re-read each pass): passenger i (+0x464 + i * 4) when set gets `CTaskComplexLeaveAnyCar(
        // GetRandomNumberInRange(-250, 250) + i * 500 + 500, true, false)` (the random call before `new`).
        // gr: unregistered.
        0x068B => {
            collect(s, 1);
            let v = vehicle_lookup(spi(0));
            let driver = v.byte_add(0x460).cast::<GPtr<u8>>().read();
            if !driver.is_null() {
                let mem = task_new(0x14);
                let t = if mem.is_null() { GPtr::NULL } else { leave_any_car_ctor(mem, 0, 1, 0) };
                let d = v.byte_add(0x460).cast::<GPtr<u8>>().read();
                let r = ped_get_ref(pool(PED_POOL), d);
                give_ped_scripted_task(s, r as u32, t, command);
            }
            let mut i: i32 = 0;
            while i < i32::from(v.byte_add(0x488).read()) {
                let p = v.byte_add(0x464 + i as isize * 4).cast::<GPtr<u8>>();
                if !p.read().is_null() {
                    let delay = random_int_in_range(-250, 250).wrapping_add(i * 500).wrapping_add(500);
                    let mem = task_new(0x14);
                    let t = if mem.is_null() { GPtr::NULL } else { leave_any_car_ctor(mem, delay as u32, 1, 0) };
                    let r = ped_get_ref(pool(PED_POOL), p.read());
                    give_ped_scripted_task(s, r as u32, t, command);
                }
                i += 1;
            }
        }
        _ => return -1,
    }
    0
}

/// 0x0682 DETACH_OBJECT (0x4954E2): `CollectParameters(5)`; `e = OBJ(SP0)`; `ny = SP2 * -1.0f` (x87, stored f32);
/// if `e && e->m_pAttachedTo (+0xFC)`: `e->DettachEntityFromEntity(SP1 * deg2rad, ny * deg2rad, SP3 (raw), SP4 != 0)`.
fn detach_object(s: Scr) {
    collect(s, 5);
    let e = object_at_ref(pool(OBJECT_POOL), spi(0));
    let ny = X87::from_f32(spf(2)).fmul_m(f(K_MINUS_1)).to_f32();
    if e.is_null() || e.byte_add(0xFC).cast::<GPtr<u8>>().read().is_null() {
        return;
    }
    let z = sp(3);
    let use_collision = u32::from(sp(4) != 0);
    let y = deg2rad(ny);
    let x = deg2rad(spf(1));
    dettach_entity_from_entity(e, x, y, z, use_collision);
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_1600_TO_1699];

/// 066A CREATE_FX_SYSTEM_ON_CHAR_WITH_DIRECTION (0x494A6E): `ReadTextLabelFromScript(name, 32)` (zeroed buffer;
/// the original's is not cleared), `CollectParameters(8)` (ped, offset P1..P3, direction P4..P6, flag P7), the
/// discarded `GetActualScriptThingIndex(CollectNextParameterWithoutIncreasingPC(), 1)`; ped = GetAtRef(P0)
/// (no null check). `fld P3; fcom -100.0`: z = ground z at (P1, P2) when `P3 <= -100` (ordered), else P3
/// (the `fld`ed value, `fstp`); `g_fx.CreateMatFromVec(&mat, &(P1, P2, z), &(P4, P5, P6))` into a stack
/// RwMatrix; `CreateRwObject` (vslot 7) when `m_pRwObject` (+0x18) is null; parent =
/// `GetModellingMatrix()`; null parent or a null `g_fxMan.CreateFxSystem(name, &mat, parent, P7 != 0)` →
/// P0 = -1; else P0 = `AddScriptEffectSystem(fx)` and, with mission cleanup, `AddEntityToList(P0, 4)`.
/// StoreParameters(1) either way.
fn create_fx_system_on_char_with_direction(s: Scr) {
    let mut name = [0u8; 36];
    let np = GPtr::from_raw(name.as_mut_ptr());
    read_text_label(s, np, 32);
    collect(s, 8);
    get_actual_script_thing_index(collect_next_parameter_without_increasing_pc(s), 1);
    let ped = ped_at_ref(pool(PED_POOL), spi(0));
    let (x, y) = (sp(1), sp(2));
    let st = X87::from_f32(spf(3));
    let z = if st.fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
        st
    } else {
        X87::from_f64_exact(find_ground_z_raw(x, y))
    }
    .to_f32()
    .to_bits();
    let mut off = [x, y, z];
    let mut dir = [sp(4), sp(5), sp(6)];
    let mut mat = [0u32; 0x10];
    let mp = GPtr::from_raw(mat.as_mut_ptr()).cast::<u8>();
    create_mat_from_vec(
        GPtr::from_addr(G_FX),
        mp,
        GPtr::from_raw(off.as_mut_ptr()),
        GPtr::from_raw(dir.as_mut_ptr()),
    );
    if ped.byte_add(0x18).cast::<u32>().read() == 0 {
        entity_create_rw_object(ped);
    }
    let parent = get_modelling_matrix(ped);
    let fx = if parent.is_null() {
        GPtr::NULL
    } else {
        create_fx_system_mat(GPtr::from_addr(G_FX_MAN), np, mp, parent, u32::from(sp(7) != 0))
    };
    if fx.is_null() {
        set_sp(0, u32::MAX);
    } else {
        let h = add_script_effect_system(fx);
        set_sp(0, h as u32);
        if uses_mission_cleanup(s) {
            mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), h, 4);
        }
    }
    store(s, 1);
}
