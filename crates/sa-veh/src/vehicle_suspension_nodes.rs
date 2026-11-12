//! PORT-VEH-3 (batch `veh3-1`): model-node setup, component positions, suspension resets and the
//! monster truck's suspension extension, the car pitch/roll/in-air queries and the heading matrix
//! helper. Ported from the 1.0 US bytes; gta-reversed as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{b, g, rd, wr, x};

orig! {
    /// `CVehicle::SetModelIndex(uint32)` (0x6D6A40)
    fn vehicle_set_model_index(v: GPtr<u8>, mi: u32) = thiscall 0x6D6A40 => unported;
    /// `CClumpModelInfo::FillFrameArray(RpClump*, RwFrame**)` (0x4C5440)
    fn fill_frame_array(clump: GPtr<u8>, frames: GPtr<u8>) = cdecl 0x4C5440 => unported;
    /// `RwFrameGetLTM(RwFrame*)` (0x7F0990)
    fn rw_frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
    /// CRT `printf` (0x821982) with two int arguments (the missing-node debug message)
    fn printf2(fmt: u32, a: i32, c: i32) -> i32 = cdecl 0x821982 => unported;
    /// `CAutomobile::ResetSuspension()` (0x6A2AE0, called directly)
    fn automobile_reset_suspension(car: GPtr<u8>) = thiscall 0x6A2AE0 => crate::automobile::reset_suspension;
    /// `CAutomobile::ProcessControlCollisionCheck(bool)` (0x6A29C0, called directly)
    fn automobile_pccc(car: GPtr<u8>, apply_speed: u32) = thiscall 0x6A29C0 => crate::automobile2::process_control_collision_check;
    /// `CMonsterTruck::ExtendSuspension()` (0x6C7D80)
    fn call_extend_suspension(car: GPtr<u8>) = thiscall 0x6C7D80 => monster_truck_extend_suspension_port;
}

/// `fcomp` + `fnstsw ax` + `test ah, mask` + `jp` taken.
#[inline]
fn jp(a: X87, m: f32, mask: u8) -> bool {
    a.fcom(x(m)).test_parity(mask)
}

/// Zero `n` node pointers at `this + off` (`rep stosd`) and fill them from the clump (+0x18).
fn setup_nodes(this: GPtr<u8>, off: isize, n: isize) {
    for k in 0..n {
        wr::<u32>(this, off + 4 * k, 0);
    }
    fill_frame_array(rd::<GPtr<u8>>(this, 0x18), this.byte_add(off));
}

/// `CAutomobile::SetupModelNodes()` (0x6A0770): clears the 25 node pointers at +0x648 and fills them
/// with `CClumpModelInfo::FillFrameArray`.
#[hook(addr = 0x6A0770, conv = thiscall, name = "CAutomobile::SetupModelNodes", this = "CAutomobile",
       expect = "8B D1 56 57 8D B2 48 06 00 00",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes",
       note = "FillFrameArray writes the node array through the clump's frame plugin")]
pub fn automobile_setup_model_nodes(this: GPtr<u8>) {
    setup_nodes(this, 0x648, 0x19);
}

/// `CBoat::SetupModelNodes()` (0x6F01A0): clears the 12 node pointers at +0x5B0 and fills them.
#[hook(addr = 0x6F01A0, conv = thiscall, name = "CBoat::SetupModelNodes", this = "CBoat",
       expect = "8B D1 56 57 8D B2 B0 05 00 00",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes")]
pub fn boat_setup_model_nodes(this: GPtr<u8>) {
    setup_nodes(this, 0x5B0, 0xC);
}

/// `CAutomobile::SetModelIndex(uint32)` (0x6A6590): `CVehicle::SetModelIndex`, then the inlined
/// SetupModelNodes (25 nodes at +0x648).
#[hook(addr = 0x6A6590, conv = thiscall, name = "CAutomobile::SetModelIndex", this = "CAutomobile",
       expect = "8B 44 24 04 56",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes", recs = "driving")]
pub fn automobile_set_model_index(this: GPtr<u8>, mi: u32) {
    vehicle_set_model_index(this, mi);
    setup_nodes(this, 0x648, 0x19);
}

/// `CBike::SetModelIndex(uint32)` (0x6B8970): 10 nodes at +0x5A0.
#[hook(addr = 0x6B8970, conv = thiscall, name = "CBike::SetModelIndex", this = "CBike",
       expect = "8B 44 24 04 56",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes", recs = "driving")]
pub fn bike_set_model_index(this: GPtr<u8>, mi: u32) {
    vehicle_set_model_index(this, mi);
    setup_nodes(this, 0x5A0, 0xA);
}

/// `CBoat::SetModelIndex(uint32)` (0x6F1140): 12 nodes at +0x5B0.
#[hook(addr = 0x6F1140, conv = thiscall, name = "CBoat::SetModelIndex", this = "CBoat",
       expect = "8B 44 24 04 56",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes")]
pub fn boat_set_model_index(this: GPtr<u8>, mi: u32) {
    vehicle_set_model_index(this, mi);
    setup_nodes(this, 0x5B0, 0xC);
}

/// `CTrain::SetModelIndex(uint32)` (0x6F5530): 17 nodes at +0x668.
#[hook(addr = 0x6F5530, conv = thiscall, name = "CTrain::SetModelIndex", this = "CTrain",
       expect = "8B 44 24 04 56",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes", recs = "driving")]
pub fn train_set_model_index(this: GPtr<u8>, mi: u32) {
    vehicle_set_model_index(this, mi);
    setup_nodes(this, 0x668, 0x11);
}

/// Copy the position row (+0x30) of `node`'s LTM into `out`.
fn copy_ltm_pos(node: GPtr<u8>, out: GPtr<u8>) {
    let ltm = rw_frame_get_ltm(node);
    for k in 0..3 {
        wr::<u32>(out, 4 * k, rd::<u32>(ltm, 0x30 + 4 * k));
    }
}

/// `CBoat::GetComponentWorldPosition(int32, CVector&)` (0x6F01D0): the LTM position of node
/// `+0x5B0[comp]`. No null check and no range check (as the original).
#[hook(addr = 0x6F01D0, conv = thiscall, name = "CBoat::GetComponentWorldPosition", this = "CBoat",
       expect = "8B 44 24 04 8B 8C 81 B0 05 00 00",
       unverified = "veh3-1", route = "L2", hazards = "none")]
pub fn boat_get_component_world_position(this: GPtr<u8>, comp: i32, out: GPtr<u8>) {
    let node = rd::<GPtr<u8>>(this, 0x5B0 + (comp as isize) * 4);
    copy_ltm_pos(node, out);
}

/// `CBike::GetComponentWorldPosition(int32, CVector&)` (0x6B5990): the LTM position of node
/// `+0x5A0[comp]`; a missing node `printf`s 0x87146C with the model index and the component and
/// leaves `out` untouched.
#[hook(addr = 0x6B5990, conv = thiscall, name = "CBike::GetComponentWorldPosition", this = "CBike",
       expect = "8B 54 24 04 8B 84 91 A0 05 00 00",
       unverified = "veh3-1", route = "L2", hazards = "none")]
pub fn bike_get_component_world_position(this: GPtr<u8>, comp: i32, out: GPtr<u8>) {
    let node = rd::<GPtr<u8>>(this, 0x5A0 + (comp as isize) * 4);
    if node.is_null() {
        printf2(0x0087_146C, i32::from(rd::<i16>(this, 0x22)), comp);
        return;
    }
    copy_ltm_pos(node, out);
}

/// `CMonsterTruck::ResetSuspension()` (0x6C7D40): `CAutomobile::ResetSuspension`, then per wheel
/// +0x838 = +0x888 (the fully extended length) and +0x988 = 1.0.
#[hook(addr = 0x6C7D40, conv = thiscall, name = "CMonsterTruck::ResetSuspension", this = "CMonsterTruck",
       expect = "56 8B F1 E8 98 AD FD FF",
       unverified = "veh3-1", route = "L2", hazards = "none")]
pub fn monster_truck_reset_suspension(this: GPtr<u8>) {
    automobile_reset_suspension(this);
    for k in 0..4 {
        wr::<u32>(this, 0x838 + 4 * k, rd::<u32>(this, 0x888 + 4 * k));
        wr::<u32>(this, 0x988 + 4 * k, 0x3F80_0000);
    }
}

/// `CBike::ResetSuspension()` (0x6B6740): for the 2 axles +0x750 = 0 and +0x80C = 0, then for the 4
/// suspension lines +0x710 = 1.0 and +0x730 = 0.
#[hook(addr = 0x6B6740, conv = thiscall, name = "CBike::ResetSuspension", this = "CBike",
       expect = "8D 81 0C 08 00 00",
       unverified = "veh3-1", route = "L2", hazards = "none", recs = "driving")]
pub fn bike_reset_suspension(this: GPtr<u8>) {
    for k in 0..2 {
        wr::<u32>(this, 0x750 + 4 * k, 0);
        wr::<u32>(this, 0x80C + 4 * k, 0);
    }
    for k in 0..4 {
        wr::<u32>(this, 0x710 + 4 * k, 0x3F80_0000);
        wr::<u32>(this, 0x730 + 4 * k, 0);
    }
}

/// `CTrailer::ResetSuspension()` (0x6CEE50): `CAutomobile::ResetSuspension`, the two support ratios
/// (+0x9E0, +0x9E4) = 1.0, and +0x9EC = 0 when towed (+0x4C4) with +0x9EC above -1000 (or NaN),
/// else 1.0.
#[hook(addr = 0x6CEE50, conv = thiscall, name = "CTrailer::ResetSuspension", this = "CTrailer",
       expect = "56 8B F1 E8 88 3C FD FF",
       unverified = "veh3-1", route = "L2", hazards = "none")]
pub fn trailer_reset_suspension(this: GPtr<u8>) {
    automobile_reset_suspension(this);
    wr::<u32>(this, 0x9E0, 0x3F80_0000);
    wr::<u32>(this, 0x9E4, 0x3F80_0000);
    if rd::<u32>(this, 0x4C4) != 0 && !x(rd::<f32>(this, 0x9EC)).fcom(x(-1000.0)).test_any(0x41) {
        wr::<u32>(this, 0x9EC, 0);
        return;
    }
    wr::<u32>(this, 0x9EC, 0x3F80_0000);
}

/// `CMonsterTruck::ExtendSuspension()` (0x6C7D80): per wheel the length +0x838 shrinks by
/// `CTimer::ms_fTimeStep * this+0x998 * 0.1` (the first wheel's factor for all four, as the bytes
/// address it) and is clamped to [+0x888, +0x878]; the wheel ratio +0x7D4 = 1.0.
pub fn monster_truck_extend_suspension_port(this: GPtr<u8>) {
    for k in 0..4isize {
        let o = 0x838 + 4 * k;
        let v = x(g::<f32>(0xB7_CB5C).read())
            .fmul_m(rd::<f32>(this, 0x998))
            .fmul_m(f32::from_bits(0x3DCC_CCCD))
            .fsubr_m(rd::<f32>(this, o))
            .to_f32();
        wr::<f32>(this, o, v);
        if !jp(x(v), rd::<f32>(this, o + 0x50), 0x05) {
            wr::<u32>(this, o, rd::<u32>(this, o + 0x50));
        } else if !x(v).fcom(x(rd::<f32>(this, o + 0x40))).test_any(0x41) {
            wr::<u32>(this, o, rd::<u32>(this, o + 0x40));
        }
        wr::<u32>(this, 0x7D4 + 4 * k, 0x3F80_0000);
    }
}

/// Hook of [`monster_truck_extend_suspension_port`].
#[hook(addr = 0x6C7D80, conv = thiscall, name = "CMonsterTruck::ExtendSuspension", this = "CMonsterTruck",
       expect = "51 56 8D 91 38 08 00 00",
       unverified = "veh3-1", route = "L2", hazards = "none")]
pub fn monster_truck_extend_suspension(this: GPtr<u8>) {
    monster_truck_extend_suspension_port(this);
}

/// `CMonsterTruck::ProcessControlCollisionCheck(bool)` (0x6C8330): ExtendSuspension, the automobile
/// version, then per wheel a ratio +0x7D4 below 1.0 becomes `(+0x878 - +0x838) / (+0x878 - +0x888)`,
/// else 1.0.
#[hook(addr = 0x6C8330, conv = thiscall, name = "CMonsterTruck::ProcessControlCollisionCheck", this = "CMonsterTruck",
       expect = "56 8B F1 E8 48 FA FF FF",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes",
       note = "the automobile collision check processes the world's collisions")]
pub fn monster_truck_process_control_collision_check(this: GPtr<u8>, apply_speed: u32) {
    call_extend_suspension(this);
    automobile_pccc(this, apply_speed);
    for k in 0..4isize {
        let r = 0x7D4 + 4 * k;
        if jp(x(rd::<f32>(this, r)), 1.0, 0x05) {
            wr::<u32>(this, r, 0x3F80_0000);
        } else {
            let o = 0x878 + 4 * k;
            let a = x(rd::<f32>(this, o)).fsub_m(rd::<f32>(this, o - 0x40));
            let d = x(rd::<f32>(this, o)).fsub_m(rd::<f32>(this, o + 0x10));
            wr::<f32>(this, r, a.fdiv(d).to_f32());
        }
    }
}

/// `atan2(z, ±sqrt(a^2 + b^2))` of the matrix rows: the sign is negative when up.z (+0x28) < 0.
fn tilt(this: GPtr<u8>, a: isize, c: isize, z: isize) -> X87 {
    let m = rd::<GPtr<u8>>(this, 0x14);
    let fa = x(rd::<f32>(m, a));
    let fc = x(rd::<f32>(m, c));
    let mut s = fa.fmul(fa).fadd(fc.fmul(fc)).fsqrt();
    if !jp(x(rd::<f32>(m, 0x28)), 0.0, 0x05) {
        s = s.fmul_m(-1.0);
    }
    x(rd::<f32>(m, z)).fpatan(s)
}

/// `CAutomobile::GetCarPitch()` (0x6A6050): `atan2(forward.z, ±|forward.xy|)` in radians (ST0),
/// negative horizontal length when the car is upside down (up.z < 0).
#[hook(addr = 0x6A6050, conv = thiscall, name = "CAutomobile::GetCarPitch", this = "CAutomobile",
       expect = "8B 49 14 D9 41 14",
       unverified = "veh3-1", route = "L2", hazards = "x87-transcendental")]
pub fn automobile_get_car_pitch(this: GPtr<u8>) -> X87 {
    tilt(this, 0x10, 0x14, 0x18)
}

/// `CAutomobile::GetCarRoll()` (0x6A6010): `atan2(right.z, ±|right.xy|) * 57.295776` in degrees
/// (ST0).
#[hook(addr = 0x6A6010, conv = thiscall, name = "CAutomobile::GetCarRoll", this = "CAutomobile",
       expect = "8B 49 14 D9 41 04",
       unverified = "veh3-1", route = "L2", hazards = "x87-transcendental")]
pub fn automobile_get_car_roll(this: GPtr<u8>) -> X87 {
    tilt(this, 0x0, 0x4, 0x8).fmul_m(f32::from_bits(0x4265_2EE0))
}

/// `0x6C4130` (`CPlaceable`, used by the heli/plane generation): the matrix of a heading, right =
/// (sin, -cos, 0), forward = (cos, sin, 0), up.z = 1 (up.xy and the z of right/forward = 0; the
/// position untouched).
#[hook(addr = 0x6C4130, conv = thiscall, name = "FUN_006c4130", this = "CPlaceable",
       expect = "D9 44 24 04 8B 41 14",
       unverified = "veh3-1", route = "L2", hazards = "x87-transcendental")]
pub fn set_heading_matrix(this: GPtr<u8>, heading: f32) {
    let m = rd::<GPtr<u8>>(this, 0x14);
    let s = x(heading).fsin();
    for o in [0x8, 0x18, 0x20, 0x24] {
        wr::<u32>(m, o, 0);
    }
    wr::<u32>(m, 0x28, 0x3F80_0000);
    wr::<f32>(m, 0x0, s.to_f32());
    let c = x(heading).fcos();
    wr::<f32>(m, 0x4, c.fchs().to_f32());
    wr::<f32>(m, 0x10, c.to_f32());
    wr::<f32>(m, 0x14, s.to_f32());
}

/// `CAutomobile::IsInAir()` (0x6A6140): true with physical flag 0x2000 set; false with flag 0x100;
/// false when any wheel ratio (+0x7D4..) is below 1.0; else true when any move-speed component is
/// non-zero (or NaN).
#[hook(addr = 0x6A6140, conv = thiscall, name = "CAutomobile::IsInAir", this = "CAutomobile",
       expect = "8B 41 40 F6 C4 20",
       unverified = "veh3-1", route = "L2", hazards = "none")]
pub fn automobile_is_in_air(this: GPtr<u8>) -> u8 {
    let f = rd::<u32>(this, 0x40);
    if f & 0x2000 != 0 {
        return 1;
    }
    if f & 0x100 != 0 {
        return 0;
    }
    for k in 0..4isize {
        // jnp: exactly one of C0/C2 = below 1.0
        if !jp(x(rd::<f32>(this, 0x7D4 + 4 * k)), 1.0, 0x05) {
            return 0;
        }
    }
    if jp(x(rd::<f32>(this, 0x44)), 0.0, 0x44) || jp(x(rd::<f32>(this, 0x48)), 0.0, 0x44) {
        return 1;
    }
    b(jp(x(rd::<f32>(this, 0x4C)), 0.0, 0x44))
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_AUTOMOBILE_SETUP_MODEL_NODES,
    HOOK_BOAT_SETUP_MODEL_NODES,
    HOOK_AUTOMOBILE_SET_MODEL_INDEX,
    HOOK_BIKE_SET_MODEL_INDEX,
    HOOK_BOAT_SET_MODEL_INDEX,
    HOOK_TRAIN_SET_MODEL_INDEX,
    HOOK_BOAT_GET_COMPONENT_WORLD_POSITION,
    HOOK_BIKE_GET_COMPONENT_WORLD_POSITION,
    HOOK_MONSTER_TRUCK_RESET_SUSPENSION,
    HOOK_BIKE_RESET_SUSPENSION,
    HOOK_TRAILER_RESET_SUSPENSION,
    HOOK_MONSTER_TRUCK_EXTEND_SUSPENSION,
    HOOK_MONSTER_TRUCK_PROCESS_CONTROL_COLLISION_CHECK,
    HOOK_AUTOMOBILE_GET_CAR_PITCH,
    HOOK_AUTOMOBILE_GET_CAR_ROLL,
    HOOK_SET_HEADING_MATRIX,
    HOOK_AUTOMOBILE_IS_IN_AIR,
];
