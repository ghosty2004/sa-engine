//! RenderWare's D3D9 vertex-shader generator `_rpD3D9VertexShaderGenerate` (0x75F0B0, catalogued as
//! `(missed by ghidra: cat7-call)`), ported from the 1.0 US bytes (WRITER-MVP-G). See docs/arch/gpu.md
//! §1 (STD-GFX) for where the shaders go.
//!
//! The generator prints vertex-shader assembly text into the buffer 0xC93AF8 with
//! `RwEngineInstance->stringFuncs.vecSprintf` (`RwGlobals` +0xF0), one call per line, from the format
//! strings at 0x8D641C..0x8D736C, then assembles it with the statically linked `D3DXAssembleShader`
//! (0x7652AE) and creates the shader with `RwD3D9CreateVertexShader` (0x7FAC60). The format strings
//! (and the swizzle tables at 0x87522C / 0x875240) are not copied: every line passes the string's
//! address in the exe image, exactly as the original pushes it, so the text is whatever the user's exe
//! holds.
//!
//! Descriptor (`_rpD3D9VertexShaderDescriptor`, the dword `*desc`, see `d3d9node`): byte 0 low nibble
//! = directional lights, high nibble = point lights; byte 1 low nibble = spot lights, high nibble =
//! texture-coordinate sets; byte 2: bit 0 morphing, bits 1-3 skin weights, bit 4 prelit colour, bit 5
//! normals, bit 6 normalise, bit 7 modulate material colour; byte 3: bits 0-1 fog mode, bits 2-7 the
//! effect (0 none, 1..11 the RpMatFX / UV-animation variants; the per-effect constant count is the
//! dword table 0xC94AF8).
//!
//! Output (`out`, 8 bytes, all 0xFF first): the constant registers the caller must upload: +0 the
//! material colour (modulate), +1 fog, +2 the effect constants, +3 the morph weights, +4 the bone
//! matrices (skinning). The register file is laid out from c5: two per directional light, then the
//! point lights (one shared attenuation register per group of four plus two each), then the spot
//! lights (one per group plus four each), then the registers above in that order. With skinning the
//! effect constants are put below the top constant (byte 0xC94C04) instead.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, p8, rd, wr};

/// The text buffer (`.bss`).
const BUF: u32 = 0xC9_3AF8;
/// The device's vertex-shader version (10 = vs_1_1, 20 = vs_2_0, 25 = vs_2_x; other values: no shaders).
const VS_VERSION: usize = 0xC9_4C00;
/// The top usable vertex-shader constant register (byte).
const VS_TOP_CONST: usize = 0xC9_4C04;
/// Constant registers each effect type needs (dword per effect type).
const EFFECT_CONSTS: u32 = 0xC9_4AF8;
/// Four component letters (`x y z w` in the 1.0 US image), followed at +4 by the pointer table of the
/// swizzle strings of 1..4 components (indexed from 0x87522C, so entry 0 is the letters).
const COMP: u32 = 0x87_522C;
/// Pointer table of the replicated single-component swizzle strings (one per component).
const REPL: u32 = 0x87_5240;

orig! {
    /// `RwEngineInstance->stringFuncs.vecSprintf(buf, fmt)` (`RwGlobals` +0xF0)
    fn rw_sprintf0(buf: u32, fmt: u32) -> i32 = cdecl table 0xC97B24 0xF0 => unported;
    /// `vecSprintf(buf, fmt, a)`
    fn rw_sprintf1(buf: u32, fmt: u32, a: u32) -> i32 = cdecl table 0xC97B24 0xF0 => unported;
    /// `vecSprintf(buf, fmt, a, b)`
    fn rw_sprintf2(buf: u32, fmt: u32, a: u32, b: u32) -> i32 = cdecl table 0xC97B24 0xF0 => unported;
    /// `vecSprintf(buf, fmt, a, b, c)`
    fn rw_sprintf3(buf: u32, fmt: u32, a: u32, b: u32, c: u32) -> i32 = cdecl table 0xC97B24 0xF0 => unported;
    /// `vecSprintf(buf, fmt, a, b, c, d)`
    fn rw_sprintf4(buf: u32, fmt: u32, a: u32, b: u32, c: u32, d: u32) -> i32 = cdecl table 0xC97B24 0xF0 => unported;
    /// `D3DXAssembleShader(src, len, defines, include, flags, &shader, &errors)` (0x7652AE, statically
    /// linked D3DX, `__stdcall`)
    fn d3dx_assemble_shader(src: u32, len: u32, defines: u32, include: u32, flags: u32, shader: GPtr<u32>, errors: u32) -> i32 = stdcall 0x7652AE => unported;
    /// `ID3DXBuffer::GetBufferPointer` (+0xC)
    fn buffer_get_pointer(buf: GPtr<u8>) -> u32 = stdcall vslot 0xC => unported;
    /// `ID3DXBuffer::Release` (+8)
    fn buffer_release(buf: GPtr<u8>) -> u32 = stdcall vslot 0x8 => unported;
    /// `RwD3D9CreateVertexShader(function, &shader)` (0x7FAC60)
    fn create_vertex_shader(func: u32, out: u32) -> u32 = cdecl 0x7FAC60 => crate::d3d9dev::create_vertex_shader;
}

/// The write cursor (ESI in the original): every line is printed at it and it advances by
/// `vecSprintf`'s result.
struct Text(u32);

impl Text {
    fn e0(&mut self, f: u32) {
        self.0 = self.0.wrapping_add(rw_sprintf0(self.0, f) as u32);
    }
    fn e1(&mut self, f: u32, a: u32) {
        self.0 = self.0.wrapping_add(rw_sprintf1(self.0, f, a) as u32);
    }
    fn e2(&mut self, f: u32, a: u32, b: u32) {
        self.0 = self.0.wrapping_add(rw_sprintf2(self.0, f, a, b) as u32);
    }
    fn e3(&mut self, f: u32, a: u32, b: u32, c: u32) {
        self.0 = self.0.wrapping_add(rw_sprintf3(self.0, f, a, b, c) as u32);
    }
    fn e4(&mut self, f: u32, a: u32, b: u32, c: u32, d: u32) {
        self.0 = self.0.wrapping_add(rw_sprintf4(self.0, f, a, b, c, d) as u32);
    }
}

/// `movsx` of the component letter `i` (bytes past the four letters for `i > 3`, as the original).
fn comp(i: u32) -> u32 {
    g::<i8>(COMP.wrapping_add(i) as usize).read() as i32 as u32
}

/// The swizzle-string pointer `[0x87522C + 4 * n]` (`n` components; `n = 0` reads the letters).
fn swz(n: u32) -> u32 {
    g::<u32>(COMP.wrapping_add(n.wrapping_mul(4)) as usize).read()
}

/// The replicated-component swizzle-string pointer `[0x875240 + 4 * i]`.
fn repl(i: u32) -> u32 {
    g::<u32>(REPL.wrapping_add(i.wrapping_mul(4)) as usize).read()
}

/// The lit colour accumulation of one light (shared by the three light loops): while at most one
/// light is left (`lights <= 1`, unsigned) and the material colour is not modulated the line writes
/// `oD0` directly with the light's component letter, otherwise it accumulates into r11 with the
/// replicated swizzle. `lights` counts down per light.
fn light_colour(t: &mut Text, lights: &mut u32, modulate: u32, reg: u32, i: u32) {
    if *lights <= 1 && modulate == 0 {
        t.e2(0x8D_6D38, reg, comp(i));
    } else {
        t.e2(0x8D_6D10, reg, repl(i));
    }
    *lights = lights.wrapping_sub(1);
}

/// `_rpD3D9VertexShaderGenerate(desc, out)` (0x75F0B0): generates, assembles and creates the vertex
/// shader for the descriptor `*desc` and returns it (NULL when the assembly fails, and 0 without
/// printing anything when the device's vertex-shader version 0xC94C00 is not 10, 20 or 25). `out`
/// receives the constant-register bases (see the module docs; 0xFF = unused).
///
/// Order of the text: version line; `dcl` lines (position, blend weight with more than one weight,
/// blend indices, colour, normal, tangent; then the texture coordinates per effect: effects 1, 2, 3,
/// 5, 6, 10, 11 declare v5 when there is at least one set, 4 and 7 declare v6 only with at least two
/// sets, the others one line per set; morph position / normal); the position transform (plain, morphed
/// into r3/r5 or r2/r4, skinned with one weight through `a0.x` or with several through `a0` (vs_2_0
/// and up) or one `a0.x` load per weight (vs_1_1), with or without normals); fog (modes 1..3); the
/// normal normalisation; then either the tangent-space bump path (effects 8 and 9 with normals: r4/r2
/// basis, the light vector of the first light kind present, effect 9 adds oT2/oT3 rows) or the lights
/// (directional, point and spot in groups of four, accumulating into r11 / oD0) and the material
/// colour; then the texture-coordinate generation per effect (1 environment map, 2 projected UVs, 3
/// both, 4..7 UV copies / transforms, 10 reflection, 11 both, other values copy every set).
///
/// Original details kept: the register strings are kept in two frame slots (position: "v0", "r2",
/// "r3"; normal: NULL without normals, "v3", "r4", "r5"), so a NULL normal string reaches
/// `vecSprintf` for the shapes that print it without normals; the component letter and swizzle
/// tables are indexed by the weight count (up to 7) past their four entries; the light countdown is
/// compared unsigned; `*desc` and `out` bytes are re-read where the original re-reads them.
#[hook(addr = 0x75F0B0, conv = cdecl, name = "(missed by ghidra: cat7-call)", expect = "83 EC 38 53 8B 5C 24 40",
       unverified = "mvpg-rw-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "_rpD3D9VertexShaderGenerate; writes the text buffer 0xC93AF8 (up to its length) and out[0..8]; only called with shader caps (cache miss in _rpD3D9GetVertexShader)")]
pub fn vertex_shader_generate(desc: GPtr<u32>, out: GPtr<u32>) -> u32 {
    let d: GPtr<u8> = desc.cast();
    let o: GPtr<u8> = out.cast();
    let db = |k: isize| rd::<u8>(d, k) as u32;
    let ob = |k: isize| rd::<u8>(o, k) as u32;

    let b2 = db(2);
    let b3 = db(3);
    let modulate = b2 >> 7; // [esp+0x28]
    let weights = (b2 >> 1) & 7; // ebp, [esp+0x1C]
    let normalize = (b2 >> 6) & 1; // [esp+0x38]
    let effect = b3 >> 2; // [esp+0x2C]
    let fog = b3 & 3; // [esp+0x30]
    let mut normals = 0u32; // [esp+0x18]
    let mut tangents = 0u32; // [esp+0x34]
    if (db(0) != 0 || db(1) & 0xF != 0 || matches!(effect, 1 | 2 | 3 | 8 | 9 | 10 | 11)) && b2 & 0x20 != 0 {
        normals = 1;
        if effect == 8 || effect == 9 {
            tangents = 1;
        }
    }

    // ---- constant registers --------------------------------------------------------------------
    wr::<u32>(o, 0, 0xFFFF_FFFF);
    wr::<u32>(o, 4, 0xFFFF_FFFF);
    let mut reg: u32 = 5; // ebx
    let mut point_base: u32 = 0xFFFF_FFFF; // [esp+0x24]
    let mut spot_base: u32 = 0xFFFF_FFFF; // [esp+0x20]
    let b0 = db(0);
    if b0 & 0xF != 0 {
        reg = (b0 & 0xF) * 2 + 5;
    }
    if b0 & 0xF0 != 0 {
        point_base = reg;
        let n = b0 >> 4;
        reg = reg.wrapping_add(n.div_ceil(4)).wrapping_add(n * 2);
    }
    let b1 = db(1);
    if b1 & 0xF != 0 {
        spot_base = reg;
        let n = b1 & 0xF;
        reg = reg.wrapping_add(n.div_ceil(4)).wrapping_add(n * 4);
    }
    if modulate != 0 {
        wr::<u8>(o, 0, reg as u8);
        reg = reg.wrapping_add(1);
    }
    if fog as i32 > 0 {
        wr::<u8>(o, 1, reg as u8);
        reg = reg.wrapping_add(1);
    }
    if db(2) & 1 != 0 {
        wr::<u8>(o, 3, reg as u8);
        reg = reg.wrapping_add(1);
    }
    if weights != 0 {
        wr::<u8>(o, 4, reg as u8);
    }
    if effect != 0 {
        let n = g::<u32>(EFFECT_CONSTS.wrapping_add(effect * 4) as usize).read();
        if n != 0 {
            if weights != 0 {
                wr::<u8>(o, 2, g::<u8>(VS_TOP_CONST).read().wrapping_sub(n as u8));
            } else {
                wr::<u8>(o, 2, reg as u8);
                reg = reg.wrapping_add(n);
            }
        }
    }

    // ---- header and declarations -----------------------------------------------------------------
    let head = match g::<u32>(VS_VERSION).read() {
        10 => 0x8D_7354,
        20 => 0x8D_735C,
        25 => 0x8D_7364,
        _ => return 0,
    };
    let mut t = Text(BUF);
    t.0 = BUF.wrapping_add(rw_sprintf0(BUF, head) as u32);
    t.e0(0x8D_7340);
    if weights != 0 {
        if weights > 1 {
            t.e0(0x8D_7328);
        }
        t.e0(0x8D_7310);
    }
    if db(2) & 0x10 != 0 {
        t.e0(0x8D_7300);
    }
    if normals != 0 {
        t.e0(0x8D_72F0);
    }
    if tangents != 0 {
        t.e0(0x8D_72DC);
    }
    let case = effect.wrapping_sub(1); // [esp+0x44]
    let tex_loop = |t: &mut Text| {
        if db(1) & 0xF0 != 0 {
            let mut i = 0u32;
            loop {
                t.e2(0x8D_72A0, i, i + 5);
                i += 1;
                if i >= db(1) >> 4 {
                    break;
                }
            }
        }
    };
    match case {
        0 | 1 | 2 | 4 | 5 | 9 | 10 => {
            if db(1) & 0xF0 != 0 {
                t.e0(0x8D_72C8);
            }
        }
        3 | 6 => {
            if db(1) & 0xF0 > 0x10 {
                t.e0(0x8D_72B4);
            }
        }
        _ => tex_loop(&mut t),
    }
    if db(2) & 1 != 0 {
        t.e0(0x8D_728C);
        if normals != 0 {
            t.e0(0x8D_7278);
        }
    }

    // ---- position (and normal) transform ---------------------------------------------------------
    let mut pos: u32 = 0x8D_7274; // [esp+0x14]
    let mut nrm: u32 = 0; // [esp+0x10]
    if normals != 0 {
        nrm = 0x8D_7270;
    }
    if db(2) & 1 == 0 && weights == 0 {
        t.e0(0x8D_7254);
    } else {
        // 0: straight to the w copy (0x75F918), 1: the skinning code (0x75F561)
        let mut skin = true;
        if db(2) & 1 != 0 {
            let m = ob(3);
            if weights == 0 {
                t.e1(0x8D_723C, m);
                t.e1(0x8D_7220, m);
                pos = 0x8D_721C;
                if normals != 0 {
                    t.e1(0x8D_7204, m);
                    t.e1(0x8D_71E8, m);
                    nrm = 0x8D_6EA4;
                }
                skin = false;
            } else {
                t.e1(0x8D_71D0, m);
                t.e1(0x8D_71B4, m);
                pos = 0x8D_71B0;
                if normals != 0 {
                    t.e1(0x8D_7198, m);
                    t.e1(0x8D_717C, m);
                    nrm = 0x8D_7178;
                }
            }
        }
        if skin && weights != 0 {
            skin_transform(&mut t, weights, normals, reg, pos, nrm);
            pos = 0x8D_721C;
            if normals != 0 {
                nrm = 0x8D_6EA4;
            }
        }
        t.e0(0x8D_6E94);
        t.e0(0x8D_6E78);
    }

    // ---- fog -------------------------------------------------------------------------------------
    if fog as i32 > 0 {
        let f = ob(1);
        t.e1(0x8D_6E64, pos);
        match fog {
            1 => t.e2(0x8D_6DE8, f, f),
            2 => {
                t.e1(0x8D_6E48, f);
                t.e0(0x8D_6E1C);
                t.e0(0x8D_6E0C);
            }
            3 => {
                t.e1(0x8D_6E48, f);
                t.e0(0x8D_6E30);
                t.e0(0x8D_6E1C);
                t.e0(0x8D_6E0C);
            }
            _ => {}
        }
    }

    // ---- normalisation ---------------------------------------------------------------------------
    if normals != 0 && normalize != 0 {
        t.e2(0x8D_6DD4, nrm, nrm);
        t.e0(0x8D_6DC4);
        t.e1(0x8D_6DAC, nrm);
        nrm = 0x8D_6EA4;
    }

    if tangents != 0 {
        bump(&mut t, d, o, weights, normalize, effect, reg, pos, nrm);
    } else {
        lights(&mut t, d, o, normals, modulate, point_base, spot_base, pos, nrm);
    }

    // ---- texture coordinates per effect ----------------------------------------------------------
    if effect == 0 {
        tex_copy(&mut t, d);
    } else {
        let e = ob(2);
        match case {
            0 => env_map(&mut t, e, nrm),
            1 => {
                if db(1) & 0xF0 != 0 {
                    t.e0(0x8D_66B8);
                    t.e2(0x8D_66A0, nrm, e);
                    t.e2(0x8D_6688, nrm, e + 1);
                } else {
                    t.e2(0x8D_6670, nrm, e);
                    t.e2(0x8D_6658, nrm, e + 1);
                }
            }
            2 => {
                t.e2(0x8D_6640, nrm, e + 2);
                t.e2(0x8D_6628, nrm, e + 3);
                env_map(&mut t, e, nrm);
            }
            3 => t.e0(0x8D_65CC),
            4 => {
                t.e0(0x8D_66B8);
                t.e0(0x8D_6780);
            }
            5 => {
                t.e1(0x8D_65B4, e);
                t.e1(0x8D_659C, e + 1);
            }
            6 => {
                t.e1(0x8D_6584, e);
                t.e1(0x8D_656C, e + 1);
            }
            7 | 8 => {}
            9 => {
                reflection(&mut t, e, pos, nrm);
                if db(1) & 0xF0 != 0 {
                    t.e0(0x8D_66B8);
                    t.e1(0x8D_64F0, e + 1);
                    t.e1(0x8D_64D8, e + 2);
                    t.e1(0x8D_64C0, e + 3);
                } else {
                    t.e1(0x8D_64A8, e + 1);
                    t.e1(0x8D_6490, e + 2);
                    t.e1(0x8D_6478, e + 3);
                }
            }
            10 => {
                env_map(&mut t, e, nrm);
                reflection(&mut t, e + 2, pos, nrm);
                t.e1(0x8D_6460, e + 3);
                t.e1(0x8D_6448, e + 4);
                t.e1(0x8D_6430, e + 5);
            }
            _ => tex_copy(&mut t, d),
        }
    }

    // ---- assemble and create ---------------------------------------------------------------------
    let mut buffer: u32 = 0; // [esp+0x3C]
    let mut shader: u32 = 0; // [esp+0x40]
    let len = t.0.wrapping_sub(BUF);
    if d3dx_assemble_shader(BUF, len, 0, 0, 4, GPtr::from_raw(&mut buffer as *mut u32), 0) >= 0 {
        let code = buffer_get_pointer(p8(buffer));
        create_vertex_shader(code, GPtr::<u32>::from_raw(&mut shader as *mut u32).addr() as u32);
    }
    if buffer != 0 {
        buffer_release(p8(buffer));
    }
    shader
}

/// The skinning transform (0x75F561..0x75F900), `weights >= 1`, bones from register `bones`.
fn skin_transform(t: &mut Text, weights: u32, normals: u32, bones: u32, pos: u32, nrm: u32) {
    if weights == 1 {
        t.e0(if g::<u32>(VS_VERSION).read() == 10 { 0x8D_7168 } else { 0x8D_7154 });
        t.e2(0x8D_7134, pos, bones);
        if normals != 0 {
            t.e2(0x8D_7114, nrm, bones);
        }
        return;
    }
    let v2 = g::<u32>(VS_VERSION).read() >= 20;
    if normals != 0 {
        // blend the bone matrices into r8..r10, then transform position and normal with them
        if v2 {
            t.e1(0x8D_7104, swz(weights));
            for i in 0..weights {
                let c = comp(i);
                if i != 0 {
                    t.e3(0x8D_70E0, c, c, bones);
                    t.e3(0x8D_70BC, c, c, bones + 1);
                    t.e3(0x8D_7098, c, c, bones + 2);
                } else {
                    t.e3(0x8D_7078, 0x78, 0x78, bones);
                    t.e3(0x8D_7058, 0x78, 0x78, bones + 1);
                    t.e3(0x8D_7038, 0x78, 0x78, bones + 2);
                }
            }
        } else {
            for i in 0..weights {
                let c = comp(i);
                t.e1(0x8D_7024, c);
                if i != 0 {
                    t.e2(0x8D_7000, c, bones);
                    t.e2(0x8D_6FDC, c, bones + 1);
                    t.e2(0x8D_6FB8, c, bones + 2);
                } else {
                    t.e2(0x8D_6F98, 0x78, bones);
                    t.e2(0x8D_6F78, 0x78, bones + 1);
                    t.e2(0x8D_6F58, 0x78, bones + 2);
                }
            }
        }
        t.e1(0x8D_6F40, pos);
        t.e1(0x8D_6F28, nrm);
    } else if v2 {
        t.e1(0x8D_7104, swz(weights));
        for i in 0..weights {
            t.e3(0x8D_6F08, pos, comp(i), bones);
            if i == 0 {
                t.e1(0x8D_6EEC, 0x78);
            } else {
                t.e1(0x8D_6EC8, comp(i));
            }
        }
    } else {
        for i in 0..weights {
            t.e1(0x8D_7024, comp(i));
            t.e2(0x8D_6EA8, pos, bones);
            if i == 0 {
                t.e1(0x8D_6EEC, 0x78);
            } else {
                t.e1(0x8D_6EC8, comp(i));
            }
        }
    }
}

/// The vertex lighting (0x75FAAC..0x76054D): directional, point and spot lights in groups of four,
/// then the material colour. Without normals or lights only the prelit / ambient colour.
#[allow(clippy::too_many_arguments)]
fn lights(
    t: &mut Text,
    d: GPtr<u8>,
    o: GPtr<u8>,
    normals: u32,
    modulate: u32,
    mut point_base: u32,
    mut spot_base: u32,
    pos: u32,
    nrm: u32,
) {
    let db = |k: isize| rd::<u8>(d, k) as u32;
    let mut left = (db(0) & 0xF) + (db(1) & 0xF) + (db(0) >> 4); // [esp+0x1C]
    if normals == 0 || left == 0 {
        if modulate != 0 {
            t.e0(if db(2) & 0x10 != 0 { 0x8D_6D98 } else { 0x8D_6D88 });
            t.e1(0x8D_6B90, rd::<u8>(o, 0) as u32);
        } else {
            t.e0(if db(2) & 0x10 != 0 { 0x8D_6B7C } else { 0x8D_6B6C });
        }
        return;
    }
    t.e0(if db(2) & 0x10 != 0 { 0x8D_6D98 } else { 0x8D_6D88 });
    // directional: two registers each from c5
    let n = db(0) & 0xF;
    if n != 0 {
        let mut done = 0u32;
        let mut base = 6u32;
        loop {
            let k = n.wrapping_sub(done).min(4);
            for i in 0..k {
                t.e3(0x8D_6D70, comp(i), base - 1 + 2 * i, nrm);
            }
            t.e2(0x8D_6D54, swz(k), swz(k));
            for i in 0..k {
                light_colour(t, &mut left, modulate, base + 2 * i, i);
            }
            done += 4;
            base += 8;
            if done >= n {
                break;
            }
        }
    }
    // point lights
    let n = db(0) >> 4;
    if n != 0 {
        let mut done = 0u32;
        loop {
            let k = n.wrapping_sub(done).min(4);
            for i in 0..k {
                let c = comp(i);
                t.e2(0x8D_6CF0, point_base.wrapping_add(1 + 2 * i), pos);
                t.e1(0x8D_6CDC, c);
                t.e2(0x8D_6CC8, c, c);
                t.e2(0x8D_6CB4, c, nrm);
            }
            let s = swz(k);
            let b = point_base;
            t.e4(0x8D_6C98, s, b, s, s);
            t.e3(0x8D_6C78, s, b.wrapping_add(1), s);
            t.e4(0x8D_6C58, s, b, s, s);
            t.e3(0x8D_6C3C, s, s, s);
            for i in 0..k {
                light_colour(t, &mut left, modulate, b.wrapping_add(2 + 2 * i), i);
            }
            point_base = point_base.wrapping_add(k * 2 + 1);
            done += 4;
            if done >= n {
                break;
            }
        }
    }
    // spot lights
    let n = db(1) & 0xF;
    if n != 0 {
        let mut done = 0u32;
        loop {
            let k = n.wrapping_sub(done).min(4);
            for i in 0..k {
                let c = comp(i);
                let r = spot_base.wrapping_add(3 + 4 * i);
                t.e2(0x8D_6CF0, r.wrapping_sub(2), pos);
                t.e1(0x8D_6CDC, c);
                t.e2(0x8D_6CC8, c, c);
                t.e2(0x8D_6CB4, c, nrm);
                t.e1(0x8D_6C20, c);
                t.e1(0x8D_6C08, r.wrapping_sub(1));
                t.e3(0x8D_6BE4, c, r, r);
            }
            let s = swz(k);
            let b = spot_base;
            t.e4(0x8D_6C98, s, b, s, s);
            t.e3(0x8D_6BC4, s, b.wrapping_add(1), s);
            t.e3(0x8D_6C78, s, b.wrapping_add(1), s);
            t.e4(0x8D_6C58, s, spot_base, s, s);
            t.e3(0x8D_6C3C, s, s, s);
            t.e3(0x8D_6BA8, s, s, s);
            for i in 0..k {
                light_colour(t, &mut left, modulate, b.wrapping_add(4 + 4 * i), i);
            }
            spot_base = spot_base.wrapping_add(k * 4 + 1);
            done += 4;
            if done >= n {
                break;
            }
        }
    }
    if modulate != 0 {
        t.e1(0x8D_6B90, rd::<u8>(o, 0) as u32);
    }
}

/// The tangent-space bump path (0x760107..0x76054B), effects 8 and 9 with normals: tangent r4,
/// binormal r2, the light vector of the first light kind present (directional, else point, else spot)
/// into oT2 (effect 8) or the colour (effect 9, which then writes the oT2 / oT3 rows from the effect
/// constants), then the two texture coordinates.
#[allow(clippy::too_many_arguments)]
fn bump(t: &mut Text, d: GPtr<u8>, o: GPtr<u8>, weights: u32, normalize: u32, effect: u32, bones: u32, pos: u32, nrm: u32) {
    let db = |k: isize| rd::<u8>(d, k) as u32;
    if weights != 0 {
        if weights == 1 {
            t.e1(0x8D_6B4C, bones);
        } else {
            t.e0(0x8D_6B34);
        }
        if normalize != 0 {
            t.e0(0x8D_6B20);
            t.e0(0x8D_6B10);
            t.e0(0x8D_6AF8);
        }
        t.e0(0x8D_6AE0);
    } else {
        t.e0(0x8D_6AD0);
    }
    if db(2) & 1 != 0 || weights != 0 {
        t.e1(0x8D_6AC0, nrm);
    }
    t.e1(0x8D_6AA4, nrm);
    t.e1(0x8D_6A84, nrm);
    if normalize != 0 {
        t.e0(0x8D_6A70);
        t.e0(0x8D_6A60);
        t.e0(0x8D_6A48);
    }
    if db(2) & 0x10 != 0 {
        t.e0(0x8D_6A38);
    }
    let b0 = db(0);
    // the colour line of effect 9 for directional (`.w` constants) or point / spot (`.z`) lights
    let colour9 = |t: &mut Text, prelit: bool, dir: bool| match (prelit, dir) {
        (true, true) => t.e0(0x8D_6978),
        (false, true) => t.e0(0x8D_6954),
        (true, false) => t.e0(0x8D_675C),
        (false, false) => t.e0(0x8D_6738),
    };
    let rows9;
    if b0 & 0xF != 0 {
        t.e0(0x8D_6A24);
        t.e0(0x8D_6A10);
        t.e1(0x8D_69FC, nrm);
        t.e0(0x8D_69E8);
        t.e0(0x8D_69D8);
        t.e0(0x8D_69C0);
        if effect != 9 {
            t.e0(0x8D_699C);
            rows9 = false;
        } else {
            colour9(t, db(2) & 0x10 != 0, true);
            rows9 = true;
        }
    } else if b0 & 0xF0 != 0 || db(1) & 0xF != 0 {
        let spot = b0 & 0xF0 == 0;
        t.e1(0x8D_6930, pos);
        t.e0(0x8D_691C);
        t.e0(0x8D_690C);
        t.e0(0x8D_68F4);
        if spot {
            t.e0(0x8D_6720);
            t.e0(0x8D_66F8);
        }
        t.e0(0x8D_68E4);
        t.e0(0x8D_68C4);
        t.e0(0x8D_68AC);
        if spot {
            t.e0(0x8D_66E0);
        }
        t.e0(0x8D_6898);
        t.e0(0x8D_6884);
        t.e1(0x8D_6870, nrm);
        t.e0(0x8D_69E8);
        t.e0(0x8D_69D8);
        t.e0(0x8D_69C0);
        t.e0(0x8D_6858);
        if spot {
            t.e0(0x8D_66C8);
        }
        if effect != 9 {
            t.e0(0x8D_6834);
            rows9 = false;
        } else {
            colour9(t, db(2) & 0x10 != 0, false);
            rows9 = true;
        }
    } else {
        rows9 = effect == 9;
    }
    if rows9 {
        let e = rd::<u8>(o, 2) as u32;
        t.e1(0x8D_681C, e);
        t.e1(0x8D_6804, e);
        t.e2(0x8D_67EC, e, nrm);
        t.e1(0x8D_67D4, e + 1);
        t.e1(0x8D_67BC, e + 1);
        t.e2(0x8D_67A4, e + 1, nrm);
    }
    t.e1(0x8D_6790, (db(1) >> 4) + 4);
    t.e0(0x8D_6780);
}

/// Effect 1 (and the tail of 3 and the head of 11): the environment-map rows from constants `e`, `e+1`.
fn env_map(t: &mut Text, e: u32, nrm: u32) {
    t.e2(0x8D_6610, nrm, e);
    t.e2(0x8D_65F8, nrm, e + 1);
    t.e0(0x8D_66B8);
    t.e0(0x8D_65DC);
}

/// Effects 10 and 11: the reflection vector r7 about the normal, eye position in constant `e`.
fn reflection(t: &mut Text, e: u32, pos: u32, nrm: u32) {
    t.e2(0x8D_654C, e, pos);
    t.e0(0x8D_691C);
    t.e0(0x8D_690C);
    t.e0(0x8D_68F4);
    t.e1(0x8D_6538, nrm);
    t.e0(0x8D_6520);
    t.e1(0x8D_6508, nrm);
}

/// No effect (or an unknown one): one copy line per texture-coordinate set (`*desc` byte 1 re-read
/// per iteration).
fn tex_copy(t: &mut Text, d: GPtr<u8>) {
    if rd::<u8>(d, 1) & 0xF0 != 0 {
        let mut i = 0u32;
        loop {
            t.e2(0x8D_641C, i, i + 5);
            i += 1;
            if i >= (rd::<u8>(d, 1) >> 4) as u32 {
                break;
            }
        }
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_VERTEX_SHADER_GENERATE];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = &[];
