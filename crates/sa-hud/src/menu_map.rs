//! `CMenuManager::PrintMap` (PORT-HUD-3). Ported from the 1.0 US bytes; gta-reversed
//! `Frontend/MenuManager_Pages.cpp` as reading aid (the bytes win).
//!
//! Only the orchestration is Rust: conditions, the inlined `StretchX`/`StretchY` (`W == 640 ? v : W * (v /
//! 640)` with the precomputed `.rdata` factors), x87 arithmetic in the original order, `FrontEndMenuManager`
//! fields through `this`, globals by address. Every drawing, radar, font, text, menu-system and pad call stays
//! an `orig!` call.
//!
//! `this` fields used (offsets from the bytes): +0x30 system menu (i8), +0x4A map legend, +0x59 drawing map,
//! +0x5A streaming disabled, +0x5B all streaming stuff loaded, +0x64 map zoom, +0x68 / +0x6C map origin,
//! +0x70 / +0x74 map cursor (world), +0x78 map loaded, +0xB8 display the mouse, +0xEA player number, +0x148
//! `m_apBackgroundTextures[7]`.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, ftol32, g, rd, wr, x, xi};

/// No globals of its own (the radar visibility state at 0xBA8270 / 0xBA8274 is addressed directly, as the
/// bytes do).
pub const GLOBALS: &[GlobalDesc] = &[];

/// `RsGlobal.maximumWidth` / `maximumHeight`
const RS_MAX_WIDTH: usize = 0xC17044;
const RS_MAX_HEIGHT: usize = 0xC17048;
/// `CPad::NewKeyState` words the map tests for its overview reset (gr: `standardKeys['Z']` / `['z']`)
const KEY_Z_UPPER: usize = 0xB7325C;
const KEY_Z_LOWER: usize = 0xB7329C;
/// `CPad::NewMouseControllerState.lmb` (byte 0xB73418)
const MOUSE_LMB: usize = 0xB73418;
/// `CTimer::m_snTimeInMillisecondsPauseMode`
const TIME_MS_PAUSE: usize = 0xB7CB7C;
/// Radar visibility change time (u32) and "view radar" (byte), addressed absolutely by the bytes (inside
/// `FrontEndMenuManager`, +0x1B2C / +0x1B28)
const RADAR_VIS_TIME: usize = 0xBA8274;
const VIEW_RADAR: usize = 0xBA8270;
/// `CTheZones::ZonesRevealed` (u32) and `CTheZones::ZonesVisited[10][10]`
const ZONES_REVEALED: usize = 0xBA372C;
const ZONES_VISITED: usize = 0xBA3730;
/// `CTheScripts::HideAllFrontEndMapBlips`, `CTheScripts::bPlayerIsOffTheMap`
const SCRIPTS_HIDE_MAP_BLIPS: usize = 0xA444A1;
const SCRIPTS_PLAYER_OFF_THE_MAP: usize = 0xA444A4;
/// `CRadar::MapLegendCounter` (u16) and `CRadar::MapLegendList[]` (i16)
const MAP_LEGEND_COUNTER: usize = 0xBA86B8;
const MAP_LEGEND_LIST: usize = 0xBA8318;

// .rdata strings
/// `"FEM_PWT"`, `"FE_MLG"`, `"FEH_MPB"`, `"FEH_MPH"`
const STR_FEM_PWT: usize = 0x865364;
const STR_FE_MLG: usize = 0x865334;
const STR_FEH_MPB: usize = 0x86531C;
const STR_FEH_MPH: usize = 0x865314;

// .rdata float constants (bits)
/// `1/640` (0x859520), `1/448` (0x859524), `1/6` (0x85F0A0), `0.2` (0x858CC4)
const K_1_640: u32 = 0x3ACC_CCCD;
const K_1_448: u32 = 0x3B12_4925;
const K_1_6: u32 = 0x3E2A_AAAB;
const K_0_2: u32 = 0x3E4C_CCCD;
/// `70.0` (0x858CE0), `140.0` (0x865374), `141.0` (0x86535C), `145.0` (0x865360)
const K_70: u32 = 0x428C_0000;
const K_140: u32 = 0x430C_0000;
const K_141: u32 = 0x430D_0000;
const K_145: u32 = 0x4311_0000;
/// Inlined stretch constants: `60.0` (0x858B34) / `60/448` (0x8652BC) / `60/640` (0x8652B8); `388.0`
/// (0x86525C) / `388/448` (0x865370); `580.0` (0x865260) / `580/640` (0x86536C); `4.0` (0x858B90) / `4/640`
/// (0x859050) / `4/448` (0x865358); `0.8/448` (0x8652D8), `0.8/640` (0x865354); `30.0` (0x858CA4) / `30/448`
/// (0x865350) / `30/640` (0x86534C); `100.0` (0x858628) / `100/448` (0x865308); `19.0` (0x865348) / `19/448`
/// (0x865344); `555.0` (0x865340) / `555/640` (0x86533C); `95.0` (0x86529C) / `95/640` (0x865298); `0.3/640`
/// (0x86530C), `0.55/448` (0x865330); `127.0` (0x86532C) / `127/448` (0x865328); `160.0` (0x858970) /
/// `160/640` (0x858C84); `350.0` (0x858A4C) / `350/640` (0x865324)
const K_60: u32 = 0x4270_0000;
const K_60_448: u32 = 0x3E09_2492;
const K_60_640: u32 = 0x3DC0_0000;
const K_388: u32 = 0x43C2_0000;
const K_388_448: u32 = 0x3F5D_B6DB;
const K_580: u32 = 0x4411_0000;
const K_580_640: u32 = 0x3F68_0000;
const K_4: u32 = 0x4080_0000;
const K_4_640: u32 = 0x3BCC_CCCD;
const K_4_448: u32 = 0x3C12_4925;
const K_0_8_448: u32 = 0x3AEA_0EA1;
const K_0_8_640: u32 = 0x3AA3_D70A;
const K_30: u32 = 0x41F0_0000;
const K_30_448: u32 = 0x3D89_2492;
const K_30_640: u32 = 0x3D40_0000;
const K_100: u32 = 0x42C8_0000;
const K_100_448: u32 = 0x3E64_9249;
const K_19: u32 = 0x4198_0000;
const K_19_448: u32 = 0x3D2D_B6DB;
const K_555: u32 = 0x440A_C000;
const K_555_640: u32 = 0x3F5E_0000;
const K_95: u32 = 0x42BE_0000;
const K_95_640: u32 = 0x3E18_0000;
const K_0_3_640: u32 = 0x39F5_C290;
const K_0_55_448: u32 = 0x3AA0_EA0F;
const K_127: u32 = 0x42FE_0000;
const K_127_448: u32 = 0x3E91_2492;
const K_160: u32 = 0x4320_0000;
const K_160_640: u32 = 0x3E80_0000;
const K_350: u32 = 0x43AF_0000;
const K_350_640: u32 = 0x3F0C_0000;
/// Immediates: map origin (320, 206), zoom 140, the clamp 70, 0.8, 640.0, 84.0, 10.0, 0.3, 0.55
const ORIGIN_X_BITS: u32 = 0x43A0_0000;
const ORIGIN_Y_BITS: u32 = 0x434E_0000;
const ZOOM_BITS: u32 = 0x430C_0000;
const ZOOM_MIN_BITS: u32 = 0x428C_0000;
const IMM_0_8: u32 = 0x3F4C_CCCD;
const IMM_640: u32 = 0x4420_0000;
const IMM_84: u32 = 0x42A8_0000;
const IMM_10: u32 = 0x4120_0000;
const IMM_0_3: u32 = 0x3E99_999A;
const IMM_0_55: u32 = 0x3F0C_CCCD;

fn k(bits: u32) -> f32 {
    f32::from_bits(bits)
}

fn b(addr: usize) -> u8 {
    g::<u8>(addr).read()
}

fn w() -> i32 {
    g::<i32>(RS_MAX_WIDTH).read()
}

fn h() -> i32 {
    g::<i32>(RS_MAX_HEIGHT).read()
}

/// Inlined `StretchX`: `fld v` at 640 wide, else `fild W; fmul v/640`.
fn str_x(v: u32, per: u32) -> X87 {
    if w() == 0x280 { x(k(v)) } else { xi(w()).fmul_m(k(per)) }
}

/// Inlined `StretchY`: `fld v` at 448 high, else `fild H; fmul v/448`.
fn str_y(v: u32, per: u32) -> X87 {
    if h() == 0x1C0 { x(k(v)) } else { xi(h()).fmul_m(k(per)) }
}

/// `StretchX(s)` of a stored float `s`: `s` at 640 wide, else `W * s * (1/640)`.
fn str_xf(s: f32) -> X87 {
    if w() == 0x280 { x(s) } else { xi(w()).fmul_m(s).fmul_m(k(K_1_640)) }
}

/// `StretchY(s)` of a stored float `s`: `s` at 448 high, else `H * s * (1/448)`.
fn str_yf(s: f32) -> X87 {
    if h() == 0x1C0 { x(s) } else { xi(h()).fmul_m(s).fmul_m(k(K_1_448)) }
}

/// The tile loop's register pair for `StretchX(s)` / `StretchY(s)`: (the x87 value, its f32 reload).
/// At 640 / 448 both are `fld s`; otherwise the value is stored (`fst`) and reloaded.
fn str_pair(v: X87) -> (X87, X87) {
    (v, x(f32_of(v)))
}

/// A local buffer as a game pointer (the original's stack temporaries).
fn local<T, const N: usize>(buf: &mut [T; N]) -> GPtr<T> {
    GPtr::from_raw(buf.as_mut_ptr())
}

orig! {
    /// `CPad::GetPad(int32)` (0x53FB70; the result is unused by `PrintMap`)
    fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CRadar::InitFrontEndMap()` (0x585960)
    fn init_front_end_map_call() = cdecl 0x585960 => crate::radar_draw::init_front_end_map;
    /// `CMenuSystem::SwitchOffMenu(uint8)` (0x580750)
    fn switch_off_menu_call(menu: u32) = cdecl 0x580750 => unported;
    /// `FindPlayerCentreOfWorld_NoSniperShift(int32)` (0x56E320; returns a `CVector*`)
    fn find_player_centre_of_world_no_sniper_shift(id: i32) -> GPtr<u8> = cdecl 0x56E320 => sa_world::world::cworld::find_player_centre_of_world_no_sniper_shift;
    /// `CRadar::TransformRealWorldPointToRadarSpace(CVector2D& out, const CVector2D& in)` (0x583530)
    fn transform_real_world_point_to_radar_space_call(out: GPtr<f32>, inp: GPtr<f32>) = cdecl 0x583530 => crate::radar::transform_real_world_point_to_radar_space;
    /// `CRadar::LimitRadarPoint(CVector2D&)` (0x5832F0, ST0; discarded here)
    fn limit_radar_point_call_st0(p: GPtr<f32>) -> f64 = cdecl 0x5832F0 => limit_radar_point_st0_rust;
    /// `CRadar::TransformRadarPointToScreenSpace(CVector2D& out, const CVector2D& in)` (0x583480; preserves EDX)
    fn transform_radar_point_to_screen_space_call(out: GPtr<f32>, inp: GPtr<f32>) = cdecl 0x583480 => crate::radar::transform_radar_point_to_screen_space;
    /// `CRGBA::CRGBA(r, g, b, a)` (0x7170C0)
    fn rgba_ctor(this: GPtr<u8>, r: u32, gg: u32, bb: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `CSprite2d::DrawRect(const CRect&, const CRGBA&)` (0x727B60)
    fn sprite2d_draw_rect_static(rect: GPtr<f32>, colour: GPtr<u8>) = cdecl 0x727B60 => unported;
    /// `CSprite2d::Draw(const CRect&, const CRGBA&)` (0x728350)
    fn sprite2d_draw_rect(this: GPtr<u8>, rect: GPtr<f32>, colour: GPtr<u8>) = thiscall 0x728350 => unported;
    /// `CMenuManager::SmallMessageScreen(const char*)` (0x574010)
    fn small_message_screen_call(this: GPtr<u8>, key: GPtr<u8>) = thiscall 0x574010 => crate::menu_mgr::small_message_screen;
    /// `CRadar::DrawRadarSectionMap(int32 x, int32 y, CRect rect)` (0x586520; the rect by value)
    fn draw_radar_section_map_call(tx: i32, ty: i32, left: f32, bottom: f32, right: f32, top: f32) = cdecl 0x586520 => crate::radar_draw2::draw_radar_section_map;
    /// `CRadar::DrawRadarGangOverlay(bool inMenu)` (0x586650)
    fn draw_radar_gang_overlay_call(in_menu: u32) = cdecl 0x586650 => crate::radar_draw2::draw_radar_gang_overlay;
    /// `CRadar::DrawBlips()` (0x588050)
    fn draw_blips_call() = cdecl 0x588050 => crate::radar_draw2::draw_blips;
    /// `CPlaceName::GetForMap(float x, float y)` (0x571D90, `ret 8`; the original loads ECX = 0xBA18F4, which the
    /// callee does not read)
    fn place_name_get_for_map(px: u32, py: u32) -> GPtr<u8> = stdcall 0x571D90 => crate::place::get_for_map;
    /// `CTheZones::GetCurrentZoneLockedOrUnlocked(float x, float y)` (0x572130)
    fn zone_locked_or_unlocked(px: u32, py: u32) -> u8 = cdecl 0x572130 => unported;
    /// `CMenuManager::DrawWindow(const CRect&, const char* key, uint8 colour, CRGBA back, bool unused, bool
    /// background)` (0x573EE0, thiscall)
    fn draw_window_call(this: GPtr<u8>, rect: GPtr<u8>, key: GPtr<u8>, colour: u32, back: u32, unused: u32, bg: u32) = thiscall 0x573EE0 => crate::menu_mgr::draw_window;
    /// `CRadar::DrawLegend(int32 x, int32 y, eRadarSprite)` (0x5828A0)
    fn draw_legend_call(px: i32, py: i32, sprite: i32) = cdecl 0x5828A0 => crate::radar_draw3::draw_legend;
    /// `CMenuSystem::Process(uint8 menu)` (0x582630)
    fn menu_system_process(menu: u32) = cdecl 0x582630 => crate::menu2::process;
    /// `CMenuManager::DisplayHelperText(const char*)` (0x57E240)
    fn display_helper_text_call(this: GPtr<u8>, key: GPtr<u8>) = thiscall 0x57E240 => crate::menu_mgr::display_helper_text;

    /// `CFont::SetFontStyle(eFontStyle)` (0x719490)
    fn font_set_font_style(v: u32) = cdecl 0x719490 => crate::font::set_font_style;
    /// `CFont::SetWrapx(float)` (0x7194D0)
    fn font_set_wrapx(v: u32) = cdecl 0x7194D0 => crate::font::set_wrapx;
    /// `CFont::SetDropColor(CRGBA)` (0x719510)
    fn font_set_drop_color(c: u32) = cdecl 0x719510 => crate::font::set_drop_color;
    /// `CFont::SetScale(float w, float h)` (0x719380)
    fn font_set_scale(w: u32, h: u32) = cdecl 0x719380 => crate::font::set_scale;
    /// `CFont::SetColor(CRGBA)` (0x719430)
    fn font_set_color(c: u32) = cdecl 0x719430 => crate::font::set_color;
    /// `CFont::SetEdge(int8)` (0x719590)
    fn font_set_edge(v: u32) = cdecl 0x719590 => crate::font::set_edge;
    /// `CFont::SetOrientation(eFontAlignment)` (0x719610)
    fn font_set_orientation(v: u32) = cdecl 0x719610 => crate::font::set_orientation;
    /// `CFont::SetRightJustifyWrap(float)` (0x7194F0)
    fn font_set_right_justify_wrap(v: u32) = cdecl 0x7194F0 => crate::font::set_right_justify_wrap;
    /// `CFont::SetDropShadowPosition(int8)` (0x719570)
    fn font_set_drop_shadow_position(v: u32) = cdecl 0x719570 => crate::font::set_drop_shadow_position;
    /// `CFont::PrintString(float x, float y, const GxtChar*)` (0x71A700)
    fn font_print_string(px: f32, py: f32, text: GPtr<u8>) = cdecl 0x71A700 => unported;
}

/// The Rust path of [`limit_radar_point_call_st0`] (phases B/C).
#[allow(dead_code)]
fn limit_radar_point_st0_rust(p: GPtr<f32>) -> f64 {
    crate::radar::limit_radar_point(p).to_f64()
}

/// `CRGBA(r, g, b, a)` on a stack temporary; returns the dword the original passes on by value.
fn rgba(r: u32, gg: u32, bb: u32, a: u32) -> u32 {
    let mut c = [0u8; 4];
    let p = rgba_ctor(local(&mut c), r, gg, bb, a);
    rd::<u32>(p, 0)
}

/// `CSprite2d::DrawRect({l, bt, r, t}, CRGBA(cr, cg, cb, ca))` with the colour built first.
fn draw_rect(l: f32, bt: f32, r: f32, t: f32, c: [u32; 4]) {
    let mut col = [0u8; 4];
    let cp = rgba_ctor(local(&mut col), c[0], c[1], c[2], c[3]);
    let mut rect = [l, bt, r, t];
    sprite2d_draw_rect_static(local(&mut rect), cp);
}

/// One of the three framed rects of the unloaded map: `{ox - d, oy + d, ox + d, oy - d}` stretched.
fn origin_rect(this: GPtr<u8>, d: u32, c: [u32; 4]) {
    let bt = f32_of(str_yf_x(x(rd::<f32>(this, 0x6C)).fadd_m(k(d))));
    let r = f32_of(str_xf_x(x(rd::<f32>(this, 0x68)).fadd_m(k(d))));
    let t = f32_of(str_yf_x(x(rd::<f32>(this, 0x6C)).fsub_m(k(d))));
    let l = f32_of(str_xf_x(x(rd::<f32>(this, 0x68)).fsub_m(k(d))));
    draw_rect(l, bt, r, t, c);
}

/// `v` stretched in x as the unloaded-map rects do it: `v` at 640 wide, else `v * (int)W * (1/640)`
/// (`fimul`).
fn str_xf_x(v: X87) -> X87 {
    if w() == 0x280 { v } else { v.fmul(xi(w())).fmul_m(k(K_1_640)) }
}

/// `v` stretched in y: `v` at 448 high, else `v * (int)H * (1/448)` (`fimul`).
fn str_yf_x(v: X87) -> X87 {
    if h() == 0x1C0 { v } else { v.fmul(xi(h())).fmul_m(k(K_1_448)) }
}

/// `CMenuManager::PrintMap()` (0x575130, thiscall): the front-end map page.
///
/// Flow as the bytes: `Z`/`z` resets the loaded / mouse flags; `m_bDrawingMap` set; `InitFrontEndMap`. Not
/// loaded yet: switch the system menu off, origin (320, 206), zoom 140, and shrink the zoom by how far the
/// player's screen point lies beyond `origin +- 140` (first matching side of x+, y+, x-, y-), minimum 70.
/// Zoom factors `a = zoom * (1/6)`, `b = zoom * 0.2`; the map area `{60, 60, 580, 388}` (stretched, `_ftol` to
/// int) and the origin offset `stretch(origin - zoom)`. Radar visibility (0xBA8270 / 0xBA8274, pause clock):
/// hidden while streaming is disabled and not yet loaded, shown 400 ms after the last change. Loaded and
/// visible: background, then the 12 x 12 radar sections whose rect `{x * sx + ox, (y + 1) * sy + oy, (x + 1)
/// * sx + ox, y * sy + oy}` (sx, sy = stretched `a`) passes the area tests; loaded but hidden: black and
/// `SmallMessageScreen("FEM_PWT")`; not loaded: three framed rects (145, 141, 140) and the overview sprite
/// (`this+0x148`) at `origin +- zoom`. Then (visible or not loaded) the gang overlay, the fog of the 10 x 10
/// unvisited zones (stretched `b`) while fewer than 80 are revealed, and the blips unless hidden. Loaded: the
/// place name of the cursor (`CPlaceName::GetForMap`, always called), the border rects, the background frame
/// (stretched 4 truncated to a byte), and when visible the place name text (all zones revealed, or the cursor
/// zone unlocked and the left mouse button up) and the legend window with its entries in two columns. Then
/// `CMenuSystem::Process` of the system menu when visible, wrap / helper text, `m_bMapLoaded = true`.
///
/// Deviations from gta-reversed (the bytes win): the visibility state is two absolute globals; the stretched
/// map area values are truncated to int and the 4-pixel frame offsets to a byte; the legend window rect is
/// `{95, y, 555, 100}` (left, bottom, right, top; gr: 550 and top/bottom the other way round) and
/// `DrawWindow`'s fifth argument is 3; the legend's start y is `ftol(ftol(... + s19) + s19)` (truncated each
/// step); the place-name test reads the pad byte 0xB73418 before a `GetPad` call whose result is unused.
#[hook(addr = 0x575130, conv = thiscall, name = "CMenuManager::PrintMap", expect = "83 EC 54 53 55")]
pub fn print_map(this: GPtr<u8>) {
    let player = i32::from(rd::<u8>(this, 0xEA));
    get_pad(player);
    let z = g::<u16>(KEY_Z_UPPER).read() != 0 || {
        get_pad(i32::from(rd::<u8>(this, 0xEA)));
        g::<u16>(KEY_Z_LOWER).read() != 0
    };
    if z {
        wr::<u8>(this, 0x78, 0);
        wr::<u8>(this, 0xB8, 0);
    }
    wr::<u8>(this, 0x59, 1);
    init_front_end_map_call();
    if rd::<u8>(this, 0x78) == 0 {
        let sm = rd::<u8>(this, 0x30);
        if (sm as i8) >= 0 {
            switch_off_menu_call(u32::from(sm));
            wr::<u8>(this, 0x30, 0x9D);
        }
        wr::<u32>(this, 0x68, ORIGIN_X_BITS);
        wr::<u32>(this, 0x6C, ORIGIN_Y_BITS);
        wr::<u32>(this, 0x64, ZOOM_BITS);
        let c = find_player_centre_of_world_no_sniper_shift(0);
        let mut inp = [rd::<u32>(c, 0), rd::<u32>(c, 4)];
        let mut r = [0u32; 2];
        transform_real_world_point_to_radar_space_call(local(&mut r).cast(), local(&mut inp).cast());
        let _ = limit_radar_point_call_st0(local(&mut r).cast());
        let mut s = [0u32; 2];
        transform_radar_point_to_screen_space_call(local(&mut s).cast(), local(&mut r).cast());
        let (sx, sy) = (f32::from_bits(s[0]), f32::from_bits(s[1]));
        let zoom = |this: GPtr<u8>| rd::<f32>(this, 0x64);
        let bx = x(rd::<f32>(this, 0x68)).fadd_m(k(K_140));
        if !x(sx).fcom(bx).test_any(0x41) {
            wr::<f32>(this, 0x64, f32_of(x(sx).fsub(bx).fsubr_m(zoom(this))));
        } else {
            let by = x(rd::<f32>(this, 0x6C)).fadd_m(k(K_140));
            if !x(sy).fcom(by).test_any(0x41) {
                wr::<f32>(this, 0x64, f32_of(x(sy).fsub(by).fsubr_m(zoom(this))));
            } else {
                let bx2 = x(rd::<f32>(this, 0x68)).fsub_m(k(K_140));
                if !x(sx).fcom(bx2).test_parity(0x05) {
                    wr::<f32>(this, 0x64, f32_of(bx2.fsub_m(sx).fsubr_m(zoom(this))));
                } else {
                    let by2 = x(rd::<f32>(this, 0x6C)).fsub_m(k(K_140));
                    if !x(sy).fcom(by2).test_parity(0x05) {
                        wr::<f32>(this, 0x64, f32_of(by2.fsub_m(sy).fsubr_m(zoom(this))));
                    }
                }
            }
        }
        if !x(zoom(this)).fcom(x(k(K_70))).test_parity(0x05) {
            wr::<u32>(this, 0x64, ZOOM_MIN_BITS);
        }
    }

    let za = f32_of(x(rd::<f32>(this, 0x64)).fmul_m(k(K_1_6)));
    let zb = f32_of(x(rd::<f32>(this, 0x64)).fmul_m(k(K_0_2)));
    let y60 = ftol32(str_y(K_60, K_60_448));
    let y388 = ftol32(str_y(K_388, K_388_448));
    let x60 = ftol32(str_x(K_60, K_60_640));
    let x580 = ftol32(str_x(K_580, K_580_640));
    let dxo = x(rd::<f32>(this, 0x68)).fsub_m(rd::<f32>(this, 0x64));
    let offx = f32_of(if w() == 0x280 { dxo } else { xi(w()).fmul(dxo).fmul_m(k(K_1_640)) });
    let dyo = x(rd::<f32>(this, 0x6C)).fsub_m(rd::<f32>(this, 0x64));
    let offy = f32_of(if h() == 0x1C0 { dyo } else { xi(h()).fmul(dyo).fmul_m(k(K_1_448)) });

    let loaded = rd::<u8>(this, 0x78) != 0;
    if loaded {
        let now = g::<u32>(TIME_MS_PAUSE).read();
        if rd::<u8>(this, 0x5A) != 0 && rd::<u8>(this, 0x5B) == 0 {
            g::<u32>(RADAR_VIS_TIME).write(now);
            g::<u8>(VIEW_RADAR).write(0);
        }
        if now.wrapping_sub(g::<u32>(RADAR_VIS_TIME).read()) > 0x190 {
            g::<u8>(VIEW_RADAR).write(1);
        }
    } else {
        g::<u32>(RADAR_VIS_TIME).write(g::<u32>(TIME_MS_PAUSE).read());
        g::<u8>(VIEW_RADAR).write(0);
    }

    if loaded {
        if rd::<u8>(this, 0x5B) != 0 {
            wr::<u8>(this, 0x5A, 0);
        }
        let wf = f32_of(xi(w()));
        let hf = f32_of(xi(h()));
        if b(VIEW_RADAR) == 0 {
            draw_rect(0.0, hf, wf, 0.0, [0, 0, 0, 0xFF]);
            small_message_screen_call(this, g(STR_FEM_PWT));
            wr::<u8>(this, 0x5B, 1);
        } else {
            draw_rect(0.0, hf, wf, 0.0, [0x6F, 0x89, 0xAA, 0xFF]);
            print_map_sections(za, offx, offy, [y60, y388, x60, x580]);
        }
    } else {
        origin_rect(this, K_145, [0x64, 0x64, 0x64, 0xFF]);
        origin_rect(this, K_141, [0, 0, 0, 0xFF]);
        origin_rect(this, K_140, [0x6F, 0x89, 0xAA, 0xFF]);
        let zoom = rd::<f32>(this, 0x64);
        let bt = f32_of(str_yf_x(x(zoom).fadd_m(rd::<f32>(this, 0x6C))));
        let r = f32_of(str_xf_x(x(rd::<f32>(this, 0x68)).fadd_m(rd::<f32>(this, 0x64))));
        let t = f32_of(str_yf_x(x(rd::<f32>(this, 0x6C)).fsub_m(rd::<f32>(this, 0x64))));
        let l = f32_of(str_xf_x(x(rd::<f32>(this, 0x68)).fsub_m(rd::<f32>(this, 0x64))));
        let mut col = [0u8; 4];
        let cp = rgba_ctor(local(&mut col), 0xFF, 0xFF, 0xFF, 0xFF);
        let mut rect = [l, bt, r, t];
        sprite2d_draw_rect(this.byte_add(0x148), local(&mut rect), cp);
    }

    let mut tail = true;
    if b(VIEW_RADAR) != 0 || rd::<u8>(this, 0x78) == 0 {
        draw_radar_gang_overlay_call(1);
        if g::<u32>(ZONES_REVEALED).read() < 0x50 {
            for zx in 0..10i32 {
                for zy in 0..10i32 {
                    if b(ZONES_VISITED + (zy + zx * 10) as usize) != 0 {
                        continue;
                    }
                    let s1 = str_yf(zb);
                    let s2 = str_xf(zb);
                    let s3 = str_yf(zb);
                    let s4 = str_xf(zb);
                    let l = f32_of(xi(zx).fmul(s4).fadd_m(offx));
                    let t = f32_of(xi(zy).fmul(s3).fadd_m(offy));
                    let r = f32_of(xi(zx + 1).fmul(s2).fadd_m(offx));
                    let bt = f32_of(xi(zy + 1).fmul(s1).fadd_m(offy));
                    draw_rect(l, bt, r, t, [0x6F, 0x89, 0xAA, 0xC8]);
                }
            }
        }
        if b(SCRIPTS_HIDE_MAP_BLIPS) == 0 && b(SCRIPTS_PLAYER_OFF_THE_MAP) == 0 {
            draw_blips_call();
        }
        if rd::<u8>(this, 0x78) == 0 {
            tail = false;
        }
    }
    let mut skip_process = false;
    if tail {
        skip_process = print_map_loaded(this, [y60, y388, x60, x580]);
    }
    if !skip_process && b(VIEW_RADAR) != 0 {
        let sm = rd::<u8>(this, 0x30);
        if (sm as i8) >= 0 {
            menu_system_process(u32::from(sm));
        }
    }
    wr::<u8>(this, 0x59, 0);
    font_set_wrapx(f32_of(xi(w().wrapping_sub(10))).to_bits());
    font_set_right_justify_wrap(IMM_10);
    if rd::<u8>(this, 0x78) != 0 {
        let key = if (rd::<u8>(this, 0x30) as i8) < 0 { STR_FEH_MPH } else { STR_FEH_MPB };
        display_helper_text_call(this, g(key));
    }
    wr::<u8>(this, 0x78, 1);
}

/// The 12 x 12 radar sections of the loaded, visible map (0x5754A2..0x575710): a section is drawn when
/// `area.left - s < x * s + ox`, `area.bottom - s < y * s + oy`, `area.right + s > (x + 1) * s + ox` and
/// `area.top + s > (y + 1) * s + oy` (`s` = stretched `zoom / 6`, as a register value and its f32 reload).
fn print_map_sections(za: f32, offx: f32, offy: f32, area: [i32; 4]) {
    let [y60, y388, x60, x580] = area;
    let x60f = f32_of(xi(x60));
    let sx = || str_pair(str_xf(za));
    let sy = || str_pair(str_yf(za));
    for tx in 0..12i32 {
        let xf = f32_of(xi(tx));
        for ty in 0..12i32 {
            let (v, re) = sx();
            if x(x60f).fsub(re).fcom(x(xf).fmul(v).fadd_m(offx)).test_parity(0x05) {
                continue;
            }
            let (v, re) = sy();
            let yf = f32_of(xi(ty));
            if xi(y60).fsub(re).fcom(x(yf).fmul(v).fadd_m(offy)).test_parity(0x05) {
                continue;
            }
            let (v, re) = sx();
            let x1f = f32_of(xi(tx + 1));
            if xi(x580).fadd(re).fcom(x(x1f).fmul(v).fadd_m(offx)).test_any(0x41) {
                continue;
            }
            let (v, re) = sy();
            let y1f = f32_of(xi(ty + 1));
            if xi(y388).fadd(re).fcom(x(y1f).fmul(v).fadd_m(offy)).test_any(0x41) {
                continue;
            }
            let s1 = str_yf(za);
            let s2 = str_xf(za);
            let s3 = str_yf(za);
            let s4 = str_xf(za);
            let left = f32_of(s4.fmul_m(xf).fadd_m(offx));
            let top = f32_of(s3.fmul_m(yf).fadd_m(offy));
            let right = f32_of(s2.fmul_m(x1f).fadd_m(offx));
            let bottom = f32_of(s1.fmul_m(y1f).fadd_m(offy));
            draw_radar_section_map_call(tx, ty, left, bottom, right, top);
        }
    }
}

/// The loaded part of `PrintMap` (0x575B54..0x576291): place name lookup, the frame, and when the radar is
/// visible the place name text and the legend. Returns `true` when the radar is not visible (the original
/// jumps past `CMenuSystem::Process` to 0x5762AC).
fn print_map_loaded(this: GPtr<u8>, area: [i32; 4]) -> bool {
    let [y60, y388, x60, x580] = area;
    let mx = rd::<u32>(this, 0x70);
    let my = rd::<u32>(this, 0x74);
    let text = place_name_get_for_map(mx, my);
    let c4x = u32::from(ftol32(str_x(K_4, K_4_640)) as u8);
    let c4y = u32::from(ftol32(str_y(K_4, K_4_448)) as u8);
    let grey = [0x64, 0x64, 0x64, 0xFF];
    let black = [0, 0, 0, 0xFF];
    let wf = || f32_of(xi(w()));
    let hf = || f32_of(xi(h()));
    let y60f = f32_of(xi(y60));
    draw_rect(0.0, y60f, wf(), 0.0, grey);
    let y388f = f32_of(xi(y388));
    draw_rect(0.0, hf(), wf(), y388f, grey);
    let x60f = f32_of(xi(x60));
    draw_rect(0.0, hf(), x60f, 0.0, grey);
    let x580f = f32_of(xi(x580));
    draw_rect(x580f, hf(), wf(), 0.0, grey);
    let v = f32_of(xi(y60.wrapping_sub(c4y as i32)));
    draw_rect(0.0, v, wf(), 0.0, black);
    let v = f32_of(xi((c4y as i32).wrapping_add(y388)));
    draw_rect(0.0, hf(), wf(), v, black);
    let v = f32_of(xi(x60.wrapping_sub(c4x as i32)));
    draw_rect(0.0, hf(), v, 0.0, black);
    let v = f32_of(xi((c4x as i32).wrapping_add(x580)));
    draw_rect(v, hf(), wf(), 0.0, black);

    if b(VIEW_RADAR) == 0 {
        return true;
    }
    let show_name = g::<u32>(ZONES_REVEALED).read() >= 0x50
        || (zone_locked_or_unlocked(rd::<u32>(this, 0x70), rd::<u32>(this, 0x74)) != 0 && {
            let lmb = b(MOUSE_LMB);
            get_pad(i32::from(rd::<u8>(this, 0xEA)));
            lmb == 0
        });
    if show_name {
        font_set_font_style(3);
        font_set_wrapx(IMM_640);
        font_set_drop_color(rgba(0, 0, 0, 0xFF));
        let sh = if h() == 0x1C0 { IMM_0_8 } else { f32_of(xi(h()).fmul_m(k(K_0_8_448))).to_bits() };
        let sw = if w() == 0x280 { IMM_0_8 } else { f32_of(xi(w()).fmul_m(k(K_0_8_640))).to_bits() };
        font_set_scale(sw, sh);
        font_set_color(rgba(0xE1, 0xE1, 0xE1, 0xFF));
        font_set_edge(2);
        font_set_orientation(2);
        let dy = str_y(K_30, K_30_448);
        let dx = str_x(K_30, K_30_640);
        let py = f32_of(x(y388f).fsub(dy));
        let px = f32_of(x(x580f).fsub(dx));
        font_print_string(px, py, text);
    }
    if rd::<u8>(this, 0x4A) == 0 {
        return false;
    }
    let n = i32::from(g::<u16>(MAP_LEGEND_COUNTER).read());
    let mid = (n - 1) / 2;
    let mut y = ftol32(str_y(K_100, K_100_448));
    let it = mid + 3;
    if it > 0 {
        for _ in 0..it {
            let s19 = str_y(K_19, K_19_448);
            y = ftol32(xi(y).fadd(s19));
        }
    }
    let x555 = str_x(K_555, K_555_640);
    let y100 = str_y(K_100, K_100_448);
    let x95 = str_x(K_95, K_95_640);
    let back = rgba(0, 0, 0, 0xBE);
    let mut rect = [f32_of(x95), f32_of(xi(y)), f32_of(x555), f32_of(y100)];
    draw_window_call(this, local(&mut rect).cast(), g(STR_FE_MLG), 0, back, 3, 1);
    font_set_wrapx(f32_of(xi(w().wrapping_add(-40))).to_bits());
    font_set_right_justify_wrap(IMM_84);
    font_set_drop_shadow_position(1);
    font_set_drop_color(rgba(0, 0, 0, 0xFF));
    font_set_orientation(1);
    font_set_font_style(2);
    let sh = if h() == 0x1C0 { IMM_0_55 } else { f32_of(xi(h()).fmul_m(k(K_0_55_448))).to_bits() };
    let sw = if w() == 0x280 { IMM_0_3 } else { f32_of(xi(w()).fmul_m(k(K_0_3_640))).to_bits() };
    font_set_scale(sw, sh);
    font_set_color(rgba(0xAC, 0xCB, 0xF1, 0xFF));
    let mut cy = ftol32(str_y(K_127, K_127_448));
    let mut cx = ftol32(str_x(K_160, K_160_640));
    let mut i = 0i32;
    if g::<u16>(MAP_LEGEND_COUNTER).read() == 0 {
        return false;
    }
    loop {
        let sprite = i32::from(g::<i16>(MAP_LEGEND_LIST + i as usize * 2).read());
        draw_legend_call(cx, cy, sprite);
        if i == mid {
            cx = ftol32(str_x(K_350, K_350_640));
            cy = ftol32(str_y(K_127, K_127_448));
        } else {
            let s19 = str_y(K_19, K_19_448);
            cy = ftol32(xi(cy).fadd(s19));
        }
        i += 1;
        if i >= i32::from(g::<u16>(MAP_LEGEND_COUNTER).read()) {
            break;
        }
    }
    false
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_PRINT_MAP];
