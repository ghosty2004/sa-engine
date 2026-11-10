//! `CRestart` (PORT-WORLD): restart points after death/arrest. Ported from the 1.0 US bytes
//! (gta-reversed `Restart.cpp` as reading aid).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{x, xi};
use super::zones::get_level_from_position_call;

pub const MAX_RESTART_POINTS: usize = 10;

sa_core::globals! {
    list GLOBALS;
    /// `CRestart::bOverrideRespawnBasePointForMission`
    pub static OVERRIDE_RESPAWN_FOR_MISSION @ 0xA43248: u8 = 0;
    /// `CRestart::ExtraPoliceStationRestartHeading`
    pub static EXTRA_POLICE_HEADING @ 0xA4324C: f32 = 0.0;
    /// `CRestart::ExtraPoliceStationRestartRadius`
    pub static EXTRA_POLICE_RADIUS @ 0xA43250: f32 = 0.0;
    /// `CRestart::ExtraHospitalRestartHeading`
    pub static EXTRA_HOSPITAL_HEADING @ 0xA43254: f32 = 0.0;
    /// `CRestart::ExtraHospitalRestartRadius`
    pub static EXTRA_HOSPITAL_RADIUS @ 0xA43258: f32 = 0.0;
    /// `CRestart::bFadeInAfterNextArrest`
    pub static FADE_IN_AFTER_NEXT_ARREST @ 0xA4325C: u8 = 0;
    /// `CRestart::bFadeInAfterNextDeath`
    pub static FADE_IN_AFTER_NEXT_DEATH @ 0xA4325D: u8 = 0;
    /// `CRestart::OverrideHeading`
    pub static OVERRIDE_HEADING @ 0xA43260: f32 = 0.0;
    /// `CRestart::bOverrideRestart`
    pub static OVERRIDE_RESTART @ 0xA43264: u8 = 0;
    /// `CRestart::NumberOfPoliceRestarts`
    pub static NUM_POLICE @ 0xA43268: u16 = 0;
    /// `CRestart::NumberOfHospitalRestarts`
    pub static NUM_HOSPITAL @ 0xA4326C: u16 = 0;
    /// `CRestart::PoliceRestartWhenToUse`
    pub static POLICE_WHEN @ 0xA43270: [i32; MAX_RESTART_POINTS] = [0; MAX_RESTART_POINTS];
    /// `CRestart::PoliceRestartHeadings`
    pub static POLICE_HEADINGS @ 0xA43298: [f32; MAX_RESTART_POINTS] = [0.0; MAX_RESTART_POINTS];
    /// `CRestart::HospitalRestartWhenToUse`
    pub static HOSPITAL_WHEN @ 0xA432C0: [i32; MAX_RESTART_POINTS] = [0; MAX_RESTART_POINTS];
    /// `CRestart::HospitalRestartHeadings`
    pub static HOSPITAL_HEADINGS @ 0xA432E8: [f32; MAX_RESTART_POINTS] = [0.0; MAX_RESTART_POINTS];
    /// `CRestart::HospitalRestartPoints` (`CVector[10]`)
    pub static HOSPITAL_POINTS @ 0xA43318: [u32; 3 * MAX_RESTART_POINTS] = [0; 3 * MAX_RESTART_POINTS];
    /// `CRestart::PoliceRestartPoints` (`CVector[10]`)
    pub static POLICE_POINTS @ 0xA43390: [u32; 3 * MAX_RESTART_POINTS] = [0; 3 * MAX_RESTART_POINTS];
    /// `CRestart::OverridePosition`
    pub static OVERRIDE_POSITION @ 0xA43408: [u32; 3] = [0; 3];
    /// `CRestart::ExtraHospitalRestartCoors`
    pub static EXTRA_HOSPITAL_COORS @ 0xA43414: [u32; 3] = [0; 3];
    /// `CRestart::ExtraPoliceStationRestartCoors`
    pub static EXTRA_POLICE_COORS @ 0xA43420: [u32; 3] = [0; 3];
    /// `CRestart::OverrideRespawnBasePointForMission`
    pub static OVERRIDE_RESPAWN_POINT @ 0xA4342C: [u32; 3] = [0; 3];
}

orig! {
    /// `CStats::GetStatValue(uint16)` (0x558E40), result in ST0 (read as f64: exact for `fld dword`
    /// and `fild dword`)
    pub fn get_stat_value_st0(stat: u16) -> f64 = cdecl 0x558E40 => stat_value_port;
}

pub fn stat_value_port(stat: u16) -> f64 {
    super::stats::get_stat_value(stat).to_f64()
}

fn vec_at(base: GPtr<u32>, i: usize) -> GPtr<u32> {
    base.wrapping_index((3 * i) as i32)
}

/// `CRestart::Initialise` (stub 0x460630 → 0x15658E0): headings and points of both tables 0 (the
/// "when to use" tables are kept), counts 0, override position/heading 0, override flags 0, both fade
/// flags 1, both extra radii 0.
#[hook(addr = 0x460630, conv = cdecl, name = "CRestart::Initialise", expect = "E9 AB 52 10 01")]
pub fn initialise() {
    HOSPITAL_HEADINGS.set([0.0; MAX_RESTART_POINTS]);
    HOSPITAL_POINTS.set([0; 3 * MAX_RESTART_POINTS]);
    POLICE_HEADINGS.set([0.0; MAX_RESTART_POINTS]);
    POLICE_POINTS.set([0; 3 * MAX_RESTART_POINTS]);
    OVERRIDE_POSITION.set([0; 3]);
    NUM_HOSPITAL.set(0);
    NUM_POLICE.set(0);
    OVERRIDE_RESTART.set(0);
    OVERRIDE_HEADING.set(0.0);
    FADE_IN_AFTER_NEXT_DEATH.set(1);
    FADE_IN_AFTER_NEXT_ARREST.set(1);
    EXTRA_HOSPITAL_RADIUS.set(0.0);
    EXTRA_POLICE_RADIUS.set(0.0);
    OVERRIDE_RESPAWN_FOR_MISSION.set(0);
}

fn add_point(n: &sa_core::mem::Global<u16>, pts: GPtr<u32>, heads: GPtr<f32>, when: GPtr<i32>, p: GPtr<u32>, h: f32, town: i32) {
    let c = n.get();
    let i = usize::from(c);
    let d = vec_at(pts, i);
    for k in 0..3 {
        d.wrapping_index(k).write(p.wrapping_index(k).read());
    }
    heads.wrapping_index(i as i32).write(h);
    when.wrapping_index(i as i32).write(town);
    n.set(c.wrapping_add(1));
}

/// `CRestart::AddHospitalRestartPoint(const CVector&, float, int32)` (0x460730); no bound check.
#[hook(addr = 0x460730, conv = cdecl, name = "CRestart::AddHospitalRestartPoint", expect = "66 8B 0D 6C 32 A4 00")]
pub fn add_hospital_restart_point(p: GPtr<u32>, heading: f32, town: i32) {
    add_point(&NUM_HOSPITAL, HOSPITAL_POINTS.ptr().cast(), HOSPITAL_HEADINGS.ptr().cast(), HOSPITAL_WHEN.ptr().cast(), p, heading, town);
}

/// `CRestart::AddPoliceRestartPoint(const CVector&, float, int32)` (0x460780); no bound check.
#[hook(addr = 0x460780, conv = cdecl, name = "CRestart::AddPoliceRestartPoint", expect = "66 8B 0D 68 32 A4 00")]
pub fn add_police_restart_point(p: GPtr<u32>, heading: f32, town: i32) {
    add_point(&NUM_POLICE, POLICE_POINTS.ptr().cast(), POLICE_HEADINGS.ptr().cast(), POLICE_WHEN.ptr().cast(), p, heading, town);
}

/// `CRestart::OverrideNextRestart(const CVector&, float)` (stub 0x4607D0 → 0x156B8F0).
#[hook(addr = 0x4607D0, conv = cdecl, name = "CRestart::OverrideNextRestart", expect = "E9 1B B1 10 01")]
pub fn override_next_restart(p: GPtr<u32>, heading: u32) {
    OVERRIDE_POSITION.set([p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read()]);
    OVERRIDE_HEADING.set(f32::from_bits(heading));
    OVERRIDE_RESTART.set(1);
}

/// `CRestart::CancelOverrideRestart` (0x460800).
#[hook(addr = 0x460800, conv = cdecl, name = "CRestart::CancelOverrideRestart", expect = "C6 05 64 32 A4 00 00")]
pub fn cancel_override_restart() {
    OVERRIDE_RESTART.set(0);
}

/// `CRestart::SetRespawnPointForDurationOfMission(CVector)` (stub 0x460810 → 0x156F610, by value).
#[hook(addr = 0x460810, conv = cdecl, name = "CRestart::SetRespawnPointForDurationOfMission", expect = "E9 FB ED 10 01")]
pub fn set_respawn_point_for_duration_of_mission(px: u32, py: u32, pz: u32) {
    OVERRIDE_RESPAWN_FOR_MISSION.set(1);
    OVERRIDE_RESPAWN_POINT.set([px, py, pz]);
}

/// `CRestart::ClearRespawnPointForDurationOfMission` (stub 0x460840 → 0x15626A0).
#[hook(addr = 0x460840, conv = cdecl, name = "CRestart::ClearRespawnPointForDurationOfMission", expect = "E9 5B 1E 10 01")]
pub fn clear_respawn_point_for_duration_of_mission() {
    OVERRIDE_RESPAWN_FOR_MISSION.set(0);
}

struct Table {
    count: &'static sa_core::mem::Global<u16>,
    when: GPtr<i32>,
    heads: GPtr<f32>,
    pts: GPtr<u32>,
    radius: &'static sa_core::mem::Global<f32>,
    coors: GPtr<u32>,
    extra_heading: &'static sa_core::mem::Global<f32>,
    /// initial best distance (bits)
    best: u32,
}

/// The shared body of `FindClosestHospitalRestartPoint` / `FindClosestPoliceRestartPoint`
/// (`(CVector pos by value, CVector* outPos, float* outHeading)`), see the module tables.
fn find_closest(t: &Table, pos_in: [u32; 3], out: GPtr<u32>, out_heading: GPtr<f32>) {
    if OVERRIDE_RESTART.get() != 0 {
        let o = OVERRIDE_POSITION.get();
        out.write(o[0]);
        out.wrapping_index(1).write(o[1]);
        out.wrapping_index(2).write(o[2]);
        out_heading.write(x(OVERRIDE_HEADING.get()).to_f32());
        OVERRIDE_RESTART.set(0);
        return;
    }
    // the by-value argument is overwritten in place
    let mut pos = pos_in;
    if OVERRIDE_RESPAWN_FOR_MISSION.get() != 0 {
        pos = OVERRIDE_RESPAWN_POINT.get();
        OVERRIDE_RESPAWN_FOR_MISSION.set(0);
    }
    let pf = |k: usize| f32::from_bits(pos[k]);
    let r = t.radius.get();
    // fld r; fcomp 0.0; test ah,0x41; jne: only r > 0
    if !x(r).fcom(X87::zero()).test_any(0x41) {
        let c = |k: i32| f32::from_bits(t.coors.wrapping_index(k).read());
        let a = x(pf(0)).fsub_m(c(0));
        let b = x(pf(1)).fsub_m(c(1));
        let d = b.fmul(b).fadd(a.fmul(a)).fsqrt();
        // fcomp r; test ah,5; jp: taken unless Less
        if !d.fcom(x(r)).test_parity(0x05) {
            out.write(t.coors.read());
            out.wrapping_index(1).write(t.coors.wrapping_index(1).read());
            out.wrapping_index(2).write(t.coors.wrapping_index(2).read());
            out_heading.write(x(t.extra_heading.get()).to_f32());
            return;
        }
    }
    let mut posv = [pf(0), pf(1), pf(2)];
    let level = get_level_from_position_call(GPtr::from_raw(posv.as_mut_ptr()));
    let mut best_i: i16 = -1;
    let mut best = f32::from_bits(t.best);
    let mut i: u16 = 0;
    while i < t.count.get() {
        let k = usize::from(i);
        let req = xi(t.when.wrapping_index(k as i32).read()).to_f32();
        let stat = X87::from_f64_exact(get_stat_value_st0(0xB5));
        // fcomp req; test ah,5; jnp skip: skip on Less
        if !stat.fcom(x(req)).test_parity(0x05) {
            i = i.wrapping_add(1);
            continue;
        }
        let p = vec_at(t.pts, k);
        let q = |j: i32| f32::from_bits(p.wrapping_index(j).read());
        let a = x(posv[0]).fsub_m(q(0));
        let b = x(posv[1]).fsub_m(q(1));
        let c = x(posv[2]).fsub_m(q(2));
        let mut d = c.fmul(c).fadd(b.fmul(b)).fadd(a.fmul(a)).fsqrt().to_f32();
        if level != 0 && get_level_from_position_call(p.cast()) != level {
            d = x(d).fmul_m(6.0).to_f32();
        }
        // fcomp best; test ah,5; jp: taken unless Less
        if !x(d).fcom(x(best)).test_parity(0x05) {
            best = d;
            best_i = i as i16;
        }
        i = i.wrapping_add(1);
    }
    if best_i >= 0 {
        let k = best_i as usize;
        let p = vec_at(t.pts, k);
        out.write(p.read());
        out.wrapping_index(1).write(p.wrapping_index(1).read());
        out.wrapping_index(2).write(p.wrapping_index(2).read());
        out_heading.cast::<u32>().write(t.heads.cast::<u32>().wrapping_index(k as i32).read());
    }
}

/// `CRestart::FindClosestHospitalRestartPoint(CVector, CVector*, float*)` (0x460850).
#[hook(addr = 0x460850, conv = cdecl, name = "CRestart::FindClosestHospitalRestartPoint", expect = "A0 64 32 A4 00")]
pub fn find_closest_hospital_restart_point(px: u32, py: u32, pz: u32, out: GPtr<u32>, heading: GPtr<f32>) {
    let t = Table {
        count: &NUM_HOSPITAL,
        when: HOSPITAL_WHEN.ptr().cast(),
        heads: HOSPITAL_HEADINGS.ptr().cast(),
        pts: HOSPITAL_POINTS.ptr().cast(),
        radius: &EXTRA_HOSPITAL_RADIUS,
        coors: EXTRA_HOSPITAL_COORS.ptr().cast(),
        extra_heading: &EXTRA_HOSPITAL_HEADING,
        best: 0x4B18_9680,
    };
    find_closest(&t, [px, py, pz], out, heading);
}

/// `CRestart::FindClosestPoliceRestartPoint(CVector, CVector*, float*)` (0x460A50).
#[hook(addr = 0x460A50, conv = cdecl, name = "CRestart::FindClosestPoliceRestartPoint", expect = "A0 64 32 A4 00")]
pub fn find_closest_police_restart_point(px: u32, py: u32, pz: u32, out: GPtr<u32>, heading: GPtr<f32>) {
    let t = Table {
        count: &NUM_POLICE,
        when: POLICE_WHEN.ptr().cast(),
        heads: POLICE_HEADINGS.ptr().cast(),
        pts: POLICE_POINTS.ptr().cast(),
        radius: &EXTRA_POLICE_RADIUS,
        coors: EXTRA_POLICE_COORS.ptr().cast(),
        extra_heading: &EXTRA_POLICE_HEADING,
        best: 0x47C3_4FF3,
    };
    find_closest(&t, [px, py, pz], out, heading);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_INITIALISE,
    HOOK_ADD_HOSPITAL_RESTART_POINT,
    HOOK_ADD_POLICE_RESTART_POINT,
    HOOK_OVERRIDE_NEXT_RESTART,
    HOOK_CANCEL_OVERRIDE_RESTART,
    HOOK_SET_RESPAWN_POINT_FOR_DURATION_OF_MISSION,
    HOOK_CLEAR_RESPAWN_POINT_FOR_DURATION_OF_MISSION,
    HOOK_FIND_CLOSEST_HOSPITAL_RESTART_POINT,
    HOOK_FIND_CLOSEST_POLICE_RESTART_POINT,
];
