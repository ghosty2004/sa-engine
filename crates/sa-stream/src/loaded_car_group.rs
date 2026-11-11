//! `CLoadedCarGroup` (0x611B90–0x611F1A): a list of up to 23 vehicle model ids (`int16`, -1 = unused).
//!
//! Ported from the 1.0 US bytes (docs/re/specs/STREAM/CLoadedCarGroup.deviations.md); gta-reversed's
//! `LoadedCarGroup.cpp` differs in these points (the original is kept):
//! - `PickRandomCar`: the pick is `_ftol((rand() & 0xFFFF) * (1/32768.f) * (float)sum)` (the
//!   `CGeneral::GetRandomNumberInRange<int>` formula, gr-deviations `LoadedCarGroup.cpp:61`); a zero
//!   frequency sum returns -1 before any `rand()` call (gr has no such test); the weight walk has no
//!   end bound (it would run past the list with negative frequencies, see `pick_random_car`).
//! - `RemoveMember` scans all 23 entries (not only the used ones) and always writes `m_models[22] = -1`
//!   when found (same effect as gr's `remove_first`).
//! - `SortBasedOnUsage` is a bubble sort (descending `m_nTimesUsed`, signed bytes) over the leading
//!   used entries; gr's `rng::sort` is not stable in the same way, so equal keys may end in a
//!   different order there.
//! - `PickLeastUsedModel`: the minimum is by (`m_nRefCount`, then `m_nTimesUsed`), first one wins;
//!   the result is -1 when the chosen model's `m_nTimesUsed > maxTimesUsed` (or the list is empty).

use sa_core::mem::{GPtr, GameData};
use sa_core::{crt, hook, orig};

use crate::shared::{field, model_info};

/// `CLoadedCarGroup` (0x2E): `int16 m_models[23]`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CLoadedCarGroup {
    pub models: [i16; 23],
}
// SAFETY: integers only.
#[allow(unsafe_code)]
unsafe impl GameData for CLoadedCarGroup {}
const _: () = assert!(core::mem::size_of::<CLoadedCarGroup>() == sa_core::layouts::size::CLoadedCarGroup);

pub const NUM_MODELS: i32 = 23;

/// Vehicle model info fields read here (`CVehicleModelInfo`).
const MI_REF_COUNT: isize = 0x08; // int16, compared signed
const MI_VEHICLE_CLASS: isize = 0x4D; // int8
const MI_TIMES_USED: isize = 0x50; // int8
const MI_FRQ: isize = 0x52; // int16

/// Return address of `call rand` in PickRandomCar (0x611D30 + 5).
pub const RAND_RET_PICK_RANDOM_CAR: u32 = 0x611D35;

sa_core::globals! {
    list GLOBALS;
}

orig! {
    /// `CTheScripts::HasCarModelBeenSuppressed(model)` (0x46A810, cdecl, AL)
    pub fn has_car_model_been_suppressed(model: i32) -> u8 = cdecl 0x46A810 => unported;
    /// `CTheScripts::HasVehicleModelBeenBlockedByScript(model)` (0x46A890, cdecl, AL)
    pub fn has_vehicle_model_been_blocked_by_script(model: i32) -> u8 = cdecl 0x46A890 => unported;
    /// `CStreaming::WeAreTryingToPhaseVehicleOut(model)` (0x407F80, cdecl, AL)
    pub fn we_are_trying_to_phase_vehicle_out(model: i32) -> u8 = cdecl 0x407F80 => unported;
}

fn slot(g: GPtr<CLoadedCarGroup>, i: i32) -> GPtr<i16> {
    g.cast::<i16>().wrapping_index(i)
}

fn mi_i16(model: i32, off: isize) -> i16 {
    field::<i16, _>(model_info(model), off).read()
}

fn mi_i8(model: i32, off: isize) -> i8 {
    field::<i8, _>(model_info(model), off).read()
}

/// `CLoadedCarGroup::Clear()` (0x611B90, thiscall): `rep stosd` of 11 dwords -1, then one word -1.
#[hook(addr = 0x611B90, conv = thiscall, name = "CLoadedCarGroup::Clear", expect = "57 8B F9 B9 0B 00 00 00", this = "CLoadedCarGroup")]
pub fn clear(g: GPtr<CLoadedCarGroup>) {
    g.write(CLoadedCarGroup { models: [-1; 23] });
}

/// `CLoadedCarGroup::AddMember(model)` (0x611BB0, thiscall, `ret 4`): stores the low word of the
/// argument into the first entry `< 0` (signed); nothing when all 23 are used.
/// Not hookable: the loop branches back into the entry bytes (`jl 0x611BB2`); phase B entry (STD-6).
#[sa_core::native(addr = 0x611BB0, conv = thiscall, name = "CLoadedCarGroup::AddMember")]
pub fn add_member(g: GPtr<CLoadedCarGroup>, model: i32) {
    for i in 0..NUM_MODELS {
        if slot(g, i).read() < 0 {
            slot(g, i).write(model as i16);
            return;
        }
    }
}

/// `CLoadedCarGroup::RemoveMember(model)` (0x611BD0, thiscall, `ret 4`).
#[hook(addr = 0x611BD0, conv = thiscall, name = "CLoadedCarGroup::RemoveMember", expect = "8B D1 8B 4C 24 04", this = "CLoadedCarGroup")]
pub fn remove_member(g: GPtr<CLoadedCarGroup>, model: i32) {
    for i in 0..NUM_MODELS {
        if slot(g, i).read() as i32 == model {
            if i < 22 {
                // rep movsw, ascending: entries i+1..=22 move down by one
                for k in i..22 {
                    let v = slot(g, k + 1).read();
                    slot(g, k).write(v);
                }
            }
            slot(g, 22).write(-1);
            return;
        }
    }
}

/// `CLoadedCarGroup::GetMember(i)` (0x611C20, thiscall, `ret 4`): `movsx` of entry `i` (no bounds
/// check, 32-bit wrapping index).
#[hook(addr = 0x611C20, conv = thiscall, name = "CLoadedCarGroup::GetMember", expect = "8B 44 24 04 0F BF 04 41", this = "CLoadedCarGroup")]
pub fn get_member(g: GPtr<CLoadedCarGroup>, i: i32) -> i32 {
    slot(g, i).read() as i32
}

/// `CLoadedCarGroup::CountMembers()` (0x611C30, thiscall): the number of leading entries `>= 0`.
/// Not hookable: the loop branches back into the entry bytes (`jl 0x611C34`); phase B entry (STD-6).
#[sa_core::native(addr = 0x611C30, conv = thiscall, name = "CLoadedCarGroup::CountMembers")]
pub fn count_members(g: GPtr<CLoadedCarGroup>) -> i32 {
    let mut n = 0;
    while n < NUM_MODELS && slot(g, n).read() >= 0 {
        n += 1;
    }
    n
}

/// The list PickRandomCar walks: the group itself or the filtered local copy.
enum Choices {
    Group(GPtr<CLoadedCarGroup>),
    Local([i16; 23]),
}

impl Choices {
    /// Entry `k`. The original's walk has no end bound; past the end it reads the memory after the
    /// group (group mode, reproduced through the pointer) or the stack after the local array (local
    /// mode: the 24th word is uninitialised and beyond it are other locals; ledger "original UB").
    /// This port reads -1 there; it can only happen with negative `m_nFrq` values.
    fn get(&self, k: i32) -> i32 {
        match self {
            Choices::Group(g) => slot(*g, k).read() as i32,
            Choices::Local(a) => a.get(k as usize).map_or(-1, |v| *v as i32),
        }
    }
}

/// `CLoadedCarGroup::PickRandomCar(bNotTooManyInTheWorld, bOnlyPickNormalCars)` (0x611C50, thiscall,
/// `ret 8`): a model id weighted by `m_nFrq`, or -1.
#[hook(addr = 0x611C50, conv = thiscall, name = "CLoadedCarGroup::PickRandomCar", expect = "83 EC 3C 55 8B D1", this = "CLoadedCarGroup")]
pub fn pick_random_car(g: GPtr<CLoadedCarGroup>, not_too_many_in_the_world: u8, only_pick_normal_cars: u8) -> i32 {
    let count = count_members(g);
    if count == 0 {
        return -1;
    }
    let mut sum: i32 = 0;
    let choices = if only_pick_normal_cars != 0 {
        let mut local = [-1i16; 23];
        let mut n = 0usize;
        for i in 0..count {
            let model = slot(g, i).read();
            let class = mi_i8(model as i32, MI_VEHICLE_CLASS) as i32;
            if class >= 0 && (class <= 2 || class == 8) {
                local[n] = model;
                sum = sum.wrapping_add(mi_i16(model as i32, MI_FRQ) as i32);
                n += 1;
            }
        }
        Choices::Local(local)
    } else {
        for i in 0..count {
            let model = slot(g, i).read() as i32;
            sum = sum.wrapping_add(mi_i16(model, MI_FRQ) as i32);
        }
        Choices::Group(g)
    };
    if sum == 0 {
        return -1;
    }
    for _try in 0..10 {
        let r = sa_core::orig::at_site(RAND_RET_PICK_RANDOM_CAR, crt::rand);
        let mut w = sa_math::general::get_random_number_in_range_i32(r, 0, sum);
        let mut k = 0;
        let mut model = choices.get(k);
        let mut frq = mi_i16(model, MI_FRQ) as i32;
        while w > frq {
            w = w.wrapping_sub(frq);
            k += 1;
            model = choices.get(k);
            frq = mi_i16(model, MI_FRQ) as i32;
        }
        if has_car_model_been_suppressed(model) == 0
            && has_vehicle_model_been_blocked_by_script(model) == 0
            && we_are_trying_to_phase_vehicle_out(model) == 0
            && (not_too_many_in_the_world == 0 || mi_i16(model, MI_REF_COUNT) < 3)
        {
            return model;
        }
    }
    -1
}

/// `CLoadedCarGroup::SortBasedOnUsage()` (0x611E10, thiscall): bubble sort of the leading used
/// entries, descending by `m_nTimesUsed` (signed byte compare; swap only when strictly lower).
#[hook(addr = 0x611E10, conv = thiscall, name = "CLoadedCarGroup::SortBasedOnUsage", expect = "83 EC 08 33 D2", this = "CLoadedCarGroup")]
pub fn sort_based_on_usage(g: GPtr<CLoadedCarGroup>) {
    let last = count_members(g) - 1;
    loop {
        let mut swapped = false;
        if last <= 0 {
            return;
        }
        for i in 0..last {
            let b = slot(g, i + 1).read();
            let a = slot(g, i).read();
            if mi_i8(a as i32, MI_TIMES_USED) < mi_i8(b as i32, MI_TIMES_USED) {
                slot(g, i).write(b);
                slot(g, i + 1).write(a);
                swapped = true;
            }
        }
        if !swapped {
            return;
        }
    }
}

/// `CLoadedCarGroup::PickLeastUsedModel(maxTimesUsed)` (0x611E90, thiscall, `ret 4`).
#[hook(addr = 0x611E90, conv = thiscall, name = "CLoadedCarGroup::PickLeastUsedModel", expect = "83 EC 08 55 56", this = "CLoadedCarGroup")]
pub fn pick_least_used_model(g: GPtr<CLoadedCarGroup>, max_times_used: i32) -> i32 {
    let count = count_members(g);
    let mut best = -1;
    let mut best_refs: i32 = 999;
    let mut best_used: i32 = 999;
    for i in 0..count {
        let model = slot(g, i).read() as i32;
        let refs = mi_i16(model, MI_REF_COUNT) as i32;
        if refs < best_refs || (refs == best_refs && (mi_i8(model, MI_TIMES_USED) as i32) < best_used) {
            best = model;
            best_refs = mi_i16(model, MI_REF_COUNT) as i32;
            best_used = mi_i8(model, MI_TIMES_USED) as i32;
        }
    }
    if best_used > max_times_used { -1 } else { best }
}

/// Every `#[hook]` of this module, in source order.
pub const HOOKS: &[sa_core::abi::HookReg] = &[
    HOOK_CLEAR,
    HOOK_REMOVE_MEMBER,
    HOOK_GET_MEMBER,
    HOOK_PICK_RANDOM_CAR,
    HOOK_SORT_BASED_ON_USAGE,
    HOOK_PICK_LEAST_USED_MODEL,
];

#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;

    fn group(v: &[i16]) -> CLoadedCarGroup {
        let mut g = CLoadedCarGroup { models: [-1; 23] };
        g.models[..v.len()].copy_from_slice(v);
        g
    }

    #[test]
    fn members() {
        let mut g = group(&[5, 6, 7]);
        let p = GPtr::from_raw(&raw mut g);
        assert_eq!(count_members(p), 3);
        add_member(p, 0x1_0009);
        assert_eq!(g.models[3], 9);
        remove_member(p, 6);
        assert_eq!(&g.models[..4], &[5, 7, 9, -1]);
        remove_member(p, 42);
        assert_eq!(get_member(p, 1), 7);
        clear(p);
        assert_eq!(g, CLoadedCarGroup { models: [-1; 23] });
    }

    #[test]
    fn remove_last_slot() {
        let mut g = CLoadedCarGroup { models: [3; 23] };
        g.models[22] = 4;
        remove_member(GPtr::from_raw(&raw mut g), 4);
        assert_eq!(g.models[22], -1);
        assert_eq!(g.models[21], 3);
    }
}
