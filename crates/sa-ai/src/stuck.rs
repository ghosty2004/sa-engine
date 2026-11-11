//! `CPedStuckChecker::TestPedStuck` (PORT-AI-5, helper A). Ported from the 1.0 US bytes (0x602C00-0x60347D); the
//! spec `docs/re/specs/AI/CPedStuckChecker.md` (medium confidence) was re-read branch by branch; where it differs
//! the bytes win (noted below). gta-reversed has no body for it. Every call to another catalogued function goes
//! through `orig!` at the original address.
//!
//! `CPedStuckChecker` (0x10): +0 last non-stuck point (CVector), +0xC u16 frame counter, +0xE u16 state (0 none,
//! 1 stuck, 2 pushed out).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTimer::ms_fTimeStep` (owned by CTimer).
const TIME_STEP: usize = 0xB7CB5C;
/// `CTimer::m_FrameCounter` (owned by CTimer).
const FRAME_COUNTER: usize = 0xB7CB4C;

orig! {
    /// `CPed::IsPlayer` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPedIntelligence::GetTaskSwim` (0x601070)
    fn intel_get_task_swim(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x601070 => unported;
    /// `CPedIntelligence::GetTaskJetPack` (0x601110)
    fn intel_get_task_jet_pack(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x601110 => unported;
    /// `CPedIntelligence::GetTaskClimb` (0x601180)
    fn intel_get_task_climb(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x601180 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60): out = a - b
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `VectorAdd(CVector* out, const CVector* a, const CVector* b)` (0x40FE30): out = a + b
    fn vector_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => unported;
    /// `FUN_00406da0` (thiscall on a CVector, ST0): `(x*x + y*y) + z*z`, the SQUARED length (no `fsqrt`)
    fn vector_squared_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CWorld::ProcessVerticalLine(const CVector*, float z, CColPoint*, CEntity**, bool buildings, bool vehicles,
    /// bool peds, bool objects, bool dummies, bool seeThrough, CStoredCollPoly*)` (0x5674E0, cdecl)
    fn world_process_vertical_line(pos: GPtr<u8>, z: u32, cp: GPtr<u8>, ent: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32, s: u32, l: u32) -> u8 = cdecl 0x5674E0 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector*, const CVector*, bool x7)` (0x56A490, cdecl)
    fn world_get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bb: u32, v: u32, p: u32, o: u32, d: u32, s: u32, c: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CPlaceable::SetPosn(const CVector*)` (0x4241C0, `ret 4`)
    fn placeable_set_posn(p: GPtr<u8>, v: GPtr<u8>) = thiscall 0x4241C0 => unported;
    /// `CVector::Normalise` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0, `ret 0xC`, the vector by value)
    fn physical_apply_move_force(p: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x5429F0 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(float x1, float y1, float x2, float y2)` (0x53CBE0, ST0)
    fn general_get_radian_angle_between_points(x1: u32, y1: u32, x2: u32, y2: u32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    fn general_limit_radian_angle(a: u32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CPlaceable::SetOrientation(float x, float y, float z)` (0x439A80, `ret 0xC`)
    fn placeable_set_orientation(p: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x439A80 => unported;
    /// `CEventGroup::GetEventOfType(int32)` (0x4AB650, `ret 4`)
    fn event_group_get_event_of_type(g: GPtr<u8>, t: i32) -> GPtr<u8> = thiscall 0x4AB650 => unported;
    /// `CEventStuckInAir::CEventStuckInAir(CPed*)` (0x4B1490, `ret 4`)
    fn stuck_in_air_ctor(e: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B1490 => unported;
    /// `FUN_004b1520` (`CEventStuckInAir::~CEventStuckInAir`)
    fn stuck_in_air_dtor(e: GPtr<u8>) = thiscall 0x4B1520 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420, `ret 8`)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
}

fn cf(addr: usize) -> X87 {
    x(g::<f32>(addr).read())
}

/// `fadd` / `fsub` / `fmul` operand 1.0f [0x858624].
fn one() -> f32 {
    g::<f32>(0x858624).read()
}

/// The ped's position: `m_matrix->pos` (+0x14 -> +0x30) when it has a matrix, else the placement (+4).
fn pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

fn local<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

fn fv(b: &[u32], i: usize) -> f32 {
    f32::from_bits(b[i])
}

/// `t = (0.01f > ms_fTimeStep) ? 0.01f : ms_fTimeStep` (`fld [0.01]; fcomp [step]; test ah, 0x41; jne`).
fn time_step() -> X87 {
    if cf(0x858C58).fcom(cf(TIME_STEP)).test_any(0x41) {
        cf(TIME_STEP)
    } else {
        cf(0x858C58)
    }
}

/// `cnt > 50.0f / (t + t)` and `cnt * 0.004f > |D.z|` (0x602DE5-0x602E3A and 0x602FED-0x603042).
fn stuck_vertically(cnt: f32, dz: f32) -> bool {
    let t = time_step();
    let q = t.fadd(t).fdivr_m(g::<f32>(0x858B40).read());
    if x(cnt).fcom(q).test_any(0x41) {
        return false;
    }
    let adz = x(f32::from_bits(dz.to_bits() & 0x7FFF_FFFF)); // fld; fabs
    !x(cnt).fmul_m(g::<f32>(0x859CE0).read()).fcom(adz).test_any(0x41)
}

/// "Hit by a ped going up": damage intensity > 0, a damage entity that is a ped (type 3) and an impact velocity z
/// > 0.3 (0x602EFA-0x602F33 and 0x60326A-0x6032A1).
fn hit_by_ped_upwards(ped: GPtr<u8>) -> bool {
    if x(rd::<f32>(ped, 0xD8)).fcom(cf(0x858B50)).test_any(0x41) {
        return false;
    }
    let e = rd::<GPtr<u8>>(ped, 0xDC);
    if e.is_null() || rd::<u8>(e, 0x36) & 7 != 3 {
        return false;
    }
    !x(rd::<f32>(ped, 0xE8)).fcom(cf(0x858C24)).test_any(0x41)
}

/// Outcome of the body before the tail.
enum Next {
    /// RESET then TAIL (0x602F6E)
    Reset,
    /// TAIL (0x602F76)
    Tail,
}

/// STUCK (0x602E40): one frame in 8 (`((u8)seed + m_FrameCounter + 3) & 7 == 0`) a vertical line from pos.z + 1
/// down to pos.z - 1 puts the ped on the ground (unless flagged +0x470 bit 0x200000 and the ground is not more
/// than 1.0 below, or hit upwards by a ped), and the ped becomes standing; on the other frames the state is 1.
fn stuck(this: GPtr<u8>, ped: GPtr<u8>) -> Next {
    let fc = g::<u32>(FRAME_COUNTER).read();
    let seed = rd::<u8>(ped, 0x20) as u32;
    if seed.wrapping_add(fc).wrapping_add(3) & 7 != 0 {
        let _ = rd::<u32>(ped, 0x480); // dead store into a local (0x602FC1)
        wr::<u16>(this, 0xE, 1);
        return Next::Tail;
    }
    let p = pos(ped);
    let mut v = [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)];
    v[2] = f32_of(x(fv(&v, 2)).fadd_m(one())).to_bits();
    let z = f32_of(x(rd::<f32>(p, 8)).fsub_m(one()));
    let mut cp1 = [0u32; 11];
    let mut ent = [0u32; 1];
    if world_process_vertical_line(local(&mut v), z.to_bits(), local(&mut cp1), local(&mut ent), 1, 1, 0, 1, 0, 0, 0) == 0 {
        return Next::Tail;
    }
    let ground_z = fv(&cp1, 2);
    let keep = rd::<u32>(ped, 0x470) & 0x20_0000 != 0
        && x(ground_z).fadd_m(one()).fcom(x(rd::<f32>(pos(ped), 8))).test_parity(5);
    if !keep && !hit_by_ped_upwards(ped) {
        v[2] = f32_of(x(ground_z).fadd_m(one())).to_bits();
        placeable_set_posn(ped, local(&mut v));
        let f = rd::<u32>(ped, 0x470);
        if f & 0x20_0000 != 0 {
            wr::<u32>(ped, 0x470, f & 0xFFDF_FFFF);
        }
    }
    wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) | 1);
    Next::Reset
}

/// One probe of the push-out (0x60307A-0x6031A1): a vertical line from `pos ± V` down to z = -20.0 (absolute) and
/// the line of sight from pos to `pos ± V`; the ground z, or `miss` (5001.0f / 5002.0f) when either test fails.
fn probe(ped: GPtr<u8>, v: &mut [u32; 3], cp: &mut [u32; 11], ent: &mut [u32; 1], add: bool, miss: u32) -> u32 {
    let side = |out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>| if add { vector_add(out, a, b) } else { vector_sub(out, a, b) };
    let mut tmp = [0u32; 3];
    let s = side(local(&mut tmp), pos(ped), local(v));
    if world_process_vertical_line(s, 0xC1A0_0000, local(cp), local(ent), 1, 0, 0, 0, 0, 0, 0) == 0 {
        return miss;
    }
    let p = pos(ped);
    let s = side(local(&mut tmp), p, local(v));
    if world_get_is_line_of_sight_clear(p, s, 1, 1, 0, 1, 0, 0, 0) == 0 {
        return miss;
    }
    cp[2]
}

/// PUSH-OUT (0x602FD8): one collided building and stuck vertically: probe the ground on both sides across the
/// movement (`V = normalise(-D.y, D.x, 1)`), stand the ped on the better side, push it away from the higher side
/// (`ApplyMoveForce(4 V.x, 4 V.y, 4)`), move it by `V / 4` (with V.z = 1) and turn it to the push direction.
fn push_out(this: GPtr<u8>, ped: GPtr<u8>, cnt: f32, d: &[u32; 3]) -> Next {
    let e = rd::<GPtr<u8>>(ped, 0xBC);
    if e.is_null() || rd::<u8>(e, 0x36) & 7 != 1 || !stuck_vertically(cnt, fv(d, 2)) {
        return Next::Tail;
    }
    let mut v = [f32_of(x(fv(d, 1)).fchs()).to_bits(), d[0], 0x3F80_0000];
    vector_normalise(local(&mut v));
    let mut cp1 = [0u32; 11];
    let mut cp2 = [0u32; 11];
    let mut ent = [0u32; 1];
    let z1 = x(f32::from_bits(probe(ped, &mut v, &mut cp1, &mut ent, true, 0x459C_4800)));
    let z2 = x(f32::from_bits(probe(ped, &mut v, &mut cp2, &mut ent, false, 0x459C_5000)));
    let choice: i16 = if !z1.fcom(cf(0x86CD78)).test_any(0x41) && !z2.fcom(cf(0x86CD78)).test_any(0x41) {
        -1
    } else if !x(rd::<f32>(pos(ped), 8)).fsub_m(one()).fcom(z1).test_parity(5)
        && !z1.fcom(cf(0x86CD78)).test_parity(5)
        && (!z2.fcom(cf(0x86CD74)).test_parity(5) || !z1.fcom(z2).test_parity(5))
    {
        1
    } else if !x(rd::<f32>(pos(ped), 8)).fsub_m(one()).fcom(z2).test_parity(5) && !z2.fcom(cf(0x86CD74)).test_parity(5) {
        2
    } else {
        0
    };
    if (choice == 1 || choice == 2) && rd::<u32>(ped, 0x470) & 0x20_0000 == 0 && !hit_by_ped_upwards(ped) {
        placeable_set_posn(ped, if choice == 2 { local(&mut cp2) } else { local(&mut cp1) });
        let p = pos(ped);
        wr::<f32>(p, 8, f32_of(x(rd::<f32>(p, 8)).fadd_m(one())));
    }
    if !z1.fcom(z2).test_parity(5) {
        let m = g::<f32>(0x858C1C).read();
        for c in v.iter_mut() {
            *c = f32_of(x(f32::from_bits(*c)).fmul_m(m)).to_bits();
        }
    }
    let four = g::<f32>(0x858B90).read();
    let fy = f32_of(x(fv(&v, 1)).fmul_m(four));
    let fx = f32_of(x(fv(&v, 0)).fmul_m(four));
    v[2] = 0x3F80_0000;
    physical_apply_move_force(ped, fx.to_bits(), fy.to_bits(), 0x4080_0000);
    if choice > -1 {
        let q = g::<f32>(0x858C84).read();
        let mut off = [0u32; 3];
        for (o, c) in off.iter_mut().zip(v.iter()) {
            *o = f32_of(x(f32::from_bits(*c)).fmul_m(q)).to_bits();
        }
        let mut tmp = [0u32; 3];
        let np = vector_add(local(&mut tmp), pos(ped), local(&mut off));
        placeable_set_posn(ped, np);
        let h = f32_of(X87::from_f64_exact(general_get_radian_angle_between_points(v[0], v[1], 0, 0)));
        wr::<f32>(ped, 0x558, h);
        let h = f32_of(X87::from_f64_exact(general_limit_radian_angle(h.to_bits())));
        wr::<f32>(ped, 0x558, h);
        wr::<f32>(ped, 0x55C, h);
        placeable_set_orientation(ped, 0, 0, h.to_bits());
    }
    wr::<u16>(this, 0xE, 2);
    Next::Tail
}

/// Steps 2..8 of the spec (0x602C59-0x60341E).
fn body(this: GPtr<u8>, ped: GPtr<u8>) -> Next {
    if rd::<u8>(ped, 0x46C) & 1 != 0 || (rd::<u32>(ped, 0x46C) >> 1) & 1 != 0 {
        return Next::Reset;
    }
    let damaged = !rd::<GPtr<u8>>(ped, 0xDC).is_null() && !x(rd::<f32>(ped, 0xD8)).fcom(cf(0x858B50)).test_any(0x41);
    if !damaged && (ped_is_player(ped) == 0 || (rd::<u32>(ped, 0x1C) >> 4) & 1 == 0) {
        return Next::Reset;
    }
    if !intel_get_task_swim(rd::<GPtr<u8>>(ped, 0x47C)).is_null()
        || !intel_get_task_jet_pack(rd::<GPtr<u8>>(ped, 0x47C)).is_null()
        || !intel_get_task_climb(rd::<GPtr<u8>>(ped, 0x47C)).is_null()
    {
        return Next::Reset;
    }
    let mut d = [0u32; 3];
    let n = rd::<u16>(this, 0xC);
    if n > 10000 || n == 0 {
        wr::<u16>(this, 0xC, 1);
        wr::<u16>(this, 0xE, 0);
        let p = pos(ped);
        for k in 0..3isize {
            wr::<u32>(this, 4 * k, rd::<u32>(p, 4 * k));
        }
    } else {
        let mut tmp = [0u32; 3];
        let r = vector_sub(local(&mut tmp), pos(ped), this);
        d = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
        wr::<u16>(this, 0xC, n.wrapping_add(1));
    }
    let t = time_step();
    let cnt = rd::<u16>(this, 0xC) as f32; // fild; fstp dword (exact)
    let q = cf(0x858B90).fmul(t).fdivr_m(g::<f32>(0x858B40).read());
    if !x(cnt).fcom(q).test_any(0x41) {
        let m = X87::from_f64_exact(vector_squared_magnitude(local(&mut d)));
        if !x(cnt).fmul_m(g::<f32>(0x858C58).read()).fcom(m).test_any(0x41) {
            return stuck(this, ped);
        }
    }
    match rd::<u8>(ped, 0xB9) {
        0 => Next::Tail,
        1 => push_out(this, ped, cnt, &d),
        _ => {
            if stuck_vertically(cnt, fv(&d, 2)) {
                stuck(this, ped)
            } else {
                Next::Tail
            }
        }
    }
}

/// `CPedStuckChecker::TestPedStuck(CPed*, CEventGroup*)` (0x602C00, `ret 8`, AL).
///
/// Without collision, attached, or in ped state 0x36 / 0x37: reset, false. Standing peds, peds neither damaged
/// (damage entity and intensity > 0) nor a player with bIsStuck, swimming / jet-pack / climbing peds: reset. Else
/// the frame counter runs (restarting at 1 with the current position after 10000 frames) and D = pos - last
/// non-stuck point; STUCK when `cnt > 50 / (4 t)` and `cnt * 0.01 > |D|²` (0x406DA0 returns the SQUARED length:
/// the spec's "sqrt" is not in the bytes), or with two or more collided entities when `cnt > 50 / (2 t)` and
/// `cnt * 0.004 > |D.z|`; one collided building tries the push-out. Tail: with a non-zero state, an existing event
/// 0x42 whose +0xC exceeds 1.0 clears bIsStanding (false); otherwise a `CEventStuckInAir(ped)` is added to `events`
/// (a stack object in the original, 0x10 bytes: a zeroed local here) and the result is true.
#[hook(addr = 0x602C00, conv = thiscall, name = "CPedStuckChecker::TestPedStuck", expect = "6A FF 64 A1 00 00 00 00")]
pub fn test_ped_stuck(this: GPtr<u8>, ped: GPtr<u8>, events: GPtr<u8>) -> u8 {
    let st = rd::<i32>(ped, 0x530);
    if rd::<u8>(ped, 0x1C) & 1 == 0 || !rd::<GPtr<u8>>(ped, 0xFC).is_null() || st == 0x36 || st == 0x37 {
        wr::<u16>(this, 0xC, 0);
        wr::<u16>(this, 0xE, 0);
        return 0;
    }
    if let Next::Reset = body(this, ped) {
        wr::<u16>(this, 0xC, 0);
        wr::<u16>(this, 0xE, 0);
    }
    if rd::<u16>(this, 0xE) == 0 {
        return 0;
    }
    if !event_group_get_event_of_type(events, 0x42).is_null() {
        let e = event_group_get_event_of_type(events, 0x42);
        if !x(rd::<f32>(e, 0xC)).fcom(cf(0x858624)).test_any(0x41) {
            wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & 0xFFFF_FFFE);
            return 0;
        }
    }
    let mut ev = [0u32; 4];
    stuck_in_air_ctor(local(&mut ev), ped);
    event_group_add(events, local(&mut ev), 0);
    stuck_in_air_dtor(local(&mut ev));
    1
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[HOOK_TEST_PED_STUCK];
