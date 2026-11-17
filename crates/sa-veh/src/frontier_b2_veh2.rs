//! FRONTIER-2 turn 2, lane B (batch `frontier2-b9`): more vehicle functions on the phase B frontier (plane
//! ordnance). Ported from the 1.0 US bytes; gta-reversed as a reading aid only (the bytes win). Unverified
//! (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr, x};
use sa_core::bindings::get_pad;
use sa_stream::bindings::ped_is_player;
use sa_world::bindings::{add_projectile, pad_start_shake};
use sa_world::world::common::entity_pos;

orig! {
    /// `CVehicle::GetPlaneOrdnanceRateOfFire(eOrdnanceType)` (0x6D4590)
    pub(crate) fn get_plane_ordnance_rate_of_fire(v: GPtr<u8>, ty: u32) -> i32 = thiscall 0x6D4590 => crate::vehicle_fire_rates_upgrades::vehicle_get_plane_ordnance_rate_of_fire;
    /// `CVehicle::GetPlaneOrdnancePosition(CVector* out, eOrdnanceType)` (0x6D46E0)
    pub(crate) fn get_plane_ordnance_position(v: GPtr<u8>, out: GPtr<u8>, ty: u32) -> GPtr<u8> = thiscall 0x6D46E0 => crate::frontier_b2_veh2::vehicle_get_plane_ordnance_position;
}

/// `CVehicle::GetPlaneOrdnancePosition(CVector* out, eOrdnanceType type)` (0x6D46E0, thiscall, `ret 8`, returns `out`):
/// the vehicle structure's weapon position (model info +0x5C, +0x9C) when any coordinate is non-zero (x87 compares
/// with 0.0; NaN counts as non-zero); otherwise by model (jump table 0x6D4848 / 0x6D4864): 425 -> 0x8D3640, 447 ->
/// 0xC1CC38, 464 -> 0xC1CC50, 476 -> 0x8D364C, 520 -> 0x8D3658 for type 1, 0x8D3664 for type 2 (else the zero vector),
/// 576 -> 0xC1CC44, any other model 0.
#[hook(addr = 0x6D46E0, conv = thiscall, name = "CVehicle::GetPlaneOrdnancePosition", expect = "83 EC 0C 56 57 0F BF 79 22",
       this = "CVehicle", unverified = "frontier2-b9", route = "L2", hazards = "none")]
pub fn vehicle_get_plane_ordnance_position(this: GPtr<u8>, out: GPtr<u8>, ty: u32) -> GPtr<u8> {
    let model = i32::from(rd::<i16>(this, 0x22));
    let d = rd::<GPtr<u8>>(crate::common::model_info(model), 0x5C).byte_add(0x9C);
    let mut v = [rd::<u32>(d, 0), rd::<u32>(d, 4), rd::<u32>(d, 8)];
    let zero = x(g::<f32>(0x85_8B50).read());
    let nonzero = |b: u32| x(f32::from_bits(b)).fcom(zero).test_parity(0x44);
    if !nonzero(v[0]) && !nonzero(v[1]) && !nonzero(v[2]) {
        let src = match model {
            0x1A9 => Some(0x8D_3640),
            0x1BF => Some(0xC1_CC38),
            0x1D0 => Some(0xC1_CC50),
            0x1DC => Some(0x8D_364C),
            0x208 if ty == 1 => Some(0x8D_3658),
            0x208 if ty == 2 => Some(0x8D_3664),
            0x208 => None,
            0x240 => Some(0xC1_CC44),
            _ => Some(0),
        };
        match src {
            Some(0) => v = [0; 3],
            Some(a) => v = [g::<u32>(a).read(), g::<u32>(a + 4).read(), g::<u32>(a + 8).read()],
            None => {}
        }
    }
    for (k, b) in v.iter().enumerate() {
        wr::<u32>(out, 4 * k as isize, *b);
    }
    out
}

/// `CVehicle::PossiblyDropFreeFallBombForPlayer(eOrdnanceType type, bool checkRate)` (0x6E07E0, thiscall, `ret 8`; the
/// bytes fire the ordnance): with `checkRate`, nothing until the rate of fire has passed since the last shot (+0x4FC
/// for type 1, else +0x500; unsigned). The ordnance position (`GetPlaneOrdnancePosition`) is added to the vehicle's
/// position, its x negated while the side bits (2..3 of +0x4A9) are set; the side bits become 1 (bit 2) when they were
/// 0, else 0; `CProjectileInfo::AddProjectile(this, 21, position, 0, &matrix.forward, null)`; a player driver's pad
/// (player 0 or 1, +0x598) shakes (240, 160, 0); the shot time (+0x4FC / +0x500) = now.
#[hook(addr = 0x6E07E0, conv = thiscall, name = "CVehicle::PossiblyDropFreeFallBombForPlayer", expect = "83 EC 18 53 8B 5C 24 20",
       this = "CVehicle", unverified = "frontier2-b9", route = "L2", hazards = "heap-writes")]
pub fn vehicle_possibly_drop_free_fall_bomb(this: GPtr<u8>, ty: u32, check: u32) {
    let last = if ty == 1 { rd::<u32>(this, 0x4FC) } else { rd::<u32>(this, 0x500) };
    if check as u8 != 0 {
        let r = get_plane_ordnance_rate_of_fire(this, ty);
        if g::<u32>(0xB7_CB84).read() <= (r as u32).wrapping_add(last) {
            return;
        }
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let p = entity_pos(this);
    let a = [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)];
    let mut b = [0f32; 3];
    let bp = GPtr::from_raw(b.as_mut_ptr()).cast::<u8>();
    get_plane_ordnance_position(this, bp, ty);
    let flags = rd::<u8>(this, 0x4A9);
    let side = (flags >> 2) & 3;
    let bx = if side != 0 { x(rd(bp, 0)).fchs() } else { x(rd(bp, 0)) };
    let px = x(a[0]).fadd(bx).to_f32();
    let py = x(rd(bp, 4)).fadd_m(a[1]).to_f32();
    let new = ((u8::from(side == 0) << 2) ^ flags) & 0xC ^ flags;
    let pz = x(rd(bp, 8)).fadd_m(a[2]).to_f32();
    wr::<u8>(this, 0x4A9, new);
    add_projectile(this, 0x15, px, py, pz, 0.0, m.byte_add(0x10), GPtr::NULL);
    shake_and_stamp(this, ty);
}

/// The ordnance shot's tail: a player driver's pad (player 0 or 1, +0x598) shakes (240, 160, 0); the shot time
/// (+0x4FC for type 1, else +0x500) = now.
pub(crate) fn shake_and_stamp(this: GPtr<u8>, ty: u32) {
    let d = rd::<GPtr<u8>>(this, 0x460);
    if !d.is_null() && ped_is_player(d) != 0 {
        let n = rd::<u32>(rd::<GPtr<u8>>(this, 0x460), 0x598);
        if n == 0 || n == 1 {
            pad_start_shake(get_pad(n as i32), 0xF0, 0xA0, 0);
        }
    }
    let now = g::<u32>(0xB7_CB84).read();
    if ty == 1 {
        wr::<u32>(this, 0x4FC, now);
    } else {
        wr::<u32>(this, 0x500, now);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_VEHICLE_POSSIBLY_DROP_FREE_FALL_BOMB, HOOK_VEHICLE_GET_PLANE_ORDNANCE_POSITION];
