//! The vehicle atomic render callbacks of `CVisibilityPlugins` (batch `std10-render-1`): `GetDotProductWithCameraVector`
//! (0x7326D0), `RenderVehicleReallyLowDetailCB` (0x7331E0), `RenderVehicleHiDetailCB` (0x733240) and
//! `RenderVehicleHiDetailAlphaCB` (0x733F80). Ported from the 1.0 US bytes (gta-reversed's versions as a reading aid).
//! Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{g, rd};

orig! {
    /// `RwFrameGetLTM(frame)` (0x7F0990)
    fn frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
    /// `AtomicDefaultRenderCallBack(atomic)` (0x7491C0)
    fn atomic_default_render(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x7491C0 => unported;
    /// `CVisibilityPlugins::RenderAtomicWithAlpha(atomic, alpha)` (0x732480)
    fn render_atomic_with_alpha(a: GPtr<u8>, alpha: i32) = cdecl 0x732480 => crate::visibility_ped::render_atomic_with_alpha;
    /// `CVisibilityPlugins::GetDotProductWithCameraVector(ltm, carLtm, atomicId)` (0x7326D0)
    fn dot_with_camera_call(m: GPtr<u8>, car: GPtr<u8>, id: u32) -> f64 = cdecl 0x7326D0 => dot_with_camera_f64;
    /// The alpha-atomic list's sorted insert (`CLinkList<AlphaObjectInfo>::InsertSorted`, 0x733910, thiscall)
    fn alpha_list_insert_sorted(list: GPtr<u8>, info: GPtr<u8>) -> GPtr<u8> = thiscall 0x733910 => crate::render_lists::sorted_list_insert;
}

/// `ms_pCameraPosn`, `gVehicleAngleToCamera`, `gVehicleDistanceFromCamera`, `ms_bigVehicleLod0Dist`,
/// `ms_vehicleLod0Dist`, `ms_vehicleLod1Dist`, the alpha-atomic list, the atomic / clump plugin offsets.
const CAMERA_POSN: usize = 0xC88050;
const ANGLE_TO_CAMERA: usize = 0xC88020;
const DISTANCE_FROM_CAMERA: usize = 0xC88024;
const LOD_SMALL_DIST: usize = 0xC8802C;
const LOD0_DIST: usize = 0xC88040;
const LOD1_DIST: usize = 0xC88044;
const ALPHA_ATOMIC_LIST: usize = 0xC88070;
const ATOMIC_PLUGIN_OFFSET: usize = 0x8D608C;
const CLUMP_PLUGIN_OFFSET: usize = 0x8D6094;
/// `.rdata` constants: 0.0, 0.1, 0.2, 0.25, 2.5, 0.0001.
const K_ZERO: usize = 0x858B50;
const K_TENTH: usize = 0x858B1C;
const K_ANGLE: usize = 0x858CC4;
const K_QUARTER: usize = 0x858C84;
const K_TWO_HALF: usize = 0x858FA0;
const K_EPS: usize = 0x858FC4;

fn k(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

fn x(v: f32) -> X87 {
    X87::from_f32(v)
}

/// `(m.y * d.y + m.x * d.x) + m.z * d.z` of the matrix row at `off` (x87).
fn dot_row(car: GPtr<u8>, off: isize, d: [f32; 3]) -> X87 {
    x(rd::<f32>(car, off + 4)).fmul_m(d[1]).fadd(x(rd::<f32>(car, off)).fmul_m(d[0])).fadd(x(rd::<f32>(car, off + 8)).fmul_m(d[2]))
}

/// `CVisibilityPlugins::GetDotProductWithCameraVector(RwMatrix* ltm, RwMatrix* carLtm, uint32 atomicId)` (0x7326D0,
/// cdecl; ST0). `d = ltm.pos - *ms_pCameraPosn` (f32 components). Front / rear atomics (0x30): `d . up`; right / left
/// (0xC): `d . right` (as `(d.y * r.y + d.x * r.x) + d.z * r.z`); else 0; negated for rear / left (0x24). Top (0x8000):
/// plus `(d . at) * (doors 0x300 ? 0.25 : 2.5)`. Doors: the second dot `d . up` (negated for the rear door 0x100) is
/// added when both have the same sign (both below zero, then both above).
#[hook(addr = 0x7326D0, conv = cdecl, name = "CVisibilityPlugins::GetDotProductWithCameraVector", expect = "83 EC 0C 8B 44 24 10 D9 40 30",
       unverified = "std10-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn dot_with_camera(m: GPtr<u8>, car: GPtr<u8>, id: u32) -> X87 {
    let c = g::<GPtr<u8>>(CAMERA_POSN).read();
    let d = [
        x(rd::<f32>(m, 0x30)).fsub_m(rd::<f32>(c, 0)).to_f32(),
        x(rd::<f32>(m, 0x34)).fsub_m(rd::<f32>(c, 4)).to_f32(),
        x(rd::<f32>(m, 0x38)).fsub_m(rd::<f32>(c, 8)).to_f32(),
    ];
    let (cl, ch) = (id as u8, (id >> 8) as u8);
    let mut p = if cl & 0x30 != 0 {
        dot_row(car, 0x10, d)
    } else if cl & 0xC != 0 {
        // `fld [m+4]; fmul d.y; fld d.x; fmul [m]`: the same products
        dot_row(car, 0, d)
    } else {
        x(k(K_ZERO))
    };
    if cl & 0x24 != 0 {
        p = p.fchs();
    }
    if ch & 0x80 != 0 {
        let z = dot_row(car, 0x20, d);
        let z = if ch & 3 != 0 { z.fmul_m(k(K_QUARTER)) } else { z.fmul_m(k(K_TWO_HALF)) };
        p = p.fadd(z);
    }
    if ch & 3 == 0 {
        return p;
    }
    let up = dot_row(car, 0x10, d);
    let s = if ch & 1 != 0 { up.fchs() } else { up };
    let zero = x(k(K_ZERO));
    if !p.fcom(zero).test_parity(5) && !s.fcom(zero).test_parity(5) {
        p = p.fadd(s);
    }
    if p.fcom(zero).test_any(0x41) || s.fcom(zero).test_any(0x41) {
        return p;
    }
    p.fadd(s)
}

#[cfg_attr(sa_phase_a, allow(dead_code))]
fn dot_with_camera_f64(m: GPtr<u8>, car: GPtr<u8>, id: u32) -> f64 {
    dot_with_camera(m, car, id).to_f64()
}

fn atomic_flags3(atomic: GPtr<u8>) -> GPtr<u8> {
    atomic.byte_add(g::<i32>(ATOMIC_PLUGIN_OFFSET).read() as isize + 3)
}

fn atomic_id(atomic: GPtr<u8>) -> u32 {
    u32::from(rd::<u16>(atomic, g::<i32>(ATOMIC_PLUGIN_OFFSET).read() as isize + 2))
}

fn dist() -> X87 {
    x(k(DISTANCE_FROM_CAMERA))
}

/// The unique-materials flag (atomic flags byte 3, 0x20) cleared below `ms_vehicleLod1Dist`, else set.
fn set_unique_materials_by_lod1(atomic: GPtr<u8>) {
    let f = atomic_flags3(atomic);
    if !dist().fcom(x(k(LOD1_DIST))).test_parity(5) {
        f.write(f.read() & 0xDF);
    } else {
        f.write(f.read() | 0x20);
    }
}

/// `CVisibilityPlugins::RenderVehicleReallyLowDetailCB(RpAtomic*)` (0x7331E0, cdecl): from `ms_vehicleLod0Dist` on
/// (not below), the unique-materials flag set and the atomic rendered (`RenderAtomicWithAlpha` unless the clump's
/// alpha is 255). Returns the atomic.
#[hook(addr = 0x7331E0, conv = cdecl, name = "CVisibilityPlugins::RenderVehicleReallyLowDetailCB", expect = "D9 05 24 80 C8 00 56",
       unverified = "std10-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn render_vehicle_really_low_detail_cb(atomic: GPtr<u8>) -> GPtr<u8> {
    if dist().fcom(x(k(LOD0_DIST))).c0() {
        return atomic;
    }
    let f = atomic_flags3(atomic);
    f.write(f.read() | 0x20);
    let clump = rd::<GPtr<u8>>(atomic, 0x3C);
    let alpha = rd::<i32>(clump, g::<i32>(CLUMP_PLUGIN_OFFSET).read() as isize + 4);
    if alpha == 0xFF {
        atomic_default_render(atomic);
    } else {
        render_atomic_with_alpha(atomic, alpha);
    }
    atomic
}

/// The back-face test of the hi-detail callbacks: the atomic is skipped (true) when beyond the small-vehicle LOD
/// distance, not flagged 0x400, the angle to the camera below 0.2, the dot product above 0 and (flag 0x80, or the
/// squared dot above a tenth of the distance).
fn hidden_from_camera(id: u32, dot: X87, dot32: f32) -> bool {
    if dist().fcom(x(k(LOD_SMALL_DIST))).test_any(0x41) || id & 0x400 != 0 {
        return false;
    }
    if x(k(ANGLE_TO_CAMERA)).fcom(x(k(K_ANGLE))).test_parity(5) {
        return false;
    }
    if dot.fcom(x(k(K_ZERO))).test_any(0x41) {
        return false;
    }
    if id & 0x80 != 0 {
        return true;
    }
    let sq = x(dot32).fmul_m(dot32);
    !dist().fmul_m(k(K_TENTH)).fcom(sq).test_parity(5)
}

/// `CVisibilityPlugins::RenderVehicleHiDetailCB(RpAtomic*)` (0x733240, cdecl): below `ms_vehicleLod0Dist` only; the
/// unique-materials flag by `ms_vehicleLod1Dist`; unless hidden from the camera (the dot product of the atomic's LTM
/// with its clump frame's, `hidden_from_camera`), `AtomicDefaultRenderCallBack`. Returns the atomic.
#[hook(addr = 0x733240, conv = cdecl, name = "CVisibilityPlugins::RenderVehicleHiDetailCB", expect = "51 D9 05 24 80 C8 00 53",
       unverified = "std10-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn render_vehicle_hi_detail_cb(atomic: GPtr<u8>) -> GPtr<u8> {
    if dist().fcom(x(k(LOD0_DIST))).test_parity(5) {
        return atomic;
    }
    set_unique_materials_by_lod1(atomic);
    let clump_frame = rd::<GPtr<u8>>(rd::<GPtr<u8>>(atomic, 0x3C), 4);
    let id = atomic_id(atomic);
    let near = dist().fcom(x(k(LOD_SMALL_DIST))).test_any(0x41)
        || id & 0x400 != 0
        || x(k(ANGLE_TO_CAMERA)).fcom(x(k(K_ANGLE))).test_parity(5);
    if !near {
        let a = frame_get_ltm(rd::<GPtr<u8>>(atomic, 4));
        let b = frame_get_ltm(clump_frame);
        let dot = X87::from_f64_exact(dot_with_camera_call(a, b, id));
        if hidden_from_camera(id, dot, dot.to_f32()) {
            return atomic;
        }
    }
    atomic_default_render(atomic);
    atomic
}

/// `CVisibilityPlugins::RenderVehicleHiDetailAlphaCB(RpAtomic*)` (0x733F80, cdecl): below `ms_vehicleLod0Dist` only;
/// the unique-materials flag by `ms_vehicleLod1Dist`; the dot product (stored as f32) always computed; unless hidden
/// from the camera, the atomic goes to the sorted alpha-atomic list (`RenderAtomic` 0x732B30, key distance - 0.0001
/// for flag 0x40, else distance + dot), rendered at once when the list is full. Returns the atomic.
#[hook(addr = 0x733F80, conv = cdecl, name = "CVisibilityPlugins::RenderVehicleHiDetailAlphaCB", expect = "D9 05 24 80 C8 00 83 EC 10",
       unverified = "std10-render-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn render_vehicle_hi_detail_alpha_cb(atomic: GPtr<u8>) -> GPtr<u8> {
    if dist().fcom(x(k(LOD0_DIST))).test_parity(5) {
        return atomic;
    }
    set_unique_materials_by_lod1(atomic);
    let clump_frame = rd::<GPtr<u8>>(rd::<GPtr<u8>>(atomic, 0x3C), 4);
    let a = frame_get_ltm(rd::<GPtr<u8>>(atomic, 4));
    let id = atomic_id(atomic);
    let b = frame_get_ltm(clump_frame);
    let dot = X87::from_f64_exact(dot_with_camera_call(a, b, id)).to_f32();
    if hidden_from_camera(id, x(dot), dot) {
        return atomic;
    }
    let key = if id & 0x40 != 0 { dist().fsub_m(k(K_EPS)) } else { dist().fadd_m(dot) };
    let mut info = [atomic.addr() as u32, 0x0073_2B30, key.to_f32().to_bits()];
    if alpha_list_insert_sorted(g::<u8>(ALPHA_ATOMIC_LIST), GPtr::from_raw(info.as_mut_ptr()).cast()).is_null() {
        atomic_default_render(atomic);
    }
    atomic
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_DOT_WITH_CAMERA,
    HOOK_RENDER_VEHICLE_REALLY_LOW_DETAIL_CB,
    HOOK_RENDER_VEHICLE_HI_DETAIL_CB,
    HOOK_RENDER_VEHICLE_HI_DETAIL_ALPHA_CB,
];
