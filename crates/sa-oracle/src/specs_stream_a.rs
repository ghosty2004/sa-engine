//! L1 specs for PORT-STREAM owner A (`sa_stream::streaming_info`, `sa_stream::colstore`; see
//! `docs/arch/stream-port.md`).
//!
//! The ports run on a copy of the scratch image through host pointers into it. Globals the
//! originals read are made L1 inputs or fixed values:
//! - `CStreamingInfo::ms_pArrayBase` (0x9654B4) is patched to the address of region 0 (an array of
//!   8 infos at `SCRATCH + 0x10`) and allowed as a read; `this`/`after` are element addresses; the
//!   `next`/`prev` indices are kept in 0..8 so every write stays in the array;
//! - `ms_files[i].StreamHandle` (0x8E4904 + i * 0x30) is patched to `i << 24 | 0x123` for the 8
//!   images (allowed reads), the image id is kept in 0..8;
//! - `CColStore::ms_EntityAreaCode` (0x965554) is read through the `mov eax, [abs]` operand patched to
//!   a scratch word (region 2).

use crate::call::{Conv::*, SCRATCH};
use crate::generate::Rng;
use crate::mspec::*;
use crate::specs::{ExpRet, RetKind::*, Spec};
use sa_core::mem::GPtr;
use sa_stream::colstore;
use sa_stream::shared::{CStreamingInfo, ColDef};
use sa_stream::streaming_info as si;

const BASE: u32 = SCRATCH + 0x10;

fn host<T>(c: &mut Ctx, r: usize) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[r])).cast()
}
/// A host pointer for a scratch address inside the copy.
fn host_at<T>(c: &mut Ctx, addr: u32) -> GPtr<T> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add((addr - SCRATCH) as usize)).cast()
}

fn g_elem(r: &mut Rng) -> u32 {
    BASE + 0x14 * r.below(8)
}
fn g_word(r: &mut Rng) -> u32 {
    match r.below(6) {
        0 => 0,
        1 => 0xFFFF_FFFF,
        _ => r.next_u32(),
    }
}

fn info_region() -> Region {
    raw(5)
}

/// `next`/`prev` of every element of region 0 (8 infos) in 0..8 (signed 16-bit).
fn fix_indices(v: &mut [u32], lay: &Layout) {
    for e in 0..8 {
        let s = lay.word_slot[0][5 * e].unwrap();
        let w = v[s];
        let next = (w & 7) as u16;
        let prev = ((w >> 16) & 7) as u16;
        v[s] = (next as u32) | ((prev as u32) << 16);
    }
}

/// The image id byte (+7) of the region-`r` info in 0..8.
fn fix_img(v: &mut [u32], lay: &Layout, r: usize) {
    let s = lay.word_slot[r][1].unwrap();
    v[s] = (v[s] & 0x00FF_FFFF) | (((v[s] >> 24) & 7) << 24);
}

fn handle(i: i32) -> u32 {
    ((i as u32) << 24) | 0x123
}
fn handle_patches() -> Vec<(u32, Vec<u8>)> {
    (0..8)
        .map(|i| (0x8E4904 + 0x30 * i as u32, handle(i).to_le_bytes().to_vec()))
        .collect()
}

fn coldef_region() -> Region {
    let mut w = vec![W::F; 4];
    w.extend([W::R; 7]);
    Region {
        words: w,
        kind: Kind::Any,
    }
}

/// Area word: 0 half of the time.
fn fix_area(v: &mut [u32], lay: &Layout) {
    let s = lay.word_slot[2][0].unwrap();
    if v[s] & 1 == 0 {
        v[s] = 0;
    }
}

/// Every spec of this file.
pub fn all() -> Vec<Spec> {
    vec![
        b(
            "CStreamingInfoInit",
            "CStreamingInfo::Init",
            0x407460,
            Thiscall,
            Void,
        )
        .regions(vec![info_region()])
        .args(vec![A::P(0)])
        .rust(|c| {
            si::init(host(c, 0));
            ExpRet::Void
        }),
        b(
            "CStreamingInfoInList",
            "CStreamingInfo::InList",
            0x407560,
            Thiscall,
            Eax,
        )
        .regions(vec![info_region()])
        .args(vec![A::P(0)])
        .rust(|c| ExpRet::Eax(si::in_list(host(c, 0)))),
        b(
            "CStreamingInfoSetCdPosnAndSize",
            "CStreamingInfo::SetCdPosnAndSize",
            0x4075E0,
            Thiscall,
            Void,
        )
        .regions(vec![info_region()])
        .args(vec![A::P(0), A::I(g_word), A::I(g_word)])
        .rust(|c| {
            let (a, b2) = (c.i(1), c.i(2));
            si::set_cd_posn_and_size(host(c, 0), a, b2);
            ExpRet::Void
        }),
        b(
            "CStreamingInfoGetCdPosn",
            "CStreamingInfo::GetCdPosn",
            0x407570,
            Thiscall,
            Eax,
        )
        .regions(vec![info_region()])
        .args(vec![A::P(0)])
        .fix(|v, lay| fix_img(v, lay, 0))
        .patch(handle_patches())
        .reads(vec![(0x8E4904, 0x8E4904 + 8 * 0x30)])
        .rust(|c| ExpRet::Eax(si::get_cd_posn_with(handle, host(c, 0)))),
        b(
            "CStreamingInfoGetCdPosnAndSize",
            "CStreamingInfo::GetCdPosnAndSize",
            0x4075A0,
            Thiscall,
            Al,
        )
        .regions(vec![info_region(), raw(1), raw(1)])
        .args(vec![A::P(0), A::P(1), A::P(2)])
        .fix(|v, lay| {
            fix_img(v, lay, 0);
            // size 0 a quarter of the time
            let s = lay.word_slot[0][3].unwrap();
            if v[s] & 3 == 0 {
                v[s] = 0;
            }
        })
        .patch(handle_patches())
        .reads(vec![(0x8E4904, 0x8E4904 + 8 * 0x30)])
        .rust(|c| {
            let r = si::get_cd_posn_and_size_with(handle, host(c, 0), host(c, 1), host(c, 2));
            ExpRet::Al(r as u8)
        }),
        b(
            "CStreamingInfoAddToList",
            "CStreamingInfo::AddToList",
            0x407480,
            Thiscall,
            Void,
        )
        .regions(vec![raw(40)])
        .args(vec![A::I(g_elem), A::I(g_elem)])
        .fix(|v, l| fix_indices(v, l))
        .patch(vec![(0x9654B4, BASE.to_le_bytes().to_vec())])
        .reads(vec![(0x9654B4, 0x9654B8)])
        .rust(|c| {
            let (t, a) = (c.i(0), c.i(1));
            let base: GPtr<CStreamingInfo> = host(c, 0);
            let this = host_at(c, t);
            let after = host_at(c, a);
            si::add_to_list_with(base, this, after);
            ExpRet::Void
        }),
        b(
            "CStreamingInfoRemoveFromList",
            "CStreamingInfo::RemoveFromList",
            0x4074E0,
            Thiscall,
            Void,
        )
        .regions(vec![raw(40)])
        .args(vec![A::I(g_elem)])
        .fix(|v, l| fix_indices(v, l))
        .patch(vec![(0x9654B4, BASE.to_le_bytes().to_vec())])
        .reads(vec![(0x9654B4, 0x9654B8)])
        .rust(|c| {
            let t = c.i(0);
            let base: GPtr<CStreamingInfo> = host(c, 0);
            let this = host_at(c, t);
            si::remove_from_list_with(base, this);
            ExpRet::Void
        }),
        b(
            "CColStoreSetIfCollisionIsRequired",
            "CColStore::SetIfCollisionIsRequired",
            0x4103D0,
            Cdecl,
            Void,
        )
        .regions(vec![vec2(), coldef_region(), raw(1)])
        .args(vec![A::P(0), A::P(1)])
        .fix(|v, l| fix_area(v, l))
        .patch(vec![(0x4103D1, (SCRATCH + 0x70).to_le_bytes().to_vec())])
        .rust(|c| {
            let area = c.u(2, 0) as i32;
            let def: GPtr<ColDef> = host(c, 1);
            colstore::set_if_collision_is_required_with(area, host(c, 0), def);
            ExpRet::Void
        }),
        b(
            "CColStoreSetIfCollisionIsRequiredReducedBB",
            "CColStore::SetIfCollisionIsRequiredReducedBB",
            0x410470,
            Cdecl,
            Void,
        )
        .regions(vec![vec2(), coldef_region(), raw(1)])
        .args(vec![A::P(0), A::P(1)])
        .fix(|v, l| fix_area(v, l))
        .patch(vec![(0x4104C2, (SCRATCH + 0x70).to_le_bytes().to_vec())])
        .rust(|c| {
            let area = c.u(2, 0) as i32;
            let def: GPtr<ColDef> = host(c, 1);
            colstore::set_if_collision_is_required_reduced_bb_with(area, host(c, 0), def);
            ExpRet::Void
        }),
    ]
}
