//! `CRunningScript::ProcessCommands700To799` (0x47FA30): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands700To799.md`.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x02BF, 0x02CE, 0x02D0, 0x02D1, 0x02DB, 0x02E0, 0x02E2, 0x02E3, 0x02E7, 0x02E8, 0x02E9, 0x02EA, 0x02EB,
    0x02EC, 0x02ED, 0x02F2, 0x030C, 0x030D, 0x0317, 0x031A,
];

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
}

/// `CRunningScript::ProcessCommands700To799(int32 command)` (0x47FA30, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x47FA30, conv = thiscall, name = "CRunningScript::ProcessCommands700To799", expect = "81 EC D8 00 00 00", this = "CRunningScript", fallback)]
pub fn process_commands_700_to_799(s: Scr, command: u32) -> i8 {
    // The prologue's `mov dword [esp+0x54], 0` initialises a frame local none of the ported cases read.
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
        _ => {
            return fallback(
                process_commands_700_to_799_original,
                "CRunningScript::ProcessCommands700To799",
                s,
                command,
            );
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_700_TO_799];
