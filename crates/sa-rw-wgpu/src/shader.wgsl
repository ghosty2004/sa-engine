// World pass of sa-rw-wgpu: the D3D9 fixed-function state the game's building pipelines use,
// approximated (see docs/arch/native-demo.md "Known gaps").

struct Frame {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    // inverse of proj * view-rotation (sky rays)
    inv_view_rot_proj: mat4x4<f32>,
    cam_pos: vec4<f32>,
    fog_color: vec4<f32>,
    // fog start, fog end (far clip), day/night balance, near plane
    fog: vec4<f32>,
    ambient: vec4<f32>,
    dir_color: vec4<f32>,
    // direction the light travels (world space)
    light_dir: vec4<f32>,
    sky_top: vec4<f32>,
    sky_bottom: vec4<f32>,
    // CTimeCycle "amb" (world ambient light, used for prelit lit geometry)
    ambient_world: vec4<f32>,
    // CPostEffects::ColourFilter: rgb added multiple of the frame (pass1 + pass2)
    color_filter: vec4<f32>,
};

@group(0) @binding(0) var<uniform> frame: Frame;

struct Material {
    color: vec4<f32>,
    // x: textured, y: modulate material colour, z: prelit, w: has normals
    flags: vec4<u32>,
    // RwSurfaceProperties ambient, specular, diffuse; w: rpGEOMETRYLIGHT
    surf: vec4<f32>,
};

@group(1) @binding(0) var<uniform> mat: Material;
@group(1) @binding(1) var tex: texture_2d<f32>;
@group(1) @binding(2) var samp: sampler;

struct VIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) day: vec4<f32>,
    @location(4) night: vec4<f32>,
    @location(5) m0: vec4<f32>,
    @location(6) m1: vec4<f32>,
    @location(7) m2: vec4<f32>,
    @location(8) m3: vec4<f32>,
    // alpha test reference (0-255), z shift (view units), alpha multiplier, unused
    @location(9) extra: vec4<f32>,
};

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) dist: f32,
    @location(3) @interpolate(flat) aref: f32,
};

@vertex
fn vs_main(v: VIn) -> VOut {
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    let world = model * vec4<f32>(v.pos, 1.0);
    let vpos = frame.view * world;
    let d = -vpos.z;
    var clip = frame.proj * vpos;
    // Reversed infinite depth: z_ndc = near / (d + zShift). RwCamera::zShift moves only the
    // depth value (LODs are drawn with -100 shift in RenderEverythingBarRoads).
    clip.z = frame.fog.w * d / max(d + v.extra.y, 1e-4);

    var c: vec4<f32>;
    if (mat.flags.z != 0u) {
        c = mix(v.day, v.night, frame.fog.z);
        if (mat.surf.w != 0.0) {
            // RW fixed-function lighting with the prelight as emissive colour: + ambient
            var lit = frame.ambient_world.rgb * mat.surf.x;
            if (mat.flags.w != 0u) {
                let n = normalize((model * vec4<f32>(v.normal.xyz, 0.0)).xyz);
                lit = lit + frame.dir_color.rgb * mat.surf.z * max(dot(n, -frame.light_dir.xyz), 0.0);
            }
            c = vec4<f32>(min(c.rgb + lit, vec3<f32>(1.0)), c.a);
        }
    } else {
        var lit = frame.ambient.rgb;
        if (mat.flags.w != 0u) {
            let n = normalize((model * vec4<f32>(v.normal.xyz, 0.0)).xyz);
            lit = lit + frame.dir_color.rgb * max(dot(n, -frame.light_dir.xyz), 0.0);
        }
        c = vec4<f32>(min(lit, vec3<f32>(1.0)), 1.0);
    }
    if (mat.flags.y != 0u) {
        c = c * mat.color;
    }
    c.a = c.a * v.extra.z;

    var o: VOut;
    o.clip = clip;
    o.uv = v.uv;
    o.color = c;
    o.dist = d;
    o.aref = v.extra.x;
    return o;
}

@fragment
fn fs_main(i: VOut) -> @location(0) vec4<f32> {
    var c = i.color;
    if (mat.flags.x != 0u) {
        c = c * textureSample(tex, samp, i.uv);
    }
    // D3DRS_ALPHAFUNC = GREATER against D3DRS_ALPHAREF
    if (c.a * 255.0 <= i.aref) {
        discard;
    }
    let f = clamp((frame.fog.y - i.dist) / max(frame.fog.y - frame.fog.x, 1e-3), 0.0, 1.0);
    return vec4<f32>(mix(frame.fog_color.rgb, c.rgb, f), c.a);
}

// ---------------------------------------------------------------------------------- sky ---

struct SkyOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) ndc: vec2<f32>,
};

@vertex
fn vs_sky(@builtin(vertex_index) i: u32) -> SkyOut {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u)) * 2.0 - 1.0;
    var o: SkyOut;
    o.clip = vec4<f32>(p, 0.0, 1.0);
    o.ndc = p;
    return o;
}

@fragment
fn fs_sky(i: SkyOut) -> @location(0) vec4<f32> {
    let h = frame.inv_view_rot_proj * vec4<f32>(i.ndc, 1.0, 1.0);
    let dir = normalize(h.xyz / h.w);
    // CClouds::RenderBackground: bottom colour at and below the horizon, blending to the top
    // colour higher up (the exact band heights are not modelled).
    let t = smoothstep(0.0, 0.35, dir.z);
    return vec4<f32>(mix(frame.sky_bottom.rgb, frame.sky_top.rgb, t), 1.0);
}

// ---------------------------------------------------------------------------- colour filter ---

@group(1) @binding(0) var scene: texture_2d<f32>;

// CPostEffects::ColourFilter (PC): two full-screen quads textured with a copy of the frame,
// vertex colour = postfx colour, blend SRCALPHA/ONE: F + F*c1*a1 + F*c2*a2.
@fragment
fn fs_post(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    let f = textureLoad(scene, vec2<i32>(pos.xy), 0);
    return vec4<f32>(min(f.rgb * (vec3<f32>(1.0) + frame.color_filter.rgb), vec3<f32>(1.0)), 1.0);
}
