//! `RenderBuffer::StartStoring` and `CVisibilityPlugins::SetupVehicleVariables` (batch `std9-render-8`): ported from
//! the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd};

orig! {
    /// `RenderBuffer::RenderStuffInBuffer()` (0x707800)
    fn render_stuff_in_buffer() = cdecl 0x707800 => unported;
    /// `RwFrameGetLTM(frame)` (0x7F0990)
    fn frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
}

/// `TempBufferIndicesStored` (0xC4B954), `TempBufferVerticesStored` (0xC4B950), `TempBufferRenderIndexList`
/// (u16, 0xC4B958), `TempVertexBuffer` (0x24-byte vertices, 0xC4D958), the requested counts (0xC40410 / 0xC40414)
const INDICES_STORED: usize = 0xC4B954;
const VERTICES_STORED: usize = 0xC4B950;
const INDEX_LIST: u32 = 0xC4B958;
const VERTEX_BUFFER: u32 = 0xC4D958;
const NUM_INDICES_REQUESTED: usize = 0xC40410;
const NUM_VERTICES_REQUESTED: usize = 0xC40414;

/// `RenderBuffer::StartStoring(int32 numIndices, int32 numVertices, RxVertexIndex** indices, RwIm3DVertex** vertices)`
/// (0x708290, cdecl): the buffer is flushed (`RenderStuffInBuffer`) when the stored indices plus `numIndices` reach
/// 0x1000, and again when the stored vertices plus `numVertices` reach 0x800 (signed); the out pointers are the next
/// free slots; the requested counts are recorded.
#[hook(addr = 0x708290, conv = cdecl, name = "RenderBuffer::StartStoring", expect = "A1 54 B9 C4 00 56 57 8B",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn render_buffer_start_storing(num_indices: i32, num_vertices: i32, indices: GPtr<u32>, vertices: GPtr<u32>) {
    if g::<i32>(INDICES_STORED).read().wrapping_add(num_indices) >= 0x1000 {
        render_stuff_in_buffer();
    }
    if g::<i32>(VERTICES_STORED).read().wrapping_add(num_vertices) >= 0x800 {
        render_stuff_in_buffer();
    }
    indices.write(INDEX_LIST.wrapping_add(2 * g::<u32>(INDICES_STORED).read()));
    vertices.write(VERTEX_BUFFER.wrapping_add(0x24 * g::<u32>(VERTICES_STORED).read()));
    g::<i32>(NUM_INDICES_REQUESTED).write(num_indices);
    g::<i32>(NUM_VERTICES_REQUESTED).write(num_vertices);
}

/// `CVisibilityPlugins::ms_pCameraPosn` (0xC88050), `ms_vehicleLod0Dist`-style results: the squared distance
/// (0xC88024) and the elevation angle (0xC88020)
const CAMERA_POSN: usize = 0xC88050;
const VEHICLE_DIST_SQ: usize = 0xC88024;
const VEHICLE_ANGLE: usize = 0xC88020;

/// `CVisibilityPlugins::SetupVehicleVariables(RwObject*)` (0x733160, cdecl): for a clump (type byte 2), the squared
/// distance of its frame's LTM position from the camera (`(dz*dz + dy*dy) + dx*dx`, f32) and the angle
/// `atan2(cam.z - pos.z, sqrt((cam.y - pos.y)^2 + (cam.x - pos.x)^2))` of its frame's own position (+0x40).
#[hook(addr = 0x733160, conv = cdecl, name = "CVisibilityPlugins::SetupVehicleVariables", expect = "8B 44 24 04 80 38 02 75",
       route = "L2", hazards = "x87-transcendental", recs = "grove-walk")]
pub fn setup_vehicle_variables(obj: GPtr<u8>) {
    if rd::<u8>(obj, 0) != 2 {
        return;
    }
    let frame = rd::<GPtr<u8>>(obj, 4);
    let m = frame_get_ltm(frame).byte_add(0x30);
    let cam = g::<GPtr<u8>>(CAMERA_POSN).read();
    let d = |o: isize| X87::from_f32(rd::<f32>(m, o)).fsub_m(rd::<f32>(cam, o));
    let (dx, dy, dz) = (d(0), d(4), d(8));
    g::<f32>(VEHICLE_DIST_SQ).write(dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).to_f32());
    let e = |o: isize| X87::from_f32(rd::<f32>(frame, 0x40 + o)).fsubr_m(rd::<f32>(cam, o));
    let (a, b, c) = (e(0), e(4), e(8));
    let h = b.fmul(b).fadd(a.fmul(a)).fsqrt();
    g::<f32>(VEHICLE_ANGLE).write(c.fpatan(h).to_f32());
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_RENDER_BUFFER_START_STORING, HOOK_SETUP_VEHICLE_VARIABLES];
