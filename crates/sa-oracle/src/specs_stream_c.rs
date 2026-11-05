//! L1 specs for PORT-STREAM owner c (`sa_stream::modelinfo::vehicle`, `sa_stream::fileloader`; see
//! `docs/arch/stream-port.md`): the CVehicleModelInfo / CLinkedUpgradeList functions whose result
//! depends only on `this` (and argument) memory and integer arguments.
//!
//! `this` is a raw region; the Rust side runs the actual port on a copy of the scratch image through
//! a host pointer into it. Returned pointers are translated back from the host copy to the 1.0 US
//! scratch address. The CLinkedUpgradeList link count (+0x78) is kept within the region (the
//! original indexes the two 30-entry arrays without a bound).

use crate::call::Conv::*;
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::mem::GPtr;
use sa_stream::modelinfo::vehicle;
use sa_math::x87::X87;

/// Host pointer to region `r` of the expected image.
fn tp<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r]).cast())
}

/// A host pointer into the expected image → the scratch address it stands for (0 stays 0).
fn back(c: &mut Ctx, p: GPtr<u8>) -> u32 {
    if p.is_null() {
        return 0;
    }
    let base = c.mem.as_mut_ptr() as usize;
    crate::call::SCRATCH + (p.addr() - base) as u32
}

fn rng(seed: u32) -> Rng {
    Rng::new(u64::from(seed) | 1 << 41)
}

fn g_i16(r: &mut Rng) -> u32 {
    match r.below(4) {
        0 => r.next_u32(),
        1 => 0xFFFF,
        _ => r.below(40),
    }
}

/// Component words for CountCompsInRule: non-negative only. A negative word never reaches 0 under
/// the original's `sar` (both the original and the port loop forever; callers pass 12-bit values).
fn g_comps(r: &mut Rng) -> u32 {
    let v = match r.below(4) {
        0 => r.next_u32(),
        1 => 0,
        2 => r.next_u32() | 0x70F0_F0F0,
        _ => r.next_u32() & 0x0FFF_0FFF,
    };
    v & 0x7FFF_FFFF
}

/// Region words of a CVehicleModelInfo-sized `this` (0x308 bytes).
const VMI_WORDS: usize = 0x308 / 4;

/// Keeps the CLinkedUpgradeList count (word 30) within 0..=30 most of the time (and the i16 search
/// values often equal to stored ones).
#[allow(clippy::ptr_arg)] // the `fix` hook type takes `&mut Inputs`
fn linked_fix(v: &mut Vec<u32>, lay: &Layout) {
    let s = lay.word_slot[0][30].unwrap();
    let mut r = rng(v[s]);
    v[s] = r.below(31);
    if r.chance(60) {
        // copy some stored values into the search argument
        let k = r.below(60) as usize;
        let w = lay.word_slot[0][k / 2].unwrap();
        let val = (v[w] >> (16 * (k % 2) as u32)) & 0xFFFF;
        if let Some(a) = lay.arg_slot.get(1).copied().flatten() {
            v[a] = val | (r.next_u32() & 0xFFFF_0000);
        }
    }
}

/// Every spec of this file.
pub fn all() -> Vec<Spec> {
    vec![
        b("VehGetNumDoors", "CVehicleModelInfo::GetNumDoors", 0x4C73C0, Thiscall, Eax)
            .regions(vec![raw(0x14)])
            .args(vec![A::P(0)])
            .rust(|c| ExpRet::Eax(vehicle::get_num_doors(tp(c, 0)) as u32)),
        b("VehGetAnimFileIndex", "CVehicleModelInfo::GetAnimFileIndex", 0x4C7660, Thiscall, Eax)
            .regions(vec![raw(VMI_WORDS)])
            .args(vec![A::P(0)])
            .rust(|c| ExpRet::Eax(vehicle::get_anim_file_index(tp(c, 0)))),
        b("LinkedAddUpgradeLink", "CLinkedUpgradeList::AddUpgradeLink", 0x4C74B0, Thiscall, Void)
            .regions(vec![raw(31)])
            .args(vec![A::P(0), A::I(g_i16), A::I(g_i16)])
            .fix(linked_fix)
            .rust(|c| {
                let (a, b2) = (c.i(1) as i16, c.i(2) as i16);
                vehicle::add_upgrade_link(tp(c, 0), a, b2);
                ExpRet::Void
            }),
        b("LinkedFindOtherUpgrade", "CLinkedUpgradeList::FindOtherUpgrade", 0x4C74D0, Thiscall, Eax)
            .regions(vec![raw(31)])
            .args(vec![A::P(0), A::I(g_i16)])
            .fix(linked_fix)
            .rust(|c| {
                let a = c.i(1) as i16;
                ExpRet::Eax(vehicle::find_other_upgrade(tp(c, 0), a))
            }),
        b("VehHasAlphaMaterialCB", "CVehicleModelInfo::HasAlphaMaterialCB", 0x4C77C0, Cdecl, Eax)
            .regions(vec![raw(2), raw(1)])
            .args(vec![A::P(0), A::P(1)])
            .fix(|v, lay| {
                // alpha byte (+7) often 0xFF
                let s = lay.word_slot[0][1].unwrap();
                let mut r = rng(v[s]);
                if r.chance(50) {
                    v[s] |= 0xFF00_0000;
                }
            })
            .rust(|c| {
                let (m, d) = (tp(c, 0), tp(c, 1));
                let p = vehicle::has_alpha_material_cb(m, d);
                ExpRet::Eax(back(c, p))
            }),
        b("VehCountCompsInRule", "CVehicleModelInfo::CountCompsInRule", 0x4C7F80, Cdecl, Eax)
            .args(vec![A::I(g_comps)])
            .rust(|c| ExpRet::Eax(vehicle::count_comps_in_rule(c.i(0) as i32) as u32)),
        b("VehGetNumRemaps", "CVehicleModelInfo::GetNumRemaps", 0x4C86B0, Thiscall, Eax)
            .regions(vec![raw(VMI_WORDS)])
            .args(vec![A::P(0)])
            .fix(remap_fix)
            .rust(|c| ExpRet::Eax(vehicle::get_num_remaps(tp(c, 0)) as u32)),
        b("VehAddRemap", "CVehicleModelInfo::AddRemap", 0x4C86D0, Thiscall, Void)
            .regions(vec![raw(VMI_WORDS)])
            .args(vec![A::P(0), A::I(g_i16)])
            .fix(remap_fix)
            .rust(|c| {
                let t = c.i(1) as i16;
                vehicle::add_remap(tp(c, 0), t);
                ExpRet::Void
            }),
        b("VehGetCustomCarPlateText", "CVehicleModelInfo::GetCustomCarPlateText", 0x4C8970, Thiscall, Eax)
            .regions(vec![raw(0x10)])
            .args(vec![A::P(0)])
            .fix(|v, lay| {
                let s = lay.word_slot[0][10].unwrap();
                let mut r = rng(v[s]);
                if r.chance(40) {
                    v[s] &= 0xFFFF_FF00;
                }
            })
            .rust(|c| {
                let p = vehicle::get_custom_car_plate_text(tp(c, 0));
                ExpRet::Eax(back(c, p))
            }),
        b("VehGetWheelSize", "CVehicleModelInfo::GetWheelSize", 0x6A06F0, Thiscall, St0)
            .regions(vec![Region {
                words: {
                    let mut w = vec![W::R; 0x12];
                    w[0x10] = W::F;
                    w[0x11] = W::F;
                    w
                },
                kind: Kind::Any,
            }])
            .args(vec![A::P(0), A::I(g_byte_word)])
            .rust(|c| {
                let front = c.i(1) as u8;
                let v = vehicle::get_wheel_size(tp(c, 0), front);
                st0(X87::from_f32(v))
            }),
        b("VehDestructor", "CVehicleModelInfo::~CVehicleModelInfo", 0x4C5920, Thiscall, Void)
            .regions(vec![raw(4)])
            .args(vec![A::P(0)])
            .rust(|c| {
                vehicle::destructor(tp(c, 0));
                ExpRet::Void
            }),
    ]
}

/// A byte argument in a word with random upper bytes (callee reads the low byte).
fn g_byte_word(r: &mut Rng) -> u32 {
    match r.below(3) {
        0 => 0,
        1 => r.next_u32() & 0xFFFF_FF00,
        _ => r.next_u32(),
    }
}

/// The remap table (+0x2FA, 4 i16 + 1 spare) mostly holds small ids or -1 terminators.
#[allow(clippy::ptr_arg)] // the `fix` hook type takes `&mut Inputs`
fn remap_fix(v: &mut Vec<u32>, lay: &Layout) {
    let s0 = lay.word_slot[0][0x2F8 / 4].unwrap();
    let mut r = rng(v[s0]);
    for w in (0x2F8 / 4)..=(0x300 / 4) {
        let s = lay.word_slot[0][w].unwrap();
        let lo = if r.chance(30) { 0xFFFF } else { r.below(20) };
        let hi = if r.chance(30) { 0xFFFF } else { r.below(20) };
        if r.chance(80) {
            v[s] = lo | hi << 16;
        }
    }
}
