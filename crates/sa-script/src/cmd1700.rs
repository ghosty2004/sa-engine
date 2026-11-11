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
    0x06BD, 0x06D5, 0x06D6, 0x06D7, 0x06E7, 0x06E8, 0x06E9, 0x06EA, 0x06ED,
];

/// `CTheScripts::UsedObjectArray[0].nModelIndex` + 0x1C (the base the `m < 0` model rule indexes from).
const USED_OBJECT_ARRAY_RULE_BASE: u32 = 0xA44B88;

orig! {
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
