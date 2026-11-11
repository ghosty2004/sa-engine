//! PORT-RENDER-2: `CStencilShadows` and `CStencilShadowObject` (0x1C bytes: +0 owner entity, +4 uint16,
//! +6 type byte (1 object, 2 vehicle), +8/+0xC, +0x10 heap buffer, +0x14 next, +0x18 previous). Ported
//! from the 1.0 US bytes; gta-reversed `StencilShadows.cpp` and `docs/re/specs/RENDER/CStencilShadows.md`
//! as reading aids. The catalogue names of 0x710B50 (`RenderBuffer`) and 0x710CC0 (`sub_710CC0`) are kept
//! although their bytes set the sun direction and flush the shadow vertex buffer.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::platform::{RS_MAXIMUM_HEIGHT, RS_MAXIMUM_WIDTH};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd, wr};

sa_core::globals! {
    list GLOBALS;
    /// `CStencilShadowObjects::pFreeObjects` (head of the free list)
    pub static FREE_OBJECTS @ 0xC6A168: u32 = 0;
    /// `CStencilShadowObjects::pActiveObjects` (head of the active list)
    pub static ACTIVE_OBJECTS @ 0xC6A16C: u32 = 0;
    /// The three 0x6000-byte work buffers of `Process` (heap)
    pub static WORK_BUFFERS @ 0xC6A170: [u32; 3] = [0; 3];
    /// `CStencilShadows::m_StencilShadowObjects[64]` (CStencilShadowObject, 0x1C bytes)
    pub static OBJECTS @ 0xC6A198: [u8; 64 * 0x1C] = [0; 64 * 0x1C];
    /// The object-shadow frame slot (0..3) and the register frame counter of `Process`
    pub static PROCESS_COUNTERS @ 0xC6A898: [u8; 2] = [0; 2];
    /// The sun direction used by the stencil shadows (CVector, normalised)
    pub static SUN_DIRECTION @ 0x8D5244: [f32; 3] = [1.0, 1.0, -2.0];
}

/// `TempBufferVerticesStored` / `TempBufferIndicesStored` (shared with the other Im3D batchers).
const TEMP_VERTICES_STORED: usize = 0xC4B950;
const TEMP_INDICES_STORED: usize = 0xC4B954;
const TEMP_VERTICES: usize = 0xC4D958;
const TEMP_INDICES: usize = 0xC4B958;
/// `g_fx` (Fx_c).
const FX: usize = 0xA9AE00;
/// `Scene.m_pRwCamera`.
const SCENE_CAMERA: usize = 0xC1703C;

orig! {
    /// `RwRenderStateSet` (`RwEngineInstance` slot 0x20)
    pub fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwIm3DTransform(RxObjSpace3DVertex*, uint32 count, RwMatrix* ltm, uint32 flags)` (0x7EF450)
    pub fn rw_im3d_transform(verts: GPtr<u8>, count: u32, ltm: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderIndexedPrimitive(RwPrimitiveType, RwImVertexIndex*, int32)` (0x7EF550)
    pub fn rw_im3d_render_indexed_primitive(ty: u32, indices: GPtr<u8>, count: i32) -> i32 = cdecl 0x7EF550 => unported;
    /// `RwIm3DEnd()` (0x7EF520)
    pub fn rw_im3d_end() -> i32 = cdecl 0x7EF520 => unported;
    /// `RwD3D9SetStencilClear(uint32)` (0x7F9D30)
    pub fn rw_d3d9_set_stencil_clear(v: u32) -> i32 = cdecl 0x7F9D30 => unported;
    /// `LittleTest()` (0x541330)
    pub fn little_test() = cdecl 0x541330 => unported;
    /// `Fx_c::GetFxQuality()` (0x49EA50)
    pub fn get_fx_quality(this: GPtr<u8>) -> u32 = thiscall 0x49EA50 => crate::fx::fx_get_fx_quality;
    /// `CVector::Normalise()` (0x59C910)
    pub fn vector_normalise(this: GPtr<f32>) = thiscall 0x59C910 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    pub fn clean_up_old_reference(this: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CMemoryMgr::Malloc(uint32 size, uint32 hint)` (0x72F420)
    pub fn memory_malloc(size: u32, hint: u32) -> u32 = cdecl 0x72F420 => unported;
    /// `CMemoryMgr::Free(void*)` (0x72F430)
    pub fn memory_free(p: u32) = cdecl 0x72F430 => unported;
    /// `CStencilShadowObject::Destroy()` (0x711310)
    pub fn call_destroy(this: GPtr<u8>) = thiscall 0x711310 => destroy;
    /// `CStencilShadows::RegisterStencilShadows(CVector* camPos, bool)` (0x711760)
    pub fn register_stencil_shadows(cam: GPtr<u8>, flag: u32) = cdecl 0x711760 => unported;
    /// `CStencilShadows::RenderForVehicle(CStencilShadowObject*, CVector*)` (0x70FAE0)
    pub fn render_for_vehicle(o: GPtr<u8>, cam: GPtr<u8>) = cdecl 0x70FAE0 => unported;
    /// `CStencilShadows::RenderForObject(CStencilShadowObject*, CVector*)` (0x710310)
    pub fn render_for_object(o: GPtr<u8>, cam: GPtr<u8>) = cdecl 0x710310 => unported;
}

/// `CStencilShadowObject::CStencilShadowObject()` (0x70F8D0): `mov eax, ecx; ret` (returns `this`). Not
/// hooked: the entry is 3 bytes.
pub fn constructor(this: GPtr<u8>) -> GPtr<u8> {
    this
}

/// Fx quality >= 1 and a frame buffer depth >= 32 (`Scene.m_pRwCamera->frameBuffer->depth`).
fn high_quality() -> bool {
    if (get_fx_quality(g(FX)) as i32) < 1 {
        return false;
    }
    let cam = g::<GPtr<u8>>(SCENE_CAMERA).read();
    rd::<i32>(rd::<GPtr<u8>>(cam, 0x60), 0x14) >= 0x20
}

/// `CStencilShadows::GraphicsHighQuality()` (0x70F9B0): FX quality >= 1 and a 32-bit frame buffer (AL;
/// the upper EAX bytes differ between the paths and are not used by the callers).
#[hook(addr = 0x70F9B0, conv = cdecl, name = "CStencilShadows::GraphicsHighQuality", expect = "53 B9 00 AE A9 00")]
pub fn graphics_high_quality() -> u8 {
    u8::from(high_quality())
}

fn object(i: i32) -> GPtr<u8> {
    OBJECTS.ptr().cast::<u8>().byte_add(i as isize * 0x1C)
}

/// `CStencilShadows::Init()` (0x70F9E0): `RwD3D9SetStencilClear(0)`, every object cleared and chained
/// into the free list (previous / next), no active object. Returns 1 in AL.
#[hook(addr = 0x70F9E0, conv = cdecl, name = "CStencilShadows::Init", expect = "53 33 DB 53 E8 47 A3 0E 00")]
pub fn init() -> u8 {
    rw_d3d9_set_stencil_clear(0);
    FREE_OBJECTS.set(object(0).addr() as u32);
    ACTIVE_OBJECTS.set(0);
    for i in 0..64 {
        let e = object(i);
        wr::<u32>(e, 0, 0);
        wr::<u16>(e, 4, 0);
        wr::<u8>(e, 6, 0);
        wr::<u32>(e, 0xC, 0);
        wr::<u32>(e, 8, 0);
        wr::<u32>(e, 0x10, 0);
        if i == 0 {
            wr::<u32>(e, 0x18, 0);
        } else {
            wr::<u32>(e, 0x18, e.byte_add(-0x1C).addr() as u32);
            if i == 63 {
                wr::<u32>(e, 0x14, 0);
                continue;
            }
        }
        wr::<u32>(e, 0x14, e.byte_add(0x1C).addr() as u32);
    }
    1
}

/// Normalises `v` (`CVector::Normalise` on a stack copy) into the sun direction.
fn set_sun_direction(v: [f32; 3]) {
    let mut t = v;
    vector_normalise(GPtr::from_raw(t.as_mut_ptr()));
    let s = SUN_DIRECTION.ptr().cast::<f32>();
    s.wrapping_index(2).write(t[2]);
    s.write(t[0]);
    s.wrapping_index(1).write(t[1]);
}

/// `CStencilShadows::RenderBuffer` (catalogue name; 0x710B50): the sun direction becomes the normalised
/// copy of `*v`. Returns 1 in AL.
#[hook(addr = 0x710B50, conv = cdecl, name = "CStencilShadows::RenderBuffer", expect = "83 EC 0C 8B 44 24 10")]
pub fn render_buffer(v: GPtr<f32>) -> u8 {
    set_sun_direction([v.read(), v.wrapping_index(1).read(), v.wrapping_index(2).read()]);
    1
}

/// `CStencilShadows::SunSetPositionFromEntity` (0x710AF0): null gives 0; otherwise the sun direction
/// becomes the normalised position (`+0x30`) of the matrix at `p+4`. Returns 1 in AL.
#[hook(addr = 0x710AF0, conv = cdecl, name = "CStencilShadows::SunSetPositionFromEntity", expect = "8B 44 24 04 83 EC 0C")]
pub fn sun_set_position_from_entity(p: GPtr<u8>) -> u8 {
    if p.is_null() {
        return 0;
    }
    let m = rd::<GPtr<u8>>(p, 4);
    set_sun_direction([rd::<f32>(m, 0x30), rd::<f32>(m, 0x34), rd::<f32>(m, 0x38)]);
    1
}

/// `CStencilShadowObject::Destroy()` (0x711310): drops the owner reference
/// (`CleanUpOldReference(&owner)` when set), clears the fields, frees the heap buffer, unlinks the
/// object from the active list and pushes it on the free list.
#[hook(addr = 0x711310, conv = thiscall, name = "CStencilShadowObject::Destroy", expect = "53 56 8B F1 8B 0E")]
pub fn destroy(this: GPtr<u8>) {
    let owner = rd::<GPtr<u8>>(this, 0);
    if !owner.is_null() {
        clean_up_old_reference(owner, this);
    }
    let buf = rd::<u32>(this, 0x10);
    wr::<u32>(this, 0, 0);
    wr::<u8>(this, 6, 0);
    wr::<u32>(this, 0xC, 0);
    wr::<u32>(this, 8, 0);
    if buf != 0 {
        memory_free(buf);
        wr::<u32>(this, 0x10, 0);
    }
    let prev = rd::<GPtr<u8>>(this, 0x18);
    if prev.is_null() {
        let next = rd::<GPtr<u8>>(this, 0x14);
        ACTIVE_OBJECTS.set(next.addr() as u32);
        if !next.is_null() {
            wr::<u32>(next, 0x18, 0);
        }
    } else {
        let next = rd::<GPtr<u8>>(this, 0x14);
        if next.is_null() {
            wr::<u32>(prev, 0x14, 0);
        } else {
            wr::<u32>(next, 0x18, prev.addr() as u32);
            wr::<u32>(rd::<GPtr<u8>>(this, 0x18), 0x14, rd::<u32>(this, 0x14));
        }
    }
    wr::<u32>(this, 0x14, FREE_OBJECTS.get());
    wr::<u32>(this, 0x18, 0);
    FREE_OBJECTS.set(this.addr() as u32);
    let next = rd::<GPtr<u8>>(this, 0x14);
    if !next.is_null() {
        wr::<u32>(next, 0x18, this.addr() as u32);
    }
}

/// `CStencilShadows::Shutdown()` (0x711390): destroys every active object (the next pointer read
/// before each `Destroy`).
#[hook(addr = 0x711390, conv = cdecl, name = "CStencilShadows::Shutdown", expect = "8B 0D 6C A1 C6 00")]
pub fn shutdown() {
    let mut o = GPtr::<u8>::from_addr(ACTIVE_OBJECTS.get() as usize);
    while !o.is_null() {
        let next = rd::<GPtr<u8>>(o, 0x14);
        call_destroy(o);
        o = next;
    }
}

/// `CStencilShadows::UpdateHierarchy(CStencilShadowObject** from, CStencilShadowObject** to,
/// CStencilShadowObject* obj)` (0x710BC0): unlinks `obj` from the `from` list and pushes it on the
/// front of `to`. EAX = `obj`.
#[hook(addr = 0x710BC0, conv = cdecl, name = "CStencilShadows::UpdateHierarchy", expect = "8B 44 24 0C 8B 48 18")]
pub fn update_hierarchy(from: GPtr<u32>, to: GPtr<u32>, obj: GPtr<u8>) -> u32 {
    let prev = rd::<GPtr<u8>>(obj, 0x18);
    if prev.is_null() {
        let next = rd::<GPtr<u8>>(obj, 0x14);
        from.write(next.addr() as u32);
        if !next.is_null() {
            wr::<u32>(next, 0x18, 0);
        }
    } else {
        let next = rd::<GPtr<u8>>(obj, 0x14);
        if next.is_null() {
            wr::<u32>(prev, 0x14, 0);
        } else {
            wr::<u32>(next, 0x18, prev.addr() as u32);
            wr::<u32>(rd::<GPtr<u8>>(obj, 0x18), 0x14, rd::<u32>(obj, 0x14));
        }
    }
    wr::<u32>(obj, 0x14, to.read());
    wr::<u32>(obj, 0x18, 0);
    to.write(obj.addr() as u32);
    let next = rd::<GPtr<u8>>(obj, 0x14);
    if !next.is_null() {
        wr::<u32>(next, 0x18, obj.addr() as u32);
    }
    obj.addr() as u32
}

/// `CStencilShadows::sub_710CC0(int32 addIndices, int32 addVertices)` (0x710CC0): when the stored batch
/// would overflow (indices + n >= 0x1000 or vertices + m >= 0x800, signed) and holds indices and
/// vertices: raster off, `LittleTest`, `RwIm3DTransform(flags 0x18)`, the indexed triangle list,
/// `RwIm3DEnd`; then both counts are reset.
#[hook(addr = 0x710CC0, conv = cdecl, name = "CStencilShadows::sub_710CC0", expect = "8B 4C 24 04 A1 54 B9 C4 00")]
pub fn sub_710cc0(add_indices: i32, add_vertices: i32) {
    let ni = g::<i32>(TEMP_INDICES_STORED).read();
    let nv = g::<i32>(TEMP_VERTICES_STORED).read();
    if ni.wrapping_add(add_indices) < 0x1000 && add_vertices.wrapping_add(nv) < 0x800 {
        return;
    }
    if ni == 0 || nv == 0 {
        return;
    }
    rw_render_state_set(1, 0);
    little_test();
    let nv = g::<u32>(TEMP_VERTICES_STORED).read();
    if !rw_im3d_transform(g(TEMP_VERTICES), nv, GPtr::from_addr(0), 0x18).is_null() {
        rw_im3d_render_indexed_primitive(3, g(TEMP_INDICES), g::<i32>(TEMP_INDICES_STORED).read());
        rw_im3d_end();
    }
    g::<u32>(TEMP_VERTICES_STORED).write(0);
    g::<u32>(TEMP_INDICES_STORED).write(0);
}

/// `CStencilShadows::Process(CVector* camPos)` (0x711D90): only with high-quality graphics (else 0 in
/// AL). Registers the shadows (`RegisterStencilShadows(camPos, counter & 7 != 0)` after incrementing the
/// counter at 0xC6A899), allocates three 0x6000-byte work buffers, renders every active object
/// (vehicles every frame, objects when their running index & 3 matches the slot at 0xC6A898), advances
/// the slot (& 3), frees the buffers. Returns 1 in AL.
#[hook(addr = 0x711D90, conv = cdecl, name = "CStencilShadows::Process", expect = "B9 00 AE A9 00")]
pub fn process(cam: GPtr<u8>) -> u8 {
    if !high_quality() {
        return 0;
    }
    let c = PROCESS_COUNTERS.ptr().cast::<u8>();
    let n = c.wrapping_index(1).read().wrapping_add(1);
    c.wrapping_index(1).write(n);
    register_stencil_shadows(cam, u32::from(n & 7 != 0));
    let w = WORK_BUFFERS.ptr().cast::<u32>();
    for i in 0..3 {
        w.wrapping_index(i).write(memory_malloc(0x6000, 0));
    }
    let mut idx: u32 = 0;
    let mut o = GPtr::<u8>::from_addr(ACTIVE_OBJECTS.get() as usize);
    while !o.is_null() {
        let next = rd::<GPtr<u8>>(o, 0x14);
        match rd::<u8>(o, 6) {
            1 => {
                if (idx & 3) as u8 == c.read() {
                    render_for_object(o, cam);
                }
                idx = idx.wrapping_add(1);
            }
            2 => render_for_vehicle(o, cam),
            _ => {}
        }
        o = next;
    }
    c.write(c.read().wrapping_add(1) & 3);
    for i in 0..3 {
        memory_free(w.wrapping_index(i).read());
        w.wrapping_index(i).write(0);
    }
    1
}

orig! {
    /// `CRGBA::CRGBA(uint8, uint8, uint8, uint8)` (0x7170C0, catalogue `FUN_007170c0`); returns `this`
    pub fn rgba_ctor(this: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `CStencilShadows::Render(CRGBA*)` (0x710D50)
    pub fn stencil_render(rgba: GPtr<u8>) = cdecl 0x710D50 => unported;
    /// `CSprite2d::InitPerFrame()` (0x727350)
    pub fn sprite2d_init_per_frame() = cdecl 0x727350 => unported;
    /// `CSprite2d::DrawRect(const CRect&, const CRGBA&)` (0x727B60)
    pub fn sprite2d_draw_rect(rect: GPtr<f32>, rgba: GPtr<u8>) = cdecl 0x727B60 => unported;
}

/// `CTimeCycle::m_CurrentColours.m_nShadowStrength` as the dword this code reads.
const SHADOW_STRENGTH: usize = 0xB7C4E8;

/// `RwRenderStateSet` over a list of (state, value) pairs, in order.
fn states(list: &[(u32, u32)]) {
    for &(s, v) in list {
        rw_render_state_set(s, v);
    }
}

/// `CStencilShadows::RenderStencilShadows()` (0x7113B0): only with high-quality graphics (else 0 in AL).
/// Sets the stencil render states, renders the shadow volumes twice (`Render(&CRGBA(0, 0, 0, 255))`,
/// first with stencil op 7 / cull 3, then op 8 / cull 2), then darkens the screen where the stencil is
/// set: `CSprite2d::DrawRect(CRect(0, height, width, 0), CRGBA(0, 0, 0, strength * 0x32 >> 8))` with the
/// time cycle's shadow strength, and restores the states. Returns 1 in AL.
#[hook(addr = 0x7113B0, conv = cdecl, name = "CStencilShadows::RenderStencilShadows", expect = "83 EC 18 B9 00 AE A9 00")]
pub fn render_stencil_shadows() -> u8 {
    if !high_quality() {
        return 0;
    }
    states(&[
        (8, 0),
        (0x15, 1),
        (7, 1),
        (0xE, 0),
        (1, 0),
        (0xC, 1),
        (0xA, 1),
        (0xB, 2),
        (0x1B, 0xFFFF_FFFF),
        (0x1C, 0xFFFF_FFFF),
        (0x19, 8),
        (0x16, 1),
        (0x17, 1),
        (0x18, 1),
        (0x1A, 0),
        (6, 1),
        (0x17, 7),
        (0x14, 3),
    ]);
    let mut rgba = [0u8; 4];
    let rp = GPtr::<u8>::from_raw(rgba.as_mut_ptr());
    stencil_render(rgba_ctor(rp, 0, 0, 0, 0xFF));
    states(&[(0x17, 8), (0x14, 2)]);
    stencil_render(rgba_ctor(rp, 0, 0, 0, 0xFF));
    states(&[
        (0xC, 0),
        (8, 1),
        (6, 1),
        (0x15, 0),
        (7, 2),
        (0x14, 2),
        (8, 0),
        (6, 0),
        (0x15, 1),
        (0xE, 0),
        (0xC, 1),
        (0xA, 5),
        (0xB, 6),
        (7, 1),
        (1, 0),
        (0x14, 1),
        (0x1A, 1),
        (0x19, 4),
        (0x16, 1),
        (0x17, 1),
        (0x18, 1),
    ]);
    sprite2d_init_per_frame();
    rgba_ctor(rp, 0, 0, 0, 0x32);
    let w = X87::from_i32(RS_MAXIMUM_WIDTH.get()).to_f32();
    let a = g::<i32>(SHADOW_STRENGTH)
        .read()
        .wrapping_mul(i32::from(rp.wrapping_index(3).read()))
        >> 8;
    rp.wrapping_index(3).write(a as u8);
    let h = X87::from_i32(RS_MAXIMUM_HEIGHT.get()).to_f32();
    let mut rect = [0.0f32, h, w, 0.0];
    sprite2d_draw_rect(GPtr::from_raw(rect.as_mut_ptr()), rp);
    states(&[(0x15, 0), (8, 1), (6, 1), (0xC, 0), (7, 2), (0x14, 2)]);
    1
}

orig! {
    /// `CStencilShadows::sub_710CC0(int32, int32)` (0x710CC0): flushes the batch when it would overflow
    pub fn call_sub_710cc0(add_indices: i32, add_vertices: i32) = cdecl 0x710CC0 => sub_710cc0;
}

/// Appends `n` vertices with the positions `p` (CVector each) and the ARGB colour `colour` to the
/// stored Im3D batch, with the indices `ni..ni + n` (the index count is the base, as in the original),
/// after `sub_710CC0(n, n)` has flushed the batch if needed.
fn add_strip(p: &[[u32; 3]], colour: u32) {
    let n = p.len() as i32;
    call_sub_710cc0(n, n);
    let ni = g::<i32>(TEMP_INDICES_STORED).read();
    let nv = g::<i32>(TEMP_VERTICES_STORED).read();
    let idx = g::<u16>(TEMP_INDICES).wrapping_index(ni);
    for k in 0..n {
        idx.wrapping_index(k).write(ni.wrapping_add(k) as u16);
    }
    let v = g::<u8>(TEMP_VERTICES).byte_add(nv as isize * 0x24);
    for (k, q) in p.iter().enumerate() {
        let vk = v.byte_add(k as isize * 0x24);
        wr::<u32>(vk, 0, q[0]);
        wr::<u32>(vk, 4, q[1]);
        wr::<u32>(vk, 8, q[2]);
        wr::<u32>(vk, 0x18, colour);
    }
    g::<i32>(TEMP_INDICES_STORED).write(ni.wrapping_add(n));
    g::<i32>(TEMP_VERTICES_STORED).write(nv.wrapping_add(n));
}

/// `CStencilShadows::Render(CRGBA* colour)` (0x710D50): resets the stored Im3D batch, then for every
/// active shadow object adds its volume (the `+0x10` buffer of `+0xC` CVectors): `count / 6` (signed)
/// six-vertex groups, then one triangle after them when `count / 3` is odd, all in the colour
/// `a << 24 | r << 16 | g << 8 | b`; flushes the rest (raster off, `LittleTest`, transform flags 0x18,
/// indexed triangle list). Returns 1 in AL.
#[hook(addr = 0x710D50, conv = cdecl, name = "CStencilShadows::Render", expect = "81 EC 84 00 00 00")]
pub fn render(colour: GPtr<u8>) -> u8 {
    let c = colour.cast::<u32>().read();
    let argb = (c >> 24) << 24 | (c & 0xFF) << 16 | ((c >> 8) & 0xFF) << 8 | ((c >> 16) & 0xFF);
    g::<u32>(TEMP_INDICES_STORED).write(0);
    g::<u32>(TEMP_VERTICES_STORED).write(0);
    let mut o = GPtr::<u8>::from_addr(ACTIVE_OBJECTS.get() as usize);
    if o.is_null() {
        return 1;
    }
    while !o.is_null() {
        let next = rd::<GPtr<u8>>(o, 0x14);
        let count = rd::<i32>(o, 0xC);
        let buf = rd::<GPtr<u8>>(o, 0x10);
        let vec = |i: i32| {
            let p = buf.byte_add(i as isize * 0xC);
            [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)]
        };
        let quads = count / 6;
        let mut done = 0;
        if quads > 0 {
            for q in 0..quads {
                let b = q * 6;
                add_strip(
                    &[vec(b), vec(b + 1), vec(b + 2), vec(b + 3), vec(b + 4), vec(b + 5)],
                    argb,
                );
            }
            done = quads * 6;
        }
        if (rd::<i32>(o, 0xC) / 3) & 1 != 0 {
            add_strip(&[vec(done), vec(done + 1), vec(done + 2)], argb);
        }
        o = next;
    }
    let ni = g::<u32>(TEMP_INDICES_STORED).read();
    let nv = g::<u32>(TEMP_VERTICES_STORED).read();
    if ni == 0 || nv == 0 {
        return 1;
    }
    rw_render_state_set(1, 0);
    little_test();
    if !rw_im3d_transform(
        g(TEMP_VERTICES),
        g::<u32>(TEMP_VERTICES_STORED).read(),
        GPtr::from_addr(0),
        0x18,
    )
    .is_null()
    {
        rw_im3d_render_indexed_primitive(3, g(TEMP_INDICES), g::<i32>(TEMP_INDICES_STORED).read());
        rw_im3d_end();
    }
    g::<u32>(TEMP_VERTICES_STORED).write(0);
    g::<u32>(TEMP_INDICES_STORED).write(0);
    1
}

orig! {
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall)
    pub fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// CRT `eh vector destructor iterator` (0x821E02, stdcall)
    pub fn eh_vector_dtor(arr: GPtr<u8>, size: u32, n: u32, dtor: u32) = stdcall 0x821E02 => unported;
    /// CRT `_atexit` (0x821D1E)
    pub fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
}

/// `__init_CStencilShadows__m_StencilShadowObjects` (0x854B60): the 64 objects' constructor (0x70F8D0,
/// destructor 0x70F8E0) through the CRT vector iterator, then `_atexit(0x856AE0)`; returns `_atexit`'s
/// EAX (`pop ecx; ret`).
#[hook(addr = 0x854B60, conv = cdecl, name = "__init_CStencilShadows__m_StencilShadowObjects", expect = "68 E0 F8 70 00")]
pub fn init_stencil_shadow_objects() -> i32 {
    eh_vector_ctor(OBJECTS.ptr().cast(), 0x1C, 0x40, 0x70F8D0, 0x70F8E0);
    atexit(0x856AE0)
}

/// `__dtor_CStencilShadows__m_StencilShadowObjects` (0x856AE0): the objects' destructor (0x70F8E0)
/// through the CRT vector iterator.
#[hook(addr = 0x856AE0, conv = cdecl, name = "__dtor_CStencilShadows__m_StencilShadowObjects", expect = "68 E0 F8 70 00")]
pub fn dtor_stencil_shadow_objects() {
    eh_vector_dtor(OBJECTS.ptr().cast(), 0x1C, 0x40, 0x70F8E0);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_GRAPHICS_HIGH_QUALITY,
    HOOK_INIT,
    HOOK_RENDER_BUFFER,
    HOOK_SUN_SET_POSITION_FROM_ENTITY,
    HOOK_DESTROY,
    HOOK_SHUTDOWN,
    HOOK_UPDATE_HIERARCHY,
    HOOK_SUB_710CC0,
    HOOK_PROCESS,
    HOOK_RENDER_STENCIL_SHADOWS,
    HOOK_RENDER,
    HOOK_INIT_STENCIL_SHADOW_OBJECTS,
    HOOK_DTOR_STENCIL_SHADOW_OBJECTS,
];
