//! PORT-RENDER-2: `CGlass` (static class; `CFallingGlassPane` 0x70 bytes, `+0x6D` exists). Ported from the
//! 1.0 US bytes; gta-reversed `Glass.cpp` and `docs/re/specs/RENDER/CGlass.deviations.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{ftol32, g, jne, jp, k, rd, wr, x};

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

orig! {
    /// `RwRenderStateSet` (`RwEngineInstance` slot 0x20)
    pub fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwIm3DTransform(RxObjSpace3DVertex*, uint32 count, RwMatrix* ltm, uint32 flags)` (0x7EF450)
    pub fn rw_im3d_transform(verts: GPtr<u8>, count: u32, ltm: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderIndexedPrimitive(RwPrimitiveType, RwImVertexIndex*, int32)` (0x7EF550)
    pub fn rw_im3d_render_indexed_primitive(ty: u32, indices: GPtr<u8>, count: i32) -> i32 = cdecl 0x7EF550 => unported;
    /// `RwIm3DEnd()` (0x7EF520)
    pub fn rw_im3d_end() -> i32 = cdecl 0x7EF520 => unported;
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
    /// `CGlass::WindowRespondsToCollision(CEntity*, float, CVector, CVector, bool)` (0x71BC40); raw dwords
    pub fn window_responds_to_collision(e: GPtr<u8>, damage: u32, sx: u32, sy: u32, sz: u32, px: u32, py: u32, pz: u32, explosion: u32) = cdecl 0x71BC40 => unported;
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
    window_responds_to_collision(e, 0, 0, 0, 0, px, py, pz, 0);
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

/// The entity's position: the matrix position when it has a matrix, else the placement position.
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

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
        0x461C_4000,
        sx.to_bits(),
        sy.to_bits(),
        sz.to_bits(),
        rd::<u32>(p, 0),
        rd::<u32>(p, 4),
        rd::<u32>(p, 8),
        1,
    );
}

orig! {
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall)
    pub fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// CRT `eh vector destructor iterator` (0x821E02, stdcall)
    pub fn eh_vector_dtor(arr: GPtr<u8>, size: u32, n: u32, dtor: u32) = stdcall 0x821E02 => unported;
    /// CRT `_atexit` (0x821D1E)
    pub fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
}

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
