//! PORT-VEH-4 (batch `veh4-3`): `CAutomobile::DoHeliDustEffect`. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::Cmp;

use crate::common::{f32_of, g, jne_mask, jp5, rd, wr, x};

orig! {
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool buildings,
    /// bool vehicles, bool peds, bool objects, bool dummies, bool seeThrough, CStoredCollPoly*)` (0x5674E0, AL)
    fn process_vertical_line(p: GPtr<u8>, z: f32, cp: GPtr<u8>, ent: GPtr<u8>, b0: u32, b1: u32, b2: u32, b3: u32, b4: u32, b5: u32, poly: u32) -> u8 = cdecl 0x5674E0 => unported;
    /// `SurfaceInfos_c::IsSand(uint32)` (0x55E6F0, AL) on `g_surfaceInfos` (0xB79538)
    fn surface_is_sand(s: GPtr<u8>, surf: u32) -> u8 = thiscall 0x55E6F0 => unported;
    /// `CWaterLevel::GetWaterLevel(float, float, float, float*, bool, CVector*)` (0x6EB690, AL)
    fn water_get_level(x: f32, y: f32, z: f32, level: GPtr<u8>, touching: u32, normal: u32) -> u8 = cdecl 0x6EB690 => unported;
    /// `CEventDanger::CEventDanger(CEntity*, float)` (0x4B2600)
    fn event_danger_ctor(ev: GPtr<u8>, e: GPtr<u8>, radius: f32) -> GPtr<u8> = thiscall 0x4B2600 => unported;
    /// `CEventDanger::~CEventDanger()` (0x4B26A0)
    fn event_danger_dtor(ev: GPtr<u8>) = thiscall 0x4B26A0 => unported;
    /// `GetEventGlobalGroup()` (0x4ABA50)
    fn get_event_global_group() -> GPtr<u8> = cdecl 0x4ABA50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(grp: GPtr<u8>, ev: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `FxManager_c::CreateFxSystem(const char*, RwV3d*, RwMatrix*, bool)` (0x4A9BE0) on `g_fxMan` (0xA9AE80)
    fn create_fx_system(mgr: GPtr<u8>, name: u32, pos: GPtr<u8>, m: u32, ignore_bbox: u32) -> GPtr<u8> = thiscall 0x4A9BE0 => unported;
    /// `FxSystem_c::SetLocalParticles(bool)` (0x4AA910)
    fn fx_set_local_particles(fx: GPtr<u8>, b: u32) = thiscall 0x4AA910 => unported;
    /// `FxSystem_c::Play()` (0x4AA2F0)
    fn fx_play(fx: GPtr<u8>) = thiscall 0x4AA2F0 => unported;
    /// `FxSystem_c::SetConstTime(bool, float)` (0x4AA6C0)
    fn fx_set_const_time(fx: GPtr<u8>, on: u32, t: f32) = thiscall 0x4AA6C0 => unported;
    /// `FxSystem_c::SetOffsetPos(const RwV3d&)` (0x4AA660)
    fn fx_set_offset_pos(fx: GPtr<u8>, v: GPtr<u8>) = thiscall 0x4AA660 => unported;
    /// `FxSystem_c::EnablePrim(int32, bool)` (0x4AA610)
    fn fx_enable_prim(fx: GPtr<u8>, prim: i32, on: u32) = thiscall 0x4AA610 => unported;
    /// `FxSystem_c::Kill()` (0x4AA3F0)
    fn fx_kill(fx: GPtr<u8>) = thiscall 0x4AA3F0 => unported;
}

/// A stack buffer of `N` dwords as a game pointer.
fn buf<const N: usize>(b: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(b as *mut [u32; N]).cast()
}

/// The position of an entity: its matrix's (+0x30) or its placement's (+4).
fn pos_ptr(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `CAutomobile::DoHeliDustEffect(float strength, float height)` (0x6B0690).
///
/// For a heli (type 3) with the rotor faster than 0.1125, or a plane (type 4): the ground below (a
/// vertical line down to -1000, its z and surface taken even without a hit) or a higher water surface
/// gives the dust height; within 50 units of the camera and below `30 * height` (3 * height for the RC
/// Goblin/Raider) above it, every 20th frame a danger event (radius 20, 30 for the Hydra) is raised
/// below 20/30 units; the `heli_dust` fx (+0x580) is created (local particles, playing) and its
/// constant time (+0x984) moves by at most 0.04 toward `(strength * (1 - h / max))^2`, offset to the
/// ground point, with the water (prim 2), sand (prim 1) or dust (prim 0) primitive enabled. Otherwise
/// the fx is killed.
#[hook(addr = 0x6B0690, conv = thiscall, name = "CAutomobile::DoHeliDustEffect", this = "CAutomobile",
       expect = "64 A1 00 00 00 00 6A FF",
       unverified = "veh4-3", route = "split", hazards = "uninit-locals,heap-writes",
       note = "reads the col point's z and surface even when ProcessVerticalLine missed (uninitialised stack in the original, zero here); creates/kills the dust FxSystem and raises a danger event; the SEH frame is not reproduced")]
pub fn automobile_do_heli_dust_effect(this: GPtr<u8>, strength: f32, height: f32) {
    let kill = |this: GPtr<u8>| {
        let fx = rd::<GPtr<u8>>(this, 0x580);
        if !fx.is_null() {
            fx_kill(fx);
            wr::<u32>(this, 0x580, 0);
            wr::<u32>(this, 0x984, 0);
        }
    };
    let sub = rd::<u32>(this, 0x594);
    let go = (sub == 3 && !jne_mask(x(rd::<f32>(this, 0x84C)), x(f32::from_bits(0x3DE6_6667)), 0x41)) || sub == 4;
    if !go {
        kill(this);
        return;
    }
    let mut cp = [0u32; 11];
    let mut ent = [0u32; 1];
    process_vertical_line(pos_ptr(this), -1000.0, buf(&mut cp), buf(&mut ent), 1, 0, 0, 0, 0, 0, 0);
    let surface = (cp[8] >> 24) & 0xFF;
    let mut ground = f32::from_bits(cp[2]);
    let sand = surface_is_sand(GPtr::from_addr(0xB7_9538), surface) != 0;
    let mut water = false;
    let p = pos_ptr(this);
    let mut level = [0u32; 1];
    if water_get_level(rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8), buf(&mut level), 0, 0) != 0
        && !jne_mask(x(f32::from_bits(level[0])), x(ground), 0x41)
    {
        ground = f32::from_bits(level[0]);
        water = true;
    }
    let model = rd::<i16>(this, 0x22);
    let base = if model == 0x1F5 || model == 0x1D1 { 3.0f32 } else { 30.0 };
    let max_h = f32_of(x(base).fmul_m(height));
    let cm = g::<GPtr<u8>>(0xB6_F03C).read();
    let cam = if cm.is_null() { GPtr::<u8>::from_addr(0xB6_F02C) } else { cm.byte_add(0x30) };
    let vp = pos_ptr(this);
    let dz = x(rd::<f32>(vp, 8)).fsub_m(rd::<f32>(cam, 8));
    let dy = x(rd::<f32>(vp, 4)).fsub_m(rd::<f32>(cam, 4));
    let dx = x(rd::<f32>(vp, 0)).fsub_m(rd::<f32>(cam, 0));
    let d2 = dx.fmul(dx).fadd(dy.fmul(dy)).fadd(dz.fmul(dz));
    let alt = || x(rd::<f32>(pos_ptr(this), 8)).fsub_m(ground);
    if jp5(d2, x(2500.0)) || jp5(alt(), x(max_h)) {
        kill(this);
        return;
    }
    if g::<u32>(0xB7_CB4C).read().is_multiple_of(20) {
        let dist = if model == 0x208 { 30.0f32 } else { 20.0 };
        if alt().fcom(x(dist)) == Cmp::Less {
            let mut ev = [0u32; 0x10];
            let ep = buf(&mut ev);
            event_danger_ctor(ep, this, dist);
            event_group_add(get_event_global_group(), ep, 0);
            event_danger_dtor(ep);
        }
    }
    if rd::<u32>(this, 0x580) == 0 {
        let mut zero = [0u32; 3];
        let fx = create_fx_system(GPtr::from_addr(0xA9_AE80), 0x85_AA04, buf(&mut zero), 0, 1);
        wr::<GPtr<u8>>(this, 0x580, fx);
        if !fx.is_null() {
            fx_set_local_particles(fx, 1);
            fx_play(rd::<GPtr<u8>>(this, 0x580));
        }
    }
    let fx = rd::<GPtr<u8>>(this, 0x580);
    if fx.is_null() {
        return;
    }
    let t = x(max_h).fsub(alt()).fdiv_m(max_h).fmul_m(strength);
    let t2 = f32_of(t.fmul(t));
    let cur = rd::<f32>(this, 0x984);
    let step = f32::from_bits(0x3D23_D70A);
    if !jne_mask(x(t2), x(cur), 0x41) {
        let v = x(cur).fadd_m(step);
        wr::<f32>(this, 0x984, if v.fcom(x(t2)) == Cmp::Less { f32_of(v) } else { t2 });
    } else if x(t2).fcom(x(cur)) == Cmp::Less {
        let v = x(cur).fsub_m(step);
        wr::<f32>(this, 0x984, if !jne_mask(v, x(t2), 0x41) { f32_of(v) } else { t2 });
    }
    fx_set_const_time(rd::<GPtr<u8>>(this, 0x580), 1, rd::<f32>(this, 0x984));
    let vp = pos_ptr(this);
    let mut off = [rd::<u32>(vp, 0), rd::<u32>(vp, 4), ground.to_bits()];
    fx_set_offset_pos(rd::<GPtr<u8>>(this, 0x580), buf(&mut off));
    let prims: [(i32, u32); 3] = if water {
        [(0, 0), (1, 0), (2, 1)]
    } else if sand {
        [(0, 0), (1, 1), (2, 0)]
    } else {
        [(0, 1), (1, 0), (2, 0)]
    };
    for (prim, on) in prims {
        fx_enable_prim(rd::<GPtr<u8>>(this, 0x580), prim, on);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_AUTOMOBILE_DO_HELI_DUST_EFFECT];
