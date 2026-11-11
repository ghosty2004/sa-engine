//! PORT-RW-1 module `camera`: `RwCamera` and CAMERAMODULE (see docs/arch/rw-port.md). Ported from the 1.0 US
//! bytes; `docs/re/specs/RW/RwCamera.md` as a reading aid.
//!
//! `RwCamera` (0x184 base): +0 type 4, +4 frame, +8 lFrame, +0x10 sync callback, +0x14 projection,
//! +0x18 beginUpdate, +0x1C endUpdate, +0x20 viewMatrix (flags +0x2C), +0x60 frameBuffer, +0x64 zBuffer,
//! +0x68 viewWindow, +0x70 recipViewWindow, +0x78 viewOffset, +0x80 near, +0x84 far, +0x88 fog,
//! +0x8C zScale, +0x90 zShift, +0x94 frustum planes (6 x 0x14), +0x10C frustum bbox, +0x124 corners[8].
//! The frame LTM is at frame + 0x50 (right +0, up +0x10, at +0x20, pos +0x30).
//!
//! Not ported (left to the original through `orig!`): `_rwCameraBuildPerspClipPlanes` (0x7EE740) and the
//! parallel corner/plane builder (0x7EEDE0), see the hand-back notes.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{error2, f32_of, p8, rd, rw_globals, rw_memory_alloc, rw_memory_free, wr, x};

/// `[0x858624]` 1.0f, `[0x858FC4]` 1e-4f, `[0x858B8C]` 0.5f, `[0x858F40]` -0.5f.
const ONE: f32 = f32::from_bits(0x3F80_0000);
const Z_INSET: f32 = f32::from_bits(0x38D1_B717);
const HALF: f32 = f32::from_bits(0x3F00_0000);
const MINUS_HALF: f32 = f32::from_bits(0xBF00_0000);
/// "Invalid projection type" message passed to `_rwerror` (0x8E224C).
const INVALID_PROJECTION_MSG: u32 = 0x8E224C;
/// Default callbacks stored by `RwCameraCreate`.
const SYNC_CB: u32 = 0x7EE5A0;
const BEGIN_UPDATE_CB: u32 = 0x7EF370;
const END_UPDATE_CB: u32 = 0x7EF340;

sa_core::globals! {
    list CAMERA_GLOBALS;
    /// The camera freelist's in-place `RwFreeList`
    pub static CAMERA_FREELIST_SPACE @ 0xC9793C: [u32; 9] = [0; 9];
    /// CAMERAMODULE offset in `RwGlobals`
    pub static CAMERA_MODULE_OFFSET @ 0xC97960: u32 = 0;
    /// CAMERAMODULE open-instance counter
    pub static CAMERA_OPEN_COUNT @ 0xC97964: i32 = 0;
    /// Camera `RwPluginRegistry`
    pub static CAMERA_REGISTRY @ 0x8E222C: [u32; 6] = [0x184, 0x184, 0, 0, 0, 0];
    /// Camera freelist entries per block, blocks preallocated
    pub static CAMERA_FREELIST_PARAMS @ 0x8E2244: [i32; 2] = [4, 1];
}

orig! {
    /// `RwFreeListCreateAndPreallocateSpace` (0x801B70)
    pub fn free_list_create_and_preallocate_space(size: i32, per_block: i32, align: i32, blocks: i32, space: GPtr<u8>, hint: u32) -> GPtr<u8> = cdecl 0x801B70 => unported;
    /// `RwMatrixOptimize(m, tolerance)` (0x7F17E0)
    pub fn matrix_optimize(m: GPtr<u8>, tol: GPtr<f32>) -> GPtr<u8> = cdecl 0x7F17E0 => crate::matrix::rw_matrix_optimize;
    /// `RwBBoxCalculate(bbox, verts, n)` (0x808F60)
    pub fn bbox_calculate(bx: GPtr<u8>, verts: GPtr<u8>, n: i32) -> GPtr<u8> = cdecl 0x808F60 => crate::matrix::rw_bbox_calculate;
    /// `_rwCameraBuildPerspClipPlanes(camera)` (0x7EE740, not ported)
    pub fn build_persp_clip_planes(c: GPtr<u8>) = cdecl 0x7EE740 => unported;
    /// Parallel corners and planes (0x7EEDE0, not ported)
    pub fn build_parallel_clip_planes(c: GPtr<u8>) = cdecl 0x7EEDE0 => unported;
    /// `G->stdFunc[..](out, in, n)` (begin update +0x4C, end update +0x70)
    pub fn call_std_func(f: u32, out: u32, inp: GPtr<u8>, n: i32) -> i32 = cdecl dyn => unported;
    /// `_rwFrameSyncDirty()` (0x809550)
    pub fn frame_sync_dirty() -> u32 = cdecl 0x809550 => crate::frame::sync_dirty;
    /// `RwFrameUpdateObjects` (0x7F0910)
    pub fn frame_update_objects(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => crate::frame::update_objects;
    /// `_rwObjectHasFrameReleaseFrame` (0x804F40)
    pub fn object_release_frame(obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x804F40 => crate::frame::object_has_frame_release_frame;
    /// `_rwPluginRegistryInitObject` (0x8086E0)
    pub fn registry_init_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x8086E0 => crate::plugin::registry_init_object;
    /// `_rwPluginRegistryDeInitObject` (0x808740)
    pub fn registry_deinit_object(reg: GPtr<u8>, obj: GPtr<u8>) -> GPtr<u8> = cdecl 0x808740 => crate::plugin::registry_deinit_object;
    /// `_rwPluginRegistryAddPlugin` (0x8084A0)
    pub fn registry_add_plugin(reg: GPtr<u8>, size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 = cdecl 0x8084A0 => crate::plugin::registry_add_plugin;
    /// `_rwPluginRegistryGetPluginOffset` (0x808470)
    pub fn registry_get_plugin_offset(reg: GPtr<u8>, id: u32) -> i32 = cdecl 0x808470 => crate::plugin::registry_get_plugin_offset;

    /// `_rwCameraUpdateZShiftScale` (0x7EE200)
    pub fn update_z_shift_scale_call(c: GPtr<u8>) = cdecl 0x7EE200 => update_z_shift_scale;
    /// Parallel view matrix (0x7EF1F0)
    pub fn build_parallel_view_matrix_call(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EF1F0 => build_parallel_view_matrix;
    /// `_rwCameraBeginUpdateHook` (0x807C80)
    pub fn begin_update_hook_call(c: GPtr<u8>) = cdecl 0x807C80 => begin_update_hook;
}

fn a32(p: GPtr<u8>) -> u32 {
    p.addr() as u32
}

/// The camera registry (`0x8E222C`).
pub fn camera_registry() -> GPtr<u8> {
    CAMERA_REGISTRY.ptr().cast()
}

/// The camera freelist: `[G + [0xC97960]]`.
fn camera_freelist() -> GPtr<u8> {
    p8(rd::<u32>(rw_globals(), CAMERA_MODULE_OFFSET.get() as i32 as isize))
}

fn update_frame(c: GPtr<u8>) {
    let f = p8(rd::<u32>(c, 4));
    if !f.is_null() {
        frame_update_objects(f);
    }
}

/// `cb_CAMERAMODULE_RwEngine_Open` (0x7EE110).
#[hook(addr = 0x7EE110, conv = cdecl, name = "cb_CAMERAMODULE_RwEngine_Open", expect = "8B 44 24 08 8B 0D 48 22 8E 00")]
pub fn module_open(instance: GPtr<u8>, offset: i32, _size: i32) -> GPtr<u8> {
    let p = CAMERA_FREELIST_PARAMS.get();
    CAMERA_MODULE_OFFSET.set(offset as u32);
    let fl = free_list_create_and_preallocate_space(
        rd::<i32>(camera_registry(), 0),
        p[0],
        4,
        p[1],
        CAMERA_FREELIST_SPACE.ptr().cast(),
        0x40005,
    );
    wr::<u32>(rw_globals(), CAMERA_MODULE_OFFSET.get() as i32 as isize, a32(fl));
    if camera_freelist().is_null() {
        return GPtr::NULL;
    }
    CAMERA_OPEN_COUNT.set(CAMERA_OPEN_COUNT.get().wrapping_add(1));
    instance
}

/// `RwCameraSetViewOffset` (0x7EE1A0): dword copies, frame update.
#[hook(addr = 0x7EE1A0, conv = cdecl, name = "RwCameraSetViewOffset", expect = "8B 44 24 08 56")]
pub fn set_view_offset(c: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(c, 0x78, rd::<u32>(v, 0));
    wr::<u32>(c, 0x7C, rd::<u32>(v, 4));
    update_frame(c);
    c
}

/// `RwCameraSetNearClipPlane` (0x7EE1D0): the value goes through `fld`/`fstp` (a signalling NaN is
/// quieted); z shift/scale first, then the frame update.
#[hook(addr = 0x7EE1D0, conv = cdecl, name = "RwCameraSetNearClipPlane", expect = "D9 44 24 08 56")]
pub fn set_near_clip_plane(c: GPtr<u8>, near: u32) -> GPtr<u8> {
    wr::<f32>(c, 0x80, f32_of(x(f32::from_bits(near))));
    update_z_shift_scale_call(c);
    update_frame(c);
    c
}

/// `_rwCameraUpdateZShiftScale` (0x7EE200), in the original x87 order: the near end of the inset z range
/// stays in a register, the far end is stored; the scale is used unrounded for the shift.
/// Not hookable: `RwCameraCreate` keeps EDX (= 0) across the call (0x7EE58C).
pub fn update_z_shift_scale(c: GPtr<u8>) {
    let g = rw_globals();
    let zn = f32_of(x(rd::<f32>(g, 0x18)));
    let zf = f32_of(x(rd::<f32>(g, 0x1C)));
    let far = x(rd::<f32>(c, 0x84));
    let (n, f) = if rd::<i32>(c, 0x14) != 2 {
        let f = far.fdivr_m(ONE);
        (x(rd::<f32>(c, 0x80)).fdivr_m(ONE), f)
    } else {
        (x(rd::<f32>(c, 0x80)), far)
    };
    let d = x(zf).fsub_m(zn).fmul_m(Z_INSET);
    let zf2 = f32_of(x(zf).fsub(d));
    let zn2 = d.fadd_m(zn);
    let scale = x(zf2).fsub(zn2).fdiv(f.fsub(n));
    wr::<f32>(c, 0x8C, f32_of(scale));
    let t = x(zf2).fadd(zn2);
    let u = n.fadd(f).fmul(scale);
    wr::<f32>(c, 0x90, f32_of(t.fsub(u).fmul_m(HALF)));
}

/// `RwCameraSetFarClipPlane` (0x7EE2A0).
#[hook(addr = 0x7EE2A0, conv = cdecl, name = "RwCameraSetFarClipPlane", expect = "D9 44 24 08 56")]
pub fn set_far_clip_plane(c: GPtr<u8>, far: u32) -> GPtr<u8> {
    wr::<f32>(c, 0x84, f32_of(x(f32::from_bits(far))));
    update_z_shift_scale_call(c);
    update_frame(c);
    c
}

/// `RwCameraFrustumTestSphere` (0x7EE2D0): 0 outside (distance > radius for a plane), 1 on a boundary
/// (distance > -radius), else 2. Unordered compares count as inside.
#[hook(addr = 0x7EE2D0, conv = cdecl, name = "RwCameraFrustumTestSphere", expect = "8B 44 24 04 8B 54 24 08")]
pub fn frustum_test_sphere(c: GPtr<u8>, s: GPtr<u8>) -> i32 {
    let mut res = 2;
    for k in 0..6 {
        let p = c.byte_add(0x94 + k * 0x14);
        let d = x(rd::<f32>(p, 4))
            .fmul_m(rd::<f32>(s, 4))
            .fadd(x(rd::<f32>(p, 0)).fmul_m(rd::<f32>(s, 0)))
            .fadd(x(rd::<f32>(p, 8)).fmul_m(rd::<f32>(s, 8)))
            .fsub_m(rd::<f32>(p, 0xC));
        let r = rd::<f32>(s, 0xC);
        let cmp = d.fcom(x(r));
        if !cmp.c0() && !cmp.c3() {
            return 0;
        }
        let cmp = d.fcom(x(r).fchs());
        if !cmp.c0() && !cmp.c3() {
            res = 1;
        }
    }
    res
}

/// `RwCameraSetProjection` (0x7EE3A0): only 1 and 2 accepted (else E_RW_INVALIDPARAM-style error
/// 0x80000003 with the message, NULL); frame update first, then z shift/scale.
#[hook(addr = 0x7EE3A0, conv = cdecl, name = "RwCameraSetProjection", expect = "8B 44 24 08 83 EC 08")]
pub fn set_projection(c: GPtr<u8>, p: i32) -> GPtr<u8> {
    if p <= 0 || p > 2 {
        error2(0x8000_0003, INVALID_PROJECTION_MSG);
        return GPtr::NULL;
    }
    wr::<i32>(c, 0x14, p);
    update_frame(c);
    update_z_shift_scale_call(c);
    c
}

/// `RwCameraSetViewWindow` (0x7EE410): reciprocals of the stored values (no zero check).
#[hook(addr = 0x7EE410, conv = cdecl, name = "RwCameraSetViewWindow", expect = "8B 44 24 08 56")]
pub fn set_view_window(c: GPtr<u8>, vw: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(c, 0x68, rd::<u32>(vw, 0));
    wr::<u32>(c, 0x6C, rd::<u32>(vw, 4));
    wr::<f32>(c, 0x70, f32_of(x(ONE).fdiv_m(rd::<f32>(c, 0x68))));
    wr::<f32>(c, 0x74, f32_of(x(ONE).fdiv_m(rd::<f32>(c, 0x6C))));
    update_frame(c);
    c
}

/// `RwCameraRegisterPlugin` (0x7EE450).
#[hook(addr = 0x7EE450, conv = cdecl, name = "RwCameraRegisterPlugin", expect = "8B 44 24 14 8B 4C 24 10")]
pub fn register_plugin(size: i32, id: u32, ctor: u32, dtor: u32, copy: u32) -> i32 {
    registry_add_plugin(camera_registry(), size, id, ctor, dtor, copy)
}

/// `RwCameraGetPluginOffset` (0x7EE480).
#[hook(addr = 0x7EE480, conv = cdecl, name = "RwCameraGetPluginOffset", expect = "8B 44 24 04 50")]
pub fn get_plugin_offset(id: u32) -> i32 {
    registry_get_plugin_offset(camera_registry(), id)
}

/// `RwCameraDestroy` (0x7EE4B0): plugin destructors, release the frame, free. Returns 1.
#[hook(addr = 0x7EE4B0, conv = cdecl, name = "RwCameraDestroy", expect = "56 8B 74 24 08")]
pub fn destroy(c: GPtr<u8>) -> u32 {
    registry_deinit_object(camera_registry(), c);
    object_release_frame(c);
    rw_memory_free(camera_freelist(), c);
    1
}

/// `RwCameraCreate` (0x7EE4F0): defaults (near 0.05, far 10, fog 5, perspective); lFrame, the view
/// matrix body, planes, bbox and corners stay as allocated.
#[hook(addr = 0x7EE4F0, conv = cdecl, name = "RwCameraCreate", expect = "A1 24 7B C9 00")]
pub fn create() -> GPtr<u8> {
    let c = rw_memory_alloc(camera_freelist(), 0x30005);
    if c.is_null() {
        return c;
    }
    wr::<u8>(c, 0, 4);
    wr::<u8>(c, 1, 0);
    wr::<u8>(c, 2, 0);
    wr::<u8>(c, 3, 0);
    wr::<u32>(c, 4, 0);
    wr::<u32>(c, 0x10, SYNC_CB);
    wr::<u32>(c, 0x18, BEGIN_UPDATE_CB);
    wr::<u32>(c, 0x1C, END_UPDATE_CB);
    for off in [0x6C, 0x68, 0x74, 0x70] {
        wr::<u32>(c, off, 0x3F80_0000);
    }
    wr::<u32>(c, 0x7C, 0);
    wr::<u32>(c, 0x78, 0);
    wr::<u32>(c, 0x80, 0x3D4C_CCCD);
    wr::<u32>(c, 0x84, 0x4120_0000);
    wr::<u32>(c, 0x88, 0x40A0_0000);
    wr::<u32>(c, 0x60, 0);
    wr::<u32>(c, 0x64, 0);
    wr::<i32>(c, 0x14, 1);
    update_z_shift_scale_call(c);
    wr::<u32>(c, 0x2C, 0);
    registry_init_object(camera_registry(), c);
    c
}

/// One row of the view matrix (`x`: column 0 of right/up/at/pos, `y`: column 1). `s` =
/// recipViewWindow times -0.5; `row` = the LTM row (right or up); `o` = the stored offset term. `t0` stays in a register,
/// `t1`/`t2` are stored f32; X/Y/Z are register values; pos = 0.5 - (sum + o).
fn view_column(v: GPtr<u8>, col: isize, l: GPtr<u8>, row: isize, s: X87, o: f32, pos_pxx_first: bool) {
    let t0 = s.fmul_m(rd::<f32>(l, row));
    let t1 = f32_of(x(rd::<f32>(l, row + 4)).fmul(s));
    let t2 = f32_of(x(rd::<f32>(l, row + 8)).fmul(s));
    let cx = x(rd::<f32>(l, 0x20)).fmul_m(o).fadd(t0);
    let cy = x(rd::<f32>(l, 0x24)).fmul_m(o).fadd_m(t1);
    let cz = x(rd::<f32>(l, 0x28)).fmul_m(o).fadd_m(t2);
    wr::<f32>(v, col, f32_of(cx));
    wr::<f32>(v, 0x10 + col, f32_of(cy));
    wr::<f32>(v, 0x20 + col, f32_of(cz));
    let py = x(rd::<f32>(l, 0x34)).fmul(cy);
    let px = x(rd::<f32>(l, 0x30)).fmul(cx);
    let sum = if pos_pxx_first { px.fadd(py) } else { py.fadd(px) };
    let sum = sum.fadd(x(rd::<f32>(l, 0x38)).fmul(cz)).fadd_m(o);
    wr::<f32>(v, 0x30 + col, f32_of(sum.fsubr_m(HALF)));
}

/// Column 2 (dword copies of L.at) and `pos.z = -((..) + p.z*a.z)`.
fn view_column_z(v: GPtr<u8>, l: GPtr<u8>, persp: bool) {
    wr::<u32>(v, 8, rd::<u32>(l, 0x20));
    wr::<u32>(v, 0x18, rd::<u32>(l, 0x24));
    wr::<u32>(v, 0x28, rd::<u32>(l, 0x28));
    let s = if persp {
        x(rd::<f32>(l, 0x24))
            .fmul_m(rd::<f32>(l, 0x34))
            .fadd(x(rd::<f32>(l, 0x30)).fmul_m(rd::<f32>(l, 0x20)))
    } else {
        x(rd::<f32>(l, 0x30))
            .fmul_m(rd::<f32>(l, 0x20))
            .fadd(x(rd::<f32>(l, 0x24)).fmul_m(rd::<f32>(l, 0x34)))
    };
    let s = s.fadd(x(rd::<f32>(l, 0x38)).fmul_m(rd::<f32>(l, 0x28)));
    wr::<f32>(v, 0x38, f32_of(s.fchs()));
}

/// `_rwCameraFrameSyncCB` (0x7EE5A0): perspective view matrix inline (x: `o = 0.5 - vo.x*s`, y:
/// `o' = vo.y*s' + 0.5`), `RwMatrixOptimize`, the perspective planes; parallel: 0x7EF1F0 + 0x7EEDE0. Then
/// the frustum bbox from the 8 corners. Requires a frame.
#[hook(addr = 0x7EE5A0, conv = cdecl, name = "_rwCameraFrameSyncCB", expect = "83 EC 0C 56 8B 74 24 14")]
pub fn frame_sync_cb(c: GPtr<u8>) -> GPtr<u8> {
    if rd::<i32>(c, 0x14) == 1 {
        let l = p8(rd::<u32>(c, 4)).byte_add(0x50);
        let v = c.byte_add(0x20);
        let s = x(rd::<f32>(c, 0x70)).fmul_m(MINUS_HALF);
        let o = f32_of(x(rd::<f32>(c, 0x78)).fmul(s).fsubr_m(HALF));
        view_column(v, 0, l, 0, s, o, false);
        let s = x(rd::<f32>(c, 0x74)).fmul_m(MINUS_HALF);
        let o = f32_of(x(rd::<f32>(c, 0x7C)).fmul(s).fadd_m(HALF));
        view_column(v, 4, l, 0x10, s, o, false);
        view_column_z(v, l, true);
        matrix_optimize(v, GPtr::NULL);
        build_persp_clip_planes(c);
    } else {
        build_parallel_view_matrix_call(c);
        build_parallel_clip_planes(c);
    }
    bbox_calculate(c.byte_add(0x10C), c.byte_add(0x124), 8);
    c
}

/// Parallel view matrix (0x7EF1F0, catalogue `FUN_007ef1f0`): as the perspective block with
/// `o = -(vo.x*s)`, `o' = vo.y*s'`; then `RwMatrixOptimize`. Returns camera.
#[hook(addr = 0x7EF1F0, conv = cdecl, name = "FUN_007ef1f0", expect = "83 EC 0C 56 8B 74 24 14")]
pub fn build_parallel_view_matrix(c: GPtr<u8>) -> GPtr<u8> {
    let l = p8(rd::<u32>(c, 4)).byte_add(0x50);
    let v = c.byte_add(0x20);
    let s = x(rd::<f32>(c, 0x70)).fmul_m(MINUS_HALF);
    let o = f32_of(x(rd::<f32>(c, 0x78)).fmul(s).fchs());
    view_column(v, 0, l, 0, s, o, true);
    let s = x(rd::<f32>(c, 0x74)).fmul_m(MINUS_HALF);
    let o = f32_of(x(rd::<f32>(c, 0x7C)).fmul(s));
    view_column(v, 4, l, 0x10, s, o, true);
    view_column_z(v, l, false);
    matrix_optimize(v, GPtr::NULL);
    c
}

/// `_rwCameraDefaultEndUpdate` (0x7EF340): `stdFunc[10](NULL, c, 0)`; on success curCamera = NULL.
#[hook(addr = 0x7EF340, conv = cdecl, name = "_rwCameraDefaultEndUpdate", expect = "A1 24 7B C9 00")]
pub fn default_end_update(c: GPtr<u8>) -> GPtr<u8> {
    if call_std_func(rd::<u32>(rw_globals(), 0x70), 0, c, 0) == 0 {
        return GPtr::NULL;
    }
    wr::<u32>(rw_globals(), 0, 0);
    c
}

/// `_rwCameraDefaultBeginUpdate` (0x7EF370): curCamera = c first, sync the dirty frames,
/// `stdFunc[1](NULL, c, 0)`, then the (empty) begin-update hook.
#[hook(addr = 0x7EF370, conv = cdecl, name = "_rwCameraDefaultBeginUpdate", expect = "A1 24 7B C9 00")]
pub fn default_begin_update(c: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(rw_globals(), 0, a32(c));
    frame_sync_dirty();
    if call_std_func(rd::<u32>(rw_globals(), 0x4C), 0, c, 0) == 0 {
        return GPtr::NULL;
    }
    begin_update_hook_call(c);
    c
}

/// `_rwCameraBeginUpdateHook` (0x807C80): a single `ret`. Not hooked: the body is one byte followed by
/// padding.
pub fn begin_update_hook(_c: GPtr<u8>) {}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_MODULE_OPEN,
    HOOK_SET_VIEW_OFFSET,
    HOOK_SET_NEAR_CLIP_PLANE,
    HOOK_SET_FAR_CLIP_PLANE,
    HOOK_FRUSTUM_TEST_SPHERE,
    HOOK_SET_PROJECTION,
    HOOK_SET_VIEW_WINDOW,
    HOOK_REGISTER_PLUGIN,
    HOOK_GET_PLUGIN_OFFSET,
    HOOK_DESTROY,
    HOOK_CREATE,
    HOOK_FRAME_SYNC_CB,
    HOOK_BUILD_PARALLEL_VIEW_MATRIX,
    HOOK_DEFAULT_END_UPDATE,
    HOOK_DEFAULT_BEGIN_UPDATE,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = CAMERA_GLOBALS;
