//! The big-vehicle, boat and heli-rotor atomic render callbacks of `CVisibilityPlugins` and
//! `InsertEntityIntoUnderwaterList` (batch `samp-rw-1`), installed by `CVehicleModelInfo::SetAtomicRendererCB_*`.
//! Ported from the 1.0 US bytes (gta-reversed's versions as a reading aid). Compared with the train callbacks
//! (`visibility_train_big_vehicle`) the big-vehicle ones test the near distance 0xC88028 instead of the small-vehicle
//! LOD distance 0xC8802C. Unverified (docs/verify/HOWTO.md).

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
    /// `CVisibilityPlugins::RenderAlphaAtomic(RpAtomic*, int32 alpha)` (0x732480)
    fn render_alpha_atomic(a: GPtr<u8>, alpha: u32) -> GPtr<u8> = cdecl 0x732480 => unported;
    /// `CVisibilityPlugins::GetDotProductWithCameraVector(ltm, carLtm, atomicId)` (0x7326D0, ST0)
    fn dot_with_camera(m: GPtr<u8>, car: GPtr<u8>, id: u32) -> f64 = cdecl 0x7326D0 => unported;
    /// `CLinkList<AlphaObjectInfo>::InsertSorted` (0x733910, thiscall): NULL when the list is full
    fn alpha_list_insert_sorted(list: GPtr<u8>, info: GPtr<u8>) -> GPtr<u8> = thiscall 0x733910 => crate::render_lists::sorted_list_insert;
}

/// `CVisibilityPlugins` globals: angle to camera 0xC88020, distance from camera 0xC88024, the big-vehicle near
/// distance 0xC88028, `ms_bigVehicleLod0Dist` 0xC88038, the boat LOD distance 0xC88040, `ms_vehicleLod1Dist`
/// 0xC88044, `ms_pCameraPosn` 0xC88050, the alpha-atomic list 0xC88070, the boat alpha list 0xC880C8, the
/// underwater entity list 0xC88178, the atomic and clump plugin offsets 0x8D608C / 0x8D6094.
const ANGLE_TO_CAMERA: usize = 0xC8_8020;
const DISTANCE_FROM_CAMERA: usize = 0xC8_8024;
const NEAR_DIST: usize = 0xC8_8028;
const BIG_VEHICLE_LOD0_DIST: usize = 0xC8_8038;
const BOAT_LOD_DIST: usize = 0xC8_8040;
const LOD1_DIST: usize = 0xC8_8044;
const CAMERA_POSN: usize = 0xC8_8050;
const ALPHA_ATOMIC_LIST: usize = 0xC8_8070;
const ALPHA_BOAT_LIST: usize = 0xC8_80C8;
const UNDERWATER_LIST: usize = 0xC8_8178;
const ATOMIC_PLUGIN_OFFSET: usize = 0x8D_608C;
const CLUMP_PLUGIN_OFFSET: usize = 0x8D_6094;
/// `.rdata`: 0.0 (0x858B50), 0.1 (0x858B1C), 0.2 (0x858CC4), 20.0 (0x858BA4), 0.0001 (0x858FC4).
const K_ZERO: usize = 0x85_8B50;
const K_TENTH: usize = 0x85_8B1C;
const K_ANGLE: usize = 0x85_8CC4;
const K_ROTOR: usize = 0x85_8BA4;
const K_TINY: usize = 0x85_8FC4;
/// `CVisibilityPlugins::RenderAtomic` (0x732B30) and `RenderEntity` (0x732B40), the render functions stored in the
/// sorted list entries.
const RENDER_ATOMIC: u32 = 0x73_2B30;
const RENDER_ENTITY: u32 = 0x73_2B40;

fn k(addr: usize) -> X87 {
    X87::from_f32(g::<f32>(addr).read())
}

fn kf(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

fn dist() -> X87 {
    k(DISTANCE_FROM_CAMERA)
}

/// `fld dist; fcomp [lod]; test ah, 5; jp`: true unless the distance is below `lod` (ordered).
fn beyond(lod: usize) -> bool {
    dist().fcom(k(lod)).test_parity(5)
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

/// The clump's frame (`atomic->clump->frame`).
fn clump_frame(atomic: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(rd::<GPtr<u8>>(atomic, 0x3C), 4)
}

/// The tests that render the big-vehicle atomic without the camera-facing check: the distance at most the near
/// distance (or unordered), id flag 0x400, or the angle to the camera not below 0.2 (or unordered).
fn big_vehicle_always_visible(id: u32) -> bool {
    dist().fcom(k(NEAR_DIST)).test_any(0x41) || id & 0x400 != 0 || k(ANGLE_TO_CAMERA).fcom(k(K_ANGLE)).test_parity(5)
}

/// An `AlphaObjectInfo { atomic or entity, render function, key }` into the sorted list at `list`: NULL when full.
fn insert(list: usize, obj: GPtr<u8>, render: u32, key: u32) -> GPtr<u8> {
    let mut info = [obj.addr() as u32, render, key];
    alpha_list_insert_sorted(g::<u8>(list), GPtr::from_raw(info.as_mut_ptr()).cast())
}

/// `CVisibilityPlugins::RenderVehicleHiDetailCB_BigVehicle(RpAtomic*)` (0x733420, cdecl): below
/// `ms_bigVehicleLod0Dist` only; the unique-materials flag by `ms_vehicleLod1Dist`; unless
/// [`big_vehicle_always_visible`], `GetDotProductWithCameraVector(LTM(atomic frame), LTM(clump frame), id)` (the
/// atomic frame's LTM first) above 0 (ordered, unrounded ST0) skips the atomic; otherwise
/// `AtomicDefaultRenderCallBack`. Returns the atomic.
#[hook(addr = 0x733420, conv = cdecl, name = "CVisibilityPlugins::RenderVehicleHiDetailCB_BigVehicle",
       expect = "D9 05 24 80 C8 00 53", unverified = "samp-rw-1", route = "L2", hazards = "none", recs = "driving")]
pub fn render_vehicle_hi_detail_cb_big_vehicle(atomic: GPtr<u8>) -> GPtr<u8> {
    if beyond(BIG_VEHICLE_LOD0_DIST) {
        return atomic;
    }
    set_unique_materials_by_lod1(atomic);
    let cf = clump_frame(atomic);
    if !dist().fcom(k(NEAR_DIST)).test_any(0x41)
        && rd::<u8>(atomic, g::<i32>(ATOMIC_PLUGIN_OFFSET).read() as isize + 3) & 4 == 0
        && !k(ANGLE_TO_CAMERA).fcom(k(K_ANGLE)).test_parity(5)
    {
        let a = frame_get_ltm(rd::<GPtr<u8>>(atomic, 4));
        let id = atomic_id(atomic);
        let b = frame_get_ltm(cf);
        let dot = X87::from_f64_exact(dot_with_camera(a, b, id));
        if !dot.fcom(k(K_ZERO)).test_any(0x41) {
            return atomic;
        }
    }
    atomic_default_render(atomic);
    atomic
}

/// `CVisibilityPlugins::RenderVehicleLoDetailCB_Boat(RpAtomic*)` (0x7334F0, cdecl): below the boat LOD distance
/// (C0 set, which includes unordered) nothing; otherwise the unique-materials flag is set and the atomic renders
/// with the clump's alpha (`clump + plugin offset + 4`): `AtomicDefaultRenderCallBack` at 0xFF, else
/// `RenderAlphaAtomic(atomic, alpha)`. Returns the atomic.
#[hook(addr = 0x7334F0, conv = cdecl, name = "CVisibilityPlugins::RenderVehicleLoDetailCB_Boat",
       expect = "D9 05 24 80 C8 00 56", unverified = "samp-rw-1", route = "L2", hazards = "none", recs = "air-water")]
pub fn render_vehicle_lo_detail_cb_boat(atomic: GPtr<u8>) -> GPtr<u8> {
    if dist().fcom(k(BOAT_LOD_DIST)).c0() {
        return atomic;
    }
    let f = atomic.byte_add(g::<i32>(ATOMIC_PLUGIN_OFFSET).read() as isize + 3);
    f.write(f.read() | 0x20);
    let alpha = rd::<u32>(rd::<GPtr<u8>>(atomic, 0x3C), g::<i32>(CLUMP_PLUGIN_OFFSET).read() as isize + 4);
    if alpha == 0xFF {
        atomic_default_render(atomic);
    } else {
        render_alpha_atomic(atomic, alpha);
    }
    atomic
}

/// `CVisibilityPlugins::RenderVehicleHiDetailCB_Boat(RpAtomic*)` (0x733550, cdecl): below the boat LOD distance
/// only; the unique-materials flag by `ms_vehicleLod1Dist`; `AtomicDefaultRenderCallBack`. Returns the atomic.
#[hook(addr = 0x733550, conv = cdecl, name = "CVisibilityPlugins::RenderVehicleHiDetailCB_Boat",
       expect = "D9 05 24 80 C8 00", unverified = "samp-rw-1", route = "L2", hazards = "none", recs = "air-water")]
pub fn render_vehicle_hi_detail_cb_boat(atomic: GPtr<u8>) -> GPtr<u8> {
    if beyond(BOAT_LOD_DIST) {
        return atomic;
    }
    set_unique_materials_by_lod1(atomic);
    atomic_default_render(atomic);
    atomic
}

/// `CVisibilityPlugins::InsertEntityIntoUnderwaterList(CEntity*, float dist)` (0x733D90, cdecl, AL): `{ entity,
/// RenderEntity (0x732B40), dist }` into the sorted underwater list 0xC88178; true when inserted (`setne al`).
#[hook(addr = 0x733D90, conv = cdecl, name = "CVisibilityPlugins::InsertEntityIntoUnderwaterList",
       expect = "83 EC 0C 8B 4C 24 14", unverified = "samp-rw-1", route = "L2", hazards = "none", recs = "air-water",
       note = "AL result; the upper EAX bits are the inserted node's in the original")]
pub fn insert_entity_into_underwater_list(entity: GPtr<u8>, dist: u32) -> u8 {
    u8::from(!insert(UNDERWATER_LIST, entity, RENDER_ENTITY, dist).is_null())
}

/// The world-space offset of the atomic from the camera, `LTM(atomic frame).pos - *ms_pCameraPosn` (unrounded),
/// with the clump frame's LTM (the atomic's LTM is fetched first).
fn rotor_frames(atomic: GPtr<u8>) -> ([X87; 3], GPtr<u8>) {
    let cf = clump_frame(atomic);
    let a = frame_get_ltm(rd::<GPtr<u8>>(atomic, 4));
    let b = frame_get_ltm(cf);
    let cam = g::<GPtr<f32>>(CAMERA_POSN).read();
    let d = [
        X87::from_f32(rd::<f32>(a, 0x30)).fsub_m(cam.read()),
        X87::from_f32(rd::<f32>(a, 0x34)).fsub_m(cam.wrapping_index(1).read()),
        X87::from_f32(rd::<f32>(a, 0x38)).fsub_m(cam.wrapping_index(2).read()),
    ];
    (d, b)
}

/// `CVisibilityPlugins::RenderHeliRotorAlphaCB(RpAtomic*)` (0x7340B0, cdecl): below `ms_bigVehicleLod0Dist` only;
/// `key = f32((at.y * dy + at.x * dx + at.z * dz) * 20.0f + dist)` with `at` the clump frame LTM's at vector and `d`
/// the offset from the camera; `{ atomic, RenderAtomic, key }` into the alpha list, rendered at once when full.
/// Returns the atomic.
#[hook(addr = 0x7340B0, conv = cdecl, name = "CVisibilityPlugins::RenderHeliRotorAlphaCB",
       expect = "D9 05 24 80 C8 00 83 EC 0C", unverified = "samp-rw-1", route = "L2", hazards = "none", recs = "air-water")]
pub fn render_heli_rotor_alpha_cb(atomic: GPtr<u8>) -> GPtr<u8> {
    if beyond(BIG_VEHICLE_LOD0_DIST) {
        return atomic;
    }
    let ([dx, dy, dz], m) = rotor_frames(atomic);
    let s = X87::from_f32(rd::<f32>(m, 0x24)).fmul(dy).fadd(X87::from_f32(rd::<f32>(m, 0x20)).fmul(dx))
        .fadd(X87::from_f32(rd::<f32>(m, 0x28)).fmul(dz));
    let key = s.fmul_m(kf(K_ROTOR)).fadd_m(kf(DISTANCE_FROM_CAMERA)).to_f32();
    if insert(ALPHA_ATOMIC_LIST, atomic, RENDER_ATOMIC, key.to_bits()).is_null() {
        atomic_default_render(atomic);
    }
    atomic
}

/// `CVisibilityPlugins::RenderHeliTailRotorAlphaCB(RpAtomic*)` (0x734170, cdecl): below the boat LOD distance
/// (0xC88040) only; `key = f32(-(right.y * dy + dx * right.x + right.z * dz) - (up.y * dy + up.x * dx + up.z * dz) +
/// dist)` with `right`/`up` of the clump frame's LTM; `{ atomic, RenderAtomic, key }` into the alpha list, rendered
/// at once when full. Returns the atomic.
#[hook(addr = 0x734170, conv = cdecl, name = "CVisibilityPlugins::RenderHeliTailRotorAlphaCB",
       expect = "D9 05 24 80 C8 00 83 EC 0C", unverified = "samp-rw-1", route = "L2", hazards = "none", recs = "air-water")]
pub fn render_heli_tail_rotor_alpha_cb(atomic: GPtr<u8>) -> GPtr<u8> {
    if beyond(BOAT_LOD_DIST) {
        return atomic;
    }
    let ([dx, dy, dz], m) = rotor_frames(atomic);
    let f = |o: isize| X87::from_f32(rd::<f32>(m, o));
    let s1 = f(4).fmul(dy).fadd(dx.fmul(f(0))).fadd(f(8).fmul(dz)).fchs();
    let s2 = f(0x14).fmul(dy).fadd(f(0x10).fmul(dx)).fadd(f(0x18).fmul(dz));
    let key = s1.fsub(s2).fadd_m(kf(DISTANCE_FROM_CAMERA)).to_f32();
    if insert(ALPHA_ATOMIC_LIST, atomic, RENDER_ATOMIC, key.to_bits()).is_null() {
        atomic_default_render(atomic);
    }
    atomic
}

/// `CVisibilityPlugins::RenderVehicleHiDetailAlphaCB_BigVehicle(RpAtomic*)` (0x734370, cdecl): below
/// `ms_bigVehicleLod0Dist` only; the unique-materials flag by `ms_vehicleLod1Dist`; the dot product with the camera
/// vector (as `RenderVehicleHiDetailCB_BigVehicle`) always computed and stored as f32. Unless
/// [`big_vehicle_always_visible`] or the dot is at most 0 (or unordered): id flag 0x80 skips the atomic, and so does
/// `dist * 0.1 < dot * dot` (`fcompp; test ah, 5; jnp`). Otherwise `{ atomic, RenderAtomic, key }` into the alpha
/// list with key `f32(dist - 0.0001f)` for id flag 0x40, else `f32(dist + dot)`; rendered at once when full. Returns
/// the atomic.
#[hook(addr = 0x734370, conv = cdecl, name = "CVisibilityPlugins::RenderVehicleHiDetailAlphaCB_BigVehicle",
       expect = "D9 05 24 80 C8 00 83 EC 10", unverified = "samp-rw-1", route = "L2", hazards = "none", recs = "driving")]
pub fn render_vehicle_hi_detail_alpha_cb_big_vehicle(atomic: GPtr<u8>) -> GPtr<u8> {
    if beyond(BIG_VEHICLE_LOD0_DIST) {
        return atomic;
    }
    set_unique_materials_by_lod1(atomic);
    let cf = clump_frame(atomic);
    let a = frame_get_ltm(rd::<GPtr<u8>>(atomic, 4));
    let id = atomic_id(atomic);
    let b = frame_get_ltm(cf);
    let dot = X87::from_f64_exact(dot_with_camera(a, b, id)).to_f32();
    if !big_vehicle_always_visible(id) && !X87::from_f32(dot).fcom(k(K_ZERO)).test_any(0x41) {
        if id & 0x80 != 0 {
            return atomic;
        }
        let sq = X87::from_f32(dot).fmul_m(dot);
        if !dist().fmul_m(kf(K_TENTH)).fcom(sq).test_parity(5) {
            return atomic;
        }
    }
    let key = if id & 0x40 != 0 { dist().fsub_m(kf(K_TINY)) } else { dist().fadd_m(dot) };
    if insert(ALPHA_ATOMIC_LIST, atomic, RENDER_ATOMIC, key.to_f32().to_bits()).is_null() {
        atomic_default_render(atomic);
    }
    atomic
}

/// `CVisibilityPlugins::RenderVehicleHiDetailAlphaCB_Boat(RpAtomic*)` (0x7344A0, cdecl): below the boat LOD distance
/// only; the unique-materials flag by `ms_vehicleLod1Dist`; with id flag 0x40, `{ atomic, RenderAtomic, dist }` into
/// the boat alpha list 0xC880C8 and done when inserted; otherwise (or when full) `AtomicDefaultRenderCallBack`.
/// Returns the atomic.
#[hook(addr = 0x7344A0, conv = cdecl, name = "CVisibilityPlugins::RenderVehicleHiDetailAlphaCB_Boat",
       expect = "D9 05 24 80 C8 00 83 EC 0C", unverified = "samp-rw-1", route = "L2", hazards = "none", recs = "air-water")]
pub fn render_vehicle_hi_detail_alpha_cb_boat(atomic: GPtr<u8>) -> GPtr<u8> {
    if beyond(BOAT_LOD_DIST) {
        return atomic;
    }
    set_unique_materials_by_lod1(atomic);
    if atomic_id(atomic) & 0x40 != 0
        && !insert(ALPHA_BOAT_LIST, atomic, RENDER_ATOMIC, g::<u32>(DISTANCE_FROM_CAMERA).read()).is_null()
    {
        return atomic;
    }
    atomic_default_render(atomic);
    atomic
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_RENDER_VEHICLE_HI_DETAIL_CB_BIG_VEHICLE,
    HOOK_RENDER_VEHICLE_LO_DETAIL_CB_BOAT,
    HOOK_RENDER_VEHICLE_HI_DETAIL_CB_BOAT,
    HOOK_INSERT_ENTITY_INTO_UNDERWATER_LIST,
    HOOK_RENDER_HELI_ROTOR_ALPHA_CB,
    HOOK_RENDER_HELI_TAIL_ROTOR_ALPHA_CB,
    HOOK_RENDER_VEHICLE_HI_DETAIL_ALPHA_CB_BIG_VEHICLE,
    HOOK_RENDER_VEHICLE_HI_DETAIL_ALPHA_CB_BOAT,
];
