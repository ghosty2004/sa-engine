//! CORE ports for the native boot: `Ps::psInitialize` (0x747420) and its small callee
//! `C_PcSave::SetSaveDirectory` (0x619040) of gta_sa.exe 1.0 US.
//!
//! Both follow the 1.0 US bytes (objdump over the exe; gta-reversed `WinPs.cpp` / `C_PcSave.cpp` as
//! reading aid only: its psInitialize differs a lot, see the port's doc). Imports are called as in
//! [`crate::platform`]: `site` for each `call dword ptr [IAT]` instruction. Strings the original
//! pushes by address are passed as the same addresses ([`rdata`]).

use crate::mem::GPtr;
use crate::platform::{PsGlobalType, RS_PS, rdata};
use crate::{globals, hook, orig};

globals! {
    list GLOBALS;
    /// `PsGlobal` (psGlobalType, the struct `RsGlobal.ps` points to)
    pub static PS_GLOBAL @ 0xC8CF88: PsGlobalType = PsGlobalType {
        window: 0,
        instance: 0,
        full_screen: 0,
        last_mouse_pos: [0.0; 2],
        last_render_time: 0,
        di_interface: 0,
        di_mouse: 0,
        di_device1: 0,
        di_device2: 0,
    };
    /// `s_OSStatus.DxVer` (result of `FUN_00745650`: 0, 0x700, 0x800, 0x801 or 0x900)
    pub static DX_VER @ 0xC8CF6C: u32 = 0;
    /// `s_OSStatus.RAM.TotalPhys` (`MEMORYSTATUS.dwTotalPhys`)
    pub static RAM_TOTAL_PHYS @ 0xC8CF70: u32 = 0;
    /// `s_OSStatus.RAM.AvailPhys`
    pub static RAM_AVAIL_PHYS @ 0xC8CF74: u32 = 0;
    /// `s_OSStatus.RAM.TotalVirtual`
    pub static RAM_TOTAL_VIRTUAL @ 0xC8CF78: u32 = 0;
    /// `s_OSStatus.RAM.AvailVirtual`
    pub static RAM_AVAIL_VIRTUAL @ 0xC8CF7C: u32 = 0;
    /// Second out-argument of `GetVideoMemInfo` (`available`); the first (`total`) is 0xC8CF80,
    /// declared as [`crate::video::VRAM_AVAIL`]
    pub static VRAM_SECOND @ 0xC8CF84: u32 = 0;
    /// `gCpuVendor` (`char[13]`, image value "UnknownVendr"; psInitialize writes the 12 CPUID bytes)
    pub static CPU_VENDOR @ 0x8D6238: [u8; 12] = *b"UnknownVendr";
    /// `C_PcSave::DefaultPCSaveFileName` (`char[MAX_PATH]`)
    pub static DEFAULT_PC_SAVE_FILE_NAME @ 0xC16F18: [u8; 260] = [0; 260];
}

/// `psGlobalType` field offsets written by psInitialize.
const PS_LAST_MOUSE_POS_X: isize = 0x0C;
const PS_LAST_MOUSE_POS_Y: isize = 0x10;
const PS_FULL_SCREEN: isize = 0x08;
const PS_DI_INTERFACE: isize = 0x18;
const PS_DI_MOUSE: isize = 0x1C;
const PS_DI_DEVICE1: isize = 0x20;
const PS_DI_DEVICE2: isize = 0x24;

/// `TheText` (CText) and `FrontEndMenuManager` (CMenuManager), thiscall `this`.
const THE_TEXT: usize = crate::game::THE_TEXT as usize;
const FRONT_END_MENU_MANAGER: usize = crate::game::FRONTEND_MENU_MANAGER as usize;

/// GXT keys in `.rdata`.
const KEY_WIN_TTL: u32 = 0x874B30;
const KEY_WIN_95: u32 = 0x874B28;
const KEY_WIN_DX: u32 = 0x874B20;
const KEY_WIN_NSC: u32 = 0x874B18;
/// `"%s\\%s"` and `"GTASAsf"` (SetSaveDirectory)
const FMT_SAVE_DIR: u32 = 0x86D2E4;
const STR_GTASASF: u32 = 0x86D2EC;

/// `OSVERSIONINFOA` (0x94 bytes) as dwords: +0 size, +4 major, +8 minor, +0x10 platform id.
type OsVersionInfoA = [u32; 0x94 / 4];
/// `MEMORYSTATUS` (0x20 bytes) as dwords.
type MemoryStatus = [u32; 8];

pub mod call {
    use super::*;
    orig! {
        /// `CFileMgr::Initialise` (0x5386F0)
        pub fn file_mgr_initialise() = cdecl 0x5386F0 => crate::filemgr::initialise;
        /// `InitUserDirectories` (0x744FB0)
        pub fn init_user_directories() -> GPtr<u8> = cdecl 0x744FB0 => crate::platform::init_user_directories;
        /// `C_PcSave::SetSaveDirectory(const char*)` (0x619040)
        pub fn set_save_directory(path: GPtr<u8>) = cdecl 0x619040 => super::set_save_directory;
        /// `Ps::InitialiseLanguage` (0x7465B0)
        pub fn initialise_language() = cdecl 0x7465B0 => crate::platform::initialise_language;
        /// `CMenuManager::LoadSettings` (0x57C8F0, not ported)
        pub fn menu_load_settings(this: GPtr<u8>) = thiscall 0x57C8F0 => unported;
        /// `FUN_00745650` (DirectX version check)
        pub fn check_dx_version() -> u32 = cdecl 0x745650 => crate::platform::check_dx_version;
        /// `CText::Get(const char*)` (0x6A0050; ported in sa-hud, not reachable from sa-core)
        pub fn text_get(text: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
        /// `Ps::GetVideoMemInfo(LPDWORD total, LPDWORD available)` (0x7455E0)
        pub fn get_video_mem_info(total: GPtr<u32>, available: GPtr<u32>) -> i32 = cdecl 0x7455E0 => crate::platform::get_video_mem_info;
        /// `Ps::CheckDirectSound` (0x745840)
        pub fn check_direct_sound() -> i32 = cdecl 0x745840 => crate::platform::check_direct_sound;
        /// `CText::Unload(bool)` (0x69FF20; ported in sa-hud, not reachable from sa-core)
        pub fn text_unload(text: GPtr<u8>, keep_mission_packs: u32) = thiscall 0x69FF20 => unported;
        /// CRT `sprintf(buf, fmt, s1, s2)` (0x821BB5)
        pub fn sprintf_ss(buf: GPtr<u8>, fmt: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x821BB5 => unported;
    }
    orig! {
        /// CRT `sprintf(buf, fmt, char*, int, char*)`
        pub fn sprintf_sis(buf: GPtr<u8>, fmt: GPtr<u8>, a: GPtr<u8>, n: i32, b: GPtr<u8>) -> i32 = cdecl 0x821BB5 => unported;
        /// CRT `sprintf(buf, fmt, day, month text, year, hour, minute, second)`
        pub fn sprintf_date(buf: GPtr<u8>, fmt: GPtr<u8>, d: u32, m: u32, y: u32, h: u32, mi: u32, s: u32) -> i32 = cdecl 0x821BB5 => unported;
        pub fn file_mgr_open_file(path: GPtr<u8>, mode: GPtr<u8>) -> u32 = cdecl 0x538900 => unported;
        pub fn file_mgr_seek(file: u32, offset: i32, origin: i32) -> i32 = cdecl 0x538990 => unported;
        pub fn file_mgr_read(file: u32, buf: GPtr<u8>, size: i32) -> i32 = cdecl 0x538950 => unported;
        pub fn file_mgr_close_file(file: u32) -> i32 = cdecl 0x5389D0 => unported;
        /// CRT `strncmp`
        pub fn strncmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x8214D0 => unported;
        /// `CGenericGameStorage::CheckDataNotCorrupt(slot, path)` (bool in AL)
        pub fn check_data_not_corrupt(slot: i32, path: GPtr<u8>) -> u8 = cdecl 0x5D1170 => unported;
        /// `CMessages::InsertNumberInString(src, n1..n6, dst)`
        pub fn insert_number_in_string(src: GPtr<u8>, n1: i32, n2: i32, n3: i32, n4: i32, n5: i32, n6: i32, dst: GPtr<u8>) = cdecl 0x69DE90 => unported;
        /// `Common::AsciiToGxtChar(src, dst)`
        pub fn ascii_to_gxt_char(src: GPtr<u8>, dst: GPtr<u8>) = cdecl 0x718600 => unported;
        // Win32 (import name, call site)
        /// KERNEL32 `GetVersionExA` at 0x7474D3
        pub fn get_version_ex_a(info: GPtr<OsVersionInfoA>) -> i32 = stdcall site 0x7474D3 => unported;
        /// USER32 `MessageBoxA` at 0x74757E (WIN_95)
        pub fn message_box_a_win95(hwnd: u32, text: GPtr<u8>, caption: GPtr<u8>, ty: u32) -> i32 = stdcall site 0x74757E => unported;
        /// USER32 `MessageBoxA` at 0x7475B8 (WIN_DX)
        pub fn message_box_a_dx(hwnd: u32, text: GPtr<u8>, caption: GPtr<u8>, ty: u32) -> i32 = stdcall site 0x7475B8 => unported;
        /// KERNEL32 `GlobalMemoryStatus` at 0x7475CD
        pub fn global_memory_status(status: GPtr<MemoryStatus>) = stdcall site 0x7475CD => unported;
        /// USER32 `MessageBoxA` at 0x747634 (WIN_NSC)
        pub fn message_box_a_nsc(hwnd: u32, text: GPtr<u8>, caption: GPtr<u8>, ty: u32) -> i32 = stdcall site 0x747634 => unported;
    }
}

/// `cpuid` with `eax = leaf`: `[eax, ebx, ecx, edx]`.
#[cfg(target_arch = "x86")]
fn cpuid(leaf: u32) -> [u32; 4] {
    let r = core::arch::x86::__cpuid(leaf);
    [r.eax, r.ebx, r.ecx, r.edx]
}

/// Non-x86 hosts (phases B/C, x86_64 tests): a deterministic CPU without extended leaves whose
/// leaf-0 vendor is the image's "UnknownVendr", so `gCpuVendor` keeps its initial bytes.
#[cfg(not(target_arch = "x86"))]
fn cpuid(leaf: u32) -> [u32; 4] {
    let w = |s: &[u8; 4]| u32::from_le_bytes(*s);
    match leaf {
        0 => [1, w(b"Unkn"), w(b"endr"), w(b"ownV")],
        _ => [0; 4],
    }
}

/// `s_OSStatus.OSVer` from `GetVersionExA`'s platform id / major / minor, as the bytes branch:
/// NT (2): major 4 → 2, 5 → 3, above 5 → 4, below 4 → no store (stays 0). 9x (1): major > 4, or
/// 4 with minor > 0 → 1, else an explicit 0. Other platform ids: no store.
fn os_ver_for(platform_id: u32, major: u32, minor: u32) -> Option<i32> {
    match platform_id {
        2 => match major {
            4 => Some(2),
            5 => Some(3),
            m if m > 5 => Some(4),
            _ => None,
        },
        1 => Some((major > 4 || (major == 4 && minor > 0)) as i32),
        _ => None,
    }
}

/// `MessageBoxA(NULL, TheText.Get(text_key), TheText.Get("WIN_TTL"), ty)` through `mb`. The caption
/// is fetched first, as the bytes do. The original passes CText's wide strings to the ANSI
/// `MessageBoxA` (kept: the box shows only the first character of each string).
fn text_message_box(mb: fn(u32, GPtr<u8>, GPtr<u8>, u32) -> i32, text_key: u32, ty: u32) {
    let the_text = GPtr::<u8>::from_addr(THE_TEXT);
    let caption = call::text_get(the_text, rdata(KEY_WIN_TTL));
    let text = call::text_get(the_text, rdata(text_key));
    mb(0, text, caption, ty);
}

/// `Ps::psInitialize` (0x747420, cdecl, RwBool). Per the bytes: zeroes `PsGlobal.lastMousePos.y`,
/// `.x`, `fullScreen`, `diInterface`, `diMouse`, `diDevice1`, `diDevice2` (in that order) and sets
/// `RsGlobal.ps = &PsGlobal`; `CFileMgr::Initialise()`; `SetSaveDirectory(InitUserDirectories())`;
/// `InitialiseLanguage()`; `gGameState = 0`. CPUID leaf 1, then 0x80000000 and (if its eax is above
/// 0x80000000) 0x80000001: the feature edx is kept only in a dead local. CPUID leaf 0 writes the
/// vendor (ebx, edx, ecx) to `gCpuVendor`. `GetVersionExA` (size 0x94) → `s_OSStatus.OSVer`
/// ([`os_ver_for`]). Then `FrontEndMenuManager.LoadSettings()` and `s_OSStatus.DxVer =
/// FUN_00745650()`; both checks come after them (gta-reversed loads the settings last). OSVer 0
/// (Windows 95 / unknown) → MessageBoxA WIN_95, FALSE; DxVer < 0x900 → WIN_DX, FALSE.
/// `GlobalMemoryStatus` (dwLength never set, as in the original) → the four RAM fields;
/// `GetVideoMemInfo(0xC8CF80, 0xC8CF84)` (result ignored); `CheckDirectSound()` failing →
/// MessageBoxA WIN_NSC with MB_ICONEXCLAMATION (0x30), FALSE. Finally `TheText.Unload(false)`, TRUE.
/// No `CoInitialize` (gta-reversed adds one).
///
/// Written by STD-2's helper for the native boot, not yet verified against the original (phase A).
#[hook(addr = 0x747420, conv = cdecl, name = "Ps::psInitialize", expect = "55 8D 6C 24 88")]
pub fn ps_initialize() -> i32 {
    let ps = PS_GLOBAL.ptr();
    let ps_u32 = |off: isize| ps.byte_add(off).cast::<u32>();
    ps_u32(PS_LAST_MOUSE_POS_Y).write(0);
    ps_u32(PS_LAST_MOUSE_POS_X).write(0);
    ps_u32(PS_FULL_SCREEN).write(0);
    ps_u32(PS_DI_INTERFACE).write(0);
    ps_u32(PS_DI_MOUSE).write(0);
    ps_u32(PS_DI_DEVICE1).write(0);
    ps_u32(PS_DI_DEVICE2).write(0);
    RS_PS.set(ps);

    call::file_mgr_initialise();
    call::set_save_directory(call::init_user_directories());
    call::initialise_language();
    // 0x747483 `mov [gGameState],esi`: samp NOPs it and writes 5 (FRONTEND_LOADING) itself at attach.
    if crate::samp::toggled(crate::samp::toggle::SAMP_STARTUP) {
        crate::video::GAME_STATE.set(5);
    } else {
        crate::video::GAME_STATE.set(0);
    }

    // Feature flags (edx of leaf 1, then of 0x80000001 or 0): stored into a local nothing reads.
    let _ = cpuid(1);
    if cpuid(0x8000_0000)[0] > 0x8000_0000 {
        let _ = cpuid(0x8000_0001);
    }
    let [_, ebx, ecx, edx] = cpuid(0);
    let mut vendor = [0u8; 12];
    vendor[0..4].copy_from_slice(&ebx.to_le_bytes());
    vendor[4..8].copy_from_slice(&edx.to_le_bytes());
    vendor[8..12].copy_from_slice(&ecx.to_le_bytes());
    CPU_VENDOR.set(vendor);

    let mut info: OsVersionInfoA = [0; 0x94 / 4];
    info[0] = 0x94;
    call::get_version_ex_a(GPtr::from_raw(&raw mut info));
    crate::input::OS_VER.set(0);
    if let Some(v) = os_ver_for(info[4], info[1], info[2]) {
        crate::input::OS_VER.set(v);
    }

    call::menu_load_settings(GPtr::from_addr(FRONT_END_MENU_MANAGER));
    let dx = call::check_dx_version();
    DX_VER.set(dx);
    if crate::input::OS_VER.get() == 0 {
        text_message_box(call::message_box_a_win95, KEY_WIN_95, 0);
        return 0;
    }
    if dx < 0x900 {
        text_message_box(call::message_box_a_dx, KEY_WIN_DX, 0);
        return 0;
    }

    let mut status: MemoryStatus = [0; 8];
    call::global_memory_status(GPtr::from_raw(&raw mut status));
    RAM_TOTAL_PHYS.set(status[2]);
    RAM_AVAIL_PHYS.set(status[3]);
    RAM_TOTAL_VIRTUAL.set(status[6]);
    RAM_AVAIL_VIRTUAL.set(status[7]);
    call::get_video_mem_info(crate::video::VRAM_AVAIL.ptr(), VRAM_SECOND.ptr());

    if call::check_direct_sound() == 0 {
        text_message_box(call::message_box_a_nsc, KEY_WIN_NSC, 0x30);
        return 0;
    }
    call::text_unload(GPtr::from_addr(THE_TEXT), 0);
    1
}

/// `C_PcSave::SetSaveDirectory(const char* path)` (0x619040, cdecl):
/// `sprintf(DefaultPCSaveFileName, "%s\\%s", path, "GTASAsf")`.
///
/// Written by STD-2's helper for the native boot, not yet verified against the original (phase A).
#[hook(addr = 0x619040, conv = cdecl, name = "C_PcSave::SetSaveDirectory", expect = "8B 44 24 04 68 EC D2 86 00")]
pub fn set_save_directory(path: GPtr<u8>) {
    call::sprintf_ss(
        DEFAULT_PC_SAVE_FILE_NAME.ptr().cast(),
        rdata(FMT_SAVE_DIR),
        path,
        rdata(STR_GTASASF),
    );
}

/// `C_PcSave::PopulateSlotInfo()` (0x619140, cdecl). All 8 slot states (0xC16EBC) := 1 (empty),
/// `[0xC17034]` := 0, the slot names (8 x 0x104 at 0xC16368) and dates (8 x 0x46 at 0xC16138) get a
/// leading NUL. Then per slot i: `sprintf(path, [0x86BAD4], 0xC16F18, i + 1, [0x86BADC])`; an opened
/// file is read at offset 5 for 0x138 bytes and, when its name (offset 4) differs from [0x8D2BE4] in
/// the first 26 bytes, 48 bytes of it become the slot name (cut at 0x18) and the slot is marked used.
/// For a used slot `CGenericGameStorage::CheckDataNotCorrupt(i, path)`; failing →
/// `CMessages::InsertNumberInString(TheText.Get([0x86D378]), i + 1, -1 x5, name)`; else the header's
/// SYSTEMTIME (offset 0x11E of the block read) becomes the date text: the month name (`TheText.Get`
/// of one of 12 keys 0x86D370 - 8 * (month - 1), converted by 0x69F8C0; an out-of-range month keeps
/// the previous pointer, initially NULL), `sprintf(buf, [0x86D2FC], day, month, year, hour, minute,
/// second)`, `AsciiToGxtChar(buf, date)`. The 0x138-byte block is a stack local in the original
/// (uninitialised; only read for slots whose file was read).
#[hook(addr = 0x619140, conv = cdecl, name = "C_PcSave::PopulateSlotInfo", expect = "81 EC A4 02 00 00",
       unverified = "std4-core-1", route = "L2", hazards = "uninit-locals", recs = "boot")]
pub fn populate_slot_info() {
    let g32 = |a: usize| GPtr::<u32>::from_addr(a);
    let mut month_text: u32 = 0;
    g32(0xC17034).write(0);
    for i in 0..8 {
        g32(0xC16EBC + 4 * i).write(1);
        GPtr::<u8>::from_addr(0xC16368 + 0x104 * i).write(0);
        GPtr::<u8>::from_addr(0xC16138 + 0x46 * i).write(0);
    }
    let mut path = [0u8; 0x104];
    let mut block = [0u8; 0x138];
    let mut buf = [0u8; 0x80];
    for i in 0..8usize {
        let name: GPtr<u8> = GPtr::from_addr(0xC16368 + 0x104 * i);
        let date: GPtr<u8> = GPtr::from_addr(0xC16138 + 0x46 * i);
        let pp: GPtr<u8> = GPtr::from_raw(path.as_mut_ptr());
        call::sprintf_sis(pp, rdata(0x86BAD4), GPtr::from_addr(0xC16F18), i as i32 + 1, rdata(0x86BADC));
        let f = call::file_mgr_open_file(pp, rdata(0x859C6C));
        let bp: GPtr<u8> = GPtr::from_raw(block.as_mut_ptr());
        if f != 0 {
            call::file_mgr_seek(f, 5, 0);
            call::file_mgr_read(f, bp, 0x138);
            if call::strncmp(bp.byte_add(4), rdata(0x8D2BE4), 0x1A) != 0 {
                for k in 0..48 {
                    name.byte_add(k).write(bp.byte_add(4 + k).read());
                }
                g32(0xC16EBC + 4 * i).write(0);
                name.byte_add(0x18).write(0);
            }
            call::file_mgr_close_file(f);
        }
        if g32(0xC16EBC + 4 * i).read() != 0 {
            continue;
        }
        if call::check_data_not_corrupt(i as i32, pp) == 0 {
            let t = call::text_get(GPtr::from_addr(crate::game::THE_TEXT as usize), rdata(0x86D378));
            call::insert_number_in_string(t, i as i32 + 1, -1, -1, -1, -1, -1, name);
            continue;
        }
        let w = |o: usize| u16::from_le_bytes([block[0x11E + o], block[0x11F + o]]) as u32;
        let (year, month, day, hour, minute, second) = (w(0), w(2), w(6), w(8), w(0xA), w(0xC));
        if (1..=12).contains(&month) {
            let key = 0x86D370 - 8 * (month - 1);
            let t = call::text_get(GPtr::from_addr(crate::game::THE_TEXT as usize), rdata(key));
            month_text = gxt_char_to_ascii(t).addr() as u32;
        }
        let b: GPtr<u8> = GPtr::from_raw(buf.as_mut_ptr());
        call::sprintf_date(b, rdata(0x86D2FC), day, month_text, year, hour, minute, second);
        call::ascii_to_gxt_char(b, date);
    }
}

/// `FUN_0069f8c0` (`GxtCharToAscii(const GxtChar* text)`, 0x69F8C0, cdecl; the one-argument 1.0 US form): copies at
/// most 256 bytes into the static buffer 0xC1BED0, each byte >= 0x7F and `/`, `*`, `?` replaced by a
/// space, stops at NUL (NULL text → empty string); returns the buffer.
#[hook(addr = 0x69F8C0, conv = cdecl, name = "FUN_0069f8c0", expect = "8B 4C 24 04 53 33 DB",
       unverified = "std4-core-1", route = "L2", hazards = "none", recs = "boot")]
pub fn gxt_char_to_ascii(text: GPtr<u8>) -> GPtr<u8> {
    let out: GPtr<u8> = GPtr::from_addr(0xC1BED0);
    let mut n = 0usize;
    if !text.is_null() {
        while n <= 0xFF {
            let c = text.byte_add(n as isize).read();
            if c == 0 {
                break;
            }
            let v = if c >= 0x7F || c == b'/' || c == b'*' || c == b'?' { b' ' } else { c };
            out.byte_add(n as isize).write(v);
            n += 1;
        }
    }
    out.byte_add(n as isize).write(0);
    out
}

/// This module's registrations (collected by [`crate::hooks`]).
pub const HOOKS: &[crate::abi::HookReg] =
    &[HOOK_PS_INITIALIZE, HOOK_SET_SAVE_DIRECTORY, HOOK_POPULATE_SLOT_INFO, HOOK_GXT_CHAR_TO_ASCII];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_version_classes() {
        assert_eq!(os_ver_for(2, 3, 51), None);
        assert_eq!(os_ver_for(2, 4, 0), Some(2));
        assert_eq!(os_ver_for(2, 5, 1), Some(3));
        assert_eq!(os_ver_for(2, 6, 1), Some(4));
        assert_eq!(os_ver_for(2, 10, 0), Some(4));
        assert_eq!(os_ver_for(1, 4, 0), Some(0));
        assert_eq!(os_ver_for(1, 4, 10), Some(1));
        assert_eq!(os_ver_for(1, 5, 0), Some(1));
        assert_eq!(os_ver_for(0, 3, 1), None);
    }
}
