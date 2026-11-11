//! PORT-CAM: the CAM static initialisers (`__init_*`). Ported from the 1.0 US bytes. The globals they
//! construct are declared by their classes' modules (`gHandShaker` handshaker.rs, `gIdleCam` idlecam.rs,
//! 0xB6FEC0 cam.rs); `TheCamera` and `gpCamColVars` (Camera.cpp) are addressed directly.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{f32_of, g, k, the_camera, x};
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

/// The header-constant initialisers (`__init_g_*`): each TU that includes the timing header computes
/// the same derived constants into its own copy. `fld a; fmul/fdiv b; fstp dword dst` (or
/// `fld a; fmul a; fdivr b`, i.e. `b / (a * a)`), every operand an `.rdata` f32 except the `* 10`
/// variants, which read the copy their TU computed just before (a variable, as the bytes do).
fn hc_mul(a: u32, b: u32, dst: usize) {
    g::<f32>(dst).write(f32_of(x(k(a)).fmul_m(k(b))));
}
fn hc_div(a: u32, b: u32, dst: usize) {
    g::<f32>(dst).write(f32_of(x(k(a)).fdiv_m(k(b))));
}
fn hc_inv_sq(a: u32, b: u32, dst: usize) {
    g::<f32>(dst).write(f32_of(x(k(a)).fmul_m(k(a)).fdivr_m(k(b))));
}
fn hc_mul_var(src: usize, b: u32, dst: usize) {
    g::<f32>(dst).write(f32_of(x(g::<f32>(src).read()).fmul_m(k(b))));
}

/// `.rdata` constants of the header initialisers: 0.005 (0x862E6C / 0x862F04 / 0x868A50 / 0x872970),
/// 0.0625 (0x858620), 1.0 (0x858624), 100.0 (0x858628), 10.0 (0x85862C), 0.27778 (0x858630),
/// 50.0 (0x862F0C / 0x868A58), 3.6 (0x862F08 / 0x868A54).
const C_0_005: u32 = 0x3BA3_D70A;
const C_0_0625: u32 = 0x3D80_0000;
const C_1: u32 = 0x3F80_0000;
const C_100: u32 = 0x42C8_0000;
const C_10: u32 = 0x4120_0000;
const C_0_2778: u32 = 0x3E8E_38EB;
const C_50: u32 = 0x4248_0000;
const C_3_6: u32 = 0x4066_6666;

/// `__init_g_b6eba0` (0x84D5F0): `[0xB6EBA0] = 0.005 * 0.0625`.
#[hook(addr = 0x84D5F0, conv = cdecl, name = "__init_g_b6eba0", expect = "D9 05 6C 2E 86 00")]
pub fn init_g_b6eba0() {
    hc_mul(C_0_005, C_0_0625, 0xB6EBA0);
}
/// `__init_g_b6ec88` (0x84D610): `[0xB6EC88] = 0.005 * 0.0625`.
#[hook(addr = 0x84D610, conv = cdecl, name = "__init_g_b6ec88", expect = "D9 05 04 2F 86 00")]
pub fn init_g_b6ec88() {
    hc_mul(C_0_005, C_0_0625, 0xB6EC88);
}
/// `__init_g_b6ec8c` (0x84D630): `[0xB6EC8C] = 1.0 / 50.0`.
#[hook(addr = 0x84D630, conv = cdecl, name = "__init_g_b6ec8c", expect = "D9 05 24 86 85 00")]
pub fn init_g_b6ec8c() {
    hc_div(C_1, C_50, 0xB6EC8C);
}
/// `__init_g_b6ec90` (0x84D650): `[0xB6EC90] = 100.0 / 3.6`.
#[hook(addr = 0x84D650, conv = cdecl, name = "__init_g_b6ec90", expect = "D9 05 28 86 85 00")]
pub fn init_g_b6ec90() {
    hc_div(C_100, C_3_6, 0xB6EC90);
}
/// `__init_g_b6ec94` (0x84D670): `[0xB6EC94] = 1.0 / (50.0 * 50.0)`.
#[hook(addr = 0x84D670, conv = cdecl, name = "__init_g_b6ec94", expect = "D9 05 0C 2F 86 00")]
pub fn init_g_b6ec94() {
    hc_inv_sq(C_50, C_1, 0xB6EC94);
}
/// `__init_g_b6ec98` (0x84D690): `[0xB6EC98] = [0xB6EC94] * 10.0`.
#[hook(addr = 0x84D690, conv = cdecl, name = "__init_g_b6ec98", expect = "D9 05 94 EC B6 00")]
pub fn init_g_b6ec98() {
    hc_mul_var(0xB6EC94, C_10, 0xB6EC98);
}
/// `__init_g_b6ec9c` (0x84D6B0): `[0xB6EC9C] = 0.27778 / 50.0`.
#[hook(addr = 0x84D6B0, conv = cdecl, name = "__init_g_b6ec9c", expect = "D9 05 30 86 85 00")]
pub fn init_g_b6ec9c() {
    hc_div(C_0_2778, C_50, 0xB6EC9C);
}
/// `__init_g_bc405c` (0x850060): `[0xBC405C] = 0.005 * 0.0625`.
#[hook(addr = 0x850060, conv = cdecl, name = "__init_g_bc405c", expect = "D9 05 50 8A 86 00")]
pub fn init_g_bc405c() {
    hc_mul(C_0_005, C_0_0625, 0xBC405C);
}
/// `__init_g_bc4060` (0x850080): `[0xBC4060] = 1.0 / 50.0`.
#[hook(addr = 0x850080, conv = cdecl, name = "__init_g_bc4060", expect = "D9 05 24 86 85 00")]
pub fn init_g_bc4060() {
    hc_div(C_1, C_50, 0xBC4060);
}
/// `__init_g_bc4064` (0x8500A0): `[0xBC4064] = 100.0 / 3.6`.
#[hook(addr = 0x8500A0, conv = cdecl, name = "__init_g_bc4064", expect = "D9 05 28 86 85 00")]
pub fn init_g_bc4064() {
    hc_div(C_100, C_3_6, 0xBC4064);
}
/// `__init_g_bc4068` (0x8500C0): `[0xBC4068] = 1.0 / (50.0 * 50.0)`.
#[hook(addr = 0x8500C0, conv = cdecl, name = "__init_g_bc4068", expect = "D9 05 58 8A 86 00")]
pub fn init_g_bc4068() {
    hc_inv_sq(C_50, C_1, 0xBC4068);
}
/// `__init_g_bc406c` (0x8500E0): `[0xBC406C] = [0xBC4068] * 10.0`.
#[hook(addr = 0x8500E0, conv = cdecl, name = "__init_g_bc406c", expect = "D9 05 68 40 BC 00")]
pub fn init_g_bc406c() {
    hc_mul_var(0xBC4068, C_10, 0xBC406C);
}
/// `__init_g_bc4070` (0x850100): `[0xBC4070] = 0.27778 / 50.0`.
#[hook(addr = 0x850100, conv = cdecl, name = "__init_g_bc4070", expect = "D9 05 30 86 85 00")]
pub fn init_g_bc4070() {
    hc_div(C_0_2778, C_50, 0xBC4070);
}
/// `__init_g_c73c4c` (0x854FD0): `[0xC73C4C] = 0.005 * 0.0625`.
#[hook(addr = 0x854FD0, conv = cdecl, name = "__init_g_c73c4c", expect = "D9 05 70 29 87 00")]
pub fn init_g_c73c4c() {
    hc_mul(C_0_005, C_0_0625, 0xC73C4C);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_INIT_G_HAND_SHAKER,
    HOOK_INIT_THE_CAMERA,
    HOOK_INIT_G_IDLE_CAM,
    HOOK_INIT_PRE_MIRROR_MAT,
    HOOK_INIT_GP_CAM_COL_VARS,
    HOOK_INIT_G_B6FEC0,
    HOOK_INIT_G_B6EBA0,
    HOOK_INIT_G_B6EC88,
    HOOK_INIT_G_B6EC8C,
    HOOK_INIT_G_B6EC90,
    HOOK_INIT_G_B6EC94,
    HOOK_INIT_G_B6EC98,
    HOOK_INIT_G_B6EC9C,
    HOOK_INIT_G_BC405C,
    HOOK_INIT_G_BC4060,
    HOOK_INIT_G_BC4064,
    HOOK_INIT_G_BC4068,
    HOOK_INIT_G_BC406C,
    HOOK_INIT_G_BC4070,
    HOOK_INIT_G_C73C4C,
];

/// This module's globals (none: see the module docs).
pub const GLOBALS: &[GlobalDesc] = &[];
