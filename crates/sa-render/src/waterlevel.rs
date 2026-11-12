//! PORT-RENDER-2: `CWaterLevel`. Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/RENDER/` as reading aids.
//!
//! Layouts: vertices `m_aVertices` 0xC22910 (0x14 bytes: +0 x i16, +2 y i16, +4 CRenPar: +4 z, +8 big
//! waves, +0xC small waves, +0x10 flow x i8, +0x11 flow y i8), quads `WaterQuads` 0xC21C90 (0xA bytes: 4 x
//! i16 vertex index, +8 u16 flags), triangles `WaterTriangles` 0xC22854 (8 bytes: 3 x i16, +6 u16 flags).
//! `m_BlockPolyInfo` 0xC21B70 (12 x 12 u16, `[x * 12 + y]`): bits 14..15 the kind (0 none, 1 quad, 2
//! triangle, 3 list into `m_PolyCombos` 0xC215F8, a run of u16 entries ended by kind 0), bits 0..13 the
//! index.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;
use sa_world::physical::base::floor_f64;

use crate::common::{Fpu, f32_of, ftol32, g, k, rd, wr, x, xi};

sa_core::globals! {
    list GLOBALS;
    /// `CWaterLevel::m_nWaterConfiguration`
    pub static WATER_CONFIGURATION @ 0xC228A0: u32 = 0;
    /// `CWaterLevel::m_nWaterTimeOffset`
    pub static WATER_TIME_OFFSET @ 0xC228A4: u32 = 0;
    /// `waterclear256Raster`, `texWaterclear256`, `seabd32Raster`, `texSeabd32`, `waterwakeRaster`,
    /// `texWaterwake`
    pub static WATERCLEAR256_RASTER @ 0xC228A8: u32 = 0;
    pub static TEX_WATERCLEAR256 @ 0xC228AC: u32 = 0;
    pub static SEABD32_RASTER @ 0xC228B0: u32 = 0;
    pub static TEX_SEABD32 @ 0xC228B4: u32 = 0;
    pub static WATERWAKE_RASTER @ 0xC228B8: u32 = 0;
    pub static TEX_WATERWAKE @ 0xC228BC: u32 = 0;
    /// `CWaterLevel::WaterColorTriangle`, `CWaterLevel::WaterColor` (CRGBA)
    pub static WATER_COLOR_TRIANGLE @ 0xC21168: u32 = 0;
    pub static WATER_COLOR @ 0xC2116C: u32 = 0;
    /// `TextureShiftFirstV`, `TextureShiftFirstU`, `TextureShiftSecondV`, `TextureShiftSecondU`
    pub static TEXTURE_SHIFT_FIRST_V @ 0xC21178: f32 = 0.0;
    pub static TEXTURE_SHIFT_FIRST_U @ 0xC2117C: f32 = 0.0;
    pub static TEXTURE_SHIFT_SECOND_V @ 0xC21180: f32 = 0.0;
    pub static TEXTURE_SHIFT_SECOND_U @ 0xC21184: f32 = 0.0;
    /// `CWaterLevel::WaterLayerAlpha[2]` (.data: 160, 90)
    pub static WATER_LAYER_ALPHA @ 0x8D3808: [u32; 2] = [0xA0, 0x5A];
    /// `CWaterLevel::bSplitBigPolys` (.data, 1)
    pub static SPLIT_BIG_POLYS @ 0x8D37F4: u8 = 1;
    /// `CWaterLevel::m_WaterFogDensity` (.data, 26)
    pub static WATER_FOG_DENSITY @ 0x8D37E0: i32 = 26;
    /// The water-fog layer colour (.data, r g b a = ff ff ff 05)
    pub static WATER_FOG_COLOUR @ 0x8D37E8: u32 = 0x05FF_FFFF;
    /// The inside-fog screen quad colour (.data, r g b a = ff ff ff 28)
    pub static WATER_FOG_INSIDE_COLOUR @ 0x8D37EC: u32 = 0x28FF_FFFF;
    /// The inside-fog fade speed (.data, 0.04)
    pub static WATER_FOG_FADE_SPEED @ 0x8D37F0: u32 = 0x3D23_D70A;
    /// The inside-fog fade (0..1) and its hold timer
    pub static WATER_FOG_FADE @ 0xC228EC: f32 = 0.0;
    pub static WATER_FOG_HOLD @ 0xC228F0: f32 = 0.0;
    /// `CWaterLevel::m_bWaterFog` (.data, 1)
    pub static WATER_FOG @ 0x8D37D4: u8 = 1;
    /// `CWaterLevel::m_fWaterFogHeight` (.data, 12.0)
    pub static WATER_FOG_HEIGHT @ 0x8D37E4: u32 = 0x4140_0000;
    /// `CWaterLevel::ms_WaterFog` (70 x i16 x1, y1, x2, y2, then 70 x f32 z)
    pub static WATER_FOG_DATA @ 0xC21188: [u8; 0x348] = [0; 0x348];
    /// `gWaterFogIndex`, `gbPlayerIsInsideWaterFog`
    pub static WATER_FOG_INDEX @ 0xC228E4: i32 = 0;
    pub static PLAYER_INSIDE_WATER_FOG @ 0xC228E8: u8 = 0;
    /// `CWaterLevel::faWaveMultipliersX` (.data: 1, .85, .73, .77, .75, .8, .73, .8)
    pub static WAVE_MUL_X @ 0x8D38C8: [u32; 8] = [
        0x3F80_0000, 0x3F59_999A, 0x3F3A_E148, 0x3F45_1EB8, 0x3F40_0000, 0x3F4C_CCCD, 0x3F3A_E148, 0x3F4C_CCCD,
    ];
    /// `CWaterLevel::faWaveMultipliersY` (.data: .75, .9, .95, .82, .7, .75, .9, 1)
    pub static WAVE_MUL_Y @ 0x8D38E8: [u32; 8] = [
        0x3F40_0000, 0x3F66_6666, 0x3F73_3333, 0x3F51_EB85, 0x3F33_3333, 0x3F40_0000, 0x3F66_6666, 0x3F80_0000,
    ];
    /// The wave colour multiplier's offset (.data, 0.27f)
    pub static WAVE_COLOUR_ADD @ 0x8D3920: u32 = 0x3E8A_3D71;
    /// The wave colour multiplier's scale (.data, 0.65f)
    pub static WAVE_COLOUR_MUL @ 0x8D3924: u32 = 0x3F26_6666;
    /// `CWaterLevel::m_PolyCombos[700]`
    pub static POLY_COMBOS @ 0xC215F8: [u16; 700] = [0; 700];
    /// `CWaterLevel::m_BlockPolyInfo[12][12]`
    pub static BLOCK_POLY_INFO @ 0xC21B70: [u16; 144] = [0; 144];
    /// `CWaterLevel::WaterQuads[301]` (0xA bytes each)
    pub static WATER_QUADS @ 0xC21C90: [u8; 301 * 0xA] = [0; 301 * 0xA];
    /// `CWaterLevel::WaterTriangles[6]` (8 bytes each)
    pub static WATER_TRIANGLES @ 0xC22854: [u8; 6 * 8] = [0; 6 * 8];
    /// `CWaterLevel::CameraRangeMaxY`, `MinY`, `MaxX`, `MinX`
    pub static CAMERA_RANGE_MAX_Y @ 0xC1F950: i32 = 0;
    pub static CAMERA_RANGE_MIN_Y @ 0xC1F954: i32 = 0;
    pub static CAMERA_RANGE_MAX_X @ 0xC1F958: i32 = 0;
    pub static CAMERA_RANGE_MIN_X @ 0xC1F95C: i32 = 0;
    /// `CWaterLevel::m_BlocksToBeRenderedOutsideWorldY[70]`
    pub static BLOCKS_OUTSIDE_Y @ 0xC214D0: [i16; 70] = [0; 70];
    /// `CWaterLevel::m_BlocksToBeRenderedOutsideWorldX[70]`
    pub static BLOCKS_OUTSIDE_X @ 0xC21560: [i16; 70] = [0; 70];
    /// `CWaterLevel::m_NumBlocksOutsideWorldToBeRendered`
    pub static NUM_BLOCKS_OUTSIDE @ 0xC215EC: i32 = 0;
    /// `CWaterLevel::DETAILEDWATERDIST` (.data, 48)
    pub static DETAILED_WATER_DIST @ 0x8D37D0: i32 = 48;
    /// `CWaterLevel::NumWaterZonePolys` (entries used in `m_PolyCombos`)
    pub static NUM_WATER_ZONE_POLYS @ 0xC215F0: i32 = 0;
    /// `CWaterLevel::m_CurrentFlow` (CVector2D)
    pub static CURRENT_FLOW @ 0xC22890: [f32; 2] = [0.0; 2];
    /// `CWaterLevel::m_CurrentDesiredFlow` (CVector2D)
    pub static CURRENT_DESIRED_FLOW @ 0xC22898: [f32; 2] = [0.0; 2];
    /// `PreRenderWater`'s function-static CVector (the player position on the first call) and its guard
    pub static PRE_RENDER_STATIC_POS @ 0xC278E4: [u32; 3] = [0; 3];
    pub static PRE_RENDER_STATIC_GUARD @ 0xC278F0: u32 = 0;
    /// `CWaterLevel::NumWaterTriangles`
    pub static NUM_WATER_TRIANGLES @ 0xC22884: u32 = 0;
    /// `CWaterLevel::NumWaterQuads`
    pub static NUM_WATER_QUADS @ 0xC22888: u32 = 0;
    /// `CWaterLevel::NumWaterVertices`
    pub static NUM_WATER_VERTICES @ 0xC2288C: u32 = 0;
    /// `CWaterLevel::m_aVertices[1021]` (0x14 bytes each)
    pub static VERTICES @ 0xC22910: [u8; 1021 * 0x14] = [0; 1021 * 0x14];
}

const VERTICES_BASE: usize = 0xC22910;
const QUADS_BASE: usize = 0xC21C90;
const TRIANGLES_BASE: usize = 0xC22854;
/// `CTimer::m_snTimeInMilliseconds`
const TIME_MS: usize = 0xB7CB84;
/// `CWeather::Wavyness`, `CWeather::SunGlare`
const WAVYNESS: usize = 0xC812E8;
const SUN_GLARE: usize = 0xC812E0;
/// `CMaths::ms_SinTable[256]` (.bss, filled by `InitMathsTables`)
const SIN_TABLE: usize = 0xBB3E00;

/// 1.0 (0x858624), 0.0 (0x858B50), 3.0 (0x858B3C), 6.0 (0x858B44), 20.0 (0x858BA4), 0.002 (0x858F44),
/// 0.5 (0x858B8C), 2^32 (0x858C54)
const ONE: u32 = 0x3F80_0000;
const ZERO: u32 = 0x0000_0000;
const THREE: u32 = 0x4040_0000;
const SIX: u32 = 0x40C0_0000;
const TWENTY: u32 = 0x41A0_0000;
const BLOCK_SCALE: u32 = 0x3B03_126F;
const HALF: u32 = 0x3F00_0000;
const TWO_POW_32: u32 = 0x4F80_0000;
/// Wave constants (0x871FF8.., 0x85A778 = 256 / 2pi, 0x859A44 = 64.0)
const W_TWO: u32 = 0x4000_0000;
const W_ONE: u32 = 0x3F80_0000;
const W_HALF: u32 = 0x3F00_0000;
const W_GLARE_DOT: u32 = 0x3F13_B646; // 0.577 (0x872004)
const W_P1_T: u32 = 0x3AA4_B5BE; // 0x872020
const W_P1_XY: u32 = 0x3DC9_0FDB; // 0x87201C
const W_P2_T: u32 = 0x3AEB_4CC7; // 0x872018
const W_P2_Y: u32 = 0x3DF7_75FA; // 0x872014
const W_P2_X: u32 = 0x3E77_75FA; // 0x872010
const W_P3_T: u32 = 0x3B09_421F; // 0x87200C
const W_P3_Y: u32 = 0x3EA0_D97C; // 0x872008
const SIN_SCALE: u32 = 0x4222_F983;
const COS_SHIFT: u32 = 0x4280_0000;
const EIGHT: u32 = 0x4100_0000; // 0x859000
const FIVE: u32 = 0x40A0_0000; // 0x858C80
const P099: u32 = 0x3F7D_70A4; // 0x862CD0

/// `CTimer::m_FrameCounter`, `CTimer::ms_fTimeStep`
const FRAME_COUNTER: usize = 0xB7CB4C;
const TIME_STEP: usize = 0xB7CB5C;
/// 0.001 (0x858CDC), 500.0 (0x858B58)
const FLOW_RATE: u32 = 0x3A83_126F;
const BLOCK_SIZE: u32 = 0x43FA_0000;
/// TheCamera +0xE4 / +0xE8 (distance to the nearest wavy water quad, that quad's z)
const CAM_DIST_TO_WATER: usize = 0xB6F10C;
const CAM_HEIGHT_OF_WATER: usize = 0xB6F110;
/// 1e7 (immediate), 1/64 (0x85A7D4)
const NEAREST_START: u32 = 0x4B18_9680;
const FLOW_SCALE: u32 = 0x3C80_0000;
/// `CGame::currArea`
const CURR_AREA: usize = 0xB72914;
/// `TheCamera.m_pRwCamera`
const RW_CAMERA: usize = 0xB6F97C;
/// 950.0 (0x858F4C), 30.0 (pushed), -100.0 (0x859014)
const MARK_Z: u32 = 0x446D_8000;
const GROUND_PROBE: u32 = 0x41F0_0000;
const NO_GROUND: u32 = 0xC2C8_0000;

orig! {
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool buildings, bool
    /// vehicles, bool peds, bool objects, bool dummies, bool seeThrough, CStoredCollPoly*)` (0x5674E0)
    pub fn process_vertical_line(p: GPtr<f32>, z: u32, cp: GPtr<u8>, e: GPtr<GPtr<u8>>, b: u32, v: u32, pe: u32, o: u32, d: u32, st: u32, poly: u32) -> u8 = cdecl 0x5674E0 => unported;
    /// `RwV3dTransformPoints(RwV3d* out, const RwV3d* in, int32 n, const RwMatrix*)` (0x7EDD90)
    pub fn rw_v3d_transform_points(out: GPtr<u8>, inp: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDD90 => unported;
    /// `CWorldScan::ScanWorld(CVector2D* points, int32 n, void (*)(int32, int32))` (0x72CAE0)
    pub fn world_scan_scan_world(points: GPtr<u8>, n: i32, cb: u32) = cdecl 0x72CAE0 => unported;
    /// `CWaterLevel::MarkQuadsAndPolysToBeRendered` (0x6E5810)
    pub fn o_mark_quads_and_polys(x: i32, y: i32, interior: u8) = cdecl 0x6E5810 => crate::waterlevel::mark_quads_and_polys_to_be_rendered;
    /// `CWaterLevel::AddWaterLevelVertex(int32 x, int32 y, CRenPar)` (0x6E5A40)
    pub fn o_add_vertex(x: i32, y: i32, z: u32, big: u32, small: u32, flow: u32) -> i32 = cdecl 0x6E5A40 => crate::waterlevel::add_water_level_vertex;
    /// The CRenPar lerp `(out, a, b, t)` (0x6E5280): every float `a * (1 - t) + b * t`
    pub fn ren_par_lerp(out: GPtr<u8>, a0: u32, a1: u32, a2: u32, a3: u32, b0: u32, b1: u32, b2: u32, b3: u32, t: u32) -> GPtr<u8> = cdecl 0x6E5280 => unported;
    /// `CWaterLevel::RenderFlatWaterRectangle` (0x6EBEC0)
    pub fn o_render_flat_water_rectangle(x1: i32, x2: i32, y1: i32, y2: i32, a0: u32, a1: u32, a2: u32, a3: u32, b0: u32, b1: u32, b2: u32, b3: u32, c0: u32, c1: u32, c2: u32, c3: u32, d0: u32, d1: u32, d2: u32, d3: u32) = cdecl 0x6EBEC0 => crate::waterlevel::render_flat_water_rectangle;
    /// `CWaterLevel::RenderFlatWaterRectangle_OneLayer` (0x6E9940)
    pub fn o_render_flat_water_rectangle_one_layer(x1: i32, x2: i32, y1: i32, y2: i32, a0: u32, a1: u32, a2: u32, a3: u32, b0: u32, b1: u32, b2: u32, b3: u32, c0: u32, c1: u32, c2: u32, c3: u32, d0: u32, d1: u32, d2: u32, d3: u32, layer: i32) = cdecl 0x6E9940 => crate::waterlevel::render_flat_water_rectangle_one_layer;
    /// `CWaterLevel::SplitWaterTriangleAlongXLine(int32 split, x1, y1, p1, x2, y2, p2, x3, y3, p3)` (0x6ECF00)
    pub fn o_split_water_triangle_along_x_line(s: i32, x1: i32, y1: i32, a0: u32, a1: u32, a2: u32, a3: u32, x2: i32, y2: i32, b0: u32, b1: u32, b2: u32, b3: u32, x3: i32, y3: i32, c0: u32, c1: u32, c2: u32, c3: u32) = cdecl 0x6ECF00 => unported;
    /// `CWaterLevel::SplitWaterTriangleAlongYLine(int32 split, x1, y1, p1, ..)` (0x6EE5A0)
    pub fn o_split_water_triangle_along_y_line(s: i32, x1: i32, y1: i32, a0: u32, a1: u32, a2: u32, a3: u32, x2: i32, y2: i32, b0: u32, b1: u32, b2: u32, b3: u32, x3: i32, y3: i32, c0: u32, c1: u32, c2: u32, c3: u32) = cdecl 0x6EE5A0 => unported;
    /// `CWaterLevel::RenderHighDetailWaterTriangle-` (0x6EDDC0)
    pub fn o_render_high_detail_water_triangle(x1: i32, y1: i32, a0: u32, a1: u32, a2: u32, a3: u32, x2: i32, y2: i32, b0: u32, b1: u32, b2: u32, b3: u32, x3: i32, y3: i32, c0: u32, c1: u32, c2: u32, c3: u32) = cdecl 0x6EDDC0 => crate::waterlevel::render_high_detail_water_triangle;
    /// `FUN_006e8780`: one layer of a high-detail water triangle (0x6E8780)
    pub fn o_render_high_detail_triangle_layer(x1: i32, y1: i32, a0: u32, a1: u32, a2: u32, a3: u32, x2: i32, y2: i32, b0: u32, b1: u32, b2: u32, b3: u32, x3: i32, y3: i32, c0: u32, c1: u32, c2: u32, c3: u32, layer: i32, sq: i32, sum: i32, n: i32) = cdecl 0x6E8780 => unported;
    /// `CCamera::IsSphereVisible(const CVector&, float, RwMatrix*)` (0x420C40)
    pub fn is_sphere_visible(this: GPtr<u8>, c: GPtr<u8>, r: u32, m: u32) -> u8 = thiscall 0x420C40 => unported;
    /// `CWaterLevel::RenderFlatWaterTriangle` (0x6EE080)
    pub fn o_render_flat_water_triangle(x1: i32, y1: i32, a0: u32, a1: u32, a2: u32, a3: u32, x2: i32, y2: i32, b0: u32, b1: u32, b2: u32, b3: u32, x3: i32, y3: i32, c0: u32, c1: u32, c2: u32, c3: u32) = cdecl 0x6EE080 => crate::waterlevel::render_flat_water_triangle;
    /// `CWaterLevel::RenderWaterRectangle` (0x6EC5D0)
    pub fn o_render_water_rectangle(x1: i32, x2: i32, y1: i32, y2: i32, a0: u32, a1: u32, a2: u32, a3: u32, b0: u32, b1: u32, b2: u32, b3: u32, c0: u32, c1: u32, c2: u32, c3: u32, d0: u32, d1: u32, d2: u32, d3: u32) = cdecl 0x6EC5D0 => crate::waterlevel::render_water_rectangle;
    /// `CWaterLevel::RenderHighDetailWaterRectangle` (0x6EB810)
    pub fn o_render_high_detail_water_rectangle(x1: i32, x2: i32, y1: i32, y2: i32, a0: u32, a1: u32, a2: u32, a3: u32, b0: u32, b1: u32, b2: u32, b3: u32, c0: u32, c1: u32, c2: u32, c3: u32, d0: u32, d1: u32, d2: u32, d3: u32) = cdecl 0x6EB810 => crate::waterlevel::render_high_detail_water_rectangle;
    /// `CWaterLevel::CalculateWavesOnlyForCoordinate(x, y, big, small, float* wave, float* colourMult, float*
    /// glare, CVector* n)` (0x6E6EF0)
    pub fn o_calculate_waves_only_for_coordinate(x0: i32, y0: i32, big: f32, small: f32, out: GPtr<f32>, color_mult: GPtr<f32>, glare: GPtr<f32>, n: GPtr<f32>) = cdecl 0x6E6EF0 => crate::waterlevel::calculate_waves_only_for_coordinate;
    /// `FUN_006e91d0`: one layer of a high-detail water rectangle (0x6E91D0)
    pub fn o_render_high_detail_rectangle_layer(x1: i32, x2: i32, y1: i32, y2: i32, a0: u32, a1: u32, a2: u32, a3: u32, b0: u32, b1: u32, b2: u32, b3: u32, c0: u32, c1: u32, c2: u32, c3: u32, d0: u32, d1: u32, d2: u32, d3: u32, layer: i32, cells: i32, verts: i32, nx: i32, ny: i32) = cdecl 0x6E91D0 => crate::waterlevel::render_high_detail_rectangle_layer;
    /// `CWaterLevel::SplitWaterRectangleAlongXLine(int32 split, x1, x2, y1, y2, p1..p4)` (0x6E73A0)
    pub fn o_split_water_rectangle_along_x_line(s: i32, x1: i32, x2: i32, y1: i32, y2: i32, a0: u32, a1: u32, a2: u32, a3: u32, b0: u32, b1: u32, b2: u32, b3: u32, c0: u32, c1: u32, c2: u32, c3: u32, d0: u32, d1: u32, d2: u32, d3: u32) = cdecl 0x6E73A0 => crate::waterlevel::split_water_rectangle_along_x_line;
    /// `CWaterLevel::SetUpWaterFog(int32, int32, int32, int32)` (0x6EA9F0)
    pub fn o_set_up_water_fog(x1: i32, y1: i32, x2: i32, y2: i32) = cdecl 0x6EA9F0 => crate::waterlevel::set_up_water_fog;
    /// `CWaterLevel::RenderFlatWaterTriangle_OneLayer` (0x6E8ED0)
    pub fn o_render_flat_water_triangle_one_layer(x1: i32, y1: i32, a0: u32, a1: u32, a2: u32, a3: u32, x2: i32, y2: i32, b0: u32, b1: u32, b2: u32, b3: u32, x3: i32, y3: i32, c0: u32, c1: u32, c2: u32, c3: u32, layer: i32) = cdecl 0x6E8ED0 => crate::waterlevel::render_flat_water_triangle_one_layer;
    /// `CPostEffects::ImmediateModeRenderStatesStore()` (0x700CC0)
    pub fn post_fx_states_store() = cdecl 0x700CC0 => unported;
    /// `CPostEffects::ImmediateModeRenderStatesSet()` (0x700D70)
    pub fn post_fx_states_set() = cdecl 0x700D70 => crate::posteffects::immediate_mode_render_states_set;
    /// `CPostEffects::ImmediateModeRenderStatesReStore()` (0x700E00)
    pub fn post_fx_states_restore() = cdecl 0x700E00 => crate::posteffects::immediate_mode_render_states_restore;
    /// `CPostEffects::DrawQuad(float x, float y, float w, float h, uint8 r, g, b, a, RwRaster*)` (0x700EC0)
    pub fn post_fx_draw_quad(x0: u32, y0: u32, w: u32, h: u32, r: u32, g: u32, b: u32, a: u32, raster: u32) = cdecl 0x700EC0 => unported;
    /// `RwRenderStateSet` through `RwEngineInstance->dOpenDevice.fpRenderStateSet` (+0x20)
    pub fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwIm3DTransform(RxObjSpace3DVertex*, uint32 count, RwMatrix* ltm, uint32 flags)` (0x7EF450)
    pub fn rw_im3d_transform(verts: GPtr<u8>, count: u32, ltm: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderPrimitive(RwPrimitiveType)` (0x7EF6B0)
    pub fn rw_im3d_render_primitive(prim: u32) -> i32 = cdecl 0x7EF6B0 => unported;
    /// `RwIm3DEnd()` (0x7EF520)
    pub fn rw_im3d_end() -> i32 = cdecl 0x7EF520 => unported;
    /// `RenderBuffer::RenderIfDoesntFit()` (0x6E7680)
    pub fn render_buffer_render_if_doesnt_fit() = cdecl 0x6E7680 => unported;
    /// `RwTextureDestroy(RwTexture*)` (0x7F3820)
    pub fn rw_texture_destroy(t: GPtr<u8>) -> i32 = cdecl 0x7F3820 => unported;
    /// `FUN_006e5750`: add a quad / triangle to a block (0x6E5750)
    pub fn o_add_to_block(bx: i32, by: i32, idx: i32, kind: i32) = cdecl 0x6E5750 => crate::waterlevel::add_to_block;
    /// `CGame::CanSeeWaterFromCurrArea()` (0x53C4B0)
    pub fn can_see_water_from_curr_area() -> u8 = cdecl 0x53C4B0 => unported;
    /// `CWaterLevel::ScanThroughBlocks()` (0x6E6D10)
    pub fn o_scan_through_blocks() = cdecl 0x6E6D10 => crate::waterlevel::scan_through_blocks;
    /// `CWaterLevel::FindNearestWaterAndItsFlow()` (0x6E9D70)
    pub fn o_find_nearest_water_and_its_flow() = cdecl 0x6E9D70 => crate::waterlevel::find_nearest_water_and_its_flow;
    /// `FindPlayerPed(int32)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => sa_world::world::cworld::find_player_ped_port;
    /// `FUN_006ea8a0`: `CWaterLevel::GetGroundLevel` (0x6EA8A0)
    pub fn o_get_ground_level(p: GPtr<f32>, out: GPtr<f32>, col: GPtr<u8>, offset: u32) -> u8 = cdecl 0x6EA8A0 => crate::waterlevel::get_ground_level;
    /// `CVector::CrossProduct-out(CVector* out, const CVector* a, const CVector* b)` (0x59C730)
    pub fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => unported;
    /// `CVector::Normalise()` (0x59C910)
    pub fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CWaterLevel::GetWaterLevelNoWaves` (0x6E8580)
    pub fn o_get_water_level_no_waves(x: u32, y: u32, z: u32, level: GPtr<f32>, big: GPtr<f32>, small: GPtr<f32>) -> u8 = cdecl 0x6E8580 => crate::waterlevel::get_water_level_no_waves;
    /// `CWaterLevel::AddWaveToResult` (0x6E81E0)
    pub fn o_add_wave_to_result(x: u32, y: u32, level: GPtr<f32>, big: u32, small: u32, normal: GPtr<u8>) = cdecl 0x6E81E0 => crate::waterlevel::add_wave_to_result;
    /// `CWaterLevel::CalculateWavesOnlyForCoordinate2` (0x6E7210)
    pub fn o_calc_waves2(x: i32, y: i32, big: u32, small: u32, out: GPtr<f32>) = cdecl 0x6E7210 => crate::waterlevel::calculate_waves_only_for_coordinate2;
    /// `FUN_006e5bb0`: water level inside a quad (0x6E5BB0)
    pub fn o_quad_level(q: GPtr<u8>, x: u32, y: u32, z: u32, level: GPtr<f32>, big: GPtr<f32>, small: GPtr<f32>) -> u8 = cdecl 0x6E5BB0 => crate::waterlevel::quad_level;
    /// `FUN_006e5e90`: water level inside a triangle (0x6E5E90)
    pub fn o_triangle_level(t: GPtr<u8>, x: u32, y: u32, z: u32, level: GPtr<f32>, big: GPtr<f32>, small: GPtr<f32>) -> u8 = cdecl 0x6E5E90 => crate::waterlevel::triangle_level;
}

/// Vertex `i` (sign-extended index, as `movsx`).
fn vtx(i: i16) -> GPtr<u8> {
    GPtr::from_addr(VERTICES_BASE).byte_add(isize::from(i) * 0x14)
}

fn vx(i: i16) -> i32 {
    i32::from(rd::<i16>(vtx(i), 0))
}

fn vy(i: i16) -> i32 {
    i32::from(rd::<i16>(vtx(i), 2))
}

/// CRenPar field at `off` (+4 z, +8 big waves, +0xC small waves) of vertex `i`.
fn vf(i: i16, off: isize) -> f32 {
    rd::<f32>(vtx(i), off)
}

/// `fild dword` of a remainder held as uint32 (`test edx, edx; jge` skips the `+2^32`).
fn fild_u32(v: u32) -> X87 {
    let r = xi(v as i32);
    if (v as i32) < 0 { r.fadd_m(k(TWO_POW_32)) } else { r }
}

/// `fstp qword; call floor` (CRT 0x8219F0, exact).
fn floor_x(v: X87) -> X87 {
    X87::from_f64_exact(floor_f64(v.to_f64()))
}

/// `(m_snTimeInMilliseconds - m_nWaterTimeOffset) % p` (unsigned `div`).
fn wave_step(p: u32) -> u32 {
    g::<u32>(TIME_MS).read().wrapping_sub(WATER_TIME_OFFSET.get()) % p
}

/// `CMaths::ms_SinTable[ftol(v) & 0xFF]`.
fn sin_lut(v: X87) -> f32 {
    let i = ftol32(v) & 0xFF;
    g::<f32>(SIN_TABLE).wrapping_index(i).read()
}

/// `|x|`, `|y|` (`neg` when negative) and the wave multiplier `WaveMulX[(x / 2) & 7] * WaveMulY[(y / 2)
/// & 7] * CWeather::Wavyness` (C division by 2: `cdq; sub; sar`).
fn wave_base(x0: i32, y0: i32) -> (i32, i32, X87) {
    let ax = if x0 < 0 { x0.wrapping_neg() } else { x0 };
    let ay = if y0 < 0 { y0.wrapping_neg() } else { y0 };
    let ix = (ax / 2) & 7;
    let iy = (ay / 2) & 7;
    let m = x(f32::from_bits(WAVE_MUL_X.ptr().cast::<u32>().wrapping_index(ix).read()))
        .fmul_m(f32::from_bits(WAVE_MUL_Y.ptr().cast::<u32>().wrapping_index(iy).read()))
        .fmul_m(g::<f32>(WAVYNESS).read());
    (ax, ay, m)
}

/// `CWaterLevel::CalculateWavesOnlyForCoordinate2(int32 x, int32 y, float bigWaves, float smallWaves,
/// float* outWave)` (0x6E7210): the three wave terms of `CalculateWavesOnlyForCoordinate` without the
/// normal: `*out += sin(ph1) * 2 * m * big`, then `+ sin(ph2) * 1 * m * small`, then `+ sin(ph3) * 0.5 * m
/// * small` (the second sum is kept in memory, the third added to the unrounded second). Phase 1 uses
/// `(fy + fx)`, phase 2 `fy` term before the `fx` term, phase 3 `fy` only. The entry inlines the
/// protection chunk 0x403A06 (`ecx = m_nWaterTimeOffset`).
#[hook(addr = 0x6E7210, conv = cdecl, name = "CWaterLevel::CalculateWavesOnlyForCoordinate2", expect = "8B 44 24 04 85 C0")]
pub fn calculate_waves_only_for_coordinate2(x0: i32, y0: i32, big: u32, small: u32, out: GPtr<f32>) {
    let (big, small) = (f32::from_bits(big), f32::from_bits(small));
    let (ax, ay, m) = wave_base(x0, y0);
    let fx = xi(ax);
    let fy = f32_of(xi(ay)); // `fstp [esp+8]`: fy as f32
    // phase 1
    let s = fild_u32(wave_step(5000)).fmul_m(k(W_P1_T));
    let ph = s.fadd(x(fy).fadd(fx).fmul_m(k(W_P1_XY))).fmul_m(k(SIN_SCALE));
    let w1 = x(sin_lut(ph)).fmul_m(k(W_TWO)).fmul(m).fmul_m(big).fadd_m(out.read());
    let t1 = f32_of(w1);
    out.write(t1);
    // phase 2: (s * c + fy * cy) + fx * cx
    let s = fild_u32(wave_step(3500)).fmul_m(k(W_P2_T));
    let ph = s.fadd(x(fy).fmul_m(k(W_P2_Y))).fadd(fx.fmul_m(k(W_P2_X))).fmul_m(k(SIN_SCALE));
    let w2 = x(sin_lut(ph)).fmul_m(k(W_ONE)).fmul(m).fmul_m(small).fadd_m(t1);
    out.write(f32_of(w2)); // `fst`: ST0 keeps the unrounded sum
    // phase 3
    let s = fild_u32(wave_step(3000)).fmul_m(k(W_P3_T));
    let ph = s.fadd(x(fy).fmul_m(k(W_P3_Y))).fmul_m(k(SIN_SCALE));
    let w3 = x(sin_lut(ph)).fmul_m(k(W_HALF)).fmul(m).fmul_m(small).fadd(w2);
    out.write(f32_of(w3));
}

/// `CWaterLevel::CalculateWavesOnlyForCoordinate(int32 x, int32 y, float big, float small, float*
/// outWave, float* colorMult, float* glare, CVector* n)` (0x6E6EF0): the three wave terms added to
/// `*outWave` (as `CalculateWavesOnlyForCoordinate2`, the cosine index `ftol(p + 64)` of the same scaled
/// phase), the normal `(d, d, 1)` with `d = -(cos1 * 2 * m * big * 0.0981)`, `+= cos2 * m * small *
/// 0.2417` on x and y, `+= cos3 * 0.5 * m * small * 0.3142` on x, normalised; `*colorMult = max(g1, 0)
/// * 0.65 + 0.27` (the .data globals) with `g1 = (n.z + n.y + n.x) * 0.577`; `*glare` = `g2 = (n.z *
/// .577 + n.y * .577) + .577 * n.x`, then `clamp(g2 * 8 - 5, 0, 0.99) * CWeather::SunGlare` (NaN kept).
#[hook(addr = 0x6E6EF0, conv = cdecl, name = "CWaterLevel::CalculateWavesOnlyForCoordinate", expect = "8B 44 24 04 83 EC 0C")]
#[allow(clippy::too_many_arguments)]
pub fn calculate_waves_only_for_coordinate(
    x0: i32,
    y0: i32,
    big: f32,
    small: f32,
    out: GPtr<f32>,
    color_mult: GPtr<f32>,
    glare: GPtr<f32>,
    n: GPtr<f32>,
) {
    let (ax, ay, m) = wave_base(x0, y0);
    let mut f = Fpu::new();
    f.push(m);
    f.push(xi(ax)); // fx
    f.push(xi(ay)); // fy
    let nx = n;
    let ny = n.wrapping_index(1);
    let nz = n.wrapping_index(2);
    // phase 1
    f.push(fild_u32(wave_step(5000)));
    f.fmul_m(k(W_P1_T));
    f.fld_st(1);
    f.fadd_st(3);
    f.fmul_m(k(W_P1_XY));
    f.faddp(1);
    f.fmul_m(k(SIN_SCALE));
    let p = f.st(0);
    f.fld(sin_lut(p));
    f.fmul_m(k(W_TWO));
    f.fmul_st(4);
    f.fmul_m(big);
    f.fadd_m(out.read());
    let v = f.fstp();
    out.write(v);
    f.fadd_m(k(COS_SHIFT));
    let p = f.pop();
    f.fld(sin_lut(p));
    f.fmul_m(k(W_TWO));
    f.fmul_st(3);
    f.fmul_m(big);
    f.fmul_m(k(W_P1_XY));
    f.fchs();
    let d = f.fstp();
    nx.write(d);
    ny.write(d);
    nz.write(k(ONE));
    // phase 2
    f.push(fild_u32(wave_step(3500)));
    f.fmul_m(k(W_P2_T));
    f.fld_st(1);
    f.fmul_m(k(W_P2_Y));
    f.faddp(1);
    f.fld_st(2);
    f.fmul_m(k(W_P2_X));
    f.faddp(1);
    f.fmul_m(k(SIN_SCALE));
    let p = f.st(0);
    f.fld(sin_lut(p));
    f.fmul_m(k(W_ONE));
    f.fmul_st(4);
    f.fmul_m(small);
    f.fadd_m(out.read());
    let v = f.fstp();
    out.write(v);
    f.fadd_m(k(COS_SHIFT));
    let p = f.pop();
    f.fld(sin_lut(p));
    f.fmul_m(k(W_ONE));
    f.fmul_st(3);
    f.fmul_m(small);
    f.fmul_m(k(W_P2_X));
    f.fld_st(0);
    f.fadd_m(nx.read());
    let v = f.fstp();
    nx.write(v);
    f.fadd_m(ny.read());
    let v = f.fstp();
    ny.write(v);
    // phase 3 (fx dropped by `fstp st(1)`)
    f.push(fild_u32(wave_step(3000)));
    f.fmul_m(k(W_P3_T));
    f.fxch(1);
    f.fmul_m(k(W_P3_Y));
    f.faddp(1);
    f.fstp_st(1);
    f.fmul_m(k(SIN_SCALE));
    let p = f.st(0);
    f.fld(sin_lut(p));
    f.fmul_m(k(W_HALF));
    f.fmul_st(2);
    f.fmul_m(small);
    f.fadd_m(out.read());
    let v = f.fstp();
    out.write(v);
    f.fadd_m(k(COS_SHIFT));
    let p = f.pop();
    f.fld(sin_lut(p));
    f.fmul_m(k(W_HALF));
    f.fmulp(1);
    f.fmul_m(small);
    f.fmul_m(k(W_P3_Y));
    f.fadd_m(nx.read());
    let v = f.fstp();
    nx.write(v);
    vector_normalise(n.cast());
    // colour multiplier
    let g1 = x(nz.read()).fadd_m(ny.read()).fadd_m(nx.read()).fmul_m(k(W_GLARE_DOT));
    // `fld 0; fcomp g1; test ah, 0x41; jne` keeps g1 when 0 <= g1 or unordered
    let g1 = if x(k(ZERO)).fcom(g1).test_any(0x41) { g1 } else { x(k(ZERO)) };
    let cm = g1.fmul_m(f32::from_bits(WAVE_COLOUR_MUL.get())).fadd_m(f32::from_bits(WAVE_COLOUR_ADD.get()));
    color_mult.write(f32_of(cm));
    // glare
    let g2 = x(nz.read())
        .fmul_m(k(W_GLARE_DOT))
        .fadd(x(ny.read()).fmul_m(k(W_GLARE_DOT)))
        .fadd(x(k(W_GLARE_DOT)).fmul_m(nx.read()));
    glare.write(f32_of(g2));
    let t = g2.fmul_m(k(EIGHT)).fsub_m(k(FIVE));
    let c = x(k(P099)).fcom(t);
    if !c.test_parity(0x05) {
        // 0.99 < t
        glare.write(f32_of(x(k(P099)).fmul_m(g::<f32>(SUN_GLARE).read())));
        return;
    }
    if !x(k(ZERO)).fcom(t).test_any(0x41) {
        // t < 0
        glare.write(f32_of(x(k(ZERO)).fmul_m(g::<f32>(SUN_GLARE).read())));
        return;
    }
    // 0x6E71E7: re-tests 0.99 against t (keeps t: greater, equal or unordered)
    let t = if x(k(P099)).fcom(t).test_parity(0x05) { t } else { x(k(P099)) };
    glare.write(f32_of(t.fmul_m(g::<f32>(SUN_GLARE).read())));
}

/// `CWaterLevel::AddWaveToResult(float x, float y, float* level, float big, float small, CVector*
/// normal)` (0x6E81E0): the waves at the three grid corners (`CalculateWavesOnlyForCoordinate2` on the
/// even 2-unit grid around `(x, y) * 0.5`) interpolated over the corner's triangle and added to `*level`
/// (`(*level + interp) + w0`); with `normal` set, the normal is `Normalise(CrossProduct(a, b))` of the two
/// triangle edges ((2, 0, w1 - w0) x (0, 2, w2 - w0), or (-2, 0, w1 - w0) x (0, -2, w2 - w0) on the upper
/// triangle, chosen by `fx + fy > 1` or unordered).
#[hook(addr = 0x6E81E0, conv = cdecl, name = "CWaterLevel::AddWaveToResult", expect = "83 EC 38 D9 44 24 3C")]
pub fn add_wave_to_result(px: u32, py: u32, level: GPtr<f32>, big: u32, small: u32, normal: GPtr<u8>) {
    let xs = x(f32::from_bits(px)).fmul_m(k(HALF));
    let xs32 = f32_of(xs);
    let fx = f32_of(floor_x(xs).fsubr_m(xs32));
    let ys = x(f32::from_bits(py)).fmul_m(k(HALF));
    let ys32 = f32_of(ys);
    let fy = f32_of(floor_x(ys).fsubr_m(ys32));
    let ix = ftol32(floor_x(x(xs32)).fadd(floor_x(x(xs32))));
    let iy = ftol32(floor_x(x(ys32)).fadd(floor_x(x(ys32))));
    let upper = x(fy).fadd_m(fx).fcom(x(k(ONE))).test_parity(0x05);
    let mut w = [0f32; 3];
    let wp = |i: usize, w: &mut [f32; 3]| GPtr::from_raw(&mut w[i] as *mut f32);
    // (corner of w0, of w1, of w2)
    let (c0, c1, c2) = if upper {
        ((ix.wrapping_add(2), iy.wrapping_add(2)), (ix, iy.wrapping_add(2)), (ix.wrapping_add(2), iy))
    } else {
        ((ix, iy), (ix.wrapping_add(2), iy), (ix, iy.wrapping_add(2)))
    };
    o_calc_waves2(c0.0, c0.1, big, small, wp(0, &mut w));
    o_calc_waves2(c1.0, c1.1, big, small, wp(1, &mut w));
    o_calc_waves2(c2.0, c2.1, big, small, wp(2, &mut w));
    let d2 = x(w[2]).fsub_m(w[0]);
    let d1 = x(w[1]).fsub_m(w[0]);
    let lv = if normal.is_null() {
        if upper {
            d2.fmul(x(k(ONE)).fsub_m(fy)).fadd(d1.fmul(x(k(ONE)).fsub_m(fx)))
        } else {
            d2.fmul_m(fy).fadd(d1.fmul_m(fx))
        }
    } else if upper {
        x(k(ONE)).fsub_m(fx).fmul(d1).fadd(x(k(ONE)).fsub_m(fy).fmul(d2))
    } else {
        x(fx).fmul(d1).fadd(d2.fmul_m(fy))
    };
    level.write(f32_of(lv.fadd_m(level.read()).fadd_m(w[0])));
    if normal.is_null() {
        return;
    }
    let (a, b) = if upper {
        ([0xC000_0000, 0, f32_of(d1).to_bits()], [0, 0xC000_0000, f32_of(d2).to_bits()])
    } else {
        ([0x4000_0000, 0, f32_of(d1).to_bits()], [0, 0x4000_0000, f32_of(d2).to_bits()])
    };
    let mut a = a;
    let mut b = b;
    let mut o = [0u32; 3];
    let r = cross_product(
        GPtr::from_raw(o.as_mut_ptr().cast()),
        GPtr::from_raw(a.as_mut_ptr().cast()),
        GPtr::from_raw(b.as_mut_ptr().cast()),
    );
    for i in 0..3 {
        wr::<u32>(normal, i * 4, rd::<u32>(r, i * 4));
    }
    vector_normalise(normal);
}

/// `FUN_006e5bb0` (0x6E5BB0, `CWaterQuad` level): `(x, y)` must lie in the quad's box (x0 <= x <= x1 from
/// vertices 0 / 1, y0 <= y <= y2 from vertices 0 / 2; unordered fails). `fx = (x - x0) / (x1 - x0)`, `fy
/// = (y - y0) / (y2 - y0)` (`fidiv`); `fx + fy <= 1` interpolates over vertices 0, 1, 2, otherwise over 3
/// (with `1 - fx`, `1 - fy`). Writes `*level`, and (when `big` is set) `*big`, `*small`; with `big` null
/// it returns true at once. Otherwise false when `level - 6 > z` and flag bit 2 is set, or when `level +
/// 20 < z`.
#[hook(addr = 0x6E5BB0, conv = cdecl, name = "FUN_006e5bb0", expect = "83 EC 10 8B 4C 24 14")]
pub fn quad_level(q: GPtr<u8>, px: u32, py: u32, pz: u32, level: GPtr<f32>, big: GPtr<f32>, small: GPtr<f32>) -> u8 {
    let (fxv, fyv, fz) = (f32::from_bits(px), f32::from_bits(py), f32::from_bits(pz));
    let qi = |i: isize| rd::<i16>(q, i * 2);
    let v0 = qi(0);
    let x0i = vx(v0);
    let x0f = f32_of(xi(x0i));
    // `fld a; fcomp b; test ah, 1; jne` = a < b or unordered
    let lt = |a: X87, b: X87| a.fcom(b).test_any(0x01);
    if lt(x(fxv), x(x0f)) {
        return 0;
    }
    let v1 = qi(1);
    let x1i = vx(v1);
    if lt(xi(x1i), x(fxv)) {
        return 0;
    }
    let y0i = vy(v0);
    let y0f = f32_of(xi(y0i));
    if lt(x(fyv), x(y0f)) {
        return 0;
    }
    let v2 = qi(2);
    let y2i = vy(v2);
    if lt(xi(y2i), x(fyv)) {
        return 0;
    }
    let fx = f32_of(x(fxv).fsub_m(x0f).fdiv(xi(x1i.wrapping_sub(x0i))));
    let fy = x(fyv).fsub_m(y0f).fdiv(xi(y2i.wrapping_sub(y0i)));
    let lower = !x(fx).fadd(fy).fcom(x(k(ONE))).test_parity(0x41);
    if lower {
        let field = |off: isize| {
            let a = vf(qi(0), off);
            x(vf(qi(2), off)).fsub_m(a).fmul(fy).fadd(x(vf(qi(1), off)).fsub_m(a).fmul_m(fx)).fadd_m(a)
        };
        level.write(f32_of(field(4)));
        if big.is_null() {
            return 1;
        }
        big.write(f32_of(field(8)));
        small.write(f32_of(field(0xC)));
    } else {
        let a = x(k(ONE)).fsub(fy); // 1 - fy
        let b = x(k(ONE)).fsub_m(fx); // 1 - fx
        let z3 = vf(qi(3), 4);
        let lv = x(vf(v2, 4)).fsub_m(z3).fmul(b).fadd(x(vf(v1, 4)).fsub_m(z3).fmul(a)).fadd_m(z3);
        level.write(f32_of(lv));
        if big.is_null() {
            return 1;
        }
        let field = |off: isize| {
            let c = vf(qi(3), off);
            x(vf(qi(1), off)).fsub_m(c).fmul(a).fadd(x(vf(qi(2), off)).fsub_m(c).fmul(b)).fadd_m(c)
        };
        big.write(f32_of(field(8)));
        small.write(f32_of(field(0xC)));
    }
    level_z_test(level.read(), fz, rd::<u8>(q, 8))
}

/// The common tail of the quad / triangle level tests: false when `level - 6 > z` with flag bit 2, or
/// when `level + 20 < z`; true otherwise (unordered passes both).
fn level_z_test(lv: f32, z: f32, flags: u8) -> u8 {
    if !x(lv).fsub_m(k(SIX)).fcom(x(z)).test_any(0x41) && flags & 4 != 0 {
        return 0;
    }
    u8::from(x(lv).fadd_m(k(TWENTY)).fcom(x(z)).test_parity(0x05))
}

/// `FUN_006e5e90` (0x6E5E90, `CWaterTriangle` level): x0 <= x <= x1 (vertices 0 / 1), `min(y0, y2) <= y
/// <= max(y0, y2)` (unordered fails); `fx = (x - x0) / (x1 - x0)`, `fy = (y - y0) / (y2 - y0)`. When
/// vertex 0 and 2 share x: inside if `fx + fy <= 1`, interpolated from vertex 0; otherwise inside if `fx
/// >= fy`, interpolated from vertex 1 with `1 - fx`. Writes `*level` and (when `big` is set) `*big`,
/// `*small`; then the z test of the quad (flag word +6).
#[hook(addr = 0x6E5E90, conv = cdecl, name = "FUN_006e5e90", expect = "83 EC 10 8B 4C 24 14")]
pub fn triangle_level(t: GPtr<u8>, px: u32, py: u32, pz: u32, level: GPtr<f32>, big: GPtr<f32>, small: GPtr<f32>) -> u8 {
    let (fxv, fyv, fz) = (f32::from_bits(px), f32::from_bits(py), f32::from_bits(pz));
    let ti = |i: isize| rd::<i16>(t, i * 2);
    let v0 = ti(0);
    let x0i = vx(v0);
    let x0f = f32_of(xi(x0i));
    let lt = |a: X87, b: X87| a.fcom(b).test_any(0x01);
    if lt(x(fxv), x(x0f)) {
        return 0;
    }
    let v1 = ti(1);
    let x1i = vx(v1);
    if lt(xi(x1i), x(fxv)) {
        return 0;
    }
    let v2 = ti(2);
    let y0s = rd::<i16>(vtx(v0), 2);
    let y2s = rd::<i16>(vtx(v2), 2);
    let y0i = i32::from(y0s);
    let y2i = i32::from(y2s);
    let y0f = f32_of(xi(y0i));
    let (lo, hi) = if y0s < y2s { (xi(y0i), xi(y2i)) } else { (xi(y2i), xi(y0i)) };
    let hi = f32_of(hi);
    if lt(x(fyv), lo) {
        return 0;
    }
    if x(fyv).fcom(x(hi)).test_parity(0x41) {
        return 0;
    }
    let fx = f32_of(x(fxv).fsub_m(x0f).fdiv(xi(x1i.wrapping_sub(x0i))));
    let fy = f32_of(x(fyv).fsub_m(y0f).fdiv(xi(y2i.wrapping_sub(y0i))));
    if rd::<i16>(vtx(v0), 0) == rd::<i16>(vtx(v2), 0) {
        if x(fy).fadd_m(fx).fcom(x(k(ONE))).test_parity(0x41) {
            return 0;
        }
        let a = vf(v0, 4);
        let lv = x(vf(v2, 4)).fsub_m(a).fmul_m(fy).fadd(x(vf(v1, 4)).fsub_m(a).fmul_m(fx)).fadd_m(a);
        level.write(f32_of(lv));
        if !big.is_null() {
            let b0 = vf(ti(0), 8);
            let bv = x(vf(ti(1), 8)).fsub_m(b0).fmul_m(fx).fadd(x(vf(ti(2), 8)).fsub_m(b0).fmul_m(fy)).fadd_m(b0);
            big.write(f32_of(bv));
            let s0 = vf(ti(0), 0xC);
            let sv = x(vf(ti(2), 0xC)).fsub_m(s0).fmul_m(fy).fadd(x(vf(ti(1), 0xC)).fsub_m(s0).fmul_m(fx)).fadd_m(s0);
            small.write(f32_of(sv));
        }
    } else {
        if lt(x(fx), x(fy)) {
            return 0;
        }
        let a = x(k(ONE)).fsub_m(fx);
        let z1 = vf(v1, 4);
        let lv = x(vf(v2, 4)).fsub_m(z1).fmul_m(fy).fadd(x(vf(v0, 4)).fsub_m(z1).fmul(a)).fadd_m(z1);
        level.write(f32_of(lv));
        if !big.is_null() {
            let b1 = vf(ti(1), 8);
            let bv = x(vf(ti(0), 8)).fsub_m(b1).fmul(a).fadd(x(vf(ti(2), 8)).fsub_m(b1).fmul_m(fy)).fadd_m(b1);
            big.write(f32_of(bv));
            let s1 = vf(ti(1), 0xC);
            let sv = x(vf(ti(0), 0xC)).fsub_m(s1).fmul(a).fadd(x(vf(ti(2), 0xC)).fsub_m(s1).fmul_m(fy)).fadd_m(s1);
            small.write(f32_of(sv));
        }
    }
    level_z_test(level.read(), fz, rd::<u8>(t, 6))
}

/// `CWaterLevel::GetWaterLevelNoWaves(float x, float y, float z, float* level, float* big, float* small)`
/// (0x6E8580): block `ftol(floor(v * 0.002 + 6))` per axis; outside 0..12: `*level = 0`, `*big = 1`,
/// `*small = 0` (each when set), true. Inside: the block's quad / triangle, or its `m_PolyCombos` run
/// (quads and triangles until a kind-0 entry, kind 3 entries skipped; the first hit wins); none: false.
#[hook(addr = 0x6E8580, conv = cdecl, name = "CWaterLevel::GetWaterLevelNoWaves", expect = "51 D9 44 24 08")]
pub fn get_water_level_no_waves(px: u32, py: u32, pz: u32, level: GPtr<f32>, big: GPtr<f32>, small: GPtr<f32>) -> u8 {
    let block = |v: u32| ftol32(floor_x(x(f32::from_bits(v)).fmul_m(k(BLOCK_SCALE)).fadd_m(k(SIX))));
    let bx = block(px);
    let by = block(py);
    if !(0..12).contains(&bx) || !(0..12).contains(&by) {
        level.write(0.0);
        if !big.is_null() {
            big.cast::<u32>().write(ONE);
        }
        if !small.is_null() {
            small.cast::<u32>().write(0);
        }
        return 1;
    }
    let info = u32::from(BLOCK_POLY_INFO.ptr().cast::<u16>().wrapping_index(bx * 12 + by).read());
    let quad = |i: u32| GPtr::<u8>::from_addr(QUADS_BASE).byte_add(i as isize * 0xA);
    let tri = |i: u32| GPtr::<u8>::from_addr(TRIANGLES_BASE).byte_add(i as isize * 8);
    match info >> 14 {
        0 => 0,
        1 => u8::from(o_quad_level(quad(info & 0x3FFF), px, py, pz, level, big, small) != 0),
        2 => u8::from(o_triangle_level(tri(info & 0x3FFF), px, py, pz, level, big, small) != 0),
        _ => {
            let mut e = POLY_COMBOS.ptr().cast::<u16>().wrapping_index((info & 0x3FFF) as i32);
            while e.read() & 0xC000 != 0 {
                let v = u32::from(e.read());
                let hit = match v >> 14 {
                    1 => o_quad_level(quad(v & 0x3FFF), px, py, pz, level, big, small),
                    2 => o_triangle_level(tri(v & 0x3FFF), px, py, pz, level, big, small),
                    _ => 0,
                };
                if hit != 0 {
                    return 1;
                }
                e = e.wrapping_index(1);
            }
            0
        }
    }
}

/// `CWaterLevel::GetWaterLevel(float x, float y, float z, float* level, bool waves, CVector* normal)`
/// (0x6EB690): `GetWaterLevelNoWaves` with local big / small wave factors; false when it fails. Without
/// `waves`, a level more than 3 above z is cleared and false returned; otherwise (waves, or `level - z
/// <= 3`, or unordered) `AddWaveToResult` runs and the result is true.
#[hook(addr = 0x6EB690, conv = cdecl, name = "CWaterLevel::GetWaterLevel-", expect = "8B 54 24 0C 83 EC 08")]
pub fn get_water_level(px: u32, py: u32, pz: u32, level: GPtr<f32>, waves: u8, normal: GPtr<u8>) -> u8 {
    let mut big = 0u32;
    let mut small = 0u32;
    let bp = GPtr::from_raw(&mut big as *mut u32).cast::<f32>();
    let sp = GPtr::from_raw(&mut small as *mut u32).cast::<f32>();
    if o_get_water_level_no_waves(px, py, pz, level, bp, sp) == 0 {
        return 0;
    }
    if waves == 0 && !x(level.read()).fsub_m(f32::from_bits(pz)).fcom(x(k(THREE))).test_any(0x41) {
        level.write(0.0);
        return 0;
    }
    o_add_wave_to_result(px, py, level, big, small, normal);
    1
}

/// Flags word of a quad (+8) or triangle (+6) entry at `e`: unless bit 1 is set, bit 0 is set when
/// `(z of the first vertex > 950) == interior` (unordered counts as not above).
fn mark_one(e: GPtr<u8>, flags_off: isize, interior: u8) {
    let f = rd::<u16>(e, flags_off);
    if f & 2 != 0 {
        return;
    }
    let above = !x(vf(rd::<i16>(e, 0), 4)).fcom(x(k(MARK_Z))).test_any(0x41);
    if interior == u8::from(above) {
        wr::<u16>(e, flags_off, f | 1);
    }
}

/// `CWaterLevel::MarkQuadsAndPolysToBeRendered(int32 x, int32 y, bool interior)` (0x6E5810): marks the
/// block's quad / triangle / `m_PolyCombos` run (see `mark_one`).
#[hook(addr = 0x6E5810, conv = cdecl, name = "CWaterLevel::MarkQuadsAndPolysToBeRendered", expect = "8B 44 24 04 8B 4C 24 08")]
pub fn mark_quads_and_polys_to_be_rendered(bx: i32, by: i32, interior: u8) {
    let info = u32::from(
        BLOCK_POLY_INFO
            .ptr()
            .cast::<u16>()
            .wrapping_index(bx.wrapping_mul(12).wrapping_add(by))
            .read(),
    );
    let quad = |i: u32| GPtr::<u8>::from_addr(QUADS_BASE).byte_add(i as isize * 0xA);
    let tri = |i: u32| GPtr::<u8>::from_addr(TRIANGLES_BASE).byte_add(i as isize * 8);
    match info >> 14 {
        0 => {}
        1 => mark_one(quad(info & 0x3FFF), 8, interior),
        2 => mark_one(tri(info & 0x3FFF), 6, interior),
        _ => {
            let mut e = POLY_COMBOS.ptr().cast::<u16>().wrapping_index((info & 0x3FFF) as i32);
            while e.read() & 0xC000 != 0 {
                let v = u32::from(e.read());
                match v >> 14 {
                    1 => mark_one(quad(v & 0x3FFF), 8, interior),
                    2 => mark_one(tri(v & 0x3FFF), 6, interior),
                    _ => {}
                }
                e = e.wrapping_index(1);
            }
        }
    }
}

/// `CWaterLevel::BlockHit(int32 x, int32 y)` (0x6E6CA0, the `CWorldScan` callback): inside the 12 x 12
/// grid it marks the block (`interior = CGame::currArea != 0`); a block on or outside the grid's border
/// (`x <= 0 || y <= 0 || x >= 11 || y >= 11`) is appended to the outside-world list (max 70).
#[hook(addr = 0x6E6CA0, conv = cdecl, name = "CWaterLevel::BlockHit", expect = "56 8B 74 24 08")]
pub fn block_hit(bx: i32, by: i32) {
    if (0..12).contains(&bx) && (0..12).contains(&by) {
        o_mark_quads_and_polys(bx, by, u8::from(g::<i32>(CURR_AREA).read() != 0));
    }
    if bx <= 0 || by <= 0 || bx >= 11 || by >= 11 {
        let n = NUM_BLOCKS_OUTSIDE.get();
        if n < 70 {
            BLOCKS_OUTSIDE_X.ptr().cast::<i16>().wrapping_index(n).write(bx as i16);
            BLOCKS_OUTSIDE_Y.ptr().cast::<i16>().wrapping_index(n).write(by as i16);
            NUM_BLOCKS_OUTSIDE.set(n + 1);
        }
    }
}

/// `CWaterLevel::ScanThroughBlocks()` (0x6E6D10): clears the outside-world list, transforms the far-plane
/// corners `(-fw, fh, f)`, `(fw, fh, f)`, `(fw, -fh, f)`, `(-fw, -fh, f)` and the origin by the camera
/// frame's matrix, maps their x / y to blocks (`v * 0.002 + 6`) and rasterises the polygon with
/// `CWorldScan::ScanWorld(points, 5, BlockHit)`.
#[hook(addr = 0x6E6D10, conv = cdecl, name = "CWaterLevel::ScanThroughBlocks", expect = "83 EC 64 A1 7C F9 B6 00")]
pub fn scan_through_blocks() {
    let cam = g::<GPtr<u8>>(RW_CAMERA).read();
    NUM_BLOCKS_OUTSIDE.set(0);
    let f = rd::<f32>(cam, 0x84);
    let vw = rd::<f32>(cam, 0x68);
    let vh = rd::<f32>(cam, 0x6C);
    let mul = |m: f32, neg: bool| {
        let v = x(f).fmul_m(m);
        f32_of(if neg { v.fchs() } else { v })
    };
    let fs = f32_of(x(f));
    let mut p: [f32; 15] = [
        mul(vw, true),
        mul(vh, false),
        fs,
        mul(vw, false),
        mul(vh, false),
        fs,
        mul(vw, false),
        mul(vh, true),
        fs,
        mul(vw, true),
        mul(vh, true),
        fs,
        0.0,
        0.0,
        0.0,
    ];
    let pp = GPtr::from_raw(p.as_mut_ptr()).cast::<u8>();
    rw_v3d_transform_points(pp, pp, 5, rd::<GPtr<u8>>(cam, 4).byte_add(0x10));
    let mut a = [0f32; 10];
    for i in 0..5 {
        for j in 0..2 {
            a[i * 2 + j] = f32_of(x(p[i * 3 + j]).fmul_m(k(BLOCK_SCALE)).fadd_m(k(SIX)));
        }
    }
    world_scan_scan_world(GPtr::from_raw(a.as_mut_ptr()).cast(), 5, 0x6E6CA0);
}

/// `FUN_006ea8a0` = `CWaterLevel::GetGroundLevel(const CVector& p, float* z, ColData* col, float offset)`
/// (0x6EA8A0): `CWorld::ProcessVerticalLine((p.x, p.y, p.z + offset), -offset, ...)` against buildings
/// and dummies; on a hit `*z` = the colpoint's z and (when `col` is set) `col[0..2]` = colpoint bytes
/// +0x23 / +0x24.
#[hook(addr = 0x6EA8A0, conv = cdecl, name = "FUN_006ea8a0", expect = "83 EC 38 8B 44 24 3C")]
pub fn get_ground_level(p: GPtr<f32>, out: GPtr<f32>, col: GPtr<u8>, offset: u32) -> u8 {
    let off = f32::from_bits(offset);
    let pu = p.cast::<u32>();
    let mut pt = [
        pu.read(),
        f32_of(x(p.wrapping_index(1).read())).to_bits(),
        f32_of(x(off).fadd_m(p.wrapping_index(2).read())).to_bits(),
    ];
    let mut cp = [0u8; 0x2C];
    let mut ent = GPtr::<u8>::NULL;
    let hit = process_vertical_line(
        GPtr::from_raw(pt.as_mut_ptr()).cast(),
        f32_of(x(off).fchs()).to_bits(),
        GPtr::from_raw(cp.as_mut_ptr()),
        GPtr::from_raw(&mut ent),
        1,
        0,
        0,
        0,
        1,
        0,
        0,
    );
    if hit == 0 {
        return 0;
    }
    out.write(f32_of(x(f32::from_le_bytes([cp[8], cp[9], cp[10], cp[11]]))));
    if !col.is_null() {
        col.write(cp[0x23]);
        col.byte_add(1).write(cp[0x24]);
    }
    1
}

/// `CWaterLevel::GetWaterDepth(const CVector& p, float* depth, float* waterLevel, float* groundZ)`
/// (0x6EA960): false when `GetWaterLevelNoWaves` fails; the ground is `GetGroundLevel(p, NULL, 30)` or
/// -100; `*depth = level - ground`, `*waterLevel = level`, `*groundZ = ground` (each when set).
#[hook(addr = 0x6EA960, conv = cdecl, name = "CWaterLevel::GetWaterDepth", expect = "51 56 8B 74 24 0C")]
pub fn get_water_depth(p: GPtr<f32>, depth: GPtr<f32>, water: GPtr<f32>, ground: GPtr<f32>) -> u8 {
    let mut level = 0u32;
    let lp = GPtr::from_raw(&mut level as *mut u32).cast::<f32>();
    let pu = p.cast::<u32>();
    if o_get_water_level_no_waves(
        pu.read(),
        pu.wrapping_index(1).read(),
        pu.wrapping_index(2).read(),
        lp,
        GPtr::NULL,
        GPtr::NULL,
    ) == 0
    {
        return 0;
    }
    let mut gz = 0u32;
    let gp = GPtr::from_raw(&mut gz as *mut u32).cast::<f32>();
    let gv = if o_get_ground_level(p, gp, GPtr::NULL, GROUND_PROBE) != 0 {
        x(f32::from_bits(gz))
    } else {
        x(k(NO_GROUND))
    };
    if !depth.is_null() {
        depth.write(f32_of(x(f32::from_bits(level)).fsub(gv)));
    }
    if !water.is_null() {
        water.cast::<u32>().write(level);
    }
    if !ground.is_null() {
        ground.write(f32_of(gv));
    }
    1
}

/// CRT `ceil` (0x823820) on a double (exact).
fn ceil_f64(v: f64) -> f64 {
    -floor_f64(-v)
}

/// `CWaterLevel::SetCameraRange()` (0x6E9C80): around TheCamera's position, `MinX = floor((x - D) * 0.5)
/// * 2`, `MaxX = ceil((D + x) * 0.5) * 2`, likewise y (`D = DETAILEDWATERDIST`; `_ftol`, `shl 1`).
#[hook(addr = 0x6E9C80, conv = cdecl, name = "CWaterLevel::SetCameraRange", expect = "A1 3C F0 B6 00")]
pub fn set_camera_range() {
    let pos = || {
        let m = g::<GPtr<u8>>(0xB6F03C).read();
        if m.is_null() { g::<u8>(0xB6F02C) } else { m.byte_add(0x30) }
    };
    let d = || xi(DETAILED_WATER_DIST.get());
    let lo = |v: X87| ftol32(floor_x(v.fmul_m(k(HALF)))).wrapping_shl(1);
    let hi = |v: X87| ftol32(X87::from_f64_exact(ceil_f64(v.fmul_m(k(HALF)).to_f64()))).wrapping_shl(1);
    CAMERA_RANGE_MIN_X.set(lo(d().fsubr_m(rd::<f32>(pos(), 0))));
    CAMERA_RANGE_MAX_X.set(hi(d().fadd_m(rd::<f32>(pos(), 0))));
    CAMERA_RANGE_MIN_Y.set(lo(d().fsubr_m(rd::<f32>(pos(), 4))));
    CAMERA_RANGE_MAX_Y.set(hi(d().fadd_m(rd::<f32>(pos(), 4))));
}

/// -3000.0 (0x859A90), 3000.0 (0x859A94)
const MIN_COORD: u32 = 0xC53B_8000;
const MAX_COORD: u32 = 0x453B_8000;

/// The x or y an added vertex stores: `<= -3000` gives -3000, `>= 3000` gives 3000 (each also resets the
/// CRenPar to z 0, big 1, small 0, flow 0), truncated to i16.
fn clamp_coord(v: i32, p: &mut [u32; 4]) -> i32 {
    let mut v = v;
    if !xi(v).fcom(x(k(MIN_COORD))).test_parity(0x41) {
        v = -3000;
        *p = [0, ONE, 0, p[3] & 0xFFFF_0000];
    }
    if !xi(v).fcom(x(k(MAX_COORD))).test_any(0x01) {
        v = 3000;
        *p = [0, ONE, 0, p[3] & 0xFFFF_0000];
    }
    v
}

/// `CWaterLevel::AddWaterLevelVertex(int32 x, int32 y, CRenPar p)` (0x6E5A40, EAX): x / y clamped to
/// +-3000 (resetting the CRenPar, see `clamp_coord`); an existing vertex with the same x, y (i16) and an
/// ordered-equal z is reused, otherwise the vertex is appended (`NumWaterVertices++`, no bounds check).
#[hook(addr = 0x6E5A40, conv = cdecl, name = "CWaterLevel::AddWaterLevelVertex", expect = "DB 44 24 04 53")]
pub fn add_water_level_vertex(vx0: i32, vy0: i32, z: u32, big: u32, small: u32, flow: u32) -> i32 {
    let mut p = [z, big, small, flow];
    let cx = clamp_coord(vx0, &mut p);
    let cy = clamp_coord(vy0, &mut p);
    let n = NUM_WATER_VERTICES.get() as i32;
    let mut i = 0;
    while i < n {
        let v = vtx(i as i16);
        if i32::from(rd::<i16>(v, 0)) == cx
            && i32::from(rd::<i16>(v, 2)) == cy
            && !x(rd::<f32>(v, 4)).fcom(x(f32::from_bits(p[0]))).test_parity(0x44)
        {
            return i;
        }
        i += 1;
    }
    let v = GPtr::<u8>::from_addr(VERTICES_BASE).byte_add(n as isize * 0x14);
    wr::<i16>(v, 0, cx as i16);
    wr::<i16>(v, 2, cy as i16);
    for (j, w) in p.iter().enumerate() {
        wr::<u32>(v, 4 + j as isize * 4, *w);
    }
    NUM_WATER_VERTICES.set(n.wrapping_add(1) as u32);
    n
}

/// The flags word of a new quad / triangle: bit 0 cleared, bit 1 = `!(flags & 1)`, bit 2 = `flags & 2`.
fn store_poly_flags(e: GPtr<u8>, off: isize, flags: u8) {
    wr::<u8>(e, off, rd::<u8>(e, off) & 0xFE);
    let w = rd::<u16>(e, off);
    wr::<u16>(e, off, if flags & 1 != 0 { w & !2 } else { w | 2 });
    let b = rd::<u8>(e, off);
    wr::<u8>(e, off, if flags & 2 != 0 { b | 4 } else { b & !4 });
}

/// `CWaterLevel::AddWaterLevelTriangle(X1, Y1, P1, X2, Y2, P2, X3, Y3, P3, flags)` (0x6E7D40): nothing
/// when the three X or the three Y are equal; otherwise the three vertices (in order) and a triangle
/// ordered by the stored coordinates: `y0 == y1`: (lower x of v0 / v1, the other, v2); `y0 == y2`: (lower
/// x of v0 / v2, the other, v1); else (lower x of v1 / v2, the other, v0) (`jge`: on equal x the second
/// comes first); flags as `store_poly_flags`; `NumWaterTriangles++`.
#[hook(addr = 0x6E7D40, conv = cdecl, name = "CWaterLevel::AddWaterLevelTriangle", expect = "8B 4C 24 04 53")]
#[allow(clippy::too_many_arguments)]
pub fn add_water_level_triangle(
    x1: i32, y1: i32, p1z: u32, p1b: u32, p1s: u32, p1f: u32,
    x2: i32, y2: i32, p2z: u32, p2b: u32, p2s: u32, p2f: u32,
    x3: i32, y3: i32, p3z: u32, p3b: u32, p3s: u32, p3f: u32,
    flags: u32,
) {
    if x1 == x2 && x1 == x3 {
        return;
    }
    if y1 == y2 && y1 == y3 {
        return;
    }
    let v0 = o_add_vertex(x1, y1, p1z, p1b, p1s, p1f);
    let v1 = o_add_vertex(x2, y2, p2z, p2b, p2s, p2f);
    let v2 = o_add_vertex(x3, y3, p3z, p3b, p3s, p3f);
    let sx = |v: i32| rd::<i16>(vtx(v as i16), 0);
    let sy = |v: i32| rd::<i16>(vtx(v as i16), 2);
    let (a, b, c) = if sy(v0) == sy(v1) {
        if sx(v0) < sx(v1) { (v0, v1, v2) } else { (v1, v0, v2) }
    } else if sy(v0) == sy(v2) {
        if sx(v0) < sx(v2) { (v0, v2, v1) } else { (v2, v0, v1) }
    } else if sx(v1) < sx(v2) {
        (v1, v2, v0)
    } else {
        (v2, v1, v0)
    };
    let n = NUM_WATER_TRIANGLES.get();
    let t = GPtr::<u8>::from_addr(TRIANGLES_BASE).byte_add(n as isize * 8);
    wr::<u16>(t, 0, a as u16);
    wr::<u16>(t, 2, b as u16);
    wr::<u16>(t, 4, c as u16);
    store_poly_flags(t, 6, flags as u8);
    NUM_WATER_TRIANGLES.set(n.wrapping_add(1));
}

/// `CWaterLevel::AddWaterLevelQuad(X1, Y1, P1, .., X4, Y4, P4, flags)` (0x6E7EF0): nothing when the four
/// X or the four Y are equal; otherwise the four vertices (in order), then the original's partition: the
/// vertices at the minimum stored y go to list A, the others to list B (two slots each, written in vertex
/// order), each list's pair ordered by x (swapped only when `x[0] > x[1]`); quad = (A0, A1, B0, B1),
/// flags as for triangles, `NumWaterQuads++`. Ported for the 2 + 2 split only; with 1 or 3 vertices on
/// the minimum line the original reads unwritten stack slots (list overflow), so that case runs the
/// original body (`fallback`, decided from the clamped Y arguments before any vertex is added).
#[hook(addr = 0x6E7EF0, conv = cdecl, name = "CWaterLevel::AddWaterLevelQuad", expect = "8B 4C 24 04 8B 44 24 1C", fallback)]
#[allow(clippy::too_many_arguments)]
pub fn add_water_level_quad(
    x1: i32, y1: i32, p1z: u32, p1b: u32, p1s: u32, p1f: u32,
    x2: i32, y2: i32, p2z: u32, p2b: u32, p2s: u32, p2f: u32,
    x3: i32, y3: i32, p3z: u32, p3b: u32, p3s: u32, p3f: u32,
    x4: i32, y4: i32, p4z: u32, p4b: u32, p4s: u32, p4f: u32,
    flags: u32,
) {
    if x1 == x2 && x1 == x3 && x1 == x4 {
        return;
    }
    if y1 == y2 && y1 == y3 && y1 == y4 {
        return;
    }
    let mut scratch = [0u32; 4];
    let ys = [y1, y2, y3, y4].map(|y| clamp_coord(y, &mut scratch) as i16);
    let min = ys.iter().copied().min().unwrap_or(0);
    if ys.iter().filter(|&&y| y == min).count() != 2 {
        add_water_level_quad_original(
            x1, y1, p1z, p1b, p1s, p1f, x2, y2, p2z, p2b, p2s, p2f, x3, y3, p3z, p3b, p3s, p3f, x4, y4, p4z, p4b,
            p4s, p4f, flags,
        );
        return;
    }
    let v = [
        o_add_vertex(x1, y1, p1z, p1b, p1s, p1f),
        o_add_vertex(x2, y2, p2z, p2b, p2s, p2f),
        o_add_vertex(x3, y3, p3z, p3b, p3s, p3f),
        o_add_vertex(x4, y4, p4z, p4b, p4s, p4f),
    ];
    let sx = |v: i32| rd::<i16>(vtx(v as i16), 0);
    let sy = |v: i32| rd::<i16>(vtx(v as i16), 2);
    let min_y = v.iter().map(|&i| sy(i)).min().unwrap_or(0);
    let mut a = [0i32; 2];
    let mut b = [0i32; 2];
    let (mut na, mut nb) = (0, 0);
    for &i in &v {
        if sy(i) == min_y {
            a[na] = i;
            na += 1;
        } else {
            b[nb] = i;
            nb += 1;
        }
    }
    if sx(a[0]) > sx(a[1]) {
        a.swap(0, 1);
    }
    if sx(b[0]) > sx(b[1]) {
        b.swap(0, 1);
    }
    let n = NUM_WATER_QUADS.get();
    let q = GPtr::<u8>::from_addr(QUADS_BASE).byte_add(n as isize * 0xA);
    wr::<u16>(q, 0, a[0] as u16);
    wr::<u16>(q, 2, a[1] as u16);
    wr::<u16>(q, 4, b[0] as u16);
    wr::<u16>(q, 6, b[1] as u16);
    store_poly_flags(q, 8, flags as u8);
    NUM_WATER_QUADS.set(n.wrapping_add(1));
}

/// `FUN_006e5750(int32 x, int32 y, int32 index, int32 kind)` (0x6E5750, `FillQuadsAndTrianglesList`'s
/// helper): an empty block takes `kind << 14 | index`; a block holding one quad / triangle becomes a new
/// `m_PolyCombos` run (old entry, new entry, 0; `NumWaterZonePolys += 3`, block = `start | 0xC000`); a
/// block that already has a run (kind 3) is assumed to own the last one: the new entry overwrites its
/// terminator and a new terminator follows (`NumWaterZonePolys += 1`).
#[hook(addr = 0x6E5750, conv = cdecl, name = "FUN_006e5750", expect = "8B 44 24 04 8B 4C 24 08")]
pub fn add_to_block(bx: i32, by: i32, idx: i32, kind: i32) {
    let cell = BLOCK_POLY_INFO.ptr().cast::<u16>().wrapping_index(bx.wrapping_mul(12).wrapping_add(by));
    let combos = POLY_COMBOS.ptr().cast::<u16>();
    let c = cell.read();
    let entry = (kind.wrapping_shl(14) | idx) as u16;
    match c >> 14 {
        0 => cell.write(entry),
        1 | 2 => {
            let n = NUM_WATER_ZONE_POLYS.get();
            combos.wrapping_index(n).write(c);
            combos.wrapping_index(n.wrapping_add(1)).write(entry);
            combos.wrapping_index(n.wrapping_add(2)).write(0);
            NUM_WATER_ZONE_POLYS.set(n.wrapping_add(3));
            cell.write((n | 0xC000) as u16);
        }
        _ => {
            let n = NUM_WATER_ZONE_POLYS.get();
            combos.wrapping_index(n.wrapping_sub(1)).write(entry);
            combos.wrapping_index(n).write(0);
            NUM_WATER_ZONE_POLYS.set(n.wrapping_add(1));
        }
    }
}

/// `CWaterLevel::FillQuadsAndTrianglesList()` (0x6E7B30): for every 500-unit block `(bx, by)` of the 12 x
/// 12 grid (block box `[x0, x0 + 500) x [y0, y0 + 500)`, `x0 = bx * 500 - 3000`), every quad with `x(v1)
/// > x0`, `x(v0) < x1`, `y(v2) > y0`, `y(v0) < y1` and every triangle with `x(v1) > x0`, `x(v0) < x1`,
/// `max(y(v0), y(v2)) > y0`, `min(..) < y1` is added to the block (`FUN_006e5750`, kind 1 / 2). The quad
/// loop re-reads `NumWaterQuads`, the triangle loop reads `NumWaterTriangles` once per block.
#[hook(addr = 0x6E7B30, conv = cdecl, name = "CWaterLevel::FillQuadsAndTrianglesList", expect = "83 EC 18 53 55")]
pub fn fill_quads_and_triangles_list() {
    let gt = |a: X87, b: X87| !a.fcom(b).test_any(0x41); // `test ah, 0x41; jne` not taken
    let lt = |a: X87, b: X87| !a.fcom(b).test_parity(0x05); // `test ah, 5; jp` not taken
    let mut bx = 0;
    let mut xc = 0;
    while xc < 6000 {
        let x0 = xi(xc).fsub_m(k(MAX_COORD));
        let x1 = x(k(BLOCK_SIZE)).fadd(x0);
        let mut by = 0;
        let mut yc = 0;
        while yc < 6000 {
            let yv = xi(yc).fsub_m(k(MAX_COORD));
            let y0 = f32_of(yv);
            let y1 = f32_of(yv.fadd_m(k(BLOCK_SIZE)));
            let mut i = 0;
            while i < NUM_WATER_QUADS.get() as i32 {
                let q = GPtr::<u8>::from_addr(QUADS_BASE).byte_add(i as isize * 0xA);
                let qi = |j: isize| rd::<i16>(q, j * 2);
                if gt(xi(vx(qi(1))), x0)
                    && lt(xi(vx(qi(0))), x1)
                    && gt(xi(vy(qi(2))), x(y0))
                    && lt(xi(vy(qi(0))), x(y1))
                {
                    o_add_to_block(bx, by, i, 1);
                }
                i += 1;
            }
            let nt = NUM_WATER_TRIANGLES.get() as i32;
            let mut i = 0;
            while i < nt {
                let t = GPtr::<u8>::from_addr(TRIANGLES_BASE).byte_add(i as isize * 8);
                let ti = |j: isize| rd::<i16>(t, j * 2);
                let ya = rd::<i16>(vtx(ti(0)), 2);
                let yb = rd::<i16>(vtx(ti(2)), 2);
                let lo = f32_of(xi(i32::from(if ya < yb { ya } else { yb })));
                let hi = if ya > yb { ya } else { yb };
                if gt(xi(vx(ti(1))), x0)
                    && lt(xi(vx(ti(0))), x1)
                    && gt(xi(i32::from(hi)), x(y0))
                    && lt(x(lo), x(y1))
                {
                    o_add_to_block(bx, by, i, 2);
                }
                i += 1;
            }
            yc += 500;
            by += 1;
        }
        xc += 500;
        bx += 1;
    }
}

/// One axis of the flow update: `d = desired - current`; `|d| < step` (`fchs` unless `d >= 0` or
/// unordered) snaps current to desired (raw copy), otherwise current moves by `step` towards desired
/// (`+` when `d >= 0` or unordered).
fn flow_axis(i: i32, step: X87) {
    let cur = CURRENT_FLOW.ptr().cast::<u32>().wrapping_index(i);
    let des = CURRENT_DESIRED_FLOW.ptr().cast::<u32>().wrapping_index(i);
    let d = x(f32::from_bits(des.read())).fsub_m(f32::from_bits(cur.read()));
    let nonneg = d.fcom(x(k(ZERO))).test_parity(0x05);
    let ad = if nonneg { d } else { d.fchs() };
    if !ad.fcom(step).test_parity(0x05) {
        cur.write(des.read());
        return;
    }
    let c = x(f32::from_bits(cur.read()));
    let c = if nonneg { c.fadd(step) } else { c.fsub(step) };
    cur.write(f32_of(c).to_bits());
}

/// `CWaterLevel::PreRenderWater()` (0x6EB710): when `CGame::CanSeeWaterFromCurrArea`: `ScanThroughBlocks`,
/// `FindNearestWaterAndItsFlow` every 32nd frame (`m_FrameCounter & 0x1F == 0x1D`), the current flow moves
/// towards the desired flow by `CTimer::ms_fTimeStep * 0.001` per axis, and the tail (0x6EAB50) stores
/// the player's position into a function-static CVector on the first call (guard bit 0 of 0xC278F0).
#[hook(addr = 0x6EB710, conv = cdecl, name = "CWaterLevel::PreRenderWater", expect = "E8 9B 0D E5 FF")]
pub fn pre_render_water() {
    if can_see_water_from_curr_area() == 0 {
        return;
    }
    o_scan_through_blocks();
    if g::<u32>(FRAME_COUNTER).read() & 0x1F == 0x1D {
        o_find_nearest_water_and_its_flow();
    }
    let step = x(g::<f32>(TIME_STEP).read()).fmul_m(k(FLOW_RATE));
    flow_axis(0, step);
    flow_axis(1, step);
    let ped = find_player_ped(-1);
    let m = rd::<GPtr<u8>>(ped, 0x14);
    let pos = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
    let guard = PRE_RENDER_STATIC_GUARD.get();
    if guard & 1 == 0 {
        PRE_RENDER_STATIC_GUARD.set(guard | 1);
        PRE_RENDER_STATIC_POS.set([rd::<u32>(pos, 0), rd::<u32>(pos, 4), rd::<u32>(pos, 8)]);
    }
}

/// `CWaterLevel::FindNearestWaterAndItsFlow()` (0x6E9D70: a `jmp` to the protection chunk 0x406B2A, which
/// loads `[0xB6F03C]` (TheCamera's matrix, `[0x54A2C1] ^ [0x1560844]`) and continues at 0x6E9D75). Around
/// TheCamera's position (outside `(-3000, 3000)` on either axis: both camera fields 0xB6F10C / 0xB6F110
/// and the desired flow are cleared): over every quad, the distance from the camera to the quad's box
/// (per axis 0 inside, else the gap; `sqrt(dy * dy + dx * dx)`). A quad nearer than the nearest wavy one
/// (any vertex with big or small waves != 0, or unordered) becomes it (distance, z of vertex 0). A quad
/// nearer than the nearest one overall becomes it and sets the desired flow to the flow bytes * (1/64)
/// of its vertex nearest to the camera (first strictly smallest squared distance, order 0..3). Writes
/// 0xB6F10C = nearest wavy distance (1e7 when none), 0xB6F110 = its z (0 when none).
#[hook(addr = 0x6E9D70, conv = cdecl, name = "CWaterLevel::FindNearestWaterAndItsFlow", expect = "E9 B5 CD D1 FF 83")]
pub fn find_nearest_water_and_its_flow() {
    let m = g::<GPtr<u8>>(0xB6F03C).read();
    let pos = if m.is_null() { g::<u8>(0xB6F02C) } else { m.byte_add(0x30) };
    let px = rd::<f32>(pos, 0);
    let py = rd::<f32>(pos, 4);
    let outside = |v: f32| {
        !x(v).fcom(x(k(MIN_COORD))).test_parity(0x41) || !x(v).fcom(x(k(MAX_COORD))).test_any(0x01)
    };
    if outside(px) || outside(py) {
        g::<u32>(CAM_DIST_TO_WATER).write(0);
        g::<u32>(CAM_HEIGHT_OF_WATER).write(0);
        CURRENT_DESIRED_FLOW.ptr().cast::<u32>().wrapping_index(1).write(0);
        CURRENT_DESIRED_FLOW.ptr().cast::<u32>().write(0);
        return;
    }
    let mut wavy = k(NEAREST_START);
    let mut any = k(NEAREST_START);
    let mut wavy_z = 0u32;
    let n = NUM_WATER_QUADS.get() as i32;
    let mut i = 0;
    while i < n {
        let q = GPtr::<u8>::from_addr(QUADS_BASE).byte_add(i as isize * 0xA);
        let qi = |j: isize| rd::<i16>(q, j * 2);
        let v0 = qi(0);
        let x0f = f32_of(xi(vx(v0)));
        // `fld p; fcomp lo; test ah, 5; jp` not taken = p < lo
        let dx = if !x(px).fcom(x(x0f)).test_parity(0x05) {
            x(x0f).fsub_m(px)
        } else {
            let x1 = xi(vx(qi(1)));
            if x(px).fcom(x1).test_any(0x41) { x(k(ZERO)) } else { x1.fsubr_m(px) }
        };
        let y0f = f32_of(xi(vy(v0)));
        let dy = if !x(py).fcom(x(y0f)).test_parity(0x05) {
            x(y0f).fsub_m(py)
        } else {
            let y2 = xi(vy(qi(2)));
            if x(py).fcom(y2).test_any(0x41) { x(k(ZERO)) } else { y2.fsubr_m(py) }
        };
        let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
        if !d.fcom(x(wavy)).test_parity(0x05) {
            // `fcomp 0; test ah, 0x44; jp` = not equal to 0 (or unordered)
            let wavy_vertex = (0..4).any(|j| {
                let v = qi(j);
                x(vf(v, 8)).fcom(x(k(ZERO))).test_parity(0x44) || x(vf(v, 0xC)).fcom(x(k(ZERO))).test_parity(0x44)
            });
            if wavy_vertex {
                wavy_z = rd::<u32>(vtx(v0), 4);
                wavy = f32_of(d);
            }
        }
        if !d.fcom(x(any)).test_parity(0x05) {
            any = f32_of(d);
            let sq = |a: X87, b: X87| a.fmul(a).fadd(b.fmul(b));
            let d0 = sq(x(px).fsub_m(x0f), x(py).fsub_m(y0f));
            let dist = |j: isize| {
                let v = qi(j);
                f32_of(sq(xi(vx(v)).fsubr_m(px), xi(vy(v)).fsubr_m(py)))
            };
            let (d1, d2, d3) = (dist(1), dist(2), dist(3));
            let lt = |a: X87, b: f32| !a.fcom(x(b)).test_parity(0x05);
            let pick = if lt(d0, d1) && lt(d0, d2) && lt(d0, d3) {
                qi(0)
            } else if lt(x(d1), d2) && lt(x(d1), d3) {
                qi(1)
            } else if lt(x(d2), d3) {
                qi(2)
            } else {
                qi(3)
            };
            let flow = |o: isize| f32_of(xi(i32::from(rd::<i8>(vtx(pick), o))).fmul_m(k(FLOW_SCALE)));
            let fx = flow(0x10);
            let fy = flow(0x11);
            CURRENT_DESIRED_FLOW.set([fx, fy]);
        }
        i += 1;
    }
    g::<f32>(CAM_DIST_TO_WATER).write(wavy);
    g::<u32>(CAM_HEIGHT_OF_WATER).write(wavy_z);
}

/// `CWaterLevel::Shutdown()` (0x6E59E0): `RwTextureDestroy` and clear each of `texWaterclear256`,
/// `texSeabd32`, `texWaterwake` that is set (the raster globals are left).
#[hook(addr = 0x6E59E0, conv = cdecl, name = "CWaterLevel::Shutdown", expect = "A1 AC 28 C2 00")]
pub fn shutdown() {
    for t in [&TEX_WATERCLEAR256, &TEX_SEABD32, &TEX_WATERWAKE] {
        let v = t.get();
        if v != 0 {
            rw_texture_destroy(GPtr::from_addr(v as usize));
            t.set(0);
        }
    }
}

/// `FUN_006e8550(int32 x, int32 y, float* out, float big, float small)` (0x6E8550):
/// `CalculateWavesOnlyForCoordinate2(x, y, big, small, out)`.
#[hook(addr = 0x6E8550, conv = cdecl, name = "FUN_006e8550", expect = "8B 44 24 0C 8B 4C 24 14")]
pub fn waves_for_coordinate(x0: i32, y0: i32, out: GPtr<f32>, big: u32, small: u32) {
    o_calc_waves2(x0, y0, big, small, out);
}

/// `CWaterLevel::SetUpWaterFog(int32 x1, int32 y1, int32 x2, int32 y2)` (0x6EA9F0: a `jmp` to the
/// protection chunk 0x403AF9, `al = m_bWaterFog`, back at 0x6EA9F5): with water fog on and fewer than 70
/// entries: the water level at `(x1, y1, 0)` (`GetWaterLevelNoWaves`; 0 when it fails, and no entry when
/// it succeeds with both wave factors 0); the player is inside when `level + m_fWaterFogHeight > z` and
/// `x1 <= x <= x2`, `y1 <= y <= y2` (sets `gbPlayerIsInsideWaterFog`); the entry stores the four
/// coordinates (i16) and the level.
/// Ported, not hooked: protection code reads the stub's jump displacement (`sub ecx, [0x6EA9F1]` at
/// 0x4014C9), so the entry cannot be patched; callers reach it through `orig!`.
pub fn set_up_water_fog(x1: i32, y1: i32, x2: i32, y2: i32) {
    if WATER_FOG.get() == 0 || WATER_FOG_INDEX.get() >= 70 {
        return;
    }
    let mut pc = [0f32; 3];
    sa_world::world::common::find_player_coors(GPtr::from_raw(pc.as_mut_ptr()), -1);
    let x1f = f32_of(xi(x1));
    let y1f = f32_of(xi(y1));
    let (mut level, mut big, mut small) = (0f32, 0f32, 0f32);
    let ok = o_get_water_level_no_waves(
        x1f.to_bits(),
        y1f.to_bits(),
        0,
        GPtr::from_raw(&mut level),
        GPtr::from_raw(&mut big),
        GPtr::from_raw(&mut small),
    );
    let lv = if ok == 0 {
        x(k(ZERO))
    } else {
        let nz = |v: f32| x(v).fcom(x(k(ZERO))).test_parity(0x44);
        if !nz(big) && !nz(small) {
            return;
        }
        x(level)
    };
    let above = !x(f32::from_bits(WATER_FOG_HEIGHT.get())).fadd(lv).fcom(x(pc[2])).test_any(0x41);
    let in_x = !x(pc[0]).fcom(x(x1f)).test_any(0x01) && !xi(x2).fcom(x(pc[0])).test_any(0x01);
    let in_y = !x(pc[1]).fcom(x(y1f)).test_any(0x01) && !xi(y2).fcom(x(pc[1])).test_any(0x01);
    if above && in_x && in_y {
        PLAYER_INSIDE_WATER_FOG.set(1);
    }
    let i = WATER_FOG_INDEX.get();
    let d = WATER_FOG_DATA.ptr().cast::<u8>();
    let w = |base: isize, v: i32| wr::<i16>(d, base + i as isize * 2, v as i16);
    w(0, x1);
    w(0x8C, y1);
    w(0x118, x2);
    w(0x1A4, y2);
    wr::<f32>(d, 0x230 + i as isize * 4, f32_of(lv));
    WATER_FOG_INDEX.set(i + 1);
}

/// `TempBufferVerticesStored`, `TempBufferIndicesStored`, `aTempBufferIndices`, `TempVertexBuffer`
/// (RxObjSpace3DVertex, 0x24 bytes: +0 pos, +0xC normal, +0x18 colour, +0x1C u, +0x20 v)
const TEMP_VERTICES_STORED: usize = 0xC4B950;
const TEMP_INDICES_STORED: usize = 0xC4B954;
const TEMP_INDICES: usize = 0xC4B958;
const TEMP_VERTICES: usize = 0xC4D958;
/// -70.0 (immediate), the sea bed colour 0xFF505050
const SEA_BED_Z: u32 = 0xC28C_0000;
const SEA_BED_COLOUR: u32 = 0xFF50_5050;
/// 4.0 (0x858B90)
const FOUR: u32 = 0x4080_0000;

/// `FUN_006e6870` = `CWaterLevel::RenderSeaBedSegment(int32 x, int32 y, float x0, float x1, float y0,
/// float y1)` (0x6E6870): one quad at z = -70 (colour 0xFF505050, normals untouched) appended to the
/// temp buffer: corners `((x + fx) * 500 - 3000, (fy + y) * 500 - 3000)` for fx in (x0, x1), fy in (y0,
/// y1) in the order (x0, y0), (x0, y1), (x1, y0), (x1, y1), uv = 8 * the fractions; indices (n, n+1,
/// n+2, n+3, n+1, n+2); no bounds check.
#[hook(addr = 0x6E6870, conv = cdecl, name = "FUN_006e6870", expect = "8B 0D 50 B9 C4 00")]
pub fn render_sea_bed_segment(bx: i32, by: i32, fx0: f32, fx1: f32, fy0: f32, fy1: f32) {
    let n = g::<i32>(TEMP_VERTICES_STORED).read();
    let v = |i: i32| g::<u8>(TEMP_VERTICES).byte_add((n.wrapping_add(i) as isize).wrapping_mul(0x24));
    let eight = |a: f32| f32_of(x(a).fmul_m(k(EIGHT)));
    let world = |c: X87| f32_of(c.fmul_m(k(BLOCK_SIZE)).fsub_m(k(MAX_COORD)));
    let xf = f32_of(xi(bx));
    let x0 = world(xi(bx).fadd_m(fx0));
    let y0 = world(x(fy0).fadd(xi(by)));
    let y1 = world(xi(by).fadd_m(fy1));
    let x1 = world(x(xf).fadd_m(fx1));
    let (u0, u1, v0, v1) = (eight(fx0), eight(fx1), eight(fy0), eight(fy1));
    for (i, (px, py, u, w)) in [(x0, y0, u0, v0), (x0, y1, u0, v1), (x1, y0, u1, v0), (x1, y1, u1, v1)]
        .into_iter()
        .enumerate()
    {
        let p = v(i as i32);
        wr::<f32>(p, 0, px);
        wr::<f32>(p, 4, py);
        wr::<u32>(p, 8, SEA_BED_Z);
        wr::<u32>(p, 0x18, SEA_BED_COLOUR);
        wr::<f32>(p, 0x1C, u);
        wr::<f32>(p, 0x20, w);
    }
    let m = g::<i32>(TEMP_INDICES_STORED).read();
    let ix = g::<u16>(TEMP_INDICES).wrapping_index(m);
    for (j, d) in [0, 1, 2, 3, 1, 2].into_iter().enumerate() {
        ix.wrapping_index(j as i32).write(n.wrapping_add(d) as u16);
    }
    g::<i32>(TEMP_VERTICES_STORED).write(n.wrapping_add(4));
    g::<i32>(TEMP_INDICES_STORED).write(m.wrapping_add(6));
}

/// Appends the four sea-bed vertices `(x0, y0) (x0, y1) (x1, y0) (x1, y1)` (z -70, colour 0xFF505050,
/// uv as given) at temp vertex `n` and the indices (n, n+1, n+2, n+3, n+1, n+2) at index `m`.
fn sea_bed_quad(n: i32, m: i32, xs: [f32; 2], ys: [f32; 2], us: [f32; 2], vs: [f32; 2]) {
    let v = |i: i32| g::<u8>(TEMP_VERTICES).byte_add((n.wrapping_add(i) as isize).wrapping_mul(0x24));
    for (i, (a, b)) in [(0, 0), (0, 1), (1, 0), (1, 1)].into_iter().enumerate() {
        let p = v(i as i32);
        wr::<f32>(p, 0, xs[a]);
        wr::<f32>(p, 4, ys[b]);
        wr::<u32>(p, 8, SEA_BED_Z);
        wr::<u32>(p, 0x18, SEA_BED_COLOUR);
        wr::<f32>(p, 0x1C, us[a]);
        wr::<f32>(p, 0x20, vs[b]);
    }
    let ix = g::<u16>(TEMP_INDICES).wrapping_index(m);
    for (j, d) in [0, 1, 2, 3, 1, 2].into_iter().enumerate() {
        ix.wrapping_index(j as i32).write(n.wrapping_add(d) as u16);
    }
}

/// `FUN_006e6a10` = `CWaterLevel::RenderDetailedSeaBedSegment(int32 x, int32 y, float x0, float x1, float
/// y0, float y1)` (0x6E6A10): the sea-bed segment split into `nx = max(ftol((x1 - x0) * 4), 1)` by `ny =
/// max(ftol((y1 - y0) * 4), 1)` quads (each as `RenderSeaBedSegment`): column `i` spans `fa = i * dX / nx
/// + x0` (kept unrounded for its x and u) to `fb = (i + 1) * dX / nx + x0` (stored as f32), row `j` spans
/// `j * dY / ny + y0` to `(j + 1) * dY / ny + y0` (both stored as f32; `ny` as f32).
#[hook(addr = 0x6E6A10, conv = cdecl, name = "FUN_006e6a10", expect = "83 EC 20 D9 44 24 30")]
pub fn render_detailed_sea_bed_segment(bx: i32, by: i32, fx0: f32, fx1: f32, fy0: f32, fy1: f32) {
    let ddx = x(fx1).fsub_m(fx0);
    let dx = f32_of(ddx);
    let nx = ftol32(ddx.fmul_m(k(FOUR))).max(1);
    let ddy = x(fy1).fsub_m(fy0);
    let dy = f32_of(ddy);
    let ny = ftol32(ddy.fmul_m(k(FOUR))).max(1);
    let mut n = g::<i32>(TEMP_VERTICES_STORED).read();
    let mut m = g::<i32>(TEMP_INDICES_STORED).read();
    let world = |c: X87| f32_of(c.fmul_m(k(BLOCK_SIZE)).fsub_m(k(MAX_COORD)));
    let mut i = 0;
    while i < nx {
        let fa = xi(i).fmul_m(dx).fdiv(xi(nx)).fadd_m(fx0);
        let fb = f32_of(xi(i.wrapping_add(1)).fmul_m(dx).fdiv(xi(nx)).fadd_m(fx0));
        let nyf = f32_of(xi(ny));
        let ua = f32_of(x(k(EIGHT)).fmul(fa));
        let xf = f32_of(xi(bx));
        let xa = world(xi(bx).fadd(fa));
        let ub = f32_of(x(fb).fmul_m(k(EIGHT)));
        let xb = world(x(xf).fadd_m(fb));
        let mut j = 0;
        while j < ny {
            let ga = f32_of(xi(j).fmul_m(dy).fdiv_m(nyf).fadd_m(fy0));
            let gb = f32_of(xi(j.wrapping_add(1)).fmul_m(dy).fdiv_m(nyf).fadd_m(fy0));
            let va = f32_of(x(ga).fmul_m(k(EIGHT)));
            let ya = world(xi(by).fadd_m(ga));
            let vb = f32_of(x(gb).fmul_m(k(EIGHT)));
            let yb = world(xi(by).fadd_m(gb));
            sea_bed_quad(n, m, [xa, xb], [ya, yb], [ua, ub], [va, vb]);
            n = n.wrapping_add(4);
            m = m.wrapping_add(6);
            j += 1;
        }
        i += 1;
    }
    g::<i32>(TEMP_INDICES_STORED).write(m);
    g::<i32>(TEMP_VERTICES_STORED).write(n);
}

/// 0.04 (0x858CEC), 0.08 (0x859018), 7.0 (0x858F48)
const LAYER1_SCALE: u32 = 0x3D23_D70A;
const LAYER0_SCALE: u32 = 0x3DA3_D70A;
const UV_SHIFT: u32 = 0x40E0_0000;

/// The flat-water vertex colour: `WaterColor`'s r, g, b each `ftol(c * 0.577) & 0xFF`, alpha `a`, as the
/// D3D dword `a << 24 | r << 16 | g << 8 | b`.
fn flat_water_colour(rgba: u32, a: u8) -> u32 {
    let ch = |s: u32| (ftol32(xi(((rgba >> s) & 0xFF) as i32).fmul_m(k(W_GLARE_DOT))) & 0xFF) as u32;
    let (r, g2, b) = (ch(0), ch(8), ch(16));
    (((u32::from(a) << 8 | r) << 8 | g2) << 8) | b
}

/// The texture origin of a flat-water layer: `(x1 * s + shiftU, y1 * s + shiftV)` minus their floors,
/// the per-unit scale and the layer alpha (layer 1: 0.04, the "first" shifts, alpha 1; layer 0: 0.08,
/// the "second" shifts, alpha 0). Returns `(u0 as f32, v0 unrounded, scale, alpha)`.
fn flat_water_layer(layer: i32, x1: i32, y1: i32) -> (f32, X87, u32, u8) {
    let (s, su, sv, scale, alpha) = if layer == 1 {
        (LAYER1_SCALE, TEXTURE_SHIFT_FIRST_U.get(), TEXTURE_SHIFT_FIRST_V.get(), LAYER1_SCALE, WATER_LAYER_ALPHA.get()[1])
    } else {
        (LAYER0_SCALE, TEXTURE_SHIFT_SECOND_U.get(), TEXTURE_SHIFT_SECOND_V.get(), LAYER0_SCALE, WATER_LAYER_ALPHA.get()[0])
    };
    let u = f32_of(xi(x1).fmul_m(k(s)).fadd_m(su));
    let v = f32_of(xi(y1).fmul_m(k(s)).fadd_m(sv));
    let u0 = f32_of(floor_x(x(u)).fsubr_m(u));
    let v0 = floor_x(x(v)).fsubr_m(v);
    (u0, v0, scale, alpha as u8)
}

/// `CWaterLevel::RenderFlatWaterRectangle_OneLayer(int32 x1, int32 x2, int32 y1, int32 y2, CRenPar p1,
/// p2, p3, p4, int32 layer)` (0x6E9940): flushes the temp buffer when it is near full
/// (`RenderBuffer::RenderIfDoesntFit` at 0xFFA indices / 0x7FC vertices), then one quad (x1, y1) (x2,
/// y1) (x1, y2) (x2, y2) with z = `p.z + 0`, the layer's uv (`u0 - 7`, `v0 -+ 7` by the sign of `y2 -
/// y1`, plus `(x2 - x1) * scale` / `(y2 - y1) * scale`) and `flat_water_colour`; indices (n, n+1, n+2,
/// n+3, n+1, n+2). Layers other than 0 / 1 read uninitialised locals in the original: they run the
/// original body (`fallback`).
#[hook(addr = 0x6E9940, conv = cdecl, name = "CWaterLevel::RenderFlatWaterRectangle_OneLayer", expect = "83 EC 10 53 55", fallback)]
#[allow(clippy::too_many_arguments)]
pub fn render_flat_water_rectangle_one_layer(
    x1: i32, x2: i32, y1: i32, y2: i32,
    p1z: u32, p1b: u32, p1s: u32, p1f: u32,
    p2z: u32, p2b: u32, p2s: u32, p2f: u32,
    p3z: u32, p3b: u32, p3s: u32, p3f: u32,
    p4z: u32, p4b: u32, p4s: u32, p4f: u32,
    layer: i32,
) {
    if layer != 0 && layer != 1 {
        render_flat_water_rectangle_one_layer_original(
            x1, x2, y1, y2, p1z, p1b, p1s, p1f, p2z, p2b, p2s, p2f, p3z, p3b, p3s, p3f, p4z, p4b, p4s, p4f, layer,
        );
        return;
    }
    if g::<i32>(TEMP_INDICES_STORED).read() >= 0xFFA || g::<i32>(TEMP_VERTICES_STORED).read() >= 0x7FC {
        render_buffer_render_if_doesnt_fit();
    }
    let m = g::<i32>(TEMP_INDICES_STORED).read();
    let n = g::<i32>(TEMP_VERTICES_STORED).read();
    let (u0, v0, scale, alpha) = flat_water_layer(layer, x1, y1);
    let u0 = f32_of(x(u0).fsub_m(k(UV_SHIFT)));
    let dy = y2.wrapping_sub(y1);
    let v0 = if dy > 0 { v0.fsub_m(k(UV_SHIFT)) } else { v0.fadd_m(k(UV_SHIFT)) };
    let colour = flat_water_colour(WATER_COLOR.get(), alpha);
    let u1 = f32_of(xi(x2.wrapping_sub(x1)).fmul_m(k(scale)).fadd_m(u0));
    let v1 = f32_of(xi(dy).fmul_m(k(scale)).fadd(v0));
    let v0 = f32_of(v0);
    let z = |p: u32| f32_of(x(f32::from_bits(p)).fadd_m(0.0));
    let (fx1, fx2, fy1, fy2) = (f32_of(xi(x1)), f32_of(xi(x2)), f32_of(xi(y1)), f32_of(xi(y2)));
    let verts = [(fx1, fy1, z(p1z), u0, v0), (fx2, fy1, z(p2z), u1, v0), (fx1, fy2, z(p3z), u0, v1), (fx2, fy2, z(p4z), u1, v1)];
    for (i, (px, py, pz, u, w)) in verts.into_iter().enumerate() {
        let p = g::<u8>(TEMP_VERTICES).byte_add((n.wrapping_add(i as i32) as isize).wrapping_mul(0x24));
        wr::<f32>(p, 0, px);
        wr::<f32>(p, 4, py);
        wr::<f32>(p, 8, pz);
        wr::<u32>(p, 0x18, colour);
        wr::<f32>(p, 0x1C, u);
        wr::<f32>(p, 0x20, w);
    }
    let ix = g::<u16>(TEMP_INDICES).wrapping_index(m);
    for (j, d) in [0, 1, 2, 3, 1, 2].into_iter().enumerate() {
        ix.wrapping_index(j as i32).write(n.wrapping_add(d) as u16);
    }
    g::<i32>(TEMP_VERTICES_STORED).write(n.wrapping_add(4));
    g::<i32>(TEMP_INDICES_STORED).write(m.wrapping_add(6));
}

/// Writes temp vertex `n + i` for each `(x, y, z, u, v)` with `colour`.
fn put_flat_vertices(n: i32, colour: u32, verts: &[(f32, f32, f32, f32, f32)]) {
    for (i, &(px, py, pz, u, w)) in verts.iter().enumerate() {
        let p = g::<u8>(TEMP_VERTICES).byte_add((n.wrapping_add(i as i32) as isize).wrapping_mul(0x24));
        wr::<f32>(p, 0, px);
        wr::<f32>(p, 4, py);
        wr::<f32>(p, 8, pz);
        wr::<u32>(p, 0x18, colour);
        wr::<f32>(p, 0x1C, u);
        wr::<f32>(p, 0x20, w);
    }
}

/// `CWaterLevel::RenderFlatWaterTriangle_OneLayer(int32 x1, int32 y1, CRenPar p1, x2, y2, p2, x3, y3, p3,
/// int32 layer)` (0x6E8ED0): as `RenderFlatWaterRectangle_OneLayer` for one triangle (flush at 0xFFD
/// indices / 0x7FD vertices, `WaterColorTriangle`, the v shift's sign from `y3 - y1`; vertex k's uv =
/// `(xk - x1) * scale + u0`, `(yk - y1) * scale + v0`); indices (n, n+1, n+2). Other layers: `fallback`.
#[hook(addr = 0x6E8ED0, conv = cdecl, name = "CWaterLevel::RenderFlatWaterTriangle_OneLayer", expect = "A1 54 B9 C4 00", fallback)]
#[allow(clippy::too_many_arguments)]
pub fn render_flat_water_triangle_one_layer(
    x1: i32, y1: i32, p1z: u32, p1b: u32, p1s: u32, p1f: u32,
    x2: i32, y2: i32, p2z: u32, p2b: u32, p2s: u32, p2f: u32,
    x3: i32, y3: i32, p3z: u32, p3b: u32, p3s: u32, p3f: u32,
    layer: i32,
) {
    if layer != 0 && layer != 1 {
        render_flat_water_triangle_one_layer_original(
            x1, y1, p1z, p1b, p1s, p1f, x2, y2, p2z, p2b, p2s, p2f, x3, y3, p3z, p3b, p3s, p3f, layer,
        );
        return;
    }
    if g::<i32>(TEMP_INDICES_STORED).read() >= 0xFFD || g::<i32>(TEMP_VERTICES_STORED).read() >= 0x7FD {
        render_buffer_render_if_doesnt_fit();
    }
    let n = g::<i32>(TEMP_VERTICES_STORED).read();
    let (u0, v0, scale, alpha) = flat_water_layer(layer, x1, y1);
    let u0 = f32_of(x(u0).fsub_m(k(UV_SHIFT)));
    let dy = y3.wrapping_sub(y1);
    let v0 = if dy > 0 { v0.fsub_m(k(UV_SHIFT)) } else { v0.fadd_m(k(UV_SHIFT)) };
    let colour = flat_water_colour(WATER_COLOR_TRIANGLE.get(), alpha);
    let u = |xk: i32| f32_of(xi(xk.wrapping_sub(x1)).fmul_m(k(scale)).fadd_m(u0));
    let v = |yk: i32| f32_of(xi(yk.wrapping_sub(y1)).fmul_m(k(scale)).fadd(v0));
    let z = |p: u32| f32_of(x(f32::from_bits(p)).fadd_m(0.0));
    let f = |c: i32| f32_of(xi(c));
    put_flat_vertices(
        n,
        colour,
        &[
            (f(x1), f(y1), z(p1z), u0, f32_of(v0)),
            (f(x2), f(y2), z(p2z), u(x2), v(y2)),
            (f(x3), f(y3), z(p3z), u(x3), v(y3)),
        ],
    );
    let m = g::<i32>(TEMP_INDICES_STORED).read();
    let ix = g::<u16>(TEMP_INDICES).wrapping_index(m);
    for j in 0..3 {
        ix.wrapping_index(j).write(n.wrapping_add(j) as u16);
    }
    g::<i32>(TEMP_VERTICES_STORED).write(n.wrapping_add(3));
    g::<i32>(TEMP_INDICES_STORED).write(m.wrapping_add(3));
}

/// `CRenPar` lerp 0x6E5280 at t = 0.5 (pushed `0x3F000000`) into a local.
fn lerp_half(a: [u32; 4], b: [u32; 4]) -> [u32; 4] {
    let mut out = [0u32; 4];
    let r = ren_par_lerp(GPtr::from_raw(out.as_mut_ptr()).cast(), a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3], HALF);
    [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8), rd::<u32>(r, 12)]
}

/// Calls `RenderFlatWaterRectangle` (through its entry 0x6EBEC0) with four CRenPars.
fn flat_rect(x1: i32, x2: i32, y1: i32, y2: i32, p: [[u32; 4]; 4]) {
    o_render_flat_water_rectangle(
        x1, x2, y1, y2, p[0][0], p[0][1], p[0][2], p[0][3], p[1][0], p[1][1], p[1][2], p[1][3], p[2][0], p[2][1],
        p[2][2], p[2][3], p[3][0], p[3][1], p[3][2], p[3][3],
    );
}

/// `CWaterLevel::RenderFlatWaterRectangle(int32 x1, int32 x2, int32 y1, int32 y2, CRenPar p1 (x1, y1),
/// p2 (x2, y1), p3 (x1, y2), p4 (x2, y2))` (0x6EBEC0: a `jmp` to the protection chunk 0x4010CA, `al =
/// bSplitBigPolys`, back at 0x6EBEC5). With `bSplitBigPolys`: `x2 - x1 > 0xA8` (signed) splits at `(x1 +
/// x2) / 2` (C division), else `y2 - y1 > 0xA8` at `(y1 + y2) / 2`; the mid CRenPars are the lerp
/// 0x6E5280 at exactly 0.5 and both halves recurse into `RenderFlatWaterRectangle`. Otherwise the
/// rectangle is drawn with `RenderFlatWaterRectangle_OneLayer`, layer 0 then layer 1. (Argument lists
/// from Ghidra's decompilation of 0x6EBEC5, checked against the pushes.)
#[hook(addr = 0x6EBEC0, conv = cdecl, name = "CWaterLevel::RenderFlatWaterRectangle", expect = "E9 05 52 D1 FF 83")]
#[allow(clippy::too_many_arguments)]
pub fn render_flat_water_rectangle(
    x1: i32, x2: i32, y1: i32, y2: i32,
    p1z: u32, p1b: u32, p1s: u32, p1f: u32,
    p2z: u32, p2b: u32, p2s: u32, p2f: u32,
    p3z: u32, p3b: u32, p3s: u32, p3f: u32,
    p4z: u32, p4b: u32, p4s: u32, p4f: u32,
) {
    let p1 = [p1z, p1b, p1s, p1f];
    let p2 = [p2z, p2b, p2s, p2f];
    let p3 = [p3z, p3b, p3s, p3f];
    let p4 = [p4z, p4b, p4s, p4f];
    if SPLIT_BIG_POLYS.get() != 0 {
        if x2.wrapping_sub(x1) > 0xA8 {
            let mid = x1.wrapping_add(x2) / 2;
            let m34 = lerp_half(p3, p4);
            let m12 = lerp_half(p1, p2);
            flat_rect(x1, mid, y1, y2, [p1, m12, p3, m34]);
            let m34 = lerp_half(p3, p4);
            let m12 = lerp_half(p1, p2);
            flat_rect(mid, x2, y1, y2, [m12, p2, m34, p4]);
            return;
        }
        if y2.wrapping_sub(y1) > 0xA8 {
            let mid = y2.wrapping_add(y1) / 2;
            let m24 = lerp_half(p2, p4);
            let m13 = lerp_half(p1, p3);
            flat_rect(x1, x2, y1, mid, [p1, p2, m13, m24]);
            let m24 = lerp_half(p2, p4);
            let m13 = lerp_half(p1, p3);
            flat_rect(x1, x2, mid, y2, [m13, m24, p3, p4]);
            return;
        }
    }
    for layer in 0..2 {
        o_render_flat_water_rectangle_one_layer(
            x1, x2, y1, y2, p1z, p1b, p1s, p1f, p2z, p2b, p2s, p2f, p3z, p3b, p3s, p3f, p4z, p4b, p4s, p4f, layer,
        );
    }
}

/// `CWaterLevel::RenderFlatWaterTriangle(int32 x1, int32 y1, CRenPar p1, x2, y2, p2, x3, y3, p3)`
/// (0x6EE080): with `bSplitBigPolys` and `x2 - x1 > 0xA8` (signed) it hands the triangle to
/// `SplitWaterTriangleAlongXLine((x1 + x2) / 2, ...)`; otherwise `RenderFlatWaterTriangle_OneLayer`
/// with layer 0, then layer 1.
#[hook(addr = 0x6EE080, conv = cdecl, name = "CWaterLevel::RenderFlatWaterTriangle", expect = "A0 F4 37 8D 00")]
#[allow(clippy::too_many_arguments)]
pub fn render_flat_water_triangle(
    x1: i32, y1: i32, p1z: u32, p1b: u32, p1s: u32, p1f: u32,
    x2: i32, y2: i32, p2z: u32, p2b: u32, p2s: u32, p2f: u32,
    x3: i32, y3: i32, p3z: u32, p3b: u32, p3s: u32, p3f: u32,
) {
    if SPLIT_BIG_POLYS.get() != 0 && x2.wrapping_sub(x1) > 0xA8 {
        o_split_water_triangle_along_x_line(
            x1.wrapping_add(x2) / 2,
            x1, y1, p1z, p1b, p1s, p1f, x2, y2, p2z, p2b, p2s, p2f, x3, y3, p3z, p3b, p3s, p3f,
        );
        return;
    }
    for layer in 0..2 {
        o_render_flat_water_triangle_one_layer(
            x1, y1, p1z, p1b, p1s, p1f, x2, y2, p2z, p2b, p2s, p2f, x3, y3, p3z, p3b, p3s, p3f, layer,
        );
    }
}

/// `CWaterLevel::RenderWaterTriangle(int32 x1, int32 y1, CRenPar p1, x2, y2, p2, x3, y3, p3)` (0x6EE240;
/// the protection chunk 0x405B30 loads `CameraRangeMaxX`, `ror([0x15607F0], [0x7316A5] & 31)`, and
/// returns to 0x6EE24C): outside the camera range in x (`x1 >= MaxX || x2 <= MinX`) or in y (`min(y1, y3)
/// >= MaxY || max(y1, y3) <= MinY`) -> `RenderFlatWaterTriangle`; crossing `MaxX` / `MinX` (`x2 > MaxX`,
/// then `x1 < MinX`) -> `SplitWaterTriangleAlongXLine` there; crossing `MaxY` / `MinY` (`max > MaxY`,
/// then `min < MinY`) -> `SplitWaterTriangleAlongYLine`; otherwise `RenderHighDetailWaterTriangle`.
#[hook(addr = 0x6EE240, conv = cdecl, name = "CWaterLevel::RenderWaterTriangle", expect = "8B 54 24 04 53")]
#[allow(clippy::too_many_arguments)]
pub fn render_water_triangle(
    x1: i32, y1: i32, p1z: u32, p1b: u32, p1s: u32, p1f: u32,
    x2: i32, y2: i32, p2z: u32, p2b: u32, p2s: u32, p2f: u32,
    x3: i32, y3: i32, p3z: u32, p3b: u32, p3s: u32, p3f: u32,
) {
    let a = [p1z, p1b, p1s, p1f];
    let b = [p2z, p2b, p2s, p2f];
    let c = [p3z, p3b, p3s, p3f];
    let max_x = CAMERA_RANGE_MAX_X.get();
    let min_x = CAMERA_RANGE_MIN_X.get();
    let (lo, hi) = if y1 < y3 { (y1, y3) } else { (y3, y1) };
    let flat = x1 >= max_x || x2 <= min_x || lo >= CAMERA_RANGE_MAX_Y.get() || hi <= CAMERA_RANGE_MIN_Y.get();
    type Tri = fn(i32, i32, u32, u32, u32, u32, i32, i32, u32, u32, u32, u32, i32, i32, u32, u32, u32, u32);
    type Split = fn(i32, i32, i32, u32, u32, u32, u32, i32, i32, u32, u32, u32, u32, i32, i32, u32, u32, u32, u32);
    let tri = |f: Tri| f(x1, y1, a[0], a[1], a[2], a[3], x2, y2, b[0], b[1], b[2], b[3], x3, y3, c[0], c[1], c[2], c[3]);
    let split = |f: Split, s: i32| {
        f(s, x1, y1, a[0], a[1], a[2], a[3], x2, y2, b[0], b[1], b[2], b[3], x3, y3, c[0], c[1], c[2], c[3])
    };
    if flat {
        tri(o_render_flat_water_triangle);
    } else if x2 > max_x {
        split(o_split_water_triangle_along_x_line, max_x);
    } else if x1 < min_x {
        split(o_split_water_triangle_along_x_line, min_x);
    } else if hi > CAMERA_RANGE_MAX_Y.get() {
        split(o_split_water_triangle_along_y_line, CAMERA_RANGE_MAX_Y.get());
    } else if lo < CAMERA_RANGE_MIN_Y.get() {
        split(o_split_water_triangle_along_y_line, CAMERA_RANGE_MIN_Y.get());
    } else {
        tri(o_render_high_detail_water_triangle);
    }
}

/// The CRenPar lerp 0x6E5280 at `t` (raw f32 bits) into a local.
fn lerp_at(a: [u32; 4], b: [u32; 4], t: u32) -> [u32; 4] {
    let mut out = [0u32; 4];
    let r = ren_par_lerp(GPtr::from_raw(out.as_mut_ptr()).cast(), a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3], t);
    [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8), rd::<u32>(r, 12)]
}

/// Calls `RenderWaterRectangle` (0x6EC5D0) with four CRenPars.
fn water_rect(x1: i32, x2: i32, y1: i32, y2: i32, p: [[u32; 4]; 4]) {
    o_render_water_rectangle(
        x1, x2, y1, y2, p[0][0], p[0][1], p[0][2], p[0][3], p[1][0], p[1][1], p[1][2], p[1][3], p[2][0], p[2][1],
        p[2][2], p[2][3], p[3][0], p[3][1], p[3][2], p[3][3],
    );
}

/// `CWaterLevel::RenderWaterRectangle(int32 x1, int32 x2, int32 y1, int32 y2, CRenPar p1 (x1, y1), p2
/// (x2, y1), p3 (x1, y2), p4 (x2, y2))` (0x6EC5D0): outside the camera range (`x1 >= MaxX || x2 <= MinX`,
/// or `min(y1, y2) >= MaxY || max(y1, y2) <= MinY`): `RenderFlatWaterRectangle` then `SetUpWaterFog(x1,
/// min, x2, max)`; crossing `MaxX` (`x2 > MaxX`) or `MinX` (`x1 < MinX`): `SplitWaterRectangleAlongXLine`
/// there; `min < MinY` or `max > MaxY`: split at that y with `t = (float)(ys - y1) / (y2 - y1)` (`fild` /
/// `fidiv`, stored f32), the mid CRenPars `lerp(p1, p3, t)` / `lerp(p2, p4, t)`, both halves recursing
/// into `RenderWaterRectangle` (the second half's start is the `MinY` read on entry, the first half's
/// end the global re-read); otherwise `RenderHighDetailWaterRectangle`. (Argument lists from Ghidra's
/// decompilation, checked against the pushes of the split paths.)
#[hook(addr = 0x6EC5D0, conv = cdecl, name = "CWaterLevel::RenderWaterRectangle", expect = "83 EC 20 53 55")]
#[allow(clippy::too_many_arguments)]
pub fn render_water_rectangle(
    x1: i32, x2: i32, y1: i32, y2: i32,
    p1z: u32, p1b: u32, p1s: u32, p1f: u32,
    p2z: u32, p2b: u32, p2s: u32, p2f: u32,
    p3z: u32, p3b: u32, p3s: u32, p3f: u32,
    p4z: u32, p4b: u32, p4s: u32, p4f: u32,
) {
    let p = [[p1z, p1b, p1s, p1f], [p2z, p2b, p2s, p2f], [p3z, p3b, p3s, p3f], [p4z, p4b, p4s, p4f]];
    let (lo, hi) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
    let min_y0 = CAMERA_RANGE_MIN_Y.get();
    let max_y0 = CAMERA_RANGE_MAX_Y.get();
    if x1 >= CAMERA_RANGE_MAX_X.get() || x2 <= CAMERA_RANGE_MIN_X.get() || lo >= max_y0 || hi <= min_y0 {
        flat_rect(x1, x2, y1, y2, p);
        o_set_up_water_fog(x1, lo, x2, hi);
        return;
    }
    let max_x = CAMERA_RANGE_MAX_X.get();
    if x2 > max_x || x1 < CAMERA_RANGE_MIN_X.get() {
        let s = if x2 > max_x { max_x } else { CAMERA_RANGE_MIN_X.get() };
        o_split_water_rectangle_along_x_line(
            s, x1, x2, y1, y2, p[0][0], p[0][1], p[0][2], p[0][3], p[1][0], p[1][1], p[1][2], p[1][3], p[2][0],
            p[2][1], p[2][2], p[2][3], p[3][0], p[3][1], p[3][2], p[3][3],
        );
        return;
    }
    let ys = if lo < min_y0 {
        min_y0
    } else if hi <= max_y0 {
        o_render_high_detail_water_rectangle(
            x1, x2, y1, y2, p[0][0], p[0][1], p[0][2], p[0][3], p[1][0], p[1][1], p[1][2], p[1][3], p[2][0],
            p[2][1], p[2][2], p[2][3], p[3][0], p[3][1], p[3][2], p[3][3],
        );
        return;
    } else {
        max_y0
    };
    let t = f32_of(xi(ys.wrapping_sub(y1)).fdiv(xi(y2.wrapping_sub(y1)))).to_bits();
    let m24 = lerp_at(p[1], p[3], t);
    let m13 = lerp_at(p[0], p[2], t);
    let ys_now = if lo < min_y0 { CAMERA_RANGE_MIN_Y.get() } else { CAMERA_RANGE_MAX_Y.get() };
    water_rect(x1, x2, y1, ys_now, [p[0], p[1], m13, m24]);
    let m24 = lerp_at(p[1], p[3], t);
    let m13 = lerp_at(p[0], p[2], t);
    water_rect(x1, x2, ys, y2, [m13, m24, p[2], p[3]]);
}

/// `CWeather::UnderWaterness`, the water-fog weather factor (0xC81338), the fog threshold (.data
/// 0x8D514C), `RsGlobal.maximumWidth` / `maximumHeight`
const UNDER_WATERNESS: usize = 0xC8132C;
const WATER_FOG_FX: usize = 0xC81338;
const WATER_FOG_THRESHOLD: usize = 0x8D514C;
const RS_MAX_WIDTH: usize = 0xC17044;
const RS_MAX_HEIGHT: usize = 0xC17048;
/// 40.0 (immediate)
const FOG_HOLD: u32 = 0x4220_0000;

/// `RwIm3DTransform(TempVertexBuffer, n, NULL, 8)`, then on success `RwIm3DRenderPrimitive(3)` and
/// `RwIm3DEnd`.
fn flush_fog(n: i32) {
    if !rw_im3d_transform(g(TEMP_VERTICES), n as u32, GPtr::NULL, 8).is_null() {
        rw_im3d_render_primitive(3);
        rw_im3d_end();
    }
}

/// `CWaterLevel::RenderWaterFog()` (0x6E7760): with water fog on (both flags): unless `UnderWaterness <
/// [0x8D514C]` (or unordered) and the density `ftol(m_WaterFogDensity * [0xC81338])` is non-zero, the
/// fog list is cleared and nothing drawn. The inside-fog fade moves up by `TimeStep * 0.04` (capped at 1,
/// hold timer 40) while the player was inside a fog box, otherwise the hold timer runs down and then the
/// fade falls (floored at 0); a positive fade draws the full-screen quad (`CPostEffects::DrawQuad`, alpha
/// `ftol(ftol(a * fade) * [0xC81338])`) and a fade of exactly 1 ends there. Then every fog box of the
/// list (which is cleared) is drawn as `ftol((1 - fade) * density)` stacked layers of two triangles,
/// `m_fWaterFogHeight / density` apart, from its level up, in Im3D batches of 0x7FE vertices, and the
/// render states are restored (tail `jmp`).
#[hook(addr = 0x6E7760, conv = cdecl, name = "CWaterLevel::RenderWaterFog", expect = "A0 D4 37 8D 00")]
pub fn render_water_fog() {
    if WATER_FOG.get() == 0 || g::<u8>(0x8D37D5).read() == 0 {
        return;
    }
    let ts = g::<f32>(TIME_STEP).read();
    if !x(g::<f32>(UNDER_WATERNESS).read()).fcom(x(g::<f32>(WATER_FOG_THRESHOLD).read())).test_any(0x01) {
        WATER_FOG_INDEX.set(0);
        return;
    }
    let density = ftol32(xi(WATER_FOG_DENSITY.get()).fmul_m(g::<f32>(WATER_FOG_FX).read()));
    if density == 0 {
        WATER_FOG_INDEX.set(0);
        return;
    }
    let densf = f32_of(xi(density));
    let step = f32_of(x(f32::from_bits(WATER_FOG_HEIGHT.get())).fdiv_m(densf));
    let count = WATER_FOG_INDEX.get();
    WATER_FOG_INDEX.set(0);
    let speed = f32::from_bits(WATER_FOG_FADE_SPEED.get());
    if PLAYER_INSIDE_WATER_FOG.get() != 0 {
        let f = x(ts).fmul_m(speed).fadd_m(WATER_FOG_FADE.get());
        WATER_FOG_FADE.set(f32_of(f));
        if !f.fcom(x(k(ONE))).test_any(0x41) {
            WATER_FOG_FADE.set(k(ONE));
        }
        WATER_FOG_HOLD.set(k(FOG_HOLD));
    } else {
        let t = x(WATER_FOG_HOLD.get()).fsub_m(ts);
        WATER_FOG_HOLD.set(f32_of(t));
        if !t.fcom(x(k(ZERO))).test_parity(0x41) {
            WATER_FOG_HOLD.set(0.0);
            let f = x(ts).fmul_m(speed).fsubr_m(WATER_FOG_FADE.get());
            WATER_FOG_FADE.set(f32_of(f));
            if !f.fcom(x(k(ZERO))).test_parity(0x05) {
                WATER_FOG_FADE.set(0.0);
            }
        }
    }
    PLAYER_INSIDE_WATER_FOG.set(0);
    if !x(WATER_FOG_FADE.get()).fcom(x(k(ZERO))).test_any(0x41) {
        let c = WATER_FOG_INSIDE_COLOUR.get();
        let a = ftol32(xi((c >> 24) as i32).fmul_m(WATER_FOG_FADE.get()));
        let a = ftol32(xi(a).fmul_m(g::<f32>(WATER_FOG_FX).read()));
        post_fx_states_store();
        post_fx_states_set();
        let h = f32_of(xi(g::<i32>(RS_MAX_HEIGHT).read()));
        let w = f32_of(xi(g::<i32>(RS_MAX_WIDTH).read()));
        post_fx_draw_quad(0, 0, w.to_bits(), h.to_bits(), c, (c >> 8) & 0xFF, (c >> 16) & 0xFF, a as u32, 0);
        post_fx_states_restore();
        // `fcomp 1.0; test ah, 0x44; jnp` = equal
        if !x(WATER_FOG_FADE.get()).fcom(x(k(ONE))).test_parity(0x44) {
            return;
        }
    }
    post_fx_states_store();
    post_fx_states_set();
    rw_render_state_set(6, 1);
    rw_render_state_set(1, 0);
    let layers = ftol32(x(k(ONE)).fsub_m(WATER_FOG_FADE.get()).fmul_m(densf));
    let mut n = 0i32;
    if count > 0 {
        let d = WATER_FOG_DATA.ptr().cast::<u8>();
        let mut i = 0;
        while i < count {
            let ent = |base: isize| f32_of(xi(i32::from(rd::<i16>(d, base + i as isize * 2))));
            let mut z = f32_of(x(rd::<f32>(d, 0x230 + i as isize * 4)));
            let (x1, y1, x2, y2) = (ent(0), ent(0x8C), ent(0x118), ent(0x1A4));
            let xs = [x1, x2, x2, x1, x2, x1];
            let ys = [y1, y1, y2, y1, y2, y2];
            if layers > 0 {
                let mut left = layers;
                loop {
                    for kk in 0..6 {
                        let c = WATER_FOG_COLOUR.get();
                        let colour = ((((c >> 24) << 8 | (c & 0xFF)) << 8 | ((c >> 8) & 0xFF)) << 8) | ((c >> 16) & 0xFF);
                        let v = g::<u8>(TEMP_VERTICES).byte_add(n as isize * 0x24);
                        wr::<f32>(v, 0, xs[kk]);
                        wr::<f32>(v, 4, ys[kk]);
                        wr::<f32>(v, 8, z);
                        n += 1;
                        wr::<u32>(v, 0x18, colour);
                        if n == 0x7FE {
                            flush_fog(n);
                            n = 0;
                        }
                    }
                    z = f32_of(x(z).fadd_m(step));
                    left -= 1;
                    if left == 0 {
                        break;
                    }
                }
            }
            i += 1;
        }
        if n > 0 {
            flush_fog(n);
        }
    }
    post_fx_states_restore();
}

/// 1/3 (0x8594EC), 0.71 (0x872168)
const THIRD: u32 = 0x3EAA_AAAA;
const SPHERE_RADIUS_SCALE: u32 = 0x3F35_C28F;
/// TheCamera's frustum plane sets (+0xA4C, +0xA94) and `m_bMirrorActive`-style flag (+0x970)
const CAM_FRUSTUM: u32 = 0xB6FA74;
const CAM_FRUSTUM_MIRROR: u32 = 0xB6FABC;
const CAM_MIRROR_FLAG: usize = 0xB6F998;

/// `CWaterLevel::RenderHighDetailWaterTriangle(int32 x1, int32 y1, CRenPar p1, x2, y2, p2, x3, y3, p3)`
/// (0x6EDDC0): visible when `TheCamera.IsSphereVisible((sum x / 3, sum y / 3, p1.z), (x2 - x1) * 0.71,
/// frustum)` (`fild` sums times 1/3, stored f32), or, with the flag at 0xB6F998, against the second plane
/// set; invisible triangles are dropped. With `n = (x2 - x1) / 2`, `n * n * 3 < 0x1000` and `sum(1..=n
/// + 1) < 0x800` it is drawn by `FUN_006e8780` (layers 0 then 1, extra arguments `n * n`, the sum, `n`);
/// otherwise `SplitWaterTriangleAlongXLine(x1 + (n / 2) * 2, ...)`.
#[hook(addr = 0x6EDDC0, conv = cdecl, name = "CWaterLevel::RenderHighDetailWaterTriangle-", expect = "83 EC 0C 8B 54 24 2C")]
#[allow(clippy::too_many_arguments)]
pub fn render_high_detail_water_triangle(
    x1: i32, y1: i32, p1z: u32, p1b: u32, p1s: u32, p1f: u32,
    x2: i32, y2: i32, p2z: u32, p2b: u32, p2s: u32, p2f: u32,
    x3: i32, y3: i32, p3z: u32, p3b: u32, p3s: u32, p3f: u32,
) {
    let cx = f32_of(xi(x1.wrapping_add(x2).wrapping_add(x3)).fmul_m(k(THIRD)));
    let cy = f32_of(xi(y1.wrapping_add(y2).wrapping_add(y3)).fmul_m(k(THIRD)));
    let mut centre = [cx.to_bits(), cy.to_bits(), p1z];
    let w = x2.wrapping_sub(x1);
    let r = f32_of(xi(w).fmul_m(k(SPHERE_RADIUS_SCALE))).to_bits();
    let cam = GPtr::<u8>::from_addr(crate::common::THE_CAMERA);
    let c = GPtr::from_raw(centre.as_mut_ptr()).cast::<u8>();
    if is_sphere_visible(cam, c, r, CAM_FRUSTUM) == 0
        && (g::<u8>(CAM_MIRROR_FLAG).read() == 0 || is_sphere_visible(cam, c, r, CAM_FRUSTUM_MIRROR) == 0)
    {
        return;
    }
    let n = w / 2;
    let sq = n.wrapping_mul(n);
    let mut sum = 0i32;
    let mut kk = 1;
    while kk <= n.wrapping_add(1) {
        sum = sum.wrapping_add(kk);
        kk += 1;
    }
    if sq.wrapping_mul(3) < 0x1000 && sum < 0x800 {
        for layer in 0..2 {
            o_render_high_detail_triangle_layer(
                x1, y1, p1z, p1b, p1s, p1f, x2, y2, p2z, p2b, p2s, p2f, x3, y3, p3z, p3b, p3s, p3f, layer, sq, sum, n,
            );
        }
        return;
    }
    o_split_water_triangle_along_x_line(
        x1.wrapping_add((n / 2).wrapping_mul(2)),
        x1, y1, p1z, p1b, p1s, p1f, x2, y2, p2z, p2b, p2s, p2f, x3, y3, p3z, p3b, p3s, p3f,
    );
}

/// The wave colour bytes of the high-detail grid, one per vertex of layer 0, reused by layer 1 (r, g, b at
/// 0xC20968, 0xC20168, 0xC1F968) and their counter (0xC1F960).
const HD_COLOUR_R: usize = 0xC20968;
const HD_COLOUR_G: usize = 0xC20168;
const HD_COLOUR_B: usize = 0xC1F968;
const HD_COLOUR_COUNT: usize = 0xC1F960;
/// The wave normal `CalculateWavesOnlyForCoordinate` writes for the grid (0xC278D4) and the flag that suppresses
/// the grid's vertex and index counters (0xC278E0).
const HD_WAVE_NORMAL: usize = 0xC278D4;
const HD_NO_COUNT: usize = 0xC278E0;
/// The high-detail fade distance (int, .data 48)
const HD_FADE_DIST: usize = 0x8D37D0;
/// 0.08 (0x859018), 0.04 (0x858CEC), 0.75 (0x858F34), 4.0 (0x858B90)
const HD_SCALE0: u32 = 0x3DA3_D70A;
const HD_SCALE1: u32 = 0x3D23_D70A;
const HD_FADE_START: u32 = 0x3F40_0000;
const HD_FADE_MUL: u32 = 0x4080_0000;

/// `FUN_006e91d0` (0x6E91D0, cdecl): one layer of a high-detail water rectangle, `(nx + 1) x (ny + 1)` vertices
/// (the extra arguments `cells` / `verts` are not read).
///
/// Colour counter 0, `RenderBuffer::RenderIfDoesntFit()`. `a = f32(1 / nx)`, `b = f32(1 / ny)`; the steps
/// `(x2 - x1) / nx`, `(y2 - y1) / ny`, `(x1 - x2) / nx`, `(y1 - y2) / ny` (`idiv`); the f32 gradients of z, the big
/// and the small wave factors from p1 along x (p2 - p1, times a) and y (p3 - p1, times b) and from p4 (p3 - p4
/// times a, p2 - p4 times b). Layer 0 / 1: the texture origin `frac(x1 * s + shiftU)`, `frac(y1 * s + shiftV)`
/// (s 0.08 with the second shifts / 0.04 with the first). For each row j and column i: below the diagonal (`i * a
/// + f32(j * b) < 1`, else `test ah, 5; jp`) the values interpolate from p1 at `(x1 + i * stepX, y1 + j *
/// stepY)`, otherwise from p4 with the counts `nx - i`, `ny - j` at `(x2 + (nx - i) * -stepX', y2 + (ny - j) *
/// -stepY')`; the camera's 2D distance (`camera - f32 position`, both f32) over the fade distance (`fidiv`) gives
/// the wave factor F (0 beyond 1, 1 up to 0.75, else `(1 - d) * 4`). Layer 0: `CalculateWavesOnlyForCoordinate(x,
/// y, f32(B * F), f32(S * F), &z, &colourMult, &glare, 0xC278D4)`, the vertex position, uv `(i * stepX * 0.08 +
/// u0, j * stepY * 0.08 + v0)`, colour `WaterLayerAlpha[0]` with `ftol(WaterColor.rgb * colourMult)` (each byte
/// kept for layer 1); layer 1: uv with 0.04 and the kept bytes under `WaterLayerAlpha[1]`; other layers write
/// nothing. Unless the flag at 0xC278E0: two triangles `(n - nx - 2, n - nx - 1, n - 1)`, `(n, n - nx - 1, n -
/// 1)` once i and j are both non-zero, and the vertex count + 1.
#[hook(addr = 0x6E91D0, conv = cdecl, name = "FUN_006e91d0", expect = "81 EC C4 00 00 00 53 55 56 57",
       unverified = "intro3-9", route = "L2", hazards = "heap-writes", recs = "intro",
       note = "the intro's first scene sees the Los Santos canal in high detail")]
#[allow(clippy::too_many_arguments)]
pub fn render_high_detail_rectangle_layer(
    x1: i32, x2: i32, y1: i32, y2: i32,
    p1z: u32, p1b: u32, p1s: u32, _p1f: u32,
    p2z: u32, p2b: u32, p2s: u32, _p2f: u32,
    p3z: u32, p3b: u32, p3s: u32, _p3f: u32,
    p4z: u32, p4b: u32, p4s: u32, _p4f: u32,
    layer: i32, _cells: i32, _verts: i32, nx: i32, ny: i32,
) {
    g::<u32>(HD_COLOUR_COUNT).write(0);
    render_buffer_render_if_doesnt_fit();
    let f = f32::from_bits;
    let a = f32_of(xi(nx).fdivr_m(k(ONE)));
    let b = f32_of(xi(ny).fdivr_m(k(ONE)));
    let step_x = x2.wrapping_sub(x1).wrapping_div(nx);
    let step_y = y2.wrapping_sub(y1).wrapping_div(ny);
    let grad = |hi: u32, lo: u32, m: f32| f32_of(x(f(hi)).fsub_m(f(lo)).fmul_m(m));
    let (dz12, dz13) = (grad(p2z, p1z, a), grad(p3z, p1z, b));
    let (db12, db13) = (grad(p2b, p1b, a), grad(p3b, p1b, b));
    let (ds12, ds13) = (grad(p2s, p1s, a), grad(p3s, p1s, b));
    let back_x = x1.wrapping_sub(x2).wrapping_div(nx);
    let (dz34, dz24) = (grad(p3z, p4z, a), grad(p2z, p4z, b));
    let back_y = y1.wrapping_sub(y2).wrapping_div(ny);
    let (db34, db24) = (grad(p3b, p4b, a), grad(p2b, p4b, b));
    let (ds34, ds24) = (grad(p3s, p4s, a), grad(p2s, p4s, b));
    let frac = |v: f32| f32_of(floor_x(x(v)).fsubr_m(v));
    let (u0, v0) = match layer {
        0 => (
            frac(f32_of(xi(x1).fmul_m(k(HD_SCALE0)).fadd_m(TEXTURE_SHIFT_SECOND_U.get()))),
            frac(f32_of(xi(y1).fmul_m(k(HD_SCALE0)).fadd_m(TEXTURE_SHIFT_SECOND_V.get()))),
        ),
        1 => (
            frac(f32_of(xi(x1).fmul_m(k(HD_SCALE1)).fadd_m(TEXTURE_SHIFT_FIRST_U.get()))),
            frac(f32_of(xi(y1).fmul_m(k(HD_SCALE1)).fadd_m(TEXTURE_SHIFT_FIRST_V.get()))),
        ),
        _ => (0.0, 0.0),
    };
    let cm = g::<GPtr<u8>>(crate::common::THE_CAMERA + 0x14).read();
    let cam = if cm.is_null() { GPtr::<u8>::from_addr(crate::common::THE_CAMERA + 4) } else { cm.byte_add(0x30) };
    let (cam_x, cam_y) = (rd::<f32>(cam, 0), rd::<f32>(cam, 4));
    let mut vtx = g::<i32>(TEMP_VERTICES_STORED).read();
    let mut idx = g::<i32>(TEMP_INDICES_STORED).read();
    let mut c = g::<i32>(HD_COLOUR_COUNT).read();
    let mut row_down = y2.wrapping_add(back_y.wrapping_mul(ny));
    let mut rows_left = ny;
    let mut jq = 0i32;
    let mut j = 0i32;
    while j <= ny {
        if nx >= 0 {
            let fj = f32_of(xi(j));
            let jb = f32_of(xi(j).fmul_m(b));
            let mut col_down = back_x.wrapping_mul(nx).wrapping_add(x2);
            let mut cols_left = nx;
            let mut ix = 0i32;
            let mut i = 0i32;
            while i <= nx {
                let fi = f32_of(xi(i));
                let (xp, yp, z, big, small);
                if !xi(i).fmul_m(a).fadd_m(jb).fcom(x(k(ONE))).test_parity(0x05) {
                    xp = x1.wrapping_add(ix);
                    yp = y1.wrapping_add(jq);
                    z = f32_of(x(fj).fmul_m(dz13).fadd(x(fi).fmul_m(dz12)).fadd_m(f(p1z)));
                    big = x(fj).fmul_m(db13).fadd(x(fi).fmul_m(db12)).fadd_m(f(p1b));
                    small = x(fi).fmul_m(ds12).fadd(x(fj).fmul_m(ds13)).fadd_m(f(p1s));
                } else {
                    let fni = f32_of(xi(cols_left));
                    let fnj = f32_of(xi(rows_left));
                    xp = col_down;
                    yp = row_down;
                    z = f32_of(xi(rows_left).fmul_m(dz24).fadd(x(fni).fmul_m(dz34)).fadd_m(f(p4z)));
                    big = x(fnj).fmul_m(db24).fadd(x(fni).fmul_m(db34)).fadd_m(f(p4b));
                    small = x(fnj).fmul_m(ds24).fadd(x(fni).fmul_m(ds34)).fadd_m(f(p4s));
                }
                let small = f32_of(small);
                let (fx, fy) = (f32_of(xi(xp)), f32_of(xi(yp)));
                let dxc = f32_of(x(cam_x).fsub(xi(xp)));
                let dyc = f32_of(x(cam_y).fsub(xi(yp)));
                let d = x(dyc).fmul_m(dyc).fadd(x(dxc).fmul_m(dxc)).fsqrt().fdiv(xi(g::<i32>(HD_FADE_DIST).read()));
                let d32 = f32_of(d);
                let fade = if !d.fcom(x(k(ONE))).test_any(0x41) {
                    x(0.0)
                } else if x(d32).fcom(x(k(HD_FADE_START))).test_any(0x41) {
                    x(k(ONE))
                } else {
                    x(k(ONE)).fsub_m(d32).fmul_m(k(HD_FADE_MUL))
                };
                let vp = g::<u8>(TEMP_VERTICES).byte_add((vtx as isize).wrapping_mul(0x24));
                match layer {
                    0 => {
                        let mut zz = z;
                        let mut colour_mult = 0f32;
                        let mut glare = 0f32;
                        o_calculate_waves_only_for_coordinate(
                            xp,
                            yp,
                            f32_of(fade.fmul(big)),
                            f32_of(x(small).fmul(fade)),
                            GPtr::from_raw(&raw mut zz),
                            GPtr::from_raw(&raw mut colour_mult),
                            GPtr::from_raw(&raw mut glare),
                            GPtr::from_addr(HD_WAVE_NORMAL),
                        );
                        wr::<f32>(vp, 0x1C, f32_of(xi(ix).fmul_m(k(HD_SCALE0)).fadd_m(u0)));
                        wr::<f32>(vp, 0x20, f32_of(xi(jq).fmul_m(k(HD_SCALE0)).fadd_m(v0)));
                        wr::<f32>(vp, 0, fx);
                        wr::<f32>(vp, 4, fy);
                        wr::<f32>(vp, 8, zz);
                        let ch = |a: usize| ftol32(xi(i32::from(g::<u8>(a).read())).fmul_m(colour_mult)) as u8;
                        let (r, gg, bb) = (ch(0xC2116C), ch(0xC2116D), ch(0xC2116E));
                        let alpha = WATER_LAYER_ALPHA.get()[0];
                        wr::<u32>(vp, 0x18, (((alpha << 8 | u32::from(r)) << 8 | u32::from(gg)) << 8) | u32::from(bb));
                        g::<u8>(HD_COLOUR_G + c as usize).write(gg);
                        g::<u8>(HD_COLOUR_R + c as usize).write(r);
                        g::<u8>(HD_COLOUR_B + c as usize).write(bb);
                        c = c.wrapping_add(1);
                        g::<i32>(HD_COLOUR_COUNT).write(c);
                    }
                    1 => {
                        wr::<f32>(vp, 0x1C, f32_of(xi(ix).fmul_m(k(HD_SCALE1)).fadd_m(u0)));
                        let r = u32::from(g::<u8>(HD_COLOUR_R + c as usize).read());
                        let gg = u32::from(g::<u8>(HD_COLOUR_G + c as usize).read());
                        let bb = u32::from(g::<u8>(HD_COLOUR_B + c as usize).read());
                        let alpha = WATER_LAYER_ALPHA.get()[1];
                        wr::<f32>(vp, 0x20, f32_of(xi(jq).fmul_m(k(HD_SCALE1)).fadd_m(v0)));
                        wr::<u32>(vp, 0x18, (((alpha << 8 | r) << 8 | gg) << 8) | bb);
                        c = c.wrapping_add(1);
                        g::<i32>(HD_COLOUR_COUNT).write(c);
                    }
                    _ => {}
                }
                if g::<u8>(HD_NO_COUNT).read() == 0 {
                    if j != 0 && i != 0 {
                        let e = vtx.wrapping_sub(nx);
                        let ix16 = g::<u16>(TEMP_INDICES).wrapping_index(idx);
                        let tri = [e.wrapping_sub(2), e.wrapping_sub(1), vtx.wrapping_sub(1), vtx, e.wrapping_sub(1), vtx.wrapping_sub(1)];
                        for (n, v) in tri.into_iter().enumerate() {
                            ix16.wrapping_index(n as i32).write(v as u16);
                        }
                        idx = idx.wrapping_add(6);
                        g::<i32>(TEMP_INDICES_STORED).write(idx);
                    }
                    vtx = vtx.wrapping_add(1);
                    g::<i32>(TEMP_VERTICES_STORED).write(vtx);
                }
                cols_left = cols_left.wrapping_sub(1);
                col_down = col_down.wrapping_sub(back_x);
                ix = ix.wrapping_add(step_x);
                i += 1;
            }
        }
        j += 1;
        rows_left = rows_left.wrapping_sub(1);
        row_down = row_down.wrapping_sub(back_y);
        jq = jq.wrapping_add(step_y);
    }
}

/// The CRenPar lerp 0x6E5280 into a buffer holding `init` (the lerp leaves bytes +0xE / +0xF alone).
fn lerp_into(init: [u32; 4], a: [u32; 4], b: [u32; 4], t: u32) -> [u32; 4] {
    let mut out = init;
    let r = ren_par_lerp(GPtr::from_raw(out.as_mut_ptr()).cast(), a[0], a[1], a[2], a[3], b[0], b[1], b[2], b[3], t);
    [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8), rd::<u32>(r, 12)]
}

/// `CWaterLevel::RenderHighDetailWaterRectangle(int32 x1, int32 x2, int32 y1, int32 y2, CRenPar p1..p4)`
/// (0x6EB810): visible when `TheCamera.IsSphereVisible(((x1 + x2) * 0.5, (y1 + y2) * 0.5, p1.z),
/// sqrt(b * b + a * a))` with `a = (x2 - x1) * 0.5`, `b = (y1 - y2) * 0.5` (or, with the flag at
/// 0xB6F998, against the second plane set); otherwise nothing. With `nx = (x2 - x1) / 2`, `ny = (max y -
/// min y) / 2`, `cells = nx * ny * 2`: `cells * 3 < 0x1000` and `(nx + 1) * (ny + 1) < 0x800` ->
/// `SetUpWaterFog(x1, min y, x2, max y)` and `FUN_006e91d0` for layers 0 and 1; else `nx <= ny` -> split
/// at `ys = min y + (ny / 2) * 2` (`t = (float)(ys - y1) / (y2 - y1)`, lerps as in
/// `RenderWaterRectangle`, both halves through `RenderWaterRectangle`); else
/// `SplitWaterRectangleAlongXLine(x1 + (nx / 2) * 2, ...)`. Mid CRenPar padding (+0xE / +0xF): the
/// original lerps into uninitialised locals (here 0), except the second `lerp(p1, p3)`, which overwrites
/// p1's argument slot (its padding kept).
#[hook(addr = 0x6EB810, conv = cdecl, name = "CWaterLevel::RenderHighDetailWaterRectangle", expect = "83 EC 24 53 8B 5C 24 30")]
#[allow(clippy::too_many_arguments)]
pub fn render_high_detail_water_rectangle(
    x1: i32, x2: i32, y1: i32, y2: i32,
    p1z: u32, p1b: u32, p1s: u32, p1f: u32,
    p2z: u32, p2b: u32, p2s: u32, p2f: u32,
    p3z: u32, p3b: u32, p3s: u32, p3f: u32,
    p4z: u32, p4b: u32, p4s: u32, p4f: u32,
) {
    let p = [[p1z, p1b, p1s, p1f], [p2z, p2b, p2s, p2f], [p3z, p3b, p3s, p3f], [p4z, p4b, p4s, p4f]];
    let cx = f32_of(xi(x1.wrapping_add(x2)).fmul_m(k(HALF)));
    let dx = x2.wrapping_sub(x1);
    let cy = f32_of(xi(y1.wrapping_add(y2)).fmul_m(k(HALF)));
    let mut centre = [cx.to_bits(), cy.to_bits(), p1z];
    let a = xi(dx).fmul_m(k(HALF));
    let b = xi(y1.wrapping_sub(y2)).fmul_m(k(HALF));
    let r = f32_of(b.fmul(b).fadd(a.fmul(a)).fsqrt()).to_bits();
    let cam = GPtr::<u8>::from_addr(crate::common::THE_CAMERA);
    let c = GPtr::from_raw(centre.as_mut_ptr()).cast::<u8>();
    if is_sphere_visible(cam, c, r, CAM_FRUSTUM) == 0
        && (g::<u8>(CAM_MIRROR_FLAG).read() == 0 || is_sphere_visible(cam, c, r, CAM_FRUSTUM_MIRROR) == 0)
    {
        return;
    }
    let (lo, hi) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
    let nx = dx / 2;
    let ny = hi.wrapping_sub(lo) / 2;
    let cells = ny.wrapping_mul(nx).wrapping_shl(1);
    let verts = ny.wrapping_add(1).wrapping_mul(nx.wrapping_add(1));
    if cells.wrapping_mul(3) < 0x1000 && verts < 0x800 {
        o_set_up_water_fog(x1, lo, x2, hi);
        for layer in 0..2 {
            o_render_high_detail_rectangle_layer(
                x1, x2, y1, y2, p[0][0], p[0][1], p[0][2], p[0][3], p[1][0], p[1][1], p[1][2], p[1][3], p[2][0],
                p[2][1], p[2][2], p[2][3], p[3][0], p[3][1], p[3][2], p[3][3], layer, cells, verts, nx, ny,
            );
        }
        return;
    }
    if nx > ny {
        o_split_water_rectangle_along_x_line(
            x1.wrapping_add((nx / 2).wrapping_mul(2)),
            x1, x2, y1, y2, p[0][0], p[0][1], p[0][2], p[0][3], p[1][0], p[1][1], p[1][2], p[1][3], p[2][0],
            p[2][1], p[2][2], p[2][3], p[3][0], p[3][1], p[3][2], p[3][3],
        );
        return;
    }
    let ys = lo.wrapping_add((ny / 2).wrapping_mul(2));
    let t = f32_of(xi(ys.wrapping_sub(y1)).fdiv(xi(y2.wrapping_sub(y1)))).to_bits();
    let m24 = lerp_at(p[1], p[3], t);
    let m13 = lerp_at(p[0], p[2], t);
    water_rect(x1, x2, y1, ys, [p[0], p[1], m13, m24]);
    let m24 = lerp_at(p[1], p[3], t);
    let m13 = lerp_into(p[0], p[0], p[2], t);
    water_rect(x1, x2, ys, y2, [m13, m24, p[2], p[3]]);
}

/// The inlined CRenPar lerp of the split functions: `a * (1 - t) + b * t` per float (`t` unrounded on
/// the x87 stack), flow bytes 0, padding `pad`.
fn lerp_inline(a: [u32; 4], b: [u32; 4], t: X87, pad: u32) -> [u32; 4] {
    let omt = x(k(ONE)).fsub(t);
    let f = |i: usize| f32_of(x(f32::from_bits(a[i])).fmul(omt).fadd(x(f32::from_bits(b[i])).fmul(t))).to_bits();
    [f(0), f(1), f(2), pad & 0xFFFF_0000]
}

/// `CWaterLevel::SplitWaterRectangleAlongXLine(int32 split, int32 x1, int32 x2, int32 y1, int32 y2,
/// CRenPar p1..p4)` (0x6E73A0): `t = (split - x1) / (x2 - x1)` (`fild` / `fidiv`), the mid CRenPars
/// `lerp(p1, p2)` and `lerp(p3, p4)` computed inline, then `RenderWaterRectangle(x1, split, .., p1, m12,
/// p3, m34)` and `RenderWaterRectangle(split, x2, .., m12, p2, m34, p4)`. Mid padding (+0xE / +0xF):
/// uninitialised locals for the first call (here 0), p1's / p3's argument slots (their padding) for the
/// second.
#[hook(addr = 0x6E73A0, conv = cdecl, name = "CWaterLevel::SplitWaterRectangleAlongXLine", expect = "83 EC 2C 8B 44 24 34")]
#[allow(clippy::too_many_arguments)]
pub fn split_water_rectangle_along_x_line(
    split: i32, x1: i32, x2: i32, y1: i32, y2: i32,
    p1z: u32, p1b: u32, p1s: u32, p1f: u32,
    p2z: u32, p2b: u32, p2s: u32, p2f: u32,
    p3z: u32, p3b: u32, p3s: u32, p3f: u32,
    p4z: u32, p4b: u32, p4s: u32, p4f: u32,
) {
    let p = [[p1z, p1b, p1s, p1f], [p2z, p2b, p2s, p2f], [p3z, p3b, p3s, p3f], [p4z, p4b, p4s, p4f]];
    let t = xi(split.wrapping_sub(x1)).fdiv(xi(x2.wrapping_sub(x1)));
    let m12 = lerp_inline(p[0], p[1], t, 0);
    let m34 = lerp_inline(p[2], p[3], t, 0);
    water_rect(x1, split, y1, y2, [p[0], m12, p[2], m34]);
    let m12 = [m12[0], m12[1], m12[2], p1f & 0xFFFF_0000];
    let m34 = [m34[0], m34[1], m34[2], p3f & 0xFFFF_0000];
    water_rect(split, x2, y1, y2, [m12, p[1], m34, p[3]]);
}

/// `CWaterLevel::SplitWaterRectangleAlongYLine(int32 split, int32 x1, int32 x2, int32 y1, int32 y2,
/// CRenPar p1..p4)` (0x6ED6D0): `t = (split - y1) / (y2 - y1)`, `m13 = lerp(p1, p3)`, `m24 = lerp(p2,
/// p4)` inline, then `RenderWaterRectangle(x1, x2, y1, split, p1, p2, m13, m24)` and
/// `RenderWaterRectangle(x1, x2, split, y2, m13, m24, p3, p4)`. Mid padding: uninitialised locals for the
/// first call (here 0), p1's / p2's argument slots for the second (derived from the pushes; Ghidra has no
/// function here).
#[hook(addr = 0x6ED6D0, conv = cdecl, name = "CWaterLevel::SplitWaterRectangleAlongYLine", expect = "83 EC 2C 8B 44 24 3C")]
#[allow(clippy::too_many_arguments)]
pub fn split_water_rectangle_along_y_line(
    split: i32, x1: i32, x2: i32, y1: i32, y2: i32,
    p1z: u32, p1b: u32, p1s: u32, p1f: u32,
    p2z: u32, p2b: u32, p2s: u32, p2f: u32,
    p3z: u32, p3b: u32, p3s: u32, p3f: u32,
    p4z: u32, p4b: u32, p4s: u32, p4f: u32,
) {
    let p = [[p1z, p1b, p1s, p1f], [p2z, p2b, p2s, p2f], [p3z, p3b, p3s, p3f], [p4z, p4b, p4s, p4f]];
    let t = xi(split.wrapping_sub(y1)).fdiv(xi(y2.wrapping_sub(y1)));
    let m24 = lerp_inline(p[1], p[3], t, 0);
    let m13 = lerp_inline(p[0], p[2], t, 0);
    water_rect(x1, x2, y1, split, [p[0], p[1], m13, m24]);
    let m13 = [m13[0], m13[1], m13[2], p1f & 0xFFFF_0000];
    let m24 = [m24[0], m24[1], m24[2], p2f & 0xFFFF_0000];
    water_rect(x1, x2, split, y2, [m13, m24, p[2], p[3]]);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_RENDER_HIGH_DETAIL_RECTANGLE_LAYER,
    HOOK_CALCULATE_WAVES_ONLY_FOR_COORDINATE2,
    HOOK_CALCULATE_WAVES_ONLY_FOR_COORDINATE,
    HOOK_ADD_WAVE_TO_RESULT,
    HOOK_QUAD_LEVEL,
    HOOK_TRIANGLE_LEVEL,
    HOOK_GET_WATER_LEVEL_NO_WAVES,
    HOOK_GET_WATER_LEVEL,
    HOOK_MARK_QUADS_AND_POLYS_TO_BE_RENDERED,
    HOOK_BLOCK_HIT,
    HOOK_SCAN_THROUGH_BLOCKS,
    HOOK_GET_GROUND_LEVEL,
    HOOK_GET_WATER_DEPTH,
    HOOK_SET_CAMERA_RANGE,
    HOOK_ADD_WATER_LEVEL_VERTEX,
    HOOK_ADD_WATER_LEVEL_TRIANGLE,
    HOOK_ADD_WATER_LEVEL_QUAD,
    HOOK_ADD_TO_BLOCK,
    HOOK_FILL_QUADS_AND_TRIANGLES_LIST,
    HOOK_PRE_RENDER_WATER,
    HOOK_FIND_NEAREST_WATER_AND_ITS_FLOW,
    HOOK_SHUTDOWN,
    HOOK_WAVES_FOR_COORDINATE,
    HOOK_RENDER_SEA_BED_SEGMENT,
    HOOK_RENDER_DETAILED_SEA_BED_SEGMENT,
    HOOK_RENDER_FLAT_WATER_RECTANGLE_ONE_LAYER,
    HOOK_RENDER_FLAT_WATER_TRIANGLE_ONE_LAYER,
    HOOK_RENDER_FLAT_WATER_RECTANGLE,
    HOOK_RENDER_FLAT_WATER_TRIANGLE,
    HOOK_RENDER_WATER_TRIANGLE,
    HOOK_RENDER_WATER_RECTANGLE,
    HOOK_RENDER_WATER_FOG,
    HOOK_RENDER_HIGH_DETAIL_WATER_TRIANGLE,
    HOOK_RENDER_HIGH_DETAIL_WATER_RECTANGLE,
    HOOK_SPLIT_WATER_RECTANGLE_ALONG_X_LINE,
    HOOK_SPLIT_WATER_RECTANGLE_ALONG_Y_LINE,
];
