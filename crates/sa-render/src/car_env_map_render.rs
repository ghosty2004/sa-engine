//! The vehicle render callback of the custom car environment-map pipeline and the three helpers it calls (batch
//! `boot-render-2`): `CCustomCarEnvMapPipeline::CustomPipeRenderCB` (0x5D9900), the environment-camera UV helper
//! (0x5D84C0), the environment-wave UV helper (0x5D8590) and the D3D9 material set-up (0x5DA790). Ported from the
//! 1.0 US bytes; gta-reversed's `CustomCarEnvMapPipeline.cpp` was used as a reading aid only (it rewrites the
//! wave helper and adds a SilentPatch fix the original lacks). Unverified (docs/verify/HOWTO.md).
//!
//! Layouts: `RxD3D9ResEntryHeader` at `resEntry + 0x18` (index buffer +8, vertex streams +0x10, `useOffsets` +0x30,
//! vertex declaration +0x34, mesh count +4), then 0x24-byte `RxD3D9InstanceData` records (material +8, vertex alpha
//! +0xC). Material: colour +4 (r, g, b, a), surface properties +0xC (ambient, specular = the game's material flags
//! dword +0x10, diffuse +0x14). Environment-map material data (plugin offset `[0x8D12C4]`): scale x/y int8 +0, +1,
//! translate-scale x/y int8 +2, +3, shininess +4, render frame word +6, texture +8. Atomic data (plugin offset
//! `[0x8D12C8]`, pool `[0xC02D2C]`): offset U +0, previous position x/y +4/+8.
//!
//! x87: every intermediate the original keeps on the FPU stack is an `X87` (24-bit precision control, as when the
//! renderer runs), every `fstp dword` a `to_f32`, every `_ftol` (0x821B40) [`ftol32`].
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, rd, wr, x, xi};

orig! {
    /// `CVisibilityPlugins::GetAtomicId(atomic)` (0x732370): the atomic plugin word +2, zero-extended
    fn get_atomic_id(a: GPtr<u8>) -> u32 = cdecl 0x732370 => unported;
    /// `_rwD3D9EnableClippingIfNeeded(object, type)` (0x756D90)
    fn enable_clipping_if_needed(object: GPtr<u8>, ty: u32) = cdecl 0x756D90 => unported;
    /// `RwD3D9GetRenderState(state, void* value)` (0x7FC320)
    fn get_render_state(state: u32, value: GPtr<u32>) = cdecl 0x7FC320 => unported;
    /// `RwD3D9SetRenderState(state, value)` (0x7FC2D0)
    fn set_render_state(state: u32, value: u32) -> u32 = cdecl 0x7FC2D0 => unported;
    /// `RwD3D9SetTextureStageState(stage, type, value)` (0x7FC340)
    fn set_tss(stage: u32, ty: u32, value: u32) = cdecl 0x7FC340 => unported;
    /// `RwD3D9SetTexture(texture, stage)` (0x7FDE70)
    fn set_texture(t: u32, stage: u32) -> u32 = cdecl 0x7FDE70 => unported;
    /// `_rwD3D9SetIndices(ib)` (0x7FA1C0)
    fn set_indices(ib: u32) = cdecl 0x7FA1C0 => unported;
    /// `_rwD3D9SetStreams(streams, useOffsets)` (0x7FA090)
    fn set_streams(streams: GPtr<u8>, use_offsets: u32) = cdecl 0x7FA090 => unported;
    /// `_rwD3D9SetVertexDeclaration(decl)` (0x7F9F70)
    fn set_vertex_declaration(decl: u32) = cdecl 0x7F9F70 => unported;
    /// `G->dOpenDevice.fpRenderStateSet(state, value)` (`[G] + 0x20`)
    fn rw_render_state_set(state: u32, value: u32) -> u32 = cdecl table 0xC97B24 0x20 => unported;
    /// `_rwD3D9RenderStateVertexAlphaEnable(enable)` (0x7FE0A0)
    fn vertex_alpha_enable(on: u32) = cdecl 0x7FE0A0 => unported;
    /// `Fx_c::GetFxQuality()` (0x49EA50) on `g_fx` (0xA9AE00)
    fn get_fx_quality(fx: GPtr<u8>) -> i32 = thiscall 0x49EA50 => unported;
    /// `RwD3D9SetLight(index, const void* light)` (0x7FA660)
    fn set_light(index: u32, light: GPtr<u8>) -> u32 = cdecl 0x7FA660 => unported;
    /// `RwD3D9EnableLight(index, enable)` (0x7FA860)
    fn enable_light(index: u32, enable: u32) -> u32 = cdecl 0x7FA860 => unported;
    /// `RwD3D9SetTransform(state, const void* matrix)` (0x7FA390)
    fn set_transform(state: u32, matrix: GPtr<u8>) -> u32 = cdecl 0x7FA390 => unported;
    /// `RwD3D9SetMaterial(const void* material)` (0x7FC430)
    fn set_material(m: GPtr<u8>) -> u32 = cdecl 0x7FC430 => unported;
    /// The environment-map atomic data pool's `New()` (0x5D9260, thiscall, catalogue `FUN_005d9260`)
    fn env_atm_pool_new(pool: GPtr<u8>) -> GPtr<u8> = thiscall 0x5D9260 => unported;
    /// `RxD3D9InstanceDataRender(header, inst)` (0x5DA640)
    fn instance_data_render(h: GPtr<u8>, inst: GPtr<u8>) = cdecl 0x5DA640 => unported;
    /// `RxD3D9InstanceDataRenderLighting(header, inst, flags, texture)` (0x5DA6A0)
    fn instance_data_render_lighting(h: GPtr<u8>, inst: GPtr<u8>, flags: u32, tex: u32) = cdecl 0x5DA6A0 => unported;
    /// The pipeline's stage reset after the meshes (0x5DAAC0, catalogue `FUN_005daac0`)
    fn render_cb_end() = cdecl 0x5DAAC0 => unported;
    /// `RwFrameGetLTM(frame)` (0x7F0990)
    fn frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
    /// `CVector::Normalise()` (0x59C910, thiscall)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// The environment-camera UV helper (0x5D84C0), register arguments
    fn env_cam_uv_call(env: GPtr<u8>, atomic: GPtr<u8>, out: GPtr<u8>) -> u32 = reg("args = eax ecx esi") 0x5D84C0 => env_cam_uv;
    /// The environment-wave UV helper (0x5D8590), register arguments plus one stack word
    fn env_wave_uv_call(atomic: GPtr<u8>, env: GPtr<u8>, atm: GPtr<u8>, out: GPtr<u8>) -> u32 = reg("args = ecx esi edi stack") 0x5D8590 => env_wave_uv;
    /// The D3D9 material set-up (0x5DA790)
    fn set_material_props_call(surf: GPtr<u8>, color: GPtr<u8>, flags: u32, spec: u32, power: u32) -> u32 = cdecl 0x5DA790 => set_material_props;
}

/// `gSpecIntensity`-like factor (0x8D12D0) and the specular scale 1.85 (0x86BEA4).
const SPEC_BASE: usize = 0x8D_12D0;
const K_SPEC_SCALE: usize = 0x86_BEA4;
/// The material / atomic plugin offsets: environment map (material), environment map (atomic), specular map.
const ENV_MAT_OFFSET: usize = 0x8D_12C4;
const ENV_ATM_OFFSET: usize = 0x8D_12C8;
const SPEC_MAT_OFFSET: usize = 0x8D_12CC;
/// The environment-map atomic data pool pointer.
const ENV_ATM_POOL: usize = 0xC0_2D2C;
/// `g_fx` (`Fx_c`), the specular light (`D3DLIGHT9`).
const G_FX: usize = 0xA9_AE00;
const SPEC_LIGHT: usize = 0xC0_2CB0;
/// The two texture-transform matrices (`D3DMATRIX`): environment camera and environment wave.
const CAM_MATRIX: usize = 0xC0_2D78;
const WAVE_MATRIX: usize = 0xC0_2D38;
/// The `D3DMATERIAL9` the material set-up fills: diffuse +0, ambient +0x10, specular +0x20, emissive +0x30, power +0x40.
const MATERIAL: usize = 0x8D_12D8;
/// `AmbientLightColour` r, g, b (`RwRGBAReal`, 0x8E2418).
const AMBIENT_LIGHT: usize = 0x8E_2418;
/// `.rdata` constants: 1.0, 100.0, 0.0, 0.1, 0.125, 50.0, 1/255, 254.0, 24.0.
const K_ONE: usize = 0x85_8624;
const K_HUNDRED: usize = 0x85_8628;
const K_ZERO: usize = 0x85_8B50;
const K_TENTH: usize = 0x85_8B1C;
const K_EIGHTH: usize = 0x85_8C48;
const K_FIFTY: usize = 0x85_8B40;
const K_INV255: usize = 0x85_9A3C;
const K_254: usize = 0x86_BE90;
const K_24: usize = 0x85_F114;
/// `RwEngineInstance` (`G`); `G + 8` is the render frame word.
const RW_ENGINE_INSTANCE: usize = 0xC9_7B24;

fn k(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

fn kx(addr: usize) -> X87 {
    x(k(addr))
}

fn ptr<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v as *mut T).cast()
}

/// An int8 field times 0.125 times 50 (`fild; fmul [0x858C48]; fmul [0x858B40]; fstp dword`).
fn scaled_i8(p: GPtr<u8>, off: isize) -> f32 {
    xi(i32::from(rd::<i8>(p, off))).fmul_m(k(K_EIGHTH)).fmul_m(k(K_FIFTY)).to_f32()
}

/// The LTM of the atomic's clump frame, or of the atomic's own frame when it has no clump.
fn atomic_or_clump_ltm(atomic: GPtr<u8>) -> GPtr<u8> {
    let clump = rd::<GPtr<u8>>(atomic, 0x3C);
    let frame = if clump.is_null() { rd::<GPtr<u8>>(atomic, 4) } else { rd::<GPtr<u8>>(clump, 4) };
    frame_get_ltm(frame)
}

/// The colour 0..255 of a factor (`_ftol`, then `cmp eax, 0xFF; jle`, signed: larger values clamp to 255, negative
/// ones keep their low byte).
fn clamp_byte(v: X87) -> u32 {
    let mut c = ftol32(v);
    if c > 0xFF {
        c = 0xFF;
    }
    c as u32 & 0xFF
}

/// The environment-camera UV helper (0x5D84C0; catalogue `FUN_005d84c0`, inlined as a lambda in gta-reversed). EAX =
/// environment-map material data, ECX = atomic, ESI = the `CVector2D` out. Translate scales `s = int8 * 0.125 * 50`
/// (f32) from bytes +2 / +3; the position is the clump frame's LTM position (the atomic's frame without a clump);
/// per axis `inv = 1 / s` (kept on the stack), `n = _ftol(p * inv)`, `out = -((p - n * s) * inv)` (f32). Returns 1.
#[hook(addr = 0x5D84C0, conv = reg("args = eax ecx esi"), name = "FUN_005d84c0", expect = "83 EC 18 0F BE 50 02",
       unverified = "boot-render-2", route = "L2", hazards = "none", recs = "grove-walk",
       note = "register arguments EAX (env material data), ECX (atomic), ESI (out vec2); called only from CustomPipeRenderCB")]
pub fn env_cam_uv(env: GPtr<u8>, atomic: GPtr<u8>, out: GPtr<u8>) -> u32 {
    let sx = scaled_i8(env, 2);
    let sy = scaled_i8(env, 3);
    let ltm = atomic_or_clump_ltm(atomic);
    let (px, py) = (rd::<f32>(ltm, 0x30), rd::<f32>(ltm, 0x34));
    let axis = |p: f32, s: f32| {
        let inv = kx(K_ONE).fdiv_m(s);
        let n = ftol32(x(p).fmul(inv));
        xi(n).fmul_m(s).fsubr_m(p).fmul(inv).fchs().to_f32()
    };
    wr::<f32>(out, 0, axis(px, sx));
    wr::<f32>(out, 4, axis(py, sy));
    1
}

/// `|(n * s - p) * inv|` (`fild n; fmul s; fsub p; fmul inv; fabs`), mirrored to `1 - t` for an even `n`.
fn wave_coord(n: i32, s: f32, p: X87, inv: f32) -> X87 {
    let t = xi(n).fmul_m(s).fsub(p).fmul_m(inv).fabs();
    if n & 1 == 0 { t.fsubr_m(k(K_ONE)) } else { t }
}

/// The environment-wave UV helper (0x5D8590; catalogue `FUN_005d8590`, inlined in gta-reversed). ECX = atomic,
/// ESI = environment-map material data, EDI = environment-map atomic data, stack: the `CVector2D` out (cdecl).
///
/// The position `p` is the clump frame's LTM position (the atomic's frame without a clump), translate scales
/// `tsx`, `tsy` as in [`env_cam_uv`]; `u` starts as the atomic data's offset U. When the material's render frame word
/// (+6) differs from `RwEngineInstance`'s (`G + 8`), it is updated and: `invY = 1/tsy`, `invX = 1/tsx` (stored as
/// f32; the integer parts `_ftol(inv * p)` use the unrounded values), the current coordinate sum `cy + cx` and the
/// previous one (from the stored previous position, with the f32 inverses) by [`wave_coord`] (f32 each); the move
/// `d = (p.x - prev.x, p.y - prev.y, 0)` (f32). If `d.x² + d.y² > 0` both `d` and a copy of the LTM's up vector are
/// normalised (`CVector::Normalise`) and, when `(up.z * d.z + d.x * up.x) + up.y * d.y < 0`,
/// `u = offU - |prevSum - currSum|`, plus 1 when below 0; otherwise (no move, or moving forward)
/// `u = |prevSum - currSum| + offU`, minus 1 unless below 1. The atomic data gets `u` and the position x, y (dword
/// copies). Then `v = at.x + at.y` (LTM at vector) is clamped to 0.1 from above (any value not below 0.1, NaN
/// included) and to 0 from below (any value not above 0), and mirrored to `1 - v` when `at.z < 0`.
/// `out = (-u, v)` (f32). Returns the out pointer with its low byte replaced by 1 (`mov eax, [out]; mov al, 1`).
#[hook(addr = 0x5D8590, conv = reg("args = ecx esi edi stack"), name = "FUN_005d8590", expect = "8B 41 3C 83 EC 44",
       unverified = "boot-render-2", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "register arguments ECX (atomic), ESI (env material data), EDI (env atomic data) + the out pointer on the stack; writes the material data's frame word and the atomic data (pool memory)")]
pub fn env_wave_uv(atomic: GPtr<u8>, env: GPtr<u8>, atm: GPtr<u8>, out: GPtr<u8>) -> u32 {
    let ltm = atomic_or_clump_ltm(atomic);
    let tsx = scaled_i8(env, 2);
    let (pos_x, pos_y) = (rd::<f32>(ltm, 0x30), rd::<f32>(ltm, 0x34));
    let frame = rd::<u16>(g::<GPtr<u8>>(RW_ENGINE_INSTANCE).read(), 8);
    let tsy = scaled_i8(env, 3);
    let mut off_u = rd::<f32>(atm, 0);
    if rd::<u16>(env, 6) != frame {
        wr::<u16>(env, 6, frame);
        let inv_y = kx(K_ONE).fdiv_m(tsy);
        let inv_y32 = inv_y.to_f32();
        let ny = ftol32(inv_y.fmul_m(pos_y));
        let inv_x = kx(K_ONE).fdiv_m(tsx);
        let inv_x32 = inv_x.to_f32();
        let nx = ftol32(inv_x.fmul_m(pos_x));
        let cx = wave_coord(nx, tsx, x(pos_x), inv_x32);
        let cy = wave_coord(ny, tsy, x(pos_y), inv_y32);
        let curr_sum = cy.fadd(cx).to_f32();
        let (prev_x, prev_y) = (x(rd::<f32>(atm, 4)), x(rd::<f32>(atm, 8)));
        let ny2 = ftol32(x(inv_y32).fmul(prev_y));
        let nx2 = ftol32(x(inv_x32).fmul(prev_x));
        let px = wave_coord(nx2, tsx, prev_x, inv_x32);
        let py = wave_coord(ny2, tsy, prev_y, inv_y32);
        let prev_sum = py.fadd(px).to_f32();
        let mut d = [x(pos_x).fsub(prev_x).to_f32(), x(pos_y).fsub(prev_y).to_f32(), 0.0f32];
        let zero = kx(K_ZERO);
        let mut backwards = false;
        if !x(d[0]).fmul_m(d[0]).fadd(x(d[1]).fmul_m(d[1])).fcom(zero).test_any(0x41) {
            let mut up = [rd::<f32>(ltm, 0x10), rd::<f32>(ltm, 0x14), rd::<f32>(ltm, 0x18)];
            vector_normalise(ptr(&mut d));
            vector_normalise(ptr(&mut up));
            let dot = x(up[2]).fmul_m(d[2]).fadd(x(d[0]).fmul_m(up[0])).fadd(x(up[1]).fmul_m(d[1]));
            backwards = !dot.fcom(zero).test_parity(5);
        }
        let diff = x(prev_sum).fsub_m(curr_sum).fabs();
        let u = if backwards {
            let t = diff.fsubr_m(off_u);
            if !t.fcom(zero).test_parity(5) { t.fadd_m(k(K_ONE)) } else { t }
        } else {
            let t = diff.fadd_m(off_u);
            if !t.fcom(kx(K_ONE)).c0() { t.fsub_m(k(K_ONE)) } else { t }
        };
        off_u = u.to_f32();
        wr::<f32>(atm, 0, off_u);
        wr::<u32>(atm, 4, rd::<u32>(ltm, 0x30));
        wr::<u32>(atm, 8, rd::<u32>(ltm, 0x34));
    }
    let (at_x, at_y, at_z) = (rd::<f32>(ltm, 0x20), rd::<f32>(ltm, 0x24), rd::<f32>(ltm, 0x28));
    let mut v = x(at_y).fadd_m(at_x);
    if v.fcom(kx(K_TENTH)).test_parity(5) {
        v = kx(K_TENTH);
    } else if v.fcom(kx(K_ZERO)).test_any(0x41) {
        v = kx(K_ZERO);
    }
    if !x(at_z).fcom(kx(K_ZERO)).test_parity(5) {
        v = v.fsubr_m(k(K_ONE));
    }
    wr::<f32>(out, 0, x(off_u).fchs().to_f32());
    wr::<f32>(out, 4, v.to_f32());
    (out.addr() as u32 & !0xFF) | 1
}

/// The D3D9 material set-up (0x5DA790, cdecl; catalogue `FUN_005da790`, inlined in gta-reversed):
/// `(RwSurfaceProperties* surf, RwRGBA* color, uint32 rxGeoFlags, float spec, float power)` with the floats passed as
/// their bits. Fills the `D3DMATERIAL9` at 0x8D12D8 and sets it with `RwD3D9SetMaterial`; returns 1.
///
/// Specular r, g, b = `spec` (dword copies, alpha untouched), power = `power`. With `MODULATE` (flags & 0x40) and a
/// colour other than opaque white (dword `!= 0xFFFFFFFF`): diffuse = `colour * (surf.diffuse / 255)` per channel
/// (alpha `a / 255`), `amb = surf.ambient / 255` (f32); with `PRELIT` (flags & 8) the ambient render state is the
/// colour as ARGB, COLORVERTEX 1, AMBIENTMATERIALSOURCE 1 (colour 1), EMISSIVEMATERIALSOURCE 0, ambient 0 and
/// emissive = `colour * AmbientLightColour * amb`; without it ambient state white, the three states 0, ambient =
/// `colour * AmbientLightColour * amb`, emissive 0. Otherwise diffuse r, g, b = `surf.diffuse`, alpha 1, ambient
/// state white, AMBIENTMATERIALSOURCE 0, COLORVERTEX / EMISSIVEMATERIALSOURCE 1 with `PRELIT` (else 0 and emissive 0),
/// ambient = `AmbientLightColour` copied when `surf.ambient` is exactly 1.0, else multiplied by it.
/// The original reuses its `spec` / `power` argument slots as temporaries (the caller does not read them).
#[hook(addr = 0x5DA790, conv = cdecl, name = "FUN_005da790", expect = "8B 44 24 10 53",
       route = "L2", hazards = "none", recs = "grove-walk",
       note = "writes the D3DMATERIAL9 global 0x8D12D8..0x8D131B; the spec/power argument slots are overwritten by the original")]
pub fn set_material_props(surf: GPtr<u8>, color: GPtr<u8>, flags: u32, spec: u32, power: u32) -> u32 {
    let m = |off: usize| g::<u32>(MATERIAL + off);
    let mf = |off: usize, v: X87| g::<f32>(MATERIAL + off).write(v.to_f32());
    let light = |i: usize| k(AMBIENT_LIGHT + 4 * i);
    m(0x28).write(spec);
    m(0x24).write(spec);
    m(0x20).write(spec);
    m(0x40).write(power);
    let fl = flags & 0x48;
    let c = |i: isize| i32::from(rd::<u8>(color, i));
    if fl & 0x40 != 0 && rd::<u32>(color, 0) != 0xFFFF_FFFF {
        let dd = x(rd::<f32>(surf, 8)).fmul_m(k(K_INV255));
        mf(0x0, xi(c(0)).fmul(dd));
        mf(0x4, xi(c(1)).fmul(dd));
        mf(0x8, xi(c(2)).fmul(dd));
        mf(0xC, xi(c(3)).fmul_m(k(K_INV255)));
        let amb = x(rd::<f32>(surf, 0)).fmul_m(k(K_INV255)).to_f32();
        let lit = |i: isize| xi(c(i)).fmul_m(light(i as usize)).fmul_m(amb);
        if fl & 8 != 0 {
            let argb = (c(3) as u32) << 24 | (c(0) as u32) << 16 | (c(1) as u32) << 8 | c(2) as u32;
            set_render_state(0x8B, argb);
            set_render_state(0x8D, 1);
            set_render_state(0x93, 1);
            set_render_state(0x94, 0);
            for off in [0x10, 0x14, 0x18] {
                m(off).write(0);
            }
            mf(0x30, lit(0));
            mf(0x34, lit(1));
            mf(0x38, lit(2));
        } else {
            set_render_state(0x8B, 0xFFFF_FFFF);
            set_render_state(0x8D, 0);
            set_render_state(0x93, 0);
            set_render_state(0x94, 0);
            mf(0x10, lit(0));
            mf(0x14, lit(1));
            for off in [0x30, 0x34, 0x38] {
                m(off).write(0);
            }
            mf(0x18, lit(2));
        }
    } else {
        let d = x(rd::<f32>(surf, 8));
        mf(0x0, d);
        mf(0x4, d);
        m(0xC).write(0x3F80_0000);
        mf(0x8, d);
        set_render_state(0x8B, 0xFFFF_FFFF);
        set_render_state(0x93, 0);
        if fl & 8 != 0 {
            set_render_state(0x8D, 1);
            set_render_state(0x94, 1);
        } else {
            set_render_state(0x8D, 0);
            set_render_state(0x94, 0);
            for off in [0x30, 0x34, 0x38] {
                m(off).write(0);
            }
        }
        let a = rd::<u32>(surf, 0);
        if a == 0x3F80_0000 {
            for i in 0..3 {
                m(0x10 + 4 * i).write(g::<u32>(AMBIENT_LIGHT + 4 * i).read());
            }
        } else {
            for i in 0..3 {
                mf(0x10 + 4 * i, x(light(i)).fmul_m(f32::from_bits(a)));
            }
        }
    }
    set_material(g::<u8>(MATERIAL));
    1
}

/// The colours a vehicle's paintable material carries in its model (the primary / secondary / tertiary /
/// quaternary placeholders and the light colours): such materials are lit as black (alpha kept). Compared as the
/// little-endian dword of r, g, b (0xBBGGRR).
const PLACEHOLDER_COLOURS: [u32; 8] = [0xAF00FF, 0xFF3C, 0x3CFF, 0xAFFF, 0xFFB9, 0xC8FF00, 0xFF00FF, 0xFFFF00];

/// `CCustomCarEnvMapPipeline::CustomPipeRenderCB(RwResEntry*, void* object, RwUInt8 type, RwUInt32 flags)` (0x5D9900,
/// cdecl), the render callback of the vehicle pipeline (gta-reversed reverses it; this follows the bytes).
///
/// `specIntensity = [0x8D12D0] * 1.85` (f32); `noReflections` = atomic id & 0x6000; clipping as needed; lighting
/// queried (`D3DRS_LIGHTING`); unlit without `PRELIT` (flags & 8): no texture, texture factor black, stage 0
/// SELECTARG2 of TFACTOR. Indices, streams, vertex declaration; per mesh (count as int32, none when ≤ 0):
/// - material flags (surface specular dword): environment camera (bit 0) and wave (bit 1, geometry `TEXTURED2`)
///   unless `noReflections`; specular from bit 2 when `g_fx.GetFxQuality() >= 2`, else bit 2 unless `noReflections`.
///   Specular with lighting on: light 1 = the specular light (0xC02CB0) enabled, `spec = specIntensity *
///   specularity * 2` (capped to 1 when above), `power = specularity * 100`, SPECULARENABLE 1, LOCALVIEWER 0,
///   SPECULARMATERIALSOURCE 0; else SPECULARENABLE 0 and both 0. Stages 1 and 2 disabled.
/// - environment camera: texture address wrap; the transform 0xC02D78 gets the scales (int8 * 0.125) and the UV
///   offset of [`env_cam_uv`]; stage-1 texture, factor `min(_ftol(shininess / 255 * specIntensity * 254), 255)` grey,
///   MULTIPLYADD with camera-space normals, projected 3-component coordinates.
/// - environment wave: texture address wrap; the atomic data allocated from the pool when missing (zeroed; when the
///   pool is full the UV offset stays 0); the transform 0xC02D38 gets identity scales and [`env_wave_uv`]'s offset;
///   stage-1 texture, factor alpha `min(_ftol(specIntensity * 24), 255)` over white, BLENDFACTORALPHA.
/// - vertex alpha from the instance or the material alpha; unlit: `RxD3D9InstanceDataRender`; else, with lighting on,
///   the material colour (placeholder colours black) through [`set_material_props`], then
///   `RxD3D9InstanceDataRenderLighting` with the material texture. Specular with lighting on is switched off again
///   (SPECULARENABLE 0, light 1 disabled).
///
/// Afterwards 0x5DAAC0 and stage 1 reset (COLOROP / ALPHAOP disabled, TEXCOORDINDEX 1, transform flags 0). The
/// original reuses its `resEntry` and `type` argument slots for per-mesh temporaries (the caller does not read them).
#[hook(addr = 0x5D9900, conv = cdecl, name = "CCustomCarEnvMapPipeline::CustomPipeRenderCB", expect = "83 EC 3C D9 05 D0 12 8D 00",
       unverified = "boot-render-2", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "allocates environment-map atomic data from its pool (0xC02D2C) and writes it and the material data's frame word; global transform matrices 0xC02D38 / 0xC02D78; argument slots reused by the original")]
pub fn car_env_map_render_cb(entry: GPtr<u8>, object: GPtr<u8>, ty: u32, flags: u32) {
    let spec_intensity = kx(SPEC_BASE).fmul_m(k(K_SPEC_SCALE)).to_f32();
    let mut lighting = 0u32;
    let geo_flags = rd::<u32>(rd::<GPtr<u8>>(object, 0x18), 8);
    let no_reflections = (get_atomic_id(object) >> 8) & 0x60 != 0;
    enable_clipping_if_needed(object, ty & 0xFF);
    get_render_state(0x89, GPtr::from_raw(&raw mut lighting));
    let unlit = lighting == 0 && flags & 8 == 0;
    if unlit {
        set_texture(0, 0);
        set_render_state(0x3C, 0xFF00_0000);
        for t in [1, 3, 4, 6] {
            set_tss(0, t, 3);
        }
    }
    let h = entry.byte_add(0x18);
    let ib = rd::<u32>(h, 8);
    if ib != 0 {
        set_indices(ib);
    }
    set_streams(h.byte_add(0x10), rd::<u32>(h, 0x30));
    set_vertex_declaration(rd::<u32>(h, 0x34));
    let n = rd::<i32>(h, 4);
    let mut inst = h.byte_add(0x40);
    for _ in 0..n.max(0) {
        let mat = rd::<GPtr<u8>>(inst, 8);
        let mflags = rd::<u32>(mat, 0x10);
        let env = rd::<GPtr<u8>>(mat, g::<i32>(ENV_MAT_OFFSET).read() as isize);
        let env_cam = mflags & 1 != 0 && !no_reflections;
        let env_wave = mflags & 2 != 0 && !no_reflections && (geo_flags >> 7) & 1 != 0;
        let use_spec = if get_fx_quality(g::<u8>(G_FX)) >= 2 { (mflags >> 2) & 1 != 0 } else { mflags & 4 != 0 && !no_reflections };
        let (spec, power) = if use_spec && lighting != 0 {
            let sp = rd::<GPtr<u8>>(mat, g::<i32>(SPEC_MAT_OFFSET).read() as isize);
            set_light(1, g::<u8>(SPEC_LIGHT));
            enable_light(1, 1);
            let s = x(spec_intensity).fmul_m(rd::<f32>(sp, 0));
            let s = s.fadd(s);
            let spec = if s.fcom(kx(K_ONE)).test_any(0x41) { s.to_f32().to_bits() } else { 0x3F80_0000 };
            let power = x(rd::<f32>(sp, 0)).fmul_m(k(K_HUNDRED)).to_f32().to_bits();
            set_render_state(0x1D, 1);
            set_render_state(0x8E, 0);
            set_render_state(0x92, 0);
            (spec, power)
        } else {
            set_render_state(0x1D, 0);
            (0, 0)
        };
        set_tss(1, 1, 1);
        set_tss(2, 1, 1);
        if env_cam {
            rw_render_state_set(2, 1);
            g::<f32>(CAM_MATRIX).write(xi(i32::from(rd::<i8>(env, 0))).fmul_m(k(K_EIGHTH)).to_f32());
            g::<u32>(CAM_MATRIX + 0x3C).write(0x3F80_0000);
            g::<u32>(CAM_MATRIX + 0x28).write(0x3F80_0000);
            g::<f32>(CAM_MATRIX + 0x14).write(xi(i32::from(rd::<i8>(env, 1))).fmul_m(k(K_EIGHTH)).to_f32());
            let mut uv = [0u32; 2];
            env_cam_uv_call(env, object, ptr(&mut uv));
            g::<u32>(CAM_MATRIX + 0x20).write(uv[0]);
            g::<u32>(CAM_MATRIX + 0x24).write(uv[1]);
            set_transform(0x11, g::<u8>(CAM_MATRIX));
            set_texture(rd::<u32>(env, 8), 1);
            let shine = xi(i32::from(rd::<u8>(env, 4))).fmul_m(k(K_INV255)).fmul_m(spec_intensity).fmul_m(k(K_254));
            let c = clamp_byte(shine);
            set_render_state(0x3C, 0xFF00_0000 | c << 16 | c << 8 | c);
            for (t, v) in [(1, 0x19), (0x1A, 1), (2, 2), (3, 3), (4, 3), (5, 2), (6, 1), (0xB, 0x10001), (0x18, 0x103)] {
                set_tss(1, t, v);
            }
            set_tss(2, 1, 1);
        }
        if env_wave {
            rw_render_state_set(2, 1);
            let slot = object.byte_add(g::<i32>(ENV_ATM_OFFSET).read() as isize).cast::<GPtr<u8>>();
            let mut atm = slot.read();
            let mut uv = [0u32; 2];
            if atm.is_null() {
                let p = env_atm_pool_new(g::<GPtr<u8>>(ENV_ATM_POOL).read());
                slot.write(p);
                if !p.is_null() {
                    atm = p;
                    wr::<u32>(atm, 0, 0);
                    wr::<u32>(atm, 8, 0);
                    wr::<u32>(atm, 4, 0);
                }
            }
            if !atm.is_null() {
                env_wave_uv_call(object, env, atm, ptr(&mut uv));
            }
            for off in [0x3C, 0x28, 0x14, 0x0] {
                g::<u32>(WAVE_MATRIX + off).write(0x3F80_0000);
            }
            g::<u32>(WAVE_MATRIX + 0x20).write(uv[0]);
            g::<u32>(WAVE_MATRIX + 0x24).write(uv[1]);
            set_transform(0x11, g::<u8>(WAVE_MATRIX));
            set_texture(rd::<u32>(env, 8), 1);
            let c = clamp_byte(x(spec_intensity).fmul_m(k(K_24)));
            set_render_state(0x3C, c << 24 | 0x00FF_FFFF);
            for (t, v) in [(1, 0xE), (2, 2), (3, 1), (4, 3), (5, 2), (6, 1), (0xB, 1), (0x18, 2)] {
                set_tss(1, t, v);
            }
            set_tss(2, 1, 1);
        }
        let alpha = rd::<u32>(inst, 0xC) != 0 || rd::<u8>(rd::<GPtr<u8>>(inst, 8), 7) != 0xFF;
        vertex_alpha_enable(u32::from(alpha));
        if unlit {
            instance_data_render(h, inst);
        } else {
            if lighting != 0 {
                let m = rd::<GPtr<u8>>(inst, 8);
                let mut col = [rd::<u8>(m, 4), rd::<u8>(m, 5), rd::<u8>(m, 6), rd::<u8>(m, 7)];
                if PLACEHOLDER_COLOURS.contains(&(u32::from_le_bytes(col) & 0xFF_FFFF)) {
                    col[2] = 0;
                    col[1] = 0;
                    col[0] = 0;
                }
                set_material_props_call(m.byte_add(0xC), ptr(&mut col), flags, spec, power);
            }
            instance_data_render_lighting(h, inst, flags, rd::<u32>(rd::<GPtr<u8>>(inst, 8), 0));
        }
        if use_spec && lighting != 0 {
            set_render_state(0x1D, 0);
            enable_light(1, 0);
        }
        inst = inst.byte_add(0x24);
    }
    render_cb_end();
    for (t, v) in [(1, 1), (4, 1), (0xB, 1), (0x18, 0)] {
        set_tss(1, t, v);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_ENV_CAM_UV, HOOK_ENV_WAVE_UV, HOOK_SET_MATERIAL_PROPS, HOOK_CAR_ENV_MAP_RENDER_CB];
