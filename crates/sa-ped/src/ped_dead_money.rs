//! `CPed::CreateDeadPedMoney` (INTRO-2: the peds the intro's drive-by kills drop their money). Ported from the 1.0
//! US bytes (stub 0x4590F0 → `.HOODLUM` 0x1564140); gta-reversed `Ped.cpp` as a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

orig! {
    /// `CLocalisation::StealFromDeadPed()` (0x56D2B0)
    fn localisation_steal_from_dead_ped() -> u8 = cdecl 0x56D2B0 => unported;
    /// `CPickups::CreateSomeMoney(CVector, int32)` (0x458970)
    fn pickups_create_some_money(px: u32, py: u32, pz: u32, amount: u32) = cdecl 0x458970 => unported;
}

fn rd<T: sa_core::mem::GameData>(p: GPtr<u8>, off: isize) -> T {
    p.byte_add(off).cast::<T>().read()
}

/// `CPed::CreateDeadPedMoney()` (stub 0x4590F0 → 0x1564140, thiscall).
///
/// Nothing unless `CLocalisation::StealFromDeadPed()`; nothing for ped types (+0x598) 6, 0x12, 0x13, for mission
/// peds (+0x484 == 2) without flag 0x20000 of +0x470, in a vehicle (+0x46C bit 8), or with less than 10 money (u16
/// +0x756). Else `CPickups::CreateSomeMoney(position, money)` and money 0. The call is return-address obfuscated:
/// target `~[0x15604E4] ^ [0x64E4B7]` = 0x458970 (decoded from the image), returning to 0x15641E6.
#[hook(addr = 0x4590F0, conv = thiscall, name = "CPed::CreateDeadPedMoney", expect = "E9 4B B0 10 01",
       unverified = "intro3-11", route = "L2", hazards = "heap-writes", recs = "intro",
       note = "CreateSomeMoney reached through a decoded return-address-obfuscated call (0x458970)")]
pub fn ped_create_dead_ped_money(ped: GPtr<u8>) {
    if localisation_steal_from_dead_ped() == 0 {
        return;
    }
    if matches!(rd::<u32>(ped, 0x598), 6 | 0x12 | 0x13) {
        return;
    }
    if rd::<u8>(ped, 0x484) == 2 && rd::<u32>(ped, 0x470) & 0x20000 == 0 {
        return;
    }
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        return;
    }
    let money = rd::<u16>(ped, 0x756);
    if money < 10 {
        return;
    }
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let p = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
    pickups_create_some_money(rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8), u32::from(money));
    ped.byte_add(0x756).cast::<u16>().write(0);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_PED_CREATE_DEAD_PED_MONEY];
