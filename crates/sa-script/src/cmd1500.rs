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
pub const PORTED: &[u16] = &[0x060B, 0x0623, 0x0624, 0x0625, 0x0626, 0x0627, 0x0628, 0x0629, 0x062A];

orig! {
    /// `CStats::IncrementStat(stat, value)` (0x55C180), `DecrementStat` (0x559FA0), the stat register (0x55C410)
    fn increment_stat(stat: u32, value: u32) = cdecl 0x55C180 => unported;
    fn decrement_stat(stat: u32, value: u32) = cdecl 0x559FA0 => unported;
    fn register_stat(stat: u32, value: u32) = cdecl 0x55C410 => unported;
    /// `CStats::SetStatValue(stat, value)` (0x55A070)
    fn set_stat_value(stat: u32, value: u32) = cdecl 0x55A070 => unported;
    /// `CStats::DisplayScriptStatUpdateMessage(state, stat, value)` (0x55B980)
    fn display_script_stat_update_message(state: u32, stat: u32, value: u32) = cdecl 0x55B980 => unported;
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
        // 062A SET_FLOAT_STAT (0x4921CD, STD-6): `CStats::SetStatValue((u16)SP0, SP1)`, then
        // `CStats::DisplayScriptStatUpdateMessage(1, SP0, SP1)`.
        0x062A => {
            collect(s, 2);
            set_stat_value(sp(0) & 0xFFFF, sp(1));
            display_script_stat_update_message(1, sp(0), sp(1));
        }
        // 0623 INCREMENT_INT_STAT / 0624 INCREMENT_FLOAT_STAT / 0625 DECREMENT_INT_STAT / 0626 DECREMENT_FLOAT_STAT /
        // 0627 REGISTER_INT_STAT / 0628 REGISTER_FLOAT_STAT / 0629 SET_INT_STAT (0x492058..0x4921BE, STD-6): the
        // stat function on `(u16)SP0` with SP1 (`fild` for the int forms), then
        // `CStats::DisplayScriptStatUpdateMessage(1, SP0, value)` (0 for the decrements; none for the
        // REGISTER forms' shared tails: they jump into the increment / set-float tails, so they display too).
        0x0623..=0x0629 => {
            collect(s, 2);
            let int_form = matches!(command, 0x0623 | 0x0625 | 0x0627 | 0x0629);
            let v = if int_form { sa_math::x87::X87::from_i32(spi(1)).to_f32().to_bits() } else { sp(1) };
            let stat = sp(0) & 0xFFFF;
            match command {
                0x0623 | 0x0624 => increment_stat(stat, v),
                0x0625 | 0x0626 => decrement_stat(stat, v),
                0x0627 | 0x0628 => register_stat(stat, v),
                _ => set_stat_value(stat, v),
            }
            let state = u32::from(!matches!(command, 0x0625 | 0x0626));
            display_script_stat_update_message(state, sp(0), v);
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
