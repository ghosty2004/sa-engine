//! `CAEGlobalWeaponAudioEntity::ServiceAmbientGunFire` (batch `std7-audio-1`): the ambient gun fire of
//! Los Santos, the San Fierro fog horn and the Las Venturas waterfall. Ported from the 1.0 US bytes;
//! unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::{GPtr, X87, audio_hardware, g, rd, sound_manager, wr, x};

/// Function statics: gun shots left, weapon type, delay, state, last time and its guard word.
const GUN_SHOTS: usize = 0xB61318;
const WEAPON_TYPE: usize = 0xB6131C;
const DELAY: usize = 0xB61320;
const STATE: usize = 0xB61324;
const LAST_TIME: usize = 0xB61364;
const GUARD: usize = 0xB61368;
/// The three waterfall sounds (0xB612F0..0xB612F8) and the fog horn sound (0xB612FC).
const WATERFALL: usize = 0xB612F0;
const FOG_HORN: usize = 0xB612FC;
/// Static positions: fog horn 0 / 1, waterfall 0 / 1 / 2 (each initialised once under `GUARD`).
const FOG_HORN_0: usize = 0xB61358;
const FOG_HORN_1: usize = 0xB6134C;
const WATERFALL_0: usize = 0xB61340;
const WATERFALL_1: usize = 0xB61334;
const WATERFALL_2: usize = 0xB61328;
/// `CTimer::m_snTimeInMilliseconds`, `CTimer::m_FrameCounter`, `CWeather::WeatherRegion`,
/// `CWeather::Foggyness_SF`
const TIME_MS: usize = 0xB7CB84;
const FRAME_COUNTER: usize = 0xB7CB4C;
const WEATHER_REGION: usize = 0xC81314;
const FOGGYNESS_SF: usize = 0xC812FC;
/// `TheCamera` matrix pointer (+0x14) and placement position (+4)
const CAMERA_MATRIX: usize = 0xB6F03C;
const CAMERA_PLACEMENT_POS: usize = 0xB6F02C;

orig! {
    /// `CPlaceable::SetPosn(const CVector&)` (0x4241C0)
    fn placeable_set_posn(e: GPtr<u8>, pos: GPtr<u8>) = thiscall 0x4241C0 => unported;
    /// `CGeneral::GetRandomNumberInRange(float, float)` (0x41BD90, ST0)
    fn random_float(min: f32, max: f32) -> f64 = cdecl 0x41BD90 => unported;
    /// `CGeneral::GetRandomNumberInRange(int, int)` (0x407180)
    fn random_int(min: i32, max: i32) -> i32 = cdecl 0x407180 => unported;
    /// `std::min<float>(a, b)` by value (0x404330, ST0)
    fn min_float(a: u32, b: u32) -> f64 = cdecl 0x404330 => unported;
    /// `CAEGlobalWeaponAudioEntity::PickAmbientGunFire(int32* shots, eWeaponType*, uint32* delay)` (0x4DEFE0)
    fn pick_ambient_gun_fire(this: GPtr<u8>, shots: GPtr<u8>, weapon: GPtr<u8>, delay: GPtr<u8>) = thiscall 0x4DEFE0 => unported;
    /// `VectorSub(CVector* out, const CVector* a, const CVector* b)` (0x40FE60, returns out)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CVector::MagnitudeSqr()` (0x406DA0, ST0), `CVector::Magnitude()` (0x4082C0, ST0)
    fn vector_magnitude_sqr(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CAEAudioHardware::IsSoundBankLoaded(bank, slot)` (0x4D88C0)
    fn hw_is_sound_bank_loaded(hw: GPtr<u8>, bank: u32, slot: u32) -> u8 = thiscall 0x4D88C0 => unported;
    /// `CAESound::Initialise(...)` (0x4EFE50, `ret 0x38`)
    fn sound_initialise(sound: GPtr<u8>, slot: u32, sfx: u32, entity: GPtr<u8>, px: u32, py: u32, pz: u32, vol: u32, max_dist: u32, speed: u32, doppler: u32, delay: u32, flags: u32, freq_var: u32, play_time: u32) = thiscall 0x4EFE50 => unported;
    /// `CAESoundManager::RequestNewSound(CAESound*)` (0x4EFB10)
    fn sm_request_new_sound(sm: GPtr<u8>, sound: GPtr<u8>) -> u32 = thiscall 0x4EFB10 => unported;
    /// `CAESound::StopSound()` (0x4EF1C0), `CAESound::~CAESound()` (0x4EF660)
    fn sound_stop_sound(sound: GPtr<u8>) = thiscall 0x4EF1C0 => crate::sound::stop_sound;
    fn sound_dtor(sound: GPtr<u8>) = thiscall 0x4EF660 => unported;
    /// `FindPlayerCoors(CVector* out, int32 player)` (0x56E010)
    fn find_player_coors(out: GPtr<u8>, id: i32) -> GPtr<u8> = cdecl 0x56E010 => unported;
    /// `CAudioEngine::ReportWeaponEvent(event, weaponType, CPhysical*)` (0x506F40)
    fn report_weapon_event(engine: GPtr<u8>, event: u32, weapon: u32, physical: u32) = thiscall 0x506F40 => crate::engine::report_weapon_event;
    /// `CAEAudioUtility::GetRandomNumberInRange<int32>(min, max)` (0x4D9C10)
    fn ae_random_int(min: i32, max: i32) -> i32 = cdecl 0x4D9C10 => crate::utility::get_random_number_in_range_int;
}

fn r32(a: usize) -> u32 {
    g::<u32>(a).read()
}

fn w32(a: usize, v: u32) {
    g::<u32>(a).write(v);
}

fn gp(a: usize) -> GPtr<u8> {
    GPtr::from_addr(a)
}

/// The position an entity's coordinates live at: its matrix's +0x30 when it has one (+0x14), else +4.
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `TheCamera`'s position (matrix +0x30, or the placement position without a matrix).
fn camera_pos() -> GPtr<u8> {
    let m = r32(CAMERA_MATRIX);
    if m == 0 { gp(CAMERA_PLACEMENT_POS) } else { gp(m as usize + 0x30) }
}

/// `Initialise(slot, sfx, this, *pos, volume, 1, 1, 1, 0, 0, 0, 0)`, +0x1C = `s1c`, +0x18 = `s18`, word +0x56
/// = 4, `RequestNewSound`.
#[allow(clippy::too_many_arguments)]
fn play(sound: GPtr<u8>, this: GPtr<u8>, slot: u32, sfx: u32, pos: usize, vol: u32, s1c: u32, s18: u32) -> u32 {
    sound_initialise(sound, slot, sfx, this, r32(pos), r32(pos + 4), r32(pos + 8), vol, 0x3F80_0000, 0x3F80_0000, 0x3F80_0000, 0, 0, 0, 0);
    wr::<u32>(sound, 0x1C, s1c);
    wr::<u32>(sound, 0x18, s18);
    wr::<u16>(sound, 0x56, 4);
    sm_request_new_sound(sound_manager(), sound)
}

/// `CAEGlobalWeaponAudioEntity::ServiceAmbientGunFire()` (0x4DF210, thiscall). A stack `CAESound` (+8
/// cleared, destroyed on every path). `now` is the time read at entry; the last time (0xB61364) is set to
/// `now` once under bit 0 of the guard 0xB61368, and the five static positions under bits 1..5. Then by
/// state (0xB61324):
///
/// - 0: weather region 1 (LA): every 1024th frame, the physical (+0xA0) is put at the camera, then moved
///   by `(cos a * r, sin a * r)` (a = random 0..2pi stored f32, r = random 40..80 kept in ST0; each sum
///   rounded to f32), its z = `min(z, 20)`; state 1; `PickAmbientGunFire`. Region 3 (LV): within
///   `[0x8AE5DC]` (squared) of waterfall 1, bank 0x27 in slot 2 loaded and no waterfall sound: three
///   sounds (slot 2, sfx 3) at waterfall 0 / 1 / 2 with +0x1C = `[0x8AE5D4]`, `[0x8AE5CC] + [0x8AE5D4]`,
///   `[0x8AE5D4] - [0x8AE5CC]` (f32) and +0x18 = `[0x8AE5D0]`; state 5. Region 4: nothing. Else: with
///   `Foggyness_SF >= [0x858B8C]`, every 1024th frame, within `[0x8AE5EC]` (squared) of fog horn 0, bank 0x3B
///   in slot 0 loaded and no fog horn: the fog horn (slot 0x11, sfx 0xD, +0x1C `[0x8AE5E4]`, +0x18
///   `[0x8AE5E0]`); last time = the current time; state 2.
/// - 1: once `last + delay <= now`: d = player coors - the physical's position (f32); when |d| <=
///   `[0x85F114]` (not NaN): state 0, last time = current time. Else the physical's byte +0x36 = (b & !1) |
///   6, `ReportWeaponEvent(0x91, weapon, physical)`; when the shot count drops to 0: a random 0..100 below
///   40 picks a new burst (`PickAmbientGunFire`), else state 0; else weapon 0x16 → delay = random
///   500..1200, 0x1E → 90..450; last time = current time.
/// - 2: after `[0x8AE5C8]` ms the fog horn stops (state 3); else after `[0x8AE5BC]` ms its volume (+0x14)
///   drops by `[0x8AE5B8]`.
/// - 3: after `[0x8AE5C4]` ms, with bank 0x3B loaded and no fog horn: fog horn 1 (+0x1C `[0x8AE5C0]`);
///   last time = current time; state 4.
/// - 4: after `[0x8AE5C8]` ms the fog horn stops, state 0; else as state 2's fade.
/// - 5: outside LV, or farther than `[0x8AE5DC]` (squared, x87 sum; NaN keeps them) from waterfall 1: the three
///   waterfall sounds stop, state 0.
///
/// The stack sound's bytes `Initialise` does not write are stack leftovers in the original (zero here).
#[hook(addr = 0x4DF210, conv = thiscall, name = "CAEGlobalWeaponAudioEntity::ServiceAmbientGunFire", expect = "6A FF 64 A1 00 00 00 00",
       route = "L2", hazards = "uninit-locals,x87-transcendental", recs = "grove-walk")]
pub fn service_ambient_gun_fire(this: GPtr<u8>) {
    let now = r32(TIME_MS);
    let mut guard = r32(GUARD);
    let last = if guard & 1 == 0 {
        guard |= 1;
        w32(GUARD, guard);
        w32(LAST_TIME, now);
        now
    } else {
        r32(LAST_TIME)
    };
    let mut buf = [0u32; 0x74 / 4];
    let sound = GPtr::from_raw(buf.as_mut_ptr().cast::<u8>());
    wr::<u32>(sound, 8, 0);
    let inits: [(u32, usize, [u32; 3]); 5] = [
        (2, FOG_HORN_0, [0xC527B000, 0x44B4A000, 0x42140000]),
        (4, FOG_HORN_1, [0xC527B000, 0x44E18000, 0x428A0000]),
        (8, WATERFALL_0, [0x4501B4CD, 0x44F0BCCD, 0x41500000]),
        (0x10, WATERFALL_1, [0x45025CCD, 0x44EE3000, 0x4134CCCD]),
        (0x20, WATERFALL_2, [0x4502B800, 0x44EC1CCD, 0x4131999A]),
    ];
    for (bit, at, v) in inits {
        if guard & bit == 0 {
            guard |= bit;
            w32(GUARD, guard);
            w32(at, v[0]);
            w32(at + 4, v[1]);
            w32(at + 8, v[2]);
        }
    }
    let physical = || rd::<GPtr<u8>>(this, 0xA0);
    match r32(STATE) {
        0 => {
            let region = g::<u16>(WEATHER_REGION).read();
            if region == 1 {
                if r32(FRAME_COUNTER) & 0x3FF == 0 {
                    placeable_set_posn(physical(), camera_pos());
                    let a = X87::from_f64_exact(random_float(0.0, f32::from_bits(0x40C90FDB))).to_f32();
                    let r = X87::from_f64_exact(random_float(40.0, 80.0));
                    let p = entity_pos(physical());
                    wr::<f32>(p, 0, x(a).fcos().fmul(r).fadd_m(rd::<f32>(p, 0)).to_f32());
                    let p = entity_pos(physical());
                    wr::<f32>(p, 4, x(a).fsin().fmul(r).fadd_m(rd::<f32>(p, 4)).to_f32());
                    let src = entity_pos(physical());
                    let dst = entity_pos(physical());
                    let z = min_float(rd::<u32>(src, 8), 0x41A0_0000);
                    wr::<f32>(dst, 8, X87::from_f64_exact(z).to_f32());
                    w32(STATE, 1);
                    pick_ambient_gun_fire(this, gp(GUN_SHOTS), gp(WEAPON_TYPE), gp(DELAY));
                }
            } else if region == 3 {
                let mut tmp = [0u32; 3];
                let d = vector_sub(GPtr::from_raw(tmp.as_mut_ptr().cast::<u8>()), camera_pos(), gp(WATERFALL_1));
                let m = X87::from_f64_exact(vector_magnitude_sqr(d));
                if !m.fcom(x(f32::from_bits(r32(0x8AE5DC)))).test_parity(5)
                    && hw_is_sound_bank_loaded(audio_hardware(), 0x27, 2) != 0
                    && r32(WATERFALL) == 0
                    && r32(WATERFALL + 4) == 0
                    && r32(WATERFALL + 8) == 0
                {
                    let vol = r32(0x8AE5D8);
                    let (base, var, s18) = (r32(0x8AE5D4), r32(0x8AE5CC), r32(0x8AE5D0));
                    let s = play(sound, this, 2, 3, WATERFALL_0, vol, base, s18);
                    w32(WATERFALL, s);
                    let up = x(f32::from_bits(var)).fadd_m(f32::from_bits(base)).to_f32().to_bits();
                    let s = play(sound, this, 2, 3, WATERFALL_1, vol, up, s18);
                    w32(WATERFALL + 4, s);
                    let down = x(f32::from_bits(base)).fsub_m(f32::from_bits(var)).to_f32().to_bits();
                    let s = play(sound, this, 2, 3, WATERFALL_2, vol, down, s18);
                    w32(WATERFALL + 8, s);
                    w32(STATE, 5);
                }
            } else if region != 4 {
                let fog = x(g::<f32>(FOGGYNESS_SF).read());
                if !fog.fcom(x(f32::from_bits(r32(0x858B8C)))).test_any(1) && r32(FRAME_COUNTER) & 0x3FF == 0 {
                    let mut tmp = [0u32; 3];
                    let d = vector_sub(GPtr::from_raw(tmp.as_mut_ptr().cast::<u8>()), camera_pos(), gp(FOG_HORN_0));
                    let m = X87::from_f64_exact(vector_magnitude_sqr(d));
                    if !m.fcom(x(f32::from_bits(r32(0x8AE5EC)))).test_parity(5)
                        && hw_is_sound_bank_loaded(audio_hardware(), 0x3B, 0) != 0
                        && r32(FOG_HORN) == 0
                    {
                        let s = play(sound, this, 0x11, 0xD, FOG_HORN_0, r32(0x8AE5E8), r32(0x8AE5E4), r32(0x8AE5E0));
                        w32(FOG_HORN, s);
                        w32(LAST_TIME, r32(TIME_MS));
                        w32(STATE, 2);
                    }
                }
            }
        }
        1 => {
            if r32(DELAY).wrapping_add(last) <= now {
                let mut tmp = [0u32; 3];
                let pc = find_player_coors(GPtr::from_raw(tmp.as_mut_ptr().cast::<u8>()), -1);
                let mut v = [rd::<f32>(pc, 0), rd::<f32>(pc, 4), rd::<f32>(pc, 8)];
                let ph = physical();
                let p = entity_pos(ph);
                v[0] = x(v[0]).fsub_m(rd::<f32>(p, 0)).to_f32();
                v[1] = x(v[1]).fsub_m(rd::<f32>(p, 4)).to_f32();
                v[2] = x(v[2]).fsub_m(rd::<f32>(p, 8)).to_f32();
                let m = X87::from_f64_exact(vector_magnitude(GPtr::from_raw(v.as_mut_ptr().cast::<u8>())));
                let reset = |state: bool| {
                    if state {
                        w32(STATE, 0);
                    }
                    w32(LAST_TIME, r32(TIME_MS));
                };
                if !m.fcom(x(f32::from_bits(r32(0x85F114)))).test_parity(0x41) {
                    reset(true);
                } else {
                    wr::<u8>(ph, 0x36, (rd::<u8>(ph, 0x36) & 0xFE) | 6);
                    report_weapon_event(GPtr::from_addr(crate::common::AUDIO_ENGINE), 0x91, r32(WEAPON_TYPE), physical().addr() as u32);
                    let shots = r32(GUN_SHOTS).wrapping_sub(1);
                    w32(GUN_SHOTS, shots);
                    if shots == 0 {
                        if random_int(0, 100) >= 40 {
                            reset(true);
                        } else {
                            pick_ambient_gun_fire(this, gp(GUN_SHOTS), gp(WEAPON_TYPE), gp(DELAY));
                            reset(false);
                        }
                    } else {
                        match r32(WEAPON_TYPE) {
                            0x16 => w32(DELAY, ae_random_int(500, 1200) as u32),
                            0x1E => w32(DELAY, ae_random_int(90, 450) as u32),
                            _ => {}
                        }
                        reset(false);
                    }
                }
            }
        }
        2 => {
            if now > r32(0x8AE5C8).wrapping_add(last) {
                let s = r32(FOG_HORN);
                if s != 0 {
                    sound_stop_sound(gp(s as usize));
                }
                w32(STATE, 3);
            } else {
                fog_horn_fade(now, last);
            }
        }
        3 => {
            if now > r32(0x8AE5C4).wrapping_add(last)
                && hw_is_sound_bank_loaded(audio_hardware(), 0x3B, 0) != 0
                && r32(FOG_HORN) == 0
            {
                let s = play(sound, this, 0x11, 0xD, FOG_HORN_1, r32(0x8AE5E8), r32(0x8AE5C0), r32(0x8AE5E0));
                w32(FOG_HORN, s);
                w32(LAST_TIME, r32(TIME_MS));
                w32(STATE, 4);
            }
        }
        4 => {
            if now > r32(0x8AE5C8).wrapping_add(last) {
                let s = r32(FOG_HORN);
                if s != 0 {
                    sound_stop_sound(gp(s as usize));
                }
                w32(STATE, 0);
            } else {
                fog_horn_fade(now, last);
            }
        }
        5 => {
            let c = camera_pos();
            let dz = x(rd::<f32>(c, 8)).fsub_m(g::<f32>(WATERFALL_1 + 8).read());
            let dy = x(rd::<f32>(c, 4)).fsub_m(g::<f32>(WATERFALL_1 + 4).read());
            let dx = x(rd::<f32>(c, 0)).fsub_m(g::<f32>(WATERFALL_1).read());
            let sum = dx.fmul(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz));
            let stop = g::<u16>(WEATHER_REGION).read() != 3
                || !sum.fcom(x(f32::from_bits(r32(0x8AE5DC)))).test_any(0x41);
            if stop {
                for i in 0..3 {
                    let s = r32(WATERFALL + i * 4);
                    if s != 0 {
                        sound_stop_sound(gp(s as usize));
                    }
                }
                w32(STATE, 0);
            }
        }
        _ => {}
    }
    sound_dtor(sound);
}

/// 0x4DF9A4: after `[0x8AE5BC]` ms the fog horn's volume (+0x14) drops by `[0x8AE5B8]` (f32).
fn fog_horn_fade(now: u32, last: u32) {
    if now > r32(0x8AE5BC).wrapping_add(last) {
        let s = r32(FOG_HORN);
        if s != 0 {
            let p = gp(s as usize);
            wr::<f32>(p, 0x14, x(rd::<f32>(p, 0x14)).fsub_m(g::<f32>(0x8AE5B8).read()).to_f32());
        }
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_SERVICE_AMBIENT_GUN_FIRE];
