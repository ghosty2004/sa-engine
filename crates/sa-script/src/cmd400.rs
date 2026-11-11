//! `CRunningScript::ProcessCommands400To499` (0x47D210): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands400To499.md`.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x0190, 0x0191, 0x01A1, 0x01A2, 0x01A3, 0x01A4, 0x01A5, 0x01A6, 0x01A7, 0x01A8, 0x01A9, 0x01AA, 0x01AB,
    0x01AC, 0x01AD, 0x01AE, 0x01AF, 0x01B0, 0x01B4, 0x01B5, 0x01B6, 0x01B7, 0x01B9, 0x01BB, 0x01BD, 0x01C0,
    0x01C2, 0x01C3, 0x01C4, 0x01C5, 0x01C7, 0x01E7, 0x01E8, 0x01E9, 0x01EA, 0x01EB, 0x01F0,
];

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
#[hook(addr = 0x47D210, conv = thiscall, name = "CRunningScript::ProcessCommands400To499", expect = "64 A1 00 00 00 00", this = "CRunningScript", fallback)]
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
        _ => {
            return fallback(
                process_commands_400_to_499_original,
                "CRunningScript::ProcessCommands400To499",
                s,
                command,
            );
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_400_TO_499];
