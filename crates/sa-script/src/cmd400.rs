//! `CRunningScript::ProcessCommands400To499` (0x47D210): complete port (PORT-SCRIPT-2 partial, completed in
//! PORT-SCRIPT-3). Every jump-table case is in [`PORTED`]; the other opcodes of the range reach the
//! original default (-1). Cases follow the 1.0 US bytes and
//! `docs/re/specs/SCRIPT/commands/ProcessCommands400To499.md`.
//!
//! Divergence notes: the 8-byte text labels of 01E3..01E5 are read into a zeroed 16-byte buffer (the
//! original's `[esp+0x20]` is not cleared; an 8-character key would be followed by stale stack bytes in
//! `CText::Get`); the `setne dl` bool of 01E4/01E5 is pushed as 0/1 (the original's upper EDX bytes are
//! whatever CollectParameters left; the callees read the low byte). Dead stack writes not ported: the
//! SEH state of 01C8's stack task, the scratch flag bytes in the command-argument slot (01C1, 01F3).

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x0190, 0x0191, 0x01A1, 0x01A2, 0x01A3, 0x01A4, 0x01A5, 0x01A6, 0x01A7, 0x01A8, 0x01A9, 0x01AA, 0x01AB,
    0x01AC, 0x01AD, 0x01AE, 0x01AF, 0x01B0, 0x01B2, 0x01B4, 0x01B5, 0x01B6, 0x01B7, 0x01B9, 0x01BB, 0x01BC,
    0x01BD, 0x01C0, 0x01C1, 0x01C2, 0x01C3, 0x01C4, 0x01C5, 0x01C7, 0x01C8, 0x01E3, 0x01E4, 0x01E5, 0x01E7,
    0x01E8, 0x01E9, 0x01EA, 0x01EB, 0x01EC, 0x01F0, 0x01F3,
];

/// `TheText` (`CText`) 0xC1B340.
const THE_TEXT: usize = 0xC1B340;
/// `CTheScripts::bAddNextMessageToPreviousBriefs` 0xA4448B.
const ADD_NEXT_MESSAGE_TO_PREVIOUS_BRIEFS: usize = 0xA4448B;
/// `CPopulation::ms_nTotalMissionPeds` 0xC0EC24.
const TOTAL_MISSION_PEDS: usize = 0xC0EC24;
/// `.rdata` 0x858B3C: 3.0f
const K_3: u32 = 0x4040_0000;
/// `.rdata` 0x858C80: 5.0f
const K_5: u32 = 0x40A0_0000;

/// `CTheScripts::UpsideDownCars` (`CUpsideDownCarCheck`) 0xA4892C.
const UPSIDE_DOWN_CARS: usize = 0xA4892C;

/// `ThePaths` (`CPathFind`) 0x96F050.
const THE_PATHS: usize = 0x96F050;
/// `CCarCtrl::CarDensityMultiplier` 0x8A5B20 (written by address as the original does).
const CAR_DENSITY_MULTIPLIER: usize = 0x8A5B20;
/// The immediate 01B4 pushes as the radius: 10.0f (`push 0x41200000`).
const K_10: u32 = 0x4120_0000;

orig! {
    /// `CUpsideDownCarCheck::AddCarToCheck(int32)` (0x4638D0)
    fn add_car_to_check(c: GPtr<u8>, h: i32) = thiscall 0x4638D0 => unported;
    /// `CUpsideDownCarCheck::RemoveCarFromCheck(int32)` (0x463910)
    fn remove_car_from_check(c: GPtr<u8>, h: i32) = thiscall 0x463910 => unported;
    /// `CRunningScript::CharInAreaCheckCommand(int32 command)` (0x488B50)
    fn char_in_area_check_command(s: Scr, command: u32) = thiscall 0x488B50 => unported;
    /// `CRunningScript::CarInAreaCheckCommand(int32 command)` (0x488EC0)
    fn car_in_area_check_command(s: Scr, command: u32) = thiscall 0x488EC0 => unported;
    /// `CRunningScript::LocateCarCommand(int32 command)` (0x487A20)
    fn locate_car_command(s: Scr, command: u32) = thiscall 0x487A20 => unported;
    /// `CPlayerPed::GetWantedLevel()` (0x41BE60)
    fn get_wanted_level(p: GPtr<u8>) -> i32 = thiscall 0x41BE60 => unported;
    /// `CTheScripts::CleanUpThisPed(CPed*)` (0x486300)
    fn clean_up_this_ped(p: GPtr<u8>) = cdecl 0x486300 => unported;
    /// `CTheScripts::CleanUpThisVehicle(CVehicle*)` (0x486670)
    fn clean_up_this_vehicle(v: GPtr<u8>) = cdecl 0x486670 => unported;
    /// `CTheScripts::CleanUpThisObject(CObject*)` (0x4866C0)
    fn clean_up_this_object(o: GPtr<u8>) = cdecl 0x4866C0 => unported;
    /// `CPlayerInfo::MakePlayerSafe(bool, float radius)` (0x56E870)
    fn make_player_safe(info: GPtr<u8>, safe: u32, radius: f32) = thiscall 0x56E870 => unported;
    /// `CWeather::ForceWeather(int16)` (0x72A4E0)
    fn force_weather(w: u32) = cdecl 0x72A4E0 => unported;
    /// `CWeather::ForceWeatherNow(int16)` (0x72A4F0)
    fn force_weather_now(w: u32) = cdecl 0x72A4F0 => unported;
    /// `CWeather::ReleaseWeather()` (0x72A510)
    fn release_weather() = cdecl 0x72A510 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0), AL
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::SetCurrentWeapon(int32 slot)` (0x5E61F0)
    fn set_current_weapon_slot(p: GPtr<u8>, slot: i32) = thiscall 0x5E61F0 => unported;
    /// `CPathFind::SwitchRoadsOffInArea(float x1, float x2, float y1, float y2, float z1, float z2, bool
    /// switchOff, bool carPaths, bool forceToggle)` (0x452C80); the floats are passed as raw dwords
    fn switch_roads_off_in_area(
        p: GPtr<u8>, x1: u32, x2: u32, y1: u32, y2: u32, z1: u32, z2: u32, off: u32, cars: u32, force: u32
    ) = thiscall 0x452C80 => unported;
    /// `CWanted::SetMaximumWantedLevel(int32)` (0x561E70)
    fn set_maximum_wanted_level(l: i32) = cdecl 0x561E70 => unported;
    /// `CPed::GiveWeapon(eWeaponType, uint32 ammo, bool)` (0x5E6080), returns the weapon slot
    fn give_weapon(p: GPtr<u8>, ty: u32, ammo: u32, flag: u32) -> u32 = thiscall 0x5E6080 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    fn get_weapon_info(ty: u32, skill: u32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CPed::RemoveWeaponModel(int32 model)` (0x5E3990)
    fn remove_weapon_model(p: GPtr<u8>, model: u32) = thiscall 0x5E3990 => unported;
    /// `CEntity` virtual slot 14 (+0x38, `Teleport(CVector, bool)`): the vector by value (three words)
    fn entity_teleport(e: GPtr<u8>, x: u32, y: u32, z: u32, clear_orientation: u32) = thiscall vslot 0x38 => unported;
    /// `CTheScripts::ClearSpaceForMissionEntity(const CVector&, CEntity*)` (0x486B00)
    fn clear_space_for_mission_entity(pos: GPtr<u32>, e: GPtr<u8>) = cdecl 0x486B00 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), floats as raw dwords, ST0 (read as f64: exact)
    fn find_ground_z_raw(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CTheScripts::IsVehicleStopped(CVehicle*)` (0x4861F0), AL
    fn is_vehicle_stopped(v: GPtr<u8>) -> u8 = cdecl 0x4861F0 => unported;
    /// `CRunningScript::GetCorrectPedModelIndexForEmergencyServiceType(ePedType, int32*)` (0x464F50)
    fn get_correct_ped_model_index(s: Scr, ped_type: i32, model: GPtr<u32>) = thiscall 0x464F50 => unported;
    /// `CPed::operator new(uint32)` (0x5E4720)
    fn ped_new(size: u32) -> GPtr<u8> = cdecl 0x5E4720 => unported;
    /// `CCopPed::CCopPed(eCopType)` (0x5DDC60), returns this
    fn cop_ped_ctor(p: GPtr<u8>, cop_type: u32) -> GPtr<u8> = thiscall 0x5DDC60 => unported;
    /// `CCivilianPed::CCivilianPed(ePedType, uint32 model)` (0x5DDB70), returns this
    fn civilian_ped_ctor(p: GPtr<u8>, ped_type: u32, model: u32) -> GPtr<u8> = thiscall 0x5DDB70 => unported;
    /// `CEmergencyPed::CEmergencyPed(ePedType, uint32 model)` (0x5DE340), returns this
    fn emergency_ped_ctor(p: GPtr<u8>, ped_type: u32, model: u32) -> GPtr<u8> = thiscall 0x5DE340 => unported;
    /// `CPed::SetCharCreatedBy(uint8)` (0x5E47E0)
    fn set_char_created_by(p: GPtr<u8>, by: u32) = thiscall 0x5E47E0 => unported;
    /// `CCarEnterExit::ComputeTargetDoorToEnterAsPassenger(const CVehicle*, int32 seat)` (0x64F190)
    fn compute_target_door_to_enter_as_passenger(veh: GPtr<u8>, seat: i32) -> i32 = cdecl 0x64F190 => unported;
    /// `CTaskSimpleCarSetPedInAsPassenger::CTaskSimpleCarSetPedInAsPassenger(CVehicle*, int32 door,
    /// CTaskUtilityLineUpPedWithCar*)` (0x646FE0)
    fn set_ped_in_as_passenger_ctor(t: GPtr<u8>, veh: GPtr<u8>, door: i32, line_up: GPtr<u8>) -> GPtr<u8> = thiscall 0x646FE0 => unported;
    /// `CTaskSimpleCarSetPedInAsPassenger::ProcessPed(CPed*)` (0x64B5D0)
    fn set_ped_in_as_passenger_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x64B5D0 => unported;
    /// `CTaskSimpleCarSetPedInAsPassenger::~CTaskSimpleCarSetPedInAsPassenger()` (0x647080)
    fn set_ped_in_as_passenger_dtor(t: GPtr<u8>) = thiscall 0x647080 => unported;
    /// `CText::Get(const char* key)` (0x6A0050)
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CMessages::AddBigMessageWithNumber(const char*, uint32 time, uint16 style, int32 n1..n6)` (0x69E5F0)
    fn add_big_message_with_number(
        t: GPtr<u8>, time: u32, style: u32, n1: u32, n2: u32, n3: u32, n4: u32, n5: u32, n6: u32
    ) = cdecl 0x69E5F0 => unported;
    /// `CMessages::AddMessageWithNumber(const char*, uint32 time, uint16 flag, int32 n1..n6, bool)` (0x69E360)
    fn add_message_with_number(
        t: GPtr<u8>, time: u32, flag: u32, n1: u32, n2: u32, n3: u32, n4: u32, n5: u32, n6: u32, prev: u32
    ) = cdecl 0x69E360 => unported;
    /// `CMessages::AddMessageJumpQWithNumber(const char*, uint32 time, uint16 flag, int32 n1..n6, bool)` (0x69E4E0)
    fn add_message_jump_q_with_number(
        t: GPtr<u8>, time: u32, flag: u32, n1: u32, n2: u32, n3: u32, n4: u32, n5: u32, n6: u32, prev: u32
    ) = cdecl 0x69E4E0 => unported;
}

/// The 8-byte key of 01E3..01E5: `ReadTextLabelFromScript(buf, 8)`, `TheText.Get(buf)` (zeroed buffer,
/// see the module notes).
fn text_from_label(s: Scr) -> GPtr<u8> {
    let mut buf = [0u8; 16];
    let b = GPtr::from_raw(buf.as_mut_ptr());
    read_text_label(s, b, 8);
    text_get(GPtr::from_addr(THE_TEXT), b)
}

/// 01E4 PRINT_WITH_NUMBER (0x47DA71) / 01E5 PRINT_WITH_NUMBER_NOW (0x47DAEC): text, `CollectParameters(3)`
/// (number, time, flag); `add(text, P1, (uint16)P2, P0, -1 x5, bAddNextMessageToPreviousBriefs != 0)`;
/// then `bAddNextMessageToPreviousBriefs = 1`.
fn print_with_number(s: Scr, now: bool) {
    let t = text_from_label(s);
    collect(s, 3);
    let prev = GPtr::<u8>::from_addr(ADD_NEXT_MESSAGE_TO_PREVIOUS_BRIEFS);
    let b = u32::from(prev.read() != 0);
    let m = u32::MAX;
    if now {
        add_message_jump_q_with_number(t, sp(1), sp(2) & 0xFFFF, sp(0), m, m, m, m, m, b);
    } else {
        add_message_with_number(t, sp(1), sp(2) & 0xFFFF, sp(0), m, m, m, m, m, b);
    }
    prev.write(1);
}

/// 01B2 GIVE_WEAPON_TO_CHAR (0x47D307): no null check. `slot = ped->GiveWeapon(P1, P2, true)`; a player
/// stores the slot byte in `m_pPlayerData (+0x480)->m_nChosenWeapon (+0x20)`; any other ped
/// `SetCurrentWeapon(slot)` and, when in a vehicle (+0x46C bit 8 and `m_pVehicle` +0x58C), removes the
/// model of its active weapon (`GetWeaponInfo(m_aWeapons[(int8)m_nActiveWeaponSlot (+0x718)].m_eWeaponType
/// (+0x5A0 + 0x1C*i), 1)->m_nModelId1 (+0xC)`).
fn give_weapon_to_char(s: Scr) {
    collect(s, 3);
    let ped = ped_at_ref(pool(PED_POOL), spi(0));
    let slot = give_weapon(ped, sp(1), sp(2), 1);
    if ped_is_player(ped) != 0 {
        ped.byte_add(0x480).cast::<GPtr<u8>>().read().byte_add(0x20).write(slot as u8);
        return;
    }
    set_current_weapon_slot(ped, slot as i32);
    if ped.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0
        && !ped.byte_add(0x58C).cast::<GPtr<u8>>().read().is_null()
    {
        let i = i32::from(ped.byte_add(0x718).cast::<i8>().read());
        let ty = ped.byte_add(0x5A0 + i.wrapping_mul(0x1C) as isize).cast::<u32>().read();
        let model = get_weapon_info(ty, 1).byte_add(0xC).cast::<u32>().read();
        remove_weapon_model(ped, model);
    }
}

/// 01BC SET_OBJECT_COORDINATES (0x47D5C9): no null check. `fld z; fcomp -100.0` (z stays the raw dword
/// unless replaced): z = ground z when `z <= -100` (ordered); `obj->Teleport((x, y, z), false)`
/// (vslot 14), then `ClearSpaceForMissionEntity(&(x, y, z), obj)`.
fn set_object_coordinates(s: Scr) {
    collect(s, 4);
    let obj = object_at_ref(pool(OBJECT_POOL), spi(0));
    let (x, y) = (sp(1), sp(2));
    let mut z = sp(3);
    if !X87::from_f32(f(z)).fcom(X87::constant(K_MINUS_100)).test_parity(0x41) {
        z = X87::from_f64_exact(find_ground_z_raw(x, y)).to_f32().to_bits();
    }
    entity_teleport(obj, x, y, z, 0);
    let mut pos = [x, y, z];
    clear_space_for_mission_entity(GPtr::from_raw(pos.as_mut_ptr()), obj);
}

/// 01C8 CREATE_CHAR_AS_PASSENGER (0x47D845): P0 vehicle (no null check), P1 ped type, P2 model (fixed up
/// by `GetCorrectPedModelIndexForEmergencyServiceType`), P3 seat. Type 6 → `CCopPed(P2)`, 0x12/0x13 →
/// `CEmergencyPed(P1, P2)`, else `CCivilianPed(P1, P2)` (after `CPed::operator new` 0x7C4 / 0x79C; a
/// null allocation gives a null ped). `SetCharCreatedBy(2)`; +0x46C bit 31 cleared, and bit 13 too when
/// the vehicle's byte +0x429 has bit 1; door = 0, or `ComputeTargetDoorToEnterAsPassenger(veh, P3)` when
/// P3 >= 0; a stack `CTaskSimpleCarSetPedInAsPassenger(veh, door, nullptr)` with +0x1C = 1 after the
/// constructor, ProcessPed(ped), destructor; mission ped count + 1; CWorld::Add; P0 = handle;
/// StoreParameters(1); mission cleanup (type 2).
fn create_char_as_passenger(s: Scr) {
    collect(s, 4);
    let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
    get_correct_ped_model_index(s, spi(1), param(2));
    let ped = match spi(1) {
        6 => {
            let p = ped_new(0x7C4);
            if p.is_null() { GPtr::NULL } else { cop_ped_ctor(p, sp(2)) }
        }
        0x12 | 0x13 => {
            let p = ped_new(0x79C);
            if p.is_null() { GPtr::NULL } else { emergency_ped_ctor(p, sp(1), sp(2)) }
        }
        _ => {
            let p = ped_new(0x79C);
            if p.is_null() { GPtr::NULL } else { civilian_ped_ctor(p, sp(1), sp(2)) }
        }
    };
    set_char_created_by(ped, 2);
    let fl = ped.byte_add(0x46C).cast::<u32>();
    let v = fl.read() & 0x7FFF_FFFF;
    fl.write(v);
    if veh.byte_add(0x429).read() & 2 != 0 {
        fl.write(v & 0xFFFF_DFFF);
    }
    let seat = spi(3);
    let door = if seat >= 0 {
        compute_target_door_to_enter_as_passenger(veh, seat)
    } else {
        0
    };
    let mut task = [0u32; 8];
    let t = GPtr::from_raw(task.as_mut_ptr()).cast::<u8>();
    set_ped_in_as_passenger_ctor(t, veh, door, GPtr::NULL);
    t.byte_add(0x1C).write(1);
    set_ped_in_as_passenger_process_ped(t, ped);
    set_ped_in_as_passenger_dtor(t);
    let n = GPtr::<u32>::from_addr(TOTAL_MISSION_PEDS);
    n.write(n.read().wrapping_add(1));
    world_add(ped);
    let h = ped_get_ref(pool(PED_POOL), ped);
    set_sp(0, h as u32);
    store(s, 1);
    if uses_mission_cleanup(s) {
        mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), spi(0), 2);
    }
}

/// 01EC SET_CAR_HEAVY (0x47DDE6): null handle → nothing. P1 != 0: physical flag bit 0 set, mass (+0x8C)
/// = handling mass (+4) * 3.0, turn mass (+0x90) = handling turn mass (+0xC) * 5.0, buoyancy (+0xA0) =
/// handling buoyancy (+0x24) doubled (x87, PC 24); P1 == 0: bit 0 cleared, the three handling values
/// copied as dwords. Handling = `m_pHandlingData` (+0x384).
fn set_car_heavy(s: Scr) {
    collect(s, 2);
    let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
    if v.is_null() {
        return;
    }
    let fl = v.byte_add(0x40).cast::<u32>();
    let h = v.byte_add(0x384).cast::<GPtr<u8>>().read();
    let hf = |o: isize| h.byte_add(o).cast::<u32>().read();
    let mass = v.byte_add(0x8C).cast::<u32>();
    let turn = v.byte_add(0x90).cast::<u32>();
    let buoy = v.byte_add(0xA0).cast::<u32>();
    if sp(1) != 0 {
        fl.write(fl.read() | 1);
        mass.write(X87::from_f32(f(hf(4))).fmul_m(f(K_3)).to_f32().to_bits());
        turn.write(X87::from_f32(f(hf(0xC))).fmul_m(f(K_5)).to_f32().to_bits());
        buoy.write(X87::from_f32(f(hf(0x24))).fdouble().to_f32().to_bits());
    } else {
        fl.write(fl.read() & !1);
        mass.write(hf(4));
        turn.write(hf(0xC));
        buoy.write(hf(0x24));
    }
}

/// 01F3 IS_CAR_IN_AIR_PROPER (0x47DEC8): no null check. flag = 1, cleared when any of the first
/// `m_nNumEntitiesCollided` (byte +0xB9) entries of `m_apCollidedEntities` (+0xBC) is non-null with
/// entity type (byte +0x36 & 7) 1 (building) or 2 (vehicle).
fn is_car_in_air_proper(s: Scr) {
    collect(s, 1);
    let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
    let n = v.byte_add(0xB9).read();
    let mut b = true;
    for i in 0..isize::from(n) {
        let e = v.byte_add(0xBC + 4 * i).cast::<GPtr<u8>>().read();
        if !e.is_null() {
            let ty = e.byte_add(0x36).read() & 7;
            if ty == 1 || ty == 2 {
                b = false;
            }
        }
    }
    set_flag(s, b);
}

/// The min/max ordering of SWITCH_ROADS_ON/OFF: `fld a; fcomp b; fnstsw; test ah,0x41; jne keep`, i.e.
/// the two dwords are swapped when `a > b` (ordered; the `fld/fstp` copy of a non-NaN is exact).
#[inline]
fn order(a: u32, b: u32) -> (u32, u32) {
    if f(a) > f(b) { (b, a) } else { (a, b) }
}

/// 01E7 SWITCH_ROADS_ON (0x47DB67) / 01E8 SWITCH_ROADS_OFF (0x47DC4B): six floats, each axis ordered
/// min/max, `ThePaths.SwitchRoadsOffInArea(x1, x2, y1, y2, z1, z2, off, 1, 0)` with `off` = 0 / 1.
fn switch_roads(s: Scr, off: u32) {
    collect(s, 6);
    let (x1, x2) = order(sp(0), sp(3));
    let (y1, y2) = order(sp(1), sp(4));
    let (z1, z2) = order(sp(2), sp(5));
    switch_roads_off_in_area(GPtr::from_addr(THE_PATHS), x1, x2, y1, y2, z1, z2, off, 1, 0);
}

/// `CRunningScript::ProcessCommands400To499(int32 command)` (0x47D210, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x47D210, conv = thiscall, name = "CRunningScript::ProcessCommands400To499", expect = "64 A1 00 00 00 00", this = "CRunningScript")]
pub fn process_commands_400_to_499(s: Scr, command: u32) -> i8 {
    match command {
        // 0190 ADD_UPSIDEDOWN_CAR_CHECK (0x47D24E) / 0191 REMOVE_UPSIDEDOWN_CAR_CHECK (0x47D27E)
        0x0190 => {
            collect(s, 1);
            add_car_to_check(GPtr::from_addr(UPSIDE_DOWN_CARS), spi(0));
        }
        0x0191 => {
            collect(s, 1);
            remove_car_from_check(GPtr::from_addr(UPSIDE_DOWN_CARS), spi(0));
        }
        // 01A1..01AA IS_CHAR_(STOPPED_)IN_AREA_* (0x47D2AD): CharInAreaCheckCommand(command) (EAX still
        // holds the command from the prologue). gr: unimplemented (-1 without reading the operands).
        0x01A1..=0x01AA => char_in_area_check_command(s, command),
        // 01AB/01AC IS_CAR_STOPPED_IN_AREA_2D/3D (0x47D2CB): CarInAreaCheckCommand(command).
        0x01AB | 0x01AC => car_in_area_check_command(s, command),
        // 01AD..01B0 LOCATE_(STOPPED_)CAR_2D/3D (0x47D2E9): LocateCarCommand(command).
        0x01AD..=0x01B0 => locate_car_command(s, command),
        // 01B4 SET_PLAYER_CONTROL (0x47D3C8): `CWorld::Players[P0].MakePlayerSafe(P1 == 0, 10.0f)`.
        0x01B4 => {
            collect(s, 2);
            let info = player_info(sp(0));
            make_player_safe(info, u32::from(sp(1) == 0), f(K_10));
        }
        // 01B5 FORCE_WEATHER (0x47D42B) / 01B6 FORCE_WEATHER_NOW (0x47D45C): (u16)P0 zero-extended.
        0x01B5 => {
            collect(s, 1);
            force_weather(sp(0) & 0xFFFF);
        }
        0x01B6 => {
            collect(s, 1);
            force_weather_now(sp(0) & 0xFFFF);
        }
        // 01B7 RELEASE_WEATHER (0x47D48D)
        0x01B7 => release_weather(),
        // 01B9 SET_CURRENT_CHAR_WEAPON (0x47D4A8): no null check. P1 == 0: slot 0 directly. Otherwise
        // every one of the 13 weapon slots (`m_aWeapons` +0x5A0, 0x1C each) whose type equals P1 (P1
        // re-read each iteration) is selected in turn: a player gets `m_pPlayerData->m_nChosenWeapon`
        // (+0x480 -> byte +0x20) = slot, any other ped `SetCurrentWeapon(slot)`.
        0x01B9 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let select = |slot: u32| {
                if ped_is_player(ped) != 0 {
                    ped.byte_add(0x480)
                        .cast::<GPtr<u8>>()
                        .read()
                        .byte_add(0x20)
                        .write(slot as u8);
                } else {
                    set_current_weapon_slot(ped, slot as i32);
                }
            };
            if sp(1) == 0 {
                select(0);
            } else {
                for slot in 0..13u32 {
                    let ty = ped.byte_add(0x5A0 + 0x1C * slot as isize).cast::<u32>().read();
                    if ty == sp(1) {
                        select(slot);
                    }
                }
            }
        }
        // 01BB GET_OBJECT_COORDINATES (0x47D567): no null check; the position is `m_matrix->pos`
        // (+0x14 -> +0x30) or `m_placement.m_vPosn` (+4) without a matrix; 3 dwords stored.
        0x01BB => {
            collect(s, 1);
            let obj = object_at_ref(pool(OBJECT_POOL), spi(0));
            let m = obj.byte_add(0x14).cast::<GPtr<u8>>().read();
            let p = if m.is_null() {
                obj.byte_add(4)
            } else {
                m.byte_add(0x30)
            }
            .cast::<u32>();
            let (x, y, z) = (p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read());
            set_sp(0, x);
            set_sp(1, y);
            set_sp(2, z);
            store(s, 3);
        }
        // 01BD GET_GAME_TIMER (0x47D69E): P0 = CTimer::m_snTimeInMilliseconds; store tail 0x47DD97.
        0x01BD => {
            set_sp(0, sa_core::timer::TIME_MS.get());
            store(s, 1);
        }
        // 01C0 STORE_WANTED_LEVEL (0x47D6A8): P0 = CWorld::Players[P0].m_pPed->GetWantedLevel(); store
        // tail 0x47DD97.
        0x01C0 => {
            collect(s, 1);
            let ped = player_info(sp(0)).cast::<GPtr<u8>>().read();
            let w = get_wanted_level(ped);
            set_sp(0, w as u32);
            store(s, 1);
        }
        // 01C2 MARK_CHAR_AS_NO_LONGER_NEEDED (0x47D721), 01C3 MARK_CAR_AS_NO_LONGER_NEEDED (0x47D75A),
        // 01C4 MARK_OBJECT_AS_NO_LONGER_NEEDED (0x47D78B): CleanUpThisPed/Vehicle/Object(GetAtRef(P0)) (no
        // null check here), then, when the script uses the mission cleanup,
        // MissionCleanUp.RemoveEntityFromList(P0, 2 / 1 / 3) (P0 re-read).
        0x01C2 => {
            collect(s, 1);
            clean_up_this_ped(ped_at_ref(pool(PED_POOL), spi(0)));
            if uses_mission_cleanup(s) {
                mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), spi(0), 2);
            }
        }
        0x01C3 => {
            collect(s, 1);
            clean_up_this_vehicle(vehicle_at_ref(pool(VEHICLE_POOL), spi(0)));
            if uses_mission_cleanup(s) {
                mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), spi(0), 1);
            }
        }
        0x01C4 => {
            collect(s, 1);
            clean_up_this_object(object_at_ref(pool(OBJECT_POOL), spi(0)));
            if uses_mission_cleanup(s) {
                mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), spi(0), 3);
            }
        }
        // 01C5 DONT_REMOVE_CHAR (0x47D7E1) / 01C7 DONT_REMOVE_OBJECT (0x47D813):
        // MissionCleanUp.RemoveEntityFromList(P0, 2 / 3) unconditionally.
        0x01C5 | 0x01C7 => {
            collect(s, 1);
            let ty = if command == 0x01C5 { 2 } else { 3 };
            mission_cleanup_remove(GPtr::from_addr(MISSION_CLEANUP), spi(0), ty);
        }
        0x01E7 => switch_roads(s, 0),
        0x01E8 => switch_roads(s, 1),
        // 01E9 GET_NUMBER_OF_PASSENGERS (0x47DD2F) / 01EA GET_MAXIMUM_NUMBER_OF_PASSENGERS (0x47DD75):
        // no null check; `movzx` of the byte at +0x484 / +0x488.
        0x01E9 | 0x01EA => {
            collect(s, 1);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let off = if command == 0x01E9 { 0x484 } else { 0x488 };
            set_sp(0, u32::from(veh.byte_add(off).read()));
            store(s, 1);
        }
        // 01EB SET_CAR_DENSITY_MULTIPLIER (0x47DDBB): raw dword copy.
        0x01EB => {
            collect(s, 1);
            GPtr::<u32>::from_addr(CAR_DENSITY_MULTIPLIER).write(sp(0));
        }
        // 01F0 SET_MAX_WANTED_LEVEL (0x47DE9A)
        0x01F0 => {
            collect(s, 1);
            set_maximum_wanted_level(spi(0));
        }
        // 01B2 GIVE_WEAPON_TO_CHAR (0x47D307)
        0x01B2 => give_weapon_to_char(s),
        // 01BC SET_OBJECT_COORDINATES (0x47D5C9)
        0x01BC => set_object_coordinates(s),
        // 01C1 IS_CAR_STOPPED (0x47D6CD): no null check; flag = IsVehicleStopped(VEH(P0)) != 0.
        0x01C1 => {
            collect(s, 1);
            let b = is_vehicle_stopped(vehicle_at_ref(pool(VEHICLE_POOL), spi(0))) != 0;
            set_flag(s, b);
        }
        // 01C8 CREATE_CHAR_AS_PASSENGER (0x47D845)
        0x01C8 => create_char_as_passenger(s),
        // 01E3 PRINT_WITH_NUMBER_BIG (0x47DA0B): text; `CollectParameters(3)` (number, time, style);
        // `AddBigMessageWithNumber(text, P1, P2 - 1, P0, -1 x5)`.
        0x01E3 => {
            let t = text_from_label(s);
            collect(s, 3);
            let m = u32::MAX;
            add_big_message_with_number(t, sp(1), sp(2).wrapping_sub(1), sp(0), m, m, m, m, m);
        }
        // 01E4 PRINT_WITH_NUMBER (0x47DA71)
        0x01E4 => print_with_number(s, false),
        // 01E5 PRINT_WITH_NUMBER_NOW (0x47DAEC)
        0x01E5 => print_with_number(s, true),
        // 01EC SET_CAR_HEAVY (0x47DDE6)
        0x01EC => set_car_heavy(s),
        // 01F3 IS_CAR_IN_AIR_PROPER (0x47DEC8)
        0x01F3 => is_car_in_air_proper(s),
        _ => return -1,
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_400_TO_499];
