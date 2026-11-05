//! `CTimeInfo` (4 bytes: on hour, off hour, other time model), `CTimeModelInfo` (vtable 0x85BCB0,
//! time info at +0x20) and `CLodTimeModelInfo` (vtable 0x85BCF0, time info at +0x24).
//!
//! Ported from the 1.0 US bytes; gta-reversed `Models/TimeInfo.cpp`, `TimeModelInfo.cpp`,
//! `LodTimeModelInfo.cpp` are reading aids. Kept from the original:
//! - `CLodTimeModelInfo::GetModelType`/`GetTimeInfo` are 0x4C56C0/0x4C56D0 (gr hooks
//!   CTimeModelInfo's addresses); `GetTimeInfo` returns `this + 0x24`;
//! - `FindOtherTimeModel` copies the name into a 0x18-byte stack buffer with an unbounded copy (the
//!   port's buffer is 0x100 bytes: names of 0x18 bytes or more overwrite the original's stack, which
//!   is original UB; the port truncates at 0xFF), swaps `_nt`↔`_dy` in place with `strncpy(p, s, 4)`,
//!   and returns the matching model's `CTimeInfo*` (the virtual `GetTimeInfo` result), or null.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::atomic::scalar_delete;
use super::base::{f, vcall_get_time_info, MiPtr, VTBL_BASE_MODEL_INFO};
use crate::shared::{mi, model_info, NUM_MODEL_INFOS};

sa_core::globals! {
    list GLOBALS;
}

orig! {
    /// CRT `strstr` (0x822650)
    pub fn strstr(s: GPtr<u8>, sub: GPtr<u8>) -> GPtr<u8> = cdecl 0x822650 => unported;
    /// CRT `strncpy` (0x821F40)
    pub fn strncpy(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
    /// `CKeyGen::GetUppercaseKey` (0x53CF30)
    pub fn get_uppercase_key(s: GPtr<u8>) -> u32 = cdecl 0x53CF30 => sa_core::keygen::get_uppercase_key;
    /// `CTimeModelInfo::~CTimeModelInfo` (0x4C5680)
    pub fn time_destructor_call(this: MiPtr) = thiscall 0x4C5680 => time_destructor;
    /// `CLodTimeModelInfo::~CLodTimeModelInfo` (0x4C56E0)
    pub fn lod_time_destructor_call(this: MiPtr) = thiscall 0x4C56E0 => lod_time_destructor;
}

/// A `CTimeInfo` pointer.
pub type TimeInfoPtr = GPtr<u8>;

/// "_nt" (0x85BB94)
pub const STR_NT: u32 = 0x85BB94;
/// "_dy" (0x85BB90)
pub const STR_DY: u32 = 0x85BB90;

/// `CTimeInfo::GetTimeOn` (0x407320): `movzx eax, byte [ecx]` (unhookable: 4 bytes).
pub fn get_time_on(this: TimeInfoPtr) -> u32 {
    this.read() as u32
}

/// `CTimeInfo::GetTimeOff` (0x407330): `movzx eax, byte [ecx + 1]`.
#[hook(addr = 0x407330, conv = thiscall, name = "CTimeInfo::GetTimeOff", expect = "0F B6 41 01 C3", this = "CTimeInfo")]
pub fn get_time_off(this: TimeInfoPtr) -> u32 {
    this.byte_add(1).read() as u32
}

/// `CTimeInfo::GetOtherTimeModel` (0x4C4A30): `movsx eax, word [ecx + 2]`.
#[hook(addr = 0x4C4A30, conv = thiscall, name = "CTimeInfo::GetOtherTimeModel", expect = "0F BF 41 02 C3", this = "CTimeInfo")]
pub fn get_other_time_model(this: TimeInfoPtr) -> i32 {
    this.byte_add(2).cast::<i16>().read() as i32
}

/// `CTimeInfo::SetTimes(uint8, uint8)` (0x5B3430, `ret 8`).
#[hook(addr = 0x5B3430, conv = thiscall, name = "CTimeInfo::SetTimes", expect = "8A 44 24 04 8A 54 24 08", this = "CTimeInfo")]
pub fn set_times(this: TimeInfoPtr, on: u32, off: u32) {
    this.write(on as u8);
    this.byte_add(1).write(off as u8);
}

/// `CTimeInfo::SetOtherTimeModel(int16)` (0x5B3440, `ret 4`).
#[hook(addr = 0x5B3440, conv = thiscall, name = "CTimeInfo::SetOtherTimeModel", expect = "66 8B 44 24 04", this = "CTimeInfo")]
pub fn set_other_time_model(this: TimeInfoPtr, other: u32) {
    this.byte_add(2).cast::<u16>().write(other as u16);
}

/// `CTimeInfo::FindOtherTimeModel(const char*)` (0x4C47E0, `ret 4`).
#[hook(addr = 0x4C47E0, conv = thiscall, name = "CTimeInfo::FindOtherTimeModel", expect = "8B 44 24 04 83 EC 18", this = "CTimeInfo")]
pub fn find_other_time_model(this: TimeInfoPtr, name: GPtr<u8>) -> GPtr<u8> {
    let mut buf = [0u8; 0x100];
    for (k, b) in buf.iter_mut().enumerate().take(0xFF) {
        let c = name.byte_add(k as isize).read();
        *b = c;
        if c == 0 {
            break;
        }
    }
    let bp = GPtr::from_raw(buf.as_mut_ptr());
    let nt = strstr(bp, GPtr::from_addr(STR_NT as usize));
    if !nt.is_null() {
        strncpy(nt, GPtr::from_addr(STR_DY as usize), 4);
    } else {
        let dy = strstr(bp, GPtr::from_addr(STR_DY as usize));
        if dy.is_null() {
            return GPtr::NULL;
        }
        strncpy(dy, GPtr::from_addr(STR_NT as usize), 4);
    }
    let key = get_uppercase_key(bp);
    for i in 0..NUM_MODEL_INFOS as i32 {
        let m = model_info(i);
        if m.is_null() {
            continue;
        }
        let ti = vcall_get_time_info(m);
        if !ti.is_null() && f::<u32>(m, mi::KEY).read() == key {
            this.byte_add(2).cast::<u16>().write(i as u16);
            return ti;
        }
    }
    GPtr::NULL
}

// ---------------------------------------------------------------------------- CTimeModelInfo ---

/// `CTimeModelInfo::GetModelType` (0x4C5660): AL = 3 (unhookable).
pub fn time_get_model_type(_this: MiPtr) -> u8 {
    3
}

/// `CTimeModelInfo::GetTimeInfo` (0x4C5670): `this + 0x20` (unhookable).
pub fn time_get_time_info(this: MiPtr) -> TimeInfoPtr {
    this.byte_add(0x20).cast()
}

/// `CTimeModelInfo::~CTimeModelInfo` (0x4C5680, `FUN_004c5680`).
#[hook(addr = 0x4C5680, conv = thiscall, name = "FUN_004c5680", expect = "C7 01 9C BB 85 00", this = "CTimeModelInfo")]
pub fn time_destructor(this: MiPtr) {
    f::<u32>(this, mi::VTABLE).write(VTBL_BASE_MODEL_INFO);
}

/// `CTimeModelInfo`'s scalar deleting destructor (0x4C6270).
#[hook(addr = 0x4C6270, conv = thiscall, name = "CTimeModelInfo::vfunc_0", expect = "56 8B F1 E8 08 F4 FF FF", this = "CTimeModelInfo")]
pub fn time_scalar_deleting_destructor(this: MiPtr, flags: u32) -> MiPtr {
    scalar_delete(this, flags, time_destructor_call)
}

// ------------------------------------------------------------------------- CLodTimeModelInfo ---

/// `CLodTimeModelInfo::GetModelType` (0x4C56C0, catalogue `vfunc_4`): AL = 3 (unhookable).
pub fn lod_time_get_model_type(_this: MiPtr) -> u8 {
    3
}

/// `CLodTimeModelInfo::GetTimeInfo` (0x4C56D0, catalogue `vfunc_5`): `this + 0x24` (unhookable).
pub fn lod_time_get_time_info(this: MiPtr) -> TimeInfoPtr {
    this.byte_add(0x24).cast()
}

/// `CLodTimeModelInfo::~CLodTimeModelInfo` (0x4C56E0, `FUN_004c56e0`).
#[hook(addr = 0x4C56E0, conv = thiscall, name = "FUN_004c56e0", expect = "C7 01 9C BB 85 00", this = "CLodTimeModelInfo")]
pub fn lod_time_destructor(this: MiPtr) {
    f::<u32>(this, mi::VTABLE).write(VTBL_BASE_MODEL_INFO);
}

/// `CLodTimeModelInfo`'s scalar deleting destructor (0x4C6290).
#[hook(addr = 0x4C6290, conv = thiscall, name = "CLodTimeModelInfo::vfunc_0", expect = "56 8B F1 E8 48 F4 FF FF", this = "CLodTimeModelInfo")]
pub fn lod_time_scalar_deleting_destructor(this: MiPtr, flags: u32) -> MiPtr {
    scalar_delete(this, flags, lod_time_destructor_call)
}

/// Every `#[hook]` of this module, in source order.
pub const HOOKS: &[HookReg] = &[
    HOOK_GET_TIME_OFF,
    HOOK_GET_OTHER_TIME_MODEL,
    HOOK_SET_TIMES,
    HOOK_SET_OTHER_TIME_MODEL,
    HOOK_FIND_OTHER_TIME_MODEL,
    HOOK_TIME_DESTRUCTOR,
    HOOK_TIME_SCALAR_DELETING_DESTRUCTOR,
    HOOK_LOD_TIME_DESTRUCTOR,
    HOOK_LOD_TIME_SCALAR_DELETING_DESTRUCTOR,
];
