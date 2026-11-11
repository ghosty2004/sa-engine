//! RpHAnim frame stream read and the RtAnim interpolator create / destroy (STD-6, batch `std6-rw-1`).
//! Ported from the 1.0 US bytes (0x7C4A00, 0x7CD520, 0x7CD590).
//!
//! RpHAnim frame extension (`frame + [0xC9B8C8]`): +0 id, +4 hierarchy. `RpHAnimHierarchy` (0x24, from
//! the freelist `[0xC9B8CC]`): +0 flags, +4 numNodes, +8 matrices (16-aligned), +0xC the unaligned
//! allocation, +0x10 node info (0x10 each: +0 id, +4 index, +8 flags, +0xC frame), +0x14 parent frame,
//! +0x18 parent hierarchy, +0x1C root parent offset, +0x20 current interpolator.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{p8, rd, rw_free, rw_malloc, rw_memory_alloc, wr};

orig! {
    /// `RwStreamReadInt32(stream, buf, size)` (0x7ED540)
    fn read_int32(s: GPtr<u8>, p: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x7ED540 => crate::stream::rw_stream_read_int32;
    /// `RtAnimInterpolatorCreate(numNodes, keyFrameSize)` (0x7CD520)
    fn interpolator_create_call(n: i32, kfs: i32) -> GPtr<u8> = cdecl 0x7CD520 => rt_anim_interpolator_create;
    /// A key-frame interpolate callback `cb(out, in1, in2, time, customData)`
    fn call_interpolate_cb(f: u32, out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, t: f32, custom: u32) = cdecl dyn => unported;
    /// `_rwerror(code)` (0x8088D0), `RwErrorSet(err)` (0x808820)
    fn rw_error1(code: u32) -> u32 = cdecl 0x8088D0 => unported;
    fn rw_error_set(e: GPtr<u32>) -> GPtr<u32> = cdecl 0x808820 => unported;
}

/// RtAnim interpolation schemes (0x30 each) and their count.
const SCHEMES: u32 = 0xC9B920;
const SCHEME_COUNT: usize = 0xC97900;

/// `RpHAnimHierarchy` freelist (`[0xC9B8CC]`).
const HANIM_FREELIST: usize = 0xC9B8CC;
/// `RwFrame` plugin offset of the RpHAnim extension (`[0xC9B8C8]`).
const HANIM_FRAME_OFFSET: usize = 0xC9B8C8;

/// `RtAnimInterpolatorCreate(numNodes, keyFrameSize)` (0x7CD520): `RwMalloc(numNodes * keyFrameSize +
/// 0x4C, 0x3001B)` (unchecked); +0x2C numNodes, +0x24 / +0x20 keyFrameSize, +0x14 -1.0f, +0x28 -1,
/// +0x38 itself, every other word of the 0x4C header 0.
#[hook(addr = 0x7CD520, conv = cdecl, name = "RtAnimInterpolatorCreate", expect = "56 8B 74 24 0C 57",
       unverified = "std6-rw-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn rt_anim_interpolator_create(num_nodes: i32, key_frame_size: i32) -> GPtr<u8> {
    let p = rw_malloc((num_nodes.wrapping_mul(key_frame_size) as u32).wrapping_add(0x4C), 0x3001B);
    wr::<i32>(p, 0x2C, num_nodes);
    wr::<i32>(p, 0x24, key_frame_size);
    wr::<i32>(p, 0x20, key_frame_size);
    for o in [0, 8, 4, 0xC] {
        wr::<u32>(p, o, 0);
    }
    wr::<u32>(p, 0x14, 0xBF80_0000);
    for o in [0x10, 0x18, 0x1C] {
        wr::<u32>(p, o, 0);
    }
    wr::<u32>(p, 0x28, 0xFFFF_FFFF);
    wr::<u32>(p, 0x30, 0);
    wr::<u32>(p, 0x34, 0);
    wr::<u32>(p, 0x38, p.addr() as u32);
    for o in [0x3C, 0x44, 0x40, 0x48] {
        wr::<u32>(p, o, 0);
    }
    p
}

/// `RtAnimInterpolatorDestroy(interp)` (0x7CD590): `RwFree(interp)` (tail jump).
#[hook(addr = 0x7CD590, conv = cdecl, name = "RtAnimInterpolatorDestroy", expect = "8B 0D 24 7B C9 00 FF A1 38 01 00 00",
       unverified = "std6-rw-1", route = "L2", hazards = "heap-writes")]
pub fn rt_anim_interpolator_destroy(interp: GPtr<u8>) {
    rw_free(interp);
}

/// `cb_HANIMPLUGIN_RwFrame_StreamRead(stream, length, frame, offset, size)` (0x7C4A00): version
/// (`RwStreamReadInt32`, must be 0x100, else NULL), the id into the extension, numNodes (read into the
/// original's `frame` argument slot; `<= 0` returns the stream with no hierarchy), flags, keyFrameSize
/// (each read failing returns NULL). The hierarchy comes from the freelist (hint 0x3011E) and is
/// zeroed (9 words, before the NULL check); NULL stores NULL and returns the stream. Otherwise +0x20
/// `RtAnimInterpolatorCreate(numNodes, keyFrameSize)`, +0 flags, +0x14 frame, +4 numNodes, +0x18 itself;
/// flag 2: no matrices (+8 = +0xC = 0), else `RwMalloc(numNodes * 64 + 15)` at +0xC and its 16-aligned
/// address at +8; node info `RwMalloc(numNodes * 16)` at +0x10, each node's id, index, flags read
/// (a failure returns NULL, the hierarchy not attached) and frame 0. The extension's +4 = hierarchy.
#[hook(addr = 0x7C4A00, conv = cdecl, name = "cb_HANIMPLUGIN_RwFrame_StreamRead", expect = "83 EC 0C A1 C8 B8 C9 00",
       unverified = "std6-rw-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn hanim_frame_stream_read(s: GPtr<u8>, _len: i32, frame: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let ext = frame.byte_add(GPtr::<i32>::from_addr(HANIM_FRAME_OFFSET).read() as isize);
    let mut words = [0i32; 4];
    let w = |i: usize, w: &mut [i32; 4]| GPtr::from_raw(&mut w[i] as *mut i32).cast::<u8>();
    if read_int32(s, w(0, &mut words), 4).is_null() {
        return GPtr::NULL;
    }
    if words[0] != 0x100 {
        return GPtr::NULL;
    }
    if read_int32(s, ext, 4).is_null() {
        return GPtr::NULL;
    }
    if read_int32(s, w(1, &mut words), 4).is_null() {
        return GPtr::NULL;
    }
    let num_nodes = words[1];
    if num_nodes <= 0 {
        return s;
    }
    if read_int32(s, w(2, &mut words), 4).is_null() {
        return GPtr::NULL;
    }
    if read_int32(s, w(3, &mut words), 4).is_null() {
        return GPtr::NULL;
    }
    let (flags, kfs) = (words[2], words[3]);
    let h = rw_memory_alloc(p8(GPtr::<u32>::from_addr(HANIM_FREELIST).read()), 0x3011E);
    for i in 0..9 {
        wr::<u32>(h, i * 4, 0);
    }
    if !h.is_null() {
        wr::<u32>(h, 0x20, interpolator_create_call(num_nodes, kfs).addr() as u32);
        wr::<i32>(h, 0, flags);
        wr::<u32>(h, 0x14, frame.addr() as u32);
        wr::<i32>(h, 4, num_nodes);
        wr::<u32>(h, 0x18, h.addr() as u32);
        if flags & 2 != 0 {
            wr::<u32>(h, 8, 0);
            wr::<u32>(h, 0xC, 0);
        } else {
            let m = rw_malloc(((num_nodes as u32) << 6).wrapping_add(0xF), 0x3011E);
            wr::<u32>(h, 0xC, m.addr() as u32);
            wr::<u32>(h, 8, (m.addr() as u32).wrapping_add(0xF) & !0xF);
        }
        let nodes = rw_malloc((num_nodes as u32) << 4, 0x3011E);
        wr::<u32>(h, 0x10, nodes.addr() as u32);
        let mut i = 0;
        while i < rd::<i32>(h, 4) {
            let n = nodes.byte_add(i as isize * 0x10);
            for o in [0, 4, 8] {
                if read_int32(s, n.byte_add(o), 4).is_null() {
                    return GPtr::NULL;
                }
            }
            wr::<u32>(n, 0xC, 0);
            i += 1;
        }
    }
    wr::<u32>(ext, 4, h.addr() as u32);
    s
}

fn ext(frame: GPtr<u8>) -> GPtr<u8> {
    frame.byte_add(GPtr::<i32>::from_addr(HANIM_FRAME_OFFSET).read() as isize)
}

/// `RpHAnimFrameSetHierarchy(frame, hierarchy)` (0x7C5130): the old hierarchy's parent frame (+0x14)
/// cleared, the new one stored and, when non-NULL, its parent frame = `frame`. Returns 1.
#[hook(addr = 0x7C5130, conv = cdecl, name = "RpHAnimFrameSetHierarchy", expect = "8B 54 24 04 A1 C8 B8 C9 00",
       unverified = "std6-rw-1", route = "L1-pure", hazards = "none")]
pub fn rp_hanim_frame_set_hierarchy(frame: GPtr<u8>, h: GPtr<u8>) -> u32 {
    let e = ext(frame);
    let old = p8(rd::<u32>(e, 4));
    if !old.is_null() {
        wr::<u32>(old, 0x14, 0);
    }
    wr::<u32>(e, 4, h.addr() as u32);
    if !h.is_null() {
        wr::<u32>(h, 0x14, frame.addr() as u32);
    }
    1
}

/// `RpHAnimFrameGetHierarchy(frame)` (0x7C5160).
#[hook(addr = 0x7C5160, conv = cdecl, name = "RpHAnimFrameGetHierarchy", expect = "8B 44 24 04 8B 0D C8 B8 C9 00 8B 44",
       unverified = "std6-rw-1", route = "L1-pure", hazards = "none")]
pub fn rp_hanim_frame_get_hierarchy(frame: GPtr<u8>) -> u32 {
    rd::<u32>(ext(frame), 4)
}

/// `RpHAnimFrameSetID(frame, id)` (0x7C5170). Returns 1.
#[hook(addr = 0x7C5170, conv = cdecl, name = "RpHAnimFrameSetID", expect = "8B 4C 24 04 8B 15 C8 B8 C9 00",
       unverified = "std6-rw-1", route = "L1-pure", hazards = "none")]
pub fn rp_hanim_frame_set_id(frame: GPtr<u8>, id: i32) -> u32 {
    wr::<i32>(ext(frame), 0, id);
    1
}

/// `RpHAnimFrameGetID(frame)` (0x7C5190).
#[hook(addr = 0x7C5190, conv = cdecl, name = "RpHAnimFrameGetID", expect = "8B 44 24 04 8B 0D C8 B8 C9 00 8B 04",
       unverified = "std6-rw-1", route = "L1-pure", hazards = "none")]
pub fn rp_hanim_frame_get_id(frame: GPtr<u8>) -> i32 {
    rd::<i32>(ext(frame), 0)
}

/// `RtAnimAnimationDestroy(animation)` (0x7CCF10): `RwFree`, returns 1.
#[hook(addr = 0x7CCF10, conv = cdecl, name = "RtAnimAnimationDestroy", expect = "8B 44 24 04 8B 0D 24 7B C9 00 50",
       unverified = "std6-rw-1", route = "L2", hazards = "heap-writes")]
pub fn rt_anim_animation_destroy(anim: GPtr<u8>) -> u32 {
    rw_free(anim);
    1
}

/// `RtAnim::RtAnimInterpolatorSetCurrentAnim(interp, anim)` (0x7CD5A0): the animation and time 0, the
/// scheme's interpolated / stored key-frame sizes (+0x24 / +0x28) and callbacks (+0x3C..+0x48); every
/// node's interpolated frame (`+0x4C + i * size`) is `interpolate(frame i, frame n + i, 0.0, custom
/// data)` with its previous / next frame pointers set to frames i and n + i; the next frame to use
/// (+8) is frame 2n. Returns 1.
#[hook(addr = 0x7CD5A0, conv = cdecl, name = "RtAnim::RtAnimInterpolatorSetCurrentAnim", expect = "53 8B 5C 24 0C 55 56",
       unverified = "std6-rw-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn rt_anim_interpolator_set_current_anim(it: GPtr<u8>, anim: GPtr<u8>) -> u32 {
    wr::<u32>(it, 0, anim.addr() as u32);
    wr::<u32>(it, 4, 0);
    let info = p8(rd::<u32>(anim, 0));
    wr::<u32>(it, 0x24, rd::<u32>(info, 4));
    wr::<u32>(it, 0x28, rd::<u32>(info, 8));
    for k in 0..4 {
        wr::<u32>(it, 0x3C + k * 4, rd::<u32>(info, 0xC + k * 4));
    }
    let mut i = 0i32;
    while i < rd::<i32>(it, 0x2C) {
        let n = rd::<i32>(it, 0x2C);
        let kfs = rd::<i32>(it, 0x28);
        let frames = rd::<u32>(anim, 0x10);
        let custom = rd::<u32>(anim, 0x14);
        let a = p8(frames.wrapping_add(kfs.wrapping_mul(i) as u32));
        let b = p8(frames.wrapping_add((n + i).wrapping_mul(kfs) as u32));
        let out = it.byte_add(0x4C + rd::<i32>(it, 0x24).wrapping_mul(i) as isize);
        call_interpolate_cb(rd::<u32>(it, 0x44), out, a, b, 0.0, custom);
        i += 1;
    }
    let n = rd::<i32>(it, 0x2C);
    let kfs = rd::<i32>(it, 0x28) as u32;
    let mut prev = rd::<u32>(anim, 0x10);
    let mut nxt = prev.wrapping_add(kfs.wrapping_mul(n as u32));
    let mut out = it.byte_add(0x4C);
    for _ in 0..n.max(0) {
        wr::<u32>(out, 0, prev);
        wr::<u32>(out, 4, nxt);
        out = out.byte_add(rd::<i32>(it, 0x24) as isize);
        prev = prev.wrapping_add(rd::<u32>(it, 0x28));
        nxt = nxt.wrapping_add(rd::<u32>(it, 0x28));
    }
    let n2 = (rd::<i32>(it, 0x2C).wrapping_mul(rd::<i32>(it, 0x28))) as u32;
    wr::<u32>(it, 8, rd::<u32>(anim, 0x10).wrapping_add(n2 * 2));
    1
}

/// `RtAnimInterpolatorSetKeyFrameCallBacks(interp, typeID)` (0x7CD660): the scheme's four callbacks
/// (else error {0x1B7, `_rwerror(2)`} and 0). Returns 1.
#[hook(addr = 0x7CD660, conv = cdecl, name = "RtAnimInterpolatorSetKeyFrameCallBacks", expect = "8B 15 00 79 C9 00 83 EC 08 33 C0 56",
       unverified = "std6-rw-1", route = "L2", hazards = "none")]
pub fn rt_anim_interpolator_set_key_frame_callbacks(it: GPtr<u8>, ty: u32) -> u32 {
    let count = GPtr::<i32>::from_addr(SCHEME_COUNT).read();
    for i in 0..count.max(0) {
        let info = p8(SCHEMES + i as u32 * 0x30);
        if rd::<u32>(info, 0) == ty {
            for k in 0..4 {
                wr::<u32>(it, 0x3C + k * 4, rd::<u32>(info, 0xC + k * 4));
            }
            return 1;
        }
    }
    let mut e = [0x1B7u32, 0];
    e[1] = rw_error1(2);
    rw_error_set(GPtr::from_raw(e.as_mut_ptr()));
    0
}

/// `RtAnimInterpolatorSetAnimLoopCallBack(interp, cb, data)` (0x7CD6F0): +0x18 / +0x1C. Returns
/// the interpolator.
#[hook(addr = 0x7CD6F0, conv = cdecl, name = "RtAnimInterpolatorSetAnimLoopCallBack", expect = "8B 44 24 04 8B 4C 24 08 8B 54 24 0C 89 48 18",
       unverified = "std6-rw-1", route = "L1-pure", hazards = "none")]
pub fn rt_anim_interpolator_set_anim_loop_callback(it: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> {
    wr::<u32>(it, 0x18, cb);
    wr::<u32>(it, 0x1C, data);
    it
}

/// `RtAnimInterpolatorSetAnimCallBack(interp, cb, time, data)` (0x7CD710): +0xC callback, +0x14 time,
/// +0x10 data. Returns the interpolator.
#[hook(addr = 0x7CD710, conv = cdecl, name = "RtAnimInterpolatorSetAnimCallBack", expect = "8B 44 24 04 8B 4C 24 08 8B 54 24 0C 89 48 0C",
       unverified = "std6-rw-1", route = "L1-pure", hazards = "none")]
pub fn rt_anim_interpolator_set_anim_callback(it: GPtr<u8>, cb: u32, time: u32, data: u32) -> GPtr<u8> {
    wr::<u32>(it, 0xC, cb);
    wr::<u32>(it, 0x14, time);
    wr::<u32>(it, 0x10, data);
    it
}

/// `RtAnimInterpolatorCopy(dst, src)` (0x7CD730): the interpolated frames (`dst` count x size bytes).
/// Returns 1.
#[hook(addr = 0x7CD730, conv = cdecl, name = "RtAnimInterpolatorCopy", expect = "8B 44 24 04 8B 54 24 08 56 57",
       unverified = "std6-rw-1", route = "L1-pure", hazards = "none")]
pub fn rt_anim_interpolator_copy(dst: GPtr<u8>, src: GPtr<u8>) -> u32 {
    let n = rd::<u32>(dst, 0x2C).wrapping_mul(rd::<u32>(dst, 0x24));
    for k in 0..n as isize {
        wr::<u8>(dst, 0x4C + k, rd::<u8>(src, 0x4C + k));
    }
    1
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_RT_ANIM_INTERPOLATOR_CREATE, HOOK_RT_ANIM_INTERPOLATOR_DESTROY, HOOK_HANIM_FRAME_STREAM_READ, HOOK_RP_HANIM_FRAME_SET_HIERARCHY, HOOK_RP_HANIM_FRAME_GET_HIERARCHY, HOOK_RP_HANIM_FRAME_SET_ID, HOOK_RP_HANIM_FRAME_GET_ID, HOOK_RT_ANIM_ANIMATION_DESTROY, HOOK_RT_ANIM_INTERPOLATOR_SET_CURRENT_ANIM, HOOK_RT_ANIM_INTERPOLATOR_SET_KEY_FRAME_CALLBACKS, HOOK_RT_ANIM_INTERPOLATOR_SET_ANIM_LOOP_CALLBACK, HOOK_RT_ANIM_INTERPOLATOR_SET_ANIM_CALLBACK, HOOK_RT_ANIM_INTERPOLATOR_COPY];
