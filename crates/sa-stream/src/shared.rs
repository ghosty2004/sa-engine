//! Types and globals used by more than one STREAM module (declared once here; the duplicate-global
//! test in `hooks` checks that no module declares an overlapping address again).
//!
//! Layouts follow gta-reversed's headers (pin 20d8d2a) and `catalog/layouts.csv`; sizes are asserted
//! against `sa_core::layouts::size` on 32-bit targets.

use sa_core::layouts::size;
use sa_core::mem::{GPtr, GameData};
use sa_core::pool::CPool;

// ------------------------------------------------------------------ resource ids (Streaming.h) ---

/// `RESOURCE_ID_DFF` .. : model ids of each streamed resource kind (1.0 US values).
pub const RESOURCE_ID_DFF: i32 = 0;
pub const RESOURCE_ID_TXD: i32 = 20000;
pub const RESOURCE_ID_COL: i32 = 25000;
pub const RESOURCE_ID_IPL: i32 = 25255;
pub const RESOURCE_ID_DAT: i32 = 25511;
pub const RESOURCE_ID_IFP: i32 = 25575;
pub const RESOURCE_ID_RRR: i32 = 25755;
pub const RESOURCE_ID_SCM: i32 = 26230;
pub const RESOURCE_ID_LOADED_LIST_START: i32 = 26312;
pub const RESOURCE_ID_LOADED_LIST_END: i32 = 26313;
pub const RESOURCE_ID_REQUEST_LIST_START: i32 = 26314;
pub const RESOURCE_ID_REQUEST_LIST_END: i32 = 26315;
/// Entries of `CStreaming::ms_aInfoForModel`.
pub const RESOURCE_ID_TOTAL: usize = 26316;
/// `CModelInfo::NUM_MODEL_INFOS`
pub const NUM_MODEL_INFOS: usize = 20000;

// ----------------------------------------------------------------------------- CStreamingInfo ---

/// `CStreamingInfo` (0x14).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CStreamingInfo {
    /// +0x00 `m_NextIndex` (index into `ms_pArrayBase`, -1 = none)
    pub next_index: i16,
    /// +0x02 `m_PrevIndex`
    pub prev_index: i16,
    /// +0x04 `m_NextIndexOnCd`
    pub next_index_on_cd: i16,
    /// +0x06 `m_Flags` (eStreamingFlags)
    pub flags: u8,
    /// +0x07 `m_ImgID`
    pub img_id: u8,
    /// +0x08 `m_CDOffset` (sectors)
    pub cd_offset: u32,
    /// +0x0C `m_CDSize` (sectors)
    pub cd_size: u32,
    /// +0x10 `m_LoadState` (eStreamingLoadState)
    pub load_state: u8,
    pub _pad: [u8; 3],
}

impl CStreamingInfo {
    /// All-zero (the `.bss` image before the static constructor runs).
    pub const ZERO: CStreamingInfo = CStreamingInfo {
        next_index: 0,
        prev_index: 0,
        next_index_on_cd: 0,
        flags: 0,
        img_id: 0,
        cd_offset: 0,
        cd_size: 0,
        load_state: 0,
        _pad: [0; 3],
    };
}

// SAFETY: integers only.
#[allow(unsafe_code)]
unsafe impl GameData for CStreamingInfo {}
const _: () = assert!(core::mem::size_of::<CStreamingInfo>() == size::CStreamingInfo);

/// `eStreamingFlags`
pub mod streaming_flags {
    pub const GAME_REQUIRED: u8 = 0x2;
    pub const MISSION_REQUIRED: u8 = 0x4;
    pub const KEEP_IN_MEMORY: u8 = 0x8;
    pub const PRIORITY_REQUEST: u8 = 0x10;
    pub const LOADING_SCENE: u8 = 0x20;
    pub const DONTREMOVE_IN_LOADSCENE: u8 = 0x3E;
}

/// `eStreamingLoadState`
pub mod load_state {
    pub const NOT_LOADED: u8 = 0;
    pub const LOADED: u8 = 1;
    pub const REQUESTED: u8 = 2;
    pub const READING: u8 = 3;
    pub const FINISHING: u8 = 4;
}

// ----------------------------------------------------------------------------- CBaseModelInfo ---

/// `CBaseModelInfo` (0x20 on 32-bit targets): the fields every model info class starts with.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CBaseModelInfo {
    /// +0x00 vtable
    pub vtable: u32,
    /// +0x04 `m_nKey` (CKeyGen::GetUppercaseKey of the name)
    pub key: u32,
    /// +0x08 `m_nRefCount`
    pub ref_count: u16,
    /// +0x0A `m_nTxdIndex`
    pub txd_index: i16,
    /// +0x0C `m_nAlpha`
    pub alpha: u8,
    /// +0x0D `m_n2dfxCount`
    pub num_2dfx: u8,
    /// +0x0E `m_n2dEffectIndex`
    pub effect_2d_index: i16,
    /// +0x10 `m_nObjectInfoIndex`
    pub object_info_index: i16,
    /// +0x12 `m_nFlags`
    pub flags: u16,
    /// +0x14 `m_pColModel`
    pub col_model: GPtr<u8>,
    /// +0x18 `m_fDrawDistance`
    pub draw_distance: f32,
    /// +0x1C `m_pRwObject` (RpClump* or RpAtomic*)
    pub rw_object: GPtr<u8>,
}

// SAFETY: integers, a float and game pointers only.
#[allow(unsafe_code)]
unsafe impl GameData for CBaseModelInfo {}
#[cfg(target_pointer_width = "32")]
const _: () = assert!(core::mem::size_of::<CBaseModelInfo>() == size::CBaseModelInfo);

/// Field offsets of [`CBaseModelInfo`] (for `GPtr::byte_add` field access).
pub mod mi {
    pub const VTABLE: isize = 0x00;
    pub const KEY: isize = 0x04;
    pub const REF_COUNT: isize = 0x08;
    pub const TXD_INDEX: isize = 0x0A;
    pub const ALPHA: isize = 0x0C;
    pub const NUM_2DFX: isize = 0x0D;
    pub const EFFECT_2D_INDEX: isize = 0x0E;
    pub const OBJECT_INFO_INDEX: isize = 0x10;
    pub const FLAGS: isize = 0x12;
    pub const COL_MODEL: isize = 0x14;
    pub const DRAW_DISTANCE: isize = 0x18;
    pub const RW_OBJECT: isize = 0x1C;
}

// ----------------------------------------------------------------------------------- store defs ---

/// `TxdDef` (0x0C): an entry of `CTxdStore::ms_pTxdPool`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TxdDef {
    /// +0x00 `m_pRwDictionary`
    pub dictionary: GPtr<u8>,
    /// +0x04 `m_wRefsCount`
    pub refs: u16,
    /// +0x06 `m_wParentIndex`
    pub parent: i16,
    /// +0x08 `m_hash`
    pub hash: u32,
}
// SAFETY: integers and a game pointer.
#[allow(unsafe_code)]
unsafe impl GameData for TxdDef {}
#[cfg(target_pointer_width = "32")]
const _: () = assert!(core::mem::size_of::<TxdDef>() == 0x0C);

/// `CRect` as stored in the store defs (left, bottom, right, top).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub left: f32,
    pub bottom: f32,
    pub right: f32,
    pub top: f32,
}
// SAFETY: floats only.
#[allow(unsafe_code)]
unsafe impl GameData for Rect {}

/// `ColDef` (0x2C): an entry of `CColStore`'s pool.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ColDef {
    /// +0x00 `m_Area`
    pub area: Rect,
    /// +0x10 `name[18]`
    pub name: [u8; 18],
    /// +0x22 `m_nModelIdStart`
    pub model_id_start: i16,
    /// +0x24 `m_nModelIdEnd`
    pub model_id_end: i16,
    /// +0x26 `m_nRefCount`
    pub ref_count: u16,
    /// +0x28 `m_bActive`
    pub active: u8,
    /// +0x29 `m_bCollisionIsRequired`
    pub collision_is_required: u8,
    /// +0x2A `m_bProcedural`
    pub procedural: u8,
    /// +0x2B `m_bInterior`
    pub interior: u8,
}
// SAFETY: integers and floats only.
#[allow(unsafe_code)]
unsafe impl GameData for ColDef {}
const _: () = assert!(core::mem::size_of::<ColDef>() == size::ColDef);

/// `IplDef` (0x34): an entry of `CIplStore`'s pool.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct IplDef {
    /// +0x00 `bb`
    pub bb: Rect,
    /// +0x10 `name[18]`
    pub name: [u8; 18],
    /// +0x22 `firstBuilding`
    pub first_building: i16,
    /// +0x24 `lastBuilding`
    pub last_building: i16,
    /// +0x26 `firstDummy`
    pub first_dummy: i16,
    /// +0x28 `lastDummy`
    pub last_dummy: i16,
    /// +0x2A `staticIdx`
    pub static_idx: i16,
    /// +0x2C `isInterior`
    pub is_interior: u8,
    /// +0x2D `loaded`
    pub loaded: u8,
    /// +0x2E `loadRequested`
    pub load_requested: u8,
    /// +0x2F `disableDynamicStreaming`
    pub disable_dynamic_streaming: u8,
    /// +0x30 `ignoreWhenDeleted`
    pub ignore_when_deleted: u8,
    /// +0x31 `isLarge`
    pub is_large: u8,
    pub _pad: [u8; 2],
}
// SAFETY: integers and floats only.
#[allow(unsafe_code)]
unsafe impl GameData for IplDef {}
const _: () = assert!(core::mem::size_of::<IplDef>() == 0x34);

pub type TxdPool = CPool<TxdDef>;
pub type ColPool = CPool<ColDef>;
pub type IplPool = CPool<IplDef>;

/// `CModelInfo::ms_modelInfoPtrs` (0xA9B0C8, `[CBaseModelInfo*; 20000]`), declared by sa-core (CPools).
pub use sa_core::pools::MODEL_INFO_PTRS;

sa_core::globals! {
    list GLOBALS;
    /// `CStreaming::ms_aInfoForModel` (owner: streaming)
    pub static INFO_FOR_MODEL @ 0x8E4CC0: [CStreamingInfo; RESOURCE_ID_TOTAL] = [CStreamingInfo::ZERO; RESOURCE_ID_TOTAL];
    /// `CStreamingInfo::ms_pArrayBase` (owner: streaming_info)
    pub static STREAMING_INFO_ARRAY_BASE @ 0x9654B4: GPtr<CStreamingInfo> = GPtr::NULL;
    /// `CTxdStore::ms_pTxdPool` (owner: txdstore)
    pub static TXD_POOL @ 0xC8800C: GPtr<TxdPool> = GPtr::NULL;
    /// `CColStore::ms_pColPool` (owner: colstore)
    pub static COL_POOL @ 0x965560: GPtr<ColPool> = GPtr::NULL;
    /// `CIplStore::ms_pPool` (owner: iplstore)
    pub static IPL_POOL @ 0x8E3FB0: GPtr<IplPool> = GPtr::NULL;
}

/// `&CStreaming::ms_aInfoForModel[id]` (no bounds check, as the original's `lea`).
pub fn info(id: i32) -> GPtr<CStreamingInfo> {
    INFO_FOR_MODEL.ptr().cast::<CStreamingInfo>().wrapping_index(id)
}

/// `CModelInfo::ms_modelInfoPtrs[id]` (no bounds check).
pub fn model_info(id: i32) -> GPtr<CBaseModelInfo> {
    MODEL_INFO_PTRS
        .ptr()
        .cast::<GPtr<CBaseModelInfo>>()
        .wrapping_index(id)
        .read()
}

/// `&CModelInfo::ms_modelInfoPtrs[id]` (the slot itself).
pub fn model_info_slot(id: i32) -> GPtr<GPtr<CBaseModelInfo>> {
    MODEL_INFO_PTRS.ptr().cast::<GPtr<CBaseModelInfo>>().wrapping_index(id)
}

/// A field of type `T` at byte offset `off` of the object at `p`.
pub fn field<T, U>(p: GPtr<U>, off: isize) -> GPtr<T> {
    p.byte_add(off).cast()
}

/// The original dereferences a null pointer at this point (no null check in the bytes; an access
/// violation that ends the game). Rust must not read or write through null, so the port stops here
/// with a panic naming the site: the observable result (the game ends) is the original's.
#[cold]
pub fn null_deref(what: &'static str) -> ! {
    panic!("{what}: the original dereferences a null pointer here (access violation)")
}

/// `p`, or [`null_deref`] when it is null.
pub fn nonnull<T>(p: GPtr<T>, what: &'static str) -> GPtr<T> {
    if p.is_null() {
        null_deref(what)
    }
    p
}

/// A pointer to a constant or code address in the original image (callbacks passed by address,
/// `.rdata` strings). Phases B/C need owned replacements where these are dereferenced.
pub fn image<T>(addr: u32) -> GPtr<T> {
    GPtr::from_addr(addr as usize)
}

/// `HOOKS` placeholder so `shared` fits the per-module pattern (no ports here).
pub const HOOKS: &[sa_core::abi::HookReg] = &[];
