//! `CRunningScript::ProcessCommands1000To1099` (0x489500): complete port (PORT-SCRIPT-2 partial, completed
//! in PORT-SCRIPT-3). Every jump-table case is in [`PORTED`]; the other opcodes of the range reach the
//! original default (-1; the common tail 0x48A1E8 only restores the SEH frame through the stub
//! 0x40133D). Cases follow the 1.0 US bytes and
//! `docs/re/specs/SCRIPT/commands/ProcessCommands1000To1099.md`.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x03E8, 0x03EA, 0x03EB, 0x03ED, 0x03EE, 0x03EF, 0x03F0, 0x03F3, 0x03F4, 0x03F5, 0x03FD, 0x03FE, 0x0400,
    0x0407, 0x0409, 0x040B, 0x040C, 0x040D, 0x0414, 0x0416, 0x0417, 0x0418, 0x041A, 0x041D, 0x041E, 0x0423,
    0x0424, 0x0425, 0x0428, 0x042B, 0x042C, 0x042D, 0x042E, 0x042F, 0x0430, 0x0431, 0x0432, 0x0433, 0x0434,
    0x0435, 0x0436, 0x043C, 0x043E, 0x043F, 0x0440, 0x0441, 0x0445, 0x0446, 0x0448, 0x0449, 0x044B,
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

orig! {
    /// `CWorld::SetAllCarsCanBeDamaged(bool)` (0x5668F0)
    fn set_all_cars_can_be_damaged(b: u32) = cdecl 0x5668F0 => unported;
    /// `CWorld::FindPlayerCoors(int32 player)` (0x56E010): the CVector returned through the hidden
    /// pointer (first argument), which is also returned in EAX
    fn find_player_coors(out: GPtr<u32>, player: i32) -> GPtr<u32> = cdecl 0x56E010 => unported;
    /// `CWorld::ExtinguishAllCarFiresInArea(CVector, float)` (0x566950): vector and float as raw dwords
    fn extinguish_all_car_fires_in_area(x: u32, y: u32, z: u32, radius: u32) = cdecl 0x566950 => unported;
    /// `Multiply3x3(const CMatrix&, const CVector&)` (0x59C790): the CVector returned through the hidden
    /// pointer (first argument), also returned in EAX
    fn multiply_3x3(out: GPtr<u32>, m: GPtr<u8>, v: GPtr<u32>) -> GPtr<u32> = cdecl 0x59C790 => unported;
    /// `CVector::operator+=(const CVector&)` (0x411A00)
    fn vector_add_assign(v: GPtr<u32>, o: GPtr<u8>) = thiscall 0x411A00 => unported;
    /// `CWorld::ClearPedsFromArea(float x1, float y1, float z1, float x2, float y2, float z2)` (0x5667F0);
    /// floats as raw dwords
    fn clear_peds_from_area(x1: u32, y1: u32, z1: u32, x2: u32, y2: u32, z2: u32) = cdecl 0x5667F0 => unported;
    /// `CTaskManager::GetSimplestActiveTask()` (0x6819D0)
    fn get_simplest_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740)
    fn find_active_task_by_type(tm: GPtr<u8>, ty: u32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CTask::GetTaskType()` (vtable slot 0x10)
    fn task_get_type(t: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `CTimer::Suspend()` (0x5619D0)
    fn timer_suspend() = cdecl 0x5619D0 => unported;
    /// `CTimer::Resume()` (0x561A00)
    fn timer_resume() = cdecl 0x561A00 => unported;
    /// `CFileMgr::ChangeDir(const char*)` (0x538730)
    fn file_change_dir(d: GPtr<u8>) = cdecl 0x538730 => unported;
    /// `CFileMgr::SetDir(const char*)` (0x5387D0)
    fn file_set_dir(d: GPtr<u8>) = cdecl 0x5387D0 => unported;
    /// `CFileMgr::SetDirMyDocuments()` (0x538860)
    fn file_set_dir_my_documents() = cdecl 0x538860 => unported;
    /// `CMenuManager::CheckMissionPackValidMenu()` (0x57D720), AL
    fn check_mission_pack_valid_menu(m: GPtr<u8>) -> u8 = thiscall 0x57D720 => unported;
    /// CRT `sprintf` (0x821BB5), called with one `%d` argument
    fn sprintf1(buf: GPtr<u8>, fmt: GPtr<u8>, a: u32) -> i32 = cdecl 0x821BB5 => unported;
    /// `CFileMgr::OpenFile(const char*, const char* mode)` (0x538900)
    fn file_open(name: GPtr<u8>, mode: GPtr<u8>) -> i32 = cdecl 0x538900 => unported;
    /// `CFileMgr::Seek(FILESTREAM, int32 offset, int32 origin)` (0x538990)
    fn file_seek(fd: i32, off: u32, origin: u32) -> i32 = cdecl 0x538990 => unported;
    /// `CFileMgr::Read(FILESTREAM, void*, int32)` (0x538950)
    fn file_read(fd: i32, buf: GPtr<u8>, n: u32) -> i32 = cdecl 0x538950 => unported;
    /// `CFileMgr::CloseFile(FILESTREAM)` (0x5389D0)
    fn file_close(fd: i32) -> i32 = cdecl 0x5389D0 => unported;
    /// `CTheScripts::WipeLocalVariableMemoryForMissionScript()` (0x464BB0)
    fn wipe_local_variable_memory_for_mission_script() = cdecl 0x464BB0 => unported;
    /// `CTheScripts::StartNewScript(uint8* ip)` (0x464C20)
    fn start_new_script(ip: GPtr<u8>) -> GPtr<u8> = cdecl 0x464C20 => crate::script_brains::start_new_script;
    /// `CGameLogic::ClearSkip(bool)` (0x441560)
    fn clear_skip(b: u32) = cdecl 0x441560 => unported;
}

/// `CTheScripts::NumberOfExclusiveMissionScripts` (int16) 0xA444B8.
const NUM_EXCLUSIVE_MISSION_SCRIPTS: usize = 0xA444B8;
/// `CTheScripts::MultiScriptArray[0]` 0xA444C8.
const MULTI_SCRIPT_ARRAY: usize = 0xA444C8;
/// The mission-pack number byte 0xB72910 (`CGame::bMissionPackGame`).
const MISSION_PACK_GAME: usize = 0xB72910;
/// `FrontEndMenuManager` 0xBA6748.
const FRONT_END_MENU_MANAGER: usize = 0xBA6748;
/// The mission-pack path buffer 0xB71670.
const MISSION_PACK_PATH: usize = 0xB71670;
/// `CTheScripts::MissionBlock` 0xA7A6A0 (69,000 = 0x10D88 bytes).
const MISSION_BLOCK: usize = 0xA7A6A0;
/// `CTheScripts::bAlreadyRunningAMissionScript` (byte) 0xA444B1.
const ALREADY_RUNNING_A_MISSION_SCRIPT: usize = 0xA444B1;
/// `.rdata` strings the cases pass: "\\" 0x859F7C, "MPACK//MPACK%d//" 0x859D78, "rb" 0x859C6C,
/// "data\\script\\main.scm" 0x859F64, "" 0x858B54.
const S_BACKSLASH: usize = 0x859F7C;
const S_MPACK_FMT: usize = 0x859D78;
const S_RB: usize = 0x859C6C;
const S_MAIN_SCM: usize = 0x859F64;
const S_EMPTY: usize = 0x858B54;

/// The tail both loaders of 0417 share: `Seek(fd, offset, 0)`, `Read(fd, MissionBlock, 0x10D88)`,
/// `CloseFile(fd)`; returns the read count.
fn read_mission(fd: i32, offset: u32) -> i32 {
    file_seek(fd, offset, 0);
    let n = file_read(fd, GPtr::from_addr(MISSION_BLOCK), 0x10D88);
    file_close(fd);
    n
}

/// 0417's mission start: `WipeLocalVariableMemoryForMissionScript()`; `sc = StartNewScript(MissionBlock)`;
/// `sc->m_UsesMissionCleanup (+0xC6) = 1`, `sc->m_IsMission (+0xDC) = 1`, `sc->m_pBaseIP (+0x10) =
/// MissionBlock`; `bAlreadyRunningAMissionScript = 1`; `ClearSkip(false)`.
fn start_mission_script() {
    wipe_local_variable_memory_for_mission_script();
    let sc = start_new_script(GPtr::from_addr(MISSION_BLOCK));
    sc.byte_add(0xC6).write(1);
    sc.byte_add(0xDC).write(1);
    sc.byte_add(0x10).cast::<u32>().write(MISSION_BLOCK as u32);
    GPtr::<u8>::from_addr(ALREADY_RUNNING_A_MISSION_SCRIPT).write(1);
    clear_skip(0);
}

/// 0448 IS_CHAR_SITTING_IN_CAR / 0449 IS_CHAR_SITTING_IN_ANY_CAR tail: the simplest active task of
/// `m_pIntelligence (+0x47C)->m_TaskMgr (+4)` exists and its type (virtual, called twice) is 0x2C5
/// (both compared by value, as the exe does).
fn sitting_task(ped: GPtr<u8>) -> bool {
    let intel = ped.byte_add(0x47C).cast::<GPtr<u8>>().read();
    let t = get_simplest_active_task(intel.byte_add(4));
    !t.is_null() && (task_get_type(t) == 0x2C5 || task_get_type(t) == 0x3FE)
}

/// 0400 GET_OFFSET_FROM_OBJECT_IN_WORLD_COORDS (0x48979F) / 0407 GET_OFFSET_FROM_CAR_IN_WORLD_COORDS
/// (0x489831) after the lookup: `r = Multiply3x3(*e->m_matrix (+0x14, not null-checked), (P1, P2, P3))`;
/// `r += m_matrix ? m_matrix->pos (+0x30) : m_placement.pos (e + 4)` (CVector::operator+=); P0..P2 = r;
/// `StoreParameters(3)`.
fn offset_in_world(s: Scr, e: GPtr<u8>) {
    let v = [sp(1), sp(2), sp(3)];
    let mut tmp = [0u32; 3];
    let m = e.byte_add(0x14).cast::<GPtr<u8>>().read();
    let r = multiply_3x3(GPtr::from_raw(tmp.as_mut_ptr()), m, GPtr::from_raw(v.as_ptr().cast_mut()));
    let mut out = [r.read(), r.wrapping_index(1).read(), r.wrapping_index(2).read()];
    let m = e.byte_add(0x14).cast::<GPtr<u8>>().read();
    let pos = if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) };
    vector_add_assign(GPtr::from_raw(out.as_mut_ptr()), pos);
    set_sp(0, out[0]);
    set_sp(1, out[1]);
    set_sp(2, out[2]);
    store(s, 3);
}

/// `CRunningScript::ProcessCommands1000To1099(int32 command)` (0x489500, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x489500, conv = thiscall, name = "CRunningScript::ProcessCommands1000To1099", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript")]
pub fn process_commands_1000_to_1099(s: Scr, command: u32) -> i8 {
    match command {
        // 03E8 FLASH_RADAR_BLIP, 03EA SET_GENERATE_CARS_AROUND_CAMERA, 0409 BLOW_UP_RC_BUGGY, 040B
        // IS_FRENCH_GAME, 0416 LOAD_AND_LAUNCH_MISSION, 042F REGISTER_HIGHEST_SCORE, 043E SET_PLAYER_HOOKER,
        // 043F PLAY_END_OF_GAME_TUNE, 0440 STOP_END_OF_GAME_TUNE (0x48A1E2): the jump table goes straight to
        // the epilogue: 0 without reading the operands (gr: nops/unimplemented, 040B's handler reads none).
        0x03E8 | 0x03EA | 0x0409 | 0x040B | 0x0416 | 0x042F | 0x043E | 0x043F | 0x0440 => {}
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
        // 03F4 SET_ALL_CARS_CAN_BE_DAMAGED (0x489683): P0 != 0: SetAllCarsCanBeDamaged(true); else
        // SetAllCarsCanBeDamaged(false), then ExtinguishAllCarFiresInArea(FindPlayerCoors(-1), 4000.0f).
        // gr: unregistered.
        0x03F4 => {
            collect(s, 1);
            if sp(0) != 0 {
                set_all_cars_can_be_damaged(1);
            } else {
                set_all_cars_can_be_damaged(0);
                let mut tmp = [0u32; 3];
                let p = find_player_coors(GPtr::from_raw(tmp.as_mut_ptr()), -1);
                let (x, y, z) = (p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read());
                extinguish_all_car_fires_in_area(x, y, z, 0x457A_0000);
            }
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
        // 0400 GET_OFFSET_FROM_OBJECT_IN_WORLD_COORDS (0x48979F): object not null-checked. gr: unregistered.
        0x0400 => {
            collect(s, 4);
            let obj = object_at_ref(pool(OBJECT_POOL), spi(0));
            offset_in_world(s, obj);
        }
        // 0407 GET_OFFSET_FROM_CAR_IN_WORLD_COORDS (0x489831): vehicle not null-checked. gr: unregistered.
        0x0407 => {
            collect(s, 4);
            let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            offset_in_world(s, veh);
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
        // 0417 LOAD_AND_LAUNCH_MISSION_INTERNAL (0x489921): with `NumberOfExclusiveMissionScripts` (int16) > 0,
        // `P0 <= 0xFFFC` (signed) returns 0 at once, else `P0 = 0xFFFF - P0` (written back). Then
        // `CTimer::Suspend()`; `offset = MultiScriptArray[P0]`; `ChangeDir("\\")`. Mission pack
        // (`[0xB72910] != 0`): `n = 0`; loop while `FrontEndMenuManager.CheckMissionPackValidMenu()`:
        // `SetDirMyDocuments()`; `sprintf(0xB71670, "MPACK//MPACK%d//", pack)`; `fd = OpenFile(0xB71670,
        // "rb")`; `fd > 0`: `n = Seek/Read/Close`, and `n >= 1` starts the mission script; the loop ends
        // when `n >= 1` (a failed open keeps the previous `n`). Then `SetDir("")`; without a mission pack
        // (re-read): OpenFile("data\\script\\main.scm", "rb") (not checked), Seek/Read/Close, start the
        // mission script. `CTimer::Resume()`. gr: the same calls (exe order kept).
        0x0417 => {
            collect(s, 1);
            if GPtr::<i16>::from_addr(NUM_EXCLUSIVE_MISSION_SCRIPTS).read() > 0 {
                if spi(0) <= 0xFFFC {
                    return 0;
                }
                set_sp(0, 0xFFFF_u32.wrapping_sub(sp(0)));
            }
            timer_suspend();
            let offset = GPtr::<u32>::from_addr(MULTI_SCRIPT_ARRAY).wrapping_index(spi(0)).read();
            file_change_dir(GPtr::from_addr(S_BACKSLASH));
            let pack = GPtr::<u8>::from_addr(MISSION_PACK_GAME);
            if pack.read() != 0 {
                let mut n: i32 = 0;
                while check_mission_pack_valid_menu(GPtr::from_addr(FRONT_END_MENU_MANAGER)) != 0 {
                    file_set_dir_my_documents();
                    sprintf1(GPtr::from_addr(MISSION_PACK_PATH), GPtr::from_addr(S_MPACK_FMT), u32::from(pack.read()));
                    let fd = file_open(GPtr::from_addr(MISSION_PACK_PATH), GPtr::from_addr(S_RB));
                    if fd > 0 {
                        n = read_mission(fd, offset);
                        if n < 1 {
                            continue;
                        }
                        start_mission_script();
                    }
                    if n >= 1 {
                        break;
                    }
                }
            }
            file_set_dir(GPtr::from_addr(S_EMPTY));
            if pack.read() == 0 {
                let fd = file_open(GPtr::from_addr(S_MAIN_SCM), GPtr::from_addr(S_RB));
                read_mission(fd, offset);
                start_mission_script();
            }
            timer_resume();
        }
        // 041A GET_AMMO_IN_CHAR_WEAPON (0x489AF6): ped not null-checked; `P0 = 0`, then over the 13 weapon
        // slots (+0x5A0, 0x1C each) every slot whose type equals P1 sets `P0 = m_nTotalAmmo` (+0xC; the
        // last match wins); `StoreParameters(1)`.
        0x041A => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let want = sp(1);
            set_sp(0, 0);
            for i in 0..13 {
                let slot = ped.byte_add(0x5A0 + i * 0x1C);
                if slot.cast::<u32>().read() == want {
                    set_sp(0, slot.byte_add(0xC).cast::<u32>().read());
                }
            }
            store(s, 1);
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
        // 042B CLEAR_AREA_OF_CHARS (0x489C92): each pair (P0,P3), (P1,P4), (P2,P5) ordered by `fld a; fcomp b;
        // test ah,0x41; jne` (swap only when `a > b`, ordered; the moved value goes through `fld; fstp`,
        // exact); ClearPedsFromArea(xmin, ymin, zmin, xmax, ymax, zmax). gr: unregistered.
        0x042B => {
            collect(s, 6);
            let pair = |a: u32, b: u32| if f(a) > f(b) { (b, a) } else { (a, b) };
            let (x1, x2) = pair(sp(0), sp(3));
            let (y1, y2) = pair(sp(1), sp(4));
            let (z1, z2) = pair(sp(2), sp(5));
            clear_peds_from_area(x1, y1, z1, x2, y2, z2);
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
        // 0433 SET_CHAR_IS_CHRIS_CRIMINAL (0x489F05): ped not null-checked; +0x470 bit 0x400 = P1 != 0.
        0x0433 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let p = ped.byte_add(0x470).cast::<u32>();
            let v = p.read();
            p.write(if sp(1) != 0 { v | 0x400 } else { v & !0x400 });
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
        // 0448 IS_CHAR_SITTING_IN_CAR (0x48A074): ped not null-checked; flag = 0 unless "in vehicle" (+0x46C
        // bit 8); only then the vehicle P1 is looked up and compared with `m_pVehicle` (+0x58C); then the
        // simplest active task's type is 0x2C5 or 0x3FE.
        0x0448 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let mut b = false;
            if ped.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0 {
                let veh = vehicle_at_ref(pool(VEHICLE_POOL), spi(1));
                if ped.byte_add(0x58C).cast::<GPtr<u8>>().read().addr() == veh.addr() {
                    b = sitting_task(ped);
                }
            }
            set_flag(s, b);
        }
        // 0449 IS_CHAR_SITTING_IN_ANY_CAR (0x48A102): as 0448 without the vehicle compare.
        0x0449 => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let b = ped.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0 && sitting_task(ped);
            set_flag(s, b);
        }
        // 044B IS_CHAR_ON_FOOT (0x48A174): ped not null-checked; flag = not "in vehicle" (+0x46C bit 8) and
        // neither `FindActiveTaskByType(0x2BC)` nor `FindActiveTaskByType(0x2BD)` of `m_pIntelligence->
        // m_TaskMgr` (in that order, the second only when the first is null).
        0x044B => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let b = if ped.byte_add(0x46C).cast::<u32>().read() & 0x100 != 0 {
                false
            } else {
                let tm = ped.byte_add(0x47C).cast::<GPtr<u8>>().read().byte_add(4);
                find_active_task_by_type(tm, 0x2BC).is_null() && {
                    let tm = ped.byte_add(0x47C).cast::<GPtr<u8>>().read().byte_add(4);
                    find_active_task_by_type(tm, 0x2BD).is_null()
                }
            };
            set_flag(s, b);
        }
        _ => return -1,
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_1000_TO_1099];
