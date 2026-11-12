//! A ped's arrival at its attractor (batch `std13-ai-1`): `CPedAttractorManager::BroadcastArrival` (0x5EFE20, the
//! type dispatch), its template instance 0x5EF920, `CPedAttractor::BroadcastArrival` (0x5EEF80) and `GetQueueSlot`
//! (0x5EB550). Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CPedAttractorManager::IsPedRegisteredWithEffect(CPed*)` (0x5EBCB0, `ret 4`)
    fn is_ped_registered_with_effect(m: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x5EBCB0 => crate::attractor_registry::is_ped_registered_with_effect_ped;
    /// `CPedAttractor::GetType()` (vtable slot 0)
    fn attractor_get_type(a: GPtr<u8>) -> u32 = thiscall vslot 0x0 => unported;
    /// `CPedAttractor::IsAtHeadOfQueue(CPed*)` (vtable slot 8, `ret 4`)
    fn is_at_head_of_queue_v(a: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall vslot 0x8 => unported;
    /// `CPedAttractor::ComputeAttractPos(int, CVector&)` (vtable slot 0xC, `ret 8`)
    fn compute_attract_pos_v(a: GPtr<u8>, slot: i32, out: GPtr<u8>) = thiscall vslot 0xC => unported;
    /// `CPedAttractor::ComputeAttractHeading(int, float&)` (vtable slot 0x10, `ret 8`)
    fn compute_attract_heading_v(a: GPtr<u8>, slot: i32, out: GPtr<u8>) = thiscall vslot 0x10 => unported;
    /// `std::vector<CPed*>::push_back` (0x5EE200, `ret 4`)
    fn peds_push_back(v: GPtr<u8>, val: GPtr<u8>) = thiscall 0x5EE200 => crate::attractor_ptr_vectors::push_back_p;
    /// `std::vector<CPed*>::erase(iterator)` (0x5EC220, `ret 8`, the iterator result through the first argument)
    fn peds_erase(v: GPtr<u8>, out: GPtr<u8>, it: u32) -> GPtr<u8> = thiscall 0x5EC220 => unported;
    /// `CTaskComplexGoToAttractor` ctor (0x66B640, `ret 0x18`)
    fn go_to_attractor_ctor(t: GPtr<u8>, a: GPtr<u8>, pos: GPtr<u8>, heading: u32, time: u32, slot: i32, ms: u32) -> GPtr<u8> = thiscall 0x66B640 => unported;
    /// `CTaskComplexUsePairedAttractor(CPedAttractor*)` (0x6331A0, `ret 4`)
    fn use_paired_attractor_ctor(t: GPtr<u8>, a: GPtr<u8>) -> GPtr<u8> = thiscall 0x6331A0 => unported;
    /// `CTaskComplexUseAttractor(CPedAttractor*)` (0x6326F0, `ret 4`)
    fn use_attractor_ctor(t: GPtr<u8>, a: GPtr<u8>) -> GPtr<u8> = thiscall 0x6326F0 => unported;
    /// `CTaskComplexWaitAtAttractor(CPedAttractor*, int slot)` (0x6325F0, `ret 8`)
    fn wait_at_attractor_ctor(t: GPtr<u8>, a: GPtr<u8>, slot: i32) -> GPtr<u8> = thiscall 0x6325F0 => unported;
    /// `CPedAttractor::SetTaskForPed` (0x5EECA0, `ret 8`)
    fn set_task_for_ped(a: GPtr<u8>, ped: GPtr<u8>, task: GPtr<u8>) = thiscall 0x5EECA0 => crate::ped_attractor_base::set_task_for_ped;
    fn get_queue_slot_o(a: GPtr<u8>, ped: GPtr<u8>) -> i32 = thiscall 0x5EB550 => get_queue_slot;
    fn broadcast_arrival_o(a: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x5EEF80 => broadcast_arrival;
    fn broadcast_arrival_t_o(m: GPtr<u8>, ped: GPtr<u8>, a: GPtr<u8>, arr: GPtr<u8>) -> u8 = thiscall 0x5EF920 => broadcast_arrival_t;
}

fn pa<const N: usize>(v: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}

fn count4(a: GPtr<u8>, first: isize) -> i32 {
    let f = rd::<u32>(a, first);
    if f == 0 { 0 } else { (rd::<u32>(a, first + 4).wrapping_sub(f) as i32) >> 2 }
}

/// `CPedAttractor::GetQueueSlot(CPed*)` (0x5EB550, thiscall, `ret 4`): the ped's index in the queue (+0x20), -1 when
/// it is not there.
#[hook(addr = 0x5EB550, conv = thiscall, name = "CPedAttractor::GetQueueSlot", expect = "56 8B 71 20 83 C8 FF",
       unverified = "std13-ai-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn get_queue_slot(this: GPtr<u8>, ped: GPtr<u8>) -> i32 {
    let n = count4(this, 0x20);
    let first = rd::<u32>(this, 0x20);
    (0..n)
        .find(|&i| GPtr::<GPtr<u8>>::from_addr(first as usize + 4 * i as usize).read() == ped)
        .unwrap_or(-1)
}

/// `CPedAttractor::BroadcastArrival(CPed*)` (0x5EEF80, thiscall, `ret 4`, SEH frame). A ped not yet in the queue is
/// pushed to it and erased from the attracted list (`erase` 0x5EC220), and every attracted ped gets a
/// `CTaskComplexGoToAttractor(this, pos(n), heading(n), +0x44, n, walk)`, n = the queue length. Then the ped gets
/// `CTaskComplexUsePairedAttractor` (type 7) or `CTaskComplexUseAttractor` at the head of the queue, else
/// `CTaskComplexWaitAtAttractor(this, GetQueueSlot(ped))`. AL = 1.
#[hook(addr = 0x5EEF80, conv = thiscall, name = "CPedAttractor::BroadcastArrival", expect = "6A FF 64 A1 00 00 00 00",
       unverified = "std13-ai-1", route = "L2", hazards = "heap-writes,uninit-locals,stack-residue,ax-al-return",
       recs = "grove-walk", note = "the original pushes the ped from its own argument slot")]
pub fn broadcast_arrival(this: GPtr<u8>, ped: GPtr<u8>) -> u8 {
    let (mut p, end) = (rd::<u32>(this, 0x20), rd::<u32>(this, 0x24));
    let mut queued = false;
    while p != end {
        if GPtr::<GPtr<u8>>::from_addr(p as usize).read() == ped {
            queued = true;
            break;
        }
        p = p.wrapping_add(4);
    }
    if !queued {
        let mut v = [ped.addr() as u32];
        peds_push_back(this.byte_add(0x1C), pa(&mut v));
        let (mut p, end) = (rd::<u32>(this, 0x10), rd::<u32>(this, 0x14));
        while p != end {
            if GPtr::<GPtr<u8>>::from_addr(p as usize).read() == ped {
                let mut out = [0u32];
                peds_erase(this.byte_add(0xC), pa(&mut out), p);
                break;
            }
            p = p.wrapping_add(4);
        }
        let mut it = rd::<u32>(this, 0x10);
        while it != rd::<u32>(this, 0x14) {
            let other = GPtr::<GPtr<u8>>::from_addr(it as usize).read();
            it = it.wrapping_add(4);
            let n = count4(this, 0x20);
            let mut pos = [0u32; 3];
            compute_attract_pos_v(this, n, pa(&mut pos));
            let mut heading = [0u32; 1];
            compute_attract_heading_v(this, n, pa(&mut heading));
            let time = rd::<u32>(this, 0x44);
            let t = task_new(0x2C);
            let go = if t.is_null() {
                GPtr::NULL
            } else {
                go_to_attractor_ctor(t, this, pa(&mut pos), heading[0], time, n, 4)
            };
            set_task_for_ped(this, other, go);
        }
    }
    let task = if is_at_head_of_queue_v(this, ped) != 0 {
        if attractor_get_type(this) == 7 {
            let t = task_new(0x24);
            if t.is_null() { GPtr::NULL } else { use_paired_attractor_ctor(t, this) }
        } else {
            let t = task_new(0x10);
            if t.is_null() { GPtr::NULL } else { use_attractor_ctor(t, this) }
        }
    } else {
        let t = task_new(0x14);
        if t.is_null() { GPtr::NULL } else { wait_at_attractor_ctor(t, this, get_queue_slot_o(this, ped)) }
    };
    set_task_for_ped(this, ped, task);
    1
}

/// `CPedAttractorManager::BroadcastArrival<CPedAttractor>(CPed*, CPedAttractor*, SArray&)` (0x5EF920, thiscall,
/// `ret 0xC`): AL = 0 for a null attractor or one not in the vector (searched by count); else its
/// `BroadcastArrival(ped)`.
#[hook(addr = 0x5EF920, conv = thiscall, name = "CPedAttractorManager::BroadcastArrival<CPedAttractor>",
       expect = "57 8B 7C 24 0C 85 FF 75", unverified = "std13-ai-1", route = "L2", hazards = "ax-al-return",
       recs = "grove-walk")]
pub fn broadcast_arrival_t(_this: GPtr<u8>, ped: GPtr<u8>, a: GPtr<u8>, arr: GPtr<u8>) -> u8 {
    if a.is_null() {
        return 0;
    }
    let n = count4(arr, 4);
    let first = rd::<u32>(arr, 4);
    for i in 0..n {
        let c = GPtr::<GPtr<u8>>::from_addr(first as usize + 4 * i as usize).read();
        if c == a {
            return if c.is_null() { 0 } else { broadcast_arrival_o(c, ped) };
        }
    }
    0
}

/// `CPedAttractorManager::BroadcastArrival(CPed*, CPedAttractor*)` (0x5EFE20, thiscall, `ret 8`): AL = 0 for a null
/// attractor, a ped not registered with an effect (`IsPedRegisteredWithEffect`) or a type above 9; else the template
/// on the type's vector (type 1 at +0, type 0 at +0x10, type k at +0x10 * k).
#[hook(addr = 0x5EFE20, conv = thiscall, name = "CPedAttractorManager::BroadcastArrival-",
       expect = "56 57 8B 7C 24 10 85 FF", unverified = "std13-ai-1", route = "L2", hazards = "ax-al-return",
       recs = "grove-walk")]
pub fn manager_broadcast_arrival(this: GPtr<u8>, ped: GPtr<u8>, a: GPtr<u8>) -> u8 {
    if a.is_null() {
        return 0;
    }
    if is_ped_registered_with_effect(this, ped) == 0 {
        return 0;
    }
    let t = attractor_get_type(a);
    if t > 9 {
        return 0;
    }
    let arr = match t {
        0 => this.byte_add(0x10),
        1 => this,
        _ => this.byte_add(0x10 * t as isize),
    };
    broadcast_arrival_t_o(this, ped, a, arr)
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_GET_QUEUE_SLOT,
    HOOK_BROADCAST_ARRIVAL,
    HOOK_BROADCAST_ARRIVAL_T,
    HOOK_MANAGER_BROADCAST_ARRIVAL,
];
