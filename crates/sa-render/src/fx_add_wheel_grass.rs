//! `Fx_c::AddWheelGrass` (0x49FF20, batch `samp2-fx-1`): grass thrown up by the player's wheels.
//! Unreversed in gta-reversed; ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, k, rd, x, xi};

orig! {
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => sa_world::world::cworld::find_player_ped_port;
    /// `FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, b: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `FxPrtMult_c::FxPrtMult_c(r, g, b, a, size, spin, life)` (0x4AB290)
    #[allow(clippy::too_many_arguments)]
    fn fx_prt_mult_ctor(p: GPtr<u8>, r: f32, g: f32, b: f32, a: f32, size: f32, spin: f32, life: f32) -> GPtr<u8> = thiscall 0x4AB290 => unported;
    /// `FxSystem_c::AddParticle(RwV3d*, RwV3d*, float, FxPrtMult_c*, float, float, float, uint8)` (0x4AA440)
    #[allow(clippy::too_many_arguments)]
    fn fx_add_particle(sys: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, time: f32, mult: GPtr<u8>, rot: f32, bright: f32, near: f32, local: u32) = thiscall 0x4AA440 => unported;
}

/// `rand() * (1 / 32767)` (0x858C7C).
fn rnd(site: u32) -> sa_math::x87::X87 {
    xi(sa_core::orig::at_site(site, sa_core::crt::rand)).fmul_m(k(0x3800_0100))
}

/// `Fx_c::AddWheelGrass(CVehicle* veh, CVector pos, bool sand, float brightness)` (0x49FF20, thiscall `ret
/// 0x18`; `this` and `sand` unused).
///
/// Only for a vehicle driven by player 0 or 1. With `d2 = (dz² + dy²) + dx²` from the camera: nothing
/// above 625 (NaN too); above 400 only every fourth frame (`(frame + model low byte) & 3 == 0`); above 64,
/// or when the player has no vehicle (`FindPlayerVehicle(-1, false)`), every other frame. Then a mult
/// (0.03, 0.09, 0.03, 1, 0, 0, 0.05) and three particles on 0xA9AE3C: size `r*0.03 + 0.03`, velocity
/// `(r * speed.x * -1.5, r * speed.y * -1.5, r*1.5 + 2)`, position `pos + (r*0.4 - 0.2, r*0.4 - 0.2, 0)`
/// (`(r*0.4 + p) - 0.2`), `AddParticle(p, v, 0, mult, -1, brightness, 0.6, 0)`; six `rand()` per particle
/// in that order. The original keeps a temporary in its `brightness` argument slot (stack only).
#[hook(addr = 0x49FF20, conv = thiscall, name = "Fx_c::AddWheelGrass", this = "Fx_c", expect = "83 EC 34 56 8B 74 24 3C",
       unverified = "samp2-fx-1", route = "L2", hazards = "heap-writes", recs = "driving",
       note = "rand at 0x4A0035/0x4A0065/0x4A008D/0x4A00A8/0x4A00DF/0x4A0106 per particle")]
#[allow(clippy::too_many_arguments)]
pub fn fx_add_wheel_grass(_this: GPtr<u8>, veh: GPtr<u8>, px: f32, py: f32, pz: f32, _sand: u32, bright: f32) {
    if rd::<GPtr<u8>>(veh, 0x460) != find_player_ped(0) && rd::<GPtr<u8>>(veh, 0x460) != find_player_ped(1) {
        return;
    }
    let pv = find_player_vehicle(-1, 0);
    let m = g::<u32>(0xB6_F03C).read();
    let cam = if m != 0 { GPtr::<u8>::from_addr(m as usize + 0x30) } else { g::<u8>(0xB6_F02C) };
    let dx = x(rd::<f32>(cam, 0)).fsub_m(px);
    let dy = x(rd::<f32>(cam, 4)).fsub_m(py);
    let dz = x(rd::<f32>(cam, 8)).fsub_m(pz);
    let d2 = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
    if d2.fcom(x(k(0x441C_4000))).test_parity(0x41) {
        return;
    }
    let frame = sa_core::timer::FRAME_COUNTER.get();
    let tick = frame.wrapping_add(u32::from(rd::<u8>(veh, 0x22))) as u8;
    if !d2.fcom(x(k(0x43C8_0000))).test_any(0x41) {
        if tick & 3 != 0 {
            return;
        }
    } else if (!d2.fcom(x(k(0x4280_0000))).test_any(0x41) || pv.is_null()) && tick & 1 != 0 {
        return;
    }
    let mut mult = [0u32; 8];
    let mp: GPtr<u8> = GPtr::from_raw(mult.as_mut_ptr()).cast();
    fx_prt_mult_ctor(mp, k(0x3CF5_C28F), k(0x3DB8_51EC), k(0x3CF5_C28F), 1.0, 0.0, 0.0, k(0x3D4C_CCCD));
    for _ in 0..3 {
        let size = rnd(0x4A_0035).fmul_m(k(0x3CF5_C28F)).fadd_m(k(0x3CF5_C28F)).to_f32();
        mp.byte_add(0x10).cast::<f32>().write(size);
        let sx = x(rd::<f32>(veh, 0x44)).fmul_m(k(0xBFC0_0000)).to_f32();
        let vx = rnd(0x4A_0065).fmul_m(sx).to_f32();
        let sy = x(rd::<f32>(veh, 0x48)).fmul_m(k(0xBFC0_0000)).to_f32();
        let vy = rnd(0x4A_008D).fmul_m(sy).to_f32();
        let vz = rnd(0x4A_00A8).fmul_m(k(0x3FC0_0000)).fadd_m(k(0x4000_0000)).to_f32();
        let ppx = rnd(0x4A_00DF).fmul_m(k(0x3ECC_CCCD)).fadd_m(px).fsub_m(k(0x3E4C_CCCD)).to_f32();
        let ppy = rnd(0x4A_0106).fmul_m(k(0x3ECC_CCCD)).fadd_m(py).fsub_m(k(0x3E4C_CCCD)).to_f32();
        let mut p = [ppx, ppy, pz];
        let mut v = [vx, vy, vz];
        fx_add_particle(g::<GPtr<u8>>(0xA9_AE3C).read(), GPtr::from_raw(p.as_mut_ptr()).cast(),
            GPtr::from_raw(v.as_mut_ptr()).cast(), 0.0, mp, -1.0, bright, k(0x3F19_999A), 0);
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_FX_ADD_WHEEL_GRASS];
