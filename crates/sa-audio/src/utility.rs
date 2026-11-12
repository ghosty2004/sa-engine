//! `CAEAudioUtility`, `CAEAudioEnvironment`, `CAEAudioChannel` / `CAEStaticChannel` game-side helpers and `CAudioZones` (PORT-AUDIO).
//! Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/AUDIO/*.md` are reading aids.
//!
//! Pure helpers (no game state; host-callable with explicit arguments): [`piecewise_linear_with`],
//! [`bytes_to_ms`], [`ms_to_bytes`]. The hooks around them only read their arguments (and, for
//! `GetPiecewiseLinear`, the caller's table through its pointer).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, rd, wr, x, xi};

sa_core::globals! {
    list GLOBALS;
    /// `CAudioZones::m_aActiveBoxes[10]` (box indices)
    pub static AZ_ACTIVE_BOXES @ 0xB6DC6C: [i32; 10] = [0; 10];
    /// `CAudioZones::m_aActiveSpheres[10]` (sphere indices)
    pub static AZ_ACTIVE_SPHERES @ 0xB6DC94: [i32; 10] = [0; 10];
    /// `CAudioZones::m_NumActiveBoxes`
    pub static AZ_NUM_ACTIVE_BOXES @ 0xB6DCBC: u32 = 0;
    /// `CAudioZones::m_NumActiveSpheres`
    pub static AZ_NUM_ACTIVE_SPHERES @ 0xB6DCC0: u32 = 0;
    /// `CAudioZones::m_NumBoxes`
    pub static AZ_NUM_BOXES @ 0xB6DCC4: u32 = 0;
    /// `CAudioZones::m_NumSpheres`
    pub static AZ_NUM_SPHERES @ 0xB6DCC8: u32 = 0;
    /// `CAudioZones::m_aBoxes[158]` (`tAudioZoneBox`, 0x18: name[8], id u16 +8, flags u16 +0xA,
    /// min i16[3] +0xC, max i16[3] +0x12; coordinates * 8)
    pub static AZ_BOXES @ 0xB6DCD0: [u8; 158 * 0x18] = [0; 158 * 0x18];
    /// `CAudioZones::Update`'s last update position (`LastUpdateCoors`, CVector)
    pub static AZ_LAST_UPDATE_POS @ 0xB6EBFC: [u32; 3] = [0; 3];
    /// `CAudioZones::m_aSpheres[3]` (`tAudioZoneSphere`, 0x1C: name[8], id u16 +8, flags u16 +0xA,
    /// position +0xC, radius +0x18)
    pub static AZ_SPHERES @ 0xB6EBA8: [u8; 3 * 0x1C] = [0; 3 * 0x1C];
}

/// `1/32768` (0x858B14)
const INV_32768: f32 = f32::from_bits(0x3800_0000);
/// `1/RAND_MAX` (0x858C7C)
const INV_RAND_MAX: f32 = f32::from_bits(0x3800_0100);
/// `2^32` (0x858C54): the unsigned fix-up after `fild` of a negative int32
const TWO_POW_32: f32 = f32::from_bits(0x4F80_0000);
/// `0.002` (0x858F44): 2 bytes per 16-bit sample / 1000 ms
const BYTES_PER_MS_SCALE: f32 = f32::from_bits(0x3B03_126F);

/// Return address of the `call rand` in `GetRandomNumberInRange<int>` (0x4D9C12).
const RAND_RET_INT: u32 = 0x4D9C17;
/// Return address of the `call rand` in `GetRandomNumberInRange<float>` (0x4D9C51).
const RAND_RET_FLOAT: u32 = 0x4D9C56;
/// Return address of the `call rand` in `ResolveProbability` (0x4D9C92).
const RAND_RET_PROB: u32 = 0x4D9C97;

orig! {
    /// `CAEAudioUtility::AudioLog10(float)` (0x4D9E50) as callers reach it (80-bit ST0 result)
    pub fn call_audio_log10(v: f32) -> X87 = reg("args = stack") 0x4D9E50 => audio_log10;
    /// `FindPlayerPed(int32 playerId)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `FindPlayerVehicle(int32 playerId, bool bIncludeRemote)` (0x56E0D0)
    pub fn find_player_vehicle(id: i32, remote: u8) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CAEAudioUtility::ConvertFromBytesToMS(uint32, uint32, uint16)` (0x4D9EF0)
    pub fn convert_from_bytes_to_ms_call(bytes: u32, freq: u32, channels: u16) -> u32 = cdecl 0x4D9EF0 => convert_from_bytes_to_ms;
    /// `CAEAudioUtility::ConvertFromMSToBytes(uint32, uint32, uint16)` (0x4D9F40)
    pub fn convert_from_ms_to_bytes_call(ms: u32, freq: u32, channels: u16) -> u32 = cdecl 0x4D9F40 => convert_from_ms_to_bytes;
}

/// `CWorld::PlayerInFocus` (owned by sa-world)
const PLAYER_IN_FOCUS: usize = 0xB7CD74;

// ---------------------------------------------------------------------------------------------------
// CAEAudioUtility
// ---------------------------------------------------------------------------------------------------

/// `CAEAudioUtility::GetRandomNumberInRange<int32>(int32 min, int32 max)` (0x4D9C10):
/// `_ftol((rand() & 0xFFFF) * (1/32768) * (max - min + 1)) + min` (the multiplications under PC=24,
/// the second one `fimul`).
#[hook(addr = 0x4D9C10, conv = cdecl, name = "CAEAudioUtility::GetRandomNumberInRange<int32>-int", expect = "51 56 E8 07 7F 34 00")]
pub fn get_random_number_in_range_int(min: i32, max: i32) -> i32 {
    let r = sa_core::orig::at_site(RAND_RET_INT, sa_core::crt::rand) & 0xFFFF;
    let n = max.wrapping_sub(min).wrapping_add(1);
    ftol32(xi(r).fmul_m(INV_32768).fmul(xi(n))).wrapping_add(min)
}

/// `CAEAudioUtility::GetRandomNumberInRange<float>(float min, float max)` (0x4D9C50):
/// `rand() * (1/RAND_MAX) * (max - min) + min` (ST0).
#[hook(addr = 0x4D9C50, conv = cdecl, name = "CAEAudioUtility::GetRandomNumberInRange<float>-float", expect = "51 E8 C8 7E 34 00")]
pub fn get_random_number_in_range_float(min: f32, max: f32) -> X87 {
    let r = sa_core::orig::at_site(RAND_RET_FLOAT, sa_core::crt::rand);
    let t = xi(r).fmul_m(INV_RAND_MAX);
    t.fmul(x(max).fsub_m(min)).fadd_m(min)
}

/// `CAEAudioUtility::ResolveProbability(float p)` (0x4D9C80): true without a rand call unless
/// `p < 1.0` (a NaN `p` draws too); otherwise `rand() * (1/RAND_MAX) < p`.
#[hook(addr = 0x4D9C80, conv = cdecl, name = "CAEAudioUtility::ResolveProbability", expect = "51 D9 44 24 08")]
pub fn resolve_probability(p: f32) -> u8 {
    if !x(p).fcom(x(1.0)).c0() {
        return 1;
    }
    let r = sa_core::orig::at_site(RAND_RET_PROB, sa_core::crt::rand);
    let c = xi(r).fmul_m(INV_RAND_MAX).fcom(x(p));
    // test ah, 5; jp -> false (greater / equal / unordered)
    if c.test_parity(0x05) { 0 } else { 1 }
}

/// `GetPiecewiseLinear` on a table read through `pt(i)` = the `i`-th float of the (x, y) pairs
/// (`2*k` = x of point k, `2*k+1` its y; `k` may be -1 when `n <= 0`, as the bytes read it).
pub fn piecewise_linear_with(v: f32, n: i16, pt: impl Fn(isize) -> f32) -> X87 {
    let last = isize::from(n) - 1;
    // fcomp last.x; test ah, 41h; jne: continue when v <= last.x or unordered
    if !x(v).fcom(x(pt(2 * last))).test_any(0x41) {
        return x(pt(2 * last + 1));
    }
    // fcomp first.x; test ah, 41h; jp: continue when v > first.x or unordered
    if !x(v).fcom(x(pt(0))).test_parity(0x41) {
        return x(pt(1));
    }
    let mut i: i16 = 0;
    if n > 0 {
        while x(pt(2 * isize::from(i))).fcom(x(v)).c0() {
            i = i.wrapping_add(1);
            if i >= n {
                break;
            }
        }
    }
    let k = isize::from(i);
    let (x0, y0, x1, y1) = (pt(2 * k - 2), pt(2 * k - 1), pt(2 * k), pt(2 * k + 1));
    let a = x(v).fsub_m(x0);
    let b = x(x1).fsub_m(x0);
    a.fdiv(b).fmul(x(y1).fsub_m(y0)).fadd_m(y0)
}

/// `CAEAudioUtility::GetPiecewiseLinear(float x, int16 numPoints, const float (*table)[2])` (0x4D9D90):
/// above the last point its y, at or below the first point its y, otherwise linear interpolation
/// between the first point whose x is not below `x` and the one before it (ST0).
#[hook(addr = 0x4D9D90, conv = cdecl, name = "CAEAudioUtility::GetPiecewiseLinear", expect = "8B 54 24 0C D9 44 24 04")]
pub fn get_piecewise_linear(v: f32, n: i16, table: GPtr<f32>) -> X87 {
    piecewise_linear_with(v, n, |i| table.byte_add(i * 4).read())
}

/// `floor` (CRT 0x8219F0) of an f64: exact.
fn floor_f64(v: f64) -> f64 {
    if v.is_nan() || v.abs() >= 4_503_599_627_370_496.0 {
        return v;
    }
    let t = v as i64 as f64;
    if t > v { t - 1.0 } else { t }
}

/// `fild dword` of a uint32 as MSVC emits it: signed load, `fadd 2^32` (PC=24) when negative.
fn fild_u32(v: u32) -> X87 {
    let r = xi(v as i32);
    if (v as i32) < 0 { r.fadd_m(TWO_POW_32) } else { r }
}

/// Pure body of `ConvertFromBytesToMS`: `_ftol(floor(bytes / (freq * channels * 0.002)))`.
pub fn bytes_to_ms(bytes: u32, freq: u32, channels: u16) -> u32 {
    let num = fild_u32(bytes);
    let den = fild_u32(u32::from(channels).wrapping_mul(freq)).fmul_m(BYTES_PER_MS_SCALE);
    let q = num.fdiv(den).to_f64(); // fstp qword: exact
    ftol32(X87::from_f64_exact(floor_f64(q))) as u32
}

/// Pure body of `ConvertFromMSToBytes`: `b = _ftol(floor(freq * channels * 0.002 * ms))`, then
/// `b + b % (2 * channels)` (signed `idiv`, as the bytes).
pub fn ms_to_bytes(ms: u32, freq: u32, channels: u16) -> u32 {
    let rate = fild_u32(u32::from(channels).wrapping_mul(freq)).fmul_m(BYTES_PER_MS_SCALE);
    let p = rate.fmul(fild_u32(ms)).to_f64();
    let b = ftol32(X87::from_f64_exact(floor_f64(p)));
    let d = 2 * i32::from(channels);
    (b.wrapping_rem(d)).wrapping_add(b) as u32
}

/// `AudioLog10`'s threshold (0x858C14, `.rdata`): 1e-5.
pub const AUDIO_LOG10_MIN: f32 = f32::from_bits(0x3727_C5AC);
/// `AudioLog10`'s result below the threshold (0x858FC8, `.rdata`): -5.0.
pub const AUDIO_LOG10_FLOOR: f32 = f32::from_bits(0xC0A0_0000);

/// `CAEAudioUtility::AudioLog10(float)` (0x4D9E50): `fld x; fcomp [1e-5]; fnstsw; test ah, 1` -- below
/// the threshold or NaN the constant -5.0, else `fldlg2; fld x; fyl2x`, whose full 64-bit result is
/// returned in ST0 (a wide [`X87`]). Every caller multiplies it by an f32 constant at PC=24 (20.0 or
/// 10.0) right away; with the model that product is exact against the CPU (L1 spec `AudioLog10*c`,
/// CPU-decided). The raw ST0 itself matches the CPU's in ~99.4% of inputs (1 ulp of the 64-bit
/// significand otherwise, ledger). Hooked with the register-convention spec `args = stack` (HARNESS-5),
/// whose entry returns an `X87` result with all 80 bits (`fld tbyte`); Rust callers go through
/// [`call_audio_log10`].
#[hook(addr = 0x4D9E50, conv = reg("args = stack"), name = "CAEAudioUtility::AudioLog10",
       expect = "D9 44 24 04 D8 1D 14 8C 85 00",
       unverified = "harness5-1", route = "L2", hazards = "x87-transcendental", recs = "driving",
       note = "80-bit ST0 result (HARNESS-5 reg conv); FYL2X model 1 ulp (64-bit) off in ~0.6% of inputs, ledger")]
pub fn audio_log10(v: f32) -> X87 {
    let xv = X87::from_f32(v);
    if xv.fcom(X87::from_f32(AUDIO_LOG10_MIN)).c0() {
        return X87::from_f32(AUDIO_LOG10_FLOOR);
    }
    X87::lg2().fyl2x(xv)
}

/// `CAEAudioUtility::ConvertFromBytesToMS(uint32 bytes, uint32 freq, uint16 channels)` (0x4D9EF0).
#[hook(addr = 0x4D9EF0, conv = cdecl, name = "CAEAudioUtility::ConvertFromBytesToMS", expect = "8B 44 24 04 DB 44 24 04")]
pub fn convert_from_bytes_to_ms(bytes: u32, freq: u32, channels: u16) -> u32 {
    bytes_to_ms(bytes, freq, channels)
}

/// `CAEAudioUtility::ConvertFromMSToBytes(uint32 ms, uint32 freq, uint16 channels)` (0x4D9F40).
#[hook(addr = 0x4D9F40, conv = cdecl, name = "CAEAudioUtility::ConvertFromMSToBytes", expect = "56 0F B7 74 24 10")]
pub fn convert_from_ms_to_bytes(ms: u32, freq: u32, channels: u16) -> u32 {
    ms_to_bytes(ms, freq, channels)
}

/// `CAEAudioUtility::FindVehicleOfPlayer()` (stub 0x4D9E10, body split at 0x401A99 / 0x4D9E17): the
/// focus player's vehicle (`FindPlayerVehicle(PlayerInFocus, true)`); when there is none, the vehicle
/// the player ped's +0xFC entity is (entity type 2), else null.
/// Not hooked: the protection reads its entry bytes as data (`sbb ebx, [0x4D9E12]` at 0x1561A3A);
/// callers reach it through `orig!` (0x4D9E10). A phase B native (STD-8).
#[sa_core::native(addr = 0x4D9E10, conv = cdecl, name = "CAEAudioUtility::FindVehicleOfPlayer")]
pub fn find_vehicle_of_player() -> GPtr<u8> {
    let focus = i32::from(g::<u8>(PLAYER_IN_FOCUS).read());
    let veh = find_player_vehicle(focus, 1);
    let ped = find_player_ped(-1);
    if veh.is_null() && !ped.is_null() {
        let e = rd::<GPtr<u8>>(ped, 0xFC);
        if !e.is_null() && rd::<u8>(e, 0x36) & 7 == 2 {
            return e;
        }
    }
    veh
}

// ---------------------------------------------------------------------------------------------------
// CAEAudioChannel (game-side)
// ---------------------------------------------------------------------------------------------------

/// `CAEAudioChannel::ConvertFromBytesToMS(uint32 bytes)` (0x4D79D0): with the channel's frequency
/// (+0x4B) and channel count (+0x49).
#[hook(addr = 0x4D79D0, conv = thiscall, name = "CAEAudioChannel::ConvertFromBytesToMS", expect = "8B 54 24 04 33 C0")]
pub fn channel_convert_from_bytes_to_ms(this: GPtr<u8>, bytes: u32) -> u32 {
    convert_from_bytes_to_ms_call(bytes, rd::<u32>(this, 0x4B), rd::<u16>(this, 0x49))
}

/// `CAEAudioChannel::ConvertFromMsToBytes(uint32 ms)` (0x4D79F0).
#[hook(addr = 0x4D79F0, conv = thiscall, name = "CAEAudioChannel::ConvertFromMsToBytes", expect = "8B 54 24 04 33 C0")]
pub fn channel_convert_from_ms_to_bytes(this: GPtr<u8>, ms: u32) -> u32 {
    convert_from_ms_to_bytes_call(ms, rd::<u32>(this, 0x4B), rd::<u16>(this, 0x49))
}

/// `CAEAudioChannel::CAEAudioChannel(IDirectSound*, uint16 channelId, uint32 samplesPerSec, uint16
/// bitsPerSample)` (0x4D7890): the base channel's fields in the original's store order (vtable
/// 0x85F03C, bytes per second `(bits >> 3) * rate` at +0x4F, volume -100 dB); returns `this`.
#[hook(addr = 0x4D7890, conv = thiscall, name = "CAEAudioChannel::CAEAudioChannel", expect = "8B C1 8B 4C 24 04")]
pub fn audio_channel_ctor(this: GPtr<u8>, ds: u32, id: u32, rate: u32, bits: u32) -> GPtr<u8> {
    let bits = bits as u16;
    wr::<u16>(this, 0x55, bits);
    wr::<u32>(this, 4, ds);
    wr::<u16>(this, 0x3A, id as u16);
    wr::<u32>(this, 0x4F, u32::from(bits >> 3).wrapping_mul(rate));
    wr::<u32>(this, 0, 0x85F03C);
    wr::<u32>(this, 0x28, 0);
    wr::<u32>(this, 0x5C, 0);
    wr::<u32>(this, 0x3C, rate);
    wr::<u16>(this, 0x57, 0);
    wr::<u16>(this, 0x53, 2);
    wr::<u16>(this, 0x49, 1);
    wr::<u32>(this, 0x4B, rate);
    wr::<u32>(this, 0x40, rate);
    wr::<u16>(this, 0x47, 1);
    wr::<u32>(this, 0x34, 0xC2C8_0000); // -100.0
    wr::<u32>(this, 8, 0);
    wr::<u32>(this, 0xC, 0);
    wr::<u8>(this, 0x38, 0);
    wr::<u8>(this, 0x44, 0);
    wr::<u8>(this, 0x45, 0);
    this
}

// ---------------------------------------------------------------------------------------------------
// CAudioZones
// ---------------------------------------------------------------------------------------------------

/// `CAudioZones::Init()` (0x5081A0): all four counters 0.
#[hook(addr = 0x5081A0, conv = cdecl, name = "CAudioZones::Init", expect = "33 C0 A3 C8 DC B6 00")]
pub fn audio_zones_init() {
    AZ_NUM_SPHERES.set(0);
    AZ_NUM_BOXES.set(0);
    AZ_NUM_ACTIVE_SPHERES.set(0);
    AZ_NUM_ACTIVE_BOXES.set(0);
}

/// `strcpy(dst, src)` as the inlined byte loop.
fn copy_name(dst: GPtr<u8>, src: GPtr<u8>) {
    let mut i = 0isize;
    loop {
        let c: u8 = rd(src, i);
        wr::<u8>(dst, i, c);
        i += 1;
        if c == 0 {
            break;
        }
    }
}

/// `flags = (flags & ~1) | (active & 1)` on the u16 at +0xA.
fn set_active_flag(z: GPtr<u8>, active: u8) {
    let f = (rd::<u16>(z, 0xA) & !1) | u16::from(active & 1);
    wr::<u16>(z, 0xA, f);
}

/// `CAudioZones::RegisterAudioSphere(char* name, int32 id, bool active, CVector pos, float radius)`
/// (0x5081C0): fills the next sphere (no bound check) and counts it. Floats are copied as bits.
#[hook(addr = 0x5081C0, conv = cdecl, name = "CAudioZones::RegisterAudioSphere", expect = "A1 C8 DC B6 00")]
pub fn register_audio_sphere(name: GPtr<u8>, id: i32, active: u8, px: u32, py: u32, pz: u32, radius: u32) {
    let n = AZ_NUM_SPHERES.get();
    let s = AZ_SPHERES
        .ptr()
        .cast::<u8>()
        .byte_add((n as isize).wrapping_mul(0x1C));
    copy_name(s, name);
    wr::<u16>(s, 8, id as u16);
    set_active_flag(s, active);
    wr::<u32>(s, 0xC, px);
    wr::<u32>(s, 0x10, py);
    wr::<u32>(s, 0x14, pz);
    wr::<u32>(s, 0x18, radius);
    AZ_NUM_SPHERES.set(n.wrapping_add(1));
}

/// `CAudioZones::RegisterAudioBox(char* name, int32 id, bool active, CVector min, CVector max)`
/// (0x508240): fills the next box (no bound check), coordinates stored as `(int16)_ftol(c * 8)`.
#[hook(addr = 0x508240, conv = cdecl, name = "CAudioZones::RegisterAudioBox", expect = "8B 44 24 04 53")]
#[allow(clippy::too_many_arguments)]
pub fn register_audio_box(
    name: GPtr<u8>,
    id: i32,
    active: u8,
    min_x: f32,
    min_y: f32,
    min_z: f32,
    max_x: f32,
    max_y: f32,
    max_z: f32,
) {
    let n = AZ_NUM_BOXES.get();
    let b = AZ_BOXES
        .ptr()
        .cast::<u8>()
        .byte_add((n as isize).wrapping_mul(0x18));
    copy_name(b, name);
    wr::<u16>(b, 8, id as u16);
    set_active_flag(b, active);
    for (off, c) in [
        (0xC, min_x),
        (0xE, min_y),
        (0x10, min_z),
        (0x12, max_x),
        (0x14, max_y),
        (0x16, max_z),
    ] {
        wr::<i16>(b, off, ftol32(x(c).fmul_m(8.0)) as i16);
    }
    AZ_NUM_BOXES.set(n.wrapping_add(1));
}

// ---------------------------------------------------------------------------------------------------
// CAEAudioEnvironment
// ---------------------------------------------------------------------------------------------------

/// `gSoundDistAttenuationTable` (0x8AC270, f32 per 0.1 m)
const DIST_ATTENUATION_TABLE: u32 = 0x8AC270;
/// `gAudioZoneToReverbEnvironmentMap` (0x8AD670, 8 bytes per zone id: env u8 +0, depth i32 +4)
const REVERB_ENV_MAP: u32 = 0x8AD670;
/// `CWeather::UnderWaterness` (owned by the weather)
const UNDER_WATERNESS: usize = 0xC8132C;
/// `CGame::currArea`
const CURR_AREA: usize = 0xB72914;
/// The directional mike's cone: `cos` of the front limit (0x8AC258), of the rear limit (0x8AC25C) and the
/// rear attenuation in dB (0x8AC260) (game data, read at the original addresses)
const MIKE_FRONT: usize = 0x8AC258;
const MIKE_REAR: usize = 0x8AC25C;
const MIKE_REAR_ATTENUATION: usize = 0x8AC260;

orig! {
    /// `CVector::Normalise()` (0x59C910)
    pub fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CCamera::Get_Just_Switched_Status()` (0x50AE10)
    pub fn camera_just_switched(cam: GPtr<u8>) -> u8 = thiscall 0x50AE10 => unported;
    /// `CAEAudioEnvironment::GetPositionRelativeToCamera(CVector* out, const CVector* pos)` (0x4D80B0)
    pub fn position_relative_to_camera_vec(out: GPtr<u8>, pos: GPtr<u8>) -> u32 = cdecl 0x4D80B0 => unported;
}

/// `TheCamera`
const THE_CAMERA: usize = 0xB6F028;
/// Doppler limits (0x85F06C -35, 0x85F070 35) and the speed of sound (0x85F074, 340)
const DOPPLER_MIN: f32 = -35.0;
const DOPPLER_MAX: f32 = 35.0;
const SPEED_OF_SOUND: f32 = 340.0;

/// Pure body of `GetDopplerRelativeFrequency` after the camera check (`just_switched` = its result):
/// 1.0 on a camera switch, a zero scale, equal distances or a non-increasing time; else the approach
/// speed `r = (new - old) * 1000 / (t_new - t_old) * scale` (m/s; |r| >= 340 gives 1.0), clamped to
/// [-35, 35], and `340 / (r + 340)`.
pub fn doppler_relative_frequency_with(
    old_dist: f32,
    new_dist: f32,
    old_time: u32,
    new_time: u32,
    scale: f32,
    just_switched: impl FnOnce() -> u8,
) -> X87 {
    let d = x(new_dist).fsub_m(old_dist).to_f32();
    let one = x(1.0);
    if just_switched() != 0 {
        return one;
    }
    // fcomp 0.0; test ah, 44h; jnp -> 1.0 (equal)
    if !x(scale).fcom(x(0.0)).test_parity(0x44) || !x(d).fcom(x(0.0)).test_parity(0x44) {
        return one;
    }
    if new_time <= old_time {
        return one;
    }
    // fmul qword 1000.0 (exact in f32 too)
    let mut r = x(d)
        .fmul_m(1000.0)
        .fdiv(fild_u32(new_time.wrapping_sub(old_time)))
        .fmul_m(scale);
    // fld st0; fabs; fcomp 340; test ah, 1; jne -> on (|r| < 340 or unordered)
    if !X87::from_f64_exact(r.to_f64().abs()).fcom(x(SPEED_OF_SOUND)).c0() {
        return one;
    }
    if !r.fcom(x(0.0)).c0() {
        // fld 35; fcomp st1; test ah, 5; jp -> keep (35 >= r or unordered)
        if !x(DOPPLER_MAX).fcom(r).test_parity(0x05) {
            r = x(DOPPLER_MAX);
        }
    } else {
        // fld -35; fcomp st1; test ah, 41h; jne -> keep (-35 <= r or unordered)
        if !x(DOPPLER_MIN).fcom(r).test_any(0x41) {
            r = x(DOPPLER_MIN);
        }
    }
    r.fadd_m(SPEED_OF_SOUND).fdivr_m(SPEED_OF_SOUND)
}

/// `CAEAudioEnvironment::GetDopplerRelativeFrequency(float oldDist, float newDist, uint32 oldTime,
/// uint32 newTime, float scale)` (0x4D7E40, ST0).
#[hook(addr = 0x4D7E40, conv = cdecl, name = "CAEAudioEnvironment::GetDopplerRelativeFrequency", expect = "D9 44 24 08 B9 28 F0 B6 00")]
pub fn get_doppler_relative_frequency(
    old_dist: f32,
    new_dist: f32,
    old_time: u32,
    new_time: u32,
    scale: f32,
) -> X87 {
    doppler_relative_frequency_with(old_dist, new_dist, old_time, new_time, scale, || {
        camera_just_switched(g(THE_CAMERA))
    })
}

/// `CAEAudioEnvironment::GetPositionRelativeToCamera(CVector* out, CPlaceable* placeable)` (0x4D8340):
/// the vector overload (tail jump, its EAX passed through) with the placeable's matrix position
/// (`[+0x14] + 0x30`) or, without a matrix, its placement position (+4). The original rewrites its own
/// argument slot with that pointer before the jump.
#[hook(addr = 0x4D8340, conv = cdecl, name = "CAEAudioEnvironment::GetPositionRelativeToCamera-placeable", expect = "8B 4C 24 08 8B 41 14")]
pub fn get_position_relative_to_camera_placeable(out: GPtr<u8>, placeable: GPtr<u8>) -> u32 {
    let m = rd::<GPtr<u8>>(placeable, 0x14);
    let pos = if m.is_null() {
        placeable.byte_add(4)
    } else {
        m.byte_add(0x30)
    };
    position_relative_to_camera_vec(out, pos)
}

/// Pure body of `GetDistanceAttenuation` with the table read through `table(i)` (`i` = the `_ftol`
/// result; the original reads `[0x8AC270 + i*4]` with 32-bit wrapping): -100 dB from 128 m on, else
/// `table(_ftol(floor(dist * 10)))` (a NaN distance takes the table path).
pub fn distance_attenuation_with(dist: f32, table: impl Fn(i32) -> f32) -> X87 {
    if !x(dist).fcom(x(128.0)).c0() {
        return x(-100.0);
    }
    let d = x(dist).fmul_m(10.0).to_f64(); // fstp qword: exact
    let i = ftol32(X87::from_f64_exact(floor_f64(d)));
    x(table(i))
}

/// `CAEAudioEnvironment::GetDistanceAttenuation(float dist)` (0x4D7F20, ST0).
#[hook(addr = 0x4D7F20, conv = cdecl, name = "CAEAudioEnvironment::GetDistanceAttenuation", expect = "D9 44 24 04 D8 1D F4 8B 85 00")]
pub fn get_distance_attenuation(dist: f32) -> X87 {
    distance_attenuation_with(dist, |i| {
        g::<f32>(DIST_ATTENUATION_TABLE.wrapping_add((i as u32).wrapping_mul(4)) as usize).read()
    })
}

/// Pure body of `GetDirectionalMikeAttenuation` after the normalisation: `v = (z + x) * 0 + y` of the
/// normalised direction (stored as f32), then 0 dB when the front limit is -1 or `v` is not below it;
/// the rear attenuation when `v <= rear`; otherwise `(front - v) / (front - rear) * rear_attenuation`.
pub fn directional_mike_attenuation_with(n: [f32; 3], front: f32, rear: f32, rear_att: f32) -> X87 {
    let v = x(n[2]).fadd_m(n[0]).fmul_m(0.0).fadd_m(n[1]).to_f32();
    // fcomp -1.0; test ah, 44h; jnp -> 0 (front == -1)
    if !x(front).fcom(x(-1.0)).test_parity(0x44) {
        return x(0.0);
    }
    // fcomp front; test ah, 1; jne -> on (v < front or unordered)
    if !x(v).fcom(x(front)).c0() {
        return x(0.0);
    }
    // fcomp rear; test ah, 41h; jp -> interpolate (v > rear or unordered)
    if !x(v).fcom(x(rear)).test_parity(0x41) {
        return x(rear_att);
    }
    let a = x(front).fsub_m(v);
    let b = x(front).fsub_m(rear);
    a.fdiv(b).fmul_m(rear_att)
}

/// `CAEAudioEnvironment::GetDirectionalMikeAttenuation(const CVector& dir)` (0x4D7F60, ST0): `dir`
/// copied to a local and normalised with `CVector::Normalise`.
#[hook(addr = 0x4D7F60, conv = cdecl, name = "CAEAudioEnvironment::GetDirectionalMikeAttenuation", expect = "83 EC 0C 8B 44 24 10")]
pub fn get_directional_mike_attenuation(dir: GPtr<u8>) -> X87 {
    let mut v = [rd::<u32>(dir, 0), rd::<u32>(dir, 4), rd::<u32>(dir, 8)];
    vector_normalise(GPtr::from_raw(v.as_mut_ptr()).cast());
    let n = v.map(f32::from_bits);
    directional_mike_attenuation_with(
        n,
        g::<f32>(MIKE_FRONT).read(),
        g::<f32>(MIKE_REAR).read(),
        g::<f32>(MIKE_REAR_ATTENUATION).read(),
    )
}

/// `CAEAudioEnvironment::GetReverbEnvironmentAndDepth(int8* env, int32* depth)` (0x4D8010): under water
/// (`UnderWaterness` not below 0.5; NaN counts as below) 22 / -12; otherwise the id of the first active audio
/// sphere, else of the first active box, mapped through the reverb table; with no zone (or id -1)
/// zone id 14 inside an interior, else 23 / -100.
#[hook(addr = 0x4D8010, conv = cdecl, name = "CAEAudioEnvironment::GetReverbEnvironmentAndDepth", expect = "D9 05 2C 13 C8 00")]
pub fn get_reverb_environment_and_depth(env: GPtr<u8>, depth: GPtr<i32>) {
    if !x(g::<f32>(UNDER_WATERNESS).read()).fcom(x(0.5)).c0() {
        env.write(0x16);
        depth.write(-12);
        return;
    }
    let mut id: i16 = -1;
    if (AZ_NUM_ACTIVE_SPHERES.get() as i32) > 0 {
        let k = AZ_ACTIVE_SPHERES.get()[0] as isize;
        id = rd::<i16>(AZ_SPHERES.ptr().cast::<u8>(), k.wrapping_mul(0x1C) + 8);
    } else if (AZ_NUM_ACTIVE_BOXES.get() as i32) > 0 {
        let k = AZ_ACTIVE_BOXES.get()[0] as isize;
        id = rd::<i16>(AZ_BOXES.ptr().cast::<u8>(), k.wrapping_mul(0x18) + 8);
    }
    if id == -1 {
        if g::<i32>(CURR_AREA).read() == 0 {
            env.write(0x17);
            depth.write(-100);
            return;
        }
        id = 14;
    }
    let e = REVERB_ENV_MAP.wrapping_add((i32::from(id) as u32).wrapping_mul(8)) as usize;
    env.write(g::<u8>(e).read());
    depth.write(g::<i32>(e + 4).read());
}

/// `sqrt((dz*dz + dy*dy) + dx*dx)` of `p - q` as the original's inline `DistanceBetweenPoints`
/// (operands loaded from memory: `p` the arguments, `q` the stored vector).
fn distance(p: [f32; 3], q: [f32; 3]) -> X87 {
    let dx = x(p[0]).fsub_m(q[0]);
    let dy = x(p[1]).fsub_m(q[1]);
    let dz = x(p[2]).fsub_m(q[2]);
    dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt()
}

/// `min * 0.125 < p < max * 0.125` on one axis of a box (int16 coordinates in 1/8 m).
fn box_axis_contains(min: i16, max: i16, p: f32) -> bool {
    xi(i32::from(min)).fmul_m(0.125).fcom(x(p)) == sa_math::x87::Cmp::Less
        && xi(i32::from(max)).fmul_m(0.125).fcom(x(p)) == sa_math::x87::Cmp::Greater
}

/// `CAudioZones::Update(bool force, CVector pos)` (0x5083C0; the frame counter read goes through the
/// protection at 0x403A48, which decodes `CTimer::m_FrameCounter` 0xB7CB4C): unless forced, every 16th
/// frame (counter & 15 == 6) or 20 m away from the last update position, nothing. Otherwise the position
/// is stored and the first 10 active spheres containing it (distance < radius) and active boxes
/// containing it (strictly inside on every axis) are listed.
#[hook(addr = 0x5083C0, conv = cdecl, name = "CAudioZones::Update", expect = "8A 44 24 04 84 C0")]
pub fn audio_zones_update(force: u32, px: u32, py: u32, pz: u32) {
    let p = [f32::from_bits(px), f32::from_bits(py), f32::from_bits(pz)];
    if force as u8 == 0 && sa_core::timer::FRAME_COUNTER.get() & 0xF != 6 {
        let last = AZ_LAST_UPDATE_POS.get().map(f32::from_bits);
        if distance(p, last).fcom(x(20.0)) == sa_math::x87::Cmp::Less {
            return;
        }
    }
    AZ_LAST_UPDATE_POS.set([px, py, pz]);
    AZ_NUM_ACTIVE_SPHERES.set(0);
    AZ_NUM_ACTIVE_BOXES.set(0);
    let n = AZ_NUM_SPHERES.get() as i32;
    let mut count = 0usize;
    let spheres = AZ_SPHERES.ptr().cast::<u8>();
    for i in 0..n.max(0) {
        let s = spheres.byte_add(i as isize * 0x1C);
        if rd::<u8>(s, 0xA) & 1 == 0 {
            continue;
        }
        let c = [rd::<f32>(s, 0xC), rd::<f32>(s, 0x10), rd::<f32>(s, 0x14)];
        if distance(p, c).fcom(x(rd(s, 0x18))) == sa_math::x87::Cmp::Less && count < 10 {
            AZ_ACTIVE_SPHERES
                .ptr()
                .cast::<i32>()
                .byte_add(count as isize * 4)
                .write(i);
            count += 1;
        }
    }
    if n > 0 {
        AZ_NUM_ACTIVE_SPHERES.set(count as u32);
    }
    let n = AZ_NUM_BOXES.get() as i32;
    let mut count = 0usize;
    let boxes = AZ_BOXES.ptr().cast::<u8>();
    for i in 0..n.max(0) {
        let b = boxes.byte_add(i as isize * 0x18);
        if rd::<u8>(b, 0xA) & 1 == 0 {
            continue;
        }
        let inside =
            (0..3).all(|k| box_axis_contains(rd(b, 0xC + 2 * k), rd(b, 0x12 + 2 * k), p[k as usize]));
        if inside && count < 10 {
            AZ_ACTIVE_BOXES
                .ptr()
                .cast::<i32>()
                .byte_add(count as isize * 4)
                .write(i);
            count += 1;
        }
    }
    if n > 0 {
        AZ_NUM_ACTIVE_BOXES.set(count as u32);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_GET_RANDOM_NUMBER_IN_RANGE_INT,
    HOOK_GET_RANDOM_NUMBER_IN_RANGE_FLOAT,
    HOOK_RESOLVE_PROBABILITY,
    HOOK_GET_PIECEWISE_LINEAR,
    HOOK_CONVERT_FROM_BYTES_TO_MS,
    HOOK_AUDIO_LOG10,
    HOOK_CONVERT_FROM_MS_TO_BYTES,
    HOOK_CHANNEL_CONVERT_FROM_BYTES_TO_MS,
    HOOK_CHANNEL_CONVERT_FROM_MS_TO_BYTES,
    HOOK_AUDIO_CHANNEL_CTOR,
    HOOK_AUDIO_ZONES_INIT,
    HOOK_REGISTER_AUDIO_SPHERE,
    HOOK_REGISTER_AUDIO_BOX,
    HOOK_GET_DISTANCE_ATTENUATION,
    HOOK_GET_DIRECTIONAL_MIKE_ATTENUATION,
    HOOK_GET_REVERB_ENVIRONMENT_AND_DEPTH,
    HOOK_GET_DOPPLER_RELATIVE_FREQUENCY,
    HOOK_GET_POSITION_RELATIVE_TO_CAMERA_PLACEABLE,
    HOOK_AUDIO_ZONES_UPDATE,
];

/// [`get_random_number_in_range_float`] as the f32 a caller's `fstp dword` / next PC=24 op sees (the
/// `orig!` phase-B target of callers in other modules).
pub fn random_float_f32(min: f32, max: f32) -> f32 {
    get_random_number_in_range_float(min, max).to_f32()
}

/// [`get_distance_attenuation`] as an f32 (the `orig!` phase-B target of callers in other modules).
pub fn distance_attenuation_f32(dist: f32) -> f32 {
    get_distance_attenuation(dist).to_f32()
}

/// [`get_directional_mike_attenuation`] as an f32 (the `orig!` phase-B target of callers in other modules).
pub fn directional_mike_attenuation_f32(dir: GPtr<u8>) -> f32 {
    get_directional_mike_attenuation(dir).to_f32()
}

/// This module's phase B natives (STD-8).
pub const NATIVES: &[sa_core::abi::NativeReg] =
    &[NATIVE_FIND_VEHICLE_OF_PLAYER, crate::ped::NATIVE_SPEECH_IS_GLOBAL_CONTEXT_PAIN,
        crate::ped::NATIVE_SPEECH_IS_GLOBAL_CONTEXT_IMPORTANT_FOR_INTERUPTING];
