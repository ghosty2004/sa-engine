//! `CGangWars::TellGangMembersTo` (batch `frontier2-a9`), ported from the 1.0 US bytes (`.HOODLUM` body 0x1568E00).
//! Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::bindings::{find_player_ped, general_random_int_in_range, task_new};
use sa_core::hook;
use sa_core::mem::GPtr;

use crate::bindings::{vehicle_is_driver, wander_gang_ctor};
use crate::common::{g, rd};

use sa_stream::bindings::ped_is_player;
use sa_world::bindings::{
    car_drive_wander_ctor, event_group_add, event_script_command_ctor, event_script_command_dtor,
    kill_ped_on_foot_ctor, simple_car_drive_ctor,
};

/// `CPools::ms_pPedPool` (0xB74490)
const PED_POOL: usize = 0xB74490;
/// `CPed` size in the pool
const PED_SIZE: u32 = 0x7C4;
/// The wander-gang task's float argument (0x86FCB0)
const WANDER_ARG: usize = 0x86FCB0;
/// The value the no-CD VM (0xCB73E0, descriptor 0x155FFB0) leaves in EBX at the function's start, whatever EBX held
/// (evaluated with tools/re/vm_eval.py 1568E37 1568E4F): -1, the player index of `FindPlayerPed` and the kill task's
/// time, and the SEH state the original stores after each event construction.
const VM_EBX: i32 = -1;

/// Adds a stack `CEventScriptCommand(TASK_PRIMARY_PRIMARY = 3, task, false)` to the ped's event group (`+0x47C` +
/// 0x68) and destroys it (0x4B0A50).
fn add_script_command(ped: GPtr<u8>, task: GPtr<u8>) {
    let mut ev = [0u32; 6];
    let e = GPtr::<u8>::from_raw(ev.as_mut_ptr().cast());
    event_script_command_ctor(e, 3, task, 0);
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), e, 0);
    event_script_command_dtor(e.cast::<u32>());
}

/// `CGangWars::TellGangMembersTo(bool isGangWarEnding)` (stub 0x444530 → 0x1568E00, cdecl). Walks the ped pool from
/// the last slot down (the pool pointer read once). Every live non-player ped whose type (+0x598) is a gang (7..16)
/// other than GANG2 (8, Grove) gets a script-command event:
/// * argument 0: `CTaskComplexKillPedOnFoot(FindPlayerPed(-1), -1, 0, 0, 0, 2)`;
/// * argument 1, ped in a vehicle (+0x46C bit 0x100): the driver `CTaskComplexCarDriveWander(vehicle, 0, 10.0)`, a
///   passenger `CTaskSimpleCarDrive(vehicle, 0, 0)`, and nothing more (gta-reversed also gives them the wander task);
///   on foot `CTaskComplexWanderGang(4, GetRandomNumberInRange(0, 8), 5000, true, [0x86FCB0])`;
/// * any other argument value: nothing (the original compares the whole dword with 0 and 1).
///
/// The tasks come from `CTask::operator new`; a NULL allocation passes a NULL task.
#[hook(addr = 0x444530, conv = cdecl, name = "CGangWars::TellGangMembersTo", expect = "E9 CB 48 12 01",
       unverified = "frontier2-a9", route = "L2", hazards = "heap-writes",
       note = "EBX = -1 comes out of the no-CD VM 0xCB73E0 (evaluated); SEH frame not modelled")]
pub fn tell_gang_members_to(arg: u32) {
    let pool = g::<GPtr<u8>>(PED_POOL).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        i -= 1;
        if rd::<GPtr<u8>>(pool, 4).byte_add(i as isize).read() & 0x80 != 0 {
            continue;
        }
        let objects = rd::<u32>(pool, 0);
        let off = (i as u32).wrapping_mul(PED_SIZE);
        if objects.wrapping_add(off) == 0 {
            continue;
        }
        let ped = GPtr::<u8>::from_addr(objects.wrapping_add(off) as usize);
        if ped_is_player(ped) != 0 {
            continue;
        }
        let ty = rd::<i32>(ped, 0x598);
        if !(7..=16).contains(&ty) || ty == 8 {
            continue;
        }
        match arg {
            0 => {
                let player = find_player_ped(VM_EBX);
                let mem = task_new(0x38);
                let t = if mem.is_null() {
                    GPtr::NULL
                } else {
                    kill_ped_on_foot_ctor(mem, player, VM_EBX, 0, 0, 0, 2)
                };
                add_script_command(ped, t);
            }
            1 => {
                if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
                    let veh = rd::<GPtr<u8>>(ped, 0x58C);
                    if vehicle_is_driver(veh, ped) != 0 {
                        let mem = task_new(0x24);
                        let t = if mem.is_null() {
                            GPtr::NULL
                        } else {
                            car_drive_wander_ctor(mem, rd::<GPtr<u8>>(ped, 0x58C), 0, 10.0)
                        };
                        add_script_command(ped, t);
                        continue;
                    }
                    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
                        let mem = task_new(0x60);
                        let t = if mem.is_null() {
                            GPtr::NULL
                        } else {
                            simple_car_drive_ctor(mem, rd::<GPtr<u8>>(ped, 0x58C), GPtr::NULL, 0)
                        };
                        add_script_command(ped, t);
                        continue;
                    }
                }
                let mem = task_new(0x38);
                let t = if mem.is_null() {
                    GPtr::NULL
                } else {
                    let f = g::<u32>(WANDER_ARG).read();
                    let dir = general_random_int_in_range(0, 8);
                    wander_gang_ctor(mem, 4, dir, 5000, 1, f)
                };
                add_script_command(ped, t);
            }
            _ => {}
        }
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_TELL_GANG_MEMBERS_TO];
