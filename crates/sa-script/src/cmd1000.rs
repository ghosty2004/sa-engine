//! `CRunningScript::ProcessCommands1000To1099` (0x489500): partial port (PORT-SCRIPT-2, ADR-002). Opcodes in [`PORTED`] run in Rust;
//! every other opcode (and the default, -1) runs the original body through the fallback.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands1000To1099.md`.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x03EB, 0x03ED, 0x03EE, 0x03EF, 0x03F0, 0x03F3, 0x03F5, 0x03FD, 0x03FE, 0x040C, 0x040D, 0x0414, 0x0418,
    0x041D, 0x041E, 0x0423, 0x0424, 0x0425, 0x0428, 0x042C, 0x042D, 0x042E, 0x0430, 0x0431, 0x0432, 0x0434,
    0x0435, 0x0436, 0x043C, 0x0441, 0x0445, 0x0446,
];

/// `AudioEngine` (`CAudioEngine`) 0xB6BC90.
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `CCutsceneMgr::ms_cutsceneProcessing` (byte) 0xB5F852.
const CUTSCENE_PROCESSING: usize = 0xB5F852;
/// `CTheScripts::UseTextCommands` (byte) 0xA44B67.
const USE_TEXT_COMMANDS: usize = 0xA44B67;
/// `CCredits::bCreditsGoing` (byte) 0xC6E97C.
const CREDITS_GOING: usize = 0xC6E97C;
/// Byte 0xB6F04D (`TheCamera` + 0x25; SET_MUSIC_DOES_FADE's flag).
const MUSIC_DOES_FADE_OFF: usize = 0xB6F04D;
/// The three car-cheat bytes ARE_ANY_CAR_CHEATS_ACTIVATED tests (0x96914C, 0x969152, 0x969153).
const CAR_CHEATS: [usize; 3] = [0x96914C, 0x969152, 0x969153];
/// `.rdata` 0x859F60: 3.3333333f (metres to feet)
const K_M2FT: u32 = 0x4055_5555;
/// The immediate 03EF pushes as the radius: 10000.0f (`push 0x461C4000`).
const K_10000: u32 = 0x461C_4000;

orig! {
    /// `CMessages::ClearSmallMessagesOnly()` (0x69DD30)
    fn clear_small_messages_only() = cdecl 0x69DD30 => unported;
    /// `CPlayerPed::CanPlayerStartMission()` (0x609590), AL
    fn can_player_start_mission(p: GPtr<u8>) -> u8 = thiscall 0x609590 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPlayerInfo::MakePlayerSafe(bool, float radius)` (0x56E870)
    fn make_player_safe(info: GPtr<u8>, safe: u32, radius: f32) = thiscall 0x56E870 => unported;
    /// `CVehicle::ExtinguishCarFire()` (0x6D2460)
    fn extinguish_car_fire(v: GPtr<u8>) = thiscall 0x6D2460 => unported;
    /// `CPad::SetDrunkInputDelay(int32)` (0x53F910)
    fn set_drunk_input_delay(pad: GPtr<u8>, d: i32) = thiscall 0x53F910 => unported;
    /// `CLocalisation::GermanGame()` (0x56D200), AL
    fn german_game() -> u8 = cdecl 0x56D200 => unported;
    /// `CLocalisation::Metric()` (0x56D220), AL
    fn metric() -> u8 = cdecl 0x56D220 => unported;
    /// `CAudioEngine::ClearMissionAudio(uint8)` (0x5072F0)
    fn clear_mission_audio(a: GPtr<u8>, slot: u32) = thiscall 0x5072F0 => unported;
    /// `CCamera::SetNearClipScript(float)` (0x50BF90); the float as its raw dword
    fn set_near_clip_script(cam: GPtr<u8>, v: u32) = thiscall 0x50BF90 => unported;
    /// `CAudioEngine::RetuneRadio(int8)` (0x507E10)
    fn retune_radio(a: GPtr<u8>, ch: u32) = thiscall 0x507E10 => unported;
    /// `CStats::SetStatValue(uint16 stat, float value)` (0x55A070); the float as its raw dword
    fn set_stat_value(stat: u32, value: u32) = cdecl 0x55A070 => unported;
    /// `CStats::RegisterFastestTime(int32 stat, int32 time)` (0x55A0B0)
    fn register_fastest_time(stat: i32, time: i32) = cdecl 0x55A0B0 => unported;
    /// `CCredits::Start()` (0x7170E0)
    fn credits_start() = cdecl 0x7170E0 => unported;
    /// `CCredits::Stop()` (0x717100)
    fn credits_stop() = cdecl 0x717100 => unported;
    /// `CCarEnterExit::ComputeTargetDoorToEnterAsPassenger(const CVehicle*, int32 seat)` (0x64F190)
    fn compute_target_door_to_enter_as_passenger(veh: GPtr<u8>, seat: i32) -> i32 = cdecl 0x64F190 => unported;
    /// `CPedIntelligence::FlushImmediately(bool)` (0x601640)
    fn flush_immediately(i: GPtr<u8>, set_primary_default: u32) = thiscall 0x601640 => unported;
    /// `CTaskSimpleCarSetPedInAsPassenger::CTaskSimpleCarSetPedInAsPassenger(CVehicle*, int32 door,
    /// CTaskUtilityLineUpPedWithCar*)` (0x646FE0)
    fn set_ped_in_as_passenger_ctor(t: GPtr<u8>, veh: GPtr<u8>, door: i32, line_up: GPtr<u8>) -> GPtr<u8> = thiscall 0x646FE0 => unported;
    /// `CTaskSimpleCarSetPedInAsPassenger::ProcessPed(CPed*)` (0x64B5D0)
    fn set_ped_in_as_passenger_process_ped(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x64B5D0 => unported;
    /// `CTaskSimpleCarSetPedInAsPassenger::~CTaskSimpleCarSetPedInAsPassenger()` (0x647080)
    fn set_ped_in_as_passenger_dtor(t: GPtr<u8>) = thiscall 0x647080 => unported;
}

/// `CRunningScript::ProcessCommands1000To1099(int32 command)` (0x489500, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x489500, conv = thiscall, name = "CRunningScript::ProcessCommands1000To1099", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript", fallback)]
pub fn process_commands_1000_to_1099(s: Scr, command: u32) -> i8 {
    match command {
        // 03EB CLEAR_SMALL_PRINTS (0x48953E)
        0x03EB => clear_small_messages_only(),
        // 03ED SET_UPSIDEDOWN_CAR_NOT_DAMAGED (0x48954A): no null check; byte +0x868 bit 3 (0x8) = P1 != 0.
        0x03ED => {
            collect(s, 2);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let p = veh.byte_add(0x868);
            let b = p.read();
            p.write(if sp(1) != 0 { b | 0x8 } else { b & 0xF7 });
        }
        // 03EE CAN_PLAYER_START_MISSION (0x489595): flag = Players[P0].m_pPed->CanPlayerStartMission() != 0.
        0x03EE => {
            collect(s, 1);
            let ped = player_info(sp(0)).cast::<GPtr<u8>>().read();
            let b = can_player_start_mission(ped) != 0;
            set_flag(s, b);
        }
        // 03EF MAKE_PLAYER_SAFE_FOR_CUTSCENE (0x4895D1): CPad::GetPad(P0) byte +0x10E |= 0x80;
        // Players[P0].MakePlayerSafe(true, 10000.0f); CCutsceneMgr::ms_cutsceneProcessing = 1.
        0x03EF => {
            collect(s, 1);
            let info = player_info(sp(0));
            let pad = get_pad(spi(0));
            let p = pad.byte_add(0x10E);
            p.write(p.read() | 0x80);
            make_player_safe(info, 1, f(K_10000));
            GPtr::<u8>::from_addr(CUTSCENE_PROCESSING).write(1);
        }
        // 03F0 USE_TEXT_COMMANDS (0x48961B): CTheScripts::UseTextCommands = (P0 != 0) + 1.
        0x03F0 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(USE_TEXT_COMMANDS).write(u8::from(sp(0) != 0) + 1);
        }
        // 03F3 GET_CAR_COLOURS (0x48963E): no null check; P0, P1 = colour bytes +0x434, +0x435 (`movzx`).
        0x03F3 => {
            collect(s, 1);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            set_sp(0, u32::from(veh.byte_add(0x434).read()));
            set_sp(1, u32::from(veh.byte_add(0x435).read()));
            store(s, 2);
        }
        // 03F5 SET_CAR_CAN_BE_DAMAGED (0x4896E8): no null check; P1 != 0: byte +0x42A |= 0x20; otherwise
        // ExtinguishCarFire() then byte +0x42A &= 0xDF.
        0x03F5 => {
            collect(s, 2);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let p = veh.byte_add(0x42A);
            if sp(1) != 0 {
                p.write(p.read() | 0x20);
            } else {
                extinguish_car_fire(veh);
                p.write(p.read() & 0xDF);
            }
        }
        // 03FD SET_DRUNK_INPUT_DELAY (0x489731): CPad::GetPad(P0)->SetDrunkInputDelay(P1).
        0x03FD => {
            collect(s, 2);
            let pad = get_pad(spi(0));
            set_drunk_input_delay(pad, spi(1));
        }
        // 03FE SET_CHAR_MONEY (0x48975D): no null check. Reads `m_nPedFlags` dword +0x470, writes
        // `m_nMoneyCount` (+0x756) = (u16)P1, then writes the flags back with 0x20000 (bMoneyHasBeenGivenByScript) set.
        0x03FE => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let flags = ped.byte_add(0x470).cast::<u32>().read();
            ped.byte_add(0x756).cast::<u16>().write(sp(1) as u16);
            ped.byte_add(0x470).cast::<u32>().write(flags | 0x20000);
        }
        // 040C IS_GERMAN_GAME (0x4898B4): flag = CLocalisation::GermanGame() != 0.
        0x040C => {
            let b = german_game() != 0;
            set_flag(s, b);
        }
        // 040D CLEAR_MISSION_AUDIO (0x4898CB): AudioEngine.ClearMissionAudio((u8)P0 - 1).
        0x040D => {
            collect(s, 1);
            clear_mission_audio(
                GPtr::from_addr(AUDIO_ENGINE),
                u32::from((sp(0) as u8).wrapping_sub(1)),
            );
        }
        // 0414 SET_FREE_HEALTH_CARE (0x4898F0): Players[P0] byte +0x152 = P1 != 0.
        0x0414 => {
            collect(s, 2);
            player_info(sp(0)).byte_add(0x152).write(u8::from(sp(1) != 0));
        }
        // 0418 SET_OBJECT_DRAW_LAST (0x489AAE): no null check; entity flags +0x1C bit 0x4000 = P1 != 0.
        0x0418 => {
            collect(s, 2);
            let obj = object_at_ref(pool(OBJECT_POOL), spi(0));
            let p = obj.byte_add(0x1C).cast::<u32>();
            let v = p.read();
            p.write(if sp(1) != 0 { v | 0x4000 } else { v & 0xFFFF_BFFF });
        }
        // 041D SET_NEAR_CLIP (0x489B53): TheCamera.SetNearClipScript(P0).
        0x041D => {
            collect(s, 1);
            set_near_clip_script(GPtr::from_addr(THE_CAMERA), sp(0));
        }
        // 041E SET_RADIO_CHANNEL (0x489B74): P0 == -1: nothing. P0 == 11 is replaced by 12 in
        // ScriptParams[0]; AudioEngine.RetuneRadio(EAX) with `inc al` (low byte + 1, upper bytes of P0 kept).
        0x041E => {
            collect(s, 1);
            let mut ch = sp(0);
            if ch != 0xFFFF_FFFF {
                if ch == 0xB {
                    ch = 0xC;
                    set_sp(0, ch);
                }
                let arg = (ch & 0xFFFF_FF00) | u32::from((ch as u8).wrapping_add(1));
                retune_radio(GPtr::from_addr(AUDIO_ENGINE), arg);
            }
        }
        // 0423 SET_CAR_TRACTION (0x489BAE): no null check; `fld P1` stored (`fstp`) to +0x8A0 when the
        // dword +0x590 is 0, else to +0x794.
        0x0423 => {
            collect(s, 2);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let v = X87::from_f32(spf(1)).to_f32().to_bits();
            let off = if veh.byte_add(0x590).cast::<u32>().read() == 0 {
                0x8A0
            } else {
                0x794
            };
            veh.byte_add(off).cast::<u32>().write(v);
        }
        // 0424 ARE_MEASUREMENTS_IN_METRES (0x489BF3): UpdateCompareFlag(CLocalisation::Metric() != 0).
        0x0424 => {
            let b = metric() != 0;
            set_flag(s, b);
        }
        // 0425 CONVERT_METRES_TO_FEET (0x489C1A): P0 = P0 `fmul [3.3333333]`; StoreParameters(1).
        0x0425 => {
            collect(s, 1);
            set_sp(0, X87::from_f32(spf(0)).fmul_m(f(K_M2FT)).to_f32().to_bits());
            store(s, 1);
        }
        // 0428 SET_CAR_AVOID_LEVEL_TRANSITIONS (0x489C48): no null check; byte +0x3DB bit 2 (0x4) = P1 != 0.
        0x0428 => {
            collect(s, 2);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let p = veh.byte_add(0x3DB);
            let b = p.read();
            p.write(if sp(1) != 0 { b | 0x4 } else { b & 0xFB });
        }
        // 042C SET_TOTAL_NUMBER_OF_MISSIONS (0x489D5B): CStats::SetStatValue(0x94, (float)P0).
        0x042C => {
            collect(s, 1);
            set_stat_value(0x94, X87::from_i32(spi(0)).to_f32().to_bits());
        }
        // 042D CONVERT_METRES_TO_FEET_INT (0x489D82): P0 = `_ftol2(fild P0 * 3.3333333)`; StoreParameters(1).
        0x042D => {
            collect(s, 1);
            set_sp(0, X87::from_i32(spi(0)).fmul_m(f(K_M2FT)).ftol() as u32);
            store(s, 1);
        }
        // 042E REGISTER_FASTEST_TIME (0x489DB4): CStats::RegisterFastestTime(P0, P1).
        0x042E => {
            collect(s, 2);
            register_fastest_time(spi(0), spi(1));
        }
        // 0430 WARP_CHAR_INTO_CAR_AS_PASSENGER (0x489DDA): no null checks. door = 0, or
        // ComputeTargetDoorToEnterAsPassenger(veh, P2) when P2 >= 0 (signed); `m_pIntelligence->
        // FlushImmediately(false)`; a stack `CTaskSimpleCarSetPedInAsPassenger(veh, door, nullptr)` (0x20
        // bytes) with `m_bIsWarpingPedIntoCar` (+0x1C) = 1 set after the constructor; ProcessPed(ped);
        // destructor. The SEH state stores around the object are not ported.
        0x0430 => {
            collect(s, 3);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(1));
            let seat = spi(2);
            let door = if seat >= 0 {
                compute_target_door_to_enter_as_passenger(veh, seat)
            } else {
                0
            };
            flush_immediately(ped.byte_add(0x47C).cast::<GPtr<u8>>().read(), 0);
            let mut task = [0u32; 8];
            let t = GPtr::from_raw(task.as_mut_ptr()).cast::<u8>();
            set_ped_in_as_passenger_ctor(t, veh, door, GPtr::NULL);
            t.byte_add(0x1C).write(1);
            set_ped_in_as_passenger_process_ped(t, ped);
            set_ped_in_as_passenger_dtor(t);
        }
        // 0431 IS_CAR_PASSENGER_SEAT_FREE (0x489E73): no null check; flag = P1 < m_nMaxPassengers (byte
        // +0x488, signed compare) && m_apPassengers[P1] (+0x464) == null.
        0x0431 => {
            collect(s, 2);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let max = i32::from(veh.byte_add(0x488).read());
            let seat = spi(1);
            let b = seat < max
                && veh
                    .byte_add(0x464 + (seat.wrapping_mul(4)) as isize)
                    .cast::<u32>()
                    .read()
                    == 0;
            set_flag(s, b);
        }
        // 0432 GET_CHAR_IN_CAR_PASSENGER_SEAT (0x489EBC): no null checks; P0 = PedPool.GetRef(m_apPassengers[P1]).
        0x0432 => {
            collect(s, 2);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let ped = veh
                .byte_add(0x464 + (spi(1).wrapping_mul(4)) as isize)
                .cast::<GPtr<u8>>()
                .read();
            let h = ped_get_ref(pool(PED_POOL), ped);
            set_sp(0, h as u32);
            store(s, 1);
        }
        // 0434 START_CREDITS (0x489F56) / 0435 STOP_CREDITS (0x489F62)
        0x0434 => credits_start(),
        0x0435 => credits_stop(),
        // 0436 ARE_CREDITS_FINISHED (0x489F6E): flag = CCredits::bCreditsGoing == 0.
        0x0436 => {
            let b = GPtr::<u8>::from_addr(CREDITS_GOING).read() == 0;
            set_flag(s, b);
        }
        // 043C SET_MUSIC_DOES_FADE (0x489F94): byte 0xB6F04D = P0 == 0.
        0x043C => {
            collect(s, 1);
            GPtr::<u8>::from_addr(MUSIC_DOES_FADE_OFF).write(u8::from(sp(0) == 0));
        }
        // 0441 GET_CAR_MODEL (0x489FB3): no null check; P0 = (int16)m_nModelIndex (+0x22, `movsx`).
        0x0441 => {
            collect(s, 1);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            set_sp(
                0,
                i32::from(veh.byte_add(0x22).cast::<u16>().read() as i16) as u32,
            );
            store(s, 1);
        }
        // 0445 ARE_ANY_CAR_CHEATS_ACTIVATED (0x489FEB): flag = any of the bytes 0x96914C, 0x969152,
        // 0x969153 is non-zero (tested in that order).
        0x0445 => {
            let b = CAR_CHEATS.iter().any(|&a| GPtr::<u8>::from_addr(a).read() != 0);
            set_flag(s, b);
        }
        // 0446 SET_CHAR_SUFFERS_CRITICAL_HITS (0x48A023): no null check; flag 0x1000 of +0x470
        // (bNoCriticalHits) = P1 == 0.
        0x0446 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let p = ped.byte_add(0x470).cast::<u32>();
            let flags = p.read();
            p.write(if sp(1) != 0 {
                flags & !0x1000
            } else {
                flags | 0x1000
            });
        }
        _ => {
            return fallback(
                process_commands_1000_to_1099_original,
                "CRunningScript::ProcessCommands1000To1099",
                s,
                command,
            );
        }
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_1000_TO_1099];
