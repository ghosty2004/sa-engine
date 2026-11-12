//! `CStencilShadows::RegisterStencilShadows` (0x711760), which keeps the stencil shadow objects of
//! the vehicles and objects near the camera. Ported from the 1.0 US bytes (spec
//! `docs/re/specs/RENDER/CStencilShadows.md`); the distance helpers 0x711160 / 0x7111F0 take EBX =
//! entity and ESI = point and return an 80-bit ST0, so they are called through `orig! { ... =
//! reg("args = ebx esi") }` (HARNESS-5).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, k, rd, wr, x};
use crate::stencilshadows::{ACTIVE_OBJECTS, FREE_OBJECTS};

orig! {
    /// `CStencilShadowObject::Destroy()` (0x711310)
    fn destroy(o: GPtr<u8>) = thiscall 0x711310 => crate::stencilshadows::destroy;
    /// `CStencilShadowObject::CreateFor(CEntity*, uint8 type)` (0x711280, `ret 8`)
    fn create_for(o: GPtr<u8>, e: GPtr<u8>, ty: u32) -> u8 = thiscall 0x711280 => unported;
    /// `CStencilShadows::FindActiveFor(CEntity*)` (0x710BA0)
    fn find_active_for(e: GPtr<u8>) -> GPtr<u8> = cdecl 0x710BA0 => unported;
    /// The object shadow distance 0x711160: EBX = entity, ESI = point, ST0 result
    fn sq_dist_object(e: GPtr<u8>, p: GPtr<u8>) -> X87 = reg("args = ebx esi") 0x711160 => crate::harness5::signed_sq_dist_object;
    /// The vehicle shadow distance 0x7111F0: EBX = entity, ESI = point, ST0 result
    fn sq_dist_vehicle(e: GPtr<u8>, p: GPtr<u8>) -> X87 = reg("args = ebx esi") 0x7111F0 => crate::harness5::signed_sq_dist_vehicle;
    /// `CEntity::GetColModel()` (0x535300)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CWorld::ClearScanCodes()` (0x563470; the port is unhookable)
    fn clear_scan_codes() = cdecl 0x563470 => sa_world::world::cworld::clear_scan_codes;
    /// `CBaseModelInfo::GetModelType()` (vtable +0x10)
    fn mi_get_model_type(mi: GPtr<u8>) -> u8 = thiscall vslot 0x10 => unported;
}

/// `CPools::ms_pVehiclePool`, `CGame::currArea`, `CWorld::ms_nCurrentScanCode`, `CWorld::ms_aSectors`,
/// `CWorld::ms_aRepeatSectors` + 8 (the object lists), `CModelInfo::ms_modelInfoPtrs`.
const VEHICLE_POOL: usize = 0xB7_4494;
const CURR_AREA: usize = 0xB7_2914;
const SCAN_CODE: usize = 0xB7_CD78;
const SECTORS: usize = 0xB7_D0B8;
const REPEAT_OBJECTS: usize = 0xB9_92C0;
const MODEL_INFO_PTRS: usize = 0xA9_B0C8;
/// 2500.0f (0x8598B0), 50.0f (0x858B40), 0.02f (0x858B38), 60.0f (0x858B34).
const RANGE_SQ: u32 = 0x451C_4000;
const RANGE: u32 = 0x4248_0000;
const SECTOR_SCALE: u32 = 0x3CA3_D70A;
const SECTOR_OFFSET: u32 = 0x4270_0000;

/// An object (type 4) that is invisible (`!(+0x1C & 0x80)`), exploded (`+0x140 & 0x40`) or broken
/// (`+0x140 & 0x400`).
fn bad_object(e: GPtr<u8>) -> bool {
    rd::<u8>(e, 0x36) & 7 == 4
        && (rd::<u32>(e, 0x1C) & 0x80 == 0 || rd::<u32>(e, 0x140) & 0x40 != 0 || rd::<u32>(e, 0x140) & 0x400 != 0)
}

/// In the current area (`+0x2F == CGame::currArea`, or 13).
fn in_area(e: GPtr<u8>) -> bool {
    let a = rd::<u8>(e, 0x2F);
    u32::from(a) == g::<u32>(CURR_AREA).read() || a == 0xD
}

/// A collision model with collision data whose flag byte +7 has bit 2.
fn has_shadow_mesh(e: GPtr<u8>) -> bool {
    let cm = get_col_model(e);
    if cm.is_null() {
        return false;
    }
    let cd = rd::<GPtr<u8>>(cm, 0x2C);
    !cd.is_null() && rd::<u8>(cd, 7) & 4 != 0
}

/// `fcomp 2500; fnstsw; test ah, 0x41; jp`: taken (skip) unless exactly one of C0 / C3 (less or equal).
fn within(d: X87) -> bool {
    !d.fcom(x(k(RANGE_SQ))).test_parity(0x41)
}

/// CRT `floor` (0x8219F0) of the double the original passes (exact in f64), `_ftol`.
fn floor_ftol(v: X87) -> i32 {
    ftol32(X87::from_f64_exact(sa_world::physical::base::floor_f64(v.to_f64())))
}

/// A sector bound: `t = v * 0.02 + 60` (stored f32), `ftol(floor(t))` from the unrounded value; in
/// range (`> 0` minimum, `< 119` maximum) recomputed from the stored f32, else 0 / 119.
fn sector_bound(v: X87, is_max: bool) -> i32 {
    let t = v.fmul_m(k(SECTOR_SCALE)).fadd_m(k(SECTOR_OFFSET));
    let n = floor_ftol(t);
    let pass = if is_max { n < 0x77 } else { n > 0 };
    if pass {
        floor_ftol(x(t.to_f32()))
    } else if is_max {
        0x77
    } else {
        0
    }
}

/// One entity of a sector list (step 4a of the spec). `Err(())`: the free list ran out (return 0).
fn consider(e: GPtr<u8>, cam: GPtr<u8>) -> Result<(), ()> {
    if rd::<u32>(e, 0x1C) & 0x400_0000 != 0 || bad_object(e) || !has_shadow_mesh(e) {
        return Ok(());
    }
    let code = g::<u16>(SCAN_CODE).read();
    if rd::<u16>(e, 0x2C) == code {
        return Ok(());
    }
    wr::<u16>(e, 0x2C, code);
    if !in_area(e) {
        return Ok(());
    }
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * rd::<i16>(e, 0x22) as isize as usize).read();
    if mi_get_model_type(mi) != 1 {
        return Ok(());
    }
    let mut a = GPtr::<u8>::from_addr(ACTIVE_OBJECTS.get() as usize);
    while !a.is_null() {
        if rd::<GPtr<u8>>(a, 0) == e {
            return Ok(());
        }
        a = rd::<GPtr<u8>>(a, 0x14);
    }
    if !within(sq_dist_object(e, cam)) {
        return Ok(());
    }
    let free = GPtr::<u8>::from_addr(FREE_OBJECTS.get() as usize);
    if free.is_null() {
        return Err(());
    }
    create_for(free, e, 1);
    if FREE_OBJECTS.get() == 0 {
        return Err(());
    }
    Ok(())
}

/// Walks an entity list (`node +0` entity, `+4` next, read first).
fn consider_list(mut node: GPtr<u8>, cam: GPtr<u8>) -> Result<(), ()> {
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        node = rd::<GPtr<u8>>(node, 4);
        consider(e, cam)?;
    }
    Ok(())
}

/// `CStencilShadows::RegisterStencilShadows(CVector* cam, bool doNotCreateNew)` (0x711760, cdecl, AL).
/// `doNotCreateNew`: active objects whose owner is gone or a bad object (see [`bad_object`]) are
/// destroyed; 1. Otherwise the active objects are also destroyed when an object's owner (type 1) is
/// farther than 50 (`0x711160` > 2500; NaN keeps) or out of the area, or a vehicle's (type 2) is
/// farther (`0x7111F0` > 2500, NaN keeps). Without a free object: 1. Every used vehicle-pool slot
/// in the world (`+0xB0` collision list), with a shadow mesh (collision data flag +7 bit 2), no
/// active object (`FindActiveFor`) and within 50 gets one (`CreateFor(free, vehicle, 2)`). Then the
/// sectors of `cam ± 50` (bounds `floor(v * 0.02 + 60)` clamped to 0..119, x minimum from the
/// unrounded value) get a new scan code and each sector's building list and repeat-sector object list
/// (`(y & 15) * 16 + (x & 15)`, unclamped indices) offer their entities (see [`consider`]). Running
/// out of free objects returns 0, else 1. The original keeps the vehicle-pool offset and the sector
/// temporaries in its own `doNotCreateNew` argument slot.
#[hook(addr = 0x711760, conv = cdecl, name = "CStencilShadows::RegisterStencilShadows", expect = "8A 44 24 08 83 EC 1C",
       route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk,driving",
       note = "calls the shadow distances 0x711160 / 0x7111F0 (EBX/ESI -> 80-bit ST0) through orig! reg; writes scan codes and the shadow object lists; uses its own argument slot as scratch")]
pub fn register_stencil_shadows(cam: GPtr<u8>, do_not_create_new: u8) -> u8 {
    let mut o = GPtr::<u8>::from_addr(ACTIVE_OBJECTS.get() as usize);
    if do_not_create_new != 0 {
        while !o.is_null() {
            let next = rd::<GPtr<u8>>(o, 0x14);
            let owner = rd::<GPtr<u8>>(o, 0);
            if owner.is_null() || bad_object(owner) {
                destroy(o);
            }
            o = next;
        }
        return 1;
    }
    while !o.is_null() {
        let next = rd::<GPtr<u8>>(o, 0x14);
        let owner = rd::<GPtr<u8>>(o, 0);
        let kill = if owner.is_null() || bad_object(owner) {
            true
        } else {
            match rd::<u8>(o, 6) {
                1 => !sq_dist_object(owner, cam).fcom(x(k(RANGE_SQ))).test_any(0x41) || !in_area(owner),
                2 => !sq_dist_vehicle(owner, cam).fcom(x(k(RANGE_SQ))).test_any(0x41),
                _ => false,
            }
        };
        if kill {
            destroy(o);
        }
        o = next;
    }
    if FREE_OBJECTS.get() == 0 {
        return 1;
    }
    let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let size = rd::<i32>(pool, 8);
    for i in 0..size.max(0) {
        if rd::<i8>(rd::<GPtr<u8>>(pool, 4), i as isize) < 0 {
            continue;
        }
        let v = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0xA18);
        if v.is_null() || !has_shadow_mesh(v) || rd::<u32>(v, 0xB0) == 0 || !find_active_for(v).is_null() {
            continue;
        }
        if !within(sq_dist_vehicle(v, cam)) {
            continue;
        }
        let free = GPtr::<u8>::from_addr(FREE_OBJECTS.get() as usize);
        if free.is_null() {
            return 0;
        }
        create_for(free, v, 2);
        if FREE_OBJECTS.get() == 0 {
            return 0;
        }
    }
    if FREE_OBJECTS.get() == 0 {
        return 1;
    }
    let (px, py) = (rd::<f32>(cam, 0), rd::<f32>(cam, 4));
    let min_x = x(px).fsub_m(k(RANGE));
    let min_y = x(py).fsub_m(k(RANGE)).to_f32();
    let max_x = x(px).fadd_m(k(RANGE)).to_f32();
    let max_y = x(py).fadd_m(k(RANGE)).to_f32();
    let x0 = sector_bound(min_x, false);
    let y0 = sector_bound(x(min_y), false);
    let x1 = sector_bound(x(max_x), true);
    let y1 = sector_bound(x(max_y), true);
    let code = g::<u16>(SCAN_CODE).read();
    if code == 0xFFFF {
        clear_scan_codes();
        g::<u16>(SCAN_CODE).write(1);
    } else {
        g::<u16>(SCAN_CODE).write(code + 1);
    }
    let mut yi = y0;
    while yi <= y1 {
        let yc = yi.clamp(0, 0x77);
        let mut xi = x0;
        while xi <= x1 {
            let xc = xi.clamp(0, 0x77);
            let buildings = g::<GPtr<u8>>(SECTORS + 8 * (yc * 0x78 + xc) as usize).read();
            if consider_list(buildings, cam).is_err() {
                return 0;
            }
            let r = ((yi & 0xF) << 4) + (xi & 0xF);
            let objects = g::<GPtr<u8>>(REPEAT_OBJECTS + 12 * r as usize).read();
            if consider_list(objects, cam).is_err() {
                return 0;
            }
            xi += 1;
        }
        yi += 1;
    }
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_REGISTER_STENCIL_SHADOWS];
