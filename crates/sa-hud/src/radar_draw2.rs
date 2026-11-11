//! `CRadar` drawing, third pass (PORT-HUD-2): blips, sprites and the radar orchestration. Ported from the
//! 1.0 US bytes; gta-reversed `Radar.cpp` as reading aid (the bytes win). Every RenderWare / `CSprite2d` /
//! colour / transform call stays an `orig!` call (`TransformRadarPointToScreenSpace` preserves EDX for its
//! original callers, so it is always called through `orig!`).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, ftol32, g, rd, wr, x, xi};

/// This module's globals.
pub const GLOBALS: &[GlobalDesc] = &[];

// Foreign globals, addressed directly.
/// `FrontEndMenuManager.m_bDrawingMap` (+0x59)
const MENU_DRAWING_MAP: usize = 0xBA67A1;
/// `FrontEndMenuManager` +0x47 (byte `DrawCoordBlip` tests on the map before drawing a trace)
const MENU_SHOW_TRACES: usize = 0xBA678F;
/// `RsGlobal.maximumWidth` / `maximumHeight`
const RS_MAX_WIDTH: usize = 0xC17044;
const RS_MAX_HEIGHT: usize = 0xC17048;
/// `CRadar::ms_RadarTrace[175]` (0x28 each)
const RADAR_TRACE: usize = 0xBA86F0;
/// `CRadar::RadarBlipSprites[64]` (`CSprite2d`, 4 bytes each)
const RADAR_BLIP_SPRITES: usize = 0xBAA250;
/// `HudColour`
const HUD_COLOUR: usize = 0xBAB22C;
/// `CRadar::m_radarRange` and `CRadar::vec2DRadarOrigin`
const RADAR_RANGE: usize = 0xBA8314;
const RADAR_ORIGIN_X: usize = 0xBAA248;
const RADAR_ORIGIN_Y: usize = 0xBAA24C;
/// `CWorld::PlayerInFocus` (byte) and `CWorld::Players[]` (stride 0x190)
const PLAYER_IN_FOCUS: usize = 0xB7CD74;
const PLAYERS: usize = 0xB7CD98;
/// `CGame::currArea`
const CURR_AREA: usize = 0xB72914;
/// `FrontEndMenuManager.m_nRadarMode` (1 = blips only)
const MENU_RADAR_MODE: usize = 0xBA676C;
/// Co-op radar centre mode (.data int, 2 in the image): 0 player 0, 1 player 1, else the midpoint
const COOP_RADAR_CENTRE: usize = 0x8A5E50;
/// `CRadar::cachedCos` / `cachedSin`
const CACHED_COS: usize = 0xBA8308;
const CACHED_SIN: usize = 0xBA830C;
/// `gRadarTextures[12][12]` (TXD slots, -1 = none)
const RADAR_TEXTURES: usize = 0xBA8478;
/// `CTxdStore::ms_pTxdPool`
const TXD_POOL: usize = 0xC8800C;
/// `CSprite2d::maVertices` and the float `SetMaskVertices` takes for the corner arcs (0xC80460)
const SPRITE2D_VERTICES: usize = 0xC80468;
const SPRITE2D_NEAR_SCREEN_Z: usize = 0xC80460;
/// `DrawRadarGangOverlay`'s function-local static `CRect` (left, bottom, right, top) and its guard bit
/// (bit 0 of 0xBAA36C)
const GANG_RECT: usize = 0xBAA35C;
const GANG_RECT_GUARD: usize = 0xBAA36C;
/// `CGangWars::bGangWarsActive`
const GANG_WARS_ACTIVE: usize = 0x96AB90;
/// `FrontEndMenuManager` +0x49 (byte: show gang areas)
const MENU_SHOW_GANG_AREAS: usize = 0xBA6791;
/// `CTheZones::TotalNumberOfNavigationZones` (int16), `NavigationZoneArray` (0x20 each),
/// `ZoneInfoArray` (0x11 each)
const NAV_ZONE_COUNT: usize = 0xBA3794;
const NAV_ZONES: usize = 0xBA37A8;
const ZONE_INFO: usize = 0xBA1DF0;
/// `CTimer::m_snTimeInMilliseconds` / `m_snTimeInMillisecondsPauseMode`
const TIME_MS: usize = 0xB7CB84;
const TIME_MS_PAUSE: usize = 0xB7CB7C;
/// `CRadar::m_radarRect` (left, bottom, right, top)
const RADAR_RECT: usize = 0x8D0920;
/// `CTheScripts::bPlayerIsOffTheMap`
const SCRIPTS_PLAYER_OFF_THE_MAP: usize = 0xA444A4;
/// `CPools::ms_pVehiclePool` (0xA18), `ms_pPedPool` (0x7C4), `ms_pObjectPool` (0x19C)
const VEHICLE_POOL: usize = 0xB74494;
const PED_POOL: usize = 0xB74490;
const OBJECT_POOL: usize = 0xB7449C;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CPickups::aPickUps` (0x20 each)
const PICKUPS: usize = 0x9788C0;
/// `FrontEndMenuManager` bytes / ints `DrawBlips` reads on the map: +0x78 (0xBA67C0, map cursor shown),
/// the cursor world position (+0x70 / +0x74, 0xBA67B8 / 0xBA67BC), the mouse position (+0xBC / +0xC0,
/// 0xBA6804 / 0xBA6808) and +0xB8 (0xBA6800)
const MENU_MAP_CURSOR_ON: usize = 0xBA67C0;
const MENU_MAP_CURSOR_X: usize = 0xBA67B8;
const MENU_MAP_CURSOR_Y: usize = 0xBA67BC;
const MENU_MOUSE_X: usize = 0xBA6804;
const MENU_MOUSE_Y: usize = 0xBA6808;
const MENU_CURSOR_FLAG: usize = 0xBA6800;
/// Byte 0xB73418 (pad state `DrawBlips` tests before the mouse range)
const PAD_MOUSE_FLAG: usize = 0xB73418;
/// `CRadar::m_fRadarOrientation`
const RADAR_ORIENTATION: usize = 0xBA8310;
/// `TheCamera.m_nActiveCam` (byte) and `TheCamera.m_aCams[i].m_nMode` (u16, stride 0x238)
const CAMERA_ACTIVE_CAM: usize = 0xB6F081;
const CAM_MODE: usize = 0xB6F1A8;
/// Two `TheCamera` matrix rows `ShowRadarMarker` uses: 0xB6F9BC (A) and 0xB6F99C (B)
const CAM_VEC_A: usize = 0xB6F9BC;
const CAM_VEC_B: usize = 0xB6F99C;
/// `CTheScripts::DbgFlag` (.data byte; markers are drawn only while it is set)
const SCRIPTS_DBG_FLAG: usize = 0x859CF8;
/// `CTheScripts::RadarZoomValue` (non-zero: blip alpha distance forced to 255)
const SCRIPTS_RADAR_ZOOM_VALUE: usize = 0xA444A3;

// .rdata float constants (bits)
/// `1/640` (0x859520)
const K_1_640: u32 = 0x3ACC_CCCD;
/// `1/448` (0x859524)
const K_1_448: u32 = 0x3B12_4925;
/// `8.0` (0x859000)
const K_8: u32 = 0x4100_0000;
/// `2^32` (0x858C54)
const K_2_32: u32 = 0x4F80_0000;
/// `0.1` (0x858B1C)
const K_0_1: u32 = 0x3DCC_CCCD;
/// `1.0` (0x858624)
const K_1: u32 = 0x3F80_0000;
/// `2.0` (0x858CA0)
const K_2: u32 = 0x4000_0000;
/// `4.0` (0x858B90)
const K_4: u32 = 0x4080_0000;
/// `0.005` (0x858B4C)
const K_0_005: u32 = 0x3BA3_D70A;
/// `0.3` (0x858C24)
const K_0_3: u32 = 0x3E99_999A;
/// `0.9` (0x858C20)
const K_0_9: u32 = 0x3F66_6666;
/// `16.666668` (0x866B8C)
const K_16_67: u32 = 0x4185_5556;
/// `340.0` (0x85F074)
const K_340: u32 = 0x43AA_0000;
/// `283.33334` (0x866B88)
const K_283_33: u32 = 0x438D_AAAB;
/// `180.0` (0x85A994)
const K_180: u32 = 0x4334_0000;
/// `3000.0` (0x859A94)
const K_3000: u32 = 0x453B_8000;
/// `0.002` (0x858F44)
const K_0_002: u32 = 0x3B03_126F;
/// `11.0` (0x858FBC)
const K_11: u32 = 0x4130_0000;
/// `1000.0` (0x858C4C)
const K_1000: u32 = 0x447A_0000;
/// `57.295776` (0x859878)
const K_RAD2DEG: u32 = 0x4265_2EE0;
/// `2000.0` (0x8631F0)
const K_2000: u32 = 0x44FA_0000;
/// `pi` as the double at 0x866B80 (`fld qword`)
const D_PI_F32: u64 = 0x4009_21FB_6000_0000;
/// `pi/12` (0x858F1C)
const K_PI_12: u32 = 0x3E86_0A92;
/// `2pi/1024` (0x865034)
const K_2PI_1024: u32 = 0x3BC9_0FDB;
/// `-0.002` (0x866B7C)
const K_M0_002: u32 = 0xBB03_126F;
/// `0.6` (0x858CC8), `1.2` (0x858F08), `2.7` (0x859AA8), `1.8` (0x859BB4), `1.6` (0x858FA8)
const K_0_6: u32 = 0x3F19_999A;
const K_1_2: u32 = 0x3F99_999A;
const K_2_7: u32 = 0x402C_CCCD;
const K_1_8: u32 = 0x3FE6_6666;
const K_1_6: u32 = 0x3FCC_CCCD;
/// `60.0` (0x858B34), `sqrt(2)` (0x866BA4), `pi` (0x858CB8)
const K_60: u32 = 0x4270_0000;
const K_SQRT2: u32 = 0x3FB5_04F3;
const K_PI: u32 = 0x4049_0FDB;
/// `1.4` (0x859994)
const K_1_4: u32 = 0x3FB3_3333;
/// `0.5` (0x858B8C)
const K_HALF: u32 = 0x3F00_0000;

fn k(bits: u32) -> f32 {
    f32::from_bits(bits)
}

fn b(addr: usize) -> u8 {
    g::<u8>(addr).read()
}

/// `fild dword` of an unsigned value, as MSVC converts it (`+ 2^32` when the sign bit is set).
fn fild_u32(v: u32) -> X87 {
    let r = xi(v as i32);
    if (v as i32) < 0 { r.fadd_m(k(K_2_32)) } else { r }
}

/// A local buffer as a game pointer (the original's stack temporaries).
fn local<T, const N: usize>(buf: &mut [T; N]) -> GPtr<T> {
    GPtr::from_raw(buf.as_mut_ptr())
}

/// Entry `i` of `ms_RadarTrace` (`i * 0x28`, wrapping).
fn trace(i: i32) -> GPtr<u8> {
    g::<u8>(RADAR_TRACE).byte_add(i.wrapping_mul(0x28) as isize)
}

orig! {
    /// `CRadar::LimitToMap(float* x, float* y)` (0x583350)
    fn limit_to_map_call(px: GPtr<f32>, py: GPtr<f32>) = cdecl 0x583350 => crate::radar_draw::limit_to_map;
    /// `CRadar::DisplayThisBlip(eRadarSprite, int8 priority)` (0x583B40)
    fn display_this_blip_call(sprite: i32, prio: u8) -> u8 = cdecl 0x583B40 => crate::radar::display_this_blip;
    /// `CRadar::AddBlipToLegendList(bool noSprite, int32 index)` (0x5859F0)
    fn add_blip_to_legend_list_call(no_sprite: u32, index: i32) = cdecl 0x5859F0 => crate::radar_draw::add_blip_to_legend_list;
    /// `CRGBA::CRGBA(r, g, b, a)` (0x7170C0)
    fn rgba_ctor(this: GPtr<u8>, r: u32, gg: u32, bb: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `CSprite2d::Draw(const CRect&, const CRGBA&)` (0x728350)
    fn sprite2d_draw_rect(this: GPtr<u8>, rect: GPtr<f32>, colour: GPtr<u8>) = thiscall 0x728350 => unported;
    /// `CTheScripts::IsPlayerOnAMission()` (0x464D50)
    fn is_player_on_a_mission() -> u8 = cdecl 0x464D50 => unported;
    /// `CRadar::GetRadarTraceColour(eBlipColour, bool bright, bool friendly)` (0x584770)
    fn get_radar_trace_colour_call(colour: u32, bright: u32, friendly: u32) -> u32 = cdecl 0x584770 => crate::radar_draw::get_radar_trace_colour;
    /// `CHudColours::GetIntColour(uint8)` (0x58FD50, thiscall)
    fn get_int_colour_call(this: GPtr<u8>, colour: u32) -> u32 = thiscall 0x58FD50 => unported;
    /// `CRadar::ShowRadarMarker(CVector pos, uint32 colour, float radius)` (0x584480)
    fn show_radar_marker_call(px: u32, py: u32, pz: u32, colour: u32, radius: u32) = cdecl 0x584480 => show_radar_marker;
    /// `CTheScripts::ScriptDebugLine3D(x1, y1, z1, x2, y2, z2, uint32 colour1, uint32 colour2)` (0x485DE0)
    fn script_debug_line_3d(x1: f32, y1: f32, z1: f32, x2: f32, y2: f32, z2: f32, c1: u32, c2: u32) = cdecl 0x485DE0 => unported;
    /// `CEntryExit::GetPositionRelativeToOutsideWorld(CVector&)` (0x43EA00, thiscall)
    fn ee_get_position_relative_to_outside_world(this: GPtr<u8>, pos: GPtr<f32>) = thiscall 0x43EA00 => sa_world::world::entryexit::get_position_relative_to_outside_world;
    /// `CRadar::TransformRealWorldPointToRadarSpace(CVector2D& out, const CVector2D& in)` (0x583530)
    fn transform_real_world_point_to_radar_space_call(out: GPtr<f32>, inp: GPtr<f32>) = cdecl 0x583530 => crate::radar::transform_real_world_point_to_radar_space;
    /// `CRadar::TransformRadarPointToScreenSpace(CVector2D& out, const CVector2D& in)` (0x583480; preserves EDX)
    fn transform_radar_point_to_screen_space_call(out: GPtr<f32>, inp: GPtr<f32>) = cdecl 0x583480 => crate::radar::transform_radar_point_to_screen_space;
    /// `CRadar::LimitRadarPoint(CVector2D&)` (0x5832F0, ST0)
    fn limit_radar_point_call_st0(p: GPtr<f32>) -> f64 = cdecl 0x5832F0 => limit_radar_point_call_target_f64;
    /// `CRadar::CalculateBlipAlpha(float)` (0x583420)
    fn calculate_blip_alpha_call(dist: f32) -> u8 = cdecl 0x583420 => crate::radar::calculate_blip_alpha;
    /// `CRadar::HasThisBlipBeenRevealed(int32)` (0x583AF0)
    fn has_this_blip_been_revealed_call(i: i32) -> u8 = cdecl 0x583AF0 => crate::radar::has_this_blip_been_revealed;
    /// `CRadar::DrawRadarSprite(uint16 sprite, float x, float y, uint8 alpha)` (0x585FF0)
    fn draw_radar_sprite_call(sprite: u32, px: f32, py: f32, alpha: u32) = cdecl 0x585FF0 => draw_radar_sprite;
    /// `FindPlayerCentreOfWorldForMap(CVector* out, int32 id)` (0x56E400)
    fn find_player_centre_of_world_for_map(out: GPtr<f32>, id: i32) -> GPtr<f32> = cdecl 0x56E400 => sa_world::world::cworld::find_player_centre_of_world_for_map;
    /// `CRadar::CalculateCachedSinCos()` (0x583670)
    fn calculate_cached_sin_cos_call() = cdecl 0x583670 => crate::radar_draw::calculate_cached_sin_cos;
    /// `FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, remote: u8) -> GPtr<u8> = cdecl 0x56E0D0 => sa_world::world::cworld::find_player_vehicle_port;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => sa_world::world::cworld::find_player_ped_port;
    /// `FindPlayerSpeed(int32)` (0x56E090)
    fn find_player_speed(id: i32) -> GPtr<u8> = cdecl 0x56E090 => sa_world::world::cworld::find_player_speed;
    /// `CPlayerInfo::IsPlayerInRemoteMode()` (0x56DAB0)
    fn player_in_remote_mode(this: GPtr<u8>) -> u8 = thiscall 0x56DAB0 => unported;
    /// `CGameLogic::IsCoopGameGoingOn()` (0x441390)
    fn is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
    /// `C3dMarkers::PlaceMarkerSet(id, type, CVector& pos, size, r, g, b, a, pulsePeriod, pulseFraction, rotateRate)` (0x725BA0)
    fn place_marker_set(id: u32, ty: u32, pos: GPtr<f32>, size: u32, r: u32, gg: u32, bb: u32, a: u32, period: u32, fraction: u32, rotate: u32) = cdecl 0x725BA0 => unported;
    /// `C3dMarkers::PlaceMarkerCone(id, CVector& pos, size, r, g, b, a, pulsePeriod, pulseFraction, rotateRate, collision)` (0x726D40)
    fn place_marker_cone(id: u32, pos: GPtr<f32>, size: u32, r: u32, gg: u32, bb: u32, a: u32, period: u32, fraction: u32, rotate: u32, collision: u32) = cdecl 0x726D40 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CPickups::GetActualPickupIndex(int32)` (0x4552A0)
    fn get_actual_pickup_index(h: i32) -> i32 = cdecl 0x4552A0 => unported;
    /// `CPickup::GetPosn(CVector* out)` (0x4549A0, thiscall, returns `out`)
    fn pickup_get_posn(this: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x4549A0 => unported;
    /// `CHudColours::GetRGB(CRGBA* out, uint8)` (0x58FEA0)
    fn hud_colours_get_rgb(this: GPtr<u8>, out: GPtr<u8>, colour: u32) -> GPtr<u8> = thiscall 0x58FEA0 => crate::hud::get_rgb;
    /// `CRadar::SetupAirstripBlips()` (0x587D20)
    fn setup_airstrip_blips_call() = cdecl 0x587D20 => crate::radar_draw::setup_airstrip_blips;
    /// `CPad::GetPad(int32)` (0x53FB70; result unused by `DrawBlips`)
    fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CRadar::DrawEntityBlip(int32 index, uint8 isSprite)` (0x587000)
    fn draw_entity_blip_call(index: i32, flag: u32) = cdecl 0x587000 => unported;
    /// `CRadar::DrawCoordBlip(int32 index, bool isSprite)` (0x586D60)
    fn draw_coord_blip_call(index: i32, flag: u8) = cdecl 0x586D60 => draw_coord_blip;
    /// `CRadar::DrawYouAreHereSprite(float x, float y)` (0x584960)
    fn draw_you_are_here_sprite_call(px: f32, py: f32) = cdecl 0x584960 => crate::radar_draw::draw_you_are_here_sprite;
    /// `FindPlayerHeading(int32)` (0x56E450, ST0)
    fn find_player_heading_st0(id: i32) -> f64 = cdecl 0x56E450 => unported;
    /// `CPlayerPed::IsHidden()` (0x609620)
    fn player_ped_is_hidden(ped: GPtr<u8>) -> u8 = thiscall 0x609620 => unported;
    /// `CRadar::DrawRotatingRadarSprite(CSprite2d*, x, y, angle, width, height, CRGBA)` (0x584850)
    fn draw_rotating_radar_sprite_call(sprite: GPtr<u8>, px: f32, py: f32, angle: f32, width: u32, height: u32, colour: u32) = cdecl 0x584850 => crate::radar_draw::draw_rotating_radar_sprite;
    /// `CRadar::DrawRadarMap()` (0x586880)
    fn draw_radar_map_call() = cdecl 0x586880 => draw_radar_map;
    /// `CRadar::DrawRadarMask()` (0x585700)
    fn draw_radar_mask_call() = cdecl 0x585700 => draw_radar_mask;
    /// `GetFirstTexture(RwTexDictionary*)` (0x734940)
    fn get_first_texture(dict: GPtr<u8>) -> GPtr<u8> = cdecl 0x734940 => unported;
    /// `CSprite2d::SetVertices(const CRect&, const CRGBA&, const CRGBA&, const CRGBA&, const CRGBA&)` (0x727420)
    fn sprite2d_set_vertices(rect: GPtr<f32>, c1: GPtr<u8>, c2: GPtr<u8>, c3: GPtr<u8>, c4: GPtr<u8>) = cdecl 0x727420 => unported;
    /// `RwIm2DRenderPrimitive(RwPrimitiveType, RwIm2DVertex*, int32)` (0x734E90)
    fn rw_im2d_render_primitive(prim: u32, verts: GPtr<u8>, n: i32) -> u32 = cdecl 0x734E90 => unported;
    /// `CSprite2d::SetMaskVertices(int32 n, float* verts, float z)` (0x7279B0)
    fn sprite2d_set_mask_vertices(n: i32, verts: GPtr<f32>, z: u32) = cdecl 0x7279B0 => unported;
    /// CRT `floor(double)` (0x8219F0)
    fn crt_floor(v: f64) -> f64 = cdecl 0x8219F0 => unported;
    /// CRT `ceil(double)` (0x823820)
    fn crt_ceil(v: f64) -> f64 = cdecl 0x823820 => unported;
    /// `CRadar::SetupRadarRect(int32, int32)` (0x584A80)
    fn setup_radar_rect_call(tx: i32, ty: i32) = cdecl 0x584A80 => crate::radar::setup_radar_rect;
    /// `CRadar::StreamRadarSections(int32, int32)` (0x584C50)
    fn stream_radar_sections_xy_call(tx: i32, ty: i32) = cdecl 0x584C50 => crate::radar::stream_radar_sections_xy;
    /// `RwRenderStateSet` through `RwEngineInstance` (`call [[0xC97B24]+0x20]`)
    fn rw_render_state_set(state: u32, value: u32) -> u32 = cdecl table 0xC97B24 0x20 => unported;
    /// `CRadar::DrawRadarSection(int32, int32)` (0x586110)
    fn draw_radar_section_call(px: i32, py: i32) = cdecl 0x586110 => draw_radar_section;
    /// `CRadar::GetTextureCorners(int32, int32, CVector2D*)` (0x584D90)
    fn get_texture_corners_call(tx: i32, ty: i32, out: GPtr<f32>) = cdecl 0x584D90 => crate::radar::get_texture_corners;
    /// `CRadar::DrawRadarGangOverlay(bool inMenu)` (0x586650)
    fn draw_radar_gang_overlay_call(in_menu: u32) = cdecl 0x586650 => draw_radar_gang_overlay;
    /// `CGangWars::CanPlayerStartAGangWarHere(CZoneInfo*)` (0x443F80)
    fn can_player_start_a_gang_war_here(zi: GPtr<u8>) -> u8 = cdecl 0x443F80 => unported;
    /// `CRadar::DrawAreaOnRadar(const CRect&, const CRGBA&, bool inMenu)` (0x5853D0)
    fn draw_area_on_radar_call(rect: GPtr<f32>, colour: GPtr<u8>, in_menu: u8) = cdecl 0x5853D0 => draw_area_on_radar;
    /// `CRadar::ClipRadarPoly(CVector2D* out, const CVector2D* in)` (0x585040)
    fn clip_radar_poly_call(out: GPtr<u8>, inp: GPtr<u8>) -> i32 = cdecl 0x585040 => crate::radar_draw::clip_radar_poly;
    /// `CSprite2d::SetVertices(int32 n, float* pos, float* uv, const CRGBA&)` (0x727890)
    fn sprite2d_set_vertices_n(n: i32, pos: GPtr<f32>, uv: GPtr<f32>, colour: GPtr<u8>) = cdecl 0x727890 => unported;
    /// `CRadar::ShowRadarTraceWithHeight(float x, float y, uint32 size, uint8 r, uint8 g, uint8 b, uint8 a,
    /// eRadarTraceHeight)` (0x584070)
    fn show_radar_trace_with_height_call(px: f32, py: f32, size: u32, r: u32, gg: u32, bb: u32, a: u32, mode: u32) = cdecl 0x584070 => show_radar_trace_with_height;
    /// `CSprite2d::DrawRect(const CRect&, const CRGBA&)` (0x727B60)
    fn sprite2d_draw_rect_static(rect: GPtr<f32>, colour: GPtr<u8>) = cdecl 0x727B60 => unported;
    /// `CSprite2d::Draw2DPolygon(x1, y1, x2, y2, x3, y3, x4, y4, const CRGBA&)` (0x7285B0)
    fn sprite2d_draw_2d_polygon(x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, x4: f32, y4: f32, colour: GPtr<u8>) = cdecl 0x7285B0 => unported;
}

/// `CRadar::DrawRadarSprite(uint16 sprite, float x, float y, uint8 alpha)` (0x585FF0): on the map the
/// point is scaled (`* width / 640`, `* height / 448`) and limited by `LimitToMap`; half size
/// `ftol(sx * 8)` x `ftol(sy * 8)` (unsigned `fild`); `RadarBlipSprites[sprite]` drawn white with `alpha`
/// over {x - w, h + y, w + x, y - h}, then `AddBlipToLegendList(false, sprite)`. Only when
/// `DisplayThisBlip(sprite, -99)`.
#[hook(addr = 0x585FF0, conv = cdecl, name = "CRadar::DrawRadarSprite", expect = "A0 A1 67 BA 00 83 EC 10")]
pub fn draw_radar_sprite(sprite: u32, px: f32, py: f32, alpha: u32) {
    let (mut px, mut py) = (px, py);
    if b(MENU_DRAWING_MAP) != 0 {
        px = f32_of(xi(g::<i32>(RS_MAX_WIDTH).read()).fmul_m(k(K_1_640)).fmul_m(px));
        py = f32_of(xi(g::<i32>(RS_MAX_HEIGHT).read()).fmul_m(k(K_1_448)).fmul_m(py));
        limit_to_map_call(GPtr::from_raw(&mut px as *mut f32), GPtr::from_raw(&mut py as *mut f32));
    }
    let w = ftol32(xi(g::<i32>(RS_MAX_WIDTH).read()).fmul_m(k(K_1_640)).fmul_m(k(K_8))) as u32;
    let h = ftol32(xi(g::<i32>(RS_MAX_HEIGHT).read()).fmul_m(k(K_1_448)).fmul_m(k(K_8))) as u32;
    let s = sprite & 0xFFFF;
    if display_this_blip_call(s as i32, 0x9D) == 0 {
        return;
    }
    let fw = fild_u32(w);
    let left = f32_of(x(px).fsub(fw));
    let fh = fild_u32(h);
    let top = f32_of(x(py).fsub(fh));
    let right = f32_of(fw.fadd_m(px));
    let bottom = f32_of(fh.fadd_m(py));
    let mut rect = [left, bottom, right, top];
    let mut c = [0u8; 4];
    let cp = rgba_ctor(local(&mut c), 0xFF, 0xFF, 0xFF, alpha);
    sprite2d_draw_rect(g(RADAR_BLIP_SPRITES + s as usize * 4), local(&mut rect), cp);
    add_blip_to_legend_list_call(0, s as i32);
}

/// `CRadar::DrawCoordBlip(int32 index, bool isSprite)` (0x586D60). Byte findings: spotlight blips (type 5)
/// are skipped while on a mission; markers (display 1/3) are drawn only with `CTheScripts::DbgFlag`; the
/// sprite pass needs `(isSprite byte) != (sprite == 0)`; a short-range sprite farther than 1.0 shows only on
/// the map; the trace pass uses the height mode 0 (above: z - 2 > player z), 1 (below: z + 4 < player z) or
/// 2, and passes the colour words `c >> 24`, `c >> 16`, `c >> 8` unmasked. The alpha argument's upper three
/// bytes are an uninitialised stack local in the original (the port passes 0 there) and the mode word's
/// upper bytes are those of the blip Z (the argument slot is reused), as the bytes.
#[hook(addr = 0x586D60, conv = cdecl, name = "CRadar::DrawCoordBlip", expect = "83 EC 24 53 56 57")]
pub fn draw_coord_blip(index: i32, is_sprite: u8) {
    let t = trace(index);
    if rd::<u8>(t, 0x26) & 0x3C == 0x14 && is_player_on_a_mission() != 0 {
        return;
    }
    let flags = rd::<u8>(t, 0x25);
    let colour = match flags >> 6 {
        0 => get_radar_trace_colour_call(rd::<u32>(t, 0), u32::from(flags & 1), 0),
        1 => get_int_colour_call(g(HUD_COLOUR), 0xD),
        _ => get_int_colour_call(g(HUD_COLOUR), 0),
    };
    let display = rd::<u8>(t, 0x26) & 3;
    if (display == 3 || display == 1) && b(SCRIPTS_DBG_FLAG) != 0 {
        show_radar_marker_call(rd::<u32>(t, 8), rd::<u32>(t, 0xC), rd::<u32>(t, 0x10), colour, rd::<u32>(t, 0x18));
        let r = x(rd::<f32>(t, 0x18)).fsub_m(k(K_0_1));
        wr::<f32>(t, 0x18, f32_of(r));
        if !r.fcom(x(k(K_1))).test_parity(0x05) {
            wr::<u32>(t, 0x18, 0x40A0_0000);
        }
    }
    let no_sprite = u32::from(rd::<u8>(t, 0x24) == 0);
    if u32::from(is_sprite) == no_sprite {
        return;
    }
    let display = rd::<u8>(t, 0x26) & 3;
    if display != 3 && display != 2 {
        return;
    }
    let mut pos = [rd::<f32>(t, 8), rd::<f32>(t, 0xC), rd::<f32>(t, 0x10)];
    let ee = rd::<GPtr<u8>>(t, 0x20);
    if !ee.is_null() {
        ee_get_position_relative_to_outside_world(ee, local(&mut pos));
    }
    let mut world = [pos[0], pos[1]];
    let mut radar = [0f32; 2];
    transform_real_world_point_to_radar_space_call(local(&mut radar), local(&mut world));
    let mut dist = f32_of(limit_radar_point_call(local(&mut radar)));
    let calc_alpha = calculate_blip_alpha_call(dist);
    if b(SCRIPTS_RADAR_ZOOM_VALUE) != 0 {
        dist = k(0x437F_0000);
    }
    let flags = rd::<u8>(t, 0x25);
    let alpha = if flags & 0x20 == 0 { colour as u8 } else { calc_alpha };
    let mut screen = [0f32; 2];
    transform_radar_point_to_screen_space_call(local(&mut screen), local(&mut radar));
    if is_sprite != 0 {
        let sprite = rd::<u8>(t, 0x24);
        if sprite == 0 {
            return;
        }
        if flags & 4 != 0 && x(dist).fcom(x(k(K_1))).test_parity(0x41) && b(MENU_DRAWING_MAP) == 0 {
            return;
        }
        if has_this_blip_been_revealed_call(index) == 0 {
            return;
        }
        let s = i32::from(rd::<i8>(t, 0x24)) as u32 & 0xFFFF;
        draw_radar_sprite_call(s, screen[0], screen[1], 0xFF);
        return;
    }
    if rd::<u8>(t, 0x24) != 0 {
        return;
    }
    let z = pos[2];
    let mut player = [0f32; 3];
    find_player_centre_of_world_for_map(local(&mut player), 0);
    let mode: u32 = if !x(z).fsub_m(k(K_2)).fcom(x(player[2])).test_any(0x41) {
        0
    } else if !x(z).fadd_m(k(K_4)).fcom(x(player[2])).test_parity(0x05) {
        1
    } else {
        2
    };
    if b(MENU_DRAWING_MAP) != 0 && b(MENU_SHOW_TRACES) == 0 {
        return;
    }
    let mode_word = (z.to_bits() & 0xFFFF_FF00) | mode;
    let size = i32::from(rd::<i16>(t, 0x1C)) as u32;
    show_radar_trace_with_height_call(
        screen[0],
        screen[1],
        size,
        colour >> 24,
        colour >> 16,
        colour >> 8,
        u32::from(alpha),
        mode_word,
    );
    add_blip_to_legend_list_call(1, index);
}

/// The entity position `DrawMap` reads: `m_matrix->pos` (+0x30) when the matrix exists, else the
/// placement position (+4).
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// The (x, y) of a `CVector*` result.
fn xy(p: GPtr<f32>) -> (X87, X87) {
    (x(rd::<f32>(p.cast(), 0)), x(rd::<f32>(p.cast(), 4)))
}

/// `CRadar::DrawMap()` (0x586B00): cached sin/cos, the radar range (planes other than the Vortex by
/// height: 340 below `z * 0.005 < 0.3`, 350 from 0.9, else `(v - 0.3) * 16.67 + 340`; other vehicles by
/// speed: 180 below 0.3, 350 from 0.9, else `(v - 0.3) * 283.33 + 180`; on foot or in remote mode
/// `180 - RadarZoomValue` when that is set, else 180), the radar origin (the focused player's centre, in a
/// co-op game per 0x8A5E50: player 0, player 1 or their midpoint), then `DrawRadarMap` outside interiors
/// unless the radar shows blips only (a tail jump in the original).
#[hook(addr = 0x586B00, conv = cdecl, name = "CRadar::DrawMap", expect = "83 EC 34 E8 68 CB FF FF")]
pub fn draw_map() {
    calculate_cached_sin_cos_call();
    let range = g::<f32>(RADAR_RANGE);
    let focus = PLAYERS + usize::from(b(PLAYER_IN_FOCUS)) * 0x190;
    if !find_player_vehicle(-1, 0).is_null() && player_in_remote_mode(g(focus)) == 0 {
        let plane = !find_player_vehicle(-1, 0).is_null()
            && rd::<u32>(find_player_vehicle(-1, 0), 0x594) == 4
            && rd::<u16>(find_player_vehicle(-1, 0), 0x22) != 0x21B;
        if plane {
            let p = entity_pos(find_player_vehicle(-1, 0));
            let v = x(rd::<f32>(p, 8)).fmul_m(k(K_0_005));
            let l0 = f32_of(v);
            if !v.fcom(x(k(K_0_3))).test_parity(0x05) {
                range.write(k(K_340));
            } else if x(l0).fcom(x(k(K_0_9))).test_parity(0x05) {
                range.write(f32::from_bits(0x43AF_0000));
            } else {
                range.write(f32_of(x(l0).fsub_m(k(K_0_3)).fmul_m(k(K_16_67)).fadd_m(k(K_340))));
            }
        } else {
            let s = find_player_speed(-1);
            let (sx, sy, sz) = (x(rd::<f32>(s, 0)), x(rd::<f32>(s, 4)), x(rd::<f32>(s, 8)));
            let v = sx.fmul(sx).fadd(sy.fmul(sy)).fadd(sz.fmul(sz)).fsqrt();
            let l0 = f32_of(v);
            if !v.fcom(x(k(K_0_3))).test_parity(0x05) {
                range.write(k(K_180));
            } else if x(l0).fcom(x(k(K_0_9))).test_parity(0x05) {
                range.write(f32::from_bits(0x43AF_0000));
            } else {
                range.write(f32_of(x(l0).fsub_m(k(K_0_3)).fmul_m(k(K_283_33)).fadd_m(k(K_180))));
            }
        }
    } else {
        let zoom = b(SCRIPTS_RADAR_ZOOM_VALUE);
        if zoom != 0 {
            range.write(f32_of(xi(0xB4 - i32::from(zoom))));
        } else {
            range.write(k(K_180));
        }
    }
    let mut c0 = [0f32; 3];
    let mut c1 = [0f32; 3];
    let (ox, oy) = if is_coop_game_going_on() == 0 {
        xy(find_player_centre_of_world_for_map(local(&mut c0), 0))
    } else {
        match g::<i32>(COOP_RADAR_CENTRE).read() {
            0 => xy(find_player_centre_of_world_for_map(local(&mut c0), 0)),
            1 => xy(find_player_centre_of_world_for_map(local(&mut c0), 1)),
            _ => {
                let a = find_player_centre_of_world_for_map(local(&mut c1), 1);
                let bp = find_player_centre_of_world_for_map(local(&mut c0), 0);
                let sx = x(rd::<f32>(bp.cast(), 0)).fadd_m(rd::<f32>(a.cast(), 0));
                let sy = f32_of(x(rd::<f32>(bp.cast(), 4)).fadd_m(rd::<f32>(a.cast(), 4)));
                (sx.fmul_m(k(K_HALF)), x(sy).fmul_m(k(K_HALF)))
            }
        }
    };
    let area = g::<i32>(CURR_AREA).read();
    g::<f32>(RADAR_ORIGIN_X).write(f32_of(ox));
    g::<f32>(RADAR_ORIGIN_Y).write(f32_of(oy));
    if area == 0 && rd::<u8>(find_player_ped(-1), 0x2F) == 0 && g::<u32>(MENU_RADAR_MODE).read() != 1 {
        draw_radar_map_call();
    }
}

/// `CRadar::DrawRadarMap()` (0x586880): the radar mask, the tile `(floor((x + 3000) * 0.002),
/// ceil(11 - (y + 3000) * 0.002))` (CRT calls on the doubles), `SetupRadarRect` and `StreamRadarSections`
/// for it, ten render states, the 3 x 3 sections around it (rows ty-1, ty, ty+1), the gang overlay and, in a
/// plane other than the Vortex, the green horizon area (`DrawAreaOnRadar` of {x - 1000, y + 2000, x + 1000,
/// y - 2 * 57.29578 * atan2(-forward.z, up.z)} in CRGBA(20, 175, 20, 200)) drawn with the cached sin/cos
/// temporarily set to `sin(pi)` / `cos(pi)` of the f32 pi and restored afterwards.
#[hook(addr = 0x586880, conv = cdecl, name = "CRadar::DrawRadarMap", expect = "83 EC 2C 53 55 56 57")]
pub fn draw_radar_map() {
    draw_radar_mask_call();
    let ox = x(g::<f32>(RADAR_ORIGIN_X).read()).fadd_m(k(K_3000));
    let oyf = f32_of(x(g::<f32>(RADAR_ORIGIN_Y).read()).fadd_m(k(K_3000)));
    let tx = ftol32(X87::from_f64_exact(crt_floor(ox.fmul_m(k(K_0_002)).to_f64())));
    let ty = ftol32(X87::from_f64_exact(crt_ceil(x(oyf).fmul_m(k(K_0_002)).fsubr_m(k(K_11)).to_f64())));
    setup_radar_rect_call(tx, ty);
    stream_radar_sections_xy_call(tx, ty);
    for (st, v) in [(0xE, 0), (0xA, 5), (0xB, 6), (9, 2), (7, 1), (6, 1), (8, 0), (0xC, 0), (2, 3), (5, 0)] {
        rw_render_state_set(st, v);
    }
    for dy in [-1, 0, 1] {
        for dx in [-1, 0, 1] {
            draw_radar_section_call(tx.wrapping_add(dx), ty.wrapping_add(dy));
        }
    }
    draw_radar_gang_overlay_call(0);
    let plane = !find_player_vehicle(-1, 0).is_null()
        && rd::<u32>(find_player_vehicle(-1, 0), 0x594) == 4
        && rd::<u16>(find_player_vehicle(-1, 0), 0x22) != 0x21B;
    if plane {
        let mut pos = [0f32; 3];
        find_player_centre_of_world_for_map(local(&mut pos), 0);
        let pi = X87::from_f64_exact(f64::from_bits(D_PI_F32));
        let saved_sin = g::<u32>(CACHED_SIN).read();
        let saved_cos = g::<u32>(CACHED_COS).read();
        g::<f32>(CACHED_SIN).write(f32_of(pi.fsin()));
        g::<f32>(CACHED_COS).write(f32_of(pi.fcos()));
        let m_up = rd::<GPtr<u8>>(find_player_vehicle(-1, 0), 0x14);
        let m_fwd = rd::<GPtr<u8>>(find_player_vehicle(-1, 0), 0x14);
        let left = f32_of(x(pos[0]).fsub_m(k(K_1000)));
        let a = x(rd::<f32>(m_fwd, 0x18)).fchs().fpatan(x(rd::<f32>(m_up, 0x28)));
        let top = f32_of(a.fmul_m(k(K_RAD2DEG)).fdouble().fsubr_m(pos[1]));
        let right = f32_of(x(pos[0]).fadd_m(k(K_1000)));
        let bottom = f32_of(x(pos[1]).fadd_m(k(K_2000)));
        let mut rect = [left, bottom, right, top];
        let mut c = [0u8; 4];
        let cp = rgba_ctor(local(&mut c), 0x14, 0xAF, 0x14, 0xC8);
        draw_area_on_radar_call(local(&mut rect), cp, 0);
        g::<u32>(CACHED_SIN).write(saved_sin);
        g::<u32>(CACHED_COS).write(saved_cos);
    }
    rw_render_state_set(6, 0);
}

/// `CRadar::DrawRadarSectionMap(int32 x, int32 y, CRect rect)` (0x586520): for a tile inside 0..=11 x
/// 0..=11 with a loaded TXD, the dictionary's first texture as raster and `rect` drawn white
/// (`SetVertices` + `RwIm2DRenderPrimitive(trifan, maVertices, 4)`). As in `DrawWeaponIcon`, a free pool slot
/// gives entry 0 and the original reads `[0]`.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x586520, conv = cdecl, name = "CRadar::DrawRadarSectionMap", expect = "8B 54 24 04 83 EC 08")]
pub fn draw_radar_section_map(tx: i32, ty: i32, left: f32, bottom: f32, right: f32, top: f32) {
    let outside = !(0..=11).contains(&tx);
    if !(0..=11).contains(&ty) || outside {
        return;
    }
    let slot = g::<i32>(RADAR_TEXTURES + (ty * 12 + tx) as usize * 4).read();
    if slot == -1 {
        return;
    }
    let pool = g::<GPtr<u8>>(TXD_POOL).read();
    let entry = if rd::<i8>(rd::<GPtr<u8>>(pool, 4), slot as isize) < 0 {
        GPtr::<u8>::NULL
    } else {
        rd::<GPtr<u8>>(pool, 0).byte_add(slot as isize * 12)
    };
    let dict = rd::<GPtr<u8>>(entry, 0);
    if dict.is_null() {
        return;
    }
    let tex = get_first_texture(dict);
    if tex.is_null() {
        return;
    }
    rw_render_state_set(1, rd::<u32>(tex, 0));
    let mut c = [[0u8; 4]; 4];
    let mut p = [GPtr::<u8>::NULL; 4];
    for (i, cc) in c.iter_mut().enumerate() {
        p[i] = rgba_ctor(local(cc), 0xFF, 0xFF, 0xFF, 0xFF);
    }
    let mut rect = [left, bottom, right, top];
    sprite2d_set_vertices(local(&mut rect), p[3], p[2], p[1], p[0]);
    rw_im2d_render_primitive(5, g(SPRITE2D_VERTICES), 4);
}

/// `CRadar::DrawRadarMask()` (0x585700): ten render states, the unit square's screen corners as a
/// mask quad (z = `RwEngineInstance` +0x1C), then for each corner an 8-point fan (the corner and 7 points
/// at `j * pi/12`, `(cos * cx, sin * cy)`, z = [0xC80460]); render state 0x1D back to 5.
#[hook(addr = 0x585700, conv = cdecl, name = "CRadar::DrawRadarMask", expect = "55 8B EC 83 E4 F8")]
pub fn draw_radar_mask() {
    let corners: [[f32; 2]; 4] = [[-1.0, 1.0], [1.0, 1.0], [1.0, -1.0], [-1.0, -1.0]];
    for (st, v) in [(1, 0), (0xA, 5), (0xB, 6), (0xE, 0), (9, 2), (7, 1), (6, 0), (8, 1), (0xC, 1), (0x1D, 8)] {
        rw_render_state_set(st, v);
    }
    let mut screen = [0f32; 16];
    for (i, c) in corners.iter().enumerate() {
        let mut inp = *c;
        let out = GPtr::from_raw(screen[i * 2..].as_mut_ptr());
        transform_radar_point_to_screen_space_call(out, local(&mut inp));
    }
    let engine = g::<GPtr<u8>>(0xC97B24).read();
    sprite2d_set_mask_vertices(4, local(&mut screen), rd::<u32>(engine, 0x1C));
    rw_im2d_render_primitive(5, g(SPRITE2D_VERTICES), 4);
    for c in corners.iter() {
        let mut inp = *c;
        transform_radar_point_to_screen_space_call(local(&mut screen), local(&mut inp));
        for j in 0..7i32 {
            let a = xi(j).fmul_m(k(K_PI_12));
            let mut p = [f32_of(a.fcos().fmul_m(c[0])), f32_of(a.fsin().fmul_m(c[1]))];
            let out = GPtr::from_raw(screen[2 + j as usize * 2..].as_mut_ptr());
            transform_radar_point_to_screen_space_call(out, local(&mut p));
        }
        sprite2d_set_mask_vertices(8, local(&mut screen), g::<u32>(SPRITE2D_NEAR_SCREEN_Z).read());
        rw_im2d_render_primitive(5, g(SPRITE2D_VERTICES), 8);
    }
    rw_render_state_set(0x1D, 5);
}

/// `CRadar::DrawRadarGangOverlay(bool inMenu)` (0x586650): the function-local static rect (0xBAA35C,
/// default-constructed once: 1e6 / -1e6) takes each navigation zone's bounds (`int16` +0, +8, +6, +2 as
/// left, bottom, right, top); while gang wars are active and the menu shows gang areas, zones whose info
/// has gang-density flags (+0xF & 0x60) and where a gang war could start are drawn with `DrawAreaOnRadar`
/// in the zone colour (+0xB..+0xE): mode 1 solid, mode 2 with the alpha pulsing as
/// `(sin((t & 0x3FF) * 2pi/1024) + 1) * 0.5 * a` (pause-mode time on the map). The pulsing alpha argument's
/// upper three bytes are an uninitialised local in the original (the port passes 0 there).
#[hook(addr = 0x586650, conv = cdecl, name = "CRadar::DrawRadarGangOverlay", expect = "8A 0D 6C A3 BA 00")]
pub fn draw_radar_gang_overlay(in_menu: u32) {
    let guard = g::<u32>(GANG_RECT_GUARD).read();
    if guard as u8 & 1 == 0 {
        g::<u32>(GANG_RECT_GUARD).write(guard | 1);
        g::<u32>(GANG_RECT).write(0x4974_2400);
        g::<u32>(GANG_RECT + 0xC).write(0x4974_2400);
        g::<u32>(GANG_RECT + 8).write(0xC974_2400);
        g::<u32>(GANG_RECT + 4).write(0xC974_2400);
    }
    if b(GANG_WARS_ACTIVE) == 0 || b(MENU_SHOW_GANG_AREAS) == 0 {
        return;
    }
    let mut i: i32 = 0;
    while i < i32::from(g::<u16>(NAV_ZONE_COUNT).read()) {
        let z = g::<u8>(NAV_ZONES + i as usize * 0x20);
        let zi_index = rd::<u16>(z, 0xC);
        i += 1;
        if zi_index == 0 {
            continue;
        }
        let zi = g::<u8>(ZONE_INFO + usize::from(zi_index) * 0x11);
        if rd::<u8>(zi, 0xF) & 0x60 == 0 || can_player_start_a_gang_war_here(zi) == 0 {
            continue;
        }
        g::<f32>(GANG_RECT).write(f32_of(xi(i32::from(rd::<i16>(z, 0)))));
        g::<f32>(GANG_RECT + 8).write(f32_of(xi(i32::from(rd::<i16>(z, 6)))));
        g::<f32>(GANG_RECT + 4).write(f32_of(xi(i32::from(rd::<i16>(z, 8)))));
        g::<f32>(GANG_RECT + 0xC).write(f32_of(xi(i32::from(rd::<i16>(z, 2)))));
        let (r, gg, bb) = (u32::from(rd::<u8>(zi, 0xB)), u32::from(rd::<u8>(zi, 0xC)), u32::from(rd::<u8>(zi, 0xD)));
        let alpha = match (rd::<u8>(zi, 0xF) >> 5) & 3 {
            1 => u32::from(rd::<u8>(zi, 0xE)),
            2 => {
                let t = if b(MENU_DRAWING_MAP) == 0 { g::<u32>(TIME_MS).read() } else { g::<u32>(TIME_MS_PAUSE).read() } & 0x3FF;
                let v = fild_u32(t)
                    .fmul_m(k(K_2PI_1024))
                    .fsin()
                    .fadd_m(k(K_1))
                    .fmul_m(k(K_HALF))
                    .fmul(xi(i32::from(rd::<u8>(zi, 0xE))));
                u32::from(ftol32(v) as u8)
            }
            _ => continue,
        };
        let mut c = [0u8; 4];
        let cp = rgba_ctor(local(&mut c), r, gg, bb, alpha);
        draw_area_on_radar_call(g(GANG_RECT), cp, in_menu as u8);
    }
}

/// `TransformRealWorldPointToRadarSpace` of `(dx, dy)` inlined as `DrawAreaOnRadar` does it:
/// `(sin * dy + dx * cos, dy * cos - dx * sin)` (register order of the corner `dx`/`dy` values).
fn area_rotate(dx: X87, dy: X87, dx_mem: Option<f32>) -> (f32, f32) {
    let s = g::<f32>(CACHED_SIN).read();
    let c = g::<f32>(CACHED_COS).read();
    match dx_mem {
        None => (
            f32_of(x(s).fmul(dy).fadd(dx.fmul_m(c))),
            f32_of(dy.fmul_m(c).fsub(dx.fmul_m(s))),
        ),
        Some(dxf) => (
            f32_of(x(s).fmul(dy).fadd(x(dxf).fmul_m(c))),
            f32_of(dy.fmul_m(c).fsub(x(dxf).fmul_m(s))),
        ),
    }
}

/// `CRadar::DrawAreaOnRadar(const CRect&, const CRGBA&, bool inMenu)` (0x5853D0): unless `inMenu`,
/// rects wholly outside `m_radarRect` are skipped; the corners (left, top), (right, top), (right, bottom),
/// (left, bottom) are moved to radar space inline, clipped (`ClipRadarPoly`), mapped to the screen (scaled on
/// the map) and drawn untextured as a fan. Byte findings: the inverse rotation back to world space is
/// computed into a local and never used, and the `uv` buffer passed to `CSprite2d::SetVertices` is an
/// uninitialised stack local (scaled in place on the map); the port passes a zero-filled buffer there, so
/// the u/v words written to `CSprite2d::maVertices` differ from the original's stack garbage (untextured
/// draw, raster 0).
#[hook(addr = 0x5853D0, conv = cdecl, name = "CRadar::DrawAreaOnRadar", expect = "8A 44 24 0C 8B 4C 24 04")]
pub fn draw_area_on_radar(rect: GPtr<f32>, colour: GPtr<u8>, in_menu: u8) {
    let rr = |i: isize| g::<f32>(RADAR_RECT + i as usize * 4).read();
    let rc = |i: isize| rd::<f32>(rect.cast(), i * 4);
    if in_menu == 0
        && (!x(rc(0)).fcom(x(rr(2))).test_any(0x41)
            || !x(rc(2)).fcom(x(rr(0))).test_parity(0x05)
            || !x(rc(3)).fcom(x(rr(1))).test_any(0x41)
            || !x(rc(1)).fcom(x(rr(3))).test_parity(0x05))
    {
        return;
    }
    let (left, bottom, right, top) = (rc(0), rc(1), rc(2), rc(3));
    let ox = g::<f32>(RADAR_ORIGIN_X).read();
    let oy = g::<f32>(RADAR_ORIGIN_Y).read();
    let a0x = f32_of(x(left).fsub_m(ox));
    let a0y = f32_of(x(top).fsub_m(oy));
    let inv = x(k(K_1)).fdiv_m(g::<f32>(RADAR_RANGE).read());
    let mut poly = [0f32; 8];
    // (left, top): dx and dy in registers
    let (px, py) = area_rotate(x(a0x).fmul(inv), inv.fmul_m(a0y), None);
    poly[0] = px;
    poly[1] = py;
    // (right, top) and (right, bottom): dx through an f32 local
    for (i, (cx, cy)) in [(right, top), (right, bottom)].into_iter().enumerate() {
        let ax = x(cx).fsub_m(ox);
        let ay = x(cy).fsub_m(oy);
        let dxf = f32_of(inv.fmul(ax));
        let dy = inv.fmul(ay);
        let (px, py) = area_rotate(x(dxf), dy, Some(dxf));
        poly[2 + i * 2] = px;
        poly[3 + i * 2] = py;
    }
    // (left, bottom): dy from the f32 locals
    let ax = x(left).fsub_m(ox);
    let ayf = f32_of(x(bottom).fsub_m(oy));
    let invf = f32_of(inv);
    let dx = ax.fmul(inv);
    let dy = x(invf).fmul_m(ayf);
    let (px, py) = area_rotate(dx, dy, None);
    poly[6] = px;
    poly[7] = py;
    let mut clipped = [0f32; 16];
    let n = clip_radar_poly_call(local(&mut clipped).cast(), local(&mut poly).cast());
    if n == 0 {
        return;
    }
    let mut screen = [0f32; 16];
    let mut uv = [0f32; 16];
    let mut world = [0f32; 16];
    if n > 0 {
        let map = b(MENU_DRAWING_MAP);
        for i in 0..n.min(8) as usize {
            let c = x(g::<f32>(CACHED_COS).read());
            let s = x(g::<f32>(CACHED_SIN).read());
            let (cx, cy) = (clipped[i * 2], clipped[i * 2 + 1]);
            world[i * 2] = f32_of(
                c.fmul_m(cx).fsub(s.fmul_m(cy)).fmul_m(g::<f32>(RADAR_RANGE).read()).fadd_m(g::<f32>(RADAR_ORIGIN_X).read()),
            );
            world[i * 2 + 1] = f32_of(
                c.fmul_m(cy).fadd(s.fmul_m(cx)).fmul_m(g::<f32>(RADAR_RANGE).read()).fadd_m(g::<f32>(RADAR_ORIGIN_Y).read()),
            );
            let out = GPtr::from_raw(screen[i * 2..].as_mut_ptr());
            let inp = GPtr::from_raw(clipped[i * 2..].as_mut_ptr());
            transform_radar_point_to_screen_space_call(out, inp);
            if map != 0 {
                let sx = xi(g::<i32>(RS_MAX_WIDTH).read()).fmul_m(k(K_1_640));
                screen[i * 2] = f32_of(sx.fmul_m(screen[i * 2]));
                uv[i * 2] = f32_of(sx.fmul_m(uv[i * 2]));
                let sy = xi(g::<i32>(RS_MAX_HEIGHT).read()).fmul_m(k(K_1_448));
                screen[i * 2 + 1] = f32_of(sy.fmul_m(screen[i * 2 + 1]));
                uv[i * 2 + 1] = f32_of(sy.fmul_m(uv[i * 2 + 1]));
            }
        }
    }
    let _ = world;
    rw_render_state_set(0xC, 1);
    rw_render_state_set(1, 0);
    sprite2d_set_vertices_n(n, local(&mut screen), local(&mut uv), colour);
    if n > 2 {
        rw_im2d_render_primitive(5, g(SPRITE2D_VERTICES), n);
    }
}

/// The four corners of a rect or tile moved to radar space inline (`DrawAreaOnRadar` /
/// `DrawRadarSection`): corner 0 with dx, dy in registers, corners 1 and 2 with dx through an f32 local,
/// corner 3 with dy from the f32 locals.
fn corners_to_radar(c: &[[f32; 2]; 4]) -> [f32; 8] {
    let ox = g::<f32>(RADAR_ORIGIN_X).read();
    let oy = g::<f32>(RADAR_ORIGIN_Y).read();
    let inv = x(k(K_1)).fdiv_m(g::<f32>(RADAR_RANGE).read());
    let mut poly = [0f32; 8];
    let a0x = f32_of(x(c[0][0]).fsub_m(ox));
    let a0y = f32_of(x(c[0][1]).fsub_m(oy));
    let (px, py) = area_rotate(x(a0x).fmul(inv), inv.fmul_m(a0y), None);
    poly[0] = px;
    poly[1] = py;
    for i in 1..3 {
        let ax = x(c[i][0]).fsub_m(ox);
        let ay = x(c[i][1]).fsub_m(oy);
        let dxf = f32_of(inv.fmul(ax));
        let dy = inv.fmul(ay);
        let (px, py) = area_rotate(x(dxf), dy, Some(dxf));
        poly[i * 2] = px;
        poly[i * 2 + 1] = py;
    }
    let ax = x(c[3][0]).fsub_m(ox);
    let ayf = f32_of(x(c[3][1]).fsub_m(oy));
    let invf = f32_of(inv);
    let (px, py) = area_rotate(ax.fmul(inv), x(invf).fmul_m(ayf), None);
    poly[6] = px;
    poly[7] = py;
    poly
}

/// `CRadar::DrawRadarSection(int32 x, int32 y)` (0x586110): the tile's corners
/// (`GetTextureCorners`) moved to radar space, clipped, and drawn as a fan with u/v
/// `((wx - (x*500 - 3000)) * 0.002, (wy - ((12-y)*500 - 3000)) * -0.002)` of the clipped points taken back to
/// world space. Tiles outside 0..=11 are drawn untextured in CRGBA(111, 137, 170, 255), every tile while
/// the player is off the map in CRGBA(204, 204, 204, 255); a missing texture skips the tile.
#[hook(addr = 0x586110, conv = cdecl, name = "CRadar::DrawRadarSection", expect = "81 EC 30 01 00 00")]
pub fn draw_radar_section(tx: i32, ty: i32) {
    let mut c = [[0f32; 2]; 4];
    get_texture_corners_call(tx, ty, GPtr::from_raw(c.as_mut_ptr().cast()));
    let outside = !(0..=11).contains(&tx) || !(0..=11).contains(&ty);
    let mut tex = GPtr::<u8>::NULL;
    if !outside && b(SCRIPTS_PLAYER_OFF_THE_MAP) == 0 {
        let slot = g::<i32>(RADAR_TEXTURES + (ty * 12 + tx) as usize * 4).read();
        if slot == -1 {
            return;
        }
        let pool = g::<GPtr<u8>>(TXD_POOL).read();
        let entry = if rd::<i8>(rd::<GPtr<u8>>(pool, 4), slot as isize) < 0 {
            GPtr::<u8>::NULL
        } else {
            rd::<GPtr<u8>>(pool, 0).byte_add(slot as isize * 12)
        };
        let dict = rd::<GPtr<u8>>(entry, 0);
        if dict.is_null() {
            return;
        }
        tex = get_first_texture(dict);
        if tex.is_null() {
            return;
        }
    }
    let mut poly = corners_to_radar(&c);
    let mut clipped = [0f32; 16];
    let n = clip_radar_poly_call(local(&mut clipped).cast(), local(&mut poly).cast());
    if n == 0 {
        return;
    }
    let mut screen = [0f32; 16];
    let mut uv = [0f32; 16];
    if n > 0 {
        let base_x = f32_of(xi(tx.wrapping_mul(500)).fsub_m(k(K_3000)));
        let base_y = f32_of(xi(12i32.wrapping_sub(ty).wrapping_mul(500)).fsub_m(k(K_3000)));
        for i in 0..n.min(8) as usize {
            let cs = g::<f32>(CACHED_COS).read();
            let sn = g::<f32>(CACHED_SIN).read();
            let (cx, cy) = (clipped[i * 2], clipped[i * 2 + 1]);
            let range = g::<f32>(RADAR_RANGE).read();
            let wx = f32_of(x(cs).fmul_m(cx).fsub(x(sn).fmul_m(cy)).fmul_m(range).fadd_m(g::<f32>(RADAR_ORIGIN_X).read()));
            let wy = f32_of(x(sn).fmul_m(cx).fadd(x(cs).fmul_m(cy)).fmul_m(range).fadd_m(g::<f32>(RADAR_ORIGIN_Y).read()));
            uv[i * 2] = f32_of(x(wx).fsub_m(base_x).fmul_m(k(K_0_002)));
            uv[i * 2 + 1] = f32_of(x(wy).fsub_m(base_y).fmul_m(k(K_M0_002)));
            let out = GPtr::from_raw(screen[i * 2..].as_mut_ptr());
            let inp = GPtr::from_raw(clipped[i * 2..].as_mut_ptr());
            transform_radar_point_to_screen_space_call(out, inp);
        }
    }
    let mut col = [0u8; 4];
    let cp = if outside {
        rw_render_state_set(1, 0);
        rgba_ctor(local(&mut col), 0x6F, 0x89, 0xAA, 0xFF)
    } else if b(SCRIPTS_PLAYER_OFF_THE_MAP) != 0 {
        rw_render_state_set(1, 0);
        rgba_ctor(local(&mut col), 0xCC, 0xCC, 0xCC, 0xFF)
    } else {
        rw_render_state_set(1, rd::<u32>(tex, 0));
        rgba_ctor(local(&mut col), 0xFF, 0xFF, 0xFF, 0xFF)
    };
    sprite2d_set_vertices_n(n, local(&mut screen), local(&mut uv), cp);
    if n > 2 {
        rw_im2d_render_primitive(5, g(SPRITE2D_VERTICES), n);
    }
}

/// A pool slot by handle as the inlined `CPool::GetAt`: `objects + (h >> 8) * size` when the flag byte of
/// that slot equals the handle's low byte, else null.
fn pool_at(pool_ptr: usize, h: i32, size: i32) -> GPtr<u8> {
    let pool = g::<GPtr<u8>>(pool_ptr).read();
    let idx = h >> 8;
    if rd::<u8>(rd::<GPtr<u8>>(pool, 4), idx as isize) == h as u8 {
        rd::<GPtr<u8>>(pool, 0).byte_add(idx.wrapping_mul(size) as isize)
    } else {
        GPtr::NULL
    }
}

/// The entity position (`m_matrix->pos` or the placement position) as three f32.
fn ent_pos(e: GPtr<u8>) -> [f32; 3] {
    let m = rd::<GPtr<u8>>(e, 0x14);
    let p = if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) };
    [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)]
}

/// `CRadar::Draw3dMarkers()` (0x585BF0): for every used blip with display 1 or 3, a 3D marker: contact
/// points (type 5, not on a mission; with an entry/exit only inside, without one only outside) a marker
/// set (type 1, size 2, CRGBA(255, 0, 0, 228), pulse 0x800 / 0.2); vehicles a cone at `bbox.max.z *
/// (Nevada 0.6 else 1.2) + z + 2`, size 2; peds (their vehicle when in one) a cone at z + 2.7, size 1.2;
/// objects and pickups a cone at `z + height + 1.8` outside (1.6 inside), size 0.8 (height: the object's
/// col model `bbox.max.z`, 2.0 for pickups; an unknown pickup keeps position 0). Cones use the blip colour
/// words `c >> 24`, `c >> 16`, `c >> 8` (unmasked), alpha 255, pulse 0x400 / 0.2, rotate 5, collision on;
/// the marker id is `counter << 16 | index`. A stale vehicle / ped / object handle gives a null entity that
/// the original dereferences (as the port does).
#[hook(addr = 0x585BF0, conv = cdecl, name = "CRadar::Draw3dMarkers", expect = "83 EC 38 53 55")]
pub fn draw_3d_markers() {
    for i in 0..175i32 {
        let t = trace(i);
        let flags = rd::<u8>(t, 0x25);
        if flags & 2 == 0 {
            continue;
        }
        let colour = get_radar_trace_colour_call(rd::<u32>(t, 0), u32::from(flags & 1), u32::from((flags >> 3) & 1));
        let disp = rd::<u8>(t, 0x26);
        let shown = disp & 3 == 3 || disp & 3 == 1;
        let id = (u32::from(rd::<u16>(t, 0x14)) << 16) | i as u32;
        let (r, gg, bb) = (colour >> 24, colour >> 16, colour >> 8);
        match (disp >> 2) & 0xF {
            5 => {
                if is_player_on_a_mission() != 0 || !shown || find_player_ped(-1).is_null() {
                    continue;
                }
                let ee = rd::<u32>(t, 0x20);
                let inside = rd::<u8>(find_player_ped(-1), 0x2F) != 0;
                if ee != 0 {
                    if !inside {
                        continue;
                    }
                } else if inside {
                    continue;
                }
                place_marker_set(id, 1, t.byte_add(8).cast(), 0x4000_0000, 0xFF, 0, 0, 0xE4, 0x800, 0x3E4C_CCCD, 0);
            }
            1 => {
                let veh = pool_at(VEHICLE_POOL, rd::<i32>(t, 4), 0xA18);
                if !shown {
                    continue;
                }
                let h = x(rd::<f32>(entity_get_col_model(veh), 0x14));
                let mut pos = ent_pos(veh);
                let f = if rd::<u16>(veh, 0x22) == 0x229 { K_0_6 } else { K_1_2 };
                pos[2] = f32_of(h.fmul_m(k(f)).fadd_m(pos[2]).fadd_m(k(K_2)));
                place_marker_cone(id, local(&mut pos), 0x4000_0000, r, gg, bb, 0xFF, 0x400, 0x3E4C_CCCD, 5, 1);
            }
            2 => {
                let ped = pool_at(PED_POOL, rd::<i32>(t, 4), 0x7C4);
                let ent = if !ped.is_null() && rd::<u32>(ped, 0x46C) & 0x100 != 0 { rd::<GPtr<u8>>(ped, 0x58C) } else { ped };
                if !shown {
                    continue;
                }
                let mut pos = ent_pos(ent);
                pos[2] = f32_of(x(pos[2]).fadd_m(k(K_2_7)));
                place_marker_cone(id, local(&mut pos), 0x3F99_999A, r, gg, bb, 0xFF, 0x400, 0x3E4C_CCCD, 5, 1);
            }
            3 | 7 => {
                if !shown {
                    continue;
                }
                let mut pos = [0f32; 3];
                let height: f32;
                if disp & 0x3C == 0xC {
                    let obj = pool_at(OBJECT_POOL, rd::<i32>(t, 4), 0x19C);
                    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS.wrapping_add((i32::from(rd::<i16>(obj, 0x22)) * 4) as usize)).read();
                    height = rd::<f32>(rd::<GPtr<u8>>(mi, 0x14), 0x14);
                    pos = ent_pos(obj);
                } else {
                    height = k(K_2);
                    let idx = get_actual_pickup_index(rd::<i32>(t, 4));
                    if idx >= 0 {
                        let mut out = [0f32; 3];
                        let p = pickup_get_posn(g(PICKUPS + (idx as usize) * 0x20), local(&mut out));
                        pos = [rd::<f32>(p.cast(), 0), rd::<f32>(p.cast(), 4), rd::<f32>(p.cast(), 8)];
                    }
                }
                let outside = g::<i32>(CURR_AREA).read() == 0 && rd::<u8>(find_player_ped(-1), 0x2F) == 0;
                pos[2] = f32_of(x(pos[2]).fadd_m(height).fadd_m(k(if outside { K_1_8 } else { K_1_6 })));
                place_marker_cone(id, local(&mut pos), 0x3F4C_CCCD, r, gg, bb, 0xFF, 0x400, 0x3E4C_CCCD, 5, 1);
            }
            _ => {}
        }
    }
}

fn sx_x87() -> X87 {
    xi(g::<i32>(RS_MAX_WIDTH).read()).fmul_m(k(K_1_640))
}

fn sy_x87() -> X87 {
    xi(g::<i32>(RS_MAX_HEIGHT).read()).fmul_m(k(K_1_448))
}

/// `CRadar::ShowRadarTraceWithHeight(float x, float y, uint32 size, uint8 r, uint8 g, uint8 b, uint8 a,
/// eRadarTraceHeight mode)` (0x584070): on the map the point is scaled and limited (`LimitToMap`); raster
/// off; mode 2 (level) draws a black square of half size `size + 1` and the coloured one of `size`
/// (`DrawRect`, scaled by `sx`/`sy`); mode 0 (above) a black up-pointing triangle (half width
/// `(size + 3) * sx`, top `y - (size + 3) * sy`, base `y + (size + 2) * sy`) and the coloured one of
/// `size + 1`; mode 1 (below) the same pointing down (top `y - (size + 2) * sy`, tip `y + (size + 3) * sy`)
/// (`Draw2DPolygon` with the tip vertex repeated). Sizes are unsigned `fild`s.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x584070, conv = cdecl, name = "CRadar::ShowRadarTraceWithHeight", expect = "A0 A1 67 BA 00 83 EC 40")]
pub fn show_radar_trace_with_height(px: f32, py: f32, size: u32, r: u32, gg: u32, bb: u32, a: u32, mode: u32) {
    let (mut px, mut py) = (px, py);
    if b(MENU_DRAWING_MAP) != 0 {
        px = f32_of(xi(g::<i32>(RS_MAX_WIDTH).read()).fmul_m(k(K_1_640)).fmul_m(px));
        py = f32_of(xi(g::<i32>(RS_MAX_HEIGHT).read()).fmul_m(k(K_1_448)).fmul_m(py));
        limit_to_map_call(GPtr::from_raw(&mut px as *mut f32), GPtr::from_raw(&mut py as *mut f32));
    }
    rw_render_state_set(1, 0);
    let mut c = [0u8; 4];
    match mode as u8 {
        2 => {
            for (s, colour) in [(size.wrapping_add(1), [0, 0, 0, a]), (size, [r, gg, bb, a])] {
                let fs = fild_u32(s);
                let w = sx_x87().fmul(fs);
                let left = f32_of(x(px).fsub(w));
                let hf = f32_of(sy_x87().fmul(fs));
                let top = f32_of(x(py).fsub_m(hf));
                let right = f32_of(w.fadd_m(px));
                let bottom = f32_of(x(hf).fadd_m(py));
                let cp = rgba_ctor(local(&mut c), colour[0], colour[1], colour[2], colour[3]);
                let mut rect = [left, bottom, right, top];
                sprite2d_draw_rect_static(local(&mut rect), cp);
            }
        }
        1 => {
            let sy = sy_x87();
            let top = f32_of(fild_u32(size.wrapping_add(2)).fmul(sy).fsubr_m(py));
            let s3 = fild_u32(size.wrapping_add(3));
            let w3 = f32_of(sx_x87().fmul(s3));
            let tip = f32_of(s3.fmul(sy).fadd_m(py));
            let cp = rgba_ctor(local(&mut c), 0, 0, 0, a);
            let l = f32_of(x(px).fsub_m(w3));
            let rr = f32_of(x(w3).fadd_m(px));
            sprite2d_draw_2d_polygon(px, tip, px, tip, rr, top, l, top, cp);
            let s1 = fild_u32(size.wrapping_add(1));
            let h = sy_x87().fmul(s1);
            let top = f32_of(x(py).fsub(h));
            let w = f32_of(sx_x87().fmul(s1));
            let tip = f32_of(h.fadd_m(py));
            let cp = rgba_ctor(local(&mut c), r, gg, bb, a);
            let l = f32_of(x(px).fsub_m(w));
            let rr = f32_of(x(w).fadd_m(px));
            sprite2d_draw_2d_polygon(px, tip, px, tip, rr, top, l, top, cp);
        }
        0 => {
            let sy = sy_x87();
            let s3 = fild_u32(size.wrapping_add(3));
            let tip = f32_of(s3.fmul(sy).fsubr_m(py));
            let base = f32_of(fild_u32(size.wrapping_add(2)).fmul(sy).fadd_m(py));
            let w3 = f32_of(sx_x87().fmul(s3));
            let cp = rgba_ctor(local(&mut c), 0, 0, 0, a);
            let l = f32_of(x(px).fsub_m(w3));
            let rr = f32_of(x(w3).fadd_m(px));
            sprite2d_draw_2d_polygon(rr, base, l, base, px, tip, px, tip, cp);
            let s1 = fild_u32(size.wrapping_add(1));
            let h = sy_x87().fmul(s1);
            let tip = f32_of(x(py).fsub(h));
            let base = f32_of(h.fadd_m(py));
            let w = f32_of(sx_x87().fmul(s1));
            let cp = rgba_ctor(local(&mut c), r, gg, bb, a);
            let l = f32_of(x(px).fsub_m(w));
            let rr = f32_of(x(w).fadd_m(px));
            sprite2d_draw_2d_polygon(rr, base, l, base, px, tip, px, tip, cp);
        }
        _ => {}
    }
}

fn max_w_x87() -> X87 {
    xi(g::<i32>(RS_MAX_WIDTH).read())
}

fn max_h_x87() -> X87 {
    xi(g::<i32>(RS_MAX_HEIGHT).read())
}
/// The player position relative to the radar origin, rotated into radar space as `DrawBlips` does it
/// (dx through an f32 local, dy in a register); with `normalise` (radar, not the map) clamped to the unit
/// circle (`* 1 / len` when `len > 1`).
fn blips_player_point(p: &[f32; 3], normalise: bool) -> [f32; 2] {
    let dx = x(p[0]).fsub_m(g::<f32>(RADAR_ORIGIN_X).read());
    let dy0 = x(p[1]).fsub_m(g::<f32>(RADAR_ORIGIN_Y).read());
    let inv = x(k(K_1)).fdiv_m(g::<f32>(RADAR_RANGE).read());
    let dxf = f32_of(inv.fmul(dx));
    let dy = inv.fmul(dy0);
    let (mut rx, mut ry) = area_rotate(x(dxf), dy, Some(dxf));
    if normalise {
        let len = x(rx).fmul_m(rx).fadd(x(ry).fmul_m(ry)).fsqrt();
        if !len.fcom(x(k(K_1))).test_any(0x41) {
            let inv2 = len.fdivr_m(k(K_1));
            rx = f32_of(x(rx).fmul(inv2));
            ry = f32_of(x(ry).fmul(inv2));
        }
    }
    [rx, ry]
}

/// `CRadar::DrawBlips()` (0x588050). Radar: the north sprite (4) at `origin + (0, range * sqrt 2)`; map:
/// the cursor cross (two 2-pixel `HudColour` 6 bars through the limited cursor point) when the cursor is
/// on and (the pad byte 0xB73418 is clear and the mouse is inside the 60-pixel margin, or 0xBA6800 is
/// clear). Then two passes (isSprite 1, then 0) over priorities 1..3: entity blips (types 1-3, 7) and
/// coord / contact blips (types 4, 5; not the waypoint sprite 0x29) when `DisplayThisBlip(sprite, prio)`,
/// spotlight and airstrip blips (types 6, 8) at priority 3 unless the player is off the map on the map
/// screen; then the waypoint blips. Map: "you are here"; radar: the player arrows of players 0 and 1 (not
/// in a plane other than the Vortex) at player 0's centre (`FindPlayerCentreOfWorldForMap(out, 0)` for both,
/// as the bytes), white or CRGBA(50, 50, 80, 255) when hidden, angle `heading + pi` (top-down camera) or
/// `heading - (orientation + pi)`, size `ftol(sx * 8)`. `CPad::GetPad(0)` is called for nothing (as the
/// bytes).
#[hook(addr = 0x588050, conv = cdecl, name = "CRadar::DrawBlips", expect = "55 8B EC 83 E4 F8")]
pub fn draw_blips() {
    let mh = max_h_x87();
    let v = x(k(K_1_448)).fmul(mh).fmul_m(k(K_60));
    let y_min = ftol32(v);
    let y_max = ftol32(mh.fsub(v));
    let mw = max_w_x87();
    let u = x(k(K_1_640)).fmul(mw).fmul_m(k(K_60));
    let x_min = ftol32(u);
    let x_max = ftol32(mw.fsub(u));
    setup_airstrip_blips_call();
    for (st, val) in [(8, 0), (6, 0), (0xC, 1), (0xA, 5), (0xB, 6), (0xE, 0)] {
        rw_render_state_set(st, val);
    }
    let mut centre_in = [0f32; 2];
    let mut screen = [0f32; 2];
    transform_radar_point_to_screen_space_call(local(&mut screen), local(&mut centre_in));
    let mut radar = [0f32; 2];
    if b(MENU_DRAWING_MAP) == 0 {
        let mut north = [
            g::<f32>(RADAR_ORIGIN_X).read(),
            f32_of(x(g::<f32>(RADAR_RANGE).read()).fmul_m(k(K_SQRT2)).fadd_m(g::<f32>(RADAR_ORIGIN_Y).read())),
        ];
        transform_real_world_point_to_radar_space_call(local(&mut radar), local(&mut north));
        limit_radar_point_call(local(&mut radar));
        transform_radar_point_to_screen_space_call(local(&mut screen), local(&mut radar));
        draw_radar_sprite_call(4, screen[0], screen[1], 0xFF);
    } else if b(MENU_MAP_CURSOR_ON) != 0 {
        let pad_flag = b(PAD_MOUSE_FLAG);
        get_pad(0);
        let my = g::<i32>(MENU_MOUSE_Y).read();
        let mx = g::<i32>(MENU_MOUSE_X).read();
        let in_range = my > y_min && my < y_max && mx > x_min && mx < x_max;
        if (pad_flag == 0 && in_range) || b(MENU_CURSOR_FLAG) == 0 {
            let mut cursor = [g::<f32>(MENU_MAP_CURSOR_X).read(), g::<f32>(MENU_MAP_CURSOR_Y).read()];
            transform_real_world_point_to_radar_space_call(local(&mut radar), local(&mut cursor));
            limit_radar_point_call(local(&mut radar));
            transform_radar_point_to_screen_space_call(local(&mut screen), local(&mut radar));
            limit_to_map_call(GPtr::from_raw(&mut screen[0] as *mut f32), GPtr::from_raw(&mut screen[1] as *mut f32));
            let sxpx = max_w_x87().fmul_m(k(K_1_640)).fmul_m(screen[0]);
            let mut rect = [f32_of(x(k(K_1)).fsubr(sxpx)), f32_of(max_h_x87()), f32_of(sxpx.fadd_m(k(K_1))), 0.0];
            let mut col = [0u8; 4];
            let cp = hud_colours_get_rgb(g(HUD_COLOUR), local(&mut col), 6);
            sprite2d_draw_rect_static(local(&mut rect), cp);
            let sypy = max_h_x87().fmul_m(k(K_1_448)).fmul_m(screen[1]);
            let mut rect = [0.0, f32_of(sypy.fadd_m(k(K_1))), f32_of(max_w_x87()), f32_of(x(k(K_1)).fsubr(sypy))];
            let cp = hud_colours_get_rgb(g(HUD_COLOUR), local(&mut col), 6);
            sprite2d_draw_rect_static(local(&mut rect), cp);
        }
    }
    let mut flag: u8 = 1;
    loop {
        for prio in 1u8..=3 {
            for i in 0..175i32 {
                let t = trace(i);
                if rd::<u8>(t, 0x25) & 2 == 0 {
                    continue;
                }
                let sprite = rd::<i8>(t, 0x24);
                match (rd::<u8>(t, 0x26) >> 2) & 0xF {
                    1..=3 | 7 => {
                        if display_this_blip_call(i32::from(sprite), prio) != 0 {
                            draw_entity_blip_call(i, u32::from(flag));
                        }
                    }
                    4 | 5 => {
                        if sprite as u8 != 0x29 && display_this_blip_call(i32::from(sprite), prio) != 0 {
                            draw_coord_blip_call(i, flag);
                        }
                    }
                    6 | 8 if prio == 3 && (b(SCRIPTS_PLAYER_OFF_THE_MAP) == 0 || b(MENU_DRAWING_MAP) == 0) => {
                        draw_entity_blip_call(i, u32::from(flag));
                    }
                    _ => {}
                }
            }
        }
        for i in 0..175i32 {
            let t = trace(i);
            if rd::<u8>(t, 0x25) & 2 == 0 {
                continue;
            }
            let ty = (rd::<u8>(t, 0x26) >> 2) & 0xF;
            if !(4..=5).contains(&ty) || rd::<u8>(t, 0x24) != 0x29 {
                continue;
            }
            if g::<i32>(CURR_AREA).read() == 0 {
                let _ = rd::<u8>(find_player_ped(-1), 0x2F);
            }
            draw_coord_blip_call(i, flag);
        }
        flag = u8::from(flag == 0);
        if flag != 0 {
            break;
        }
    }
    if b(MENU_DRAWING_MAP) != 0 {
        let mut c = [0f32; 3];
        find_player_centre_of_world_for_map(local(&mut c), 0);
        let mut r = blips_player_point(&c, false);
        transform_radar_point_to_screen_space_call(local(&mut screen), local(&mut r));
        draw_you_are_here_sprite_call(screen[0], screen[1]);
        if b(MENU_DRAWING_MAP) != 0 {
            return;
        }
    }
    for i in 0..2i32 {
        if find_player_ped(i).is_null() {
            continue;
        }
        if !find_player_vehicle(i, 0).is_null()
            && rd::<u32>(find_player_vehicle(i, 0), 0x594) == 4
            && rd::<u16>(find_player_vehicle(i, 0), 0x22) != 0x21B
        {
            continue;
        }
        let mut c = [0f32; 3];
        find_player_centre_of_world_for_map(local(&mut c), 0);
        let mut r = blips_player_point(&c, b(MENU_DRAWING_MAP) == 0);
        transform_radar_point_to_screen_space_call(local(&mut screen), local(&mut r));
        let heading = f32_of(find_player_heading(i));
        let ped = g::<GPtr<u8>>(PLAYERS + i as usize * 0x190).read();
        let mut col = [0u8; 4];
        let cp = if player_ped_is_hidden(ped) != 0 {
            rgba_ctor(local(&mut col), 0x32, 0x32, 0x50, 0xFF)
        } else {
            rgba_ctor(local(&mut col), 0xFF, 0xFF, 0xFF, 0xFF)
        };
        let colour = rd::<u32>(cp, 0);
        let cam = usize::from(b(CAMERA_ACTIVE_CAM)) * 0x238;
        let size = ftol32(max_w_x87().fmul_m(k(K_1_640)).fmul_m(k(K_8))) as u32;
        let angle = if g::<u16>(CAM_MODE + cam).read() == 1 {
            f32_of(x(heading).fadd_m(k(K_PI)))
        } else {
            f32_of(x(g::<f32>(RADAR_ORIENTATION).read()).fadd_m(k(K_PI)).fsubr_m(heading))
        };
        draw_rotating_radar_sprite_call(g(RADAR_BLIP_SPRITES + 8), screen[0], screen[1], angle, size, size, colour);
    }
}

/// One end of a `ShowRadarMarker` line: `pos +/- v * r`; x from the register product, y and z through
/// f32 temporaries, in the operand order of the four line blocks (`first`: `fld pos; op st, st(1)` /
/// `fld pos; op [t]`, else `fld t / product; fadd [pos]`).
fn marker_end(pos: [f32; 3], v: usize, r: X87, plus: bool, first: bool) -> [f32; 3] {
    let vx = x(g::<f32>(v).read()).fmul(r);
    let t1 = f32_of(r.fmul_m(g::<f32>(v + 4).read()));
    let t2 = f32_of(r.fmul_m(g::<f32>(v + 8).read()));
    if !plus {
        return [f32_of(x(pos[0]).fsub(vx)), f32_of(x(pos[1]).fsub_m(t1)), f32_of(x(pos[2]).fsub_m(t2))];
    }
    if first {
        [f32_of(x(pos[0]).fadd(vx)), f32_of(x(pos[1]).fadd_m(t1)), f32_of(x(pos[2]).fadd_m(t2))]
    } else {
        [f32_of(vx.fadd_m(pos[0])), f32_of(x(t1).fadd_m(pos[1])), f32_of(x(t2).fadd_m(pos[2]))]
    }
}

/// `CRadar::ShowRadarMarker(CVector pos, uint32 colour, float radius)` (0x584480): four debug lines
/// (`CTheScripts::ScriptDebugLine3D`) from `pos + A * 1.4r` to `pos + A * 0.5r`, `pos - A * 1.4r` to
/// `pos - A * 0.5r`, and the same along B (two camera matrix rows, 0xB6F9BC and 0xB6F99C). The first end's
/// x of line 1 uses the unrounded `1.4r` register, the others the f32 copy (same value). The `radius`
/// argument slot is overwritten with `0.5r` (as the bytes).
#[hook(addr = 0x584480, conv = cdecl, name = "CRadar::ShowRadarMarker", expect = "83 EC 28 D9 44 24 3C")]
pub fn show_radar_marker(px: u32, py: u32, pz: u32, colour: u32, radius: u32) {
    let pos = [f32::from_bits(px), f32::from_bits(py), f32::from_bits(pz)];
    let r1 = x(f32::from_bits(radius)).fmul_m(k(K_1_4));
    let r1f = x(f32_of(r1));
    let r2 = x(f32_of(x(f32::from_bits(radius)).fmul_m(k(K_HALF))));
    let a = marker_end(pos, CAM_VEC_A, r1, true, true);
    let bb = marker_end(pos, CAM_VEC_A, r2, true, false);
    script_debug_line_3d(a[0], a[1], a[2], bb[0], bb[1], bb[2], colour, colour);
    let a = marker_end(pos, CAM_VEC_A, r1f, false, false);
    let bb = marker_end(pos, CAM_VEC_A, r2, false, false);
    script_debug_line_3d(a[0], a[1], a[2], bb[0], bb[1], bb[2], colour, colour);
    let a = marker_end(pos, CAM_VEC_B, r1f, true, false);
    let bb = marker_end(pos, CAM_VEC_B, r2, true, false);
    script_debug_line_3d(a[0], a[1], a[2], bb[0], bb[1], bb[2], colour, colour);
    let a = marker_end(pos, CAM_VEC_B, r1f, false, false);
    let bb = marker_end(pos, CAM_VEC_B, r2, false, false);
    script_debug_line_3d(a[0], a[1], a[2], bb[0], bb[1], bb[2], colour, colour);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_DRAW_RADAR_SPRITE, HOOK_DRAW_COORD_BLIP, HOOK_DRAW_MAP, HOOK_DRAW_RADAR_MAP, HOOK_DRAW_RADAR_SECTION_MAP, HOOK_DRAW_RADAR_MASK, HOOK_DRAW_RADAR_GANG_OVERLAY, HOOK_DRAW_AREA_ON_RADAR, HOOK_DRAW_RADAR_SECTION, HOOK_DRAW_3D_MARKERS, HOOK_SHOW_RADAR_TRACE_WITH_HEIGHT, HOOK_DRAW_BLIPS, HOOK_SHOW_RADAR_MARKER];

/// Phase B target of `limit_radar_point_call_st0`: the port's ST0 result as f64.
#[allow(dead_code)]
fn limit_radar_point_call_target_f64(p: GPtr<f32>) -> f64 {
    crate::radar::limit_radar_point(p).to_f64()
}

/// `limit_radar_point_call_st0` with its ST0 result as `X87` (orig! returns stay register-sized).
#[allow(dead_code)]
fn limit_radar_point_call(p: GPtr<f32>) -> X87 {
    X87::from_f64_exact(limit_radar_point_call_st0(p))
}

/// `find_player_heading_st0` with its ST0 result as `X87` (orig! returns stay register-sized).
#[allow(dead_code)]
fn find_player_heading(id: i32) -> X87 {
    X87::from_f64_exact(find_player_heading_st0(id))
}
