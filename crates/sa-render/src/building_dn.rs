//! The day / night vertex-colour blend of the building DN pipeline (STD-6, batch `std6-render-1`).
//! Ported from the 1.0 US bytes (0x5D6830, 0x5D6850, 0x5D7200, 0x5D72A0, 0x5D72C0, 0x5D7F80, 0x5D8050).
//!
//! DN geometry extension (`geometry + [0x8D12BC]`): +0 night colours, +4 day colours (RGBA per vertex),
//! +8 the balance the prelit colours (+0x30) were last blended with.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GameData};
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

orig! {
    /// `RpGeometryLock(geometry, flags)` (0x74C7D0), `RpGeometryUnlock(geometry)` (0x74C800)
    fn geometry_lock(g: GPtr<u8>, flags: u32) -> GPtr<u8> = cdecl 0x74C7D0 => unported;
    fn geometry_unlock(g: GPtr<u8>) -> GPtr<u8> = cdecl 0x74C800 => unported;
    /// `RpClumpForAllAtomics(clump, cb, data)` (0x749B70)
    fn clump_for_all_atomics(c: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `CCustomBuildingDNPipeline::NVCPipelineProcess(atomic, balance)` (0x5D6850)
    fn nvc_pipeline_process_call(a: GPtr<u8>, balance: f32) = cdecl 0x5D6850 => nvc_pipeline_process;
    /// `CCustomBuildingDNPipeline::PreRenderUpdate(atomic, force)` (0x5D7200)
    fn pre_render_update_call(a: GPtr<u8>, force: u32) = cdecl 0x5D7200 => pre_render_update_atomic;
    /// `CCustomBuildingRenderer::UpdateDayNightBalanceParam()` (0x5D7F80), the frame-slot advance (0x5D6830)
    fn update_dn_balance_call() = cdecl 0x5D7F80 => update_day_night_balance_param;
    fn advance_slot_call(force_all: u32) = cdecl 0x5D6830 => advance_update_slot;
}

const EXT_OFFSET: usize = 0x8D12BC;
/// `CCustomBuildingDNPipeline::m_fDNBalanceParam`, the DN pipeline, the update slot and "update all".
const DN_BALANCE: usize = 0x8D12C0;
const DN_PIPE: usize = 0xC02C1C;
const UPDATE_SLOT: usize = 0xC02C14;
const UPDATE_ALL: usize = 0xC02C18;

fn g<T: GameData>(a: usize) -> GPtr<T> {
    GPtr::from_addr(a)
}

fn rd<T: GameData>(p: GPtr<u8>, o: isize) -> T {
    p.byte_add(o).cast::<T>().read()
}

/// The frame-slot advance (0x5D6830): slot = (slot + 1) & 15, "update all" = the argument.
#[hook(addr = 0x5D6830, conv = cdecl, name = "FUN_005d6830", expect = "A1 14 2C C0 00 8B 4C 24 04",
       route = "L1-pure", hazards = "none")]
pub fn advance_update_slot(force_all: u32) {
    g::<u32>(UPDATE_SLOT).write(g::<u32>(UPDATE_SLOT).read().wrapping_add(1) & 0xF);
    g::<u32>(UPDATE_ALL).write(force_all);
}

/// `CCustomBuildingDNPipeline::NVCPipelineProcess(atomic, balance)` (0x5D6850): the balance is stored in
/// the extension, clamped to [0, 1] (NaN kept) as t, u = 1 - t; with the geometry locked (flags 8) every
/// prelit colour byte = `_ftol(night * t + day * u)` (x87, `fild` of the bytes); then unlocked.
#[hook(addr = 0x5D6850, conv = cdecl, name = "CCustomBuildingDNPipeline::NVCPipelineProcess", expect = "83 EC 28 8B 44 24 2C",
       route = "L2", hazards = "none", recs = "new-game")]
pub fn nvc_pipeline_process(atomic: GPtr<u8>, balance: f32) {
    let geom = GPtr::<u8>::from_addr(rd::<u32>(atomic, 0x18) as usize);
    let ext = geom.byte_add(g::<i32>(EXT_OFFSET).read() as isize);
    ext.byte_add(8).cast::<f32>().write(balance);
    let night = GPtr::<u8>::from_addr(rd::<u32>(ext, 0) as usize);
    let n = rd::<i32>(geom, 0x14);
    let out = GPtr::<u8>::from_addr(rd::<u32>(geom, 0x30) as usize);
    let day = GPtr::<u8>::from_addr(rd::<u32>(ext, 4) as usize);
    geometry_lock(geom, 8);
    let t = balance.clamp(0.0, 1.0);
    let u = X87::from_f32(1.0).fsub_m(t).to_f32();
    for k in 0..(n.max(0) as isize * 4) {
        let a = X87::from_i32(night.byte_add(k).read() as i32).fmul_m(t);
        let b = X87::from_i32(day.byte_add(k).read() as i32).fmul_m(u);
        out.byte_add(k).write(a.fadd(b).ftol() as u8);
    }
    geometry_unlock(geom);
}

/// `CCustomBuildingDNPipeline::PreRenderUpdate(atomic, force)` (0x5D7200; the DN pipeline pointer comes
/// from the protection chunk 0x407EC6): for an atomic on the DN pipeline whose stored balance differs
/// from the current one by more than 0.01 (or with `force`): re-blended when its address slot
/// (`atomic / 112 & 15`) is this frame's, "update all" is set, `force` is set, or the difference
/// exceeds 0.3.
#[hook(addr = 0x5D7200, conv = cdecl, name = "CCustomBuildingDNPipeline::PreRenderUpdate-Atomic", expect = "8B 4C 24 04 85 C9",
       route = "L2", hazards = "none", recs = "new-game")]
pub fn pre_render_update_atomic(atomic: GPtr<u8>, force: u32) {
    if atomic.is_null() || rd::<u32>(atomic, 0x6C) != g::<u32>(DN_PIPE).read() {
        return;
    }
    let geom = GPtr::<u8>::from_addr(rd::<u32>(atomic, 0x18) as usize);
    let stored = rd::<f32>(geom, g::<i32>(EXT_OFFSET).read() as isize + 8);
    let diff = X87::from_f32(stored).fsub_m(g::<f32>(DN_BALANCE).read()).fabs();
    let d = diff.to_f32();
    let big = matches!(diff.fcom(X87::from_f32(0.01)), Cmp::Greater);
    if !big && force == 0 {
        return;
    }
    let slot = ((atomic.addr() as u32) / 112) & 0xF;
    let go = slot == g::<u32>(UPDATE_SLOT).read()
        || g::<u32>(UPDATE_ALL).read() != 0
        || force != 0
        // `fcomp 0.3; test ah, 0x41; jne skip`: only "greater" (C0 = C3 = 0) updates
        || matches!(X87::from_f32(d).fcom(X87::from_f32(0.3)), Cmp::Greater);
    if go {
        nvc_pipeline_process_call(atomic, g::<f32>(DN_BALANCE).read());
    }
}

/// The clump variant's atomic callback (0x5D72A0): `PreRenderUpdate(atomic, *data)`. Returns the atomic.
#[hook(addr = 0x5D72A0, conv = cdecl, name = "(missed by ghidra: data-code-immediate)", expect = "8B 44 24 08 8B 08 56",
       unverified = "std6-render-1", route = "L2", hazards = "none")]
pub fn pre_render_update_atomic_cb(atomic: GPtr<u8>, data: GPtr<u32>) -> GPtr<u8> {
    pre_render_update_call(atomic, data.read());
    atomic
}

/// `CCustomBuildingDNPipeline::PreRenderUpdate(clump, force)` (0x5D72C0): every atomic.
#[hook(addr = 0x5D72C0, conv = cdecl, name = "CCustomBuildingDNPipeline::PreRenderUpdate-Clump", expect = "8B 4C 24 04 8D 44 24 08",
       unverified = "std6-render-1", route = "L2", hazards = "none")]
pub fn pre_render_update_clump(clump: GPtr<u8>, mut force: u32) {
    clump_for_all_atomics(clump, 0x5D72A0, GPtr::from_raw(&mut force as *mut u32).addr() as u32);
}

/// `CCustomBuildingRenderer::UpdateDayNightBalanceParam()` (0x5D7F80): from the game time in minutes
/// (`hours * 60 + minutes + seconds / 60`): night (1.0) before 6:00, fading to day (0.0) until 7:00,
/// day until 20:00, fading to night until 21:00, night after.
#[hook(addr = 0x5D7F80, conv = cdecl, name = "CCustomBuildingRenderer::UpdateDayNightBalanceParam", expect = "51 0F B6 05 53 01 B7 00",
       route = "L1-pure", hazards = "none", recs = "new-game")]
pub fn update_day_night_balance_param() {
    let minutes = g::<u8>(0xB70153).read() as i32 * 60 + g::<u8>(0xB70152).read() as i32;
    let secs = g::<u16>(0xB70150).read() as i32;
    let sixtieth = g::<f32>(0x859044).read();
    let t = X87::from_i32(minutes).fadd(X87::from_i32(secs).fmul_m(sixtieth));
    let tf = t.to_f32();
    let out = g::<f32>(DN_BALANCE);
    if matches!(t.fcom(X87::from_f32(360.0)), Cmp::Less) {
        out.write(1.0);
        return;
    }
    let tx = X87::from_f32(tf);
    if matches!(tx.fcom(X87::from_f32(420.0)), Cmp::Less) {
        out.write(X87::from_f32(420.0).fsub_m(tf).fmul_m(sixtieth).to_f32());
        return;
    }
    if matches!(tx.fcom(X87::from_f32(1200.0)), Cmp::Less) {
        out.write(0.0);
        return;
    }
    if matches!(tx.fcom(X87::from_f32(1260.0)), Cmp::Less) {
        out.write(X87::from_f32(1.0).fsub(X87::from_f32(1260.0).fsub_m(tf).fmul_m(sixtieth)).to_f32());
        return;
    }
    out.write(1.0);
}

/// `CCustomBuildingRenderer::Update()` (0x5D8050): the balance updated, then the frame slot advanced
/// ("update all" off).
#[hook(addr = 0x5D8050, conv = cdecl, name = "CCustomBuildingRenderer::Update", expect = "E8 2B FF FF FF 6A 00",
       route = "L2", hazards = "none", recs = "new-game")]
pub fn custom_building_renderer_update() {
    update_dn_balance_call();
    advance_slot_call(0);
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_ADVANCE_UPDATE_SLOT,
    HOOK_NVC_PIPELINE_PROCESS,
    HOOK_PRE_RENDER_UPDATE_ATOMIC,
    HOOK_PRE_RENDER_UPDATE_ATOMIC_CB,
    HOOK_PRE_RENDER_UPDATE_CLUMP,
    HOOK_UPDATE_DAY_NIGHT_BALANCE_PARAM,
    HOOK_CUSTOM_BUILDING_RENDERER_UPDATE,
];
