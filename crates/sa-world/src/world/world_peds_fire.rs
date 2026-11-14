//! WRITER-WORLD-2 (batch `world6-5`): `CWorld::SetPedsOnFire`, `CWorld::SetPedsChoking` and
//! `CWorld::FindUnsuspectingTargetPed`.
//! Ported from the 1.0 US bytes; gta-reversed as a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use super::common::{rd, x};

use crate::bindings::get_local_direction;
use crate::bindings::start_fire_on;
use sa_core::bindings::ped_is_alive;
use sa_core::bindings::tm_get_active_task as get_active_task;
use sa_stream::bindings::ped_is_player;
use crate::bindings::generate_damage_event;
use sa_core::bindings::vector_sub;
orig! {
    /// `CTask::GetTaskType()` (vtable +0x10)
    fn v_task_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
}

/// The peds of the pool (highest slot first) that may be hurt by an area effect: not dead
/// (+0x530 != 55), not in a vehicle, not burning (+0x730), not fire proof (+0x40 bit 19).
fn each_area_ped(mut f: impl FnMut(GPtr<u8>)) {
    let pool = GPtr::<GPtr<u8>>::from_addr(0xB7_4490).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        let flag = rd::<i8>(rd::<GPtr<u8>>(pool, 4), (i - 1) as isize);
        i -= 1;
        if flag < 0 {
            continue;
        }
        let ped = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0x7C4);
        if ped.is_null()
            || rd::<u32>(ped, 0x530) == 0x37
            || rd::<u32>(ped, 0x46C) & 0x100 != 0
            || !rd::<GPtr<u8>>(ped, 0x730).is_null()
            || rd::<u32>(ped, 0x40) & 0x8_0000 != 0
        {
            continue;
        }
        f(ped);
    }
}

use crate::world::common::entity_pos as ped_pos;

/// `CWorld::SetPedsChoking(float x, float y, float z, float radius, CEntity* creator)` (0x565800):
/// every area ped (see `each_area_ped`) that is not of type 2 (+0x484) and lies within 5 vertically
/// and `radius` in x and y gets a tear-gas damage event (weapon 17, 1 damage, torso) from `creator`,
/// its direction from the ped's offset to the point.
#[hook(addr = 0x565800, conv = cdecl, name = "CWorld::SetPedsChoking",
       expect = "83 EC 24 8B 15 90 44 B7 00",
       unverified = "world6-5", route = "L2", hazards = "heap-writes")]
pub fn set_peds_choking(px: f32, py: f32, pz: f32, radius: f32, creator: GPtr<u8>) {
    each_area_ped(|ped| {
        if rd::<u8>(ped, 0x484) == 2 {
            return;
        }
        let pos = ped_pos(ped);
        if abs_diff(rd::<f32>(pos, 8), pz).fcom(x(5.0)).test_parity(0x05)
            || abs_diff(rd::<f32>(pos, 0), px).fcom(x(radius)).test_parity(0x05)
            || abs_diff(rd::<f32>(pos, 4), py).fcom(x(radius)).test_parity(0x05)
        {
            return;
        }
        let mut target = [px, py, pz];
        let mut out = [0f32; 3];
        let r = vector_sub(GPtr::from_raw(&mut out as *mut [f32; 3]).cast(), ped_pos(ped),
                           GPtr::from_raw(&mut target as *mut [f32; 3]).cast());
        let mut v2 = [rd::<u32>(r, 0), rd::<u32>(r, 4)];
        let dir = get_local_direction(ped, GPtr::from_raw(&mut v2 as *mut [u32; 2]).cast());
        generate_damage_event(ped, creator, 0x11, 1, 3, dir as u32);
    });
}

/// `|v - c|` the way the bytes compute it (negated when the difference is below 0).
fn abs_diff(v: f32, c: f32) -> sa_math::x87::X87 {
    let d = x(v).fsub_m(c);
    if !d.fcom(x(0.0)).test_parity(0x05) { d.fchs() } else { d }
}

/// `CWorld::SetPedsOnFire(float x, float y, float z, float radius, CEntity* creator)` (0x565610):
/// every ped of the pool (highest slot first) that is not dead (+0x530 != 55), not in a vehicle,
/// not burning (+0x730) and not fire proof (+0x40 bit 19), within 5 vertically and within `radius`
/// (half of it for the creator itself) in x and y, is set on fire (size 0.8, 7000 ms, 2 generations)
/// by `creator`; peds with +0x40 bit 22 only by a player ped (or a null creator).
#[hook(addr = 0x565610, conv = cdecl, name = "CWorld::SetPedsOnFire",
       expect = "83 EC 0C A1 90 44 B7 00",
       unverified = "world6-5", route = "L2", hazards = "heap-writes")]
pub fn set_peds_on_fire(px: f32, py: f32, pz: f32, radius: f32, creator: GPtr<u8>) {
    each_area_ped(|ped| {
        let f40 = rd::<u32>(ped, 0x40);
        let pos = ped_pos(ped);
        if abs_diff(rd::<f32>(pos, 8), pz).fcom(x(5.0)).test_parity(0x05) {
            return;
        }
        let factor: f32 = if ped == creator { 0.5 } else { 1.0 };
        let rf = x(factor).fmul_m(radius).to_f32();
        if abs_diff(rd::<f32>(pos, 0), px).fcom(x(rf)).test_parity(0x05)
            || abs_diff(rd::<f32>(pos, 4), py).fcom(x(rf)).test_parity(0x05)
        {
            return;
        }
        if f40 & 0x40_0000 != 0
            && !creator.is_null()
            && (rd::<u8>(creator, 0x36) & 7 != 3 || ped_is_player(creator) == 0)
        {
            return;
        }
        start_fire_on(GPtr::from_addr(0xB7_1F80), ped, creator, f32::from_bits(0x3F4C_CCCD), 1, 0x1B58, 2);
    });
}

/// `CWorld::FindUnsuspectingTargetPed(CVector pos, CVector target)` (0x566DA0): the nearest (2D,
/// `fsqrt`, below 9999.9, kept as f32) random ped (+0x484 == 1) that is alive, of a gang type (7..16)
/// or type 4, not running task 0x390, and whose forward axis points towards `target` (`(target -
/// pos) . forward > 0`, summed z, y, x); null when none.
#[hook(addr = 0x566DA0, conv = cdecl, name = "CWorld::FindUnsuspectingTargetPed",
       expect = "83 EC 08 55 8B 2D 90 44 B7 00",
       unverified = "world6-5", route = "L2", hazards = "x87-transcendental")]
#[allow(clippy::too_many_arguments)]
pub fn find_unsuspecting_target_ped(px: f32, py: f32, _pz: f32, tx: f32, ty: f32, tz: f32) -> GPtr<u8> {
    let mut best = f32::from_bits(0x461C_3F9A);
    let mut found = GPtr::<u8>::NULL;
    let pool = GPtr::<GPtr<u8>>::from_addr(0xB7_4490).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        let flag = rd::<i8>(rd::<GPtr<u8>>(pool, 4), (i - 1) as isize);
        i -= 1;
        if flag < 0 {
            continue;
        }
        let ped = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0x7C4);
        if ped.is_null() || rd::<u8>(ped, 0x484) != 1 || ped_is_alive(ped) == 0 {
            continue;
        }
        let pt = rd::<i32>(ped, 0x598);
        if !((7..=16).contains(&pt) || pt == 4) {
            continue;
        }
        let task = get_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
        if !task.is_null() && v_task_type(task) == 0x390 {
            continue;
        }
        let pos = ped_pos(ped);
        let dx = x(px).fsub_m(rd::<f32>(pos, 0));
        let dy = x(py).fsub_m(rd::<f32>(pos, 4));
        let d = dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt();
        if d.fcom(x(best)).test_parity(0x05) {
            continue;
        }
        let m = rd::<GPtr<u8>>(ped, 0x14);
        let ex = x(tx).fsub_m(rd::<f32>(pos, 0));
        let ey = x(ty).fsub_m(rd::<f32>(pos, 4));
        let ez = x(tz).fsub_m(rd::<f32>(pos, 8));
        let dot = ez.fmul_m(rd::<f32>(m, 0x18)).fadd(ey.fmul_m(rd::<f32>(m, 0x14))).fadd(ex.fmul_m(rd::<f32>(m, 0x10)));
        if !dot.fcom(x(0.0)).test_any(0x41) {
            best = d.to_f32();
            found = ped;
        }
    }
    found
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[HOOK_SET_PEDS_ON_FIRE, HOOK_SET_PEDS_CHOKING, HOOK_FIND_UNSUSPECTING_TARGET_PED];
