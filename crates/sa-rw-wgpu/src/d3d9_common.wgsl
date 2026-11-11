// Shared declarations of the D3D9 fixed-function emulation (`d3d9.rs`).
// Must match `Uniforms`/`LightU` in d3d9.rs byte for byte.

struct Light {
    diffuse: vec4f,
    ambient: vec4f,
    // xyz position (world), w = D3DLIGHTTYPE (1 point, 2 spot, 3 directional)
    pos: vec4f,
    // xyz direction (world), w = range
    dir: vec4f,
    // xyz attenuation0/1/2, w = falloff
    att: vec4f,
    // x = cos(theta / 2), y = cos(phi / 2)
    spot: vec4f,
}

struct U {
    // D3D row-vector matrices uploaded row by row: WGSL sees the transpose, so `m * v` == `v * M`.
    wvp: mat4x4f,
    world: mat4x4f,
    wv: mat4x4f,
    // D3D viewport x, y, width, height (pixels)
    viewport: vec4f,
    // min_z, max_z, clear depth, -
    vpz: vec4f,
    // x = F_* bits, y = ALPHAFUNC, z = ALPHAREF, w = number of active lights
    flags: vec4u,
    // per stage s (0, 1): [3s] = colorop, colorarg1, colorarg2, colorarg0
    //                     [3s+1] = alphaop, alphaarg1, alphaarg2, alphaarg0
    //                     [3s+2] = texcoord index, -, -, -
    stages: array<vec4u, 6>,
    tfactor: vec4f,
    fog_color: vec4f,
    // start, end, density, -
    fog: vec4f,
    // x = FOGTABLEMODE, y = FOGVERTEXMODE
    fog_mode: vec4u,
    ambient: vec4f,
    mat_diffuse: vec4f,
    mat_ambient: vec4f,
    mat_specular: vec4f,
    mat_emissive: vec4f,
    // diffuse, specular, ambient, emissive material sources (D3DMCS_*)
    mat_src: vec4u,
    lights: array<Light, 8>,
}

@group(0) @binding(0) var<uniform> u: U;

const F_PRETRANSFORMED: u32 = 1u;
const F_LIGHTING: u32 = 2u;
const F_COLORVERTEX: u32 = 4u;
const F_HAS_C0: u32 = 8u;
const F_HAS_C1: u32 = 16u;
const F_FOG: u32 = 32u;
const F_ALPHATEST: u32 = 64u;
const F_HAS_NORMAL: u32 = 128u;
const F_SPECULAR: u32 = 256u;
const F_RANGEFOG: u32 = 512u;

// D3DFOGMODE: 1 EXP, 2 EXP2, 3 LINEAR. Returns the fog factor (1 = no fog).
fn fog_of(mode: u32, d: f32) -> f32 {
    switch mode {
        case 1u: {
            return exp(-d * u.fog.z);
        }
        case 2u: {
            let x = d * u.fog.z;
            return exp(-x * x);
        }
        case 3u: {
            let span = u.fog.y - u.fog.x;
            if abs(span) < 1e-12 {
                return select(1.0, 0.0, d >= u.fog.y);
            }
            return clamp((u.fog.y - d) / span, 0.0, 1.0);
        }
        default: {
            return 1.0;
        }
    }
}

// D3DCMP: 1 NEVER, 2 LESS, 3 EQUAL, 4 LESSEQUAL, 5 GREATER, 6 NOTEQUAL, 7 GREATEREQUAL, 8 ALWAYS.
fn cmp(func: u32, a: f32, r: f32) -> bool {
    switch func {
        case 1u: {
            return false;
        }
        case 2u: {
            return a < r;
        }
        case 3u: {
            return a == r;
        }
        case 4u: {
            return a <= r;
        }
        case 5u: {
            return a > r;
        }
        case 6u: {
            return a != r;
        }
        case 7u: {
            return a >= r;
        }
        default: {
            return true;
        }
    }
}

// What D3D9 applies after the pixel stage (texture stages or a ps_1_x/ps_2_x pixel shader): fog
// blending (`fog`: x = distance for table fog, y = vertex fog factor) and the alpha test.
fn ff_fog_alpha(c: vec4f, fog: vec2f) -> vec4f {
    var cur = c;
    let f = u.flags.x;
    if (f & F_FOG) != 0u {
        var ff = fog.y;
        if u.fog_mode.x != 0u {
            ff = fog_of(u.fog_mode.x, fog.x);
        }
        cur = vec4f(mix(u.fog_color.rgb, cur.rgb, clamp(ff, 0.0, 1.0)), cur.a);
    }
    if (f & F_ALPHATEST) != 0u {
        if !cmp(u.flags.y, round(clamp(cur.a, 0.0, 1.0) * 255.0), f32(u.flags.z)) {
            discard;
        }
    }
    return cur;
}
