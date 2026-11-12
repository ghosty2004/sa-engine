//! The phase B render-device seam on wgpu: [`WgpuDevice`] implements
//! [`sa_platform::gpu::D3d9Device`] (the `IDirect3DDevice9` subset of RenderWare's D3D9 driver).
//!
//! # Design
//! * **Back buffer.** The device renders into its own back buffer ([`COLOR_FORMAT`] colour plus
//!   a [`DEPTH_STENCIL_FORMAT`] depth/stencil texture). A windowed app shows it after
//!   [`D3d9Device::present`] with [`WgpuDevice::blit_to`] (a full-screen copy into e.g. the
//!   surface texture's view, scaled if the sizes differ, gamma-preserving on sRGB targets):
//!   `dev.present()?; let frame = surface.get_current_texture(); dev.blit_to(&view, format); frame.present();`.
//! * **Immediate mode.** D3D9 executes calls in order; wgpu writes buffers/textures at the next
//!   submit. Draws (and clears, drawn as full-viewport triangles) are recorded into a pending list
//!   with everything they need (pipeline, bind groups, a per-draw uniform block at a dynamic
//!   offset, vertex/index buffer references, viewport, scissor, stencil reference). The list is
//!   encoded as one render pass and submitted ("flushed") before anything that could change what
//!   a pending draw sees: updating a vertex/index buffer range a pending draw reads, updating a
//!   texture a pending draw samples, `set_render_target`, `reset`, `present`,
//!   `read_render_target`, `blit_to`. `DrawPrimitiveUP` data and generated indices go into a
//!   per-flush arena buffer.
//! * **Fixed function** is one WGSL uber-shader (`d3d9_common.wgsl` + `d3d9_ff_vs.wgsl` +
//!   `d3d9_ff_ps.wgsl`) driven by the per-draw uniform block; only the vertex input struct is
//!   generated per vertex layout. Pipelines are cached by program, vertex layout, topology, blend,
//!   colour write mask, depth, depth bias, stencil and cull mode.
//! * **Shaders** (docs/arch/gpu.md): `create_*_shader` decodes the D3D9 token stream
//!   ([`crate::sm`]) and translates it to WGSL once to reject what the translator lacks. A draw
//!   with a vertex and/or pixel shader builds a module from the translated `vs_body`/`ps_body`
//!   and generated entry points: vertex declaration elements feed the input register whose `dcl`
//!   has their usage, the vertex shader's outputs get D3D9's half-pixel offset and `oD0`/`oD1`
//!   saturation, and the pixel stage (shader or texture stages) is followed by fog and the alpha
//!   test as in D3D9. A missing half falls back to the fixed-function one (fixed-function vertices
//!   give a pixel shader texture coordinate sets 0 and 1 only; a vertex shader gives the
//!   fixed-function texture stages `oT0`/`oT1`). Float/int/bool constants (vertex and pixel) live
//!   in one uniform block (group 0, binding 1) pushed once per change; the shader path binds eight
//!   texture stages, 2D or cube per the pixel shader's declarations (ps_2_x) or the bound
//!   textures (ps_1_x).
//! * **Topologies.** Pipelines only use point/line/triangle lists: strips and fans (no wgpu fan,
//!   and strips with index buffers would need primitive-restart formats) are expanded to lists
//!   with generated `u32` indices, keeping D3D's winding (odd strip triangles swapped).
//! * **Textures.** Uncompressed formats are converted to RGBA8 on the CPU. DXT1/3/5 (and the
//!   premultiplied DXT2/4, treated as 3/5) are uploaded as BC1/2/3 when the device has
//!   `TEXTURE_COMPRESSION_BC` and the size is a multiple of 4, otherwise decoded with
//!   [`sa_rw::dxt`].
//!
//! # Implemented
//! Transforms (`v * World * View * Proj`, D3D row vectors), pre-transformed `XYZRHW` vertices, the
//! D3D9 half-pixel convention (pixel centres at integer coordinates), vertex colours (`D3DCOLOR`
//! swizzled from BGRA), all `D3DDECLTYPE`s that have a wgpu vertex format, texture stages 0 and 1
//! (`COLOROP`/`ALPHAOP` DISABLE…LERP except PREMODULATE/BUMPENVMAP*, args DIFFUSE, CURRENT,
//! TEXTURE, TFACTOR, SPECULAR with COMPLEMENT/ALPHAREPLICATE, `TEXCOORDINDEX` 0/1), alpha test,
//! fog (table and vertex, LINEAR/EXP/EXP2, `RANGEFOGENABLE`, fog from specular alpha when both
//! modes are NONE), lighting (global ambient, material with `COLORVERTEX` material sources, up to
//! 8 directional/point/spot lights), `SPECULARENABLE` (adds the vertex specular), blending (all
//! `D3DBLEND` factors wgpu has, `BLENDOP`, separate alpha, `BLENDFACTOR`), depth (`ZENABLE`,
//! `ZWRITEENABLE`, `ZFUNC`, `DEPTHBIAS`, `SLOPESCALEDEPTHBIAS`), stencil (incl. two-sided),
//! `CULLMODE`, `COLORWRITEENABLE`, scissor test, samplers (`ADDRESSU/V`, `MIN/MAG/MIPFILTER`,
//! `MAXANISOTROPY`, `MAXMIPLEVEL`), render-target textures, `Clear` on the viewport (and scissor).
//!
//! # Approximated
//! * Lighting runs in world space with normals transformed by the world matrix and always
//!   normalised (D3D: camera space, inverse transpose, normalised only with `NORMALIZENORMALS`);
//!   no specular lighting (the lit specular colour is black), no light ambient for out-of-range lights.
//! * Fog distance is `|z|` in view space (and `w` for pre-transformed vertices with table fog).
//!   A vertex without a specular colour has fog factor 1 (D3D's default specular is 0).
//! * A stage without a texture samples opaque white. `D3DFMT_A8` samples as (0, 0, 0, a).
//! * `BORDERCOLOR` maps to the nearest of transparent black / opaque black / opaque white (and
//!   BORDER falls back to CLAMP without `ADDRESS_MODE_CLAMP_TO_BORDER`); `MIPMAPLODBIAS`,
//!   `MIRRORONCE` (= MIRROR), texture transforms, `FILLMODE`, `SHADEMODE` (always Gouraud),
//!   point sizes, clip planes and `LASTPIXEL` are ignored.
//! * `set_render_target(Some(rt), None)` uses an internal depth buffer of the target's size
//!   (D3D would render without depth only if no depth surface were set).
//! * Sampling the texture that is the current render target binds white instead (undefined in D3D).
//!
//! # Not available
//! Shader instructions the translator lacks (`create_*_shader` returns [`D3dError::NotAvailable`]:
//! ps_1_x texbem/texm3x*/texreg2*/texdp3*, vs_3_0/ps_3_0, subroutines, predicates), cube render
//! targets, `P8`/`V8U8` formats, fixed-function texture stages 2–7.
//! On wasm32, [`D3d9Device::read_render_target`] returns `NotAvailable` (it needs a blocking map).

use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use bytemuck::{Pod, Zeroable};
use sa_platform::gpu::{
    Caps, D3d9Device, D3dError, D3dResult, IndexBufferId, Light, Material, PrimitiveType, ShaderId, TextureDesc, TextureId,
    VertexBufferId, VertexDeclId, VertexElement, Viewport, clear, decltype, declusage, fmt, rs, samp, ts, tss,
};
use sa_rw::dxt;

use crate::sm::{self, wgsl::ConstBlock};

/// Colour format of the back buffer and render-target textures (non-sRGB: D3D9 blends in gamma space).
pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// Depth/stencil format of every depth attachment.
pub const DEPTH_STENCIL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;

const COMMON_WGSL: &str = include_str!("d3d9_common.wgsl");
const FF_VS_WGSL: &str = include_str!("d3d9_ff_vs.wgsl");
const FF_PS_WGSL: &str = include_str!("d3d9_ff_ps.wgsl");
/// Entry points of the fixed-function pipeline.
const FF_ENTRIES_WGSL: &str = r#"
@vertex
fn vs_main(i: VIn) -> VOut {
    return ff_vertex(i);
}

@fragment
fn fs_main(i: VOut) -> @location(0) vec4f {
    return ff_fog_alpha(ff_pixel(i.c0, i.c1, i.t0, i.t1), i.fog);
}
"#;
/// Varyings of the shader path (a vertex or pixel shader is set): D3D9's `oD0`/`oD1`/`oT0..7`
/// and fog (x = distance for table fog, y = vertex fog factor).
const SV_WGSL: &str = r#"
struct SV {
    @builtin(position) pos: vec4f,
    @location(0) d0: vec4f,
    @location(1) d1: vec4f,
    @location(2) t0: vec4f,
    @location(3) t1: vec4f,
    @location(4) t2: vec4f,
    @location(5) t3: vec4f,
    @location(6) t4: vec4f,
    @location(7) t5: vec4f,
    @location(8) t6: vec4f,
    @location(9) t7: vec4f,
    @location(10) fog: vec2f,
}
"#;
const CLEAR_WGSL: &str = r#"
@vertex
fn vs_clear(@builtin(vertex_index) vi: u32) -> @builtin(position) vec4f {
    let p = vec2f(f32((vi << 1u) & 2u), f32(vi & 2u)) * 2.0 - vec2f(1.0);
    return vec4f(p, u.vpz.z, 1.0);
}

@fragment
fn fs_clear() -> @location(0) vec4f {
    return u.tfactor;
}
"#;
const BLIT_WGSL: &str = r#"
@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;

struct VO {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
}

@vertex
fn vs_blit(@builtin(vertex_index) vi: u32) -> VO {
    let t = vec2f(f32((vi << 1u) & 2u), f32(vi & 2u));
    var o: VO;
    o.pos = vec4f(t * 2.0 - vec2f(1.0), 0.0, 1.0);
    o.uv = vec2f(t.x, 1.0 - t.y);
    return o;
}

@fragment
fn fs_blit(i: VO) -> @location(0) vec4f {
    return textureSampleLevel(src, smp, i.uv, 0.0);
}

// sRGB targets encode on write: decode first so the stored bytes equal the back buffer's.
@fragment
fn fs_blit_srgb(i: VO) -> @location(0) vec4f {
    let c = textureSampleLevel(src, smp, i.uv, 0.0);
    let lo = c.rgb / 12.92;
    let hi = pow((c.rgb + vec3f(0.055)) / 1.055, vec3f(2.4));
    return vec4f(select(hi, lo, c.rgb <= vec3f(0.04045)), c.a);
}
"#;

// Render states without a named constant in sa_platform::gpu::rs.
const RS_TWOSIDEDSTENCILMODE: u32 = 185;
const RS_CCW_STENCILFAIL: u32 = 186;
const RS_CCW_STENCILZFAIL: u32 = 187;
const RS_CCW_STENCILPASS: u32 = 188;
const RS_CCW_STENCILFUNC: u32 = 189;
const RS_BLENDFACTOR: u32 = 193;

// Shader flag bits (d3d9_ff.wgsl F_*).
const F_PRETRANSFORMED: u32 = 1;
const F_LIGHTING: u32 = 2;
const F_COLORVERTEX: u32 = 4;
const F_HAS_C0: u32 = 8;
const F_HAS_C1: u32 = 16;
const F_FOG: u32 = 32;
const F_ALPHATEST: u32 = 64;
const F_HAS_NORMAL: u32 = 128;
const F_SPECULAR: u32 = 256;
const F_RANGEFOG: u32 = 512;

const MAX_STAGES: usize = 8;
const MAX_STREAMS: usize = 16;
const MAX_LIGHTS: usize = 8;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, PartialEq)]
struct LightU {
    diffuse: [f32; 4],
    ambient: [f32; 4],
    pos: [f32; 4],
    dir: [f32; 4],
    att: [f32; 4],
    spot: [f32; 4],
}

/// The per-draw uniform block (`struct U` in d3d9_common.wgsl).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, PartialEq)]
struct Uniforms {
    wvp: [[f32; 4]; 4],
    world: [[f32; 4]; 4],
    wv: [[f32; 4]; 4],
    viewport: [f32; 4],
    vpz: [f32; 4],
    flags: [u32; 4],
    stages: [[u32; 4]; 6],
    tfactor: [f32; 4],
    fog_color: [f32; 4],
    fog: [f32; 4],
    fog_mode: [u32; 4],
    ambient: [f32; 4],
    mat_diffuse: [f32; 4],
    mat_ambient: [f32; 4],
    mat_specular: [f32; 4],
    mat_emissive: [f32; 4],
    mat_src: [u32; 4],
    lights: [LightU; MAX_LIGHTS],
}

const IDENTITY: [[f32; 4]; 4] = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];

/// Row-major product `a * b` (D3D convention).
fn mul(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut o = [[0.0; 4]; 4];
    for (r, row) in o.iter_mut().enumerate() {
        for (c, v) in row.iter_mut().enumerate() {
            *v = (0..4).map(|k| a[r][k] * b[k][c]).sum();
        }
    }
    o
}

/// `D3DCOLOR` (0xAARRGGBB) → RGBA floats.
fn color(c: u32) -> [f32; 4] {
    let ch = |s: u32| ((c >> s) & 0xFF) as f32 / 255.0;
    [ch(16), ch(8), ch(0), ch(24)]
}

/// D3D9 device default render states.
fn default_render_states() -> Vec<u32> {
    let mut r = vec![0u32; rs::COUNT];
    let one = 1.0f32.to_bits();
    for (s, v) in [
        (rs::ZENABLE, 1),
        (rs::FILLMODE, 3),
        (rs::SHADEMODE, 2),
        (rs::ZWRITEENABLE, 1),
        (rs::LASTPIXEL, 1),
        (rs::SRCBLEND, 2),
        (rs::DESTBLEND, 1),
        (rs::CULLMODE, 3),
        (rs::ZFUNC, 4),
        (rs::ALPHAFUNC, 8),
        (rs::FOGEND, one),
        (rs::FOGDENSITY, one),
        (rs::STENCILFAIL, 1),
        (rs::STENCILZFAIL, 1),
        (rs::STENCILPASS, 1),
        (rs::STENCILFUNC, 8),
        (rs::STENCILMASK, 0xFFFF_FFFF),
        (rs::STENCILWRITEMASK, 0xFFFF_FFFF),
        (rs::TEXTUREFACTOR, 0xFFFF_FFFF),
        (rs::CLIPPING, 1),
        (rs::LIGHTING, 1),
        (rs::COLORVERTEX, 1),
        (rs::LOCALVIEWER, 1),
        (rs::DIFFUSEMATERIALSOURCE, 1),
        (rs::SPECULARMATERIALSOURCE, 2),
        (rs::POINTSIZE, one),
        (rs::COLORWRITEENABLE, 0xF),
        (rs::BLENDOP, 1),
        (RS_CCW_STENCILFAIL, 1),
        (RS_CCW_STENCILZFAIL, 1),
        (RS_CCW_STENCILPASS, 1),
        (RS_CCW_STENCILFUNC, 8),
        (RS_BLENDFACTOR, 0xFFFF_FFFF),
        (190, 0xF), // COLORWRITEENABLE1..3
        (191, 0xF),
        (192, 0xF),
        (rs::SRCBLENDALPHA, 2),
        (rs::DESTBLENDALPHA, 1),
        (rs::BLENDOPALPHA, 1),
    ] {
        r[s as usize] = v;
    }
    r
}

/// D3D9 device default texture stage states.
fn default_stage_states() -> [[u32; tss::COUNT]; MAX_STAGES] {
    let mut t = [[0u32; tss::COUNT]; MAX_STAGES];
    for (i, s) in t.iter_mut().enumerate() {
        s[tss::COLOROP as usize] = if i == 0 { 4 } else { 1 };
        s[tss::COLORARG1 as usize] = 2;
        s[tss::COLORARG2 as usize] = 1;
        s[tss::ALPHAOP as usize] = if i == 0 { 2 } else { 1 };
        s[tss::ALPHAARG1 as usize] = 2;
        s[tss::ALPHAARG2 as usize] = 1;
        s[tss::TEXCOORDINDEX as usize] = i as u32;
        s[tss::COLORARG0 as usize] = 1;
        s[tss::ALPHAARG0 as usize] = 1;
        s[tss::RESULTARG as usize] = 1;
    }
    t
}

fn default_sampler_states() -> [[u32; samp::COUNT]; MAX_STAGES] {
    let mut t = [[0u32; samp::COUNT]; MAX_STAGES];
    for s in t.iter_mut() {
        s[samp::ADDRESSU as usize] = 1;
        s[samp::ADDRESSV as usize] = 1;
        s[samp::ADDRESSW as usize] = 1;
        s[samp::MAGFILTER as usize] = 1;
        s[samp::MINFILTER as usize] = 1;
        s[samp::MAXANISOTROPY as usize] = 1;
    }
    t
}

/// `D3DDECLTYPE` → (wgpu format, WGSL input type, expansion to `vec4f` of `X`).
fn decl_format(t: u8) -> Option<(wgpu::VertexFormat, &'static str, &'static str)> {
    use wgpu::VertexFormat as V;
    Some(match t {
        decltype::FLOAT1 => (V::Float32, "f32", "vec4f(X, 0.0, 0.0, 1.0)"),
        decltype::FLOAT2 => (V::Float32x2, "vec2f", "vec4f(X, 0.0, 1.0)"),
        decltype::FLOAT3 => (V::Float32x3, "vec3f", "vec4f(X, 1.0)"),
        decltype::FLOAT4 => (V::Float32x4, "vec4f", "X"),
        decltype::D3DCOLOR => (V::Unorm8x4, "vec4f", "X.zyxw"),
        decltype::UBYTE4 => (V::Uint8x4, "vec4u", "vec4f(X)"),
        decltype::SHORT2 => (V::Sint16x2, "vec2i", "vec4f(vec2f(X), 0.0, 1.0)"),
        decltype::SHORT4 => (V::Sint16x4, "vec4i", "vec4f(X)"),
        decltype::UBYTE4N => (V::Unorm8x4, "vec4f", "X"),
        decltype::SHORT2N => (V::Snorm16x2, "vec2f", "vec4f(X, 0.0, 1.0)"),
        decltype::SHORT4N => (V::Snorm16x4, "vec4f", "X"),
        decltype::USHORT2N => (V::Unorm16x2, "vec2f", "vec4f(X, 0.0, 1.0)"),
        decltype::USHORT4N => (V::Unorm16x4, "vec4f", "X"),
        decltype::FLOAT16_2 => (V::Float16x2, "vec2f", "vec4f(X, 0.0, 1.0)"),
        decltype::FLOAT16_4 => (V::Float16x4, "vec4f", "X"),
        _ => return None,
    })
}

/// Shader location of a declaration element (0 position, 1 normal, 2/3 colours, 4/5 texcoords).
fn location(usage: u8, index: u8) -> Option<u32> {
    Some(match (usage, index) {
        (declusage::POSITION | declusage::POSITIONT, 0) => 0,
        (declusage::NORMAL, 0) => 1,
        (declusage::COLOR, 0) => 2,
        (declusage::COLOR, 1) => 3,
        (declusage::TEXCOORD, 0) => 4,
        (declusage::TEXCOORD, 1) => 5,
        _ => return None,
    })
}

/// The generated part of the vertex shader: `VIn` and `load_attrs`.
fn vertex_header(sig: &[(u32, u8)]) -> String {
    let mut s = String::from("struct VIn {\n");
    for &(loc, t) in sig {
        let (_, ty, _) = decl_format(t).unwrap_or((wgpu::VertexFormat::Float32x4, "vec4f", "X"));
        s += &format!("    @location({loc}) a{loc}: {ty},\n");
    }
    s += "}\n\nfn load_attrs(i: VIn) -> Attrs {\n    var a: Attrs;\n    a.pos = vec4f(0.0, 0.0, 0.0, 1.0);\n    \
          a.normal = vec3f(0.0);\n    a.c0 = vec4f(1.0);\n    a.c1 = vec4f(0.0);\n    a.t0 = vec4f(0.0, 0.0, 0.0, 1.0);\n    \
          a.t1 = vec4f(0.0, 0.0, 0.0, 1.0);\n";
    for &(loc, t) in sig {
        let (_, _, conv) = decl_format(t).unwrap_or((wgpu::VertexFormat::Float32x4, "vec4f", "X"));
        let e = conv.replace('X', &format!("i.a{loc}"));
        let field = ["pos", "normal", "c0", "c1", "t0", "t1"][loc as usize];
        if loc == 1 {
            s += &format!("    a.normal = ({e}).xyz;\n");
        } else {
            s += &format!("    a.{field} = {e};\n");
        }
    }
    s += "    return a;\n}\n";
    s
}

/// Vertex inputs of a translated vertex shader: `VIn` (location = input register) and
/// `load_regs` (registers without an element read (0, 0, 0, 1)).
fn shader_vertex_header(sig: &[(u32, u8)]) -> String {
    let mut s = String::from("struct VIn {\n");
    for &(loc, t) in sig {
        let (_, ty, _) = decl_format(t).unwrap_or((wgpu::VertexFormat::Float32x4, "vec4f", "X"));
        s += &format!("    @location({loc}) a{loc}: {ty},\n");
    }
    if sig.is_empty() {
        s += "    @builtin(vertex_index) unused: u32,\n";
    }
    s += "}\n\nfn load_regs(i: VIn) -> array<vec4f, 16> {\n    var v: array<vec4f, 16>;\n";
    s += "    for (var k = 0; k < 16; k++) {\n        v[k] = vec4f(0.0, 0.0, 0.0, 1.0);\n    }\n";
    for &(loc, t) in sig {
        let (_, _, conv) = decl_format(t).unwrap_or((wgpu::VertexFormat::Float32x4, "vec4f", "X"));
        s += &format!("    v[{loc}] = {};\n", conv.replace('X', &format!("i.a{loc}")));
    }
    s += "    return v;\n}\n";
    s
}

fn compare(v: u32) -> wgpu::CompareFunction {
    use wgpu::CompareFunction as C;
    match v {
        1 => C::Never,
        2 => C::Less,
        3 => C::Equal,
        4 => C::LessEqual,
        5 => C::Greater,
        6 => C::NotEqual,
        7 => C::GreaterEqual,
        _ => C::Always,
    }
}

fn blend_factor(v: u8) -> wgpu::BlendFactor {
    use wgpu::BlendFactor as B;
    match v {
        1 => B::Zero,
        3 => B::Src,
        4 => B::OneMinusSrc,
        5 => B::SrcAlpha,
        6 => B::OneMinusSrcAlpha,
        7 => B::DstAlpha,
        8 => B::OneMinusDstAlpha,
        9 => B::Dst,
        10 => B::OneMinusDst,
        11 => B::SrcAlphaSaturated,
        14 => B::Constant,
        15 => B::OneMinusConstant,
        _ => B::One,
    }
}

fn blend_op(v: u8) -> wgpu::BlendOperation {
    use wgpu::BlendOperation as O;
    match v {
        2 => O::Subtract,
        3 => O::ReverseSubtract,
        4 => O::Min,
        5 => O::Max,
        _ => O::Add,
    }
}

fn blend_component(src: u8, dst: u8, op: u8) -> wgpu::BlendComponent {
    let operation = blend_op(op);
    if matches!(operation, wgpu::BlendOperation::Min | wgpu::BlendOperation::Max) {
        // D3D ignores the factors for MIN/MAX; wgpu requires One.
        return wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation };
    }
    wgpu::BlendComponent { src_factor: blend_factor(src), dst_factor: blend_factor(dst), operation }
}

/// `D3DBLEND_BOTHSRCALPHA`/`BOTHINVSRCALPHA` set both factors from the source one.
fn fix_both(src: u32, dst: u32) -> (u8, u8) {
    match src {
        12 => (5, 6),
        13 => (6, 5),
        _ => (src as u8, dst as u8),
    }
}

fn stencil_op(v: u8) -> wgpu::StencilOperation {
    use wgpu::StencilOperation as S;
    match v {
        2 => S::Zero,
        3 => S::Replace,
        4 => S::IncrementClamp,
        5 => S::DecrementClamp,
        6 => S::Invert,
        7 => S::IncrementWrap,
        8 => S::DecrementWrap,
        _ => S::Keep,
    }
}

/// Mip levels of a full chain.
/// `Set*ShaderConstant*`: registers past the end are dropped (D3D9 fails the call; RW never does it).
fn set_regs<T: Copy>(regs: &mut [T], start: u32, data: &[T]) {
    for (k, v) in data.iter().enumerate() {
        if let Some(r) = regs.get_mut(start as usize + k) {
            *r = *v;
        }
    }
}

fn full_chain(w: u32, h: u32) -> u32 {
    32 - w.max(h).max(1).leading_zeros()
}

fn expand5(v: u16) -> u8 {
    let v = (v & 31) as u8;
    (v << 3) | (v >> 2)
}

fn expand6(v: u16) -> u8 {
    let v = (v & 63) as u8;
    (v << 2) | (v >> 4)
}

fn expand4(v: u16) -> u8 {
    let v = (v & 15) as u8;
    (v << 4) | v
}

/// Converts `w`×`h` texels of an uncompressed D3D format (rows `pitch` bytes apart) to RGBA8.
fn convert_to_rgba8(format: u32, data: &[u8], pitch: usize, w: usize, h: usize) -> D3dResult<Vec<u8>> {
    let bpp = fmt::bytes_per_pixel(format).ok_or_else(|| D3dError::NotAvailable(format!("texture format {format}")))? as usize;
    if h > 0 && data.len() < (h - 1) * pitch + w * bpp {
        return Err(D3dError::InvalidCall(format!("texture data too short: {} bytes for {w}x{h}, pitch {pitch}", data.len())));
    }
    let mut out = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        let row = &data[y * pitch..y * pitch + w * bpp];
        for px in row.chunks_exact(bpp) {
            let u16v = || u16::from_le_bytes([px[0], px[1]]);
            let rgba = match format {
                fmt::A8R8G8B8 => [px[2], px[1], px[0], px[3]],
                fmt::X8R8G8B8 => [px[2], px[1], px[0], 255],
                fmt::A8B8G8R8 => [px[0], px[1], px[2], px[3]],
                fmt::R8G8B8 => [px[2], px[1], px[0], 255],
                fmt::R5G6B5 => {
                    let v = u16v();
                    [expand5(v >> 11), expand6(v >> 5), expand5(v), 255]
                }
                fmt::X1R5G5B5 | fmt::A1R5G5B5 => {
                    let v = u16v();
                    let a = if format == fmt::X1R5G5B5 || v & 0x8000 != 0 { 255 } else { 0 };
                    [expand5(v >> 10), expand5(v >> 5), expand5(v), a]
                }
                fmt::A4R4G4B4 | fmt::X4R4G4B4 => {
                    let v = u16v();
                    let a = if format == fmt::X4R4G4B4 { 255 } else { expand4(v >> 12) };
                    [expand4(v >> 8), expand4(v >> 4), expand4(v), a]
                }
                fmt::L8 => [px[0], px[0], px[0], 255],
                fmt::A8 => [0, 0, 0, px[0]],
                fmt::A8L8 => [px[0], px[0], px[0], px[1]],
                _ => return Err(D3dError::NotAvailable(format!("texture format {format}"))),
            };
            out.extend_from_slice(&rgba);
        }
    }
    Ok(out)
}

/// How a texture's data is stored on the GPU.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Store {
    /// Converted to RGBA8 (all uncompressed formats, render targets).
    Rgba8,
    /// Native BC with this many bytes per block.
    Bc(u32),
    /// DXT decoded to RGBA8 on the CPU.
    Dxt(dxt::Kind),
    /// Depth/stencil surface (not sampleable).
    Depth,
}

struct Tex {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    desc: TextureDesc,
    levels: u32,
    store: Store,
}

struct GpuBuf {
    buf: wgpu::Buffer,
    cpu: Vec<u8>,
    /// `fmt::INDEX16`/`INDEX32` for index buffers.
    index_format: u32,
}

#[derive(Clone, Copy, Default)]
struct Stream {
    vb: Option<VertexBufferId>,
    offset: u32,
    stride: u32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Input {
    Fvf(u32),
    Decl(VertexDeclId),
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct StreamKey {
    stream: u32,
    stride: u32,
    /// (offset, D3DDECLTYPE, shader location)
    attrs: Vec<(u32, u8, u32)>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct StencilKey {
    /// fail, zfail, pass, func (D3D values)
    front: [u8; 4],
    back: [u8; 4],
    read: u8,
    write: u8,
}

/// The programmable stages of a draw: shader ids (0 = fixed function) and, for the pixel shader,
/// which of stages 0..7 sample cube textures.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
struct ProgKey {
    vs: ShaderId,
    ps: ShaderId,
    cube: u8,
}

/// Number of texture stages the shader path binds.
const SHADER_STAGES: usize = 8;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
struct PipeKey {
    prog: ProgKey,
    layout: Vec<StreamKey>,
    /// 0 points, 1 lines, 2 triangles
    topology: u8,
    /// src, dst, op, src alpha, dst alpha, op alpha
    blend: Option<[u8; 6]>,
    write_mask: u8,
    depth_cmp: u8,
    depth_write: bool,
    bias: i32,
    slope_bits: u32,
    stencil: Option<StencilKey>,
    cull: u8,
}

/// Sampler key: address U, V, mag, min, mip, max anisotropy, max mip level, border class.
type SampKey = [u32; 8];
type BindKey = (TextureId, SampKey, TextureId, SampKey);
type ShBindKey = ([TextureId; SHADER_STAGES], [SampKey; SHADER_STAGES], u8);

#[derive(Clone)]
enum BufSrc {
    Buf(wgpu::Buffer),
    Arena,
}

enum DrawKind {
    Plain { first: u32, count: u32 },
    Indexed { first: u32, count: u32, base: i32 },
}

struct DrawCmd {
    pipeline: wgpu::RenderPipeline,
    textures: wgpu::BindGroup,
    uniform_offset: u32,
    const_offset: u32,
    vertex_buffers: Vec<(BufSrc, u64)>,
    index: Option<(BufSrc, u64, wgpu::IndexFormat)>,
    kind: DrawKind,
    viewport: [f32; 6],
    scissor: [u32; 4],
    stencil_ref: u32,
    blend_constant: wgpu::Color,
}

#[derive(Default)]
struct Pending {
    draws: Vec<DrawCmd>,
    arena: Vec<u8>,
    uniforms: Vec<u8>,
    last_uniform: Option<(Uniforms, u32)>,
    /// Shader constant blocks (`ConstBlock`), one per change.
    consts: Vec<u8>,
    last_consts: Option<u32>,
    vb_ranges: HashMap<VertexBufferId, (u64, u64)>,
    ib_ranges: HashMap<IndexBufferId, (u64, u64)>,
    textures: HashSet<TextureId>,
}

struct Target {
    color_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    size: (u32, u32),
    color_id: Option<TextureId>,
    /// The target's D3D format has no alpha channel (X8R8G8B8, R5G6B5, ...): alpha writes are masked
    /// and clears store alpha 1, so the stored alpha is 1 as D3D9 samples and blends it.
    opaque: bool,
}

/// A D3D colour format without an alpha channel: D3D9 reads its alpha as 1 (sampling and the
/// DESTALPHA blend factors) and ignores alpha writes.
fn format_is_opaque(format: u32) -> bool {
    matches!(format, fmt::X8R8G8B8 | fmt::R8G8B8 | fmt::R5G6B5 | fmt::X1R5G5B5 | fmt::X4R4G4B4)
}

/// Where a draw's vertices/indices come from.
enum DrawSrc<'a> {
    Stream { start_vertex: u32 },
    Indexed { base_vertex: i32, min_index: u32, num_vertices: u32, start_index: u32 },
    Up { vertices: &'a [u8], stride: u32 },
    UpIndexed { min_index: u32, num_vertices: u32, indices: &'a [u8], index_format: u32, vertices: &'a [u8], stride: u32 },
}

/// Counters (cumulative).
#[derive(Debug, Clone, Copy, Default)]
pub struct D3d9Stats {
    pub draws: u64,
    pub flushes: u64,
    pub pipelines: u64,
}

/// [`D3d9Device`] on wgpu. See the module documentation.
pub struct WgpuDevice {
    device: wgpu::Device,
    queue: wgpu::Queue,
    bc: bool,
    border: bool,
    size: (u32, u32),
    back: wgpu::Texture,
    back_view: wgpu::TextureView,
    target: Target,
    depth_by_size: HashMap<(u32, u32), wgpu::TextureView>,

    next_id: u32,
    textures: HashMap<TextureId, Tex>,
    vbs: HashMap<VertexBufferId, GpuBuf>,
    ibs: HashMap<IndexBufferId, GpuBuf>,
    decls: HashMap<VertexDeclId, Vec<VertexElement>>,

    rs: Vec<u32>,
    tss: [[u32; tss::COUNT]; MAX_STAGES],
    samp: [[u32; samp::COUNT]; MAX_STAGES],
    stage_tex: [Option<TextureId>; MAX_STAGES],
    world: [[f32; 4]; 4],
    view: [[f32; 4]; 4],
    proj: [[f32; 4]; 4],
    material: Material,
    lights: HashMap<u32, Light>,
    enabled_lights: BTreeSet<u32>,
    viewport: Viewport,
    scissor: (u32, u32, u32, u32),
    input: Option<Input>,
    streams: [Stream; MAX_STREAMS],
    indices: Option<IndexBufferId>,

    fvf_cache: HashMap<u32, Vec<VertexElement>>,
    modules: HashMap<Vec<(u32, u8)>, wgpu::ShaderModule>,
    program_modules: HashMap<(Vec<(u32, u8)>, ProgKey), wgpu::ShaderModule>,
    shaders: HashMap<ShaderId, sm::Shader>,
    vs: Option<ShaderId>,
    ps: Option<ShaderId>,
    consts: ConstBlock,
    const_stride: u32,
    sh_layouts: HashMap<u8, (wgpu::BindGroupLayout, wgpu::PipelineLayout)>,
    sh_bind_groups: HashMap<ShBindKey, wgpu::BindGroup>,
    white_cube: wgpu::TextureView,
    pipelines: HashMap<PipeKey, wgpu::RenderPipeline>,
    clear_pipelines: HashMap<(bool, bool, bool), wgpu::RenderPipeline>,
    clear_module: wgpu::ShaderModule,
    uniform_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    samplers: HashMap<SampKey, wgpu::Sampler>,
    bind_groups: HashMap<BindKey, wgpu::BindGroup>,
    white: wgpu::TextureView,
    uniform_stride: u32,
    uniform_buf: Option<(wgpu::Buffer, wgpu::Buffer, wgpu::BindGroup)>,
    arena_buf: Option<wgpu::Buffer>,
    pending: Pending,

    blit_module: wgpu::ShaderModule,
    blit_layout: wgpu::BindGroupLayout,
    blit_pipeline_layout: wgpu::PipelineLayout,
    blit_sampler: wgpu::Sampler,
    blit_pipelines: HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    /// The back buffer's D3D format has no alpha channel ([`WgpuDevice::set_back_buffer_format`]).
    back_opaque: bool,
    /// Writes alpha 1 into a rectangle of a colour target (the blit shader on the white texture, alpha
    /// writes only): `StretchRect` into a target without alpha.
    alpha_one_pipeline: Option<wgpu::RenderPipeline>,

    errors: Arc<Mutex<Vec<String>>>,
    pub stats: D3d9Stats,
}

fn color_texture(device: &wgpu::Device, w: u32, h: u32, label: &str) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: COLOR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn depth_texture(device: &wgpu::Device, w: u32, h: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("d3d9 depth"),
        size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_STENCIL_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}

/// Minimal executor for wgpu's futures (they complete without an external reactor natively).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn block_on<F: std::future::Future>(f: F) -> F::Output {
    use std::task::{Context, Poll, Wake, Waker};
    struct W(std::thread::Thread);
    impl Wake for W {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(W(std::thread::current())));
    let mut cx = Context::from_waker(&waker);
    let mut f = std::pin::pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::park();
    }
}

impl WgpuDevice {
    /// Device features this backend uses when the adapter offers them (request them with the device).
    pub fn optional_features() -> wgpu::Features {
        wgpu::Features::TEXTURE_COMPRESSION_BC | wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER
    }

    /// A device rendering into its own `width`×`height` back buffer. Installs an uncaptured-error
    /// handler on `device` that records wgpu validation errors (see [`WgpuDevice::take_errors`])
    /// instead of panicking.
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, width: u32, height: u32) -> WgpuDevice {
        let errors = Arc::new(Mutex::new(Vec::new()));
        {
            let errors = errors.clone();
            device.on_uncaptured_error(Arc::new(move |e: wgpu::Error| {
                let msg = e.to_string();
                eprintln!("sa-rw-wgpu d3d9: wgpu error: {msg}");
                if let Ok(mut v) = errors.lock() {
                    v.push(msg);
                }
            }));
        }
        let bc = device.features().contains(wgpu::Features::TEXTURE_COMPRESSION_BC);
        let border = device.features().contains(wgpu::Features::ADDRESS_MODE_CLAMP_TO_BORDER);
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("d3d9 uniforms"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(size_of::<Uniforms>() as u64),
                    },
                    count: None,
                },
                // shader constant registers (`sm::wgsl::PRELUDE`)
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(size_of::<ConstBlock>() as u64),
                    },
                    count: None,
                },
            ],
        });
        let tex_entry = |binding: u32| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let samp_entry = |binding: u32| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("d3d9 textures"),
            entries: &[tex_entry(0), samp_entry(1), tex_entry(2), samp_entry(3)],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("d3d9"),
            bind_group_layouts: &[Some(&uniform_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        let clear_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("d3d9 clear"),
            source: wgpu::ShaderSource::Wgsl(format!("{COMMON_WGSL}\n{CLEAR_WGSL}").into()),
        });
        let blit_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("d3d9 blit"),
            source: wgpu::ShaderSource::Wgsl(BLIT_WGSL.into()),
        });
        let blit_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("d3d9 blit"),
            entries: &[tex_entry(0), samp_entry(1)],
        });
        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("d3d9 blit"),
            bind_group_layouts: &[Some(&blit_layout)],
            immediate_size: 0,
        });
        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("d3d9 blit"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let white_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("d3d9 white"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &white_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &[255u8; 4],
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4), rows_per_image: Some(1) },
            wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        );
        let align = device.limits().min_uniform_buffer_offset_alignment.max(16);
        let uniform_stride = (size_of::<Uniforms>() as u32).div_ceil(align) * align;
        let const_stride = (size_of::<ConstBlock>() as u32).div_ceil(align) * align;
        let white_cube_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("d3d9 white cube"),
            size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 6 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &white_cube_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &[255u8; 24],
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4), rows_per_image: Some(1) },
            wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 6 },
        );
        let white_cube = white_cube_tex.create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::Cube), ..Default::default() });
        let (width, height) = (width.max(1), height.max(1));
        let back = color_texture(&device, width, height, "d3d9 back buffer");
        let back_view = back.create_view(&Default::default());
        let back_depth = depth_texture(&device, width, height).create_view(&Default::default());
        let mut depth_by_size = HashMap::new();
        depth_by_size.insert((width, height), back_depth.clone());
        let target = Target { color_view: back_view.clone(), depth_view: back_depth, size: (width, height), color_id: None, opaque: false };
        WgpuDevice {
            white: white_tex.create_view(&Default::default()),
            device,
            queue,
            bc,
            border,
            size: (width, height),
            back,
            back_view,
            back_opaque: false,
            alpha_one_pipeline: None,
            target,
            depth_by_size,
            next_id: 1,
            textures: HashMap::new(),
            vbs: HashMap::new(),
            ibs: HashMap::new(),
            decls: HashMap::new(),
            rs: default_render_states(),
            tss: default_stage_states(),
            samp: default_sampler_states(),
            stage_tex: [None; MAX_STAGES],
            world: IDENTITY,
            view: IDENTITY,
            proj: IDENTITY,
            material: Material::default(),
            lights: HashMap::new(),
            enabled_lights: BTreeSet::new(),
            viewport: Viewport { x: 0, y: 0, width, height, min_z: 0.0, max_z: 1.0 },
            scissor: (0, 0, width, height),
            input: None,
            streams: [Stream::default(); MAX_STREAMS],
            indices: None,
            fvf_cache: HashMap::new(),
            modules: HashMap::new(),
            program_modules: HashMap::new(),
            shaders: HashMap::new(),
            vs: None,
            ps: None,
            consts: ConstBlock::default(),
            const_stride,
            sh_layouts: HashMap::new(),
            sh_bind_groups: HashMap::new(),
            white_cube,
            pipelines: HashMap::new(),
            clear_pipelines: HashMap::new(),
            clear_module,
            uniform_layout,
            texture_layout,
            pipeline_layout,
            samplers: HashMap::new(),
            bind_groups: HashMap::new(),
            uniform_stride,
            uniform_buf: None,
            arena_buf: None,
            pending: Pending::default(),
            blit_module,
            blit_layout,
            blit_pipeline_layout,
            blit_sampler,
            blit_pipelines: HashMap::new(),
            errors,
            stats: D3d9Stats::default(),
        }
    }

    /// A device on the default adapter without a window (tests, screenshots). `None` when no
    /// adapter or device is available.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_headless(width: u32, height: u32) -> Option<WgpuDevice> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }))
        .ok()?;
        let features = adapter.features() & Self::optional_features();
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("sa-rw-wgpu d3d9"),
            required_features: features,
            required_limits: wgpu::Limits::default(),
            experimental_features: Default::default(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: Default::default(),
        }))
        .ok()?;
        Some(WgpuDevice::new(device, queue, width, height))
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// The back buffer ([`COLOR_FORMAT`], `RENDER_ATTACHMENT | TEXTURE_BINDING | COPY_SRC | COPY_DST`).
    pub fn back_buffer(&self) -> &wgpu::Texture {
        &self.back
    }

    /// Whether DXT textures are uploaded as BC (no software decode).
    pub fn uses_bc(&self) -> bool {
        self.bc
    }

    /// wgpu validation errors recorded since the last call.
    pub fn take_errors(&self) -> Vec<String> {
        self.errors.lock().map(|mut v| std::mem::take(&mut *v)).unwrap_or_default()
    }

    /// Submits the pending draws (also done implicitly whenever D3D9 ordering requires it).
    /// The D3D format of the back buffer (`D3DPRESENT_PARAMETERS::BackBufferFormat`): without an
    /// alpha channel (X8R8G8B8, R5G6B5, ...) the back buffer's stored alpha stays 1, as D3D9 reads
    /// it (`StretchRect` into render-target textures, the DESTALPHA blend factors). Default: alpha kept.
    pub fn set_back_buffer_format(&mut self, format: u32) {
        self.flush();
        self.back_opaque = format_is_opaque(format);
        if self.back_opaque {
            let v = self.back_view.clone();
            self.clear_to_opaque_black(&v);
        }
        if self.target.color_id.is_none() {
            self.target.opaque = self.back_opaque;
        }
    }

    /// Initialises a colour target to (0, 0, 0, 1) (a new target of a format without alpha).
    fn clear_to_opaque_black(&self, view: &wgpu::TextureView) {
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("d3d9 opaque init") });
        enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("d3d9 opaque init"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        self.queue.submit([enc.finish()]);
    }

    /// Sets the alpha of `rect` (x, y, w, h) of a colour target to 1, keeping its colour.
    fn set_alpha_one(&mut self, view: &wgpu::TextureView, rect: (u32, u32, u32, u32)) {
        let pipeline = self
            .alpha_one_pipeline
            .get_or_insert_with(|| {
                self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("d3d9 alpha one"),
                    layout: Some(&self.blit_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &self.blit_module,
                        entry_point: Some("vs_blit"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &self.blit_module,
                        entry_point: Some("fs_blit"),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState { format: COLOR_FORMAT, blend: None, write_mask: wgpu::ColorWrites::ALPHA })],
                    }),
                    multiview_mask: None,
                    cache: None,
                })
            })
            .clone();
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("d3d9 alpha one"),
            layout: &self.blit_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&self.white) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.blit_sampler) },
            ],
        });
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("d3d9 alpha one") });
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("d3d9 alpha one"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.set_scissor_rect(rect.0, rect.1, rect.2, rect.3);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([enc.finish()]);
    }

    pub fn flush(&mut self) {
        let p = std::mem::take(&mut self.pending);
        if p.draws.is_empty() {
            return;
        }
        self.stats.flushes += 1;
        let need_u = p.uniforms.len() as u64;
        let need_c = p.consts.len() as u64;
        if self.uniform_buf.as_ref().is_none_or(|(b, c, _)| b.size() < need_u || c.size() < need_c) {
            let (old_u, old_c) = self.uniform_buf.as_ref().map(|(b, c, _)| (b.size(), c.size())).unwrap_or((0, 0));
            let size = need_u.next_power_of_two().max(64 * 1024).max(old_u);
            let csize = need_c.next_power_of_two().max(64 * 1024).max(old_c);
            let mk = |size: u64, label: &str| {
                self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            };
            let buf = mk(size, "d3d9 uniforms");
            let cbuf = mk(csize, "d3d9 shader constants");
            let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("d3d9 uniforms"),
                layout: &self.uniform_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &buf,
                            offset: 0,
                            size: wgpu::BufferSize::new(size_of::<Uniforms>() as u64),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &cbuf,
                            offset: 0,
                            size: wgpu::BufferSize::new(size_of::<ConstBlock>() as u64),
                        }),
                    },
                ],
            });
            self.uniform_buf = Some((buf, cbuf, group));
        }
        let Some((ubuf, cbuf, ugroup)) = self.uniform_buf.clone() else { return };
        self.queue.write_buffer(&ubuf, 0, &p.uniforms);
        if !p.consts.is_empty() {
            self.queue.write_buffer(&cbuf, 0, &p.consts);
        }
        let mut arena = p.arena;
        arena.resize(arena.len().div_ceil(4) * 4, 0);
        if !arena.is_empty() {
            let need = arena.len() as u64;
            if self.arena_buf.as_ref().is_none_or(|b| b.size() < need) {
                self.arena_buf = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("d3d9 arena"),
                    size: need.next_power_of_two().max(64 * 1024),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
            }
            if let Some(b) = &self.arena_buf {
                self.queue.write_buffer(b, 0, &arena);
            }
        }
        let arena_buf = self.arena_buf.clone();
        let resolve = |s: &BufSrc| -> Option<wgpu::Buffer> {
            match s {
                BufSrc::Buf(b) => Some(b.clone()),
                BufSrc::Arena => arena_buf.clone(),
            }
        };
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("d3d9") });
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("d3d9"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.target.color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.target.depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store }),
                    stencil_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store }),
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            for d in &p.draws {
                pass.set_pipeline(&d.pipeline);
                pass.set_bind_group(0, &ugroup, &[d.uniform_offset, d.const_offset]);
                pass.set_bind_group(1, &d.textures, &[]);
                for (slot, (src, off)) in d.vertex_buffers.iter().enumerate() {
                    if let Some(b) = resolve(src) {
                        pass.set_vertex_buffer(slot as u32, b.slice(*off..));
                    }
                }
                if let Some((src, off, f)) = &d.index
                    && let Some(b) = resolve(src)
                {
                    pass.set_index_buffer(b.slice(*off..), *f);
                }
                let v = d.viewport;
                pass.set_viewport(v[0], v[1], v[2], v[3], v[4], v[5]);
                pass.set_scissor_rect(d.scissor[0], d.scissor[1], d.scissor[2], d.scissor[3]);
                pass.set_stencil_reference(d.stencil_ref);
                pass.set_blend_constant(d.blend_constant);
                match d.kind {
                    DrawKind::Plain { first, count } => pass.draw(first..first + count, 0..1),
                    DrawKind::Indexed { first, count, base } => pass.draw_indexed(first..first + count, base, 0..1),
                }
            }
        }
        self.queue.submit([enc.finish()]);
    }

    /// Draws the back buffer into `target` (e.g. a surface texture's view) of format `format`,
    /// stretched to its size. Call after [`D3d9Device::present`].
    pub fn blit_to(&mut self, target: &wgpu::TextureView, format: wgpu::TextureFormat) {
        self.flush();
        let pipeline = self
            .blit_pipelines
            .entry(format)
            .or_insert_with(|| {
                self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("d3d9 blit"),
                    layout: Some(&self.blit_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &self.blit_module,
                        entry_point: Some("vs_blit"),
                        compilation_options: Default::default(),
                        buffers: &[],
                    },
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: Default::default(),
                    fragment: Some(wgpu::FragmentState {
                        module: &self.blit_module,
                        entry_point: Some(if format.is_srgb() { "fs_blit_srgb" } else { "fs_blit" }),
                        compilation_options: Default::default(),
                        targets: &[Some(format.into())],
                    }),
                    multiview_mask: None,
                    cache: None,
                })
            })
            .clone();
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("d3d9 blit"),
            layout: &self.blit_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&self.back_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.blit_sampler) },
            ],
        });
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("d3d9 blit") });
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("d3d9 blit"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        self.queue.submit([enc.finish()]);
    }

    fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        id
    }

    fn r(&self, state: u32) -> u32 {
        self.rs.get(state as usize).copied().unwrap_or(0)
    }

    fn rf(&self, state: u32) -> f32 {
        f32::from_bits(self.r(state))
    }

    fn arena_push(&mut self, bytes: &[u8]) -> u64 {
        let a = &mut self.pending.arena;
        a.resize(a.len().div_ceil(16) * 16, 0);
        let off = a.len() as u64;
        a.extend_from_slice(bytes);
        off
    }

    fn push_uniforms(&mut self, u: &Uniforms) -> u32 {
        if let Some((last, off)) = &self.pending.last_uniform
            && last == u
        {
            return *off;
        }
        let off = self.pending.uniforms.len() as u32;
        self.pending.uniforms.extend_from_slice(bytemuck::bytes_of(u));
        self.pending.uniforms.resize((off + self.uniform_stride) as usize, 0);
        self.pending.last_uniform = Some((*u, off));
        off
    }

    /// The current shader constants in the pending constant arena (pushed once per change).
    fn push_consts(&mut self) -> u32 {
        if let Some(off) = self.pending.last_consts {
            return off;
        }
        let off = self.pending.consts.len() as u32;
        self.pending.consts.extend_from_slice(bytemuck::bytes_of(&self.consts));
        self.pending.consts.resize((off + self.const_stride) as usize, 0);
        self.pending.last_consts = Some(off);
        off
    }

    /// Vertex buffer ranges read by pending draws overlap `[start, end)`?
    fn vb_pending(&self, vb: VertexBufferId, start: u64, end: u64) -> bool {
        self.pending.vb_ranges.get(&vb).is_some_and(|&(s, e)| s < end && start < e)
    }

    fn ib_pending(&self, ib: IndexBufferId, start: u64, end: u64) -> bool {
        self.pending.ib_ranges.get(&ib).is_some_and(|&(s, e)| s < end && start < e)
    }

    fn note_range(map: &mut HashMap<u32, (u64, u64)>, id: u32, start: u64, end: u64) {
        let e = map.entry(id).or_insert((start, end));
        e.0 = e.0.min(start);
        e.1 = e.1.max(end);
    }

    fn create_buffer(&mut self, size: u32, usage: wgpu::BufferUsages, index_format: u32) -> D3dResult<u32> {
        if size == 0 {
            return Err(D3dError::InvalidCall("zero-sized buffer".into()));
        }
        let padded = (size as u64).div_ceil(4) * 4;
        if padded > self.device.limits().max_buffer_size {
            return Err(D3dError::OutOfMemory);
        }
        let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: padded,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let id = self.alloc_id();
        let b = GpuBuf { buf, cpu: vec![0; size as usize], index_format };
        if usage.contains(wgpu::BufferUsages::INDEX) {
            self.ibs.insert(id, b);
        } else {
            self.vbs.insert(id, b);
        }
        Ok(id)
    }

    /// Writes `data` at `offset` into the CPU copy and the GPU buffer (4-byte aligned window).
    fn write_buf(queue: &wgpu::Queue, b: &mut GpuBuf, offset: u32, data: &[u8]) -> D3dResult<()> {
        let start = offset as usize;
        let end = start + data.len();
        if end > b.cpu.len() {
            return Err(D3dError::InvalidCall(format!("buffer write {start}..{end} past size {}", b.cpu.len())));
        }
        if data.is_empty() {
            return Ok(());
        }
        b.cpu[start..end].copy_from_slice(data);
        let a0 = start / 4 * 4;
        let a1 = end.div_ceil(4) * 4;
        let mut window = vec![0u8; a1 - a0];
        let have = b.cpu.len().min(a1) - a0;
        window[..have].copy_from_slice(&b.cpu[a0..a0 + have]);
        queue.write_buffer(&b.buf, a0 as u64, &window);
        Ok(())
    }

    fn input_elements(&mut self) -> D3dResult<Vec<VertexElement>> {
        match self.input {
            None => Err(D3dError::InvalidCall("no FVF or vertex declaration set".into())),
            Some(Input::Fvf(code)) => {
                if let Some(e) = self.fvf_cache.get(&code) {
                    return Ok(e.clone());
                }
                let (e, _) = VertexElement::from_fvf(code).ok_or_else(|| D3dError::InvalidCall(format!("unsupported FVF {code:#x}")))?;
                self.fvf_cache.insert(code, e.clone());
                Ok(e)
            }
            Some(Input::Decl(id)) => self.decls.get(&id).cloned().ok_or_else(|| D3dError::InvalidCall(format!("vertex declaration {id}"))),
        }
    }

    fn samp_key(&self, stage: usize) -> SampKey {
        let s = &self.samp[stage];
        let border = s[samp::BORDERCOLOR as usize];
        let class = if (border >> 24) < 0x80 {
            0
        } else if ((border >> 16) & 0xFF) + ((border >> 8) & 0xFF) + (border & 0xFF) >= 384 {
            2
        } else {
            1
        };
        [
            s[samp::ADDRESSU as usize],
            s[samp::ADDRESSV as usize],
            s[samp::MAGFILTER as usize],
            s[samp::MINFILTER as usize],
            s[samp::MIPFILTER as usize],
            s[samp::MAXANISOTROPY as usize],
            s[samp::MAXMIPLEVEL as usize],
            class,
        ]
    }

    fn sampler(&mut self, k: SampKey) -> wgpu::Sampler {
        if let Some(s) = self.samplers.get(&k) {
            return s.clone();
        }
        let addr = |a: u32| match a {
            2 | 5 => wgpu::AddressMode::MirrorRepeat,
            3 => wgpu::AddressMode::ClampToEdge,
            4 if self.border => wgpu::AddressMode::ClampToBorder,
            4 => wgpu::AddressMode::ClampToEdge,
            _ => wgpu::AddressMode::Repeat,
        };
        let filt = |f: u32| if f >= 2 { wgpu::FilterMode::Linear } else { wgpu::FilterMode::Nearest };
        let (mag, min) = (filt(k[2]), filt(k[3]));
        let mip = if k[4] >= 2 { wgpu::MipmapFilterMode::Linear } else { wgpu::MipmapFilterMode::Nearest };
        let lod_min = k[6] as f32;
        let lod_max = if k[4] == 0 { lod_min } else { 32.0f32.max(lod_min) };
        let aniso_ok = (k[2] == 3 || k[3] == 3) && k[4] >= 2 && mag == wgpu::FilterMode::Linear && min == wgpu::FilterMode::Linear;
        let anisotropy_clamp = if aniso_ok { k[5].clamp(1, 16) as u16 } else { 1 };
        let border_color = match k[7] {
            0 => wgpu::SamplerBorderColor::TransparentBlack,
            1 => wgpu::SamplerBorderColor::OpaqueBlack,
            _ => wgpu::SamplerBorderColor::OpaqueWhite,
        };
        let uses_border = k[0] == 4 || k[1] == 4;
        let s = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: addr(k[0]),
            address_mode_v: addr(k[1]),
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: mag,
            min_filter: min,
            mipmap_filter: mip,
            lod_min_clamp: lod_min,
            lod_max_clamp: lod_max,
            compare: None,
            anisotropy_clamp,
            border_color: (uses_border && self.border).then_some(border_color),
        });
        self.samplers.insert(k, s.clone());
        s
    }

    /// The 2D texture id bound to `stage` as the fixed-function shader sees it (0 = white).
    fn stage_binding(&self, stage: usize) -> TextureId {
        let id = self.stage_binding_any(stage);
        if self.textures.get(&id).is_some_and(|t| t.desc.cube) { 0 } else { id }
    }

    /// The texture id bound to `stage` (2D or cube), 0 if none or not sampleable.
    fn stage_binding_any(&self, stage: usize) -> TextureId {
        match self.stage_tex[stage] {
            Some(id) if Some(id) != self.target.color_id => match self.textures.get(&id) {
                Some(t) if t.store != Store::Depth => id,
                _ => 0,
            },
            _ => 0,
        }
    }

    fn texture_group(&mut self, t0: TextureId, s0: SampKey, t1: TextureId, s1: SampKey) -> wgpu::BindGroup {
        let key = (t0, s0, t1, s1);
        if let Some(g) = self.bind_groups.get(&key) {
            return g.clone();
        }
        let smp0 = self.sampler(s0);
        let smp1 = self.sampler(s1);
        let view = |id: TextureId| self.textures.get(&id).map(|t| &t.view).unwrap_or(&self.white);
        let g = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view(t0)) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&smp0) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(view(t1)) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&smp1) },
            ],
        });
        self.bind_groups.insert(key, g.clone());
        g
    }

    fn shader_module(&mut self, sig: Vec<(u32, u8)>) -> wgpu::ShaderModule {
        if let Some(m) = self.modules.get(&sig) {
            return m.clone();
        }
        let src = format!("{COMMON_WGSL}\n{}\n{FF_VS_WGSL}\n{FF_PS_WGSL}\n{FF_ENTRIES_WGSL}", vertex_header(&sig));
        let m = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("d3d9 fixed function"),
            source: wgpu::ShaderSource::Wgsl(src.into()),
        });
        self.modules.insert(sig, m.clone());
        m
    }

    fn depth_stencil_state(&self, key: &PipeKey) -> wgpu::DepthStencilState {
        let face = |f: [u8; 4]| wgpu::StencilFaceState {
            compare: compare(f[3] as u32),
            fail_op: stencil_op(f[0]),
            depth_fail_op: stencil_op(f[1]),
            pass_op: stencil_op(f[2]),
        };
        let stencil = match &key.stencil {
            Some(s) => wgpu::StencilState { front: face(s.front), back: face(s.back), read_mask: s.read as u32, write_mask: s.write as u32 },
            None => wgpu::StencilState::default(),
        };
        wgpu::DepthStencilState {
            format: DEPTH_STENCIL_FORMAT,
            depth_write_enabled: Some(key.depth_write),
            depth_compare: Some(compare(key.depth_cmp as u32)),
            stencil,
            bias: wgpu::DepthBiasState { constant: key.bias, slope_scale: f32::from_bits(key.slope_bits), clamp: 0.0 },
        }
    }

    fn pipeline(&mut self, key: &PipeKey) -> D3dResult<wgpu::RenderPipeline> {
        if let Some(p) = self.pipelines.get(key) {
            return Ok(p.clone());
        }
        let mut sig: Vec<(u32, u8)> = key.layout.iter().flat_map(|s| s.attrs.iter().map(|&(_, t, loc)| (loc, t))).collect();
        sig.sort_unstable();
        let (module, layout) = if key.prog == ProgKey::default() {
            (self.shader_module(sig), self.pipeline_layout.clone())
        } else {
            (self.program_module(sig, key.prog)?, self.shader_layout(key.prog.cube).1)
        };
        let attrs: Vec<Vec<wgpu::VertexAttribute>> = key
            .layout
            .iter()
            .map(|s| {
                s.attrs
                    .iter()
                    .map(|&(off, t, loc)| wgpu::VertexAttribute {
                        format: decl_format(t).map(|f| f.0).unwrap_or(wgpu::VertexFormat::Float32x4),
                        offset: off as u64,
                        shader_location: loc,
                    })
                    .collect()
            })
            .collect();
        let buffers: Vec<wgpu::VertexBufferLayout<'_>> = key
            .layout
            .iter()
            .zip(&attrs)
            .map(|(s, a)| wgpu::VertexBufferLayout { array_stride: s.stride as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: a })
            .collect();
        let blend = key.blend.map(|b| wgpu::BlendState { color: blend_component(b[0], b[1], b[2]), alpha: blend_component(b[3], b[4], b[5]) });
        let topology = match key.topology {
            0 => wgpu::PrimitiveTopology::PointList,
            1 => wgpu::PrimitiveTopology::LineList,
            _ => wgpu::PrimitiveTopology::TriangleList,
        };
        let cull_mode = match key.cull {
            2 => Some(wgpu::Face::Front),
            3 => Some(wgpu::Face::Back),
            _ => None,
        };
        let p = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("d3d9"),
            layout: Some(&layout),
            vertex: wgpu::VertexState { module: &module, entry_point: Some("vs_main"), compilation_options: Default::default(), buffers: &buffers },
            primitive: wgpu::PrimitiveState {
                topology,
                // D3D front faces are clockwise on screen
                front_face: wgpu::FrontFace::Cw,
                cull_mode,
                ..Default::default()
            },
            depth_stencil: Some(self.depth_stencil_state(key)),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: COLOR_FORMAT,
                    blend,
                    write_mask: wgpu::ColorWrites::from_bits_truncate(key.write_mask as u32),
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        self.stats.pipelines += 1;
        self.pipelines.insert(key.clone(), p.clone());
        Ok(p)
    }

    fn clear_pipeline(&mut self, c: bool, d: bool, s: bool) -> wgpu::RenderPipeline {
        if let Some(p) = self.clear_pipelines.get(&(c, d, s)) {
            return p.clone();
        }
        let face = wgpu::StencilFaceState {
            compare: wgpu::CompareFunction::Always,
            fail_op: wgpu::StencilOperation::Replace,
            depth_fail_op: wgpu::StencilOperation::Replace,
            pass_op: wgpu::StencilOperation::Replace,
        };
        let p = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("d3d9 clear"),
            layout: Some(&self.pipeline_layout),
            vertex: wgpu::VertexState {
                module: &self.clear_module,
                entry_point: Some("vs_clear"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_STENCIL_FORMAT,
                depth_write_enabled: Some(d),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: wgpu::StencilState { front: face, back: face, read_mask: 0xFF, write_mask: if s { 0xFF } else { 0 } },
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &self.clear_module,
                entry_point: Some("fs_clear"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: COLOR_FORMAT,
                    blend: None,
                    write_mask: if c { wgpu::ColorWrites::ALL } else { wgpu::ColorWrites::empty() },
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        self.clear_pipelines.insert((c, d, s), p.clone());
        p
    }

    /// The current viewport clamped to the target, as wgpu viewport + scissor (`None` if empty).
    fn viewport_and_scissor(&self) -> Option<([f32; 6], [u32; 4])> {
        let (tw, th) = self.target.size;
        let vp = &self.viewport;
        let x0 = vp.x.min(tw);
        let y0 = vp.y.min(th);
        let x1 = vp.x.saturating_add(vp.width).min(tw);
        let y1 = vp.y.saturating_add(vp.height).min(th);
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        let min_z = vp.min_z.clamp(0.0, 1.0);
        let max_z = vp.max_z.clamp(min_z, 1.0);
        let viewport = [x0 as f32, y0 as f32, (x1 - x0) as f32, (y1 - y0) as f32, min_z, max_z];
        let mut sc = [0, 0, tw, th];
        if self.r(rs::SCISSORTESTENABLE) != 0 {
            // D3D RECT: left, top, right, bottom
            let (l, t, r, b) = self.scissor;
            let (l, t) = (l.min(tw), t.min(th));
            let (r, b) = (r.min(tw).max(l), b.min(th).max(t));
            if r == l || b == t {
                return None;
            }
            sc = [l, t, r - l, b - t];
        }
        Some((viewport, sc))
    }

    fn base_uniforms(&self) -> Uniforms {
        let vp = &self.viewport;
        let mut u = Uniforms::zeroed();
        u.viewport = [vp.x as f32, vp.y as f32, vp.width.max(1) as f32, vp.height.max(1) as f32];
        u.vpz = [vp.min_z, vp.max_z, 0.0, 0.0];
        u
    }

    fn draw_uniforms(&self, flags: u32) -> Uniforms {
        let mut u = self.base_uniforms();
        let wv = mul(&self.world, &self.view);
        u.wvp = mul(&wv, &self.proj);
        u.world = self.world;
        u.wv = wv;
        let mut f = flags;
        let pre = flags & F_PRETRANSFORMED != 0;
        if !pre && self.r(rs::LIGHTING) != 0 {
            f |= F_LIGHTING;
        }
        if self.r(rs::COLORVERTEX) != 0 {
            f |= F_COLORVERTEX;
        }
        if self.r(rs::FOGENABLE) != 0 {
            f |= F_FOG;
        }
        if self.r(rs::ALPHATESTENABLE) != 0 {
            f |= F_ALPHATEST;
        }
        if self.r(rs::SPECULARENABLE) != 0 {
            f |= F_SPECULAR;
        }
        if self.r(rs::RANGEFOGENABLE) != 0 {
            f |= F_RANGEFOG;
        }
        let lights: Vec<&Light> = self.enabled_lights.iter().filter_map(|i| self.lights.get(i)).take(MAX_LIGHTS).collect();
        u.flags = [f, self.r(rs::ALPHAFUNC), self.r(rs::ALPHAREF) & 0xFF, if f & F_LIGHTING != 0 { lights.len() as u32 } else { 0 }];
        for s in 0..2 {
            let t = &self.tss[s];
            let g = |k: u32| t[k as usize];
            u.stages[s * 3] = [g(tss::COLOROP), g(tss::COLORARG1), g(tss::COLORARG2), g(tss::COLORARG0)];
            u.stages[s * 3 + 1] = [g(tss::ALPHAOP), g(tss::ALPHAARG1), g(tss::ALPHAARG2), g(tss::ALPHAARG0)];
            u.stages[s * 3 + 2] = [g(tss::TEXCOORDINDEX) & 0xFFFF, 0, 0, 0];
        }
        u.tfactor = color(self.r(rs::TEXTUREFACTOR));
        u.fog_color = color(self.r(rs::FOGCOLOR));
        u.fog = [self.rf(rs::FOGSTART), self.rf(rs::FOGEND), self.rf(rs::FOGDENSITY), 0.0];
        u.fog_mode = [self.r(rs::FOGTABLEMODE), if pre { 0 } else { self.r(rs::FOGVERTEXMODE) }, 0, 0];
        u.ambient = color(self.r(rs::AMBIENT));
        let m = &self.material;
        u.mat_diffuse = m.diffuse;
        u.mat_ambient = m.ambient;
        u.mat_specular = m.specular;
        u.mat_emissive = m.emissive;
        u.mat_src = [
            self.r(rs::DIFFUSEMATERIALSOURCE),
            self.r(rs::SPECULARMATERIALSOURCE),
            self.r(rs::AMBIENTMATERIALSOURCE),
            self.r(rs::EMISSIVEMATERIALSOURCE),
        ];
        for (dst, l) in u.lights.iter_mut().zip(&lights) {
            *dst = LightU {
                diffuse: l.diffuse,
                ambient: l.ambient,
                pos: [l.position[0], l.position[1], l.position[2], l.kind as f32],
                dir: [l.direction[0], l.direction[1], l.direction[2], if l.range > 0.0 { l.range } else { f32::MAX }],
                att: [l.attenuation[0], l.attenuation[1], l.attenuation[2], l.falloff],
                spot: [(l.theta * 0.5).cos(), (l.phi * 0.5).cos(), 0.0, 0.0],
            };
        }
        u
    }

    fn pipe_key(&self, layout: Vec<StreamKey>, topology: u8) -> PipeKey {
        let tri = topology == 2;
        let blend = (self.r(rs::ALPHABLENDENABLE) != 0).then(|| {
            let (sb, db) = fix_both(self.r(rs::SRCBLEND), self.r(rs::DESTBLEND));
            let op = self.r(rs::BLENDOP) as u8;
            let (sa, da, opa) = if self.r(rs::SEPARATEALPHABLENDENABLE) != 0 {
                let (sa, da) = fix_both(self.r(rs::SRCBLENDALPHA), self.r(rs::DESTBLENDALPHA));
                (sa, da, self.r(rs::BLENDOPALPHA) as u8)
            } else {
                (sb, db, op)
            };
            [sb, db, op, sa, da, opa]
        });
        let zen = self.r(rs::ZENABLE) != 0;
        let stencil = (self.r(rs::STENCILENABLE) != 0).then(|| {
            let front = [self.r(rs::STENCILFAIL) as u8, self.r(rs::STENCILZFAIL) as u8, self.r(rs::STENCILPASS) as u8, self.r(rs::STENCILFUNC) as u8];
            let back = if self.r(RS_TWOSIDEDSTENCILMODE) != 0 {
                [
                    self.r(RS_CCW_STENCILFAIL) as u8,
                    self.r(RS_CCW_STENCILZFAIL) as u8,
                    self.r(RS_CCW_STENCILPASS) as u8,
                    self.r(RS_CCW_STENCILFUNC) as u8,
                ]
            } else {
                front
            };
            StencilKey { front, back, read: self.r(rs::STENCILMASK) as u8, write: self.r(rs::STENCILWRITEMASK) as u8 }
        });
        let (bias, slope_bits) = if tri {
            let b = (self.rf(rs::DEPTHBIAS) * 16_777_216.0).round();
            let s = self.rf(rs::SLOPESCALEDEPTHBIAS);
            (if b.is_finite() { b as i32 } else { 0 }, if s.is_finite() && s != 0.0 { s.to_bits() } else { 0 })
        } else {
            (0, 0)
        };
        PipeKey {
            prog: ProgKey::default(),
            layout,
            topology,
            blend,
            write_mask: (self.r(rs::COLORWRITEENABLE) & if self.target.opaque { 7 } else { 0xF }) as u8,
            depth_cmp: if zen { self.r(rs::ZFUNC) as u8 } else { 8 },
            depth_write: zen && self.r(rs::ZWRITEENABLE) != 0,
            bias,
            slope_bits,
            stencil,
            cull: if tri { self.r(rs::CULLMODE) as u8 } else { 1 },
        }
    }

    fn draw(&mut self, prim: PrimitiveType, count: u32, src: DrawSrc<'_>) -> D3dResult<()> {
        if count == 0 {
            return Ok(());
        }
        let Some((viewport, scissor)) = self.viewport_and_scissor() else { return Ok(()) };
        let elements = self.input_elements()?;
        let up = matches!(src, DrawSrc::Up { .. } | DrawSrc::UpIndexed { .. });
        let up_stride = match src {
            DrawSrc::Up { stride, .. } | DrawSrc::UpIndexed { stride, .. } => Some(stride),
            _ => None,
        };

        // vertex layout
        let mut layout: Vec<StreamKey> = Vec::new();
        let mut max_end: Vec<u32> = Vec::new();
        let mut used = [false; 16];
        let mut flags = 0u32;
        // with a vertex shader, elements feed the input registers whose `dcl` matches their usage
        let vs_inputs: Option<Vec<(u32, u8, u8)>> = self.vs.and_then(|id| self.shaders.get(&id)).map(|s| s.input_decls());
        for e in &elements {
            if e.stream == 0xFF || e.decl_type == decltype::UNUSED {
                break;
            }
            let loc = match &vs_inputs {
                Some(inputs) => match inputs.iter().find(|d| d.1 == e.usage && d.2 == e.usage_index && d.0 < 16) {
                    Some(d) => d.0,
                    None => continue,
                },
                None => match location(e.usage, e.usage_index) {
                    Some(l) => l,
                    None => continue,
                },
            };
            if used[loc as usize] {
                continue;
            }
            let Some((vf, _, _)) = decl_format(e.decl_type) else {
                return Err(D3dError::NotAvailable(format!("D3DDECLTYPE {}", e.decl_type)));
            };
            used[loc as usize] = true;
            match loc {
                _ if vs_inputs.is_some() => {}
                0 if e.usage == declusage::POSITIONT => flags |= F_PRETRANSFORMED,
                1 => flags |= F_HAS_NORMAL,
                2 => flags |= F_HAS_C0,
                3 => flags |= F_HAS_C1,
                _ => {}
            }
            let stream = e.stream as u32;
            if up && stream != 0 {
                return Err(D3dError::InvalidCall("DrawPrimitiveUP with a multi-stream declaration".into()));
            }
            let idx = match layout.iter().position(|s| s.stream == stream) {
                Some(i) => i,
                None => {
                    let stride = match up_stride {
                        Some(s) => s,
                        None => self.streams.get(stream as usize).map(|s| s.stride).unwrap_or(0),
                    };
                    layout.push(StreamKey { stream, stride, attrs: Vec::new() });
                    max_end.push(0);
                    layout.len() - 1
                }
            };
            let end = e.offset as u32 + vf.size() as u32;
            layout[idx].attrs.push((e.offset as u32, e.decl_type, loc));
            max_end[idx] = max_end[idx].max(end);
        }
        if !used[0] && vs_inputs.is_none() {
            return Err(D3dError::InvalidCall("vertex layout without a position".into()));
        }
        for (s, &end) in layout.iter().zip(&max_end) {
            if s.stride == 0 || !s.stride.is_multiple_of(4) || end > s.stride {
                return Err(D3dError::InvalidCall(format!("stream {} stride {} (attributes end at {end})", s.stream, s.stride)));
            }
        }

        // topology and index expansion
        let (topology, expand) = match prim {
            PrimitiveType::PointList => (0u8, false),
            PrimitiveType::LineList => (1, false),
            PrimitiveType::LineStrip => (1, true),
            PrimitiveType::TriangleList => (2, false),
            PrimitiveType::TriangleStrip | PrimitiveType::TriangleFan => (2, true),
        };
        let n = prim.vertex_count(count);
        let expand_with = |idx: &dyn Fn(u32) -> u32| -> Vec<u32> {
            let mut out = Vec::with_capacity(count as usize * 3);
            for i in 0..count {
                match prim {
                    PrimitiveType::LineStrip => out.extend([idx(i), idx(i + 1)]),
                    PrimitiveType::TriangleStrip if i % 2 == 1 => out.extend([idx(i + 1), idx(i), idx(i + 2)]),
                    PrimitiveType::TriangleStrip => out.extend([idx(i), idx(i + 1), idx(i + 2)]),
                    _ => out.extend([idx(0), idx(i + 1), idx(i + 2)]),
                }
            }
            out
        };

        let mut vertex_buffers: Vec<(BufSrc, u64)> = Vec::new();
        let mut vb_notes: Vec<(VertexBufferId, u64, u64)> = Vec::new();
        let mut ib_note: Option<(IndexBufferId, u64, u64)> = None;
        let index: Option<(BufSrc, u64, wgpu::IndexFormat)>;
        let kind: DrawKind;
        let read_indices = |bytes: &[u8], f: u32, i: u32| -> u32 {
            let i = i as usize;
            if f == fmt::INDEX32 {
                bytes.get(i * 4..i * 4 + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).unwrap_or(0)
            } else {
                bytes.get(i * 2..i * 2 + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as u32).unwrap_or(0)
            }
        };
        let index_wgpu = |f: u32| if f == fmt::INDEX32 { wgpu::IndexFormat::Uint32 } else { wgpu::IndexFormat::Uint16 };
        let isize_of = |f: u32| if f == fmt::INDEX32 { 4u64 } else { 2 };

        match src {
            DrawSrc::Up { vertices, stride } => {
                let bytes = ((n - 1) as u64 * stride as u64 + max_end[0] as u64) as usize;
                let v = vertices.get(..bytes).ok_or_else(|| D3dError::InvalidCall("DrawPrimitiveUP: vertex data too short".into()))?;
                let off = self.arena_push(v);
                vertex_buffers.push((BufSrc::Arena, off));
                if expand {
                    let generated = expand_with(&|i| i);
                    let ioff = self.arena_push(bytemuck::cast_slice(&generated));
                    index = Some((BufSrc::Arena, ioff, wgpu::IndexFormat::Uint32));
                    kind = DrawKind::Indexed { first: 0, count: generated.len() as u32, base: 0 };
                } else {
                    index = None;
                    kind = DrawKind::Plain { first: 0, count: n };
                }
            }
            DrawSrc::UpIndexed { min_index, num_vertices, indices, index_format, vertices, stride } => {
                let isz = isize_of(index_format) as usize;
                let ib = indices.get(..n as usize * isz).ok_or_else(|| D3dError::InvalidCall("DrawIndexedPrimitiveUP: index data too short".into()))?;
                let max_index = (0..n).map(|i| read_indices(ib, index_format, i)).max().unwrap_or(0).max(min_index + num_vertices.max(1) - 1);
                let bytes = (max_index as u64 * stride as u64 + max_end[0] as u64) as usize;
                let v = vertices.get(..bytes.min(vertices.len())).unwrap_or(vertices);
                let off = self.arena_push(v);
                vertex_buffers.push((BufSrc::Arena, off));
                if expand {
                    let generated = expand_with(&|i| read_indices(ib, index_format, i));
                    let ioff = self.arena_push(bytemuck::cast_slice(&generated));
                    index = Some((BufSrc::Arena, ioff, wgpu::IndexFormat::Uint32));
                    kind = DrawKind::Indexed { first: 0, count: generated.len() as u32, base: 0 };
                } else {
                    let mut padded = ib.to_vec();
                    padded.resize(padded.len().div_ceil(4) * 4, 0);
                    let ioff = self.arena_push(&padded);
                    index = Some((BufSrc::Arena, ioff, index_wgpu(index_format)));
                    kind = DrawKind::Indexed { first: 0, count: n, base: 0 };
                }
            }
            DrawSrc::Stream { .. } | DrawSrc::Indexed { .. } => {
                // vertex streams
                for (s, &end) in layout.iter().zip(&max_end) {
                    let st = self.streams.get(s.stream as usize).copied().unwrap_or_default();
                    let vb_id = st.vb.ok_or_else(|| D3dError::InvalidCall(format!("no vertex buffer on stream {}", s.stream)))?;
                    let vb = self.vbs.get(&vb_id).ok_or_else(|| D3dError::InvalidCall(format!("vertex buffer {vb_id}")))?;
                    let len = vb.cpu.len() as u64;
                    let off = st.offset as u64;
                    if !off.is_multiple_of(4) || off >= len {
                        return Err(D3dError::InvalidCall(format!("stream {} offset {off}", s.stream)));
                    }
                    let stride = s.stride as u64;
                    let (r0, r1) = match src {
                        DrawSrc::Stream { start_vertex } => {
                            let last = off + (start_vertex as u64 + n as u64 - 1) * stride + end as u64;
                            if last > len {
                                return Err(D3dError::InvalidCall(format!("draw reads vertex buffer {vb_id} up to {last}, size {len}")));
                            }
                            (off + start_vertex as u64 * stride, last)
                        }
                        DrawSrc::Indexed { base_vertex, min_index, num_vertices, .. } if num_vertices > 0 => {
                            let first = (base_vertex as i64 + min_index as i64).max(0) as u64;
                            (off + first * stride, (off + (first + num_vertices as u64) * stride).min(len))
                        }
                        _ => (0, len),
                    };
                    vb_notes.push((vb_id, r0, r1));
                    vertex_buffers.push((BufSrc::Buf(vb.buf.clone()), off));
                }
                match src {
                    DrawSrc::Stream { start_vertex } => {
                        if expand {
                            let generated = expand_with(&|i| i);
                            let ioff = self.arena_push(bytemuck::cast_slice(&generated));
                            index = Some((BufSrc::Arena, ioff, wgpu::IndexFormat::Uint32));
                            kind = DrawKind::Indexed { first: 0, count: generated.len() as u32, base: start_vertex as i32 };
                        } else {
                            index = None;
                            kind = DrawKind::Plain { first: start_vertex, count: n };
                        }
                    }
                    DrawSrc::Indexed { base_vertex, start_index, .. } => {
                        let ib_id = self.indices.ok_or_else(|| D3dError::InvalidCall("no index buffer set".into()))?;
                        let ib = self.ibs.get(&ib_id).ok_or_else(|| D3dError::InvalidCall(format!("index buffer {ib_id}")))?;
                        let isz = isize_of(ib.index_format);
                        let (i0, i1) = (start_index as u64 * isz, (start_index as u64 + n as u64) * isz);
                        if i1 > ib.cpu.len() as u64 {
                            return Err(D3dError::InvalidCall(format!("indices {start_index}+{n} past index buffer {ib_id}")));
                        }
                        let ib_format = ib.index_format;
                        let ib_buf = ib.buf.clone();
                        let generated = expand.then(|| expand_with(&|i| read_indices(&ib.cpu, ib_format, start_index + i)));
                        if let Some(generated) = generated {
                            let ioff = self.arena_push(bytemuck::cast_slice(&generated));
                            index = Some((BufSrc::Arena, ioff, wgpu::IndexFormat::Uint32));
                            kind = DrawKind::Indexed { first: 0, count: generated.len() as u32, base: base_vertex };
                        } else {
                            ib_note = Some((ib_id, i0, i1));
                            index = Some((BufSrc::Buf(ib_buf), 0, index_wgpu(ib_format)));
                            kind = DrawKind::Indexed { first: start_index, count: n, base: base_vertex };
                        }
                    }
                    _ => unreachable!(),
                }
            }
        }

        // A positive base vertex becomes a vertex-buffer offset (`base * stride` per stream): the same vertices, and
        // WebGL2 has no base-vertex draws (wgpu's GL backend panics on them).
        let mut kind = kind;
        if let DrawKind::Indexed { first, count, base } = kind
            && base > 0
            && vertex_buffers.len() == layout.len()
            && layout.iter().all(|s| s.stride % 4 == 0)
        {
            for (vb, s) in vertex_buffers.iter_mut().zip(layout.iter()) {
                vb.1 += base as u64 * s.stride as u64;
            }
            kind = DrawKind::Indexed { first, count, base: 0 };
        }
        let mut key = self.pipe_key(layout, topology);
        if self.vs.is_some() || self.ps.is_some() {
            key.prog = ProgKey { vs: self.vs.unwrap_or(0), ps: self.ps.unwrap_or(0), cube: self.ps.map(|id| self.ps_cube_mask(id)).unwrap_or(0) };
        }
        let pipeline = self.pipeline(&key)?;
        let u = self.draw_uniforms(flags);
        let uniform_offset = self.push_uniforms(&u);
        let const_offset = self.push_consts();
        let textures = if key.prog == ProgKey::default() {
            let (t0, t1) = (self.stage_binding(0), self.stage_binding(1));
            let (s0, s1) = (self.samp_key(0), self.samp_key(1));
            for t in [t0, t1] {
                if t != 0 {
                    self.pending.textures.insert(t);
                }
            }
            self.texture_group(t0, s0, t1, s1)
        } else {
            self.shader_texture_group(key.prog.cube)
        };
        for (id, a, b) in vb_notes {
            Self::note_range(&mut self.pending.vb_ranges, id, a, b);
        }
        if let Some((id, a, b)) = ib_note {
            Self::note_range(&mut self.pending.ib_ranges, id, a, b);
        }
        let bf = color(self.r(RS_BLENDFACTOR));
        self.pending.draws.push(DrawCmd {
            pipeline,
            textures,
            uniform_offset,
            const_offset,
            vertex_buffers,
            index,
            kind,
            viewport,
            scissor,
            stencil_ref: self.r(rs::STENCILREF) & 0xFF,
            blend_constant: wgpu::Color { r: bf[0] as f64, g: bf[1] as f64, b: bf[2] as f64, a: bf[3] as f64 },
        });
        self.stats.draws += 1;
        Ok(())
    }

    // ------------------------------------------------------------------ shader path ------

    fn create_shader(&mut self, bytecode: &[u32], pixel: bool) -> D3dResult<ShaderId> {
        let sh = sm::parse(bytecode).map_err(|e| D3dError::InvalidCall(format!("shader bytecode: {e}")))?;
        if sh.version.pixel != pixel {
            return Err(D3dError::InvalidCall(format!("{} passed to Create{}Shader", sh.version, if pixel { "Pixel" } else { "Vertex" })));
        }
        // translate once now so unsupported shaders fail at creation, as the caps promise
        sm::wgsl::translate(&sh, &|_| 2).map_err(|e| D3dError::NotAvailable(format!("{}: {e}", sh.version)))?;
        let id = self.alloc_id();
        self.shaders.insert(id, sh);
        Ok(id)
    }

    /// Stages pixel shader `id` samples as cube maps: declared (`dcl_cube`, ps_2_x) or, for
    /// ps_1_x, the dimension of the bound texture.
    fn ps_cube_mask(&self, id: ShaderId) -> u8 {
        let Some(sh) = self.shaders.get(&id) else { return 0 };
        let mut m = 0u8;
        if sh.version.major >= 2 {
            for (s, t) in sh.sampler_decls() {
                if t == sm::sampler_type::CUBE && (s as usize) < SHADER_STAGES {
                    m |= 1 << s;
                }
            }
        } else {
            for s in 0..SHADER_STAGES {
                if let Some(t) = self.stage_tex[s].and_then(|id| self.textures.get(&id))
                    && t.desc.cube
                {
                    m |= 1 << s;
                }
            }
        }
        m
    }

    /// Texture bind group layout of the shader path (stage `s` at bindings `2s`/`2s + 1`, cube
    /// where `cube` has bit `s`) and its pipeline layout.
    fn shader_layout(&mut self, cube: u8) -> (wgpu::BindGroupLayout, wgpu::PipelineLayout) {
        if let Some(l) = self.sh_layouts.get(&cube) {
            return l.clone();
        }
        let mut entries = Vec::new();
        for st in 0..SHADER_STAGES as u32 {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 2 * st,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: if cube >> st & 1 != 0 { wgpu::TextureViewDimension::Cube } else { wgpu::TextureViewDimension::D2 },
                    multisampled: false,
                },
                count: None,
            });
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 2 * st + 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            });
        }
        let bgl = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("d3d9 shader textures"), entries: &entries });
        let pl = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("d3d9 shader"),
            bind_group_layouts: &[Some(&self.uniform_layout), Some(&bgl)],
            immediate_size: 0,
        });
        self.sh_layouts.insert(cube, (bgl.clone(), pl.clone()));
        (bgl, pl)
    }

    /// The eight stages' textures and samplers for the shader path. A texture whose dimension
    /// does not match the layout binds white (D3D9: undefined).
    fn shader_texture_group(&mut self, cube: u8) -> wgpu::BindGroup {
        let mut ids = [0; SHADER_STAGES];
        let mut samps = [[0; 8]; SHADER_STAGES];
        for st in 0..SHADER_STAGES {
            let id = self.stage_binding_any(st);
            let want_cube = cube >> st & 1 != 0;
            ids[st] = if self.textures.get(&id).is_some_and(|t| t.desc.cube == want_cube) { id } else { 0 };
            samps[st] = self.samp_key(st);
            if ids[st] != 0 {
                self.pending.textures.insert(ids[st]);
            }
        }
        let key = (ids, samps, cube);
        if let Some(g) = self.sh_bind_groups.get(&key) {
            return g.clone();
        }
        let (bgl, _) = self.shader_layout(cube);
        let samplers: Vec<wgpu::Sampler> = samps.iter().map(|k| self.sampler(*k)).collect();
        let views: Vec<&wgpu::TextureView> = (0..SHADER_STAGES)
            .map(|st| match self.textures.get(&ids[st]) {
                Some(t) => &t.view,
                None if cube >> st & 1 != 0 => &self.white_cube,
                None => &self.white,
            })
            .collect();
        let mut entries = Vec::new();
        for st in 0..SHADER_STAGES {
            entries.push(wgpu::BindGroupEntry { binding: 2 * st as u32, resource: wgpu::BindingResource::TextureView(views[st]) });
            entries.push(wgpu::BindGroupEntry { binding: 2 * st as u32 + 1, resource: wgpu::BindingResource::Sampler(&samplers[st]) });
        }
        let g = self.device.create_bind_group(&wgpu::BindGroupDescriptor { label: None, layout: &bgl, entries: &entries });
        self.sh_bind_groups.insert(key, g.clone());
        g
    }

    /// The WGSL module of a draw with a vertex and/or pixel shader (`sig`: input location =
    /// register for a vertex shader, the fixed-function locations otherwise).
    fn program_module(&mut self, sig: Vec<(u32, u8)>, prog: ProgKey) -> D3dResult<wgpu::ShaderModule> {
        let key = (sig, prog);
        if let Some(m) = self.program_modules.get(&key) {
            return Ok(m.clone());
        }
        let (sig, _) = &key;
        let tr = |id: ShaderId, kinds: &dyn Fn(u32) -> u8| -> D3dResult<sm::wgsl::Translated> {
            let sh = self.shaders.get(&id).ok_or_else(|| D3dError::InvalidCall(format!("shader {id}")))?;
            sm::wgsl::translate(sh, kinds).map_err(|e| D3dError::NotAvailable(e.to_string()))
        };
        let mut src = format!("{COMMON_WGSL}\n{SV_WGSL}\n{}\n", sm::wgsl::PRELUDE);
        // vertex stage
        if prog.vs != 0 {
            let t = tr(prog.vs, &|_| 2)?;
            src += &t.code;
            src += &shader_vertex_header(sig);
            src += r#"
@vertex
fn vs_main(i: VIn) -> SV {
    let r = vs_body(load_regs(i));
    var p = r.pos;
    // D3D9 half-pixel convention
    p.x += p.w / u.viewport.z;
    p.y -= p.w / u.viewport.w;
    var o: SV;
    o.pos = p;
    // oD0/oD1 are clamped to [0, 1] before interpolation
    o.d0 = clamp(r.d0, vec4f(0.0), vec4f(1.0));
    o.d1 = clamp(r.d1, vec4f(0.0), vec4f(1.0));
    o.t0 = r.t[0];
    o.t1 = r.t[1];
    o.t2 = r.t[2];
    o.t3 = r.t[3];
    o.t4 = r.t[4];
    o.t5 = r.t[5];
    o.t6 = r.t[6];
    o.t7 = r.t[7];
    o.fog = vec2f(p.w, r.fog);
    return o;
}
"#;
        } else {
            src += &vertex_header(sig);
            src += FF_VS_WGSL;
            src += r#"
@vertex
fn vs_main(i: VIn) -> SV {
    let f = ff_vertex(i);
    var o: SV;
    o.pos = f.pos;
    o.d0 = f.c0;
    o.d1 = f.c1;
    o.t0 = vec4f(f.t0, 0.0, 1.0);
    o.t1 = vec4f(f.t1, 0.0, 1.0);
    o.fog = f.fog;
    return o;
}
"#;
        }
        // pixel stage
        if prog.ps != 0 {
            let cube = prog.cube;
            let t = tr(prog.ps, &|s| if s < 8 && cube >> s & 1 != 0 { 3 } else { 2 })?;
            src += &t.code;
            src += "\nfn ps_inputs(i: SV) -> SmPsOut {\n    return ps_body(i.d0, i.d1, array<vec4f, 8>(i.t0, i.t1, i.t2, i.t3, i.t4, i.t5, i.t6, i.t7));\n}\n";
            if t.writes_depth {
                src += r#"
struct PsOutD {
    @location(0) color: vec4f,
    @builtin(frag_depth) depth: f32,
}

@fragment
fn fs_main(i: SV) -> PsOutD {
    let r = ps_inputs(i);
    return PsOutD(ff_fog_alpha(r.color, i.fog), clamp(r.depth, 0.0, 1.0));
}
"#;
            } else {
                src += r#"
@fragment
fn fs_main(i: SV) -> @location(0) vec4f {
    return ff_fog_alpha(ps_inputs(i).color, i.fog);
}
"#;
            }
        } else {
            src += FF_PS_WGSL;
            src += r#"
@fragment
fn fs_main(i: SV) -> @location(0) vec4f {
    return ff_fog_alpha(ff_pixel(i.d0, i.d1, i.t0.xy, i.t1.xy), i.fog);
}
"#;
        }
        let m = self.device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("d3d9 shader"), source: wgpu::ShaderSource::Wgsl(src.into()) });
        self.program_modules.insert(key, m.clone());
        Ok(m)
    }

    fn tex_level_size(t: &Tex, level: u32) -> (u32, u32) {
        ((t.desc.width >> level).max(1), (t.desc.height >> level).max(1))
    }
}

impl D3d9Device for WgpuDevice {
    fn caps(&self) -> Caps {
        let l = self.device.limits();
        Caps {
            max_texture_width: l.max_texture_dimension_2d,
            max_texture_height: l.max_texture_dimension_2d,
            max_anisotropy: 16,
            max_active_lights: MAX_LIGHTS as u32,
            max_simultaneous_textures: 2,
            vertex_shader_version: 0xFFFE_0200,
            pixel_shader_version: 0xFFFF_0200,
            dxt: true,
            non_pow2_textures: true,
        }
    }

    fn back_buffer_size(&self) -> (u32, u32) {
        self.size
    }

    fn reset(&mut self, width: u32, height: u32) -> D3dResult<()> {
        self.flush();
        let (w, h) = (width.max(1), height.max(1));
        let max = self.device.limits().max_texture_dimension_2d;
        if w > max || h > max {
            return Err(D3dError::InvalidCall(format!("back buffer {w}x{h} exceeds {max}")));
        }
        self.size = (w, h);
        self.back = color_texture(&self.device, w, h, "d3d9 back buffer");
        self.back_view = self.back.create_view(&Default::default());
        if self.back_opaque {
            let v = self.back_view.clone();
            self.clear_to_opaque_black(&v);
        }
        let depth = depth_texture(&self.device, w, h).create_view(&Default::default());
        self.depth_by_size.insert((w, h), depth);
        if self.target.color_id.is_none() {
            self.set_render_target(None, None);
        }
        Ok(())
    }

    fn create_texture(&mut self, desc: &TextureDesc) -> D3dResult<TextureId> {
        if desc.cube && (desc.render_target || desc.depth_stencil) {
            return Err(D3dError::NotAvailable("cube render targets are not supported yet".into()));
        }
        if desc.cube && desc.width != desc.height {
            return Err(D3dError::InvalidCall(format!("cube texture {}x{}", desc.width, desc.height)));
        }
        let max = self.device.limits().max_texture_dimension_2d;
        if desc.width == 0 || desc.height == 0 || desc.width > max || desc.height > max {
            return Err(D3dError::InvalidCall(format!("texture size {}x{}", desc.width, desc.height)));
        }
        let chain = full_chain(desc.width, desc.height);
        let (store, format, levels, usage) = if desc.depth_stencil {
            (Store::Depth, DEPTH_STENCIL_FORMAT, 1, wgpu::TextureUsages::RENDER_ATTACHMENT)
        } else if desc.render_target {
            (
                Store::Rgba8,
                COLOR_FORMAT,
                1,
                wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST,
            )
        } else {
            let levels = if desc.levels == 0 { chain } else { desc.levels.min(chain) };
            let usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
            let kind = match desc.format {
                fmt::DXT1 => Some((dxt::Kind::Bc1, wgpu::TextureFormat::Bc1RgbaUnorm, 8)),
                fmt::DXT2 | fmt::DXT3 => Some((dxt::Kind::Bc2, wgpu::TextureFormat::Bc2RgbaUnorm, 16)),
                fmt::DXT4 | fmt::DXT5 => Some((dxt::Kind::Bc3, wgpu::TextureFormat::Bc3RgbaUnorm, 16)),
                _ => None,
            };
            match kind {
                Some((_, f, bytes)) if self.bc && desc.width.is_multiple_of(4) && desc.height.is_multiple_of(4) => (Store::Bc(bytes), f, levels, usage),
                Some((k, _, _)) => (Store::Dxt(k), wgpu::TextureFormat::Rgba8Unorm, levels, usage),
                None => {
                    if fmt::bytes_per_pixel(desc.format).is_none() || matches!(desc.format, fmt::P8 | fmt::V8U8) {
                        return Err(D3dError::NotAvailable(format!("texture format {}", desc.format)));
                    }
                    (Store::Rgba8, wgpu::TextureFormat::Rgba8Unorm, levels, usage)
                }
            }
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width: desc.width, height: desc.height, depth_or_array_layers: if desc.cube { 6 } else { 1 } },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(if desc.cube { wgpu::TextureViewDimension::Cube } else { wgpu::TextureViewDimension::D2 }),
            ..Default::default()
        });
        if desc.render_target && format_is_opaque(desc.format) {
            self.clear_to_opaque_black(&view);
        }
        let id = self.alloc_id();
        self.textures.insert(id, Tex { texture, view, desc: *desc, levels, store });
        Ok(id)
    }

    fn update_texture(&mut self, tex: TextureId, face: u32, level: u32, rect: Option<(u32, u32, u32, u32)>, data: &[u8], pitch: u32) -> D3dResult<()> {
        if self.pending.textures.contains(&tex) || self.target.color_id == Some(tex) {
            self.flush();
        }
        let t = self.textures.get(&tex).ok_or_else(|| D3dError::InvalidCall(format!("texture {tex}")))?;
        if face >= if t.desc.cube { 6 } else { 1 } {
            return Err(D3dError::InvalidCall(format!("face {face} of a {} texture", if t.desc.cube { "cube" } else { "2D" })));
        }
        if level >= t.levels {
            return Err(D3dError::InvalidCall(format!("level {level} of {}", t.levels)));
        }
        let (lw, lh) = Self::tex_level_size(t, level);
        let (x, y, w, h) = rect.unwrap_or((0, 0, lw, lh));
        if x >= lw || y >= lh || w == 0 || h == 0 {
            return Ok(());
        }
        let (w, h) = (w.min(lw - x), h.min(lh - y));
        let pitch = pitch as usize;
        let write = |origin: (u32, u32), bytes: &[u8], bpr: u32, rows: u32, extent: (u32, u32)| {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &t.texture,
                    mip_level: level,
                    origin: wgpu::Origin3d { x: origin.0, y: origin.1, z: face },
                    aspect: wgpu::TextureAspect::All,
                },
                bytes,
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bpr), rows_per_image: Some(rows) },
                wgpu::Extent3d { width: extent.0, height: extent.1, depth_or_array_layers: 1 },
            );
        };
        // block data of the rectangle repacked to tight rows
        let blocks = |block: usize| -> D3dResult<(Vec<u8>, usize, usize)> {
            if !x.is_multiple_of(4) || !y.is_multiple_of(4) {
                return Err(D3dError::InvalidCall("DXT rectangle not block aligned".into()));
            }
            let (bw, bh) = (w.div_ceil(4) as usize, h.div_ceil(4) as usize);
            let row = bw * block;
            if data.len() < (bh - 1) * pitch + row {
                return Err(D3dError::InvalidCall("DXT data too short".into()));
            }
            let mut out = Vec::with_capacity(row * bh);
            for r in 0..bh {
                out.extend_from_slice(&data[r * pitch..r * pitch + row]);
            }
            Ok((out, bw, bh))
        };
        match t.store {
            Store::Depth => return Err(D3dError::InvalidCall("depth textures cannot be updated".into())),
            Store::Rgba8 => {
                let rgba = convert_to_rgba8(t.desc.format, data, pitch, w as usize, h as usize)?;
                write((x, y), &rgba, w * 4, h, (w, h));
            }
            Store::Bc(block) => {
                let (bytes, bw, bh) = blocks(block as usize)?;
                // copies of block formats use the physical (block-rounded) size
                let (pw, ph) = (lw.div_ceil(4) * 4, lh.div_ceil(4) * 4);
                let ew = (bw as u32 * 4).min(pw - x);
                let eh = (bh as u32 * 4).min(ph - y);
                write((x, y), &bytes, bw as u32 * block, bh as u32, (ew, eh));
            }
            Store::Dxt(kind) => {
                let block = if kind == dxt::Kind::Bc1 { 8 } else { 16 };
                let (bytes, bw, bh) = blocks(block)?;
                let full = dxt::decode(kind, &bytes, bw as u32 * 4, bh as u32 * 4);
                let mut crop = Vec::with_capacity((w * h * 4) as usize);
                for r in 0..h as usize {
                    let s = r * bw * 16;
                    crop.extend_from_slice(&full[s..s + w as usize * 4]);
                }
                write((x, y), &crop, w * 4, h, (w, h));
            }
        }
        Ok(())
    }

    fn release_texture(&mut self, tex: TextureId) {
        if self.textures.remove(&tex).is_some() {
            self.bind_groups.retain(|k, _| k.0 != tex && k.2 != tex);
            self.sh_bind_groups.retain(|k, _| !k.0.contains(&tex));
            for s in self.stage_tex.iter_mut() {
                if *s == Some(tex) {
                    *s = None;
                }
            }
            if self.target.color_id == Some(tex) {
                self.set_render_target(None, None);
            }
        }
    }

    fn create_vertex_buffer(&mut self, size: u32, _dynamic: bool) -> D3dResult<VertexBufferId> {
        self.create_buffer(size, wgpu::BufferUsages::VERTEX, 0)
    }

    fn update_vertex_buffer(&mut self, vb: VertexBufferId, offset: u32, data: &[u8]) -> D3dResult<()> {
        if self.vb_pending(vb, offset as u64, offset as u64 + data.len() as u64) {
            self.flush();
        }
        let b = self.vbs.get_mut(&vb).ok_or_else(|| D3dError::InvalidCall(format!("vertex buffer {vb}")))?;
        Self::write_buf(&self.queue, b, offset, data)
    }

    fn release_vertex_buffer(&mut self, vb: VertexBufferId) {
        self.vbs.remove(&vb);
        for s in self.streams.iter_mut() {
            if s.vb == Some(vb) {
                s.vb = None;
            }
        }
    }

    fn create_index_buffer(&mut self, size: u32, format: u32, _dynamic: bool) -> D3dResult<IndexBufferId> {
        if format != fmt::INDEX16 && format != fmt::INDEX32 {
            return Err(D3dError::InvalidCall(format!("index format {format}")));
        }
        self.create_buffer(size, wgpu::BufferUsages::INDEX, format)
    }

    fn update_index_buffer(&mut self, ib: IndexBufferId, offset: u32, data: &[u8]) -> D3dResult<()> {
        if self.ib_pending(ib, offset as u64, offset as u64 + data.len() as u64) {
            self.flush();
        }
        let b = self.ibs.get_mut(&ib).ok_or_else(|| D3dError::InvalidCall(format!("index buffer {ib}")))?;
        Self::write_buf(&self.queue, b, offset, data)
    }

    fn release_index_buffer(&mut self, ib: IndexBufferId) {
        self.ibs.remove(&ib);
        if self.indices == Some(ib) {
            self.indices = None;
        }
    }

    fn create_vertex_declaration(&mut self, elements: &[VertexElement]) -> D3dResult<VertexDeclId> {
        let id = self.alloc_id();
        self.decls.insert(id, elements.to_vec());
        Ok(id)
    }

    fn release_vertex_declaration(&mut self, decl: VertexDeclId) {
        self.decls.remove(&decl);
        if self.input == Some(Input::Decl(decl)) {
            self.input = None;
        }
    }

    fn create_vertex_shader(&mut self, bytecode: &[u32]) -> D3dResult<ShaderId> {
        self.create_shader(bytecode, false)
    }

    fn create_pixel_shader(&mut self, bytecode: &[u32]) -> D3dResult<ShaderId> {
        self.create_shader(bytecode, true)
    }

    fn release_shader(&mut self, shader: ShaderId) {
        if self.shaders.remove(&shader).is_some() {
            self.flush();
            self.pipelines.retain(|k, _| k.prog.vs != shader && k.prog.ps != shader);
            self.program_modules.retain(|k, _| k.1.vs != shader && k.1.ps != shader);
            if self.vs == Some(shader) {
                self.vs = None;
            }
            if self.ps == Some(shader) {
                self.ps = None;
            }
        }
    }

    fn set_render_state(&mut self, state: u32, value: u32) {
        if let Some(v) = self.rs.get_mut(state as usize) {
            *v = value;
        }
    }

    fn render_state(&self, state: u32) -> u32 {
        self.r(state)
    }

    fn set_texture_stage_state(&mut self, stage: u32, kind: u32, value: u32) {
        if let Some(v) = self.tss.get_mut(stage as usize).and_then(|s| s.get_mut(kind as usize)) {
            *v = value;
        }
    }

    fn set_sampler_state(&mut self, sampler: u32, kind: u32, value: u32) {
        if let Some(v) = self.samp.get_mut(sampler as usize).and_then(|s| s.get_mut(kind as usize)) {
            *v = value;
        }
    }

    fn set_texture(&mut self, stage: u32, tex: Option<TextureId>) {
        if let Some(s) = self.stage_tex.get_mut(stage as usize) {
            *s = tex;
        }
    }

    fn set_transform(&mut self, kind: u32, m: &[[f32; 4]; 4]) {
        match kind {
            ts::WORLD => self.world = *m,
            ts::VIEW => self.view = *m,
            ts::PROJECTION => self.proj = *m,
            _ => {} // texture transforms and WORLDMATRIX(1..) are not emulated
        }
    }

    fn set_material(&mut self, m: &Material) {
        self.material = *m;
    }

    fn set_light(&mut self, index: u32, l: &Light) {
        self.lights.insert(index, *l);
    }

    fn light_enable(&mut self, index: u32, on: bool) {
        if on {
            self.lights.entry(index).or_insert(Light {
                kind: 3,
                diffuse: [1.0, 1.0, 1.0, 0.0],
                direction: [0.0, 0.0, 1.0],
                ..Default::default()
            });
            self.enabled_lights.insert(index);
        } else {
            self.enabled_lights.remove(&index);
        }
    }

    fn set_viewport(&mut self, vp: &Viewport) {
        self.viewport = *vp;
    }

    fn set_scissor_rect(&mut self, rect: (u32, u32, u32, u32)) {
        self.scissor = rect;
    }

    fn set_fvf(&mut self, fvf: u32) {
        self.input = Some(Input::Fvf(fvf));
    }

    fn set_vertex_declaration(&mut self, decl: Option<VertexDeclId>) {
        self.input = decl.map(Input::Decl);
    }

    fn set_vertex_shader(&mut self, shader: Option<ShaderId>) {
        self.vs = shader.filter(|id| self.shaders.get(id).is_some_and(|s| !s.version.pixel));
    }

    fn set_pixel_shader(&mut self, shader: Option<ShaderId>) {
        self.ps = shader.filter(|id| self.shaders.get(id).is_some_and(|s| s.version.pixel));
    }

    fn set_vertex_shader_constant_f(&mut self, start: u32, data: &[[f32; 4]]) {
        set_regs(&mut self.consts.vc, start, data);
        self.pending.last_consts = None;
    }

    fn set_pixel_shader_constant_f(&mut self, start: u32, data: &[[f32; 4]]) {
        set_regs(&mut self.consts.pc, start, data);
        self.pending.last_consts = None;
    }

    fn set_vertex_shader_constant_i(&mut self, start: u32, data: &[[i32; 4]]) {
        set_regs(&mut self.consts.vi, start, data);
        self.pending.last_consts = None;
    }

    fn set_pixel_shader_constant_i(&mut self, start: u32, data: &[[i32; 4]]) {
        set_regs(&mut self.consts.pi, start, data);
        self.pending.last_consts = None;
    }

    fn set_vertex_shader_constant_b(&mut self, start: u32, data: &[bool]) {
        let v: Vec<u32> = data.iter().map(|&b| b as u32).collect();
        set_regs(&mut self.consts.vb, start, &v);
        self.pending.last_consts = None;
    }

    fn set_pixel_shader_constant_b(&mut self, start: u32, data: &[bool]) {
        let v: Vec<u32> = data.iter().map(|&b| b as u32).collect();
        set_regs(&mut self.consts.pb, start, &v);
        self.pending.last_consts = None;
    }

    fn set_stream_source(&mut self, stream: u32, vb: Option<VertexBufferId>, offset: u32, stride: u32) {
        if let Some(s) = self.streams.get_mut(stream as usize) {
            *s = Stream { vb, offset, stride };
        }
    }

    fn set_indices(&mut self, ib: Option<IndexBufferId>) {
        self.indices = ib;
    }

    fn set_render_target(&mut self, color: Option<TextureId>, depth: Option<TextureId>) {
        self.flush();
        let (view, size, id, opaque) = match color.and_then(|id| self.textures.get(&id).filter(|t| t.desc.render_target).map(|t| (id, t))) {
            Some((id, t)) => (t.view.clone(), (t.desc.width, t.desc.height), Some(id), format_is_opaque(t.desc.format)),
            None => (self.back_view.clone(), self.size, None, self.back_opaque),
        };
        let depth_view = match depth.and_then(|d| self.textures.get(&d)) {
            Some(t) if t.store == Store::Depth && (t.desc.width, t.desc.height) == size => t.view.clone(),
            _ => self
                .depth_by_size
                .entry(size)
                .or_insert_with(|| depth_texture(&self.device, size.0, size.1).create_view(&Default::default()))
                .clone(),
        };
        self.target = Target { color_view: view, depth_view, size, color_id: id, opaque };
        // D3D9 resets the viewport and scissor rectangle to the new target
        self.viewport = Viewport { x: 0, y: 0, width: size.0, height: size.1, min_z: 0.0, max_z: 1.0 };
        self.scissor = (0, 0, size.0, size.1);
    }

    fn begin_scene(&mut self) -> D3dResult<()> {
        Ok(())
    }

    fn end_scene(&mut self) -> D3dResult<()> {
        Ok(())
    }

    fn clear(&mut self, flags: u32, color_value: u32, z: f32, stencil: u32) {
        let (c, d, s) = (flags & clear::TARGET != 0, flags & clear::ZBUFFER != 0, flags & clear::STENCIL != 0);
        if !(c || d || s) {
            return;
        }
        let Some((mut viewport, scissor)) = self.viewport_and_scissor() else { return };
        viewport[4] = 0.0;
        viewport[5] = 1.0;
        let pipeline = self.clear_pipeline(c, d, s);
        let mut u = self.base_uniforms();
        u.tfactor = color(color_value);
        if self.target.opaque {
            u.tfactor[3] = 1.0;
        }
        u.vpz[2] = z.clamp(0.0, 1.0);
        let uniform_offset = self.push_uniforms(&u);
        let textures = self.texture_group(0, self.samp_key(0), 0, self.samp_key(0));
        let const_offset = self.push_consts();
        self.pending.draws.push(DrawCmd {
            pipeline,
            textures,
            uniform_offset,
            const_offset,
            vertex_buffers: Vec::new(),
            index: None,
            kind: DrawKind::Plain { first: 0, count: 3 },
            viewport,
            scissor,
            stencil_ref: stencil & 0xFF,
            blend_constant: wgpu::Color::WHITE,
        });
    }

    fn draw_primitive(&mut self, prim: PrimitiveType, start_vertex: u32, count: u32) -> D3dResult<()> {
        self.draw(prim, count, DrawSrc::Stream { start_vertex })
    }

    fn draw_indexed_primitive(
        &mut self,
        prim: PrimitiveType,
        base_vertex: i32,
        min_index: u32,
        num_vertices: u32,
        start_index: u32,
        count: u32,
    ) -> D3dResult<()> {
        self.draw(prim, count, DrawSrc::Indexed { base_vertex, min_index, num_vertices, start_index })
    }

    fn draw_primitive_up(&mut self, prim: PrimitiveType, count: u32, vertices: &[u8], stride: u32) -> D3dResult<()> {
        let r = self.draw(prim, count, DrawSrc::Up { vertices, stride });
        // D3D9: stream 0 is unset after DrawPrimitiveUP
        self.streams[0] = Stream::default();
        r
    }

    fn draw_indexed_primitive_up(
        &mut self,
        prim: PrimitiveType,
        min_index: u32,
        num_vertices: u32,
        count: u32,
        indices: &[u8],
        index_format: u32,
        vertices: &[u8],
        stride: u32,
    ) -> D3dResult<()> {
        let r = self.draw(prim, count, DrawSrc::UpIndexed { min_index, num_vertices, indices, index_format, vertices, stride });
        // D3D9: stream 0 and the index buffer are unset after DrawIndexedPrimitiveUP
        self.streams[0] = Stream::default();
        self.indices = None;
        r
    }

    fn present(&mut self) -> D3dResult<()> {
        self.flush();
        Ok(())
    }

    /// Equal-size rectangles of two different targets: a GPU copy. Scaled (or same-target) copies
    /// read the source back and resample on the CPU (not on wasm32, which has no blocking read-back).
    fn stretch_rect(
        &mut self,
        src: Option<TextureId>,
        src_rect: Option<(u32, u32, u32, u32)>,
        dst: Option<TextureId>,
        dst_rect: Option<(u32, u32, u32, u32)>,
        linear: bool,
    ) -> D3dResult<()> {
        self.flush();
        let target = |id: Option<TextureId>| -> D3dResult<(wgpu::Texture, (u32, u32))> {
            match id {
                None => Ok((self.back.clone(), self.size)),
                Some(id) => match self.textures.get(&id) {
                    Some(t) if t.desc.render_target => Ok((t.texture.clone(), (t.desc.width, t.desc.height))),
                    _ => Err(D3dError::InvalidCall(format!("StretchRect: texture {id} is not a render target"))),
                },
            }
        };
        let (st, ss) = target(src)?;
        let (dt, ds) = target(dst)?;
        let opaque = |id: Option<TextureId>| match id {
            None => self.back_opaque,
            Some(id) => self.textures.get(&id).is_some_and(|t| format_is_opaque(t.desc.format)),
        };
        let (src_opaque, dst_opaque) = (opaque(src), opaque(dst));
        let sr = src_rect.unwrap_or((0, 0, ss.0, ss.1));
        let dr = dst_rect.unwrap_or((0, 0, ds.0, ds.1));
        let inside = |r: (u32, u32, u32, u32), s: (u32, u32)| {
            r.2 > 0 && r.3 > 0 && r.0.checked_add(r.2).is_some_and(|e| e <= s.0) && r.1.checked_add(r.3).is_some_and(|e| e <= s.1)
        };
        if !inside(sr, ss) || !inside(dr, ds) {
            return Err(D3dError::InvalidCall(format!("StretchRect: rectangle {sr:?} -> {dr:?} outside the targets")));
        }
        if (sr.2, sr.3) == (dr.2, dr.3) && src != dst {
            let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("d3d9 stretch") });
            enc.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo { texture: &st, mip_level: 0, origin: wgpu::Origin3d { x: sr.0, y: sr.1, z: 0 }, aspect: wgpu::TextureAspect::All },
                wgpu::TexelCopyTextureInfo { texture: &dt, mip_level: 0, origin: wgpu::Origin3d { x: dr.0, y: dr.1, z: 0 }, aspect: wgpu::TextureAspect::All },
                wgpu::Extent3d { width: sr.2, height: sr.3, depth_or_array_layers: 1 },
            );
            self.queue.submit([enc.finish()]);
            // D3D9 drops the alpha of a copy into a target without alpha (it samples as 1)
            if dst_opaque && !src_opaque {
                let view = match dst {
                    None => self.back_view.clone(),
                    Some(id) => self.textures[&id].view.clone(),
                };
                self.set_alpha_one(&view, dr);
            }
            return Ok(());
        }
        let (w, _, rgba) = self.read_render_target(src)?;
        let px = |x: u32, y: u32| {
            let o = ((y * w + x) * 4) as usize;
            [rgba[o], rgba[o + 1], rgba[o + 2], rgba[o + 3]]
        };
        let mut out = Vec::with_capacity((dr.2 * dr.3 * 4) as usize);
        for j in 0..dr.3 {
            for i in 0..dr.2 {
                // texel centre of the destination pixel in source coordinates
                let u = sr.0 as f32 + (i as f32 + 0.5) * sr.2 as f32 / dr.2 as f32;
                let v = sr.1 as f32 + (j as f32 + 0.5) * sr.3 as f32 / dr.3 as f32;
                let (x0, x1) = (sr.0, sr.0 + sr.2 - 1);
                let (y0, y1) = (sr.1, sr.1 + sr.3 - 1);
                if linear {
                    let (fu, fv) = (u - 0.5, v - 0.5);
                    let (xa, ya) = (fu.floor(), fv.floor());
                    let (tx, ty) = (fu - xa, fv - ya);
                    let cx = |x: f32| (x.max(x0 as f32) as u32).clamp(x0, x1);
                    let cy = |y: f32| (y.max(y0 as f32) as u32).clamp(y0, y1);
                    let (a, b) = (px(cx(xa), cy(ya)), px(cx(xa + 1.0), cy(ya)));
                    let (c, d) = (px(cx(xa), cy(ya + 1.0)), px(cx(xa + 1.0), cy(ya + 1.0)));
                    for k in 0..4 {
                        let top = a[k] as f32 * (1.0 - tx) + b[k] as f32 * tx;
                        let bot = c[k] as f32 * (1.0 - tx) + d[k] as f32 * tx;
                        out.push((top * (1.0 - ty) + bot * ty).round() as u8);
                    }
                } else {
                    let p = px((u as u32).clamp(x0, x1), (v as u32).clamp(y0, y1));
                    out.extend_from_slice(&p);
                }
                if let Some(a) = out.last_mut().filter(|_| dst_opaque) {
                    *a = 255;
                }
            }
        }
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &dt, mip_level: 0, origin: wgpu::Origin3d { x: dr.0, y: dr.1, z: 0 }, aspect: wgpu::TextureAspect::All },
            &out,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(dr.2 * 4), rows_per_image: Some(dr.3) },
            wgpu::Extent3d { width: dr.2, height: dr.3, depth_or_array_layers: 1 },
        );
        Ok(())
    }

    #[cfg(target_arch = "wasm32")]
    fn read_render_target(&mut self, _tex: Option<TextureId>) -> D3dResult<(u32, u32, Vec<u8>)> {
        Err(D3dError::NotAvailable("render-target read-back needs a blocking map (not on wasm32)".into()))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn read_render_target(&mut self, tex: Option<TextureId>) -> D3dResult<(u32, u32, Vec<u8>)> {
        self.flush();
        let (texture, (w, h)) = match tex {
            None => (self.back.clone(), self.size),
            Some(id) => {
                let t = self.textures.get(&id).ok_or_else(|| D3dError::InvalidCall(format!("texture {id}")))?;
                if !t.desc.render_target {
                    return Err(D3dError::InvalidCall("read_render_target on a non-render-target texture".into()));
                }
                (t.texture.clone(), (t.desc.width, t.desc.height))
            }
        };
        let bpr = (w * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("d3d9 readback"),
            size: (bpr * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = self.device.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bpr), rows_per_image: Some(h) },
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        self.queue.submit([enc.finish()]);
        let slice = buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        match rx.recv() {
            Ok(Ok(())) => {}
            _ => return Err(D3dError::DeviceLost),
        }
        let data = slice.get_mapped_range();
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h as usize {
            out.extend_from_slice(&data[y * bpr as usize..y * bpr as usize + (w * 4) as usize]);
        }
        drop(data);
        buf.unmap();
        Ok((w, h, out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sa_platform::gpu::fvf;

    const RHW_C: u32 = fvf::XYZRHW | fvf::DIFFUSE;
    const RHW_CT: u32 = fvf::XYZRHW | fvf::DIFFUSE | 0x100;
    const XYZ_C: u32 = fvf::XYZ | fvf::DIFFUSE;
    const XYZ_N: u32 = fvf::XYZ | fvf::NORMAL;

    type Image = (u32, u32, Vec<u8>);

    fn dev(w: u32, h: u32) -> Option<WgpuDevice> {
        let d = WgpuDevice::new_headless(w, h);
        if d.is_none() {
            eprintln!("skipping: no wgpu adapter available");
        }
        d
    }

    fn px(img: &Image, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * img.0 + x) * 4) as usize;
        [img.2[i], img.2[i + 1], img.2[i + 2], img.2[i + 3]]
    }

    #[track_caller]
    fn assert_px(img: &Image, x: u32, y: u32, want: [u8; 3], tol: i32) {
        let p = px(img, x, y);
        let ok = (0..3).all(|k| (p[k] as i32 - want[k] as i32).abs() <= tol);
        assert!(ok, "pixel ({x}, {y}) = {p:?}, want {want:?} ±{tol}");
    }

    fn f(buf: &mut Vec<u8>, v: &[f32]) {
        for x in v {
            buf.extend_from_slice(&x.to_le_bytes());
        }
    }

    fn u(buf: &mut Vec<u8>, v: u32) {
        buf.extend_from_slice(&v.to_le_bytes());
    }

    /// XYZRHW | DIFFUSE vertex.
    fn rhw_c(buf: &mut Vec<u8>, x: f32, y: f32, c: u32) {
        f(buf, &[x, y, 0.5, 1.0]);
        u(buf, c);
    }

    /// XYZRHW | DIFFUSE | TEX1 vertex.
    fn rhw_ct(buf: &mut Vec<u8>, x: f32, y: f32, c: u32, tu: f32, tv: f32) {
        rhw_c(buf, x, y, c);
        f(buf, &[tu, tv]);
    }

    fn xyz_c(buf: &mut Vec<u8>, x: f32, y: f32, z: f32, c: u32) {
        f(buf, &[x, y, z]);
        u(buf, c);
    }

    /// Screen-space quad as a 4-vertex strip (clockwise front faces).
    fn quad_ct(x0: f32, y0: f32, x1: f32, y1: f32, c: u32) -> Vec<u8> {
        let mut v = Vec::new();
        rhw_ct(&mut v, x0, y0, c, 0.0, 0.0);
        rhw_ct(&mut v, x1, y0, c, 1.0, 0.0);
        rhw_ct(&mut v, x0, y1, c, 0.0, 1.0);
        rhw_ct(&mut v, x1, y1, c, 1.0, 1.0);
        v
    }

    fn quad_c(x0: f32, y0: f32, x1: f32, y1: f32, c: u32) -> Vec<u8> {
        let mut v = Vec::new();
        rhw_c(&mut v, x0, y0, c);
        rhw_c(&mut v, x1, y0, c);
        rhw_c(&mut v, x0, y1, c);
        rhw_c(&mut v, x1, y1, c);
        v
    }

    fn tex_desc(w: u32, h: u32, levels: u32, format: u32) -> TextureDesc {
        TextureDesc { width: w, height: h, levels, format, render_target: false, depth_stencil: false, dynamic: false, cube: false }
    }

    fn begin(d: &mut WgpuDevice, color: u32) {
        d.begin_scene().unwrap();
        d.clear(clear::TARGET | clear::ZBUFFER | clear::STENCIL, color, 1.0, 0);
    }

    fn finish(d: &mut WgpuDevice) -> Image {
        d.end_scene().unwrap();
        d.present().unwrap();
        let img = d.read_render_target(None).unwrap();
        let errs = d.take_errors();
        assert!(errs.is_empty(), "wgpu errors: {errs:?}");
        img
    }

    #[test]
    fn clear_and_read_back() {
        let Some(mut d) = dev(40, 30) else { return };
        begin(&mut d, 0xFF33_6699);
        // partial clear: only the viewport
        d.set_viewport(&Viewport { x: 0, y: 0, width: 8, height: 8, min_z: 0.0, max_z: 1.0 });
        d.clear(clear::TARGET, 0xFFFF_0000, 1.0, 0);
        let img = finish(&mut d);
        assert_eq!((img.0, img.1, img.2.len()), (40, 30, 40 * 30 * 4));
        assert_eq!(px(&img, 20, 20), [0x33, 0x66, 0x99, 0xFF]);
        assert_eq!(px(&img, 39, 29), [0x33, 0x66, 0x99, 0xFF]);
        assert_eq!(px(&img, 2, 2), [0xFF, 0, 0, 0xFF]);
        assert_eq!(px(&img, 8, 2), [0x33, 0x66, 0x99, 0xFF]);
    }

    #[test]
    fn pretransformed_triangle() {
        let Some(mut d) = dev(64, 64) else { return };
        begin(&mut d, 0xFF00_0000);
        d.set_fvf(RHW_C);
        let mut v = Vec::new();
        rhw_c(&mut v, 10.0, 10.0, 0xFF00_FF00);
        rhw_c(&mut v, 54.0, 10.0, 0xFF00_FF00);
        rhw_c(&mut v, 10.0, 54.0, 0xFF00_FF00);
        d.draw_primitive_up(PrimitiveType::TriangleList, 1, &v, 20).unwrap();
        let img = finish(&mut d);
        assert_px(&img, 20, 20, [0, 255, 0], 0);
        assert_px(&img, 11, 11, [0, 255, 0], 0);
        assert_px(&img, 50, 50, [0, 0, 0], 0);
        assert_px(&img, 5, 5, [0, 0, 0], 0);
        // D3D9 pixel centres are at integer coordinates: x = 10 covers pixel 10, not 9
        assert_px(&img, 9, 20, [0, 0, 0], 0);
        assert_px(&img, 10, 20, [0, 255, 0], 0);
    }

    #[test]
    fn textured_modulate_and_alpha_test() {
        let Some(mut d) = dev(64, 64) else { return };
        // 2x2 A8R8G8B8 (BGRA bytes): red, transparent green / blue, white
        let t = d.create_texture(&tex_desc(2, 2, 1, fmt::A8R8G8B8)).unwrap();
        let data = [0, 0, 255, 255, 0, 255, 0, 0, 255, 0, 0, 255, 255, 255, 255, 255];
        d.update_texture(t, 0, 0, None, &data, 8).unwrap();
        begin(&mut d, 0xFF20_2020);
        d.set_texture(0, Some(t));
        d.set_render_state(rs::ALPHATESTENABLE, 1);
        d.set_render_state(rs::ALPHAFUNC, 5); // GREATER
        d.set_render_state(rs::ALPHAREF, 0x80);
        d.set_fvf(RHW_CT);
        let v = quad_ct(0.0, 0.0, 64.0, 64.0, 0xFF80_8080);
        d.draw_primitive_up(PrimitiveType::TriangleStrip, 2, &v, 28).unwrap();
        let img = finish(&mut d);
        assert_px(&img, 8, 8, [128, 0, 0], 2);
        assert_px(&img, 40, 8, [0x20, 0x20, 0x20], 0); // alpha 0: discarded
        assert_px(&img, 8, 40, [0, 0, 128], 2);
        assert_px(&img, 40, 40, [128, 128, 128], 2);
    }

    #[test]
    fn transforms_depth_and_culling() {
        let Some(mut d) = dev(64, 64) else { return };
        begin(&mut d, 0xFF00_0000);
        d.set_render_state(rs::LIGHTING, 0);
        d.set_transform(ts::WORLD, &[[0.5, 0.0, 0.0, 0.0], [0.0, 0.5, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]]);
        d.set_transform(ts::VIEW, &IDENTITY);
        d.set_transform(ts::PROJECTION, &IDENTITY);
        d.set_fvf(XYZ_C);
        // world = 2 * NDC; near green first, then the far red one
        let mut v = Vec::new();
        xyz_c(&mut v, -2.0, 2.0, 0.2, 0xFF00_FF00);
        xyz_c(&mut v, 0.0, 2.0, 0.2, 0xFF00_FF00);
        xyz_c(&mut v, -2.0, 0.0, 0.2, 0xFF00_FF00);
        d.draw_primitive_up(PrimitiveType::TriangleList, 1, &v, 16).unwrap();
        let mut v = Vec::new();
        xyz_c(&mut v, -2.0, 2.0, 0.8, 0xFFFF_0000);
        xyz_c(&mut v, 2.0, 2.0, 0.8, 0xFFFF_0000);
        xyz_c(&mut v, -2.0, -2.0, 0.8, 0xFFFF_0000);
        d.draw_primitive_up(PrimitiveType::TriangleList, 1, &v, 16).unwrap();
        // small triangles around pixel centres, clockwise (D3D front) or counter-clockwise
        let tri = |cx: f32, cy: f32, ccw: bool| {
            let w = |px: f32, py: f32| [2.0 * (px / 32.0 - 1.0), 2.0 * (1.0 - py / 32.0)];
            let mut p = [w(cx - 6.0, cy - 6.0), w(cx + 6.0, cy - 6.0), w(cx - 6.0, cy + 6.0)];
            if ccw {
                p.swap(1, 2);
            }
            let mut v = Vec::new();
            for q in p {
                xyz_c(&mut v, q[0], q[1], 0.5, 0xFF00_00FF);
            }
            v
        };
        // default CULLMODE = CCW: counter-clockwise culled
        d.draw_primitive_up(PrimitiveType::TriangleList, 1, &tri(40.0, 40.0, true), 16).unwrap();
        d.set_render_state(rs::CULLMODE, 2); // CW
        d.draw_primitive_up(PrimitiveType::TriangleList, 1, &tri(56.0, 40.0, true), 16).unwrap();
        d.draw_primitive_up(PrimitiveType::TriangleList, 1, &tri(48.0, 56.0, false), 16).unwrap();
        let img = finish(&mut d);
        assert_px(&img, 5, 5, [0, 255, 0], 0);
        assert_px(&img, 40, 5, [255, 0, 0], 0);
        assert_px(&img, 37, 37, [0, 0, 0], 0);
        assert_px(&img, 53, 37, [0, 0, 255], 0);
        assert_px(&img, 45, 53, [0, 0, 0], 0);
    }

    #[test]
    fn triangle_fan_from_vertex_buffer() {
        let Some(mut d) = dev(64, 64) else { return };
        let mut v = Vec::new();
        for (x, y) in [(32.0, 32.0), (8.0, 8.0), (56.0, 8.0), (56.0, 56.0), (8.0, 56.0), (8.0, 8.0)] {
            rhw_c(&mut v, x, y, 0xFFFF_FF00);
        }
        let vb = d.create_vertex_buffer(v.len() as u32, false).unwrap();
        d.update_vertex_buffer(vb, 0, &v).unwrap();
        begin(&mut d, 0xFF00_0000);
        d.set_fvf(RHW_C);
        d.set_stream_source(0, Some(vb), 0, 20);
        d.draw_primitive(PrimitiveType::TriangleFan, 0, 4).unwrap();
        let img = finish(&mut d);
        for (x, y) in [(10, 30), (30, 10), (50, 50), (30, 54), (32, 32)] {
            assert_px(&img, x, y, [255, 255, 0], 0);
        }
        assert_px(&img, 2, 2, [0, 0, 0], 0);
        assert_px(&img, 60, 30, [0, 0, 0], 0);
    }

    #[test]
    fn indexed_draw_and_update_ordering() {
        let Some(mut d) = dev(64, 64) else { return };
        let mut v = vec![0u8; 3 * 20]; // three unused vertices before the quad
        v.extend(quad_c(8.0, 8.0, 24.0, 24.0, 0xFFFF_0000));
        let vb = d.create_vertex_buffer(v.len() as u32, true).unwrap();
        d.update_vertex_buffer(vb, 0, &v).unwrap();
        let idx: Vec<u8> = [9u16, 9, 0, 1, 2, 2, 1, 3].iter().flat_map(|i| i.to_le_bytes()).collect();
        let ib = d.create_index_buffer(idx.len() as u32, fmt::INDEX16, false).unwrap();
        d.update_index_buffer(ib, 0, &idx).unwrap();
        begin(&mut d, 0xFF00_0000);
        d.set_fvf(RHW_C);
        d.set_stream_source(0, Some(vb), 0, 20);
        d.set_indices(Some(ib));
        d.draw_indexed_primitive(PrimitiveType::TriangleList, 3, 0, 4, 2, 2).unwrap();
        // rewrite the quad while the first draw is pending: it must still see the old data
        d.update_vertex_buffer(vb, 60, &quad_c(40.0, 40.0, 56.0, 56.0, 0xFF00_00FF)).unwrap();
        d.draw_indexed_primitive(PrimitiveType::TriangleList, 3, 0, 4, 2, 2).unwrap();
        let img = finish(&mut d);
        assert_px(&img, 16, 16, [255, 0, 0], 0);
        assert_px(&img, 48, 48, [0, 0, 255], 0);
        assert_px(&img, 32, 32, [0, 0, 0], 0);
        assert!(d.stats.flushes >= 2);
    }

    #[test]
    fn alpha_blending() {
        let Some(mut d) = dev(32, 32) else { return };
        begin(&mut d, 0xFF00_00FF);
        d.set_render_state(rs::ALPHABLENDENABLE, 1);
        d.set_render_state(rs::SRCBLEND, 5);
        d.set_render_state(rs::DESTBLEND, 6);
        // untextured: diffuse only, as RW sets the stage without a raster
        d.set_texture_stage_state(0, tss::COLOROP, 3);
        d.set_texture_stage_state(0, tss::ALPHAOP, 3);
        d.set_fvf(RHW_C);
        d.draw_primitive_up(PrimitiveType::TriangleStrip, 2, &quad_c(0.0, 0.0, 16.0, 32.0, 0x80FF_0000), 20).unwrap();
        let img = finish(&mut d);
        assert_px(&img, 8, 16, [128, 0, 127], 2);
        assert_px(&img, 24, 16, [0, 0, 255], 0);
    }

    /// RenderWare's Im2D path (STD-7): a POSITIONT / D3DCOLOR / TEXCOORD declaration (stride 0x1C) from a
    /// vertex buffer, stage 0 SELECTARG2 of DIFFUSE for colour and alpha, no texture bound, vertex alpha blended
    /// over white (the loading screen's fade to black).
    #[test]
    fn im2d_declaration_untextured_uses_vertex_colour() {
        let Some(mut d) = dev(32, 32) else { return };
        let decl = d
            .create_vertex_declaration(&[
                element(0, 0, decltype::FLOAT4, declusage::POSITIONT, 0),
                element(0, 16, decltype::D3DCOLOR, declusage::COLOR, 0),
                element(0, 20, decltype::FLOAT2, declusage::TEXCOORD, 0),
            ])
            .unwrap();
        let mut v = Vec::new();
        for (x, y) in [(0.0, 0.0), (32.0, 0.0), (0.0, 32.0), (32.0, 32.0)] {
            rhw_ct(&mut v, x, y, 0x8000_0000, 0.0, 0.0);
        }
        let vb = d.create_vertex_buffer(v.len() as u32, false).unwrap();
        d.update_vertex_buffer(vb, 0, &v).unwrap();
        begin(&mut d, 0xFFFF_FFFF);
        d.set_render_state(rs::CULLMODE, 1);
        d.set_render_state(rs::LIGHTING, 0);
        d.set_texture(0, None);
        d.set_texture_stage_state(0, tss::COLOROP, 3);
        d.set_texture_stage_state(0, tss::COLORARG2, 0);
        d.set_texture_stage_state(0, tss::ALPHAOP, 3);
        d.set_texture_stage_state(0, tss::ALPHAARG2, 0);
        d.set_render_state(rs::ALPHABLENDENABLE, 1);
        d.set_render_state(rs::SRCBLEND, 5);
        d.set_render_state(rs::DESTBLEND, 6);
        d.set_vertex_declaration(Some(decl));
        d.set_stream_source(0, Some(vb), 0, 0x1C);
        d.draw_primitive(PrimitiveType::TriangleStrip, 0, 2).unwrap();
        let img = finish(&mut d);
        assert_px(&img, 16, 16, [127, 127, 127], 2);
    }

    fn dxt1_block() -> Vec<u8> {
        let bits: u32 = (4..16).map(|i| 1u32 << (2 * i)).sum();
        let mut b = Vec::new();
        b.extend_from_slice(&0xF800u16.to_le_bytes()); // red
        b.extend_from_slice(&0x001Fu16.to_le_bytes()); // blue
        b.extend_from_slice(&bits.to_le_bytes());
        b
    }

    fn dxt1_quad(d: &mut WgpuDevice) -> Image {
        let t = d.create_texture(&tex_desc(4, 4, 0, fmt::DXT1)).unwrap();
        for level in 0..3 {
            d.update_texture(t, 0, level, None, &dxt1_block(), 8).unwrap();
        }
        begin(d, 0xFF00_0000);
        d.set_texture(0, Some(t));
        d.set_fvf(fvf::XYZRHW | 0x100);
        let mut v = Vec::new();
        for (x, y) in [(0.0, 0.0), (64.0, 0.0), (0.0, 64.0), (64.0, 64.0)] {
            f(&mut v, &[x, y, 0.5, 1.0, x / 64.0, y / 64.0]);
        }
        d.draw_primitive_up(PrimitiveType::TriangleStrip, 2, &v, 24).unwrap();
        let img = finish(d);
        d.release_texture(t);
        img
    }

    #[test]
    fn dxt1_texture() {
        let Some(mut d) = dev(64, 64) else { return };
        eprintln!("BC upload: {}", d.uses_bc());
        let img = dxt1_quad(&mut d);
        assert_px(&img, 32, 4, [255, 0, 0], 0);
        assert_px(&img, 32, 40, [0, 0, 255], 0);
        // software decode path
        d.bc = false;
        let img = dxt1_quad(&mut d);
        assert_px(&img, 32, 4, [255, 0, 0], 0);
        assert_px(&img, 32, 40, [0, 0, 255], 0);
    }

    #[test]
    fn directional_lighting() {
        let Some(mut d) = dev(32, 32) else { return };
        begin(&mut d, 0xFF00_0000);
        d.set_material(&Material {
            diffuse: [1.0, 0.5, 0.0, 1.0],
            ambient: [0.2, 0.2, 0.2, 1.0],
            ..Default::default()
        });
        d.set_render_state(rs::AMBIENT, 0xFF80_8080);
        d.set_light(0, &Light { kind: 3, diffuse: [1.0, 1.0, 1.0, 1.0], direction: [0.0, 0.0, 1.0], ..Default::default() });
        d.light_enable(0, true);
        d.set_fvf(XYZ_N);
        let mut v = Vec::new();
        for (x, y) in [(-1.0, 1.0), (1.0, 1.0), (-1.0, -1.0)] {
            f(&mut v, &[x, y, 0.5, 0.0, 0.0, -1.0]);
        }
        d.draw_primitive_up(PrimitiveType::TriangleList, 1, &v, 24).unwrap();
        let img = finish(&mut d);
        // diffuse 1 * (1, 0.5, 0) + ambient 0.2 * 0.502
        assert_px(&img, 5, 5, [255, 153, 26], 2);
    }

    #[test]
    fn linear_fog_vertex_and_table() {
        for table in [false, true] {
            let Some(mut d) = dev(32, 32) else { return };
            begin(&mut d, 0xFF00_0000);
            d.set_render_state(rs::LIGHTING, 0);
            d.set_render_state(rs::FOGENABLE, 1);
            d.set_render_state(if table { rs::FOGTABLEMODE } else { rs::FOGVERTEXMODE }, 3);
            d.set_render_state(rs::FOGSTART, 0.0f32.to_bits());
            d.set_render_state(rs::FOGEND, 1.0f32.to_bits());
            d.set_render_state(rs::FOGCOLOR, 0xFF00_00FF);
            d.set_fvf(XYZ_C);
            let mut v = Vec::new();
            for (x, y) in [(-1.0, 1.0), (1.0, 1.0), (-1.0, -1.0)] {
                xyz_c(&mut v, x, y, 0.25, 0xFFFF_FFFF);
            }
            d.draw_primitive_up(PrimitiveType::TriangleList, 1, &v, 16).unwrap();
            let img = finish(&mut d);
            assert_px(&img, 5, 5, [191, 191, 255], 2);
        }
    }

    #[test]
    fn render_target_texture() {
        let Some(mut d) = dev(32, 32) else { return };
        let rt = d
            .create_texture(&TextureDesc { width: 16, height: 16, levels: 1, format: fmt::A8R8G8B8, render_target: true, depth_stencil: false, dynamic: false, cube: false })
            .unwrap();
        d.set_render_target(Some(rt), None);
        d.clear(clear::TARGET | clear::ZBUFFER, 0xFF00_FF00, 1.0, 0);
        let rimg = d.read_render_target(Some(rt)).unwrap();
        assert_eq!((rimg.0, rimg.1), (16, 16));
        assert_px(&rimg, 8, 8, [0, 255, 0], 0);
        d.set_render_target(None, None);
        begin(&mut d, 0xFF00_0000);
        d.set_texture(0, Some(rt));
        d.set_fvf(RHW_CT);
        d.draw_primitive_up(PrimitiveType::TriangleStrip, 2, &quad_ct(0.0, 0.0, 16.0, 16.0, 0xFFFF_FFFF), 28).unwrap();
        let img = finish(&mut d);
        assert_px(&img, 8, 8, [0, 255, 0], 0);
        assert_px(&img, 24, 24, [0, 0, 0], 0);
    }

    /// A render-target texture without alpha (X8R8G8B8) keeps alpha 1: clears and draws do not write
    /// alpha, and a StretchRect from a target with alpha does not carry it over.
    #[test]
    fn render_target_without_alpha() {
        let Some(mut d) = dev(32, 32) else { return };
        let desc = |format| TextureDesc { width: 16, height: 16, levels: 1, format, render_target: true, depth_stencil: false, dynamic: false, cube: false };
        let x8 = d.create_texture(&desc(fmt::X8R8G8B8)).unwrap();
        let a8 = d.create_texture(&desc(fmt::A8R8G8B8)).unwrap();
        let alpha = |d: &mut WgpuDevice, id| d.read_render_target(Some(id)).unwrap().2[(8 * 16 + 8) * 4 + 3];
        assert_eq!(alpha(&mut d, x8), 255, "a new X8 target");
        d.set_render_target(Some(x8), None);
        d.clear(clear::TARGET, 0x0000_FF00, 1.0, 0);
        assert_eq!(alpha(&mut d, x8), 255, "cleared with alpha 0");
        d.set_render_target(Some(x8), None);
        d.set_fvf(RHW_CT);
        d.draw_primitive_up(PrimitiveType::TriangleStrip, 2, &quad_ct(0.0, 0.0, 16.0, 16.0, 0x00FF_FFFF), 28).unwrap();
        assert_eq!(alpha(&mut d, x8), 255, "drawn with alpha 0");
        d.set_render_target(Some(a8), None);
        d.clear(clear::TARGET, 0x0000_FF00, 1.0, 0);
        assert_eq!(alpha(&mut d, a8), 0);
        d.stretch_rect(Some(a8), None, Some(x8), None, false).unwrap();
        let img = d.read_render_target(Some(x8)).unwrap();
        assert_eq!(img.2[(8 * 16 + 8) * 4..(8 * 16 + 8) * 4 + 4], [0, 255, 0, 255], "copied from A8R8G8B8");
    }

    fn tokens(src: &str) -> Vec<u32> {
        crate::sm::asm::assemble(src).unwrap_or_else(|e| panic!("{e}\n{src}"))
    }

    fn element(stream: u16, offset: u16, decl_type: u8, usage: u8, usage_index: u8) -> VertexElement {
        VertexElement { stream, offset, decl_type, method: 0, usage, usage_index }
    }

    /// Clip-space quad (x0..x1 × y0..y1) as a 4-vertex strip of `XYZ` + extra floats per vertex.
    fn clip_quad(x0: f32, y0: f32, x1: f32, y1: f32, extra: &[f32]) -> Vec<u8> {
        let mut v = Vec::new();
        for (x, y) in [(x0, y1), (x1, y1), (x0, y0), (x1, y0)] {
            f(&mut v, &[x, y, 0.5]);
            f(&mut v, extra);
        }
        v
    }

    const IDENTITY_ROWS: [[f32; 4]; 4] = [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];

    #[test]
    fn shader_creation() {
        let Some(mut d) = dev(8, 8) else { return };
        assert_eq!(d.caps().vertex_shader_version, 0xFFFE_0200);
        // no end token / wrong kind / an instruction the translator does not have
        assert!(matches!(d.create_vertex_shader(&[0xFFFE_0101]), Err(D3dError::InvalidCall(_))));
        assert!(matches!(d.create_vertex_shader(&tokens("ps_1_1\nmov r0, v0\n")), Err(D3dError::InvalidCall(_))));
        assert!(matches!(d.create_pixel_shader(&tokens("ps_1_1\ntex t0\ntexbem t1, t0\nmov r0, t1\n")), Err(D3dError::NotAvailable(_))));
        let v = d.create_vertex_shader(&tokens("vs_1_1\ndcl_position v0\nmov oPos, v0\n")).unwrap();
        let p = d.create_pixel_shader(&tokens("ps_1_1\nmov r0, v0\n")).unwrap();
        d.set_vertex_shader(Some(p)); // a pixel shader is not a vertex shader
        assert_eq!(d.vs, None);
        d.set_vertex_shader(Some(v));
        d.release_shader(v);
        assert_eq!(d.vs, None);
        d.release_shader(p);
    }

    /// A vs_2_0 shader over a two-stream declaration, with float, integer and bool constants
    /// changed between draws, and the fixed-function pixel stage.
    #[test]
    fn vertex_shader_streams_and_constants() {
        let Some(mut d) = dev(32, 32) else { return };
        let vs = d
            .create_vertex_shader(&tokens(
                "vs_2_0
dcl_position v0
dcl_color v4
mov r0, c5
rep i0
add r0, r0, c6
endrep
if b2
mov r0.x, c7.x
endif
m4x4 oPos, v0, c0
mul oD0, v4, r0
",
            ))
            .unwrap();
        let decl = d.create_vertex_declaration(&[element(0, 0, decltype::FLOAT3, declusage::POSITION, 0), element(1, 0, decltype::D3DCOLOR, declusage::COLOR, 0)]).unwrap();
        let pos = [clip_quad(-1.0, -1.0, 0.0, 1.0, &[]), clip_quad(0.0, -1.0, 1.0, 1.0, &[])].concat();
        let vb0 = d.create_vertex_buffer(pos.len() as u32, false).unwrap();
        d.update_vertex_buffer(vb0, 0, &pos).unwrap();
        let mut cols = Vec::new();
        for _ in 0..8 {
            u(&mut cols, 0xFFFF_00FF); // R = 1, G = 0, B = 1
        }
        let vb1 = d.create_vertex_buffer(cols.len() as u32, false).unwrap();
        d.update_vertex_buffer(vb1, 0, &cols).unwrap();
        begin(&mut d, 0xFF00_0000);
        d.set_render_state(rs::CULLMODE, 1);
        d.set_render_state(rs::LIGHTING, 0);
        d.set_vertex_declaration(Some(decl));
        d.set_vertex_shader(Some(vs));
        d.set_stream_source(0, Some(vb0), 0, 12);
        d.set_stream_source(1, Some(vb1), 0, 4);
        d.set_vertex_shader_constant_f(0, &IDENTITY_ROWS);
        d.set_vertex_shader_constant_f(5, &[[0.0, 0.0, 0.0, 1.0], [0.25, 0.25, 0.25, 0.0], [1.0, 0.0, 0.0, 0.0]]);
        // left: r0 = 2 * 0.25, then b2 sets r0.x = 1 → (1, 0, 0.5)
        d.set_vertex_shader_constant_i(0, &[[2, 0, 1, 0]]);
        d.set_vertex_shader_constant_b(2, &[true]);
        d.draw_primitive(PrimitiveType::TriangleStrip, 0, 2).unwrap();
        // right: r0 = 4 * 0.25 = 1 → (1, 0, 1)
        d.set_vertex_shader_constant_i(0, &[[4, 0, 1, 0]]);
        d.set_vertex_shader_constant_b(2, &[false]);
        d.draw_primitive(PrimitiveType::TriangleStrip, 4, 2).unwrap();
        let img = finish(&mut d);
        assert_px(&img, 8, 16, [255, 0, 128], 1);
        assert_px(&img, 24, 16, [255, 0, 255], 1);
    }

    /// The RpMatFX shape: a ps_1_1 shader after fixed-function vertex processing, with constant
    /// clamping, then D3D9's alpha test on the shader's output.
    #[test]
    fn pixel_shader_after_fixed_function() {
        let Some(mut d) = dev(32, 32) else { return };
        let ps = d.create_pixel_shader(&tokens("ps_1_1\ntex t0\nmul r0, t0, c0\nmul r0, r0, c1\n")).unwrap();
        let tex = d.create_texture(&TextureDesc { width: 2, height: 2, levels: 1, format: fmt::A8R8G8B8, render_target: false, depth_stencil: false, dynamic: false, cube: false }).unwrap();
        d.update_texture(tex, 0, 0, None, &[0xFF; 16], 8).unwrap();
        begin(&mut d, 0xFF00_0000);
        d.set_render_state(rs::CULLMODE, 1);
        d.set_texture(0, Some(tex));
        d.set_fvf(RHW_CT);
        d.set_pixel_shader(Some(ps));
        // c1 = 2 is clamped to 1 in ps_1_x
        d.set_pixel_shader_constant_f(0, &[[1.0, 0.5, 0.25, 0.75], [2.0, 2.0, 2.0, 2.0]]);
        d.set_render_state(rs::ALPHATESTENABLE, 1);
        d.set_render_state(rs::ALPHAFUNC, 5); // GREATER
        d.set_render_state(rs::ALPHAREF, 0x80);
        d.draw_primitive_up(PrimitiveType::TriangleStrip, 2, &quad_ct(0.0, 0.0, 16.0, 32.0, 0xFFFF_FFFF), 28).unwrap();
        // alpha 0.25: rejected by the alpha test
        d.set_pixel_shader_constant_f(0, &[[1.0, 0.5, 0.25, 0.25]]);
        d.draw_primitive_up(PrimitiveType::TriangleStrip, 2, &quad_ct(16.0, 0.0, 32.0, 32.0, 0xFFFF_FFFF), 28).unwrap();
        let img = finish(&mut d);
        assert_px(&img, 8, 16, [255, 128, 64], 1);
        assert_px(&img, 24, 16, [0, 0, 0], 0);
    }

    /// Cube textures sampled by a ps_2_0 shader with texture coordinates from a vertex shader.
    #[test]
    fn cube_texture_through_shaders() {
        let Some(mut d) = dev(32, 32) else { return };
        let cube = d.create_texture(&TextureDesc { width: 1, height: 1, levels: 1, format: fmt::A8R8G8B8, render_target: false, depth_stencil: false, dynamic: false, cube: true }).unwrap();
        // +X red, -X green, +Y blue, -Y yellow, +Z cyan, -Z magenta (B, G, R, A bytes)
        let faces: [[u8; 4]; 6] = [[0, 0, 255, 255], [0, 255, 0, 255], [255, 0, 0, 255], [0, 255, 255, 255], [255, 255, 0, 255], [255, 0, 255, 255]];
        for (k, c) in faces.iter().enumerate() {
            d.update_texture(cube, k as u32, 0, None, c, 4).unwrap();
        }
        assert!(d.update_texture(cube, 6, 0, None, &faces[0], 4).is_err());
        let vs = d.create_vertex_shader(&tokens("vs_1_1\ndcl_position v0\ndcl_texcoord v5\nmov oPos, v0\nmov oT0, v5\n")).unwrap();
        let ps = d.create_pixel_shader(&tokens("ps_2_0\ndcl t0.xyz\ndcl_cube s0\ntexld r0, t0, s0\nmov oC0, r0\n")).unwrap();
        let decl = d.create_vertex_declaration(&[element(0, 0, decltype::FLOAT3, declusage::POSITION, 0), element(0, 12, decltype::FLOAT3, declusage::TEXCOORD, 0)]).unwrap();
        begin(&mut d, 0xFF00_0000);
        d.set_render_state(rs::CULLMODE, 1);
        d.set_vertex_declaration(Some(decl));
        d.set_vertex_shader(Some(vs));
        d.set_pixel_shader(Some(ps));
        d.set_texture(0, Some(cube));
        d.draw_primitive_up(PrimitiveType::TriangleStrip, 2, &clip_quad(-1.0, -1.0, 0.0, 1.0, &[0.9, 0.1, -0.2]), 24).unwrap();
        d.draw_primitive_up(PrimitiveType::TriangleStrip, 2, &clip_quad(0.0, -1.0, 1.0, 1.0, &[0.1, -0.3, -0.8]), 24).unwrap();
        let img = finish(&mut d);
        assert_px(&img, 8, 16, [255, 0, 0], 0);
        assert_px(&img, 24, 16, [255, 0, 255], 0);
    }

    /// A composite scene; written as PNG to `$SA_D3D9_PNG` when set.
    #[test]
    fn composite_scene() {
        let Some(mut d) = dev(256, 256) else { return };
        // 8x8 checker with a transparent hole, X8R8G8B8 + A1R5G5B5 + R5G6B5 textures
        let mut checker = Vec::new();
        for y in 0..8u32 {
            for x in 0..8u32 {
                let on = (x + y) % 2 == 0;
                let a = if (2..6).contains(&x) && (2..6).contains(&y) { 0u16 } else { 0x8000 };
                let c: u16 = if on { 0x7C00 } else { 0x7FFF };
                checker.extend_from_slice(&(a | c).to_le_bytes());
            }
        }
        let t1 = d.create_texture(&tex_desc(8, 8, 1, fmt::A1R5G5B5)).unwrap();
        d.update_texture(t1, 0, 0, None, &checker, 16).unwrap();
        let grad: Vec<u8> = (0..16u32 * 16).flat_map(|i| (((i % 16) as u16 * 2) << 11 | ((i / 16) as u16 * 4) << 5 | 0x10).to_le_bytes()).collect();
        let t2 = d.create_texture(&tex_desc(16, 16, 1, fmt::R5G6B5)).unwrap();
        d.update_texture(t2, 0, 0, None, &grad, 32).unwrap();

        begin(&mut d, 0xFF10_1828);
        // fan: octagon with per-vertex colours
        d.set_fvf(RHW_C);
        let mut fan = Vec::new();
        rhw_c(&mut fan, 64.0, 64.0, 0xFFFF_FFFF);
        for k in 0..=8 {
            let a = k as f32 * std::f32::consts::TAU / 8.0;
            let c = [0xFFFF_0000, 0xFFFF_FF00, 0xFF00_FF00, 0xFF00_FFFF, 0xFF00_00FF, 0xFFFF_00FF, 0xFFFF_8000, 0xFF80_FF80][k % 8];
            rhw_c(&mut fan, 64.0 + 50.0 * a.cos(), 64.0 + 50.0 * a.sin(), c);
        }
        d.draw_primitive_up(PrimitiveType::TriangleFan, 8, &fan, 20).unwrap();
        // checker with alpha test, linear filtering
        d.set_fvf(RHW_CT);
        d.set_texture(0, Some(t1));
        d.set_render_state(rs::ALPHATESTENABLE, 1);
        d.set_render_state(rs::ALPHAFUNC, 7);
        d.set_render_state(rs::ALPHAREF, 0x80);
        d.draw_primitive_up(PrimitiveType::TriangleStrip, 2, &quad_ct(140.0, 14.0, 240.0, 114.0, 0xFFFF_FFFF), 28).unwrap();
        d.set_render_state(rs::ALPHATESTENABLE, 0);
        // gradient texture, MODULATE2X, bilinear
        d.set_texture(0, Some(t2));
        d.set_texture_stage_state(0, tss::COLOROP, 5);
        d.set_sampler_state(0, samp::MAGFILTER, 2);
        d.set_sampler_state(0, samp::MINFILTER, 2);
        d.draw_primitive_up(PrimitiveType::TriangleStrip, 2, &quad_ct(14.0, 140.0, 114.0, 240.0, 0xFF80_8080), 28).unwrap();
        d.set_texture_stage_state(0, tss::COLOROP, 4);
        d.set_texture(0, None);
        // translucent overlay across the fan and the checker
        d.set_texture_stage_state(0, tss::ALPHAOP, 3);
        d.set_render_state(rs::ALPHABLENDENABLE, 1);
        d.set_render_state(rs::SRCBLEND, 5);
        d.set_render_state(rs::DESTBLEND, 6);
        d.set_fvf(RHW_C);
        d.draw_primitive_up(PrimitiveType::TriangleStrip, 2, &quad_c(90.0, 40.0, 170.0, 90.0, 0x8000_80FF), 20).unwrap();
        d.set_render_state(rs::ALPHABLENDENABLE, 0);
        // lit, fogged 3D triangles in the lower right
        let proj = {
            // D3DXMatrixPerspectiveFovLH(fov 60°, aspect 1, near 1, far 100)
            let ys = 1.0 / (std::f32::consts::FRAC_PI_6).tan();
            let q = 100.0 / 99.0;
            [[ys, 0.0, 0.0, 0.0], [0.0, ys, 0.0, 0.0], [0.0, 0.0, q, 1.0], [0.0, 0.0, -q, 0.0]]
        };
        d.set_transform(ts::PROJECTION, &proj);
        d.set_transform(ts::VIEW, &IDENTITY);
        d.set_viewport(&Viewport { x: 128, y: 128, width: 128, height: 128, min_z: 0.0, max_z: 1.0 });
        d.set_material(&Material { diffuse: [1.0, 0.8, 0.6, 1.0], ambient: [1.0; 4], ..Default::default() });
        d.set_render_state(rs::AMBIENT, 0xFF20_2020);
        d.set_light(0, &Light { kind: 3, diffuse: [1.0; 4], direction: [0.5, -0.5, 1.0], ..Default::default() });
        d.light_enable(0, true);
        d.set_render_state(rs::FOGENABLE, 1);
        d.set_render_state(rs::FOGTABLEMODE, 3);
        d.set_render_state(rs::FOGSTART, 4.0f32.to_bits());
        d.set_render_state(rs::FOGEND, 14.0f32.to_bits());
        d.set_render_state(rs::FOGCOLOR, 0xFF10_1828);
        d.set_render_state(rs::CULLMODE, 1);
        d.set_fvf(XYZ_N);
        for (k, z) in [3.0f32, 6.0, 9.0, 12.0].iter().enumerate() {
            let x = -2.0 + k as f32 * 1.6;
            let mut v = Vec::new();
            // a tilted triangle facing the camera
            for (dx, dy) in [(0.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
                f(&mut v, &[x + dx * 0.8, dy * 0.8 + 0.3 * k as f32 - 0.5, *z, -0.3, 0.2, -1.0]);
            }
            d.set_transform(ts::WORLD, &IDENTITY);
            d.draw_primitive_up(PrimitiveType::TriangleList, 1, &v, 24).unwrap();
        }
        let img = finish(&mut d);
        if let Ok(path) = std::env::var("SA_D3D9_PNG") {
            let file = std::fs::File::create(&path).expect("create png");
            let mut enc = png::Encoder::new(std::io::BufWriter::new(file), img.0, img.1);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().expect("png header");
            w.write_image_data(&img.2).expect("png data");
            eprintln!("wrote {path}");
        }
        // the fan centre is white, the checker hole shows the clear colour
        assert_px(&img, 64, 64, [255, 255, 255], 8);
        assert_px(&img, 190, 64, [0x10, 0x18, 0x28], 0);
    }
}
