//! STD-4 (load lane): `RpClumpStreamRead` (0x74B420) and `_rpClumpAtomicStreamRead` (0x74B060), ported
//! from the 1.0 US bytes (`docs/re/specs/RW/RpClump.md` is the reading aid), plus the GTA RW helper
//! callbacks `RwHelper::GetFirstAtomicCallback` (0x734810) and
//! `RwHelper::forceLinearFilteringMatTexturesCB` (0x734D60) the plant models reach.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{RwError, g, p8, rd, rw_free, rw_malloc, rw_memory_free, wr};

const CLUMP_REGISTRY: u32 = 0x8D6264;
const ATOMIC_REGISTRY: u32 = 0x8D624C;
/// Engine offset of the clump module (clump freelist +4, atomic freelist +0).
const CLUMP_MODULE: usize = 0xC924AC;
const LIGHT_EXT: usize = 0xC924DC;
const CAMERA_EXT: usize = 0xC924D8;
const E_READ: u32 = 0x8000_001A;
const E_VERSION: u32 = 0x8000_0004;

orig! {
    fn stream_find_chunk(s: GPtr<u8>, ty: u32, len: GPtr<u32>, ver: GPtr<u32>) -> i32 = cdecl 0x7ED2D0 => unported;
    fn stream_read(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => unported;
    fn stream_read_int32(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> GPtr<u8> = cdecl 0x7ED540 => unported;
    fn rw_error(code: u32) -> u32 = cdecl 0x8088D0 => unported;
    fn rw_error2(code: u32, a: u32) -> u32 = cdecl 0x8088D0 => unported;
    fn rw_error_set(e: GPtr<RwError>) -> GPtr<RwError> = cdecl 0x808820 => unported;
    fn clump_create() -> GPtr<u8> = cdecl 0x74A290 => unported;
    fn clump_destroy(c: GPtr<u8>) -> i32 = cdecl 0x74A310 => unported;
    fn atomic_create() -> GPtr<u8> = cdecl 0x749C50 => unported;
    fn atomic_destroy(a: GPtr<u8>) -> i32 = cdecl 0x749DC0 => unported;
    fn atomic_get_world(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x751050 => unported;
    fn registry_deinit_object(reg: u32, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808740 => unported;
    fn registry_read_data_chunks(reg: u32, s: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808980 => unported;
    fn registry_invoke_rights(reg: u32, id: u32, obj: GPtr<u8>, data: u32) -> i32 = cdecl 0x808AB0 => unported;
    fn frame_list_stream_read(s: GPtr<u8>, fl: GPtr<u8>) -> GPtr<u8> = cdecl 0x807970 => unported;
    fn frame_list_deinitialize(fl: GPtr<u8>) -> GPtr<u8> = cdecl 0x807720 => unported;
    fn frame_destroy_hierarchy(f: GPtr<u8>) -> i32 = cdecl 0x7F08A0 => unported;
    fn frame_update_objects(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => unported;
    fn object_has_frame_set_frame(o: GPtr<u8>, f: GPtr<u8>) -> GPtr<u8> = cdecl 0x804EF0 => unported;
    fn object_has_frame_release_frame(o: GPtr<u8>) -> GPtr<u8> = cdecl 0x804F40 => unported;
    fn resources_free_res_entry(e: GPtr<u8>) -> i32 = cdecl 0x807DE0 => unported;
    fn geometry_stream_read(s: GPtr<u8>) -> GPtr<u8> = cdecl 0x74D190 => unported;
    fn geometry_destroy(g: GPtr<u8>) -> i32 = cdecl 0x74CCC0 => unported;
    fn geometry_add_ref(g: GPtr<u8>) -> GPtr<u8> = cdecl 0x74CCB0 => unported;
    fn geometry_list_destroy(l: GPtr<u8>) -> GPtr<u8> = cdecl 0x74AE50 => unported;
    fn light_stream_read(s: GPtr<u8>) -> GPtr<u8> = cdecl 0x751F00 => unported;
    fn camera_stream_read(s: GPtr<u8>) -> GPtr<u8> = cdecl 0x808DE0 => unported;
    fn clump_for_all_atomics(c: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749B70 => unported;
    fn clump_for_all_lights(c: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749C00 => unported;
    fn clump_for_all_cameras(c: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749BB0 => unported;
    fn atomic_stream_read_call(s: GPtr<u8>, fl: GPtr<u8>, gl: GPtr<u8>) -> GPtr<u8> = cdecl 0x74B060 => atomic_stream_read;
}

/// `RwErrorSet({2, _rwerror(code)})` as these functions build it.
fn error(code: u32) {
    let mut e = RwError { plugin_id: 2, error_code: 0 };
    e.error_code = rw_error(code) as i32;
    rw_error_set(GPtr::from_raw(&mut e));
}

fn ok_version(v: u32) -> bool {
    (0x34000..=0x36003).contains(&v)
}

/// The inline `RpAtomicDestroy` of the atomic reader's failure paths: plugin data, the resource
/// entry, the geometry (frame update when in a world), the frame, the atomic freelist.
fn destroy_atomic_inline(a: GPtr<u8>) {
    registry_deinit_object(ATOMIC_REGISTRY, a);
    let res = rd::<u32>(a, 0x14);
    if res != 0 {
        resources_free_res_entry(p8(res));
    }
    let geo = rd::<u32>(a, 0x18);
    if geo != 0 {
        geometry_destroy(p8(geo));
        let frame = rd::<u32>(a, 4);
        wr::<u32>(a, 0x18, 0);
        if frame != 0 && !atomic_get_world(a).is_null() {
            frame_update_objects(p8(frame));
        }
    }
    object_has_frame_release_frame(a);
    let engine = g::<u32>(0xC97B24).read();
    let fl = GPtr::<u32>::from_addr(engine.wrapping_add(g::<u32>(CLUMP_MODULE).read()) as usize).read();
    rw_memory_free(p8(fl), a);
}

/// The inline `RpAtomicSetGeometry(atomic, geometry, 0)`: a different geometry gets a reference,
/// the old one is destroyed, the bounding sphere (+0x1C, 16 bytes) copied from the geometry's first
/// morph target (+4), and the frame updated when the atomic is in a world.
fn set_geometry_inline(a: GPtr<u8>, geo: u32) {
    if geo == rd::<u32>(a, 0x18) {
        return;
    }
    if geo != 0 {
        geometry_add_ref(p8(geo));
    }
    let old = rd::<u32>(a, 0x18);
    if old != 0 {
        geometry_destroy(p8(old));
    }
    wr::<u32>(a, 0x18, geo);
    if geo != 0 {
        let mt = rd::<u32>(p8(geo), 0x5C).wrapping_add(4);
        for k in 0..4 {
            wr::<u32>(a, 0x1C + 4 * k, rd::<u32>(p8(mt), 4 * k));
        }
    }
    let frame = rd::<u32>(a, 4);
    if frame != 0 && !atomic_get_world(a).is_null() {
        frame_update_objects(p8(frame));
    }
}

/// `_rpClumpAtomicStreamRead(stream, frameList, geometryList)` (0x74B060): the atomic's struct
/// chunk (version 0x34000..0x36003) read into a zeroed 16-byte record `{frame, geometry, flags,
/// unused}`, `RpAtomicCreate`, the flags byte, the frame (when the list has frames: set, private flag
/// 1), the geometry from the list (inline set) or streamed (inline set, creation reference dropped),
/// the plugin data with the rights globals 0xC924A8 / 0xC92480 cleared first and the rights invoked
/// when set. Errors (plugin 2) as the original; on a failed geometry the atomic is destroyed inline;
/// a failed plugin read leaks the atomic as the original does. Divergence: a struct chunk longer
/// than 16 bytes overflows the original's stack local; the port reads it into a buffer of its length
/// and keeps the first 16 bytes (the rest read in further `RwStreamRead` calls).
#[hook(addr = 0x74B060, conv = cdecl, name = "_rpClumpAtomicStreamRead", expect = "83 EC 20 8D 44 24 00",
       unverified = "std4-load-2", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn atomic_stream_read(s: GPtr<u8>, fl: GPtr<u8>, gl: GPtr<u8>) -> GPtr<u8> {
    let mut len: u32 = 0;
    let mut ver: u32 = 0;
    if stream_find_chunk(s, 1, GPtr::from_raw(&mut len), GPtr::from_raw(&mut ver)) == 0 {
        error(E_READ);
        return GPtr::NULL;
    }
    if !ok_version(ver) {
        error(E_VERSION);
        return GPtr::NULL;
    }
    let mut buf = [0u8; 16];
    let got = if len <= 16 {
        stream_read(s, GPtr::from_raw(buf.as_mut_ptr()), len)
    } else {
        let mut got = stream_read(s, GPtr::from_raw(buf.as_mut_ptr()), 16);
        let mut rest = len - 16;
        let mut scratch = [0u8; 64];
        while rest > 0 && got == len - rest {
            let n = rest.min(64);
            got = got.wrapping_add(stream_read(s, GPtr::from_raw(scratch.as_mut_ptr()), n));
            rest -= n;
        }
        got
    };
    if got != len {
        error(E_READ);
        return GPtr::NULL;
    }
    let w = |i: usize| u32::from_le_bytes([buf[4 * i], buf[4 * i + 1], buf[4 * i + 2], buf[4 * i + 3]]);
    let a = atomic_create();
    if a.is_null() {
        return GPtr::NULL;
    }
    wr::<u8>(a, 2, w(2) as u8);
    if rd::<u32>(fl, 4) != 0 {
        let frames = rd::<u32>(fl, 0);
        object_has_frame_set_frame(a, p8(rd::<u32>(p8(frames), 4 * w(0) as isize)));
        wr::<u8>(a, 3, rd::<u8>(a, 3) | 1);
    }
    if rd::<u32>(gl, 4) != 0 {
        let geos = rd::<u32>(gl, 0);
        set_geometry_inline(a, rd::<u32>(p8(geos), 4 * w(1) as isize));
    } else {
        let mut ver2: u32 = 0;
        if stream_find_chunk(s, 0xF, GPtr::NULL, GPtr::from_raw(&mut ver2)) == 0 {
            destroy_atomic_inline(a);
            error(E_READ);
            return GPtr::NULL;
        }
        if !ok_version(ver2) {
            destroy_atomic_inline(a);
            error(E_VERSION);
            return GPtr::NULL;
        }
        let geo = geometry_stream_read(s);
        if geo.is_null() {
            destroy_atomic_inline(a);
            error(E_READ);
            return GPtr::NULL;
        }
        set_geometry_inline(a, geo.addr() as u32);
        geometry_destroy(geo);
    }
    g::<u32>(0xC924A8).write(0);
    g::<u32>(0xC92480).write(0);
    if registry_read_data_chunks(ATOMIC_REGISTRY, s, a).is_null() {
        error(E_READ);
        return GPtr::NULL;
    }
    let rights = g::<u32>(0xC924A8).read();
    if rights != 0 {
        registry_invoke_rights(ATOMIC_REGISTRY, rights, a, g::<u32>(0xC92480).read());
    }
    a
}

/// The clump destruction of `RpClumpStreamRead`'s first failure paths (0x74B4F7): plugin data, every
/// atomic (`RpAtomicDestroy`, stopping at a NULL atomic pointer), lights and cameras (remove and
/// destroy callbacks), the frame hierarchy, the clump freelist (+4).
fn destroy_clump_c1(c: GPtr<u8>) {
    registry_deinit_object(CLUMP_REGISTRY, c);
    let head = c.addr() as u32 + 8;
    let mut l = rd::<u32>(c, 8);
    while l != head {
        let next = rd::<u32>(p8(l), 0);
        let a = l.wrapping_sub(0x40);
        atomic_destroy(p8(a));
        if a == 0 {
            break;
        }
        l = next;
    }
    destroy_clump_tail(c);
}

/// Lights, cameras, frame hierarchy and the clump freelist, then the read error.
fn destroy_clump_tail(c: GPtr<u8>) {
    clump_for_all_lights(c, 0x74A1C0, 0);
    clump_for_all_cameras(c, 0x74A200, 0);
    let parent = rd::<u32>(c, 4);
    if parent != 0 {
        frame_destroy_hierarchy(p8(parent));
    }
    let engine = g::<u32>(0xC97B24).read();
    let fl = GPtr::<u32>::from_addr(engine.wrapping_add(g::<u32>(CLUMP_MODULE).read()).wrapping_add(4) as usize).read();
    rw_memory_free(p8(fl), c);
    error(E_READ);
}

/// `RpClumpStreamRead(stream)` (0x74B420): see docs/re/specs/RW/RpClump.md. The clump struct
/// (12 bytes `{atomics, lights, cameras}`, version-checked), `RpClumpCreate`, the frame list
/// (the clump's parent is its first frame), the geometry list (struct with its count, each
/// geometry chunk version-checked and streamed), the atomics (`_rpClumpAtomicStreamRead`, linked at
/// the head of the clump's atomic list), lights and cameras (frame index, object, extension linked
/// at the head of the clump's lists), then the list references dropped, the array freed, the frame
/// list deinitialised and the clump's plugin data read (failure: `RpClumpDestroy`). Every failure
/// path cleans up as the original and sets the read error (plugin 2).
#[hook(addr = 0x74B420, conv = cdecl, name = "RpClumpStreamRead", expect = "83 EC 44 8D 44 24 04",
       route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn clump_stream_read(s: GPtr<u8>) -> GPtr<u8> {
    let mut len: u32 = 0;
    let mut ver: u32 = 0;
    if stream_find_chunk(s, 1, GPtr::from_raw(&mut len), GPtr::from_raw(&mut ver)) == 0 {
        error(E_READ);
        return GPtr::NULL;
    }
    if !ok_version(ver) {
        error(E_VERSION);
        return GPtr::NULL;
    }
    let mut info = [0i32; 3];
    if stream_read(s, GPtr::from_raw(info.as_mut_ptr()).cast(), 12) != 12 {
        error(E_READ);
        return GPtr::NULL;
    }
    let c = clump_create();
    if c.is_null() {
        return GPtr::NULL;
    }
    let mut ver2: u32 = 0;
    if stream_find_chunk(s, 0xE, GPtr::NULL, GPtr::from_raw(&mut ver2)) == 0 {
        destroy_clump_c1(c);
        return GPtr::NULL;
    }
    // rwFrameList {frames, numFrames}, rpGeometryList {geometries, numGeoms}
    let mut fl = [0u32; 2];
    let flp: GPtr<u8> = GPtr::from_raw(fl.as_mut_ptr()).cast();
    if frame_list_stream_read(s, flp).is_null() {
        destroy_clump_c1(c);
        return GPtr::NULL;
    }
    wr::<u32>(c, 4, rd::<u32>(p8(fl[0]), 0));
    if stream_find_chunk(s, 0x1A, GPtr::NULL, GPtr::from_raw(&mut ver2)) == 0 {
        frame_list_deinitialize(flp);
        destroy_clump_c1(c);
        return GPtr::NULL;
    }
    let mut gl = [0u32; 2];
    let glp: GPtr<u8> = GPtr::from_raw(gl.as_mut_ptr()).cast();
    // C2: frame list, plugin data, atomics (callback 0x74BCC0), lights, cameras, hierarchy, freelist
    let c2 = |c: GPtr<u8>, flp: GPtr<u8>| {
        frame_list_deinitialize(flp);
        registry_deinit_object(CLUMP_REGISTRY, c);
        clump_for_all_atomics(c, 0x74BCC0, 0);
        destroy_clump_tail(c);
    };
    // C3: the list's geometries destroyed and the array freed, then C2
    let c3 = |c: GPtr<u8>, gl: &mut [u32; 2]| {
        let mut i = 0i32;
        while i < gl[1] as i32 {
            geometry_destroy(p8(GPtr::<u32>::from_addr(gl[0] as usize).wrapping_index(i).read()));
            i += 1;
        }
        if gl[0] != 0 {
            rw_free(p8(gl[0]));
            gl[0] = 0;
        }
        c2(c, flp);
    };
    let mut len3: u32 = 0;
    let mut ver3: u32 = 0;
    if stream_find_chunk(s, 1, GPtr::from_raw(&mut len3), GPtr::from_raw(&mut ver3)) == 0 {
        c2(c, flp);
        return GPtr::NULL;
    }
    if !ok_version(ver3) {
        error(E_VERSION);
        c2(c, flp);
        return GPtr::NULL;
    }
    let mut n: i32 = 0;
    if stream_read(s, GPtr::from_raw(&mut n).cast(), 4) != 4 {
        c2(c, flp);
        return GPtr::NULL;
    }
    gl[1] = 0;
    if n > 0 {
        let arr = rw_malloc((n as u32).wrapping_mul(4), 0x3000F);
        gl[0] = arr.addr() as u32;
        if arr.is_null() {
            let mut e = RwError { plugin_id: 2, error_code: 0 };
            e.error_code = rw_error2(0x8000_0013, (n as u32).wrapping_mul(4)) as i32;
            rw_error_set(GPtr::from_raw(&mut e));
            c2(c, flp);
            return GPtr::NULL;
        }
    } else {
        gl[0] = 0;
    }
    let mut i = 0i32;
    while i < n {
        if stream_find_chunk(s, 0xF, GPtr::NULL, GPtr::from_raw(&mut ver3)) == 0 {
            c3(c, &mut gl);
            return GPtr::NULL;
        }
        if !ok_version(ver3) {
            geometry_list_destroy(glp);
            c2(c, flp);
            return GPtr::NULL;
        }
        let geo = geometry_stream_read(s);
        GPtr::<u32>::from_addr(gl[0] as usize).wrapping_index(i).write(geo.addr() as u32);
        if geo.is_null() {
            geometry_list_destroy(glp);
            c2(c, flp);
            return GPtr::NULL;
        }
        gl[1] += 1;
        i += 1;
    }
    for _ in 0..info[0].max(0) {
        if stream_find_chunk(s, 0x14, GPtr::NULL, GPtr::from_raw(&mut ver)) == 0 {
            c3(c, &mut gl);
            return GPtr::NULL;
        }
        let a = atomic_stream_read_call(s, flp, glp);
        if a.is_null() {
            c3(c, &mut gl);
            return GPtr::NULL;
        }
        let list = c.addr() as u32 + 8;
        let link = a.addr() as u32 + 0x40;
        wr::<u32>(a, 0x44, list);
        wr::<u32>(p8(link), 0, rd::<u32>(c, 8));
        wr::<u32>(p8(rd::<u32>(c, 8)), 4, link);
        wr::<u32>(c, 8, link);
        wr::<u32>(a, 0x3C, c.addr() as u32);
    }
    for (count, chunk, ext, list_off) in [(info[1], 0x12u32, LIGHT_EXT, 0x10u32), (info[2], 5, CAMERA_EXT, 0x18)] {
        for _ in 0..count.max(0) {
            let mut idx: i32 = 0;
            if stream_find_chunk(s, 1, GPtr::NULL, GPtr::NULL) == 0
                || stream_read_int32(s, GPtr::from_raw(&mut idx).cast(), 4).is_null()
                || stream_find_chunk(s, chunk, GPtr::NULL, GPtr::NULL) == 0
            {
                c3(c, &mut gl);
                return GPtr::NULL;
            }
            let o = if chunk == 0x12 { light_stream_read(s) } else { camera_stream_read(s) };
            if o.is_null() {
                c3(c, &mut gl);
                return GPtr::NULL;
            }
            object_has_frame_set_frame(o, p8(GPtr::<u32>::from_addr(fl[0] as usize).wrapping_index(idx).read()));
            let e = (o.addr() as u32).wrapping_add(g::<u32>(ext).read());
            let list = c.addr() as u32 + list_off;
            let link = e + 4;
            wr::<u32>(p8(e), 8, list);
            wr::<u32>(p8(link), 0, rd::<u32>(p8(list), 0));
            wr::<u32>(p8(rd::<u32>(p8(list), 0)), 4, link);
            wr::<u32>(p8(list), 0, link);
            wr::<u32>(p8(e), 0, c.addr() as u32);
        }
    }
    let mut i = 0i32;
    while i < gl[1] as i32 {
        geometry_destroy(p8(GPtr::<u32>::from_addr(gl[0] as usize).wrapping_index(i).read()));
        i += 1;
    }
    if gl[0] != 0 {
        // (the original also clears its local array pointer)
        rw_free(p8(gl[0]));
    }
    frame_list_deinitialize(flp);
    if registry_read_data_chunks(CLUMP_REGISTRY, s, c).is_null() {
        clump_destroy(c);
        error(E_READ);
        return GPtr::NULL;
    }
    c
}

/// `RwHelper::GetFirstAtomicCallback(atomic, data)` (0x734810): `*data = atomic`; returns NULL (stops
/// the iteration at the first atomic).
#[hook(addr = 0x734810, conv = cdecl, name = "RwHelper::GetFirstAtomicCallback", expect = "8B 44 24 08 8B 4C 24 04",
       unverified = "std4-load-2", route = "L2", hazards = "none", recs = "new-game")]
pub fn get_first_atomic_callback(a: GPtr<u8>, data: GPtr<u32>) -> GPtr<u8> {
    data.write(a.addr() as u32);
    GPtr::NULL
}

/// `RwHelper::forceLinearFilteringMatTexturesCB(material, filter)` (0x734D60): the material's
/// texture (if any) gets the filter mode in its filter byte (+0x50); returns the material.
#[hook(addr = 0x734D60, conv = cdecl, name = "RwHelper::forceLinearFilteringMatTexturesCB", expect = "8B 44 24 04 8B 08",
       unverified = "std4-load-2", route = "L2", hazards = "none", recs = "new-game")]
pub fn force_linear_filtering_cb(m: GPtr<u8>, filter: u32) -> GPtr<u8> {
    let t = rd::<u32>(m, 0);
    if t != 0 {
        wr::<u8>(p8(t), 0x50, filter as u8);
    }
    m
}

orig! {
    fn engine_get_plugin_offset(id: u32) -> i32 = cdecl 0x7F2BE0 => unported;
    fn atomic_get_plugin_offset(id: u32) -> i32 = cdecl 0x74BEC0 => unported;
    fn skin_atomic_set_pipeline(a: GPtr<u8>, ty: u32) -> GPtr<u8> = cdecl 0x7C89B0 => unported;
}

/// The RpSkin atomic "always" stream callback (0x7C6AE0, `RpAtomicSetStreamAlwaysCallBack`): the
/// pipeline type is 1, or 2 when the MatFX plugin (0x120) is attached and the atomic's MatFX flag is
/// set; a skinned geometry (`[0xC978A8]` extension non-NULL) gets `_rpSkinAtomicSetPipeline(atomic,
/// type)`, the type falling back to 1 when its plugin (MatFX 0x120 for 2, 0x12E for 3) is missing.
/// Returns 1.
#[hook(addr = 0x7C6AE0, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "56 57 68 20 01 00 00",
       route = "L2", hazards = "none", recs = "new-game")]
#[allow(clippy::if_same_then_else)]
pub fn skin_atomic_always_cb(a: GPtr<u8>, _offset: i32, _size: i32) -> u32 {
    let mut t = 1u32;
    if engine_get_plugin_offset(0x120) != -1 && rd::<u8>(a, atomic_get_plugin_offset(0x120) as isize) != 0 {
        t = 2;
    }
    let geo = rd::<u32>(a, 0x18);
    if geo != 0 && rd::<u32>(p8(geo), g::<u32>(0xC978A8).read() as i32 as isize) != 0 {
        if engine_get_plugin_offset(0x120) == -1 && t == 2 {
            t = 1;
        } else if engine_get_plugin_offset(0x12E) == -1 && t == 3 {
            t = 1;
        }
        skin_atomic_set_pipeline(a, t);
    }
    1
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_SKIN_ATOMIC_ALWAYS_CB,
    HOOK_ATOMIC_STREAM_READ,
    HOOK_CLUMP_STREAM_READ,
    HOOK_GET_FIRST_ATOMIC_CALLBACK,
    HOOK_FORCE_LINEAR_FILTERING_CB,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];

