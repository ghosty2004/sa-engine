//! `CRunningScript::ProcessCommands1700To1799` (0x496E00): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands1700To1799.md`.
//!
//! Every ported case returns 0. The "protected store" stub 0x4021A4 (`jmp 0x4021A4` at 0x498C8F, anti-tamper
//! code that computes 0xA43C78 and writes EAX there) is ported as `ScriptParams[0] = eax; StoreParameters(1)`.
//! The 0/1 compare scratch byte the original keeps in a stack slot before `UpdateCompareFlag` is not ported.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x06A4, 0x06A6, 0x06AA, 0x06B9, 0x06BD, 0x06C6, 0x06C8, 0x06CB, 0x06CE, 0x06CF, 0x06D0, 0x06D5, 0x06D6,
    0x06D7, 0x06D8, 0x06D9, 0x06DA, 0x06DB, 0x06DF, 0x06E0, 0x06E7, 0x06E8, 0x06E9, 0x06EA, 0x06ED, 0x06F1,
    0x06F2, 0x06F3, 0x06F4, 0x06F6, 0x06F7, 0x06F9, 0x06FA, 0x06FB, 0x06FE, 0x0700, 0x0701, 0x0707,
];

/// `CTheScripts::UsedObjectArray[0].nModelIndex` + 0x1C (the base the `m < 0` model rule indexes from).
const USED_OBJECT_ARRAY_RULE_BASE: u32 = 0xA44B88;

orig! {
    /// `CTrain::CreateMissionTrain(CVector, bool clockwise, uint32 type, CTrain** first, CTrain** last, int32
    /// node, int32 track, bool missionTrain)` (0x6F7550), 10 dwords; the vector as raw dwords
    fn create_mission_train(
        x: u32, y: u32, z: u32, dir: u32, ty: u32, first: GPtr<u32>, last: GPtr<u32>, node: i32, track: i32, mission: u32
    ) = cdecl 0x6F7550 => unported;
    /// `CPool<CVehicle>::GetRef(CVehicle*)` (0x424160), ECX = the vehicle pool
    fn vehicle_get_ref_(pool: GPtr<u8>, v: u32) -> i32 = thiscall 0x424160 => unported;
    /// `CRunningScript::CollectNextParameterWithoutIncreasingPC()` (0x464250)
    fn collect_next(s: Scr) -> i32 = thiscall 0x464250 => crate::vm::collect_next_parameter_without_increasing_pc;
    /// `CTheScripts::GetActualScriptThingIndex(int32 ref, uint8 type)` (0x4839A0)
    fn get_actual_script_thing_index(r: i32, ty: u32) -> i32 = cdecl 0x4839A0 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool x7)` (0x56A490), AL
    fn get_is_line_of_sight_clear(a: GPtr<u32>, b: GPtr<u32>, buildings: u32, vehicles: u32, peds: u32, objects: u32, dummies: u32, see_through: u32, ignore_some: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CTheScripts::AddScriptCheckpoint(CVector at, CVector pointTo, float radius, int32 type)` (0x4935A0)
    fn add_script_checkpoint(x: u32, y: u32, z: u32, px: u32, py: u32, pz: u32, radius: u32, ty: u32) -> i32 = cdecl 0x4935A0 => unported;
    /// `CTheScripts::RemoveScriptCheckpoint(int32)` (0x4936C0)
    fn remove_script_checkpoint(h: i32) = cdecl 0x4936C0 => unported;
    /// `CTrain::DisableRandomTrains(bool)` (0x6F5DB0)
    fn disable_random_trains(b: u32) = cdecl 0x6F5DB0 => unported;
    /// `CVehicle::AddVehicleUpgrade(int32 modelId)` (0x6E3290), EAX
    fn add_vehicle_upgrade(v: GPtr<u8>, m: i32) -> i32 = thiscall 0x6E3290 => unported;
    /// `CVehicle::RemoveVehicleUpgrade(int32 modelId)` (0x6DF930)
    fn remove_vehicle_upgrade(v: GPtr<u8>, m: i32) = thiscall 0x6DF930 => unported;
    /// `CStreaming::RequestVehicleUpgrade(int32 modelId, int32 flags)` (0x408C70)
    fn request_vehicle_upgrade(m: i32, flags: i32) = cdecl 0x408C70 => unported;
    /// `CStreaming::HasVehicleUpgradeLoaded(int32 modelId)` (0x407820), AL
    fn has_vehicle_upgrade_loaded(m: i32) -> u8 = cdecl 0x407820 => unported;
    /// `CTrain::RemoveMissionTrains()` (0x6F6A20)
    fn remove_mission_trains() = cdecl 0x6F6A20 => unported;
    /// `CTrain::ReleaseMissionTrains()` (0x6F6B60)
    fn release_mission_trains() = cdecl 0x6F6B60 => unported;
    /// `CTrain::RemoveAllTrains()` (0x6F6AA0)
    fn remove_all_trains() = cdecl 0x6F6AA0 => unported;
    /// `CPlayerPed::RemovePlayerPed(int32)` (0x6094A0)
    fn remove_player_ped(i: i32) = cdecl 0x6094A0 => unported;
    /// `CCamera::StartCooperativeCamMode()` (0x50C260)
    fn start_cooperative_cam_mode(cam: GPtr<u8>) = thiscall 0x50C260 => unported;
    /// `CVehicle::SetRemap(int32)` (0x6D0C00)
    fn set_remap(v: GPtr<u8>, remap: i32) = thiscall 0x6D0C00 => unported;
}

/// The model id rule of this switch: `m < 0` (a `UsedObjectArray` index) reads
/// `*(int32*)(0xA44B88 - m * 0x1C)` (`imul m,0x1C; sub`).
#[inline]
fn model_id(m: i32) -> i32 {
    if m < 0 {
        GPtr::<i32>::from_addr(
            USED_OBJECT_ARRAY_RULE_BASE.wrapping_sub((m as u32).wrapping_mul(0x1C)) as usize,
        )
        .read()
    } else {
        m
    }
}

#[inline]
fn veh(h: u32) -> GPtr<u8> {
    vehicle_at_ref(pool(VEHICLE_POOL), h as i32)
}

/// `CRunningScript::ProcessCommands1700To1799(int32 command)` (0x496E00, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x496E00, conv = thiscall, name = "CRunningScript::ProcessCommands1700To1799", expect = "6A FF 68 7D 8D 83 00", this = "CRunningScript", fallback)]
pub fn process_commands_1700_to_1799(s: Scr, command: u32) -> i8 {
    match command {
        // 06BD IS_LINE_OF_SIGHT_CLEAR (0x497852)
        0x06BD => is_line_of_sight_clear(s),
        // 06D5 CREATE_CHECKPOINT (0x497E54)
        0x06D5 => create_checkpoint(s),
        // 06D6 DELETE_CHECKPOINT (0x497EF9)
        0x06D6 => delete_checkpoint(s),
        // 06D7 SWITCH_RANDOM_TRAINS (0x497F51): SP0 != 0 pushes 0, else 1
        0x06D7 => {
            collect(s, 1);
            disable_random_trains(u32::from(sp(0) == 0));
        }
        // 06E7 ADD_VEHICLE_MOD (0x4985A6): VEH(SP0) looked up before the model rule; result -> protected store
        0x06E7 => {
            collect(s, 2);
            let v = veh(sp(0));
            let r = add_vehicle_upgrade(v, model_id(spi(1)));
            set_sp(0, r as u32);
            store(s, 1);
        }
        // 06E8 REMOVE_VEHICLE_MOD (0x4985E4)
        0x06E8 => {
            collect(s, 2);
            let v = veh(sp(0));
            remove_vehicle_upgrade(v, model_id(spi(1)));
        }
        // 06E9 REQUEST_VEHICLE_MOD (0x498624)
        0x06E9 => {
            collect(s, 1);
            request_vehicle_upgrade(model_id(spi(0)), 0xC);
        }
        // 06EA HAS_VEHICLE_MOD_LOADED (0x498654): shared compare tail 0x497580/0x497585
        0x06EA => {
            collect(s, 1);
            let r = has_vehicle_upgrade_loaded(model_id(spi(0)));
            set_flag(s, r != 0);
        }
        // 06ED GIVE_VEHICLE_PAINTJOB (0x49870C): no null check
        0x06ED => {
            collect(s, 2);
            let v = veh(sp(0));
            set_remap(v, spi(1));
        }
        // 06D8 CREATE_MISSION_TRAIN (0x497F85): `CreateMissionTrain((P1, P2, P3), dir, P0, &first, nullptr, -1,
        // -1, true)`; P0 = VehiclePool.GetRef(first) (stored through the 0x4021A4 stub), StoreParameters(1).
        // `dir` is the exact dword the original pushes: `setne dl` (P4 != 0) over EDX = P3 (the callee reads
        // bit 0 of the low byte). `first` starts at 0: the original's slot is uninitialised, and
        // CreateMissionTrain writes it on its first carriage whenever the type's config (0x8D44F8 + 0x40 *
        // type) starts with a non-zero model, which holds for all 16 types 0..15 in the exe image (the
        // table is only read); an out-of-range type would leave the original's slot stale.
        0x06D8 => {
            collect(s, 5);
            let dir = (sp(3) & 0xFFFF_FF00) | u32::from(sp(4) != 0);
            let mut first = [0u32; 1];
            create_mission_train(
                sp(1),
                sp(2),
                sp(3),
                dir,
                sp(0),
                GPtr::from_raw(first.as_mut_ptr()),
                GPtr::NULL,
                -1,
                -1,
                1,
            );
            let h = vehicle_get_ref_(pool(VEHICLE_POOL), first[0]);
            set_sp(0, h as u32);
            store(s, 1);
        }
        // 06A4, 06A6, 06C6, 06CB, 06CE, 06F4, 06F6, 06F7, 06F9, 06FB, 06FE, 0700 (0x498EED; gr: nop or
        // unregistered): straight to the epilogue, operands not read, return 0.
        0x06A4 | 0x06A6 | 0x06C6 | 0x06CB | 0x06CE | 0x06F4 => {}
        0x06F6 | 0x06F7 | 0x06F9 | 0x06FB | 0x06FE | 0x0700 => {}
        // 06AA IS_RECORDING_GOING_ON_FOR_CAR (0x4971B8, gr: nop): CollectParameters(1); UpdateCompareFlag(0).
        0x06AA => {
            collect(s, 1);
            set_flag(s, false);
        }
        // 06CF DISPLAY_TIMER_BARS (0x497DB8, gr: nop): CollectParameters(1), nothing else.
        0x06CF => collect(s, 1),
        // 06B9 HAS_CUTSCENE_LOADED (0x4976A2, gr: unregistered): no operands; flag =
        // `CCutsceneMgr::ms_cutsceneLoadStatus` (0xB5F84C) == 2.
        0x06B9 => {
            let b = GPtr::<u32>::from_addr(0xB5F84C).read() == 2;
            set_flag(s, b);
        }
        // 06C8 SET_LA_RIOTS (0x497CDB) / 06D0 SWITCH_EMERGENCY_SERVICES (0x497DC8), gr: unregistered:
        // byte 0xB72958 / 0x8A5B28 = P0 != 0.
        0x06C8 | 0x06D0 => {
            collect(s, 1);
            let a = if command == 0x06C8 { 0xB72958 } else { 0x8A5B28 };
            GPtr::<u8>::from_addr(a).write(u8::from(sp(0) != 0));
        }
        // 06D9 DELETE_MISSION_TRAINS (0x498001), 06DA MARK_MISSION_TRAINS_AS_NO_LONGER_NEEDED (0x49800D),
        // 06DB DELETE_ALL_TRAINS (0x498019), gr: unregistered: CTrain::RemoveMissionTrains /
        // ReleaseMissionTrains / RemoveAllTrains.
        0x06D9 => remove_mission_trains(),
        0x06DA => release_mission_trains(),
        0x06DB => remove_all_trains(),
        // 06DF DELETE_PLAYER (0x4980C9): CPlayerPed::RemovePlayerPed(P0).
        0x06DF => {
            collect(s, 1);
            remove_player_ped(spi(0));
        }
        // 06E0 SET_TWO_PLAYER_CAMERA_MODE (0x4980E7): CollectParameters(1) (unused);
        // TheCamera.StartCooperativeCamMode().
        0x06E0 => {
            collect(s, 1);
            start_cooperative_cam_mode(GPtr::from_addr(THE_CAMERA));
        }
        // 06F1 LIMIT_TWO_PLAYER_DISTANCE (0x49883D): byte 0x8A5E4A = 1, dword 0x8A5E4C = P0 (raw).
        0x06F1 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0x8A5E4A).write(1);
            GPtr::<u32>::from_addr(0x8A5E4C).write(sp(0));
        }
        // 06F2 RELEASE_TWO_PLAYER_DISTANCE (0x498860): no operands; byte 0x8A5E4A = 0.
        0x06F2 => GPtr::<u8>::from_addr(0x8A5E4A).write(0),
        // 06F3 SET_PLAYER_PLAYER_TARGETTING (0x49886E): byte 0x96A8B2 = P0 == 0.
        0x06F3 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0x96A8B2).write(u8::from(sp(0) == 0));
        }
        // 06FA SET_PLAYERS_CAN_BE_IN_SEPARATE_CARS (0x498A2C): byte 0x96A8B3 = P0 != 0.
        0x06FA => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0x96A8B3).write(u8::from(sp(0) != 0));
        }
        // 0701 SKIP_CUTSCENE_END (0x498BCF, gr: unregistered): no operands; `m_SceneSkipIP` (+0xD8) = 0.
        // 0707 SKIP_CUTSCENE_START_INTERNAL (0x498ED8, gr: unregistered): it = P0.
        0x0701 => s.byte_add(crate::vm::SCENE_SKIP_IP).cast::<u32>().write(0),
        0x0707 => {
            collect(s, 1);
            s.byte_add(crate::vm::SCENE_SKIP_IP).cast::<u32>().write(sp(0));
        }
        _ => {
            return fallback(
                process_commands_1700_to_1799_original,
                "CRunningScript::ProcessCommands1700To1799",
                s,
                command,
            );
        }
    }
    0
}

/// 0x06BD IS_LINE_OF_SIGHT_CLEAR (0x497852): `CollectParameters(11)`;
/// `GetIsLineOfSightClear(&(SP0..SP2), &(SP3..SP5), SP6..SP10 != 0, false, false)`; the AL byte is the compare flag.
/// The pushed bools carry stale upper register bytes in the original (the callee reads the low byte); the
/// port passes 0/1.
fn is_line_of_sight_clear(s: Scr) {
    collect(s, 11);
    let a = [sp(0), sp(1), sp(2)];
    let b = [sp(3), sp(4), sp(5)];
    let nz = |i| u32::from(sp(i) != 0);
    let r = get_is_line_of_sight_clear(
        GPtr::from_raw(a.as_ptr().cast_mut()),
        GPtr::from_raw(b.as_ptr().cast_mut()),
        nz(6),
        nz(7),
        nz(8),
        nz(9),
        nz(10),
        0,
        0,
    );
    flag(s, u32::from(r));
}

/// 0x06D5 CREATE_CHECKPOINT (0x497E54): `CollectParameters(8)`; `GetActualScriptThingIndex(
/// CollectNextParameterWithoutIncreasingPC(), 3)` evaluated and discarded; `SP0 = AddScriptCheckpoint(SP1..SP3, SP4..SP6,
/// SP7, SP0)`; with mission cleanup `AddEntityToList(SP0, 0xC)` (0x4974EA); `StoreParameters(1)`.
fn create_checkpoint(s: Scr) {
    collect(s, 8);
    let n = collect_next(s);
    get_actual_script_thing_index(n, 3);
    let h = add_script_checkpoint(sp(1), sp(2), sp(3), sp(4), sp(5), sp(6), sp(7), sp(0));
    set_sp(0, h as u32);
    if uses_mission_cleanup(s) {
        mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), h, 0xC);
    }
    store(s, 1);
}

/// 0x06D6 DELETE_CHECKPOINT (0x497EF9): `CollectParameters(1)`; if `GetActualScriptThingIndex(SP0, 3) >= 0`:
/// `RemoveScriptCheckpoint(SP0)` (the handle, not the index) and, with mission cleanup, `RemoveEntityFromList(SP0, 0xC)`.
fn delete_checkpoint(s: Scr) {
    collect(s, 1);
    if get_actual_script_thing_index(spi(0), 3) < 0 {
        return;
    }
    remove_script_checkpoint(spi(0));
    if uses_mission_cleanup(s) {
        mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), spi(0), 0xC);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_1700_TO_1799];
