//! PORT-VEH-3 (batch `veh3-3`): `CAutomobile::BurstTyre` and `CAutomobile::Fix`. Ported from the
//! 1.0 US bytes; gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::common::{rd, wr};

use crate::bindings::dm_reset_damage_status;
use sa_world::bindings::dm_set_door_status;
use sa_pop::bindings::dm_set_wheel_status;
use sa_world::bindings::get_wheel_status as dm_get_wheel_status;
use sa_core::bindings::matrix_dtor;
use sa_core::bindings::matrix_set_translate;
use sa_core::bindings::matrix_update_rw;
use sa_core::bindings::random_float_in_range;
use sa_core::bindings::rp_clump_for_all_atomics;
use sa_core::bindings::stats_increment_stat;
use sa_world::bindings::audio_add_event;
use sa_core::bindings::matrix_ctor_rw;
use sa_world::bindings::switch_vehicle_to_real_physics;
use sa_world::bindings::vector_scale as scalar_mul_vector;
orig! {
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0)
    fn apply_move_force(e: GPtr<u8>, force: CVector) = thiscall 0x5429F0 => sa_world::physical::forces::apply_move_force;
    /// `CPhysical::ApplyTurnForce(CVector, CVector)` (0x542A50)
    fn apply_turn_force(e: GPtr<u8>, force: CVector, point: CVector) = thiscall 0x542A50 => sa_world::physical::forces::apply_turn_force;
}

/// `matrix row * (rand(-0.03, 0.03) * mass_at)` (a temporary `CVector`).
fn kick(this: GPtr<u8>, mass_off: isize) -> CVector {
    let c = f32::from_bits(0x3CF5_C28F);
    let k = X87::from_f64_exact(random_float_in_range(-c, c)).fmul_m(rd::<f32>(this, mass_off)).to_f32();
    let mut out = [0f32; 3];
    let r = scalar_mul_vector(GPtr::from_raw(&mut out as *mut [f32; 3]).cast(), k, rd::<GPtr<u8>>(this, 0x14));
    CVector::new(rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8))
}

/// `CAutomobile::BurstTyre(uint8 component, bool applyForces)` (0x6A32B0): not for the Rhino (432),
/// tyre-proof cars (+0x42B bit 7) or physical flag 0x20000000. The wheel is the component (13/14/15/16
/// -> 0/2/1/3, any other value used as is); an intact wheel (status 0) bursts (status 1): stat 129,
/// audio event 0x5A, a simple-physics car (state 2) switches to real physics, and with forces a
/// random push along `right` (`rand(-0.03, 0.03)` x mass) and a random twist (x turn mass, at the
/// forward point). True when it burst.
#[hook(addr = 0x6A32B0, conv = thiscall, name = "CAutomobile::BurstTyre", this = "CAutomobile",
       expect = "83 EC 0C 53 56",
       unverified = "veh3-3", route = "L2", hazards = "none",
       recs = "driving", note = "GetRandomNumberInRange consumes rand() (its own call site)")]
pub fn automobile_burst_tyre(this: GPtr<u8>, comp: u8, apply_forces: u8) -> u8 {
    if rd::<u16>(this, 0x22) == 0x1B0 || (rd::<u8>(this, 0x42B) as i8) < 0 || rd::<u32>(this, 0x40) & 0x2000_0000 != 0 {
        return 0;
    }
    let w = match comp {
        13 => 0,
        14 => 2,
        15 => 1,
        16 => 3,
        c => c,
    };
    let dm = this.byte_add(0x5A0);
    if dm_get_wheel_status(dm, i32::from(w) as u32) != 0 {
        return 0;
    }
    dm_set_wheel_status(dm, i32::from(w), 1);
    stats_increment_stat(0x81, 1.0);
    audio_add_event(this.byte_add(0x138), 0x5A, 0.0);
    if rd::<u8>(this, 0x36) & 0xF8 == 0x10 {
        switch_vehicle_to_real_physics(this);
    }
    if apply_forces != 0 {
        let f = kick(this, 0x8C);
        apply_move_force(this, f);
        let t = kick(this, 0x90);
        let m = rd::<GPtr<u8>>(this, 0x14);
        let p = CVector::new(rd::<f32>(m, 0x10), rd::<f32>(m, 0x14), rd::<f32>(m, 0x18));
        apply_turn_force(this, t, p);
        wr::<u8>(this, 0xB8, 0);
    }
    1
}

/// `CAutomobile::Fix()` (0x6A3440): damage reset; handling model flag 0x100 marks the doors 2..5
/// missing; +0x42A bit 0 cleared; every atomic through `HideAllComponentsAtomicCB` (0x4C7790, flag
/// 2); the 17 nodes from +0x668 get their rotation reset; the four wheels status 0; the four
/// bouncing panels (+0x6AC, 0x20 each) node id -1.
#[hook(addr = 0x6A3440, conv = thiscall, name = "CAutomobile::Fix", this = "CAutomobile",
       expect = "6A FF 64 A1 00 00 00 00",
       unverified = "veh3-3", route = "L2", hazards = "heap-writes",
       note = "writes the RwFrame modelling matrices and the atomic flags; the SEH frame is not reproduced")]
pub fn automobile_fix(this: GPtr<u8>) {
    let dm = this.byte_add(0x5A0);
    dm_reset_damage_status(dm);
    if rd::<u32>(rd::<GPtr<u8>>(this, 0x384), 0xCC) & 0x100 != 0 {
        for d in 2..6 {
            dm_set_door_status(dm, d as u32, 4);
        }
    }
    wr::<u8>(this, 0x42A, rd::<u8>(this, 0x42A) & 0xFE);
    rp_clump_for_all_atomics(rd::<GPtr<u8>>(this, 0x18), 0x004C_7790, GPtr::from_addr(2));
    for k in 0..0x11isize {
        let node = rd::<GPtr<u8>>(this, 0x668 + 4 * k);
        if node.is_null() {
            continue;
        }
        let mut buf = [0u32; 0x12];
        let m: GPtr<u8> = GPtr::from_raw(&mut buf as *mut [u32; 0x12]).cast();
        matrix_ctor_rw(m, node.byte_add(0x10), 0);
        matrix_set_translate(m, f32::from_bits(rd::<u32>(m, 0x30)), f32::from_bits(rd::<u32>(m, 0x34)), f32::from_bits(rd::<u32>(m, 0x38)));
        matrix_update_rw(m);
        matrix_dtor(m);
    }
    for w in 0..4 {
        dm_set_wheel_status(dm, w, 0);
    }
    for k in 0..4isize {
        wr::<u16>(this, 0x6AC + 0x20 * k, 0xFFFF);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_AUTOMOBILE_BURST_TYRE, HOOK_AUTOMOBILE_FIX];
