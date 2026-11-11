//! `CRunningScript::ProcessCommands2200To2299` (0x474900): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands2200To2299.md`.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::{hook, orig};

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[0x08D2, 0x08D7, 0x08D9, 0x08DA];

orig! {
    /// `CMenuSystem::CheckForSelected(uint8)` (0x5807E0), AL (the full ScriptParams dword is pushed)
    fn check_for_selected(id: u32) -> i8 = cdecl 0x5807E0 => unported;
    /// `CMenuSystem::ActivateOneItem(uint8, uint8, bool)` (0x581B30)
    fn activate_one_item(id: u32, row: u32, on: u32) = cdecl 0x581B30 => unported;
    /// `CMenuSystem::SwitchOffMenu(uint8)` (0x580750)
    fn switch_off_menu(id: u32) = cdecl 0x580750 => unported;
}

/// `CRunningScript::ProcessCommands2200To2299(int32 command)` (0x474900, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x474900, conv = thiscall, name = "CRunningScript::ProcessCommands2200To2299", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript", fallback)]
pub fn process_commands_2200_to_2299(s: Scr, command: u32) -> i8 {
    match command {
        // 08D2 SET_OBJECT_SCALE (0x4754B8): `OBJ(SP0)->m_fScale (+0x15C) = SP1` (raw dword, no null check)
        0x08D2 => {
            collect(s, 2);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            o.byte_add(0x15C).cast::<u32>().write(sp(1));
        }
        // 08D7 GET_MENU_ITEM_SELECTED (0x475602): `SP0 = (int8)CheckForSelected(SP0)` (movsx); `StoreParameters(1)`
        0x08D7 => {
            collect(s, 1);
            let r = check_for_selected(sp(0));
            set_sp(0, i32::from(r) as u32);
            store(s, 1);
        }
        // 08D9 ACTIVATE_MENU_ITEM (0x475661): `ActivateOneItem((u8)SP0, (u8)SP1, (u8)SP2)` (zero-extended pushes)
        0x08D9 => {
            collect(s, 3);
            activate_one_item(sp(0) & 0xFF, sp(1) & 0xFF, sp(2) & 0xFF);
        }
        // 08DA DELETE_MENU (0x475693): `SwitchOffMenu((u8)SP0)`
        0x08DA => {
            collect(s, 1);
            switch_off_menu(sp(0) & 0xFF);
        }
        _ => {
            return fallback(
                process_commands_2200_to_2299_original,
                "CRunningScript::ProcessCommands2200To2299",
                s,
                command,
            );
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_2200_TO_2299];
