//! WRITER-CLIMB (batch `climb-5`): `CGarage::IsEntityTouching3D` (0x448EE0) and the garage's "a car blocks the
//! door" test (0x4494F0, a `.HOODLUM` stub into 0x156D610; gta-reversed's `CGarage::IsAnyCarBlockingDoor`), reached
//! when CJ is near his garage. Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{g, rd, x};

orig! {
    /// `CGarage::IsEntityTouching3D(CEntity*)` (0x448EE0, `ret 4`)
    fn garage_is_entity_touching_3d(g: GPtr<u8>, e: GPtr<u8>) -> u8 = thiscall 0x448EE0 => garage_is_entity_touching_3d_port;
    /// `CEntity::GetColModel()` (0x535300)
    fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CPlaceable::GetMatrix()` (0x411990)
    fn placeable_get_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x411990 => unported;
    /// `CMatrix * CVector` (0x59C890, `(CVector* out, const CMatrix*, const CVector*)`)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CGarage::IsPointInsideGarage(CVector, float)` (0x4487D0, `ret 0x10`)
    fn garage_is_point_inside_r(g: GPtr<u8>, x: u32, y: u32, z: u32, r: u32) -> u8 = thiscall 0x4487D0 => unported;
}

/// `CPools::ms_pVehiclePool` (0xB74494): slots +0, flags +4, size +8; 0xA18-byte vehicles.
const VEHICLE_POOL: usize = 0xB7_4494;

/// `bool CGarage::IsAnyCarBlockingDoor()` (0x4494F0 -> 0x156D610).
///
/// Over the vehicle pool from the last slot down, every used slot (flag byte without bit 7) whose vehicle
/// `IsEntityTouching3D` this garage: any collision sphere centre (through the vehicle matrix) with its radius
/// `IsPointInsideGarage` gives true (the sphere count, a signed word, is re-read per sphere). Otherwise false.
#[hook(addr = 0x4494F0, conv = thiscall, name = "thunk_FUN_0156d610",
       expect = "E9 1B 41 12 01",
       unverified = "climb-5", route = "L2", hazards = "ax-al-return", recs = "grove-walk")]
pub fn garage_is_any_car_blocking_door(this: GPtr<u8>) -> u8 {
    let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        i -= 1;
        if rd::<i8>(rd::<GPtr<u8>>(pool, 4), i as isize) < 0 {
            continue;
        }
        let v: GPtr<u8> = GPtr::from_addr(rd::<u32>(pool, 0).wrapping_add((i as u32).wrapping_mul(0xA18)) as usize);
        if v.is_null() {
            continue;
        }
        if garage_is_entity_touching_3d(this, v) == 0 {
            continue;
        }
        let col = rd::<GPtr<u8>>(entity_get_col_model(v), 0x2C);
        let mut j = 0i32;
        while j < i32::from(rd::<i16>(col, 0)) {
            let sphere = rd::<GPtr<u8>>(col, 8).byte_add(j as isize * 0x14);
            let mut out = [0u32; 3];
            let r = matrix_mul_vec(GPtr::from_raw(out.as_mut_ptr()).cast(), rd::<GPtr<u8>>(v, 0x14), sphere);
            let radius = rd::<u32>(rd::<GPtr<u8>>(col, 8), j as isize * 0x14 + 0xC);
            if garage_is_point_inside_r(this, rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8), radius) != 0 {
                return 1;
            }
            j += 1;
        }
    }
    0
}

/// `bool CGarage::IsEntityTouching3D(CEntity* entity)` (0x448EE0, `ret 4`).
///
/// With `r` the entity's model collision radius (model info +0x14, +0x24; kept in the entity argument slot), false
/// when the entity's position lies outside the garage box grown by `r` on any axis (x: +0x28 / +0x2C, y: +0x30 /
/// +0x34, z: +0x8 / +0x1C; `min - r > p` or `r + max < p`, ordered). Otherwise true as soon as one collision
/// sphere (centre through `GetMatrix()`, its radius) `IsPointInsideGarage`; false when none does.
#[hook(addr = 0x448EE0, conv = thiscall, name = "CGarage::IsEntityTouching3D",
       expect = "83 EC 10 53 8B 5C 24 18",
       unverified = "climb-5", route = "L2", hazards = "ax-al-return", recs = "grove-walk")]
pub fn garage_is_entity_touching_3d_port(this: GPtr<u8>, e: GPtr<u8>) -> u8 {
    let info = g::<GPtr<u8>>(0xA9_B0C8).wrapping_index(i32::from(rd::<i16>(e, 0x22))).read();
    let r = rd::<f32>(rd::<GPtr<u8>>(info, 0x14), 0x24);
    let pos = || {
        let m = rd::<GPtr<u8>>(e, 0x14);
        if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
    };
    for (lo, hi, k) in [(0x28isize, 0x2Cisize, 0isize), (0x30, 0x34, 4), (0x8, 0x1C, 8)] {
        if !x(rd::<f32>(this, lo)).fsub_m(r).fcom(x(rd::<f32>(pos(), k))).test_any(0x41) {
            return 0;
        }
        if !x(r).fadd_m(rd::<f32>(this, hi)).fcom(x(rd::<f32>(pos(), k))).test_parity(0x05) {
            return 0;
        }
    }
    let col = rd::<GPtr<u8>>(entity_get_col_model(e), 0x2C);
    let mut j = 0i32;
    while j < i32::from(rd::<i16>(col, 0)) {
        let sphere = rd::<GPtr<u8>>(col, 8).byte_add(j as isize * 0x14);
        let m = placeable_get_matrix(e);
        let mut out = [0u32; 3];
        let c = matrix_mul_vec(GPtr::from_raw(out.as_mut_ptr()).cast(), m, sphere);
        let radius = rd::<u32>(rd::<GPtr<u8>>(col, 8), j as isize * 0x14 + 0xC);
        if garage_is_point_inside_r(this, rd::<u32>(c, 0), rd::<u32>(c, 4), rd::<u32>(c, 8), radius) != 0 {
            return 1;
        }
        j += 1;
    }
    0
}

pub const HOOKS: &[HookReg] =
    &[HOOK_GARAGE_IS_ANY_CAR_BLOCKING_DOOR, HOOK_GARAGE_IS_ENTITY_TOUCHING_3D_PORT];
