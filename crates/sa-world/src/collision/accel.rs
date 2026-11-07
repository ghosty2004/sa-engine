//! `CColAccel`: the collision/IPL cache written on the first run (`m_iCacheState` 1) and replayed on
//! later loads (state 2). Ported from the 1.0 US bytes; gta-reversed `Collision/ColAccel.cpp` as reading
//! aid. Kept from the original:
//! - `isCacheLoading` reads the state through a protection chunk (0x40220F: `[[0x1560734] ^
//!   [0x4E3FB6]]` = 0xBC40A0, decoded statically);
//! - `getColDef` returns the incremented bound count in EAX (not the result pointer);
//! - `cacheLoadCol` copies into the new col model without a null check of `operator new`'s result
//!   (the original's SEH frame around the constructor is not reproduced: nothing in it throws);
//! - `addIPLEntity` dereferences the entity's LOD pointer without a null check.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_stream::modelinfo::base::MiPtr;

sa_core::globals! {
    list GLOBALS;
    /// `CColAccel::m_colBounds` (CColAccelColBound*, 0x18 each)
    pub static COL_BOUNDS @ 0xBC4090: GPtr<u8> = GPtr::NULL;
    /// `CColAccel::m_iplDefs` (IplDef*, 0x34 each)
    pub static IPL_DEFS @ 0xBC4094: GPtr<u8> = GPtr::NULL;
    /// `CColAccel::m_iSectionSize` (int32*)
    pub static SECTION_SIZE @ 0xBC4098: GPtr<i32> = GPtr::NULL;
    /// `CColAccel::m_iCachingColSize`
    pub static CACHING_COL_SIZE @ 0xBC409C: i32 = 0;
    /// `CColAccel::m_iCacheState`
    pub static CACHE_STATE @ 0xBC40A0: i32 = 0;
    /// `CColAccel::mp_caccColItems` (CColAccelColEntry*, 0x30 each)
    pub static COL_ITEMS @ 0xBC40A4: GPtr<u8> = GPtr::NULL;
    /// `CColAccel::m_iNumColItems`
    pub static NUM_COL_ITEMS @ 0xBC40A8: i32 = 0;
    /// `CColAccel::mp_caccIPLItems` (CColAccelIPLEntry*, 0x14 each)
    pub static IPL_ITEMS @ 0xBC40AC: GPtr<u8> = GPtr::NULL;
    /// `CColAccel::m_iNumIPLItems`
    pub static NUM_IPL_ITEMS @ 0xBC40B0: i32 = 0;
    /// `CColAccel::m_iNumSections`
    pub static NUM_SECTIONS @ 0xBC40B4: i32 = 0;
    /// `CColAccel::m_iNumColBounds`
    pub static NUM_COL_BOUNDS @ 0xBC40B8: i32 = 0;
    /// `CColAccel::mp_cCacheName` (the cache file name)
    pub static CACHE_NAME @ 0x8D0F84: u32 = 0x0086_8B14;
}

/// An `IplDef` passed by value (0x34 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct IplDefVal(pub [u32; 13]);

/// A `ColDef` passed by value (0x30 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ColDefVal(pub [u32; 12]);

orig! {
    /// CRT `operator new` (0x821195)
    pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x821195 => unported;
    /// CRT `operator delete` (0x8213AE)
    pub fn operator_delete(p: GPtr<u8>) = cdecl 0x8213AE => unported;
    /// `CFileMgr::OpenFileForWriting` (0x538910)
    pub fn open_file_for_writing(path: GPtr<u8>) -> GPtr<u8> = cdecl 0x538910 => sa_core::filemgr::open_file_for_writing;
    /// `CFileMgr::Write` (0x538970)
    pub fn file_write(f: GPtr<u8>, buf: GPtr<u8>, size: u32) -> u32 = cdecl 0x538970 => sa_core::filemgr::write;
    /// `CFileMgr::CloseFile` (0x5389D0)
    pub fn file_close(f: GPtr<u8>) -> i32 = cdecl 0x5389D0 => sa_core::filemgr::close_file;
    /// `CColModel::operator new` (0x40FC30)
    pub fn col_model_new(size: u32) -> GPtr<u8> = cdecl 0x40FC30 => col_model_new_port;
    /// `CColModel::CColModel` (0x40FB60)
    pub fn col_model_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x40FB60 => unported;
    /// `CBaseModelInfo::SetColModel(CColModel*, bool)` (0x4C4BC0)
    pub fn set_col_model(mi: MiPtr, col: GPtr<u8>, owns: u32) = thiscall 0x4C4BC0 => sa_stream::modelinfo::base::set_col_model;
    /// `CBaseModelInfo::DeleteCollisionModel` (0x4C4C40)
    pub fn delete_collision_model(mi: MiPtr) = thiscall 0x4C4C40 => unported;
    /// `CColStore::IncludeModelIndex(int32 slot, int32 model)` (0x410820)
    pub fn include_model_index(slot: i32, model: i32) = cdecl 0x410820 => sa_stream::colstore::include_model_index;
}

pub fn col_model_new_port(size: u32) -> GPtr<u8> {
    crate::collision::data::col_model_operator_new(size).cast()
}

fn mi(id: i32) -> MiPtr {
    sa_core::pools::MODEL_INFO_PTRS
        .ptr()
        .cast::<MiPtr>()
        .wrapping_index(id)
        .read()
}

fn copy_words(dst: GPtr<u8>, src: GPtr<u8>, n: i32) {
    let (d, s) = (dst.cast::<u32>(), src.cast::<u32>());
    for i in 0..n {
        d.wrapping_index(i).write(s.wrapping_index(i).read());
    }
}

fn at<T>(base: GPtr<u8>, off: i32) -> GPtr<T> {
    base.byte_add(off as isize).cast()
}

/// `CColAccel::isCacheLoading` (0x5B2AC0, entry `nop; jmp` into the protection chunk): `m_iCacheState
/// == 2` as 0/1 in EAX.
#[hook(addr = 0x5B2AC0, conv = cdecl, name = "CColAccel::isCacheLoading", expect = "90 E9 49 F7 E4 FF")]
pub fn is_cache_loading() -> u32 {
    (CACHE_STATE.get() == 2) as u32
}

/// `CColAccel::endCache` (0x5B2AD0): when building (state 1), writes the cache file (counts and arrays
/// in the original order); then frees the IPL items, col items (each when set), section sizes, IPL
/// defs and col bounds (0x8213AE), clears them and `m_iCacheState = 0`.
#[hook(addr = 0x5B2AD0, conv = cdecl, name = "CColAccel::endCache", expect = "83 3D A0 40 BC 00 01")]
pub fn end_cache() {
    if CACHE_STATE.get() == 1 {
        let f = open_file_for_writing(GPtr::from_addr(CACHE_NAME.get() as usize));
        file_write(f, NUM_COL_ITEMS.ptr().cast(), 4);
        file_write(f, COL_ITEMS.get(), (NUM_COL_ITEMS.get() * 0x30) as u32);
        file_write(f, NUM_SECTIONS.ptr().cast(), 4);
        file_write(f, SECTION_SIZE.get().cast(), (NUM_SECTIONS.get() * 4) as u32);
        file_write(f, IPL_DEFS.get(), 0x3400);
        file_write(f, NUM_COL_BOUNDS.ptr().cast(), 4);
        file_write(f, COL_BOUNDS.get(), (NUM_COL_BOUNDS.get() * 0x18) as u32);
        file_write(f, NUM_IPL_ITEMS.ptr().cast(), 4);
        file_write(f, IPL_ITEMS.get(), (NUM_IPL_ITEMS.get() * 0x14) as u32);
        file_close(f);
    }
    if !IPL_ITEMS.get().is_null() {
        operator_delete(IPL_ITEMS.get());
        IPL_ITEMS.set(GPtr::NULL);
    }
    if !COL_ITEMS.get().is_null() {
        operator_delete(COL_ITEMS.get());
        COL_ITEMS.set(GPtr::NULL);
    }
    operator_delete(SECTION_SIZE.get().cast());
    SECTION_SIZE.set(GPtr::NULL);
    operator_delete(IPL_DEFS.get());
    IPL_DEFS.set(GPtr::NULL);
    operator_delete(COL_BOUNDS.get());
    COL_BOUNDS.set(GPtr::NULL);
    CACHE_STATE.set(0);
}

/// `CColAccel::addCacheCol(int32 idx, const CColModel& cm)` (0x5B2C20): when building, item `n`
/// gets the model index (+0x28), the box and sphere (10 dwords), the col slot (+0x2C) and bit 0 of
/// the model's flags byte (+0x2D); `n++`.
#[hook(addr = 0x5B2C20, conv = cdecl, name = "CColAccel::addCacheCol", expect = "A1 A0 40 BC 00")]
pub fn add_cache_col(idx: i32, cm: GPtr<u8>) {
    if CACHE_STATE.get() != 1 {
        return;
    }
    let item = |n: i32| COL_ITEMS.get().byte_add((n * 0x30) as isize);
    at::<i32>(item(NUM_COL_ITEMS.get()), 0x28).write(idx);
    let mut local = [0u32; 10];
    copy_words(GPtr::from_raw(local.as_mut_ptr()).cast(), cm, 10);
    copy_words(
        item(NUM_COL_ITEMS.get()),
        GPtr::from_raw(local.as_mut_ptr()).cast(),
        10,
    );
    at::<u8>(item(NUM_COL_ITEMS.get()), 0x2C).write(at::<u8>(cm, 0x28).read());
    at::<u8>(item(NUM_COL_ITEMS.get()), 0x2D).write(at::<u8>(cm, 0x29).read() & 1);
    NUM_COL_ITEMS.set(NUM_COL_ITEMS.get().wrapping_add(1));
}

/// `CColAccel::cacheLoadCol` (0x5B2CC0): when loading (state 2), for every col item: a new `CColModel`
/// (`operator new` + constructor) gets the box and sphere, the col slot and bit 0 of the flags; it is
/// given to the model info with `SetColModel(cm, true)` and `CColStore::IncludeModelIndex(slot,
/// model)`.
#[hook(addr = 0x5B2CC0, conv = cdecl, name = "CColAccel::cacheLoadCol", expect = "64 A1 00 00 00 00")]
pub fn cache_load_col() {
    if CACHE_STATE.get() != 2 {
        return;
    }
    let mut i = 0;
    while i < NUM_COL_ITEMS.get() {
        let item = |i: i32| COL_ITEMS.get().byte_add((i * 0x30) as isize);
        let m = mi(at::<i32>(item(i), 0x28).read());
        let p = col_model_new(0x30);
        let cm = if p.is_null() { p } else { col_model_ctor(p) };
        copy_words(cm, item(i), 10);
        at::<u8>(cm, 0x28).write(at::<u8>(item(i), 0x2C).read());
        let b = at::<u8>(item(i), 0x2D).read();
        let f = at::<u8>(cm, 0x29).read();
        at::<u8>(cm, 0x29).write(f ^ ((b ^ f) & 1));
        set_col_model(m, cm, 1);
        let model = at::<i32>(item(i), 0x28).read();
        let slot = at::<u8>(item(i), 0x2C).read() as i32;
        include_model_index(slot, model);
        i += 1;
    }
}

/// `CColAccel::addColDef(ColDef)` (0x5B2DD0, the ColDef by value): bound `n` gets the area (4 dwords),
/// `m_nModelIdStart` (+0x22 → +0x10), `m_nModelIdEnd` (+0x24 → +0x12) and the bytes +0x2A/+0x2B
/// (→ +0x14/+0x15); `n++`.
#[hook(addr = 0x5B2DD0, conv = cdecl, name = "CColAccel::addColDef", expect = "A1 B8 40 BC 00")]
pub fn add_col_def(def: ColDefVal) {
    let bound = |n: i32| COL_BOUNDS.get().byte_add((n * 0x18) as isize);
    let b = bound(NUM_COL_BOUNDS.get());
    for (i, w) in def.0.iter().take(4).enumerate() {
        at::<u32>(b, 4 * i as i32).write(*w);
    }
    at::<u16>(bound(NUM_COL_BOUNDS.get()), 0x10).write((def.0[8] >> 16) as u16);
    at::<u16>(bound(NUM_COL_BOUNDS.get()), 0x12).write(def.0[9] as u16);
    at::<u8>(bound(NUM_COL_BOUNDS.get()), 0x14).write((def.0[10] >> 16) as u8);
    at::<u8>(bound(NUM_COL_BOUNDS.get()), 0x15).write((def.0[10] >> 24) as u8);
    NUM_COL_BOUNDS.set(NUM_COL_BOUNDS.get().wrapping_add(1));
}

/// `CColAccel::getColDef(ColDef& out)` (0x5B2E60): bound `n` (copied to a local first) back into the
/// ColDef fields `addColDef` reads; `n++`. EAX = the new `n`.
#[hook(addr = 0x5B2E60, conv = cdecl, name = "CColAccel::getColDef", expect = "A1 B8 40 BC 00")]
pub fn get_col_def(out: GPtr<u8>) -> i32 {
    let b = COL_BOUNDS.get().byte_add((NUM_COL_BOUNDS.get() * 0x18) as isize);
    let mut l = [0u32; 6];
    copy_words(GPtr::from_raw(l.as_mut_ptr()).cast(), b, 6);
    for (i, w) in l.iter().take(4).enumerate() {
        at::<u32>(out, 4 * i as i32).write(*w);
    }
    at::<u16>(out, 0x24).write((l[4] >> 16) as u16);
    at::<u16>(out, 0x22).write(l[4] as u16);
    at::<u8>(out, 0x2A).write(l[5] as u8);
    at::<u8>(out, 0x2B).write((l[5] >> 8) as u8);
    let n = NUM_COL_BOUNDS.get().wrapping_add(1);
    NUM_COL_BOUNDS.set(n);
    n
}

/// `CColAccel::setIplDef(int32 idx, IplDef def)` (0x5B2ED0): `m_iplDefs[idx] = def` (13 dwords).
#[hook(addr = 0x5B2ED0, conv = cdecl, name = "CColAccel::setIplDef", expect = "8B 0D 94 40 BC 00")]
pub fn set_ipl_def(idx: i32, def: IplDefVal) {
    let d = IPL_DEFS.get().byte_add(idx.wrapping_mul(0x34) as isize);
    for (i, w) in def.0.iter().enumerate() {
        at::<u32>(d, 4 * i as i32).write(*w);
    }
}

/// `CColAccel::getIplDef(int32 idx)` (0x5B2EF0, result by value through the hidden pointer `out`,
/// EAX = `out`).
#[hook(addr = 0x5B2EF0, conv = cdecl, name = "CColAccel::getIplDef", expect = "A1 94 40 BC 00")]
pub fn get_ipl_def(out: GPtr<u8>, idx: i32) -> GPtr<u8> {
    let s = IPL_DEFS.get().byte_add(idx.wrapping_mul(0x34) as isize);
    copy_words(out, s, 13);
    out
}

/// `CColAccel::cacheIPLSection(CEntity** entities)` (0x5B2F10). Loading (state 2): applies the IPL
/// items of the current section (`numIPL` up to `sectionSize[numSections]`, both re-read): flag 1 sets
/// bit 0x100000 of the LOD entity's flags, 8 sets bit 0x20 of the model info's byte +0x12, 4 sets its
/// draw distance to 400.0, 2 gives the LOD model the model's col model (`DeleteCollisionModel`, then
/// `SetColModel(col, false)`); then `numSections++`. Building (state 1): `sectionSize[numSections++] =
/// numIPL`.
#[hook(addr = 0x5B2F10, conv = cdecl, name = "CColAccel::cacheIPLSection", expect = "A1 A0 40 BC 00")]
pub fn cache_ipl_section(entities: GPtr<GPtr<u8>>) {
    match CACHE_STATE.get() {
        2 => {
            let limit = || SECTION_SIZE.get().wrapping_index(NUM_SECTIONS.get()).read();
            let item = || IPL_ITEMS.get().byte_add((NUM_IPL_ITEMS.get() * 0x14) as isize);
            let flags = || at::<u8>(item(), 0x10).read();
            let mut n = NUM_IPL_ITEMS.get();
            if n < limit() {
                loop {
                    if flags() & 1 != 0 {
                        let e = entities.wrapping_index(at::<i32>(item(), 4).read()).read();
                        let p = at::<u32>(e, 0x1C);
                        p.write(p.read() | 0x10_0000);
                    }
                    if flags() & 8 != 0 {
                        let p = at::<u8>(mi(at::<i32>(item(), 8).read()).cast(), 0x12);
                        p.write(p.read() | 0x20);
                    }
                    if flags() & 4 != 0 {
                        at::<u32>(mi(at::<i32>(item(), 8).read()).cast(), 0x18).write(0x43C8_0000);
                    }
                    if flags() & 2 != 0 {
                        let lod = mi(at::<i32>(item(), 0xC).read());
                        let m = mi(at::<i32>(item(), 8).read());
                        delete_collision_model(lod);
                        set_col_model(lod, at::<GPtr<u8>>(m.cast(), 0x14).read(), 0);
                    }
                    n = NUM_IPL_ITEMS.get().wrapping_add(1);
                    NUM_IPL_ITEMS.set(n);
                    if n >= limit() {
                        break;
                    }
                }
            }
            NUM_SECTIONS.set(NUM_SECTIONS.get().wrapping_add(1));
        }
        1 => {
            SECTION_SIZE
                .get()
                .wrapping_index(NUM_SECTIONS.get())
                .write(NUM_IPL_ITEMS.get());
            NUM_SECTIONS.set(NUM_SECTIONS.get().wrapping_add(1));
        }
        _ => {}
    }
}

/// `CColAccel::addIPLEntity(CEntity** entities, int32 count, int32 index)` (0x5B3040). When building:
/// flags 8 for an entity with LOD children (byte +0x34) or whose model's draw distance times
/// `TheCamera.m_fLODDistMultiplier` exceeds 300 (strict, NaN not); the LOD's index in `entities` (-1
/// when absent); for a LOD with exactly one child: 1 when the entity has flag 0x100000 (+0x1C), 2 when
/// the model has a col model different from the LOD model's; otherwise 4 when the model info's byte
/// +0x12 has bit 0x20. A non-zero flags word appends `{index, lodIndex, model, lodModel, flags}`.
#[hook(addr = 0x5B3040, conv = cdecl, name = "CColAccel::addIPLEntity", expect = "A1 A0 40 BC 00")]
pub fn add_ipl_entity(entities: GPtr<GPtr<u8>>, count: i32, index: i32) {
    if CACHE_STATE.get() != 1 {
        return;
    }
    let e = entities.wrapping_index(index).read();
    let lod = at::<GPtr<u8>>(e, 0x30).read();
    let lod_model = at::<i16>(lod, 0x22).read() as i32;
    let children = at::<u8>(e, 0x34).read();
    let model = at::<i16>(e, 0x22).read() as i32;
    let mut flags: u32 = 0;
    let far = || {
        let mut f = super::fpu::Fpu::new();
        f.fld(sa_stream::modelinfo::atomic::LOD_DIST_MULTIPLIER.get());
        f.fmul_m(at::<f32>(mi(model).cast(), 0x18).read());
        f.fcomp_m(f32::from_bits(0x4396_0000));
        !f.jne(0x41)
    };
    if children != 0 || far() {
        flags = 8;
    }
    let mut lod_index: i32 = -1;
    for i in 0..count.max(0) {
        if entities.wrapping_index(i).read() == lod {
            lod_index = i;
            break;
        }
    }
    let (m, lm) = (mi(model), mi(lod_model));
    if at::<u8>(lod, 0x34).read() == 1 {
        if at::<u32>(e, 0x1C).read() & 0x10_0000 != 0 {
            flags |= 1;
        }
        let col = at::<GPtr<u8>>(m.cast(), 0x14).read();
        if at::<GPtr<u8>>(lm.cast(), 0x14).read() != col && !col.is_null() {
            flags |= 2;
        }
    } else if at::<u8>(m.cast(), 0x12).read() & 0x20 != 0 {
        flags |= 4;
    }
    if flags != 0 {
        let d = IPL_ITEMS.get().byte_add((NUM_IPL_ITEMS.get() * 0x14) as isize);
        for (i, w) in [
            index as u32,
            lod_index as u32,
            model as u32,
            lod_model as u32,
            flags,
        ]
        .iter()
        .enumerate()
        {
            at::<u32>(d, 4 * i as i32).write(*w);
        }
        NUM_IPL_ITEMS.set(NUM_IPL_ITEMS.get().wrapping_add(1));
    }
}

/// `CColAccel::startCache` (0x5B31A0): `m_iCachingColSize = ms_pColModelPool->m_nSize`; section sizes
/// `new(0x100)`; 256 IPL defs `new(0x3400)` and 256 col bounds `new(0x1800)`, each rect set to
/// `{1e6, -1e6, -1e6, 1e6}` (left, bottom, right, top as stored) before the pointer is stored (null
/// when `new` fails).
#[hook(addr = 0x5B31A0, conv = cdecl, name = "CColAccel::startCache", expect = "A1 A4 44 B7 00")]
pub fn start_cache() {
    let pool: GPtr<u8> = sa_core::pools::COL_MODEL_POOL.get().cast();
    CACHING_COL_SIZE.set(at::<i32>(pool, 8).read());
    SECTION_SIZE.set(operator_new(0x100).cast());
    let init = |p: GPtr<u8>, stride: i32| {
        for i in 0..256 {
            let e = p.byte_add((i * stride) as isize);
            at::<u32>(e, 0).write(0x4974_2400);
            at::<u32>(e, 0xC).write(0x4974_2400);
            at::<u32>(e, 8).write(0xC974_2400);
            at::<u32>(e, 4).write(0xC974_2400);
        }
    };
    let defs = operator_new(0x3400);
    if !defs.is_null() {
        init(defs, 0x34);
    }
    IPL_DEFS.set(defs);
    let bounds = operator_new(0x1800);
    if !bounds.is_null() {
        init(bounds, 0x18);
    }
    COL_BOUNDS.set(bounds);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_IS_CACHE_LOADING,
    HOOK_END_CACHE,
    HOOK_ADD_CACHE_COL,
    HOOK_CACHE_LOAD_COL,
    HOOK_ADD_COL_DEF,
    HOOK_GET_COL_DEF,
    HOOK_SET_IPL_DEF,
    HOOK_GET_IPL_DEF,
    HOOK_CACHE_IPL_SECTION,
    HOOK_ADD_IPL_ENTITY,
    HOOK_START_CACHE,
];
