//! FRONTIER-2 turn 9, lane B (batch `frontier2-b21`): `CBike::BurstTyre` (0x6BEB20, vtable +0xB0). Ported
//! from the 1.0 US bytes with docs/re/specs/VEH/CBike.md as reading aid (gta-reversed has no body).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::fpu::fcom_parity;
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::common::{f32_of, rd, wr, x};

use sa_core::bindings::random_float_in_range;
use sa_pop::bindings::switch_vehicle_to_real_physics;
use sa_world::bindings::audio_add_event;
use sa_world::bindings::physical_apply_move_force_5429f0 as apply_move_force;
use sa_world::bindings::physical_apply_turn_force as apply_turn_force;
use sa_world::bindings::vector_scale as scalar_mul_vector;

/// `CBike::BurstTyre`'s random kick `GetRandomNumberInRange(-[0x8D322C], [0x8D322C]) * mass` along the
/// matrix row `right`: the product is stored as an f32 (`fst`), z multiplies the unrounded value, y and
/// x reload the f32.
fn kick(this: GPtr<u8>, mass_off: isize) -> CVector {
    let c = sa_core::field::gf(0x8D322C);
    let k: X87 = random_float_in_range(-c, c).fmul_m(rd::<f32>(this, mass_off));
    let kf = f32_of(k);
    let m = rd::<GPtr<u8>>(this, 0x14);
    let z = f32_of(k.fmul_m(rd::<f32>(m, 8)));
    let y = f32_of(x(kf).fmul_m(rd::<f32>(m, 4)));
    let xx = f32_of(x(kf).fmul_m(rd::<f32>(m, 0)));
    CVector::new(xx, y, z)
}

/// The matrix forward row as a `CVector`.
fn forward(this: GPtr<u8>) -> CVector {
    let m = rd::<GPtr<u8>>(this, 0x14);
    CVector::new(rd::<f32>(m, 0x10), rd::<f32>(m, 0x14), rd::<f32>(m, 0x18))
}

/// `CBike::BurstTyre(uint8 component, bool applyForces)` (0x6BEB20, thiscall, `ret 8`, AL; SEH frame
/// for two CEventKnockOffBike locals): not for tyre-proof bikes (+0x42B bit 7) or physical flag
/// 0x20000000. The wheel is the component with 13 -> 0 and 15 -> 1 (any other value used as is, no
/// bounds check, at +0x65C); an intact wheel bursts (status 1): audio event 0x5A, a simple-physics bike
/// switches to real physics, with forces a random push along `right` (x mass) and a random twist (x
/// turn mass, at the forward point); the result is true. Then, with a driver, the rider test runs on
/// the remapped wheel: 13 (unreachable after the remap; it would add CEventKnockOffBike to the driver
/// and passenger) or 14 (only for component 14: the rear ratios +0x728 / +0x72C, one below 1.0) with
/// the speed above [0x8D3230] (and [0x8D3234] for the player) applies a turn force `right * 2 *
/// ([0x8D322C] * turn mass)` at the forward point. The 13 branch is not ported: no input reaches it.
#[hook(addr = 0x6BEB20, conv = thiscall, name = "CBike::BurstTyre", expect = "6A FF 64 A1 00 00 00 00 68 63 83 84 00",
       this = "CBike", unverified = "frontier2-b21", route = "L2", hazards = "none",
       note = "GetRandomNumberInRange consumes rand(); the unreachable knock-off event branch (wheel 13) is not ported")]
pub fn bike_burst_tyre(this: GPtr<u8>, comp: u8, apply_forces: u8) -> u8 {
    if (rd::<u8>(this, 0x42B) as i8) < 0 || rd::<u32>(this, 0x40) & 0x2000_0000 != 0 {
        return 0;
    }
    let w = match comp {
        13 => 0,
        15 => 1,
        c => c,
    };
    let mut result = 0u8;
    let st = this.byte_add(0x65C + isize::from(w));
    if rd::<u8>(st, 0) == 0 {
        wr::<u8>(st, 0, 1);
        audio_add_event(this.byte_add(0x138), 0x5A, 0.0);
        if rd::<u8>(this, 0x36) & 0xF8 == 0x10 {
            switch_vehicle_to_real_physics(this);
        }
        if apply_forces != 0 {
            let f = kick(this, 0x8C);
            apply_move_force(this, f);
            let t = kick(this, 0x90);
            apply_turn_force(this, t, forward(this));
        }
        result = 1;
    }
    if rd::<GPtr<u8>>(this, 0x460).is_null() {
        return result;
    }
    let one = x(sa_core::field::gf(0x871284));
    // `fcomp 1.0; test ah, 5; jnp`: below 1.0
    let below = |off: isize| !fcom_parity(x(rd::<f32>(this, off)), one, 5);
    let go = match w {
        13 => below(0x720) || below(0x724),
        14 => below(0x728) || below(0x72C),
        _ => false,
    };
    if !go {
        return result;
    }
    let ms = this.byte_add(0x44);
    let (vx, vy, vz) = (x(rd::<f32>(ms, 0)), x(rd::<f32>(ms, 4)), x(rd::<f32>(ms, 8)));
    let s = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fsqrt();
    if sa_math::fpu::le_or_nan(s, x(sa_core::field::gf(0x8D3230))) {
        return result;
    }
    if rd::<u8>(this, 0x36) & 0xF8 == 0 && sa_math::fpu::le_or_nan(s, x(sa_core::field::gf(0x8D3234))) {
        return result;
    }
    if w == 13 {
        // unreachable: component 13 was remapped to wheel 0 above
        return result;
    }
    // fld [0x8D322C]; fmul turnMass; fadd st0, st0; fstp (f32 argument)
    let a = x(sa_core::field::gf(0x8D322C)).fmul_m(rd::<f32>(this, 0x90));
    let k = f32_of(a.fadd(a));
    let mut out = [0f32; 3];
    let r = scalar_mul_vector(sa_core::field::ptr_of(&mut out), k, rd::<GPtr<u8>>(this, 0x14));
    let force = CVector::new(rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8));
    apply_turn_force(this, force, forward(this));
    result
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_BIKE_BURST_TYRE];
