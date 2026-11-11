//! `CRunningScript::ProcessCommands1900To1999` (0x46B460): complete port (PORT-SCRIPT-2 partial, completed in
//! PORT-SCRIPT-4). Every opcode with a jump-table case is in [`PORTED`]; 07B9 reaches the default (`or al,0xFF`
//! at 0x46CBCF: -1), as does every command outside the range.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands1900To1999.md`.
//!
//! The prologue zeroes a function-scope `CQuaternion` at [esp+0x28] (0x46B491..0x46B4A9); 07C3/07C5 build
//! their quaternion on a zeroed local, the same value. Divergence notes (ledger classes of PORT-SCRIPT-2/3):
//! text labels are read into zeroed buffers (8-byte labels: 16 bytes, 18-byte labels: 19 bytes); 07B0's
//! `strncpy` buffer starts zeroed (16 bytes; an 8-character shop name leaves the original's unterminated);
//! the `RwMatrix`/`CMatrix` temporaries of 07C4/07C6 are zeroed Rust buffers; bools the original pushes as a
//! dword whose low byte only was written are passed as 0/1. The SEH state stores are not ported.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x076C, 0x076D, 0x076E, 0x076F, 0x0770, 0x0771, 0x0772, 0x0773, 0x0774, 0x0775, 0x0776, 0x0777, 0x0778,
    0x0779, 0x077A, 0x077B, 0x077C, 0x077D, 0x077E, 0x077F, 0x0780, 0x0781, 0x0782, 0x0783, 0x0784, 0x0785,
    0x0786, 0x0787, 0x0788, 0x0789, 0x078A, 0x078B, 0x078C, 0x078D, 0x078E, 0x078F, 0x0790, 0x0791, 0x0792,
    0x0793, 0x0794, 0x0795, 0x0796, 0x0797, 0x0798, 0x0799, 0x079A, 0x079B, 0x079C, 0x079D, 0x079E, 0x079F,
    0x07A0, 0x07A1, 0x07A2, 0x07A3, 0x07A4, 0x07A5, 0x07A6, 0x07A7, 0x07A8, 0x07A9, 0x07AA, 0x07AB, 0x07AC,
    0x07AD, 0x07AE, 0x07AF, 0x07B0, 0x07B1, 0x07B2, 0x07B3, 0x07B4, 0x07B5, 0x07B6, 0x07B7, 0x07B8, 0x07BA,
    0x07BB, 0x07BC, 0x07BD, 0x07BE, 0x07BF, 0x07C0, 0x07C1, 0x07C2, 0x07C3, 0x07C4, 0x07C5, 0x07C6, 0x07C7,
    0x07C8, 0x07C9, 0x07CA, 0x07CB, 0x07CC, 0x07CD, 0x07CE, 0x07CF,
];

/// `CGame::currArea` 0xB72914 (owned by the game module; read by address as the original does).
const CURR_AREA: usize = 0xB72914;
/// `.rdata` 0x859878: 57.295776f; 0x859E2C: 360.0f; 0x858B50: 0.0f; 0x8595EC: 0.017453292f
const K_RAD2DEG: u32 = 0x4265_2EE0;
const K_360: u32 = 0x43B4_0000;
const K_ZERO: u32 = 0;
const K_DEG2RAD: u32 = 0x3C8E_FA35;

orig! {
    /// A virtual `CVehicle` method without arguments (thiscall through the vtable slot read at run time)
    fn vcall0(f: u32, this: GPtr<u8>) = thiscall dyn => unported;
    /// `CTrain::RemoveOneMissionTrain(CTrain*)` (0x6F5DC0)
    fn remove_one_mission_train(t: GPtr<u8>) = cdecl 0x6F5DC0 => unported;
    /// `CTrain::SetNewTrainPosition(CTrain*, CVector)` (0x6F7140; the vector by value)
    fn set_new_train_position(t: GPtr<u8>, x: u32, y: u32, z: u32) = cdecl 0x6F7140 => unported;
    /// `CTrain::ReleaseOneMissionTrain(CTrain*)` (0x6F5DF0)
    fn release_one_mission_train(t: GPtr<u8>) = cdecl 0x6F5DF0 => unported;
    /// `CRunningScript::ReadTextLabelFromScript(char*, uint8)` (0x463D50)
    fn read_label(s: Scr, buf: GPtr<u8>, len: u32) = thiscall 0x463D50 => crate::vm::read_text_label_from_script;
    /// `CTheZones::FindZoneByLabel(const char*, eZoneType)` (0x572C40), AX
    fn find_zone_by_label(label: GPtr<u8>, ty: u32) -> u16 = cdecl 0x572C40 => unported;
    /// `CBaseModelInfo` virtual slot 0x10 (`GetModelType()`), AL
    fn model_info_type(mi: GPtr<u8>) -> u8 = thiscall vslot 0x10 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CRunningScript::GivePedScriptedTask(int32 pedHandle, CTask*, int32 opcode)` (0x465C20)
    fn give_ped_scripted_task(s: Scr, ped: u32, task: GPtr<u8>, command: u32) = thiscall 0x465C20 => unported;
    /// `CTaskComplexSeekEntity<CEntitySeekPosCalculatorStandard>::CTaskComplexSeekEntity(CEntity*, int32 time,
    /// int32 scanInterval, float, float, float, bool, bool)` (0x46AC10); floats raw
    #[allow(clippy::too_many_arguments)]
    fn task_seek_entity_ctor(t: GPtr<u8>, e: GPtr<u8>, time: u32, scan: u32, a: u32, b: u32, c: u32, d: u32, e2: u32) -> GPtr<u8> = thiscall 0x46AC10 => unported;
    /// `CIplStore::FindIplSlot(const char*)` (0x404AC0)
    fn find_ipl_slot(name: GPtr<u8>) -> u32 = cdecl 0x404AC0 => unported;
    /// `CIplStore::RequestIplAndIgnore` (0x405850) / `RemoveIplAndIgnore` (0x405890) / `RemoveIplWhenFarAway`
    /// (0x4058D0), `(int32 slot)`
    fn request_ipl_and_ignore(slot: u32) = cdecl 0x405850 => unported;
    fn remove_ipl_and_ignore(slot: u32) = cdecl 0x405890 => unported;
    fn remove_ipl_when_far_away(slot: u32) = cdecl 0x4058D0 => unported;
    /// `CPedType::GetPedFlag(ePedType)` (0x608830)
    fn get_ped_flag(t: u32) -> u32 = cdecl 0x608830 => unported;
    /// `CAcquaintance::SetAsAcquaintance(int32, uint32)` (0x608DA0) / `ClearAsAcquaintance` (0x608980)
    fn set_as_acquaintance(a: GPtr<u8>, ty: u32, fl: u32) = thiscall 0x608DA0 => unported;
    fn clear_as_acquaintance(a: GPtr<u8>, ty: u32, fl: u32) = thiscall 0x608980 => unported;
    /// `CAcquaintance::ClearAcquaintances(int32)` (0x6089A0)
    fn clear_acquaintances(a: GPtr<u8>, ty: u32) = thiscall 0x6089A0 => unported;
    /// `CAutomobile::GetCarPitch()` (0x6A6050), ST0 (an FPATAN-based result read as f64, the MATH-2 model)
    fn get_car_pitch(v: GPtr<u8>) -> f64 = thiscall 0x6A6050 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    fn get_weapon_info(ty: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CShopping::GetExtraInfo(uint32, int32)` (0x49ADE0)
    fn get_extra_info(a: u32, b: u32) -> u32 = cdecl 0x49ADE0 => unported;
    /// `CPedClothesDesc::SetTextureAndModel(uint32 tex, uint32 model, int32 type)` (0x5A8050)
    fn set_texture_and_model(d: GPtr<u8>, tex: u32, model: u32, ty: u32) = thiscall 0x5A8050 => unported;
    /// `CFireManager::GetNumFiresInArea(float x6)` (0x539860), ECX = gFireManager 0xB71F80; floats raw
    fn get_num_fires_in_area(m: GPtr<u8>, a: u32, b: u32, c: u32, d: u32, e: u32, f: u32) -> u32 = thiscall 0x539860 => unported;
    /// `CVehicle::InitWinch(int32)` (0x6D3B60)
    fn init_winch(v: GPtr<u8>, n: u32) = thiscall 0x6D3B60 => unported;
    /// `CVehicle::ReleasePickedUpEntityWithWinch()` (0x6D3CB0)
    fn release_picked_up_entity_with_winch(v: GPtr<u8>) = thiscall 0x6D3CB0 => unported;
    /// `CTrain::FindCarriage(CTrain*, uint8)` (0x6F5EB0)
    fn find_carriage(t: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x6F5EB0 => unported;
    /// `CPool<CVehicle>::GetRef(CVehicle*)` (0x424160)
    fn vehicle_get_ref(pool: GPtr<u8>, v: GPtr<u8>) -> u32 = thiscall 0x424160 => unported;
    /// `CVehicle::QueryPickedUpEntityWithWinch()` (0x6D3CF0)
    fn query_picked_up_entity_with_winch(v: GPtr<u8>) -> GPtr<u8> = thiscall 0x6D3CF0 => unported;
    /// `CObject::GetObjectCarriedWithRope()` (0x59F3C0)
    fn get_object_carried_with_rope(o: GPtr<u8>) -> GPtr<u8> = thiscall 0x59F3C0 => unported;
    /// `CShopping::GetNameTag(uint32)` (0x49ADA0)
    fn get_name_tag(i: u32) -> GPtr<u8> = cdecl 0x49ADA0 => unported;
    /// CRT `strncpy` (0x821F40)
    fn strncpy_(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
    /// 0x46A630 (thiscall; the 0x14-byte `CTaskComplexClimb` constructor, no arguments)
    fn task_climb_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x46A630 => unported;
    /// `CShopping::Buy(uint32, int32)` (0x49BF70)
    fn shopping_buy(i: u32, b: u32) = cdecl 0x49BF70 => unported;
    /// `CPedIntelligence::FlushImmediately(bool)` (0x601640)
    fn flush_immediately(intel: GPtr<u8>, b: u32) = thiscall 0x601640 => unported;
    /// `CShopping::StoreClothesState()` (0x49B200) / `RestoreClothesState()` (0x49B240)
    fn store_clothes_state() = cdecl 0x49B200 => unported;
    fn restore_clothes_state() = cdecl 0x49B240 => unported;
    /// `CObject::GetRopeHeight()` (0x59F380), ST0 (read as f64)
    fn get_rope_height(o: GPtr<u8>) -> f64 = thiscall 0x59F380 => unported;
    /// `CObject::SetRopeHeight(float)` (0x59F3A0); raw
    fn set_rope_height(o: GPtr<u8>, h: u32) = thiscall 0x59F3A0 => unported;
    /// `CObject::ReleaseObjectCarriedWithRope()` (0x59F3E0)
    fn release_object_carried_with_rope(o: GPtr<u8>) = thiscall 0x59F3E0 => unported;
    /// `CPool<CPed>::GetRef(CPed*)` / `CPool<CObject>::GetRef(CObject*)` with the raw result (0x4442D0 / 0x465070)
    fn ped_get_ref_(pool: GPtr<u8>, p: GPtr<u8>) -> u32 = thiscall 0x4442D0 => unported;
    fn object_get_ref_(pool: GPtr<u8>, o: GPtr<u8>) -> u32 = thiscall 0x465070 => unported;
    /// `CTheScripts::GetActualScriptThingIndex(int32 ref, uint8 type)` (0x4839A0)
    fn get_actual_script_thing_index(r: u32, ty: u32) -> i32 = cdecl 0x4839A0 => unported;
    /// `CTaskComplexUseSequence::CTaskComplexUseSequence(int32)` (0x635450)
    fn task_use_sequence_ctor(t: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x635450 => unported;
    /// `CTaskComplexSeekEntityAiming::CTaskComplexSeekEntityAiming(CEntity*, float, float)` (0x694B90); raw
    fn task_seek_entity_aiming_ctor(t: GPtr<u8>, e: GPtr<u8>, a: u32, b: u32) -> GPtr<u8> = thiscall 0x694B90 => unported;
    /// `CPedScriptedTaskRecord::GetStatus(const CPed*, int32 opcode)` (0x608710)
    fn scripted_task_status(p: GPtr<u8>, op: u32) -> u32 = cdecl 0x608710 => unported;
    /// `CTask` virtual slot 8 (`GetSubTask()`)
    fn task_get_sub_task(t: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x08 => unported;
    /// `CTask` virtual slot 0x10 (`GetTaskType()`)
    fn task_get_type(t: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `CTaskComplexKillPedOnFoot::CTaskComplexKillPedOnFoot(CPed*, int32, uint32, uint32, int32, int8)` (0x620E30)
    fn task_kill_ped_on_foot_ctor(t: GPtr<u8>, target: GPtr<u8>, time: u32, a: u32, b: u32, c: u32, d: u32) -> GPtr<u8> = thiscall 0x620E30 => unported;
    /// `CTagManager::GetNearestTag(const CVector&)` (0x49D160)
    fn get_nearest_tag(pos: GPtr<u32>) -> GPtr<u8> = cdecl 0x49D160 => unported;
    /// `CTaskSimpleJetPack::CTaskSimpleJetPack(const CVector*, float, int32, CEntity*)` (0x67B4E0); raw
    fn task_jetpack_ctor(t: GPtr<u8>, pos: u32, h: u32, a: u32, e: u32) -> GPtr<u8> = thiscall 0x67B4E0 => unported;
    /// 0x493960 (cdecl `(CPed*, int32* out)`, AL: the searchlight query; writes `*out` when it returns true)
    fn is_ped_in_any_searchlight(p: GPtr<u8>, out: GPtr<u32>) -> u8 = cdecl 0x493960 => unported;
    /// `CTheScripts::GetUniqueScriptThingIndex(int32, uint8)` (0x4810C0)
    fn get_unique_script_thing_index(i: u32, ty: u32) -> u32 = cdecl 0x4810C0 => unported;
    /// `MakeUpperCase(char*)` (0x718710)
    fn make_upper_case(s: GPtr<u8>) = cdecl 0x718710 => unported;
    /// `CAudioEngine::GetBeatInfo()` (0x5071B0), ECX = AudioEngine 0xB6BC90
    fn get_beat_info(a: GPtr<u8>) -> GPtr<u8> = thiscall 0x5071B0 => unported;
    /// `CPedGroupIntelligence::SetDefaultTaskAllocatorType(int32)` (0x5FBB70)
    fn set_default_task_allocator_type(g: GPtr<u8>, t: u32) = thiscall 0x5FBB70 => unported;
    /// `CPedIntelligence::SetPedDecisionMakerType(int32)` (0x600B50)
    fn set_ped_decision_maker_type(intel: GPtr<u8>, t: i32) = thiscall 0x600B50 => unported;
    /// `CTaskSimpleSetCharDecisionMaker::CTaskSimpleSetCharDecisionMaker(int32)` (0x46A470)
    fn task_set_char_dm_ctor(t: GPtr<u8>, dm: i32) -> GPtr<u8> = thiscall 0x46A470 => unported;
    /// `CRadar::SetBlipAlwaysDisplayInZoom(int32, uint8)` (0x583DB0)
    fn set_blip_always_display_in_zoom(h: u32, b: u32) = cdecl 0x583DB0 => unported;
    /// `CVehicleRecording::RequestRecordingFile(int32)` (0x45A020) / `HasRecordingFileBeenLoaded(int32)`
    /// (0x45A060, AL)
    fn request_recording_file(i: u32) = cdecl 0x45A020 => unported;
    fn has_recording_file_been_loaded(i: u32) -> u8 = cdecl 0x45A060 => unported;
    /// `CEntity::GetModellingMatrix()` (0x46A2D0)
    fn get_modelling_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x46A2D0 => unported;
    /// 0x59C3E0 (thiscall on a `CQuaternion`, `RwMatrix*`; gr `CQuaternion::Set(const RwMatrix&)`)
    fn quaternion_set(q: GPtr<u32>, m: GPtr<u8>) = thiscall 0x59C3E0 => unported;
    /// `CQuaternion::Get(RwMatrix*)` (0x59C080)
    fn quaternion_get(q: GPtr<u32>, m: GPtr<u8>) = thiscall 0x59C080 => unported;
    /// 0x59C050 (thiscall; `CMatrix::CMatrix(RwMatrix*, bool owner)`)
    fn matrix_from_rw(m: GPtr<u8>, rw: GPtr<u8>, owner: u32) -> GPtr<u8> = thiscall 0x59C050 => unported;
    /// `CPlaceable::SetMatrix(const CMatrix&)` (0x54F610)
    fn placeable_set_matrix(e: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F610 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CPlaceable::SetPosn(const CVector&)` (0x4241C0)
    fn placeable_set_posn(e: GPtr<u8>, p: GPtr<u32>) = thiscall 0x4241C0 => unported;
    /// 0x6919C0 (thiscall; the 0x34-byte task constructor of vtable 0x870B98, `(CEntity*, int32)`)
    fn task_pickup_object_ctor(t: GPtr<u8>, o: GPtr<u8>, n: u32) -> GPtr<u8> = thiscall 0x6919C0 => unported;
    /// 0x53FB70 (cdecl `(int32)`: `CPad::GetPad`)
    fn get_pad(i: u32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CTaskComplexSequence::CTaskComplexSequence()` (0x632BD0) / `AddTask(CTask*)` (0x632D10)
    fn task_sequence_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632BD0 => unported;
    fn task_sequence_add(seq: GPtr<u8>, t: GPtr<u8>) = thiscall 0x632D10 => unported;
    /// `CTaskComplexGoToPointAndStandStill::CTaskComplexGoToPointAndStandStill(int32, const CVector&, float,
    /// float, bool, bool)` (0x668120); floats raw
    fn task_go_to_point_ctor(t: GPtr<u8>, ms: u32, pos: GPtr<u32>, r: u32, r2: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x668120 => unported;
    /// `CTaskSimpleSlideToCoord::CTaskSimpleSlideToCoord(const CVector&, float, float)` (0x66C3E0); raw
    fn task_slide_to_coord_ctor(t: GPtr<u8>, pos: GPtr<u32>, h: u32, speed: u32) -> GPtr<u8> = thiscall 0x66C3E0 => unported;
}

/// `this->vtable[0xF8 / 4]()` (`CVehicle::BreakTowLink`)
#[inline]
fn break_tow_link(v: GPtr<u8>) {
    let f = v.cast::<GPtr<u32>>().read().byte_add(0xF8).read();
    vcall0(f, v);
}

/// `VEH(h)` / `PED(h)` / `OBJ(h)` (GetAtRef; no null check by the callers unless stated).
#[inline]
fn veh(h: u32) -> GPtr<u8> {
    vehicle_at_ref(pool(VEHICLE_POOL), h as i32)
}
#[inline]
fn ped(h: u32) -> GPtr<u8> {
    ped_at_ref(pool(PED_POOL), h as i32)
}
#[inline]
fn obj(h: u32) -> GPtr<u8> {
    object_at_ref(pool(OBJECT_POOL), h as i32)
}

/// The entity position: `m_matrix (+0x14) ? m_matrix + 0x30 : this + 4`.
fn ent_pos(e: GPtr<u8>) -> GPtr<u32> {
    let m = e.byte_add(0x14).cast::<GPtr<u8>>().read();
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
    .cast::<u32>()
}

/// `task` or null when `operator new` failed (the constructor runs only on a non-null block).
#[inline]
fn new_task(size: u32, ctor: impl FnOnce(GPtr<u8>) -> GPtr<u8>) -> GPtr<u8> {
    let t = task_new(size);
    if t.is_null() {
        GPtr::NULL
    } else {
        ctor(t)
    }
}

/// The zone's extra-info record: `0xBA1DEC + 17 * (u16)[0xBA37B4 + 32 * z]` (`z` sign-extended, unchecked).
#[inline]
fn zone_info(z: u16) -> u32 {
    let navi =
        GPtr::<u16>::from_addr(((i32::from(z as i16) << 5) as u32).wrapping_add(0xBA37B4) as usize).read();
    u32::from(navi) * 0x11
}

/// `CRunningScript::ProcessCommands1900To1999(int32 command)` (0x46B460, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x46B460, conv = thiscall, name = "CRunningScript::ProcessCommands1900To1999", expect = "6A FF 68 4D 7F 83 00", this = "CRunningScript")]
pub fn process_commands_1900_to_1999(s: Scr, command: u32) -> i8 {
    match command {
        // 076E, 0773..0775, 0779, 077F, 0785, 0787, 078D, 078E, 0791, 0795, 079A..079C, 07A2, 07AA, 07AD, 07AE,
        // 07B2, 07B5..07B8, 07BA, 07C2, 07C8, 07CA, 07CE, 07CF: the jump table sends them to 0x46CBCB (`xor
        // al,al`: no operand read, return 0). gr: nop (0787, 078E, 079B, 07B2: unimplemented, -1).
        0x076E | 0x0773..=0x0775 | 0x0779 | 0x077F | 0x0785 | 0x0787 | 0x078D | 0x078E | 0x0791 => {}
        0x0795 | 0x079A..=0x079C | 0x07A2 | 0x07AA | 0x07AD | 0x07AE | 0x07B2 | 0x07B5..=0x07B8 => {}
        0x07BA | 0x07C2 | 0x07C8 | 0x07CA | 0x07CE | 0x07CF => {}
        // 076C SET_ZONE_GANG_STRENGTH (0x46B4BE): 8-byte label (zeroed 16-byte buffer), `CollectParameters(2)`, `z =
        // FindZoneByLabel(label, 2)`; if `z >= 0`: byte `0xBA1DF0 + info + P0` = (u8)P1 (gang index unchecked).
        0x076C => {
            let mut label = [0u8; 16];
            let lp = GPtr::from_raw(label.as_mut_ptr());
            read_label(s, lp, 8);
            collect(s, 2);
            let z = find_zone_by_label(lp, 2);
            if (z as i16) >= 0 {
                let info = zone_info(z);
                let a = info.wrapping_add(sp(0)).wrapping_add(0xBA1DF0);
                GPtr::<u8>::from_addr(a as usize).write(sp(1) as u8);
            }
        }
        // 076D GET_ZONE_GANG_STRENGTH (0x46B517): label, `CollectParameters(1)`, no `z >= 0` check (kept); P0 =
        // byte `0xBA1DF0 + info + P0`; store 1.
        0x076D => {
            let mut label = [0u8; 16];
            let lp = GPtr::from_raw(label.as_mut_ptr());
            read_label(s, lp, 8);
            collect(s, 1);
            let z = find_zone_by_label(lp, 2);
            let info = zone_info(z);
            let a = info.wrapping_add(sp(0)).wrapping_add(0xBA1DF0);
            set_sp(0, u32::from(GPtr::<u8>::from_addr(a as usize).read()));
            store(s, 1);
        }
        // 076F IS_MESSAGE_BEING_DISPLAYED (0x46B571): flag = `CMessages::BriefMessages[0].m_pText` (0xC1A7F0) != 0.
        0x076F => set_flag(s, GPtr::<u32>::from_addr(0xC1A7F0).read() != 0),
        // 0770 SET_CHAR_IS_TARGET_PRIORITY (0x46B597): `PED(P0)` +0x470 bit 29 = P1 != 0.
        0x0770 => {
            collect(s, 2);
            let fl = ped(sp(0)).byte_add(0x470).cast::<u32>();
            let v = fl.read();
            fl.write(if sp(1) != 0 {
                v | 0x2000_0000
            } else {
                v & 0xDFFF_FFFF
            });
        }
        // 0771 CUSTOM_PLATE_DESIGN_FOR_NEXT_CAR (0x46B5E8): `mi = ms_modelInfoPtrs[P0]` (0xA9B0C8, unchecked); if
        // `mi`, `(u8)mi->GetModelType() == 6` and `mi+0x24 != 0`: byte mi+0x31 = (u8)P1.
        0x0771 => {
            collect(s, 2);
            let mi =
                GPtr::<GPtr<u8>>::from_addr(0xA9B0C8u32.wrapping_add(sp(0).wrapping_mul(4)) as usize).read();
            if !mi.is_null() && model_info_type(mi) == 6 && mi.byte_add(0x24).cast::<u32>().read() != 0 {
                mi.byte_add(0x31).write(sp(1) as u8);
            }
        }
        // 0772 TASK_GOTO_CAR (0x46B630): `v = VEH(P1)`; `t = P2 < 0 ? 50000 : P2`; task (0x4C)
        // `CTaskComplexSeekEntity(v, t, 1000, P3, [0x859E30], [0x859E34], true, true)`; `GivePedScriptedTask(P0,
        // task, command)`.
        0x0772 => {
            collect(s, 4);
            let (p0, p3) = (sp(0), sp(3));
            let mut t = sp(2);
            let v = veh(sp(1));
            if (t as i32) < 0 {
                t = 50000;
            }
            let task = new_task(0x4C, |k| {
                let a = GPtr::<u32>::from_addr(0x859E34).read();
                let b = GPtr::<u32>::from_addr(0x859E30).read();
                task_seek_entity_ctor(k, v, t, 1000, p3, b, a, 1, 1)
            });
            give_ped_scripted_task(s, p0, task, command);
        }
        // 0776 REQUEST_IPL (0x46B6D4) / 0777 REMOVE_IPL (0x46B6FC) / 0778 REMOVE_IPL_DISCREETLY (0x46B724): 18-byte
        // label (zeroed 19-byte buffer); `CIplStore::RequestIplAndIgnore / RemoveIplAndIgnore /
        // RemoveIplWhenFarAway(FindIplSlot(label))`.
        0x0776..=0x0778 => {
            let mut label = [0u8; 19];
            let lp = GPtr::from_raw(label.as_mut_ptr());
            read_label(s, lp, 0x12);
            let slot = find_ipl_slot(lp);
            if command == 0x0776 {
                request_ipl_and_ignore(slot);
            } else if command == 0x0777 {
                remove_ipl_and_ignore(slot);
            } else {
                remove_ipl_when_far_away(slot);
            }
        }
        // 077A SET_CHAR_RELATIONSHIP (0x46B74C) / 077B CLEAR_CHAR_RELATIONSHIP (0x46B791): `p = PED(P0)`;
        // `p->m_acquaintance (+0x4E0).Set/ClearAsAcquaintance(P1, GetPedFlag(P2))`.
        0x077A | 0x077B => {
            collect(s, 3);
            let (a, t) = (sp(1), sp(2));
            let p = ped(sp(0));
            let fl = get_ped_flag(t);
            if command == 0x077A {
                set_as_acquaintance(p.byte_add(0x4E0), a, fl);
            } else {
                clear_as_acquaintance(p.byte_add(0x4E0), a, fl);
            }
        }
        // 077C CLEAR_ALL_CHAR_RELATIONSHIPS (0x46B7D7): `PED(P0)->m_acquaintance.ClearAcquaintances(P1)`.
        0x077C => {
            collect(s, 2);
            let a = sp(1);
            clear_acquaintances(ped(sp(0)).byte_add(0x4E0), a);
        }
        // 077D GET_CAR_PITCH (0x46B80C): `p = GetCarPitch(VEH(P0)) * 57.295776` (`fst` f32); ST0 < 0 (`test ah,5;
        // jp`) -> `p += 360`; then `p > 360` (`test ah,0x41; jne`: only Greater) -> `p -= 360`; P0 = p; store 1.
        0x077D => {
            collect(s, 1);
            let st = X87::from_f64_exact(get_car_pitch(veh(sp(0)))).fmul_m(f(K_RAD2DEG));
            let mut p = st.to_f32();
            if st.fcom(X87::from_f32(f(K_ZERO))) == Cmp::Less {
                p = X87::from_f32(p).fadd_m(f(K_360)).to_f32();
            }
            if X87::from_f32(p).fcom(X87::from_f32(f(K_360))) == Cmp::Greater {
                p = X87::from_f32(p).fsub_m(f(K_360)).to_f32();
            }
            set_sp(0, p.to_bits());
            store(s, 1);
        }
        // 077E GET_AREA_VISIBLE (0x46B88C): `SP0 = CGame::currArea`; `StoreParameters(1)`
        0x077E => {
            set_sp(0, GPtr::<u32>::from_addr(CURR_AREA).read());
            store(s, 1);
        }
        // 0780 HELI_KEEP_ENTITY_IN_VIEW (0x46B8A6)
        0x0780 => heli_keep_entity_in_view(s),
        // 0781 GET_WEAPONTYPE_MODEL (0x46B960) / 0782 GET_WEAPONTYPE_SLOT (0x46B992): P0 = `GetWeaponInfo(P0,
        // 1)` +0xC / +0x14; store 1.
        0x0781 | 0x0782 => {
            collect(s, 1);
            let wi = get_weapon_info(sp(0), 1);
            let off = if command == 0x0781 { 0xC } else { 0x14 };
            set_sp(0, wi.byte_add(off).cast::<u32>().read());
            store(s, 1);
        }
        // 0783 GET_SHOPPING_EXTRA_INFO (0x46B9C5): P0 = `CShopping::GetExtraInfo(P0, P1)`; store 1.
        0x0783 => {
            collect(s, 2);
            let v = get_extra_info(sp(0), sp(1));
            set_sp(0, v);
            store(s, 1);
        }
        // 0784 GIVE_PLAYER_CLOTHES (0x46B9F8): `Players[P0].m_pPedClothesDesc (+8)->SetTextureAndModel(P1, P2,
        // P3)`.
        0x0784 => {
            collect(s, 4);
            let (a, b, c) = (sp(1), sp(2), sp(3));
            let d = player_info(sp(0)).byte_add(8).cast::<GPtr<u8>>().read();
            set_texture_and_model(d, a, b, c);
        }
        // 0786 GET_NUMBER_OF_FIRES_IN_AREA (0x46BA33)
        0x0786 => number_of_fires_in_area(s),
        // 0788 ATTACH_WINCH_TO_HELI (0x46BB10): `VEH(P0)->InitWinch(P1)`.
        0x0788 => {
            collect(s, 2);
            let n = sp(1);
            init_winch(veh(sp(0)), n);
        }
        // 0789 RELEASE_ENTITY_FROM_WINCH (0x46BB3F): `VEH(P0)->ReleasePickedUpEntityWithWinch()`.
        0x0789 => {
            collect(s, 1);
            release_picked_up_entity_with_winch(veh(sp(0)));
        }
        // 078A GET_TRAIN_CARRIAGE (0x46BB68): `c = CTrain::FindCarriage(VEH(P0), (u8)P1)`; P0 = c ? GetRef(c) :
        // -1; store 1.
        0x078A => {
            collect(s, 2);
            let v = veh(sp(0));
            let c = find_carriage(v, sp(1) & 0xFF);
            let r = if c.is_null() {
                u32::MAX
            } else {
                vehicle_get_ref(pool(VEHICLE_POOL), c)
            };
            set_sp(0, r);
            store(s, 1);
        }
        // 078B GRAB_ENTITY_ON_WINCH (0x46BBD3) / 0798 GRAB_ENTITY_ON_ROPE_FOR_OBJECT (0x46BD94): P0..P2 = -1; `e =
        // VEH(P0)->QueryPickedUpEntityWithWinch()` / `OBJ(P0)->GetObjectCarriedWithRope()`; tail 0x46BDCB.
        0x078B | 0x0798 => {
            collect(s, 1);
            let h = sp(0);
            let src = if command == 0x078B { veh(h) } else { obj(h) };
            set_sp(0, u32::MAX);
            set_sp(1, u32::MAX);
            set_sp(2, u32::MAX);
            let e = if command == 0x078B {
                query_picked_up_entity_with_winch(src)
            } else {
                get_object_carried_with_rope(src)
            };
            grabbed_entity_tail(s, e);
        }
        // 078C GET_NAME_OF_ITEM (0x46BC30): `n = CShopping::GetNameTag(P0)`; `strncpy(GetPointerToScriptVariable(2),
        // n, 8)`.
        0x078C => {
            collect(s, 1);
            let n = get_name_tag(sp(0));
            let p = var_ptr(s, 2);
            strncpy_(p.cast(), n, 8);
        }
        // 078F TASK_CLIMB (0x46BC65): `b = P1 != 0`; task (0x14) 0x46A630(); byte task+0x10 = b (not null-checked);
        // `GivePedScriptedTask(P0, task, command)`.
        0x078F => {
            collect(s, 2);
            let b = u8::from(sp(1) != 0);
            let p0 = sp(0);
            let t = new_task(0x14, task_climb_ctor);
            t.byte_add(0x10).write(b);
            give_ped_scripted_task(s, p0, t, command);
        }
        // 0790 BUY_ITEM (0x46BCC6): `CShopping::Buy(P0, 0)`.
        0x0790 => {
            collect(s, 1);
            shopping_buy(sp(0), 0);
        }
        // 0792 CLEAR_CHAR_TASKS_IMMEDIATELY (0x46BCE6): `PED(P0)->m_pIntelligence->FlushImmediately(true)`.
        0x0792 => {
            collect(s, 1);
            let intel = ped(sp(0)).byte_add(0x47C).cast::<GPtr<u8>>().read();
            flush_immediately(intel, 1);
        }
        // 0793 STORE_CLOTHES_STATE (0x46BD15) / 0794 RESTORE_CLOTHES_STATE (0x46BD21)
        0x0793 => store_clothes_state(),
        0x0794 => restore_clothes_state(),
        // 0796 GET_ROPE_HEIGHT_FOR_OBJECT (0x46BD2D): P0 = `(float)OBJ(P0)->GetRopeHeight()` (`fstp`); store 1.
        0x0796 => {
            collect(s, 1);
            let h = X87::from_f64_exact(get_rope_height(obj(sp(0)))).to_f32();
            set_sp(0, h.to_bits());
            store(s, 1);
        }
        // 0797 SET_ROPE_HEIGHT_FOR_OBJECT (0x46BD65): `OBJ(P0)->SetRopeHeight(P1)` (raw).
        0x0797 => {
            collect(s, 2);
            let h = sp(1);
            set_rope_height(obj(sp(0)), h);
        }
        // 0799 RELEASE_ENTITY_FROM_ROPE_FOR_OBJECT (0x46BE30): `OBJ(P0)->ReleaseObjectCarriedWithRope()`.
        0x0799 => {
            collect(s, 1);
            release_object_carried_with_rope(obj(sp(0)));
        }
        // 079D PLAYER_ENTERED_DOCK_CRANE (0x46BE58) / 079E PLAYER_ENTERED_BUILDINGSITE_CRANE (0x46BE70) / 079F
        // PLAYER_LEFT_CRANE (0x46BE88): dword 0xB76898 = 1 / 2 / 0; byte 0x8D37D5 = 0 / 0 / 1.
        0x079D..=0x079F => {
            let (a, b) = if command == 0x079D {
                (1, 0)
            } else if command == 0x079E {
                (2, 0)
            } else {
                (0, 1)
            };
            GPtr::<u32>::from_addr(0xB76898).write(a);
            GPtr::<u8>::from_addr(0x8D37D5).write(b);
        }
        // 07A0 PERFORM_SEQUENCE_TASK_FROM_PROGRESS (0x46BEA0): `CollectParameters(4)`; `i =
        // GetActualScriptThingIndex(P1, 4)`; if `0 <= i < 64`: task (0x1C) `CTaskComplexUseSequence(i)`; task+0x14 =
        // P3, task+0x10 = P2 (not null-checked); `GivePedScriptedTask(P0, task, command)`.
        0x07A0 => {
            collect(s, 4);
            let p0 = sp(0);
            let i = get_actual_script_thing_index(sp(1), 4);
            let (p2, p3) = (sp(2), sp(3));
            if (0..64).contains(&i) {
                let t = new_task(0x1C, |t| task_use_sequence_ctor(t, i));
                t.byte_add(0x14).cast::<u32>().write(p3);
                t.byte_add(0x10).cast::<u32>().write(p2);
                give_ped_scripted_task(s, p0, t, command);
            }
        }
        // 07A1 SET_NEXT_DESIRED_MOVE_STATE (0x46BF39): dword 0x8D237C = P0.
        0x07A1 => {
            collect(s, 1);
            GPtr::<u32>::from_addr(0x8D237C).write(sp(0));
        }
        // 07A3 TASK_GOTO_CHAR_AIMING (0x46BF53): `target = PED(P1)`; task (0x18) `CTaskComplexSeekEntityAiming(
        // target, P2, P3)`; `GivePedScriptedTask(P0, ...)`.
        0x07A3 => {
            collect(s, 4);
            let (p0, p2, p3) = (sp(0), sp(2), sp(3));
            let target = ped(sp(1));
            let t = new_task(0x18, |t| task_seek_entity_aiming_ctor(t, target, p2, p3));
            give_ped_scripted_task(s, p0, t, command);
        }
        // 07A4 GET_SEQUENCE_PROGRESS_RECURSIVE (0x46BFD8): `p = PED(P0)`; `a = b = -1`; if
        // `CPedScriptedTaskRecord::GetStatus(p, 0x618)`: `t = p->m_pIntelligence+0x10` (not null-checked), `a =
        // t+0x10`, `sub = t->GetSubTask()`; `sub && sub->GetTaskType() == 0x113` -> `b = sub+0x10`. P0 = a, P1 = b;
        // store 2.
        0x07A4 => {
            collect(s, 1);
            let p = ped(sp(0));
            let (mut a, mut b) = (u32::MAX, u32::MAX);
            if scripted_task_status(p, 0x618) != 0 {
                let t = p
                    .byte_add(0x47C)
                    .cast::<GPtr<u8>>()
                    .read()
                    .byte_add(0x10)
                    .cast::<GPtr<u8>>()
                    .read();
                a = t.byte_add(0x10).cast::<u32>().read();
                let sub = task_get_sub_task(t);
                if !sub.is_null() && task_get_type(sub) == 0x113 {
                    b = sub.byte_add(0x10).cast::<u32>().read();
                }
            }
            set_sp(0, a);
            set_sp(1, b);
            store(s, 2);
        }
        // 07A5 TASK_KILL_CHAR_ON_FOOT_TIMED (0x46C04F): `target = PED(P1)`; task (0x38)
        // `CTaskComplexKillPedOnFoot(target, P2, 0, 0, 0, 1)`; `GivePedScriptedTask(P0, ...)`.
        0x07A5 => {
            collect(s, 3);
            let (p0, p2) = (sp(0), sp(2));
            let target = ped(sp(1));
            let t = new_task(0x38, |t| task_kill_ped_on_foot_ctor(t, target, p2, 0, 0, 0, 1));
            give_ped_scripted_task(s, p0, t, command);
        }
        // 07A6 GET_NEAREST_TAG_POSITION (0x46C0AF): `t = CTagManager::GetNearestTag(&(P0, P1, P2))`; P0..P2 = t's
        // position, or -4000.0f (0xC57A0000) each; store 3.
        0x07A6 => {
            collect(s, 3);
            let mut pos = [sp(0), sp(1), sp(2)];
            let t = get_nearest_tag(GPtr::from_raw(pos.as_mut_ptr()));
            if t.is_null() {
                for i in 0..3 {
                    set_sp(i, 0xC57A_0000);
                }
            } else {
                let p = ent_pos(t);
                let (x, y, z) = (p.read(), p.byte_add(4).read(), p.byte_add(8).read());
                set_sp(0, x);
                set_sp(1, y);
                set_sp(2, z);
            }
            store(s, 3);
        }
        // 07A7 TASK_JETPACK (0x46C149): task (0x70) `CTaskSimpleJetPack(nullptr, 10.0f, 0, nullptr)`;
        // `GivePedScriptedTask(P0, ...)`.
        0x07A7 => {
            collect(s, 1);
            let p0 = sp(0);
            let t = new_task(0x70, |t| task_jetpack_ctor(t, 0, 0x4120_0000, 0, 0));
            give_ped_scripted_task(s, p0, t, command);
        }
        // 07A8 SET_AREA51_SAM_SITE (0x46C1A7): `CObject::bArea51SamSiteDisabled` (0xBB4A72) = P0 == 0.
        0x07A8 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0xBB4A72).write(u8::from(sp(0) == 0));
        }
        // 07A9 IS_CHAR_IN_ANY_SEARCHLIGHT (0x46C1C7): `p = PED(P0)`; P0 = -1; if `0x493960(p, &out)`: P0 = out, flag
        // 1 (else 0); store 1; then UpdateCompareFlag. `out` is read only when the callee returned true.
        0x07A9 => {
            collect(s, 1);
            let p = ped(sp(0));
            let mut out = [0u32; 1];
            set_sp(0, u32::MAX);
            let r = is_ped_in_any_searchlight(p, GPtr::from_raw(out.as_mut_ptr())) != 0;
            if r {
                set_sp(0, out[0]);
            }
            store(s, 1);
            set_flag(s, r);
        }
        // 07AB IS_TRAILER_ATTACHED_TO_CAB (0x46C22E): `t = P0 == -1 ? 0 : VEH(P0)`, `c = P1 == -1 ? 0 : VEH(P1)`
        // (pool pointer loaded once); `t`: flag = `c ? t->m_pTractor (+0x4C4) == c : t->m_pTractor != 0`; else flag =
        // `c->m_pTrailer (+0x4C8) != 0` (c not null-checked).
        0x07AB => {
            collect(s, 2);
            let p = pool(VEHICLE_POOL);
            let t = if spi(0) == -1 {
                GPtr::NULL
            } else {
                vehicle_at_ref(p, spi(0))
            };
            let c = if spi(1) == -1 {
                GPtr::NULL
            } else {
                vehicle_at_ref(p, spi(1))
            };
            let r = if !t.is_null() {
                let tr = t.byte_add(0x4C4).cast::<GPtr<u8>>().read();
                if c.is_null() {
                    !tr.is_null()
                } else {
                    tr == c
                }
            } else {
                !c.byte_add(0x4C8).cast::<GPtr<u8>>().read().is_null()
            };
            set_flag(s, r);
        }
        // 07AC DETACH_TRAILER_FROM_CAB (0x46C2AF)
        0x07AC => detach_trailer_from_cab(s),
        // 07AF GET_PLAYER_GROUP (0x46C334): P0 = `GetUniqueScriptThingIndex(Players[P0].m_pPed->m_pPlayerData
        // (+0x480)->m_nPlayerGroup (+0x38), 8)`; store 1.
        0x07AF => {
            collect(s, 1);
            let p = player_info(sp(0)).cast::<GPtr<u8>>().read();
            let g = p
                .byte_add(0x480)
                .cast::<GPtr<u8>>()
                .read()
                .byte_add(0x38)
                .cast::<u32>()
                .read();
            let v = get_unique_script_thing_index(g, 8);
            set_sp(0, v);
            store(s, 1);
        }
        // 07B0 GET_LOADED_SHOP (0x46C364): `strncpy(buf, CShopping::ms_shopLoaded (0xA9A7D8), 8)`;
        // `MakeUpperCase(buf)`; `strncpy(GetPointerToScriptVariable(2), buf, 8)`.
        0x07B0 => {
            let mut buf = [0u8; 16];
            let bp = GPtr::from_raw(buf.as_mut_ptr());
            strncpy_(bp, GPtr::from_addr(0xA9A7D8), 8);
            make_upper_case(bp);
            let p = var_ptr(s, 2);
            strncpy_(p.cast(), bp, 8);
        }
        // 07B1 GET_BEAT_PROXIMITY (0x46C3A2)
        0x07B1 => beat_proximity(s),
        // 07B3 SET_GROUP_DEFAULT_TASK_ALLOCATOR (0x46C4B7): `g = GetActualScriptThingIndex(P0, 8)`; if `0 <= g <
        // 8`: `(0xC09950 + g * 0x2D4)->SetDefaultTaskAllocatorType(P1)`.
        0x07B3 => {
            collect(s, 2);
            let g = get_actual_script_thing_index(sp(0), 8);
            if (0..8).contains(&g) {
                let gi = GPtr::from_addr(0xC09950 + g as usize * 0x2D4);
                set_default_task_allocator_type(gi, sp(1));
            }
        }
        // 07B4 SET_PLAYER_GROUP_RECRUITMENT (0x46C503): `Players[P0].m_pPed->m_pPlayerData (+0x480)` dword +0x34:
        // P1 != 0 -> `&= ~0x100`, else `|= 0x100`.
        0x07B4 => {
            collect(s, 2);
            let p = player_info(sp(0)).cast::<GPtr<u8>>().read();
            let fl = p
                .byte_add(0x480)
                .cast::<GPtr<u8>>()
                .read()
                .byte_add(0x34)
                .cast::<u32>();
            let v = fl.read();
            fl.write(if sp(1) != 0 { v & 0xFFFF_FEFF } else { v | 0x100 });
        }
        // 07BB ACTIVATE_HELI_SPEED_CHEAT (0x46C552): byte `VEH(P0)+0x3D9` = (u8)P1.
        0x07BB => {
            collect(s, 2);
            let v = veh(sp(0));
            v.byte_add(0x3D9).write(sp(1) as u8);
        }
        // 07BC TASK_SET_CHAR_DECISION_MAKER (0x46C580): `dm = P1 != -1 ? GetActualScriptThingIndex(P1, 7) : -1`;
        // `P0 != -1`: `PED(P0)->m_pIntelligence->SetPedDecisionMakerType(dm)`; else task (0xC)
        // `CTaskSimpleSetCharDecisionMaker(dm)`, `GivePedScriptedTask(-1, task, command)`.
        0x07BC => {
            collect(s, 2);
            let p0 = sp(0);
            let dm = if spi(1) != -1 {
                get_actual_script_thing_index(sp(1), 7)
            } else {
                -1
            };
            if p0 as i32 != -1 {
                let intel = ped(p0).byte_add(0x47C).cast::<GPtr<u8>>().read();
                set_ped_decision_maker_type(intel, dm);
            } else {
                let t = new_task(0xC, |t| task_set_char_dm_ctor(t, dm));
                give_ped_scripted_task(s, u32::MAX, t, command);
            }
        }
        // 07BD DELETE_MISSION_TRAIN (0x46C615)
        0x07BD => {
            collect(s, 1);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            if !v.is_null() {
                remove_one_mission_train(v);
            }
        }
        // 07BE MARK_MISSION_TRAIN_AS_NO_LONGER_NEEDED (0x46C647): if `VEH(P0)`: `CTrain::ReleaseOneMissionTrain`.
        0x07BE => {
            collect(s, 1);
            let v = veh(sp(0));
            if !v.is_null() {
                release_one_mission_train(v);
            }
        }
        // 07BF SET_BLIP_ALWAYS_DISPLAY_ON_ZOOMED_RADAR (0x46C67A): `CRadar::SetBlipAlwaysDisplayInZoom(P0, (u8)P1)`.
        0x07BF => {
            collect(s, 2);
            set_blip_always_display_in_zoom(sp(0), sp(1) & 0xFF);
        }
        // 07C0 REQUEST_CAR_RECORDING (0x46C6A1) / 07C1 HAS_CAR_RECORDING_BEEN_LOADED (0x46C6C0, flag = AL != 0).
        0x07C0 => {
            collect(s, 1);
            request_recording_file(sp(0));
        }
        0x07C1 => {
            collect(s, 1);
            set_flag(s, has_recording_file_been_loaded(sp(0)) != 0);
        }
        // 07C3 GET_OBJECT_QUATERNION (0x46C6E4) / 07C5 GET_VEHICLE_QUATERNION (0x46C7F7): `q` (the zeroed
        // function-scope quaternion) `.Set(*GetModellingMatrix(OBJ/VEH(P0)))`; P0..P3 = q; store 4.
        0x07C3 | 0x07C5 => {
            collect(s, 1);
            let e = if command == 0x07C3 { obj(sp(0)) } else { veh(sp(0)) };
            let m = get_modelling_matrix(e);
            let mut q = [0u32; 4];
            quaternion_set(GPtr::from_raw(q.as_mut_ptr()), m);
            for (i, v) in q.iter().enumerate() {
                set_sp(i as i32, *v);
            }
            store(s, 4);
        }
        // 07C4 SET_OBJECT_QUATERNION (0x46C747) / 07C6 SET_VEHICLE_QUATERNION (0x46C85A)
        0x07C4 | 0x07C6 => set_quaternion(s, command == 0x07C4),
        // 07C7 SET_MISSION_TRAIN_COORDINATES (0x46C928): the vector is copied before the lookup (no null check)
        0x07C7 => {
            collect(s, 4);
            let (x, y, z) = (sp(1), sp(2), sp(3));
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            set_new_train_position(v, x, y, z);
        }
        // 07C9 TASK_COMPLEX_PICKUP_OBJECT (0x46C988): `o = OBJ(P1)`; task (0x34) `0x6919C0(o, 0x51)`;
        // `GivePedScriptedTask(P0, ...)` (P0 read after the constructor).
        0x07C9 => {
            collect(s, 2);
            let o = obj(sp(1));
            let t = new_task(0x34, |t| task_pickup_object_ctor(t, o, 0x51));
            give_ped_scripted_task(s, sp(0), t, command);
        }
        // 07CB LISTEN_TO_PLAYER_GROUP_COMMANDS (0x46C9F2): `PED(P0)` +0x474 bit 5 = P1 == 0.
        0x07CB => {
            collect(s, 2);
            let fl = ped(sp(0)).byte_add(0x474).cast::<u32>();
            let v = fl.read();
            fl.write(if sp(1) != 0 { v & 0xFFFF_FFDF } else { v | 0x20 });
        }
        // 07CC SET_PLAYER_ENTER_CAR_BUTTON (0x46CA3C): byte `CPad::GetPad(P0)+0x119` = P1 != 0 ? 0 : 1.
        0x07CC => {
            collect(s, 2);
            let on = sp(1) != 0;
            get_pad(sp(0)).byte_add(0x119).write(u8::from(!on));
        }
        // 07CD TASK_CHAR_SLIDE_TO_COORD (0x46CA88)
        0x07CD => task_char_slide_to_coord(s, command),
        _ => return -1,
    }
    0
}

/// 0780 HELI_KEEP_ENTITY_IN_VIEW (0x46B8A6): `CollectParameters(5)`; `h = VEH(P0)`; `e = 0`; `P1 >= 0` -> `e =
/// PED(P1)`; `P2 >= 0` -> `e = VEH(P2)`; mission guard -> 0x33; `field = h+0x41C`: old non-null ->
/// `old->CleanUpOldReference(&field)`; `field = e`; `e->RegisterReference(&field)` (not null-checked: both handles
/// < 0 call it with this = 0, kept); +0x9B0 = P4 (raw); cruise (+0x3D0) = 100; byte +0x3D8 = `(u8)_ftol(P3)`.
fn heli_keep_entity_in_view(s: Scr) {
    collect(s, 5);
    let h = veh(sp(0));
    let mut e = GPtr::NULL;
    if spi(1) >= 0 {
        e = ped(sp(1));
    }
    if spi(2) >= 0 {
        e = veh(sp(2));
    }
    let (p3, p4) = (spf(3), sp(4));
    let m = h.byte_add(0x3BA);
    let c = m.read();
    if c != 0x39 && c != 0x3A {
        m.write(0x33);
    }
    let field = h.byte_add(0x41C);
    let old = field.cast::<GPtr<u8>>().read();
    if !old.is_null() {
        clean_up_old_reference(old, field);
    }
    field.cast::<GPtr<u8>>().write(e);
    register_reference(e, field);
    h.byte_add(0x9B0).cast::<u32>().write(p4);
    h.byte_add(0x3D0).write(100);
    h.byte_add(0x3D8).write(X87::from_f32(p3).ftol() as u8);
}

/// 0786 GET_NUMBER_OF_FIRES_IN_AREA (0x46BA33): `CollectParameters(6)`; per axis (P0, P3), (P1, P4), (P2, P5):
/// when `lo > hi` (`fcomp; test ah,0x41; jne`: only Greater) the pair is swapped as `hi = fld/fstp lo`, `lo = hi`
/// (raw); P0 = `gFireManager.GetNumFiresInArea(xlo, ylo, zlo, xhi, yhi, zhi)`; store 1.
fn number_of_fires_in_area(s: Scr) {
    collect(s, 6);
    let mut lo = [sp(0), sp(1), sp(2)];
    let mut hi = [sp(3), sp(4), sp(5)];
    for i in 0..3 {
        if X87::from_f32(f(lo[i])).fcom(X87::from_f32(f(hi[i]))) == Cmp::Greater {
            let l = lo[i];
            lo[i] = hi[i];
            hi[i] = fld_fst(f(l)).to_bits();
        }
    }
    let n = get_num_fires_in_area(
        GPtr::from_addr(0xB71F80),
        lo[0],
        lo[1],
        lo[2],
        hi[0],
        hi[1],
        hi[2],
    );
    set_sp(0, n);
    store(s, 1);
}

/// The 078B / 0798 tail (0x46BDCB): `e` non-null: by `e+0x36 & 7`: 2 -> P0 = vehicle GetRef, 3 -> P1 = ped
/// GetRef, 4 -> P2 = object GetRef; store 3.
fn grabbed_entity_tail(s: Scr, e: GPtr<u8>) {
    if !e.is_null() {
        match e.byte_add(0x36).read() & 7 {
            2 => set_sp(0, vehicle_get_ref(pool(VEHICLE_POOL), e)),
            3 => set_sp(1, ped_get_ref_(pool(PED_POOL), e)),
            4 => set_sp(2, object_get_ref_(pool(OBJECT_POOL), e)),
            _ => {}
        }
    }
    store(s, 3);
}

/// 0x07AC DETACH_TRAILER_FROM_CAB (0x46C2AF): `CollectParameters(2)`; `t = SP0 == -1 ? null : VEH(SP0)`,
/// `c = SP1 == -1 ? null : VEH(SP1)` (pool pointer loaded once). If `t`: `t->BreakTowLink()` (vtable +0xF8; both
/// original sub-branches on `c` are identical). Else `x = c->m_pTrailer (+0x4C8)` (c not null-checked); if `x`:
/// `x->BreakTowLink()`.
fn detach_trailer_from_cab(s: Scr) {
    collect(s, 2);
    let p = pool(VEHICLE_POOL);
    let t = if spi(0) == -1 {
        GPtr::NULL
    } else {
        vehicle_at_ref(p, spi(0))
    };
    let c = if spi(1) == -1 {
        GPtr::NULL
    } else {
        vehicle_at_ref(p, spi(1))
    };
    if !t.is_null() {
        break_tow_link(t);
        return;
    }
    let x = c.byte_add(0x4C8).cast::<GPtr<u8>>().read();
    if !x.is_null() {
        break_tow_link(x);
    }
}

/// 07B1 GET_BEAT_PROXIMITY (0x46C3A2): `CollectParameters(1)`: n; `info = AudioEngine (0xB6BC90).GetBeatInfo()`;
/// `info+0xA0 == 0` -> P0..P2 = -1. n != 0: `i = n < 0 ? n + 10 : n + 9`; P0 = `info[i*8]`, P1 = `info[i*8+4]`,
/// P2 = `info+0xA8`. n == 0: `a = info+0x48`, `b = info+0x50`; `b != 0 && info+0xA8 != 0 && |b| > |a|` -> P0 =
/// a, P1 = info+0x4C, P2 = info+0xA8 - 1; else P0 = b, P1 = info+0x54, P2 = info+0xA8. Store 3.
fn beat_proximity(s: Scr) {
    collect(s, 1);
    let n = spi(0);
    let info = get_beat_info(GPtr::from_addr(0xB6BC90));
    let rd = |o: u32| info.byte_add(o as i32 as isize).cast::<u32>().read();
    if rd(0xA0) == 0 {
        for i in 0..3 {
            set_sp(i, u32::MAX);
        }
    } else if n != 0 {
        let i = if n < 0 {
            n.wrapping_add(10)
        } else {
            n.wrapping_add(9)
        } as u32;
        set_sp(0, rd(i.wrapping_mul(8)));
        set_sp(1, rd(i.wrapping_mul(8).wrapping_add(4)));
        set_sp(2, rd(0xA8));
    } else {
        let a = rd(0x48) as i32;
        let b = rd(0x50) as i32;
        if b != 0 && rd(0xA8) != 0 && b.wrapping_abs() > a.wrapping_abs() {
            set_sp(0, a as u32);
            set_sp(1, rd(0x4C));
            set_sp(2, rd(0xA8).wrapping_sub(1));
        } else {
            set_sp(0, b as u32);
            set_sp(1, rd(0x54));
            set_sp(2, rd(0xA8));
        }
    }
    store(s, 3);
}

/// 07C4 SET_OBJECT_QUATERNION (0x46C747) / 07C6 SET_VEHICLE_QUATERNION (0x46C85A): `CollectParameters(5)`; `e =
/// OBJ/VEH(P0)` (unchecked); `q = (P1, P2, P3, P4)`; `q.Get(&rwm)`; `pos` = e's position (copied); `CMatrix
/// tmp(&rwm, false)`; `e->SetMatrix(tmp)`; `~tmp`; `e->SetPosn(pos)`. The RwMatrix (0x40) and CMatrix (0x48)
/// temporaries are zeroed Rust buffers.
fn set_quaternion(s: Scr, object: bool) {
    collect(s, 5);
    let e = if object { obj(sp(0)) } else { veh(sp(0)) };
    let mut q = [sp(1), sp(2), sp(3), sp(4)];
    let mut rwm = [0u32; 16];
    let rp = GPtr::from_raw(rwm.as_mut_ptr()).cast::<u8>();
    quaternion_get(GPtr::from_raw(q.as_mut_ptr()), rp);
    let p = ent_pos(e);
    let mut pos = [p.read(), p.byte_add(4).read(), p.byte_add(8).read()];
    let mut tmp = [0u32; 18];
    let tp = GPtr::from_raw(tmp.as_mut_ptr()).cast::<u8>();
    matrix_from_rw(tp, rp, 0);
    placeable_set_matrix(e, tp);
    matrix_dtor(tp);
    placeable_set_posn(e, GPtr::from_raw(pos.as_mut_ptr()));
}

/// 07CD TASK_CHAR_SLIDE_TO_COORD (0x46CA88): `CollectParameters(6)`: ped P0, x P1, y P2, z P3, heading P4 (deg),
/// speed P5. `h = P4 * 0.017453292` (f32); `speed = fld/fst P5`, `P5 < 0` (`test ah,5; jp`) -> 0.1f; the vector
/// (x, y via `fld/fstp`, z raw). `seq = new(0x40) CTaskComplexSequence()`; `seq->AddTask(new(0x28)
/// CTaskComplexGoToPointAndStandStill(4, vec, [0x86FC84], [0x86FC88], 0, 0))`; `seq->AddTask(new(0x80)
/// CTaskSimpleSlideToCoord(vec, h, speed))` (seq not null-checked); `GivePedScriptedTask(P0, seq, command)`.
fn task_char_slide_to_coord(s: Scr, command: u32) {
    collect(s, 6);
    let x = fld_fst(spf(1));
    let y = fld_fst(spf(2));
    let z = sp(3);
    let p0 = sp(0);
    let h = X87::from_f32(spf(4)).fmul_m(f(K_DEG2RAD)).to_f32();
    let mut speed = fld_fst(spf(5)).to_bits();
    if X87::from_f32(spf(5)).fcom(X87::from_f32(f(K_ZERO))) == Cmp::Less {
        speed = 0x3DCC_CCCD;
    }
    let mut vec = [x.to_bits(), y.to_bits(), z];
    let vp = GPtr::from_raw(vec.as_mut_ptr());
    let seq = new_task(0x40, task_sequence_ctor);
    let t1 = new_task(0x28, |t| {
        let a = GPtr::<u32>::from_addr(0x86FC84).read();
        let b = GPtr::<u32>::from_addr(0x86FC88).read();
        task_go_to_point_ctor(t, 4, vp, a, b, 0, 0)
    });
    task_sequence_add(seq, t1);
    let t2 = new_task(0x80, |t| task_slide_to_coord_ctor(t, vp, h.to_bits(), speed));
    task_sequence_add(seq, t2);
    give_ped_scripted_task(s, p0, seq, command);
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_1900_TO_1999];
