// D3D9 fixed-function vertex processing (`d3d9.rs`). Prepended at runtime by d3d9_common.wgsl and
// a generated header that declares `struct VIn` (the vertex layout of the draw) and
// `fn load_attrs(i: VIn) -> Attrs` (D3DDECLTYPE expansion with D3D's (0, 0, 0, 1) defaults,
// D3DCOLOR swizzled from BGRA). The entry point is generated too: it returns `ff_vertex(i)`
// directly, or converts it to the shader-path varyings when a pixel shader is set.

struct Attrs {
    pos: vec4f,
    normal: vec3f,
    c0: vec4f,
    c1: vec4f,
    t0: vec4f,
    t1: vec4f,
}

struct VOut {
    @builtin(position) pos: vec4f,
    @location(0) c0: vec4f,
    @location(1) c1: vec4f,
    @location(2) t0: vec2f,
    @location(3) t1: vec2f,
    // x = fog distance (table fog), y = fog factor (vertex fog / specular alpha)
    @location(4) fog: vec2f,
}

fn mat_color(src: u32, m: vec4f, c0: vec4f, c1: vec4f) -> vec4f {
    let f = u.flags.x;
    if (f & F_COLORVERTEX) != 0u {
        if src == 1u && (f & F_HAS_C0) != 0u {
            return c0;
        }
        if src == 2u && (f & F_HAS_C1) != 0u {
            return c1;
        }
    }
    return m;
}

fn ff_vertex(i: VIn) -> VOut {
    let a = load_attrs(i);
    let f = u.flags.x;
    var o: VOut;
    var c0 = a.c0;
    var c1 = a.c1;
    let spec_fog = select(1.0, a.c1.a, (f & F_HAS_C1) != 0u);
    if (f & F_PRETRANSFORMED) != 0u {
        // XYZRHW: screen pixels (D3D9 pixel centres are at integer coordinates, wgpu's at +0.5)
        let rhw = select(a.pos.w, 1.0, a.pos.w == 0.0);
        let w = 1.0 / rhw;
        let nx = (a.pos.x + 0.5 - u.viewport.x) / u.viewport.z * 2.0 - 1.0;
        let ny = 1.0 - (a.pos.y + 0.5 - u.viewport.y) / u.viewport.w * 2.0;
        let nz = (a.pos.z - u.vpz.x) / max(u.vpz.y - u.vpz.x, 1e-6);
        o.pos = vec4f(nx, ny, nz, 1.0) * w;
        o.fog = vec2f(w, spec_fog);
    } else {
        let p = vec4f(a.pos.xyz, 1.0);
        var clip = u.wvp * p;
        // D3D9 half-pixel convention
        clip.x += clip.w / u.viewport.z;
        clip.y -= clip.w / u.viewport.w;
        o.pos = clip;
        let vpos = u.wv * p;
        var d = abs(vpos.z);
        if (f & F_RANGEFOG) != 0u {
            d = length(vpos.xyz);
        }
        var vf = spec_fog;
        if u.fog_mode.y != 0u {
            vf = fog_of(u.fog_mode.y, d);
        }
        o.fog = vec2f(d, vf);
        if (f & F_LIGHTING) != 0u {
            let wpos = (u.world * p).xyz;
            var n = vec3f(0.0);
            if (f & F_HAS_NORMAL) != 0u {
                n = (u.world * vec4f(a.normal, 0.0)).xyz;
                let len = length(n);
                // always normalised (approximation: D3D only does it with NORMALIZENORMALS
                // and transforms normals by the inverse transpose of world*view)
                if len > 1e-12 {
                    n = n / len;
                }
            }
            let md = mat_color(u.mat_src.x, u.mat_diffuse, a.c0, a.c1);
            let ma = mat_color(u.mat_src.z, u.mat_ambient, a.c0, a.c1);
            let me = mat_color(u.mat_src.w, u.mat_emissive, a.c0, a.c1);
            var amb = u.ambient.rgb;
            var dif = vec3f(0.0);
            for (var k = 0u; k < u.flags.w; k++) {
                let l = u.lights[k];
                let kind = u32(l.pos.w);
                var ldir = vec3f(0.0, 0.0, 1.0);
                var att = 1.0;
                if kind == 3u {
                    ldir = -normalize(l.dir.xyz);
                } else {
                    let v = l.pos.xyz - wpos;
                    let dist = length(v);
                    if dist > l.dir.w {
                        continue;
                    }
                    ldir = v / max(dist, 1e-6);
                    att = 1.0 / max(l.att.x + l.att.y * dist + l.att.z * dist * dist, 1e-6);
                    if kind == 2u {
                        let rho = dot(normalize(l.dir.xyz), -ldir);
                        if rho <= l.spot.y {
                            att = 0.0;
                        } else if rho < l.spot.x {
                            att = att * pow((rho - l.spot.y) / max(l.spot.x - l.spot.y, 1e-6), l.att.w);
                        }
                    }
                }
                amb += l.ambient.rgb * att;
                dif += l.diffuse.rgb * max(dot(n, ldir), 0.0) * att;
            }
            c0 = vec4f(clamp(me.rgb + ma.rgb * amb + md.rgb * dif, vec3f(0.0), vec3f(1.0)), clamp(md.a, 0.0, 1.0));
            c1 = vec4f(0.0, 0.0, 0.0, a.c1.a);
        }
    }
    o.c0 = c0;
    o.c1 = c1;
    o.t0 = a.t0.xy;
    o.t1 = a.t1.xy;
    return o;
}
