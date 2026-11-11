//! MVP-A writer batch `mvpa-render-1`: boot/loading-time RENDER functions (initialisers, shutdowns, the
//! game clock helpers the render side uses, sprite buffer helpers, the env-map pipe's pool constructors).
//! Ported from the 1.0 US bytes; gta-reversed as a reading aid. Every hook is unverified.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{Fpu, g, k, rd, wr};

/// `CTxdStore` "particle" (0x85C648).
const PARTICLE_TXD: usize = 0x85C648;
/// `RwRenderState` values used here.
const RS_TEXTURERASTER: u32 = 1;

orig! {
    /// `CTxdStore::PushCurrentTxd()` (0x7316A0)
    pub fn push_current_txd() = cdecl 0x7316A0 => sa_stream::txdstore::push_current_txd;
    /// `CTxdStore::PopCurrentTxd()` (0x7316B0)
    pub fn pop_current_txd() = cdecl 0x7316B0 => sa_stream::txdstore::pop_current_txd;
    /// `CTxdStore::SetCurrentTxd(int)` (0x7319C0)
    pub fn set_current_txd(slot: i32) = cdecl 0x7319C0 => sa_stream::txdstore::set_current_txd;
    /// `CTxdStore::FindTxdSlot(const char*)` (0x731850)
    pub fn find_txd_slot(name: GPtr<u8>) -> i32 = cdecl 0x731850 => sa_stream::txdstore::find_txd_slot_name;
    /// `RwTextureRead(const char* name, const char* mask)` (0x7F3AC0)
    pub fn rw_texture_read(name: GPtr<u8>, mask: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F3AC0 => unported;
    /// `RwTextureDestroy(RwTexture*)` (0x7F3820)
    pub fn rw_texture_destroy(tex: GPtr<u8>) -> i32 = cdecl 0x7F3820 => unported;
    /// `RwRasterDestroy(RwRaster*)` (0x7FB020)
    pub fn rw_raster_destroy(r: GPtr<u8>) -> i32 = cdecl 0x7FB020 => unported;
    /// `RwRasterLock(RwRaster*, RwUInt8 level, RwInt32 lockMode)` (0x7FB2D0)
    pub fn rw_raster_lock(r: GPtr<u8>, level: u32, mode: u32) -> GPtr<u8> = cdecl 0x7FB2D0 => unported;
    /// `RwRenderStateSet(state, value)`: `RwEngineInstance->dOpenDevice.fpRenderStateSet` (slot +0x20)
    pub fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwIm2DRenderPrimitive(type, verts, n)` (0x734E90)
    pub fn rw_im2d_render_primitive(ty: u32, verts: GPtr<u8>, n: i32) -> i32 = cdecl 0x734E90 => unported;
    /// `RwD3D9GetCaps()` (0x7FAD20): the device's `D3DCAPS9`
    pub fn rw_d3d9_get_caps() -> GPtr<u8> = cdecl 0x7FAD20 => unported;
    /// `operator new(size_t)` (0x82119A)
    pub fn operator_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// CRT `free` (0x8214BD)
    pub fn crt_free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// `CSprite2d::SetVertices(RwD3D9Vertex*, const CRect&, const CRGBA& c1..c4, float u1, v1 .. u4, v4)`
    /// (0x727A00)
    pub fn set_vertices_d3d9(v: GPtr<u8>, r: GPtr<u8>, c1: GPtr<u8>, c2: GPtr<u8>, c3: GPtr<u8>, c4: GPtr<u8>,
        u1: u32, v1: u32, u2: u32, v2: u32, u3: u32, v3: u32, u4: u32, v4: u32) = cdecl 0x727A00 => crate::sprite2d::set_vertices_d3d9;
    /// `CSprite2d::SetVertices(const CRect&, const CRGBA& c1..c4)` (0x727420)
    pub fn set_vertices_rect(r: GPtr<u8>, c1: GPtr<u8>, c2: GPtr<u8>, c3: GPtr<u8>, c4: GPtr<u8>) = cdecl 0x727420 => crate::sprite2d::set_vertices_rect;
    /// `CSprite2d::RenderVertexBuffer()` (0x7273D0)
    pub fn render_vertex_buffer() = cdecl 0x7273D0 => crate::sprite2d::render_vertex_buffer;
    /// `CRGBA::CRGBA(u8 r, u8 g, u8 b, u8 a)` (0x7170C0, thiscall, returns this)
    pub fn crgba_ctor(this: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// `CPlantSurfPropMgr::LoadPlantsDat(const char*)` (0x5DD3B0)
    pub fn load_plants_dat(name: GPtr<u8>) -> u8 = cdecl 0x5DD3B0 => unported;
    /// `C3dMarkers::Shutdown()` (0x722710)
    pub fn markers_shutdown() = cdecl 0x722710 => crate::markers::shutdown;
    /// `CPointLights::Init()` (0x6FFB40)
    pub fn point_lights_init() = cdecl 0x6FFB40 => crate::pointlights::init;
    /// `CEscalators::Init()` (0x717C50)
    pub fn escalators_init() = cdecl 0x717C50 => sa_world::world::escalators::init;
    /// `CEscalator::SwitchOff()` (0x717860)
    pub fn escalator_switch_off(e: GPtr<u8>) = thiscall 0x717860 => sa_world::world::escalators::switch_off;
    /// `CClouds::VolumetricCloudsInit()` (0x7131C0)
    pub fn volumetric_clouds_init() = cdecl 0x7131C0 => crate::clouds::volumetric_clouds_init;
    /// `CClouds::MovingFogInit()` (0x713660)
    pub fn moving_fog_init() = cdecl 0x713660 => crate::clouds::moving_fog_init;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    pub fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `CEntity` virtual slot +0x20 (`Remove()`-style no-argument virtual called before CWorld::Remove)
    pub fn entity_vslot_20(e: GPtr<u8>) = thiscall vslot 0x20 => unported;
    /// virtual slot 0: the scalar deleting destructor (`delete this` with flag 1)
    pub fn entity_delete(e: GPtr<u8>, flags: u32) -> GPtr<u8> = thiscall vslot 0 => unported;
    /// `List_c::AddItem(ListItem_c*)` (0x4A8DF0)
    pub fn list_add_item(l: GPtr<u8>, item: GPtr<u8>) = thiscall 0x4A8DF0 => unported;
    /// `List_c::RemoveItem(ListItem_c*)` (0x4A8E30)
    pub fn list_remove_item(l: GPtr<u8>, item: GPtr<u8>) = thiscall 0x4A8E30 => unported;
    /// `List_c::RemoveAll()` (0x4A8EB0)
    pub fn list_remove_all(l: GPtr<u8>) = thiscall 0x4A8EB0 => unported;
    /// `ProcObjectMan_c::LoadDataFile()` (0x5A3140)
    pub fn proc_object_man_load_data_file(this: GPtr<u8>) = thiscall 0x5A3140 => unported;
    /// `ProcSurfaceInfo_c::Exit()` (0x5A3270)
    pub fn proc_surface_info_exit_call(this: GPtr<u8>) = thiscall 0x5A3270 => proc_surface_info_exit;
    /// `SurfaceInfos_c::GetSurfaceIdFromName(const char*)` (0x55D220, on g_surfaceInfos 0xB79538)
    pub fn get_surface_id_from_name(this: GPtr<u8>, name: GPtr<u8>) -> u32 = thiscall 0x55D220 => unported;
    /// `CModelInfo::GetModelInfo(const char* name, int* index)` (0x4C5940)
    pub fn get_model_info_full(name: GPtr<u8>, index: GPtr<u8>) -> GPtr<u8> = cdecl 0x4C5940 => unported;
    /// `CRope::Remove()` (0x556780)
    pub fn rope_remove_call(this: GPtr<u8>) = thiscall 0x556780 => rope_remove;
    /// `FxManager_c::DestroyFxSystem(FxSystem_c*)` (0x4A9810)
    pub fn destroy_fx_system(this: GPtr<u8>, sys: GPtr<u8>) = thiscall 0x4A9810 => unported;
    /// `CRealTimeShadow::~CRealTimeShadow()` (0x706440)
    pub fn real_time_shadow_destructor(this: GPtr<u8>) = thiscall 0x706440 => unported;
    /// `CShadowCamera::Destroy()` (0x705400)
    pub fn shadow_camera_destroy(this: GPtr<u8>) = thiscall 0x705400 => unported;
    /// `CClock::NormaliseGameClock()` (0x52CDE0)
    pub fn normalise_game_clock_call() = cdecl 0x52CDE0 => normalise_game_clock;
}

/// `CSprite2d::Draw(const CRect& r, const CRGBA& c)` (0x728350, thiscall, `ret 8`): the quad in colour c
/// (`SetVertices(r, c, c, c, c)`), `RwRenderStateSet(TEXTURERASTER, texture ? texture->raster : NULL)`,
/// one triangle fan of `maVertices` (0xC80468), then `RwRenderStateSet(TEXTURERASTER, NULL)`. The null
/// texture path loads `RwEngineInstance` through the protection chunk 0x409B78 (decodes to 0xC97B24).
#[hook(addr = 0x728350, conv = thiscall, name = "CSprite2d::Draw-CRectCRGBA", expect = "8B 44 24 08 56 50 50 50 50",
       preserve = "ecx", unverified = "mvpa-render-1", route = "L2", hazards = "preserved-regs", recs = "grove-walk",
       note = "caller 0x59001B does `or cl, 0xFF` after the call (a byte write that ignores CL; the scan counts it as a read), so ECX is preserved only to satisfy the check")]
pub fn sprite2d_draw_rect(this: GPtr<u8>, r: GPtr<u8>, c: GPtr<u8>) {
    set_vertices_rect(r, c, c, c, c);
    let tex = rd::<GPtr<u8>>(this, 0);
    let raster = if tex.is_null() { 0 } else { rd::<u32>(tex, 0) };
    rw_render_state_set(RS_TEXTURERASTER, raster);
    rw_im2d_render_primitive(5, GPtr::from_addr(0xC80468), 4);
    rw_render_state_set(RS_TEXTURERASTER, 0);
}

/// `CSprite2d::AddToBuffer(const CRect& r, const CRGBA& c, float u1, v1, u2, v2, u3, v3, u4, v4)`
/// (0x728200, cdecl): `SetVertices(&TempVertexBuffer[n], r, c, c, c, c, u1..v4)` for
/// `n = nextBufferVertex` (TempVertexBuffer 0xC5F958, 0x1C-byte RwD3D9Vertex), indices (n, n+1, n+2,
/// n+3, n, n+2) at `aTempBufferIndices[nextBufferIndex]` (0xC4B958, written in the original's order
/// +1, +0, +4, +2, +3, +5), `nextBufferVertex += 4`, `nextBufferIndex += 6`, then
/// `RenderVertexBuffer()` when `n + 4 > 0x5FC` or `index + 6 > 0xFFA` (signed).
#[hook(addr = 0x728200, conv = cdecl, name = "CSprite2d::AddToBuffer", expect = "8B 44 24 28 8B 4C 24 24",
       unverified = "mvpa-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
#[allow(clippy::too_many_arguments)]
pub fn sprite2d_add_to_buffer(
    r: GPtr<u8>,
    c: GPtr<u8>,
    u1: u32,
    v1: u32,
    u2: u32,
    v2: u32,
    u3: u32,
    v3: u32,
    u4: u32,
    v4: u32,
) {
    let n = g::<i32>(0xC8045C).read();
    let vtx = GPtr::<u8>::from_addr((n.wrapping_mul(0x1C) as u32).wrapping_add(0xC5F958) as usize);
    set_vertices_d3d9(vtx, r, c, c, c, c, u1, v1, u2, v2, u3, v3, u4, v4);
    let ix = g::<i32>(0xC80458).read();
    let idx = GPtr::<u16>::from_addr(0xC4B958);
    let w = |o: i32, v: i32| idx.wrapping_index(ix.wrapping_add(o)).write(v as u16);
    w(1, n.wrapping_add(1));
    w(0, n);
    w(4, n);
    let n4 = n.wrapping_add(4);
    w(2, n.wrapping_add(2));
    w(3, n.wrapping_add(3));
    w(5, n.wrapping_add(2));
    let ix6 = ix.wrapping_add(6);
    g::<i32>(0xC8045C).write(n4);
    g::<i32>(0xC80458).write(ix6);
    if n4 > 0x5FC || ix6 > 0xFFA {
        render_vertex_buffer();
    }
}

/// `D3DResourceSystem::Init()` (0x730830, cdecl): resets the D3D resource caches. `[0xC87C60] = 1`,
/// `UseD3DResourceBuffering (0x8D6084) = 1`, `[0xC87C68] = 0`, then 0x18-byte list headers (+0 type,
/// +4 size, +8 capacity 0x10, +0xC/+0x10 counts 0, +0x14 = `operator new(0x40)`): one at 0xC87C6C with
/// (0, -1); 16 at 0xC87E48.. with (0x65, 100 * (i + 1)); one at 0xC87FC8 with (0x65, 0). Stores in the
/// original order around each allocation.
#[hook(addr = 0x730830, conv = cdecl, name = "D3DResourceSystem::Init", expect = "53 55 56 B8 01 00 00 00",
       unverified = "mvpa-render-1", route = "split", hazards = "none", recs = "grove-walk",
       note = "one-time initialiser: allocates 18 0x40-byte arrays with operator new")]
pub fn d3d_resource_system_init() {
    g::<u32>(0xC87C60).write(1);
    g::<u8>(0x8D6084).write(1);
    g::<u32>(0xC87C68).write(0);
    g::<u32>(0xC87C6C).write(0);
    g::<u32>(0xC87C70).write(0xFFFF_FFFF);
    let p = operator_new(0x40);
    g::<GPtr<u8>>(0xC87C80).write(p);
    g::<u32>(0xC87C74).write(0x10);
    g::<u32>(0xC87C7C).write(0);
    g::<u32>(0xC87C78).write(0);
    let mut size = 100u32;
    let mut e = 0xC87E5Cusize;
    while e < 0xC87FDC {
        g::<u32>(e - 0x14).write(0x65);
        g::<u32>(e - 0x10).write(size);
        let p = operator_new(0x40);
        g::<GPtr<u8>>(e).write(p);
        g::<u32>(e - 0xC).write(0x10);
        g::<u32>(e - 4).write(0);
        g::<u32>(e - 8).write(0);
        e += 0x18;
        size += 100;
    }
    g::<u32>(0xC87FC8).write(0x65);
    g::<u32>(0xC87FCC).write(0);
    let p = operator_new(0x40);
    g::<u32>(0xC87FD0).write(0x10);
    g::<u32>(0xC87FD8).write(0);
    g::<u32>(0xC87FD4).write(0);
    g::<GPtr<u8>>(0xC87FDC).write(p);
}

/// `D3DResourceSystem::SetUseD3DResourceBuffering(bool)` (0x730AC0, cdecl): stores the byte at 0x8D6084.
#[hook(addr = 0x730AC0, conv = cdecl, name = "D3DResourceSystem::SetUseD3DResourceBuffering", expect = "8A 44 24 04 A2 84 60 8D 00",
       unverified = "mvpa-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn d3d_resource_system_set_use_buffering(on: u8) {
    g::<u8>(0x8D6084).write(on);
}

/// `IsEnvironmentMappingSupported()` (0x5D8980, cdecl): from `RwD3D9GetCaps()`: 0 unless
/// `MaxTextureBlendStages (+0x94) > 1` (signed), `MaxSimultaneousTextures (+0x98) >= 2` (unsigned),
/// TextureOpCaps (+0x90) bits 13 and 24 set; then bit 9 of +0x38 (0 or 1 in the whole EAX).
#[hook(addr = 0x5D8980, conv = cdecl, name = "IsEnvironmentMappingSupported", expect = "E8 9B 23 22 00",
       unverified = "mvpa-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn is_environment_mapping_supported() -> u32 {
    let caps = rw_d3d9_get_caps();
    if rd::<i32>(caps, 0x94) <= 1 {
        return 0;
    }
    let ops = rd::<u32>(caps, 0x90);
    let stages_ok = rd::<u32>(caps, 0x98) >= 2;
    let (dl, cl) = ((ops >> 24) & 1, (ops >> 13) & 1);
    if !stages_ok || cl == 0 || dl == 0 {
        return 0;
    }
    (rd::<u32>(caps, 0x38) >> 9) & 1
}

/// The three `CPool<T>` constructors of the car env-map pipe (thiscall `(int size, const char*)`,
/// `ret 8`, return this): objects = `operator new(size * elem)`, flags = `operator new(size)`, +8 size,
/// +0xC firstFree -1, +0x10 bOwnsAllocations 1; every flag byte becomes `(b | 0x80) & 0x80` (two
/// read-modify-writes as in the original: empty, id 0).
fn env_map_pool_ctor(this: GPtr<u8>, size: i32, elem: i32) -> GPtr<u8> {
    let objs = operator_new(size.wrapping_mul(elem) as u32);
    wr::<GPtr<u8>>(this, 0, objs);
    let flags = operator_new(size as u32);
    wr::<GPtr<u8>>(this, 4, flags);
    wr::<u8>(this, 0x10, 1);
    wr::<i32>(this, 8, size);
    wr::<i32>(this, 0xC, -1);
    let mut i = 0;
    while i < size {
        let f = rd::<GPtr<u8>>(this, 4).byte_add(i as isize);
        f.write(f.read() | 0x80);
        let f = rd::<GPtr<u8>>(this, 4).byte_add(i as isize);
        f.write(f.read() & 0x80);
        i += 1;
    }
    this
}

/// `CPool<CustomEnvMapPipeMaterialData>::CPool(int size, const char*)` (0x5D9060): 12-byte objects;
/// see [`env_map_pool_ctor`].
#[hook(addr = 0x5D9060, conv = thiscall, name = "FUN_005d9060", expect = "56 57 8B 7C 24 0C 8D 04 7F",
       unverified = "mvpa-render-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "boot-time pool constructor (operator new twice)")]
pub fn env_map_material_pool_ctor(this: GPtr<u8>, size: i32, _name: u32) -> GPtr<u8> {
    env_map_pool_ctor(this, size, 12)
}

/// `CPool<CustomEnvMapPipeAtomicData>::CPool(int size, const char*)` (0x5D91B0): 12-byte objects;
/// see [`env_map_pool_ctor`].
#[hook(addr = 0x5D91B0, conv = thiscall, name = "FUN_005d91b0", expect = "56 57 8B 7C 24 0C 8D 04 7F",
       unverified = "mvpa-render-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "boot-time pool constructor (operator new twice)")]
pub fn env_map_atomic_pool_ctor(this: GPtr<u8>, size: i32, _name: u32) -> GPtr<u8> {
    env_map_pool_ctor(this, size, 12)
}

/// `CPool<CustomSpecMapPipeMaterialData>::CPool(int size, const char*)` (0x5D9300): 8-byte objects;
/// see [`env_map_pool_ctor`].
#[hook(addr = 0x5D9300, conv = thiscall, name = "FUN_005d9300", expect = "56 57 8B 7C 24 0C 8D 04 FD 00 00 00 00",
       unverified = "mvpa-render-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "boot-time pool constructor (operator new twice)")]
pub fn spec_map_material_pool_ctor(this: GPtr<u8>, size: i32, _name: u32) -> GPtr<u8> {
    env_map_pool_ctor(this, size, 8)
}

/// `CPlantSurfPropMgr::Initialise()` (0x5DD6C0, cdecl): `m_countSurfPropsAllocated (0xC39ED4) = 0`,
/// the 0xB2 entries of `m_SurfPropPtrTab` (0xC38070) zeroed, then each of the 57 `CPlantSurfProp`
/// (0x7C bytes at 0xC38338): +0 word 0 and three 0x28-byte plant records from +4 (+0 word 0xFFFF,
/// +2 word 0, +4 colour = `CRGBA(255,255,255,255)` built in a stack temporary by 0x7170C0, +8 byte 0xFF,
/// +9 byte 0, +0xC/+0x10 1.0f, +0x14/+0x18/+0x20/+0x24/+0x1C 0); finally
/// `LoadPlantsDat("PLANTS.DAT") != 0` (AL; EAX upper bits from the callee, the only caller tests AL).
#[hook(addr = 0x5DD6C0, conv = cdecl, name = "CPlantSurfPropMgr::Initialise", expect = "83 EC 08 53 55 56 57",
       unverified = "mvpa-render-1", route = "split", hazards = "none", recs = "grove-walk",
       note = "loads PLANTS.DAT (file effects)")]
pub fn plant_surf_prop_mgr_initialise() -> u8 {
    g::<u32>(0xC39ED4).write(0);
    let mut i = 0;
    while i < 0xB2 {
        g::<u32>(0xC38070 + i * 4).write(0);
        i += 1;
    }
    let mut e = 0xC38338usize;
    while e < 0xC39ED4 {
        let base = GPtr::<u8>::from_addr(e);
        wr::<u16>(base, 0, 0);
        let mut p = base.byte_add(6);
        for _ in 0..3 {
            let mut tmp = [0u8; 4];
            wr::<u16>(p, -2, 0xFFFF);
            wr::<u16>(p, 0, 0);
            let c = crgba_ctor(GPtr::from_raw(tmp.as_mut_ptr()), 0xFF, 0xFF, 0xFF, 0xFF);
            wr::<u32>(p, 2, rd::<u32>(c, 0));
            wr::<u8>(p, 6, 0xFF);
            wr::<u8>(p, 7, 0);
            wr::<u32>(p, 0xA, 0x3F80_0000);
            wr::<u32>(p, 0xE, 0x3F80_0000);
            wr::<u32>(p, 0x12, 0);
            wr::<u32>(p, 0x16, 0);
            wr::<u32>(p, 0x1E, 0);
            wr::<u32>(p, 0x22, 0);
            wr::<u32>(p, 0x1A, 0);
            p = p.byte_add(0x28);
        }
        e += 0x7C;
    }
    u8::from(load_plants_dat(GPtr::from_addr(0x86BF88)) != 0)
}

/// `CPlantSurfPropMgr::GetSurfProperties(uint16 id)` (0x6F9DE0, cdecl): `m_SurfPropPtrTab[id]` (0xC38070)
/// for `id < 0xB2`, else NULL. The entry jumps through the protection chunk 0x401A13 (loads AX).
#[hook(addr = 0x6F9DE0, conv = cdecl, name = "CPlantSurfPropMgr::GetSurfProperties", expect = "E9 2E 7C D0 FF 66 3D B2 00",
       unverified = "mvpa-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn plant_surf_prop_mgr_get_surf_properties(id: u32) -> GPtr<u8> {
    let id = id as u16;
    if id >= 0xB2 {
        return GPtr::NULL;
    }
    g::<GPtr<u8>>(0xC38070 + usize::from(id) * 4).read()
}

/// `CCustomRoadsignMgr::Initialise()` (0x6FE120, cdecl): slot = `FindTxdSlot("particle")`, push, set;
/// `pCharsetTex (0xC3EF84) = RwTextureRead("roadsignfont", NULL)` with filter byte +0x50 = 1 and
/// addressing byte +0x51 = 0x33; pop; `pCharsetLockedRaster (0xC3EF88) = RwRasterLock(tex->raster, 0,
/// 2)`; returns AL = 1 (upper EAX from RwRasterLock; the only caller tests AL).
#[hook(addr = 0x6FE120, conv = cdecl, name = "CCustomRoadsignMgr::Initialise", expect = "56 68 48 C6 85 00",
       unverified = "mvpa-render-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "reads a texture and locks its raster (one-time initialiser)")]
pub fn custom_roadsign_mgr_initialise() -> u8 {
    let slot = find_txd_slot(GPtr::from_addr(PARTICLE_TXD));
    push_current_txd();
    set_current_txd(slot);
    let tex = rw_texture_read(GPtr::from_addr(0x872590), GPtr::NULL);
    wr::<u8>(tex, 0x50, 1);
    wr::<u8>(tex, 0x51, 0x33);
    g::<GPtr<u8>>(0xC3EF84).write(tex);
    pop_current_txd();
    let tex = g::<GPtr<u8>>(0xC3EF84).read();
    let p = rw_raster_lock(rd::<GPtr<u8>>(tex, 0), 0, 2);
    g::<GPtr<u8>>(0xC3EF88).write(p);
    1
}

/// `CBirds::Init()` (0x711EC0): `aBirds[i].m_bCreated (+0x3F of each 0x44-byte CBird at 0xC6A8B0) = 0`
/// for the 6 birds, `uiNumberOfBirds (0xC6A8A4) = 0`, `bHasBirdBeenShot (0xC6A8A0) = 0`.
#[hook(addr = 0x711EC0, conv = cdecl, name = "CBirds::Init", expect = "B8 EF A8 C6 00 33 C9",
       unverified = "mvpa-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn birds_init() {
    let mut a = 0xC6A8EFusize;
    while a < 0xC6AA87 {
        g::<u8>(a).write(0);
        a += 0x44;
    }
    g::<u32>(0xC6A8A4).write(0);
    g::<u8>(0xC6A8A0).write(0);
}

/// `CMovingThings::Init()` (0x717D00): zeroes the first 0x40 bytes of each of the three 0x100-byte
/// blocks at 0xC71468 (the moving-things lists), then `CPointLights::Init()` and (tail call)
/// `CEscalators::Init()`.
#[hook(addr = 0x717D00, conv = cdecl, name = "CMovingThings::Init", expect = "BA 68 14 C7 00 57",
       unverified = "mvpa-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn moving_things_init() {
    let mut a = 0xC71468usize;
    loop {
        let next = a + 0x100;
        let mut i = 0;
        while i < 0x10 {
            g::<u32>(a + i * 4).write(0);
            i += 1;
        }
        a = next;
        if a >= 0xC71768 {
            break;
        }
    }
    point_lights_init();
    escalators_init();
}

/// `CCheckpoints::Init()` (0x722880): the 32 `CCheckpoint`s (0x38 bytes at 0xC7F158): +0 word 0x101
/// (type 0x101), +2 byte 0, +3 byte 1, +4 identifier 0, colour (+8..+0xB) 0xFF each, +0xC word 0x400,
/// +0xE word 5, +0x10..+0x27 zero, +0x28 0.25f, +0x2C 1.0f, +0x30/+0x34 0; then `NumActiveCPts
/// (0xC7C6D4) = 0`. Stores in the original order.
#[hook(addr = 0x722880, conv = cdecl, name = "CCheckpoints::Init", expect = "B8 5A F1 C7 00 33 C9",
       unverified = "mvpa-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn checkpoints_init() {
    let mut a = 0xC7F15Ausize;
    while a < 0xC7F85A {
        let p = GPtr::<u8>::from_addr(a);
        wr::<u16>(p, -2, 0x101);
        wr::<u8>(p, 0, 0);
        wr::<u32>(p, 2, 0);
        wr::<u8>(p, 6, 0xFF);
        wr::<u8>(p, 7, 0xFF);
        wr::<u8>(p, 8, 0xFF);
        wr::<u8>(p, 9, 0xFF);
        wr::<u16>(p, 0xA, 0x400);
        wr::<u16>(p, 0xC, 5);
        for o in [0xE, 0x12, 0x16, 0x1A, 0x1E, 0x22] {
            wr::<u32>(p, o, 0);
        }
        wr::<u32>(p, 0x26, 0x3E80_0000);
        wr::<u32>(p, 0x2A, 0x3F80_0000);
        wr::<u32>(p, 0x2E, 0);
        wr::<u32>(p, 0x32, 0);
        wr::<u8>(p, 1, 1);
        a += 0x38;
    }
    g::<u32>(0xC7C6D4).write(0);
}

/// `CRopes::Init()` (0x555DC0): `aRopes[i].m_bExists (+0x325 of each 0x328-byte CRope at 0xB768B8) = 0`
/// for the 8 ropes, `PlayerControlsCrane (0xB76898) = 0`.
#[hook(addr = 0x555DC0, conv = cdecl, name = "CRopes::Init", expect = "B8 DD 6B B7 00",
       unverified = "mvpa-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn ropes_init() {
    let mut a = 0xB76BDDusize;
    while a < 0xB7851D {
        g::<u8>(a).write(0);
        a += 0x328;
    }
    g::<u32>(0xB76898).write(0);
}

/// `CRope::Remove()` (0x556780, thiscall): clears `m_bExists (+0x325)`; with an attached object
/// (+0x318) its flags +0x40 lose bits 25 and 31, +0x318 = NULL, the rope's holder (+0x314) gets flag bit
/// 0 of +0x1C and `m_nFlagsCounter? (+0x326) = 60`; then a holder (+0x314) is removed from the world,
/// deleted through its vtable (scalar deleting destructor, flag 1) and +0x314 = NULL.
#[hook(addr = 0x556780, conv = thiscall, name = "CRope::Remove", expect = "56 8B F1 8B 86 18 03 00 00",
       unverified = "mvpa-render-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "deletes the rope's entity (heap)")]
pub fn rope_remove(this: GPtr<u8>) {
    let att = rd::<GPtr<u8>>(this, 0x318);
    wr::<u8>(this, 0x325, 0);
    if !att.is_null() {
        wr::<u32>(att, 0x40, rd::<u32>(att, 0x40) & 0xFDFF_FFFF);
        let att = rd::<GPtr<u8>>(this, 0x318);
        wr::<u32>(att, 0x40, rd::<u32>(att, 0x40) & 0x7FFF_FFFF);
        let holder = rd::<GPtr<u8>>(this, 0x314);
        wr::<u32>(this, 0x318, 0);
        wr::<u32>(holder, 0x1C, rd::<u32>(holder, 0x1C) | 1);
        wr::<u8>(this, 0x326, 0x3C);
    }
    let holder = rd::<GPtr<u8>>(this, 0x314);
    if !holder.is_null() {
        world_remove(holder);
        let h = rd::<GPtr<u8>>(this, 0x314);
        if !h.is_null() {
            entity_delete(h, 1);
        }
        wr::<u32>(this, 0x314, 0);
    }
}

/// `CRopes::Shutdown()` (0x556B10): `CRope::Remove()` for each of the 8 ropes (0xB768B8, stride 0x328)
/// whose `m_bExists (+0x325)` is set. The entry runs through the protection chunk 0x409AA0 (ESI =
/// 0xB768B8).
#[hook(addr = 0x556B10, conv = cdecl, name = "CRopes::Shutdown", expect = "56 E9 8A 2F EB FF",
       unverified = "mvpa-render-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "CRope::Remove deletes rope entities")]
pub fn ropes_shutdown() {
    let mut a = 0xB768B8usize;
    while a < 0xB781F8 {
        let r = GPtr::<u8>::from_addr(a);
        if rd::<u8>(r, 0x325) != 0 {
            rope_remove_call(r);
        }
        a += 0x328;
    }
}

/// `CClouds::Init()` (0x7138D0): with the "particle" TXD current (push, find, set): `gpCloudTex
/// (0xC6AA78) = RwTextureRead("cloudmasked", NULL)` (names at 0x872874.. in the exe), `gpCloudMaskTex
/// (0xC6AA7C)`, `gpMoonMask (0xC6AA74)` (with mask), `ms_vc.texture (0xC6BFC8)` (with mask); pop;
/// `CloudRotation = 0`; `VolumetricCloudsInit()`; (tail call) `MovingFogInit()`.
#[hook(addr = 0x7138D0, conv = cdecl, name = "CClouds::Init", expect = "E8 CB DD 01 00",
       unverified = "mvpa-render-1", route = "split", hazards = "none", recs = "grove-walk",
       note = "reads four textures (one-time initialiser)")]
pub fn clouds_init() {
    push_current_txd();
    set_current_txd(find_txd_slot(GPtr::from_addr(PARTICLE_TXD)));
    let t = rw_texture_read(GPtr::from_addr(0x872874), GPtr::NULL);
    g::<GPtr<u8>>(0xC6AA78).write(t);
    let t = rw_texture_read(GPtr::from_addr(0x872868), GPtr::NULL);
    g::<GPtr<u8>>(0xC6AA7C).write(t);
    let t = rw_texture_read(GPtr::from_addr(0x872858), GPtr::from_addr(0x872860));
    g::<GPtr<u8>>(0xC6AA74).write(t);
    let t = rw_texture_read(GPtr::from_addr(0x872840), GPtr::from_addr(0x87284C));
    g::<GPtr<u8>>(0xC6BFC8).write(t);
    pop_current_txd();
    g::<u32>(0xC6AA70).write(0);
    volumetric_clouds_init();
    moving_fog_init();
}

/// The 32 skidmarks (0x158 bytes at 0xC79BFE): `m_nState (+0) = 0`, `m_bActive? (+1) = 0`.
fn skidmarks_clear_all() {
    let mut a = 0xC79BFFusize;
    while a < 0xC7C6FF {
        g::<u8>(a - 1).write(0);
        g::<u8>(a).write(0);
        a += 0x158;
    }
}

/// `CSkidmarks::Init()` (0x7204E0): push TXD, set "particle", `m_pTexture (0xC79A88) =
/// RwTextureRead("particleskid", NULL)`, pop; clear the 32 skidmarks; the 16 strip index sextets at
/// 0xC799C8 (u16: 2i, 2i+2, 2i+1, 2i+1, 2i+2, 2i+3).
#[hook(addr = 0x7204E0, conv = cdecl, name = "CSkidmarks::Init", expect = "E8 BB 11 01 00",
       unverified = "mvpa-render-1", route = "split", hazards = "none", recs = "grove-walk",
       note = "reads a texture (one-time initialiser)")]
pub fn skidmarks_init() {
    push_current_txd();
    set_current_txd(find_txd_slot(GPtr::from_addr(PARTICLE_TXD)));
    let t = rw_texture_read(GPtr::from_addr(0x87299C), GPtr::NULL);
    g::<GPtr<u8>>(0xC79A88).write(t);
    pop_current_txd();
    skidmarks_clear_all();
    let mut a = 0xC799CAusize;
    let mut i: u32 = 0;
    while a < 0xC79A8A {
        let c = i * 2;
        g::<u16>(a - 2).write(c as u16);
        g::<u16>(a).write((c + 2) as u16);
        g::<u16>(a + 2).write((c + 1) as u16);
        g::<u16>(a + 4).write((c + 1) as u16);
        g::<u16>(a + 6).write((c + 2) as u16);
        g::<u16>(a + 8).write((c + 3) as u16);
        a += 0xC;
        i += 1;
    }
}

/// `CSkidmarks::Clear()` (0x720590): bytes +0 and +1 of each of the 32 skidmarks cleared.
#[hook(addr = 0x720590, conv = cdecl, name = "CSkidmarks::Clear", expect = "B8 FF 9B C7 00 32 C9",
       unverified = "mvpa-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn skidmarks_clear() {
    skidmarks_clear_all();
}

/// `CSpecialFX::Shutdown()` (0x723390): `C3dMarkers::Shutdown()`; then the special-FX texture (0xC7C718,
/// `RwTextureDestroy`) and the two rasters (0xC7C71C, 0xC7C720, `RwRasterDestroy`) are destroyed when
/// set and cleared; 0xC7C724 and 0xC7C618 = 0.
#[hook(addr = 0x723390, conv = cdecl, name = "CSpecialFX::Shutdown", expect = "56 E8 7A F3 FF FF",
       unverified = "mvpa-render-1", route = "split", hazards = "none", recs = "grove-walk",
       note = "destroys a texture and two rasters (one-time)")]
pub fn special_fx_shutdown() {
    markers_shutdown();
    let t = g::<GPtr<u8>>(0xC7C718).read();
    if !t.is_null() {
        rw_texture_destroy(t);
        g::<u32>(0xC7C718).write(0);
    }
    for a in [0xC7C71Cusize, 0xC7C720] {
        let r = g::<GPtr<u8>>(a).read();
        if !r.is_null() {
            rw_raster_destroy(r);
            g::<u32>(a).write(0);
        }
    }
    g::<u32>(0xC7C724).write(0);
    g::<u32>(0xC7C618).write(0);
}

/// `CMovingThings::Shutdown()` (0x717C20): for the 32 escalators (0x150 bytes at 0xC6E9A8):
/// `SwitchOff()` and `m_bExist (+0x78) = 0`.
#[hook(addr = 0x717C20, conv = cdecl, name = "CMovingThings::Shutdown", expect = "56 BE 20 EA C6 00",
       unverified = "mvpa-render-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "SwitchOff removes the escalator step objects")]
pub fn moving_things_shutdown() {
    let mut a = 0xC6EA20usize;
    while a < 0xC71420 {
        escalator_switch_off(GPtr::from_addr(a - 0x78));
        g::<u8>(a).write(0);
        a += 0x150;
    }
}

/// `ProcObjectMan_c::Init()` (0x5A3EA0, thiscall): the 0x200 `ProcObjectListItem_c` (0x14 bytes at
/// this+0x2408) are added to the unused list (this+0x4C08); `m_numSurfaceInfos (+4) = 0`;
/// `LoadDataFile()`; `+0 = 0`; returns AL = 1.
#[hook(addr = 0x5A3EA0, conv = thiscall, name = "ProcObjectMan_c::Init", expect = "53 55 56 57 8B F9",
       unverified = "mvpa-render-1", route = "split", hazards = "none", recs = "grove-walk",
       note = "loads procobj.dat (file effects)")]
pub fn proc_object_man_init(this: GPtr<u8>) -> u8 {
    let list = this.byte_add(0x4C08);
    let mut item = this.byte_add(0x2408);
    for _ in 0..0x200 {
        list_add_item(list, item);
        item = item.byte_add(0x14);
    }
    wr::<u32>(this, 4, 0);
    proc_object_man_load_data_file(this);
    wr::<u32>(this, 0, 0);
    1
}

/// `ProcObjectMan_c::Exit()` (0x5A3EE0, thiscall): `ProcSurfaceInfo_c::Exit()` for each of the
/// `m_numSurfaceInfos (+4, re-read per iteration, signed)` infos (0x48 bytes at +8), then (tail call)
/// `List_c::RemoveAll()` of the unused list (+0x4C08).
#[hook(addr = 0x5A3EE0, conv = thiscall, name = "ProcObjectMan_c::Exit", expect = "53 8B D9 8B 43 04",
       unverified = "mvpa-render-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "deletes procedural objects")]
pub fn proc_object_man_exit(this: GPtr<u8>) {
    let mut i = 0;
    let mut info = this.byte_add(8);
    while i < rd::<i32>(this, 4) {
        proc_surface_info_exit_call(info);
        i += 1;
        info = info.byte_add(0x48);
    }
    list_remove_all(this.byte_add(0x4C08));
}

/// `ProcSurfaceInfo_c::Init(surfaceName, modelName, spacing, minDist, minRot, maxRot, minScl, maxScl,
/// minSclZ, maxSclZ, zOffsetMin, zOffsetMax, u8 align, u8 useGrid)` (0x5A2EB0, thiscall, `ret 0x38`):
/// +0 byte = `g_surfaceInfos.GetSurfaceIdFromName(surfaceName)`; `CModelInfo::GetModelInfo(modelName,
/// this+4)`; NULL → AL = 0 (EAX 0). Else minDist is clamped to 80.0 when greater (x87 `fcomp`, unordered
/// keeps it); +8 spacing, +0xC `1.0f / (spacing * spacing)`, +0x10 `minDist * minDist`, +0x14/+0x18
/// `(float)minRot/maxRot * (pi/180)`, +0x1C..+0x30 the six floats as dwords, +0x34 align, +0x38
/// `(float)useGrid`; AL = 1 (upper EAX = zOffsetMax's, the caller ignores the result).
#[hook(addr = 0x5A2EB0, conv = thiscall, name = "ProcSurfaceInfo_c::Init", expect = "8B 44 24 04 56 8B F1",
       unverified = "mvpa-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
#[allow(clippy::too_many_arguments)]
pub fn proc_surface_info_init(
    this: GPtr<u8>,
    surface: GPtr<u8>,
    model: GPtr<u8>,
    spacing: u32,
    min_dist: u32,
    min_rot: i32,
    max_rot: i32,
    a6: u32,
    a7: u32,
    a8: u32,
    a9: u32,
    a10: u32,
    a11: u32,
    align: u32,
    use_grid: u32,
) -> u8 {
    let id = get_surface_id_from_name(GPtr::from_addr(0xB79538), surface);
    wr::<u8>(this, 0, id as u8);
    if get_model_info_full(model, this.byte_add(4)).is_null() {
        return 0;
    }
    let spacing_bits = spacing;
    let spacing = f32::from_bits(spacing);
    let mut min_dist = f32::from_bits(min_dist);
    let mut f = Fpu::new();
    f.fld(min_dist);
    f.fcomp_m(k(0x42A0_0000));
    if !f.jne(0x41) {
        min_dist = k(0x42A0_0000);
    }
    f.fld(spacing);
    f.fmul_m(spacing);
    wr::<u32>(this, 0x20, a7);
    wr::<u32>(this, 0x1C, a6);
    wr::<u32>(this, 8, spacing_bits);
    f.fdivr_m(k(0x3F80_0000));
    wr::<u32>(this, 0x2C, a10);
    wr::<u32>(this, 0x28, a9);
    wr::<u32>(this, 0x24, a8);
    wr::<u32>(this, 0x30, a11);
    wr::<u8>(this, 0x34, align as u8);
    wr::<f32>(this, 0xC, f.fstp());
    f.fld(min_dist);
    f.fmul_m(min_dist);
    wr::<f32>(this, 0x10, f.fstp());
    f.fild(min_rot);
    f.fmul_m(k(0x3C8E_FA35));
    wr::<f32>(this, 0x14, f.fstp());
    f.fild(max_rot);
    f.fmul_m(k(0x3C8E_FA35));
    wr::<f32>(this, 0x18, f.fstp());
    f.fild(i32::from(use_grid as u8));
    wr::<f32>(this, 0x38, f.fstp());
    1
}

/// `ProcSurfaceInfo_c::Exit()` (0x5A3270, thiscall): for every item of the object list (+0x3C, next at
/// item+4 read first): removed from it and added to `g_procObjMan`'s unused list (0xBBC8B8); its object
/// (+8) gets virtual slot +0x20, `CWorld::Remove`, is deleted (vtable slot 0, flag 1, when non-NULL)
/// and +8 = NULL.
#[hook(addr = 0x5A3270, conv = thiscall, name = "ProcSurfaceInfo_c::Exit", expect = "53 8D 59 3C 56",
       unverified = "mvpa-render-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "deletes procedural objects")]
pub fn proc_surface_info_exit(this: GPtr<u8>) {
    let list = this.byte_add(0x3C);
    let mut p = rd::<GPtr<u8>>(list, 0);
    while !p.is_null() {
        let next = rd::<GPtr<u8>>(p, 4);
        list_remove_item(list, p);
        list_add_item(GPtr::from_addr(0xBBC8B8), p);
        entity_vslot_20(rd::<GPtr<u8>>(p, 8));
        world_remove(rd::<GPtr<u8>>(p, 8));
        let obj = rd::<GPtr<u8>>(p, 8);
        if !obj.is_null() {
            entity_delete(obj, 1);
        }
        wr::<u32>(p, 8, 0);
        p = next;
    }
}

/// `FxManager_c::DestroyAllFxSystems()` (0x4A98B0, thiscall): `DestroyFxSystem` for every system of the
/// active list (head +0xC, next at +4 read before the call).
#[hook(addr = 0x4A98B0, conv = thiscall, name = "FxManager_c::DestroyAllFxSystems", expect = "57 8B F9 8B 47 0C",
       unverified = "mvpa-render-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "frees FX systems")]
pub fn fx_manager_destroy_all_fx_systems(this: GPtr<u8>) {
    let mut p = rd::<GPtr<u8>>(this, 0xC);
    while !p.is_null() {
        let next = rd::<GPtr<u8>>(p, 4);
        destroy_fx_system(this, p);
        p = next;
    }
}

/// `CRealTimeShadowManager::Exit()` (0x706A60, thiscall): when initialised (+0 byte): each of the 16
/// shadows (+4) is destroyed (`~CRealTimeShadow` + `free`) when set and its slot cleared; the two shadow
/// cameras (+0x44, +0x4C) `Destroy()`; +0 = 0.
#[hook(addr = 0x706A60, conv = thiscall, name = "CRealTimeShadowManager::Exit", expect = "55 8B E9 80 7D 00 00",
       unverified = "mvpa-render-1", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "frees the shadows and destroys rasters (one-time)")]
pub fn real_time_shadow_manager_exit(this: GPtr<u8>) {
    if rd::<u8>(this, 0) == 0 {
        return;
    }
    let mut i = 0;
    while i < 16 {
        let p = rd::<GPtr<u8>>(this, 4 + i * 4);
        if !p.is_null() {
            real_time_shadow_destructor(p);
            crt_free(p);
        }
        wr::<u32>(this, 4 + i * 4, 0);
        i += 1;
    }
    shadow_camera_destroy(this.byte_add(0x44));
    shadow_camera_destroy(this.byte_add(0x4C));
    wr::<u8>(this, 0, 0);
}

/// `CClock::GetIsTimeInRange(u8 from, u8 to)` (0x52CEE0, cdecl): with `h = ms_nGameClockHours
/// (0xB70153)`: `from > to` → `h >= from || h < to`, else `h >= from && h < to` (unsigned bytes). The
/// entry runs through the protection chunk 0x4017E3 (loads CL). AL result; callers test AL.
#[hook(addr = 0x52CEE0, conv = cdecl, name = "CClock::GetIsTimeInRange", expect = "8A 44 24 04 8A 54 24 08",
       unverified = "mvpa-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn clock_get_is_time_in_range(from: u8, to: u8) -> u8 {
    let h = g::<u8>(0xB70153).read();
    let r = if from > to { h >= from || h < to } else { h >= from && h < to };
    u8::from(r)
}

/// `CClock::NormaliseGameClock()` (0x52CDE0): carries seconds (u16 0xB70150) ≥ 60 into minutes (u8
/// 0xB70152, wrapping add), minutes ≥ 60 into hours (0xB70153), hours ≥ 24 into days (0xB70154, wrapping
/// add); days > 31 → month (0xB70155) + 1, day 1; month > 12 → 1. The remainders are computed by
/// repeated subtraction as in the original (same values as `%`).
#[hook(addr = 0x52CDE0, conv = cdecl, name = "CClock::NormaliseGameClock", expect = "66 8B 0D 50 01 B7 00",
       unverified = "mvpa-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn normalise_game_clock() {
    let secs = g::<u16>(0xB70150).read();
    let mut mins = g::<u8>(0xB70152).read();
    if secs >= 60 {
        mins = mins.wrapping_add((secs / 60) as u8);
        g::<u8>(0xB70152).write(mins);
        g::<u16>(0xB70150).write(secs % 60);
    }
    let mut hours = g::<u8>(0xB70153).read();
    if mins >= 60 {
        hours = hours.wrapping_add(mins / 60);
        g::<u8>(0xB70153).write(hours);
        g::<u8>(0xB70152).write(mins % 60);
    }
    if hours >= 24 {
        let days = g::<u8>(0xB70154).read();
        g::<u8>(0xB70154).write(days.wrapping_add(hours / 24));
        g::<u8>(0xB70153).write(hours % 24);
    }
    if g::<u8>(0xB70154).read() > 0x1F {
        let m = g::<u8>(0xB70155).read();
        g::<u8>(0xB70155).write(m.wrapping_add(1));
        g::<u8>(0xB70154).write(1);
    }
    if g::<u8>(0xB70155).read() > 0xC {
        g::<u8>(0xB70155).write(1);
    }
}

/// `CClock::SetGameClock(u8 hours, u8 minutes, u8 day)` (0x52D150, cdecl): `ms_nLastClockTick
/// (0xB70158) = CTimer::m_snTimeInMilliseconds`, hours, minutes; `day != 0` → `CurrentDay (0xB7014E) =
/// day` and `ms_nGameClockDays (0xB70154)++`; seconds = 0; (tail call) `NormaliseGameClock()`.
#[hook(addr = 0x52D150, conv = cdecl, name = "CClock::SetGameClock", expect = "A1 84 CB B7 00",
       unverified = "mvpa-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn clock_set_game_clock(hours: u8, minutes: u8, day: u8) {
    g::<u32>(0xB70158).write(g::<u32>(0xB7CB84).read());
    g::<u8>(0xB70153).write(hours);
    g::<u8>(0xB70152).write(minutes);
    if day != 0 {
        g::<u8>(0xB7014E).write(day);
        let d = g::<u8>(0xB70154).read();
        g::<u8>(0xB70154).write(d.wrapping_add(1));
    }
    g::<u16>(0xB70150).write(0);
    normalise_game_clock_call();
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_SPRITE2D_DRAW_RECT,
    HOOK_SPRITE2D_ADD_TO_BUFFER,
    HOOK_D3D_RESOURCE_SYSTEM_INIT,
    HOOK_D3D_RESOURCE_SYSTEM_SET_USE_BUFFERING,
    HOOK_IS_ENVIRONMENT_MAPPING_SUPPORTED,
    HOOK_ENV_MAP_MATERIAL_POOL_CTOR,
    HOOK_ENV_MAP_ATOMIC_POOL_CTOR,
    HOOK_SPEC_MAP_MATERIAL_POOL_CTOR,
    HOOK_PLANT_SURF_PROP_MGR_INITIALISE,
    HOOK_PLANT_SURF_PROP_MGR_GET_SURF_PROPERTIES,
    HOOK_CUSTOM_ROADSIGN_MGR_INITIALISE,
    HOOK_BIRDS_INIT,
    HOOK_MOVING_THINGS_INIT,
    HOOK_CHECKPOINTS_INIT,
    HOOK_ROPES_INIT,
    HOOK_ROPE_REMOVE,
    HOOK_ROPES_SHUTDOWN,
    HOOK_CLOUDS_INIT,
    HOOK_SKIDMARKS_INIT,
    HOOK_SKIDMARKS_CLEAR,
    HOOK_SPECIAL_FX_SHUTDOWN,
    HOOK_MOVING_THINGS_SHUTDOWN,
    HOOK_PROC_OBJECT_MAN_INIT,
    HOOK_PROC_OBJECT_MAN_EXIT,
    HOOK_PROC_SURFACE_INFO_INIT,
    HOOK_PROC_SURFACE_INFO_EXIT,
    HOOK_FX_MANAGER_DESTROY_ALL_FX_SYSTEMS,
    HOOK_REAL_TIME_SHADOW_MANAGER_EXIT,
    HOOK_CLOCK_GET_IS_TIME_IN_RANGE,
    HOOK_NORMALISE_GAME_CLOCK,
    HOOK_CLOCK_SET_GAME_CLOCK,
];
