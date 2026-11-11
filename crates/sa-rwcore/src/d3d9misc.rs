//! PORT-RW-4 module `d3d9misc`: small D3D9 driver functions: dynamic-geometry list and world restore
//! callback, the default world-sector / atomic pipeline create / destroy, vertex-declaration element
//! sizes, D3D9 usage flags, the world-sector all-in-one init and lighting callback, vertex-shader cache
//! purge and constant count, `RwCameraShowRaster` and two node-definition getters (see
//! docs/arch/rw-port.md, PORT-RW-4). Ported from the 1.0 US bytes.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, p8, rd, rw_globals, rw_memory_alloc, rw_memory_free, wr};

const DYN_GEOMETRY: usize = 0xC92520;
const DYN_GEOMETRY_FL: usize = 0xC92524;
const SAVED_RESTORE_CB: usize = 0xC9251C;
/// World plugin module offset in RwGlobals (+0x54 default atomic pipeline, +0x58 sector pipeline).
const WORLD_MODULE_OFFSET: usize = 0xC9BC60;
/// Vertex-declaration element type sizes (u32 per D3DDECLTYPE).
const DECL_TYPE_SIZES: u32 = 0x874EF8;
const GEOMETRY_USAGE_OFFSET: usize = 0xC926E8;
const SECTOR_USAGE_OFFSET: usize = 0xC926EC;
/// Vertex-shader cache: count, entries of 0x14 from 0xC92700 (shader first), the key template.
const VS_COUNT: usize = 0xC94BF8;
const VS_ENTRIES: u32 = 0xC92700;
const VS_LIGHT_TABLE: u32 = 0xC94AF8;

orig! {
    /// `_rwFreeListCreate(entrySize, entriesPerBlock, alignment, hint)` (0x801980)
    pub fn free_list_create(size: u32, per_block: u32, align: u32, hint: u32) -> GPtr<u8> = cdecl 0x801980 => unported;
    /// The saved device restore callback, through its pointer
    pub fn call_restore_cb(f: u32) = cdecl dyn => unported;
    /// `RxPipelineCreate()` (0x8057B0)
    pub fn rx_pipeline_create() -> GPtr<u8> = cdecl 0x8057B0 => unported;
    /// `RxPipelineLock(pipeline)` (0x806990)
    pub fn rx_pipeline_lock(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x806990 => unported;
    /// `RxLockedPipeAddFragment(pipeline, firstIndex, nodeDef, NULL)` (0x806BE0, varargs)
    pub fn rx_locked_pipe_add_fragment(p: GPtr<u8>, first: u32, node: GPtr<u8>, end: u32) -> GPtr<u8> = cdecl 0x806BE0 => unported;
    /// `RxLockedPipeUnlock(pipeline)` (0x805F40)
    pub fn rx_locked_pipe_unlock(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x805F40 => unported;
    /// `_rxPipelineDestroy(pipeline)` (0x805820)
    pub fn rx_pipeline_destroy(p: GPtr<u8>) = cdecl 0x805820 => unported;
    /// `RxNodeDefinitionGetD3D9WorldSectorAllInOne()` (0x75E9F0)
    pub fn node_def_world_sector() -> GPtr<u8> = cdecl 0x75E9F0 => unported;
    /// `RxNodeDefinitionGetD3D9AtomicAllInOne()` (0x7582E0)
    pub fn node_def_atomic() -> GPtr<u8> = cdecl 0x7582E0 => unported;
    /// `RpWorldSetDefaultSectorPipeline(pipeline)` (0x75E270)
    pub fn set_default_sector_pipeline(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x75E270 => unported;
    /// `RpAtomicSetDefaultPipeline(pipeline)` (0x75E2A0)
    pub fn set_default_atomic_pipeline(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x75E2A0 => unported;
    /// `_rwD3D9LightsOpen()` (0x755D80)
    pub fn lights_open() -> u32 = cdecl 0x755D80 => unported;
    /// 0x760CF0 (vertex-shader module open, result unused)
    pub fn vs_open() -> u32 = cdecl 0x760CF0 => unported;
    /// `_rwD3D9LightsGlobalEnable(flags)` (0x756070)
    pub fn lights_global_enable(flags: u32) -> u32 = cdecl 0x756070 => unported;
    /// `_rwD3D9LightLocalEnable(light)` (0x756390)
    pub fn light_local_enable(l: GPtr<u8>) -> u32 = cdecl 0x756390 => unported;
    /// `_rwD3D9LightsEnable(enable, type)` (0x756600)
    pub fn lights_enable(enable: u32, ty: u32) = cdecl 0x756600 => unported;
    /// `RwD3D9DeleteVertexShader(vs)` (0x7FAC90)
    pub fn delete_vertex_shader(vs: GPtr<u8>) -> u32 = cdecl 0x7FAC90 => crate::d3d9dev::delete_vertex_shader;
    /// `_rpD3D9GetVertexShader(desc, callbacks)` (0x75EED0)
    pub fn get_vertex_shader(desc: u32, cbs: u32) -> u32 = cdecl 0x75EED0 => unported;
    /// `RwRasterShowRaster(raster, dev, flags)` (0x7FB1A0)
    pub fn raster_show_raster(r: GPtr<u8>, dev: u32, flags: u32) -> u32 = cdecl 0x7FB1A0 => unported;
}

#[inline]
fn gu(addr: usize) -> u32 {
    g::<u32>(addr).read()
}

#[inline]
fn su(addr: usize, v: u32) {
    g::<u32>(addr).write(v)
}

fn world_module() -> GPtr<u8> {
    rw_globals().byte_add(gu(WORLD_MODULE_OFFSET) as i32 as isize)
}

/// `_rpD3D9AddDynamicGeometry` (0x74E970): adds a `{geometry, next}` node (freelist created on first use,
/// 8 x 128, hint 0x3050D) at the list head unless the geometry is listed already.
#[hook(addr = 0x74E970, conv = cdecl, name = "_rpD3D9AddDynamicGeometry", expect = "A1 20 25 C9 00 56")]
pub fn add_dynamic_geometry(geometry: u32) {
    let mut n = p8(gu(DYN_GEOMETRY));
    while !n.is_null() {
        if rd::<u32>(n, 0) == geometry {
            return;
        }
        n = p8(rd::<u32>(n, 4));
    }
    if gu(DYN_GEOMETRY_FL) == 0 {
        su(DYN_GEOMETRY_FL, free_list_create(8, 0x80, 4, 0x3_050D).addr() as u32);
    }
    let node = rw_memory_alloc(p8(gu(DYN_GEOMETRY_FL)), 0x3_050D);
    wr::<u32>(node, 0, geometry);
    wr::<u32>(node, 4, gu(DYN_GEOMETRY));
    su(DYN_GEOMETRY, node.addr() as u32);
}

/// `_rpD3D9RemoveDynamicGeometry` (0x74E9E0): unlinks and frees the geometry's node (its geometry field
/// cleared first).
#[hook(addr = 0x74E9E0, conv = cdecl, name = "_rpD3D9RemoveDynamicGeometry", expect = "8B 15 20 25 C9 00")]
pub fn remove_dynamic_geometry(geometry: u32) {
    let head = gu(DYN_GEOMETRY);
    let mut prev = GPtr::<u8>::NULL;
    let mut n = p8(head);
    while !n.is_null() {
        if rd::<u32>(n, 0) == geometry {
            if n.addr() as u32 == head {
                su(DYN_GEOMETRY, rd::<u32>(n, 4));
            } else {
                wr::<u32>(prev, 4, rd::<u32>(n, 4));
            }
            wr::<u32>(n, 0, 0);
            rw_memory_free(p8(gu(DYN_GEOMETRY_FL)), n);
            return;
        }
        prev = n;
        n = p8(rd::<u32>(n, 4));
    }
}

/// `_rpD3D9WorldRestoreCallback` (0x74EA80): chains to the saved restore callback, then marks every
/// dynamic geometry fully locked (`lockedSinceLastInst = 0xFFF`).
#[hook(addr = 0x74EA80, conv = cdecl, name = "_rpD3D9WorldRestoreCallback", expect = "A1 1C 25 C9 00 85 C0")]
pub fn world_restore_callback() {
    let cb = gu(SAVED_RESTORE_CB);
    if cb != 0 {
        call_restore_cb(cb);
    }
    let mut n = p8(gu(DYN_GEOMETRY));
    while !n.is_null() {
        wr::<u16>(p8(rd::<u32>(n, 0)), 0xC, 0xFFF);
        n = p8(rd::<u32>(n, 4));
    }
}

/// A pipeline (`+0x2C` = 2) with one node; NULL (pipeline destroyed) on failure.
fn one_node_pipeline(node: fn() -> GPtr<u8>) -> Option<GPtr<u8>> {
    let p = rx_pipeline_create();
    if p.is_null() {
        return None;
    }
    wr::<u32>(p, 0x2C, 2);
    let lp = rx_pipeline_lock(p);
    if !lp.is_null() {
        let def = node();
        let f = rx_locked_pipe_add_fragment(lp, 0, def, 0);
        if !rx_locked_pipe_unlock(f).is_null() {
            return Some(p);
        }
    }
    rx_pipeline_destroy(p);
    None
}

/// `_rpCreateD3D9WorldSectorAllInOnePipe` (0x74EAD0): the default sector pipeline (stored at world
/// module +0x58 and set as default); 1, or 0 on failure.
#[hook(addr = 0x74EAD0, conv = cdecl, name = "_rpCreateD3D9WorldSectorAllInOnePipe", expect = "56 57 E8 D9 6C 0B 00")]
pub fn create_world_sector_pipe() -> u32 {
    match one_node_pipeline(node_def_world_sector) {
        Some(p) => {
            wr::<u32>(world_module(), 0x58, p.addr() as u32);
            set_default_sector_pipeline(p);
            1
        }
        None => 0,
    }
}

/// `_rpDestroyD3D9WorldSectorPipe` (0x74EB40).
#[hook(addr = 0x74EB40, conv = cdecl, name = "_rpDestroyD3D9WorldSectorPipe", expect = "6A 00 E8 29 F7 00 00")]
pub fn destroy_world_sector_pipe() {
    set_default_sector_pipeline(GPtr::NULL);
    let p = rd::<u32>(world_module(), 0x58);
    if p != 0 {
        rx_pipeline_destroy(p8(p));
        wr::<u32>(world_module(), 0x58, 0);
    }
}

/// `_rpCreateD3D9AtomicAllInOnePipe` (0x74EB80): the default atomic pipeline (world module +0x54, set as
/// default), then `_rwD3D9LightsOpen` (failure destroys the pipeline, leaving the stored pointer) and
/// the vertex-shader open.
#[hook(addr = 0x74EB80, conv = cdecl, name = "_rpCreateD3D9AtomicAllInOnePipe", expect = "56 57 E8 29 6C 0B 00")]
pub fn create_atomic_pipe() -> u32 {
    let Some(p) = one_node_pipeline(node_def_atomic) else {
        return 0;
    };
    wr::<u32>(world_module(), 0x54, p.addr() as u32);
    set_default_atomic_pipeline(p);
    if lights_open() == 0 {
        rx_pipeline_destroy(p);
        return 0;
    }
    vs_open();
    1
}

/// `_rpD3D9VertexDeclarationGetSize` (0x7522E0): the byte size of a D3DDECLTYPE.
#[hook(addr = 0x7522E0, conv = cdecl, name = "_rpD3D9VertexDeclarationGetSize", expect = "8B 44 24 04 8B 04 85 F8 4E 87 00")]
pub fn vertex_declaration_get_size(ty: u32) -> u32 {
    g::<u32>(DECL_TYPE_SIZES.wrapping_add(ty.wrapping_mul(4)) as usize).read()
}

/// `_rpD3D9VertexDeclarationGetStride` (0x7522F0): sum of the element sizes up to D3DDECL_END (type 0x11).
#[hook(addr = 0x7522F0, conv = cdecl, name = "_rpD3D9VertexDeclarationGetStride", expect = "8B 4C 24 04 33 C0")]
pub fn vertex_declaration_get_stride(elements: GPtr<u8>) -> u32 {
    let mut s = 0u32;
    let mut i = 0;
    loop {
        let t = rd::<u8>(elements, 4 + i * 8);
        if t >= 0x11 {
            return s;
        }
        s = s.wrapping_add(vertex_declaration_get_size(t as u32));
        i += 1;
    }
}

/// `RpD3D9GeometrySetUsageFlags` (0x7588B0).
#[hook(addr = 0x7588B0, conv = cdecl, name = "RpD3D9GeometrySetUsageFlags", expect = "8B 4C 24 04 8B 15 E8 26 C9 00")]
pub fn geometry_set_usage_flags(geometry: GPtr<u8>, flags: u32) {
    wr::<u32>(geometry, gu(GEOMETRY_USAGE_OFFSET) as i32 as isize, flags);
}

/// `RpD3D9GeometryGetUsageFlags` (0x7588D0).
#[hook(addr = 0x7588D0, conv = cdecl, name = "RpD3D9GeometryGetUsageFlags", expect = "8B 44 24 04 8B 0D E8 26 C9 00")]
pub fn geometry_get_usage_flags(geometry: GPtr<u8>) -> u32 {
    rd::<u32>(geometry, gu(GEOMETRY_USAGE_OFFSET) as i32 as isize)
}

/// `RpD3D9WorldSectorSetUsageFlags` (0x7588E0).
#[hook(addr = 0x7588E0, conv = cdecl, name = "RpD3D9WorldSectorSetUsageFlags", expect = "8B 4C 24 04 8B 15 EC 26 C9 00")]
pub fn world_sector_set_usage_flags(sector: GPtr<u8>, flags: u32) {
    wr::<u32>(sector, gu(SECTOR_USAGE_OFFSET) as i32 as isize, flags);
}

/// `RpD3D9WorldSectorGetUsageFlags` (0x758900).
#[hook(addr = 0x758900, conv = cdecl, name = "RpD3D9WorldSectorGetUsageFlags", expect = "8B 44 24 04 8B 0D EC 26 C9 00")]
pub fn world_sector_get_usage_flags(sector: GPtr<u8>) -> u32 {
    rd::<u32>(sector, gu(SECTOR_USAGE_OFFSET) as i32 as isize)
}

/// `_rwD3D9WorldSectorAllInOnePipelineInit` (0x75E540): instance 0x75E5E0, no reinstance, lighting
/// 0x75E570, render 0x756DF0; returns 1.
#[hook(addr = 0x75E540, conv = cdecl, name = "_rwD3D9WorldSectorAllInOnePipelineInit", expect = "8B 44 24 04 8B 40 14")]
pub fn world_sector_pipeline_init(node: GPtr<u8>) -> u32 {
    let c = p8(rd::<u32>(node, 0x14));
    wr::<u32>(c, 0, 0x75E5E0);
    wr::<u32>(c, 4, 0);
    wr::<u32>(c, 8, 0x75E570);
    wr::<u32>(c, 0xC, 0x756DF0);
    1
}

/// `_rxD3D9WorldSectorDefaultLightingCallback` (0x75E570): with world lighting (world flags 0x20) the
/// global lights plus the sector's world-lighting local lights (`flags & 2`, once per light frame
/// counter increment), then `_rwD3D9LightsEnable(any, -1)`.
#[hook(addr = 0x75E570, conv = cdecl, name = "_rxD3D9WorldSectorDefaultLightingCallback", expect = "A1 24 7B C9 00 8B 48 04")]
pub fn world_sector_default_lighting_callback(sector: GPtr<u8>) {
    let world = p8(rd::<u32>(rw_globals(), 4));
    if rd::<u8>(world, 8) & 0x20 == 0 {
        lights_enable(0, 0xFFFF_FFFF);
        return;
    }
    let mut any = lights_global_enable(2);
    let gl = rw_globals();
    wr::<u16>(gl, 0xA, rd::<u16>(gl, 0xA).wrapping_add(1));
    let end = sector.byte_add(0x40);
    let mut t = p8(rd::<u32>(sector, 0x40));
    while t != end {
        let light = p8(rd::<u32>(t, 8));
        if !light.is_null() && rd::<u8>(light, 2) & 2 != 0 {
            any |= light_local_enable(light);
        }
        t = p8(rd::<u32>(t, 0));
    }
    lights_enable(any, 0xFFFF_FFFF);
}

/// `_rpD3D9GetNumConstantsUsed` (0x75EDD0): the vertex-shader constant count of a shader descriptor
/// (4 bytes: lights, morph / fog / textures, flags, effect index into the table at 0xC94AF8).
#[hook(addr = 0x75EDD0, conv = cdecl, name = "_rpD3D9GetNumConstantsUsed", expect = "56 57 8B 7C 24 0C")]
pub fn get_num_constants_used(desc: GPtr<u8>) -> u32 {
    let mut n: i32 = 5;
    let b0 = rd::<u8>(desc, 0) as i32;
    if b0 & 0xF != 0 {
        n = (b0 & 0xF) * 2 + 5;
    }
    if b0 & 0xF0 != 0 {
        let s = b0 >> 4;
        n += (s + 3) / 4 + s * 2;
    }
    let b1 = rd::<u8>(desc, 1) as i32;
    if b1 & 0xF != 0 {
        let s = b1 & 0xF;
        n += (s + 3) / 4 + s * 4;
    }
    let b2 = rd::<u8>(desc, 2);
    if (b2 as i8) < 0 {
        n += 1;
    }
    let b3 = rd::<u8>(desc, 3) as u32;
    if b3 & 3 != 0 {
        n += 1;
    }
    if b3 & 0xFC != 0 {
        n = n.wrapping_add(g::<i32>(VS_LIGHT_TABLE.wrapping_add((b3 >> 2) * 4) as usize).read());
    }
    if b2 & 1 != 0 {
        n += 1;
    }
    n as u32
}

/// `_rpD3D9VertexShaderCachePurge` (0x75EE60): deletes every cached shader, empties the cache and resets
/// the lookup key (0xC926F8.. = -1, last-hit pointer 0xC94BFC = 0xC926F8).
#[hook(addr = 0x75EE60, conv = cdecl, name = "_rpD3D9VertexShaderCachePurge", expect = "A1 F8 4B C9 00 57")]
pub fn vertex_shader_cache_purge() {
    let mut i: i32 = 0;
    while i < gu(VS_COUNT) as i32 {
        let slot = g::<u32>(VS_ENTRIES.wrapping_add(i as u32 * 0x14) as usize);
        if slot.read() != 0 {
            delete_vertex_shader(p8(slot.read()));
            slot.write(0);
        }
        i += 1;
    }
    su(VS_COUNT, 0);
    su(0xC926F8, 0xFFFF_FFFF);
    su(0xC94BFC, 0xC926F8);
    for a in [0xC926FC, 0xC92700, 0xC92704, 0xC92708] {
        su(a, 0xFFFF_FFFF);
    }
}

/// `_rxD3D9VertexShaderDefaultGetMaterialShaderCallBack` (0x761010): `_rpD3D9GetVertexShader(desc, cbs)`.
#[hook(addr = 0x761010, conv = cdecl, name = "_rxD3D9VertexShaderDefaultGetMaterialShaderCallBack", expect = "8B 44 24 0C 8B 4C 24 08")]
pub fn vs_default_get_material_shader(_material: u32, desc: u32, cbs: u32) -> u32 {
    get_vertex_shader(desc, cbs)
}

/// `RwCore::RwCameraShowRaster` (0x7EE370): `RwRasterShowRaster(camera->frameBuffer, dev, flags)`;
/// the camera, or NULL on failure.
#[hook(addr = 0x7EE370, conv = cdecl, name = "RwCore::RwCameraShowRaster", expect = "8B 44 24 0C 8B 4C 24 08")]
pub fn camera_show_raster(camera: GPtr<u8>, dev: u32, flags: u32) -> u32 {
    if raster_show_raster(p8(rd::<u32>(camera, 0x60)), dev, flags) != 0 {
        camera.addr() as u32
    } else {
        0
    }
}

/// `RxNodeDefinitionGetD3D9ImmInstance` (0x80E990).
#[hook(addr = 0x80E990, conv = cdecl, name = "RxNodeDefinitionGetD3D9ImmInstance", expect = "B8 18 29 8E 00 C3")]
pub fn node_def_imm_instance() -> u32 {
    0x8E_2918
}

/// `RxNodeDefinitionGetD3D9SubmitNoLight` (0x80E9A0).
#[hook(addr = 0x80E9A0, conv = cdecl, name = "RxNodeDefinitionGetD3D9SubmitNoLight", expect = "B8 78 29 8E 00 C3")]
pub fn node_def_submit_no_light() -> u32 {
    0x8E_2978
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_ADD_DYNAMIC_GEOMETRY,
    HOOK_REMOVE_DYNAMIC_GEOMETRY,
    HOOK_WORLD_RESTORE_CALLBACK,
    HOOK_CREATE_WORLD_SECTOR_PIPE,
    HOOK_DESTROY_WORLD_SECTOR_PIPE,
    HOOK_CREATE_ATOMIC_PIPE,
    HOOK_VERTEX_DECLARATION_GET_SIZE,
    HOOK_VERTEX_DECLARATION_GET_STRIDE,
    HOOK_GEOMETRY_SET_USAGE_FLAGS,
    HOOK_GEOMETRY_GET_USAGE_FLAGS,
    HOOK_WORLD_SECTOR_SET_USAGE_FLAGS,
    HOOK_WORLD_SECTOR_GET_USAGE_FLAGS,
    HOOK_WORLD_SECTOR_PIPELINE_INIT,
    HOOK_WORLD_SECTOR_DEFAULT_LIGHTING_CALLBACK,
    HOOK_GET_NUM_CONSTANTS_USED,
    HOOK_VERTEX_SHADER_CACHE_PURGE,
    HOOK_VS_DEFAULT_GET_MATERIAL_SHADER,
    HOOK_CAMERA_SHOW_RASTER,
    HOOK_NODE_DEF_IMM_INSTANCE,
    HOOK_NODE_DEF_SUBMIT_NO_LIGHT,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
