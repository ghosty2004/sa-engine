//! `CPlantMgr::_ColEntityCache_Update` (0x5DC510), the plant manager's cache of collision entities
//! near the camera. Ported from the 1.0 US bytes (gta-reversed `PlantMgr.cpp` as reading aid); its
//! distance test calls `CPlantMgr::_CalcDistanceSqrToEntity` (0x5DBE40, EBX = entity, ESI = position,
//! the 80-bit result in ST0) through `orig! { ... = reg("args = ebx esi") }` (HARNESS-5).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, k, rd, wr, x};

orig! {
    /// `CPlantMgr::_CalcDistanceSqrToEntity(CEntity*, const CVector*)` (0x5DBE40): EBX = entity, ESI =
    /// position, ST0 result
    fn calc_distance_sqr_to_entity(entity: GPtr<u8>, pos: GPtr<u8>) -> X87 = reg("args = ebx esi") 0x5DBE40 => crate::harness5::calc_distance_sqr_to_entity;
    /// `CPlantColEntEntry::ReleaseEntry()` (0x5DB8A0)
    fn release_entry(e: GPtr<u8>) = thiscall 0x5DB8A0 => crate::mvpb_r1::plant_col_ent_entry_release_entry;
    /// `CPlantColEntEntry::AddEntry(CEntity*)` (0x5DB7D0)
    fn add_entry(e: GPtr<u8>, entity: GPtr<u8>) -> GPtr<u8> = thiscall 0x5DB7D0 => crate::mvpb_r1::plant_col_ent_entry_add_entry;
    /// `CWorld::ClearScanCodes()` (0x563470; the port is unhookable)
    fn clear_scan_codes() = cdecl 0x563470 => sa_world::world::cworld::clear_scan_codes;
    /// `CBaseModelInfo::GetModelType()` (vtable +0x10)
    fn mi_get_model_type(mi: GPtr<u8>) -> u8 = thiscall vslot 0x10 => unported;
}

/// `CPlantMgr::m_CloseColEntListHead` / `m_UnusedColEntListHead`.
const CLOSE_HEAD: usize = 0xC0_399C;
const UNUSED_HEAD: usize = 0xC0_3998;
/// `CGame::currArea`, `CWorld::ms_nCurrentScanCode`, `CWorld::ms_aSectors`, `CModelInfo::ms_modelInfoPtrs`.
const CURR_AREA: usize = 0xB7_2914;
const SCAN_CODE: usize = 0xB7_CD78;
const SECTORS: usize = 0xB7_D0B8;
const MODEL_INFO_PTRS: usize = 0xA9_B0C8;
/// 340.0f (0x85F074), 115600.0f = 340² (0x86BF20), 0.02f (0x858B38), 60.0f (0x858B34).
const RANGE: u32 = 0x43AA_0000;
const RANGE_SQ: u32 = 0x47E1_C800;
const SECTOR_SCALE: u32 = 0x3CA3_D70A;
const SECTOR_OFFSET: u32 = 0x4270_0000;

/// CRT `floor` (0x8219F0) of the double the original passes (exact in f64), `_ftol`.
fn floor_ftol(v: X87) -> i32 {
    ftol32(X87::from_f64_exact(sa_world::physical::base::floor_f64(v.to_f64())))
}

/// A sector bound: `t = v * 0.02 + 60` (the f32 kept in the original's argument / local slot), `ftol(floor(t))`
/// from the unrounded value; in range (`> 0` for a minimum, `< 119` for a maximum) recomputed from the
/// stored f32, else 0 / 119.
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

/// `d <= 340²` for `_CalcDistanceSqrToEntity`'s result (`fcomp; test ah, 0x41`): the first loop
/// releases on "neither C0 nor C3" (greater; NaN keeps), the add test (`jp`) adds on exactly one (less
/// or equal; NaN does not add).
fn within(d: X87) -> (bool, bool) {
    let c = d.fcom(x(k(RANGE_SQ)));
    (c.test_any(0x41), !c.test_parity(0x41))
}

/// `CPlantMgr::_ColEntityCache_Update(const CVector& cam, bool fast)` (0x5DC510, cdecl, AL result).
/// `fast`: every close entry without an entity is released; 1. Otherwise every close entry is
/// released when it has no entity, is farther than 340 (squared distance > 115600 by 0x5DBE40) or its
/// entity's area (+0x2F) is neither `CGame::currArea` nor 13. Without unused entries: 1. Then the
/// sectors of `cam ± 340` (bounds through `floor(v * 0.02 + 60)`, clamped to 0..119; x from the
/// unrounded `cam.x - 340`, y / maxima from their stored f32) get a new scan code (0xFFFF wraps to 1
/// after `CWorld::ClearScanCodes`), and every building of `CWorld::ms_aSectors[y][x]` (list 0) that
/// is not flagged 0x4000000 (+0x1C) and not yet scanned is marked; in the area (as above), with an
/// atomic model info (type 1) whose flags byte +0x13 has bit 1, not already in the close list and
/// within 340, it takes an unused entry (`AddEntry`); running out of unused entries returns 0. 1.
/// The original stores the sector temporaries in its own `fast` argument slot (0x5DC5FD..).
#[hook(addr = 0x5DC510, conv = cdecl, name = "CPlantMgr::_ColEntityCache_Update", expect = "8A 44 24 08 83 EC 20",
       unverified = "regconv-render-1", route = "L2", hazards = "heap-writes,stack-residue", recs = "grove-walk,driving",
       note = "calls _CalcDistanceSqrToEntity (EBX/ESI -> 80-bit ST0) through orig! reg; writes the scan codes and the entry lists; the sector temporaries go into its own argument slot")]
pub fn col_entity_cache_update(cam: GPtr<u8>, fast: u8) -> u8 {
    if fast != 0 {
        let mut e = g::<GPtr<u8>>(CLOSE_HEAD).read();
        while !e.is_null() {
            let next = rd::<GPtr<u8>>(e, 0xC);
            if rd::<u32>(e, 0) == 0 {
                release_entry(e);
            }
            e = next;
        }
        return 1;
    }
    let in_area = |ent: GPtr<u8>| {
        let a = rd::<u8>(ent, 0x2F);
        u32::from(a) == g::<u32>(CURR_AREA).read() || a == 0xD
    };
    let mut e = g::<GPtr<u8>>(CLOSE_HEAD).read();
    while !e.is_null() {
        let ent = rd::<GPtr<u8>>(e, 0);
        let next = rd::<GPtr<u8>>(e, 0xC);
        let keep = !ent.is_null() && within(calc_distance_sqr_to_entity(ent, cam)).0 && in_area(ent);
        if !keep {
            release_entry(e);
        }
        e = next;
    }
    if g::<u32>(UNUSED_HEAD).read() == 0 {
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
    let code = if code < 0xFFFF {
        code + 1
    } else {
        clear_scan_codes();
        1
    };
    g::<u16>(SCAN_CODE).write(code);
    let mut yi = y0;
    while yi <= y1 {
        let yc = yi.clamp(0, 0x77);
        let mut xi = x0;
        while xi <= x1 {
            let xc = xi.clamp(0, 0x77);
            let mut node = g::<GPtr<u8>>(SECTORS + 8 * (yc * 0x78 + xc) as usize).read();
            while !node.is_null() {
                let ent = rd::<GPtr<u8>>(node, 0);
                node = rd::<GPtr<u8>>(node, 4);
                let code = g::<u16>(SCAN_CODE).read();
                if rd::<u32>(ent, 0x1C) & 0x400_0000 != 0 || rd::<u16>(ent, 0x2C) == code {
                    continue;
                }
                wr::<u16>(ent, 0x2C, code);
                if !in_area(ent) {
                    continue;
                }
                let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * rd::<i16>(ent, 0x22) as isize as usize).read();
                if mi_get_model_type(mi) != 1 || rd::<u8>(mi, 0x13) & 2 == 0 {
                    continue;
                }
                let mut c = g::<GPtr<u8>>(CLOSE_HEAD).read();
                let mut cached = false;
                while !c.is_null() {
                    if rd::<GPtr<u8>>(c, 0) == ent {
                        cached = true;
                        break;
                    }
                    c = rd::<GPtr<u8>>(c, 0xC);
                }
                if cached || !within(calc_distance_sqr_to_entity(ent, cam)).1 {
                    continue;
                }
                let unused = g::<GPtr<u8>>(UNUSED_HEAD).read();
                if unused.is_null() {
                    return 0;
                }
                add_entry(unused, ent);
                if g::<u32>(UNUSED_HEAD).read() == 0 {
                    return 0;
                }
            }
            xi += 1;
        }
        yi += 1;
    }
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_COL_ENTITY_CACHE_UPDATE];
