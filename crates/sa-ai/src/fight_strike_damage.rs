//! WRITER-NATIVE (batch `native-24`): 0x61C740, `CTaskSimpleFight`'s strike damage (unnamed), called when a punch
//! lands. Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CStats::GetFatAndMuscleModifier(int32)` (0x559AF0, ST0)
    fn stats_get_fat_and_muscle_modifier(i: i32) -> f64 = cdecl 0x559AF0 => unported;
}

/// `float CTaskSimpleFight::<strike damage>(CPed* ped)` (0x61C740, `ret 4`, ST0).
///
/// `d = (float)(uint8)[0xC17125 + (+0x24 - 4) * 0x88 + +0x25]` (the move table's damage byte). The player: 50 with
/// player-data (+0x480) flag +0x34 bit 1, else `GetFatAndMuscleModifier(4) * d`. Other peds by the current weapon
/// (+0x5A0 + slot +0x718 * 0x1C): brass knuckles (1) `d * 1.5`, fists (0) `d * pedStats(+0x59C) +0x28`, else `d`.
#[hook(addr = 0x61C740, conv = thiscall, name = "FUN_0061c740",
       expect = "51 0F BE 41 24 0F BE 49 25",
       unverified = "native-24", route = "L2", hazards = "none", recs = "weapon-combat",
       note = "ST0 result")]
pub fn fight_strike_damage(this: GPtr<u8>, ped: GPtr<u8>) -> X87 {
    let i = (i32::from(rd::<i8>(this, 0x24)) - 4).wrapping_mul(0x88) + i32::from(rd::<i8>(this, 0x25));
    let d = xi(i32::from(g::<u8>((0xC1_7125u32).wrapping_add(i as u32) as usize).read())).to_f32();
    if ped_is_player(ped) != 0 {
        if rd::<u8>(rd::<GPtr<u8>>(ped, 0x480), 0x34) & 2 != 0 {
            return x(50.0);
        }
        return X87::from_f64_exact(stats_get_fat_and_muscle_modifier(4)).fmul_m(d);
    }
    let slot = i32::from(rd::<i8>(ped, 0x718));
    let w = rd::<u32>(ped, (0x5A0 + slot * 0x1C) as isize);
    match w {
        1 => x(d).fmul_m(1.5),
        0 => x(d).fmul_m(rd::<f32>(rd::<GPtr<u8>>(ped, 0x59C), 0x28)),
        _ => x(d),
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FIGHT_STRIKE_DAMAGE];
