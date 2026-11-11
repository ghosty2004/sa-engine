//! IMAGEMODULE engine-module callbacks and `RwImageSetGamma` (PORT-RW-1 follow-up, written for the
//! native boot). The RwImage API itself belongs to the raster/image port (`image.rs`, PORT-RW-4).
//! Ported from the 1.0 US bytes.
//!
//! Module `M = G + [0xC9A65C]`: +0 image freelist, +4 search-path buffer, +8 its size, +0xC gamma
//! table (256 bytes), +0x10C inverse gamma table (256 bytes), +0x20C gamma (f32), +0x210 scratch
//! buffer, +0x214 its size, +0x218 file-format freelist (0x34-byte entries), +0x21C format list
//! (next at +0x30).

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, g, p8, rd, rw_free, rw_globals, rw_malloc, rw_memory_free, wr, x};

/// IMAGEMODULE offset global.
const IMAGE_MODULE: usize = 0xC9A65C;

sa_core::globals! {
    list IMAGE_GLOBALS;
    /// IMAGEMODULE open-instance counter
    pub static IMAGE_INSTANCES @ 0xC9A660: i32 = 0;
    /// Static space of the image freelist (`RwFreeList`, 0x24 bytes)
    pub static IMAGE_FL_SPACE @ 0xC9A664: [u32; 9] = [0; 9];
    /// Static space of the file-format freelist (0x24 bytes)
    pub static FORMAT_FL_SPACE @ 0xC9A638: [u32; 9] = [0; 9];
    /// Image freelist entries per block, also used for the format freelist (0x80)
    pub static IMAGE_FL_PER_BLOCK @ 0x8E2688: u32 = 0x80;
    /// Image freelist preallocated blocks (1)
    pub static IMAGE_FL_PREALLOC @ 0x8E268C: u32 = 1;
    /// Format freelist entries per block (3; not read by the module open, see `image_module_open`)
    pub static FORMAT_FL_PER_BLOCK @ 0x8E2690: u32 = 3;
    /// Format freelist preallocated blocks (1)
    pub static FORMAT_FL_PREALLOC @ 0x8E2694: u32 = 1;
}

/// The image plugin registry's `sizeOfStruct` (0x1C at start; the registry is RW-4's).
const IMAGE_REGISTRY_SIZE: usize = 0x8E2670;

/// `1.0f` (0x858624), `1/255` (0x859A3C), `255.0` (0x859AAC), `0.5` (0x858B8C).
const K_ONE: u32 = 0x3F80_0000;
const K_INV255: u32 = 0x3B80_8081;
const K_255: u32 = 0x437F_0000;
const K_HALF: u32 = 0x3F00_0000;

orig! {
    /// `RwFreeListCreateAndPreallocateSpace` (0x801B70)
    fn free_list_create_prealloc(size: u32, per_block: u32, align: u32, n: u32, space: GPtr<u8>, hint: u32) -> GPtr<u8> = cdecl 0x801B70 => crate::memory::rw_freelist_create_and_preallocate_space;
    /// `RwFreeListDestroy(fl)` (0x801B80)
    fn free_list_destroy(fl: GPtr<u8>) -> u32 = cdecl 0x801B80 => crate::memory::rw_freelist_destroy;
    /// `RwImageSetGamma(gamma)` (0x803FE0)
    fn image_set_gamma_call(gamma: f32) -> i32 = cdecl 0x803FE0 => rw_image_set_gamma;
    /// CRT `pow(double, double)` (0x8220F0; takes the SSE2 path when available, so no host model)
    fn crt_pow(a: f64, b: f64) -> f64 = cdecl 0x8220F0 => unported;
}

/// `M = G + [0xC9A65C]`, recomputed on every access as the original reloads both globals.
fn imod() -> GPtr<u8> {
    let off = g::<u32>(IMAGE_MODULE).read();
    rw_globals().byte_add(off as isize)
}

fn a32(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

/// `cb_IMAGEMODULE_RwEngine_Open(instance, offset, size)` (0x802280): image freelist (entry size from
/// the registry, hint 0x40018), file-format freelist (entry 0x34, hint 0x40406; quirk kept: its
/// entries-per-block is read from the image freelist's 0x8E2688, not 0x8E2690), a 0x100-byte path
/// buffer (`rwmalloc(.., 0x1040406)`, first byte 0), instance count, `RwImageSetGamma(1.0)`, empty
/// format list, a 0x100-byte scratch buffer (`0x1040018`). Each failure undoes the earlier steps in
/// reverse order (without the gamma and the instance count) and returns NULL; else `instance`.
#[hook(addr = 0x802280, conv = cdecl, name = "cb_IMAGEMODULE_RwEngine_Open", expect = "8B 44 24 08 8B 0D 8C 26 8E 00")]
pub fn image_module_open(instance: GPtr<u8>, offset: u32, _size: u32) -> GPtr<u8> {
    let prealloc = IMAGE_FL_PREALLOC.get();
    let per_block = IMAGE_FL_PER_BLOCK.get();
    g::<u32>(IMAGE_MODULE).write(offset);
    let size = g::<u32>(IMAGE_REGISTRY_SIZE).read();
    let fl = free_list_create_prealloc(size, per_block, 4, prealloc, IMAGE_FL_SPACE.ptr().cast(), 0x40018);
    wr::<u32>(imod(), 0, a32(fl));
    if rd::<u32>(imod(), 0) == 0 {
        return GPtr::NULL;
    }
    let fl = free_list_create_prealloc(
        0x34,
        IMAGE_FL_PER_BLOCK.get(),
        4,
        FORMAT_FL_PREALLOC.get(),
        FORMAT_FL_SPACE.ptr().cast(),
        0x40406,
    );
    wr::<u32>(imod(), 0x218, a32(fl));
    if rd::<u32>(imod(), 0x218) == 0 {
        free_list_destroy(p8(rd::<u32>(imod(), 0)));
        wr::<u32>(imod(), 0, 0);
        return GPtr::NULL;
    }
    wr::<u32>(imod(), 8, 0x100);
    let buf = rw_malloc(rd::<u32>(imod(), 8), 0x1040406);
    wr::<u32>(imod(), 4, a32(buf));
    let buf = rd::<u32>(imod(), 4);
    if buf == 0 {
        free_list_destroy(p8(rd::<u32>(imod(), 0x218)));
        wr::<u32>(imod(), 0x218, 0);
        free_list_destroy(p8(rd::<u32>(imod(), 0)));
        wr::<u32>(imod(), 0, 0);
        return GPtr::NULL;
    }
    wr::<u8>(p8(buf), 0, 0);
    IMAGE_INSTANCES.set(IMAGE_INSTANCES.get().wrapping_add(1));
    image_set_gamma_call(f32::from_bits(K_ONE));
    wr::<u32>(imod(), 0x21C, 0);
    wr::<u32>(imod(), 0x214, 0x100);
    let scratch = rw_malloc(rd::<u32>(imod(), 0x214), 0x1040018);
    wr::<u32>(imod(), 0x210, a32(scratch));
    if rd::<u32>(imod(), 0x210) == 0 {
        rw_free(p8(rd::<u32>(imod(), 4)));
        wr::<u32>(imod(), 4, 0);
        wr::<u32>(imod(), 8, 0);
        free_list_destroy(p8(rd::<u32>(imod(), 0x218)));
        wr::<u32>(imod(), 0x218, 0);
        free_list_destroy(p8(rd::<u32>(imod(), 0)));
        wr::<u32>(imod(), 0, 0);
        return GPtr::NULL;
    }
    instance
}

/// `cb_IMAGEMODULE_RwEngine_Close(instance, offset, size)` (0x8024E0): frees the scratch buffer
/// (pointer and size cleared) and the path buffer (pointer and size cleared) when present, returns
/// every file-format entry to the format freelist (list head advanced before each free), destroys the
/// format freelist and the image freelist when present, decrements the instance count. Returns
/// `instance`.
#[hook(addr = 0x8024E0, conv = cdecl, name = "cb_IMAGEMODULE_RwEngine_Close", expect = "8B 15 5C A6 C9 00")]
pub fn image_module_close(instance: GPtr<u8>, _offset: u32, _size: u32) -> GPtr<u8> {
    let s = rd::<u32>(imod(), 0x210);
    if s != 0 {
        rw_free(p8(s));
        wr::<u32>(imod(), 0x210, 0);
        wr::<u32>(imod(), 0x214, 0);
    }
    let b = rd::<u32>(imod(), 4);
    if b != 0 {
        rw_free(p8(b));
        wr::<u32>(imod(), 4, 0);
        wr::<u32>(imod(), 8, 0);
    }
    loop {
        let c = rd::<u32>(imod(), 0x21C);
        if c == 0 {
            break;
        }
        wr::<u32>(imod(), 0x21C, rd::<u32>(p8(c), 0x30));
        rw_memory_free(p8(rd::<u32>(imod(), 0x218)), p8(c));
    }
    let fl = rd::<u32>(imod(), 0x218);
    if fl != 0 {
        free_list_destroy(p8(fl));
        wr::<u32>(imod(), 0x218, 0);
    }
    let fl = rd::<u32>(imod(), 0);
    if fl != 0 {
        free_list_destroy(p8(fl));
        wr::<u32>(imod(), 0, 0);
    }
    IMAGE_INSTANCES.set(IMAGE_INSTANCES.get().wrapping_sub(1));
    instance
}

/// `fistp dword` under round-toward-zero of an f32-loaded value: the truncated integer, or the
/// integer indefinite 0x80000000 for NaN and values outside the i32 range.
fn fistp_chop(v: f32) -> i32 {
    if v.is_nan() {
        return i32::MIN;
    }
    let t = v.trunc();
    if t >= 2_147_483_648.0 || t < -2_147_483_648.0 {
        i32::MIN
    } else {
        t as i32
    }
}

/// One table byte: `(int)(f32)(pow(x, e) * 255 + 0.5)` (x87 under the game's precision, f32 store,
/// truncating `fistp`, low byte stored).
fn gamma_byte(xv: f32, e: f32) -> u8 {
    let p = crt_pow(f64::from(xv), f64::from(e));
    let v = X87::from_f64_exact(p)
        .fmul_m(f32::from_bits(K_255))
        .fadd_m(f32::from_bits(K_HALF));
    fistp_chop(f32_of(v)) as u8
}

/// `RwImageSetGamma(gamma)` (0x803FE0): stores `gamma` at `M+0x20C`, entry 0 of both tables = 0, then
/// for i = 1..255 with `x = (f32)(i * (1/255))`: gamma table[i] = byte of `pow(x, (f32)(1/gamma))`,
/// inverse table[i] = byte of `pow(x, gamma)` (see [`gamma_byte`]; `pow` is the CRT's, called with
/// both operands as doubles). Returns TRUE. No check of `gamma` (0 gives an infinite exponent).
#[hook(addr = 0x803FE0, conv = cdecl, name = "RwImageSetGamma", expect = "55 8B EC 83 E4 F8")]
pub fn rw_image_set_gamma(gamma: f32) -> i32 {
    wr::<f32>(imod(), 0x20C, f32_of(x(gamma)));
    let inv = f32_of(x(f32::from_bits(K_ONE)).fdiv_m(gamma));
    wr::<u8>(imod(), 0xC, 0);
    wr::<u8>(imod(), 0x10C, 0);
    let mut i: i32 = 1;
    while i < 0x100 {
        let xv = f32_of(X87::from_i32(i).fmul_m(f32::from_bits(K_INV255)));
        let b = gamma_byte(xv, inv);
        wr::<u8>(imod(), 0xC + i as isize, b);
        let b = gamma_byte(xv, gamma);
        wr::<u8>(imod(), 0x10C + i as isize, b);
        i += 1;
    }
    1
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_IMAGE_MODULE_OPEN, HOOK_IMAGE_MODULE_CLOSE, HOOK_RW_IMAGE_SET_GAMMA];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = IMAGE_GLOBALS;
