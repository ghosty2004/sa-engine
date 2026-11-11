//! `CRunningScript::ProcessCommands1900To1999` (0x46B460): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands1900To1999.md`.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[0x077E, 0x07AC, 0x07BD, 0x07C7];

/// `CGame::currArea` 0xB72914 (owned by the game module; read by address as the original does).
const CURR_AREA: usize = 0xB72914;

orig! {
    /// A virtual `CVehicle` method without arguments (thiscall through the vtable slot read at run time)
    fn vcall0(f: u32, this: GPtr<u8>) = thiscall dyn => unported;
    /// `CTrain::RemoveOneMissionTrain(CTrain*)` (0x6F5DC0)
    fn remove_one_mission_train(t: GPtr<u8>) = cdecl 0x6F5DC0 => unported;
    /// `CTrain::SetNewTrainPosition(CTrain*, CVector)` (0x6F7140; the vector by value)
    fn set_new_train_position(t: GPtr<u8>, x: u32, y: u32, z: u32) = cdecl 0x6F7140 => unported;
}

/// `this->vtable[0xF8 / 4]()` (`CVehicle::BreakTowLink`)
#[inline]
fn break_tow_link(v: GPtr<u8>) {
    let f = v.cast::<GPtr<u32>>().read().byte_add(0xF8).read();
    vcall0(f, v);
}

/// `CRunningScript::ProcessCommands1900To1999(int32 command)` (0x46B460, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x46B460, conv = thiscall, name = "CRunningScript::ProcessCommands1900To1999", expect = "6A FF 68 4D 7F 83 00", this = "CRunningScript", fallback)]
pub fn process_commands_1900_to_1999(s: Scr, command: u32) -> i8 {
    match command {
        // 077E GET_AREA_VISIBLE (0x46B88C): `SP0 = CGame::currArea`; `StoreParameters(1)`
        0x077E => {
            set_sp(0, GPtr::<u32>::from_addr(CURR_AREA).read());
            store(s, 1);
        }
        // 07AC DETACH_TRAILER_FROM_CAB (0x46C2AF)
        0x07AC => detach_trailer_from_cab(s),
        // 07BD DELETE_MISSION_TRAIN (0x46C615)
        0x07BD => {
            collect(s, 1);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            if !v.is_null() {
                remove_one_mission_train(v);
            }
        }
        // 07C7 SET_MISSION_TRAIN_COORDINATES (0x46C928): the vector is copied before the lookup (no null check)
        0x07C7 => {
            collect(s, 4);
            let (x, y, z) = (sp(1), sp(2), sp(3));
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            set_new_train_position(v, x, y, z);
        }
        _ => {
            return fallback(
                process_commands_1900_to_1999_original,
                "CRunningScript::ProcessCommands1900To1999",
                s,
                command,
            );
        }
    }
    0
}

/// 0x07AC DETACH_TRAILER_FROM_CAB (0x46C2AF): `CollectParameters(2)`; `t = SP0 == -1 ? null : VEH(SP0)`,
/// `c = SP1 == -1 ? null : VEH(SP1)` (pool pointer loaded once). If `t`: `t->BreakTowLink()` (vtable +0xF8; both
/// original sub-branches on `c` are identical). Else `x = c->m_pTrailer (+0x4C8)` (c not null-checked); if `x`:
/// `x->BreakTowLink()`.
fn detach_trailer_from_cab(s: Scr) {
    collect(s, 2);
    let p = pool(VEHICLE_POOL);
    let t = if spi(0) == -1 {
        GPtr::NULL
    } else {
        vehicle_at_ref(p, spi(0))
    };
    let c = if spi(1) == -1 {
        GPtr::NULL
    } else {
        vehicle_at_ref(p, spi(1))
    };
    if !t.is_null() {
        break_tow_link(t);
        return;
    }
    let x = c.byte_add(0x4C8).cast::<GPtr<u8>>().read();
    if !x.is_null() {
        break_tow_link(x);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_1900_TO_1999];
