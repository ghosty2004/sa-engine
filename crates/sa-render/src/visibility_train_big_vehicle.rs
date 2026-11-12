//! The train and big-vehicle atomic render callbacks of `CVisibilityPlugins` (batch `boot-render-1`), installed by
//! `CVehicleModelInfo::SetAtomicRendererCB_Train` / `_BigVehicle`: `RenderVehicleReallyLowDetailCB_BigVehicle`
//! (0x732820), `RenderTrainHiDetailCB` (0x733330) and `RenderTrainHiDetailAlphaCB` (0x734240). Ported from the 1.0 US
//! bytes (gta-reversed's versions as a reading aid). They differ from the ordinary vehicle callbacks (0x7331E0,
//! 0x733240, 0x733F80) in the LOD distance (`ms_bigVehicleLod0Dist`) and, for the alpha one, the sort key of
//! flag 0x40. Unverified (docs/verify/HOWTO.md).

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
    /// `CVisibilityPlugins::GetDotProductWithCameraVector(ltm, carLtm, atomicId)` (0x7326D0, ST0; computed from f32
    /// operands under precision control 24, so the double carries it exactly)
    fn dot_with_camera(m: GPtr<u8>, car: GPtr<u8>, id: u32) -> f64 = cdecl 0x7326D0 => unported;
    /// The alpha-atomic list's sorted insert (`CLinkList<AlphaObjectInfo>::InsertSorted`, 0x733910, thiscall);
    /// NULL when the list is full
    fn alpha_list_insert_sorted(list: GPtr<u8>, info: GPtr<u8>) -> GPtr<u8> = thiscall 0x733910 => crate::render_lists::sorted_list_insert;
}

/// `gVehicleAngleToCamera`, `gVehicleDistanceFromCamera`, `ms_vehicleLod0Dist`... (`CVisibilityPlugins`):
/// the small-vehicle LOD distance (0xC8802C), the big-vehicle LOD0 distance (0xC88038), `ms_vehicleLod1Dist`
/// (0xC88044), the alpha-atomic list (0xC88070), the atomic plugin offset (0x8D608C).
const ANGLE_TO_CAMERA: usize = 0xC8_8020;
const DISTANCE_FROM_CAMERA: usize = 0xC8_8024;
const LOD_SMALL_DIST: usize = 0xC8_802C;
const BIG_VEHICLE_LOD0_DIST: usize = 0xC8_8038;
const LOD1_DIST: usize = 0xC8_8044;
const ALPHA_ATOMIC_LIST: usize = 0xC8_8070;
const ATOMIC_PLUGIN_OFFSET: usize = 0x8D_608C;
/// `.rdata` constants: 0.0 (0x858B50), 0.1 (0x858B1C), 0.2 (0x858CC4).
const K_ZERO: usize = 0x85_8B50;
const K_TENTH: usize = 0x85_8B1C;
const K_ANGLE: usize = 0x85_8CC4;
/// `CVisibilityPlugins::RenderAtomic` (0x732B30), the render function stored in the alpha list entries.
const RENDER_ATOMIC: u32 = 0x73_2B30;

fn k(addr: usize) -> X87 {
    X87::from_f32(g::<f32>(addr).read())
}

fn dist() -> X87 {
    k(DISTANCE_FROM_CAMERA)
}

/// `fld dist; fcomp [ms_bigVehicleLod0Dist]; test ah, 5; jp`: true (skip) unless the distance is below it.
fn beyond_big_lod0() -> bool {
    dist().fcom(k(BIG_VEHICLE_LOD0_DIST)).test_parity(5)
}

/// The atomic's plugin flags byte 3: the unique-materials bit 0x20 cleared below `ms_vehicleLod1Dist`, else set.
fn set_unique_materials_by_lod1(atomic: GPtr<u8>) {
    let f = atomic.byte_add(g::<i32>(ATOMIC_PLUGIN_OFFSET).read() as isize + 3);
    if !dist().fcom(k(LOD1_DIST)).test_parity(5) {
        f.write(f.read() & 0xDF);
    } else {
        f.write(f.read() | 0x20);
    }
}

/// The atomic id (plugin word +2, zero-extended).
fn atomic_id(atomic: GPtr<u8>) -> u32 {
    u32::from(rd::<u16>(atomic, g::<i32>(ATOMIC_PLUGIN_OFFSET).read() as isize + 2))
}

/// The close-range tests that make the atomic render without a camera-facing check: the distance at most the
/// small-vehicle LOD distance (or unordered), id flag 0x400, or the angle to the camera not below 0.2.
fn always_visible(id: u32) -> bool {
    dist().fcom(k(LOD_SMALL_DIST)).test_any(0x41)
        || id & 0x400 != 0
        || k(ANGLE_TO_CAMERA).fcom(k(K_ANGLE)).test_parity(5)
}

/// The back-face rejection after [`always_visible`] failed: rendered when `dot` (the value compared: ST0 in
/// `RenderTrainHiDetailCB`, the stored f32 in the alpha callback) is at most 0 (or unordered); else rejected for id
/// flag 0x80, else rejected when `dist * 0.1 < dot32 * dot32` on the stored f32 (`fcompp` then `test ah, 5; jnp`).
fn rejected_by_dot(id: u32, dot: X87, dot32: f32) -> bool {
    if dot.fcom(k(K_ZERO)).test_any(0x41) {
        return false;
    }
    if id & 0x80 != 0 {
        return true;
    }
    let sq = X87::from_f32(dot32).fmul_m(dot32);
    !dist().fmul_m(g::<f32>(K_TENTH).read()).fcom(sq).test_parity(5)
}

/// `CVisibilityPlugins::RenderVehicleReallyLowDetailCB_BigVehicle(RpAtomic*)` (0x732820, cdecl): from
/// `ms_bigVehicleLod0Dist` on (`test ah, 1; jne` skips only below it or unordered),
/// `AtomicDefaultRenderCallBack(atomic)`. Returns the atomic. Unlike the ordinary vehicle callback it neither sets
/// the unique-materials flag nor looks at the clump alpha.
#[hook(addr = 0x732820, conv = cdecl, name = "CVisibilityPlugins::RenderVehicleReallyLowDetailCB_BigVehicle",
       expect = "D9 05 24 80 C8 00 D8 1D 38 80 C8 00",
       unverified = "boot-render-1", route = "L2", hazards = "none", recs = "boot")]
pub fn render_vehicle_really_low_detail_cb_big_vehicle(atomic: GPtr<u8>) -> GPtr<u8> {
    if !dist().fcom(k(BIG_VEHICLE_LOD0_DIST)).c0() {
        atomic_default_render(atomic);
    }
    atomic
}

/// `CVisibilityPlugins::RenderTrainHiDetailCB(RpAtomic*)` (0x733330, cdecl): below `ms_bigVehicleLod0Dist` only;
/// the unique-materials flag by `ms_vehicleLod1Dist`; unless [`always_visible`], the dot product of the atomic's
/// LTM with its clump frame's LTM (`GetDotProductWithCameraVector`, the atomic frame's `RwFrameGetLTM` first) is
/// compared unrounded with 0 and squared as the stored f32 ([`rejected_by_dot`]); otherwise `AtomicDefaultRenderCallBack`. Returns the atomic.
#[hook(addr = 0x733330, conv = cdecl, name = "CVisibilityPlugins::RenderTrainHiDetailCB",
       expect = "51 D9 05 24 80 C8 00 53",
       unverified = "boot-render-1", route = "L2", hazards = "none", recs = "boot")]
pub fn render_train_hi_detail_cb(atomic: GPtr<u8>) -> GPtr<u8> {
    if beyond_big_lod0() {
        return atomic;
    }
    set_unique_materials_by_lod1(atomic);
    let clump_frame = rd::<GPtr<u8>>(rd::<GPtr<u8>>(atomic, 0x3C), 4);
    let id = atomic_id(atomic);
    if !always_visible(id) {
        let a = frame_get_ltm(rd::<GPtr<u8>>(atomic, 4));
        let b = frame_get_ltm(clump_frame);
        // `fst dword [local]`, then `fcomp 0.0` on the unrounded ST0
        let dot = X87::from_f64_exact(dot_with_camera(a, b, id));
        if rejected_by_dot(id, dot, dot.to_f32()) {
            return atomic;
        }
    }
    atomic_default_render(atomic);
    atomic
}

/// `CVisibilityPlugins::RenderTrainHiDetailAlphaCB(RpAtomic*)` (0x734240, cdecl): below `ms_bigVehicleLod0Dist`
/// only; the unique-materials flag by `ms_vehicleLod1Dist`; the dot product always computed first (the atomic
/// frame's LTM, then the clump frame's) and stored as f32; unless [`always_visible`], [`rejected_by_dot`] may skip
/// the atomic. Otherwise an `AlphaObjectInfo { atomic, RenderAtomic (0x732B30), key }` goes to the sorted
/// alpha-atomic list with key = the distance (its bits copied) for id flag 0x40, else `dist + dot` rounded to f32;
/// when the list is full (insert returns NULL) the atomic is rendered at once. Returns the atomic.
#[hook(addr = 0x734240, conv = cdecl, name = "CVisibilityPlugins::RenderTrainHiDetailAlphaCB",
       expect = "D9 05 24 80 C8 00 83 EC 10",
       unverified = "boot-render-1", route = "L2", hazards = "none", recs = "boot")]
pub fn render_train_hi_detail_alpha_cb(atomic: GPtr<u8>) -> GPtr<u8> {
    if beyond_big_lod0() {
        return atomic;
    }
    set_unique_materials_by_lod1(atomic);
    let clump_frame = rd::<GPtr<u8>>(rd::<GPtr<u8>>(atomic, 0x3C), 4);
    let a = frame_get_ltm(rd::<GPtr<u8>>(atomic, 4));
    let id = atomic_id(atomic);
    let b = frame_get_ltm(clump_frame);
    let dot = X87::from_f64_exact(dot_with_camera(a, b, id)).to_f32();
    if !always_visible(id) && rejected_by_dot(id, X87::from_f32(dot), dot) {
        return atomic;
    }
    let key = if id & 0x40 != 0 {
        g::<u32>(DISTANCE_FROM_CAMERA).read()
    } else {
        dist().fadd_m(dot).to_f32().to_bits()
    };
    let mut info = [atomic.addr() as u32, RENDER_ATOMIC, key];
    if alpha_list_insert_sorted(g::<u8>(ALPHA_ATOMIC_LIST), GPtr::from_raw(info.as_mut_ptr()).cast()).is_null() {
        atomic_default_render(atomic);
    }
    atomic
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_RENDER_VEHICLE_REALLY_LOW_DETAIL_CB_BIG_VEHICLE,
    HOOK_RENDER_TRAIN_HI_DETAIL_CB,
    HOOK_RENDER_TRAIN_HI_DETAIL_ALPHA_CB,
];
