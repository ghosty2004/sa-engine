//! PORT-VEH-3 (batch `world3-2`, SA:MP-executed rows): `CWorld::TriggerExplosion`, the sector walk
//! that hands an explosion to the repeat-sector lists. Ported from the 1.0 US bytes; gta-reversed as
//! a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{ftol32, x};

orig! {
    /// CRT `floor(double)` (0x8219F0), ST0
    fn crt_floor(v: f64) -> f64 = cdecl 0x8219F0 => unported;
    /// `CWorld::TriggerExplosionSectorList(CPtrListDoubleLink&, const CVector&, float, float, CEntity*, CEntity*, bool, float)` (0x567750)
    fn trigger_explosion_sector_list(list: GPtr<u8>, point: GPtr<u8>, radius: u32, vis: u32, victim: u32, creator: u32, bomb: u32, damage: u32) = cdecl 0x567750 => unported;
}

/// `_ftol(floor(v * 0.02 + 60))`: the sector index of a world coordinate (the product as a double).
fn sector(v: X87) -> i32 {
    let s = v.fmul_m(f32::from_bits(0x3CA3_D70A)).fadd_m(60.0);
    ftol32(X87::from_f64_exact(crt_floor(s.to_f64())))
}

/// `CWorld::TriggerExplosion(const CVector&, float radius, float, CEntity* victim, CEntity* creator,
/// bool, float damage)` (0x56B790): for every sector of `point ± radius` (y outer, x inner, no
/// clamping) the three repeat-sector lists (0xB992B8 + ((x & 15) + (y & 15) * 16) * 12: vehicles,
/// peds, objects) get `TriggerExplosionSectorList` with the same arguments.
#[hook(addr = 0x56B790, conv = cdecl, name = "CWorld::TriggerExplosion",
       expect = "83 EC 10 53 56",
       unverified = "world3-2", route = "L2", hazards = "heap-writes",
       note = "the sector lists damage and push entities; the original keeps the x end in its first argument slot")]
#[allow(clippy::too_many_arguments)]
pub fn trigger_explosion(point: GPtr<u8>, radius: u32, vis: u32, victim: u32, creator: u32, bomb: u32, damage: u32) {
    let r = f32::from_bits(radius);
    let px = point.cast::<f32>().read();
    let py = point.byte_add(4).cast::<f32>().read();
    let x0 = sector(x(px).fsub_m(r));
    let y0 = sector(x(py).fsub_m(r));
    let x1 = sector(x(r).fadd_m(px));
    let y1 = sector(x(r).fadd_m(py));
    let mut y = y0;
    while y <= y1 {
        let mut xx = x0;
        while xx <= x1 {
            let idx = ((xx & 0xF) + ((y & 0xF) << 4)) as usize;
            let base = 0xB9_92B8 + idx * 12;
            for l in 0..3 {
                trigger_explosion_sector_list(GPtr::from_addr(base + 4 * l), point, radius, vis, victim, creator, bomb, damage);
            }
            xx += 1;
        }
        y += 1;
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_TRIGGER_EXPLOSION];
