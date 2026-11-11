//! PORT-CAM: the CAM static initialisers (`__init_*`). Ported from the 1.0 US bytes. The globals they
//! construct are declared by their classes' modules (`gHandShaker` handshaker.rs, `gIdleCam` idlecam.rs,
//! 0xB6FEC0 cam.rs); `TheCamera` and `gpCamColVars` (Camera.cpp) are addressed directly.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, the_camera};
use crate::handshaker::{G_HAND_SHAKER, HAND_SHAKER_CTOR, HAND_SHAKER_DTOR};

/// `gpCamColVars` (Camera.cpp, float*)
const GP_CAM_COL_VARS: usize = 0xB6FE88;
/// `gCamColVars` (Camera.cpp, 0x18-byte rows) and `gCurCamColVars` (u8)
const G_CAM_COL_VARS: usize = 0x8CC8E0;
const G_CUR_CAM_COL_VARS: usize = 0x8CCB80;

/// The `_atexit` destructors (`__dtor_*`).
const DTOR_G_HAND_SHAKER: u32 = 0x8563B0;
const DTOR_THE_CAMERA: u32 = 0x856410;
const DTOR_G_IDLE_CAM: u32 = 0x8563D0;
const DTOR_PRE_MIRROR_MAT: u32 = 0x8563E0;
const DTOR_G_B6FEC0: u32 = 0x8563F0;
/// The object at 0xB6FEC0: constructor 0x5173F0, destructor 0x50E080 (a bare `ret`).
const B6FEC0_CTOR: u32 = 0x5173F0;
const B6FEC0_DTOR: u32 = 0x50E080;

orig! {
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall)
    pub fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// CRT `_atexit` (0x821D1E)
    pub fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
    /// `CCamera::Constructor()` (0x51A450)
    pub fn camera_constructor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x51A450 => unported;
    /// `CIdleCam::Init()` (0x50E6D0)
    pub fn idle_cam_init_call(this: GPtr<u8>) = thiscall 0x50E6D0 => crate::idlecam::idle_cam_init;
}

/// `__init_gHandShaker` (0x84D6D0): `eh vector constructor iterator(gHandShaker, 0x94, 6,
/// CHandShaker::CHandShaker, CHandShaker::~CHandShaker)`, `_atexit(__dtor_gHandShaker)`.
#[hook(addr = 0x84D6D0, conv = cdecl, name = "__init_gHandShaker", expect = "68 10 98 50 00")]
pub fn init_g_hand_shaker() {
    eh_vector_ctor(
        G_HAND_SHAKER.ptr().cast(),
        0x94,
        6,
        HAND_SHAKER_CTOR,
        HAND_SHAKER_DTOR,
    );
    atexit(DTOR_G_HAND_SHAKER);
}

/// `__init_TheCamera` (0x84D700): `TheCamera.CCamera()`, `_atexit(__dtor_TheCamera)`.
#[hook(addr = 0x84D700, conv = cdecl, name = "__init_TheCamera", expect = "B9 28 F0 B6 00")]
pub fn init_the_camera() {
    camera_constructor(the_camera());
    atexit(DTOR_THE_CAMERA);
}

/// `__init_gIdleCam` (0x84D720): `gIdleCam.Init()` (the constructor inlined), `_atexit(__dtor_gIdleCam)`.
#[hook(addr = 0x84D720, conv = cdecl, name = "__init_gIdleCam", expect = "B9 A0 FD B6 00")]
pub fn init_g_idle_cam() {
    idle_cam_init_call(crate::idlecam::G_IDLE_CAM.ptr().cast());
    atexit(DTOR_G_IDLE_CAM);
}

/// `__init_preMirrorMat` (0x84D740): only `_atexit(__dtor_preMirrorMat)` (trivial constructor).
#[hook(addr = 0x84D740, conv = cdecl, name = "__init_preMirrorMat", expect = "68 E0 63 85 00")]
pub fn init_pre_mirror_mat() {
    atexit(DTOR_PRE_MIRROR_MAT);
}

/// `__init_gpCamColVars` (0x84D750): `gpCamColVars = &gCamColVars[gCurCamColVars]` (0x18-byte rows).
#[hook(addr = 0x84D750, conv = cdecl, name = "__init_gpCamColVars", expect = "0F B6 05 80 CB 8C 00")]
pub fn init_gp_cam_col_vars() {
    let i = g::<u8>(G_CUR_CAM_COL_VARS).read();
    let p = g::<u8>(G_CAM_COL_VARS).byte_add(isize::from(i) * 0x18);
    g::<GPtr<u8>>(GP_CAM_COL_VARS).write(p);
}

/// `__init_g_b6fec0` (0x84D770): `eh vector constructor iterator(0xB6FEC0, 0x9C, 1, 0x5173F0,
/// 0x50E080)`, `_atexit(__dtor_g_b6fec0)`.
#[hook(addr = 0x84D770, conv = cdecl, name = "__init_g_b6fec0", expect = "68 80 E0 50 00")]
pub fn init_g_b6fec0() {
    eh_vector_ctor(
        crate::cam::G_B6FEC0.ptr().cast(),
        0x9C,
        1,
        B6FEC0_CTOR,
        B6FEC0_DTOR,
    );
    atexit(DTOR_G_B6FEC0);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_INIT_G_HAND_SHAKER,
    HOOK_INIT_THE_CAMERA,
    HOOK_INIT_G_IDLE_CAM,
    HOOK_INIT_PRE_MIRROR_MAT,
    HOOK_INIT_GP_CAM_COL_VARS,
    HOOK_INIT_G_B6FEC0,
];

/// This module's globals (none: see the module docs).
pub const GLOBALS: &[GlobalDesc] = &[];
