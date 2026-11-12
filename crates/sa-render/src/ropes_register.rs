//! `CRopes::RegisterRope` (batch `std8-render-1`): ported from the 1.0 US bytes (not reversed in gta-reversed).
//! Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd, wr};

orig! {
    /// `CEntity::RegisterReference(CEntity** ref)` (0x571B70, `ret 4`)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CRope::CreateHookObjectForRope()` (0x556070)
    fn rope_create_hook_object(rope: GPtr<u8>) = thiscall 0x556070 => crate::mvpb_r1::rope_create_hook_object;
}

/// `CRopes::aRopes` (8 × 0x328 bytes from 0xB768B8): `m_aSegments[32]` +0, `m_aSegmentsReleased[32]` +0x180, `m_nId`
/// +0x300, the type byte +0x325.
const ROPES: usize = 0xB768B8;
const ROPE_SIZE: usize = 0x328;
const ROPE_COUNT: usize = 8;
/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84)
const TIME_MS: usize = 0xB7CB84;

/// One `fld m32` / `fstp m32` copy (an sNaN comes out quiet).
fn fcopy(v: f32) -> f32 {
    X87::from_f32(v).to_f32()
}

/// `segments[k] = (segments[k-1].x op len, y, z)` or, `along_z`, `(x, y, z - len)` (the x87 operand order of the
/// two unrolled loops: `len + x`, `x - len`, `z - len`; x87 results stored as f32, the untouched x copied raw
/// in the vertical case and through `fld`/`fstp` in the horizontal one, y and z always through `fld`/`fstp`).
fn place_segment(rope: GPtr<u8>, k: usize, along_z: bool) {
    let len = X87::from_f32(rd::<f32>(rope, 0x30C));
    let prev = (k as isize - 1) * 12;
    let cur = k as isize * 12;
    if along_z {
        let xb = rd::<u32>(rope, prev);
        let y = fcopy(rd::<f32>(rope, prev + 4));
        let z = X87::from_f32(rd::<f32>(rope, prev + 8)).fsub(len).to_f32();
        wr::<u32>(rope, cur, xb);
        wr::<f32>(rope, cur + 4, y);
        wr::<f32>(rope, cur + 8, z);
    } else {
        let px = rd::<f32>(rope, prev);
        let xv = if k & 1 != 0 {
            len.fadd_m(px)
        } else {
            X87::from_f32(px).fsub(len)
        }
        .to_f32();
        let y = fcopy(rd::<f32>(rope, prev + 4));
        let z = fcopy(rd::<f32>(rope, prev + 8));
        wr::<f32>(rope, cur, xv);
        wr::<f32>(rope, cur + 4, y);
        wr::<f32>(rope, cur + 8, z);
        let rel = 0x180 + cur;
        wr::<u32>(rope, rel, 0);
        wr::<u32>(rope, rel + 4, 0);
        wr::<u32>(rope, rel + 8, 0);
    }
}

/// `CRopes::RegisterRope(uint32 id, uint32 type, CVector pos, bool expires, uint8 segmentCount, uint8 flags,
/// CPhysical* holder, uint32 timeToExpire)` (0x556B40, cdecl, returns 0 / 1 in EAX).
///
/// A rope with that id (type byte +0x325 set, +0x300 == id) is reset: `segments[0] = pos`, `released[0] = 0`, flag
/// bit 0 of +0x327 set, +0x324 = segmentCount, then for `i = 0 ..= +0x324` (the byte re-read every pass: a large
/// count writes past the arrays, through the rope's own fields) `segments[i] = pos`, `released[i] = 0`;
/// `CreateHookObjectForRope`; 1.
///
/// Otherwise the first rope with a zero type byte is taken (none: 0): +0x300 = id, `segments[0] = pos`,
/// `released[0] = 0`, +0x324 = segmentCount, +0x327 = `(+0x327 & 0xF9) | (flags & 1) << 2 | 1`, +0x304 = +0x314 =
/// +0x318 = 0, +0x31C = 0.9 when the holder is a vehicle (`+0x36 & 7 == 2`) else 0.5, +0x326 = 0, +0x310 = holder
/// (`RegisterReference` when not null), +0x325 = type, +0x320 = `expires ? CTimer::m_snTimeInMilliseconds +
/// timeToExpire : 0`. By type (jump table 0x557514): 3 → (10.0, 0x3EA5294A), 4 → (50.0, 0x3FCE739D), 5..7 → (68.0,
/// 0x400C6319), else (20.0, 0x3F25294A) into +0x308 / +0x30C (the segment length). Types 4..7 hang the 31 further
/// segments straight down (`z - length`); the others zig-zag along x (`len + x` on odd, `x - len` on even segments)
/// and clear `released[1..32]`. `CreateHookObjectForRope`; 1.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x556B40, conv = cdecl, name = "CRopes::RegisterRope", expect = "8B 54 24 04 83 EC 38",
       unverified = "std8-render-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "the reset path's segment loop runs to the count byte inclusive and can write past the arrays")]
pub fn ropes_register_rope(
    id: u32,
    rope_type: u32,
    px: u32,
    py: u32,
    pz: u32,
    expires: u32,
    segment_count: u32,
    flags: u32,
    holder: GPtr<u8>,
    time_to_expire: u32,
) -> u32 {
    let rope_at = |i: usize| GPtr::<u8>::from_addr(ROPES + i * ROPE_SIZE);
    let set_pos = |r: GPtr<u8>, off: isize| {
        wr::<u32>(r, off, px);
        wr::<u32>(r, off + 4, py);
        wr::<u32>(r, off + 8, pz);
    };
    let zero = |r: GPtr<u8>, off: isize| {
        wr::<u32>(r, off, 0);
        wr::<u32>(r, off + 4, 0);
        wr::<u32>(r, off + 8, 0);
    };
    if let Some(i) =
        (0..ROPE_COUNT).find(|&i| rd::<u8>(rope_at(i), 0x325) != 0 && rd::<u32>(rope_at(i), 0x300) == id)
    {
        let r = rope_at(i);
        set_pos(r, 0);
        zero(r, 0x180);
        wr::<u8>(r, 0x327, rd::<u8>(r, 0x327) | 1);
        wr::<u8>(r, 0x324, segment_count as u8);
        let mut k = 0isize;
        loop {
            set_pos(r, k * 12);
            zero(r, 0x180 + k * 12);
            k += 1;
            if k > rd::<u8>(r, 0x324) as isize {
                break;
            }
        }
        rope_create_hook_object(r);
        return 1;
    }
    let Some(i) = (0..ROPE_COUNT).find(|&i| rd::<u8>(rope_at(i), 0x325) == 0) else {
        return 0;
    };
    let r = rope_at(i);
    wr::<u32>(r, 0x300, id);
    set_pos(r, 0);
    zero(r, 0x180);
    let fl = (((flags as u8) & 1) << 2) | (rd::<u8>(r, 0x327) & 0xF9) | 1;
    wr::<u8>(r, 0x324, segment_count as u8);
    wr::<u8>(r, 0x327, fl);
    wr::<u32>(r, 0x304, 0);
    wr::<u32>(r, 0x314, 0);
    wr::<u32>(r, 0x318, 0);
    let vehicle = !holder.is_null() && rd::<u8>(holder, 0x36) & 7 == 2;
    wr::<u32>(r, 0x31C, if vehicle { 0x3F666666 } else { 0x3F000000 });
    wr::<u8>(r, 0x326, 0);
    wr::<GPtr<u8>>(r, 0x310, holder);
    wr::<u8>(r, 0x325, rope_type as u8);
    if !holder.is_null() {
        entity_register_reference(holder, r.byte_add(0x310));
    }
    let expiry = if expires as u8 != 0 {
        g::<u32>(TIME_MS).read().wrapping_add(time_to_expire)
    } else {
        0
    };
    wr::<u32>(r, 0x320, expiry);
    let ty = rd::<u8>(r, 0x325);
    let (weight, len) = match ty {
        3 => (0x41200000u32, 0x3EA5294Au32),
        4 => (0x42480000, 0x3FCE739D),
        5..=7 => (0x42880000, 0x400C6319),
        _ => (0x41A00000, 0x3F25294A),
    };
    wr::<u32>(r, 0x308, weight);
    wr::<u32>(r, 0x30C, len);
    let along_z = (4..=7).contains(&ty);
    for k in 1..32 {
        place_segment(r, k, along_z);
    }
    rope_create_hook_object(r);
    1
}

pub const HOOKS: &[HookReg] = &[HOOK_ROPES_REGISTER_ROPE];
