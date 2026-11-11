//! PORT-RENDER-2: `C3dMarker` / `C3dMarkers`. Ported from the 1.0 US bytes; gta-reversed
//! `3dMarker.cpp` / `3dMarkers.cpp` as reading aids.
//!
//! `C3dMarker` (0xA0 bytes, `m_aMarkerArray[32]` at 0xC7DD58): +0 CMatrix (+0x40 attach matrix, +0x44
//! owns-attach flag), +0x48 RpAtomic*, +0x4C RpMaterial*, +0x50 type (u16), +0x52 is-used, +0x53
//! must-be-rendered-this-frame, +0x54 id, +0x58 RGBA, +0x5C pulse period (u16), +0x5E rotate rate (i16),
//! +0x60 start time, +0x64 pulse fraction, +0x68 standard size, +0x6C size, +0x70 brightness, +0x74
//! camera range, +0x78 normal, +0x84 / +0x86 last map-read x / y (u16), +0x88 last map-read z, +0x8C
//! roof height, +0x90 last position, +0x9C on-screen test time.
//!
//! User markers (`ms_user3dMarkers[5]`, 0x1C bytes at 0xC80258): +0 used, +4 position, +0x10 / +0x14 /
//! +0x18 colour (i32). Direction arrows (`ms_directionArrows[5]`, 0x30 bytes at 0xC802E8): +0 used, +4
//! position, +0x10 size, +0x14 direction, +0x20..+0x2C r, g, b, a (i32).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, ftol32, g, jne, jp, k, rd, wr, x};

sa_core::globals! {
    list GLOBALS;
    /// `C3dMarkers::m_aMarkerArray[32]` (C3dMarker, 0xA0 bytes)
    pub static MARKER_ARRAY @ 0xC7DD58: [u8; 32 * 0xA0] = [0; 32 * 0xA0];
    /// `C3dMarkers::m_pRpClumpArray[7]`
    pub static CLUMP_ARRAY @ 0xC7C6DC: [u32; 7] = [0; 7];
    /// `C3dMarkers::NumActiveMarkers`
    pub static NUM_ACTIVE_MARKERS @ 0xC7C6D8: i32 = 0;
    /// `C3dMarkers::m_angleDiamond`
    pub static ANGLE_DIAMOND @ 0xC7C700: f32 = 0.0;
    /// `C3dMarkers::IgnoreRenderLimit`
    pub static IGNORE_RENDER_LIMIT @ 0xC7C704: u8 = 0;
    /// `C3dMarkers::m_user3dMarkerColor` (RwRGBA)
    pub static USER_MARKER_COLOUR @ 0xC7C620: u32 = 0;
    /// `C3dMarkers::ms_user3dMarkers[5]` (0x1C bytes)
    pub static USER_MARKERS @ 0xC80258: [u8; 5 * 0x1C] = [0; 5 * 0x1C];
    /// `C3dMarkers::ms_directionArrows[5]` (0x30 bytes)
    pub static DIRECTION_ARROWS @ 0xC802E8: [u8; 5 * 0x30] = [0; 5 * 0x30];
}

const MARKERS_BASE: usize = 0xC7DD58;
const USER_BASE: usize = 0xC80258;
const ARROWS_BASE: usize = 0xC802E8;
/// `CTimer::ms_fTimeStep` (0xB7CB5C), `CTimer::m_snTimeInMilliseconds` (0xB7CB84).
const TIME_STEP: usize = 0xB7CB5C;
const TIME_MS: usize = 0xB7CB84;
/// `TheCamera`, its `m_mCameraMatrix`-style matrix passed to `IsSphereVisible` (0xB6FA74).
const THE_CAMERA: usize = 0xB6F028;
const CAM_SPHERE_MATRIX: u32 = 0xB6FA74;
/// The alpha byte `PlaceMarkerCone` substitutes (0x8D5D8B, initially 0xFF; `.data`, owned here).
const CONE_ALPHA: usize = 0x8D5D8B;
/// The alpha of the last rendered marker (0x8D5E44, `.data`, initially 0xFF).
const LAST_MARKER_ALPHA: usize = 0x8D5E44;
/// `CHudColours HudColour` (0xBAB22C).
const HUD_COLOUR: usize = 0xBAB22C;
/// `MarkerAtomicCB` (0x722220), `RwHelper::RemoveRefsCB` (0x7226D0), `__dtor_C3dMarkers__m_aMarkerArray`
/// (0x856BD0), `C3dMarker::C3dMarker` (0x720F60), `C3dMarker::~C3dMarker` (0x720F70).
const MARKER_ATOMIC_CB: u32 = 0x722220;
const REMOVE_REFS_CB: u32 = 0x7226D0;
const DTOR_MARKER_ARRAY: u32 = 0x856BD0;
/// `C3dMarkers::User3dMarkerAtomicCB` (0x7210D0).
const USER_MARKER_ATOMIC_CB: u32 = 0x7210D0;
const MARKER_CTOR: u32 = 0x720F60;
const MARKER_DTOR: u32 = 0x720F70;

/// 5.0 (0x858C80), 3.0 (0x858B3C), 10000.0 (0x859AA4), 1.0 (0x858624), 0.05 (0x858C28), 150.0
/// (0x858A28), 1.6 (0x858FA8), 1/3 (0x859040)
const K_FIVE: u32 = 0x40A0_0000;
const K_THREE: u32 = 0x4040_0000;
const K_10000: u32 = 0x461C_4000;
const K_ONE: u32 = 0x3F80_0000;
const K_0_05: u32 = 0x3D4C_CCCD;
const K_150: u32 = 0x4316_0000;
const K_1_6: u32 = 0x3FCC_CCCD;
const K_THIRD: u32 = 0x3EAA_AAAB;
/// pi/180 (0x8595EC), 0.25 (0x858C84)
const K_DEG2RAD: u32 = 0x3C8E_FA35;
const K_0_25: u32 = 0x3E80_0000;
/// 1e-4 (0x858FC4), 1.5 (0x858CE8), 65535.0 (0x8729E8), 0.5 (0x858B8C), 0.1 (0x858B1C), 0.15
/// (0x858FCC), 0.3 (0x858C24), 25.0 (0x858FE8), 0.015 (0x8717A8), 12.0 (0x858CCC), 0.0318...
/// (0x8729E4), 0.65 (0x858F50)
const K_1E_4: u32 = 0x38D1_B717;
const K_1_5: u32 = 0x3FC0_0000;
const K_65535: u32 = 0x477F_FF00;
const K_0_5: u32 = 0x3F00_0000;
const K_0_1: u32 = 0x3DCC_CCCD;
const K_0_15: u32 = 0x3E19_999A;
const K_0_3: u32 = 0x3E99_999A;
const K_25: u32 = 0x41C8_0000;
const K_0_015: u32 = 0x3C75_C290;
const K_12: u32 = 0x4140_0000;
const K_FADE: u32 = 0x3D02_53C9;
const K_0_65: u32 = 0x3F26_6666;
/// 0.8 (0x858C98)
const K_0_8: u32 = 0x3F4C_CCCD;

orig! {
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    pub fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CMatrix::SetUnity()` (0x59AE70)
    pub fn matrix_set_unity(m: GPtr<u8>) = thiscall 0x59AE70 => unported;
    /// `CMatrix::Attach(RwMatrix*, bool owns)` (0x59BD10)
    pub fn matrix_attach(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) = thiscall 0x59BD10 => unported;
    /// `CMatrix::UpdateRW()` (0x59BBB0)
    pub fn matrix_update_rw(m: GPtr<u8>) = thiscall 0x59BBB0 => unported;
    /// `RpClumpForAllAtomics(RpClump*, callback, void* data)` (0x749B70)
    pub fn rp_clump_for_all_atomics(c: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `RpAtomicClone(RpAtomic*)` (0x749E60)
    pub fn rp_atomic_clone(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x749E60 => unported;
    /// `RwFrameCreate()` (0x7F0410)
    pub fn rw_frame_create() -> GPtr<u8> = cdecl 0x7F0410 => unported;
    /// `RpAtomicSetFrame(RpAtomic*, RwFrame*)` (0x74BF20)
    pub fn rp_atomic_set_frame(a: GPtr<u8>, f: GPtr<u8>) -> GPtr<u8> = cdecl 0x74BF20 => unported;
    /// `CVisibilityPlugins::SetAtomicRenderCallback(RpAtomic*, callback)` (0x7328A0)
    pub fn set_atomic_render_callback(a: GPtr<u8>, cb: u32) = cdecl 0x7328A0 => unported;
    /// `RpAtomicDestroy(RpAtomic*)` (0x749DC0)
    pub fn rp_atomic_destroy(a: GPtr<u8>) -> i32 = cdecl 0x749DC0 => unported;
    /// `RwFrameDestroy(RwFrame*)` (0x7F05A0)
    pub fn rw_frame_destroy(f: GPtr<u8>) -> i32 = cdecl 0x7F05A0 => unported;
    /// `RwFrameUpdateObjects(RwFrame*)` (0x7F0910)
    pub fn rw_frame_update_objects(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => unported;
    /// `RwRenderStateSet` (`RwEngineInstance` slot 0x20)
    pub fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwRenderStateGet` (`RwEngineInstance` slot 0x24)
    pub fn rw_render_state_get(state: u32, out: GPtr<u8>) -> i32 = cdecl table 0xC97B24 0x24 => unported;
    /// `RwFrameTranslate(RwFrame*, const RwV3d*, RwOpCombineType)` (0x7F0E30)
    pub fn rw_frame_translate(f: GPtr<u8>, v: GPtr<u8>, combine: u32) -> GPtr<u8> = cdecl 0x7F0E30 => unported;
    /// `RpClumpRender(RpClump*)` (0x749B20)
    pub fn rp_clump_render(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x749B20 => unported;
    /// `CVector::CrossProduct(CVector* out, const CVector* a, const CVector* b)` (0x59C730, returns `out`)
    pub fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => unported;
    /// `CVector::Normalise()` (0x59C910)
    pub fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => unported;
    /// `CMatrix::operator*=(const CMatrix&)` (0x411A80)
    pub fn matrix_mul_assign(m: GPtr<u8>, r: GPtr<u8>) = thiscall 0x411A80 => unported;
    /// `CMatrix::Scale(float)` (0x459350)
    pub fn matrix_scale(m: GPtr<u8>, s: u32) = thiscall 0x459350 => unported;
    /// `CMatrix` per-axis scale `(x, y, z)` (0x5A2E60, unnamed in the catalogue)
    pub fn matrix_scale_xyz(m: GPtr<u8>, sx: u32, sy: u32, sz: u32) = thiscall 0x5A2E60 => unported;
    /// `Light::SetBrightMarkerColours(float)` (0x735BD0)
    pub fn set_bright_marker_colours(b: u32) = cdecl 0x735BD0 => unported;
    /// `Light::ReSetAmbientAndDirectionalColours()` (0x735C40)
    pub fn reset_ambient_and_directional_colours() = cdecl 0x735C40 => unported;
    /// `atomic->renderCallBack(atomic)` (the pointer at RpAtomic +0x48)
    pub fn atomic_render_callback(f: u32, a: GPtr<u8>) -> GPtr<u8> = cdecl dyn => unported;
    /// `CWorld::FindPlayerCentreOfWorld(int32)` (0x56E250, returns a CVector*)
    pub fn find_player_centre_of_world(id: i32) -> GPtr<u8> = cdecl 0x56E250 => unported;
    /// `CWorld::ProcessLineOfSight(start, end, CColPoint&, CEntity*&, buildings, vehicles, peds, objects,
    /// dummies, seeThrough, cameraIgnore, shootThrough)` (0x56BA00)
    pub fn process_line_of_sight(s: GPtr<u8>, e: GPtr<u8>, cp: GPtr<u8>, ent: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32, st: u32, ci: u32, sh: u32) -> u8 = cdecl 0x56BA00 => unported;
    /// `CWaterLevel::GetWaterLevelNoWaves(float x, float y, float z, float* out, float*, float*)` (0x6E8580)
    pub fn get_water_level_no_waves(x: u32, y: u32, z: u32, out: GPtr<u8>, a: u32, b: u32) -> u8 = cdecl 0x6E8580 => unported;
    /// `CMatrix::RotateZ(float)` (0x59B390)
    pub fn matrix_rotate_z(m: GPtr<u8>, a: u32) = thiscall 0x59B390 => unported;
    /// `CMatrix::SetTranslate(float x, float y, float z)` (0x59AF40)
    pub fn matrix_set_translate(m: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x59AF40 => unported;
    /// `C3dMarker::UpdateZCoordinate(CVector, float)` (0x724D40)
    pub fn update_z_coordinate_call(m: GPtr<u8>, x: u32, y: u32, z: u32, d: u32) = thiscall 0x724D40 => unported;
    /// `C3dMarker::AddMarker(...)` (0x722230)
    pub fn add_marker_call(m: GPtr<u8>, id: u32, ty: u32, size: u32, r: u32, g: u32, b: u32, a: u32, pp: u32, pf: u32, rr: u32) -> u8 = thiscall 0x722230 => unported;
    /// `CMatrix::CMatrix(RwMatrix*, bool owns)` (0x59C050)
    pub fn matrix_ctor_rw(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) -> GPtr<u8> = thiscall 0x59C050 => unported;
    /// `CMatrix operator*(const CMatrix&, const CMatrix&)` (0x59BE30, result through `out`)
    pub fn matrix_mul(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59BE30 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0)
    pub fn matrix_assign(m: GPtr<u8>, r: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BBC0 => unported;
    /// `RpClumpDestroy(RpClump*)` (0x74A310)
    pub fn rp_clump_destroy(c: GPtr<u8>) -> i32 = cdecl 0x74A310 => unported;
    /// `CColStore::HasCollisionLoaded(const CVector&, int32 area)` (0x410CE0)
    pub fn has_collision_loaded(p: GPtr<f32>, area: i32) -> u8 = cdecl 0x410CE0 => unported;
    /// `CWorld::FindGroundZFor3DCoord(float x, float y, float z, bool* found, CEntity** out)` (0x5696C0, ST0)
    pub fn find_ground_z_for_3d_coord(x: u32, y: u32, z: f32, found: GPtr<u8>, out: u32) -> X87 = cdecl 0x5696C0 => unported;
    /// `CHudColours::GetRGB(CRGBA* ret, eHudColours)` (0x58FEA0, struct returned through `ret`)
    pub fn hud_get_rgb(this: GPtr<u8>, ret: GPtr<u8>, colour: u32) -> GPtr<u8> = thiscall 0x58FEA0 => unported;
    /// `CGenericGameStorage::LoadDataFromWorkBuffer(void*, int32)` (0x5D1300)
    pub fn load_data_from_work_buffer(d: GPtr<u8>, n: i32) -> u8 = cdecl 0x5D1300 => unported;
    /// `CGenericGameStorage::SaveDataToWorkBuffer(void*, int32)` (0x5D1270)
    pub fn save_data_to_work_buffer(d: GPtr<u8>, n: i32) -> u8 = cdecl 0x5D1270 => unported;
    /// `Light::ActivateDirectional()` (0x735C80)
    pub fn activate_directional() = cdecl 0x735C80 => unported;
    /// `Light::SetAmbientColours(RwRGBAReal*)` (0x735D50)
    pub fn set_ambient_colours(c: u32) = cdecl 0x735D50 => unported;
    /// `Light::SetDirectionalColours(RwRGBAReal*)` (0x735D70)
    pub fn set_directional_colours(c: u32) = cdecl 0x735D70 => unported;
    /// `C3dMarkers::User3dMarkersDraw()` (0x723240)
    pub fn user_3d_markers_draw_call() = cdecl 0x723240 => unported;
    /// `C3dMarkers::DirectionArrowsDraw()` (0x7211F0)
    pub fn direction_arrows_draw_call() = cdecl 0x7211F0 => unported;
    /// `CCamera::IsSphereVisible(const CVector&, float, const CMatrix*)` (0x420C40)
    pub fn is_sphere_visible(cam: GPtr<u8>, c: GPtr<u8>, r: u32, m: u32) -> u8 = thiscall 0x420C40 => unported;
    /// `C3dMarker::Render()` (0x7223D0)
    pub fn marker_render_call(m: GPtr<u8>) = thiscall 0x7223D0 => unported;
    /// `C3dMarkers::PlaceMarker(id, type, CVector& pos, size, r, g, b, a, pulsePeriod, pulseFraction,
    /// rotateRate, nrm x, nrm y, nrm z, zCheck)` (0x725120)
    pub fn place_marker_call(id: u32, ty: u32, pos: u32, size: u32, r: u32, g: u32, b: u32, a: u32, pp: u32, pf: u32, rr: u32, nx: u32, ny: u32, nz: u32, zc: u32) -> GPtr<u8> = cdecl 0x725120 => unported;
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall)
    pub fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// CRT `eh vector destructor iterator` (0x821E02, stdcall)
    pub fn eh_vector_dtor(arr: GPtr<u8>, size: u32, n: u32, dtor: u32) = stdcall 0x821E02 => unported;
    /// CRT `_atexit` (0x821D1E)
    pub fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
    /// `CTxdStore::FindTxdSlot(const char*)` (0x731850)
    pub fn find_txd_slot(name: u32) -> i32 = cdecl 0x731850 => unported;
    /// `CTxdStore::PushCurrentTxd()` (0x7316A0)
    pub fn push_current_txd() = cdecl 0x7316A0 => unported;
    /// `CTxdStore::SetCurrentTxd(int32)` (0x7319C0)
    pub fn set_current_txd(slot: i32) = cdecl 0x7319C0 => unported;
    /// `CTxdStore::PopCurrentTxd()` (0x7316B0)
    pub fn pop_current_txd() = cdecl 0x7316B0 => unported;
    /// `CFileMgr::ChangeDir(const char*)` (0x538730)
    pub fn change_dir(d: u32) = cdecl 0x538730 => unported;
    /// `CModelInfo::GetModelInfo(const char*, int32* index)` (0x4C5940)
    pub fn get_model_info(name: u32, idx: GPtr<i32>) -> GPtr<u8> = cdecl 0x4C5940 => unported;
    /// `CStreaming::RequestModel(int32, int32 flags)` (0x4087E0)
    pub fn request_model(idx: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CStreaming::LoadAllRequestedModels(bool)` (0x40EA10)
    pub fn load_all_requested_models(prio: u32) = cdecl 0x40EA10 => unported;
    /// `CBaseModelInfo::CreateInstance()` (virtual, slot 0x2C)
    pub fn model_create_instance(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x2C => unported;
    /// `CBaseModelInfo::AddRef()` (0x4C4BA0)
    pub fn model_add_ref(mi: GPtr<u8>) = thiscall 0x4C4BA0 => unported;
    /// `RpClumpCreate()` (0x74A290)
    pub fn rp_clump_create() -> GPtr<u8> = cdecl 0x74A290 => unported;
    /// `RwFrameSetIdentity(RwFrame*)` (0x7F10B0)
    pub fn rw_frame_set_identity(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F10B0 => unported;
    /// `RpClumpAddAtomic(RpClump*, RpAtomic*)` (0x74A490)
    pub fn rp_clump_add_atomic(c: GPtr<u8>, a: GPtr<u8>) -> GPtr<u8> = cdecl 0x74A490 => unported;
}

fn marker(i: i32) -> GPtr<u8> {
    GPtr::from_addr(MARKERS_BASE).byte_add(i as isize * 0xA0)
}

fn user(i: i32) -> GPtr<u8> {
    GPtr::from_addr(USER_BASE).byte_add(i as isize * 0x1C)
}

fn arrow(i: i32) -> GPtr<u8> {
    GPtr::from_addr(ARROWS_BASE).byte_add(i as isize * 0x30)
}

/// The inlined `C3dMarker::DeleteMarkerObject` body (also in `Shutdown` / `Render`): id, start time,
/// used / render flags 0, type 0x101, atomic and its frame destroyed, atomic 0.
fn delete_marker_object_body(m: GPtr<u8>) {
    let a = rd::<GPtr<u8>>(m, 0x48);
    wr::<u32>(m, 0x54, 0);
    wr::<u32>(m, 0x60, 0);
    wr::<u8>(m, 0x52, 0);
    wr::<u8>(m, 0x53, 0);
    wr::<u16>(m, 0x50, 0x101);
    let f = rd::<GPtr<u8>>(a, 4);
    rp_atomic_destroy(a);
    rw_frame_destroy(f);
    wr::<u32>(m, 0x48, 0);
}

/// `C3dMarker::C3dMarker()` (0x720F60, thiscall): attach matrix and owns flag 0; returns `this`.
#[hook(addr = 0x720F60, conv = thiscall, name = "C3dMarker::C3dMarker", expect = "8B C1 33 C9 89 48 40")]
pub fn marker_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0x40, 0);
    wr::<u32>(this, 0x44, 0);
    this
}

/// `C3dMarker::~C3dMarker()` (0x720F70): a tail jump to `CMatrix::~CMatrix` (0x59ACD0).
#[hook(addr = 0x720F70, conv = thiscall, name = "C3dMarker::~C3dMarker", expect = "E9 5B 9D E7 FF 90")]
pub fn marker_dtor(this: GPtr<u8>) {
    matrix_dtor(this);
}

/// `C3dMarker::DeleteMarkerObject()` (0x722390).
#[hook(addr = 0x722390, conv = thiscall, name = "C3dMarker::DeleteMarkerObject", expect = "53 56 8B F1 8B 46 48")]
pub fn delete_marker_object(this: GPtr<u8>) {
    delete_marker_object_body(this);
}

/// `C3dMarker::AddMarker(uint32 id, uint16 type, float size, uint8 r, g, b, a, uint16 pulsePeriod, float
/// pulseFraction, int16 rotateRate)` (0x722230, thiscall, AL): id, unity matrix, the atomic of the type's
/// clump (type 2 takes clump 1, type 3 clump 0, otherwise clump `type`; found through `MarkerAtomicCB`)
/// cloned onto a new frame without render callback, geometry flag 0x40 (modulate material colour), the
/// matrix attached to the frame's modelling matrix, material 0, the parameters, start / on-screen-test
/// time = now, last position 0, roof height 65535.0. Returns `atomic != 0`. The original uses its `id`
/// argument slot as the callback's out pointer (stack only).
#[hook(addr = 0x722230, conv = thiscall, name = "C3dMarker::AddMarker", expect = "83 EC 0C 8B 44 24 10")]
#[allow(clippy::too_many_arguments)]
pub fn add_marker(
    this: GPtr<u8>,
    id: u32,
    ty: u32,
    size: u32,
    r: u32,
    gg: u32,
    b: u32,
    a: u32,
    pulse_period: u32,
    pulse_fraction: u32,
    rotate_rate: u32,
) -> u8 {
    wr::<u32>(this, 0x54, id);
    matrix_set_unity(this);
    let ty = ty as u16;
    let mut found = GPtr::<u8>::NULL;
    let clump = match ty {
        2 => CLUMP_ARRAY.get()[1],
        3 => CLUMP_ARRAY.get()[0],
        _ => g::<u32>(0xC7C6DC + usize::from(ty) * 4).read(),
    };
    rp_clump_for_all_atomics(GPtr::from_addr(clump as usize), MARKER_ATOMIC_CB, GPtr::from_raw(&mut found).cast());
    let atomic = rp_atomic_clone(found);
    rp_atomic_set_frame(atomic, rw_frame_create());
    set_atomic_render_callback(atomic, 0);
    let geom = rd::<GPtr<u8>>(atomic, 0x18);
    wr::<u32>(geom, 8, rd::<u32>(geom, 8) | 0x40);
    wr::<GPtr<u8>>(this, 0x48, atomic);
    matrix_attach(this, rd::<GPtr<u8>>(atomic, 4).byte_add(0x10), 0);
    wr::<u32>(this, 0x4C, rd::<GPtr<u32>>(geom, 0x20).read());
    wr::<u32>(this, 0x6C, size);
    wr::<u32>(this, 0x68, size);
    wr::<u8>(this, 0x58, r as u8);
    wr::<u8>(this, 0x59, gg as u8);
    wr::<u8>(this, 0x5A, b as u8);
    wr::<u8>(this, 0x5B, a as u8);
    wr::<u16>(this, 0x5C, pulse_period as u16);
    wr::<u32>(this, 0x64, pulse_fraction);
    wr::<u16>(this, 0x5E, rotate_rate as u16);
    wr::<u32>(this, 0x60, g::<u32>(TIME_MS).read());
    wr::<u32>(this, 0x90, 0);
    wr::<u32>(this, 0x94, 0);
    wr::<u16>(this, 0x50, ty);
    wr::<u32>(this, 0x8C, 0x477F_FF00);
    wr::<u32>(this, 0x98, 0);
    wr::<u32>(this, 0x9C, g::<u32>(TIME_MS).read());
    u8::from(!rd::<GPtr<u8>>(this, 0x48).is_null())
}

/// Whether the cached map-read x / y (u16) equal `_ftol` of the position's x / y (low 16 bits).
fn z_up_to_date(this: GPtr<u8>) -> bool {
    ftol32(x(rd::<f32>(this, 0x30))) as u16 == rd::<u16>(this, 0x84)
        && ftol32(x(rd::<f32>(this, 0x34))) as u16 == rd::<u16>(this, 0x86)
}

/// `C3dMarker::IsZCoordinateUpToDate()` (0x7226A0, thiscall, AL).
#[hook(addr = 0x7226A0, conv = thiscall, name = "C3dMarker::IsZCoordinateUpToDate", expect = "56 8B F1 D9 46 30")]
pub fn is_z_coordinate_up_to_date(this: GPtr<u8>) -> u8 {
    u8::from(z_up_to_date(this))
}

/// `C3dMarker::SetZCoordinateIfNotUpToDate(float z)` (0x724E10, thiscall): position z = `z` (raw).
#[hook(addr = 0x724E10, conv = thiscall, name = "C3dMarker::SetZCoordinateIfNotUpToDate", expect = "56 8B F1 D9 46 30")]
pub fn set_z_coordinate_if_not_up_to_date(this: GPtr<u8>, z: u32) {
    if !z_up_to_date(this) {
        wr::<u32>(this, 0x38, z);
    }
}

/// `C3dMarker::UpdateZCoordinate(CVector point, float zDistance)` (0x724D40, thiscall): when the cache is
/// stale and the marker is within 100 units (2D, `dx^2 + dy^2 < 10000`, ordered) of `point` and collision
/// is loaded there: ground z below (x, y, z + 1) found → z = ground - zDistance * 0.05; the cache x / y
/// updated in any case. The original uses its `point.x` argument slot as the found flag (stack only).
#[hook(addr = 0x724D40, conv = thiscall, name = "C3dMarker::UpdateZCoordinate", expect = "56 8B F1 D9 46 30")]
pub fn update_z_coordinate(this: GPtr<u8>, px: u32, py: u32, _pz: u32, z_dist: u32) {
    if z_up_to_date(this) {
        return;
    }
    let dy = x(f32::from_bits(py)).fsub_m(rd::<f32>(this, 0x34));
    let dx = x(f32::from_bits(px)).fsub_m(rd::<f32>(this, 0x30));
    let s = dx.fmul(dx).fadd(dy.fmul(dy));
    if jp(s, x(k(K_10000)), 0x05) {
        return;
    }
    if has_collision_loaded(this.byte_add(0x30).cast(), 0) == 0 {
        return;
    }
    let z1 = f32_of(x(rd::<f32>(this, 0x38)).fadd_m(k(K_ONE)));
    let mut found = 0u32;
    let gz = find_ground_z_for_3d_coord(
        rd::<u32>(this, 0x30),
        rd::<u32>(this, 0x34),
        z1,
        GPtr::from_raw(&mut found).cast(),
        0,
    );
    if found as u8 != 0 {
        let v = x(f32::from_bits(z_dist)).fmul_m(k(K_0_05));
        wr::<f32>(this, 0x38, f32_of(gz.fsub(v)));
    }
    wr::<u16>(this, 0x84, ftol32(x(rd::<f32>(this, 0x30))) as u16);
    wr::<u16>(this, 0x86, ftol32(x(rd::<f32>(this, 0x34))) as u16);
}

/// `C3dMarkers::Init()` (0x724E40): every marker reset (atomic 0, type 0x101, flags / id / start time 0,
/// colour 0xFFFFFFFF, pulse period 0x400, rotate rate 5, pulse fraction 0.25, sizes / brightness 1.0,
/// camera range and normal 0, normal z 1.0, map-read x 30000, roof height 65535.0, last position 0), the
/// clump array cleared, no active markers, diamond angle 0; then from the "particle" TXD (after
/// `CFileMgr::ChangeDir("\\")`) the models "cylinder" (clump 1), "hoop" (clump 4) and "diamond_3"
/// (clumps 0, 6, 5) are requested, loaded, instanced and wrapped in a new clump each (the atomic's frame
/// reset to identity and made the clump's frame).
#[hook(addr = 0x724E40, conv = cdecl, name = "C3dMarkers::Init", expect = "83 EC 10 53 55")]
pub fn init() {
    for i in 0..32 {
        let m = marker(i);
        wr::<u32>(m, 0x90, 0);
        wr::<u32>(m, 0x48, 0);
        wr::<u16>(m, 0x50, 0x101);
        wr::<u8>(m, 0x52, 0);
        wr::<u8>(m, 0x53, 0);
        wr::<u32>(m, 0x54, 0);
        wr::<u32>(m, 0x58, 0xFFFF_FFFF);
        wr::<u16>(m, 0x5C, 0x400);
        wr::<u16>(m, 0x5E, 5);
        wr::<u32>(m, 0x60, 0);
        wr::<u32>(m, 0x64, 0x3E80_0000);
        wr::<u32>(m, 0x68, K_ONE);
        wr::<u32>(m, 0x6C, K_ONE);
        wr::<u32>(m, 0x70, K_ONE);
        wr::<u32>(m, 0x74, 0);
        wr::<u32>(m, 0x78, 0);
        wr::<u32>(m, 0x7C, 0);
        wr::<u32>(m, 0x80, K_ONE);
        wr::<u16>(m, 0x84, 0x7530);
        wr::<u32>(m, 0x8C, 0x477F_FF00);
        wr::<u32>(m, 0x94, 0);
        wr::<u32>(m, 0x98, 0);
    }
    CLUMP_ARRAY.set([0; 7]);
    NUM_ACTIVE_MARKERS.set(0);
    ANGLE_DIAMOND.set(0.0);
    let slot = find_txd_slot(0x85C648);
    push_current_txd();
    set_current_txd(slot);
    change_dir(0x859F7C);
    let mk = |name: u32| -> GPtr<u8> {
        let mut idx = 0i32;
        let mi = get_model_info(name, GPtr::from_raw(&mut idx));
        request_model(idx, 2);
        load_all_requested_models(0);
        let atomic = model_create_instance(mi);
        model_add_ref(mi);
        let clump = rp_clump_create();
        let f = rd::<GPtr<u8>>(atomic, 4);
        rw_frame_set_identity(f);
        rp_clump_add_atomic(clump, atomic);
        wr::<GPtr<u8>>(clump, 4, f);
        clump
    };
    let c = mk(0x8729D8);
    g::<u32>(0xC7C6E0).write(c.addr() as u32);
    let c = mk(0x8729D0);
    g::<u32>(0xC7C6EC).write(c.addr() as u32);
    let c = mk(0x8729C4).addr() as u32;
    g::<u32>(0xC7C6DC).write(c);
    g::<u32>(0xC7C6F4).write(c);
    g::<u32>(0xC7C6F0).write(c);
    pop_current_txd();
}

/// `C3dMarkers::Shutdown()` (0x722710): every marker with an atomic deleted (inlined
/// `DeleteMarkerObject`); every clump: atomics' model references removed (`RwHelper::RemoveRefsCB`),
/// destroyed, every other entry holding the same clump cleared, then its own entry.
#[hook(addr = 0x722710, conv = cdecl, name = "C3dMarkers::Shutdown", expect = "53 56 57 BE B8 DD C7 00")]
pub fn shutdown() {
    for i in 0..32 {
        let m = marker(i);
        if rd::<u32>(m, 0x48) != 0 {
            delete_marker_object_body(m);
        }
    }
    for i in 0..7usize {
        let e = g::<u32>(0xC7C6DC + i * 4);
        let c = e.read();
        if c == 0 {
            continue;
        }
        rp_clump_for_all_atomics(GPtr::from_addr(c as usize), REMOVE_REFS_CB, GPtr::NULL);
        rp_clump_destroy(GPtr::from_addr(e.read() as usize));
        for j in 0..7usize {
            if j != i && g::<u32>(0xC7C6DC + j * 4).read() == e.read() {
                g::<u32>(0xC7C6DC + j * 4).write(0);
            }
        }
        e.write(0);
    }
}

/// `C3dMarkers::Update()` (0x7227B0): `m_angleDiamond += timeStep * 5`; every used marker's matrix
/// written to its frame (`UpdateRW`), the frame's objects updated, render flag set.
#[hook(addr = 0x7227B0, conv = cdecl, name = "C3dMarkers::Update", expect = "D9 05 5C CB B7 00")]
pub fn update() {
    let a = x(g::<f32>(TIME_STEP).read()).fmul_m(k(K_FIVE)).fadd_m(ANGLE_DIAMOND.get());
    ANGLE_DIAMOND.set(f32_of(a));
    for i in 0..32 {
        let m = marker(i);
        if rd::<u8>(m, 0x52) != 0 {
            matrix_update_rw(m);
            rw_frame_update_objects(rd::<GPtr<u8>>(rd::<GPtr<u8>>(m, 0x48), 4));
            wr::<u8>(m, 0x53, 1);
        }
    }
}

/// `C3dMarkers::ForceRender(bool)` (0x722870): `IgnoreRenderLimit = arg` (stored in the protection chunk
/// 0x401981, which jumps back to the `ret` at 0x722879).
#[hook(addr = 0x722870, conv = cdecl, name = "C3dMarkers::ForceRender", expect = "8A 44 24 04 E9 08 F1 CD FF")]
pub fn force_render(on: u32) {
    IGNORE_RENDER_LIMIT.set(on as u8);
}

/// `C3dMarkers::Render()` (0x725040): `NumActiveMarkers = 0`, directional light on with the ambient /
/// directional colours at 0xC80444 / 0xC80434, user markers drawn; every marker flagged for this frame is
/// rendered when its camera range is < 150 (ordered), the render limit is ignored, or it is type 4, and
/// the camera sees a radius-2 sphere at its position; it is counted and its flags cleared. Unflagged
/// markers with an atomic are deleted. Tail-calls `DirectionArrowsDraw`.
#[hook(addr = 0x725040, conv = cdecl, name = "C3dMarkers::Render", expect = "53 56 33 DB 57")]
pub fn render() {
    NUM_ACTIVE_MARKERS.set(0);
    activate_directional();
    set_ambient_colours(0xC80444);
    set_directional_colours(0xC80434);
    user_3d_markers_draw_call();
    for i in 0..32 {
        let m = marker(i);
        if rd::<u8>(m, 0x53) != 0 {
            let near = !jp(x(rd::<f32>(m, 0x74)), x(k(K_150)), 0x05);
            if (near || IGNORE_RENDER_LIMIT.get() != 0 || rd::<u16>(m, 0x50) == 4)
                && is_sphere_visible(GPtr::from_addr(THE_CAMERA), m.byte_add(0x30), 0x4000_0000, CAM_SPHERE_MATRIX) != 0
            {
                marker_render_call(m);
            }
            let n = NUM_ACTIVE_MARKERS.get();
            wr::<u8>(m, 0x52, 0);
            wr::<u8>(m, 0x53, 0);
            NUM_ACTIVE_MARKERS.set(n.wrapping_add(1));
        } else if rd::<u32>(m, 0x48) != 0 {
            delete_marker_object_body(m);
        }
    }
    direction_arrows_draw_call();
}

/// Marker types 0 / 3 / 5 / 6 (the "ground" markers `PlaceMarker` may recycle by camera range).
fn is_0356(t: u16) -> bool {
    matches!(t, 0 | 3 | 5 | 6)
}

/// `PlaceMarker`'s standard size for types 0 / 3 / 5 / 6: below 25 units the size shrinks (by 0.3 within
/// 5 units, else by `(25 - dist) * size * 0.015`); otherwise the size itself (raw).
fn place_std_size(m: GPtr<u8>, t: u16, dist: f32, size: u32) {
    if !is_0356(t) {
        return;
    }
    let s = f32::from_bits(size);
    if !jp(x(dist), x(k(K_25)), 0x05) {
        let v = if !jne(x(dist), x(k(K_FIVE)), 0x41) {
            x(k(K_25)).fsub_m(dist).fmul_m(s).fmul_m(k(K_0_015)).fsubr_m(s)
        } else {
            x(s).fmul_m(k(K_0_3)).fsubr_m(s)
        };
        wr::<f32>(m, 0x68, f32_of(v));
    } else {
        wr::<u32>(m, 0x68, size);
    }
}

/// `PlaceMarker`'s alpha for types other than 3: fades in over 12 units beyond `size + 1`
/// (`(1 - (size + 12 - dist) * 0.0318...) * a`), 0.65 * a within `size + 1`, `a` beyond `size + 12`.
fn place_alpha(m: GPtr<u8>, t: u16, dist: f32, size: u32, a: u8) {
    if t == 3 {
        return;
    }
    let s = f32::from_bits(size);
    let tt = f32_of(x(s).fadd_m(k(K_12)));
    let v = if !jp(x(dist), x(tt), 0x05) {
        if !jp(x(s).fadd_m(k(K_ONE)), x(dist), 0x05) {
            ftol32(x(tt).fsub_m(dist).fmul_m(k(K_FADE)).fsubr_m(k(K_ONE)).fmul(crate::common::xi(i32::from(a)))) as u8
        } else {
            ftol32(crate::common::xi(i32::from(a)).fmul_m(k(K_0_65))) as u8
        }
    } else {
        a
    };
    wr::<u8>(m, 0x5B, v);
}

/// `PlaceMarker`'s water-level fix for type 1: the water level (no waves) under the marker when found
/// and not below it, else `UpdateZCoordinate(player centre, size)`.
fn place_water(m: GPtr<u8>, pc: [u32; 3], size: u32) {
    let mut out = 0u32;
    let found = get_water_level_no_waves(
        rd::<u32>(m, 0x30),
        rd::<u32>(m, 0x34),
        rd::<u32>(m, 0x38),
        GPtr::from_raw(&mut out).cast(),
        0,
        0,
    );
    if found != 0 && !jne(x(f32::from_bits(out)), x(rd::<f32>(m, 0x38)), 0x01) {
        wr::<u32>(m, 0x38, out);
    } else {
        update_z_coordinate_call(m, pc[0], pc[1], pc[2], size);
    }
}

/// `C3dMarkers::PlaceMarker(uint32 id, uint16 type, CVector& pos, float size, uint8 r, g, b, a, uint16
/// pulsePeriod, float pulseFraction, int16 rotateRate, float nx, ny, nz, bool zCheck)` (0x725120):
/// returns the marker placed this frame, or null.
///
/// The 2D distance to `FindPlayerCentreOfWorld(0)` (f32; type 2: * 0.25; types other than 0..6: null).
/// Slot: an unused-this-frame marker with this id, else a free one (type 0x101), else (types 0 / 3 /
/// 5 / 6 only) the 0 / 3 / 5 / 6 marker with the largest camera range above the distance (made free).
/// Its camera range = the distance. Same id and type: types 0 / 5 re-measure the roof (a line of sight
/// from 1.5 below `pos` up to `pos`, buildings only) when the last position is near the origin
/// (squared length < 1e-4) or 2 s have passed, and `pos` changed; under a roof the size halves and
/// `pos.z` = roof - bounding radius * 0.1. Types 5 / 6 bob `pos.z` by `sin(diamond angle) * (0.15
/// under a roof, else 0.3)`. Then standard size, alpha, size pulse (`std - sin(0.0) * std *
/// pulseFraction`: the original's constant is 0.0), z-rotation by `rotateRate * timeStep` degrees
/// (position x / y kept, z kept only when the z cache is stale), the position by type (0 / 1 / 5 / 6:
/// x, y through the x87, z when the cache is stale; 4 / 2: raw, type 2 with `zCheck` updating z from the
/// ground (distance 10); type 1: water level). Otherwise the old marker object is deleted (when it had
/// an id), the z cache reset (x = 30000), types 5 / 6 bob `pos.z` by 0.3, the matrix translated to
/// `pos`, `AddMarker`, type 1's water level, `UpdateRW`, standard size and alpha. Both paths set the
/// normal and mark the marker used. The original writes `pos.z` (the caller's vector) and uses its
/// `type` / `pos` / `size` argument slots as scratch.
#[hook(addr = 0x725120, conv = cdecl, name = "C3dMarkers::PlaceMarker", expect = "83 EC 58 53 55")]
#[allow(clippy::too_many_arguments)]
pub fn place_marker(
    id: u32,
    ty: u32,
    pos: GPtr<f32>,
    size: u32,
    r: u32,
    gg: u32,
    b: u32,
    a: u32,
    pp: u32,
    pf: u32,
    rr: u32,
    nx: u32,
    ny: u32,
    nz: u32,
    zc: u32,
) -> GPtr<u8> {
    let p = find_player_centre_of_world(0);
    let pc = [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)];
    let pw = |i: i32| pos.wrapping_index(i);
    let dy = x(pw(1).read()).fsub_m(f32::from_bits(pc[1]));
    let dx = x(pw(0).read()).fsub_m(f32::from_bits(pc[0]));
    let mut dist = f32_of(dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt());
    let t = ty as u16;
    if t == 2 {
        dist = f32_of(x(dist).fmul_m(k(K_0_25)));
    } else if !matches!(t, 0 | 1 | 3 | 4 | 5 | 6) {
        return GPtr::NULL;
    }
    let mut found = (0..32).map(marker).find(|&m| rd::<u8>(m, 0x52) == 0 && rd::<u32>(m, 0x54) == id);
    if found.is_none() {
        found = (0..32).map(marker).find(|&m| rd::<u16>(m, 0x50) == 0x101);
    }
    let m = match found {
        Some(m) => m,
        None => {
            if !is_0356(t) {
                return GPtr::NULL;
            }
            let mut best = GPtr::<u8>::NULL;
            for i in 0..32 {
                let c = marker(i);
                if jp(x(dist), x(rd::<f32>(c, 0x74)), 0x05) || !is_0356(rd::<u16>(c, 0x50)) {
                    continue;
                }
                if !best.is_null() && jne(x(rd::<f32>(c, 0x74)), x(rd::<f32>(best, 0x74)), 0x41) {
                    continue;
                }
                best = c;
            }
            if best.is_null() {
                return GPtr::NULL;
            }
            wr::<u16>(best, 0x50, 0x101);
            best
        }
    };
    let old_id = rd::<u32>(m, 0x54);
    wr::<f32>(m, 0x74, dist);
    if old_id == id && rd::<u16>(m, 0x50) == t {
        let mut size = size;
        if t == 0 || t == 5 {
            let radius = rd::<f32>(rd::<GPtr<u8>>(m, 0x48), 0x28);
            let (lx, ly, lz) = (x(rd::<f32>(m, 0x90)), x(rd::<f32>(m, 0x94)), x(rd::<f32>(m, 0x98)));
            let l2 = lx.fmul(lx).fadd(ly.fmul(ly)).fadd(lz.fmul(lz));
            let now = g::<u32>(TIME_MS).read();
            if !jp(l2, x(k(K_1E_4)), 0x05) || now.wrapping_sub(rd::<u32>(m, 0x9C)) >= 2000 {
                wr::<u32>(m, 0x9C, now);
                let same = |o: isize, i: i32| !jp(x(rd::<f32>(m, o)), x(pw(i).read()), 0x44);
                if !(same(0x90, 0) && same(0x94, 1) && same(0x98, 2)) {
                    let pr = pos.cast::<u32>();
                    let (px, py, pz) = (pr.read(), pr.wrapping_index(1).read(), pr.wrapping_index(2).read());
                    wr::<u32>(m, 0x90, px);
                    wr::<u32>(m, 0x94, py);
                    wr::<u32>(m, 0x98, pz);
                    let mut end = [px, py, pz];
                    let mut start =
                        [px, f32_of(x(pw(1).read())).to_bits(), f32_of(x(pw(2).read()).fsub_m(k(K_1_5))).to_bits()];
                    let mut cp = [0u32; 0x2C / 4];
                    let mut ent = 0u32;
                    let hit = process_line_of_sight(
                        GPtr::from_raw(start.as_mut_ptr().cast()),
                        GPtr::from_raw(end.as_mut_ptr().cast()),
                        GPtr::from_raw(cp.as_mut_ptr().cast()),
                        GPtr::from_raw(&mut ent).cast(),
                        1,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                    );
                    wr::<u32>(m, 0x8C, if hit != 0 { cp[2] } else { K_65535 });
                }
            }
            if !jp(x(rd::<f32>(m, 0x8C)), x(k(K_65535)), 0x05) {
                size = f32_of(x(f32::from_bits(size)).fmul_m(k(K_0_5))).to_bits();
                pw(2).write(f32_of(x(radius).fmul_m(k(K_0_1)).fsubr_m(rd::<f32>(m, 0x8C))));
            }
        }
        if t == 5 || t == 6 {
            let roof = !jp(x(rd::<f32>(m, 0x8C)), x(k(K_65535)), 0x05);
            let s = x(ANGLE_DIAMOND.get()).fmul_m(k(K_DEG2RAD)).fsin();
            let v = if roof { s.fmul_m(k(K_0_15)) } else { s.fmul_m(k(K_0_3)) };
            pw(2).write(f32_of(v.fadd_m(pw(2).read())));
        }
        place_std_size(m, t, dist, size);
        place_alpha(m, t, dist, size, a as u8);
        let std = x(rd::<f32>(m, 0x68));
        let pulse = X87::from_f64_exact(0.0).fsin().fmul(std).fmul_m(f32::from_bits(pf));
        wr::<f32>(m, 0x6C, f32_of(std.fsub(pulse)));
        let rate = rd::<i16>(m, 0x5E);
        if rate != 0 {
            let ang = f32_of(
                crate::common::xi(i32::from(rate)).fmul_m(g::<f32>(TIME_STEP).read()).fmul_m(k(K_DEG2RAD)),
            );
            let sv = [rd::<u32>(m, 0x30), rd::<u32>(m, 0x34), rd::<u32>(m, 0x38)];
            matrix_rotate_z(m, ang.to_bits());
            wr::<u32>(m, 0x30, sv[0]);
            wr::<u32>(m, 0x34, sv[1]);
            if !(ftol32(x(f32::from_bits(sv[0]))) as u16 == rd::<u16>(m, 0x84)
                && ftol32(x(f32::from_bits(sv[1]))) as u16 == rd::<u16>(m, 0x86))
            {
                wr::<u32>(m, 0x38, sv[2]);
            }
        }
        if matches!(t, 0 | 1 | 5 | 6) {
            wr::<u32>(m, 0x30, pos.cast::<u32>().read());
            let y = f32_of(x(pw(1).read()));
            wr::<f32>(m, 0x34, y);
            let z = x(pw(2).read());
            if !(ftol32(x(rd::<f32>(m, 0x30))) as u16 == rd::<u16>(m, 0x84)
                && ftol32(x(y)) as u16 == rd::<u16>(m, 0x86))
            {
                wr::<f32>(m, 0x38, f32_of(z));
            }
        }
        let pr = pos.cast::<u32>();
        if t == 4 || t == 2 {
            wr::<u32>(m, 0x30, pr.read());
            let y = pr.wrapping_index(1).read();
            wr::<u32>(m, 0x34, y);
            if t == 2 && zc as u8 != 0 {
                if !(ftol32(x(rd::<f32>(m, 0x30))) as u16 == rd::<u16>(m, 0x84)
                    && ftol32(x(f32::from_bits(y))) as u16 == rd::<u16>(m, 0x86))
                {
                    wr::<u32>(m, 0x38, pr.wrapping_index(2).read());
                }
                update_z_coordinate_call(
                    m,
                    pr.read(),
                    pr.wrapping_index(1).read(),
                    pr.wrapping_index(2).read(),
                    0x4120_0000,
                );
            } else {
                wr::<u32>(m, 0x38, pr.wrapping_index(2).read());
            }
        } else if t == 1 {
            place_water(m, pc, size);
        }
    } else {
        if old_id != 0 {
            delete_marker_object_body(m);
        }
        wr::<u16>(m, 0x84, 0x7530);
        if t == 5 || t == 6 {
            let s = x(ANGLE_DIAMOND.get()).fmul_m(k(K_DEG2RAD)).fsin();
            pw(2).write(f32_of(s.fmul_m(k(K_0_3)).fadd_m(pw(2).read())));
        }
        let pr = pos.cast::<u32>();
        matrix_set_translate(m, pr.read(), pr.wrapping_index(1).read(), pr.wrapping_index(2).read());
        add_marker_call(m, id, ty, size, r, gg, b, a, pp, pf, rr);
        if t == 1 {
            place_water(m, pc, size);
        }
        matrix_update_rw(m);
        place_std_size(m, t, dist, size);
        place_alpha(m, t, dist, size, a as u8);
    }
    wr::<u32>(m, 0x78, nx);
    wr::<u32>(m, 0x7C, ny);
    wr::<u32>(m, 0x80, nz);
    wr::<u8>(m, 0x52, 1);
    m
}

/// `C3dMarkers::PlaceMarkerCone(uint32 id, CVector& pos, float size, uint8 r, g, b, a, uint16
/// pulsePeriod, float pulseFraction, bool isCheckpoint)` (0x726D40): nothing within 1.6 of the camera
/// (distance `(dz^2 + dx^2) + dy^2`, `<` ordered); else `PlaceMarker(id, isCheckpoint ? 5 : 6, pos, size,
/// r, g, b, a', pulsePeriod, pulseFraction, 0, 0, 0, 0, 0)` where `a'` is the caller's `a` slot with its
/// low byte replaced by the byte at 0x8D5D8B (the original stores it into that slot).
#[hook(addr = 0x726D40, conv = cdecl, name = "C3dMarkers::PlaceMarkerCone", expect = "A1 3C F0 B6 00")]
#[allow(clippy::too_many_arguments)]
pub fn place_marker_cone(id: u32, pos: GPtr<f32>, size: u32, r: u32, gg: u32, b: u32, a: u32, pp: u32, pf: u32, cp: u32) {
    let c = sa_world::world::common::cam_pos();
    let dx = x(pos.read()).fsub_m(c.read());
    let dy = x(pos.wrapping_index(1).read()).fsub_m(c.wrapping_index(1).read());
    let dz = x(pos.wrapping_index(2).read()).fsub_m(c.wrapping_index(2).read());
    let d = dz.fmul(dz).fadd(dx.fmul(dx)).fadd(dy.fmul(dy)).fsqrt();
    if !jp(d, x(k(K_1_6)), 0x05) {
        return;
    }
    let a2 = (a & 0xFFFF_FF00) | u32::from(g::<u8>(CONE_ALPHA).read());
    let ty = if cp as u8 != 0 { 5 } else { 6 };
    place_marker_call(id, ty, pos.addr() as u32, size, r, gg, b, a2, pp, pf, 0, 0, 0, 0, 0);
}

/// `C3dMarkers::PlaceMarkerSet(uint32 id, uint16 type, CVector& pos, float size, uint8 r, g, b, a,
/// uint16 pulsePeriod, float pulseFraction)` (0x725BA0): `PlaceMarker(id, type, pos, size, r, g, b,
/// _ftol(a * 1/3), pulsePeriod, pulseFraction, 1, 0, 0, 0, 0)`. The entry jumps into the protection
/// chunk 0x402085 (`movzx edx, byte [esp+0x20]`), which jumps back to the body at 0x725BA5; the original
/// stores the zero-extended `a` into its own argument slot.
#[hook(addr = 0x725BA0, conv = cdecl, name = "C3dMarkers::PlaceMarkerSet", expect = "E9 E0 C4 CD FF 8B 44 24 28")]
#[allow(clippy::too_many_arguments)]
pub fn place_marker_set(id: u32, ty: u32, pos: u32, size: u32, r: u32, gg: u32, b: u32, a: u32, pp: u32, pf: u32) {
    let a3 = ftol32(crate::common::xi(i32::from(a as u8)).fmul_m(k(K_THIRD))) as u32;
    place_marker_call(id, ty, pos, size, r, gg, b, a3, pp, pf, 1, 0, 0, 0, 0);
}

/// `C3dMarker::Render()` (0x7223D0, thiscall): nothing without an atomic. Cull mode none for type 1, else
/// back; the marker matrix written to its frame; a local matrix attached (not owned) to the marker's
/// attach matrix; types 4 / 3 multiply it by a rotation built from the normal (unless the normal is
/// exactly (0, 0, 1): x / y compared with `== 0.0`, z by its bits): right = normalised `up x n`, forward =
/// normalised `n x right`, up = n. Scaled by the size (type 2 additionally (1, 1, 20) via 0x5A2E60),
/// written to the frame, the frame's objects updated. Types 0 / 5 / 6 force alpha 255; an alpha equal to
/// the byte at 0x8D5E44 is stepped by one (down when > 0x7F, else up); 0x8D5E44 = the alpha. Material
/// colour = the marker colour, bright-marker light colours from the brightness, z-write off (type 3 also
/// cull none), `atomic->renderCallBack(atomic)`, z-write on, light colours reset. The local matrices are
/// destroyed (`~CMatrix`); the SEH frame is not reproduced (the callees do not throw).
#[hook(addr = 0x7223D0, conv = thiscall, name = "C3dMarker::Render", expect = "64 A1 00 00 00 00")]
pub fn marker_render(this: GPtr<u8>) {
    if rd::<u32>(this, 0x48) == 0 {
        return;
    }
    rw_render_state_set(0x14, if rd::<u16>(this, 0x50) == 1 { 1 } else { 2 });
    matrix_update_rw(this);
    let mut m = [0u32; 0x12];
    let mp = GPtr::<u8>::from_raw(m.as_mut_ptr().cast());
    matrix_attach(mp, rd::<GPtr<u8>>(this, 0x40), 0);
    let ty = rd::<u16>(this, 0x50);
    if ty == 4 || ty == 3 {
        let mut r = [0u32; 0x12];
        let rp = GPtr::<u8>::from_raw(r.as_mut_ptr().cast());
        matrix_set_unity(rp);
        let n = this.byte_add(0x78);
        // `fcomp 0.0` + `test ah, 0x44` + `jp`: taken unless equal
        let is_zero = |v: f32| !jp(x(v), x(0.0), 0x44);
        if !(is_zero(rd::<f32>(this, 0x78)) && is_zero(rd::<f32>(this, 0x7C)) && rd::<u32>(this, 0x80) == K_ONE) {
            let mut up = [0u32, 0, K_ONE];
            let mut tmp = [0u32; 3];
            let tp = GPtr::<u8>::from_raw(tmp.as_mut_ptr().cast());
            let p = cross_product(tp, GPtr::from_raw(up.as_mut_ptr().cast()), n);
            let mut v1 = [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)];
            let v1p = GPtr::<u8>::from_raw(v1.as_mut_ptr().cast());
            vector_normalise(v1p);
            let p = cross_product(tp, n, v1p);
            let mut v2 = [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)];
            let v2p = GPtr::<u8>::from_raw(v2.as_mut_ptr().cast());
            vector_normalise(v2p);
            for i in 0..3 {
                wr::<u32>(rp, i * 4, rd::<u32>(v1p, i * 4));
                wr::<u32>(rp, 0x10 + i * 4, rd::<u32>(v2p, i * 4));
                wr::<u32>(rp, 0x20 + i * 4, rd::<u32>(this, 0x78 + i * 4));
            }
        }
        matrix_mul_assign(mp, rp);
        matrix_dtor(rp);
    }
    matrix_scale(mp, rd::<u32>(this, 0x6C));
    if rd::<u16>(this, 0x50) == 2 {
        matrix_scale_xyz(mp, K_ONE, K_ONE, 0x41A0_0000);
    }
    matrix_update_rw(mp);
    rw_frame_update_objects(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x48), 4));
    let ty = rd::<u16>(this, 0x50);
    if ty == 0 || ty == 5 || ty == 6 {
        wr::<u8>(this, 0x5B, 0xFF);
    }
    let a = rd::<u8>(this, 0x5B);
    if a == g::<u8>(LAST_MARKER_ALPHA).read() {
        wr::<u8>(this, 0x5B, if a > 0x7F { a - 1 } else { a + 1 });
    }
    g::<u8>(LAST_MARKER_ALPHA).write(rd::<u8>(this, 0x5B));
    wr::<u32>(rd::<GPtr<u8>>(this, 0x4C), 4, rd::<u32>(this, 0x58));
    set_bright_marker_colours(rd::<u32>(this, 0x70));
    rw_render_state_set(8, 0);
    if rd::<u16>(this, 0x50) == 3 {
        rw_render_state_set(0x14, 1);
    }
    let atomic = rd::<GPtr<u8>>(this, 0x48);
    atomic_render_callback(rd::<u32>(atomic, 0x48), atomic);
    rw_render_state_set(8, 1);
    reset_ambient_and_directional_colours();
    matrix_dtor(mp);
}

/// `C3dMarkers::DirectionArrowsDraw()` (0x7211F0): for each used arrow: the first one saves the cull /
/// z-test / z-write / fog states (into 0xC4028C / 0xC40290 / 0xC40294 / 0xC4029C) and sets cull none,
/// z-test on, z-write off, fog off. Diamond clump 5's frame is reset and translated (replace) to the
/// arrow position; a matrix on the frame's modelling matrix is written back (`UpdateRW`); a local
/// matrix attached to it is multiplied by the rotation built from the direction (as in
/// `C3dMarker::Render`: skipped when the direction equals (0, 0, 1), all three compared with the x87,
/// unlike `Render`'s bit test of z) through `CMatrix operator*` and `operator=`; its three axes are
/// scaled by the arrow size, written to the frame, and the clump rendered black (alpha 255); then the
/// axes are scaled by 0.8 and the clump rendered in the arrow colour. The arrow is marked unused. The
/// saved states are restored when any arrow was drawn. The local matrices are destroyed (`~CMatrix`);
/// the SEH frame is not reproduced.
#[hook(addr = 0x7211F0, conv = cdecl, name = "C3dMarkers::DirectionArrowsDraw", expect = "6A FF 68 A9 8C 84 00")]
pub fn direction_arrows_draw() {
    let mut saved = false;
    for i in 0..5 {
        let e = arrow(i);
        if rd::<u8>(e, 0) == 0 {
            continue;
        }
        if !saved {
            rw_render_state_get(0x14, GPtr::from_addr(0xC4028C));
            rw_render_state_get(6, GPtr::from_addr(0xC40290));
            rw_render_state_get(8, GPtr::from_addr(0xC40294));
            rw_render_state_get(0xC, GPtr::from_addr(0xC4029C));
            rw_render_state_set(0x14, 1);
            rw_render_state_set(6, 1);
            rw_render_state_set(8, 0);
            rw_render_state_set(0xC, 0);
            saved = true;
        }
        let clump = GPtr::<u8>::from_addr(g::<u32>(0xC7C6F0).read() as usize);
        let mut pos = [rd::<u32>(e, 4), rd::<u32>(e, 8), rd::<u32>(e, 0xC)];
        let frame = rd::<GPtr<u8>>(clump, 4);
        rw_frame_set_identity(frame);
        rw_frame_translate(frame, GPtr::from_raw(pos.as_mut_ptr().cast()), 0);
        let mut dir = [rd::<u32>(e, 0x14), rd::<u32>(e, 0x18), rd::<u32>(e, 0x1C)];
        let dp = GPtr::<u8>::from_raw(dir.as_mut_ptr().cast());
        let mut fm = [0u32; 0x12];
        let fmp = GPtr::<u8>::from_raw(fm.as_mut_ptr().cast());
        matrix_ctor_rw(fmp, frame.byte_add(0x10), 0);
        matrix_update_rw(fmp);
        let mut am = [0u32; 0x12];
        let ap = GPtr::<u8>::from_raw(am.as_mut_ptr().cast());
        matrix_attach(ap, rd::<GPtr<u8>>(fmp, 0x40), 0);
        let mut bm = [0u32; 0x12];
        let bp = GPtr::<u8>::from_raw(bm.as_mut_ptr().cast());
        matrix_set_unity(bp);
        // `test ah, 0x44` + `jp` (x, y vs 0.0) / `jnp` (z vs 1.0): the rotation is skipped only when equal
        let eq = |o: isize, kv: u32| !jp(x(rd::<f32>(dp, o)), x(k(kv)), 0x44);
        if !(eq(0, 0) && eq(4, 0) && eq(8, K_ONE)) {
            let mut up = [0u32, 0, K_ONE];
            let mut tmp = [0u32; 3];
            let tp = GPtr::<u8>::from_raw(tmp.as_mut_ptr().cast());
            let p = cross_product(tp, GPtr::from_raw(up.as_mut_ptr().cast()), dp);
            let mut v1 = [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)];
            let v1p = GPtr::<u8>::from_raw(v1.as_mut_ptr().cast());
            vector_normalise(v1p);
            let p = cross_product(tp, dp, v1p);
            let mut v2 = [rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<u32>(p, 8)];
            let v2p = GPtr::<u8>::from_raw(v2.as_mut_ptr().cast());
            vector_normalise(v2p);
            for j in 0..3 {
                wr::<u32>(bp, j * 4, rd::<u32>(v1p, j * 4));
                wr::<u32>(bp, 0x10 + j * 4, rd::<u32>(v2p, j * 4));
                wr::<u32>(bp, 0x20 + j * 4, rd::<u32>(dp, j * 4));
            }
        }
        let mut cm = [0u32; 0x12];
        let cp = GPtr::<u8>::from_raw(cm.as_mut_ptr().cast());
        let r = matrix_mul(cp, ap, bp);
        matrix_assign(ap, r);
        matrix_dtor(cp);
        let size = rd::<f32>(e, 0x10);
        for row in 0..3isize {
            for c in 0..3isize {
                let o = row * 0x10 + c * 4;
                wr::<f32>(ap, o, f32_of(x(size).fmul_m(rd::<f32>(ap, o))));
            }
        }
        matrix_update_rw(ap);
        rw_frame_update_objects(frame);
        USER_MARKER_COLOUR.set(0xFF00_0000);
        rp_clump_for_all_atomics(clump, USER_MARKER_ATOMIC_CB, GPtr::NULL);
        rp_clump_render(clump);
        for row in 0..3isize {
            for c in 0..3isize {
                let o = row * 0x10 + c * 4;
                wr::<f32>(ap, o, f32_of(x(rd::<f32>(ap, o)).fmul_m(k(K_0_8))));
            }
        }
        matrix_update_rw(ap);
        rw_frame_update_objects(frame);
        let c = USER_MARKER_COLOUR.ptr().cast::<u8>();
        c.write(rd::<u32>(e, 0x20) as u8);
        c.wrapping_index(1).write(rd::<u32>(e, 0x24) as u8);
        c.wrapping_index(2).write(rd::<u32>(e, 0x28) as u8);
        c.wrapping_index(3).write(rd::<u32>(e, 0x2C) as u8);
        rp_clump_for_all_atomics(clump, USER_MARKER_ATOMIC_CB, GPtr::NULL);
        rp_clump_render(clump);
        wr::<u8>(e, 0, 0);
        matrix_dtor(bp);
        matrix_dtor(ap);
        matrix_dtor(fmp);
    }
    if saved {
        rw_render_state_set(0x14, g::<u32>(0xC4028C).read());
        rw_render_state_set(6, g::<u32>(0xC40290).read());
        rw_render_state_set(8, g::<u32>(0xC40294).read());
        rw_render_state_set(0xC, g::<u32>(0xC4029C).read());
    }
}

/// `C3dMarkers::User3dMarkersDraw()` (0x723240): for each used user marker: the first one saves the cull
/// / z-test / z-write render states (into 0xC4028C / 0xC40290 / 0xC40294) and sets cull none, z-test on,
/// z-write off; diamond clump 5's frame is reset and translated (replace) to the marker's position raised
/// by `sin(m_angleDiamond * pi/180) * 0.25` (FSIN, sa-math's f64 model; the sum stored as f32), the user
/// colour set to (r, g, b, 255), every atomic's material coloured (`User3dMarkerAtomicCB`) and the clump
/// rendered. The saved states are restored when any marker was drawn.
#[hook(addr = 0x723240, conv = cdecl, name = "C3dMarkers::User3dMarkersDraw", expect = "83 EC 0C 53 55")]
pub fn user_3d_markers_draw() {
    let mut saved = false;
    for i in 0..5 {
        let u = user(i);
        if rd::<u8>(u, 0) == 0 {
            continue;
        }
        if !saved {
            rw_render_state_get(0x14, GPtr::from_addr(0xC4028C));
            rw_render_state_get(6, GPtr::from_addr(0xC40290));
            rw_render_state_get(8, GPtr::from_addr(0xC40294));
            rw_render_state_set(0x14, 1);
            rw_render_state_set(6, 1);
            rw_render_state_set(8, 0);
            saved = true;
        }
        let s = x(ANGLE_DIAMOND.get()).fmul_m(k(K_DEG2RAD)).fsin();
        let clump = GPtr::<u8>::from_addr(g::<u32>(0xC7C6F0).read() as usize);
        let mut pos = [rd::<u32>(u, 4), rd::<u32>(u, 8), rd::<u32>(u, 0xC)];
        let frame = rd::<GPtr<u8>>(clump, 4);
        pos[2] = f32_of(s.fmul_m(k(K_0_25)).fadd_m(f32::from_bits(pos[2]))).to_bits();
        rw_frame_set_identity(frame);
        rw_frame_translate(frame, GPtr::from_raw(pos.as_mut_ptr().cast()), 0);
        let c = USER_MARKER_COLOUR.ptr().cast::<u8>();
        c.write(rd::<u32>(u, 0x10) as u8);
        c.wrapping_index(1).write(rd::<u32>(u, 0x14) as u8);
        c.wrapping_index(2).write(rd::<u32>(u, 0x18) as u8);
        c.wrapping_index(3).write(0xFF);
        rp_clump_for_all_atomics(clump, USER_MARKER_ATOMIC_CB, GPtr::NULL);
        rp_clump_render(clump);
    }
    if saved {
        rw_render_state_set(0x14, g::<u32>(0xC4028C).read());
        rw_render_state_set(6, g::<u32>(0xC40290).read());
        rw_render_state_set(8, g::<u32>(0xC40294).read());
    }
}

/// `C3dMarkers::User3dMarkerFindFirstFreeSlot()` (0x720FB0): the first unused user marker, or -1.
#[hook(addr = 0x720FB0, conv = cdecl, name = "C3dMarkers::User3dMarkerFindFirstFreeSlot", expect = "33 C0 B9 58 02 C8 00")]
pub fn user_3d_marker_find_first_free_slot() -> i32 {
    (0..5).find(|&i| rd::<u8>(user(i), 0) == 0).unwrap_or(-1)
}

/// `C3dMarkers::User3dMarkerSet(float x, float y, float z, eHudColours colour)` (0x720FD0): the first
/// free user marker (none: -1) gets the position and the HUD colour's r, g, b (one `GetRGB` call each,
/// into a local), marked used; returns its index.
#[hook(addr = 0x720FD0, conv = cdecl, name = "C3dMarkers::User3dMarkerSet", expect = "51 57 33 FF B8 58 02 C8 00")]
pub fn user_3d_marker_set(px: u32, py: u32, pz: u32, colour: u32) -> i32 {
    let i = user_3d_marker_find_first_free_slot();
    if i == -1 {
        return -1;
    }
    let u = user(i);
    wr::<u32>(u, 4, px);
    wr::<u32>(u, 8, py);
    wr::<u32>(u, 0xC, pz);
    let mut tmp = 0u32;
    for c in 0..3 {
        let p = hud_get_rgb(GPtr::from_addr(HUD_COLOUR), GPtr::from_raw(&mut tmp).cast(), colour);
        wr::<u32>(u, 0x10 + c * 4, u32::from(rd::<u8>(p, c)));
    }
    wr::<u8>(u, 0, 1);
    i
}

/// `C3dMarkers::User3dMarkerDelete(int32 i)` (0x721090): unused when `0 <= i <= 4`.
#[hook(addr = 0x721090, conv = cdecl, name = "C3dMarkers::User3dMarkerDelete", expect = "8B 44 24 04 85 C0")]
pub fn user_3d_marker_delete(i: i32) {
    if (0..=4).contains(&i) {
        wr::<u8>(user(i), 0, 0);
    }
}

/// `C3dMarkers::User3dMarkerDeleteAll()` (0x7210B0).
#[hook(addr = 0x7210B0, conv = cdecl, name = "C3dMarkers::User3dMarkerDeleteAll", expect = "B8 58 02 C8 00")]
pub fn user_3d_marker_delete_all() {
    for i in 0..5 {
        wr::<u8>(user(i), 0, 0);
    }
}

/// `C3dMarkers::User3dMarkerAtomicCB(RpAtomic*, void*)` (0x7210D0): material 0's colour =
/// `m_user3dMarkerColor`; returns null.
#[hook(addr = 0x7210D0, conv = cdecl, name = "C3dMarkers::User3dMarkerAtomicCB", expect = "8B 44 24 04 8B 48 18")]
pub fn user_3d_marker_atomic_cb(atomic: GPtr<u8>, _data: u32) -> GPtr<u8> {
    let mat = rd::<GPtr<u8>>(rd::<GPtr<u8>>(rd::<GPtr<u8>>(atomic, 0x18), 0x20), 0);
    wr::<u32>(mat, 4, USER_MARKER_COLOUR.get());
    GPtr::NULL
}

/// `C3dMarkers::LoadUser3dMarkers()` (0x5D42E0, AL = 1): the five user markers from the work buffer.
#[hook(addr = 0x5D42E0, conv = cdecl, name = "C3dMarkers::LoadUser3dMarkers", expect = "56 BE 58 02 C8 00")]
pub fn load_user_3d_markers() -> u8 {
    for i in 0..5 {
        load_data_from_work_buffer(user(i), 0x1C);
    }
    1
}

/// `C3dMarkers::SaveUser3dMarkers()` (0x5D4300, AL = 1).
#[hook(addr = 0x5D4300, conv = cdecl, name = "C3dMarkers::SaveUser3dMarkers", expect = "56 BE 58 02 C8 00")]
pub fn save_user_3d_markers() -> u8 {
    for i in 0..5 {
        save_data_to_work_buffer(user(i), 0x1C);
    }
    1
}

/// `C3dMarkers::DirectionArrowsInit()` (0x721100): all arrows unused.
#[hook(addr = 0x721100, conv = cdecl, name = "C3dMarkers::DirectionArrowsInit", expect = "B8 E8 02 C8 00")]
pub fn direction_arrows_init() {
    for i in 0..5 {
        wr::<u8>(arrow(i), 0, 0);
    }
}

/// `C3dMarkers::DirectionArrowFindFirstFreeSlot()` (0x721120): first unused arrow, or -1.
#[hook(addr = 0x721120, conv = cdecl, name = "C3dMarkers::DirectionArrowFindFirstFreeSlot", expect = "33 C0 B9 E8 02 C8 00")]
pub fn direction_arrow_find_first_free_slot() -> i32 {
    (0..5).find(|&i| rd::<u8>(arrow(i), 0) == 0).unwrap_or(-1)
}

/// `C3dMarkers::DirectionArrowSet(CVector pos, float size, int32 r, g, b, a, float dirX, dirY, dirZ)`
/// (0x721140): the first free arrow (none: nothing) gets x, y, `z + 3` (f32), size, direction, colour;
/// marked used.
#[hook(addr = 0x721140, conv = cdecl, name = "C3dMarkers::DirectionArrowSet", expect = "33 C9 B8 E8 02 C8 00")]
#[allow(clippy::too_many_arguments)]
pub fn direction_arrow_set(px: u32, py: u32, pz: u32, size: u32, r: u32, gg: u32, b: u32, a: u32, dx: u32, dy: u32, dz: u32) {
    let Some(i) = (0..5).find(|&i| rd::<u8>(arrow(i), 0) == 0) else {
        return;
    };
    let e = arrow(i);
    wr::<u32>(e, 4, px);
    wr::<f32>(e, 0xC, f32_of(x(f32::from_bits(pz)).fadd_m(k(K_THREE))));
    wr::<u32>(e, 8, py);
    for (o, v) in [(0x10, size), (0x14, dx), (0x18, dy), (0x1C, dz), (0x20, r), (0x24, gg), (0x28, b), (0x2C, a)] {
        wr::<u32>(e, o, v);
    }
    wr::<u8>(e, 0, 1);
}

/// `__init_C3dMarkers__m_aMarkerArray` (0x855310, CRT initialiser): the 32 markers constructed, the
/// array destructor registered.
#[hook(addr = 0x855310, conv = cdecl, name = "__init_C3dMarkers__m_aMarkerArray", expect = "68 70 0F 72 00")]
pub fn init_marker_array() {
    eh_vector_ctor(GPtr::from_addr(MARKERS_BASE), 0xA0, 0x20, MARKER_CTOR, MARKER_DTOR);
    atexit(DTOR_MARKER_ARRAY);
}

/// `__dtor_C3dMarkers__m_aMarkerArray` (0x856BD0): the 32 markers destroyed.
#[hook(addr = 0x856BD0, conv = cdecl, name = "__dtor_C3dMarkers__m_aMarkerArray", expect = "68 70 0F 72 00")]
pub fn dtor_marker_array() {
    eh_vector_dtor(GPtr::from_addr(MARKERS_BASE), 0xA0, 0x20, MARKER_DTOR);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_MARKER_CTOR,
    HOOK_MARKER_DTOR,
    HOOK_DELETE_MARKER_OBJECT,
    HOOK_ADD_MARKER,
    HOOK_IS_Z_COORDINATE_UP_TO_DATE,
    HOOK_SET_Z_COORDINATE_IF_NOT_UP_TO_DATE,
    HOOK_UPDATE_Z_COORDINATE,
    HOOK_INIT,
    HOOK_SHUTDOWN,
    HOOK_UPDATE,
    HOOK_FORCE_RENDER,
    HOOK_RENDER,
    HOOK_PLACE_MARKER,
    HOOK_PLACE_MARKER_CONE,
    HOOK_PLACE_MARKER_SET,
    HOOK_USER_3D_MARKERS_DRAW,
    HOOK_DIRECTION_ARROWS_DRAW,
    HOOK_MARKER_RENDER,
    HOOK_USER_3D_MARKER_FIND_FIRST_FREE_SLOT,
    HOOK_USER_3D_MARKER_SET,
    HOOK_USER_3D_MARKER_DELETE,
    HOOK_USER_3D_MARKER_DELETE_ALL,
    HOOK_USER_3D_MARKER_ATOMIC_CB,
    HOOK_LOAD_USER_3D_MARKERS,
    HOOK_SAVE_USER_3D_MARKERS,
    HOOK_DIRECTION_ARROWS_INIT,
    HOOK_DIRECTION_ARROW_FIND_FIRST_FREE_SLOT,
    HOOK_DIRECTION_ARROW_SET,
    HOOK_INIT_MARKER_ARRAY,
    HOOK_DTOR_MARKER_ARRAY,
];
