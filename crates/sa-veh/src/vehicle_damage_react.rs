//! `CVehicle::ReactToVehicleDamage(CPed*)` (0x6D5490, batch `attr-3`): the driver and front passenger look at the
//! ped who damaged the car (or at each other). Ported from the 1.0 US bytes. Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{ftol32, g, rand_at, rand_frac, rd};

orig! {
    /// `IKChainManager_c::LookAt(name, ped, target, time, bone, pos, bool, speed, blend, priority, bool)` (0x618970,
    /// `ret 0x2C`)
    fn ik_look_at(m: GPtr<u8>, name: u32, ped: GPtr<u8>, target: GPtr<u8>, time: i32, bone: i32, pos: u32, b: u32, speed: u32, blend: i32, prio: i32, b2: u32) = thiscall 0x618970 => unported;
}

/// `g_ikChainMan`
const IK_CHAIN_MAN: usize = 0xC15448;
/// `"ReactToVhclDam"`
const NAME: u32 = 0x871E68;

fn look(ped: GPtr<u8>, target: GPtr<u8>, time: i32) {
    ik_look_at(GPtr::from_addr(IK_CHAIN_MAN), NAME, ped, target, time, 5, 0, 0, 0x3E80_0000, 500, 3, 0);
}

/// `CVehicle::ReactToVehicleDamage(CPed* causer)` (0x6D5490, thiscall, `ret 4`). Look time `2000 - ftol((rand() &
/// 0xFFFF) * 2^-15 * -3000.0)`, drawn before each of the two blocks (the second draw also when there is no driver).
/// Driver (+0x460): with a passenger (+0x464) it looks at the passenger when `rand() < 0x3FFF`, else at the causer;
/// without one at the causer. Passenger: at the driver when `rand() < 0x3FFF`, else at the causer. Head bone, speed
/// 0.25, blend 500, priority 3.
#[hook(addr = 0x6D5490, conv = thiscall, name = "CVehicle::ReactToVehicleDamage", expect = "51 53 56 57 8B F1 E8",
       unverified = "attr-3", route = "L2", hazards = "none", recs = "grove-walk",
       note = "rand() at 0x6D549B, 0x6D54DD, 0x6D553E, 0x6D5572")]
pub fn react_to_vehicle_damage(this: GPtr<u8>, causer: GPtr<u8>) {
    let m = g::<f32>(0x859A90).read();
    let t = 2000i32.wrapping_sub(ftol32(rand_frac(rand_at(0x6D549B)).fmul_m(m)));
    let driver = rd::<GPtr<u8>>(this, 0x460);
    if !driver.is_null() {
        if rd::<u32>(this, 0x464) != 0 {
            if rand_at(0x6D54DD) < 0x3FFF {
                look(rd::<GPtr<u8>>(this, 0x460), rd::<GPtr<u8>>(this, 0x464), t);
            } else {
                look(rd::<GPtr<u8>>(this, 0x460), causer, t);
            }
        } else {
            look(driver, causer, t);
        }
    }
    let t = 2000i32.wrapping_sub(ftol32(rand_frac(rand_at(0x6D553E)).fmul_m(m)));
    if rd::<u32>(this, 0x464) != 0 {
        if rand_at(0x6D5572) < 0x3FFF {
            look(rd::<GPtr<u8>>(this, 0x464), rd::<GPtr<u8>>(this, 0x460), t);
        } else {
            look(rd::<GPtr<u8>>(this, 0x464), causer, t);
        }
    }
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_REACT_TO_VEHICLE_DAMAGE];
