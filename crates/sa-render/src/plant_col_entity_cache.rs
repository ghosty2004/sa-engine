//! `CPlantMgr::_ColEntityCache_Update` (batch `mvpm-render-3`): keeps the list of nearby plant-bearing buildings up
//! to date. Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
//!
//! `CPlantColEntEntry` (from the bytes): +0 entity, +0xC next. The close list head is 0xC0399C, the unused list head
//! 0xC03998.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd, wr, x};

orig! {
    /// `CPlantColEntEntry::ReleaseEntry()` (0x5DB8A0)
    fn col_ent_release_entry(e: GPtr<u8>) = thiscall 0x5DB8A0 => unported;
    /// `CPlantColEntEntry::AddEntry(CEntity*)` (0x5DB7D0, `ret 4`)
    fn col_ent_add_entry(e: GPtr<u8>, entity: GPtr<u8>) -> GPtr<u8> = thiscall 0x5DB7D0 => unported;
    /// `CPlantMgr::_CalcDistanceSqrToEntity(CEntity* (EBX), const CVector* (ESI))` (0x5DBE40, the full ST0 result)
    fn calc_distance_sqr_to_entity(e: GPtr<u8>, pos: GPtr<u8>) -> X87 = reg("args = ebx esi") 0x5DBE40 => crate::harness5::calc_distance_sqr_to_entity;
    /// `CWorld::ClearScanCodes()` (0x563470)
    fn world_clear_scan_codes() = cdecl 0x563470 => unported;
    /// `CBaseModelInfo::GetModelType()` (vtable +0x10), AL
    fn v_model_type(mi: GPtr<u8>) -> u8 = thiscall vslot 0x10 => unported;
}

const CLOSE_HEAD: usize = 0xC0399C;
const UNUSED_HEAD: usize = 0xC03998;
/// The cache radius around the camera (0x85F074)
const K_RADIUS: usize = 0x85F074;
/// The squared distance limit (0x86BF20)
const K_LIMIT: usize = 0x86BF20;
/// Sector scale (0x858B38) and offset (0x858B34): `x * scale + offset`
const K_SECTOR_SCALE: usize = 0x858B38;
const K_SECTOR_OFFSET: usize = 0x858B34;
/// `CGame::currArea` (0xB72914)
const CURR_AREA: usize = 0xB72914;
/// `CWorld::ms_nCurrentScanCode` (u16, 0xB7CD78)
const SCAN_CODE: usize = 0xB7CD78;
/// `CWorld::ms_aSectors` (0xB7D0B8, 120 x 120, 8 bytes: +0 buildings list)
const SECTORS: u32 = 0xB7D0B8;
/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8)
const MODEL_INFOS: u32 = 0xA9B0C8;

/// CRT `floor` (0x8219F0) on a double.
fn floor_d(v: f64) -> f64 {
    if v.is_nan() || v.is_infinite() || v.abs() >= 4_503_599_627_370_496.0 {
        return v;
    }
    let t = v as i64 as f64;
    if t > v { t - 1.0 } else { t }
}
/// `_ftol(floor(v))`.
fn floor_i(v: X87) -> i32 {
    X87::from_f64_exact(floor_d(v.to_f64())).ftol() as u32 as i32
}
fn kf(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// One sector bound: `v = f32(c * scale + offset)` (`fst`); the test uses `_ftol(floor(v))` of the register value,
/// the result `_ftol(floor(v))` of the stored f32 (`low`: positive test, else 0; high: below 0x77, else 0x77).
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

/// `CPlantMgr::_ColEntityCache_Update(const CVector& cameraPos, bool fast)` (0x5DC510, cdecl), AL.
///
/// Fast: every close-list entry without an entity is released (next read first); AL = 1. Otherwise every close-list
/// entry is released when its entity is gone, its `_CalcDistanceSqrToEntity` is above the limit (`test ah, 0x41`
/// clear), or its area (+0x2F) is neither `CGame::currArea` nor 13. With an empty unused list: AL = 1. Else the sector
/// rectangle of `camera ± radius` (x - r, y - r, x + r, y + r stored as f32 in that order, the low x converted from the
/// unrounded register) clamped to [0, 0x77], a new scan code (`ClearScanCodes` and 1 at 0xFFFF), and for each sector
/// (y outer, x inner) each building (+0x1C bit 26 clear, scan code not current; the scan code is set) in the current
/// area or area 13 whose model info is atomic (`GetModelType() == 1`) with +0x13 bit 1, not yet in the close list, and
/// within the limit (`test ah, 0x41; jp`): `AddEntry` on the unused head; AL = 0 when the unused list is (or becomes)
/// empty. AL = 1 at the end.
#[hook(addr = 0x5dc510, conv = cdecl, name = "CPlantMgr::_ColEntityCache_Update", expect = "8A 44 24 08 83 EC 20",
       unverified = "mvpm-render-3", route = "L2", hazards = "ax-al-return,heap-writes", recs = "grove-walk",
       note = "AL result with the upper EAX bytes of the last value; _CalcDistanceSqrToEntity is a register-convention callee (EBX, ESI)")]
pub fn col_entity_cache_update(cam: GPtr<u8>, fast: u32) -> u8 {
    if fast as u8 != 0 {
        let mut e = g::<GPtr<u8>>(CLOSE_HEAD).read();
        while !e.is_null() {
            let next = rd::<GPtr<u8>>(e, 0xC);
            if rd::<u32>(e, 0) == 0 {
                col_ent_release_entry(e);
            }
            e = next;
        }
        return 1;
    }
    let limit = x(kf(K_LIMIT));
    let mut e = g::<GPtr<u8>>(CLOSE_HEAD).read();
    while !e.is_null() {
        let ent = rd::<GPtr<u8>>(e, 0);
        let next = rd::<GPtr<u8>>(e, 0xC);
        let keep = !ent.is_null()
            && calc_distance_sqr_to_entity(ent, cam).fcom(limit).test_any(0x41)
            && {
                let area = rd::<u8>(rd::<GPtr<u8>>(e, 0), 0x2F);
                u32::from(area) == g::<u32>(CURR_AREA).read() || area == 0xD
            };
        if !keep {
            col_ent_release_entry(e);
        }
        e = next;
    }
    if g::<u32>(UNUSED_HEAD).read() == 0 {
        return 1;
    }
    let r = kf(K_RADIUS);
    let (cx, cy) = (rd::<f32>(cam, 0), rd::<f32>(cam, 4));
    let lo_x = x(cx).fsub_m(r);
    let lo_y = x(cy).fsub_m(r).to_f32();
    let hi_x = x(cx).fadd_m(r).to_f32();
    let hi_y = x(cy).fadd_m(r).to_f32();
    let start_x = sector(lo_x, true);
    let start_y = sector(x(lo_y), true);
    let end_x = sector(x(hi_x), false);
    let end_y = sector(x(hi_y), false);
    let sc = g::<u16>(SCAN_CODE);
    let mut code = sc.read();
    if code < 0xFFFF {
        code += 1;
    } else {
        world_clear_scan_codes();
        code = 1;
    }
    sc.write(code);
    let mut yy = start_y;
    while yy <= end_y {
        let cy = if yy <= 0 { 0 } else { yy };
        let mut xx = start_x;
        while xx <= end_x {
            let mut cxi = if xx <= 0 { 0 } else { xx };
            if cxi >= 0x77 {
                cxi = 0x77;
            }
            let cyi = if cy >= 0x77 { 0x77 } else { cy };
            let idx = (cyi * 0x78 + cxi) as u32;
            let mut node = g::<GPtr<u8>>((SECTORS + idx * 8) as usize).read();
            while !node.is_null() {
                let ent = rd::<GPtr<u8>>(node, 0);
                node = rd::<GPtr<u8>>(node, 4);
                if rd::<u32>(ent, 0x1C) & 0x400_0000 != 0 || rd::<u16>(ent, 0x2C) == code {
                    code = sc.read();
                    continue;
                }
                wr::<u16>(ent, 0x2C, code);
                let area = rd::<u8>(ent, 0x2F);
                if u32::from(area) == g::<u32>(CURR_AREA).read() || area == 0xD {
                    let mi_idx = rd::<i16>(ent, 0x22) as i32;
                    let mi = g::<GPtr<u8>>((MODEL_INFOS as i32).wrapping_add(mi_idx.wrapping_mul(4)) as u32 as usize).read();
                    if v_model_type(mi) == 1 && rd::<u8>(mi, 0x13) & 2 != 0 {
                        let mut c = g::<GPtr<u8>>(CLOSE_HEAD).read();
                        let mut known = false;
                        while !c.is_null() {
                            if rd::<GPtr<u8>>(c, 0) == ent {
                                known = true;
                                break;
                            }
                            c = rd::<GPtr<u8>>(c, 0xC);
                        }
                        if !known
                            && !calc_distance_sqr_to_entity(ent, cam).fcom(limit).test_parity(0x41)
                        {
                            let u = g::<GPtr<u8>>(UNUSED_HEAD).read();
                            if u.is_null() {
                                return 0;
                            }
                            col_ent_add_entry(u, ent);
                            if g::<u32>(UNUSED_HEAD).read() == 0 {
                                return 0;
                            }
                        }
                    }
                }
                code = sc.read();
            }
            xx += 1;
        }
        yy += 1;
    }
    1
}

pub const HOOKS: &[HookReg] = &[HOOK_COL_ENTITY_CACHE_UPDATE];
