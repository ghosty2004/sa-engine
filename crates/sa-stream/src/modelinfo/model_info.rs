//! `CModelInfo`: the model table lookups, the per-type stores (`CStore<T, N>`: `int32 count` then
//! `N` elements) with their static initialisers/destructors, `Initialise`/`ShutDown`, the vehicle
//! type queries.
//!
//! Ported from the 1.0 US bytes; gta-reversed `Models/ModelInfo.cpp` is a reading aid. Kept from the
//! original:
//! - `GetModelInfo(name, &idx)` searches upwards from `ms_lastPositionSearched` (read through the
//!   protection chunk 0x4014ED: `rol([0x1560644], [0x46706D] & 31)` = 0xAAE948), then downwards from
//!   it to 0, and stores the hit in `ms_lastPositionSearched`;
//! - `GetModelInfoUInt16` stores `(uint16)idx` with `idx` initialised to 0, so a name that is not
//!   found writes 0 (not -1) into the output;
//! - `Is*Model` re-read the table entry after the virtual `GetModelType`, return AL;
//!   `IsVehicleModelType` returns -1 for `id >= 20000` (signed: negative ids index before the table);
//! - `Initialise` fills `ms_modelInfoPtrs` with `rep stosd`, resets the store counts (atomic count
//!   to 1 directly) and creates the 8 "temp" atomic models 374–381 (`CTempColModels` col models
//!   0x968E50 + 0x30·k, txd "generic", draw distance 80.0);
//! - `ShutDown` calls each element's `Shutdown` through the vcall thunk 0x4C6200, re-reading every
//!   store's count per iteration, and frees the 2d-effect store's textures (lights) and road-sign
//!   text/atomics (type 7);
//! - the store destructors jump into the per-store helper (`eh vector destructor iterator` over the
//!   whole array, the count untouched; `docs/re/specs/STREAM/_static-init.md`).

use sa_core::abi::HookReg;
use core::sync::atomic::{AtomicUsize, Ordering};

use sa_core::mem::{GPtr, GameData};
use sa_core::{hook, orig};

use super::base::{f, vcall_get_model_type, vcall_init, MiPtr};
use crate::shared::{mi, model_info, model_info_slot, NUM_MODEL_INFOS};

/// `CStore<T, N>` with element size `S`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Store<const S: usize, const N: usize> {
    pub count: i32,
    pub items: [[u8; S]; N],
}
// SAFETY: an integer and bytes.
#[allow(unsafe_code)]
unsafe impl<const S: usize, const N: usize> GameData for Store<S, N> {}

impl<const S: usize, const N: usize> Store<S, N> {
    pub const ZERO: Store<S, N> = Store { count: 0, items: [[0; S]; N] };
}

sa_core::globals! {
    list GLOBALS;
    /// `CModelInfo::ms_lastPositionSearched`
    pub static LAST_POSITION_SEARCHED @ 0xAAE948: i32 = 0;
    /// `CModelInfo::ms_atomicModelInfoStore` (0x20 × 14000)
    pub static ATOMIC_STORE @ 0xAAE950: Store<0x20, 14000> = Store::ZERO;
    /// `CModelInfo::ms_damageAtomicModelInfoStore` (0x24 × 70)
    pub static DAMAGE_ATOMIC_STORE @ 0xB1BF58: Store<0x24, 70> = Store::ZERO;
    /// `CModelInfo::ms_lodAtomicModelInfoStore` (0x24 × 1)
    pub static LOD_ATOMIC_STORE @ 0xB1C934: Store<0x24, 1> = Store::ZERO;
    /// `CModelInfo::ms_timeModelInfoStore` (0x24 × 169)
    pub static TIME_STORE @ 0xB1C960: Store<0x24, 169> = Store::ZERO;
    /// `CModelInfo::ms_lodTimeModelInfoStore` (0x28 × 1)
    pub static LOD_TIME_STORE @ 0xB1E128: Store<0x28, 1> = Store::ZERO;
    /// `CModelInfo::ms_weaponModelInfoStore` (0x28 × 51)
    pub static WEAPON_STORE @ 0xB1E158: Store<0x28, 51> = Store::ZERO;
    /// `CModelInfo::ms_clumpModelInfoStore` (0x24 × 92)
    pub static CLUMP_STORE @ 0xB1E958: Store<0x24, 92> = Store::ZERO;
    /// `CModelInfo::ms_vehicleModelInfoStore` (0x308 × 212)
    pub static VEHICLE_STORE @ 0xB1F650: Store<0x308, 212> = Store::ZERO;
    /// `CModelInfo::ms_pedModelInfoStore` (0x44 × 278)
    pub static PED_STORE @ 0xB478F8: Store<0x44, 278> = Store::ZERO;
    /// `CModelInfo::ms_2dFXInfoStore` (`C2dEffect`, 0x40 × 100)
    pub static TWO_D_EFFECT_STORE @ 0xB4C2D8: Store<0x40, 100> = Store::ZERO;
    /// Protection constants of `GetModelInfo`'s chunk 0x4014ED (`.HOODLUM` dword, rotated left by
    /// the low byte of the `.text` dword below = 0xAAE948)
    pub static PROT_GMI_A @ 0x1560644: u32 = 0xE948_00AA;
    /// ... `.text` dword (rotate count)
    pub static PROT_GMI_B @ 0x46706D: u32 = 0xFFF9_B170;
}

/// One model-info store as `(count address, first element, element size)`.
#[derive(Clone, Copy)]
pub struct StoreRef {
    pub count: GPtr<i32>,
    pub items: GPtr<u8>,
    pub size: u32,
}

impl StoreRef {
    pub fn item(self, i: i32) -> MiPtr {
        self.items.byte_add((i as u32).wrapping_mul(self.size) as i32 as isize).cast()
    }
}

macro_rules! store_ref {
    ($g:ident, $s:expr) => {
        StoreRef { count: $g.ptr().cast(), items: $g.ptr().cast::<u8>().byte_add(4), size: $s }
    };
    ($g:ident, $s:expr, $raised:expr) => {{
        let p = $raised.get().map_or($g.ptr().cast::<u8>(), GPtr::from_addr);
        StoreRef { count: p.cast(), items: p.byte_add(4), size: $s }
    }};
}

pub fn atomic_store() -> StoreRef {
    store_ref!(ATOMIC_STORE, 0x20, RAISED_ATOMIC)
}
pub fn damage_atomic_store() -> StoreRef {
    store_ref!(DAMAGE_ATOMIC_STORE, 0x24, RAISED_DAMAGE_ATOMIC)
}
pub fn lod_atomic_store() -> StoreRef {
    store_ref!(LOD_ATOMIC_STORE, 0x24)
}
pub fn time_store() -> StoreRef {
    store_ref!(TIME_STORE, 0x24, RAISED_TIME)
}
pub fn lod_time_store() -> StoreRef {
    store_ref!(LOD_TIME_STORE, 0x28)
}
pub fn weapon_store() -> StoreRef {
    store_ref!(WEAPON_STORE, 0x28, RAISED_WEAPON)
}
pub fn clump_store() -> StoreRef {
    store_ref!(CLUMP_STORE, 0x24, RAISED_CLUMP)
}
pub fn vehicle_store() -> StoreRef {
    store_ref!(VEHICLE_STORE, 0x308, RAISED_VEHICLE)
}
pub fn ped_store() -> StoreRef {
    store_ref!(PED_STORE, 0x44, RAISED_PED)
}

/// Where a store a host made larger lives instead of its global (0: the global) and its capacity
/// ([`raise_capacity`]).
struct Raised {
    at: AtomicUsize,
    capacity: AtomicUsize,
}

impl Raised {
    const fn none() -> Raised {
        Raised { at: AtomicUsize::new(0), capacity: AtomicUsize::new(0) }
    }

    fn get(&self) -> Option<usize> {
        Some(self.at.load(Ordering::Relaxed)).filter(|&a| a != 0)
    }
}

static RAISED_ATOMIC: Raised = Raised::none();
static RAISED_DAMAGE_ATOMIC: Raised = Raised::none();
static RAISED_TIME: Raised = Raised::none();
static RAISED_WEAPON: Raised = Raised::none();
static RAISED_CLUMP: Raised = Raised::none();
static RAISED_VEHICLE: Raised = Raised::none();
static RAISED_PED: Raised = Raised::none();

/// The model-info stores a host can make larger, for the definitions its data adds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModelStore {
    /// `ms_atomicModelInfoStore`.
    Atomic,
    /// `ms_damageAtomicModelInfoStore`.
    DamageAtomic,
    /// `ms_timeModelInfoStore`.
    Time,
    /// `ms_weaponModelInfoStore`.
    Weapon,
    /// `ms_clumpModelInfoStore`.
    Clump,
    /// `ms_vehicleModelInfoStore`.
    Vehicle,
    /// `ms_pedModelInfoStore`.
    Ped,
}

impl ModelStore {
    /// The original store (address, element size, capacity) and its larger copy.
    fn layout(self) -> (GPtr<u8>, u32, u32, &'static Raised) {
        match self {
            ModelStore::Atomic => (ATOMIC_STORE.ptr().cast(), 0x20, 14000, &RAISED_ATOMIC),
            ModelStore::DamageAtomic => (DAMAGE_ATOMIC_STORE.ptr().cast(), 0x24, 70, &RAISED_DAMAGE_ATOMIC),
            ModelStore::Time => (TIME_STORE.ptr().cast(), 0x24, 169, &RAISED_TIME),
            ModelStore::Weapon => (WEAPON_STORE.ptr().cast(), 0x28, 51, &RAISED_WEAPON),
            ModelStore::Clump => (CLUMP_STORE.ptr().cast(), 0x24, 92, &RAISED_CLUMP),
            ModelStore::Vehicle => (VEHICLE_STORE.ptr().cast(), 0x308, 212, &RAISED_VEHICLE),
            ModelStore::Ped => (PED_STORE.ptr().cast(), 0x44, 278, &RAISED_PED),
        }
    }

    /// The number of elements the store holds.
    pub fn capacity(self) -> u32 {
        let (_, _, n, raised) = self.layout();
        raised.get().map_or(n, |_| raised.capacity.load(Ordering::Relaxed) as u32)
    }

    /// The number of elements in use.
    pub fn count(self) -> i32 {
        let (orig, _, _, raised) = self.layout();
        raised.get().map_or(orig, GPtr::from_addr).cast::<i32>().read()
    }
}

/// Makes `store` hold `extra` more elements than the original's: a copy of the store (its count and its
/// constructed elements, in memory from the game's `operator new`) with `extra` more elements carrying the class's
/// vtable, used in place of the global from then on. Call once per store, after the CRT's static initialisers
/// constructed the stores and before `CModelInfo::Initialise` fills them; the parity build never calls it. False
/// when the store was raised already or the memory is not there.
pub fn raise_capacity(store: ModelStore, extra: u32) -> bool {
    let (orig, size, n, raised) = store.layout();
    if raised.get().is_some() {
        return false;
    }
    let total = n + extra;
    let p = crate::txdstore::operator_new(4 + size * total);
    if p.is_null() {
        return false;
    }
    for k in 0..(4 + size * n) as isize {
        p.byte_add(k).write(orig.byte_add(k).read());
    }
    let vtable = orig.byte_add(4).cast::<u32>().read();
    for i in n..total {
        let e = p.byte_add((4 + size * i) as isize);
        for k in 0..size as isize {
            e.byte_add(k).write(0);
        }
        e.cast::<u32>().write(vtable);
    }
    raised.capacity.store(total as usize, Ordering::Relaxed);
    raised.at.store(p.addr(), Ordering::Relaxed);
    true
}

pub fn two_d_effect_store() -> StoreRef {
    store_ref!(TWO_D_EFFECT_STORE, 0x40)
}

/// The model-info stores in the order `ShutDown`/`Initialise` use.
pub fn model_stores() -> [StoreRef; 9] {
    [
        atomic_store(),
        damage_atomic_store(),
        lod_atomic_store(),
        time_store(),
        lod_time_store(),
        weapon_store(),
        clump_store(),
        vehicle_store(),
        ped_store(),
    ]
}

/// "generic" (0x858A34)
pub const STR_GENERIC: u32 = 0x858A34;
/// `CTempColModels::ms_colModelBBox`.. (0x968E50, eight col models 0x30 apart)
pub const TEMP_COL_MODELS: u32 = 0x968E50;
/// First of the eight temp atomic models `Initialise` creates.
pub const FIRST_TEMP_MODEL: i32 = 374;

pub use sa_core::bindings::rw_texture_destroy;
pub use sa_core::bindings::atexit;
pub use sa_core::bindings::memory_free as memory_mgr_free;
pub use sa_core::bindings::rp_atomic_destroy;
pub use sa_core::bindings::rp_atomic_set_frame;
pub use sa_core::bindings::rw_frame_destroy;
pub use sa_core::bindings::get_uppercase_key;
pub use sa_core::bindings::base_model_info_set_col_model as set_col_model;
pub use sa_core::bindings::get_model_info as get_model_info_call;
pub use crate::bindings::mi_set_tex_dictionary as set_tex_dictionary;
orig! {
    /// `CBaseModelInfo::Init2dEffects` (0x4C4C20)
    pub fn init_2d_effects(this: MiPtr) = thiscall 0x4C4C20 => super::base::init_2d_effects;
    /// `CBaseModelInfo` Shutdown vcall thunk (0x4C6200)
    pub fn vcall_shutdown_thunk(this: MiPtr) = thiscall 0x4C6200 => super::base::vcall_shutdown_thunk;
    /// Store constructors (`CStore<T, N>::CStore`, catalogue `FUN_*`, not STREAM rows)
    pub fn atomic_store_ctor(s: GPtr<u8>) -> GPtr<u8> = thiscall 0x4C5CB0 => unported;
    pub fn damage_atomic_store_ctor(s: GPtr<u8>) -> GPtr<u8> = thiscall 0x4C5D10 => unported;
    pub fn lod_atomic_store_ctor(s: GPtr<u8>) -> GPtr<u8> = thiscall 0x4C5D70 => unported;
    pub fn time_store_ctor(s: GPtr<u8>) -> GPtr<u8> = thiscall 0x4C5DD0 => unported;
    pub fn lod_time_store_ctor(s: GPtr<u8>) -> GPtr<u8> = thiscall 0x4C5E30 => unported;
    pub fn weapon_store_ctor(s: GPtr<u8>) -> GPtr<u8> = thiscall 0x4C5E90 => unported;
    pub fn clump_store_ctor(s: GPtr<u8>) -> GPtr<u8> = thiscall 0x4C5EF0 => unported;
    pub fn vehicle_store_ctor(s: GPtr<u8>) -> GPtr<u8> = thiscall 0x4C5F50 => unported;
    pub fn ped_store_ctor(s: GPtr<u8>) -> GPtr<u8> = thiscall 0x4C67D0 => unported;
    /// Store destructor helpers (`eh vector destructor iterator` over the elements)
    pub fn atomic_store_dtor(s: GPtr<u8>) = thiscall 0x4C5840 => crate::modelinfo::model_info::atomic_store_dtor_native;
    pub fn damage_atomic_store_dtor(s: GPtr<u8>) = thiscall 0x4C5860 => crate::modelinfo::model_info::damage_atomic_store_dtor_native;
    pub fn lod_atomic_store_dtor(s: GPtr<u8>) = thiscall 0x4C5880 => crate::modelinfo::model_info::lod_atomic_store_dtor_native;
    pub fn time_store_dtor(s: GPtr<u8>) = thiscall 0x4C58A0 => crate::modelinfo::model_info::time_store_dtor_native;
    pub fn lod_time_store_dtor(s: GPtr<u8>) = thiscall 0x4C58C0 => crate::modelinfo::model_info::lod_time_store_dtor_native;
    pub fn weapon_store_dtor(s: GPtr<u8>) = thiscall 0x4C58E0 => crate::modelinfo::model_info::weapon_store_dtor_native;
    pub fn clump_store_dtor(s: GPtr<u8>) = thiscall 0x4C5900 => crate::modelinfo::model_info::clump_store_dtor_native;
    pub fn vehicle_store_dtor(s: GPtr<u8>) = thiscall 0x4C6370 => crate::modelinfo::model_info::vehicle_store_dtor_native;
    pub fn ped_store_dtor(s: GPtr<u8>) = thiscall 0x4C6390 => crate::modelinfo::model_info::ped_store_dtor_native;
}

fn key_of(m: MiPtr) -> u32 {
    f::<u32>(m, mi::KEY).read()
}

/// The address `GetModelInfo`'s protection chunk computes (`rol(A, B & 31)`).
fn last_position_cell() -> GPtr<i32> {
    let a = PROT_GMI_A.get().rotate_left(PROT_GMI_B.get() & 31);
    GPtr::from_addr(a as usize)
}

/// `CModelInfo::GetModelInfo(const char*, int32*)` (0x4C5940, cdecl; entry `call
/// GetUppercaseKey; nop; jmp 0x4014ED`).
#[hook(addr = 0x4C5940, conv = cdecl, name = "CModelInfo::GetModelInfo-full", expect = "8B 44 24 04 56 50")]
pub fn get_model_info_full(name: GPtr<u8>, out: GPtr<i32>) -> MiPtr {
    let key = get_uppercase_key(name);
    let last = last_position_cell().read();
    let found = |i: i32| {
        let m = model_info(i);
        (!m.is_null() && key_of(m) == key).then_some(m)
    };
    let mut hit = None;
    let mut i = last;
    while i < NUM_MODEL_INFOS as i32 {
        if let Some(m) = found(i) {
            hit = Some((i, m));
            break;
        }
        i += 1;
    }
    if hit.is_none() {
        let mut i = last;
        while i >= 0 {
            if let Some(m) = found(i) {
                hit = Some((i, m));
                break;
            }
            i -= 1;
        }
    }
    let Some((i, m)) = hit else { return GPtr::NULL };
    if !out.is_null() {
        out.write(i);
    }
    LAST_POSITION_SEARCHED.set(i);
    m
}

/// `CModelInfo::GetModelInfo(const char*, int32 min, int32 max)` (0x4C5A20, cdecl): `min..=max`
/// (signed), no `ms_lastPositionSearched` update.
#[hook(addr = 0x4C5A20, conv = cdecl, name = "CModelInfo::GetModelInfo-minmax", expect = "8B 44 24 04 56 50")]
pub fn get_model_info_minmax(name: GPtr<u8>, min: i32, max: i32) -> MiPtr {
    let key = get_uppercase_key(name);
    let mut i = min;
    while i <= max {
        let m = model_info(i);
        if !m.is_null() && key_of(m) == key {
            return m;
        }
        i += 1;
    }
    GPtr::NULL
}

/// `CModelInfo::GetModelInfoFromHashKey(uint32, int32*)` (0x4C59B0, cdecl).
#[hook(addr = 0x4C59B0, conv = cdecl, name = "CModelInfo::GetModelInfoFromHashKey", expect = "8B 54 24 04 33 C9")]
pub fn get_model_info_from_hash_key(key: u32, out: GPtr<i32>) -> MiPtr {
    for i in 0..NUM_MODEL_INFOS as i32 {
        let m = model_info(i);
        if !m.is_null() && key_of(m) == key {
            if !out.is_null() {
                out.write(i);
            }
            return m;
        }
    }
    GPtr::NULL
}

/// `CModelInfo::GetModelInfoUInt16(const char*, uint16*)` (0x4C59F0, cdecl).
#[hook(addr = 0x4C59F0, conv = cdecl, name = "CModelInfo::GetModelInfoUInt16", expect = "51 8B 4C 24 08")]
pub fn get_model_info_uint16(name: GPtr<u8>, out: GPtr<u16>) -> MiPtr {
    let mut idx = 0i32;
    let m = get_model_info_call(name, GPtr::from_raw(&mut idx));
    if !out.is_null() {
        out.write(idx as u16);
    }
    m.cast::<crate::shared::CBaseModelInfo>()
}

/// `CModelInfo::Get2dEffectStore` (0x4C5A60): `&ms_2dFXInfoStore`.
#[hook(addr = 0x4C5A60, conv = cdecl, name = "CModelInfo::Get2dEffectStore", expect = "B8 D8 C2 B4 00")]
pub fn get_2d_effect_store() -> GPtr<u8> {
    TWO_D_EFFECT_STORE.ptr().cast()
}

/// `vehicle type == n` for a vehicle model info (`GetModelType() == 6`, type at +0x3C).
fn is_vehicle_type(id: i32, n: u32) -> bool {
    let m = model_info(id);
    if m.is_null() || vcall_get_model_type(m) != 6 {
        return false;
    }
    f::<u32>(model_info(id), 0x3C).read() == n
}

/// `CModelInfo::IsBoatModel` (0x4C5A70, cdecl, AL): vehicle type 5.
#[hook(addr = 0x4C5A70, conv = cdecl, name = "CModelInfo::IsBoatModel", expect = "56 8B 74 24 08")]
pub fn is_boat_model(id: i32) -> bool {
    is_vehicle_type(id, 5)
}

/// `CModelInfo::IsCarModel` (0x4C5AA0, cdecl, AL): vehicle type 0.
#[hook(addr = 0x4C5AA0, conv = cdecl, name = "CModelInfo::IsCarModel", expect = "56 8B 74 24 08")]
pub fn is_car_model(id: i32) -> bool {
    is_vehicle_type(id, 0)
}

/// `CModelInfo::IsTrainModel` (0x4C5AD0, cdecl, AL): vehicle type 6.
#[hook(addr = 0x4C5AD0, conv = cdecl, name = "CModelInfo::IsTrainModel", expect = "56 8B 74 24 08")]
pub fn is_train_model(id: i32) -> bool {
    is_vehicle_type(id, 6)
}

/// `CModelInfo::IsHeliModel` (0x4C5B00, cdecl, AL): vehicle type 3.
#[hook(addr = 0x4C5B00, conv = cdecl, name = "CModelInfo::IsHeliModel", expect = "56 8B 74 24 08")]
pub fn is_heli_model(id: i32) -> bool {
    is_vehicle_type(id, 3)
}

/// `CModelInfo::IsPlaneModel` (0x4C5B30, cdecl, AL): vehicle type 4.
#[hook(addr = 0x4C5B30, conv = cdecl, name = "CModelInfo::IsPlaneModel", expect = "56 8B 74 24 08")]
pub fn is_plane_model(id: i32) -> bool {
    is_vehicle_type(id, 4)
}

/// `CModelInfo::IsBikeModel` (0x4C5B60, cdecl, AL): vehicle type 9.
#[hook(addr = 0x4C5B60, conv = cdecl, name = "CModelInfo::IsBikeModel", expect = "56 8B 74 24 08")]
pub fn is_bike_model(id: i32) -> bool {
    is_vehicle_type(id, 9)
}

/// `CModelInfo::IsFakePlaneModel` (0x4C5B90, cdecl, AL): vehicle type 8.
#[hook(addr = 0x4C5B90, conv = cdecl, name = "CModelInfo::IsFakePlaneModel", expect = "56 8B 74 24 08")]
pub fn is_fake_plane_model(id: i32) -> bool {
    is_vehicle_type(id, 8)
}

/// `CModelInfo::IsMonsterTruckModel` (0x4C5BC0, cdecl, AL): vehicle type 1.
#[hook(addr = 0x4C5BC0, conv = cdecl, name = "CModelInfo::IsMonsterTruckModel", expect = "56 8B 74 24 08")]
pub fn is_monster_truck_model(id: i32) -> bool {
    is_vehicle_type(id, 1)
}

/// `CModelInfo::IsQuadBikeModel` (0x4C5BF0, cdecl, AL): vehicle type 2.
#[hook(addr = 0x4C5BF0, conv = cdecl, name = "CModelInfo::IsQuadBikeModel", expect = "56 8B 74 24 08")]
pub fn is_quad_bike_model(id: i32) -> bool {
    is_vehicle_type(id, 2)
}

/// `CModelInfo::IsBmxModel` (0x4C5C20, cdecl, AL): vehicle type 10.
#[hook(addr = 0x4C5C20, conv = cdecl, name = "CModelInfo::IsBmxModel", expect = "56 8B 74 24 08")]
pub fn is_bmx_model(id: i32) -> bool {
    is_vehicle_type(id, 10)
}

/// `CModelInfo::IsTrailerModel` (0x4C5C50, cdecl, AL): vehicle type 11.
#[hook(addr = 0x4C5C50, conv = cdecl, name = "CModelInfo::IsTrailerModel", expect = "56 8B 74 24 08")]
pub fn is_trailer_model(id: i32) -> bool {
    is_vehicle_type(id, 11)
}

/// `CModelInfo::IsVehicleModelType(int32)` (0x4C5C80, cdecl): the vehicle type, or -1.
#[hook(addr = 0x4C5C80, conv = cdecl, name = "CModelInfo::IsVehicleModelType", expect = "8B 44 24 04 3D 20 4E 00 00")]
pub fn is_vehicle_model_type(id: i32) -> i32 {
    if id >= NUM_MODEL_INFOS as i32 {
        return -1;
    }
    let m = model_info(id);
    if m.is_null() || vcall_get_model_type(m) != 6 {
        return -1;
    }
    f::<i32>(m, 0x3C).read()
}

/// The `Add*Model` sequence: `p = &items[count++]; p->Init(); ms_modelInfoPtrs[id] = p`.
fn add_model(s: StoreRef, id: i32) -> MiPtr {
    let n = s.count.read();
    let p = s.item(n);
    s.count.write(n.wrapping_add(1));
    vcall_init(p);
    model_info_slot(id).write(p);
    p
}

/// `CModelInfo::AddAtomicModel(int32)` (0x4C6620, cdecl).
#[hook(addr = 0x4C6620, conv = cdecl, name = "CModelInfo::AddAtomicModel", expect = "A1 50 E9 AA 00")]
pub fn add_atomic_model(id: i32) -> MiPtr {
    add_model(atomic_store(), id)
}

/// `CModelInfo::AddDamageAtomicModel(int32)` (0x4C6650, cdecl).
#[hook(addr = 0x4C6650, conv = cdecl, name = "CModelInfo::AddDamageAtomicModel", expect = "A1 58 BF B1 00")]
pub fn add_damage_atomic_model(id: i32) -> MiPtr {
    add_model(damage_atomic_store(), id)
}

/// `CModelInfo::AddLodAtomicModel(int32)` (0x4C6680, cdecl).
#[hook(addr = 0x4C6680, conv = cdecl, name = "CModelInfo::AddLodAtomicModel", expect = "A1 34 C9 B1 00")]
pub fn add_lod_atomic_model(id: i32) -> MiPtr {
    add_model(lod_atomic_store(), id)
}

/// `CModelInfo::AddTimeModel(int32)` (0x4C66B0, cdecl).
#[hook(addr = 0x4C66B0, conv = cdecl, name = "CModelInfo::AddTimeModel", expect = "A1 60 C9 B1 00")]
pub fn add_time_model(id: i32) -> MiPtr {
    add_model(time_store(), id)
}

/// `CModelInfo::AddLodTimeModel(int32)` (0x4C66E0, cdecl).
#[hook(addr = 0x4C66E0, conv = cdecl, name = "CModelInfo::AddLodTimeModel", expect = "A1 28 E1 B1 00")]
pub fn add_lod_time_model(id: i32) -> MiPtr {
    add_model(lod_time_store(), id)
}

/// `CModelInfo::AddWeaponModel(int32)` (0x4C6710, cdecl).
#[hook(addr = 0x4C6710, conv = cdecl, name = "CModelInfo::AddWeaponModel", expect = "A1 58 E1 B1 00")]
pub fn add_weapon_model(id: i32) -> MiPtr {
    add_model(weapon_store(), id)
}

/// `CModelInfo::AddClumpModel(int32)` (0x4C6740, cdecl).
#[hook(addr = 0x4C6740, conv = cdecl, name = "CModelInfo::AddClumpModel", expect = "A1 58 E9 B1 00")]
pub fn add_clump_model(id: i32) -> MiPtr {
    add_model(clump_store(), id)
}

/// `CModelInfo::AddVehicleModel(int32)` (0x4C6770, cdecl).
#[hook(addr = 0x4C6770, conv = cdecl, name = "CModelInfo::AddVehicleModel", expect = "A1 50 F6 B1 00")]
pub fn add_vehicle_model(id: i32) -> MiPtr {
    add_model(vehicle_store(), id)
}

/// `CModelInfo::AddPedModel(int32)` (0x4C67A0, cdecl).
#[hook(addr = 0x4C67A0, conv = cdecl, name = "CModelInfo::AddPedModel", expect = "A1 F8 78 B4 00")]
pub fn add_ped_model(id: i32) -> MiPtr {
    add_model(ped_store(), id)
}

/// `CModelInfo::Initialise` (0x4C6810).
#[hook(addr = 0x4C6810, conv = cdecl, name = "CModelInfo::Initialise", expect = "53 56 57 33 C0")]
pub fn initialise() {
    for i in 0..NUM_MODEL_INFOS as i32 {
        model_info_slot(i).write(GPtr::NULL);
    }
    for s in &model_stores()[1..] {
        s.count.write(0);
    }
    two_d_effect_store().count.write(0);
    let atoms = atomic_store();
    atoms.count.write(1);
    let mut p = atoms.item(0);
    for k in 0..8i32 {
        if k > 0 {
            let n = atoms.count.read();
            let next = atoms.item(n);
            if k == 1 {
                // element 0's draw distance is stored between element 1's index computation and
                // its count store
                f::<u32>(p, mi::DRAW_DISTANCE).write(0x42A0_0000);
            }
            atoms.count.write(n.wrapping_add(1));
            p = next;
        }
        vcall_init(p);
        model_info_slot(FIRST_TEMP_MODEL + k).write(p);
        set_col_model(p.cast::<crate::shared::CBaseModelInfo>().cast::<u8>(), GPtr::from_addr((TEMP_COL_MODELS + 0x30 * k as u32) as usize), 0);
        set_tex_dictionary(p.cast::<crate::shared::CBaseModelInfo>(), GPtr::from_addr(STR_GENERIC as usize));
        if k > 0 {
            f::<u32>(p, mi::DRAW_DISTANCE).write(0x42A0_0000);
        }
    }
}

/// `CModelInfo::ShutDown` (0x4C63E0).
#[hook(addr = 0x4C63E0, conv = cdecl, name = "CModelInfo::ShutDown", expect = "A1 50 E9 AA 00")]
pub fn shut_down() {
    for s in model_stores() {
        let mut i = 0;
        while i < s.count.read() {
            vcall_shutdown_thunk(s.item(i));
            i += 1;
        }
    }
    let fx = two_d_effect_store();
    let mut i = 0;
    while i < fx.count.read() {
        let e = fx.item(i).cast::<u8>();
        let ty = e.byte_add(0x0C).read();
        if ty == 0 {
            for off in [0x30isize, 0x34] {
                let t = e.byte_add(off).cast::<GPtr<u8>>();
                if !t.read().is_null() {
                    rw_texture_destroy(t.read());
                    t.write(GPtr::NULL);
                }
            }
        } else if ty == 7 {
            let text = e.byte_add(0x28).cast::<GPtr<u8>>();
            if !text.read().is_null() {
                memory_mgr_free(text.read());
                text.write(GPtr::NULL);
            }
            let ap = e.byte_add(0x2C).cast::<GPtr<u8>>();
            let atomic = ap.read();
            if !atomic.is_null() {
                let frame = atomic.byte_add(4).cast::<GPtr<u8>>().read();
                if !frame.is_null() {
                    rp_atomic_set_frame(atomic, GPtr::NULL);
                    rw_frame_destroy(frame);
                }
                rp_atomic_destroy(atomic);
                ap.write(GPtr::NULL);
            }
        }
        i += 1;
    }
    for s in model_stores() {
        s.count.write(0);
    }
    fx.count.write(0);
}

/// `CModelInfo::ReInit2dEffects` (0x4C63B0): 2d-effect store count 0, `Init2dEffects` on every
/// model info.
#[hook(addr = 0x4C63B0, conv = cdecl, name = "CModelInfo::ReInit2dEffects", expect = "56 C7 05 D8 C2 B4 00 00 00 00 00")]
pub fn re_init_2d_effects() {
    two_d_effect_store().count.write(0);
    for i in 0..NUM_MODEL_INFOS as i32 {
        let m = model_info(i);
        if !m.is_null() {
            init_2d_effects(m);
        }
    }
}

/// `__init_CModelInfo__ms_atomicModelInfoStore` (0x84BBF0, CRT initialiser): store constructor, then `_atexit(0x856230)`.
#[hook(addr = 0x84BBF0, conv = cdecl, name = "__init_CModelInfo__ms_atomicModelInfoStore", expect = "B9 50 E9 AA 00")]
pub fn init_atomic_store() {
    atomic_store_ctor(ATOMIC_STORE.ptr().cast());
    atexit(0x856230);
}

/// `__dtor_CModelInfo__ms_atomicModelInfoStore` (0x856230, atexit): `mov ecx, &store; jmp` the store's destructor helper.
#[hook(addr = 0x856230, conv = cdecl, name = "__dtor_CModelInfo__ms_atomicModelInfoStore", expect = "B9 50 E9 AA 00")]
pub fn dtor_atomic_store() {
    atomic_store_dtor(ATOMIC_STORE.ptr().cast());
}

/// `__init_CModelInfo__ms_damageAtomicModelInfoStore` (0x84BC10, CRT initialiser): store constructor, then `_atexit(0x856240)`.
#[hook(addr = 0x84BC10, conv = cdecl, name = "__init_CModelInfo__ms_damageAtomicModelInfoStore", expect = "B9 58 BF B1 00")]
pub fn init_damage_atomic_store() {
    damage_atomic_store_ctor(DAMAGE_ATOMIC_STORE.ptr().cast());
    atexit(0x856240);
}

/// `__dtor_CModelInfo__ms_damageAtomicModelInfoStore` (0x856240, atexit): `mov ecx, &store; jmp` the store's destructor helper.
#[hook(addr = 0x856240, conv = cdecl, name = "__dtor_CModelInfo__ms_damageAtomicModelInfoStore", expect = "B9 58 BF B1 00")]
pub fn dtor_damage_atomic_store() {
    damage_atomic_store_dtor(DAMAGE_ATOMIC_STORE.ptr().cast());
}

/// `__init_CModelInfo__ms_lodAtomicModelInfoStore` (0x84BC30, CRT initialiser): store constructor, then `_atexit(0x856250)`.
#[hook(addr = 0x84BC30, conv = cdecl, name = "__init_CModelInfo__ms_lodAtomicModelInfoStore", expect = "B9 34 C9 B1 00")]
pub fn init_lod_atomic_store() {
    lod_atomic_store_ctor(LOD_ATOMIC_STORE.ptr().cast());
    atexit(0x856250);
}

/// `__dtor_CModelInfo__ms_lodAtomicModelInfoStore` (0x856250, atexit): `mov ecx, &store; jmp` the store's destructor helper.
#[hook(addr = 0x856250, conv = cdecl, name = "__dtor_CModelInfo__ms_lodAtomicModelInfoStore", expect = "B9 34 C9 B1 00")]
pub fn dtor_lod_atomic_store() {
    lod_atomic_store_dtor(LOD_ATOMIC_STORE.ptr().cast());
}

/// `__init_CModelInfo__ms_timeModelInfoStore` (0x84BC50, CRT initialiser): store constructor, then `_atexit(0x856260)`.
#[hook(addr = 0x84BC50, conv = cdecl, name = "__init_CModelInfo__ms_timeModelInfoStore", expect = "B9 60 C9 B1 00")]
pub fn init_time_store() {
    time_store_ctor(TIME_STORE.ptr().cast());
    atexit(0x856260);
}

/// `__dtor_CModelInfo__ms_timeModelInfoStore` (0x856260, atexit): `mov ecx, &store; jmp` the store's destructor helper.
#[hook(addr = 0x856260, conv = cdecl, name = "__dtor_CModelInfo__ms_timeModelInfoStore", expect = "B9 60 C9 B1 00")]
pub fn dtor_time_store() {
    time_store_dtor(TIME_STORE.ptr().cast());
}

/// `__init_CModelInfo__ms_lodTimeModelInfoStore` (0x84BC70, CRT initialiser): store constructor, then `_atexit(0x856270)`.
#[hook(addr = 0x84BC70, conv = cdecl, name = "__init_CModelInfo__ms_lodTimeModelInfoStore", expect = "B9 28 E1 B1 00")]
pub fn init_lod_time_store() {
    lod_time_store_ctor(LOD_TIME_STORE.ptr().cast());
    atexit(0x856270);
}

/// `__dtor_CModelInfo__ms_lodTimeModelInfoStore` (0x856270, atexit): `mov ecx, &store; jmp` the store's destructor helper.
#[hook(addr = 0x856270, conv = cdecl, name = "__dtor_CModelInfo__ms_lodTimeModelInfoStore", expect = "B9 28 E1 B1 00")]
pub fn dtor_lod_time_store() {
    lod_time_store_dtor(LOD_TIME_STORE.ptr().cast());
}

/// `__init_CModelInfo__ms_weaponModelInfoStore` (0x84BC90, CRT initialiser): store constructor, then `_atexit(0x856280)`.
#[hook(addr = 0x84BC90, conv = cdecl, name = "__init_CModelInfo__ms_weaponModelInfoStore", expect = "B9 58 E1 B1 00")]
pub fn init_weapon_store() {
    weapon_store_ctor(WEAPON_STORE.ptr().cast());
    atexit(0x856280);
}

/// `__dtor_CModelInfo__ms_weaponModelInfoStore` (0x856280, atexit): `mov ecx, &store; jmp` the store's destructor helper.
#[hook(addr = 0x856280, conv = cdecl, name = "__dtor_CModelInfo__ms_weaponModelInfoStore", expect = "B9 58 E1 B1 00")]
pub fn dtor_weapon_store() {
    weapon_store_dtor(WEAPON_STORE.ptr().cast());
}

/// `__init_CModelInfo__ms_clumpModelInfoStore` (0x84BCB0, CRT initialiser): store constructor, then `_atexit(0x856290)`.
#[hook(addr = 0x84BCB0, conv = cdecl, name = "__init_CModelInfo__ms_clumpModelInfoStore", expect = "B9 58 E9 B1 00")]
pub fn init_clump_store() {
    clump_store_ctor(CLUMP_STORE.ptr().cast());
    atexit(0x856290);
}

/// `__dtor_CModelInfo__ms_clumpModelInfoStore` (0x856290, atexit): `mov ecx, &store; jmp` the store's destructor helper.
#[hook(addr = 0x856290, conv = cdecl, name = "__dtor_CModelInfo__ms_clumpModelInfoStore", expect = "B9 58 E9 B1 00")]
pub fn dtor_clump_store() {
    clump_store_dtor(CLUMP_STORE.ptr().cast());
}

/// `__init_CModelInfo__ms_vehicleModelInfoStore` (0x84BCD0, CRT initialiser): store constructor, then `_atexit(0x8562A0)`.
#[hook(addr = 0x84BCD0, conv = cdecl, name = "__init_CModelInfo__ms_vehicleModelInfoStore", expect = "B9 50 F6 B1 00")]
pub fn init_vehicle_store() {
    vehicle_store_ctor(VEHICLE_STORE.ptr().cast());
    atexit(0x8562A0);
}

/// `__dtor_CModelInfo__ms_vehicleModelInfoStore` (0x8562A0, atexit): `mov ecx, &store; jmp` the store's destructor helper.
#[hook(addr = 0x8562A0, conv = cdecl, name = "__dtor_CModelInfo__ms_vehicleModelInfoStore", expect = "B9 50 F6 B1 00")]
pub fn dtor_vehicle_store() {
    vehicle_store_dtor(VEHICLE_STORE.ptr().cast());
}

/// `__init_CModelInfo__ms_pedModelInfoStore` (0x84BCF0, CRT initialiser): store constructor, then `_atexit(0x8562B0)`.
#[hook(addr = 0x84BCF0, conv = cdecl, name = "__init_CModelInfo__ms_pedModelInfoStore", expect = "B9 F8 78 B4 00")]
pub fn init_ped_store() {
    ped_store_ctor(PED_STORE.ptr().cast());
    atexit(0x8562B0);
}

/// `__dtor_CModelInfo__ms_pedModelInfoStore` (0x8562B0, atexit): `mov ecx, &store; jmp` the store's destructor helper.
#[hook(addr = 0x8562B0, conv = cdecl, name = "__dtor_CModelInfo__ms_pedModelInfoStore", expect = "B9 F8 78 B4 00")]
pub fn dtor_ped_store() {
    ped_store_dtor(PED_STORE.ptr().cast());
}

/// Every `#[hook]` of this module, in source order.
pub const HOOKS: &[HookReg] = &[
    HOOK_GET_MODEL_INFO_FULL,
    HOOK_GET_MODEL_INFO_MINMAX,
    HOOK_GET_MODEL_INFO_FROM_HASH_KEY,
    HOOK_GET_MODEL_INFO_UINT16,
    HOOK_GET_2D_EFFECT_STORE,
    HOOK_IS_BOAT_MODEL,
    HOOK_IS_CAR_MODEL,
    HOOK_IS_TRAIN_MODEL,
    HOOK_IS_HELI_MODEL,
    HOOK_IS_PLANE_MODEL,
    HOOK_IS_BIKE_MODEL,
    HOOK_IS_FAKE_PLANE_MODEL,
    HOOK_IS_MONSTER_TRUCK_MODEL,
    HOOK_IS_QUAD_BIKE_MODEL,
    HOOK_IS_BMX_MODEL,
    HOOK_IS_TRAILER_MODEL,
    HOOK_IS_VEHICLE_MODEL_TYPE,
    HOOK_ADD_ATOMIC_MODEL,
    HOOK_ADD_DAMAGE_ATOMIC_MODEL,
    HOOK_ADD_LOD_ATOMIC_MODEL,
    HOOK_ADD_TIME_MODEL,
    HOOK_ADD_LOD_TIME_MODEL,
    HOOK_ADD_WEAPON_MODEL,
    HOOK_ADD_CLUMP_MODEL,
    HOOK_ADD_VEHICLE_MODEL,
    HOOK_ADD_PED_MODEL,
    HOOK_INITIALISE,
    HOOK_SHUT_DOWN,
    HOOK_RE_INIT_2D_EFFECTS,
    HOOK_INIT_ATOMIC_STORE,
    HOOK_DTOR_ATOMIC_STORE,
    HOOK_INIT_DAMAGE_ATOMIC_STORE,
    HOOK_DTOR_DAMAGE_ATOMIC_STORE,
    HOOK_INIT_LOD_ATOMIC_STORE,
    HOOK_DTOR_LOD_ATOMIC_STORE,
    HOOK_INIT_TIME_STORE,
    HOOK_DTOR_TIME_STORE,
    HOOK_INIT_LOD_TIME_STORE,
    HOOK_DTOR_LOD_TIME_STORE,
    HOOK_INIT_WEAPON_STORE,
    HOOK_DTOR_WEAPON_STORE,
    HOOK_INIT_CLUMP_STORE,
    HOOK_DTOR_CLUMP_STORE,
    HOOK_INIT_VEHICLE_STORE,
    HOOK_DTOR_VEHICLE_STORE,
    HOOK_INIT_PED_STORE,
    HOOK_DTOR_PED_STORE,
    HOOK_ATOMIC_STORE_DTOR_NATIVE,
    HOOK_DAMAGE_ATOMIC_STORE_DTOR_NATIVE,
    HOOK_LOD_ATOMIC_STORE_DTOR_NATIVE,
    HOOK_TIME_STORE_DTOR_NATIVE,
    HOOK_LOD_TIME_STORE_DTOR_NATIVE,
    HOOK_WEAPON_STORE_DTOR_NATIVE,
    HOOK_CLUMP_STORE_DTOR_NATIVE,
    HOOK_VEHICLE_STORE_DTOR_NATIVE,
    HOOK_PED_STORE_DTOR_NATIVE,
];

// ---- FRONTIER-2 lane C: the store destructor bodies, tail targets of their `__dtor_` stubs: phase B natives in turn 1, hooks
// since turn 3 (CAT-12 made them catalogue rows): `eh_vector_destructor_iterator(this + 4, size, count, dtor)`.

/// The `atomic` model-info store destructor body (0x4C5840, thiscall; reached by `jmp` from its `__dtor_` stub):
/// `eh_vector_destructor_iterator(this + 4, 0x20, 0x36B0, 0x4C5590)`.
#[hook(addr = 0x4C5840, conv = thiscall, name = "CModelInfo::ms_atomicModelInfoStore::~CStore", expect = "68 90 55 4C 00",
       unverified = "frontier2-c8", route = "split", hazards = "heap-writes",
       note = "FRONTIER-2 lane C turn 3: was a phase B native until the body became a catalogue row (CAT-12 tail target)")]
pub fn atomic_store_dtor_native(s: GPtr<u8>) {
    sa_core::bindings::eh_vector_dtor(s.byte_add(4), 0x20, 0x36B0, 0x4C5590);
}

/// The `damage_atomic` model-info store destructor body (0x4C5860, thiscall; reached by `jmp` from its `__dtor_` stub):
/// `eh_vector_destructor_iterator(this + 4, 0x24, 0x46, 0x4C55D0)`.
#[hook(addr = 0x4C5860, conv = thiscall, name = "CModelInfo::ms_damageAtomicModelInfoStore::~CStore", expect = "68 D0 55 4C 00",
       unverified = "frontier2-c8", route = "split", hazards = "heap-writes",
       note = "FRONTIER-2 lane C turn 3: was a phase B native until the body became a catalogue row (CAT-12 tail target)")]
pub fn damage_atomic_store_dtor_native(s: GPtr<u8>) {
    sa_core::bindings::eh_vector_dtor(s.byte_add(4), 0x24, 0x46, 0x4C55D0);
}

/// The `lod_atomic` model-info store destructor body (0x4C5880, thiscall; reached by `jmp` from its `__dtor_` stub):
/// `eh_vector_destructor_iterator(this + 4, 0x24, 0x1, 0x4C5620)`.
#[hook(addr = 0x4C5880, conv = thiscall, name = "CModelInfo::ms_lodAtomicModelInfoStore::~CStore", expect = "68 20 56 4C 00",
       unverified = "frontier2-c8", route = "split", hazards = "heap-writes",
       note = "FRONTIER-2 lane C turn 3: was a phase B native until the body became a catalogue row (CAT-12 tail target)")]
pub fn lod_atomic_store_dtor_native(s: GPtr<u8>) {
    sa_core::bindings::eh_vector_dtor(s.byte_add(4), 0x24, 0x1, 0x4C5620);
}

/// The `time` model-info store destructor body (0x4C58A0, thiscall; reached by `jmp` from its `__dtor_` stub):
/// `eh_vector_destructor_iterator(this + 4, 0x24, 0xA9, 0x4C5680)`.
#[hook(addr = 0x4C58A0, conv = thiscall, name = "CModelInfo::ms_timeModelInfoStore::~CStore", expect = "68 80 56 4C 00",
       unverified = "frontier2-c8", route = "split", hazards = "heap-writes",
       note = "FRONTIER-2 lane C turn 3: was a phase B native until the body became a catalogue row (CAT-12 tail target)")]
pub fn time_store_dtor_native(s: GPtr<u8>) {
    sa_core::bindings::eh_vector_dtor(s.byte_add(4), 0x24, 0xA9, 0x4C5680);
}

/// The `lod_time` model-info store destructor body (0x4C58C0, thiscall; reached by `jmp` from its `__dtor_` stub):
/// `eh_vector_destructor_iterator(this + 4, 0x28, 0x1, 0x4C56E0)`.
#[hook(addr = 0x4C58C0, conv = thiscall, name = "CModelInfo::ms_lodTimeModelInfoStore::~CStore", expect = "68 E0 56 4C 00",
       unverified = "frontier2-c8", route = "split", hazards = "heap-writes",
       note = "FRONTIER-2 lane C turn 3: was a phase B native until the body became a catalogue row (CAT-12 tail target)")]
pub fn lod_time_store_dtor_native(s: GPtr<u8>) {
    sa_core::bindings::eh_vector_dtor(s.byte_add(4), 0x28, 0x1, 0x4C56E0);
}

/// The `weapon` model-info store destructor body (0x4C58E0, thiscall; reached by `jmp` from its `__dtor_` stub):
/// `eh_vector_destructor_iterator(this + 4, 0x28, 0x33, 0x4C5790)`.
#[hook(addr = 0x4C58E0, conv = thiscall, name = "CModelInfo::ms_weaponModelInfoStore::~CStore", expect = "68 90 57 4C 00",
       unverified = "frontier2-c8", route = "split", hazards = "heap-writes",
       note = "FRONTIER-2 lane C turn 3: was a phase B native until the body became a catalogue row (CAT-12 tail target)")]
pub fn weapon_store_dtor_native(s: GPtr<u8>) {
    sa_core::bindings::eh_vector_dtor(s.byte_add(4), 0x28, 0x33, 0x4C5790);
}

/// The `clump` model-info store destructor body (0x4C5900, thiscall; reached by `jmp` from its `__dtor_` stub):
/// `eh_vector_destructor_iterator(this + 4, 0x24, 0x5C, 0x4C5750)`.
#[hook(addr = 0x4C5900, conv = thiscall, name = "CModelInfo::ms_clumpModelInfoStore::~CStore", expect = "68 50 57 4C 00",
       unverified = "frontier2-c8", route = "split", hazards = "heap-writes",
       note = "FRONTIER-2 lane C turn 3: was a phase B native until the body became a catalogue row (CAT-12 tail target)")]
pub fn clump_store_dtor_native(s: GPtr<u8>) {
    sa_core::bindings::eh_vector_dtor(s.byte_add(4), 0x24, 0x5C, 0x4C5750);
}

/// The `vehicle` model-info store destructor body (0x4C6370, thiscall; reached by `jmp` from its `__dtor_` stub):
/// `eh_vector_destructor_iterator(this + 4, 0x308, 0xD4, 0x4C5920)`.
#[hook(addr = 0x4C6370, conv = thiscall, name = "CModelInfo::ms_vehicleModelInfoStore::~CStore", expect = "68 20 59 4C 00",
       unverified = "frontier2-c8", route = "split", hazards = "heap-writes",
       note = "FRONTIER-2 lane C turn 3: was a phase B native until the body became a catalogue row (CAT-12 tail target)")]
pub fn vehicle_store_dtor_native(s: GPtr<u8>) {
    sa_core::bindings::eh_vector_dtor(s.byte_add(4), 0x308, 0xD4, 0x4C5920);
}

/// The `ped` model-info store destructor body (0x4C6390, thiscall; reached by `jmp` from its `__dtor_` stub):
/// `eh_vector_destructor_iterator(this + 4, 0x44, 0x116, 0x4C62F0)`.
#[hook(addr = 0x4C6390, conv = thiscall, name = "CModelInfo::ms_pedModelInfoStore::~CStore", expect = "68 F0 62 4C 00",
       unverified = "frontier2-c8", route = "split", hazards = "heap-writes",
       note = "FRONTIER-2 lane C turn 3: was a phase B native until the body became a catalogue row (CAT-12 tail target)")]
pub fn ped_store_dtor_native(s: GPtr<u8>) {
    sa_core::bindings::eh_vector_dtor(s.byte_add(4), 0x44, 0x116, 0x4C62F0);
}
