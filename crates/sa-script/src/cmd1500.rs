//! `CRunningScript::ProcessCommands1500To1599` (0x490DB0): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands1500To1599.md`.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[0x060B];

orig! {
    /// `CTheScripts::GetActualScriptThingIndex(int32 ref, uint8 type)` (0x4839A0)
    fn get_actual_script_thing_index(r: i32, ty: u32) -> i32 = cdecl 0x4839A0 => unported;
    /// `CPedIntelligence::SetPedDecisionMakerType(int32)` (0x600B50)
    fn set_ped_decision_maker_type(intel: GPtr<u8>, ty: i32) = thiscall 0x600B50 => unported;
}

/// `CRunningScript::ProcessCommands1500To1599(int32 command)` (0x490DB0, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x490DB0, conv = thiscall, name = "CRunningScript::ProcessCommands1500To1599", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript", fallback)]
pub fn process_commands_1500_to_1599(s: Scr, command: u32) -> i8 {
    match command {
        // 060B SET_CHAR_DECISION_MAKER (0x49152D): `dm = SP1 == -1 ? -1 : GetActualScriptThingIndex(SP1, 7)`;
        // `PED(SP0)->m_pIntelligence (+0x47C)->SetPedDecisionMakerType(dm)` (no null check)
        0x060B => {
            collect(s, 2);
            let p = ped_at_ref(pool(PED_POOL), spi(0));
            let dm = if spi(1) == -1 {
                -1
            } else {
                get_actual_script_thing_index(spi(1), 7)
            };
            set_ped_decision_maker_type(p.byte_add(0x47C).cast::<GPtr<u8>>().read(), dm);
        }
        _ => {
            return fallback(
                process_commands_1500_to_1599_original,
                "CRunningScript::ProcessCommands1500To1599",
                s,
                command,
            );
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_1500_TO_1599];
