//! The hardware (vertex shader) render step of the D3D9 skin node: the shader descriptor, the shared constants, the
//! per-mesh material, fog and bone matrix constants and the pipeline's shader callbacks.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{f32_of, g, p8, rd, wr, x};
use crate::d3d9_skin_blend::MATRIX_BUFFER;
use crate::d3d9dev::{bind_instanced_header, dev_set_vs_constant};
use crate::d3d9state::device;

use crate::bindings::frame_get_ltm;
use crate::bindings::set_render_state;
use crate::bindings::vertex_alpha_enable;
orig! {
    /// `_rpD3D9VertexShaderSetActiveWorldMatrix`-like (0x764650)
    fn vs_set_active_world_matrix(m: GPtr<u8>) -> u32 = cdecl 0x764650 => crate::d3d9vsmat::vs_set_active_world_matrix;
    /// The composed world-view-projection matrix into `out` (0x7646E0)
    fn vs_get_composed_transform_matrix(out: GPtr<u8>) -> GPtr<u8> = cdecl 0x7646E0 => crate::d3d9vsmat::vs_get_composed_transform_matrix;
    /// The light colour constants for a descriptor, ambient and diffuse (0x761720)
    fn vs_update_lights_colors(p: GPtr<u8>, desc: GPtr<u8>, ambient: f32, diffuse: f32) -> i32 = cdecl 0x761720 => crate::d3d9vsmat::vs_update_lights_colors;
    /// The material colour constant (0x761820)
    fn vs_update_material_color(color: GPtr<u8>, dispatch: GPtr<u8>) -> i32 = cdecl 0x761820 => crate::d3d9vsmat::vs_update_material_color;
    /// The fog constants (0x7618B0)
    fn vs_update_fog_data(desc: GPtr<u8>, dispatch: GPtr<u8>) -> i32 = cdecl 0x7618B0 => crate::d3d9vsmat::vs_update_fog_data;
    /// The bone matrices of the vertex-shader path into `out` (0x7C78A0)
    fn skin_vs_matrix_update(out: GPtr<u8>, atomic: GPtr<u8>, skin: GPtr<u8>) = cdecl 0x7C78A0 => crate::skin::rp_d3d9_skin_vertex_shader_matrix_update;
    /// The constant registers a shader descriptor uses (0x75EDD0)
    fn get_num_constants_used(desc: GPtr<u8>) -> u32 = cdecl 0x75EDD0 => crate::d3d9misc::get_num_constants_used;
    /// The pipeline's shader callbacks: begin / end `(atomic, type, desc)`, the shared constants `(atomic, type,
    /// dst, desc)` returning the end of what it wrote, the mesh shader `(material, desc, dispatch)` and the mesh
    /// render `(header, instance, desc, dispatch)`
    fn call_vs_begin_end(f: u32, atomic: GPtr<u8>, ty: u32, desc: GPtr<u8>) = cdecl dyn => unported;
    fn call_vs_constants(f: u32, atomic: GPtr<u8>, ty: u32, dst: u32, desc: GPtr<u8>) -> u32 = cdecl dyn => unported;
    fn call_vs_mesh_shader(f: u32, material: GPtr<u8>, desc: GPtr<u8>, dispatch: GPtr<u8>) -> u32 = cdecl dyn => unported;
    fn call_vs_mesh_render(f: u32, header: GPtr<u8>, inst: GPtr<u8>, desc: GPtr<u8>, dispatch: GPtr<u8>) = cdecl dyn => unported;
}

/// The vertex shader constants: the composed transform (0xC970A0) and the shared constants after it (0xC970E0).
const VS_CONSTANTS: u32 = 0xC970A0;
const VS_SHARED_CONSTANTS: u32 = 0xC970E0;
/// The constant registers available to the skin shaders and the bone count above which the whole palette is not
/// uploaded at once.
const VS_MAX_CONSTANTS: usize = 0xC978EC;
const VS_MAX_BONES: usize = 0xC978E8;

/// Drops the next optional feature of a shader descriptor that still uses too many constants: byte 1's low nibble,
/// else byte 0's high nibble, else byte 0's low nibble (each counted down by one), else byte 3's low two bits.
/// False when none is left.
fn reduce_descriptor(d: &mut [u8; 4]) -> bool {
    if d[1] & 0xF != 0 {
        d[1] = (d[1] & 0xF0) | (d[1].wrapping_sub(1) & 0xF);
    } else if d[0] & 0xF0 != 0 {
        d[0] = (d[0] & 0xF0).wrapping_sub(1) ^ (!d[0] & 0xF);
    } else if d[0] & 0xF != 0 {
        d[0] = (d[0] & 0xF0) | (d[0].wrapping_sub(1) & 0xF);
    } else if d[3] & 3 != 0 {
        d[3] &= 0xFC;
    } else {
        return false;
    }
    true
}

/// Uploads runs of consecutive bones of `list` (`used` entries): each run's matrices (3 registers each, from the
/// matrix buffer at the run's first bone) at `base` plus three times the run's first list index (`by_index`) or the
/// run's first bone.
fn upload_bone_runs(base: u32, list: GPtr<u8>, used: u32, by_index: bool) {
    let buf = g::<u32>(MATRIX_BUFFER).read();
    let at = |i: u32| u32::from(rd::<u8>(list, i as isize));
    let mut s = 0u32;
    loop {
        let start = s;
        while s < used.wrapping_sub(1) && at(s) + 1 == at(s + 1) {
            s += 1;
        }
        s += 1;
        let first = at(start);
        let reg = base.wrapping_add(3 * if by_index { start } else { first });
        dev_set_vs_constant(device(), reg, buf.wrapping_add(first * 0x30), (s - start).wrapping_mul(3));
        if s >= used {
            break;
        }
    }
}

/// `_rpD3D9SkinRenderVS`-like (0x7C8060, cdecl `(callbacks, atomic, geometry, skin, hierarchy)`; name from the RW
/// symbols, low confidence): the hardware skin render. The begin callback fills the shader descriptor; clipping on
/// (render state 0x88), the descriptor's weight count (byte 2 bits 1..3) from the skin (+0x10), the world matrix and
/// the composed transform, the shared constants written by the constants callback from register 0 (which also fills the
/// descriptor's light counts), the bone matrices (0x7C78A0); the instanced header (the geometry's resource entry with
/// one morph target, else the atomic's) bound. Per mesh: vertex alpha, the mesh shader callback (when the descriptor
/// needs more constants than available with the skin's bone count, its features are dropped one by one, the shader
/// chosen again and the cached ambient reset to `1 - ambient`); the light colours when ambient or diffuse changed (as
/// ratios to the cached ones, which start at 1); the material colour when the dispatch's first byte or the colour
/// changed; the fog when its second byte changed; the bone matrices for the dispatch's base register (byte 4): with the
/// skin's split data (+0x34) the mesh's ranges of (+0x38) through the remap table (+0x30), else, when the base changed,
/// the whole palette or its runs of consecutive used bones (+8); then the mesh render callback and the instance's
/// shader cleared. Finally the end callback. The descriptor and the dispatch start as the caller's stack residue
/// (`peek`; the callbacks fill them), and the original keeps the colour pointer and the base register in the caller's
/// geometry argument slot.
#[hook(addr = 0x7C8060, conv = cdecl, name = "(missed by ghidra: cat7-call)", expect = "83 EC 3C 53 55 56 8B 74 24 50",
       peek = 0x38, unverified = "frontier2-c27", route = "L2", hazards = "heap-writes,stack-residue",
       note = "keeps the material colour pointer and the bone base register in the caller's geometry argument slot")]
pub fn skin_render_vertex_shader(cbs: GPtr<u8>, atomic: GPtr<u8>, geo: GPtr<u8>, skin: GPtr<u8>, hier: GPtr<u8>) {
    let residue = |off: i32| sa_core::abi::peek_stack(0x7C8060, off).unwrap_or(0);
    let mut cache = [0u8, 0xFF, 0xFF, 0xFF];
    let mut desc = residue(-0x38).to_le_bytes();
    let (mut ambient, mut diffuse) = (1.0f32, 1.0f32);
    let mut dispatch = [residue(-8), residue(-4)];
    let p = |v: &mut [u8; 4]| GPtr::<u8>::from_raw(v.as_mut_ptr());
    let pd = |v: &mut [u32; 2]| GPtr::<u32>::from_raw(v.as_mut_ptr()).cast::<u8>();
    call_vs_begin_end(rd(cbs, 0), atomic, 1, p(&mut desc));
    set_render_state(0x88, 1);
    desc[2] = (desc[2] & !0xE) | (rd::<u8>(skin, 0x10).wrapping_mul(2) & 0xE);
    vs_set_active_world_matrix(frame_get_ltm(p8(rd::<u32>(atomic, 4))));
    vs_get_composed_transform_matrix(p8(VS_CONSTANTS));
    let end = call_vs_constants(rd(cbs, 4), atomic, 1, VS_SHARED_CONSTANTS, p(&mut desc));
    dev_set_vs_constant(device(), 0, VS_CONSTANTS, end.wrapping_sub(VS_CONSTANTS) >> 4);
    skin_vs_matrix_update(p8(g::<u32>(MATRIX_BUFFER).read()), atomic, skin);
    let entry = if rd::<u32>(geo, 0x18) == 1 { rd::<u32>(geo, 0x58) } else { rd::<u32>(atomic, 0x14) };
    let hdr = p8(entry).byte_add(0x18);
    bind_instanced_header(hdr);
    let ranges = p8(rd::<u32>(skin, 0x38));
    let mut split = p8(rd::<u32>(skin, 0x34));
    let mut colour = rd::<u32>(p8(rd::<u32>(hdr, 0x48)), 4);
    let mut n = rd::<u32>(hdr, 4);
    let mut inst = hdr.byte_add(0x40);
    while n != 0 {
        let mat = p8(rd::<u32>(inst, 8));
        vertex_alpha_enable((rd::<u8>(mat, 7) != 0xFF || rd::<u32>(inst, 0xC) != 0) as u32);
        wr::<u32>(inst, 0x10, call_vs_mesh_shader(rd(cbs, 8), mat, p(&mut desc), pd(&mut dispatch)));
        let too_many = |d: &mut [u8; 4]| {
            get_num_constants_used(p(d)).wrapping_add(rd::<u32>(skin, 0x1C).wrapping_mul(3)) > g::<u32>(VS_MAX_CONSTANTS).read()
        };
        if too_many(&mut desc) {
            while reduce_descriptor(&mut desc) && too_many(&mut desc) {}
            wr::<u32>(inst, 0x10, call_vs_mesh_shader(rd(cbs, 8), mat, p(&mut desc), pd(&mut dispatch)));
            ambient = f32_of(x(1.0).fsub_m(rd::<f32>(mat, 0xC)));
        }
        if ambient != rd::<f32>(mat, 0xC) || diffuse != rd::<f32>(mat, 0x14) {
            let d = f32_of(x(rd::<f32>(mat, 0x14)).fdiv_m(diffuse));
            let a = f32_of(x(rd::<f32>(mat, 0xC)).fdiv_m(ambient));
            vs_update_lights_colors(p8(VS_SHARED_CONSTANTS), p(&mut desc), a, d);
            ambient = rd::<f32>(mat, 0xC);
            diffuse = rd::<f32>(mat, 0x14);
        }
        let disp = |d: &[u32; 2], i: usize| (d[i / 4] >> (8 * (i % 4))) as u8;
        if cache[2] != disp(&dispatch, 0) || colour != rd::<u32>(mat, 4) {
            cache[2] = disp(&dispatch, 0);
            colour = rd::<u32>(mat, 4);
            vs_update_material_color(mat.byte_add(4), pd(&mut dispatch));
        }
        if cache[3] != disp(&dispatch, 1) {
            cache[3] = disp(&dispatch, 1);
            vs_update_fog_data(p(&mut desc), pd(&mut dispatch));
        }
        let base = disp(&dispatch, 4);
        if !split.is_null() {
            let buf = g::<u32>(MATRIX_BUFFER).read();
            let (start, count) = (u32::from(rd::<u8>(split, 0)), u32::from(rd::<u8>(split, 1)));
            cache[1] = base;
            split = split.byte_add(2);
            let remap = p8(rd::<u32>(skin, 0x30));
            for r in 0..count {
                let pair = ranges.byte_add(2 * (start + r) as isize);
                let (mut bone, mut left) = (u32::from(rd::<u8>(pair, 0)), u32::from(rd::<u8>(pair, 1)));
                let mut m = buf.wrapping_add(bone * 0x30);
                loop {
                    let reg = u32::from(base).wrapping_add(3 * u32::from(rd::<u8>(remap, bone as isize)));
                    dev_set_vs_constant(device(), reg, m, 3);
                    bone = bone.wrapping_add(1);
                    m = m.wrapping_add(0x30);
                    left = left.wrapping_sub(1);
                    if left == 0 {
                        break;
                    }
                }
            }
        } else if cache[1] != base {
            cache[1] = base;
            let nodes = rd::<u32>(hier, 4);
            let used = rd::<u32>(skin, 4);
            let list = p8(rd::<u32>(skin, 8));
            if nodes > g::<u32>(VS_MAX_BONES).read() {
                upload_bone_runs(u32::from(base), list, used, true);
            } else if used < nodes {
                upload_bone_runs(u32::from(base), list, used, false);
            } else {
                dev_set_vs_constant(device(), u32::from(base), g::<u32>(MATRIX_BUFFER).read(), nodes.wrapping_mul(3));
            }
        }
        call_vs_mesh_render(rd(cbs, 0xC), hdr, inst, p(&mut desc), pd(&mut dispatch));
        wr::<u32>(inst, 0x10, 0);
        inst = inst.byte_add(0x24);
        n -= 1;
    }
    call_vs_begin_end(rd(cbs, 0x10), atomic, 1, p(&mut desc));
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_SKIN_RENDER_VERTEX_SHADER];
