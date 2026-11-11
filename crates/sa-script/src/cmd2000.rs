//! `CRunningScript::ProcessCommands2000To2099` (0x472310): complete port (PORT-SCRIPT-2 partial, completed in
//! PORT-SCRIPT-4). Every opcode with a jump-table case is in [`PORTED`]; 07D8, 07D9, 0802 and 082B..082F reach
//! the default (`or al,0xFF` at 0x474372: -1), as does every command outside the range.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands2000To2099.md`.
//!
//! Divergence notes (ledger classes of PORT-SCRIPT-2/3): text labels are read into zeroed buffers one byte
//! longer than the label (8-byte labels: 16 bytes); stack vectors, bounding boxes and out-parameters the
//! callees fill start zeroed; bools the original pushes as a dword whose low byte only was written are passed
//! as 0/1. The SEH state stores are not ported. 07FC's `_CIpow` (0x822130) is called through
//! `orig::x87_binary` (operands and result as doubles, the sa-world precedent).

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x07D0, 0x07D1, 0x07D2, 0x07D3, 0x07D4, 0x07D5, 0x07D6, 0x07D7, 0x07DA, 0x07DB, 0x07DC, 0x07DD, 0x07DE,
    0x07DF, 0x07E0, 0x07E1, 0x07E2, 0x07E3, 0x07E4, 0x07E5, 0x07E6, 0x07E7, 0x07E8, 0x07E9, 0x07EA, 0x07EB,
    0x07EC, 0x07ED, 0x07EE, 0x07EF, 0x07F0, 0x07F1, 0x07F2, 0x07F3, 0x07F4, 0x07F5, 0x07F6, 0x07F7, 0x07F8,
    0x07F9, 0x07FA, 0x07FB, 0x07FC, 0x07FD, 0x07FE, 0x07FF, 0x0800, 0x0801, 0x0803, 0x0804, 0x0805, 0x0806,
    0x0807, 0x0808, 0x0809, 0x080A, 0x080B, 0x080C, 0x080D, 0x080E, 0x080F, 0x0810, 0x0811, 0x0812, 0x0813,
    0x0814, 0x0815, 0x0816, 0x0817, 0x0818, 0x0819, 0x081A, 0x081B, 0x081C, 0x081D, 0x081E, 0x081F, 0x0820,
    0x0821, 0x0822, 0x0823, 0x0824, 0x0825, 0x0826, 0x0827, 0x0828, 0x0829, 0x082A, 0x0830, 0x0831, 0x0832,
    0x0833,
];

/// `MI_HYDRAULICS` 0x8CD76C (the "hydralics" model index, a word set at model loading; owned by sa-stream).
const HYDRAULICS_MODEL: usize = 0x8CD76C;
/// `CTimer::ms_fTimeStep` 0xB7CB5C.
const TIME_STEP: usize = 0xB7CB5C;
/// `.rdata` constants: 0x858B38 0.02f, 0x858624 1.0f, 0x858C1C -1.0f, 0x858B50 0.0f, 0x8595EC 0.017453292f
const K_0_02: u32 = 0x3CA3_D70A;
const K_ONE: u32 = 0x3F80_0000;
const K_MINUS_ONE: u32 = 0xBF80_0000;
const K_ZERO: u32 = 0;
const K_DEG2RAD: u32 = 0x3C8E_FA35;
/// `CTheScripts::NumberOfIntroTextLinesThisFrame` (u16) 0xA44B68; the 0x44-byte intro text lines whose fields
/// the cases address as 0xA913E8 + 0x44 * n + offset.
const NUM_TEXT_LINES: usize = 0xA44B68;

orig! {
    /// `CVehicle::AddVehicleUpgrade(int32 modelId)` (0x6E3290)
    fn add_vehicle_upgrade(v: GPtr<u8>, m: i32) -> i32 = thiscall 0x6E3290 => unported;
    /// `CVehicle::RemoveVehicleUpgrade(int32 modelId)` (0x6DF930)
    fn remove_vehicle_upgrade(v: GPtr<u8>, m: i32) = thiscall 0x6DF930 => unported;
    /// `CRunningScript::PlayAnimScriptCommand(int32 command)` (0x470150; reads its own parameters)
    fn play_anim_script_command(s: Scr, command: u32) = thiscall 0x470150 => unported;
    /// `CRunningScript::ReadTextLabelFromScript(char*, uint8)` (0x463D50)
    fn read_label(s: Scr, buf: GPtr<u8>, len: u32) = thiscall 0x463D50 => crate::vm::read_text_label_from_script;
    /// `CStreamedScripts::GetProperIndexFromIndexUsedByScript(int16)` (0x470810), AX
    fn get_proper_index(m: GPtr<u8>, i: u32) -> u16 = thiscall 0x470810 => unported;
    /// `CScriptsForBrains::AddNewStreamedScriptBrainForCodeUse(int16, const char*, int8)` (0x46A9C0)
    fn add_brain_for_code_use(m: GPtr<u8>, i: u32, name: GPtr<u8>, ty: u32) = thiscall 0x46A9C0 => unported;
    /// `Multiply3x3(CVector* out, const CMatrix*, const CVector*)` (0x59C790), returns out
    fn multiply3x3(out: GPtr<u32>, m: GPtr<u8>, v: GPtr<u32>) -> GPtr<u32> = cdecl 0x59C790 => unported;
    /// 0x411A00 (thiscall on a `CVector`, `const CVector*`: `operator+=`)
    fn vec_add_assign(v: GPtr<u32>, o: GPtr<u32>) = thiscall 0x411A00 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vec_normalise(v: GPtr<u32>) = thiscall 0x59C910 => unported;
    /// 0x40FE60 (cdecl `(CVector* out, const CVector* a, const CVector* b)`: out = a - b), returns out
    fn vec_sub(out: GPtr<u32>, a: GPtr<u32>, b: GPtr<u32>) -> GPtr<u32> = cdecl 0x40FE60 => unported;
    /// `CrossProduct(CVector* out, const CVector* a, const CVector* b)` (0x59C730), returns out
    fn cross_product(out: GPtr<u32>, a: GPtr<u32>, b: GPtr<u32>) -> GPtr<u32> = cdecl 0x59C730 => unported;
    /// 0x406DA0 (thiscall on a `CVector`: `SquaredMagnitude()`), ST0 (read as f64)
    fn vec_squared_magnitude(v: GPtr<u32>) -> f64 = thiscall 0x406DA0 => unported;
    /// 0x40FEF0 (thiscall on a `CVector`, float: `operator*=`); raw
    fn vec_mul_assign(v: GPtr<u32>, k: u32) = thiscall 0x40FEF0 => unported;
    /// `CPhysical::ApplyForce(CVector force, CVector point, bool)` (0x542B50); vectors by value, raw
    fn apply_force(p: GPtr<u8>, fx: u32, fy: u32, fz: u32, px: u32, py: u32, pz: u32, b: u32) = thiscall 0x542B50 => unported;
    /// `CPhysical::AddToMovingList()` (0x542800)
    fn add_to_moving_list(p: GPtr<u8>) = thiscall 0x542800 => unported;
    /// `CEntity` virtual slot 0x10 (`SetIsStatic(bool)`)
    fn set_is_static(e: GPtr<u8>, b: u32) = thiscall vslot 0x10 => unported;
    /// 0x45AFB0 (thiscall on a `CPhysical`, three floats; a `.HOODLUM` stub: `SetTurnSpeed`); raw
    fn set_turn_speed(p: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x45AFB0 => unported;
    /// 0x407800 (cdecl `(int32 model)`, AL: `CStreaming::IsObjectInCdImage`)
    fn is_object_in_cd_image(m: u32) -> u8 = cdecl 0x407800 => unported;
    /// `CShadows::RemoveOilInArea(float x1, float x2, float y1, float y2)` (0x7074F0); raw
    fn remove_oil_in_area(x1: u32, x2: u32, y1: u32, y2: u32) = cdecl 0x7074F0 => unported;
    /// `CRadar::SetBlipFriendly(int32, uint8)` (0x583EB0)
    fn set_blip_friendly(h: u32, b: u32) = cdecl 0x583EB0 => unported;
    /// `CPedIntelligence::GetTaskSwim()` (0x601070)
    fn get_task_swim(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x601070 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CRunningScript::GivePedScriptedTask(int32 pedHandle, CTask*, int32 opcode)` (0x465C20)
    fn give_ped_scripted_task(s: Scr, ped: u32, task: GPtr<u8>, command: u32) = thiscall 0x465C20 => unported;
    /// `CTaskSimpleSwim::CTaskSimpleSwim(const CVector*, CPed*)` (0x688930)
    fn task_swim_ctor(t: GPtr<u8>, pos: GPtr<u32>, p: u32) -> GPtr<u8> = thiscall 0x688930 => unported;
    /// `CTheScripts::GetActualScriptThingIndex(int32 ref, uint8 type)` (0x4839A0)
    fn get_actual_script_thing_index(r: u32, ty: u32) -> i32 = cdecl 0x4839A0 => unported;
    /// `CRunningScript::CollectNextParameterWithoutIncreasingPC()` (0x464250)
    fn collect_next_no_pc(s: Scr) -> u32 = thiscall 0x464250 => unported;
    /// `CDecisionMakerTypes::GetInstance()` (0x4684F0)
    fn dm_types_instance() -> GPtr<u8> = cdecl 0x4684F0 => unported;
    /// 0x6070F0 (thiscall on `CDecisionMakerTypes`, `(int32 src, int32 type, bool mission)`: copies a decision
    /// maker, returns the new index)
    fn dm_copy(t: GPtr<u8>, src: i32, ty: u32, mission: u32) -> u32 = thiscall 0x6070F0 => unported;
    /// `CTheScripts::GetNewUniqueScriptThingIndex(int32, uint8)` (0x483720)
    fn get_new_unique_script_thing_index(i: u32, ty: u32) -> u32 = cdecl 0x483720 => unported;
    /// `CTaskComplexDrivePointRoute::CTaskComplexDrivePointRoute(CVehicle*, const CPointRoute&, float, int32,
    /// int32, float, int32)` (0x6433E0); floats raw
    fn task_drive_point_route_ctor(t: GPtr<u8>, v: GPtr<u8>, route: u32, speed: u32, mode: u32, model: u32, r: u32, style: u32) -> GPtr<u8> = thiscall 0x6433E0 => unported;
    /// `CPedType::GetPedTypeAcquaintances(int32, ePedType)` (0x6089D0)
    fn get_ped_type_acquaintances(a: u32, t: u32) -> u32 = cdecl 0x6089D0 => unported;
    /// `CPedType::GetPedFlag(ePedType)` (0x608830)
    fn get_ped_flag(t: u32) -> u32 = cdecl 0x608830 => unported;
    /// `CTheZones::GetLevelFromPosition(const CVector*)` (0x572300)
    fn get_level_from_position(p: GPtr<u32>) -> u32 = cdecl 0x572300 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), ST0 (read as f64: exact); floats raw
    fn find_ground_z(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CObjectPool::GetAt(int32 slot)` (0x404870), ECX = the object pool
    fn object_pool_get_at(pool: GPtr<u8>, i: u32) -> GPtr<u8> = thiscall 0x404870 => unported;
    /// `CVector::Magnitude()` (0x4082C0), ST0 (an `fsqrt` result under PC=24: exact as f64)
    fn vector_magnitude(v: GPtr<u32>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    fn get_vehicle_appearance(v: GPtr<u8>) -> i32 = thiscall 0x6D1080 => unported;
    /// `CCheckpoints::UpdatePos(uint32 id, const CVector*)` (0x722900)
    fn checkpoints_update_pos(id: u32, p: GPtr<u32>) = cdecl 0x722900 => unported;
    /// `CVehicle::GetSpecialColModel()` (0x6DF3D0)
    fn get_special_col_model(v: GPtr<u8>) -> u8 = thiscall 0x6DF3D0 => unported;
    /// `CPedGroupMembership::GetLeader()` (0x5F69A0) / `CountMembersExcludingLeader()` (0x5F6AA0)
    fn group_get_leader(m: GPtr<u8>) -> u32 = thiscall 0x5F69A0 => unported;
    fn count_members_excluding_leader(m: GPtr<u8>) -> u32 = thiscall 0x5F6AA0 => unported;
    /// `CCarAI::TellCarToFollowOtherCar(CVehicle*, CVehicle*, float)` (0x41C960); raw
    fn tell_car_to_follow_other_car(a: GPtr<u8>, b: GPtr<u8>, r: u32) = cdecl 0x41C960 => unported;
    /// 0x43F9B0 (cdecl `(const char*, bool)`: `CEntryExitManager::SetEnabledByName`)
    fn set_entry_exit_enabled_by_name(name: GPtr<u8>, on: u32) = cdecl 0x43F9B0 => unported;
    /// `CText::Get(const char*)` (0x6A0050), ECX = `TheText` 0xC1B340 (result discarded by 07FC)
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// CRT `strncpy` (0x821F40)
    fn strncpy_(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
    /// `CGameLogic::IsCoopGameGoingOn()` (0x441390), AL
    fn is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
    /// `CTaskComplexSequence::CTaskComplexSequence()` (0x632BD0) / `AddTask(CTask*)` (0x632D10)
    fn task_sequence_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x632BD0 => unported;
    fn task_sequence_add(seq: GPtr<u8>, t: GPtr<u8>) = thiscall 0x632D10 => unported;
    /// `CTaskComplexGoToPointAndStandStill::CTaskComplexGoToPointAndStandStill(int32, const CVector&, float,
    /// float, bool, bool)` (0x668120); floats raw
    fn task_go_to_point_ctor(t: GPtr<u8>, ms: u32, pos: GPtr<u32>, r: u32, r2: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x668120 => unported;
    /// `CTaskSimpleSlideToCoord::CTaskSimpleSlideToCoord(const CVector&, float heading, float speed, const char*
    /// anim, const char* block, uint32 flags, float delta, bool, uint32 time)` (0x66C450); floats raw
    #[allow(clippy::too_many_arguments)]
    fn task_slide_anim_ctor(t: GPtr<u8>, pos: GPtr<u32>, h: u32, speed: u32, anim: GPtr<u8>, block: GPtr<u8>, flags: u32, delta: u32, b: u32, time: u32) -> GPtr<u8> = thiscall 0x66C450 => unported;
    /// `CDarkel::FindTotalPedsKilledByPlayer(int32)` (0x43D6E0)
    fn find_total_peds_killed_by_player(p: u32) -> u32 = cdecl 0x43D6E0 => unported;
    /// `CEntity::FindTriggerPointCoors(CVector* out, int32)` (0x533380), returns out
    fn find_trigger_point_coors(e: GPtr<u8>, out: GPtr<u32>, i: u32) -> GPtr<u32> = thiscall 0x533380 => unported;
    /// `CPathFind::FindParkingNodeInArea(CVector* out, float x6)` (0x4513F0), ECX = ThePaths 0x96F050, returns out
    fn find_parking_node_in_area(m: GPtr<u8>, out: GPtr<u32>, a: u32, b: u32, c: u32, d: u32, e: u32, f: u32) -> GPtr<u32> = thiscall 0x4513F0 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740)
    fn find_active_task_by_type(tm: GPtr<u8>, ty: u32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CPool<CVehicle>::GetRef(CVehicle*)` (0x424160)
    fn vehicle_get_ref(pool: GPtr<u8>, v: GPtr<u8>) -> u32 = thiscall 0x424160 => unported;
    /// 0x470100 (thiscall on a `CBoundingBox`, two `CVector`s by value: `SetMinMax(min, max)`); raw
    fn bbox_set_min_max(b: GPtr<u8>, x0: u32, y0: u32, z0: u32, x1: u32, y1: u32, z1: u32) = thiscall 0x470100 => unported;
    /// `CStuntJumpManager::AddOne(const CBoundingBox&, const CBoundingBox&, const CVector&, int32)` (0x49CB40)
    fn stunt_jump_add_one(a: GPtr<u8>, b: GPtr<u8>, cam: GPtr<u32>, reward: u32) = cdecl 0x49CB40 => unported;
    /// 0x4119D0 (cdecl `(CVector* out, const CVector*, float)`: `operator/`), returns out; raw
    fn vec_div(out: GPtr<u32>, v: GPtr<u32>, k: u32) -> GPtr<u32> = cdecl 0x4119D0 => unported;
    /// `CTrain::MarkSurroundingEntitiesForCollisionWithTrain(CVector, float, CEntity*, bool)` (0x6F6640); raw
    fn mark_surrounding_entities(x: u32, y: u32, z: u32, r: u32, e: GPtr<u8>, b: u32) = cdecl 0x6F6640 => unported;
    /// `CEntity` virtual slot 0x38 (`Teleport(CVector, bool)`); raw
    fn teleport(e: GPtr<u8>, x: u32, y: u32, z: u32, b: u32) = thiscall vslot 0x38 => unported;
    /// `CTheScripts::ClearSpaceForMissionEntity(const CVector&, CEntity*)` (0x486B00)
    fn clear_space_for_mission_entity(p: GPtr<u32>, e: GPtr<u8>) = cdecl 0x486B00 => unported;
    /// 0x470070 (thiscall on the ped, bool: `SetKindaStayInSamePlace`)
    fn set_kinda_stay_in_same_place(p: GPtr<u8>, b: u32) = thiscall 0x470070 => unported;
    /// 0x674930 (thiscall; the 0x30-byte follow-patrol-route task constructor `(int32 moveState, const route*,
    /// int32 mode, float, float)`); floats raw
    fn task_follow_patrol_route_ctor(t: GPtr<u8>, ms: u32, route: u32, mode: u32, a: u32, b: u32) -> GPtr<u8> = thiscall 0x674930 => unported;
    /// `CTaskManager::GetSimplestActiveTask()` (0x6819D0)
    fn get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTask` virtual slot 0x10 (`GetTaskType()`)
    fn task_get_type(t: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `CWorld::FindGroundZFor3DCoord(float, float, float, bool*, CEntity**)` (0x5696C0), ST0 (read as f64)
    fn find_ground_z_3d(x: u32, y: u32, z: u32, a: u32, b: u32) -> f64 = cdecl 0x5696C0 => unported;
    /// `CRGBA::CRGBA(uint8, uint8, uint8, uint8)` (0x7170C0), returns this
    fn rgba_ctor(c: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `CModelInfo::IsBoatModel` (0x4C5A70) / `IsPlaneModel` (0x4C5B30) / `IsHeliModel` (0x4C5B00), AL
    fn is_boat_model(m: u32) -> u8 = cdecl 0x4C5A70 => unported;
    fn is_plane_model(m: u32) -> u8 = cdecl 0x4C5B30 => unported;
    fn is_heli_model(m: u32) -> u8 = cdecl 0x4C5B00 => unported;
    /// `CTaskComplexPartnerGreet::CTaskComplexPartnerGreet(const char*, CPed*, bool, float, int32, CVector)`
    /// (0x684210); float and vector raw
    fn task_partner_greet_ctor(t: GPtr<u8>, name: u32, p: GPtr<u8>, lead: u32, d: u32, ty: u32, x: u32, y: u32, z: u32) -> GPtr<u8> = thiscall 0x684210 => unported;
    /// `CTheScripts::ScriptConnectLodsFunction(int32, int32)` (0x470A20)
    fn script_connect_lods(a: u32, b: u32) = cdecl 0x470A20 => unported;
    /// `CTheScripts::AddToListOfConnectedLodObjects(CObject*, CObject*)` (0x470980)
    fn add_connected_lod_objects(a: GPtr<u8>, b: GPtr<u8>) = cdecl 0x470980 => unported;
    /// `CTaskSimpleDie::CTaskSimpleDie(const char*, const char*, eAnimationFlags, float, float)` (0x62FA60); raw
    fn task_die_named_ctor(t: GPtr<u8>, anim: GPtr<u8>, block: GPtr<u8>, flags: u32, delta: u32, speed: u32) -> GPtr<u8> = thiscall 0x62FA60 => unported;
    /// 0x53FB70 (cdecl `(int32)`: `CPad::GetPad`)
    fn get_pad(i: u32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
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

/// A model operand: a negative value indexes `CTheScripts::UsedObjectArray` backwards.
#[inline]
fn model_operand(v: u32) -> u32 {
    if (v as i32) < 0 {
        GPtr::<u32>::from_addr(0xA44B88u32.wrapping_sub(v.wrapping_mul(0x1C)) as usize).read()
    } else {
        v
    }
}

/// The ordered swap of a (lo, hi) pair of raw floats: when `lo > hi` (`fcomp; test ah,0x41; jne`: only
/// Greater) `hi = fld/fstp lo`, `lo = hi` (raw).
#[inline]
fn order_pair(lo: &mut u32, hi: &mut u32) {
    if X87::from_f32(f(*lo)).fcom(X87::from_f32(f(*hi))) == Cmp::Greater {
        let l = *lo;
        *lo = *hi;
        *hi = fld_fst(f(l)).to_bits();
    }
}

/// A field of the current intro text line: `0xA913E8 + 0x44 * n + off`, `n = u16 [0xA44B68]`.
#[inline]
fn text_line(off: u32) -> GPtr<u8> {
    let n = u32::from(GPtr::<u16>::from_addr(NUM_TEXT_LINES).read());
    GPtr::from_addr((n * 0x44 + 0xA913E8 + off) as usize)
}

/// `CRunningScript::ProcessCommands2000To2099(int32 command)` (0x472310, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x472310, conv = thiscall, name = "CRunningScript::ProcessCommands2000To2099", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript")]
pub fn process_commands_2000_to_2099(s: Scr, command: u32) -> i8 {
    match command {
        // 07D1 07D2 07D4 07DC 07E2 07E3 07E9 07EA..07ED 07F4 0805 0807..0809 080C 080D 080F 0813 081B 0821 0824
        // 0832: the jump table sends them to 0x47436E (`xor al,al`: no operand read, return 0). gr: nop (07E9,
        // 080C, 080D, 0821: unimplemented, -1).
        0x07D1 | 0x07D2 | 0x07D4 | 0x07DC | 0x07E2 | 0x07E3 | 0x07E9..=0x07ED | 0x07F4 | 0x0805 => {}
        0x0807..=0x0809 | 0x080C | 0x080D | 0x080F | 0x0813 | 0x081B | 0x0821 | 0x0824 | 0x0832 => {}
        // 07D0 GET_CURRENT_DAY_OF_WEEK (0x47234E): P0 = (u8) `CClock::CurrentDay` (0xB7014E); store 1.
        0x07D0 => {
            set_sp(0, u32::from(GPtr::<u8>::from_addr(0xB7014E).read()));
            store(s, 1);
        }
        // 07D3 REGISTER_SCRIPT_BRAIN_FOR_CODE_USE (0x47236A): P0 = (i16)`StreamedScripts.GetProperIndex((u16)P0)`;
        // 8-byte label (zeroed 16-byte buffer); `ScriptsForBrains (0xA90CF0).AddNewStreamedScriptBrainForCodeUse(
        // (u16)P0, label, 3)`.
        0x07D3 => {
            collect(s, 1);
            let i = get_proper_index(GPtr::from_addr(0xA47B60), sp(0) & 0xFFFF);
            set_sp(0, i32::from(i as i16) as u32);
            let mut label = [0u8; 16];
            let lp = GPtr::from_raw(label.as_mut_ptr());
            read_label(s, lp, 8);
            add_brain_for_code_use(GPtr::from_addr(0xA90CF0), sp(0) & 0xFFFF, lp, 3);
        }
        // 07D5 APPLY_FORCE_TO_CAR (0x4723C6)
        0x07D5 => apply_force_to_car(s),
        // 07D6 IS_INT_LVAR_EQUAL_TO_INT_VAR (0x472510) / 07D7 IS_FLOAT_LVAR_EQUAL_TO_FLOAT_VAR (0x47253A): `a =
        // GetPointerToScriptVariable(1)`, `b = GetPointerToScriptVariable(2)`; flag = `*b == *a` (i32 / x87
        // ordered equality).
        0x07D6 | 0x07D7 => {
            let a = var_ptr(s, 1);
            let b = var_ptr(s, 2);
            let r = if command == 0x07D6 {
                b.read() == a.read()
            } else {
                X87::from_f32(f(b.read())).fcom(X87::from_f32(f(a.read()))) == Cmp::Equal
            };
            set_flag(s, r);
        }
        // 07DA ADD_TO_CAR_ROTATION_VELOCITY (0x472576) / 07DB SET_CAR_ROTATION_VELOCITY (0x472677)
        0x07DA | 0x07DB => car_rotation_velocity(s, command == 0x07DA),
        // 07DD SET_CHAR_SHOOT_RATE (0x472759): byte `PED(P0)+0x719` = (u8)P1.
        0x07DD => {
            collect(s, 2);
            let p = ped(sp(0));
            p.byte_add(0x719).write(sp(1) as u8);
        }
        // 07DE IS_MODEL_IN_CDIMAGE (0x472786): flag = `0x407800(model) != 0` (model rule).
        0x07DE => {
            collect(s, 1);
            set_flag(s, is_object_in_cd_image(model_operand(sp(0))) != 0);
        }
        // 07DF REMOVE_OIL_PUDDLES_IN_AREA (0x4727B4): (P0, P2) and (P1, P3) ordered; `CShadows::RemoveOilInArea(x1,
        // x2, y1, y2)`.
        0x07DF => {
            collect(s, 4);
            let (mut x1, mut y1, mut x2, mut y2) = (sp(0), sp(1), sp(2), sp(3));
            order_pair(&mut x1, &mut x2);
            order_pair(&mut y1, &mut y2);
            remove_oil_in_area(x1, x2, y1, y2);
        }
        // 07E0 SET_BLIP_AS_FRIENDLY (0x472845): `CRadar::SetBlipFriendly(P0, (u8)P1)`.
        0x07E0 => {
            collect(s, 2);
            set_blip_friendly(sp(0), sp(1) & 0xFF);
        }
        // 07E1 TASK_SWIM_TO_COORD (0x47286C): `P0 != -1`, `p = PED(P0)` non-null and `GetTaskSwim()` non-null ->
        // the target is written to `GetTaskSwim()+0x14` (called twice); else task (0x64) `CTaskSimpleSwim(&target,
        // nullptr)`, `GivePedScriptedTask(P0, ...)`.
        0x07E1 => {
            collect(s, 4);
            let p0 = sp(0);
            let mut target = [sp(1), sp(2), sp(3)];
            if p0 as i32 != -1 {
                let p = ped(p0);
                if !p.is_null() {
                    let intel = |p: GPtr<u8>| p.byte_add(0x47C).cast::<GPtr<u8>>().read();
                    if !get_task_swim(intel(p)).is_null() {
                        let t = get_task_swim(intel(p)).byte_add(0x14).cast::<u32>();
                        t.write(target[0]);
                        t.byte_add(4).write(target[1]);
                        t.byte_add(8).write(target[2]);
                        return 0;
                    }
                }
            }
            let tp = GPtr::from_raw(target.as_mut_ptr());
            let t = new_task(0x64, |t| task_swim_ctor(t, tp, 0));
            give_ped_scripted_task(s, p0, t, command);
        }
        // 07E4 GET_MODEL_DIMENSIONS (0x472932): `cm = ms_modelInfoPtrs[model]->m_pColModel (+0x14)` (no checks); P0..P2
        // = the bounding box min, P3..P5 = max; store 6.
        0x07E4 => {
            collect(s, 1);
            let m = model_operand(sp(0));
            let mi = GPtr::<GPtr<u8>>::from_addr(0xA9B0C8u32.wrapping_add(m.wrapping_mul(4)) as usize).read();
            let cm = mi.byte_add(0x14).cast::<GPtr<u32>>().read();
            for i in 0..6 {
                set_sp(i, cm.byte_add(4 * i as isize).read());
            }
            store(s, 6);
        }
        // 07E5 COPY_CHAR_DECISION_MAKER (0x4729A4) / 07E6 COPY_GROUP_DECISION_MAKER (0x472A32): `src = P0 == -1 ? -1 :
        // GetActualScriptThingIndex(P0, 7)`; `GetActualScriptThingIndex(CollectNextParameterWithoutIncreasingPC(),
        // 7)` (discarded); `dm = 0x6070F0(GetInstance(), src, 0 / 1, (u8)m_UsesMissionCleanup)`; P0 =
        // `GetNewUniqueScriptThingIndex(dm, 7)`; store 1; mission cleanup -> `AddEntityToList(P0, 9)`.
        0x07E5 | 0x07E6 => {
            collect(s, 1);
            let src = if spi(0) == -1 {
                -1
            } else {
                get_actual_script_thing_index(sp(0), 7)
            };
            let n = collect_next_no_pc(s);
            get_actual_script_thing_index(n, 7);
            let mission = u32::from(s.byte_add(crate::vm::USES_MISSION_CLEANUP).read());
            let ty = u32::from(command == 0x07E6);
            let dm = dm_copy(dm_types_instance(), src, ty, mission);
            set_sp(0, get_new_unique_script_thing_index(dm, 7));
            store(s, 1);
            if uses_mission_cleanup(s) {
                mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), spi(0), 9);
            }
        }
        // 07E7 TASK_DRIVE_POINT_ROUTE_ADVANCED (0x472ABF): `v = P1 < 0 ? 0 : VEH(P1)`; `model = P4 == 0 ? -1 : P4 ==
        // 1 ? 0x19F : P4`; task (0x2C) `CTaskComplexDrivePointRoute(v, route 0xC18D50, P2, P3, model, -1.0f, P5)`;
        // `GivePedScriptedTask(P0, ...)`.
        0x07E7 => {
            collect(s, 6);
            let p0 = sp(0);
            let v = if spi(1) < 0 { GPtr::NULL } else { veh(sp(1)) };
            let mut model = sp(4);
            let (p2, p3) = (sp(2), sp(3));
            if model == 0 {
                model = u32::MAX;
            } else if model == 1 {
                model = 0x19F;
            }
            let p5 = sp(5);
            let t = new_task(0x2C, |t| {
                task_drive_point_route_ctor(t, v, 0xC18D50, p2, p3, model, K_MINUS_ONE, p5)
            });
            give_ped_scripted_task(s, p0, t, command);
        }
        // 07E8 IS_RELATIONSHIP_SET (0x472B88): flag = `(GetPedTypeAcquaintances(P0, P1) & GetPedFlag(P2)) != 0`.
        0x07E8 => {
            collect(s, 3);
            let p2 = sp(2);
            let a = get_ped_type_acquaintances(sp(0), sp(1));
            let fl = get_ped_flag(p2);
            set_flag(s, a & fl != 0);
        }
        // 07EE SET_CAR_ALWAYS_CREATE_SKIDS (0x472BD0): byte `VEH(P0)+0x42D` bit 0 = P1 != 0.
        0x07EE => {
            collect(s, 2);
            let b = veh(sp(0)).byte_add(0x42D);
            let c = b.read();
            b.write(if sp(1) != 0 { c | 1 } else { c & 0xFE });
        }
        // 07EF GET_CITY_FROM_COORDS (0x472C1B): P0 = `CTheZones::GetLevelFromPosition(&(P0, P1, P2))`; store 1.
        0x07EF => {
            collect(s, 3);
            let mut p = [sp(0), sp(1), sp(2)];
            let v = get_level_from_position(GPtr::from_raw(p.as_mut_ptr()));
            set_sp(0, v);
            store(s, 1);
        }
        // 07F0 HAS_OBJECT_OF_TYPE_BEEN_SMASHED (0x472C63)
        0x07F0 => object_of_type_smashed(s),
        // 07F1 IS_PLAYER_PERFORMING_WHEELIE (0x472D93) / 07F2 IS_PLAYER_PERFORMING_STOPPIE (0x472DF2): `info =
        // Players[P0]`; flag = ped +0x46C bit 8 (in a vehicle) and `GetVehicleAppearance(ped+0x58C) == 2` and
        // `info+0x104` / `info+0x10C` != 0.
        0x07F1 | 0x07F2 => {
            collect(s, 1);
            let info = player_info(sp(0));
            let p = info.cast::<GPtr<u8>>().read();
            let mut r = false;
            if p.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0
                && get_vehicle_appearance(p.byte_add(0x58C).cast::<GPtr<u8>>().read()) == 2
            {
                let off = if command == 0x07F1 { 0x104 } else { 0x10C };
                r = info.byte_add(off).cast::<u32>().read() != 0;
            }
            set_flag(s, r);
        }
        // 07F3 SET_CHECKPOINT_COORDS (0x472E4A): `i = GetActualScriptThingIndex(P0, 3)`; `i >= 0` (no upper bound):
        // `e = [0xA44074 + 8 * i]`; non-null -> `CCheckpoints::UpdatePos(e+4, &(P1, P2, P3))`.
        0x07F3 => {
            collect(s, 4);
            let mut v = [sp(1), sp(2), sp(3)];
            let i = get_actual_script_thing_index(sp(0), 3);
            if i >= 0 {
                let e =
                    GPtr::<GPtr<u32>>::from_addr((i as u32).wrapping_mul(8).wrapping_add(0xA44074) as usize)
                        .read();
                if !e.is_null() {
                    checkpoints_update_pos(e.byte_add(4).read(), GPtr::from_raw(v.as_mut_ptr()));
                }
            }
        }
        // 07F5 CONTROL_CAR_HYDRAULICS (0x472EBC): `v = VEH(P0)`; `(i8)v+0x48B < 0` -> `GetSpecialColModel()`; `i =
        // (i8)v+0x48B`; `i > -1`: `0xC1CB78 + 0x28 * i` +0, +4, +8, +0xC = P1..P4 (`fld/fstp`; the index re-read
        // before each store).
        0x07F5 => {
            collect(s, 5);
            let v = veh(sp(0));
            let idx = v.byte_add(0x48B);
            if (idx.read() as i8) < 0 {
                get_special_col_model(v);
            }
            if (idx.read() as i8) > -1 {
                for k in 0..4u32 {
                    let val = fld_fst(spf(1 + k as i32));
                    let i = i32::from(idx.read() as i8) as u32;
                    let a = i.wrapping_mul(0x28).wrapping_add(0xC1CB78 + 4 * k);
                    GPtr::<f32>::from_addr(a as usize).write(val);
                }
            }
        }
        // 07F6 GET_GROUP_SIZE (0x472F57): `g = GetActualScriptThingIndex(P0, 8)`; `0 <= g < 8`: `m = 0xC09928 + g *
        // 0x2D4`; P0 = `m->GetLeader() != 0`, P1 = `m->CountMembersExcludingLeader()`; else P0 = P1 = 0; store 2.
        0x07F6 => {
            collect(s, 1);
            let g = get_actual_script_thing_index(sp(0), 8);
            if (0..8).contains(&g) {
                let m = GPtr::from_addr(0xC09928 + g as usize * 0x2D4);
                let l = u32::from(group_get_leader(m) != 0);
                let n = count_members_excluding_leader(m);
                set_sp(0, l);
                set_sp(1, n);
            } else {
                set_sp(0, 0);
                set_sp(1, 0);
            }
            store(s, 2);
        }
        // 07F7 SET_OBJECT_COLLISION_DAMAGE_EFFECT (0x472FD4): `o = OBJ(P0)`; P1 != 0: byte +0x145 non-zero ->
        // +0x144 = it, +0x145 = 0; else +0x144 non-zero -> +0x145 = it, +0x144 = 0.
        0x07F7 => {
            collect(s, 2);
            let o = obj(sp(0));
            let (from, to) = if sp(1) != 0 {
                (0x145, 0x144)
            } else {
                (0x144, 0x145)
            };
            let c = o.byte_add(from).read();
            if c != 0 {
                o.byte_add(to).write(c);
                o.byte_add(from).write(0);
            }
        }
        // 07F8 SET_CAR_FOLLOW_CAR (0x47303D): `CCarAI::TellCarToFollowOtherCar(VEH(P0), VEH(P1), P2)` (P1 looked up
        // first).
        0x07F8 => {
            collect(s, 3);
            let p2 = sp(2);
            let b = veh(sp(1));
            let a = veh(sp(0));
            tell_car_to_follow_other_car(a, b, p2);
        }
        // 07F9 PLAYER_ENTERED_QUARRY_CRANE (0x47307F): dword 0xB76898 = 4. 07FA PLAYER_ENTERED_LAS_VEGAS_CRANE
        // (0x473090): dword 0xB76898 = 3, byte 0x8D37D5 = 0.
        0x07F9 => GPtr::<u32>::from_addr(0xB76898).write(4),
        0x07FA => {
            GPtr::<u32>::from_addr(0xB76898).write(3);
            GPtr::<u8>::from_addr(0x8D37D5).write(0);
        }
        // 07FB SWITCH_ENTRY_EXIT (0x4730A8): 8-byte label (zeroed 16-byte buffer), `CollectParameters(1)`;
        // `0x43F9B0(label, P0 != 0)`.
        0x07FB => {
            let mut label = [0u8; 16];
            let lp = GPtr::from_raw(label.as_mut_ptr());
            read_label(s, lp, 8);
            collect(s, 1);
            set_entry_exit_enabled_by_name(lp, u32::from(sp(0) != 0));
        }
        // 07FC DISPLAY_TEXT_WITH_FLOAT (0x4730F4)
        0x07FC => display_text_with_float(s),
        // 07FD DOES_GROUP_EXIST (0x4731CE): flag = `0 <= GetActualScriptThingIndex(P0, 8) < 8`.
        0x07FD => {
            collect(s, 1);
            let g = get_actual_script_thing_index(sp(0), 8);
            set_flag(s, (0..8).contains(&g));
        }
        // 07FE GIVE_MELEE_ATTACK_TO_CHAR (0x47320D): `p = PED(SP0)` (no null check); if `(u8)p->m_nFightingStyle (+0x72D)
        // != SP1` (movzx vs the full dword): `+0x72D = (u8)SP1`, `+0x72E = 0`. Then the inner table 0x474520 on
        // `SP2 - 1` (unsigned, > 5 does nothing): 1 -> `+0x72E = 0`, 2..5 -> `|= 1, 2, 4, 8`, 6 -> `|= 0xF`.
        0x07FE => {
            collect(s, 3);
            let p = ped(sp(0));
            let style = p.byte_add(0x72D);
            let moves = p.byte_add(0x72E);
            if u32::from(style.read()) != sp(1) {
                style.write(sp(1) as u8);
                moves.write(0);
            }
            let k = sp(2).wrapping_sub(1);
            if k == 0 {
                moves.write(0);
            } else if k <= 5 {
                let bits = if k == 5 { 0xF } else { 1u8 << (k - 1) };
                moves.write(moves.read() | bits);
            }
        }
        // 07FF SET_CAR_HYDRAULICS (0x4732B1): `v = VEH(SP0)` (no null check); `SP1 != 0` ->
        // `v->AddVehicleUpgrade(word [0x8CD76C])` (result discarded), else `v->RemoveVehicleUpgrade(word [0x8CD76C])`
        0x07FF => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let m = i32::from(GPtr::<u16>::from_addr(HYDRAULICS_MODEL).read());
            if sp(1) != 0 {
                add_vehicle_upgrade(v, m);
            } else {
                remove_vehicle_upgrade(v, m);
            }
        }
        // 0800 IS_2PLAYER_GAME_GOING_ON (0x4732FE): flag = `CGameLogic::IsCoopGameGoingOn() != 0`.
        0x0800 => set_flag(s, is_coop_game_going_on() != 0),
        // 0801 GET_CAMERA_FOV (0x473328): P0 = `TheCamera.m_aCams[(u8)[0xB6F081]].m_fFOV` (0xB6F250 + 0x238 * i);
        // store 1.
        0x0801 => {
            let i = u32::from(GPtr::<u8>::from_addr(0xB6F081).read());
            set_sp(0, GPtr::<u32>::from_addr((0xB6F250 + i * 0x238) as usize).read());
            store(s, 1);
        }
        // 0803 DOES_CAR_HAVE_HYDRAULICS (0x473350): flag = `v+0x594 == 0 && v+0x38C & 0x20000`.
        0x0803 => {
            collect(s, 1);
            let v = veh(sp(0));
            let r = v.byte_add(0x594).cast::<u32>().read() == 0
                && v.byte_add(0x38C).cast::<u32>().read() & 0x2_0000 != 0;
            set_flag(s, r);
        }
        // 0804 TASK_CHAR_SLIDE_TO_COORD_AND_PLAY_ANIM (0x4733A6)
        0x0804 => task_char_slide_to_coord_and_play_anim(s, command),
        // 0806 GET_TOTAL_NUMBER_OF_PEDS_KILLED_BY_PLAYER (0x4735FD): P0 = `CDarkel::FindTotalPedsKilledByPlayer(P0)`.
        0x0806 => {
            collect(s, 1);
            let v = find_total_peds_killed_by_player(sp(0));
            set_sp(0, v);
            store(s, 1);
        }
        // 080A GET_LEVEL_DESIGN_COORDS_FOR_OBJECT (0x473617): `p = OBJ(P0)->FindTriggerPointCoors(&tmp, P1)`; P0..P2 =
        // *p; store 3.
        0x080A => {
            collect(s, 2);
            let o = obj(sp(0));
            let mut tmp = [0u32; 3];
            let p = find_trigger_point_coors(o, GPtr::from_raw(tmp.as_mut_ptr()), sp(1));
            let (x, y, z) = (p.read(), p.byte_add(4).read(), p.byte_add(8).read());
            set_sp(0, x);
            set_sp(1, y);
            set_sp(2, z);
            store(s, 3);
        }
        // 080B SAVE_TEXT_LABEL_TO_DEBUG_FILE (0x473661): the exe reads a 16-byte label (zeroed 17-byte buffer) and
        // drops it (gr: nop).
        0x080B => {
            let mut label = [0u8; 17];
            read_label(s, GPtr::from_raw(label.as_mut_ptr()), 0x10);
        }
        // 080E GET_CHAR_HIGHEST_PRIORITY_EVENT (0x473679): P0 = byte `PED(P0)->m_pIntelligence+0xD1`; store 1.
        0x080E => {
            collect(s, 1);
            let intel = ped(sp(0)).byte_add(0x47C).cast::<GPtr<u8>>().read();
            set_sp(0, u32::from(intel.byte_add(0xD1).read()));
            store(s, 1);
        }
        // 0810 GET_PARKING_NODE_IN_AREA (0x4736A6): per axis ordered (x: P0/P3, y: P1/P4, z: P2/P5); `p =
        // ThePaths.FindParkingNodeInArea(&tmp, x1, x2, y1, y2, z1, z2)`; P0..P2 = *p; store 3.
        0x0810 => {
            collect(s, 6);
            let (mut x1, mut y1, mut z1, mut x2, mut y2, mut z2) = (sp(0), sp(1), sp(2), sp(3), sp(4), sp(5));
            order_pair(&mut x1, &mut x2);
            order_pair(&mut y1, &mut y2);
            order_pair(&mut z1, &mut z2);
            let mut tmp = [0u32; 3];
            let p = find_parking_node_in_area(
                GPtr::from_addr(0x96F050),
                GPtr::from_raw(tmp.as_mut_ptr()),
                x1,
                x2,
                y1,
                y2,
                z1,
                z2,
            );
            let (x, y, z) = (p.read(), p.byte_add(4).read(), p.byte_add(8).read());
            set_sp(0, x);
            set_sp(1, y);
            set_sp(2, z);
            store(s, 3);
        }
        // 0811 GET_CAR_CHAR_IS_USING (0x47379B): in a vehicle (+0x46C bit 8): `v = p+0x58C`; else `t =
        // FindActiveTaskByType(0x2BD)` or `FindActiveTaskByType(0x2BC)`, `v = t+0xC`; P0 = v ? GetRef(v) : -1;
        // store 1.
        0x0811 => {
            collect(s, 1);
            let p = ped(sp(0));
            let v = if p.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0 {
                p.byte_add(0x58C).cast::<GPtr<u8>>().read()
            } else {
                let tm = |p: GPtr<u8>| p.byte_add(0x47C).cast::<GPtr<u8>>().read().byte_add(4);
                let mut t = find_active_task_by_type(tm(p), 0x2BD);
                if t.is_null() {
                    t = find_active_task_by_type(tm(p), 0x2BC);
                }
                if t.is_null() {
                    GPtr::NULL
                } else {
                    t.byte_add(0xC).cast::<GPtr<u8>>().read()
                }
            };
            let r = if v.is_null() {
                u32::MAX
            } else {
                vehicle_get_ref(pool(VEHICLE_POOL), v)
            };
            set_sp(0, r);
            store(s, 1);
        }
        // 0812 TASK_PLAY_ANIM_NON_INTERRUPTABLE (0x473831): `this->PlayAnimScriptCommand(command)` (EBP = command)
        0x0812 => play_anim_script_command(s, command),
        // 0814 ADD_STUNT_JUMP (0x473840)
        0x0814 => add_stunt_jump(s),
        // 0815 SET_OBJECT_COORDINATES_AND_VELOCITY (0x473A01)
        0x0815 => set_object_coordinates_and_velocity(s),
        // 0816 SET_CHAR_KINDA_STAY_IN_SAME_PLACE (0x473BF5): `0x470070(PED(P0), P1 != 0)`.
        0x0816 => {
            collect(s, 2);
            let p = ped(sp(0));
            set_kinda_stay_in_same_place(p, u32::from(sp(1) != 0));
        }
        // 0817 TASK_FOLLOW_PATROL_ROUTE (0x473C38): task (0x30) `0x674930(P1, route 0xC18DB8, P2, [0x86FD08],
        // [0x86FD0C])`; `GivePedScriptedTask(P0, ...)`.
        0x0817 => {
            collect(s, 3);
            let (p0, p1, p2) = (sp(0), sp(1), sp(2));
            let t = new_task(0x30, |t| {
                let a = GPtr::<u32>::from_addr(0x86FD08).read();
                let b = GPtr::<u32>::from_addr(0x86FD0C).read();
                task_follow_patrol_route_ctor(t, p1, 0xC18DB8, p2, a, b)
            });
            give_ped_scripted_task(s, p0, t, command);
        }
        // 0818 IS_CHAR_IN_AIR (0x473CB3): `p = PED(P0)`; +0x46C bit 9 -> 1; bit 0 -> 0; `+0x470` bit 4 or bit 31 ->
        // flag = `GetSimplestActiveTask()` non-null and its type == 0xCF (called twice); else 0.
        0x0818 => {
            collect(s, 1);
            let p = ped(sp(0));
            let st = p.byte_add(0x46C).cast::<u32>().read();
            let r = if st & 0x200 != 0 {
                true
            } else if st & 1 != 0 {
                false
            } else {
                let e = p.byte_add(0x470).cast::<u32>().read();
                if e & 0x10 == 0 && (e as i32) >= 0 {
                    false
                } else {
                    let tm = |p: GPtr<u8>| p.byte_add(0x47C).cast::<GPtr<u8>>().read().byte_add(4);
                    if get_simplest_active_task(tm(p)).is_null() {
                        false
                    } else {
                        task_get_type(get_simplest_active_task(tm(p))) == 0xCF
                    }
                }
            };
            set_flag(s, r);
        }
        // 0819 GET_CHAR_HEIGHT_ABOVE_GROUND (0x473D38): `pos` = the ped's position (copied); `g =
        // FindGroundZFor3DCoord(pos, nullptr, nullptr)`; P0 = `pos.z - g` (x87, `fstp`); store 1.
        0x0819 => {
            collect(s, 1);
            let p = ent_pos(ped(sp(0)));
            let (x, y, z) = (p.read(), p.byte_add(4).read(), p.byte_add(8).read());
            let g = X87::from_f64_exact(find_ground_z_3d(x, y, z, 0, 0));
            set_sp(0, X87::from_f32(f(z)).fsub(g).to_f32().to_bits());
            store(s, 1);
        }
        // 081A SET_CHAR_WEAPON_SKILL (0x473DA3): byte `PED(P0)+0x72C` = (u8)P1.
        0x081A => {
            collect(s, 2);
            let p = ped(sp(0));
            p.byte_add(0x72C).write(sp(1) as u8);
        }
        // 081C SET_TEXT_EDGE (0x473DD1): a stack `CRGBA(P1, P2, P3, P4)`; the current text line's bytes +0x1D..+0x20
        // = its r, g, b, a; +0x22 = (u8)P0.
        0x081C => {
            collect(s, 5);
            let mut c = [0u8; 4];
            let cp = rgba_ctor(
                GPtr::from_raw(c.as_mut_ptr()),
                sp(1) & 0xFF,
                sp(2) & 0xFF,
                sp(3) & 0xFF,
                sp(4) & 0xFF,
            );
            let (r, g, b, a) = (
                cp.read(),
                cp.byte_add(1).read(),
                cp.byte_add(2).read(),
                cp.byte_add(3).read(),
            );
            text_line(0x1D).write(r);
            text_line(0x1E).write(g);
            text_line(0x1F).write(b);
            text_line(0x20).write(a);
            text_line(0x22).write(sp(0) as u8);
        }
        // 081D SET_CAR_ENGINE_BROKEN (0x473E51): P1 != 0: `+0x42D |= 2`, `+0x428 &= 0xEF`; else `+0x42D &= 0xFD`.
        0x081D => {
            collect(s, 2);
            let v = veh(sp(0));
            let b = v.byte_add(0x42D);
            if sp(1) != 0 {
                let (d, c) = (b.read() | 2, v.byte_add(0x428).read() & 0xEF);
                b.write(d);
                v.byte_add(0x428).write(c);
            } else {
                b.write(b.read() & 0xFD);
            }
        }
        // 081E IS_THIS_MODEL_A_BOAT (0x473EA8) / 081F ..._A_PLANE (0x473EE1) / 0820 ..._A_HELI (0x473F19): flag =
        // `CModelInfo::IsBoatModel / IsPlaneModel / IsHeliModel(P0) != 0`.
        0x081E..=0x0820 => {
            collect(s, 1);
            let m = sp(0);
            let r = if command == 0x081E {
                is_boat_model(m)
            } else if command == 0x081F {
                is_plane_model(m)
            } else {
                is_heli_model(m)
            };
            set_flag(s, r != 0);
        }
        // 0822 SET_FIRST_PERSON_IN_CAR_CAMERA_MODE (0x473F52): P0 != 0 -> byte 0xB6F06D = 0; else byte 0xB6F06D =
        // 1, dword 0xB6F0DC = 2.
        0x0822 => {
            collect(s, 1);
            if sp(0) != 0 {
                GPtr::<u8>::from_addr(0xB6F06D).write(0);
            } else {
                GPtr::<u8>::from_addr(0xB6F06D).write(1);
                GPtr::<u32>::from_addr(0xB6F0DC).write(2);
            }
        }
        // 0823 TASK_GREET_PARTNER (0x473F8A)
        0x0823 => task_greet_partner(s, command),
        // 0825 SET_HELI_BLADES_FULL_SPEED (0x4740DC): subtype (+0x594) 3: +0x84C = 0.22f; 4: +0x9C4 = 0.18f.
        0x0825 => {
            collect(s, 1);
            let v = veh(sp(0));
            let st = v.byte_add(0x594).cast::<u32>().read();
            if st == 3 {
                v.byte_add(0x84C).cast::<u32>().write(0x3E61_47AE);
            } else if st == 4 {
                v.byte_add(0x9C4).cast::<u32>().write(0x3E38_51EC);
            }
        }
        // 0826 DISPLAY_HUD (0x474128): `CTheScripts::bDisplayHud` (0xA444A0) = P0 != 0.
        0x0826 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0xA444A0).write(u8::from(sp(0) != 0));
        }
        // 0827 CONNECT_LODS (0x474148): `a = OBJ(P0)`, `b = OBJ(P1)`; `ScriptConnectLodsFunction(P0, P1)`;
        // `AddToListOfConnectedLodObjects(a, b)`.
        0x0827 => {
            collect(s, 2);
            let (h0, h1) = (sp(0), sp(1));
            let a = obj(h0);
            let b = obj(h1);
            script_connect_lods(h0, h1);
            add_connected_lod_objects(a, b);
        }
        // 0828 SET_MAX_FIRE_GENERATIONS (0x47418F): dword 0xB728E0 = P0.
        0x0828 => {
            collect(s, 1);
            GPtr::<u32>::from_addr(0xB728E0).write(sp(0));
        }
        // 0829 TASK_DIE_NAMED_ANIM (0x4741AB): ped = P0; 24-byte anim and 16-byte block labels (zeroed 25/17-byte
        // buffers); `CollectParameters(2)`: delta P0, flag P1; task (0x28) `CTaskSimpleDie(anim, block, P1 ? 0xD0 :
        // 0x10, P0, 1.0f)`; `GivePedScriptedTask(ped, ...)`.
        0x0829 => {
            collect(s, 1);
            let p = sp(0);
            let mut anim = [0u8; 25];
            let mut block = [0u8; 17];
            let ap = GPtr::from_raw(anim.as_mut_ptr());
            let bp = GPtr::from_raw(block.as_mut_ptr());
            read_label(s, ap, 0x18);
            read_label(s, bp, 0x10);
            collect(s, 2);
            let delta = sp(0);
            let flags = if sp(1) != 0 { 0xD0 } else { 0x10 };
            let t = new_task(0x28, |t| task_die_named_ctor(t, ap, bp, flags, delta, K_ONE));
            give_ped_scripted_task(s, p, t, command);
        }
        // 082A SET_PLAYER_DUCK_BUTTON (0x474261): byte `CPad::GetPad(P0)+0x11A` = P1 != 0 ? 0 : 1.
        0x082A => {
            collect(s, 2);
            let on = sp(1) != 0;
            get_pad(sp(0)).byte_add(0x11A).write(u8::from(!on));
        }
        // 0830 SET_POOL_TABLE_COORDS (0x4742AC): `CWorld::SnookerTableMax` (0x8CDEF4..) = (P3, P4, P5), `Min`
        // (0x8CDF00..) = (P0, P1, P2) (raw; store order max.z, max.x, max.y, min.z, min.x, min.y).
        0x0830 => {
            collect(s, 6);
            let w = |a: usize, v: u32| GPtr::<u32>::from_addr(a).write(v);
            w(0x8CDEFC, sp(5));
            w(0x8CDEF4, sp(3));
            w(0x8CDEF8, sp(4));
            w(0x8CDF08, sp(2));
            w(0x8CDF00, sp(0));
            w(0x8CDF04, sp(1));
        }
        // 0831 IS_AUDIO_BUILD (0x474315): `UpdateCompareFlag(0)` (gr: nop).
        0x0831 => flag(s, 0),
        // 0833 HAS_OBJECT_BEEN_PHOTOGRAPHED (0x474322): `o = OBJ(P0)`; flag = `o && o+0x140 & 0x1000`, the bit
        // cleared when set.
        0x0833 => {
            collect(s, 1);
            let o = obj(sp(0));
            let mut r = false;
            if !o.is_null() {
                let fl = o.byte_add(0x140).cast::<u32>();
                let v = fl.read();
                if v & 0x1000 != 0 {
                    r = true;
                    fl.write(v & 0xFFFF_EFFF);
                }
            }
            set_flag(s, r);
        }
        _ => return -1,
    }
    0
}

/// 07D5 APPLY_FORCE_TO_CAR (0x4723C6): `CollectParameters(7)`: vehicle P0, force F = (P1, P2, P3), point P = (P4,
/// P5, P6). `v = VEH(P0)`; `com = v+0xA4`; `P += Multiply3x3(m, com)` (0x411A00); `n = F`, `n.Normalise()`; `r =
/// P - Multiply3x3(m, com)` (0x40FE60); `c = CrossProduct(r, n)`; `k = 1 / (c.SquaredMagnitude() / turnMass (+0x90) +
/// 1 / mass (+0x8C))` (x87, `fstp` f32); `F *= k` (0x40FEF0); `v->ApplyForce(F, P, true)`. The out vectors are
/// zeroed locals.
fn apply_force_to_car(s: Scr) {
    collect(s, 7);
    let v = veh(sp(0));
    let mut fv = [sp(1), sp(2), sp(3)];
    let mut p = [sp(4), sp(5), sp(6)];
    let fp = GPtr::from_raw(fv.as_mut_ptr());
    let pp = GPtr::from_raw(p.as_mut_ptr());
    let com = v.byte_add(0xA4).cast::<u32>();
    let m = |v: GPtr<u8>| v.byte_add(0x14).cast::<GPtr<u8>>().read();
    let mut t1 = [0u32; 3];
    let a = multiply3x3(GPtr::from_raw(t1.as_mut_ptr()), m(v), com);
    vec_add_assign(pp, a);
    let mut n = fv;
    let np = GPtr::from_raw(n.as_mut_ptr());
    vec_normalise(np);
    let mut t2 = [0u32; 3];
    let b = multiply3x3(GPtr::from_raw(t2.as_mut_ptr()), m(v), com);
    let mut t3 = [0u32; 3];
    let r = vec_sub(GPtr::from_raw(t3.as_mut_ptr()), pp, b);
    let mut t4 = [0u32; 3];
    let c = cross_product(GPtr::from_raw(t4.as_mut_ptr()), r, np);
    let sq = X87::from_f64_exact(vec_squared_magnitude(c));
    let tm = v.byte_add(0x90).cast::<f32>().read();
    let mass = v.byte_add(0x8C).cast::<f32>().read();
    let k = sq
        .fdiv_m(tm)
        .fadd(X87::from_f32(f(K_ONE)).fdiv_m(mass))
        .fdivr_m(f(K_ONE))
        .to_f32();
    vec_mul_assign(fp, k.to_bits());
    apply_force(v, fv[0], fv[1], fv[2], p[0], p[1], p[2], 1);
}

/// 07DA ADD_TO_CAR_ROTATION_VELOCITY (0x472576) / 07DB SET_CAR_ROTATION_VELOCITY (0x472677): `v = VEH(P0)`.
/// - 07DA: `w = (P1, P2, P3) * 0.02f` (each `fld; fmul; fstp`), `cur = v->m_vecTurnSpeed (+0x50)` (copied first);
///   `d = Multiply3x3(m, &w)`; `new = d + cur` (per component, f32); static (`+0x1C & 4`) -> `SetIsStatic(false)`,
///   `AddToMovingList()`; +0x50 = new.
/// - 07DB: `w = ts * (P1, P2, P3)` (`fld ts; fmul P`); `d = Multiply3x3(m, &w)`; static -> as above;
///   `0x45AFB0(v, d.x * 0.02f, d.y * 0.02f, d.z * 0.02f)` (SetTurnSpeed).
fn car_rotation_velocity(s: Scr, add: bool) {
    collect(s, 4);
    let v = veh(sp(0));
    let ts = GPtr::<f32>::from_addr(TIME_STEP);
    let comp = |i: i32| {
        if add {
            X87::from_f32(spf(i)).fmul_m(f(K_0_02)).to_f32().to_bits()
        } else {
            X87::from_f32(ts.read()).fmul_m(spf(i)).to_f32().to_bits()
        }
    };
    let ws = v.byte_add(0x50).cast::<u32>();
    let x = comp(1);
    let cur = [ws.read(), ws.byte_add(4).read(), ws.byte_add(8).read()];
    let mut w = [x, comp(2), comp(3)];
    let mut out = [0u32; 3];
    let d = multiply3x3(
        GPtr::from_raw(out.as_mut_ptr()),
        v.byte_add(0x14).cast::<GPtr<u8>>().read(),
        GPtr::from_raw(w.as_mut_ptr()),
    );
    let d = [d.read(), d.byte_add(4).read(), d.byte_add(8).read()];
    let new: [u32; 3] = core::array::from_fn(|i| X87::from_f32(f(d[i])).fadd_m(f(cur[i])).to_f32().to_bits());
    if v.byte_add(0x1C).read() & 4 != 0 {
        set_is_static(v, 0);
        add_to_moving_list(v);
    }
    if add {
        ws.write(new[0]);
        ws.byte_add(4).write(new[1]);
        ws.byte_add(8).write(new[2]);
    } else {
        let k: [u32; 3] =
            core::array::from_fn(|i| X87::from_f32(f(d[i])).fmul_m(f(K_0_02)).to_f32().to_bits());
        set_turn_speed(v, k[0], k[1], k[2]);
    }
}

/// 07F0 HAS_OBJECT_OF_TYPE_BEEN_SMASHED (0x472C63): `CollectParameters(5)`: centre P0..P2 (z: ground when `z <=
/// -100`, `fstp` f32), radius P3, model P4 (UsedObjectArray rule). The object pool (loaded once) from the top slot
/// down while not found (0x404870): `o` with `+0x140 & 0x400` and `(i16)model == model`: `d = Magnitude(centre -
/// pos)` (0x40FE60); `d <= radius` (`test ah,0x41; jp`: Less or Equal) -> found.
fn object_of_type_smashed(s: Scr) {
    collect(s, 5);
    let (x, y) = (sp(0), sp(1));
    let z = if X87::from_f32(spf(2))
        .fcom(X87::from_f32(f(K_MINUS_100)))
        .test_parity(0x41)
    {
        fld_fst(spf(2))
    } else {
        X87::from_f64_exact(find_ground_z(x, y)).to_f32()
    };
    let mut centre = [x, y, z.to_bits()];
    let radius = spf(3);
    let model = model_operand(sp(4)) as i32;
    let op = pool(OBJECT_POOL);
    let mut n = op.byte_add(8).cast::<u32>().read();
    let mut found = false;
    while n != 0 {
        n -= 1;
        if found {
            break;
        }
        let o = object_pool_get_at(op, n);
        if o.is_null()
            || o.byte_add(0x140).cast::<u32>().read() & 0x400 == 0
            || i32::from(o.byte_add(0x22).cast::<i16>().read()) != model
        {
            continue;
        }
        let mut out = [0u32; 3];
        let d = vec_sub(
            GPtr::from_raw(out.as_mut_ptr()),
            GPtr::from_raw(centre.as_mut_ptr()),
            ent_pos(o),
        );
        let mut dd = [d.read(), d.byte_add(4).read(), d.byte_add(8).read()];
        let mag = X87::from_f64_exact(vector_magnitude(GPtr::from_raw(dd.as_mut_ptr())));
        if !mag.fcom(X87::from_f32(radius)).test_parity(0x41) {
            found = true;
        }
    }
    set_flag(s, found);
}

/// 07FC DISPLAY_TEXT_WITH_FLOAT (0x4730F4): `CollectParameters(2)` (x, y); 8-byte label (zeroed 16-byte buffer);
/// `TheText.Get(label)` (discarded); line `n`: +0x2C = P0, +0x30 = P1 (raw). `CollectParameters(2)`: value P0
/// (f32), precision P1; `i = _ftol(P0)`; `p = (float)P1` (`fild; fstp`); `strncpy(line+0x34, label, 8)`; +0x3C = i;
/// +0x40 = `_ftol(_CIpow(10.0, p) * (P0 - (float)i))` (x87); `n += 1`.
fn display_text_with_float(s: Scr) {
    collect(s, 2);
    let mut label = [0u8; 16];
    let lp = GPtr::from_raw(label.as_mut_ptr());
    read_label(s, lp, 8);
    text_get(GPtr::from_addr(0xC1B340), lp);
    text_line(0x2C).cast::<u32>().write(sp(0));
    text_line(0x30).cast::<u32>().write(sp(1));
    collect(s, 2);
    let v = spf(0);
    let i = X87::from_f32(v).ftol() as u32;
    let prec = X87::from_i32(spi(1)).to_f32();
    strncpy_(text_line(0x34), lp, 8);
    let line = text_line(0);
    line.byte_add(0x3C).cast::<u32>().write(i);
    let pow = X87::from_f64_exact(sa_core::orig::x87_binary(0x822130, 10.0, f64::from(prec)));
    let frac = X87::from_i32(i as i32).fsubr_m(v);
    let r = pow.fmul(frac).ftol() as u32;
    line.byte_add(0x40).cast::<u32>().write(r);
    let n = GPtr::<u16>::from_addr(NUM_TEXT_LINES);
    n.write(n.read().wrapping_add(1));
}

/// 0804 TASK_CHAR_SLIDE_TO_COORD_AND_PLAY_ANIM (0x4733A6): `CollectParameters(6)`: ped P0, x/y/z P1..P3 (raw),
/// heading P4 (deg), speed P5. `h = P4 * 0.017453292`; `speed = fld/fst P5`, `P5 < 0` -> 0.1f; 24-byte anim and
/// 16-byte block labels (zeroed 25/17-byte buffers); `CollectParameters(6)`: delta Q0, loop Q1, lockX Q2, lockY Q3,
/// keep-last Q4 (inverted), time Q5. `flags = (Q1 != 0 || Q5 > 0) ? 0x12 : 0x10`, `| 0x40` (Q2), `| 0x80` (Q3),
/// `| 8` (Q4 == 0); `b = [0x8D2E98] >= 0` (a stack byte, passed 0/1). `seq` (0x40) + `GoToPointAndStandStill(4,
/// vec, [0x86FC84], [0x86FC88], 0, 0)` (0x28) + `SlideToCoord(vec, h, speed, anim, block, flags, Q0, b, Q5 > 0 ?
/// Q5 : -1)` (0x80); seq not null-checked; `GivePedScriptedTask(P0, seq, command)`.
fn task_char_slide_to_coord_and_play_anim(s: Scr, command: u32) {
    collect(s, 6);
    let h = X87::from_f32(spf(4)).fmul_m(f(K_DEG2RAD)).to_f32().to_bits();
    let p0 = sp(0);
    let mut speed = fld_fst(spf(5)).to_bits();
    if X87::from_f32(spf(5)).fcom(X87::from_f32(f(K_ZERO))) == Cmp::Less {
        speed = 0x3DCC_CCCD;
    }
    let mut vec = [sp(1), sp(2), sp(3)];
    let mut anim = [0u8; 25];
    let mut block = [0u8; 17];
    let ap = GPtr::from_raw(anim.as_mut_ptr());
    let bp = GPtr::from_raw(block.as_mut_ptr());
    read_label(s, ap, 0x18);
    read_label(s, bp, 0x10);
    collect(s, 6);
    let delta = sp(0);
    let time = spi(5);
    let mut flags: u32 = if sp(1) != 0 || time > 0 { 0x12 } else { 0x10 };
    if sp(2) != 0 {
        flags |= 0x40;
    }
    if sp(3) != 0 {
        flags |= 0x80;
    }
    if sp(4) == 0 {
        flags |= 8;
    }
    let b = u32::from(GPtr::<i32>::from_addr(0x8D2E98).read() >= 0);
    let vp = GPtr::from_raw(vec.as_mut_ptr());
    let seq = new_task(0x40, task_sequence_ctor);
    let t1 = new_task(0x28, |t| {
        let a = GPtr::<u32>::from_addr(0x86FC84).read();
        let c = GPtr::<u32>::from_addr(0x86FC88).read();
        task_go_to_point_ctor(t, 4, vp, a, c, 0, 0)
    });
    task_sequence_add(seq, t1);
    let tm = if time > 0 { time as u32 } else { u32::MAX };
    let t2 = new_task(0x80, |t| {
        task_slide_anim_ctor(t, vp, h, speed, ap, bp, flags, delta, b, tm)
    });
    task_sequence_add(seq, t2);
    give_ped_scripted_task(s, p0, seq, command);
}

/// 0814 ADD_STUNT_JUMP (0x473840): `CollectParameters(16)`. Box 1: max = (P3 + P0, P4 + P1, P5 + P2), min = (P0 -
/// P3, P1 - P4, P2 - P5) (each one x87 add/sub of two f32 values, stored f32); `SetMinMax(&box1, min, max)`. Box 2
/// the same with P6..P11. `CStuntJumpManager::AddOne(&box1, &box2, &(P12, P13, P14), P15)`. The boxes are zeroed
/// 0x18-byte locals.
fn add_stunt_jump(s: Scr) {
    collect(s, 16);
    let mut b1 = [0u32; 6];
    let mut b2 = [0u32; 6];
    for (k, b) in [&mut b1, &mut b2].into_iter().enumerate() {
        let o = 6 * k as i32;
        let add = |a: i32, c: i32| X87::from_f32(spf(o + a)).fadd_m(spf(o + c)).to_f32().to_bits();
        let sub = |a: i32, c: i32| X87::from_f32(spf(o + a)).fsub_m(spf(o + c)).to_f32().to_bits();
        let (mx, my, mz) = (add(3, 0), add(4, 1), add(5, 2));
        let (nx, ny, nz) = (sub(0, 3), sub(1, 4), sub(2, 5));
        bbox_set_min_max(GPtr::from_raw(b.as_mut_ptr()).cast(), nx, ny, nz, mx, my, mz);
    }
    let mut cam = [sp(12), sp(13), sp(14)];
    stunt_jump_add_one(
        GPtr::from_raw(b1.as_mut_ptr()).cast(),
        GPtr::from_raw(b2.as_mut_ptr()).cast(),
        GPtr::from_raw(cam.as_mut_ptr()),
        sp(15),
    );
}

/// 0815 SET_OBJECT_COORDINATES_AND_VELOCITY (0x473A01): `CollectParameters(4)`; `o = OBJ(P0)`; `T = (P1, P2,
/// z)`, `z = P3` (raw) or `(float)FindGroundZForCoord(P1, P2)` when `P3 <= -100`. `+0x44 = 0x4119D0(&tmp, T - pos
/// (0x40FE60), ts)`; each of +0x44/+0x48/+0x4C: `< -1` (`test ah,5`) -> -1, then `> 1` (`test ah,0x41`) -> 1.
/// `MarkSurroundingEntitiesForCollisionWithTrain(T, 25.0f, o, false)`; `o->Teleport(T, false)` (slot 0x38);
/// `ClearSpaceForMissionEntity(&T, o)`.
fn set_object_coordinates_and_velocity(s: Scr) {
    collect(s, 4);
    let o = obj(sp(0));
    let (x, y) = (sp(1), sp(2));
    let mut z = sp(3);
    if !X87::from_f32(f(z))
        .fcom(X87::from_f32(f(K_MINUS_100)))
        .test_parity(0x41)
    {
        z = X87::from_f64_exact(find_ground_z(x, y)).to_f32().to_bits();
    }
    let pos = ent_pos(o);
    let ts = GPtr::<u32>::from_addr(TIME_STEP).read();
    let mut t = [x, y, z];
    let mut d = [0u32; 3];
    let dp = vec_sub(
        GPtr::from_raw(d.as_mut_ptr()),
        GPtr::from_raw(t.as_mut_ptr()),
        pos,
    );
    let mut q = [0u32; 3];
    let vel = vec_div(GPtr::from_raw(q.as_mut_ptr()), dp, ts);
    let ms = o.byte_add(0x44).cast::<u32>();
    let (vx, vy, vz) = (vel.read(), vel.byte_add(4).read(), vel.byte_add(8).read());
    ms.write(vx);
    ms.byte_add(4).write(vy);
    ms.byte_add(8).write(vz);
    for i in 0..3isize {
        let c = ms.byte_add(4 * i);
        if X87::from_f32(f(c.read())).fcom(X87::from_f32(f(K_MINUS_ONE))) == Cmp::Less {
            c.write(K_MINUS_ONE);
        }
        if X87::from_f32(f(c.read())).fcom(X87::from_f32(f(K_ONE))) == Cmp::Greater {
            c.write(K_ONE);
        }
    }
    mark_surrounding_entities(x, y, z, 0x41C8_0000, o, 0);
    teleport(o, x, y, z, 0);
    let mut tt = [x, y, z];
    clear_space_for_mission_entity(GPtr::from_raw(tt.as_mut_ptr()), o);
}

/// 0823 TASK_GREET_PARTNER (0x473F8A): `a = PED(P0)`, `b = PED(P1)`; task (0x78) `CTaskComplexPartnerGreet(
/// 0x859E8C, b, 1, P2, P3, CVector(0, 0, 0))` -> `GivePedScriptedTask(P0, ...)`; task (0x78) `(0x859E8C, a, 0, P2,
/// P3, 0)` -> `GivePedScriptedTask(P1, ...)` (P0/P1/P3 re-read from ScriptParams as compiled).
fn task_greet_partner(s: Scr, command: u32) {
    collect(s, 4);
    let a = ped(sp(0));
    let b = ped(sp(1));
    let p2 = sp(2);
    let t1 = new_task(0x78, |t| {
        task_partner_greet_ctor(t, 0x859E8C, b, 1, p2, sp(3), 0, 0, 0)
    });
    give_ped_scripted_task(s, sp(0), t1, command);
    let t2 = new_task(0x78, |t| {
        task_partner_greet_ctor(t, 0x859E8C, a, 0, p2, sp(3), 0, 0, 0)
    });
    give_ped_scripted_task(s, sp(1), t2, command);
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_2000_TO_2099];
