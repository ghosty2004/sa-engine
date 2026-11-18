//! FRONTIER-2 lane C (batch `frontier2-c21`): the screenshot path of the JPEG plugin, `JPegCompressScreen`
//! (0x5D0470) and `JPegCompressScreenToFile` (0x5D0820), reached by the gallery photo of `CPostEffects::Render`.
//! Ported from the 1.0 US bytes; gta-reversed `JPegCompress.cpp` as a reading aid.
//!
//! The IJG libjpeg 6b they call is statically linked at 0x5C26B0-0x5D0260 (JPEG_LIB_VERSION 62). Like the other
//! statically linked third-party libraries (d3dx9, libpng + zlib, QuickTime: `excluded:thirdparty:*` in
//! tools/catalog/classify_unknowns.py) it is not ported: the catalogue classes it `excluded:thirdparty:libjpeg-6b`
//! (docs/re/specs/RW/_unk-M.md), its jmemnobs back end 0x5CF800-0x5CF830 included. The ports call it through the declarations below; the `jpeg_compress_struct`
//! fields are written at their libjpeg 6b offsets. Without the exe, sa-game serves these entry points natively
//! (`libjpeg.rs`).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr};
use sa_core::bindings::{file_close, file_write, image_destroy, open_file};
use sa_core::field::ptr_of as lp;

orig! {
    /// `Rs::RsGrabScreen(RwCamera*)` (0x619AF0)
    fn rs_grab_screen(cam: GPtr<u8>) -> GPtr<u8> = cdecl 0x619AF0 => sa_core::platform::rs_grab_screen;
    /// libjpeg `jpeg_std_error(jpeg_error_mgr*)` (0x5CD810)
    fn jpeg_std_error(err: GPtr<u8>) -> GPtr<u8> = cdecl 0x5CD810 => unported;
    /// libjpeg `jpeg_CreateCompress(j_compress_ptr, int version, size_t struct size)` (0x5CF960)
    fn jpeg_create_compress(cinfo: GPtr<u8>, version: i32, size: u32) = cdecl 0x5CF960 => unported;
    /// libjpeg `jpeg_set_defaults(j_compress_ptr)` (0x5C7590)
    fn jpeg_set_defaults(cinfo: GPtr<u8>) = cdecl 0x5C7590 => unported;
    /// libjpeg `jpeg_start_compress(j_compress_ptr, boolean write_all_tables)` (0x5C26B0)
    fn jpeg_start_compress(cinfo: GPtr<u8>, all: u32) = cdecl 0x5C26B0 => unported;
    /// libjpeg `jpeg_write_scanlines(j_compress_ptr, JSAMPARRAY, JDIMENSION)` (0x5C2730)
    fn jpeg_write_scanlines(cinfo: GPtr<u8>, rows: GPtr<u8>, n: u32) -> u32 = cdecl 0x5C2730 => unported;
    /// libjpeg `jpeg_finish_compress(j_compress_ptr)` (0x5CFAC0)
    fn jpeg_finish_compress(cinfo: GPtr<u8>) = cdecl 0x5CFAC0 => unported;
    /// libjpeg `jpeg_destroy_compress(j_compress_ptr)` (0x5CFA30)
    fn jpeg_destroy_compress(cinfo: GPtr<u8>) = cdecl 0x5CFA30 => unported;
}

/// `JPegCompressScreen(RwCamera*, jpeg_destination_mgr&)` (0x5D0470, cdecl): `RsGrabScreen(camera)`; a
/// `jpeg_compress_struct` (0x168 bytes) with `jpeg_std_error`'s manager, `jpeg_CreateCompress(.., 62, 0x168)`,
/// `in_color_space` (+0x28) = JCS_RGB before `jpeg_set_defaults`; then width / height (+0x1C / +0x20) from the
/// image (+4 / +8), 4 input components (+0x24), +0xB8 = 2, `dest` (+0x18) = the manager;
/// `jpeg_start_compress(.., TRUE)`; one scanline at a time (row `i` = pixels +0x14 + `i * width * 4`) while
/// `next_scanline` (+0xD0) < height and each write takes one line; `jpeg_finish_compress`,
/// `jpeg_destroy_compress`, `RwImageDestroy(image)`.
#[hook(addr = 0x5D0470, conv = cdecl, name = "JPegPlugin::JPegCompressScreen", expect = "81 EC F0 01 00 00",
       unverified = "frontier2-c21", route = "L2", hazards = "heap-writes")]
pub fn jpeg_compress_screen(cam: GPtr<u8>, dst: GPtr<u8>) {
    let img = rs_grab_screen(cam);
    let mut err = [0u32; 0x84 / 4 + 32];
    let mut cinfo = [0u32; 0x168 / 4];
    let c = lp(&mut cinfo);
    let e = jpeg_std_error(lp(&mut err));
    wr::<u32>(c, 0, e.addr() as u32);
    jpeg_create_compress(c, 0x3E, 0x168);
    wr::<u32>(c, 0x28, 2);
    jpeg_set_defaults(c);
    wr::<u32>(c, 0x1C, rd::<u32>(img, 4));
    wr::<u32>(c, 0x20, rd::<u32>(img, 8));
    wr::<u32>(c, 0x24, 4);
    wr::<u32>(c, 0xB8, 2);
    wr::<u32>(c, 0x18, dst.addr() as u32);
    jpeg_start_compress(c, 1);
    let pixels = rd::<u32>(img, 0x14);
    let mut row = [pixels];
    let mut i = 0u32;
    while rd::<u32>(c, 0xD0) < rd::<u32>(c, 0x20) {
        row[0] = pixels.wrapping_add(rd::<u32>(img, 4).wrapping_mul(i).wrapping_mul(4));
        if jpeg_write_scanlines(c, lp(&mut row), 1) != 1 {
            break;
        }
        i = i.wrapping_add(1);
    }
    jpeg_finish_compress(c);
    jpeg_destroy_compress(c);
    image_destroy(img);
}

/// `JPegCompressScreenToFile(RwCamera*, const char* path)` (0x5D0820, cdecl): `CFileMgr::OpenFile(path, "wb")`
/// into `JPegPlugin`'s file handle (0xBD0B74); when it is above 0, `JPegCompressScreen` with a destination manager
/// on the stack whose callbacks are the plugin's init / empty / term destination (0x5D03E0 / 0x5D0400 /
/// 0x5D0440; the buffer fields are set by the init callback), then `CFileMgr::CloseFile`.
#[hook(addr = 0x5D0820, conv = cdecl, name = "JPegPlugin::JPegCompressScreenToFile", expect = "8B 44 24 08 83 EC 14",
       unverified = "frontier2-c21", route = "L2", hazards = "heap-writes",
       note = "the destination manager's two buffer fields start as stack residue until libjpeg calls init_destination")]
pub fn jpeg_compress_screen_to_file(cam: GPtr<u8>, path: GPtr<u8>) {
    let f = open_file(path, g(0x86_0ADC));
    g::<u32>(FILE).write(f.addr() as u32);
    if (f.addr() as u32 as i32) <= 0 {
        return;
    }
    let mut dst = [0u32, 0, 0x5D_03E0, 0x5D_0400, 0x5D_0440];
    jpeg_compress_screen(cam, lp(&mut dst));
    file_close(g::<GPtr<u8>>(FILE).read());
}

/// The file destination's `empty_output_buffer(j_compress_ptr)` (0x5D0400, cdecl): the whole buffer (0xBD0B78,
/// 0x32000 bytes) written with `CFileMgr::Write` to `JPegPlugin`'s file, the manager (`cinfo` +0x18) reset to the
/// buffer's start and size; returns TRUE (AL = 1).
#[hook(addr = 0x5D0400, conv = cdecl, name = "JPegPlugin::EmptyOutputBuffer", expect = "A1 74 0B BD 00 68 00 20 03 00",
       unverified = "pure1f-1", route = "L2", hazards = "heap-writes", note = "writes the JPEG file")]
pub fn jpeg_empty_output_buffer(cinfo: GPtr<u8>) -> u8 {
    file_write(g::<GPtr<u8>>(FILE).read(), GPtr::from_addr(BUFFER), BUFFER_SIZE);
    let dst = GPtr::<u8>::from_addr(rd::<u32>(cinfo, 0x18) as usize);
    wr::<u32>(dst, 0, BUFFER as u32);
    wr::<u32>(dst, 4, BUFFER_SIZE);
    1
}

/// `JPegPlugin`'s output file handle (0xBD0B74).
const FILE: usize = 0xBD_0B74;
/// The file destination's buffer (0xBD0B78) and its size.
const BUFFER: usize = 0xBD_0B78;
const BUFFER_SIZE: u32 = 0x3_2000;

pub const HOOKS: &[HookReg] =
    &[HOOK_JPEG_COMPRESS_SCREEN, HOOK_JPEG_COMPRESS_SCREEN_TO_FILE, HOOK_JPEG_EMPTY_OUTPUT_BUFFER];
