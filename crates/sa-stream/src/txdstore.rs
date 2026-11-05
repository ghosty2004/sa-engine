//! `CTxdStore` (0x408340, 0x408370, 0x731680–0x73210D): the texture dictionary pool
//! (`CPool<TxdDef>`, 5000 slots) and its RenderWare glue.
//!
//! Ported from the 1.0 US bytes (gta-reversed `TxdStore.cpp` as reading aid; RenderWare through
//! `orig!`). Where they differ the original is kept:
//! - *new*: the pool lookups are **not null-checked**: `GetTxd`, `GetParentTxdSlot`, `SetCurrentTxd`,
//!   `AddRef`, `RemoveRef`, `RemoveRefWithoutDelete`, `GetNumRefs`, `Create`, `RemoveTxdSlot`,
//!   `SetupTxdParent`, `FinishLoadTxd`, `RemoveTxd` read or write through the slot pointer even when
//!   the slot is free (`GetAt` = 0, an access violation); `StartLoadTxd`/`LoadTxd` do so only after
//!   `RwStreamFindChunk` succeeded. gta-reversed checks for null and returns quietly. The port stops
//!   with [`null_deref`] at those points (the game ends in both).
//! - *new*: `GetNumRefs` returns the reference count **sign-extended** (`movsx`), gta-reversed
//!   returns the `uint16` zero-extended; `RemoveRef` compares the decremented count as `int16 <= 0`.
//! - `RemoveTxd` and `SetupTxdParent` inline `RemoveRef`/`AddRef` of the parent (no call), and
//!   `SetupTxdParent` stores the parent dictionary at `dict + ms_txdPluginOffset` itself.
//! - `PushCurrentTxd`'s store goes through a protection chunk (0x401B78): the target address is
//!   `[0x15606BC] - [0x59190A] - 1` (= `ms_pStoredTxd`, 0xC88010), computed at run time as the
//!   original does ([`stored_txd_cell`]).
//! - `Shutdown` frees the arrays with `free` (0x8214BD) and leaves `ms_pTxdPool` dangling (as gr).
//! - `LoadTxd(filename)` copies the name with `sprintf("%s")` into a 260-byte stack buffer (no length
//!   limit: a longer name overflows the original's stack, original UB) and retries `RwStreamOpen`
//!   until it succeeds.

use sa_core::mem::GPtr;
use sa_core::platform::TXD_PLUGIN_OFFSET;
use sa_core::pool;
use sa_core::{hook, orig};

use crate::shared::{RESOURCE_ID_TXD, TXD_POOL, TxdDef, TxdPool, field, image, nonnull};

sa_core::globals! {
    list GLOBALS;
    /// `CTxdStore::defaultTxds` (int16[4])
    pub static DEFAULT_TXDS @ 0xC88004: [i16; 4] = [0; 4];
    /// `CTxdStore::ms_pStoredTxd`
    pub static STORED_TXD @ 0xC88010: GPtr<u8> = GPtr::NULL;
    /// `CTxdStore::ms_lastSlotFound`
    pub static LAST_SLOT_FOUND @ 0xC88014: i32 = 0;
    /// Protection constants of `PushCurrentTxd`'s chunk (0x401B78): `.HOODLUM` dword ...
    pub static PROT_PUSH_A @ 0x15606BC: u32 = 0x00AF_829E;
    /// ... minus this `.text` dword minus 1 (`stc; sbb`) = &ms_pStoredTxd
    pub static PROT_PUSH_B @ 0x59190A: u32 = 0xFFE7_028D;
}

/// `TxdStoreFindCB` (0x731720), passed to RenderWare by address.
pub const FIND_CB: u32 = 0x731720;
/// `TxdStoreLoadCB` (0x731710).
pub const LOAD_CB: u32 = 0x731710;
/// `RemoveIfRefCountIsGreaterThanOne` (0x731680).
pub const REMOVE_IF_REF_CB: u32 = 0x731680;
/// "TexDictionary" (0x872AF0)
pub const NAME_TEX_DICTIONARY: u32 = 0x872AF0;
/// "*" (0x872AEC)
pub const NAME_STAR: u32 = 0x872AEC;
/// "%s" (0x85C714)
pub const FMT_S: u32 = 0x85C714;
/// `rwID_TEXDICTIONARY`
pub const RW_ID_TEXDICTIONARY: u32 = 0x16;

orig! {
    /// `RwTexDictionaryGetCurrent()` (0x7F3A90)
    pub fn rw_tex_dictionary_get_current() -> GPtr<u8> = cdecl 0x7F3A90 => unported;
    /// `RwTexDictionarySetCurrent(dict)` (0x7F3A70)
    pub fn rw_tex_dictionary_set_current(dict: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3A70 => unported;
    /// `RwTextureDestroy(tex)` (0x7F3820)
    pub fn rw_texture_destroy(tex: GPtr<u8>) -> i32 = cdecl 0x7F3820 => unported;
    /// `RwTexDictionaryRemoveTexture(tex)` (0x7F39C0)
    pub fn rw_tex_dictionary_remove_texture(tex: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F39C0 => unported;
    /// `RwTexDictionaryFindNamedTexture(dict, name)` (0x7F39F0)
    pub fn rw_tex_dictionary_find_named_texture(dict: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F39F0 => unported;
    /// `RwStreamFindChunk(stream, type, lengthOut, versionOut)` (0x7ED2D0)
    pub fn rw_stream_find_chunk(stream: GPtr<u8>, ty: u32, len: GPtr<u32>, ver: GPtr<u32>) -> i32 = cdecl 0x7ED2D0 => unported;
    /// `RwTexDictionaryCreate()` (0x7F3600)
    pub fn rw_tex_dictionary_create() -> GPtr<u8> = cdecl 0x7F3600 => unported;
    /// `RwTexDictionaryDestroy(dict)` (0x7F36A0)
    pub fn rw_tex_dictionary_destroy(dict: GPtr<u8>) -> i32 = cdecl 0x7F36A0 => unported;
    /// `RwTexDictionaryForAllTextures(dict, cb, data)` (0x7F3730)
    pub fn rw_tex_dictionary_for_all_textures(dict: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3730 => unported;
    /// `RwTextureSetFindCallBack(cb)` (0x7F34D0)
    pub fn rw_texture_set_find_call_back(cb: u32) -> i32 = cdecl 0x7F34D0 => unported;
    /// `RwTextureSetReadCallBack(cb)` (0x7F3500)
    pub fn rw_texture_set_read_call_back(cb: u32) -> i32 = cdecl 0x7F3500 => unported;
    /// `RwStreamOpen(type, access, data)` (0x7ECEF0)
    pub fn rw_stream_open(ty: u32, access: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7ECEF0 => unported;
    /// `RwStreamClose(stream, data)` (0x7ECE20)
    pub fn rw_stream_close(stream: GPtr<u8>, data: GPtr<u8>) -> i32 = cdecl 0x7ECE20 => unported;
    /// CRT `sprintf(buf, fmt, s)` (0x821BB5)
    pub fn sprintf_s(buf: GPtr<u8>, fmt: GPtr<u8>, s: GPtr<u8>) -> i32 = cdecl 0x821BB5 => unported;
    /// CRT `operator new` (0x82119A)
    pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// CRT `free` (0x8214BD)
    pub fn free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// `CKeyGen::GetUppercaseKey(name)` (0x53CF30)
    pub fn get_uppercase_key(name: GPtr<u8>) -> u32 = cdecl 0x53CF30 => sa_core::keygen::get_uppercase_key;
    /// `CStreaming::RemoveModel(id)` (0x4089A0)
    pub fn remove_model(id: i32) = cdecl 0x4089A0 => crate::streaming::remove_model;
    /// `RwTexDictionaryGtaStreamRead(stream)` (0x730FC0)
    pub fn rw_tex_dictionary_gta_stream_read(stream: GPtr<u8>) -> GPtr<u8> = cdecl 0x730FC0 => crate::rw_gta_stream::rw_tex_dictionary_gta_stream_read;
    /// `RwTexDictionaryGtaStreamRead1(stream)` (0x731070)
    pub fn rw_tex_dictionary_gta_stream_read1(stream: GPtr<u8>) -> GPtr<u8> = cdecl 0x731070 => crate::rw_gta_stream::rw_tex_dictionary_gta_stream_read1;
    /// `RwTexDictionaryGtaStreamRead2(stream, dict)` (0x731150)
    pub fn rw_tex_dictionary_gta_stream_read2(stream: GPtr<u8>, dict: GPtr<u8>) -> GPtr<u8> = cdecl 0x731150 => crate::rw_gta_stream::rw_tex_dictionary_gta_stream_read2;
    /// `CPool<TxdDef>::New()` (0x731B80)
    pub fn txd_pool_new(p: GPtr<TxdPool>) -> GPtr<TxdDef> = thiscall 0x731B80 => crate::misc::txd_pool_new;
    /// `CPool<TxdDef>::CPool(size, name)` (0x731AD0)
    pub fn txd_pool_ctor(p: GPtr<TxdPool>, size: i32, name: GPtr<u8>) -> GPtr<TxdPool> = thiscall 0x731AD0 => crate::misc::txd_pool_ctor;
    /// `CTxdStore::RemoveTxdSlot(i)` (0x731CD0)
    pub fn remove_txd_slot_call(i: i32) = cdecl 0x731CD0 => remove_txd_slot;
    /// `CTxdStore::SetupTxdParent(i)` (0x731D50)
    pub fn setup_txd_parent_call(i: i32) = cdecl 0x731D50 => setup_txd_parent;
    /// `CTxdStore::LoadTxd(i, stream)` (0x731DD0)
    pub fn load_txd_stream_call(i: i32, stream: GPtr<u8>) -> u8 = cdecl 0x731DD0 => load_txd_stream;
}

/// `ms_pTxdPool->GetAt(i)`: the slot, or null when its map byte has bit 7 set (no bounds check).
fn at(p: GPtr<TxdPool>, i: i32) -> GPtr<TxdDef> {
    pool::pool_at(p, i)
}

fn dict(d: GPtr<TxdDef>) -> GPtr<GPtr<u8>> {
    field(d, 0)
}
fn refs(d: GPtr<TxdDef>) -> GPtr<i16> {
    field(d, 4)
}
fn parent(d: GPtr<TxdDef>) -> GPtr<i16> {
    field(d, 6)
}
fn hash(d: GPtr<TxdDef>) -> GPtr<u32> {
    field(d, 8)
}

/// The address `PushCurrentTxd`'s protection chunk stores to: `[0x15606BC] - [0x59190A] - 1`.
pub fn stored_txd_cell() -> GPtr<GPtr<u8>> {
    if sa_core::PHASE_A {
        let a = PROT_PUSH_A.get().wrapping_sub(PROT_PUSH_B.get()).wrapping_sub(1);
        GPtr::from_addr(a as usize)
    } else {
        STORED_TXD.ptr()
    }
}

/// `CTxdStore::GetTxd(i)` (stub 0x408340 → .HOODLUM 0x156C9B0, cdecl): `m_pRwDictionary`.
#[hook(addr = 0x408340, conv = cdecl, name = "CTxdStore::GetTxd", expect = "E9 6B 46 16 01")]
pub fn get_txd(i: i32) -> GPtr<u8> {
    dict(nonnull(at(TXD_POOL.get(), i), "CTxdStore::GetTxd")).read()
}

/// `CTxdStore::GetParentTxdSlot(i)` (stub 0x408370 → .HOODLUM 0x15700A0, cdecl): `movsx` of the
/// parent index.
#[hook(addr = 0x408370, conv = cdecl, name = "CTxdStore::GetParentTxdSlot", expect = "E9 2B 7D 16 01")]
pub fn get_parent_txd_slot(i: i32) -> i32 {
    parent(nonnull(at(TXD_POOL.get(), i), "CTxdStore::GetParentTxdSlot")).read() as i32
}

/// `RemoveIfRefCountIsGreaterThanOne(texture, data)` (0x731680, cdecl): when `refCount` (+0x54,
/// signed) > 1, `RwTextureDestroy` then `RwTexDictionaryRemoveTexture`; returns the texture.
#[hook(addr = 0x731680, conv = cdecl, name = "CTxdStore::RemoveIfRefCountIsGreaterThanOne", expect = "56 8B 74 24 08")]
pub fn remove_if_ref_count_is_greater_than_one(tex: GPtr<u8>, _data: GPtr<u8>) -> GPtr<u8> {
    if field::<i32, _>(tex, 0x54).read() > 1 {
        rw_texture_destroy(tex);
        rw_tex_dictionary_remove_texture(tex);
    }
    tex
}

/// `CTxdStore::PushCurrentTxd()` (0x7316A0, cdecl): `ms_pStoredTxd = RwTexDictionaryGetCurrent()`
/// (the store happens in the protection chunk 0x401B78, see the module docs).
#[hook(addr = 0x7316A0, conv = cdecl, name = "CTxdStore::PushCurrentTxd", expect = "E8 EB 23 0C 00")]
pub fn push_current_txd() {
    let d = rw_tex_dictionary_get_current();
    stored_txd_cell().write(d);
}

/// `CTxdStore::PopCurrentTxd()` (0x7316B0, cdecl).
#[hook(addr = 0x7316B0, conv = cdecl, name = "CTxdStore::PopCurrentTxd", expect = "A1 10 80 C8 00")]
pub fn pop_current_txd() {
    rw_tex_dictionary_set_current(STORED_TXD.get());
    STORED_TXD.set(GPtr::NULL);
}

/// `CTxdStore::TxdStoreLoadCB(name, mask)` (0x731710, cdecl): returns null. Not hookable
/// (`xor eax, eax; ret` is 3 bytes).
pub fn txd_store_load_cb(_name: GPtr<u8>, _mask: GPtr<u8>) -> GPtr<u8> {
    GPtr::NULL
}

/// `CTxdStore::TxdStoreFindCB(name)` (0x731720, cdecl): search the current dictionary and its
/// parents (`*(dict + ms_txdPluginOffset)`).
#[hook(addr = 0x731720, conv = cdecl, name = "CTxdStore::TxdStoreFindCB", expect = "56 57 E8 69 23 0C 00")]
pub fn txd_store_find_cb(name: GPtr<u8>) -> GPtr<u8> {
    let mut d = rw_tex_dictionary_get_current();
    while !d.is_null() {
        let t = rw_tex_dictionary_find_named_texture(d, name);
        if !t.is_null() {
            return t;
        }
        d = d.byte_add(TXD_PLUGIN_OFFSET.get() as isize).cast::<GPtr<u8>>().read();
    }
    GPtr::NULL
}

/// The match test of both `FindTxdSlot` loops: used slot, `objects + 12 i` non-null, hash equal.
fn slot_matches(p: GPtr<TxdPool>, map: GPtr<u8>, i: i32, key: u32) -> bool {
    if (map.wrapping_index(i).read() as i8) < 0 {
        return false;
    }
    // `mov ecx, [pool]` is re-read for every slot
    let d = p.read().objects.wrapping_index(i);
    !d.is_null() && hash(d).read() == key
}

/// `CTxdStore::FindTxdSlot(name)` (0x731850, cdecl): downwards from `ms_lastSlotFound` to 0, then
/// upwards from `ms_lastSlotFound + 1` to `size - 1` (signed); a hit is cached in `ms_lastSlotFound`.
#[hook(addr = 0x731850, conv = cdecl, name = "CTxdStore::FindTxdSlot-name", expect = "8B 44 24 04 53")]
pub fn find_txd_slot_name(name: GPtr<u8>) -> i32 {
    let key = get_uppercase_key(name);
    let last = LAST_SLOT_FOUND.get();
    let p = TXD_POOL.get();
    let map = p.read().byte_map;
    let mut i = last;
    while i >= 0 {
        if slot_matches(p, map, i, key) {
            LAST_SLOT_FOUND.set(i);
            return i;
        }
        i -= 1;
    }
    let size = p.read().size;
    let map = p.read().byte_map;
    let mut i = last.wrapping_add(1);
    while i < size {
        if slot_matches(p, map, i, key) {
            LAST_SLOT_FOUND.set(i);
            return i;
        }
        i += 1;
    }
    -1
}

/// `CTxdStore::FindTxdSlot(hash)` (0x7318E0, cdecl): ascending, first match, no cache.
#[hook(addr = 0x7318E0, conv = cdecl, name = "CTxdStore::FindTxdSlot-hash", expect = "53 55 56 57 8B 3D 0C 80 C8 00")]
pub fn find_txd_slot_hash(key: u32) -> i32 {
    let p = TXD_POOL.get();
    let h = p.read();
    for i in 0..h.size.max(0) {
        if slot_matches(p, h.byte_map, i, key) {
            return i;
        }
    }
    -1
}

/// `CTxdStore::StartLoadTxd(i, stream)` (0x731930, cdecl, AL).
#[hook(addr = 0x731930, conv = cdecl, name = "CTxdStore::StartLoadTxd", expect = "8B 0D 0C 80 C8 00")]
pub fn start_load_txd(i: i32, stream: GPtr<u8>) -> bool {
    let d = at(TXD_POOL.get(), i);
    if rw_stream_find_chunk(stream, RW_ID_TEXDICTIONARY, GPtr::NULL, GPtr::NULL) == 0 {
        return false;
    }
    let t = rw_tex_dictionary_gta_stream_read1(stream);
    dict(nonnull(d, "CTxdStore::StartLoadTxd")).write(t);
    !t.is_null()
}

/// `CTxdStore::Create(i)` (0x731990, cdecl): `RwTexDictionaryCreate()` is called before the store
/// (also for a free slot, where the store then faults).
#[hook(addr = 0x731990, conv = cdecl, name = "CTxdStore::Create", expect = "8B 0D 0C 80 C8 00")]
pub fn create(i: i32) {
    let d = at(TXD_POOL.get(), i);
    let t = rw_tex_dictionary_create();
    dict(nonnull(d, "CTxdStore::Create")).write(t);
}

/// `CTxdStore::SetCurrentTxd(i)` (0x7319C0, cdecl): tail call `RwTexDictionarySetCurrent`.
#[hook(addr = 0x7319C0, conv = cdecl, name = "CTxdStore::SetCurrentTxd", expect = "8B 0D 0C 80 C8 00")]
pub fn set_current_txd(i: i32) {
    let d = nonnull(at(TXD_POOL.get(), i), "CTxdStore::SetCurrentTxd");
    rw_tex_dictionary_set_current(dict(d).read());
}

/// `CTxdStore::AddRef(i)` (0x731A00, cdecl): `inc word [def+4]`.
#[hook(addr = 0x731A00, conv = cdecl, name = "CTxdStore::AddRef", expect = "8B 0D 0C 80 C8 00")]
pub fn add_ref(i: i32) {
    let r = refs(nonnull(at(TXD_POOL.get(), i), "CTxdStore::AddRef"));
    r.write(r.read().wrapping_add(1));
}

/// `CTxdStore::RemoveRef(i)` (0x731A30, cdecl): decrement; `CStreaming::RemoveModel(i + 20000)`
/// when the count is `<= 0` as int16 (tail call).
#[hook(addr = 0x731A30, conv = cdecl, name = "CTxdStore::RemoveRef", expect = "A1 0C 80 C8 00")]
pub fn remove_ref(i: i32) {
    let r = refs(nonnull(at(TXD_POOL.get(), i), "CTxdStore::RemoveRef"));
    r.write(r.read().wrapping_sub(1));
    if r.read() <= 0 {
        remove_model(i.wrapping_add(RESOURCE_ID_TXD));
    }
}

/// `CTxdStore::RemoveRefWithoutDelete(i)` (0x731A70, cdecl).
#[hook(addr = 0x731A70, conv = cdecl, name = "CTxdStore::RemoveRefWithoutDelete", expect = "8B 0D 0C 80 C8 00")]
pub fn remove_ref_without_delete(i: i32) {
    let r = refs(nonnull(at(TXD_POOL.get(), i), "CTxdStore::RemoveRefWithoutDelete"));
    r.write(r.read().wrapping_sub(1));
}

/// `CTxdStore::GetNumRefs(i)` (0x731AA0, cdecl): the count sign-extended.
#[hook(addr = 0x731AA0, conv = cdecl, name = "CTxdStore::GetNumRefs", expect = "8B 0D 0C 80 C8 00")]
pub fn get_num_refs(i: i32) -> i32 {
    refs(nonnull(at(TXD_POOL.get(), i), "CTxdStore::GetNumRefs")).read() as i32
}

/// The slot fill of `AddTxdSlot` (also inlined into `Initialise`): dictionary 0, refs 0, parent -1,
/// hash of `name`; returns the index `(def - objects) / 12` (signed, objects re-read).
fn fill_new_slot(d: GPtr<TxdDef>, name: GPtr<u8>) -> i32 {
    let d = nonnull(d, "CTxdStore::AddTxdSlot");
    dict(d).write(GPtr::NULL);
    refs(d).write(0);
    parent(d).write(-1);
    let k = get_uppercase_key(name);
    hash(d).write(k);
    d.index_from(TXD_POOL.get().read().objects)
}

/// `CTxdStore::AddTxdSlot(name)` (0x731C80, cdecl).
#[hook(addr = 0x731C80, conv = cdecl, name = "CTxdStore::AddTxdSlot", expect = "8B 0D 0C 80 C8 00")]
pub fn add_txd_slot(name: GPtr<u8>) -> i32 {
    let d = txd_pool_new(TXD_POOL.get());
    fill_new_slot(d, name)
}

/// `CTxdStore::RemoveTxdSlot(i)` (0x731CD0, cdecl): destroys the dictionary, then the inlined
/// `CPool::Delete` (`map[idx] |= 0x80`, `first_free = idx` when lower, signed).
#[hook(addr = 0x731CD0, conv = cdecl, name = "CTxdStore::RemoveTxdSlot", expect = "8B 0D 0C 80 C8 00")]
pub fn remove_txd_slot(i: i32) {
    let p = TXD_POOL.get();
    let d = nonnull(at(p, i), "CTxdStore::RemoveTxdSlot");
    let t = dict(d).read();
    let p = if !t.is_null() {
        rw_tex_dictionary_destroy(t);
        TXD_POOL.get()
    } else {
        p
    };
    // GetAt again; for a slot freed meanwhile the original computes the index from 0 - objects
    let d2 = at(p, i);
    let h = p.read();
    let idx = if d2.is_null() {
        GPtr::<TxdDef>::NULL.index_from(h.objects)
    } else {
        d2.index_from(h.objects)
    };
    let m = h.byte_map.wrapping_index(idx);
    m.write(m.read() | 0x80);
    let ff = field::<i32, _>(p, 0x0C);
    if idx < ff.read() {
        ff.write(idx);
    }
}

/// `CTxdStore::SetupTxdParent(i)` (0x731D50, cdecl): `*(dict + ms_txdPluginOffset) = parent dict`,
/// then the parent's `AddRef` inline.
#[hook(addr = 0x731D50, conv = cdecl, name = "CTxdStore::SetupTxdParent", expect = "8B 44 24 04 56")]
pub fn setup_txd_parent(i: i32) {
    let p = TXD_POOL.get();
    let map = p.read().byte_map;
    let d = nonnull(at(p, i), "CTxdStore::SetupTxdParent");
    let par = parent(d).read();
    if par == -1 {
        return;
    }
    // the parent's map byte is tested with the map pointer read at entry
    let pd = if (map.wrapping_index(par as i32).read() as i8) < 0 {
        GPtr::NULL
    } else {
        p.read().objects.wrapping_index(par as i32)
    };
    let child = dict(d).read();
    let pdict = dict(nonnull(pd, "CTxdStore::SetupTxdParent (parent)")).read();
    child
        .byte_add(TXD_PLUGIN_OFFSET.get() as isize)
        .cast::<GPtr<u8>>()
        .write(pdict);
    let par = parent(d).read() as i32;
    let r = refs(nonnull(at(TXD_POOL.get(), par), "CTxdStore::SetupTxdParent (AddRef)"));
    r.write(r.read().wrapping_add(1));
}

/// `CTxdStore::LoadTxd(i, stream)` (0x731DD0, cdecl, AL).
#[hook(addr = 0x731DD0, conv = cdecl, name = "CTxdStore::LoadTxd-stream", expect = "A1 0C 80 C8 00")]
pub fn load_txd_stream(i: i32, stream: GPtr<u8>) -> u8 {
    let d = at(TXD_POOL.get(), i);
    if rw_stream_find_chunk(stream, RW_ID_TEXDICTIONARY, GPtr::NULL, GPtr::NULL) == 0 {
        return 0;
    }
    let t = rw_tex_dictionary_gta_stream_read(stream);
    let d = nonnull(d, "CTxdStore::LoadTxd");
    dict(d).write(t);
    if !t.is_null() {
        setup_txd_parent_call(i);
    }
    (!dict(d).read().is_null()) as u8
}

/// `CTxdStore::FinishLoadTxd(i, stream)` (0x731E40, cdecl, AL).
#[hook(addr = 0x731E40, conv = cdecl, name = "CTxdStore::FinishLoadTxd", expect = "A1 0C 80 C8 00")]
pub fn finish_load_txd(i: i32, stream: GPtr<u8>) -> u8 {
    let d = nonnull(at(TXD_POOL.get(), i), "CTxdStore::FinishLoadTxd");
    let t = rw_tex_dictionary_gta_stream_read2(stream, dict(d).read());
    dict(d).write(t);
    if !t.is_null() {
        setup_txd_parent_call(i);
    }
    (!dict(d).read().is_null()) as u8
}

/// `CTxdStore::RemoveTxd(i)` (0x731E90, cdecl).
#[hook(addr = 0x731E90, conv = cdecl, name = "CTxdStore::RemoveTxd", expect = "8B 0D 0C 80 C8 00")]
pub fn remove_txd(i: i32) {
    let mut p = TXD_POOL.get();
    let d = nonnull(at(p, i), "CTxdStore::RemoveTxd");
    let t = dict(d).read();
    if !t.is_null() {
        rw_tex_dictionary_for_all_textures(t, REMOVE_IF_REF_CB, GPtr::NULL);
        rw_tex_dictionary_destroy(dict(d).read());
        p = TXD_POOL.get();
    }
    let par = parent(d).read();
    if par != -1 {
        let r = refs(nonnull(at(p, par as i32), "CTxdStore::RemoveTxd (parent)"));
        r.write(r.read().wrapping_sub(1));
        if r.read() <= 0 {
            remove_model((par as i32).wrapping_add(RESOURCE_ID_TXD));
        }
    }
    dict(d).write(GPtr::NULL);
}

/// `CTxdStore::Initialise()` (0x731F20, cdecl): creates the pool when null (`operator new(0x14)`,
/// `CPool<TxdDef>(5000, "TexDictionary")`), adds four `"*"` slots into `defaultTxds` (AddTxdSlot
/// inlined), installs the RW texture callbacks.
#[hook(addr = 0x731F20, conv = cdecl, name = "CTxdStore::Initialise", expect = "6A FF 68 9B 8D 84 00")]
pub fn initialise() {
    if TXD_POOL.get().is_null() {
        let m: GPtr<TxdPool> = operator_new(0x14).cast();
        let p = if m.is_null() {
            GPtr::NULL
        } else {
            txd_pool_ctor(m, 5000, image(NAME_TEX_DICTIONARY))
        };
        TXD_POOL.set(p);
    }
    for k in 0..4 {
        let d = txd_pool_new(TXD_POOL.get());
        let idx = fill_new_slot(d, image(NAME_STAR));
        DEFAULT_TXDS.ptr().cast::<i16>().wrapping_index(k).write(idx as i16);
    }
    rw_texture_set_find_call_back(FIND_CB);
    rw_texture_set_read_call_back(LOAD_CB);
}

/// `CTxdStore::Shutdown()` (0x732000, cdecl): the pool's arrays (when `size > 0` and owned) and
/// the pool itself go to `free`; the header is cleared; `ms_pTxdPool` keeps its value.
#[hook(addr = 0x732000, conv = cdecl, name = "CTxdStore::Shutdown", expect = "A1 0C 80 C8 00")]
pub fn shutdown() {
    let p = TXD_POOL.get();
    if p.is_null() {
        return;
    }
    let h = p.read();
    if h.size > 0 {
        if h.owns_arrays.get() {
            free(h.objects.cast());
            free(p.read().byte_map);
        }
        field::<GPtr<u8>, _>(p, 0).write(GPtr::NULL);
        field::<GPtr<u8>, _>(p, 4).write(GPtr::NULL);
        field::<u32, _>(p, 8).write(0);
        field::<u32, _>(p, 0x0C).write(0);
    }
    free(p.cast());
}

/// `CTxdStore::GameShutdown()` (0x732060, cdecl): every used slot 0..5000 with zero references is
/// removed (`RemoveTxdSlot`); the pool pointer is re-read only after such a call.
#[hook(addr = 0x732060, conv = cdecl, name = "CTxdStore::GameShutdown", expect = "8B 0D 0C 80 C8 00")]
pub fn game_shutdown() {
    let mut p = TXD_POOL.get();
    for i in 0..5000 {
        let h = p.read();
        if (h.byte_map.wrapping_index(i).read() as i8) < 0 {
            continue;
        }
        let d = h.objects.wrapping_index(i);
        if d.is_null() || refs(d).read() != 0 {
            continue;
        }
        remove_txd_slot_call(i);
        p = TXD_POOL.get();
    }
}

/// `CTxdStore::LoadTxd(i, filename)` (0x7320B0, cdecl, AL; entry through the chunk 0x402161).
#[hook(addr = 0x7320B0, conv = cdecl, name = "CTxdStore::LoadTxd-filename", expect = "8B 44 24 08 90")]
pub fn load_txd_filename(i: i32, filename: GPtr<u8>) -> u8 {
    let mut buf = [0u8; 0x104];
    let b = GPtr::from_raw(buf.as_mut_ptr());
    sprintf_s(b, image(FMT_S), filename);
    let s = loop {
        let s = rw_stream_open(2, 1, b);
        if !s.is_null() {
            break s;
        }
    };
    let r = load_txd_stream_call(i, s);
    rw_stream_close(s, GPtr::NULL);
    r
}

/// Every `#[hook]` of this module, in source order.
pub const HOOKS: &[sa_core::abi::HookReg] = &[
    HOOK_GET_TXD,
    HOOK_GET_PARENT_TXD_SLOT,
    HOOK_REMOVE_IF_REF_COUNT_IS_GREATER_THAN_ONE,
    HOOK_PUSH_CURRENT_TXD,
    HOOK_POP_CURRENT_TXD,
    HOOK_TXD_STORE_FIND_CB,
    HOOK_FIND_TXD_SLOT_NAME,
    HOOK_FIND_TXD_SLOT_HASH,
    HOOK_START_LOAD_TXD,
    HOOK_CREATE,
    HOOK_SET_CURRENT_TXD,
    HOOK_ADD_REF,
    HOOK_REMOVE_REF,
    HOOK_REMOVE_REF_WITHOUT_DELETE,
    HOOK_GET_NUM_REFS,
    HOOK_ADD_TXD_SLOT,
    HOOK_REMOVE_TXD_SLOT,
    HOOK_SETUP_TXD_PARENT,
    HOOK_LOAD_TXD_STREAM,
    HOOK_FINISH_LOAD_TXD,
    HOOK_REMOVE_TXD,
    HOOK_INITIALISE,
    HOOK_SHUTDOWN,
    HOOK_GAME_SHUTDOWN,
    HOOK_LOAD_TXD_FILENAME,
];

#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    #[test]
    fn protected_cell_is_ms_p_stored_txd() {
        let a = super::PROT_PUSH_A.get().wrapping_sub(super::PROT_PUSH_B.get()).wrapping_sub(1);
        assert_eq!(a, super::STORED_TXD.addr());
    }
}
