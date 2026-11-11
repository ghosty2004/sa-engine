//! ANIM: `CAnimBlendSequence` (PORT-ANIM-INPUT), plus the `orig!` declarations the blend modules
//! share. Ported from the 1.0 US bytes (gta-reversed `AnimBlendSequence.cpp` as a reading aid,
//! `docs/re/specs/ANIM/CAnimBlendSequence.deviations.md`). See `docs/arch/anim-input-port.md`.
//!
//! Layout (0xC): +0x0 `m_NameHashKey` / bone tag (int16 in the low word), +0x4 flags byte (bit0
//! HasRotation, bit1 HasTranslation, bit2 IsCompressed, bit3 UsingExternalMemory, bit4
//! IsUsingBoneTag), +0x6 `m_FramesNum` (int16), +0x8 `m_Frames`.
//! Key frames: uncompressed `KeyFrame` 0x14 (quat f32[4] +0, delta time +0x10), `KeyFrameTrans` 0x20
//! (+ translation f32[3] at +0x14); compressed `KeyFrameCompressed` 0xA (quat int16[4] scaled by 4096,
//! time int16 in 1/60 s at +8), `KeyFrameTransCompressed` 0x10 (+ translation int16[3] scaled by 1024
//! at +0xA).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::orig;
use sa_math::x87::X87;

use crate::common::{at, ftol32, rd, wr, x};

sa_core::globals! {
    list GLOBALS;
}

orig! {
    /// `CMemoryMgr::Malloc(size)` (0x72F420); the blend code pushes a second word (0, RwMalloc's hint)
    pub fn mem_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F420 => unported;
    /// `CMemoryMgr::Free(p)` (0x72F430)
    pub fn mem_free(p: GPtr<u8>) = cdecl 0x72F430 => unported;
    /// `CMemoryMgr::MoveMemory(p)` (0x72F500; returns its argument in this build)
    pub fn mem_move_memory(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x72F500 => unported;
    /// CRT `operator new` (0x82119A)
    pub fn operator_new(n: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// CRT `operator new[]` (0x821195)
    pub fn operator_new_array(n: u32) -> GPtr<u8> = cdecl 0x821195 => unported;
    /// CRT `operator delete` (0x8213AE)
    pub fn operator_delete(p: GPtr<u8>) = cdecl 0x8213AE => unported;
    /// CRT `free` (0x8214BD, `delete[]` of a vector-new block)
    pub fn crt_free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// `CKeyGen::GetUppercaseKey(const char*)` (0x53CF30)
    pub fn get_uppercase_key(s: GPtr<u8>) -> u32 = cdecl 0x53CF30 => unported;
    /// `CKeyGen::GetKey(const char*)` (0x53CED0)
    pub fn get_key(s: GPtr<u8>) -> u32 = cdecl 0x53CED0 => unported;
    /// CRT `_stricmp` (0x8229B6)
    pub fn stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// CRT `strncpy` (0x8245A0)
    pub fn strncpy(d: GPtr<u8>, s: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x8245A0 => unported;
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall)
    pub fn eh_vector_ctor(p: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
}

pub const FLAG_ROT: u8 = 1;
pub const FLAG_TRANS: u8 = 2;
pub const FLAG_COMPRESSED: u8 = 4;
pub const FLAG_EXTERNAL: u8 = 8;
pub const FLAG_BONE_TAG: u8 = 0x10;

/// 1/4096 (0x858BFC)
pub const K_INV4096: f32 = f32::from_bits(0x3980_0000);
/// 1/60 (0x859044, 0x3C888889)
pub fn k_inv60() -> f32 {
    f32::from_bits(0x3C88_8889)
}
/// 1/1024 (0x85C6D4)
pub const K_INV1024: f32 = f32::from_bits(0x3A80_0000);

#[inline]
pub fn flags(s: GPtr<u8>) -> u8 {
    rd::<u8>(s, 4)
}
#[inline]
pub fn set_flags(s: GPtr<u8>, f: u8) {
    wr::<u8>(s, 4, f)
}
#[inline]
pub fn num_frames(s: GPtr<u8>) -> i16 {
    rd::<i16>(s, 6)
}
#[inline]
pub fn frames(s: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(s, 8)
}

/// `fild` of an int16 times a constant, stored as f32 (the `Uncompress` conversions).
#[inline]
fn unq(v: i16, k: f32) -> f32 {
    X87::from_i32(i32::from(v)).fmul_m(k).to_f32()
}

/// `_ftol(f * k)` low word (the `CompressKeyframes` conversions).
#[inline]
fn q(f: f32, k: f32) -> i16 {
    ftol32(x(f).fmul_m(k)) as i16
}

/// `CAnimBlendSequence::GetUKeyFrame(int32 n)` (0x4CF1F0): `m_Frames + n * (trans ? 0x20 : 0x14)`.
#[hook(addr = 0x4CF1F0, conv = thiscall, name = "CAnimBlendSequence::GetUKeyFrame", expect = "8A 41 04 D0 E8", this = "CAnimBlendSequence")]
pub fn get_u_key_frame(this: GPtr<u8>, n: i32) -> GPtr<u8> {
    let stride = if flags(this) & FLAG_TRANS != 0 { 0x20 } else { 0x14 };
    frames(this).byte_add((n as isize).wrapping_mul(stride))
}

/// `CAnimBlendSequence::GetCKeyFrame(int32 n)` (0x4CF220): `m_Frames + n * (trans ? 0x10 : 0xA)`.
#[hook(addr = 0x4CF220, conv = thiscall, name = "CAnimBlendSequence::GetCKeyFrame", expect = "8A 41 04 D0 E8", this = "CAnimBlendSequence")]
pub fn get_c_key_frame(this: GPtr<u8>, n: i32) -> GPtr<u8> {
    let stride = if flags(this) & FLAG_TRANS != 0 { 0x10 } else { 0xA };
    frames(this).byte_add((n as isize).wrapping_mul(stride))
}

/// `CAnimBlendSequence::CAnimBlendSequence()` (0x4D0C10): flags word, count, frames 0, tag word -1;
/// returns `this`.
#[hook(addr = 0x4D0C10, conv = thiscall, name = "CAnimBlendSequence::Constructor", expect = "8B C1 33 C9 66 89 48 04", this = "CAnimBlendSequence")]
pub fn constructor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u16>(this, 4, 0);
    wr::<u16>(this, 6, 0);
    wr::<u32>(this, 8, 0);
    wr::<u16>(this, 0, 0xFFFF);
    this
}

/// `CAnimBlendSequence::~CAnimBlendSequence()` (0x4D0C30): frees the frames unless external.
#[hook(addr = 0x4D0C30, conv = thiscall, name = "CAnimBlendSequence::Destructor", expect = "F6 41 04 08 75 0E", this = "CAnimBlendSequence")]
pub fn destructor(this: GPtr<u8>) {
    if flags(this) & FLAG_EXTERNAL == 0 {
        let f = frames(this);
        if !f.is_null() {
            mem_free(f);
        }
    }
}

/// `CAnimBlendSequence::SetName(const char*)` (0x4D0C50): hash = `CKeyGen::GetUppercaseKey(name)`;
/// the flags are not touched (gta-reversed's bugfix clears bit 4; the original does not).
#[hook(addr = 0x4D0C50, conv = thiscall, name = "CAnimBlendSequence::SetName", expect = "8B 44 24 04 56", this = "CAnimBlendSequence")]
pub fn set_name(this: GPtr<u8>, name: GPtr<u8>) {
    let h = get_uppercase_key(name);
    wr::<u32>(this, 0, h);
}

/// `CAnimBlendSequence::SetBoneTag(int32)` (0x4D0C70): for tag != -1 sets flag bit 4 and the low word.
#[hook(addr = 0x4D0C70, conv = thiscall, name = "CAnimBlendSequence::SetBoneTag", expect = "8B 44 24 04 83 F8 FF", this = "CAnimBlendSequence")]
pub fn set_bone_tag(this: GPtr<u8>, tag: i32) {
    if tag != -1 {
        set_flags(this, flags(this) | FLAG_BONE_TAG);
        wr::<u16>(this, 0, tag as u16);
    }
}

/// `CAnimBlendSequence::GetDataSize(bool compressed)` (0x4D0C90): frames * frame size.
#[hook(addr = 0x4D0C90, conv = thiscall, name = "CAnimBlendSequence::GetDataSize", expect = "8A 44 24 04 84 C0", this = "CAnimBlendSequence")]
pub fn get_data_size(this: GPtr<u8>, compressed: u8) -> u32 {
    let n = i32::from(num_frames(this)) as u32;
    let trans = flags(this) & FLAG_TRANS != 0;
    n.wrapping_mul(match (compressed != 0, trans) {
        (true, true) => 16,
        (true, false) => 10,
        (false, true) => 32,
        (false, false) => 20,
    })
}

/// `CAnimBlendSequence::SetNumFrames(int32 count, bool hasTrans, bool compressed, void* data)`
/// (0x4D0CD0): the flag bits are only OR-ed (bit 3 never written); null data = `CMemoryMgr::Malloc`.
#[hook(addr = 0x4D0CD0, conv = thiscall, name = "CAnimBlendSequence::SetNumFrames", expect = "8A 44 24 08 53", this = "CAnimBlendSequence")]
pub fn set_num_frames(this: GPtr<u8>, count: i32, has_trans: u8, compressed: u8, data: GPtr<u8>) {
    let c = count as u32;
    let mut data = data;
    let size = if has_trans != 0 {
        set_flags(this, flags(this) | 3);
        if compressed != 0 { c << 4 } else { c << 5 }
    } else {
        set_flags(this, flags(this) | 1);
        if compressed != 0 { c.wrapping_mul(10) } else { c.wrapping_mul(20) }
    };
    if data.is_null() {
        data = mem_malloc(size, 0);
    }
    wr::<GPtr<u8>>(this, 8, data);
    wr::<u16>(this, 6, count as u16);
    if compressed != 0 {
        set_flags(this, flags(this) | FLAG_COMPRESSED);
    }
}

/// `CAnimBlendSequence::Uncompress(void* buffer)` (0x4D0D40): converts the compressed frames into
/// `buffer` (or a new `CMemoryMgr::Malloc` block), frees the old frames unless external, then
/// external = (buffer != null), compressed = 0.
#[hook(addr = 0x4D0D40, conv = thiscall, name = "CAnimBlendSequence::Uncompress", expect = "56 8B F1 66 8B 46 06", this = "CAnimBlendSequence")]
pub fn uncompress(this: GPtr<u8>, buffer: GPtr<u8>) {
    let n = num_frames(this);
    if n == 0 {
        return;
    }
    let trans = flags(this) & FLAG_TRANS != 0;
    let dst = if !buffer.is_null() {
        buffer
    } else {
        let n = i32::from(n) as u32;
        mem_malloc(if trans { n << 5 } else { n.wrapping_mul(20) }, 0)
    };
    let src = frames(this);
    let mut i = 0i32;
    while i < i32::from(num_frames(this)) {
        if trans {
            let s = src.byte_add(i as isize * 0x10);
            let d = dst.byte_add(i as isize * 0x20);
            for k in 0..4 {
                wr::<f32>(d, 4 * k, unq(rd::<i16>(s, 2 * k), K_INV4096));
            }
            wr::<f32>(d, 0x10, unq(rd::<i16>(s, 8), k_inv60()));
            for k in 0..3 {
                wr::<f32>(d, 0x14 + 4 * k, unq(rd::<i16>(s, 0xA + 2 * k), K_INV1024));
            }
        } else {
            let s = src.byte_add(i as isize * 0xA);
            let d = dst.byte_add(i as isize * 0x14);
            for k in 0..4 {
                wr::<f32>(d, 4 * k, unq(rd::<i16>(s, 2 * k), K_INV4096));
            }
            wr::<f32>(d, 0x10, unq(rd::<i16>(s, 8), k_inv60()));
        }
        i += 1;
    }
    if flags(this) & FLAG_EXTERNAL == 0 {
        mem_free(frames(this));
    }
    wr::<GPtr<u8>>(this, 8, dst);
    if !buffer.is_null() {
        set_flags(this, (flags(this) | FLAG_EXTERNAL) & !FLAG_COMPRESSED);
    } else {
        set_flags(this, flags(this) & !FLAG_EXTERNAL & !FLAG_COMPRESSED);
    }
}

/// `CAnimBlendSequence::CompressKeyframes(void* buffer)` (0x4D0F40): the reverse conversion
/// (`_ftol(v * 4096)`, `_ftol(t * 60 + 0.5)`, `_ftol(p * 1024)`, low words); then external =
/// (buffer != null), compressed = 1.
#[hook(addr = 0x4D0F40, conv = thiscall, name = "CAnimBlendSequence::CompressKeyframes", expect = "51 53 8B D9 66 8B 43 06", this = "CAnimBlendSequence")]
pub fn compress_keyframes(this: GPtr<u8>, buffer: GPtr<u8>) {
    let n = num_frames(this);
    if n == 0 {
        return;
    }
    let trans = flags(this) & FLAG_TRANS != 0;
    let dst = if !buffer.is_null() {
        buffer
    } else {
        let n = i32::from(n) as u32;
        mem_malloc(if trans { n << 4 } else { n.wrapping_mul(10) }, 0)
    };
    let src = frames(this);
    let time = |f: f32| ftol32(x(f).fmul_m(60.0).fadd_m(0.5)) as i16;
    let mut i = 0i32;
    while i < i32::from(num_frames(this)) {
        if trans {
            let s = src.byte_add(i as isize * 0x20);
            let d = dst.byte_add(i as isize * 0x10);
            for k in 0..4 {
                wr::<i16>(d, 2 * k, q(rd::<f32>(s, 4 * k), 4096.0));
            }
            wr::<i16>(d, 8, time(rd::<f32>(s, 0x10)));
            for k in 0..3 {
                wr::<i16>(d, 0xA + 2 * k, q(rd::<f32>(s, 0x14 + 4 * k), 1024.0));
            }
        } else {
            let s = src.byte_add(i as isize * 0x14);
            let d = dst.byte_add(i as isize * 0xA);
            for k in 0..4 {
                wr::<i16>(d, 2 * k, q(rd::<f32>(s, 4 * k), 4096.0));
            }
            wr::<i16>(d, 8, time(rd::<f32>(s, 0x10)));
        }
        i += 1;
    }
    if flags(this) & FLAG_EXTERNAL == 0 {
        mem_free(frames(this));
    }
    wr::<GPtr<u8>>(this, 8, dst);
    if !buffer.is_null() {
        set_flags(this, flags(this) | FLAG_EXTERNAL | FLAG_COMPRESSED);
    } else {
        set_flags(this, (flags(this) & !FLAG_EXTERNAL) | FLAG_COMPRESSED);
    }
}

/// `CAnimBlendSequence::MoveMemory()` (0x4D1150): own frames moved by `CMemoryMgr::MoveMemory`;
/// AL = moved.
#[hook(addr = 0x4D1150, conv = thiscall, name = "CAnimBlendSequence::MoveMemory", expect = "56 8B F1 8A 46 04", this = "CAnimBlendSequence")]
pub fn move_memory(this: GPtr<u8>) -> u8 {
    if flags(this) & FLAG_EXTERNAL != 0 {
        return 0;
    }
    let f = frames(this);
    if f.is_null() {
        return 0;
    }
    let n = mem_move_memory(f);
    if n.addr() == frames(this).addr() {
        return 0;
    }
    wr::<GPtr<u8>>(this, 8, n);
    1
}

/// `CAnimBlendSequence::Print()` (0x4D1180): `ret` (not hookable).
#[sa_core::native(addr = 0x4D1180, conv = thiscall, name = "CAnimBlendSequence::Print")]
pub fn print(_this: GPtr<u8>) {}

/// `CAnimBlendSequence::RemoveQuaternionFlips()` (0x4D1190): for frames 1.., when the 4D dot product
/// with the previous (already corrected) frame, `((pw*cw + pz*cz) + py*cy) + cx*px`, is < 0 the
/// quaternion is negated.
#[hook(addr = 0x4D1190, conv = thiscall, name = "CAnimBlendSequence::RemoveQuaternionFlips", expect = "66 8B 51 06 83 EC 20", this = "CAnimBlendSequence")]
pub fn remove_quaternion_flips(this: GPtr<u8>) {
    if num_frames(this) < 2 {
        return;
    }
    let mut prev = [0u32; 4];
    let f0 = frames(this);
    for (k, p) in prev.iter_mut().enumerate() {
        *p = rd::<u32>(f0, 4 * k as isize);
    }
    let mut i = 1i32;
    while i < i32::from(num_frames(this)) {
        let stride = if flags(this) & FLAG_TRANS != 0 { 0x20 } else { 0x14 };
        let c = frames(this).byte_add(i as isize * stride);
        let cv = |k: isize| x(rd::<f32>(c, 4 * k));
        let pv = |k: usize| x(f32::from_bits(prev[k]));
        let s = pv(3)
            .fmul(cv(3))
            .fadd(pv(2).fmul(cv(2)))
            .fadd(pv(1).fmul(cv(1)))
            .fadd(cv(0).fmul(pv(0)));
        if s.fcom(X87::zero()) == sa_math::x87::Cmp::Less {
            for k in 0..4 {
                let p = at::<f32>(c, 4 * k);
                p.write(x(p.read()).fchs().to_f32());
            }
        }
        for (k, p) in prev.iter_mut().enumerate() {
            *p = rd::<u32>(c, 4 * k as isize);
        }
        i += 1;
    }
}

/// `CAnimBlendSequence::RemoveUncompressedData(void* buffer)` (0x4D12A0): with frames,
/// `CompressKeyframes(buffer)` (tail jump).
#[hook(addr = 0x4D12A0, conv = thiscall, name = "CAnimBlendSequence::RemoveUncompressedData", expect = "66 83 79 06 00", this = "CAnimBlendSequence")]
pub fn remove_uncompressed_data(this: GPtr<u8>, buffer: GPtr<u8>) {
    if num_frames(this) != 0 {
        call_compress_keyframes(this, buffer);
    }
}

orig! {
    /// `CAnimBlendSequence::CompressKeyframes` (0x4D0F40)
    pub fn call_compress_keyframes(this: GPtr<u8>, buffer: GPtr<u8>) = thiscall 0x4D0F40 => compress_keyframes;
    /// `CAnimBlendSequence::Uncompress` (0x4D0D40)
    pub fn call_uncompress(this: GPtr<u8>, buffer: GPtr<u8>) = thiscall 0x4D0D40 => uncompress;
    /// `CAnimBlendSequence::MoveMemory` (0x4D1150)
    pub fn call_move_memory(this: GPtr<u8>) -> u8 = thiscall 0x4D1150 => move_memory;
    /// `CAnimBlendSequence::RemoveQuaternionFlips` (0x4D1190)
    pub fn call_remove_quaternion_flips(this: GPtr<u8>) = thiscall 0x4D1190 => remove_quaternion_flips;
    /// `CAnimBlendSequence::~CAnimBlendSequence` (0x4D0C30)
    pub fn call_destructor(this: GPtr<u8>) = thiscall 0x4D0C30 => destructor;
    /// `CAnimBlendSequence::GetDataSize` (0x4D0C90)
    pub fn call_get_data_size(this: GPtr<u8>, compressed: u8) -> u32 = thiscall 0x4D0C90 => get_data_size;
}

pub const HOOKS: &[HookReg] = &[
    HOOK_GET_U_KEY_FRAME,
    HOOK_GET_C_KEY_FRAME,
    HOOK_CONSTRUCTOR,
    HOOK_DESTRUCTOR,
    HOOK_SET_NAME,
    HOOK_SET_BONE_TAG,
    HOOK_GET_DATA_SIZE,
    HOOK_SET_NUM_FRAMES,
    HOOK_UNCOMPRESS,
    HOOK_COMPRESS_KEYFRAMES,
    HOOK_MOVE_MEMORY,
    HOOK_REMOVE_QUATERNION_FLIPS,
    HOOK_REMOVE_UNCOMPRESSED_DATA,
];
