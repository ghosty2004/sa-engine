//! `CRunningScript::ProcessCommands1100To1199` (0x48A320): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands1100To1199.md`.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x0453, 0x0454, 0x045C, 0x0460, 0x0463, 0x0465, 0x0466, 0x0467, 0x0468, 0x0470, 0x0471, 0x0472, 0x0473,
    0x0474, 0x0475, 0x0476, 0x0477, 0x0484, 0x0485, 0x0488, 0x048A, 0x048B, 0x048F, 0x0491, 0x04A3, 0x04A4,
    0x04A7, 0x04A9, 0x04AB, 0x04AD, 0x04AE, 0x04AF,
];

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
}

/// `fld P; fmul [0.017453292]; fstp` (PC=24).
#[inline]
fn deg2rad(v: u32) -> f32 {
    X87::from_f32(f(v)).fmul_m(f(K_DEG2RAD)).to_f32()
}

/// `CRunningScript::ProcessCommands1100To1199(int32 command)` (0x48A320, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x48A320, conv = thiscall, name = "CRunningScript::ProcessCommands1100To1199", expect = "81 EC 84 00 00 00", this = "CRunningScript", fallback)]
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
        _ => {
            return fallback(
                process_commands_1100_to_1199_original,
                "CRunningScript::ProcessCommands1100To1199",
                s,
                command,
            );
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_1100_TO_1199];
