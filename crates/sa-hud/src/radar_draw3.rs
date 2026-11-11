//! `CRadar`: `DrawEntityBlip`, `DrawLegend` (PORT-HUD-3). Ported from the 1.0 US bytes; gta-reversed as reading
//! aid (the bytes win; gta-reversed has no body for `DrawEntityBlip`, only a `plugin::Call` stub).
//!
//! Only the orchestration is Rust: conditions, coordinates, x87 arithmetic in the original order, global reads
//! and writes. Every RenderWare / `CSprite2d` / `CFont` / transform / pool call stays an `orig!` call
//! (`TransformRadarPointToScreenSpace` preserves EDX for original callers, so it is always called through
//! `orig!`).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, ftol32, g, rd, wr, x, xi};

sa_core::globals! {
    list GLOBALS_DECL;
    /// `DrawLegend`'s trace-height cycle for the arrow legends (gr `legendTraceHeight`, byte; 0 low, 1 high,
    /// 2 normal; .bss, no init guard in the bytes)
    pub static LEGEND_TRACE_HEIGHT @ 0xBAA350: u8 = 0;
    /// `DrawLegend`'s timer for that cycle (gr `legendTraceTimer`; pause-mode milliseconds)
    pub static LEGEND_TRACE_TIMER @ 0xBAA354: u32 = 0;
    /// `DrawEntityBlip`'s airstrip arrow: "arrow reached the strip" byte
    pub static AIRSTRIP_DONE @ 0xBAA370: u8 = 0;
    /// `DrawEntityBlip`'s airstrip arrow: time of the last step (pause-mode milliseconds)
    pub static AIRSTRIP_TIME @ 0xBAA374: u32 = 0;
    /// `DrawEntityBlip`'s airstrip arrow: the offset where the arrow last reached the radar edge
    pub static AIRSTRIP_A @ 0xBAA378: i16 = 0;
    /// `DrawEntityBlip`'s airstrip arrow: the current offset along the strip
    pub static AIRSTRIP_B @ 0xBAA37C: i16 = 0;
    /// `DrawEntityBlip`'s airstrip arrow: init guard bits (1: `AIRSTRIP_B`, 2: `AIRSTRIP_A`)
    pub static AIRSTRIP_INIT @ 0xBAA380: u32 = 0;
}

/// This module's globals.
pub const GLOBALS: &[GlobalDesc] = GLOBALS_DECL;

// Foreign globals, addressed directly.
/// `FrontEndMenuManager.m_bDrawingMap` (+0x59)
const MENU_DRAWING_MAP: usize = 0xBA67A1;
/// `FrontEndMenuManager` +0x47 (gr `m_ShowMissionBlips`)
const MENU_SHOW_MISSION_BLIPS: usize = 0xBA678F;
/// `RsGlobal.maximumWidth` / `maximumHeight`
const RS_MAX_WIDTH: usize = 0xC17044;
const RS_MAX_HEIGHT: usize = 0xC17048;
/// `CRadar::ms_RadarTrace[175]` (0x28 each)
const RADAR_TRACE: usize = 0xBA86F0;
/// `CRadar::RadarBlipSprites[64]` (`CSprite2d`, 4 bytes each); [8] (0xBAA258) is the entity arrow,
/// [0x38] (0xBAA330) and [0x39] (0xBAA334) the airstrip sprites
const RADAR_BLIP_SPRITES: u32 = 0xBAA250;
const SPRITE_ARROW: usize = 0xBAA258;
const SPRITE_AIRSTRIP_0: usize = 0xBAA330;
const SPRITE_AIRSTRIP_1: usize = 0xBAA334;
/// `CRadar::ArrowBlipColour[6]` (`CRGBA`), indexed by `-sprite`
const ARROW_BLIP_COLOUR: u32 = 0xBA86D4;
/// `CRadar::m_radarRange`, `m_fRadarOrientation`
const RADAR_RANGE: usize = 0xBA8314;
const RADAR_ORIENTATION: usize = 0xBA8310;
/// `airstrip_location` (byte) and `airstrip_table[]` (0x10 each: x, y, heading in degrees, +0xC length)
const AIRSTRIP_LOCATION: usize = 0xBA8300;
const AIRSTRIP_TABLE: usize = 0x8D06E0;
/// `CTimer::m_snTimeInMillisecondsPauseMode`
const TIME_MS_PAUSE: usize = 0xB7CB7C;
/// `CTheScripts::RadarZoomValue`, `RadarShowBlipOnAllLevels`, `DbgFlag`
const SCRIPTS_RADAR_ZOOM_VALUE: usize = 0xA444A3;
const SCRIPTS_RADAR_SHOW_ALL_LEVELS: usize = 0xA444A2;
const SCRIPTS_DBG_FLAG: usize = 0x859CF8;
/// `TheCamera.m_nActiveCam` (byte) and `TheCamera.m_aCams[i].m_nMode` (u16, stride 0x238)
const CAMERA_ACTIVE_CAM: usize = 0xB6F081;
const CAM_MODE: usize = 0xB6F1A8;
/// `CPools::ms_pVehiclePool`, `ms_pPedPool`, `ms_pObjectPool`
const VEHICLE_POOL: usize = 0xB74494;
const PED_POOL: usize = 0xB74490;
const OBJECT_POOL: usize = 0xB7449C;
/// `CTheScripts::ScriptSearchLightArray` (0x7C each)
const SEARCH_LIGHTS: u32 = 0xA94D68;
/// `CPickups::aPickUps` (0x20 each)
const PICKUPS: u32 = 0x9788C0;
/// `TheText` (`CText`)
const THE_TEXT: usize = 0xC1B340;

// .rdata float constants (bits)
/// `1/640` (0x859520), `1/448` (0x859524)
const K_1_640: u32 = 0x3ACC_CCCD;
const K_1_448: u32 = 0x3B12_4925;
/// `8.0` (0x859000), `16.0` (0x8599D0)
const K_8: u32 = 0x4100_0000;
const K_16: u32 = 0x4180_0000;
/// `1.0` (0x858624), `2.0` (0x858CA0), `3.0` (0x858B3C), `4.0` (0x858B90), `5.0` (0x858C80), `6.0` (0x858B44),
/// `7.0` (0x858F48), `20.0` (0x858BA4)
const K_1: u32 = 0x3F80_0000;
const K_2: u32 = 0x4000_0000;
const K_3: u32 = 0x4040_0000;
const K_4: u32 = 0x4080_0000;
const K_5: u32 = 0x40A0_0000;
const K_6: u32 = 0x40C0_0000;
const K_7: u32 = 0x40E0_0000;
const K_20: u32 = 0x41A0_0000;
/// `0.1` (0x858B1C), `0.6` (0x858CC8), `0.9` (0x858C20)
const K_0_1: u32 = 0x3DCC_CCCD;
const K_0_6: u32 = 0x3F19_999A;
const K_0_9: u32 = 0x3F66_6666;
/// `94.0` (0x866B78), `90.0` (0x85991C), `500.0` (0x858B58)
const K_94: u32 = 0x42BC_0000;
const K_90: u32 = 0x42B4_0000;
const K_500: u32 = 0x43FA_0000;
/// `pi` (0x858CB8), `pi/180` (0x8595EC)
const K_PI: u32 = 0x4049_0FDB;
const K_DEG2RAD: u32 = 0x3C8E_FA35;
/// `5.0` stored by `mov dword` (radius reset)
const FIVE_BITS: u32 = 0x40A0_0000;

fn k(bits: u32) -> f32 {
    f32::from_bits(bits)
}

fn b(addr: usize) -> u8 {
    g::<u8>(addr).read()
}

fn max_w() -> X87 {
    xi(g::<i32>(RS_MAX_WIDTH).read())
}

fn max_h() -> X87 {
    xi(g::<i32>(RS_MAX_HEIGHT).read())
}

/// A float from its dword (the ports keep copied positions as dwords so no value passes through an x87
/// load/store that the original only `mov`s).
fn fb(v: u32) -> f32 {
    f32::from_bits(v)
}

/// A local buffer as a game pointer (the original's stack temporaries).
fn local<T, const N: usize>(buf: &mut [T; N]) -> GPtr<T> {
    GPtr::from_raw(buf.as_mut_ptr())
}

/// Entry `i` of `ms_RadarTrace` (`i * 0x28`, wrapping).
fn trace(i: i32) -> GPtr<u8> {
    g::<u8>(RADAR_TRACE).byte_add(i.wrapping_mul(0x28) as isize)
}

/// `ftol(max * scale * c)`: the sprite half sizes (`W8`/`H8`/`W16`/`H16`).
fn size(max: X87, scale: u32, c: u32) -> u32 {
    ftol32(max.fmul_m(k(scale)).fmul_m(k(c))) as u32
}

/// `sqrt(r.y * r.y + r.x * r.x)` in the original's order (y term first).
fn radar_len(r: &[u32; 2]) -> X87 {
    let yy = x(fb(r[1])).fmul_m(fb(r[1]));
    let xx = x(fb(r[0])).fmul_m(fb(r[0]));
    yy.fadd(xx).fsqrt()
}

orig! {
    /// `CText::Get(const char*)` (0x6A0050)
    fn text_get(this: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => crate::text::text_get;
    /// `CFont::PrintString(float x, float y, const GxtChar*)` (0x71A700)
    fn font_print_string(px: f32, py: f32, text: GPtr<u8>) = cdecl 0x71A700 => unported;
    /// `CRGBA::CRGBA(r, g, b, a)` (0x7170C0)
    fn rgba_ctor(this: GPtr<u8>, r: u32, gg: u32, bb: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `CSprite2d::Draw(const CRect&, const CRGBA&)` (0x728350)
    fn sprite2d_draw_rect(this: GPtr<u8>, rect: GPtr<f32>, colour: GPtr<u8>) = thiscall 0x728350 => unported;
    /// `CSprite2d::DrawRect(const CRect&, const CRGBA&)` (0x727B60)
    fn sprite2d_draw_rect_static(rect: GPtr<f32>, colour: GPtr<u8>) = cdecl 0x727B60 => unported;
    /// `CSprite2d::Draw2DPolygon(x1, y1, x2, y2, x3, y3, x4, y4, const CRGBA&)` (0x7285B0)
    fn sprite2d_draw_2d_polygon(x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, x4: f32, y4: f32, colour: GPtr<u8>) = cdecl 0x7285B0 => unported;
    /// `CSprite2d::DrawCircleAtNearClip(const CVector2D&, float radius, const CRGBA&, int32 numVerts)` (0x727D60)
    fn sprite2d_draw_circle_at_near_clip(pos: GPtr<u32>, radius: f32, colour: GPtr<u8>, n: i32) = cdecl 0x727D60 => unported;

    /// `CVehiclePool::GetAt(int32 handle)` (0x4048E0)
    fn vehicle_pool_get_at(pool: GPtr<u8>, h: i32) -> GPtr<u8> = thiscall 0x4048E0 => unported;
    /// `CPedPool::GetAtRef(int32 handle)` (0x404910)
    fn ped_pool_get_at_ref(pool: GPtr<u8>, h: i32) -> GPtr<u8> = thiscall 0x404910 => unported;
    /// `CObjectPool::GetAt(int32 handle)` (0x465040)
    fn object_pool_get_at(pool: GPtr<u8>, h: i32) -> GPtr<u8> = thiscall 0x465040 => unported;
    /// `CTheScripts::GetActualScriptThingIndex(int32 ref, uint8 type)` (0x4839A0)
    fn get_actual_script_thing_index(r: i32, ty: u32) -> i32 = cdecl 0x4839A0 => unported;
    /// `CPickups::GetActualPickupIndex(int32)` (0x4552A0)
    fn get_actual_pickup_index(h: i32) -> i32 = cdecl 0x4552A0 => sa_world::world::pickups::get_actual_pickup_index;
    /// `CPickup::GetPosn(CVector* out)` (0x4549A0, thiscall, returns `out`)
    fn pickup_get_posn(this: GPtr<u8>, out: GPtr<u32>) -> GPtr<u32> = thiscall 0x4549A0 => unported;
    /// `CEntryExit::GetPositionRelativeToOutsideWorld(CVector&)` (0x43EA00, thiscall)
    fn ee_get_position_relative_to_outside_world(this: GPtr<u8>, pos: GPtr<f32>) = thiscall 0x43EA00 => sa_world::world::entryexit::get_position_relative_to_outside_world;
    /// `CPlaceable::GetHeading()` (0x441DB0, ST0)
    fn placeable_get_heading_st0(e: GPtr<u8>) -> f64 = thiscall 0x441DB0 => unported;
    /// `FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, remote: u8) -> GPtr<u8> = cdecl 0x56E0D0 => sa_world::world::cworld::find_player_vehicle_port;
    /// `FindPlayerCentreOfWorldForMap(CVector* out, int32 id)` (0x56E400)
    fn find_player_centre_of_world_for_map(out: GPtr<f32>, id: i32) -> GPtr<f32> = cdecl 0x56E400 => sa_world::world::cworld::find_player_centre_of_world_for_map;

    /// `CRadar::GetRadarTraceColour(eBlipColour, bool bright, bool friendly)` (0x584770)
    fn get_radar_trace_colour_call(colour: u32, bright: u32, friendly: u32) -> u32 = cdecl 0x584770 => crate::radar_draw::get_radar_trace_colour;
    /// `CRadar::ShowRadarMarker(CVector pos, uint32 colour, float radius)` (0x584480)
    fn show_radar_marker_call(px: u32, py: u32, pz: u32, colour: u32, radius: u32) = cdecl 0x584480 => crate::radar_draw2::show_radar_marker;
    /// `CRadar::TransformRealWorldPointToRadarSpace(CVector2D& out, const CVector2D& in)` (0x583530)
    fn transform_real_world_point_to_radar_space_call(out: GPtr<f32>, inp: GPtr<f32>) = cdecl 0x583530 => crate::radar::transform_real_world_point_to_radar_space;
    /// `CRadar::TransformRadarPointToScreenSpace(CVector2D& out, const CVector2D& in)` (0x583480; preserves EDX)
    fn transform_radar_point_to_screen_space_call(out: GPtr<f32>, inp: GPtr<f32>) = cdecl 0x583480 => crate::radar::transform_radar_point_to_screen_space;
    /// `CRadar::LimitRadarPoint(CVector2D&)` (0x5832F0, ST0)
    fn limit_radar_point_call_st0(p: GPtr<f32>) -> f64 = cdecl 0x5832F0 => limit_radar_point_st0_rust;
    /// `CRadar::HasThisBlipBeenRevealed(int32)` (0x583AF0)
    fn has_this_blip_been_revealed_call(i: i32) -> u8 = cdecl 0x583AF0 => crate::radar::has_this_blip_been_revealed;
    /// `CRadar::DrawRadarSprite(uint16 sprite, float x, float y, uint8 alpha)` (0x585FF0)
    fn draw_radar_sprite_call(sprite: u32, px: f32, py: f32, alpha: u32) = cdecl 0x585FF0 => crate::radar_draw2::draw_radar_sprite;
    /// `CRadar::DrawRotatingRadarSprite(CSprite2d*, x, y, angle, width, height, CRGBA)` (0x584850)
    fn draw_rotating_radar_sprite_call(sprite: GPtr<u8>, px: f32, py: f32, angle: f32, width: u32, height: u32, colour: u32) = cdecl 0x584850 => crate::radar_draw::draw_rotating_radar_sprite;
    /// `CRadar::ShowRadarTraceWithHeight(float x, float y, uint32 size, uint8 r, uint8 g, uint8 b, uint8 a,
    /// eRadarTraceHeight)` (0x584070)
    fn show_radar_trace_with_height_call(px: f32, py: f32, size: u32, r: u32, gg: u32, bb: u32, a: u32, mode: u32) = cdecl 0x584070 => crate::radar_draw2::show_radar_trace_with_height;
    /// `CRadar::AddBlipToLegendList(bool noSprite, int32 index)` (0x5859F0)
    fn add_blip_to_legend_list_call(no_sprite: u32, index: i32) = cdecl 0x5859F0 => crate::radar_draw::add_blip_to_legend_list;
}

/// The Rust path of the ST0 declaration above (phases B/C).
#[allow(dead_code)]
fn limit_radar_point_st0_rust(p: GPtr<f32>) -> f64 {
    crate::radar::limit_radar_point(p).to_f64()
}

/// [`limit_radar_point_call_st0`] as an x87 value.
fn limit_radar_point(p: GPtr<f32>) -> X87 {
    X87::from_f64_exact(limit_radar_point_call_st0(p))
}

/// The GXT key `DrawLegend` looks up for sprite `-5..=63` (jump table 0x5831D8, index `sprite + 5`); 0 = the
/// default case, which passes no text but the `y` argument as the string pointer.
const LEGEND_KEYS: [u32; 69] = [
    0x866970, // -5 LG_56
    0x866978, // -4 LG_55
    0x866980, // -3 LG_54
    0x866988, // -2 LG_50
    0x866990, // -1 LG_49
    0,        // 0
    0,        // 1
    0,        // 2
    0x866B68, // 3 LG_01
    0,        // 4
    0x866B60, // 5 LG_02
    0x866B58, // 6 LG_03
    0x866B50, // 7 LG_04
    0x866B48, // 8 LG_05
    0x866B40, // 9 LG_06
    0x866B38, // 10 LG_07
    0x8669A8, // 11 LG_66
    0x866B30, // 12 LG_09
    0x866B28, // 13 LG_10
    0x866B20, // 14 LG_11
    0x866B18, // 15 LG_12
    0x866B10, // 16 LG_13
    0x8669A0, // 17 LG_67
    0x866B08, // 18 LG_15
    0x866B00, // 19 LG_16
    0x866AF8, // 20 LG_17
    0x866AF0, // 21 LG_18
    0x866AE8, // 22 LG_19
    0x866AE0, // 23 LG_20
    0x866AD8, // 24 LG_21
    0x866AD0, // 25 LG_22
    0x866AC8, // 26 LG_23
    0x866AC0, // 27 LG_24
    0x866AB8, // 28 LG_25
    0x866AB0, // 29 LG_26
    0x866AA8, // 30 LG_27
    0x866AA0, // 31 LG_28
    0x866A98, // 32 LG_29
    0x866A90, // 33 LG_30
    0x866A88, // 34 LG_31
    0x866A80, // 35 LG_32
    0x866A78, // 36 LG_33
    0x8669B8, // 37 LG_63
    0x866A68, // 38 LG_35
    0x866A60, // 39 LG_36
    0x866A58, // 40 LG_37
    0x8669B0, // 41 LG_64
    0x866A50, // 42 LG_39
    0x866A48, // 43 LG_40
    0x866A40, // 44 LG_41
    0x866A38, // 45 LG_42
    0x866A30, // 46 LG_43
    0x866A28, // 47 LG_44
    0x866A20, // 48 LG_45
    0x866A18, // 49 LG_46
    0x866A10, // 50 LG_47
    0x866A08, // 51 LG_48
    0x866A00, // 52 LG_51
    0x8669F8, // 53 LG_52
    0x8669F0, // 54 LG_53
    0x8669E8, // 55 LG_57
    0,        // 56
    0x866998, // 57 LG_65
    0x8669E0, // 58 LG_58
    0x8669D8, // 59 LG_59
    0x8669D0, // 60 LG_60
    0x8669C8, // 61 LG_61
    0x8669C0, // 62 LG_62
    0x866A70, // 63 LG_34
];

/// The `CRect` {left, bottom, right, top} the original builds on its stack.
fn rect(l: f32, bt: f32, r: f32, t: f32) -> [f32; 4] {
    [l, bt, r, t]
}

/// `CRadar::DrawLegend(int32 x, int32 y, eRadarSprite sprite)` (0x5828A0): the legend line of the front-end
/// map. Prints `TheText.Get(LG_xx)` at `(x + sx * 20, y + sy * 3)`; a sprite `>= 0` is drawn white over
/// {x, y + sx * 16, x + sx * 16, y} (both offsets use the *x* scale); a negative sprite is a trace arrow in
/// `ArrowBlipColour[-sprite]` whose height state cycles low -> high -> normal every 600 ms (pause-mode
/// clock), centred at `(ftol(sx * 8 + x), ftol(sy * 8 + y))`.
///
/// Deviations from gta-reversed (the bytes win): for sprites with no jump-table key (0, 1, 2, 4, 56 and
/// anything outside -5..63) the original passes the `y` argument as the text pointer to `PrintString` (the
/// port does the same); the centre is truncated with `_ftol`, not rounded; state 0 (low) draws the outline
/// with its base at `+ 6 * sy` and its tip at `- 7 * sy` (`(cx + 7sx, cy + 6sy), (cx - 7sx, cy + 6sy),
/// (cx, cy - 7sy) x2`) and state 1 (high) draws `(cx, cy + 7sy) x2, (cx + 7sx, cy - 6sy), (cx - 7sx,
/// cy - 6sy)`; the inner polygons use 5 as gr does (vertex order as the bytes). State 2 draws two rects.
#[hook(addr = 0x5828A0, conv = cdecl, name = "CRadar::DrawLegend", expect = "83 EC 38 56 8B 74 24 48")]
pub fn draw_legend(px: i32, py: i32, sprite: i32) {
    let idx = sprite.wrapping_add(5) as u32;
    let key = if idx <= 0x44 { LEGEND_KEYS[idx as usize] } else { 0 };
    let text = if key != 0 { text_get(g(THE_TEXT), g(key as usize)) } else { GPtr::from_addr(py as u32 as usize) };
    let yf = f32_of(xi(py));
    let xf = f32_of(xi(px));
    let ty = f32_of(max_h().fmul_m(k(K_1_448)).fmul_m(k(K_3)).fadd_m(yf));
    let tx = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_20)).fadd_m(xf));
    font_print_string(tx, ty, text);
    if sprite >= 0 {
        let s = max_w().fmul_m(k(K_1_640)).fmul_m(k(K_16));
        let right = f32_of(x(xf).fadd(s));
        let bottom = f32_of(s.fadd_m(yf));
        let mut r = rect(xf, bottom, right, yf);
        let mut c = [0u8; 4];
        let cp = rgba_ctor(local(&mut c), 0xFF, 0xFF, 0xFF, 0xFF);
        let spr = RADAR_BLIP_SPRITES.wrapping_add((sprite as u32).wrapping_mul(4));
        sprite2d_draw_rect(g(spr as usize), local(&mut r), cp);
        return;
    }
    let mut arrow = [g::<u32>(ARROW_BLIP_COLOUR.wrapping_sub((sprite as u32).wrapping_mul(4)) as usize).read()];
    let now = g::<u32>(TIME_MS_PAUSE).read();
    if now.wrapping_sub(LEGEND_TRACE_TIMER.get()) > 0x258 {
        LEGEND_TRACE_TIMER.set(now);
        let h = LEGEND_TRACE_HEIGHT.get();
        LEGEND_TRACE_HEIGHT.set(if h == 2 { 0 } else { h.wrapping_add(1) });
    }
    let sx = max_w().fmul_m(k(K_1_640));
    let sx_f = f32_of(sx);
    let cxi = ftol32(sx.fmul_m(k(K_8)).fadd_m(xf));
    let sy = max_h().fmul_m(k(K_1_448));
    let sy_f = f32_of(sy);
    let cyi = ftol32(sy.fmul_m(k(K_8)).fadd_m(yf));
    let arrow_p: GPtr<u8> = local(&mut arrow).cast();
    match LEGEND_TRACE_HEIGHT.get() {
        2 => {
            let cx = f32_of(xi(cxi));
            let sx5 = x(sx_f).fmul_m(k(K_5));
            let l = f32_of(x(cx).fsub(sx5));
            let cy = f32_of(xi(cyi));
            let sy5 = x(sy_f).fmul_m(k(K_5));
            let t = f32_of(x(cy).fsub(sy5));
            let r = f32_of(sx5.fadd_m(cx));
            let bt = f32_of(sy5.fadd_m(cy));
            let mut c = [0u8; 4];
            let cp = rgba_ctor(local(&mut c), 0, 0, 0, 0xFF);
            let mut r1 = rect(l, bt, r, t);
            sprite2d_draw_rect_static(local(&mut r1), cp);
            let sx4 = max_w().fmul_m(k(K_1_640)).fmul_m(k(K_4));
            let l = f32_of(x(cx).fsub(sx4));
            let sy4 = max_h().fmul_m(k(K_1_448)).fmul_m(k(K_4));
            let t = f32_of(x(cy).fsub(sy4));
            let r = f32_of(sx4.fadd_m(cx));
            let bt = f32_of(sy4.fadd_m(cy));
            let mut r2 = rect(l, bt, r, t);
            sprite2d_draw_rect_static(local(&mut r2), arrow_p);
        }
        1 => {
            let cy = f32_of(xi(cyi));
            let sy = max_h().fmul_m(k(K_1_448));
            let top = f32_of(x(k(K_6)).fmul(sy).fsubr_m(cy));
            let cx = f32_of(xi(cxi));
            let sxk = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_7)));
            let bottom = f32_of(sy.fmul_m(k(K_7)).fadd_m(cy));
            let mut c = [0u8; 4];
            let cp = rgba_ctor(local(&mut c), 0, 0, 0, 0xFF);
            let x4 = f32_of(x(cx).fsub_m(sxk));
            let x3 = f32_of(x(sxk).fadd_m(cx));
            sprite2d_draw_2d_polygon(cx, bottom, cx, bottom, x3, top, x4, top, cp);
            let sy5 = max_h().fmul_m(k(K_1_448)).fmul_m(k(K_5));
            let t2 = f32_of(x(cy).fsub(sy5));
            let sx5 = max_w().fmul_m(k(K_1_640)).fmul_m(k(K_5));
            let b2 = f32_of(sy5.fadd_m(cy));
            let x4 = f32_of(x(cx).fsub(sx5));
            let x3 = f32_of(sx5.fadd_m(cx));
            sprite2d_draw_2d_polygon(cx, b2, cx, b2, x3, t2, x4, t2, arrow_p);
        }
        0 => {
            let cy = f32_of(xi(cyi));
            let sy = max_h().fmul_m(k(K_1_448));
            let tip = f32_of(x(k(K_7)).fmul(sy).fsubr_m(cy));
            let cx = f32_of(xi(cxi));
            let base = f32_of(sy.fmul_m(k(K_6)).fadd_m(cy));
            let sxk = f32_of(max_w().fmul_m(k(K_1_640)).fmul_m(k(K_7)));
            let mut c = [0u8; 4];
            let cp = rgba_ctor(local(&mut c), 0, 0, 0, 0xFF);
            let x2 = f32_of(x(cx).fsub_m(sxk));
            let x1 = f32_of(x(sxk).fadd_m(cx));
            sprite2d_draw_2d_polygon(x1, base, x2, base, cx, tip, cx, tip, cp);
            let sy5 = max_h().fmul_m(k(K_1_448)).fmul_m(k(K_5));
            let t = f32_of(x(cy).fsub(sy5));
            let bt = f32_of(sy5.fadd_m(cy));
            let sx5 = max_w().fmul_m(k(K_1_640)).fmul_m(k(K_5));
            let x2 = f32_of(x(cx).fsub(sx5));
            let x1 = f32_of(sx5.fadd_m(cx));
            sprite2d_draw_2d_polygon(x1, bt, x2, bt, cx, t, cx, t, arrow_p);
        }
        _ => {}
    }
}

/// The entity position the original copies: `m_matrix->pos` (+0x30) when the matrix exists, else the
/// placement position (+4).
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// Three dwords at `p` (a `CVector` copied with `mov`).
fn vec3(p: GPtr<u8>) -> [u32; 3] {
    [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)]
}

/// `(i16)ftol(-airstrip.+0xC)`: the arrow's start offset.
fn airstrip_start(a: GPtr<u8>) -> i16 {
    ftol32(x(rd::<f32>(a, 0xC)).fchs()) as i16
}

/// The colour argument words `ShowRadarTraceWithHeight` gets: `c >> 24`, `c >> 16`, `c >> 8`, `c`
/// (unmasked dwords, as the bytes push them).
fn trace_with_height(s: &[u32; 2], t: GPtr<u8>, c: u32, mode: u32) {
    let size = i32::from(rd::<i16>(t, 0x1C)) as u32;
    show_radar_trace_with_height_call(fb(s[0]), fb(s[1]), size, c >> 24, c >> 16, c >> 8, c, mode);
}

/// `CRGBA(r, g, b, a)` on a stack temporary, returned as the dword the original passes by value.
fn rgba(r: u32, gg: u32, bb: u32, a: u32) -> u32 {
    let mut c = [0u8; 4];
    let p = rgba_ctor(local(&mut c), r, gg, bb, a);
    rd::<u32>(p, 0)
}

/// `CRadar::DrawEntityBlip(int32 index, uint8 isSprite)` (0x587000): draws the blip of an entity trace
/// (`ms_RadarTrace[index]`), sprite pass or trace pass. Type (`+0x26 >> 2 & 0xF`) 1 vehicle, 2 ped (its
/// vehicle when in one, `+0x46C & 0x100`), 3 object: the entity position (entry-exit corrected for peds with
/// `+0x78C`); 6 search light (`+0x14` of `ScriptSearchLightArray[i]`); 7 pickup (`CPickup::GetPosn`); 8 the
/// airstrip arrow (below); 4, 5 and others return. Then: colour, debug marker (`DbgFlag`, radius shrinks by
/// 0.1 and resets to 5 below 1), entry-exit correction, radar / limited / screen point; a short-range blip
/// farther than 1.0 shows only on the map. With `RadarZoomValue`, on foot, and not an object or pickup: no
/// sprite; outside the radius 1 unless `+0x25 & 0x10`; the trace above / below (only with
/// `RadarShowBlipOnAllLevels`) or the rotating arrow sprite 8 (heading + pi in camera mode 1, else
/// `heading - (orientation + pi)`). Otherwise the sprite pass draws the revealed sprite; the trace pass
/// draws search-light circles (within 0.9, not on the map), the airstrip sprites, or the plain trace with
/// its height mode, then adds the legend entry on the map (not for search lights or the airstrip).
///
/// Airstrip (type 8): `FindPlayerVehicle` is called four times without a null test (the original crashes on
/// foot as well); `asDist = sqrt((A.y - P.y)^2 + (A.x - P.x)^2)`. From 500 on: `AIRSTRIP_DONE = 0` and **the
/// position stays uninitialised stack** in the original (it feeds the debug marker, the radar point, the
/// short-range test and the 0.9 test); the port uses (0, 0, 0) there (divergence candidate). Closer: the
/// arrow walks along the strip heading in steps of 100 (every call outside the 0.9 radar radius, else
/// after more than 3 ms), wrapping to `-length` beyond `length`.
///
/// The trace pass's height argument is a dword whose low byte is the mode and whose upper bytes are those of
/// the entity pointer local (0 for search lights and pickups), as the bytes (derived exactly).
/// gta-reversed has no body for this function (`plugin::Call`).
#[hook(addr = 0x587000, conv = cdecl, name = "CRadar::DrawEntityBlip", expect = "83 EC 3C 53 55")]
pub fn draw_entity_blip(index: i32, is_sprite: u8) {
    let t = trace(index);
    let mut ent = GPtr::<u8>::NULL;
    let mut sl = GPtr::<u8>::NULL;
    let mut is_as = false;
    // Uninitialised stack in the original on the far airstrip path (see the doc comment): 0 here.
    let mut pos = [0u32; 3];
    let mut as_dist = 0f32;
    let ty = (rd::<u8>(t, 0x26) >> 2) & 0xF;
    let handle = rd::<i32>(t, 4);
    match ty {
        1..=3 => {
            let e = match ty {
                1 => vehicle_pool_get_at(g::<GPtr<u8>>(VEHICLE_POOL).read(), handle),
                2 => {
                    let p = ped_pool_get_at_ref(g::<GPtr<u8>>(PED_POOL).read(), handle);
                    if p.is_null() {
                        return;
                    }
                    if rd::<u32>(p, 0x46C) & 0x100 != 0 { rd::<GPtr<u8>>(p, 0x58C) } else { p }
                }
                _ => object_pool_get_at(g::<GPtr<u8>>(OBJECT_POOL).read(), handle),
            };
            ent = e;
            if e.is_null() {
                return;
            }
            pos = vec3(entity_pos(e));
            if rd::<u8>(e, 0x36) & 7 == 3 {
                let ee = rd::<GPtr<u8>>(e, 0x78C);
                if !ee.is_null() {
                    ee_get_position_relative_to_outside_world(ee, local(&mut pos).cast());
                }
            }
        }
        6 => {
            let i = get_actual_script_thing_index(handle, 2);
            if i < 0 {
                return;
            }
            sl = g::<u8>(SEARCH_LIGHTS.wrapping_add((i as u32).wrapping_mul(0x7C)) as usize);
            if sl.is_null() {
                return;
            }
            pos = vec3(sl.byte_add(0x14));
        }
        7 => {
            let i = get_actual_pickup_index(handle);
            if i < 0 {
                return;
            }
            let p = g::<u8>(PICKUPS.wrapping_add((i as u32) << 5) as usize);
            if p.is_null() {
                return;
            }
            let mut out = [0u32; 3];
            let r = pickup_get_posn(p, local(&mut out));
            pos = vec3(r.cast());
        }
        8 => {
            is_as = true;
            let p1 = entity_pos(find_player_vehicle(-1, 0));
            let p2 = entity_pos(find_player_vehicle(-1, 0));
            let p3 = entity_pos(find_player_vehicle(-1, 0));
            let p4 = entity_pos(find_player_vehicle(-1, 0));
            let a = g::<u8>(AIRSTRIP_TABLE + usize::from(b(AIRSTRIP_LOCATION)) * 0x10);
            let ay = rd::<f32>(a, 4);
            let ax = rd::<f32>(a, 0);
            let dy = x(ay).fsub_m(rd::<f32>(p4, 4)).fmul(x(ay).fsub_m(rd::<f32>(p3, 4)));
            let dx = x(ax).fsub_m(rd::<f32>(p1, 0)).fmul(x(ax).fsub_m(rd::<f32>(p2, 0)));
            as_dist = f32_of(dy.fadd(dx).fsqrt());
            if x(as_dist).fcom(x(k(K_500))).test_parity(0x05) {
                AIRSTRIP_DONE.set(0);
            } else {
                let mut init = AIRSTRIP_INIT.get();
                let mut bp = if init & 1 == 0 {
                    init |= 1;
                    AIRSTRIP_INIT.set(init);
                    let v = airstrip_start(a);
                    AIRSTRIP_B.set(v);
                    v
                } else {
                    AIRSTRIP_B.get()
                };
                if init & 2 == 0 {
                    init |= 2;
                    AIRSTRIP_INIT.set(init);
                    AIRSTRIP_A.set(airstrip_start(a));
                }
                pos = vec3(t.byte_add(8));
                let mut inp = [pos[0], pos[1]];
                let mut r = [0u32; 2];
                transform_real_world_point_to_radar_space_call(local(&mut r).cast(), local(&mut inp).cast());
                let rad = x(rd::<f32>(a, 8)).fmul_m(k(K_DEG2RAD));
                let bf = xi(i32::from(bp));
                let qx = f32_of(rad.fcos().fmul(bf).fadd_m(fb(pos[0])));
                pos[0] = qx.to_bits();
                let qy = f32_of(rad.fsin().fmul(bf).fsubr_m(fb(pos[1])));
                pos[1] = qy.to_bits();
                let mut q = [pos[0], pos[1]];
                transform_real_world_point_to_radar_space_call(local(&mut r).cast(), local(&mut q).cast());
                let now = g::<u32>(TIME_MS_PAUSE).read();
                let m = radar_len(&r);
                let below = m.fcom(x(k(K_0_9))).test_any(0x01);
                if !below || now.wrapping_sub(AIRSTRIP_TIME.get()) > 3 {
                    bp = bp.wrapping_add(100);
                    AIRSTRIP_B.set(bp);
                    if !xi(i32::from(bp)).fcom(x(rd::<f32>(a, 0xC))).test_any(0x41) {
                        bp = airstrip_start(a);
                        AIRSTRIP_B.set(bp);
                    }
                    if AIRSTRIP_A.get() == bp {
                        AIRSTRIP_DONE.set(0);
                    }
                    if !m.fcom(x(k(K_0_9))).test_parity(0x05) {
                        AIRSTRIP_A.set(bp);
                        AIRSTRIP_DONE.set(1);
                    }
                    AIRSTRIP_TIME.set(now);
                }
                let _ = limit_radar_point(local(&mut r).cast());
            }
        }
        _ => return,
    }

    // COMMON (0x58713D -> 0x405B03 -> 0x587143)
    let f25 = rd::<u8>(t, 0x25);
    let colour = get_radar_trace_colour_call(rd::<u32>(t, 0), u32::from(f25 & 1), u32::from((f25 >> 3) & 1));
    let d = rd::<u8>(t, 0x26) & 3;
    if (d == 3 || d == 1) && b(SCRIPTS_DBG_FLAG) != 0 {
        show_radar_marker_call(pos[0], pos[1], pos[2], colour, rd::<u32>(t, 0x18));
        let r = x(rd::<f32>(t, 0x18)).fsub_m(k(K_0_1));
        wr::<f32>(t, 0x18, f32_of(r));
        if !r.fcom(x(k(K_1))).test_parity(0x05) {
            wr::<u32>(t, 0x18, FIVE_BITS);
        }
    }
    let ee = rd::<GPtr<u8>>(t, 0x20);
    if !ee.is_null() {
        ee_get_position_relative_to_outside_world(ee, local(&mut pos).cast());
    }
    let d = rd::<u8>(t, 0x26) & 3;
    if d != 3 && d != 2 {
        return;
    }
    let mut inp = [pos[0], pos[1]];
    let mut r = [0u32; 2];
    transform_real_world_point_to_radar_space_call(local(&mut r).cast(), local(&mut inp).cast());
    let lim = f32_of(limit_radar_point(local(&mut r).cast()));
    let mut s = [0u32; 2];
    transform_radar_point_to_screen_space_call(local(&mut s).cast(), local(&mut r).cast());
    if rd::<u8>(t, 0x25) & 4 != 0 && !x(lim).fcom(x(k(K_1))).test_any(0x41) && b(MENU_DRAWING_MAP) == 0 {
        return;
    }

    if b(SCRIPTS_RADAR_ZOOM_VALUE) != 0 && find_player_vehicle(-1, 0).is_null() && {
        let ty2 = (rd::<u8>(t, 0x26) >> 2) & 0xF;
        ty2 != 3 && ty2 != 7
    } {
        // ZOOM (0x5872A0)
        if is_sprite != 0 {
            return;
        }
        if rd::<u8>(t, 0x25) & 0x10 == 0 && radar_len(&r).fcom(x(k(K_1))).test_parity(0x41) {
            return;
        }
        let heading = if ent.is_null() {
            0f32
        } else {
            f32_of(X87::from_f64_exact(placeable_get_heading_st0(ent)))
        };
        if b(MENU_DRAWING_MAP) != 0 && b(MENU_SHOW_MISSION_BLIPS) == 0 {
            return;
        }
        let z = fb(pos[2]);
        let mut c = [0f32; 3];
        find_player_centre_of_world_for_map(local(&mut c), 0);
        if !x(z).fsub_m(k(K_2)).fcom(x(c[2])).test_any(0x41) {
            if b(SCRIPTS_RADAR_SHOW_ALL_LEVELS) == 0 {
                return;
            }
            trace_with_height(&s, t, colour, 0);
            return;
        }
        if !x(z).fadd_m(k(K_4)).fcom(x(c[2])).test_parity(0x05) {
            if b(SCRIPTS_RADAR_SHOW_ALL_LEVELS) == 0 {
                return;
            }
            trace_with_height(&s, t, colour, 1);
            return;
        }
        let cam = usize::from(b(CAMERA_ACTIVE_CAM)) * 0x238;
        let mode1 = g::<u16>(CAM_MODE + cam).read() == 1;
        let col = rgba(colour >> 24, colour >> 16, colour >> 8, 0xFF);
        let h8 = size(max_h(), K_1_448, K_8);
        let w8 = size(max_w(), K_1_640, K_8);
        let angle = if mode1 {
            f32_of(x(heading).fadd_m(k(K_PI)))
        } else {
            f32_of(x(g::<f32>(RADAR_ORIENTATION).read()).fadd_m(k(K_PI)).fsubr_m(heading))
        };
        draw_rotating_radar_sprite_call(g(SPRITE_ARROW), fb(s[0]), fb(s[1]), angle, w8, h8, col);
        return;
    }

    // NOT ZOOM (0x587790)
    let sprite = rd::<u8>(t, 0x24);
    if is_sprite != 0 {
        if sprite == 0 || has_this_blip_been_revealed_call(index) == 0 {
            return;
        }
        let sp = rd::<i8>(t, 0x24) as i16 as u16;
        draw_radar_sprite_call(u32::from(sp), fb(s[0]), fb(s[1]), 0xFF);
        return;
    }
    if sprite != 0 {
        return;
    }
    let map = b(MENU_DRAWING_MAP);
    if map != 0 && b(MENU_SHOW_MISSION_BLIPS) == 0 {
        return;
    }
    if !sl.is_null() {
        // Search light: circles within 0.9 and off the map; never a legend entry.
        if radar_len(&r).fcom(x(k(K_0_9))).test_parity(0x41) || map != 0 {
            return;
        }
        let r1 = f32_of(
            max_w()
                .fmul_m(k(K_1_640))
                .fmul_m(k(K_94))
                .fdiv_m(g::<f32>(RADAR_RANGE).read())
                .fmul_m(rd::<f32>(sl, 0x20))
                .fmul_m(k(K_0_6)),
        );
        let mut c1 = [0u8; 4];
        let cp1 = rgba_ctor(local(&mut c1), 0, 0, 0, 0x96);
        let mut sc = s;
        sprite2d_draw_circle_at_near_clip(local(&mut sc), r1, cp1, 15);
        let mut c2 = [0u8; 4];
        let cp2 = rgba_ctor(local(&mut c2), 0xDC, 0xDC, 0xDC, 0xC8);
        let mut sc2 = s;
        let r2 = f32_of(max_w().fmul_m(k(K_1_640)).fsubr_m(r1));
        sprite2d_draw_circle_at_near_clip(local(&mut sc2), r2, cp2, 15);
        return;
    }
    if is_as {
        if map == 0
            && !radar_len(&r).fcom(x(k(K_0_9))).test_parity(0x05)
            && !x(as_dist).fcom(x(k(K_500))).test_parity(0x05)
        {
            let white = rgba(0xFF, 0xFF, 0xFF, 0xFF);
            let h8 = size(max_h(), K_1_448, K_8);
            let w8 = size(max_w(), K_1_640, K_8);
            draw_rotating_radar_sprite_call(g(SPRITE_AIRSTRIP_0), fb(s[0]), fb(s[1]), 0.0, w8, h8, white);
        }
        pos = vec3(t.byte_add(8));
        let mut inp = [pos[0], pos[1]];
        transform_real_world_point_to_radar_space_call(local(&mut r).cast(), local(&mut inp).cast());
        let _ = limit_radar_point(local(&mut r).cast());
        transform_radar_point_to_screen_space_call(local(&mut s).cast(), local(&mut r).cast());
        if b(MENU_DRAWING_MAP) != 0 {
            let loc = b(AIRSTRIP_LOCATION);
            let mut a = rd::<f32>(g::<u8>(AIRSTRIP_TABLE + usize::from(loc) * 0x10), 8);
            if loc != 1 {
                a = f32_of(x(a).fsub_m(k(K_90)));
            }
            let white = rgba(0xFF, 0xFF, 0xFF, 0xFF);
            let h16 = size(max_h(), K_1_448, K_16);
            let w16 = size(max_w(), K_1_640, K_16);
            let angle = f32_of(x(a).fmul_m(k(K_DEG2RAD)));
            draw_rotating_radar_sprite_call(g(SPRITE_AIRSTRIP_1), fb(s[0]), fb(s[1]), angle, w16, h16, white);
            add_blip_to_legend_list_call(0, 0x39);
        } else {
            if AIRSTRIP_DONE.get() != 0 {
                return;
            }
            let white = rgba(0xFF, 0xFF, 0xFF, 0xFF);
            let h16 = size(max_h(), K_1_448, K_16);
            let w16 = size(max_w(), K_1_640, K_16);
            let orient = x(g::<f32>(RADAR_ORIENTATION).read()).fchs();
            let a = g::<u8>(AIRSTRIP_TABLE + usize::from(b(AIRSTRIP_LOCATION)) * 0x10);
            let turn = x(rd::<f32>(a, 8)).fsub_m(k(K_90)).fmul_m(k(K_DEG2RAD));
            let angle = f32_of(orient.fsub(turn));
            draw_rotating_radar_sprite_call(g(SPRITE_AIRSTRIP_1), fb(s[0]), fb(s[1]), angle, w16, h16, white);
        }
        // LEGEND: the airstrip never adds one (isAS).
        return;
    }

    // PLAIN (0x587B02)
    let z = fb(pos[2]);
    let mut c = [0f32; 3];
    find_player_centre_of_world_for_map(local(&mut c), 0);
    let mode: u32 = if !x(z).fsub_m(k(K_2)).fcom(x(c[2])).test_any(0x41) {
        0
    } else if !x(z).fadd_m(k(K_4)).fcom(x(c[2])).test_parity(0x05) {
        1
    } else {
        2
    };
    let height = (ent.addr() as u32 & 0xFFFF_FF00) | mode;
    trace_with_height(&s, t, colour, height);
    // LEGEND (0x587B84 -> 0x401418 -> 0x587B89)
    if b(MENU_DRAWING_MAP) != 0 {
        add_blip_to_legend_list_call(1, index);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_DRAW_LEGEND, HOOK_DRAW_ENTITY_BLIP];
