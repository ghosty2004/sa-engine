//! `CCustomCarEnvMapPipeline` specular-material stream read (STD-6, batch `std6-render-1`). Ported from
//! the 1.0 US bytes (0x5D9880).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, wr};

orig! {
    /// `RwStreamRead(stream, buf, len)` (0x7EC9D0)
    fn stream_read(s: GPtr<u8>, buf: GPtr<u8>, len: u32) -> u32 = cdecl 0x7EC9D0 => unported;
    /// `RwTextureRead(name, mask)` (0x7F3AC0)
    fn texture_read(name: GPtr<u8>, mask: u32) -> u32 = cdecl 0x7F3AC0 => unported;
    /// The specular-material pool's `New` (0x5D93B0, thiscall)
    fn spec_pool_new(pool: GPtr<u8>) -> GPtr<u8> = thiscall 0x5D93B0 => crate::mvpa_render2::spec_map_material_pool_new;
}

/// `CCustomCarEnvMapPipeline::ms_specularMapPluginOffset` and the specular-material pool pointer.
const SPEC_OFFSET: usize = 0x8D12CC;
const SPEC_POOL: usize = 0xC02D30;

/// `CCustomCarEnvMapPipeline::pluginSpecMatStreamReadCB(stream, length, material, offset, size)`
/// (0x5D9880): reads `length` bytes into a stack record {specularity f32, texture name}; a specularity
/// that is not 0.0 (`fcomp` with 0x858B50; NaN counts as non-zero) loads the texture (`RwTextureRead`,
/// no mask) and, when found, a pool entry {specularity, texture} becomes the material's plugin data
/// (NULL when the pool is full). Returns the stream.
#[hook(addr = 0x5D9880, conv = cdecl, name = "CCustomCarEnvMapPipeline::pluginSpecMatStreamReadCB", expect = "8B 44 24 0C 8B 0D CC 12 8D 00",
       unverified = "std6-render-1", route = "L2", hazards = "heap-writes,uninit-locals", recs = "new-game",
       note = "the record is a 0x1C-byte stack buffer read without a bound (a longer chunk overruns it in the original)")]
pub fn spec_mat_stream_read(s: GPtr<u8>, len: u32, material: GPtr<u8>, _offset: i32, _size: i32) -> GPtr<u8> {
    let slot = material.byte_add(g::<i32>(SPEC_OFFSET).read() as isize);
    let mut rec = [0u8; 0x100];
    let n = len.min(rec.len() as u32);
    stream_read(s, GPtr::from_raw(rec.as_mut_ptr()), n);
    let spec = f32::from_le_bytes([rec[0], rec[1], rec[2], rec[3]]);
    if spec == 0.0 {
        return s;
    }
    let tex = texture_read(GPtr::from_raw(rec[4..].as_mut_ptr()), 0);
    if tex == 0 {
        return s;
    }
    let p = spec_pool_new(GPtr::from_addr(g::<u32>(SPEC_POOL).read() as usize));
    slot.cast::<u32>().write(p.addr() as u32);
    if !p.is_null() {
        wr::<u32>(p, 4, tex);
        wr::<f32>(p, 0, spec);
    }
    s
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_SPEC_MAT_STREAM_READ];
