//! FRONTIER-2 (batch `frontier2-1`): `CTrailer::SetupSuspensionLines` (0x6CF1A0), user-hit at the level crossing.
//! Ported from the 1.0 US bytes; gta-reversed differs twice, the bytes are followed: the wheel-base average is
//! `|y0 + y1| * 0.5` (not `(|y0| + |y1|) * 0.5`), and all four wheel positions get the support height.

use sa_core::abi::HookReg;
use sa_core::bindings::{memory_malloc, rw_matrix_transform};
use sa_core::field::gf;
use sa_core::hook;
use sa_math::fpu::{fabs, gt, lt};
use sa_stream::bindings::get_wheel_posn;

use crate::bindings::automobile_setup_suspension_lines;
use crate::common::*;
use crate::mvpd_v2::magnitude;
use sa_core::mem::GPtr;

/// `CTrailer::SetupSuspensionLines()` (0x6CF1A0).
///
/// A baggage trailer (+0x9EC == -1000.0) uses `CAutomobile::SetupSuspensionLines`. Otherwise the collision data gets
/// six lines (allocated once, `CMemoryMgr::Malloc(0xC0, 0)`). Per wheel: the wheel z (+0x838), the line from the
/// wheel position raised by the upper limit (handling +0xB8) down by `(lower - upper) - wheelSize / 2`, the spring
/// length (+0x878) and line length (+0x888). The box min z takes line 0's end when lower; the sphere radius the larger
/// corner magnitude when larger. The node +0x698's frame, transformed up its parents, gives +0x9E8. Support lines 4 /
/// 5 run from `(±min.x, max.y, max.z)` to `min.z`; with `ratio = |y4| / (|y0 + y1| * 0.5 + |y4|)` the ride height
/// `spring0 * (1 - ratio / (force * 4)) + (wheelSize / 2 - z0)` goes to +0x898 / +0x89C, the support lines' end z to
/// `z4 - (z4 + h) / (1 - (1 - ratio) / (2 * [0x8D3464]))`, and every wheel position to `wheelSize / 2 - h`.
#[hook(addr = 0x6CF1A0, conv = thiscall, name = "CTrailer::SetupSuspensionLines", expect = "83 EC 5C 53 8B D9 0F BF",
       unverified = "frontier2-1", route = "L2", hazards = "heap-writes")]
#[allow(clippy::too_many_lines)]
pub fn trailer_setup_suspension_lines(this: GPtr<u8>) {
    let mi = g::<GPtr<u8>>(0xA9_B0C8 + 4 * rd::<i16>(this, 0x22) as usize).read();
    let cm = entity_get_col_model(this);
    let cd = rd::<GPtr<u8>>(cm, 0x2C);
    if rd::<u32>(this, 0x9EC) == 0xC47A_0000 {
        automobile_setup_suspension_lines(this);
        return;
    }
    if rd::<u32>(cd, 0x10) == 0 {
        wr::<u8>(cd, 6, 6);
        wr(cd, 0x10, memory_malloc(0xC0, 0));
    }
    let handling = || rd::<GPtr<u8>>(this, 0x384);
    let lines = || rd::<GPtr<u8>>(cd, 0x10);
    for i in 0..4isize {
        let mut w = [0f32; 3];
        get_wheel_posn(mi, i as i32, GPtr::from_raw(w.as_mut_ptr()).cast(), 0);
        wr(this, 0x838 + 4 * i, w[2]);
        w[2] = x(w[2]).fadd_m(rd::<f32>(handling(), 0xB8)).to_f32();
        let l = lines().byte_add(0x20 * i);
        wr(l, 0, w[0]);
        wr(l, 4, w[1]);
        wr(l, 8, w[2]);
        let h = handling();
        let d = x(rd::<f32>(h, 0xBC)).fsub_m(rd::<f32>(h, 0xB8)).fsub(x(rd::<f32>(mi, 0x40)).fmul_m(gf(0x85_8B8C)));
        w[2] = d.fadd_m(w[2]).to_f32();
        let l = lines().byte_add(0x20 * i);
        wr(l, 0x10, w[0]);
        wr(l, 0x14, w[1]);
        wr(l, 0x18, w[2]);
        let h = handling();
        wr(this, 0x878 + 4 * i, x(rd::<f32>(h, 0xB8)).fsub_m(rd::<f32>(h, 0xBC)).to_f32());
        let l = lines().byte_add(0x20 * i);
        wr(this, 0x888 + 4 * i, x(rd::<f32>(l, 8)).fsub_m(rd::<f32>(l, 0x18)).to_f32());
    }
    if lt(x(rd::<f32>(lines(), 0x18)), x(rd::<f32>(cm, 8))) {
        wr(cm, 8, rd::<u32>(lines(), 0x18));
    }
    let mut m = magnitude(cm);
    if !gt(m, magnitude(cm.byte_add(0xC))) {
        m = magnitude(cm.byte_add(0xC));
    }
    if gt(m, x(rd::<f32>(cm, 0x24))) {
        wr(cm, 0x24, m.to_f32());
    }
    let node = rd::<GPtr<u8>>(this, 0x698);
    if !node.is_null() {
        let mut mat = [0u32; 16];
        for (k, v) in mat.iter_mut().enumerate() {
            *v = rd::<u32>(node, 0x10 + 4 * k as isize);
        }
        let mp: GPtr<u8> = GPtr::from_raw(mat.as_mut_ptr()).cast();
        let mut p = rd::<GPtr<u8>>(node, 4);
        while !p.is_null() {
            rw_matrix_transform(mp, p.byte_add(0x10), 2);
            p = rd::<GPtr<u8>>(p, 4);
            if p == node {
                break;
            }
        }
        wr(this, 0x9E8, mat[14]);
    }
    let (min_x, max_y, max_z, min_z) = (rd::<u32>(cm, 0), rd::<u32>(cm, 0x10), rd::<u32>(cm, 0x14), rd::<u32>(cm, 8));
    let put = |off: isize, v: [u32; 3]| {
        let l = lines().byte_add(off);
        wr(l, 0, v[0]);
        wr(l, 4, v[1]);
        wr(l, 8, v[2]);
    };
    put(0x80, [min_x, max_y, max_z]);
    put(0x90, [min_x, max_y, min_z]);
    let neg = x(f32::from_bits(min_x)).fmul_m(gf(0x85_8C1C)).to_f32().to_bits();
    put(0xA0, [neg, max_y, rd::<u32>(cm, 0x14)]);
    put(0xB0, [neg, max_y, rd::<u32>(cm, 8)]);
    let l = lines();
    let a = fabs(x(rd::<f32>(l, 0x84)));
    let ratio = a.fdiv(fabs(x(rd::<f32>(l, 0x24)).fadd_m(rd::<f32>(l, 4))).fmul_m(gf(0x85_8B8C)).fadd(a));
    let c = x(rd::<f32>(handling(), 0xAC)).fmul_m(gf(0x85_8B90));
    let one = gf(0x85_8624);
    let h = ratio
        .fdiv(c)
        .fsubr_m(one)
        .fmul_m(rd::<f32>(this, 0x878))
        .fadd(x(rd::<f32>(mi, 0x40)).fmul_m(gf(0x85_8B8C)).fsub_m(rd::<f32>(l, 8)));
    wr(this, 0x898, h.to_f32());
    wr(this, 0x89C, h.to_f32());
    let s = x(rd::<f32>(l, 0x88));
    let f = x(gf(0x8D_3464));
    let q = x(one).fsub(ratio).fdiv(f.fadd(f)).fsubr_m(one);
    let end = s.fsub(s.fadd_m(rd::<f32>(this, 0x898)).fdiv(q));
    wr(l, 0x98, end.to_f32());
    let l = lines();
    wr(l, 0xB8, rd::<u32>(l, 0x98));
    for i in 0..4isize {
        wr(this, 0x838 + 4 * i, x(rd::<f32>(mi, 0x40)).fmul_m(gf(0x85_8B8C)).fsub_m(rd::<f32>(this, 0x898)).to_f32());
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_TRAILER_SETUP_SUSPENSION_LINES];
