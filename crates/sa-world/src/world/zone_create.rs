//! STD-5: `CTheZones::CreateZone` (0x5728A0), ported from the 1.0 US bytes (gta-reversed
//! `TheZones.cpp` as the reading aid; the bytes win: the first label goes to zone +0 and the second to
//! +8, both upper-cased in place in the caller's strings first, and type 2 creates nothing).
//!
//! `CZone` (0x20): +0 / +8 labels (8 bytes), +0x10..+0x1A x1, y1, z1, x2, y2, z2 (int16), +0x1C extra
//! info index, byte +0x1E type, byte +0x1F level. Navigation zones 0xBA3798 (count u16 0xBA3794), map
//! zones 0xBA1908 (count u16 0xBA1900).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

orig! {
    /// CRT `strncpy(dst, src, n)` (0x821F40)
    fn crt_strncpy(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
    /// `CTheZones::AssignZoneInfoForThisZone(int16)` (0x572180)
    fn assign_zone_info_for_this_zone(index: u32) = cdecl 0x572180 => unported;
}

const NAVI_COUNT: usize = 0xBA3794;
const NAVI_ZONES: usize = 0xBA3798;
const MAP_COUNT: usize = 0xBA1900;
const MAP_ZONES: usize = 0xBA1908;

/// Upper-cases the first `strlen & 0xFFFF` bytes of `s` in place (a-z only).
fn upper_in_place(s: GPtr<u8>) {
    let mut n = 0usize;
    while s.byte_add(n as isize).read() != 0 {
        n += 1;
    }
    for i in 0..(n as u16) as isize {
        let c = s.byte_add(i).read();
        if c.is_ascii_lowercase() {
            s.byte_add(i).write(c - 0x20);
        }
    }
}

/// `strcpy` of the 8-byte local into the zone field (up to and including the NUL).
fn copy_label(dst: GPtr<u8>, src: &[u8; 8]) {
    for (i, &c) in src.iter().enumerate() {
        dst.byte_add(i as isize).write(c);
        if c == 0 {
            break;
        }
    }
}

/// `CTheZones::CreateZone(const char* label0, eZoneType type, CVector p1, CVector p2, eLevelName level,
/// const char* label1)` (0x5728A0): each axis's pair is swapped (the first value through `fld`/`fstp`)
/// unless `p1 <= p2` or unordered; both labels are upper-cased in place and `strncpy`'d (7 bytes) into
/// 8-byte locals with a NUL at 7. Type 0 / 1: the next navigation zone (type byte = type, then
/// `AssignZoneInfoForThisZone(index)`), type 3: the next map zone (type byte 3); other types: nothing.
/// The coordinates are truncated (`_ftol`) to int16 in the order x1, y1, z1, x2, y2, z2.
#[hook(addr = 0x5728A0, conv = cdecl, name = "CTheZones::CreateZone", expect = "D9 44 24 0C 83 EC 10 D8 5C 24",
       unverified = "std5-zones-1", route = "L2", hazards = "none", recs = "new-game")]
#[allow(clippy::too_many_arguments)]
pub fn create_zone(
    label0: GPtr<u8>,
    ty: i32,
    x1: f32,
    y1: f32,
    z1: f32,
    x2: f32,
    y2: f32,
    z2: f32,
    level: u32,
    label1: GPtr<u8>,
) {
    let mut a = [x1, y1, z1];
    let mut b = [x2, y2, z2];
    for k in 0..3 {
        // `test ah, 0x41; jne` keeps the order for <, == and unordered
        let keep = a[k] <= b[k] || a[k].is_nan() || b[k].is_nan();
        if !keep {
            let t = a[k];
            a[k] = b[k];
            b[k] = X87::from_f32(t).to_f32();
        }
    }
    let mut l0 = [0u8; 8];
    let mut l1 = [0u8; 8];
    upper_in_place(label0);
    crt_strncpy(GPtr::from_raw(l0.as_mut_ptr()), label0, 7);
    l0[7] = 0;
    upper_in_place(label1);
    crt_strncpy(GPtr::from_raw(l1.as_mut_ptr()), label1, 7);
    l1[7] = 0;
    let (count, zones) = match ty {
        0 | 1 => (NAVI_COUNT, NAVI_ZONES),
        3 => (MAP_COUNT, MAP_ZONES),
        _ => return,
    };
    let idx = GPtr::<u16>::from_addr(count).read();
    let z = GPtr::<u8>::from_addr(zones + 0x20 * idx as usize);
    copy_label(z, &l0);
    copy_label(z.byte_add(8), &l1);
    z.byte_add(0x1E).write(ty as u8);
    for (k, v) in a.iter().chain(b.iter()).enumerate() {
        z.byte_add(0x10 + 2 * k as isize).cast::<u16>().write(X87::from_f32(*v).ftol() as u16);
    }
    z.byte_add(0x1F).write(level as u8);
    if ty == 3 {
        GPtr::<u16>::from_addr(count).write(idx.wrapping_add(1));
    } else {
        assign_zone_info_for_this_zone(u32::from(idx));
        GPtr::<u16>::from_addr(count).write(idx.wrapping_add(1));
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_CREATE_ZONE];
