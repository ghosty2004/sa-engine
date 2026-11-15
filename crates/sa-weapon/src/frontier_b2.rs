//! FRONTIER-2 turn 2, lane B (batch `frontier2-b8`): weapon functions on the phase B frontier (the frenzy target
//! search). Ported from the 1.0 US bytes; gta-reversed as a reading aid only (the bytes win). Unverified
//! (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::field::rd;
use sa_core::hook;
use sa_core::mem::GPtr;

use crate::common::{g, x, xi};
use sa_core::bindings::{calc_screen_coors, find_player_vehicle};
use sa_pop::misc2::this_vehicle_should_be_killed_for_frenzy;
use sa_pop::misc3::this_ped_should_be_killed_for_frenzy;
use sa_world::world::common::entity_pos;

/// The pool walk of `FindNearestTargetEntityWithScreenCoors` for one pool (from the last slot down): each candidate
/// `ok` accepts whose position projects on screen (`CSprite::CalcScreenCoors(pos, &s, &w, &h, true, true)`) with a
/// screen distance `sqrt(dy*dy + dx*dx)` below the best so far and a world distance (`(dz*dz + dy*dy) + dx*dx`)
/// below `range * range` becomes the best; with `outX` the screen position is written back as `s / (size * 0.5) - 1`.
#[allow(clippy::too_many_arguments)]
fn scan(pool: GPtr<u8>, stride: u32, ok: &dyn Fn(GPtr<u8>) -> bool, sx: f32, sy: f32, range: f32, p: [f32; 3], out_x: GPtr<f32>, out_y: GPtr<f32>, best: &mut (f32, GPtr<u8>)) {
    let mut i = rd::<u32>(pool, 8);
    while i != 0 {
        i -= 1;
        if rd::<i8>(rd::<GPtr<u8>>(pool, 4), i as isize) < 0 {
            continue;
        }
        let e: GPtr<u8> = GPtr::from_addr(rd::<u32>(pool, 0).wrapping_add(i.wrapping_mul(stride)) as usize);
        if e.is_null() || !ok(e) {
            continue;
        }
        let mut s = [0f32; 3];
        let mut wh = [0f32; 2];
        let sp = GPtr::from_raw(s.as_mut_ptr());
        let whp = GPtr::from_raw(wh.as_mut_ptr());
        if calc_screen_coors(entity_pos(e).cast(), sp, whp.byte_add(4), whp, 1, 1) == 0 {
            continue;
        }
        let dx = x(sp.read()).fsub_m(sx);
        let dy = x(sp.byte_add(4).read()).fsub_m(sy);
        let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
        if d.fcom(x(best.0)).test_parity(5) {
            continue;
        }
        let q = entity_pos(e);
        let wx = x(rd(q, 0)).fsub_m(p[0]);
        let wy = x(rd(q, 4)).fsub_m(p[1]);
        let wz = x(rd(q, 8)).fsub_m(p[2]);
        let s2 = wz.fmul(wz).fadd(wy.fmul(wy)).fadd(wx.fmul(wx));
        if x(range).fmul_m(range).fcom(s2).test_any(0x41) {
            continue;
        }
        *best = (d.to_f32(), e);
        if !out_x.is_null() {
            let half = |a: usize| xi(g::<i32>(a).read()).fmul_m(g::<f32>(0x85_8B8C).read());
            out_x.write(half(0xC1_7044).fdivr_m(sp.read()).fsub_m(g::<f32>(0x85_8624).read()).to_f32());
            out_y.write(half(0xC1_7048).fdivr_m(sp.byte_add(4).read()).fsub_m(g::<f32>(0x85_8624).read()).to_f32());
        }
    }
}

/// `CWeapon::FindNearestTargetEntityWithScreenCoors(float screenX, float screenY, float range, CVector point, float*
/// outX, float* outY)` (0x73E240, cdecl): the best screen distance starts at `width * [0x863E0C]` (0xC17044 the screen
/// width, 0xC17048 the height); the screen point is converted to pixels (`(x + 1) * width * 0.5`, `(y + 1) * height *
/// 0.5`, written over the caller's `screenX` / `screenY` slots); the peds (not dead (+0x530 != 55), not in a vehicle
/// (+0x46C bit 0x100), `CDarkel::ThisPedShouldBeKilledForFrenzy`) then the vehicles (not the player's,
/// `ThisVehicleShouldBeKilledForFrenzy`) are searched ([`scan`]). Returns the best entity or null.
#[hook(addr = 0x73E240, conv = cdecl, name = "CWeapon::FindNearestTargetEntityWithScreenCoors", expect = "83 EC 1C DB 05 44 70 C1 00",
       unverified = "frontier2-b8", route = "L2", hazards = "x87-transcendental,stack-residue",
       note = "FSQRT screen distance; the pixel coordinates are written over the caller's screenX / screenY slots")]
#[allow(clippy::too_many_arguments)]
pub fn weapon_find_nearest_target_entity_with_screen_coors(sx: f32, sy: f32, range: f32, px: f32, py: f32, pz: f32, out_x: GPtr<f32>, out_y: GPtr<f32>) -> GPtr<u8> {
    let w = xi(g::<i32>(0xC1_7044).read());
    let maxd = x(g::<f32>(0x86_3E0C).read()).fmul(w).to_f32();
    let half = g::<f32>(0x85_8B8C).read();
    let one = g::<f32>(0x85_8624).read();
    let sx = x(sx).fadd_m(one).fmul(w).fmul_m(half).to_f32();
    let sy = x(sy).fadd_m(one).fmul(xi(g::<i32>(0xC1_7048).read())).fmul_m(half).to_f32();
    let p = [px, py, pz];
    let mut best = (maxd, GPtr::<u8>::NULL);
    let peds = g::<GPtr<u8>>(0xB7_4490).read();
    let ped_ok = |e: GPtr<u8>| {
        rd::<u32>(e, 0x530) != 0x37 && rd::<u32>(e, 0x46C) & 0x100 == 0 && this_ped_should_be_killed_for_frenzy(e) != 0
    };
    scan(peds, 0x7C4, &ped_ok, sx, sy, range, p, out_x, out_y, &mut best);
    let vehs = g::<GPtr<u8>>(0xB7_4494).read();
    let veh_ok = |e: GPtr<u8>| e != find_player_vehicle(-1, 0) && this_vehicle_should_be_killed_for_frenzy(e) != 0;
    scan(vehs, 0xA18, &veh_ok, sx, sy, range, p, out_x, out_y, &mut best);
    best.1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_WEAPON_FIND_NEAREST_TARGET_ENTITY_WITH_SCREEN_COORS];
