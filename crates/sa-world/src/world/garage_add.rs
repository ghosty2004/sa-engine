//! STD-5: `CGarages::AddOne` (0x4471E0), ported from the 1.0 US bytes (gta-reversed only has a
//! `plugin::Call` stub). `CGarage` (0xD8, `CGarages::aGarages` 0x96C048, count `NumGarages` 0x96C024):
//! +0 base (x, y, z), +0xC / +0x10 the front direction (normalised), +0x14 / +0x18 the side direction
//! (normalised), +0x1C top z, +0x20 / +0x24 the two lengths, +0x28 min x, +0x2C max x, +0x30 min y,
//! +0x34 max y, +0x44 name (strncpy 7), byte +0x4C type, byte +0x4E flags (8, 0x10, 0x20 from the
//! argument's bits 0..2), byte +0x4F original type.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

orig! {
    /// CRT `strncpy(dst, src, n)` (0x821F40)
    fn crt_strncpy(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
}

const GARAGES: usize = 0x96C048;
const NUM_GARAGES: usize = 0x96C024;

/// `fld a; fcomp b; test ah, 5; jp`: a when a < b, else b (also for NaN).
fn lt_pick(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}
/// `test ah, 0x41; jne`: a when a > b, else b (also for NaN).
fn gt_pick(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

/// Normalises the 2D direction at `g + o` (`fsqrt` of the squares' sum stored at `g + len`; the first
/// component divided by the register value, the second by the stored length).
fn normalise(g: GPtr<u8>, o: isize, len: isize) {
    let f = |k: isize| g.byte_add(k).cast::<f32>();
    let (dx, dy) = (X87::from_f32(f(o).read()), X87::from_f32(f(o + 4).read()));
    let l = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
    f(len).write(l.to_f32());
    f(o).write(X87::from_f32(f(o).read()).fdiv(l).to_f32());
    f(o + 4).write(X87::from_f32(f(o + 4).read()).fdiv_m(f(len).read()).to_f32());
}

/// `CGarages::AddOne(x1, y1, z1, frontX, frontY, x2, y2, z2, type, unused, name, flags)` (0x4471E0):
/// fills garage `NumGarages` (its bounding rectangle from the four corners base, front, side and front
/// + side - base, each min / max in the original's comparison order) and returns its index, then
/// `NumGarages + 1`.
#[hook(addr = 0x4471E0, conv = cdecl, name = "CGarages::AddOne", expect = "51 D9 44 24 08 D8 5C 24 14 DF",
       unverified = "std5-world-1", route = "L2", hazards = "none", recs = "new-game")]
#[allow(clippy::too_many_arguments)]
pub fn add_one(
    x1: f32,
    y1: f32,
    z1: f32,
    front_x: f32,
    front_y: f32,
    x2: f32,
    y2: f32,
    z2: u32,
    ty: u32,
    _unused: u32,
    name: GPtr<u8>,
    flags: u32,
) -> u32 {
    let idx = GPtr::<u32>::from_addr(NUM_GARAGES).read();
    let g = GPtr::<u8>::from_addr(GARAGES + 0xD8 * idx as usize);
    let f = |k: isize| g.byte_add(k).cast::<f32>();
    let t = X87::from_f32(front_x).fadd_m(x2).fsub_m(x1).to_f32();
    f(0x28).write(lt_pick(lt_pick(lt_pick(x1, front_x), x2), t));
    f(0x2C).write(gt_pick(gt_pick(gt_pick(x1, front_x), x2), t));
    let t = X87::from_f32(front_y).fadd_m(y2).fsub_m(y1).to_f32();
    f(0x30).write(lt_pick(lt_pick(lt_pick(y1, front_y), y2), t));
    f(0x34).write(gt_pick(gt_pick(gt_pick(y1, front_y), y2), t));
    f(0).write(x1);
    f(4).write(y1);
    f(8).write(z1);
    f(0xC).write(X87::from_f32(front_x).fsub_m(x1).to_f32());
    f(0x10).write(X87::from_f32(front_y).fsub_m(y1).to_f32());
    f(0x14).write(X87::from_f32(x2).fsub_m(x1).to_f32());
    f(0x18).write(X87::from_f32(y2).fsub_m(y1).to_f32());
    g.byte_add(0x1C).cast::<u32>().write(z2);
    normalise(g, 0xC, 0x20);
    normalise(g, 0x14, 0x24);
    g.byte_add(0x4C).write(ty as u8);
    g.byte_add(0x4F).write(ty as u8);
    crt_strncpy(g.byte_add(0x44), name, 7);
    let fb = g.byte_add(0x4E);
    let mut b = fb.read();
    b = if flags & 1 != 0 { b | 8 } else { b & 0xF7 };
    fb.write(b);
    b = if flags & 2 != 0 { b | 0x10 } else { b & 0xEF };
    fb.write(b);
    let b = fb.read();
    fb.write(if flags & 4 != 0 { b | 0x20 } else { b & 0xDF });
    GPtr::<u32>::from_addr(NUM_GARAGES).write(idx + 1);
    idx
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_ADD_ONE];
