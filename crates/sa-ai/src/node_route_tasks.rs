//! Route-following task helpers of the loading and on-foot path (batch `mvpi-ai-1`):
//! `CTaskComplexFollowNodeRoute` construction. Ported from the 1.0 US bytes (protection fragments
//! inlined; the exe wins over gta-reversed). Every call to another catalogued function goes through
//! `orig!` at the original address; unverified (docs/verify/HOWTO.md).
//!
//! `CTaskComplexFollowNodeRoute` (0x60): `CTaskComplex` +0..+0xB, target +0xC (CVector), move state
//! +0x18, radius +0x1C, two floats +0x20/+0x24, +0x28 (dword, written as a word first), `CNodeRoute*`
//! +0x2C, `CPointRoute*` +0x30, +0x34 (int16), +0x38, time +0x3C, +0x40, +0x44, two bytes +0x48/+0x49,
//! flags +0x4C (bit 0, bit 3, bits 5-6), +0x50..+0x5C.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

/// `CTaskComplexFollowNodeRoute` vtable (0x8700A8).
const VT_FOLLOW_NODE_ROUTE: u32 = 0x8700A8;
/// The dword the constructor copies into +0x28 (0x8A5F44, an empty-node-address constant).
const EMPTY_NODE_ADDRESS: usize = 0x8A5F44;

orig! {
    /// `CTaskComplex::CTaskComplex` (0x61A3B0)
    fn task_complex_ctor(t: u32) -> u32 = thiscall 0x61A3B0 => crate::gen_ctors::g_ctask_complex_61a3b0;
    /// `CPointRoute::operator new(uint32)` (0x41B7C0, cdecl)
    fn point_route_new(size: u32) -> GPtr<u8> = cdecl 0x41B7C0 => sa_world::world::mvpa2::point_route_new;
    /// `CNodeRoute::operator new(uint32)` (0x41B860, cdecl)
    fn node_route_new(size: u32) -> GPtr<u8> = cdecl 0x41B860 => node_route_new_host;
}

/// Phase B/C path of [`node_route_new`].
pub fn node_route_new_host(size: u32) -> GPtr<u8> {
    sa_core::pools::node_route_operator_new(size).cast()
}

/// `CTaskComplexFollowNodeRoute::CTaskComplexFollowNodeRoute(moveState, const CVector& target, radius,
/// f1, f2, bool flag0, time, bool flag3)` (0x66EA30, thiscall, `ret 0x20`; the SEH handler push sits in
/// the fragment 0x407441). `CTaskComplex` constructor, vtable 0x8700A8, target +0xC (copied x, y, z),
/// radius +0x1C, move state +0x18, +0x20 / +0x24 = f1 / f2, word +0x28 = -1, word +0x34 = -1, time
/// +0x3C, +0x38 / +0x40 / +0x44 = 0, bytes +0x48 / +0x49 = 0; flags +0x4C = (flags & !9) | (flag3 & 1)
/// << 3 | (flag0 & 1) (the previous value of +0x4C is read); with bit 3 set also bits 5-6 cleared and
/// +0x50..+0x5C zeroed. `CPointRoute::operator new(0x64)` (count +0 = 0 when non-null) → +0x30,
/// `CNodeRoute::operator new(0x24)` (count 0, the eight int16 node addresses at +4 step 4 = -1, when
/// non-null) → +0x2C, word +0x34 = -1 again, dword +0x28 = [0x8A5F44]. Returns `this`.
#[hook(addr = 0x66ea30, conv = thiscall, name = "CTaskComplexFollowNodeRoute::Constructor",
       expect = "6A FF E9 0A 8A D9 FF", unverified = "mvpi-ai-1", route = "L2", hazards = "heap-writes",
       recs = "grove-walk", note = "allocates a CPointRoute and a CNodeRoute from their pools")]
#[allow(clippy::too_many_arguments)]
pub fn follow_node_route_ctor(
    this: GPtr<u8>,
    move_state: u32,
    target: GPtr<u32>,
    radius: u32,
    f1: u32,
    f2: u32,
    flag0: u32,
    time: u32,
    flag3: u32,
) -> GPtr<u8> {
    task_complex_ctor(this.addr() as u32);
    wr::<u32>(this, 0, VT_FOLLOW_NODE_ROUTE);
    wr::<u32>(this, 0xC, target.read());
    wr::<u32>(this, 0x10, target.wrapping_index(1).read());
    wr::<u32>(this, 0x14, target.wrapping_index(2).read());
    wr::<u32>(this, 0x1C, radius);
    wr::<u32>(this, 0x18, move_state);
    wr::<u32>(this, 0x20, f1);
    wr::<u32>(this, 0x24, f2);
    wr::<u16>(this, 0x28, 0xFFFF);
    wr::<u16>(this, 0x34, 0xFFFF);
    wr::<u32>(this, 0x3C, time);
    wr::<u32>(this, 0x38, 0);
    wr::<u32>(this, 0x40, 0);
    wr::<u32>(this, 0x44, 0);
    wr::<u8>(this, 0x48, 0);
    wr::<u8>(this, 0x49, 0);
    let mut fl = ((flag3 & 0xFF & 1) << 3) | (flag0 & 0xFF & 1) | (rd::<u32>(this, 0x4C) & 0xFFFF_FFF6);
    wr::<u32>(this, 0x4C, fl);
    if fl & 8 != 0 {
        fl &= 0xFFFF_FF9F;
        wr::<u32>(this, 0x4C, fl);
        wr::<u32>(this, 0x50, 0);
        wr::<u32>(this, 0x54, 0);
        wr::<u32>(this, 0x58, 0);
        wr::<u32>(this, 0x5C, 0);
    }
    let pr = point_route_new(0x64);
    if !pr.is_null() {
        wr::<u32>(pr, 0, 0);
    }
    wr::<GPtr<u8>>(this, 0x30, pr);
    let nr = node_route_new(0x24);
    if !nr.is_null() {
        wr::<u32>(nr, 0, 0);
        for i in 0..8 {
            wr::<u16>(nr, 4 + 4 * i, 0xFFFF);
        }
    }
    wr::<GPtr<u8>>(this, 0x2C, nr);
    wr::<u16>(this, 0x34, 0xFFFF);
    wr::<u32>(this, 0x28, g::<u32>(EMPTY_NODE_ADDRESS).read());
    this
}

pub const HOOKS: &[HookReg] = &[HOOK_FOLLOW_NODE_ROUTE_CTOR];
