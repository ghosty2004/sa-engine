//! `CQuaternion::Get(RwMatrix*)` (0x59C080, batch `std11-anim-1`): the rotation matrix of a quaternion (bike rider
//! and frame-update paths). Ported from the 1.0 US bytes; unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_math::x87::X87;

fn rf(p: GPtr<u8>, off: isize) -> f32 {
    p.byte_add(off).cast::<f32>().read()
}

fn wf(p: GPtr<u8>, off: isize, v: X87) {
    p.byte_add(off).cast::<f32>().write(v.to_f32());
}

/// `CQuaternion::Get(RwMatrix* out) const` (0x59C080, thiscall, `ret 4`), x87. q = (x, y, z, w) at +0..+0xC; X2 = x+x,
/// Y2 = y+y (registers), z2 = f32(z+z). Stored f32 products: xx = X2*x, yx = Y2*x, zx = z2*x, yy = Y2*y, zy = z2*y,
/// zz = z2*z, xw = X2*w; register products YW = Y2*w, ZW = z2*w. Out: right = (1 - (zz + yy), ZW + yx, zx - YW),
/// up = (yx - ZW, 1 - (zz + xx), xw + zy), at = (YW + zx, zy - xw, 1 - (yy + xx)) (1.0 from [0x858624]); the
/// matrix's flags and padding (+0xC, +0x1C, +0x2C) and position are not written.
#[hook(addr = 0x59C080, conv = thiscall, name = "CQuaternion::Get-", expect = "83 EC 20 D9 01 8B 44 24 24",
       unverified = "std11-anim-1", route = "L2", hazards = "none", recs = "grove-walk,driving")]
pub fn quaternion_get_matrix(q: GPtr<u8>, out: GPtr<u8>) {
    let r = |v: X87| X87::from_f32(v.to_f32());
    let (x, y, z, w) = (rf(q, 0), rf(q, 4), rf(q, 8), rf(q, 0xC));
    let x2 = X87::from_f32(x).fadd(X87::from_f32(x));
    let y2 = X87::from_f32(y).fadd(X87::from_f32(y));
    let z2 = r(X87::from_f32(z).fadd(X87::from_f32(z)));
    let xx = r(x2.fmul_m(x));
    let yx = r(y2.fmul_m(x));
    let zx = r(z2.fmul_m(x));
    let yy = r(y2.fmul_m(y));
    let zy = r(z2.fmul_m(y));
    let zz = r(z2.fmul_m(z));
    let xw = r(x2.fmul_m(w));
    let yw = y2.fmul_m(w);
    let zw = z2.fmul_m(w);
    let one = X87::from_f32(GPtr::<f32>::from_addr(0x858624).read());
    wf(out, 0, zz.fadd(yy).fsubr(one));
    wf(out, 0x10, yx.fsub(zw));
    wf(out, 0x20, yw.fadd(zx));
    wf(out, 4, zw.fadd(yx));
    wf(out, 0x14, zz.fadd(xx).fsubr(one));
    wf(out, 0x24, zy.fsub(xw));
    wf(out, 8, zx.fsub(yw));
    wf(out, 0x18, xw.fadd(zy));
    wf(out, 0x28, yy.fadd(xx).fsubr(one));
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_QUATERNION_GET_MATRIX];
