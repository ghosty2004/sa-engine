//! PORT-VEH-3 (batch `veh3-3`): `CAutomobile::BurstTyre` and `CAutomobile::Fix`. Ported from the
//! 1.0 US bytes; gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::common::{rd, wr};

orig! {
    /// `CDamageManager::GetWheelStatus(eCarWheel)` (0x6C21B0)
    fn dm_get_wheel_status(dm: GPtr<u8>, wheel: i32) -> u32 = thiscall 0x6C21B0 => crate::damage::get_wheel_status;
    /// `CDamageManager::SetWheelStatus(eCarWheel, uint32)` (0x6C21A0)
    fn dm_set_wheel_status(dm: GPtr<u8>, wheel: i32, status: u32) = thiscall 0x6C21A0 => crate::damage::set_wheel_status;
    /// `CDamageManager::SetDoorStatus(eDoors, uint32)` (0x6C21C0)
    fn dm_set_door_status(dm: GPtr<u8>, door: i32, status: u32) = thiscall 0x6C21C0 => crate::damage::set_door_status;
    /// `CDamageManager::ResetDamageStatus()` (0x6C20E0)
    fn dm_reset_damage_status(dm: GPtr<u8>) = thiscall 0x6C20E0 => crate::damage::reset_damage_status;
    /// `CStats::IncrementStat(uint16, float)` (0x55C180)
    fn stats_increment_stat(stat: u32, v: f32) = cdecl 0x55C180 => unported;
    /// `CAEVehicleAudioEntity::AddAudioEvent(eAudioEvents, float)` (0x4F6420)
    fn audio_add_event(a: GPtr<u8>, event: u32, volume: f32) = thiscall 0x4F6420 => unported;
    /// `CCarCtrl::SwitchVehicleToRealPhysics(CVehicle*)` (0x423FC0)
    fn switch_vehicle_to_real_physics(v: GPtr<u8>) = cdecl 0x423FC0 => unported;
    /// `CGeneral::GetRandomNumberInRange(float, float)` (0x41BD90), ST0
    fn random_float_in_range(lo: f32, hi: f32) -> f64 = cdecl 0x41BD90 => unported;
    /// `operator*(float, const CVector&)` (0x40FE90): hidden return pointer first
    fn scalar_mul_vector(out: GPtr<u8>, s: f32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => sa_world::world::mvpa_math::scalar_mul_vector;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0)
    fn apply_move_force(e: GPtr<u8>, force: CVector) = thiscall 0x5429F0 => sa_world::physical::forces::apply_move_force;
    /// `CPhysical::ApplyTurnForce(CVector, CVector)` (0x542A50)
    fn apply_turn_force(e: GPtr<u8>, force: CVector, point: CVector) = thiscall 0x542A50 => sa_world::physical::forces::apply_turn_force;
    /// `RpClumpForAllAtomics(RpClump*, callback, void*)` (0x749B70)
    fn rp_clump_for_all_atomics(clump: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `CMatrix::CMatrix(RwMatrix*, bool)` (0x59C050)
    fn matrix_ctor_rw(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) -> GPtr<u8> = thiscall 0x59C050 => unported;
    /// `CMatrix::SetTranslate(CVector)` (0x59AF40)
    fn matrix_set_translate(m: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x59AF40 => unported;
    /// `CMatrix::UpdateRW()` (0x59BBB0)
    fn matrix_update_rw(m: GPtr<u8>) = thiscall 0x59BBB0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
}

/// `matrix row * (rand(-0.03, 0.03) * mass_at)` (a temporary `CVector`).
fn kick(this: GPtr<u8>, mass_off: isize) -> CVector {
    let c = f32::from_bits(0x3CF5_C28F);
    let k = X87::from_f64_exact(random_float_in_range(-c, c)).fmul_m(rd::<f32>(this, mass_off)).to_f32();
    let mut out = [0f32; 3];
    let r = scalar_mul_vector(GPtr::from_raw(&mut out as *mut [f32; 3]).cast(), k, rd::<GPtr<u8>>(this, 0x14));
    CVector::new(rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8))
}

/// `CAutomobile::BurstTyre(uint8 component, bool applyForces)` (0x6A32B0): not for the Rhino (432),
/// tyre-proof cars (+0x42B bit 7) or physical flag 0x20000000. The wheel is the component (13/14/15/16
/// -> 0/2/1/3, any other value used as is); an intact wheel (status 0) bursts (status 1): stat 129,
/// audio event 0x5A, a simple-physics car (state 2) switches to real physics, and with forces a
/// random push along `right` (`rand(-0.03, 0.03)` x mass) and a random twist (x turn mass, at the
/// forward point). True when it burst.
#[hook(addr = 0x6A32B0, conv = thiscall, name = "CAutomobile::BurstTyre", this = "CAutomobile",
       expect = "83 EC 0C 53 56",
       unverified = "veh3-3", route = "L2", hazards = "none",
       recs = "driving", note = "GetRandomNumberInRange consumes rand() (its own call site)")]
pub fn automobile_burst_tyre(this: GPtr<u8>, comp: u8, apply_forces: u8) -> u8 {
    if rd::<u16>(this, 0x22) == 0x1B0 || (rd::<u8>(this, 0x42B) as i8) < 0 || rd::<u32>(this, 0x40) & 0x2000_0000 != 0 {
        return 0;
    }
    let w = match comp {
        13 => 0,
        14 => 2,
        15 => 1,
        16 => 3,
        c => c,
    };
    let dm = this.byte_add(0x5A0);
    if dm_get_wheel_status(dm, i32::from(w)) != 0 {
        return 0;
    }
    dm_set_wheel_status(dm, i32::from(w), 1);
    stats_increment_stat(0x81, 1.0);
    audio_add_event(this.byte_add(0x138), 0x5A, 0.0);
    if rd::<u8>(this, 0x36) & 0xF8 == 0x10 {
        switch_vehicle_to_real_physics(this);
    }
    if apply_forces != 0 {
        let f = kick(this, 0x8C);
        apply_move_force(this, f);
        let t = kick(this, 0x90);
        let m = rd::<GPtr<u8>>(this, 0x14);
        let p = CVector::new(rd::<f32>(m, 0x10), rd::<f32>(m, 0x14), rd::<f32>(m, 0x18));
        apply_turn_force(this, t, p);
        wr::<u8>(this, 0xB8, 0);
    }
    1
}

/// `CAutomobile::Fix()` (0x6A3440): damage reset; handling model flag 0x100 marks the doors 2..5
/// missing; +0x42A bit 0 cleared; every atomic through `HideAllComponentsAtomicCB` (0x4C7790, flag
/// 2); the 17 nodes from +0x668 get their rotation reset; the four wheels status 0; the four
/// bouncing panels (+0x6AC, 0x20 each) node id -1.
#[hook(addr = 0x6A3440, conv = thiscall, name = "CAutomobile::Fix", this = "CAutomobile",
       expect = "6A FF 64 A1 00 00 00 00",
       unverified = "veh3-3", route = "L2", hazards = "heap-writes",
       note = "writes the RwFrame modelling matrices and the atomic flags; the SEH frame is not reproduced")]
pub fn automobile_fix(this: GPtr<u8>) {
    let dm = this.byte_add(0x5A0);
    dm_reset_damage_status(dm);
    if rd::<u32>(rd::<GPtr<u8>>(this, 0x384), 0xCC) & 0x100 != 0 {
        for d in 2..6 {
            dm_set_door_status(dm, d, 4);
        }
    }
    wr::<u8>(this, 0x42A, rd::<u8>(this, 0x42A) & 0xFE);
    rp_clump_for_all_atomics(rd::<GPtr<u8>>(this, 0x18), 0x004C_7790, 2);
    for k in 0..0x11isize {
        let node = rd::<GPtr<u8>>(this, 0x668 + 4 * k);
        if node.is_null() {
            continue;
        }
        let mut buf = [0u32; 0x12];
        let m: GPtr<u8> = GPtr::from_raw(&mut buf as *mut [u32; 0x12]).cast();
        matrix_ctor_rw(m, node.byte_add(0x10), 0);
        matrix_set_translate(m, rd::<u32>(m, 0x30), rd::<u32>(m, 0x34), rd::<u32>(m, 0x38));
        matrix_update_rw(m);
        matrix_dtor(m);
    }
    for w in 0..4 {
        dm_set_wheel_status(dm, w, 0);
    }
    for k in 0..4isize {
        wr::<u16>(this, 0x6AC + 0x20 * k, 0xFFFF);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_AUTOMOBILE_BURST_TYRE, HOOK_AUTOMOBILE_FIX];
