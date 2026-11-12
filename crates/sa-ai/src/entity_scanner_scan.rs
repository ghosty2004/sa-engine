//! STD-7: `CEntityScanner::ScanForEntitiesInRange` (batch `std7-ai-1`), ported from the 1.0 US bytes;
//! unverified (docs/verify/HOWTO.md).
//!
//! `CEntityScanner`: +4 the frame counter, +8 its period, +0xC sixteen entity slots, +0x4C the closest entity.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CWorld::ms_nCurrentScanCode`, `CWorld::ms_aRepeatSectors` (16 x 16, 12 bytes: vehicles, peds, objects)
const SCAN_CODE: usize = 0xB7CD78;
const REPEAT_SECTORS: usize = 0xB992B8;

orig! {
    /// `CEntityScanner::Clear()` (0x5FF9D0)
    fn scanner_clear(s: GPtr<u8>) = thiscall 0x5FF9D0 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CWorld::ClearScanCodes()` (0x563470)
    fn world_clear_scan_codes() = cdecl 0x563470 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// CRT `floor(double)` (0x8219F0)
    fn floor(v: f64) -> f64 = cdecl 0x8219F0 => unported;
}

fn ldf(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// `ftol(floor(double((c ± r) * k + b)))` (k `[0x858B38]`, b `[0x858B34]`; x87, the double of the register).
fn sector_of(v: X87) -> i32 {
    ftol32(X87::from_f64_exact(floor(v.fmul_m(ldf(0x858B38)).fadd_m(ldf(0x858B34)).to_f64())))
}

/// `CEntityScanner::ScanForEntitiesInRange(int32 type, const CPed& ped)` (0x5FFA20, thiscall, `ret 8`). The
/// counter (+4) is incremented (reset to 0 above the period +8); nothing unless it was 0. Then `Clear()`; the
/// range is the larger of the ped intelligence's +0xBC / +0xC0 (+0xC0 on a tie or NaN); the sectors around the
/// ped's position (low bounds clamped to 0, high ones to 0x77, each bound computed twice as the original does);
/// a new scan code (`ClearScanCodes` at 0xFFFF) also given to the ped. For every entity of the repeat-sector list
/// `type` (0 vehicles, 1 peds, 2 objects) not yet scanned (dead peds, state 0x37, skipped for type 1 unless the
/// ped is the player): its squared distance `(dz² + dx²) + dy²` (x87) below the squared range is inserted into
/// the 16 slots sorted by distance (an empty slot takes it at once; otherwise the farther ones shift by one, the
/// count capped at 16). The filled slots then register their references, and slot 0 becomes the closest entity
/// (+0x4C, referenced).
#[hook(addr = 0x5FFA20, conv = thiscall, name = "CEntityScanner::ScanForEntitiesInRange", expect = "83 EC 64 53 8B D9 8B 43 04",
       unverified = "std7-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn scan_for_entities_in_range(this: GPtr<u8>, kind: i32, ped: GPtr<u8>) {
    let c = rd::<i32>(this, 4);
    let first = c == 0;
    let c = c.wrapping_add(1);
    wr::<i32>(this, 4, c);
    if c > rd::<i32>(this, 8) {
        wr::<i32>(this, 4, 0);
    }
    if !first {
        return;
    }
    scanner_clear(this);
    let is_player = find_player_ped(-1).addr() == ped.addr();
    let ti = rd::<GPtr<u8>>(ped, 0x47C);
    let r = if !x(rd::<f32>(ti, 0xBC)).fcom(x(rd::<f32>(ti, 0xC0))).test_any(0x41) {
        rd::<f32>(ti, 0xBC)
    } else {
        rd::<f32>(ti, 0xC0)
    };
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let pos = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
    let mut count: i32 = 0;
    let mut dists = [f32::MAX; 16];
    let low = |c: X87| if sector_of(c) > 0 { sector_of(c) } else { 0 };
    let high = |c: X87| if sector_of(c) < 0x77 { sector_of(c) } else { 0x77 };
    let x0 = low(x(rd::<f32>(pos, 0)).fsub_m(r));
    let y0 = low(x(rd::<f32>(pos, 4)).fsub_m(r));
    let x1 = high(x(r).fadd_m(rd::<f32>(pos, 0)));
    let y1 = high(x(r).fadd_m(rd::<f32>(pos, 4)));
    let sc = g::<u16>(SCAN_CODE);
    let mut code = sc.read();
    if code == 0xFFFF {
        world_clear_scan_codes();
        code = 1;
    } else {
        code += 1;
    }
    sc.write(code);
    wr::<u16>(ped, 0x2C, code);
    let slots = this.byte_add(0xC);
    let mut y = y0;
    while y <= y1 {
        let mut xx = x0;
        while xx <= x1 {
            let sector = REPEAT_SECTORS + ((((y & 0xF) << 4) + (xx & 0xF)) as usize) * 12;
            let list = match kind {
                0 => sector,
                1 => sector + 4,
                2 => sector + 8,
                _ => 0,
            };
            let mut node = g::<u32>(list).read();
            while node != 0 {
                let n = GPtr::<u8>::from_addr(node as usize);
                let e = rd::<GPtr<u8>>(n, 0);
                let code = sc.read();
                node = rd::<u32>(n, 4);
                if rd::<u16>(e, 0x2C) == code {
                    continue;
                }
                wr::<u16>(e, 0x2C, code);
                if kind == 1 && !is_player && rd::<u32>(e, 0x530) == 0x37 {
                    continue;
                }
                let em = rd::<GPtr<u8>>(e, 0x14);
                let ep = if em.is_null() { e.byte_add(4) } else { em.byte_add(0x30) };
                let dx = x(rd::<f32>(ep, 0)).fsub_m(rd::<f32>(pos, 0));
                let dy = x(rd::<f32>(ep, 4)).fsub_m(rd::<f32>(pos, 4));
                let dz = x(rd::<f32>(ep, 8)).fsub_m(rd::<f32>(pos, 8));
                let s = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy));
                if s.fcom(x(r).fmul_m(r)).test_parity(5) {
                    continue;
                }
                let mut i = 0usize;
                while i < 16 {
                    if rd::<u32>(slots, 4 * i as isize) == 0 {
                        count += 1;
                        dists[i] = s.to_f32();
                        wr::<u32>(slots, 4 * i as isize, e.addr() as u32);
                        break;
                    }
                    if !s.fcom(x(dists[i])).test_parity(5) {
                        let mut k = if xi(count).fcom(x(ldf(0x858B48))).test_parity(5) { ftol32(x(ldf(0x858B48))) } else { count } as usize;
                        while k > i {
                            wr::<u32>(slots, 4 * k as isize, rd::<u32>(slots, 4 * (k - 1) as isize));
                            dists[k] = dists[k - 1];
                            k -= 1;
                        }
                        dists[i] = s.to_f32();
                        wr::<u32>(slots, 4 * i as isize, e.addr() as u32);
                        if count < 16 {
                            count += 1;
                        }
                        break;
                    }
                    i += 1;
                }
            }
            xx += 1;
        }
        y += 1;
    }
    for i in 0..count.max(0) as isize {
        let s = rd::<GPtr<u8>>(slots, 4 * i);
        if !s.is_null() {
            register_reference(s, slots.byte_add(4 * i));
        }
    }
    let first = rd::<GPtr<u8>>(slots, 0);
    if !first.is_null() {
        wr::<u32>(this, 0x4C, first.addr() as u32);
        register_reference(first, this.byte_add(0x4C));
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_SCAN_FOR_ENTITIES_IN_RANGE];
