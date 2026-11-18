//! The auto-aim of the instant-hit shots: `CWeapon::DoDoomAiming` (NPCs on foot) and `CWeapon::DoDriveByAutoAiming`
//! (drive-bys and vehicle guns).

use sa_core::abi::HookReg;
use sa_core::field::{rd, rd3f};
use sa_core::field::gf;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{x, X87};
use crate::weapon::world_process_line_of_sight;
use crate::weapon_fire::ensure_matrix;
use sa_world::bindings::{dist_to_line, find_objects_in_range, placeable_get_matrix};
use sa_world::world::common::entity_pos;

orig! {
    /// `CPed::CanSeeEntity(CEntity*, float threshold)` (0x5E0730, AL)
    fn ped_can_see_entity(ped: GPtr<u8>, e: GPtr<u8>, thr: f32) -> u8 = thiscall 0x5E0730 => unported;
    /// `CVehicle::GetPlaneGunsAutoAimAngle()` (0x6D3E00, ST0)
    fn get_plane_guns_auto_aim_angle(v: GPtr<u8>) -> X87 = reg("args = ecx") 0x6D3E00 => unported;
}

/// 10000.0 (immediate 0x461C4000), 9000.0 (0x872C30), 0.0 (0x858B50), 1.5 (0x858CE8), 0.3 (0x858C24), 0.8 (0x858C98),
/// 0.15 (0x858FCC), 5.0 (0x858C80), 0.5 (0x858B8C), pi / 180 (0x8595EC), 2.5 (0x858FA0).
const K_FAR: u32 = 0x461C_4000;
const K_9000: usize = 0x87_2C30;
const K_0: usize = 0x85_8B50;
const K_1_5: usize = 0x85_8CE8;
const K_0_3: usize = 0x85_8C24;
const K_0_8: usize = 0x85_8C98;
const K_0_15: usize = 0x85_8FCC;
const K_5: usize = 0x85_8C80;
const K_0_5: usize = 0x85_8B8C;
const K_DEG: usize = 0x85_95EC;
const K_2_5: usize = 0x85_8FA0;



/// `sqrt((dz*dz + dy*dy) + dx*dx)` of `a - b` (x87, each difference an extended register).
fn dist_zyx(a: [f32; 3], b: [f32; 3]) -> X87 {
    let d = |k: usize| x(a[k]).fsub_m(b[k]);
    let (dx, dy, dz) = (d(0), d(1), d(2));
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt()
}

/// `CWeapon::DoDoomAiming(CEntity* owner, CVector* start, CVector* end)` (0x73CDC0, cdecl). Candidates:
/// `FindObjectsInRange(*start, |*end - *start|, 2D, &n, 15, list, vehicles and peds)`; for each (i16 loop) that is not
/// the owner, that the owner can see (`CanSeeEntity(c, pi / 8)`; gta-reversed skips the visible ones) and whose status
/// is not 5, 6 or 7: both matrices ensured (owner first), `d2 = sqrt(dx*dx + dy*dy)` of owner - c (stored f32),
/// `az = |owner.z - c.z|` (through GetMatrix, negated when below 0, kept extended); with `1.5 * az < d2` the 3D
/// distance `sqrt(az*az + d2*d2)` below the closest (from 10000) makes it the best. With the closest below 9000 and no
/// building between `*start` and the best's position (`ProcessLineOfSight` buildings only, shoot-through): `end->z =
/// (L / D) * (tz - s.z) + s.z` with `L` / `D` the 2D lengths (y term first) of `end - start` / `T - start` and `tz =
/// T.z + 0.3` (- 0.8 for a ducking ped).
#[hook(addr = 0x73CDC0, conv = cdecl, name = "CWeapon::DoDoomAiming", expect = "81 EC 94 00 00 00",
       unverified = "frontier2-a26", route = "L2", hazards = "heap-writes",
       note = "may allocate the owner's / a candidate's matrix (ensure-matrix pattern)")]
pub fn do_doom_aiming(owner: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>) {
    let s = rd3f(start);
    let r = dist_zyx(rd3f(end), s).to_f32();
    let mut list = [GPtr::<u8>::NULL; 16];
    let mut count = 0i16;
    find_objects_in_range(
        start, r, 1, GPtr::from_raw(&mut count as *mut i16).cast(), 15,
        GPtr::from_raw(list.as_mut_ptr()).cast(), 0, 1, 1, 0, 0,
    );
    let mut closest = f32::from_bits(K_FAR);
    if count <= 0 {
        return;
    }
    let mut best = 0usize;
    let mut i = 0i16;
    while i < count {
        let k = i as usize;
        i = i.wrapping_add(1);
        let c = list[k];
        if c == owner || ped_can_see_entity(owner, c, f32::from_bits(0x3EC9_0FDB)) == 0 {
            continue;
        }
        if matches!(rd::<u8>(list[k], 0x36) >> 3, 5..=7) {
            continue;
        }
        ensure_matrix(owner);
        ensure_matrix(list[k]);
        let om = rd::<GPtr<u8>>(owner, 0x14);
        let cm = rd::<GPtr<u8>>(list[k], 0x14);
        let dx = x(rd::<f32>(om, 0x30)).fsub_m(rd::<f32>(cm, 0x30));
        let dy = x(rd::<f32>(om, 0x34)).fsub_m(rd::<f32>(cm, 0x34));
        let d2 = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt().to_f32();
        let v = x(rd::<f32>(om, 0x38)).fsub_m(rd::<f32>(cm, 0x38));
        let neg = !v.fcom(x(gf(K_0))).test_parity(0x05);
        let a = rd::<f32>(placeable_get_matrix(owner), 0x38);
        let az = x(a).fsub_m(rd::<f32>(placeable_get_matrix(list[k]), 0x38));
        let az = if neg { az.fchs() } else { az };
        if x(gf(K_1_5)).fmul(az).fcom(x(d2)).test_parity(0x05) {
            continue;
        }
        let d3 = az.fmul(az).fadd(x(d2).fmul_m(d2)).fsqrt();
        if d3.fcom(x(closest)).test_parity(0x05) {
            continue;
        }
        closest = d3.to_f32();
        best = k;
    }
    if x(closest).fcom(x(gf(K_9000))).test_parity(0x05) {
        return;
    }
    let t = list[best];
    let mut cp = [0u32; 11];
    let mut hit = GPtr::<u8>::NULL;
    let hit_any = world_process_line_of_sight(
        start, entity_pos(t), GPtr::from_raw(cp.as_mut_ptr()).cast(),
        GPtr::from_raw(&mut hit as *mut GPtr<u8>).cast(), 1, 0, 0, 0, 0, 0, 0, 1,
    );
    if hit_any != 0 {
        return;
    }
    let e = rd3f(end);
    let ey = x(e[1]).fsub_m(s[1]);
    let ex = x(e[0]).fsub_m(s[0]);
    let l = ey.fmul(ey).fadd(ex.fmul(ex)).fsqrt();
    let tp = rd3f(entity_pos(t));
    let ty = x(tp[1]).fsub_m(s[1]);
    let tx = x(tp[0]).fsub_m(s[0]);
    let dd = ty.fmul(ty).fadd(tx.fmul(tx)).fsqrt();
    let mut tz = x(tp[2]).fadd_m(gf(K_0_3));
    if rd::<u8>(t, 0x36) & 7 == 3 && rd::<u32>(t, 0x46C) & 0x400_0000 != 0 {
        tz = tz.fsub_m(gf(K_0_8));
    }
    let k = l.fdiv(dd);
    let z = k.fmul(tz.fsub_m(s[2])).fadd_m(s[2]).to_f32();
    sa_core::field::wr::<f32>(end, 8, z);
}

/// `CWeapon::DoDriveByAutoAiming(CEntity* owner, CVehicle* vehicle, CVector* start, CVector* end, bool
/// canAimVehicles)` (0x73D720, cdecl). `len = |*end - *start|` (stored f32); nothing without an owner. Candidates:
/// `FindObjectsInRange(*start, len, 2D, &n1, 16, list, peds)` and, when the low byte of `canAimVehicles` is set,
/// `FindObjectsInRange(..., &n2, 16, &list[n1], vehicles)` (both counts dwords zeroed first, the total compared as i16).
/// For each candidate that is not the owner (a ped also not dying / dead (+0x530 0x36 / 0x37) nor attached to the
/// vehicle (+0xFC)): `dl = DistToLine(*start, *end, pos)`; the score is `dl + |pos - vp| * 0.15` (vp = the vehicle's
/// position), for a heli / plane (+0x594 3 / 4) `dl / 5` when `|pos - vp| < 5` else `dl / |pos - vp|` (recomputed).
/// Only candidates ahead (`(b.z*a.z + b.y*a.y) + b.x*a.x > 0` for `a = pos - start`, `b = end - start`) with a score
/// below the best (from 10000, stored f32) count. Then the threshold `tan(angle * pi / 180)` of
/// `GetPlaneGunsAutoAimAngle()` when the angle is above 0.5 (fptan), else 2.5; a best below it moves `*end` onto the
/// line from `*start` through the target T: `k = |start - end| / |start - T|` (stored f32), `end = (T - start) * k +
/// start` (vx * k stored f32, all three computed before any store).
#[hook(addr = 0x73D720, conv = cdecl, name = "CWeapon::DoDriveByAutoAiming", expect = "81 EC B0 00 00 00 53 55",
       unverified = "frontier2-a26", route = "L2", hazards = "x87-transcendental",
       note = "FPTAN threshold; no candidate with a threshold above 10000 reads list[0] (the original: an unwritten index)")]
pub fn do_drive_by_auto_aiming(owner: GPtr<u8>, veh: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>, aim_vehicles: u32) {
    let s = rd3f(start);
    let len = dist_zyx(rd3f(end), s).to_f32();
    let mut counts = [0u32; 2];
    if owner.is_null() {
        return;
    }
    let mut list = [GPtr::<u8>::NULL; 32];
    let lp = GPtr::from_raw(list.as_mut_ptr());
    let cp = GPtr::from_raw(counts.as_mut_ptr());
    find_objects_in_range(start, len, 1, cp.cast(), 16, lp.cast(), 0, 0, 1, 0, 0);
    if aim_vehicles & 0xFF != 0 {
        let n1 = isize::from(counts[0] as u16 as i16);
        find_objects_in_range(start, len, 1, cp.byte_add(4).cast(), 16, lp.byte_add(4 * n1).cast(), 0, 1, 0, 0, 0);
    }
    let total = counts[0].wrapping_add(counts[1]) as u16 as i16;
    let mut best_score = f32::from_bits(K_FAR);
    let mut best = 0usize;
    let mut i = 0i16;
    while i < total {
        let k = i as usize;
        i = i.wrapping_add(1);
        let c = list[k];
        if c == owner {
            continue;
        }
        if rd::<u8>(c, 0x36) & 7 == 3
            && (matches!(rd::<u32>(c, 0x530), 0x36 | 0x37) || rd::<GPtr<u8>>(c, 0xFC) == veh)
        {
            continue;
        }
        let dl = X87::from_f64_exact(dist_to_line(start, end, entity_pos(c)));
        let cpos = || rd3f(entity_pos(list[k]));
        let score = match rd::<u32>(veh, 0x594) {
            3 | 4 => {
                let vp = || rd3f(entity_pos(veh));
                if !dist_zyx(cpos(), vp()).fcom(x(gf(K_5))).test_parity(0x05) {
                    dl.fdiv(x(gf(K_5)))
                } else {
                    dl.fdiv(dist_zyx(cpos(), vp()))
                }
            }
            _ => dl.fadd(dist_zyx(cpos(), rd3f(entity_pos(veh))).fmul_m(gf(K_0_15))),
        };
        let p = cpos();
        let e = rd3f(end);
        let a = |j: usize| x(p[j]).fsub_m(s[j]);
        let b = |j: usize| x(e[j]).fsub_m(s[j]);
        let dot = b(2).fmul(a(2)).fadd(b(1).fmul(a(1))).fadd(b(0).fmul(a(0)));
        if dot.fcom(x(gf(K_0))).test_any(0x41) {
            continue;
        }
        if score.fcom(x(best_score)).test_parity(0x05) {
            continue;
        }
        best_score = score.to_f32();
        best = k;
    }
    let ang = get_plane_guns_auto_aim_angle(veh);
    let thr = if ang.fcom(x(gf(K_0_5))).test_any(0x41) {
        x(gf(K_2_5))
    } else {
        let r = ang.fmul_m(gf(K_DEG));
        r.fptan().map_or(r, |(t, _one)| t)
    };
    if x(best_score).fcom(thr).test_parity(0x05) {
        return;
    }
    let t = list[best];
    let e = rd3f(end);
    let l = dist_zyx(s, e);
    let tp = rd3f(entity_pos(t));
    let dd = dist_zyx(s, tp);
    let vz = x(tp[2]).fsub_m(s[2]).to_f32();
    let k = l.fdiv(dd).to_f32();
    let vxk = x(tp[0]).fsub_m(s[0]).fmul_m(k).to_f32();
    let vyk = x(tp[1]).fsub_m(s[1]).fmul_m(k);
    let vzk = x(vz).fmul_m(k);
    let nx = x(vxk).fadd_m(s[0]).to_f32();
    let ny = vyk.fadd_m(s[1]).to_f32();
    let nz = vzk.fadd_m(s[2]).to_f32();
    sa_core::field::wr::<f32>(end, 0, nx);
    sa_core::field::wr::<f32>(end, 4, ny);
    sa_core::field::wr::<f32>(end, 8, nz);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_DO_DOOM_AIMING, HOOK_DO_DRIVE_BY_AUTO_AIMING];
