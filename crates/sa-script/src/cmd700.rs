//! `CRunningScript::ProcessCommands700To799` (0x47FA30): complete port (PORT-SCRIPT-2 partial, completed in
//! PORT-SCRIPT-3). Every jump-table case is in [`PORTED`]; the other opcodes of the range reach the
//! original default (-1). Cases follow the 1.0 US bytes and
//! `docs/re/specs/SCRIPT/commands/ProcessCommands700To799.md`.
//!
//! The prologue's `[esp+0x54] = 0` is BOAT_GOTO_COORDS' water-level local (ported as a zeroed local).
//! Divergence notes: the 8-byte label cases (02DD, 02E4, 02FA, 02FD..0308, 0318) read the label into a
//! zeroed 16-byte buffer (the original's is not cleared: an 8-character label is followed by stale
//! stack bytes in the C-string readers, and 02DD's copy loop overruns its 8-byte local); SIN/COS use
//! the 53-bit FSIN/FCOS model (accepted project-wide).

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x02BF, 0x02C0, 0x02C1, 0x02C2, 0x02CA, 0x02CB, 0x02CC, 0x02CE, 0x02CF, 0x02D0, 0x02D1, 0x02D3, 0x02D4,
    0x02D6, 0x02D8, 0x02DB, 0x02DD, 0x02E0, 0x02E1, 0x02E2, 0x02E3, 0x02E4, 0x02E5, 0x02E6, 0x02E7, 0x02E8,
    0x02E9, 0x02EA, 0x02EB, 0x02EC, 0x02ED, 0x02EE, 0x02F2, 0x02F3, 0x02F6, 0x02F7, 0x02F8, 0x02F9, 0x02FA,
    0x02FD, 0x02FF, 0x0302, 0x0303, 0x0308, 0x030C, 0x030D, 0x0317, 0x0318, 0x0319, 0x031A, 0x031D, 0x031E,
];

/// `.rdata` 0x859014: -100.0f
const K_MINUS_100: u32 = 0xC2C8_0000;
/// 999999.0f (an immediate, `push 0x497423FE`)
const K_999999: u32 = 0x4974_23FE;
/// `.rdata` 0x8595EC: 0.017453292f (degrees to radians)
const K_DEG2RAD: u32 = 0x3C8E_FA35;
/// `.rdata` 0x858C80: 5.0f
const K_5: u32 = 0x40A0_0000;
/// `.rdata` 0x858B8C: 0.5f
const K_HALF: u32 = 0x3F00_0000;
/// `ThePaths` (`CPathFind`) 0x96F050 and its `m_pPathNodes[]` (+0x804) 0x96F854.
const THE_PATHS: usize = 0x96F050;
const PATH_NODES: usize = 0x96F854;
/// `TheText` (`CText`) 0xC1B340.
const THE_TEXT: usize = 0xC1B340;
/// `CTheScripts::bAddNextMessageToPreviousBriefs` 0xA4448B.
const ADD_NEXT_MESSAGE_TO_PREVIOUS_BRIEFS: usize = 0xA4448B;
/// `CTheScripts::LastRandomPedId` 0xA476A4.
const LAST_RANDOM_PED_ID: usize = 0xA476A4;
/// `CTheScripts::LastMissionPassedTime` 0xA476A8.
const LAST_MISSION_PASSED_TIME: usize = 0xA476A8;
/// `CStats::LastMissionPassedName` 0xB78A00.
const LAST_MISSION_PASSED_NAME: usize = 0xB78A00;
/// `CPopulation::ms_nTotalMissionPeds` 0xC0EC24.
const TOTAL_MISSION_PEDS: usize = 0xC0EC24;
/// `CTheScripts::DbgFlag` (byte in `.rdata`, 0) 0x859CF8.
const DBG_FLAG: usize = 0x859CF8;
/// `ModelIndices::MI_MONEY` (int16) 0x8CD59C.
const MI_MONEY: usize = 0x8CD59C;
/// `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8.
const MODEL_INFO_PTRS: usize = 0xA9B0C8;

/// `gFireManager` (`CFireManager`) 0xB71F80.
const FIRE_MANAGER: usize = 0xB71F80;
/// `.rdata` 0x858B40: 50.0f
const K_50: u32 = 0x4248_0000;
/// `CWorld::PlayerInFocus` (byte) 0xB7CD74.
const PLAYER_IN_FOCUS: usize = 0xB7CD74;

/// `.rdata` 0x858624: 1.0f
const K_ONE: u32 = 0x3F80_0000;
/// `g_surfaceInfos` (`SurfaceInfos_c`) 0xB79538.
const SURFACE_INFOS: usize = 0xB79538;

orig! {
    /// `CTheScripts::GetActualScriptThingIndex(int32 ref, uint8 type)` (0x4839A0)
    fn get_actual_script_thing_index(r: i32, ty: u32) -> i32 = cdecl 0x4839A0 => unported;
    /// `CFireManager::IsScriptFireExtinguished(int16)` (0x5396E0), AL
    fn is_script_fire_extinguished(fm: GPtr<u8>, i: i32) -> u8 = thiscall 0x5396E0 => unported;
    /// `CFireManager::RemoveScriptFire(int16)` (0x539700)
    fn remove_script_fire(fm: GPtr<u8>, i: i32) = thiscall 0x539700 => unported;
    /// `CFireManager::RemoveAllScriptFires()` (0x539720)
    fn remove_all_script_fires(fm: GPtr<u8>) = thiscall 0x539720 => unported;
    /// `CVector::Magnitude()` (0x4082C0), ST0 (an FSQRT result under PC=24: exact as f64)
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CCutsceneMgr::StartCutscene()` (0x5B1460)
    fn start_cutscene() = cdecl 0x5B1460 => unported;
    /// `CCutsceneMgr::GetCutsceneTimeInMilleseconds()` (0x5B0550)
    fn get_cutscene_time_in_milleseconds() -> u32 = cdecl 0x5B0550 => unported;
    /// `CCutsceneMgr::HasCutsceneFinished()` (0x5B0570), AL
    fn has_cutscene_finished() -> u8 = cdecl 0x5B0570 => unported;
    /// `CCutsceneMgr::DeleteCutsceneData()` (0x4D5ED0)
    fn delete_cutscene_data() = cdecl 0x4D5ED0 => unported;
    /// `CStats::IncrementStat(uint16 stat, float value)` (0x55C180); the float as its raw dword
    fn increment_stat(stat: u32, value: u32) = cdecl 0x55C180 => unported;
    /// `CStats::SetStatValue(uint16 stat, float value)` (0x55A070); the float as its raw dword
    fn set_stat_value(stat: u32, value: u32) = cdecl 0x55A070 => unported;
    /// `SurfaceInfos_c::IsShallowWater(uint32 surface)` (0x55E730), AL
    fn is_shallow_water(si: GPtr<u8>, surface: u32) -> u8 = thiscall 0x55E730 => unported;
    /// `CWorld::FindGroundZFor3DCoord(float x, float y, float z, bool* outResult, CEntity** outEntity)`
    /// (0x5696C0), floats as raw dwords, ST0 (read as f64: exact)
    fn find_ground_z_for_3d_coord(x: u32, y: u32, z: u32, out: GPtr<u8>, ent: GPtr<u8>) -> f64 = cdecl 0x5696C0 => unported;
    /// `CCamera::RestoreWithJumpCut()` (0x50BAB0)
    fn restore_with_jump_cut(cam: GPtr<u8>) = thiscall 0x50BAB0 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), floats as raw dwords, ST0 (exact as f64)
    fn find_ground_z_for_coord(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CPathFind::FindNodeClosestToCoors(CVector, uint8 nodeType, float maxDist, uint16, uint16,
    /// uint16, bool, bool)` (0x44F460), the `CNodeAddress` returned through the hidden pointer (EAX)
    fn find_node_closest_to_coors(paths: GPtr<u8>, out: GPtr<u32>, x: u32, y: u32, z: u32, ty: u32, max: u32, a: u32, b: u32, c: u32, d: u32, e: u32) -> GPtr<u32> = thiscall 0x44F460 => unported;
    /// `CPathNode::GetPosition()` (0x420A10), the vector returned through the hidden pointer
    fn path_node_get_position(node: GPtr<u8>, out: GPtr<u32>) -> GPtr<u32> = thiscall 0x420A10 => unported;
    /// `CPathFind::FindNodeCoorsForScript(CNodeAddress, bool* found)` (0x4505E0), hidden-pointer vector
    /// (writes `*found` on every path)
    fn find_node_coors_for_script(paths: GPtr<u8>, out: GPtr<u32>, addr: u32, found: GPtr<u8>) -> GPtr<u32> = thiscall 0x4505E0 => unported;
    /// `CEntity::GetDistanceFromCentreOfMassToBaseOfModel()` (0x536BE0), ST0 (exact as f64)
    fn get_distance_from_centre_of_mass_to_base_of_model(e: GPtr<u8>) -> f64 = thiscall 0x536BE0 => unported;
    /// `CCarCtrl::JoinCarWithRoadSystemGotoCoors(CVehicle*, const CVector&, bool, bool)` (0x42F870), AL
    fn join_car_with_road_system_goto_coors(v: GPtr<u8>, pos: GPtr<u32>, a: u32, b: u32) -> u8 = cdecl 0x42F870 => unported;
    /// Unnamed thiscall on the vehicle with one argument (0x41BDD0, a `.HOODLUM` stub; the
    /// `CAR_GOTO_*`/`BOAT_GOTO_COORDS` cases pass 1)
    fn vehicle_41bdd0(v: GPtr<u8>, a: u32) = thiscall 0x41BDD0 => unported;
    /// `CEntity::GetBoundCentre()` (0x534250), the vector returned through the hidden pointer
    fn get_bound_centre(e: GPtr<u8>, out: GPtr<u32>) -> GPtr<u32> = thiscall 0x534250 => unported;
    /// `CCamera::IsSphereVisible(const CVector&, float)` (0x420D40), the float as its raw dword, AL
    fn is_sphere_visible(cam: GPtr<u8>, c: GPtr<u32>, r: u32) -> u8 = thiscall 0x420D40 => unported;
    /// `CFireManager::StartScriptFire(const CVector&, CEntity*, float, uint8, int8 generation, int32
    /// nStrength)` (0x53A270)
    fn start_script_fire(fm: GPtr<u8>, pos: GPtr<u32>, target: GPtr<u8>, a: u32, b: u32, generation: u32, strength: u32) -> i32 = thiscall 0x53A270 => unported;
    /// `CWaterLevel::GetWaterLevel(float x, float y, float z, float* level, bool touching, CVector*)`
    /// (0x6EB690), AL (discarded)
    fn get_water_level(x: u32, y: u32, z: u32, level: GPtr<u32>, touching: u32, normal: u32) -> u8 = cdecl 0x6EB690 => unported;
    /// `CPlaceable::IsWithinArea(float x1, float y1, float x2, float y2)` (0x54F200), AL
    fn is_within_area(e: GPtr<u8>, x1: u32, y1: u32, x2: u32, y2: u32) -> u8 = thiscall 0x54F200 => unported;
    /// `CTheScripts::HighlightImportantArea(uint32 id, float x1, float y1, float x2, float y2, float z)` (0x485E00)
    fn highlight_important_area(id: u32, x1: u32, y1: u32, x2: u32, y2: u32, z: u32) = cdecl 0x485E00 => unported;
    /// `CTheScripts::DrawDebugSquare(float, float, float, float)` (0x486840)
    fn draw_debug_square(x1: u32, y1: u32, x2: u32, y2: u32) = cdecl 0x486840 => unported;
    /// `CWeapon::IsTypeMelee()` (0x73B1C0), AL
    fn weapon_is_type_melee(w: GPtr<u8>) -> u8 = thiscall 0x73B1C0 => unported;
    /// `CRunningScript::ReadTextLabelFromScript(char*, uint8)` (0x463D50)
    fn read_label(s: Scr, buf: GPtr<u8>, len: u32) = thiscall 0x463D50 => unported;
    /// `CWorld::FindPlayerCoors(int32)` (0x56E010), the vector returned through the hidden pointer
    fn find_player_coors(out: GPtr<u32>, player: i32) -> GPtr<u32> = cdecl 0x56E010 => unported;
    /// `CRunningScript::IsPedDead(CPed*)` (0x464D70), AL
    fn is_ped_dead(s: Scr, p: GPtr<u8>) -> u8 = thiscall 0x464D70 => unported;
    /// `CRunningScript::ThisIsAValidRandomPed(ePedType, bool civilian, bool gang, bool criminal)` (0x489490), AL
    fn this_is_a_valid_random_ped(s: Scr, ty: u32, a: u32, b: u32, c: u32) -> u8 = thiscall 0x489490 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80)
    fn get_peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CTheZones::FindZone(const CVector*, uint64 label, eZoneType)` (0x572B80), AL
    fn find_zone(pos: GPtr<u32>, lo: u32, hi: u32, ty: u32) -> u8 = cdecl 0x572B80 => unported;
    /// `CWorld::FindRoofZFor3DCoord(float, float, float, bool*)` (0x569750), ST0 (discarded)
    fn find_roof_z_for_3d_coord(x: u32, y: u32, z: u32, found: GPtr<u8>) -> f64 = cdecl 0x569750 => unported;
    /// `CPed::SetCharCreatedBy(uint8)` (0x5E47E0)
    fn set_char_created_by(p: GPtr<u8>, by: u32) = thiscall 0x5E47E0 => unported;
    /// `CPickups::GetActualPickupIndex(int32)` (0x4552A0), result discarded
    fn get_actual_pickup_index(h: i32) -> i32 = cdecl 0x4552A0 => unported;
    /// `CPickups::GenerateNewOne(CVector, uint32 model, ePickupType, uint32 ammo, uint32 moneyPerDay,
    /// bool isEmpty, const char* message)` (0x456F20)
    fn generate_new_one(x: u32, y: u32, z: u32, model: u32, ty: u32, ammo: u32, money: u32, empty: u32, msg: u32) -> i32 = cdecl 0x456F20 => unported;
    /// `CCutsceneMgr::LoadCutsceneData(const char*)` (0x4D5E80)
    fn load_cutscene_data(name: GPtr<u8>) = cdecl 0x4D5E80 => unported;
    /// `CProjectileInfo::IsProjectileInRange(float x1, float x2, float y1, float y2, float z1, float z2,
    /// bool destroy)` (0x739860), AL
    fn is_projectile_in_range(x1: u32, x2: u32, y1: u32, y2: u32, z1: u32, z2: u32, destroy: u32) -> u8 = cdecl 0x739860 => unported;
    /// Unnamed debug cube draw (0x486AE0; `CTheScripts::DrawDebugCube(x1, y1, z1, x2, y2, z2)`)
    fn draw_debug_cube(x1: u32, y1: u32, z1: u32, x2: u32, y2: u32, z2: u32) = cdecl 0x486AE0 => unported;
    /// `CGarages::GetGarageNumberByName(const char*)` (0x447680), AX (EAX taken whole)
    fn get_garage_number_by_name(name: GPtr<u8>) -> u32 = cdecl 0x447680 => unported;
    /// `CGarages::ChangeGarageType(int16, uint8, uint32)` (0x4476D0)
    fn change_garage_type(id: u32, ty: u32, model: u32) = cdecl 0x4476D0 => unported;
    /// `CText::Get(const char* key)` (0x6A0050)
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CMessages::AddMessageWithNumber(const char*, uint32 time, uint16 flag, int32 n1..n6, bool)` (0x69E360)
    fn add_message_with_number(
        t: GPtr<u8>, time: u32, flag: u32, n1: u32, n2: u32, n3: u32, n4: u32, n5: u32, n6: u32, prev: u32
    ) = cdecl 0x69E360 => unported;
    /// `CMessages::AddMessageJumpQWithNumber(const char*, uint32 time, uint16 flag, int32 n1..n6, bool)` (0x69E4E0)
    fn add_message_jump_q_with_number(
        t: GPtr<u8>, time: u32, flag: u32, n1: u32, n2: u32, n3: u32, n4: u32, n5: u32, n6: u32, prev: u32
    ) = cdecl 0x69E4E0 => unported;
    /// `CStats::GetStatValue(uint16)` (0x558E40), ST0 (an f32 value: exact as f64)
    fn get_stat_value(stat: u32) -> f64 = cdecl 0x558E40 => unported;
    /// `CDarkel::CheckDamagedWeaponType(int32 damageWeaponId, int32 expectedDamageWeaponId)` (0x43D9E0), AL
    fn check_damaged_weapon_type(got: i32, want: i32) -> u8 = cdecl 0x43D9E0 => unported;
}

/// The "-100 ground rule" of this switch (`fld z; fcom [-100.0]; test ah,0x41; jp keep`, else `fstp
/// st(0)` and ST0 = `FindGroundZForCoord(x, y)`; ST0 then stored as f32).
fn ground_rule(x: u32, y: u32, z: u32) -> u32 {
    let st = X87::from_f32(f(z));
    let st = if st.fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
        st
    } else {
        X87::from_f64_exact(find_ground_z_for_coord(x, y))
    };
    st.to_f32().to_bits()
}

/// A local `CVector` (three raw dwords) as the pointer the original passes.
fn vec_ptr(v: &[u32; 3]) -> GPtr<u32> {
    GPtr::from_raw(v.as_ptr().cast_mut())
}

/// `CPlaceable::GetPosition()` inlined: `m_matrix` (+0x14) ? `+0x30` of it : `m_placement.pos` (+0x4).
fn entity_pos(e: GPtr<u8>) -> [u32; 3] {
    let m = e.byte_add(0x14).cast::<GPtr<u8>>().read();
    let p = if m.is_null() { e.byte_add(0x4) } else { m.byte_add(0x30) }.cast::<u32>();
    [p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read()]
}

/// An 8-byte text label read into a zeroed 16-byte buffer (`ReadTextLabelFromScript(buf, 8)`).
/// Divergence note: the original's stack buffer is not cleared, so an 8-character label has no
/// terminator and a C-string reader goes on into stale stack bytes.
fn label8(s: Scr, buf: &mut [u8; 16]) -> GPtr<u8> {
    let p = GPtr::from_raw(buf.as_mut_ptr());
    read_label(s, p, 8);
    p
}

/// The zone-name copy of 02DD: the label byte by byte up to and including its NUL into a zeroed
/// 8-byte local, returned as the two dwords passed by value (see 0154 in cmd300 for the overrun note).
fn zone_label(s: Scr) -> (u32, u32) {
    let mut buf = [0u8; 16];
    label8(s, &mut buf);
    let mut z = [0u8; 8];
    for (d, &c) in z.iter_mut().zip(buf.iter()) {
        *d = c;
        if c == 0 {
            break;
        }
    }
    (u32::from_le_bytes([z[0], z[1], z[2], z[3]]), u32::from_le_bytes([z[4], z[5], z[6], z[7]]))
}

/// The tail of 02C2/02D3: status = PHYSICS (`byte +0x36 = (b & 7) | 0x18`), `0x41BDD0(veh, 1)`,
/// cruise speed (byte +0x3D0) raised to at least 1 (unsigned), `m_nTimeToStartMission` (+0x3AC) =
/// `CTimer::m_snTimeInMilliseconds`.
fn car_goto_tail(veh: GPtr<u8>) {
    let st = veh.byte_add(0x36);
    st.write((st.read() & 7) | 0x18);
    vehicle_41bdd0(veh, 1);
    let cs = veh.byte_add(0x3D0);
    let v = cs.read();
    cs.write(if v > 1 { v } else { 1 });
    veh.byte_add(0x3AC).cast::<u32>().write(sa_core::timer::TIME_MS.get());
}

/// `AutoPilot.m_nCarMission` (+0x3BA) = `m` unless it is 0x39 or 0x3A.
fn set_car_mission(veh: GPtr<u8>, m: u8) {
    let p = veh.byte_add(0x3BA);
    let c = p.read();
    if c != 0x39 && c != 0x3A {
        p.write(m);
    }
}

/// 02CA/02CB/02CC IS_*_ON_SCREEN: r = `ms_modelInfoPtrs[(int16)e->m_nModelIndex]->m_pColModel` (+0x14)
/// `->m_boundSphere.m_fRadius` (+0x24), read before the call; flag = `TheCamera.IsSphereVisible(
/// e->GetBoundCentre(), r)` != 0. No null checks.
fn on_screen(s: Scr, e: GPtr<u8>) {
    let m = i32::from(e.byte_add(0x22).cast::<u16>().read() as i16);
    let mi = GPtr::<GPtr<u8>>::from_addr(MODEL_INFO_PTRS.wrapping_add((m as u32).wrapping_mul(4) as usize)).read();
    let r = mi.byte_add(0x14).cast::<GPtr<u8>>().read().byte_add(0x24).cast::<u32>().read();
    let mut out = [0u32; 3];
    let c = get_bound_centre(e, GPtr::from_raw(out.as_mut_ptr()));
    let b = is_sphere_visible(GPtr::from_addr(THE_CAMERA), c, r) != 0;
    set_flag(s, b);
}

/// 02DD GET_RANDOM_CHAR_IN_ZONE (0x4802C4): zone label (copied as in 0154, `zone_label`, read before the
/// operands); CollectParameters(3) (civilian, gang, criminal); `pz` = FindPlayerCoors(-1).z. Walks the
/// ped pool from slot `m_nSize - 1` down to 0 until a handle is found (none when the size is 0); a slot
/// is skipped when free (`(int8)m_byteMap[i] < 0`), its object address is 0, its handle is
/// `LastRandomPedId`, `m_nCreatedBy` (+0x484) != 1, `+0x1C` bit 11, byte `+0x470` bit 3, IsPedDead,
/// `m_pVehicle` (+0x58C) != 0, `+0x46C` bit 8, `!ThisIsAValidRandomPed(m_nPedType (+0x598), P0, P1, P2)`,
/// `GetPedsGroup(ped) != 0`, `!FindZone(&pos, label, 0)`, not `pz - 5 <= z` (`test ah,0x41; jp`), `pz + 5 <
/// z` or unordered (`test ah,1; jne`), or FindRoofZFor3DCoord(pos, &found) sets `found` (ST0 discarded).
/// The chosen ped: handle -> LastRandomPedId, SetCharCreatedBy(2), mission ped count + 1, cleanup add
/// (type 2) with mission cleanup. P0 = handle or -1; StoreParameters(1). The pool pointer is re-read
/// per slot (the original re-reads it after most checks; it does not change during the loop).
fn get_random_char_in_zone(s: Scr) {
    let (lo, hi) = zone_label(s);
    collect(s, 3);
    let size = pool(PED_POOL).byte_add(8).cast::<u32>().read();
    let mut tmp = [0u32; 3];
    let pz = find_player_coors(GPtr::from_raw(tmp.as_mut_ptr()), -1).wrapping_index(2).read();
    let mut h: i32 = -1;
    let mut i = size;
    while i != 0 {
        i = i.wrapping_sub(1);
        if h != -1 {
            break;
        }
        let p = pool(PED_POOL);
        let flags = p.byte_add(4).cast::<GPtr<u8>>().read();
        if (flags.byte_add(i as isize).read() as i8) < 0 {
            continue;
        }
        let base = p.cast::<u32>().read();
        let ped = GPtr::<u8>::from_addr(base.wrapping_add(i.wrapping_mul(0x7C4)) as usize);
        if ped.is_null()
            || ped_get_ref(p, ped) == GPtr::<i32>::from_addr(LAST_RANDOM_PED_ID).read()
            || ped.byte_add(0x484).read() != 1
            || ped.byte_add(0x1C).cast::<u32>().read() & 0x800 != 0
            || ped.byte_add(0x470).read() & 8 != 0
            || is_ped_dead(s, ped) != 0
            || ped.byte_add(0x58C).cast::<u32>().read() != 0
            || ped.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0
            || this_is_a_valid_random_ped(s, ped.byte_add(0x598).cast::<u32>().read(), sp(0), sp(1), sp(2)) == 0
            || !get_peds_group(ped).is_null()
        {
            continue;
        }
        let pos = entity_pos(ped);
        if find_zone(vec_ptr(&pos), lo, hi, 0) == 0 {
            continue;
        }
        let z = X87::from_f32(f(pos[2]));
        if X87::from_f32(f(pz)).fsub_m(f(K_5)).fcom(z).test_parity(0x41)
            || X87::from_f32(f(pz)).fadd_m(f(K_5)).fcom(z).test_any(0x01)
        {
            continue;
        }
        let mut found = [0u8; 1];
        find_roof_z_for_3d_coord(pos[0], pos[1], pos[2], GPtr::from_raw(found.as_mut_ptr()));
        if found[0] != 0 {
            continue;
        }
        h = ped_get_ref(pool(PED_POOL), ped);
        GPtr::<i32>::from_addr(LAST_RANDOM_PED_ID).write(h);
        set_char_created_by(ped, 2);
        let n = GPtr::<u32>::from_addr(TOTAL_MISSION_PEDS);
        n.write(n.read().wrapping_add(1));
        if uses_mission_cleanup(s) {
            mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), h, 2);
        }
    }
    set_sp(0, h as u32);
    store(s, 1);
}

/// 02C0/02C1: store P0..P2 = `v` and flag = `found`, `StoreParameters(3)` before `UpdateCompareFlag`.
fn store_node(s: Scr, v: [u32; 3], found: bool) {
    set_sp(1, v[1]);
    set_sp(2, v[2]);
    set_sp(0, v[0]);
    store(s, 3);
    set_flag(s, found);
}

/// `CRunningScript::ProcessCommands700To799(int32 command)` (0x47FA30, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x47FA30, conv = thiscall, name = "CRunningScript::ProcessCommands700To799", expect = "81 EC D8 00 00 00", this = "CRunningScript")]
pub fn process_commands_700_to_799(s: Scr, command: u32) -> i8 {
    // The prologue's `mov dword [esp+0x54], 0` initialises the local 02D3 reads (`level`).
    match command {
        // 02BF IS_CAR_IN_WATER (0x47FA67): null -> false. True when `m_nPhysicalFlags` (+0x40) bit 8
        // (bIsInWater) is set; else, for model 0x21B (Vortex, `cmp word [+0x22]`), when
        // `m_fWheelsSuspensionCompression[0]` (+0x7D4) `fcomp [1.0]; test ah,5; jp` is below 1.0 (ordered)
        // and `g_surfaceInfos.IsShallowWater((u8)[+0x747])` returns non-zero.
        0x02BF => {
            collect(s, 1);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let mut b = false;
            if !veh.is_null() {
                if veh.byte_add(0x40).cast::<u32>().read() & 0x100 != 0 {
                    b = true;
                } else if veh.byte_add(0x22).cast::<u16>().read() == 0x21B {
                    let c =
                        X87::from_f32(f(veh.byte_add(0x7D4).cast::<u32>().read())).fcom(X87::constant(K_ONE));
                    if !c.test_parity(5) {
                        let surface = u32::from(veh.byte_add(0x747).read());
                        b = is_shallow_water(GPtr::from_addr(SURFACE_INFOS), surface) != 0;
                    }
                }
            }
            set_flag(s, b);
        }
        // 02CE GET_GROUND_Z_FOR_3D_COORD (0x47FE8C): P0 = FindGroundZFor3DCoord(x, y, z, &local, nullptr)
        // (`fstp` into ScriptParams[0]; the bool local is not read); StoreParameters(1).
        0x02CE => {
            collect(s, 3);
            let mut found = [0u8; 1];
            let r = find_ground_z_for_3d_coord(
                sp(0),
                sp(1),
                sp(2),
                GPtr::from_raw(found.as_mut_ptr()),
                GPtr::NULL,
            );
            set_sp(0, X87::from_f64_exact(r).to_f32().to_bits());
            store(s, 1);
        }
        // 02D0 IS_SCRIPT_FIRE_EXTINGUISHED (0x47FF83): i = GetActualScriptThingIndex(P0, 5); flag =
        // `0 <= i < 60 && gFireManager.IsScriptFireExtinguished(i) != 0` (compare tail 0x47FAC2).
        0x02D0 => {
            collect(s, 1);
            let i = get_actual_script_thing_index(spi(0), 5);
            let b =
                (0..0x3C).contains(&i) && is_script_fire_extinguished(GPtr::from_addr(FIRE_MANAGER), i) != 0;
            set_flag(s, b);
        }
        // 02D1 REMOVE_SCRIPT_FIRE (0x47FFC3): i as above; gFireManager.RemoveScriptFire(i) when 0 <= i < 60.
        0x02D1 => {
            collect(s, 1);
            let i = get_actual_script_thing_index(spi(0), 5);
            if (0..0x3C).contains(&i) {
                remove_script_fire(GPtr::from_addr(FIRE_MANAGER), i);
            }
        }
        // 02DB SET_BOAT_CRUISE_SPEED (0x480288): `_ftol2(P1)` (low byte) before the lookup; no null check;
        // byte +0x3D0 = it.
        0x02DB => {
            collect(s, 2);
            let v = X87::from_f32(spf(1)).ftol() as u8;
            let boat = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            boat.byte_add(0x3D0).write(v);
        }
        // 02E0 IS_CHAR_SHOOTING (0x480521): no null check; flag = byte +0x46E bit 0 (bFiringWeapon).
        0x02E0 => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            set_flag(s, ped.byte_add(0x46E).read() & 1 != 0);
        }
        // 02E2 SET_CHAR_ACCURACY (0x48067D): no null check; `m_nWeaponAccuracy` (+0x71A) = (u8)P1.
        0x02E2 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            ped.byte_add(0x71A).write(sp(1) as u8);
        }
        // 02E3 GET_CAR_SPEED (0x4806B3): no null check; P0 = m_vecMoveSpeed (+0x44).Magnitude() `fmul [50.0]`.
        0x02E3 => {
            collect(s, 1);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let m = X87::from_f64_exact(vector_magnitude(veh.byte_add(0x44)));
            set_sp(0, m.fmul_m(f(K_50)).to_f32().to_bits());
            store(s, 1);
        }
        // 02E7 START_CUTSCENE (0x48072B)
        0x02E7 => start_cutscene(),
        // 02E8 GET_CUTSCENE_TIME (0x48073F): P0 = GetCutsceneTimeInMilleseconds(); StoreParameters(1).
        0x02E8 => {
            set_sp(0, get_cutscene_time_in_milleseconds());
            store(s, 1);
        }
        // 02E9 HAS_CUTSCENE_FINISHED (0x480761): flag = HasCutsceneFinished() != 0.
        0x02E9 => {
            let b = has_cutscene_finished() != 0;
            set_flag(s, b);
        }
        // 02EA CLEAR_CUTSCENE (0x48078A): CCutsceneMgr::DeleteCutsceneData().
        0x02EA => delete_cutscene_data(),
        // 02EB RESTORE_CAMERA_JUMPCUT (0x48079E)
        0x02EB => restore_with_jump_cut(GPtr::from_addr(THE_CAMERA)),
        // 02EC CREATE_COLLECTABLE1 (0x4807B7): operands read only.
        0x02EC => collect(s, 3),
        // 02ED SET_COLLECTABLE1_TOTAL (0x4807CF): CWorld::Players[PlayerInFocus (u8)].m_nTotalPackages
        // (+0xC4) = P0.
        0x02ED => {
            collect(s, 1);
            let i = u32::from(GPtr::<u8>::from_addr(PLAYER_IN_FOCUS).read());
            player_info(i).byte_add(0xC4).cast::<u32>().write(sp(0));
        }
        // 02F2 IS_CHAR_MODEL (0x480913): no null check; flag = (int16)m_nModelIndex (+0x22) == P1.
        0x02F2 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let m = i32::from(ped.byte_add(0x22).cast::<u16>().read() as i16);
            set_flag(s, m == spi(1));
        }
        // 030C PLAYER_MADE_PROGRESS (0x480CFC): CStats::IncrementStat(0, (float)P0) (`fild; fstp` f32).
        0x030C => {
            collect(s, 1);
            increment_stat(0, X87::from_i32(spi(0)).to_f32().to_bits());
        }
        // 030D SET_PROGRESS_TOTAL (0x480D28): CStats::SetStatValue(1, (float)P0).
        0x030D => {
            collect(s, 1);
            set_stat_value(1, X87::from_i32(spi(0)).to_f32().to_bits());
        }
        // 0317 REGISTER_MISSION_GIVEN (0x480D54): CStats::IncrementStat(0x92, 1.0f).
        0x0317 => increment_stat(0x92, 0x3F80_0000),
        // 031A REMOVE_ALL_SCRIPT_FIRES (0x480DFB)
        0x031A => remove_all_script_fires(GPtr::from_addr(FIRE_MANAGER)),
        // 02C0 GET_CLOSEST_CHAR_NODE (0x47FAE6) / 02C1 GET_CLOSEST_CAR_NODE (0x47FBDE, gr: unregistered):
        // -100 ground rule on z; a = ThePaths.FindNodeClosestToCoors((x,y,z), 1, 999999.0, 1, 0, 0, 0, 0)
        // (02C1: (x,y,z), 0, 999999.0, 0, 1, 0, 0, 0). 02C0: `(u16)a == 0xFFFF` -> not found, else the
        // position of node `m_pPathNodes[(u16)a] + (a >> 16) * 0x1C` (CPathNode::GetPosition). 02C1:
        // FindNodeCoorsForScript(a, &found) and its vector when found. Found: P0..P2 = the vector, flag 1;
        // else P0..P2 = 0, flag 0; StoreParameters(3) before UpdateCompareFlag.
        0x02C0 | 0x02C1 => {
            collect(s, 3);
            let (x, y) = (sp(0), sp(1));
            let z = ground_rule(x, y, sp(2));
            let paths = GPtr::from_addr(THE_PATHS);
            let mut ret = [0u32; 1];
            let rp = GPtr::from_raw(ret.as_mut_ptr());
            let mut out = [0u32; 3];
            let op = GPtr::from_raw(out.as_mut_ptr());
            let found = if command == 0x02C0 {
                let a = find_node_closest_to_coors(paths, rp, x, y, z, 1, K_999999, 1, 0, 0, 0, 0).read();
                if a & 0xFFFF == 0xFFFF {
                    false
                } else {
                    let base = GPtr::<u32>::from_addr(PATH_NODES.wrapping_add(((a & 0xFFFF) * 4) as usize)).read();
                    let node = GPtr::<u8>::from_addr(base.wrapping_add((a >> 16).wrapping_mul(0x1C)) as usize);
                    path_node_get_position(node, op);
                    true
                }
            } else {
                let a = find_node_closest_to_coors(paths, rp, x, y, z, 0, K_999999, 0, 1, 0, 0, 0).read();
                let mut f = [0u8; 1];
                find_node_coors_for_script(paths, op, a, GPtr::from_raw(f.as_mut_ptr()));
                f[0] != 0
            };
            if found {
                store_node(s, out, true);
            } else {
                store_node(s, [0; 3], false);
            }
        }
        // 02C2 CAR_GOTO_COORDINATES_ACCURATE (0x47FCA7, gr: unregistered): no null check; -100 ground rule
        // on z, then `+ GetDistanceFromCentreOfMassToBaseOfModel` (ST0, z as memory operand);
        // JoinCarWithRoadSystemGotoCoors(veh, &(x,y,z), 0, 0): true -> mission 0xD, false -> 0xC (unless
        // 0x39/0x3A); car_goto_tail.
        0x02C2 => {
            collect(s, 4);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let (x, y) = (sp(1), sp(2));
            let zl = ground_rule(x, y, sp(3));
            let d = X87::from_f64_exact(get_distance_from_centre_of_mass_to_base_of_model(veh));
            let pos = [x, y, d.fadd_m(f(zl)).to_f32().to_bits()];
            let ok = join_car_with_road_system_goto_coors(veh, vec_ptr(&pos), 0, 0) != 0;
            set_car_mission(veh, if ok { 0xD } else { 0xC });
            car_goto_tail(veh);
        }
        // 02CA IS_CAR_ON_SCREEN (0x47FDA1, gr: unregistered), 02CB IS_CHAR_ON_SCREEN (0x47FDDD), 02CC
        // IS_OBJECT_ON_SCREEN (0x47FE34, gr: unregistered): see `on_screen`.
        0x02CA => {
            collect(s, 1);
            on_screen(s, vehicle_at_ref(pool(VEHICLE_POOL), spi(0)));
        }
        0x02CB => {
            collect(s, 1);
            on_screen(s, ped_at_ref(pool(PED_POOL), spi(0)));
        }
        0x02CC => {
            collect(s, 1);
            on_screen(s, object_at_ref(pool(OBJECT_POOL), spi(0)));
        }
        // 02CF START_SCRIPT_FIRE (0x47FEEC): CollectParameters(5); -100 ground rule on z; P0 =
        // gFireManager.StartScriptFire(&(x,y,z), nullptr, 0.8f, 1, (u8)P3, P4); StoreParameters(1).
        0x02CF => {
            collect(s, 5);
            let (x, y) = (sp(0), sp(1));
            let pos = [x, y, ground_rule(x, y, sp(2))];
            let h = start_script_fire(
                GPtr::from_addr(FIRE_MANAGER),
                vec_ptr(&pos),
                GPtr::NULL,
                0x3F4C_CCCD,
                1,
                sp(3) & 0xFF,
                sp(4),
            );
            set_sp(0, h as u32);
            store(s, 1);
        }
        // 02D3 BOAT_GOTO_COORDS (0x480008, gr: unregistered): no null check. Only when z <= -100
        // (ordered) CWaterLevel::GetWaterLevel(x, y, z, &level, 1, 0) runs; the target z is `level`,
        // the frame local the prologue zeroes (`[esp+0x54] = 0`), so a script z > -100 gives target z 0.0
        // (original bug kept). Mission 0xE (unless 0x39/0x3A); m_vecDestinationCoors (+0x3EC..+0x3F4) =
        // (x, y, level via `fld; fstp`); car_goto_tail.
        0x02D3 => {
            collect(s, 4);
            let boat = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let (x, y, z) = (sp(1), sp(2), sp(3));
            let mut level = [0u32; 1];
            if !X87::from_f32(f(z)).fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
                get_water_level(x, y, fld_fst(f(z)).to_bits(), GPtr::from_raw(level.as_mut_ptr()), 1, 0);
            }
            set_car_mission(boat, 0xE);
            boat.byte_add(0x3F4).cast::<u32>().write(fld_fst(f(level[0])).to_bits());
            boat.byte_add(0x3F0).cast::<u32>().write(y);
            boat.byte_add(0x3EC).cast::<u32>().write(x);
            car_goto_tail(boat);
        }
        // 02D4 BOAT_STOP (0x4800E5, gr: unregistered): no null check; mission 0 (unless 0x39/0x3A);
        // status PHYSICS; byte +0x428 &= 0xEF; cruise speed (+0x3D0) = 0.
        0x02D4 => {
            collect(s, 1);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            set_car_mission(v, 0);
            let st = v.byte_add(0x36);
            st.write((st.read() & 7) | 0x18);
            let fl = v.byte_add(0x428);
            fl.write(fl.read() & 0xEF);
            v.byte_add(0x3D0).write(0);
        }
        // 02D6 IS_CHAR_SHOOTING_IN_AREA (0x480148): no null check; flag = bFiringWeapon (byte +0x46E bit 0)
        // && IsWithinArea(P1, P2, P3, P4) != 0; then, when P5 != 0, HighlightImportantArea(this + m_IP,
        // P1..P4, -100.0); DrawDebugSquare(P1..P4) when CTheScripts::DbgFlag (read from 0x859CF8).
        0x02D6 => {
            collect(s, 6);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let b = ped.byte_add(0x46E).read() & 1 != 0 && is_within_area(ped, sp(1), sp(2), sp(3), sp(4)) != 0;
            set_flag(s, b);
            if sp(5) != 0 {
                let id = s.byte_add(crate::vm::IP).cast::<u32>().read().wrapping_add(s.addr() as u32);
                highlight_important_area(id, sp(1), sp(2), sp(3), sp(4), K_MINUS_100);
            }
            if GPtr::<u8>::from_addr(DBG_FLAG).read() != 0 {
                draw_debug_square(sp(1), sp(2), sp(3), sp(4));
            }
        }
        // 02D8 IS_CURRENT_CHAR_WEAPON (0x48021D): no null check; w = the active weapon (+0x5A0 + 0x1C *
        // (int8)+0x718); flag = (P1 == 0x38 && CWeapon::IsTypeMelee(w)) || w.m_eWeaponType == P1.
        0x02D8 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let w = || {
                let slot = i32::from(ped.byte_add(0x718).read() as i8);
                ped.byte_add(0x5A0 + slot.wrapping_mul(0x1C) as isize)
            };
            let b = (sp(1) == 0x38 && weapon_is_type_melee(w()) != 0) || w().cast::<u32>().read() == sp(1);
            set_flag(s, b);
        }
        0x02DD => get_random_char_in_zone(s),
        // 02E1 CREATE_MONEY_PICKUP (0x48056A, gr: unregistered): CollectParameters(5); when z <= -100
        // (ordered) z = FindGroundZForCoord(x, y) `fadd [0.5]`; discarded GetActualPickupIndex of the
        // output operand; P0 = CPickups::GenerateNewOne((x,y,z), (u16)MI_MONEY, P4 != 0 ? 0x13 : 8, P3,
        // 0, 0, 0); StoreParameters(1).
        0x02E1 => {
            collect(s, 5);
            let (x, y) = (sp(0), sp(1));
            let mut z = fld_fst(spf(2)).to_bits();
            if !X87::from_f32(spf(2)).fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
                z = X87::from_f64_exact(crate::cmdlib::find_ground_z_for_coord(f(x), f(y)))
                    .fadd_m(f(K_HALF))
                    .to_f32()
                    .to_bits();
            }
            get_actual_pickup_index(collect_next_parameter_without_increasing_pc(s));
            let ty = if sp(4) != 0 { 0x13 } else { 8 };
            let model = u32::from(GPtr::<u16>::from_addr(MI_MONEY).read());
            let h = generate_new_one(x, y, z, model, ty, sp(3), 0, 0, 0);
            set_sp(0, h as u32);
            store(s, 1);
        }
        // 02E4 LOAD_CUTSCENE (0x4806FB, gr: unregistered): CCutsceneMgr::LoadCutsceneData(label) (zeroed
        // 16-byte buffer, see `label8`).
        0x02E4 => {
            let mut buf = [0u8; 16];
            load_cutscene_data(label8(s, &mut buf));
        }
        // 02E5 CREATE_CUTSCENE_OBJECT, 02E6 SET_CUTSCENE_ANIM, 02F3 LOAD_SPECIAL_MODEL, 0319
        // SET_CHAR_RUNNING (0x480EF1, gr: unimplemented): straight to the epilogue, operands not read.
        0x02E5 | 0x02E6 | 0x02F3 | 0x0319 => {}
        // 02EE IS_PROJECTILE_IN_AREA (0x4807FF, gr: unregistered): CollectParameters(6); per axis the two
        // bounds are swapped when the first is greater (ordered, `test ah,0x41; jne`): the low one takes
        // the raw dword of the other, the high one the `fld; fstp` copy. flag =
        // IsProjectileInRange(x1, x2, y1, y2, z1, z2, 0) != 0; DrawDebugCube(x1, y1, z1, x2, y2, z2) when
        // DbgFlag.
        0x02EE => {
            collect(s, 6);
            let order = |a: u32, b: u32| {
                if X87::from_f32(f(a)).fcom(X87::from_f32(f(b))).test_any(0x41) {
                    (a, b)
                } else {
                    (b, fld_fst(f(a)).to_bits())
                }
            };
            let (x1, x2) = order(sp(0), sp(3));
            let (y1, y2) = order(sp(1), sp(4));
            let (z1, z2) = order(sp(2), sp(5));
            let b = is_projectile_in_range(x1, x2, y1, y2, z1, z2, 0) != 0;
            set_flag(s, b);
            if GPtr::<u8>::from_addr(DBG_FLAG).read() != 0 {
                draw_debug_cube(x1, y1, z1, x2, y2, z2);
            }
        }
        // 02F6 SIN (0x480948) / 02F7 COS (0x48097D): P0 = fsin/fcos(`fld P0; fmul [0.017453292]`) (FSIN/FCOS
        // modelled at 53 bits, accepted project-wide); StoreParameters(1).
        0x02F6 | 0x02F7 => {
            collect(s, 1);
            let a = X87::from_f32(spf(0)).fmul_m(f(K_DEG2RAD));
            let r = if command == 0x02F6 { a.fsin() } else { a.fcos() };
            set_sp(0, r.to_f32().to_bits());
            store(s, 1);
        }
        // 02F8 GET_CAR_FORWARD_X (0x4809B2) / 02F9 GET_CAR_FORWARD_Y (0x480A04), gr: unregistered: no null
        // check, `m_matrix` (+0x14) not checked; fx/fy = forward (+0x10/+0x14); P0 = (fx or fy) /
        // fsqrt(fy*fy + fx*fx) (registers, PC=24); StoreParameters(1).
        0x02F8 | 0x02F9 => {
            collect(s, 1);
            let m = vehicle_at_ref(pool(VEHICLE_POOL), spi(0)).byte_add(0x14).cast::<GPtr<u8>>().read();
            let fx = X87::from_f32(f(m.byte_add(0x10).cast::<u32>().read()));
            let fy = X87::from_f32(f(m.byte_add(0x14).cast::<u32>().read()));
            let len = fy.fmul(fy).fadd(fx.fmul(fx)).fsqrt();
            let num = if command == 0x02F8 { fx } else { fy };
            set_sp(0, len.fdivr(num).to_f32().to_bits());
            store(s, 1);
        }
        // 02FA CHANGE_GARAGE_TYPE (0x480A57, gr: unregistered): id = GetGarageNumberByName(label) (zeroed
        // 16-byte buffer, see `label8`); CollectParameters(1); when (int16)id >= 0
        // ChangeGarageType(id (EAX as returned), (u8)P0, 0).
        0x02FA => {
            let mut buf = [0u8; 16];
            let id = get_garage_number_by_name(label8(s, &mut buf));
            collect(s, 1);
            if (id as u16 as i16) >= 0 {
                change_garage_type(id, sp(0) & 0xFF, 0);
            }
        }
        // 02FD PRINT_WITH_2_NUMBERS_NOW (0x480AAE), 02FF PRINT_WITH_3_NUMBERS (0x480B26), 0302
        // PRINT_WITH_4_NUMBERS (0x480B88), 0303 PRINT_WITH_4_NUMBERS_NOW (0x480BEF), 0308
        // PRINT_WITH_6_NUMBERS (0x480C70): t = TheText.Get(label) (zeroed 16-byte buffer); CollectParameters(n
        // + 2); AddMessage{JumpQ (_NOW), }WithNumber(t, P[n], (u16)P[n+1], P0..P[n-1], -1 for the
        // rest, bAddNextMessageToPreviousBriefs != 0); then bAddNextMessageToPreviousBriefs = 1.
        0x02FD | 0x02FF | 0x0302 | 0x0303 | 0x0308 => {
            let mut buf = [0u8; 16];
            let t = text_get(GPtr::from_addr(THE_TEXT), label8(s, &mut buf));
            let n: i32 = if command == 0x02FD {
                2
            } else if command == 0x02FF {
                3
            } else if command == 0x0308 {
                6
            } else {
                4
            };
            collect(s, (n + 2) as u32);
            let prev = GPtr::<u8>::from_addr(ADD_NEXT_MESSAGE_TO_PREVIOUS_BRIEFS);
            let p = u32::from(prev.read() != 0);
            let num = |i: i32| if i < n { sp(i) } else { 0xFFFF_FFFF };
            let (time, flag) = (sp(n), sp(n + 1) & 0xFFFF);
            if command == 0x02FD || command == 0x0303 {
                add_message_jump_q_with_number(t, time, flag, num(0), num(1), num(2), num(3), num(4), num(5), p);
            } else {
                add_message_with_number(t, time, flag, num(0), num(1), num(2), num(3), num(4), num(5), p);
            }
            prev.write(1);
        }
        // 0318 REGISTER_MISSION_PASSED (0x480D75, gr: unregistered): TheText.Get(label) (result discarded;
        // zeroed 16-byte buffer); strncpy(CStats::LastMissionPassedName, label, 8); IncrementStat(0x93,
        // 1.0); IncrementStat(0xB1, GetStatValue(0xB0)) (the ST0 result `fstp`ed into a dead argument slot
        // that becomes the pushed argument); SetStatValue(0xB0, 0.0); LastMissionPassedTime =
        // CTimer::m_snTimeInMilliseconds.
        0x0318 => {
            let mut buf = [0u8; 16];
            let l = label8(s, &mut buf);
            text_get(GPtr::from_addr(THE_TEXT), l);
            crate::vm::strncpy(GPtr::from_addr(LAST_MISSION_PASSED_NAME), l, 8);
            increment_stat(0x93, 0x3F80_0000);
            let v = X87::from_f64_exact(get_stat_value(0xB0)).to_f32().to_bits();
            increment_stat(0xB1, v);
            set_stat_value(0xB0, 0);
            GPtr::<u32>::from_addr(LAST_MISSION_PASSED_TIME).write(sa_core::timer::TIME_MS.get());
        }
        // 031D HAS_CHAR_BEEN_DAMAGED_BY_WEAPON (0x480E14) / 031E HAS_CAR_BEEN_DAMAGED_BY_WEAPON (0x480E8A,
        // gr: unregistered): null -> 0; w = (int8)last damage weapon (ped +0x760, vehicle +0x508); P1 0x38
        // or 0x39 -> CDarkel::CheckDamagedWeaponType(w, P1) != 0; else w == P1.
        0x031D | 0x031E => {
            collect(s, 2);
            let (e, off) = if command == 0x031D {
                (ped_at_ref(pool(PED_POOL), spi(0)), 0x760)
            } else {
                (vehicle_at_ref(pool(VEHICLE_POOL), spi(0)), 0x508)
            };
            let b = if e.is_null() {
                false
            } else {
                let w = i32::from(e.byte_add(off).read() as i8);
                let want = spi(1);
                if want == 0x38 || want == 0x39 {
                    check_damaged_weapon_type(w, want) != 0
                } else {
                    w == want
                }
            };
            set_flag(s, b);
        }
        _ => return -1,
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_700_TO_799];
