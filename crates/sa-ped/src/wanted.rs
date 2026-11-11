//! `CWanted` / `CCrime` leaves (PORT-WEAPON-PED). Ported from the 1.0 US bytes (gta-reversed `Wanted.cpp`,
//! `Crime.cpp` and `docs/re/specs/PED/CWanted.deviations.md` as reading aids).
//!
//! `CWanted` (0x29C): +0 chaos level, +4 chaos level before parole, +0xC last time wanted level changed,
//! +0x10 time of parole, +0x14 multiplier (f32), +0x18 cops in pursuit (u8), +0x19 max cops in pursuit
//! (u8), +0x1A max cop cars (u8), +0x1B, +0x1C road block size (u16), +0x1E flags (bit 0..2 everybody
//! backoff / gangs / drop to zero, bit 3 swat, 4 fbi, 5 army), +0x28, +0x2C wanted level, +0x30 wanted
//! level before parole, +0x34 crimes being queued (16 x 0x1C: type +0, ped +4, time +8, coors +0xC, reported
//! +0x18, police don't care +0x19), +0x1F4 pursuit cops [10], +0x21C police scanner audio entity, +0x298.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

const _: () = assert!(sa_core::layouts::size::CWanted == 0x29C);

sa_core::globals! {
    list GLOBALS;
    /// `CWanted::MaximumWantedLevel`
    pub static MAXIMUM_WANTED_LEVEL @ 0x8CDEE4: i32 = 6;
    /// `CWanted::nMaximumWantedLevel` (chaos)
    pub static MAXIMUM_CHAOS_LEVEL @ 0x8CDEE8: i32 = 0x23F0;
    /// `CWanted::bUseNewsHeliInAdditionToPolice`
    pub static USE_NEWS_HELI @ 0xB7CB8C: u8 = 0;
}

/// `CCheat::m_aCheatsActive[...]` byte tested by `SetWantedLevel` / `ReportCrimeNow` (never-wanted cheat)
const NEVER_WANTED: usize = 0x969171;
/// `CGame::currArea` (u16 at 0x96A704 compared with 1 by `ReportCrimeNow`)
const CURR_AREA_WORD: usize = 0x96A704;
const CRIMES: isize = 0x34;
const CRIME_SIZE: isize = 0x1C;
const COPS: isize = 0x1F4;

orig! {
    /// `CWanted::AddCrimeToQ` (0x562000)
    pub fn add_crime_to_q_call(this: GPtr<u8>, ty: i32, ped: GPtr<u8>, coors: GPtr<f32>, reported: u8, dont_care: u8) -> u8 = thiscall 0x562000 => add_crime_to_q;
    /// `CWanted::Initialise` (0x562390)
    pub fn initialise_call(this: GPtr<u8>) = thiscall 0x562390 => initialise;
    /// `CWanted::UpdateWantedLevel` (0x561C90)
    pub fn update_wanted_level_call(this: GPtr<u8>) = thiscall 0x561C90 => update_wanted_level;
    /// `CWanted::ReportCrimeNow` (0x562120)
    pub fn report_crime_now_call(this: GPtr<u8>, ty: i32, coors: GPtr<f32>, dont_care: u8) = thiscall 0x562120 => report_crime_now;
    /// `CWanted::ComputePursuitCopToDisplace` (0x562B00)
    pub fn compute_pursuit_cop_to_displace_call(cop: GPtr<u8>, cops: GPtr<GPtr<u8>>) -> GPtr<u8> = cdecl 0x562B00 => compute_pursuit_cop_to_displace;
    /// `CWanted::RemovePursuitCop(CCopPed*, CCopPed**, uint8&)` (0x562300)
    pub fn remove_pursuit_cop_func_call(cop: GPtr<u8>, cops: GPtr<GPtr<u8>>, n: GPtr<u8>) = cdecl 0x562300 => remove_pursuit_cop_func;
    /// `CWanted::CanCopJoinPursuit(CCopPed*, uint8, CCopPed**, uint8&)` (0x562F60)
    pub fn can_cop_join_pursuit_func_call(cop: GPtr<u8>, max: u32, cops: GPtr<GPtr<u8>>, n: GPtr<u8>) -> u8 = cdecl 0x562F60 => can_cop_join_pursuit_func;
    /// `CWanted::CanCopJoinPursuit(CCopPed*)` (0x562FB0)
    pub fn can_cop_join_pursuit_method_call(this: GPtr<u8>, cop: GPtr<u8>) -> u8 = thiscall 0x562FB0 => can_cop_join_pursuit_method;
    /// `CWanted::SetWantedLevel(int32)` (0x562470)
    pub fn set_wanted_level_call(this: GPtr<u8>, level: i32) = thiscall 0x562470 => set_wanted_level;
    /// `CWanted::SetWantedLevelNoDrop` (0x562570)
    pub fn set_wanted_level_no_drop_call(this: GPtr<u8>, level: i32) = thiscall 0x562570 => set_wanted_level_no_drop;
    /// `CGangWars::GangWarFightingGoingOn` (0x443AC0)
    pub fn gang_war_fighting_going_on() -> u8 = cdecl 0x443AC0 => unported;
    /// `CAEPoliceScannerAudioEntity::AddAudioEvent(int32 event, eCrimeType, const CVector&)` (0x4E71E0)
    pub fn police_scanner_add_audio_event(this: GPtr<u8>, ev: i32, ty: i32, coors: GPtr<f32>) = thiscall 0x4E71E0 => unported;
}

fn crime(this: GPtr<u8>, i: u32) -> GPtr<u8> {
    this.byte_add(CRIMES + CRIME_SIZE * i as isize)
}

fn cop(this: GPtr<u8>, i: u32) -> GPtr<GPtr<u8>> {
    this.byte_add(COPS + 4 * i as isize).cast()
}

/// `(p - q)` components then `sqrt((dz*dz + dy*dy) + dx*dx)` as the original's x87 sequence computes it.
fn dist3_zyx(p: [f32; 3], q: [f32; 3]) -> X87 {
    let dx = x(p[0]).fsub_m(q[0]);
    let dy = x(p[1]).fsub_m(q[1]);
    let dz = x(p[2]).fsub_m(q[2]);
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx))
}

/// `(dz*dz + dx*dx) + dy*dy` (the order `IsClosestCop` uses).
fn dist2_zxy(p: [f32; 3], q: [f32; 3]) -> X87 {
    let dx = x(p[0]).fsub_m(q[0]);
    let dy = x(p[1]).fsub_m(q[1]);
    let dz = x(p[2]).fsub_m(q[2]);
    dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy))
}

/// `CCrime::FindImmediateDetectionRange(eCrimeType)` (0x531FC0, cdecl, ST0): 60 for types 0x14, 30 for
/// 0xC/0x10/0x11/0x12/0x13 ... per the byte table 0x531FFC (index type - 0xC), else 14.
#[hook(addr = 0x531FC0, conv = cdecl, name = "CCrime::FindImmediateDetectionRange", preserve = "ecx", expect = "8B 44 24 04 83 C0 F4")]
pub fn find_immediate_detection_range(ty: i32) -> X87 {
    // byte table at 0x531FFC: [0, 2, 2, 2, 0, 0, 2, 2, 1] → targets 0x531FDA (60), 0x531FE8 (14), 0x531FE1 (30)
    const TABLE: [u8; 9] = [0, 2, 2, 2, 0, 0, 2, 2, 1];
    let i = ty.wrapping_sub(0xC) as u32;
    let v = if i > 8 {
        14.0
    } else {
        match TABLE[i as usize] {
            0 => 60.0,
            1 => 30.0,
            _ => 14.0,
        }
    };
    x(v)
}

/// `CWanted::InitialiseStaticVariables` (0x561C70)
#[hook(addr = 0x561C70, conv = cdecl, name = "CWanted::InitialiseStaticVariables", expect = "C7 05 E4 DE 8C 00 06 00 00 00")]
pub fn initialise_static_variables() {
    MAXIMUM_WANTED_LEVEL.set(6);
    MAXIMUM_CHAOS_LEVEL.set(0x23F0);
    USE_NEWS_HELI.set(0);
}

/// `CWanted::SetMaximumWantedLevel(int32)` (0x561E70): table of (level, chaos) for 0..6; 0 → (0, 0); other
/// values leave both unchanged.
#[hook(addr = 0x561E70, conv = cdecl, name = "CWanted::SetMaximumWantedLevel", preserve = "ecx", expect = "8B 44 24 04 83 F8 06")]
pub fn set_maximum_wanted_level(level: i32) {
    let (l, c) = match level {
        0 => (0, 0),
        1 => (1, 0x73),
        2 => (2, 0x16D),
        3 => (3, 0x36B),
        4 => (4, 0x708),
        5 => (5, 0xDAC),
        6 => (6, 0x1AF4),
        _ => return,
    };
    MAXIMUM_WANTED_LEVEL.set(l);
    MAXIMUM_CHAOS_LEVEL.set(c);
}

/// `CWanted::AreMiamiViceRequired` (0x561F30): wanted level >= 3 (EAX 0/1).
#[hook(addr = 0x561F30, conv = thiscall, name = "CWanted::AreMiamiViceRequired", expect = "8B 51 2C 33 C0", this = "CWanted")]
pub fn are_miami_vice_required(this: GPtr<u8>) -> u32 {
    (rd::<i32>(this, 0x2C) >= 3) as u32
}

fn level_or_flag(this: GPtr<u8>, level: i32, bit: u8) -> u32 {
    (rd::<i32>(this, 0x2C) == level || rd::<u8>(this, 0x1E) & bit != 0) as u32
}

/// `CWanted::AreSwatRequired` (0x561F40): level 4 or flag 0x08.
#[hook(addr = 0x561F40, conv = thiscall, name = "CWanted::AreSwatRequired", expect = "83 79 2C 04 74 09", this = "CWanted")]
pub fn are_swat_required(this: GPtr<u8>) -> u32 {
    level_or_flag(this, 4, 8)
}

/// `CWanted::AreFbiRequired` (0x561F60): level 5 or flag 0x10.
#[hook(addr = 0x561F60, conv = thiscall, name = "CWanted::AreFbiRequired", expect = "83 79 2C 05 74 09", this = "CWanted")]
pub fn are_fbi_required(this: GPtr<u8>) -> u32 {
    level_or_flag(this, 5, 0x10)
}

/// `CWanted::AreArmyRequired` (0x561F80): level 6 or flag 0x20.
#[hook(addr = 0x561F80, conv = thiscall, name = "CWanted::AreArmyRequired", expect = "83 79 2C 06 74 09", this = "CWanted")]
pub fn are_army_required(this: GPtr<u8>) -> u32 {
    level_or_flag(this, 6, 0x20)
}

/// `CWanted::NumOfHelisRequired` (0x561FA0): 0 with any of flags 0..2; level 3 → 1, 4..6 → 2, else 0.
#[hook(addr = 0x561FA0, conv = thiscall, name = "CWanted::NumOfHelisRequired", expect = "F6 41 1E 07 74 03", this = "CWanted")]
pub fn num_of_helis_required(this: GPtr<u8>) -> u32 {
    if rd::<u8>(this, 0x1E) & 7 != 0 {
        return 0;
    }
    match rd::<i32>(this, 0x2C) {
        3 => 1,
        4..=6 => 2,
        _ => 0,
    }
}

/// `CWanted::AddCrimeToQ(eCrimeType, CEntity* ped, const CVector& coors, bool alreadyReported, bool
/// policeDontCare)` (0x562000, `ret 0x14`, AL): an entry with the same type and ped: returns 1 if it was
/// reported, else sets its reported flag from the argument (when non-zero) and returns 0. Otherwise the
/// first free entry (type 0) gets every field; returns 0.
#[hook(addr = 0x562000, conv = thiscall, name = "CWanted::AddCrimeToQ", expect = "53 56 8B 74 24 10", this = "CWanted")]
pub fn add_crime_to_q(this: GPtr<u8>, ty: i32, ped: GPtr<u8>, coors: GPtr<f32>, reported: u8, dont_care: u8) -> u8 {
    for i in 0..16 {
        let c = crime(this, i);
        if rd::<i32>(c, 0) == ty && rd::<GPtr<u8>>(c, 4) == ped {
            if rd::<u8>(c, 0x18) != 0 {
                return 1;
            }
            if reported != 0 {
                wr::<u8>(c, 0x18, reported);
            }
            return 0;
        }
    }
    for i in 0..16 {
        let c = crime(this, i);
        if rd::<i32>(c, 0) == 0 {
            wr::<GPtr<u8>>(c, 4, ped);
            wr::<i32>(c, 0, ty);
            wr::<u32>(c, 0xC, coors.cast::<u32>().read());
            wr::<u32>(c, 0x10, coors.cast::<u32>().wrapping_index(1).read());
            wr::<u32>(c, 0x14, coors.cast::<u32>().wrapping_index(2).read());
            wr::<u32>(c, 8, sa_core::timer::TIME_MS.get());
            wr::<u8>(c, 0x18, reported);
            wr::<u8>(c, 0x19, dont_care);
            return 0;
        }
    }
    0
}

/// `CWanted::RemovePursuitCop(CCopPed*, CCopPed** cops, uint8& count)` (0x562300, cdecl): clears the first
/// matching slot of 10 and decrements `count`.
#[hook(addr = 0x562300, conv = cdecl, name = "CWanted::RemovePursuitCop-func", expect = "8B 4C 24 08 8B 54 24 04")]
pub fn remove_pursuit_cop_func(c: GPtr<u8>, cops: GPtr<GPtr<u8>>, n: GPtr<u8>) {
    for i in 0..10 {
        if cops.wrapping_index(i).read() == c {
            cops.wrapping_index(i).write(GPtr::NULL);
            n.write(n.read().wrapping_sub(1));
            return;
        }
    }
}

/// `CWanted::IsInPursuit(CCopPed*)` (0x562330, `ret 4`, AL)
#[hook(addr = 0x562330, conv = thiscall, name = "CWanted::IsInPursuit", expect = "8B 54 24 04 33 C0", this = "CWanted")]
pub fn is_in_pursuit(this: GPtr<u8>, c: GPtr<u8>) -> u8 {
    (0..10).any(|i| cop(this, i).read() == c) as u8
}

/// `CWanted::Initialise` (0x562390)
#[hook(addr = 0x562390, conv = thiscall, name = "CWanted::Initialise", expect = "8B D1 8A 42 1E", this = "CWanted")]
pub fn initialise(this: GPtr<u8>) {
    wr::<u8>(this, 0x1E, rd::<u8>(this, 0x1E) & 0xC0);
    for o in [0, 4, 8, 0xC, 0x10] {
        wr::<u32>(this, o, 0);
    }
    wr::<u8>(this, 0x18, 0);
    wr::<u8>(this, 0x19, 0);
    wr::<u8>(this, 0x1A, 0);
    wr::<u16>(this, 0x1C, 0);
    wr::<u32>(this, 0x14, 0x3F80_0000);
    wr::<u8>(this, 0x28, 0);
    wr::<u8>(this, 0x298, 0);
    wr::<u32>(this, 0x2C, 0);
    wr::<u32>(this, 0x30, 0);
    wr::<u8>(this, 0x1B, 0);
    for i in 0..10 {
        cop(this, i).write(GPtr::NULL);
    }
    for i in 0..16 {
        wr::<u32>(crime(this, i), 0, 0);
    }
}

/// `CWanted::RemovePursuitCop(CCopPed*)` (0x562C10, `ret 4`)
#[hook(addr = 0x562C10, conv = thiscall, name = "CWanted::RemovePursuitCop-method", expect = "8B 54 24 04 33 C0", this = "CWanted")]
pub fn remove_pursuit_cop_method(this: GPtr<u8>, c: GPtr<u8>) {
    for i in 0..10 {
        if cop(this, i).read() == c {
            cop(this, i).write(GPtr::NULL);
            wr::<u8>(this, 0x18, rd::<u8>(this, 0x18).wrapping_sub(1));
            return;
        }
    }
}

/// `CWanted::UpdateEachFrame` (0x562360, cdecl): sets `bUseNewsHeliInAdditionToPolice` when the local
/// player's wanted needs no police helicopter (the `NumOfHelisRequired` test, inlined).
#[hook(addr = 0x562360, conv = cdecl, name = "CWanted::UpdateEachFrame", expect = "6A FF E8 C9 BE 00 00")]
pub fn update_each_frame() {
    let w = find_player_wanted(-1);
    let need = rd::<u8>(w, 0x1E) & 7 == 0 && matches!(rd::<i32>(w, 0x2C), 3..=6);
    if !need {
        USE_NEWS_HELI.set(1);
    }
}

/// `CWanted::Reset` (0x562400): `jmp CWanted::Initialise`.
#[hook(addr = 0x562400, conv = thiscall, name = "CWanted::Reset", expect = "E9 8B FF FF FF 90", this = "CWanted")]
pub fn reset(this: GPtr<u8>) {
    initialise_call(this);
}

/// `CWanted::RegisterCrime(eCrimeType, const CVector& coors, CPed* ped, bool policeDontCare)` (0x562410,
/// `ret 0x10`): `AddCrimeToQ(type, ped, coors, false, policeDontCare)`.
#[hook(addr = 0x562410, conv = thiscall, name = "CWanted::RegisterCrime", expect = "8B 44 24 10 8B 54 24 08", this = "CWanted")]
pub fn register_crime(this: GPtr<u8>, ty: i32, coors: GPtr<f32>, ped: GPtr<u8>, dont_care: u8) {
    add_crime_to_q_call(this, ty, ped, coors, 0, dont_care);
}

/// `CWanted::RegisterCrime_Immediately(eCrimeType, const CVector&, CPed*, bool)` (0x562430, `ret 0x10`):
/// `AddCrimeToQ(type, ped, coors, true, dontCare)`; when it returns false, `ReportCrimeNow(type, coors,
/// dontCare)`.
#[hook(addr = 0x562430, conv = thiscall, name = "CWanted::RegisterCrime_Immediately", expect = "8B 44 24 0C 53", this = "CWanted")]
pub fn register_crime_immediately(this: GPtr<u8>, ty: i32, coors: GPtr<f32>, ped: GPtr<u8>, dont_care: u8) {
    if add_crime_to_q_call(this, ty, ped, coors, 1, dont_care) == 0 {
        report_crime_now_call(this, ty, coors, dont_care);
    }
}

/// `CWanted::WorkOutPolicePresence(CVector pos, float radius)` (0x5625F0, cdecl, EAX): alive cops (ped
/// type 6) in the ped pool and law-enforcer vehicles (flag +0x428 bit 0, or model 497) in the vehicle
/// pool, other than the player's vehicle and not wrecked/blown (status 4/5), within `radius` (x87 sqrt
/// of `(dz^2 + dy^2) + dx^2` < radius, NaN not counted). Both pools are walked from the last slot down.
#[hook(addr = 0x5625F0, conv = cdecl, name = "CWanted::WorkOutPolicePresence", expect = "51 53 8B 1D 90 44 B7 00")]
pub fn work_out_police_presence(px: f32, py: f32, pz: f32, radius: f32) -> i32 {
    let p = [px, py, pz];
    let within = |e: GPtr<u8>| {
        let d = dist3_zyx(p, vec3(ent_pos(e))).fsqrt();
        !d.fcom(x(radius)).test_parity(5)
    };
    let mut n = 0i32;
    let peds = g::<GPtr<u8>>(PED_POOL).read();
    let mut i = pool_size(peds);
    while i != 0 {
        i -= 1;
        if (pool_flag(peds, i as u32) as i8) < 0 {
            continue;
        }
        let ped = pool_slot(peds, i as u32, PED_SLOT);
        if ped.is_null() || rd::<i32>(ped, 0x598) != 6 || ped_is_alive(ped) == 0 {
            continue;
        }
        if within(ped) {
            n += 1;
        }
    }
    let vehs = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let mut i = pool_size(vehs);
    while i != 0 {
        i -= 1;
        if (pool_flag(vehs, i as u32) as i8) < 0 {
            continue;
        }
        let v = pool_slot(vehs, i as u32, VEHICLE_SLOT);
        if v.is_null() {
            continue;
        }
        if rd::<u8>(v, 0x428) & 1 == 0 && rd::<u16>(v, 0x22) != 0x1F1 {
            continue;
        }
        if v == find_player_vehicle(-1, 0) {
            continue;
        }
        let st = rd::<u8>(v, 0x36) >> 3;
        if st == 4 || st == 5 {
            continue;
        }
        if within(v) {
            n += 1;
        }
    }
    n
}

/// `CWanted::IsClosestCop(CPed* ped, int32 numCopsToCheck)` (0x5627D0, `ret 8`, AL): the non-null pursuit
/// cops are ranked by squared distance to the local player (f32, `(dz^2 + dx^2) + dy^2`); the
/// `numCopsToCheck` closest (strictly smaller wins, first on ties, NaN never) are taken in turn; true if
/// `ped` is one of them. Counts above 10 overrun the original's stack arrays (not modelled).
#[hook(addr = 0x5627D0, conv = thiscall, name = "CWanted::IsClosestCop", expect = "8B 81 F4 01 00 00", this = "CWanted")]
pub fn is_closest_cop(this: GPtr<u8>, ped: GPtr<u8>, num: i32) -> u8 {
    let mut cops = [GPtr::<u8>::NULL; 10];
    let mut n = 0usize;
    for i in 0..10 {
        let c = cop(this, i).read();
        if !c.is_null() {
            cops[n] = c;
            n += 1;
        }
    }
    let mut dists = [f32::MAX; 10];
    let mut out = [GPtr::<u8>::NULL; 10];
    let pp = vec3(ent_pos(find_player_ped(-1)));
    for i in 0..n {
        dists[i] = dist2_zxy(pp, vec3(ent_pos(cops[i]))).to_f32();
    }
    let num = num.clamp(0, 10) as usize;
    for slot in out.iter_mut().take(num) {
        let mut best = x(f32::MAX);
        let mut idx: i32 = -1;
        for (j, d) in dists.iter().enumerate().take(n) {
            if best.fcom(x(*d)) == sa_math::x87::Cmp::Greater {
                best = x(*d);
                idx = j as i32;
            }
        }
        if idx != -1 {
            *slot = cops[idx as usize];
            cops[idx as usize] = GPtr::NULL;
            dists[idx as usize] = f32::MAX;
        }
    }
    out.iter().take(num).any(|c| *c == ped) as u8
}

/// `CWanted::ComputePursuitCopToDisplace(CCopPed* cop, CCopPed** cops)` (0x562B00, cdecl): the pursuit
/// cop farther from the local player than every other and than `cop` (squared distances in x87, the
/// threshold starts at `cop`'s squared distance stored as f32, or 1.0 when that is below 1.0 or there is
/// no `cop`). A dead cop in the list is returned at once.
#[hook(addr = 0x562B00, conv = cdecl, name = "CWanted::ComputePursuitCopToDisplace", expect = "51 8B 44 24 08")]
pub fn compute_pursuit_cop_to_displace(c: GPtr<u8>, cops: GPtr<GPtr<u8>>) -> GPtr<u8> {
    let mut best: f32 = 1.0;
    if !c.is_null() {
        let cp = vec3(ent_pos(c));
        let pp = vec3(ent_pos(find_player_ped(-1)));
        best = dist3_zyx(pp, cp).to_f32();
        if !x(best).fcom(x(1.0)).test_parity(5) {
            best = 1.0;
        }
    }
    let mut res = GPtr::NULL;
    for i in 0..10 {
        let e = cops.wrapping_index(i).read();
        if e.is_null() {
            continue;
        }
        if ped_is_alive(e) == 0 {
            return cops.wrapping_index(i).read();
        }
        let e = cops.wrapping_index(i).read();
        let ep = vec3(ent_pos(e));
        let pp = vec3(ent_pos(find_player_ped(-1)));
        let d = dist3_zyx(pp, ep);
        if !d.fcom(x(best)).test_any(0x41) {
            res = cops.wrapping_index(i).read();
            best = d.to_f32();
        }
    }
    res
}

/// `CWanted::RemoveExcessPursuitCops` (0x562C40): while more cops than allowed, displace one (from
/// `ComputePursuitCopToDisplace(null, cops)`).
#[hook(addr = 0x562C40, conv = thiscall, name = "CWanted::RemoveExcessPursuitCops", expect = "57 8B F9 8A 47 18", this = "CWanted")]
pub fn remove_excess_pursuit_cops(this: GPtr<u8>) {
    if rd::<u8>(this, 0x18) <= rd::<u8>(this, 0x19) {
        return;
    }
    let cops = cop(this, 0);
    loop {
        let d = compute_pursuit_cop_to_displace_call(GPtr::NULL, cops);
        for i in 0..10 {
            if cop(this, i).read() == d {
                cop(this, i).write(GPtr::NULL);
                wr::<u8>(this, 0x18, rd::<u8>(this, 0x18).wrapping_sub(1));
                break;
            }
        }
        if rd::<u8>(this, 0x18) <= rd::<u8>(this, 0x19) {
            return;
        }
    }
}

/// `CWanted::CanCopJoinPursuit(CCopPed*, uint8 maxCops, CCopPed** cops, uint8& count)` (0x562F60, cdecl,
/// AL): false for max 0; while `count >= max`, displace (`ComputePursuitCopToDisplace(cop, cops)`; none →
/// false) with `RemovePursuitCop`; then true.
#[hook(addr = 0x562F60, conv = cdecl, name = "CWanted::CanCopJoinPursuit-func", expect = "53 8A 5C 24 0C")]
pub fn can_cop_join_pursuit_func(c: GPtr<u8>, max: u32, cops: GPtr<GPtr<u8>>, n: GPtr<u8>) -> u8 {
    let max = max as u8;
    if max == 0 {
        return 0;
    }
    while n.read() >= max {
        let d = compute_pursuit_cop_to_displace_call(c, cops);
        if d.is_null() {
            return 0;
        }
        remove_pursuit_cop_func_call(d, cops, n);
    }
    1
}

/// `CWanted::CanCopJoinPursuit(CCopPed*)` (0x562FB0, `ret 4`, AL): false with any of flags 0..2; else the
/// static version on copies of the cop array and the count (max = +0x19).
#[hook(addr = 0x562FB0, conv = thiscall, name = "CWanted::CanCopJoinPursuit-method", expect = "8A 41 1E 83 EC 30", this = "CWanted")]
pub fn can_cop_join_pursuit_method(this: GPtr<u8>, c: GPtr<u8>) -> u8 {
    if rd::<u8>(this, 0x1E) & 7 != 0 {
        return 0;
    }
    let mut copy = [GPtr::<u8>::NULL; 10];
    for (i, s) in copy.iter_mut().enumerate() {
        *s = cop(this, i as u32).read();
    }
    let mut n = [rd::<u8>(this, 0x18)];
    let max = rd::<u8>(this, 0x19);
    can_cop_join_pursuit_func_call(c, u32::from(max), lp(&mut copy).cast(), lp(&mut n).cast())
}

/// `CWanted::SetPursuitCop(CCopPed*)` (0x563060, `ret 4`, AL): `CanCopJoinPursuit` false → returns it
/// (AL 0); when full, displaces cops as `CanCopJoinPursuit` did; then puts the cop into the first empty
/// slot and counts it (AL 1 either way).
#[hook(addr = 0x563060, conv = thiscall, name = "CWanted::SetPursuitCop", expect = "53 8B 5C 24 08", this = "CWanted")]
pub fn set_pursuit_cop(this: GPtr<u8>, c: GPtr<u8>) -> u8 {
    let r = can_cop_join_pursuit_method_call(this, c);
    if r == 0 {
        return r;
    }
    if rd::<u8>(this, 0x18) >= rd::<u8>(this, 0x19) {
        loop {
            let d = compute_pursuit_cop_to_displace_call(c, cop(this, 0));
            for i in 0..10 {
                if cop(this, i).read() == d {
                    cop(this, i).write(GPtr::NULL);
                    wr::<u8>(this, 0x18, rd::<u8>(this, 0x18).wrapping_sub(1));
                    break;
                }
            }
            if rd::<u8>(this, 0x18) < rd::<u8>(this, 0x19) {
                break;
            }
        }
    }
    for i in 0..10 {
        if cop(this, i).read().is_null() {
            wr::<u8>(this, 0x18, rd::<u8>(this, 0x18).wrapping_add(1));
            cop(this, i).write(c);
            return 1;
        }
    }
    1
}

/// `CWanted::UpdateWantedLevel` (0x561C90): clamps the chaos level to the maximum and derives the wanted
/// level (thresholds 0x11F8/0x960/0x4B0/0x226/0xB4/0x32), incrementing stat 0x83 by the levels gained
/// (`fild (new - old)` stored as f32) or stat 0x84 by 1.0 when dropping from 1 to 0; sets cop limits;
/// stamps the change time; flags 0..2 zero the limits.
#[hook(addr = 0x561C90, conv = thiscall, name = "CWanted::UpdateWantedLevel", expect = "51 A1 E8 DE 8C 00", this = "CWanted")]
pub fn update_wanted_level(this: GPtr<u8>) {
    let max = MAXIMUM_CHAOS_LEVEL.get();
    let old = rd::<i32>(this, 0x2C);
    if rd::<i32>(this, 0) > max {
        wr::<i32>(this, 0, max);
    }
    let chaos = rd::<i32>(this, 0);
    const T: [(i32, i32, u8, u8, u16); 6] = [
        (0x11F8, 6, 3, 0xA, 0x1E),
        (0x960, 5, 3, 8, 0x18),
        (0x4B0, 4, 2, 6, 0x12),
        (0x226, 3, 2, 4, 0xC),
        (0xB4, 2, 2, 3, 0),
        (0x32, 1, 1, 1, 0),
    ];
    let mut hit = false;
    for (th, lvl, cars, cops, rb) in T {
        if chaos >= th {
            increment_stat(0x83, xi(lvl.wrapping_sub(old)).to_f32());
            wr::<i32>(this, 0x2C, lvl);
            wr::<u8>(this, 0x1A, cars);
            wr::<u8>(this, 0x19, cops);
            wr::<u16>(this, 0x1C, rb);
            hit = true;
            break;
        }
    }
    if !hit {
        if old == 1 {
            increment_stat(0x84, 1.0);
        }
        wr::<i32>(this, 0x2C, 0);
        wr::<u8>(this, 0x1A, 0);
        wr::<u8>(this, 0x19, 0);
        wr::<u16>(this, 0x1C, 0);
    }
    if old != rd::<i32>(this, 0x2C) {
        wr::<u32>(this, 0xC, sa_core::timer::TIME_MS.get());
    }
    if rd::<u8>(this, 0x1E) & 7 != 0 {
        wr::<u8>(this, 0x1A, 0);
        wr::<u8>(this, 0x19, 0);
        wr::<u16>(this, 0x1C, 0);
    }
}

/// `CWanted::ClearWantedLevelAndGoOnParole` (0x5625A0): stat 0x84 += wanted level (`fild`), parole copies
/// of the local player's chaos and wanted level (re-read through `FindPlayerWanted(-1)`), parole time,
/// chaos and level 0.
#[hook(addr = 0x5625A0, conv = thiscall, name = "CWanted::ClearWantedLevelAndGoOnParole", expect = "56 8B F1 DB 46 2C", this = "CWanted")]
pub fn clear_wanted_level_and_go_on_parole(this: GPtr<u8>) {
    increment_stat(0x84, xi(rd::<i32>(this, 0x2C)).to_f32());
    wr::<i32>(this, 4, rd::<i32>(find_player_wanted(-1), 0));
    wr::<i32>(this, 0x30, rd::<i32>(find_player_wanted(-1), 0x2C));
    wr::<u32>(this, 0x10, sa_core::timer::TIME_MS.get());
    wr::<i32>(this, 0, 0);
    wr::<i32>(this, 0x2C, 0);
}

/// `CWanted::ReportCrimeNow(eCrimeType, const CVector&, bool policeDontCare)` (0x562120, `ret 0xC`): with
/// the never-wanted cheat off: multiplier (x0.3 in area 1, below-or-equal 0 → 0, gang war fighting → 0,
/// x0.333 when the police don't care, gang war again → 0) times the per-crime factor (jump table
/// 0x5622A0, types 2..0x16), added to the chaos level (`fiadd`, `_ftol`); chaos = max(chaos, parole
/// chaos); `UpdateWantedLevel`; a raised wanted level plays police-scanner event 0xA4.
#[hook(addr = 0x562120, conv = thiscall, name = "CWanted::ReportCrimeNow", expect = "51 A0 71 91 96 00", this = "CWanted")]
pub fn report_crime_now(this: GPtr<u8>, ty: i32, coors: GPtr<f32>, dont_care: u8) {
    if g::<u8>(NEVER_WANTED).read() != 0 {
        return;
    }
    let old = rd::<i32>(this, 0x2C);
    let mut m = x(rd::<f32>(this, 0x14));
    if g::<u16>(CURR_AREA_WORD).read() == 1 {
        m = m.fmul_m(0.3);
    }
    // fld 0.0; fcomp st(1); test ah, 0x41; jne: 0.0 > m (ordered) → 0, otherwise m
    let mut local: f32 = if !x(0.0).fcom(m).test_any(0x41) { 0.0 } else { m.to_f32() };
    if gang_war_fighting_going_on() != 0 {
        local = 0.0;
    }
    if dont_care != 0 {
        local = x(local).fmul_m(f32::from_bits(0x3EAA_7EFA)).to_f32();
    }
    if gang_war_fighting_going_on() != 0 {
        local = 0.0;
    }
    let i = ty.wrapping_sub(2) as u32;
    if i <= 0x14 {
        let v = match crime_factor(i) {
            Some(k) => x(local).fmul_m(f32::from_bits(k)),
            None => x(local).fdouble(),
        };
        let r = v.fadd(xi(rd::<i32>(this, 0)));
        wr::<i32>(this, 0, ftol32(r));
    }
    let c = rd::<i32>(this, 0);
    let p = rd::<i32>(this, 4);
    wr::<i32>(this, 0, if c > p { c } else { p });
    update_wanted_level_call(this);
    if rd::<i32>(this, 0x2C) > old {
        police_scanner_add_audio_event(this.byte_add(0x21C), 0xA4, ty, coors);
    }
}

/// The f32 constant `ReportCrimeNow` multiplies by for crime type `i + 2` (jump table 0x5622A0); `None` =
/// `fadd st0, st0` (types 0x15, 0x16).
fn crime_factor(i: u32) -> Option<u32> {
    // targets: 0x5621B7 [0x858C80], 0x5621C6 [0x858CB4], 0x5621D5 [0x85F070], 0x5621E1 [0x858628],
    // 0x5621ED [0x858CA4], 0x5621F9 [0x858B48], 0x562205 [0x85862C], 0x562211 [0x859008],
    // 0x56221D [0x85A700], 0x562229 [0x858CE0], 0x562235 [0x858A14], 0x562241 [0x858BA4],
    // 0x56224D [0x858FE8], 0x562259 fadd st0,st0
    const T: [u32; 21] = [
        0x5621B7, 0x5621C6, 0x5621ED, 0x562235, 0x5621F9, 0x562205, 0x5621B7, 0x5621B7, 0x562211, 0x562235, 0x56221D,
        0x562241, 0x562235, 0x562241, 0x56221D, 0x56224D, 0x5621D5, 0x5621E1, 0x562229, 0x562259, 0x562259,
    ];
    // .rdata bits: 5, 45, 35, 100, 30, 15, 10, 18, 400, 70, 80, 20, 25
    let a = match T[i as usize] {
        0x5621B7 => 0x40A0_0000,
        0x5621C6 => 0x4234_0000,
        0x5621D5 => 0x420C_0000,
        0x5621E1 => 0x42C8_0000,
        0x5621ED => 0x41F0_0000,
        0x5621F9 => 0x4170_0000,
        0x562205 => 0x4120_0000,
        0x562211 => 0x4190_0000,
        0x56221D => 0x43C8_0000,
        0x562229 => 0x428C_0000,
        0x562235 => 0x42A0_0000,
        0x562241 => 0x41A0_0000,
        0x56224D => 0x41C8_0000,
        _ => return None,
    };
    Some(a)
}

orig! {
    /// `CWanted::SetMaximumWantedLevel` (0x561E70)
    pub fn set_maximum_wanted_level_call(level: i32) = cdecl 0x561E70 => set_maximum_wanted_level;
    /// `CWanted::UpdateCrimesQ` (0x562760)
    pub fn update_crimes_q_call(this: GPtr<u8>) = thiscall 0x562760 => update_crimes_q;
    /// `CWanted::WorkOutPolicePresence(CVector, float)` (0x5625F0)
    pub fn work_out_police_presence_call(px: f32, py: f32, pz: f32, radius: f32) -> i32 = cdecl 0x5625F0 => work_out_police_presence;
    /// `CWanted::RegisterCrime` (0x562410)
    pub fn register_crime_call(this: GPtr<u8>, ty: i32, coors: GPtr<f32>, ped: GPtr<u8>, dont_care: u32) = thiscall 0x562410 => register_crime_u32;
    /// `CWanted::RegisterCrime_Immediately` (0x562430)
    pub fn register_crime_immediately_call(this: GPtr<u8>, ty: i32, coors: GPtr<f32>, ped: GPtr<u8>, dont_care: u32) = thiscall 0x562430 => register_crime_immediately_u32;
    /// `CStats::SetNewRecordStat(uint16, float)` (0x55C410)
    pub fn set_new_record_stat(stat: u32, v: f32) = cdecl 0x55C410 => unported;
    /// `CStats::SetStatValue(uint16, float)` (0x55A070)
    pub fn set_stat_value(stat: u32, v: f32) = cdecl 0x55A070 => unported;
    /// `CVehicle::IsLawEnforcementVehicle` (0x6D2370)
    pub fn is_law_enforcement_vehicle(v: GPtr<u8>) -> u8 = thiscall 0x6D2370 => unported;
    /// `FindPlayerCoors(CVector* out, int32)` (0x56E010)
    pub fn find_player_coors(out: GPtr<f32>, id: i32) -> GPtr<f32> = cdecl 0x56E010 => sa_world::world::cworld::find_player_coors_port;
    /// `CGameLogic::SetPlayerWantedLevelForForbiddenTerritories(bool)` (0x441770)
    pub fn set_player_wanted_level_for_forbidden_territories(b: u32) = cdecl 0x441770 => unported;
    /// CRT printf (0x821982, `FID_conflict:_wprintf`): the debug messages of `CWanted::Update`
    pub fn debug_printf(fmt: u32) -> i32 = cdecl 0x821982 => unported;
    /// `CPedType::PoliceDontCareAboutCrimesAgainstPedType(ePedType)` (0x608A40)
    pub fn police_dont_care_about_crimes_against_ped_type(ty: i32) -> u8 = cdecl 0x608A40 => crate::ped_type::police_dont_care_about_crimes_against_ped_type;
    /// `IsPedPointerValid(CPed*)` (0x5E8A30)
    pub fn is_ped_pointer_valid(ped: GPtr<u8>) -> u8 = cdecl 0x5E8A30 => unported;
    /// `CPlayerPed::GetWantedLevel` (0x41BE60)
    pub fn player_get_wanted_level(ped: GPtr<u8>) -> i32 = thiscall 0x41BE60 => crate::player_ped::get_wanted_level;
    /// `CText::Get(const char*)` (0x6A0050) on `TheText` (0xC1B340)
    pub fn text_get(text: u32, key: u32) -> u32 = thiscall 0x6A0050 => unported;
    /// `CMessages::AddBigMessage(const char* text, uint32 time, eMessageStyle)` (0x69F2B0)
    pub fn add_big_message(text: u32, time: u32, style: u32) = cdecl 0x69F2B0 => unported;
    /// `CCrime::FindImmediateDetectionRange` (0x531FC0)
    pub fn find_immediate_detection_range_call(ty: i32) -> f64 = cdecl 0x531FC0 => find_immediate_detection_range_f64;
    /// `CLocalisation::GermanGame` (0x56D200)
    pub fn german_game() -> u8 = cdecl 0x56D200 => unported;
    /// `CPlayerPed::SetWantedLevelNoDrop(int32)` (0x609F30)
    pub fn player_set_wanted_level_no_drop(ped: GPtr<u8>, level: i32) = thiscall 0x609F30 => crate::player_ped::set_wanted_level_no_drop;
}

pub fn register_crime_u32(this: GPtr<u8>, ty: i32, coors: GPtr<f32>, ped: GPtr<u8>, dont_care: u32) {
    register_crime(this, ty, coors, ped, dont_care as u8)
}

pub fn register_crime_immediately_u32(this: GPtr<u8>, ty: i32, coors: GPtr<f32>, ped: GPtr<u8>, dont_care: u32) {
    register_crime_immediately(this, ty, coors, ped, dont_care as u8)
}

pub fn find_immediate_detection_range_f64(ty: i32) -> f64 {
    find_immediate_detection_range(ty).to_f64()
}

/// `CWanted::SetWantedLevel(int32)` (0x562470, `ret 4`): with the never-wanted cheat off: level clamped to
/// `MaximumWantedLevel` (signed); every queued crime's type cleared; chaos set per level (0, 0x46, 0xC8,
/// 0x23A, 0x4C4, 0x974, 0x120C; other values keep it); `UpdateWantedLevel`.
#[hook(addr = 0x562470, conv = thiscall, name = "CWanted::SetWantedLevel", expect = "A0 71 91 96 00", this = "CWanted")]
pub fn set_wanted_level(this: GPtr<u8>, level: i32) {
    if g::<u8>(NEVER_WANTED).read() != 0 {
        return;
    }
    let max = MAXIMUM_WANTED_LEVEL.get();
    let l = if level > max { max } else { level };
    for i in 0..16 {
        wr::<u32>(crime(this, i), 0, 0);
    }
    const CHAOS: [i32; 7] = [0, 0x46, 0xC8, 0x23A, 0x4C4, 0x974, 0x120C];
    if (l as u32) <= 6 {
        wr::<i32>(this, 0, CHAOS[l as usize]);
    }
    update_wanted_level_call(this);
}

/// `CWanted::CheatWantedLevel(int32)` (0x562540, `ret 4`): raises the maximum when needed, then
/// `SetWantedLevel` and `UpdateWantedLevel`.
#[hook(addr = 0x562540, conv = thiscall, name = "CWanted::CheatWantedLevel", expect = "A1 E4 DE 8C 00", this = "CWanted")]
pub fn cheat_wanted_level(this: GPtr<u8>, level: i32) {
    if level > MAXIMUM_WANTED_LEVEL.get() {
        set_maximum_wanted_level_call(level);
    }
    set_wanted_level_call(this, level);
    update_wanted_level_call(this);
}

/// `CWanted::SetWantedLevelNoDrop(int32)` (0x562570, `ret 4`): first back to the parole level when below
/// it, then up to `level` when above.
#[hook(addr = 0x562570, conv = thiscall, name = "CWanted::SetWantedLevelNoDrop", expect = "56 8B F1 8B 46 30", this = "CWanted")]
pub fn set_wanted_level_no_drop(this: GPtr<u8>, level: i32) {
    let parole = rd::<i32>(this, 0x30);
    if rd::<i32>(this, 0x2C) < parole {
        set_wanted_level_call(this, parole);
    }
    if level > rd::<i32>(this, 0x2C) {
        set_wanted_level_call(this, level);
    }
}

/// `CWanted::UpdateCrimesQ` (0x562760): for each queued crime with a type: after 500 ms (unsigned) and not
/// yet reported → `ReportCrimeNow(type, &coors, dontCare)` and mark reported; after 10 s the type is cleared.
#[hook(addr = 0x562760, conv = thiscall, name = "CWanted::UpdateCrimesQ", expect = "55 56 57 8B F9", this = "CWanted")]
pub fn update_crimes_q(this: GPtr<u8>) {
    for i in 0..16 {
        let c = crime(this, i);
        let ty = rd::<i32>(c, 0);
        if ty == 0 {
            continue;
        }
        if sa_core::timer::TIME_MS.get() > rd::<u32>(c, 8).wrapping_add(0x1F4) && rd::<u8>(c, 0x18) == 0 {
            report_crime_now_call(this, ty, c.byte_add(0xC).cast(), rd::<u8>(c, 0x19));
            wr::<u8>(c, 0x18, 1);
        }
        if sa_core::timer::TIME_MS.get() > rd::<u32>(c, 8).wrapping_add(0x2710) {
            wr::<u32>(c, 0, 0);
        }
    }
}

/// `fild dword` of a counter the original treats as unsigned (`+ 2^32` when negative), stored as f32.
fn fild_unsigned(v: i32) -> f32 {
    let f = xi(v);
    if v < 0 { f.fadd_m(4294967296.0).to_f32() } else { f.to_f32() }
}

/// `CWanted::Update` (0x562C90): the 5-star timer (+0x20 seconds, +0x24 last tick, +0x28 running) and its
/// stats 0x33/0x34; parole expiry after 20 s; once per second (unsigned): the chaos decay (by 2 when the
/// gang-war state 0xC81314 is 0 or 4, else 1; clamped at 0) when no police is within 18 m, skipped for
/// level > 1 unless (state 0/4 and [0xB72914] == 0), and in a law-enforcement vehicle or a vehicle of
/// type 3/4; then `UpdateCrimesQ`, the pursuit list recount (debug printf on mismatch) and compaction;
/// finally `UpdateWantedLevel` when the "everybody back off" flags changed.
#[hook(addr = 0x562C90, conv = thiscall, name = "CWanted::Update", expect = "83 EC 14 53 55", this = "CWanted")]
pub fn update(this: GPtr<u8>) {
    let now = || sa_core::timer::TIME_MS.get();
    if rd::<i32>(this, 0x2C) >= 5 {
        if rd::<u8>(this, 0x28) == 0 {
            wr::<u32>(this, 0x20, 0);
            wr::<u32>(this, 0x24, now());
            wr::<u8>(this, 0x28, 1);
        }
        if rd::<u8>(this, 0x28) != 0 && now().wrapping_sub(rd::<u32>(this, 0x24)) > 0x3E8 {
            wr::<i32>(this, 0x20, rd::<i32>(this, 0x20).wrapping_add(1));
            wr::<u32>(this, 0x24, now());
        }
    } else if rd::<u8>(this, 0x28) != 0 {
        set_new_record_stat(0x33, fild_unsigned(rd::<i32>(this, 0x20)));
        set_stat_value(0x34, fild_unsigned(rd::<i32>(this, 0x20)));
        wr::<u8>(this, 0x28, 0);
    }
    if now() > rd::<u32>(this, 0x10).wrapping_add(0x4E20) {
        wr::<u32>(this, 4, 0);
        wr::<u32>(this, 0x30, 0);
    }
    if now().wrapping_sub(rd::<u32>(this, 8)) <= 0x3E8 {
        update_flags(this);
        return;
    }
    let gw = g::<u16>(0xC81314).read();
    let bl = gw == 0 || gw == 4;
    // short-circuit order = the original's call order (FindPlayerVehicle up to four times)
    let no_decay = (rd::<i32>(this, 0x2C) > 1 && (!bl || g::<u32>(0xB72914).read() != 0))
        || (!find_player_vehicle(-1, 0).is_null()
            && (is_law_enforcement_vehicle(find_player_vehicle(-1, 0)) != 0
                || rd::<i32>(find_player_vehicle(-1, 0), 0x594) == 4
                || rd::<i32>(find_player_vehicle(-1, 0), 0x594) == 3));
    if no_decay {
        wr::<u32>(this, 8, now());
    } else {
        let mut v = [0f32; 3];
        let p = vec3(find_player_coors(lp(&mut v).cast(), -1));
        if work_out_police_presence_call(p[0], p[1], p[2], 18.0) == 0 {
            wr::<u32>(this, 8, now());
            let c = rd::<i32>(this, 0).wrapping_sub(if bl { 2 } else { 1 });
            wr::<i32>(this, 0, if c < 0 { 0 } else { c });
            update_wanted_level_call(this);
            set_player_wanted_level_for_forbidden_territories(1);
        }
    }
    update_crimes_q_call(this);
    let mut count: u8 = 0;
    let mut seen_null = false;
    let mut messed = false;
    for i in 0..10 {
        if cop(this, i).read().is_null() {
            seen_null = true;
        } else {
            count = count.wrapping_add(1);
            if seen_null {
                messed = true;
            }
        }
    }
    if count != rd::<u8>(this, 0x18) {
        debug_printf(0x864E64);
        wr::<u8>(this, 0x18, count);
    }
    if messed {
        debug_printf(0x864E34);
        let mut none_moved = true;
        let mut d = 0u32;
        while d < 10 {
            if cop(this, d).read().is_null() {
                let mut moved = false;
                for a in d..10 {
                    let e = cop(this, a).read();
                    if !e.is_null() {
                        cop(this, d).write(e);
                        cop(this, a).write(GPtr::NULL);
                        none_moved = false;
                        moved = true;
                        break;
                    }
                }
                if !moved && none_moved {
                    break;
                }
            }
            d += 1;
        }
    }
    update_flags(this);
}

fn update_flags(this: GPtr<u8>) {
    let f = (rd::<u8>(this, 0x1E) & 7 != 0) as u8;
    if rd::<u8>(this, 0x298) == f {
        return;
    }
    update_wanted_level_call(this);
    wr::<u8>(this, 0x298, (rd::<u8>(this, 0x1E) & 7 != 0) as u8);
}

/// `CCrime::ReportCrime(eCrimeType, CEntity* victim, CPed* offender)` (0x532010, cdecl): only for player
/// offenders (ped type 0/1). Type 2 against a ped flagged +0x46C bit 23 by a player without a wanted
/// level: "GOODBOY" big message and $50 (unless the victim's state is 0x36/0x37), dereferencing the victim
/// without a null check (as the original). Otherwise, with the player's wanted multiplier not below 0:
/// police nearby (immediate detection range) or German-game types 4/5/0xD/0xE → `RegisterCrime_Immediately`
/// and wanted level at least 1, else `RegisterCrime`. Types 3 / 5 / 0x13 raise the player's wanted level
/// to 1 / 2 / 2.
#[hook(addr = 0x532010, conv = cdecl, name = "CCrime::ReportCrime", expect = "83 EC 10 53 8B 5C 24 20")]
pub fn report_crime(ty: i32, victim: GPtr<u8>, offender: GPtr<u8>) {
    if offender.is_null() {
        return;
    }
    let pt = rd::<i32>(offender, 0x598);
    if pt != 0 && pt != 1 {
        return;
    }
    let mut dont_care: u8 = 0;
    if !victim.is_null() && rd::<u8>(victim, 0x36) & 7 == 3 {
        dont_care = police_dont_care_about_crimes_against_ped_type(rd::<i32>(victim, 0x598));
    }
    if ty == 2 {
        if rd::<u8>(victim, 0x36) & 7 == 3
            && is_ped_pointer_valid(victim) != 0
            && player_get_wanted_level(offender) == 0
            && rd::<u32>(victim, 0x46C) & 0x80_0000 != 0
        {
            let st = rd::<i32>(victim, 0x530);
            if st == 0x36 || st == 0x37 {
                return;
            }
            add_big_message(text_get(0xC1B340, 0x86386C), 0x1388, 0);
            let focus = usize::from(g::<u8>(PLAYER_IN_FOCUS).read());
            let money = g::<i32>(0xB7CE50 + focus * PLAYER_INFO_SIZE);
            money.write(money.read().wrapping_add(0x32));
            return;
        }
    } else if ty == 0 {
        return;
    }
    let mut pos = vec3(ent_pos(offender));
    let wanted_of = || rd::<GPtr<u8>>(rd::<GPtr<u8>>(find_player_ped(-1), 0x480), 0);
    let m = rd::<f32>(wanted_of(), 0x14);
    if !x(m).fcom(x(0.0)).test_any(1) {
        let range = X87::from_f64_exact(find_immediate_detection_range_call(ty)).to_f32();
        let p = vec3(ent_pos(offender));
        let presence = work_out_police_presence_call(p[0], p[1], p[2], range);
        let immediate = presence != 0 || (matches!(ty, 4 | 5 | 0xD | 0xE) && german_game() != 0);
        if !immediate {
            register_crime_call(wanted_of(), ty, lp(&mut pos).cast(), victim, u32::from(dont_care));
        } else {
            register_crime_immediately_call(wanted_of(), ty, lp(&mut pos).cast(), victim, u32::from(dont_care));
            set_wanted_level_no_drop_call(wanted_of(), 1);
        }
    }
    if ty == 3 {
        player_set_wanted_level_no_drop(find_player_ped(-1), 1);
    } else if ty == 5 || ty == 0x13 {
        player_set_wanted_level_no_drop(find_player_ped(-1), 2);
    }
}

/// `CWanted::ClearQdCrimes()` (0x561FE0, thiscall; batch `mvpl-ped-1`): `EAX = this + 0x34`, `ECX = 16` (set in the
/// in-text fragment 0x404CD2), then the loop `FUN_00561ff0` zeroes the type dword of each of the 16 queued crimes
/// (stride 0x1C). The original returns with EAX past the last crime and ECX = 0 (no caller reads them).
#[hook(addr = 0x561fe0, conv = thiscall, name = "CWanted::ClearQdCrimes", expect = "8D 41 34 E9 EA 2C EA FF",
       unverified = "mvpl-ped-1", route = "L2", hazards = "entry-protection-read", recs = "grove-walk",
       note = "the entry jumps through the fragment 0x404CD2 (mov ecx, 0x10) into the loop at 0x561FF0; the protection reads the entry jmp at 0x401409 (keycells)")]
pub fn clear_qd_crimes(this: GPtr<u8>) {
    for i in 0..16 {
        wr::<u32>(this, CRIMES + i * CRIME_SIZE, 0);
    }
}

// --- hook list (generated) ---

pub const HOOKS: &[HookReg] = &[
    HOOK_FIND_IMMEDIATE_DETECTION_RANGE,
    HOOK_INITIALISE_STATIC_VARIABLES,
    HOOK_SET_MAXIMUM_WANTED_LEVEL,
    HOOK_ARE_MIAMI_VICE_REQUIRED,
    HOOK_ARE_SWAT_REQUIRED,
    HOOK_ARE_FBI_REQUIRED,
    HOOK_ARE_ARMY_REQUIRED,
    HOOK_NUM_OF_HELIS_REQUIRED,
    HOOK_ADD_CRIME_TO_Q,
    HOOK_REMOVE_PURSUIT_COP_FUNC,
    HOOK_IS_IN_PURSUIT,
    HOOK_INITIALISE,
    HOOK_REMOVE_PURSUIT_COP_METHOD,
    HOOK_UPDATE_EACH_FRAME,
    HOOK_RESET,
    HOOK_REGISTER_CRIME,
    HOOK_REGISTER_CRIME_IMMEDIATELY,
    HOOK_WORK_OUT_POLICE_PRESENCE,
    HOOK_IS_CLOSEST_COP,
    HOOK_COMPUTE_PURSUIT_COP_TO_DISPLACE,
    HOOK_REMOVE_EXCESS_PURSUIT_COPS,
    HOOK_CAN_COP_JOIN_PURSUIT_FUNC,
    HOOK_CAN_COP_JOIN_PURSUIT_METHOD,
    HOOK_SET_PURSUIT_COP,
    HOOK_UPDATE_WANTED_LEVEL,
    HOOK_CLEAR_WANTED_LEVEL_AND_GO_ON_PAROLE,
    HOOK_REPORT_CRIME_NOW,
    HOOK_SET_WANTED_LEVEL,
    HOOK_CHEAT_WANTED_LEVEL,
    HOOK_SET_WANTED_LEVEL_NO_DROP,
    HOOK_UPDATE_CRIMES_Q,
    HOOK_UPDATE,
    HOOK_REPORT_CRIME,
    HOOK_CLEAR_QD_CRIMES,
];
