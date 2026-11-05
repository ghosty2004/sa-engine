//! L1 specs for PORT-STREAM owner M (`sa_stream`; see `docs/arch/stream-port.md`): the `this`-only
//! members of `CLoadedCarGroup` (and other pure leaves of the main agent's modules).
//!
//! The Rust side runs the actual `sa_stream` port on a copy of the scratch image through `GPtr`s into
//! that copy (host build = Rust-owned memory) and must leave it byte-identical to the original's.

use crate::call::{Conv::*, SCRATCH};
use crate::mspec::*;
use crate::specs::{ExpRet, Inputs, RetKind::*, Spec};
use sa_core::mem::GPtr;
use sa_stream::loaded_car_group::{self as lcg, CLoadedCarGroup};

/// Region 0 = a `CLoadedCarGroup` (0x2E bytes) plus slack up to 0x30.
fn group_region() -> Region {
    raw(12)
}

/// Shapes the random group words: word 11's low byte picks a layout (`k` leading used entries
/// followed by -1, fully random, all used, or duplicates); `arg` (an argument slot) is often one of
/// the members.
fn shape_group(v: &mut Inputs, l: &Layout, arg: Option<usize>) {
    let w = |k: usize| l.word_slot[0][k].unwrap();
    let sel = v[w(11)];
    let mode = sel & 7;
    let k = ((sel >> 8) % 24) as usize;
    let mut m = [0i16; 24];
    for (i, e) in m.iter_mut().enumerate() {
        let x = v[w(i / 2)] >> (16 * (i % 2));
        *e = match mode {
            0 | 1 => x as i16,
            2 | 3 => {
                if i < k {
                    (x % 700) as i16 + 400
                } else {
                    -1
                }
            }
            4 => (x % 4) as i16 + 400,
            _ => {
                if i < k {
                    (x & 0x7FFF) as i16
                } else if x & 0x100 != 0 {
                    -1
                } else {
                    x as i16
                }
            }
        };
    }
    for i in 0..12 {
        v[w(i)] = (m[2 * i] as u16 as u32) | ((m[2 * i + 1] as u16 as u32) << 16);
    }
    if let Some(a) = arg {
        let s = l.arg_slot[a].unwrap();
        let pick = (v[s] >> 8) % 26;
        if v[s] & 3 != 0 && (pick as usize) < 23 {
            let e = m[pick as usize];
            v[s] = if v[s] & 4 != 0 { e as u16 as u32 } else { e as i32 as u32 };
        }
    }
}

fn group_ptr(c: &mut Ctx) -> GPtr<CLoadedCarGroup> {
    GPtr::from_raw(c.mem.as_mut_ptr().wrapping_add(c.lay.off[0]).cast())
}

fn g_index(r: &mut crate::generate::Rng) -> u32 {
    match r.below(10) {
        0..=7 => r.below(23),
        _ => r.below(0x80),
    }
}

/// Every spec of this file.
pub fn all() -> Vec<Spec> {
    vec![
        b("CLoadedCarGroupClear", "CLoadedCarGroup::Clear", 0x611B90, Thiscall, Void)
            .regions(vec![group_region()])
            .args(vec![A::P(0)])
            .rust(|c| {
                lcg::clear(group_ptr(c));
                ExpRet::Void
            }),
        b("CLoadedCarGroupAddMember", "CLoadedCarGroup::AddMember", 0x611BB0, Thiscall, Void)
            .regions(vec![group_region()])
            .args(vec![A::P(0), A::I(|r| r.next_u32())])
            .fix(|v, l| shape_group(v, l, None))
            .rust(|c| {
                let m = c.i(1) as i32;
                lcg::add_member(group_ptr(c), m);
                ExpRet::Void
            }),
        b("CLoadedCarGroupRemoveMember", "CLoadedCarGroup::RemoveMember", 0x611BD0, Thiscall, Void)
            .regions(vec![group_region()])
            .args(vec![A::P(0), A::I(|r| r.next_u32())])
            .fix(|v, l| shape_group(v, l, Some(1)))
            .rust(|c| {
                let m = c.i(1) as i32;
                lcg::remove_member(group_ptr(c), m);
                ExpRet::Void
            }),
        b("CLoadedCarGroupGetMember", "CLoadedCarGroup::GetMember", 0x611C20, Thiscall, Eax)
            .regions(vec![raw(64)])
            .args(vec![A::P(0), A::I(g_index)])
            .rust(|c| {
                // indices < 0x80 stay inside the 0x100-byte region
                let i = c.i(1) as i32;
                ExpRet::Eax(lcg::get_member(group_ptr(c), i) as u32)
            }),
        b("CLoadedCarGroupCountMembers", "CLoadedCarGroup::CountMembers", 0x611C30, Thiscall, Eax)
            .regions(vec![group_region()])
            .args(vec![A::P(0)])
            .fix(|v, l| shape_group(v, l, None))
            .rust(|c| ExpRet::Eax(lcg::count_members(group_ptr(c)) as u32)),
        // __init_g_bab358: the `fstp dword [0xBAB358]` operand (0x84F2BC + 2) patched to region 0
        b("InitGBab358", "__init_g_bab358", 0x84F2B0, Cdecl, Void)
            .regions(vec![out1()])
            .patch(vec![(0x84F2BE, (SCRATCH + 0x10).to_le_bytes().to_vec())])
            .reads(vec![(0x858630, 0x858634), (0x866CC0, 0x866CC4)])
            .rust(|c| {
                c.set_u(0, 0, sa_stream::loading_screen::g_bab358_value().to_bits());
                ExpRet::Void
            }),
    ]
    .into_iter()
    .chain(pools())
    .collect()
}

/// The `CPool<T>` instantiations of `sa_stream::misc` (same generic algorithm as sa-core's pools,
/// whose spec builders are reused with these addresses and slot sizes).
fn pools() -> Vec<Spec> {
    use crate::specs_core::{ctor_spec, op_new_spec, pool_new_spec};
    vec![
        pool_new_spec("CTxdPoolNew", "FUN_00731b80", 0x731B80, 0x0C),
        ctor_spec("CTxdPoolCtor", "FUN_00731ad0", 0x731AD0, 0x0C, 0x82119A),
        ctor_spec("CColPoolCtor", "thunk_FUN_015611f0", 0x410F80, 0x2C, 0x821195),
        ctor_spec("CVehicleStructurePoolCtor", "FUN_005b8340", 0x5B8340, 0x314, 0x821195),
        op_new_spec("CDummyOperatorNew", "CDummy::operator new", 0x5326E0, &[0x532630], 0xB744A0, 0x38),
    ]
}
