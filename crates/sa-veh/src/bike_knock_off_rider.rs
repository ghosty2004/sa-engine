//! WRITER-NATIVE (batch `native-4`): `CBike::DamageKnockOffRider` (0x6B5A10), reached from bike collisions
//! (police bikes, ambient bikers) on the native walk, and the `CEventKnockOffBike` destructor (0x4AFE70) it
//! calls. Ported from the 1.0 US bytes; gta-reversed (`Bike.cpp`) as a reading aid. x87 arithmetic follows
//! the original's instruction order on the `sa_math::x87::X87` model.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{rd, wr, x};
use sa_core::mem::GPtr;

orig! {
    /// `CPed::GetBikeRidingSkill()` (0x5DF510, ST0)
    fn ped_get_bike_riding_skill(p: GPtr<u8>) -> f64 = thiscall 0x5DF510 => unported;
    /// `CTaskManager::GetActiveTask()` (0x681720)
    fn task_manager_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTask::GetTaskType()` (vtable slot 0x10)
    fn task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CCullZones::CamStairsForPlayer()` (0x72DD30)
    fn cull_zones_cam_stairs_for_player() -> u8 = cdecl 0x72DD30 => unported;
    /// `CCullZones::FindZoneWithStairsAttributeForPlayer()` (0x72DAD0)
    fn cull_zones_find_zone_with_stairs_for_player() -> u32 = cdecl 0x72DAD0 => unported;
    /// `CPed::GetLocalDirection(const CVector2D&)` (0x5DEF60)
    fn ped_get_local_direction(p: GPtr<u8>, dir: GPtr<u8>) -> i32 = thiscall 0x5DEF60 => unported;
    /// `CEventKnockOffBike::CEventKnockOffBike(CVehicle*, CVector* moveSpeed, CVector* impactVelocity, float
    /// damageIntensity, float height, uint8 type, uint8 direction, int32 time, CPed*, bool isVictimDriver, bool
    /// forceKnockOff)` (0x4AFCF0)
    #[allow(clippy::too_many_arguments)]
    fn event_knock_off_bike_ctor(e: GPtr<u8>, veh: GPtr<u8>, move_speed: GPtr<u8>, impact: GPtr<u8>, damage: u32, height: f32, kind: u32, dir: u32, time: u32, ped: u32, driver: u32, force: u32) -> GPtr<u8> = thiscall 0x4AFCF0 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(group: GPtr<u8>, e: GPtr<u8>, unused: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CEventKnockOffBike::~CEventKnockOffBike()` (0x4AFE70)
    fn event_knock_off_bike_dtor(e: GPtr<u8>) = thiscall 0x4AFE70 => event_knock_off_bike_destructor;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
}

fn c(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// `CPed::CantBeKnockedOffBike` (+0x474 bits 27..28).
fn knock_off_class(p: GPtr<u8>) -> u32 {
    rd::<u32>(p, 0x474) & 0x1800_0000
}

/// `(m.a.y * v.y + m.a.z * v.z) + m.a.x * v.x` for the matrix row at `row`.
fn dot_row(m: GPtr<u8>, row: isize, v: GPtr<u8>) -> X87 {
    x(rd::<f32>(m, row + 4))
        .fmul_m(rd::<f32>(v, 4))
        .fadd(x(rd::<f32>(m, row + 8)).fmul_m(rd::<f32>(v, 8)))
        .fadd(x(rd::<f32>(m, row)).fmul_m(rd::<f32>(v, 0)))
}

/// `CBike::DamageKnockOffRider(CVehicle* vehicle, float damageIntensity, uint16 pieceType, CEntity* damager,
/// const CVector& collisionPos, const CVector& impactVelocity)` (0x6B5A10, cdecl, bool in AL, SEH frame).
///
/// `force = damage / mass * 800`; the player's bike x 0.75 and x `(1 - skill * 0.5)`, other drivers (unless
/// "always normal") x `(1 - skill * 0.6)`. Nothing without a driving driver (state 50), with `force <= 10`,
/// or when the driver's active task is `TASK_SIMPLE_BE_HIT` (0x3FE) and he is not a cop (+0x598 = 6).
/// Weights from the matrix dots with the impact velocity: forward 0.6, or `7 t^2 + 0.6` above |0.85| (`t` =
/// the impact z when >= 0.85 by the original's `((vy + vx) * 0 + vz)`), 5 when upside down; Sanchez (468) /
/// quads (+0x594 = 2) scale them; a forward impact x `(1 - skill * 0.6)`. `force *= |fwd| * fw + max(up, 0) *
/// uw + |right| * 0.45 - min(up, 0) * bw`; 0 for the player on stairs. Knocked off above 75 (20 for "always
/// hard"), never for "never" drivers or passengers: a `CEventKnockOffBike` (type 0x31, height `force *
/// 0.05`) to the driver's and the passenger's event groups, the direction from the driver
/// (`GetLocalDirection(-impact.xy)`). Returns 1 then.
#[hook(addr = 0x6B5A10, conv = cdecl, name = "CBike::DamageKnockOffRider",
       expect = "6A FF 68 D3 81 84 00",
       unverified = "native-4", route = "L2", hazards = "heap-writes,ax-al-return",
       recs = "grove-walk,driving",
       note = "SEH frame; events go to the peds' event groups; the original reuses the back-weight slot for the 2D direction")]
pub fn damage_knock_off_rider(
    veh: GPtr<u8>,
    damage: f32,
    _piece: u32,
    _damager: GPtr<u8>,
    _pos: GPtr<u8>,
    impact: GPtr<u8>,
) -> u8 {
    let mut force = x(damage).fdiv_m(rd::<f32>(veh, 0x8C)).fmul_m(800.0).to_f32();
    let driver = rd::<GPtr<u8>>(veh, 0x460);
    if rd::<u8>(veh, 0x36) & 0xF8 == 0 {
        force = x(force).fmul_m(0.75).to_f32();
        if !driver.is_null() {
            let s = X87::from_f64_exact(ped_get_bike_riding_skill(driver));
            force = s.fmul_m(0.5).fsubr_m(1.0).fmul_m(force).to_f32();
        }
    } else if !driver.is_null() && knock_off_class(driver) != 0x1000_0000 {
        let s = X87::from_f64_exact(ped_get_bike_riding_skill(driver));
        force = s.fmul_m(c(0x3F19_999A)).fsubr_m(1.0).fmul_m(force).to_f32();
    }
    let driver = rd::<GPtr<u8>>(veh, 0x460);
    if driver.is_null() || rd::<u32>(driver, 0x530) != 0x32 {
        return 0;
    }
    if x(force).fcom(x(10.0)).test_any(0x41) {
        return 0;
    }
    let intel = rd::<GPtr<u8>>(driver, 0x47C);
    if !task_manager_get_active_task(intel.byte_add(4)).is_null() {
        let d = rd::<GPtr<u8>>(veh, 0x460);
        let t = task_manager_get_active_task(rd::<GPtr<u8>>(d, 0x47C).byte_add(4));
        if task_get_type(t) == 0x3FE && rd::<u32>(rd::<GPtr<u8>>(veh, 0x460), 0x598) != 6 {
            return 0;
        }
    }
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let mut fw: f32;
    if !dot_row(m, 0x10, impact).fabs().fcom(x(c(0x3F59_999A))).test_any(0x41) {
        let mut t = x(rd::<f32>(impact, 4))
            .fadd_m(rd::<f32>(impact, 0))
            .fmul_m(0.0)
            .fadd_m(rd::<f32>(impact, 8));
        if !x(0.0).fcom(t).test_any(0x41) || !t.fcom(x(c(0x3F59_999A))).test_parity(0x05) {
            t = x(0.0);
        }
        fw = x(7.0).fmul(t).fmul(t).fadd_m(c(0x3F19_999A)).to_f32();
    } else {
        fw = c(0x3F19_999A);
    }
    let mut bw: f32 = 1.5;
    if !x(rd::<f32>(m, 0x28)).fcom(x(0.0)).test_parity(0x05) {
        fw = 5.0;
    }
    let mut uw: f32 = c(0x3D4C_CCCD);
    if rd::<u16>(veh, 0x22) == 0x1D4 {
        fw = x(c(0x3F26_6666)).fmul_m(fw).to_f32();
        uw = x(0.75).fmul_m(c(0x3D4C_CCCD)).to_f32();
    } else if rd::<u32>(veh, 0x594) == 2 {
        bw = x(2.0).fmul_m(1.5).to_f32();
        fw = x(c(0x3F26_6666)).fmul_m(fw).to_f32();
        uw = x(0.75).fmul_m(c(0x3D4C_CCCD)).to_f32();
    }
    let driver = rd::<GPtr<u8>>(veh, 0x460);
    if !driver.is_null() && !dot_row(m, 0x10, impact).fcom(x(0.0)).test_any(0x41) {
        let s = X87::from_f64_exact(ped_get_bike_riding_skill(driver));
        fw = s.fmul_m(c(0x3F19_999A)).fsubr_m(1.0).fmul_m(fw).to_f32();
    }
    let m = rd::<GPtr<u8>>(veh, 0x14);
    let u = dot_row(m, 0x20, impact);
    let max_u = if u.fcom(x(0.0)).test_parity(0x05) { u } else { x(0.0) };
    // the second dot multiplies `v.x * up.x` (same product)
    let u2 = dot_row(m, 0x20, impact);
    let min_u = if u2.fcom(x(0.0)).test_any(0x41) { u2 } else { x(0.0) };
    let fd = dot_row(m, 0x10, impact);
    let rdot = dot_row(m, 0, impact);
    let rabs = rdot.fabs().to_f32();
    let s1 = fd.fabs().fmul_m(fw).fadd(max_u.fmul_m(uw));
    let s2 = s1.fadd(x(rabs).fmul_m(c(0x3EE6_6666)));
    let s3 = s2.fsub(min_u.fmul_m(bw));
    force = s3.fmul_m(force).to_f32();
    let driver = rd::<GPtr<u8>>(veh, 0x460);
    if ped_is_player(driver) != 0 && cull_zones_cam_stairs_for_player() != 0 && cull_zones_find_zone_with_stairs_for_player() != 0 {
        force = 0.0;
    }
    if x(force).fcom(x(75.0)).test_any(0x41) {
        if knock_off_class(rd::<GPtr<u8>>(veh, 0x460)) != 0x1800_0000 {
            return 0;
        }
        if x(force).fcom(x(20.0)).test_any(0x41) {
            return 0;
        }
    }
    let driver = rd::<GPtr<u8>>(veh, 0x460);
    if !driver.is_null() && knock_off_class(driver) == 0x0800_0000 {
        return 0;
    }
    let passenger = rd::<GPtr<u8>>(veh, 0x464);
    if !passenger.is_null() && knock_off_class(passenger) == 0x0800_0000 {
        return 0;
    }
    let mut dir2 = [
        x(rd::<f32>(impact, 0)).fmul_m(-1.0).to_f32(),
        x(rd::<f32>(impact, 4)).fmul_m(-1.0).to_f32(),
    ];
    let dir2p: GPtr<u8> = GPtr::from_raw(&mut dir2 as *mut [f32; 2]).cast();
    let height = |f: f32| x(c(0x3D4C_CCCD)).fmul_m(f).to_f32();
    let mut dir: i32 = -10;
    if !driver.is_null() {
        dir = ped_get_local_direction(driver, dir2p);
        let mut ev = [0u32; 16];
        let evp: GPtr<u8> = GPtr::from_raw(&mut ev as *mut [u32; 16]).cast();
        event_knock_off_bike_ctor(evp, veh, veh.byte_add(0x44), impact, damage.to_bits(), height(force), 0x31, dir as u32, 0, 0, 1, 0);
        let d = rd::<GPtr<u8>>(veh, 0x460);
        event_group_add(rd::<GPtr<u8>>(d, 0x47C).byte_add(0x68), evp, 0);
        event_knock_off_bike_dtor(evp);
    }
    let passenger = rd::<GPtr<u8>>(veh, 0x464);
    if !passenger.is_null() {
        if dir == -10 {
            dir = ped_get_local_direction(passenger, dir2p);
        }
        let mut ev = [0u32; 16];
        let evp: GPtr<u8> = GPtr::from_raw(&mut ev as *mut [u32; 16]).cast();
        event_knock_off_bike_ctor(evp, veh, veh.byte_add(0x44), impact, damage.to_bits(), height(force), 0x31, dir as u32, 0, 0, 0, 0);
        let p = rd::<GPtr<u8>>(veh, 0x464);
        event_group_add(rd::<GPtr<u8>>(p, 0x47C).byte_add(0x68), evp, 0);
        event_knock_off_bike_dtor(evp);
    }
    1
}

/// `CEventKnockOffBike::~CEventKnockOffBike()` (0x4AFE70, SEH frame): vtable 0x85B2E0, `CleanUpOldReference`
/// on the ped (+0x38) and the vehicle (+0x30) references when set, then the `CEvent` vtable 0x85AB38.
#[hook(addr = 0x4AFE70, conv = thiscall, name = "FUN_004afe70",
       expect = "6A FF 68 68 9F 83 00",
       unverified = "native-4", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn event_knock_off_bike_destructor(this: GPtr<u8>) {
    wr::<u32>(this, 0, 0x85_B2E0);
    let p = rd::<GPtr<u8>>(this, 0x38);
    if !p.is_null() {
        entity_clean_up_old_reference(p, this.byte_add(0x38));
    }
    let v = rd::<GPtr<u8>>(this, 0x30);
    if !v.is_null() {
        entity_clean_up_old_reference(v, this.byte_add(0x30));
    }
    wr::<u32>(this, 0, 0x85_AB38);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_DAMAGE_KNOCK_OFF_RIDER, HOOK_EVENT_KNOCK_OFF_BIKE_DESTRUCTOR];
