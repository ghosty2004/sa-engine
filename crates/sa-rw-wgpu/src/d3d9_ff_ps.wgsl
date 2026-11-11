// D3D9 fixed-function texture stages (`d3d9.rs`), after d3d9_common.wgsl. The fragment entry point
// is generated: `ff_pixel` on the fixed-function or vertex-shader varyings, then `ff_fog_alpha`.

@group(1) @binding(0) var tex0: texture_2d<f32>;
@group(1) @binding(1) var smp0: sampler;
@group(1) @binding(2) var tex1: texture_2d<f32>;
@group(1) @binding(3) var smp1: sampler;

// D3DTA_*: 0 DIFFUSE, 1 CURRENT, 2 TEXTURE, 3 TFACTOR, 4 SPECULAR, 5 TEMP, 6 CONSTANT;
// 0x10 COMPLEMENT, 0x20 ALPHAREPLICATE.
fn arg(a: u32, cur: vec4f, dif: vec4f, spec: vec4f, tex: vec4f) -> vec4f {
    var v = vec4f(0.0);
    switch a & 0xfu {
        case 0u: {
            v = dif;
        }
        case 1u: {
            v = cur;
        }
        case 2u: {
            v = tex;
        }
        case 3u, 6u: {
            v = u.tfactor;
        }
        case 4u: {
            v = spec;
        }
        default: {
            v = vec4f(0.0);
        }
    }
    if (a & 0x20u) != 0u {
        v = vec4f(v.a);
    }
    if (a & 0x10u) != 0u {
        v = vec4f(1.0) - v;
    }
    return v;
}

// D3DTEXTUREOP.
fn op(o: u32, a0: vec4f, a1: vec4f, a2: vec4f, cur: vec4f, dif: vec4f, tex: vec4f) -> vec4f {
    switch o {
        case 2u: {
            return a1;
        }
        case 3u: {
            return a2;
        }
        case 4u: {
            return a1 * a2;
        }
        case 5u: {
            return a1 * a2 * 2.0;
        }
        case 6u: {
            return a1 * a2 * 4.0;
        }
        case 7u: {
            return a1 + a2;
        }
        case 8u: {
            return a1 + a2 - vec4f(0.5);
        }
        case 9u: {
            return (a1 + a2 - vec4f(0.5)) * 2.0;
        }
        case 10u: {
            return a1 - a2;
        }
        case 11u: {
            return a1 + a2 - a1 * a2;
        }
        case 12u: {
            return mix(a2, a1, dif.a);
        }
        case 13u: {
            return mix(a2, a1, tex.a);
        }
        case 14u: {
            return mix(a2, a1, u.tfactor.a);
        }
        case 15u: {
            return a1 + a2 * (1.0 - tex.a);
        }
        case 16u: {
            return mix(a2, a1, cur.a);
        }
        case 18u: {
            return vec4f(a1.rgb + a1.a * a2.rgb, a1.a);
        }
        case 19u: {
            return vec4f(a1.rgb * a2.rgb + vec3f(a1.a), a1.a);
        }
        case 20u: {
            return vec4f((1.0 - a1.a) * a2.rgb + a1.rgb, a1.a);
        }
        case 21u: {
            return vec4f((vec3f(1.0) - a1.rgb) * a2.rgb + vec3f(a1.a), a1.a);
        }
        case 24u: {
            return vec4f(clamp(4.0 * dot(a1.rgb - vec3f(0.5), a2.rgb - vec3f(0.5)), 0.0, 1.0));
        }
        case 25u: {
            return a0 + a1 * a2;
        }
        case 26u: {
            return a0 * a1 + (vec4f(1.0) - a0) * a2;
        }
        default: {
            return a1;
        }
    }
}

fn stage(s: u32, cur: vec4f, dif: vec4f, spec: vec4f, tex: vec4f) -> vec4f {
    let c = u.stages[s * 3u];
    let al = u.stages[s * 3u + 1u];
    let col = op(c.x, arg(c.w, cur, dif, spec, tex), arg(c.y, cur, dif, spec, tex), arg(c.z, cur, dif, spec, tex), cur, dif, tex);
    var alpha = cur.a;
    if al.x != 1u {
        alpha = op(al.x, arg(al.w, cur, dif, spec, tex), arg(al.y, cur, dif, spec, tex), arg(al.z, cur, dif, spec, tex), cur, dif, tex).a;
    }
    return clamp(vec4f(col.rgb, alpha), vec4f(0.0), vec4f(1.0));
}

// Texture stages 0 and 1 plus the specular add; `t0`/`t1` are texture coordinate sets 0 and 1
// (TEXCOORDINDEX picks one of them per stage).
fn ff_pixel(c0: vec4f, c1: vec4f, t0: vec2f, t1: vec2f) -> vec4f {
    // sample first: textureSample needs uniform control flow
    let uv0 = select(t0, t1, u.stages[2].x == 1u);
    let uv1 = select(t0, t1, u.stages[5].x == 1u);
    let tx0 = textureSample(tex0, smp0, uv0);
    let tx1 = textureSample(tex1, smp1, uv1);
    var cur = c0;
    if u.stages[0].x != 1u {
        cur = stage(0u, cur, c0, c1, tx0);
        if u.stages[3].x != 1u {
            cur = stage(1u, cur, c0, c1, tx1);
        }
    }
    if (u.flags.x & F_SPECULAR) != 0u {
        cur = vec4f(min(cur.rgb + c1.rgb, vec3f(1.0)), cur.a);
    }
    return cur;
}
