//! CCarCtrl driving AI leftovers (WRITER-POP-4, helper A). Ported from the 1.0 US bytes; gta-reversed and
//! `docs/re/specs/POP/*.md` as reading aids (gta-reversed has none of these functions reversed). x87
//! sequences follow the original instruction by instruction: values the bytes keep on the FPU stack stay
//! [`X87`], values they store are rounded to f32 where they are stored.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{rd, wr};

/// Globals owned by this module: none.
pub const GLOBALS: &[GlobalDesc] = &[];

/// This module's hooks.
pub const HOOKS: &[HookReg] = &[
    HOOK_FLY_AI_HELI_IN_CERTAIN_DIRECTION,
    HOOK_FLY_AI_HELI_TO_TARGET_FIXED_ORIENTATION,
    HOOK_STEER_AI_CAR_PARK_PARALLEL,
    HOOK_STEER_AI_CAR_PARK_PERPENDICULAR,
    HOOK_STEER_AI_CAR_WITH_PHYSICS_FOLLOW_PRE_RECORDED_PATH,
    HOOK_DRAG_CAR_TO_POINT,
    HOOK_FLY_AI_PLANE_IN_CERTAIN_DIRECTION,
    HOOK_DEAL_WITH_BEND_RACING,
    HOOK_STEER_AI_CAR_WITH_PHYSICS_FOLLOW_PATH_RACING,
];

// ---------------------------------------------------------------------------------------------------------
// Shorthands

/// An f32 constant of the exe by its bits.
#[inline]
fn k(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// `fld dword`.
#[inline]
fn x_(v: f32) -> X87 {
    X87::from_f32(v)
}

/// `fild dword`.
#[inline]
fn xi_(v: i32) -> X87 {
    X87::from_i32(v)
}

/// `fcomp; fnstsw ax; test ah, 5; jp` taken: `a >= b` or unordered.
#[inline]
fn ge5(a: X87, b: X87) -> bool {
    a.fcom(b).test_parity(0x05)
}

/// `fcomp; fnstsw ax; test ah, 0x41; jne` taken: `a <= b` or unordered.
#[inline]
fn le41(a: X87, b: X87) -> bool {
    a.fcom(b).test_any(0x41)
}

/// `CEntity::GetPosition()` inlined: the matrix position (+0x14 → +0x30) or the placement (+4).
fn entity_pos(e: GPtr<u8>) -> GPtr<f32> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4).cast()
    } else {
        m.byte_add(0x30).cast()
    }
}

#[inline]
fn vx(p: GPtr<f32>, i: usize) -> f32 {
    p.wrapping_index(i as i32).read()
}

/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84).
fn time_ms() -> u32 {
    crate::common::g::<u32>(0xB7CB84).read()
}

/// `CTimer::m_snPreviousTimeInMilliseconds` (0xB7CB78).
fn prev_time_ms() -> u32 {
    crate::common::g::<u32>(0xB7CB78).read()
}

/// `CTimer::ms_fTimeStep` (0xB7CB5C).
fn time_step() -> f32 {
    crate::common::g::<f32>(0xB7CB5C).read()
}

/// `GetATanOfXY` through `orig!` as an x87 value.
fn atan_xy(x: f32, y: f32) -> X87 {
    X87::from_f64_exact(crate::common::get_atan_of_xy(x, y))
}

orig! {
    /// `CVector::Normalise()` (0x59C910, thiscall)
    fn vector_normalise(v: GPtr<f32>) = thiscall 0x59C910 => unported;
    /// `CWorld::ProcessLineOfSight(const CVector&, const CVector&, CColPoint&, CEntity*&, buildings,
    /// vehicles, peds, objects, dummies, seeThrough, ignoreCamera, shootThrough)` (0x56BA00)
    fn process_line_of_sight(s: GPtr<f32>, e: GPtr<f32>, cp: GPtr<u8>, ent: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32, st: u32, ci: u32, sh: u32) -> u8 = cdecl 0x56BA00 => unported;
}

/// `ProcessLineOfSight(from, to, cp, ent, 1, 0, 0, 0, 0, 0, 0, 1)` (buildings only, shoot-through), as
/// every call of FlyAIHeliInCertainDirection makes it.
fn los_buildings(from: GPtr<f32>, to: GPtr<f32>, cp: GPtr<u8>, ent: GPtr<u8>) -> bool {
    process_line_of_sight(from, to, cp, ent, 1, 0, 0, 0, 0, 0, 0, 1) != 0
}

#[inline]
fn vp(v: &mut [f32; 3]) -> GPtr<f32> {
    GPtr::from_raw(v.as_mut_ptr())
}

/// `while (d > PI) d -= 2PI;` (`fcom PI; test ah, 0x41; je loop`) on the x87 value.
fn wrap_down(mut d: X87) -> X87 {
    while !le41(d, x_(k(0x4049_0FDB))) {
        d = d.fsub_m(k(0x40C9_0FDB));
    }
    d
}

/// `while (d < -PI) d += 2PI;` (`fcom -PI; test ah, 5; jnp loop`) on the x87 value.
fn wrap_up(mut d: X87) -> X87 {
    while !ge5(d, x_(k(0xC049_0FDB))) {
        d = d.fadd_m(k(0x40C9_0FDB));
    }
    d
}

// ---------------------------------------------------------------------------------------------------------
// Helis

/// `CCarCtrl::FlyAIHeliInCertainDirection(CHeli* heli, float heading, float distance, bool approach)`
/// (0x429A70, cdecl). The heli controls from a wanted heading:
/// 1. News heli (mission 0x29) with `(float)+0x3DF > distance` and `|mv.xy| < 0.01`: `slow`, heading
///    += pi/2 (the argument slot, used by everything below).
/// 2. On 500 ms boundary frames (`(now + seed) % 500 < (prev + seed) % 500`, unsigned, seed +0x20): +0x9AC =
///    +0x9A8; `A = P + mv * 50` (x / y products stored f32 first, z kept on the FPU); `c = fcos(heading)`,
///    `s = fsin(heading)` stored f32; a line of sight from A along `Normalise(c, s, -1) * 60` that hits
///    raises +0x9AC to the hit height plus +0x9B0 (half of it when `slow`; `test ah, 0x41`: equal / NaN
///    take the hit). Then `len = |mv.xy| * 100 + (bIsRCVehicle ? 5 : 30)` (f32), +0x9B8 = 1, and a probe
///    from A along `(c, s, 0) * len` (z: `len * 0.0`): a miss clears +0x9B8 and +0x994. A hit probes both
///    sides (`P ± Normalise(right.xy, 0) * 10`): a blocked side point gives 0, a hit of its own probe the
///    distance to the hit (the right one stored f32, the left one kept on the FPU), otherwise 1000;
///    +0x994 = `left > right ? 0.5 : -0.5` (equal / NaN → -0.5).
/// 3. Mission 0x30: +0x9B8 = 0, +0x994 = 0.
/// 4. Lift +0x998: `(+0x9AC - (mv.z * 100 + P.z)) * (> 0 ? 0.1 : 0.2)` plus `((rand() & 15) - 7) * 0.002`,
///    clamped to [-0.3, 1] (>= 1 / NaN → 1); `q = v - 0.5` kept for step 8.
/// 5. Yaw +0x98C: `(heading - atan(fwd)) ` wrapped (first `> pi`, then `< -pi`) times -2, clamped to
///    [-1, 1] (NaN → 1).
/// 6. Pitch +0x990: -0.8 when `distance > 60` or `!approach`; otherwise with `e = |P.xy + mv.xy * 50 -
///    dest.xy| - +0x3DF`: unchanged when `e < 0`, else `e * -0.8 / (30 - +0x3DF)`.
/// 7. Pitch < 0: times `max(0, 1 - 2.1220658 * |wrap(atan(fwd) - heading)|)` (wrapped `< -pi` first);
///    0.3 when that angle exceeds pi/2 and the heli moves forward.
/// 8. `q > 0`: pitch *= `1 - min(q, 1)`.
/// 9. Pitch < 0: `max(pitch, -clamp((cruise - fwd·mv * 60) * 0.1, 0, 1))`; with a non-zero +0x3D9, pitch <
///    -0.2 and +0x9B8 clear, `mv += fwd * (int8)+0x3D9 * timestep * 0.001` (z times 0.0).
/// 10. +0x9B8: pitch = `Normalise(fwd.xy, 0) · mv * 2`. 11. Pitch clamped to [-1, 1] (NaN → 1).
///
/// The bytes match `docs/re/specs/POP/CCarCtrl.p2.md` (gta-reversed: `plugin::Call`). Every comparison is
/// the bytes' `fcomp`/`test ah` with its NaN direction.
#[hook(addr = 0x429A70, conv = cdecl, name = "CCarCtrl::FlyAIHeliInCertainDirection", expect = "81 EC 90 00 00 00 53 56",
       unverified = "popw4-drive-1", route = "L2", hazards = "heap-writes,x87-transcendental",
       recs = "police-gang,air-water")]
pub fn fly_ai_heli_in_certain_direction(heli: GPtr<u8>, heading: f32, distance: f32, approach: u8) {
    let mut heading = heading;
    let mut slow = false;
    // 1.
    if rd::<u8>(heli, 0x3BA) == 0x29 && !le41(xi_(i32::from(rd::<u8>(heli, 0x3DF))), x_(distance)) {
        let (my, mx) = (rd::<f32>(heli, 0x48), rd::<f32>(heli, 0x44));
        let sp = x_(mx).fmul(x_(mx)).fadd(x_(my).fmul(x_(my))).fsqrt();
        if !ge5(sp, x_(k(0x3C23_D70A))) {
            slow = true;
            heading = x_(heading).fadd_m(k(0x3FC9_0FDB)).to_f32();
        }
    }
    // 2. the probes
    let seed = u32::from(rd::<u16>(heli, 0x20));
    let prev = prev_time_ms().wrapping_add(seed) % 500;
    let now = time_ms().wrapping_add(seed) % 500;
    if now < prev {
        wr::<u32>(heli, 0x9AC, rd::<u32>(heli, 0x9A8));
        let mvx50 = x_(rd::<f32>(heli, 0x44)).fmul_m(50.0).to_f32(); // [E+0x24]
        let mvy50 = x_(rd::<f32>(heli, 0x48)).fmul_m(50.0).to_f32(); // [E+0x28]
        let mvz50 = x_(rd::<f32>(heli, 0x4C)).fmul_m(50.0);
        let p = entity_pos(heli);
        let mut a = [
            x_(mvx50).fadd_m(vx(p, 0)).to_f32(),
            x_(mvy50).fadd_m(vx(p, 1)).to_f32(),
            mvz50.fadd_m(vx(p, 2)).to_f32(),
        ]; // [E+0x34]
        let c = x_(heading).fcos().to_f32(); // [E+0x1C]
        let s = x_(heading).fsin().to_f32(); // [E+0x20]
        let mut d = [c, s, -1.0]; // [E+0x4C]
        vector_normalise(vp(&mut d));
        let mut b = [
            x_(d[0]).fmul_m(60.0).fadd_m(a[0]).to_f32(),
            x_(d[1]).fmul_m(60.0).fadd_m(a[1]).to_f32(),
            x_(x_(d[2]).fmul_m(60.0).to_f32()).fadd_m(a[2]).to_f32(),
        ]; // [E+0x58]
        let mut cp = [0u32; 11]; // CColPoint [E+0x70]
        let cpp: GPtr<u8> = GPtr::from_raw(cp.as_mut_ptr()).cast();
        let mut ent = 0u32; // [E+0x30] (CEntity*, never read)
        let entp: GPtr<u8> = GPtr::from_raw(&mut ent as *mut u32).cast();
        if los_buildings(vp(&mut a), vp(&mut b), cpp, entp) {
            let cpz = f32::from_bits(cp[2]);
            let h = if slow {
                x_(rd::<f32>(heli, 0x9B0)).fmul_m(0.5).fadd_m(cpz)
            } else {
                x_(cpz).fadd_m(rd::<f32>(heli, 0x9B0))
            };
            let cur = x_(rd::<f32>(heli, 0x9AC));
            let v = if le41(cur, h) { h } else { cur };
            wr::<f32>(heli, 0x9AC, v.to_f32());
        }
        let (vxm, vym) = (rd::<f32>(heli, 0x44), rd::<f32>(heli, 0x48));
        let r = if rd::<u8>(heli, 0x42C) & 0x80 != 0 {
            5.0
        } else {
            30.0
        };
        let len = x_(vym)
            .fmul_m(vym)
            .fadd(x_(vxm).fmul_m(vxm))
            .fsqrt()
            .fmul_m(100.0)
            .fadd_m(r)
            .to_f32(); // [E+0x1C]
        d = [c, s, 0.0];
        wr::<u8>(heli, 0x9B8, 1);
        b = [
            x_(c).fmul_m(len).fadd_m(a[0]).to_f32(),
            x_(s).fmul_m(len).fadd_m(a[1]).to_f32(),
            x_(x_(len).fmul_m(0.0).to_f32()).fadd_m(a[2]).to_f32(),
        ];
        if !los_buildings(vp(&mut a), vp(&mut b), cpp, entp) {
            wr::<u8>(heli, 0x9B8, 0);
            wr::<u32>(heli, 0x994, 0);
        } else {
            let m = rd::<GPtr<u8>>(heli, 0x14);
            let mut rv = [rd::<f32>(m, 0), rd::<f32>(m, 4), 0.0]; // [E+0x40]
            vector_normalise(vp(&mut rv));
            let hit_dist = |cp: &[u32; 11], a: &[f32; 3]| {
                let dx = x_(f32::from_bits(cp[0])).fsub_m(a[0]);
                let dy = x_(f32::from_bits(cp[1])).fsub_m(a[1]);
                let dz = x_(f32::from_bits(cp[2])).fsub_m(a[2]);
                dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt()
            };
            // the right side
            let p = entity_pos(heli);
            let rz10 = x_(rv[2]).fmul_m(10.0).to_f32();
            a = [
                x_(rv[0]).fmul_m(10.0).fadd_m(vx(p, 0)).to_f32(),
                x_(rv[1]).fmul_m(10.0).fadd_m(vx(p, 1)).to_f32(),
                x_(rz10).fadd_m(vx(p, 2)).to_f32(),
            ];
            b = [
                x_(d[0]).fmul_m(len).fadd_m(a[0]).to_f32(),
                x_(d[1]).fmul_m(len).fadd_m(a[1]).to_f32(),
                x_(x_(d[2]).fmul_m(len).to_f32()).fadd_m(a[2]).to_f32(),
            ];
            let mut right = 1000.0f32; // [E+0xC]
            if los_buildings(entity_pos(heli), vp(&mut a), cpp, entp) {
                right = 0.0;
            } else if los_buildings(vp(&mut a), vp(&mut b), cpp, entp) {
                right = hit_dist(&cp, &a).to_f32();
            }
            // the left side
            let p = entity_pos(heli);
            let lz10 = x_(rv[2]).fmul_m(10.0).to_f32();
            a = [
                x_(vx(p, 0)).fsub(x_(rv[0]).fmul_m(10.0)).to_f32(),
                x_(vx(p, 1)).fsub(x_(rv[1]).fmul_m(10.0)).to_f32(),
                x_(vx(p, 2)).fsub_m(lz10).to_f32(),
            ];
            b = [
                x_(d[0]).fmul_m(len).fadd_m(a[0]).to_f32(),
                x_(d[1]).fmul_m(len).fadd_m(a[1]).to_f32(),
                x_(x_(d[2]).fmul_m(len).to_f32()).fadd_m(a[2]).to_f32(),
            ];
            let left = if los_buildings(entity_pos(heli), vp(&mut a), cpp, entp) {
                x_(0.0)
            } else if los_buildings(vp(&mut a), vp(&mut b), cpp, entp) {
                hit_dist(&cp, &a)
            } else {
                x_(1000.0)
            };
            let v: u32 = if le41(left, x_(right)) {
                0xBF00_0000
            } else {
                0x3F00_0000
            };
            wr::<u32>(heli, 0x994, v);
        }
    }
    // 3.
    if rd::<u8>(heli, 0x3BA) == 0x30 {
        wr::<u8>(heli, 0x9B8, 0);
        wr::<u32>(heli, 0x994, 0);
    }
    // 4. the lift
    let m = rd::<GPtr<u8>>(heli, 0x14);
    let cur = atan_xy(rd::<f32>(m, 0x10), rd::<f32>(m, 0x14)).to_f32(); // [E+0x1C]
    let p = entity_pos(heli);
    wr::<u32>(heli, 0x998, 0);
    let lift = x_(rd::<f32>(heli, 0x4C))
        .fmul_m(100.0)
        .fadd_m(vx(p, 2))
        .fsubr_m(rd::<f32>(heli, 0x9AC));
    let lift = if le41(lift, x_(0.0)) {
        lift.fmul_m(k(0x3E4C_CCCD))
    } else {
        lift.fmul_m(k(0x3DCC_CCCD))
    };
    wr::<f32>(heli, 0x998, lift.to_f32());
    let r = crate::common::rand_at(0x42A0FB) & 0xF;
    let v = xi_(r)
        .fsub_m(7.0)
        .fmul_m(k(0x3B03_126F))
        .fadd_m(rd::<f32>(heli, 0x998));
    wr::<f32>(heli, 0x998, v.to_f32());
    let q = v.fsub_m(0.5).to_f32(); // [E+0x30]
    let vf = v.to_f32(); // [E+0xC]
    let lift = if ge5(v, x_(1.0)) {
        1.0
    } else if le41(x_(k(0xBE99_999A)), x_(vf)) {
        vf
    } else {
        k(0xBE99_999A)
    };
    wr::<f32>(heli, 0x998, lift);
    // 5. the yaw
    let d = wrap_up(wrap_down(x_(heading).fsub_m(cur))).fmul_m(-2.0);
    wr::<f32>(heli, 0x98C, d.to_f32());
    let yaw = if ge5(d, x_(1.0)) {
        x_(1.0)
    } else if le41(x_(-1.0), d) {
        d
    } else {
        x_(-1.0)
    };
    wr::<f32>(heli, 0x98C, yaw.to_f32());
    // 6. the pitch target
    if le41(x_(distance), x_(60.0)) && approach != 0 {
        let fx = x_(rd::<f32>(heli, 0x44)).fmul_m(50.0);
        let fy50 = x_(rd::<f32>(heli, 0x48)).fmul_m(50.0).to_f32(); // [E+0x68]
        let p = entity_pos(heli);
        let fx = fx.fadd_m(vx(p, 0));
        let fy = x_(fy50).fadd_m(vx(p, 1));
        let ex = fx.fsub_m(rd::<f32>(heli, 0x3EC)).to_f32(); // [E+0x24]
        let ey = fy.fsub_m(rd::<f32>(heli, 0x3F0));
        let td = xi_(i32::from(rd::<u8>(heli, 0x3DF))).to_f32(); // [E+0xC]
        let e = ey.fmul(ey).fadd(x_(ex).fmul_m(ex)).fsqrt().fsub_m(td);
        if ge5(e, x_(0.0)) {
            let v = e.fmul_m(k(0xBF4C_CCCD)).fdiv(x_(30.0).fsub_m(td));
            wr::<f32>(heli, 0x990, v.to_f32());
        }
    } else {
        wr::<u32>(heli, 0x990, 0xBF4C_CCCD);
    }
    // 7.
    if !ge5(x_(rd::<f32>(heli, 0x990)), x_(0.0)) {
        let mut d = wrap_down(wrap_up(x_(cur).fsub_m(heading)));
        if !ge5(d, x_(0.0)) {
            d = d.fchs();
        }
        let f = x_(k(0x4007_CFED)).fmul(d).fsubr_m(1.0);
        let f = if le41(x_(0.0), f) { f } else { x_(0.0) };
        wr::<f32>(heli, 0x990, f.fmul_m(rd::<f32>(heli, 0x990)).to_f32());
        if !le41(d, x_(k(0x3FC9_0FDB))) {
            let m = rd::<GPtr<u8>>(heli, 0x14);
            let dot = x_(rd::<f32>(heli, 0x4C))
                .fmul_m(rd::<f32>(m, 0x18))
                .fadd(x_(rd::<f32>(heli, 0x48)).fmul_m(rd::<f32>(m, 0x14)))
                .fadd(x_(rd::<f32>(m, 0x10)).fmul_m(rd::<f32>(heli, 0x44)));
            if !le41(dot, x_(0.0)) {
                wr::<u32>(heli, 0x990, 0x3E99_999A);
            }
        }
    }
    // 8.
    if !le41(x_(q), x_(0.0)) {
        let gq = if ge5(x_(q), x_(1.0)) { x_(1.0) } else { x_(q) };
        let v = x_(1.0).fsub(gq).fmul_m(rd::<f32>(heli, 0x990));
        wr::<f32>(heli, 0x990, v.to_f32());
    }
    // 9.
    let mut probe_set_jump = false;
    if !ge5(x_(rd::<f32>(heli, 0x990)), x_(0.0)) {
        let m = rd::<GPtr<u8>>(heli, 0x14);
        let dot = x_(rd::<f32>(heli, 0x4C))
            .fmul_m(rd::<f32>(m, 0x18))
            .fadd(x_(rd::<f32>(heli, 0x48)).fmul_m(rd::<f32>(m, 0x14)))
            .fadd(x_(rd::<f32>(heli, 0x44)).fmul_m(rd::<f32>(m, 0x10)));
        let kv = xi_(i32::from(rd::<u8>(heli, 0x3D0)))
            .fsub(dot.fmul_m(60.0))
            .fmul_m(k(0x3DCC_CCCD));
        // `1 >= k` (or NaN): `0 <= k` (or NaN) keeps k, else 0; `1 < k`: 1
        let kv = if ge5(x_(1.0), kv) {
            if le41(x_(0.0), kv) { kv } else { x_(0.0) }
        } else {
            x_(1.0)
        };
        let nk = kv.fchs();
        let cur = rd::<f32>(heli, 0x990);
        let v = if ge5(nk, x_(cur)) { nk } else { x_(cur) };
        wr::<f32>(heli, 0x990, v.to_f32());
        let mult = rd::<i8>(heli, 0x3D9);
        if mult != 0 && !ge5(v, x_(k(0xBE4C_CCCD))) {
            if rd::<u8>(heli, 0x9B8) != 0 {
                probe_set_jump = true;
            } else {
                let b = xi_(i32::from(mult)).fmul_m(time_step()).fmul_m(k(0x3A83_126F));
                let fwd = [rd::<f32>(m, 0x10), rd::<f32>(m, 0x14), rd::<f32>(m, 0x18)];
                let by = x_(fwd[1]).fmul(b).to_f32();
                let bz = b.fmul_m(0.0).to_f32();
                let nx = x_(fwd[0]).fmul(b).fadd_m(rd::<f32>(heli, 0x44)).to_f32();
                let ny = x_(by).fadd_m(rd::<f32>(heli, 0x48)).to_f32();
                let nz = x_(bz).fadd_m(rd::<f32>(heli, 0x4C)).to_f32();
                wr::<f32>(heli, 0x44, nx);
                wr::<f32>(heli, 0x48, ny);
                wr::<f32>(heli, 0x4C, nz);
            }
        }
    }
    // 10.
    if probe_set_jump || rd::<u8>(heli, 0x9B8) != 0 {
        let m = rd::<GPtr<u8>>(heli, 0x14);
        let mut f = [rd::<f32>(m, 0x10), rd::<f32>(m, 0x14), 0.0];
        vector_normalise(vp(&mut f));
        let v = x_(f[1])
            .fmul_m(rd::<f32>(heli, 0x48))
            .fadd(x_(f[2]).fmul_m(rd::<f32>(heli, 0x4C)))
            .fadd(x_(f[0]).fmul_m(rd::<f32>(heli, 0x44)));
        wr::<f32>(heli, 0x990, v.fadd(v).to_f32());
    }
    // 11.
    let v = x_(rd::<f32>(heli, 0x990));
    if ge5(v, x_(1.0)) {
        wr::<u32>(heli, 0x990, 0x3F80_0000);
    } else if !le41(x_(-1.0), v) {
        wr::<u32>(heli, 0x990, 0xBF80_0000);
    }
}

/// The yaw / roll clamp pattern of the heli steering (`fcom 1.0; fst field; test ah, 5; jp`, then `-1.0`
/// against the value with `test ah, 0x41`): `>= 1` or NaN → 1, `< -1` → -1, otherwise the x87 value.
fn clamp_unit(v: X87, lo_bits: u32, hi_bits: u32) -> X87 {
    if ge5(v, x_(k(hi_bits))) {
        x_(k(hi_bits))
    } else if le41(x_(k(lo_bits)), v) {
        v
    } else {
        x_(k(lo_bits))
    }
}

/// `CCarCtrl::FlyAIHeliToTarget_FixedOrientation(CHeli* heli, float orientation, CVector target)`
/// (0x423940, cdecl, the vector by value). On 500 ms boundary frames of the seed-shifted clock (as
/// [`fly_ai_heli_in_certain_direction`]): +0x9AC = +0x9A8, and a line of sight from `P + mv * 50` along
/// `Normalise(cos o, sin o, -1) * 60` that hits raises +0x9AC to `hit.z + +0x9B0` (`test ah, 5; jp`:
/// `>=` / NaN take the hit). Lift +0x998: `(+0x9AC - (mv.z * 100 + P.z)) * (> 0 ? 0.1 : 0.2)` plus
/// `((rand() & 15) - 7) * 0.002`, clamped to [-0.3, 1] on the x87 value (NaN → 1). Yaw +0x98C:
/// `(o - atan(fwd))` wrapped (`> pi` first) times -0.5, clamped to [-1, 1]. With `d = target.xy - P.xy`
/// and the speeds along the right / forward axes `sR`, `sF` (stored f32): the lateral error
/// `lat = (d.y * R.y + d.x * R.x) + R.z * 0 + sR * 80` (stored f32) sets the roll +0x994 to `lat * -0.02`
/// (an f64 constant) when `|lat| >= 5`, else to `sR` (the raw f32; the bytes store the speed itself),
/// clamped to [-0.75, 0.75]; the forward error (same form with F, kept on the FPU) sets the pitch +0x990
/// to `fe * -0.015` (f64) when `|fe| >= 5`, else `sF`, clamped to [-0.5, 0.5]. NaN errors take the
/// multiplied branch (`jp`); the final clamps send NaN to the upper bound.
///
/// The original reuses its argument slots: the line of sight's `CEntity*&` and later `rand() & 15` and
/// `sR` go to the `heli` slot, `lat` to `orientation`, `sF` to `target.x`. Both callers (0x42A66E,
/// 0x42A994) only pop them (`add esp, 0x18`), so the port keeps them local.
#[hook(addr = 0x423940, conv = cdecl, name = "CCarCtrl::FlyAIHeliToTarget_FixedOrientation", expect = "A1 78 CB B7 00 83 EC 60",
       unverified = "popw4-drive-1", route = "L2", hazards = "heap-writes,x87-transcendental",
       recs = "police-gang,air-water")]
pub fn fly_ai_heli_to_target_fixed_orientation(heli: GPtr<u8>, orientation: f32, tx: f32, ty: f32, _tz: f32) {
    let prev = prev_time_ms();
    let seed = u32::from(rd::<u16>(heli, 0x20));
    let prev = prev.wrapping_add(seed) % 500;
    let now = time_ms().wrapping_add(seed) % 500;
    if now < prev {
        wr::<u32>(heli, 0x9AC, rd::<u32>(heli, 0x9A8));
        let mvx50 = x_(rd::<f32>(heli, 0x44)).fmul_m(50.0).to_f32();
        let mvy50 = x_(rd::<f32>(heli, 0x48)).fmul_m(50.0).to_f32();
        let mvz50 = x_(rd::<f32>(heli, 0x4C)).fmul_m(50.0);
        let p = entity_pos(heli);
        let mut s = [
            x_(mvx50).fadd_m(vx(p, 0)).to_f32(),
            x_(mvy50).fadd_m(vx(p, 1)).to_f32(),
            mvz50.fadd_m(vx(p, 2)).to_f32(),
        ]; // [S+0x18]
        let mut d = [
            x_(orientation).fcos().to_f32(),
            x_(orientation).fsin().to_f32(),
            -1.0,
        ]; // [S+0xC]
        vector_normalise(vp(&mut d));
        let dz60 = x_(d[2]).fmul_m(60.0).to_f32(); // [S+0x38]
        let mut e = [
            x_(d[0]).fmul_m(60.0).fadd_m(s[0]).to_f32(),
            x_(d[1]).fmul_m(60.0).fadd_m(s[1]).to_f32(),
            x_(dz60).fadd_m(s[2]).to_f32(),
        ]; // [S+0x24]
        let mut cp = [0u32; 11]; // [S+0x3C]
        let mut ent = 0u32; // the `heli` argument slot
        if los_buildings(
            vp(&mut s),
            vp(&mut e),
            GPtr::from_raw(cp.as_mut_ptr()).cast(),
            GPtr::from_raw(&mut ent as *mut u32).cast(),
        ) {
            let z = x_(f32::from_bits(cp[2])).fadd_m(rd::<f32>(heli, 0x9B0));
            let cur = rd::<f32>(heli, 0x9AC);
            let v = if ge5(z, x_(cur)) { z } else { x_(cur) };
            wr::<f32>(heli, 0x9AC, v.to_f32());
        }
    }
    let m = rd::<GPtr<u8>>(heli, 0x14);
    let h = atan_xy(rd::<f32>(m, 0x10), rd::<f32>(m, 0x14)).to_f32(); // [S+8]
    // the lift
    let p = entity_pos(heli);
    let e = x_(rd::<f32>(heli, 0x4C)).fmul_m(100.0).fadd_m(vx(p, 2));
    wr::<u32>(heli, 0x998, 0);
    let e = e.fsubr_m(rd::<f32>(heli, 0x9AC));
    let a = if le41(e, x_(0.0)) {
        e.fmul_m(k(0x3E4C_CCCD))
    } else {
        e.fmul_m(k(0x3DCC_CCCD))
    };
    wr::<f32>(heli, 0x998, a.to_f32());
    let r = crate::common::rand_at(0x423AF8) & 0xF;
    let v = xi_(r)
        .fsub_m(7.0)
        .fmul_m(k(0x3B03_126F))
        .fadd_m(rd::<f32>(heli, 0x998));
    wr::<f32>(heli, 0x998, v.to_f32());
    wr::<f32>(heli, 0x998, clamp_unit(v, 0xBE99_999A, 0x3F80_0000).to_f32());
    // the yaw
    let y = wrap_up(wrap_down(x_(orientation).fsub_m(h))).fmul_m(-0.5);
    wr::<f32>(heli, 0x98C, y.to_f32());
    wr::<f32>(heli, 0x98C, clamp_unit(y, 0xBF80_0000, 0x3F80_0000).to_f32());
    // roll and pitch from the lateral / forward errors
    let m = rd::<GPtr<u8>>(heli, 0x14);
    let p = entity_pos(heli);
    let dx = x_(tx).fsub_m(vx(p, 0));
    let dy = x_(ty).fsub_m(vx(p, 1));
    let (mvx, mvy, mvz) = (
        rd::<f32>(heli, 0x44),
        rd::<f32>(heli, 0x48),
        rd::<f32>(heli, 0x4C),
    );
    let s_r = x_(mvz)
        .fmul_m(rd::<f32>(m, 8))
        .fadd(x_(mvy).fmul_m(rd::<f32>(m, 4)))
        .fadd(x_(rd::<f32>(m, 0)).fmul_m(mvx))
        .to_f32(); // [S+0x6C]
    let s_f = x_(mvz)
        .fmul_m(rd::<f32>(m, 0x18))
        .fadd(x_(mvy).fmul_m(rd::<f32>(m, 0x14)))
        .fadd(x_(mvx).fmul_m(rd::<f32>(m, 0x10)))
        .to_f32(); // [S+0x74]
    let lat = dy
        .fmul_m(rd::<f32>(m, 4))
        .fadd(dx.fmul_m(rd::<f32>(m, 0)))
        .fadd(x_(rd::<f32>(m, 8)).fmul_m(0.0))
        .fadd(x_(s_r).fmul_m(80.0))
        .to_f32(); // [S+0x70]
    let fe = dy
        .fmul_m(rd::<f32>(m, 0x14))
        .fadd(dx.fmul_m(rd::<f32>(m, 0x10)))
        .fadd(x_(rd::<f32>(m, 0x18)).fmul_m(0.0))
        .fadd(x_(s_f).fmul_m(80.0));
    let abs5 = |v: X87| if ge5(v, x_(0.0)) { v } else { v.fchs() };
    if ge5(abs5(x_(lat)), x_(5.0)) {
        let c = X87::from_f64_exact(f64::from_bits(0xBF94_7AE1_47AE_147B)); // -0.02
        wr::<f32>(heli, 0x994, x_(lat).fmul(c).to_f32());
    } else {
        wr::<f32>(heli, 0x994, s_r);
    }
    let roll = x_(rd::<f32>(heli, 0x994));
    let roll = if ge5(roll, x_(0.75)) {
        x_(0.75)
    } else if le41(x_(-0.75), roll) {
        roll
    } else {
        x_(-0.75)
    };
    wr::<f32>(heli, 0x994, roll.to_f32());
    if ge5(abs5(fe), x_(5.0)) {
        let c = X87::from_f64_exact(f64::from_bits(0xBF8E_B851_EB85_1EB8)); // -0.015
        wr::<f32>(heli, 0x990, fe.fmul(c).to_f32());
    } else {
        wr::<f32>(heli, 0x990, s_f);
    }
    let v = x_(rd::<f32>(heli, 0x990));
    if ge5(v, x_(0.5)) {
        wr::<f32>(heli, 0x990, 0.5);
    } else if !le41(x_(-0.5), v) {
        wr::<f32>(heli, 0x990, -0.5);
    }
}

// ---------------------------------------------------------------------------------------------------------
// Parking

orig! {
    /// `CPathNode::GetPosition(CVector*)` (0x420A10, thiscall, returns `out`)
    fn get_position(node: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x420A10 => crate::pathfind::get_position;
    /// `CCarCtrl::SteerAICarWithPhysicsHeadingForTarget(CVehicle*, CPhysical*, float x, float y, float*, float*, float*, bool*)` (0x433280)
    #[allow(clippy::too_many_arguments)]
    fn steer_heading_for_target(v: GPtr<u8>, target: GPtr<u8>, x: f32, y: f32, steer: GPtr<f32>, gas: GPtr<f32>, brake: GPtr<f32>, hb: GPtr<u8>) = cdecl 0x433280 => crate::carctrl3::steer_ai_car_with_physics_heading_for_target;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskComplexLeaveAnyCar::CTaskComplexLeaveAnyCar(int32 delay, bool sensible, bool forceGetOut)` (stub 0x421150, thiscall, returns `this`)
    fn task_leave_any_car_ctor(t: GPtr<u8>, delay: i32, sensible: u32, force: u32) -> GPtr<u8> = thiscall 0x421150 => unported;
    /// `CTaskComplex::CTaskComplex()` (0x61A3B0, thiscall)
    fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CTaskManager::SetTask(CTask*, int32 slot, bool)` (0x681AF0, thiscall)
    fn task_manager_set_task(m: GPtr<u8>, t: GPtr<u8>, slot: i32, b: u32) = thiscall 0x681AF0 => unported;
    /// `CCollision::DistToMathematicalLine(const CVector*, const CVector*, const CVector*)` (0x412970, ST0)
    fn dist_to_mathematical_line(a: GPtr<f32>, b: GPtr<f32>, p: GPtr<f32>) -> f64 = cdecl 0x412970 => dist_to_mathematical_line_abi;
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn dist_to_mathematical_line_abi(a: GPtr<f32>, b: GPtr<f32>, p: GPtr<f32>) -> f64 {
    sa_world::collision::prim::dist_to_mathematical_line(a, b, p).to_f64()
}

/// `ThePaths.m_pPathNodes[area]` (0x96F854).
fn path_nodes(area: u32) -> GPtr<u8> {
    crate::common::g::<GPtr<u8>>((0x96F854 + area * 4) as usize).read()
}

/// The `CPathNode` (0x1C bytes) of a `CNodeAddress` dword (area low word, node high word).
fn path_node(a: u32) -> GPtr<u8> {
    path_nodes(a & 0xFFFF).byte_add(((a >> 16) * 0x1C) as isize)
}

/// `CPathNode::GetPosition` into a fresh vector.
fn node_pos(node: GPtr<u8>) -> [f32; 3] {
    let mut v = [0f32; 3];
    let r = get_position(node, vp(&mut v));
    [vx(r, 0), vx(r, 1), vx(r, 2)]
}

/// The parking stop shared by both park steerings: every occupant gets `CTaskComplexLeaveAnyCar(0, true,
/// false)` in the primary slot 3. The driver's task is built by the constructor (stub 0x421150); the
/// passengers' (slots +0x464.. in order) inline it: `CTaskComplex` constructor, vtable 0x858EB0, +0xC = 0,
/// +0x10 = 1, +0x11 = 0. A null `new` passes null to `SetTask`; the driver / passenger pointers are read
/// again for `SetTask` as the bytes do.
fn occupants_leave_car(veh: GPtr<u8>) {
    if !rd::<GPtr<u8>>(veh, 0x460).is_null() {
        let t = task_new(0x14);
        let t = if t.is_null() {
            GPtr::NULL
        } else {
            task_leave_any_car_ctor(t, 0, 1, 0)
        };
        let ped = rd::<GPtr<u8>>(veh, 0x460);
        task_manager_set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 3, 0);
    }
    for i in 0..8 {
        let off = 0x464 + 4 * i;
        if rd::<GPtr<u8>>(veh, off).is_null() {
            continue;
        }
        let t = task_new(0x14);
        let t = if t.is_null() {
            GPtr::NULL
        } else {
            task_complex_ctor(t);
            wr::<u32>(t, 0, 0x0085_8EB0);
            wr::<u32>(t, 0xC, 0);
            wr::<u8>(t, 0x10, 1);
            wr::<u8>(t, 0x11, 0);
            t
        };
        let ped = rd::<GPtr<u8>>(veh, off);
        task_manager_set_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), t, 3, 0);
    }
}

/// The invalid-area exit of both park steerings: mission 0xB (stop forever), `*brake`, `*gas`, `*steer`
/// = 0 in that order (the handbrake untouched).
fn park_invalid(veh: GPtr<u8>, steer: GPtr<f32>, gas: GPtr<f32>, brake: GPtr<f32>) {
    wr::<u8>(veh, 0x3BA, 0xB);
    brake.write(0.0);
    gas.write(0.0);
    steer.write(0.0);
}

/// `|T.xy - P.xy|` as both park steerings compute it: `dy = T.y - P.y`, `dx = T.x - P.x` (x87),
/// `sqrt(dx * dx + dy * dy)`.
fn dist_2d_to(tx: f32, ty: f32, p: GPtr<f32>) -> X87 {
    let dy = x_(ty).fsub_m(vx(p, 1));
    let dx = x_(tx).fsub_m(vx(p, 0));
    dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt()
}

/// `CCarCtrl::SteerAICarParkParallel(CVehicle* veh, float* steer, float* gas, float* brake, bool*
/// handbrake)` (0x433BA0, cdecl). Either link area (+0x394 next tested first, then +0x390) unloaded →
/// mission 0xB and brake / gas / steer 0. Target: mission 0x2E → `Normalise(next - cur) + next` (1 m past
/// the next node, f32 stores), otherwise the next node. `HeadingForTarget(veh, null, T.x, T.y, ...)`,
/// cruise `min(cruise, 8)`. Then `|T.xy - P.xy|`: mission 0x2E below 4 → mission 0x32; other missions below
/// 2 → +0x428 `&= 0xAF` (engine and lights off), mission 0xB and every occupant leaves the car
/// ([`occupants_leave_car`]). `>=` / NaN do nothing (`jp`).
///
/// The SEH frame only frees the task when its constructor throws (no behaviour). The original stores the
/// node addresses into its `veh` argument slot; the only caller (0x43798E) pops it unread.
#[hook(addr = 0x433BA0, conv = cdecl, name = "CCarCtrl::SteerAICarParkParallel", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "popw4-drive-1", route = "L2", hazards = "heap-writes,x87-transcendental",
       recs = "car-ai,driving", note = "allocates CTaskComplexLeaveAnyCar for every occupant at the stop")]
pub fn steer_ai_car_park_parallel(
    veh: GPtr<u8>,
    steer: GPtr<f32>,
    gas: GPtr<f32>,
    brake: GPtr<f32>,
    hb: GPtr<u8>,
) {
    let next_a = rd::<u32>(veh, 0x394);
    if path_nodes(next_a & 0xFFFF).is_null() {
        return park_invalid(veh, steer, gas, brake);
    }
    let cur_a = rd::<u32>(veh, 0x390);
    if path_nodes(cur_a & 0xFFFF).is_null() {
        return park_invalid(veh, steer, gas, brake);
    }
    let next = path_node(next_a);
    let cur = path_node(cur_a);
    let t = if rd::<u8>(veh, 0x3BA) == 0x2E {
        let c = node_pos(cur);
        let n = node_pos(next);
        let mut d = [
            x_(n[0]).fsub_m(c[0]).to_f32(),
            x_(n[1]).fsub_m(c[1]).to_f32(),
            x_(n[2]).fsub_m(c[2]).to_f32(),
        ];
        vector_normalise(vp(&mut d));
        let n = node_pos(next);
        [
            x_(d[0]).fadd_m(n[0]).to_f32(),
            x_(d[1]).fadd_m(n[1]).to_f32(),
            x_(d[2]).fadd_m(n[2]).to_f32(),
        ]
    } else {
        node_pos(next)
    };
    steer_heading_for_target(veh, GPtr::NULL, t[0], t[1], steer, gas, brake, hb);
    let c = rd::<u8>(veh, 0x3D0);
    wr::<u8>(veh, 0x3D0, c.min(8));
    let dist = dist_2d_to(t[0], t[1], entity_pos(veh));
    if rd::<u8>(veh, 0x3BA) == 0x2E {
        if !ge5(dist, x_(4.0)) {
            wr::<u8>(veh, 0x3BA, 0x32);
        }
        return;
    }
    if ge5(dist, x_(2.0)) {
        return;
    }
    let f = rd::<u8>(veh, 0x428) & 0xAF;
    wr::<u8>(veh, 0x3BA, 0xB);
    wr::<u8>(veh, 0x428, f);
    occupants_leave_car(veh);
}

/// `CCarCtrl::SteerAICarParkPerpendicular(CVehicle* veh, float* steer, float* gas, float* brake, bool*
/// handbrake)` (0x433EA0, cdecl). The invalid-area exit as [`steer_ai_car_park_parallel`]. Mission 0x2D:
/// `DistToMathematicalLine(&(next.xy, 0), &(cur.xy, 0), &(P.xy, 0)) < 6` → mission 0x31; the target is the
/// current node while the mission is still 0x2D, otherwise (and for every other mission) the next node.
/// `HeadingForTarget(veh, null, T.x, T.y, ...)`, cruise `min(cruise, 8)`. Then, whatever the mission,
/// `|next.xy - P.xy| < 2` (the next node read again): +0x3DC `|= 2` (bIsParked), +0x428 `&= 0xAF`, mission
/// 0xB and every occupant leaves the car ([`occupants_leave_car`]). The node addresses and the node y
/// values pass through the `veh` argument slot in the original; the only caller (0x43795A) pops it unread.
#[hook(addr = 0x433EA0, conv = cdecl, name = "CCarCtrl::SteerAICarParkPerpendicular", expect = "64 A1 00 00 00 00 6A FF",
       unverified = "popw4-drive-1", route = "L2", hazards = "heap-writes,x87-transcendental",
       recs = "car-ai,driving", note = "allocates CTaskComplexLeaveAnyCar for every occupant at the stop")]
pub fn steer_ai_car_park_perpendicular(
    veh: GPtr<u8>,
    steer: GPtr<f32>,
    gas: GPtr<f32>,
    brake: GPtr<f32>,
    hb: GPtr<u8>,
) {
    let next_a = rd::<u32>(veh, 0x394);
    if path_nodes(next_a & 0xFFFF).is_null() {
        return park_invalid(veh, steer, gas, brake);
    }
    let cur_a = rd::<u32>(veh, 0x390);
    if path_nodes(cur_a & 0xFFFF).is_null() {
        return park_invalid(veh, steer, gas, brake);
    }
    let next = path_node(next_a);
    let cur = path_node(cur_a);
    let t = if rd::<u8>(veh, 0x3BA) == 0x2D {
        let ny = node_pos(next)[1];
        let nx = node_pos(next)[0];
        let mut a = [nx, ny, 0.0];
        let cy = node_pos(cur)[1];
        let cx = node_pos(cur)[0];
        let mut b = [cx, cy, 0.0];
        let p = entity_pos(veh);
        let mut v = [vx(p, 0), vx(p, 1), 0.0];
        let d = X87::from_f64_exact(dist_to_mathematical_line(vp(&mut a), vp(&mut b), vp(&mut v)));
        if !ge5(d, x_(6.0)) {
            wr::<u8>(veh, 0x3BA, 0x31);
        }
        if rd::<u8>(veh, 0x3BA) == 0x2D {
            node_pos(cur)
        } else {
            node_pos(next)
        }
    } else {
        node_pos(next)
    };
    steer_heading_for_target(veh, GPtr::NULL, t[0], t[1], steer, gas, brake, hb);
    let c = rd::<u8>(veh, 0x3D0);
    wr::<u8>(veh, 0x3D0, c.min(8));
    let p = entity_pos(veh);
    let n = node_pos(next);
    if ge5(dist_2d_to(n[0], n[1], p), x_(2.0)) {
        return;
    }
    let mf = rd::<u8>(veh, 0x3DC) | 2;
    let f = rd::<u8>(veh, 0x428) & 0xAF;
    wr::<u8>(veh, 0x3BA, 0xB);
    wr::<u8>(veh, 0x3DC, mf);
    wr::<u8>(veh, 0x428, f);
    occupants_leave_car(veh);
}

// ---------------------------------------------------------------------------------------------------------
// Steering along a vehicle recording

orig! {
    /// `CCarCtrl::FindAngleToWeaveThroughTraffic(CVehicle*, CPhysical*, float, float, float)` (0x4325C0, ST0)
    fn find_angle_to_weave_call(veh: GPtr<u8>, ignore: GPtr<u8>, heading: f32, target: f32, range: f32) -> f64 = cdecl 0x4325C0 => find_angle_to_weave_abi;
    /// `CCarCtrl::FindMaxSteerAngle(CVehicle*)` (0x427FE0, ST0)
    fn find_max_steer_angle(v: GPtr<u8>) -> f64 = cdecl 0x427FE0 => find_max_steer_angle_abi;
    /// `CVehicleRecording::StopPlaybackWithIndex(int32)` (0x459440)
    fn stop_playback_with_index(i: i32) = cdecl 0x459440 => unported;
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn find_angle_to_weave_abi(veh: GPtr<u8>, ignore: GPtr<u8>, heading: f32, target: f32, range: f32) -> f64 {
    crate::carctrl3::find_angle_to_weave_through_traffic(veh, ignore, heading, target, range).to_f64()
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn find_max_steer_angle_abi(v: GPtr<u8>) -> f64 {
    crate::carctrl2::find_max_steer_angle(v).to_f64()
}

/// `|V - p|` (3D) as the recording steering computes it: `dx = V.x - p.x`, `dy`, `dz` on the FPU,
/// `sqrt((dz * dz + dy * dy) + dx * dx)`.
fn dist_3d_from(v: GPtr<f32>, p: [f32; 3]) -> X87 {
    let dx = x_(vx(v, 0)).fsub_m(p[0]);
    let dy = x_(vx(v, 1)).fsub_m(p[1]);
    let dz = x_(vx(v, 2)).fsub_m(p[2]);
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt()
}

/// The position (+0x14) of a 0x20-byte `CVehicleStateEachFrame` record.
fn rec_pos(f: GPtr<u8>) -> [f32; 3] {
    [rd::<f32>(f, 0x14), rd::<f32>(f, 0x18), rd::<f32>(f, 0x1C)]
}

/// `CCarCtrl::SteerAICarWithPhysicsFollowPreRecordedPath(CVehicle* veh, float* steer, float* gas, float*
/// brake, bool* handbrake)` (0x432DD0, cdecl). `rid = (int8)+0x424`:
/// - `rid < 0` or no playback buffer (0x97D800): brake, gas, steer 0, handbrake 0, mission 0xB.
/// - paused (0x97D6D0): gas 0, steer 0, brake 0.5, handbrake 0 (mission kept).
/// - Otherwise `F` = the frame at `PlaybackIndex[rid]` (0x97D7C0) on entry. While `|V - pos(frame)| < 10`
///   or `|V - pos(frame + 1)| < |V - pos(frame)|` (x87 values, `test ah, 5; jnp`), the index advances by
///   0x20; reaching `PlaybackBufferSize[rid] - 0x20` (0x97D780, unsigned) stops the playback
///   (`StopPlaybackWithIndex`), +0x424 = -1, brake / gas / steer 0, handbrake 0, mission 0xB. `F` itself
///   stays the entry frame (the original's one-frame lag, kept).
/// - Steer: `FindAngleToWeaveThroughTraffic(veh, null, atan(F.pos - V), atan(fwd), 2) - atan(fwd)` wrapped
///   (`< -pi` first, f32 store) and clamped to ±`FindMaxSteerAngle` (`jp` / `jne 0x41`: NaN kept).
/// - Target speed `|F.vel| / 16383 * PlaybackSpeed[rid] * 60` (0x97D700), at least 5 (`<= 5` / NaN → 5)
///   while the (advanced) index is at most 0x320. Own speed `|mv.xy| * 60` (f32). `diff > 0`: gas
///   `min(diff * (own < 2 ? 0.25 : 0.125), 1)`; otherwise gas 0, brake `min(diff * -0.05, 0.5)` (NaN
///   kept). Brake is first cleared to 0; handbrake 0.
///
/// The original keeps rid, the heading and the steer in its `veh` argument slot; the only caller
/// (0x437540) pops it unread.
#[hook(addr = 0x432DD0, conv = cdecl, name = "CCarCtrl::SteerAICarWithPhysicsFollowPreRecordedPath", expect = "83 EC 1C 56 8B 74 24 24",
       unverified = "popw4-drive-1", route = "L2", hazards = "heap-writes,x87-transcendental",
       recs = "car-ai,driving", note = "writes CVehicleRecording::PlaybackIndex[rid] (0x97D7C0)")]
pub fn steer_ai_car_with_physics_follow_pre_recorded_path(
    veh: GPtr<u8>,
    steer: GPtr<f32>,
    gas: GPtr<f32>,
    brake: GPtr<f32>,
    hb: GPtr<u8>,
) {
    let rid = i32::from(rd::<i8>(veh, 0x424));
    if rid < 0 {
        brake.write(0.0);
        gas.write(0.0);
        steer.write(0.0);
        hb.write(0);
        wr::<u8>(veh, 0x3BA, 0xB);
        return;
    }
    let ri = rid as usize;
    let buf = crate::common::g::<GPtr<u8>>(0x97D800 + ri * 4).read();
    if buf.is_null() {
        brake.write(0.0);
        gas.write(0.0);
        steer.write(0.0);
        hb.write(0);
        wr::<u8>(veh, 0x3BA, 0xB);
        return;
    }
    if crate::common::g::<u8>(0x97D6D0 + ri).read() != 0 {
        gas.write(0.0);
        steer.write(0.0);
        brake.write(0.5);
        hb.write(0);
        return;
    }
    let idx_p = crate::common::g::<u32>(0x97D7C0 + ri * 4);
    let f = buf.byte_add(idx_p.read() as isize);
    let mut d0 = dist_3d_from(entity_pos(veh), rec_pos(f));
    let mut d1 = dist_3d_from(entity_pos(veh), rec_pos(f.byte_add(0x20)));
    while !ge5(d0, x_(10.0)) || !ge5(d1, d0) {
        let idx = idx_p.read().wrapping_add(0x20);
        let lim = crate::common::g::<u32>(0x97D780 + ri * 4)
            .read()
            .wrapping_sub(0x20);
        idx_p.write(idx);
        if idx >= lim {
            stop_playback_with_index(rid);
            wr::<i8>(veh, 0x424, -1);
            brake.write(0.0);
            gas.write(0.0);
            steer.write(0.0);
            hb.write(0);
            wr::<u8>(veh, 0x3BA, 0xB);
            return;
        }
        let nf = buf.byte_add(idx as isize);
        d0 = dist_3d_from(entity_pos(veh), rec_pos(nf));
        d1 = dist_3d_from(entity_pos(veh), rec_pos(nf.byte_add(0x20)));
    }
    // the steer
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let h = atan_xy(rd::<f32>(m, 0x10), rd::<f32>(m, 0x14)).to_f32();
    let p = entity_pos(veh);
    let ty = x_(rd::<f32>(f, 0x18)).fsub_m(vx(p, 1)).to_f32();
    let tx = x_(rd::<f32>(f, 0x14)).fsub_m(vx(p, 0)).to_f32();
    let at = atan_xy(tx, ty).to_f32();
    let w = X87::from_f64_exact(find_angle_to_weave_call(veh, GPtr::NULL, at, h, 2.0)).fsub_m(h);
    let mut s = wrap_down(wrap_up(w)).to_f32();
    let ms = X87::from_f64_exact(find_max_steer_angle(veh));
    let nms = ms.fchs();
    if !ge5(x_(s), nms) {
        s = nms.to_f32();
    }
    if !le41(x_(s), ms) {
        s = ms.to_f32();
    }
    // the speeds
    let k16 = k(0x3880_0100); // 1/16383
    let vy = xi_(i32::from(rd::<i16>(f, 6))).fmul_m(k16);
    let vxr = xi_(i32::from(rd::<i16>(f, 4))).fmul_m(k16);
    let speed = crate::common::g::<f32>(0x97D700 + ri * 4).read();
    let mut target = vxr.fmul(vxr).fadd(vy.fmul(vy)).fsqrt().fmul_m(speed).fmul_m(60.0);
    if idx_p.read() <= 0x320 && le41(target, x_(5.0)) {
        target = x_(5.0);
    }
    let (my, mx) = (rd::<f32>(veh, 0x48), rd::<f32>(veh, 0x44));
    brake.write(0.0);
    let own = x_(mx)
        .fmul(x_(mx))
        .fadd(x_(my).fmul(x_(my)))
        .fsqrt()
        .fmul_m(60.0)
        .to_f32();
    let diff = target.fsub_m(own);
    if le41(diff, x_(0.0)) {
        let b = diff.fmul_m(k(0xBD4C_CCCD));
        gas.write(0.0);
        brake.write(if ge5(x_(0.5), b) { b.to_f32() } else { 0.5 });
    } else {
        let g = diff.fmul_m(if ge5(x_(own), x_(2.0)) { 0.125 } else { 0.25 });
        gas.write(if ge5(x_(1.0), g) { g.to_f32() } else { 1.0 });
    }
    steer.write(s);
    hb.write(0);
}

// ---------------------------------------------------------------------------------------------------------
// Cars on rails dragged to a point

orig! {
    /// `CCollision::ProcessLineSphere(const CColLine&, const CColSphere&, CColPoint&, float& depth)` (0x412AA0)
    fn process_line_sphere(line: GPtr<u8>, sphere: GPtr<u8>, cp: GPtr<u8>, depth: GPtr<f32>) -> u8 = cdecl 0x412AA0 => unported;
    /// `CCollision::IsStoredPolyStillValidVerticalLine(const CVector&, float zEnd, CColPoint&, CStoredCollPoly*)` (0x414D70)
    fn is_stored_poly_still_valid_vertical_line(p: GPtr<f32>, z: f32, cp: GPtr<u8>, poly: GPtr<u8>) -> u8 = cdecl 0x414D70 => unported;
    /// `CWorld::ProcessVerticalLine(const CVector&, float zEnd, CColPoint&, CEntity*&, buildings, vehicles, peds,
    /// objects, dummies, seeThrough, CStoredCollPoly*)` (0x5674E0)
    fn process_vertical_line(p: GPtr<f32>, z: f32, cp: GPtr<u8>, ent: GPtr<u8>, b: u32, v: u32, pe: u32, o: u32, d: u32, st: u32, poly: GPtr<u8>) -> u8 = cdecl 0x5674E0 => unported;
    /// `CVector::CrossProduct(CVector* out, const CVector* a, const CVector* b)` (0x59C730, `a x b`, returns `out`)
    fn cross_product(out: GPtr<f32>, a: GPtr<f32>, b: GPtr<f32>) -> GPtr<f32> = cdecl 0x59C730 => unported;
    /// `CVehicle` vtable slot 0xD4 `GetHeightAboveRoad()` (ST0)
    fn vehicle_get_height_above_road(v: GPtr<u8>) -> f64 = thiscall vslot 0xD4 => unported;
    /// `CVehicle::UpdateLightingFromStoredPolys()` (0x6D0CC0, thiscall)
    fn update_lighting_from_stored_polys(v: GPtr<u8>) = thiscall 0x6D0CC0 => unported;
}

/// One ground probe of [`drag_car_to_point`] at `(x, y)` around the height `z0` (`zx` = the x87 value the
/// low end is computed from, `z0` its f32 store): the stored poly at `poly` (`IsStoredPolyStillValid
/// VerticalLine((x, y, z0 + k2), (zx - k1))`), then `ProcessVerticalLine((x, y, z0 + 1.5), z0 - 2)`, then
/// `ProcessVerticalLine((x, y, z0 + k2), zx - k1)` (buildings only, the stored poly). A ProcessVerticalLine
/// hit stores the entity in +0x48C, copies its +0x1C bits 30 and 31 and clears the counter +0x3E1. Returns
/// the hit's z, or `None` when all three miss.
#[allow(clippy::too_many_arguments)]
fn drag_ground_probe(
    veh: GPtr<u8>,
    x: f32,
    y: f32,
    zx: X87,
    z0: f32,
    k1: f32,
    k2: f32,
    poly: GPtr<u8>,
) -> Option<f32> {
    let mut cp = [0u32; 11]; // [T+0x84]
    let cpp: GPtr<u8> = GPtr::from_raw(cp.as_mut_ptr()).cast();
    let mut ent = 0u32; // [T+0x34]
    let entp: GPtr<u8> = GPtr::from_raw(&mut ent as *mut u32).cast();
    let z_low = zx.fsub_m(k1).to_f32(); // [T+0x60]
    let z_high = x_(z0).fadd_m(k2).to_f32(); // [T+0x54]
    let mut v = [x, y, z_high]; // [T+0x28]
    if is_stored_poly_still_valid_vertical_line(vp(&mut v), z_low, cpp, poly) != 0 {
        return Some(f32::from_bits(cp[2]));
    }
    v[2] = x_(z0).fadd_m(k(0x3FC0_0000)).to_f32();
    let z_end = x_(z0).fsub_m(2.0).to_f32();
    let mut hit = process_vertical_line(vp(&mut v), z_end, cpp, entp, 1, 0, 0, 0, 0, 0, poly) != 0;
    if !hit {
        v[2] = z_high;
        hit = process_vertical_line(vp(&mut v), z_low, cpp, entp, 1, 0, 0, 0, 0, 0, poly) != 0;
    }
    if !hit {
        return None;
    }
    let e: GPtr<u8> = GPtr::from_addr(ent as usize);
    wr::<u32>(veh, 0x48C, ent);
    let vf = rd::<u32>(veh, 0x1C);
    let vf = vf ^ ((vf ^ rd::<u32>(e, 0x1C)) & 0x4000_0000);
    wr::<u32>(veh, 0x1C, vf);
    let ef = rd::<u32>(e, 0x1C);
    wr::<u32>(veh, 0x1C, ((ef ^ vf) & 0x7FFF_FFFF) ^ ef);
    wr::<u8>(veh, 0x3E1, 0);
    Some(f32::from_bits(cp[2]))
}

/// `CCarCtrl::DragCarToPoint(CVehicle* veh, CVector* target)` (0x42EC90, cdecl): places a car on rails
/// between `target.xy` (the front) and a rear anchor, on the ground.
/// 1. `L = (bbox.max.y - bbox.min.y) * 0.95` (f32 store, the x87 value kept for the anchor); `k1 = k2 = 3`,
///    100 when the counter +0x3E1 (int8) exceeds 16; `F` = a copy of the forward vector.
/// 2. Rear anchor `r = P.xy - F.xy * L * 0.5`. If the segment `r .. r + F.xy * 1.5` (z 0) hits the sphere
///    around `(target.xy, 0)` of radius `L` (`ProcessLineSphere`, depth 1): `r` = the hit point; otherwise
///    `r = (r - t) * (L / |t - r|) + t` (the bytes divide the f32 `L`, not `k1` as the POP spec says).
/// 3. Front ground at `t` around `zf = P.z + F.z * L * 0.5` and rear ground at `r` around `zr = P.z - that
///    half` ([`drag_ground_probe`], the counter +0x3E1 incremented before the front probe, stored polys
///    +0x51C / +0x548); a total miss keeps the old +0x4E4 / +0x4E8. Both stored back.
/// 4. `pitch = fpatan((fz - rz) / L, 1)`; right = `((t.y - r.y) / L, (t.x - r.x) * (-1 / L), 0)`; forward
///    = `(-(cos * right.y), cos * right.x, sin)`; up = `right x forward` (CrossProduct).
/// 5. Position `((t + r) / 2, (rz + fz) / 2 + GetHeightAboveRoad())` (vtable +0xD4), after
///    `UpdateLightingFromStoredPolys`; into the matrix or the placement.
/// 6. `+0x494 = clamp((newFwd x F).z * -10, -0.5, 0.5)` (NaN → 0.5).
#[hook(addr = 0x42EC90, conv = cdecl, name = "CCarCtrl::DragCarToPoint", expect = "81 EC 10 01 00 00 56 8B",
       unverified = "popw4-drive-1", route = "L2", hazards = "heap-writes,x87-transcendental",
       recs = "car-ai,driving", note = "writes the vehicle matrix (heap) and the stored coll polys +0x51C/+0x548")]
pub fn drag_car_to_point(veh: GPtr<u8>, target: GPtr<f32>) {
    let cm = rd::<GPtr<u8>>(crate::common::model_info(i32::from(rd::<i16>(veh, 0x22))), 0x14);
    let (mut k1, mut k2) = (3.0f32, 3.0f32);
    let lx = x_(rd::<f32>(cm, 0x10))
        .fsub_m(rd::<f32>(cm, 4))
        .fmul_m(k(0x3F73_3333));
    let lf = lx.to_f32(); // [T+0x50]
    if rd::<i8>(veh, 0x3E1) > 16 {
        k2 = 100.0;
        k1 = 100.0;
    }
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let f = [rd::<f32>(m, 0x10), rd::<f32>(m, 0x14), rd::<f32>(m, 0x18)]; // [T+0x44], [T+0x78]
    let p = entity_pos(veh);
    let mut rx = x_(f[0]).fmul(lx).fmul_m(0.5).fsubr_m(vx(p, 0)).to_f32(); // [T+0x18]
    let mut ry = x_(f[1]).fmul(lx).fmul_m(0.5).fsubr_m(vx(p, 1)).to_f32(); // [T+0x1C]
    let tx = vx(target, 0); // [T+0xC]
    let ty = vx(target, 1); // [T+0x10]
    let fy15 = x_(f[1]).fmul_m(k(0x3FC0_0000)).to_f32();
    let mut line = [0f32; 8]; // CColLine [T+0xC4]: start, pad, end, pad
    line[0] = rx;
    line[1] = ry;
    line[4] = x_(f[0]).fmul_m(k(0x3FC0_0000)).fadd_m(rx).to_f32();
    line[5] = x_(fy15).fadd_m(ry).to_f32();
    // CColSphere [T+0xB0] (centre, radius; the surface bytes +0x10 are uninitialised in the original and
    // only reach the unused surface fields of the hit point)
    let mut sphere = [tx, ty, 0.0, lf, 0.0];
    let mut cp = [0u32; 11]; // [T+0xF0]
    let mut depth = 1.0f32; // [T+0x68]
    if process_line_sphere(
        GPtr::from_raw(line.as_mut_ptr()).cast(),
        GPtr::from_raw(sphere.as_mut_ptr()).cast(),
        GPtr::from_raw(cp.as_mut_ptr()).cast(),
        GPtr::from_raw(&mut depth as *mut f32),
    ) != 0
    {
        rx = f32::from_bits(cp[0]);
        ry = f32::from_bits(cp[1]);
    } else {
        let dy = x_(ty).fsub_m(ry);
        let dx = x_(tx).fsub_m(rx);
        let kk = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt().fdivr_m(lf);
        rx = x_(rx).fsub_m(tx).fmul(kk).fadd_m(tx).to_f32();
        ry = x_(ry).fsub_m(ty).fmul(kk).fadd_m(ty).to_f32();
    }
    // the front ground
    let p = entity_pos(veh);
    let half = x_(f[2]).fmul_m(lf).fmul_m(0.5);
    let half_f = half.to_f32(); // [T+0x64]
    let zf = half.fadd_m(vx(p, 2));
    let c = rd::<u8>(veh, 0x3E1).wrapping_add(1);
    wr::<u8>(veh, 0x3E1, c);
    let fz = match drag_ground_probe(veh, tx, ty, zf, zf.to_f32(), k1, k2, veh.byte_add(0x51C)) {
        Some(z) => z,
        None => rd::<f32>(veh, 0x4E4),
    }; // [T+0x14]
    wr::<f32>(veh, 0x4E4, fz);
    // the rear ground
    let p = entity_pos(veh);
    let zr = x_(vx(p, 2)).fsub_m(half_f);
    let rz = match drag_ground_probe(veh, rx, ry, zr, zr.to_f32(), k1, k2, veh.byte_add(0x548)) {
        Some(z) => z,
        None => rd::<f32>(veh, 0x4E8),
    }; // [T+0x20]
    // the orientation
    let inv_l = x_(1.0).fdiv_m(lf);
    let m = rd::<GPtr<u8>>(veh, 0x14);
    wr::<f32>(veh, 0x4E8, rz);
    let pitch = x_(fz).fsub_m(rz).fmul(inv_l).fpatan(x_(1.0));
    let cos = pitch.fcos();
    wr::<f32>(m, 0, x_(ty).fsub_m(ry).fmul(inv_l).to_f32());
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let nl = x_(-1.0).fdiv_m(lf);
    wr::<f32>(m, 4, x_(tx).fsub_m(rx).fmul(nl).to_f32());
    let m = rd::<GPtr<u8>>(veh, 0x14);
    wr::<u32>(m, 8, 0);
    let m = rd::<GPtr<u8>>(veh, 0x14);
    wr::<f32>(m, 0x10, cos.fmul_m(rd::<f32>(m, 4)).fchs().to_f32());
    let m = rd::<GPtr<u8>>(veh, 0x14);
    wr::<f32>(m, 0x14, cos.fmul_m(rd::<f32>(m, 0)).to_f32());
    let m = rd::<GPtr<u8>>(veh, 0x14);
    wr::<f32>(m, 0x18, pitch.fsin().to_f32());
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let mut up = [0f32; 3]; // [T+0x6C]
    let u = cross_product(vp(&mut up), m.cast(), m.byte_add(0x10).cast());
    let (ux, uy, uz) = (vx(u, 0), vx(u, 1), vx(u, 2));
    let sx = x_(tx).fadd_m(rx);
    let sy = x_(ty).fadd_m(ry);
    let sz = x_(rz).fadd_m(fz).to_f32(); // [T+0x74]
    wr::<f32>(rd::<GPtr<u8>>(veh, 0x14), 0x20, ux);
    let px = sx.fmul_m(0.5).to_f32();
    wr::<f32>(rd::<GPtr<u8>>(veh, 0x14), 0x24, uy);
    let py = sy.fmul_m(0.5).to_f32();
    wr::<f32>(rd::<GPtr<u8>>(veh, 0x14), 0x28, uz);
    let pz0 = x_(sz).fmul_m(0.5).to_f32(); // [T+0x4C]
    let pz = X87::from_f64_exact(vehicle_get_height_above_road(veh))
        .fadd_m(pz0)
        .to_f32();
    update_lighting_from_stored_polys(veh);
    let m = rd::<GPtr<u8>>(veh, 0x14);
    if !m.is_null() {
        wr::<f32>(m, 0x30, px);
        wr::<f32>(rd::<GPtr<u8>>(veh, 0x14), 0x34, py);
        wr::<f32>(rd::<GPtr<u8>>(veh, 0x14), 0x38, pz);
    } else {
        wr::<f32>(veh, 4, px);
        wr::<f32>(veh, 8, py);
        wr::<f32>(veh, 0xC, pz);
    }
    // the steer
    let mut fc = f; // [T+0x78]
    let mut c = [0f32; 3]; // [T+0xE4]
    let m = rd::<GPtr<u8>>(veh, 0x14);
    cross_product(vp(&mut c), m.byte_add(0x10).cast(), vp(&mut fc));
    let s = x_(c[2]).fmul_m(-10.0);
    let v = if ge5(s, x_(0.5)) {
        x_(0.5)
    } else if le41(s, x_(-0.5)) {
        x_(-0.5)
    } else {
        s
    };
    wr::<f32>(veh, 0x494, v.to_f32());
}

// ---------------------------------------------------------------------------------------------------------
// Planes

orig! {
    /// `FUN_00422F80(CVehicle* plane, float heading)` (0x422F80, cdecl, ST0): the terrain probe of the plane
    /// route planner (the highest ground ahead along `heading`; 100000 when the first probe is blocked)
    fn plane_probe_heading(v: GPtr<u8>, heading: f32) -> f64 = cdecl 0x422F80 => unported;
    /// `CPlane::SetGearUp()` (0x6CAC20, thiscall)
    fn plane_set_gear_up(v: GPtr<u8>) = thiscall 0x6CAC20 => unported;
}

/// CRT `_CIasin` (0x821E70; operand and result in ST0) on an f32 operand: exact through
/// [`sa_core::orig::x87_unary`] under the game's 24-bit precision (see sa-weapon `bullet_shot.rs`).
fn ci_asin(v: f32) -> X87 {
    X87::from_f64_exact(sa_core::orig::x87_unary(0x821E70, f64::from(v)))
}

/// `CCarCtrl::FlyAIPlaneInCertainDirection(CPlane* plane)` (0x423000, cdecl). The catalogued entry is a
/// `jmp 0x401961`, a no-CD fragment that loads `CTimer::m_snPreviousTimeInMilliseconds` (0xB7CB78, through
/// the obfuscated `[0x1560698] + [0x1569C94]`) into EAX and jumps back to 0x423005; the body follows
/// in place (no `.HOODLUM` body, no VM). The hook treats the entry as an inline one (`expect` covers the
/// following `sub esp, 0x18`), so the original trampoline keeps the relocated `jmp` into the fragment.
///
/// A. Once per 1000 ms of the seed-shifted clock: with `h = atan(fwd)` (x87) the desired heading is
///    +0x9B4, or by temp action +0x3BB 16: `h`, 17: `h - 2`, 18: `2 + h` (f32); +0x9AC = 500, +0x9B8 =
///    +0x9B4 + pi; `maxDev = |wrap(h - desired)| < pi/6 ? 0 : diff * 1.5` (`> pi` wrapped first). Two passes
///    over the offsets `k * ±pi/12` (i = 1..19, k = i / 2, odd i positive): pass 0 the offsets strictly
///    between 0 and maxDev, pass 1 the others; the first `FUN_00422F80(plane, off + desired) < 150` of a
///    pass sets +0x9B8 = `off * 1.1 + +0x9B4` and +0x9AC = `max(r + +0x9B0, +0x9A8)` (strictly greater keeps
///    `r + min`) and ends that pass (pass 1 still runs).
/// B. Target speed 7 ([0x8A5B54]) for RC planes, else 32; `fs = (fwd.y * mv.y + fwd.x * mv.x) * 60` (f32).
///    Any wheel compression +0x7D4..+0x7E0 below 1 sets +0x9C0 = now. More than 4000 ms since +0x9C0: gear
///    up (unless +0x9CC is 1.0), `a = asin(fwd.z)`, pitch input +0x98C = `(clamp((+0x9AC - (mv.z * 100 +
///    P.z)) / 30, ±0.4) [min 0.25 below the target speed] - (a + (a - +0x9BC) * 100 / timestep)) * [0x8A5B50]`,
///    +0x9BC = a. Otherwise +0x98C = fs >= target ? 0.4 : 0. Yaw +0x988 from `wrap(+0x9B8 - atan(fwd))`
///    (`< -pi` first): early (the 4000 ms branch not taken) `clamp(-(+0x9A4 - atan(fwd)) * 10, ±1)` and roll
///    +0x990 = 0; otherwise `y = clamp(-d * 1.5, ±0.9)` (RC: ±0.7), +0x988 = y (`4y` below |y| < 0.1, where
///    the roll target is 0, else `-y`), the roll target scaled by `max(0, 1 - (1.2 t - fs) / (0.5 t))` below
///    `1.2 t`; bank `= fpatan(right.z, ±|right.xy|)` (negative when up.z < 0; 0 without a matrix); roll
///    +0x990 = `clamp(wrap(target - (bank + (bank - +0x99C) * 30 / max(1, timestep))) * [0x8A5B48], ±1)`,
///    +0x99C = bank; the pitch gets `|0.23 * bank|` added, a negative old pitch is halved (`min`). Pitch
///    clamped to ±1 (NaN → 1). +0x994 = +0x998; temp action 15: pitch 1 and, below 20 (f64), temp action 0.
///
/// The tunables 0x8A5B44..0x8A5B54 are .data and read at run time. `_CIasin` runs the original CRT code.
#[hook(addr = 0x423000, conv = cdecl, name = "CCarCtrl::FlyAIPlaneInCertainDirection", expect = "E9 5C E9 FD FF 83 EC 18",
       unverified = "popw4-drive-1", route = "L2", hazards = "heap-writes,x87-transcendental",
       recs = "air-water", note = "entry is a no-CD fragment jump that only loads CTimer::m_snPreviousTimeInMilliseconds")]
pub fn fly_ai_plane_in_certain_direction(plane: GPtr<u8>) {
    let tun = |a: usize| crate::common::g::<f32>(a).read();
    let seed = u32::from(rd::<u16>(plane, 0x20));
    let prev = prev_time_ms().wrapping_add(seed) % 1000;
    let now = time_ms().wrapping_add(seed) % 1000;
    if now < prev {
        let m = rd::<GPtr<u8>>(plane, 0x14);
        let h = atan_xy(rd::<f32>(m, 0x10), rd::<f32>(m, 0x14));
        let ta = i32::from(rd::<i8>(plane, 0x3BB));
        let desired = match ta {
            16 => h.to_f32(),
            17 => h.fsub_m(2.0).to_f32(),
            18 => x_(2.0).fadd(h).to_f32(),
            _ => rd::<f32>(plane, 0x9B4),
        }; // [S+0xC]
        wr::<u32>(plane, 0x9AC, 0x43FA_0000);
        wr::<f32>(
            plane,
            0x9B8,
            x_(rd::<f32>(plane, 0x9B4)).fadd_m(k(0x4049_0FDB)).to_f32(),
        );
        let diff = wrap_up(wrap_down(h.fsub_m(desired)));
        let ad = if ge5(diff, x_(0.0)) { diff } else { diff.fchs() };
        let max_dev = if ge5(ad, x_(k(0x3F06_0A92))) {
            diff.fmul_m(1.5).to_f32()
        } else {
            0.0
        }; // [S+0x10]
        for pass in 0..2 {
            for i in 1..20i32 {
                let kk = xi_(i / 2);
                let off = if i & 1 != 0 {
                    kk.fmul_m(k(0x3E86_0A92))
                } else {
                    kk.fmul_m(k(0xBE86_0A92))
                }
                .to_f32(); // the `plane` argument slot
                let cand = x_(off).fadd_m(desired).to_f32(); // [S+0x14]
                let inside = if !ge5(x_(off), x_(0.0)) && !le41(x_(off), x_(max_dev)) {
                    true
                } else {
                    !le41(x_(off), x_(0.0)) && !ge5(x_(off), x_(max_dev))
                };
                if inside != (pass == 0) {
                    continue;
                }
                let r = X87::from_f64_exact(plane_probe_heading(plane, cand));
                if ge5(r, x_(150.0)) {
                    continue;
                }
                wr::<f32>(
                    plane,
                    0x9B8,
                    x_(off)
                        .fmul_m(k(0x3F8C_CCCD))
                        .fadd_m(rd::<f32>(plane, 0x9B4))
                        .to_f32(),
                );
                let alt = r.fadd_m(rd::<f32>(plane, 0x9B0));
                let max_alt = rd::<f32>(plane, 0x9A8);
                let v = if le41(alt, x_(max_alt)) { x_(max_alt) } else { alt };
                wr::<f32>(plane, 0x9AC, v.to_f32());
                break;
            }
        }
    }
    // B. steering
    let tgt = if rd::<u8>(plane, 0x42C) & 0x80 != 0 {
        tun(0x8A5B54)
    } else {
        32.0
    }; // the `plane` argument slot
    let m = rd::<GPtr<u8>>(plane, 0x14);
    let h2 = atan_xy(rd::<f32>(m, 0x10), rd::<f32>(m, 0x14)).to_f32(); // [S+0x10]
    let m = rd::<GPtr<u8>>(plane, 0x14);
    let fs = x_(rd::<f32>(m, 0x14))
        .fmul_m(rd::<f32>(plane, 0x48))
        .fadd(x_(rd::<f32>(m, 0x10)).fmul_m(rd::<f32>(plane, 0x44)))
        .fmul_m(60.0)
        .to_f32(); // [S+0xC]
    let mut m = m; // EDI
    if (0..4).any(|i| !ge5(x_(rd::<f32>(plane, 0x7D4 + 4 * i)), x_(1.0))) {
        wr::<u32>(plane, 0x9C0, time_ms());
    }
    let late = time_ms().wrapping_sub(rd::<u32>(plane, 0x9C0)) > 4000;
    if late {
        if rd::<u32>(plane, 0x9CC) != 0x3F80_0000 {
            plane_set_gear_up(plane);
        }
        m = rd::<GPtr<u8>>(plane, 0x14);
        let p = entity_pos(plane);
        let alt = x_(rd::<f32>(plane, 0x4C)).fmul_m(100.0).fadd_m(vx(p, 2)).to_f32(); // [S+0x20]
        let a = ci_asin(rd::<f32>(m, 0x18));
        let pred = a
            .fsub_m(rd::<f32>(plane, 0x9BC))
            .fmul(x_(100.0).fdiv_m(time_step()))
            .fadd(a)
            .to_f32(); // [S+0x14]
        wr::<f32>(plane, 0x9BC, a.to_f32());
        let v = x_(rd::<f32>(plane, 0x9AC)).fsub_m(alt).fmul_m(k(0x3D08_8889));
        let mut v = if !ge5(v, x_(k(0x3ECC_CCCD))) {
            if le41(v, x_(k(0xBECC_CCCD))) {
                x_(k(0xBECC_CCCD))
            } else {
                v
            }
        } else {
            x_(k(0x3ECC_CCCD))
        };
        if !ge5(x_(fs), x_(tgt)) && ge5(v, x_(0.25)) {
            v = x_(0.25);
        }
        wr::<f32>(plane, 0x98C, v.fsub_m(pred).fmul_m(tun(0x8A5B50)).to_f32());
    } else if ge5(x_(fs), x_(tgt)) {
        wr::<u32>(plane, 0x98C, 0x3ECC_CCCD);
    } else {
        wr::<u32>(plane, 0x98C, 0);
    }
    let d = wrap_down(wrap_up(x_(rd::<f32>(plane, 0x9B8)).fsub_m(h2)));
    if !late {
        let y = x_(rd::<f32>(plane, 0x9A4)).fsub_m(h2).fchs().fmul_m(10.0);
        let y = if ge5(x_(1.0), y) {
            if le41(x_(-1.0), y) { y } else { x_(-1.0) }
        } else {
            x_(1.0)
        };
        wr::<f32>(plane, 0x988, y.to_f32());
        wr::<u32>(plane, 0x990, 0);
    } else {
        let y = d.fchs().fmul_m(1.5);
        let mut y = if ge5(x_(k(0x3F66_6666)), y) {
            if le41(x_(k(0xBF66_6666)), y) {
                y
            } else {
                x_(k(0xBF66_6666))
            }
        } else {
            x_(k(0x3F66_6666))
        };
        // the |y| copy the 0.1 test reads
        let mut ay = None;
        if rd::<u8>(plane, 0x42C) & 0x80 != 0 {
            if !ge5(x_(k(0x3F33_3333)), y) {
                y = x_(k(0x3F33_3333));
                ay = Some(y);
            } else if !le41(x_(k(0xBF33_3333)), y) {
                y = x_(k(0xBF33_3333));
                ay = Some(y.fchs());
            }
        }
        let ay = ay.unwrap_or(if ge5(y, x_(0.0)) { y } else { y.fchs() });
        let mut r = if ge5(ay, x_(k(0x3DCC_CCCD))) {
            wr::<f32>(plane, 0x988, y.to_f32());
            y.fchs()
        } else {
            wr::<f32>(plane, 0x988, y.fmul_m(4.0).to_f32());
            x_(0.0)
        };
        let t12 = x_(tgt).fmul_m(k(0x3F99_999A));
        if !ge5(x_(fs), t12) {
            let f = t12.fsub_m(fs).fdiv(x_(tgt).fmul_m(0.5)).fsubr_m(1.0);
            let f = if le41(x_(0.0), f) { f } else { x_(0.0) };
            r = r.fmul(f);
        }
        let bank = if !m.is_null() {
            let (rx, ry) = (rd::<f32>(m, 0), rd::<f32>(m, 4));
            let mut hl = x_(rx).fmul_m(rx).fadd(x_(ry).fmul_m(ry)).fsqrt();
            if !ge5(x_(rd::<f32>(m, 0x28)), x_(0.0)) {
                hl = hl.fmul_m(-1.0);
            }
            x_(rd::<f32>(m, 8)).fpatan(hl).to_f32()
        } else {
            0.0
        }; // the `plane` argument slot
        let ts = time_step();
        let t = if le41(x_(1.0), x_(ts)) { x_(ts) } else { x_(1.0) };
        let q = x_(tun(0x8A5B4C)).fdiv(t);
        let pred = x_(bank).fsub_m(rd::<f32>(plane, 0x99C)).fmul(q).fadd_m(bank);
        let e = wrap_up(wrap_down(r.fsub(pred))).fmul_m(tun(0x8A5B48));
        wr::<f32>(plane, 0x99C, bank);
        wr::<f32>(plane, 0x990, e.to_f32());
        let e = if ge5(e, x_(1.0)) { x_(1.0) } else { e };
        wr::<f32>(plane, 0x990, e.to_f32());
        let e = if le41(e, x_(-1.0)) { x_(-1.0) } else { e };
        wr::<f32>(plane, 0x990, e.to_f32());
        // the pitch from the bank
        let p = x_(rd::<f32>(plane, 0x98C));
        let b = x_(tun(0x8A5B44)).fmul_m(bank);
        let b = if ge5(b, x_(0.0)) { b } else { b.fchs() };
        let n = b.fadd_m(rd::<f32>(plane, 0x98C)).to_f32();
        wr::<f32>(plane, 0x98C, n);
        if !ge5(p, x_(0.0)) {
            let ph = p.fmul_m(0.5);
            let v = if le41(ph, x_(n)) { ph } else { x_(n) };
            wr::<f32>(plane, 0x98C, v.to_f32());
        }
    }
    // the pitch clamp
    let v = x_(rd::<f32>(plane, 0x98C));
    let v = if ge5(v, x_(1.0)) { x_(1.0) } else { v };
    wr::<f32>(plane, 0x98C, v.to_f32());
    let v = if le41(v, x_(-1.0)) { x_(-1.0) } else { v };
    wr::<f32>(plane, 0x98C, v.to_f32());
    wr::<u32>(plane, 0x994, rd::<u32>(plane, 0x998));
    if rd::<u8>(plane, 0x3BB) == 0xF {
        wr::<u32>(plane, 0x98C, 0x3F80_0000);
        if !ge5(x_(fs), X87::from_f64_exact(20.0)) {
            wr::<u8>(plane, 0x3BB, 0);
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// Racing

orig! {
    /// `CCarPathLink::OneWayLaneOffset()` (0x44DB00, thiscall, ST0)
    fn one_way_lane_offset(link: GPtr<u8>) -> f64 = thiscall 0x44DB00 => one_way_lane_offset_abi;
    /// `CCarCtrl::FindIntersection2Lines(x1, y1, dx1, dy1, x2, y2, dx2, dy2, float* ox, float* oy)` (0x4226F0)
    #[allow(clippy::too_many_arguments)]
    fn find_intersection_2_lines(x1: f32, y1: f32, dx1: f32, dy1: f32, x2: f32, y2: f32, dx2: f32, dy2: f32, ox: GPtr<f32>, oy: GPtr<f32>) = cdecl 0x4226F0 => crate::carctrl2::find_intersection_2_lines;
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn one_way_lane_offset_abi(link: GPtr<u8>) -> f64 {
    crate::pathfind::one_way_lane_offset(link).to_f64()
}

/// The navi link (`CCarPathLink`, 14 bytes) of a `CCarPathLinkAddress`: `ThePaths.m_pNaviNodes[a >> 10]`
/// (0x96F974) + `(a & 0x3FF) * 14`.
fn navi_link(a: u16) -> GPtr<u8> {
    let a = u32::from(a);
    crate::common::g::<GPtr<u8>>((0x96F974 + (a >> 10) * 4) as usize)
        .read()
        .byte_add(((a & 0x3FF) * 14) as isize)
}

/// `fild word; fmul 0.125` of a navi link coordinate.
fn link_coord(l: GPtr<u8>, o: isize) -> X87 {
    xi_(i32::from(rd::<i16>(l, o))).fmul_m(0.125)
}

/// `|a - b|^2 < 100` between two navi links as DealWithBend_Racing tests it: `ey = a.y - b.y`, `ex = a.x -
/// b.x` (each coordinate × 0.125), `ex * ex + ey * ey` against 100 (`test ah, 5; jp`: NaN fails).
fn links_close(a: GPtr<u8>, b: GPtr<u8>) -> bool {
    let ey = link_coord(a, 2).fsub(link_coord(b, 2));
    let ex = link_coord(a, 0).fsub(link_coord(b, 0));
    !ge5(ex.fmul(ex).fadd(ey.fmul(ey)), x_(100.0))
}

/// `(int8 dir * 0.01) * (float)sign` of a navi link's direction byte at `o`, stored f32.
fn link_dir(l: GPtr<u8>, o: isize, sign: i8) -> f32 {
    xi_(i32::from(rd::<i8>(l, o)))
        .fmul_m(k(0x3C23_D70A))
        .fmul(xi_(i32::from(sign)))
        .to_f32()
}

/// `(OneWayLaneOffset(l) + ((float)count - 1)) * d * 5.4` (a fresh OneWayLaneOffset call per use).
fn lane_edge(l: GPtr<u8>, count: u8, d: f32) -> X87 {
    X87::from_f64_exact(one_way_lane_offset(l))
        .fadd(xi_(i32::from(count)).fsub_m(1.0))
        .fmul_m(d)
        .fmul_m(k(0x40AC_CCCD))
}

/// `|v|` as `fcom 0; fld st0; test ah, 5; jp skip; fchs`.
fn abs5(v: X87) -> X87 {
    if ge5(v, x_(0.0)) { v } else { v.fchs() }
}

/// `min(|diff * 2/pi|, 1)` as DealWithBend_Racing computes it twice (`1 < |t|` → 1; NaN keeps `|t|`).
fn bend_curve(diff: f32) -> X87 {
    let t = x_(diff).fmul_m(k(0x3F22_F983));
    if ge5(x_(1.0), abs5(t)) { abs5(t) } else { x_(1.0) }
}

/// The two edge points of a navi link `l` (lane offsets from the opposite / same lane counts, bits 0-2 /
/// 3-5 of +0xB) around `(cx, cy)` along the scaled direction `(dx, dy)`, and the one nearer to `(rx, ry)`
/// (`dQ > dP` picks P, `<=` / NaN Q): returns `(P.x, P.y, Q.x, Q.y as x87, p_nearer)`.
#[allow(clippy::too_many_arguments)]
fn link_edges(
    l: GPtr<u8>,
    cx: f32,
    cy: f32,
    dx: f32,
    dy: f32,
    rx: f32,
    ry: f32,
) -> (f32, f32, f32, X87, bool) {
    let b = rd::<u8>(l, 0xB);
    let (opp, same) = (b & 7, (b >> 3) & 7);
    let px = lane_edge(l, opp, dy).fadd_m(cx).to_f32();
    let py = lane_edge(l, opp, dx).fsubr_m(cy).to_f32();
    let qx = lane_edge(l, same, dy).fsubr_m(cx).to_f32();
    let qy = lane_edge(l, same, dx).fadd_m(cy);
    let b_ = x_(py).fsub_m(ry);
    let a_ = x_(px).fsub_m(rx);
    let c_ = qy.fsub_m(ry);
    let d_ = x_(qx).fsub_m(rx);
    let dp = a_.fmul(a_).fadd(b_.fmul(b_));
    let dq = d_.fmul(d_).fadd(c_.fmul(c_));
    (px, py, qx, qy, !le41(dq, dp))
}

/// `CCarCtrl::DealWithBend_Racing(CVehicle* veh, CCarPathLinkAddress l1, l2, l3, l4, int8 s1, s2, s3, s4, float
/// speed, float* outAngle, float* outSpeedMult, float* outAngleDiff, float* outDist, CVector2D* outPos)`
/// (0x428040, cdecl, AL). Returns 0 for an invalid `l1` / `l2`; then `*outSpeedMult = 1`, and 0 for an
/// unloaded l1 / l2 area. `*outDist = |n1 - P|` (n1 = the l1 navi point; `dy^2 + dx^2`); farther than 60 →
/// 0. The exit link B: l2 (sign s2); with a valid l3 within 10 of l2: l3 (s3); then (only with a valid l3)
/// a valid l4 within 10 of the current choice: l4 (s4). The scaled directions `D1 = l1.dir * 0.01 * s1`,
/// `DB` (f32). `diff = atan(fwd) - atan(DB)` wrapped (`> pi` first, f32 copy kept); below 30° (BMX +0x594 ==
/// 10: 20°) → 0, and below 60° (40°) when both links have at least 4 lanes → 0. `*outAngleDiff = diff`.
/// The edge points of l1 nearest to B give N (the other F, pulled to within 11 of N); those of B nearest
/// to n1 give M. outPos = N when `|D1.x - DB.x| < 0.1` and `|D1.y - DB.y| != 0` (the original's odd
/// parallel test, kept; NaN counts as not equal), otherwise `FindIntersection2Lines(N, D1, M, DB)`.
/// Below the distance limit 40 (BMX 50): `*outSpeedMult = 1 - ((lim - dist) / lim) * c * 0.6` with
/// `c = min(|diff * 2/pi|, 1)`. Within `clamp(speed / 30, 0.2, 1) * c * 20` the angle is towards outPos,
/// otherwise towards F; returns 1.
///
/// The original keeps n1.y and later M.y in the `l1` argument slot and lane counts in dead locals; the
/// only caller (0x435E05) pops them. After FindIntersection2Lines it reads `outPos` from EDX, which that
/// callee preserves (its port is `preserve = "edx"`); the port uses the argument.
#[hook(addr = 0x428040, conv = cdecl, name = "CCarCtrl::DealWithBend_Racing", expect = "8B 44 24 08 83 EC 40 66",
       unverified = "popw4-drive-1", route = "L2", hazards = "x87-transcendental",
       recs = "car-ai,driving", note = "writes only through its five out-pointers (outPos: 8 bytes)")]
#[allow(clippy::too_many_arguments)]
pub fn deal_with_bend_racing(
    veh: GPtr<u8>,
    l1: u32,
    l2: u32,
    l3: u32,
    l4: u32,
    s1: u32,
    s2: u32,
    s3: u32,
    s4: u32,
    speed: f32,
    out_angle: GPtr<f32>,
    out_speed_mult: GPtr<f32>,
    out_angle_diff: GPtr<f32>,
    out_dist: GPtr<f32>,
    out_pos: GPtr<f32>,
) -> u8 {
    let (l1, l2, l3, l4) = (l1 as u16, l2 as u16, l3 as u16, l4 as u16);
    let (s1, s2, s3, s4) = (s1 as u8 as i8, s2 as u8 as i8, s3 as u8 as i8, s4 as u8 as i8);
    if l1 == 0xFFFF || l2 == 0xFFFF {
        return 0;
    }
    let bmx = rd::<i32>(veh, 0x594) == 10;
    let (thr_a, thr_b, dist_lim) = if bmx {
        (k(0x3EB2_B8C3), k(0x3F32_B8C3), 50.0f32)
    } else {
        (k(0x3F06_0A92), k(0x3F86_0A92), 40.0f32)
    };
    out_speed_mult.write(1.0);
    if path_nodes(u32::from(l1) >> 10).is_null() || path_nodes(u32::from(l2) >> 10).is_null() {
        return 0;
    }
    let k1 = navi_link(l1);
    let a = navi_link(l2);
    let n1x = link_coord(k1, 0).to_f32(); // [sp+0x10]
    let n1y = link_coord(k1, 2).to_f32(); // the `l1` slot
    let p = entity_pos(veh);
    let dy = x_(n1y).fsub_m(vx(p, 1));
    let dx = x_(n1x).fsub_m(vx(p, 0));
    let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
    out_dist.write(d.to_f32());
    if !le41(d, x_(60.0)) {
        return 0;
    }
    // the exit link
    let (b, sb) = if l3 == 0xFFFF {
        (a, s2)
    } else {
        let k3 = navi_link(l3);
        let (mut b, mut sb) = if links_close(a, k3) { (k3, s3) } else { (a, s2) };
        if l4 != 0xFFFF {
            let k4 = navi_link(l4);
            if links_close(b, k4) {
                b = k4;
                sb = s4;
            }
        }
        (b, sb)
    };
    let bx = link_coord(b, 0).to_f32(); // [sp+0x28]
    let by = link_coord(b, 2).to_f32(); // [sp+0x2C]
    let d1x = link_dir(k1, 8, s1); // [sp+0x3C]
    let d1y = link_dir(k1, 9, s1); // [sp+0x38]
    let dbx = link_dir(b, 8, sb); // [sp+0x30]
    let dby = link_dir(b, 9, sb); // [sp+0x34]
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let h = atan_xy(rd::<f32>(m, 0x10), rd::<f32>(m, 0x14)).to_f32();
    let diff = wrap_up(wrap_down(atan_xy(dbx, dby).fsubr_m(h)));
    let diff_f = diff.to_f32(); // [sp+0x44]
    if !ge5(abs5(diff), x_(thr_a)) {
        return 0;
    }
    let lanes = |l: GPtr<u8>| {
        let v = rd::<u8>(l, 0xB);
        ((v >> 3) & 7) + (v & 7)
    };
    if lanes(b) >= 4 && lanes(k1) >= 4 && !ge5(abs5(diff), x_(thr_b)) {
        return 0;
    }
    out_angle_diff.write(diff_f);
    // N / F from l1's edges (nearest to B)
    let (px, py, qx, qy, p_near) = link_edges(k1, n1x, n1y, d1x, d1y, bx, by);
    let (nx, ny, mut fx, mut fy) = if p_near {
        (px, py, qx, qy.to_f32())
    } else {
        (qx, qy.to_f32(), px, py)
    };
    let ex = x_(fx).fsub_m(nx);
    let ey = x_(fy).fsub_m(ny);
    let w = ey.fmul(ey).fadd(ex.fmul(ex)).fsqrt();
    if !le41(w, x_(11.0)) {
        let kk = w.fdivr_m(11.0);
        fx = kk.fmul(ex).fadd_m(nx).to_f32();
        fy = kk.fmul(ey).fadd_m(ny).to_f32();
    }
    // M from B's edges (nearest to n1)
    let (p2x, p2y, q2x, q2y, p2_near) = link_edges(b, bx, by, dbx, dby, n1x, n1y);
    let (mx, my) = if p2_near { (p2x, p2y) } else { (q2x, q2y.to_f32()) };
    // the corner
    let par_x = !ge5(abs5(x_(d1x).fsub_m(dbx)), x_(k(0x3DCC_CCCD)));
    // `fcomp 0.0; test ah, 0x44; jnp`: equal goes to the intersection, NaN / non-zero copy N
    let copy_n = par_x && abs5(x_(d1y).fsub_m(dby)).fcom(x_(0.0)).test_parity(0x44);
    if copy_n {
        out_pos.write(x_(nx).to_f32());
        out_pos.byte_add(4).write(ny);
    } else {
        find_intersection_2_lines(nx, ny, d1x, d1y, mx, my, dbx, dby, out_pos, out_pos.byte_add(4));
    }
    if !ge5(x_(out_dist.read()), x_(dist_lim)) {
        let c = bend_curve(diff_f);
        let v = x_(dist_lim)
            .fsub_m(out_dist.read())
            .fdiv_m(dist_lim)
            .fmul(c)
            .fmul_m(k(0x3F19_999A))
            .fsubr_m(1.0);
        out_speed_mult.write(v.to_f32());
    }
    let c = bend_curve(diff_f);
    let s = x_(speed).fmul_m(k(0x3D08_8889));
    let s = if !ge5(x_(1.0), s) {
        x_(1.0)
    } else if !le41(x_(k(0x3E4C_CCCD)), s) {
        x_(k(0x3E4C_CCCD))
    } else {
        s
    };
    let lim = s.fmul(c).fmul_m(20.0);
    let p = entity_pos(veh);
    let (tx, ty) = if !ge5(x_(out_dist.read()), lim) {
        (out_pos.read(), out_pos.byte_add(4).read())
    } else {
        (fx, fy)
    };
    let ay = x_(ty).fsub_m(vx(p, 1)).to_f32();
    let ax = x_(tx).fsub_m(vx(p, 0)).to_f32();
    out_angle.write(atan_xy(ax, ay).to_f32());
    1
}

orig! {
    /// `CCarCtrl::StopCarIfNodesAreInvalid(CVehicle*)` (0x422590)
    fn stop_car_if_nodes_are_invalid(v: GPtr<u8>) -> u8 = cdecl 0x422590 => crate::carctrl2::stop_car_if_nodes_are_invalid;
    /// `CPathFind::DoPathSearch(...)` (0x4515D0, thiscall on ThePaths, gr parameter order)
    #[allow(clippy::too_many_arguments)]
    fn do_path_search(this: GPtr<u8>, path_type: u32, ox: f32, oy: f32, oz: f32, origin_addr: u32, tx: f32, ty: f32, tz: f32, out_nodes: GPtr<u32>, out_count: GPtr<i16>, max_nodes: i32, out_distance: GPtr<f32>, max_search: f32, target_addr: GPtr<u32>, max_depth: f32, same_lane: u32, forbidden: u32, allow_water: u32, for_boats: u32) = thiscall 0x4515D0 => crate::pathfind::do_path_search;
    /// `CAutoPilot::RemoveOnePathNode()` (0x41B950, thiscall)
    fn remove_one_path_node(ap: GPtr<u8>) = thiscall 0x41B950 => crate::carctrl2::auto_pilot_remove_one_path_node;
    /// `CNodeAddress::operator==` (0x420920, thiscall)
    fn node_eq(a: GPtr<u16>, b: GPtr<u16>) -> u32 = thiscall 0x420920 => crate::pathfind::node_address_eq;
    /// `CNodeAddress::operator<` (0x420980, thiscall)
    fn node_lt(a: GPtr<u16>, b: GPtr<u16>) -> u32 = thiscall 0x420980 => crate::pathfind::node_address_lt;
    /// `CPathFind::FindLinkBetweenNodes(CNodeAddress, CNodeAddress)` (0x451350, thiscall, hidden return pointer)
    fn find_link_between_nodes(this: GPtr<u8>, out: GPtr<u16>, a: u32, b: u32) -> GPtr<u16> = thiscall 0x451350 => crate::pathfind::find_link_between_nodes;
    /// `CCarCtrl::ClitargetOrientationToLink(CVehicle*, CCarPathLinkAddress, int8 side, float* heading, float x, float y)` (0x422760)
    fn clitarget_orientation_to_link(v: GPtr<u8>, link: u32, side: u32, heading: GPtr<f32>, x: f32, y: f32) = cdecl 0x422760 => unported;
    /// `CCarCtrl::FindPercDependingOnDistToLink(CVehicle*, CCarPathLinkAddress)` (0x422620, ST0)
    fn find_perc_depending_on_dist_to_link(v: GPtr<u8>, link: u16) -> f64 = cdecl 0x422620 => find_perc_abi;
    /// `CTrafficLights::ShouldCarStopForBridge(CVehicle*)` (0x49D420)
    fn should_car_stop_for_bridge(v: GPtr<u8>) -> u8 = cdecl 0x49D420 => crate::misc2::should_car_stop_for_bridge;
    /// `CCarAI::CarHasReasonToStop(CVehicle*)` (0x41C050)
    fn car_has_reason_to_stop(v: GPtr<u8>) = cdecl 0x41C050 => crate::carctrl::car_has_reason_to_stop;
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn find_perc_abi(v: GPtr<u8>, link: u16) -> f64 {
    crate::carctrl2::find_perc_depending_on_dist_to_link(v, link).to_f64()
}

/// `p * a + q * b` of two navi link coordinates as the Racing steering interpolates them: `fild word;
/// fmul 0.125; fmul st(i)` per term, summed on the FPU (the first term the one loaded first).
fn mix_coord(la: u16, wa: X87, lb: u16, wb: X87, o: isize) -> X87 {
    link_coord(navi_link(la), o)
        .fmul(wa)
        .fadd(link_coord(navi_link(lb), o).fmul(wb))
}

/// `CCarCtrl::SteerAICarWithPhysicsFollowPath_Racing(CVehicle* veh, float* steer, float* gas, float* brake,
/// bool* handbrake)` (0x435830, cdecl).
/// 1. `StopCarIfNodesAreInvalid` → return. `fs = ((mv.z * fwd.z + mv.y * fwd.y) + mv.x * fwd.x) * 60`
///    (f32); the 2D forward normalised (`len == 0` exactly: x = 1, y = the raw fwd.y; NaN divides).
/// 2. Either link area (+0x3A4, +0x3A6) unloaded: brake 1, gas 0, steer 0, handbrake 0.
/// 3. With the navi points `P1` (current link) and `P2` (next), `dist = |V - P1|`, `segLen = |P2 - P1|`,
///    `dot` (all f32): advance when `dist < 8`, or `dot > 0` and `dist < 12`, or `dot / (segLen * dist) >
///    0.7`, or the links are equal. Advancing: below 4 path nodes (+0x418) `DoPathSearch(0, V, +0x394, dest
///    +0x3EC, +0x3F8, &count, 8, null, 999999.875, null, 999999.875, (+0x3DB >> 6) & 1, [0x8A5F44], model ==
///    0x21B (the pushed dword keeps the upper three bytes of the matrix pointer: `sete cl`), 0)`, then
///    `RemoveOnePathNode` once, twice when +0x394 equals the first node; no node left → mission 9 and
///    `HeadingForTarget(dest)`. Then the node / link / sign shift (prev = cur, cur = next, next = +0x3F8[0],
///    RemoveOnePathNode, links and signs moved), the new next link found by the unbounded scan of the
///    current node's links, sign +0x3B6 = `cur < next ? -1 : 1`.
/// 4. Lookahead links / nodes / signs from the current state and up to 4 path nodes
///    (`FindLinkBetweenNodes`); `DealWithBend_Racing` over the windows j = 0..3 picks the first bend.
/// 5. Target angle: no bend → the heading clipped (`ClitargetOrientationToLink`) by the links 5..2, the
///    next link, and the point `Lc * p + Ln * (1 - p)` (p = `FindPercDependingOnDistToLink`) with the
///    current link; a bend → its angle, clipped by the next link unless the bend is at the current link, and
///    the point `Ln * (1 - q) + Lc * q` with `q = (p + 2) / 3`.
/// 6. Steer = `FindAngleToWeaveThroughTraffic(angle, heading, 2) - heading` wrapped (`< -pi` first),
///    limited to ±(|mv| > 0.7 ? 0.2 : min(0.9 - |mv|, 0.7)) on the x87 value (clamped flag).
/// 7. Speed factor `min(bridge stop ? 0 : 1, bend multiplier)`; `diff = factor * cruise - fs` (f32): small
///    target and diff → brake 1, gas 0; `diff > 0` → gas `min(diff * (fs < 2 ? 0.25 : 0.125), 1)` (0 when
///    clamped and fs > 7; BMX with diff > 3 and +0x818 <= 0 → +0x818 = 15); otherwise gas 0, brake
///    `min(diff * -1/15, 1)` (BMX) or `min(diff * -0.05, 0.5)`. Steer stored, handbrake 0.
///
/// Uncertain: the original leaves the node half of unused lookahead nodes and the signs of unused
/// lookahead links uninitialised; the port uses 0. They are only read for links / nodes the code itself
/// marks invalid (a path-node list with a hole at an area of 0xFFFF before a valid entry would make
/// `FindLinkBetweenNodes` read one: not reproduced). The `CCarPathLinkAddress` / `CNodeAddress` vector
/// constructors (0x403D60) are replaced by their writes.
#[hook(addr = 0x435830, conv = cdecl, name = "CCarCtrl::SteerAICarWithPhysicsFollowPath_Racing", expect = "83 EC 68 53 55 56 8B 74",
       unverified = "popw4-drive-1", route = "L2", hazards = "heap-writes,x87-transcendental",
       recs = "car-ai,driving", note = "path search regions as UpdateCarOnRails (ThePaths, aNodesToBeCleared)")]
pub fn steer_ai_car_with_physics_follow_path_racing(
    veh: GPtr<u8>,
    steer: GPtr<f32>,
    gas: GPtr<f32>,
    brake: GPtr<f32>,
    hb: GPtr<u8>,
) {
    let mut clamped = false; // [E+0x13]
    if stop_car_if_nodes_are_invalid(veh) != 0 {
        return;
    }
    let m = rd::<GPtr<u8>>(veh, 0x14); // ECX
    let fs = x_(rd::<f32>(veh, 0x4C))
        .fmul_m(rd::<f32>(m, 0x18))
        .fadd(x_(rd::<f32>(veh, 0x48)).fmul_m(rd::<f32>(m, 0x14)))
        .fadd(x_(rd::<f32>(veh, 0x44)).fmul_m(rd::<f32>(m, 0x10)))
        .fmul_m(60.0)
        .to_f32(); // [E+0x1C]
    let (fwx, fwy) = (rd::<f32>(m, 0x10), rd::<f32>(m, 0x14));
    let len = x_(fwy).fmul_m(fwy).fadd(x_(fwx).fmul(x_(fwx))).fsqrt();
    let (fnx, fny) = if !len.fcom(x_(0.0)).test_parity(0x44) {
        (1.0f32, fwy)
    } else {
        let inv = len.fdivr_m(1.0);
        (inv.fmul(x_(fwx)).to_f32(), inv.fmul_m(fwy).to_f32())
    }; // [E+0x28], [E+0x18]
    let cur = rd::<u16>(veh, 0x3A4); // DI
    let nxt = rd::<u16>(veh, 0x3A6); // DX
    if path_nodes(u32::from(cur) >> 10).is_null() || path_nodes(u32::from(nxt) >> 10).is_null() {
        brake.write(1.0);
        gas.write(0.0);
        steer.write(0.0);
        hb.write(0);
        return;
    }
    let p1x = link_coord(navi_link(cur), 0).to_f32();
    let p1y = link_coord(navi_link(cur), 2).to_f32();
    let p2x = link_coord(navi_link(nxt), 0).to_f32();
    let p2y = link_coord(navi_link(nxt), 2).to_f32();
    let pos: GPtr<f32> = if m.is_null() {
        veh.byte_add(4).cast()
    } else {
        m.byte_add(0x30).cast()
    };
    let dx = x_(vx(pos, 0)).fsub_m(p1x);
    let dy = x_(vx(pos, 1)).fsub_m(p1y);
    let dist = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt().to_f32(); // [E+0x14]
    let ex = x_(p2x).fsub_m(p1x);
    let ey = x_(p2y).fsub_m(p1y);
    let seg = ey.fmul(ey).fadd(ex.fmul(ex)).fsqrt().to_f32(); // [E+0x30]
    let dot = ey.fmul(dy).fadd(ex.fmul(dx)).to_f32(); // [E+0x24]
    let advance = !ge5(x_(dist), x_(8.0))
        || (!le41(x_(dot), x_(0.0)) && !ge5(x_(dist), x_(12.0)))
        || !le41(x_(seg).fmul_m(dist).fdivr_m(dot), x_(k(0x3F33_3333)))
        || nxt == cur;
    if advance {
        let count = veh.byte_add(0x418).cast::<i16>();
        let nodes_p = veh.byte_add(0x3F8).cast::<u32>();
        if count.read() < 4 {
            let flag = (m.addr() as u32 & 0xFFFF_FF00) | u32::from(rd::<u16>(veh, 0x22) == 0x21B);
            do_path_search(
                crate::common::g::<u8>(0x96F050),
                0,
                vx(pos, 0),
                vx(pos, 1),
                vx(pos, 2),
                rd::<u32>(veh, 0x394),
                rd::<f32>(veh, 0x3EC),
                rd::<f32>(veh, 0x3F0),
                rd::<f32>(veh, 0x3F4),
                nodes_p,
                count,
                8,
                GPtr::NULL,
                k(0x4974_23FE),
                GPtr::NULL,
                k(0x4974_23FE),
                u32::from((rd::<u8>(veh, 0x3DB) >> 6) & 1),
                crate::common::g::<u32>(0x8A5F44).read(),
                flag,
                0,
            );
            if count.read() > 0 {
                remove_one_path_node(veh.byte_add(0x390));
                if count.read() > 0 && node_eq(veh.byte_add(0x394).cast(), nodes_p.cast()) != 0 {
                    remove_one_path_node(veh.byte_add(0x390));
                }
            }
            if count.read() < 1 {
                wr::<u8>(veh, 0x3BA, 9);
                steer_heading_for_target(
                    veh,
                    GPtr::NULL,
                    rd::<f32>(veh, 0x3EC),
                    rd::<f32>(veh, 0x3F0),
                    steer,
                    gas,
                    brake,
                    hb,
                );
                return;
            }
        }
        // the shift
        wr::<u32>(veh, 0x398, rd::<u32>(veh, 0x390));
        wr::<u32>(veh, 0x390, rd::<u32>(veh, 0x394));
        wr::<u32>(veh, 0x394, rd::<u32>(veh, 0x3F8));
        remove_one_path_node(veh.byte_add(0x390));
        let old_cur = rd::<u16>(veh, 0x3A4);
        wr::<u16>(veh, 0x3A4, rd::<u16>(veh, 0x3A6));
        wr::<u8>(veh, 0x3B4, rd::<u8>(veh, 0x3B5));
        wr::<u8>(veh, 0x3B7, rd::<u8>(veh, 0x3B8));
        wr::<u16>(veh, 0x3A8, old_cur);
        wr::<u8>(veh, 0x3B5, rd::<u8>(veh, 0x3B6));
        let c = rd::<u32>(veh, 0x390);
        let area = (c & 0xFFFF) as usize;
        let base = i32::from(rd::<i16>(path_node(c), 0x10));
        let links = crate::common::g::<GPtr<u8>>(0x96FA94 + area * 4).read();
        let (na, nn) = (rd::<u16>(veh, 0x394), rd::<u16>(veh, 0x396));
        let mut i = 0i32;
        loop {
            let e = links.byte_add(((base + i) * 4) as isize);
            if rd::<u16>(e, 0) == na && rd::<u16>(e, 2) == nn {
                break;
            }
            i += 1;
        }
        let navi = crate::common::g::<GPtr<u8>>(0x96FDF4 + area * 4).read();
        let c = rd::<u32>(veh, 0x390);
        let base = i32::from(rd::<i16>(
            path_nodes(c & 0xFFFF).byte_add(((c >> 16) * 0x1C) as isize),
            0x10,
        ));
        wr::<u16>(veh, 0x3A6, rd::<u16>(navi, ((base + i) * 2) as isize));
        let lt = node_lt(veh.byte_add(0x390).cast(), veh.byte_add(0x394).cast());
        wr::<i8>(veh, 0x3B6, if lt != 0 { -1 } else { 1 });
    }
    // 4. the lookahead tables
    let mut links = [0xFFFFu16; 8]; // [E+0x48]
    let mut nodes = [0xFFFFu32; 8]; // [E+0x58] (node halves uninitialised in the original)
    let mut signs = [0i8; 8]; // [E+0x34] (2.. uninitialised in the original)
    links[0] = rd::<u16>(veh, 0x3A4);
    links[1] = rd::<u16>(veh, 0x3A6);
    signs[0] = rd::<i8>(veh, 0x3B5);
    signs[1] = rd::<i8>(veh, 0x3B6);
    nodes[0] = rd::<u32>(veh, 0x390);
    nodes[1] = rd::<u32>(veh, 0x394);
    for j in 0..4usize {
        let pn = rd::<u32>(veh, 0x3F8 + 4 * j as isize);
        if (j as i32) < i32::from(rd::<i16>(veh, 0x418)) && pn as u16 != 0xFFFF {
            nodes[2 + j] = pn;
            let mut out = 0xFFFFu16; // [E+0x14]
            let r = find_link_between_nodes(
                crate::common::g::<u8>(0x96F050),
                GPtr::from_raw(&mut out as *mut u16),
                nodes[1 + j],
                pn,
            );
            links[2 + j] = r.read();
            let np = nodes.as_mut_ptr();
            let lt = node_lt(
                GPtr::from_raw(np.wrapping_add(1 + j)).cast(),
                GPtr::from_raw(np.wrapping_add(2 + j)).cast(),
            );
            signs[2 + j] = if lt != 0 { -1 } else { 1 };
        } else {
            links[2 + j] = 0xFFFF;
            nodes[2 + j] = (nodes[2 + j] & 0xFFFF_0000) | 0xFFFF;
        }
    }
    // the bend search
    let mut out_angle = 0f32; // [E+0x14]
    let mut out_speed_mult = 1.0f32; // [E+0x24]
    let mut out_angle_diff = 0f32; // [E+0x20]
    let mut out_dist = 0f32; // [E+0x2C]
    let mut out_pos = [0f32; 2]; // [E+0x3C]
    let mut chosen = 0xFFFFu16;
    for j in 0..4usize {
        let s = |i: usize| u32::from(signs[i] as u8);
        if deal_with_bend_racing_call(
            veh,
            u32::from(links[j]),
            u32::from(links[j + 1]),
            u32::from(links[j + 2]),
            u32::from(links[j + 3]),
            s(j),
            s(j + 1),
            s(j + 2),
            s(j + 3),
            fs,
            GPtr::from_raw(&mut out_angle as *mut f32),
            GPtr::from_raw(&mut out_speed_mult as *mut f32),
            GPtr::from_raw(&mut out_angle_diff as *mut f32),
            GPtr::from_raw(&mut out_dist as *mut f32),
            GPtr::from_raw(out_pos.as_mut_ptr()),
        ) != 0
        {
            chosen = links[j];
            break;
        }
    }
    let heading = atan_xy(fnx, fny).to_f32(); // [E+0x20]
    let mut angle; // [E+0x14]
    let ap = |a: &mut f32| GPtr::from_raw(a as *mut f32);
    let (x, y);
    if chosen == 0xFFFF {
        angle = heading;
        for idx in (2..=5).rev() {
            let l = links[idx];
            if l == 0xFFFF {
                continue;
            }
            let ly = link_coord(navi_link(l), 2).to_f32();
            let lx = link_coord(navi_link(l), 0).to_f32();
            clitarget_orientation_to_link(
                veh,
                u32::from(l),
                u32::from(signs[idx] as u8),
                ap(&mut angle),
                lx,
                ly,
            );
        }
        let ln = rd::<u16>(veh, 0x3A6);
        let ly = link_coord(navi_link(ln), 2).to_f32();
        let lx = link_coord(navi_link(ln), 0).to_f32();
        clitarget_orientation_to_link(
            veh,
            u32::from(ln),
            u32::from(rd::<u8>(veh, 0x3B6)),
            ap(&mut angle),
            lx,
            ly,
        );
        let p = X87::from_f64_exact(find_perc_depending_on_dist_to_link(veh, rd::<u16>(veh, 0x3A4)));
        let q = x_(1.0).fsub(p);
        let (lc, ln) = (rd::<u16>(veh, 0x3A4), rd::<u16>(veh, 0x3A6));
        y = mix_coord(lc, p, ln, q, 2).to_f32();
        x = mix_coord(lc, p, ln, q, 0).to_f32();
    } else {
        angle = out_angle;
        if rd::<u16>(veh, 0x3A4) != chosen {
            let ln = rd::<u16>(veh, 0x3A6);
            let ly = link_coord(navi_link(ln), 2).to_f32();
            let lx = link_coord(navi_link(ln), 0).to_f32();
            clitarget_orientation_to_link(
                veh,
                u32::from(ln),
                u32::from(rd::<u8>(veh, 0x3B6)),
                ap(&mut angle),
                lx,
                ly,
            );
        }
        let p = X87::from_f64_exact(find_perc_depending_on_dist_to_link(veh, rd::<u16>(veh, 0x3A4)));
        let q = p.fadd_m(2.0).fmul_m(k(0x3EAA_AAAB));
        let r = x_(1.0).fsub(q);
        let (lc, ln) = (rd::<u16>(veh, 0x3A4), rd::<u16>(veh, 0x3A6));
        y = mix_coord(ln, r, lc, q, 2).to_f32();
        x = mix_coord(ln, r, lc, q, 0).to_f32();
    }
    clitarget_orientation_to_link(
        veh,
        u32::from(rd::<u16>(veh, 0x3A4)),
        u32::from(rd::<u8>(veh, 0x3B5)),
        ap(&mut angle),
        x,
        y,
    );
    // 6. the steer
    let w =
        X87::from_f64_exact(find_angle_to_weave_call(veh, GPtr::NULL, angle, heading, 2.0)).fsub_m(heading);
    let mut st = wrap_down(wrap_up(w));
    let mut steer_f = st.to_f32(); // [E+0x18]
    let (mx, my, mz) = (rd::<f32>(veh, 0x44), rd::<f32>(veh, 0x48), rd::<f32>(veh, 0x4C));
    let sp = x_(mx)
        .fmul(x_(mx))
        .fadd(x_(my).fmul(x_(my)))
        .fadd(x_(mz).fmul(x_(mz)))
        .fsqrt();
    let lim = if !le41(sp, x_(k(0x3F33_3333))) {
        k(0x3E4C_CCCD)
    } else {
        let v = sp.fsubr_m(k(0x3F66_6666));
        if ge5(x_(k(0x3F33_3333)), v) {
            v.to_f32()
        } else {
            k(0x3F33_3333)
        }
    }; // [E+0x14]
    let nlim = x_(lim).fchs().to_f32(); // [E+0x28]
    if !ge5(st, x_(nlim)) {
        clamped = true;
        st = x_(nlim);
        steer_f = nlim;
    }
    if !le41(st, x_(lim)) {
        steer_f = lim;
        clamped = true;
    }
    // 7. the speed
    let f = if should_car_stop_for_bridge(veh) != 0 {
        car_has_reason_to_stop(veh);
        x_(0.0)
    } else {
        x_(1.0)
    };
    let f = if !ge5(f, x_(out_speed_mult)) {
        f
    } else {
        x_(out_speed_mult)
    };
    brake.write(0.0);
    let desired = f.fmul(xi_(i32::from(rd::<u8>(veh, 0x3D0))));
    let diff = desired.fsub_m(fs).to_f32(); // [E+0x14]
    let bmx = rd::<i32>(veh, 0x594) == 10;
    if !ge5(desired, x_(k(0x3D4C_CCCD))) && !ge5(x_(diff), x_(k(0x3CF5_C28F))) {
        brake.write(1.0);
        gas.write(0.0);
    } else if !le41(x_(diff), x_(0.0)) {
        if clamped && !le41(x_(fs), x_(7.0)) {
            gas.write(0.0);
        } else {
            let g = x_(diff).fmul_m(if ge5(x_(fs), x_(2.0)) { 0.125 } else { 0.25 });
            gas.write(if ge5(x_(1.0), g) { g.to_f32() } else { 1.0 });
            if bmx && !le41(x_(diff), x_(3.0)) && !x_(rd::<f32>(veh, 0x818)).fcom(x_(0.0)).test_parity(0x41) {
                wr::<f32>(veh, 0x818, 15.0);
            }
        }
    } else {
        gas.write(0.0);
        let b = if bmx {
            let b = x_(diff).fmul_m(k(0xBD88_8889));
            if ge5(x_(1.0), b) { b } else { x_(1.0) }
        } else {
            let b = x_(diff).fmul_m(k(0xBD4C_CCCD));
            if ge5(x_(0.5), b) { b } else { x_(0.5) }
        };
        brake.write(b.to_f32());
    }
    steer.write(steer_f);
    hb.write(0);
}

orig! {
    /// `CCarCtrl::DealWithBend_Racing(...)` (0x428040, AL)
    #[allow(clippy::too_many_arguments)]
    fn deal_with_bend_racing_call(veh: GPtr<u8>, l1: u32, l2: u32, l3: u32, l4: u32, s1: u32, s2: u32, s3: u32, s4: u32, speed: f32, out_angle: GPtr<f32>, out_speed_mult: GPtr<f32>, out_angle_diff: GPtr<f32>, out_dist: GPtr<f32>, out_pos: GPtr<f32>) -> u8 = cdecl 0x428040 => deal_with_bend_racing;
}
