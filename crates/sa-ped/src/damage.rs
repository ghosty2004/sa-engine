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


orig! {
    /// `CPed::Say(uint16 phrase, uint32 startTime, float volume, bool, bool, bool)` (0x5EFFE0)
    fn ped_say(ped: GPtr<u8>, phrase: u32, start: u32, vol: f32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `CPedDamageResponseCalculator::ComputeWillKillPed(CPed*, CPedDamageResponse&, bool)` (0x4B3210)
    fn compute_will_kill_ped_call(this: GPtr<u8>, ped: GPtr<u8>, r: GPtr<u8>, speak: u32) = thiscall 0x4B3210 => compute_will_kill_ped;
    /// `CPedDamageResponseCalculator::AccountForPedDamageStats` (0x4AD430)
    fn account_for_ped_damage_stats_call(this: GPtr<u8>, ped: GPtr<u8>, r: GPtr<u8>) = thiscall 0x4AD430 => account_for_ped_damage_stats;
    /// `CPedDamageResponseCalculator::AccountForPedArmour` (0x4AD550)
    fn account_for_ped_armour_call(this: GPtr<u8>, ped: GPtr<u8>, r: GPtr<u8>) = thiscall 0x4AD550 => account_for_ped_armour;
    /// `CPedDamageResponseCalculator::ComputeWillForceDeath` (0x4AD610)
    fn compute_will_force_death_call(this: GPtr<u8>, ped: GPtr<u8>, r: GPtr<u8>) -> u8 = thiscall 0x4AD610 => compute_will_force_death;
    /// `CPlayerPed::AnnoyPlayerPed(bool)` (0x60A040)
    fn player_on_hit_60a040(p: GPtr<u8>, b: u32) = thiscall 0x60A040 => unported;
    /// `CEventDamage::CEventDamage(CEntity*, uint32, eWeaponType, ePedPieceTypes, uint8, bool, bool)` (0x4AD830)
    fn event_damage_ctor(e: GPtr<u8>, src: GPtr<u8>, t: u32, wt: u32, part: u32, dir: u32, a: u32, b: u32) = thiscall 0x4AD830 => unported;
    /// `CEventDamage::~CEventDamage()` (0x4AD960)
    fn event_damage_dtor(e: GPtr<u8>) = thiscall 0x4AD960 => unported;
    /// `CPlayerPed::MakeGroupRespondToPlayerTakingDamage(CEventDamage&)` (0x60A110)
    fn make_group_respond_to_player_taking_damage(p: GPtr<u8>, e: GPtr<u8>) = thiscall 0x60A110 => unported;
    /// `CPedGroupMembership`-style notification of the damaging player's group (0x5F6A10, on group + 0x...)
    fn group_notify_damaged(g: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x5F6A10 => unported;
}

fn say(ped: GPtr<u8>, phrase: u32) {
    ped_say(ped, phrase, 0, 1.0, 0, 0, 0);
}

/// `CPedDamageResponseCalculator::ComputeWillKillPed(CPed*, CPedDamageResponse&, bool speak)` (0x4B3210,
/// `ret 0xC`). The player with the invulnerability switch 0x96916D takes no damage from weapon types
/// < 0x2F. Otherwise `forceDeath` (+9) = `ComputeWillForceDeath`; with the switch 0x969173 weapon types
/// 0/1/0x2E deal the whole health. Force death or `health - damage < 1.0` (unordered too) kills: +8 = 1,
/// +0 = health, health = 0. Weapon 0x36 with `+0x10` set: health = `max(health - damage, 5.0)` (the x87
/// difference kept unrounded for `+0 = old - new`), phrase 0x159 when speaking. Otherwise +8 = 0, +0 =
/// damage, health -= damage; when speaking: phrase 0x165 for weapon 0x35, else 0x158 unless damage < 5.0
/// and the old health > 10.0 (then 0x159). The original writes the old/new health into its first
/// argument slot (stack-only, not reproduced).
#[hook(addr = 0x4B3210, conv = thiscall, name = "CPedDamageResponseCalculator::ComputeWillKillPed", expect = "53 55 56 8B 74 24 10", this = "CPedDamageResponseCalculator")]
pub fn compute_will_kill_ped(this: GPtr<u8>, ped: GPtr<u8>, r: GPtr<u8>, speak: u32) {
    if ped_is_player(ped) != 0 && g::<u8>(0x96916D).read() != 0 && rd::<i32>(this, 0xC) < 0x2F {
        return;
    }
    let old = rd::<f32>(ped, 0x540);
    let fd = compute_will_force_death_call(this, ped, r);
    wr::<u8>(r, 9, fd);
    if g::<u8>(0x969173).read() != 0 && matches!(rd::<i32>(this, 0xC), 0 | 1 | 0x2E) {
        wr::<u32>(this, 4, rd::<u32>(ped, 0x540));
    }
    let kill = |ped: GPtr<u8>, r: GPtr<u8>| {
        wr::<u8>(r, 8, 1);
        wr::<u32>(r, 0, rd::<u32>(ped, 0x540));
        wr::<u32>(ped, 0x540, 0);
    };
    if rd::<u8>(r, 9) != 0 {
        kill(ped, r);
        return;
    }
    let five = f32::from_bits(0x40A0_0000);
    if rd::<i32>(this, 0xC) == 0x36 && rd::<u8>(this, 0x10) != 0 {
        wr::<u8>(r, 8, 0);
        let mut v = x(rd::<f32>(ped, 0x540)).fsub_m(rd::<f32>(this, 4));
        if !x(five).fcom(v).test_any(0x41) {
            v = x(five);
        }
        wr::<f32>(ped, 0x540, v.to_f32());
        wr::<f32>(r, 0, x(old).fsub(v).to_f32());
        if speak as u8 != 0 {
            say(ped, 0x159);
        }
        return;
    }
    if x(rd::<f32>(ped, 0x540)).fsub_m(rd::<f32>(this, 4)).fcom(X87::one()).test_any(1) {
        kill(ped, r);
        return;
    }
    wr::<u8>(r, 8, 0);
    wr::<u32>(r, 0, rd::<u32>(this, 4));
    let h = x(rd::<f32>(ped, 0x540)).fsub_m(rd::<f32>(this, 4)).to_f32();
    wr::<f32>(ped, 0x540, h);
    if speak as u8 == 0 {
        return;
    }
    if rd::<i32>(this, 0xC) == 0x35 {
        say(ped, 0x165);
        return;
    }
    if !x(rd::<f32>(this, 4)).fcom(x(five)).test_parity(5)
        && !x(old).fcom(x(10.0)).test_any(0x41)
    {
        say(ped, 0x159);
    } else {
        say(ped, 0x158);
    }
}

/// `CPedDamageResponseCalculator::ComputeDamageResponse(CPed*, CPedDamageResponse&, bool speak)`
/// (0x4B5AC0, `ret 0xC`). Nothing when the response is already computed (+0xA). Otherwise the response is
/// reset (+0/+4 = 0, +8/+9 = 0, +0xA/+0xB = 1), damage stats, armour and `ComputeWillKillPed`; a damager
/// that is the player ped or the player's vehicle hitting another ped for health+armour damage > 3.0
/// counts a hit on the focused player's info (+0x140); a player victim calls `AnnoyPlayerPed(false)` on the player ped;
/// a player victim hit by a ped gets a `CEventDamage` (Rust stack buffer, original ctor/dtor; the
/// in-vehicle flag pushed as `(flags >> 8) & 0xFFFFFF01` as the original) for
/// `MakeGroupRespondToPlayerTakingDamage`. With damage > 0: non-player victims of weapon types 4/8/9 get
/// +0x72F = 200; the player victim stamps the time into the player info (+0xE4); +0x760 = weapon type
/// byte; with a damager, the last-damage entity (+0x764, registered reference) and time (+0x768), and a
/// damaging player ped notifies its group (0x5F6A10 on `0xC09928 + group * 0x2D4`). SEH frame not
/// reproduced.
#[hook(addr = 0x4B5AC0, conv = thiscall, name = "CPedDamageResponseCalculator::ComputeDamageResponse", expect = "64 A1 00 00 00 00", this = "CPedDamageResponseCalculator")]
pub fn compute_damage_response(this: GPtr<u8>, ped: GPtr<u8>, r: GPtr<u8>, speak: u32) {
    if rd::<u8>(r, 0xA) != 0 {
        return;
    }
    wr::<u32>(r, 0, 0);
    wr::<u32>(r, 4, 0);
    wr::<u8>(r, 8, 0);
    wr::<u8>(r, 9, 0);
    wr::<u8>(r, 0xA, 1);
    wr::<u8>(r, 0xB, 1);
    account_for_ped_damage_stats_call(this, ped, r);
    account_for_ped_armour_call(this, ped, r);
    compute_will_kill_ped_call(this, ped, r, speak);
    let three = f32::from_bits(0x4040_0000);
    let src = rd::<GPtr<u8>>(this, 0);
    if !src.is_null() {
        let by_player = src == find_player_ped(-1) || rd::<GPtr<u8>>(this, 0) == find_player_vehicle(-1, 0);
        if by_player
            && src != ped
            && !x(rd::<f32>(r, 0)).fadd_m(rd::<f32>(r, 4)).fcom(x(three)).test_any(0x41)
        {
            let i = usize::from(g::<u8>(PLAYER_IN_FOCUS).read()) * PLAYER_INFO_SIZE;
            let c = g::<u32>(0xB7CED8 + i);
            c.write(c.read().wrapping_add(1));
        }
    }
    if ped == find_player_ped(-1) {
        player_on_hit_60a040(find_player_ped(-1), 0);
    }
    if ped_is_player(ped) != 0 {
        let src = rd::<GPtr<u8>>(this, 0);
        if !src.is_null() && rd::<u8>(src, 0x36) & 7 == 3 {
            let flag = (rd::<u32>(ped, 0x46C) >> 8) & 0xFFFF_FF01;
            let mut ev = [0u32; sa_core::layouts::size::CEventDamage / 4];
            let e = lp(&mut ev).cast::<u8>();
            event_damage_ctor(e, src, 0, rd::<u32>(this, 0xC), rd::<u32>(this, 8), 0, 0, flag);
            make_group_respond_to_player_taking_damage(ped, e);
            event_damage_dtor(e);
        }
    }
    if x(rd::<f32>(r, 0)).fadd_m(rd::<f32>(r, 4)).fcom(X87::zero()).test_any(0x41) {
        return;
    }
    if ped_is_player(ped) == 0 && matches!(rd::<i32>(this, 0xC), 4 | 8 | 9) {
        wr::<u8>(ped, 0x72F, 0xC8);
    }
    if ped == find_player_ped(-1) {
        let i = usize::from(g::<u8>(PLAYER_IN_FOCUS).read()) * PLAYER_INFO_SIZE;
        g::<u32>(0xB7CE7C + i).write(g::<u32>(0xB7CB84).read());
    }
    wr::<u8>(ped, 0x760, rd::<u8>(this, 0xC));
    let src = rd::<GPtr<u8>>(this, 0);
    if src.is_null() {
        return;
    }
    wr::<GPtr<u8>>(ped, 0x764, src);
    crate::ped::register_reference(src, ped.byte_add(0x764).cast());
    wr::<u32>(ped, 0x768, g::<u32>(0xB7CB84).read());
    let s = rd::<GPtr<u8>>(this, 0);
    if rd::<u8>(s, 0x36) & 7 == 3 && ped_is_player(s) != 0 {
        let grp = rd::<u32>(rd::<GPtr<u8>>(find_player_ped(-1), 0x480), 0x38);
        group_notify_damaged(GPtr::from_addr((grp.wrapping_mul(0x2D4)).wrapping_add(0xC09928) as usize), ped);
    }
}

// --- hook list (generated) ---

pub const HOOKS: &[HookReg] = &[
    HOOK_CONSTRUCTOR,
    HOOK_ACCOUNT_FOR_PED_ARMOUR,
    HOOK_COMPUTE_WILL_FORCE_DEATH,
    HOOK_ACCOUNT_FOR_PED_DAMAGE_STATS,
    HOOK_COMPUTE_WILL_KILL_PED,
    HOOK_COMPUTE_DAMAGE_RESPONSE,
];
