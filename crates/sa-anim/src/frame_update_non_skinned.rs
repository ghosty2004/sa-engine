//! `RpAnimBlend::FrameUpdateCallBackNonSkinned` (batch `samp-rw-2`; catalogue subsystem RW), ported from the 1.0 US
//! bytes with the original's x87 evaluation order (f32 at every `fstp dword`). The layouts are those of
//! `frame_update` (STD-7). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::*;

/// `gpAnimBlendClump`
const ANIM_BLEND_CLUMP: usize = 0xB4EA0C;

orig! {
    /// `CAnimBlendNode::Update(CVector& t, CQuaternion& q, float weight)` (0x4D06C0), AL
    fn node_update(n: GPtr<u8>, t: GPtr<u8>, q: GPtr<u8>, w: f32) -> u8 = thiscall 0x4D06C0 => unported;
    /// `CQuaternion::Normalise()` (0x4D1610)
    fn quat_normalise(q: GPtr<u8>) = thiscall 0x4D1610 => crate::frame_update::quaternion_normalise;
    /// `CQuaternion::Get(RwMatrix*)` (0x59C080, thiscall, `ret 4`)
    fn quat_get(q: GPtr<u8>, m: GPtr<u8>) = thiscall 0x59C080 => unported;
    /// `RwMatrixUpdate(RwMatrix*)` (0x7F18A0)
    fn rw_matrix_update(m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18A0 => unported;
    /// `FrameUpdateCallBackNonSkinnedWithVelocityExtraction(data, fd)` (0x4D2100, `data` in EAX) and the 3D variant
    /// (0x4D2450)
    fn non_skinned_with_velocity(data: GPtr<u8>, fd: GPtr<u8>) = reg("args = eax stack") 0x4D2100 => unported;
    /// `CAnimBlendNode::UpdateCompressed(CVector&, CQuaternion&, float)` (0x4D08D0), AL
    fn node_update_compressed(n: GPtr<u8>, t: GPtr<u8>, q: GPtr<u8>, w: f32) -> u8 = thiscall 0x4D08D0 => unported;
    /// `FrameUpdateCallBackWithVelocityExtractionCompressedNonSkinned(data, fd)` (0x4D27F0, `data` in EAX)
    fn compressed_non_skinned_with_velocity(data: GPtr<u8>, fd: GPtr<u8>) = reg("args = eax stack") 0x4D27F0 => unported;
    fn non_skinned_with_3d_velocity(data: GPtr<u8>, fd: GPtr<u8>) = reg("args = eax stack") 0x4D2450 => frame_update_non_skinned_with_3d_velocity;
    /// `CAnimBlendNode::GetCurrentTranslation(CVector&, float)` (0x4CFC50), `GetEndTranslation` (0x4CFD90)
    fn node_get_current_translation(n: GPtr<u8>, t: GPtr<u8>, w: f32) = thiscall 0x4CFC50 => unported;
    fn node_get_end_translation(n: GPtr<u8>, t: GPtr<u8>, w: f32) = thiscall 0x4CFD90 => unported;
}

fn one() -> f32 {
    g::<f32>(0x858624).read()
}

/// `FrameUpdateCallBackNonSkinned(AnimBlendFrameData* fd, void* data)` (0x4D30A0, cdecl). With frame flag bit 3
/// and the clump's ped position (`gpAnimBlendClump` +0xC): the 3D (flag bit 4, 0x4D2450) or planar (0x4D2100)
/// velocity-extraction variant (`data` in EAX). Otherwise, with the frame's matrix `m` = `fd->frame + 0x10`: when
/// the update data's dword is set and the first node is not null, the partial blend sum (the blend amounts of the
/// nodes with a sequence whose association has the partial flag, +0x2E bit 4); when the first node is not null,
/// each node with a sequence: `Update(t, q, f32(1 - partial))`; with a translation in the sequence (+4 bit 1) `T +=
/// t` and `blendT += association blend`; `Q += q` (no sign test, unlike the skinned callback); each node pointer
/// advances by one node. Unless flag bit 1: `m` flags |= 0x20003, `m` := identity rotation with a zero position,
/// `Q.Normalise()`, `Q.Get(m)`. Unless flag bit 2: `m.pos = T + (1 - blendT) * bonePos` (T not scaled by blendT,
/// unlike the skinned callback). Always `RwMatrixUpdate(m)`.
#[hook(addr = 0x4D30A0, conv = cdecl, name = "RpAnimBlend::FrameUpdateCallBackNonSkinned", expect = "83 EC 3C 53 8B 5C 24 44",
       unverified = "samp-rw-2", route = "L2", hazards = "heap-writes", recs = "anim-input",
       note = "writes the frame's matrix; the original keeps the partial sum in its fd argument slot")]
pub fn frame_update_non_skinned(fd: GPtr<u8>, data: GPtr<u8>) {
    let flags = rd::<u8>(fd, 0);
    if flags & 8 != 0 && rd::<u32>(g::<GPtr<u8>>(ANIM_BLEND_CLUMP).read(), 0xC) != 0 {
        if flags & 0x10 != 0 {
            non_skinned_with_3d_velocity(data, fd);
        } else {
            non_skinned_with_velocity(data, fd);
        }
        return;
    }
    non_skinned_body(fd, data, false);
}

/// `FrameUpdateCallBackCompressedNonSkinned(AnimBlendFrameData* fd, void* data)` (0x4D32D0, cdecl): with frame flag
/// bit 3 and the clump's ped position, the compressed velocity-extraction variant (0x4D27F0, `data` in EAX; flag bit
/// 4 not tested); otherwise the body of [`frame_update_non_skinned`] with `UpdateCompressed` (0x4D08D0).
#[hook(addr = 0x4D32D0, conv = cdecl, name = "RpAnimBlend::FrameUpdateCallBackCompressedNonSkinned", expect = "83 EC 3C 53 8B 5C 24 44 8A 03",
       unverified = "intro2-1", route = "L2", hazards = "heap-writes", recs = "intro",
       note = "writes the frame's matrix (the intro's objects)")]
pub fn frame_update_compressed_non_skinned(fd: GPtr<u8>, data: GPtr<u8>) {
    if rd::<u8>(fd, 0) & 8 != 0 && rd::<u32>(g::<GPtr<u8>>(ANIM_BLEND_CLUMP).read(), 0xC) != 0 {
        compressed_non_skinned_with_velocity(data, fd);
        return;
    }
    non_skinned_body(fd, data, true);
}

/// The shared body of 0x4D30A0 and 0x4D32D0 (`Update` / `UpdateCompressed`).
fn non_skinned_body(fd: GPtr<u8>, data: GPtr<u8>, compressed: bool) {
    let m = rd::<GPtr<u8>>(fd, 0x10).byte_add(0x10);
    let mut tsum = [0f32; 3];
    let mut qsum = [0f32; 4];
    let mut blend_t = 0f32;
    let mut partial = 0f32;
    if rd::<u32>(data, 0) != 0 && rd::<u32>(data, 4) != 0 {
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
    if rd::<u32>(p, 0) != 0 {
        loop {
            let node = rd::<GPtr<u8>>(p, 0);
            if rd::<u32>(node, 0x10) != 0 {
                let mut t = [0f32; 3];
                let mut q = [0f32; 4];
                let w = x(one()).fsub_m(partial).to_f32();
                let (tp, qp) = (GPtr::from_raw(t.as_mut_ptr()).cast(), GPtr::from_raw(q.as_mut_ptr()).cast());
                if compressed {
                    node_update_compressed(node, tp, qp, w);
                } else {
                    node_update(node, tp, qp, w);
                }
                let node = rd::<GPtr<u8>>(p, 0);
                let seq = rd::<GPtr<u8>>(node, 0x10);
                if (rd::<u8>(seq, 4) >> 1) & 1 != 0 {
                    for (s, v) in tsum.iter_mut().zip(t) {
                        *s = x(v).fadd_m(*s).to_f32();
                    }
                    blend_t = x(rd::<f32>(rd::<GPtr<u8>>(node, 0x14), 0x18)).fadd_m(blend_t).to_f32();
                }
                for (s, v) in qsum.iter_mut().zip(q) {
                    *s = x(v).fadd_m(*s).to_f32();
                }
            }
            let node = rd::<GPtr<u8>>(p, 0);
            wr::<u32>(p, 0, node.addr() as u32 + 0x18);
            p = p.byte_add(4);
            if rd::<u32>(p, 0) == 0 {
                break;
            }
        }
    }
    if rd::<u8>(fd, 0) & 2 == 0 {
        wr::<u32>(m, 0xC, rd::<u32>(m, 0xC) | 0x20003);
        for o in [0x28, 0x14, 0] {
            wr::<u32>(m, o, 0x3F80_0000);
        }
        for o in [0x10, 8, 4, 0x24, 0x20, 0x18, 0x38, 0x34, 0x30] {
            wr::<u32>(m, o, 0);
        }
        let qp = GPtr::from_raw(qsum.as_mut_ptr()).cast::<u8>();
        quat_normalise(qp);
        quat_get(qp, m);
    }
    if rd::<u8>(fd, 0) & 4 == 0 {
        let k = x(one()).fsub_m(blend_t);
        for (i, v) in tsum.into_iter().enumerate() {
            wr::<f32>(m, 0x30 + 4 * i as isize, v);
        }
        for i in 0..3isize {
            let o = 0x30 + 4 * i;
            wr::<f32>(m, o, k.fmul_m(rd::<f32>(fd, 4 + 4 * i)).fadd_m(rd::<f32>(m, o)).to_f32());
        }
    }
    rw_matrix_update(m);
}

/// `f32(a + b)` with `a` loaded first.
fn add(a: f32, b: f32) -> f32 {
    x(a).fadd_m(b).to_f32()
}

/// `FrameUpdateCallBackNonSkinnedWith3dVelocityExtraction(data, fd)` (0x4D2450, `data` in EAX, `fd` on the stack,
/// cdecl): the body of the skinned 3D variant (0x4D1A50) on the frame's matrix `m` = `fd->frame + 0x10`. Partial
/// sum: associations with the partial flag (+0x2E bit 4; the first node pointer not null-checked). Pass 1 (pointers
/// not advanced): nodes with a sequence with a translation whose association has velocity extraction (+0x2E bit 6):
/// `GetCurrentTranslation(t, f32(1 - partial))`, `curr += t`. Pass 2 (pointers advanced): `looped = Update(t, q,
/// w)`, `Q += q` (no sign test); with a translation `T += t`, and with velocity extraction `delta += t`, `anyLooped
/// |= looped`, a looped node's `GetEndTranslation(t, w)` into `loop`. Unless flag bit 1: `m` flags |= 0x20003, `m`
/// := identity with a zero position, `Q.Normalise()`, `Q.Get(m)`. Unless bit 2: the ped position (`gpAnimBlendClump`
/// +0xC) = `delta - curr` (+ `loop` when any looped), `m.pos = T - delta + bonePos`. Always `RwMatrixUpdate(m)`.
#[hook(addr = 0x4D2450, conv = reg("args = eax stack"), name = "RpAnimBlend::FrameUpdateCallBackNonSkinnedWith3dVelocityExtraction", expect = "83 EC 74 53 56 8B F0",
       unverified = "intro2-1", route = "L2", hazards = "heap-writes", recs = "intro",
       note = "the intro cutscene's objects; writes the frame's matrix")]
pub fn frame_update_non_skinned_with_3d_velocity(data: GPtr<u8>, fd: GPtr<u8>) {
    let m = rd::<GPtr<u8>>(fd, 0x10).byte_add(0x10);
    let mut partial = 0f32;
    if rd::<u32>(data, 0) != 0 {
        let mut p = data.byte_add(4);
        loop {
            let n = rd::<GPtr<u8>>(p, 0);
            if rd::<u32>(n, 0x10) != 0 {
                let a = rd::<GPtr<u8>>(n, 0x14);
                if (rd::<u8>(a, 0x2E) >> 4) & 1 != 0 {
                    partial = add(rd::<f32>(a, 0x18), partial);
                }
            }
            p = p.byte_add(4);
            if rd::<u32>(p, 0) == 0 {
                break;
            }
        }
    }
    let mut curr = [0f32; 3];
    let mut delta = [0f32; 3];
    let mut lp = [0f32; 3];
    let mut tsum = [0f32; 3];
    let mut qsum = [0f32; 4];
    let mut any_looped = false;
    let mut t = [0f32; 3];
    let tp = GPtr::from_raw(t.as_mut_ptr()).cast::<u8>();
    let mut p = data.byte_add(4);
    loop {
        let node = rd::<GPtr<u8>>(p, 0);
        let seq = rd::<GPtr<u8>>(node, 0x10);
        if !seq.is_null()
            && (rd::<u8>(seq, 4) >> 1) & 1 != 0
            && (rd::<u8>(rd::<GPtr<u8>>(node, 0x14), 0x2E) >> 6) & 1 != 0
        {
            node_get_current_translation(node, tp, x(one()).fsub_m(partial).to_f32());
            for i in 0..3 {
                curr[i] = add(t[i], curr[i]);
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
            for i in 0..4 {
                qsum[i] = add(q[i], qsum[i]);
            }
            let node = rd::<GPtr<u8>>(p, 0);
            if (rd::<u8>(rd::<GPtr<u8>>(node, 0x10), 4) >> 1) & 1 != 0 {
                for i in 0..3 {
                    tsum[i] = add(t[i], tsum[i]);
                }
                if (rd::<u8>(rd::<GPtr<u8>>(node, 0x14), 0x2E) >> 6) & 1 != 0 {
                    for i in 0..3 {
                        delta[i] = add(t[i], delta[i]);
                    }
                    any_looped |= looped != 0;
                    if looped != 0 {
                        node_get_end_translation(node, tp, w);
                        for i in 0..3 {
                            lp[i] = add(t[i], lp[i]);
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
        wr::<u32>(m, 0xC, rd::<u32>(m, 0xC) | 0x20003);
        wr::<u32>(m, 0x28, 0x3F80_0000);
        wr::<u32>(m, 0x14, 0x3F80_0000);
        wr::<u32>(m, 0, 0x3F80_0000);
        for o in [0x10isize, 8, 4, 0x24, 0x20, 0x18, 0x38, 0x34, 0x30] {
            wr::<u32>(m, o, 0);
        }
        let qp = GPtr::from_raw(qsum.as_mut_ptr()).cast::<u8>();
        quat_normalise(qp);
        quat_get(qp, m);
    }
    if rd::<u8>(fd, 0) & 4 == 0 {
        let clump = g::<GPtr<u8>>(ANIM_BLEND_CLUMP).read();
        let ped = rd::<GPtr<u8>>(clump, 0xC);
        for i in 0..3 {
            wr::<f32>(ped, 4 * i as isize, x(delta[i]).fsub_m(curr[i]).to_f32());
        }
        if any_looped {
            let ped = rd::<GPtr<u8>>(clump, 0xC);
            for i in 0..3 {
                let o = 4 * i as isize;
                wr::<f32>(ped, o, add(lp[i], rd::<f32>(ped, o)));
            }
        }
        for i in 0..3 {
            wr::<f32>(m, 0x30 + 4 * i as isize, x(tsum[i]).fsub_m(delta[i]).to_f32());
        }
        for i in 0..3 {
            let o = 0x30 + 4 * i as isize;
            wr::<f32>(m, o, add(rd::<f32>(fd, 4 + 4 * i as isize), rd::<f32>(m, o)));
        }
    }
    rw_matrix_update(m);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_FRAME_UPDATE_NON_SKINNED,
    HOOK_FRAME_UPDATE_NON_SKINNED_WITH_3D_VELOCITY,
    HOOK_FRAME_UPDATE_COMPRESSED_NON_SKINNED,
];
