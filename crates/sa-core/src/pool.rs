//! `CPool<T, S>` as instantiated in gta_sa.exe 1.0 US for peds, vehicles and objects.
//!
//! Layout (`CPool<int32>` is 0x14 bytes, layouts.csv): `m_pObjects`, `m_byteMap` (one byte per slot:
//! bit 7 = free, bits 0..6 = reuse counter), `m_nSize`, `m_nFirstFree`, two bools. A slot is
//! `sizeof(S)`, the pool's widest type: CCopPed 0x7C4, CHeli 0xA18, CCutsceneObject 0x19C.
//!
//! The algorithms follow the original bytes, not gta-reversed's `Pool.h`, which differs (NOTSA fills,
//! `FindFreeSlot` from `m_nFirstFree` instead of `m_nFirstFree + 1`, bounds checks in `GetAtRef`, a
//! `FIX_BUGS` null check in `Delete`). Each difference followed is listed in `docs/arch/pilot-ports.md`.
//! The pure slot logic works on any [`ByteMap`] so the L1 oracle can drive it with emulator memory.

use crate::layouts::size;
use crate::mem::{GBool, GPtr, GameData};
use crate::{globals, hook, orig};

/// Slot storage of the ped pool (`CPool<CPed, CCopPed>`).
#[repr(C, align(4))]
#[derive(Clone, Copy)]
pub struct PedSlot(pub [u8; size::CCopPed]);
/// Slot storage of the vehicle pool (`CPool<CVehicle, CHeli>`).
#[repr(C, align(4))]
#[derive(Clone, Copy)]
pub struct VehicleSlot(pub [u8; size::CHeli]);
/// Slot storage of the object pool (`CPool<CObject, CCutsceneObject>`).
#[repr(C, align(4))]
#[derive(Clone, Copy)]
pub struct ObjectSlot(pub [u8; size::CCutsceneObject]);

/// `CPool<T>` header, original layout on 32-bit targets.
#[repr(C)]
pub struct CPool<T: 'static> {
    /// +0x00 `m_pObjects`
    pub objects: GPtr<T>,
    /// +0x04 `m_byteMap`
    pub byte_map: GPtr<u8>,
    /// +0x08 `m_nSize`
    pub size: i32,
    /// +0x0C `m_nFirstFree` (the slot the last search ended at; the next one starts after it)
    pub first_free: i32,
    /// +0x10 `m_bOwnsAllocations`
    pub owns_arrays: GBool,
    /// +0x11 `m_bIsLocked` (gta-reversed `m_DealWithNoMemory`)
    pub locked: GBool,
    pub _pad: [u8; 2],
}

impl<T: 'static> Clone for CPool<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: 'static> Copy for CPool<T> {}
// SAFETY: game pointers, integers and bytes only.
#[allow(unsafe_code)]
unsafe impl<T: 'static> GameData for CPool<T> {}

/// Bit 7 of a byte-map entry: the slot is free.
pub const FREE: u8 = 0x80;

/// The slot byte map, addressed by a signed slot index (the original never bounds-checks it).
pub trait ByteMap {
    fn get(&self, i: i32) -> u8;
    fn set(&mut self, i: i32, v: u8);
}

impl ByteMap for GPtr<u8> {
    fn get(&self, i: i32) -> u8 {
        self.wrapping_index(i).read()
    }
    fn set(&mut self, i: i32, v: u8) {
        self.wrapping_index(i).write(v)
    }
}

/// `CPool::New` slot search (0x5E45E0 / 0x6E2A50 / 0x5A1C20). Returns the slot and the new
/// `m_nFirstFree`.
///
/// ```text
/// looped = 0
/// L: ++first_free (stored); if first_free == size { first_free = 0; if looped return null; looped = 1 }
///    if !(map[first_free] & 0x80) goto L
/// map[i] &= 0x7F; map[i] = (map[i] & 0x80) | ((map[i] + 1) & 0x7F)      ; xor-merge at +0x29
/// ```
/// The search starts *after* `m_nFirstFree`. A pool with `size == 0` and `first_free == -1` tests
/// `map[0]` (no bounds check), as the original does.
pub fn new_slot(map: &mut impl ByteMap, size: i32, first_free: i32) -> (Option<i32>, i32) {
    let mut ff = first_free;
    let mut looped = false;
    loop {
        ff = ff.wrapping_add(1);
        if ff == size {
            ff = 0;
            if looped {
                return (None, 0);
            }
            looped = true;
        }
        if map.get(ff) & FREE != 0 {
            break;
        }
    }
    let cleared = map.get(ff) & 0x7F;
    map.set(ff, cleared);
    let b = map.get(ff);
    map.set(ff, b ^ ((b.wrapping_add(1) ^ b) & 0x7F));
    (Some(ff), ff)
}

/// `CPool::GetAtRef` slot (0x404910 and the inlined copies in `CPools::Get*`): `idx = ref >> 8`
/// (arithmetic), valid iff `map[idx] == (uint8)ref`. No bounds check.
pub fn at_ref_slot(map: &impl ByteMap, handle: i32) -> Option<i32> {
    let idx = handle >> 8;
    (map.get(idx) == handle as u8).then_some(idx)
}

/// `CPool::GetAt` slot (0x156DE60): valid iff bit 7 of `map[idx]` is clear (`cmp byte,0; jns`).
pub fn at_slot(map: &impl ByteMap, idx: i32) -> Option<i32> {
    ((map.get(idx) as i8) >= 0).then_some(idx)
}

/// `CPool::GetRef` (inlined in `CPools::Get*Ref`): `(idx << 8) + map[idx]`.
pub fn ref_of_slot(map: &impl ByteMap, idx: i32) -> i32 {
    idx.wrapping_shl(8).wrapping_add(map.get(idx) as i32)
}

/// `CPool::Delete` (inlined in `CPed::operator delete`): `map[idx] |= 0x80`;
/// `if (idx < m_nFirstFree) m_nFirstFree = idx` (signed). Returns the new `m_nFirstFree`.
pub fn delete_slot(map: &mut impl ByteMap, first_free: i32, idx: i32) -> i32 {
    let b = map.get(idx);
    map.set(idx, b | FREE);
    if idx < first_free { idx } else { first_free }
}

fn first_free_field<T>(pool: GPtr<CPool<T>>) -> GPtr<i32> {
    pool.byte_add(core::mem::offset_of!(CPool<T>, first_free) as isize)
        .cast()
}

/// `CPool<T>::New()`.
pub fn pool_new<T: 'static>(pool: GPtr<CPool<T>>) -> GPtr<T> {
    let p = pool.read();
    let mut map = p.byte_map;
    let (slot, ff) = new_slot(&mut map, p.size, p.first_free);
    first_free_field(pool).write(ff);
    match slot {
        Some(i) => p.objects.wrapping_index(i),
        None => GPtr::NULL,
    }
}

/// `CPool<T>::GetAtRef(ref)`.
pub fn pool_at_ref<T: 'static>(pool: GPtr<CPool<T>>, handle: i32) -> GPtr<T> {
    let p = pool.read();
    match at_ref_slot(&p.byte_map, handle) {
        Some(i) => p.objects.wrapping_index(i),
        None => GPtr::NULL,
    }
}

/// `CPool<T>::GetAt(idx)`.
pub fn pool_at<T: 'static>(pool: GPtr<CPool<T>>, idx: i32) -> GPtr<T> {
    let p = pool.read();
    match at_slot(&p.byte_map, idx) {
        Some(i) => p.objects.wrapping_index(i),
        None => GPtr::NULL,
    }
}

/// `CPool<T>::GetRef(obj)`.
pub fn pool_ref<T: 'static>(pool: GPtr<CPool<T>>, obj: GPtr<T>) -> i32 {
    let p = pool.read();
    ref_of_slot(&p.byte_map, obj.index_from(p.objects))
}

/// `CPool<T>::Delete(obj)`. No null check (gta-reversed's is `FIX_BUGS`).
pub fn pool_delete<T: 'static>(pool: GPtr<CPool<T>>, obj: GPtr<T>) {
    let p = pool.read();
    let mut map = p.byte_map;
    let ff = delete_slot(&mut map, p.first_free, obj.index_from(p.objects));
    first_free_field(pool).write(ff);
}

globals! {
    list POOL_GLOBALS;
    /// `CPools::ms_pPedPool`
    pub static PED_POOL @ 0xB74490: GPtr<CPool<PedSlot>> = GPtr::NULL;
    /// `CPools::ms_pVehiclePool`
    pub static VEHICLE_POOL @ 0xB74494: GPtr<CPool<VehicleSlot>> = GPtr::NULL;
    /// `CPools::ms_pObjectPool`
    pub static OBJECT_POOL @ 0xB7449C: GPtr<CPool<ObjectSlot>> = GPtr::NULL;
}

/// Typed calls to the pool functions (original or Rust, decided by their hooks in phase A).
pub mod call {
    use super::*;
    orig! {
        /// `CPedPool::New` (0x5E45E0)
        pub fn ped_pool_new(pool: GPtr<CPool<PedSlot>>) -> GPtr<PedSlot> = thiscall 0x5E45E0 => super::ped_pool_new;
    }
}

// ------------------------------------------------------------------ hooked instantiations ------

/// `CPedPool::New` (0x5E45E0, thiscall).
#[hook(addr = 0x5E45E0, conv = thiscall, name = "CPedPool::New", expect = "8B 51 08 56 32 C0", this = "CPool<int32>")]
pub fn ped_pool_new(pool: GPtr<CPool<PedSlot>>) -> GPtr<PedSlot> {
    pool_new(pool)
}

/// `CVehiclePool::New` (0x6E2A50, thiscall).
#[hook(addr = 0x6E2A50, conv = thiscall, name = "CVehiclePool::New", expect = "8B 51 08 56 32 C0", this = "CPool<int32>")]
pub fn vehicle_pool_new(pool: GPtr<CPool<VehicleSlot>>) -> GPtr<VehicleSlot> {
    pool_new(pool)
}

/// `CObjectPool::New` (0x5A1C20, thiscall; its `m_nFirstFree = 0` store sits in a chunk at 0x405B24).
#[hook(addr = 0x5A1C20, conv = thiscall, name = "CObjectPool::New", expect = "8B 51 08 56 32 C0", this = "CPool<int32>")]
pub fn object_pool_new(pool: GPtr<CPool<ObjectSlot>>) -> GPtr<ObjectSlot> {
    pool_new(pool)
}

/// `CPedPool::GetAtRef` (0x404910, thiscall, `ret 4`).
#[hook(addr = 0x404910, conv = thiscall, name = "CPedPool::GetAtRef", expect = "8B 54 24 04 53", this = "CPool<int32>")]
pub fn ped_pool_at_ref(pool: GPtr<CPool<PedSlot>>, handle: i32) -> GPtr<PedSlot> {
    pool_at_ref(pool, handle)
}

/// `CObjectPool::GetAt` (stub 0x404870 → .HOODLUM 0x156DE60, thiscall, `ret 4`).
#[hook(addr = 0x404870, conv = thiscall, name = "CObjectPool::GetAt", expect = "E9 EB 95 16 01", this = "CPool<int32>")]
pub fn object_pool_at(pool: GPtr<CPool<ObjectSlot>>, idx: i32) -> GPtr<ObjectSlot> {
    pool_at(pool, idx)
}

/// `CPed::operator new(size)` (0x5E4720, cdecl): `mov ecx, ms_pPedPool; jmp CPedPool::New`.
#[hook(addr = 0x5E4720, conv = cdecl, name = "CPed::operator new-anon", expect = "8B 0D 90 44 B7 00")]
pub fn ped_operator_new(_size: u32) -> GPtr<PedSlot> {
    call::ped_pool_new(PED_POOL.get())
}

/// `CPed::operator delete(obj)` (0x5E4760, cdecl): `CPool::Delete` on `ms_pPedPool`, inlined.
#[hook(addr = 0x5E4760, conv = cdecl, name = "CPed::operator delete-anon", expect = "8B 4C 24 04 56")]
pub fn ped_operator_delete(obj: GPtr<PedSlot>) {
    pool_delete(PED_POOL.get(), obj)
}

/// `CPools::GetPedRef(ped)` (0x54FF60, cdecl).
#[hook(addr = 0x54FF60, conv = cdecl, name = "CPools::GetPedRef", expect = "8B 4C 24 04 56")]
pub fn get_ped_ref(ped: GPtr<PedSlot>) -> i32 {
    pool_ref(PED_POOL.get(), ped)
}

/// `CPools::GetPed(ref)` (0x54FF90, cdecl).
#[hook(addr = 0x54FF90, conv = cdecl, name = "CPools::GetPed", expect = "8B 4C 24 04 8B 15 90 44 B7 00")]
pub fn get_ped(handle: i32) -> GPtr<PedSlot> {
    pool_at_ref(PED_POOL.get(), handle)
}

/// `CPools::GetVehicleRef(vehicle)` (0x54FFC0, cdecl).
#[hook(addr = 0x54FFC0, conv = cdecl, name = "CPools::GetVehicleRef", expect = "8B 4C 24 04 56")]
pub fn get_vehicle_ref(vehicle: GPtr<VehicleSlot>) -> i32 {
    pool_ref(VEHICLE_POOL.get(), vehicle)
}

/// `CPools::GetVehicle(ref)` (0x54FFF0, cdecl).
#[hook(addr = 0x54FFF0, conv = cdecl, name = "CPools::GetVehicle", expect = "8B 4C 24 04 8B 15 94 44 B7 00")]
pub fn get_vehicle(handle: i32) -> GPtr<VehicleSlot> {
    pool_at_ref(VEHICLE_POOL.get(), handle)
}

/// `CPools::GetObjectRef(object)` (0x550020, cdecl).
#[hook(addr = 0x550020, conv = cdecl, name = "CPools::GetObjectRef", expect = "8B 4C 24 04 56")]
pub fn get_object_ref(object: GPtr<ObjectSlot>) -> i32 {
    pool_ref(OBJECT_POOL.get(), object)
}

/// `CPools::GetObject(ref)` (0x550050, cdecl).
#[hook(addr = 0x550050, conv = cdecl, name = "CPools::GetObject", expect = "8B 4C 24 04 8B 15 9C 44 B7 00")]
pub fn get_object(handle: i32) -> GPtr<ObjectSlot> {
    pool_at_ref(OBJECT_POOL.get(), handle)
}

// Field offsets read from the instantiations (0x5E45E0: `[ecx]` objects, `[ecx+4]` byte map,
// `[ecx+8]` size, `[ecx+0Ch]` first free; the bools follow, total 0x14 per layouts.csv).
#[cfg(target_pointer_width = "32")]
const _: () = {
    use core::mem::offset_of;
    assert!(offset_of!(CPool<PedSlot>, objects) == 0x00);
    assert!(offset_of!(CPool<PedSlot>, byte_map) == 0x04);
    assert!(offset_of!(CPool<PedSlot>, size) == 0x08);
    assert!(offset_of!(CPool<PedSlot>, first_free) == 0x0C);
    assert!(offset_of!(CPool<PedSlot>, owns_arrays) == 0x10);
    assert!(offset_of!(CPool<PedSlot>, locked) == 0x11);
};

// Host tests use Rust-owned memory; a phase A build cannot run them.
#[cfg(all(test, not(sa_phase_a)))]
mod tests {
    use super::*;
    use std::vec;
    use std::vec::Vec;

    /// A byte map over a slice, offset so that index `-off` is element 0.
    struct VecMap<'a>(&'a mut [u8], i32);
    impl ByteMap for VecMap<'_> {
        fn get(&self, i: i32) -> u8 {
            self.0[(i + self.1) as usize]
        }
        fn set(&mut self, i: i32, v: u8) {
            self.0[(i + self.1) as usize] = v
        }
    }

    #[test]
    fn new_searches_after_first_free_and_wraps_once() {
        let mut m = vec![0x80u8, 0x05, 0x80, 0x01];
        // search starts at first_free + 1 = 2
        assert_eq!(new_slot(&mut VecMap(&mut m, 0), 4, 1), (Some(2), 2));
        assert_eq!(m[2], 0x01);
        // from 2: 3 is used, wraps to 0
        assert_eq!(new_slot(&mut VecMap(&mut m, 0), 4, 2), (Some(0), 0));
        assert_eq!(m[0], 0x01);
        // full: null, first_free reset to 0
        assert_eq!(new_slot(&mut VecMap(&mut m, 0), 4, 0), (None, 0));
        // reuse counter wraps 0x7F -> 0 and keeps bit 7 clear
        let mut m = vec![0xFFu8];
        assert_eq!(new_slot(&mut VecMap(&mut m, 0), 1, -1), (Some(0), 0));
        assert_eq!(m[0], 0x00);
        // size 0, first_free -1: tests map[0] (no bounds check)
        let mut m = vec![0x80u8];
        assert_eq!(new_slot(&mut VecMap(&mut m, 0), 0, -1), (Some(0), 0));
    }

    #[test]
    fn refs_round_trip() {
        let mut m = vec![0x00u8, 0x13, 0x92, 0x7F];
        let map = VecMap(&mut m, 0);
        assert_eq!(ref_of_slot(&map, 1), 0x113);
        assert_eq!(at_ref_slot(&map, 0x113), Some(1));
        assert_eq!(at_ref_slot(&map, 0x114), None);
        // a free slot's byte has bit 7: its old ref no longer matches
        assert_eq!(at_ref_slot(&map, 0x212), None);
        assert_eq!(at_ref_slot(&map, 0x292), Some(2));
        assert_eq!(at_slot(&map, 2), None);
        assert_eq!(at_slot(&map, 3), Some(3));
        // negative refs index before the map (no bounds check)
        let mut m = vec![0x42u8, 0x00];
        assert_eq!(at_ref_slot(&VecMap(&mut m, 1), -0xBE), Some(-1));
    }

    #[test]
    fn delete_lowers_first_free_only() {
        let mut m = vec![0x01u8, 0x02, 0x03];
        assert_eq!(delete_slot(&mut VecMap(&mut m, 0), 2, 1), 1);
        assert_eq!(m[1], 0x82);
        assert_eq!(delete_slot(&mut VecMap(&mut m, 0), 0, 2), 0);
        assert_eq!(m[2], 0x83);
    }

    /// `CPools::Get*Ref` divide `(obj - objects)` by the slot size with a magic multiply
    /// (`imul magic; sar edx, k; edx += edx >> 31`). `GPtr::index_from` uses a plain signed division;
    /// both agree on every input checked (edges + 3M pseudo-random 32-bit differences per pool).
    #[test]
    fn magic_division_matches() {
        fn magic(x: i32, m: u32, k: u32) -> i32 {
            let hi = ((x as i64 * m as i32 as i64) >> 32) as i32;
            let q = hi >> k;
            q + ((q as u32) >> 31) as i32
        }
        let cases: [(u32, u32, i32); 3] = [
            (0x41EE7CA7, 9, 0x7C4),
            (0x06572EC3, 6, 0xA18),
            (0x13E22CBD, 5, 0x19C),
        ];
        let mut xs: Vec<i32> = vec![0, 1, -1, i32::MIN, i32::MAX, i32::MIN + 1];
        for (_, _, d) in cases {
            for k in -3..=3 {
                xs.push(d * 1000 + k);
                xs.push(-d * 1000 + k);
            }
        }
        let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
        for _ in 0..3_000_000 {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            xs.push(s as i32);
        }
        for (m, k, d) in cases {
            for &x in &xs {
                assert_eq!(magic(x, m, k), x / d, "x={x} d={d:#x}");
            }
        }
    }

    #[test]
    fn whole_pool_in_owned_memory() {
        let mut storage: Vec<ObjectSlot> = vec![ObjectSlot([0; size::CCutsceneObject]); 3];
        let mut map = vec![FREE; 3];
        let mut pool = CPool::<ObjectSlot> {
            objects: GPtr::from_raw(storage.as_mut_ptr()),
            byte_map: GPtr::from_raw(map.as_mut_ptr()),
            size: 3,
            first_free: -1,
            owns_arrays: GBool::TRUE,
            locked: GBool::FALSE,
            _pad: [0; 2],
        };
        let pp = GPtr::from_raw(&raw mut pool);
        let a = pool_new(pp);
        let b = pool_new(pp);
        assert_eq!(a.addr(), storage.as_ptr() as usize);
        assert_eq!(b.index_from(a), 1);
        let r = pool_ref(pp, b);
        assert_eq!(r, 0x101);
        assert_eq!(pool_at_ref(pp, r), b);
        pool_delete(pp, a);
        assert_eq!(pool.first_free, 0);
        assert_eq!(map[0], 0x81);
        assert_eq!(pool_at(GPtr::from_raw(&raw mut pool), 0), GPtr::NULL);
        assert_eq!(pool_at_ref(GPtr::from_raw(&raw mut pool), r), b);
    }
}
