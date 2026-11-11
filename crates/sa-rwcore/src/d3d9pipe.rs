//! PORT-RW-4 module `d3d9pipe`: the D3D9 atomic all-in-one pipeline node and its default lighting and
//! render callbacks, the callback setters / getters and the clipping helper (see docs/arch/rw-port.md,
//! PORT-RW-4). Ported from the 1.0 US bytes (0x756D90..0x7578B7).
//!
//! Resource entry (`RwResEntry` 0x18 + RxD3D9ResEntryHeader): +0x18 serial, +0x1C numMeshes, +0x20 index
//! buffer, +0x24 primitive type, +0x28 two vertex streams (0x10 each), +0x48 useOffsets, +0x4C vertex
//! declaration, meshes (RxD3D9InstanceData, 0x24) from +0x58: +0 numIndex, +4 minVert, +8 material,
//! +0xC vertexAlpha, +0x10 vertex shader, +0x14 baseIndex, +0x18 numVertices, +0x1C startIndex,
//! +0x20 numPrimitives. Callbacks (node private data): +0 instance, +4 reinstance, +8 lighting, +0xC render.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, p8, rd, rw_globals, wr, x};
use crate::d3d9state::device;

const LAST_DECL: usize = 0x8E2444;
const LAST_FVF: usize = 0x8E2440;
const LAST_VS: usize = 0x8E2448;
const LAST_PS: usize = 0x8E244C;
const LAST_IB: usize = 0x8E2450;
/// `AmbientSaturated` (r, g, b, a f32).
const AMB: usize = 0x8E2418;
/// RESOURCESMODULE offset in RwGlobals (the used-entries list pointer is at +0x24).
const RES_MODULE_OFFSET: usize = 0xC9BC58;

orig! {
    /// `IDirect3DDevice9::SetPixelShader` (+0x1AC)
    pub fn dev_set_pixel_shader(dev: GPtr<u8>, ps: u32) -> i32 = stdcall vslot 0x1AC => unported;
    /// `IDirect3DDevice9::SetIndices` (+0x1A0)
    pub fn dev_set_indices(dev: GPtr<u8>, ib: u32) -> i32 = stdcall vslot 0x1A0 => unported;
    /// `IDirect3DDevice9::SetVertexDeclaration` (+0x15C)
    pub fn dev_set_vertex_declaration(dev: GPtr<u8>, decl: u32) -> i32 = stdcall vslot 0x15C => unported;
    /// `IDirect3DDevice9::SetVertexShader` (+0x170)
    pub fn dev_set_vertex_shader(dev: GPtr<u8>, vs: u32) -> i32 = stdcall vslot 0x170 => unported;
    /// `IDirect3DDevice9::DrawPrimitive` (+0x144)
    pub fn dev_draw_primitive(dev: GPtr<u8>, prim: u32, start: u32, count: u32) -> i32 = stdcall vslot 0x144 => unported;
    /// `IDirect3DDevice9::DrawIndexedPrimitive` (+0x148)
    pub fn dev_draw_indexed_primitive(dev: GPtr<u8>, prim: u32, base: u32, min: u32, num: u32, start: u32, count: u32) -> i32 = stdcall vslot 0x148 => unported;

    /// `RpAtomicGetWorldBoundingSphere(atomic)` (0x749330)
    pub fn atomic_get_world_bounding_sphere(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x749330 => unported;
    /// `RwD3D9CameraIsSphereFullyInsideFrustum(camera, sphere)` (0x7FAD30)
    pub fn sphere_inside(camera: GPtr<u8>, sphere: GPtr<u8>) -> u32 = cdecl 0x7FAD30 => crate::d3d9dev::camera_is_sphere_fully_inside_frustum;
    /// `RwD3D9CameraIsBBoxFullyInsideFrustum(camera, bbox)` (0x7FAD90)
    pub fn bbox_inside(camera: GPtr<u8>, bbox: GPtr<u8>) -> u32 = cdecl 0x7FAD90 => crate::d3d9dev::camera_is_bbox_fully_inside_frustum;
    /// `RwD3D9SetRenderState(state, value)` (0x7FC2D0)
    pub fn set_render_state(state: u32, value: u32) -> u32 = cdecl 0x7FC2D0 => crate::d3d9state::set_render_state;
    /// `RwD3D9GetRenderState(state, out)` (0x7FC320)
    pub fn get_render_state(state: u32, out: GPtr<u32>) = cdecl 0x7FC320 => crate::d3d9state::get_render_state;
    /// `RwD3D9SetTextureStageState(stage, type, value)` (0x7FC340)
    pub fn set_tss(stage: u32, ty: u32, value: u32) = cdecl 0x7FC340 => crate::d3d9state::set_texture_stage_state;
    /// `RwD3D9SetTexture(texture, stage)` (0x7FDE70)
    pub fn set_texture(t: GPtr<u8>, stage: u32) -> u32 = cdecl 0x7FDE70 => crate::d3d9state::set_texture;
    /// `_rwD3D9RenderStateVertexAlphaEnable(enable)` (0x7FE0A0)
    pub fn vertex_alpha_enable(v: u32) -> u32 = cdecl 0x7FE0A0 => crate::d3d9state::rs_vertex_alpha_enable;
    /// `_rwD3D9RenderStateIsVertexAlphaEnable()` (0x7FE190)
    pub fn is_vertex_alpha_enable() -> u32 = cdecl 0x7FE190 => crate::d3d9state::rs_is_vertex_alpha_enable;
    /// `RwD3D9SetSurfaceProperties(surfaceProps, color, flags)` (0x7FC4D0)
    pub fn set_surface_properties(sp: GPtr<u8>, color: GPtr<u8>, flags: u32) -> u32 = cdecl 0x7FC4D0 => crate::d3d9state::set_surface_properties;
    /// `_rwD3D9SetStreams(streams, useOffsets)` (0x7FA090)
    pub fn set_streams(s: GPtr<u8>, use_offsets: u32) = cdecl 0x7FA090 => crate::d3d9dev::set_streams;
    /// `_rwD3D9RenderStateFlushCache()` (0x7FC200)
    pub fn flush_cache() = cdecl 0x7FC200 => crate::d3d9state::flush_cache;
    /// `_rwD3D9RasterHasAlpha(texture)` (0x4C9EA0, called with the material's texture)
    pub fn raster_has_alpha(t: GPtr<u8>) -> u32 = cdecl 0x4C9EA0 => unported;
    /// `RwResourcesFreeResEntry(entry)` (0x807DE0)
    pub fn res_free(e: GPtr<u8>) -> u32 = cdecl 0x807DE0 => unported;
    /// `_rxD3D9Instance(object, owner, type, &repEntry, meshHeader, instanceCb, allocateNative)` (0x756960)
    pub fn d3d9_instance(obj: GPtr<u8>, owner: GPtr<u8>, ty: u32, rep: GPtr<u8>, mesh: GPtr<u8>, cb: u32, native: u32) -> GPtr<u8> = cdecl 0x756960 => unported;
    /// `RwFrameGetLTM(frame)` (0x7F0990)
    pub fn frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
    /// `RwD3D9SetTransformWorld(matrix)` (0x7FA520)
    pub fn set_transform_world(m: GPtr<u8>) -> u32 = cdecl 0x7FA520 => crate::d3d9dev::set_transform_world;
    /// `_rwD3D9LightDirectionalEnable(light)` (0x756260)
    pub fn light_directional_enable(l: GPtr<u8>) -> u32 = cdecl 0x756260 => unported;
    /// `_rwD3D9LightLocalEnable(light)` (0x756390)
    pub fn light_local_enable(l: GPtr<u8>) -> u32 = cdecl 0x756390 => unported;
    /// `_rwD3D9LightsEnable(enable, type)` (0x756600)
    pub fn lights_enable(enable: u32, ty: u32) = cdecl 0x756600 => unported;
    /// The reinstance callback `(object, resEntry, instanceCb)` through its stored pointer
    pub fn call_reinstance_cb(f: u32, obj: u32, e: u32, inst: u32) -> u32 = cdecl dyn => unported;
    /// The lighting callback `(object)` through its stored pointer
    pub fn call_lighting_cb(f: u32, obj: u32) = cdecl dyn => unported;
    /// The render callback `(resEntry, object, type, flags)` through its stored pointer
    pub fn call_render_cb(f: u32, e: u32, obj: u32, ty: u32, flags: u32) = cdecl dyn => unported;
}

#[inline]
fn gu(addr: usize) -> u32 {
    g::<u32>(addr).read()
}

#[inline]
fn su(addr: usize, v: u32) {
    g::<u32>(addr).write(v)
}

/// The current camera (`RwGlobals` +0).
fn current_camera() -> GPtr<u8> {
    p8(rd::<u32>(rw_globals(), 0))
}

/// CLIPPING = "not fully inside the frustum" (the atomic's world sphere, or the sector's bbox at +0x60).
fn clip_value(object: GPtr<u8>, ty: u32) -> u32 {
    let cam = current_camera();
    let inside = if ty == 1 {
        sphere_inside(cam, atomic_get_world_bounding_sphere(object))
    } else {
        bbox_inside(cam, object.byte_add(0x60))
    };
    (inside == 0) as u32
}

/// `_rwD3D9EnableClippingIfNeeded` (0x756D90): `RwD3D9SetRenderState(CLIPPING, ...)` (a tail jump that
/// reuses the argument slots in the original: not reproduced).
#[hook(addr = 0x756D90, conv = cdecl, name = "_rwD3D9EnableClippingIfNeeded", expect = "A1 24 7B C9 00 56")]
pub fn enable_clipping_if_needed(object: GPtr<u8>, ty: u32) {
    set_render_state(0x88, clip_value(object, ty));
}

/// Draws one mesh of a resource entry (flush first; indexed when the entry has an index buffer).
fn draw_mesh(hdr: GPtr<u8>, m: GPtr<u8>) {
    flush_cache();
    if rd::<u32>(hdr, 0x20) != 0 {
        dev_draw_indexed_primitive(
            device(),
            rd::<u32>(hdr, 0x24),
            rd::<u32>(m, 0x14),
            0,
            rd::<u32>(m, 0x18),
            rd::<u32>(m, 0x1C),
            rd::<u32>(m, 0x20),
        );
    } else {
        dev_draw_primitive(device(), rd::<u32>(hdr, 0x24), rd::<u32>(m, 0x14), rd::<u32>(m, 0x20));
    }
}

/// The mesh's vertex shader through the cache (direct `SetVertexShader`, no result check).
fn mesh_vertex_shader(m: GPtr<u8>) {
    let vs = rd::<u32>(m, 0x10);
    if gu(LAST_VS) != vs {
        su(LAST_VS, vs);
        dev_set_vertex_shader(device(), vs);
    }
}

/// `_rxD3D9DefaultRenderCallback` (0x756DF0): pixel shader off, clipping, index buffer / streams /
/// declaration, then per mesh: unlit and not prelit, a black TFACTOR render with flat shading (texture
/// alpha kept for textured meshes); otherwise vertex alpha, surface properties or TFACTOR modulation and
/// the texture stages by state, then the draw.
#[hook(addr = 0x756DF0, conv = cdecl, name = "_rxD3D9DefaultRenderCallback", expect = "A1 4C 24 8E 00 83 EC 08")]
pub fn default_render_callback(hdr: GPtr<u8>, object: GPtr<u8>, ty: u32, flags: u32) {
    if gu(LAST_PS) != 0 {
        su(LAST_PS, 0);
        dev_set_pixel_shader(device(), 0);
    }
    // only the low byte of `type` is compared (`cmp al, 1`)
    set_render_state(0x88, clip_value(object, ty & 0xFF));
    let ib = rd::<u32>(hdr, 0x20);
    if ib != 0 && gu(LAST_IB) != ib {
        su(LAST_IB, ib);
        dev_set_indices(device(), rd::<u32>(hdr, 0x20));
    }
    set_streams(hdr.byte_add(0x28), rd::<u32>(hdr, 0x48));
    let decl = rd::<u32>(hdr, 0x4C);
    if gu(LAST_DECL) != decl {
        su(LAST_DECL, decl);
        su(LAST_FVF, 0xFFFF_FFFF);
        dev_set_vertex_declaration(device(), rd::<u32>(hdr, 0x4C));
    }
    let mut lighting: u32 = 0;
    get_render_state(0x89, GPtr::from_raw(&mut lighting));
    let n = rd::<u32>(hdr, 0x1C);
    if lighting == 0 && flags & 8 == 0 {
        let (mut dither, mut shade) = (0u32, 0u32);
        get_render_state(0x1A, GPtr::from_raw(&mut dither));
        get_render_state(9, GPtr::from_raw(&mut shade));
        vertex_alpha_enable(0);
        set_render_state(0x3C, 0xFF00_0000);
        set_render_state(0x1A, 0);
        set_render_state(9, 1);
        set_tss(0, 1, 3);
        set_tss(0, 2, 2);
        set_tss(0, 3, 3);
        set_tss(0, 4, 3);
        set_tss(0, 5, 2);
        set_tss(0, 6, 3);
        set_texture(GPtr::NULL, 0);
        let textured = flags & 0x84 != 0;
        let mut alpha_tex = false;
        for i in 0..n {
            let m = hdr.byte_add(0x58 + i as isize * 0x24);
            let mat = p8(rd::<u32>(m, 8));
            let tex = p8(rd::<u32>(mat, 0));
            if textured && !tex.is_null() && raster_has_alpha(tex) != 0 {
                set_texture(p8(rd::<u32>(p8(rd::<u32>(m, 8)), 0)), 0);
                if !alpha_tex {
                    alpha_tex = true;
                    set_tss(0, 4, 4);
                }
            } else if alpha_tex {
                alpha_tex = false;
                set_tss(0, 4, 3);
                set_texture(GPtr::NULL, 0);
            }
            mesh_vertex_shader(m);
            draw_mesh(hdr, m);
        }
        set_render_state(0x1A, dither);
        set_render_state(9, shade);
        return;
    }
    let mut va = is_vertex_alpha_enable();
    let mut last: u32 = 0x8000_0000;
    if n == 0 {
        return;
    }
    for i in 0..n {
        let m = hdr.byte_add(0x58 + i as isize * 0x24);
        let mat = p8(rd::<u32>(m, 8));
        let mut st: u32 = 0;
        if rd::<u8>(mat, 7) == 0xFF && rd::<u32>(m, 0xC) == 0 {
            if va != 0 {
                va = 0;
                vertex_alpha_enable(0);
            }
        } else if va == 0 {
            va = 1;
            vertex_alpha_enable(1);
        }
        if lighting != 0 {
            set_surface_properties(mat.byte_add(0xC), mat.byte_add(4), flags);
        } else if flags & 0x40 != 0 && rd::<u32>(mat, 4) != 0xFFFF_FFFF {
            st = 2;
            let c = |k: isize| rd::<u8>(mat, 4 + k) as u32;
            set_render_state(0x3C, (c(3) << 24) | (c(0) << 16) | (c(1) << 8) | c(2));
        }
        let tex = p8(rd::<u32>(mat, 0));
        if !tex.is_null() && flags & 0x84 != 0 {
            st |= 1;
            set_texture(tex, 0);
        } else {
            set_texture(GPtr::NULL, 0);
        }
        if st != last {
            if st & 1 != 0 {
                if last & 1 == 0 {
                    set_tss(0, 1, 4);
                    set_tss(0, 2, 2);
                    set_tss(0, 3, 0);
                    set_tss(0, 4, 4);
                    set_tss(0, 5, 2);
                    set_tss(0, 6, 0);
                }
                if st & 2 != 0 {
                    set_tss(1, 1, 4);
                    set_tss(1, 2, 1);
                    set_tss(1, 3, 3);
                    set_tss(1, 4, 4);
                    set_tss(1, 5, 1);
                    set_tss(1, 6, 3);
                } else if last == 3 {
                    set_tss(1, 1, 1);
                    set_tss(1, 4, 1);
                }
            } else {
                if st & 2 != 0 {
                    set_tss(0, 1, 4);
                    set_tss(0, 2, 0);
                    set_tss(0, 3, 3);
                    set_tss(0, 4, 4);
                    set_tss(0, 5, 0);
                    set_tss(0, 6, 3);
                } else {
                    set_tss(0, 1, 3);
                    set_tss(0, 3, 0);
                    set_tss(0, 4, 3);
                    set_tss(0, 6, 0);
                }
                if last == 3 {
                    set_tss(1, 1, 1);
                    set_tss(1, 4, 1);
                }
            }
            last = st;
        }
        mesh_vertex_shader(m);
        draw_mesh(hdr, m);
    }
    if last == 3 {
        set_tss(1, 1, 1);
        set_tss(1, 4, 1);
    }
}

/// The callback block of an all-in-one node (`node + 0x14`).
fn callbacks(node: GPtr<u8>) -> GPtr<u8> {
    p8(rd::<u32>(node, 0x14))
}

/// `RxD3D9AllInOneSetInstanceCallBack` (0x757380).
#[hook(addr = 0x757380, conv = cdecl, name = "RxD3D9AllInOneSetInstanceCallBack", expect = "8B 44 24 04 8B 54 24 08")]
pub fn set_instance_callback(node: GPtr<u8>, cb: u32) {
    wr::<u32>(callbacks(node), 0, cb);
}

/// `RxD3D9AllInOneGetInstanceCallBack` (0x757390).
#[hook(addr = 0x757390, conv = cdecl, name = "RxD3D9AllInOneGetInstanceCallBack", expect = "8B 44 24 04 8B 48 14")]
pub fn get_instance_callback(node: GPtr<u8>) -> u32 {
    rd::<u32>(callbacks(node), 0)
}

/// `RxD3D9AllInOneSetReinstanceCallBack` (0x7573A0).
#[hook(addr = 0x7573A0, conv = cdecl, name = "RxD3D9AllInOneSetReinstanceCallBack", expect = "8B 44 24 04 8B 54 24 08")]
pub fn set_reinstance_callback(node: GPtr<u8>, cb: u32) {
    wr::<u32>(callbacks(node), 4, cb);
}

/// `RxD3D9AllInOneGetReinstanceCallBack` (0x7573B0).
#[hook(addr = 0x7573B0, conv = cdecl, name = "RxD3D9AllInOneGetReinstanceCallBack", expect = "8B 44 24 04 8B 48 14")]
pub fn get_reinstance_callback(node: GPtr<u8>) -> u32 {
    rd::<u32>(callbacks(node), 4)
}

/// `RxD3D9AllInOneSetLightingCallBack` (0x7573C0).
#[hook(addr = 0x7573C0, conv = cdecl, name = "RxD3D9AllInOneSetLightingCallBack", expect = "8B 44 24 04 8B 54 24 08")]
pub fn set_lighting_callback(node: GPtr<u8>, cb: u32) {
    wr::<u32>(callbacks(node), 8, cb);
}

/// `RxD3D9AllInOneGetLightingCallBack` (0x7573D0).
#[hook(addr = 0x7573D0, conv = cdecl, name = "RxD3D9AllInOneGetLightingCallBack", expect = "8B 44 24 04 8B 48 14")]
pub fn get_lighting_callback(node: GPtr<u8>) -> u32 {
    rd::<u32>(callbacks(node), 8)
}

/// `RxD3D9AllInOneSetRenderCallBack` (0x7573E0).
#[hook(addr = 0x7573E0, conv = cdecl, name = "RxD3D9AllInOneSetRenderCallBack", expect = "8B 44 24 04 8B 54 24 08")]
pub fn set_render_callback(node: GPtr<u8>, cb: u32) {
    wr::<u32>(callbacks(node), 0xC, cb);
}

/// `RxD3D9AllInOneGetRenderCallBack` (0x7573F0).
#[hook(addr = 0x7573F0, conv = cdecl, name = "RxD3D9AllInOneGetRenderCallBack", expect = "8B 44 24 04 8B 48 14")]
pub fn get_render_callback(node: GPtr<u8>) -> u32 {
    rd::<u32>(callbacks(node), 0xC)
}

/// `_rxD3D9AtomicDefaultLightingCallback` (0x757400): with rpGEOMETRYLIGHT and a world: ambient sum of
/// the world's atomic-lighting lights (directional ones enabled when the geometry has normals), AMBIENT
/// render state; with normals, every local light of the atomic's sectors whose sphere reaches the atomic's
/// world sphere (`(r_l + r_a)^2 > d^2`, once per light frame). `_rwD3D9LightsEnable(any, 1)`.
#[hook(addr = 0x757400, conv = cdecl, name = "_rxD3D9AtomicDefaultLightingCallback", expect = "83 EC 08 53 55")]
pub fn atomic_default_lighting_callback(atomic: GPtr<u8>) {
    let flags = rd::<u32>(p8(rd::<u32>(atomic, 0x18)), 8);
    let world = rd::<u32>(rw_globals(), 4);
    if flags & 0x20 == 0 || world == 0 {
        lights_enable(0, 1);
        return;
    }
    for (i, v) in [0u32, 0, 0, 0x3F80_0000].into_iter().enumerate() {
        su(AMB + i * 4, v);
    }
    let world = p8(rd::<u32>(rw_globals(), 4));
    let mut any: u32 = 0;
    let end = world.byte_add(0x3C);
    let mut link = p8(rd::<u32>(world, 0x3C));
    while link != end {
        let light = link.byte_add(-0x34);
        if !light.is_null() && rd::<u8>(light, 2) & 1 != 0 {
            if rd::<u8>(light, 1) == 1 {
                if flags & 0x10 != 0 {
                    light_directional_enable(light);
                    any = 1;
                }
            } else {
                for k in 0..3 {
                    let a = g::<f32>(AMB + k * 4);
                    a.write(x(a.read()).fadd_m(rd::<f32>(light, 0x18 + k as isize * 4)).to_f32());
                }
                any = 1;
            }
        }
        link = p8(rd::<u32>(link, 0));
    }
    set_render_state(0x8B, if any != 0 { 0xFFFF_FFFF } else { 0 });
    if flags & 0x10 != 0 {
        let gl = rw_globals();
        wr::<u16>(gl, 0xA, rd::<u16>(gl, 0xA).wrapping_add(1));
        let ties_end = atomic.byte_add(0x64);
        let mut tie = p8(rd::<u32>(atomic, 0x64));
        while tie != ties_end {
            let sector = p8(rd::<u32>(tie, 8));
            let lend = sector.byte_add(0x40);
            let mut lt = p8(rd::<u32>(sector, 0x40));
            while lt != lend {
                let light = p8(rd::<u32>(lt, 8));
                let frame_no = rd::<u16>(rw_globals(), 0xA);
                if !light.is_null() && rd::<u16>(light, 0x3C) != frame_no && rd::<u8>(light, 2) & 1 != 0 {
                    wr::<u16>(light, 0x3C, frame_no);
                    let pos = frame_get_ltm(p8(rd::<u32>(p8(rd::<u32>(lt, 8)), 4))).byte_add(0x30);
                    let s = atomic_get_world_bounding_sphere(atomic);
                    let dx = x(rd::<f32>(s, 0)).fsub_m(rd::<f32>(pos, 0));
                    let dy = x(rd::<f32>(s, 4)).fsub_m(rd::<f32>(pos, 4));
                    let dz = x(rd::<f32>(s, 8)).fsub_m(rd::<f32>(pos, 8));
                    let light = p8(rd::<u32>(lt, 8));
                    let r = x(rd::<f32>(light, 0x14)).fadd_m(rd::<f32>(s, 0xC));
                    let d = dy.fmul(dy).fadd(dx.fmul(dx)).fadd(dz.fmul(dz));
                    if !r.fmul(r).fcom(d).test_any(0x41) {
                        any |= light_local_enable(light);
                    }
                }
                lt = p8(rd::<u32>(lt, 0));
            }
            tie = p8(rd::<u32>(tie, 0));
        }
    }
    lights_enable(any, 1);
}

/// The squared length `(x*x + y*y) + z*z` of a matrix row (x87, stored as f32) within [0.9, 1.1] by the
/// signed integer compare of its bits.
fn row_normalised(m: GPtr<u8>, off: isize) -> bool {
    let (vx, vy, vz) = (x(rd::<f32>(m, off)), x(rd::<f32>(m, off + 4)), x(rd::<f32>(m, off + 8)));
    let l = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).to_f32().to_bits() as i32;
    (0x3F66_6666..=0x3F8C_CCCD).contains(&l)
}

/// `_rwD3D9AtomicAllInOneNode` (0x7575F0): instances (or reuses, re-instancing a locked geometry) the
/// atomic's resource entry, then unless rpGEOMETRYNATIVEINSTANCE: lighting callback, world transform,
/// NORMALIZENORMALS when lit and a row of the non-orthonormal LTM is not unit length, render callback.
/// Returns 1, or 0 when instancing / reinstancing fails.
#[hook(addr = 0x7575F0, conv = cdecl, name = "_rwD3D9AtomicAllInOneNode", expect = "8B 44 24 08 83 EC 0C")]
#[allow(clippy::needless_late_init)]
pub fn atomic_all_in_one_node(node: GPtr<u8>, params: GPtr<u8>) -> u32 {
    let cbs = callbacks(node);
    let atomic = p8(rd::<u32>(params, 0));
    let geo = p8(rd::<u32>(atomic, 0x18));
    if rd::<i32>(geo, 0x14) <= 0 || rd::<u16>(p8(rd::<u32>(geo, 0x54)), 4) == 0 {
        return 1;
    }
    let flags = rd::<u32>(geo, 8);
    let res: GPtr<u8>;
    if flags & 0x100_0000 != 0 {
        res = p8(rd::<u32>(geo, 0x58));
    } else {
        let single = rd::<u32>(geo, 0x18) == 1;
        let mut e = if single { p8(rd::<u32>(geo, 0x58)) } else { p8(rd::<u32>(atomic, 0x14)) };
        let mut reuse = false;
        if !e.is_null() {
            let mesh = p8(rd::<u32>(geo, 0x54));
            if rd::<u32>(e, 0x18) != rd::<u16>(mesh, 6) as u32 {
                res_free(e);
            } else {
                if rd::<u16>(geo, 0xC) != 0 || !single {
                    let re = rd::<u32>(cbs, 4);
                    if re != 0 {
                        let ok = call_reinstance_cb(re, atomic.addr() as u32, e.addr() as u32, rd::<u32>(cbs, 0));
                        if ok == 0 {
                            res_free(e);
                            return 0;
                        }
                    }
                    wr::<u32>(atomic, 0x4C, rd::<u32>(atomic, 0x4C) & !1);
                    wr::<u16>(geo, 0xC, 0);
                }
                reuse = true;
            }
        }
        if reuse {
            if rd::<u32>(e, 0) != 0 {
                // _rwResourcesUseResEntry: move the entry to the head of the used list.
                wr::<u32>(p8(rd::<u32>(e, 4)), 0, rd::<u32>(e, 0));
                wr::<u32>(p8(rd::<u32>(e, 0)), 4, rd::<u32>(e, 4));
                let head = || p8(rd::<u32>(rw_globals().byte_add(gu(RES_MODULE_OFFSET) as i32 as isize), 0x24));
                wr::<u32>(e, 0, rd::<u32>(head(), 0));
                wr::<u32>(e, 4, head().addr() as u32);
                wr::<u32>(p8(rd::<u32>(head(), 0)), 4, e.addr() as u32);
                wr::<u32>(head(), 0, e.addr() as u32);
            }
        } else {
            let mesh = p8(rd::<u32>(geo, 0x54));
            let (owner, slot) = if rd::<u32>(geo, 0x18) == 1 {
                (geo, geo.byte_add(0x58))
            } else {
                (atomic, atomic.byte_add(0x14))
            };
            e = d3d9_instance(atomic, owner, 1, slot, mesh, rd::<u32>(cbs, 0), (flags >> 25) & 1);
            if e.is_null() {
                return 0;
            }
            wr::<u16>(geo, 0xC, 0);
        }
        res = e;
    }
    if flags & 0x200_0000 != 0 {
        return 1;
    }
    let light_cb = rd::<u32>(cbs, 8);
    if light_cb != 0 {
        call_lighting_cb(light_cb, atomic.addr() as u32);
    }
    let ltm = frame_get_ltm(p8(rd::<u32>(atomic, 4)));
    set_transform_world(ltm);
    let mut lighting: u32 = 0;
    get_render_state(0x89, GPtr::from_raw(&mut lighting));
    let normalize = lighting != 0
        && rd::<u32>(ltm, 0xC) & 0x2_0001 == 0
        && !(row_normalised(ltm, 0) && row_normalised(ltm, 0x10) && row_normalised(ltm, 0x20));
    set_render_state(0x8F, normalize as u32);
    let render_cb = rd::<u32>(cbs, 0xC);
    if render_cb != 0 {
        call_render_cb(render_cb, res.addr() as u32, atomic.addr() as u32, 1, flags);
    }
    1
}

/// `_rwD3D9AtomicAllInOnePipelineInit` (0x757890): the default callbacks (instance 0x7578C0, reinstance
/// 0x758270, lighting 0x757400, render 0x756DF0); returns 1.
#[hook(addr = 0x757890, conv = cdecl, name = "_rwD3D9AtomicAllInOnePipelineInit", expect = "8B 44 24 04 8B 40 14")]
pub fn atomic_all_in_one_pipeline_init(node: GPtr<u8>) -> u32 {
    let c = callbacks(node);
    wr::<u32>(c, 0, 0x7578C0);
    wr::<u32>(c, 4, 0x758270);
    wr::<u32>(c, 8, 0x757400);
    wr::<u32>(c, 0xC, 0x756DF0);
    1
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_ENABLE_CLIPPING_IF_NEEDED,
    HOOK_DEFAULT_RENDER_CALLBACK,
    HOOK_SET_INSTANCE_CALLBACK,
    HOOK_GET_INSTANCE_CALLBACK,
    HOOK_SET_REINSTANCE_CALLBACK,
    HOOK_GET_REINSTANCE_CALLBACK,
    HOOK_SET_LIGHTING_CALLBACK,
    HOOK_GET_LIGHTING_CALLBACK,
    HOOK_SET_RENDER_CALLBACK,
    HOOK_GET_RENDER_CALLBACK,
    HOOK_ATOMIC_DEFAULT_LIGHTING_CALLBACK,
    HOOK_ATOMIC_ALL_IN_ONE_NODE,
    HOOK_ATOMIC_ALL_IN_ONE_PIPELINE_INIT,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
