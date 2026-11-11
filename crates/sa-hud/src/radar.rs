//! `CRadar` (PORT-HUD): the blip table (`ms_RadarTrace`, 175 x `tRadarTrace` 0x28) and the pure radar
//! transforms. Ported from the 1.0 US bytes; gta-reversed `Radar.cpp` as reading aid. Rendering,
//! RenderWare and texture calls stay `orig!` calls.
//!
//! `tRadarTrace` (0x28): +0x00 colour, +0x04 entity handle, +0x08 position (CVector), +0x14 counter
//! (u16), +0x18 sphere radius, +0x1C blip size (u16), +0x20 entry/exit, +0x24 sprite (u8), +0x25 flags
//! (bit0 bright, bit1 in use, bit2 short range, bit3 friendly, bit4 always display in zoom, bit5 fade,
//! bits 6-7 appearance), +0x26 display (bits 0-1), blip type (bits 2-5), bits 6-7 kept.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, ftol32, g, rd, wr, x, xi};

/// Number of `ms_RadarTrace` entries.
pub const MAX_RADAR_TRACES: i32 = 175;
/// Size of one `tRadarTrace`.
pub const TRACE_SIZE: i32 = 0x28;

sa_core::globals! {
    list GLOBALS;
    /// `CRadar::airstrip_location` (u8 index into the airstrip table)
    pub static AIRSTRIP_LOCATION @ 0xBA8300: u8 = 0;
    /// `CRadar::airstrip_blip` (blip handle)
    pub static AIRSTRIP_BLIP @ 0xBA8304: i32 = 0;
    /// `CRadar::cachedCos`
    pub static CACHED_COS @ 0xBA8308: f32 = 0.0;
    /// `CRadar::cachedSin`
    pub static CACHED_SIN @ 0xBA830C: f32 = 0.0;
    /// `CRadar::m_fRadarOrientation`
    pub static RADAR_ORIENTATION @ 0xBA8310: f32 = 0.0;
    /// `CRadar::m_radarRange`
    pub static RADAR_RANGE @ 0xBA8314: f32 = 0.0;
    /// `gRadarTextures[12][12]` (TXD slots, row-major, -1 = none)
    pub static RADAR_TEXTURES @ 0xBA8478: [i32; 144] = [0; 144];
    /// `CRadar::ms_RadarTrace[175]` (0x28 each)
    pub static RADAR_TRACE @ 0xBA86F0: [u8; 175 * 0x28] = [0; 175 * 0x28];
    /// `CRadar::vec2DRadarOrigin`
    pub static RADAR_ORIGIN @ 0xBAA248: [f32; 2] = [0.0; 2];
    /// `CRadar::m_radarRect` (left, bottom, right, top)
    pub static RADAR_RECT @ 0x8D0920: [f32; 4] = [1_000_000.0, -1_000_000.0, -1_000_000.0, 1_000_000.0];
}

// Foreign globals (owners: CMenuManager, CGame, CStreaming, RsGlobal), addressed directly.
/// `FrontEndMenuManager.m_bDrawRadarOrMap` (CMenuManager 0xBA6748 +0x59)
const MENU_DRAW_RADAR_OR_MAP: usize = 0xBA67A1;
/// `FrontEndMenuManager.m_fMapZoom` (+0x64)
const MENU_MAP_ZOOM: usize = 0xBA67AC;
/// `FrontEndMenuManager.m_vMapOrigin` (+0x68, CVector2D)
const MENU_MAP_ORIGIN: usize = 0xBA67B0;
/// `FrontEndMenuManager` legend filter bytes (+0x45, +0x46, +0x48) `DisplayThisBlip` tests
const MENU_SHOW_A: usize = 0xBA678D;
const MENU_SHOW_B: usize = 0xBA678E;
const MENU_SHOW_C: usize = 0xBA6790;
/// `RsGlobal.maximumWidth` / `maximumHeight`
const RS_MAX_WIDTH: usize = 0xC17044;
const RS_MAX_HEIGHT: usize = 0xC17048;
/// `CGame::currArea` (read through the protection stub at 0x408040: `[0x15608B8] + [0x52F59D]`)
const CURR_AREA: usize = 0xB72914;
/// `CStreaming::ms_disableStreaming`
const DISABLE_STREAMING: usize = 0x9654B0;
/// `"radar%02d"` (.rdata)
const RADAR_TXD_FMT: usize = 0x866B98;

orig! {
    /// `CRadar::ClearActualBlip(int32)` (0x587C10)
    pub fn clear_actual_blip_call(i: i32) = cdecl 0x587C10 => clear_actual_blip;
    /// `CRadar::SetCoordBlip(eBlipType, CVector, eBlipColour, eBlipDisplay, const char*)` (0x583820)
    pub fn set_coord_blip_call(t: i32, px: f32, py: f32, pz: f32, colour: i32, display: i32, name: GPtr<u8>) -> i32 = cdecl 0x583820 => set_coord_blip;
    /// `CRadar::StreamRadarSections(int32, int32)` (0x584C50)
    pub fn stream_radar_sections_xy_call(x: i32, y: i32) = cdecl 0x584C50 => stream_radar_sections_xy;
    /// `CTheZones::GetCurrentZoneLockedOrUnlocked(CVector2D)` (0x572130)
    pub fn zone_locked_or_unlocked(x: f32, y: f32) -> u8 = cdecl 0x572130 => sa_world::world::zones::get_current_zone_locked_or_unlocked;
    /// `CTxdStore::FindTxdSlot(const char*)` (0x731850)
    pub fn find_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731850 => sa_stream::txdstore::find_txd_slot_name;
    /// `CStreaming::RequestModel(int32, int32)` (0x4087E0)
    pub fn request_model(id: i32, flags: i32) = cdecl 0x4087E0 => sa_stream::streaming::request_model;
    /// `CStreaming::RemoveModel(int32)` (0x4089A0)
    pub fn remove_model(id: i32) = cdecl 0x4089A0 => sa_stream::streaming::remove_model;
    /// `FindPlayerPed(int32)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => sa_world::world::cworld::find_player_ped_port;
    /// CRT `floor(double)` (0x8219F0)
    pub fn crt_floor(v: f64) -> f64 = cdecl 0x8219F0 => unported;
    /// CRT `ceil(double)` (0x823820)
    pub fn crt_ceil(v: f64) -> f64 = cdecl 0x823820 => unported;
}

fn menu_map() -> bool {
    g::<u8>(MENU_DRAW_RADAR_OR_MAP).read() != 0
}

/// Entry `i` of a trace table at `base` (the original's `i * 0x28`, wrapping).
fn tr_in(base: GPtr<u8>, i: i32) -> GPtr<u8> {
    base.byte_add(i.wrapping_mul(TRACE_SIZE) as isize)
}

fn traces() -> GPtr<u8> {
    RADAR_TRACE.ptr().cast()
}

fn tr(i: i32) -> GPtr<u8> {
    tr_in(traces(), i)
}

/// The inlined handle check: `h & 0xFFFF` when the counter matches `h >> 16` and the entry is in use,
/// else -1.
fn actual_index_in(base: GPtr<u8>, h: i32) -> i32 {
    if h == -1 {
        return -1;
    }
    let i = h & 0xFFFF;
    let t = tr_in(base, i);
    if rd::<u16>(t, 0x14) != ((h as u32) >> 16) as u16 || rd::<u8>(t, 0x25) & 2 == 0 {
        return -1;
    }
    i
}

/// The fields `ClearActualBlip` (and its inlined copies) reset.
fn clear_entry(t: GPtr<u8>) {
    wr::<u16>(t, 0x1C, 1);
    wr::<u8>(t, 0x25, 1);
    wr::<f32>(t, 0x18, 1.0);
    wr::<u32>(t, 0x20, 0);
    wr::<u8>(t, 0x24, 0);
    wr::<u8>(t, 0x26, rd::<u8>(t, 0x26) & 0xC0);
}

/// The inlined `GetNewUniqueBlipIndex`: bumps the counter (wrapping to 1 from 0xFFFE) and returns the
/// handle.
fn new_unique_index(i: i32) -> i32 {
    let t = tr(i);
    let c = rd::<u16>(t, 0x14);
    let c = if c < 0xFFFE { c + 1 } else { 1 };
    wr::<u16>(t, 0x14, c);
    ((u32::from(c) << 16) | i as u32) as i32
}

/// The inlined free-slot search of `SetCoordBlip` / `SetEntityBlip` (reads one entry past the found
/// one, as the original; -1 when none of the 175 is free).
fn free_slot() -> i32 {
    let mut i = 0;
    if rd::<u8>(tr(0), 0x25) & 2 != 0 {
        loop {
            if i >= MAX_RADAR_TRACES {
                return -1;
            }
            let b = rd::<u8>(tr(i + 1), 0x25);
            i += 1;
            if b & 2 == 0 {
                break;
            }
        }
        if i >= MAX_RADAR_TRACES {
            return -1;
        }
    }
    i
}

/// `CRadar::GetNewUniqueBlipIndex(int32)` (0x582820).
#[hook(addr = 0x582820, conv = cdecl, name = "CRadar::GetNewUniqueBlipIndex", expect = "8B 54 24 04 8D 04 92")]
pub fn get_new_unique_blip_index(i: i32) -> i32 {
    new_unique_index(i)
}

/// `CRadar::GetActualBlipArrayIndex(int32)` (0x582870).
#[hook(addr = 0x582870, conv = cdecl, name = "CRadar::GetActualBlipArrayIndex", expect = "8B 4C 24 04 83 F9 FF")]
pub fn get_actual_blip_array_index(h: i32) -> i32 {
    actual_index_in(traces(), h)
}

/// [`get_actual_blip_array_index`] over a trace table at `base` (L1).
pub fn get_actual_blip_array_index_in(base: GPtr<u8>, h: i32) -> i32 {
    actual_index_in(base, h)
}

/// `CRadar::ChangeBlipColour(int32, uint32)` (0x583AB0).
#[hook(addr = 0x583AB0, conv = cdecl, name = "CRadar::ChangeBlipColour", expect = "8B 4C 24 04 83 F9 FF")]
pub fn change_blip_colour(h: i32, colour: u32) {
    let i = actual_index_in(traces(), h);
    if i != -1 {
        wr::<u32>(tr(i), 0, colour);
    }
}

/// `CRadar::SetBlipSprite(int32, eRadarSprite)` (0x583D70).
#[hook(addr = 0x583D70, conv = cdecl, name = "CRadar::SetBlipSprite", expect = "8B 4C 24 04 83 F9 FF")]
pub fn set_blip_sprite(h: i32, sprite: i32) {
    let i = actual_index_in(traces(), h);
    if i != -1 {
        wr::<u8>(tr(i), 0x24, sprite as u8);
    }
}

/// `CRadar::ChangeBlipDisplay(int32, eBlipDisplay)` (0x583D20): bits 0-1 of +0x26.
#[hook(addr = 0x583D20, conv = cdecl, name = "CRadar::ChangeBlipDisplay", expect = "8B 54 24 04 83 FA FF")]
pub fn change_blip_display(h: i32, display: i32) {
    let i = actual_index_in(traces(), h);
    if i != -1 {
        let t = tr(i);
        let b = rd::<u8>(t, 0x26);
        wr::<u8>(t, 0x26, b ^ ((b ^ display as u8) & 3));
    }
}

/// `CRadar::ChangeBlipScale(int32, int32)` (0x583CC0): size 1 while the map is shown.
#[hook(addr = 0x583CC0, conv = cdecl, name = "CRadar::ChangeBlipScale", expect = "8B 4C 24 04 83 F9 FF")]
pub fn change_blip_scale(h: i32, size: i32) {
    let i = actual_index_in(traces(), h);
    let s = if menu_map() { 1 } else { size };
    if i != -1 {
        wr::<u16>(tr(i), 0x1C, s as u16);
    }
}

/// `CRadar::ChangeBlipBrightness(int32, int32)` (0x583C70): brightness 1 clears the bright bit, any other
/// value sets it.
#[hook(addr = 0x583C70, conv = cdecl, name = "CRadar::ChangeBlipBrightness", expect = "8B 54 24 04 83 FA FF")]
pub fn change_blip_brightness(h: i32, brightness: i32) {
    let i = actual_index_in(traces(), h);
    if i != -1 {
        let t = tr(i);
        let b = rd::<u8>(t, 0x25);
        wr::<u8>(t, 0x25, if brightness == 1 { b & 0xFE } else { b | 1 });
    }
}

/// One flag bit of +0x25 from the low bit of `v` (`SetBlipFriendly` / `AlwaysDisplayInZoom` / `Fade`).
fn set_flag_bit(h: i32, v: u8, shift: u32) {
    let i = actual_index_in(traces(), h);
    if i != -1 {
        let t = tr(i);
        let b = rd::<u8>(t, 0x25);
        let m = 1u8 << shift;
        wr::<u8>(t, 0x25, b ^ (((v << shift) ^ b) & m));
    }
}

/// `CRadar::SetBlipFriendly(int32, bool)` (0x583EB0).
#[hook(addr = 0x583EB0, conv = cdecl, name = "CRadar::SetBlipFriendly", expect = "8B 54 24 04 83 FA FF")]
pub fn set_blip_friendly(h: i32, v: u8) {
    set_flag_bit(h, v, 3);
}

/// `CRadar::SetBlipAlwaysDisplayInZoom(int32, bool)` (0x583DB0).
#[hook(addr = 0x583DB0, conv = cdecl, name = "CRadar::SetBlipAlwaysDisplayInZoom", expect = "8B 54 24 04 83 FA FF")]
pub fn set_blip_always_display_in_zoom(h: i32, v: u8) {
    set_flag_bit(h, v, 4);
}

/// `CRadar::SetBlipFade(int32, bool)` (0x583E00).
#[hook(addr = 0x583E00, conv = cdecl, name = "CRadar::SetBlipFade", expect = "8B 54 24 04 83 FA FF")]
pub fn set_blip_fade(h: i32, v: u8) {
    set_flag_bit(h, v, 5);
}

/// `CRadar::SetCoordBlipAppearance(int32, eBlipAppearance)` (0x583E50): only for blip type 4 (coord) and
/// an appearance <= 2 (unsigned byte compare).
#[hook(addr = 0x583E50, conv = cdecl, name = "CRadar::SetCoordBlipAppearance", expect = "8B 54 24 04 83 FA FF")]
pub fn set_coord_blip_appearance(h: i32, appearance: u8) {
    let i = actual_index_in(traces(), h);
    if i == -1 {
        return;
    }
    let t = tr(i);
    if rd::<u8>(t, 0x26) & 0x3C != 0x10 || appearance > 2 {
        return;
    }
    wr::<u8>(t, 0x25, (rd::<u8>(t, 0x25) & 0x3F) | (appearance << 6));
}

/// `CRadar::SetBlipEntryExit(int32, CEntryExit*)` (0x583F00).
#[hook(addr = 0x583F00, conv = cdecl, name = "CRadar::SetBlipEntryExit", expect = "8B 4C 24 04 83 F9 FF")]
pub fn set_blip_entry_exit(h: i32, ee: GPtr<u8>) {
    let i = actual_index_in(traces(), h);
    if i != -1 {
        wr(tr(i), 0x20, ee);
    }
}

/// `CRadar::ClearActualBlip(int32)` (0x587C10): only for 0 <= i < 175.
#[hook(addr = 0x587C10, conv = cdecl, name = "CRadar::ClearActualBlip-OG", expect = "8B 44 24 04 33 D2")]
pub fn clear_actual_blip(i: i32) {
    if (0..MAX_RADAR_TRACES).contains(&i) {
        clear_entry(tr(i));
    }
}

/// `CRadar::ClearBlip(int32)` (0x587CE0): tail call of `ClearActualBlip`.
#[hook(addr = 0x587CE0, conv = cdecl, name = "CRadar::ClearBlip", expect = "8B 4C 24 04 83 F9 FF")]
pub fn clear_blip(h: i32) {
    let i = actual_index_in(traces(), h);
    if i != -1 {
        clear_actual_blip_call(i);
    }
}

/// `CRadar::ClearBlipForEntity(eBlipType, int32)` (0x587C60): every entry of that type and entity
/// handle (the counter is not touched).
#[hook(addr = 0x587C60, conv = cdecl, name = "CRadar::ClearBlipForEntity-OG", expect = "53 55 8B 6C 24 10")]
pub fn clear_blip_for_entity(blip_type: i32, handle: i32) {
    for i in 0..MAX_RADAR_TRACES {
        let t = tr(i);
        if i32::from((rd::<u8>(t, 0x26) >> 2) & 0xF) == blip_type && rd::<i32>(t, 4) == handle {
            clear_entry(t);
        }
    }
}

/// `CRadar::HasThisBlipBeenRevealed(int32)` (0x583AF0): false only on the map, for a short-range blip,
/// with fewer than 80 zones revealed (unsigned) and its zone locked.
#[hook(addr = 0x583AF0, conv = cdecl, name = "CRadar::HasThisBlipBeenRevealed", expect = "A0 A1 67 BA 00")]
pub fn has_this_blip_been_revealed(i: i32) -> u8 {
    if !menu_map() {
        return 1;
    }
    let t = tr(i);
    if rd::<u8>(t, 0x25) & 4 == 0 {
        return 1;
    }
    if (sa_world::world::zones::ZONES_REVEALED.get() as u32) >= 0x50 {
        return 1;
    }
    let r = zone_locked_or_unlocked(rd::<f32>(t, 8), rd::<f32>(t, 0xC));
    if r != 0 { 1 } else { r }
}

/// `CRadar::CalculateBlipAlpha(float)` (0x583420): 255 on the map; else
/// `max((uint32)(255 - ftol(d / 6 * 255)), 70.0)` (the clamp is a max, as the bytes).
#[hook(addr = 0x583420, conv = cdecl, name = "CRadar::CalculateBlipAlpha", expect = "51 A0 A1 67 BA 00")]
pub fn calculate_blip_alpha(dist: f32) -> u8 {
    calculate_blip_alpha_with(menu_map(), dist)
}

/// [`calculate_blip_alpha`] with the menu flag as a parameter (L1).
pub fn calculate_blip_alpha_with(map: bool, dist: f32) -> u8 {
    if map {
        return 0xFF;
    }
    let v = ftol32(x(dist).fmul_m(f32::from_bits(0x3E2A_AAAB)).fmul_m(255.0));
    let u = 255i32.wrapping_sub(v);
    let mut f = xi(u);
    if u < 0 {
        f = f.fadd_m(4_294_967_296.0);
    }
    let r = if x(70.0).fcom(f).test_any(0x41) {
        f
    } else {
        x(70.0)
    };
    ftol32(r) as u8
}

/// `CRadar::LimitRadarPoint(CVector2D&)` (stub 0x5832F0): the length; off the map a point beyond 1 is
/// scaled to length 1 (`(1 / d) * x`).
#[hook(addr = 0x5832F0, conv = cdecl, name = "CRadar::LimitRadarPoint", expect = "E9 D0 EB E7 FF 84 C0")]
pub fn limit_radar_point(p: GPtr<f32>) -> X87 {
    limit_radar_point_with(menu_map(), p)
}

/// [`limit_radar_point`] with the menu flag as a parameter (L1).
pub fn limit_radar_point_with(map: bool, p: GPtr<f32>) -> X87 {
    let pb = p.cast::<u8>();
    let px = x(rd::<f32>(pb, 0));
    let py = x(rd::<f32>(pb, 4));
    let d = px.fmul(px).fadd(py.fmul(py)).fsqrt();
    if map || d.fcom(x(1.0)).test_any(0x41) {
        return d;
    }
    let s = x(1.0).fdiv(d);
    wr::<f32>(pb, 0, f32_of(s.fmul_m(rd::<f32>(pb, 0))));
    wr::<f32>(pb, 4, f32_of(s.fmul_m(rd::<f32>(pb, 4))));
    d
}

/// `CRadar::IsPointInsideRadar(const CVector2D&)` (0x584D40): -1 < x <= 1 and -1 < y <= 1 (NaN inside).
#[hook(addr = 0x584D40, conv = cdecl, name = "CRadar::IsPointInsideRadar", expect = "8B 4C 24 04 D9 01")]
pub fn is_point_inside_radar(p: GPtr<f32>) -> u8 {
    let pb = p.cast::<u8>();
    for o in [0, 4] {
        if !x(rd::<f32>(pb, o)).fcom(x(-1.0)).test_parity(5) {
            return 0;
        }
        if !x(rd::<f32>(pb, o)).fcom(x(1.0)).test_any(0x41) {
            return 0;
        }
    }
    1
}

/// `CRadar::ClipRadarTileCoords(int32&, int32&)` (0x584B00): clamps both to [0, 11], true if clamped.
#[hook(addr = 0x584B00, conv = cdecl, name = "CRadar::ClipRadarTileCoords", expect = "8B 54 24 04 8B 0A")]
pub fn clip_radar_tile_coords(px: GPtr<i32>, py: GPtr<i32>) -> u8 {
    let mut r = 0;
    for p in [px, py] {
        let v = p.read();
        if v < 0 {
            p.write(0);
            r = 1;
        } else if v > 11 {
            p.write(11);
            r = 1;
        }
    }
    r
}

/// `CRadar::TransformRealWorldPointToRadarSpace(CVector2D& out, const CVector2D& in)` (0x583530):
/// `d = (in - origin) / range` (x as f32, y kept in ST), then rotated by the cached sin/cos.
#[hook(addr = 0x583530, conv = cdecl, name = "CRadar::TransformRealWorldPointToRadarSpace", expect = "83 EC 08 8B 44 24 10")]
pub fn transform_real_world_point_to_radar_space(out: GPtr<f32>, inp: GPtr<f32>) {
    let o = RADAR_ORIGIN.get();
    transform_real_world_point_to_radar_space_with(
        out,
        inp,
        o,
        RADAR_RANGE.get(),
        CACHED_COS.get(),
        CACHED_SIN.get(),
    );
}

/// [`transform_real_world_point_to_radar_space`] with the globals as parameters (L1).
pub fn transform_real_world_point_to_radar_space_with(
    out: GPtr<f32>,
    inp: GPtr<f32>,
    origin: [f32; 2],
    range: f32,
    cos: f32,
    sin: f32,
) {
    let (ib, ob) = (inp.cast::<u8>(), out.cast::<u8>());
    let a = x(rd::<f32>(ib, 0)).fsub_m(origin[0]);
    let b = x(rd::<f32>(ib, 4)).fsub_m(origin[1]);
    let s = x(1.0).fdiv_m(range);
    let dx = f32_of(s.fmul(a));
    let dy = s.fmul(b);
    let rx = x(cos).fmul_m(dx);
    let ox = x(sin).fmul(dy).fadd(rx);
    wr::<f32>(ob, 0, f32_of(ox));
    let sy = x(sin).fmul_m(dx);
    let oy = x(cos).fmul(dy).fsub(sy);
    wr::<f32>(ob, 4, f32_of(oy));
}

/// `CRadar::TransformRadarPointToScreenSpace(CVector2D& out, const CVector2D& in)` (0x583480).
#[hook(addr = 0x583480, conv = cdecl, name = "CRadar::TransformRadarPointToScreenSpace", expect = "A0 A1 67 BA 00", preserve = "edx")]
pub fn transform_radar_point_to_screen_space(out: GPtr<f32>, inp: GPtr<f32>) {
    let mo = g::<[f32; 2]>(MENU_MAP_ORIGIN).read();
    transform_radar_point_to_screen_space_with(
        out,
        inp,
        menu_map(),
        g::<f32>(MENU_MAP_ZOOM).read(),
        mo,
        [g::<i32>(RS_MAX_WIDTH).read(), g::<i32>(RS_MAX_HEIGHT).read()],
    );
}

/// [`transform_radar_point_to_screen_space`] with the globals as parameters (L1).
pub fn transform_radar_point_to_screen_space_with(
    out: GPtr<f32>,
    inp: GPtr<f32>,
    map: bool,
    zoom: f32,
    map_origin: [f32; 2],
    screen: [i32; 2],
) {
    let (ib, ob) = (inp.cast::<u8>(), out.cast::<u8>());
    if map {
        wr::<f32>(
            ob,
            0,
            f32_of(x(zoom).fmul_m(rd::<f32>(ib, 0)).fadd_m(map_origin[0])),
        );
        wr::<f32>(
            ob,
            4,
            f32_of(x(zoom).fmul_m(rd::<f32>(ib, 4)).fsubr_m(map_origin[1])),
        );
        return;
    }
    let a = xi(screen[0]).fmul_m(f32::from_bits(0x3ACC_CCCD));
    let p = x(94.0).fmul(a);
    let q = x(0.5).fmul(p);
    let base = q.fadd(a.fmul_m(40.0));
    let r = p.fmul_m(rd::<f32>(ib, 0)).fmul_m(0.5);
    wr::<f32>(ob, 0, f32_of(base.fadd(r)));
    let h = xi(screen[1]);
    let b = x(f32::from_bits(0x3B12_4925)).fmul(h);
    let p2 = x(76.0).fmul(b);
    let t = h.fsub(b.fmul_m(104.0));
    let t = t.fadd(p2.fmul_m(0.5));
    let r2 = p2.fmul_m(rd::<f32>(ib, 4)).fmul_m(0.5);
    wr::<f32>(ob, 4, f32_of(t.fsub(r2)));
}

/// `CRadar::SetupRadarRect(int32 x, int32 y)` (0x584A80): the 3x3 tile window around (x, y), 500 units
/// a tile (wrapping int32 arithmetic, then `fild`).
#[hook(addr = 0x584A80, conv = cdecl, name = "CRadar::SetupRadarRect", expect = "8B 4C 24 04 8B 54 24 08")]
pub fn setup_radar_rect(tx: i32, ty: i32) {
    let c = tx.wrapping_sub(6);
    let r = 5i32.wrapping_sub(ty);
    RADAR_RECT.set([
        c.wrapping_mul(500).wrapping_sub(500) as f32,
        r.wrapping_add(2).wrapping_mul(500) as f32,
        c.wrapping_add(2).wrapping_mul(500) as f32,
        r.wrapping_mul(500).wrapping_sub(500) as f32,
    ]);
}

/// `CRadar::GetTextureCorners(int32 x, int32 y, CVector2D* corners)` (0x584D90): (ex, ey), (ex+500, ey),
/// (ex+500, ey+500), (ex, ey+500) with ex = (x-6)*500, ey = (5-y)*500.
#[hook(addr = 0x584D90, conv = cdecl, name = "CRadar::GetTextureCorners", expect = "8B 54 24 04 8B 44 24 08")]
pub fn get_texture_corners(tx: i32, ty: i32, out: GPtr<f32>) {
    let ex = tx.wrapping_sub(6).wrapping_mul(500);
    let ey = 5i32.wrapping_sub(ty).wrapping_mul(500);
    let (a, b) = (ex as f32, ey as f32);
    let (c, d) = (ex.wrapping_add(500) as f32, ey.wrapping_add(500) as f32);
    let o = out.cast::<u8>();
    for (k, v) in [a, b, c, b, c, d, a, d].into_iter().enumerate() {
        wr::<f32>(o, 4 * k as isize, v);
    }
}

/// `CRadar::SetCoordBlip(eBlipType, CVector, eBlipColour, eBlipDisplay, const char*)` (0x583820): the
/// colour argument and the script name are not used (colour 8).
#[hook(addr = 0x583820, conv = cdecl, name = "CRadar::SetCoordBlip", expect = "A0 15 87 BA 00")]
pub fn set_coord_blip(
    blip_type: i32,
    px: f32,
    py: f32,
    pz: f32,
    _colour: i32,
    display: i32,
    _name: GPtr<u8>,
) -> i32 {
    let i = free_slot();
    if i == -1 {
        return -1;
    }
    let t = tr(i);
    wr::<f32>(t, 8, px);
    wr::<f32>(t, 0xC, py);
    wr::<f32>(t, 0x10, pz);
    let b = rd::<u8>(t, 0x26);
    wr::<u8>(
        t,
        0x26,
        (((blip_type as u8) & 0xF) << 2) | (b & 0xC0) | (display as u8 & 3),
    );
    wr::<u32>(t, 0, 8);
    wr::<f32>(t, 0x18, 1.0);
    wr::<u32>(t, 4, 0);
    wr::<u16>(t, 0x1C, 1);
    wr::<u8>(t, 0x24, 0);
    wr::<u8>(t, 0x25, 3);
    wr::<u32>(t, 0x20, 0);
    new_unique_index(i)
}

/// `CRadar::SetShortRangeCoordBlip(...)` (0x583920): `SetCoordBlip` then the short-range bit; an
/// invalid handle sets bit 2 of the byte at `ms_RadarTrace[-1] + 0x25` (0xBA86ED), as the bytes.
#[hook(addr = 0x583920, conv = cdecl, name = "CRadar::SetShortRangeCoordBlip", expect = "8B 44 24 1C 8B 4C 24 18")]
pub fn set_short_range_coord_blip(
    blip_type: i32,
    px: f32,
    py: f32,
    pz: f32,
    colour: i32,
    display: i32,
    name: GPtr<u8>,
) -> i32 {
    let h = set_coord_blip_call(blip_type, px, py, pz, colour, display, name);
    if h == -1 {
        return h;
    }
    let t = tr(actual_index_in(traces(), h));
    wr::<u8>(t, 0x25, rd::<u8>(t, 0x25) | 4);
    h
}

/// `CRadar::SetEntityBlip(eBlipType, int32, eBlipColour, eBlipDisplay, const char*)` (0x5839A0): colour 7
/// for types 1 and 2, else 1 (the colour argument is not used).
#[hook(addr = 0x5839A0, conv = cdecl, name = "CRadar::SetEntityBlip", expect = "A0 15 87 BA 00")]
pub fn set_entity_blip(blip_type: i32, handle: i32, _colour: i32, display: i32, _name: GPtr<u8>) -> i32 {
    let i = free_slot();
    if i == -1 {
        return -1;
    }
    let t = tr(i);
    let b = rd::<u8>(t, 0x26);
    let b = b ^ ((((blip_type as u8) << 2) ^ b) & 0x3C);
    wr::<u8>(t, 0x26, b);
    wr::<u32>(t, 0, if blip_type == 2 || blip_type == 1 { 7 } else { 1 });
    wr::<i32>(t, 4, handle);
    wr::<f32>(t, 0x18, 1.0);
    wr::<u16>(t, 0x1C, 1);
    wr::<u8>(t, 0x26, ((b ^ display as u8) & 3) ^ b);
    wr::<u8>(t, 0x24, 0);
    wr::<u8>(t, 0x25, 3);
    wr::<u32>(t, 0x20, 0);
    new_unique_index(i)
}

/// `CRadar::Initialise()` (0x587FB0): every trace cleared with counter 1, range 350, and the 144 radar
/// TXD slots `radar%02d`.
#[hook(addr = 0x587FB0, conv = cdecl, name = "CRadar::Initialise", expect = "83 EC 10 33 D2")]
pub fn initialise() {
    AIRSTRIP_BLIP.set(0);
    AIRSTRIP_LOCATION.set(0);
    for i in 0..MAX_RADAR_TRACES {
        let t = tr(i);
        clear_entry(t);
        wr::<u16>(t, 0x14, 1);
    }
    RADAR_RANGE.set(350.0);
    let mut buf = [0u8; 16];
    let p = GPtr::from_raw(buf.as_mut_ptr());
    for i in 0..144 {
        crate::common::sprintf1(p, g::<u8>(RADAR_TXD_FMT), i);
        let slot = find_txd_slot(p);
        RADAR_TEXTURES.ptr().cast::<i32>().wrapping_index(i).write(slot);
    }
}

/// `CRadar::StreamRadarSections(int32 x, int32 y)` (0x584C50): requests the radar TXDs (model 20000 +
/// slot, flags 0xA) of the 3x3 tiles around (x, y), removes every other one; columns outer, rows inner.
#[hook(addr = 0x584C50, conv = cdecl, name = "CRadar::StreamRadarSections-xy", expect = "51 53 55 56 33 D2")]
pub fn stream_radar_sections_xy(tx: i32, ty: i32) {
    let tex = RADAR_TEXTURES.ptr().cast::<i32>();
    for c in 0..12i32 {
        for r in 0..12i32 {
            let slot = tex.wrapping_index(r * 12 + c).read();
            let near = c >= tx.wrapping_sub(1)
                && c <= tx.wrapping_add(1)
                && r >= ty.wrapping_sub(1)
                && r <= ty.wrapping_add(1);
            if slot == -1 {
                continue;
            }
            if near {
                request_model(slot.wrapping_add(20000), 0xA);
            } else {
                remove_model(slot.wrapping_add(20000));
            }
        }
    }
}

/// `CRadar::StreamRadarSections(const CVector&)` (0x5858D0): tile `ftol(floor((x + 3000) * 0.002))`,
/// `ftol(ceil(11 - (y + 3000) * 0.002))`, streamed unless streaming is disabled.
#[hook(addr = 0x5858D0, conv = cdecl, name = "CRadar::StreamRadarSections-vec", expect = "56 8B 74 24 08")]
pub fn stream_radar_sections_vec(pos: GPtr<f32>) {
    let pb = pos.cast::<u8>();
    let k = f32::from_bits(0x3B03_126F);
    let fx = x(rd::<f32>(pb, 0)).fadd_m(3000.0).fmul_m(k);
    let tx = ftol32(X87::from_f64_exact(crt_floor(fx.to_f64())));
    let fy = x(rd::<f32>(pb, 4)).fadd_m(3000.0).fmul_m(k).fsubr_m(11.0);
    let ty = ftol32(X87::from_f64_exact(crt_ceil(fy.to_f64())));
    if g::<u8>(DISABLE_STREAMING).read() == 0 {
        stream_radar_sections_xy_call(tx, ty);
    }
}

/// `DisplayThisBlip`'s jump table indices (0x583BE4) for sprites 0..=0x3F in the main area: 0 shown,
/// 1/2/3 legend filters (0x583B74 / 0x583B8C / 0x583BA4).
const DISPLAY_MAIN: [u8; 64] = [
    0, 0, 0, 0, 0, 1, 1, 1, 2, 1, 1, 1, 2, 2, 1, 2, 2, 1, 2, 3, 3, 2, 1, 2, 2, 2, 2, 1, 2, 1, 1, 3, 3, 1, 2,
    1, 1, 2, 2, 1, 2, 3, 2, 2, 2, 1, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 3, 3, 2, 2, 2, 2, 2, 1,
];
/// Indices (0x583C2C) for sprites 0..=0x34 in an interior: 0 shown, 1 hidden.
const DISPLAY_INTERIOR: [u8; 53] = [
    0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 0, 1, 1, 1, 1, 0, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 0,
];

fn legend_filter(flag: usize, prio: u8, want: u8) -> u8 {
    if g::<u8>(flag).read() != 0 && (prio as i8) < 0 {
        return 1;
    }
    u8::from(prio == want)
}

/// `CRadar::DisplayThisBlip(eRadarSprite, int8 priority)` (stub 0x583B40 → protection stub 0x408040
/// reading `CGame::currArea`).
#[hook(addr = 0x583B40, conv = cdecl, name = "CRadar::DisplayThisBlip", expect = "E9 FB 44 E8 FF 85 C0")]
pub fn display_this_blip(sprite: i32, prio: u8) -> u8 {
    let main = g::<i32>(CURR_AREA).read() == 0 && rd::<u8>(find_player_ped(-1), 0x2F) == 0;
    if main {
        let k = if (sprite as u32) > 0x3F {
            3
        } else {
            DISPLAY_MAIN[sprite as usize]
        };
        match k {
            0 => 1,
            1 => legend_filter(MENU_SHOW_A, prio, 1),
            2 => legend_filter(MENU_SHOW_B, prio, 3),
            _ => legend_filter(MENU_SHOW_C, prio, 2),
        }
    } else if (sprite as u32) > 0x34 {
        0
    } else {
        u8::from(DISPLAY_INTERIOR[sprite as usize] == 0)
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_GET_NEW_UNIQUE_BLIP_INDEX,
    HOOK_GET_ACTUAL_BLIP_ARRAY_INDEX,
    HOOK_CHANGE_BLIP_COLOUR,
    HOOK_SET_BLIP_SPRITE,
    HOOK_CHANGE_BLIP_DISPLAY,
    HOOK_CHANGE_BLIP_SCALE,
    HOOK_CHANGE_BLIP_BRIGHTNESS,
    HOOK_SET_BLIP_FRIENDLY,
    HOOK_SET_BLIP_ALWAYS_DISPLAY_IN_ZOOM,
    HOOK_SET_BLIP_FADE,
    HOOK_SET_COORD_BLIP_APPEARANCE,
    HOOK_SET_BLIP_ENTRY_EXIT,
    HOOK_CLEAR_ACTUAL_BLIP,
    HOOK_CLEAR_BLIP,
    HOOK_CLEAR_BLIP_FOR_ENTITY,
    HOOK_HAS_THIS_BLIP_BEEN_REVEALED,
    HOOK_CALCULATE_BLIP_ALPHA,
    HOOK_LIMIT_RADAR_POINT,
    HOOK_IS_POINT_INSIDE_RADAR,
    HOOK_CLIP_RADAR_TILE_COORDS,
    HOOK_TRANSFORM_REAL_WORLD_POINT_TO_RADAR_SPACE,
    HOOK_TRANSFORM_RADAR_POINT_TO_SCREEN_SPACE,
    HOOK_SETUP_RADAR_RECT,
    HOOK_GET_TEXTURE_CORNERS,
    HOOK_SET_COORD_BLIP,
    HOOK_SET_SHORT_RANGE_COORD_BLIP,
    HOOK_SET_ENTITY_BLIP,
    HOOK_INITIALISE,
    HOOK_STREAM_RADAR_SECTIONS_XY,
    HOOK_STREAM_RADAR_SECTIONS_VEC,
    HOOK_DISPLAY_THIS_BLIP,
];
