//! `CRunningScript::ProcessCommands600To699` (0x47F370): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands600To699.md`.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x0293, 0x0294, 0x0296, 0x0297, 0x0298, 0x02A0, 0x02A3, 0x02A7, 0x02A8, 0x02A9, 0x02AA, 0x02AB, 0x02AC,
];

/// `CTheScripts::ScriptResourceManager` 0xA485A8.
const SCRIPT_RESOURCE_MANAGER: usize = 0xA485A8;

/// `.rdata` 0x859014: -100.0f
const K_MINUS_100: u32 = 0xC2C8_0000;

orig! {
    /// `CScriptResourceManager::RemoveFromResourceManager(int32, uint32 type, CRunningScript*)` (0x470510), AL
    fn remove_from_resource_manager(m: GPtr<u8>, model: i32, ty: u32, s: Scr) -> u8 = thiscall 0x470510 => unported;
    /// `CStreaming::SetMissionDoesntRequireSpecialChar(int32 slot)` (0x40B490)
    fn set_mission_doesnt_require_special_char(slot: i32) = cdecl 0x40B490 => unported;
    /// `CDarkel::ResetModelsKilledByPlayer(int32 player)` (0x43D6A0)
    fn reset_models_killed_by_player(player: i32) = cdecl 0x43D6A0 => unported;
    /// `CDarkel::QueryModelsKilledByPlayer(int32 model, int32 player)` (0x43D6C0), AX
    fn query_models_killed_by_player(model: i32, player: i32) -> u16 = cdecl 0x43D6C0 => unported;
    /// `CTheScripts::IsPedStopped(CPed*)` (0x486110), AL
    fn is_ped_stopped(p: GPtr<u8>) -> u8 = cdecl 0x486110 => unported;
    /// `CRunningScript::CollectNextParameterWithoutIncreasingPC()` (0x464250)
    fn collect_next(s: Scr) -> i32 = thiscall 0x464250 => crate::vm::collect_next_parameter_without_increasing_pc;
    /// `CCamera::SetWideScreenOn()` (0x50C140)
    fn set_wide_screen_on(cam: GPtr<u8>) = thiscall 0x50C140 => unported;
    /// `CCamera::SetWideScreenOff()` (0x50C150)
    fn set_wide_screen_off(cam: GPtr<u8>) = thiscall 0x50C150 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), floats as raw dwords, ST0 (read as f64: exact)
    fn find_ground_z_for_coord(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CRadar::GetActualBlipArrayIndex(int32)` (0x582870); the result is discarded here
    fn get_actual_blip_array_index(h: i32) -> i32 = cdecl 0x582870 => unported;
    /// `CRadar::SetCoordBlip(eBlipType, CVector, uint32 colour, eBlipDisplay, const char*)` (0x583820);
    /// the by-value vector as three raw dwords
    fn set_coord_blip(ty: i32, x: u32, y: u32, z: u32, colour: u32, display: u32, name: GPtr<u8>) -> i32 = cdecl 0x583820 => unported;
    /// `CRadar::SetBlipSprite(int32, int32)` (0x583D70)
    fn set_blip_sprite(h: i32, sprite: i32) = cdecl 0x583D70 => unported;
}

/// 02A7 / 02A8 body: `fld z; fst local; fcomp -100.0; test ah,0x41; jp`: when `z <= -100` (ordered)
/// local z = FindGroundZForCoord(x, y) (`fstp`); discarded GetActualBlipArrayIndex of the output
/// operand; h = SetCoordBlip(ty, (x, y, z), colour, 3 BOTH, m_szName); SetBlipSprite(h, P3); P0 = h;
/// StoreParameters(1) (shared tail 0x47F7FB).
fn sprite_blip_for_coord(s: Scr, ty: i32, colour: u32) {
    collect(s, 4);
    let (x, y) = (sp(0), sp(1));
    let st = X87::from_f32(f(sp(2)));
    let mut z = st.to_f32().to_bits();
    if !st.fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
        z = X87::from_f64_exact(find_ground_z_for_coord(x, y))
            .to_f32()
            .to_bits();
    }
    get_actual_blip_array_index(collect_next(s));
    let h = set_coord_blip(ty, x, y, z, colour, 3, s.byte_add(8));
    set_blip_sprite(h, spi(3));
    set_sp(0, h as u32);
    store(s, 1);
}

/// `e->m_nPhysicalFlags` (+0x40) bit `mask` = `v != 0` (one read-modify-write per flag, as compiled).
#[inline]
fn set_physical_flag(e: GPtr<u8>, mask: u32, v: u32) {
    let p = e.byte_add(0x40).cast::<u32>();
    let old = p.read();
    p.write(if v != 0 { old | mask } else { old & !mask });
}

/// 02AB / 02AC body (shared tail 0x47F8AA): bullet (0x40000) = P1, fire (0x80000) = P2, explosion
/// (0x800000) = P3, collision (0x100000) = P4, melee (0x200000) = P5. No null check.
fn set_proofs(e: GPtr<u8>) {
    set_physical_flag(e, 0x40000, sp(1));
    set_physical_flag(e, 0x80000, sp(2));
    set_physical_flag(e, 0x800000, sp(3));
    set_physical_flag(e, 0x100000, sp(4));
    set_physical_flag(e, 0x200000, sp(5));
}

/// `CRunningScript::ProcessCommands600To699(int32 command)` (0x47F370, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x47F370, conv = thiscall, name = "CRunningScript::ProcessCommands600To699", expect = "8B 44 24 04 83 EC 2C", this = "CRunningScript", fallback)]
pub fn process_commands_600_to_699(s: Scr, command: u32) -> i8 {
    match command {
        // 0293 GET_CONTROLLER_MODE (0x47F399): P0 = 0; StoreParameters(1).
        0x0293 => {
            set_sp(0, 0);
            store(s, 1);
        }
        // 0294 SET_CAN_RESPRAY_CAR (0x47F3B8): no null check; byte +0x868 bit 1 (0x2) = P1 == 0.
        0x0294 => {
            collect(s, 2);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let p = veh.byte_add(0x868);
            let b = p.read();
            p.write(if sp(1) != 0 { b & 0xFD } else { b | 0x2 });
        }
        // 0296 UNLOAD_SPECIAL_CHARACTER (0x47F40D): ScriptParams[0] is decremented in place; when
        // ScriptResourceManager.RemoveFromResourceManager(P0 + 0x122, 2, this) returns true,
        // SetMissionDoesntRequireSpecialChar(P0) (re-read).
        0x0296 => {
            collect(s, 1);
            let slot = sp(0).wrapping_sub(1);
            set_sp(0, slot);
            let m = slot.wrapping_add(0x122) as i32;
            if remove_from_resource_manager(GPtr::from_addr(SCRIPT_RESOURCE_MANAGER), m, 2, s) != 0 {
                set_mission_doesnt_require_special_char(spi(0));
            }
        }
        // 0297 RESET_NUM_OF_MODELS_KILLED_BY_PLAYER (0x47F457)
        0x0297 => {
            collect(s, 1);
            reset_models_killed_by_player(spi(0));
        }
        // 0298 GET_NUM_OF_MODELS_KILLED_BY_PLAYER (0x47F47A): P0 = (int16)QueryModelsKilledByPlayer(P1, P0)
        // (`movsx`).
        0x0298 => {
            collect(s, 2);
            let n = query_models_killed_by_player(spi(1), spi(0));
            set_sp(0, i32::from(n as i16) as u32);
            store(s, 1);
        }
        // 02A0 IS_CHAR_STOPPED (0x47F63A): flag = CTheScripts::IsPedStopped(GetAtRef(P0)) != 0.
        0x02A0 => {
            collect(s, 1);
            let b = is_ped_stopped(ped_at_ref(pool(PED_POOL), spi(0))) != 0;
            set_flag(s, b);
        }
        // 02A3 SWITCH_WIDESCREEN (0x47F684)
        0x02A3 => {
            collect(s, 1);
            let cam = GPtr::from_addr(THE_CAMERA);
            if sp(0) != 0 {
                set_wide_screen_on(cam);
            } else {
                set_wide_screen_off(cam);
            }
        }
        // 02A7 ADD_SPRITE_BLIP_FOR_CONTACT_POINT (0x47F6BD): SetCoordBlip(5 CONTACT_POINT, .., colour 2, 3).
        0x02A7 => sprite_blip_for_coord(s, 5, 2),
        // 02A8 ADD_SPRITE_BLIP_FOR_COORD (0x47F75D): SetCoordBlip(4 COORD, .., colour 5, 3).
        0x02A8 => sprite_blip_for_coord(s, 4, 5),
        // 02A9 SET_CHAR_ONLY_DAMAGED_BY_PLAYER (0x47F821) / 02AA SET_CAR_ONLY_DAMAGED_BY_PLAYER
        // (0x47F85E, shared tail 0x47F83C): physical flag 0x400000 = P1; no null check.
        0x02A9 | 0x02AA => {
            collect(s, 2);
            let e = if command == 0x02A9 {
                ped_at_ref(pool(PED_POOL), spi(0))
            } else {
                vehicle_at_ref(pool(VEHICLE_POOL), spi(0))
            };
            set_physical_flag(e, 0x400000, sp(1));
        }
        // 02AB SET_CHAR_PROOFS (0x47F88F)
        0x02AB => {
            collect(s, 6);
            set_proofs(ped_at_ref(pool(PED_POOL), spi(0)));
        }
        // 02AC SET_CAR_PROOFS (0x47F908)
        0x02AC => {
            collect(s, 6);
            set_proofs(vehicle_at_ref(pool(VEHICLE_POOL), spi(0)));
        }
        _ => {
            return fallback(
                process_commands_600_to_699_original,
                "CRunningScript::ProcessCommands600To699",
                s,
                command,
            );
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_600_TO_699];
