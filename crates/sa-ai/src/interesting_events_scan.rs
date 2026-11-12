//! WRITER-NATIVE (batch `native-20`): `CInterestingEvents::ScanForNearbyEntities` (0x605A30), unreversed in
//! gta-reversed. Ported from the 1.0 US bytes; x87 arithmetic follows the original's instruction order.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CVector::NormaliseAndMag()` (0x59C970, ST0)
    fn vector_normalise_and_mag(v: GPtr<u8>) -> f64 = thiscall 0x59C970 => unported;
    /// CRT `floor(double)` (0x8219F0)
    fn floor(v: f64) -> f64 = cdecl 0x8219F0 => unported;
    /// `CWorld::ClearScanCodes()` (0x563470)
    fn world_clear_scan_codes() = cdecl 0x563470 => unported;
    /// `CInterestingEvents::Add(EType, CEntity*)` (0x602590)
    fn interesting_events_add(ie: GPtr<u8>, ty: i32, e: GPtr<u8>) = thiscall 0x602590 => crate::interesting_events_add::interesting_events_add;
}

/// `CWorld::ms_aRepeatSectors` (0xB992B8, 16 x 16 x 3 lists)
const REPEAT_SECTORS: usize = 0xB9_92B8;
/// `CWorld::ms_nCurrentScanCode` (0xB7CD78)
const SCAN_CODE: usize = 0xB7_CD78;

/// `ftol(floor(double(v * [0x858B38] + [0x858B34])))` (the double of the x87 register).
fn sector_of(v: X87) -> i32 {
    let k = g::<f32>(0x85_8B38).read();
    let b = g::<f32>(0x85_8B34).read();
    ftol32(X87::from_f64_exact(floor(v.fmul_m(k).fadd_m(b).to_f64())))
}

/// The entity's position (`m_matrix` +0x30, else the placement position +4).
fn pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// The next scan code (`ClearScanCodes` and 1 after 0xFFFF), stored.
fn next_scan_code() -> u16 {
    let c = g::<u16>(SCAN_CODE).read();
    let n = if c < 0xFFFF {
        c + 1
    } else {
        world_clear_scan_codes();
        1
    };
    g::<u16>(SCAN_CODE).write(n);
    n
}

/// `CInterestingEvents::ScanForNearbyEntities()` (0x605A30, thiscall).
///
/// Only when enabled (byte 0xC0B184 bit 0) and at least 500 ms after the last scan (+0x134). Once per frame
/// (+0x130): the look direction +0x148 = `player - active camera source` with z = 0, normalised (the player's
/// forward vector when its length is exactly 0), and the scan centre +0x13C = player + direction * radius
/// (+0x138). The repeat sectors around the centre (`centre -/+ radius`, clamped to [0, 119]) are walked with a
/// new scan code (also given to the player): peds not in state 0x37 add an event for themselves or their
/// vehicle (cop 6 -> type 5, 0x14 -> 6, 0x16 -> 4, gangs 7..16 -> 7); vehicles without bit 29 of +0x40, with a
/// driver and a siren state +0x3B9 other than 0 / 6 add type 0xE.
#[hook(addr = 0x605A30, conv = thiscall, name = "CInterestingEvents::ScanForNearbyEntities",
       expect = "55 8B EC 83 E4 F8 83 EC 28",
       unverified = "native-20", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "CRT floor; scan codes; adds interesting events")]
pub fn interesting_events_scan_for_nearby_entities(this: GPtr<u8>) {
    if g::<u8>(0xC0_B184).read() & 1 == 0 {
        return;
    }
    let now = g::<u32>(0xB7_CB84).read();
    if now.wrapping_sub(rd::<u32>(this, 0x134)) < 0x1F4 {
        return;
    }
    wr::<u32>(this, 0x134, now);
    let player = find_player_ped(-1);
    let frame = g::<u32>(0xB7_CB4C).read();
    if rd::<u32>(this, 0x130) != frame {
        wr::<u32>(this, 0x130, frame);
        let active = u32::from(g::<u8>(0xB6_F081).read());
        let src = GPtr::<f32>::from_addr((0xB6_F338 + active * 0x238) as usize);
        let s = [src.read(), src.wrapping_index(1).read(), src.wrapping_index(2).read()];
        let pp = pos(player);
        let dir = this.byte_add(0x148);
        wr::<f32>(dir, 0, x(rd::<f32>(pp, 0)).fsub_m(s[0]).to_f32());
        wr::<f32>(dir, 4, x(rd::<f32>(pp, 4)).fsub_m(s[1]).to_f32());
        wr::<f32>(dir, 8, x(rd::<f32>(pp, 8)).fsub_m(s[2]).to_f32());
        wr::<u32>(this, 0x150, 0);
        let mag = X87::from_f64_exact(vector_normalise_and_mag(dir));
        if !mag.fcom(x(0.0)).test_parity(0x44) {
            let fwd = rd::<GPtr<u8>>(player, 0x14).byte_add(0x10);
            for o in [0, 4, 8] {
                wr::<u32>(dir, o, rd::<u32>(fwd, o));
            }
        }
        let r = x(rd::<f32>(this, 0x138));
        let tx = r.fmul_m(rd::<f32>(dir, 0)).to_f32();
        let ty = r.fmul_m(rd::<f32>(dir, 4)).to_f32();
        let tz = r.fmul_m(rd::<f32>(dir, 8));
        let pp = pos(player);
        wr::<f32>(this, 0x13C, x(tx).fadd_m(rd::<f32>(pp, 0)).to_f32());
        wr::<f32>(this, 0x140, x(ty).fadd_m(rd::<f32>(pp, 4)).to_f32());
        wr::<f32>(this, 0x144, tz.fadd_m(rd::<f32>(pp, 8)).to_f32());
    }
    let r = rd::<f32>(this, 0x138);
    let (cx, cy) = (rd::<f32>(this, 0x13C), rd::<f32>(this, 0x140));
    let x0 = sector_of(x(cx).fsub_m(r)).max(0);
    let y0 = sector_of(x(cy).fsub_m(r)).max(0);
    let x1 = sector_of(x(r).fadd_m(cx)).min(0x77);
    let y1 = sector_of(x(r).fadd_m(cy)).min(0x77);
    let code = next_scan_code();
    wr::<u16>(player, 0x2C, code);
    let ie: GPtr<u8> = GPtr::from_addr(0xC0_B058);
    for yy in y0..=y1 {
        for xx in x0..=x1 {
            let sector = REPEAT_SECTORS + ((((yy & 0xF) << 4) + (xx & 0xF)) as usize) * 12;
            let mut node = g::<GPtr<u8>>(sector + 4).read();
            while !node.is_null() {
                let e = rd::<GPtr<u8>>(node, 0);
                node = rd::<GPtr<u8>>(node, 4);
                let cur = g::<u16>(SCAN_CODE).read();
                if rd::<u16>(e, 0x2C) == cur {
                    continue;
                }
                wr::<u16>(e, 0x2C, cur);
                if rd::<u32>(e, 0x530) == 0x37 {
                    continue;
                }
                let v = rd::<GPtr<u8>>(e, 0x58C);
                let target = if rd::<u32>(e, 0x46C) & 0x100 != 0 && !v.is_null() { v } else { e };
                let ty = match rd::<i32>(e, 0x598) {
                    6 => 5,
                    0x14 => 6,
                    0x16 => 4,
                    7..=0x10 => 7,
                    _ => continue,
                };
                interesting_events_add(ie, ty, target);
            }
            let mut node = g::<GPtr<u8>>(sector).read();
            while !node.is_null() {
                let e = rd::<GPtr<u8>>(node, 0);
                node = rd::<GPtr<u8>>(node, 4);
                let cur = g::<u16>(SCAN_CODE).read();
                if rd::<u16>(e, 0x2C) == cur {
                    continue;
                }
                wr::<u16>(e, 0x2C, cur);
                if rd::<u32>(e, 0x40) & 0x2000_0000 != 0 || rd::<u32>(e, 0x460) == 0 {
                    continue;
                }
                let siren = rd::<u8>(e, 0x3B9);
                if siren == 0 || siren == 6 {
                    continue;
                }
                interesting_events_add(ie, 0xE, e);
            }
        }
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_INTERESTING_EVENTS_SCAN_FOR_NEARBY_ENTITIES];
