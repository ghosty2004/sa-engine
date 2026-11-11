//! `CStencilShadows::RegisterStencilShadows` (batch `mvpm-render-5`): drops stale stencil shadow objects and creates
//! new ones for nearby vehicles, buildings and objects. Ported from the 1.0 US bytes
//! (`docs/re/specs/RENDER/CStencilShadows.md`; gta-reversed has an empty body). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd, wr, x};

orig! {
    /// `CStencilShadowObject::Destroy()` (0x711310)
    fn shadow_object_destroy(o: GPtr<u8>) = thiscall 0x711310 => crate::stencilshadows::destroy;
    /// The signed squared distance to an object's bound sphere (0x711160; EBX = entity, ESI = point; ST0)
    fn sq_dist_object(e: GPtr<u8>, p: GPtr<u8>) -> X87 = reg("args = ebx esi") 0x711160 => crate::harness5::signed_sq_dist_object;
    /// The same for vehicles (0x7111F0)
    fn sq_dist_vehicle(e: GPtr<u8>, p: GPtr<u8>) -> X87 = reg("args = ebx esi") 0x7111F0 => crate::harness5::signed_sq_dist_vehicle;
    /// `CStencilShadowObject::CreateFor(CEntity*, uint8 type)` (0x711280, `ret 8`)
    fn shadow_object_create_for(o: GPtr<u8>, e: GPtr<u8>, ty: u32) -> GPtr<u8> = thiscall 0x711280 => unported;
    /// The active shadow object of an entity (0x710BA0, cdecl)
    fn shadow_object_find_active_for(e: GPtr<u8>) -> GPtr<u8> = cdecl 0x710BA0 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CWorld::ClearScanCodes()` (0x563470)
    fn world_clear_scan_codes() = cdecl 0x563470 => unported;
    /// `CBaseModelInfo::GetModelType()` (vtable +0x10), AL
    fn v_model_type(mi: GPtr<u8>) -> u8 = thiscall vslot 0x10 => unported;
}

const FIRST_AVAILABLE: usize = 0xC6A168;
const FIRST_ACTIVE: usize = 0xC6A16C;
/// 2500.0f (0x8598B0): the squared range
const K_RANGE_SQ: usize = 0x8598B0;
/// 50.0f (0x858B40): the sector rectangle half size
const K_HALF_SIZE: usize = 0x858B40;
const K_SECTOR_SCALE: usize = 0x858B38;
const K_SECTOR_OFFSET: usize = 0x858B34;
const VEHICLE_POOL: usize = 0xB74494;
const CURR_AREA: usize = 0xB72914;
const SCAN_CODE: usize = 0xB7CD78;
const SECTORS: u32 = 0xB7D0B8;
/// `CWorld::ms_aRepeatSectors` + 8 (the object lists, 12-byte entries, 0xB992C0)
const REPEAT_OBJECTS: u32 = 0xB992C0;
const MODEL_INFOS: u32 = 0xA9B0C8;

fn kf(a: usize) -> f32 {
    g::<f32>(a).read()
}
fn floor_d(v: f64) -> f64 {
    if v.is_nan() || v.is_infinite() || v.abs() >= 4_503_599_627_370_496.0 {
        return v;
    }
    let t = v as i64 as f64;
    if t > v { t - 1.0 } else { t }
}
fn floor_i(v: X87) -> i32 {
    X87::from_f64_exact(floor_d(v.to_f64())).ftol() as u32 as i32
}
/// One sector bound (see `CPlantMgr::_ColEntityCache_Update`, the same inlined `CWorld::GetSector`).
fn sector(c: X87, low: bool) -> i32 {
    let v = c.fmul_m(kf(K_SECTOR_SCALE)).fadd_m(kf(K_SECTOR_OFFSET));
    let stored = v.to_f32();
    let t = floor_i(v);
    if low {
        if t > 0 { floor_i(x(stored)) } else { 0 }
    } else if t < 0x77 {
        floor_i(x(stored))
    } else {
        0x77
    }
}
/// The "bad object" test: an object (type 4) that is not visible (+0x1C bit 7) or exploded / broken (+0x140 bits 6,
/// 10).
fn bad_object(e: GPtr<u8>) -> bool {
    rd::<u8>(e, 0x36) & 7 == 4
        && (rd::<u32>(e, 0x1C) & 0x80 == 0 || rd::<u32>(e, 0x140) & 0x40 != 0 || rd::<u32>(e, 0x140) & 0x400 != 0)
}
fn has_shadow_data(e: GPtr<u8>) -> bool {
    let cm = entity_get_col_model(e);
    if cm.is_null() {
        return false;
    }
    let cd = rd::<GPtr<u8>>(cm, 0x2C);
    !cd.is_null() && rd::<u8>(cd, 7) & 4 != 0
}
fn in_range(d: X87) -> bool {
    !d.fcom(x(kf(K_RANGE_SQ))).test_parity(0x41)
}

/// One sector list (buildings or objects): every entity that qualifies gets a type-1 shadow object. `Err(())` = the
/// free list ran empty (AL = 0).
fn scan_list(mut node: GPtr<u8>, cam: GPtr<u8>) -> Result<(), ()> {
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        node = rd::<GPtr<u8>>(node, 4);
        if rd::<u32>(e, 0x1C) & 0x400_0000 != 0 || bad_object(e) || !has_shadow_data(e) {
            continue;
        }
        let code = g::<u16>(SCAN_CODE).read();
        if rd::<u16>(e, 0x2C) == code {
            continue;
        }
        wr::<u16>(e, 0x2C, code);
        let area = rd::<u8>(e, 0x2F);
        if u32::from(area) != g::<u32>(CURR_AREA).read() && area != 0xD {
            continue;
        }
        let mi_idx = rd::<i16>(e, 0x22) as i32;
        let mi = g::<GPtr<u8>>((MODEL_INFOS as i32).wrapping_add(mi_idx.wrapping_mul(4)) as u32 as usize).read();
        if v_model_type(mi) != 1 {
            continue;
        }
        let mut a = g::<GPtr<u8>>(FIRST_ACTIVE).read();
        let mut owned = false;
        while !a.is_null() {
            if rd::<GPtr<u8>>(a, 0) == e {
                owned = true;
                break;
            }
            a = rd::<GPtr<u8>>(a, 0x14);
        }
        if owned || !in_range(sq_dist_object(e, cam)) {
            continue;
        }
        let free = g::<GPtr<u8>>(FIRST_AVAILABLE).read();
        if free.is_null() {
            return Err(());
        }
        shadow_object_create_for(free, e, 1);
        if g::<u32>(FIRST_AVAILABLE).read() == 0 {
            return Err(());
        }
    }
    Ok(())
}

/// `CStencilShadows::RegisterStencilShadows(CVector* camPos, bool doNotCreateNew)` (0x711760, cdecl), AL.
///
/// With `doNotCreateNew`: every active object (next read first) whose owner is gone or a bad object is destroyed; AL =
/// 1. Otherwise the same walk also destroys type-1 objects beyond 2500 (`test ah, 0x41` clear) or outside the current
/// area / area 13, and type-2 objects beyond 2500. With free objects left: every vehicle pool slot in use with collision
/// shadow data (+7 bit 2), in the world (+0xB0), without a shadow object and within range (`test ah, 0x41; jp`) gets a
/// type-2 object; then over the sector rectangle of `camPos ± 50` (clamped to [0, 0x77]; a new scan code), for each
/// sector (y outer, x inner) its building list and the repeat sector's object list (`(y & 15) * 16 + (x & 15)`) as
/// [`scan_list`]. AL = 0 as soon as the free list is empty while creating, else 1.
#[hook(addr = 0x711760, conv = cdecl, name = "CStencilShadows::RegisterStencilShadows", expect = "8A 44 24 08 83 EC 1C",
       unverified = "mvpm-render-5", route = "L2", hazards = "ax-al-return,heap-writes", recs = "driving,grove-walk",
       note = "the distance helpers 0x711160 / 0x7111F0 are register-convention callees (EBX, ESI -> ST0)")]
pub fn register_stencil_shadows(cam: GPtr<u8>, do_not_create_new: u32) -> u8 {
    let mut o = g::<GPtr<u8>>(FIRST_ACTIVE).read();
    if do_not_create_new as u8 != 0 {
        while !o.is_null() {
            let next = rd::<GPtr<u8>>(o, 0x14);
            let owner = rd::<GPtr<u8>>(o, 0);
            if owner.is_null() || bad_object(owner) {
                shadow_object_destroy(o);
            }
            o = next;
        }
        return 1;
    }
    while !o.is_null() {
        let owner = rd::<GPtr<u8>>(o, 0);
        let next = rd::<GPtr<u8>>(o, 0x14);
        let destroy = if owner.is_null() || bad_object(owner) {
            true
        } else {
            match rd::<u8>(o, 6) {
                1 => {
                    let far = !sq_dist_object(owner, cam).fcom(x(kf(K_RANGE_SQ))).test_any(0x41);
                    far || {
                        let area = rd::<u8>(owner, 0x2F);
                        u32::from(area) != g::<u32>(CURR_AREA).read() && area != 0xD
                    }
                }
                2 => !sq_dist_vehicle(owner, cam).fcom(x(kf(K_RANGE_SQ))).test_any(0x41),
                _ => false,
            }
        };
        if destroy {
            shadow_object_destroy(o);
        }
        o = next;
    }
    if g::<u32>(FIRST_AVAILABLE).read() == 0 {
        return 1;
    }
    let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let size = rd::<i32>(pool, 8);
    let mut off: u32 = 0;
    for i in 0..size.max(0) {
        let flags = rd::<u32>(pool, 4);
        if (GPtr::<u8>::from_addr(flags.wrapping_add(i as u32) as usize).read() as i8) >= 0 {
            let v = GPtr::<u8>::from_addr(rd::<u32>(pool, 0).wrapping_add(off) as usize);
            if !v.is_null()
                && has_shadow_data(v)
                && rd::<u32>(v, 0xB0) != 0
                && shadow_object_find_active_for(v).is_null()
                && in_range(sq_dist_vehicle(v, cam))
            {
                let free = g::<GPtr<u8>>(FIRST_AVAILABLE).read();
                if free.is_null() {
                    return 0;
                }
                shadow_object_create_for(free, v, 2);
                if g::<u32>(FIRST_AVAILABLE).read() == 0 {
                    return 0;
                }
            }
        }
        off = off.wrapping_add(0xA18);
    }
    if g::<u32>(FIRST_AVAILABLE).read() == 0 {
        return 1;
    }
    let half = kf(K_HALF_SIZE);
    let (cx, cy) = (rd::<f32>(cam, 0), rd::<f32>(cam, 4));
    let lo_x = x(cx).fsub_m(half);
    let lo_y = x(cy).fsub_m(half).to_f32();
    let hi_x = x(cx).fadd_m(half).to_f32();
    let hi_y = x(cy).fadd_m(half).to_f32();
    let x0 = sector(lo_x, true);
    let y0 = sector(x(lo_y), true);
    let x1 = sector(x(hi_x), false);
    let y1 = sector(x(hi_y), false);
    let sc = g::<u16>(SCAN_CODE);
    if sc.read() == 0xFFFF {
        world_clear_scan_codes();
        sc.write(1);
    } else {
        sc.write(sc.read().wrapping_add(1));
    }
    let mut y = y0;
    while y <= y1 {
        let cy = if y <= 0 { 0 } else { y };
        let mut xx = x0;
        while xx <= x1 {
            let mut cxi = if xx <= 0 { 0 } else { xx };
            if cxi >= 0x77 {
                cxi = 0x77;
            }
            let cyi = if cy >= 0x77 { 0x77 } else { cy };
            let idx = (cyi * 0x78 + cxi) as u32;
            if scan_list(g::<GPtr<u8>>((SECTORS + idx * 8) as usize).read(), cam).is_err() {
                return 0;
            }
            let r = (((y & 0xF) << 4) + (xx & 0xF)) as u32;
            if scan_list(g::<GPtr<u8>>((REPEAT_OBJECTS + r * 12) as usize).read(), cam).is_err() {
                return 0;
            }
            xx += 1;
        }
        y += 1;
    }
    1
}

pub const HOOKS: &[HookReg] = &[HOOK_REGISTER_STENCIL_SHADOWS];
