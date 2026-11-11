//! MVP-A writer batch `mvpa-fx-2`: `FxManager_c::Init` and `FxManager_c::ShouldCreate`. Ported from the
//! 1.0 US bytes; gta-reversed as a reading aid. Every hook is unverified.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr};

orig! {
    /// `FxMemoryPool_c::Init()` (0x4A9C30)
    pub fn fx_memory_pool_init(this: GPtr<u8>) -> u8 = thiscall 0x4A9C30 => unported;
    /// `RwMatrixCreate()` (0x7F2A50)
    pub fn rw_matrix_create() -> GPtr<u8> = cdecl 0x7F2A50 => unported;
    /// `RwMatrixMultiply(RwMatrix* out, const RwMatrix* a, const RwMatrix* b)` (0x7F18B0)
    pub fn rw_matrix_multiply(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F18B0 => unported;
    /// `RwV3dTransformPoints(RwV3d* out, const RwV3d* in, RwInt32 n, const RwMatrix*)` (0x7EDD90)
    pub fn rw_v3d_transform_points(out: GPtr<u8>, inp: GPtr<u8>, n: i32, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x7EDD90 => unported;
    /// `FxEmitterPrt_c::operator new[](size_t)` (0x4A18B0)
    pub fn fx_emitter_prt_new_array(size: u32) -> GPtr<u8> = cdecl 0x4A18B0 => unported;
    /// `` `eh vector constructor iterator' `` (0x821D30, stdcall: ptr, size, count, ctor, dtor)
    pub fn eh_vector_constructor_iterator(p: GPtr<u8>, size: u32, count: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// `List_c::AddItem(ListItem_c*)` (0x4A8DF0)
    pub fn list_add_item(l: GPtr<u8>, item: GPtr<u8>) = thiscall 0x4A8DF0 => unported;
    /// `FxSphere_c::FxSphere_c()` (0x4A9F70)
    pub fn fx_sphere_ctor(this: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A9F70 => unported;
    /// `FxFrustumInfo_c::IsCollision(FxSphere_c*)` (0x4AA030)
    pub fn fx_frustum_is_collision(this: GPtr<u8>, s: GPtr<u8>) -> u32 = thiscall 0x4AA030 => crate::mvpa_render3::fx_frustum_is_collision;
}

/// `FxManager_c::Init()` (0x4A98E0, thiscall): the FX memory pool (+0xAC) `Init()`; the matrix stack
/// index (+0x88) = 0 and its 8 matrices (+0x8C) = `RwMatrixCreate()`; 1000 `FxEmitterPrt_c` (0x3C
/// bytes) from `operator new[](0xEA64)` (count dword 1000 at the block start, constructed by the CRT's
/// `eh vector constructor iterator` with ctor 0x4A94E0 / dtor 0x4A94F0; NULL on failure) at +0x18,
/// each of them added to the free particle list (+0x1C); AL = 1. The SEH frame of the original only
/// covers the construction (nothing throws).
#[hook(addr = 0x4A98E0, conv = thiscall, name = "FxManager_c::Init", expect = "6A FF 68 CB 98 83 00",
       unverified = "mvpa-fx-2", route = "split", hazards = "heap-writes", recs = "grove-walk",
       note = "one-time initialiser: allocates the particle array and matrices")]
pub fn fx_manager_init(this: GPtr<u8>) -> u8 {
    fx_memory_pool_init(this.byte_add(0xAC));
    wr::<u32>(this, 0x88, 0);
    for i in 0..8isize {
        let m = rw_matrix_create();
        wr::<GPtr<u8>>(this, 0x8C + i * 4, m);
    }
    let p = fx_emitter_prt_new_array(0xEA64);
    let arr = if p.is_null() {
        GPtr::NULL
    } else {
        wr::<u32>(p, 0, 1000);
        let arr = p.byte_add(4);
        eh_vector_constructor_iterator(arr, 0x3C, 1000, 0x4A94E0, 0x4A94F0);
        arr
    };
    wr::<GPtr<u8>>(this, 0x18, arr);
    let mut off = 0isize;
    while off < 0xEA60 {
        list_add_item(this.byte_add(0x1C), rd::<GPtr<u8>>(this, 0x18).byte_add(off));
        off += 0x3C;
    }
    1
}

/// `FxManager_c::ShouldCreate(FxSystemBP_c* bp, RwMatrix* local, RwMatrix* parent, bool
/// ignoreBoundingChecks)` (0x4A9500, thiscall, `ret 0x10`): AL = 1 when ignoring or when the blueprint
/// has no bounding sphere (+0x20). Otherwise a matrix is taken from g_fxMan's matrix stack (index
/// 0xA9AF08 post-incremented, matrices at 0xA9AF0C): `*local` copied (no parent) or `local * parent`;
/// the sphere centre is transformed into a stack `FxSphere_c` (its constructor run first), the radius
/// copied, the index decremented, and the result is `this->frustum (+0x34).IsCollision(&sphere) != 0`.
/// Callers test AL.
#[hook(addr = 0x4A9500, conv = thiscall, name = "FxManager_c::ShouldCreate", expect = "83 EC 18 8A 44 24 28",
       unverified = "mvpa-fx-2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn fx_manager_should_create(this: GPtr<u8>, bp: GPtr<u8>, local: GPtr<u8>, parent: GPtr<u8>, ignore: u32) -> u8 {
    if ignore as u8 != 0 {
        return 1;
    }
    if rd::<GPtr<u8>>(bp, 0x20).is_null() {
        return 1;
    }
    let idx = g::<i32>(0xA9AF08).read();
    let m = GPtr::<GPtr<u8>>::from_addr(0xA9AF0C).wrapping_index(idx).read();
    g::<i32>(0xA9AF08).write(idx.wrapping_add(1));
    if parent.is_null() {
        for o in (0..0x40).step_by(4) {
            wr::<u32>(m, o, rd::<u32>(local, o));
        }
    } else {
        rw_matrix_multiply(m, local, parent);
    }
    let mut sphere = [0u32; 6];
    let s = GPtr::<u32>::from_raw(sphere.as_mut_ptr()).cast::<u8>();
    fx_sphere_ctor(s);
    rw_v3d_transform_points(s, rd::<GPtr<u8>>(bp, 0x20), 1, m);
    wr::<u32>(s, 0xC, rd::<u32>(rd::<GPtr<u8>>(bp, 0x20), 0xC));
    let idx = g::<i32>(0xA9AF08).read();
    g::<i32>(0xA9AF08).write(idx.wrapping_sub(1));
    u8::from(fx_frustum_is_collision(this.byte_add(0x34), s) != 0)
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_FX_MANAGER_INIT, HOOK_FX_MANAGER_SHOULD_CREATE];
