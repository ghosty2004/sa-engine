//! WRITER-MVP-D (batch `mvpd-render-4`): `CWeather::Update`, the per-frame weather state machine.
//!
//! Ported from the 1.0 US bytes; gta-reversed `Weather.cpp` as reading aid. x87 arithmetic follows the
//! original's instruction order on the `sa_math::x87::X87` model (every f32 store rounds); the direct
//! `call rand` sites run under `orig::at_site` with their original return addresses.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, jp, k, rd, x};

orig! {
    /// `CWeather::UpdateWeatherRegion(CVector*)` (0x72A640)
    fn update_weather_region(pos: u32) = cdecl 0x72A640 => unported;
    /// `CWeather::FindWeatherTypesList()` (0x72A520)
    fn find_weather_types_list() -> GPtr<u8> = cdecl 0x72A520 => unported;
    /// `CCullZones::CamNoRain()` (0x72DDB0, bool in AL)
    fn cam_no_rain() -> u8 = cdecl 0x72DDB0 => unported;
    /// `CCullZones::PlayerNoRain()` (0x72DDC0, bool in AL)
    fn player_no_rain() -> u8 = cdecl 0x72DDC0 => unported;
    /// `CAEWeatherAudioEntity::AddAudioEvent(eAudioEvents)` (0x506800)
    fn weather_audio_add_event(this: GPtr<u8>, ev: u32) = thiscall 0x506800 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn pad_get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::StartShake(int16 time, uint8 freq, uint32)` (0x53F920)
    fn pad_start_shake(this: GPtr<u8>, time: u32, freq: u32, a: u32) = thiscall 0x53F920 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CWeather::AddRain()` (0x72A9A0)
    fn add_rain() = cdecl 0x72A9A0 => crate::mvpd_r2::add_rain;
    /// `CTaskManager::GetSimplestActiveTask()` (0x6819D0)
    fn task_manager_simplest_active(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x6819D0 => unported;
    /// `CTask::GetTaskType()` (vtable slot 0x10)
    fn v_task_type(this: GPtr<u8>) -> u32 = thiscall vslot 0x10 => unported;
    /// `CWeather::UpdateInTunnelness()` (0x72B630)
    fn update_in_tunnelness() = cdecl 0x72B630 => unported;
    /// `CAEWeatherAudioEntity::Service()` (0x5052F0)
    fn weather_audio_service(this: GPtr<u8>) = thiscall 0x5052F0 => unported;
}

const INTERP: usize = 0xC8130C;
const OLD: usize = 0xC81320;
const NEW: usize = 0xC8131C;
const RAIN: usize = 0xC81324;
const TIME_MS: usize = 0xB7CB84;
const FRAME: usize = 0xB7CB4C;
const ONE: u32 = 0x3F80_0000;

const RAINY: &[i16] = &[0x10, 8];
const SUNNY: &[i16] = &[1, 3, 0xE, 5, 0xA, 0x12];
const EXTRA: &[i16] = &[0, 2, 0xD, 6, 0xB, 0x11];
const SUNNY_OR_EXTRA: &[i16] = &[1, 3, 0xE, 5, 0xA, 0x12, 0, 2, 0xD, 6, 0xB, 0x11];
const CLOUDY: &[i16] = &[4, 0xF, 0xC, 7];
const FOGGY: &[i16] = &[9, 0x13];
const HEAT: &[i16] = &[0x12, 0x11, 0xB, 0];

fn interp() -> f32 {
    g::<f32>(INTERP).read()
}

fn one_minus_interp() -> f32 {
    X87::one().fsub_m(interp()).to_f32()
}

/// The common "old → new" store: `g = old ∈ set ? 1 - interp : 0`, then `g += interp` when new ∈ set
/// (two f32 stores, as the original does for CloudCoverage, Foggyness, ExtraSunnyness, SunGlare, ...).
fn lerp_store(addr: usize, old_in: bool, new_in: bool) {
    let v = g::<f32>(addr);
    v.write(if old_in { one_minus_interp() } else { 0.0 });
    if new_in {
        v.write(x(v.read()).fadd_m(interp()).to_f32());
    }
}

/// `notsa::step_to` as compiled: `|target - v| < step` → v = target, else v ∓ step by the sign of the
/// difference (`<= 0` or unordered steps down).
fn step_to(addr: usize, target: f32, step: X87) {
    let v = g::<f32>(addr);
    let diff = x(target).fsub_m(v.read());
    if !jp(diff.fabs(), step, 5) {
        v.write(target);
    } else if diff.fcom(X87::zero()).test_any(0x41) {
        v.write(x(v.read()).fsub(step).to_f32());
    } else {
        v.write(x(v.read()).fadd(step).to_f32());
    }
}

/// `fild dword` of an unsigned value as MSVC does it (plus 2^32 when negative).
fn fild_u32(v: u32) -> X87 {
    let r = X87::from_i32(v as i32);
    if (v as i32) < 0 { r.fadd_m(k(0x4F80_0000)) } else { r }
}

fn rand_at(site: u32) -> i32 {
    sa_core::orig::at_site(site, sa_core::crt::rand)
}

/// `CWeather::Update()` (0x72B850). Every 16th frame `UpdateWeatherRegion(NULL)`. Outside replay
/// playback (0xA43088 != 1) the interpolation becomes `(seconds / 60 + minutes) / 60`; when it wraps,
/// the region is updated, old = new, and new = the forced type (>= 0) or, below camera z 950, the
/// next entry of `FindWeatherTypesList()` (index 0xC81310 + 1 mod 64). Lightning (old and new rainy,
/// no rain cull zone, `[0xC8132C] <= 0`, outside): bursts start on `rand() & 0xFFFF < 200`, stop on
/// `rand() & 0xFF < 24` (duration min(frames, 20), thunder at `now + (20 - d) * 150`), flash toggling
/// on `rand() & 1` after 50 ms; the thunder plays (audio event 0x8D, pad shake `d * 40 + 100` /
/// `(d + 40) * 2`). Then WetRoads, Rain and Sandstorm (stepped by `timeStep * 0.005` towards the
/// rainy/sandstorm blend times `((now >> 13) & 3) * 0.1 + 0.7`), CloudCoverage, Foggyness,
/// Foggyness_SF, ExtraSunnyness, Rainbow, SunGlare (sun height factor and, unless a snapshot is active,
/// `1 - (rand() & 31) * 0.007`), HeatHaze with the heat haze FX fade (0xC81448 by the post-effects
/// hours/speeds 0x8D50D4..0x8D50E0), the water fog FX control from the time-cycle word 0xB7C540, the
/// wind (table 0x8D5E50) and its direction waves (0x8D6038 offsets, 0x8D5FF8 scales), the clipped
/// wind, Rain capped at `1 - [0xC8132C]`, the night factor 0xC812A8, 0xC812D0, `AddRain()`, a dead
/// `FindPlayerPed` call (the removed player comment), `UpdateInTunnelness()` and the weather audio
/// `Service()` (tail call).
#[hook(addr = 0x72B850, conv = cdecl, name = "CWeather::Update", expect = "A0 4C CB B7 00",
       route = "L2", hazards = "x87-transcendental",
       recs = "grove-walk",
       note = "FCOS wind waves; rand sites 0x72B9E0, 0x72BA5A, 0x72BA8D, 0x72C0BE")]
pub fn weather_update() {
    if g::<u8>(FRAME).read() & 0xF == 0 {
        update_weather_region(0);
    }
    if g::<u8>(0xA43088).read() != 1 {
        let sec = X87::from_i32(i32::from(g::<u8>(0xB70150).read()));
        let min = X87::from_i32(i32::from(g::<u8>(0xB70152).read()));
        let fv = sec.fmul_m(k(0x3C88_8889)).fadd(min).fmul_m(k(0x3C88_8889));
        let f = fv.to_f32();
        if jp(fv, x(interp()), 5) {
            g::<f32>(INTERP).write(f);
        } else {
            update_weather_region(0);
            let new = g::<i16>(NEW).read();
            let forced = g::<i16>(0xC81318).read();
            g::<i16>(OLD).write(new);
            if forced >= 0 {
                g::<i16>(NEW).write(forced);
                g::<f32>(INTERP).write(f);
            } else {
                let cm = g::<GPtr<u8>>(0xB6F03C).read();
                let cam = if cm.is_null() { g::<u8>(0xB6F02C) } else { cm.byte_add(0x30) };
                if jp(x(rd::<f32>(cam, 8)), x(k(0x446D_8000)), 5) {
                    g::<f32>(INTERP).write(f);
                } else {
                    let idx = (g::<i32>(0xC81310).read() + 1) % 64;
                    g::<i32>(0xC81310).write(idx);
                    let list = find_weather_types_list();
                    g::<i16>(NEW).write(i16::from(list.byte_add(idx as isize).cast::<i8>().read()));
                    g::<f32>(INTERP).write(f);
                }
            }
        }
    }
    let burst = g::<u8>(0xC812CD);
    let flash = g::<u8>(0xC812CC);
    let when = g::<u32>(0xC812C0);
    let both_rainy = RAINY.contains(&g::<i16>(NEW).read()) && RAINY.contains(&g::<i16>(OLD).read());
    if both_rainy
        && cam_no_rain() == 0
        && player_no_rain() == 0
        && !x(g::<f32>(0xC8132C).read()).fcom(X87::zero()).test_parity(0x41)
        && g::<i32>(0xB72914).read() == 0
    {
        if burst.read() != 0 {
            if rand_at(0x72_B9E0) & 0xFF < 0x18 {
                let diff = g::<u32>(FRAME).read().wrapping_sub(g::<u32>(0xC812C8).read());
                burst.write(0);
                let d = if diff > 0x14 { 0x14 } else { diff };
                g::<u32>(0xC812C4).write(d);
                when.write((0x14 - d).wrapping_mul(0x96).wrapping_add(g::<u32>(TIME_MS).read()));
                flash.write(0);
            } else {
                let now = g::<u32>(TIME_MS).read();
                if now.wrapping_sub(g::<u32>(0xC812BC).read()) > 0x32 {
                    let was = flash.read();
                    let r = rand_at(0x72_BA5A);
                    let nf = (r as u8) & 1;
                    flash.write(nf);
                    if was != nf {
                        g::<u32>(0xC812BC).write(g::<u32>(TIME_MS).read());
                    }
                }
            }
        } else {
            let r = rand_at(0x72_BA8D) & 0xFFFF;
            if r < 0xC8 {
                g::<u32>(0xC812C8).write(g::<u32>(FRAME).read());
                g::<u32>(0xC812BC).write(g::<u32>(TIME_MS).read());
                burst.write(1);
                flash.write(1);
            } else {
                flash.write(0);
            }
        }
    } else {
        flash.write(0);
        burst.write(0);
    }
    let w = when.read();
    if w != 0 && g::<u32>(TIME_MS).read() > w {
        weather_audio_add_event(g(0xC81360), 0x8D);
        let d = g::<u32>(0xC812C4).read();
        let freq = u32::from(((d as u8).wrapping_add(0x28)) << 1);
        pad_start_shake(pad_get_pad(0), d.wrapping_mul(40).wrapping_add(0x64), freq, 0);
        when.write(0);
    }
    let old = g::<i16>(OLD).read();
    let new = g::<i16>(NEW).read();
    let wet = g::<f32>(0xC81308);
    if RAINY.contains(&old) {
        wet.write(if RAINY.contains(&new) { 1.0 } else { one_minus_interp() });
    } else {
        wet.write(if RAINY.contains(&new) { interp() } else { 0.0 });
    }
    let now = g::<u32>(TIME_MS).read();
    let blend = |set: &[i16]| {
        let mut t = X87::zero();
        if set.contains(&new) {
            t = x(interp());
        }
        if set.contains(&old) {
            t = t.fadd(X87::one().fsub_m(interp()));
        }
        t
    };
    let rt = blend(RAINY);
    let inten = fild_u32((now >> 13) & 3).fmul_m(k(0x3DCC_CCCD)).fadd_m(k(0x3F33_3333)).to_f32();
    let rtarget = x(inten).fmul(rt).to_f32();
    let step = x(g::<f32>(0xB7CB5C).read()).fmul_m(k(0x3BA3_D70A));
    step_to(RAIN, rtarget, step);
    let st = blend(&[0x13]);
    let starget = x(inten).fmul(st).to_f32();
    step_to(0xC812F4, starget, step);
    lerp_store(0xC81304, !SUNNY_OR_EXTRA.contains(&old), !SUNNY_OR_EXTRA.contains(&new));
    lerp_store(0xC81300, FOGGY.contains(&old), FOGGY.contains(&new));
    lerp_store(0xC812FC, old == 9, new == 9);
    lerp_store(0xC812F8, EXTRA.contains(&old), EXTRA.contains(&new));
    let hours = g::<u8>(0xB70153).read();
    let rainbow = g::<f32>(0xC812E4);
    if CLOUDY.contains(&old) && SUNNY.contains(&new) && !jp(x(interp()), x(k(0x3F00_0000)), 5) && hours > 6 && hours < 0x15 {
        let v = x(interp()).fsub_m(k(0x3E80_0000));
        let a = if jp(v, X87::zero(), 5) { v } else { v.fchs() };
        rainbow.write(a.fmul_m(k(0x4080_0000)).fsubr_m(k(ONE)).to_f32());
    } else {
        rainbow.write(0.0);
    }
    lerp_store(0xC812E0, SUNNY_OR_EXTRA.contains(&old), SUNNY_OR_EXTRA.contains(&new));
    let glare = g::<f32>(0xC812E0);
    if !x(glare.read()).fcom(X87::zero()).test_any(0x41) {
        let t = g::<i32>(0xB79FD0).read() as isize;
        let mut sz = x(g::<f32>(0xB7CA58).byte_add(t * 12).read()).fmul_m(k(0x40E0_0000));
        if !jp(X87::one(), sz, 5) {
            sz = X87::one();
        }
        glare.write(sz.fmul_m(glare.read()).to_f32());
        if !X87::zero().fcom(x(glare.read())).test_any(0x41) {
            glare.write(0.0);
        } else if !jp(X87::one(), x(glare.read()), 5) {
            glare.write(1.0);
        }
        if g::<u8>(0xC7C714).read() == 0 {
            let r = rand_at(0x72_C0BE) & 0x1F;
            glare.write(X87::from_i32(r).fmul_m(k(0x3BE5_6042)).fsubr_m(k(ONE)).fmul_m(glare.read()).to_f32());
        }
    }
    lerp_store(0xC812DC, HEAT.contains(&old), HEAT.contains(&new));
    let mut speed = g::<u32>(0x8D50DC).read();
    let haze = g::<f32>(0xC812DC);
    if !x(haze.read()).fcom(X87::zero()).test_any(0x41) {
        let minutes = u32::from(g::<u8>(0xB70152).read());
        let hstart = g::<i32>(0x8D50D4).read();
        let hend = g::<i32>(0x8D50D8).read();
        let h = i32::from(hours);
        let changed = minutes != g::<u32>(0x8D6078).read();
        g::<u32>(0xC812D8).write(0);
        let fade_in = h >= hstart && h < hend;
        let fade_out = h >= hend;
        let fade = g::<f32>(0xC81448);
        let outside = g::<i32>(0xB72914).read() == 0
            && rd::<u8>(find_player_ped(-1), 0x2F) == 0
            && cam_no_rain() == 0
            && player_no_rain() == 0;
        let mut ctl: X87;
        let mut do_out = true;
        if outside {
            if fade_in {
                if changed {
                    fade.write(x(fade.read()).fadd_m(f32::from_bits(speed)).to_f32());
                }
                if !x(fade.read()).fcom(X87::one()).test_any(0x41) {
                    fade.write(1.0);
                }
                ctl = x(fade.read());
            } else {
                ctl = x(g::<f32>(0xC812D8).read());
            }
            do_out = fade_out;
        } else {
            speed = g::<u32>(0x8D50E0).read();
            ctl = X87::zero();
        }
        if do_out {
            if changed {
                fade.write(x(fade.read()).fsub_m(f32::from_bits(speed)).to_f32());
            }
            if !jp(x(fade.read()), X87::zero(), 5) {
                fade.write(0.0);
            }
            ctl = x(fade.read());
        }
        g::<f32>(0xC812D8).write(ctl.fmul_m(haze.read()).to_f32());
        g::<u32>(0x8D6078).write(minutes);
    }
    let wf = g::<u16>(0xB7C540).read();
    let v = X87::from_i32(i32::from(wf)).fmul_m(k(0x3C23_D70A));
    let ctl = g::<f32>(0xC81444);
    let active = g::<u8>(0xC81440);
    if wf == 0 {
        ctl.write(0.0);
    }
    if wf >= 0x5F {
        active.write(1);
    }
    if active.read() == 0 {
        if !v.fcom(x(ctl.read())).test_any(0x41) {
            ctl.write(v.to_f32());
        }
    } else {
        if !jp(v, x(ctl.read()), 5) {
            ctl.write(v.to_f32());
        }
        if !x(ctl.read()).fcom(X87::zero()).test_parity(0x41) {
            active.write(0);
        }
    }
    let wfx = g::<f32>(0xC81338);
    let u = x(ctl.read()).fmul_m(k(0x3FB3_3333));
    wfx.write(u.to_f32());
    if !jp(u, X87::zero(), 5) {
        wfx.write(0.0);
    } else if !x(wfx.read()).fcom(X87::one()).test_any(0x41) {
        wfx.write(1.0);
    }
    let wind_t = |i: i16| g::<f32>(0x8D5E50).byte_add(isize::from(i) * 4).read();
    let wind = x(wind_t(old)).fmul(X87::one().fsub_m(interp())).fadd(x(wind_t(new)).fmul_m(interp())).to_f32();
    g::<f32>(0xC812F0).write(wind);
    let wc = g::<f32>(0xC812EC);
    if jp(X87::one(), x(wind), 5) {
        wc.write(wind);
    } else {
        wc.write(1.0);
    }
    let now = g::<u32>(TIME_MS).read();
    let off = |i: u32| g::<f32>(0x8D6038).byte_add((i & 0xF) as isize * 4).read();
    let scl = |i: u32| g::<f32>(0x8D5FF8).byte_add((i & 0xF) as isize * 4).read();
    let wd = x(wc.read()).fmul_m(k(0x3F33_3333)).to_f32();
    g::<f32>(0xC813E0).write(wd);
    g::<f32>(0xC813E4).write(wd);
    let e = (now >> 10) & 0xF;
    let a = fild_u32(now & 0x3FF).fmul_m(k(0x3A80_0000)).fmul_m(k(0x4049_0FDB)).fcos().fmul_m(k(0x3F00_0000)).fsubr_m(k(0x3F00_0000)).to_f32();
    let b = X87::one().fsub_m(a).to_f32();
    let wcv = wc.read();
    let mut wx = x(a).fmul_m(off(e + 1)).fadd(x(b).fmul_m(off(e))).fmul_m(wcv).fmul_m(k(0x3ECC_CCCD)).fadd_m(g::<f32>(0xC813E0).read());
    let mut wy = x(b).fmul_m(off(e + 3)).fadd(x(a).fmul_m(off(e + 4))).fmul_m(wcv).fmul_m(k(0x3ECC_CCCD)).fadd_m(g::<f32>(0xC813E4).read());
    let wz = x(b).fmul_m(off(e + 6)).fadd(x(a).fmul_m(off(e + 7))).fmul_m(wcv).fmul_m(k(0x3E4C_CCCD)).to_f32();
    g::<f32>(0xC813E8).write(wz);
    let gust = x(wcv).fsub_m(k(0x3F00_0000)).fmul_m(k(0x3ECC_CCCD));
    if !gust.fcom(X87::zero()).test_any(0x41) {
        let e2 = (now >> 8) & 0xF;
        let uu = fild_u32(now & 0xFF).fmul_m(k(0x3B80_0000));
        let vv = X87::one().fsub(uu);
        wx = wx.fadd(uu.fmul_m(off(e2 + 1)).fadd(vv.fmul_m(off(e2))).fmul(gust));
        wy = wy.fadd(vv.fmul_m(off(e2 + 3)).fadd(uu.fmul_m(off(e2 + 4))).fmul(gust));
        let gz = vv.fmul_m(off(e2 + 6)).fadd(uu.fmul_m(off(e2 + 7))).fmul(gust).fadd_m(g::<f32>(0xC813E8).read());
        g::<f32>(0xC813E8).write(gz.to_f32());
    }
    let e3 = (now >> 11) & 0xF;
    let a3 = fild_u32(now & 0x7FF).fmul_m(k(0x3A00_0000)).fmul_m(k(0x4049_0FDB)).fcos().fmul_m(k(0x3F00_0000)).fsubr_m(k(0x3F00_0000));
    let m = X87::one().fsub(a3).fmul_m(scl(e3)).fadd(a3.fmul_m(scl(e3 + 1)));
    let mf = m.to_f32();
    g::<f32>(0xC813E0).write(m.fmul(wx).to_f32());
    g::<f32>(0xC813E4).write(x(mf).fmul(wy).to_f32());
    g::<f32>(0xC813E8).write(x(mf).fmul_m(g::<f32>(0xC813E8).read()).to_f32());
    let wc2 = x(wc.read()).fadd_m(k(0x3E99_999A));
    if jp(X87::one(), wc2, 5) {
        g::<f32>(0xC812E8).write(wc2.to_f32());
    } else {
        g::<f32>(0xC812E8).write(1.0);
    }
    let cap = X87::one().fsub_m(g::<f32>(0xC8132C).read());
    if jp(x(g::<f32>(RAIN).read()), cap, 5) {
        g::<f32>(RAIN).write(cap.to_f32());
    }
    let hours = g::<u8>(0xB70153).read();
    let night = g::<f32>(0xC812A8);
    let minf = || X87::from_i32(i32::from(g::<u8>(0xB70152).read())).fmul_m(k(0x3C88_8889));
    if hours > 0x14 || hours <= 5 {
        night.write(1.0);
    } else if hours > 0x13 {
        night.write(minf().to_f32());
    } else if hours > 6 {
        night.write(0.0);
    } else {
        night.write(minf().fsubr_m(k(ONE)).to_f32());
    }
    let fog = || g::<f32>(0xC81300).read();
    let rain = || g::<f32>(RAIN).read();
    let mx = if x(fog()).fcom(x(rain())).test_any(0x41) { rain() } else { fog() };
    if jp(X87::one(), x(mx), 5) {
        let v = if x(fog()).fcom(x(rain())).test_any(0x41) { rain() } else { fog() };
        g::<f32>(0xC812D0).write(v);
    } else {
        g::<f32>(0xC812D0).write(1.0);
    }
    for src in [0xC81308usize, 0xC81300, RAIN] {
        if x(night.read()).fcom(x(g::<f32>(src).read())).test_any(0x41) {
            night.write(g::<f32>(src).read());
        }
    }
    add_rain();
    let mut talking = false;
    let tm = rd::<GPtr<u8>>(find_player_ped(-1), 0x47C).byte_add(4);
    if !task_manager_simplest_active(tm).is_null() {
        let tm = rd::<GPtr<u8>>(find_player_ped(-1), 0x47C).byte_add(4);
        if v_task_type(task_manager_simplest_active(tm)) == 0xFE {
            talking = true;
        }
    }
    if SUNNY_OR_EXTRA.contains(&g::<i16>(NEW).read())
        && g::<i32>(0xB72914).read() == 0
        && g::<u8>(0xB5F851).read() == 0
        && g::<u8>(FRAME).read() & 7 == 0
        && talking
    {
        find_player_ped(-1);
    }
    update_in_tunnelness();
    weather_audio_service(g(0xC81360));
}

orig! {
    /// `FindPlayerCoors(CVector* out, int32)` (0x56E010)
    fn find_player_coors(out: GPtr<u8>, id: i32) -> GPtr<u8> = cdecl 0x56E010 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(this: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// CRT `printf` (0x821982)
    fn crt_printf(fmt: u32) -> i32 = cdecl 0x821982 => unported;
}

/// The oscillation used by `CCheckpoints::Update`: above `hi` with the flag set the flag clears; below
/// `lo` with the flag clear it sets; then the speed is stepped up (flag) or down by `d`.
fn osc(speed: GPtr<f32>, flag: GPtr<u8>, hi: u32, lo: u32, up: f32, down: f32) {
    if !x(speed.read()).fcom(x(k(hi))).test_any(0x41) && flag.read() != 0 {
        flag.write(0);
    }
    if !jp(x(speed.read()), x(k(lo)), 5) && flag.read() == 0 {
        flag.write(1);
    }
    speed.write(if flag.read() != 0 { up } else { down });
}

/// `|v|` the way `CCheckpoints::Update` does it: `v * -1.0` when below zero.
fn neg_abs(v: X87) -> X87 {
    if jp(v, X87::zero(), 5) { v } else { v.fmul_m(k(0xBF80_0000)) }
}

/// `CCheckpoints::Update()` (0x7229C0): for each of the 32 checkpoints (0x38 bytes at 0xC7F158: type
/// u16 +0, flag +3, position +0x10, direction +0x1C, radius +0x2C, speed +0x34). Type 5 swings its
/// speed between 0.1 and -0.1 (flag flips beyond ±0.5) and adds it to every direction component
/// (normalised). Type 6 steps its speed by ±0.01 (flips at 0.75 / 0) and becomes 0x101 once the
/// speed is negative. Type 7 (player within the 2D radius → type 8, speed -0.1; else speed ±0.02,
/// flips at 0.2 / -0.2, z += speed). Type 8 out of range → type 7, flag set, speed 0.02; then z +=
/// speed unless the player is not below z + 0.75 (a debug `printf` instead).
#[hook(addr = 0x7229C0, conv = cdecl, name = "CCheckpoints::Update", expect = "83 EC 0C 53 55",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn checkpoints_update() {
    let mut p = [0f32; 3];
    find_player_coors(GPtr::from_raw(p.as_mut_ptr()).cast(), -1);
    for i in 0..32isize {
        let c = g::<u8>(0xC7F158).byte_add(i * 0x38);
        let ty = c.cast::<u16>();
        let flag = c.byte_add(3);
        let speed = c.byte_add(0x34).cast::<f32>();
        if ty.read() == 5 {
            if !x(speed.read()).fcom(x(k(0x3F00_0000))).test_any(0x41) && flag.read() != 0 {
                flag.write(0);
            }
            if !jp(x(speed.read()), x(k(0xBF00_0000)), 5) && flag.read() == 0 {
                flag.write(1);
            }
            speed.write(if flag.read() != 0 { f32::from_bits(0x3DCC_CCCD) } else { f32::from_bits(0xBDCC_CCCD) });
            let d = c.byte_add(0x1C).cast::<f32>();
            d.write(x(speed.read()).fadd_m(d.read()).to_f32());
            d.wrapping_index(1).write(x(d.wrapping_index(1).read()).fadd_m(speed.read()).to_f32());
            d.wrapping_index(2).write(x(speed.read()).fadd_m(d.wrapping_index(2).read()).to_f32());
            vector_normalise(d.cast());
        }
        if ty.read() == 6 {
            let up = x(speed.read()).fadd_m(k(0x3C23_D70A)).to_f32();
            let down = x(speed.read()).fsub_m(k(0x3C23_D70A)).to_f32();
            osc(speed, flag, 0x3F40_0000, 0, up, down);
            if !jp(x(speed.read()), X87::zero(), 5) {
                ty.write(0x101);
            }
        }
        let dist_in = || {
            let dx = neg_abs(x(p[0]).fsub_m(rd::<f32>(c, 0x10)));
            let dy = neg_abs(x(p[1]).fsub_m(rd::<f32>(c, 0x14)));
            let r = x(rd::<f32>(c, 0x2C));
            r.fmul(r).fcom(dy.fmul(dy).fadd(dx.fmul(dx)))
        };
        if ty.read() == 7 {
            if !dist_in().c0() {
                ty.write(8);
                speed.write(f32::from_bits(0xBDCC_CCCD));
            } else {
                let up = x(speed.read()).fadd_m(k(0x3CA3_D70A)).to_f32();
                let down = x(speed.read()).fsub_m(k(0x3CA3_D70A)).to_f32();
                osc(speed, flag, 0x3E4C_CCCD, 0xBE4C_CCCD, up, down);
                let z = c.byte_add(0x18).cast::<f32>();
                z.write(x(speed.read()).fadd_m(z.read()).to_f32());
            }
        }
        if ty.read() == 8 {
            if !dist_in().test_parity(5) {
                ty.write(7);
                flag.write(1);
                speed.write(f32::from_bits(0x3CA3_D70A));
            }
            let z = c.byte_add(0x18).cast::<f32>();
            if jp(x(p[2]).fsub_m(k(0x3F40_0000)), x(z.read()), 5) {
                crt_printf(0x8729BC);
            } else {
                z.write(x(speed.read()).fadd_m(z.read()).to_f32());
            }
        }
    }
}

orig! {
    /// `Fx_c::GetFxQuality()` (0x49EA50)
    fn fx_get_fx_quality(this: GPtr<u8>) -> u32 = thiscall 0x49EA50 => unported;
    /// `RwRenderStateSet` (`RwEngineInstance` slot 0x20)
    fn rs(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `CPlantSurfPropMgr::GetSurfProperties(uint16)` (0x6F9DE0; pushed as a dword)
    fn plant_get_surf_properties(surface: u32) -> GPtr<u8> = cdecl 0x6F9DE0 => unported;
    /// `CCamera::IsSphereVisible(const CVector&, float, const RwMatrix*)` (0x420C40, bool in AL)
    fn camera_is_sphere_visible_m(this: GPtr<u8>, c: GPtr<u8>, r: u32, m: u32) -> u8 = thiscall 0x420C40 => unported;
    /// `CGrassRenderer::AddTriPlant(PPTriPlant*, uint32)` (0x5DB1D0)
    fn grass_add_tri_plant(plant: GPtr<u8>, mode: u32) = cdecl 0x5DB1D0 => crate::mvpd_r3::grass_add_tri_plant;
    /// `CGrassRenderer::FlushTriPlantBuffer()` (0x5DB250)
    fn grass_flush_tri_plant_buffer() = cdecl 0x5DB250 => unported;
}

/// `CPlantMgr::Render()` (0x5DBAE0): nothing at FX quality 0. States (8,0), (6,1), (12,1), (10,5), (11,6),
/// (14,1), (30,0), (29,8). For each of the 4 pipe modes (close plant list 0xC03988, texture table
/// 0xC039E0): every loc tri with plants (+0x48 bit 0) takes its surface's 3 plant slots
/// (`GetSurfProperties(+0x46)`, 0x28 bytes from +0x14); a slot with a model (≠ 0xFFFF) and a visible
/// sphere (+0x24, radius +0x30; matrix 0xB6FA74, or 0xB6FABC when `TheCamera` +0x970) becomes a
/// `PPTriPlant` (corners, centre, model, `(count + 8) & ~7` plants, scale, texture, colour scaled by
/// the light `ftol(((lo/30) (1 - D) + (hi/30) D) * 255)` with D = [0x8D12C0] and lo/hi the nibbles of
/// +0x47, intensities, seed +0x34, scale variation and wind values) passed to `AddTriPlant(…, mode)`;
/// `FlushTriPlantBuffer()` after each list. Finally (8,1), (6,1), (29,7), (30,0).
#[hook(addr = 0x5DBAE0, conv = cdecl, name = "CPlantMgr::Render", expect = "83 EC 78 B9 00 AE A9 00",
       unverified = "mvpd-render-4", route = "L2", hazards = "uninit-locals",
       recs = "grove-walk", note = "two padding bytes (+0x46/+0x47) of the stack PPTriPlant are never written by the original and are copied by AddTriPlant; zero here")]
pub fn plant_mgr_render() {
    if fx_get_fx_quality(g(0xA9AE00)) == 0 {
        return;
    }
    rs(8, 0);
    rs(6, 1);
    rs(0xC, 1);
    rs(0xA, 5);
    rs(0xB, 6);
    rs(0xE, 1);
    rs(0x1E, 0);
    rs(0x1D, 8);
    for mode in 0..4u32 {
        let tex_tbl = g::<GPtr<u8>>(0xC039E0).byte_add(mode as isize * 4).read();
        let mut t = g::<GPtr<u8>>(0xC03988).byte_add(mode as isize * 4).read();
        while !t.is_null() {
            if rd::<u8>(t, 0x48) & 1 != 0 {
                let sp = plant_get_surf_properties(u32::from(rd::<u8>(t, 0x46)));
                for j in 0..3isize {
                    let slot = sp.byte_add(0x14 + j * 0x28);
                    let r = rd::<u32>(t, 0x30);
                    let centre = t.byte_add(0x24);
                    if camera_is_sphere_visible_m(g(0xB6F028), centre, r, 0xB6FA74) == 0
                        && (g::<u8>(0xB6F998).read() == 0 || camera_is_sphere_visible_m(g(0xB6F028), centre, r, 0xB6FABC) == 0)
                    {
                        continue;
                    }
                    let model = slot.byte_add(-0x10).cast::<u16>().read();
                    if model == 0xFFFF {
                        continue;
                    }
                    let mut pl = [0u32; 0x17];
                    for (w, d) in pl.iter_mut().take(12).enumerate() {
                        *d = rd::<u32>(t, w as isize * 4);
                    }
                    let cnt = (t.byte_add(0x40 + j * 2).cast::<u16>().read().wrapping_add(8)) & 0xFFF8;
                    pl[12] = u32::from(model) | (u32::from(cnt) << 16);
                    pl[13] = rd::<u32>(slot, -4);
                    pl[14] = rd::<u32>(slot, 0);
                    pl[15] = tex_tbl.cast::<u32>().wrapping_index(i32::from(slot.byte_add(-0xE).cast::<u16>().read())).read();
                    let d = x(g::<f32>(0x8D12C0).read());
                    let lv = rd::<u8>(t, 0x47);
                    let lo = X87::from_i32(i32::from(lv & 0xF)).fmul_m(k(0x3D08_8889));
                    let hi = X87::from_i32(i32::from(lv >> 4)).fmul_m(k(0x3D08_8889));
                    let l = lo.fmul(X87::one().fsub(d)).fadd(hi.fmul(d)).fmul_m(k(0x437F_0000)).ftol() as u8 as u32;
                    let c = rd::<u32>(slot, -0xC).to_le_bytes();
                    let cr = (u32::from(c[0]) * l) >> 8;
                    let cg = (u32::from(c[1]) * l) >> 8;
                    let cb = (u32::from(c[2]) * l) >> 8;
                    pl[16] = (cr & 0xFF) | ((cg & 0xFF) << 8) | ((cb & 0xFF) << 16) | (u32::from(c[3]) << 24);
                    pl[17] = u32::from(rd::<u8>(slot, -8)) | (u32::from(rd::<u8>(slot, -7)) << 8);
                    pl[18] = rd::<u32>(t, 0x34 + j * 4);
                    pl[19] = rd::<u32>(slot, 4);
                    pl[20] = rd::<u32>(slot, 8);
                    pl[21] = rd::<u32>(slot, 0x10);
                    pl[22] = rd::<u32>(slot, 0x14);
                    grass_add_tri_plant(GPtr::from_raw(pl.as_mut_ptr()).cast(), mode);
                }
            }
            t = rd::<GPtr<u8>>(t, 0x4C);
        }
        grass_flush_tri_plant_buffer();
    }
    rs(8, 1);
    rs(6, 1);
    rs(0x1D, 7);
    rs(0x1E, 0);
}

orig! {
    /// `CEntity::GetColModel()` (0x535300)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CMemoryMgr::Malloc(uint32 size, uint32 hint)` (0x72F420)
    fn memory_mgr_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F420 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(this: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CStencilShadows::UpdateHierarchy(CStencilShadowObject** first, CStencilShadowObject** last,
    /// CStencilShadowObject*)` (0x710BC0)
    fn stencil_update_hierarchy(a: GPtr<u8>, b: GPtr<u8>, o: GPtr<u8>) = cdecl 0x710BC0 => unported;
}

/// 0x710BA0 (`FUN_00710ba0`, called by `CStencilShadows::RegisterStencilShadows`), cdecl
/// `(CEntity* owner)`: the first stencil shadow object of the list at 0xC6A16C (linked through +0x14)
/// whose owner (+0) is `owner`, else NULL.
#[hook(addr = 0x710BA0, conv = cdecl, name = "FUN_00710ba0", expect = "A1 6C A1 C6 00",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn stencil_find_object(owner: u32) -> GPtr<u8> {
    let mut o = g::<GPtr<u8>>(0xC6A16C).read();
    while !o.is_null() {
        if rd::<u32>(o, 0) == owner {
            return o;
        }
        o = rd::<GPtr<u8>>(o, 0x14);
    }
    GPtr::NULL
}

/// 0x711280 (`FUN_00711280`, a `CStencilShadowObject` set-up called by `RegisterStencilShadows`),
/// thiscall `(CPhysical* owner, uint8 type)`, `ret 8`: type 1..2 and a col model whose data (+0x2C) has
/// shadow triangles (flag +7 bit 2, count +0x20 > 0): count (u16 +4), `count * 15` (+8), owner (+0),
/// type (+6), +0xC = 0, a `CMemoryMgr::Malloc(count * 180, 0)` buffer (+0x10), the owner reference
/// registered and `CStencilShadows::UpdateHierarchy(0xC6A168, 0xC6A16C, this)`; returns `this`, else
/// NULL.
#[hook(addr = 0x711280, conv = thiscall, name = "FUN_00711280", expect = "53 8A 5C 24 0C",
       unverified = "mvpd-render-4", route = "L2", hazards = "heap-writes",
       recs = "grove-walk", note = "allocates the shadow vertex buffer and links the object into the stencil shadow lists")]
pub fn stencil_object_setup(this: GPtr<u8>, owner: GPtr<u8>, ty: u32) -> GPtr<u8> {
    let t = ty as u8;
    if t == 0 || t > 2 {
        return GPtr::NULL;
    }
    let cd = rd::<GPtr<u8>>(get_col_model(owner), 0x2C);
    if cd.is_null() || rd::<u8>(cd, 7) & 4 == 0 {
        return GPtr::NULL;
    }
    let n = rd::<i32>(cd, 0x20);
    if n <= 0 {
        return GPtr::NULL;
    }
    this.byte_add(4).cast::<u16>().write(n as u16);
    let n15 = n.wrapping_mul(15);
    this.byte_add(8).cast::<i32>().write(n15);
    this.cast::<u32>().write(owner.addr() as u32);
    this.byte_add(6).write(t);
    this.byte_add(0xC).cast::<u32>().write(0);
    let buf = memory_mgr_malloc(n15.wrapping_mul(12) as u32, 0);
    this.byte_add(0x10).cast::<u32>().write(buf.addr() as u32);
    entity_register_reference(GPtr::from_addr(rd::<u32>(this, 0) as usize), this);
    stencil_update_hierarchy(g(0xC6A168), g(0xC6A16C), this);
    this
}

pub const HOOKS: &[HookReg] = &[
    HOOK_WEATHER_UPDATE,
    HOOK_CHECKPOINTS_UPDATE,
    HOOK_PLANT_MGR_RENDER,
    HOOK_STENCIL_FIND_OBJECT,
    HOOK_STENCIL_OBJECT_SETUP,
];
