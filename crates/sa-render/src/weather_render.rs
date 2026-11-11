//! WRITER-MVP-G: `CWeather::RenderRainStreaks`. Ported from the 1.0 US bytes; gta-reversed
//! `Weather.cpp` as reading aid. x87 arithmetic follows the original's instruction order on the
//! `sa_math::x87::X87` model; every direct `call rand` runs under `orig::at_site` with its original
//! return address.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, k, x, xi};

orig! {
    /// `CCullZones::CamNoRain()` (0x72DDB0, bool in AL)
    fn cam_no_rain() -> u8 = cdecl 0x72DDB0 => crate::cullzones::cam_no_rain;
    /// `CCullZones::PlayerNoRain()` (0x72DDC0, bool in AL)
    fn player_no_rain() -> u8 = cdecl 0x72DDC0 => crate::cullzones::player_no_rain;
    /// `operator new(size)` (0x821195)
    fn operator_new(size: u32) -> u32 = cdecl 0x821195 => unported;
    /// `CVector::Magnitude()` (0x4082C0, ST0)
    fn vector_magnitude(v: GPtr<f32>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn random_in_range(min: i32, max: i32) -> i32 = cdecl 0x407180 => unported;
    /// `RwRenderStateSet(state, value)` (`RwGlobals` +0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwIm3DTransform(verts, count, ltm, flags)` (0x7EF450)
    fn rw_im3d_transform(verts: u32, count: u32, ltm: u32, flags: u32) -> GPtr<u8> = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderIndexedPrimitive(primType, indices, numIndices)` (0x7EF550)
    fn rw_im3d_render_indexed_primitive(prim: u32, indices: u32, n: u32) -> u32 = cdecl 0x7EF550 => unported;
    /// `RwIm3DEnd()` (0x7EF520)
    fn rw_im3d_end() -> u32 = cdecl 0x7EF520 => unported;
}

/// The pause byte checked first, the integer the target count scales with (64 - it), `CWeather::Rain`,
/// the streak count, the underwaterness, `CGame::currArea`.
const PAUSE: usize = 0xB7_CB48;
const RAIN_SCALE_INT: usize = 0xB7_9E48;
const RAIN: usize = 0xC8_1324;
const NUM_STREAKS: usize = 0xC8_12B0;
const UNDER_WATERNESS: usize = 0xC8_132C;
const CURR_AREA: usize = 0xB7_2914;
/// The streak arrays (heap, allocated on first use): x, y, z (int32[32]) and alpha (u8[32]).
const STREAK_X: usize = 0xC8_1420;
const STREAK_Y: usize = 0xC8_141C;
const STREAK_Z: usize = 0xC8_1418;
const STREAK_ALPHA: usize = 0xC8_1414;
/// The wind strength and direction (`CWeather`), the camera front vector (`TheCamera` +0x984..).
const WIND: usize = 0xC8_12F0;
const WIND_DIR: usize = 0xC8_13E0;
const CAM_FRONT: usize = 0xB6_F9AC;
const CAM_MATRIX: usize = 0xB6_F03C;
const CAM_PLACEMENT_POS: usize = 0xB6_F02C;
/// `TempBufferVerticesStored`, `TempBufferIndicesStored`, `aTempBufferIndices`, `TempBufferRenderVertices`.
const TEMP_VERTICES_STORED: usize = 0xC4_B950;
const TEMP_INDICES_STORED: usize = 0xC4_B954;
const TEMP_BUFFER_INDICES: u32 = 0xC4_B958;
const TEMP_RENDER_VERTICES: u32 = 0xC4_D958;
/// 1 / 32767 (0x858C7C).
const RAND_SCALE: u32 = 0x3800_0100;

fn rand_at(site: u32) -> i32 {
    sa_core::orig::at_site(site, sa_core::crt::rand)
}

fn cam_pos() -> GPtr<f32> {
    let cm = g::<GPtr<u8>>(CAM_MATRIX).read();
    if cm.is_null() { GPtr::from_addr(CAM_PLACEMENT_POS) } else { cm.byte_add(0x30).cast() }
}

fn arr(base: usize, i: u32) -> GPtr<i32> {
    GPtr::from_addr(g::<u32>(base).read().wrapping_add(4 * i) as usize)
}

fn alpha_at(i: u32) -> GPtr<u8> {
    GPtr::from_addr(g::<u32>(STREAK_ALPHA).read().wrapping_add(i) as usize)
}

fn fresh_alpha() -> u8 {
    ftol32(xi(g::<i32>(NUM_STREAKS).read()).fmul_m(k(0x3F19_999A))) as u8
}

/// `ftol(rand * 5/32767 + front * 6 + cam - 2.5)` for one axis (`rand` at `site`).
fn respawn_axis(site: u32, front: usize, axis: usize) -> i32 {
    let r = xi(rand_at(site)).fmul_m(k(RAND_SCALE)).fmul_m(5.0);
    let c = x(g::<f32>(front).read()).fmul_m(6.0);
    let cam = cam_pos().wrapping_index(axis as i32).read();
    ftol32(r.fadd(c).fadd_m(cam).fsub_m(2.5))
}

/// `CWeather::RenderRainStreaks()` (0x72AF70). Nothing while 0xB7CB48 is set. The streak count
/// 0xC812B0 moves by one toward `ftol(ftol(Rain * 110) * (64 - [0xB79E48]) / 64)` and is clamped at
/// 0; with no streaks, `CamNoRain` / `PlayerNoRain`, positive underwaterness, an interior or the
/// camera above z 900 nothing is drawn. The four streak arrays are allocated on first use (`new`
/// 0x80, 0x80, 0x80, 0x20: y, x, z, alpha), positions zeroed and alphas `ftol(count * 0.6)`. For
/// each of the 32 streaks: one with alpha, z above 0 and within 8 of the camera stays, otherwise it
/// respawns around the camera (`ftol(rand * 5/32767 + front * 6 + cam - 2.5)` per axis, rand at
/// 0x72B1A6 / 0x72B1F4 / 0x72B243; alpha `ftol(count * 0.6)`). The tail end is the position minus
/// the wind (`dir * wind * 0.1`, odd streaks; even ones also `* Rain`); a line (indices n, n+1) from
/// `pos + (r2, r3, r4')` (colour `alpha << 24 | 0xD2D2E6`) to `tail + (r2, r3, r1 * 0.4 + 0.1 + r4')`
/// (colour `alpha << 23 | 0xD2D2E6`) with `r1 = rand / 32767`, `r2, r3 = rand / 32767 * 0.4 - 0.2`,
/// `r4' = rand / 32767 * 0.2 - 0.1` (rand at 0x72B361, 0x72B394, 0x72B3B7, 0x72B3DA); then the
/// streak falls by `rand / 32767 * 0.09 + 0.01` (0x72B4BA; stored with ftol) and its alpha drops by
/// `GetRandomNumberInRange(2, 5)` (to 0 when not above it). With lines stored: states (8,0) (6,1)
/// (14,0) (16,1) (10,5) (11,6) (12,1) (1,0), `RwIm3DTransform(.., NULL, 8)`, indexed line list, end,
/// then (8,1) (6,1) (10,5) (11,6) (14,0) (12,0). Both temp-buffer counts end at 0.
#[hook(addr = 0x72AF70, conv = cdecl, name = "CWeather::RenderRainStreaks", expect = "A0 48 CB B7 00 83 EC 2C",
       unverified = "mvpg-render-1", route = "L2", hazards = "x87-transcendental,heap-writes",
       recs = "grove-walk",
       note = "fsqrt in CVector::Magnitude; rand at 0x72B1A6/1F4/243/361/394/3B7/3DA/4BA; allocates and writes the four streak arrays (heap), TempBufferRenderVertices and aTempBufferIndices")]
pub fn render_rain_streaks() {
    if g::<u8>(PAUSE).read() != 0 {
        return;
    }
    let a = x(64.0).fsub(xi(g::<i32>(RAIN_SCALE_INT).read()));
    let n = ftol32(x(g::<f32>(RAIN).read()).fmul_m(110.0));
    let target = ftol32(a.fmul(xi(n)).fmul_m(k(0x3C80_0000)));
    let mut c = g::<i32>(NUM_STREAKS).read();
    if c < target {
        c += 1;
    } else if c > target {
        c -= 1;
    }
    let c = if c < 0 { 0 } else { c };
    g::<i32>(NUM_STREAKS).write(c);
    if c == 0 || cam_no_rain() != 0 || player_no_rain() != 0 {
        return;
    }
    if !x(g::<f32>(UNDER_WATERNESS).read()).fcom(x(0.0)).test_any(0x41) {
        return;
    }
    if g::<u32>(CURR_AREA).read() != 0 {
        return;
    }
    if !x(cam_pos().wrapping_index(2).read()).fcom(x(900.0)).test_any(0x41) {
        return;
    }
    g::<u32>(TEMP_INDICES_STORED).write(0);
    g::<u32>(TEMP_VERTICES_STORED).write(0);
    if g::<u32>(STREAK_X).read() == 0 {
        g::<u32>(STREAK_Y).write(operator_new(0x80));
        g::<u32>(STREAK_X).write(operator_new(0x80));
        g::<u32>(STREAK_Z).write(operator_new(0x80));
        g::<u32>(STREAK_ALPHA).write(operator_new(0x20));
        for i in 0..32u32 {
            arr(STREAK_Y, i).write(0);
            arr(STREAK_X, i).write(0);
            arr(STREAK_Z, i).write(0);
            alpha_at(i).write(fresh_alpha());
        }
    }
    for i in 0..32u32 {
        let p = cam_pos();
        let mut v = [
            xi(arr(STREAK_X, i).read()).fsub_m(p.read()).to_f32(),
            xi(arr(STREAK_Y, i).read()).fsub_m(p.wrapping_index(1).read()).to_f32(),
            0f32,
        ];
        let zi = xi(arr(STREAK_Z, i).read());
        v[2] = zi.fsub_m(p.wrapping_index(2).read()).to_f32();
        let keep = alpha_at(i).read() > 0
            && zi.fcom(x(0.0)).test_parity(0x41)
            && X87::from_f64_exact(vector_magnitude(GPtr::from_raw(v.as_mut_ptr()))).fcom(x(8.0)).test_any(0x41);
        if !keep {
            arr(STREAK_X, i).write(respawn_axis(0x72_B1AB, CAM_FRONT, 0));
            arr(STREAK_Y, i).write(respawn_axis(0x72_B1F9, CAM_FRONT + 4, 1));
            arr(STREAK_Z, i).write(respawn_axis(0x72_B248, CAM_FRONT + 8, 2));
            alpha_at(i).write(fresh_alpha());
        }
        let wind = x(g::<f32>(WIND).read());
        let rain = g::<f32>(RAIN).read();
        let tail = |a: usize, d: usize| {
            let w = x(g::<f32>(WIND_DIR + 4 * d).read()).fmul(wind);
            let w = if i & 1 != 0 { w } else { w.fmul_m(rain) };
            xi(arr(a, i).read()).fsub(w.fmul_m(k(0x3DCC_CCCD))).to_f32()
        };
        let tx = tail(STREAK_X, 0); // [esp+0x10]
        let ty = tail(STREAK_Y, 1); // [esp+0x14]
        let tz = tail(STREAK_Z, 2); // [esp+0x18]
        let r1 = rand_at(0x72_B366);
        let nv = g::<u32>(TEMP_VERTICES_STORED).read();
        let ni = g::<u32>(TEMP_INDICES_STORED).read();
        g::<u16>((TEMP_BUFFER_INDICES + 2 * ni) as usize).write(nv as u16);
        g::<u16>((TEMP_BUFFER_INDICES + 2 * ni + 2) as usize).write((nv + 1) as u16);
        let f1 = xi(r1).fmul_m(k(RAND_SCALE)).to_f32(); // [esp+0x20]
        let f2 = xi(rand_at(0x72_B399)).fmul_m(k(RAND_SCALE)).fmul_m(k(0x3ECC_CCCD)).fsub_m(k(0x3E4C_CCCD)).to_f32(); // [esp+0x24]
        let f3 = xi(rand_at(0x72_B3BC)).fmul_m(k(RAND_SCALE)).fmul_m(k(0x3ECC_CCCD)).fsub_m(k(0x3E4C_CCCD)).to_f32(); // [esp+0x28]
        let r4 = rand_at(0x72_B3DF);
        let al = u32::from(alpha_at(i).read());
        let q = xi(r4).fmul_m(k(RAND_SCALE)).fmul_m(k(0x3E4C_CCCD)).fsub_m(k(0x3DCC_CCCD));
        let v0 = GPtr::<u8>::from_addr((TEMP_RENDER_VERTICES + 0x24 * nv) as usize);
        let v1 = v0.byte_add(0x24);
        let wf = |p: GPtr<u8>, off: isize, val: X87| crate::common::wr::<f32>(p, off, val.to_f32());
        crate::common::wr::<u32>(v0, 0x18, (al << 24) | 0xD2_D2E6);
        wf(v0, 0, xi(arr(STREAK_X, i).read()).fadd_m(f2));
        wf(v0, 4, xi(arr(STREAK_Y, i).read()).fadd_m(f3));
        wf(v0, 8, xi(arr(STREAK_Z, i).read()).fadd(q));
        crate::common::wr::<u32>(v1, 0x18, (al << 23) | 0xD2_D2E6);
        g::<u32>(TEMP_VERTICES_STORED).write(nv + 2);
        wf(v1, 0, x(f2).fadd_m(tx));
        wf(v1, 4, x(f3).fadd_m(ty));
        wf(v1, 8, x(f1).fmul_m(k(0x3ECC_CCCD)).fadd_m(k(0x3DCC_CCCD)).fadd(q).fadd_m(tz));
        g::<u32>(TEMP_INDICES_STORED).write(ni + 2);
        let r5 = xi(rand_at(0x72_B4BF)).fmul_m(k(RAND_SCALE));
        let fall = r5.fmul_m(k(0x3DB8_51EC)).fadd_m(k(0x3C23_D70A));
        arr(STREAK_Z, i).write(ftol32(xi(arr(STREAK_Z, i).read()).fsub(fall)));
        let drop = random_in_range(2, 5);
        let a8 = alpha_at(i).read();
        alpha_at(i).write(if i32::from(a8) > drop { a8.wrapping_sub(drop as u8) } else { 0 });
    }
    if g::<u32>(TEMP_INDICES_STORED).read() != 0 {
        rw_render_state_set(8, 0);
        rw_render_state_set(6, 1);
        rw_render_state_set(0xE, 0);
        rw_render_state_set(0x10, 1);
        rw_render_state_set(0xA, 5);
        rw_render_state_set(0xB, 6);
        rw_render_state_set(0xC, 1);
        rw_render_state_set(1, 0);
        if !rw_im3d_transform(TEMP_RENDER_VERTICES, g::<u32>(TEMP_VERTICES_STORED).read(), 0, 8).is_null() {
            rw_im3d_render_indexed_primitive(1, TEMP_BUFFER_INDICES, g::<u32>(TEMP_INDICES_STORED).read());
            rw_im3d_end();
        }
        rw_render_state_set(8, 1);
        rw_render_state_set(6, 1);
        rw_render_state_set(0xA, 5);
        rw_render_state_set(0xB, 6);
        rw_render_state_set(0xE, 0);
        rw_render_state_set(0xC, 0);
    }
    g::<u32>(TEMP_INDICES_STORED).write(0);
    g::<u32>(TEMP_VERTICES_STORED).write(0);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_RENDER_RAIN_STREAKS];
