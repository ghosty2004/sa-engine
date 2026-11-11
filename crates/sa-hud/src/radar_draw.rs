//! `CRadar`, second pass (PORT-HUD-2): the radar orientation cache, the front-end map legend, the radar
//! section streaming helpers and the trace colours. Ported from the 1.0 US bytes; gta-reversed `Radar.cpp`
//! as reading aid. Sprite, streaming and colour calls stay `orig!` calls.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use sa_math::x87::X87;

use crate::common::{g, rd, wr, x};
use crate::radar::{AIRSTRIP_BLIP, AIRSTRIP_LOCATION, CACHED_COS, CACHED_SIN, RADAR_ORIENTATION, RADAR_TEXTURES, RADAR_TRACE};

sa_core::globals! {
    list GLOBALS;
    /// `CRadar::MapLegendList[175]` (`eRadarSprite`, i16)
    pub static MAP_LEGEND_LIST @ 0xBA8318: [i16; 175] = [0; 175];
    /// `CRadar::MapLegendCounter`
    pub static MAP_LEGEND_COUNTER @ 0xBA86B8: u16 = 0;
    /// `CRadar::ArrowBlipColour[6]` (`CRGBA`)
    pub static ARROW_BLIP_COLOUR @ 0xBA86D4: [u8; 24] = [0; 24];
}

// Foreign globals, addressed directly.
/// `FrontEndMenuManager.m_bDrawingMap` (+0x59)
const MENU_DRAWING_MAP: usize = 0xBA67A1;
/// `TheCamera`
const THE_CAMERA: usize = 0xB6F028;
/// `TheCamera.m_placement.m_fHeading` (+0x10) and `TheCamera.m_matrix` (+0x14)
const CAMERA_HEADING: usize = 0xB6F038;
const CAMERA_MATRIX: usize = 0xB6F03C;
/// `TheCamera.m_nActiveCam` (+0x59)
const CAMERA_ACTIVE_CAM: usize = 0xB6F081;
/// `TheCamera.m_aCams[i]` fields (stride 0x238): `Mode` (+0xC, u16), `Source` (+0x1A8),
/// `CamTargetEntity` (+0x21C)
const CAM_STRIDE: usize = 0x238;
const CAM_MODE: usize = 0xB6F1A8;
const CAM_SOURCE: usize = 0xB6F344;
const CAM_TARGET_ENTITY: usize = 0xB6F3B8;
/// `CRadar::RadarBlipSprites[64]` (`CSprite2d`)
const RADAR_BLIP_SPRITES: usize = 0xBAA250;
/// `HudColour`
const HUD_COLOUR: usize = 0xBAB22C;
/// `CRadar::vec2DRadarOrigin` (y at +4) and `CRadar::m_radarRange`
const RADAR_ORIGIN_X: usize = 0xBAA248;
const RADAR_ORIGIN_Y: usize = 0xBAA24C;
const RADAR_RANGE: usize = 0xBA8314;
/// First TXD model id (`RESOURCE_ID_TXD`)
const TXD_BASE: i32 = 20000;

orig! {
    /// `CCamera::GetLookDirection()` (0x50AE90)
    pub fn camera_get_look_direction(cam: GPtr<u8>) -> i32 = thiscall 0x50AE90 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    pub fn placeable_allocate_matrix(ent: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    pub fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CVector::Normalise()` (0x59C910)
    pub fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CRadar::CalculateCachedSinCos()` (0x583670)
    pub fn calculate_cached_sin_cos_call() = cdecl 0x583670 => calculate_cached_sin_cos;
    /// `CHudColours::GetIntColour(uint8)` (0x58FD50)
    pub fn get_int_colour_call(this: GPtr<u8>, colour: u32) -> u32 = thiscall 0x58FD50 => unported;
    /// `CRadar::GetRadarTraceColour(eBlipColour, bool, bool)` (0x584770)
    pub fn get_radar_trace_colour_call(colour: u32, bright: u32, friendly: u32) -> u32 = cdecl 0x584770 => get_radar_trace_colour;
    /// `CRGBA::CRGBA(uint8 r, uint8 g, uint8 b, uint8 a)` (0x7170C0)
    pub fn rgba_ctor(this: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `CStreaming::RequestModel(int32, int32)` (0x4087E0)
    pub fn request_model(id: i32, flags: i32) = cdecl 0x4087E0 => sa_stream::streaming::request_model;
    /// `CStreaming::RemoveModel(int32)` (0x4089A0)
    pub fn remove_model(id: i32) = cdecl 0x4089A0 => sa_stream::streaming::remove_model;
    /// `CStreaming::RemoveTxdModel(int32)` (0x40C180)
    pub fn remove_txd_model(slot: i32) = cdecl 0x40C180 => unported;
    /// `CSprite2d::Delete()` (0x727240)
    pub fn sprite2d_delete(this: GPtr<u8>) = thiscall 0x727240 => unported;
    /// `CRadar::RemoveRadarSections()` (0x584BF0)
    pub fn remove_radar_sections_call() = cdecl 0x584BF0 => remove_radar_sections;
}

fn trace(i: i32) -> GPtr<u8> {
    RADAR_TRACE.ptr().cast::<u8>().byte_add(i.wrapping_mul(0x28) as isize)
}

/// `CamTargetEntity`'s matrix forward vector of camera `i` (the matrix allocated and updated from the
/// placement first when missing), normalised by `CVector::Normalise` (raw dword copy, as the original).
fn target_forward(i: usize) -> [u32; 3] {
    let ent: GPtr<u8> = g::<GPtr<u8>>(CAM_TARGET_ENTITY + i * CAM_STRIDE).read();
    if rd::<u32>(ent, 0x14) == 0 {
        placeable_allocate_matrix(ent);
        simple_transform_update_matrix(ent.byte_add(4), rd(ent, 0x14));
    }
    let m: GPtr<u8> = rd(ent, 0x14);
    let mut v = [rd::<u32>(m, 0x10), rd::<u32>(m, 0x14), rd::<u32>(m, 0x18)];
    vector_normalise(GPtr::from_raw(v.as_mut_ptr()).cast());
    v
}

/// `CRadar::CalculateCachedSinCos()` (0x583670): on the front-end map sin = 0, cos = 1 (orientation
/// kept). Otherwise the orientation is `atan2(-v.x, v.y)` of: the camera matrix forward (or its heading
/// without a matrix) when `GetLookDirection() == 3`; else, for camera mode 16 or when the second
/// `GetLookDirection()` is 0, the target entity's normalised forward, otherwise target position minus the
/// camera source. sin / cos of the unrounded FPATAN result.
#[hook(addr = 0x583670, conv = cdecl, name = "CRadar::CalculateCachedSinCos", expect = "A0 A1 67 BA 00")]
pub fn calculate_cached_sin_cos() {
    if g::<u8>(MENU_DRAWING_MAP).read() != 0 {
        CACHED_SIN.set(0.0);
        CACHED_COS.set(1.0);
        return;
    }
    let cam = g::<u8>(THE_CAMERA);
    let angle = if camera_get_look_direction(cam) == 3 {
        let m: GPtr<u8> = g::<GPtr<u8>>(CAMERA_MATRIX).read();
        if !m.is_null() {
            x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14)))
        } else {
            x(g::<f32>(CAMERA_HEADING).read())
        }
    } else {
        let i = usize::from(g::<u8>(CAMERA_ACTIVE_CAM).read());
        let mut v = if g::<u16>(CAM_MODE + i * CAM_STRIDE).read() == 0x10 {
            target_forward(i)
        } else {
            let ent: GPtr<u8> = g::<GPtr<u8>>(CAM_TARGET_ENTITY + i * CAM_STRIDE).read();
            let m: GPtr<u8> = rd(ent, 0x14);
            let pos = if m.is_null() { ent.byte_add(4) } else { m.byte_add(0x30) };
            let src = CAM_SOURCE + i * CAM_STRIDE;
            let d = |k: usize| {
                x(rd::<f32>(pos, (4 * k) as isize))
                    .fsub_m(g::<f32>(src + 4 * k).read())
                    .to_f32()
                    .to_bits()
            };
            [d(0), d(1), d(2)]
        };
        if camera_get_look_direction(cam) == 0 {
            let i = usize::from(g::<u8>(CAMERA_ACTIVE_CAM).read());
            v = target_forward(i);
        }
        x(f32::from_bits(v[0])).fchs().fpatan(x(f32::from_bits(v[1])))
    };
    CACHED_SIN.set(angle.fsin().to_f32());
    CACHED_COS.set(angle.fcos().to_f32());
    RADAR_ORIENTATION.set(angle.to_f32());
}

/// `CRadar::GetRadarTraceColour(eBlipColour, bool bright, bool friendly)` (0x584770): the HUD colour of a
/// blip colour (red 0 / 5: bright 0 else 9; green 1: 1 / 10; blue 2 / 6: 3 / 13; white 3: 4 / 8;
/// yellow 4: 11 / 6; threat 7: friendly 13 else 0; destination 8: 11) through `GetIntColour`; any other
/// value is returned unchanged.
#[hook(addr = 0x584770, conv = cdecl, name = "CRadar::GetRadarTraceColour", expect = "8B 44 24 04 83 F8 08")]
pub fn get_radar_trace_colour(colour: u32, bright: u32, friendly: u32) -> u32 {
    let (bright, friendly) = (bright as u8 != 0, friendly as u8 != 0);
    let pick = |a: u32, b: u32| if bright { a } else { b };
    let id = match colour {
        0 | 5 => pick(0, 9),
        1 => pick(1, 0xA),
        2 | 6 => pick(3, 0xD),
        3 => pick(4, 8),
        4 => pick(0xB, 6),
        7 => {
            if friendly {
                0xD
            } else {
                0
            }
        }
        8 => 0xB,
        _ => return colour,
    };
    get_int_colour_call(g(HUD_COLOUR), id)
}

/// `CRadar::AddBlipToLegendList(bool noSprite, int32 blipIndex)` (0x5859F0; the entry jumps to a 0x401B31
/// chunk that reads `m_bDrawingMap`): on the front-end map, the sprite (the blip index's low byte, or for
/// `noSprite` the arrow sprite of the trace's type: car / char -4 (+1 when friendly), object -2, coord -1,
/// else -5) is appended to `MapLegendList` unless already listed (all 175 entries are scanned; the u16
/// counter is not bounded). A new arrow sprite also stores the trace colour in `ArrowBlipColour[-sprite]`
/// (alpha 255).
#[hook(addr = 0x5859F0, conv = cdecl, name = "CRadar::AddBlipToLegendList", expect = "51 E9 3B C1 E7 FF")]
pub fn add_blip_to_legend_list(no_sprite: u32, index: i32) {
    if g::<u8>(MENU_DRAWING_MAP).read() == 0 {
        return;
    }
    let no_sprite = no_sprite as u8 != 0;
    let t = trace(index);
    let mut sprite = index as u8;
    if no_sprite {
        sprite = match (rd::<u8>(t, 0x26) >> 2) & 0xF {
            1 | 2 => ((rd::<u8>(t, 0x25) >> 3) & 1) | 0xFC,
            3 => 0xFE,
            4 => 0xFF,
            _ => 0xFB,
        };
    }
    let s = i16::from(sprite as i8);
    let list = MAP_LEGEND_LIST.ptr().cast::<i16>();
    let mut found = false;
    for k in 0..175 {
        if list.byte_add(2 * k).read() == s {
            found = true;
        }
    }
    if found {
        return;
    }
    let n = MAP_LEGEND_COUNTER.get();
    MAP_LEGEND_COUNTER.set(n.wrapping_add(1));
    list.byte_add(2 * n as isize).write(s);
    if no_sprite {
        let b = rd::<u8>(t, 0x25);
        let c = get_radar_trace_colour_call(rd::<u32>(t, 0), u32::from(b & 1), u32::from((b >> 3) & 1));
        let mut tmp = [0u8; 4];
        let p = rgba_ctor(GPtr::from_raw(tmp.as_mut_ptr()), c >> 24, c >> 16, c >> 8, 0xFF);
        let dst = ARROW_BLIP_COLOUR
            .ptr()
            .cast::<u8>()
            .byte_add(-(isize::from(sprite as i8) * 4));
        for k in 0..4 {
            dst.byte_add(k).write(p.byte_add(k).read());
        }
    }
}

/// `CRadar::InitFrontEndMap()` (0x585960): `CalculateCachedSinCos`, clears `MapLegendList`, origin
/// (0, 0), range 2990, counter 0, and the six `ArrowBlipColour`s to `CRGBA(0, 0, 0, 0)`.
#[hook(addr = 0x585960, conv = cdecl, name = "CRadar::InitFrontEndMap", expect = "51 56 57 E8 08 DD FF FF")]
pub fn init_front_end_map() {
    calculate_cached_sin_cos_call();
    MAP_LEGEND_LIST.set([0; 175]);
    g::<u32>(RADAR_ORIGIN_Y).write(0);
    g::<u32>(RADAR_ORIGIN_X).write(0);
    g::<u32>(RADAR_RANGE).write(0x453A_E000);
    MAP_LEGEND_COUNTER.set(0);
    let dst = ARROW_BLIP_COLOUR.ptr().cast::<u8>();
    for i in 0..6 {
        let mut tmp = [0u8; 4];
        let p = rgba_ctor(GPtr::from_raw(tmp.as_mut_ptr()), 0, 0, 0, 0);
        for k in 0..4 {
            dst.byte_add(i * 4 + k).write(p.byte_add(k).read());
        }
    }
}

fn texture(x: i32, y: i32) -> i32 {
    RADAR_TEXTURES.ptr().cast::<i32>().byte_add(((y * 12 + x) * 4) as isize).read()
}

/// `CRadar::RequestMapSection(int32 x, int32 y)` (0x584B50): x is clamped to 0..11 first, and nothing
/// happens when it had to be clamped or y is outside 0..11; a section with a texture requests TXD model
/// `slot + 20000` with flags 10 (the original tail-jumps, overwriting its two argument slots with them).
#[hook(addr = 0x584B50, conv = cdecl, name = "CRadar::RequestMapSection", expect = "8B 44 24 04 32 D2")]
pub fn request_map_section(x: i32, y: i32) {
    if !(0..=11).contains(&x) || !(0..=11).contains(&y) {
        return;
    }
    let t = texture(x, y);
    if t != -1 {
        request_model(t.wrapping_add(TXD_BASE), 10);
    }
}

/// `CRadar::RemoveMapSection(int32 x, int32 y)` (0x584BB0): `CStreaming::RemoveTxdModel` of the section's
/// TXD slot when both coordinates are in 0..11 and it has one.
#[hook(addr = 0x584BB0, conv = cdecl, name = "CRadar::RemoveMapSection", expect = "8B 4C 24 04 85 C9")]
pub fn remove_map_section(x: i32, y: i32) {
    if !(0..=11).contains(&x) || !(0..=11).contains(&y) {
        return;
    }
    let t = texture(x, y);
    if t != -1 {
        remove_txd_model(t);
    }
}

/// `CRadar::RemoveRadarSections()` (0x584BF0; the entry jumps to a 0x401C33 chunk that loads the table
/// address): `CStreaming::RemoveModel(slot + 20000)` for every section with a texture, column by column
/// (x outer, y inner).
#[hook(addr = 0x584BF0, conv = cdecl, name = "CRadar::RemoveRadarSections", expect = "53 56 57 E9 3B D0 E7 FF")]
pub fn remove_radar_sections() {
    for xx in 0..12 {
        for yy in 0..12 {
            let t = texture(xx, yy);
            if t != -1 {
                remove_model(t.wrapping_add(TXD_BASE));
            }
        }
    }
}

/// `CRadar::Shutdown()` (0x585940): `CSprite2d::Delete` on the 64 `RadarBlipSprites`, then
/// `RemoveRadarSections` (tail jump).
#[hook(addr = 0x585940, conv = cdecl, name = "CRadar::Shutdown", expect = "56 BE 50 A2 BA 00")]
pub fn shutdown() {
    for i in 0..64 {
        sprite2d_delete(g(RADAR_BLIP_SPRITES + 4 * i));
    }
    remove_radar_sections_call();
}

/// One edge test of `LineRadarBoxCollision`: `a`, `b` = signed distances of start and end from the edge.
/// Returns `(t, v)` as stored f32s when the segment crosses the edge (`a * b < 0`) and the crossing's other
/// coordinate `v = (e - s) * t32 + s` is within -1..1 (`v` compared unrounded against -1, the stored f32
/// against 1); `t = a / (a - b)`.
fn edge_cross(a: X87, b: X87, s: f32, e: f32) -> Option<(f32, f32)> {
    if b.fmul(a).fcom(X87::zero()).test_parity(0x05) {
        return None;
    }
    let t = a.fdiv(a.fsub(b));
    let t32 = t.to_f32();
    let v = x(e).fsub_m(s).fmul_m(t32).fadd_m(s);
    let v32 = v.to_f32();
    if v.fcom(x(-1.0)).c0() {
        return None;
    }
    if x(v32).fcom(x(1.0)).test_parity(0x41) {
        return None;
    }
    Some((t32, v32))
}

/// `LineRadarBoxCollision(CVector2D& out, const CVector2D& start, const CVector2D& end)` (0x584E00): the
/// edge of the radar square (-1..1) the segment crosses first. Edges in order: x = -1 (side 3), x = 1
/// (side 1), y = -1 (side 0), y = 1 (side 2, returned at once). A crossing replaces the previous one when
/// its f32 parameter is <= the best so far (the first edge compares against the constant 1.0, the last
/// one its unrounded parameter); no early return at t = 0 (gta-reversed differs). Returns the side, or -1.
/// The original also stores its temporaries (t, v) in the caller's start / end argument slots; its only
/// caller, `ClipRadarPoly`, does not read them back.
#[hook(addr = 0x584E00, conv = cdecl, name = "CRadar::LineRadarBoxCollision", expect = "D9 05 24 86 85 00")]
pub fn line_radar_box_collision(out: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>) -> i32 {
    let (sx, sy) = (rd::<f32>(start, 0), rd::<f32>(start, 4));
    let (ex, ey) = (rd::<f32>(end, 0), rd::<f32>(end, 4));
    let m1 = || x(-1.0);
    let mut best = x(1.0);
    let mut side = -1;
    let put = |xb: u32, yb: u32| {
        wr::<u32>(out, 0, xb);
        wr::<u32>(out, 4, yb);
    };
    // x = -1
    if let Some((t32, v32)) = edge_cross(m1().fsub_m(sx), m1().fsub_m(ex), sy, ey)
        && !x(t32).fcom(x(1.0)).test_parity(0x41)
    {
        best = x(t32);
        put(0xBF80_0000, v32.to_bits());
        side = 3;
    }
    // x = 1
    if let Some((t32, v32)) = edge_cross(x(sx).fsub_m(1.0), x(ex).fsub_m(1.0), sy, ey)
        && !x(t32).fcom(best).test_parity(0x41)
    {
        best = x(t32);
        put(0x3F80_0000, v32.to_bits());
        side = 1;
    }
    // y = -1
    if let Some((t32, v32)) = edge_cross(m1().fsub_m(sy), m1().fsub_m(ey), sx, ex)
        && !x(t32).fcom(best).test_parity(0x41)
    {
        best = x(t32);
        put(v32.to_bits(), 0xBF80_0000);
        side = 0;
    }
    // y = 1: the parameter stays in the register (not rounded to f32 before the product and the compare)
    let (a, b) = (x(sy).fsub_m(1.0), x(ey).fsub_m(1.0));
    if b.fmul(a).fcom(X87::zero()).test_parity(0x05) {
        return side;
    }
    let t = a.fdiv(a.fsub(b));
    let v = x(ex).fsub_m(sx).fmul(t).fadd_m(sx);
    let v32 = v.to_f32();
    if v.fcom(x(-1.0)).c0() || x(v32).fcom(x(1.0)).test_parity(0x41) {
        return side;
    }
    if t.fcom(best).test_parity(0x41) {
        return side;
    }
    put(v32.to_bits(), 0x3F80_0000);
    2
}

orig! {
    /// `LineRadarBoxCollision(CVector2D&, const CVector2D&, const CVector2D&)` (0x584E00)
    pub fn line_radar_box_collision_call(out: GPtr<u8>, start: GPtr<u8>, end: GPtr<u8>) -> i32 = cdecl 0x584E00 => line_radar_box_collision;
}

/// `CRadar::ClipRadarPoly(CVector2D* out, const CVector2D* in)` (0x585040): clips the quad `in[4]` to the
/// radar square. Vertices inside (NaN counts as inside) are copied; for an outside vertex the crossings
/// towards the previous (`s1`) and the next (`s2`) vertex are appended (`LineRadarBoxCollision`). When
/// only `s2` exists, the square's corners from the last known side (`s1` of an earlier vertex, else a
/// backwards search over the quad's edges i..3 into a stack temporary) up to `s2` are inserted before the
/// `s2` point. With no output vertex the square itself is returned when both line tests say it lies inside
/// the quad. Returns the vertex count.
///
/// Original quirk kept: with no known side (-1) the corner walk starts at `corners[-1]`, which in the
/// original is two stack locals: `&inside[-1]` and the `s2` output pointer. The first is an address in
/// the original's frame (`push ebp; mov ebp, esp; and esp, -8; sub esp, 0x4C`, three pushes, `inside` at
/// `[esp+0x14]`): `((entry ESP - 4) & !7) - 0x45`, which the port computes from its entry ESP (`peek`,
/// PORT-HUD-3); entered without its hook (a direct Rust call) it uses its own `&inside[-1]`.
#[hook(addr = 0x585040, conv = cdecl, name = "CRadar::ClipRadarPoly", expect = "55 8B EC 83 E4 F8", peek = 4)]
pub fn clip_radar_poly(out: GPtr<u8>, inp: GPtr<u8>) -> i32 {
    let entry = sa_core::abi::peek_entry_esp(0x585040);
    clip_radar_poly_with(out, inp, move |inside_m1, slot| {
        (entry.map_or(inside_m1, clip_radar_poly_inside_m1), slot.addr() as u32)
    })
}

/// The original `ClipRadarPoly`'s `&inside[-1]` for an entry ESP (see [`clip_radar_poly`]).
pub fn clip_radar_poly_inside_m1(entry_esp: u32) -> u32 {
    (entry_esp.wrapping_sub(4) & !7).wrapping_sub(0x45)
}

/// [`clip_radar_poly`] with the `corners[-1]` words supplied by `quirk(&inside[-1], s2 slot)` (the L1
/// spec passes the emulator's stack address and the slot's emulated address).
pub fn clip_radar_poly_with(out: GPtr<u8>, inp: GPtr<u8>, quirk: impl Fn(u32, GPtr<u8>) -> (u32, u32)) -> i32 {
    const CORNERS: [(u32, u32); 4] = [
        (0x3F80_0000, 0xBF80_0000),
        (0x3F80_0000, 0x3F80_0000),
        (0xBF80_0000, 0x3F80_0000),
        (0xBF80_0000, 0xBF80_0000),
    ];
    let v = |k: i32| inp.byte_add(k as isize * 8);
    let o = |k: i32| out.byte_add(k as isize * 8);
    let mut inside = [0u8; 4];
    for (k, f) in inside.iter_mut().enumerate() {
        let p = v(k as i32);
        let (px, py) = (x(rd::<f32>(p, 0)), x(rd::<f32>(p, 4)));
        *f = u8::from(
            px.fcom(x(-1.0)).test_parity(0x05)
                && px.fcom(x(1.0)).test_any(0x41)
                && py.fcom(x(-1.0)).test_parity(0x05)
                && py.fcom(x(1.0)).test_any(0x41),
        );
    }
    let inside_m1 = GPtr::<u8>::from_raw(inside.as_mut_ptr()).byte_add(-1).addr() as u32;
    let mut count = 0i32;
    let mut prev_side = -1i32;
    for i in 0..4 {
        if inside[i as usize] != 0 {
            wr::<u32>(o(count), 0, rd::<u32>(v(i), 0));
            wr::<u32>(o(count), 4, rd::<u32>(v(i), 4));
            count += 1;
            continue;
        }
        let s1 = line_radar_box_collision_call(o(count), v(i), v((i - 1) & 3));
        if s1 != -1 {
            count += 1;
            prev_side = s1;
        }
        let slot = o(count);
        let s2 = line_radar_box_collision_call(slot, v(i), v((i + 1) & 3));
        if s2 == -1 {
            continue;
        }
        if s1 == -1 {
            if prev_side == -1 {
                let mut tmp = [0u32; 2];
                let t = GPtr::<u32>::from_raw(tmp.as_mut_ptr()).cast::<u8>();
                let mut k = 3;
                loop {
                    let r = if k == 0 {
                        line_radar_box_collision_call(t, v(0), v(3))
                    } else {
                        line_radar_box_collision_call(t, v(k), v(k - 1))
                    };
                    if r != -1 {
                        prev_side = r;
                        break;
                    }
                    k -= 1;
                    if k < i {
                        break;
                    }
                }
            }
            // held in x87 registers across the corner writes (`fld m32` quiets a signalling NaN)
            let (px, py) = (x(rd::<f32>(slot, 0)).to_f32().to_bits(), x(rd::<f32>(slot, 4)).to_f32().to_bits());
            let mut c = prev_side;
            let mut dst = slot;
            if c != s2 {
                loop {
                    count += 1;
                    let (cx, cy) = if c == -1 { quirk(inside_m1, slot) } else { CORNERS[c as usize] };
                    wr::<u32>(dst, 0, cx);
                    wr::<u32>(dst, 4, cy);
                    dst = dst.byte_add(8);
                    c = (c + 1) & 3;
                    if c == s2 {
                        break;
                    }
                }
            }
            wr::<u32>(o(count), 0, px);
            wr::<u32>(o(count), 4, py);
        }
        count += 1;
    }
    if count != 0 {
        return count;
    }
    let f = |k: i32, c: isize| rd::<f32>(v(k), c);
    let k1 = x(f(0, 4)).fsub_m(f(1, 4)).fdiv(x(f(0, 0)).fsub_m(f(1, 0)));
    let k2 = x(f(0, 4)).fsub_m(f(3, 4)).fdiv(x(f(0, 0)).fsub_m(f(3, 0))).to_f32();
    let a = k1.fmul_m(f(3, 0)).fsub_m(f(3, 4));
    let b = k1.fmul_m(f(0, 0)).fsub_m(f(0, 4));
    if a.fmul(b).fcom(X87::zero()).test_parity(0x05) {
        return 0;
    }
    let c = x(k2).fmul_m(f(1, 0)).fsub_m(f(1, 4));
    let d = x(k2).fmul_m(f(0, 0)).fsub_m(f(0, 4));
    if c.fmul(d).fcom(X87::zero()).test_parity(0x05) {
        return 0;
    }
    for (k, (cx, cy)) in CORNERS.iter().enumerate() {
        wr::<u32>(o(k as i32), 0, *cx);
        wr::<u32>(o(k as i32), 4, *cy);
    }
    4
}

orig! {
    /// `CTxdStore::PushCurrentTxd()` (0x7316A0)
    pub fn push_current_txd() = cdecl 0x7316A0 => sa_stream::txdstore::push_current_txd;
    /// `CTxdStore::FindTxdSlot(const char*)` (0x731850)
    pub fn find_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731850 => sa_stream::txdstore::find_txd_slot_name;
    /// `CTxdStore::SetCurrentTxd(int32)` (0x7319C0)
    pub fn set_current_txd(slot: i32) = cdecl 0x7319C0 => sa_stream::txdstore::set_current_txd;
    /// `CTxdStore::PopCurrentTxd()` (0x7316B0)
    pub fn pop_current_txd() = cdecl 0x7316B0 => sa_stream::txdstore::pop_current_txd;
    /// `CSprite2d::SetTexture(char*)` (0x727270)
    pub fn sprite2d_set_texture(this: GPtr<u8>, name: GPtr<u8>) = thiscall 0x727270 => unported;
}

/// `"hud"` (.rdata)
const HUD_TXD_NAME: usize = 0x86696C;
/// `RadarBlipSpriteFilenames[64]` (`{ const char* name; const char* mask; }`)
const RADAR_BLIP_SPRITE_FILENAMES: usize = 0x8D0720;
/// `FrontEndMenuManager.m_bMapLoaded` (+0x78), `m_fMapZoom` (+0x64), `m_vMapOrigin` (+0x68)
const MENU_MAP_LOADED: usize = 0xBA67C0;
const MENU_MAP_ZOOM: usize = 0xBA67AC;
const MENU_MAP_ORIGIN_X: usize = 0xBA67B0;
const MENU_MAP_ORIGIN_Y: usize = 0xBA67B4;
/// `RsGlobal.maximumWidth` / `maximumHeight`
const RS_MAX_WIDTH: usize = 0xC17044;
const RS_MAX_HEIGHT: usize = 0xC17048;
/// 1/640 and 1/448 (.rdata 0x859520 / 0x859524)
const INV_640: f32 = f32::from_bits(0x3ACC_CCCD);
const INV_448: f32 = f32::from_bits(0x3B12_4925);

/// `CRadar::LoadTextures()` (0x5827D0): with the "hud" TXD current, `RadarBlipSprites[i].SetTexture` of
/// every sprite file name, then `PopCurrentTxd` (tail jump).
#[hook(addr = 0x5827D0, conv = cdecl, name = "CRadar::LoadTextures", expect = "56 57 E8 C9 EE 1A 00")]
pub fn load_textures() {
    push_current_txd();
    set_current_txd(find_txd_slot(g(HUD_TXD_NAME)));
    for i in 0..64 {
        let name: GPtr<u8> = g::<GPtr<u8>>(RADAR_BLIP_SPRITE_FILENAMES + 8 * i).read();
        sprite2d_set_texture(g(RADAR_BLIP_SPRITES + 4 * i), name);
    }
    pop_current_txd();
}

/// `CRadar::LimitToMap(float* x, float* y)` (0x583350): clamps the point to the map square around the
/// map origin, half size `zoom` = `(int16) m_fMapZoom` (140 when the map is not loaded), scaled to the
/// screen by `width / 640` and `height / 448`; each bound is stored only when strictly beyond it
/// (a NaN coordinate is left alone).
#[hook(addr = 0x583350, conv = cdecl, name = "CRadar::LimitToMap", expect = "51 A0 C0 67 BA 00")]
pub fn limit_to_map(px: GPtr<f32>, py: GPtr<f32>) {
    limit_to_map_with(
        px,
        py,
        g::<u8>(MENU_MAP_LOADED).read() != 0,
        g::<f32>(MENU_MAP_ZOOM).read(),
        [g::<f32>(MENU_MAP_ORIGIN_X).read(), g::<f32>(MENU_MAP_ORIGIN_Y).read()],
        [g::<i32>(RS_MAX_WIDTH).read(), g::<i32>(RS_MAX_HEIGHT).read()],
    );
}

/// [`limit_to_map`] with the menu state and the screen size as parameters (L1).
pub fn limit_to_map_with(px: GPtr<f32>, py: GPtr<f32>, loaded: bool, zoom: f32, origin: [f32; 2], screen: [i32; 2]) {
    let zoom: i32 = if !loaded { 0x8C } else { i32::from(x(zoom).ftol() as u16 as i16) };
    let z = X87::from_i32(zoom);
    let sx = || X87::from_i32(screen[0]).fmul_m(INV_640);
    let sy = || X87::from_i32(screen[1]).fmul_m(INV_448);
    let [ox, oy] = origin;
    // min: store when min > v (neither C0 nor C3); max: store when max < v (C0 alone)
    let lo = x(ox).fsub(z).fmul(sx());
    if !lo.fcom(x(px.read())).test_any(0x41) {
        px.write(lo.to_f32());
    }
    let hi = sx().fmul(z.fadd_m(ox));
    if !hi.fcom(x(px.read())).test_parity(0x05) {
        px.write(hi.to_f32());
    }
    let lo = sy().fmul(x(oy).fsub(z));
    if !lo.fcom(x(py.read())).test_any(0x41) {
        py.write(lo.to_f32());
    }
    let hi = sy().fmul(z.fadd_m(oy));
    if !hi.fcom(x(py.read())).test_parity(0x05) {
        py.write(hi.to_f32());
    }
}

/// `CRadar::TransformRadarPointToRealWorldSpace(CVector2D& out, const CVector2D& in)` (0x5835A0): rotates
/// counter-clockwise by the cached sin / cos, scales by `m_radarRange` and adds `vec2DRadarOrigin`, in the
/// original's store order (`out.x` is written before `in` is read again, so `out == in` behaves as the
/// original; `out.y`'s scaled value stays in the register for the final add).
#[hook(addr = 0x5835A0, conv = cdecl, name = "CRadar::TransformRadarPointToRealWorldSpace", expect = "8B 4C 24 08 D9 05 08 83 BA 00")]
pub fn transform_radar_point_to_real_world_space(out: GPtr<f32>, inp: GPtr<f32>) {
    transform_radar_point_to_real_world_space_with(
        out,
        inp,
        CACHED_COS.get(),
        CACHED_SIN.get(),
        g::<f32>(RADAR_RANGE).read(),
        [g::<f32>(RADAR_ORIGIN_X).read(), g::<f32>(RADAR_ORIGIN_Y).read()],
    );
}

/// [`transform_radar_point_to_real_world_space`] with the cached cos / sin, range and origin as
/// parameters (L1).
pub fn transform_radar_point_to_real_world_space_with(out: GPtr<f32>, inp: GPtr<f32>, c: f32, s: f32, r: f32, origin: [f32; 2]) {
    let ox = out;
    let oy = out.byte_add(4);
    let (ix, iy) = (inp, inp.byte_add(4));
    let a = x(c).fmul_m(ix.read());
    let b = x(s).fmul_m(iy.read());
    ox.write(a.fsub(b).to_f32());
    let a = x(c).fmul_m(iy.read());
    let b = x(s).fmul_m(ix.read());
    oy.write(a.fadd(b).to_f32());
    ox.write(x(r).fmul_m(ox.read()).to_f32());
    let ry = x(r).fmul_m(oy.read());
    oy.write(ry.to_f32());
    ox.write(x(origin[0]).fadd_m(ox.read()).to_f32());
    oy.write(ry.fadd_m(origin[1]).to_f32());
}

orig! {
    /// `CRadar::LimitToMap(float*, float*)` (0x583350)
    pub fn limit_to_map_call(px: GPtr<f32>, py: GPtr<f32>) = cdecl 0x583350 => limit_to_map;
    /// `RwRenderStateSet(state, value)` (`RwEngineInstance` table slot 0x20)
    pub fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `CSprite2d::DrawRect(const CRect&, const CRGBA&)` (0x727B60)
    pub fn sprite2d_draw_rect(rect: GPtr<f32>, colour: GPtr<u8>) = cdecl 0x727B60 => unported;
    /// `CSprite2d::Draw(x1, y1, x2, y2, x3, y3, x4, y4, const CRGBA&)` (0x728520)
    pub fn sprite2d_draw_quad(this: GPtr<u8>, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, x4: f32, y4: f32, colour: GPtr<u8>) = thiscall 0x728520 => unported;
}

/// 2^32 (.rdata 0x858C54): `fild` of an unsigned argument adds it when the sign bit is set.
const TWO_POW_32: f32 = f32::from_bits(0x4F80_0000);
/// pi/4 (0x859AB0) and pi/2 (0x858FE4)
const FRAC_PI_4: f32 = f32::from_bits(0x3F49_0FDB);
const FRAC_PI_2: f32 = f32::from_bits(0x3FC9_0FDB);

/// `fild dword` of an unsigned value, as MSVC converts it.
fn fild_u32(v: u32) -> X87 {
    let r = X87::from_i32(v as i32);
    if (v as i32) < 0 { r.fadd_m(TWO_POW_32) } else { r }
}

/// On the front-end map the point is scaled to the screen (`* width / 640`, `* height / 448`) and
/// limited by `LimitToMap` (in the original both are done on the argument slots).
fn map_point(px: &mut f32, py: &mut f32) {
    if g::<u8>(MENU_DRAWING_MAP).read() == 0 {
        return;
    }
    *px = X87::from_i32(g::<i32>(RS_MAX_WIDTH).read()).fmul_m(INV_640).fmul_m(*px).to_f32();
    *py = X87::from_i32(g::<i32>(RS_MAX_HEIGHT).read()).fmul_m(INV_448).fmul_m(*py).to_f32();
    limit_to_map_call(GPtr::from_raw(px as *mut f32), GPtr::from_raw(py as *mut f32));
}

/// `CRadar::ShowRadarTrace(float x, float y, uint32 size, uint8 r, uint8 g, uint8 b, uint8 a)`
/// (0x583F40): on the map the point goes through [`map_point`]; texture raster off
/// (`RwRenderStateSet(1, 0)`), then a black rectangle one unit larger than `size` (alpha `a`) and the
/// coloured one, both `CSprite2d::DrawRect` (rect = left, bottom, right, top).
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x583F40, conv = cdecl, name = "CRadar::ShowRadarTrace", expect = "A0 A1 67 BA 00")]
pub fn show_radar_trace(px: f32, py: f32, size: u32, r: u32, gg: u32, b: u32, a: u32) {
    let (mut px, mut py) = (px, py);
    map_point(&mut px, &mut py);
    rw_render_state_set(1, 0);
    let s = fild_u32(size);
    let xs = x(px).fsub(s);
    let ys = x(py).fsub(s);
    let xp = x(px).fadd(s);
    let yp = s.fadd_m(py);
    let mut outer = [
        xs.fsub_m(1.0).to_f32(),
        yp.fadd_m(1.0).to_f32(),
        xp.fadd_m(1.0).to_f32(),
        ys.fsub_m(1.0).to_f32(),
    ];
    let mut inner = [xs.to_f32(), yp.to_f32(), xp.to_f32(), ys.to_f32()];
    let mut c = [0u8; 4];
    let cp = rgba_ctor(GPtr::from_raw(c.as_mut_ptr()), 0, 0, 0, a);
    sprite2d_draw_rect(GPtr::from_raw(outer.as_mut_ptr()), cp);
    let cp = rgba_ctor(GPtr::from_raw(c.as_mut_ptr()), r, gg, b, a);
    sprite2d_draw_rect(GPtr::from_raw(inner.as_mut_ptr()), cp);
}

/// `CRadar::DrawRotatingRadarSprite(CSprite2d*, float x, float y, float angle, uint32 width,
/// uint32 height, CRGBA colour)` (0x584850): on the map the point goes through [`map_point`]; corner `i`
/// (0..3) is at `theta = i * pi/2 + (angle - pi/4)`: `x + (0 * cos + sin) * width`,
/// `y + (cos - sin * 0) * height` (sin / cos of the unrounded register value), drawn as
/// `Draw(v3, v2, v0, v1, colour)`.
#[allow(clippy::too_many_arguments)]
#[hook(addr = 0x584850, conv = cdecl, name = "CRadar::DrawRotatingRadarSprite", expect = "A0 A1 67 BA 00")]
pub fn draw_rotating_radar_sprite(sprite: GPtr<u8>, px: f32, py: f32, angle: f32, width: u32, height: u32, colour: u32) {
    let (mut px, mut py) = (px, py);
    map_point(&mut px, &mut py);
    let a = x(angle).fsub_m(FRAC_PI_4);
    let w = fild_u32(width);
    let h = fild_u32(height);
    let mut v = [[0f32; 2]; 4];
    for (i, p) in v.iter_mut().enumerate() {
        let theta = X87::from_i32(i as i32).fmul_m(FRAC_PI_2).fadd(a);
        let (s, c) = (theta.fsin(), theta.fcos());
        p[0] = X87::zero().fmul(c).fadd(s).fmul(w).fadd_m(px).to_f32();
        p[1] = c.fsub(s.fmul_m(0.0)).fmul(h).fadd_m(py).to_f32();
    }
    let mut col = colour;
    sprite2d_draw_quad(
        sprite,
        v[3][0],
        v[3][1],
        v[2][0],
        v[2][1],
        v[0][0],
        v[0][1],
        v[1][0],
        v[1][1],
        GPtr::from_raw(&mut col as *mut u32).cast(),
    );
}

orig! {
    /// `FindPlayerVehicle(int32 playerId, bool bIncludeRemote)` (0x56E0D0)
    pub fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CRadar::ClearBlip(int32)` (0x587CE0)
    pub fn clear_blip_call(h: i32) = cdecl 0x587CE0 => crate::radar::clear_blip;
    /// `CRadar::ClearActualBlip(int32)` (0x587C10)
    pub fn clear_actual_blip_call(i: i32) = cdecl 0x587C10 => crate::radar::clear_actual_blip;
    /// `CRadar::SetCoordBlip(eBlipType, CVector, eBlipColour, eBlipDisplay, const char*)` (0x583820)
    pub fn set_coord_blip_call(t: i32, px: f32, py: f32, pz: f32, colour: i32, display: i32, name: GPtr<u8>) -> i32 = cdecl 0x583820 => crate::radar::set_coord_blip;
}

/// `airstrip_table[4]` (0x8D06E0, 0x10 each: x, y, direction, ...)
const AIRSTRIP_TABLE: usize = 0x8D06E0;
/// `"CODEAIR"` (.rdata), the blip's script name
const CODEAIR: usize = 0x866B90;
/// `CTimer::m_FrameCounter`
const FRAME_COUNTER: usize = 0xB7CB4C;

/// The player vehicle's position: matrix position, or the placement without a matrix.
fn player_vehicle_pos() -> GPtr<f32> {
    let v = find_player_vehicle(-1, 0);
    let m: GPtr<u8> = rd(v, 0x14);
    if m.is_null() { v.byte_add(4).cast() } else { m.byte_add(0x30).cast() }
}

/// `CRadar::SetupAirstripBlips()` (0x587D20): while the player flies a plane other than the Vortex
/// (model 539), every 4th frame (frame counter bit 2) the nearest of the four airstrips is chosen
/// (distances `sqrt((ty - p.y)^2 + (tx - p.x)^2)` with the position fetched once per operand; ties go
/// to the later strip); a change clears the blip and stores the new strip only when a blip exists. A
/// missing blip is created at the current strip (`SetCoordBlip(8, (x, y, 0), 0, 2, "CODEAIR")`).
/// Outside such a plane the blip is cleared (the inlined handle check, then `ClearActualBlip`) and the
/// handle reset to 0.
#[hook(addr = 0x587D20, conv = cdecl, name = "CRadar::SetupAirstripBlips", expect = "83 EC 18 53 55")]
pub fn setup_airstrip_blips() {
    let flying = !find_player_vehicle(-1, 0).is_null()
        && rd::<u32>(find_player_vehicle(-1, 0), 0x594) == 4
        && rd::<u16>(find_player_vehicle(-1, 0), 0x22) != 0x21B;
    if !flying {
        let h = AIRSTRIP_BLIP.get();
        if h == 0 {
            return;
        }
        if h != -1 {
            let i = h & 0xFFFF;
            let t = trace(i);
            if rd::<u16>(t, 0x14) == ((h as u32) >> 16) as u16 && rd::<u8>(t, 0x25) & 2 != 0 {
                clear_actual_blip_call(i);
            }
        }
        AIRSTRIP_BLIP.set(0);
        return;
    }
    let mut create = true;
    if g::<u32>(FRAME_COUNTER).read() & 4 != 0 {
        let mut d = [0f32; 4];
        for (k, dk) in d.iter_mut().enumerate() {
            let a = player_vehicle_pos();
            let b = player_vehicle_pos();
            let c = player_vehicle_pos();
            let e = player_vehicle_pos();
            let t = AIRSTRIP_TABLE + 0x10 * k;
            let (tx, ty) = (g::<f32>(t).read(), g::<f32>(t + 4).read());
            let yy = x(ty).fsub_m(e.byte_add(4).read()).fmul(x(ty).fsub_m(c.byte_add(4).read()));
            let xx = x(tx).fsub_m(a.read()).fmul(x(tx).fsub_m(b.read()));
            *dk = yy.fadd(xx).fsqrt().to_f32();
        }
        let lt = |i: usize, j: usize| x(d[i]).fcom(x(d[j])).c0() && !x(d[i]).fcom(x(d[j])).c2();
        let nearest: u8 = if lt(0, 1) && lt(0, 2) && lt(0, 3) {
            0
        } else if lt(1, 0) && lt(1, 2) && lt(1, 3) {
            1
        } else if lt(2, 0) && lt(2, 1) && lt(2, 3) {
            2
        } else {
            3
        };
        if AIRSTRIP_LOCATION.get() != nearest {
            let h = AIRSTRIP_BLIP.get();
            if h != 0 {
                clear_blip_call(h);
                AIRSTRIP_LOCATION.set(nearest);
            }
        } else {
            create = AIRSTRIP_BLIP.get() == 0;
        }
    } else {
        create = AIRSTRIP_BLIP.get() == 0;
    }
    if !create {
        return;
    }
    let t = AIRSTRIP_TABLE + 0x10 * usize::from(AIRSTRIP_LOCATION.get());
    let bx = g::<f32>(t).read();
    // `fld` / `fstp` of y through a stack temporary (a signalling NaN is stored quiet)
    let by = x(g::<f32>(t + 4).read()).to_f32();
    let h = set_coord_blip_call(8, bx, by, 0.0, 0, 2, g(CODEAIR));
    AIRSTRIP_BLIP.set(h);
}

orig! {
    /// `FindPlayerHeading(int32)` (0x56E450, ST0)
    pub fn find_player_heading_st0(id: i32) -> f64 = cdecl 0x56E450 => unported;
    /// `CRadar::DrawRotatingRadarSprite(...)` (0x584850)
    #[allow(clippy::too_many_arguments)]
    pub fn draw_rotating_radar_sprite_call(sprite: GPtr<u8>, px: f32, py: f32, angle: f32, width: u32, height: u32, colour: u32) = cdecl 0x584850 => draw_rotating_radar_sprite;
    /// `FindPlayerPed(int32)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => sa_world::world::cworld::find_player_ped_port;
    /// `CRadar::InitFrontEndMap()` (0x585960)
    pub fn init_front_end_map_call() = cdecl 0x585960 => init_front_end_map;
    /// `FindPlayerCentreOfWorldForMap(int32)` (0x56E400, `CVector` through a hidden return pointer)
    pub fn find_player_centre_of_world_for_map(out: GPtr<f32>, id: i32) -> GPtr<f32> = cdecl 0x56E400 => unported;
    /// `CRadar::TransformRealWorldPointToRadarSpace(CVector2D&, const CVector2D&)` (0x583530)
    pub fn transform_real_world_point_to_radar_space_call(out: GPtr<f32>, inp: GPtr<f32>) = cdecl 0x583530 => crate::radar::transform_real_world_point_to_radar_space;
    /// `CRadar::LimitRadarPoint(CVector2D&)` (0x5832F0, ST0)
    pub fn limit_radar_point_call_st0(p: GPtr<f32>) -> f64 = cdecl 0x5832F0 => limit_radar_point_call_st0_rust;
}

/// `CTimer::m_snTimeInMillisecondsPauseMode`
const TIME_PAUSE_MODE: usize = 0xB7CB7C;
/// `DrawYouAreHereSprite`'s blink timer and state (`mapYouAreHereTimer`, `mapYouAreHereDisplay`)
const YOU_ARE_HERE_TIMER: usize = 0xBAA358;
const YOU_ARE_HERE_DISPLAY: usize = 0x8D0930;
/// pi (0x858CB8), 25.0 (0x858FE8), 17.0 (0x85F100)
const PI: f32 = f32::from_bits(0x4049_0FDB);
const C25: f32 = 25.0;
const C17: f32 = 17.0;
/// `CTheScripts::HideAllFrontEndMapBlips`, `CTheScripts::bPlayerIsOffTheMap`
const HIDE_ALL_FRONT_END_MAP_BLIPS: usize = 0xA444A1;
const PLAYER_IS_OFF_THE_MAP: usize = 0xA444A4;
/// `FrontEndMenuManager.m_vMousePos` (+0x70)
const MENU_MOUSE_POS: usize = 0xBA67B8;

/// `CRadar::DrawYouAreHereSprite(float x, float y)` (0x584960): the marker blinks (on for 500 ms, off
/// for 200 ms of pause-mode time, unsigned differences); while on, `RadarBlipSprites[3]` is drawn rotated
/// by `heading + pi`, 17 units from (x, y) towards `heading + 3pi/2`, `(int)(25 * width / 640)` x
/// `(int)(25 * height / 448)`, white. Sprite 3 is appended to the legend in every case.
#[hook(addr = 0x584960, conv = cdecl, name = "CRadar::DrawYouAreHereSprite", expect = "A0 30 09 8D 00")]
pub fn draw_you_are_here_sprite(px: f32, py: f32) {
    let now = g::<u32>(TIME_PAUSE_MODE).read();
    let timer = g::<u32>(YOU_ARE_HERE_TIMER);
    let display = g::<u8>(YOU_ARE_HERE_DISPLAY);
    let draw = if display.read() != 0 {
        if now.wrapping_sub(timer.read()) > 500 {
            timer.write(now);
            display.write(0);
            false
        } else {
            true
        }
    } else if now.wrapping_sub(timer.read()) > 200 {
        timer.write(now);
        display.write(1);
        true
    } else {
        false
    };
    if draw {
        let a = find_player_heading(0).fadd_m(PI).to_f32();
        let mut c = [0u8; 4];
        let cp = rgba_ctor(GPtr::from_raw(c.as_mut_ptr()), 0xFF, 0xFF, 0xFF, 0xFF);
        let circle = x(a).fadd_m(FRAC_PI_2);
        let colour = cp.cast::<u32>().read();
        let h = X87::from_i32(g::<i32>(RS_MAX_HEIGHT).read()).fmul_m(INV_448).fmul_m(C25).ftol() as u32;
        let w = X87::from_i32(g::<i32>(RS_MAX_WIDTH).read()).fmul_m(INV_640).fmul_m(C25).ftol() as u32;
        let sy = circle.fsin().fmul_m(C17).fsubr_m(py).to_f32();
        let sx = circle.fcos().fmul_m(C17).fadd_m(px).to_f32();
        draw_rotating_radar_sprite_call(g(RADAR_BLIP_SPRITES + 3 * 4), sx, sy, a, w, h, colour);
    }
    let n = MAP_LEGEND_COUNTER.get();
    MAP_LEGEND_LIST.ptr().cast::<i16>().byte_add(2 * n as isize).write(3);
    MAP_LEGEND_COUNTER.set(n.wrapping_add(1));
}

/// `CRadar::SetMapCentreToPlayerCoords()` (0x585B20): with a player ped, `InitFrontEndMap` with
/// `m_bDrawingMap` set, the player's map position (0 when the script hides the map blips or the player
/// is off the map) becomes the mouse position, and the map origin is placed so that the limited radar
/// point of that position lands on the screen centre (320, 224): `320 - zoom * r.x`,
/// `zoom * r.y + 224`; `m_bDrawingMap` is cleared again.
#[hook(addr = 0x585B20, conv = cdecl, name = "CRadar::SetMapCentreToPlayerCoords", expect = "83 EC 1C 6A FF")]
pub fn set_map_centre_to_player_coords() {
    if find_player_ped(-1).is_null() {
        return;
    }
    g::<u8>(MENU_DRAWING_MAP).write(1);
    init_front_end_map_call();
    let mut pos = [0u32; 3];
    let pp = GPtr::from_raw(pos.as_mut_ptr()).cast::<f32>();
    find_player_centre_of_world_for_map(pp, 0);
    if g::<u8>(HIDE_ALL_FRONT_END_MAP_BLIPS).read() != 0 || g::<u8>(PLAYER_IS_OFF_THE_MAP).read() != 0 {
        pos[1] = 0;
        pos[0] = 0;
    }
    let mut world = [pos[0], pos[1]];
    let mut radar = [0u32; 2];
    let rp = GPtr::from_raw(radar.as_mut_ptr()).cast::<f32>();
    transform_real_world_point_to_radar_space_call(rp, GPtr::from_raw(world.as_mut_ptr()).cast());
    let _ = limit_radar_point_call(rp);
    let zoom = g::<f32>(MENU_MAP_ZOOM).read();
    let ox = x(zoom).fmul_m(f32::from_bits(radar[0])).fsubr_m(320.0);
    g::<u32>(MENU_MOUSE_POS).write(pos[0]);
    g::<u32>(MENU_MOUSE_POS + 4).write(pos[1]);
    g::<u8>(MENU_DRAWING_MAP).write(0);
    g::<f32>(MENU_MAP_ORIGIN_X).write(ox.to_f32());
    let oy = x(zoom).fmul_m(f32::from_bits(radar[1])).fadd_m(224.0);
    g::<f32>(MENU_MAP_ORIGIN_Y).write(oy.to_f32());
}

orig! {
    /// `CRadar::Initialise()` (0x587FB0)
    pub fn radar_initialise_call() = cdecl 0x587FB0 => crate::radar::initialise;
    /// `CGenericGameStorage::LoadDataFromWorkBuffer(void*, int32)` (0x5D1300)
    pub fn load_data_from_work_buffer(p: GPtr<u8>, size: i32) = cdecl 0x5D1300 => unported;
    /// `CGenericGameStorage::SaveDataToWorkBuffer(void*, int32)` (0x5D1270)
    pub fn save_data_to_work_buffer(p: GPtr<u8>, size: i32) = cdecl 0x5D1270 => unported;
}

/// `CEntryExitManager::mp_poolEntryExits` (`CPool<CEntryExit>`, 0x3C per object)
const ENTRY_EXIT_POOL: usize = 0x96A7D8;

/// `CRadar::Load()` (0x5D53C0): `Initialise`, then the 175 traces from the save buffer (0x28 bytes
/// each); a stored entry/exit (+0x20, saved as pool index + 1) becomes the pool object, or null when that
/// slot is free. Returns true (AL).
#[hook(addr = 0x5D53C0, conv = cdecl, name = "CRadar::Load", expect = "53 56 57 E8 E8 2B FB FF")]
pub fn load() -> u8 {
    radar_initialise_call();
    for i in 0..175 {
        let t = trace(i);
        load_data_from_work_buffer(t, 0x28);
        let v = rd::<u32>(t, 0x20);
        if v != 0 {
            let pool: GPtr<u8> = g::<GPtr<u8>>(ENTRY_EXIT_POOL).read();
            let map: GPtr<u8> = rd(pool, 4);
            let idx = v.wrapping_sub(1);
            let r = if (map.byte_add(idx as i32 as isize).read() as i8) < 0 {
                0
            } else {
                rd::<u32>(pool, 0).wrapping_add(idx.wrapping_mul(0x3C))
            };
            wr::<u32>(t, 0x20, r);
        }
    }
    1
}

/// `CRadar::Save()` (0x5D5860): every trace is written to the save buffer (0x28 bytes) with its
/// entry/exit pointer replaced by pool index + 1 (only for a pointer into a used pool slot) and, for a
/// blip type other than 4 / 5, the in-use bit cleared; both are restored after the write. Returns true.
#[hook(addr = 0x5D5860, conv = cdecl, name = "CRadar::Save", expect = "83 EC 08 53 55")]
pub fn save() -> u8 {
    for i in 0..175 {
        let t = trace(i);
        let p = rd::<u32>(t, 0x20);
        let mut saved = 0u32;
        if p != 0 {
            let pool: GPtr<u8> = g::<GPtr<u8>>(ENTRY_EXIT_POOL).read();
            let idx = (p.wrapping_sub(rd::<u32>(pool, 0)) as i32) / 0x3C;
            if idx >= 0 && idx < rd::<i32>(pool, 8) {
                let map: GPtr<u8> = rd(pool, 4);
                if (map.byte_add(idx as isize).read() as i8) >= 0 {
                    saved = p;
                    wr::<u32>(t, 0x20, (idx + 1) as u32);
                }
            }
        }
        let kind = (rd::<u8>(t, 0x26) >> 2) & 0xF;
        let mut cleared = false;
        if kind != 4 && kind != 5 {
            let f = rd::<u8>(t, 0x25);
            if f & 2 != 0 {
                wr::<u8>(t, 0x25, f & 0xFD);
                cleared = true;
            }
        }
        save_data_to_work_buffer(t, 0x28);
        if saved != 0 {
            wr::<u32>(t, 0x20, saved);
        }
        if cleared {
            wr::<u8>(t, 0x25, rd::<u8>(t, 0x25) | 2);
        }
    }
    1
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_CALCULATE_CACHED_SIN_COS,
    HOOK_GET_RADAR_TRACE_COLOUR,
    HOOK_ADD_BLIP_TO_LEGEND_LIST,
    HOOK_INIT_FRONT_END_MAP,
    HOOK_REQUEST_MAP_SECTION,
    HOOK_REMOVE_MAP_SECTION,
    HOOK_REMOVE_RADAR_SECTIONS,
    HOOK_SHUTDOWN,
    HOOK_LINE_RADAR_BOX_COLLISION,
    HOOK_CLIP_RADAR_POLY,
    HOOK_LOAD_TEXTURES,
    HOOK_LIMIT_TO_MAP,
    HOOK_TRANSFORM_RADAR_POINT_TO_REAL_WORLD_SPACE,
    HOOK_SHOW_RADAR_TRACE,
    HOOK_DRAW_ROTATING_RADAR_SPRITE,
    HOOK_SETUP_AIRSTRIP_BLIPS,
    HOOK_DRAW_YOU_ARE_HERE_SPRITE,
    HOOK_SET_MAP_CENTRE_TO_PLAYER_COORDS,
    HOOK_LOAD,
    HOOK_SAVE,
];

/// [`find_player_heading_st0`] as an x87 value (ST0, returned through the f64 ABI).
#[allow(dead_code)]
pub fn find_player_heading(id: i32) -> X87 {
    X87::from_f64_exact(find_player_heading_st0(id))
}

/// The Rust path of [`limit_radar_point_call`] as an ST0 value.
#[allow(dead_code)]
fn limit_radar_point_call_st0_rust(p: GPtr<f32>) -> f64 {
    crate::radar::limit_radar_point(p).to_f64()
}

/// [`limit_radar_point_call_st0`] as an x87 value (ST0, returned through the f64 ABI).
#[allow(dead_code)]
pub fn limit_radar_point_call(p: GPtr<f32>) -> X87 {
    X87::from_f64_exact(limit_radar_point_call_st0(p))
}
