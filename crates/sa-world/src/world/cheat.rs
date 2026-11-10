//! `CCheat` (PORT-WORLD): cheat input and the simple cheats. Ported from the 1.0 US bytes (gta-reversed
//! `Cheat.cpp`, `docs/re/specs/WORLD/CCheat*.md` as reading aids).
//!
//! `m_CheatString` (0x969110, 30 bytes, newest key first), `m_aCheatsActive` (0x969130, 92 bytes),
//! `m_aCheatHashKeys` (0x8A5CC8, 92 dwords), `m_aCheatFunctions` (0x8A5B58, 92 function pointers, null for
//! toggle-only cheats), `m_bHasPlayerCheated` (0x96918C).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{find_player_ped, g, rd, x};

pub const NUM_CHEATS: usize = 92;

sa_core::globals! {
    list GLOBALS;
    /// `CCheat::m_CheatString`
    pub static CHEAT_STRING @ 0x969110: [u8; 30] = [0; 30];
    /// `CCheat::m_aCheatsActive`
    pub static CHEATS_ACTIVE @ 0x969130: [u8; NUM_CHEATS] = [0; NUM_CHEATS];
    /// `CCheat::m_bHasPlayerCheated`
    pub static HAS_PLAYER_CHEATED @ 0x96918C: u8 = 0;
}

/// `CCheat::m_aCheatFunctions` (.data, filled by the image: addresses of the cheat functions)
const CHEAT_FUNCTIONS: usize = 0x8A5B58;
/// `CCheat::m_aCheatHashKeys`
const CHEAT_HASH_KEYS: usize = 0x8A5CC8;
/// `CCutsceneMgr::ms_running`
const CUTSCENE_RUNNING: usize = 0xB5F851;
/// `CTimer::ms_fTimeScale`
const TIME_SCALE: usize = 0xB7CB64;
/// `TheText`
const THE_TEXT: usize = 0xC1B340;
/// `CPad::NewKeyState.standardKeys` / `OldKeyState.standardKeys` (int16[256])
const NEW_STANDARD_KEYS: usize = 0xB731A8;
const OLD_STANDARD_KEYS: usize = 0xB72F38;

orig! {
    /// a cheat function from `m_aCheatFunctions` (cdecl, no arguments)
    pub fn call_cheat_fn(f: u32) = cdecl dyn => unported;
    /// `CKeyGen::GetKey(const char*)` (0x53CF00), null-terminated form
    pub fn key_gen_get_key(s: GPtr<u8>) -> u32 = cdecl 0x53CF00 => unported;
    /// `CText::Get(const char*)` (0x6A0050)
    pub fn text_get(text: GPtr<u8>, key: GPtr<u8>) -> GPtr<u16> = thiscall 0x6A0050 => unported;
    /// `CHud::SetHelpMessage(const char*, bool quick, bool permanent, bool addToBrief)` (0x588BE0)
    pub fn hud_set_help_message(t: GPtr<u16>, quick: i32, permanent: i32, brief: i32) = cdecl 0x588BE0 => unported;
    /// `CStats::IncrementStat(uint16, float)` (0x55C180)
    pub fn stats_increment_stat(stat: u32, v: u32) = cdecl 0x55C180 => unported;
    /// `CStats::SetStatValue(uint16, float)` (0x55A070)
    pub fn stats_set_stat_value(stat: u32, v: u32) = cdecl 0x55A070 => unported;
    /// `CCheat::AddToCheatString(char)` (0x438480)
    pub fn add_to_cheat_string_call(c: u32) = cdecl 0x438480 => add_to_cheat_string_port;
    /// `CWeather::ReleaseWeather` (0x72A510)
    pub fn weather_release_weather() = cdecl 0x72A510 => unported;
    /// `CWeather::ForceWeatherNow(int16)` (0x72A4F0)
    pub fn weather_force_weather_now(w: u32) = cdecl 0x72A4F0 => unported;
    /// `CPlayerPed::CheatWantedLevel(int32)` (0x609F50)
    pub fn player_cheat_wanted_level(ped: GPtr<u8>, level: i32) = thiscall 0x609F50 => unported;
    /// `CClothes::RebuildPlayer(CPlayerPed*, bool)` (0x5A82C0)
    pub fn clothes_rebuild_player(ped: GPtr<u8>, b: i32) = cdecl 0x5A82C0 => unported;
    /// `CCheat::HandleSpecialCheats(eCheats)` (0x439A10)
    pub fn handle_special_cheats(c: i32) = cdecl 0x439A10 => unported;
    /// `CStreaming::ReclassifyLoadedCars` (0x40AFA0)
    pub fn streaming_reclassify_loaded_cars() = cdecl 0x40AFA0 => unported;
    /// `CStreaming::StreamPedsIntoRandomSlots(const int32* models)` (0x40BDA0)
    pub fn streaming_stream_peds_into_random_slots(m: GPtr<i32>) = cdecl 0x40BDA0 => unported;
    /// `CCheat::VehicleCheat(eModelID)` (0x43A0B0)
    pub fn vehicle_cheat(model: i32) -> GPtr<u8> = cdecl 0x43A0B0 => unported;
}

pub fn add_to_cheat_string_port(c: u32) {
    add_to_cheat_string(c as u8)
}

fn toggle(addr: usize) -> u8 {
    let p = g::<u8>(addr);
    let v = (p.read() == 0) as u8;
    p.write(v);
    v
}

/// `CCheat::IsZoneStreamingAllowed` (stub 0x407410 → 0x1563A60): false while any of the cheats
/// 0x27..0x2A, 0x2C, 0x2D, 0x46, 0x4B is active.
#[hook(addr = 0x407410, conv = cdecl, name = "CCheat::IsZoneStreamingAllowed", expect = "E9 4B C6 15 01")]
pub fn is_zone_streaming_allowed() -> u8 {
    for a in [0x969157, 0x969158, 0x969159, 0x96915A, 0x96915C, 0x96915D, 0x969176, 0x96917B] {
        if g::<u8>(a).read() != 0 {
            return 1;
        }
    }
    0
}

/// `CCheat::ApplyCheat(eCheats)` (0x438370): the cheat's function (tail jump) or a toggle.
#[hook(addr = 0x438370, conv = cdecl, name = "CCheat::ApplyCheat", expect = "8B 44 24 04 8B 0C 85 58 5B 8A 00")]
pub fn apply_cheat(i: i32) {
    let f = g::<u32>(CHEAT_FUNCTIONS).wrapping_index(i).read();
    if f != 0 {
        call_cheat_fn(f);
        return;
    }
    let p = CHEATS_ACTIVE.ptr().cast::<u8>().byte_add(i as isize);
    p.write((p.read() == 0) as u8);
}

/// `CCheat::ResetCheats` (stub 0x438450 → 0x1560F40): every cheat inactive, `ReleaseWeather`, time
/// scale 1.0, empty cheat string, `m_bHasPlayerCheated` = 0.
#[hook(addr = 0x438450, conv = cdecl, name = "CCheat::ResetCheats", expect = "E9 EB 8A 12 01")]
pub fn reset_cheats() {
    CHEATS_ACTIVE.set([0; NUM_CHEATS]);
    weather_release_weather();
    g::<u32>(TIME_SCALE).write(0x3F80_0000);
    CHEAT_STRING.ptr().cast::<u8>().write(0);
    HAS_PLAYER_CHEATED.set(0);
}

/// `CCheat::AddToCheatString(char)` (0x438480): not during a cutscene; shifts the string, puts the key
/// first, then tries the hashes (`CKeyGen::GetKey`) of the longest prefix down to 6 characters against
/// the 92 cheat keys. On a match: help text CHEAT1 (was off) / CHEAT8 (was on), stat 0x89 += 1,
/// `m_bHasPlayerCheated` = 1, the function or the toggle, and the string is cleared.
#[hook(addr = 0x438480, conv = cdecl, name = "CCheat::AddToCheatString", expect = "A0 51 F8 B5 00")]
pub fn add_to_cheat_string(c: u8) {
    if g::<u8>(CUTSCENE_RUNNING).read() != 0 {
        return;
    }
    let s = CHEAT_STRING.ptr().cast::<u8>();
    let mut k: isize = 0x1C;
    while k >= 0 {
        s.byte_add(k + 1).write(s.byte_add(k).read());
        k -= 1;
    }
    s.write(c);
    s.byte_add(0x1D).write(0);
    let mut len = 0usize;
    while s.byte_add(len as isize).read() != 0 {
        len += 1;
    }
    if len < 6 {
        return;
    }
    let mut tmp = [0u8; 32];
    let mut n = 0usize;
    loop {
        let b = s.byte_add(n as isize).read();
        tmp[n] = b;
        n += 1;
        if b == 0 {
            break;
        }
    }
    let tp = GPtr::from_raw(tmp.as_mut_ptr());
    let mut e = len;
    while e >= 6 {
        let key = key_gen_get_key(tp);
        if let Some(i) = (0..NUM_CHEATS as i32).find(|&i| g::<u32>(CHEAT_HASH_KEYS).wrapping_index(i).read() == key) {
            let active = CHEATS_ACTIVE.ptr().cast::<u8>().byte_add(i as isize);
            let label = if active.read() == 0 { 0x859080 } else { 0x859088 };
            let t = text_get(g(THE_TEXT), g(label));
            hud_set_help_message(t, 1, 0, 0);
            stats_increment_stat(0x89, 0x3F80_0000);
            let f = g::<u32>(CHEAT_FUNCTIONS).wrapping_index(i).read();
            HAS_PLAYER_CHEATED.set(1);
            if f != 0 {
                call_cheat_fn(f);
            } else {
                active.write((active.read() == 0) as u8);
            }
            s.write(0);
            return;
        }
        tmp[e - 1] = 0;
        e -= 1;
    }
}

/// `CCheat::DoCheats` (stub 0x439AF0 → 0x1566D70): `AddToCheatString(k)` for every standard key just
/// pressed (new state != 0, old state == 0), k = 0..255.
#[hook(addr = 0x439AF0, conv = cdecl, name = "CCheat::DoCheats", expect = "E9 7B D2 12 01")]
pub fn do_cheats() {
    for k in 0..0x100 {
        if g::<i16>(NEW_STANDARD_KEYS).wrapping_index(k).read() != 0
            && g::<i16>(OLD_STANDARD_KEYS).wrapping_index(k).read() == 0
        {
            add_to_cheat_string_call(k as u32);
        }
    }
}

/// `CCheat::SunnyWeatherCheat` (0x438F40): `ForceWeatherNow(1)`.
#[hook(addr = 0x438F40, conv = cdecl, name = "CCheat::SunnyWeatherCheat", expect = "6A 01 E8 A9 15 2F 00")]
pub fn sunny_weather_cheat() {
    weather_force_weather_now(1);
}

/// `CCheat::ExtraSunnyWeatherCheat` (0x438F50): `ForceWeatherNow(0)`.
#[hook(addr = 0x438F50, conv = cdecl, name = "CCheat::ExtraSunnyWeatherCheat", expect = "6A 00 E8 99 15 2F 00")]
pub fn extra_sunny_weather_cheat() {
    weather_force_weather_now(0);
}

/// `CCheat::CloudyWeatherCheat` (0x438F60): `ForceWeatherNow(4)`.
#[hook(addr = 0x438F60, conv = cdecl, name = "CCheat::CloudyWeatherCheat", expect = "6A 04 E8 89 15 2F 00")]
pub fn cloudy_weather_cheat() {
    weather_force_weather_now(4);
}

/// `CCheat::RainyWeatherCheat` (0x438F70): `ForceWeatherNow(16)`.
#[hook(addr = 0x438F70, conv = cdecl, name = "CCheat::RainyWeatherCheat", expect = "6A 10 E8 79 15 2F 00")]
pub fn rainy_weather_cheat() {
    weather_force_weather_now(0x10);
}

/// `CCheat::FoggyWeatherCheat` (0x438F80): `ForceWeatherNow(9)`.
#[hook(addr = 0x438F80, conv = cdecl, name = "CCheat::FoggyWeatherCheat", expect = "6A 09 E8 69 15 2F 00")]
pub fn foggy_weather_cheat() {
    weather_force_weather_now(9);
}

/// `CCheat::FastTimeCheat` (0x438F90): time scale doubled while below 4.0.
#[hook(addr = 0x438F90, conv = cdecl, name = "CCheat::FastTimeCheat", expect = "D9 05 64 CB B7 00")]
pub fn fast_time_cheat() {
    let p = g::<f32>(TIME_SCALE);
    // test ah,5; jp: only Less
    if !x(p.read()).fcom(x(4.0)).test_parity(0x05) {
        let v = x(p.read());
        p.write(v.fadd(v).to_f32());
    }
}

/// `CCheat::SlowTimeCheat` (0x438FC0): time scale halved while above 0.25.
#[hook(addr = 0x438FC0, conv = cdecl, name = "CCheat::SlowTimeCheat", expect = "D9 05 64 CB B7 00")]
pub fn slow_time_cheat() {
    let p = g::<f32>(TIME_SCALE);
    // test ah,0x41; jne: only Greater
    if !x(p.read()).fcom(x(0.25)).test_any(0x41) {
        p.write(x(p.read()).fmul_m(0.5).to_f32());
    }
}

/// `CCheat::PinkCarsCheat` (0x4390D0): toggle; when on, black cars off.
#[hook(addr = 0x4390D0, conv = cdecl, name = "CCheat::PinkCarsCheat", expect = "A0 50 91 96 00")]
pub fn pink_cars_cheat() {
    if toggle(0x969150) != 0 {
        g::<u8>(0x969151).write(0);
    }
}

/// `CCheat::BlackCarsCheat` (0x4390F0): toggle; when on, pink cars off.
#[hook(addr = 0x4390F0, conv = cdecl, name = "CCheat::BlackCarsCheat", expect = "A0 51 91 96 00")]
pub fn black_cars_cheat() {
    if toggle(0x969151) != 0 {
        g::<u8>(0x969150).write(0);
    }
}

/// `CCheat::GangLandCheat` (0x4393D0): toggle, copied to `CGangWars::bGangWarsActive`... (0xC0FCB3).
#[hook(addr = 0x4393D0, conv = cdecl, name = "CCheat::GangLandCheat", expect = "A0 5B 91 96 00")]
pub fn gang_land_cheat() {
    let v = toggle(0x96915B);
    g::<u8>(0xC0FCB3).write((v != 0) as u8);
}

/// `CCheat::MidnightCheat` (0x439510): toggle; when on, the clock (0xB70150..0xB70153) = 00:00:00.
#[hook(addr = 0x439510, conv = cdecl, name = "CCheat::MidnightCheat", expect = "A0 67 91 96 00")]
pub fn midnight_cheat() {
    if toggle(0x969167) != 0 {
        g::<u8>(0xB70153).write(0);
        g::<u8>(0xB70152).write(0);
        g::<u16>(0xB70150).write(0);
    }
}

/// `CCheat::DuskCheat` (0x439540): toggle; when on, the clock = 21:00:00.
#[hook(addr = 0x439540, conv = cdecl, name = "CCheat::DuskCheat", expect = "A0 68 91 96 00")]
pub fn dusk_cheat() {
    if toggle(0x969168) != 0 {
        g::<u8>(0xB70153).write(0x15);
        g::<u8>(0xB70152).write(0);
        g::<u16>(0xB70150).write(0);
    }
}

/// `CCheat::StormCheat` (0x439570): `ForceWeatherNow(16)`, rain 1.0 (0xC81324), wet 1.0 (0xC812F0).
#[hook(addr = 0x439570, conv = cdecl, name = "CCheat::StormCheat", expect = "6A 10 E8 79 0F 2F 00")]
pub fn storm_cheat() {
    weather_force_weather_now(0x10);
    g::<u32>(0xC81324).write(0x3F80_0000);
    g::<u32>(0xC812F0).write(0x3F80_0000);
}

/// `CCheat::SandstormCheat` (0x439590): `ForceWeatherNow(19)`.
#[hook(addr = 0x439590, conv = cdecl, name = "CCheat::SandstormCheat", expect = "6A 13 E8 59 0F 2F 00")]
pub fn sandstorm_cheat() {
    weather_force_weather_now(0x13);
}

/// `CCheat::PredatorCheat` (0x4395A0): a lone `ret` (not hookable).
pub fn predator_cheat() {}

/// `CCheat::NotWantedCheat` (0x4396C0): wanted level 0, toggle 0x969171.
#[hook(addr = 0x4396C0, conv = cdecl, name = "CCheat::NotWantedCheat", expect = "6A 00 6A FF E8 47 4B 13 00")]
pub fn not_wanted_cheat() {
    player_cheat_wanted_level(find_player_ped(-1), 0);
    toggle(0x969171);
}

/// `CCheat::WantedCheat` (0x4396F0): wanted level 6.
#[hook(addr = 0x4396F0, conv = cdecl, name = "CCheat::WantedCheat", expect = "6A 06 6A FF E8 17 4B 13 00")]
pub fn wanted_cheat() {
    player_cheat_wanted_level(find_player_ped(-1), 6);
}

/// The player's wanted level (`ped->m_pPlayerData->m_pWanted->m_nWantedLevel`, null data → reads at 0).
fn wanted_level_plus_2() -> i32 {
    let data = rd::<GPtr<u8>>(find_player_ped(-1), 0x480);
    let wanted = if data.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(data, 0) };
    rd::<i32>(wanted, 0x2C).wrapping_add(2)
}

/// `CCheat::WantedLevelUpCheat` (0x438E90): `min(level + 2, 6)` (level re-read for the call).
#[hook(addr = 0x438E90, conv = cdecl, name = "CCheat::WantedLevelUpCheat", expect = "6A FF E8 79 53 13 00")]
pub fn wanted_level_up_cheat() {
    let v = if wanted_level_plus_2() < 6 { wanted_level_plus_2() } else { 6 };
    player_cheat_wanted_level(find_player_ped(-1), v);
}

/// `CCheat::WantedLevelDownCheat` (0x438F20): wanted level 0.
#[hook(addr = 0x438F20, conv = cdecl, name = "CCheat::WantedLevelDownCheat", expect = "6A 00 6A FF E8 E7 52 13 00")]
pub fn wanted_level_down_cheat() {
    player_cheat_wanted_level(find_player_ped(-1), 0);
}

/// `CCheat::RiotCheat` (0x439710): toggle 0x969175.
#[hook(addr = 0x439710, conv = cdecl, name = "CCheat::RiotCheat", expect = "A0 75 91 96 00")]
pub fn riot_cheat() {
    toggle(0x969175);
}

/// `CCheat::StaminaCheat` (0x439930): stat 0x16 = 1000.
#[hook(addr = 0x439930, conv = cdecl, name = "CCheat::StaminaCheat", expect = "68 00 00 7A 44")]
pub fn stamina_cheat() {
    stats_set_stat_value(0x16, 0x447A_0000);
}

/// `CCheat::WeaponSkillsCheat` (0x439940): stats 0x45..0x4F = 1000.
#[hook(addr = 0x439940, conv = cdecl, name = "CCheat::WeaponSkillsCheat", expect = "68 00 00 7A 44")]
pub fn weapon_skills_cheat() {
    for s in 0x45..=0x4F {
        stats_set_stat_value(s, 0x447A_0000);
    }
}

/// `CCheat::VehicleSkillsCheat` (0x4399D0): stats 0xA0, 0xDF, 0xE5, 0xE6 = 1000.
#[hook(addr = 0x4399D0, conv = cdecl, name = "CCheat::VehicleSkillsCheat", expect = "68 00 00 7A 44")]
pub fn vehicle_skills_cheat() {
    for s in [0xA0, 0xDF, 0xE5, 0xE6] {
        stats_set_stat_value(s, 0x447A_0000);
    }
}

/// `CCheat::TankCheat` (0x43A4A0): `VehicleCheat(0x1b0)`.
#[hook(addr = 0x43A4A0, conv = cdecl, name = "CCheat::TankCheat", expect = "68 B0 01 00 00")]
pub fn tank_cheat() {
    vehicle_cheat(0x1b0);
}

/// `CCheat::StockCarCheat` (0x43A4B0): `VehicleCheat(0x1f8)`.
#[hook(addr = 0x43A4B0, conv = cdecl, name = "CCheat::StockCarCheat", expect = "68 F8 01 00 00")]
pub fn stock_car_cheat() {
    vehicle_cheat(0x1f8);
}

/// `CCheat::StockCar2Cheat` (0x43A4C0): `VehicleCheat(0x1f9)`.
#[hook(addr = 0x43A4C0, conv = cdecl, name = "CCheat::StockCar2Cheat", expect = "68 F9 01 00 00")]
pub fn stock_car2_cheat() {
    vehicle_cheat(0x1f9);
}

/// `CCheat::StockCar3Cheat` (0x43A4D0): `VehicleCheat(0x1f6)`.
#[hook(addr = 0x43A4D0, conv = cdecl, name = "CCheat::StockCar3Cheat", expect = "68 F6 01 00 00")]
pub fn stock_car3_cheat() {
    vehicle_cheat(0x1f6);
}

/// `CCheat::StockCar4Cheat` (0x43A4E0): `VehicleCheat(0x1f7)`.
#[hook(addr = 0x43A4E0, conv = cdecl, name = "CCheat::StockCar4Cheat", expect = "68 F7 01 00 00")]
pub fn stock_car4_cheat() {
    vehicle_cheat(0x1f7);
}

/// `CCheat::HearseCheat` (0x43A4F0): `VehicleCheat(0x1ba)`.
#[hook(addr = 0x43A4F0, conv = cdecl, name = "CCheat::HearseCheat", expect = "68 BA 01 00 00")]
pub fn hearse_cheat() {
    vehicle_cheat(0x1ba);
}

/// `CCheat::LovefistCheat` (0x43A500): `VehicleCheat(0x199)`.
#[hook(addr = 0x43A500, conv = cdecl, name = "CCheat::LovefistCheat", expect = "68 99 01 00 00")]
pub fn lovefist_cheat() {
    vehicle_cheat(0x199);
}

/// `CCheat::TrashmasterCheat` (0x43A510): `VehicleCheat(0x198)`.
#[hook(addr = 0x43A510, conv = cdecl, name = "CCheat::TrashmasterCheat", expect = "68 98 01 00 00")]
pub fn trashmaster_cheat() {
    vehicle_cheat(0x198);
}

/// `CCheat::GolfcartCheat` (0x43A520): `VehicleCheat(0x1c9)`.
#[hook(addr = 0x43A520, conv = cdecl, name = "CCheat::GolfcartCheat", expect = "68 C9 01 00 00")]
pub fn golfcart_cheat() {
    vehicle_cheat(0x1c9);
}

/// `CCheat::FlyboyCheat` (0x43A530): `VehicleCheat(0x208)`.
#[hook(addr = 0x43A530, conv = cdecl, name = "CCheat::FlyboyCheat", expect = "68 08 02 00 00")]
pub fn flyboy_cheat() {
    vehicle_cheat(0x208);
}

/// `CCheat::VortexCheat` (0x43A540): `VehicleCheat(0x21b)`.
#[hook(addr = 0x43A540, conv = cdecl, name = "CCheat::VortexCheat", expect = "68 1B 02 00 00")]
pub fn vortex_cheat() {
    vehicle_cheat(0x21b);
}

/// `CCheat::ApacheCheat` (0x43A550): `VehicleCheat(0x1a9)`.
#[hook(addr = 0x43A550, conv = cdecl, name = "CCheat::ApacheCheat", expect = "68 A9 01 00 00")]
pub fn apache_cheat() {
    vehicle_cheat(0x1a9);
}

/// `CCheat::QuadCheat` (0x43A560): `VehicleCheat(0x1d7)`.
#[hook(addr = 0x43A560, conv = cdecl, name = "CCheat::QuadCheat", expect = "68 D7 01 00 00")]
pub fn quad_cheat() {
    vehicle_cheat(0x1d7);
}

/// `CCheat::DozerCheat` (0x43A660): `VehicleCheat(0x1e6)`.
#[hook(addr = 0x43A660, conv = cdecl, name = "CCheat::DozerCheat", expect = "68 E6 01 00 00")]
pub fn dozer_cheat() {
    vehicle_cheat(0x1e6);
}

/// `CCheat::StuntPlaneCheat` (0x43A670): `VehicleCheat(0x201)`.
#[hook(addr = 0x43A670, conv = cdecl, name = "CCheat::StuntPlaneCheat", expect = "68 01 02 00 00")]
pub fn stunt_plane_cheat() {
    vehicle_cheat(0x201);
}

/// `CCheat::MonsterTruckCheat` (0x43A680): `VehicleCheat(0x22c)`.
#[hook(addr = 0x43A680, conv = cdecl, name = "CCheat::MonsterTruckCheat", expect = "68 2C 02 00 00")]
pub fn monster_truck_cheat() {
    vehicle_cheat(0x22c);
}


/// Fat/muscle/skinny tail: `CClothes::RebuildPlayer(ped, false)` unless the ped state (+0x530) is 0x32.
fn rebuild_player_unless_dead() {
    if rd::<i32>(find_player_ped(-1), 0x530) != 0x32 {
        clothes_rebuild_player(find_player_ped(-1), 0);
    }
}

/// `CCheat::FatCheat` (0x439110): stat 0x15 = 1000, rebuild the player.
#[hook(addr = 0x439110, conv = cdecl, name = "CCheat::FatCheat", expect = "68 00 00 7A 44")]
pub fn fat_cheat() {
    stats_set_stat_value(0x15, 0x447A_0000);
    rebuild_player_unless_dead();
}

/// `CCheat::MuscleCheat` (0x439150): stat 0x17 = 1000, rebuild the player.
#[hook(addr = 0x439150, conv = cdecl, name = "CCheat::MuscleCheat", expect = "68 00 00 7A 44")]
pub fn muscle_cheat() {
    stats_set_stat_value(0x17, 0x447A_0000);
    rebuild_player_unless_dead();
}

/// `CCheat::SkinnyCheat` (0x439190): stats 0x15, 0x17 = 0, rebuild the player.
#[hook(addr = 0x439190, conv = cdecl, name = "CCheat::SkinnyCheat", expect = "6A 00 6A 15 E8 D7 0E 12 00")]
pub fn skinny_cheat() {
    stats_set_stat_value(0x15, 0);
    stats_set_stat_value(0x17, 0);
    rebuild_player_unless_dead();
}

/// `CCheat::AllCarsAreShitCheat` (0x4394B0): toggle 0x96915E; when on, `HandleSpecialCheats(0x2E)` and
/// `CStreaming::ReclassifyLoadedCars` (tail jump).
#[hook(addr = 0x4394B0, conv = cdecl, name = "CCheat::AllCarsAreShitCheat", expect = "8A 15 5E 91 96 00")]
pub fn all_cars_are_shit_cheat() {
    if toggle(0x96915E) != 0 {
        handle_special_cheats(0x2E);
        streaming_reclassify_loaded_cars();
    }
}

/// `CCheat::AllCarsAreGreatCheat` (0x4394E0): toggle 0x96915F; when on, `HandleSpecialCheats(0x2F)` and
/// `CStreaming::ReclassifyLoadedCars` (tail jump).
#[hook(addr = 0x4394E0, conv = cdecl, name = "CCheat::AllCarsAreGreatCheat", expect = "8A 15 5F 91 96 00")]
pub fn all_cars_are_great_cheat() {
    if toggle(0x96915F) != 0 {
        handle_special_cheats(0x2F);
        streaming_reclassify_loaded_cars();
    }
}

/// `CCheat::GangsCheat` (0x439360): toggle 0x96915A; when on, `StreamPedsIntoRandomSlots` with the gang
/// models 0x66..0x6A, 0x6C..0x6E.
#[hook(addr = 0x439360, conv = cdecl, name = "CCheat::GangsCheat", expect = "8A 15 5A 91 96 00")]
pub fn gangs_cheat() {
    if toggle(0x96915A) != 0 {
        let mut m: [i32; 8] = [0x66, 0x67, 0x68, 0x69, 0x6A, 0x6C, 0x6D, 0x6E];
        streaming_stream_peds_into_random_slots(GPtr::from_raw(m.as_mut_ptr()));
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_FAT_CHEAT,
    HOOK_MUSCLE_CHEAT,
    HOOK_SKINNY_CHEAT,
    HOOK_ALL_CARS_ARE_SHIT_CHEAT,
    HOOK_ALL_CARS_ARE_GREAT_CHEAT,
    HOOK_GANGS_CHEAT,
    HOOK_IS_ZONE_STREAMING_ALLOWED,
    HOOK_APPLY_CHEAT,
    HOOK_RESET_CHEATS,
    HOOK_ADD_TO_CHEAT_STRING,
    HOOK_DO_CHEATS,
    HOOK_SUNNY_WEATHER_CHEAT,
    HOOK_EXTRA_SUNNY_WEATHER_CHEAT,
    HOOK_CLOUDY_WEATHER_CHEAT,
    HOOK_RAINY_WEATHER_CHEAT,
    HOOK_FOGGY_WEATHER_CHEAT,
    HOOK_FAST_TIME_CHEAT,
    HOOK_SLOW_TIME_CHEAT,
    HOOK_PINK_CARS_CHEAT,
    HOOK_BLACK_CARS_CHEAT,
    HOOK_GANG_LAND_CHEAT,
    HOOK_MIDNIGHT_CHEAT,
    HOOK_DUSK_CHEAT,
    HOOK_STORM_CHEAT,
    HOOK_SANDSTORM_CHEAT,
    HOOK_NOT_WANTED_CHEAT,
    HOOK_WANTED_CHEAT,
    HOOK_WANTED_LEVEL_UP_CHEAT,
    HOOK_WANTED_LEVEL_DOWN_CHEAT,
    HOOK_RIOT_CHEAT,
    HOOK_STAMINA_CHEAT,
    HOOK_WEAPON_SKILLS_CHEAT,
    HOOK_VEHICLE_SKILLS_CHEAT,
    HOOK_TANK_CHEAT,
    HOOK_STOCK_CAR_CHEAT,
    HOOK_STOCK_CAR2_CHEAT,
    HOOK_STOCK_CAR3_CHEAT,
    HOOK_STOCK_CAR4_CHEAT,
    HOOK_HEARSE_CHEAT,
    HOOK_LOVEFIST_CHEAT,
    HOOK_TRASHMASTER_CHEAT,
    HOOK_GOLFCART_CHEAT,
    HOOK_FLYBOY_CHEAT,
    HOOK_VORTEX_CHEAT,
    HOOK_APACHE_CHEAT,
    HOOK_QUAD_CHEAT,
    HOOK_DOZER_CHEAT,
    HOOK_STUNT_PLANE_CHEAT,
    HOOK_MONSTER_TRUCK_CHEAT,
];
