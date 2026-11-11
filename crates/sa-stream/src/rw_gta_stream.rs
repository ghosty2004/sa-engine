//! GTA's split RenderWare stream readers (0x72E140–0x72E6F7, 0x730E60–0x7311CA): texture
//! dictionaries and clumps read in two halves ("Read1" at stream start, "Read2" when the rest of the
//! file has arrived). gta-reversed has no code for them; ported from the 1.0 US bytes following
//! `docs/re/specs/STREAM/RwGtaStream.md`. RenderWare itself through `orig!`.
//!
//! Points worth knowing (all original behaviour):
//! - the dictionary readers read the struct chunk (`len` bytes) into a 4-byte header local followed
//!   by the `len` and `ver` locals: `len` up to 12 overwrites `len`/`ver` (the result compare then
//!   uses the overwritten `len`), more overwrites the return address (original UB; the port stops
//!   with a panic there). Real files have `len == 4`;
//! - `RwTextureGtaStreamRead` keeps a running average of the per-texture load time while the game
//!   state is 8 (x87 under PC=24; the cycle counter and rate are converted as unsigned through
//!   `fild` + `fadd 2^32`);
//! - the failure paths leak what they allocated (geometry arrays, already-read geometries);
//! - 0x72E140/0x72E220 take their list (and stream) in EBX/EDI: not hookable, ported as plain
//!   functions. Their callers here call the Rust functions directly (no toggle exists for them). The
//!   two register-ABI helpers they use, 0x72E270 (atomic read: EAX = stream, EBX = frame list, stack =
//!   geometry list) and 0x72E360 (geometry list deinitialise: ESI = list), are not STREAM rows
//!   (catalogue `UNK-game`) and are ported here as private helpers for the same reason.

use sa_core::mem::{GPtr, GameData};
use sa_core::video::GAME_STATE;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::shared::{field, image};

/// `rpGeometryList` / `rwFrameList`: `{ T** array; int32 count }`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PtrList {
    pub items: GPtr<GPtr<u8>>,
    pub count: i32,
    /// explicit tail padding on 64-bit hosts (none on 32-bit targets)
    pub _pad: [u8; PTR_LIST_PAD],
}

const PTR_LIST_PAD: usize = core::mem::size_of::<usize>() - 4;
// SAFETY: a game pointer and an integer.
#[allow(unsafe_code)]
unsafe impl GameData for PtrList {}

impl PtrList {
    pub const ZERO: PtrList = PtrList {
        items: GPtr::NULL,
        count: 0,
        _pad: [0; PTR_LIST_PAD],
    };
}

sa_core::globals! {
    list GLOBALS;
    /// `numberGeometrys` (geometries the first clump half read)
    pub static NUMBER_GEOMETRYS @ 0x8D607C: i32 = -1;
    /// `numberTextures` (textures left for the second dictionary half)
    pub static NUMBER_TEXTURES @ 0x8D6088: i32 = -1;
    /// `gClumpInfo` {numAtomics, numLights, numCameras}
    pub static CLUMP_INFO @ 0xC87AE8: [i32; 3] = [0; 3];
    /// `gFrameList`
    pub static FRAME_LIST @ 0xC87AF4: PtrList = PtrList::ZERO;
    /// clump `streamPosition`
    pub static CLUMP_STREAM_POSITION @ 0xC87AFC: u32 = 0;
    /// `gGeomList`
    pub static GEOM_LIST @ 0xC87B00: PtrList = PtrList::ZERO;
    /// texture dictionary `streamPosition`
    pub static TXD_STREAM_POSITION @ 0xC87FE8: u32 = 0;
    /// average texture load time in ms (proposed `gTexLoadAvgMs`)
    pub static TEX_LOAD_AVG_MS @ 0xC87FEC: f32 = 0.0;
    /// its sample count
    pub static TEX_LOAD_COUNT @ 0xC87FF0: i32 = 0;
    /// `anisotropySupportedByGFX`
    pub static ANISOTROPY_SUPPORTED @ 0xC87FFC: u8 = 0;
}

/// The local callback 0x730E50 (`RwTextureDestroy(t); return NULL;`), passed by address.
pub const DESTROY_FIRST_CB: u32 = 0x730E50;
/// `g_fx` (Fx_c)
pub const G_FX: u32 = 0xA9AE00;
/// f32 4294967296.0 at 0x858C54
pub const TWO_POW_32: u32 = 0x4F80_0000;

pub const RW_ID_STRUCT: u32 = 0x01;
pub const RW_ID_FRAMELIST: u32 = 0x0E;
pub const RW_ID_GEOMETRY: u32 = 0x0F;
pub const RW_ID_ATOMIC: u32 = 0x14;
pub const RW_ID_TEXTURENATIVE: u32 = 0x15;
pub const RW_ID_GEOMETRYLIST: u32 = 0x1A;

orig! {
    /// `RwStreamFindChunk(stream, type, lengthOut, versionOut)` (0x7ED2D0)
    pub fn rw_stream_find_chunk(stream: GPtr<u8>, ty: u32, len: GPtr<u32>, ver: GPtr<u32>) -> i32 = cdecl 0x7ED2D0 => unported;
    /// `RwStreamRead(stream, buf, len)` (0x7EC9D0)
    pub fn rw_stream_read(stream: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => unported;
    /// `RwStreamSkip(stream, n)` (0x7ECD00)
    pub fn rw_stream_skip(stream: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x7ECD00 => unported;
    /// `RpGeometryStreamRead(stream)` (0x74D190)
    pub fn rp_geometry_stream_read(stream: GPtr<u8>) -> GPtr<u8> = cdecl 0x74D190 => unported;
    /// `RpGeometryDestroy(geometry)` (0x74CCC0)
    pub fn rp_geometry_destroy(g: GPtr<u8>) -> i32 = cdecl 0x74CCC0 => unported;
    /// `CMemoryMgr::Malloc(size, hint)` (0x72F420)
    pub fn memory_mgr_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F420 => unported;
    /// `CMemoryMgr::Free(p)` (0x72F430)
    pub fn memory_mgr_free(p: GPtr<u8>) = cdecl 0x72F430 => unported;
    /// `RwTexDictionaryCreate()` (0x7F3600)
    pub fn rw_tex_dictionary_create() -> GPtr<u8> = cdecl 0x7F3600 => unported;
    /// `RwTexDictionaryDestroy(dict)` (0x7F36A0)
    pub fn rw_tex_dictionary_destroy(dict: GPtr<u8>) -> i32 = cdecl 0x7F36A0 => unported;
    /// `RwTexDictionaryForAllTextures(dict, cb, data)` (0x7F3730)
    pub fn rw_tex_dictionary_for_all_textures(dict: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3730 => unported;
    /// `RwTexDictionaryAddTexture(dict, tex)` (0x7F3980)
    pub fn rw_tex_dictionary_add_texture(dict: GPtr<u8>, tex: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3980 => unported;
    /// `RpClumpCreate()` (0x74A290)
    pub fn rp_clump_create() -> GPtr<u8> = cdecl 0x74A290 => unported;
    /// `RpClumpDestroy(clump)` (0x74A310)
    pub fn rp_clump_destroy(clump: GPtr<u8>) -> i32 = cdecl 0x74A310 => unported;
    /// `RpClumpAddAtomic(clump, atomic)` (0x74A490)
    pub fn rp_clump_add_atomic(clump: GPtr<u8>, atomic: GPtr<u8>) -> GPtr<u8> = cdecl 0x74A490 => unported;
    /// `RpAtomicCreate()` (0x749C50)
    pub fn rp_atomic_create() -> GPtr<u8> = cdecl 0x749C50 => unported;
    /// `RpAtomicDestroy(atomic)` (0x749DC0)
    pub fn rp_atomic_destroy(atomic: GPtr<u8>) -> i32 = cdecl 0x749DC0 => unported;
    /// `RpAtomicSetFrame(atomic, frame)` (0x74BF20)
    pub fn rp_atomic_set_frame(atomic: GPtr<u8>, frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x74BF20 => unported;
    /// `RpAtomicSetGeometry(atomic, geometry, flags)` (0x749D40)
    pub fn rp_atomic_set_geometry(atomic: GPtr<u8>, g: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x749D40 => unported;
    /// `_rwFrameListStreamRead(stream, frameList)` (0x807970)
    pub fn rw_frame_list_stream_read(stream: GPtr<u8>, fl: GPtr<PtrList>) -> GPtr<PtrList> = cdecl 0x807970 => unported;
    /// `_rwFrameListDeinitialize(frameList)` (0x807720)
    pub fn rw_frame_list_deinitialize(fl: GPtr<PtrList>) -> GPtr<PtrList> = cdecl 0x807720 => unported;
    /// `CTimer::GetCurrentTimeInCycles()` (0x561A80)
    pub fn get_current_time_in_cycles() -> u32 = cdecl 0x561A80 => sa_core::timer::get_current_time_in_cycles;
    /// `CTimer::GetCyclesPerMillisecond()` (0x561A40)
    pub fn get_cycles_per_millisecond() -> u32 = cdecl 0x561A40 => sa_core::timer::get_cycles_per_millisecond;
    /// `RwEngineInstance->stdFunc[rwSTANDARDNATIVETEXTUREREAD](stream, &texture, length)` (+0xB0)
    pub fn rw_native_texture_read(stream: GPtr<u8>, tex: GPtr<GPtr<u8>>, len: u32) -> i32 = cdecl table 0xC97B24 0xB0 => unported;
    /// `RpAnisotTextureGetMaxAnisotropy(tex)` (0x748F50, AL)
    pub fn rp_anisot_texture_get_max_anisotropy(tex: GPtr<u8>) -> i8 = cdecl 0x748F50 => unported;
    /// `RpAnisotGetMaxSupportedMaxAnisotropy()` (0x748F20)
    pub fn rp_anisot_get_max_supported_max_anisotropy() -> i32 = cdecl 0x748F20 => unported;
    /// `RpAnisotTextureSetMaxAnisotropy(tex, value)` (0x748F30)
    pub fn rp_anisot_texture_set_max_anisotropy(tex: GPtr<u8>, v: i32) -> GPtr<u8> = cdecl 0x748F30 => unported;
    /// `Fx_c::GetFxQuality()` (0x49EA50, thiscall on `g_fx`)
    pub fn fx_get_fx_quality(fx: GPtr<u8>) -> i32 = thiscall 0x49EA50 => unported;
}

/// `RwStream` +0xC: the memory stream position GTA saves and compares.
fn stream_pos(stream: GPtr<u8>) -> u32 {
    field::<u32, _>(stream, 0x0C).read()
}

/// `fild dword` of an unsigned value as the original converts it: `fild` (signed), then `fadd`
/// 4294967296.0 when the int32 is negative (rounded to 24 bits under PC=24).
fn u32_to_x87(v: u32) -> X87 {
    let x = X87::from_i32(v as i32);
    if (v as i32) < 0 { x.fadd(X87::constant(TWO_POW_32)) } else { x }
}

/// The `{hdr, len, ver}` stack block of the dictionary readers, read with `len` bytes into `hdr`.
/// Returns `(numTextures, len after the read)`, or `None` when the read count differs from `len`.
fn read_dict_header(stream: GPtr<u8>) -> Option<u16> {
    let mut blk = [0u32; 3];
    let p = GPtr::from_raw(blk.as_mut_ptr());
    if rw_stream_find_chunk(stream, RW_ID_STRUCT, p.wrapping_index(1), p.wrapping_index(2)) == 0 {
        return None;
    }
    let len = blk[1];
    if len > 12 {
        panic!("RwTexDictionaryGtaStreamRead: struct chunk of {len} bytes overflows the original's stack (original UB)");
    }
    let n = rw_stream_read(stream, p.cast(), len);
    // the result is compared with `len` re-read from its (possibly overwritten) stack slot
    if n != blk[1] {
        return None;
    }
    Some(blk[0] as u16)
}

/// The failure path of the dictionary readers: destroy the first texture (the callback stops the
/// walk by returning NULL), then the dictionary.
fn destroy_dict(dict: GPtr<u8>) {
    rw_tex_dictionary_for_all_textures(dict, DESTROY_FIRST_CB, GPtr::NULL);
    rw_tex_dictionary_destroy(dict);
}

orig! {
    /// `RwTextureGtaStreamRead(stream)` (0x730E60)
    pub fn rw_texture_gta_stream_read_call(stream: GPtr<u8>) -> GPtr<u8> = cdecl 0x730E60 => rw_texture_gta_stream_read;
}

orig! {
    /// `GeometryListStreamRead1` (0x72E140): stream on the stack (caller pops), list in EBX
    fn geometry_list_stream_read1_call(stream: GPtr<u8>, list: GPtr<PtrList>) -> GPtr<PtrList> = reg("args = stack ebx") 0x72E140 => geometry_list_stream_read1;
    /// `GeometryListStreamRead2` (0x72E220): stream in EBX, list in EDI
    fn geometry_list_stream_read2_call(stream: GPtr<u8>, list: GPtr<PtrList>) -> GPtr<PtrList> = reg("args = ebx edi") 0x72E220 => geometry_list_stream_read2;
}

/// `GeometryListStreamRead1` (0x72E140; list in EBX, stream on the stack, caller pops; returns the
/// list or NULL in EAX): reads the count and the first half (`n / 2`, truncating) of the geometries.
#[hook(addr = 0x72E140, conv = reg("args = stack ebx"), name = "(missed by ghidra: cat7-call)", expect = "83 EC 0C 55 8B 6C 24 14",
       unverified = "regconv-stream-1", route = "L2", hazards = "heap-writes", recs = "boot",
       note = "register convention stack + EBX (HARNESS-5; was an unhooked plain function)")]
pub fn geometry_list_stream_read1(stream: GPtr<u8>, list: GPtr<PtrList>) -> GPtr<PtrList> {
    NUMBER_GEOMETRYS.set(0);
    let mut lv = [0u32; 2];
    let lvp = GPtr::from_raw(lv.as_mut_ptr());
    if rw_stream_find_chunk(stream, RW_ID_STRUCT, lvp, lvp.wrapping_index(1)) == 0 {
        return GPtr::NULL;
    }
    let mut n: i32 = 0;
    if rw_stream_read(stream, GPtr::from_raw((&raw mut n).cast()), 4) != 4 {
        return GPtr::NULL;
    }
    NUMBER_GEOMETRYS.set(n / 2);
    field::<i32, _>(list, 4).write(n);
    if n > 0 {
        let p = memory_mgr_malloc((n as u32).wrapping_mul(4), 0);
        field::<GPtr<u8>, _>(list, 0).write(p);
        if p.is_null() {
            return GPtr::NULL;
        }
        for k in 0..n {
            p.cast::<u32>().wrapping_index(k).write(0);
        }
    } else {
        field::<GPtr<u8>, _>(list, 0).write(GPtr::NULL);
    }
    let mut i = 0;
    while i < NUMBER_GEOMETRYS.get() {
        if rw_stream_find_chunk(stream, RW_ID_GEOMETRY, GPtr::NULL, lvp.wrapping_index(1)) == 0 {
            return GPtr::NULL;
        }
        let g = rp_geometry_stream_read(stream);
        list.read().items.wrapping_index(i).write(g);
        if list.read().items.wrapping_index(i).read().is_null() {
            return GPtr::NULL;
        }
        i += 1;
    }
    list
}

/// `GeometryListStreamRead2` (0x72E220; stream in EBX, list in EDI; returns the list or NULL in EAX):
/// the geometries from `numberGeometrys` (read once) to `numGeoms` (re-read).
#[hook(addr = 0x72E220, conv = reg("args = ebx edi"), name = "FUN_0072e220", expect = "51 8B 47 04 56",
       unverified = "regconv-stream-1", route = "L2", hazards = "heap-writes", recs = "boot",
       note = "register convention EBX/EDI (HARNESS-5; was an unhooked plain function)")]
pub fn geometry_list_stream_read2(stream: GPtr<u8>, list: GPtr<PtrList>) -> GPtr<PtrList> {
    let mut ver = 0u32;
    let mut i = NUMBER_GEOMETRYS.get();
    while i < list.read().count {
        if rw_stream_find_chunk(stream, RW_ID_GEOMETRY, GPtr::NULL, GPtr::from_raw(&raw mut ver)) == 0 {
            return GPtr::NULL;
        }
        let g = rp_geometry_stream_read(stream);
        list.read().items.wrapping_index(i).write(g);
        if list.read().items.wrapping_index(i).read().is_null() {
            return GPtr::NULL;
        }
        i += 1;
    }
    list
}

/// The atomic reader 0x72E270 (EAX = stream, EBX = frame list, `[esp+4]` = geometry list; not a
/// STREAM row). The struct chunk is read into a 16-byte local `{frame, geometry, flags, unused}`;
/// a longer chunk overflows the original's stack (original UB, panic here).
fn atomic_stream_read(stream: GPtr<u8>, frames: GPtr<PtrList>, geoms: GPtr<PtrList>) -> GPtr<u8> {
    let mut lv = [0u32; 2];
    let lvp = GPtr::from_raw(lv.as_mut_ptr());
    if rw_stream_find_chunk(stream, RW_ID_STRUCT, lvp, lvp.wrapping_index(1)) == 0 {
        return GPtr::NULL;
    }
    let len = lv[0];
    if len > 16 {
        panic!("atomic struct chunk of {len} bytes overflows the original's stack (original UB)");
    }
    let mut buf = [0i32; 4];
    if rw_stream_read(stream, GPtr::from_raw(buf.as_mut_ptr()).cast(), len) != lv[0] {
        return GPtr::NULL;
    }
    let atomic = rp_atomic_create();
    if atomic.is_null() {
        return GPtr::NULL;
    }
    field::<u8, _>(atomic, 2).write(buf[2] as u8);
    if frames.read().count != 0 {
        let f = frames.read().items.wrapping_index(buf[0]).read();
        rp_atomic_set_frame(atomic, f);
    }
    if geoms.read().count != 0 {
        let g = geoms.read().items.wrapping_index(buf[1]).read();
        rp_atomic_set_geometry(atomic, g, 0);
        return atomic;
    }
    if rw_stream_find_chunk(stream, RW_ID_GEOMETRY, GPtr::NULL, lvp.wrapping_index(1)) == 0 {
        rp_atomic_destroy(atomic);
        return GPtr::NULL;
    }
    let g = rp_geometry_stream_read(stream);
    if g.is_null() {
        rp_atomic_destroy(atomic);
        return GPtr::NULL;
    }
    rp_atomic_set_geometry(atomic, g, 0);
    rp_geometry_destroy(g);
    atomic
}

/// The geometry list deinitialiser 0x72E360 (ESI = list; not a STREAM row): when `numGeoms != 0`
/// destroy every non-null geometry (count re-read) and free the array; then `numGeoms = 0`
/// (protection chunk 0x4020F3).
fn geometry_list_deinitialise(list: GPtr<PtrList>) -> GPtr<PtrList> {
    if list.read().count != 0 {
        let mut i = 0;
        while i < list.read().count {
            let g = list.read().items.wrapping_index(i).read();
            if !g.is_null() {
                rp_geometry_destroy(g);
            }
            i += 1;
        }
        memory_mgr_free(list.read().items.cast());
    }
    field::<i32, _>(list, 4).write(0);
    list
}

/// `RpClumpGtaStreamRead1(stream)` (0x72E570, cdecl, AL).
#[hook(addr = 0x72E570, conv = cdecl, name = "RpClumpGtaStreamRead1", expect = "83 EC 0C 56 8B 74 24 14")]
pub fn rp_clump_gta_stream_read1(stream: GPtr<u8>) -> u8 {
    let mut lv = [0u32; 2];
    let lvp = GPtr::from_raw(lv.as_mut_ptr());
    if rw_stream_find_chunk(stream, RW_ID_STRUCT, lvp, lvp.wrapping_index(1)) == 0 {
        return 0;
    }
    if rw_stream_read(stream, CLUMP_INFO.ptr().cast(), 12) != 12 {
        return 0;
    }
    if rw_stream_find_chunk(stream, RW_ID_FRAMELIST, GPtr::NULL, lvp.wrapping_index(1)) == 0 {
        return 0;
    }
    if rw_frame_list_stream_read(stream, FRAME_LIST.ptr()).is_null() {
        return 0;
    }
    if rw_stream_find_chunk(stream, RW_ID_GEOMETRYLIST, GPtr::NULL, lvp.wrapping_index(1)) != 0
        && !geometry_list_stream_read1_call(stream, GEOM_LIST.ptr()).is_null()
    {
        CLUMP_STREAM_POSITION.set(stream_pos(stream));
        return 1;
    }
    rw_frame_list_deinitialize(FRAME_LIST.ptr());
    0
}

/// `RpClumpGtaStreamRead2(stream)` (0x72E620, cdecl): the rest of the geometries, then the atomics;
/// lights and cameras are not read.
#[hook(addr = 0x72E620, conv = cdecl, name = "RpClumpGtaStreamRead2", expect = "51 55 E8 69 BC 01 00")]
pub fn rp_clump_gta_stream_read2(stream: GPtr<u8>) -> GPtr<u8> {
    let clump = rp_clump_create();
    if clump.is_null() {
        return GPtr::NULL;
    }
    rw_stream_skip(stream, CLUMP_STREAM_POSITION.get().wrapping_sub(stream_pos(stream)));
    let fail = |clump: GPtr<u8>| {
        geometry_list_deinitialise(GEOM_LIST.ptr());
        rw_frame_list_deinitialize(FRAME_LIST.ptr());
        rp_clump_destroy(clump);
        GPtr::NULL
    };
    if geometry_list_stream_read2_call(stream, GEOM_LIST.ptr()).is_null() {
        return fail(clump);
    }
    // RpClumpSetFrame as a raw store of frames[0] into clump->object.parent
    let f0 = FRAME_LIST.get().items.read();
    field::<GPtr<u8>, _>(clump, 4).write(f0);
    let mut ver = 0u32;
    let mut i = 0;
    while i < CLUMP_INFO.get()[0] {
        if rw_stream_find_chunk(stream, RW_ID_ATOMIC, GPtr::NULL, GPtr::from_raw(&raw mut ver)) == 0 {
            return fail(clump);
        }
        let a = atomic_stream_read(stream, FRAME_LIST.ptr(), GEOM_LIST.ptr());
        if a.is_null() {
            return fail(clump);
        }
        rp_clump_add_atomic(clump, a);
        i += 1;
    }
    geometry_list_deinitialise(GEOM_LIST.ptr());
    rw_frame_list_deinitialize(FRAME_LIST.ptr());
    clump
}

/// `RwTextureGtaStreamRead(stream)` (0x730E60, cdecl).
#[hook(addr = 0x730E60, conv = cdecl, name = "RwTextureGtaStreamRead", expect = "83 EC 18 56 8B 74 24 20")]
pub fn rw_texture_gta_stream_read(stream: GPtr<u8>) -> GPtr<u8> {
    let mut lv = [0u32; 2];
    let lvp = GPtr::from_raw(lv.as_mut_ptr());
    if rw_stream_find_chunk(stream, RW_ID_TEXTURENATIVE, lvp, lvp.wrapping_index(1)) == 0 {
        return GPtr::NULL;
    }
    let t = u32_to_x87(get_current_time_in_cycles()).to_f32();
    let start = u32_to_x87(get_cycles_per_millisecond()).fdivr_m(t).to_f32();
    // `tex` is an uninitialised local in the original; the native reader writes it on success
    let mut tex: GPtr<u8> = GPtr::NULL;
    if rw_native_texture_read(stream, GPtr::from_raw(&raw mut tex), lv[0]) == 0 {
        return GPtr::NULL;
    }
    if GAME_STATE.get() == 8 {
        let t2 = u32_to_x87(get_current_time_in_cycles()).to_f32();
        let d = u32_to_x87(get_cycles_per_millisecond()).fdivr_m(t2).fsub_m(start);
        let cnt = TEX_LOAD_COUNT.get();
        let new = cnt.wrapping_add(1);
        TEX_LOAD_COUNT.set(new);
        let avg = X87::from_i32(cnt)
            .fmul_m(TEX_LOAD_AVG_MS.get())
            .fadd(d)
            .fdiv(X87::from_i32(new));
        TEX_LOAD_AVG_MS.set(avg.to_f32());
    }
    if tex.is_null() {
        return GPtr::NULL;
    }
    let fa = field::<u8, _>(tex, 0x50);
    match fa.read() {
        1 => fa.write(2),
        3 => fa.write(4),
        _ => {}
    }
    if ANISOTROPY_SUPPORTED.get() != 0
        && rp_anisot_texture_get_max_anisotropy(tex) >= 1
        && fx_get_fx_quality(image(G_FX)) >= 2
    {
        let m = rp_anisot_get_max_supported_max_anisotropy();
        rp_anisot_texture_set_max_anisotropy(tex, m);
    }
    tex
}

/// `RwTexDictionaryGtaStreamRead(stream)` (0x730FC0, cdecl): the whole dictionary.
#[hook(addr = 0x730FC0, conv = cdecl, name = "RwTexDictionaryGtaStreamRead", expect = "83 EC 0C 57 8B 7C 24 14")]
pub fn rw_tex_dictionary_gta_stream_read(stream: GPtr<u8>) -> GPtr<u8> {
    let Some(mut n) = read_dict_header(stream) else {
        return GPtr::NULL;
    };
    let dict = rw_tex_dictionary_create();
    if dict.is_null() {
        return GPtr::NULL;
    }
    while n != 0 {
        n -= 1;
        let tex = rw_texture_gta_stream_read_call(stream);
        if tex.is_null() {
            destroy_dict(dict);
            return GPtr::NULL;
        }
        rw_tex_dictionary_add_texture(dict, tex);
    }
    dict
}

/// `RwTexDictionaryGtaStreamRead1(stream)` (0x731070, cdecl): the first `n - n/2` textures.
#[hook(addr = 0x731070, conv = cdecl, name = "RwTexDictionaryGtaStreamRead1", expect = "83 EC 0C 57 8B 7C 24 14")]
pub fn rw_tex_dictionary_gta_stream_read1(stream: GPtr<u8>) -> GPtr<u8> {
    NUMBER_TEXTURES.set(0);
    let Some(mut n) = read_dict_header(stream) else {
        return GPtr::NULL;
    };
    let dict = rw_tex_dictionary_create();
    if dict.is_null() {
        return GPtr::NULL;
    }
    NUMBER_TEXTURES.set((n >> 1) as i32);
    if (n as i32) > (n >> 1) as i32 {
        loop {
            let tex = rw_texture_gta_stream_read_call(stream);
            if tex.is_null() {
                destroy_dict(dict);
                return GPtr::NULL;
            }
            rw_tex_dictionary_add_texture(dict, tex);
            n = n.wrapping_sub(1);
            if (n as i32) <= NUMBER_TEXTURES.get() {
                break;
            }
        }
    }
    TXD_STREAM_POSITION.set(stream_pos(stream));
    NUMBER_TEXTURES.set(n as i32);
    dict
}

/// `RwTexDictionaryGtaStreamRead2(stream, dict)` (0x731150, cdecl): the remaining textures; ends
/// with `numberTextures = -1` (one decrement past 0).
#[hook(addr = 0x731150, conv = cdecl, name = "RwTexDictionaryGtaStreamRead2", expect = "A1 E8 7F C8 00")]
pub fn rw_tex_dictionary_gta_stream_read2(stream: GPtr<u8>, dict: GPtr<u8>) -> GPtr<u8> {
    rw_stream_skip(stream, TXD_STREAM_POSITION.get().wrapping_sub(stream_pos(stream)));
    while NUMBER_TEXTURES.get() != 0 {
        NUMBER_TEXTURES.set(NUMBER_TEXTURES.get().wrapping_sub(1));
        let tex = rw_texture_gta_stream_read_call(stream);
        if tex.is_null() {
            destroy_dict(dict);
            return GPtr::NULL;
        }
        rw_tex_dictionary_add_texture(dict, tex);
    }
    NUMBER_TEXTURES.set(NUMBER_TEXTURES.get().wrapping_sub(1));
    dict
}

/// Every `#[hook]` of this module, in source order.
pub const HOOKS: &[sa_core::abi::HookReg] = &[
    HOOK_GEOMETRY_LIST_STREAM_READ1,
    HOOK_GEOMETRY_LIST_STREAM_READ2,
    HOOK_RP_CLUMP_GTA_STREAM_READ1,
    HOOK_RP_CLUMP_GTA_STREAM_READ2,
    HOOK_RW_TEXTURE_GTA_STREAM_READ,
    HOOK_RW_TEX_DICTIONARY_GTA_STREAM_READ,
    HOOK_RW_TEX_DICTIONARY_GTA_STREAM_READ1,
    HOOK_RW_TEX_DICTIONARY_GTA_STREAM_READ2,
];
