//! ANIM: `CAnimBlendHierarchy` (PORT-ANIM-INPUT). Ported from the 1.0 US bytes (gta-reversed
//! `AnimBlendHierarchy.cpp` as a reading aid, `docs/re/specs/ANIM/CAnimBlendHierarchy.deviations.md`).
//! See `docs/arch/anim-input-port.md`.
//!
//! Layout (0x18): +0x0 `m_hashKey`, +0x4 `m_pSequences` (`CAnimBlendSequence[]` from `new[]`, count
//! at [-4]), +0x8 `m_nSeqCount` (int16), +0xA `m_bIsCompressed`, +0xB `m_bKeepCompressed`, +0xC
//! `m_nAnimBlockId`, +0x10 `m_fTotalTime`, +0x14 `m_Link`.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::orig;
use sa_math::x87::{Cmp, X87};

use crate::common::{rd, wr, x};
use crate::sequence::{
    self, FLAG_EXTERNAL, FLAG_TRANS, call_get_data_size, call_move_memory, crt_free, get_uppercase_key,
    mem_free, mem_malloc, mem_move_memory,
};

sa_core::globals! {
    list GLOBALS;
}

orig! {
    /// CRT `eh vector destructor iterator` (0x821E02, stdcall)
    pub fn eh_vector_dtor(p: GPtr<u8>, size: u32, n: u32, dtor: u32) = stdcall 0x821E02 => unported;
    /// `CAnimManager::RemoveFromUncompressedCache(CAnimBlendHierarchy*)` (0x4D42A0)
    pub fn remove_from_uncompressed_cache(h: GPtr<u8>) = cdecl 0x4D42A0 => crate::manager::remove_from_uncompressed_cache;
    /// `CAnimManager::UncompressAnimation(CAnimBlendHierarchy*)` (0x4D41C0)
    pub fn uncompress_animation(h: GPtr<u8>) = cdecl 0x4D41C0 => crate::manager::uncompress_animation;
    /// `CAnimBlendHierarchy::AllocSequenceBlock(bool)` (0x4CF510)
    pub fn call_alloc_sequence_block(this: GPtr<u8>, compressed: u8) -> GPtr<u8> = thiscall 0x4CF510 => alloc_sequence_block;
    /// `CAnimBlendHierarchy::CalcTotalTime()` (0x4CF2F0)
    pub fn call_calc_total_time(this: GPtr<u8>) = thiscall 0x4CF2F0 => calc_total_time;
    /// `CAnimBlendHierarchy::CalcTotalTimeCompressed()` (0x4CF3E0)
    pub fn call_calc_total_time_compressed(this: GPtr<u8>) = thiscall 0x4CF3E0 => calc_total_time_compressed;
    /// `CAnimBlendHierarchy::RemoveAnimSequences()` (0x4CF8E0)
    pub fn call_remove_anim_sequences(this: GPtr<u8>) = thiscall 0x4CF8E0 => remove_anim_sequences;
    /// `CAnimBlendHierarchy::Uncompress()` (0x4CF5F0)
    pub fn call_uncompress(this: GPtr<u8>) = thiscall 0x4CF5F0 => uncompress;
    /// `CAnimBlendHierarchy::RemoveUncompressedData()` (0x4CF760)
    pub fn call_remove_uncompressed_data(this: GPtr<u8>) = thiscall 0x4CF760 => remove_uncompressed_data;
    /// `CAnimBlendHierarchy::RemoveQuaternionFlips()` (0x4CF4E0)
    pub fn call_remove_quaternion_flips(this: GPtr<u8>) = thiscall 0x4CF4E0 => remove_quaternion_flips;
    /// `CAnimBlendHierarchy::FindSequence(const char*)` (0x4CF290)
    pub fn call_find_sequence(this: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = thiscall 0x4CF290 => find_sequence;
    /// `CAnimBlendHierarchy::SetName(const char*)` (0x4CF2D0)
    pub fn call_set_name(this: GPtr<u8>, name: GPtr<u8>) = thiscall 0x4CF2D0 => set_name;
    /// `CAnimBlendHierarchy::CAnimBlendHierarchy()` (0x4CF270)
    pub fn call_constructor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4CF270 => constructor;
    /// `CAnimBlendHierarchy::Shutdown()` (0x4CF980)
    pub fn call_shutdown(this: GPtr<u8>) = thiscall 0x4CF980 => shutdown;
    /// `CAnimBlendSequence::Print()` (0x4D1180)
    pub fn sequence_print(this: GPtr<u8>) = thiscall 0x4D1180 => sequence::print;
    /// `CAnimBlendSequence::RemoveUncompressedData(void*)` (0x4D12A0)
    pub fn sequence_remove_uncompressed_data(this: GPtr<u8>, b: GPtr<u8>) = thiscall 0x4D12A0 => sequence::remove_uncompressed_data;
}

#[inline]
pub fn seqs(h: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(h, 4)
}
#[inline]
pub fn seq_count(h: GPtr<u8>) -> i16 {
    rd::<i16>(h, 8)
}
#[inline]
fn seq(h: GPtr<u8>, i: i32) -> GPtr<u8> {
    seqs(h).byte_add(i as isize * 0xC)
}

/// The inlined `GetSequenceBlock`: the first sequence's frames when it uses external memory, else 0.
fn sequence_block(h: GPtr<u8>) -> GPtr<u8> {
    let s0 = seqs(h);
    if sequence::flags(s0) & FLAG_EXTERNAL != 0 { sequence::frames(s0) } else { GPtr::NULL }
}

/// `CAnimBlendHierarchy::CAnimBlendHierarchy()` (0x4CF270): every field but the hash key; returns `this`.
#[hook(addr = 0x4CF270, conv = thiscall, name = "CAnimBlendHierarchy::Constructor", expect = "8B C1 33 C9 89 48 04", this = "CAnimBlendHierarchy")]
pub fn constructor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 4, 0);
    wr::<u16>(this, 8, 0);
    wr::<u8>(this, 0xA, 0);
    wr::<u8>(this, 0xB, 0);
    wr::<i32>(this, 0xC, -1);
    wr::<u32>(this, 0x10, 0);
    wr::<u32>(this, 0x14, 0);
    this
}

/// `CAnimBlendHierarchy::FindSequence(const char*)` (0x4CF290): the sequence whose hash equals
/// `CKeyGen::GetUppercaseKey(name)`, else null.
#[hook(addr = 0x4CF290, conv = thiscall, name = "CAnimBlendHierarchy::FindSequence", expect = "8B 44 24 04 56", this = "CAnimBlendHierarchy")]
pub fn find_sequence(this: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> {
    let key = get_uppercase_key(name);
    let n = i32::from(seq_count(this));
    let mut i = 0;
    while i < n {
        if rd::<u32>(seq(this, i), 0) == key {
            return seq(this, i);
        }
        i += 1;
    }
    GPtr::NULL
}

/// `CAnimBlendHierarchy::SetName(const char*)` (0x4CF2D0): hash = `CKeyGen::GetUppercaseKey(name)`.
#[hook(addr = 0x4CF2D0, conv = thiscall, name = "CAnimBlendHierarchy::SetName", expect = "8B 44 24 04 56", this = "CAnimBlendHierarchy")]
pub fn set_name(this: GPtr<u8>, name: GPtr<u8>) {
    let h = get_uppercase_key(name);
    wr::<u32>(this, 0, h);
}

/// `CAnimBlendHierarchy::CalcTotalTime()` (0x4CF2F0): total = the largest last-frame time (`fcom`,
/// stored when greater), then every sequence's times become deltas (`t[j] -= t[j-1]`, last to first).
#[hook(addr = 0x4CF2F0, conv = thiscall, name = "CAnimBlendHierarchy::CalcTotalTime", expect = "83 EC 10 33 C0", this = "CAnimBlendHierarchy")]
pub fn calc_total_time(this: GPtr<u8>) {
    wr::<u32>(this, 0x10, 0);
    let mut i = 0i32;
    while i < i32::from(seq_count(this)) {
        let s = seq(this, i);
        let n = i32::from(sequence::num_frames(s));
        let stride: isize = if sequence::flags(s) & FLAG_TRANS != 0 { 0x20 } else { 0x14 };
        let last = sequence::frames(s).byte_add((n - 1) as isize * stride);
        let t = rd::<f32>(last, 0x10);
        if x(t).fcom(x(rd::<f32>(this, 0x10))) == Cmp::Greater {
            wr::<f32>(this, 0x10, t);
        }
        let mut j = n - 1;
        while j >= 1 {
            let stride: isize = if sequence::flags(s) & FLAG_TRANS != 0 { 0x20 } else { 0x14 };
            let f = sequence::frames(s);
            let cur = f.byte_add(j as isize * stride);
            let prev = f.byte_add((j - 1) as isize * stride);
            let d = x(rd::<f32>(cur, 0x10)).fsub(x(rd::<f32>(prev, 0x10))).to_f32();
            wr::<f32>(cur, 0x10, d);
            j -= 1;
        }
        i += 1;
    }
}

/// `CAnimBlendHierarchy::CalcTotalTimeCompressed()` (0x4CF3E0): as `CalcTotalTime` on the int16
/// times (`fild t * (1/60)` for the total; 16-bit deltas).
#[hook(addr = 0x4CF3E0, conv = thiscall, name = "CAnimBlendHierarchy::CalcTotalTimeCompressed", expect = "83 EC 10 33 C0", this = "CAnimBlendHierarchy")]
pub fn calc_total_time_compressed(this: GPtr<u8>) {
    wr::<u32>(this, 0x10, 0);
    let mut i = 0i32;
    while i < i32::from(seq_count(this)) {
        let s = seq(this, i);
        let n = i32::from(sequence::num_frames(s));
        let stride: isize = if sequence::flags(s) & FLAG_TRANS != 0 { 0x10 } else { 0xA };
        let last = sequence::frames(s).byte_add((n - 1) as isize * stride);
        let t = X87::from_i32(i32::from(rd::<i16>(last, 8))).fmul_m(sequence::k_inv60());
        if t.fcom(x(rd::<f32>(this, 0x10))) == Cmp::Greater {
            wr::<f32>(this, 0x10, t.to_f32());
        }
        let mut j = n - 1;
        while j >= 1 {
            let stride: isize = if sequence::flags(s) & FLAG_TRANS != 0 { 0x10 } else { 0xA };
            let f = sequence::frames(s);
            let cur = f.byte_add(j as isize * stride);
            let prev = f.byte_add((j - 1) as isize * stride);
            wr::<i16>(cur, 8, rd::<i16>(cur, 8).wrapping_sub(rd::<i16>(prev, 8)));
            j -= 1;
        }
        i += 1;
    }
}

/// `CAnimBlendHierarchy::RemoveQuaternionFlips()` (0x4CF4E0): every sequence's.
#[hook(addr = 0x4CF4E0, conv = thiscall, name = "CAnimBlendHierarchy::RemoveQuaternionFlips", expect = "56 57 8B F1 33 FF", this = "CAnimBlendHierarchy")]
pub fn remove_quaternion_flips(this: GPtr<u8>) {
    let mut i = 0;
    while i < i32::from(seq_count(this)) {
        sequence::call_remove_quaternion_flips(seq(this, i));
        i += 1;
    }
}

/// `CAnimBlendHierarchy::AllocSequenceBlock(bool compressed)` (0x4CF510): `CMemoryMgr::Malloc` of
/// the sum of the sequences' `GetDataSize(compressed)`.
#[hook(addr = 0x4CF510, conv = thiscall, name = "CAnimBlendHierarchy::AllocSequenceBlock", expect = "55 56 57 8B F1", this = "CAnimBlendHierarchy")]
pub fn alloc_sequence_block(this: GPtr<u8>, compressed: u8) -> GPtr<u8> {
    let mut size = 0u32;
    let mut i = 0;
    while i < i32::from(seq_count(this)) {
        size = size.wrapping_add(call_get_data_size(seq(this, i), compressed));
        i += 1;
    }
    mem_malloc(size, 0)
}

/// `CAnimBlendHierarchy::GetSequenceBlock()` (0x4CF560): the first sequence's frames when it uses
/// external memory, else 0.
#[hook(addr = 0x4CF560, conv = thiscall, name = "CAnimBlendHierarchy::GetSequenceBlock", expect = "8B 41 04 66 8B 48 04", this = "CAnimBlendHierarchy")]
pub fn get_sequence_block(this: GPtr<u8>) -> GPtr<u8> {
    sequence_block(this)
}

/// `CAnimBlendHierarchy::Uncompress()` (0x4CF5F0): every sequence into one new block
/// (`AllocSequenceBlock(false)`), the old block freed, `m_bIsCompressed = 0`; when the total time is
/// exactly 0: `RemoveQuaternionFlips` of each sequence and `CalcTotalTime` (tail jump).
#[hook(addr = 0x4CF5F0, conv = thiscall, name = "CAnimBlendHierarchy::Uncompress", expect = "51 53 55 56 57", this = "CAnimBlendHierarchy")]
pub fn uncompress(this: GPtr<u8>) {
    let mut block = call_alloc_sequence_block(this, 0);
    let old = sequence_block(this);
    let mut i = 0;
    while i < i32::from(seq_count(this)) {
        sequence::call_uncompress(seq(this, i), block);
        block = block.byte_add(call_get_data_size(seq(this, i), 0) as i32 as isize);
        i += 1;
    }
    if !old.is_null() {
        mem_free(old);
    }
    wr::<u8>(this, 0xA, 0);
    if x(rd::<f32>(this, 0x10)).fcom(X87::zero()).test_parity(0x44) {
        return;
    }
    let mut i = 0;
    while i < i32::from(seq_count(this)) {
        sequence::call_remove_quaternion_flips(seq(this, i));
        i += 1;
    }
    call_calc_total_time(this);
}

/// `CAnimBlendHierarchy::CompressKeyframes()` (0x4CF6C0): every sequence into one new block
/// (`AllocSequenceBlock(true)`), the old block freed (the compressed flag is not written).
#[hook(addr = 0x4CF6C0, conv = thiscall, name = "CAnimBlendHierarchy::CompressKeyframes", expect = "51 53 55 56 6A 01", this = "CAnimBlendHierarchy")]
pub fn compress_keyframes(this: GPtr<u8>) {
    let mut block = call_alloc_sequence_block(this, 1);
    let old = sequence_block(this);
    let mut i = 0;
    while i < i32::from(seq_count(this)) {
        sequence::call_compress_keyframes(seq(this, i), block);
        block = block.byte_add(call_get_data_size(seq(this, i), 1) as i32 as isize);
        i += 1;
    }
    if !old.is_null() {
        mem_free(old);
    }
}

/// `CAnimBlendHierarchy::RemoveUncompressedData()` (0x4CF760): as `CompressKeyframes` through each
/// sequence's `RemoveUncompressedData`, then `m_bIsCompressed = 1`.
#[hook(addr = 0x4CF760, conv = thiscall, name = "CAnimBlendHierarchy::RemoveUncompressedData", expect = "51 53 55 56 6A 01", this = "CAnimBlendHierarchy")]
pub fn remove_uncompressed_data(this: GPtr<u8>) {
    let mut block = call_alloc_sequence_block(this, 1);
    let old = sequence_block(this);
    let mut i = 0;
    while i < i32::from(seq_count(this)) {
        sequence_remove_uncompressed_data(seq(this, i), block);
        block = block.byte_add(call_get_data_size(seq(this, i), 1) as i32 as isize);
        i += 1;
    }
    if !old.is_null() {
        mem_free(old);
    }
    wr::<u8>(this, 0xA, 1);
}

/// `CAnimBlendHierarchy::MoveMemory()` (0x4CF800, gta-reversed: unreachable): shared external block:
/// moved by `CMemoryMgr::MoveMemory`, every sequence's frames rebased; own blocks: each sequence's
/// `MoveMemory`. AL = anything moved.
#[hook(addr = 0x4CF800, conv = thiscall, name = "CAnimBlendHierarchy::MoveMemory", expect = "53 56 8B F1 8B 46 04", this = "CAnimBlendHierarchy")]
pub fn move_memory(this: GPtr<u8>) -> u8 {
    let s0 = seqs(this);
    if s0.is_null() {
        return 0;
    }
    let mut moved = 0u8;
    if sequence::flags(s0) & FLAG_EXTERNAL != 0 {
        let old = sequence::frames(s0);
        if old.is_null() {
            return 0;
        }
        let new = mem_move_memory(old);
        if new.addr() == old.addr() {
            return 0;
        }
        let d = (new.addr() as u32).wrapping_sub(old.addr() as u32);
        let mut i = 0;
        while i < i32::from(seq_count(this)) {
            let p = seq(this, i);
            wr::<u32>(p, 8, rd::<u32>(p, 8).wrapping_add(d));
            i += 1;
        }
        return 1;
    }
    let mut i = 0;
    while i < i32::from(seq_count(this)) {
        if call_move_memory(seq(this, i)) != 0 {
            moved = 1;
        }
        i += 1;
    }
    moved
}

/// `CAnimBlendHierarchy::Print()` (0x4CF8A0): `UncompressAnimation(this)`, then each sequence's
/// (empty) `Print`.
#[hook(addr = 0x4CF8A0, conv = thiscall, name = "CAnimBlendHierarchy::Print", expect = "56 8B F1 57 56", this = "CAnimBlendHierarchy")]
pub fn print(this: GPtr<u8>) {
    uncompress_animation(this);
    let mut i = 0;
    while i < i32::from(seq_count(this)) {
        sequence_print(seq(this, i));
        i += 1;
    }
}

/// `CAnimBlendHierarchy::RemoveAnimSequences()` (0x4CF8E0): out of the uncompressed cache, the
/// sequence array destroyed (`delete[]`), fields cleared, the shared block freed.
#[hook(addr = 0x4CF8E0, conv = thiscall, name = "CAnimBlendHierarchy::RemoveAnimSequences", expect = "56 57 8B F9 8B 47 04", this = "CAnimBlendHierarchy")]
pub fn remove_anim_sequences(this: GPtr<u8>) {
    let s0 = seqs(this);
    let block = if s0.is_null() { GPtr::NULL } else { sequence_block(this) };
    remove_from_uncompressed_cache(this);
    let s = seqs(this);
    if !s.is_null() {
        let n = rd::<u32>(s, -4);
        eh_vector_dtor(s, 0xC, n, 0x4D0C30);
        crt_free(s.byte_add(-4));
    }
    wr::<u32>(this, 4, 0);
    wr::<u16>(this, 8, 0);
    wr::<u32>(this, 0x10, 0);
    if !block.is_null() {
        mem_free(block);
    }
}

/// `CAnimBlendHierarchy::~CAnimBlendHierarchy()` (0x4CF970): `jmp RemoveAnimSequences`.
#[hook(addr = 0x4CF970, conv = thiscall, name = "CAnimBlendHierarchy::Destructor", expect = "E9 6B FF FF FF 90", this = "CAnimBlendHierarchy")]
pub fn destructor(this: GPtr<u8>) {
    call_remove_anim_sequences(this);
}

/// `CAnimBlendHierarchy::Shutdown()` (0x4CF980): `RemoveAnimSequences`, `m_bIsCompressed = 0`.
#[hook(addr = 0x4CF980, conv = thiscall, name = "CAnimBlendHierarchy::Shutdown", expect = "56 8B F1 E8 58 FF FF FF", this = "CAnimBlendHierarchy")]
pub fn shutdown(this: GPtr<u8>) {
    call_remove_anim_sequences(this);
    wr::<u8>(this, 0xA, 0);
}

/// `FUN_004CFA30` = `FixedQuat<int16, 4096>` from a `CQuaternion` (cdecl `(int16* out, const float* q)`):
/// `out[k] = (int16)_ftol(q[k] * 4096.0f)`, k = x, y, z, w.
#[hook(addr = 0x4CFA30, conv = cdecl, name = "FUN_004cfa30", expect = "56 8B 74 24 0C")]
pub fn compress_quaternion(out: GPtr<u8>, q: GPtr<u8>) {
    for k in 0..4 {
        let v = crate::common::ftol32(x(rd::<f32>(q, 4 * k)).fmul_m(4096.0)) as i16;
        wr::<i16>(out, 2 * k, v);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_CONSTRUCTOR,
    HOOK_FIND_SEQUENCE,
    HOOK_SET_NAME,
    HOOK_CALC_TOTAL_TIME,
    HOOK_CALC_TOTAL_TIME_COMPRESSED,
    HOOK_REMOVE_QUATERNION_FLIPS,
    HOOK_ALLOC_SEQUENCE_BLOCK,
    HOOK_GET_SEQUENCE_BLOCK,
    HOOK_UNCOMPRESS,
    HOOK_COMPRESS_KEYFRAMES,
    HOOK_REMOVE_UNCOMPRESSED_DATA,
    HOOK_MOVE_MEMORY,
    HOOK_PRINT,
    HOOK_REMOVE_ANIM_SEQUENCES,
    HOOK_DESTRUCTOR,
    HOOK_SHUTDOWN,
    HOOK_COMPRESS_QUATERNION,
];
