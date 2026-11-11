//! `CRunningScript::ProcessCommands2100To2199` (0x470A90): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands2100To2199.md`.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[0x0840, 0x0860, 0x087E, 0x0893];

/// `CEntryExitManager::ms_entryExitStackPosn` 0x96A7C4 (owned by sa-world; written by address as the original does).
const ENTRY_EXIT_STACK_POSN: usize = 0x96A7C4;

orig! {
    /// `CPed::IsPlayer()` (0x5DF8F0), AL
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CTimeCycle::StopExtraColour(bool)` (0x55FF20)
    fn stop_extra_colour(a: i32) = cdecl 0x55FF20 => unported;
    /// `CVehicle::SetTowLink(CVehicle*, bool)` through the vtable slot +0xF4, read at run time; AL discarded
    fn vcall_set_tow_link(f: u32, this: GPtr<u8>, c: GPtr<u8>, b: u32) = thiscall dyn => unported;
}

/// `CRunningScript::ProcessCommands2100To2199(int32 command)` (0x470A90, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x470A90, conv = thiscall, name = "CRunningScript::ProcessCommands2100To2199", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript", fallback)]
pub fn process_commands_2100_to_2199(s: Scr, command: u32) -> i8 {
    match command {
        // 0840 SET_VEHICLE_AREA_VISIBLE (0x470E5B): `VEH(SP0)->m_nAreaCode (+0x2F) = (u8)SP1` (no null check)
        0x0840 => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            v.byte_add(0x2F).write(sp(1) as u8);
        }
        // 0860 SET_CHAR_AREA_VISIBLE (0x4714A5): `p = PED(SP0)` (no null check); `+0x2F = (u8)SP1`; if `SP1 == 0`:
        // `+0x78C = 0`, and if `p->IsPlayer()`: `CEntryExitManager::ms_entryExitStackPosn = 0`, `CTimeCycle::StopExtraColour(false)`.
        0x0860 => {
            collect(s, 2);
            let p = ped_at_ref(pool(PED_POOL), spi(0));
            p.byte_add(0x2F).write(sp(1) as u8);
            if sp(1) == 0 {
                p.byte_add(0x78C).cast::<u32>().write(0);
                if ped_is_player(p) != 0 {
                    GPtr::<u32>::from_addr(ENTRY_EXIT_STACK_POSN).write(0);
                    stop_extra_colour(0);
                }
            }
        }
        // 087E SET_CHAR_DROPS_WEAPONS_WHEN_DEAD (0x471B4F): `PED(SP0)` +0x474 bit 9 (bDoesntDropWeaponsWhenDead) =
        // `SP1 == 0` (no null check)
        0x087E => {
            collect(s, 2);
            let p = ped_at_ref(pool(PED_POOL), spi(0));
            let fl = p.byte_add(0x474).cast::<u32>();
            let v = fl.read();
            fl.write(if sp(1) != 0 { v & 0xFFFF_FDFF } else { v | 0x200 });
        }
        // 0893 ATTACH_TRAILER_TO_CAB (0x4720BB): `t = VEH(SP0)`, `c = VEH(SP1)` (pool loaded once); if both:
        // `t->SetTowLink(c, true)` (vtable +0xF4)
        0x0893 => {
            collect(s, 2);
            let pl = pool(VEHICLE_POOL);
            let t = vehicle_at_ref(pl, spi(0));
            let c = vehicle_at_ref(pl, spi(1));
            if !t.is_null() && !c.is_null() {
                let f = t.cast::<GPtr<u32>>().read().byte_add(0xF4).read();
                vcall_set_tow_link(f, t, c, 1);
            }
        }
        _ => {
            return fallback(
                process_commands_2100_to_2199_original,
                "CRunningScript::ProcessCommands2100To2199",
                s,
                command,
            );
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_2100_TO_2199];
