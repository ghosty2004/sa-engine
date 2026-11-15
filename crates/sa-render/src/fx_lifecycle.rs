//! FRONTIER-2 lane B (batch `frontier2-b1`): the `Fx_c` / `FxManager_c` shutdown and particle-recycling paths
//! (`Fx_c::~Fx_c`, `ExitStaticSystems`, `ExitEntitySystems`, `FxManager_c::~FxManager_c`, `Exit`,
//! `FreeUpParticle`, `FxMemoryPool_c::Exit`, `FxSystemBP_c::FreePrtFromSystem`, `FxEmitterBP_c::FreePrtFromPrim`,
//! `FxSystem_c::SetMustCreatePrts`) and the small `Fx_c` effect helpers on the frontier (`CreateMatFromVec`,
//! `AddTyreBurst`, `AddWood`, `TriggerBulletSplash`, `TriggerFootSplash`). Ported from the 1.0 US bytes
//! (gta-reversed `Fx/*.cpp` as a reading aid), `rand()` at its original call sites.

use sa_core::abi::HookReg;
use sa_core::crt::rand_at;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::fpu::{x, xi};

use crate::common::{g, rd, wr, Fpu};
use crate::bindings::{fx_add_particle, fx_create_system, fx_play_and_kill};
use crate::fx::{list_item_dtor, return_particle, FX_MAN};
use sa_core::bindings::{
    crt_free_8214bd as crt_free, destroy_fx_system, list_dtor, list_get_num_items, list_remove_all, list_remove_item,
    memory_free, rw_matrix_destroy, rw_matrix_update, rw_v3d_normalize,
};
use sa_stream::bindings::remove_txd_slot;
use sa_world::bindings::fx_prt_mult_ctor;

orig! {
    /// `FxMemoryPool_c::~FxMemoryPool_c()` (0x4A9C20, a lone `ret`)
    fn memory_pool_dtor(this: GPtr<u8>) = thiscall 0x4A9C20 => unported;
    /// `List_c::GetItemOffset(bool fromHead, int32 n)` (0x4A9010)
    fn list_get_item_offset(this: GPtr<u8>, from_head: u32, n: i32) -> GPtr<u8> = thiscall 0x4A9010 => unported;
    /// `FxSystemBP_c::FreePrtFromSystem(FxSystem_c*)` (0x4AA1B0)
    fn system_bp_free_prt_from_system(this: GPtr<u8>, sys: GPtr<u8>) -> u8 = thiscall 0x4AA1B0 => system_bp_free_prt_from_system_port;
    /// `FxPrimBP_c` virtual slot 0x18: `FreePrtFromSystem(FxSystem_c*)` (as `FxSystemBP_c::FreePrtFromSystem` calls it)
    fn prim_bp_vfree_prt(this: GPtr<u8>, sys: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `FxMemoryPool_c::Exit()` (0x4A9C80)
    fn memory_pool_exit(this: GPtr<u8>) = thiscall 0x4A9C80 => fx_memory_pool_exit;
}

/// `"water_splash"` (0x85A754) and `"water_splsh_sml"` (0x85A764): the two splash systems.
const BULLET_SPLASH_NAME: usize = 0x85A754;
const FOOT_SPLASH_NAME: usize = 0x85A764;
/// 625.0 (0x85A6E8): the squared camera distance limit of the `Fx_c` effects.
const K_625: usize = 0x85A6E8;
/// `g_fx` +0x24 (the tyre-burst system), +0 (the wood system).
const FX_TYRE_BURST: isize = 0x24;
const FX_WOOD: isize = 0;

fn gf(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// The `Fx_c` effects' camera test: `TheCamera`'s position (its matrix +0x30, or the placement +4 without one)
/// minus `pos`, `dx² + dz² + dy²` on the x87 stack, compared with 625.0: true (create) when below or equal or
/// unordered (`test ah, 41h; je` skips).
fn near_camera(pos: GPtr<u8>) -> bool {
    let c = sa_world::world::common::cam_pos().cast::<u8>();
    let dx = x(rd(c, 0)).fsub_m(rd(pos, 0));
    let dy = x(rd(c, 4)).fsub_m(rd(pos, 4));
    let dz = x(rd(c, 8)).fsub_m(rd(pos, 8));
    let s = dx.fmul(dx).fadd(dz.fmul(dz)).fadd(dy.fmul(dy));
    s.fcom(x(gf(K_625))).test_any(0x41)
}

/// `Fx_c::~Fx_c()` (0x49E630): the entity-system list (+0x44) destroyed (`List_c` destructor, a tail jump).
#[hook(addr = 0x49E630, conv = thiscall, name = "Fx_c::Destructor", expect = "83 C1 44 E9 A8 A7 00 00", this = "Fx_c",
       unverified = "frontier2-b1", route = "L2", hazards = "none")]
pub fn fx_destructor(this: GPtr<u8>) {
    list_dtor(this.byte_add(0x44));
}

/// `Fx_c::ExitStaticSystems()` (0x49E850): `g_fxMan.DestroyFxSystem` of each of the 17 static systems +0..+0x40.
#[hook(addr = 0x49E850, conv = thiscall, name = "Fx_c::ExitStaticSystems", expect = "56 8B F1 8B 06", this = "Fx_c",
       unverified = "frontier2-b1", route = "L2", hazards = "heap-writes")]
pub fn fx_exit_static_systems(this: GPtr<u8>) {
    for i in 0..17isize {
        destroy_fx_system(g(FX_MAN), rd(this, 4 * i));
    }
}

/// `Fx_c::ExitEntitySystems()` (0x4A12D0): every item of the entity-system list (+0x44, next +4) is unlinked, its
/// system (+8) destroyed by `g_fxMan`, the item destructed and freed (`_free`).
#[hook(addr = 0x4A12D0, conv = thiscall, name = "Fx_c::ExitEntitySystems", expect = "53 8D 59 44 56", this = "Fx_c",
       unverified = "frontier2-b1", route = "L2", hazards = "heap-writes")]
pub fn fx_exit_entity_systems(this: GPtr<u8>) {
    let list = this.byte_add(0x44);
    let mut n: GPtr<u8> = rd(list, 0);
    while !n.is_null() {
        let next: GPtr<u8> = rd(n, 4);
        list_remove_item(list, n);
        destroy_fx_system(g(FX_MAN), rd(n, 8));
        list_item_dtor(n);
        crt_free(n);
        n = next;
    }
}

/// `FxManager_c::~FxManager_c()` (0x4A90A0): the memory pool (+0xAC) and the lists +0x1C, +0xC and +0 destructed in
/// that order (the SEH frame is not reproduced: nothing throws).
#[hook(addr = 0x4A90A0, conv = thiscall, name = "FxManager_c::Destructor", expect = "6A FF 68 3E 98 83 00",
       this = "FxManager_c", unverified = "frontier2-b1", route = "L2", hazards = "none")]
pub fn fx_manager_destructor(this: GPtr<u8>) {
    memory_pool_dtor(this.byte_add(0xAC));
    list_dtor(this.byte_add(0x1C));
    list_dtor(this.byte_add(0xC));
    list_dtor(this);
}

/// `FxManager_c::FreeUpParticle()` (0x4A9400): picks random systems of the list +0xC (`rand() % count`, `idiv`,
/// `GetItemOffset(true, i)`) until one that may create particles (byte +0x62 bit 0x20 clear) gives a particle back
/// through its blueprint's `FreePrtFromSystem`.
#[hook(addr = 0x4A9400, conv = thiscall, name = "FxManager_c::FreeUpParticle", expect = "56 57 8D 79 0C",
       this = "FxManager_c", unverified = "frontier2-b1", route = "L2", hazards = "heap-writes",
       note = "rand site 0x4A9413")]
pub fn fx_manager_free_up_particle(this: GPtr<u8>) {
    let list = this.byte_add(0xC);
    loop {
        let n = list_get_num_items(list) as i32;
        let i = rand_at(0x4A_9413).wrapping_rem(n);
        let sys = list_get_item_offset(list, 1, i);
        if rd::<u8>(sys, 0x62) & 0x20 != 0 {
            continue;
        }
        if system_bp_free_prt_from_system(rd(sys, 8), sys) != 0 {
            return;
        }
    }
}

/// `FxManager_c::Exit()` (0x4A9A10): every system of the list +0xC destroyed (next +4 read first), the list +0
/// emptied, +0x18 = 0, the particle list +0x1C emptied, the FX txd slot (+0x28) removed, the eight pooled matrices
/// (+0x8C..) destroyed, then the memory pool's (+0xAC) `Exit` (a tail jump).
#[hook(addr = 0x4A9A10, conv = thiscall, name = "FxManager_c::Exit", expect = "53 56 8B F1 8B 46 0C",
       this = "FxManager_c", unverified = "frontier2-b1", route = "L2", hazards = "heap-writes")]
pub fn fx_manager_exit(this: GPtr<u8>) {
    let mut s: GPtr<u8> = rd(this, 0xC);
    while !s.is_null() {
        let next: GPtr<u8> = rd(s, 4);
        destroy_fx_system(this, s);
        s = next;
    }
    list_remove_all(this);
    wr::<u32>(this, 0x18, 0);
    list_remove_all(this.byte_add(0x1C));
    remove_txd_slot(rd(this, 0x28));
    for i in 0..8isize {
        rw_matrix_destroy(rd(this, 0x8C + 4 * i));
    }
    memory_pool_exit(this.byte_add(0xAC));
}

/// `FxMemoryPool_c::Exit()` (0x4A9C80): `CMemoryMgr::Free(base)` when the base (+0) is set.
#[hook(addr = 0x4A9C80, conv = thiscall, name = "FxMemoryPool_c::Exit", expect = "8B 01 85 C0 74 07",
       this = "FxMemoryPool_c", unverified = "frontier2-b1", route = "L2", hazards = "none")]
pub fn fx_memory_pool_exit(this: GPtr<u8>) {
    let base: GPtr<u8> = rd(this, 0);
    if !base.is_null() {
        memory_free(base);
    }
}

/// `FxSystemBP_c::FreePrtFromSystem(FxSystem_c*)` (0x4AA1B0): the first primitive blueprint (array +0x1C, count the
/// signed byte +0x1B) whose virtual `FreePrtFromSystem` (slot 0x18) frees one gives true (AL = 1); else false.
#[hook(addr = 0x4AA1B0, conv = thiscall, name = "FxSystemBP_c::FreePrtFromSystem", expect = "53 56 57 8B F9",
       this = "FxSystemBP_c", unverified = "frontier2-b1", route = "L2", hazards = "heap-writes")]
pub fn system_bp_free_prt_from_system_port(this: GPtr<u8>, sys: GPtr<u8>) -> u8 {
    let mut i = 0i32;
    while i < i32::from(rd::<i8>(this, 0x1B)) {
        let prims: GPtr<u8> = rd(this, 0x1C);
        if prim_bp_vfree_prt(rd(prims, 4 * i as isize), sys) != 0 {
            return 1;
        }
        i += 1;
    }
    0
}

/// `FxEmitterBP_c::FreePrtFromPrim(FxSystem_c*)` (0x4A2510, virtual slot 0x18): the first particle of the list
/// +0x20 (next +4) that belongs to `sys` (+0x28) is unlinked and returned to `g_fxMan`; AL = 1, else 0.
#[hook(addr = 0x4A2510, conv = thiscall, name = "FxEmitterBP_c::FreePrtFromPrim", expect = "83 C1 20 56 8B 31",
       this = "FxEmitterBP_c", unverified = "frontier2-b1", route = "L2", hazards = "heap-writes")]
pub fn emitter_bp_free_prt_from_prim(this: GPtr<u8>, sys: u32) -> u8 {
    let list = this.byte_add(0x20);
    let mut p: GPtr<u8> = rd(list, 0);
    while !p.is_null() {
        if rd::<u32>(p, 0x28) == sys {
            list_remove_item(list, p);
            return_particle(g(FX_MAN), p);
            return 1;
        }
        p = rd(p, 4);
    }
    0
}

/// `FxSystem_c::SetMustCreatePrts(bool)` (0x4AAC70): bit 0x20 of byte +0x62 = bit 0 of the argument.
#[hook(addr = 0x4AAC70, conv = thiscall, name = "FxSystem_c::SetMustCreatePrts", expect = "8A 44 24 04 8A 51 62",
       unverified = "frontier2-b1", route = "L2", hazards = "none")]
pub fn fx_system_set_must_create_prts(this: GPtr<u8>, on: u32) {
    let b: u8 = rd(this, 0x62);
    wr::<u8>(this, 0x62, b ^ (((on as u8) << 5 ^ b) & 0x20));
}

/// `Fx_c::CreateMatFromVec(RwMatrix* m, CVector* pos, CVector* dir)` (0x49E950, `this` unused): `m` = identity (the
/// flags +0xC read first, `|= 0x20003`), position `*pos`, up `*dir` normalised in place (`RwV3dNormalize`, the
/// length dropped), right (+0) = `up × (0, 0, -1)` and at (+0x20) = `right × up` with the original's x87 operand
/// order (right kept on the stack for the second product), then `RwMatrixUpdate`.
#[hook(addr = 0x49E950, conv = thiscall, name = "Fx_c::CreateMatFromVec", expect = "56 8B 74 24 08", this = "Fx_c",
       unverified = "frontier2-b1", route = "L2", hazards = "none")]
pub fn fx_create_mat_from_vec(_this: GPtr<u8>, m: GPtr<u8>, pos: GPtr<u8>, dir: GPtr<u8>) {
    let flags: u32 = rd(m, 0xC);
    for o in [0x14, 0x28, 0] {
        wr::<u32>(m, o, 0x3F80_0000);
    }
    for o in [0x38, 0x34, 0x30, 0x10, 0x18, 8, 4, 0x24, 0x20] {
        wr::<u32>(m, o, 0);
    }
    wr(m, 0xC, flags | 0x20003);
    for o in [0, 4, 8] {
        wr::<u32>(m, 0x30 + o, rd(pos, o));
    }
    for o in [0, 4, 8] {
        wr::<u32>(m, 0x10 + o, rd(dir, o));
    }
    let up = m.byte_add(0x10);
    rw_v3d_normalize(up, up);
    let k0 = gf(0x858B50);
    let km1 = gf(0x858C1C);
    let f = |o: isize| rd::<f32>(m, o);
    let mut s = Fpu::new();
    s.fld(f(0x18));
    s.fmul_m(k0);
    s.fld_st(0);
    s.fld(f(0x14));
    s.fmul_m(km1);
    s.fsubp(1);
    wr(m, 0, s.fst());
    s.fld(f(0x10));
    s.fmul_m(km1);
    s.fsub_st(2);
    wr(m, 4, s.fst());
    s.fld(f(0x14));
    s.fmul_m(k0);
    s.fld(f(0x10));
    s.fmul_m(k0);
    s.fsubp(1);
    wr(m, 8, s.fst());
    s.fld_st(1);
    s.fmul_m(f(0x18));
    s.fld_st(1);
    s.fmul_m(f(0x14));
    s.fsubp(1);
    wr(m, 0x20, s.fstp());
    s.fmul_m(f(0x10));
    s.fld_st(2);
    s.fmul_m(f(0x18));
    s.fsubp(1);
    wr(m, 0x24, s.fstp());
    s.fxch(1);
    s.fmul_m(f(0x14));
    s.fxch(1);
    s.fmul_m(f(0x10));
    s.fsubp(1);
    wr(m, 0x28, s.fstp());
    s.fstp_st(0);
    rw_matrix_update(m);
}

/// The splash triggers: within the camera test the named system is created at `pos` and played-and-killed.
fn trigger_named(pos: GPtr<u8>, name: usize) {
    if !near_camera(pos) {
        return;
    }
    let sys = fx_create_system(g(FX_MAN), g(name), pos, GPtr::NULL, 0);
    if !sys.is_null() {
        fx_play_and_kill(sys);
    }
}

/// `Fx_c::TriggerBulletSplash(CVector&)` (0x4A10E0): the bullet splash system (name 0x85A754).
#[hook(addr = 0x4A10E0, conv = thiscall, name = "Fx_c::TriggerBulletSplash", expect = "A1 3C F0 B6 00", this = "Fx_c",
       unverified = "frontier2-b1", route = "L2", hazards = "heap-writes")]
pub fn fx_trigger_bullet_splash(_this: GPtr<u8>, pos: GPtr<u8>) {
    trigger_named(pos, BULLET_SPLASH_NAME);
}

/// `Fx_c::TriggerFootSplash(CVector&)` (0x4A1150): the foot splash system (name 0x85A764).
#[hook(addr = 0x4A1150, conv = thiscall, name = "Fx_c::TriggerFootSplash", expect = "A1 3C F0 B6 00", this = "Fx_c",
       unverified = "frontier2-b1", route = "L2", hazards = "heap-writes")]
pub fn fx_trigger_foot_splash(_this: GPtr<u8>, pos: GPtr<u8>) {
    trigger_named(pos, FOOT_SPLASH_NAME);
}

/// `Fx_c::AddTyreBurst(CVector& pos, CVector& vel)` (0x49F300): within the camera test, `FxPrtMult_c(1, 1, 1, 0.4,
/// 0.12, 0, 0.1)` and four particles of the tyre-burst system (`g_fx` +0x24) at `pos` with `vel`, times `i * 0.05`
/// (`fild` of the counter), rotation -1, brightness 1.2, camera 0.6.
#[hook(addr = 0x49F300, conv = thiscall, name = "Fx_c::AddTyreBurst", expect = "A1 3C F0 B6 00", this = "Fx_c",
       unverified = "frontier2-b1", route = "L2", hazards = "heap-writes")]
pub fn fx_add_tyre_burst(this: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>) {
    if !near_camera(pos) {
        return;
    }
    let mut mult = [0u32; 7];
    let mp: GPtr<u8> = GPtr::from_raw(mult.as_mut_ptr()).cast();
    fx_prt_mult_ctor(mp, 1.0, 1.0, 1.0, f32::from_bits(0x3ECC_CCCD), f32::from_bits(0x3DF5_C28F), 0.0, f32::from_bits(0x3DCC_CCCD));
    for i in 0..4 {
        let t = xi(i).fmul_m(gf(0x858C28)).to_f32();
        fx_add_particle(rd(this, FX_TYRE_BURST), pos, vel, t.to_bits(), mp, 0xBF80_0000, f32::from_bits(0x3F99_999A), f32::from_bits(0x3F19_999A), 0);
    }
}

/// `rand() % 10000` (`cdq; idiv`) at its original return address, times 0.0001 (0x858FC4), on the x87 stack.
fn r_unit(site: u32) -> sa_math::x87::X87 {
    xi(rand_at(site).wrapping_rem(10000)).fmul_m(gf(0x858FC4))
}

/// `Fx_c::AddWood(CVector& pos, CVector& dir, int32 amount, float brightness)` (0x49EE10): within the camera test,
/// `FxPrtMult_c(0.5, 0.25, 0, 1, 0.3, 0, 1)`; per particle the colour (r `u*0.12+0.13`, g `u*0.03+0.12`, b
/// `u*0.03+0.04`) and size (`u*0.3+0.7`) are re-randomised, the velocity is `dir * 4` plus `u*4-2` per axis, and
/// the wood system (`g_fx` +0) gets it at `pos` (time 0, rotation -1, `brightness`, camera 0.6). The original keeps
/// each `rand() % 10000` in its own `pos` argument slot (it holds `pos` in EBX).
#[hook(addr = 0x49EE10, conv = thiscall, name = "Fx_c::AddWood", expect = "A1 3C F0 B6 00", this = "Fx_c",
       unverified = "frontier2-b1", route = "L2", hazards = "heap-writes,stack-residue",
       note = "rand sites 0x49EEA5..0x49EFD5; the rand values go to its own pos argument slot")]
pub fn fx_add_wood(this: GPtr<u8>, pos: GPtr<u8>, dir: GPtr<u8>, amount: i32, brightness: f32) {
    if !near_camera(pos) {
        return;
    }
    let mut mult = [0u32; 7];
    let mp: GPtr<u8> = GPtr::from_raw(mult.as_mut_ptr()).cast();
    fx_prt_mult_ctor(mp, 0.5, 0.25, 0.0, 1.0, f32::from_bits(0x3E99_999A), 0.0, 1.0);
    let mut n = amount;
    if n <= 0 {
        return;
    }
    let mf = |o: isize, v: f32| wr::<f32>(mp, o, v);
    let mut vel = [0f32; 3];
    loop {
        mf(0, r_unit(0x49_EEA5).fmul_m(gf(0x85A6F8)).fadd_m(gf(0x859020)).to_f32());
        mf(4, r_unit(0x49_EED0).fmul_m(gf(0x85A6F4)).fadd_m(gf(0x85A6F0)).to_f32());
        mf(8, r_unit(0x49_EEFB).fmul_m(gf(0x85A6EC)).fadd_m(gf(0x858CEC)).to_f32());
        mf(0x10, r_unit(0x49_EF26).fmul_m(gf(0x858C24)).fadd_m(gf(0x858CB0)).to_f32());
        let k4 = gf(0x858B90);
        for (i, v) in vel.iter_mut().enumerate() {
            *v = x(rd::<f32>(dir, 4 * i as isize)).fmul_m(k4).to_f32();
        }
        for (i, site) in [0x49_EF77u32, 0x49_EFA6, 0x49_EFD5].into_iter().enumerate() {
            vel[i] = r_unit(site).fmul_m(k4).fsub_m(gf(0x858CA0)).fadd_m(vel[i]).to_f32();
        }
        let vp: GPtr<u8> = GPtr::from_raw(vel.as_mut_ptr()).cast();
        fx_add_particle(rd(this, FX_WOOD), pos, vp, 0, mp, 0xBF80_0000, brightness, f32::from_bits(0x3F19_999A), 0);
        n -= 1;
        if n == 0 {
            break;
        }
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_FX_DESTRUCTOR,
    HOOK_FX_EXIT_STATIC_SYSTEMS,
    HOOK_FX_EXIT_ENTITY_SYSTEMS,
    HOOK_FX_MANAGER_DESTRUCTOR,
    HOOK_FX_MANAGER_FREE_UP_PARTICLE,
    HOOK_FX_MANAGER_EXIT,
    HOOK_FX_MEMORY_POOL_EXIT,
    HOOK_SYSTEM_BP_FREE_PRT_FROM_SYSTEM_PORT,
    HOOK_EMITTER_BP_FREE_PRT_FROM_PRIM,
    HOOK_FX_SYSTEM_SET_MUST_CREATE_PRTS,
    HOOK_FX_CREATE_MAT_FROM_VEC,
    HOOK_FX_TRIGGER_BULLET_SPLASH,
    HOOK_FX_TRIGGER_FOOT_SPLASH,
    HOOK_FX_ADD_TYRE_BURST,
    HOOK_FX_ADD_WOOD,
];
