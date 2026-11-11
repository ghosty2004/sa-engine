//! `CRunningScript::ProcessCommands2000To2099` (0x472310): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands2000To2099.md`.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[0x07FE, 0x07FF, 0x0812];

/// `MI_HYDRAULICS` 0x8CD76C (the "hydralics" model index, a word set at model loading; owned by sa-stream).
const HYDRAULICS_MODEL: usize = 0x8CD76C;

orig! {
    /// `CVehicle::AddVehicleUpgrade(int32 modelId)` (0x6E3290)
    fn add_vehicle_upgrade(v: GPtr<u8>, m: i32) -> i32 = thiscall 0x6E3290 => unported;
    /// `CVehicle::RemoveVehicleUpgrade(int32 modelId)` (0x6DF930)
    fn remove_vehicle_upgrade(v: GPtr<u8>, m: i32) = thiscall 0x6DF930 => unported;
    /// `CRunningScript::PlayAnimScriptCommand(int32 command)` (0x470150; reads its own parameters)
    fn play_anim_script_command(s: Scr, command: u32) = thiscall 0x470150 => unported;
}

/// `CRunningScript::ProcessCommands2000To2099(int32 command)` (0x472310, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x472310, conv = thiscall, name = "CRunningScript::ProcessCommands2000To2099", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript", fallback)]
pub fn process_commands_2000_to_2099(s: Scr, command: u32) -> i8 {
    match command {
        // 07FE GIVE_MELEE_ATTACK_TO_CHAR (0x47320D): `p = PED(SP0)` (no null check); if `(u8)p->m_nFightingStyle (+0x72D)
        // != SP1` (movzx vs the full dword): `+0x72D = (u8)SP1`, `+0x72E = 0`. Then the inner table 0x474520 on
        // `SP2 - 1` (unsigned, > 5 does nothing): 1 -> `+0x72E = 0`, 2..5 -> `|= 1, 2, 4, 8`, 6 -> `|= 0xF`.
        0x07FE => {
            collect(s, 3);
            let p = ped_at_ref(pool(PED_POOL), spi(0));
            let style = p.byte_add(0x72D);
            let moves = p.byte_add(0x72E);
            if u32::from(style.read()) != sp(1) {
                style.write(sp(1) as u8);
                moves.write(0);
            }
            match sp(2).wrapping_sub(1) {
                0 => moves.write(0),
                1 => moves.write(moves.read() | 0x1),
                2 => moves.write(moves.read() | 0x2),
                3 => moves.write(moves.read() | 0x4),
                4 => moves.write(moves.read() | 0x8),
                5 => moves.write(moves.read() | 0xF),
                _ => {}
            }
        }
        // 07FF SET_CAR_HYDRAULICS (0x4732B1): `v = VEH(SP0)` (no null check); `SP1 != 0` ->
        // `v->AddVehicleUpgrade(word [0x8CD76C])` (result discarded), else `v->RemoveVehicleUpgrade(word [0x8CD76C])`
        0x07FF => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let m = i32::from(GPtr::<u16>::from_addr(HYDRAULICS_MODEL).read());
            if sp(1) != 0 {
                add_vehicle_upgrade(v, m);
            } else {
                remove_vehicle_upgrade(v, m);
            }
        }
        // 0812 TASK_PLAY_ANIM_NON_INTERRUPTABLE (0x473831): `this->PlayAnimScriptCommand(command)` (EBP = command)
        0x0812 => play_anim_script_command(s, command),
        _ => {
            return fallback(
                process_commands_2000_to_2099_original,
                "CRunningScript::ProcessCommands2000To2099",
                s,
                command,
            );
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_2000_TO_2099];
