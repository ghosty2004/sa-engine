//! `CPedDamageResponseCalculator` (PORT-WEAPON-PED). Ported from the 1.0 US bytes (gta-reversed
//! `PedDamageResponseCalculator.cpp`, `docs/re/specs/PED/CPedDamageResponseCalculator*.md`).
//!
//! Layout (0x14): +0 `m_pDamager` (CEntity*), +4 `m_fDamageFactor` (f32), +8 `m_bodyPart`, +0xC
//! `m_weaponType`, +0x10 `m_bSpeak` (bool). `CPedDamageResponse` (0xC): +0 damage to health, +4 damage to
//! armour, +8 flags.

use sa_core::abi::HookReg;
use sa_core::crt::rand;
use sa_core::orig::at_site;
use sa_core::{hook, orig};

use crate::common::*;

const _: () = assert!(sa_core::layouts::size::CPedDamageResponseCalculator == 0x14);

orig! {
    /// `CPed::IsPlayer` (0x5DF8F0)
    pub fn ped_is_player(ped: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => crate::ped::is_player;
    /// `CPedIntelligence::GetTaskFighting` (0x600F30)
    pub fn get_task_fighting(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x600F30 => unported;
    /// `CLocalisation::KillPeds` (0x56D2C0)
    pub fn localisation_kill_peds() -> u8 = cdecl 0x56D2C0 => unported;
    /// `CPedGroups::AreInSameGroup(const CPed*, const CPed*)` (0x5F7F40)
    pub fn are_in_same_group(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x5F7F40 => unported;
    /// `CPedIntelligence::AreFriends(const CPed&, const CPed&)` (0x601D10)
    pub fn are_friends(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x601D10 => unported;
    /// `std::min<float>(a, b)` instance (0x404330): `b` unless `a < b`
    pub fn min_f(a: f32, b: f32) -> f64 = cdecl 0x404330 => min_f_port;
}

pub fn min_f_port(a: f32, b: f32) -> f64 {
    if x(a).fcom(x(b)).test_parity(5) { f64::from(b) } else { f64::from(a) }
}

/// `CPedDamageResponseCalculator::CPedDamageResponseCalculator(CEntity* damager, float damage,
/// eWeaponType, ePedPieceTypes, bool speak)` (0x4AD3F0, `ret 0x14`): stores damager +0, damage +4, weapon
/// type (3rd argument) +8 ... as the bytes do: +8 = 4th argument, +0xC = 3rd argument; returns `this`.
#[hook(addr = 0x4AD3F0, conv = thiscall, name = "CPedDamageResponseCalculator::Constructor", expect = "8B 54 24 08 8B C1", this = "CPedDamageResponseCalculator")]
pub fn constructor(this: GPtr<u8>, damager: u32, damage: u32, a3: u32, a4: u32, speak: u32) -> GPtr<u8> {
    wr::<u32>(this, 0, damager);
    wr::<u32>(this, 4, damage);
    wr::<u32>(this, 8, a4);
    wr::<u32>(this, 0xC, a3);
    wr::<u8>(this, 0x10, speak as u8);
    this
}

/// `CPedDamageResponseCalculator::AccountForPedArmour(CPed*, CPedDamageResponse&)` (0x4AD550, `ret 8`):
/// nothing with armour exactly 0 or weapon type 0x35/0x36; stamps the local player's "last hit by" time;
/// damage <= armour (ordered): response armour = damage, armour -= damage, damage = 0; otherwise damage -=
/// armour, response armour = armour, armour = 0.
#[hook(addr = 0x4AD550, conv = thiscall, name = "CPedDamageResponseCalculator::AccountForPedArmour", expect = "56 57 8B 7C 24 0C", this = "CPedDamageResponseCalculator")]
pub fn account_for_ped_armour(this: GPtr<u8>, ped: GPtr<u8>, resp: GPtr<u8>) {
    if !x(rd::<f32>(ped, 0x548)).fcom(x(0.0)).test_parity(0x44) {
        return;
    }
    let w = rd::<i32>(this, 0xC);
    if w == 0x35 || w == 0x36 {
        return;
    }
    if find_player_ped(-1) == ped {
        let focus = usize::from(g::<u8>(PLAYER_IN_FOCUS).read());
        g::<u32>(0xB7CE80 + focus * PLAYER_INFO_SIZE).write(sa_core::timer::TIME_MS.get());
    }
    if !x(rd::<f32>(this, 4)).fcom(x(rd::<f32>(ped, 0x548))).test_parity(0x41) {
        wr::<u32>(resp, 4, rd::<u32>(this, 4));
        wr::<f32>(ped, 0x548, x(rd::<f32>(ped, 0x548)).fsub_m(rd::<f32>(this, 4)).to_f32());
        wr::<u32>(this, 4, 0);
    } else {
        wr::<f32>(this, 4, x(rd::<f32>(this, 4)).fsub_m(rd::<f32>(ped, 0x548)).to_f32());
        wr::<u32>(resp, 4, rd::<u32>(ped, 0x548));
        wr::<u32>(ped, 0x548, 0);
    }
}

/// `CPedDamageResponseCalculator::ComputeWillForceDeath(CPed*, CPedDamageResponse&)` (0x4AD610, `ret 8`,
/// AL): never for peds flagged +0x470 bit 12. By the weapon type's class (byte table 0x4AD7C4, index type
/// - 8): class 0 (player fist fight with a finishing move): `rand() & 1` (only for move kind 2) then
/// `rand() & 7 == 0`; class 1 (KillPeds localisation, not a player, not in a vehicle): `damage + 1.0 >
/// health`; class 2: a head shot (body part 9) unless the ped resists (`rand() & 7` for non-players,
/// except weapon types 0x21/0x22) and none of the exemptions (in-vehicle driver, player-pistol rules)
/// apply.
#[hook(addr = 0x4AD610, conv = thiscall, name = "CPedDamageResponseCalculator::ComputeWillForceDeath", expect = "51 56 8B 74 24 0C", this = "CPedDamageResponseCalculator")]
pub fn compute_will_force_death(this: GPtr<u8>, ped: GPtr<u8>, _resp: GPtr<u8>) -> u8 {
    if rd::<u32>(ped, 0x470) & 0x1000 != 0 {
        return 0;
    }
    const CLASS: [u8; 45] = [
        0, 3, 3, 3, 3, 3, 3, 3, 1, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 3, 2, 1, 3, 3, 3, 3, 3, 3,
        3, 3, 3, 3, 3, 1, 2,
    ];
    let w = rd::<i32>(this, 0xC);
    let idx = w.wrapping_sub(8) as u32;
    if idx > 0x2C {
        return 0;
    }
    match CLASS[idx as usize] {
        0 => {
            let src = rd::<GPtr<u8>>(this, 0);
            if src.is_null() || rd::<u8>(src, 0x36) & 7 != 3 || ped_is_player(src) == 0 {
                return 0;
            }
            let src = rd::<GPtr<u8>>(this, 0);
            let intel = || rd::<GPtr<u8>>(src, 0x47C);
            if get_task_fighting(intel()).is_null() {
                return 0;
            }
            if rd::<u8>(get_task_fighting(intel()), 0x24) != 0xB {
                return 0;
            }
            if rd::<u8>(get_task_fighting(intel()), 0x25) == 2 && at_site(0x4AD6B5, rand) & 1 != 0 {
                return 1;
            }
            (at_site(0x4AD6C2, rand) & 7 == 0) as u8
        }
        1 => {
            if localisation_kill_peds() == 0 || ped_is_player(ped) != 0 || rd::<u32>(ped, 0x46C) & 0x100 != 0 {
                return 0;
            }
            let d = x(rd::<f32>(this, 4)).fadd_m(1.0);
            (!d.fcom(x(rd::<f32>(ped, 0x540))).test_any(0x41)) as u8
        }
        2 => {
            let resist: i32 = if ped_is_player(ped) != 0 || rd::<u32>(ped, 0x470) & 0x1000 != 0 {
                1
            } else if w == 0x22 || w == 0x21 {
                0
            } else {
                at_site(0x4AD6F1, rand) & 7
            };
            let in_veh_exempt = rd::<u32>(ped, 0x46C) & 0x100 != 0
                && rd::<u32>(ped, 0x58C) != 0
                && ped_is_player(ped) == 0
                && rd::<u32>(ped, 0x470) & 0x1000 == 0;
            let check = if in_veh_exempt {
                true
            } else {
                let mut by_player = false;
                if ped_is_player(ped) == 0 {
                    let src = rd::<GPtr<u8>>(this, 0);
                    if !src.is_null() && rd::<u8>(src, 0x36) & 7 == 3 && ped_is_player(src) != 0 {
                        let pd = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0), 0x480);
                        by_player = rd::<i32>(pd, 0x70) == 5 || rd::<u8>(pd, 0x34) & 8 != 0;
                    }
                }
                by_player || resist == 0
            };
            (check && rd::<i32>(this, 8) == 9) as u8
        }
        _ => 0,
    }
}

/// `CPedDamageResponseCalculator::AccountForPedDamageStats(CPed*, CPedDamageResponse&)` (0x4AD430, `ret
/// 8`): damage x 0.33 for the player, else x the ped stat's +0x2C; then, for friendly fire (a player
/// hitting a ped of their group, a ped hitting a friendly player, a group member hit by a ped of the same
/// group with ped +0x484 == 2; not for weapon types 0x33/0x25 or the 5555.55 "instant kill" damage),
/// damage = `min(12.5, damage * 0.1)`.
#[hook(addr = 0x4AD430, conv = thiscall, name = "CPedDamageResponseCalculator::AccountForPedDamageStats", expect = "53 56 8B F1 D9 46 04", this = "CPedDamageResponseCalculator")]
pub fn account_for_ped_damage_stats(this: GPtr<u8>, ped: GPtr<u8>, _resp: GPtr<u8>) {
    // [0x8A6260] is a .data f32 (5555.55, the instant-kill damage)
    let kill = !x(rd::<f32>(this, 4)).fcom(x(g::<f32>(0x8A6260).read())).test_parity(0x44);
    let d = if ped_is_player(ped) != 0 {
        x(rd::<f32>(this, 4)).fmul_m(f32::from_bits(0x3EA8_F5C3))
    } else {
        x(rd::<f32>(rd::<GPtr<u8>>(ped, 0x59C), 0x2C)).fmul_m(rd::<f32>(this, 4))
    };
    wr::<f32>(this, 4, d.to_f32());
    let scale = 'f: {
        let src = rd::<GPtr<u8>>(this, 0);
        if !src.is_null() && rd::<u8>(src, 0x36) & 7 == 3 && ped_is_player(src) != 0 {
            let w = rd::<i32>(this, 0xC);
            break 'f are_in_same_group(ped, rd::<GPtr<u8>>(this, 0)) != 0 && w != 0x33 && w != 0x25 && !kill;
        }
        if !ped.is_null() && ped_is_player(ped) != 0 {
            let s = rd::<GPtr<u8>>(this, 0);
            if s.is_null() {
                break 'f false;
            }
            if rd::<u8>(s, 0x36) & 7 == 3 && are_friends(ped, s) != 0 {
                break 'f true;
            }
        }
        let s = rd::<GPtr<u8>>(this, 0);
        if s.is_null() || rd::<u8>(s, 0x36) & 7 != 3 || rd::<u8>(ped, 0x484) != 2 {
            break 'f false;
        }
        if are_in_same_group(ped, s) == 0 {
            break 'f false;
        }
        let w = rd::<i32>(this, 0xC);
        w != 0x33 && w != 0x25
    };
    if scale {
        let v = x(rd::<f32>(this, 4)).fmul_m(0.1).to_f32();
        wr::<f32>(this, 4, X87::from_f64_exact(min_f(12.5, v)).to_f32());
    }
}

// --- hook list (generated) ---

pub const HOOKS: &[HookReg] = &[
    HOOK_CONSTRUCTOR,
    HOOK_ACCOUNT_FOR_PED_ARMOUR,
    HOOK_COMPUTE_WILL_FORCE_DEATH,
    HOOK_ACCOUNT_FOR_PED_DAMAGE_STATS,
];
