//! `CRunningScript::ProcessCommands1100To1199` (0x48A320): complete port (PORT-SCRIPT-2 partial, completed
//! in PORT-SCRIPT-3). Every jump-table case is in [`PORTED`]; the other opcodes of the range reach the
//! original default (-1). Cases follow the 1.0 US bytes and
//! `docs/re/specs/SCRIPT/commands/ProcessCommands1100To1199.md`.
//!
//! Divergence notes: the 8-byte label cases (0459, 045A, 045B) read the label into a zeroed 16-byte
//! buffer (the original's is not cleared); 04A5's three out locals start zeroed (the original's are
//! stale stack slots, written by the callee).

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x044D, 0x0450, 0x0451, 0x0453, 0x0454, 0x0457, 0x0458, 0x0459, 0x045A, 0x045B, 0x045C, 0x045D, 0x0460,
    0x0463, 0x0464, 0x0465, 0x0466, 0x0467, 0x0468, 0x0469, 0x046C, 0x046D, 0x046E, 0x0470, 0x0471, 0x0472,
    0x0473, 0x0474, 0x0475, 0x0476, 0x0477, 0x047A, 0x0480, 0x0481, 0x0482, 0x0484, 0x0485, 0x0488, 0x0489,
    0x048A, 0x048B, 0x048C, 0x048F, 0x0491, 0x0493, 0x0494, 0x0495, 0x0496, 0x049C, 0x049E, 0x049F, 0x04A1,
    0x04A2, 0x04A3, 0x04A4, 0x04A5, 0x04A6, 0x04A7, 0x04A9, 0x04AB, 0x04AD, 0x04AE, 0x04AF,
];

/// `.rdata` 0x859014: -100.0f
const K_MINUS_100: u32 = 0xC2C8_0000;
/// `.rdata` 0x858B8C: 0.5f
const K_HALF: u32 = 0x3F00_0000;
/// `.rdata` 0x859F80: 250.0f
const K_250: u32 = 0x437A_0000;
/// `CCamera::m_bUseMouse3rdPerson` (byte) 0xB6EC2E.
const USE_MOUSE_3RD_PERSON: usize = 0xB6EC2E;
/// `TheText` (`CText`) 0xC1B340.
const THE_TEXT: usize = 0xC1B340;
/// `CTheScripts::NumberOfIntroTextLinesThisFrame` (uint16) 0xA44B68.
const NUM_INTRO_TEXT_LINES: usize = 0xA44B68;
/// `CTheScripts::IntroTextLines[0]` fields as the cases address them (0x44 bytes per line): X/Y position
/// (+0xA91414/+0xA91418), the 8-byte key (+0xA9141C), the two numbers (+0xA91424/+0xA91428).
const INTRO_LINE_X: usize = 0xA91414;
const INTRO_LINE_KEY: usize = 0xA9141C;
const INTRO_LINE_N1: usize = 0xA91424;
/// `CPickups::aPickUps` 0x9788C0 (`CPickup`, 0x20 bytes, 620 of them).
const PICKUPS: usize = 0x9788C0;
/// `ModelIndices::MI_MONEY` 0x8CD59C / `MI_PICKUP_REVENUE`-slot (int16) 0x8CD5E0 as CREATE_PROTECTION_PICKUP reads it.
const MI_PROTECTION_PICKUP: usize = 0x8CD5E0;

/// `CTheScripts::FailCurrentMission` (byte) 0xA444B0.
const FAIL_CURRENT_MISSION: usize = 0xA444B0;
/// `.rdata` 0x858628: 100.0f
const K_100: u32 = 0x42C8_0000;
/// `TheCamera` fields read by the debug camera commands: 0xB6F79C..0xB6F7A4 and 0xB6F7A8..0xB6F7B0.
const CAM_FRONT: usize = 0xB6F79C;
const CAM_SOURCE: usize = 0xB6F7A8;
/// `CModelInfo::ms_modelInfoPtrs` 0xA9B0C8.
const MODEL_INFO_PTRS: u32 = 0xA9B0C8;
/// Byte 0xC1CC00 (SET_ENABLE_RC_DETONATE's flag).
const RC_DETONATE_DISABLED: usize = 0xC1CC00;

/// `.rdata` 0x8595EC: 0.017453292f (degrees to radians)
const K_DEG2RAD: u32 = 0x3C8E_FA35;

orig! {
    /// `CCamera::SetParametersForScriptInterpolation(float stopMoving, float catchUp, uint32 time)` (0x50C030);
    /// the floats as raw dwords
    fn set_parameters_for_script_interpolation(cam: GPtr<u8>, stop: u32, catch_up: u32, time: u32) = thiscall 0x50C030 => unported;
    /// `CPed::DettachPedFromEntity()` (0x5E7EC0)
    fn dettach_ped_from_entity(p: GPtr<u8>) = thiscall 0x5E7EC0 => unported;
    /// `CRunningScript::LocateCharObjectCommand(int32 command)` (0x487720)
    fn locate_char_object_command(s: Scr, command: u32) = thiscall 0x487720 => unported;
    /// Unnamed thiscall on the vehicle pool taking the vehicle (0x424160, a `.HOODLUM` stub; the handle
    /// GET_REMOTE_CONTROLLED_CAR stores, i.e. `CPool<CVehicle>::GetRef`)
    fn vehicle_pool_ref_424160(pool: GPtr<u8>, v: GPtr<u8>) -> i32 = thiscall 0x424160 => unported;
    /// `CPed::ClearWeapons()` (0x5E6320)
    fn clear_weapons(p: GPtr<u8>) = thiscall 0x5E6320 => unported;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    fn get_vehicle_appearance(v: GPtr<u8>) -> i32 = thiscall 0x6D1080 => unported;
    /// `CPlaceable::SetOrientation(float x, float y, float z)` (0x439A80)
    fn set_orientation(e: GPtr<u8>, x: f32, y: f32, z: f32) = thiscall 0x439A80 => unported;
    /// `CEntity::UpdateRwMatrix()` (0x446F90)
    fn update_rw_matrix(e: GPtr<u8>) = thiscall 0x446F90 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    fn update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CPlayerPed::GetPadFromPlayer()` (0x609560)
    fn get_pad_from_player(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CPad::GetTarget()` (0x540670), AL
    fn pad_get_target(pad: GPtr<u8>) -> u8 = thiscall 0x540670 => unported;
    /// `CText::Get(const char* key)` (0x6A0050)
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CPed::AttachPedToEntity(CEntity*, CVector offset, uint16 position, float angleLimit, eWeaponType)`
    /// (0x5E7CB0); the vector and float as raw dwords
    fn attach_ped_to_entity(p: GPtr<u8>, e: GPtr<u8>, x: u32, y: u32, z: u32, pos: u32, angle: u32, weapon: u32) = thiscall 0x5E7CB0 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80)
    fn get_peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPedGroupMembership::IsLeader(const CPed*)` (0x5F69C0), AL
    fn membership_is_leader(m: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x5F69C0 => unported;
    /// `CPedGroupMembership::CountMembersExcludingLeader()` (0x5F6AA0), AL
    fn count_members_excluding_leader(m: GPtr<u8>) -> u8 = thiscall 0x5F6AA0 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), floats as raw dwords, ST0 (exact as f64)
    fn find_ground_z_for_coord(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CRemote::GivePlayerRemoteControlledCar(float x, float y, float z, float angle, uint16 model)` (0x45AB10)
    fn give_player_remote_controlled_car(x: u32, y: u32, z: u32, angle: u32, model: u32) = cdecl 0x45AB10 => unported;
    /// `CPed::IsAlive()` (0x5E0170), AL
    fn ped_is_alive(p: GPtr<u8>) -> u8 = thiscall 0x5E0170 => unported;
    /// `CPedGeometryAnalyser::CanPedTargetPed(CPed&, CPed&, bool)` (0x5F1C40), AL
    fn can_ped_target_ped(a: GPtr<u8>, b: GPtr<u8>, c: u32) -> u8 = cdecl 0x5F1C40 => unported;
    /// `CPed::DisablePedSpeech(bool)` (0x5EFF60)
    fn disable_ped_speech(p: GPtr<u8>, stop_now: u32) = thiscall 0x5EFF60 => unported;
    /// `CPed::EnablePedSpeech()` (0x5EFF70)
    fn enable_ped_speech(p: GPtr<u8>) = thiscall 0x5EFF70 => unported;
    /// `CPickup::GetPosn()` (0x4549A0), the vector returned through the hidden pointer
    fn pickup_get_posn(p: GPtr<u8>, out: GPtr<u32>) -> GPtr<u32> = thiscall 0x4549A0 => unported;
    /// `CVector::Magnitude()` (0x4082C0), ST0 (an FSQRT result under PC=24: exact as f64)
    fn vector_magnitude(v: GPtr<u32>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CDamageManager::GetEngineStatus()` (0x6C22C0)
    fn get_engine_status(dm: GPtr<u8>) -> u32 = thiscall 0x6C22C0 => unported;
    /// `CDamageManager::GetWheelStatus(int32)` (0x6C21B0)
    fn get_wheel_status(dm: GPtr<u8>, w: u32) -> u32 = thiscall 0x6C21B0 => unported;
    /// `CAutomobile::TellHeliToGoToCoors(float x, float y, float z, float altMin, float altMax)` (0x6A2390),
    /// floats as raw dwords
    fn tell_heli_to_go_to_coors(v: GPtr<u8>, x: u32, y: u32, z: u32, a: u32, b: u32) = thiscall 0x6A2390 => unported;
    /// `CPed::CreateDeadPedPickupCoors(float&, float&, float&)` (0x459180)
    fn create_dead_ped_pickup_coors(p: GPtr<u8>, x: GPtr<u32>, y: GPtr<u32>, z: GPtr<u32>) = thiscall 0x459180 => unported;
    /// `CPickups::GetActualPickupIndex(int32)` (0x4552A0), result discarded
    fn get_actual_pickup_index(h: i32) -> i32 = cdecl 0x4552A0 => unported;
    /// `CPickups::GenerateNewOne(CVector, uint32 model, ePickupType, uint32 ammo, uint32 moneyPerDay,
    /// bool isEmpty, const char* message)` (0x456F20)
    fn generate_new_one(x: u32, y: u32, z: u32, model: u32, ty: u32, ammo: u32, money: u32, empty: u32, msg: u32) -> i32 = cdecl 0x456F20 => unported;
}

/// The two DISPLAY_TEXT_WITH_*NUMBER* cases (0x48A5B5 / 0x48A660): CollectParameters(2) (x, y);
/// `TheText.Get(label)` (result discarded; the label in a zeroed 16-byte buffer, divergence note: the
/// original's is not cleared, an 8-character label is followed by stale stack bytes); line =
/// `NumberOfIntroTextLinesThisFrame` (re-read at each use): X/Y = P0/P1; CollectParameters(n);
/// `strncpy(key, label, 8)`; number 1 = P0, number 2 = P1 (n = 2) or -1; the counter + 1 (16-bit).
fn display_text_with_numbers(s: Scr, n: u32) {
    collect(s, 2);
    let mut buf = [0u8; 16];
    let l = GPtr::from_raw(buf.as_mut_ptr());
    read_text_label(s, l, 8);
    text_get(GPtr::from_addr(THE_TEXT), l);
    let cnt = GPtr::<u16>::from_addr(NUM_INTRO_TEXT_LINES);
    let line = |off: usize| GPtr::<u32>::from_addr(off.wrapping_add(usize::from(cnt.read()) * 0x44));
    line(INTRO_LINE_X).write(sp(0));
    line(INTRO_LINE_X + 4).write(sp(1));
    collect(s, n);
    crate::vm::strncpy(line(INTRO_LINE_KEY).cast(), l, 8);
    let c = cnt.read();
    line(INTRO_LINE_N1).write(sp(0));
    line(INTRO_LINE_N1 + 4).write(if n == 2 { sp(1) } else { 0xFFFF_FFFF });
    cnt.write(c.wrapping_add(1));
}

/// The ped's vehicle when `m_nPedFlags` bit 8 (+0x46C) is set, else null (no null check on the ped).
fn vehicle_if_in_one(ped: GPtr<u8>) -> GPtr<u8> {
    if ped.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0 {
        ped.byte_add(0x58C).cast::<GPtr<u8>>().read()
    } else {
        GPtr::NULL
    }
}

/// CREATE_PROTECTION_PICKUP's coordinates rule (as 02E1): z = `FindGroundZForCoord(x, y) + 0.5` (x87)
/// when `z <= -100` (ordered), else the `fld; fst` copy of z.
fn pickup_z(x: u32, y: u32, z: u32) -> u32 {
    if X87::from_f32(f(z)).fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
        fld_fst(f(z)).to_bits()
    } else {
        X87::from_f64_exact(find_ground_z_for_coord(x, y)).fadd_m(f(K_HALF)).to_f32().to_bits()
    }
}

/// `fld P; fmul [0.017453292]; fstp` (PC=24).
#[inline]
fn deg2rad(v: u32) -> f32 {
    X87::from_f32(f(v)).fmul_m(f(K_DEG2RAD)).to_f32()
}

/// `CRunningScript::ProcessCommands1100To1199(int32 command)` (0x48A320, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x48A320, conv = thiscall, name = "CRunningScript::ProcessCommands1100To1199", expect = "81 EC 84 00 00 00", this = "CRunningScript")]
pub fn process_commands_1100_to_1199(s: Scr, command: u32) -> i8 {
    match command {
        // 0454 GET_DEBUG_CAMERA_COORDINATES (0x48A3D2): P0..P2 = the three dwords at 0xB6F7A8 (raw);
        // StoreParameters(3).
        0x0454 => {
            let c = GPtr::<u32>::from_addr(CAM_SOURCE);
            let (x, y, z) = (c.read(), c.wrapping_index(1).read(), c.wrapping_index(2).read());
            set_sp(2, z);
            set_sp(0, x);
            set_sp(1, y);
            store(s, 3);
        }
        // 045C FAIL_CURRENT_MISSION (0x48A70D): CTheScripts::FailCurrentMission = 2.
        0x045C => GPtr::<u8>::from_addr(FAIL_CURRENT_MISSION).write(2),
        // 0460 SET_INTERPOLATION_PARAMETERS (0x48A723): TheCamera.SetParametersForScriptInterpolation(P0,
        // `100.0 - P0` (x87, memory operand P0), P1).
        0x0460 => {
            collect(s, 2);
            let catch_up = X87::constant(K_100).fsub_m(spf(0)).to_f32().to_bits();
            set_parameters_for_script_interpolation(GPtr::from_addr(THE_CAMERA), sp(0), catch_up, sp(1));
        }
        // 0463 GET_DEBUG_CAMERA_POINT_AT (0x48A766): P[i] = [0xB6F79C + 4i] `fadd` [0xB6F7A8 + 4i];
        // StoreParameters(3).
        0x0463 => {
            let a = GPtr::<u32>::from_addr(CAM_FRONT);
            let b = GPtr::<u32>::from_addr(CAM_SOURCE);
            for i in 0..3 {
                let v = X87::from_f32(f(a.wrapping_index(i).read())).fadd_m(f(b.wrapping_index(i).read()));
                set_sp(i, v.to_f32().to_bits());
            }
            store(s, 3);
        }
        // 0465 DETACH_CHAR_FROM_CAR (0x48A861): null ped or `m_pAttachedTo` (+0xFC) null -> nothing;
        // otherwise DettachPedFromEntity().
        0x0465 => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            if !ped.is_null() && ped.byte_add(0xFC).cast::<u32>().read() != 0 {
                dettach_ped_from_entity(ped);
            }
        }
        // 0466 SET_CAR_STAY_IN_FAST_LANE (0x48A8A8): no null check; byte +0x3DB bit 3 (0x8) = P1 != 0.
        0x0466 => {
            collect(s, 2);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let p = veh.byte_add(0x3DB);
            let b = p.read();
            p.write(if sp(1) != 0 { b | 0x8 } else { b & 0xF7 });
        }
        // 0467 CLEAR_CHAR_LAST_WEAPON_DAMAGE (0x48A902) / 0468 CLEAR_CAR_LAST_WEAPON_DAMAGE (0x48A93B):
        // null -> nothing; byte +0x760 (ped) / +0x508 (vehicle) = 0xFF.
        0x0467 | 0x0468 => {
            collect(s, 1);
            let (e, off) = if command == 0x0467 {
                (ped_at_ref(pool(PED_POOL), spi(0)), 0x760)
            } else {
                (vehicle_at_ref(pool(VEHICLE_POOL), spi(0)), 0x508)
            };
            if !e.is_null() {
                e.byte_add(off).write(0xFF);
            }
        }
        // 0453 SET_OBJECT_ROTATION (0x48A350): no null check. CWorld::Remove(obj) (its argument slot is
        // reused for the next call's third argument, no `add esp`); SetOrientation(P1, P2, P3 each
        // `* 0.017453292`, computed z first); UpdateRwMatrix; UpdateRwFrame; CWorld::Add(obj).
        0x0453 => {
            collect(s, 4);
            let obj = object_at_ref(pool(OBJECT_POOL), spi(0));
            world_remove(obj);
            let z = deg2rad(sp(3));
            let y = deg2rad(sp(2));
            let x = deg2rad(sp(1));
            set_orientation(obj, x, y, z);
            update_rw_matrix(obj);
            update_rw_frame(obj);
            world_add(obj);
        }
        // 0470 GET_CURRENT_CHAR_WEAPON (0x48AACF): no null check. P0 = `m_aWeapons[(int8)
        // m_nActiveWeaponSlot (+0x718)].m_eWeaponType` (+0x5A0, 0x1C each; `movsx`); StoreParameters(1).
        0x0470 => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let slot = ped.byte_add(0x718).read() as i8 as i32;
            let ty = ped
                .byte_add(0x5A0 + slot.wrapping_mul(0x1C) as isize)
                .cast::<u32>()
                .read();
            set_sp(0, ty);
            store(s, 1);
        }
        // 0471..0476 LOCATE_CHAR_*_OBJECT_2D/3D (0x48AB19): LocateCharObjectCommand(command) (ECX holds the
        // command from the prologue).
        0x0471..=0x0476 => locate_char_object_command(s, command),
        // 0477 SET_CAR_TEMP_ACTION (0x48AB30): no null check; byte +0x3BB = (u8)P1; dword +0x3BC =
        // CTimer::m_snTimeInMilliseconds + P2.
        0x0477 => {
            collect(s, 3);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            veh.byte_add(0x3BB).write(sp(1) as u8);
            let t = sa_core::timer::TIME_MS.get().wrapping_add(sp(2));
            veh.byte_add(0x3BC).cast::<u32>().write(t);
        }
        // 0484 GET_REMOTE_CONTROLLED_CAR (0x48AC3E): P0 = -1 when Players[P0].m_pRemoteVehicle (+0xB0) is
        // null, else 0x424160(VehiclePool, it); StoreParameters(1) (tail 0x48A9CB).
        0x0484 => {
            collect(s, 1);
            let v = player_info(sp(0)).byte_add(0xB0).cast::<GPtr<u8>>().read();
            let h = if v.is_null() {
                -1
            } else {
                vehicle_pool_ref_424160(pool(VEHICLE_POOL), v)
            };
            set_sp(0, h as u32);
            store(s, 1);
        }
        // 0485 IS_PC_VERSION (0x48AC7B): UpdateCompareFlag(1).
        0x0485 => set_flag(s, true),
        // 0488 IS_MODEL_AVAILABLE (0x48AC93): flag = CModelInfo::ms_modelInfoPtrs[P0] != null.
        0x0488 => {
            collect(s, 1);
            let a = MODEL_INFO_PTRS.wrapping_add(sp(0).wrapping_mul(4));
            let b = GPtr::<u32>::from_addr(a as usize).read() != 0;
            set_flag(s, b);
        }
        // 048A SET_ENABLE_RC_DETONATE (0x48AD30): byte 0xC1CC00 = P0 == 0.
        0x048A => {
            collect(s, 1);
            GPtr::<u8>::from_addr(RC_DETONATE_DISABLED).write(u8::from(sp(0) == 0));
        }
        // 048B SET_CAR_RANDOM_ROUTE_SEED (0x48AD58): no null check; word +0x45E = (u16)P1.
        0x048B => {
            collect(s, 2);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            veh.byte_add(0x45E).cast::<u16>().write(sp(1) as u16);
        }
        // 048F REMOVE_ALL_CHAR_WEAPONS (0x48AE69): no null check; ClearWeapons().
        0x048F => {
            collect(s, 1);
            clear_weapons(ped_at_ref(pool(PED_POOL), spi(0)));
        }
        // 0491 HAS_CHAR_GOT_WEAPON (0x48AE9A): no null check; flag = some of the 13 weapon slots (+0x5A0,
        // 0x1C each, scanned in order up to the first match) has type P1.
        0x0491 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let want = sp(1);
            let b = (0..13).any(|i| ped.byte_add(0x5A0 + 0x1C * i).cast::<u32>().read() == want);
            set_flag(s, b);
        }
        // 04A3 IS_INT_VAR_EQUAL_TO_CONSTANT (0x48B0DC) / 04A4 IS_INT_LVAR_EQUAL_TO_CONSTANT (0x48B100):
        // p = GetPointerToScriptVariable(2 / 1); CollectParameters(1); flag = *p == P0.
        0x04A3 | 0x04A4 => {
            let p = var_ptr(s, if command == 0x04A3 { 2 } else { 1 });
            collect(s, 1);
            set_flag(s, p.read() == sp(0));
        }
        // 04A7 IS_CHAR_IN_ANY_BOAT (0x48B268), 04A9 IS_CHAR_IN_ANY_HELI (0x48B2B0), 04AB IS_CHAR_IN_ANY_PLANE
        // (0x48B30A): no null check; flag = `m_nPedFlags` bit 8 (in vehicle, +0x46C) &&
        // m_pVehicle (+0x58C)->GetVehicleAppearance() == 4 / 3 / 5.
        0x04A7 | 0x04A9 | 0x04AB => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let want = if command == 0x04A7 {
                4
            } else if command == 0x04A9 {
                3
            } else {
                5
            };
            let b = ped.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0
                && get_vehicle_appearance(ped.byte_add(0x58C).cast::<GPtr<u8>>().read()) == want;
            set_flag(s, b);
        }
        // 04AD IS_CHAR_IN_WATER (0x48B365): flag = ped != null && `m_nPhysicalFlags` (+0x40) bit 8.
        0x04AD => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let b = !ped.is_null() && ped.byte_add(0x40).cast::<u32>().read() & 0x100 != 0;
            set_flag(s, b);
        }
        // 04AE SET_VAR_INT_TO_CONSTANT (0x48B3B1) / 04AF SET_LVAR_INT_TO_CONSTANT (0x48B3DB):
        // p = GetPointerToScriptVariable(2 / 1); CollectParameters(1); *p = P0.
        0x04AE | 0x04AF => {
            let p = var_ptr(s, if command == 0x04AE { 2 } else { 1 });
            collect(s, 1);
            p.write(sp(0));
        }
        // 044D LOAD_SPLASH_SCREEN, 0450 SET_JAMES_CAR_ON_PATH_TO_PLAYER, 0451 LOAD_END_OF_GAME_TUNE, 045D
        // GET_CLOSEST_OBJECT_OF_TYPE, 0469 GET_RANDOM_COP_IN_AREA, 0481/0482 SET_*_RANGE_MULTIPLIER, 0493
        // SET_TANK_DETONATE_CARS, 049C/049E/049F/04A1 *_OBJECT_PATH (0x48B3F7, gr: nop): straight to the
        // epilogue, operands not read, return 0.
        0x044D | 0x0450 | 0x0451 | 0x045D | 0x0469 | 0x0481 => {}
        0x0482 | 0x0493 | 0x049C | 0x049E | 0x049F | 0x04A1 => {}
        // 0457 IS_PLAYER_TARGETTING_CHAR (0x48A40C): no null checks; ped = Players[P0].m_pPed, target =
        // GetAtRef(P1); flag = ped->m_pTargetedObject (+0x71C) is a ped (`(+0x36 & 7) == 3`) and == target;
        // then, when CCamera::m_bUseMouse3rdPerson and `ped->GetPadFromPlayer()->GetTarget()`, flag also
        // set when `m_p3rdPersonMouseTarget` (+0x79C) != 0 and == target.
        0x0457 => {
            collect(s, 2);
            let pi = player_info(sp(0));
            let target = ped_at_ref(pool(PED_POOL), spi(1));
            let ped = pi.cast::<GPtr<u8>>().read();
            let t = ped.byte_add(0x71C).cast::<GPtr<u8>>().read();
            let mut b = !t.is_null() && t.byte_add(0x36).read() & 7 == 3 && t == target;
            if GPtr::<u8>::from_addr(USE_MOUSE_3RD_PERSON).read() != 0 && pad_get_target(get_pad_from_player(ped)) != 0 {
                let m = pi.cast::<GPtr<u8>>().read().byte_add(0x79C).cast::<GPtr<u8>>().read();
                if !m.is_null() && m == target {
                    b = true;
                }
            }
            set_flag(s, b);
        }
        // 0458 IS_PLAYER_TARGETTING_OBJECT (0x48A4A9): flag = Players[P0].m_pPed->m_pTargetedObject is an
        // object (`(+0x36 & 7) == 4`) and == ObjectPool->GetAtRef(P1) (looked up only then).
        0x0458 => {
            collect(s, 2);
            let t = player_info(sp(0)).cast::<GPtr<u8>>().read().byte_add(0x71C).cast::<GPtr<u8>>().read();
            let b = !t.is_null() && t.byte_add(0x36).read() & 7 == 4 && t == object_at_ref(pool(OBJECT_POOL), spi(1));
            set_flag(s, b);
        }
        // 0459 TERMINATE_ALL_SCRIPTS_WITH_THIS_NAME (0x48A503): the label's 8 bytes lowercased ('A'..'Z'
        // + 0x20, all 8 bytes, NUL or not); for each script of pActiveScripts (the next pointer read
        // first) whose `m_szName` (+8) equals it (inlined strcmp: byte by byte up to a mismatch or the
        // NUL): RemoveScriptFromList(&pActiveScripts), AddScriptToList(&pIdleScripts),
        // ShutdownThisScript. Divergence note: zeroed 16-byte label buffer (the original's byte 8
        // onwards are stale stack bytes, compared only for an 8-character name).
        0x0459 => {
            let mut buf = [0u8; 16];
            read_text_label(s, GPtr::from_raw(buf.as_mut_ptr()), 8);
            for c in buf.iter_mut().take(8) {
                if c.is_ascii_uppercase() {
                    *c += 0x20;
                }
            }
            let mut sc = crate::cmd0::ACTIVE_SCRIPTS.get();
            while !sc.is_null() {
                let next = sc.cast::<GPtr<u8>>().read();
                let name = sc.byte_add(8);
                let mut i = 0usize;
                let equal = loop {
                    let a = name.byte_add(i as isize).read();
                    if a != buf.get(i).copied().unwrap_or(0) {
                        break false;
                    }
                    if a == 0 {
                        break true;
                    }
                    i += 1;
                };
                if equal {
                    crate::cmd0::remove_script_from_list(sc, crate::cmd0::ACTIVE_SCRIPTS.ptr());
                    crate::cmd0::add_script_to_list(sc, crate::cmd0::IDLE_SCRIPTS.ptr());
                    crate::cmd0::shutdown_this_script(sc);
                }
                sc = next;
            }
        }
        // 045A DISPLAY_TEXT_WITH_NUMBER (0x48A5B5) / 045B DISPLAY_TEXT_WITH_2_NUMBERS (0x48A660)
        0x045A => display_text_with_numbers(s, 1),
        0x045B => display_text_with_numbers(s, 2),
        // 0464 ATTACH_CHAR_TO_CAR (0x48A7B4): no null checks; AttachPedToEntity(veh, (P2, P3, P4), (u16)P5,
        // `P6 * 0.017453292`, P7); x and y pass through `fld; fstp` (quieted), z is the raw dword.
        0x0464 => {
            collect(s, 8);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(1));
            let (x, y) = (fld_fst(spf(2)).to_bits(), fld_fst(spf(3)).to_bits());
            let a = deg2rad(sp(6)).to_bits();
            attach_ped_to_entity(ped, veh, x, y, sp(4), sp(5) & 0xFFFF, a, sp(7));
        }
        // 046C GET_DRIVER_OF_CAR (0x48A974, gr: unregistered): no null check; P0 = m_pDriver (+0x460) ?
        // PedPool->GetRef(it) : -1; StoreParameters(1).
        0x046C => {
            collect(s, 1);
            let d = vehicle_at_ref(pool(VEHICLE_POOL), spi(0)).byte_add(0x460).cast::<GPtr<u8>>().read();
            let h = if d.is_null() { -1 } else { ped_get_ref(pool(PED_POOL), d) };
            set_sp(0, h as u32);
            store(s, 1);
        }
        // 046D GET_NUMBER_OF_FOLLOWERS (0x48A9E3): g = GetPedsGroup(ped); P0 = (u8)(g &&
        // g->m_groupMembership (+8).IsLeader(ped) ? CountMembersExcludingLeader() : 0); StoreParameters(1).
        0x046D => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let g = get_peds_group(ped);
            let mut n = 0u8;
            if !g.is_null() && membership_is_leader(g.byte_add(8), ped) != 0 {
                n = count_members_excluding_leader(g.byte_add(8));
            }
            set_sp(0, u32::from(n));
            store(s, 1);
        }
        // 046E GIVE_REMOTE_CONTROLLED_MODEL_TO_PLAYER (0x48AA48): CollectParameters(6); -100 ground rule on
        // z (P3); CRemote::GivePlayerRemoteControlledCar(P1, P2, z, `P4 * 0.017453292`, (u16)P5) (P0, the
        // player, is not passed).
        0x046E => {
            collect(s, 6);
            let (x, y) = (sp(1), sp(2));
            let st = X87::from_f32(spf(3));
            let z = if st.fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
                st.to_f32().to_bits()
            } else {
                X87::from_f64_exact(find_ground_z_for_coord(x, y)).to_f32().to_bits()
            };
            let a = deg2rad(sp(4)).to_bits();
            give_player_remote_controlled_car(x, y, z, a, sp(5) & 0xFFFF);
        }
        // 047A IS_CHAR_ON_ANY_BIKE (0x48AB79): no null check; flag = in a vehicle && it != 0 &&
        // GetVehicleAppearance() == 2 (the set-1 path is the out-of-line 0x404CF3).
        0x047A => {
            collect(s, 1);
            let v = vehicle_if_in_one(ped_at_ref(pool(PED_POOL), spi(0)));
            let b = !v.is_null() && get_vehicle_appearance(v) == 2;
            set_flag(s, b);
        }
        // 0480 CAN_CHAR_SEE_DEAD_CHAR (0x48ABDA): CollectParameters(2), P1 unused; no null check; for the 16
        // entities of `m_pIntelligence (+0x47C) + 0x130` in order: a non-null one with `!IsAlive()` and
        // `CPedGeometryAnalyser::CanPedTargetPed(ped, it, true)` sets the flag and stops.
        0x0480 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let arr = ped.byte_add(0x47C).cast::<GPtr<u8>>().read().byte_add(0x130).cast::<GPtr<u8>>();
            let mut b = false;
            for i in 0..16 {
                let e = arr.wrapping_index(i).read();
                if !e.is_null() && ped_is_alive(e) == 0 && can_ped_target_ped(ped, e, 1) != 0 {
                    b = true;
                    break;
                }
            }
            set_flag(s, b);
        }
        // 0489 SHUT_CHAR_UP (0x48ACCD): null ped -> nothing; P1 != 0 -> DisablePedSpeech(false), else
        // EnablePedSpeech().
        0x0489 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            if !ped.is_null() {
                if sp(1) != 0 {
                    disable_ped_speech(ped, 0);
                } else {
                    enable_ped_speech(ped);
                }
            }
        }
        // 048C IS_ANY_PICKUP_AT_COORDS (0x48AD8F, gr: unregistered): over the 620 `aPickUps` while none
        // matched: a pickup with `m_nPickupType` (+0x1C) != 0 matches when `|GetPosn() - (P0, P1, P2)| <
        // 0.5` (ordered; each difference `fsub` stored as f32, P0 as the memory operand, P1/P2 the `fld;
        // fstp` copies; Magnitude ST0).
        0x048C => {
            collect(s, 3);
            let x = spf(0);
            let y = fld_fst(spf(1));
            let z = fld_fst(spf(2));
            let mut b = false;
            for i in 0..620usize {
                if b {
                    break;
                }
                let p = GPtr::<u8>::from_addr(PICKUPS + i * 0x20);
                if p.byte_add(0x1C).read() == 0 {
                    continue;
                }
                let mut out = [0u32; 3];
                let q = pickup_get_posn(p, GPtr::from_raw(out.as_mut_ptr()));
                let d = |k: i32, m: f32| X87::from_f32(f(q.wrapping_index(k).read())).fsub_m(m).to_f32().to_bits();
                let dv = [d(0, x), d(1, y), d(2, z)];
                let m = X87::from_f64_exact(vector_magnitude(GPtr::from_raw(dv.as_ptr().cast_mut())));
                if !m.fcom(X87::constant(K_HALF)).test_parity(5) {
                    b = true;
                }
            }
            set_flag(s, b);
        }
        // 0494 GET_POSITION_OF_ANALOGUE_STICKS (0x48AEDC, gr: unregistered): pad = CPad::GetPad(P0);
        // P0..P3 = the four int16 at pad +0/+2/+4/+6 (`movsx`); StoreParameters(4).
        0x0494 => {
            collect(s, 1);
            let pad = get_pad(spi(0)).cast::<u16>();
            for i in 0..4 {
                set_sp(i, i32::from(pad.wrapping_index(i).read() as i16) as u32);
            }
            store(s, 4);
        }
        // 0495 IS_CAR_ON_FIRE (0x48AF33, gr: unregistered): no null check; flag = m_pFire (+0x490) != 0, or
        // (automobile, +0x590 == 0) `m_damageManager (+0x5A0).GetEngineStatus() >= 225`, or health (+0x4C0)
        // `< 250.0` (ordered, `test ah,5; jp`).
        0x0495 => {
            collect(s, 1);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let mut b = v.byte_add(0x490).cast::<u32>().read() != 0;
            if v.byte_add(0x590).cast::<u32>().read() == 0 && get_engine_status(v.byte_add(0x5A0)) >= 0xE1 {
                b = true;
            }
            let h = X87::from_f32(f(v.byte_add(0x4C0).cast::<u32>().read()));
            if !h.fcom(X87::constant(K_250)).test_parity(5) {
                b = true;
            }
            set_flag(s, b);
        }
        // 0496 IS_CAR_TYRE_BURST (0x48AFBB, gr: unregistered): no null check. Bike (+0x590 == 9): P1 == 4 ->
        // either tyre byte (+0x65C, +0x65D) == 1; else P1 2 -> 0 and 3 -> 1 written back to ScriptParams[1]
        // (original side effect kept), then byte `+0x65C + P1` == 1. Others: P1 == 4 -> any
        // GetWheelStatus(0..3) == 1 (all four called); else GetWheelStatus(P1) == 1.
        0x0496 => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let t = sp(1);
            let b = if v.byte_add(0x590).cast::<u32>().read() == 9 {
                if t == 4 {
                    v.byte_add(0x65C).read() == 1 || v.byte_add(0x65D).read() == 1
                } else {
                    let w = if t == 2 || t == 3 {
                        set_sp(1, t - 2);
                        t - 2
                    } else {
                        t
                    };
                    v.byte_add(0x65C).byte_add(w as i32 as isize).read() == 1
                }
            } else {
                let dm = v.byte_add(0x5A0);
                if t == 4 {
                    let mut any = false;
                    for i in 0..4 {
                        if get_wheel_status(dm, i) == 1 {
                            any = true;
                        }
                    }
                    any
                } else {
                    get_wheel_status(dm, t) == 1
                }
            };
            set_flag(s, b);
        }
        // 04A2 HELI_GOTO_COORDS (0x48B070, gr: unregistered): no null check; TellHeliToGoToCoors(P1, P2, P3,
        // P4, P5).
        0x04A2 => {
            collect(s, 6);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            tell_heli_to_go_to_coors(v, sp(1), sp(2), sp(3), sp(4), sp(5));
        }
        // 04A5 GET_DEAD_CHAR_PICKUP_COORDS (0x48B138): no null check; CreateDeadPedPickupCoors(&x, &y, &z);
        // P0..P2 = x, y, z; StoreParameters(3). The locals start zeroed here (the original's are stale
        // stack slots the callee is expected to write).
        0x04A5 => {
            collect(s, 1);
            let mut o = [0u32; 3];
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let p = GPtr::from_raw(o.as_mut_ptr());
            create_dead_ped_pickup_coors(ped, p, p.wrapping_index(1), p.wrapping_index(2));
            set_sp(0, o[0]);
            set_sp(1, o[1]);
            set_sp(2, o[2]);
            store(s, 3);
        }
        // 04A6 CREATE_PROTECTION_PICKUP (0x48B19D, gr: unregistered): CollectParameters(5); z = ground + 0.5
        // when z <= -100; discarded GetActualPickupIndex of the output operand; P0 =
        // GenerateNewOne((x,y,z), (u16)[0x8CD5E0], 0x10, P3, P4, 0, 0); StoreParameters(1).
        0x04A6 => {
            collect(s, 5);
            let (x, y) = (sp(0), sp(1));
            let z = pickup_z(x, y, sp(2));
            get_actual_pickup_index(collect_next_parameter_without_increasing_pc(s));
            let model = u32::from(GPtr::<u16>::from_addr(MI_PROTECTION_PICKUP).read());
            let h = generate_new_one(x, y, z, model, 0x10, sp(3), sp(4), 0, 0);
            set_sp(0, h as u32);
            store(s, 1);
        }
        _ => return -1,
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_1100_TO_1199];
