//! `CBaseModelInfo` (vtable 0x85BB9C): the fields and non-virtual helpers every model info shares.
//!
//! Ported from the 1.0 US bytes (capstone over the exe, Ghidra decompilation as reading aid);
//! gta-reversed `Models/BaseModelInfo.cpp` is a reading aid only. Differences kept from the original
//! (see `docs/arch/stream-port.md`):
//! - the flag getters return the masked flag in EAX (`IsLod` = `flags & 0x80`, `HasComplexHierarchy`
//!   = `flags & 0x200`, ...), not a normalised `bool`; `IsTagModel`/`IsBreakableStatuePart`/
//!   `SwaysInWind` return 0/1 in EAX;
//! - `IsLod`/`SetIsLod` (0x4C4A00/0x4C4A10) use bit 7 of the low flags byte (`bIsLod`); `SetColModel`
//!   sets it from its second argument and `DeleteCollisionModel`/`Shutdown` delete the col model only
//!   when it is set (as gta-reversed);
//! - `SetColModel` (0x4C4BC0) walks the time-model chain iteratively: every "other time model" gets the
//!   same col model with `bIsLod` cleared;
//! - `Add2dEffect` computes the store index with a signed 32-bit `(effect - store - 4) >> 6`;
//! - `Get2dEffect` calls `GetRwModelType` a second time when the first test fails (two virtual calls).
//!
//! Virtual calls go through `orig!` `vslot`s (slot list in `docs/re/specs/STREAM/CBaseModelInfo.md`).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::shared::{field, mi, model_info, CBaseModelInfo};

/// A model info pointer.
pub type MiPtr = GPtr<CBaseModelInfo>;

/// `CBaseModelInfo` vtable (what every reduced model-info destructor stores).
pub const VTBL_BASE_MODEL_INFO: u32 = 0x85BB9C;

/// vtable slot offsets (CBaseModelInfo.md)
pub mod slot {
    pub const AS_ATOMIC: u32 = 0x04;
    pub const GET_MODEL_TYPE: u32 = 0x10;
    pub const GET_TIME_INFO: u32 = 0x14;
    pub const INIT: u32 = 0x18;
    pub const SHUTDOWN: u32 = 0x1C;
    pub const DELETE_RW_OBJECT: u32 = 0x20;
    pub const GET_RW_MODEL_TYPE: u32 = 0x24;
    pub const CREATE_INSTANCE: u32 = 0x2C;
    pub const GET_ANIM_FILE_INDEX: u32 = 0x38;
}

sa_core::globals! {
    list GLOBALS;
    // foreign (owner: RW 2d-effect plugin, `g2dEffectPluginOffset`)
    /// Offset of the 2d-effect plugin data in `RpGeometry`
    pub static G2D_EFFECT_PLUGIN_OFFSET @ 0xC3A1E0: u32 = 0;
}

orig! {
    /// CRT `operator delete` / `free` (0x8214BD)
    pub fn operator_delete(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// `CTxdStore::AddRef` (0x731A00)
    pub fn txd_add_ref(index: i32) = cdecl 0x731A00 => crate::txdstore::add_ref;
    /// `CTxdStore::RemoveRef` (0x731A30)
    pub fn txd_remove_ref(index: i32) = cdecl 0x731A30 => crate::txdstore::remove_ref;
    /// `CTxdStore::FindTxdSlot(const char*)` (0x731850)
    pub fn txd_find_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731850 => crate::txdstore::find_txd_slot_name;
    /// `CTxdStore::AddTxdSlot` (0x731C80)
    pub fn txd_add_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731C80 => crate::txdstore::add_txd_slot;
    /// `CColModel::~CColModel` (0x40F700)
    pub fn col_model_destructor(col: GPtr<u8>) = thiscall 0x40F700 => unported;
    /// `CColModel::operator delete` (0x40FC40)
    pub fn col_model_operator_delete(col: GPtr<u8>) = cdecl 0x40FC40 => unported;
    /// `RwHelper::Get2DEffectAtomic` (0x734880)
    pub fn get_2d_effect_atomic(clump: GPtr<u8>) -> GPtr<u8> = cdecl 0x734880 => unported;
    /// `CModelInfo::Get2dEffectStore` (0x4C5A60)
    pub fn get_2d_effect_store_call() -> GPtr<u8> = cdecl 0x4C5A60 => super::model_info::get_2d_effect_store;
    /// virtual `GetTimeInfo` (slot 5)
    pub fn vcall_get_time_info(this: MiPtr) -> GPtr<u8> = thiscall vslot 0x14 => unported;
    /// virtual `DeleteRwObject` (slot 8)
    pub fn vcall_delete_rw_object(this: MiPtr) = thiscall vslot 0x20 => unported;
    /// virtual `GetRwModelType` (slot 9)
    pub fn vcall_get_rw_model_type(this: MiPtr) -> i32 = thiscall vslot 0x24 => unported;
    /// virtual `Shutdown` (slot 7)
    pub fn vcall_shutdown(this: MiPtr) = thiscall vslot 0x1C => unported;
    /// virtual `GetModelType` (slot 4, AL)
    pub fn vcall_get_model_type(this: MiPtr) -> u8 = thiscall vslot 0x10 => unported;
    /// virtual `Init` (slot 6)
    pub fn vcall_init(this: MiPtr) = thiscall vslot 0x18 => unported;
    /// virtual `GetAnimFileIndex` (slot 14)
    pub fn vcall_get_anim_file_index(this: MiPtr) -> i32 = thiscall vslot 0x38 => unported;
    /// virtual `CreateInstance()` (slot 11)
    pub fn vcall_create_instance(this: MiPtr) -> GPtr<u8> = thiscall vslot 0x2C => unported;
}

/// A field of the model info at byte offset `off`.
pub fn f<T>(this: MiPtr, off: isize) -> GPtr<T> {
    field(this, off)
}

fn flags_lo(this: MiPtr) -> GPtr<u8> {
    f(this, mi::FLAGS)
}

fn flags(this: MiPtr) -> u16 {
    f::<u16>(this, mi::FLAGS).read()
}

/// The 2d-effect count the plugin stores for `geometry` (the original's inlined
/// `RpGeometryGet2dFxCount`): `p = *(geometry + offset); p ? *p : 0`.
pub fn geometry_2dfx_count(geometry: GPtr<u8>, plugin_offset: u32) -> u32 {
    let p = geometry.byte_add(plugin_offset as isize).cast::<GPtr<u32>>().read();
    if p.is_null() { 0 } else { p.read() }
}

/// [`geometry_2dfx_count`] of `atomic->geometry` (+0x18) with the plugin offset global.
pub fn atomic_2dfx_count(atomic: GPtr<u8>) -> u32 {
    let geometry = atomic.byte_add(0x18).cast::<GPtr<u8>>().read();
    geometry_2dfx_count(geometry, G2D_EFFECT_PLUGIN_OFFSET.get())
}

/// `byte += v` / `byte -= v` on the 2dfx count (+0x0D), 8-bit wrapping.
pub fn add_2dfx_count(this: MiPtr, v: u32, sub: bool) {
    let p = f::<u8>(this, mi::NUM_2DFX);
    let b = p.read();
    p.write(if sub { b.wrapping_sub(v as u8) } else { b.wrapping_add(v as u8) });
}

fn set_flag_bit(this: MiPtr, set: bool, bit: u8) {
    let p = flags_lo(this);
    let b = p.read();
    p.write(if set { b | bit } else { b & !bit });
}

/// `CBaseModelInfo::SwaysInWind` (stub 0x4212C0 → .HOODLUM 0x156C780): 1 when
/// `(flags & 0x7800)` is 0x800 (TREE) or 0x1000 (PALM), else 0, in EAX.
#[hook(addr = 0x4212C0, conv = thiscall, name = "CBaseModelInfo::SwaysInWind", expect = "E9 BB B4 14 01", this = "CBaseModelInfo")]
pub fn sways_in_wind(this: MiPtr) -> u32 {
    let s = flags(this) & 0x7800;
    (s == 0x800 || s == 0x1000) as u32
}

/// `CBaseModelInfo::IsTagModel` (0x49CC20): `(flags & 0x7800) == 0x3000` as 0/1 in EAX.
#[hook(addr = 0x49CC20, conv = thiscall, name = "CBaseModelInfo::IsTagModel", expect = "33 C0 66 8B 41 12", this = "CBaseModelInfo")]
pub fn is_tag_model(this: MiPtr) -> u32 {
    (flags(this) & 0x7800 == 0x3000) as u32
}

/// `CBaseModelInfo::IsBreakableStatuePart` (0x59F090): `(flags & 0x7800) == 0x5800` as 0/1.
#[hook(addr = 0x59F090, conv = thiscall, name = "CBaseModelInfo::IsBreakableStatuePart", expect = "33 C0 66 8B 41 12", this = "CBaseModelInfo")]
pub fn is_breakable_statue_part(this: MiPtr) -> u32 {
    (flags(this) & 0x7800 == 0x5800) as u32
}

/// `CBaseModelInfo::SetHasBeenPreRendered(int)` (0x4C42F0, `ret 4`): bit 0 of the low flags byte
/// set when the dword argument is non-zero.
#[hook(addr = 0x4C42F0, conv = thiscall, name = "CBaseModelInfo::SetHasBeenPreRendered", expect = "8B 44 24 04 85 C0", this = "CBaseModelInfo")]
pub fn set_has_been_pre_rendered(this: MiPtr, v: u32) {
    set_flag_bit(this, v != 0, 0x01)
}

/// `CBaseModelInfo::IsLod` (0x4C4A00): `flags_lo & 0x80` in EAX.
#[hook(addr = 0x4C4A00, conv = thiscall, name = "CBaseModelInfo::IsLod", expect = "33 C0 8A 41 12", this = "CBaseModelInfo")]
pub fn is_lod(this: MiPtr) -> u32 {
    (flags_lo(this).read() & 0x80) as u32
}

/// `CBaseModelInfo::SetIsLod(bool)` (0x4C4A10, `ret 4`; the dword argument is tested whole).
#[hook(addr = 0x4C4A10, conv = thiscall, name = "CBaseModelInfo::SetIsLod", expect = "8B 44 24 04 85 C0", this = "CBaseModelInfo")]
pub fn set_is_lod(this: MiPtr, v: u32) {
    set_flag_bit(this, v != 0, 0x80)
}

/// `CBaseModelInfo::AsAtomicModelInfoPtr` (0x4C4A80): `xor eax, eax; ret` (unhookable: 3 bytes).
pub fn as_atomic_model_info_ptr(_this: MiPtr) -> MiPtr {
    GPtr::NULL
}

/// `CBaseModelInfo::AsDamageAtomicModelInfoPtr` (0x4C4A90): null (unhookable).
pub fn as_damage_atomic_model_info_ptr(_this: MiPtr) -> MiPtr {
    GPtr::NULL
}

/// `CBaseModelInfo::AsLodAtomicModelInfoPtr` (0x4C4AA0): null (unhookable).
pub fn as_lod_atomic_model_info_ptr(_this: MiPtr) -> MiPtr {
    GPtr::NULL
}

/// `CBaseModelInfo::GetTimeInfo` (0x4C4AB0): null (unhookable).
pub fn get_time_info(_this: MiPtr) -> GPtr<u8> {
    GPtr::NULL
}

/// `CBaseModelInfo::SetAnimFile` (0x4C4AC0): `ret 4`, nothing (unhookable).
pub fn set_anim_file(_this: MiPtr, _name: GPtr<u8>) {}

/// `CBaseModelInfo::ConvertAnimFileIndex` (0x4C4AD0): `ret`, nothing (unhookable).
pub fn convert_anim_file_index(_this: MiPtr) {}

/// `CBaseModelInfo::GetAnimFileIndex` (0x4C4AE0): -1 (unhookable: 4 bytes).
pub fn get_anim_file_index(_this: MiPtr) -> i32 {
    -1
}

/// `CBaseModelInfo::Init` (0x4C4B10). Store order of the original: ref count, txd, col model,
/// 2dfx index, 2dfx count, object info, draw distance 2000.0, rw object, flags = 0xC0.
#[hook(addr = 0x4C4B10, conv = thiscall, name = "CBaseModelInfo::Init", expect = "33 C0 83 CA FF", this = "CBaseModelInfo")]
pub fn init(this: MiPtr) {
    f::<u16>(this, mi::REF_COUNT).write(0);
    f::<i16>(this, mi::TXD_INDEX).write(-1);
    f::<u32>(this, mi::COL_MODEL).write(0);
    f::<i16>(this, mi::EFFECT_2D_INDEX).write(-1);
    f::<u8>(this, mi::NUM_2DFX).write(0);
    f::<i16>(this, mi::OBJECT_INFO_INDEX).write(-1);
    f::<u32>(this, mi::DRAW_DISTANCE).write(0x44FA_0000);
    f::<u32>(this, mi::RW_OBJECT).write(0);
    f::<u16>(this, mi::FLAGS).write(0xC0);
}

/// `CBaseModelInfo::ClearTexDictionary` (0x4C4B70): `m_nTxdIndex = -1`.
#[hook(addr = 0x4C4B70, conv = thiscall, name = "CBaseModelInfo::ClearTexDictionary", expect = "66 C7 41 0A FF FF", this = "CBaseModelInfo")]
pub fn clear_tex_dictionary(this: MiPtr) {
    f::<i16>(this, mi::TXD_INDEX).write(-1);
}

/// `CBaseModelInfo::SetColModel(CColModel*, bool)` (0x4C4BC0, `ret 8`). The owner gets the bit
/// (0x80 of the low flags byte) set; then each model of the time-info chain gets the same col model
/// with the bit cleared (loop, `owns` = 0 after the first model).
#[hook(addr = 0x4C4BC0, conv = thiscall, name = "CBaseModelInfo::SetColModel", expect = "8A 54 24 08 56", this = "CBaseModelInfo")]
pub fn set_col_model(this: MiPtr, col: GPtr<u8>, owns: u32) {
    let mut m = this;
    let mut owns = owns as u8;
    loop {
        f::<GPtr<u8>>(m, mi::COL_MODEL).write(col);
        if owns == 0 {
            set_flag_bit(m, false, 0x80);
            return;
        }
        set_flag_bit(m, true, 0x80);
        let ti = vcall_get_time_info(m);
        if ti.is_null() {
            return;
        }
        let other = ti.byte_add(2).cast::<i16>().read() as i32;
        if other == -1 {
            return;
        }
        m = model_info(other);
        owns = 0;
    }
}

/// `CBaseModelInfo::Init2dEffects` (0x4C4C20): index -1, count 0.
#[hook(addr = 0x4C4C20, conv = thiscall, name = "CBaseModelInfo::Init2dEffects", expect = "66 C7 41 0E FF FF", this = "CBaseModelInfo")]
pub fn init_2d_effects(this: MiPtr) {
    f::<i16>(this, mi::EFFECT_2D_INDEX).write(-1);
    f::<u8>(this, mi::NUM_2DFX).write(0);
}

/// `CBaseModelInfo::IsRoad` (0x4C4DF0): `flags & 0x100` in EAX.
#[hook(addr = 0x4C4DF0, conv = thiscall, name = "CBaseModelInfo::IsRoad", expect = "33 C0 66 8B 41 12", this = "CBaseModelInfo")]
pub fn is_road(this: MiPtr) -> u32 {
    (flags(this) & 0x100) as u32
}

/// `CBaseModelInfo::HasComplexHierarchy` (0x4C4E00): `flags & 0x200` in EAX.
#[hook(addr = 0x4C4E00, conv = thiscall, name = "CBaseModelInfo::HasComplexHierarchy", expect = "33 C0 66 8B 41 12", this = "CBaseModelInfo")]
pub fn has_complex_hierarchy(this: MiPtr) -> u32 {
    (flags(this) & 0x200) as u32
}

/// `CBaseModelInfo::HasBeenPreRendered` (0x5328B0): `flags_lo & 1`.
#[hook(addr = 0x5328B0, conv = thiscall, name = "CBaseModelInfo::HasBeenPreRendered", expect = "33 C0 8A 41 12", this = "CBaseModelInfo")]
pub fn has_been_pre_rendered(this: MiPtr) -> u32 {
    (flags_lo(this).read() & 0x01) as u32
}

/// `CBaseModelInfo::GetIsDrawLast` (0x5328C0): `flags_lo & 2`.
#[hook(addr = 0x5328C0, conv = thiscall, name = "CBaseModelInfo::GetIsDrawLast", expect = "33 C0 8A 41 12", this = "CBaseModelInfo")]
pub fn get_is_draw_last(this: MiPtr) -> u32 {
    (flags_lo(this).read() & 0x02) as u32
}

/// `CBaseModelInfo::SetOwnsColModel(bool)` (0x5328D0, `ret 4`): bit 0x20 of the low flags byte.
#[hook(addr = 0x5328D0, conv = thiscall, name = "CBaseModelInfo::SetOwnsColModel", expect = "8B 44 24 04 85 C0", this = "CBaseModelInfo")]
pub fn set_owns_col_model(this: MiPtr, v: u32) {
    set_flag_bit(this, v != 0, 0x20)
}

/// `CBaseModelInfo::IsBackfaceCulled` (0x5328F0): `flags_lo & 0x40`.
#[hook(addr = 0x5328F0, conv = thiscall, name = "CBaseModelInfo::IsBackfaceCulled", expect = "33 C0 8A 41 12", this = "CBaseModelInfo")]
pub fn is_backface_culled(this: MiPtr) -> u32 {
    (flags_lo(this).read() & 0x40) as u32
}

/// `CBaseModelInfo::IncreaseAlpha` (0x532900): `alpha < 0xEF` (unsigned) → `+0x10`, else 0xFF.
#[hook(addr = 0x532900, conv = thiscall, name = "CBaseModelInfo::IncreaseAlpha", expect = "8A 41 0C 3C EF", this = "CBaseModelInfo")]
pub fn increase_alpha(this: MiPtr) {
    let p = f::<u8>(this, mi::ALPHA);
    let a = p.read();
    p.write(if a < 0xEF { a + 0x10 } else { 0xFF });
}

/// The flag byte `CBaseModelInfo::SetBaseModelInfoFlags` computes from IDE `flags`.
pub fn base_model_info_flags_byte(b: u8, flags: u32) -> u8 {
    let mut b = b;
    let mut put = |cond: bool, bit: u8| b = if cond { b | bit } else { b & !bit };
    put(flags & 0x0C != 0, 0x02);
    put(flags & 0x08 != 0, 0x04);
    put(flags & 0x40 != 0, 0x08);
    put(flags & 0x80 != 0, 0x10);
    put((flags >> 21) & 1 == 0, 0x40);
    b
}

/// `CBaseModelInfo::SetBaseModelInfoFlags(CBaseModelInfo*, uint32)` (0x5B3AD0, cdecl): six
/// read-modify-writes of the low flags byte (draw last, additive, no z-write, no shadows, and
/// "backface culled" = IDE bit 21 clear).
#[hook(addr = 0x5B3AD0, conv = cdecl, name = "CBaseModelInfo::SetBaseModelInfoFlags", expect = "8B 44 24 04 8B 4C 24 08", preserve = "edx")]
pub fn set_base_model_info_flags(m: MiPtr, flags: u32) {
    let p = flags_lo(m);
    p.write(base_model_info_flags_byte(p.read(), flags));
}

/// `CBaseModelInfo`'s scalar deleting destructor (vtable slot 0, 0x4C4AF0, `ret 4`): vptr =
/// 0x85BB9C, `operator delete(this)` when `flags & 1`; returns `this`.
#[hook(addr = 0x4C4AF0, conv = thiscall, name = "CBaseModelInfo::`scalar deleting destructor'", expect = "F6 44 24 04 01", this = "CBaseModelInfo")]
pub fn scalar_deleting_destructor(this: MiPtr, flags: u32) -> MiPtr {
    f::<u32>(this, mi::VTABLE).write(VTBL_BASE_MODEL_INFO);
    if flags & 1 != 0 {
        operator_delete(this.cast());
    }
    this
}

/// `CBaseModelInfo::AddTexDictionaryRef` (0x4C4B80): `CTxdStore::AddRef(m_nTxdIndex)`.
#[hook(addr = 0x4C4B80, conv = thiscall, name = "CBaseModelInfo::AddTexDictionaryRef", expect = "0F BF 41 0A 50", this = "CBaseModelInfo")]
pub fn add_tex_dictionary_ref(this: MiPtr) {
    txd_add_ref(f::<i16>(this, mi::TXD_INDEX).read() as i32);
}

/// `CBaseModelInfo::RemoveTexDictionaryRef` (0x4C4B90): `CTxdStore::RemoveRef(m_nTxdIndex)`.
#[hook(addr = 0x4C4B90, conv = thiscall, name = "CBaseModelInfo::RemoveTexDictionaryRef", expect = "0F BF 41 0A 50", this = "CBaseModelInfo")]
pub fn remove_tex_dictionary_ref(this: MiPtr) {
    txd_remove_ref(f::<i16>(this, mi::TXD_INDEX).read() as i32);
}

/// `CBaseModelInfo::AddRef` (0x4C4BA0): `++m_nRefCount` (16-bit), then the txd's AddRef.
#[hook(addr = 0x4C4BA0, conv = thiscall, name = "CBaseModelInfo::AddRef", expect = "0F BF 41 0A 66 FF 41 08", this = "CBaseModelInfo")]
pub fn add_ref(this: MiPtr) {
    let txd = f::<i16>(this, mi::TXD_INDEX).read() as i32;
    let p = f::<u16>(this, mi::REF_COUNT);
    p.write(p.read().wrapping_add(1));
    txd_add_ref(txd);
}

/// `CBaseModelInfo::RemoveRef` (0x4C4BB0): `--m_nRefCount`, then the txd's RemoveRef.
#[hook(addr = 0x4C4BB0, conv = thiscall, name = "CBaseModelInfo::RemoveRef", expect = "0F BF 41 0A 66 FF 49 08", this = "CBaseModelInfo")]
pub fn remove_ref(this: MiPtr) {
    let txd = f::<i16>(this, mi::TXD_INDEX).read() as i32;
    let p = f::<u16>(this, mi::REF_COUNT);
    p.write(p.read().wrapping_sub(1));
    txd_remove_ref(txd);
}

/// `CBaseModelInfo::SetTexDictionary(const char*)` (0x4C4B40, `ret 4`): `FindTxdSlot`; when its
/// low 16 bits are 0xFFFF, `AddTxdSlot`. Both results are stored (16 bits).
#[hook(addr = 0x4C4B40, conv = thiscall, name = "CBaseModelInfo::SetTexDictionary", expect = "56 57 8B 7C 24 0C", this = "CBaseModelInfo")]
pub fn set_tex_dictionary(this: MiPtr, name: GPtr<u8>) {
    let r = txd_find_txd_slot(name) as u16;
    f::<u16>(this, mi::TXD_INDEX).write(r);
    if r == 0xFFFF {
        let r = txd_add_txd_slot(name) as u16;
        f::<u16>(this, mi::TXD_INDEX).write(r);
    }
}

/// `CBaseModelInfo::Add2dEffect(C2dEffect*)` (0x4C4D20, `ret 4`): the first effect sets the index
/// (`(effect - store - 4) >> 6`, signed) and count 1; later ones increment the count (8-bit).
#[hook(addr = 0x4C4D20, conv = thiscall, name = "CBaseModelInfo::Add2dEffect", expect = "56 8B F1 66 83 7E 0E 00", this = "CBaseModelInfo")]
pub fn add_2d_effect(this: MiPtr, effect: GPtr<u8>) {
    add_2d_effect_with(this, effect.addr() as u32, || get_2d_effect_store_call().addr() as u32)
}

/// [`add_2d_effect`] on 32-bit addresses, the store address from `store` (called only for the
/// first effect, as the original calls `Get2dEffectStore`).
pub fn add_2d_effect_with(this: MiPtr, effect: u32, store: impl FnOnce() -> u32) {
    if f::<i16>(this, mi::EFFECT_2D_INDEX).read() >= 0 {
        add_2dfx_count(this, 1, false);
        return;
    }
    let store = store();
    let idx = (effect.wrapping_sub(store).wrapping_sub(4) as i32) >> 6;
    f::<u16>(this, mi::EFFECT_2D_INDEX).write(idx as u16);
    f::<u8>(this, mi::NUM_2DFX).write(1);
}

/// `CBaseModelInfo::Get2dEffect(int32)` (0x4C4C70, `ret 4`): effects of the model's geometry come
/// after the store's: `n = count - geometry effects`; `index >= n` (signed) → the geometry's
/// effect `index - n` (`*(geometry + plugin) + 4 + 64 * k`), else the store's `index + m_n2dEffectIndex`.
/// The geometry is the atomic's (rw type 1) or `Get2DEffectAtomic(clump)`'s (rw type 2); with no
/// geometry the original reads `*(0 + plugin offset)`, which the port reproduces.
#[hook(addr = 0x4C4C70, conv = thiscall, name = "CBaseModelInfo::Get2dEffect", expect = "53 55 8B D9 8B 03", this = "CBaseModelInfo")]
pub fn get_2d_effect(this: MiPtr, index: i32) -> GPtr<u8> {
    let mut n = f::<u8>(this, mi::NUM_2DFX).read() as i32;
    let mut geometry = GPtr::<u8>::NULL;
    let rw = || f::<GPtr<u8>>(this, mi::RW_OBJECT).read();
    let mut found = false;
    if vcall_get_rw_model_type(this) == 1 && !rw().is_null() {
        geometry = rw().byte_add(0x18).cast::<GPtr<u8>>().read();
        found = true;
    } else if vcall_get_rw_model_type(this) == 2 && !rw().is_null() {
        let a = get_2d_effect_atomic(rw());
        if !a.is_null() {
            geometry = a.byte_add(0x18).cast::<GPtr<u8>>().read();
            found = true;
        }
    }
    if found {
        n = n.wrapping_sub(geometry_2dfx_count(geometry, G2D_EFFECT_PLUGIN_OFFSET.get()) as i32);
    }
    if index >= n {
        let data = geometry
            .byte_add(G2D_EFFECT_PLUGIN_OFFSET.get() as isize)
            .cast::<GPtr<u8>>()
            .read();
        let k = (index.wrapping_sub(n) as u32) << 6;
        return data.byte_add(k as i32 as isize + 4);
    }
    let store = get_2d_effect_store_call();
    let k = ((f::<i16>(this, mi::EFFECT_2D_INDEX).read() as i32).wrapping_add(index) as u32) << 6;
    store.byte_add(k as i32 as isize + 4)
}

/// Deletes the col model when this model owns it (bit 0x80 of the low flags byte): the
/// original's inlined `delete m_pColModel` (destructor + `CColModel::operator delete`).
fn delete_owned_col_model(this: MiPtr) {
    let col = f::<GPtr<u8>>(this, mi::COL_MODEL).read();
    if !col.is_null() && flags_lo(this).read() & 0x80 != 0 {
        col_model_destructor(col);
        col_model_operator_delete(col);
    }
}

/// `CBaseModelInfo::DeleteCollisionModel` (0x4C4C40).
#[hook(addr = 0x4C4C40, conv = thiscall, name = "CBaseModelInfo::DeleteCollisionModel", expect = "56 8B F1 57 8B 7E 14", this = "CBaseModelInfo")]
pub fn delete_collision_model(this: MiPtr) {
    delete_owned_col_model(this);
    f::<u32>(this, mi::COL_MODEL).write(0);
}

/// `CBaseModelInfo::Shutdown` (0x4C4D50): virtual `DeleteRwObject`, the owned col model, then
/// flags |= 0x80 (low byte), col = 0, 2dfx index -1, 2dfx count 0, object info -1, txd -1.
#[hook(addr = 0x4C4D50, conv = thiscall, name = "CBaseModelInfo::Shutdown", expect = "56 8B F1 8B 06", this = "CBaseModelInfo")]
pub fn shutdown(this: MiPtr) {
    vcall_delete_rw_object(this);
    delete_owned_col_model(this);
    set_flag_bit(this, true, 0x80);
    f::<u32>(this, mi::COL_MODEL).write(0);
    f::<i16>(this, mi::EFFECT_2D_INDEX).write(-1);
    f::<u8>(this, mi::NUM_2DFX).write(0);
    f::<i16>(this, mi::OBJECT_INFO_INDEX).write(-1);
    f::<i16>(this, mi::TXD_INDEX).write(-1);
}

/// `CBaseModelInfo::Shutdown` virtual-call thunk (0x4C6200, catalogue `FUN_004c6200`):
/// `mov eax, [ecx]; jmp [eax + 0x1C]`.
#[hook(addr = 0x4C6200, conv = thiscall, name = "FUN_004c6200", expect = "8B 01 FF 60 1C", this = "CBaseModelInfo")]
pub fn vcall_shutdown_thunk(this: MiPtr) {
    vcall_shutdown(this)
}

/// Every `#[hook]` of this module, in source order.
pub const HOOKS: &[HookReg] = &[
    HOOK_SWAYS_IN_WIND,
    HOOK_IS_TAG_MODEL,
    HOOK_IS_BREAKABLE_STATUE_PART,
    HOOK_SET_HAS_BEEN_PRE_RENDERED,
    HOOK_IS_LOD,
    HOOK_SET_IS_LOD,
    HOOK_INIT,
    HOOK_CLEAR_TEX_DICTIONARY,
    HOOK_SET_COL_MODEL,
    HOOK_INIT_2D_EFFECTS,
    HOOK_IS_ROAD,
    HOOK_HAS_COMPLEX_HIERARCHY,
    HOOK_HAS_BEEN_PRE_RENDERED,
    HOOK_GET_IS_DRAW_LAST,
    HOOK_SET_OWNS_COL_MODEL,
    HOOK_IS_BACKFACE_CULLED,
    HOOK_INCREASE_ALPHA,
    HOOK_SET_BASE_MODEL_INFO_FLAGS,
    HOOK_SCALAR_DELETING_DESTRUCTOR,
    HOOK_ADD_TEX_DICTIONARY_REF,
    HOOK_REMOVE_TEX_DICTIONARY_REF,
    HOOK_ADD_REF,
    HOOK_REMOVE_REF,
    HOOK_SET_TEX_DICTIONARY,
    HOOK_ADD_2D_EFFECT,
    HOOK_GET_2D_EFFECT,
    HOOK_DELETE_COLLISION_MODEL,
    HOOK_SHUTDOWN,
    HOOK_VCALL_SHUTDOWN_THUNK,
];
