//! `CShadows::AddPermanentShadow` (0x706F60, batch `samp2-render-1`): bullet holes, blood and scorch
//! marks. Unreversed in gta-reversed; ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;

use crate::common::{g, k, x};

/// `CShadows::aPermanentShadows` (48 x 0x38 at 0xC4AC30): position +0, front x/y +0xC/+0x10, side x/y
/// +0x14/+0x18, draw distance +0x1C, up distance +0x20, creation time +0x24, lifetime +0x28, texture +0x2C,
/// intensity u16 +0x30, type +0x32 (0 = free), r/g/b +0x33..+0x35.
const SHADOWS: usize = 0xC4_AC30;

/// `x*x + y*y < 0.25` (x87, NaN fails) for the two floats at `p` (the original loads them in either order;
/// the sum is the same).
fn small(p: usize) -> bool {
    let a = x(g::<f32>(p).read());
    let b = x(g::<f32>(p + 4).read());
    !a.fmul(a).fadd(b.fmul(b)).fcom(x(k(0x3E80_0000))).test_parity(5)
}

/// `CShadows::AddPermanentShadow(uint8 type, RwTexture* tex, CVector* pos, float frontX, float frontY, float
/// sideX, float sideY, int16 intensity, uint8 r, uint8 g, uint8 b, float drawDistance, uint32 lifetime, float
/// upDistance)` (0x706F60, cdecl).
///
/// The first free slot (type 0); without one the slot with the smallest lifetime (unsigned, first wins on a
/// tie, starting from 0xFFFFFFFF) among those whose front and side vectors are both shorter than 0.5
/// (squared below 0.25); none: nothing. The slot gets every argument and the creation time
/// `CTimer::m_snTimeInMilliseconds`.
#[hook(addr = 0x706F60, conv = cdecl, name = "CShadows::AddPermanentShadow", expect = "56 57 33 FF B8 9A AC C4 00",
       unverified = "samp2-render-1", route = "L2", hazards = "none", recs = "weapon-combat")]
#[allow(clippy::too_many_arguments)]
pub fn add_permanent_shadow(ty: u32, tex: u32, pos: GPtr<u32>, front_x: u32, front_y: u32, side_x: u32, side_y: u32,
    intensity: u32, r: u32, gg: u32, b: u32, draw: u32, life: u32, up: u32) {
    let slot = |i: usize| SHADOWS + i * 0x38;
    let mut pick = (0..48).find(|&i| g::<u8>(slot(i) + 0x32).read() == 0);
    if pick.is_none() {
        let mut min = u32::MAX;
        for i in 0..48 {
            let s = slot(i);
            if small(s + 0xC) && small(s + 0x14) {
                let l = g::<u32>(s + 0x28).read();
                if l < min {
                    pick = Some(i);
                    min = l;
                }
            }
        }
    }
    let Some(i) = pick else { return };
    let s = slot(i);
    g::<u8>(s + 0x32).write(ty as u8);
    g::<u32>(s + 0x2C).write(tex);
    g::<u32>(s).write(pos.wrapping_index(0).read());
    g::<u32>(s + 4).write(pos.wrapping_index(1).read());
    g::<u32>(s + 8).write(pos.wrapping_index(2).read());
    g::<u32>(s + 0xC).write(front_x);
    g::<u32>(s + 0x10).write(front_y);
    g::<u32>(s + 0x14).write(side_x);
    g::<u32>(s + 0x18).write(side_y);
    g::<u16>(s + 0x30).write(intensity as u16);
    g::<u8>(s + 0x33).write(r as u8);
    g::<u8>(s + 0x34).write(gg as u8);
    g::<u8>(s + 0x35).write(b as u8);
    g::<u32>(s + 0x1C).write(draw);
    g::<u32>(s + 0x28).write(life);
    g::<u32>(s + 0x20).write(up);
    g::<u32>(s + 0x24).write(sa_core::timer::TIME_MS.get());
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_ADD_PERMANENT_SHADOW];
