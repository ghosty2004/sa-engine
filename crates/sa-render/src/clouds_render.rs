//! WRITER-MVP-G: the `CClouds` render passes (moving fog). Ported from the 1.0 US bytes; gta-reversed
//! `Clouds.cpp` as reading aid. x87 arithmetic follows the original's instruction order on the
//! `sa_math::x87::X87` model (every f32 store rounds). Layout of `ms_mf` (0xC6C158): see `clouds`.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{THE_CAMERA, ftol32, g, jne, jp, k, rd, wr, x, xi};

orig! {
    /// `CCullZones::CamNoRain()` (0x72DDB0, bool in AL)
    fn cam_no_rain() -> u8 = cdecl 0x72DDB0 => crate::cullzones::cam_no_rain;
    /// `CCullZones::PlayerNoRain()` (0x72DDC0, bool in AL)
    fn player_no_rain() -> u8 = cdecl 0x72DDC0 => crate::cullzones::player_no_rain;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPlaceable::GetAtDirection()` (0x50E420, CVector by hidden pointer)
    fn get_at_direction(this: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x50E420 => sa_world::world::entity::get_at_direction;
    /// `CPlaceable::GetRightDirection()` (0x41CC70, CVector by hidden pointer)
    fn get_right_direction(this: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x41CC70 => sa_world::world::entity::get_right_direction;
    /// `CPostEffects::ImmediateModeRenderStatesStore()` (0x700CC0)
    fn rs_store() = cdecl 0x700CC0 => crate::mvpd_r3::immediate_mode_render_states_store;
    /// `CPostEffects::ImmediateModeRenderStatesSet()` (0x700D70)
    fn rs_set_immediate() = cdecl 0x700D70 => crate::posteffects::immediate_mode_render_states_set;
    /// `CPostEffects::ImmediateModeRenderStatesReStore()` (0x700E00)
    fn rs_restore() = cdecl 0x700E00 => crate::posteffects::immediate_mode_render_states_restore;
    /// `RwRenderStateSet(state, value)` (`RwGlobals` +0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwIm3DTransform(verts, count, ltm, flags)` (0x7EF450)
    fn rw_im3d_transform(verts: u32, count: u32, ltm: u32, flags: u32) -> GPtr<u8> = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderPrimitive(primType)` (0x7EF6B0)
    fn rw_im3d_render_primitive(prim: u32) -> u32 = cdecl 0x7EF6B0 => unported;
    /// `RwIm3DEnd()` (0x7EF520)
    fn rw_im3d_end() -> u32 = cdecl 0x7EF520 => unported;
    /// `CClouds::MovingFog_Update()` (0x716B10)
    fn moving_fog_update() = cdecl 0x716B10 => unported;
}

/// `CWeather::Foggyness_SF`.
const FOGGYNESS_SF: usize = 0xC8_12FC;
/// `CGame::currArea`.
const CURR_AREA: usize = 0xB7_2914;
/// `CWeather::UnderWaterness`.
const UNDER_WATERNESS: usize = 0xC8_132C;
/// The moving-fog fade (`.data`, initially 1.0).
const MF_FADE: usize = 0x8D_5798;
/// The 0.535 the underwaterness is compared with (0x8D514C, `.data`).
const MF_UNDERWATER_LIMIT: usize = 0x8D_514C;
/// `CTimeCycle::m_CurrentColours` fog red / green / blue words.
const FOG_RED: usize = 0xB7_C4CA;
const FOG_GREEN: usize = 0xB7_C4CC;
const FOG_BLUE: usize = 0xB7_C4CE;
/// `ms_mf` fields: slot flags, positions (CVector), sizes, intensities, primitive indices (6 dwords).
const MF_SLOTS: usize = 0xC6_C158;
const MF_POS: usize = 0xC6_C2B8;
const MF_SIZE: usize = 0xC6_D320;
const MF_INTENSITY: usize = 0xC6_D898;
const MF_PRIM_INDICES: usize = 0xC6_E93C;
/// `TempBufferRenderVertices` (RwIm3DVertex, 0x24 bytes).
const TEMP_RENDER_VERTICES: u32 = 0xC4_D958;
/// `CTimer::ms_fTimeStep`.
const TIME_STEP: usize = 0xB7_CB5C;

/// `CClouds::MovingFogRender()` (0x716C90). Returns at once when `Foggyness_SF` is 0, in an interior
/// (`CGame::currArea`) or with the player ped's area code (+0x2F) set. The fade 0x8D5798 moves by
/// `timeStep / 300` (`timeStep * 0.0033333334`): down (clamped at 0) while both `CamNoRain` and
/// `PlayerNoRain`, else up (clamped at 1, NaN kept). With `UnderWaterness >= 0.535` (not unordered)
/// the fade is reset to 0 and nothing drawn; a zero fade draws nothing. Neither early return calls
/// `MovingFog_Update`.
///
/// Drawing: the camera's at and right vectors, the immediate-mode states (stored, set), then
/// `ZTESTENABLE = 1`, the cloud-mask texture's raster, `TEXTUREFILTER = 2`. The colour is the time
/// cycle's fog colour + 132 per channel (clamped at 255, signed compares) with alpha
/// `ftol(Foggyness_SF * intensity * fade)` (no clamp, shifted in whole). Each used fog slot (350)
/// emits the six corners listed by `ms_mf.m_nPrimIndices` (corner 0: `p + right*s + at*s` with
/// UV (0, 0), 1: `p + right*s - at*s` (1, 0), 2: `p - right*s - at*s` (1, 1), 3: `p - right*s + at*s`
/// (0, 1), with `s = size / 2`; another index re-emits the previous vertex) into
/// `TempBufferRenderVertices`, flushing (`RwIm3DTransform(.., NULL, 9)`, triangle list, end) at 2046
/// vertices and once more after the loop. Then the states are restored and `MovingFog_Update` runs
/// (tail call). The corner temporaries start as the original's uninitialised stack slots (0 here).
#[hook(addr = 0x716C90, conv = cdecl, name = "CClouds::MovingFogRender", expect = "D9 05 FC 12 C8 00 83 EC 60",
       route = "L2", hazards = "heap-writes",
       recs = "grove-walk",
       note = "writes TempBufferRenderVertices 0xC4D958.. (2046 x 0x24) and the fade 0x8D5798; tail-calls MovingFog_Update 0x716B10")]
pub fn moving_fog_render() {
    let fog = || x(g::<f32>(FOGGYNESS_SF).read());
    let zero = x(0.0);
    if !jp(fog(), zero, 0x44) {
        return;
    }
    if g::<u32>(CURR_AREA).read() != 0 {
        return;
    }
    if rd::<u8>(find_player_ped(-1), 0x2F) != 0 {
        return;
    }
    // 1/300 (0x864E08)
    let step = x(g::<f32>(TIME_STEP).read()).fmul_m(k(0x3B5A_740E)).to_f32();
    let fade = g::<f32>(MF_FADE);
    if cam_no_rain() != 0 && player_no_rain() != 0 {
        let v = x(fade.read()).fsub_m(step);
        fade.write(v.to_f32());
        if !jp(v, zero, 5) {
            fade.write(0.0);
        }
    } else {
        let v = x(fade.read()).fadd_m(step);
        fade.write(v.to_f32());
        if !v.fcom(x(1.0)).test_any(0x41) {
            fade.write(1.0);
        }
    }
    if !x(g::<f32>(UNDER_WATERNESS).read()).fcom(x(g::<f32>(MF_UNDERWATER_LIMIT).read())).test_any(1) {
        fade.write(0.0);
        return;
    }
    if !jp(x(fade.read()), zero, 0x44) {
        return;
    }

    let cam = GPtr::<u8>::from_addr(THE_CAMERA);
    let mut at = [0f32; 3];
    let mut right = [0f32; 3];
    get_at_direction(cam, GPtr::from_raw(at.as_mut_ptr()));
    get_right_direction(cam, GPtr::from_raw(right.as_mut_ptr()));
    rs_store();
    rs_set_immediate();
    rw_render_state_set(6, 1);
    let mask = g::<GPtr<u8>>(0xC6_AA7C).read();
    rw_render_state_set(1, rd::<u32>(mask, 0));
    rw_render_state_set(9, 2);

    let chan = |a: usize| (u32::from(g::<u16>(a).read()) + 0x84).min(0xFF);
    let (red, green, blue) = (chan(FOG_RED), chan(FOG_GREEN), chan(FOG_BLUE));
    let mut n: u32 = 0; // esi
    // the corner temporaries [esp+0x1C..0x2C] (x, y, z, u, v)
    let (mut cx, mut cy, mut cz, mut cu, mut cv) = (0f32, 0f32, 0f32, 0f32, 0f32);
    for i in 0..350usize {
        if g::<u8>(MF_SLOTS + i).read() == 0 {
            continue;
        }
        let s = x(g::<f32>(MF_SIZE + 4 * i).read()).fmul_m(0.5);
        let p = [
            g::<f32>(MF_POS + 0xC * i).read(),
            g::<f32>(MF_POS + 0xC * i + 4).read(),
            g::<f32>(MF_POS + 0xC * i + 8).read(),
        ];
        let ua = [x(at[0]).fmul(s).to_f32(), x(at[1]).fmul(s).to_f32(), x(at[2]).fmul(s).to_f32()];
        let ur = [
            x(right[0]).fmul(s).to_f32(),
            x(right[1]).fmul(s).to_f32(),
            x(right[2]).fmul(s).to_f32(),
        ];
        let alpha = ftol32(fog().fmul_m(g::<f32>(MF_INTENSITY + 4 * i).read()).fmul_m(fade.read())) as u32;
        let colour = (((alpha << 8 | red) << 8 | green) << 8) | blue;
        for j in 0..6usize {
            match g::<u32>(MF_PRIM_INDICES + 4 * j).read() {
                0 => {
                    cv = 0.0;
                    cx = x(ur[0]).fadd_m(ua[0]).fadd_m(p[0]).to_f32();
                    cy = x(ur[1]).fadd_m(ua[1]).fadd_m(p[1]).to_f32();
                    cz = x(ur[2]).fadd_m(ua[2]).fadd_m(p[2]).to_f32();
                    cu = 0.0;
                }
                1 => {
                    cu = 1.0;
                    cv = 0.0;
                    cx = x(ur[0]).fadd_m(p[0]).fsub_m(ua[0]).to_f32();
                    cy = x(ur[1]).fadd_m(p[1]).fsub_m(ua[1]).to_f32();
                    cz = x(ur[2]).fadd_m(p[2]).fsub_m(ua[2]).to_f32();
                }
                2 => {
                    cu = 1.0;
                    cv = 1.0;
                    cx = x(p[0]).fsub_m(ur[0]).fsub_m(ua[0]).to_f32();
                    cy = x(p[1]).fsub_m(ur[1]).fsub_m(ua[1]).to_f32();
                    cz = x(p[2]).fsub_m(ur[2]).fsub_m(ua[2]).to_f32();
                }
                3 => {
                    cv = 1.0;
                    cx = x(p[0]).fsub_m(ur[0]).fadd_m(ua[0]).to_f32();
                    cy = x(p[1]).fsub_m(ur[1]).fadd_m(ua[1]).to_f32();
                    cz = x(p[2]).fsub_m(ur[2]).fadd_m(ua[2]).to_f32();
                    cu = 0.0;
                }
                _ => {}
            }
            let v = GPtr::<u8>::from_addr(TEMP_RENDER_VERTICES.wrapping_add(n * 0x24) as usize);
            wr::<f32>(v, 0, cx);
            wr::<f32>(v, 4, cy);
            wr::<f32>(v, 8, cz);
            wr::<u32>(v, 0x18, colour);
            wr::<f32>(v, 0x1C, cu);
            wr::<f32>(v, 0x20, cv);
            n += 1;
            if n == 0x7FE {
                if !rw_im3d_transform(TEMP_RENDER_VERTICES, n, 0, 9).is_null() {
                    rw_im3d_render_primitive(3);
                    rw_im3d_end();
                }
                n = 0;
            }
        }
    }
    if n as i32 > 0 && !rw_im3d_transform(TEMP_RENDER_VERTICES, n, 0, 9).is_null() {
        rw_im3d_render_primitive(3);
        rw_im3d_end();
    }
    rs_restore();
    moving_fog_update();
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_MOVING_FOG_RENDER, HOOK_CLOUDS_RENDER, HOOK_RENDER_SKY_POLYS, HOOK_RENDER_BOTTOM_FROM_HEIGHT, HOOK_VOLUMETRIC_CLOUDS_RENDER];

// ---- WRITER-RENDER-BIG: CClouds::Render, RenderSkyPolys, RenderBottomFromHeight, VolumetricCloudsRender ----

orig! {
    /// `CGame::CanSeeOutSideFromCurrArea()` (0x53C4A0, bool in AL)
    fn can_see_outside_from_curr_area() -> u32 = cdecl 0x53C4A0 => sa_core::game::can_see_outside_from_curr_area;
    /// `CSprite::InitSpriteBuffer()` (0x70CFB0)
    fn init_sprite_buffer() = cdecl 0x70CFB0 => crate::sprite::init_sprite_buffer;
    /// `CSprite::FlushSpriteBuffer()` (0x70CF20)
    fn flush_sprite_buffer() = cdecl 0x70CF20 => crate::sprite::flush_sprite_buffer;
    /// `CSprite::CalcScreenCoors(const RwV3d&, RwV3d* out, float* w, float* h, bool, bool)` (0x70CE30)
    fn calc_screen_coors(inp: GPtr<f32>, out: GPtr<f32>, w: GPtr<f32>, h: GPtr<f32>, max_vis: u8, min_vis: u8) -> u8 = cdecl 0x70CE30 => crate::sprite::calc_screen_coors;
    /// `CSprite::RenderOneXLUSprite(x, y, z, halfW, halfH, r, g, b, intensity, rhw, a, uDir, vDir)`
    /// (0x70D000); every argument as the raw dword the original pushes
    fn render_one_xlu_sprite(px: u32, py: u32, pz: u32, hw: u32, hh: u32, r: u32, gg: u32, b: u32, intensity: u32, rhw: u32, a: u32, u_dir: u32, v_dir: u32) = cdecl 0x70D000 => unported;
    /// `CSprite::RenderBufferedOneXLUSprite(x, y, z, w, h, r, g, b, intensity, recipNearZ, alpha)` (0x70E4A0)
    fn render_buffered_one_xlu_sprite(px: f32, py: f32, pz: f32, w: f32, h: f32, r: u32, gg: u32, b: u32, intensity: u32, recip_z: u32, alpha: u32) = cdecl 0x70E4A0 => crate::mvpd_r3::render_buffered_one_xlu_sprite;
    /// `CSprite::RenderBufferedOneXLUSprite_Rotate_Dimension(x, y, z, w, h, r, g, b, intensity, recipNearZ,
    /// angle, alpha)` (0x70EAB0)
    fn render_buffered_one_xlu_sprite_rotate_dimension(px: f32, py: f32, pz: f32, w: f32, h: f32, r: u32, gg: u32, b: u32, intensity: u32, recip_z: u32, angle: f32, alpha: u32) = cdecl 0x70EAB0 => crate::mvpd_r5::render_buffered_one_xlu_sprite_rotate_dimension;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<sa_math::vector::CVector>) = thiscall 0x59C910 => sa_world::collision::data::normalise_port;
    /// `RwIm3DRenderIndexedPrimitive(primType, indices, numIndices)` (0x7EF550)
    fn rw_im3d_render_indexed_primitive(prim: u32, indices: u32, n: u32) -> u32 = cdecl 0x7EF550 => unported;
}

/// `CClock` game clock: seconds (u16), minutes, hours, days (bytes).
const CLOCK_SECONDS: usize = 0xB7_0150;
const CLOCK_MINUTES: usize = 0xB7_0152;
const CLOCK_HOURS: usize = 0xB7_0153;
const CLOCK_DAYS: usize = 0xB7_0154;
/// `CCoronas::SunBlockedByClouds` (bool).
const SUN_BLOCKED_BY_CLOUDS: usize = 0xC3_E030;
/// `CCoronas::MoonSize` (int32, converted as unsigned).
const MOON_SIZE: usize = 0x8D_4B60;
/// `CDraw::ms_fFarClipZ`.
const FAR_CLIP_Z: usize = 0xC3_EF9C;
/// `CWeather::Foggyness`, `CWeather::CloudCoverage`, `CWeather::ExtraSunnyness`, `CWeather::Rainbow`.
const FOGGYNESS: usize = 0xC8_1300;
const CLOUD_COVERAGE: usize = 0xC8_1304;
const EXTRA_SUNNYNESS: usize = 0xC8_12F8;
const RAINBOW: usize = 0xC8_12E4;
/// `CWeather::OldWeatherType`, `CWeather::NewWeatherType` (int16).
const OLD_WEATHER_TYPE: usize = 0xC8_1320;
const NEW_WEATHER_TYPE: usize = 0xC8_131C;
/// `gpCoronaTexture[0]` (star), `[2]` (moon), `gpMoonMask`, `gpCloudTex` (RwTexture*, raster at +0).
const CORONA_TEX_STAR: usize = 0xC3_E000;
const CORONA_TEX_MOON: usize = 0xC3_E008;
const MOON_MASK_TEX: usize = 0xC6_AA74;
const CLOUD_TEX: usize = 0xC6_AA78;
/// `CClouds::ms_cameraRoll`.
const CAMERA_ROLL: usize = 0xC6_AA64;
/// `TheCamera.m_matrix` (CMatrix*).
const CAM_MATRIX: usize = 0xB6_F03C;
/// The R* logo star tables (9 floats each, `.data`): y and z offsets, sizes.
const STAR_Y: usize = 0x8D_55EC;
const STAR_Z: usize = 0x8D_5610;
const STAR_SIZE: usize = 0x8D_5634;
/// The low-cloud offsets (12 floats each, `.data`).
const LOW_CLOUD_X: usize = 0x8D_5394;
const LOW_CLOUD_Y: usize = 0x8D_53C4;
const LOW_CLOUD_Z: usize = 0x8D_53F4;
/// The rainbow line colours (6 bytes each, `.data`): red, green, blue.
const RAINBOW_R: usize = 0x8D_5370;
const RAINBOW_G: usize = 0x8D_5378;
const RAINBOW_B: usize = 0x8D_5380;
/// The streak line's two vertex indices (`.data`, RwImVertexIndex[2]).
const STREAK_INDICES: u32 = 0x8D_579C;
/// `CTimeCycle::m_CurrentColours` low-cloud red / green / blue words.
const LOW_CLOUDS_RED: usize = 0xB7_C4FC;
const LOW_CLOUDS_GREEN: usize = 0xB7_C4FE;
const LOW_CLOUDS_BLUE: usize = 0xB7_C500;
/// `CTimer::m_snTimeInMilliseconds`.
const TIME_IN_MS: usize = 0xB7_CB84;

fn rs(state: u32, value: u32) {
    rw_render_state_set(state, value);
}

/// The raster of the texture whose pointer is stored at `slot` (`RwTextureGetRaster`, +0).
fn raster_of(slot: usize) -> u32 {
    rd::<u32>(g::<GPtr<u8>>(slot).read(), 0)
}

fn rand_at(site: u32) -> i32 {
    sa_core::orig::at_site(site, sa_core::crt::rand)
}

/// `fld [a]; fcomp [b]; fnstsw ax; test ah, 41h; jne` → `[b]`, else `[a]`: `[a]` when it is larger;
/// ties and unordered give `[b]`.
fn pick_larger(a: usize, b: usize) -> X87 {
    if jne(x(g::<f32>(a).read()), x(g::<f32>(b).read()), 0x41) { x(g::<f32>(b).read()) } else { x(g::<f32>(a).read()) }
}

/// `fild` of the moon size dword as unsigned (`+ 2^32` when negative), doubled, `+ 4`.
fn moon_size() -> X87 {
    let m = g::<u32>(MOON_SIZE).read();
    let mut v = xi(m as i32);
    if (m as i32) < 0 {
        v = v.fadd_m(k(0x4F80_0000));
    }
    v.fadd(v).fadd_m(4.0)
}

/// The dword a `fstp dword` stores.
fn bits(v: X87) -> u32 {
    v.to_f32().to_bits()
}

/// `CSprite::CalcScreenCoors(in, &out, &w, &h, false, true)` on the frame slots (fresh pointers per call).
fn calc(inp: &mut [u32; 3], out: &mut [u32; 3], w: &mut f32, h: &mut f32) -> bool {
    calc_screen_coors(
        GPtr::<u32>::from_raw(inp.as_mut_ptr()).cast(),
        GPtr::<u32>::from_raw(out.as_mut_ptr()).cast(),
        GPtr::from_raw(w),
        GPtr::from_raw(h),
        0,
        1,
    ) != 0
}

/// `CClouds::Render()` (0x713950). Nothing unless `CGame::CanSeeOutSideFromCurrArea`. Clears
/// `CCoronas::SunBlockedByClouds`, sets ZWRITE 0, ZTEST 0, VERTEXALPHA 1, SRCBLEND/DESTBLEND ONE, inits
/// the sprite buffer, then (each part follows the bytes, which differ from gta-reversed in places):
///
/// - **Moon:** `d = ftol(|(h*60 + m) + s * (1/60) - 220|)` (sum and difference rounded on the x87,
///   `fabs` by `test ah, 5; jp`, so a NaN keeps its sign); `d < 220` (signed) →
///   `c = ftol((1 - max(CloudCoverage, Foggyness)) * (220 - d))` (`CloudCoverage` when larger, else
///   `Foggyness`), `c > 0`. The point is the camera position with y - 100 and z + 15 (x copied raw).
///   After `CalcScreenCoors(.., false, true)`: `days / 31` (f32), three `RenderOneXLUSprite`s at
///   `(out.x, out.y, FarClipZ)` with `rhw = 1 / FarClipZ` and `ms = 2 * MoonSize + 4` (MoonSize unsigned):
///   black `(w*ms, h*ms)` intensity 255; the mask (`gpMoonMask`) at `(out.x + (w*ms)*5.4 * (days/31 -
///   0.5), out.y + (h*ms)*0.7)` sized `(w*ms*1.7, 1.7*h*ms)` intensity 0 (the bytes use h*ms*0.7 for y,
///   not the mask size, and x carries the day term); the moon texture (`gpCoronaTexture[2]`, SRCBLEND
///   DESTALPHA, ZWRITE 0) colour `(c, c, ftol(c * 0.85))` (`c`, not 220 - d: the original reused the
///   slot). Blend states SRCALPHA/ONE before the first two, ONE/ONE after the last.
/// - **R\* logo** (hours 22..=23 and 0..=5, the hours byte re-read): `t = minutes * 255 / 60` at hour
///   22, `(60 - minutes) * 255 / 60` at hour 5 (signed division; `t == 0` skips), else 255;
///   `lc = ftol((1 - max(CloudCoverage, Foggyness)) * t)`. 11 stars (`gpCoronaTexture[0]`): x offset 100
///   (-100 from star 9), y `cam.y - Y[i%9] * 90`, z `Z[i%9] * 80 + (cam.z + 10)`; on screen,
///   `rhw = 1 / out.z`, `rand()` (0x713EA4), colour `ftol((1 - (rand & 31) * 0.015) * lc)`, size `(w, h) *
///   S[i%9] * 0.8`, buffered. Then `rand()` (0x713F7F, before the screen test) gives the big star's colour
///   `ftol(((rand & 127) * 0.0015625 + 0.5) * lc)`; at `(cam.x + 100, cam.y - 90, cam.z + 10)`, size `(w,
///   h) * 5`, `RenderOneXLUSprite`. Flush (only on this path).
/// - **Low clouds:** balance `max(ExtraSunnyness, max(Foggyness, CloudCoverage))` (a larger first operand
///   wins; ties and NaN take the second), colour bytes `ftol(low-cloud colour * (1 - balance))` (the
///   original stores only the low byte of each). `gpCloudTex`; `ms_cameraRoll` = `atan2(right.z,
///   ±|right.xy|)` (negated when up.z < 0) with a camera matrix, else 0. 12 clouds at `(cam.x + X*800,
///   cam.y + Y*800, Z*60 + 40)` (no cam.z), size `(w*320, h*40)`, rotated buffered sprites; flush.
/// - **Rainbow** (`Rainbow != 0`; unordered draws): 6 lines at `cam + (i*1.5, 100, 5)`, size `(w+w,
///   h*50)`, colour `ftol(table * Rainbow)`; flush.
/// - SRCBLEND SRCALPHA, DESTBLEND INVSRCALPHA; **streak** at hours < 5 with both weather types in {0, 2,
///   6, 11, 13, 17} and `t % 8192 < 800`: with `q = (t >> 13) & 63`, `size = normalise(((q%7)-3)*0.1,
///   ((q&7)-4)*0.1, 1)`, `dir = normalise(q%9-5, q%10-5, 0.1)`, `base = dir*1000 + cam` (base.y kept
///   unrounded on the x87 stack), `s = (400 - t%8192) * 2`: two vertices `base + size*s` (colour
///   0xE1FFFFFF) and `base + size*(s+50)` (0x00FFFFFF) in `TempBufferRenderVertices`,
///   `RwIm3DTransform(.., 2, NULL, 0x18)`, an indexed polyline (the two indices at 0x8D579C), `RwIm3DEnd`
///   (tail call).
#[hook(addr = 0x713950, conv = cdecl, name = "CClouds::Render", expect = "83 EC 68 E8 48 8B E2 FF",
       route = "L2", hazards = "x87-transcendental,heap-writes",
       recs = "grove-walk",
       note = "rand at 0x713EA4 / 0x713F7F; FSQRT+FPATAN camera roll; writes ms_cameraRoll 0xC6AA64, SunBlockedByClouds 0xC3E030, TempBufferRenderVertices 0xC4D958..0xC4D9A0 and the sprite buffer through its callees")]
pub fn clouds_render() {
    if can_see_outside_from_curr_area() as u8 == 0 {
        return;
    }
    g::<u8>(SUN_BLOCKED_BY_CLOUDS).write(0);
    rs(8, 0);
    rs(6, 0);
    rs(12, 1);
    rs(10, 2);
    rs(11, 2);
    init_sprite_buffer();

    let far = || g::<u32>(FAR_CLIP_Z).read();
    let mut inp = [0u32; 3];
    let mut out = [0u32; 3];
    let mut w = 0f32;
    let mut h = 0f32;
    let f = f32::from_bits;
    let one = || x(1.0);
    let cam = |off: isize| rd::<f32>(sa_world::world::common::cam_pos().cast(), off);

    // ---- moon (0x7139B2..0x713D2A)
    let hours = g::<u8>(CLOCK_HOURS).read();
    let hm = i32::from(hours) * 60 + i32::from(g::<u8>(CLOCK_MINUTES).read());
    let secs = i32::from(g::<u16>(CLOCK_SECONDS).read());
    // 1/60 (0x859044), 220 (0x863994)
    let mut dt = xi(hm).fadd(xi(secs).fmul_m(k(0x3C88_8889))).fsub_m(220.0);
    if !jp(dt, x(0.0), 5) {
        dt = dt.fchs();
    }
    let d = ftol32(dt);
    if d < 0xDC {
        let c = ftol32(one().fsub(pick_larger(CLOUD_COVERAGE, FOGGYNESS)).fmul(xi(0xDC - d)));
        if c > 0 {
            inp[0] = rd::<u32>(sa_world::world::common::cam_pos().cast(), 0);
            inp[1] = bits(x(cam(4)).fsub_m(100.0));
            inp[2] = bits(x(cam(8)).fadd_m(15.0));
            if calc(&mut inp, &mut out, &mut w, &mut h) {
                // 1/31 (0x87287C)
                let days = xi(i32::from(g::<u8>(CLOCK_DAYS).read())).fmul_m(k(0x3D04_2108)).to_f32();
                rs(1, 0);
                rs(10, 5);
                rs(11, 2);
                let ms = moon_size();
                let rhw = bits(one().fdiv_m(f(far())));
                render_one_xlu_sprite(out[0], out[1], far(), bits(ms.fmul_m(w)), bits(x(h).fmul(ms)), 0, 0, 0, 0xFF, rhw, 0xFF, 0, 0);

                rs(1, raster_of(MOON_MASK_TEX));
                rs(10, 5);
                rs(11, 2);
                let ms = moon_size();
                let hms = x(h).fmul(ms);
                let wms = ms.fmul_m(w).to_f32();
                let rhw = bits(one().fdiv_m(f(far())));
                // 1.7 (0x859B58), 0.7 (0x872838), 5.4 (0x858C50)
                let mask_h = bits(x(k(0x3FD9_999A)).fmul(hms));
                let mask_w = bits(x(wms).fmul_m(k(0x3FD9_999A)));
                let my = bits(hms.fmul_m(k(0x3F33_3334)).fadd_m(f(out[1])));
                let mx = bits(x(wms).fmul_m(k(0x40AC_CCCD)).fmul(x(days).fsub_m(0.5)).fadd_m(f(out[0])));
                render_one_xlu_sprite(mx, my, far(), mask_w, mask_h, 0, 0, 0, 0, rhw, 0xFF, 0, 0);

                rs(10, 7);
                rs(11, 2);
                rs(8, 0);
                rs(1, raster_of(CORONA_TEX_MOON));
                let ms = moon_size();
                let rhw = bits(one().fdiv_m(f(far())));
                // 0.85 (0x862E1C)
                let blue = ftol32(xi(c).fmul_m(k(0x3F59_999A))) as u32;
                let hh = bits(x(h).fmul(ms));
                let hw = bits(ms.fmul_m(w));
                render_one_xlu_sprite(out[0], out[1], far(), hw, hh, c as u32, c as u32, blue, 0xFF, rhw, 0xFF, 0, 0);
                rs(10, 2);
                rs(11, 2);
            }
        }
    }

    // ---- R* logo (0x713D2A..0x71401E)
    let hours = g::<u8>(CLOCK_HOURS).read();
    if !(6..22).contains(&hours) {
        let t = if hours == 22 || hours == 5 {
            let m = i32::from(g::<u8>(CLOCK_MINUTES).read());
            let n = if hours == 22 { m } else { 60 - m };
            (n * 0xFF) / 60
        } else {
            0xFF
        };
        if t != 0 {
            let lc = ftol32(one().fsub(pick_larger(CLOUD_COVERAGE, FOGGYNESS)).fmul(xi(t)));
            rs(1, raster_of(CORONA_TEX_STAR));
            for i in 0..11usize {
                let offx: f32 = if i >= 9 { -100.0 } else { 100.0 };
                inp[0] = bits(x(offx).fadd_m(cam(0)));
                let zc = x(cam(8)).fadd_m(10.0);
                let idx = i % 9;
                // 90 (0x85991C), 80 (0x858A14)
                inp[1] = bits(x(g::<f32>(STAR_Y + 4 * idx).read()).fmul_m(90.0).fsubr_m(cam(4)));
                inp[2] = bits(x(g::<f32>(STAR_Z + 4 * idx).read()).fmul_m(80.0).fadd(zc));
                if !calc(&mut inp, &mut out, &mut w, &mut h) {
                    continue;
                }
                let rhw = bits(one().fdiv_m(f(out[2])));
                let r = rand_at(0x71_3EA4) & 0x1F;
                // 0.015 (0x859F00)
                let cc = ftol32(xi(r).fmul_m(k(0x3C75_C28F)).fsubr_m(1.0).fmul(xi(lc))) as u32;
                let s = g::<f32>(STAR_SIZE + 4 * idx).read();
                // 0.8 (0x858C98)
                let hh = x(h).fmul_m(s).fmul_m(k(0x3F4C_CCCD)).to_f32();
                let hw = x(w).fmul_m(s).fmul_m(k(0x3F4C_CCCD)).to_f32();
                render_buffered_one_xlu_sprite(f(out[0]), f(out[1]), f(out[2]), hw, hh, cc, cc, cc, 0xFF, rhw, 0xFF);
            }
            inp[0] = bits(x(cam(0)).fadd_m(100.0));
            inp[2] = bits(x(cam(8)).fadd_m(10.0));
            inp[1] = bits(x(cam(4)).fsub_m(90.0));
            let r = rand_at(0x71_3F7F) & 0x7F;
            // 0.0015625 (0x859520)
            let sc = ftol32(xi(r).fmul_m(k(0x3ACC_CCCD)).fadd_m(0.5).fmul(xi(lc))) as u32;
            if calc(&mut inp, &mut out, &mut w, &mut h) {
                let rhw = bits(one().fdiv_m(f(out[2])));
                render_one_xlu_sprite(out[0], out[1], out[2], bits(x(w).fmul_m(5.0)), bits(x(h).fmul_m(5.0)), sc, sc, sc, 0xFF, rhw, 0xFF, 0, 0);
            }
            flush_sprite_buffer();
        }
    }

    // ---- low clouds (0x71401E..0x71422F)
    let bal1 = pick_larger(FOGGYNESS, CLOUD_COVERAGE);
    let es = x(g::<f32>(EXTRA_SUNNYNESS).read());
    let bal = if jne(es, bal1, 0x41) { pick_larger(FOGGYNESS, CLOUD_COVERAGE) } else { es };
    let inv = bal.fsubr_m(1.0);
    let cr = ftol32(xi(i32::from(g::<u16>(LOW_CLOUDS_RED).read())).fmul(inv)) as u8;
    let cg = ftol32(xi(i32::from(g::<u16>(LOW_CLOUDS_GREEN).read())).fmul(inv)) as u8;
    let cb = ftol32(xi(i32::from(g::<u16>(LOW_CLOUDS_BLUE).read())).fmul(inv)) as u8;
    rs(1, raster_of(CLOUD_TEX));
    let m = g::<GPtr<u8>>(CAM_MATRIX).read();
    let roll = if m.is_null() {
        x(0.0)
    } else {
        let rx = x(rd::<f32>(m, 0));
        let ry = x(rd::<f32>(m, 4));
        let mut s = rx.fmul(rx).fadd(ry.fmul(ry)).fsqrt();
        if !jp(x(rd::<f32>(m, 0x28)), x(0.0), 5) {
            s = s.fmul_m(-1.0);
        }
        x(rd::<f32>(m, 8)).fpatan(s)
    };
    g::<f32>(CAMERA_ROLL).write(roll.to_f32());
    for i in 0..12usize {
        // 800 (0x8594DC), 60 (0x858B34), 40 (0x858A10)
        inp[0] = bits(x(g::<f32>(LOW_CLOUD_X + 4 * i).read()).fmul_m(800.0).fadd_m(cam(0)));
        inp[1] = bits(x(g::<f32>(LOW_CLOUD_Y + 4 * i).read()).fmul_m(800.0).fadd_m(cam(4)));
        inp[2] = bits(x(g::<f32>(LOW_CLOUD_Z + 4 * i).read()).fmul_m(60.0).fadd_m(40.0));
        if !calc(&mut inp, &mut out, &mut w, &mut h) {
            continue;
        }
        let rhw = bits(one().fdiv_m(f(out[2])));
        let hh = x(h).fmul_m(40.0).to_f32();
        let hw = x(w).fmul_m(320.0).to_f32();
        let angle = g::<f32>(CAMERA_ROLL).read();
        render_buffered_one_xlu_sprite_rotate_dimension(f(out[0]), f(out[1]), f(out[2]), hw, hh, cr.into(), cg.into(), cb.into(), 0xFF, rhw, angle, 0xFF);
    }
    flush_sprite_buffer();

    // ---- rainbow (0x71422F..0x71438C)
    let rb = || g::<f32>(RAINBOW).read();
    if jp(x(rb()), x(0.0), 0x44) {
        rs(1, raster_of(CORONA_TEX_STAR));
        for i in 0..6usize {
            // 1.5 (0x858CE8)
            let ox = xi(i as i32).fmul_m(1.5).to_f32();
            inp[0] = bits(x(ox).fadd_m(cam(0)));
            inp[1] = bits(x(cam(4)).fadd_m(100.0));
            inp[2] = bits(x(cam(8)).fadd_m(5.0));
            if !calc(&mut inp, &mut out, &mut w, &mut h) {
                continue;
            }
            let rhw = bits(one().fdiv_m(f(out[2])));
            let b = ftol32(xi(i32::from(g::<u8>(RAINBOW_B + i).read())).fmul_m(rb())) as u32;
            let gg = ftol32(xi(i32::from(g::<u8>(RAINBOW_G + i).read())).fmul_m(rb())) as u32;
            let r = ftol32(xi(i32::from(g::<u8>(RAINBOW_R + i).read())).fmul_m(rb())) as u32;
            let hh = x(h).fmul_m(50.0).to_f32();
            let hw = x(w).fadd(x(w)).to_f32();
            render_buffered_one_xlu_sprite(f(out[0]), f(out[1]), f(out[2]), hw, hh, r, gg, b, 0xFF, rhw, 0xFF);
        }
        flush_sprite_buffer();
    }

    // ---- streak (0x71438C..)
    rs(10, 5);
    rs(11, 6);
    if g::<u8>(CLOCK_HOURS).read() >= 5 {
        return;
    }
    let sunny = |t: i16| matches!(t, 0 | 2 | 0xD | 6 | 0xB | 0x11);
    if !sunny(g::<i16>(OLD_WEATHER_TYPE).read()) || !sunny(g::<i16>(NEW_WEATHER_TYPE).read()) {
        return;
    }
    let tms = g::<u32>(TIME_IN_MS).read();
    let rem = (tms & 0x1FFF) as i32;
    let q = ((tms >> 13) & 0x3F) as i32;
    if rem >= 0x320 {
        return;
    }
    let c = [cam(0), cam(4), cam(8)];
    // 0.1 (0x858B1C)
    let tenth = k(0x3DCC_CCCD);
    let mut size = [xi(q % 7 - 3).fmul_m(tenth).to_f32(), xi((q & 7) - 4).fmul_m(tenth).to_f32(), 1.0f32];
    vector_normalise(GPtr::<f32>::from_raw(size.as_mut_ptr()).cast());
    let mut dir = [xi(q % 9 - 5).to_f32(), xi(q % 10 - 5).to_f32(), tenth];
    vector_normalise(GPtr::<f32>::from_raw(dir.as_mut_ptr()).cast());
    // 1000 (0x858C4C)
    let bx = x(dir[0]).fmul_m(1000.0).fadd_m(c[0]).to_f32();
    let by = x(dir[1]).fmul_m(1000.0).fadd_m(c[1]); // kept on the x87 stack
    let az = x(dir[2]).fmul_m(1000.0).to_f32();
    let bz = x(c[2]).fadd_m(az).to_f32();
    let s = xi(0x190 - rem);
    let s = s.fadd(s);
    let s50 = s.fadd_m(50.0).to_f32();
    let sx = x(size[0]).fmul(s).to_f32();
    let sy = x(size[1]).fmul(s).to_f32();
    let sz = x(size[2]).fmul(s).to_f32();
    let v0x = x(sx).fadd_m(bx);
    let v0y = x(sy).fadd(by);
    let v0z = x(sz).fadd_m(bz).to_f32();
    let v1x = x(size[0]).fmul_m(s50).fadd_m(bx).to_f32();
    let v1y = x(size[1]).fmul_m(s50).fadd(by).to_f32();
    let s50z = x(size[2]).fmul_m(s50).to_f32();
    let v1z = x(s50z).fadd_m(bz).to_f32();
    g::<f32>(0xC4_D960).write(v0z);
    g::<u32>(0xC4_D970).write(0xE1FF_FFFF);
    g::<u32>(0xC4_D994).write(0x00FF_FFFF);
    g::<f32>(0xC4_D97C).write(v1x);
    g::<f32>(0xC4_D980).write(v1y);
    g::<f32>(0xC4_D958).write(v0x.to_f32());
    g::<f32>(0xC4_D984).write(v1z);
    g::<f32>(0xC4_D95C).write(v0y.to_f32());
    if rw_im3d_transform(TEMP_RENDER_VERTICES, 2, 0, 0x18).is_null() {
        return;
    }
    rw_im3d_render_indexed_primitive(2, STREAK_INDICES, 2);
    rw_im3d_end();
}

/// `TheCamera.m_placement.m_fHeading`.
const CAM_HEADING: usize = 0xB6_F038;
/// `CTimeCycle::m_BelowHorizonGrey` (RGBA bytes).
const BELOW_HORIZON_GREY: usize = 0xB7_CB10;
/// `CTimeCycle::m_CurrentColours` sky top / bottom red, green, blue words (read as words or low bytes).
const SKY_TOP_RED: usize = 0xB7_C4C4;
const SKY_TOP_GREEN: usize = 0xB7_C4C6;
const SKY_TOP_BLUE: usize = 0xB7_C4C8;
const SKY_BOTTOM_RED: usize = 0xB7_C4CA;
const SKY_BOTTOM_GREEN: usize = 0xB7_C4CC;
const SKY_BOTTOM_BLUE: usize = 0xB7_C4CE;
/// `uiTempBufferIndicesStored`, `uiTempBufferVerticesStored`.
const TEMP_INDICES_STORED: usize = 0xC4_B954;
const TEMP_VERTICES_STORED: usize = 0xC4_B950;

orig! {
    /// `CClouds::SetUpOneSkyPoly(CVector, CVector, CVector, CVector, uint8 r1, g1, b1, r2, g2, b2)` (0x713060)
    fn set_up_one_sky_poly(v1x: u32, v1y: u32, v1z: u32, v2x: u32, v2y: u32, v2z: u32, v3x: u32, v3y: u32, v3z: u32, v4x: u32, v4y: u32, v4z: u32, r1: u8, g1: u8, b1: u8, r2: u8, g2: u8, b2: u8) = cdecl 0x713060 => crate::clouds::set_up_one_sky_poly;
    /// `CBrightLights::RenderOutGeometryBuffer()` (0x722150)
    fn bright_lights_render_out_geometry_buffer() = cdecl 0x722150 => unported;
}

/// `CClouds::RenderSkyPolys()` (0x714650). The camera's forward vector (matrix +0x10, copied raw; without
/// a matrix `(-sin heading, cos heading)` by FSIN/FCOS) with z = 0, normalised (`CVector::Normalise`);
/// the camera position copied raw. Blend `f = (pos.z - 25) * 0.0125` (f32), clamped to 1 (when `1 <
/// f`) and 0 (when `0 > f`; NaN kept), then `Foggyness` unless `f > Foggyness` (ties and NaN take
/// `Foggyness`). The below-horizon grey bytes are `ftol(grey * (1 - f) + skyBottom * f)` per channel.
/// Render states: TEXTURERASTER NULL, ZTEST 0, ZWRITE 0, VERTEXALPHA 0, SRCBLEND SRCALPHA, DESTBLEND
/// INVSRCALPHA, FOGENABLE 0, CULLMODE NONE; the temp buffer's vertex and index counts are zeroed.
/// Then five `SetUpOneSkyPoly` quads (top/bottom, bottom, bottom/grey, top, grey; the colours are
/// the low bytes of the time-cycle words), the corners computed in the original's operation order
/// (width factor 1.4, distance 30, heights +0.5, 0, -0.1, -0.3 against the normal's z; every
/// expression below is the compiled one, operands and rounding included, as the x87 trace of the
/// bytes gives it), and `CBrightLights::RenderOutGeometryBuffer` (tail call).
#[allow(unused_variables, clippy::cognitive_complexity)]
#[hook(addr = 0x714650, conv = cdecl, name = "CClouds::RenderSkyPolys", expect = "A1 3C F0 B6 00 83 EC 64",
       route = "L2", hazards = "x87-transcendental,heap-writes",
       recs = "grove-walk",
       note = "FSIN/FCOS of the camera heading only without a camera matrix; fills TempBufferVertices/indices through SetUpOneSkyPoly and flushes through CBrightLights::RenderOutGeometryBuffer")]
pub fn render_sky_polys() {
    let m = g::<GPtr<u8>>(CAM_MATRIX).read();
    let mut n = [0u32; 3];
    if m.is_null() {
        let hd = g::<f32>(CAM_HEADING).read();
        n[0] = bits(x(hd).fsin().fchs());
        n[1] = bits(x(hd).fcos());
    } else {
        n = [rd::<u32>(m, 0x10), rd::<u32>(m, 0x14), rd::<u32>(m, 0x18)];
    }
    n[2] = 0;
    vector_normalise(GPtr::<u32>::from_raw(n.as_mut_ptr()).cast());
    let f = f32::from_bits;
    let b = |v: f32| v.to_bits();
    let (nx, ny, nz) = (f(n[0]), f(n[1]), f(n[2]));
    let f_14 = x(nx).fchs().to_f32();
    let cp = sa_world::world::common::cam_pos().cast::<u8>();
    let (px, py, pz) = (rd::<f32>(cp, 0), rd::<f32>(cp, 4), rd::<f32>(cp, 8));
    // 25 (0x858FE8), 0.0125 (0x85A5C4)
    let blend = x(pz).fsub_m(25.0).fmul_m(k(0x3C4C_CCCD)).to_f32();
    let mut fb = if !jp(x(1.0), x(blend), 5) {
        x(1.0)
    } else if !jne(x(0.0), x(blend), 0x41) {
        x(0.0)
    } else {
        x(blend)
    };
    let fog = g::<f32>(FOGGYNESS).read();
    if jne(fb, x(fog), 0x41) {
        fb = x(fog);
    }
    let grey_rgba = g::<u32>(BELOW_HORIZON_GREY).read();
    let omf = x(1.0).fsub(fb);
    let mix = |gc: u32, sky: usize| {
        ftol32(xi(gc as i32).fmul(omf).fadd(xi(i32::from(g::<u16>(sky).read())).fmul(fb))) as u8
    };
    let grey = [
        mix(grey_rgba & 0xFF, SKY_BOTTOM_RED),
        mix((grey_rgba >> 8) & 0xFF, SKY_BOTTOM_GREEN),
        mix(u32::from(g::<u8>(BELOW_HORIZON_GREY + 2).read()), SKY_BOTTOM_BLUE),
    ];
    rs(1, 0);
    rs(6, 0);
    rs(8, 0);
    rs(12, 0);
    rs(10, 5);
    rs(11, 6);
    rs(14, 0);
    rs(20, 1);
    g::<u32>(TEMP_INDICES_STORED).write(0);
    g::<u32>(TEMP_VERTICES_STORED).write(0);
    let top = (g::<u8>(SKY_TOP_RED).read(), g::<u8>(SKY_TOP_GREEN).read(), g::<u8>(SKY_TOP_BLUE).read());
    let bot = (g::<u8>(SKY_BOTTOM_RED).read(), g::<u8>(SKY_BOTTOM_GREEN).read(), g::<u8>(SKY_BOTTOM_BLUE).read());
    // 1.4 (0x859994), 30 (0x858CA4), 0.5 (0x858B8C), 0.1 (0x858B1C), 0.3 (0x858C24)
    const K1_4: f32 = 1.4;
    const K30: f32 = 30.0;
    const K0_5: f32 = 0.5;
    const K0_1: f32 = 0.1;
    const K0_3: f32 = 0.3;
    let f_c = x(ny).fmul_m(K1_4).to_f32();
    let f_18 = x(f_c).fadd_m(nx).to_f32();
    let f_14_2 = x(f_14).fmul_m(K1_4).fadd_m(ny).to_f32();
    let f_14_3 = x(f_14_2).fmul_m(K30).to_f32();
    let f_10 = x(nz).fmul_m(K30).to_f32();
    let f_3c = x(f_18).fmul_m(K30).fadd_m(px).to_f32();
    let f_38 = x(py).fadd_m(f_14_3).to_f32();
    let f_34 = x(pz).fadd_m(f_10).to_f32();
    let f_c_2 = x(nx).fsub(x(ny).fmul_m(K1_4)).to_f32();
    let f_64_2 = x(nz).to_f32();
    let f_18_2 = x(f_c_2).fmul_m(K30).to_f32();
    let f_14_4 = x(ny).fsub(x(f_14).fmul_m(K1_4)).fmul_m(K30).to_f32();
    let f_30 = x(f_18_2).fadd_m(px).to_f32();
    let f_2c = x(f_14_4).fadd_m(py).to_f32();
    let f_28 = x(nz).fmul_m(K30).fadd_m(pz).to_f32();
    let f_8 = x(f_14).fmul_m(K1_4).to_f32();
    let f_4 = x(nz).to_f32();
    let f_18_3 = x(ny).fmul_m(K1_4).fadd_m(nx).to_f32();
    let f_18_4 = x(f_18_3).fmul_m(K30).to_f32();
    let f_14_5 = x(f_8).fadd_m(ny).fmul_m(K30).to_f32();
    let f_24 = x(f_18_4).fadd_m(px).to_f32();
    let f_20 = x(f_14_5).fadd_m(py).to_f32();
    let f_1c = x(f_4).fadd_m(K0_5).fmul_m(K30).fadd_m(pz).to_f32();
    let f_8_2 = x(f_14).fmul_m(K1_4).to_f32();
    let f_18_5 = x(ny).fmul_m(K1_4).fsubr_m(nx).to_f32();
    let f_c_3 = x(f_18_5).fmul_m(K30).to_f32();
    let f_8_3 = x(ny).fsub_m(f_8_2).fmul_m(K30).to_f32();
    let f_18_6 = x(f_c_3).fadd_m(px).to_f32();
    let f_14_6 = x(f_8_3).fadd_m(py).to_f32();
    let f_10_2 = x(f_64_2).fadd_m(K0_5).fmul_m(K30).fadd_m(pz).to_f32();
    set_up_one_sky_poly(b(f_18_6), b(f_14_6), b(f_10_2), b(f_24), b(f_20), b(f_1c), b(f_30), b(f_2c), b(f_28), b(f_3c), b(f_38), b(f_34), top.0, top.1, top.2, bot.0, bot.1, bot.2);
    let f_8_4 = x(f_14).fmul_m(K1_4).to_f32();
    let f_4_2 = x(nz).to_f32();
    let f_18_7 = x(ny).fmul_m(K1_4).fadd_m(nx).to_f32();
    let f_c_4 = x(f_18_7).fmul_m(K30).to_f32();
    let f_8_5 = x(f_8_4).fadd_m(ny).fmul_m(K30).to_f32();
    let f_18_8 = x(f_c_4).fadd_m(px).to_f32();
    let f_14_7 = x(f_8_5).fadd_m(py).to_f32();
    let f_10_3 = x(f_4_2).fsub_m(K0_1).fmul_m(K30).fadd_m(pz).to_f32();
    let f_8_6 = x(f_14).fmul_m(K1_4).to_f32();
    let f_24_2 = x(ny).fmul_m(K1_4).fsubr_m(nx).to_f32();
    let f_c_5 = x(f_24_2).fmul_m(K30).to_f32();
    let f_8_7 = x(ny).fsub_m(f_8_6).fmul_m(K30).to_f32();
    let f_24_3 = x(f_c_5).fadd_m(px).to_f32();
    let f_20_2 = x(f_8_7).fadd_m(py).to_f32();
    let f_1c_2 = x(f_64_2).fsub_m(K0_1).fmul_m(K30).fadd_m(pz).to_f32();
    let f_c_6 = x(ny).fmul_m(K1_4).fadd_m(nx).to_f32();
    let f_c_7 = x(f_c_6).fmul_m(K30).to_f32();
    let f_8_8 = x(f_14).fmul_m(K1_4).fadd_m(ny).fmul_m(K30).to_f32();
    let f_30_2 = x(f_c_7).fadd_m(px).to_f32();
    let f_2c_2 = x(f_8_8).fadd_m(py).to_f32();
    let f_28_2 = x(nz).fmul_m(K30).fadd_m(pz).to_f32();
    let f_c_8 = x(nx).fsub(x(ny).fmul_m(K1_4)).to_f32();
    let f_c_9 = x(f_c_8).fmul_m(K30).to_f32();
    let f_8_9 = x(ny).fsub(x(f_14).fmul_m(K1_4)).fmul_m(K30).to_f32();
    let f_3c_2 = x(f_c_9).fadd_m(px).to_f32();
    let f_38_2 = x(f_8_9).fadd_m(py).to_f32();
    let f_34_2 = x(f_64_2).fmul_m(K30).fadd_m(pz).to_f32();
    set_up_one_sky_poly(b(f_3c_2), b(f_38_2), b(f_34_2), b(f_30_2), b(f_2c_2), b(f_28_2), b(f_24_3), b(f_20_2), b(f_1c_2), b(f_18_8), b(f_14_7), b(f_10_3), bot.0, bot.1, bot.2, bot.0, bot.1, bot.2);
    let f_8_10 = x(f_14).fmul_m(K1_4).to_f32();
    let f_4_3 = x(nz).to_f32();
    let f_18_9 = x(ny).fmul_m(K1_4).fadd_m(nx).to_f32();
    let f_c_10 = x(f_18_9).fmul_m(K30).to_f32();
    let f_8_11 = x(ny).fadd_m(f_8_10).fmul_m(K30).to_f32();
    let f_18_10 = x(f_c_10).fadd_m(px).to_f32();
    let f_14_8 = x(py).fadd_m(f_8_11).to_f32();
    let f_10_4 = x(pz).fadd(x(f_4_3).fsub_m(K0_3).fmul_m(K30)).to_f32();
    let f_8_12 = x(f_14).fmul_m(K1_4).to_f32();
    let f_24_4 = x(ny).fmul_m(K1_4).fsubr_m(nx).to_f32();
    let f_c_11 = x(f_24_4).fmul_m(K30).to_f32();
    let f_8_13 = x(ny).fsub_m(f_8_12).fmul_m(K30).to_f32();
    let f_24_5 = x(f_c_11).fadd_m(px).to_f32();
    let f_20_3 = x(py).fadd_m(f_8_13).to_f32();
    let f_1c_3 = x(pz).fadd(x(f_64_2).fsub_m(K0_3).fmul_m(K30)).to_f32();
    let f_8_14 = x(f_14).fmul_m(K1_4).to_f32();
    let f_4_4 = x(nz).to_f32();
    let f_30_3 = x(ny).fmul_m(K1_4).fadd_m(nx).to_f32();
    let f_c_12 = x(f_30_3).fmul_m(K30).to_f32();
    let f_8_15 = x(f_8_14).fadd_m(ny).fmul_m(K30).to_f32();
    let f_30_4 = x(f_c_12).fadd_m(px).to_f32();
    let f_2c_3 = x(py).fadd_m(f_8_15).to_f32();
    let f_28_3 = x(pz).fadd(x(f_4_4).fsub_m(K0_1).fmul_m(K30)).to_f32();
    let f_8_16 = x(f_14).fmul_m(K1_4).to_f32();
    let f_3c_3 = x(ny).fmul_m(K1_4).fsubr_m(nx).to_f32();
    let f_c_13 = x(f_3c_3).fmul_m(K30).to_f32();
    let f_8_17 = x(ny).fsub_m(f_8_16).fmul_m(K30).to_f32();
    let f_3c_4 = x(f_c_13).fadd_m(px).to_f32();
    let f_38_3 = x(py).fadd_m(f_8_17).to_f32();
    let f_34_3 = x(pz).fadd(x(f_64_2).fsub_m(K0_1).fmul_m(K30)).to_f32();
    set_up_one_sky_poly(b(f_3c_4), b(f_38_3), b(f_34_3), b(f_30_4), b(f_2c_3), b(f_28_3), b(f_24_5), b(f_20_3), b(f_1c_3), b(f_18_10), b(f_14_8), b(f_10_4), bot.0, bot.1, bot.2, grey[0], grey[1], grey[2]);
    let f_c_14 = x(ny).fmul_m(K1_4).to_f32();
    let f_8_18 = x(f_14).fmul_m(K1_4).to_f32();
    let f_18_11 = x(nx).fchs().to_f32();
    let f_14_9 = x(ny).fchs().to_f32();
    let f_60 = x(nz).fchs().to_f32();
    let f_10_5 = x(nz).fchs().to_f32();
    let f_10_6 = x(f_10_5).to_f32();
    let f_c_15 = x(f_c_14).fadd_m(f_18_11).to_f32();
    let f_8_19 = x(f_14_9).fadd_m(f_8_18).to_f32();
    let f_8_20 = x(f_8_19).fmul_m(K30).to_f32();
    let f_4_5 = x(f_10_6).fadd_m(K0_5).fmul_m(K30).to_f32();
    let f_18_12 = x(f_c_15).fmul_m(K30).fadd_m(px).to_f32();
    let f_14_10 = x(py).fadd_m(f_8_20).to_f32();
    let f_10_7 = x(pz).fadd_m(f_4_5).to_f32();
    let f_20_4 = x(f_14).fmul_m(K1_4).to_f32();
    let f_8_21 = x(ny).fchs().to_f32();
    let f_1c_4 = x(f32::from_bits(f_60.to_bits())).to_f32();
    let f_c_16 = x(ny).fmul_m(K1_4).fsubr(x(nx).fchs()).to_f32();
    let f_8_22 = x(f_8_21).fsub_m(f_20_4).to_f32();
    let f_8_23 = x(f_8_22).fmul_m(K30).to_f32();
    let f_4_6 = x(f_1c_4).fadd_m(K0_5).fmul_m(K30).to_f32();
    let f_24_6 = x(f_c_16).fmul_m(K30).fadd_m(px).to_f32();
    let f_20_5 = x(py).fadd_m(f_8_23).to_f32();
    let f_1c_5 = x(pz).fadd_m(f_4_6).to_f32();
    let f_8_24 = x(f_14).fmul_m(K1_4).to_f32();
    let f_28_4 = x(nz).to_f32();
    let f_c_17 = x(ny).fmul_m(K1_4).fadd_m(nx).to_f32();
    let f_8_25 = x(f_8_24).fadd_m(ny).to_f32();
    let f_8_26 = x(f_8_25).fmul_m(K30).to_f32();
    let f_4_7 = x(f_28_4).fadd_m(K0_5).fmul_m(K30).to_f32();
    let f_30_5 = x(f_c_17).fmul_m(K30).fadd_m(px).to_f32();
    let f_2c_4 = x(py).fadd_m(f_8_26).to_f32();
    let f_28_5 = x(pz).fadd_m(f_4_7).to_f32();
    let f_8_27 = x(f_14).fmul_m(K1_4).to_f32();
    let f_c_18 = x(ny).fmul_m(K1_4).fsubr_m(nx).to_f32();
    let f_8_28 = x(ny).fsub_m(f_8_27).to_f32();
    let f_8_29 = x(f_8_28).fmul_m(K30).to_f32();
    let f_4_8 = x(f_64_2).fadd_m(K0_5).fmul_m(K30).to_f32();
    let f_3c_5 = x(f_c_18).fmul_m(K30).fadd_m(px).to_f32();
    let f_38_4 = x(py).fadd_m(f_8_29).to_f32();
    let f_34_4 = x(pz).fadd_m(f_4_8).to_f32();
    set_up_one_sky_poly(b(f_3c_5), b(f_38_4), b(f_34_4), b(f_30_5), b(f_2c_4), b(f_28_5), b(f_24_6), b(f_20_5), b(f_1c_5), b(f_18_12), b(f_14_10), b(f_10_7), top.0, top.1, top.2, top.0, top.1, top.2);
    let f_14_11 = x(f_14).fmul_m(K1_4).to_f32();
    let f_8_30 = x(ny).fchs().to_f32();
    let f_10_8 = x(f32::from_bits(f_60.to_bits())).to_f32();
    let f_c_19 = x(ny).fmul_m(K1_4).fadd(x(nx).fchs()).to_f32();
    let f_8_31 = x(f_8_30).fadd_m(f_14_11).to_f32();
    let f_8_32 = x(f_8_31).fmul_m(K30).to_f32();
    let f_4_9 = x(f_10_8).fsub_m(K0_3).fmul_m(K30).to_f32();
    let f_24_7 = x(f_c_19).fmul_m(K30).fadd_m(px).to_f32();
    let f_20_6 = x(py).fadd_m(f_8_32).to_f32();
    let f_1c_6 = x(pz).fadd_m(f_4_9).to_f32();
    let f_c_20 = x(ny).fmul_m(K1_4).to_f32();
    let f_8_33 = x(f_14).fmul_m(K1_4).to_f32();
    let f_18_13 = x(nx).fchs().to_f32();
    let f_14_12 = x(ny).fchs().to_f32();
    let f_4_10 = x(f32::from_bits(f_60.to_bits())).to_f32();
    let f_18_14 = x(f_18_13).fsub_m(f_c_20).to_f32();
    let f_c_21 = x(f_18_14).fmul_m(K30).to_f32();
    let f_8_34 = x(f_14_12).fsub_m(f_8_33).fmul_m(K30).to_f32();
    let f_18_15 = x(f_c_21).fadd_m(px).to_f32();
    let f_14_13 = x(py).fadd_m(f_8_34).to_f32();
    let f_10_9 = x(pz).fadd(x(f_4_10).fsub_m(K0_3).fmul_m(K30)).to_f32();
    let f_8_35 = x(f_14).fmul_m(K1_4).to_f32();
    let f_8_36 = x(f_8_35).fadd_m(ny).to_f32();
    let f_4_11 = x(nz).to_f32();
    let f_4_12 = x(f_4_11).fsub_m(K0_3).to_f32();
    let f_30_6 = x(ny).fmul_m(K1_4).fadd_m(nx).fmul_m(K30).to_f32();
    let f_30_7 = x(f_30_6).fadd_m(px).to_f32();
    let f_2c_5 = x(py).fadd(x(f_8_36).fmul_m(K30)).to_f32();
    let f_28_6 = x(pz).fadd(x(f_4_12).fmul_m(K30)).to_f32();
    let f_c_22 = x(ny).fmul_m(K1_4).to_f32();
    let f_8_37 = x(f_14).fmul_m(K1_4).to_f32();
    let f_3c_6 = x(nx).fsub_m(f_c_22).to_f32();
    let f_c_23 = x(f_3c_6).fmul_m(K30).to_f32();
    let f_8_38 = x(ny).fsub_m(f_8_37).fmul_m(K30).to_f32();
    let f_3c_7 = x(f_c_23).fadd_m(px).to_f32();
    let f_38_5 = x(py).fadd_m(f_8_38).to_f32();
    let f_34_5 = x(pz).fadd(x(f_64_2).fsub_m(K0_3).fmul_m(K30)).to_f32();
    set_up_one_sky_poly(b(f_3c_7), b(f_38_5), b(f_34_5), b(f_30_7), b(f_2c_5), b(f_28_6), b(f_18_15), b(f_14_13), b(f_10_9), b(f_24_7), b(f_20_6), b(f_1c_6), grey[0], grey[1], grey[2], grey[0], grey[1], grey[2]);
    bright_lights_render_out_geometry_buffer();
}

/// `CTimeCycle::m_CurrentColours` fluffy-cloud bottom red / green / blue words.
const FLUFFY_BOTTOM_RED: usize = 0xB7_C502;
const FLUFFY_BOTTOM_GREEN: usize = 0xB7_C504;
const FLUFFY_BOTTOM_BLUE: usize = 0xB7_C506;
/// `CTimeCycle::m_CurrentColours.m_fCloudAlpha` (float).
const CLOUD_ALPHA: usize = 0xB7_C538;
/// The bottom-cloud wind shift (float, `.bss`).
const WIND_SHIFT: usize = 0xC6_E954;
/// `CWeather::Wind`.
const WIND: usize = 0xC8_12F0;
/// `gpCloudMaskTex`.
const CLOUD_MASK_TEX: usize = 0xC6_AA7C;
/// The cloud-bottom height table (80 floats, `.data`) and the fluffy sprites' x / y offsets (30 each).
const BOTTOM_HEIGHTS: usize = 0x8D_5658;
const FLUFFY_X: usize = 0x8D_5818;
const FLUFFY_Y: usize = 0x8D_57A0;
/// `TempBufferIndices` (RwImVertexIndex).
const TEMP_BUFFER_INDICES: u32 = 0xC4_B958;

/// `fcomp 0; test ah, 5; jp` + `fchs`: negated when below zero (NaN kept).
fn abs_jp(v: X87) -> X87 {
    if jp(v, x(0.0), 5) { v } else { v.fchs() }
}

/// The larger of `|cam.x - px|` and `|cam.y - py|` as the original selects it (`fcomp`, `test ah, 41h`,
/// `jne`: ties and NaN take the y distance; each distance is recomputed from memory).
fn horizontal_distance(px: f32, py: f32) -> X87 {
    let cp = sa_world::world::common::cam_pos().cast::<u8>();
    let ax = abs_jp(x(rd::<f32>(cp, 0)).fsub_m(px));
    let ay = abs_jp(x(rd::<f32>(cp, 4)).fsub_m(py));
    if jne(ax, ay, 0x41) { ay } else { ax }
}

fn height(i: i32) -> f32 {
    g::<f32>(BOTTOM_HEIGHTS + 4 * (i % 80) as usize).read()
}

/// `CClouds::RenderBottomFromHeight()` (0x7154B0). Colours `ftol(min(2*r + 20, 255))`,
/// `ftol(min(g*1.5, 255))`, `ftol(min(b*1.5, 255))` of the fluffy-cloud bottom words (`255 < v` takes
/// 255, NaN kept). Nothing when the camera z is below -90 (not unordered). Heights 160 / 190, both
/// raised by `h = (z - 190 - 10) * 0.3` when `h > 0`. The wind shift 0xC6E954 grows by `timeStep *
/// Wind * 0.25`. States: ZWRITE 0, ZTEST 1, VERTEXALPHA 1, FOGENABLE 0, SRCALPHA / INVSRCALPHA, the
/// cloud-mask raster. With `base = ftol(m_fCloudAlpha) != 0`:
///
/// - **Bottom quads**, i = 1..=28 (`T` = the 80-entry height table, indices mod 80): `shift = (T[i+5] *
///   0.5 + 1) * windShift` (f32); `A = T[i-1] * 512 + shift - cam.x`, `B = T[i] * 512 + shift - cam.y`
///   (unrounded); each snapped to `(ftol(v) & ~0x1FF) + 0x100`, so `X = (A - snapA) + cam.x`, `Y = (B -
///   snapB) + cam.y`, `Z = (high - low) * T[i+1] + low`. Alpha `base`, or `ftol((1 - (Z - cam.z) * 0.004)
///   * base)` when `Z - cam.z > 0`; skipped at <= 0. The horizontal distance `d = max(|dx|, |dy|) /
///   256` above 0.75 scales it by `1 - (d - 0.75) * 4` (skipped at <= 0). Sizes `h1 = T[i+2] * 100 +
///   60`, `h2 = T[i+3] * 100 + 60`; four vertices `(X - h1, Y + h2)`, `(X + h1, Y + h2)`, `(X - h1, Y -
///   h2)`, `(X + h1, Y - h2)` at z = Z, uv (0,0), (0,1), (1,0), (1,1), the first coloured `alpha << 24 |
///   clamped rgb`, the others `alpha << 24 | raw words` (OR-ed and shifted as dwords), indices `n+2,
///   n+1, n, n+2, n+3, n+1`. Drawn once at the end (`RwIm3DTransform(.., NULL, 0x19)`, indexed
///   triangle list), the counts cleared.
/// - **Fluffy sprites**, j = 0..30 (sprite buffer, inited first): `A = windShift * 0.5 + X[j] - cam.x`
///   (f32), `B = .. + Y[j] - cam.y` (unrounded), snapped to `& ~0xFF) + 0x80`; `Z = (high - low) * T[j]
///   + low`; alpha `ftol((1 - |cam.z - Z| * 0.004) * (float)base)`, skipped at <= 0; size `(T[j+1] + 1)
///   * 12`; `d = max(|dx|, |dy|) / 128` (>= 1 or NaN → 1); above 0.75 alpha `ftol((1 - (d - 0.75) * 4)
///   * alpha)`; `d <= 0.05` (or NaN) skips; `d < 0.1` → `ftol((d - 0.05) * alpha * 20)`. On screen: a
///   buffered sprite `(w * size, h * size)` in the raw colour words, intensity 0x100, `rhw = 1 / z`.
///   Flush.
///
/// Finally TEXTURERASTER NULL, VERTEXALPHA 0, ZWRITE 1, ZTEST 1.
#[allow(clippy::cognitive_complexity)]
#[hook(addr = 0x7154B0, conv = cdecl, name = "CClouds::RenderBottomFromHeight", expect = "83 EC 44 0F B7 05 04 C5 B7 00",
       route = "L2", hazards = "heap-writes",
       recs = "grove-walk",
       note = "writes the wind shift 0xC6E954, TempBufferRenderVertices 0xC4D958.. (112 x 0x24), TempBufferIndices 0xC4B958.., the counts 0xC4B950/0xC4B954 and the sprite buffer")]
pub fn render_bottom_from_height() {
    let raw_r = u32::from(g::<u16>(FLUFFY_BOTTOM_RED).read());
    let raw_g = u32::from(g::<u16>(FLUFFY_BOTTOM_GREEN).read());
    let raw_b = u32::from(g::<u16>(FLUFFY_BOTTOM_BLUE).read());
    // 20 (0x858BA4), 255 (0x859AAC), 1.5 (0x858CE8)
    let clamp255 = |v: X87| if jp(x(255.0), v, 5) { v } else { x(255.0) };
    let r2 = xi(raw_r as i32);
    let red = ftol32(clamp255(r2.fadd(r2).fadd_m(20.0))) as u32;
    let green = ftol32(clamp255(xi(raw_g as i32).fmul_m(1.5))) as u32;
    let blue = ftol32(clamp255(xi(raw_b as i32).fmul_m(1.5))) as u32;
    let cam = |off: isize| rd::<f32>(sa_world::world::common::cam_pos().cast(), off);
    // -90 (0x859CE8)
    if !jp(x(cam(8)), x(-90.0), 5) {
        return;
    }
    let mut low: f32 = 160.0;
    let mut high: f32 = 190.0;
    // 190 (0x858A20), 10 (0x85862C), 0.3 (0x858C24)
    let h = x(cam(8)).fsub_m(190.0).fsub_m(10.0).fmul_m(k(0x3E99_999A));
    if !jne(h, x(0.0), 0x41) {
        low = x(160.0).fadd(h).to_f32();
        high = h.fadd_m(190.0).to_f32();
    }
    // 0.25 (0x858C84)
    let ws = x(g::<f32>(TIME_STEP).read()).fmul_m(g::<f32>(WIND).read()).fmul_m(0.25).fadd_m(g::<f32>(WIND_SHIFT).read());
    g::<f32>(WIND_SHIFT).write(ws.to_f32());
    rs(8, 0);
    rs(6, 1);
    rs(12, 1);
    rs(14, 0);
    rs(10, 5);
    rs(11, 6);
    rs(1, raster_of(CLOUD_MASK_TEX));
    let base = ftol32(x(g::<f32>(CLOUD_ALPHA).read()));
    let f = f32::from_bits;
    if base != 0 {
        let dz = x(high).fsub_m(low);
        let (mut ni, mut nv) = (0u32, 0u32);
        g::<u32>(TEMP_INDICES_STORED).write(0);
        g::<u32>(TEMP_VERTICES_STORED).write(0);
        for i in 1..=28i32 {
            // 0.5 (0x858B8C), 512 (0x8594D8)
            let shift = x(height(i + 5)).fmul_m(0.5).fadd_m(1.0).fmul_m(g::<f32>(WIND_SHIFT).read()).to_f32();
            let a = x(height(i - 1)).fmul_m(512.0).fadd_m(shift).fsub_m(cam(0));
            let b = x(height(i)).fmul_m(512.0).fadd_m(shift).fsub_m(cam(4));
            let sa = (ftol32(a) & !0x1FF).wrapping_add(0x100);
            let dxa = a.fsub(xi(sa)).to_f32();
            let sb = (ftol32(b) & !0x1FF).wrapping_add(0x100);
            let dyb = b.fsub(xi(sb));
            let px = x(dxa).fadd_m(cam(0)).to_f32();
            let py = dyb.fadd_m(cam(4)).to_f32();
            let pz = dz.fmul_m(height(i + 1)).fadd_m(low).to_f32();
            let dzc = x(pz).fsub_m(cam(8));
            // 0.004 (0x859CE0)
            let mut alpha = if jne(dzc, x(0.0), 0x41) {
                base
            } else {
                ftol32(dzc.fmul_m(k(0x3B83_126F)).fsubr_m(1.0).fmul(xi(base)))
            };
            if alpha <= 0 {
                continue;
            }
            // 1/256 (0x859AA0), 0.75 (0x858F34), 4 (0x858B90)
            let d = horizontal_distance(px, py).fmul_m(k(0x3B80_0000));
            if !jne(d, x(0.75), 0x41) {
                alpha = ftol32(d.fsub_m(0.75).fmul_m(4.0).fsubr_m(1.0).fmul(xi(alpha)));
            }
            if alpha <= 0 {
                continue;
            }
            // 100 (0x858628), 60 (0x858B34)
            let h1 = x(height(i + 2)).fmul_m(100.0).fadd_m(60.0);
            let h2 = x(height(i + 3)).fmul_m(100.0).fadd_m(60.0);
            let idx = |o: u32, v: u32| g::<u16>((TEMP_BUFFER_INDICES + 2 * (ni + o)) as usize).write(v as u16);
            idx(0, nv + 2);
            idx(1, nv + 1);
            idx(2, nv);
            idx(3, nv + 2);
            idx(4, nv + 3);
            idx(5, nv + 1);
            let a8 = (alpha as u32) << 8;
            let c0 = (((a8 | red) << 8 | green) << 8) | blue;
            let c1 = (((a8 | raw_r) << 8 | raw_g) << 8) | raw_b;
            let v = GPtr::<u8>::from_addr(TEMP_RENDER_VERTICES.wrapping_add(nv * 0x24) as usize);
            let xm = x(px).fsub(h1).to_f32();
            let xp = h1.fadd_m(px).to_f32();
            let yp = h2.fadd(x(py)).to_f32();
            let ym = x(py).fsub(h2).to_f32();
            let corners = [(xm, yp, 0.0f32, 0.0f32, c0), (xp, h2.fadd_m(py).to_f32(), 0.0, 1.0, c1), (xm, ym, 1.0, 0.0, c1), (xp, ym, 1.0, 1.0, c1)];
            for (j, &(cx, cy, u, vv, c)) in corners.iter().enumerate() {
                let o = 0x24 * j as isize;
                wr::<u32>(v, o + 0x18, c);
                wr::<f32>(v, o, cx);
                wr::<f32>(v, o + 4, cy);
                wr::<f32>(v, o + 8, pz);
                wr::<f32>(v, o + 0x1C, u);
                wr::<f32>(v, o + 0x20, vv);
            }
            ni += 6;
            nv += 4;
            g::<u32>(TEMP_INDICES_STORED).write(ni);
            g::<u32>(TEMP_VERTICES_STORED).write(nv);
        }
        if ni != 0 && !rw_im3d_transform(TEMP_RENDER_VERTICES, nv, 0, 0x19).is_null() {
            rw_im3d_render_indexed_primitive(3, TEMP_BUFFER_INDICES, g::<u32>(TEMP_INDICES_STORED).read());
            rw_im3d_end();
        }
        g::<u32>(TEMP_INDICES_STORED).write(0);
        g::<u32>(TEMP_VERTICES_STORED).write(0);
    }
    init_sprite_buffer();
    if base > 0 {
        let dz2 = x(high).fsub_m(low).to_f32();
        let fa = xi(base).to_f32();
        let mut inp: [u32; 3];
        let mut out = [0u32; 3];
        let mut w = 0f32;
        let mut hh = 0f32;
        for j in 0..30i32 {
            let half = x(g::<f32>(WIND_SHIFT).read()).fmul_m(0.5);
            let a = half.fadd_m(g::<f32>(FLUFFY_X + 4 * j as usize).read()).fsub_m(cam(0)).to_f32();
            let b = half.fadd_m(g::<f32>(FLUFFY_Y + 4 * j as usize).read()).fsub_m(cam(4));
            let sa = (ftol32(x(a)) & !0xFF).wrapping_add(0x80);
            let dxa = xi(sa).fsubr_m(a).to_f32();
            let sb = (ftol32(b) & !0xFF).wrapping_add(0x80);
            let dyb = b.fsub(xi(sb));
            let px = x(dxa).fadd_m(cam(0)).to_f32();
            let py = dyb.fadd_m(cam(4)).to_f32();
            let pz = x(dz2).fmul_m(height(j)).fadd_m(low).to_f32();
            inp = [px.to_bits(), py.to_bits(), pz.to_bits()];
            let mut alpha = ftol32(abs_jp(x(cam(8)).fsub_m(pz)).fmul_m(k(0x3B83_126F)).fsubr_m(1.0).fmul_m(fa));
            if alpha <= 0 {
                continue;
            }
            // 12 (0x858CCC)
            let size = x(height(j + 1)).fadd_m(1.0).fmul_m(12.0).to_f32();
            // 1/128 (0x858B88)
            let mut d = horizontal_distance(px, py).fmul_m(k(0x3C00_0000));
            let reduce = if jp(d, x(1.0), 5) {
                d = x(1.0);
                true
            } else {
                !jne(d, x(0.75), 0x41)
            };
            if reduce {
                alpha = ftol32(d.fsub(x(0.75)).fmul_m(4.0).fsubr_m(1.0).fmul(xi(alpha)));
            }
            // 0.05 (0x858C28), 0.1 (0x858B1C), 20 (0x858BA4)
            if jne(d, x(k(0x3D4C_CCCD)), 0x41) {
                continue;
            }
            if !jp(d, x(k(0x3DCC_CCCD)), 5) {
                alpha = ftol32(d.fsub_m(k(0x3D4C_CCCD)).fmul(xi(alpha)).fmul_m(20.0));
            }
            if !calc(&mut inp, &mut out, &mut w, &mut hh) {
                continue;
            }
            let rhw = bits(x(1.0).fdiv_m(f(out[2])));
            let sh = x(hh).fmul_m(size).to_f32();
            let sw = x(w).fmul_m(size).to_f32();
            render_buffered_one_xlu_sprite(f(out[0]), f(out[1]), f(out[2]), sw, sh, raw_r, raw_g, raw_b, 0x100, rhw, alpha as u32);
        }
    }
    flush_sprite_buffer();
    rs(1, 0);
    rs(12, 0);
    rs(8, 1);
    rs(6, 1);
}

orig! {
    /// `Fx_c::GetFxQuality()` (0x49EA50, thiscall on `g_fx`)
    fn get_fx_quality(this: GPtr<u8>) -> u32 = thiscall 0x49EA50 => crate::fx::fx_get_fx_quality;
    /// `CClouds::VolumetricCloudsGetMaxDistance()` (0x713630, ST0)
    fn volumetric_clouds_get_max_distance() -> f64 = cdecl 0x713630 => self::max_distance_port;
    /// `CClouds::VolumetricClouds_Delete(int32)` (0x7135F0)
    fn volumetric_clouds_delete(i: i32) = cdecl 0x7135F0 => crate::clouds::volumetric_clouds_delete;
    /// `CClouds::VolumetricClouds_Create(CVector*)` (0x715F40)
    fn volumetric_clouds_create(pos: GPtr<f32>) = cdecl 0x715F40 => unported;
    /// `FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, remote: u8) -> GPtr<u8> = cdecl 0x56E0D0 => sa_world::world::cworld::find_player_vehicle_port;
    /// `FindPlayerCoors(CVector* out, int32)` (0x56E010)
    fn find_player_coors(out: GPtr<f32>, id: i32) -> GPtr<f32> = cdecl 0x56E010 => sa_world::world::cworld::find_player_coors_port;
    /// `operator!=(const CVector&, const CVector&)` (0x509760, bool in AL)
    fn vector_ne(a: GPtr<f32>, b: GPtr<f32>) -> u8 = cdecl 0x509760 => unported;
    /// `operator*(CVector* out, const CVector&, float)` (0x40FEC0); the float as its raw dword
    fn vector_mul(out: GPtr<f32>, v: GPtr<f32>, s: u32) -> GPtr<f32> = cdecl 0x40FEC0 => unported;
    /// `CVector::operator+=(const CVector&)` (0x411A00)
    fn vector_add_assign(v: GPtr<f32>, o: GPtr<f32>) = thiscall 0x411A00 => unported;
    /// `CGeneral::GetRandomNumberInRange<float>(min, max)` (0x41BD90, ST0)
    fn random_float(min: f32, max: f32) -> f64 = cdecl 0x41BD90 => self::random_float_port;
    /// `RwIm3DRenderPrimitive(primType)` (0x7EF6B0)
    fn rw_im3d_render_primitive_v(prim: u32) -> u32 = cdecl 0x7EF6B0 => unported;
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn max_distance_port() -> f64 {
    crate::clouds::volumetric_clouds_get_max_distance().to_f64()
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn random_float_port(min: f32, max: f32) -> f64 {
    sa_core::general::get_random_number_in_range_f32(min, max).to_f64()
}

/// `CGame::currArea`.
const CURR_AREA_V: usize = 0xB7_2914;
/// `g_fx` (Fx_c).
const G_FX: usize = 0xA9_AE00;
/// The volumetric cloud quality factor (`.data`, 1.0 initially) and the "follow the player" flag byte.
const VC_QUALITY: usize = 0x8D_5388;
const VC_ENABLED: usize = 0x8D_538C;
/// The wind drift factor (`.data`, 0.1).
const VC_DRIFT: usize = 0x8D_5390;
/// `CClouds::m_VolumetricCloudsUsedNum`, `CClouds::m_fVolumetricCloudMaxDistance`.
const VC_USED_NUM: usize = 0xC6_AA5C;
const VC_MAX_DISTANCE: usize = 0xC6_AA58;
/// The function's static guard dword (bits 1, 2) and statics: the old camera position, the old player
/// position, the altitude fade.
const VC_GUARD: usize = 0xC6_E974;
const VC_OLD_CAM: usize = 0xC6_E964;
const VC_OLD_PLAYER: usize = 0xC6_E958;
const VC_FADE: usize = 0xC6_E970;
/// `ms_vc`: slot flags, "just created" flags, positions (CVector), sizes (CVector), alphas (int32).
const VC_SLOTS: usize = 0xC6_AAB0;
const VC_JUST_CREATED: usize = 0xC6_AB64;
const VC_POS: usize = 0xC6_AC18;
const VC_SIZE: usize = 0xC6_B488;
const VC_ALPHA: usize = 0xC6_BCF8;
/// The volumetric cloud texture, the three light directions (CVector), the 18-vertex model tables
/// (x, y, z, u, v; 18 floats each).
const VC_TEX: usize = 0xC6_BFC8;
const VC_LIGHTS: usize = 0xC6_BFCC;
const VC_MODEL: usize = 0xC6_BFF0;
/// `CWeather::WindDir` x, y.
const WIND_DIR_X: usize = 0xC8_13E0;
const WIND_DIR_Y: usize = 0xC8_13E4;
/// The camera front the clouds are spawned along without a vehicle (`TheCamera.m_aCams[0].Front`).
const CAM_FRONT: u32 = 0xB6_F9AC;

/// `CClouds::VolumetricCloudsRender()` (0x716380). Nothing in an interior or with the player's area code
/// (+0x2F) set. The quality factor 0x8D5388 is set to 1.0, or 0.66 / 0.5 for FX quality 1 / 0;
/// `VolumetricCloudsUsedNum = ftol(factor * 180)` (nothing at <= 0, capped at 180), the max distance
/// from `VolumetricCloudsGetMaxDistance`. The camera position is copied; static guard bit 1 is set.
/// The altitude fade 0xC6E970 (with the enable byte 0x8D538C, else 0): camera z below 220 (not
/// unordered) → `fade = timeStep * 4 + fade`; not below 255 (or NaN) → 255 and **return**; else `fade -
/// timeStep * 4`, 0 when below zero.
///
/// Immediate-mode states stored and set, ZTEST 1, the cloud raster, TEXTUREFILTER 2;
/// `FindPlayerVehicle(-1, false)`, `FindPlayerCoors`; guard bit 2. When the camera or the player moved
/// (`operator!=` against the statics, both called), the statics are updated (always) and one cloud is
/// created at `F * (maxDistance + 200)` added to the vehicle position (matrix or placement), or at
/// `cam + F * (..)` along the camera front without a vehicle, offset by `GetRandomNumberInRange` (-200,
/// 200), (-200, 200), (-50, 50).
///
/// Colour `min((sum of the six sky words) / 6 + 64, 255)` (signed division). For every used slot:
/// `d = sqrt((dz² + dy²) + dx²)` to the camera; the position drifts by `0.1 * WindDir`; a slot not
/// just created with `d > maxDistance` is deleted; the just-created flag clears when `d < max` or
/// `max + 200 < d`. Alpha `max(0, slot alpha - ftol(fade))`; beyond `max - 100` it fades out linearly
/// (`ftol((A - B) * alpha / A)`, A = `max - (max - 100)`, B = `d - (max - 100)`, clamped to 0..alpha),
/// skipped at 0 or beyond `max`. Three light intensities `ftol(|L · normalise(p - cam)| * alpha)` (the
/// vector normalised only when a component is non-zero, recopied for each light) give the alphas of
/// three colours `(c, c, c, a_k)`; 18 vertices `size * model + pos` (6 per colour) with the model's uv
/// go to `TempBufferRenderVertices`, flushed (`RwIm3DTransform(.., NULL, 9)`, triangle list) at 2034
/// vertices and after the loop (the slot count re-read every iteration). The states are restored
/// (tail call).
#[allow(clippy::cognitive_complexity)]
#[hook(addr = 0x716380, conv = cdecl, name = "CClouds::VolumetricCloudsRender", expect = "A1 14 29 B7 00 83 EC 7C",
       route = "L2", hazards = "x87-transcendental,heap-writes",
       recs = "grove-walk",
       note = "FSQRT distances; static guard 0xC6E974 bits 1/2 and statics 0xC6E958..0xC6E970; writes 0x8D5388, VolumetricCloudsUsedNum/MaxDistance, ms_vc positions/flags (0xC6AB64.., 0xC6AC18..), TempBufferRenderVertices; rand through GetRandomNumberInRange")]
pub fn volumetric_clouds_render() {
    if g::<u32>(CURR_AREA_V).read() != 0 {
        return;
    }
    if rd::<u8>(sa_world::world::common::find_player_ped(-1), 0x2F) != 0 {
        return;
    }
    let q = get_fx_quality(GPtr::from_addr(G_FX));
    g::<u32>(VC_QUALITY).write(0x3F80_0000);
    if q == 1 {
        g::<u32>(VC_QUALITY).write(0x3F28_F5C3);
    } else if q == 0 {
        g::<u32>(VC_QUALITY).write(0x3F00_0000);
    }
    // 180 (0x85A994)
    let n = ftol32(x(g::<f32>(VC_QUALITY).read()).fmul_m(180.0));
    g::<i32>(VC_USED_NUM).write(n);
    if n <= 0 {
        return;
    }
    if n > 0xB4 {
        g::<i32>(VC_USED_NUM).write(0xB4);
    }
    g::<f32>(VC_MAX_DISTANCE).write(X87::from_f64_exact(volumetric_clouds_get_max_distance()).to_f32());
    let cp = sa_world::world::common::cam_pos().cast::<u8>();
    let mut cam = [rd::<u32>(cp, 0), rd::<u32>(cp, 4), rd::<u32>(cp, 8)];
    let f = f32::from_bits;
    let guard = g::<u32>(VC_GUARD);
    if guard.read() & 1 == 0 {
        guard.write(guard.read() | 1);
    }
    let fade = g::<f32>(VC_FADE);
    if g::<u8>(VC_ENABLED).read() == 0 {
        fade.write(0.0);
    } else {
        let z = x(f(cam[2]));
        // 220 (0x863994), 4 (0x858B90), 255 (0x859AAC)
        let step = x(g::<f32>(TIME_STEP).read()).fmul_m(4.0);
        if !jp(z, x(220.0), 5) {
            let v = step.fadd_m(fade.read());
            fade.write(v.to_f32());
            if !v.fcom(x(255.0)).test_any(1) {
                fade.write(255.0);
                return;
            }
        } else {
            let v = step.fsubr_m(fade.read());
            fade.write(v.to_f32());
            if !jp(v, x(0.0), 5) {
                fade.write(0.0);
            }
        }
    }
    rs_store();
    rs_set_immediate();
    rs(6, 1);
    rs(1, raster_of(VC_TEX));
    rs(9, 2);
    let veh = find_player_vehicle(-1, 0);
    let mut player = [0u32; 3];
    find_player_coors(GPtr::<u32>::from_raw(player.as_mut_ptr()).cast(), -1);
    if guard.read() & 2 == 0 {
        guard.write(guard.read() | 2);
    }
    let pcam = GPtr::<u32>::from_raw(cam.as_mut_ptr()).cast::<f32>();
    let pplayer = GPtr::<u32>::from_raw(player.as_mut_ptr()).cast::<f32>();
    let mut moved = vector_ne(pcam, GPtr::from_addr(VC_OLD_CAM)) != 0;
    if vector_ne(pplayer, GPtr::from_addr(VC_OLD_PLAYER)) != 0 {
        moved = true;
    }
    g::<u32>(VC_OLD_CAM + 8).write(cam[2]);
    g::<u32>(VC_OLD_CAM).write(cam[0]);
    g::<u32>(VC_OLD_CAM + 4).write(cam[1]);
    g::<u32>(VC_OLD_PLAYER).write(player[0]);
    g::<u32>(VC_OLD_PLAYER + 4).write(player[1]);
    g::<u32>(VC_OLD_PLAYER + 8).write(player[2]);
    if moved {
        // 200 (0x858A48)
        let dist = bits(x(g::<f32>(VC_MAX_DISTANCE).read()).fadd_m(200.0));
        let mut v = [0u32; 3];
        let mut tmp = [0u32; 3];
        let ptmp = GPtr::<u32>::from_raw(tmp.as_mut_ptr()).cast::<f32>();
        if veh.is_null() {
            let r = vector_mul(ptmp, GPtr::from_addr(CAM_FRONT as usize), dist);
            let r = [rd::<u32>(r.cast(), 0), rd::<u32>(r.cast(), 4), rd::<u32>(r.cast(), 8)];
            v[0] = bits(x(f(cam[0])).fadd_m(f(r[0])));
            v[1] = bits(x(f(cam[1])).fadd_m(f(r[1])));
            v[2] = bits(x(f(r[2])).fadd_m(f(cam[2])));
        } else {
            let m = rd::<GPtr<u8>>(veh, 0x14);
            let fwd = if m.is_null() { GPtr::<u8>::from_addr(0x10) } else { m.byte_add(0x10) };
            let r = vector_mul(ptmp, fwd.cast(), dist);
            v = [rd::<u32>(r.cast(), 0), rd::<u32>(r.cast(), 4), rd::<u32>(r.cast(), 8)];
            let pos = if m.is_null() { veh.byte_add(4) } else { m.byte_add(0x30) };
            vector_add_assign(GPtr::<u32>::from_raw(v.as_mut_ptr()).cast(), pos.cast());
        }
        v[0] = bits(X87::from_f64_exact(random_float(-200.0, 200.0)).fadd_m(f(v[0])));
        v[1] = bits(X87::from_f64_exact(random_float(-200.0, 200.0)).fadd_m(f(v[1])));
        v[2] = bits(X87::from_f64_exact(random_float(-50.0, 50.0)).fadd_m(f(v[2])));
        volumetric_clouds_create(GPtr::<u32>::from_raw(v.as_mut_ptr()).cast());
    }

    let sum = [SKY_BOTTOM_GREEN, SKY_BOTTOM_BLUE, SKY_BOTTOM_RED, SKY_TOP_BLUE, SKY_TOP_GREEN, SKY_TOP_RED]
        .iter()
        .fold(0i32, |s, &a| s.wrapping_add(i32::from(g::<u16>(a).read())));
    let c = (sum / 6 + 0x40).min(0xFF) as u8;
    let mut nv: u32 = 0;
    let max_d = || g::<f32>(VC_MAX_DISTANCE).read();
    let mut i: usize = 0;
    while (i as i32) < g::<i32>(VC_USED_NUM).read() {
        let slot = i;
        i += 1;
        if g::<u8>(VC_SLOTS + slot).read() == 0 {
            continue;
        }
        let p = VC_POS + 0xC * slot;
        let dx = x(g::<f32>(p).read()).fsub_m(f(cam[0]));
        let dy = x(g::<f32>(p + 4).read()).fsub_m(f(cam[1]));
        let dz = x(g::<f32>(p + 8).read()).fsub_m(f(cam[2]));
        let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt().to_f32();
        let drift = || g::<f32>(VC_DRIFT).read();
        g::<f32>(p).write(x(drift()).fmul_m(g::<f32>(WIND_DIR_X).read()).fadd_m(g::<f32>(p).read()).to_f32());
        g::<f32>(p + 4).write(x(g::<f32>(WIND_DIR_Y).read()).fmul_m(drift()).fadd_m(g::<f32>(p + 4).read()).to_f32());
        let jc = g::<u8>(VC_JUST_CREATED + slot);
        if jc.read() == 0 && !jne(x(d), x(max_d()), 0x41) {
            volumetric_clouds_delete(slot as i32);
            continue;
        }
        if !jp(x(d), x(max_d()), 5) || !jp(x(max_d()).fadd_m(200.0), x(d), 5) {
            jc.write(0);
        }
        let mut alpha = g::<i32>(VC_ALPHA + 4 * slot).read().wrapping_sub(ftol32(x(fade.read())));
        if alpha < 0 {
            alpha = 0;
        }
        // 100 (0x858628)
        let inner = x(max_d()).fsub_m(100.0).to_f32();
        if !jne(x(d), x(inner), 0x41) {
            let top = alpha;
            if top == 0 {
                continue;
            }
            if !jne(x(d), x(max_d()), 0x41) {
                continue;
            }
            let a = x(max_d()).fsub_m(inner);
            let b = x(d).fsub_m(inner);
            let v = ftol32(a.fsub(b).fmul(xi(alpha)).fdiv(a));
            alpha = ftol32(xi(v));
            if alpha < 0 {
                alpha = 0;
            }
            if alpha > top {
                alpha = top;
            }
            if alpha == 0 {
                continue;
            }
        }
        let size = [g::<f32>(VC_SIZE + 0xC * slot).read(), g::<f32>(VC_SIZE + 0xC * slot + 4).read(), g::<f32>(VC_SIZE + 0xC * slot + 8).read()];
        let pos = [g::<f32>(p).read(), g::<f32>(p + 4).read(), g::<f32>(p + 8).read()];
        let rel = [
            x(pos[0]).fsub_m(f(cam[0])).to_f32(),
            x(pos[1]).fsub_m(f(cam[1])).to_f32(),
            x(pos[2]).fsub_m(f(cam[2])).to_f32(),
        ];
        let fa = xi(alpha).to_f32();
        let mut la = [0u8; 3];
        for (l, out) in la.iter_mut().enumerate() {
            let mut v = rel;
            if [v[0], v[1], v[2]].iter().any(|&c| jp(x(c), x(0.0), 0x44)) {
                vector_normalise(GPtr::<f32>::from_raw(v.as_mut_ptr()).cast());
            }
            let lp = VC_LIGHTS + 0xC * l;
            let (lx, ly, lz) = (g::<f32>(lp).read(), g::<f32>(lp + 4).read(), g::<f32>(lp + 8).read());
            let dot = x(lz).fmul_m(v[2]).fadd(x(ly).fmul_m(v[1])).fadd(x(lx).fmul_m(v[0]));
            *out = ftol32(dot.fabs().fmul_m(fa)) as u8;
        }
        for k in 0..18usize {
            let a = la[k / 6];
            let vtx = GPtr::<u8>::from_addr(TEMP_RENDER_VERTICES.wrapping_add(nv * 0x24) as usize);
            let m = |t: usize| g::<f32>(VC_MODEL + 0x48 * t + 4 * k).read();
            wr::<f32>(vtx, 0, x(size[0]).fmul_m(m(0)).fadd_m(pos[0]).to_f32());
            wr::<f32>(vtx, 4, x(size[1]).fmul_m(m(1)).fadd_m(pos[1]).to_f32());
            let cc = u32::from(c);
            wr::<u32>(vtx, 0x18, (((u32::from(a) << 8 | cc) << 8 | cc) << 8) | cc);
            wr::<f32>(vtx, 8, x(size[2]).fmul_m(m(2)).fadd_m(pos[2]).to_f32());
            nv += 1;
            wr::<f32>(vtx, 0x1C, x(m(3)).to_f32());
            wr::<f32>(vtx, 0x20, x(m(4)).to_f32());
            if nv == 0x7F2 {
                if !rw_im3d_transform(TEMP_RENDER_VERTICES, nv, 0, 9).is_null() {
                    rw_im3d_render_primitive_v(3);
                    rw_im3d_end();
                }
                nv = 0;
            }
        }
    }
    if nv as i32 > 0 && !rw_im3d_transform(TEMP_RENDER_VERTICES, nv, 0, 9).is_null() {
        rw_im3d_render_primitive_v(3);
        rw_im3d_end();
    }
    rs_restore();
}
