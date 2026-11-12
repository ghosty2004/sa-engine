//! `CRunningScript::DoDeathArrestCheck` (batch `std7-script-1`). Ported from the 1.0 US bytes; its body
//! lives in `.HOODLUM` (0x1564F00) and reaches `CMessages::ClearSmallMessagesOnly` through a
//! return-address-obfuscated call (decoded below). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

/// `CTheScripts::OnAMissionFlag` (offset into the script space) and `CTheScripts::ScriptSpace`
const ON_A_MISSION_FLAG: usize = 0xA476AC;
const SCRIPT_SPACE: usize = 0xA49960;
/// `CWorld::PlayerInFocus` and `CWorld::Players` (0x190 each)
const PLAYER_IN_FOCUS: usize = 0xB7CD74;
const PLAYERS: usize = 0xB7CD98;

orig! {
    /// `CPlayerInfo::IsRestartingAfterDeath()` (0x56E550), `CPlayerInfo::IsRestartingAfterArrest()` (0x56E560)
    fn is_restarting_after_death(p: GPtr<u8>) -> u8 = thiscall 0x56E550 => unported;
    fn is_restarting_after_arrest(p: GPtr<u8>) -> u8 = thiscall 0x56E560 => unported;
    /// `CMessages::ClearSmallMessagesOnly()` (0x69DD30)
    fn clear_small_messages_only() = cdecl 0x69DD30 => unported;
}

fn mission_flag_cell() -> GPtr<u32> {
    GPtr::from_addr(SCRIPT_SPACE + GPtr::<u32>::from_addr(ON_A_MISSION_FLAG).read() as usize)
}

/// `CRunningScript::DoDeathArrestCheck()` (stub 0x485A50 → body 0x1564F00, thiscall). Nothing unless
/// the check is enabled (+0xD3), `OnAMissionFlag` is non-zero and its script-space dword is 1, and the
/// player in focus is restarting after death or arrest. Then the gosub stack pointer (word +0x38) is
/// lowered to 1 when above (a decrement loop) and decremented, the IP (+0x14) = the stack slot it now
/// indexes (+0x18 + 4 * sp; a pointer of 0 wraps to 0xFFFF as in the original), the small messages are
/// cleared, the mission flag dword = 0, +0xD4 = 1 and +0xCC = 0.
///
/// The call to `ClearSmallMessagesOnly` is obfuscated: `call $+5; push ecx; mov ecx, [0x15604F4]; xor
/// ecx, [0x736A72]; xchg [esp], ecx; add dword [esp + 4], 0x19; ret` jumps to 0xFFA4EBF5 ^ 0xFFCD36C5 =
/// 0x69DD30 with the return address 0x1564FAC (the instruction after the sequence).
#[hook(addr = 0x485A50, conv = thiscall, name = "CRunningScript::DoDeathArrestCheck", expect = "E9 AB F4 0D 01",
       unverified = "std7-script-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn do_death_arrest_check(this: GPtr<u8>) {
    if this.byte_add(0xD3).read() == 0 || GPtr::<u32>::from_addr(ON_A_MISSION_FLAG).read() == 0 {
        return;
    }
    if mission_flag_cell().read() != 1 {
        return;
    }
    let player = GPtr::<u8>::from_addr(PLAYERS + GPtr::<u8>::from_addr(PLAYER_IN_FOCUS).read() as usize * 0x190);
    if is_restarting_after_death(player) == 0 && is_restarting_after_arrest(player) == 0 {
        return;
    }
    let sp = this.byte_add(0x38).cast::<u16>();
    if sp.read() > 1 {
        sp.write(1);
    }
    let v = sp.read().wrapping_sub(1);
    sp.write(v);
    let ip = this.byte_add(0x18 + 4 * v as isize).cast::<u32>().read();
    this.byte_add(0x14).cast::<u32>().write(ip);
    clear_small_messages_only();
    mission_flag_cell().write(0);
    this.byte_add(0xD4).write(1);
    this.byte_add(0xCC).cast::<u32>().write(0);
}

pub const HOOKS: &[HookReg] = &[HOOK_DO_DEATH_ARREST_CHECK];
