//! `CCheckpoints::PlaceMarker` (batch `samp-render-3`), ported from the 1.0 US bytes. Unverified
//! (docs/verify/HOWTO.md).
//!
//! `CCheckpoints::m_aCheckPtArray[32]` (0x38 each) at 0xC7F158: +0 type (word, 0x101 = free), +2 active, +4 id,
//! +8..+0xB red, green, blue, alpha, +0xC pulse period (word), +0xE rotate rate (word), +0x10 position, +0x1C
//! direction, +0x28 pulse fraction, +0x2C size, +0x30 distance to the player (2D).
#![allow(clippy::too_many_arguments)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{k, rd, wr};

orig! {
    /// `CWorld::FindPlayerCentreOfWorld(int32)` (0x56E250): a pointer to the position
    fn find_player_centre_of_world(id: i32) -> GPtr<f32> = cdecl 0x56E250 => unported;
    /// `CWorld::FindPlayerCoors(CVector* out, int32)` (0x56E010): returns `out`
    fn find_player_coors(out: GPtr<f32>, id: i32) -> GPtr<f32> = cdecl 0x56E010 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
}

const CHECKPOINTS: usize = 0xC7_F158;
const SIZE: usize = 0x38;

fn cp(i: usize) -> GPtr<u8> {
    GPtr::from_addr(CHECKPOINTS + SIZE * i)
}

fn xf(v: f32) -> X87 {
    X87::from_f32(v)
}

/// `CCheckpoints::PlaceMarker(uint32 id, uint16 type, CVector& pos, CVector& dir, float size, uint8 r, uint8 g,
/// uint8 b, uint8 a, uint16 pulsePeriod, float pulseFraction, int16 rotateRate)` (0x722C40, cdecl): `dist` = the 2D
/// distance from `FindPlayerCentreOfWorld(0)` (x87 `fsqrt`, stored as f32 into the `pos` argument slot). An active
/// checkpoint with this id is reused; for the types 3, 5 and 7 it then becomes type 8 (from 7) or 6 when the
/// player is within 2.0 in 2D (`(pos.y - D.y) * f32(pos.y - C.y) + (pos.x - B.x) * (pos.x - A.x)` with four
/// `FindPlayerCoors(-1)` calls, `fsqrt`, below 2.0 ordered). Otherwise the first free checkpoint (type 0x101);
/// otherwise the farthest checkpoint whose distance is above `dist` (ordered tests; it is marked free first);
/// otherwise null. The fields are filled (type from the possibly changed argument, active = 1), the direction
/// becomes `dir - pos` (f32 each) and is normalised (`CVector::Normalise`). Returns the checkpoint.
#[hook(addr = 0x722C40, conv = cdecl, name = "CCheckpoints::PlaceMarker", expect = "83 EC 3C 53 55 56 57",
       unverified = "samp-render-3", route = "L2", hazards = "x87-transcendental", recs = "world-cheats",
       note = "FSQRT; the original keeps dist and the changed type in its own argument slots")]
pub fn checkpoints_place_marker(id: u32, ty: u32, pos: GPtr<f32>, dir: GPtr<f32>, size: u32, r: u32, g: u32, b: u32,
    a: u32, period: u32, fraction: u32, rotate: u32) -> GPtr<u8> {
    let p = find_player_centre_of_world(0);
    let (px, py) = (p.read(), p.wrapping_index(1).read());
    let dy = xf(pos.wrapping_index(1).read()).fsub_m(py);
    let dx = xf(pos.read()).fsub_m(px);
    let dist = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt().to_f32();
    let mut ty = ty;
    let mut found = None;
    for i in 0..32 {
        let c = cp(i);
        if rd::<u8>(c, 2) != 0 && rd::<u32>(c, 4) == id {
            found = Some(c);
            break;
        }
    }
    if let Some(_c) = found {
        let t16 = ty as u16;
        if t16 == 3 || t16 == 5 || t16 == 7 {
            let (posx, posy) = (pos.read(), pos.wrapping_index(1).read());
            let mut a_ = [0f32; 3];
            let mut b_ = [0f32; 3];
            let mut c_ = [0f32; 3];
            let mut d_ = [0f32; 3];
            let pa = find_player_coors(GPtr::from_raw(a_.as_mut_ptr()), -1);
            let pb = find_player_coors(GPtr::from_raw(b_.as_mut_ptr()), -1);
            let pc = find_player_coors(GPtr::from_raw(c_.as_mut_ptr()), -1);
            let ycy = xf(posy).fsub_m(pc.wrapping_index(1).read()).to_f32();
            let pd = find_player_coors(GPtr::from_raw(d_.as_mut_ptr()), -1);
            let yprod = xf(posy).fsub_m(pd.wrapping_index(1).read()).fmul_m(ycy);
            let xprod = xf(posx).fsub_m(pb.read()).fmul(xf(posx).fsub_m(pa.read()));
            let d = yprod.fadd(xprod).fsqrt();
            if !d.fcom(xf(k(0x4000_0000))).test_parity(5) {
                ty = if t16 == 7 { 8 } else { 6 };
            }
        }
    } else {
        for i in 0..32 {
            if rd::<u16>(cp(i), 0) == 0x101 {
                found = Some(cp(i));
                break;
            }
        }
        if found.is_none() {
            let mut best: Option<GPtr<u8>> = None;
            for i in 0..32 {
                let c = cp(i);
                let cd = rd::<f32>(c, 0x30);
                if xf(dist).fcom(xf(cd)).test_parity(5) {
                    continue;
                }
                if let Some(bc) = best
                    && xf(cd).fcom(xf(rd::<f32>(bc, 0x30))).test_any(0x41)
                {
                    continue;
                }
                best = Some(c);
            }
            let Some(bc) = best else {
                return GPtr::NULL;
            };
            wr::<u16>(bc, 0, 0x101);
            found = Some(bc);
        }
    }
    let c = found.unwrap_or(GPtr::NULL);
    wr::<f32>(c, 0x30, dist);
    wr::<u32>(c, 0x2C, size);
    wr::<u8>(c, 8, r as u8);
    wr::<u8>(c, 9, g as u8);
    wr::<u8>(c, 0xB, a as u8);
    wr::<u8>(c, 0xA, b as u8);
    wr::<u16>(c, 0xE, rotate as u16);
    for i in 0..3isize {
        wr::<f32>(c, 0x10 + 4 * i, pos.wrapping_index(i as i32).read());
    }
    for i in 0..3isize {
        wr::<f32>(c, 0x1C + 4 * i, dir.wrapping_index(i as i32).read());
    }
    wr::<u32>(c, 4, id);
    wr::<u16>(c, 0, ty as u16);
    wr::<u16>(c, 0xC, period as u16);
    wr::<u8>(c, 2, 1);
    wr::<u32>(c, 0x28, fraction);
    for i in 0..3isize {
        let o = 0x1C + 4 * i;
        wr::<f32>(c, o, xf(rd::<f32>(c, o)).fsub_m(rd::<f32>(c, 0x10 + 4 * i)).to_f32());
    }
    vector_normalise(c.byte_add(0x1C));
    c
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_CHECKPOINTS_PLACE_MARKER];
