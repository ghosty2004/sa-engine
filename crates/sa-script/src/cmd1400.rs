//! `CRunningScript::ProcessCommands1400To1499` (0x48EAA0): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands1400To1499.md`.
//!
//! The task cases build the task with `CTask::operator new` + the constructor (EAX = the task, null when `new`
//! fails) and give it with `GivePedScriptedTask(ped, task, command)`. The SEH state writes around `new` + ctor
//! (`[esp+0x450]`, the stub 0x4011FA) only matter for exception unwinding and are not ported.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x057B, 0x057D, 0x057F, 0x0585, 0x058D, 0x058E, 0x0591, 0x0592, 0x0596, 0x059B, 0x05B1, 0x05B2, 0x05B3,
    0x05B5, 0x05B7, 0x05B8, 0x05C4, 0x05C6, 0x05CA, 0x05CB, 0x05CC, 0x05CD, 0x05CE, 0x05D0, 0x05D5,
];

/// `.rdata` 0x86E69C / 0x86E698: the default enter-car times (int 20000), read at run time as the original does.
const ENTER_AS_PASSENGER_DEFAULT_TIME: usize = 0x86E69C;
const ENTER_AS_DRIVER_DEFAULT_TIME: usize = 0x86E698;
/// The dword 0x8D237C (initial value 6) the enter-car cases copy into the new task and then reset to 6
/// (spec open question: its name).
const NEXT_ENTER_CAR_DWORD: usize = 0x8D237C;

orig! {
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CRunningScript::GivePedScriptedTask(int32 pedHandle, CTask*, int32 opcode)` (0x465C20)
    fn give_ped_scripted_task(s: Scr, ped: u32, task: GPtr<u8>, command: u32) = thiscall 0x465C20 => unported;
    /// `CTaskSimpleHandsUp::CTaskSimpleHandsUp(int32 time)` (0x48E970), returns this
    fn task_simple_hands_up_ctor(t: GPtr<u8>, time: u32) -> GPtr<u8> = thiscall 0x48E970 => unported;
    /// `CCarEnterExit::ComputeTargetDoorToEnterAsPassenger(const CVehicle*, int32 seat)` (0x64F190)
    fn compute_target_door_to_enter_as_passenger(v: GPtr<u8>, seat: u32) -> u32 = cdecl 0x64F190 => unported;
    /// `CTaskComplexEnterCarAsPassengerTimed::CTaskComplexEnterCarAsPassengerTimed(CVehicle*, int32 door, int32 time,
    /// bool)` (0x63B030), returns this
    fn task_enter_car_as_passenger_timed_ctor(t: GPtr<u8>, v: GPtr<u8>, door: u32, time: u32, b: u32) -> GPtr<u8> = thiscall 0x63B030 => unported;
    /// `CTaskComplexEnterCarAsDriverTimed::CTaskComplexEnterCarAsDriverTimed(CVehicle*, int32 time)` (0x63AD70), returns this
    fn task_enter_car_as_driver_timed_ctor(t: GPtr<u8>, v: GPtr<u8>, time: u32) -> GPtr<u8> = thiscall 0x63AD70 => unported;
    /// `CTaskComplexLeaveCar::CTaskComplexLeaveCar(CVehicle*, int32 door, int32 delay, bool sensible, bool forceGetOut)`
    /// (0x63B8C0), returns this
    fn task_complex_leave_car_ctor(t: GPtr<u8>, v: GPtr<u8>, door: u32, delay: u32, sensible: u32, force: u32) -> GPtr<u8> = thiscall 0x63B8C0 => unported;
}

/// The enter-car time mapping: -2 -> -1, -1 -> the dword at `default_at`, else the value.
#[inline]
fn enter_car_time(t: u32, default_at: usize) -> u32 {
    match t as i32 {
        -2 => u32::MAX,
        -1 => GPtr::<u32>::from_addr(default_at).read(),
        _ => t,
    }
}

/// `t->field = [0x8D237C]` (t not null-checked, as in the original), then `[0x8D237C] = 6`.
#[inline]
fn take_next_enter_car_dword(t: GPtr<u8>, field: isize) {
    let g = GPtr::<u32>::from_addr(NEXT_ENTER_CAR_DWORD);
    t.byte_add(field).cast::<u32>().write(g.read());
    g.write(6);
}

/// `CRunningScript::ProcessCommands1400To1499(int32 command)` (0x48EAA0, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x48EAA0, conv = thiscall, name = "CRunningScript::ProcessCommands1400To1499", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript", fallback)]
pub fn process_commands_1400_to_1499(s: Scr, command: u32) -> i8 {
    match command {
        // 05C4 TASK_HANDS_UP (0x49017F): `time = SP1 == -1 ? 20000 : SP1 == -2 ? 0x7FFFFFFF : SP1`;
        // `new(0x34) CTaskSimpleHandsUp(time)`; tail 0x48FB64/0x48FB66
        0x05C4 => {
            collect(s, 2);
            let time = match spi(1) {
                -1 => 0x4E20,
                -2 => 0x7FFF_FFFF,
                _ => sp(1),
            };
            let ped = sp(0);
            let t = task_new(0x34);
            let t = if t.is_null() {
                GPtr::NULL
            } else {
                task_simple_hands_up_ctor(t, time)
            };
            give_ped_scripted_task(s, ped, t, command);
        }
        // 05CA TASK_ENTER_CAR_AS_PASSENGER (0x49036C): `v = VEH(SP1)`; `door = SP3 == -1 ? 0 :
        // ComputeTargetDoorToEnterAsPassenger(v, SP3)`; `new(0x2C) CTaskComplexEnterCarAsPassengerTimed(v, door, time,
        // true)`; tail 0x490431: `t+0x1C = [0x8D237C]` (unchecked), `[0x8D237C] = 6`, give.
        0x05CA => {
            collect(s, 4);
            let (ped, time, seat) = (sp(0), sp(2), sp(3));
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(1));
            let door = if seat as i32 == -1 {
                0
            } else {
                compute_target_door_to_enter_as_passenger(v, seat)
            };
            let t = task_new(0x2C);
            let t = if t.is_null() {
                GPtr::NULL
            } else {
                let time = enter_car_time(time, ENTER_AS_PASSENGER_DEFAULT_TIME);
                task_enter_car_as_passenger_timed_ctor(t, v, door, time, 1)
            };
            take_next_enter_car_dword(t, 0x1C);
            give_ped_scripted_task(s, ped, t, command);
        }
        // 05CB TASK_ENTER_CAR_AS_DRIVER (0x49046B): `new(0x24) CTaskComplexEnterCarAsDriverTimed(VEH(SP1), time)`;
        // `t+0x14 = [0x8D237C]` (unchecked), `[0x8D237C] = 6`; give (tail 0x4908F6).
        0x05CB => {
            collect(s, 3);
            let (ped, time) = (sp(0), sp(2));
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(1));
            let t = task_new(0x24);
            let t = if t.is_null() {
                GPtr::NULL
            } else {
                let time = enter_car_time(time, ENTER_AS_DRIVER_DEFAULT_TIME);
                task_enter_car_as_driver_timed_ctor(t, v, time)
            };
            take_next_enter_car_dword(t, 0x14);
            give_ped_scripted_task(s, ped, t, command);
        }
        // 05CD TASK_LEAVE_CAR (0x490554): `new(0x34) CTaskComplexLeaveCar(VEH(SP1), 0, 0, true, false)`; tail
        // 0x490B60/0x490B62
        0x05CD => {
            collect(s, 2);
            let ped = sp(0);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(1));
            let t = task_new(0x34);
            let t = if t.is_null() {
                GPtr::NULL
            } else {
                task_complex_leave_car_ctor(t, v, 0, 0, 1, 0)
            };
            give_ped_scripted_task(s, ped, t, command);
        }
        // 057B, 057D, 057F, 0585, 058D, 058E, 0591, 0592, 059B, 05B1, 05B2, 05B3, 05B5, 05B7, 05B8, 05C6, 05CC, 05CE,
        // 05D0, 05D5 (0x490C0B): the jump table goes straight to the epilogue: 0 without reading the operands.
        0x057B | 0x057D | 0x057F | 0x0585 | 0x058D | 0x058E | 0x0591 | 0x0592 | 0x059B | 0x05B1 => {}
        0x05B2 | 0x05B3 | 0x05B5 | 0x05B7 | 0x05B8 | 0x05C6 | 0x05CC | 0x05CE | 0x05D0 | 0x05D5 => {}
        // 0596 IS_PLAYER_IN_SHORTCUT_TAXI (0x48EE39): `CollectParameters(1)`; UpdateCompareFlag(false).
        0x0596 => {
            collect(s, 1);
            set_flag(s, false);
        }
        _ => {
            return fallback(
                process_commands_1400_to_1499_original,
                "CRunningScript::ProcessCommands1400To1499",
                s,
                command,
            );
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_1400_TO_1499];
