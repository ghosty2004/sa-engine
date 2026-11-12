//! `CAEWeatherAudioEntity::UpdateParameters` (batch `std9-audio-1`): ported from the 1.0 US bytes (0x505A00..0x5067FF);
//! gta-reversed's `AEWeatherAudioEntity.cpp` is a reading aid (it restructures the two inlined line-of-sight blocks and
//! the wind frequency step; the exe wins). Unverified (docs/verify/HOWTO.md).
//!
//! x87 values through [`X87`] in the original's operation order; floats are rounded where the original stores them.
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::{GPtr, X87, ae_resolve_probability, f32_of, g, rd, wr, x, xi};

orig! {
    /// `operator==(const CVector&, const CVector&)` (0x4DC670, AL)
    fn vector_equal(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x4DC670 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPlaceable::GetRightDirection(CVector* out)` (0x41CC70, returns `out`)
    fn placeable_get_right(this: GPtr<u8>, out: GPtr<u8>) -> u32 = thiscall 0x41CC70 => unported;
    /// `CPlaceable::GetTopDirection(CVector* out)` (0x41CCB0, returns `out`)
    fn placeable_get_top(this: GPtr<u8>, out: GPtr<u8>) -> u32 = thiscall 0x41CCB0 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(this: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CWorld::ProcessLineOfSight(start, end, CColPoint&, CEntity*&, buildings, vehicles, peds, objects, dummies,
    /// seeThrough, camera, shootThrough)` (0x56BA00, AL)
    fn world_process_line_of_sight(s: GPtr<u8>, e: GPtr<u8>, cp: GPtr<u8>, ent: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32, st: u32, c: u32, sh: u32) -> u8 = cdecl 0x56BA00 => unported;
    /// `CEntity::GetIsTypePhysical()` (0x4DA030, AL)
    fn entity_is_physical(this: GPtr<u8>) -> u8 = thiscall 0x4DA030 => unported;
    /// `operator*(float, const CVector&)` (0x40FE90, returns the hidden `out`)
    fn scalar_mul_vector(out: GPtr<u8>, s: u32, v: GPtr<u8>) -> u32 = cdecl 0x40FE90 => unported;
    /// `operator-(const CVector&, const CVector&)` (0x40FE60, returns the hidden `out`)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> u32 = cdecl 0x40FE60 => unported;
    /// `CVector::Magnitude()` (0x4082C0, ST0)
    fn vector_magnitude(this: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CAEAudioUtility::GetPiecewiseLinear(float, int16, const float (*)[2])` (0x4D9D90, ST0)
    fn get_piecewise_linear(v: u32, n: u32, table: u32) -> f64 = cdecl 0x4D9D90 => unported;
    /// `CAESound::StopSoundAndForget()` (0x4EF850)
    fn sound_stop_and_forget(this: GPtr<u8>) = thiscall 0x4EF850 => unported;
}

/// `CWeather::LightningDuration` (uint32), `CWeather::WindClipped`, `CGame::currArea`
const LIGHTNING_DURATION: usize = 0xC812C4;
const WIND_CLIPPED: usize = 0xC812EC;
const CURR_AREA: usize = 0xB72914;
/// Pointer to the default event volumes (int8 per event: +0x8D thunder, +0x90 city noise).
const EVENT_VOLUMES_PTR: usize = 0xBD00F8;
/// `TheCamera` (its `CPlaceable`), its matrix pointer (+0x14) and placement position (+4)
const THE_CAMERA: usize = 0xB6F028;
const CAMERA_MATRIX: usize = 0xB6F03C;
const CAMERA_PLACEMENT_POS: usize = 0xB6F02C;
/// The city-noise statics: (volume boost residue, last entity) for the left (0xB6BC68) and right (0xB6BC70) sides.
const RESIDUE_LEFT: usize = 0xB6BC68;
const LAST_ENTITY_LEFT: usize = 0xB6BC6C;
const RESIDUE_RIGHT: usize = 0xB6BC70;
const LAST_ENTITY_RIGHT: usize = 0xB6BC74;
/// The wind statics: offset toggle (byte 0x8CC2C0), offset (0xB6BAFC), frequency (0xB6BAF8), old frequency (0x8CC2C4).
const WIND_TOGGLE: usize = 0x8CC2C0;
const WIND_OFFSET: usize = 0xB6BAFC;
const WIND_FREQ: usize = 0xB6BAF8;
const WIND_OLD_FREQ: usize = 0x8CC2C4;
/// `CRT _CIpow` (0x822130)
const CI_POW: usize = 0x822130;

/// An f32 of the exe's data (`.rdata` constants and the `.data` floats the function reads as constants).
fn c(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

/// `fild dword` of an unsigned value: the signed load plus 2^32 ([0x858C54]) when the sign bit is set.
fn fild_u32(v: u32) -> X87 {
    let r = xi(v as i32);
    if (v as i32) < 0 { r.fadd_m(c(0x858C54)) } else { r }
}

/// `fldlg2; fxch; fyl2x`: log10 of ST0 (64-bit result).
fn log10(v: X87) -> X87 {
    X87::lg2().fyl2x(v)
}

/// The camera position (matrix +0x30, else the placement +4).
fn camera_pos() -> GPtr<u8> {
    let m = g::<u32>(CAMERA_MATRIX).read();
    if m != 0 { g(m as usize + 0x30) } else { g(CAMERA_PLACEMENT_POS) }
}

/// The default volume byte of an event (`movsx [[0xBD00F8] + off]`).
fn event_volume(off: usize) -> i32 {
    let p = g::<u32>(EVENT_VOLUMES_PTR).read();
    rd::<i8>(g(p as usize), off as isize) as i32
}

/// A local `CVector` as a pointer.
fn vp(v: &mut [f32; 3]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}

/// Fade-out (0x5067B0): volume `<= -50` or NaN stops the sound (`StopSoundAndForget`); otherwise volume = `max'(volume
/// - 0.6, -50)` (-50 unless the step is above it).
fn fade_out(sound: GPtr<u8>) {
    let v = x(rd::<f32>(sound, 0x14));
    if v.fcom(x(c(0x859B60))).test_any(0x41) {
        sound_stop_and_forget(sound);
        return;
    }
    let n = v.fsub_m(c(0x8CC2AC));
    let r = if n.fcom(x(c(0x859B60))).test_any(0x41) { x(c(0x859B60)) } else { n };
    wr::<f32>(sound, 0x14, f32_of(r));
}

/// Event 1 (thunder, 0x505A85): `req = log10(dur * 0.0375 + 0.25) * 20 + default(+0x8D)` (dur = `LightningDuration`
/// as unsigned), `reqPos = 600 - dur / 20 * 500` (uint32 arithmetic). `pos <= reqPos` (unsigned): volume =
/// `log10(pos / reqPos) * 20 + req`; `pos <= reqPos + 200` and `req > 0`: `p = 10^(req * 0.05)`, volume =
/// `log10((pos - reqPos) * (1 - p) * 0.005 + p) * 20`; otherwise `min'(req, 0)`.
fn thunder(sound: GPtr<u8>, pos: i32) {
    let dur = g::<u32>(LIGHTNING_DURATION).read();
    let v = fild_u32(dur).fmul_m(c(0x862E40)).fadd(X87::from_f64_exact(0.25));
    let def = event_volume(0x8D);
    let req_pos = 600u32.wrapping_sub((dur / 20).wrapping_mul(500));
    let req = f32_of(log10(v).fmul_m(c(0x858BA4)).fadd(xi(def)));
    let p = pos as u32;
    let vol = if p <= req_pos {
        let r = xi(pos).fdiv(fild_u32(req_pos));
        log10(r).fmul_m(c(0x858BA4)).fadd_m(req)
    } else if p <= req_pos.wrapping_add(0xC8) && !x(req).fcom(x(c(0x858B50))).test_any(0x41) {
        let e = x(req).fmul_m(c(0x858C28));
        let pw = X87::from_f64_exact(sa_core::orig::x87_binary(CI_POW, 10.0, e.to_f64()));
        let d = fild_u32(p.wrapping_sub(req_pos));
        let t = d.fmul(X87::one().fsub(pw)).fmul_m(c(0x858B4C)).fadd(pw);
        log10(t).fmul_m(c(0x858BA4))
    } else if x(req).fcom(x(c(0x858B50))).test_parity(0x05) {
        x(c(0x858B50))
    } else {
        x(req)
    };
    wr::<f32>(sound, 0x14, f32_of(vol));
}

/// One side of the city noise (two inlined copies in the original, 0x505CC1 right and 0x5060D8 left): the line of
/// sight from `origin` to `target` (buildings, vehicles, objects, dummies). A hit entity other than the side's last
/// one, with the camera within 6 of the listener height: `force = |(vel . fwd) fwd - (hitVel . fwd) fwd|` (vel = the
/// player's vehicle move speed when in one, hitVel = the hit physical's; fwd = the camera's normalised top
/// direction); above 0.35: `s = (force - 0.35) * [0x862E30]`, volume add = `log10((1 - |pov + right - hit| / 6) * s) *
/// 20 + 30`, speed = `s * 1.75 + 1.75`. Otherwise add 0, speed 1. Then [`city_noise_volume`].
#[allow(clippy::too_many_arguments)]
fn city_noise_side(
    sound: GPtr<u8>,
    player: GPtr<u8>,
    pov: [f32; 3],
    right: [f32; 3],
    mut origin: [f32; 3],
    mut target: [f32; 3],
    def: f32,
    residue: usize,
    last_entity: usize,
) {
    let mut cp = [0u32; 11];
    let mut hit: u32 = 0;
    world_process_line_of_sight(
        vp(&mut origin),
        vp(&mut target),
        GPtr::from_raw(cp.as_mut_ptr()).cast(),
        GPtr::from_raw(&raw mut hit).cast(),
        1,
        1,
        0,
        1,
        1,
        0,
        0,
        0,
    );
    // L4 (volume add) starts at 0, Lc (speed) at 1.0
    let mut add = 0f32;
    let mut speed = x(1.0);
    'hit: {
        if hit == 0 || hit == g::<u32>(last_entity).read() {
            break 'hit;
        }
        let cz = rd::<f32>(camera_pos(), 8);
        if x(cz).fsub_m(pov[2]).fabs().fcom(x(c(0x858B44))).test_parity(0x05) {
            break 'hit;
        }
        let mut vel = [0f32; 3];
        if !player.is_null() && rd::<u32>(player, 0x46C) & 0x100 != 0 {
            let veh = rd::<GPtr<u8>>(player, 0x58C);
            if !veh.is_null() {
                vel = [rd(veh, 0x44), rd(veh, 0x48), rd(veh, 0x4C)];
            }
        }
        let he = g::<u8>(hit as usize);
        let mut hvel = [0f32; 3];
        if entity_is_physical(he) != 0 {
            hvel = [rd(he, 0x44), rd(he, 0x48), rd(he, 0x4C)];
        }
        let mut fwd = [0f32; 3];
        placeable_get_top(g(THE_CAMERA), vp(&mut fwd));
        vector_normalise(vp(&mut fwd));
        let d1 = f32_of(x(fwd[2]).fmul_m(vel[2]).fadd(x(fwd[1]).fmul_m(vel[1])).fadd(x(fwd[0]).fmul_m(vel[0])));
        let d2 = f32_of(x(fwd[2]).fmul_m(hvel[2]).fadd(x(fwd[1]).fmul_m(hvel[1])).fadd(x(fwd[0]).fmul_m(hvel[0])));
        let mut a = [0f32; 3];
        let mut b = [0f32; 3];
        let mut diff = [0f32; 3];
        scalar_mul_vector(vp(&mut b), d2.to_bits(), vp(&mut fwd));
        scalar_mul_vector(vp(&mut a), d1.to_bits(), vp(&mut fwd));
        vector_sub(vp(&mut diff), vp(&mut a), vp(&mut b));
        let force = X87::from_f64_exact(vector_magnitude(vp(&mut diff)));
        if force.fcom(x(c(0x858F9C))).test_any(0x41) {
            break 'hit;
        }
        let s = f32_of(force.fsub_m(c(0x858F9C)).fmul_m(c(0x862E30)));
        let px = x(right[0]).fadd_m(pov[0]);
        let py = x(pov[1]).fadd_m(right[1]);
        let pz = f32_of(x(right[2]).fadd_m(pov[2]));
        let mut d = [
            f32_of(px.fsub_m(f32::from_bits(cp[0]))),
            f32_of(py.fsub_m(f32::from_bits(cp[1]))),
            f32_of(x(pz).fsub_m(f32::from_bits(cp[2]))),
        ];
        let m = X87::from_f64_exact(vector_magnitude(vp(&mut d)));
        let t = m.fmul_m(c(0x85F0A0)).fsubr_m(c(0x858624)).fmul_m(s);
        add = f32_of(log10(t).fmul_m(c(0x858BA4)).fadd_m(c(0x858CA4)));
        speed = x(s).fmul_m(c(0x862E2C)).fadd_m(c(0x862E2C));
    }
    city_noise_volume(sound, def, add, speed, hit, residue, last_entity);
}

/// The city-noise volume step (0x505F2C / 0x50637B): `T = default + add`. volume `< T`: volume = `min'(volume + add +
/// 0.3, T)`, residue = add. volume `> T` with residue `<= 0`: volume = `max'(volume - 0.3, default)`. volume `> T`
/// with a positive residue: `r = min'(1.3, residue)`, volume = `max'(volume - 0.3, default) - r` (the choice compares
/// `default - r` with `volume - 0.3 - r`), residue -= r. Always speed (+0x1C) and the last entity.
fn city_noise_volume(sound: GPtr<u8>, def: f32, add: f32, speed: X87, hit: u32, residue: usize, last_entity: usize) {
    let cur = rd::<f32>(sound, 0x14);
    let t = f32_of(x(def).fadd_m(add));
    let k03 = c(0x858C24);
    let k13 = c(0x859918);
    if !x(cur).fcom(x(t)).test_parity(0x05) {
        // cur < T
        let n = x(cur).fadd_m(add).fadd_m(k03);
        let v = if n.fcom(x(t)).test_parity(0x05) { x(t) } else { n };
        wr::<f32>(sound, 0x14, f32_of(v));
        wr::<f32>(sound, 0x1C, f32_of(speed));
        g::<u32>(last_entity).write(hit);
        g::<f32>(residue).write(add);
        return;
    }
    if !x(cur).fcom(x(t)).test_any(0x41) {
        // cur > T
        let res = g::<f32>(residue).read();
        if x(res).fcom(x(c(0x858B50))).test_any(0x41) {
            let n = x(cur).fsub_m(k03);
            let v = if n.fcom(x(def)).test_any(0x41) { x(def) } else { n };
            wr::<f32>(sound, 0x14, f32_of(v));
        } else {
            // r = (1.3 < residue) ? 1.3 : residue
            let r = || if x(k13).fcom(x(res)).test_parity(0x05) { x(res) } else { x(k13) };
            let r1 = r();
            let r2 = r();
            let cm = x(cur).fsub(x(k03));
            let cm_f = f32_of(cm);
            let a = cm.fsub(r1);
            let b = x(def).fsub(r2);
            let v = if !b.fcom(a).test_parity(0x05) {
                // (default - r) < (volume - 0.3 - r)
                r().fsubr_m(cm_f)
            } else {
                r().fsubr_m(def)
            };
            wr::<f32>(sound, 0x14, f32_of(v));
            let r3 = r();
            g::<f32>(residue).write(f32_of(x(res).fsub(r3)));
        }
    }
    wr::<f32>(sound, 0x1C, f32_of(speed));
    g::<u32>(last_entity).write(hit);
}

/// Event 3 (city noise, 0x505BEB): in an interior the fade-out; otherwise the listener `pov` = the camera position
/// with the player's z (when there is a player), `right` = the camera's normalised right direction. A sound at the
/// default position (-0.906, 0.423, 0) checks the right side (`pov + right` to `pov + right * 4`, statics
/// 0xB6BC70 / 0xB6BC74), any other the left (`pov - right` to `pov - right * 4`, 0xB6BC68 / 0xB6BC6C).
fn city_noise(sound: GPtr<u8>) {
    if g::<u32>(CURR_AREA).read() != 0 {
        fade_out(sound);
        return;
    }
    let def = f32_of(xi(event_volume(0x90)));
    let cp = camera_pos();
    let mut pov = [rd::<f32>(cp, 0), rd::<f32>(cp, 4), rd::<f32>(cp, 8)];
    let player = find_player_ped(-1);
    if !player.is_null() {
        let m = rd::<GPtr<u8>>(player, 0x14);
        let pp = if m.is_null() { player.byte_add(4) } else { m.byte_add(0x30) };
        pov[2] = rd::<f32>(pp, 8);
    }
    let mut right = [0f32; 3];
    placeable_get_right(g(THE_CAMERA), vp(&mut right));
    vector_normalise(vp(&mut right));
    let k4 = c(0x858B90);
    let r4 = [f32_of(x(right[0]).fmul_m(k4)), f32_of(x(right[1]).fmul_m(k4)), f32_of(x(right[2]).fmul_m(k4))];
    let mut default_pos = [f32::from_bits(0xBF67EF9E), f32::from_bits(0x3ED89375), 0.0];
    if vector_equal(sound.byte_add(0x24), vp(&mut default_pos)) != 0 {
        let target = [
            f32_of(x(r4[0]).fadd_m(pov[0])),
            f32_of(x(pov[1]).fadd_m(r4[1])),
            f32_of(x(r4[2]).fadd_m(pov[2])),
        ];
        let origin = [
            f32_of(x(right[0]).fadd_m(pov[0])),
            f32_of(x(pov[1]).fadd_m(right[1])),
            f32_of(x(right[2]).fadd_m(pov[2])),
        ];
        city_noise_side(sound, player, pov, right, origin, target, def, RESIDUE_RIGHT, LAST_ENTITY_RIGHT);
    } else {
        let m1 = c(0x858C1C);
        let (nx, ny, nz) = (x(right[0]).fmul_m(m1), x(right[1]).fmul_m(m1), f32_of(x(right[2]).fmul_m(m1)));
        let target = [
            f32_of(x(r4[0]).fmul_m(m1).fadd_m(pov[0])),
            f32_of(x(r4[1]).fmul_m(m1).fadd_m(pov[1])),
            f32_of(x(f32_of(x(r4[2]).fmul_m(m1))).fadd_m(pov[2])),
        ];
        let origin = [f32_of(nx.fadd_m(pov[0])), f32_of(ny.fadd_m(pov[1])), f32_of(x(nz).fadd_m(pov[2]))];
        city_noise_side(sound, player, pov, right, origin, target, def, RESIDUE_LEFT, LAST_ENTITY_LEFT);
    }
}

/// Events 4 and 5 (wind, 0x506527): `zf = clamp'(camera z * 0.002, 0, 1)`, `ratio = (pw2 - pw1) * clamp'(WindClipped,
/// 0, 1) + pw1` (the piecewise-linear tables at 0x8CC2D0 / 0x8CC2E8, 3 points); `ResolveProbability([0x8CC2CC])`
/// toggles the wind byte b; offset = `21 * b * ratio`, frequency = `1.2 * b * ratio`. In an interior the fade-out;
/// otherwise the volume steps by 0.6 towards `-33 + offset`, and the old frequency: below the frequency it rises by
/// 0.1 (capped at the frequency); above it, it falls by 0.1 but not below 1.0; the sound's speed (+0x1C) = it.
fn wind(sound: GPtr<u8>) {
    let cz = || x(rd::<f32>(camera_pos(), 8)).fmul_m(c(0x858F44));
    let zf: f32 = if cz().fcom(x(c(0x858624))).test_any(0x41) {
        if x(c(0x858B50)).fcom(cz()).test_any(0x41) {
            if cz().fcom(x(c(0x858624))).test_any(0x41) { f32_of(cz()) } else { 1.0 }
        } else {
            0.0
        }
    } else if cz().fcom(x(c(0x858624))).test_any(0x41) {
        f32_of(cz())
    } else {
        1.0
    };
    let pw1 = f32_of(X87::from_f64_exact(get_piecewise_linear(zf.to_bits(), 3, 0x8CC2D0)));
    let pw2 = X87::from_f64_exact(get_piecewise_linear(zf.to_bits(), 3, 0x8CC2E8));
    let wc = c(WIND_CLIPPED);
    let wcl = if !x(c(0x858624)).fcom(x(wc)).test_parity(0x05) {
        x(c(0x858624))
    } else if x(c(0x858B50)).fcom(x(wc)).test_any(0x41) {
        x(wc)
    } else {
        x(c(0x858B50))
    };
    let ratio = f32_of(pw2.fsub_m(pw1).fmul(wcl).fadd_m(pw1));
    if ae_resolve_probability(c(0x8CC2CC)) != 0 {
        let b = g::<u8>(WIND_TOGGLE).read();
        g::<u8>(WIND_TOGGLE).write(u8::from(b == 0));
    }
    let b = xi(i32::from(g::<u8>(WIND_TOGGLE).read()));
    g::<f32>(WIND_OFFSET).write(f32_of(x(c(0x8CC2B0)).fmul(b).fmul_m(ratio)));
    g::<f32>(WIND_FREQ).write(f32_of(b.fmul_m(c(0x8CC2B4)).fmul_m(ratio)));
    if g::<u32>(CURR_AREA).read() != 0 {
        fade_out(sound);
        return;
    }
    let target = f32_of(x(c(0x8CC2A4)).fadd_m(c(WIND_OFFSET)));
    let vol = x(rd::<f32>(sound, 0x14));
    if !vol.fcom(x(target)).test_parity(0x05) {
        let n = vol.fadd_m(c(0x8CC2A8));
        let v = if n.fcom(x(target)).test_parity(0x05) { x(target) } else { n };
        wr::<f32>(sound, 0x14, f32_of(v));
    } else if !vol.fcom(x(target)).test_any(0x41) {
        let n = vol.fsub_m(c(0x8CC2AC));
        let v = if n.fcom(x(target)).test_any(0x41) { x(target) } else { n };
        wr::<f32>(sound, 0x14, f32_of(v));
    }
    let old = c(WIND_OLD_FREQ);
    let freq = c(WIND_FREQ);
    if !x(old).fcom(x(freq)).test_parity(0x05) {
        let n = x(c(0x8CC2B8)).fadd_m(old);
        if n.fcom(x(freq)).test_parity(0x05) {
            g::<u32>(WIND_OLD_FREQ).write(freq.to_bits());
        } else {
            g::<f32>(WIND_OLD_FREQ).write(f32_of(n));
        }
    } else if !x(old).fcom(x(freq)).test_any(0x41) {
        let n = x(old).fsub_m(c(0x8CC2BC));
        if n.fcom(x(c(0x858624))).test_any(0x41) {
            g::<u32>(WIND_OLD_FREQ).write(0x3F80_0000);
        } else {
            g::<f32>(WIND_OLD_FREQ).write(f32_of(n));
        }
    }
    wr::<u32>(sound, 0x1C, g::<u32>(WIND_OLD_FREQ).read());
}

/// `CAEWeatherAudioEntity::UpdateParameters(CAESound* sound, int16 curPlayPos)` (0x505A00, `ret 8`): nothing for a
/// play position `<= 0`; by the sound's event (+0xC): 1 thunder, 3 city noise, 4 / 5 wind, anything else nothing.
#[hook(addr = 0x505A00, conv = thiscall, name = "CAEWeatherAudioEntity::UpdateParameters", expect = "81 EC 94 00 00 00 57",
       unverified = "std9-audio-1", route = "L2", hazards = "uninit-locals", recs = "grove-walk",
       note = "the line of sight's CColPoint is an uninitialised stack local in the original (zero here); \
               _CIpow through orig::x87_binary (libm in phases B/C)")]
pub fn weather_update_parameters(_this: GPtr<u8>, sound: GPtr<u8>, cur_play_pos: u32) {
    let pos = i32::from(cur_play_pos as u16 as i16);
    if pos <= 0 {
        return;
    }
    match rd::<u32>(sound, 0xC) {
        1 => thunder(sound, pos),
        3 => city_noise(sound),
        4 | 5 => wind(sound),
        _ => {}
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_WEATHER_UPDATE_PARAMETERS];
