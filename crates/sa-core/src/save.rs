//! SAVE-1: the save game core of gta_sa.exe 1.0 US: `C_PcSave` (`SaveSlot`, `GenerateGameFilename`,
//! `DeleteSlot`), `CGenericGameStorage` (file and work-buffer handling, `GenericSave` / `GenericLoad`, the
//! checks, the before-save / after-load steps), `CSimpleVariablesSaveStructure` (block 0) and the small
//! helpers the save path reaches (`TextCopy`, the `CPad::UpdatePads` thunk 0x541E50).
//!
//! Ported from the bytes (objdump of the exe; `docs/re/specs/SAVE/*.md`; gta-reversed
//! `GenericGameStorage.cpp` / `C_PcSave.cpp` / `SimpleVariablesSaveStructure.cpp` as a reading aid). Where
//! gta-reversed differs, the bytes win (each case is in the function's doc and in docs/arch/save-port.md):
//! the block tag is written / read inline against the literal 0xC800 (`SaveDataToWorkBuffer` compares with
//! 0xC800 too, `LoadDataFromWorkBuffer` with `ms_WorkBufferSize` but splits at `0xC800 - pos`),
//! `GenericLoad` never checks the version number (it calls `GetCurrentVersionNumber` and drops the result),
//! leaves `ms_bLoading` set when a block tag cannot be read, and writes its out flag only once (false).
//!
//! The save file is a 202,748-byte image of the work buffer stream plus a 4-byte checksum (sa-formats
//! `save.rs`). Its block 0 is a `CSimpleVariablesSaveStructure` built in an uninitialised 0x138-byte local of
//! `GenericSave`: the bytes `Construct` never writes are the stack residue the callers of `GenericSave` left
//! there. The port takes them from its footprint wrapper's entry snapshot (ADR-001 §5b), which is exactly the
//! original's residue whenever `GenericSave`'s caller chain is original code (L2 / split); with the callers on
//! Rust the residue is theirs (docs/arch/save-port.md, ledger).

use crate::abi::footprint_residue_u8;
use crate::mem::GPtr;
use crate::{hook, native, orig};

/// `CGenericGameStorage::ms_WorkBuffer` (u8*, `operator new(0xC801)`)
const WORK_BUFFER: usize = 0xC16EE8;
/// `ms_WorkBufferPos`
const WORK_BUFFER_POS: usize = 0xC16EEC;
/// `ms_FileHandle`
const FILE_HANDLE: usize = 0xC16EF0;
/// `ms_FilePos`
const FILE_POS: usize = 0xC16EF4;
/// `ms_FileSize`
const FILE_SIZE: usize = 0xC16EF8;
/// `ms_bFailed`
const FAILED: usize = 0xC16EFC;
/// `ms_bLoading`
const LOADING: usize = 0xC16EFD;
/// `ms_WorkBufferSize` (.data, image value 0xC800)
const WORK_BUFFER_SIZE: usize = 0x8D2BE0;
/// `ms_CheckSum`
const CHECKSUM: usize = 0xC16134;
/// `PcSaveHelper` (`C_PcSave`; its first dword is the error code)
pub const PC_SAVE_HELPER: usize = 0xC17034;
/// `ms_LoadFileName` / `ms_LoadFileNameWithPath`
const LOAD_FILE_NAME: usize = 0xC15FC8;
const LOAD_FILE_NAME_WITH_PATH: usize = 0xC15F60;
/// `ms_SaveFileName` / `ms_SaveFileNameJustSaved`
const SAVE_FILE_NAME: usize = 0xC16DB8;
const SAVE_FILE_NAME_JUST_SAVED: usize = 0xC16030;
/// `ms_SlotFileName` (stride 0x104) / `ms_SlotSaveDate` (stride 0x46)
const SLOT_FILE_NAME: usize = 0xC16368;
const SLOT_SAVE_DATE: usize = 0xC16138;
/// `ms_BlockTagName` ("BLOCK", 5 bytes written / compared)
const BLOCK_TAG: usize = 0x8D2BD8;
/// `C_PcSave::DefaultPCSaveFileName`
const DEFAULT_PC_SAVE_FILE_NAME: usize = 0xC16F18;
/// `JustLoadedDontFadeInYet` / `StillToFadeOut`
const JUST_LOADED_DONT_FADE_IN_YET: usize = 0xC16EDC;
const STILL_TO_FADE_OUT: usize = 0xC16EDD;
/// `TheText`, `TheCamera`, `ThePaths`
const THE_TEXT: usize = 0xC1B340;
const THE_CAMERA: usize = 0xB6F028;
const THE_PATHS: usize = 0x96F050;
/// `CWorld::Players` (stride 0x190; +0 `m_pPed`), `CWorld::PlayerInFocus`
const PLAYERS: usize = 0xB7CD98;
const PLAYER_IN_FOCUS: usize = 0xB7CD74;

/// The work buffer's size as the code spells it (`cmp ecx, 0xC800`, `mov esi, 0xC800`).
const BUFFER_SIZE: u32 = 0xC800;
/// `SIZE_OF_ONE_GAME_IN_BYTES` (`cmp eax, 0x317FC`).
const SIZE_OF_ONE_GAME: u32 = 0x317FC;
/// The footprint window of `GenericSave` (bytes below its entry ESP; ADR-001 §5b).
const GENERIC_SAVE_WINDOW: u32 = 0x1000;

// .rdata strings pushed by address
const FMT_S_I_S: usize = 0x86BAD4; // "%s%i%s"
const STR_DOT_B: usize = 0x86BADC; // ".b"
const FMT_S_I: usize = 0x86BAE0; // "%s%i"
const FMT_S_S: usize = 0x86BAE8; // "%s%s"
const STR_BUILD_DATE: usize = 0x86BAF0; // "Apr 28 2005"
const STR_BUILD_TIME: usize = 0x86BAFC; // "10:28:55"
const FMT_S_I_DOT_B: usize = 0x86D2F4; // "%s%i.b"
const MODE_RB: usize = 0x859C6C; // "rb"
const MODE_WB: usize = 0x860ADC; // "wb"

fn g<T>(a: usize) -> GPtr<T> {
    GPtr::from_addr(a)
}

fn set_error(e: u32) {
    g::<u32>(PC_SAVE_HELPER).write(e);
}

fn failed() -> bool {
    g::<u8>(FAILED).read() != 0
}

fn work_buffer() -> GPtr<u8> {
    g::<GPtr<u8>>(WORK_BUFFER).read()
}

/// `operator delete(ms_WorkBuffer); ms_WorkBuffer = nullptr` when it is set (inlined in several places).
fn free_work_buffer() {
    let b = work_buffer();
    if !b.is_null() {
        call::free(b);
        g::<u32>(WORK_BUFFER).write(0);
    }
}

fn file_handle() -> GPtr<u8> {
    g::<GPtr<u8>>(FILE_HANDLE).read()
}

pub mod call {
    use super::*;
    orig! {
        /// CRT `sprintf(buf, fmt, char*, int, char*)` (0x821BB5)
        pub fn sprintf_sis(buf: GPtr<u8>, fmt: GPtr<u8>, a: GPtr<u8>, n: i32, b: GPtr<u8>) -> i32 = cdecl 0x821BB5 => unported;
        /// CRT `sprintf(buf, fmt, char*, int)` (0x821BB5)
        pub fn sprintf_si(buf: GPtr<u8>, fmt: GPtr<u8>, a: GPtr<u8>, n: i32) -> i32 = cdecl 0x821BB5 => unported;
        /// CRT `sprintf(buf, fmt, char*, char*)` (0x821BB5)
        pub fn sprintf_ss(buf: GPtr<u8>, fmt: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x821BB5 => unported;
        /// CRT `strncpy` (0x821F40)
        pub fn strncpy(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
        /// CRT `strncmp` (0x8214D0)
        pub fn strncmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x8214D0 => unported;
        /// `operator new` (0x821195)
        pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x821195 => unported;
        /// `operator delete` / `free` as the save code calls it (0x8213AE)
        pub fn free(p: GPtr<u8>) = cdecl 0x8213AE => unported;
        /// `CFileMgr::OpenFile(path, mode)` (0x538900)
        pub fn open_file(path: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x538900 => crate::filemgr::open_file;
        /// `CFileMgr::Read` (0x538950)
        pub fn read(f: GPtr<u8>, buf: GPtr<u8>, size: u32) -> u32 = cdecl 0x538950 => crate::filemgr::read;
        /// `CFileMgr::Write` (0x538970)
        pub fn write(f: GPtr<u8>, buf: GPtr<u8>, size: u32) -> u32 = cdecl 0x538970 => crate::filemgr::write;
        /// `CFileMgr::CloseFile` (0x5389D0)
        pub fn close_file(f: GPtr<u8>) -> i32 = cdecl 0x5389D0 => crate::filemgr::close_file;
        /// `CFileMgr::GetTotalSize` (0x5389E0)
        pub fn get_total_size(f: GPtr<u8>) -> i32 = cdecl 0x5389E0 => crate::filemgr::get_total_size;
        /// `CFileMgr::GetErrorReadWrite` (0x538A50)
        pub fn get_error_read_write(f: GPtr<u8>) -> u32 = cdecl 0x538A50 => crate::filemgr::get_error_read_write;
        /// `CFileMgr::InitUserDirectories` (0x744FB0)
        pub fn init_user_directories() -> GPtr<u8> = cdecl 0x744FB0 => crate::platform::init_user_directories;
        /// `CKeyGen::GetKey(const char*)` (0x53CF00)
        pub fn get_key(s: GPtr<u8>) -> u32 = cdecl 0x53CF00 => crate::keygen::get_key;
        /// KERNEL32 `DeleteFileA` at 0x619103 (C_PcSave::DeleteSlot)
        pub fn delete_file_a(path: GPtr<u8>) -> i32 = stdcall site 0x619103 => unported;
        /// KERNEL32 `GetLocalTime` at 0x5D1E54 (determinism mode redirects this site to the virtual clock)
        pub fn get_local_time(out: GPtr<u8>) = stdcall site 0x5D1E54 => unported;
    }
    orig! {
        /// `CGenericGameStorage::OpenFileForReading(name, slot)` (0x5D0D20)
        pub fn open_file_for_reading(name: GPtr<u8>, slot: i32) -> u8 = cdecl 0x5D0D20 => super::open_file_for_reading;
        /// `CGenericGameStorage::OpenFileForWriting()` (0x5D0DD0)
        pub fn open_file_for_writing() -> u8 = cdecl 0x5D0DD0 => super::open_file_for_writing;
        /// `CGenericGameStorage::CloseFile()` (0x5D0E30)
        pub fn storage_close_file() -> u8 = cdecl 0x5D0E30 => super::close_file;
        /// `CGenericGameStorage::MakeValidSaveName(slot)` (0x5D0E90)
        pub fn make_valid_save_name(slot: i32) = cdecl 0x5D0E90 => super::make_valid_save_name;
        /// `CGenericGameStorage::GetCurrentVersionNumber()` (0x5D0F50)
        pub fn get_current_version_number() -> u32 = cdecl 0x5D0F50 => super::get_current_version_number;
        /// `CGenericGameStorage::SaveWorkBuffer(bool)` (0x5D0F80)
        pub fn save_work_buffer(checksum: u32) -> u8 = cdecl 0x5D0F80 => super::save_work_buffer;
        /// `CGenericGameStorage::LoadWorkBuffer()` (0x5D10B0)
        pub fn load_work_buffer() -> u8 = cdecl 0x5D10B0 => super::load_work_buffer;
        /// `CGenericGameStorage::CheckDataNotCorrupt(slot, name)` (0x5D1170)
        pub fn check_data_not_corrupt(slot: i32, name: GPtr<u8>) -> u8 = cdecl 0x5D1170 => super::check_data_not_corrupt;
        /// `CGenericGameStorage::SaveDataToWorkBuffer(void*, int32)` (0x5D1270)
        pub fn save_data(p: GPtr<u8>, n: i32) -> u8 = cdecl 0x5D1270 => super::save_data_to_work_buffer;
        /// `CGenericGameStorage::LoadDataFromWorkBuffer(void*, int32)` (0x5D1300)
        pub fn load_data(p: GPtr<u8>, n: i32) -> u8 = cdecl 0x5D1300 => super::load_data_from_work_buffer;
        /// `CGenericGameStorage::GenericSave()` (0x5D13E0)
        pub fn generic_save() -> u8 = cdecl 0x5D13E0 => super::generic_save;
        /// `CGenericGameStorage::ReportError(block, error)` (0x5D08C0)
        pub fn report_error(block: i32, error: i32) = cdecl 0x5D08C0 => super::report_error;
        /// `CSimpleVariablesSaveStructure::Construct()` (0x5D1B80)
        pub fn simple_vars_construct(this: GPtr<u8>) = thiscall 0x5D1B80 => super::simple_vars_construct;
        /// `CSimpleVariablesSaveStructure::Extract(uint32& version)` (0x5D1EA0)
        pub fn simple_vars_extract(this: GPtr<u8>, version: GPtr<u32>) = thiscall 0x5D1EA0 => super::simple_vars_extract;
        /// `CGenericGameStorage::DoGameSpecificStuffAfterSucessLoad()` (0x618E90)
        pub fn after_success_load() = cdecl 0x618E90 => super::do_game_specific_stuff_after_success_load;
        /// `CGenericGameStorage::DoGameSpecificStuffBeforeSave()` (0x618F50)
        pub fn before_save() = cdecl 0x618F50 => super::do_game_specific_stuff_before_save;
        /// `C_PcSave::GenerateGameFilename(slot, out)` (0x6190A0)
        pub fn generate_game_filename(this: GPtr<u8>, slot: i32, out: GPtr<u8>) = thiscall 0x6190A0 => super::generate_game_filename;
        /// `TextCopy(GxtChar* dst, const GxtChar* src)` (0x69F910)
        pub fn text_copy(dst: GPtr<u8>, src: GPtr<u8>) = cdecl 0x69F910 => super::text_copy;
        /// `CPad::UpdatePads()` through its thunk (0x541E50)
        pub fn update_pads_thunk() = cdecl 0x541E50 => super::update_pads_thunk;
        /// `CPad::UpdatePads()` (0x541DD0)
        pub fn update_pads() = cdecl 0x541DD0 => unported;
    }
    // GenericSave / GenericLoad block functions (eBlocks order), and the steps around them.
    orig! {
        pub fn cheat_reset_cheats() = cdecl 0x438450 => unported;
        pub fn script_connect_lods_function(a: i32, b: i32) = cdecl 0x470A20 => unported;
        pub fn streaming_delete_all_rw_objects() = cdecl 0x4090A0 => unported;
        pub fn scripts_save() -> u8 = cdecl 0x5D4C40 => unported;
        pub fn scripts_load() -> u8 = cdecl 0x5D4FD0 => unported;
        pub fn pools_save() -> u8 = cdecl 0x5D0880 => crate::pools::save;
        pub fn pools_load() -> u8 = cdecl 0x5D0890 => crate::pools::load;
        pub fn scripts_setup_after_pools_have_loaded() = cdecl 0x5D3390 => super::do_script_setup_after_pools_have_loaded;
        pub fn garages_save() -> u8 = cdecl 0x5D3160 => unported;
        pub fn garages_load() -> u8 = cdecl 0x5D3270 => unported;
        pub fn game_logic_save() -> u8 = cdecl 0x5D33C0 => unported;
        pub fn game_logic_load() -> u8 = cdecl 0x5D3440 => unported;
        pub fn path_find_save(this: GPtr<u8>) -> u8 = thiscall 0x5D34C0 => unported;
        pub fn path_find_load(this: GPtr<u8>) -> u8 = thiscall 0x5D3500 => unported;
        pub fn pickups_save() -> u8 = cdecl 0x5D3540 => unported;
        pub fn pickups_load() -> u8 = cdecl 0x5D35A0 => unported;
        pub fn restart_save() -> u8 = cdecl 0x5D3620 => unported;
        pub fn restart_load() -> u8 = cdecl 0x5D3770 => unported;
        pub fn radar_save() -> u8 = cdecl 0x5D5860 => unported;
        pub fn radar_load() -> u8 = cdecl 0x5D53C0 => unported;
        pub fn zones_save() -> u8 = cdecl 0x5D2E60 => unported;
        pub fn zones_load() -> u8 = cdecl 0x5D2F40 => unported;
        pub fn gangs_save() -> u8 = cdecl 0x5D3A60 => unported;
        pub fn gangs_load() -> u8 = cdecl 0x5D3A90 => unported;
        pub fn car_generators_save() -> u8 = cdecl 0x5D38C0 => unported;
        pub fn car_generators_load() -> u8 = cdecl 0x5D39B0 => unported;
        pub fn player_info_save(this: GPtr<u8>) -> u8 = thiscall 0x5D3AC0 => unported;
        pub fn player_info_load(this: GPtr<u8>) -> u8 = thiscall 0x5D3B00 => unported;
        pub fn stats_save() -> u8 = cdecl 0x5D3B40 => unported;
        pub fn stats_load() -> u8 = cdecl 0x5D3BF0 => unported;
        pub fn set_pieces_save() -> u8 = cdecl 0x5D3C70 => unported;
        pub fn set_pieces_load() -> u8 = cdecl 0x5D3CA0 => unported;
        pub fn streaming_save() -> u8 = cdecl 0x5D29A0 => unported;
        pub fn streaming_load() -> u8 = cdecl 0x5D29E0 => unported;
        pub fn ped_type_save() -> u8 = cdecl 0x5D3CD0 => unported;
        pub fn ped_type_load() -> u8 = cdecl 0x5D3D10 => unported;
        pub fn tag_manager_save() -> u8 = cdecl 0x5D3D60 => unported;
        pub fn tag_manager_load() -> u8 = cdecl 0x5D3DA0 => unported;
        pub fn ipl_store_save() -> u8 = cdecl 0x5D5420 => unported;
        pub fn ipl_store_load() -> u8 = cdecl 0x5D54A0 => unported;
        pub fn shopping_save() -> u8 = cdecl 0x5D3DE0 => unported;
        pub fn shopping_load() -> u8 = cdecl 0x5D3E40 => unported;
        pub fn gang_wars_save() -> u8 = cdecl 0x5D5530 => unported;
        pub fn gang_wars_load() -> u8 = cdecl 0x5D3EB0 => unported;
        pub fn stunt_jumps_save() -> u8 = cdecl 0x5D5570 => unported;
        pub fn stunt_jumps_load() -> u8 = cdecl 0x5D5920 => unported;
        pub fn entry_exits_save() -> u8 = cdecl 0x5D5970 => unported;
        pub fn entry_exits_load() -> u8 = cdecl 0x5D55C0 => unported;
        pub fn radio_tracks_save() -> u8 = cdecl 0x5D3EE0 => unported;
        pub fn radio_tracks_load() -> u8 = cdecl 0x5D40E0 => unported;
        pub fn user_3d_markers_save() -> u8 = cdecl 0x5D4300 => unported;
        pub fn user_3d_markers_load() -> u8 = cdecl 0x5D42E0 => unported;
    }
    // DoGameSpecificStuffBeforeSave / AfterSucessLoad callees
    orig! {
        pub fn ropes_shutdown() = cdecl 0x556B10 => unported;
        pub fn pickups_remove_pickup_objects() = cdecl 0x455470 => unported;
        pub fn gang_wars_end_gang_war(end: u32) = cdecl 0x4464C0 => unported;
        pub fn stats_increment_stat(stat: u32, value: f32) = cdecl 0x55C180 => unported;
        pub fn game_logic_pass_time(minutes: u32) = cdecl 0x4414C0 => unported;
        pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
        pub fn player_ped_reset_sprint_energy(ped: GPtr<u8>) = thiscall 0x60A530 => unported;
        pub fn player_ped_set_wanted_level(ped: GPtr<u8>, level: i32) = thiscall 0x609F10 => unported;
        pub fn game_tidy_up_memory(a: u32, b: u32) = cdecl 0x53C500 => unported;
        pub fn text_load(this: GPtr<u8>, keep_mission_packs: u32) = thiscall 0x6A01A0 => unported;
        pub fn collision_sort_out_collision_after_load() = cdecl 0x411E30 => unported;
        pub fn streaming_load_scene_collision(pos: GPtr<u8>) = cdecl 0x40ED80 => unported;
        pub fn streaming_load_scene(pos: GPtr<u8>) = cdecl 0x40EB70 => unported;
        pub fn streaming_load_all_requested_models(priority: u32) = cdecl 0x40EA10 => unported;
        pub fn camera_fade(this: GPtr<u8>, time: f32, dir: u32) = thiscall 0x50AC20 => unported;
        pub fn scripts_process() = cdecl 0x46A000 => unported;
        pub fn tag_manager_update_num_tagged() = cdecl 0x49CDE0 => unported;
        pub fn clothes_rebuild_player(ped: GPtr<u8>, ignore_fat_and_muscle: u32) = cdecl 0x5A82C0 => unported;
        pub fn pop_cycle_update() = cdecl 0x610BF0 => unported;
        pub fn streaming_remove_inappropriate_ped_models() = cdecl 0x40B3A0 => unported;
        pub fn gang_wars_clear_specific_zones_to_trigger_gang_war() = cdecl 0x443FF0 => unported;
        pub fn zones_fill_zones_with_gang_colours(disable_radar: u32) = cdecl 0x572440 => unported;
    }
    // CSimpleVariablesSaveStructure
    orig! {
        pub fn text_get(this: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
        pub fn ascii_to_gxt_char(src: GPtr<u8>, dst: GPtr<u8>) = cdecl 0x718600 => crate::common::ascii_to_gxt_char;
        pub fn gxt_char_strlen(s: GPtr<u8>) -> i32 = cdecl 0x718690 => unported;
        pub fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
        pub fn matrix_set_translate(m: GPtr<u8>, x: f32, y: f32, z: f32) = thiscall 0x59AF40 => unported;
        /// the cheat function `CCheat::m_aCheatFunctions` entry at 0x8A5CC4 (+0x135 of the structure)
        pub fn cheat_fn_8a5cc4() = cdecl cell 0x8A5CC4 => unported;
        /// the cheat function entry at 0x8A5CC0 (+0x136 of the structure)
        pub fn cheat_fn_8a5cc0() = cdecl cell 0x8A5CC0 => unported;
    }
}

// ---------------------------------------------------------------------------- files ---------------

/// `CGenericGameStorage::ReportError(eBlocks, eSaveLoadError)` (0x5D08C0): copies the block's name into a
/// local (jump table 0x5D0CAC; a block above 0x1B leaves it uninitialised) and, for error 0 / 1 / 2,
/// `sprintf`s "Saving error: %s" / "Loading error: %s" / "Loading sync error: %s" into a second local. Nothing
/// is output or stored: the port has no observable effect (the message stays a local). The name copy runs
/// through a protection chunk (`jmp 0x40A405`, which reloads the error argument and jumps back to 0x5D0C46).
#[hook(addr = 0x5D08C0, conv = cdecl, name = "CGenericGameStorage::ReportError", expect = "8B 44 24 04 81 EC 8C 00 00 00",
       unverified = "save-core-1", route = "L3-only", hazards = "none",
       note = "error paths only (a corrupt or truncated save); no observable effect")]
pub fn report_error(_block: i32, _error: i32) {}

/// `CGenericGameStorage::OpenFileForReading(const char* name, int32 slot)` (0x5D0D20): with a name, it is
/// copied to `ms_LoadFileName` and `ms_LoadFileNameWithPath = sprintf("%s%i%s", DefaultPCSaveFileName, slot +
/// 1, ".b")` (the file opened is `ms_LoadFileName`, either way). `CFileMgr::OpenFile(ms_LoadFileName, "rb")`;
/// failure: error 4, false. Else size, `ms_WorkBufferSize = ms_WorkBufferPos = 0xC800`, `ms_FilePos = 0`, the
/// work buffer allocated (0xC801) if missing; true.
#[hook(addr = 0x5D0D20, conv = cdecl, name = "CGenericGameStorage::OpenFileForReading", expect = "8B 44 24 04 85 C0 74 34",
       route = "split", hazards = "heap-writes", recs = "save-load")]
pub fn open_file_for_reading(name: GPtr<u8>, slot: i32) -> u8 {
    if !name.is_null() {
        crate::filemgr::strcpy(g(LOAD_FILE_NAME), name);
        call::sprintf_sis(g(LOAD_FILE_NAME_WITH_PATH), g(FMT_S_I_S), g(DEFAULT_PC_SAVE_FILE_NAME), slot.wrapping_add(1), g(STR_DOT_B));
    }
    let h = call::open_file(g(LOAD_FILE_NAME), g(MODE_RB));
    g::<GPtr<u8>>(FILE_HANDLE).write(h);
    if h.is_null() {
        set_error(4);
        return 0;
    }
    let size = call::get_total_size(h);
    g::<i32>(FILE_SIZE).write(size);
    g::<u32>(WORK_BUFFER_SIZE).write(BUFFER_SIZE);
    g::<u32>(WORK_BUFFER_POS).write(BUFFER_SIZE);
    g::<u32>(FILE_POS).write(0);
    if work_buffer().is_null() {
        let b = call::operator_new(BUFFER_SIZE + 1);
        g::<GPtr<u8>>(WORK_BUFFER).write(b);
    }
    1
}

/// `CGenericGameStorage::OpenFileForWriting()` (0x5D0DD0): `OpenFile(ms_SaveFileName, "wb")`; failure: error
/// 4, false. Else `ms_FilePos = ms_WorkBufferPos = 0` and the work buffer allocated if missing
/// (`ms_WorkBufferSize` is not reset); true.
#[hook(addr = 0x5D0DD0, conv = cdecl, name = "CGenericGameStorage::OpenFileForWriting", expect = "68 DC 0A 86 00",
       route = "split", hazards = "heap-writes", recs = "save-load")]
pub fn open_file_for_writing() -> u8 {
    let h = call::open_file(g(SAVE_FILE_NAME), g(MODE_WB));
    g::<GPtr<u8>>(FILE_HANDLE).write(h);
    if h.is_null() {
        set_error(4);
        return 0;
    }
    g::<u32>(FILE_POS).write(0);
    g::<u32>(WORK_BUFFER_POS).write(0);
    if work_buffer().is_null() {
        let b = call::operator_new(BUFFER_SIZE + 1);
        g::<GPtr<u8>>(WORK_BUFFER).write(b);
    }
    1
}

/// `CGenericGameStorage::CloseFile()` (0x5D0E30): frees the work buffer, `CFileMgr::CloseFile`; true when
/// that returned 0.
#[hook(addr = 0x5D0E30, conv = cdecl, name = "CGenericGameStorage::CloseFile", expect = "A1 E8 6E C1 00",
       unverified = "save-core-1", route = "split", hazards = "heap-writes", recs = "save-load")]
pub fn close_file() -> u8 {
    free_work_buffer();
    u8::from(call::close_file(file_handle()) == 0)
}

/// Replaces every '?' by ' ' from `s` up to the first NUL or '\n' (inlined in `MakeValidSaveName`).
fn question_marks_to_spaces(s: GPtr<u8>) {
    let mut p = s;
    loop {
        let c = p.read();
        if c == 0 || c == b'\n' {
            break;
        }
        if c == b'?' {
            p.write(b' ');
        }
        p = p.byte_add(1);
    }
}

/// `CGenericGameStorage::MakeValidSaveName(int32 slot)` (0x5D0E90): `buf[0] = 0`, `sprintf(buf, "%s%i",
/// DefaultPCSaveFileName, slot + 1)`; a name of 0x101+ characters is cut at 0x101; ".b" appended; copied to
/// `ms_SaveFileName`, whose '?' become ' ' up to the first NUL or '\n'.
#[hook(addr = 0x5D0E90, conv = cdecl, name = "CGenericGameStorage::MakeValidSaveName", expect = "81 EC 04 01 00 00",
       route = "split", hazards = "none", recs = "save-load")]
pub fn make_valid_save_name(slot: i32) {
    let mut buf = [0u8; 0x104];
    let b = GPtr::<u8>::from_raw(buf.as_mut_ptr());
    call::sprintf_si(b, g(FMT_S_I), g(DEFAULT_PC_SAVE_FILE_NAME), slot.wrapping_add(1));
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    if len >= 0x101 {
        buf[0x101] = 0;
    }
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    buf[end..end + 3].copy_from_slice(b".b\0");
    crate::filemgr::strcpy(g(SAVE_FILE_NAME), GPtr::from_raw(buf.as_mut_ptr()));
    question_marks_to_spaces(g(SAVE_FILE_NAME));
}

/// `CGenericGameStorage::GetNameOfSavedGame(int32 slot)` (0x5D0F40): `ms_SlotFileName + 0x104 * slot`.
#[hook(addr = 0x5D0F40, conv = cdecl, name = "CGenericGameStorage::GetNameOfSavedGame", expect = "8B 44 24 04 69 C0 04 01 00 00",
       route = "L1-pure", hazards = "none")]
pub fn get_name_of_saved_game(slot: i32) -> u32 {
    (SLOT_FILE_NAME as u32).wrapping_add((slot as u32).wrapping_mul(0x104))
}

/// `CGenericGameStorage::GetCurrentVersionNumber()` (0x5D0F50): `CKeyGen::GetKey(sprintf("%s%s", "Apr 28
/// 2005", "10:28:55"))`.
#[hook(addr = 0x5D0F50, conv = cdecl, name = "CGenericGameStorage::GetCurrentVersionNumber", expect = "83 EC 28 68 FC BA 86 00",
       route = "split", hazards = "none", recs = "save-load")]
pub fn get_current_version_number() -> u32 {
    let mut buf = [0u8; 0x28];
    let b = GPtr::<u8>::from_raw(buf.as_mut_ptr());
    call::sprintf_ss(b, g(FMT_S_S), g(STR_BUILD_DATE), g(STR_BUILD_TIME));
    call::get_key(b)
}

/// The failure tail of `SaveWorkBuffer` (0x5D104B): error 0xB, the buffer freed, `CloseFile` (error 6 when
/// it fails), `strncpy(ms_SaveFileNameJustSaved, ms_SaveFileName, 0x103)`, `ms_bFailed = 1`.
fn save_work_buffer_fail() -> u8 {
    set_error(0xB);
    free_work_buffer();
    if call::close_file(file_handle()) != 0 {
        set_error(6);
    }
    call::strncpy(g(SAVE_FILE_NAME_JUST_SAVED), g(SAVE_FILE_NAME), 0x103);
    g::<u8>(FAILED).write(1);
    0
}

/// `CGenericGameStorage::SaveWorkBuffer(bool includeChecksum)` (0x5D0F80): false when failed; true at once
/// for an empty buffer. Adds every buffered byte to `ms_CheckSum`; with the checksum flag, a buffer above
/// 0xC7FC is flushed first by a recursive `SaveWorkBuffer(false)` (its bytes are summed twice, original bug)
/// and the checksum dword is appended. `CFileMgr::Write`; an I/O error or a short write fails (see
/// [`save_work_buffer_fail`]); else `ms_FilePos += pos`, `pos = 0`, true.
#[hook(addr = 0x5D0F80, conv = cdecl, name = "CGenericGameStorage::SaveWorkBuffer", expect = "A0 FC 6E C1 00",
       route = "split", hazards = "heap-writes", recs = "save-load")]
pub fn save_work_buffer(include_checksum: u32) -> u8 {
    if failed() {
        return 0;
    }
    let n = g::<u32>(WORK_BUFFER_POS).read();
    if n == 0 {
        return 1;
    }
    let buf = work_buffer();
    let mut i = 0u32;
    while i < n {
        let s = g::<u32>(CHECKSUM).read().wrapping_add(u32::from(buf.byte_add(i as isize).read()));
        g::<u32>(CHECKSUM).write(s);
        i += 1;
    }
    let (mut pos, mut buf) = (n, buf);
    if include_checksum as u8 != 0 {
        if pos > 0xC7FC {
            call::save_work_buffer(0);
            pos = g::<u32>(WORK_BUFFER_POS).read();
            buf = work_buffer();
        }
        buf.byte_add(pos as isize).cast::<u32>().write(g::<u32>(CHECKSUM).read());
        pos = g::<u32>(WORK_BUFFER_POS).read().wrapping_add(4);
        buf = work_buffer();
        g::<u32>(WORK_BUFFER_POS).write(pos);
    }
    let w = call::write(file_handle(), buf, pos);
    if call::get_error_read_write(file_handle()) != 0 {
        return save_work_buffer_fail();
    }
    let pos = g::<u32>(WORK_BUFFER_POS).read();
    if w != pos {
        return save_work_buffer_fail();
    }
    g::<u32>(FILE_POS).write(g::<u32>(FILE_POS).read().wrapping_add(pos));
    g::<u32>(WORK_BUFFER_POS).write(0);
    1
}

/// `CGenericGameStorage::LoadWorkBuffer()` (0x5D10B0): false when failed. `size = ms_FilePos + 0xC800 >
/// ms_FileSize ? ms_FileSize - ms_FilePos : 0xC800` (unsigned); 0 or not a multiple of 4: false without any
/// error. `CFileMgr::Read`; an I/O error or a short read: error 5, `CloseFile()` (error 6 when it fails),
/// `ms_bFailed = 1`, false. Else `ms_FilePos += size`, `ms_WorkBufferSize = size`, `pos = 0`, true.
#[hook(addr = 0x5D10B0, conv = cdecl, name = "CGenericGameStorage::LoadWorkBuffer", expect = "A0 FC 6E C1 00",
       route = "split", hazards = "heap-writes", recs = "save-load")]
pub fn load_work_buffer() -> u8 {
    if failed() {
        return 0;
    }
    let pos = g::<u32>(FILE_POS).read();
    let total = g::<u32>(FILE_SIZE).read();
    let size = if pos.wrapping_add(BUFFER_SIZE) > total { total.wrapping_sub(pos) } else { BUFFER_SIZE };
    if size == 0 || size != size.wrapping_add(3) & 0xFFFF_FFFC {
        return 0;
    }
    let r = call::read(file_handle(), work_buffer(), size);
    if call::get_error_read_write(file_handle()) != 0 || r != size {
        set_error(5);
        if call::storage_close_file() == 0 {
            set_error(6);
        }
        g::<u8>(FAILED).write(1);
        return 0;
    }
    g::<u32>(FILE_POS).write(g::<u32>(FILE_POS).read().wrapping_add(size));
    g::<u32>(WORK_BUFFER_SIZE).write(size);
    g::<u32>(WORK_BUFFER_POS).write(0);
    1
}

/// `CGenericGameStorage::CheckDataNotCorrupt(int32 slot, const char* name)` (0x5D1170): `ms_bFailed = 0`;
/// `OpenFileForReading(name, slot)` or false; every chunk `LoadWorkBuffer` reads is summed (u8 each). Then
/// an error code, or a last chunk shorter than 4 bytes: buffer freed, file closed, false. Else the stored
/// dword (the last 4 bytes of the last chunk) is compared with the sum minus those 4 bytes; buffer freed,
/// file closed (result ignored).
#[hook(addr = 0x5D1170, conv = cdecl, name = "CGenericGameStorage::CheckDataNotCorrupt", expect = "51 8B 44 24 08",
       route = "split", hazards = "heap-writes", recs = "save-load")]
pub fn check_data_not_corrupt(slot: i32, name: GPtr<u8>) -> u8 {
    g::<u8>(FAILED).write(0);
    if call::open_file_for_reading(name, slot) == 0 {
        return 0;
    }
    let mut sum = 0u32;
    while call::load_work_buffer() != 0 {
        let n = g::<u32>(WORK_BUFFER_SIZE).read();
        let buf = work_buffer();
        let mut i = 0u32;
        while i < n {
            sum = sum.wrapping_add(u32::from(buf.byte_add(i as isize).read()));
            i += 1;
        }
    }
    let n = g::<u32>(WORK_BUFFER_SIZE).read();
    if n < 4 || g::<u32>(PC_SAVE_HELPER).read() != 0 {
        free_work_buffer();
        call::close_file(file_handle());
        return 0;
    }
    let buf = work_buffer();
    let tail = buf.byte_add(n as isize - 4);
    let stored = tail.cast::<u32>().read();
    let mut t = 0u32;
    for k in 0..4 {
        t = t.wrapping_add(u32::from(tail.byte_add(k).read()));
    }
    sum = sum.wrapping_sub(t);
    free_work_buffer();
    call::close_file(file_handle());
    u8::from(sum == stored)
}

/// `CGenericGameStorage::SaveDataToWorkBuffer(void*, int32)` (0x5D1270): false when failed; true for a size
/// <= 0. A block that does not fit (`pos + size > 0xC800`) is split: `SaveDataToWorkBuffer(data, 0xC800 -
/// pos)`, `SaveWorkBuffer(false)`, the rest copied. Copy, `pos += size`, true.
#[hook(addr = 0x5D1270, conv = cdecl, name = "CGenericGameStorage::SaveDataToWorkBuffer-org", expect = "A0 FC 6E C1 00",
       route = "split", hazards = "heap-writes", recs = "save-load")]
pub fn save_data_to_work_buffer(data: GPtr<u8>, size: i32) -> u8 {
    if failed() {
        return 0;
    }
    if size <= 0 {
        return 1;
    }
    let (mut src, mut n) = (data, size as u32);
    let pos = g::<u32>(WORK_BUFFER_POS).read();
    if pos.wrapping_add(n) > BUFFER_SIZE {
        let k = BUFFER_SIZE.wrapping_sub(pos);
        if call::save_data(src, k as i32) == 0 {
            return 0;
        }
        n = n.wrapping_sub(k);
        src = src.byte_add(k as isize);
        if call::save_work_buffer(0) == 0 {
            return 0;
        }
    }
    let pos = g::<u32>(WORK_BUFFER_POS).read();
    copy(work_buffer().byte_add(pos as isize), src, n);
    g::<u32>(WORK_BUFFER_POS).write(g::<u32>(WORK_BUFFER_POS).read().wrapping_add(n));
    1
}

/// `CGenericGameStorage::LoadDataFromWorkBuffer(void*, int32)` (0x5D1300): false when failed; true for a size
/// <= 0. A block past the buffered data (`pos + size > ms_WorkBufferSize`) is split at `0xC800 - pos` (the
/// literal, not the buffered size): `LoadDataFromWorkBuffer(data, 0xC800 - pos)`, `LoadWorkBuffer()`, the
/// rest copied. Copy, `pos += size`, true.
#[hook(addr = 0x5D1300, conv = cdecl, name = "CGenericGameStorage::LoadDataFromWorkBuffer-org", expect = "A0 FC 6E C1 00",
       route = "split", hazards = "heap-writes", recs = "save-load")]
pub fn load_data_from_work_buffer(data: GPtr<u8>, size: i32) -> u8 {
    if failed() {
        return 0;
    }
    if size <= 0 {
        return 1;
    }
    let (mut dst, mut n) = (data, size as u32);
    let pos = g::<u32>(WORK_BUFFER_POS).read();
    if pos.wrapping_add(n) > g::<u32>(WORK_BUFFER_SIZE).read() {
        let k = BUFFER_SIZE.wrapping_sub(pos);
        if call::load_data(dst, k as i32) == 0 {
            return 0;
        }
        n = n.wrapping_sub(k);
        dst = dst.byte_add(k as isize);
        if call::load_work_buffer() == 0 {
            return 0;
        }
    }
    let pos = g::<u32>(WORK_BUFFER_POS).read();
    copy(dst, work_buffer().byte_add(pos as isize), n);
    g::<u32>(WORK_BUFFER_POS).write(g::<u32>(WORK_BUFFER_POS).read().wrapping_add(n));
    1
}

/// `memcpy` as `rep movsd; rep movsb` does it (forward, dwords then bytes).
fn copy(dst: GPtr<u8>, src: GPtr<u8>, n: u32) {
    let mut i = 0isize;
    while i + 4 <= n as isize {
        dst.byte_add(i).cast::<u32>().write(src.byte_add(i).cast::<u32>().read());
        i += 4;
    }
    while i < n as isize {
        dst.byte_add(i).write(src.byte_add(i).read());
        i += 1;
    }
}

/// `CGenericGameStorage::CheckSlotDataValid(int32 slot)` (0x5D1380): `C_PcSave::GenerateGameFilename` into a
/// local; `CheckDataNotCorrupt(slot, name)`: `CStreaming::DeleteAllRwObjects()`, true; else error 7, false.
#[hook(addr = 0x5D1380, conv = cdecl, name = "CGenericGameStorage::CheckSlotDataValid", expect = "81 EC 04 01 00 00",
       route = "split", hazards = "heap-writes", recs = "save-load")]
pub fn check_slot_data_valid(slot: i32) -> u8 {
    let mut name = [0u8; 0x104];
    let n = GPtr::<u8>::from_raw(name.as_mut_ptr());
    call::generate_game_filename(g(PC_SAVE_HELPER), slot, n);
    if call::check_data_not_corrupt(slot, n) != 0 {
        call::streaming_delete_all_rw_objects();
        return 1;
    }
    set_error(7);
    0
}

/// The block tag write of `GenericSave` (0x5D1410..0x5D148C, an inlined `SaveDataToWorkBuffer` against the
/// literal 0xC800): false when the split write or the flush fails.
fn save_block_tag() -> bool {
    let (mut src, mut n) = (g::<u8>(BLOCK_TAG), 5u32);
    let pos = g::<u32>(WORK_BUFFER_POS).read();
    if pos.wrapping_add(5) > BUFFER_SIZE {
        let k = BUFFER_SIZE.wrapping_sub(pos);
        if call::save_data(g(BLOCK_TAG), k as i32) == 0 {
            return false;
        }
        n = n.wrapping_sub(k);
        src = src.byte_add(k as isize);
        if call::save_work_buffer(0) == 0 {
            return false;
        }
    }
    let pos = g::<u32>(WORK_BUFFER_POS).read();
    copy(work_buffer().byte_add(pos as isize), src, n);
    g::<u32>(WORK_BUFFER_POS).write(g::<u32>(WORK_BUFFER_POS).read().wrapping_add(n));
    true
}

/// The failure exits of `GenericSave` (0x5D1628 / 0x5D1660): buffer freed, `CFileMgr::CloseFile`, false.
fn generic_save_fail() -> u8 {
    free_work_buffer();
    call::close_file(file_handle());
    0
}

/// `CGenericGameStorage::GenericSave()` (0x5D13E0): `ms_bFailed = 0`; `OpenFileForWriting()` or false;
/// `ms_CheckSum = 0`. For each of the 28 blocks: failed -> fail; the tag ([`save_block_tag`]); block 0 is a
/// `CSimpleVariablesSaveStructure` local (0x138 bytes, `Construct`, `ms_bFailed = !SaveDataToWorkBuffer`),
/// the others call their subsystem's `Save` (7, 13, 14 nothing; `ThePaths.Save()`, the focused player's
/// `CPlayerInfo::Save()`); failed -> fail. Then the stream is padded to 0x317FC bytes with whatever the work
/// buffer still holds (whole-buffer flushes while `0x317FC - pos - filePos` would not fit), `SaveWorkBuffer(true)`
/// (failure: buffer freed, file closed, false), buffer freed, `CFileMgr::CloseFile`,
/// `strncpy(ms_SaveFileNameJustSaved, ms_SaveFileName, 0x103)`; a failed close returns false, else
/// `CPad::UpdatePads()` (thunk 0x541E50) and true.
///
/// The structure's bytes `Construct` leaves unwritten are the original's stack residue at entry ESP -0x138..0
/// (the footprint snapshot; see the module doc).
#[hook(addr = 0x5D13E0, conv = cdecl, name = "CGenericGameStorage::GenericSave", expect = "81 EC 38 01 00 00",
       route = "split", hazards = "heap-writes,uninit-locals", recs = "save-load",
       footprint = 0x1000, note = "writes the save file; block 0 carries stack residue (L2, ledger SAVE-1: the save bytes differ only in uninitialised-stack residue, state after load identical)")]
pub fn generic_save() -> u8 {
    g::<u8>(FAILED).write(0);
    if call::open_file_for_writing() == 0 {
        return 0;
    }
    g::<u32>(CHECKSUM).write(0);
    for block in 0..28u32 {
        if failed() || !save_block_tag() {
            return generic_save_fail();
        }
        match block {
            0 => {
                let mut vars = [0u8; 0x138];
                for (k, b) in vars.iter_mut().enumerate() {
                    *b = footprint_residue_u8(k as i32 - 0x138, GENERIC_SAVE_WINDOW);
                }
                let v = GPtr::<u8>::from_raw(vars.as_mut_ptr());
                call::simple_vars_construct(v);
                let ok = call::save_data(v, 0x138);
                g::<u8>(FAILED).write(u8::from(ok == 0));
            }
            1 => {
                call::scripts_save();
            }
            2 => {
                call::pools_save();
            }
            3 => {
                call::garages_save();
            }
            4 => {
                call::game_logic_save();
            }
            5 => {
                call::path_find_save(g(THE_PATHS));
            }
            6 => {
                call::pickups_save();
            }
            8 => {
                call::restart_save();
            }
            9 => {
                call::radar_save();
            }
            10 => {
                call::zones_save();
            }
            11 => {
                call::gangs_save();
            }
            12 => {
                call::car_generators_save();
            }
            15 => {
                call::player_info_save(focused_player_info());
            }
            16 => {
                call::stats_save();
            }
            17 => {
                call::set_pieces_save();
            }
            18 => {
                call::streaming_save();
            }
            19 => {
                call::ped_type_save();
            }
            20 => {
                call::tag_manager_save();
            }
            21 => {
                call::ipl_store_save();
            }
            22 => {
                call::shopping_save();
            }
            23 => {
                call::gang_wars_save();
            }
            24 => {
                call::stunt_jumps_save();
            }
            25 => {
                call::entry_exits_save();
            }
            26 => {
                call::radio_tracks_save();
            }
            27 => {
                call::user_3d_markers_save();
            }
            _ => {}
        }
        if failed() {
            return generic_save_fail();
        }
    }
    // padding (0x5D15C2)
    let total = |pos: u32| pos.wrapping_add(g::<u32>(FILE_POS).read());
    if total(g::<u32>(WORK_BUFFER_POS).read()) < SIZE_OF_ONE_GAME {
        loop {
            let pos = g::<u32>(WORK_BUFFER_POS).read();
            let rem = SIZE_OF_ONE_GAME.wrapping_sub(pos).wrapping_sub(g::<u32>(FILE_POS).read());
            if pos.wrapping_add(rem) < BUFFER_SIZE {
                g::<u32>(WORK_BUFFER_POS).write(pos.wrapping_add(rem));
                break;
            }
            g::<u32>(WORK_BUFFER_POS).write(BUFFER_SIZE);
            if call::save_work_buffer(0) == 0 {
                return generic_save_fail();
            }
            if total(g::<u32>(WORK_BUFFER_POS).read()) >= SIZE_OF_ONE_GAME {
                break;
            }
        }
    }
    if call::save_work_buffer(1) == 0 {
        return generic_save_fail();
    }
    free_work_buffer();
    let closed = call::close_file(file_handle()) == 0;
    call::strncpy(g(SAVE_FILE_NAME_JUST_SAVED), g(SAVE_FILE_NAME), 0x103);
    if !closed {
        return 0;
    }
    call::update_pads_thunk();
    1
}

/// `&CWorld::Players[CWorld::PlayerInFocus]`.
fn focused_player_info() -> GPtr<u8> {
    let i = u32::from(g::<u8>(PLAYER_IN_FOCUS).read());
    g((PLAYERS as u32).wrapping_add(i * 0x190) as usize)
}

/// The block tag read of `GenericLoad` (0x5D1800..0x5D1889): an inlined `LoadDataFromWorkBuffer` of 5 bytes
/// into `tag` (split at `0xC800 - pos` when `pos + 5 > ms_WorkBufferSize`), then `strncmp(tag, "BLOCK", 5)`.
/// `None`: the read failed; `Some(true)`: the tag matches.
fn load_block_tag(tag: &mut [u8; 8]) -> Option<bool> {
    let t = GPtr::<u8>::from_raw(tag.as_mut_ptr());
    let (mut dst, mut n) = (t, 5u32);
    let pos = g::<u32>(WORK_BUFFER_POS).read();
    if pos.wrapping_add(5) > g::<u32>(WORK_BUFFER_SIZE).read() {
        let k = BUFFER_SIZE.wrapping_sub(pos);
        if call::load_data(t, k as i32) == 0 {
            return None;
        }
        n = n.wrapping_sub(k);
        dst = t.byte_add(k as isize);
        if call::load_work_buffer() == 0 {
            return None;
        }
    }
    let pos = g::<u32>(WORK_BUFFER_POS).read();
    copy(dst, work_buffer().byte_add(pos as isize), n);
    g::<u32>(WORK_BUFFER_POS).write(g::<u32>(WORK_BUFFER_POS).read().wrapping_add(n));
    Some(call::strncmp(t, g(BLOCK_TAG), 5) == 0)
}

/// `CGenericGameStorage::GenericLoad(bool& outVariablesLoaded)` (0x5D17B0): `*out = 0` (never set again),
/// `ms_bFailed = 0`, `ms_CheckSum = 0`, `CCheat::ResetCheats()`; `OpenFileForReading(nullptr, 0)` or false;
/// `ms_bLoading = 1`. For each block: the tag ([`load_block_tag`]): unreadable -> buffer freed, file closed,
/// false (`ms_bLoading` stays set); not "BLOCK" -> `ReportError(block - 1, 2)` for a block > 0, the backup
/// `Extract`ed for block 1, buffer freed, file closed, `ms_bLoading = 0`, false. Block 0: a backup `Construct`ed,
/// the structure read (`ms_bFailed = !LoadDataFromWorkBuffer`; failure exits below), `Extract`ed, and
/// `GetCurrentVersionNumber()` called with its result ignored (1.0 checks no version). The others call their
/// subsystem's `Load` (`CPools::Load()` success also `CTheScripts::DoScriptSetupAfterPoolsHaveLoaded()`).
/// Failed -> `ReportError(block, 1)`, buffer freed, file closed, `ms_bLoading = 0`, false. After the loop
/// `ms_bLoading = 0`, buffer freed, `CFileMgr::CloseFile`: non-zero -> false; else
/// `DoGameSpecificStuffAfterSucessLoad()`, true.
#[hook(addr = 0x5D17B0, conv = cdecl, name = "CGenericGameStorage::GenericLoad", expect = "8B 44 24 04 81 EC 7C 02 00 00",
       route = "split", hazards = "heap-writes", recs = "save-load",
       note = "reads the save file; restarts the world from it")]
pub fn generic_load(out_variables_loaded: GPtr<u8>) -> u8 {
    out_variables_loaded.write(0);
    g::<u8>(FAILED).write(0);
    g::<u32>(CHECKSUM).write(0);
    call::cheat_reset_cheats();
    if call::open_file_for_reading(GPtr::NULL, 0) == 0 {
        return 0;
    }
    g::<u8>(LOADING).write(1);
    let mut backup = [0u8; 0x138];
    let mut vars = [0u8; 0x138];
    let mut version = 0u32;
    let bk = GPtr::<u8>::from_raw(backup.as_mut_ptr());
    let vp = GPtr::<u8>::from_raw(vars.as_mut_ptr());
    let ver = GPtr::<u32>::from_raw(&mut version);
    for block in 0..28u32 {
        let mut tag = [0u8; 8];
        let ok = if failed() { None } else { load_block_tag(&mut tag) };
        match ok {
            None => {
                free_work_buffer();
                call::close_file(file_handle());
                return 0;
            }
            Some(false) => {
                if block as i32 > 0 {
                    call::report_error(block as i32 - 1, 2);
                }
                if block == 1 {
                    call::simple_vars_extract(bk, ver);
                }
                free_work_buffer();
                call::close_file(file_handle());
                g::<u8>(LOADING).write(0);
                return 0;
            }
            Some(true) => {}
        }
        match block {
            0 => {
                call::simple_vars_construct(bk);
                let ok = call::load_data(vp, 0x138);
                g::<u8>(FAILED).write(u8::from(ok == 0));
                if ok != 0 {
                    call::simple_vars_extract(vp, ver);
                    call::get_current_version_number();
                }
            }
            1 => {
                call::scripts_load();
            }
            2 => {
                if call::pools_load() != 0 {
                    call::scripts_setup_after_pools_have_loaded();
                }
            }
            3 => {
                call::garages_load();
            }
            4 => {
                call::game_logic_load();
            }
            5 => {
                call::path_find_load(g(THE_PATHS));
            }
            6 => {
                call::pickups_load();
            }
            8 => {
                call::restart_load();
            }
            9 => {
                call::radar_load();
            }
            10 => {
                call::zones_load();
            }
            11 => {
                call::gangs_load();
            }
            12 => {
                call::car_generators_load();
            }
            15 => {
                call::player_info_load(focused_player_info());
            }
            16 => {
                call::stats_load();
            }
            17 => {
                call::set_pieces_load();
            }
            18 => {
                call::streaming_load();
            }
            19 => {
                call::ped_type_load();
            }
            20 => {
                call::tag_manager_load();
            }
            21 => {
                call::ipl_store_load();
            }
            22 => {
                call::shopping_load();
            }
            23 => {
                call::gang_wars_load();
            }
            24 => {
                call::stunt_jumps_load();
            }
            25 => {
                call::entry_exits_load();
            }
            26 => {
                call::radio_tracks_load();
            }
            27 => {
                call::user_3d_markers_load();
            }
            _ => {}
        }
        if failed() {
            call::report_error(block as i32, 1);
            free_work_buffer();
            call::close_file(file_handle());
            g::<u8>(LOADING).write(0);
            return 0;
        }
    }
    g::<u8>(LOADING).write(0);
    free_work_buffer();
    if call::close_file(file_handle()) != 0 {
        return 0;
    }
    call::after_success_load();
    1
}

// ------------------------------------------------------------ CSimpleVariablesSaveStructure ----

fn wr<T: crate::mem::GameData>(p: GPtr<u8>, off: isize, v: T) {
    p.byte_add(off).cast::<T>().write(v);
}

fn rd<T: crate::mem::GameData>(p: GPtr<u8>, off: isize) -> T {
    p.byte_add(off).cast::<T>().read()
}

/// `CSimpleVariablesSaveStructure::Construct()` (0x5D1B80, thiscall): +4 the save name (`TheText.Get` of the
/// last mission's key 0xB78A00, or "MPACKID" in a mission pack, else "ITBEG"), cut at 0x16 characters with
/// "..." (the 5 ASCII bytes at 0x86BB14 through `AsciiToGxtChar`, the 4th made NUL) when longer, +0x1B = 0;
/// +0 the version; then the simple globals (+0x68..+0x136, in the bytes' order; +0x11E `GetLocalTime`
/// through the call site 0x5D1E54). The other bytes are not written.
#[hook(addr = 0x5D1B80, conv = thiscall, name = "CSimpleVariablesSaveStructure::Construct", expect = "A0 00 8A B7 00",
       this = "CSimpleVariablesSaveStructure",
       route = "split", hazards = "none", recs = "save-load")]
pub fn simple_vars_construct(this: GPtr<u8>) {
    let key: usize = if g::<u8>(0xB78A00).read() != 0 {
        0xB78A00
    } else if g::<u8>(0xB72910).read() != 0 {
        0x86BB1C
    } else {
        0x8663B0
    };
    let name = call::text_get(g(THE_TEXT), g(key));
    let mut ascii = [0u8; 8];
    ascii[..4].copy_from_slice(&g::<u32>(0x86BB14).read().to_le_bytes());
    ascii[4] = g::<u8>(0x86BB18).read();
    let mut dots = [0u8; 8];
    let dp = GPtr::<u8>::from_raw(dots.as_mut_ptr());
    call::ascii_to_gxt_char(GPtr::from_raw(ascii.as_mut_ptr()), dp);
    dp.byte_add(3).write(0);
    call::text_copy(this.byte_add(4), name);
    let len = call::gxt_char_strlen(this.byte_add(4));
    this.byte_add(4 + len as isize).write(0);
    if len > 0x16 {
        call::text_copy(this.byte_add(0x16), dp);
    }
    wr::<u8>(this, 0x1B, 0);
    wr::<u32>(this, 0, call::get_current_version_number());
    wr::<u8>(this, 0x68, g::<u8>(0xB72910).read());
    wr::<u32>(this, 0x6C, g::<u32>(0xB7290C).read());
    wr::<u32>(this, 0x70, g::<u32>(0xB6F9CC).read());
    wr::<u32>(this, 0x74, g::<u32>(0xB6F9D0).read());
    wr::<u32>(this, 0x78, g::<u32>(0xB6F9D4).read());
    wr::<u32>(this, 0x7C, g::<u32>(0xB7015C).read());
    wr::<u32>(this, 0x80, g::<u32>(0xB70158).read());
    for (k, a) in [0xB70155usize, 0xB70154, 0xB70153, 0xB70152, 0xB7014E, 0xB7014D, 0xB7014C, 0xB7014B, 0xB7014A, 0xB70144]
        .into_iter()
        .enumerate()
    {
        wr::<u8>(this, 0x84 + k as isize, g::<u8>(a).read());
    }
    let pad = call::get_pad(0);
    wr::<u16>(this, 0x8E, rd::<u16>(pad, 0x10A));
    wr::<u8>(this, 0x90, u8::from(g::<u8>(0x96918C).read() != 0));
    wr::<u32>(this, 0x94, g::<u32>(0xB7CB84).read());
    wr::<u32>(this, 0x98, g::<u32>(0xB7CB64).read());
    wr::<u32>(this, 0x9C, g::<u32>(0xB7CB5C).read());
    wr::<u32>(this, 0xA0, g::<u32>(0xB7CB58).read());
    wr::<u32>(this, 0xA4, g::<u32>(0xB7CB4C).read());
    wr::<u16>(this, 0xA8, g::<u16>(0xC81320).read());
    wr::<u16>(this, 0xAA, g::<u16>(0xC8131C).read());
    wr::<u16>(this, 0xAC, g::<u16>(0xC81318).read());
    wr::<u32>(this, 0xB0, g::<u32>(0xC8130C).read());
    wr::<u32>(this, 0xB4, g::<u32>(0xC81310).read());
    wr::<u32>(this, 0xB8, g::<u32>(0xC81324).read());
    wr::<u32>(this, 0xBC, g::<u32>(0xB6F0DC).read());
    wr::<u32>(this, 0xC0, g::<u32>(0xB6F0F0).read());
    wr::<u8>(this, 0x11C, g::<u8>(0x8CC381).read());
    wr::<u32>(this, 0xC4, g::<u32>(0xB72914).read());
    wr::<u8>(this, 0xC8, u8::from(g::<u8>(0xB73402).read() != 0));
    wr::<u32>(this, 0xCC, g::<u32>(0xB79E44).read());
    wr::<u8>(this, 0xD0, u8::from(g::<u32>(0xB7C484).read() != 0));
    wr::<u32>(this, 0xD4, g::<u32>(0xB79E3C).read());
    wr::<u32>(this, 0xD8, g::<u32>(0xB79E40).read());
    wr::<u32>(this, 0xDC, g::<u32>(0xC228A0).read());
    wr::<u8>(this, 0xE0, g::<u8>(0xB72958).read());
    wr::<u8>(this, 0xE1, g::<u8>(0xB72959).read());
    wr::<u32>(this, 0xE4, g::<u32>(0x8CDEE4).read());
    wr::<u32>(this, 0xE8, g::<u32>(0x8CDEE8).read());
    wr::<u8>(this, 0xED, g::<u8>(0xB9B7ED).read());
    wr::<u8>(this, 0xEC, g::<u8>(0xB9B7EE).read());
    wr::<u8>(this, 0xEE, g::<u8>(0xB9B7EC).read());
    call::get_local_time(this.byte_add(0x11E));
    wr::<u32>(this, 0x130, g::<u32>(0xBA6774).read());
    wr::<u8>(this, 0x134, g::<u8>(0xC0BC15).read());
    wr::<u8>(this, 0x135, u8::from(g::<u8>(0x96918B).read() != 0));
    wr::<u8>(this, 0x136, u8::from(g::<u8>(0x96918A).read() != 0));
}

/// `CSimpleVariablesSaveStructure::Extract(uint32& version)` (0x5D1EA0, thiscall, `ret 4`): the inverse of
/// `Construct` in the bytes' order: the camera position through `CMatrix::SetTranslate(0xB6F99C, x, y, z)`,
/// +0xD0 widened to the dword 0xB7C484, the game clock (+0x94) also forced to 1 together with 0xB70158 when
/// above 0xF6C36300 (unsigned); +0x135 / +0x136 run the cheat functions at 0x8A5CC4 / 0x8A5CC0 (their
/// flag 0x96918B / 0x96918A toggled when the entry is null).
#[hook(addr = 0x5D1EA0, conv = thiscall, name = "CSimpleVariablesSaveStructure::Extract", expect = "56 8B F1 8B 06",
       this = "CSimpleVariablesSaveStructure",
       route = "split", hazards = "none", recs = "save-load")]
pub fn simple_vars_extract(this: GPtr<u8>, version: GPtr<u32>) {
    version.write(rd::<u32>(this, 0));
    g::<u8>(0xB72910).write(rd::<u8>(this, 0x68));
    g::<u32>(0xB7290C).write(rd::<u32>(this, 0x6C));
    call::matrix_set_translate(g(0xB6F99C), rd::<f32>(this, 0x70), rd::<f32>(this, 0x74), rd::<f32>(this, 0x78));
    g::<u32>(0xB7015C).write(rd::<u32>(this, 0x7C));
    g::<u32>(0xB70158).write(rd::<u32>(this, 0x80));
    for (k, a) in [0xB70155usize, 0xB70154, 0xB70153, 0xB70152, 0xB7014E, 0xB7014D, 0xB7014C, 0xB7014B, 0xB7014A]
        .into_iter()
        .enumerate()
    {
        g::<u8>(a).write(rd::<u8>(this, 0x84 + k as isize));
    }
    g::<u8>(0xB70144).write(u8::from(rd::<u8>(this, 0x8D) != 0));
    let pad = call::get_pad(0);
    wr::<u16>(pad, 0x10A, rd::<u16>(this, 0x8E));
    g::<u8>(0x96918C).write(rd::<u8>(this, 0x90));
    g::<u32>(0xB7CB84).write(rd::<u32>(this, 0x94));
    if rd::<u32>(this, 0x94) > 0xF6C3_6300 {
        g::<u32>(0xB70158).write(1);
        g::<u32>(0xB7CB84).write(1);
    }
    g::<u32>(0xB7CB64).write(rd::<u32>(this, 0x98));
    g::<u32>(0xB7CB5C).write(rd::<u32>(this, 0x9C));
    g::<u32>(0xB7CB58).write(rd::<u32>(this, 0xA0));
    g::<u32>(0xB7CB4C).write(rd::<u32>(this, 0xA4));
    g::<u16>(0xC81320).write(rd::<u16>(this, 0xA8));
    g::<u16>(0xC8131C).write(rd::<u16>(this, 0xAA));
    g::<u16>(0xC81318).write(rd::<u16>(this, 0xAC));
    g::<u32>(0xC8130C).write(rd::<u32>(this, 0xB0));
    g::<u32>(0xC81310).write(rd::<u32>(this, 0xB4));
    g::<u32>(0xC81324).write(rd::<u32>(this, 0xB8));
    g::<u32>(0xB6F0DC).write(rd::<u32>(this, 0xBC));
    g::<u32>(0xB6F0F0).write(rd::<u32>(this, 0xC0));
    g::<u8>(0x8CC381).write(rd::<u8>(this, 0x11C));
    g::<u32>(0xB72914).write(rd::<u32>(this, 0xC4));
    g::<u8>(0xB73402).write(rd::<u8>(this, 0xC8));
    g::<u32>(0xB79E44).write(rd::<u32>(this, 0xCC));
    g::<u32>(0xB7C484).write(u32::from(rd::<u8>(this, 0xD0)));
    g::<u32>(0xB79E3C).write(rd::<u32>(this, 0xD4));
    g::<u32>(0xB79E40).write(rd::<u32>(this, 0xD8));
    g::<u32>(0xC228A0).write(rd::<u32>(this, 0xDC));
    g::<u8>(0xB72958).write(rd::<u8>(this, 0xE0));
    g::<u8>(0xB72959).write(rd::<u8>(this, 0xE1));
    g::<u32>(0x8CDEE4).write(rd::<u32>(this, 0xE4));
    g::<u32>(0x8CDEE8).write(rd::<u32>(this, 0xE8));
    g::<u8>(0xB9B7ED).write(rd::<u8>(this, 0xED));
    g::<u8>(0xB9B7EE).write(rd::<u8>(this, 0xEC));
    g::<u8>(0xB9B7EC).write(rd::<u8>(this, 0xEE));
    g::<u32>(0xBA6774).write(rd::<u32>(this, 0x130));
    g::<u8>(0xC0BC15).write(rd::<u8>(this, 0x134));
    if rd::<u8>(this, 0x135) != 0 {
        if g::<u32>(0x8A5CC4).read() != 0 {
            call::cheat_fn_8a5cc4();
        } else {
            let v = g::<u8>(0x96918B).read();
            g::<u8>(0x96918B).write(u8::from(v == 0));
        }
    }
    if rd::<u8>(this, 0x136) != 0 {
        if g::<u32>(0x8A5CC0).read() != 0 {
            call::cheat_fn_8a5cc0();
        } else {
            let v = g::<u8>(0x96918A).read();
            g::<u8>(0x96918A).write(u8::from(v == 0));
        }
    }
}

// -------------------------------------------------------------- around the save / load ----------

/// `GetSavedGameDateAndTime(int32 slot)` (0x618D00): `ms_SlotSaveDate + 0x46 * slot`.
#[hook(addr = 0x618D00, conv = cdecl, name = "CGenericGameStorage::GetSavedGameDateAndTime", expect = "8B 44 24 04 6B C0 46",
       route = "L1-pure", hazards = "none")]
pub fn get_saved_game_date_and_time(slot: i32) -> u32 {
    (SLOT_SAVE_DATE as u32).wrapping_add((slot as u32).wrapping_mul(0x46))
}

/// `CGenericGameStorage::InitRadioStationPositionList()` (0x618E70): empty (`ret`). Not hookable (a lone
/// `ret`).
#[native(addr = 0x618E70, conv = cdecl, name = "CGenericGameStorage::InitRadioStationPositionList")]
pub fn init_radio_station_position_list() {}

/// `CGenericGameStorage::RestoreForStartLoad()` (0x619000): `xor al, al; ret` (3 bytes before the next
/// function's padding). Not hookable; phase B entry.
#[native(addr = 0x619000, conv = cdecl, name = "CGenericGameStorage::RestoreForStartLoad")]
pub fn restore_for_start_load() -> u8 {
    0
}

/// `TheCamera.GetPosition()` as the bytes inline it: the matrix's +0x30 when `m_matrix` is set, else the
/// placement position (0xB6F02C).
fn camera_position() -> GPtr<u8> {
    let m = g::<u32>(0xB6F03C).read();
    if m != 0 { g((m + 0x30) as usize) } else { g(0xB6F02C) }
}

/// `CGenericGameStorage::DoGameSpecificStuffAfterSucessLoad()` (0x618E90): `TheText.Load(false)`,
/// `CCollision::SortOutCollisionAfterLoad()`, `LoadSceneCollision` / `LoadScene` of the camera position
/// (re-read for each), `LoadAllRequestedModels(false)`, `CGame::TidyUpMemory(true, false)`,
/// `JustLoadedDontFadeInYet = StillToFadeOut = 1`, `TheCamera.Fade(0, 0)`, `CTheScripts::Process()`,
/// `CTagManager::UpdateNumTagged()`, `CClothes::RebuildPlayer(Players[0].m_pPed, false)`,
/// `CPopCycle::Update()`, `CStreaming::RemoveInappropriatePedModels()`,
/// `CGangWars::ClearSpecificZonesToTriggerGangWar()`, then `bTrainingMission = 0` and
/// `CTheZones::FillZonesWithGangColours(bGangWarsActive)` (read before the store).
#[hook(addr = 0x618E90, conv = cdecl, name = "CGenericGameStorage::DoGameSpecificStuffAfterSucessLoad", expect = "6A 00 B9 40 B3 C1 00",
       route = "L3-only", hazards = "heap-writes", recs = "save-load",
       note = "calls the scripts, streaming and the scene load: one call per load, verified by the trace")]
pub fn do_game_specific_stuff_after_success_load() {
    call::text_load(g(THE_TEXT), 0);
    call::collision_sort_out_collision_after_load();
    call::streaming_load_scene_collision(camera_position());
    call::streaming_load_scene(camera_position());
    call::streaming_load_all_requested_models(0);
    call::game_tidy_up_memory(1, 0);
    g::<u8>(JUST_LOADED_DONT_FADE_IN_YET).write(1);
    g::<u8>(STILL_TO_FADE_OUT).write(1);
    call::camera_fade(g(THE_CAMERA), 0.0, 0);
    call::scripts_process();
    call::tag_manager_update_num_tagged();
    call::clothes_rebuild_player(g::<GPtr<u8>>(PLAYERS).read(), 0);
    call::pop_cycle_update();
    call::streaming_remove_inappropriate_ped_models();
    call::gang_wars_clear_specific_zones_to_trigger_gang_war();
    let active = u32::from(g::<u8>(0x96AB90).read());
    g::<u8>(0x96AB91).write(0);
    call::zones_fill_zones_with_gang_colours(active);
}

/// `CGenericGameStorage::DoGameSpecificStuffBeforeSave()` (0x618F50): `CRopes::Shutdown()`,
/// `CPickups::RemovePickupObjects()`, `Players[0].m_pPed->m_fHealth = min(Players[0] max health byte (+0x14F),
/// 200)` (`fild`), `CGangWars::EndGangWar(false)`, `CStats::IncrementStat(0x88, 1.0)`,
/// `CGameLogic::PassTime(360)`, the focused player's +0x144 word = 0, `ResetSprintEnergy` and
/// `SetWantedLevel(0)` on `FindPlayerPed(-1)` (fetched for each), `CGame::TidyUpMemory(true, false)`.
#[hook(addr = 0x618F50, conv = cdecl, name = "CGenericGameStorage::DoGameSpecificStuffBeforeSave", expect = "51 E8 BA DB F3 FF",
       route = "L3-only", hazards = "heap-writes", recs = "save-load",
       note = "one call per save, verified by the trace and the save bytes")]
pub fn do_game_specific_stuff_before_save() {
    call::ropes_shutdown();
    call::pickups_remove_pickup_objects();
    let h = g::<u8>(PLAYERS + 0x14F).read().min(200);
    let ped = g::<GPtr<u8>>(PLAYERS).read();
    wr::<f32>(ped, 0x540, h as f32);
    call::gang_wars_end_gang_war(0);
    call::stats_increment_stat(0x88, 1.0);
    call::game_logic_pass_time(360);
    let i = u32::from(g::<u8>(PLAYER_IN_FOCUS).read());
    g::<u16>((0xB7CEDCu32 + i * 0x190) as usize).write(0);
    call::player_ped_reset_sprint_energy(call::find_player_ped(-1));
    call::player_ped_set_wanted_level(call::find_player_ped(-1), 0);
    call::game_tidy_up_memory(1, 0);
}

// ------------------------------------------------------------------------------ C_PcSave ---------

/// `C_PcSave::SaveSlot(int32 slot)` (0x619060, thiscall, `ret 4`): `MakeValidSaveName((int8)slot)`, error =
/// 0, `InitUserDirectories()`, `DoGameSpecificStuffBeforeSave()`, `GenericSave()`; EAX = 0 on success, else 2.
#[hook(addr = 0x619060, conv = thiscall, name = "C_PcSave::SaveSlot", expect = "0F BE 44 24 04 50",
       route = "L3-only", hazards = "heap-writes", recs = "save-load",
       note = "one call per save; the save bytes and the trace check it (L2, ledger SAVE-1: the save bytes differ only in uninitialised-stack residue, state after load identical)")]
pub fn save_slot(_this: GPtr<u8>, slot: i32) -> u32 {
    call::make_valid_save_name(i32::from(slot as i8));
    set_error(0);
    call::init_user_directories();
    call::before_save();
    if call::generic_save() != 0 { 0 } else { 2 }
}

/// `C_PcSave::GenerateGameFilename(int32 slot, char* out)` (0x6190A0, thiscall, `ret 8`): `sprintf(out,
/// "%s%i%s", DefaultPCSaveFileName, slot + 1, ".b")`.
#[hook(addr = 0x6190A0, conv = thiscall, name = "C_PcSave::GenerateGameFilename", expect = "8B 44 24 04 8B 4C 24 08",
       route = "split", hazards = "none", recs = "save-load")]
pub fn generate_game_filename(_this: GPtr<u8>, slot: i32, out: GPtr<u8>) {
    call::sprintf_sis(out, g(FMT_S_I_S), g(DEFAULT_PC_SAVE_FILE_NAME), slot.wrapping_add(1), g(STR_DOT_B));
}

/// `C_PcSave::DeleteSlot(int32 slot)` (0x6190D0, thiscall, `ret 4`): error = 0, `sprintf(path, "%s%i.b",
/// DefaultPCSaveFileName, (int8)slot + 1)`, `DeleteFileA(path)`; then the file is probed with `OpenFile(path,
/// "rb")`: still there -> closed, false; gone -> true.
#[hook(addr = 0x6190D0, conv = thiscall, name = "C_PcSave::DeleteSlot", expect = "0F BE 44 24 04 81 EC 04 01 00 00",
       unverified = "save-core-1", route = "split", hazards = "none",
       note = "deletes a save file; no corpus recording deletes one")]
pub fn delete_slot(_this: GPtr<u8>, slot: i32) -> u8 {
    let mut path = [0u8; 0x104];
    let p = GPtr::<u8>::from_raw(path.as_mut_ptr());
    set_error(0);
    call::sprintf_si(p, g(FMT_S_I_DOT_B), g(DEFAULT_PC_SAVE_FILE_NAME), i32::from(slot as i8).wrapping_add(1));
    call::delete_file_a(p);
    let f = call::open_file(p, g(MODE_RB));
    if !f.is_null() {
        call::close_file(f);
        return 0;
    }
    1
}

// ------------------------------------------------------------------------------- helpers ---------

/// `TextCopy(GxtChar* dst, const GxtChar* src)` (0x69F910): copies up to and including the NUL (an empty
/// source writes the NUL only).
#[hook(addr = 0x69F910, conv = cdecl, name = "TextCopy", expect = "8B 4C 24 08 8A 01",
       route = "split", hazards = "none", recs = "save-load")]
pub fn text_copy(dst: GPtr<u8>, src: GPtr<u8>) {
    let mut i = 0isize;
    loop {
        let c = src.byte_add(i).read();
        dst.byte_add(i).write(c);
        if c == 0 {
            break;
        }
        i += 1;
    }
}

/// 0x541E50: `jmp CPad::UpdatePads` (0x541DD0), the thunk `GenericSave` calls after a successful save. Not
/// hookable in phase A (a `jmp` outside `.HOODLUM` is no inline entry sa-hook can relocate as a stub): phase B
/// entry; in phase A the thunk runs `CPad::UpdatePads`' own hook.
#[native(addr = 0x541E50, conv = cdecl, name = "thunk_FUN_00541dd0")]
pub fn update_pads_thunk() {
    call::update_pads();
}

/// `CTheScripts::DoScriptSetupAfterPoolsHaveLoaded()` (0x5D3390): every LOD link pair (10 x 8 at 0xA44800)
/// with neither side -1 is reconnected (`CTheScripts::ScriptConnectLodsFunction(a, b)`). Not hookable in
/// phase A (its loop branches back to 0x5D3393, inside the patched entry): phase B entry.
#[native(addr = 0x5D3390, conv = cdecl, name = "CTheScripts::DoScriptSetupAfterPoolsHaveLoaded")]
pub fn do_script_setup_after_pools_have_loaded() {
    for k in 0..10usize {
        let a = g::<i32>(0xA44800 + 8 * k).read();
        if a == -1 {
            continue;
        }
        let b = g::<i32>(0xA44804 + 8 * k).read();
        if b == -1 {
            continue;
        }
        call::script_connect_lods_function(a, b);
    }
}

pub const HOOKS: &[crate::abi::HookReg] = &[
    HOOK_REPORT_ERROR,
    HOOK_OPEN_FILE_FOR_READING,
    HOOK_OPEN_FILE_FOR_WRITING,
    HOOK_CLOSE_FILE,
    HOOK_MAKE_VALID_SAVE_NAME,
    HOOK_GET_NAME_OF_SAVED_GAME,
    HOOK_GET_CURRENT_VERSION_NUMBER,
    HOOK_SAVE_WORK_BUFFER,
    HOOK_LOAD_WORK_BUFFER,
    HOOK_CHECK_DATA_NOT_CORRUPT,
    HOOK_SAVE_DATA_TO_WORK_BUFFER,
    HOOK_LOAD_DATA_FROM_WORK_BUFFER,
    HOOK_CHECK_SLOT_DATA_VALID,
    HOOK_GENERIC_SAVE,
    HOOK_GENERIC_LOAD,
    HOOK_SIMPLE_VARS_CONSTRUCT,
    HOOK_SIMPLE_VARS_EXTRACT,
    HOOK_GET_SAVED_GAME_DATE_AND_TIME,
    HOOK_DO_GAME_SPECIFIC_STUFF_AFTER_SUCCESS_LOAD,
    HOOK_DO_GAME_SPECIFIC_STUFF_BEFORE_SAVE,
    HOOK_SAVE_SLOT,
    HOOK_GENERATE_GAME_FILENAME,
    HOOK_DELETE_SLOT,
    HOOK_TEXT_COPY,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_tables() {
        assert_eq!(get_name_of_saved_game(0), 0xC16368);
        assert_eq!(get_name_of_saved_game(7), 0xC16368 + 7 * 0x104);
        assert_eq!(get_saved_game_date_and_time(3), 0xC16138 + 3 * 0x46);
        assert_eq!(restore_for_start_load(), 0);
    }
}
