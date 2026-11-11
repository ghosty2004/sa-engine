//! PORT-RW-5 module `d3d9node`: the D3D9 world-sector all-in-one node, the atomic node-definition
//! getter, the vertex-shader cache lookup and the default vertex-shader pipeline callbacks (see
//! docs/arch/rw-port.md, PORT-RW-5). Ported from the 1.0 US bytes.
//!
//! Vertex-shader cache (0xC926F8, at most 0x100 entries of 0x14, sorted by key for the binary search):
//! +0 descriptor key, +4 the frame stamp (`RwGlobals` +8, u16 zero-extended), +8 the shader, +0xC / +0x10
//! the two dwords the shader creator returns through its second argument. Count 0xC94BF8, last hit
//! 0xC94BFC.
//!
//! Vertex-shader descriptor (`_rpD3D9VertexShaderDescriptor`, one dword): byte 1 bits 4-7 number of
//! texture coordinate sets; byte 2: 0x01 morphing, 0x10 prelit, 0x20 normals, 0x40 normalise / lighting
//! in local space, 0x80 modulate material colour; byte 3 bits 0-1 fog mode.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, p8, rd, rw_globals, wr, x};
use crate::d3d9state::device;

/// RESOURCESMODULE offset in RwGlobals (the used-entries list pointer is at +0x24).
const RES_MODULE_OFFSET: usize = 0xC9BC58;
const LAST_VS: usize = 0x8E2448;
const LAST_PS: usize = 0x8E244C;
const VS_TABLE: u32 = 0xC926F8;
const VS_COUNT: usize = 0xC94BF8;
const VS_LAST: usize = 0xC94BFC;
/// The f32 0.0 the morph test compares against.
const ZERO_ADDR: usize = 0x858B50;

orig! {
    /// `IDirect3DDevice9::SetPixelShader` (+0x1AC)
    fn dev_set_pixel_shader(dev: GPtr<u8>, ps: u32) -> i32 = stdcall vslot 0x1AC => unported;
    /// `IDirect3DDevice9::SetVertexShader` (+0x170)
    fn dev_set_vertex_shader(dev: GPtr<u8>, vs: u32) -> i32 = stdcall vslot 0x170 => unported;
    /// `IDirect3DDevice9::DrawPrimitive` (+0x144)
    fn dev_draw_primitive(dev: GPtr<u8>, prim: u32, start: u32, count: u32) -> i32 = stdcall vslot 0x144 => unported;
    /// `IDirect3DDevice9::DrawIndexedPrimitive` (+0x148)
    fn dev_draw_indexed_primitive(dev: GPtr<u8>, prim: u32, base: u32, min: u32, num: u32, start: u32, count: u32) -> i32 = stdcall vslot 0x148 => unported;

    /// `RwResourcesFreeResEntry(entry)` (0x807DE0)
    fn res_free(e: GPtr<u8>) -> u32 = cdecl 0x807DE0 => crate::memory::rw_resources_free_res_entry;
    /// `_rxD3D9Instance(object, owner, type, &repEntry, meshHeader, instanceCb, allocateNative)` (0x756960)
    fn d3d9_instance(obj: GPtr<u8>, owner: GPtr<u8>, ty: u32, rep: GPtr<u8>, mesh: GPtr<u8>, cb: u32, native: u32) -> GPtr<u8> = cdecl 0x756960 => unported;
    /// `RwD3D9SetTransformWorld(matrix)` (0x7FA520)
    fn set_transform_world(m: GPtr<u8>) -> u32 = cdecl 0x7FA520 => crate::d3d9dev::set_transform_world;
    /// `RwD3D9SetRenderState(state, value)` (0x7FC2D0)
    fn set_render_state(state: u32, value: u32) -> u32 = cdecl 0x7FC2D0 => crate::d3d9state::set_render_state;
    /// `RwD3D9GetRenderState(state, out)` (0x7FC320)
    fn get_render_state(state: u32, out: GPtr<u32>) = cdecl 0x7FC320 => crate::d3d9state::get_render_state;
    /// `RwD3D9SetTextureStageState(stage, type, value)` (0x7FC340)
    fn set_tss(stage: u32, ty: u32, value: u32) = cdecl 0x7FC340 => crate::d3d9state::set_texture_stage_state;
    /// `RwD3D9SetTexture(texture, stage)` (0x7FDE70)
    fn set_texture(t: GPtr<u8>, stage: u32) -> u32 = cdecl 0x7FDE70 => crate::d3d9state::set_texture;
    /// `_rwD3D9RenderStateFlushCache()` (0x7FC200)
    fn flush_cache() = cdecl 0x7FC200 => crate::d3d9state::flush_cache;
    /// `RwFrameGetLTM(frame)` (0x7F0990)
    fn frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => crate::frame::get_ltm;
    /// `RpD3D9GeometryGetUsageFlags(geometry)` (0x7588D0)
    fn geometry_get_usage_flags(geo: GPtr<u8>) -> u32 = cdecl 0x7588D0 => crate::d3d9misc::geometry_get_usage_flags;
    /// `RwD3D9DeleteVertexShader(shader)` (0x7FAC90)
    fn delete_vertex_shader(vs: GPtr<u8>) -> u32 = cdecl 0x7FAC90 => crate::d3d9dev::delete_vertex_shader;
    /// `_rpD3D9VertexShaderGenerate(desc, out)` (0x75F0B0): returns the shader, fills two dwords
    fn create_vertex_shader(desc: GPtr<u32>, out: GPtr<u32>) -> u32 = cdecl 0x75F0B0 => crate::d3d9vsgen::vertex_shader_generate;
    /// CRT `memmove(dst, src, n)` (0x822A20)
    fn crt_memmove(dst: u32, src: u32, n: u32) -> u32 = cdecl 0x822A20 => unported;
    /// `RwRenderStateGet(state, out)` through `RwGlobals` +0x24 (the device's render-state getter)
    fn rw_render_state_get(state: u32, out: GPtr<u32>) -> u32 = cdecl table 0xC97B24 0x24 => unported;
    /// The lighting callback `(object)` through its stored pointer
    fn call_lighting_cb(f: u32, obj: u32) = cdecl dyn => unported;
    /// The render callback `(resEntry, object, type, flags)` through its stored pointer
    fn call_render_cb(f: u32, e: u32, obj: u32, ty: u32, flags: u32) = cdecl dyn => unported;
}

#[inline]
fn gu(addr: usize) -> u32 {
    g::<u32>(addr).read()
}

#[inline]
fn su(addr: usize, v: u32) {
    g::<u32>(addr).write(v)
}

/// The frame stamp of the vertex-shader cache: `RwGlobals` +8 as u16, zero-extended.
fn frame_stamp() -> u32 {
    rd::<u16>(rw_globals(), 8) as u32
}

/// `RxNodeDefinitionGetD3D9AtomicAllInOne()` (0x7582E0): the static node definition at 0x8D6338.
#[hook(addr = 0x7582E0, conv = cdecl, name = "RxNodeDefinitionGetD3D9AtomicAllInOne", expect = "B8 38 63 8D 00 C3",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn node_def_atomic_all_in_one() -> u32 {
    0x8D_6338
}

/// `_rwD3D9WorldSectorAllInOneNode(node, params)` (0x75E3F0). With no polygons (`numTriangles` u16 at
/// +0x82 is 0) or an empty mesh header (+0x78, `numMeshes` u16 at +4) returns 1. The flags are the
/// current world's (`RwGlobals` +4, flags at +8). Unless rpWORLDNATIVE (0x1000000): the sector's
/// resource entry (+0x34) is freed when its serial (+0x18) differs from the mesh header's serial (u16
/// at +6), then (or when there is none) `_rxD3D9Instance(sector, sector, 0xFF, &sector->repEntry,
/// meshHeader, instanceCb, (flags >> 25) & 1)` (NULL: returns 0); an entry with the same serial is moved
/// to the head of the used-resources list (`_rwResourcesUseResEntry` inlined) when it is linked. Unless
/// rpWORLDNATIVEINSTANCE (0x2000000): the lighting callback (sector), `RwD3D9SetTransformWorld(NULL)`,
/// `RwD3D9SetRenderState(NORMALIZENORMALS, 0)` and the render callback `(entry, sector, 0xFF, flags)`.
/// Returns 1. The original keeps the callbacks and the mesh header in its argument slots (not
/// reproduced; the callers pop them).
#[hook(addr = 0x75E3F0, conv = cdecl, name = "_rwD3D9WorldSectorAllInOneNode", expect = "8B 44 24 08 8B 4C 24 04",
       unverified = "rw5-node-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the sector's resource entry and the resources used list")]
pub fn world_sector_all_in_one_node(node: GPtr<u8>, params: GPtr<u8>) -> u32 {
    let cbs = p8(rd::<u32>(node, 0x14));
    let sector = p8(rd::<u32>(params, 0));
    if rd::<u16>(sector, 0x82) == 0 {
        return 1;
    }
    let mesh = p8(rd::<u32>(sector, 0x78));
    if rd::<u16>(mesh, 4) == 0 {
        return 1;
    }
    let mut e = p8(rd::<u32>(sector, 0x34));
    let flags = rd::<u32>(p8(rd::<u32>(rw_globals(), 4)), 8);
    if flags & 0x100_0000 == 0 {
        let mut instance = true;
        if !e.is_null() {
            if rd::<u32>(e, 0x18) == rd::<u16>(mesh, 6) as u32 {
                instance = false;
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
                res_free(e);
            }
        }
        if instance {
            e = d3d9_instance(
                sector,
                sector,
                0xFF,
                sector.byte_add(0x34),
                mesh,
                rd::<u32>(cbs, 0),
                (flags >> 25) & 1,
            );
            if e.is_null() {
                return 0;
            }
        }
    }
    if flags & 0x200_0000 != 0 {
        return 1;
    }
    let light_cb = rd::<u32>(cbs, 8);
    if light_cb != 0 {
        call_lighting_cb(light_cb, sector.addr() as u32);
    }
    set_transform_world(GPtr::NULL);
    set_render_state(0x8F, 0);
    let render_cb = rd::<u32>(cbs, 0xC);
    if render_cb != 0 {
        call_render_cb(render_cb, e.addr() as u32, sector.addr() as u32, 0xFF, flags);
    }
    1
}

/// A matrix row's squared length outside [0.9, 1.1] (x87 `(x*x + y*y) + z*z` stored as f32, signed
/// integer compares of the bits against 0x3F8CCCCD and 0x3F666666).
fn row_off_unit(m: GPtr<u8>, off: isize) -> bool {
    let (vx, vy, vz) = (x(rd::<f32>(m, off)), x(rd::<f32>(m, off + 4)), x(rd::<f32>(m, off + 8)));
    let l = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).to_f32().to_bits() as i32;
    !(0x3F66_6666..=0x3F8C_CCCD).contains(&l)
}

/// `_rxD3D9VertexShaderDefaultBeginCallBack(object, type, desc)` (0x760DF0): zeroes `*desc` and fills
/// it. Atomic (type 1): morphing (byte 2 |= 1) when the geometry has more than one morph target, the
/// interpolator's start and end targets (u16 +0x50 / +0x52) differ and `recipTime * position` (+0x5C *
/// +0x58, x87) is not equal to 0.0 (or unordered); texture-coordinate count (geometry +0x1C) into
/// byte 1's high nibble; prelit (flag 0x08) byte 2 |= 0x10; with normals (flag 0x10): byte 2 |= 0x20,
/// and |= 0x40 when `RpD3D9GeometryGetUsageFlags` has a bit of 0xFFF or (otherwise) the LTM is neither
/// identity nor orthonormal (flags & 0x20001 == 0) and a row's squared length is outside [0.9, 1.1];
/// modulate (flag 0x40) byte 2 |= 0x80 only on the normals path. Other types (world sector, from the
/// current world's flags): TEXTURED2 (0x80) byte 1 = 0x20 | low nibble, else TEXTURED (4) 0x10 | low
/// nibble; PRELIT 0x10, NORMALS 0x20, MODULATE 0x80 into byte 2. Then, when FOGENABLE (render state
/// 0x1C) is on, the FOGTYPE from `RwRenderStateGet` (into the original's `desc` argument slot; a local
/// here) sets byte 3 bits 0-1: linear 1 (bit 1 cleared), exponential 2 (bit 0 cleared), exponential² 3.
#[hook(addr = 0x760DF0, conv = cdecl, name = "_rxD3D9VertexShaderDefaultBeginCallBack", expect = "51 8B 4C 24 0C",
       unverified = "rw5-node-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the caller's descriptor; the original's RwRenderStateGet writes its own desc argument slot")]
pub fn vs_default_begin_callback(object: GPtr<u8>, ty: u32, desc: GPtr<u8>) {
    wr::<u32>(desc, 0, 0);
    let or2 = |v: u8| wr::<u8>(desc, 2, rd::<u8>(desc, 2) | v);
    if ty == 1 {
        let atomic = object;
        let geo = p8(rd::<u32>(atomic, 0x18));
        let flags = rd::<u32>(geo, 8);
        if rd::<u32>(geo, 0x18) != 1 && rd::<u16>(atomic, 0x50) != rd::<u16>(atomic, 0x52) {
            let c = x(rd::<f32>(atomic, 0x5C))
                .fmul(x(rd::<f32>(atomic, 0x58)))
                .fcom(x(g::<f32>(ZERO_ADDR).read()));
            if c.test_parity(0x44) {
                or2(1);
            }
        }
        let al = rd::<u8>(geo, 0x1C) << 4;
        wr::<u8>(desc, 1, (rd::<u8>(desc, 1) & 0xF) | (al & 0xF0));
        if flags & 8 != 0 {
            or2(0x10);
        }
        if flags & 0x10 != 0 {
            let usage = geometry_get_usage_flags(geo);
            or2(0x20);
            if usage & 0xFFF != 0 {
                or2(0x40);
            } else {
                let ltm = frame_get_ltm(p8(rd::<u32>(atomic, 4)));
                if rd::<u32>(ltm, 0xC) & 0x2_0001 == 0
                    && (row_off_unit(ltm, 0) || row_off_unit(ltm, 0x10) || row_off_unit(ltm, 0x20))
                {
                    or2(0x40);
                }
            }
            if flags & 0x40 != 0 {
                or2(0x80);
            }
        }
    } else {
        let flags = rd::<u32>(p8(rd::<u32>(rw_globals(), 4)), 8) as u8;
        if flags & 0x80 != 0 {
            wr::<u8>(desc, 1, (rd::<u8>(desc, 1) & 0xF) | 0x20);
        } else if flags & 4 != 0 {
            wr::<u8>(desc, 1, (rd::<u8>(desc, 1) & 0xF) | 0x10);
        }
        if flags & 8 != 0 {
            or2(0x10);
        }
        if flags & 0x10 != 0 {
            or2(0x20);
        }
        if flags & 0x40 != 0 {
            or2(0x80);
        }
    }
    let mut fog: u32 = 0;
    get_render_state(0x1C, GPtr::from_raw(&mut fog));
    if fog == 0 {
        return;
    }
    let mut ty_fog: u32 = desc.addr() as u32;
    rw_render_state_get(0x10, GPtr::from_raw(&mut ty_fog));
    let b3 = rd::<u8>(desc, 3);
    match ty_fog {
        1 => wr::<u8>(desc, 3, (b3 & 0xFD) | 1),
        2 => wr::<u8>(desc, 3, (b3 & 0xFE) | 2),
        3 => wr::<u8>(desc, 3, b3 | 3),
        _ => {}
    }
}

/// `_rxD3D9VertexShaderDefaultMeshRenderCallBack(header, mesh)` (0x761030): clears the cached pixel
/// shader (0x8E244C = 0, then `SetPixelShader(NULL)`) when one is set; with a material texture
/// `RwD3D9SetTexture(texture, 0)` and stage 0 MODULATE texture × diffuse for colour and alpha (COLOROP 4,
/// COLORARG1 2, COLORARG2 0, ALPHAOP 4, ALPHAARG1 2, ALPHAARG2 0), else `RwD3D9SetTexture(NULL, 0)` and
/// SELECTARG2 diffuse (COLOROP 3, COLORARG2 0, ALPHAOP 3, ALPHAARG2 0); sets the mesh's vertex shader
/// (+0x10) when it differs from the cached one (0x8E2448); flushes the render-state cache and draws:
/// `DrawIndexedPrimitive(primType, baseIndex, 0, numVertices, startIndex, numPrimitives)` with an index
/// buffer, else `DrawPrimitive(primType, baseIndex, numPrimitives)`. Returns the draw's HRESULT.
#[hook(addr = 0x761030, conv = cdecl, name = "_rxD3D9VertexShaderDefaultMeshRenderCallBack", expect = "A1 4C 24 8E 00",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn vs_default_mesh_render_callback(header: GPtr<u8>, mesh: GPtr<u8>) -> i32 {
    if gu(LAST_PS) != 0 {
        let dev = device();
        su(LAST_PS, 0);
        dev_set_pixel_shader(dev, 0);
    }
    let tex = p8(rd::<u32>(p8(rd::<u32>(mesh, 8)), 0));
    if !tex.is_null() {
        set_texture(tex, 0);
        set_tss(0, 1, 4);
        set_tss(0, 2, 2);
        set_tss(0, 3, 0);
        set_tss(0, 4, 4);
        set_tss(0, 5, 2);
        set_tss(0, 6, 0);
    } else {
        set_texture(GPtr::NULL, 0);
        set_tss(0, 1, 3);
        set_tss(0, 3, 0);
        set_tss(0, 4, 3);
        set_tss(0, 6, 0);
    }
    let vs = rd::<u32>(mesh, 0x10);
    if gu(LAST_VS) != vs {
        su(LAST_VS, vs);
        dev_set_vertex_shader(device(), rd::<u32>(mesh, 0x10));
    }
    if rd::<u32>(header, 8) != 0 {
        flush_cache();
        dev_draw_indexed_primitive(
            device(),
            rd::<u32>(header, 0xC),
            rd::<u32>(mesh, 0x14),
            0,
            rd::<u32>(mesh, 0x18),
            rd::<u32>(mesh, 0x1C),
            rd::<u32>(mesh, 0x20),
        )
    } else {
        flush_cache();
        dev_draw_primitive(device(), rd::<u32>(header, 0xC), rd::<u32>(mesh, 0x14), rd::<u32>(mesh, 0x20))
    }
}

/// The cache entry `i` (0x14 bytes from 0xC926F8).
fn vs_entry(i: i32) -> GPtr<u8> {
    p8(VS_TABLE.wrapping_add((i as u32).wrapping_mul(0x14)))
}

/// `_rpD3D9GetVertexShader(desc, out)` (0x75EED0): the cached shader for `*desc`. The last hit
/// (0xC94BFC) is tried first, then a binary search over the sorted table (unsigned key compares; the
/// midpoint is `lo + n/2 + (n & 1) - 1` for `n = hi - lo + 1`, the last single candidate decides
/// between "insert before" and "insert after"). A hit refreshes the entry's frame stamp, copies its two
/// extra dwords to `out` and returns its shader. A miss creates the shader (`0x75F0B0(desc, out)`); with
/// 0x100 entries the entry with the smallest stamp (first one on ties) is overwritten (its old shader
/// deleted) in place, which breaks the table's order (original bug, kept); otherwise the tail is moved
/// up one entry (CRT `memmove`) and the new entry inserted at its sorted position and the count
/// incremented. Either way the new entry becomes the last hit and its +0xC / +0x10 take `out[0..2]`.
#[hook(addr = 0x75EED0, conv = cdecl, name = "_rpD3D9GetVertexShader", expect = "A1 FC 4B C9 00",
       unverified = "rw5-node-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the vertex-shader cache 0xC926F8..0xC94BFC and the caller's out pair")]
pub fn get_vertex_shader(desc: GPtr<u32>, out: GPtr<u32>) -> u32 {
    let key = desc.read();
    let hit = |e: GPtr<u8>| -> u32 {
        wr::<u32>(e, 4, frame_stamp());
        let last = p8(gu(VS_LAST));
        wr::<u32>(p8(out.addr() as u32), 0, rd::<u32>(last, 0xC));
        wr::<u32>(p8(out.addr() as u32), 4, rd::<u32>(last, 0x10));
        rd::<u32>(p8(gu(VS_LAST)), 8)
    };
    let last = p8(gu(VS_LAST));
    if rd::<u32>(last, 0) == key {
        return hit(last);
    }
    // binary search; `found` >= 0 is the index, else -1 - insertion point
    let count = gu(VS_COUNT) as i32;
    let mut lo: i32 = 0;
    let found: i32 = 'search: {
        if count <= 0 {
            break 'search -1 - lo;
        }
        let mut hi = count - 1;
        let mut n = count;
        loop {
            if n <= 1 {
                let k = rd::<u32>(vs_entry(lo), 0);
                if key == k {
                    break 'search lo;
                }
                break 'search if key < k { -1 - lo } else { -2 - lo };
            }
            let mid = (n & 1) + lo + n / 2 - 1;
            let k = rd::<u32>(vs_entry(mid), 0);
            if key == k {
                break 'search mid;
            }
            if key >= k {
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
            n = hi - lo + 1;
            if n <= 0 {
                break 'search -1 - lo;
            }
        }
    };
    if found >= 0 {
        let e = vs_entry(found);
        su(VS_LAST, e.addr() as u32);
        return hit(e);
    }
    let shader = create_vertex_shader(desc, out);
    let count = gu(VS_COUNT) as i32;
    if count >= 0x100 {
        let mut e = vs_entry(0);
        let mut c = vs_entry(1);
        let mut left = count - 1;
        while left > 0 {
            if rd::<u32>(c, 4) < rd::<u32>(e, 4) {
                e = c;
            }
            c = c.byte_add(0x14);
            left -= 1;
        }
        wr::<u32>(e, 0, desc.read());
        wr::<u32>(e, 4, frame_stamp());
        let old = rd::<u32>(e, 8);
        if old != 0 {
            delete_vertex_shader(p8(old));
        }
        wr::<u32>(e, 8, shader);
        wr::<u32>(e, 0xC, out.read());
        wr::<u32>(e, 0x10, rd::<u32>(p8(out.addr() as u32), 4));
        su(VS_LAST, e.addr() as u32);
        return shader;
    }
    let pos = -1 - found;
    if pos < count {
        let at = vs_entry(pos).addr() as u32;
        crt_memmove(at.wrapping_add(0x14), at, ((count - pos) as u32).wrapping_mul(0x14));
    }
    su(VS_LAST, vs_entry(pos).addr() as u32);
    wr::<u32>(p8(gu(VS_LAST)), 0, desc.read());
    wr::<u32>(p8(gu(VS_LAST)), 4, frame_stamp());
    wr::<u32>(p8(gu(VS_LAST)), 8, shader);
    wr::<u32>(p8(gu(VS_LAST)), 0xC, out.read());
    wr::<u32>(p8(gu(VS_LAST)), 0x10, rd::<u32>(p8(out.addr() as u32), 4));
    su(VS_COUNT, gu(VS_COUNT).wrapping_add(1));
    shader
}

/// `_rpD3DVertexShaderSetUVAnimMatrix(matrix, offsets)` (0x7619A0): the 2×4 constant block
/// `{m[0], m[0x10], m[0x20], m[0x30]; m[4], m[0x14], m[0x24], m[0x34]}` (right / up / at / pos x then y,
/// on the stack) uploaded with `SetVertexShaderConstantF(register = offsets byte +2, data, 2)`.
/// Returns the HRESULT.
#[hook(addr = 0x7619A0, conv = cdecl, name = "_rpD3DVertexShaderSetUVAnimMatrix", expect = "83 EC 20 8B 44 24 24",
       unverified = "rw5-node-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn vs_set_uv_anim_matrix(m: GPtr<u8>, offsets: GPtr<u8>) -> i32 {
    let mut c = [0u32; 8];
    for (k, off) in [0, 0x10, 0x20, 0x30, 4, 0x14, 0x24, 0x34].into_iter().enumerate() {
        c[k] = rd::<u32>(m, off);
    }
    dev_set_vs_constant_f(device(), rd::<u8>(offsets, 2) as u32, GPtr::from_raw(c.as_mut_ptr()).cast(), 2)
}

orig! {
    /// `IDirect3DDevice9::SetVertexShaderConstantF(start, data, count)` (+0x178)
    fn dev_set_vs_constant_f(dev: GPtr<u8>, start: u32, data: GPtr<u8>, count: u32) -> i32 = stdcall vslot 0x178 => unported;
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_NODE_DEF_ATOMIC_ALL_IN_ONE,
    HOOK_WORLD_SECTOR_ALL_IN_ONE_NODE,
    HOOK_VS_DEFAULT_BEGIN_CALLBACK,
    HOOK_VS_DEFAULT_MESH_RENDER_CALLBACK,
    HOOK_GET_VERTEX_SHADER,
    HOOK_VS_SET_UV_ANIM_MATRIX,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
