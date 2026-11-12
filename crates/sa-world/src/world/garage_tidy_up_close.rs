//! `CGarage::TidyUpGarageClose` (0x449D10, batch `writerq-misc-1`): removes wrecked cars and bikes caught by a
//! closing garage door. Not reversed in gta-reversed; ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd};

orig! {
    /// `CGarage::IsEntityTouching3D(CEntity*)` (0x448EE0, `ret 4`)
    fn garage_is_entity_touching_3d(g: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x448EE0 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `operator*(CVector* out, const CMatrix*, const CVector*)` (0x59C890, cdecl)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CGarage::IsPointInsideGarage(CVector, float)` (0x4487D0, `ret 0x10`)
    fn garage_is_point_inside_r(g: GPtr<u8>, x: u32, y: u32, z: u32, r: u32) -> u8 = thiscall 0x4487D0 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `CVehicle` scalar deleting destructor (vtable slot 0)
    fn vehicle_delete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
}

/// `CPools::ms_pVehiclePool`
const VEHICLE_POOL: usize = 0xB74494;

/// `CGarage::TidyUpGarageClose()` (0x449D10, thiscall). Vehicle pool slots from size - 1 down to 1 (slot 0 is never
/// visited): an occupied automobile (+0x590 = 0) or bike (9) with status 5 (wrecked, byte +0x36 & 0xF8 = 0x28) touching
/// the garage (`IsEntityTouching3D`) is removed (`CWorld::Remove`, then the deleting destructor with 1) when the
/// garage's +0x4D is 0, or when one of its collision spheres (centre through the vehicle's matrix) is not inside
/// (`IsPointInsideGarage(centre, radius)`; a model without spheres keeps the vehicle).
#[hook(addr = 0x449D10, conv = thiscall, name = "CGarage::TidyUpGarageClose", this = "CGarage",
       expect = "83 EC 24 53 55", unverified = "writerq-misc-1", route = "L2", hazards = "heap-writes",
       recs = "grove-walk", note = "deletes vehicles (pool / heap)")]
pub fn garage_tidy_up_close(this: GPtr<u8>) {
    let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let mut i = rd::<i32>(pool, 8) - 1;
    while i != 0 {
        let flags = rd::<GPtr<u8>>(pool, 4);
        let veh = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0xA18);
        let take = rd::<i8>(flags, i as isize) >= 0
            && !veh.is_null()
            && matches!(rd::<u32>(veh, 0x590), 0 | 9)
            && rd::<u8>(veh, 0x36) & 0xF8 == 0x28
            && garage_is_entity_touching_3d(this, veh) != 0;
        if take {
            let mut remove = true;
            if rd::<u8>(this, 0x4D) != 0 {
                remove = false;
                let data = rd::<GPtr<u8>>(entity_get_col_model(veh), 0x2C);
                if rd::<i16>(data, 0) <= 0 {
                    i -= 1;
                    continue;
                }
                let mut k = 0;
                while k < i32::from(rd::<i16>(data, 0)) {
                    let s = rd::<GPtr<u8>>(data, 8).byte_add(k as isize * 0x14);
                    let mut c = [0u32; 3];
                    let p = matrix_mul_vec(
                        GPtr::from_raw(c.as_mut_ptr()).cast(),
                        rd::<GPtr<u8>>(veh, 0x14),
                        s,
                    );
                    let r = rd::<u32>(rd::<GPtr<u8>>(data, 8).byte_add(k as isize * 0x14), 0xC);
                    if garage_is_point_inside_r(this, rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8), r)
                        == 0
                    {
                        remove = true;
                    }
                    k += 1;
                }
            }
            if remove {
                world_remove(veh);
                vehicle_delete(veh, 1);
            }
        }
        i -= 1;
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_GARAGE_TIDY_UP_CLOSE];
