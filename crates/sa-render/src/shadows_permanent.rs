//! `CShadows::AddPermanentShadow` (PEDHIT: the blood pool under a ped a car killed). Ported from the 1.0 US bytes;
//! gta-reversed only wraps the original (`.Reversed = false`).
#![allow(clippy::too_many_arguments)]

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

use crate::common::{g, rd, wr};

/// `CShadows::aPermanentShadows` (48 entries of 0x38 bytes): +0 position, +0xC front x / y, +0x14 side x / y,
/// +0x1C draw distance, +0x20 up distance, +0x24 creation time, +0x28 lifetime, +0x2C texture, +0x30 intensity
/// (int16), +0x32 type (0 = free), +0x33..+0x35 colour.
const PERMANENT_SHADOWS: usize = 0xC4_AC30;
const COUNT: usize = 48;
const SIZE: usize = 0x38;

/// `x² + y²` of two adjacent floats of `e`, `< 0.25f` (0x858C84; NaN fails).
fn small(e: GPtr<u8>, off: isize) -> bool {
    let a = X87::from_f32(rd::<f32>(e, off));
    let b = X87::from_f32(rd::<f32>(e, off + 4));
    !a.fmul(a).fadd(b.fmul(b)).fcom(X87::from_f32(0.25)).test_parity(5)
}

/// `CShadows::AddPermanentShadow(uint8 type, RwTexture*, CVector* pos, float frontX, float frontY, float sideX,
/// float sideY, int16 intensity, uint8 r, uint8 g, uint8 b, float drawDistance, uint32 time, float upDistance)`
/// (0x706F60, cdecl): the first free entry (type 0); with none, the entry with the smallest creation time (strictly
/// smaller, unsigned) among those whose front and side vectors both have a squared length below 0.25; with none of
/// those either, nothing. The entry takes the arguments and `CTimer::m_snTimeInMilliseconds` as its creation time.
#[hook(addr = 0x706F60, conv = cdecl, name = "CShadows::AddPermanentShadow", expect = "56 57 33 FF B8 9A AC C4 00",
       unverified = "pedhit-1", route = "L2", hazards = "none", recs = "driving")]
pub fn shadows_add_permanent_shadow(
    ty: u32,
    tex: u32,
    pos: GPtr<u8>,
    front_x: u32,
    front_y: u32,
    side_x: u32,
    side_y: u32,
    intensity: u32,
    r: u32,
    gr: u32,
    b: u32,
    draw_distance: u32,
    time: u32,
    up_distance: u32,
) {
    let entry = |i: usize| g::<u8>(PERMANENT_SHADOWS + i * SIZE);
    let mut slot = (0..COUNT).find(|&i| rd::<u8>(entry(i), 0x32) == 0);
    if slot.is_none() {
        let mut oldest = u32::MAX;
        for i in 0..COUNT {
            let e = entry(i);
            if small(e, 0xC) && small(e, 0x14) && rd::<u32>(e, 0x24) < oldest {
                slot = Some(i);
                oldest = rd::<u32>(e, 0x24);
            }
        }
    }
    let Some(i) = slot else { return };
    let e = entry(i);
    wr::<u8>(e, 0x32, ty as u8);
    wr::<u32>(e, 0x2C, tex);
    for o in [0, 4, 8] {
        wr::<u32>(e, o, rd::<u32>(pos, o));
    }
    wr::<u32>(e, 0xC, front_x);
    wr::<u32>(e, 0x10, front_y);
    wr::<u32>(e, 0x14, side_x);
    wr::<u32>(e, 0x18, side_y);
    wr::<u16>(e, 0x30, intensity as u16);
    wr::<u8>(e, 0x33, r as u8);
    wr::<u8>(e, 0x34, gr as u8);
    wr::<u8>(e, 0x35, b as u8);
    wr::<u32>(e, 0x1C, draw_distance);
    wr::<u32>(e, 0x28, time);
    wr::<u32>(e, 0x20, up_distance);
    wr::<u32>(e, 0x24, g::<u32>(0xB7_CB84).read());
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_SHADOWS_ADD_PERMANENT_SHADOW];
