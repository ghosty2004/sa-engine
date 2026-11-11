//! CVehicleRecording, CStuckCarCheck, CUpsideDownCarCheck, CCurrentVehicle and CVehicleAnimGroup
//! (PORT-VEH). Ported from the 1.0 US bytes; gta-reversed as a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{ftol32, g, jp5, model_info, rd, wr, x};

sa_core::globals! {
    list GLOBALS;
    /// `CVehicleRecording::bPlaybackPaused[16]`
    pub static PLAYBACK_PAUSED @ 0x97D6D0: [u8; 16] = [0; 16];
    /// `CVehicleRecording::bPlaybackGoingOn[16]`
    pub static PLAYBACK_GOING_ON @ 0x97D6F0: [u8; 16] = [0; 16];
    /// `CVehicleRecording::pPlaybackBuffer[16]`
    pub static PLAYBACK_BUFFER @ 0x97D800: [GPtr<u8>; 16] = [GPtr::NULL; 16];
    /// `CVehicleRecording::pVehicleForPlayback[16]`
    pub static VEHICLE_FOR_PLAYBACK @ 0x97D840: [GPtr<u8>; 16] = [GPtr::NULL; 16];
    /// `CVehicleRecording::StreamingArray[475]` (`CPath`: +0 file number, +4 data, +8 size, +0xC refs)
    pub static STREAMING_ARRAY @ 0x97D880: [[u32; 4]; 475] = [[0; 4]; 475];
    /// `CVehicleRecording::NumPlayBackFiles`
    pub static NUM_PLAYBACK_FILES @ 0x97F630: i32 = 0;
}

orig! {
    /// `CVehicleRecording::Init()` (0x459390)
    pub fn recording_init_call() = cdecl 0x459390 => recording_init;
    /// `CUpsideDownCarCheck::IsCarUpsideDown(const CVehicle*)` (0x463830)
    pub fn is_car_upside_down_call(c: GPtr<u8>, v: GPtr<u8>) -> u8 = thiscall 0x463830 => is_car_upside_down;
    /// `CVehicle` virtual slot 0xE4 (gta-reversed: `CanPedStepOutCar(bool)`), called with `false`
    pub fn vehicle_vslot_e4(v: GPtr<u8>, arg: u32) -> u8 = thiscall vslot 0xE4 => unported;
    /// `CText::Get(const char*)` (0x6A0050)
    pub fn text_get(text: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CHud::SetVehicleName(const char*)` (0x588F50)
    pub fn hud_set_vehicle_name(name: GPtr<u8>) = cdecl 0x588F50 => unported;
}

// ---------------------------------------------------------------------------------------------
// CVehicleRecording

fn path(k: i32) -> GPtr<u8> {
    STREAMING_ARRAY.ptr().cast::<u8>().byte_add(16 * k as isize)
}

/// `CVehicleRecording::Init()` (0x459390): playback flags, buffers and vehicles cleared; every
/// StreamingArray entry's data pointer and reference count zeroed.
#[hook(addr = 0x459390, conv = cdecl, name = "CVehicleRecording::Init", expect = "33 C9 89 0D F0 D6 97 00")]
pub fn recording_init() {
    PLAYBACK_GOING_ON.set([0; 16]);
    PLAYBACK_PAUSED.set([0; 16]);
    PLAYBACK_BUFFER.set([GPtr::NULL; 16]);
    VEHICLE_FOR_PLAYBACK.set([GPtr::NULL; 16]);
    for k in 0..475 {
        wr::<u32>(path(k), 4, 0);
        wr::<u8>(path(k), 0xC, 0);
    }
}

/// `CVehicleRecording::InitAtStartOfGame()` (0x45A1B0): data pointers zeroed, then (tail jump) Init.
#[hook(addr = 0x45A1B0, conv = cdecl, name = "CVehicleRecording::InitAtStartOfGame", expect = "B8 84 D8 97 00")]
pub fn recording_init_at_start_of_game() {
    for k in 0..475 {
        wr::<u32>(path(k), 4, 0);
    }
    recording_init_call();
}

/// The `FindIndexWithFileNameNumber` loop (inlined in HasRecordingFileBeenLoaded): 0 when not found.
fn find_index(file: i32) -> i32 {
    let n = NUM_PLAYBACK_FILES.get();
    let mut i = 0;
    while i < n {
        if rd::<i32>(path(i), 0) == file {
            return i;
        }
        i += 1;
    }
    0
}

/// `CVehicleRecording::FindIndexWithFileNameNumber(int32)` (0x459FF0): 0 when not found.
#[hook(addr = 0x459FF0, conv = cdecl, name = "CVehicleRecording::FindIndexWithFileNameNumber", expect = "8B 15 30 F6 97 00")]
pub fn find_index_with_file_name_number(file: i32) -> i32 {
    find_index(file)
}

/// `CVehicleRecording::HasRecordingFileBeenLoaded(int32)` (0x45A060, body in .HOODLUM): data pointer
/// of the found (or 0th) entry non-null. EAX's upper bytes differ: the original leaves
/// `(index << 4) & ~0xFF` there (callers test AL).
#[hook(addr = 0x45A060, conv = cdecl, name = "CVehicleRecording::HasRecordingFileBeenLoaded", expect = "E9 2B 6E 10 01")]
pub fn has_recording_file_been_loaded(file: i32) -> u8 {
    u8::from(rd::<u32>(path(find_index(file)), 4) != 0)
}

/// `CVehicleRecording::IsPlaybackPausedForCar(CVehicle*)` (0x4595A0): the original returns false on
/// both paths (`xor al, al` after the search; EAX ends 0), so it is always false.
#[hook(addr = 0x4595A0, conv = cdecl, name = "CVehicleRecording::IsPlaybackPausedForCar", expect = "8B 4C 24 04 33 C0")]
pub fn is_playback_paused_for_car(_v: GPtr<u8>) -> u32 {
    0
}

// ---------------------------------------------------------------------------------------------
// CStuckCarCheck (16 entries of 0x24: +0 handle, +4 last pos, +0x10 last checked time, +0x14 radius,
// +0x18 time, +0x1C stuck, +0x1D warp, +0x1E warp camera, +0x1F warp only if not visible,
// +0x20 flip, +0x21 path to warp to)

type Stuck = GPtr<u8>;

fn stuck_reset(e: GPtr<u8>) {
    wr::<u32>(e, 0, 0xFFFF_FFFF);
    for k in 0..3 {
        wr::<u32>(e, 4 + 4 * k, 0xC59C_4000); // -5000.0
    }
    wr::<u32>(e, 0x10, 0xFFFF_FFFF);
    wr::<u32>(e, 0x14, 0);
    wr::<u32>(e, 0x18, 0);
    for k in 0x1C..0x22 {
        wr::<u8>(e, k, 0);
    }
}

/// `CStuckCarCheck::Init()` (0x4639E0): all 16 entries reset (bytes +0x22/+0x23 untouched).
#[hook(addr = 0x4639E0, conv = thiscall, name = "CStuckCarCheck::Init", expect = "83 EC 0C 53 55", this = "CStuckCarCheck")]
pub fn stuck_init(c: Stuck) {
    for k in 0..16 {
        stuck_reset(c.byte_add(0x24 * k));
    }
}

/// `CStuckCarCheck::ResetArrayElement(uint16)` (0x463970, body in .HOODLUM): no range check.
#[hook(addr = 0x463970, conv = thiscall, name = "CStuckCarCheck::ResetArrayElement", expect = "E9 4B 27 10 01", this = "CStuckCarCheck")]
pub fn stuck_reset_array_element(c: Stuck, index: u32) {
    stuck_reset(c.byte_add(0x24 * (index & 0xFFFF) as isize));
}

/// `CStuckCarCheck::ClearStuckFlagForCar(int32)` (0x463C40, body in .HOODLUM): every matching entry.
#[hook(addr = 0x463C40, conv = thiscall, name = "CStuckCarCheck::ClearStuckFlagForCar", expect = "E9 BB 16 10 01", this = "CStuckCarCheck")]
pub fn stuck_clear_stuck_flag_for_car(c: Stuck, handle: i32) {
    for k in 0..16 {
        let e = c.byte_add(0x24 * k);
        if rd::<i32>(e, 0) == handle {
            wr::<u8>(e, 0x1C, 0);
        }
    }
}

fn stuck_find(c: Stuck, handle: i32) -> Option<GPtr<u8>> {
    (0..16)
        .map(|k| c.byte_add(0x24 * k))
        .find(|&e| rd::<i32>(e, 0) == handle)
}

/// `CStuckCarCheck::HasCarBeenStuckForAWhile(int32)` (0x463C00, body in .HOODLUM).
#[hook(addr = 0x463C00, conv = thiscall, name = "CStuckCarCheck::HasCarBeenStuckForAWhile", expect = "E9 AB DD 0F 01", this = "CStuckCarCheck")]
pub fn stuck_has_car_been_stuck_for_a_while(c: Stuck, handle: i32) -> u8 {
    stuck_find(c, handle).map_or(0, |e| u8::from(rd::<u8>(e, 0x1C) != 0))
}

/// `CStuckCarCheck::IsCarInStuckCarArray(int32)` (0x463C70, body in .HOODLUM).
#[hook(addr = 0x463C70, conv = thiscall, name = "CStuckCarCheck::IsCarInStuckCarArray", expect = "E9 9B 66 10 01", this = "CStuckCarCheck")]
pub fn stuck_is_car_in_stuck_car_array(c: Stuck, handle: i32) -> u8 {
    u8::from(stuck_find(c, handle).is_some())
}

// ---------------------------------------------------------------------------------------------
// CUpsideDownCarCheck (6 entries of 8: +0 handle, +4 time)

type Upside = GPtr<u8>;

/// `CUpsideDownCarCheck::IsCarUpsideDown(const CVehicle*)` (0x463830): `this` is not read (ECX is
/// overwritten with the vehicle). Contact wheels: automobile (type 0) `+0x960`, bike (9) `+0x804`, else 0.
/// "Upside down" = `up.z < 0.3 && wheels < 4 || up.z < 0.0`; then the result is the vehicle's virtual
/// slot 0xE4 called with false (true when it returns non-zero). gta-reversed names that slot
/// CanPedStepOutCar and negates it; the bytes do not negate.
#[hook(addr = 0x463830, conv = thiscall, name = "CUpsideDownCarCheck::IsCarUpsideDown-vehicle", expect = "8B 4C 24 04 8B 81 90 05 00 00", this = "CUpsideDownCarCheck")]
pub fn is_car_upside_down(_c: Upside, v: GPtr<u8>) -> u8 {
    let wheels: u32 = match rd::<u32>(v, 0x590) {
        0 => u32::from(rd::<u8>(v, 0x960)),
        9 => u32::from(rd::<u8>(v, 0x804)),
        _ => 0,
    };
    let m = rd::<GPtr<u8>>(v, 0x14);
    let up = |k: f32| jp5(x(rd::<f32>(m, 0x28)), x(k));
    let cond = (!up(f32::from_bits(0x3E99_999A)) && wheels < 4) || !up(0.0);
    if cond && vehicle_vslot_e4(v, 0) != 0 {
        return 1;
    }
    0
}

/// `CUpsideDownCarCheck::UpdateTimers()` (0x4655E0): step = `_ftol(timeStep * 0.02 * 1000.0)`; per
/// entry, a handle that does not resolve in the vehicle pool (also -1) becomes -1 with time 0;
/// otherwise the time grows by the step while the car is upside down and is 0 when not.
#[hook(addr = 0x4655E0, conv = thiscall, name = "CUpsideDownCarCheck::UpdateTimers", expect = "51 D9 05 5C CB B7 00", this = "CUpsideDownCarCheck")]
pub fn update_timers(c: Upside) {
    let step = ftol32(
        x(g::<f32>(0xB7CB5C).read())
            .fmul_m(f32::from_bits(0x3CA3_D70A))
            .fmul_m(1000.0),
    ) as u32;
    for k in 0..6 {
        let e = c.byte_add(8 * k);
        let handle = rd::<i32>(e, 0);
        let pool = g::<GPtr<u8>>(0xB74494).read();
        let flags = rd::<GPtr<u8>>(pool, 4);
        let idx = handle >> 8;
        let veh = if flags.byte_add(idx as isize).read() == handle as u8 {
            rd::<u32>(pool, 0).wrapping_add((idx as u32).wrapping_mul(0xA18))
        } else {
            0
        };
        if veh == 0 {
            wr::<i32>(e, 0, -1);
            wr::<u32>(e, 4, 0);
            continue;
        }
        if is_car_upside_down_call(c, GPtr::from_addr(veh as usize)) != 0 {
            wr::<u32>(e, 4, rd::<u32>(e, 4).wrapping_add(step));
        } else {
            wr::<u32>(e, 4, 0);
        }
    }
}

/// `CUpsideDownCarCheck::AddCarToCheck(int32)` (0x4638D0, body in .HOODLUM): first entry whose handle
/// is negative.
#[hook(addr = 0x4638D0, conv = thiscall, name = "CUpsideDownCarCheck::AddCarToCheck", expect = "E9 DB 82 10 01", this = "CUpsideDownCarCheck")]
pub fn upside_add_car_to_check(c: Upside, handle: i32) {
    for k in 0..6 {
        let e = c.byte_add(8 * k);
        if rd::<i32>(e, 0) < 0 {
            wr::<i32>(e, 0, handle);
            wr::<u32>(e, 4, 0);
            return;
        }
    }
}

/// `CUpsideDownCarCheck::RemoveCarFromCheck(int32)` (0x463910, body in .HOODLUM): every matching entry.
#[hook(addr = 0x463910, conv = thiscall, name = "CUpsideDownCarCheck::RemoveCarFromCheck", expect = "E9 0B C1 10 01", this = "CUpsideDownCarCheck")]
pub fn upside_remove_car_from_check(c: Upside, handle: i32) {
    for k in 0..6 {
        let e = c.byte_add(8 * k);
        if rd::<i32>(e, 0) == handle {
            wr::<i32>(e, 0, -1);
            wr::<u32>(e, 4, 0);
        }
    }
}

/// `CUpsideDownCarCheck::HasCarBeenUpsideDownForAWhile(int32)` (0x463940, body in .HOODLUM): the first
/// matching entry's time > 2000 (unsigned).
#[hook(addr = 0x463940, conv = thiscall, name = "CUpsideDownCarCheck::HasCarBeenUpsideDownForAWhile", expect = "E9 2B EF 0F 01", this = "CUpsideDownCarCheck")]
pub fn upside_has_car_been_upside_down_for_a_while(c: Upside, handle: i32) -> u8 {
    for k in 0..6 {
        let e = c.byte_add(8 * k);
        if rd::<i32>(e, 0) == handle {
            return u8::from(rd::<u32>(e, 4) > 2000);
        }
    }
    0
}

// ---------------------------------------------------------------------------------------------
// CCurrentVehicle

/// `CCurrentVehicle::Process()` (0x572040): the focus player's ped (`CWorld::Players[PlayerInFocus]`)
/// in a vehicle (`+0x46C` bit 8): `this->m_pVehicle = ped->m_pVehicle` and, when non-null, the HUD
/// vehicle name = `TheText.Get(modelInfo->m_gameName)`; otherwise `m_pVehicle = 0` (when not in a
/// vehicle) and the HUD name null.
#[hook(addr = 0x572040, conv = thiscall, name = "CCurrentVehicle::Process", expect = "0F B6 05 74 CD B7 00")]
pub fn current_vehicle_process(c: GPtr<u8>) {
    let focus = u32::from(g::<u8>(0xB7CD74).read());
    let ped = g::<GPtr<u8>>(0xB7CD98 + 0x190 * focus as usize).read();
    if rd::<u32>(ped, 0x46C) & 0x100 != 0 {
        let v = rd::<GPtr<u8>>(ped, 0x58C);
        wr::<GPtr<u8>>(c, 0, v);
        if !v.is_null() {
            let mi = model_info(i32::from(rd::<i16>(v, 0x22)));
            let name = text_get(GPtr::from_addr(0xC1B340), mi.byte_add(0x32));
            hud_set_vehicle_name(name);
            return;
        }
    } else {
        wr::<u32>(c, 0, 0);
    }
    hud_set_vehicle_name(GPtr::NULL);
}

// ---------------------------------------------------------------------------------------------
// CVehicleAnimGroup

/// `CVehicleAnimGroup::CVehicleAnimGroup()` (0x5BD420): zeroes bytes +0, +1 and +4..+0x7C (the last
/// two 12-byte entries of the 0x94-byte struct stay uninitialised, as gta-reversed notes); returns
/// `this`.
#[hook(addr = 0x5BD420, conv = thiscall, name = "CVehicleAnimGroup::CVehicleAnimGroup", expect = "83 EC 0C 8B C1", this = "CVehicleAnimGroup")]
pub fn vehicle_anim_group_ctor(a: GPtr<u8>) -> GPtr<u8> {
    wr::<u8>(a, 0, 0);
    wr::<u8>(a, 1, 0);
    for k in 1..0x1F {
        wr::<u32>(a, 4 * k, 0);
    }
    a
}

pub const HOOKS: &[HookReg] = &[
    HOOK_RECORDING_INIT,
    HOOK_RECORDING_INIT_AT_START_OF_GAME,
    HOOK_FIND_INDEX_WITH_FILE_NAME_NUMBER,
    HOOK_HAS_RECORDING_FILE_BEEN_LOADED,
    HOOK_IS_PLAYBACK_PAUSED_FOR_CAR,
    HOOK_STUCK_INIT,
    HOOK_STUCK_RESET_ARRAY_ELEMENT,
    HOOK_STUCK_CLEAR_STUCK_FLAG_FOR_CAR,
    HOOK_STUCK_HAS_CAR_BEEN_STUCK_FOR_A_WHILE,
    HOOK_STUCK_IS_CAR_IN_STUCK_CAR_ARRAY,
    HOOK_IS_CAR_UPSIDE_DOWN,
    HOOK_UPDATE_TIMERS,
    HOOK_UPSIDE_ADD_CAR_TO_CHECK,
    HOOK_UPSIDE_REMOVE_CAR_FROM_CHECK,
    HOOK_UPSIDE_HAS_CAR_BEEN_UPSIDE_DOWN_FOR_A_WHILE,
    HOOK_CURRENT_VEHICLE_PROCESS,
    HOOK_VEHICLE_ANIM_GROUP_CTOR,
];
