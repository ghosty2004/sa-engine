//! PORT-RENDER: the FX system (`Fx_c` g_fx 0xA9AE00, `FxManager_c` g_fxMan 0xA9AE80, `FxSystem_c`,
//! `FxSystemBP_c`, `FxEmitter_c`, `FxEmitterBP_c`, `FxPrimBP_c`, `FxMemoryPool_c`, `FxInterpInfo*_c`,
//! `FxInfo*_c`). Ported from the 1.0 US bytes; gta-reversed `Fx/*.cpp` and
//! `docs/re/specs/RENDER/Fx*.md` as reading aids.
//!
//! Not hookable (the entry is shorter than the 5-byte patch: control leaves with `ret` first), ported
//! for phases B/C and reached through `orig!`: `Fx_c::GetFxQuality` (4 bytes), `FxManager_c::
//! GetFrustumInfo` (4), `FxMemoryPool_c::FxMemoryPool_c` (3), `FxSystem_c::GetPlayStatus` (5 incl. `ret`).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{ftol32, g, jne, k, rd, wr, x, xi};

pub mod info;

/// `g_fxMan` (FxManager_c, owned by the FX system; its storage stays original until the FX init ports).
pub const FX_MAN: usize = 0xA9AE80;
/// `g_fxMan` +0xAC: the FX memory pool (FxMemoryPool_c).
pub const FX_POOL: usize = 0xA9AF2C;
/// `g_fx` (Fx_c, 0x70 bytes; its storage stays original until the FX init ports).
pub const FX: usize = 0xA9AE00;
/// `g_fx` +0x58 .. +0x6C: the immediate-mode render state `RenderBegin` / `RenderEnd` share.
const FX_TXD_VERTS: isize = 0x58;
const FX_TXD_INDICES: isize = 0x5C;
const FX_RENDER_FLAGS: isize = 0x60;
const FX_RENDER_RASTER: isize = 0x64;
const FX_RENDER_LTM: isize = 0x68;
const FX_RENDER_VERTS_PTR: isize = 0x6C;
/// The FX immediate-mode vertex buffer (0xC4D958).
const FX_VERTEX_BUFFER: u32 = 0xC4D958;
/// `"models\\effects.fxp"` (0x85A6D4)
const FX_PROJECT_PATH: usize = 0x85A6D4;
/// `"water_splash_big"` (0x85A740, the system `TriggerWaterSplash` creates)
const WATER_SPLASH_NAME: usize = 0x85A740;
/// The wind vector / strength `Fx_c::Init` hands to `SetWindData` (`CWeather::WindDir` 0xC813E0,
/// `CWeather::Wind` 0xC812F0).
const WEATHER_WIND_DIR: usize = 0xC813E0;
const WEATHER_WIND: usize = 0xC812F0;
/// The debug format of `FindFxSystemBP`'s failure `sprintf` (0x85A99C).
const FIND_BP_FMT: usize = 0x85A99C;
/// `FxEmitter_c` vtable (0x85A7A4).
const VT_EMITTER: u32 = 0x85A7A4;
/// `CAEAudioEntity`'s base vtable restored by `FxSystem_c`'s destructor (0x859988).
const VT_AUDIO_ENTITY: u32 = 0x859988;
/// 625.0 (0x85A6E8): `TriggerWaterSplash`'s squared distance limit.
const K_625: u32 = 0x441C_4000;
/// 0.0001 (0x858FC4): `Play`'s `rand() % 10000` scale.
const K_RAND_SCALE: u32 = 0x38D1_B717;

/// `FxInterpInfoFloat_c` vtable.
const VT_INTERP_FLOAT: u32 = 0x85A970;
/// `FxInterpInfoU255_c` vtable.
const VT_INTERP_U255: u32 = 0x85A978;
/// `FxInterpInfo32_c` vtable.
const VT_INTERP_32: u32 = 0x85A980;
/// `FxInterpInfo255_c` vtable.
const VT_INTERP_255: u32 = 0x85A988;
/// `FxEmitterBP_c` vtable.
const VT_EMITTER_BP: u32 = 0x85A788;
/// `FxPrimBP_c` vtable.
const VT_PRIM_BP: u32 = 0x85A9C4;
/// `FxSystem_c` member +0x7C vtable (its `FxSphere_c` / audio helper object).
const VT_SYSTEM_7C: u32 = 0x85AA94;
/// 1000.0 (0x858C4C)
const K_1000: u32 = 0x447A_0000;
/// 256.0 (0x858FB4)
const K_256: u32 = 0x4380_0000;

orig! {
    /// `List_c::List_c()` (0x4A8DD0): head, tail, count = 0
    pub fn list_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A8DD0 => list_ctor_port;
    /// `ListItem_c::ListItem_c()` (0x4A8DB0): prev, next = 0
    pub fn list_item_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A8DB0 => list_item_ctor_port;
    /// `List_c::AddItem(ListItem_c*)` (0x4A8DF0)
    pub fn list_add_item(this: GPtr<u8>, item: GPtr<u8>) = thiscall 0x4A8DF0 => unported;
    /// `List_c::RemoveHead()` (0x4A8E70)
    pub fn list_remove_head(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A8E70 => unported;
    /// `FxManager_c::Update(RwCamera*, float)` (0x4A9A80)
    pub fn fx_manager_update(this: GPtr<u8>, cam: GPtr<u8>, dt: u32) = thiscall 0x4A9A80 => fx_manager_update_port;
    /// `FxManager_c::FindFxSystemBP(const char*)` (0x4A9360)
    pub fn find_fx_system_bp(this: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A9360 => fx_manager_find_fx_system_bp;
    /// `FxManager_c::CreateFxSystem(FxSystemBP_c*, RwMatrix*, RwMatrix*, bool)` (0x4A96B0)
    pub fn create_fx_system_bp(this: GPtr<u8>, bp: GPtr<u8>, m: GPtr<u8>, parent: GPtr<u8>, ignore_bounds: u32) -> GPtr<u8> = thiscall 0x4A96B0 => unported;
    /// `FxManager_c::GetFrustumInfo()` (0x4A9130)
    pub fn get_frustum_info(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A9130 => fx_manager_get_frustum_info;
    /// `FxMemoryPool_c::GetMem(int32 size, int32 align)` (0x4A9CA0)
    pub fn pool_get_mem(this: GPtr<u8>, size: u32, align: u32) -> GPtr<u8> = thiscall 0x4A9CA0 => fx_memory_pool_get_mem;
    /// `FxPrim_c::Enable(bool)` (0x4A9F50)
    pub fn prim_enable(this: GPtr<u8>, on: u32) = thiscall 0x4A9F50 => fx_prim_enable;
    /// `FxPrimBP_c::FxPrimBP_c()` (0x4A9CF0)
    pub fn prim_bp_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A9CF0 => fx_prim_bp_constructor;
    /// `FxFrustumInfo_c::IsCollision(FxSphere_c*)` (0x4AA030)
    pub fn frustum_is_collision(this: GPtr<u8>, s: GPtr<u8>) -> u32 = thiscall 0x4AA030 => unported;
    /// `FxSystem_c::Play()` (0x4AA2F0)
    pub fn system_play(this: GPtr<u8>) = thiscall 0x4AA2F0 => fx_system_play;
    /// `FxSystem_c::Stop()` (0x4AA390)
    pub fn system_stop(this: GPtr<u8>) = thiscall 0x4AA390 => fx_system_stop;
    /// `FxSystem_c::GetBoundingSphereWld(FxSphere_c*)` (0x4AAAD0)
    pub fn system_get_bounding_sphere_wld(this: GPtr<u8>, out: GPtr<u8>) -> u8 = thiscall 0x4AAAD0 => fx_system_get_bounding_sphere_wld;
    /// `FxPrim_c` virtual slot 0x10: `Update(float)` (as `FxSystemBP_c::Update` calls it)
    pub fn prim_bp_vupdate(this: GPtr<u8>, dt: u32) = thiscall vslot 0x10 => unported;
    /// `FxPrim_c` virtual slot 0xC (as `FxSystem_c::Stop` and `Play` call it)
    pub fn prim_vstop(this: GPtr<u8>) = thiscall vslot 0xC => unported;
    /// `CMemoryMgr::Realloc(void*, uint32, const char*)` (0x72F440)
    pub fn memory_realloc(p: GPtr<u8>, size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F440 => unported;
    /// `RwMatrixUpdate(RwMatrix*)` (0x7F18A0)
    pub fn rw_matrix_update(m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18A0 => unported;
    /// `RwMatrixMultiply(RwMatrix* out, const RwMatrix* a, const RwMatrix* b)` (0x7F18B0)
    pub fn rw_matrix_multiply(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18B0 => unported;
    /// `RwMatrixCreate()` (0x7F2A50)
    pub fn rw_matrix_create() -> GPtr<u8> = cdecl 0x7F2A50 => unported;
    /// `RwMatrixDestroy(RwMatrix*)` (0x7F2A20)
    pub fn rw_matrix_destroy(m: GPtr<u8>) -> i32 = cdecl 0x7F2A20 => unported;
    /// `RwV3dTransformPoints(RwV3d* out, const RwV3d* in, int32 n, const RwMatrix* m)` (0x7EDD90)
    pub fn rw_v3d_transform_points(out: GPtr<u8>, inp: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDD90 => unported;
    /// `RwRenderStateSet` (`RwEngineInstance` slot 0x20)
    pub fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwRenderStateGet` (`RwEngineInstance` slot 0x24)
    pub fn rw_render_state_get(state: u32, out: GPtr<u8>) -> i32 = cdecl table 0xC97B24 0x24 => unported;
    /// `List_c::GetNumItems()` (0x4A8EC0)
    pub fn list_get_num_items(this: GPtr<u8>) -> i32 = thiscall 0x4A8EC0 => unported;
    /// `List_c::RemoveItem(ListItem_c*)` (0x4A8E30)
    pub fn list_remove_item(this: GPtr<u8>, item: GPtr<u8>) = thiscall 0x4A8E30 => unported;
    /// `ListItem_c::~ListItem_c()` (0x49EA70)
    pub fn list_item_dtor(this: GPtr<u8>) = thiscall 0x49EA70 => unported;
    /// `ListItem_c` base destructor as `FxSystem_c::~FxSystem_c` calls it (0x4A8DC0, a lone `ret`)
    pub fn list_item_dtor_b(this: GPtr<u8>) = thiscall 0x4A8DC0 => unported;
    /// CRT `operator new` (0x82119A)
    pub fn operator_new(n: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// CRT `operator delete` (0x8213AE)
    pub fn operator_delete(p: GPtr<u8>) = cdecl 0x8213AE => unported;
    /// CRT `free` (0x8214BD)
    pub fn crt_free(p: GPtr<u8>) = cdecl 0x8214BD => unported;
    /// CRT `sprintf(buf, fmt, arg)` (0x821BB5)
    pub fn sprintf1(buf: GPtr<u8>, fmt: GPtr<u8>, a: u32) -> i32 = cdecl 0x821BB5 => unported;
    /// `CKeyGen::GetUppercaseKey(const char*)` (0x53CF30)
    pub fn get_uppercase_key(s: GPtr<u8>) -> u32 = cdecl 0x53CF30 => unported;
    /// `CMemoryMgr::Malloc(uint32, const char*)` (0x72F420)
    pub fn memory_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F420 => unported;
    /// `CMemoryMgr::Free(void*)` (0x72F430)
    pub fn memory_free(p: GPtr<u8>) = cdecl 0x72F430 => unported;
    /// `FxPrim_c::FxPrim_c()` (0x4A9F20)
    pub fn prim_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A9F20 => unported;
    /// `FxFrustumInfo_c::FxFrustumInfo_c()` (0x4A9F70)
    pub fn frustum_info_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A9F70 => unported;
    /// `FxMemoryPool_c::FxMemoryPool_c()` (0x4A9C10)
    pub fn memory_pool_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A9C10 => fx_memory_pool_ctor;
    /// `FxManager_c::Init()` (0x4A98E0)
    pub fn fx_manager_init(this: GPtr<u8>) = thiscall 0x4A98E0 => unported;
    /// `FxManager_c::LoadFxProject(const char*)` (0x5C2420)
    pub fn load_fx_project(this: GPtr<u8>, path: GPtr<u8>) = thiscall 0x5C2420 => unported;
    /// `FxManager_c::SetWindData(CVector*, float*)` (0x4A93E0)
    pub fn set_wind_data(this: GPtr<u8>, dir: u32, strength: u32) = thiscall 0x4A93E0 => fx_manager_set_wind_data;
    /// `FxManager_c::CalcFrustumInfo(RwCamera*)` (0x4A9140)
    pub fn calc_frustum_info(this: GPtr<u8>, cam: GPtr<u8>) = thiscall 0x4A9140 => unported;
    /// `FxManager_c::DestroyFxSystem(FxSystem_c*)` (0x4A9810)
    pub fn destroy_fx_system(this: GPtr<u8>, sys: GPtr<u8>) = thiscall 0x4A9810 => unported;
    /// `FxManager_c::CreateFxSystem(const char*, ...)` (0x4A9BE0)
    pub fn create_fx_system_name(this: GPtr<u8>, name: GPtr<u8>, m: GPtr<u8>, parent: GPtr<u8>, ignore_bounds: u32) -> GPtr<u8> = thiscall 0x4A9BE0 => fx_manager_create_fx_system_name;
    /// `FxManager_c::FxRwMatrixCreate()` (0x4A9440)
    pub fn fx_rw_matrix_create(this: GPtr<u8>) -> u32 = thiscall 0x4A9440 => fx_manager_fx_rw_matrix_create;
    /// `FxManager_c::FxRwMatrixDestroy(RwMatrix*)` (0x4A9460)
    pub fn fx_rw_matrix_destroy(this: GPtr<u8>, m: u32) = thiscall 0x4A9460 => fx_manager_fx_rw_matrix_destroy;
    /// `FxManager_c::ReturnParticle(FxEmitterPrt_c*)` (0x4A93B0)
    pub fn return_particle(this: GPtr<u8>, prt: GPtr<u8>) = thiscall 0x4A93B0 => fx_manager_return_particle;
    /// `Fx_c::InitStaticSystems()` (0x49E660)
    pub fn init_static_systems(this: GPtr<u8>) = thiscall 0x49E660 => unported;
    /// `FxSystemBP_c::Update(float)` (0x4AA130)
    pub fn system_bp_update(this: GPtr<u8>, dt: u32) -> u32 = thiscall 0x4AA130 => fx_system_bp_update;
    /// `FxSystem_c::Update(RwCamera*, float)` (0x4AAF70)
    pub fn system_update(this: GPtr<u8>, cam: GPtr<u8>, dt: u32) -> u8 = thiscall 0x4AAF70 => unported;
    /// `FxSystem_c::Kill()` (0x4AA3F0)
    pub fn system_kill(this: GPtr<u8>) = thiscall 0x4AA3F0 => fx_system_kill;
    /// `FxSystem_c::PlayAndKill()` (0x4AA3D0)
    pub fn system_play_and_kill(this: GPtr<u8>) = thiscall 0x4AA3D0 => fx_system_play_and_kill;
    /// `FxEmitterBP_c::UpdateParticle(float, FxEmitterPrt_c*)` (0x4A21D0)
    pub fn update_particle(this: GPtr<u8>, dt: u32, prt: GPtr<u8>) -> u8 = thiscall 0x4A21D0 => unported;
    /// `CAESound::~CAESound()` (0x4EF660)
    pub fn ae_sound_dtor(this: GPtr<u8>) = thiscall 0x4EF660 => unported;
    /// `CAEFireAudioEntity::Terminate()` (0x4DCF40)
    pub fn fire_audio_terminate(this: GPtr<u8>) = thiscall 0x4DCF40 => unported;
    /// `FxPrimBP_c` virtual slot 0x14 (render) as `FxSystemBP_c::Render` calls it: `(a1, 0, a2, a3)`
    pub fn prim_bp_vrender(this: GPtr<u8>, cam: u32, zero: u32, b: u32, c: u32) = thiscall vslot 0x14 => unported;
    /// `FxPrim_c` virtual slot 0: the scalar deleting destructor
    pub fn prim_vdelete(this: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    /// `FxInfo_c` virtual slot 8 (`GetValue`) as `ProcessMovementInfo` calls it (6 arguments)
    pub fn info_vget_value(this: GPtr<u8>, a1: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32) = thiscall vslot 0x8 => unported;
}

fn copy_dwords(dst: GPtr<u8>, src: GPtr<u8>, n: isize) {
    for i in 0..n {
        wr::<u32>(dst, i * 4, rd::<u32>(src, i * 4));
    }
}

/// `List_c::List_c()` (0x4A8DD0, catalogued outside RENDER; phases B/C path only).
pub fn list_ctor_port(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0);
    wr::<u32>(this, 4, 0);
    wr::<u32>(this, 8, 0);
    this
}

/// `ListItem_c::ListItem_c()` (0x4A8DB0, catalogued outside RENDER; phases B/C path only).
pub fn list_item_ctor_port(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, 0);
    wr::<u32>(this, 4, 0);
    this
}

// ---------------------------------------------------------------------------------------------
// Fx_c

/// `Fx_c::Fx_c()` (0x49E620): the list at +0x44 constructed; returns `this`.
#[hook(addr = 0x49E620, conv = thiscall, name = "Fx_c::Constructor", expect = "56 8B F1 8D 4E 44", this = "Fx_c")]
pub fn fx_constructor(this: GPtr<u8>) -> GPtr<u8> {
    list_ctor(this.byte_add(0x44));
    this
}

/// `Fx_c::Update(RwCamera*, float)` (0x49E640): tail-jumps to `g_fxMan.Update(cam, dt)`.
#[hook(addr = 0x49E640, conv = thiscall, name = "Fx_c::Update", expect = "B9 80 AE A9 00", this = "Fx_c")]
pub fn fx_update(_this: GPtr<u8>, cam: GPtr<u8>, dt: u32) {
    fx_manager_update(g(FX_MAN), cam, dt);
}

/// `Fx_c::SetFxQuality(FxQuality_e)` (0x49EA40): +0x54.
#[hook(addr = 0x49EA40, conv = thiscall, name = "Fx_c::SetFxQuality", expect = "8B 44 24 04 89 41 54", this = "Fx_c")]
pub fn fx_set_fx_quality(this: GPtr<u8>, q: u32) {
    wr(this, 0x54, q);
}

/// `Fx_c::GetFxQuality()` (0x49EA50): +0x54. Not hooked (4-byte entry).
pub fn fx_get_fx_quality(this: GPtr<u8>) -> u32 {
    rd(this, 0x54)
}

// ---------------------------------------------------------------------------------------------
// FxEmitter_c

/// `FxEmitter_c::Init(FxPrimBP_c*, FxSystem_c*)` (0x4A2550): +4 / +8 = the arguments, +0x10 = 0;
/// returns true (AL = 1).
#[hook(addr = 0x4A2550, conv = thiscall, name = "FxEmitter_c::Init_Reversed", expect = "8B 44 24 04 8B 54 24 08", this = "FxEmitter_c")]
pub fn fx_emitter_init(this: GPtr<u8>, bp: u32, system: u32) -> u8 {
    wr(this, 4, bp);
    wr(this, 8, system);
    wr::<u32>(this, 0x10, 0);
    1
}

/// `FxEmitter_c::Reset()` (0x4A2570): +0x10 = 0.
#[hook(addr = 0x4A2570, conv = thiscall, name = "FxEmitter_c::Reset_Reversed", expect = "C7 41 10 00 00 00 00", this = "FxEmitter_c")]
pub fn fx_emitter_reset(this: GPtr<u8>) {
    wr::<u32>(this, 0x10, 0);
}

// ---------------------------------------------------------------------------------------------
// FxInterpInfo*_c constructors: +4 (byte) = 0, +8 = 0, vtable, +0xC = 0; return `this`.

fn interp_info_ctor(this: GPtr<u8>, vt: u32) -> GPtr<u8> {
    wr::<u8>(this, 4, 0);
    wr::<u32>(this, 8, 0);
    wr::<u32>(this, 0, vt);
    wr::<u32>(this, 0xC, 0);
    this
}

/// `FxInterpInfoFloat_c::FxInterpInfoFloat_c()` (0x4A8440; the vtable store and the +0xC store run
/// in the protection chunk 0x406A71 and the mis-split tail 0x4A8450).
#[hook(addr = 0x4A8440, conv = thiscall, name = "FxInterpInfoFloat_c::FxInterpInfoFloat_c", expect = "8B C1 33 C9 88 48 04")]
pub fn fx_interp_info_float_ctor(this: GPtr<u8>) -> GPtr<u8> {
    interp_info_ctor(this, VT_INTERP_FLOAT)
}

/// `FxInterpInfoU255_c::FxInterpInfoU255_c()` (0x4A87D0).
#[hook(addr = 0x4A87D0, conv = thiscall, name = "FxInterpInfoU255_c::FxInterpInfoU255_c", expect = "8B C1 33 C9 88 48 04")]
pub fn fx_interp_info_u255_ctor(this: GPtr<u8>) -> GPtr<u8> {
    interp_info_ctor(this, VT_INTERP_U255)
}

/// `FxInterpInfo32_c::FxInterpInfo32_c()` (0x4A8990).
#[hook(addr = 0x4A8990, conv = thiscall, name = "FxInterpInfo32_c::FxInterpInfo32_c", expect = "8B C1 33 C9 88 48 04")]
pub fn fx_interp_info_32_ctor(this: GPtr<u8>) -> GPtr<u8> {
    interp_info_ctor(this, VT_INTERP_32)
}

/// `FxInterpInfo255_c::FxInterpInfo255_c()` (0x4A8B50).
#[hook(addr = 0x4A8B50, conv = thiscall, name = "FxInterpInfo255_c::FxInterpInfo255_c", expect = "8B C1 33 C9 88 48 04")]
pub fn fx_interp_info_255_ctor(this: GPtr<u8>) -> GPtr<u8> {
    interp_info_ctor(this, VT_INTERP_255)
}

// ---------------------------------------------------------------------------------------------
// FxManager_c

/// `FxManager_c::GetFrustumInfo()` (0x4A9130): `this + 0x34`. Not hooked (4-byte entry).
pub fn fx_manager_get_frustum_info(this: GPtr<u8>) -> GPtr<u8> {
    this.byte_add(0x34)
}

/// `FxManager_c::ReturnParticle(FxEmitterPrt_c*)` (0x4A93B0): tail-jumps to the particle list's
/// (`+0x1C`) `AddItem`.
#[hook(addr = 0x4A93B0, conv = thiscall, name = "FxManager_c::ReturnParticle", expect = "83 C1 1C E9 38 FA FF FF", this = "FxManager_c")]
pub fn fx_manager_return_particle(this: GPtr<u8>, prt: GPtr<u8>) {
    list_add_item(this.byte_add(0x1C), prt);
}

/// `FxManager_c::GetParticle(bool)` (0x4A93C0): null when the flag byte is set, else the particle
/// list's (`+0x1C`) `RemoveHead()`.
#[hook(addr = 0x4A93C0, conv = thiscall, name = "FxManager_c::GetParticle", expect = "8A 44 24 04 84 C0", this = "FxManager_c")]
pub fn fx_manager_get_particle(this: GPtr<u8>, flag: u32) -> GPtr<u8> {
    if flag as u8 != 0 {
        return GPtr::NULL;
    }
    list_remove_head(this.byte_add(0x1C))
}

/// `FxManager_c::SetWindData(CVector*, float*)` (0x4A93E0): +0x2C / +0x30 = the two pointers.
#[hook(addr = 0x4A93E0, conv = thiscall, name = "FxManager_c::SetWindData", expect = "8B 44 24 04 8B 54 24 08", this = "FxManager_c")]
pub fn fx_manager_set_wind_data(this: GPtr<u8>, dir: u32, strength: u32) {
    wr(this, 0x2C, dir);
    wr(this, 0x30, strength);
}

/// `FxManager_c::FxRwMatrixCreate()` (0x4A9440): `matrices[+0x88]` (array at +0x8C) and the index
/// post-incremented.
#[hook(addr = 0x4A9440, conv = thiscall, name = "FxManager_c::FxRwMatrixCreate", expect = "8B 91 88 00 00 00", this = "FxManager_c")]
pub fn fx_manager_fx_rw_matrix_create(this: GPtr<u8>) -> u32 {
    let i: i32 = rd(this, 0x88);
    let m: u32 = rd(this, (i.wrapping_mul(4) as isize).wrapping_add(0x8C));
    wr(this, 0x88, i.wrapping_add(1));
    m
}

/// `FxManager_c::FxRwMatrixDestroy(RwMatrix*)` (0x4A9460): decrements the index +0x88 (the argument is
/// unused).
#[hook(addr = 0x4A9460, conv = thiscall, name = "FxManager_c::FxRwMatrixDestroy", expect = "FF 89 88 00 00 00", this = "FxManager_c")]
pub fn fx_manager_fx_rw_matrix_destroy(this: GPtr<u8>, _m: u32) {
    let i: i32 = rd(this, 0x88);
    wr(this, 0x88, i.wrapping_sub(1));
}

/// `FxManager_c::CreateFxSystem(const char* name, RwMatrix* m, RwMatrix* parent, bool ignoreBounds)`
/// (0x4A9BE0): `CreateFxSystem(FindFxSystemBP(name), m, parent, ignoreBounds)`.
#[hook(addr = 0x4A9BE0, conv = thiscall, name = "FxManager_c::CreateFxSystem-4", expect = "8B 44 24 10 8B 54 24 08", this = "FxManager_c")]
pub fn fx_manager_create_fx_system_name(
    this: GPtr<u8>,
    name: GPtr<u8>,
    m: GPtr<u8>,
    parent: GPtr<u8>,
    ignore_bounds: u32,
) -> GPtr<u8> {
    let bp = find_fx_system_bp(this, name);
    create_fx_system_bp(this, bp, m, parent, ignore_bounds)
}

// ---------------------------------------------------------------------------------------------
// FxMemoryPool_c / operator new

/// `FxMemoryPool_c::FxMemoryPool_c()` (0x4A9C10): returns `this`. Not hooked (3-byte entry).
pub fn fx_memory_pool_ctor(this: GPtr<u8>) -> GPtr<u8> {
    this
}

/// `FxMemoryPool_c::GetMem(int32 size, int32 align)` (0x4A9CA0): the position +8 rounded up to
/// `align` (unsigned `div`; the round-up runs when the remainder is > 0 as a signed int), then
/// `base(+0) + pos` returned and `pos += size`.
#[hook(addr = 0x4A9CA0, conv = thiscall, name = "FxMemoryPool_c::GetMem", expect = "56 8B 71 08 57", this = "FxMemoryPool_c")]
pub fn fx_memory_pool_get_mem(this: GPtr<u8>, size: u32, align: u32) -> GPtr<u8> {
    let pos: u32 = rd(this, 8);
    let rem = pos % align;
    if (rem as i32) > 0 {
        wr(this, 8, pos.wrapping_sub(rem).wrapping_add(align));
    }
    let pos: u32 = rd(this, 8);
    let base: u32 = rd(this, 0);
    wr(this, 8, pos.wrapping_add(size));
    GPtr::from_addr(base.wrapping_add(pos) as usize)
}

/// `FxMemoryPool_c::Optimise()` (0x4A9CD0): `base = CMemoryMgr::Realloc(base, pos, 0)`, size +4 = pos.
#[hook(addr = 0x4A9CD0, conv = thiscall, name = "FxMemoryPool_c::Optimise", expect = "56 8B F1 8B 46 08", this = "FxMemoryPool_c")]
pub fn fx_memory_pool_optimise(this: GPtr<u8>) {
    let p = memory_realloc(rd(this, 0), rd(this, 8), 0);
    let pos: u32 = rd(this, 8);
    wr(this, 0, p);
    wr(this, 4, pos);
}

/// `FxEmitterPrt_c::operator new[](uint32)` (0x4A18B0): `g_fxMan` pool `GetMem(size, 4)`.
#[hook(addr = 0x4A18B0, conv = cdecl, name = "FxEmitterPrt_c::operator new[]", expect = "8B 44 24 04 6A 04")]
pub fn fx_emitter_prt_new_array(size: u32) -> GPtr<u8> {
    pool_get_mem(g(FX_POOL), size, 4)
}

/// `FxSystemBP_c::operator new(uint32)` (0x4AA100): `g_fxMan` pool `GetMem(size, 4)`.
#[hook(addr = 0x4AA100, conv = cdecl, name = "FxSystemBP_c::operator new", expect = "8B 44 24 04 6A 04")]
pub fn fx_system_bp_new(size: u32) -> GPtr<u8> {
    pool_get_mem(g(FX_POOL), size, 4)
}

// ---------------------------------------------------------------------------------------------
// FxPrim_c / FxPrimBP_c / FxEmitterBP_c

/// `FxPrim_c::Enable(bool)` (0x4A9F50): byte +0xC.
#[hook(addr = 0x4A9F50, conv = thiscall, name = "FxPrim_c::Enable", expect = "8A 44 24 04 88 41 0C")]
pub fn fx_prim_enable(this: GPtr<u8>, on: u32) {
    wr(this, 0xC, on as u8);
}

/// `FxPrimBP_c::FxPrimBP_c()` (0x4A9CF0): vtable, the list at +0x20, +0xC..+0x18 = 0; returns `this`.
#[hook(addr = 0x4A9CF0, conv = thiscall, name = "FxPrimBP_c::FxPrimBP_c", expect = "56 8B F1 8D 4E 20", this = "FxPrimBP_c")]
pub fn fx_prim_bp_constructor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0, VT_PRIM_BP);
    list_ctor(this.byte_add(0x20));
    for o in [0xC, 0x10, 0x14, 0x18] {
        wr::<u32>(this, o, 0);
    }
    this
}

/// `FxEmitterBP_c::FxEmitterBP_c()` (0x4A18D0): the `FxPrimBP_c` base, vtable, byte +4 = 0.
#[hook(addr = 0x4A18D0, conv = thiscall, name = "FxEmitterBP_c::Constructor", expect = "56 8B F1 E8 18 84 00 00", this = "FxEmitterBP_c")]
pub fn fx_emitter_bp_constructor(this: GPtr<u8>) -> GPtr<u8> {
    prim_bp_ctor(this);
    wr::<u32>(this, 0, VT_EMITTER_BP);
    wr::<u8>(this, 4, 0);
    this
}

/// `FxEmitterBP_c::IsFxInfoPresent(int32 type)` (0x4A24D0): any of the `+0x2C` infos (array `+0x30`)
/// whose u16 type (+4) equals the argument (compared as a full dword).
#[hook(addr = 0x4A24D0, conv = thiscall, name = "FxEmitterBP_c::IsFxInfoPresent", expect = "8B 51 2C 56 33 C0", this = "FxEmitterBP_c")]
pub fn fx_emitter_bp_is_fx_info_present(this: GPtr<u8>, ty: u32) -> u8 {
    let n: i32 = rd(this, 0x2C);
    let arr: GPtr<u8> = rd(this, 0x30);
    let mut i = 0;
    while i < n {
        let info: GPtr<u8> = rd(arr, (i as isize) * 4);
        if u32::from(rd::<u16>(info, 4)) == ty {
            return 1;
        }
        i += 1;
    }
    0
}

// ---------------------------------------------------------------------------------------------
// FxSystemBP_c

/// `FxSystemBP_c::FxSystemBP_c()` (0x4AA0D0): the `ListItem_c` base, +0x20 = 0.
#[hook(addr = 0x4AA0D0, conv = thiscall, name = "FxSystemBP_c::Constructor", expect = "56 8B F1 E8 D8 EC FF FF", this = "FxSystemBP_c")]
pub fn fx_system_bp_constructor(this: GPtr<u8>) -> GPtr<u8> {
    list_item_ctor(this);
    wr::<u32>(this, 0x20, 0);
    this
}

/// `FxSystemBP_c::Update(float)` (0x4AA130): virtual `Update(dt)` (slot 0x10) of each of the
/// `(int8)+0x1B` prims (array +0x1C); returns 0 (EAX).
#[hook(addr = 0x4AA130, conv = thiscall, name = "FxSystemBP_c::Update", expect = "56 57 8B F9 8A 47 1B", this = "FxSystemBP_c")]
pub fn fx_system_bp_update(this: GPtr<u8>, dt: u32) -> u32 {
    let mut i = 0i32;
    while i < i32::from(rd::<i8>(this, 0x1B)) {
        let arr: GPtr<u8> = rd(this, 0x1C);
        prim_bp_vupdate(rd(arr, (i as isize) * 4), dt);
        i += 1;
    }
    0
}

// ---------------------------------------------------------------------------------------------
// FxSystem_c

/// `FxSystem_c::FxSystem_c()` (0x4AAF00): the `ListItem_c` base; +0x8C, +0x100, +8, +0x74 = 0, the
/// +0x7C member's vtable, flag bit 0x20 of +0x62 cleared.
#[hook(addr = 0x4AAF00, conv = thiscall, name = "FxSystem_c::Constructor", expect = "56 8B F1 E8 A8 DE FF FF")]
pub fn fx_system_constructor(this: GPtr<u8>) -> GPtr<u8> {
    list_item_ctor(this);
    wr::<u32>(this, 0x8C, 0);
    wr::<u32>(this, 0x100, 0);
    wr::<u32>(this, 0x7C, VT_SYSTEM_7C);
    wr::<u32>(this, 8, 0);
    wr::<u32>(this, 0x74, 0);
    let f: u8 = rd(this, 0x62);
    wr(this, 0x62, f & 0xDF);
    this
}

/// `FxSystem_c::Stop()` (0x4AA390): play status +0x50 = 1, +0x54 = 0, virtual slot 0xC of each prim
/// (`(int8)bp->+0x1B` of them, array +0x78; the blueprint +8 re-read every iteration).
#[hook(addr = 0x4AA390, conv = thiscall, name = "FxSystem_c::Stop", expect = "56 8B F1 8B 46 08")]
pub fn fx_system_stop(this: GPtr<u8>) {
    wr::<u8>(this, 0x50, 1);
    wr::<u32>(this, 0x54, 0);
    let mut i = 0i32;
    loop {
        let bp: GPtr<u8> = rd(this, 8);
        if i >= i32::from(rd::<i8>(bp, 0x1B)) {
            break;
        }
        let arr: GPtr<u8> = rd(this, 0x78);
        prim_vstop(rd(arr, (i as isize) * 4));
        i += 1;
    }
}

/// `FxSystem_c::PlayAndKill()` (0x4AA3D0): `Play()`, then kill status +0x51 = 1 unless the blueprint's
/// (+8) byte +0x1A (`m_nPlayMode`) is set.
#[hook(addr = 0x4AA3D0, conv = thiscall, name = "FxSystem_c::PlayAndKill", expect = "56 8B F1 E8 18 FF FF FF")]
pub fn fx_system_play_and_kill(this: GPtr<u8>) {
    system_play(this);
    let bp: GPtr<u8> = rd(this, 8);
    if rd::<u8>(bp, 0x1A) == 0 {
        wr::<u8>(this, 0x51, 1);
    }
}

/// `FxSystem_c::Kill()` (0x4AA3F0): `Stop()`, kill status +0x51 = 2.
#[hook(addr = 0x4AA3F0, conv = thiscall, name = "FxSystem_c::Kill", expect = "56 8B F1 E8 98 FF FF FF")]
pub fn fx_system_kill(this: GPtr<u8>) {
    system_stop(this);
    wr::<u8>(this, 0x51, 2);
}

/// `FxSystem_c::EnablePrim(int32 index, bool on)` (0x4AA610): `prims[index]->Enable(on)` (array +0x78;
/// the flag dword passed on as pushed).
#[hook(addr = 0x4AA610, conv = thiscall, name = "FxSystem_c::EnablePrim", expect = "8B 44 24 08 8B 49 78")]
pub fn fx_system_enable_prim(this: GPtr<u8>, index: i32, on: u32) {
    let arr: GPtr<u8> = rd(this, 0x78);
    prim_enable(rd(arr, (index.wrapping_mul(4)) as isize), on);
}

/// `FxSystem_c::SetMatrix(RwMatrix*)` (0x4AA630): with flag bit 1 of +0x62 (own matrix) the 0x40 bytes
/// are copied into the matrix +0xC points at, else +0xC = the argument.
#[hook(addr = 0x4AA630, conv = thiscall, name = "FxSystem_c::SetMatrix", expect = "8B C1 F6 40 62 01")]
pub fn fx_system_set_matrix(this: GPtr<u8>, m: GPtr<u8>) {
    if rd::<u8>(this, 0x62) & 1 != 0 {
        copy_dwords(rd(this, 0xC), m, 16);
    } else {
        wr(this, 0xC, m);
    }
}

/// `FxSystem_c::SetOffsetPos(CVector*)` (0x4AA660): the local matrix's (+0x10) position (+0x40) set,
/// then `RwMatrixUpdate(+0x10)`.
#[hook(addr = 0x4AA660, conv = thiscall, name = "FxSystem_c::SetOffsetPos", expect = "8B 44 24 04 8B 10")]
pub fn fx_system_set_offset_pos(this: GPtr<u8>, pos: GPtr<u8>) {
    copy_dwords(this.byte_add(0x40), pos, 3);
    rw_matrix_update(this.byte_add(0x10));
}

/// `FxSystem_c::SetConstTime(bool, float)` (0x4AA6C0): byte +0x52 = flag, word +0x5C =
/// `_ftol(time * 256.0)`.
#[hook(addr = 0x4AA6C0, conv = thiscall, name = "FxSystem_c::SetConstTime", expect = "D9 44 24 08 8A 44 24 04")]
pub fn fx_system_set_const_time(this: GPtr<u8>, on: u32, time: u32) {
    let v = x(f32::from_bits(time)).fmul_m(k(K_256));
    wr::<u8>(this, 0x52, on as u8);
    wr::<u16>(this, 0x5C, ftol32(v) as u16);
}

/// `FxSystem_c::SetRateMult(float)` (0x4AA6F0): word +0x5E = `_ftol(mult * 1000.0)`.
#[hook(addr = 0x4AA6F0, conv = thiscall, name = "FxSystem_c::SetRateMult", expect = "D9 44 24 04 56")]
pub fn fx_system_set_rate_mult(this: GPtr<u8>, mult: u32) {
    let v = x(f32::from_bits(mult)).fmul_m(k(K_1000));
    wr::<u16>(this, 0x5E, ftol32(v) as u16);
}

/// `FxSystem_c::SetVelAdd(CVector*)` (0x4AA730): +0x68..+0x74 = the vector (dword copies).
#[hook(addr = 0x4AA730, conv = thiscall, name = "FxSystem_c::SetVelAdd", expect = "8B 44 24 04 8B 10")]
pub fn fx_system_set_vel_add(this: GPtr<u8>, v: GPtr<u8>) {
    copy_dwords(this.byte_add(0x68), v, 3);
}

/// `FxSystem_c::CopyParentMatrix()` (0x4AA890): a new `RwMatrixCreate()` at +0xC receives a copy of
/// the old parent matrix (0x40 bytes; the `mov ecx, 0x10` runs in the protection chunk 0x4076B1),
/// flag bit 1 of +0x62 set.
#[hook(addr = 0x4AA890, conv = thiscall, name = "FxSystem_c::CopyParentMatrix", expect = "53 56 8B D9 8B 73 0C")]
pub fn fx_system_copy_parent_matrix(this: GPtr<u8>) {
    let old: GPtr<u8> = rd(this, 0xC);
    let m = rw_matrix_create();
    wr(this, 0xC, m);
    copy_dwords(m, old, 16);
    let f: u8 = rd(this, 0x62);
    wr(this, 0x62, f | 1);
}

/// `FxSystem_c::GetCompositeMatrix(RwMatrix* out)` (0x4AA8C0): without a parent (+0xC) a copy of the
/// local matrix (+0x10), else `RwMatrixMultiply(out, local, parent)`. EAX: 0 / the multiply's result.
#[hook(addr = 0x4AA8C0, conv = thiscall, name = "FxSystem_c::GetCompositeMatrix", expect = "8B 41 0C 85 C0")]
pub fn fx_system_get_composite_matrix(this: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> {
    let parent: GPtr<u8> = rd(this, 0xC);
    if parent.is_null() {
        copy_dwords(out, this.byte_add(0x10), 16);
        return GPtr::NULL;
    }
    rw_matrix_multiply(out, this.byte_add(0x10), parent)
}

/// `FxSystem_c::GetPlayStatus()` (0x4AA900): `(int8)+0x50`. Not hooked (the `ret` is at +4).
pub fn fx_system_get_play_status(this: GPtr<u8>) -> i32 {
    i32::from(rd::<i8>(this, 0x50))
}

/// `FxSystem_c::SetLocalParticles(bool)` (0x4AA910): flag bit 2 of +0x62 = bit 0 of the argument.
#[hook(addr = 0x4AA910, conv = thiscall, name = "FxSystem_c::SetLocalParticles", expect = "8A 44 24 04 8A 51 62")]
pub fn fx_system_set_local_particles(this: GPtr<u8>, on: u32) {
    let d: u8 = rd(this, 0x62);
    let a = (on as u8).wrapping_shl(1);
    wr(this, 0x62, d ^ ((a ^ d) & 2));
}

/// `FxSystem_c::IsVisible()` (0x4AAF30): visible when `GetBoundingSphereWld` fails, else
/// `g_fxMan.GetFrustumInfo()->IsCollision(&sphere)` (EAX tested). The local `FxSphere_c` (0x14 bytes)
/// has its last dword zeroed first.
#[hook(addr = 0x4AAF30, conv = thiscall, name = "FxSystem_c::IsVisible", expect = "83 EC 14 8D 44 24 00")]
pub fn fx_system_is_visible(this: GPtr<u8>) -> u8 {
    let mut loc = [0u32; 5];
    let lp = GPtr::<u32>::from_raw(loc.as_mut_ptr()).cast::<u8>();
    if system_get_bounding_sphere_wld(this, lp) == 0 {
        return 1;
    }
    let fr = get_frustum_info(g(FX_MAN));
    u8::from(frustum_is_collision(fr, lp) != 0)
}

// ---------------------------------------------------------------------------------------------
// Second pass: Fx_c / FxManager_c / FxSystem_c / FxSystemBP_c / FxEmitterBP_c workers

/// `Fx_c::Init()` (0x49EA90): `g_fxMan.Init()`, `LoadFxProject("models\\effects.fxp")`,
/// `SetWindData(&CWeather::WindDir, &CWeather::Wind)`, `InitStaticSystems()`, +0x50 = 0; returns true.
#[hook(addr = 0x49EA90, conv = thiscall, name = "Fx_c::Init", expect = "56 8B F1 B9 80 AE A9 00", this = "Fx_c")]
pub fn fx_init(this: GPtr<u8>) -> u8 {
    fx_manager_init(g(FX_MAN));
    load_fx_project(g(FX_MAN), g(FX_PROJECT_PATH));
    set_wind_data(g(FX_MAN), WEATHER_WIND_DIR as u32, WEATHER_WIND as u32);
    init_static_systems(this);
    wr::<u32>(this, 0x50, 0);
    1
}

/// `Fx_c::DestroyEntityFx(CEntity*)` (0x4A1280): every node of the entity-FX list (+0x44) whose entity
/// (+0xC) matches is unlinked, its system (+8) killed, the node destroyed and freed (the next pointer
/// is read before).
#[hook(addr = 0x4A1280, conv = thiscall, name = "Fx_c::DestroyEntityFx", expect = "53 8D 59 44 56", this = "Fx_c")]
pub fn fx_destroy_entity_fx(this: GPtr<u8>, entity: u32) {
    let list = this.byte_add(0x44);
    let mut n: GPtr<u8> = rd(list, 0);
    while !n.is_null() {
        let next: GPtr<u8> = rd(n, 4);
        if rd::<u32>(n, 0xC) == entity {
            list_remove_item(list, n);
            system_kill(rd(n, 8));
            list_item_dtor(n);
            crt_free(n);
        }
        n = next;
    }
}

/// `RenderBegin(RwRaster*, RwMatrix* ltm, uint32 flags)` (0x4A13B0, cdecl): the `g_fx` immediate-mode
/// state (+0x58 / +0x5C counts = 0, +0x60 flags, +0x64 raster, +0x68 LTM, +0x6C = the vertex buffer
/// 0xC4D958), then the texture raster render state (1) set when it differs. The original lets
/// `RwRenderStateGet` write into its own first argument slot; the port uses a local.
#[hook(addr = 0x4A13B0, conv = cdecl, name = "Fx_c::RenderBegin", expect = "8B 44 24 08 8B 4C 24 0C")]
pub fn fx_render_begin(raster: u32, ltm: u32, flags: u32) {
    let fx = g::<u8>(FX);
    wr(fx, FX_RENDER_LTM, ltm);
    wr::<u32>(fx, FX_TXD_INDICES, 0);
    wr::<u32>(fx, FX_TXD_VERTS, 0);
    wr(fx, FX_RENDER_RASTER, raster);
    wr(fx, FX_RENDER_FLAGS, flags);
    wr(fx, FX_RENDER_VERTS_PTR, FX_VERTEX_BUFFER);
    let mut cur = [raster];
    rw_render_state_get(1, GPtr::<u32>::from_raw(cur.as_mut_ptr()).cast());
    if cur[0] != raster {
        rw_render_state_set(1, raster);
    }
}

/// `Fx_c::TriggerWaterSplash(CVector&)` (0x4A1070): within 25 m of the camera (`|cam - pos|^2`, summed
/// x, z, y on the x87 stack, `> 625.0` skips; unordered creates) the big water splash system is created
/// at `pos` and played-and-killed.
#[hook(addr = 0x4A1070, conv = thiscall, name = "Fx_c::TriggerWaterSplash", expect = "A1 3C F0 B6 00", this = "Fx_c")]
pub fn fx_trigger_water_splash(_this: GPtr<u8>, pos: GPtr<u8>) {
    let c = sa_world::world::common::cam_pos().cast::<u8>();
    let dx = x(rd(c, 0)).fsub_m(rd(pos, 0));
    let dy = x(rd(c, 4)).fsub_m(rd(pos, 4));
    let dz = x(rd(c, 8)).fsub_m(rd(pos, 8));
    let s = dx.fmul(dx).fadd(dz.fmul(dz)).fadd(dy.fmul(dy));
    if !jne(s, x(k(K_625)), 0x41) {
        return;
    }
    let sys = create_fx_system_name(g(FX_MAN), g(WATER_SPLASH_NAME), pos, GPtr::NULL, 0);
    if !sys.is_null() {
        system_play_and_kill(sys);
    }
}

/// `FxEmitterBP_c::CreateInstance()` (0x4A2B40): `new FxEmitter_c` (0x14 bytes: the `FxPrim_c` base and
/// the emitter vtable); null when the allocation fails. (SEH frame not reproduced: nothing throws.)
#[hook(addr = 0x4A2B40, conv = thiscall, name = "FxEmitterBP_c::CreateInstance", expect = "6A FF 68 9B 8E 83 00", this = "FxEmitterBP_c")]
pub fn fx_emitter_bp_create_instance(_this: GPtr<u8>) -> GPtr<u8> {
    let p = operator_new(0x14);
    if p.is_null() {
        return GPtr::NULL;
    }
    prim_ctor(p);
    wr::<u32>(p, 0, VT_EMITTER);
    p
}

/// `FxEmitterBP_c::Update(float)` (0x4A2BC0): returns the particle count of the list +0x20 (0: nothing
/// else runs). Each particle: a system kill status (+0x51) of 3 becomes 2; unless the system is stopped
/// (+0x50 == 2), `UpdateParticle(dt, prt)` true unlinks it and returns it to `g_fxMan`.
#[hook(addr = 0x4A2BC0, conv = thiscall, name = "FxEmitterBP_c::Update", expect = "51 53 8B D9 55", this = "FxEmitterBP_c")]
pub fn fx_emitter_bp_update(this: GPtr<u8>, dt: u32) -> i32 {
    let list = this.byte_add(0x20);
    let n = list_get_num_items(list);
    if n == 0 {
        return 0;
    }
    let mut p: GPtr<u8> = rd(list, 0);
    while !p.is_null() {
        let sys: GPtr<u8> = rd(p, 0x28);
        if rd::<u8>(sys, 0x51) == 3 {
            wr::<u8>(sys, 0x51, 2);
        }
        let sys: GPtr<u8> = rd(p, 0x28);
        if rd::<u8>(sys, 0x50) != 2 && update_particle(this, dt, p) != 0 {
            let next: GPtr<u8> = rd(p, 4);
            list_remove_item(list, p);
            return_particle(g(FX_MAN), p);
            p = next;
        } else {
            p = rd(p, 4);
        }
    }
    n
}

/// `FxInfoManager_c::ProcessMovementInfo(...)` (0x4A4A10, 6 stack arguments, the last the output
/// `MovementInfo_t*`): the output's +0x18..+0x24 and bytes +0x28 / +0x29 cleared, then virtual slot 8
/// of every movement info (flag 0x20 of byte +5) from index `(int8)+8` to `(int8)+9` (re-read).
#[hook(addr = 0x4A4A10, conv = thiscall, name = "FxInfoManager_c::ProcessMovementInfo", expect = "33 C0 53 56 8B 74 24 20", this = "FxInfoManager_c")]
#[allow(clippy::too_many_arguments)]
pub fn fx_info_manager_process_movement_info(
    this: GPtr<u8>,
    a1: u32,
    a2: u32,
    a3: u32,
    a4: u32,
    a5: u32,
    out: GPtr<u8>,
) {
    for o in [0x18, 0x1C, 0x20, 0x24] {
        wr::<u32>(out, o, 0);
    }
    wr::<u8>(out, 0x28, 0);
    wr::<u8>(out, 0x29, 0);
    let mut i = i32::from(rd::<i8>(this, 8));
    while i < i32::from(rd::<i8>(this, 9)) {
        let arr: GPtr<u8> = rd(this, 4);
        let info: GPtr<u8> = rd(arr, (i as isize) * 4);
        if rd::<u8>(info, 5) & 0x20 != 0 {
            info_vget_value(info, a1, a2, a3, a4, a5, out.addr() as u32);
        }
        i += 1;
    }
}

/// `FxManager_c::FxManager_c()` (0x4A9470): the three lists (+0, +0xC, +0x1C), the frustum info (+0x34),
/// the memory pool (+0xAC), +0x18 = 0. (SEH frame not reproduced: nothing throws.)
#[hook(addr = 0x4A9470, conv = thiscall, name = "FxManager_c::Constructor", expect = "6A FF 68 6E 98 83 00", this = "FxManager_c")]
pub fn fx_manager_constructor(this: GPtr<u8>) -> GPtr<u8> {
    list_ctor(this);
    list_ctor(this.byte_add(0xC));
    list_ctor(this.byte_add(0x1C));
    frustum_info_ctor(this.byte_add(0x34));
    memory_pool_ctor(this.byte_add(0xAC));
    wr::<u32>(this, 0x18, 0);
    this
}

/// `FxManager_c::FindFxSystemBP(const char*)` (0x4A9360): the blueprint (list +0, next +4) whose key
/// (+8) is `CKeyGen::GetUppercaseKey(name)`; when none, a debug message is `sprintf`ed into a 0x80-byte
/// local and null returned.
#[hook(addr = 0x4A9360, conv = thiscall, name = "FxManager_c::FindFxSystemBP", expect = "81 EC 80 00 00 00", this = "FxManager_c")]
pub fn fx_manager_find_fx_system_bp(this: GPtr<u8>, name: GPtr<u8>) -> GPtr<u8> {
    let key = get_uppercase_key(name);
    let mut bp: GPtr<u8> = rd(this, 0);
    while !bp.is_null() {
        if rd::<u32>(bp, 8) == key {
            return bp;
        }
        bp = rd(bp, 4);
    }
    let mut buf = [0u32; 0x20];
    sprintf1(
        GPtr::<u32>::from_raw(buf.as_mut_ptr()).cast(),
        g(FIND_BP_FMT),
        name.addr() as u32,
    );
    GPtr::NULL
}

/// `FxManager_c::Update(RwCamera*, float)` (0x4A9A80): frustum info, every blueprint's `Update(dt)`,
/// then every system's `Update(cam, dt)` (next read first), a true result destroying it.
#[hook(addr = 0x4A9A80, conv = thiscall, name = "FxManager_c::Update", expect = "8B 44 24 04 53", this = "FxManager_c")]
pub fn fx_manager_update_port(this: GPtr<u8>, cam: GPtr<u8>, dt: u32) {
    calc_frustum_info(this, cam);
    let mut bp: GPtr<u8> = rd(this, 0);
    while !bp.is_null() {
        system_bp_update(bp, dt);
        bp = rd(bp, 4);
    }
    let mut s: GPtr<u8> = rd(this, 0xC);
    while !s.is_null() {
        let next: GPtr<u8> = rd(s, 4);
        if system_update(s, cam, dt) != 0 {
            destroy_fx_system(this, s);
        }
        s = next;
    }
}

/// `FxMemoryPool_c::Init()` (0x4A9C30): size +4 = 1 MiB, position +8 = 0, base +0 =
/// `CMemoryMgr::Malloc(1 MiB, 0)`; false when that fails, else the block zeroed (`rep stos` dwords then
/// the remaining bytes) and true.
#[hook(addr = 0x4A9C30, conv = thiscall, name = "FxMemoryPool_c::Init", expect = "56 57 8B F1 6A 00", this = "FxMemoryPool_c")]
pub fn fx_memory_pool_init(this: GPtr<u8>) -> u8 {
    wr::<u32>(this, 4, 0x10_0000);
    wr::<u32>(this, 8, 0);
    let p = memory_malloc(0x10_0000, 0);
    wr(this, 0, p);
    if p.is_null() {
        return 0;
    }
    let n: u32 = rd(this, 4);
    for i in 0..(n >> 2) as isize {
        wr::<u32>(p, i * 4, 0);
    }
    let base = ((n >> 2) * 4) as isize;
    for i in 0..(n & 3) as isize {
        wr::<u8>(p, base + i, 0);
    }
    1
}

/// `FxSystemBP_c::Render(RwCamera*, float, bool)` (0x4AA160, `ret 0xC`): virtual slot 0x14 of each prim
/// with `(a1, 0, a2, a3)`.
#[hook(addr = 0x4AA160, conv = thiscall, name = "FxSystemBP_c::Render", expect = "56 57 8B F9 8A 47 1B", this = "FxSystemBP_c")]
pub fn fx_system_bp_render(this: GPtr<u8>, a1: u32, a2: u32, a3: u32) {
    let mut i = 0i32;
    while i < i32::from(rd::<i8>(this, 0x1B)) {
        let arr: GPtr<u8> = rd(this, 0x1C);
        prim_bp_vrender(rd(arr, (i as isize) * 4), a1, 0, a2, a3);
        i += 1;
    }
}

/// `FxSystemBP_c::SetBoundingSphere(CVector*, float)` (0x4AA200): once (+0x20 null) and for a radius
/// `> 0.0` (`fcomp` + `test ah, 0x41`), an `FxSphere_c` (0x14 bytes, +0x10 = 0) from the FX pool holds the
/// centre and the radius (raw dword).
#[hook(addr = 0x4AA200, conv = thiscall, name = "FxSystemBP_c::SetBoundingSphere", expect = "56 8B F1 8B 46 20", this = "FxSystemBP_c")]
pub fn fx_system_bp_set_bounding_sphere(this: GPtr<u8>, centre: GPtr<u8>, radius: u32) {
    if rd::<u32>(this, 0x20) != 0 || jne(x(f32::from_bits(radius)), x(0.0), 0x41) {
        return;
    }
    let p = pool_get_mem(g(FX_POOL), 0x14, 4);
    if !p.is_null() {
        wr::<u32>(p, 0x10, 0);
    }
    wr(this, 0x20, p);
    copy_dwords(p, centre, 3);
    let s: GPtr<u8> = rd(this, 0x20);
    wr(s, 0xC, radius);
}

/// `FxSystem_c::~FxSystem_c()` (0x4AA260): blueprint +8 = null, the bounding sphere (+0x74) freed
/// (`CMemoryMgr::Free`), an owned parent matrix (+0x62 bit 1) destroyed, the audio member's (+0x7C)
/// base vtable restored and its `CAESound` (+0x84) destroyed, then the `ListItem_c` base.
#[hook(addr = 0x4AA260, conv = thiscall, name = "FxSystem_c::Destructor", expect = "6A FF 68 33 99 83 00")]
pub fn fx_system_destructor(this: GPtr<u8>) {
    wr::<u32>(this, 8, 0);
    let s: GPtr<u8> = rd(this, 0x74);
    if !s.is_null() {
        memory_free(s);
    }
    if rd::<u8>(this, 0x62) & 1 != 0 {
        rw_matrix_destroy(rd(this, 0xC));
    }
    wr::<u32>(this, 0x7C, VT_AUDIO_ENTITY);
    ae_sound_dtor(this.byte_add(0x84));
    list_item_dtor_b(this);
}

/// `FxSystem_c::Play()` (0x4AA2F0): unless stopped for good (+0x50 == 2), +0x54 = 0 and virtual slot 0xC
/// of each prim; then kill / play status (+0x51, +0x50) = 0, flags +0x62 &= 0xE7 and the time +0x64 =
/// `lo + (rand() % 10000) * 0.0001 * (hi - lo)` (blueprint +0x10 / +0x14; `rand` at 0x4AA332).
#[hook(addr = 0x4AA2F0, conv = thiscall, name = "FxSystem_c::Play", expect = "51 56 8B F1 80 7E 50 02")]
pub fn fx_system_play(this: GPtr<u8>) {
    if rd::<u8>(this, 0x50) != 2 {
        wr::<u32>(this, 0x54, 0);
        let mut i = 0i32;
        loop {
            let bp: GPtr<u8> = rd(this, 8);
            if i >= i32::from(rd::<i8>(bp, 0x1B)) {
                break;
            }
            let arr: GPtr<u8> = rd(this, 0x78);
            prim_vstop(rd(arr, (i as isize) * 4));
            i += 1;
        }
    }
    let bp: GPtr<u8> = rd(this, 8);
    wr::<u8>(this, 0x51, 0);
    wr::<u8>(this, 0x50, 0);
    let r = sa_core::orig::at_site(0x4AA337, sa_core::crt::rand);
    let rem = r.wrapping_rem(10000);
    let f: u8 = rd(this, 0x62);
    let lo: f32 = rd(bp, 0x10);
    let v = xi(rem)
        .fmul_m(k(K_RAND_SCALE))
        .fmul(x(rd(bp, 0x14)).fsub_m(lo))
        .fadd_m(lo);
    wr(this, 0x62, f & 0xE7);
    wr(this, 0x64, v.to_f32());
}

/// `FxSystem_c::Exit()` (0x4AA840): every non-null prim deleted (virtual slot 0, flag 1), the prim array
/// (+0x78) `operator delete`d, then `CAEFireAudioEntity::Terminate` of the member +0x7C (tail jump).
#[hook(addr = 0x4AA840, conv = thiscall, name = "FxSystem_c::Exit", expect = "56 8B F1 8B 46 08")]
pub fn fx_system_exit(this: GPtr<u8>) {
    let mut i = 0i32;
    loop {
        let bp: GPtr<u8> = rd(this, 8);
        if i >= i32::from(rd::<i8>(bp, 0x1B)) {
            break;
        }
        let arr: GPtr<u8> = rd(this, 0x78);
        let p: GPtr<u8> = rd(arr, (i as isize) * 4);
        if !p.is_null() {
            prim_vdelete(p, 1);
        }
        i += 1;
    }
    operator_delete(rd(this, 0x78));
    fire_audio_terminate(this.byte_add(0x7C));
}

/// `FxSystem_c::GetBoundingSphereWld(FxSphere_c*)` (0x4AAAD0): false without a bounding sphere (+0x74);
/// else the composite matrix (in a `g_fxMan` scratch matrix) transforms its centre into `out`, the
/// radius (+0xC) is copied, the scratch matrix released; true.
#[hook(addr = 0x4AAAD0, conv = thiscall, name = "FxSystem_c::GetBoundingSphereWld", expect = "53 8B D9 8B 43 74")]
pub fn fx_system_get_bounding_sphere_wld(this: GPtr<u8>, out: GPtr<u8>) -> u8 {
    if rd::<u32>(this, 0x74) == 0 {
        return 0;
    }
    let m = fx_rw_matrix_create(g(FX_MAN));
    let mp = GPtr::<u8>::from_addr(m as usize);
    let parent: GPtr<u8> = rd(this, 0xC);
    if parent.is_null() {
        copy_dwords(mp, this.byte_add(0x10), 16);
    } else {
        rw_matrix_multiply(mp, this.byte_add(0x10), parent);
    }
    rw_v3d_transform_points(out, rd(this, 0x74), 1, mp);
    let s: GPtr<u8> = rd(this, 0x74);
    wr(out, 0xC, rd::<u32>(s, 0xC));
    fx_rw_matrix_destroy(g(FX_MAN), m);
    1
}

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];

/// This file's registrations.
const OWN_HOOKS: &[HookReg] = &[
    HOOK_FX_CONSTRUCTOR,
    HOOK_FX_UPDATE,
    HOOK_FX_SET_FX_QUALITY,
    HOOK_FX_EMITTER_INIT,
    HOOK_FX_EMITTER_RESET,
    HOOK_FX_INTERP_INFO_FLOAT_CTOR,
    HOOK_FX_INTERP_INFO_U255_CTOR,
    HOOK_FX_INTERP_INFO_32_CTOR,
    HOOK_FX_INTERP_INFO_255_CTOR,
    HOOK_FX_MANAGER_RETURN_PARTICLE,
    HOOK_FX_MANAGER_GET_PARTICLE,
    HOOK_FX_MANAGER_SET_WIND_DATA,
    HOOK_FX_MANAGER_FX_RW_MATRIX_CREATE,
    HOOK_FX_MANAGER_FX_RW_MATRIX_DESTROY,
    HOOK_FX_MANAGER_CREATE_FX_SYSTEM_NAME,
    HOOK_FX_MEMORY_POOL_GET_MEM,
    HOOK_FX_MEMORY_POOL_OPTIMISE,
    HOOK_FX_EMITTER_PRT_NEW_ARRAY,
    HOOK_FX_SYSTEM_BP_NEW,
    HOOK_FX_PRIM_ENABLE,
    HOOK_FX_PRIM_BP_CONSTRUCTOR,
    HOOK_FX_EMITTER_BP_CONSTRUCTOR,
    HOOK_FX_EMITTER_BP_IS_FX_INFO_PRESENT,
    HOOK_FX_SYSTEM_BP_CONSTRUCTOR,
    HOOK_FX_SYSTEM_BP_UPDATE,
    HOOK_FX_SYSTEM_CONSTRUCTOR,
    HOOK_FX_SYSTEM_STOP,
    HOOK_FX_SYSTEM_PLAY_AND_KILL,
    HOOK_FX_SYSTEM_KILL,
    HOOK_FX_SYSTEM_ENABLE_PRIM,
    HOOK_FX_SYSTEM_SET_MATRIX,
    HOOK_FX_SYSTEM_SET_OFFSET_POS,
    HOOK_FX_SYSTEM_SET_CONST_TIME,
    HOOK_FX_SYSTEM_SET_RATE_MULT,
    HOOK_FX_SYSTEM_SET_VEL_ADD,
    HOOK_FX_SYSTEM_COPY_PARENT_MATRIX,
    HOOK_FX_SYSTEM_GET_COMPOSITE_MATRIX,
    HOOK_FX_SYSTEM_SET_LOCAL_PARTICLES,
    HOOK_FX_SYSTEM_IS_VISIBLE,
    HOOK_FX_INIT,
    HOOK_FX_DESTROY_ENTITY_FX,
    HOOK_FX_RENDER_BEGIN,
    HOOK_FX_TRIGGER_WATER_SPLASH,
    HOOK_FX_EMITTER_BP_CREATE_INSTANCE,
    HOOK_FX_EMITTER_BP_UPDATE,
    HOOK_FX_INFO_MANAGER_PROCESS_MOVEMENT_INFO,
    HOOK_FX_MANAGER_CONSTRUCTOR,
    HOOK_FX_MANAGER_FIND_FX_SYSTEM_BP,
    HOOK_FX_MANAGER_UPDATE_PORT,
    HOOK_FX_MEMORY_POOL_INIT,
    HOOK_FX_SYSTEM_BP_RENDER,
    HOOK_FX_SYSTEM_BP_SET_BOUNDING_SPHERE,
    HOOK_FX_SYSTEM_DESTRUCTOR,
    HOOK_FX_SYSTEM_PLAY,
    HOOK_FX_SYSTEM_EXIT,
    HOOK_FX_SYSTEM_GET_BOUNDING_SPHERE_WLD,
];

const PARTS: &[&[HookReg]] = &[OWN_HOOKS, info::HOOKS];

const fn total(parts: &[&[HookReg]]) -> usize {
    let (mut n, mut i) = (0, 0);
    while i < parts.len() {
        n += parts[i].len();
        i += 1;
    }
    n
}

const fn concat<const N: usize>(parts: &[&[HookReg]]) -> [HookReg; N] {
    let mut out = [sa_core::hooks::ALL[0]; N];
    let (mut k, mut i) = (0, 0);
    while i < parts.len() {
        let mut j = 0;
        while j < parts[i].len() {
            out[k] = parts[i][j];
            k += 1;
            j += 1;
        }
        i += 1;
    }
    assert!(k == N);
    out
}

const ALL_ARRAY: [HookReg; total(PARTS)] = concat(PARTS);

/// This module's registrations (this file's, then the submodules').
pub const HOOKS: &[HookReg] = &ALL_ARRAY;
