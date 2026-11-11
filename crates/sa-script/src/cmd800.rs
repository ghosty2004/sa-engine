//! `CRunningScript::ProcessCommands800To899` (0x481300): complete port (PORT-SCRIPT-2 partial, completed in
//! PORT-SCRIPT-3). Every jump-table case is in [`PORTED`]; the other opcodes of the range reach the
//! original default (the common `~CMatrix` tail, then -1). Cases follow the 1.0 US bytes and
//! `docs/re/specs/SCRIPT/commands/ProcessCommands800To899.md`.
//!
//! Divergence notes: the 8-byte text labels of 033E, 0360, 0361 and 036D are read into a zeroed 16-byte
//! buffer (the original's `[esp+0xE8]`/`[esp+0xF4]` is not cleared; an 8-character key would be followed by
//! stale stack bytes in `CText::Get` / `GetGarageNumberByName`; 033E's `strncpy(.., 8)` copy is the same
//! either way). 034D/034E use a zeroed local in place of the function-scope `CMatrix` (`SetRotateZ` and
//! `operator=` write every field the callees read). 034D's `GetHeading` result is the FPATAN value at 53
//! bits (project-wide FPATAN model). The stack objects of 0321 (`CPedDamageResponseCalculator`,
//! `CEventDamage`) start zeroed; the SEH state stores around them are not ported.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x0321, 0x0323, 0x0325, 0x0326, 0x0327, 0x0329, 0x032A, 0x032B, 0x032C, 0x0330, 0x0331, 0x0332, 0x0335,
    0x0337, 0x0338, 0x0339, 0x033E, 0x033F, 0x0340, 0x0341, 0x0342, 0x0343, 0x0344, 0x0345, 0x0348, 0x0349,
    0x034D, 0x034E, 0x034F, 0x0350, 0x0356, 0x035C, 0x035D, 0x035F, 0x0360, 0x0361, 0x0362, 0x0363, 0x0364, 0x0366, 0x036A,
    0x036D, 0x0373, 0x0376, 0x037E, 0x0381, 0x0382, 0x0383,
];

/// `gFireManager` 0xB71F80.
const FIRE_MANAGER: usize = 0xB71F80;
/// `CTimer::m_snTimeInMilliseconds` 0xB7CB84.
const TIME_IN_MILLISECONDS: usize = 0xB7CB84;
/// `TheCamera.m_nActiveCam` (byte) 0xB6F081 and `TheCamera.m_aCams[0].m_nMode` (u16, stride 0x238) 0xB6F1A8.
const ACTIVE_CAM: usize = 0xB6F081;
const CAM_MODES: usize = 0xB6F1A8;
/// The player-visibility byte FindPlayerPed's SET_CHAR_VISIBLE writes (0x8CC380).
const PLAYER_VISIBLE: usize = 0x8CC380;
/// `TheText` (`CText`) 0xC1B340.
const THE_TEXT: usize = 0xC1B340;
/// `CPopulation::ms_nTotalMissionPeds` 0xC0EC24.
const TOTAL_MISSION_PEDS: usize = 0xC0EC24;
/// `CWorld::PlayerInFocus` (byte) 0xB7CD74.
const PLAYER_IN_FOCUS: usize = 0xB7CD74;
/// `CGarages::aGarages` (`CGarage`, 0xD8 bytes each) 0x96C048.
const GARAGES: usize = 0x96C048;
/// `CTheScripts::InvisibilitySettingArray` (20 `CEntity*`) 0xA449E0.
const INVISIBILITY_SETTING_ARRAY: usize = 0xA449E0;
/// `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8.
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CTheScripts::UsedObjectArray[0].nModelIndex` as the negative-model lookups address it
/// (`0xA44B88 + (-id)*0x1C`).
const USED_OBJECT_MODEL: usize = 0xA44B88;
/// The immediate START_CAR_FIRE / START_CHAR_FIRE push as the fire strength: 0.8f (`push 0x3F4CCCCD`).
const K_0_8: u32 = 0x3F4C_CCCD;
/// The immediate EXPLODE_CHAR_HEAD pushes as the damage: 1000.0f (`push 0x447A0000`).
const K_1000: u32 = 0x447A_0000;
/// `.rdata` 0x858B8C: 0.5f
const K_0_5: u32 = 0x3F00_0000;
/// `.rdata` 0x858624: 1.0f
const K_ONE: u32 = 0x3F80_0000;
/// `.rdata` 0x858628: 100.0f
const K_100: u32 = 0x42C8_0000;
/// `.rdata` 0x858B50: 0.0f
const K_ZERO: u32 = 0;
/// `.rdata` 0x859878: 57.295776f (180 / pi)
const K_RAD_TO_DEG: u32 = 0x4265_2EE0;
/// `.rdata` 0x859E2C: 360.0f
const K_360: u32 = 0x43B4_0000;
/// `.rdata` 0x8595EC: 0.017453292f (pi / 180)
const K_DEG_TO_RAD: u32 = 0x3C8E_FA35;
/// `.rdata` 0x858B38: 0.02f
const K_0_02: u32 = 0x3CA3_D70A;

/// `CGarages::RespraysAreFree` (byte) 0x96C009.
const RESPRAYS_ARE_FREE: usize = 0x96C009;
/// `CTheScripts::NumberOfIntroTextLinesThisFrame` (u16) 0xA44B68.
const NUM_INTRO_TEXT_LINES: usize = 0xA44B68;
/// `CTheScripts::IntroTextLines` (`tScriptText`, 0x44 bytes each) 0xA913E8.
const INTRO_TEXT_LINES: u32 = 0xA913E8;

/// `.rdata` 0x859014: -100.0f
const K_MINUS_100: u32 = 0xC2C8_0000;

orig! {
    /// `CPed::SetStayInSamePlace(bool)` (0x481090)
    fn set_stay_in_same_place(p: GPtr<u8>, stay: u32) = thiscall 0x481090 => unported;
    /// Unnamed `CObject` method returning AL (0x46A2F0, a `.HOODLUM` stub; HAS_OBJECT_BEEN_DAMAGED's callee)
    fn object_has_been_damaged(o: GPtr<u8>) -> u8 = thiscall 0x46A2F0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), floats as raw dwords, ST0 (read as f64: exact)
    fn find_ground_z_for_coord(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CPedIntelligence::FlushImmediately(bool)` (0x601640)
    fn flush_immediately(i: GPtr<u8>, set_primary_default: u32) = thiscall 0x601640 => unported;
    /// `CEntity::GetDistanceFromCentreOfMassToBaseOfModel()` (0x536BE0), ST0
    fn get_distance_from_centre_of_mass_to_base_of_model(e: GPtr<u8>) -> f64 = thiscall 0x536BE0 => unported;
    /// `CEntity::Teleport(CVector, bool)` (vtable slot 0x38); the by-value vector as three raw dwords
    fn teleport(e: GPtr<u8>, x: u32, y: u32, z: u32, reset_rotation: u32) = thiscall vslot 0x38 => unported;
    /// `CTheScripts::ClearSpaceForMissionEntity(const CVector&, CEntity*)` (0x486B00)
    fn clear_space_for_mission_entity(pos: GPtr<u32>, e: GPtr<u8>) = cdecl 0x486B00 => unported;
    /// `CTaskSimpleCarSetPedInAsDriver::CTaskSimpleCarSetPedInAsDriver(CVehicle*, CTaskUtilityLineUpPedWithCar*)` (0x6470E0)
    fn set_ped_in_as_driver_ctor(t: GPtr<u8>, veh: GPtr<u8>, line_up: GPtr<u8>) -> GPtr<u8> = thiscall 0x6470E0 => unported;
    /// `CTaskSimpleCarSetPedInAsDriver::ProcessPed(CPed*)` (0x64B950)
    fn set_ped_in_as_driver_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x64B950 => unported;
    /// `CTaskSimpleCarSetPedInAsDriver::~CTaskSimpleCarSetPedInAsDriver()` (0x647170)
    fn set_ped_in_as_driver_dtor(t: GPtr<u8>) = thiscall 0x647170 => unported;
    /// `CCamera::SetCameraDirectlyBehindForFollowPed_CamOnAString()` (0x50BD40)
    fn set_camera_directly_behind_for_follow_ped_cam_on_a_string(cam: GPtr<u8>) = thiscall 0x50BD40 => unported;
    /// `CPedDamageResponseCalculator::CPedDamageResponseCalculator(CEntity*, float, eWeaponType, ePedPieceTypes, bool)` (0x4AD3F0)
    fn damage_calc_ctor(c: GPtr<u8>, source: u32, damage: u32, weapon: u32, piece: u32, speak: u32) -> GPtr<u8> = thiscall 0x4AD3F0 => unported;
    /// `CPedDamageResponseCalculator::~CPedDamageResponseCalculator()` (0x4AD420)
    fn damage_calc_dtor(c: GPtr<u8>) = thiscall 0x4AD420 => unported;
    /// `CPedDamageResponseCalculator::ComputeDamageResponse(CPed*, CPedDamageResponse&, bool)` (0x4B5AC0)
    fn compute_damage_response(c: GPtr<u8>, ped: GPtr<u8>, resp: GPtr<u8>, speak: u32) = thiscall 0x4B5AC0 => unported;
    /// `CEventDamage::CEventDamage(CEntity*, uint32 time, eWeaponType, ePedPieceTypes, uint8 direction,
    /// bool, bool pedInVehicle)` (0x4AD830)
    fn event_damage_ctor(e: GPtr<u8>, source: u32, time: u32, weapon: u32, piece: u32, dir: u32, a: u32, in_vehicle: u32) -> GPtr<u8> = thiscall 0x4AD830 => unported;
    /// `CEventDamage::~CEventDamage()` (0x4AD960)
    fn event_damage_dtor(e: GPtr<u8>) = thiscall 0x4AD960 => unported;
    /// `CEventDamage::AffectsPed(CPed*)` (0x4B35A0), AL
    fn event_damage_affects_ped(e: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x4B35A0 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, unk: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CFireManager::StartScriptFire(const CVector&, CEntity*, float, uint8, int8, int32)` (0x53A270)
    fn start_script_fire(m: GPtr<u8>, pos: GPtr<u32>, e: GPtr<u8>, strength: u32, a: u32, b: u32, c: u32) -> i32 = thiscall 0x53A270 => unported;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    fn get_vehicle_appearance(v: GPtr<u8>) -> i32 = thiscall 0x6D1080 => unported;
    /// `CVehicle::CanBeDeleted()` (0x6D1180), AL
    fn vehicle_can_be_deleted(v: GPtr<u8>) -> u8 = thiscall 0x6D1180 => unported;
    /// `CPlaceable::IsWithinArea(float x1, float y1, float x2, float y2)` (0x54F200), AL; raw dwords
    fn is_within_area(e: GPtr<u8>, x1: u32, y1: u32, x2: u32, y2: u32) -> u8 = thiscall 0x54F200 => unported;
    /// `CPool<CVehicle>::GetRef(CVehicle*)` (0x424160), ECX = the vehicle pool
    fn vehicle_get_ref(pool: GPtr<u8>, v: GPtr<u8>) -> i32 = thiscall 0x424160 => unported;
    /// `CVehicle::SetVehicleCreatedBy(int32)` (0x6D5D70)
    fn set_vehicle_created_by(v: GPtr<u8>, by: u32) = thiscall 0x6D5D70 => unported;
    /// `CCamera::SetZoomValueFollowPedScript(int16)` (0x50C160)
    fn set_zoom_value_follow_ped_script(cam: GPtr<u8>, z: u32) = thiscall 0x50C160 => unported;
    /// `CCamera::SetZoomValueCamStringScript(int16)` (0x50C1B0)
    fn set_zoom_value_cam_string_script(cam: GPtr<u8>, z: u32) = thiscall 0x50C1B0 => unported;
    /// `CPickups::GetActualPickupIndex(int32)` (0x4552A0); the result is discarded here
    fn get_actual_pickup_index(h: i32) -> i32 = cdecl 0x4552A0 => unported;
    /// `CPickups::GenerateNewOne(CVector, uint32 model, uint8 type, uint32 ammo, uint32 money, bool, char*)`
    /// (0x456F20); the vector by value (three raw dwords)
    fn generate_new_pickup(x: u32, y: u32, z: u32, model: u32, ty: u32, ammo: u32, money: u32, empty: u32, msg: u32) -> i32 = cdecl 0x456F20 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CWorld::FindObjectsIntersectingCube(const CVector&, const CVector&, int16*, int16, CEntity**, bool x5)` (0x568DD0)
    fn find_objects_intersecting_cube(
        a: GPtr<u32>, b: GPtr<u32>, count: GPtr<i16>, max: u32, out: GPtr<u8>, bb: u32, bv: u32, bp: u32, bo: u32, bd: u32
    ) = cdecl 0x568DD0 => unported;
    /// `CText::Get(const char* key)` (0x6A0050)
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CRGBA::CRGBA(uint8, uint8, uint8, uint8)` (0x7170C0), returns this
    fn rgba_ctor(c: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `CTheScripts::RemoveThisPed(CPed*)` (0x486240)
    fn remove_this_ped(p: GPtr<u8>) = cdecl 0x486240 => unported;
    /// `CPed::SetCharCreatedBy(uint8)` (0x5E47E0)
    fn set_char_created_by(p: GPtr<u8>, by: u32) = thiscall 0x5E47E0 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80)
    fn get_peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedGroupMembership::IsMember(const CPed*)` (0x5F69E0), AL
    fn membership_is_member(m: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x5F69E0 => unported;
    /// `CPedGroupMembership::RemoveMember(CPed*)` (0x5FB210)
    fn membership_remove_member(m: GPtr<u8>, p: GPtr<u8>) = thiscall 0x5FB210 => unported;
    /// `CExplosion::TestForExplosionInArea(eExplosionType, float x1, x2, y1, y2, z1, z2)` (0x736950), AL; raw dwords
    fn test_for_explosion_in_area(ty: u32, x1: u32, x2: u32, y1: u32, y2: u32, z1: u32, z2: u32) -> u8 = cdecl 0x736950 => unported;
    /// `CPhysical::PlacePhysicalRelativeToOtherPhysical(CPhysical*, CPhysical*, CVector)` (0x546DB0)
    fn place_physical_relative_to_other_physical(base: GPtr<u8>, e: GPtr<u8>, x: u32, y: u32, z: u32) = cdecl 0x546DB0 => unported;
    /// `CObject::SetObjectTargettable(bool)` (0x59F300)
    fn set_object_targettable(o: GPtr<u8>, on: u32) = thiscall 0x59F300 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0), AL
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CWorld::FindPlayerSlotWithPedPointer(void*)` (0x563FA0)
    fn find_player_slot_with_ped_pointer(p: GPtr<u8>) -> i32 = cdecl 0x563FA0 => unported;
    /// `CGarages::GetGarageNumberByName(const char*)` (0x447680), AX
    fn get_garage_number_by_name(name: GPtr<u8>) -> u16 = cdecl 0x447680 => unported;
    /// `CGarage::OpenThisGarage()` (0x447D50)
    fn open_this_garage(g: GPtr<u8>) = thiscall 0x447D50 => unported;
    /// `CGarage::CloseThisGarage()` (0x447D70)
    fn close_this_garage(g: GPtr<u8>) = thiscall 0x447D70 => unported;
    /// `CWorld::FindObjectsOfTypeInRange(uint32 model, const CVector&, float, bool, int16*, int16, CEntity**,
    /// bool x5)` (0x564C70); radius as a raw dword
    fn find_objects_of_type_in_range(
        model: u32, pos: GPtr<u32>, radius: u32, d2: u32, count: GPtr<i16>, max: u32, out: GPtr<u32>,
        bb: u32, bv: u32, bp: u32, bo: u32, bd: u32
    ) = cdecl 0x564C70 => unported;
    /// `CWorld::FindLodOfTypeInRange(uint32 model, const CVector&, float, bool, int16*, int16, CEntity**)` (0x564ED0)
    fn find_lod_of_type_in_range(
        model: u32, pos: GPtr<u32>, radius: u32, d2: u32, count: GPtr<i16>, max: u32, out: GPtr<u32>
    ) = cdecl 0x564ED0 => unported;
    /// `CTheScripts::AddToInvisibilitySwapArray(CEntity*, bool)` (0x481200)
    fn add_to_invisibility_swap_array(e: GPtr<u8>, visible: u32) = cdecl 0x481200 => unported;
    /// `CPed::OurPedCanSeeThisEntity(CEntity*, bool)` (0x5E1660), AL
    fn our_ped_can_see_this_entity(p: GPtr<u8>, e: GPtr<u8>, is_spotted: u32) -> u8 = thiscall 0x5E1660 => unported;
    /// `CMessages::AddBigMessageWithNumber(const char*, uint32 time, uint16 style, int32 n1..n6)` (0x69E5F0)
    fn add_big_message_with_number(
        t: GPtr<u8>, time: u32, style: u32, n1: u32, n2: u32, n3: u32, n4: u32, n5: u32, n6: u32
    ) = cdecl 0x69E5F0 => unported;
    /// `CPopulation::ChooseCivilianOccupation(bool, bool, int32, int32, int32, bool, bool, bool, char*)` (0x612F90)
    fn choose_civilian_occupation(a: u32, b: u32, c: i32, d: i32, e: i32, f: u32, g: u32, h: u32, i: u32) -> i32 = cdecl 0x612F90 => unported;
    /// `CPed::operator new(uint32)` (0x5E4720)
    fn ped_new(size: u32) -> GPtr<u8> = cdecl 0x5E4720 => unported;
    /// `CCivilianPed::CCivilianPed(ePedType, uint32 model)` (0x5DDB70), returns this
    fn civilian_ped_ctor(p: GPtr<u8>, ped_type: u32, model: u32) -> GPtr<u8> = thiscall 0x5DDB70 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskSimpleStandStill::CTaskSimpleStandStill(int32 time, bool looped, bool hold, float blend)` (0x62F310)
    fn task_simple_stand_still_ctor(t: GPtr<u8>, time: i32, looped: u32, hold: u32, blend: u32) -> GPtr<u8> = thiscall 0x62F310 => unported;
    /// `CTaskManager::SetTask(CTask*, int32 slot, bool)` (0x681AF0)
    fn task_manager_set_task(m: GPtr<u8>, task: GPtr<u8>, slot: i32, force: u32) = thiscall 0x681AF0 => unported;
    /// `CPlaceable::SetPosn(const CVector&)` (0x4241C0)
    fn set_posn_v(e: GPtr<u8>, pos: GPtr<u32>) = thiscall 0x4241C0 => unported;
    /// `CPlaceable::SetOrientation(float, float, float)` (0x439A80)
    fn set_orientation(e: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x439A80 => unported;
    /// `CPool<CPed>::GetRef(CPed*)` (0x4442D0), ECX = the ped pool
    fn ped_get_ref_(pool: GPtr<u8>, p: GPtr<u8>) -> i32 = thiscall 0x4442D0 => unported;
    /// `CPlaceable::GetHeading()` (0x441DB0): the raw FPATAN result in ST0, read as f64 (53 of its 64
    /// significand bits, the project-wide FPATAN model; the product below is rounded to 24 bits)
    fn get_heading(e: GPtr<u8>) -> f64 = thiscall 0x441DB0 => unported;
    /// `CPlaceable::SetHeading(float)` (0x43E0C0)
    fn set_heading(e: GPtr<u8>, h: u32) = thiscall 0x43E0C0 => unported;
    /// `CEntity::UpdateRwMatrix()` (0x446F90)
    fn update_rw_matrix(e: GPtr<u8>) = thiscall 0x446F90 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    fn update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CMatrix::SetRotateZ(float)` (0x59B0E0)
    fn matrix_set_rotate_z(m: GPtr<u8>, a: u32) = thiscall 0x59B0E0 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0)
    fn matrix_assign(m: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BBC0 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890): the result through the hidden pointer
    fn matrix_mul_vec(out: GPtr<u32>, m: GPtr<u8>, v: GPtr<u32>) -> GPtr<u32> = cdecl 0x59C890 => unported;
    /// `CWorld::FindObjectsIntersectingAngledCollisionBox(const CBox&, const CMatrix&, const CVector&, float
    /// minX, minY, maxX, maxY, int16*, int16, CEntity**, bool x5)` (0x568FF0); floats as raw dwords
    fn find_objects_intersecting_angled_collision_box(
        bbox: GPtr<u8>, m: GPtr<u8>, pos: GPtr<u32>, min_x: u32, min_y: u32, max_x: u32, max_y: u32,
        count: GPtr<i16>, max: u32, out: GPtr<u8>, bb: u32, bv: u32, bp: u32, bo: u32, bd: u32
    ) = cdecl 0x568FF0 => unported;
    /// `CPhysical::SetVelocity(CVector)`-style setter at 0x441130 (thiscall, the vector by value)
    fn set_velocity(e: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x441130 => unported;
}

/// An entity's position: `m_matrix (+0x14) ? &m_matrix->pos (+0x30) : &placement.pos (+0x04)`.
#[inline]
fn entity_pos(e: GPtr<u8>) -> GPtr<u32> {
    let m = e.byte_add(0x14).cast::<GPtr<u8>>().read();
    if m.is_null() { e.byte_add(0x4).cast() } else { m.byte_add(0x30).cast() }
}

/// `(b < a) ? b : a`, then the same against c and d (`fcomp; test ah,5; jp`: ordered `<`, ties and NaN keep
/// the earlier value); the values are raw dwords (`mov`), only the compares load them.
fn min4(a: u32, b: u32, c: u32, d: u32) -> u32 {
    let m = if f(b) < f(a) { b } else { a };
    let m = if f(c) < f(m) { c } else { m };
    if f(d) < f(m) { d } else { m }
}

/// As [`min4`] with `>` (`test ah,0x41; jne`).
fn max4(a: u32, b: u32, c: u32, d: u32) -> u32 {
    let m = if f(b) > f(a) { b } else { a };
    let m = if f(c) > f(m) { c } else { m };
    if f(d) > f(m) { d } else { m }
}

/// The obstacle test of 034D ROTATE_OBJECT (0x481E25..0x4822EF) and 034E SLIDE_OBJECT (0x48255C..0x482A26,
/// the same code): the corners of the object's col-model bounding box (min `n`, max `x`) A = (nx, ny, nz),
/// B = (xx, ny, xz), C = (xx, xy, nz), D = (nx, xy, xz) (as the stack copies compose them) are transformed by
/// `operator*(local, corner)` in that order; min/max over x and y ([`min4`]/[`max4`] in the order B vs A,
/// then C, then D); `FindObjectsIntersectingAngledCollisionBox(colModel, local, pos, minX, minY, maxX, maxY,
/// &count, 2, nullptr, false, true, true, false, false)`; true when `(int16)count > 0`. Model info:
/// `ms_modelInfoPtrs[(int16)obj +0x22]`, col model at +0x14 (bounding box at +0).
fn angled_box_occupied(obj: GPtr<u8>, local: GPtr<u8>, pos: GPtr<u32>) -> bool {
    let model = i32::from(obj.byte_add(0x22).cast::<i16>().read());
    let mi = GPtr::<GPtr<u8>>::from_addr(MODEL_INFO_PTRS.wrapping_add((model as u32).wrapping_mul(4) as usize)).read();
    let col = mi.byte_add(0x14).cast::<GPtr<u8>>().read();
    let b = |o: isize| col.byte_add(o).cast::<u32>().read();
    let (nx, ny, nz, xx, xy, xz) = (b(0), b(4), b(8), b(0xC), b(0x10), b(0x14));
    let corners = [[nx, ny, nz], [xx, ny, xz], [xx, xy, nz], [nx, xy, xz]];
    let mut t = [[0u32; 3]; 4];
    for (i, c) in corners.iter().enumerate() {
        let mut v = *c;
        let mut out = [0u32; 3];
        let r = matrix_mul_vec(GPtr::from_raw(out.as_mut_ptr()), local, GPtr::from_raw(v.as_mut_ptr()));
        t[i] = [r.read(), r.byte_add(4).read(), r.byte_add(8).read()];
    }
    let (a, bb, c, d) = (t[0], t[1], t[2], t[3]);
    let min_x = min4(a[0], bb[0], c[0], d[0]);
    let max_x = max4(a[0], bb[0], c[0], d[0]);
    let min_y = min4(a[1], bb[1], c[1], d[1]);
    let max_y = max4(a[1], bb[1], c[1], d[1]);
    let mut count: i16 = 0;
    find_objects_intersecting_angled_collision_box(
        col,
        local,
        pos,
        min_x,
        min_y,
        max_x,
        max_y,
        GPtr::from_raw(&mut count),
        2,
        GPtr::NULL,
        0,
        1,
        1,
        0,
        0,
    );
    count > 0
}

/// 034D ROTATE_OBJECT (0x481CA5): no null check. h = GetHeading * 57.295776 (+ 360 when < 0), kept in ST0.
/// h == P1 (ordered) → flag 1. Else cw = P1 - h (stored f32, + 360 when < 0), ccw = h - P1 (ST0, + 360 when
/// < 0); cw < ccw: new = cw < P2 ? P1 : P2 + h; else new = ccw < P2 ? P1 : h - P2 (all ordered `<`). With P3
/// != 0: local (a zeroed stand-in for the switch's function-scope CMatrix) `SetRotateZ(new * 0.017453292)`,
/// its pos += the object's position (`fld; fadd; fstp`); an obstacle ([`angled_box_occupied`]) → flag 1, no
/// rotation. Otherwise `SetHeading(new * 0.017453292)`, UpdateRwMatrix, UpdateRwFrame, flag = new == P1.
fn rotate_object(s: Scr) {
    collect(s, 4);
    let obj = object_at_ref(pool(OBJECT_POOL), spi(0));
    let mut h = X87::from_f64_exact(get_heading(obj)).fmul_m(f(K_RAD_TO_DEG));
    if h.fcom(X87::constant(K_ZERO)) == sa_math::x87::Cmp::Less {
        h = h.fadd_m(f(K_360));
    }
    let target = sp(1);
    let step = sp(2);
    if !h.fcom(X87::from_f32(f(target))).test_parity(0x44) {
        set_flag(s, true);
        return;
    }
    let mut cw = X87::from_f32(f(target)).fsub(h).to_f32();
    let mut ccw = X87::from_f32(f(target)).fsubr(h);
    if X87::from_f32(cw).fcom(X87::constant(K_ZERO)) == sa_math::x87::Cmp::Less {
        cw = X87::from_f32(cw).fadd_m(f(K_360)).to_f32();
    }
    if ccw.fcom(X87::constant(K_ZERO)) == sa_math::x87::Cmp::Less {
        ccw = ccw.fadd_m(f(K_360));
    }
    let new = if X87::from_f32(cw).fcom(ccw) == sa_math::x87::Cmp::Less {
        if X87::from_f32(cw).fcom(X87::from_f32(f(step))) == sa_math::x87::Cmp::Less {
            target
        } else {
            X87::from_f32(f(step)).fadd(h).to_f32().to_bits()
        }
    } else if ccw.fcom(X87::from_f32(f(step))) == sa_math::x87::Cmp::Less {
        target
    } else {
        h.fsub_m(f(step)).to_f32().to_bits()
    };
    if sp(3) != 0 {
        let p = entity_pos(obj);
        let mut pos = [p.read(), p.byte_add(4).read(), p.byte_add(8).read()];
        let mut m = [0u32; 0x12];
        let local = GPtr::from_raw(m.as_mut_ptr()).cast::<u8>();
        matrix_set_rotate_z(local, X87::from_f32(f(new)).fmul_m(f(K_DEG_TO_RAD)).to_f32().to_bits());
        for i in 0..3 {
            let c = local.byte_add(0x30 + 4 * i).cast::<f32>();
            c.write(X87::from_f32(c.read()).fadd_m(f(pos[i as usize])).to_f32());
        }
        if angled_box_occupied(obj, local, GPtr::from_raw(pos.as_mut_ptr())) {
            set_flag(s, true);
            return;
        }
    }
    set_heading(obj, X87::from_f32(f(new)).fmul_m(f(K_DEG_TO_RAD)).to_f32().to_bits());
    update_rw_matrix(obj);
    update_rw_frame(obj);
    let eq = !X87::from_f32(f(new)).fcom(X87::from_f32(f(target))).test_parity(0x44);
    set_flag(s, eq);
}

/// One axis of 034E: `d < 0` (`test ah,1`): `-d > step` → p + step (`fld step; fadd p`), else the target;
/// otherwise (d >= 0 or NaN): `d > step` → p - step, else the target. Ties and NaN snap to the target.
fn slide_axis(d: X87, p: u32, t: u32, step: u32) -> u32 {
    if d.fcom(X87::constant(K_ZERO)).c0() {
        if d.fchs().fcom(X87::from_f32(f(step))) == sa_math::x87::Cmp::Greater {
            X87::from_f32(f(step)).fadd_m(f(p)).to_f32().to_bits()
        } else {
            t
        }
    } else if d.fcom(X87::from_f32(f(step))) == sa_math::x87::Cmp::Greater {
        X87::from_f32(f(p)).fsub_m(f(step)).to_f32().to_bits()
    } else {
        t
    }
}

/// 034E SLIDE_OBJECT (0x482339): no null check. p = the object's position, t = (P1, P2, P3), step = (P4, P5,
/// P6). p == t (ordered, per axis) → flag 1. dx = p.x - t.x (ST0), dy, dz (stored f32); each axis moves by
/// [`slide_axis`]. With P7 != 0: local = `*obj->m_matrix` (`CMatrix::operator=` on a zeroed stand-in for the
/// function-scope CMatrix, no null check), local.pos = p; an obstacle ([`angled_box_occupied`]) → flag 1, no
/// move. Otherwise `Teleport(p, false)` (vslot 14) and flag = p == t.
fn slide_object(s: Scr) {
    collect(s, 8);
    let obj = object_at_ref(pool(OBJECT_POOL), spi(0));
    let src = entity_pos(obj);
    let mut p = [src.read(), src.byte_add(4).read(), src.byte_add(8).read()];
    let t = [sp(1), sp(2), sp(3)];
    let eq = |a: u32, b: u32| !X87::from_f32(f(a)).fcom(X87::from_f32(f(b))).test_parity(0x44);
    if eq(p[0], t[0]) && eq(p[1], t[1]) && eq(p[2], t[2]) {
        set_flag(s, true);
        return;
    }
    let dx = X87::from_f32(f(p[0])).fsub_m(f(t[0]));
    let dy = X87::from_f32(f(p[1])).fsub_m(f(t[1])).to_f32();
    let dz = X87::from_f32(f(p[2])).fsub_m(f(t[2])).to_f32();
    p[0] = slide_axis(dx, p[0], t[0], sp(4));
    p[1] = slide_axis(X87::from_f32(dy), p[1], t[1], sp(5));
    p[2] = slide_axis(X87::from_f32(dz), p[2], t[2], sp(6));
    if sp(7) != 0 {
        let mut m = [0u32; 0x12];
        let local = GPtr::from_raw(m.as_mut_ptr()).cast::<u8>();
        matrix_assign(local, obj.byte_add(0x14).cast::<GPtr<u8>>().read());
        for i in 0..3 {
            local.byte_add(0x30 + 4 * i).cast::<u32>().write(p[i as usize]);
        }
        if angled_box_occupied(obj, local, GPtr::from_raw(p.as_mut_ptr())) {
            set_flag(s, true);
            return;
        }
    }
    teleport(obj, p[0], p[1], p[2], 0);
    let b = eq(p[0], t[0]) && eq(p[1], t[1]) && eq(p[2], t[2]);
    set_flag(s, b);
}

/// 0325 START_CAR_FIRE (0x481479) / 0326 START_CHAR_FIRE (0x4814E4, shared tail 0x4814A3): no null check;
/// the entity's position copied to the stack; `P0 = gFireManager.StartScriptFire(&pos, e, 0.8, 1, 0, 1)`;
/// StoreParameters(1).
fn start_entity_fire(s: Scr, e: GPtr<u8>) {
    let p = entity_pos(e);
    let mut pos = [p.read(), p.byte_add(4).read(), p.byte_add(8).read()];
    let h = start_script_fire(GPtr::from_addr(FIRE_MANAGER), GPtr::from_raw(pos.as_mut_ptr()), e, K_0_8, 1, 0, 1);
    set_sp(0, h as u32);
    store(s, 1);
}

/// 0321 EXPLODE_CHAR_HEAD (0x48135B): no null check. A stack `CPedDamageResponseCalculator(nullptr, 1000.0,
/// 34, 9 /*head*/, false)` and `CEventDamage(nullptr, CTimer::m_snTimeInMilliseconds, 34, 9, 0, false,
/// (m_nPedFlags (+0x46C) >> 8) & 0xFFFFFF01)` (the exact dword the original pushes; the callee uses the
/// low byte); when `event.AffectsPed(ped)`: `calc.ComputeDamageResponse(ped, event.m_damageResponse
/// (+0x38), true)`, else `event +0x42 = 1`; `ped->m_pIntelligence (+0x47C)->m_eventGroup (+0x68).Add(&event,
/// false)`; the two destructors (event first). The SEH state stores around the objects are not ported.
fn explode_char_head(s: Scr) {
    collect(s, 1);
    let ped = ped_at_ref(pool(PED_POOL), spi(0));
    let mut calc = [0u32; 0x10];
    let c = GPtr::from_raw(calc.as_mut_ptr()).cast::<u8>();
    damage_calc_ctor(c, 0, K_1000, 0x22, 9, 0);
    let in_vehicle = (ped.byte_add(0x46C).cast::<u32>().read() >> 8) & 0xFFFF_FF01;
    let time = GPtr::<u32>::from_addr(TIME_IN_MILLISECONDS).read();
    let mut event = [0u32; 0x20];
    let e = GPtr::from_raw(event.as_mut_ptr()).cast::<u8>();
    event_damage_ctor(e, 0, time, 0x22, 9, 0, 0, in_vehicle);
    if event_damage_affects_ped(e, ped) != 0 {
        compute_damage_response(c, ped, e.byte_add(0x38), 1);
    } else {
        e.byte_add(0x42).write(1);
    }
    let intel = ped.byte_add(0x47C).cast::<GPtr<u8>>().read();
    event_group_add(intel.byte_add(0x68), e, 0);
    event_damage_dtor(e);
    damage_calc_dtor(c);
}

/// 0327 GET_RANDOM_CAR_OF_TYPE_IN_AREA (0x48150A): scans the vehicle pool from the top slot down until a
/// handle is found: a used slot (flag byte >= 0) whose vehicle (non-null) has `GetVehicleAppearance()`
/// 1 or 2 (called twice when not 1), +0x428 bit 0 clear, model (int16 +0x22) == P4 or P4 < 0,
/// `CanBeDeleted()` and `IsWithinArea(P0, P1, P2, P3)`: handle = GetRef, `SetVehicleCreatedBy(2)`, mission
/// cleanup (type 1). P0 = handle (-1 when none), StoreParameters(1).
fn get_random_car_of_type_in_area(s: Scr) {
    collect(s, 5);
    let mut h: i32 = -1;
    let mut n = pool(VEHICLE_POOL).byte_add(8).cast::<i32>().read();
    if n != 0 {
        let mut off = n.wrapping_mul(0xA18);
        loop {
            off = off.wrapping_sub(0xA18);
            n = n.wrapping_sub(1);
            if h != -1 {
                break;
            }
            let p = pool(VEHICLE_POOL);
            let flags = p.byte_add(4).cast::<GPtr<i8>>().read();
            if flags.byte_add(n as isize).read() >= 0 {
                let v = p.cast::<GPtr<u8>>().read().byte_add(off as isize);
                if !v.is_null() {
                    let app_ok = get_vehicle_appearance(v) == 1 || get_vehicle_appearance(v) == 2;
                    if app_ok && v.byte_add(0x428).read() & 1 == 0 {
                        let model = i32::from(v.byte_add(0x22).cast::<i16>().read());
                        let want = spi(4);
                        if (model == want || want < 0)
                            && vehicle_can_be_deleted(v) != 0
                            && is_within_area(v, sp(0), sp(1), sp(2), sp(3)) != 0
                        {
                            h = vehicle_get_ref(pool(VEHICLE_POOL), v);
                            set_vehicle_created_by(v, 2);
                            if uses_mission_cleanup(s) {
                                mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), h, 1);
                            }
                        }
                    }
                }
            }
            if n == 0 {
                break;
            }
        }
    }
    set_sp(0, h as u32);
    store(s, 1);
}

/// 032B CREATE_PICKUP_WITH_AMMO (0x481678): model P0 (< 0: used-object model), type P1 (byte), ammo P2,
/// x/y raw P3/P4, `fld P5; fst z; fcomp -100.0`: z = FindGroundZForCoord(x, y) + 0.5 (x87) when `z <= -100`
/// (ordered); the discarded `GetActualPickupIndex(CollectNextParameterWithoutIncreasingPC())`;
/// `P0 = GenerateNewOne((x, y, z), model, type, ammo, 0, false, nullptr)`; StoreParameters(1).
fn create_pickup_with_ammo(s: Scr) {
    collect(s, 6);
    let mut model = spi(0);
    if model < 0 {
        model = GPtr::<i32>::from_addr(
            USED_OBJECT_MODEL.wrapping_add(model.wrapping_neg().wrapping_mul(0x1C) as u32 as usize),
        )
        .read();
    }
    let st = X87::from_f32(f(sp(5)));
    let (x, y) = (sp(3), sp(4));
    let mut z = st.to_f32().to_bits();
    if !st.fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
        z = X87::from_f64_exact(find_ground_z_for_coord(x, y))
            .fadd_m(f(K_0_5))
            .to_f32()
            .to_bits();
    }
    get_actual_pickup_index(collect_next_parameter_without_increasing_pc(s));
    let h = generate_new_pickup(x, y, z, model as u32, sp(1) & 0xFF, sp(2), 0, 0, 0);
    set_sp(0, h as u32);
    store(s, 1);
}

/// `fld a; fcomp b; test ah,0x41; jne keep`: the raw pair swapped when `a > b` (ordered; the `fld/fstp`
/// copy of a non-NaN value is exact).
#[inline]
fn order(a: u32, b: u32) -> (u32, u32) {
    if f(a) > f(b) { (b, a) } else { (a, b) }
}

/// 0339 IS_AREA_OCCUPIED (0x4818BB): corners (P0, P1, P2) / (P3, P4, P5), x and y ordered as raw dwords,
/// z kept on the x87 stack (both through `fld`, so a signalling NaN comes back quiet) and stored as
/// (min, max) when `z1 > z2` else (z1, z2); `FindObjectsIntersectingCube(&min, &max, &count, 2, nullptr,
/// P6 != 0, P7 != 0, P8 != 0, P9 != 0, P10 != 0)`; flag = count > 0 (shared tail 0x4819F2).
fn is_area_occupied(s: Scr) {
    collect(s, 11);
    let (x1, x2) = order(sp(0), sp(3));
    let (y1, y2) = order(sp(1), sp(4));
    let (z1, z2) = (fld_fst(spf(2)), fld_fst(spf(5)));
    let (zmin, zmax) = if z1 > z2 { (z2, z1) } else { (z1, z2) };
    let mut lo = [x1, y1, zmin.to_bits()];
    let mut hi = [x2, y2, zmax.to_bits()];
    let mut count: i16 = 0;
    let nz = |i: i32| u32::from(sp(i) != 0);
    find_objects_intersecting_cube(
        GPtr::from_raw(lo.as_mut_ptr()),
        GPtr::from_raw(hi.as_mut_ptr()),
        GPtr::from_raw(&mut count),
        2,
        GPtr::NULL,
        nz(6),
        nz(7),
        nz(8),
        nz(9),
        nz(10),
    );
    set_flag(s, count > 0);
}

/// 034F REMOVE_CHAR_ELEGANTLY (0x482A96): for a mission ped (`m_nCreatedBy` +0x484 == 2): in a vehicle
/// (+0x46C bit 8 and `m_pVehicle` +0x58C) `CTheScripts::RemoveThisPed(ped)`; otherwise
/// `SetCharCreatedBy(1)`, removal from its group's membership (+8) when a member, mission ped count - 1,
/// +0x470 bit 3 set, `CWorld::RemoveReferencesToDeletedObject(ped)`. Then, with mission cleanup,
/// `RemoveEntityFromList(P0, 2)` (also for a null or non-mission ped).
fn remove_char_elegantly(s: Scr) {
    collect(s, 1);
    let ped = ped_at_ref(pool(PED_POOL), spi(0));
    if !ped.is_null() && ped.byte_add(0x484).read() == 2 {
        if ped.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0
            && !ped.byte_add(0x58C).cast::<GPtr<u8>>().read().is_null()
        {
            remove_this_ped(ped);
        } else {
            set_char_created_by(ped, 1);
            let g = get_peds_group(ped);
            if !g.is_null() {
                let m = g.byte_add(8);
                if membership_is_member(m, ped) != 0 {
                    membership_remove_member(m, ped);
                }
            }
            let n = GPtr::<u32>::from_addr(TOTAL_MISSION_PEDS);
            n.write(n.read().wrapping_sub(1));
            let fl = ped.byte_add(0x470).cast::<u32>();
            fl.write(fl.read() | 8);
            world_remove_references_to_deleted_object(ped);
        }
    }
    if uses_mission_cleanup(s) {
        mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), spi(0), 2);
    }
}

/// 035D MAKE_OBJECT_TARGETTABLE (0x482CDB): no null check. P1 != 0: `SetObjectTargettable(true)`; else
/// `SetObjectTargettable(false)` and, when `Players[PlayerInFocus].m_pPed->m_pTargetedObject` (+0x71C) is
/// this object, `obj->CleanUpOldReference(&that)` (when non-null) and the field cleared.
fn make_object_targettable(s: Scr) {
    collect(s, 2);
    let obj = object_at_ref(pool(OBJECT_POOL), spi(0));
    let focus = u32::from(GPtr::<u8>::from_addr(PLAYER_IN_FOCUS).read());
    let ped = player_info(focus).cast::<GPtr<u8>>().read();
    if sp(1) != 0 {
        set_object_targettable(obj, 1);
        return;
    }
    set_object_targettable(obj, 0);
    let r = ped.byte_add(0x71C);
    let t = r.cast::<GPtr<u8>>().read();
    if obj.addr() == t.addr() {
        if !t.is_null() {
            clean_up_old_reference(t, r);
        }
        r.cast::<u32>().write(0);
    }
}

/// 035F ADD_ARMOUR_TO_CHAR (0x482D52): no null check. armour (+0x548) = `fild P1; fadd armour` (stored);
/// a player clamps it to `Players[FindPlayerSlotWithPedPointer(ped)].m_nMaxArmour` (byte +0x150, `fild`;
/// kept when max <= armour or unordered), any other ped to 100.0 (when armour >= 100 or unordered); then
/// armour <= 0 (or unordered) → 0.0. Each step is a store as compiled.
fn add_armour_to_char(s: Scr) {
    collect(s, 2);
    let ped = ped_at_ref(pool(PED_POOL), spi(0));
    let a = ped.byte_add(0x548).cast::<f32>();
    a.write(X87::from_i32(spi(1)).fadd_m(a.read()).to_f32());
    let v = if ped_is_player(ped) != 0 {
        let slot = find_player_slot_with_ped_pointer(ped);
        let max = i32::from(player_info(slot as u32).byte_add(0x150).read());
        let mx = X87::from_i32(max);
        if mx.fcom(X87::from_f32(a.read())).test_any(0x41) { mx } else { X87::from_f32(a.read()) }
    } else if X87::from_f32(a.read()).fcom(X87::constant(K_100)).test_parity(0x05) {
        X87::constant(K_100)
    } else {
        X87::from_f32(a.read())
    };
    a.write(v.to_f32());
    if X87::from_f32(a.read()).fcom(X87::constant(K_ZERO)).test_any(0x41) {
        a.write(f(K_ZERO));
    } else {
        a.write(fld_fst(a.read()));
    }
}

/// 0360 OPEN_GARAGE (0x482E1D) / 0361 CLOSE_GARAGE (0x482E60): `ReadTextLabelFromScript(buf, 8)` (zeroed
/// 16-byte buffer, see the module notes); `n = GetGarageNumberByName(buf)` (int16); n >= 0 →
/// `aGarages[n].OpenThisGarage()` / `CloseThisGarage()`.
fn open_close_garage(s: Scr, open: bool) {
    let mut buf = [0u8; 16];
    let b = GPtr::from_raw(buf.as_mut_ptr());
    read_text_label(s, b, 8);
    let n = get_garage_number_by_name(b) as i16;
    if n >= 0 {
        let g = GPtr::from_addr(GARAGES.wrapping_add((i32::from(n) * 0xD8) as usize));
        if open { open_this_garage(g) } else { close_this_garage(g) }
    }
}

/// 0363 SET_VISIBILITY_OF_CLOSEST_OBJECT_OF_TYPE (0x482F84): x/y raw P0/P1, z = P2 or the ground z when
/// `P2 <= -100` (ordered); best = `fld radius; fadd st,st` (stored f32); model P4 (< 0: used-object
/// model); `FindObjectsOfTypeInRange(model, &pos, radius, true, &count, 16, list, true, false, false, true,
/// true)`, when none `FindLodOfTypeInRange(model, &pos, radius, true, &count, 16, list)`; the closest entity
/// (x87 distance, PC 24, `<` the best so far) gets: P5 != 0 → +0x1C |= 0x81 and, when byte +0x2E == 0, its
/// InvisibilitySettingArray entry cleared; P5 == 0 → +0x1C &= ~0x81, AddToInvisibilitySwapArray(e, false).
fn set_visibility_of_closest_object_of_type(s: Scr) {
    collect(s, 6);
    let (x, y) = (sp(0), sp(1));
    let mut z = sp(2);
    if !X87::from_f32(f(z)).fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
        z = X87::from_f64_exact(find_ground_z_for_coord(x, y)).to_f32().to_bits();
    }
    let radius = sp(3);
    let mut best = X87::from_f32(f(radius)).fdouble().to_f32();
    let mut model = spi(4);
    if model < 0 {
        model = GPtr::<i32>::from_addr(
            USED_OBJECT_MODEL.wrapping_add(model.wrapping_neg().wrapping_mul(0x1C) as u32 as usize),
        )
        .read();
    }
    let mut list = [0u32; 16];
    let mut count: i16 = 0;
    let mut pos = [x, y, z];
    find_objects_of_type_in_range(
        model as u32,
        GPtr::from_raw(pos.as_mut_ptr()),
        radius,
        1,
        GPtr::from_raw(&mut count),
        16,
        GPtr::from_raw(list.as_mut_ptr()),
        1,
        0,
        0,
        1,
        1,
    );
    if count == 0 {
        let mut pos2 = [x, y, z];
        find_lod_of_type_in_range(
            model as u32,
            GPtr::from_raw(pos2.as_mut_ptr()),
            radius,
            1,
            GPtr::from_raw(&mut count),
            16,
            GPtr::from_raw(list.as_mut_ptr()),
        );
    }
    let n = i32::from(count);
    let mut found: GPtr<u8> = GPtr::NULL;
    let mut i: u16 = 0;
    while i32::from(i) < n {
        let e = GPtr::<u8>::from_addr(list[usize::from(i)] as usize);
        let p = entity_pos(e);
        let dz = X87::from_f32(f(p.byte_add(8).read())).fsub_m(f(z));
        let dy = X87::from_f32(f(p.byte_add(4).read())).fsub_m(f(y));
        let dx = X87::from_f32(f(p.read())).fsub_m(f(x));
        let d = dx.fmul(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz)).fsqrt();
        if !d.fcom(X87::from_f32(best)).test_parity(0x05) {
            best = d.to_f32();
            found = e;
        }
        i = i.wrapping_add(1);
    }
    if found.is_null() {
        return;
    }
    let fl = found.byte_add(0x1C).cast::<u32>();
    if sp(5) != 0 {
        fl.write(fl.read() | 0x81);
        if found.byte_add(0x2E).read() != 0 {
            return;
        }
        let arr = GPtr::<u32>::from_addr(INVISIBILITY_SETTING_ARRAY);
        for k in 0..20isize {
            let slot = arr.byte_add(4 * k);
            if slot.read() as usize == found.addr() {
                slot.write(0);
                break;
            }
        }
    } else {
        fl.write(fl.read() & 0xFFFF_FF7E);
        add_to_invisibility_swap_array(found, 0);
    }
}

/// 0376 CREATE_RANDOM_CHAR (0x48332A): model = ChooseCivilianOccupation(false, false, -1, -1, -1, false,
/// false, false, nullptr) up to 5 times until the model info's ped type (+0x28, read only when its +0x1C
/// is non-null) is 4 or 5 (starting from 6); a model whose info has a null +0x1C falls back to model 7 and
/// its ped type. `CCivilianPed(type, model)` (null allocation → null ped); primary task 4 =
/// `CTaskSimpleStandStill(999999, true, false, 8.0)`; created by mission script (2); +0x46C bit 31 cleared;
/// z = (ground z when P2 <= -100, ordered, else P2) + 1.0 (x87); SetPosn(&(P0, P1, z)), zero orientation,
/// clear space, +0x1C bit 0x40000 with mission cleanup, CWorld::Add, mission ped count + 1, P0 = handle,
/// StoreParameters(1), mission cleanup (type 2).
fn create_random_char(s: Scr) {
    collect(s, 3);
    let mi = |m: i32| {
        GPtr::<GPtr<u8>>::from_addr(MODEL_INFO_PTRS.wrapping_add((m as u32).wrapping_mul(4) as usize)).read()
    };
    let mut ped_type: u32 = 6;
    let mut model: i32 = 0;
    let mut tries: u16 = 0;
    while ped_type != 5 && tries < 5 {
        model = choose_civilian_occupation(0, 0, -1, -1, -1, 0, 0, 0, 0);
        let m = mi(model);
        if !m.byte_add(0x1C).cast::<GPtr<u8>>().read().is_null() {
            ped_type = m.byte_add(0x28).cast::<u32>().read();
        }
        tries = tries.wrapping_add(1);
        if ped_type == 4 {
            break;
        }
    }
    if mi(model).byte_add(0x1C).cast::<GPtr<u8>>().read().is_null() {
        ped_type = mi(7).byte_add(0x28).cast::<u32>().read();
        model = 7;
    }
    let p = ped_new(0x79C);
    let ped = if p.is_null() { GPtr::NULL } else { civilian_ped_ctor(p, ped_type, model as u32) };
    let t = task_new(0x20);
    let t = if t.is_null() {
        GPtr::NULL
    } else {
        task_simple_stand_still_ctor(t, 999_999, 1, 0, 0x4100_0000)
    };
    let intel = ped.byte_add(0x47C).cast::<GPtr<u8>>().read();
    task_manager_set_task(intel.byte_add(4), t, 4, 0);
    set_char_created_by(ped, 2);
    let fl = ped.byte_add(0x46C).cast::<u32>();
    fl.write(fl.read() & 0x7FFF_FFFF);
    let (x, y) = (sp(0), sp(1));
    let st = X87::from_f32(f(sp(2)));
    let st = if st.fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
        st
    } else {
        X87::from_f64_exact(find_ground_z_for_coord(x, y))
    };
    let z = st.fadd_m(f(K_ONE)).to_f32().to_bits();
    let mut pos = [x, y, z];
    let pp = GPtr::from_raw(pos.as_mut_ptr());
    set_posn_v(ped, pp);
    set_orientation(ped, 0, 0, 0);
    clear_space_for_mission_entity(pp, ped);
    if uses_mission_cleanup(s) {
        let f1 = ped.byte_add(0x1C).cast::<u32>();
        f1.write(f1.read() | 0x40000);
    }
    world_add(ped);
    let n = GPtr::<u32>::from_addr(TOTAL_MISSION_PEDS);
    n.write(n.read().wrapping_add(1));
    let h = ped_get_ref_(pool(PED_POOL), ped);
    set_sp(0, h as u32);
    store(s, 1);
    if uses_mission_cleanup(s) {
        mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), spi(0), 2);
    }
}

/// The current intro text line: `IntroTextLines[NumberOfIntroTextLinesThisFrame]` (`movzx` of the u16,
/// `imul 0x44`), re-read at every access as compiled.
#[inline]
fn text_line() -> GPtr<u8> {
    let n = u32::from(GPtr::<u16>::from_addr(NUM_INTRO_TEXT_LINES).read());
    GPtr::from_addr(INTRO_TEXT_LINES.wrapping_add(n.wrapping_mul(0x44)) as usize)
}

/// `dword [e+off]` bit `mask` = `v != 0` (read, or/and, write).
#[inline]
fn set_dword_flag(e: GPtr<u8>, off: isize, mask: u32, v: u32) {
    let p = e.byte_add(off).cast::<u32>();
    let old = p.read();
    p.write(if v != 0 { old | mask } else { old & !mask });
}

/// The common return-0 tail 0x4835B1 (via the .text stub 0x4039D5): `~CMatrix` on the function-scope
/// `CMatrix` at [esp+0x9C], whose attach fields (+0x40, +0x44) the prologue zeroes (so the destructor
/// frees nothing). The port runs the call on a zeroed local of the same size (0x48 bytes).
#[inline]
fn common_tail() {
    let m = [0u32; 0x12];
    matrix_dtor(GPtr::from_raw(m.as_ptr().cast_mut()).cast());
}

/// `CRunningScript::ProcessCommands800To899(int32 command)` (0x481300, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x481300, conv = thiscall, name = "CRunningScript::ProcessCommands800To899", expect = "6A FF 68 A8 83 83 00", this = "CRunningScript")]
pub fn process_commands_800_to_899(s: Scr, command: u32) -> i8 {
    match command {
        // 0323 ANCHOR_BOAT (0x481435): no null check; byte +0x5AC bit 2 (0x4) = P1 != 0.
        0x0323 => {
            collect(s, 2);
            let boat = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let p = boat.byte_add(0x5AC);
            let b = p.read();
            p.write(if sp(1) != 0 { b | 0x4 } else { b & 0xFB });
        }
        // 0330 SET_PLAYER_NEVER_GETS_TIRED (0x48175F) / 0331 SET_PLAYER_FAST_RELOAD (0x48178E):
        // CWorld::Players[P0] byte +0x14C / +0x14D = P1 != 0.
        0x0330 | 0x0331 => {
            collect(s, 2);
            let off = if command == 0x0330 { 0x14C } else { 0x14D };
            player_info(sp(0)).byte_add(off).write(u8::from(sp(1) != 0));
        }
        // 0332 SET_CHAR_BLEEDING (0x4817BD): no null check; `m_nPedFlags` dword +0x46C bit 0x40000 = P1 != 0.
        0x0332 => {
            collect(s, 2);
            set_dword_flag(ped_at_ref(pool(PED_POOL), spi(0)), 0x46C, 0x40000, sp(1));
        }
        // 0335 SET_FREE_RESPRAYS (0x481808): CGarages::RespraysAreFree = P0 != 0.
        0x0335 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(RESPRAYS_ARE_FREE).write(u8::from(sp(0) != 0));
        }
        // 0338 SET_CAR_VISIBLE (0x481880): no null check; entity flags +0x1C bit 0x80 (bIsVisible) = P1 != 0.
        0x0338 => {
            collect(s, 2);
            set_dword_flag(vehicle_at_ref(pool(VEHICLE_POOL), spi(0)), 0x1C, 0x80, sp(1));
        }
        // 033F SET_TEXT_SCALE (0x481AAE): line +0 / +4 = P0 / P1 (raw dwords).
        0x033F => {
            collect(s, 2);
            let l = text_line();
            l.cast::<u32>().write(sp(0));
            l.byte_add(4).cast::<u32>().write(sp(1));
        }
        // 0341 SET_TEXT_JUSTIFY (0x481B47), 0342 SET_TEXT_CENTRE (0x481B84), 0345 SET_TEXT_BACKGROUND
        // (0x481C08), 0348 SET_TEXT_PROPORTIONAL (0x481C45): line byte +0xC / +0xD / +0xE / +0x1C = P0 != 0.
        0x0341 | 0x0342 | 0x0345 | 0x0348 => {
            collect(s, 1);
            let off = if command == 0x0341 {
                0xC
            } else if command == 0x0342 {
                0xD
            } else if command == 0x0345 {
                0xE
            } else {
                0x1C
            };
            let v = u8::from(sp(0) != 0);
            text_line().byte_add(off).write(v);
        }
        // 0343 SET_TEXT_WRAPX (0x481BC1), 0344 SET_TEXT_CENTRE_SIZE (0x481BE5), 0349 SET_TEXT_FONT
        // (0x481C82): line dword +0x10 / +0x14 / +0x28 = P0.
        0x0343 | 0x0344 | 0x0349 => {
            collect(s, 1);
            let off = if command == 0x0343 {
                0x10
            } else if command == 0x0344 {
                0x14
            } else {
                0x28
            };
            text_line().byte_add(off).cast::<u32>().write(sp(0));
        }
        // 0350 SET_CHAR_STAY_IN_SAME_PLACE (0x482B4D): no null check; SetStayInSamePlace(P1 != 0).
        0x0350 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            set_stay_in_same_place(ped, u32::from(sp(1) != 0));
        }
        // 0366 HAS_OBJECT_BEEN_DAMAGED (0x4831F8): no null check; flag = 0x46A2F0(obj) != 0.
        0x0366 => {
            collect(s, 1);
            let obj = object_at_ref(pool(OBJECT_POOL), spi(0));
            let b = object_has_been_damaged(obj) != 0;
            set_flag(s, b);
        }
        // 0382 SET_OBJECT_COLLISION (0x483580): no null check; entity flags +0x1C bit 0 (bUsesCollision) = P1 != 0.
        0x0382 => {
            collect(s, 2);
            set_dword_flag(object_at_ref(pool(OBJECT_POOL), spi(0)), 0x1C, 0x1, sp(1));
        }
        // 0362 WARP_CHAR_FROM_CAR_TO_COORD (0x482EA3): no null check. `fld P3; fst z; fcomp -100.0; test
        // ah,0x41; jp`: when `z <= -100` (ordered) z = FindGroundZForCoord(P1, P2). Then
        // `m_pIntelligence (+0x47C)->FlushImmediately(true)`; z = GetDistanceFromCentreOfMassToBaseOfModel()
        // `fadd [z]`; `Teleport((P1, P2, z), false)` (EDI = 0 for the whole switch); ClearSpaceForMissionEntity.
        0x0362 => {
            collect(s, 4);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let st = X87::from_f32(f(sp(3)));
            let (x, y) = (sp(1), sp(2));
            let mut z = st.to_f32().to_bits();
            if !st.fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
                z = X87::from_f64_exact(find_ground_z_for_coord(x, y))
                    .to_f32()
                    .to_bits();
            }
            flush_immediately(ped.byte_add(0x47C).cast::<GPtr<u8>>().read(), 1);
            let d = X87::from_f64_exact(get_distance_from_centre_of_mass_to_base_of_model(ped));
            let z = d.fadd_m(f(z)).to_f32().to_bits();
            teleport(ped, x, y, z, 0);
            let pos = [x, y, z];
            clear_space_for_mission_entity(GPtr::from_raw(pos.as_ptr().cast_mut()), ped);
        }
        // 036A WARP_CHAR_INTO_CAR (0x483239): no null checks. `m_pIntelligence->FlushImmediately(false)`;
        // a stack `CTaskSimpleCarSetPedInAsDriver(veh, nullptr)` (0x1C bytes) with
        // `m_bIsWarpingPedIntoCar` (+0x18) = 1 set after the constructor (gr passes it to the constructor);
        // ProcessPed(ped); destructor. The SEH state stores around the object are not ported.
        0x036A => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(1));
            flush_immediately(ped.byte_add(0x47C).cast::<GPtr<u8>>().read(), 0);
            let mut task = [0u32; 7];
            let t = GPtr::from_raw(task.as_mut_ptr()).cast::<u8>();
            set_ped_in_as_driver_ctor(t, veh, GPtr::NULL);
            t.byte_add(0x18).write(1);
            set_ped_in_as_driver_process_ped(t, ped);
            set_ped_in_as_driver_dtor(t);
        }
        // 0321 EXPLODE_CHAR_HEAD (0x48135B)
        0x0321 => explode_char_head(s),
        // 0325 START_CAR_FIRE (0x481479)
        0x0325 => {
            collect(s, 1);
            start_entity_fire(s, vehicle_at_ref(pool(VEHICLE_POOL), spi(0)));
        }
        // 0326 START_CHAR_FIRE (0x4814E4)
        0x0326 => {
            collect(s, 1);
            start_entity_fire(s, ped_at_ref(pool(PED_POOL), spi(0)));
        }
        // 0327 GET_RANDOM_CAR_OF_TYPE_IN_AREA (0x48150A)
        0x0327 => get_random_car_of_type_in_area(s),
        // 0329 HAS_RESPRAY_HAPPENED, 032C SET_CAR_RAM_CAR, 037E IS_SNIPER_BULLET_IN_AREA, 0383
        // IS_ICECREAM_JINGLE_ON: the jump table sends them to the common tail 0x4835B1 (no operand read,
        // return 0; gr: unimplemented/nop).
        0x0329 | 0x032C | 0x037E | 0x0383 => {}
        // 032A SET_CAMERA_ZOOM (0x481619): by the active camera's mode (u16): 4 → SetZoomValueFollowPedScript,
        // 0x12 → SetZoomValueCamStringScript, with (uint16)P0.
        0x032A => {
            collect(s, 1);
            let cam = u32::from(GPtr::<u8>::from_addr(ACTIVE_CAM).read());
            let mode = GPtr::<u16>::from_addr(CAM_MODES.wrapping_add(cam.wrapping_mul(0x238) as usize)).read();
            if mode == 4 {
                set_zoom_value_follow_ped_script(GPtr::from_addr(THE_CAMERA), sp(0) & 0xFFFF);
            } else if mode == 0x12 {
                set_zoom_value_cam_string_script(GPtr::from_addr(THE_CAMERA), sp(0) & 0xFFFF);
            }
        }
        // 032B CREATE_PICKUP_WITH_AMMO (0x481678)
        0x032B => create_pickup_with_ammo(s),
        // 0337 SET_CHAR_VISIBLE (0x481824): no null check; for FindPlayerPed(-1) the byte 0x8CC380 = P1 != 0;
        // entity flags +0x1C bit 0x80 = P1 != 0.
        0x0337 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            if ped.addr() == find_player_ped(-1).addr() {
                GPtr::<u8>::from_addr(PLAYER_VISIBLE).write(u8::from(sp(1) != 0));
            }
            set_dword_flag(ped, 0x1C, 0x80, sp(1));
        }
        // 0339 IS_AREA_OCCUPIED (0x4818BB)
        0x0339 => is_area_occupied(s),
        // 033E DISPLAY_TEXT (0x481A08): `CollectParameters(2)`; 8-byte key (zeroed buffer); `TheText.Get(key)`
        // (result unused); line +0x2C/+0x30 = P0/P1, `strncpy(line + 0x34, key, 8)`, line count + 1,
        // line +0x3C/+0x40 = -1 (the line before the increment). The case runs the common tail inline.
        0x033E => {
            collect(s, 2);
            let mut buf = [0u8; 16];
            let b = GPtr::from_raw(buf.as_mut_ptr());
            read_text_label(s, b, 8);
            text_get(GPtr::from_addr(THE_TEXT), b);
            let l = text_line();
            l.byte_add(0x2C).cast::<u32>().write(sp(0));
            l.byte_add(0x30).cast::<u32>().write(sp(1));
            crate::vm::strncpy(l.byte_add(0x34), b, 8);
            let c = GPtr::<u16>::from_addr(NUM_INTRO_TEXT_LINES);
            c.write(c.read().wrapping_add(1));
            l.byte_add(0x3C).cast::<u32>().write(u32::MAX);
            l.byte_add(0x40).cast::<u32>().write(u32::MAX);
        }
        // 0340 SET_TEXT_COLOUR (0x481ADE): a stack `CRGBA(P0, P1, P2, P3)` (low bytes) copied to line +8..+0xB.
        0x0340 => {
            collect(s, 4);
            let mut c = [0u32; 1];
            let cp = GPtr::from_raw(c.as_mut_ptr()).cast::<u8>();
            rgba_ctor(cp, sp(0) & 0xFF, sp(1) & 0xFF, sp(2) & 0xFF, sp(3) & 0xFF);
            let l = text_line().byte_add(8);
            for i in 0..4 {
                l.byte_add(i).write(cp.byte_add(i).read());
            }
        }
        // 034D ROTATE_OBJECT (0x481CA5)
        0x034D => rotate_object(s),
        // 034E SLIDE_OBJECT (0x482339)
        0x034E => slide_object(s),
        // 034F REMOVE_CHAR_ELEGANTLY (0x482A96)
        0x034F => remove_char_elegantly(s),
        // 0356 IS_EXPLOSION_IN_AREA (0x482B89): type P0, the three axes ordered as raw dwords;
        // flag = TestForExplosionInArea(P0, x1, x2, y1, y2, z1, z2) != 0.
        0x0356 => {
            collect(s, 7);
            let (x1, x2) = order(sp(1), sp(4));
            let (y1, y2) = order(sp(2), sp(5));
            let (z1, z2) = order(sp(3), sp(6));
            let b = test_for_explosion_in_area(sp(0), x1, x2, y1, y2, z1, z2) != 0;
            set_flag(s, b);
        }
        // 035C PLACE_OBJECT_RELATIVE_TO_CAR (0x482C67): no null checks;
        // PlacePhysicalRelativeToOtherPhysical(veh, obj, (P2, P3, P4)).
        0x035C => {
            collect(s, 5);
            let obj = object_at_ref(pool(OBJECT_POOL), spi(0));
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(1));
            place_physical_relative_to_other_physical(veh, obj, sp(2), sp(3), sp(4));
        }
        // 035D MAKE_OBJECT_TARGETTABLE (0x482CDB)
        0x035D => make_object_targettable(s),
        // 035F ADD_ARMOUR_TO_CHAR (0x482D52)
        0x035F => add_armour_to_char(s),
        // 0360 OPEN_GARAGE (0x482E1D)
        0x0360 => open_close_garage(s, true),
        // 0361 CLOSE_GARAGE (0x482E60)
        0x0361 => open_close_garage(s, false),
        // 0363 SET_VISIBILITY_OF_CLOSEST_OBJECT_OF_TYPE (0x482F84)
        0x0363 => set_visibility_of_closest_object_of_type(s),
        // 0364 HAS_CHAR_SPOTTED_CHAR (0x4831A2): no null checks; flag = OurPedCanSeeThisEntity(P1 ped, true).
        0x0364 => {
            collect(s, 2);
            let a = ped_at_ref(pool(PED_POOL), spi(0));
            let b = ped_at_ref(pool(PED_POOL), spi(1));
            let v = our_ped_can_see_this_entity(a, b, 1) != 0;
            set_flag(s, v);
        }
        // 036D PRINT_WITH_2_NUMBERS_BIG (0x4832BB): 8-byte key (zeroed buffer), `TheText.Get`;
        // `CollectParameters(4)`; AddBigMessageWithNumber(text, P2, P3 - 1, P0, P1, -1 x4).
        0x036D => {
            let mut buf = [0u8; 16];
            let b = GPtr::from_raw(buf.as_mut_ptr());
            read_text_label(s, b, 8);
            let t = text_get(GPtr::from_addr(THE_TEXT), b);
            collect(s, 4);
            let m = u32::MAX;
            add_big_message_with_number(t, sp(2), sp(3).wrapping_sub(1), sp(0), sp(1), m, m, m, m);
        }
        // 0376 CREATE_RANDOM_CHAR (0x48332A)
        0x0376 => create_random_char(s),
        // 0381 SET_OBJECT_VELOCITY (0x48352B): no null check; `SetVelocity((P1, P2, P3) * 0.02)` (x87, each
        // `fld; fmul; fstp`, z first).
        0x0381 => {
            collect(s, 4);
            let obj = object_at_ref(pool(OBJECT_POOL), spi(0));
            let k = f(K_0_02);
            let z = X87::from_f32(spf(3)).fmul_m(k).to_f32().to_bits();
            let y = X87::from_f32(spf(2)).fmul_m(k).to_f32().to_bits();
            let x = X87::from_f32(spf(1)).fmul_m(k).to_f32().to_bits();
            set_velocity(obj, x, y, z);
        }
        // 0373 SET_CAMERA_BEHIND_PLAYER (0x48331B)
        0x0373 => set_camera_directly_behind_for_follow_ped_cam_on_a_string(GPtr::from_addr(THE_CAMERA)),
        _ => {
            common_tail();
            return -1;
        }
    }
    common_tail();
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_800_TO_899];
