//! `RpHAnimHierarchyUpdateMatrices` and `RpHAnimKeyFrameApply` (STD-6, batch `std6-rw-1`). Ported from
//! the 1.0 US bytes (0x7C51D0, 0x7C5B80).
//!
//! Hierarchy flags: 1 sub-hierarchy, 2 no matrices, 0x1000 update modelling matrices, 0x2000 update
//! LTMs, 0x4000 local-space matrices. Node flags (`node + 8`): 1 pop, 2 push. Interpolated key frames
//! (`interp + 0x4C + i * size`): +0 / +4 the frame pair, +8 quaternion (x, y, z, w), +0x18 translation.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, g, p8, rd, wr};

orig! {
    /// `RwFrameDirty(frame)` (0x7F0340), `RwFrameGetLTM(frame)` (0x7F0990), `RwFrameUpdateObjects(frame)`
    /// (0x7F0910)
    fn frame_dirty(f: GPtr<u8>) -> u32 = cdecl 0x7F0340 => crate::frame::dirty;
    fn frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => crate::frame::get_ltm;
    fn frame_update_objects(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => crate::frame::update_objects;
    /// `RwMatrixMultiply(out, a, b)` (0x7F18B0)
    fn matrix_multiply(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18B0 => crate::matrix::rw_matrix_multiply;
    /// A key-frame apply callback `cb(matrix, keyFrame)`
    fn call_apply_cb(f: u32, m: GPtr<u8>, kf: GPtr<u8>) = cdecl dyn => unported;
}

/// `RpHAnimKeyFrameApply`'s address: `UpdateMatrices` inlines it when the interpolator uses it.
const KEY_FRAME_APPLY: u32 = 0x7C5B80;

/// The rotation matrix of the quaternion at `q` (x, y, z, w) with the translation at `q + 0x10`, flags
/// 3, in the original's x87 order (squares and cross products stored as floats, `w*y` / `w*z` kept on
/// the stack). The 0x10 / 0x1C / 0x2C pads are not written.
fn quat_matrix(m: GPtr<u8>, q: GPtr<u8>) {
    let c = |o: isize| X87::from_f32(rd::<f32>(q, o));
    let (qx, qy, qz, qw) = (c(0), c(4), c(8), c(0xC));
    let st = |v: X87| x(f32_of(v));
    let xx = st(qx.fmul(qx));
    let yy = st(qy.fmul(qy));
    let zz = st(qz.fmul(qz));
    let yz = st(qz.fmul(qy));
    let xz = st(qz.fmul(qx));
    let xy = st(qy.fmul(qx));
    let wx = st(qw.fmul(qx));
    let wy = qw.fmul(qy);
    let wz = qw.fmul(qz);
    let one = x(1.0);
    let two = |v: X87| v.fadd(v);
    wr::<f32>(m, 0, f32_of(one.fsub(two(zz.fadd(yy)))));
    wr::<f32>(m, 4, f32_of(two(xy.fadd(wz))));
    wr::<f32>(m, 8, f32_of(two(xz.fsub(wy))));
    wr::<f32>(m, 0x10, f32_of(two(xy.fsub(wz))));
    wr::<f32>(m, 0x14, f32_of(one.fsub(two(zz.fadd(xx)))));
    wr::<f32>(m, 0x18, f32_of(two(wx.fadd(yz))));
    wr::<f32>(m, 0x20, f32_of(two(wy.fadd(xz))));
    wr::<f32>(m, 0x24, f32_of(two(yz.fsub(wx))));
    wr::<f32>(m, 0x28, f32_of(one.fsub(two(yy.fadd(xx)))));
    wr::<u32>(m, 0xC, 3);
    for k in 0..3 {
        wr::<u32>(m, 0x30 + k * 4, rd::<u32>(q, 0x10 + k * 4));
    }
}

fn x(v: f32) -> X87 {
    X87::from_f32(v)
}

/// `RpHAnimKeyFrameApply(matrix, keyFrame)` (0x7C5B80): the matrix of the key frame's quaternion (+8)
/// and translation (+0x18), flags 3.
#[hook(addr = 0x7C5B80, conv = cdecl, name = "RpHAnimKeyFrameApply", expect = "83 EC 28 8B 4C 24 30 D9 41 08",
       unverified = "std6-rw-1", route = "L2", hazards = "none")]
pub fn rp_hanim_key_frame_apply(m: GPtr<u8>, kf: GPtr<u8>) {
    quat_matrix(m, kf.byte_add(8));
}

/// The node's local matrix from its interpolated key frame (inline `RpHAnimKeyFrameApply`, else the
/// interpolator's apply callback).
fn apply(cb: u32, m: GPtr<u8>, kf: GPtr<u8>) {
    if cb == KEY_FRAME_APPLY {
        quat_matrix(m, kf.byte_add(8));
    } else {
        call_apply_cb(cb, m, kf);
    }
}

fn identity(m: GPtr<u8>) {
    for (o, v) in [(0x28isize, 1.0f32), (0x14, 1.0), (0, 1.0)] {
        wr::<f32>(m, o, v);
    }
    for o in [0x10isize, 8, 4, 0x24, 0x20, 0x18, 0x38, 0x34, 0x30] {
        wr::<u32>(m, o, 0);
    }
    wr::<u32>(m, 0xC, rd::<u32>(m, 0xC) | 0x20003);
}

fn copy64(dst: GPtr<u8>, src: GPtr<u8>) {
    for k in 0..16 {
        wr::<u32>(dst, k * 4, rd::<u32>(src, k * 4));
    }
}

/// The parent frame's world matrix: identity without a parent; the cached LTM when the parent's root
/// is not dirty; else the product of the modelling matrices up the chain.
fn parent_matrix(out: GPtr<u8>, frame: GPtr<u8>) {
    let mut f = p8(rd::<u32>(frame, 4));
    if f.is_null() {
        identity(out);
        return;
    }
    if frame_dirty(f) == 0 {
        copy64(out, frame_get_ltm(f));
        return;
    }
    copy64(out, f.byte_add(0x10));
    let mut tmp = [0u8; 0x40];
    let tp = GPtr::from_raw(tmp.as_mut_ptr());
    f = p8(rd::<u32>(f, 4));
    while !f.is_null() {
        copy64(tp, out);
        matrix_multiply(out, tp, f.byte_add(0x10));
        f = p8(rd::<u32>(f, 4));
    }
}

/// Puts the root of `frame` on the RW dirty-frame list unless it is there already (the original's
/// inline `RwFrameUpdateObjects` prologue).
fn mark_root_dirty(frame: GPtr<u8>) {
    let root = p8(rd::<u32>(frame, 0xA0));
    if rd::<u8>(root, 3) & 3 != 0 {
        return;
    }
    let gl = p8(g::<u32>(0xC97B24).read());
    wr::<u32>(root, 8, rd::<u32>(gl, 0xBC));
    wr::<u32>(root, 0xC, gl.addr() as u32 + 0xBC);
    wr::<u32>(p8(rd::<u32>(gl, 0xBC)), 4, root.addr() as u32 + 8);
    wr::<u32>(gl, 0xBC, root.addr() as u32 + 8);
    wr::<u8>(root, 3, rd::<u8>(root, 3) | 2);
}

/// `RpHAnimHierarchyUpdateMatrices(hierarchy)` (0x7C51D0). With matrices (flag 2 clear): the parent
/// matrix is the parent hierarchy's matrix of this sub-hierarchy's root (flag 1), an identity for local
/// space (0x4000), or the parent frame's world matrix; each node's matrix (+8 array) = its key-frame
/// matrix x the parent matrix, the push / pop node flags keeping a stack of parent pointers; with
/// 0x1000 the key-frame matrix becomes the node frame's modelling matrix (objects updated unless
/// 0x2000), with 0x2000 the frame's LTM is set (times the parent frame's world matrix in local space)
/// and the frame marked; 0x2000 also marks the root frame dirty first. Without matrices (flag 2) the
/// same walk keeps the parent matrices by value and only updates frames. Returns 1.
#[hook(addr = 0x7C51D0, conv = cdecl, name = "RpHAnimHierarchyUpdateMatrices", expect = "81 EC 14 0A 00 00 53 55",
       unverified = "std6-rw-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn rp_hanim_hierarchy_update_matrices(h: GPtr<u8>) -> u32 {
    let flags = rd::<u32>(h, 0);
    let mut world = [0u8; 0x40];
    let wp = GPtr::from_raw(world.as_mut_ptr());
    let mut local = [0u8; 0x40];
    let lp = GPtr::from_raw(local.as_mut_ptr());
    let n = rd::<i32>(h, 4);
    let interp = p8(rd::<u32>(h, 0x20));
    let kfs = rd::<u32>(interp, 0x24);
    let cb = rd::<u32>(interp, 0x3C);
    let mut kf = interp.byte_add(0x4C);
    let update_ltms = flags & 0x2000 != 0;
    if flags & 2 != 0 {
        parent_matrix(wp, p8(rd::<u32>(h, 0x14)));
        if update_ltms {
            mark_root_dirty(p8(rd::<u32>(h, 0x14)));
        }
        let mut stack = vec_stack::<0x800>();
        let mut sp = 0usize;
        let mut prod = [0u8; 0x40];
        let pp = GPtr::from_raw(prod.as_mut_ptr());
        for i in 0..n.max(0) {
            apply(cb, lp, kf);
            matrix_multiply(pp, lp, wp);
            let node = p8(rd::<u32>(h, 0x10)).byte_add(i as isize * 0x10);
            let frame = p8(rd::<u32>(node, 0xC));
            if !frame.is_null() {
                let mut set_ltm = update_ltms;
                if flags & 0x1000 != 0 {
                    copy64(frame.byte_add(0x10), lp);
                    if !update_ltms {
                        frame_update_objects(frame);
                    }
                    set_ltm = update_ltms;
                }
                if set_ltm {
                    copy64(frame.byte_add(0x50), pp);
                    wr::<u8>(frame, 3, (rd::<u8>(frame, 3) & 0xFB) | 8);
                }
            }
            match rd::<u32>(node, 8) & 3 {
                0 => copy64(wp, pp),
                1 => {
                    sp = sp.saturating_sub(0x40);
                    copy64(wp, GPtr::from_raw(stack[sp..].as_mut_ptr()));
                }
                2 => {
                    copy64(GPtr::from_raw(stack[sp..].as_mut_ptr()), wp);
                    sp = (sp + 0x40).min(stack.len() - 0x40);
                    copy64(wp, pp);
                }
                _ => {}
            }
            kf = kf.byte_add(kfs as isize);
        }
        return 1;
    }
    let mut ident = [0u8; 0x40];
    let ip = GPtr::from_raw(ident.as_mut_ptr());
    let mut parent: GPtr<u8>;
    let mut frame_for_world = GPtr::<u8>::NULL;
    let mut world_needed = false;
    if flags & 1 != 0 && rd::<i32>(h, 0x1C) != -1 {
        let ph = p8(rd::<u32>(h, 0x18));
        parent = p8(rd::<u32>(ph, 8)).byte_add((rd::<i32>(h, 0x1C) as isize) << 6);
        if flags & 0x2000 != 0 && flags & 0x4000 != 0 {
            frame_for_world = p8(rd::<u32>(ph, 0x14));
            world_needed = true;
        }
    } else if flags & 0x4000 != 0 {
        identity(ip);
        parent = ip;
        if flags & 0x2000 != 0 {
            frame_for_world = p8(rd::<u32>(h, 0x14));
            world_needed = true;
        }
    } else {
        parent = wp;
        frame_for_world = p8(rd::<u32>(h, 0x14));
        world_needed = true;
    }
    if world_needed {
        if frame_for_world.is_null() {
            identity(wp);
        } else {
            parent_matrix(wp, frame_for_world);
        }
    }
    if update_ltms {
        mark_root_dirty(p8(rd::<u32>(h, 0x14)));
    }
    let mut out = p8(rd::<u32>(h, 8));
    let mut stack = [0u32; 0x200];
    let mut sp = 0usize;
    for i in 0..n.max(0) {
        apply(cb, lp, kf);
        matrix_multiply(out, lp, parent);
        let node = p8(rd::<u32>(h, 0x10)).byte_add(i as isize * 0x10);
        let frame = p8(rd::<u32>(node, 0xC));
        if !frame.is_null() {
            if flags & 0x1000 != 0 {
                copy64(frame.byte_add(0x10), lp);
                if !update_ltms {
                    frame_update_objects(frame);
                }
            }
            if update_ltms {
                if flags & 0x4000 != 0 {
                    matrix_multiply(frame.byte_add(0x50), out, wp);
                } else {
                    copy64(frame.byte_add(0x50), out);
                }
                wr::<u8>(frame, 3, (rd::<u8>(frame, 3) & 0xFB) | 8);
            }
        }
        match rd::<u32>(node, 8) & 3 {
            0 => parent = out,
            1 => {
                sp = sp.saturating_sub(1);
                parent = p8(stack[sp]);
            }
            2 => {
                stack[sp] = parent.addr() as u32;
                sp = (sp + 1).min(stack.len() - 1);
                parent = out;
            }
            _ => {}
        }
        kf = kf.byte_add(kfs as isize);
        out = out.byte_add(0x40);
    }
    1
}

/// A zeroed byte stack (the original's 0x7B0-byte local area for the parent matrices).
fn vec_stack<const N: usize>() -> [u8; N] {
    [0u8; N]
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_RP_HANIM_KEY_FRAME_APPLY, HOOK_RP_HANIM_HIERARCHY_UPDATE_MATRICES];
