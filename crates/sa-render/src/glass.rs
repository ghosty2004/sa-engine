//! PORT-RENDER-2: `CGlass` (static class; `CFallingGlassPane` 0x70 bytes, `+0x6D` exists). Ported from the
//! 1.0 US bytes; gta-reversed `Glass.cpp` and `docs/re/specs/RENDER/CGlass.deviations.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{ftol32, g, jne, jp, k, rd, wr, x};
use sa_core::bindings::{cross_product, vector_normalise};
use sa_core::crt::rand_at;
use sa_math::fpu::xi;
use sa_math::x87::X87;
use sa_world::bindings::find_ground_z_for_3d_coord;

sa_core::globals! {
    list GLOBALS;
    /// `CGlass::ReflectionPolyVertexBaseIndex`, `ReflectionPolyIndexBaseIndex`,
    /// `ShatteredVerticesBaseIndex`, `ShatteredIndicesBaseIndex`, `H_iCurrentHiLightPolyVerticeIndex`,
    /// `H_iCurrentHiLightPolyIndicesIndex` (0xC71B18..0xC71B2C)
    pub static POLY_INDICES @ 0xC71B18: [u32; 6] = [0; 6];
    /// `CGlass::PanePolyCenterPositions[5]` (CVector2D)
    pub static PANE_POLY_CENTER_POSITIONS @ 0xC71B30: [f32; 10] = [0.0; 10];
    /// `CGlass::apEntitiesToBeRendered[32]` (CEntity*)
    pub static ENTITIES_TO_BE_RENDERED @ 0xC71B58: [u32; 32] = [0; 32];
    /// `CGlass::NumGlassEntities`
    pub static NUM_GLASS_ENTITIES @ 0xC71BD8: u32 = 0;
    /// `CGlass::aGlassPanes[45]` (CFallingGlassPane, 0x70 bytes)
    pub static GLASS_PANES @ 0xC71BF8: [u8; 45 * 0x70] = [0; 45 * 0x70];
}

/// `CGlass::PanePolyPositions[5][3]` (CVector2D, `.data`, read only).
const PANE_POLY_POSITIONS: usize = 0x8D5CD8;
/// `CModelInfo::ms_modelInfoPtrs`.
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `TheCamera` + 0x984: the camera's forward vector (CVector).
const CAMERA_FRONT: usize = 0xB6F9AC;
/// `AudioEngine` (CAudioEngine).
const AUDIO_ENGINE: usize = 0xB6BC90;
/// `TempBufferVertices` (RxObjSpace3DVertex) and `TempBufferIndices` (RwImVertexIndex) as `CGlass`
/// renders them.
const TEMP_VERTICES: usize = 0xC4D958;
const TEMP_INDICES: usize = 0xC4B958;
/// `gpShatterTexture`, `gpCrackedGlassTexture` (RwTexture**; the raster is the texture's first field).
const SHATTER_TEXTURE: usize = 0xC40408;
const CRACKED_GLASS_TEXTURE: usize = 0xC403F4;
const REFLECTION_TEXTURE: usize = 0xC403F8;
/// `CTimeCycle::m_CurrentColours.m_nFogColor{Red,Green,Blue}` (uint16).
const FOG_COLOUR: usize = 0xB7C4CA;

pub use sa_world::bindings::rw_im3d_end;
pub use sa_world::bindings::window_responds_to_collision;
pub use sa_world::bindings::rw_im3d_render_indexed_primitive;
pub use sa_world::bindings::rw_im3d_transform;
orig! {
    /// `RwRenderStateSet` (`RwEngineInstance` slot 0x20)
    pub fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `CBaseModelInfo::AsAtomicModelInfoPtr()` (vtable +4)
    pub fn mi_as_atomic_model_info_ptr(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x4 => unported;
    /// `CFallingGlassPane::Update()` (0x71AA10)
    pub fn pane_update(this: GPtr<u8>) = thiscall 0x71AA10 => unported;
    /// `CFallingGlassPane::Render()` (0x71B100)
    pub fn pane_render(this: GPtr<u8>) = thiscall 0x71B100 => unported;
    /// `CGlass::RenderHiLightPolys()` (0x71ADA0)
    pub fn call_render_hilight_polys() = cdecl 0x71ADA0 => render_hilight_polys;
    /// `CGlass::RenderShatteredPolys()` (0x71AE30)
    pub fn call_render_shattered_polys() = cdecl 0x71AE30 => render_shattered_polys;
    /// `CGlass::RenderReflectionPolys()` (0x71AED0)
    pub fn call_render_reflection_polys() = cdecl 0x71AED0 => render_reflection_polys;
    /// `CAudioEngine::ReportGlassCollisionEvent(eAudioEvents, const CVector&)` (0x506EE0)
    pub fn report_glass_collision_event(this: GPtr<u8>, event: i32, pos: GPtr<f32>) = thiscall 0x506EE0 => unported;
}

fn pane(i: i32) -> GPtr<u8> {
    GLASS_PANES.ptr().cast::<u8>().byte_add(i as isize * 0x70)
}

/// The model of a building / object entity (type 4) is an atomic model info with glass flags
/// (`+0x12 & 0x7800` = 0x2000 or 0x2800).
fn is_glass_model(e: GPtr<u8>) -> bool {
    let idx = i32::from(rd::<i16>(e, 0x22));
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(idx).read();
    let ami = mi_as_atomic_model_info_ptr(mi);
    if ami.is_null() {
        return false;
    }
    let f = rd::<u16>(ami, 0x12) & 0x7800;
    f == 0x2000 || f == 0x2800
}

/// `CGlass::IsObjectGlass(CEntity*)` (0x46A760): an object (type 4) whose model is glass. The non-object
/// path clears only AL (`xor al, al`; EAX keeps the entity's upper bytes); callers test AL.
#[hook(addr = 0x46A760, conv = cdecl, name = "CGlass::IsObjectGlass", expect = "8B 44 24 04 8A 48 36")]
pub fn is_object_glass(e: GPtr<u8>) -> u8 {
    if rd::<u8>(e, 0x36) & 7 != 4 {
        return 0;
    }
    u8::from(is_glass_model(e))
}

/// `CGlass::Init()` (0x71A8D0): every pane stops existing, and the five pane-polygon centres are the
/// averages of their three corners (the first two summed p0 + p1 + p2, the other three p2 + p1 + p0,
/// times 1/3 at 0x859040).
#[hook(addr = 0x71A8D0, conv = cdecl, name = "CGlass::Init", expect = "B8 65 1C C7 00")]
pub fn init() {
    for i in 0..45 {
        wr::<u8>(pane(i), 0x6D, 0);
    }
    let p = g::<f32>(PANE_POLY_POSITIONS);
    let third = k(0x3EAA_AAAB);
    let c = PANE_POLY_CENTER_POSITIONS.ptr().cast::<f32>();
    for poly in 0..5 {
        for comp in 0..2 {
            let at = |corner: i32| p.wrapping_index(poly * 6 + corner * 2 + comp).read();
            let sum = if poly < 2 {
                x(at(0)).fadd_m(at(1)).fadd_m(at(2))
            } else {
                x(at(2)).fadd_m(at(1)).fadd_m(at(0))
            };
            c.wrapping_index(poly * 2 + comp)
                .write(sum.fmul_m(third).to_f32());
        }
    }
}

/// `CGlass::FindFreePane()` (0x71ACA0): the first pane that does not exist, or null.
#[hook(addr = 0x71ACA0, conv = cdecl, name = "CGlass::FindFreePane", expect = "33 C0 B9 65 1C C7 00")]
pub fn find_free_pane() -> GPtr<u8> {
    for i in 0..45 {
        if rd::<u8>(pane(i), 0x6D) == 0 {
            return pane(i);
        }
    }
    GPtr::from_addr(0)
}

/// `(float)(uint32)n` as the bytes build it: `fild` of the int plus 2^32 (0x858C54) when negative.
fn u32_x87(n: u32) -> X87 {
    let v = xi(n as i32);
    if (n as i32) < 0 { v.fadd_m(f32::from_bits(0x4F80_0000)) } else { v }
}

/// `CGlass::GeneratePanesForWindow(ePaneType type, CVector point, CVector fwd, CVector right, CVector
/// velocity, CVector centre, float drag, bool shatter, bool max1, int32 sections, bool b)` (0x71B620,
/// cdecl; entry `nop; jmp 0x403985`, which allocates the frame and jumps back to 0x71B626).
///
/// From the bytes (gta-reversed differs: the per-axis count is `max(len + 0.75, 1)` before `_ftol`, the
/// pane offsets are `i * len / count`, the delayed time is `_ftol(|pos - centre| * 100 + now)`, the
/// random offsets take the rand() calls in another order). `lenF = |fwd|`, `lenR = |right|` (x87,
/// `((z z + y y) + x x)`); `nY = _ftol(max(lenF + 0.75, 1)) * sections`, `nX` likewise from lenR, each
/// capped at 3 (unsigned) and at 1 with `max1`; `sizeY = lenF / nY`, `sizeX = lenR / nX`. The ground z
/// under the point (`FindGroundZFor3DCoord`; none: `point.z - 2`). For every y, x and piece 0..4 a free
/// pane (none: next piece) gets the piece, right = `right / lenR * sizeX` (the first component with the
/// unrounded reciprocal), up = `fwd * f32(1 / lenF) * sizeY`, at = `Normalise(right x up)`, position =
/// `point + fwd/lenF * (sizeY * cy + yOff) + right/lenR * (sizeX * cx + xOff)` per component (`cx`,
/// `cy` the piece's centre at 0xC71B30, offsets `i * len / n`), velocity = `velocity + (r1, r2, 0) *
/// 0.0015` (r = rand() & 0x7F - 0x40) plus `Normalise(pos - centre) * drag` unless drag == 0, the
/// random rotation `(r_b, r_a, r_c) * 0.002` (a, b, c in call order), the creation time (type 0 / 2:
/// now; 1: as above; others: unchanged), ground z, shatter, size (sizeY), `b` and exists = 1.
#[hook(addr = 0x71B620, conv = cdecl, name = "CGlass::GeneratePanesForWindow", expect = "90 E9 5F 83 CE FF",
       unverified = "frontier2-c16", route = "L2", hazards = "heap-writes",
       note = "rand() at 0x71B9BB / 0x71B9DB / 0x71BAF5 / 0x71BAFC / 0x71BB31")]
#[allow(clippy::too_many_arguments)]
pub fn generate_panes_for_window(ty: u32, px: f32, py: f32, pz: f32, fx: f32, fy: f32, fz: f32, rx: f32, ry: f32, rz: f32,
                                 vx: f32, vy: f32, vz: f32, cx: f32, cy: f32, cz: f32, drag: f32, shatter: u32, max1: u32,
                                 sections: i32, b: u32) {
    let len = |a: f32, b: f32, c: f32| x(c).fmul_m(c).fadd(x(b).fmul_m(b)).fadd(x(a).fmul_m(a)).fsqrt().to_f32();
    let len_f = len(fx, fy, fz);
    let len_r = len(rx, ry, rz);
    let count = |l: f32| {
        let v = x(l).fadd_m(0.75);
        let v = if x(1.0).fcom(v).test_any(0x41) { v } else { x(1.0) };
        ftol32(v) as u32
    };
    let mut ny = count(len_f).wrapping_mul(sections as u32);
    let mut nx = count(len_r).wrapping_mul(sections as u32);
    ny = ny.min(3);
    nx = nx.min(3);
    if max1 as u8 != 0 {
        ny = ny.min(1);
        nx = nx.min(1);
    }
    let fny = u32_x87(ny).to_f32();
    let size_y = x(len_f).fdiv_m(fny).to_f32();
    let fnx = u32_x87(nx).to_f32();
    let size_x = x(len_r).fdiv_m(fnx).to_f32();
    let mut hit: u8 = 0;
    let mut ground = find_ground_z_for_3d_coord(px, py, pz, GPtr::from_raw(&mut hit), GPtr::NULL) as f32;
    if hit == 0 {
        ground = x(pz).fsub_m(2.0).to_f32();
    }
    let (p, f, r, v, c) = ([px, py, pz], [fx, fy, fz], [rx, ry, rz], [vx, vy, vz], [cx, cy, cz]);
    for iy in 0..ny {
        let y_off = u32_x87(iy).fmul_m(len_f).fdiv_m(fny).to_f32();
        for ix in 0..nx {
            let x_off = u32_x87(ix).fmul_m(len_r).fdiv_m(fnx).to_f32();
            for piece in 0..5u32 {
                let pane = find_free_pane();
                if pane.is_null() {
                    continue;
                }
                wr::<u8>(pane, 0x6C, piece as u8);
                let inv_r_wide = x(1.0).fdiv_m(len_r);
                let inv_r = inv_r_wide.to_f32();
                wr::<f32>(pane, 0, inv_r_wide.fmul_m(rx).fmul_m(size_x).to_f32());
                wr::<f32>(pane, 4, x(inv_r).fmul_m(ry).fmul_m(size_x).to_f32());
                wr::<f32>(pane, 8, x(inv_r).fmul_m(rz).fmul_m(size_x).to_f32());
                let inv_f = x(1.0).fdiv_m(len_f).to_f32();
                for k in 0..3 {
                    wr::<f32>(pane, 0x20 + 4 * k as isize, x(inv_f).fmul_m(f[k]).fmul_m(size_y).to_f32());
                }
                let mut at = [0f32; 3];
                let mut tmp = [0f32; 3];
                let o = cross_product(GPtr::from_raw(tmp.as_mut_ptr()).cast(), pane, pane.byte_add(0x20));
                for (k, a) in at.iter_mut().enumerate() {
                    *a = rd::<f32>(o, 4 * k as isize);
                }
                vector_normalise(GPtr::from_raw(&mut at));
                for (k, a) in at.iter().enumerate() {
                    wr::<f32>(pane, 0x10 + 4 * k as isize, *a);
                }
                let centre = |o: usize| g::<f32>(0xC7_1B30 + piece as usize * 8 + o).read();
                let pcx = x(size_x).fmul_m(centre(0)).fadd_m(x_off);
                let pcy = x(size_y).fmul_m(centre(4)).fadd_m(y_off);
                for k in 0..3 {
                    let a = x(inv_f).fmul_m(f[k]).fmul(pcy);
                    let bb = x(inv_r).fmul_m(r[k]).fmul(pcx);
                    wr::<f32>(pane, 0x30 + 4 * k as isize, a.fadd(bb).fadd_m(p[k]).to_f32());
                }
                let rnd = |site: u32| xi((rand_at(site) & 0x7F) - 0x40);
                let r1 = rnd(0x71_B9BB).fmul_m(0.0015).to_f32();
                let r2 = rnd(0x71_B9DB).fmul_m(0.0015);
                wr::<f32>(pane, 0x48, x(vx).fadd_m(r1).to_f32());
                wr::<f32>(pane, 0x4C, x(vy).fadd(r2).to_f32());
                wr::<f32>(pane, 0x50, v[2]);
                if x(drag).fcom(x(0.0)).test_parity(0x44) {
                    let mut d = [0f32; 3];
                    for (k, dk) in d.iter_mut().enumerate() {
                        *dk = x(rd::<f32>(pane, 0x30 + 4 * k as isize)).fsub_m(c[k]).to_f32();
                    }
                    vector_normalise(GPtr::from_raw(&mut d));
                    let dz = x(d[2]).fmul_m(drag).to_f32();
                    wr::<f32>(pane, 0x48, x(d[0]).fmul_m(drag).fadd_m(rd::<f32>(pane, 0x48)).to_f32());
                    wr::<f32>(pane, 0x4C, x(d[1]).fmul_m(drag).fadd_m(rd::<f32>(pane, 0x4C)).to_f32());
                    wr::<f32>(pane, 0x50, x(dz).fadd_m(rd::<f32>(pane, 0x50)).to_f32());
                }
                let ra = rand_at(0x71_BAF5);
                let rb = rand_at(0x71_BAFC);
                let k2 = |r: i32| xi((r & 0x7F) - 0x40).fmul_m(0.002).to_f32();
                let (sb, sa) = (k2(rb), k2(ra));
                let sc = k2(rand_at(0x71_BB31));
                wr::<f32>(pane, 0x54, sb);
                wr::<f32>(pane, 0x58, sa);
                wr::<f32>(pane, 0x5C, sc);
                let now = g::<u32>(0xB7_CB84).read();
                match ty {
                    0 | 2 => wr::<u32>(pane, 0x60, now),
                    1 => {
                        let d: [X87; 3] = core::array::from_fn(|k| x(rd::<f32>(pane, 0x30 + 4 * k as isize)).fsub_m(c[k]));
                        let l = d[2].fmul(d[2]).fadd(d[1].fmul(d[1])).fadd(d[0].fmul(d[0])).fsqrt();
                        wr::<u32>(pane, 0x60, ftol32(l.fmul_m(100.0).fadd(u32_x87(now))) as u32);
                    }
                    _ => {}
                }
                wr::<f32>(pane, 0x64, ground);
                wr::<u8>(pane, 0x6E, shatter as u8);
                wr::<f32>(pane, 0x68, size_y);
                wr::<u8>(pane, 0x6F, b as u8);
                wr::<u8>(pane, 0x6D, 1);
            }
        }
    }
}

/// `CGlass::AskForObjectToBeRenderedInGlass(CEntity*)` (0x71ACD0): appended while fewer than 31 are
/// listed (unsigned compare).
#[hook(addr = 0x71ACD0, conv = cdecl, name = "CGlass::AskForObjectToBeRenderedInGlass", expect = "A1 D8 1B C7 00")]
pub fn ask_for_object_to_be_rendered_in_glass(e: u32) {
    let n = NUM_GLASS_ENTITIES.get();
    if n >= 0x1F {
        return;
    }
    ENTITIES_TO_BE_RENDERED
        .ptr()
        .cast::<u32>()
        .wrapping_index(n as i32)
        .write(e);
    NUM_GLASS_ENTITIES.set(n + 1);
}

/// `CGlass::CalcAlphaWithNormal(CVector* normal)` (0x71ACF0): the camera front `f` reflected on the
/// normal (`r = f - 2 (f . n) n`, the dot summed `(fy ny + fz nz) + fx nx`, `2 (f . n) ny` and
/// `2 (f . n) nz` stored as floats), `v = (rx + ry) * 0.57 - rz * 0.57`, alpha = `_ftol(v^6 * 235 +
/// 20)` in EAX.
#[hook(addr = 0x71ACF0, conv = cdecl, name = "CGlass::CalcAlphaWithNormal", expect = "83 EC 18 A1 AC F9 B6 00")]
pub fn calc_alpha_with_normal(n: GPtr<f32>) -> i32 {
    let f = g::<f32>(CAMERA_FRONT);
    let (fx, fy, fz) = (f.read(), f.wrapping_index(1).read(), f.wrapping_index(2).read());
    let (nx, ny, nz) = (n.read(), n.wrapping_index(1).read(), n.wrapping_index(2).read());
    let dot = x(fy).fmul_m(ny).fadd(x(fz).fmul_m(nz)).fadd(x(fx).fmul_m(nx));
    let d = dot.fadd(dot);
    let dnx = d.fmul_m(nx);
    let dny = d.fmul_m(ny).to_f32();
    let dnz = d.fmul_m(nz).to_f32();
    let rx = dnx.fsubr_m(fx);
    let ry = x(fy).fsub_m(dny);
    let rz = x(fz).fsub_m(dnz);
    let kk = k(0x3F11_EB85);
    let v = rx.fadd(ry).fmul_m(kk).fsub(rz.fmul_m(kk));
    let mut p = v;
    for _ in 0..5 {
        p = p.fmul(v);
    }
    ftol32(p.fmul_m(k(0x436B_0000)).fadd_m(k(0x41A0_0000)))
}

/// `CGlass::Update()` (0x71B0D0): `CFallingGlassPane::Update` of every existing pane.
#[hook(addr = 0x71B0D0, conv = cdecl, name = "CGlass::Update", expect = "56 BE F8 1B C7 00")]
pub fn update() {
    for i in 0..45 {
        let p = pane(i);
        if rd::<u8>(p, 0x6D) != 0 {
            pane_update(p);
        }
    }
}

/// The common draw of the three poly renderers: `RwIm3DTransform(verts, POLY_INDICES[vcell] - vbase, 0,
/// 1)` and, when it succeeded, an indexed triangle list (`rwPRIMTYPETRILIST` 3) of `POLY_INDICES[icell] -
/// ibase` indices from `indices`, then `RwIm3DEnd`. The counters are read after the render states, as
/// the original does.
fn draw_polys(verts: usize, vcell: i32, vbase: u32, indices: usize, icell: i32, ibase: u32) {
    let ix = POLY_INDICES.ptr().cast::<u32>();
    let nverts = ix.wrapping_index(vcell).read().wrapping_sub(vbase);
    if !rw_im3d_transform(g(verts), nverts, GPtr::from_addr(0), 1).is_null() {
        let nidx = ix.wrapping_index(icell).read().wrapping_sub(ibase);
        rw_im3d_render_indexed_primitive(3, g(indices), nidx as i32);
        rw_im3d_end();
    }
}

/// The raster of the `RwTexture*` stored at `cell` (the texture's first field).
fn raster_of(cell: usize) -> u32 {
    rd::<u32>(g::<GPtr<u8>>(cell).read(), 0)
}

/// `CGlass::RenderHiLightPolys()` (0x71ADA0): with hi-light vertices, blend 2 / 2 and then the
/// cracked-glass raster, the batch from `TempBufferVertices[0]` / indices `[0]`; both counters reset.
#[hook(addr = 0x71ADA0, conv = cdecl, name = "CGlass::RenderHiLightPolys", expect = "A1 28 1B C7 00")]
pub fn render_hilight_polys() {
    let ix = POLY_INDICES.ptr().cast::<u32>();
    if ix.wrapping_index(4).read() == 0 {
        return;
    }
    rw_render_state_set(10, 2);
    rw_render_state_set(11, 2);
    rw_render_state_set(1, raster_of(CRACKED_GLASS_TEXTURE));
    draw_polys(TEMP_VERTICES, 4, 0, TEMP_INDICES, 5, 0);
    ix.wrapping_index(5).write(0);
    ix.wrapping_index(4).write(0);
}

/// `CGlass::RenderShatteredPolys()` (0x71AE30): when vertices were added past 0x400, the shatter raster
/// and blend 5 / 6, the batch from `TempBufferVertices[0x400]` / indices `[0x800]`; the bases are reset
/// to 0x400 / 0x800.
#[hook(addr = 0x71AE30, conv = cdecl, name = "CGlass::RenderShatteredPolys", expect = "81 3D 20 1B C7 00 00 04 00 00")]
pub fn render_shattered_polys() {
    let ix = POLY_INDICES.ptr().cast::<u32>();
    if ix.wrapping_index(2).read() == 0x400 {
        return;
    }
    rw_render_state_set(1, raster_of(SHATTER_TEXTURE));
    rw_render_state_set(10, 5);
    rw_render_state_set(11, 6);
    draw_polys(
        TEMP_VERTICES + 0x400 * 0x24,
        2,
        0x400,
        TEMP_INDICES + 0x800 * 2,
        3,
        0x800,
    );
    ix.wrapping_index(2).write(0x400);
    ix.wrapping_index(3).write(0x800);
}

/// `CGlass::RenderReflectionPolys()` (0x71AED0): when vertices were added past 0x600, the reflection
/// raster and blend 5 / 6, the batch from `TempBufferVertices[0x600]` / indices `[0xC00]`; the bases
/// are reset to 0x600 / 0xC00.
#[hook(addr = 0x71AED0, conv = cdecl, name = "CGlass::RenderReflectionPolys", expect = "81 3D 18 1B C7 00 00 06 00 00")]
pub fn render_reflection_polys() {
    let ix = POLY_INDICES.ptr().cast::<u32>();
    if ix.wrapping_index(0).read() == 0x600 {
        return;
    }
    rw_render_state_set(1, raster_of(REFLECTION_TEXTURE));
    rw_render_state_set(10, 5);
    rw_render_state_set(11, 6);
    draw_polys(
        TEMP_VERTICES + 0x600 * 0x24,
        0,
        0x600,
        TEMP_INDICES + 0xC00 * 2,
        1,
        0xC00,
    );
    ix.wrapping_index(0).write(0x600);
    ix.wrapping_index(1).write(0xC00);
}

/// `CGlass::Render()` (0x71CE20): resets the six poly counters, sets the render states (z-write off,
/// shade mode 2, vertex alpha on, the fog colour `0xFF << 24 | r << 16 | g << 8 | b` built with ORs of
/// the 16-bit components, blend 2 / 2, alpha test ref 1), renders every existing pane, then the three
/// poly batches, and restores the states.
#[hook(addr = 0x71CE20, conv = cdecl, name = "CGlass::Render", expect = "A1 24 7B C9 00")]
pub fn render() {
    let ix = POLY_INDICES.ptr().cast::<u32>();
    ix.wrapping_index(4).write(0);
    ix.wrapping_index(5).write(0);
    ix.wrapping_index(2).write(0x400);
    ix.wrapping_index(3).write(0x800);
    ix.wrapping_index(0).write(0x600);
    ix.wrapping_index(1).write(0xC00);
    rw_render_state_set(8, 0);
    rw_render_state_set(9, 2);
    rw_render_state_set(0xE, 1);
    let fc = g::<u16>(FOG_COLOUR);
    let (r, gg, b) = (
        u32::from(fc.read()),
        u32::from(fc.wrapping_index(1).read()),
        u32::from(fc.wrapping_index(2).read()),
    );
    rw_render_state_set(0xF, ((((r | 0xFFFF_FF00) << 8) | gg) << 8) | b);
    rw_render_state_set(10, 2);
    rw_render_state_set(11, 2);
    rw_render_state_set(0xC, 1);
    for i in 0..45 {
        let p = pane(i);
        if rd::<u8>(p, 0x6D) != 0 {
            pane_render(p);
        }
    }
    call_render_hilight_polys();
    call_render_shattered_polys();
    call_render_reflection_polys();
    rw_render_state_set(8, 1);
    rw_render_state_set(6, 1);
    rw_render_state_set(10, 5);
    rw_render_state_set(11, 6);
    rw_render_state_set(0xE, 0);
}

/// `CGlass::WasGlassHitByBullet(CEntity*, CVector hitPos)` (0x71C0D0): a glass object (type 4, glass
/// model) that uses collision: the first hit marks it (`+0x140 |= 0x10`), later hits shatter it with
/// `WindowRespondsToCollision(e, 0, 0-vector, hitPos, false)` when `rand() & 3 == 2` (low byte).
#[hook(addr = 0x71C0D0, conv = cdecl, name = "CGlass::WasGlassHitByBullet", expect = "83 EC 0C 56 8B 74 24 14")]
pub fn was_glass_hit_by_bullet(e: GPtr<u8>, px: u32, py: u32, pz: u32) {
    if rd::<u8>(e, 0x36) & 7 != 4 || !is_glass_model(e) || rd::<u8>(e, 0x1C) & 1 == 0 {
        return;
    }
    let f = rd::<u32>(e, 0x140);
    if f & 0x10 == 0 {
        wr::<u32>(e, 0x140, f | 0x10);
        return;
    }
    let r = sa_core::orig::at_site(0x71C138, sa_core::crt::rand) as u8;
    if r & 3 != 2 {
        return;
    }
    window_responds_to_collision(e, f32::from_bits(0), f32::from_bits(0), f32::from_bits(0), f32::from_bits(0), f32::from_bits(px), f32::from_bits(py), f32::from_bits(pz), 0);
}

/// `CGlass::WindowRespondsToSoftCollision(CEntity*, float force)` (0x71AF70): an entity that uses
/// collision, hit with a force > 50 (`test ah, 0x41`), not yet marked: the glass collision sound (event
/// 0x7B at the matrix position, or the placement position without a matrix) and the mark
/// (`+0x140 |= 0x10`).
#[hook(addr = 0x71AF70, conv = cdecl, name = "CGlass::WindowRespondsToSoftCollision", expect = "56 8B 74 24 08")]
pub fn window_responds_to_soft_collision(e: GPtr<u8>, force: f32) {
    if rd::<u8>(e, 0x1C) & 1 == 0 || jne(x(force), x(k(0x4248_0000)), 0x41) || rd::<u8>(e, 0x140) & 0x10 != 0
    {
        return;
    }
    let m = rd::<GPtr<u8>>(e, 0x14);
    let pos = if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    };
    report_glass_collision_event(g(AUDIO_ENGINE), 0x7B, pos.cast());
    wr::<u32>(e, 0x140, rd::<u32>(e, 0x140) | 0x10);
}

/// `CWorld::ms_nCurrentScanCode` (uint16).
const CURRENT_SCAN_CODE: usize = 0xB7CD78;

use sa_world::world::common::entity_pos;

/// `CGlass::FindWindowSectorList(CPtrListSingleLink& list, float* minDist, CEntity** out, float x,
/// float y, float z)` (0x71AFC0): every glass object of the list not yet scanned this pass (the scan
/// code is set) whose distance `sqrt(((z - pz)^2 + (y - py)^2) + (x - px)^2)` is below `*minDist`
/// becomes the nearest (`*minDist`, `*out`). The original keeps the next node in its `list` argument
/// slot (stack only).
#[hook(addr = 0x71AFC0, conv = cdecl, name = "CGlass::FindWindowSectorList<CPtrListSingleLink<CPhysical*>>", expect = "8B 44 24 04 8B 00")]
pub fn find_window_sector_list(
    list: GPtr<GPtr<u8>>,
    min_dist: GPtr<f32>,
    out: GPtr<u32>,
    px: f32,
    py: f32,
    pz: f32,
) {
    let mut node = list.read();
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        node = rd::<GPtr<u8>>(node, 4);
        if rd::<u8>(e, 0x36) & 7 != 4 || !is_glass_model(e) {
            continue;
        }
        let code = g::<u16>(CURRENT_SCAN_CODE).read();
        if rd::<u16>(e, 0x2C) == code {
            continue;
        }
        wr::<u16>(e, 0x2C, code);
        let p = entity_pos(e);
        let dz = x(pz).fsub_m(rd::<f32>(p, 8));
        let dy = x(py).fsub_m(rd::<f32>(p, 4));
        let dx = x(px).fsub_m(rd::<f32>(p, 0));
        let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
        // `fcomp [minDist]` + `test ah, 5` + `jp`: kept unless d < *minDist.
        if !jp(d, x(min_dist.read()), 5) {
            min_dist.write(d.to_f32());
            out.write(e.addr() as u32);
        }
    }
}

/// `CGlass::WindowRespondsToExplosion(CEntity*, CVector pos)` (0x71C1A0): for an entity that uses
/// collision, `d = entityPos - pos`, `len = sqrt((dz^2 + dy^2) + dx^2)`: below 10 the window shatters
/// (`WindowRespondsToCollision(e, 10000, d * (0.3 / len), entityPos, true)`); otherwise below 30 it is
/// only marked (`+0x140 |= 0x10`).
#[hook(addr = 0x71C1A0, conv = cdecl, name = "CGlass::WindowRespondsToExplosion", expect = "8B 54 24 04 8A 42 1C")]
pub fn window_responds_to_explosion(e: GPtr<u8>, px: f32, py: f32, pz: f32) {
    if rd::<u8>(e, 0x1C) & 1 == 0 {
        return;
    }
    let p = entity_pos(e);
    let dx = x(rd::<f32>(p, 0)).fsub_m(px).to_f32();
    let dy = x(rd::<f32>(p, 4)).fsub_m(py).to_f32();
    let dz = x(rd::<f32>(p, 8)).fsub_m(pz).to_f32();
    let len = x(dz)
        .fmul_m(dz)
        .fadd(x(dy).fmul_m(dy))
        .fadd(x(dx).fmul_m(dx))
        .fsqrt();
    // `fcom 10.0` + `test ah, 5` + `jp`: shatters only when len < 10.
    if jp(len, x(k(0x4120_0000)), 5) {
        if !jp(len, x(k(0x41F0_0000)), 5) {
            wr::<u32>(e, 0x140, rd::<u32>(e, 0x140) | 0x10);
        }
        return;
    }
    let f = len.fdivr_m(k(0x3E99_999A));
    let sx = x(dx).fmul(f).to_f32();
    let sy = x(dy).fmul(f).to_f32();
    let sz = x(dz).fmul(f).to_f32();
    let p = entity_pos(e);
    window_responds_to_collision(
        e,
        f32::from_bits(0x461C_4000),
        f32::from_bits(sx.to_bits()),
        f32::from_bits(sy.to_bits()),
        f32::from_bits(sz.to_bits()),
        f32::from_bits(rd::<u32>(p, 0)),
        f32::from_bits(rd::<u32>(p, 4)),
        f32::from_bits(rd::<u32>(p, 8)),
        1,
    );
}

pub use sa_core::bindings::atexit;
pub use sa_core::bindings::eh_vector_ctor;
pub use sa_core::bindings::eh_vector_dtor;

/// `__init_CGlass__aGlassPanes` (0x854EE0): the 45 panes' constructor (0x71A8B0, destructor 0x71A8C0)
/// through the CRT vector iterator, then `_atexit(0x856BA0)`; returns `_atexit`'s EAX (`pop ecx; ret`).
#[hook(addr = 0x854EE0, conv = cdecl, name = "__init_CGlass__aGlassPanes", expect = "68 C0 A8 71 00")]
pub fn init_glass_panes() -> i32 {
    eh_vector_ctor(GLASS_PANES.ptr().cast(), 0x70, 0x2D, 0x71A8B0, 0x71A8C0);
    atexit(0x856BA0)
}

/// `__dtor_CGlass__aGlassPanes` (0x856BA0): the panes' destructor (0x71A8C0) through the CRT vector
/// iterator.
#[hook(addr = 0x856BA0, conv = cdecl, name = "__dtor_CGlass__aGlassPanes", expect = "68 C0 A8 71 00")]
pub fn dtor_glass_panes() {
    eh_vector_dtor(GLASS_PANES.ptr().cast(), 0x70, 0x2D, 0x71A8C0);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_IS_OBJECT_GLASS,
    HOOK_INIT,
    HOOK_FIND_FREE_PANE,
    HOOK_GENERATE_PANES_FOR_WINDOW,
    HOOK_ASK_FOR_OBJECT_TO_BE_RENDERED_IN_GLASS,
    HOOK_CALC_ALPHA_WITH_NORMAL,
    HOOK_UPDATE,
    HOOK_RENDER_HILIGHT_POLYS,
    HOOK_RENDER_SHATTERED_POLYS,
    HOOK_RENDER_REFLECTION_POLYS,
    HOOK_RENDER,
    HOOK_WAS_GLASS_HIT_BY_BULLET,
    HOOK_WINDOW_RESPONDS_TO_SOFT_COLLISION,
    HOOK_FIND_WINDOW_SECTOR_LIST,
    HOOK_WINDOW_RESPONDS_TO_EXPLOSION,
    HOOK_INIT_GLASS_PANES,
    HOOK_DTOR_GLASS_PANES,
];
