//! FRONTIER-2 turn 11, lane B (batch `frontier2-b24`): `CWeapon::DoTankDoomAiming` (0x73D1E0), the aim correction of
//! the tank cannon and the fixed vehicle guns. Ported from the 1.0 US bytes with docs/re/specs/WEAPON/CWeapon.md as
//! reading aid (gta-reversed has no body). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::field::{gf, rd, wr};
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::fpu::{f32_of, ge_or_nan, gt, lt, x};

use sa_world::bindings::{dist_to_line, find_objects_in_range, placeable_get_matrix};
use sa_world::world::common::entity_pos;
use sa_world::world::hand_object::model_info;

/// 0.0 (0x858B50), 3.0 (0x858B3C), 9000.0 (0x872C30), 0.3 (0x858C24).
const K_ZERO: usize = 0x85_8B50;
const K_THREE: usize = 0x85_8B3C;
const K_LIMIT: usize = 0x87_2C30;
const K_LIFT: usize = 0x85_8C24;

/// `CWeapon::DoTankDoomAiming(CEntity* vehicle, CEntity* owner, CVector* start, CVector* end)` (0x73D1E0, cdecl,
/// plain `ret`; entry `nop; jmp 0x40A181`, the frame set-up detour).
///
/// `d = end - start` (each stored f32; the y difference stays unrounded on the stack for the range). The vehicles
/// within `sqrt((dz*dz + dy*dy) + dx*dx)` of `start` (2D, at most 15) are the candidates; `dzn = (e.z - s.z) /
/// sqrt(...)` of the stored differences. A candidate other than `vehicle` / `owner`, not a train (status 6 / 7) and
/// not a scorched vehicle (type 2 with +0x40 bit 29) is measured against `vehicle` (both matrices through the inlined
/// `GetMatrix`): `d2` the 2D distance, `az = |V.z - (d2 * dzn + C.z)|` (both f32); it passes when `az * 3 < d2`, its
/// col model's bounding radius times 3 is above `DistToLine(start2D, end2D, its position2D)` and `sqrt(az*az + d2*d2)`
/// is below the closest so far (10000). With one closer than 9000, `end.z = (|end - start|2D / |T - start|2D) *
/// ((T.z + 0.3) - s.z) + s.z` (T its position; the only write).
#[hook(addr = 0x73D1E0, conv = cdecl, name = "CWeapon::DoTankDoomAiming", expect = "90 E9 9B CF CC FF",
       unverified = "frontier2-b24", route = "L2", hazards = "none",
       note = "may allocate the vehicle's / candidates' matrices (inlined GetMatrix)")]
pub fn do_tank_doom_aiming(vehicle: GPtr<u8>, owner: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>) {
    let (sx, sy, sz) = (rd::<f32>(start, 0), rd::<f32>(start, 4), rd::<f32>(start, 8));
    let (ex, ey, ez) = (rd::<f32>(end, 0), rd::<f32>(end, 4), rd::<f32>(end, 8));
    let dx = f32_of(x(ex).fsub_m(sx));
    let dy_x = x(ey).fsub_m(sy);
    let dy = f32_of(dy_x);
    let dz = f32_of(x(ez).fsub_m(sz));
    let mut s2 = [sx, sy, 0.0f32];
    let mut e2 = [ex, ey, 0.0f32];
    let r = f32_of(x(dz).fmul_m(dz).fadd(dy_x.fmul(dy_x)).fadd(x(dx).fmul_m(dx)).fsqrt());
    let mut count: i16 = 0;
    let mut list = [0u32; 15];
    find_objects_in_range(
        start,
        r,
        1,
        GPtr::from_raw(&mut count as *mut i16).cast(),
        15,
        GPtr::from_raw(list.as_mut_ptr()).cast(),
        0,
        1,
        0,
        0,
        0,
    );
    let len = x(dz).fmul_m(dz).fadd(x(dy).fmul_m(dy)).fadd(x(dx).fmul_m(dx)).fsqrt();
    let dzn = f32_of(x(rd::<f32>(end, 8)).fsub_m(rd::<f32>(start, 8)).fdiv(len));
    let mut closest = 10000.0f32;
    let mut best: u32 = 0;
    if count <= 0 {
        return;
    }
    let mut i: u32 = 0;
    loop {
        let c: GPtr<u8> = GPtr::from_addr(list[i as i16 as usize] as usize);
        let status = rd::<u8>(c, 0x36);
        let skip = c == vehicle
            || c == owner
            || status >> 3 == 6
            || status >> 3 == 7
            || (status & 7 == 2 && rd::<u32>(c, 0x40) & 0x2000_0000 != 0);
        if !skip {
            let vm = placeable_get_matrix(vehicle);
            let cm = placeable_get_matrix(c);
            let ddx = x(rd::<f32>(vm, 0x30)).fsub_m(rd::<f32>(cm, 0x30));
            let ddy = x(rd::<f32>(vm, 0x34)).fsub_m(rd::<f32>(cm, 0x34));
            let d2 = f32_of(ddx.fmul(ddx).fadd(ddy.fmul(ddy)).fsqrt());
            let h = f32_of(x(d2).fmul_m(dzn));
            let v = x(h).fadd_m(rd::<f32>(cm, 0x38)).fsubr_m(rd::<f32>(vm, 0x38));
            let az = if ge_or_nan(v, x(gf(K_ZERO))) { f32_of(v) } else { f32_of(v.fchs()) };
            if lt(x(az).fmul_m(gf(K_THREE)), x(d2)) {
                let p = entity_pos(c);
                let mut pc = [rd::<f32>(p, 0), rd::<f32>(p, 4), 0.0f32];
                let col = rd::<GPtr<u8>>(model_info(rd::<i16>(c, 0x22) as i32), 0x14);
                let rad = rd::<f32>(col, 0x24);
                let dl = dist_to_line(
                    GPtr::from_raw(s2.as_mut_ptr()).cast(),
                    GPtr::from_raw(e2.as_mut_ptr()).cast(),
                    GPtr::from_raw(pc.as_mut_ptr()).cast(),
                );
                if gt(x(rad).fmul_m(gf(K_THREE)), dl) {
                    let d = x(az).fmul_m(az).fadd(x(d2).fmul_m(d2)).fsqrt();
                    if lt(d, x(closest)) {
                        closest = f32_of(d);
                        best = i;
                    }
                }
            }
        }
        i = i.wrapping_add(1);
        if (i as i16) >= count {
            break;
        }
    }
    if ge_or_nan(x(closest), x(gf(K_LIMIT))) {
        return;
    }
    let t: GPtr<u8> = GPtr::from_addr(list[best as i16 as usize] as usize);
    let dyl = x(rd::<f32>(end, 4)).fsub_m(rd::<f32>(start, 4));
    let dxl = x(rd::<f32>(end, 0)).fsub_m(rd::<f32>(start, 0));
    let l = dxl.fmul(dxl).fadd(dyl.fmul(dyl)).fsqrt();
    let tp = entity_pos(t);
    let s0 = rd::<f32>(start, 0);
    let s1 = rd::<f32>(start, 4);
    let ty = x(rd::<f32>(tp, 4)).fsub_m(s1);
    let tx = x(rd::<f32>(tp, 0)).fsub_m(s0);
    let dd = ty.fmul(ty).fadd(tx.fmul(tx)).fsqrt();
    let hz = x(rd::<f32>(tp, 8)).fadd_m(gf(K_LIFT)).fsub_m(rd::<f32>(start, 8));
    let z = l.fdiv(dd).fmul(hz).fadd_m(rd::<f32>(start, 8));
    wr::<f32>(end, 8, f32_of(z));
}

pub const HOOKS: &[HookReg] = &[HOOK_DO_TANK_DOOM_AIMING];
