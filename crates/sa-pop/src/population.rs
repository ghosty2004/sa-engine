//! `CPopulation` (PORT-POP), the leaf part. Ported from the 1.0 US bytes; gta-reversed `Population.cpp` and
//! `docs/re/specs/POP/CPopulation*.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;
use sa_stream::loaded_car_group::CLoadedCarGroup;

use crate::common::{jne_mask, jp5, model_info, rd, wr, x};

pub const CAR_GROUPS_N: usize = 34;
pub const PED_GROUPS_N: usize = 57;
/// `CLoadedCarGroup` stride inside `m_LoadedGangCars` (0x2E, packed)
pub const LOADED_GROUP_STRIDE: usize = 0x2E;

sa_core::globals! {
    list GLOBALS;
    /// `CPopulation::PedDensityMultiplier`
    pub static PED_DENSITY_MULTIPLIER @ 0x8D2530: f32 = 1.0;
    /// `CPopulation::m_AllRandomPedsThisType`
    pub static ALL_RANDOM_PEDS_THIS_TYPE @ 0x8D2534: i32 = -1;
    /// `CPopulation::NumberOfPedsInUseInterior`
    pub static NUMBER_OF_PEDS_IN_USE_INTERIOR @ 0x8D253C: u32 = 40;
    /// `CPopulation::m_TranslationArray` (`int32[33][3]`: the ped groups of the 18 popcycle groups and the gangs per world zone)
    pub static TRANSLATION_ARRAY @ 0x8D2540: [i32; 99] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 9, 9, 10, 10, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
        23, 24, 25, 26, 27, 28, 29, 29, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 39, 39, 40, 40, 40, 41, 41, 41,
        42, 42, 42, 43, 43, 43, 44, 44, 44, 45, 45, 45, 46, 46, 46, 47, 47, 47, 48, 48, 48, 49, 49, 49, 50, 50, 50,
        51, 51, 51, 52, 52, 52, 53, 53, 53, 54, 54, 54, 55, 55, 55, 56, 56, 56,
    ];
    /// `m_LoadedBoats` (0xC0E998), `m_InAppropriateLoadedCars` (0xC0E9C8), `m_AppropriateLoadedCars`
    /// (0xC0E9F8) and the 11 `m_LoadedGangCars` (0xC0EA28, stride 0x2E) up to `bZoneChangeHasHappened`
    pub static LOADED_CAR_GROUPS @ 0xC0E998: [u8; 0x28A] = [0; 0x28A];
    /// `CPopulation::bZoneChangeHasHappened`
    pub static ZONE_CHANGE_HAS_HAPPENED @ 0xC0EC22: u8 = 0;
    /// `CPopulation::m_CountDownToPedsAtStart`
    pub static COUNT_DOWN_TO_PEDS_AT_START @ 0xC0EC23: u8 = 0;
    /// `ms_nTotalMissionPeds` 0xC0EC24, `ms_nTotalPeds` +4, `ms_nTotalGangPeds` +8, `ms_nTotalCivPeds` +0xC,
    /// `ms_nTotalCarPassengerPeds` +0x10, `ms_nNumDealers` +0x14, `ms_nNumGang[10]` +0x18,
    /// `ms_nNumEmergency` +0x40, `ms_nNumCop` +0x44, `ms_nNumCivFemale` +0x48, `ms_nNumCivMale` +0x4C
    pub static PED_COUNTS @ 0xC0EC24: [u32; 20] = [0; 20];
    /// `CPopulation::m_nNumCarsInGroup`
    pub static NUM_CARS_IN_GROUP @ 0xC0EC78: [u16; CAR_GROUPS_N] = [0; CAR_GROUPS_N];
    /// `CPopulation::m_nNumPedsInGroup` (57 used, 3 padding words up to `m_CarGroups`)
    pub static NUM_PEDS_IN_GROUP @ 0xC0ECC0: [u16; 60] = [0; 60];
    /// `CPopulation::m_CarGroups` (`int16[34][23]`)
    pub static CAR_GROUPS @ 0xC0ED38: [u16; CAR_GROUPS_N * 23] = [0; CAR_GROUPS_N * 23];
    /// `CPopulation::m_PedGroups` (`int16[57][21]`)
    pub static PED_GROUPS @ 0xC0F358: [u16; PED_GROUPS_N * 21] = [0; PED_GROUPS_N * 21];
    /// `m_bDontCreateRandomGangMembers` 0xC0FCB2, `m_bOnlyCreateRandomGangMembers` +1,
    /// `m_bDontCreateRandomCops` +2, `m_bMoreCarsAndFewerPeds` +3, `bInPoliceStation` +4
    pub static POP_FLAGS @ 0xC0FCB2: [u8; 5] = [0; 5];
    /// `CPopulation::CurrentWorldZone`
    pub static CURRENT_WORLD_ZONE @ 0xC0FCBC: u32 = 0;
}

fn counter(off: usize) -> GPtr<u32> {
    PED_COUNTS.ptr().cast::<u8>().byte_add(off as isize).cast()
}

/// `CPopulation::m_LoadedGangCars[gang]`.
pub fn loaded_gang_cars(gang: i32) -> GPtr<CLoadedCarGroup> {
    LOADED_CAR_GROUPS
        .ptr()
        .cast::<u8>()
        .byte_add(0x90 + gang.wrapping_mul(LOADED_GROUP_STRIDE as i32) as isize)
        .cast()
}

orig! {
    /// `CLoadedCarGroup::PickRandomCar(bool, bool)` (0x611C50)
    pub fn pick_random_car(g: GPtr<CLoadedCarGroup>, a: u8, b: u8) -> i32 = thiscall 0x611C50 => sa_stream::loaded_car_group::pick_random_car;
    /// `CLoadedCarGroup::Clear()` (0x611B90)
    pub fn loaded_car_group_clear(g: GPtr<CLoadedCarGroup>) = thiscall 0x611B90 => sa_stream::loaded_car_group::clear;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    pub fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// virtual slot 0 (scalar deleting destructor) of a `CPed`
    pub fn delete_entity(e: GPtr<u8>, flags: i32) = thiscall vslot 0 => unported;
    /// `CPedGroupPlacer::PlaceGroup(ePedType, int32, const CVector&, ePedGroupDefaultTaskAllocatorType)` (0x5FD810)
    pub fn ped_group_placer_place_group(this: GPtr<u8>, ty: i32, n: i32, pos: GPtr<f32>, alloc: i32) -> u8 = thiscall 0x5FD810 => unported;
    /// `CPopulation::LoadPedGroups()` (0x5BCFE0)
    pub fn load_ped_groups() = cdecl 0x5BCFE0 => unported;
    /// `CPopulation::LoadCarGroups()` (0x5BD1A0)
    pub fn load_car_groups() = cdecl 0x5BD1A0 => unported;
}

/// `CPopulation::FindCarMultiplierMotorway()` (0x611B60): 1.7 with `m_bMoreCarsAndFewerPeds`, else 1.0.
#[hook(addr = 0x611B60, conv = cdecl, name = "CPopulation::FindCarMultiplierMotorway", expect = "A0 B5 FC C0 00")]
pub fn find_car_multiplier_motorway() -> X87 {
    if POP_FLAGS.get()[3] != 0 {
        x(f32::from_bits(0x3FD9_999A))
    } else {
        x(1.0)
    }
}

/// `CPopulation::FindDummyDistForModel(int32)` (0x610ED0): 750.0 for the two model indices at 0x8CD6AC /
/// 0x8CD6B0 (`ModelIndices`), else 80.0.
#[hook(addr = 0x610ED0, conv = cdecl, name = "CPopulation::FindDummyDistForModel", expect = "0F B7 0D AC D6 8C 00")]
pub fn find_dummy_dist_for_model(model: i32) -> X87 {
    let mi = sa_stream::modelinfo::model_indices::MODEL_INDICES_A
        .ptr()
        .cast::<u16>();
    if model == i32::from(mi.wrapping_index(220).read()) || model == i32::from(mi.wrapping_index(222).read())
    {
        x(750.0)
    } else {
        x(80.0)
    }
}

/// `CPopulation::IsSunbather(int32 modelIndex)` (0x611760): ped model info +0x2C is 0x26 or 0x27.
#[hook(addr = 0x611760, conv = cdecl, name = "CPopulation::IsSunbather-ModelID", expect = "8B 44 24 04 8B 04 85 C8 B0 A9 00")]
pub fn is_sunbather(model: i32) -> u8 {
    let v = rd::<i32>(model_info(model), 0x2C);
    u8::from(v == 0x26 || v == 0x27)
}

/// `CPopulation::DoesCarGroupHaveModelId(int32 group, int32 model)` (stub 0x406F50 → 0x15642E0).
#[hook(addr = 0x406F50, conv = cdecl, name = "CPopulation::DoesCarGroupHaveModelId", expect = "E9 8B D3 15 01")]
pub fn does_car_group_have_model_id(group: i32, model: i32) -> u8 {
    let n = i32::from(NUM_CARS_IN_GROUP.ptr().cast::<i16>().wrapping_index(group).read());
    let row = CAR_GROUPS
        .ptr()
        .cast::<u16>()
        .wrapping_index(group.wrapping_mul(23));
    let mut i = 0;
    while i < n {
        if i32::from(row.wrapping_index(i).read()) == model {
            return 1;
        }
        i += 1;
    }
    0
}

/// `CPopulation::FindPedRaceFromName(const char*)` (0x5B6D40): the first of the first two characters
/// (upper-cased if a..z) that is B (1), W (2), I or O (3), H (4); else 0 (jump table 0x5B6DA4/0x5B6D90).
#[hook(addr = 0x5B6D40, conv = cdecl, name = "CPopulation::FindPedRaceFromName", expect = "8B 54 24 04 33 C9")]
pub fn find_ped_race_from_name(name: GPtr<u8>) -> i32 {
    for i in 0..2 {
        let mut c = name.wrapping_index(i).read();
        if c.is_ascii_lowercase() {
            c = c.wrapping_add(0xE0);
        }
        match c {
            b'B' => return 1,
            b'W' => return 2,
            b'I' | b'O' => return 3,
            b'H' => return 4,
            _ => {}
        }
    }
    0
}

/// `CPopulation::PickGangCar(int32 gang)` (0x614490): `m_LoadedGangCars[gang].PickRandomCar(false, false)`.
#[hook(addr = 0x614490, conv = cdecl, name = "CPopulation::PickGangCar", expect = "8B 4C 24 04 6B C9 2E")]
pub fn pick_gang_car(gang: i32) -> i32 {
    pick_random_car(loaded_gang_cars(gang), 0, 0)
}

/// `CPopulation::RemovePed(CPed*)` (0x610F20): `CWorld::Remove`, then the scalar deleting destructor.
#[hook(addr = 0x610F20, conv = cdecl, name = "CPopulation::RemovePed", expect = "56 8B 74 24 08")]
pub fn remove_ped(ped: GPtr<u8>) {
    world_remove(ped);
    if !ped.is_null() {
        delete_entity(ped, 1);
    }
}

/// `CPopulation::PlaceGangMembers(ePedType, int32, const CVector&)` (0x6117D0): a local
/// `CPedGroupPlacer` (4 uninitialised bytes) `.PlaceGroup(type, n, pos, 5)`.
#[hook(addr = 0x6117D0, conv = cdecl, name = "CPopulation::PlaceGangMembers", expect = "51 8B 44 24 10")]
pub fn place_gang_members(ty: i32, n: i32, pos: GPtr<f32>) -> u8 {
    let mut placer = [0u8; 4];
    ped_group_placer_place_group(GPtr::from_raw(placer.as_mut_ptr()), ty, n, pos, 5)
}

/// `CPopulation::Initialise()` (0x610E10): clears the counters and flags, resets the .data defaults
/// (density 1.0, all-random type -1, `MaxNumberOfPedsInUse` 25, interior 40), loads the ped and car groups.
#[hook(addr = 0x610E10, conv = cdecl, name = "CPopulation::Initialise", expect = "33 D2 57 33 C0")]
pub fn initialise() {
    let mut c = PED_COUNTS.get();
    c[19] = 0;
    c[18] = 0;
    c[17] = 0;
    c[16] = 0;
    for v in &mut c[6..16] {
        *v = 0;
    }
    for v in &mut c[0..5] {
        *v = 0;
    }
    PED_COUNTS.set(c);
    COUNT_DOWN_TO_PEDS_AT_START.set(5);
    ZONE_CHANGE_HAS_HAPPENED.set(0);
    PED_DENSITY_MULTIPLIER.set(1.0);
    ALL_RANDOM_PEDS_THIS_TYPE.set(-1);
    sa_core::inifile::MAX_NUMBER_OF_PEDS_IN_USE.set(25);
    NUMBER_OF_PEDS_IN_USE_INTERIOR.set(40);
    let mut f = POP_FLAGS.get();
    f[4] = 0;
    f[0] = 0;
    f[1] = 0;
    f[2] = 0;
    f[3] = 0;
    POP_FLAGS.set(f);
    load_ped_groups();
    load_car_groups();
}

/// `CPopulation::UpdatePedCount(CPed*, uint8 type)` (0x611570): type 0 adds the ped once (flag bit 2 of
/// +0x474), any other value removes it; the counter follows the ped type (+0x598, jump tables
/// 0x611650/0x611634 and 0x611680/0x611664).
#[hook(addr = 0x611570, conv = cdecl, name = "CPopulation::UpdatePedCount", expect = "8A 44 24 08 8B 4C 24 04")]
pub fn update_ped_count(ped: GPtr<u8>, ty: u8) {
    let flags = rd::<u32>(ped, 0x474);
    let add = ty == 0;
    if add {
        if flags & 4 != 0 {
            return;
        }
        wr(ped, 0x474, flags | 4);
    } else {
        if flags & 4 == 0 {
            return;
        }
        wr(ped, 0x474, flags & !4);
    }
    let pt = rd::<i32>(ped, 0x598);
    let k = pt.wrapping_sub(4) as u32;
    if k > 0x12 {
        return;
    }
    const CASE: [u8; 19] = [0, 1, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 5, 5, 0, 6, 1];
    let cell = match CASE[k as usize] {
        0 => counter(0x4C),
        1 => counter(0x48),
        2 => counter(0x44),
        3 => PED_COUNTS
            .ptr()
            .cast::<u8>()
            .byte_add((pt.wrapping_mul(4) - 4) as isize)
            .cast(),
        4 => counter(0x14),
        5 => counter(0x40),
        _ => return,
    };
    let v = cell.read();
    cell.write(if add { v.wrapping_add(1) } else { v.wrapping_sub(1) });
}

orig! {
    /// `FindPlayerVehicle(int32, bool)` (0x56E0D0)
    pub fn find_player_vehicle(id: i32, remote: u8) -> GPtr<u8> = cdecl 0x56E0D0 => sa_world::world::cworld::find_player_vehicle_port;
}

/// `CPopulation::PedCreationDistMultiplier()` (0x6116C0): with a player vehicle,
/// `clamp(sqrt(vy*vy + vx*vx) - 0.1 + 1.0, 1.0, 1.5)` of its move speed (FindPlayerVehicle called five
/// times, as the original); else 1.0.
#[hook(addr = 0x6116C0, conv = cdecl, name = "CPopulation::PedCreationDistMultiplier", expect = "6A 00 6A FF E8 07 CA F5 FF")]
pub fn ped_creation_dist_multiplier() -> X87 {
    if find_player_vehicle(-1, 0).is_null() {
        return x(1.0);
    }
    let esi = find_player_vehicle(-1, 0);
    let edi = find_player_vehicle(-1, 0);
    let ebx = find_player_vehicle(-1, 0);
    let eax = find_player_vehicle(-1, 0);
    let y = x(rd::<f32>(eax, 0x48)).fmul_m(rd::<f32>(ebx, 0x48));
    let xx = x(rd::<f32>(edi, 0x44)).fmul_m(rd::<f32>(esi, 0x44));
    let v = y.fadd(xx).fsqrt().fsub_m(f32::from_bits(0x3DCC_CCCD)).fadd_m(1.0);
    // fld 1.0; fcomp st(1); test ah, 0x41; jne -> 1.0 <= v
    if !jne_mask(x(1.0), v, 0x41) {
        return x(1.0);
    }
    // fld 1.5; fcomp st(1); test ah, 5; jp -> 1.5 >= v: keep v
    if jp5(x(1.5), v) {
        return v;
    }
    x(1.5)
}

pub const HOOKS: &[HookReg] = &[
    HOOK_FIND_CAR_MULTIPLIER_MOTORWAY,
    HOOK_FIND_DUMMY_DIST_FOR_MODEL,
    HOOK_IS_SUNBATHER,
    HOOK_DOES_CAR_GROUP_HAVE_MODEL_ID,
    HOOK_FIND_PED_RACE_FROM_NAME,
    HOOK_PICK_GANG_CAR,
    HOOK_REMOVE_PED,
    HOOK_PLACE_GANG_MEMBERS,
    HOOK_INITIALISE,
    HOOK_UPDATE_PED_COUNT,
    HOOK_PED_CREATION_DIST_MULTIPLIER,
];
