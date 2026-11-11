//! cHandlingDataMgr (PORT-VEH). Ported from the 1.0 US bytes; gta-reversed as a reading aid.
//!
//! The manager (0xC624, global instance 0xC2B9C8): +0x14 car handling entries (0xE0 each, drive type at
//! entry +0x74), +0x7B24 + 0x58 * id flying data, +0x9A50 + 0x3C * id boat data. All members are
//! thiscall (the callers pass the manager in ECX even where it is unused).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr, x};

orig! {
    /// CRT `strncmp` (0x8214D0)
    pub fn strncmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x8214D0 => unported;
    /// CRT `strncpy` (0x821F40)
    pub fn strncpy(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
    /// `cHandlingDataMgr::LoadHandlingData()` (0x5BD830)
    pub fn load_handling_data(mgr: GPtr<u8>) = thiscall 0x5BD830 => unported;
}

/// `VehicleNames[210][14]` (0x8D3978), the handling.cfg identifiers.
const VEHICLE_NAMES: usize = 0x8D3978;

/// The drive-type test of HasFront/RearWheelDrive: EAX keeps the upper bytes of `id * 0xE0` from the
/// `imul` (only AL is replaced by the result).
fn drive_type_is_not(mgr: GPtr<u8>, id: u32, c: u8) -> u32 {
    let e = (id & 0xFF).wrapping_mul(0xE0);
    let t = rd::<u8>(mgr, e as isize + 0x88);
    (e & 0xFFFF_FF00) | u32::from(t != c)
}

/// `cHandlingDataMgr::HasFrontWheelDrive(uint8)` (0x6A0480): drive type != 'R'.
#[hook(addr = 0x6A0480, conv = thiscall, name = "cHandlingDataMgr::HasFrontWheelDrive", expect = "0F B6 44 24 04", this = "cHandlingDataMgr")]
pub fn has_front_wheel_drive(mgr: GPtr<u8>, id: u32) -> u32 {
    drive_type_is_not(mgr, id, b'R')
}

/// `cHandlingDataMgr::HasRearWheelDrive(uint8)` (0x6A04B0): drive type != 'F'.
#[hook(addr = 0x6A04B0, conv = thiscall, name = "cHandlingDataMgr::HasRearWheelDrive", expect = "0F B6 44 24 04", this = "cHandlingDataMgr")]
pub fn has_rear_wheel_drive(mgr: GPtr<u8>, id: u32) -> u32 {
    drive_type_is_not(mgr, id, b'F')
}

/// `cHandlingDataMgr::GetBoatPointer(uint8)` (0x6F5300): ids 0xAF..=0xBA -> `this + 0x9A50 + 0x3C * id`,
/// else `this + 0xC354`.
#[hook(addr = 0x6F5300, conv = thiscall, name = "cHandlingDataMgr::GetBoatPointer", expect = "8A 44 24 04 3C AF", this = "cHandlingDataMgr")]
pub fn get_boat_pointer(mgr: GPtr<u8>, id: u32) -> GPtr<u8> {
    let id = id as u8;
    if (0xAF..=0xBA).contains(&id) {
        mgr.byte_add(isize::from(id) * 0x3C + 0x9A50)
    } else {
        mgr.byte_add(0xC354)
    }
}

/// `cHandlingDataMgr::GetFlyingPointer(uint8)` (0x6F52D0, tail at 0x403751): ids 0xBA..=0xD1 ->
/// `this + 0x7B24 + 0x58 * id`, else `this + 0xBB14`.
#[hook(addr = 0x6F52D0, conv = thiscall, name = "cHandlingDataMgr::GetFlyingPointer", expect = "8A 44 24 04 3C BA", this = "cHandlingDataMgr")]
pub fn get_flying_pointer(mgr: GPtr<u8>, id: u32) -> GPtr<u8> {
    let id = id as u8;
    if (0xBA..=0xD1).contains(&id) {
        mgr.byte_add(isize::from(id) * 0x58 + 0x7B24)
    } else {
        mgr.byte_add(0xBB14)
    }
}

/// `cHandlingDataMgr::ConvertBikeDataToGameUnits(tBikeHandlingData*)` (0x6F5290): degrees to radians
/// (`* 0x8595EC`); +0x14, +0x2C and +0x30 also go through FSIN, +0x18 does not.
#[hook(addr = 0x6F5290, conv = thiscall, name = "cHandlingDataMgr::ConvertBikeDataToGameUnits", expect = "8B 44 24 04 D9 40 14", this = "cHandlingDataMgr")]
pub fn convert_bike_data_to_game_units(_mgr: GPtr<u8>, data: GPtr<u8>) {
    let k = g::<f32>(0x8595EC).read();
    wr::<f32>(data, 0x14, x(rd::<f32>(data, 0x14)).fmul_m(k).fsin().to_f32());
    wr::<f32>(data, 0x18, x(rd::<f32>(data, 0x18)).fmul_m(k).to_f32());
    wr::<f32>(data, 0x2C, x(rd::<f32>(data, 0x2C)).fmul_m(k).fsin().to_f32());
    wr::<f32>(data, 0x30, x(rd::<f32>(data, 0x30)).fmul_m(k).fsin().to_f32());
}

/// `cHandlingDataMgr::GetHandlingId(const char*)` (0x6F4FD0): index of the first of the 210
/// `VehicleNames` entries with `strncmp(name, entry, 14) == 0`; 210 when none matches.
#[hook(addr = 0x6F4FD0, conv = thiscall, name = "cHandlingDataMgr::GetHandlingId", expect = "53 8B 5C 24 08", this = "cHandlingDataMgr")]
pub fn get_handling_id(_mgr: GPtr<u8>, name: GPtr<u8>) -> i32 {
    let mut i = 0;
    while i < 210 {
        if strncmp(name, GPtr::from_addr(VEHICLE_NAMES + 14 * i as usize), 14) == 0 {
            break;
        }
        i += 1;
    }
    i
}

/// `cHandlingDataMgr::FindExactWord(const char* line, const char* table, int32 entrySize, int32 count)`
/// (0x6F4F30): do-while over the table (the first entry is tested even for `count <= 0`): copies the
/// entry's `strlen` characters to a 20-byte stack buffer with `strncpy` and compares `strncmp(line, buf,
/// len)`; returns the index of the first match, else `count`. Deviation: an entry longer than 20
/// characters overflows the original's stack frame; here it is compared in place instead.
#[hook(addr = 0x6F4F30, conv = thiscall, name = "cHandlingDataMgr::FindExactWord", expect = "83 EC 14 53 55", this = "cHandlingDataMgr")]
pub fn find_exact_word(_mgr: GPtr<u8>, line: GPtr<u8>, table: GPtr<u8>, entry_size: i32, count: i32) -> i32 {
    let mut i = 0i32;
    let mut p = table;
    loop {
        let mut len = 0u32;
        while rd::<u8>(p, len as isize) != 0 {
            len += 1;
        }
        let mut buf = [0u8; 0x14];
        let bp = GPtr::from_raw(&raw mut buf).cast::<u8>();
        let cmp = if len as usize <= buf.len() {
            strncpy(bp, p, len);
            strncmp(line, bp, len)
        } else {
            strncmp(line, p, len)
        };
        if cmp == 0 {
            return i;
        }
        i += 1;
        p = p.byte_add(entry_size as isize);
        if i >= count {
            return count;
        }
    }
}

/// `cHandlingDataMgr::ConvertDataToWorldUnits(tHandlingData*)` (0x6F5010): `t = 1.0 / [0xC2B9B4]`;
/// +0x7C = t * +0x7C, then (re-read from the f32 store) times 4.0 for drive type '4' or doubled; +0x84 /=
/// [0xC2B9BC]; +0x94 *= t; +0xC8 = +0x4 * +0xC8 * 0.0005.
#[hook(addr = 0x6F5010, conv = thiscall, name = "cHandlingDataMgr::ConvertDataToWorldUnits", expect = "D9 05 24 86 85 00", this = "cHandlingDataMgr")]
pub fn convert_data_to_world_units(_mgr: GPtr<u8>, data: GPtr<u8>) {
    let t = x(g::<f32>(0x858624).read()).fdiv_m(g::<f32>(0xC2B9B4).read());
    let four_wd = rd::<u8>(data, 0x74) == 0x34;
    let s = t.fmul_m(rd::<f32>(data, 0x7C)).to_f32();
    wr::<f32>(data, 0x7C, s);
    wr::<f32>(
        data,
        0x84,
        x(rd::<f32>(data, 0x84))
            .fdiv_m(g::<f32>(0xC2B9BC).read())
            .to_f32(),
    );
    wr::<f32>(data, 0x94, t.fmul_m(rd::<f32>(data, 0x94)).to_f32());
    let r = if four_wd {
        x(s).fmul_m(g::<f32>(0x858B90).read())
    } else {
        x(s).fdouble()
    };
    wr::<f32>(data, 0x7C, r.to_f32());
    let v = x(rd::<f32>(data, 4))
        .fmul_m(rd::<f32>(data, 0xC8))
        .fmul_m(g::<f32>(0x859978).read());
    wr::<f32>(data, 0xC8, v.to_f32());
}

/// `cHandlingDataMgr::Initialise()` (0x5BF3D0): LoadHandlingData, then the five manager floats
/// +0..+0x10 (0.1, 0.9, 1.0, 0.8, 0.98).
#[hook(addr = 0x5BF3D0, conv = thiscall, name = "cHandlingDataMgr::Initialise", expect = "56 8B F1 E8 58 E4 FF FF", this = "cHandlingDataMgr")]
pub fn initialise(mgr: GPtr<u8>) {
    load_handling_data(mgr);
    wr::<u32>(mgr, 0, 0x3DCC_CCCD);
    wr::<u32>(mgr, 4, 0x3F66_6666);
    wr::<u32>(mgr, 8, 0x3F80_0000);
    wr::<u32>(mgr, 0xC, 0x3F4C_CCCD);
    wr::<u32>(mgr, 0x10, 0x3F7A_E148);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_HAS_FRONT_WHEEL_DRIVE,
    HOOK_HAS_REAR_WHEEL_DRIVE,
    HOOK_GET_BOAT_POINTER,
    HOOK_GET_FLYING_POINTER,
    HOOK_CONVERT_BIKE_DATA_TO_GAME_UNITS,
    HOOK_GET_HANDLING_ID,
    HOOK_FIND_EXACT_WORD,
    HOOK_CONVERT_DATA_TO_WORLD_UNITS,
    HOOK_INITIALISE,
];

/// This module's globals (replace with a `sa_core::globals! { list GLOBALS; ... }` block when it gets some).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
