//! `CCarCtrl` and `CCarAI` (PORT-POP), the leaf part. Ported from the 1.0 US bytes; gta-reversed
//! `CarCtrl.cpp`, `CarAI.cpp` and `docs/re/specs/POP/CCarCtrl*.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::hook;
use sa_core::mem::GPtr;
use sa_core::pools::NUM_RANDOM_CARS;
use sa_core::timer::{FRAME_COUNTER, TIME_MS};
use sa_math::x87::X87;

use crate::common::{find_player_coors, ftol32, jp5, rand_at, rand_frac, rd, wr, x, xi};
use crate::population::{
    CAR_GROUPS, LOADED_CAR_GROUPS, NUM_CARS_IN_GROUP, loaded_car_group_clear, loaded_gang_cars,
};

sa_core::globals! {
    list GLOBALS;
    /// `CCarCtrl::bSequenceOtherWay`
    pub static SEQUENCE_OTHER_WAY @ 0x969070: u8 = 0;
    /// `CCarCtrl::SequenceRandomOffset`
    pub static SEQUENCE_RANDOM_OFFSET @ 0x969074: i32 = 0;
    /// `CCarCtrl::SequenceElements`
    pub static SEQUENCE_ELEMENTS @ 0x969078: i32 = 0;
    /// `CCarCtrl::TimeOfVehiclesOfInterest[2]` (gr name)
    pub static TIME_OF_VEHICLES_OF_INTEREST @ 0x96907C: [u32; 2] = [0; 2];
    /// `CCarCtrl::apCarsToKeep[2]` (the vehicles of interest)
    pub static VEHICLES_OF_INTEREST @ 0x969084: [GPtr<u8>; 2] = [GPtr::NULL; 2];
    /// `NumLawEnforcerCars` 0x969098, `NumMissionCars` +4, `NumParkedCars` +8, `NumPermanentVehicles` +0xC,
    /// `NumAmbulancesOnDuty` +0x10, `NumFireTrucksOnDuty` +0x14, `LastTimeAmbulanceCreated` +0x18,
    /// `LastTimeFireTruckCreated` +0x1C, `LastTimeLawEnforcerCreated` +0x20 (the order the bytes use)
    pub static CAR_COUNTS @ 0x969098: [i32; 9] = [0; 9];
    /// `CCarCtrl::TimeNextMadDriverChaseCreated`
    pub static TIME_NEXT_MAD_DRIVER_CHASE_CREATED @ 0x9690BC: f32 = 0.0;
    /// `CCarCtrl::CountDownToCarsAtStart`
    pub static COUNT_DOWN_TO_CARS_AT_START @ 0x9690C0: u8 = 0;
    /// `CCarCtrl::CarDensityMultiplier`
    pub static CAR_DENSITY_MULTIPLIER @ 0x8A5B20: f32 = 1.0;
    /// `CCarCtrl::bAllowEmergencyServicesToBeCreated`
    pub static ALLOW_EMERGENCY_SERVICES @ 0x8A5B28: u8 = 1;
}

/// `CCarCtrl::bCarsGeneratedAroundCamera` (0x9690C1; read by sa-world's `FindPlayerCentreOfWorld` too)
const CARS_GENERATED_AROUND_CAMERA: usize = 0x9690C1;

fn count(i: i32) -> GPtr<i32> {
    CAR_COUNTS.ptr().cast::<i32>().wrapping_index(i)
}

fn interest(i: i32) -> GPtr<GPtr<u8>> {
    VEHICLES_OF_INTEREST.ptr().cast::<GPtr<u8>>().wrapping_index(i)
}

fn interest_time(i: i32) -> GPtr<u32> {
    TIME_OF_VEHICLES_OF_INTEREST.ptr().cast::<u32>().wrapping_index(i)
}

/// `CCarAI::CarHasReasonToStop(CVehicle*)` (stub 0x41C050 → 0x156FB90): autopilot +0x3AC = now.
/// Ported, unhookable: the stub bytes are read as data by protection code (`lea reg, [reg*4 + 0x41C050]`
/// at 0x14B2FB5, 0x14B2FC3, 0x14B5ED1, 0x14B5EDF; sa-hook `every_hook_installs_on_the_exe`).
pub fn car_has_reason_to_stop(veh: GPtr<u8>) {
    wr(veh, 0x3AC, TIME_MS.get());
}

/// `CCarCtrl::ClearInterestingVehicleList()` (stub 0x423F00).
#[hook(addr = 0x423F00, conv = cdecl, name = "CCarCtrl::ClearInterestingVehicleList", expect = "E9 9B 0A 14 01")]
pub fn clear_interesting_vehicle_list() {
    VEHICLES_OF_INTEREST.set([GPtr::NULL; 2]);
}

/// `CCarCtrl::IsThisVehicleInteresting(CVehicle*)` (0x423EA0).
#[hook(addr = 0x423EA0, conv = cdecl, name = "CCarCtrl::IsThisVehicleInteresting", expect = "8B 4C 24 04 33 C0")]
pub fn is_this_vehicle_interesting(veh: GPtr<u8>) -> u8 {
    for i in 0..2 {
        if interest(i).read() == veh {
            return 1;
        }
    }
    0
}

/// `CCarCtrl::RemoveFromInterestingVehicleList(CVehicle*)` (stub 0x423ED0).
#[hook(addr = 0x423ED0, conv = cdecl, name = "CCarCtrl::RemoveFromInterestingVehicleList", expect = "E9 6B D1 13 01")]
pub fn remove_from_interesting_vehicle_list(veh: GPtr<u8>) {
    for i in 0..2 {
        if interest(i).read() == veh {
            interest(i).write(GPtr::NULL);
        }
    }
}

/// `CCarCtrl::RegisterVehicleOfInterest(CVehicle*)` (0x423DE0): refresh the time of a listed vehicle,
/// else take the first free slot, else replace the oldest (unsigned `<`, first wins).
#[hook(addr = 0x423DE0, conv = cdecl, name = "CCarCtrl::RegisterVehicleOfInterest", expect = "53 8B 5C 24 08")]
pub fn register_vehicle_of_interest(veh: GPtr<u8>) {
    for i in 0..2 {
        if interest(i).read() == veh {
            interest_time(i).write(TIME_MS.get());
            return;
        }
    }
    for i in 0..2 {
        if interest(i).read().is_null() {
            interest(i).write(veh);
            interest_time(i).write(TIME_MS.get());
            return;
        }
    }
    let (mut best, mut min) = (0, u32::MAX);
    for i in 0..2 {
        if !interest(i).read().is_null() {
            let t = interest_time(i).read();
            if t < min {
                best = i;
                min = t;
            }
        }
    }
    interest(best).write(veh);
    interest_time(best).write(TIME_MS.get());
}

/// `CCarCtrl::PruneVehiclesOfInterest()` (0x423F10): on frames with `(counter & 0x3F) == 0x13`, unless the
/// player is at z >= 950, forgets vehicles registered more than 180 s ago.
#[hook(addr = 0x423F10, conv = cdecl, name = "CCarCtrl::PruneVehiclesOfInterest", expect = "A1 4C CB B7 00")]
pub fn prune_vehicles_of_interest() {
    if (FRAME_COUNTER.get() & 0x3F) as u8 != 0x13 {
        return;
    }
    let mut v = [0f32; 3];
    let p = find_player_coors(GPtr::from_raw(v.as_mut_ptr()), -1);
    if jp5(x(p.wrapping_index(2).read()), x(950.0)) {
        return;
    }
    let now = TIME_MS.get();
    for i in 0..2 {
        if !interest(i).read().is_null() && now > interest_time(i).read().wrapping_add(180_000) {
            interest(i).write(GPtr::NULL);
        }
    }
}

/// `CCarCtrl::InitSequence(int32 numElements)` (stub 0x421740 → 0x15616E0).
#[hook(addr = 0x421740, conv = cdecl, name = "CCarCtrl::InitSequence", expect = "E9 9B FF 13 01")]
pub fn init_sequence(n: i32) {
    SEQUENCE_ELEMENTS.set(n);
    let r = rand_at(0x15616EE);
    SEQUENCE_RANDOM_OFFSET.set(r.wrapping_rem(SEQUENCE_ELEMENTS.get()));
    let r = rand_at(0x1561700);
    SEQUENCE_OTHER_WAY.set(((r as u8) >> 4) & 1);
}

/// `CCarCtrl::FindSequenceElement(int32)` (stub 0x421770 → 0x1565130).
#[hook(addr = 0x421770, conv = cdecl, name = "CCarCtrl::FindSequenceElement", expect = "E9 BB 39 14 01")]
pub fn find_sequence_element(i: i32) -> i32 {
    let n = SEQUENCE_ELEMENTS.get();
    if SEQUENCE_OTHER_WAY.get() != 0 {
        i.wrapping_add(SEQUENCE_RANDOM_OFFSET.get()).wrapping_rem(n)
    } else {
        n.wrapping_sub(i)
            .wrapping_add(SEQUENCE_RANDOM_OFFSET.get())
            .wrapping_rem(n)
    }
}

/// `CCarCtrl::ThisVehicleShouldTryNotToTurn(CVehicle*)` (0x421FE0): models 403, 406, 431, 437, 443, 455,
/// 514, 515, 524 (byte table 0x422010).
#[hook(addr = 0x421FE0, conv = cdecl, name = "CCarCtrl::ThisVehicleShouldTryNotToTurn", expect = "8B 44 24 04 0F BF 40 22")]
pub fn this_vehicle_should_try_not_to_turn(veh: GPtr<u8>) -> u8 {
    let m = i32::from(rd::<i16>(veh, 0x22));
    u8::from(matches!(m, 403 | 406 | 431 | 437 | 443 | 455 | 514 | 515 | 524))
}

/// `CCarCtrl::FindSpeedMultiplierWithSpeedFromNodes(int8)` (0x424130).
#[hook(addr = 0x424130, conv = cdecl, name = "CCarCtrl::FindSpeedMultiplierWithSpeedFromNodes", expect = "8A 44 24 04 3C FF")]
pub fn find_speed_multiplier_with_speed_from_nodes(s: u8) -> X87 {
    match s {
        0xFF => x(0.5),
        0 => x(f32::from_bits(0x3F26_6666)),
        2 => x(f32::from_bits(0x4013_3333)),
        _ => x(1.0),
    }
}

/// `CCarCtrl::SwitchVehicleToRealPhysics(CVehicle*)` (stub 0x423FC0 → 0x1569420).
#[hook(addr = 0x423FC0, conv = cdecl, name = "CCarCtrl::SwitchVehicleToRealPhysics", expect = "E9 5B 54 14 01")]
pub fn switch_vehicle_to_real_physics(veh: GPtr<u8>) {
    let b = rd::<u8>(veh, 0x36);
    wr(veh, 0x36, (b & 7) | 0x18);
    wr::<u8>(veh, 0x3BB, 0);
    wr(veh, 0x3AC, TIME_MS.get().wrapping_add(2000));
    wr(veh, 0x3B0, TIME_MS.get());
    wr::<u8>(veh, 0xB8, 0);
}

/// `CCarCtrl::UpdateCarCount(CVehicle*, bool remove)` (0x424000): by `m_nCreatedBy` (+0x4A4) 1 random
/// (and law enforcer with +0x428 bit 0), 2 mission (clears the law bit and its count on add), 3 parked,
/// 4 permanent; removing clamps every counter at 0.
#[hook(addr = 0x424000, conv = cdecl, name = "CCarCtrl::UpdateCarCount", expect = "8A 44 24 08 8B 4C 24 04")]
pub fn update_car_count(veh: GPtr<u8>, remove: u8) {
    let by = u32::from(rd::<u8>(veh, 0x4A4)).wrapping_sub(1);
    if by > 3 {
        return;
    }
    let law = count(0);
    let nrc = NUM_RANDOM_CARS.ptr();
    let dec = |c: GPtr<i32>| {
        let v = c.read().wrapping_sub(1);
        c.write(if v < 0 { 0 } else { v });
    };
    if remove == 0 {
        match by {
            0 => {
                if rd::<u8>(veh, 0x428) & 1 != 0 {
                    law.write(law.read().wrapping_add(1));
                }
                nrc.write(nrc.read().wrapping_add(1));
            }
            1 => {
                let f = rd::<u8>(veh, 0x428);
                if f & 1 != 0 {
                    wr(veh, 0x428, f & 0xFE);
                    law.write(law.read().wrapping_sub(1));
                }
                count(1).write(count(1).read().wrapping_add(1));
            }
            2 => count(2).write(count(2).read().wrapping_add(1)),
            _ => count(3).write(count(3).read().wrapping_add(1)),
        }
    } else {
        match by {
            0 => {
                if rd::<u8>(veh, 0x428) & 1 != 0 {
                    dec(law);
                }
                dec(nrc);
            }
            1 => dec(count(1)),
            2 => dec(count(2)),
            _ => dec(count(3)),
        }
    }
}

/// The car-group resets `Init` and `ReInit` share: the 11 gang groups (0xC0EA28, stride 0x2E, up to
/// 0xC0EC22), then appropriate, inappropriate and boats.
fn clear_loaded_groups() {
    for k in 0..11 {
        loaded_car_group_clear(loaded_gang_cars(k));
    }
    let base = LOADED_CAR_GROUPS.ptr().cast::<u8>();
    loaded_car_group_clear(base.byte_add(0x60).cast());
    loaded_car_group_clear(base.byte_add(0x30).cast());
    loaded_car_group_clear(base.cast());
}

/// `CCarCtrl::Init()` (0x4212E0).
#[hook(addr = 0x4212E0, conv = cdecl, name = "CCarCtrl::Init", expect = "51 53 33 DB 56")]
pub fn init() {
    CAR_DENSITY_MULTIPLIER.set(1.0);
    NUM_RANDOM_CARS.set(0);
    for i in 0..8 {
        count(i).write(0);
    }
    let r = rand_at(0x42132A);
    VEHICLES_OF_INTEREST.set([GPtr::NULL; 2]);
    ALLOW_EMERGENCY_SERVICES.set(1);
    crate::common::g::<u8>(CARS_GENERATED_AROUND_CAMERA).write(0);
    COUNT_DOWN_TO_CARS_AT_START.set(2);
    let t = xi(r)
        .fmul_m(f32::from_bits(0x3800_0100))
        .fmul_m(600.0)
        .fadd_m(600.0);
    TIME_NEXT_MAD_DRIVER_CHASE_CREATED.set(t.to_f32());
    clear_loaded_groups();
}

/// `CCarCtrl::ReInit()` (0x4213B0).
#[hook(addr = 0x4213B0, conv = cdecl, name = "CCarCtrl::ReInit", expect = "33 C0 A3 94 90 96 00")]
pub fn re_init() {
    NUM_RANDOM_CARS.set(0);
    for i in 0..6 {
        count(i).write(0);
    }
    count(8).write(0);
    interest(0).write(GPtr::NULL);
    CAR_DENSITY_MULTIPLIER.set(1.0);
    ALLOW_EMERGENCY_SERVICES.set(1);
    COUNT_DOWN_TO_CARS_AT_START.set(2);
    interest(1).write(GPtr::NULL);
    clear_loaded_groups();
}

/// `CCarCtrl::ChooseCarModelToLoad(int32 group)` (0x421900): up to 16 random picks
/// `m_CarGroups[group][_ftol(n * frac)]`; the first whose `CStreaming` load state is not 1 (loaded), else -1.
#[hook(addr = 0x421900, conv = cdecl, name = "CCarCtrl::ChooseCarModelToLoad", expect = "83 EC 08 55 56")]
pub fn choose_car_model_to_load(group: i32) -> i32 {
    let info = sa_stream::shared::INFO_FOR_MODEL.ptr().cast::<u8>();
    for _ in 0..16 {
        let n = i32::from(NUM_CARS_IN_GROUP.ptr().cast::<i16>().wrapping_index(group).read());
        let frac = rand_frac(rand_at(0x421922));
        let idx = ftol32(xi(n).fmul(frac));
        let mi = i32::from(
            CAR_GROUPS
                .ptr()
                .cast::<u16>()
                .wrapping_index(idx.wrapping_add(group.wrapping_mul(23)))
                .read(),
        );
        if rd::<u8>(info, (mi * 0x14 + 0x10) as isize) != 1 {
            return mi;
        }
    }
    -1
}

pub const HOOKS: &[HookReg] = &[
    HOOK_CLEAR_INTERESTING_VEHICLE_LIST,
    HOOK_IS_THIS_VEHICLE_INTERESTING,
    HOOK_REMOVE_FROM_INTERESTING_VEHICLE_LIST,
    HOOK_REGISTER_VEHICLE_OF_INTEREST,
    HOOK_PRUNE_VEHICLES_OF_INTEREST,
    HOOK_INIT_SEQUENCE,
    HOOK_FIND_SEQUENCE_ELEMENT,
    HOOK_THIS_VEHICLE_SHOULD_TRY_NOT_TO_TURN,
    HOOK_FIND_SPEED_MULTIPLIER_WITH_SPEED_FROM_NODES,
    HOOK_SWITCH_VEHICLE_TO_REAL_PHYSICS,
    HOOK_UPDATE_CAR_COUNT,
    HOOK_INIT,
    HOOK_RE_INIT,
    HOOK_CHOOSE_CAR_MODEL_TO_LOAD,
];
