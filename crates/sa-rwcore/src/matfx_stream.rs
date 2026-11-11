//! RpMatFX material stream read (STD-6, batch `std6-rw-1`). Ported from the 1.0 US bytes (0x811410,
//! 0x816280). Layouts as in `matfx.rs`: `MatFXData` (0x34) has two 0x18-byte effect slots (type at
//! slot +0x14).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{RwError, f32_of, g, p8, rd, rw_memory_alloc, wr, x};

orig! {
    /// `RwStreamReadInt32(stream, buf, size)` (0x7ED540)
    fn read_int32(s: GPtr<u8>, p: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x7ED540 => crate::stream::rw_stream_read_int32;
    /// `RwStreamReadReal(stream, buf, size)` (0x7ED4F0)
    fn read_real(s: GPtr<u8>, p: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x7ED4F0 => crate::stream::rw_stream_read_real;
    /// `RwStreamFindChunk(stream, type, len, ver)` (0x7ED2D0)
    fn find_chunk(s: GPtr<u8>, ty: u32, len: GPtr<u32>, ver: GPtr<u32>) -> u32 = cdecl 0x7ED2D0 => crate::stream::rw_stream_find_chunk;
    /// `RwErrorGet(out)` (0x808880)
    fn error_get(out: GPtr<RwError>) -> GPtr<RwError> = cdecl 0x808880 => crate::engine::rw_error_get;
    /// `RwErrorSet(err)` (0x808820)
    fn error_set(e: GPtr<RwError>) -> GPtr<RwError> = cdecl 0x808820 => unported;
    /// `RwTextureStreamRead(stream)` (0x8046E0)
    fn texture_stream_read(s: GPtr<u8>) -> GPtr<u8> = cdecl 0x8046E0 => crate::texture::rw_texture_stream_read;
    /// `RwTextureDestroy(tex)` (0x7F3820)
    fn texture_destroy(t: GPtr<u8>) -> u32 = cdecl 0x7F3820 => crate::texture::rw_texture_destroy;
    /// `RpMatFXMaterialSetEffects(material, flags)` (0x811C80)
    fn set_effects(m: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x811C80 => crate::matfx::rp_matfx_material_set_effects;
    /// `_rpMatFXMaterialFindEffect(material, type)` (0x811150)
    fn find_effect(m: GPtr<u8>, ty: u32) -> GPtr<u8> = cdecl 0x811150 => crate::matfx::rp_matfx_material_find_effect;
    /// `_rpMatFXSetupDualRenderState(slot, state)` (0x816280)
    fn setup_dual_state(slot: GPtr<u8>, state: u32) -> u32 = cdecl 0x816280 => matfx_setup_dual_render_state;
    /// `RpMatFXMaterialSetDualBlendModes(material, src, dst)` (0x8128C0)
    fn set_dual_blend_modes(m: GPtr<u8>, src: u32, dst: u32) -> GPtr<u8> = cdecl 0x8128C0 => crate::matfx::rp_matfx_material_set_dual_blend_modes;
    /// `RpMatFXMaterialSetBumpMapTexture(material, tex)` (0x812160)
    fn set_bump_map_texture(m: GPtr<u8>, t: GPtr<u8>) -> GPtr<u8> = cdecl 0x812160 => crate::matfx::rp_matfx_material_set_bump_map_texture;
}

/// Material plugin offset (`[0xC9AB74]`) and the `MatFXData` freelist (`[0xC9AB80]`).
const MATFX_MATERIAL_OFFSET: usize = 0xC9AB74;
const MATFX_FREELIST: usize = 0xC9AB80;

fn data_of(m: GPtr<u8>) -> GPtr<u8> {
    p8(rd::<u32>(m, g::<i32>(MATFX_MATERIAL_OFFSET).read() as isize))
}

/// The first of the two slots (0, then 1) whose type is `ty`, NULL when none (the callers write
/// through it anyway, as the original does).
fn slot_of_type(m: GPtr<u8>, ty: u32) -> GPtr<u8> {
    let d = data_of(m);
    for i in 0..2 {
        if rd::<u32>(d, i * 0x18 + 0x14) == ty {
            return d.byte_add(i * 0x18);
        }
    }
    GPtr::NULL
}

/// The texture chunk of an effect: `RwStreamFindChunk(stream, rwID_TEXTURE, NULL, NULL)`, then
/// `RwTextureStreamRead`; a NULL texture whose `RwErrorGet` code is neither 0x80000000 nor 0x16 sets
/// the error again and fails. `Err(())` = the original returns NULL.
fn read_texture(s: GPtr<u8>) -> Result<GPtr<u8>, ()> {
    if find_chunk(s, 6, GPtr::NULL, GPtr::NULL) == 0 {
        return Err(());
    }
    let mut e = RwError::default();
    error_get(GPtr::from_raw(&mut e));
    let t = texture_stream_read(s);
    if !t.is_null() {
        return Ok(t);
    }
    error_get(GPtr::from_raw(&mut e));
    if e.error_code as u32 != 0x8000_0000 && e.error_code != 0x16 {
        error_set(GPtr::from_raw(&mut e));
        return Err(());
    }
    Ok(GPtr::NULL)
}

fn read_i32(s: GPtr<u8>) -> Option<i32> {
    let mut v = 0i32;
    if read_int32(s, GPtr::from_raw(&mut v as *mut i32).cast(), 4).is_null() { None } else { Some(v) }
}

fn read_f32(s: GPtr<u8>) -> Option<f32> {
    let mut v = 0f32;
    if read_real(s, GPtr::from_raw(&mut v as *mut f32).cast(), 4).is_null() { None } else { Some(v) }
}

/// `_rpMatFXSetupDualRenderState(slot, state)` (0x816280): returns 1 (the D3D9 build does nothing).
#[hook(addr = 0x816280, conv = cdecl, name = "FUN_00816280", expect = "B8 01 00 00 00 C3",
       unverified = "std6-rw-1", route = "L1-pure", hazards = "none")]
pub fn matfx_setup_dual_render_state(_slot: GPtr<u8>, _state: u32) -> u32 {
    1
}

/// `cb_MATERIALEFFECTSPLUGIN_RpMaterial_StreamRead(stream, length, material, offset, size)` (0x811410):
/// creates the material's `MatFXData` when missing (freelist, hint 0x30120, 13 words zeroed; NULL
/// returns NULL), reads the effect flags and applies them (`RpMatFXMaterialSetEffects`), then for each of
/// the two slots an effect type: bump map (1): coefficient (real), a texture flag and texture, a bumped
/// texture flag and texture; with a texture the slot (this iteration's) keeps texture +4, bumped texture
/// +8 and `1 / raster width` +0x10, else a bumped texture alone goes through
/// `RpMatFXMaterialSetBumpMapTexture` (then released), else +4 = +8 = 0; the bump slot's +0xC =
/// -coefficient. Environment map (2): coefficient, frame-buffer alpha, texture flag and texture; the
/// texture replaces the env slot's +4 (one reference added, the stream's dropped); the env slot's +8 /
/// +0xC = coefficient / alpha. Dual (4): src and dst blend (8 bytes), texture flag and texture; the
/// texture replaces the dual slot's +0 (`_rpMatFXSetupDualRenderState` 2 and 1), then
/// `RpMatFXMaterialSetDualBlendModes`. Any other type is skipped. Every failed read returns NULL; a
/// NULL stream after a texture read also returns NULL (the original's check).
#[hook(addr = 0x811410, conv = cdecl, name = "cb_MATERIALEFFECTSPLUGIN_RpMaterial_StreamRead", expect = "83 EC 60 A1 74 AB C9 00",
       unverified = "std6-rw-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn matfx_material_stream_read(s: GPtr<u8>, _len: i32, m: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let mut d = data_of(m);
    if d.is_null() {
        d = rw_memory_alloc(p8(g::<u32>(MATFX_FREELIST).read()), 0x30120);
        if d.is_null() {
            return GPtr::NULL;
        }
        for i in 0..13 {
            wr::<u32>(d, i * 4, 0);
        }
        wr::<u32>(m, g::<i32>(MATFX_MATERIAL_OFFSET).read() as isize, d.addr() as u32);
    }
    let Some(flags) = read_i32(s) else {
        return GPtr::NULL;
    };
    set_effects(m, flags as u32);
    for i in 0..2isize {
        let Some(ty) = read_i32(s) else {
            return GPtr::NULL;
        };
        match ty {
            1 => {
                let slot = d.byte_add(i * 0x18);
                let Some(coef) = read_f32(s) else {
                    return GPtr::NULL;
                };
                let Some(has_tex) = read_i32(s) else {
                    return GPtr::NULL;
                };
                let tex = if has_tex != 0 {
                    match read_texture(s) {
                        Ok(t) => t,
                        Err(()) => return GPtr::NULL,
                    }
                } else {
                    GPtr::NULL
                };
                if s.is_null() {
                    return GPtr::NULL;
                }
                let release = |t: GPtr<u8>| {
                    if !t.is_null() {
                        texture_destroy(t);
                    }
                };
                let Some(has_bumped) = read_i32(s) else {
                    release(tex);
                    return GPtr::NULL;
                };
                let bumped = if has_bumped != 0 {
                    match read_texture(s) {
                        Ok(t) => t,
                        Err(()) => {
                            release(tex);
                            return GPtr::NULL;
                        }
                    }
                } else {
                    GPtr::NULL
                };
                if !tex.is_null() {
                    wr::<u32>(slot, 4, tex.addr() as u32);
                    wr::<u32>(slot, 8, bumped.addr() as u32);
                    let w = rd::<i32>(p8(rd::<u32>(tex, 0)), 0xC);
                    // fild; fdivr 1.0 (0x858624); fstp dword
                    wr::<f32>(slot, 0x10, f32_of(x(1.0).fdiv(X87::from_i32(w))));
                } else if !bumped.is_null() {
                    set_bump_map_texture(m, bumped);
                    texture_destroy(bumped);
                } else {
                    wr::<u32>(slot, 4, 0);
                    wr::<u32>(slot, 8, 0);
                }
                wr::<f32>(slot_of_type(m, 1), 0xC, -coef);
            }
            2 => {
                let Some(coef) = read_f32(s) else {
                    return GPtr::NULL;
                };
                let Some(fb_alpha) = read_i32(s) else {
                    return GPtr::NULL;
                };
                let Some(has_tex) = read_i32(s) else {
                    return GPtr::NULL;
                };
                let tex = if has_tex != 0 {
                    match read_texture(s) {
                        Ok(t) => t,
                        Err(()) => return GPtr::NULL,
                    }
                } else {
                    GPtr::NULL
                };
                if s.is_null() {
                    return GPtr::NULL;
                }
                if !tex.is_null() {
                    let e = find_effect(m, 2);
                    wr::<i32>(tex, 0x54, rd::<i32>(tex, 0x54).wrapping_add(1));
                    let old = p8(rd::<u32>(e, 4));
                    if !old.is_null() {
                        texture_destroy(old);
                        wr::<u32>(e, 4, 0);
                    }
                    wr::<u32>(e, 4, tex.addr() as u32);
                    texture_destroy(tex);
                }
                wr::<f32>(slot_of_type(m, 2), 8, coef);
                wr::<i32>(slot_of_type(m, 2), 0xC, fb_alpha);
            }
            4 => {
                let mut blend = [0i32; 2];
                if read_int32(s, GPtr::from_raw(blend.as_mut_ptr()).cast(), 8).is_null() {
                    return GPtr::NULL;
                }
                let Some(has_tex) = read_i32(s) else {
                    return GPtr::NULL;
                };
                let tex = if has_tex != 0 {
                    match read_texture(s) {
                        Ok(t) => t,
                        Err(()) => return GPtr::NULL,
                    }
                } else {
                    GPtr::NULL
                };
                if s.is_null() {
                    return GPtr::NULL;
                }
                if !tex.is_null() {
                    let e = find_effect(m, 4);
                    wr::<i32>(tex, 0x54, rd::<i32>(tex, 0x54).wrapping_add(1));
                    let old = p8(rd::<u32>(e, 0));
                    if !old.is_null() {
                        texture_destroy(old);
                        wr::<u32>(e, 0, 0);
                    }
                    wr::<u32>(e, 0, tex.addr() as u32);
                    setup_dual_state(e, 2);
                    setup_dual_state(e, 1);
                    texture_destroy(tex);
                }
                set_dual_blend_modes(m, blend[0] as u32, blend[1] as u32);
            }
            _ => {}
        }
    }
    s
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_MATFX_SETUP_DUAL_RENDER_STATE, HOOK_MATFX_MATERIAL_STREAM_READ];
