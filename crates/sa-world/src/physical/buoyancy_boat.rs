//! `cBuoyancy::ProcessBuoyancyBoat` (0x6C3030, batch `samp2-phys-1`): boat buoyancy over a 3x3 grid.
//! Ported from the 1.0 US bytes (gta-reversed `Buoyancy.cpp` as a reading aid). Unverified
//! (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use super::buoyancy::{POINT_VOL_MULTIPLIER, get_water_level, pre_calc_setup, simple_sum_buoyancy_data_call};
use super::forces::{apply_turn_force_call, get_speed_call};
use super::shared::*;
use crate::world::common::x;

orig! {
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0, thiscall)
    fn matrix_assign(m: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BBC0 => unported;
    /// `cBuoyancy::FindWaterLevelNorm(const CVector&, CVector*, tWaterLevel*, CVector*)` (0x6C28C0)
    fn find_water_level_norm_call(b: GPtr<u8>, initial: GPtr<u8>, offset: GPtr<u8>, state: GPtr<u32>, normal: GPtr<u8>) = thiscall 0x6C28C0 => super::buoyancy::find_water_level_norm;
}

/// `fPointVolMultiplier` tables (3x3 floats, `[y][x]`): speed boats 0x8D335C, dinghies 0x8D3338, the
/// sail boat 0x8D3314, others 0x8D32CC; the normal's z bias 2.0 (0x8D3384, `.data`).
const VOL_SPEED: usize = 0x8D_335C;
const VOL_DINGHY: usize = 0x8D_3338;
const VOL_SAIL: usize = 0x8D_3314;
const VOL_DEFAULT: usize = 0x8D_32CC;
const NORMAL_BIAS: usize = 0x8D_3384;

fn gf(a: usize) -> f32 {
    GPtr::<f32>::from_addr(a).read()
}

fn cv(p: GPtr<CVector>) -> CVector {
    let a: [f32; 3] = p.cast::<[f32; 3]>().read();
    CVector { x: a[0], y: a[1], z: a[2] }
}

fn vp(v: &mut CVector) -> GPtr<u8> {
    GPtr::from_raw(v as *mut CVector).cast()
}

/// `cBuoyancy::ProcessBuoyancyBoat(CVehicle* veh, float buoyancy, CVector* turnPoint, CVector* force, bool
/// underwater)` (0x6C3030, thiscall `ret 0x14`, AL).
///
/// `GetWaterLevel(pos, &this->waterLevel, veh->bTouchingWater (flags bit 27, the shifted dword pushed),
/// null)`; none: false. The entity matrix is copied, `PreCalcSetup(veh, buoyancy)`, `hr = 1 / ((max.z -
/// min.z) * 9)`. For x = min.x + k*offset.x (k = 0..2, outer, x accumulated in f32) and y = min.y +
/// j*offset.y (inner): `FindWaterLevelNorm(initial, &(x, y, 0), &state, &n)` with n = (0, 0, 1); `n' = (n +
/// (2*0, 0, 2)) * (1 / (2 + 1))` (the bias from 0x8D3384; x/y kept at 80 bits before the scale); the
/// point volume multiplier from the model's table `[j][k]` (446, 452, 493, 595 speed; 472, 473 dinghy;
/// 484 sail; else default). Points not above water: `c = (timeStep * this->buoyancy) * (SimpleSum(point,
/// state) * hr)` (f32), the point's world offset `Multiply3x3(veh matrix, (x, y, 0))`, its speed
/// `GetSpeed(offset)`, `w = max(1 - ((s.z*n.z + s.y*n.y) + s.x*n.x) * handling.damping, 0) * c`;
/// `force.z += w`; unless underwater `ApplyTurnForce(n' * w, Multiply3x3(this->matrix, point))`. Then the
/// immersion `*= hr`, `*turnPoint = Multiply3x3(this->matrix, this->turnPoint)`; true when processing a
/// boat (+0xBA), else the in-water flag (+0x98).
#[hook(addr = 0x6C3030, conv = thiscall, name = "cBuoyancy::ProcessBuoyancyBoat", this = "cBuoyancy",
       expect = "81 EC 94 00 00 00",
       unverified = "samp2-phys-1", route = "L2", hazards = "heap-writes", recs = "boat",
       note = "writes fPointVolMultiplier (0x8D32C8) per point")]
pub fn process_buoyancy_boat(this: GPtr<u8>, veh: GPtr<u8>, buoyancy: f32, turn_point: GPtr<u8>, force: GPtr<u8>,
    underwater: u32) -> u8 {
    let p = pos_ptr(veh);
    let touching = (ru(veh, 0x40) >> 0x1B) & 0xFFFF_FF01;
    if get_water_level(rf(p, 0), rf(p, 4), rf(p, 8), this.byte_add(0x64).cast(), touching, GPtr::NULL) == 0 {
        return 0;
    }
    matrix_assign(this.byte_add(0xC), rp(veh, 0x14));
    pre_calc_setup(this, veh, buoyancy);
    let hr = x(rf(this, 0x78)).fsub_m(rf(this, 0x84)).fmul_m(cf(0x4110_0000)).fdivr_m(1.0).to_f32();
    let mut cx = rf(this, 0x7C);
    for k in 0..3usize {
        let mut cy = x(rf(this, 0x80));
        for j in 0..3usize {
            let y = cy.to_f32();
            let mut point = CVector { x: cx, y, z: 0.0 };
            let mut n = CVector { x: 0.0, y: 0.0, z: 1.0 };
            let mut state = 0u32;
            find_water_level_norm_call(this, this.byte_add(0x58), vp(&mut point), GPtr::from_raw(&mut state), vp(&mut n));
            let bias = gf(NORMAL_BIAS);
            let a = x(bias).fmul_m(0.0);
            let a32 = a.to_f32();
            let nx = a.fadd_m(n.x);
            let ny = x(a32).fadd_m(n.y);
            let nz = x(gf(NORMAL_BIAS)).fadd_m(n.z).to_f32();
            let third = x(gf(NORMAL_BIAS)).fadd_m(1.0).fdivr_m(1.0).to_f32();
            n.x = nx.fmul_m(third).to_f32();
            n.y = ny.fmul_m(third).to_f32();
            n.z = x(nz).fmul_m(third).to_f32();
            let table = match rh(veh, 0x22) {
                446 | 452 | 493 | 595 => VOL_SPEED,
                472 | 473 => VOL_DINGHY,
                484 => VOL_SAIL,
                _ => VOL_DEFAULT,
            };
            POINT_VOL_MULTIPLIER.set(gf(table + j * 12 + k * 4));
            if state != 0 {
                let d = X87::from_f64_exact(simple_sum_buoyancy_data_call(this, vp(&mut point), state)).fmul_m(hr);
                let c = x(sa_core::timer::TIME_STEP.get()).fmul_m(rf(this, 0x6C)).fmul(d).to_f32();
                let mut off = CVector { x: cx, y, z: 0.0 };
                let mut out = CVector::default();
                let r = multiply3x3(GPtr::from_raw(&mut out), rp(veh, 0x14), GPtr::from_raw(&mut off));
                let world = cv(r);
                let mut sp = CVector::default();
                let s = cv(get_speed_call(veh, GPtr::from_raw(&mut sp), world));
                let dot = x(s.z).fmul_m(n.z).fadd(x(s.y).fmul_m(n.y)).fadd(x(s.x).fmul_m(n.x));
                let mut w = dot.fmul_m(rf(rp(veh, 0x384), 0xB0)).fsubr_m(1.0);
                if !x(0.0).fcom(w).test_any(0x41) {
                    w = x(0.0);
                }
                let w = w.fmul_m(c);
                wf(force, 8, w.fadd_m(rf(force, 8)).to_f32());
                if underwater as u8 == 0 {
                    let t = CVector { x: x(n.x).fmul(w).to_f32(), y: x(n.y).fmul(w).to_f32(), z: x(n.z).fmul(w).to_f32() };
                    let mut o2 = CVector::default();
                    let at = cv(multiply3x3(GPtr::from_raw(&mut o2), this.byte_add(0xC), GPtr::from_raw(&mut point)));
                    apply_turn_force_call(veh, t, at);
                }
            }
            cy = x(cy.to_f32()).fadd_m(rf(this, 0xA0));
        }
        cx = x(cx).fadd_m(rf(this, 0x9C)).to_f32();
    }
    wf(this, 0xBC, x(hr).fmul_m(rf(this, 0xBC)).to_f32());
    let mut out = CVector::default();
    let r = multiply3x3(GPtr::from_raw(&mut out), this.byte_add(0xC), this.byte_add(0xC0).cast());
    let t = cv(r);
    turn_point.cast::<[f32; 3]>().write([t.x, t.y, t.z]);
    if rb(this, 0xBA) != 0 { 1 } else { rb(this, 0x98) }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_BUOYANCY_BOAT];
