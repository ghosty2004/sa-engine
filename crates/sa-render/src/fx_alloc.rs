//! STD-5: small effects-system pieces of the loading path: `FxEmitterBP_c::operator new` (the
//! effects manager's memory pool) and the `FxSphere_c` constructor. Ported from the 1.0 US bytes.

use sa_core::abi::{HookReg, NativeReg};
use sa_core::mem::GPtr;
use sa_core::{hook, native, orig};

orig! {
    /// `FxMemoryPool_c::GetMem(size, align)` (0x4A9CA0)
    fn pool_get_mem(this: GPtr<u8>, size: u32, align: u32) -> GPtr<u8> = thiscall 0x4A9CA0 => crate::fx::fx_memory_pool_get_mem;
}

/// `g_fxMan`'s memory pool (0xA9AF2C).
const FX_MEMORY_POOL: usize = 0xA9AF2C;

/// `FxEmitterBP_c::operator new(size)` (0x4A1900): `g_fxMan.m_pool.GetMem(size, 4)`.
#[hook(addr = 0x4A1900, conv = cdecl, name = "FUN_004a1900", expect = "8B 44 24 04 6A 04 50 B9 2C AF",
       unverified = "std5-fx-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn emitter_bp_new(size: u32) -> GPtr<u8> {
    pool_get_mem(GPtr::from_addr(FX_MEMORY_POOL), size, 4)
}

/// `FxSphere_c::FxSphere_c()` (0x4A9F70, protected: `mov eax, ecx; nop; nop; jmp 0x4096E1`, whose chunk
/// writes +0x10 and returns): +0x10 = 0.
#[native(addr = 0x4A9F70, conv = thiscall, name = "FxSphere_c::FxSphere_c")]
pub fn fx_sphere_ctor(this: GPtr<u8>) -> GPtr<u8> {
    this.byte_add(0x10).cast::<u32>().write(0);
    this
}

/// `FxInfoSelfLit_c::Load(FILE*, int32 version)` (0x4A67B0): nothing (`ret 8`).
#[native(addr = 0x4A67B0, conv = thiscall, name = "FxInfoSelfLit_c::Load")]
pub fn info_self_lit_load(_this: GPtr<u8>, _file: GPtr<u8>, _version: i32) {}

/// `FxManager_c::GetFrustumInfo()` (0x4A9130): `this + 0x34`.
#[native(addr = 0x4A9130, conv = thiscall, name = "FxManager_c::GetFrustumInfo")]
pub fn manager_get_frustum_info(this: GPtr<u8>) -> GPtr<u8> {
    this.byte_add(0x34)
}

/// `FxMemoryPool_c::FxMemoryPool_c()` (0x4A9C10): nothing.
#[native(addr = 0x4A9C10, conv = thiscall, name = "FxMemoryPool_c::FxMemoryPool_c")]
pub fn memory_pool_ctor(this: GPtr<u8>) -> GPtr<u8> {
    this
}

/// `FxPrim_c::FxPrim_c()` (0x4A9F20): vtable 0x85A9E0, +4 = +8 = 0, byte +0xC = 1.
#[hook(addr = 0x4A9F20, conv = thiscall, name = "FxPrim_c::FxPrim_c", expect = "8B C1 33 C9 C7 00 E0 A9 85 00",
       unverified = "std5-fx-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn prim_ctor(this: GPtr<u8>) -> GPtr<u8> {
    this.cast::<u32>().write(0x85A9E0);
    this.byte_add(4).cast::<u32>().write(0);
    this.byte_add(8).cast::<u32>().write(0);
    this.byte_add(0xC).write(1);
    this
}

/// `FxPrim_c::~FxPrim_c()` (0x4A9F40): vtable 0x85A9E0, +4 = +8 = 0; EAX = 0.
#[hook(addr = 0x4A9F40, conv = thiscall, name = "FxPrim_c::~FxPrim_c", expect = "33 C0 C7 01 E0 A9 85 00 89 41",
       unverified = "std5-fx-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn prim_dtor(this: GPtr<u8>) -> u32 {
    this.cast::<u32>().write(0x85A9E0);
    this.byte_add(4).cast::<u32>().write(0);
    this.byte_add(8).cast::<u32>().write(0);
    0
}

orig! {
    /// `FxManager_c::DestroyAllFxSystems()` (0x4A98B0)
    fn destroy_all_fx_systems(this: GPtr<u8>) = thiscall 0x4A98B0 => unported;
    /// `Fx_c::InitStaticSystems()` (0x49E660)
    fn init_static_systems(this: GPtr<u8>) = thiscall 0x49E660 => unported;
}

/// `Fx_c::Reset()` (0x49EAE0): `g_fxMan.DestroyAllFxSystems()`, then (tail jump)
/// `InitStaticSystems()` on `this`.
#[hook(addr = 0x49EAE0, conv = thiscall, name = "Fx_c::Reset", expect = "56 8B F1 B9 80 AE A9 00 E8",
       unverified = "std5-fx-1", route = "L2", hazards = "heap-writes", recs = "new-game")]
pub fn fx_reset(this: GPtr<u8>) {
    destroy_all_fx_systems(GPtr::from_addr(0xA9AE80));
    init_static_systems(this);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_EMITTER_BP_NEW, HOOK_PRIM_CTOR, HOOK_PRIM_DTOR, HOOK_FX_RESET];
/// Every `#[native]` of this module.
pub const NATIVES: &[NativeReg] = &[
    NATIVE_FX_SPHERE_CTOR,
    NATIVE_INFO_SELF_LIT_LOAD,
    NATIVE_MANAGER_GET_FRUSTUM_INFO,
    NATIVE_MEMORY_POOL_CTOR,
];
