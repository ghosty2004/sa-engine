//! `CRunningScript::ProcessCommands1300To1399` (0x48CDD0): partial port (PORT-SCRIPT-2, ADR-002).
//! Opcodes in [`PORTED`] run in Rust; every other opcode (and the default, -1) runs the original body
//! through the fallback. Cases follow the 1.0 US bytes and
//! `docs/re/specs/SCRIPT/commands/ProcessCommands1300To1399.md`.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::orig;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[0x052C, 0x053F, 0x0570];

orig! {
    /// `CMBlur::ClearDrunkBlur()` (0x71D780)
    pub fn clear_drunk_blur() = cdecl 0x71D780 => unported;
}

/// `CRunningScript::ProcessCommands1300To1399(int32 command)` (0x48CDD0, thiscall, `ret 4`): AL = 0
/// continue, -1 unknown.
#[hook(addr = 0x48CDD0, conv = thiscall, name = "CRunningScript::ProcessCommands1300To1399", expect = "8B 44 24 04 83 EC 68", this = "CRunningScript", fallback)]
pub fn process_commands_1300_to_1399(s: Scr, command: u32) -> i8 {
    match command {
        // 052C SET_PLAYER_DRUNKENNESS (case 0x48D239): `Players[SP0].m_nDrunkenness (+0x44) = (uint8)SP1;
        // m_nFadeDrunkenness (+0x45) = 0`; `CMBlur::ClearDrunkBlur()` when the byte is 0.
        0x052C => {
            collect(s, 2);
            let p = player_info(sp(0));
            let d = sp(1) as u8;
            p.byte_add(0x44).write(d);
            p.byte_add(0x45).write(0u8);
            if d == 0 {
                clear_drunk_blur();
            }
            0
        }
        // 053F SET_CAN_BURST_CAR_TYRES (case 0x48D356): `veh->bTyresDontBurst` (bit 7 of +0x42B) =
        // SP1 == 0; the vehicle pointer is not null-checked (as in the original). gr: unregistered.
        0x053F => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let b = v.byte_add(0x42B);
            let x = b.read();
            b.write(if sp(1) != 0 { x & 0x7F } else { x | 0x80 });
            0
        }
        // 0570 ADD_SHORT_RANGE_SPRITE_BLIP_FOR_CONTACT_POINT (case 0x48DB3D): type 5 (contact point),
        // colour 2. gr: unregistered.
        0x0570 => {
            short_range_coord_blip(s, 5, 2);
            0
        }
        _ => fallback(
            process_commands_1300_to_1399_original,
            "CRunningScript::ProcessCommands1300To1399",
            s,
            command,
        ),
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_1300_TO_1399];
