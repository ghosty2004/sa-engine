//! `CameraSize` (0x72FC70, app_camera.cpp): resizes the main camera's rasters and view window, ported
//! from the 1.0 US bytes (gta-reversed `app/app_camera.cpp` as reading aid only: its version keeps a
//! static `origSize`, a static default rect, a `w > 0 && h > 0` test and a different exclusive-mode
//! order, none of which are in the 1.0 US bytes).

use crate::abi::HookReg;
use crate::mem::{GPtr, GameData, GlobalDesc};
use crate::platform::{RS_MAXIMUM_HEIGHT, RS_MAXIMUM_WIDTH};
use crate::{hook, orig};
use sa_math::x87::X87;

/// `RwVideoMode` (rwcore.h): width, height, depth, flags, refRate, format.
pub type RwVideoMode = [i32; 6];

/// `rwVIDEOMODEEXCLUSIVE`
const VIDEO_MODE_EXCLUSIVE: i32 = 1;

/// `RwCamera::frameBuffer` / `RwCamera::zBuffer`.
const CAMERA_FRAME_BUFFER: isize = 0x60;
const CAMERA_Z_BUFFER: isize = 0x64;
/// `RwRaster::width` / `RwRaster::height`.
const RASTER_WIDTH: isize = 0x0C;
const RASTER_HEIGHT: isize = 0x10;
/// `RwRect::w` / `RwRect::h` (x +0, y +4).
const RECT_W: isize = 8;
const RECT_H: isize = 0xC;

fn at<T: GameData>(p: GPtr<u8>, off: isize) -> GPtr<T> {
    p.byte_add(off).cast()
}

/// Calls into other functions (original or Rust, decided by their hooks).
pub mod call {
    use super::*;
    orig! {
        /// `RwEngineGetCurrentVideoMode` (0x7F2D20; ported in sa-rwcore, which depends on sa-core)
        pub fn rw_engine_get_current_video_mode() -> i32 = cdecl 0x7F2D20 => unported;
        /// `RwEngineGetVideoModeInfo(RwVideoMode*, RwInt32)` (0x7F2CF0; ported in sa-rwcore)
        pub fn rw_engine_get_video_mode_info(info: GPtr<RwVideoMode>, mode: i32) -> GPtr<RwVideoMode> = cdecl 0x7F2CF0 => unported;
        /// `RwCameraSetViewWindow(RwCamera*, const RwV2d*)` (0x7EE410; ported in sa-rwcore)
        pub fn rw_camera_set_view_window(camera: GPtr<u8>, view_window: GPtr<[f32; 2]>) -> GPtr<u8> = cdecl 0x7EE410 => unported;
    }
}

/// `CameraSize(RwCamera*, RwRect*, RwReal viewWindow, RwReal aspectRatio)` (0x72FC70, cdecl, void).
///
/// Per the bytes: a null camera returns at once. Otherwise the current video mode's info is fetched
/// into a stack `RwVideoMode` (`RwEngineGetVideoModeInfo(&vmi, RwEngineGetCurrentVideoMode())`, the
/// result ignored). The frame buffer raster's width/height are read. A null `rect` is replaced by a
/// stack rect {0, 0, w, h} where w/h are the video mode's size when `vmi.flags` has
/// rwVIDEOMODEEXCLUSIVE, else the raster's (stores in the order w, h, y, x). A non-null rect is used
/// as is, even in exclusive mode. If `rect.w/h` differ from the frame buffer's size, they are written
/// to the z buffer and the frame buffer (zw, fw, zh, fh; the z buffer pointer is read only then; no
/// positivity test). View window: exclusive → `{viewWindow, viewWindow / aspectRatio}` (`fld; fdiv
/// m32`, x is a bit copy); else, with `w = rect.w`, `h = rect.h` re-read, `w > h` (signed) →
/// `{viewWindow, h * viewWindow / w}`, otherwise `{w * viewWindow / h, viewWindow}` (`fild; fmul m32;
/// fidiv`, PC=24; `aspectRatio` unused). Then `RwCameraSetViewWindow(camera, &vw)` and
/// `RsGlobal.maximumWidth/Height = rect.w/h` (re-read).
///
/// Not modelled: the original stores `rect.w` into the camera argument's stack slot in the
/// non-exclusive path (dead, caller-owned slot), and an `RwVideoMode` that the request failed to fill
/// is stack garbage there (zeroed here).
///
/// Written by STD-2's helper for the native boot, not yet verified against the original (phase A).
#[hook(addr = 0x72FC70, conv = cdecl, name = "CameraSize", expect = "83 EC 34 57 8B 7C 24 3C")]
pub fn camera_size(camera: GPtr<u8>, rect: GPtr<u8>, view_window: f32, aspect: f32) {
    if camera.is_null() {
        return;
    }
    let mut vmi: RwVideoMode = [0; 6];
    let mode = call::rw_engine_get_current_video_mode();
    call::rw_engine_get_video_mode_info(GPtr::from_raw(&raw mut vmi), mode);
    let fb = at::<u32>(camera, CAMERA_FRAME_BUFFER).read();
    let fb = GPtr::<u8>::from_addr(fb as usize);
    let fb_w = at::<i32>(fb, RASTER_WIDTH).read();
    let fb_h = at::<i32>(fb, RASTER_HEIGHT).read();
    let exclusive = vmi[3] & VIDEO_MODE_EXCLUSIVE != 0;

    let mut local: [i32; 4] = [0; 4];
    let rect = if rect.is_null() {
        let (w, h) = if exclusive { (vmi[0], vmi[1]) } else { (fb_w, fb_h) };
        local[2] = w;
        local[3] = h;
        local[1] = 0;
        local[0] = 0;
        GPtr::<u8>::from_raw((&raw mut local).cast())
    } else {
        rect
    };

    let w = at::<i32>(rect, RECT_W).read();
    if fb_w != w || fb_h != at::<i32>(rect, RECT_H).read() {
        let zb = at::<u32>(camera, CAMERA_Z_BUFFER).read();
        let zb = GPtr::<u8>::from_addr(zb as usize);
        at::<i32>(zb, RASTER_WIDTH).write(w);
        at::<i32>(fb, RASTER_WIDTH).write(w);
        let h = at::<i32>(rect, RECT_H).read();
        at::<i32>(zb, RASTER_HEIGHT).write(h);
        at::<i32>(fb, RASTER_HEIGHT).write(h);
    }

    let vw_bits = view_window.to_bits();
    let mut vw: [f32; 2] = [0.0; 2];
    if exclusive {
        vw[0] = f32::from_bits(vw_bits);
        vw[1] = X87::from_f32(view_window).fdiv_m(aspect).to_f32();
    } else {
        let w = at::<i32>(rect, RECT_W).read();
        let h = at::<i32>(rect, RECT_H).read();
        if w > h {
            vw[0] = f32::from_bits(vw_bits);
            vw[1] = X87::from_i32(h).fmul_m(view_window).fdiv(X87::from_i32(w)).to_f32();
        } else {
            vw[1] = f32::from_bits(vw_bits);
            vw[0] = X87::from_i32(w).fmul_m(view_window).fdiv(X87::from_i32(h)).to_f32();
        }
    }
    call::rw_camera_set_view_window(camera, GPtr::from_raw(&raw mut vw));
    RS_MAXIMUM_WIDTH.set(at::<i32>(rect, RECT_W).read());
    RS_MAXIMUM_HEIGHT.set(at::<i32>(rect, RECT_H).read());
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_CAMERA_SIZE];

/// Globals owned by this module (none: `RsGlobal` is declared in [`crate::platform`]).
pub const GLOBALS: &[GlobalDesc] = &[];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orig::host;

    static mut MODE: RwVideoMode = [0; 6];
    static mut LAST_VW: [f32; 2] = [0.0; 2];

    fn cur_mode() -> i32 {
        7
    }
    fn mode_info(info: GPtr<RwVideoMode>, mode: i32) -> GPtr<RwVideoMode> {
        assert_eq!(mode, 7);
        // SAFETY: single-threaded test data.
        #[allow(unsafe_code)]
        info.write(unsafe { MODE });
        info
    }
    fn set_vw(camera: GPtr<u8>, vw: GPtr<[f32; 2]>) -> GPtr<u8> {
        // SAFETY: single-threaded test data.
        #[allow(unsafe_code)]
        unsafe {
            LAST_VW = vw.read()
        };
        camera
    }

    /// Two zeroed `RwRaster` stand-ins (8 dwords each) at addresses that fit in 32 bits.
    #[cfg(target_pointer_width = "32")]
    fn low_rasters() -> (&'static mut [i32; 8], &'static mut [i32; 8]) {
        static mut RASTERS: [[i32; 8]; 2] = [[0; 8]; 2];
        // SAFETY: used by this single test only.
        #[allow(unsafe_code)]
        unsafe {
            let r = &mut *(&raw mut RASTERS);
            *r = [[0; 8]; 2];
            let (a, b) = r.split_at_mut(1);
            (&mut a[0], &mut b[0])
        }
    }

    /// Two zeroed `RwRaster` stand-ins (8 dwords each) at addresses that fit in 32 bits: one page
    /// mapped with `MAP_32BIT` (Linux x86-64; never unmapped, the test process is short-lived).
    #[cfg(all(target_pointer_width = "64", target_os = "linux", target_arch = "x86_64"))]
    fn low_rasters() -> (&'static mut [i32; 8], &'static mut [i32; 8]) {
        const PROT_READ_WRITE: i32 = 0x1 | 0x2;
        const MAP_PRIVATE_ANONYMOUS_32BIT: i32 = 0x02 | 0x20 | 0x40;
        #[allow(unsafe_code)]
        unsafe extern "C" {
            fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, off: i64) -> *mut u8;
        }
        // SAFETY: a fresh private anonymous page (zero-filled), exclusively owned by this test.
        #[allow(unsafe_code)]
        unsafe {
            let p = mmap(core::ptr::null_mut(), 0x1000, PROT_READ_WRITE, MAP_PRIVATE_ANONYMOUS_32BIT, -1, 0);
            assert!(!p.is_null() && p as isize != -1, "mmap(MAP_32BIT) failed");
            (&mut *p.cast::<[i32; 8]>(), &mut *p.add(0x40).cast::<[i32; 8]>())
        }
    }

    #[test]
    #[cfg_attr(
        not(any(target_pointer_width = "32", all(target_os = "linux", target_arch = "x86_64"))),
        ignore = "needs memory below 4 GiB for the camera's 32-bit raster pointers"
    )]
    fn sizes_rasters_and_view_window() {
        host::set::<fn() -> i32>("sa_core::camerasize::call::rw_engine_get_current_video_mode", cur_mode);
        host::set::<fn(GPtr<RwVideoMode>, i32) -> GPtr<RwVideoMode>>(
            "sa_core::camerasize::call::rw_engine_get_video_mode_info",
            mode_info,
        );
        host::set::<fn(GPtr<u8>, GPtr<[f32; 2]>) -> GPtr<u8>>(
            "sa_core::camerasize::call::rw_camera_set_view_window",
            set_vw,
        );
        // The camera holds 32-bit raster pointers (the 1.0 US layout), so the rasters must sit below
        // 4 GiB: on a 64-bit host a stack address would be truncated (and the read would fault).
        let (fb, zb) = low_rasters();
        fb[3] = 640;
        fb[4] = 480;
        let mut cam = [0u32; 0x40];
        cam[0x60 / 4] = u32::try_from(core::ptr::from_mut(fb) as usize).unwrap();
        cam[0x64 / 4] = u32::try_from(core::ptr::from_mut(zb) as usize).unwrap();
        let camp = GPtr::<u8>::from_raw((&raw mut cam).cast());

        // windowed, explicit rect 800x600
        let mut rect = [0i32, 0, 800, 600];
        camera_size(camp, GPtr::from_raw((&raw mut rect).cast()), 0.7, 1.0);
        assert_eq!((fb[3], fb[4], zb[3], zb[4]), (800, 600, 800, 600));
        #[allow(unsafe_code)]
        let vw = unsafe { LAST_VW };
        assert_eq!(vw[0], 0.7);
        assert_eq!(vw[1], X87::from_i32(600).fmul_m(0.7).fdiv(X87::from_i32(800)).to_f32());
        assert_eq!((RS_MAXIMUM_WIDTH.get(), RS_MAXIMUM_HEIGHT.get()), (800, 600));

        // exclusive, null rect: the video mode's size; vw.y = vw / aspect
        #[allow(unsafe_code)]
        unsafe {
            MODE = [1024, 768, 32, 1, 60, 0]
        };
        camera_size(camp, GPtr::NULL, 0.7, 4.0 / 3.0);
        assert_eq!((fb[3], fb[4], zb[3], zb[4]), (1024, 768, 1024, 768));
        #[allow(unsafe_code)]
        let vw = unsafe { LAST_VW };
        assert_eq!(vw[0], 0.7);
        assert_eq!(vw[1], X87::from_f32(0.7).fdiv_m(4.0 / 3.0).to_f32());

        // null camera: nothing
        camera_size(GPtr::NULL, GPtr::NULL, 0.7, 1.0);
        for k in [
            "sa_core::camerasize::call::rw_engine_get_current_video_mode",
            "sa_core::camerasize::call::rw_engine_get_video_mode_info",
            "sa_core::camerasize::call::rw_camera_set_view_window",
        ] {
            host::clear(k);
        }
    }
}
