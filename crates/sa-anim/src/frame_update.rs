//! STD-7: the frame update callbacks of `RpAnimBlendClumpUpdateAnimations` (batch `std7-anim-1`) and
//! `CQuaternion::Normalise`. Ported from the 1.0 US bytes with the original's x87 evaluation order (sums kept
//! in registers, f32 at every `fstp dword`); unverified (docs/verify/HOWTO.md).
//!
//! `AnimBlendFrameData` (0x18): +0 flags (bit 1 ignore orientation, bit 2 ignore translation, bit 3 has
//! velocity, bit 4 has z velocity), +4 bone position, +0x10 the key frame (`q` at +0, `t` at +0x10). The
//! update data: +0 "has partial" word, +4.. the null-terminated node pointer list (each pointer advanced by
//! one node, 0x18 bytes, per frame).
#![allow(clippy::unreadable_literal, clippy::needless_range_loop)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::*;

/// `gpAnimBlendClump`
const ANIM_BLEND_CLUMP: usize = 0xB4EA0C;

orig! {
    /// `CAnimBlendNode::Update(CVector& t, CQuaternion& q, float weight)` (0x4D06C0), AL
    fn node_update(n: GPtr<u8>, t: GPtr<u8>, q: GPtr<u8>, w: f32) -> u8 = thiscall 0x4D06C0 => unported;
    /// `CAnimBlendNode::GetCurrentTranslation(CVector&, float)` (0x4CFC50), `GetEndTranslation` (0x4CFD90)
    fn node_get_current_translation(n: GPtr<u8>, t: GPtr<u8>, w: f32) = thiscall 0x4CFC50 => unported;
    fn node_get_end_translation(n: GPtr<u8>, t: GPtr<u8>, w: f32) = thiscall 0x4CFD90 => unported;
    /// `CQuaternion::Normalise()` (0x4D1610)
    fn quat_normalise(q: GPtr<u8>) = thiscall 0x4D1610 => quaternion_normalise;
    /// `FrameUpdateCallBackSkinnedWithVelocityExtraction(c, fd)` (0x4D1680, `c` in EAX) and the 3D variant (0x4D1A50)
    fn skinned_with_velocity(c: GPtr<u8>, fd: GPtr<u8>) = reg("args = eax stack") 0x4D1680 => frame_update_skinned_with_velocity;
    fn skinned_with_3d_velocity(c: GPtr<u8>, fd: GPtr<u8>) = reg("args = eax stack") 0x4D1A50 => unported;
}

fn zero() -> X87 {
    x(g::<f32>(0x858B50).read())
}

fn one() -> f32 {
    g::<f32>(0x858624).read()
}

/// `CQuaternion::Normalise()` (0x4D1610, thiscall): `s = ((x² + y²) + z²) + w²` (x87); `s == 0` → `w = 1`;
/// else each component `*= 1 / sqrt(s)` (the reciprocal kept in ST0).
#[hook(addr = 0x4D1610, conv = thiscall, name = "CQuaternion::Normalise", expect = "D9 41 0C D9 41 08 D9 41 04",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn quaternion_normalise(q: GPtr<u8>) {
    let c = |o: isize| x(rd::<f32>(q, o));
    let s = c(0).fmul(c(0)).fadd(c(4).fmul(c(4))).fadd(c(8).fmul(c(8))).fadd(c(0xC).fmul(c(0xC)));
    if !s.fcom(zero()).test_parity(0x44) {
        wr::<u32>(q, 0xC, 0x3F80_0000);
        return;
    }
    let inv = s.fsqrt().fdivr_m(one());
    for o in [0isize, 4, 8, 0xC] {
        wr::<f32>(q, o, inv.fmul_m(rd::<f32>(q, o)).to_f32());
    }
}

/// The velocity-extraction dispatch shared by the callbacks: frame flag bit 3 and the clump's ped position
/// (`gpAnimBlendClump` +0xC) set.
fn wants_velocity(fd: GPtr<u8>) -> bool {
    rd::<u8>(fd, 0) & 8 != 0 && rd::<u32>(g::<GPtr<u8>>(ANIM_BLEND_CLUMP).read(), 0xC) != 0
}

/// `FrameUpdateCallBackSkinned(AnimBlendFrameData* fd, void* data)` (0x4D2B90, cdecl). With velocity
/// (`wants_velocity`): the 3D (flag bit 4) or planar velocity-extraction variant. Else: the partial blend sum
/// (when the update data's word is set: the blend amounts of the nodes with a sequence whose association has
/// the partial flag, +0x2E bit 4); each node with a sequence: `Update(t, q, f32(1 - partial))`, then, when its
/// sequence has a translation (+4 bit 1), `T += t` and `blendT += association blend`; `Q += q` or `Q -= q` by
/// the sign of the dot product `((w·W + z·Z) + y·Y) + x·X` (x87; NaN adds); each node pointer advances by one
/// node (the first entry is not null-checked). Unless flag bit 1: `Q` normalised into the key frame's `q`.
/// Unless flag bit 2: key frame `t = T·blendT + (1 - blendT)·bonePos` (the products stored as f32, the sum
/// with the x87 `1 - blendT`).
#[hook(addr = 0x4D2B90, conv = cdecl, name = "RpAnimBlend::FrameUpdateCallBackSkinned", expect = "83 EC 3C 55 8B 6C 24 44",
       unverified = "std7-anim-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn frame_update_skinned(fd: GPtr<u8>, data: GPtr<u8>) {
    if wants_velocity(fd) {
        if rd::<u8>(fd, 0) & 0x10 != 0 {
            skinned_with_3d_velocity(data, fd);
        } else {
            skinned_with_velocity(data, fd);
        }
        return;
    }
    let kf = rd::<GPtr<u8>>(fd, 0x10);
    let mut tsum = [0f32; 3];
    let mut qsum = [0f32; 4];
    let mut blend_t = 0f32;
    let mut partial = 0f32;
    if rd::<u32>(data, 0) != 0 {
        let mut p = data.byte_add(4);
        let mut n = rd::<GPtr<u8>>(p, 0);
        while !n.is_null() {
            if rd::<u32>(n, 0x10) != 0 {
                let a = rd::<GPtr<u8>>(n, 0x14);
                if (rd::<u8>(a, 0x2E) >> 4) & 1 != 0 {
                    partial = x(rd::<f32>(a, 0x18)).fadd_m(partial).to_f32();
                }
            }
            p = p.byte_add(4);
            n = rd::<GPtr<u8>>(p, 0);
        }
    }
    let mut p = data.byte_add(4);
    loop {
        let node = rd::<GPtr<u8>>(p, 0);
        if rd::<u32>(node, 0x10) != 0 {
            let mut t = [0f32; 3];
            let mut q = [0f32; 4];
            let w = x(one()).fsub_m(partial).to_f32();
            node_update(node, GPtr::from_raw(t.as_mut_ptr()).cast(), GPtr::from_raw(q.as_mut_ptr()).cast(), w);
            let node = rd::<GPtr<u8>>(p, 0);
            let seq = rd::<GPtr<u8>>(node, 0x10);
            if (rd::<u8>(seq, 4) >> 1) & 1 != 0 {
                for i in 0..3 {
                    tsum[i] = x(t[i]).fadd_m(tsum[i]).to_f32();
                }
                blend_t = x(rd::<f32>(rd::<GPtr<u8>>(node, 0x14), 0x18)).fadd_m(blend_t).to_f32();
            }
            let dot = x(q[3]).fmul_m(qsum[3]).fadd(x(q[2]).fmul_m(qsum[2])).fadd(x(q[1]).fmul_m(qsum[1])).fadd(x(q[0]).fmul_m(qsum[0]));
            if dot.fcom(zero()).test_parity(5) {
                for i in 0..4 {
                    qsum[i] = x(q[i]).fadd_m(qsum[i]).to_f32();
                }
            } else {
                for i in 0..4 {
                    qsum[i] = x(qsum[i]).fsub_m(q[i]).to_f32();
                }
            }
        }
        let node = rd::<GPtr<u8>>(p, 0);
        wr::<u32>(p, 0, node.addr() as u32 + 0x18);
        p = p.byte_add(4);
        if rd::<u32>(p, 0) == 0 {
            break;
        }
    }
    let flags = rd::<u8>(fd, 0);
    if flags & 2 == 0 {
        quat_normalise(GPtr::from_raw(qsum.as_mut_ptr()).cast());
        for (i, v) in qsum.iter().enumerate() {
            wr::<f32>(kf, i as isize * 4, *v);
        }
    }
    if rd::<u8>(fd, 0) & 4 == 0 {
        apply_translation(fd, kf, tsum, blend_t);
    }
}

/// Key frame `t = T·b + (1 - b)·bonePos` as the callbacks store it.
fn apply_translation(fd: GPtr<u8>, kf: GPtr<u8>, tsum: [f32; 3], b: f32) {
    for i in 0..3 {
        wr::<f32>(kf, 0x10 + 4 * i as isize, x(tsum[i]).fmul_m(b).to_f32());
    }
    let k = x(one()).fsub_m(b);
    for i in 0..3isize {
        let o = 0x10 + 4 * i;
        wr::<f32>(kf, o, k.fmul_m(rd::<f32>(fd, 4 + 4 * i)).fadd_m(rd::<f32>(kf, o)).to_f32());
    }
}

/// `FrameUpdateCallBackOffscreen(AnimBlendFrameData* fd, void* data)` (0x4D2E10, cdecl): only the
/// planar velocity extraction (`wants_velocity`), whatever flag bit 4 says.
#[hook(addr = 0x4D2E10, conv = cdecl, name = "FrameUpdateCallBackOffscreen", expect = "8B 44 24 04 F6 00 08",
       unverified = "std7-anim-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn frame_update_offscreen(fd: GPtr<u8>, data: GPtr<u8>) {
    if wants_velocity(fd) {
        skinned_with_velocity(data, fd);
    }
}

/// Sums `+=` the way the callbacks store them: `f32(a + b)` with `a` loaded first.
fn add(a: f32, b: f32) -> f32 {
    x(a).fadd_m(b).to_f32()
}

/// The partial blend sum of the velocity variants: like [`frame_update_skinned`]'s, but associations with
/// flag 0x400 (+0x2E word bit 10) are left out.
fn partial_blend_no_add(data: GPtr<u8>) -> f32 {
    let mut partial = 0f32;
    if rd::<u32>(data, 0) != 0 {
        let mut p = data.byte_add(4);
        let mut n = rd::<GPtr<u8>>(p, 0);
        loop {
            if rd::<u32>(n, 0x10) != 0 {
                let a = rd::<GPtr<u8>>(n, 0x14);
                let fl = rd::<u16>(a, 0x2E);
                if (fl as u8 >> 4) & 1 != 0 && (fl >> 10) & 1 == 0 {
                    partial = add(rd::<f32>(a, 0x18), partial);
                }
            }
            p = p.byte_add(4);
            n = rd::<GPtr<u8>>(p, 0);
            if n.is_null() {
                break;
            }
        }
    }
    partial
}

/// `FrameUpdateCallBackSkinnedWithVelocityExtraction(c, fd)` (0x4D1680, `c` in EAX, `fd` on the stack, cdecl).
/// Partial sum without the 0x400 associations. Pass 1 (pointers not advanced): nodes with a sequence that has a
/// translation, whose association neither ignores the root translation (+0x2E bit 13) nor lacks velocity
/// extraction (bit 6): `GetCurrentTranslation(t, f32(1 - partial))`, `curr.y += t.y`, with bit 7 `curr.x +=
/// t.x`. Pass 2 (pointers advanced): `looped = Update(t, q, w)`; the quaternion sum as in the plain callback;
/// with a translation and no root-ignore: `T += t`; with velocity extraction: `delta.y += t.y` (bit 7: `delta.x
/// += t.x`), `anyLooped |= looped`, and when this node looped `GetEndTranslation(t, w)` into `loop`. Then unless
/// flag bit 1 the normalised quaternion; unless bit 2: the ped position (+0xC of `gpAnimBlendClump`) =
/// `delta - curr` (+ `loop` when any looped), key frame `t = (T.x - delta.x, T.y - delta.y, T.z)`, z adjusted
/// for z >= -0.8 (`+ bone.z`, or below -0.4 `+ (z·2.5 + 2)·bone.z`), then `x += bone.x`, `y += bone.y`.
#[hook(addr = 0x4D1680, conv = reg("args = eax stack"), name = "RpAnimBlend::FrameUpdateCallBackSkinnedWithVelocityExtraction", expect = "83 EC 68 55 56 57 8B F8",
       unverified = "std7-anim-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn frame_update_skinned_with_velocity(data: GPtr<u8>, fd: GPtr<u8>) {
    let kf = rd::<GPtr<u8>>(fd, 0x10);
    let partial = partial_blend_no_add(data);
    let mut curr = [0f32; 2];
    let mut delta = [0f32; 2];
    let mut lp = [0f32; 2];
    let mut tsum = [0f32; 3];
    let mut qsum = [0f32; 4];
    let mut any_looped = false;
    let mut t = [0f32; 3];
    let tp = GPtr::from_raw(t.as_mut_ptr()).cast::<u8>();
    let mut p = data.byte_add(4);
    loop {
        let node = rd::<GPtr<u8>>(p, 0);
        let seq = rd::<GPtr<u8>>(node, 0x10);
        if !seq.is_null() && (rd::<u8>(seq, 4) >> 1) & 1 != 0 {
            let a = rd::<GPtr<u8>>(node, 0x14);
            let fl = rd::<u16>(a, 0x2E);
            if (fl >> 13) & 1 == 0 && (fl as u8 >> 6) & 1 != 0 {
                node_get_current_translation(node, tp, x(one()).fsub_m(partial).to_f32());
                curr[1] = add(curr[1], rd::<f32>(tp, 4));
                if rd::<u8>(rd::<GPtr<u8>>(rd::<GPtr<u8>>(p, 0), 0x14), 0x2E) >> 7 != 0 {
                    curr[0] = add(curr[0], rd::<f32>(tp, 0));
                }
            }
        }
        p = p.byte_add(4);
        if rd::<u32>(p, 0) == 0 {
            break;
        }
    }
    let mut p = data.byte_add(4);
    loop {
        let node = rd::<GPtr<u8>>(p, 0);
        if rd::<u32>(node, 0x10) != 0 {
            let w = x(one()).fsub_m(partial).to_f32();
            let mut q = [0f32; 4];
            let looped = node_update(node, tp, GPtr::from_raw(q.as_mut_ptr()).cast(), w);
            let dot = x(q[3]).fmul_m(qsum[3]).fadd(x(q[2]).fmul_m(qsum[2])).fadd(x(q[1]).fmul_m(qsum[1])).fadd(x(q[0]).fmul_m(qsum[0]));
            if dot.fcom(zero()).test_parity(5) {
                for i in 0..4 {
                    qsum[i] = add(q[i], qsum[i]);
                }
            } else {
                for i in 0..4 {
                    qsum[i] = x(qsum[i]).fsub_m(q[i]).to_f32();
                }
            }
            let node = rd::<GPtr<u8>>(p, 0);
            let seq = rd::<GPtr<u8>>(node, 0x10);
            let fl = rd::<u16>(rd::<GPtr<u8>>(node, 0x14), 0x2E);
            if (rd::<u8>(seq, 4) >> 1) & 1 != 0 && (fl >> 13) & 1 == 0 {
                tsum[0] = add(tsum[0], rd::<f32>(tp, 0));
                tsum[1] = add(tsum[1], rd::<f32>(tp, 4));
                tsum[2] = add(rd::<f32>(tp, 8), tsum[2]);
                if (fl as u8 >> 6) & 1 != 0 {
                    delta[1] = add(delta[1], rd::<f32>(tp, 4));
                    if (fl as u8 >> 7) & 1 != 0 {
                        delta[0] = add(delta[0], rd::<f32>(tp, 0));
                    }
                    any_looped |= looped != 0;
                    if looped != 0 {
                        node_get_end_translation(node, tp, w);
                        lp[1] = add(lp[1], rd::<f32>(tp, 4));
                        if rd::<u8>(rd::<GPtr<u8>>(rd::<GPtr<u8>>(p, 0), 0x14), 0x2E) >> 7 != 0 {
                            lp[0] = add(lp[0], rd::<f32>(tp, 0));
                        }
                    }
                }
            }
        }
        let node = rd::<GPtr<u8>>(p, 0);
        wr::<u32>(p, 0, node.addr() as u32 + 0x18);
        p = p.byte_add(4);
        if rd::<u32>(p, 0) == 0 {
            break;
        }
    }
    if rd::<u8>(fd, 0) & 2 == 0 {
        quat_normalise(GPtr::from_raw(qsum.as_mut_ptr()).cast());
        for (i, v) in qsum.iter().enumerate() {
            wr::<f32>(kf, i as isize * 4, *v);
        }
    }
    if rd::<u8>(fd, 0) & 4 != 0 {
        return;
    }
    let clump = g::<GPtr<u8>>(ANIM_BLEND_CLUMP).read();
    let ped = rd::<GPtr<u8>>(clump, 0xC);
    wr::<f32>(ped, 0, x(delta[0]).fsub_m(curr[0]).to_f32());
    let ped = rd::<GPtr<u8>>(clump, 0xC);
    wr::<f32>(ped, 4, x(delta[1]).fsub_m(curr[1]).to_f32());
    if any_looped {
        let ped = rd::<GPtr<u8>>(clump, 0xC);
        wr::<f32>(ped, 0, add(lp[0], rd::<f32>(ped, 0)));
        let ped = rd::<GPtr<u8>>(clump, 0xC);
        wr::<f32>(ped, 4, add(lp[1], rd::<f32>(ped, 4)));
    }
    wr::<f32>(kf, 0x10, x(tsum[0]).fsub_m(delta[0]).to_f32());
    wr::<f32>(kf, 0x14, x(tsum[1]).fsub_m(delta[1]).to_f32());
    wr::<f32>(kf, 0x18, tsum[2]);
    let z = x(rd::<f32>(kf, 0x18));
    if z.fcom(x(g::<f32>(0x858C9C).read())).test_parity(5) {
        let bz = rd::<f32>(fd, 0xC);
        let v = if z.fcom(x(g::<f32>(0x858F0C).read())).test_parity(5) {
            x(bz)
        } else {
            z.fmul_m(g::<f32>(0x858FA0).read()).fadd_m(g::<f32>(0x858CA0).read()).fmul_m(bz)
        };
        wr::<f32>(kf, 0x18, v.fadd_m(rd::<f32>(kf, 0x18)).to_f32());
    }
    wr::<f32>(kf, 0x10, add(rd::<f32>(fd, 4), rd::<f32>(kf, 0x10)));
    wr::<f32>(kf, 0x14, add(rd::<f32>(fd, 8), rd::<f32>(kf, 0x14)));
}

pub const HOOKS: &[HookReg] = &[
    HOOK_QUATERNION_NORMALISE,
    HOOK_FRAME_UPDATE_SKINNED,
    HOOK_FRAME_UPDATE_OFFSCREEN,
    HOOK_FRAME_UPDATE_SKINNED_WITH_VELOCITY,
];
