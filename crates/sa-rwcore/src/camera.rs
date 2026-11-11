//! PORT-RW-1 module `camera`: `RwCamera` and CAMERAMODULE (see docs/arch/rw-port.md). Ported from the 1.0 US
//! bytes; `docs/re/specs/RW/RwCamera.md` as a reading aid.
//!
//! `RwCamera` (0x184 base): +0 type 4, +4 frame, +8 lFrame, +0x10 sync callback, +0x14 projection,
//! +0x18 beginUpdate, +0x1C endUpdate, +0x20 viewMatrix (flags +0x2C), +0x60 frameBuffer, +0x64 zBuffer,
//! +0x68 viewWindow, +0x70 recipViewWindow, +0x78 viewOffset, +0x80 near, +0x84 far, +0x88 fog,
//! +0x8C zScale, +0x90 zShift, +0x94 frustum planes (6 x 0x14), +0x10C frustum bbox, +0x124 corners[8].
//! The frame LTM is at frame + 0x50 (right +0, up +0x10, at +0x20, pos +0x30).
//!
//! `_rwCameraBuildPerspClipPlanes` (0x7EE740) and the parallel builder (0x7EEDE0) are transcribed
//! instruction by instruction (x87 register stack modelled by [`Fpu`]), see their doc comments.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_core::light::RwV3d;
use sa_math::x87::X87;

use crate::matrix::inv_sqrt_call as inv_sqrt;
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
    /// `_rwCameraBuildPerspClipPlanes(camera)` (0x7EE740)
    pub fn build_persp_clip_planes(c: GPtr<u8>) = cdecl 0x7EE740 => rw_camera_build_persp_clip_planes;
    /// Parallel corners and planes (0x7EEDE0)
    pub fn build_parallel_clip_planes(c: GPtr<u8>) = cdecl 0x7EEDE0 => rw_camera_build_parallel_clip_planes;
    /// `RwV3dTransformPoints(out, in, n, matrix)` (0x7EDD90)
    fn transform_points(out: GPtr<RwV3d>, inp: GPtr<RwV3d>, n: i32, m: GPtr<u8>) -> GPtr<RwV3d> = cdecl 0x7EDD90 => crate::vector::rw_v3d_transform_points;
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

orig! {
    /// `RwCamera::beginUpdate` / `endUpdate(camera)` (stored pointers, +0x18 / +0x1C)
    fn call_camera_cb(f: u32, camera: GPtr<u8>) -> GPtr<u8> = cdecl dyn => unported;
}

/// `RwCameraEndUpdate(camera)` (0x7EE180): a tail jump through `camera->endUpdate` (+0x1C).
/// Written by STD-3, unverified.
#[hook(addr = 0x7EE180, conv = cdecl, name = "RwCore::RwCameraEndUpdate", expect = "8B 44 24 04 89 44 24 04")]
pub fn camera_end_update(camera: GPtr<u8>) -> GPtr<u8> {
    call_camera_cb(rd::<u32>(camera, 0x1C), camera)
}

/// `RwCameraBeginUpdate(camera)` (0x7EE190): a tail jump through `camera->beginUpdate` (+0x18).
/// Written by STD-3, unverified.
#[hook(addr = 0x7EE190, conv = cdecl, name = "RwCore::RwCameraBeginUpdate", expect = "8B 44 24 04 89 44 24 04")]
pub fn camera_begin_update(camera: GPtr<u8>) -> GPtr<u8> {
    call_camera_cb(rd::<u32>(camera, 0x18), camera)
}

/// The x87 register stack as the transcribed camera builders use it (values in the `X87` model of
/// the game's 24-bit precision control). `st(i)` is the i-th register from the top.
struct Fpu {
    r: [X87; 8],
    n: usize,
}

impl Fpu {
    fn new() -> Self {
        Fpu {
            r: [X87::zero(); 8],
            n: 0,
        }
    }
    fn push(&mut self, v: X87) {
        assert!(self.n < 8, "x87 stack overflow");
        self.r[self.n] = v;
        self.n += 1;
    }
    fn pop(&mut self) {
        self.n -= 1;
    }
    fn st(&self, i: usize) -> X87 {
        self.r[self.n - 1 - i]
    }
    fn set(&mut self, i: usize, v: X87) {
        let k = self.n - 1 - i;
        self.r[k] = v;
    }
    /// `fxch st(i)`
    fn xch(&mut self, i: usize) {
        let (a, b) = (self.n - 1, self.n - 1 - i);
        self.r.swap(a, b);
    }
    /// `fstp st(i)`
    fn stp(&mut self, i: usize) {
        let v = self.st(0);
        self.set(i, v);
        self.pop();
    }
}

/// `_rwCameraBuildPerspClipPlanes(camera)` (0x7EE740, catalogue `FUN_007ee740`): from the frame's LTM
/// (`frame + 0x50`), the view window/offset and the near/far distances, the 8 frustum corners
/// (+0x124: near 0..3, far 4..7) and the 6 frustum planes (+0x94, 0x14 bytes each: normal, distance,
/// three closest-corner bytes = 1 when the component's sign bit is clear else 0): far (normal = at),
/// near (-at), then four side planes from corner differences crossed and normalised with
/// `_rwInvSqrt`. Transcribed instruction by instruction from the bytes by a mechanical translator
/// (x87 stack in [`Fpu`], the function's stack slots in `loc` including the stores over its own
/// argument slot, the address comments are the original's), so the operation order, the f32 stores
/// between steps and the register-held intermediates are the original's. EAX (a closest-corner
/// byte, unused by the only caller) is not returned.
#[hook(addr = 0x7EE740, conv = cdecl, name = "FUN_007ee740", expect = "83 EC 34 8B 54 24 38")]
#[allow(unused_variables, clippy::all)]
pub fn rw_camera_build_persp_clip_planes(c: GPtr<u8>) {
    let mut f = Fpu::new();
    let mut loc = [0u32; 40];
    // 7ee740 sub esp,0x34
    // 7ee743 mov edx,DWORD PTR [esp+0x38]
    // 7ee747 push ebx
    // 7ee748 push ebp
    // 7ee749 push esi
    // 7ee74a fld DWORD PTR [edx+0x78]
    f.push(x(rd::<f32>(c, 0x78)));
    // 7ee74d mov eax,DWORD PTR [edx+0x4]
    let t1 = rd::<u32>(c, 4);
    // 7ee750 push edi
    // 7ee751 fchs 
    f.set(0, f.st(0).fchs());
    // 7ee753 add eax,0x50
    let m = p8(t1).byte_add(0x50);
    // 7ee756 lea edi,[edx+0x124]
    // 7ee75c fld st(0)
    f.push(f.st(0));
    // 7ee75e fmul DWORD PTR [eax]
    f.set(0, f.st(0).fmul_m(rd::<f32>(m, 0x0)));
    // 7ee760 fld DWORD PTR [eax+0x4]
    f.push(x(rd::<f32>(m, 0x4)));
    // 7ee763 fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7ee765 lea esi,[edx+0x94]
    // 7ee76b fstp DWORD PTR [esp+0x30]
    let t2 = f.st(0).to_f32();
    loc[27] = t2.to_bits();
    f.pop();
    // 7ee76f fld DWORD PTR [eax+0x8]
    f.push(x(rd::<f32>(m, 0x8)));
    // 7ee772 fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7ee774 fstp DWORD PTR [esp+0x34]
    let t3 = f.st(0).to_f32();
    loc[28] = t3.to_bits();
    f.pop();
    // 7ee778 fld DWORD PTR [edx+0x7c]
    f.push(x(rd::<f32>(c, 0x7c)));
    // 7ee77b fmul DWORD PTR [eax+0x10]
    f.set(0, f.st(0).fmul_m(rd::<f32>(m, 0x10)));
    // 7ee77e fadd st,st(1)
    f.set(0, f.st(0).fadd(f.st(1)));
    // 7ee780 fstp st(2)
    f.stp(2);
    // 7ee782 fstp st(0)
    f.stp(0);
    // 7ee784 fld DWORD PTR [edx+0x7c]
    f.push(x(rd::<f32>(c, 0x7c)));
    // 7ee787 fmul DWORD PTR [eax+0x14]
    f.set(0, f.st(0).fmul_m(rd::<f32>(m, 0x14)));
    // 7ee78a fadd DWORD PTR [esp+0x30]
    f.set(0, f.st(0).fadd_m(f32::from_bits(loc[27])));
    // 7ee78e fld DWORD PTR [edx+0x7c]
    f.push(x(rd::<f32>(c, 0x7c)));
    // 7ee791 fmul DWORD PTR [eax+0x18]
    f.set(0, f.st(0).fmul_m(rd::<f32>(m, 0x18)));
    // 7ee794 fadd DWORD PTR [esp+0x34]
    f.set(0, f.st(0).fadd_m(f32::from_bits(loc[28])));
    // 7ee798 fstp DWORD PTR [esp+0x34]
    let t4 = f.st(0).to_f32();
    loc[28] = t4.to_bits();
    f.pop();
    // 7ee79c fld DWORD PTR [edx+0x68]
    f.push(x(rd::<f32>(c, 0x68)));
    // 7ee79f fmul DWORD PTR [eax]
    f.set(0, f.st(0).fmul_m(rd::<f32>(m, 0x0)));
    // 7ee7a1 fld DWORD PTR [edx+0x68]
    f.push(x(rd::<f32>(c, 0x68)));
    // 7ee7a4 fmul DWORD PTR [eax+0x4]
    f.set(0, f.st(0).fmul_m(rd::<f32>(m, 0x4)));
    // 7ee7a7 fld DWORD PTR [edx+0x68]
    f.push(x(rd::<f32>(c, 0x68)));
    // 7ee7aa fmul DWORD PTR [eax+0x8]
    f.set(0, f.st(0).fmul_m(rd::<f32>(m, 0x8)));
    // 7ee7ad fstp DWORD PTR [esp+0x40]
    let t5 = f.st(0).to_f32();
    loc[31] = t5.to_bits();
    f.pop();
    // 7ee7b1 fld DWORD PTR [eax+0x20]
    f.push(x(rd::<f32>(m, 0x20)));
    // 7ee7b4 fadd st,st(2)
    f.set(0, f.st(0).fadd(f.st(2)));
    // 7ee7b6 fld DWORD PTR [eax+0x24]
    f.push(x(rd::<f32>(m, 0x24)));
    // 7ee7b9 fadd st,st(2)
    f.set(0, f.st(0).fadd(f.st(2)));
    // 7ee7bb fld DWORD PTR [eax+0x28]
    f.push(x(rd::<f32>(m, 0x28)));
    // 7ee7be fadd DWORD PTR [esp+0x40]
    f.set(0, f.st(0).fadd_m(f32::from_bits(loc[31])));
    // 7ee7c2 fstp DWORD PTR [esp+0x1c]
    let t6 = f.st(0).to_f32();
    loc[22] = t6.to_bits();
    f.pop();
    // 7ee7c6 fld DWORD PTR [edx+0x6c]
    f.push(x(rd::<f32>(c, 0x6c)));
    // 7ee7c9 fmul DWORD PTR [eax+0x10]
    f.set(0, f.st(0).fmul_m(rd::<f32>(m, 0x10)));
    // 7ee7cc mov ecx,edi
    // 7ee7ce fstp DWORD PTR [esp+0x20]
    let t7 = f.st(0).to_f32();
    loc[23] = t7.to_bits();
    f.pop();
    // 7ee7d2 fld DWORD PTR [edx+0x6c]
    f.push(x(rd::<f32>(c, 0x6c)));
    // 7ee7d5 fmul DWORD PTR [eax+0x14]
    f.set(0, f.st(0).fmul_m(rd::<f32>(m, 0x14)));
    // 7ee7d8 fstp DWORD PTR [esp+0x24]
    let t8 = f.st(0).to_f32();
    loc[24] = t8.to_bits();
    f.pop();
    // 7ee7dc fld DWORD PTR [edx+0x6c]
    f.push(x(rd::<f32>(c, 0x6c)));
    // 7ee7df fmul DWORD PTR [eax+0x18]
    f.set(0, f.st(0).fmul_m(rd::<f32>(m, 0x18)));
    // 7ee7e2 fstp DWORD PTR [esp+0x28]
    let t9 = f.st(0).to_f32();
    loc[25] = t9.to_bits();
    f.pop();
    // 7ee7e6 fld DWORD PTR [esp+0x20]
    f.push(x(f32::from_bits(loc[23])));
    // 7ee7ea fadd st,st(2)
    f.set(0, f.st(0).fadd(f.st(2)));
    // 7ee7ec fstp DWORD PTR [esp+0x14]
    let t10 = f.st(0).to_f32();
    loc[20] = t10.to_bits();
    f.pop();
    // 7ee7f0 fld DWORD PTR [esp+0x24]
    f.push(x(f32::from_bits(loc[24])));
    // 7ee7f4 fadd st,st(1)
    f.set(0, f.st(0).fadd(f.st(1)));
    // 7ee7f6 mov ebx,DWORD PTR [esp+0x14]
    let t11: u32 = loc[20];
    // 7ee7fa mov DWORD PTR [ecx],ebx
    wr::<u32>(c, 0x124, t11);
    // 7ee7fc fstp DWORD PTR [esp+0x18]
    let t12 = f.st(0).to_f32();
    loc[21] = t12.to_bits();
    f.pop();
    // 7ee800 mov ebx,DWORD PTR [esp+0x18]
    let t13: u32 = loc[21];
    // 7ee804 fstp st(0)
    f.stp(0);
    // 7ee806 fstp st(0)
    f.stp(0);
    // 7ee808 fld DWORD PTR [esp+0x28]
    f.push(x(f32::from_bits(loc[25])));
    // 7ee80c fadd DWORD PTR [esp+0x1c]
    f.set(0, f.st(0).fadd_m(f32::from_bits(loc[22])));
    // 7ee810 mov DWORD PTR [ecx+0x4],ebx
    wr::<u32>(c, 0x128, t13);
    // 7ee813 fstp DWORD PTR [esp+0x1c]
    let t14 = f.st(0).to_f32();
    loc[22] = t14.to_bits();
    f.pop();
    // 7ee817 fxch st(1)
    f.xch(1);
    // 7ee819 fadd st(0),st
    f.set(0, f.st(0).fadd(f.st(0)));
    // 7ee81b mov ebx,DWORD PTR [esp+0x1c]
    let t15: u32 = loc[22];
    // 7ee81f mov DWORD PTR [ecx+0x8],ebx
    wr::<u32>(c, 0x12c, t15);
    // 7ee822 lea ebx,[edi+0xc]
    // 7ee825 fstp DWORD PTR [esp+0x38]
    let t16 = f.st(0).to_f32();
    loc[29] = t16.to_bits();
    f.pop();
    // 7ee829 mov ecx,ebx
    // 7ee82b fadd st(0),st
    f.set(0, f.st(0).fadd(f.st(0)));
    // 7ee82d fld DWORD PTR [esp+0x40]
    f.push(x(f32::from_bits(loc[31])));
    // 7ee831 fadd st(0),st
    f.set(0, f.st(0).fadd(f.st(0)));
    // 7ee833 fld DWORD PTR [esp+0x20]
    f.push(x(f32::from_bits(loc[23])));
    // 7ee837 fadd st(0),st
    f.set(0, f.st(0).fadd(f.st(0)));
    // 7ee839 fld DWORD PTR [esp+0x24]
    f.push(x(f32::from_bits(loc[24])));
    // 7ee83d fadd st(0),st
    f.set(0, f.st(0).fadd(f.st(0)));
    // 7ee83f fld DWORD PTR [esp+0x28]
    f.push(x(f32::from_bits(loc[25])));
    // 7ee843 fadd st(0),st
    f.set(0, f.st(0).fadd(f.st(0)));
    // 7ee845 fstp DWORD PTR [esp+0x28]
    let t17 = f.st(0).to_f32();
    loc[25] = t17.to_bits();
    f.pop();
    // 7ee849 fld DWORD PTR [esp+0x14]
    f.push(x(f32::from_bits(loc[20])));
    // 7ee84d fsub DWORD PTR [esp+0x38]
    f.set(0, f.st(0).fsub_m(f32::from_bits(loc[29])));
    // 7ee851 fstp DWORD PTR [esp+0x14]
    let t18 = f.st(0).to_f32();
    loc[20] = t18.to_bits();
    f.pop();
    // 7ee855 fld DWORD PTR [esp+0x18]
    f.push(x(f32::from_bits(loc[21])));
    // 7ee859 fsub st,st(4)
    f.set(0, f.st(0).fsub(f.st(4)));
    // 7ee85b mov ebp,DWORD PTR [esp+0x14]
    let t19: u32 = loc[20];
    // 7ee85f mov DWORD PTR [ecx],ebp
    wr::<u32>(c, 0x130, t19);
    // 7ee861 fstp DWORD PTR [esp+0x18]
    let t20 = f.st(0).to_f32();
    loc[21] = t20.to_bits();
    f.pop();
    // 7ee865 fld DWORD PTR [esp+0x1c]
    f.push(x(f32::from_bits(loc[22])));
    // 7ee869 mov ebp,DWORD PTR [esp+0x18]
    let t21: u32 = loc[21];
    // 7ee86d fsub st,st(3)
    f.set(0, f.st(0).fsub(f.st(3)));
    // 7ee86f mov DWORD PTR [ecx+0x4],ebp
    wr::<u32>(c, 0x134, t21);
    // 7ee872 fstp DWORD PTR [esp+0x1c]
    let t22 = f.st(0).to_f32();
    loc[22] = t22.to_bits();
    f.pop();
    // 7ee876 mov ebp,DWORD PTR [esp+0x1c]
    let t23: u32 = loc[22];
    // 7ee87a fld DWORD PTR [esp+0x14]
    f.push(x(f32::from_bits(loc[20])));
    // 7ee87e mov DWORD PTR [ecx+0x8],ebp
    wr::<u32>(c, 0x138, t23);
    // 7ee881 fsub st,st(2)
    f.set(0, f.st(0).fsub(f.st(2)));
    // 7ee883 fstp DWORD PTR [esp+0x14]
    let t24 = f.st(0).to_f32();
    loc[20] = t24.to_bits();
    f.pop();
    // 7ee887 fld DWORD PTR [esp+0x18]
    f.push(x(f32::from_bits(loc[21])));
    // 7ee88b fsub st,st(1)
    f.set(0, f.st(0).fsub(f.st(1)));
    // 7ee88d mov ebp,DWORD PTR [esp+0x14]
    let t25: u32 = loc[20];
    // 7ee891 lea ecx,[edi+0x18]
    // 7ee894 fstp DWORD PTR [esp+0x18]
    let t26 = f.st(0).to_f32();
    loc[21] = t26.to_bits();
    f.pop();
    // 7ee898 mov DWORD PTR [ecx],ebp
    wr::<u32>(c, 0x13c, t25);
    // 7ee89a mov ebp,DWORD PTR [esp+0x18]
    let t27: u32 = loc[21];
    // 7ee89e fstp st(0)
    f.stp(0);
    // 7ee8a0 fstp st(0)
    f.stp(0);
    // 7ee8a2 fld DWORD PTR [esp+0x1c]
    f.push(x(f32::from_bits(loc[22])));
    // 7ee8a6 fsub DWORD PTR [esp+0x28]
    f.set(0, f.st(0).fsub_m(f32::from_bits(loc[25])));
    // 7ee8aa mov DWORD PTR [ecx+0x4],ebp
    wr::<u32>(c, 0x140, t27);
    // 7ee8ad fstp DWORD PTR [esp+0x1c]
    let t28 = f.st(0).to_f32();
    loc[22] = t28.to_bits();
    f.pop();
    // 7ee8b1 fld DWORD PTR [esp+0x14]
    f.push(x(f32::from_bits(loc[20])));
    // 7ee8b5 fadd DWORD PTR [esp+0x38]
    f.set(0, f.st(0).fadd_m(f32::from_bits(loc[29])));
    // 7ee8b9 mov ebp,DWORD PTR [esp+0x1c]
    let t29: u32 = loc[22];
    // 7ee8bd mov DWORD PTR [ecx+0x8],ebp
    wr::<u32>(c, 0x144, t29);
    // 7ee8c0 lea ecx,[edi+0x24]
    // 7ee8c3 fstp DWORD PTR [esp+0x14]
    let t30 = f.st(0).to_f32();
    loc[20] = t30.to_bits();
    f.pop();
    // 7ee8c7 fld DWORD PTR [esp+0x18]
    f.push(x(f32::from_bits(loc[21])));
    // 7ee8cb fadd st,st(2)
    f.set(0, f.st(0).fadd(f.st(2)));
    // 7ee8cd mov ebp,DWORD PTR [esp+0x14]
    let t31: u32 = loc[20];
    // 7ee8d1 mov DWORD PTR [ecx],ebp
    wr::<u32>(c, 0x148, t31);
    // 7ee8d3 fstp DWORD PTR [esp+0x18]
    let t32 = f.st(0).to_f32();
    loc[21] = t32.to_bits();
    f.pop();
    // 7ee8d7 fld DWORD PTR [esp+0x1c]
    f.push(x(f32::from_bits(loc[22])));
    // 7ee8db mov ebp,DWORD PTR [esp+0x18]
    let t33: u32 = loc[21];
    // 7ee8df fadd st,st(1)
    f.set(0, f.st(0).fadd(f.st(1)));
    // 7ee8e1 mov DWORD PTR [ecx+0x4],ebp
    wr::<u32>(c, 0x14c, t33);
    // 7ee8e4 fstp DWORD PTR [esp+0x1c]
    let t34 = f.st(0).to_f32();
    loc[22] = t34.to_bits();
    f.pop();
    // 7ee8e8 mov ebp,DWORD PTR [esp+0x1c]
    let t35: u32 = loc[22];
    // 7ee8ec fstp st(0)
    f.stp(0);
    // 7ee8ee mov DWORD PTR [ecx+0x8],ebp
    wr::<u32>(c, 0x150, t35);
    // 7ee8f1 lea ecx,[edi+0x38]
    // 7ee8f4 fstp st(0)
    f.stp(0);
    // 7ee8f6 mov ebp,0x4
    // 7ee8fb fld DWORD PTR [ecx-0x38]
    f.push(x(rd::<f32>(c, 0x124)));
    // 7ee8fe fsub st,st(2)
    f.set(0, f.st(0).fsub(f.st(2)));
    // 7ee900 fld DWORD PTR [ecx-0x34]
    f.push(x(rd::<f32>(c, 0x128)));
    // 7ee903 fsub st,st(2)
    f.set(0, f.st(0).fsub(f.st(2)));
    // 7ee905 fld DWORD PTR [ecx-0x30]
    f.push(x(rd::<f32>(c, 0x12c)));
    // 7ee908 fsub DWORD PTR [esp+0x34]
    f.set(0, f.st(0).fsub_m(f32::from_bits(loc[28])));
    // 7ee90c fstp DWORD PTR [esp+0x1c]
    let t36 = f.st(0).to_f32();
    loc[22] = t36.to_bits();
    f.pop();
    // 7ee910 fld DWORD PTR [eax+0x30]
    f.push(x(rd::<f32>(m, 0x30)));
    // 7ee913 fadd st,st(4)
    f.set(0, f.st(0).fadd(f.st(4)));
    // 7ee915 fstp DWORD PTR [ecx-0x38]
    let t37 = f.st(0).to_f32();
    wr::<f32>(c, 0x124, t37);
    f.pop();
    // 7ee918 fld DWORD PTR [eax+0x34]
    f.push(x(rd::<f32>(m, 0x34)));
    // 7ee91b fadd st,st(3)
    f.set(0, f.st(0).fadd(f.st(3)));
    // 7ee91d fstp DWORD PTR [ecx-0x34]
    let t38 = f.st(0).to_f32();
    wr::<f32>(c, 0x128, t38);
    f.pop();
    // 7ee920 fld DWORD PTR [eax+0x38]
    f.push(x(rd::<f32>(m, 0x38)));
    // 7ee923 fadd DWORD PTR [esp+0x34]
    f.set(0, f.st(0).fadd_m(f32::from_bits(loc[28])));
    // 7ee927 fstp DWORD PTR [ecx-0x30]
    let t39 = f.st(0).to_f32();
    wr::<f32>(c, 0x12c, t39);
    f.pop();
    // 7ee92a fld DWORD PTR [edx+0x80]
    f.push(x(rd::<f32>(c, 0x80)));
    // 7ee930 fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7ee932 fadd DWORD PTR [ecx-0x38]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x124)));
    // 7ee935 fstp DWORD PTR [ecx-0x38]
    let t40 = f.st(0).to_f32();
    wr::<f32>(c, 0x124, t40);
    f.pop();
    // 7ee938 fld DWORD PTR [edx+0x80]
    f.push(x(rd::<f32>(c, 0x80)));
    // 7ee93e fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7ee940 fadd DWORD PTR [ecx-0x34]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x128)));
    // 7ee943 fstp DWORD PTR [ecx-0x34]
    let t41 = f.st(0).to_f32();
    wr::<f32>(c, 0x128, t41);
    f.pop();
    // 7ee946 fld DWORD PTR [edx+0x80]
    f.push(x(rd::<f32>(c, 0x80)));
    // 7ee94c fmul DWORD PTR [esp+0x1c]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[22])));
    // 7ee950 fadd DWORD PTR [ecx-0x30]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x12c)));
    // 7ee953 fstp DWORD PTR [ecx-0x30]
    let t42 = f.st(0).to_f32();
    wr::<f32>(c, 0x12c, t42);
    f.pop();
    // 7ee956 fld DWORD PTR [eax+0x30]
    f.push(x(rd::<f32>(m, 0x30)));
    // 7ee959 fadd st,st(4)
    f.set(0, f.st(0).fadd(f.st(4)));
    // 7ee95b fstp DWORD PTR [ecx-0x8]
    let t43 = f.st(0).to_f32();
    wr::<f32>(c, 0x154, t43);
    f.pop();
    // 7ee95e fld DWORD PTR [eax+0x34]
    f.push(x(rd::<f32>(m, 0x34)));
    // 7ee961 fadd st,st(3)
    f.set(0, f.st(0).fadd(f.st(3)));
    // 7ee963 fstp DWORD PTR [ecx-0x4]
    let t44 = f.st(0).to_f32();
    wr::<f32>(c, 0x158, t44);
    f.pop();
    // 7ee966 fld DWORD PTR [eax+0x38]
    f.push(x(rd::<f32>(m, 0x38)));
    // 7ee969 fadd DWORD PTR [esp+0x34]
    f.set(0, f.st(0).fadd_m(f32::from_bits(loc[28])));
    // 7ee96d fstp DWORD PTR [ecx]
    let t45 = f.st(0).to_f32();
    wr::<f32>(c, 0x15c, t45);
    f.pop();
    // 7ee96f fld DWORD PTR [edx+0x84]
    f.push(x(rd::<f32>(c, 0x84)));
    // 7ee975 fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7ee977 fadd DWORD PTR [ecx-0x8]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x154)));
    // 7ee97a fstp DWORD PTR [ecx-0x8]
    let t46 = f.st(0).to_f32();
    wr::<f32>(c, 0x154, t46);
    f.pop();
    // 7ee97d fld DWORD PTR [edx+0x84]
    f.push(x(rd::<f32>(c, 0x84)));
    // 7ee983 fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7ee985 fadd DWORD PTR [ecx-0x4]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x158)));
    // 7ee988 fstp DWORD PTR [ecx-0x4]
    let t47 = f.st(0).to_f32();
    wr::<f32>(c, 0x158, t47);
    f.pop();
    // 7ee98b fstp st(0)
    f.stp(0);
    // 7ee98d fstp st(0)
    f.stp(0);
    // 7ee98f fld DWORD PTR [edx+0x84]
    f.push(x(rd::<f32>(c, 0x84)));
    // 7ee995 fmul DWORD PTR [esp+0x1c]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[22])));
    // 7ee999 add ecx,0xc
    // 7ee99c dec ebp
    // 7ee99d fadd DWORD PTR [ecx-0xc]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x15c)));
    // 7ee9a0 fstp DWORD PTR [ecx-0xc]
    let t48 = f.st(0).to_f32();
    wr::<f32>(c, 0x15c, t48);
    f.pop();
    // 7ee9a3 jne 0x7ee8fb
    // 7ee8fb fld DWORD PTR [ecx-0x38]
    f.push(x(rd::<f32>(c, 0x130)));
    // 7ee8fe fsub st,st(2)
    f.set(0, f.st(0).fsub(f.st(2)));
    // 7ee900 fld DWORD PTR [ecx-0x34]
    f.push(x(rd::<f32>(c, 0x134)));
    // 7ee903 fsub st,st(2)
    f.set(0, f.st(0).fsub(f.st(2)));
    // 7ee905 fld DWORD PTR [ecx-0x30]
    f.push(x(rd::<f32>(c, 0x138)));
    // 7ee908 fsub DWORD PTR [esp+0x34]
    f.set(0, f.st(0).fsub_m(f32::from_bits(loc[28])));
    // 7ee90c fstp DWORD PTR [esp+0x1c]
    let t49 = f.st(0).to_f32();
    loc[22] = t49.to_bits();
    f.pop();
    // 7ee910 fld DWORD PTR [eax+0x30]
    f.push(x(rd::<f32>(m, 0x30)));
    // 7ee913 fadd st,st(4)
    f.set(0, f.st(0).fadd(f.st(4)));
    // 7ee915 fstp DWORD PTR [ecx-0x38]
    let t50 = f.st(0).to_f32();
    wr::<f32>(c, 0x130, t50);
    f.pop();
    // 7ee918 fld DWORD PTR [eax+0x34]
    f.push(x(rd::<f32>(m, 0x34)));
    // 7ee91b fadd st,st(3)
    f.set(0, f.st(0).fadd(f.st(3)));
    // 7ee91d fstp DWORD PTR [ecx-0x34]
    let t51 = f.st(0).to_f32();
    wr::<f32>(c, 0x134, t51);
    f.pop();
    // 7ee920 fld DWORD PTR [eax+0x38]
    f.push(x(rd::<f32>(m, 0x38)));
    // 7ee923 fadd DWORD PTR [esp+0x34]
    f.set(0, f.st(0).fadd_m(f32::from_bits(loc[28])));
    // 7ee927 fstp DWORD PTR [ecx-0x30]
    let t52 = f.st(0).to_f32();
    wr::<f32>(c, 0x138, t52);
    f.pop();
    // 7ee92a fld DWORD PTR [edx+0x80]
    f.push(x(rd::<f32>(c, 0x80)));
    // 7ee930 fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7ee932 fadd DWORD PTR [ecx-0x38]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x130)));
    // 7ee935 fstp DWORD PTR [ecx-0x38]
    let t53 = f.st(0).to_f32();
    wr::<f32>(c, 0x130, t53);
    f.pop();
    // 7ee938 fld DWORD PTR [edx+0x80]
    f.push(x(rd::<f32>(c, 0x80)));
    // 7ee93e fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7ee940 fadd DWORD PTR [ecx-0x34]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x134)));
    // 7ee943 fstp DWORD PTR [ecx-0x34]
    let t54 = f.st(0).to_f32();
    wr::<f32>(c, 0x134, t54);
    f.pop();
    // 7ee946 fld DWORD PTR [edx+0x80]
    f.push(x(rd::<f32>(c, 0x80)));
    // 7ee94c fmul DWORD PTR [esp+0x1c]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[22])));
    // 7ee950 fadd DWORD PTR [ecx-0x30]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x138)));
    // 7ee953 fstp DWORD PTR [ecx-0x30]
    let t55 = f.st(0).to_f32();
    wr::<f32>(c, 0x138, t55);
    f.pop();
    // 7ee956 fld DWORD PTR [eax+0x30]
    f.push(x(rd::<f32>(m, 0x30)));
    // 7ee959 fadd st,st(4)
    f.set(0, f.st(0).fadd(f.st(4)));
    // 7ee95b fstp DWORD PTR [ecx-0x8]
    let t56 = f.st(0).to_f32();
    wr::<f32>(c, 0x160, t56);
    f.pop();
    // 7ee95e fld DWORD PTR [eax+0x34]
    f.push(x(rd::<f32>(m, 0x34)));
    // 7ee961 fadd st,st(3)
    f.set(0, f.st(0).fadd(f.st(3)));
    // 7ee963 fstp DWORD PTR [ecx-0x4]
    let t57 = f.st(0).to_f32();
    wr::<f32>(c, 0x164, t57);
    f.pop();
    // 7ee966 fld DWORD PTR [eax+0x38]
    f.push(x(rd::<f32>(m, 0x38)));
    // 7ee969 fadd DWORD PTR [esp+0x34]
    f.set(0, f.st(0).fadd_m(f32::from_bits(loc[28])));
    // 7ee96d fstp DWORD PTR [ecx]
    let t58 = f.st(0).to_f32();
    wr::<f32>(c, 0x168, t58);
    f.pop();
    // 7ee96f fld DWORD PTR [edx+0x84]
    f.push(x(rd::<f32>(c, 0x84)));
    // 7ee975 fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7ee977 fadd DWORD PTR [ecx-0x8]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x160)));
    // 7ee97a fstp DWORD PTR [ecx-0x8]
    let t59 = f.st(0).to_f32();
    wr::<f32>(c, 0x160, t59);
    f.pop();
    // 7ee97d fld DWORD PTR [edx+0x84]
    f.push(x(rd::<f32>(c, 0x84)));
    // 7ee983 fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7ee985 fadd DWORD PTR [ecx-0x4]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x164)));
    // 7ee988 fstp DWORD PTR [ecx-0x4]
    let t60 = f.st(0).to_f32();
    wr::<f32>(c, 0x164, t60);
    f.pop();
    // 7ee98b fstp st(0)
    f.stp(0);
    // 7ee98d fstp st(0)
    f.stp(0);
    // 7ee98f fld DWORD PTR [edx+0x84]
    f.push(x(rd::<f32>(c, 0x84)));
    // 7ee995 fmul DWORD PTR [esp+0x1c]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[22])));
    // 7ee999 add ecx,0xc
    // 7ee99c dec ebp
    // 7ee99d fadd DWORD PTR [ecx-0xc]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x168)));
    // 7ee9a0 fstp DWORD PTR [ecx-0xc]
    let t61 = f.st(0).to_f32();
    wr::<f32>(c, 0x168, t61);
    f.pop();
    // 7ee9a3 jne 0x7ee8fb
    // 7ee8fb fld DWORD PTR [ecx-0x38]
    f.push(x(rd::<f32>(c, 0x13c)));
    // 7ee8fe fsub st,st(2)
    f.set(0, f.st(0).fsub(f.st(2)));
    // 7ee900 fld DWORD PTR [ecx-0x34]
    f.push(x(rd::<f32>(c, 0x140)));
    // 7ee903 fsub st,st(2)
    f.set(0, f.st(0).fsub(f.st(2)));
    // 7ee905 fld DWORD PTR [ecx-0x30]
    f.push(x(rd::<f32>(c, 0x144)));
    // 7ee908 fsub DWORD PTR [esp+0x34]
    f.set(0, f.st(0).fsub_m(f32::from_bits(loc[28])));
    // 7ee90c fstp DWORD PTR [esp+0x1c]
    let t62 = f.st(0).to_f32();
    loc[22] = t62.to_bits();
    f.pop();
    // 7ee910 fld DWORD PTR [eax+0x30]
    f.push(x(rd::<f32>(m, 0x30)));
    // 7ee913 fadd st,st(4)
    f.set(0, f.st(0).fadd(f.st(4)));
    // 7ee915 fstp DWORD PTR [ecx-0x38]
    let t63 = f.st(0).to_f32();
    wr::<f32>(c, 0x13c, t63);
    f.pop();
    // 7ee918 fld DWORD PTR [eax+0x34]
    f.push(x(rd::<f32>(m, 0x34)));
    // 7ee91b fadd st,st(3)
    f.set(0, f.st(0).fadd(f.st(3)));
    // 7ee91d fstp DWORD PTR [ecx-0x34]
    let t64 = f.st(0).to_f32();
    wr::<f32>(c, 0x140, t64);
    f.pop();
    // 7ee920 fld DWORD PTR [eax+0x38]
    f.push(x(rd::<f32>(m, 0x38)));
    // 7ee923 fadd DWORD PTR [esp+0x34]
    f.set(0, f.st(0).fadd_m(f32::from_bits(loc[28])));
    // 7ee927 fstp DWORD PTR [ecx-0x30]
    let t65 = f.st(0).to_f32();
    wr::<f32>(c, 0x144, t65);
    f.pop();
    // 7ee92a fld DWORD PTR [edx+0x80]
    f.push(x(rd::<f32>(c, 0x80)));
    // 7ee930 fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7ee932 fadd DWORD PTR [ecx-0x38]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x13c)));
    // 7ee935 fstp DWORD PTR [ecx-0x38]
    let t66 = f.st(0).to_f32();
    wr::<f32>(c, 0x13c, t66);
    f.pop();
    // 7ee938 fld DWORD PTR [edx+0x80]
    f.push(x(rd::<f32>(c, 0x80)));
    // 7ee93e fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7ee940 fadd DWORD PTR [ecx-0x34]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x140)));
    // 7ee943 fstp DWORD PTR [ecx-0x34]
    let t67 = f.st(0).to_f32();
    wr::<f32>(c, 0x140, t67);
    f.pop();
    // 7ee946 fld DWORD PTR [edx+0x80]
    f.push(x(rd::<f32>(c, 0x80)));
    // 7ee94c fmul DWORD PTR [esp+0x1c]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[22])));
    // 7ee950 fadd DWORD PTR [ecx-0x30]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x144)));
    // 7ee953 fstp DWORD PTR [ecx-0x30]
    let t68 = f.st(0).to_f32();
    wr::<f32>(c, 0x144, t68);
    f.pop();
    // 7ee956 fld DWORD PTR [eax+0x30]
    f.push(x(rd::<f32>(m, 0x30)));
    // 7ee959 fadd st,st(4)
    f.set(0, f.st(0).fadd(f.st(4)));
    // 7ee95b fstp DWORD PTR [ecx-0x8]
    let t69 = f.st(0).to_f32();
    wr::<f32>(c, 0x16c, t69);
    f.pop();
    // 7ee95e fld DWORD PTR [eax+0x34]
    f.push(x(rd::<f32>(m, 0x34)));
    // 7ee961 fadd st,st(3)
    f.set(0, f.st(0).fadd(f.st(3)));
    // 7ee963 fstp DWORD PTR [ecx-0x4]
    let t70 = f.st(0).to_f32();
    wr::<f32>(c, 0x170, t70);
    f.pop();
    // 7ee966 fld DWORD PTR [eax+0x38]
    f.push(x(rd::<f32>(m, 0x38)));
    // 7ee969 fadd DWORD PTR [esp+0x34]
    f.set(0, f.st(0).fadd_m(f32::from_bits(loc[28])));
    // 7ee96d fstp DWORD PTR [ecx]
    let t71 = f.st(0).to_f32();
    wr::<f32>(c, 0x174, t71);
    f.pop();
    // 7ee96f fld DWORD PTR [edx+0x84]
    f.push(x(rd::<f32>(c, 0x84)));
    // 7ee975 fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7ee977 fadd DWORD PTR [ecx-0x8]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x16c)));
    // 7ee97a fstp DWORD PTR [ecx-0x8]
    let t72 = f.st(0).to_f32();
    wr::<f32>(c, 0x16c, t72);
    f.pop();
    // 7ee97d fld DWORD PTR [edx+0x84]
    f.push(x(rd::<f32>(c, 0x84)));
    // 7ee983 fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7ee985 fadd DWORD PTR [ecx-0x4]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x170)));
    // 7ee988 fstp DWORD PTR [ecx-0x4]
    let t73 = f.st(0).to_f32();
    wr::<f32>(c, 0x170, t73);
    f.pop();
    // 7ee98b fstp st(0)
    f.stp(0);
    // 7ee98d fstp st(0)
    f.stp(0);
    // 7ee98f fld DWORD PTR [edx+0x84]
    f.push(x(rd::<f32>(c, 0x84)));
    // 7ee995 fmul DWORD PTR [esp+0x1c]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[22])));
    // 7ee999 add ecx,0xc
    // 7ee99c dec ebp
    // 7ee99d fadd DWORD PTR [ecx-0xc]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x174)));
    // 7ee9a0 fstp DWORD PTR [ecx-0xc]
    let t74 = f.st(0).to_f32();
    wr::<f32>(c, 0x174, t74);
    f.pop();
    // 7ee9a3 jne 0x7ee8fb
    // 7ee8fb fld DWORD PTR [ecx-0x38]
    f.push(x(rd::<f32>(c, 0x148)));
    // 7ee8fe fsub st,st(2)
    f.set(0, f.st(0).fsub(f.st(2)));
    // 7ee900 fld DWORD PTR [ecx-0x34]
    f.push(x(rd::<f32>(c, 0x14c)));
    // 7ee903 fsub st,st(2)
    f.set(0, f.st(0).fsub(f.st(2)));
    // 7ee905 fld DWORD PTR [ecx-0x30]
    f.push(x(rd::<f32>(c, 0x150)));
    // 7ee908 fsub DWORD PTR [esp+0x34]
    f.set(0, f.st(0).fsub_m(f32::from_bits(loc[28])));
    // 7ee90c fstp DWORD PTR [esp+0x1c]
    let t75 = f.st(0).to_f32();
    loc[22] = t75.to_bits();
    f.pop();
    // 7ee910 fld DWORD PTR [eax+0x30]
    f.push(x(rd::<f32>(m, 0x30)));
    // 7ee913 fadd st,st(4)
    f.set(0, f.st(0).fadd(f.st(4)));
    // 7ee915 fstp DWORD PTR [ecx-0x38]
    let t76 = f.st(0).to_f32();
    wr::<f32>(c, 0x148, t76);
    f.pop();
    // 7ee918 fld DWORD PTR [eax+0x34]
    f.push(x(rd::<f32>(m, 0x34)));
    // 7ee91b fadd st,st(3)
    f.set(0, f.st(0).fadd(f.st(3)));
    // 7ee91d fstp DWORD PTR [ecx-0x34]
    let t77 = f.st(0).to_f32();
    wr::<f32>(c, 0x14c, t77);
    f.pop();
    // 7ee920 fld DWORD PTR [eax+0x38]
    f.push(x(rd::<f32>(m, 0x38)));
    // 7ee923 fadd DWORD PTR [esp+0x34]
    f.set(0, f.st(0).fadd_m(f32::from_bits(loc[28])));
    // 7ee927 fstp DWORD PTR [ecx-0x30]
    let t78 = f.st(0).to_f32();
    wr::<f32>(c, 0x150, t78);
    f.pop();
    // 7ee92a fld DWORD PTR [edx+0x80]
    f.push(x(rd::<f32>(c, 0x80)));
    // 7ee930 fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7ee932 fadd DWORD PTR [ecx-0x38]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x148)));
    // 7ee935 fstp DWORD PTR [ecx-0x38]
    let t79 = f.st(0).to_f32();
    wr::<f32>(c, 0x148, t79);
    f.pop();
    // 7ee938 fld DWORD PTR [edx+0x80]
    f.push(x(rd::<f32>(c, 0x80)));
    // 7ee93e fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7ee940 fadd DWORD PTR [ecx-0x34]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x14c)));
    // 7ee943 fstp DWORD PTR [ecx-0x34]
    let t80 = f.st(0).to_f32();
    wr::<f32>(c, 0x14c, t80);
    f.pop();
    // 7ee946 fld DWORD PTR [edx+0x80]
    f.push(x(rd::<f32>(c, 0x80)));
    // 7ee94c fmul DWORD PTR [esp+0x1c]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[22])));
    // 7ee950 fadd DWORD PTR [ecx-0x30]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x150)));
    // 7ee953 fstp DWORD PTR [ecx-0x30]
    let t81 = f.st(0).to_f32();
    wr::<f32>(c, 0x150, t81);
    f.pop();
    // 7ee956 fld DWORD PTR [eax+0x30]
    f.push(x(rd::<f32>(m, 0x30)));
    // 7ee959 fadd st,st(4)
    f.set(0, f.st(0).fadd(f.st(4)));
    // 7ee95b fstp DWORD PTR [ecx-0x8]
    let t82 = f.st(0).to_f32();
    wr::<f32>(c, 0x178, t82);
    f.pop();
    // 7ee95e fld DWORD PTR [eax+0x34]
    f.push(x(rd::<f32>(m, 0x34)));
    // 7ee961 fadd st,st(3)
    f.set(0, f.st(0).fadd(f.st(3)));
    // 7ee963 fstp DWORD PTR [ecx-0x4]
    let t83 = f.st(0).to_f32();
    wr::<f32>(c, 0x17c, t83);
    f.pop();
    // 7ee966 fld DWORD PTR [eax+0x38]
    f.push(x(rd::<f32>(m, 0x38)));
    // 7ee969 fadd DWORD PTR [esp+0x34]
    f.set(0, f.st(0).fadd_m(f32::from_bits(loc[28])));
    // 7ee96d fstp DWORD PTR [ecx]
    let t84 = f.st(0).to_f32();
    wr::<f32>(c, 0x180, t84);
    f.pop();
    // 7ee96f fld DWORD PTR [edx+0x84]
    f.push(x(rd::<f32>(c, 0x84)));
    // 7ee975 fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7ee977 fadd DWORD PTR [ecx-0x8]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x178)));
    // 7ee97a fstp DWORD PTR [ecx-0x8]
    let t85 = f.st(0).to_f32();
    wr::<f32>(c, 0x178, t85);
    f.pop();
    // 7ee97d fld DWORD PTR [edx+0x84]
    f.push(x(rd::<f32>(c, 0x84)));
    // 7ee983 fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7ee985 fadd DWORD PTR [ecx-0x4]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x17c)));
    // 7ee988 fstp DWORD PTR [ecx-0x4]
    let t86 = f.st(0).to_f32();
    wr::<f32>(c, 0x17c, t86);
    f.pop();
    // 7ee98b fstp st(0)
    f.stp(0);
    // 7ee98d fstp st(0)
    f.stp(0);
    // 7ee98f fld DWORD PTR [edx+0x84]
    f.push(x(rd::<f32>(c, 0x84)));
    // 7ee995 fmul DWORD PTR [esp+0x1c]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[22])));
    // 7ee999 add ecx,0xc
    // 7ee99c dec ebp
    // 7ee99d fadd DWORD PTR [ecx-0xc]
    f.set(0, f.st(0).fadd_m(rd::<f32>(c, 0x180)));
    // 7ee9a0 fstp DWORD PTR [ecx-0xc]
    let t87 = f.st(0).to_f32();
    wr::<f32>(c, 0x180, t87);
    f.pop();
    // 7ee9a3 jne 0x7ee8fb
    // 7ee9a9 lea ecx,[eax+0x20]
    // 7ee9ac mov edx,esi
    // 7ee9ae fstp st(0)
    f.stp(0);
    // 7ee9b0 mov ebp,DWORD PTR [ecx]
    let t88: u32 = rd::<u32>(m, 0x20);
    // 7ee9b2 mov DWORD PTR [edx],ebp
    wr::<u32>(c, 0x94, t88);
    // 7ee9b4 mov ebp,DWORD PTR [ecx+0x4]
    let t89: u32 = rd::<u32>(m, 0x24);
    // 7ee9b7 mov DWORD PTR [edx+0x4],ebp
    wr::<u32>(c, 0x98, t89);
    // 7ee9ba mov ecx,DWORD PTR [ecx+0x8]
    let t90: u32 = rd::<u32>(m, 0x28);
    // 7ee9bd fstp st(0)
    f.stp(0);
    // 7ee9bf mov DWORD PTR [edx+0x8],ecx
    wr::<u32>(c, 0x9c, t90);
    // 7ee9c2 mov edx,DWORD PTR [esi]
    let t91: u32 = rd::<u32>(c, 0x94);
    // 7ee9c4 fld DWORD PTR [eax+0x20]
    f.push(x(rd::<f32>(m, 0x20)));
    // 7ee9c7 fmul DWORD PTR [edi+0x30]
    f.set(0, f.st(0).fmul_m(rd::<f32>(c, 0x154)));
    // 7ee9ca mov ecx,DWORD PTR [esi+0x8]
    let t92: u32 = rd::<u32>(c, 0x9c);
    // 7ee9cd sar edx,0x1f
    let t93 = ((t91 as i32) >> 31) as u32;
    // 7ee9d0 fld DWORD PTR [edi+0x34]
    f.push(x(rd::<f32>(c, 0x158)));
    // 7ee9d3 fmul DWORD PTR [eax+0x24]
    f.set(0, f.st(0).fmul_m(rd::<f32>(m, 0x24)));
    // 7ee9d6 inc dl
    let t94 = (t93 & !0xFF) | u32::from((t93 as u8).wrapping_add(1));
    // 7ee9d8 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7ee9da sar ecx,0x1f
    let t95 = ((t92 as i32) >> 31) as u32;
    // 7ee9dd fld DWORD PTR [edi+0x38]
    f.push(x(rd::<f32>(c, 0x15c)));
    // 7ee9e0 fmul DWORD PTR [eax+0x28]
    f.set(0, f.st(0).fmul_m(rd::<f32>(m, 0x28)));
    // 7ee9e3 mov eax,DWORD PTR [esi+0x4]
    let t96: u32 = rd::<u32>(c, 0x98);
    // 7ee9e6 mov BYTE PTR [esi+0x10],dl
    wr::<u8>(c, 0xa4, t94 as u8);
    // 7ee9e9 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7ee9eb sar eax,0x1f
    let t97 = ((t96 as i32) >> 31) as u32;
    // 7ee9ee inc al
    let t98 = (t97 & !0xFF) | u32::from((t97 as u8).wrapping_add(1));
    // 7ee9f0 inc cl
    let t99 = (t95 & !0xFF) | u32::from((t95 as u8).wrapping_add(1));
    // 7ee9f2 fstp DWORD PTR [esi+0xc]
    let t100 = f.st(0).to_f32();
    wr::<f32>(c, 0xa0, t100);
    f.pop();
    // 7ee9f5 fld DWORD PTR [esi]
    f.push(x(rd::<f32>(c, 0x94)));
    // 7ee9f7 fchs 
    f.set(0, f.st(0).fchs());
    // 7ee9f9 fst DWORD PTR [esi+0x14]
    let t101 = f.st(0).to_f32();
    wr::<f32>(c, 0xa8, t101);
    // 7ee9fc fld DWORD PTR [esi+0x4]
    f.push(x(rd::<f32>(c, 0x98)));
    // 7ee9ff fchs 
    f.set(0, f.st(0).fchs());
    // 7eea01 fst DWORD PTR [esi+0x18]
    let t102 = f.st(0).to_f32();
    wr::<f32>(c, 0xac, t102);
    // 7eea04 fld DWORD PTR [esi+0x8]
    f.push(x(rd::<f32>(c, 0x9c)));
    // 7eea07 fchs 
    f.set(0, f.st(0).fchs());
    // 7eea09 fst DWORD PTR [esi+0x1c]
    let t103 = f.st(0).to_f32();
    wr::<f32>(c, 0xb0, t103);
    // 7eea0c mov BYTE PTR [esi+0x11],al
    wr::<u8>(c, 0xa5, t98 as u8);
    // 7eea0f mov BYTE PTR [esi+0x12],cl
    wr::<u8>(c, 0xa6, t99 as u8);
    // 7eea12 fld DWORD PTR [edi+0x4]
    f.push(x(rd::<f32>(c, 0x128)));
    // 7eea15 fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7eea17 mov edx,DWORD PTR [esi+0x14]
    let t104: u32 = rd::<u32>(c, 0xa8);
    // 7eea1a fxch st(3)
    f.xch(3);
    // 7eea1c fmul DWORD PTR [edi]
    f.set(0, f.st(0).fmul_m(rd::<f32>(c, 0x124)));
    // 7eea1e faddp st(3),st
    f.set(3, f.st(3).fadd(f.st(0)));
    f.pop();
    // 7eea20 fld DWORD PTR [edi+0x8]
    f.push(x(rd::<f32>(c, 0x12c)));
    // 7eea23 fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7eea25 faddp st(3),st
    f.set(3, f.st(3).fadd(f.st(0)));
    f.pop();
    // 7eea27 fxch st(2)
    f.xch(2);
    // 7eea29 fstp DWORD PTR [esi+0x20]
    let t105 = f.st(0).to_f32();
    wr::<f32>(c, 0xb4, t105);
    f.pop();
    // 7eea2c fstp st(1)
    f.stp(1);
    // 7eea2e fstp st(0)
    f.stp(0);
    // 7eea30 mov eax,DWORD PTR [esi+0x18]
    let t106: u32 = rd::<u32>(c, 0xac);
    // 7eea33 mov ecx,DWORD PTR [esi+0x1c]
    let t107: u32 = rd::<u32>(c, 0xb0);
    // 7eea36 sar edx,0x1f
    let t108 = ((t104 as i32) >> 31) as u32;
    // 7eea39 sar eax,0x1f
    let t109 = ((t106 as i32) >> 31) as u32;
    // 7eea3c inc dl
    let t110 = (t108 & !0xFF) | u32::from((t108 as u8).wrapping_add(1));
    // 7eea3e inc al
    let t111 = (t109 & !0xFF) | u32::from((t109 as u8).wrapping_add(1));
    // 7eea40 sar ecx,0x1f
    let t112 = ((t107 as i32) >> 31) as u32;
    // 7eea43 inc cl
    let t113 = (t112 & !0xFF) | u32::from((t112 as u8).wrapping_add(1));
    // 7eea45 mov BYTE PTR [esi+0x24],dl
    wr::<u8>(c, 0xb8, t110 as u8);
    // 7eea48 mov BYTE PTR [esi+0x25],al
    wr::<u8>(c, 0xb9, t111 as u8);
    // 7eea4b mov BYTE PTR [esi+0x26],cl
    wr::<u8>(c, 0xba, t113 as u8);
    // 7eea4e fld DWORD PTR [ebx]
    f.push(x(rd::<f32>(c, 0x130)));
    // 7eea50 fsub DWORD PTR [edi+0x3c]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x160)));
    // 7eea53 fstp DWORD PTR [esp+0x14]
    let t114 = f.st(0).to_f32();
    loc[20] = t114.to_bits();
    f.pop();
    // 7eea57 fld DWORD PTR [edi+0x10]
    f.push(x(rd::<f32>(c, 0x134)));
    // 7eea5a fsub DWORD PTR [edi+0x40]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x164)));
    // 7eea5d fstp DWORD PTR [esp+0x18]
    let t115 = f.st(0).to_f32();
    loc[21] = t115.to_bits();
    f.pop();
    // 7eea61 fld DWORD PTR [edi+0x14]
    f.push(x(rd::<f32>(c, 0x138)));
    // 7eea64 fsub DWORD PTR [edi+0x44]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x168)));
    // 7eea67 fstp DWORD PTR [esp+0x1c]
    let t116 = f.st(0).to_f32();
    loc[22] = t116.to_bits();
    f.pop();
    // 7eea6b fld DWORD PTR [edi+0x48]
    f.push(x(rd::<f32>(c, 0x16c)));
    // 7eea6e fsub DWORD PTR [edi+0x3c]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x160)));
    // 7eea71 fld DWORD PTR [edi+0x4c]
    f.push(x(rd::<f32>(c, 0x170)));
    // 7eea74 fsub DWORD PTR [edi+0x40]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x164)));
    // 7eea77 fld DWORD PTR [edi+0x50]
    f.push(x(rd::<f32>(c, 0x174)));
    // 7eea7a fsub DWORD PTR [edi+0x44]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x168)));
    // 7eea7d fld DWORD PTR [esp+0x18]
    f.push(x(f32::from_bits(loc[21])));
    // 7eea81 fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7eea83 fld st(2)
    f.push(f.st(2));
    // 7eea85 fmul DWORD PTR [esp+0x1c]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[22])));
    // 7eea89 fsubp st(1),st
    f.set(1, f.st(1).fsub(f.st(0)));
    f.pop();
    // 7eea8b fst DWORD PTR [esp+0x10]
    let t117 = f.st(0).to_f32();
    loc[19] = t117.to_bits();
    // 7eea8f fstp DWORD PTR [esi+0x28]
    let t118 = f.st(0).to_f32();
    wr::<f32>(c, 0xbc, t118);
    f.pop();
    // 7eea92 fld st(2)
    f.push(f.st(2));
    // 7eea94 fmul DWORD PTR [esp+0x1c]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[22])));
    // 7eea98 fxch st(1)
    f.xch(1);
    // 7eea9a fmul DWORD PTR [esp+0x14]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[20])));
    // 7eea9e fsubp st(1),st
    f.set(1, f.st(1).fsub(f.st(0)));
    f.pop();
    // 7eeaa0 fst DWORD PTR [esp+0x48]
    let t119 = f.st(0).to_f32();
    loc[33] = t119.to_bits();
    // 7eeaa4 fstp DWORD PTR [esi+0x2c]
    let t120 = f.st(0).to_f32();
    wr::<f32>(c, 0xc0, t120);
    f.pop();
    // 7eeaa7 fmul DWORD PTR [esp+0x14]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[20])));
    // 7eeaab fxch st(1)
    f.xch(1);
    // 7eeaad fmul DWORD PTR [esp+0x18]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[21])));
    // 7eeab1 fsubp st(1),st
    f.set(1, f.st(1).fsub(f.st(0)));
    f.pop();
    // 7eeab3 fst DWORD PTR [esi+0x30]
    let t121 = f.st(0).to_f32();
    wr::<f32>(c, 0xc4, t121);
    // 7eeab6 fld DWORD PTR [esp+0x48]
    f.push(x(f32::from_bits(loc[33])));
    // 7eeaba fld DWORD PTR [esp+0x10]
    f.push(x(f32::from_bits(loc[19])));
    // 7eeabe fld st(0)
    f.push(f.st(0));
    // 7eeac0 fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7eeac2 fld st(2)
    f.push(f.st(2));
    // 7eeac4 fmul st,st(3)
    f.set(0, f.st(0).fmul(f.st(3)));
    // 7eeac6 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7eeac8 fld st(3)
    f.push(f.st(3));
    // 7eeaca fmul st,st(4)
    f.set(0, f.st(0).fmul(f.st(4)));
    // 7eeacc faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7eeace fstp DWORD PTR [esp+0x48]
    let t122 = f.st(0).to_f32();
    loc[33] = t122.to_bits();
    f.pop();
    // 7eead2 mov edx,DWORD PTR [esp+0x48]
    let t123: u32 = loc[33];
    // 7eead6 fstp st(2)
    f.stp(2);
    // 7eead8 fstp st(1)
    f.stp(1);
    // 7eeada fstp st(0)
    f.stp(0);
    // 7eeadc push edx
    // 7eeadd call 0x7edb90
    f.push(X87::from_f64_exact(inv_sqrt(f32::from_bits(t123))));
    // 7eeae2 fld DWORD PTR [esi+0x28]
    f.push(x(rd::<f32>(c, 0xbc)));
    // 7eeae5 fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7eeae7 fst DWORD PTR [esi+0x28]
    let t124 = f.st(0).to_f32();
    wr::<f32>(c, 0xbc, t124);
    // 7eeaea fld DWORD PTR [esi+0x2c]
    f.push(x(rd::<f32>(c, 0xc0)));
    // 7eeaed fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7eeaef fst DWORD PTR [esi+0x2c]
    let t125 = f.st(0).to_f32();
    wr::<f32>(c, 0xc0, t125);
    // 7eeaf2 fld DWORD PTR [esi+0x30]
    f.push(x(rd::<f32>(c, 0xc4)));
    // 7eeaf5 fmul st,st(3)
    f.set(0, f.st(0).fmul(f.st(3)));
    // 7eeaf7 fst DWORD PTR [esi+0x30]
    let t126 = f.st(0).to_f32();
    wr::<f32>(c, 0xc4, t126);
    // 7eeafa fld DWORD PTR [edi+0x10]
    f.push(x(rd::<f32>(c, 0x134)));
    // 7eeafd fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7eeaff fld DWORD PTR [ebx]
    f.push(x(rd::<f32>(c, 0x130)));
    // 7eeb01 fmul st,st(4)
    f.set(0, f.st(0).fmul(f.st(4)));
    // 7eeb03 mov eax,DWORD PTR [esi+0x28]
    let t127: u32 = rd::<u32>(c, 0xbc);
    // 7eeb06 mov ecx,DWORD PTR [esi+0x2c]
    let t128: u32 = rd::<u32>(c, 0xc0);
    // 7eeb09 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7eeb0b mov edx,DWORD PTR [esi+0x30]
    let t129: u32 = rd::<u32>(c, 0xc4);
    // 7eeb0e sar eax,0x1f
    let t130 = ((t127 as i32) >> 31) as u32;
    // 7eeb11 fld DWORD PTR [edi+0x14]
    f.push(x(rd::<f32>(c, 0x138)));
    // 7eeb14 fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7eeb16 sar ecx,0x1f
    let t131 = ((t128 as i32) >> 31) as u32;
    // 7eeb19 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7eeb1b inc al
    let t132 = (t130 & !0xFF) | u32::from((t130 as u8).wrapping_add(1));
    // 7eeb1d inc cl
    let t133 = (t131 & !0xFF) | u32::from((t131 as u8).wrapping_add(1));
    // 7eeb1f sar edx,0x1f
    let t134 = ((t129 as i32) >> 31) as u32;
    // 7eeb22 fstp DWORD PTR [esi+0x34]
    let t135 = f.st(0).to_f32();
    wr::<f32>(c, 0xc8, t135);
    f.pop();
    // 7eeb25 inc dl
    let t136 = (t134 & !0xFF) | u32::from((t134 as u8).wrapping_add(1));
    // 7eeb27 mov BYTE PTR [esi+0x38],al
    wr::<u8>(c, 0xcc, t132 as u8);
    // 7eeb2a fstp st(0)
    f.stp(0);
    // 7eeb2c fstp st(0)
    f.stp(0);
    // 7eeb2e fstp st(0)
    f.stp(0);
    // 7eeb30 fstp st(0)
    f.stp(0);
    // 7eeb32 mov BYTE PTR [esi+0x39],cl
    wr::<u8>(c, 0xcd, t133 as u8);
    // 7eeb35 mov BYTE PTR [esi+0x3a],dl
    wr::<u8>(c, 0xce, t136 as u8);
    // 7eeb38 fld DWORD PTR [edi+0x30]
    f.push(x(rd::<f32>(c, 0x154)));
    // 7eeb3b fsub DWORD PTR [edi+0x3c]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x160)));
    // 7eeb3e fld DWORD PTR [edi+0x34]
    f.push(x(rd::<f32>(c, 0x158)));
    // 7eeb41 fsub DWORD PTR [edi+0x40]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x164)));
    // 7eeb44 fld DWORD PTR [edi+0x38]
    f.push(x(rd::<f32>(c, 0x15c)));
    // 7eeb47 fsub DWORD PTR [edi+0x44]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x168)));
    // 7eeb4a fld st(1)
    f.push(f.st(1));
    // 7eeb4c fmul DWORD PTR [esp+0x20]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[22])));
    // 7eeb50 fld st(1)
    f.push(f.st(1));
    // 7eeb52 fmul DWORD PTR [esp+0x1c]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[21])));
    // 7eeb56 fsubp st(1),st
    f.set(1, f.st(1).fsub(f.st(0)));
    f.pop();
    // 7eeb58 fst DWORD PTR [esp+0x14]
    let t137 = f.st(0).to_f32();
    loc[19] = t137.to_bits();
    // 7eeb5c fstp DWORD PTR [esi+0x3c]
    let t138 = f.st(0).to_f32();
    wr::<f32>(c, 0xd0, t138);
    f.pop();
    // 7eeb5f fmul DWORD PTR [esp+0x18]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[20])));
    // 7eeb63 fld st(2)
    f.push(f.st(2));
    // 7eeb65 fmul DWORD PTR [esp+0x20]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[22])));
    // 7eeb69 fsubp st(1),st
    f.set(1, f.st(1).fsub(f.st(0)));
    f.pop();
    // 7eeb6b fst DWORD PTR [esp+0x4c]
    let t139 = f.st(0).to_f32();
    loc[33] = t139.to_bits();
    // 7eeb6f fstp DWORD PTR [esi+0x40]
    let t140 = f.st(0).to_f32();
    wr::<f32>(c, 0xd4, t140);
    f.pop();
    // 7eeb72 fxch st(1)
    f.xch(1);
    // 7eeb74 fmul DWORD PTR [esp+0x1c]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[21])));
    // 7eeb78 fld st(1)
    f.push(f.st(1));
    // 7eeb7a fmul DWORD PTR [esp+0x18]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[20])));
    // 7eeb7e fsubp st(1),st
    f.set(1, f.st(1).fsub(f.st(0)));
    f.pop();
    // 7eeb80 fxch st(1)
    f.xch(1);
    // 7eeb82 fstp st(0)
    f.stp(0);
    // 7eeb84 fst DWORD PTR [esi+0x44]
    let t141 = f.st(0).to_f32();
    wr::<f32>(c, 0xd8, t141);
    // 7eeb87 fld DWORD PTR [esp+0x4c]
    f.push(x(f32::from_bits(loc[33])));
    // 7eeb8b fld DWORD PTR [esp+0x14]
    f.push(x(f32::from_bits(loc[19])));
    // 7eeb8f fld st(0)
    f.push(f.st(0));
    // 7eeb91 fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7eeb93 fld st(2)
    f.push(f.st(2));
    // 7eeb95 fmul st,st(3)
    f.set(0, f.st(0).fmul(f.st(3)));
    // 7eeb97 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7eeb99 fld st(3)
    f.push(f.st(3));
    // 7eeb9b fmul st,st(4)
    f.set(0, f.st(0).fmul(f.st(4)));
    // 7eeb9d faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7eeb9f fstp DWORD PTR [esp+0x4c]
    let t142 = f.st(0).to_f32();
    loc[33] = t142.to_bits();
    f.pop();
    // 7eeba3 mov eax,DWORD PTR [esp+0x4c]
    let t143: u32 = loc[33];
    // 7eeba7 fstp st(2)
    f.stp(2);
    // 7eeba9 fstp st(1)
    f.stp(1);
    // 7eebab fstp st(0)
    f.stp(0);
    // 7eebad push eax
    // 7eebae call 0x7edb90
    f.push(X87::from_f64_exact(inv_sqrt(f32::from_bits(t143))));
    // 7eebb3 fld DWORD PTR [esi+0x3c]
    f.push(x(rd::<f32>(c, 0xd0)));
    // 7eebb6 fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7eebb8 fst DWORD PTR [esi+0x3c]
    let t144 = f.st(0).to_f32();
    wr::<f32>(c, 0xd0, t144);
    // 7eebbb fld DWORD PTR [esi+0x40]
    f.push(x(rd::<f32>(c, 0xd4)));
    // 7eebbe fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7eebc0 fst DWORD PTR [esi+0x40]
    let t145 = f.st(0).to_f32();
    wr::<f32>(c, 0xd4, t145);
    // 7eebc3 fld DWORD PTR [esi+0x44]
    f.push(x(rd::<f32>(c, 0xd8)));
    // 7eebc6 fmul st,st(3)
    f.set(0, f.st(0).fmul(f.st(3)));
    // 7eebc8 fst DWORD PTR [esi+0x44]
    let t146 = f.st(0).to_f32();
    wr::<f32>(c, 0xd8, t146);
    // 7eebcb fld DWORD PTR [ebx]
    f.push(x(rd::<f32>(c, 0x130)));
    // 7eebcd fmul st,st(3)
    f.set(0, f.st(0).fmul(f.st(3)));
    // 7eebcf fld DWORD PTR [edi+0x10]
    f.push(x(rd::<f32>(c, 0x134)));
    // 7eebd2 fmul st,st(3)
    f.set(0, f.st(0).fmul(f.st(3)));
    // 7eebd4 mov ecx,DWORD PTR [esi+0x3c]
    let t147: u32 = rd::<u32>(c, 0xd0);
    // 7eebd7 mov edx,DWORD PTR [esi+0x40]
    let t148: u32 = rd::<u32>(c, 0xd4);
    // 7eebda faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7eebdc mov eax,DWORD PTR [esi+0x44]
    let t149: u32 = rd::<u32>(c, 0xd8);
    // 7eebdf sar ecx,0x1f
    let t150 = ((t147 as i32) >> 31) as u32;
    // 7eebe2 fld DWORD PTR [edi+0x14]
    f.push(x(rd::<f32>(c, 0x138)));
    // 7eebe5 fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7eebe7 sar edx,0x1f
    let t151 = ((t148 as i32) >> 31) as u32;
    // 7eebea faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7eebec inc cl
    let t152 = (t150 & !0xFF) | u32::from((t150 as u8).wrapping_add(1));
    // 7eebee inc dl
    let t153 = (t151 & !0xFF) | u32::from((t151 as u8).wrapping_add(1));
    // 7eebf0 sar eax,0x1f
    let t154 = ((t149 as i32) >> 31) as u32;
    // 7eebf3 fstp DWORD PTR [esi+0x48]
    let t155 = f.st(0).to_f32();
    wr::<f32>(c, 0xdc, t155);
    f.pop();
    // 7eebf6 inc al
    let t156 = (t154 & !0xFF) | u32::from((t154 as u8).wrapping_add(1));
    // 7eebf8 mov BYTE PTR [esi+0x4c],cl
    wr::<u8>(c, 0xe0, t152 as u8);
    // 7eebfb fstp st(0)
    f.stp(0);
    // 7eebfd fstp st(0)
    f.stp(0);
    // 7eebff fstp st(0)
    f.stp(0);
    // 7eec01 fstp st(0)
    f.stp(0);
    // 7eec03 mov BYTE PTR [esi+0x4d],dl
    wr::<u8>(c, 0xe1, t153 as u8);
    // 7eec06 mov BYTE PTR [esi+0x4e],al
    wr::<u8>(c, 0xe2, t156 as u8);
    // 7eec09 fld DWORD PTR [edi+0x24]
    f.push(x(rd::<f32>(c, 0x148)));
    // 7eec0c fsub DWORD PTR [edi+0x54]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x178)));
    // 7eec0f fstp DWORD PTR [esp+0x1c]
    let t157 = f.st(0).to_f32();
    loc[20] = t157.to_bits();
    f.pop();
    // 7eec13 fld DWORD PTR [edi+0x28]
    f.push(x(rd::<f32>(c, 0x14c)));
    // 7eec16 fsub DWORD PTR [edi+0x58]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x17c)));
    // 7eec19 fstp DWORD PTR [esp+0x20]
    let t158 = f.st(0).to_f32();
    loc[21] = t158.to_bits();
    f.pop();
    // 7eec1d fld DWORD PTR [edi+0x2c]
    f.push(x(rd::<f32>(c, 0x150)));
    // 7eec20 fsub DWORD PTR [edi+0x5c]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x180)));
    // 7eec23 fstp DWORD PTR [esp+0x24]
    let t159 = f.st(0).to_f32();
    loc[22] = t159.to_bits();
    f.pop();
    // 7eec27 fld DWORD PTR [edi+0x30]
    f.push(x(rd::<f32>(c, 0x154)));
    // 7eec2a fsub DWORD PTR [edi+0x54]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x178)));
    // 7eec2d fld DWORD PTR [edi+0x34]
    f.push(x(rd::<f32>(c, 0x158)));
    // 7eec30 fsub DWORD PTR [edi+0x58]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x17c)));
    // 7eec33 fld DWORD PTR [edi+0x38]
    f.push(x(rd::<f32>(c, 0x15c)));
    // 7eec36 fsub DWORD PTR [edi+0x5c]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x180)));
    // 7eec39 fld DWORD PTR [esp+0x20]
    f.push(x(f32::from_bits(loc[21])));
    // 7eec3d fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7eec3f fld st(2)
    f.push(f.st(2));
    // 7eec41 fmul DWORD PTR [esp+0x24]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[22])));
    // 7eec45 fsubp st(1),st
    f.set(1, f.st(1).fsub(f.st(0)));
    f.pop();
    // 7eec47 fst DWORD PTR [esp+0x18]
    let t160 = f.st(0).to_f32();
    loc[19] = t160.to_bits();
    // 7eec4b fstp DWORD PTR [esi+0x50]
    let t161 = f.st(0).to_f32();
    wr::<f32>(c, 0xe4, t161);
    f.pop();
    // 7eec4e fld st(2)
    f.push(f.st(2));
    // 7eec50 fmul DWORD PTR [esp+0x24]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[22])));
    // 7eec54 fxch st(1)
    f.xch(1);
    // 7eec56 fmul DWORD PTR [esp+0x1c]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[20])));
    // 7eec5a fsubp st(1),st
    f.set(1, f.st(1).fsub(f.st(0)));
    f.pop();
    // 7eec5c fst DWORD PTR [esp+0x50]
    let t162 = f.st(0).to_f32();
    loc[33] = t162.to_bits();
    // 7eec60 fstp DWORD PTR [esi+0x54]
    let t163 = f.st(0).to_f32();
    wr::<f32>(c, 0xe8, t163);
    f.pop();
    // 7eec63 fmul DWORD PTR [esp+0x1c]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[20])));
    // 7eec67 fxch st(1)
    f.xch(1);
    // 7eec69 fmul DWORD PTR [esp+0x20]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[21])));
    // 7eec6d fsubp st(1),st
    f.set(1, f.st(1).fsub(f.st(0)));
    f.pop();
    // 7eec6f fst DWORD PTR [esi+0x58]
    let t164 = f.st(0).to_f32();
    wr::<f32>(c, 0xec, t164);
    // 7eec72 fld DWORD PTR [esp+0x50]
    f.push(x(f32::from_bits(loc[33])));
    // 7eec76 fld DWORD PTR [esp+0x18]
    f.push(x(f32::from_bits(loc[19])));
    // 7eec7a fld st(0)
    f.push(f.st(0));
    // 7eec7c fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7eec7e fld st(2)
    f.push(f.st(2));
    // 7eec80 fmul st,st(3)
    f.set(0, f.st(0).fmul(f.st(3)));
    // 7eec82 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7eec84 fld st(3)
    f.push(f.st(3));
    // 7eec86 fmul st,st(4)
    f.set(0, f.st(0).fmul(f.st(4)));
    // 7eec88 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7eec8a fstp DWORD PTR [esp+0x50]
    let t165 = f.st(0).to_f32();
    loc[33] = t165.to_bits();
    f.pop();
    // 7eec8e mov ecx,DWORD PTR [esp+0x50]
    let t166: u32 = loc[33];
    // 7eec92 fstp st(2)
    f.stp(2);
    // 7eec94 fstp st(1)
    f.stp(1);
    // 7eec96 fstp st(0)
    f.stp(0);
    // 7eec98 push ecx
    // 7eec99 call 0x7edb90
    f.push(X87::from_f64_exact(inv_sqrt(f32::from_bits(t166))));
    // 7eec9e fld DWORD PTR [esi+0x50]
    f.push(x(rd::<f32>(c, 0xe4)));
    // 7eeca1 fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7eeca3 fst DWORD PTR [esi+0x50]
    let t167 = f.st(0).to_f32();
    wr::<f32>(c, 0xe4, t167);
    // 7eeca6 fld DWORD PTR [esi+0x54]
    f.push(x(rd::<f32>(c, 0xe8)));
    // 7eeca9 fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7eecab fst DWORD PTR [esi+0x54]
    let t168 = f.st(0).to_f32();
    wr::<f32>(c, 0xe8, t168);
    // 7eecae fld DWORD PTR [esi+0x58]
    f.push(x(rd::<f32>(c, 0xec)));
    // 7eecb1 fmul st,st(3)
    f.set(0, f.st(0).fmul(f.st(3)));
    // 7eecb3 fst DWORD PTR [esi+0x58]
    let t169 = f.st(0).to_f32();
    wr::<f32>(c, 0xec, t169);
    // 7eecb6 fld DWORD PTR [edi+0x28]
    f.push(x(rd::<f32>(c, 0x14c)));
    // 7eecb9 fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7eecbb mov edx,DWORD PTR [esi+0x50]
    let t170: u32 = rd::<u32>(c, 0xe4);
    // 7eecbe mov eax,DWORD PTR [esi+0x54]
    let t171: u32 = rd::<u32>(c, 0xe8);
    // 7eecc1 fld DWORD PTR [edi+0x24]
    f.push(x(rd::<f32>(c, 0x148)));
    // 7eecc4 fmul st,st(4)
    f.set(0, f.st(0).fmul(f.st(4)));
    // 7eecc6 mov ecx,DWORD PTR [esi+0x58]
    let t172: u32 = rd::<u32>(c, 0xec);
    // 7eecc9 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7eeccb sar edx,0x1f
    let t173 = ((t170 as i32) >> 31) as u32;
    // 7eecce sar eax,0x1f
    let t174 = ((t171 as i32) >> 31) as u32;
    // 7eecd1 fld DWORD PTR [edi+0x2c]
    f.push(x(rd::<f32>(c, 0x150)));
    // 7eecd4 fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7eecd6 inc dl
    let t175 = (t173 & !0xFF) | u32::from((t173 as u8).wrapping_add(1));
    // 7eecd8 inc al
    let t176 = (t174 & !0xFF) | u32::from((t174 as u8).wrapping_add(1));
    // 7eecda faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7eecdc sar ecx,0x1f
    let t177 = ((t172 as i32) >> 31) as u32;
    // 7eecdf inc cl
    let t178 = (t177 & !0xFF) | u32::from((t177 as u8).wrapping_add(1));
    // 7eece1 mov BYTE PTR [esi+0x60],dl
    wr::<u8>(c, 0xf4, t175 as u8);
    // 7eece4 fstp DWORD PTR [esi+0x5c]
    let t179 = f.st(0).to_f32();
    wr::<f32>(c, 0xf0, t179);
    f.pop();
    // 7eece7 mov BYTE PTR [esi+0x61],al
    wr::<u8>(c, 0xf5, t176 as u8);
    // 7eecea mov BYTE PTR [esi+0x62],cl
    wr::<u8>(c, 0xf6, t178 as u8);
    // 7eeced fstp st(0)
    f.stp(0);
    // 7eecef fstp st(0)
    f.stp(0);
    // 7eecf1 fstp st(0)
    f.stp(0);
    // 7eecf3 fstp st(0)
    f.stp(0);
    // 7eecf5 fld DWORD PTR [edi+0x48]
    f.push(x(rd::<f32>(c, 0x16c)));
    // 7eecf8 fsub DWORD PTR [edi+0x54]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x178)));
    // 7eecfb fld DWORD PTR [edi+0x4c]
    f.push(x(rd::<f32>(c, 0x170)));
    // 7eecfe fsub DWORD PTR [edi+0x58]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x17c)));
    // 7eed01 fld DWORD PTR [edi+0x50]
    f.push(x(rd::<f32>(c, 0x174)));
    // 7eed04 fsub DWORD PTR [edi+0x5c]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x180)));
    // 7eed07 fld st(1)
    f.push(f.st(1));
    // 7eed09 fmul DWORD PTR [esp+0x28]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[22])));
    // 7eed0d fld st(1)
    f.push(f.st(1));
    // 7eed0f fmul DWORD PTR [esp+0x24]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[21])));
    // 7eed13 fsubp st(1),st
    f.set(1, f.st(1).fsub(f.st(0)));
    f.pop();
    // 7eed15 fst DWORD PTR [esp+0x1c]
    let t180 = f.st(0).to_f32();
    loc[19] = t180.to_bits();
    // 7eed19 fstp DWORD PTR [esi+0x64]
    let t181 = f.st(0).to_f32();
    wr::<f32>(c, 0xf8, t181);
    f.pop();
    // 7eed1c fmul DWORD PTR [esp+0x20]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[20])));
    // 7eed20 fld st(2)
    f.push(f.st(2));
    // 7eed22 fmul DWORD PTR [esp+0x28]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[22])));
    // 7eed26 fsubp st(1),st
    f.set(1, f.st(1).fsub(f.st(0)));
    f.pop();
    // 7eed28 fst DWORD PTR [esp+0x54]
    let t182 = f.st(0).to_f32();
    loc[33] = t182.to_bits();
    // 7eed2c fstp DWORD PTR [esi+0x68]
    let t183 = f.st(0).to_f32();
    wr::<f32>(c, 0xfc, t183);
    f.pop();
    // 7eed2f fxch st(1)
    f.xch(1);
    // 7eed31 fmul DWORD PTR [esp+0x24]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[21])));
    // 7eed35 fld st(1)
    f.push(f.st(1));
    // 7eed37 fmul DWORD PTR [esp+0x20]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[20])));
    // 7eed3b fsubp st(1),st
    f.set(1, f.st(1).fsub(f.st(0)));
    f.pop();
    // 7eed3d fxch st(1)
    f.xch(1);
    // 7eed3f fstp st(0)
    f.stp(0);
    // 7eed41 fst DWORD PTR [esi+0x6c]
    let t184 = f.st(0).to_f32();
    wr::<f32>(c, 0x100, t184);
    // 7eed44 fld DWORD PTR [esp+0x54]
    f.push(x(f32::from_bits(loc[33])));
    // 7eed48 fld DWORD PTR [esp+0x1c]
    f.push(x(f32::from_bits(loc[19])));
    // 7eed4c fld st(0)
    f.push(f.st(0));
    // 7eed4e fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7eed50 fld st(2)
    f.push(f.st(2));
    // 7eed52 fmul st,st(3)
    f.set(0, f.st(0).fmul(f.st(3)));
    // 7eed54 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7eed56 fld st(3)
    f.push(f.st(3));
    // 7eed58 fmul st,st(4)
    f.set(0, f.st(0).fmul(f.st(4)));
    // 7eed5a faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7eed5c fstp DWORD PTR [esp+0x54]
    let t185 = f.st(0).to_f32();
    loc[33] = t185.to_bits();
    f.pop();
    // 7eed60 mov edx,DWORD PTR [esp+0x54]
    let t186: u32 = loc[33];
    // 7eed64 fstp st(2)
    f.stp(2);
    // 7eed66 fstp st(1)
    f.stp(1);
    // 7eed68 fstp st(0)
    f.stp(0);
    // 7eed6a push edx
    // 7eed6b call 0x7edb90
    f.push(X87::from_f64_exact(inv_sqrt(f32::from_bits(t186))));
    // 7eed70 fld DWORD PTR [esi+0x64]
    f.push(x(rd::<f32>(c, 0xf8)));
    // 7eed73 fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7eed75 fst DWORD PTR [esi+0x64]
    let t187 = f.st(0).to_f32();
    wr::<f32>(c, 0xf8, t187);
    // 7eed78 fld DWORD PTR [esi+0x68]
    f.push(x(rd::<f32>(c, 0xfc)));
    // 7eed7b fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7eed7d fst DWORD PTR [esi+0x68]
    let t188 = f.st(0).to_f32();
    wr::<f32>(c, 0xfc, t188);
    // 7eed80 fld DWORD PTR [esi+0x6c]
    f.push(x(rd::<f32>(c, 0x100)));
    // 7eed83 fmul st,st(3)
    f.set(0, f.st(0).fmul(f.st(3)));
    // 7eed85 fst DWORD PTR [esi+0x6c]
    let t189 = f.st(0).to_f32();
    wr::<f32>(c, 0x100, t189);
    // 7eed88 fld DWORD PTR [edi+0x28]
    f.push(x(rd::<f32>(c, 0x14c)));
    // 7eed8b fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7eed8d fld DWORD PTR [edi+0x24]
    f.push(x(rd::<f32>(c, 0x148)));
    // 7eed90 fmul st,st(4)
    f.set(0, f.st(0).fmul(f.st(4)));
    // 7eed92 add esp,0x10
    // 7eed95 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7eed97 fld DWORD PTR [edi+0x2c]
    f.push(x(rd::<f32>(c, 0x150)));
    // 7eed9a fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7eed9c faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7eed9e fstp DWORD PTR [esi+0x70]
    let t190 = f.st(0).to_f32();
    wr::<f32>(c, 0x104, t190);
    f.pop();
    // 7eeda1 fstp st(0)
    f.stp(0);
    // 7eeda3 fstp st(0)
    f.stp(0);
    // 7eeda5 fstp st(0)
    f.stp(0);
    // 7eeda7 mov eax,DWORD PTR [esi+0x64]
    let t191: u32 = rd::<u32>(c, 0xf8);
    // 7eedaa mov ecx,DWORD PTR [esi+0x68]
    let t192: u32 = rd::<u32>(c, 0xfc);
    // 7eedad mov edx,DWORD PTR [esi+0x6c]
    let t193: u32 = rd::<u32>(c, 0x100);
    // 7eedb0 pop edi
    // 7eedb1 sar eax,0x1f
    let t194 = ((t191 as i32) >> 31) as u32;
    // 7eedb4 sar ecx,0x1f
    let t195 = ((t192 as i32) >> 31) as u32;
    // 7eedb7 inc al
    let t196 = (t194 & !0xFF) | u32::from((t194 as u8).wrapping_add(1));
    // 7eedb9 inc cl
    let t197 = (t195 & !0xFF) | u32::from((t195 as u8).wrapping_add(1));
    // 7eedbb sar edx,0x1f
    let t198 = ((t193 as i32) >> 31) as u32;
    // 7eedbe inc dl
    let t199 = (t198 & !0xFF) | u32::from((t198 as u8).wrapping_add(1));
    // 7eedc0 mov BYTE PTR [esi+0x74],al
    wr::<u8>(c, 0x108, t196 as u8);
    // 7eedc3 mov BYTE PTR [esi+0x75],cl
    wr::<u8>(c, 0x109, t197 as u8);
    // 7eedc6 mov BYTE PTR [esi+0x76],dl
    wr::<u8>(c, 0x10a, t199 as u8);
    // 7eedc9 pop esi
    // 7eedca pop ebp
    // 7eedcb fstp st(0)
    f.stp(0);
    // 7eedcd pop ebx
    // 7eedce add esp,0x34
    // 7eedd1 ret 
    debug_assert!(f.n == 0);
}

/// `_rwCameraBuildFrustumBBox` (0x7EEDE0, catalogue `FUN_007eede0`), the parallel-projection
/// builder: the 8 corners in camera space from the view window, view offset and near/far distances,
/// transformed in place by `RwV3dTransformPoints(corners, corners, 8, LTM)`, then the 6 planes as in
/// the perspective builder (far/near from `at`, sides from cross products normalised with
/// `_rwInvSqrt`, closest-corner bytes from the sign bits). Transcribed instruction by instruction like
/// [`rw_camera_build_persp_clip_planes`].
#[hook(addr = 0x7EEDE0, conv = cdecl, name = "FUN_007eede0", expect = "83 EC 1C 8B 44 24 20")]
#[allow(unused_variables, clippy::all)]
pub fn rw_camera_build_parallel_clip_planes(c: GPtr<u8>) {
    let mut f = Fpu::new();
    let mut loc = [0u32; 40];
    // 7eede0 sub esp,0x1c
    // 7eede3 mov eax,DWORD PTR [esp+0x20]
    // 7eede7 push ebx
    // 7eede8 push esi
    // 7eede9 push edi
    // 7eedea fld DWORD PTR [eax+0x68]
    f.push(x(rd::<f32>(c, 0x68)));
    // 7eeded fld DWORD PTR [eax+0x80]
    f.push(x(rd::<f32>(c, 0x80)));
    // 7eedf3 fld DWORD PTR [eax+0x84]
    f.push(x(rd::<f32>(c, 0x84)));
    // 7eedf9 mov ebx,DWORD PTR [eax+0x4]
    let t1 = rd::<u32>(c, 4);
    // 7eedfc mov ecx,DWORD PTR [eax+0x6c]
    let t2: u32 = rd::<u32>(c, 0x6c);
    // 7eedff fst DWORD PTR [esp+0x14]
    let t3 = f.st(0).to_f32();
    loc[27] = t3.to_bits();
    // 7eee03 fld DWORD PTR [eax+0x78]
    f.push(x(rd::<f32>(c, 0x78)));
    // 7eee06 mov edx,DWORD PTR [eax+0x7c]
    let t4: u32 = rd::<u32>(c, 0x7c);
    // 7eee09 lea edi,[eax+0x124]
    // 7eee0f fchs 
    f.set(0, f.st(0).fchs());
    // 7eee11 fstp DWORD PTR [esp+0xc]
    let t5 = f.st(0).to_f32();
    loc[25] = t5.to_bits();
    f.pop();
    // 7eee15 fld st(1)
    f.push(f.st(1));
    // 7eee17 mov DWORD PTR [esp+0x18],edx
    loc[28] = t4;
    // 7eee1b mov DWORD PTR [esp+0x2c],ecx
    loc[33] = t2;
    // 7eee1f fstp DWORD PTR [edi+0x2c]
    let t6 = f.st(0).to_f32();
    wr::<f32>(c, 0x150, t6);
    f.pop();
    // 7eee22 fld st(1)
    f.push(f.st(1));
    // 7eee24 add ebx,0x50
    let m = p8(t1).byte_add(0x50);
    // 7eee27 lea esi,[eax+0x94]
    // 7eee2d fstp DWORD PTR [edi+0x20]
    let t7 = f.st(0).to_f32();
    wr::<f32>(c, 0x144, t7);
    f.pop();
    // 7eee30 fld st(1)
    f.push(f.st(1));
    // 7eee32 push ebx
    // 7eee33 push 0x8
    // 7eee35 fstp DWORD PTR [edi+0x14]
    let t8 = f.st(0).to_f32();
    wr::<f32>(c, 0x138, t8);
    f.pop();
    // 7eee38 fld st(1)
    f.push(f.st(1));
    // 7eee3a push edi
    // 7eee3b push edi
    // 7eee3c fstp DWORD PTR [edi+0x8]
    let t9 = f.st(0).to_f32();
    wr::<f32>(c, 0x12c, t9);
    f.pop();
    // 7eee3f fst DWORD PTR [edi+0x5c]
    let t10 = f.st(0).to_f32();
    wr::<f32>(c, 0x180, t10);
    // 7eee42 fst DWORD PTR [edi+0x50]
    let t11 = f.st(0).to_f32();
    wr::<f32>(c, 0x174, t11);
    // 7eee45 fst DWORD PTR [edi+0x44]
    let t12 = f.st(0).to_f32();
    wr::<f32>(c, 0x168, t12);
    // 7eee48 fstp DWORD PTR [edi+0x38]
    let t13 = f.st(0).to_f32();
    wr::<f32>(c, 0x15c, t13);
    f.pop();
    // 7eee4b fld DWORD PTR ds:0x858624
    f.push(x(crate::common::g::<f32>(0x858624).read()));
    // 7eee51 fsub st,st(1)
    f.set(0, f.st(0).fsub(f.st(1)));
    // 7eee53 fstp DWORD PTR [esp+0x20]
    let t14 = f.st(0).to_f32();
    loc[26] = t14.to_bits();
    f.pop();
    // 7eee57 fstp st(0)
    f.stp(0);
    // 7eee59 fld DWORD PTR [esp+0x20]
    f.push(x(f32::from_bits(loc[26])));
    // 7eee5d fmul DWORD PTR [esp+0x1c]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[25])));
    // 7eee61 fld st(0)
    f.push(f.st(0));
    // 7eee63 fadd st,st(2)
    f.set(0, f.st(0).fadd(f.st(2)));
    // 7eee65 fst DWORD PTR [edi+0x24]
    let t15 = f.st(0).to_f32();
    wr::<f32>(c, 0x148, t15);
    // 7eee68 fstp DWORD PTR [edi]
    let t16 = f.st(0).to_f32();
    wr::<f32>(c, 0x124, t16);
    f.pop();
    // 7eee6a fsub st,st(1)
    f.set(0, f.st(0).fsub(f.st(1)));
    // 7eee6c fst DWORD PTR [edi+0x18]
    let t17 = f.st(0).to_f32();
    wr::<f32>(c, 0x13c, t17);
    // 7eee6f fstp DWORD PTR [edi+0xc]
    let t18 = f.st(0).to_f32();
    wr::<f32>(c, 0x130, t18);
    f.pop();
    // 7eee72 fld DWORD PTR ds:0x858624
    f.push(x(crate::common::g::<f32>(0x858624).read()));
    // 7eee78 fsub DWORD PTR [esp+0x24]
    f.set(0, f.st(0).fsub_m(f32::from_bits(loc[27])));
    // 7eee7c fst DWORD PTR [esp+0x24]
    let t19 = f.st(0).to_f32();
    loc[27] = t19.to_bits();
    // 7eee80 fmul DWORD PTR [esp+0x1c]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[25])));
    // 7eee84 fld st(0)
    f.push(f.st(0));
    // 7eee86 fadd st,st(2)
    f.set(0, f.st(0).fadd(f.st(2)));
    // 7eee88 fst DWORD PTR [edi+0x54]
    let t20 = f.st(0).to_f32();
    wr::<f32>(c, 0x178, t20);
    // 7eee8b fstp DWORD PTR [edi+0x30]
    let t21 = f.st(0).to_f32();
    wr::<f32>(c, 0x154, t21);
    f.pop();
    // 7eee8e fsubrp st(1),st
    f.set(1, f.st(0).fsub(f.st(1)));
    f.pop();
    // 7eee90 fst DWORD PTR [edi+0x48]
    let t22 = f.st(0).to_f32();
    wr::<f32>(c, 0x16c, t22);
    // 7eee93 fstp DWORD PTR [edi+0x3c]
    let t23 = f.st(0).to_f32();
    wr::<f32>(c, 0x160, t23);
    f.pop();
    // 7eee96 fld DWORD PTR [esp+0x20]
    f.push(x(f32::from_bits(loc[26])));
    // 7eee9a fmul DWORD PTR [esp+0x28]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[28])));
    // 7eee9e fld DWORD PTR [esp+0x3c]
    f.push(x(f32::from_bits(loc[33])));
    // 7eeea2 fadd st,st(1)
    f.set(0, f.st(0).fadd(f.st(1)));
    // 7eeea4 fst DWORD PTR [edi+0x10]
    let t24 = f.st(0).to_f32();
    wr::<f32>(c, 0x134, t24);
    // 7eeea7 fstp DWORD PTR [edi+0x4]
    let t25 = f.st(0).to_f32();
    wr::<f32>(c, 0x128, t25);
    f.pop();
    // 7eeeaa fsub DWORD PTR [esp+0x3c]
    f.set(0, f.st(0).fsub_m(f32::from_bits(loc[33])));
    // 7eeeae fst DWORD PTR [edi+0x28]
    let t26 = f.st(0).to_f32();
    wr::<f32>(c, 0x14c, t26);
    // 7eeeb1 fstp DWORD PTR [edi+0x1c]
    let t27 = f.st(0).to_f32();
    wr::<f32>(c, 0x140, t27);
    f.pop();
    // 7eeeb4 fld DWORD PTR [esp+0x24]
    f.push(x(f32::from_bits(loc[27])));
    // 7eeeb8 fmul DWORD PTR [esp+0x28]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[28])));
    // 7eeebc fld DWORD PTR [esp+0x3c]
    f.push(x(f32::from_bits(loc[33])));
    // 7eeec0 fadd st,st(1)
    f.set(0, f.st(0).fadd(f.st(1)));
    // 7eeec2 fst DWORD PTR [edi+0x40]
    let t28 = f.st(0).to_f32();
    wr::<f32>(c, 0x164, t28);
    // 7eeec5 fstp DWORD PTR [edi+0x34]
    let t29 = f.st(0).to_f32();
    wr::<f32>(c, 0x158, t29);
    f.pop();
    // 7eeec8 fsub DWORD PTR [esp+0x3c]
    f.set(0, f.st(0).fsub_m(f32::from_bits(loc[33])));
    // 7eeecc fst DWORD PTR [edi+0x58]
    let t30 = f.st(0).to_f32();
    wr::<f32>(c, 0x17c, t30);
    // 7eeecf fstp DWORD PTR [edi+0x4c]
    let t31 = f.st(0).to_f32();
    wr::<f32>(c, 0x170, t31);
    f.pop();
    // 7eeed2 call 0x7edd90
    transform_points(c.byte_add(0x124).cast(), c.byte_add(0x124).cast(), 8, m);
    // 7eeed7 add ebx,0x20
    // 7eeeda mov eax,esi
    // 7eeedc mov ecx,DWORD PTR [ebx]
    let t32: u32 = rd::<u32>(m, 0x20);
    // 7eeede mov DWORD PTR [eax],ecx
    wr::<u32>(c, 0x94, t32);
    // 7eeee0 mov edx,DWORD PTR [ebx+0x4]
    let t33: u32 = rd::<u32>(m, 0x24);
    // 7eeee3 mov DWORD PTR [eax+0x4],edx
    wr::<u32>(c, 0x98, t33);
    // 7eeee6 mov ecx,DWORD PTR [ebx+0x8]
    let t34: u32 = rd::<u32>(m, 0x28);
    // 7eeee9 mov DWORD PTR [eax+0x8],ecx
    wr::<u32>(c, 0x9c, t34);
    // 7eeeec mov edx,DWORD PTR [esi]
    let t35: u32 = rd::<u32>(c, 0x94);
    // 7eeeee fld DWORD PTR [esi+0x4]
    f.push(x(rd::<f32>(c, 0x98)));
    // 7eeef1 fmul DWORD PTR [edi+0x34]
    f.set(0, f.st(0).fmul_m(rd::<f32>(c, 0x158)));
    // 7eeef4 mov eax,DWORD PTR [esi+0x4]
    let t36: u32 = rd::<u32>(c, 0x98);
    // 7eeef7 mov ecx,DWORD PTR [esi+0x8]
    let t37: u32 = rd::<u32>(c, 0x9c);
    // 7eeefa sar edx,0x1f
    let t38 = ((t35 as i32) >> 31) as u32;
    // 7eeefd fld DWORD PTR [edi+0x30]
    f.push(x(rd::<f32>(c, 0x154)));
    // 7eef00 fmul DWORD PTR [esi]
    f.set(0, f.st(0).fmul_m(rd::<f32>(c, 0x94)));
    // 7eef02 sar eax,0x1f
    let t39 = ((t36 as i32) >> 31) as u32;
    // 7eef05 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7eef07 inc dl
    let t40 = (t38 & !0xFF) | u32::from((t38 as u8).wrapping_add(1));
    // 7eef09 inc al
    let t41 = (t39 & !0xFF) | u32::from((t39 as u8).wrapping_add(1));
    // 7eef0b sar ecx,0x1f
    let t42 = ((t37 as i32) >> 31) as u32;
    // 7eef0e fld DWORD PTR [edi+0x38]
    f.push(x(rd::<f32>(c, 0x15c)));
    // 7eef11 fmul DWORD PTR [esi+0x8]
    f.set(0, f.st(0).fmul_m(rd::<f32>(c, 0x9c)));
    // 7eef14 inc cl
    let t43 = (t42 & !0xFF) | u32::from((t42 as u8).wrapping_add(1));
    // 7eef16 mov BYTE PTR [esi+0x10],dl
    wr::<u8>(c, 0xa4, t40 as u8);
    // 7eef19 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7eef1b mov BYTE PTR [esi+0x11],al
    wr::<u8>(c, 0xa5, t41 as u8);
    // 7eef1e mov BYTE PTR [esi+0x12],cl
    wr::<u8>(c, 0xa6, t43 as u8);
    // 7eef21 fstp DWORD PTR [esi+0xc]
    let t44 = f.st(0).to_f32();
    wr::<f32>(c, 0xa0, t44);
    f.pop();
    // 7eef24 fld DWORD PTR [esi]
    f.push(x(rd::<f32>(c, 0x94)));
    // 7eef26 fchs 
    f.set(0, f.st(0).fchs());
    // 7eef28 fst DWORD PTR [esi+0x14]
    let t45 = f.st(0).to_f32();
    wr::<f32>(c, 0xa8, t45);
    // 7eef2b fld DWORD PTR [esi+0x4]
    f.push(x(rd::<f32>(c, 0x98)));
    // 7eef2e fchs 
    f.set(0, f.st(0).fchs());
    // 7eef30 fst DWORD PTR [esi+0x18]
    let t46 = f.st(0).to_f32();
    wr::<f32>(c, 0xac, t46);
    // 7eef33 fld DWORD PTR [esi+0x8]
    f.push(x(rd::<f32>(c, 0x9c)));
    // 7eef36 fchs 
    f.set(0, f.st(0).fchs());
    // 7eef38 fst DWORD PTR [esi+0x1c]
    let t47 = f.st(0).to_f32();
    wr::<f32>(c, 0xb0, t47);
    // 7eef3b fld DWORD PTR [edi+0x4]
    f.push(x(rd::<f32>(c, 0x128)));
    // 7eef3e fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7eef40 mov edx,DWORD PTR [esi+0x14]
    let t48: u32 = rd::<u32>(c, 0xa8);
    // 7eef43 mov eax,DWORD PTR [esi+0x18]
    let t49: u32 = rd::<u32>(c, 0xac);
    // 7eef46 sar edx,0x1f
    let t50 = ((t48 as i32) >> 31) as u32;
    // 7eef49 fxch st(3)
    f.xch(3);
    // 7eef4b fmul DWORD PTR [edi]
    f.set(0, f.st(0).fmul_m(rd::<f32>(c, 0x124)));
    // 7eef4d inc dl
    let t51 = (t50 & !0xFF) | u32::from((t50 as u8).wrapping_add(1));
    // 7eef4f faddp st(3),st
    f.set(3, f.st(3).fadd(f.st(0)));
    f.pop();
    // 7eef51 sar eax,0x1f
    let t52 = ((t49 as i32) >> 31) as u32;
    // 7eef54 fld DWORD PTR [edi+0x8]
    f.push(x(rd::<f32>(c, 0x12c)));
    // 7eef57 fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7eef59 mov BYTE PTR [esi+0x24],dl
    wr::<u8>(c, 0xb8, t51 as u8);
    // 7eef5c inc al
    let t53 = (t52 & !0xFF) | u32::from((t52 as u8).wrapping_add(1));
    // 7eef5e faddp st(3),st
    f.set(3, f.st(3).fadd(f.st(0)));
    f.pop();
    // 7eef60 fxch st(2)
    f.xch(2);
    // 7eef62 fstp DWORD PTR [esi+0x20]
    let t54 = f.st(0).to_f32();
    wr::<f32>(c, 0xb4, t54);
    f.pop();
    // 7eef65 fstp st(1)
    f.stp(1);
    // 7eef67 fstp st(0)
    f.stp(0);
    // 7eef69 mov ecx,DWORD PTR [esi+0x1c]
    let t55: u32 = rd::<u32>(c, 0xb0);
    // 7eef6c mov BYTE PTR [esi+0x25],al
    wr::<u8>(c, 0xb9, t53 as u8);
    // 7eef6f sar ecx,0x1f
    let t56 = ((t55 as i32) >> 31) as u32;
    // 7eef72 inc cl
    let t57 = (t56 & !0xFF) | u32::from((t56 as u8).wrapping_add(1));
    // 7eef74 mov BYTE PTR [esi+0x26],cl
    wr::<u8>(c, 0xba, t57 as u8);
    // 7eef77 fld DWORD PTR [edi+0xc]
    f.push(x(rd::<f32>(c, 0x130)));
    // 7eef7a fsub DWORD PTR [edi+0x3c]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x160)));
    // 7eef7d fstp DWORD PTR [esp+0x2c]
    let t58 = f.st(0).to_f32();
    loc[29] = t58.to_bits();
    f.pop();
    // 7eef81 fld DWORD PTR [edi+0x10]
    f.push(x(rd::<f32>(c, 0x134)));
    // 7eef84 fsub DWORD PTR [edi+0x40]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x164)));
    // 7eef87 fstp DWORD PTR [esp+0x30]
    let t59 = f.st(0).to_f32();
    loc[30] = t59.to_bits();
    f.pop();
    // 7eef8b fld DWORD PTR [edi+0x14]
    f.push(x(rd::<f32>(c, 0x138)));
    // 7eef8e fsub DWORD PTR [edi+0x44]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x168)));
    // 7eef91 fstp DWORD PTR [esp+0x34]
    let t60 = f.st(0).to_f32();
    loc[31] = t60.to_bits();
    f.pop();
    // 7eef95 fld DWORD PTR [edi+0x48]
    f.push(x(rd::<f32>(c, 0x16c)));
    // 7eef98 fsub DWORD PTR [edi+0x3c]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x160)));
    // 7eef9b fld DWORD PTR [edi+0x4c]
    f.push(x(rd::<f32>(c, 0x170)));
    // 7eef9e fsub DWORD PTR [edi+0x40]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x164)));
    // 7eefa1 fld DWORD PTR [edi+0x50]
    f.push(x(rd::<f32>(c, 0x174)));
    // 7eefa4 fsub DWORD PTR [edi+0x44]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x168)));
    // 7eefa7 fld DWORD PTR [esp+0x30]
    f.push(x(f32::from_bits(loc[30])));
    // 7eefab fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7eefad fld st(2)
    f.push(f.st(2));
    // 7eefaf fmul DWORD PTR [esp+0x34]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[31])));
    // 7eefb3 fsubp st(1),st
    f.set(1, f.st(1).fsub(f.st(0)));
    f.pop();
    // 7eefb5 fst DWORD PTR [esp+0x28]
    let t61 = f.st(0).to_f32();
    loc[28] = t61.to_bits();
    // 7eefb9 fstp DWORD PTR [esi+0x28]
    let t62 = f.st(0).to_f32();
    wr::<f32>(c, 0xbc, t62);
    f.pop();
    // 7eefbc fld st(2)
    f.push(f.st(2));
    // 7eefbe fmul DWORD PTR [esp+0x34]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[31])));
    // 7eefc2 fxch st(1)
    f.xch(1);
    // 7eefc4 fmul DWORD PTR [esp+0x2c]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[29])));
    // 7eefc8 fsubp st(1),st
    f.set(1, f.st(1).fsub(f.st(0)));
    f.pop();
    // 7eefca fst DWORD PTR [esp+0x3c]
    let t63 = f.st(0).to_f32();
    loc[33] = t63.to_bits();
    // 7eefce fstp DWORD PTR [esi+0x2c]
    let t64 = f.st(0).to_f32();
    wr::<f32>(c, 0xc0, t64);
    f.pop();
    // 7eefd1 fmul DWORD PTR [esp+0x2c]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[29])));
    // 7eefd5 fxch st(1)
    f.xch(1);
    // 7eefd7 fmul DWORD PTR [esp+0x30]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[30])));
    // 7eefdb fsubp st(1),st
    f.set(1, f.st(1).fsub(f.st(0)));
    f.pop();
    // 7eefdd fst DWORD PTR [esi+0x30]
    let t65 = f.st(0).to_f32();
    wr::<f32>(c, 0xc4, t65);
    // 7eefe0 fld DWORD PTR [esp+0x3c]
    f.push(x(f32::from_bits(loc[33])));
    // 7eefe4 fld DWORD PTR [esp+0x28]
    f.push(x(f32::from_bits(loc[28])));
    // 7eefe8 fld st(0)
    f.push(f.st(0));
    // 7eefea fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7eefec fld st(2)
    f.push(f.st(2));
    // 7eefee fmul st,st(3)
    f.set(0, f.st(0).fmul(f.st(3)));
    // 7eeff0 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7eeff2 fld st(3)
    f.push(f.st(3));
    // 7eeff4 fmul st,st(4)
    f.set(0, f.st(0).fmul(f.st(4)));
    // 7eeff6 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7eeff8 fstp DWORD PTR [esp+0x3c]
    let t66 = f.st(0).to_f32();
    loc[33] = t66.to_bits();
    f.pop();
    // 7eeffc mov edx,DWORD PTR [esp+0x3c]
    let t67: u32 = loc[33];
    // 7ef000 fstp st(2)
    f.stp(2);
    // 7ef002 fstp st(1)
    f.stp(1);
    // 7ef004 fstp st(0)
    f.stp(0);
    // 7ef006 push edx
    // 7ef007 call 0x7edb90
    f.push(X87::from_f64_exact(inv_sqrt(f32::from_bits(t67))));
    // 7ef00c fld DWORD PTR [esi+0x28]
    f.push(x(rd::<f32>(c, 0xbc)));
    // 7ef00f fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7ef011 fst DWORD PTR [esi+0x28]
    let t68 = f.st(0).to_f32();
    wr::<f32>(c, 0xbc, t68);
    // 7ef014 fld DWORD PTR [esi+0x2c]
    f.push(x(rd::<f32>(c, 0xc0)));
    // 7ef017 fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7ef019 fst DWORD PTR [esi+0x2c]
    let t69 = f.st(0).to_f32();
    wr::<f32>(c, 0xc0, t69);
    // 7ef01c fld DWORD PTR [esi+0x30]
    f.push(x(rd::<f32>(c, 0xc4)));
    // 7ef01f fmul st,st(3)
    f.set(0, f.st(0).fmul(f.st(3)));
    // 7ef021 fst DWORD PTR [esi+0x30]
    let t70 = f.st(0).to_f32();
    wr::<f32>(c, 0xc4, t70);
    // 7ef024 fld DWORD PTR [edi+0x10]
    f.push(x(rd::<f32>(c, 0x134)));
    // 7ef027 fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7ef029 fld DWORD PTR [edi+0xc]
    f.push(x(rd::<f32>(c, 0x130)));
    // 7ef02c fmul st,st(4)
    f.set(0, f.st(0).fmul(f.st(4)));
    // 7ef02e mov eax,DWORD PTR [esi+0x28]
    let t71: u32 = rd::<u32>(c, 0xbc);
    // 7ef031 mov ecx,DWORD PTR [esi+0x2c]
    let t72: u32 = rd::<u32>(c, 0xc0);
    // 7ef034 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7ef036 mov edx,DWORD PTR [esi+0x30]
    let t73: u32 = rd::<u32>(c, 0xc4);
    // 7ef039 sar eax,0x1f
    let t74 = ((t71 as i32) >> 31) as u32;
    // 7ef03c fld DWORD PTR [edi+0x14]
    f.push(x(rd::<f32>(c, 0x138)));
    // 7ef03f fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7ef041 sar ecx,0x1f
    let t75 = ((t72 as i32) >> 31) as u32;
    // 7ef044 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7ef046 inc al
    let t76 = (t74 & !0xFF) | u32::from((t74 as u8).wrapping_add(1));
    // 7ef048 inc cl
    let t77 = (t75 & !0xFF) | u32::from((t75 as u8).wrapping_add(1));
    // 7ef04a sar edx,0x1f
    let t78 = ((t73 as i32) >> 31) as u32;
    // 7ef04d fstp DWORD PTR [esi+0x34]
    let t79 = f.st(0).to_f32();
    wr::<f32>(c, 0xc8, t79);
    f.pop();
    // 7ef050 inc dl
    let t80 = (t78 & !0xFF) | u32::from((t78 as u8).wrapping_add(1));
    // 7ef052 mov BYTE PTR [esi+0x38],al
    wr::<u8>(c, 0xcc, t76 as u8);
    // 7ef055 fstp st(0)
    f.stp(0);
    // 7ef057 fstp st(0)
    f.stp(0);
    // 7ef059 fstp st(0)
    f.stp(0);
    // 7ef05b fstp st(0)
    f.stp(0);
    // 7ef05d mov BYTE PTR [esi+0x39],cl
    wr::<u8>(c, 0xcd, t77 as u8);
    // 7ef060 mov BYTE PTR [esi+0x3a],dl
    wr::<u8>(c, 0xce, t80 as u8);
    // 7ef063 fld DWORD PTR [edi+0x30]
    f.push(x(rd::<f32>(c, 0x154)));
    // 7ef066 fsub DWORD PTR [edi+0x3c]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x160)));
    // 7ef069 fld DWORD PTR [edi+0x34]
    f.push(x(rd::<f32>(c, 0x158)));
    // 7ef06c fsub DWORD PTR [edi+0x40]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x164)));
    // 7ef06f fld DWORD PTR [edi+0x38]
    f.push(x(rd::<f32>(c, 0x15c)));
    // 7ef072 fsub DWORD PTR [edi+0x44]
    f.set(0, f.st(0).fsub_m(rd::<f32>(c, 0x168)));
    // 7ef075 fld st(1)
    f.push(f.st(1));
    // 7ef077 fmul DWORD PTR [esp+0x38]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[31])));
    // 7ef07b fld st(1)
    f.push(f.st(1));
    // 7ef07d fmul DWORD PTR [esp+0x34]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[30])));
    // 7ef081 fsubp st(1),st
    f.set(1, f.st(1).fsub(f.st(0)));
    f.pop();
    // 7ef083 fst DWORD PTR [esp+0x2c]
    let t81 = f.st(0).to_f32();
    loc[28] = t81.to_bits();
    // 7ef087 fstp DWORD PTR [esi+0x3c]
    let t82 = f.st(0).to_f32();
    wr::<f32>(c, 0xd0, t82);
    f.pop();
    // 7ef08a fmul DWORD PTR [esp+0x30]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[29])));
    // 7ef08e fld st(2)
    f.push(f.st(2));
    // 7ef090 fmul DWORD PTR [esp+0x38]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[31])));
    // 7ef094 fsubp st(1),st
    f.set(1, f.st(1).fsub(f.st(0)));
    f.pop();
    // 7ef096 fst DWORD PTR [esp+0x40]
    let t83 = f.st(0).to_f32();
    loc[33] = t83.to_bits();
    // 7ef09a fstp DWORD PTR [esi+0x40]
    let t84 = f.st(0).to_f32();
    wr::<f32>(c, 0xd4, t84);
    f.pop();
    // 7ef09d fxch st(1)
    f.xch(1);
    // 7ef09f fmul DWORD PTR [esp+0x34]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[30])));
    // 7ef0a3 fld st(1)
    f.push(f.st(1));
    // 7ef0a5 fmul DWORD PTR [esp+0x30]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[29])));
    // 7ef0a9 fsubp st(1),st
    f.set(1, f.st(1).fsub(f.st(0)));
    f.pop();
    // 7ef0ab fxch st(1)
    f.xch(1);
    // 7ef0ad fstp st(0)
    f.stp(0);
    // 7ef0af fst DWORD PTR [esi+0x44]
    let t85 = f.st(0).to_f32();
    wr::<f32>(c, 0xd8, t85);
    // 7ef0b2 fld DWORD PTR [esp+0x40]
    f.push(x(f32::from_bits(loc[33])));
    // 7ef0b6 fld DWORD PTR [esp+0x2c]
    f.push(x(f32::from_bits(loc[28])));
    // 7ef0ba fld st(0)
    f.push(f.st(0));
    // 7ef0bc fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7ef0be fld st(2)
    f.push(f.st(2));
    // 7ef0c0 fmul st,st(3)
    f.set(0, f.st(0).fmul(f.st(3)));
    // 7ef0c2 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7ef0c4 fld st(3)
    f.push(f.st(3));
    // 7ef0c6 fmul st,st(4)
    f.set(0, f.st(0).fmul(f.st(4)));
    // 7ef0c8 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7ef0ca fstp DWORD PTR [esp+0x40]
    let t86 = f.st(0).to_f32();
    loc[33] = t86.to_bits();
    f.pop();
    // 7ef0ce mov eax,DWORD PTR [esp+0x40]
    let t87: u32 = loc[33];
    // 7ef0d2 fstp st(2)
    f.stp(2);
    // 7ef0d4 fstp st(1)
    f.stp(1);
    // 7ef0d6 fstp st(0)
    f.stp(0);
    // 7ef0d8 push eax
    // 7ef0d9 call 0x7edb90
    f.push(X87::from_f64_exact(inv_sqrt(f32::from_bits(t87))));
    // 7ef0de fld DWORD PTR [esi+0x3c]
    f.push(x(rd::<f32>(c, 0xd0)));
    // 7ef0e1 fmul st,st(1)
    f.set(0, f.st(0).fmul(f.st(1)));
    // 7ef0e3 fst DWORD PTR [esi+0x3c]
    let t88 = f.st(0).to_f32();
    wr::<f32>(c, 0xd0, t88);
    // 7ef0e6 fld DWORD PTR [esi+0x40]
    f.push(x(rd::<f32>(c, 0xd4)));
    // 7ef0e9 add esp,0x18
    // 7ef0ec fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7ef0ee mov eax,DWORD PTR [esi+0x3c]
    let t89: u32 = rd::<u32>(c, 0xd0);
    // 7ef0f1 sar eax,0x1f
    let t90 = ((t89 as i32) >> 31) as u32;
    // 7ef0f4 fstp DWORD PTR [esp+0x2c]
    let t91 = f.st(0).to_f32();
    loc[33] = t91.to_bits();
    f.pop();
    // 7ef0f8 fld DWORD PTR [esi+0x44]
    f.push(x(rd::<f32>(c, 0xd8)));
    // 7ef0fb mov ecx,DWORD PTR [esp+0x2c]
    let t92: u32 = loc[33];
    // 7ef0ff inc al
    let t93 = (t90 & !0xFF) | u32::from((t90 as u8).wrapping_add(1));
    // 7ef101 fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7ef103 mov DWORD PTR [esi+0x40],ecx
    wr::<u32>(c, 0xd4, t92);
    // 7ef106 sar ecx,0x1f
    let t94 = ((t92 as i32) >> 31) as u32;
    // 7ef109 fstp DWORD PTR [esp+0xc]
    let t95 = f.st(0).to_f32();
    loc[25] = t95.to_bits();
    f.pop();
    // 7ef10d mov edx,DWORD PTR [esp+0xc]
    let t96: u32 = loc[25];
    // 7ef111 inc cl
    let t97 = (t94 & !0xFF) | u32::from((t94 as u8).wrapping_add(1));
    // 7ef113 mov DWORD PTR [esi+0x44],edx
    wr::<u32>(c, 0xd8, t96);
    // 7ef116 fld DWORD PTR [edi+0x10]
    f.push(x(rd::<f32>(c, 0x134)));
    // 7ef119 fmul DWORD PTR [esp+0x2c]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[33])));
    // 7ef11d sar edx,0x1f
    let t98 = ((t96 as i32) >> 31) as u32;
    // 7ef120 inc dl
    let t99 = (t98 & !0xFF) | u32::from((t98 as u8).wrapping_add(1));
    // 7ef122 fld DWORD PTR [edi+0xc]
    f.push(x(rd::<f32>(c, 0x130)));
    // 7ef125 fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7ef127 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7ef129 fld DWORD PTR [edi+0x14]
    f.push(x(rd::<f32>(c, 0x138)));
    // 7ef12c fmul DWORD PTR [esp+0xc]
    f.set(0, f.st(0).fmul_m(f32::from_bits(loc[25])));
    // 7ef130 mov BYTE PTR [esi+0x4c],al
    wr::<u8>(c, 0xe0, t93 as u8);
    // 7ef133 mov BYTE PTR [esi+0x4d],cl
    wr::<u8>(c, 0xe1, t97 as u8);
    // 7ef136 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7ef138 mov BYTE PTR [esi+0x4e],dl
    wr::<u8>(c, 0xe2, t99 as u8);
    // 7ef13b fstp DWORD PTR [esi+0x48]
    let t100 = f.st(0).to_f32();
    wr::<f32>(c, 0xdc, t100);
    f.pop();
    // 7ef13e fld DWORD PTR [esi+0x28]
    f.push(x(rd::<f32>(c, 0xbc)));
    // 7ef141 fchs 
    f.set(0, f.st(0).fchs());
    // 7ef143 fst DWORD PTR [esi+0x50]
    let t101 = f.st(0).to_f32();
    wr::<f32>(c, 0xe4, t101);
    // 7ef146 fld DWORD PTR [esi+0x2c]
    f.push(x(rd::<f32>(c, 0xc0)));
    // 7ef149 fchs 
    f.set(0, f.st(0).fchs());
    // 7ef14b fst DWORD PTR [esi+0x54]
    let t102 = f.st(0).to_f32();
    wr::<f32>(c, 0xe8, t102);
    // 7ef14e fld DWORD PTR [esi+0x30]
    f.push(x(rd::<f32>(c, 0xc4)));
    // 7ef151 fchs 
    f.set(0, f.st(0).fchs());
    // 7ef153 fst DWORD PTR [esi+0x58]
    let t103 = f.st(0).to_f32();
    wr::<f32>(c, 0xec, t103);
    // 7ef156 fld DWORD PTR [edi+0x24]
    f.push(x(rd::<f32>(c, 0x148)));
    // 7ef159 fmul st,st(3)
    f.set(0, f.st(0).fmul(f.st(3)));
    // 7ef15b mov eax,DWORD PTR [esi+0x50]
    let t104: u32 = rd::<u32>(c, 0xe4);
    // 7ef15e sar eax,0x1f
    let t105 = ((t104 as i32) >> 31) as u32;
    // 7ef161 fld DWORD PTR [edi+0x28]
    f.push(x(rd::<f32>(c, 0x14c)));
    // 7ef164 fmul st,st(3)
    f.set(0, f.st(0).fmul(f.st(3)));
    // 7ef166 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7ef168 fld DWORD PTR [edi+0x2c]
    f.push(x(rd::<f32>(c, 0x150)));
    // 7ef16b fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7ef16d faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7ef16f fstp DWORD PTR [esi+0x5c]
    let t106 = f.st(0).to_f32();
    wr::<f32>(c, 0xf0, t106);
    f.pop();
    // 7ef172 fstp st(0)
    f.stp(0);
    // 7ef174 fstp st(0)
    f.stp(0);
    // 7ef176 fstp st(0)
    f.stp(0);
    // 7ef178 mov ecx,DWORD PTR [esi+0x54]
    let t107: u32 = rd::<u32>(c, 0xe8);
    // 7ef17b mov edx,DWORD PTR [esi+0x58]
    let t108: u32 = rd::<u32>(c, 0xec);
    // 7ef17e fchs 
    f.set(0, f.st(0).fchs());
    // 7ef180 fstp st(1)
    f.stp(1);
    // 7ef182 fst DWORD PTR [esi+0x64]
    let t109 = f.st(0).to_f32();
    wr::<f32>(c, 0xf8, t109);
    // 7ef185 fld DWORD PTR [esp+0x2c]
    f.push(x(f32::from_bits(loc[33])));
    // 7ef189 fchs 
    f.set(0, f.st(0).fchs());
    // 7ef18b fst DWORD PTR [esi+0x68]
    let t110 = f.st(0).to_f32();
    wr::<f32>(c, 0xfc, t110);
    // 7ef18e fld DWORD PTR [esp+0xc]
    f.push(x(f32::from_bits(loc[25])));
    // 7ef192 sar ecx,0x1f
    let t111 = ((t107 as i32) >> 31) as u32;
    // 7ef195 inc al
    let t112 = (t105 & !0xFF) | u32::from((t105 as u8).wrapping_add(1));
    // 7ef197 inc cl
    let t113 = (t111 & !0xFF) | u32::from((t111 as u8).wrapping_add(1));
    // 7ef199 sar edx,0x1f
    let t114 = ((t108 as i32) >> 31) as u32;
    // 7ef19c inc dl
    let t115 = (t114 & !0xFF) | u32::from((t114 as u8).wrapping_add(1));
    // 7ef19e mov BYTE PTR [esi+0x60],al
    wr::<u8>(c, 0xf4, t112 as u8);
    // 7ef1a1 fchs 
    f.set(0, f.st(0).fchs());
    // 7ef1a3 fst DWORD PTR [esi+0x6c]
    let t116 = f.st(0).to_f32();
    wr::<f32>(c, 0x100, t116);
    // 7ef1a6 mov BYTE PTR [esi+0x61],cl
    wr::<u8>(c, 0xf5, t113 as u8);
    // 7ef1a9 mov BYTE PTR [esi+0x62],dl
    wr::<u8>(c, 0xf6, t115 as u8);
    // 7ef1ac fld DWORD PTR [edi+0x24]
    f.push(x(rd::<f32>(c, 0x148)));
    // 7ef1af fmul st,st(3)
    f.set(0, f.st(0).fmul(f.st(3)));
    // 7ef1b1 mov eax,DWORD PTR [esi+0x64]
    let t117: u32 = rd::<u32>(c, 0xf8);
    // 7ef1b4 mov ecx,DWORD PTR [esi+0x68]
    let t118: u32 = rd::<u32>(c, 0xfc);
    // 7ef1b7 mov edx,DWORD PTR [esi+0x6c]
    let t119: u32 = rd::<u32>(c, 0x100);
    // 7ef1ba fld DWORD PTR [edi+0x28]
    f.push(x(rd::<f32>(c, 0x14c)));
    // 7ef1bd fmul st,st(3)
    f.set(0, f.st(0).fmul(f.st(3)));
    // 7ef1bf sar eax,0x1f
    let t120 = ((t117 as i32) >> 31) as u32;
    // 7ef1c2 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7ef1c4 sar ecx,0x1f
    let t121 = ((t118 as i32) >> 31) as u32;
    // 7ef1c7 inc al
    let t122 = (t120 & !0xFF) | u32::from((t120 as u8).wrapping_add(1));
    // 7ef1c9 inc cl
    let t123 = (t121 & !0xFF) | u32::from((t121 as u8).wrapping_add(1));
    // 7ef1cb fld DWORD PTR [edi+0x2c]
    f.push(x(rd::<f32>(c, 0x150)));
    // 7ef1ce fmul st,st(2)
    f.set(0, f.st(0).fmul(f.st(2)));
    // 7ef1d0 sar edx,0x1f
    let t124 = ((t119 as i32) >> 31) as u32;
    // 7ef1d3 faddp st(1),st
    f.set(1, f.st(1).fadd(f.st(0)));
    f.pop();
    // 7ef1d5 inc dl
    let t125 = (t124 & !0xFF) | u32::from((t124 as u8).wrapping_add(1));
    // 7ef1d7 mov BYTE PTR [esi+0x74],al
    wr::<u8>(c, 0x108, t122 as u8);
    // 7ef1da mov BYTE PTR [esi+0x75],cl
    wr::<u8>(c, 0x109, t123 as u8);
    // 7ef1dd mov BYTE PTR [esi+0x76],dl
    wr::<u8>(c, 0x10a, t125 as u8);
    // 7ef1e0 fstp DWORD PTR [esi+0x70]
    let t126 = f.st(0).to_f32();
    wr::<f32>(c, 0x104, t126);
    f.pop();
    // 7ef1e3 pop edi
    // 7ef1e4 pop esi
    // 7ef1e5 fstp st(0)
    f.stp(0);
    // 7ef1e7 fstp st(0)
    f.stp(0);
    // 7ef1e9 fstp st(0)
    f.stp(0);
    // 7ef1eb pop ebx
    // 7ef1ec add esp,0x1c
    // 7ef1ef ret 
    debug_assert!(f.n == 0);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_CAMERA_END_UPDATE,
    HOOK_CAMERA_BEGIN_UPDATE,
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
    HOOK_RW_CAMERA_BUILD_PERSP_CLIP_PLANES,
    HOOK_RW_CAMERA_BUILD_PARALLEL_CLIP_PLANES,
];

/// Globals owned by this module.
pub const GLOBALS: &[GlobalDesc] = CAMERA_GLOBALS;
