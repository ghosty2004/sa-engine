//! `CCollision::CheckCameraCollisionVehicles` (0x41A990): the vehicles part of the camera collision cache. Ported from
//! the 1.0 US bytes (gta-reversed `Collision.cpp` as a reading aid; the bytes win: the bound sphere comes from the
//! model info's col model, the velocity test keeps the differences in extended precision). Phase B only
//! (`#[native]`): the entry cannot be hooked in phase A (the no-CD protection reads its entry bytes as data and a
//! protection call targets 0x41A997, inside the first instructions). Not verified.
#![allow(clippy::unreadable_literal)]

use sa_core::mem::GPtr;
use sa_core::{native, orig};
use sa_math::x87::X87;

use super::mvpe_coll1::entity_bound_sphere;
use super::mvpc_coll1::local;
use crate::world::common::{g, rd, wr};

orig! {
    /// `CCollision::IsThisVehicleSittingOnMe(CVehicle*, CVehicle*)` (0x4152C0, AL)
    fn is_this_vehicle_sitting_on_me(veh: GPtr<u8>, on_me: GPtr<u8>) -> u8 = cdecl 0x4152C0 => super::prim::is_this_vehicle_sitting_on_me;
    /// `CCollision::TestSphereSphere(const CColSphere&, const CColSphere&)` (0x411E70, AL)
    fn test_sphere_sphere(a: GPtr<f32>, b: GPtr<f32>) -> u32 = cdecl 0x411E70 => super::prim::test_sphere_sphere;
    /// `CCollision::SphereCastVsEntity(const CColSphere&, const CColSphere&, CEntity*)` (0x419F00, AL)
    fn sphere_cast_vs_entity(a: GPtr<u8>, b: GPtr<u8>, e: GPtr<u8>) -> u8 = cdecl 0x419F00 => super::sphere_cast::sphere_cast_vs_entity;
}

/// `CWorld::ms_aRepeatSectors` vehicle lists (0xB992B8, 12 bytes per sector)
const REPEAT_SECTOR_VEHICLES: usize = 0xB992B8;
/// `CWorld::ms_nCurrentScanCode` (u16), `CWorld::pIgnoreEntity`
const SCAN_CODE: usize = 0xB7CD78;
const IGNORE_ENTITY: usize = 0xB7CD68;
/// `gpLastSittingOnEntity`, `gFramesSittingOnTimeOut`, `gFramesToConsiderSittingOnStillTrue` (30)
const LAST_SITTING_ON: usize = 0x9689D8;
const SITTING_TIME_OUT: usize = 0x9689D4;
const SITTING_FRAMES: usize = 0x8A5B1C;
/// `relVelCamCollisionVehiclesSqr` (f32)
const REL_VEL_SQR: usize = 0x8A5B18;

/// `CCollision::CheckCameraCollisionVehicles(int32 x, int32 y, const CColBox&, const CColSphere& spS, const CColSphere&
/// spA, const CColSphere& spB, const CVector* plyrVehVel)` (0x41A990, cdecl, AL).
///
/// Repeat sector `((y & 15) << 4) + (x & 15)`, vehicle list. Per vehicle not yet scanned (scan code set) and not
/// `pIgnoreEntity`: sitting on the ignored vehicle (`IsThisVehicleSittingOnMe(pIgnoreEntity, v)`) → it becomes
/// `gpLastSittingOnEntity` with a 30-frame (0x8A5B1C) time-out, skipped; the last sitting-on entity → its time-out is
/// decremented (post-decrement 0 → forgotten), skipped. With a velocity, `(dz*dz + dy*dy) + dx*dx` of `vel -
/// moveSpeed` (+0x44, extended) not below `relVelCamCollisionVehiclesSqr` (or NaN) skips. Then the bound sphere
/// (model info col model radius, `GetBoundCentre`, `CColSphere::Set(r, c, 0, 0, 0xFF)`) against `spS`
/// (`TestSphereSphere`) and `SphereCastVsEntity(spA, spB, v)` → true.
///
/// The original passes the radius through its own `x` / `y` argument slot (not reproduced).
#[native(addr = 0x41A990, conv = cdecl, name = "CCollision::CheckCameraCollisionVehicles")]
#[allow(clippy::too_many_arguments)]
pub fn check_camera_collision_vehicles(x: i32, y: i32, _bb: GPtr<u8>, sps: GPtr<u8>, spa: GPtr<u8>, spb: GPtr<u8>,
                                       vel: GPtr<u8>) -> u8 {
    let idx = (((y & 0xF) << 4) + (x & 0xF)) as usize;
    let mut node = g::<GPtr<u8>>(REPEAT_SECTOR_VEHICLES + 12 * idx).read();
    let mut hit = 0u8;
    while !node.is_null() {
        let v = rd::<GPtr<u8>>(node, 0);
        let sc = g::<u16>(SCAN_CODE).read();
        if rd::<u16>(v, 0x2C) != sc {
            wr::<u16>(v, 0x2C, sc);
            let ignore = g::<GPtr<u8>>(IGNORE_ENTITY).read();
            if ignore != v {
                if is_this_vehicle_sitting_on_me(ignore, v) != 0 {
                    g::<GPtr<u8>>(LAST_SITTING_ON).write(v);
                    g::<i32>(SITTING_TIME_OUT).write(g::<i32>(SITTING_FRAMES).read());
                } else if g::<GPtr<u8>>(LAST_SITTING_ON).read() == v {
                    let t = g::<i32>(SITTING_TIME_OUT).read();
                    g::<i32>(SITTING_TIME_OUT).write(t.wrapping_sub(1));
                    if t == 0 {
                        g::<u32>(LAST_SITTING_ON).write(0);
                    }
                } else {
                    let fast = !vel.is_null() && {
                        let d = |k: isize| X87::from_f32(rd::<f32>(vel, k)).fsub_m(rd::<f32>(v, 0x44 + k));
                        let (dx, dy, dz) = (d(0), d(4), d(8));
                        let s = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
                        s.fcom(X87::from_f32(g::<f32>(REL_VEL_SQR).read())).test_parity(5)
                    };
                    if !fast {
                        let mut sb = [0u32; 5];
                        let sp = local(&mut sb);
                        entity_bound_sphere(v, sp);
                        if test_sphere_sphere(sps.cast(), sp.cast()) as u8 != 0 && sphere_cast_vs_entity(spa, spb, v) != 0 {
                            hit = 1;
                        }
                    }
                }
            }
        }
        node = rd::<GPtr<u8>>(node, 4);
    }
    hit
}
