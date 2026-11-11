//! `CRunningScript::ProcessCommands2500To2599` (0x47A760): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands2500To2599.md`.
//!
//! SA:MP writes its own opcodes 09C6 / 09D3 into this switch's jump table; they never reach the Rust match
//! as ported cases and stay on the fallback (the dispatch is unchanged).

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[0x09C4, 0x09DB];

/// `.rdata` 0x859520: 0.0015625f (1/640)
const K_ONE_640TH: u32 = 0x3ACC_CCCD;

orig! {
    /// `CMenuSystem::SetColumnWidth(uint8 menu, uint8 column, uint16 width)` (0x582050)
    fn set_column_width(menu: u32, column: u32, width: u32) = cdecl 0x582050 => unported;
}

/// `CRunningScript::ProcessCommands2500To2599(int32 command)` (0x47A760, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x47A760, conv = thiscall, name = "CRunningScript::ProcessCommands2500To2599", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript", fallback)]
pub fn process_commands_2500_to_2599(s: Scr, command: u32) -> i8 {
    match command {
        // 09C4 SET_PETROL_TANK_WEAKPOINT (0x47A7A2): `VEH(SP0)` +0x42E bit 6 = bit 0 of SP1 (`shl cl,6; xor; and 0x40;
        // xor`, no null check)
        0x09C4 => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let b = v.byte_add(0x42E);
            let old = b.read();
            let cl = ((sp(1) as u8) << 6 ^ old) & 0x40;
            b.write(old ^ cl);
        }
        // 09DB SET_MENU_COLUMN_WIDTH (0x47AD10): `w = _ftol(fild RsGlobal.maximumWidth * 0.0015625f * (int)(u16)SP2)`
        // (x87 PC=24: `fild; fmul m32; fimul m32`, `_ftol` 0x821B40 inlined as `X87::ftol`, low dword);
        // `SetColumnWidth((u8)SP0, (u8)SP1, w)`
        0x09DB => {
            collect(s, 3);
            let width = sa_core::platform::RS_MAXIMUM_WIDTH.get();
            let w = X87::from_i32(width)
                .fmul_m(f(K_ONE_640TH))
                .fmul(X87::from_i32(i32::from(sp(2) as u16)))
                .ftol() as u32;
            set_column_width(sp(0) & 0xFF, sp(1) & 0xFF, w);
        }
        _ => {
            return fallback(
                process_commands_2500_to_2599_original,
                "CRunningScript::ProcessCommands2500To2599",
                s,
                command,
            );
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_2500_TO_2599];
