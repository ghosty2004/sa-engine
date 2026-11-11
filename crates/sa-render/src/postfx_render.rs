//! WRITER-RENDER-BIG-2: `CPostEffects::Render` (0x7046E0), the per-frame post-effect dispatcher. Ported
//! from the 1.0 US bytes; gta-reversed `PostEffects.cpp` as reading aid (several of its branches differ
//! from the bytes, noted below). x87 arithmetic follows the original's instruction order on the
//! `sa_math::x87::X87` model.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, jne, jp, k, rd, x, xi};

orig! {
    /// `RwRenderStateSet(state, value)` (`RwGlobals` +0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xc97b24 0x20 => unported;
    /// `RwCameraEndUpdate(camera)` (0x7ee180)
    fn rw_camera_end_update(cam: GPtr<u8>) -> GPtr<u8> = cdecl 0x7ee180 => unported;
    /// `RwRasterPushContext(raster)` (0x7fb060)
    fn rw_raster_push_context(r: GPtr<u8>) -> GPtr<u8> = cdecl 0x7fb060 => unported;
    /// `RwRasterRenderFast(raster, x, y)` (0x7faf50)
    fn rw_raster_render_fast(r: GPtr<u8>, px: i32, py: i32) -> GPtr<u8> = cdecl 0x7faf50 => unported;
    /// `RwRasterPopContext()` (0x7fb110)
    fn rw_raster_pop_context() -> u32 = cdecl 0x7fb110 => unported;
    /// `RsCameraBeginUpdate(camera)` (0x619450)
    fn rs_camera_begin_update(cam: GPtr<u8>) -> i32 = cdecl 0x619450 => sa_core::platform::rs_camera_begin_update;
    /// `CPostEffects::Fog()` (0x704150)
    fn post_fog() = cdecl 0x704150 => unported;
    /// `FindPlayerPed(int)` (0x56e210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56e210 => sa_world::world::cworld::find_player_ped;
    /// `FindPlayerVehicle(int, bool)` (0x56e0d0)
    fn find_player_vehicle(id: i32, include_remote: u8) -> GPtr<u8> = cdecl 0x56e0d0 => sa_world::world::cworld::find_player_vehicle;
    /// `FindPlayerSpeed(int)` (0x56e090)
    fn find_player_speed(id: i32) -> GPtr<u8> = cdecl 0x56e090 => sa_world::world::cworld::find_player_speed;
    /// `CPhysical::GetLightingFromCol(bool)` (0x5447b0, ST0; the dword the original pushes)
    fn get_lighting_from_col(e: GPtr<u8>, interior: u32) -> f64 = thiscall 0x5447b0 => unported;
    /// `CPostEffects::ColourFilter(RwRGBA pass1, RwRGBA pass2)` (0x703650)
    fn colour_filter(pass1: u32, pass2: u32) = cdecl 0x703650 => crate::mvpd_r3::colour_filter;
    /// `CPostEffects::DarknessFilter(int alpha)` (0x702f00)
    fn darkness_filter(alpha: u32) = cdecl 0x702f00 => unported;
    /// `CPostEffects::Radiosity(int limit, int filterPasses, int renderPasses, int intensity)` (0x702080)
    fn radiosity(limit: u32, filter: u32, render: u32, intensity: u32) = cdecl 0x702080 => unported;
    /// `CPostEffects::SetSpeedFXManualSpeedCurrentFrame(float)` (0x700be0; the float as a raw dword)
    fn set_speed_fx_manual(v: u32) = cdecl 0x700be0 => unported;
    /// `CPostEffects::SpeedFX(float)` (0x7030a0; the float as a raw dword)
    fn speed_fx(v: u32) = cdecl 0x7030a0 => unported;
    /// `DotProduct(const CVector&, const CVector&)` (0x40fdb0, ST0)
    fn dot_product(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40fdb0 => unported;
    /// `CVector::Magnitude()` (0x4082c0, ST0)
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082c0 => unported;
    /// `CPostEffects::NightVision(RwRGBA)` (0x7011c0)
    fn night_vision(col: u32) = cdecl 0x7011c0 => unported;
    /// `CPostEffects::Grain(int strength, bool)` (0x7037c0)
    fn grain(strength: u32, update: u32) = cdecl 0x7037c0 => unported;
    /// `CPostEffects::InfraredVision(RwRGBA, RwRGBA)` (0x703f80)
    fn infrared_vision(c1: u32, c2: u32) = cdecl 0x703f80 => unported;
    /// `CCullZones::CamNoRain()` (0x72ddb0)
    fn cam_no_rain() -> u8 = cdecl 0x72ddb0 => crate::cullzones::cam_no_rain;
    /// `CCullZones::PlayerNoRain()` (0x72ddc0)
    fn player_no_rain() -> u8 = cdecl 0x72ddc0 => crate::cullzones::player_no_rain;
    /// `CPostEffects::HeatHazeFX(float intensity, bool alwaysOn)` (0x701780; the float as a raw dword)
    fn heat_haze_fx(intensity: u32, always: u32) = cdecl 0x701780 => unported;
    /// `CPostEffects::UnderWaterRipple(RwRGBA, float xo, float yo, int strength, float speed, float freq)`
    /// (0x7039c0; floats as raw dwords)
    fn under_water_ripple(col: u32, xo: u32, yo: u32, strength: u32, speed: u32, freq: u32) = cdecl 0x7039c0 => unported;
    /// `CPostEffects::CCTV()` (0x702f40)
    fn cctv() = cdecl 0x702f40 => unported;
    /// `CTimer::Suspend()` (0x5619d0)
    fn timer_suspend() = cdecl 0x5619d0 => sa_core::timer::suspend;
    /// `CTimer::Resume()` (0x561a00)
    fn timer_resume() = cdecl 0x561a00 => sa_core::timer::resume;
    /// `CVisibilityPlugins::RenderWeaponPedsForPC()` (0x732f30)
    fn render_weapon_peds_for_pc() = cdecl 0x732f30 => unported;
    /// `CLinkList<CPed*>::Clear()` (0x53e8d0; `ResetWeaponPedsForPC` inlined)
    fn link_list_clear(this: GPtr<u8>) = thiscall 0x53e8d0 => sa_core::mvpb_app::link_list_clear;
    /// `CFileMgr::SetDirMyDocuments()` (0x538860)
    fn set_dir_my_documents() -> i32 = cdecl 0x538860 => sa_core::filemgr::set_dir_my_documents;
    /// `CFileMgr::SetDir(const char*)` (0x5387d0)
    fn set_dir(path: GPtr<u8>) -> i32 = cdecl 0x5387d0 => sa_core::filemgr::set_dir;
    /// CRT `sprintf(buf, fmt, int)` (0x821bb5)
    fn sprintf1(buf: GPtr<u8>, fmt: GPtr<u8>, v: u32) -> i32 = cdecl 0x821bb5 => unported;
    /// CRT `fopen` (0x8232d8)
    fn fopen(name: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x8232d8 => unported;
    /// CRT `fclose` (0x82318b)
    fn fclose(f: GPtr<u8>) -> i32 = cdecl 0x82318b => unported;
    /// `JPegCompressScreenToFile(RwCamera*, const char*)` (0x5d0820)
    fn jpeg_compress_screen_to_file(cam: GPtr<u8>, name: GPtr<u8>) = cdecl 0x5d0820 => unported;
}

/// `CPostEffects` flags (bytes).
const DISABLE_ALL: usize = 0xc4_02cf;
const FOG: usize = 0xc4_02c6;
const NIGHT_VISION: usize = 0xc4_02b8;
const INFRARED_VISION: usize = 0xc4_02b9;
const DARKNESS_FILTER: usize = 0xc4_02c4;
const SPEED_FX_TEST_MODE: usize = 0xc4_02c7;
const IN_CUTSCENE: usize = 0xc4_02b7;
const RADIOSITY: usize = 0xc4_02cc;
const RADIOSITY_BYPASS: usize = 0xc4_02ce;
const RAIN_ENABLE: usize = 0xc4_02d1;
const GRAIN_ENABLE: usize = 0xc4_02b4;
const HEAT_HAZE_FX: usize = 0xc4_02ba;
const WATER_ENABLE: usize = 0xc4_02d3;
const CCTV: usize = 0xc4_02c5;
const SAVE_PHOTO_FROM_SCRIPT: usize = 0xc4_02d0;
/// `.data` settings: `m_bColorEnable`, `m_bSpeedFX`, `m_bSpeedFXUserFlag`, `m_bSpeedFXUserFlagCurrentFrame`,
/// `m_bWaterDepthDarkness`.
const COLOR_ENABLE: usize = 0x8d_518c;
const SPEED_FX: usize = 0x8d_5100;
const SPEED_FX_USER: usize = 0x8d_5108;
const SPEED_FX_USER_FRAME: usize = 0x8d_5109;
const WATER_DEPTH_DARKNESS: usize = 0x8d_5144;
/// `m_fSpeedFXManualSpeedCurrentFrame`.
const SPEED_FX_MANUAL: usize = 0xc4_02c8;
/// `s_ExtraMult` and its guard; `s_CurrentStrength`, `s_WaterGreen`, `s_WaitForOneFrame`, `s_SavePhotoToGallery`.
const EXTRA_MULT: usize = 0xc4_032c;
const EXTRA_MULT_GUARD: usize = 0xc4_0330;
const CURRENT_STRENGTH: usize = 0xc4_0328;
const WATER_GREEN: usize = 0xc4_0324;
const WAIT_FOR_ONE_FRAME: usize = 0xc4_0321;
const SAVE_TO_GALLERY: usize = 0xc4_0320;
/// `.data` floats: `SCREEN_EXTRA_MULT_BASE_CAP`, change rate, base mult, the two colour multipliers,
/// `gfLaRiotsLightMult`.
const EXTRA_CAP: usize = 0x8d_516c;
const EXTRA_RATE: usize = 0x8d_5168;
const EXTRA_BASE_MULT: usize = 0x8d_5170;
const COLOR1_MULT: usize = 0x8d_5160;
const COLOR2_MULT: usize = 0x8d_5164;
const LA_RIOTS_LIGHT_MULT: usize = 0x8c_d060;
/// `CTimeCycle::m_fPostFx1Red` .. `m_fPostFx2Alpha` (8 floats).
const POSTFX_COLOURS: usize = 0xb7_c518;
/// `CTimer::ms_fTimeStep`.
const TIME_STEP: usize = 0xb7_cb5c;
/// `CWeather::Rain`, `UnderWaterness`, `WaterDepth`, `HeatHaze`, `HeatHazeFXControl`.
const WEATHER_RAIN: usize = 0xc8_1324;
const WEATHER_UNDER_WATERNESS: usize = 0xc8_132c;
const WEATHER_WATER_DEPTH: usize = 0xc8_1330;
const WEATHER_HEAT_HAZE: usize = 0xc8_12dc;
const WEATHER_HEAT_HAZE_CONTROL: usize = 0xc8_12d8;
/// `m_fWaterFXStartUnderWaterness`, `m_fWaterFullDarknessDepth`.
const WATER_FX_START: usize = 0x8d_514c;
const WATER_FULL_DARKNESS: usize = 0x8d_5148;
/// `g_fxMan.m_bHeatHazeEnabled` (byte).
const FX_HEAT_HAZE_ENABLED: usize = 0xa9_af38;
/// `CCutsceneMgr::ms_running`, `ms_cutsceneProcessing`.
const CUTSCENE_RUNNING: usize = 0xb5_f851;
const CUTSCENE_PROCESSING: usize = 0xb5_f852;
/// `CGame::currArea`.
const CURR_AREA: usize = 0xb7_2914;
/// `TheCamera.m_matrix` (pointer) and `TheCamera.m_placement` position.
const CAMERA_MATRIX: usize = 0xb6_f03c;
const CAMERA_PLACEMENT_POS: usize = 0xb6_f02c;
/// `RsGlobal.maximumWidth`, `maximumHeight`.
const RS_WIDTH: usize = 0xc1_7044;
const RS_HEIGHT: usize = 0xc1_7048;
/// `Scene.m_pRwCamera`, `CPostEffects::pRasterFrontBuffer`, `TheCamera.m_pRwCamera`.
const SCENE_CAMERA: usize = 0xc1_703c;
const RASTER_FRONT_BUFFER: usize = 0xc4_02d8;
const THE_CAMERA_RW: usize = 0xb6_f97c;
/// `CWeapon::ms_bTakePhoto`, `FrontEndMenuManager.m_bIsSaveDone`, `.m_bSavePhotos`,
/// `.m_bActivateMenuNextFrame`; `CSpecialFX::bSnapShotActive`, `SnapShotFrames`.
const TAKE_PHOTO: usize = 0xc8_a7c1;
const MENU_SAVE_DONE: usize = 0xba_67a7;
const MENU_SAVE_PHOTOS: usize = 0xba_6830;
const MENU_ACTIVATE_NEXT: usize = 0xba_677b;
const SNAPSHOT_ACTIVE: usize = 0xc7_c714;
const SNAPSHOT_FRAMES: usize = 0xc7_c710;
/// `gString`, `"Gallery\\gallery%d.jpg"`, `"rb"`, `""`, `CVisibilityPlugins::ms_weaponPedsForPC`.
const G_STRING: usize = 0xb7_1670;
const GALLERY_FMT: usize = 0x87_2644;
const MODE_RB: usize = 0x85_a53c;
const EMPTY_STR: usize = 0x85_8b54;
const WEAPON_PEDS_FOR_PC: usize = 0xc8_8224;

fn gf(a: usize) -> f32 {
    g::<f32>(a).read()
}

fn gb(a: usize) -> u8 {
    g::<u8>(a).read()
}

fn gd(a: usize) -> u32 {
    g::<u32>(a).read()
}

/// The inlined byte clamp: 255 when `v >= 255` or unordered, 0 when `v <= 0`, otherwise `v`.
fn clamp255(v: X87) -> X87 {
    let c255 = x(k(0x437f_0000));
    if !jp(v, c255, 0x5) {
        if jne(v, x(0.0), 0x41) {
            return x(0.0);
        }
        return v;
    }
    c255
}

/// `_ftol` of a clamped product, low byte.
fn ftol8(v: X87) -> u8 {
    ftol32(v) as u8
}

/// `CPostEffects::Render()` (0x7046E0). Nothing when `m_bDisableAllPostEffect`. TEXTUREADDRESS clamp,
/// `RwCameraEndUpdate(Scene.camera)`, the camera raster copied to `pRasterFrontBuffer`
/// (push context, `RenderFast(.., 0, 0)`, pop), `RsCameraBeginUpdate`. `s_ExtraMult` is initialised
/// once from the cap. `Fog()` when `m_bFog`.
///
/// Colours: the 8 timecycle post-fx floats through `_ftol` (low bytes); night / infrared vision set the
/// six RGB bytes to 64 (alphas kept). Pass 1 RGB times `gfLaRiotsLightMult` (`_ftol`, no clamp). With
/// a player ped, `s_ExtraMult` moves towards `GetLightingFromCol(false)` by at most `timestep * rate`
/// (`GetLightingFromCol` is called twice and `FindPlayerPed` three times, as the original), capped
/// at the cap; the factor is `(1 - extra / cap) * baseMult + 1` (1.0 without a player). Pass 1 RGB times
/// `mult1 * factor`, pass 2 RGB times `mult2 * factor`, each clamped to 0..255 (unordered: 255) and
/// `_ftol`ed; the original then clamps the six bytes once more (a no-op on bytes, not repeated).
///
/// Then: `ColourFilter(pass1, pass2)` (`m_bColorEnable`); darkness filter + `Radiosity(limit, 0, 2,
/// 255)` without night / infrared vision; speed FX (below); night vision (outside cutscenes) or the
/// switch-on count reset; infrared vision; radiosity (bypass limit or the timecycle's); rain grain;
/// grain; heat haze; underwater ripple; CCTV; the photo.
///
/// Bytes over gta-reversed: the speed-FX nitro value `2 * dot * (gas + 1)` is capped at 1.0; with
/// `m_bHeatHazeFX` set, or with heat haze / FX heat haze and underwaterness >= start,
/// `HeatHazeFX(1.0, false)` is called; the water depth darkness is 1.0 (not 0.0) when
/// `m_bWaterDepthDarkness` is off, and the ripple colour's alpha is 0; the photo path clears
/// `s_WaitForOneFrame` (gr sets it).
#[allow(clippy::cognitive_complexity)]
#[hook(addr = 0x7046e0, conv = cdecl, name = "CPostEffects::Render", expect = "A0 CF 02 C4 00 83 EC 10",
       route = "split", hazards = "heap-writes",
       recs = "grove-walk,driving",
       note = "camera end/begin update and a raster copy (RW heap, D3D); the photo path writes a JPEG file (fopen probe loop) and suspends the timer")]
pub fn post_effects_render() {
    if gb(DISABLE_ALL) != 0 {
        return;
    }
    rw_render_state_set(2, 3);
    let cam = g::<GPtr<u8>>(SCENE_CAMERA).read();
    rw_camera_end_update(cam);
    rw_raster_push_context(g::<GPtr<u8>>(RASTER_FRONT_BUFFER).read());
    let cam = g::<GPtr<u8>>(SCENE_CAMERA).read();
    rw_raster_render_fast(rd::<GPtr<u8>>(cam, 0x60), 0, 0);
    rw_raster_pop_context();
    rs_camera_begin_update(g::<GPtr<u8>>(SCENE_CAMERA).read());
    if gb(EXTRA_MULT_GUARD) & 1 == 0 {
        g::<u32>(EXTRA_MULT_GUARD).write(gd(EXTRA_MULT_GUARD) | 1);
        g::<u32>(EXTRA_MULT).write(gd(EXTRA_CAP));
    }
    if gb(FOG) != 0 {
        post_fog();
    }

    // pass 1 = c[0..4], pass 2 = c[4..8]
    let mut c = [0u8; 8];
    for (i, b) in c.iter_mut().enumerate() {
        *b = ftol32(x(gf(POSTFX_COLOURS + 4 * i))) as u8;
    }
    for flag in [NIGHT_VISION, INFRARED_VISION] {
        if gb(flag) != 0 {
            for i in [0, 1, 2, 4, 5, 6] {
                c[i] = 0x40;
            }
        }
    }
    for b in &mut c[0..3] {
        *b = ftol32(xi(i32::from(*b)).fmul_m(gf(LA_RIOTS_LIGHT_MULT))) as u8;
    }

    let factor = if !find_player_ped(-1).is_null() {
        let step = x(gf(TIME_STEP)).fmul_m(gf(EXTRA_RATE)).to_f32();
        let l = X87::from_f64_exact(get_lighting_from_col(find_player_ped(-1), 0));
        let near = !jp(l.fsub_m(gf(EXTRA_MULT)).fabs(), x(step), 0x5);
        let l2 = X87::from_f64_exact(get_lighting_from_col(find_player_ped(-1), 0));
        let v = if near {
            l2
        } else {
            let e = x(gf(EXTRA_MULT));
            if jne(l2, e, 0x41) { e.fsub_m(step) } else { e.fadd_m(step) }
        };
        g::<f32>(EXTRA_MULT).write(v.to_f32());
        if !jne(x(gf(EXTRA_MULT)), x(gf(EXTRA_CAP)), 0x41) {
            g::<u32>(EXTRA_MULT).write(gd(EXTRA_CAP));
        }
        x(gf(EXTRA_MULT)).fdiv_m(gf(EXTRA_CAP)).fsubr_m(1.0).fmul_m(gf(EXTRA_BASE_MULT)).fadd_m(1.0)
    } else {
        x(1.0)
    };
    let m1 = x(gf(COLOR1_MULT)).fmul(factor);
    for b in &mut c[0..3] {
        *b = ftol8(clamp255(xi(i32::from(*b)).fmul(m1)));
    }
    let m2 = factor.fmul_m(gf(COLOR2_MULT));
    for b in &mut c[4..7] {
        *b = ftol8(clamp255(xi(i32::from(*b)).fmul(m2)));
    }

    if gb(COLOR_ENABLE) != 0 {
        colour_filter(u32::from_le_bytes([c[0], c[1], c[2], c[3]]), u32::from_le_bytes([c[4], c[5], c[6], c[7]]));
    }
    if gb(DARKNESS_FILTER) != 0 && gb(NIGHT_VISION) == 0 && gb(INFRARED_VISION) == 0 {
        darkness_filter(gd(0x8d_5204));
        radiosity(gd(0x8d_50f8), 0, 2, 0xff);
    }
    if gb(SPEED_FX_TEST_MODE) != 0 {
        set_speed_fx_manual(0x3f80_0000);
    }
    if gb(SPEED_FX) != 0 && gb(SPEED_FX_USER) != 0 && gb(SPEED_FX_USER_FRAME) != 0 {
        speed_fx_block();
    }

    g::<u32>(SPEED_FX_MANUAL).write(0);
    g::<u8>(SPEED_FX_USER_FRAME).write(1);
    let in_cut = u8::from(gb(CUTSCENE_RUNNING) != 0 || gb(CUTSCENE_PROCESSING) != 0);
    g::<u8>(IN_CUTSCENE).write(in_cut);
    let mut in_cut = in_cut;
    if gb(NIGHT_VISION) != 0 {
        if in_cut == 0 {
            night_vision(gd(0x8d_50ac));
            grain(gd(0x8d_50a8), 1);
            in_cut = gb(IN_CUTSCENE);
        }
    } else {
        // m_fNightVisionSwitchOnFXCount = m_fNightVisionSwitchOnFXTime
        g::<u32>(0xc4_0300).write(gd(0x8d_50b0));
    }
    if gb(INFRARED_VISION) != 0 && in_cut == 0 {
        infrared_vision(gd(0x8d_50cc), gd(0x8d_50d0));
        grain(gd(0x8d_50b4), 1);
    }
    if gb(RADIOSITY) != 0 && gb(DARKNESS_FILTER) == 0 {
        // CTimeCycle::m_CurrentColours.m_nHighLightMinIntensity (0xB7C53C) or m_RadiosityIntensityLimit
        let limit = if gb(RADIOSITY_BYPASS) == 0 { gd(0xb7_c53c) } else { gd(0x8d_5114) };
        radiosity(limit, gd(0x8d_510c), gd(0x8d_5110), gd(0x8d_5118));
    }

    rain_grain();
    if gb(GRAIN_ENABLE) != 0 {
        grain(gd(0x8d_5094), 1);
    }
    heat_haze();
    under_water();
    if gb(CCTV) != 0 {
        cctv();
    }
    photo();
}

/// The speed-FX part (0x704DA2..0x704E92).
fn speed_fx_block() {
    let veh = find_player_vehicle(-1, 0);
    if jp(x(gf(SPEED_FX_MANUAL)), x(0.0), 0x44) {
        speed_fx(gd(SPEED_FX_MANUAL));
        return;
    }
    let mut any = true;
    if veh.is_null() {
        any = false;
    } else {
        let ty = rd::<u32>(veh, 0x594);
        if (3..=6).contains(&ty) {
            any = false;
        } else if ty == 0 && rd::<u32>(veh, 0x38c) & 0x8_0000 != 0 && !jp(x(rd::<f32>(veh, 0x8a4)), x(0.0), 0x5) {
            let fwd = rd::<GPtr<u8>>(veh, 0x14).byte_add(0x10);
            let dot = X87::from_f64_exact(dot_product(veh.byte_add(0x44), fwd));
            // 0.2 (0x858CC4)
            if !jne(dot, x(k(0x3e4c_cccd)), 0x41) {
                let v = dot.fmul(x(rd::<f32>(veh, 0x49c)).fadd_m(1.0));
                let v = v.fadd(v);
                let arg = if jne(v, x(1.0), 0x41) { v.to_f32().to_bits() } else { 0x3f80_0000 };
                set_speed_fx_manual(arg);
                speed_fx(gd(SPEED_FX_MANUAL));
                return;
            }
        }
    }
    if gb(CUTSCENE_RUNNING) == 0 && any {
        let m = X87::from_f64_exact(vector_magnitude(find_player_speed(-1)));
        speed_fx(m.to_f32().to_bits());
    }
}

/// Rain grain (0x704FBB..0x705080): `s_CurrentStrength` steps towards `128 * Rain` (incremented when
/// below, then decremented when the new value is above; unordered changes nothing), floored at 0;
/// `Grain(strength / 4, true)` outside no-rain zones, with underwaterness <= 0, in area 0 and the
/// camera at z <= 900.
fn rain_grain() {
    let mut cur = g::<i32>(CURRENT_STRENGTH).read();
    if gb(RAIN_ENABLE) == 0 && cur == 0 {
        return;
    }
    // 128 (0x858BF4)
    let r = x(gf(WEATHER_RAIN)).fmul_m(k(0x4300_0000));
    if !jp(xi(g::<i32>(CURRENT_STRENGTH).read()), r, 0x5) {
        cur = cur.wrapping_add(1);
        g::<i32>(CURRENT_STRENGTH).write(cur);
    }
    if !jne(xi(g::<i32>(CURRENT_STRENGTH).read()), r, 0x41) {
        cur = cur.wrapping_sub(1);
    }
    let cur = if cur < 0 { 0 } else { cur };
    g::<i32>(CURRENT_STRENGTH).write(cur);
    if cam_no_rain() != 0 || player_no_rain() != 0 {
        return;
    }
    if jp(x(gf(WEATHER_UNDER_WATERNESS)), x(0.0), 0x41) || g::<u32>(CURR_AREA).read() != 0 {
        return;
    }
    let m = g::<GPtr<u8>>(CAMERA_MATRIX).read();
    let pos = if m.is_null() { GPtr::<u8>::from_addr(CAMERA_PLACEMENT_POS) } else { m.byte_add(0x30) };
    // 900 (0x858978)
    if jp(x(rd::<f32>(pos, 8)), x(k(0x4461_0000)), 0x41) {
        return;
    }
    grain((g::<i32>(CURRENT_STRENGTH).read() / 4) as u32, 1);
}

/// Heat haze (0x705099..0x70511E).
fn heat_haze() {
    let start = || x(gf(WATER_FX_START));
    let uw = || x(gf(WEATHER_UNDER_WATERNESS));
    if gb(HEAT_HAZE_FX) != 0 {
        heat_haze_fx(0x3f80_0000, 0);
        return;
    }
    let enabled = gb(FX_HEAT_HAZE_ENABLED);
    let heat_pos = !jne(x(gf(WEATHER_HEAT_HAZE)), x(0.0), 0x41);
    if heat_pos || enabled != 0 {
        if !jne(uw(), start(), 0x1) {
            heat_haze_fx(0x3f80_0000, 0);
        } else if !jne(x(gf(WEATHER_HEAT_HAZE)), x(0.0), 0x41) {
            heat_haze_fx(gd(WEATHER_HEAT_HAZE_CONTROL), 0);
        } else if enabled != 0 {
            heat_haze_fx(0x3f80_0000, 1);
        }
    } else if !jne(uw(), start(), 0x1) {
        heat_haze_fx(0x3f80_0000, 0);
    }
}

/// Underwater ripple (0x70511E..0x7052A4). Without `m_waterEnable` and with underwaterness below the
/// start (or unordered) `s_WaterGreen = 0`. Otherwise the colour is `min(r + 184, 255)`, `min(g +
/// ftol(green + 184), 255)` (signed), `min(b + 184, 255)` times the depth darkness `1 - min(depth,
/// full) / full` (1.0 when the flag is off), `_ftol`ed, alpha 0 (the high byte of the clamped red
/// int). `s_WaterGreen = min(green + timestep, 24)`; `UnderWaterRipple(colour, (width / 640) *
/// (green / 24) * xoffset, (height / 448) * yoffset, strength, speed, freq)`.
fn under_water() {
    if gb(WATER_ENABLE) == 0 && jne(x(gf(WEATHER_UNDER_WATERNESS)), x(gf(WATER_FX_START)), 0x1) {
        g::<u32>(WATER_GREEN).write(0);
        return;
    }
    let mut d = x(1.0);
    if gb(WATER_DEPTH_DARKNESS) != 0 {
        let wd = x(gf(WEATHER_WATER_DEPTH));
        let full = gf(WATER_FULL_DARKNESS);
        let v = if jne(wd, x(full), 0x41) { wd } else { x(gf(WATER_FULL_DARKNESS)) };
        d = v.fdiv_m(gf(WATER_FULL_DARKNESS)).fsubr_m(1.0);
    }
    let col = gd(0x8d_5140);
    let b = i32::from(gb(0x8d_5142));
    // 184 (0x872660)
    let e = ftol32(x(gf(WATER_GREEN)).fadd_m(k(0x4338_0000)));
    let cl = |v: i32| if v > 0xff { 0xff } else { v };
    let ri = cl((col & 0xff) as i32 + 0xb8);
    let gi = cl((((col >> 8) & 0xff) as i32).wrapping_add(e));
    let bi = cl(b + 0xb8);
    let r8 = ftol8(xi(ri).fmul(d));
    let g8 = ftol8(xi(gi).fmul(d));
    let b8 = ftol8(xi(bi).fmul(d));
    let colour = u32::from_le_bytes([r8, g8, b8, 0]);
    let v = x(gf(TIME_STEP)).fadd_m(gf(WATER_GREEN));
    g::<f32>(WATER_GREEN).write(v.to_f32());
    // 24 (0x85F114)
    if !jne(v, x(k(0x41c0_0000)), 0x41) {
        g::<u32>(WATER_GREEN).write(0x41c0_0000);
    }
    // 1/448 (0x859524), 1/640 (0x859520), 1/24 (0x87265C)
    let yo = xi(g::<i32>(RS_HEIGHT).read()).fmul_m(k(0x3b12_4925)).fmul_m(gf(0x8d_5134)).to_f32();
    let w = xi(g::<i32>(RS_WIDTH).read()).fmul_m(k(0x3acc_cccd));
    let xo = w.fmul(x(gf(WATER_GREEN)).fmul_m(k(0x3d2a_aaab))).fmul_m(gf(0x8d_5130)).to_f32();
    under_water_ripple(colour, xo.to_bits(), yo.to_bits(), gd(0x8d_512c), gd(0x8d_5138), gd(0x8d_513c));
}

/// The photo (0x7052B2..0x7053E3). A taken photo with the menu's save done clears `ms_bTakePhoto` and
/// `m_bSavePhotoFromScript`. Without `s_WaitForOneFrame`: a photo arms it (and `s_SavePhotoToGallery`
/// when the menu saves photos). With it and a photo: it is cleared, the timer suspended; to the
/// gallery: weapon peds rendered and their list cleared, `SetDirMyDocuments`, the first
/// `Gallery\gallery%d.jpg` (from 1) that `fopen(.., "rb")` cannot open, `JPegCompressScreenToFile`,
/// `SetDir("")`; the timer resumed; the snapshot started unless the menu opens next frame; the three
/// photo flags cleared.
fn photo() {
    let mut take = gb(TAKE_PHOTO);
    if take != 0 && gb(MENU_SAVE_DONE) != 0 {
        take = 0;
        g::<u8>(TAKE_PHOTO).write(0);
        g::<u8>(SAVE_PHOTO_FROM_SCRIPT).write(0);
    }
    if gb(WAIT_FOR_ONE_FRAME) == 0 {
        if take == 0 {
            return;
        }
        if gb(MENU_SAVE_PHOTOS) != 0 {
            g::<u8>(SAVE_TO_GALLERY).write(1);
        }
        g::<u8>(WAIT_FOR_ONE_FRAME).write(1);
        return;
    }
    if take == 0 {
        return;
    }
    g::<u8>(WAIT_FOR_ONE_FRAME).write(0);
    timer_suspend();
    if gb(SAVE_TO_GALLERY) != 0 {
        render_weapon_peds_for_pc();
        link_list_clear(GPtr::from_addr(WEAPON_PEDS_FOR_PC));
        set_dir_my_documents();
        let buf = GPtr::<u8>::from_addr(G_STRING);
        let mut n: u32 = 1;
        sprintf1(buf, GPtr::from_addr(GALLERY_FMT), n);
        let mut f = fopen(buf, GPtr::from_addr(MODE_RB));
        while !f.is_null() {
            n = n.wrapping_add(1);
            sprintf1(buf, GPtr::from_addr(GALLERY_FMT), n);
            fclose(f);
            f = fopen(buf, GPtr::from_addr(MODE_RB));
        }
        jpeg_compress_screen_to_file(g::<GPtr<u8>>(THE_CAMERA_RW).read(), buf);
        set_dir(GPtr::from_addr(EMPTY_STR));
    }
    timer_resume();
    if gb(MENU_ACTIVATE_NEXT) == 0 {
        g::<u8>(SNAPSHOT_ACTIVE).write(1);
        g::<u32>(SNAPSHOT_FRAMES).write(0);
    }
    g::<u8>(SAVE_TO_GALLERY).write(0);
    g::<u8>(TAKE_PHOTO).write(0);
    g::<u8>(SAVE_PHOTO_FROM_SCRIPT).write(0);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_POST_EFFECTS_RENDER];
