//! `CAutomobile::FireTruckControl(CFire*)` (0x729B60; FRONTIER-2 part 3, batch `frontier2-6`): the water cannon of the
//! fire truck (and the SWAT van, model 0x259) — aimed by the player (camera or pad) or at a fire by the AI crew
//! (crossing scenario, 4000 frames). Ported from the 1.0 US bytes; gta-reversed `Automobile.cpp` as a reading aid.
//!
//! Fields: +0x94C cannon heading (relative to the vehicle), +0x950 cannon pitch, +0x698 the cannon's frame.

use sa_core::abi::HookReg;
use sa_core::field::{gf as fc, rd3f as v3, v3_ptr as ptr};
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::fpu::{fabs, ge_or_nan, gt, le_or_nan, lt};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::bindings::get_car_gun_left_right;
use crate::common::{f32_of, g, rand_at, rd, wr, x, xi};
use sa_core::bindings::{find_player_vehicle, frame_get_ltm, get_pad, matrix_mul_vec};
use sa_pop::bindings::get_car_gun_up_down;
use sa_world::bindings::{multiply3x3, multiply3x3_vm, pad_get_car_gun_fired, physical_get_speed};

orig! {
    /// `CWaterCannons::UpdateOne(uint32 id, CVector* pos, CVector* dir)` (0x728CB0)
    fn water_cannons_update_one(id: GPtr<u8>, pos: GPtr<u8>, dir: GPtr<u8>) = cdecl 0x728CB0 => unported;
}

const SWAT_VAN: u16 = 0x259;
/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7CB5C;
const PI: usize = 0x858CB8;
const TWO_PI: usize = 0x858CBC;


/// An angle brought within pi of `around` (one 2pi step either way; `a > around + pi` → -2pi, `a < around - pi`
/// → +2pi), as the x87 sequences do it.
fn wrap_near(a: X87, around: f32) -> X87 {
    if gt(a, x(around).fadd_m(fc(PI))) {
        a.fsub_m(fc(TWO_PI))
    } else if lt(a, x(around).fsub_m(fc(PI))) {
        a.fadd_m(fc(TWO_PI))
    } else {
        a
    }
}

/// `CAutomobile::FireTruckControl(CFire* fire)` (0x729B60, `ret 4`; the model test passes through the island
/// 0x407293). The player's vehicle: with camera mode 0x12 the cannon turns towards the camera's front in the
/// vehicle's frame (heading `atan2(-x, y)` wrapped within pi of the cannon heading, pitch `atan2(z, |xy|)` plus 0.384
/// (SWAT) / 0.262) by at most timestep × 0.05 / 0.02 per frame; otherwise the pad's car-gun left/right and up/down
/// move it (× timestep × 0.05 / 0.02 / 128). The heading is wrapped to [-pi, pi], the pitch clamped (SWAT
/// [-0.175, 0.611], else [-0.349, 0.349]); the cannon fires only while the car-gun button is held. An AI vehicle with
/// status bits 0x18 (+0x36 & 0xF8) and a fire turns the cannon towards the fire (relative angle wrapped within pi of
/// the cannon heading; a step of timestep × 0.01, or straight onto it when closer) and sweeps its pitch with
/// `sin((time & 0xFFF) × 0.0015332) × 0.15`. Firing: from the cannon frame's position (or the vehicle's point
/// (0, 1.5, 1.8)), the direction (−sin h cos p, cos h cos p, sin p) turned into world axes; with a frame the start
/// moves 2.0 (SWAT) / 1.2 along it and on by the vehicle's speed at that point × timestep; the direction gets z +
/// `(rand() & 15) × 0.001` (0x72A1C9), is scaled by 0.4 / 0.5 and gets the vehicle's move speed (z × 0.3) added, then
/// `CWaterCannons::UpdateOne(this, start, direction)`.
#[hook(addr = 0x729B60, conv = thiscall, name = "CAutomobile::FireTruckControl", expect = "83 EC 34 53 56 57",
       this = "CAutomobile", unverified = "frontier2-6", route = "L2", hazards = "x87-transcendental,heap-writes",
       note = "rand site 0x72A1C9; FPATAN / FSIN / FCOS; crossing 4000 (the fire-truck crew)")]
pub fn fire_truck_control(this: GPtr<u8>, fire: GPtr<u8>) {
    let swat = rd::<u16>(this, 0x22) == SWAT_VAN;
    let ts = || fc(TIME_STEP);
    if this == find_player_vehicle(-1, 0) {
        let cam = usize::from(g::<u8>(0xB6F081).read()) * 0x238;
        if g::<u16>(0xB6F1A8 + cam).read() == 0x12 {
            let mut front = v3(GPtr::from_addr(0xB6F32C + cam));
            let mut local = [0f32; 3];
            let r = multiply3x3_vm(ptr(&mut local), ptr(&mut front), rd::<GPtr<u8>>(this, 0x14));
            let v = v3(r);
            let heading = x(v[0]).fchs().fpatan(x(v[1]));
            let flat = x(v[0]).fmul_m(v[0]).fadd(x(v[1]).fmul_m(v[1])).fsqrt();
            let pitch = x(v[2]).fpatan(flat);
            let base = fc(if swat { 0x872A44 } else { 0x872A38 });
            let target_pitch = f32_of(x(base).fadd(pitch));
            let h = wrap_near(heading, rd::<f32>(this, 0x94C));
            let d = h.fsub_m(rd::<f32>(this, 0x94C));
            let step = f32_of(x(ts()).fmul_m(fc(0x872A48)));
            let nh = if gt(d, x(step)) {
                x(step).fadd_m(rd::<f32>(this, 0x94C))
            } else if !ge_or_nan(d, x(step).fchs()) {
                x(rd::<f32>(this, 0x94C)).fsub_m(step)
            } else {
                h
            };
            wr::<f32>(this, 0x94C, f32_of(nh));
            let d = x(target_pitch).fsub_m(rd::<f32>(this, 0x950));
            let step = f32_of(x(ts()).fmul_m(fc(0x872A4C)));
            if gt(d, x(step)) {
                wr::<f32>(this, 0x950, f32_of(x(step).fadd_m(rd::<f32>(this, 0x950))));
            } else if !ge_or_nan(d, x(step).fchs()) {
                wr::<f32>(this, 0x950, f32_of(x(rd::<f32>(this, 0x950)).fsub_m(step)));
            } else {
                wr::<f32>(this, 0x950, target_pitch);
            }
        } else {
            let lr = get_car_gun_left_right(get_pad(0)) as i16;
            let v = xi(i32::from(lr)).fmul_m(ts()).fmul_m(fc(0x872A48)).fmul_m(fc(0x858B88));
            wr::<f32>(this, 0x94C, f32_of(v.fsubr_m(rd::<f32>(this, 0x94C))));
            let ud = get_car_gun_up_down(get_pad(0));
            let v = xi(i32::from(ud)).fmul_m(ts()).fmul_m(fc(0x872A4C)).fmul_m(fc(0x858B88));
            wr::<f32>(this, 0x950, f32_of(v.fadd_m(rd::<f32>(this, 0x950))));
        }
        let h = rd::<f32>(this, 0x94C);
        if !ge_or_nan(x(h), x(fc(0x858CC0))) {
            wr::<f32>(this, 0x94C, f32_of(x(h).fadd_m(fc(TWO_PI))));
        } else if gt(x(h), x(fc(PI))) {
            wr::<f32>(this, 0x94C, f32_of(x(rd::<f32>(this, 0x94C)).fsub_m(fc(TWO_PI))));
        }
        let (lo, hi) = if swat { (fc(0x872A40), fc(0x872A3C)) } else { (fc(0x872A34), fc(0x872A30)) };
        if gt(x(lo), x(rd::<f32>(this, 0x950))) {
            wr::<f32>(this, 0x950, lo);
        } else if gt(x(rd::<f32>(this, 0x950)), x(hi)) {
            wr::<f32>(this, 0x950, hi);
        }
        if pad_get_car_gun_fired(get_pad(0)) == 0 {
            return;
        }
    } else {
        if rd::<u8>(this, 0x36) & 0xF8 != 0x18 || fire.is_null() {
            return;
        }
        let m = rd::<GPtr<u8>>(this, 0x14);
        let pos = if m.is_null() { this.byte_add(4) } else { m.byte_add(0x30) };
        let a = x(rd::<f32>(fire, 4)).fsub_m(rd::<f32>(pos, 0)).fchs().fpatan(x(rd::<f32>(fire, 8)).fsub_m(rd::<f32>(pos, 4)));
        let vh = if m.is_null() {
            x(rd::<f32>(this, 0x10))
        } else {
            x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14)))
        };
        let a = if gt(a, x(fc(PI)).fadd(vh)) {
            a.fsub_m(fc(TWO_PI))
        } else if lt(a, x(fc(PI)).fsubr(vh)) {
            a.fadd_m(fc(TWO_PI))
        } else {
            a
        };
        let rel = wrap_near(a.fsub(vh), rd::<f32>(this, 0x94C));
        let f = f32_of(rel.fsub_m(rd::<f32>(this, 0x94C)));
        let step = f32_of(x(ts()).fmul_m(fc(0x858C58)));
        let nh = if lt(fabs(x(f)), x(step)) {
            rel
        } else if le_or_nan(x(f), x(0.0)) {
            x(rd::<f32>(this, 0x94C)).fsub_m(step)
        } else {
            x(step).fadd_m(rd::<f32>(this, 0x94C))
        };
        wr::<f32>(this, 0x94C, f32_of(nh));
        let t = (g::<u32>(0xB7CB84).read() & 0xFFF) as i32;
        wr::<f32>(this, 0x950, f32_of(xi(t).fmul_m(fc(0x863988)).fsin().fmul_m(fc(0x858FCC))));
    }
    let frame = rd::<GPtr<u8>>(this, 0x698);
    let mut w = [0f32; 3];
    let src = if frame.is_null() {
        let mut v = [0.0, 1.5, f32::from_bits(0x3FE6_6666)];
        matrix_mul_vec(ptr(&mut w), rd::<GPtr<u8>>(this, 0x14), ptr(&mut v))
    } else {
        frame_get_ltm(frame).byte_add(0x30)
    };
    let mut p = v3(src);
    let (hh, pp) = (rd::<f32>(this, 0x94C), rd::<f32>(this, 0x950));
    let cp = x(pp).fcos();
    let mut v = [
        f32_of(x(hh).fsin().fmul(cp).fchs()),
        f32_of(x(hh).fcos().fmul(cp)),
        f32_of(x(pp).fsin()),
    ];
    let mut w2 = [0f32; 3];
    let r = multiply3x3(ptr(&mut w2), rd::<GPtr<u8>>(this, 0x14), ptr(&mut v));
    let mut d = v3(r);
    if !frame.is_null() {
        let k = fc(if swat { 0x872A54 } else { 0x872A50 });
        let zk = f32_of(x(d[2]).fmul_m(k));
        p[0] = f32_of(x(d[0]).fmul_m(k).fadd_m(p[0]));
        p[1] = f32_of(x(d[1]).fmul_m(k).fadd_m(p[1]));
        p[2] = f32_of(x(zk).fadd_m(p[2]));
        let m = rd::<GPtr<u8>>(this, 0x14);
        let o = if m.is_null() { this.byte_add(4) } else { m.byte_add(0x30) };
        let rel = CVector {
            x: f32_of(x(p[0]).fsub_m(rd::<f32>(o, 0))),
            y: f32_of(x(p[1]).fsub_m(rd::<f32>(o, 4))),
            z: f32_of(x(p[2]).fsub_m(rd::<f32>(o, 8))),
        };
        let mut out = CVector::default();
        let s = physical_get_speed(this, GPtr::from_raw(&mut out), rel).cast::<u8>();
        let s = v3(s);
        let sz = f32_of(x(s[2]).fmul_m(ts()));
        p[0] = f32_of(x(s[0]).fmul_m(ts()).fadd_m(p[0]));
        p[1] = f32_of(x(s[1]).fmul_m(ts()).fadd_m(p[1]));
        p[2] = f32_of(x(sz).fadd_m(p[2]));
    }
    let r = rand_at(0x72A1C9) & 0xF;
    let z = xi(r).fmul_m(fc(0x858CDC)).fadd_m(d[2]);
    let s = x(fc(if swat { 0x872A5C } else { 0x872A58 }));
    let dxs = x(d[0]).fmul(s);
    d[1] = f32_of(x(d[1]).fmul(s));
    d[2] = f32_of(z.fmul(s));
    let ms = v3(this.byte_add(0x44));
    let msz = f32_of(x(ms[2]).fmul_m(fc(0x858C24)));
    d[0] = f32_of(x(ms[0]).fadd(dxs));
    d[1] = f32_of(x(ms[1]).fadd_m(d[1]));
    d[2] = f32_of(x(msz).fadd_m(d[2]));
    water_cannons_update_one(this, ptr(&mut p), ptr(&mut d));
}

pub const HOOKS: &[HookReg] = &[HOOK_FIRE_TRUCK_CONTROL];
