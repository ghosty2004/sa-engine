//! ANIM: `CVehicleAnimGroup` and `CVehicleAnimGroupData` (PORT-ANIM-INPUT, fork D). Ported from the
//! 1.0 US bytes; gta-reversed `VehicleAnimGroupData.cpp` as a reading aid,
//! `docs/re/specs/ANIM/CVehicleAnimGroup.deviations.md` for the kept original behaviour. See
//! `docs/arch/anim-input-port.md`.
//!
//! `CVehicleAnimGroup` (0x94): +0 first group (u8), +1 second group (u8), +4 anim flags, +8 special
//! flags, +0xC general timing (5 floats), +0x20 start in/out timing (4 floats), +0x30 end in/out
//! timing (4 floats), +0x40 door offsets (7 `CVector`). `CVehicleAnimGroupData::m_vehicleAnimGroups`
//! = 0xC1CDC0 (30 groups).

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::orig;
use sa_math::x87::{Cmp, X87};

use crate::common::{rd, wr, x};

sa_core::globals! {
    list GLOBALS;
    /// `CVehicleAnimGroupData::m_vehicleAnimGroups` (30 × 0x94)
    pub static VEHICLE_ANIM_GROUPS @ 0xC1CDC0: [u8; 30 * 0x94] = [0; 30 * 0x94];
}

/// `&m_vehicleAnimGroups[i]` (`imul` 32-bit, no bounds check).
pub fn group(i: i32) -> GPtr<u8> {
    VEHICLE_ANIM_GROUPS.ptr().cast::<u8>().byte_add(i.wrapping_mul(0x94) as isize)
}

orig! {
    /// `CVehicleAnimGroup::GetGroup(animId)` (0x6E3B00)
    pub fn call_get_group(this: GPtr<u8>, anim: i32) -> i32 = thiscall 0x6E3B00 => get_group;
    /// `CVehicleAnimGroup::ComputeAnimDoorOffsets(CVector* ret, door)` (0x6E3D10)
    pub fn call_compute_anim_door_offsets(this: GPtr<u8>, ret: GPtr<u8>, door: i32) -> GPtr<u8> = thiscall 0x6E3D10 => compute_anim_door_offsets;
    /// CRT `pow(double, double)` (0x8220F0): the computation `_CIpow` (0x822130) shares
    pub fn crt_pow(a: f64, b: f64) -> f64 = cdecl 0x8220F0 => crt_pow_host;
    /// `CAnimManager::GetAnimAssociation(groupId, animId)` (0x4D3A60)
    pub fn get_anim_association(group: i32, anim: i32) -> GPtr<u8> = cdecl 0x4D3A60 => unported;
    /// `CAnimManager::UncompressAnimation(hierarchy)` (0x4D41C0)
    pub fn uncompress_animation(h: GPtr<u8>) = cdecl 0x4D41C0 => unported;
}

/// Phase B/C path of [`crt_pow`]: only `pow(2.0, integer)` is used here, which is exact
/// (overflow → +inf, underflow → +0).
pub fn crt_pow_host(a: f64, b: f64) -> f64 {
    assert!(a == 2.0 && b == (b as i64) as f64, "crt_pow_host: only pow(2, integer)");
    let e = b as i64;
    if e > 1023 {
        f64::INFINITY
    } else if e >= -1022 {
        f64::from_bits(((e + 1023) as u64) << 52)
    } else if e >= -1074 {
        f64::from_bits(1u64 << (e + 1074))
    } else {
        0.0
    }
}

/// `CVehicleAnimGroup::CopyAnimGroup(src)` (0x5B9E40): both group bytes, the dwords +4..+0x20, then
/// the in/out timings interleaved (+0x30, +0x24, +0x34, +0x28, +0x38, +0x2C, +0x3C); the door
/// offsets and bytes 2..3 are not copied. EAX = the last dword copied.
#[hook(addr = 0x5B9E40, conv = thiscall, name = "CVehicleAnimGroup::CopyAnimGroup", expect = "8B 44 24 04 8A 10", this = "CVehicleAnimGroup")]
pub fn copy_anim_group(this: GPtr<u8>, src: GPtr<u8>) -> u32 {
    wr::<u8>(this, 0, rd::<u8>(src, 0));
    wr::<u8>(this, 1, rd::<u8>(src, 1));
    let mut last = 0;
    for o in [4, 8, 0xC, 0x10, 0x14, 0x18, 0x1C, 0x20, 0x30, 0x24, 0x34, 0x28, 0x38, 0x2C, 0x3C] {
        last = rd::<u32>(src, o);
        wr::<u32>(this, o, last);
    }
    last
}

/// `CVehicleAnimGroup::InitAnimGroup(first, second, animFlags, specialFlags, general, start, end)`
/// (0x5B9EB0): the stores in the original's order; EAX = `start[3]` (the last dword moved through
/// EAX; EDX carries `end[3]`).
#[hook(addr = 0x5B9EB0, conv = thiscall, name = "CVehicleAnimGroup::InitAnimGroup", expect = "8A 44 24 04 8A 54 24 08", this = "CVehicleAnimGroup")]
#[allow(clippy::too_many_arguments)]
pub fn init_anim_group(
    this: GPtr<u8>,
    first: u8,
    second: u8,
    anim_flags: u32,
    special_flags: u32,
    general: GPtr<u8>,
    start: GPtr<u8>,
    end: GPtr<u8>,
) -> u32 {
    wr::<u8>(this, 0, first);
    wr::<u8>(this, 1, second);
    wr::<u32>(this, 4, anim_flags);
    wr::<u32>(this, 8, special_flags);
    for k in 0..5 {
        wr::<u32>(this, 0xC + 4 * k, rd::<u32>(general, 4 * k));
    }
    wr::<u32>(this, 0x20, rd::<u32>(start, 0));
    wr::<u32>(this, 0x30, rd::<u32>(end, 0));
    wr::<u32>(this, 0x24, rd::<u32>(start, 4));
    wr::<u32>(this, 0x34, rd::<u32>(end, 4));
    wr::<u32>(this, 0x28, rd::<u32>(start, 8));
    wr::<u32>(this, 0x38, rd::<u32>(end, 8));
    let s3 = rd::<u32>(start, 0xC);
    wr::<u32>(this, 0x2C, s3);
    wr::<u32>(this, 0x3C, rd::<u32>(end, 0xC));
    s3
}

/// The flag bit `GetGroup` tests for an anim id (byte table 0x6E3C58 → jump table 0x6E3C0C), `None`
/// for index 26 (anim 377), whose case keeps the anim id itself.
fn group_bit(anim: i32) -> i32 {
    const BYTES: [u8; 40] = [
        0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 3, 5, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 18, 11, 11, 12, 12, 13,
        13, 14, 14, 15, 16, 16, 17, 17,
    ];
    let idx = anim.wrapping_sub(351) as u32;
    if idx > 39 {
        return anim;
    }
    match BYTES[idx as usize] {
        18 => anim,
        b => i32::from(b),
    }
}

/// `CVehicleAnimGroup::GetGroup(animId)` (0x6E3B00): `mask = _ftol(pow(2.0, bit))` (low dword), then
/// `m_animFlags & mask ? (int8)second : (int8)first` (EAX sign-extended). `bit` is the case's flag
/// for ids 351..390 and the anim id itself otherwise (and for 377), so ids >= 32 give mask 0.
#[hook(addr = 0x6E3B00, conv = thiscall, name = "CVehicleAnimGroup::GetGroup", expect = "8B 44 24 04 05 A1 FE FF FF", this = "CVehicleAnimGroup")]
pub fn get_group(this: GPtr<u8>, anim: i32) -> i32 {
    let bit = group_bit(anim);
    let p = crt_pow(2.0, f64::from(bit));
    let mask = X87::from_f64_exact(p).ftol() as u32;
    if rd::<u32>(this, 4) & mask != 0 {
        i32::from(rd::<i8>(this, 1))
    } else {
        i32::from(rd::<i8>(this, 0))
    }
}

/// `CVehicleAnimGroup::ComputeCriticalBlendTime(animId)` (0x6E3C80): byte table 0x6E3CE0 → the
/// general timing float (+0xC, +0x14, +0x18, +0x10, +0x1C) or 0.0. Index 26 (anim 377) jumps
/// through the dword after the 6-entry table, which is 0: the original crashes there.
#[hook(addr = 0x6E3C80, conv = thiscall, name = "CVehicleAnimGroup::ComputeCriticalBlendTime", expect = "8B 44 24 04 05 A1 FE FF FF", this = "CVehicleAnimGroup")]
pub fn compute_critical_blend_time(this: GPtr<u8>, anim: i32) -> X87 {
    const BYTES: [u8; 40] = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 6, 3, 3, 1, 1, 1, 1, 4, 4,
        1, 5, 5, 1, 1,
    ];
    let idx = anim.wrapping_sub(351) as u32;
    let case = if idx > 39 { 1 } else { BYTES[idx as usize] };
    let off = match case {
        0 => 0xC,
        1 => return x(0.0),
        2 => 0x14,
        3 => 0x18,
        4 => 0x10,
        5 => 0x1C,
        _ => panic!("CVehicleAnimGroup::ComputeCriticalBlendTime(377): the original jumps to address 0"),
    };
    x(rd::<f32>(this, off))
}

/// `CVehicleAnimGroup::ComputeAnimDoorOffsets(CVector* ret, door)` (0x6E3D10): when the cached
/// offset `this + 0x40 + door*12` (no range check) has `(x*x + y*y) + z*z == 0` exactly, it is set
/// from the last frame's translation of the door anim's first sequence (door 0..6 → anims 359, 361,
/// 363, 373, 375, 378, 379; any other door value is used as the anim id); `*ret` = the offset.
#[hook(addr = 0x6E3D10, conv = thiscall, name = "CVehicleAnimGroup::ComputeAnimDoorOffsets", expect = "8B 54 24 08 8D 04 52", this = "CVehicleAnimGroup")]
pub fn compute_anim_door_offsets(this: GPtr<u8>, ret: GPtr<u8>, door: i32) -> GPtr<u8> {
    let o = this.byte_add(0x40 + (door.wrapping_mul(12)) as isize);
    let (ox, oy, oz) = (x(rd::<f32>(o, 0)), x(rd::<f32>(o, 4)), x(rd::<f32>(o, 8)));
    let len = ox.fmul(ox).fadd(oy.fmul(oy)).fadd(oz.fmul(oz));
    if len.fcom(x(0.0)) == Cmp::Equal {
        let anim = match door {
            0 => 0x167,
            1 => 0x169,
            2 => 0x16B,
            3 => 0x175,
            4 => 0x177,
            5 => 0x17A,
            6 => 0x17B,
            d => d,
        };
        let g = call_get_group(this, anim);
        let h = rd::<GPtr<u8>>(get_anim_association(g, anim), 0x10);
        let seq = rd::<GPtr<u8>>(h, 4);
        uncompress_animation(h);
        let n = rd::<i16>(seq, 6);
        if n > 0 {
            let last = i32::from(n) - 1;
            let frames = rd::<GPtr<u8>>(seq, 8);
            let stride = if (rd::<u8>(seq, 4) >> 1) & 1 != 0 { 0x20 } else { 0x14 };
            let t = frames.byte_add((last.wrapping_mul(stride) + 0x14) as isize);
            for k in 0..3 {
                wr::<u32>(o, 4 * k, rd::<u32>(t, 4 * k));
            }
        }
    }
    for k in 0..3 {
        wr::<u32>(ret, 4 * k, rd::<u32>(o, 4 * k));
    }
    ret
}

/// `CVehicleAnimGroupData::GetGroupForAnim(groupIndex, animId)` (0x639FC0): `GetGroup` of
/// `m_vehicleAnimGroups[groupIndex]`.
#[hook(addr = 0x639FC0, conv = cdecl, name = "CVehicleAnimGroupData::GetGroupForAnim", expect = "8B 4C 24 04 8B 44 24 08")]
pub fn get_group_for_anim(index: i32, anim: i32) -> i32 {
    call_get_group(group(index), anim)
}

/// `CVehicleAnimGroupData::UsesTruckDrivingAnims(groupIndex)` (0x639FE0): `(flags >> 3)` with AL
/// masked to bit 0 (EAX keeps the shifted upper bits).
#[hook(addr = 0x639FE0, conv = cdecl, name = "CVehicleAnimGroupData::UsesTruckDrivingAnims", expect = "8B 44 24 04 69 C0 94 00 00 00")]
pub fn uses_truck_driving_anims(index: i32) -> u32 {
    let v = rd::<u32>(group(index), 8) >> 3;
    (v & 0xFFFF_FF00) | (v & 1)
}

/// `CVehicleAnimGroupData::UsesKartDrivingAnims(groupIndex)` (0x6D09E0): as
/// [`uses_truck_driving_anims`] with bit 2.
#[hook(addr = 0x6D09E0, conv = cdecl, name = "CVehicleAnimGroupData::UsesKartDrivingAnims", expect = "8B 44 24 04 69 C0 94 00 00 00")]
pub fn uses_kart_driving_anims(index: i32) -> u32 {
    let v = rd::<u32>(group(index), 8) >> 2;
    (v & 0xFFFF_FF00) | (v & 1)
}

/// `CVehicleAnimGroupData::GetAnimDoorOffset(CVector* out, groupIndex, door)` (0x645600): returns
/// `out`.
// Not hooked (INF-20): protection code at 0x1473D70/0x1473D9C uses 0x645602, inside the patched entry, as an
// immediate (`adc eax, 0x645602`). The port stays a plain function.
pub fn get_anim_door_offset(out: GPtr<u8>, index: i32, door: i32) -> GPtr<u8> {
    call_compute_anim_door_offsets(group(index), out, door);
    out
}

/// `CVehicleAnimGroupData::GetInOutTimings(groupIndex, type, float* start, float* end)` (0x645630;
/// the `imul` sits in a chunk at 0x40A238): `*start = timing[type] start`, `*end = ... end`
/// (raw dwords); EAX = the end value's bits.
#[hook(addr = 0x645630, conv = cdecl, name = "CVehicleAnimGroupData::GetInOutTimings", expect = "8B 44 24 04 8B 4C 24 08")]
pub fn get_in_out_timings(index: i32, kind: i32, start: GPtr<u32>, end: GPtr<u32>) -> u32 {
    let g = group(index).byte_add((kind.wrapping_mul(4)) as isize);
    start.write(rd::<u32>(g, 0x20));
    let e = rd::<u32>(g, 0x30);
    end.write(e);
    e
}

pub const HOOKS: &[HookReg] = &[
    HOOK_COPY_ANIM_GROUP,
    HOOK_INIT_ANIM_GROUP,
    HOOK_GET_GROUP,
    HOOK_COMPUTE_CRITICAL_BLEND_TIME,
    HOOK_COMPUTE_ANIM_DOOR_OFFSETS,
    HOOK_GET_GROUP_FOR_ANIM,
    HOOK_USES_TRUCK_DRIVING_ANIMS,
    HOOK_USES_KART_DRIVING_ANIMS,
    HOOK_GET_IN_OUT_TIMINGS,
];
