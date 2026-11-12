//! STD-15 (batch `std15-ai-1`): `CTaskUtilityLineUpPedWithCar::GetLocalPositionToOpenCarDoor` (0x64FC10) and
//! `CTaskUtilityLineUpPedWithCar::ProcessPed` (0x6513A0), both unreversed in gta-reversed. Ported from the 1.0 US
//! bytes, the x87 order and the f32 temporaries of the original kept. Unverified (docs/verify/HOWTO.md).
//!
//! Layout used (0x1C bytes): +0 the offset (x, y, z), +0xC the door-open position z, +0x10 the end time (ms),
//! +0x14 the position type (0 slide, 1 / 2 keep the ped's own z, 2 the ped's own position), +0x18 the door.
//! Anim association: +0x14 the hierarchy (+0x10 total length), +0x18 the blend amount, +0x20 the current time,
//! +0x2C the anim id. Vehicle: +0x384 the handling (+0xD4 the door offset scale, +0xDE the anim group), +0x429
//! bit 0, +0x590 the type (5 boat, 9 bike), +0x594 the subclass.

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `FUN_0064e4b0(int32 animGroup, int32 animId)` (0x64E4B0, cdecl, ST0): the group's critical blend time
    fn veh_anim_critical_blend_time(group: u32, anim: u32) -> f64 = cdecl 0x64E4B0 => unported;
    /// `CVehicleAnimGroup::GetGroup(int32)` (0x6E3B00, `ret 4`)
    fn veh_anim_group_get_group(g: GPtr<u8>, anim: i32) -> i32 = thiscall 0x6E3B00 => unported;
    /// `CVehicleAnimGroup::ComputeCriticalBlendTime(int32)` (0x6E3C80, `ret 4`, ST0)
    fn veh_anim_group_critical_blend_time(g: GPtr<u8>, anim: i32) -> f64 = thiscall 0x6E3C80 => unported;
    /// `CVehicleAnimGroup::ComputeAnimDoorOffsets(CVector* ret, int32 door)` (0x6E3D10, `ret 8`)
    fn veh_anim_group_door_offsets(g: GPtr<u8>, ret: GPtr<u8>, door: u32) -> GPtr<u8> = thiscall 0x6E3D10 => unported;
    /// `CTaskUtilityLineUpPedWithCar::GetPositionToOpenCarDoor(CVector* out, CVehicle*, float, assoc)` (0x650A80)
    fn line_up_get_position_to_open_car_door(u: GPtr<u8>, out: GPtr<u8>, veh: GPtr<u8>, t: u32, a: GPtr<u8>) -> GPtr<u8> = thiscall 0x650A80 => unported;
    /// `CBike::GetCorrectedWorldDoorPosition(CVector& out, CVector a, CVector b)` (0x6BF230, `ret 0x1C`)
    fn bike_get_corrected_world_door_position(b: GPtr<u8>, out: GPtr<u8>, a0: u32, a1: u32, a2: u32, b0: u32, b1: u32, b2: u32) -> GPtr<u8> = thiscall 0x6BF230 => unported;
    /// `Multiply3x3(CVector* out, const CVector&, const CMatrix&)` (0x59C810, cdecl): the transposed product
    fn multiply3x3_transposed(out: GPtr<u8>, v: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C810 => unported;
    /// `Multiply3x3(CVector* out, const CMatrix&, const CVector&)` (0x59C790, cdecl)
    fn multiply3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => unported;
    /// `CPedPlacement::FindZCoorForPed(CVector*)` (0x616920, cdecl)
    fn find_z_coor_for_ped(pos: GPtr<u8>) -> u8 = cdecl 0x616920 => unported;
    /// `CVehicle::IsUpsideDown()` (0x6D1D90)
    fn vehicle_is_upside_down(v: GPtr<u8>) -> u8 = thiscall 0x6D1D90 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    fn limit_radian_angle(a: u32) -> f64 = cdecl 0x53CB50 => unported;
    /// the vehicle's virtual at +0xD4 (ST0; the distance its base sits below the position)
    fn v_vehicle_d4(v: GPtr<u8>) -> f64 = thiscall vslot 0xD4 => unported;
    /// `CMatrix::CMatrix(const CMatrix&)` (0x59BCF0, `ret 4`)
    fn matrix_copy_ctor(m: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x59BCF0 => unported;
    /// `CMatrix::~CMatrix` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CMatrix::SetRotateZ(float)` (0x59B0E0)
    fn matrix_set_rotate_z(m: GPtr<u8>, a: u32) = thiscall 0x59B0E0 => unported;
    /// `CMatrix::SetRotateZOnly(float)` (0x59B020)
    fn matrix_set_rotate_z_only(m: GPtr<u8>, a: u32) = thiscall 0x59B020 => unported;
    /// `CMatrix::operator*=(const CMatrix&)` (0x411A80)
    fn matrix_mul_assign(m: GPtr<u8>, o: GPtr<u8>) -> GPtr<u8> = thiscall 0x411A80 => unported;
    /// `CMatrix::CopyToRwMatrix(RwMatrix*)` (0x59B8B0)
    fn matrix_copy_to_rw(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59B8B0 => unported;
    /// `CMatrix::UpdateMatrix(RwMatrix*)` (0x59AD20)
    fn matrix_update(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD20 => unported;
    /// `RtQuatConvertFromMatrix(RtQuat*, const RwMatrix*)` (0x7EB5C0, cdecl)
    fn rt_quat_convert_from_matrix(q: GPtr<u8>, m: GPtr<u8>) -> u32 = cdecl 0x7EB5C0 => unported;
    /// `RtQuatSetupSlerpCache(const RtQuat* from, const RtQuat* to, RtQuatSlerpCache*)` (0x7EC220, cdecl)
    fn rt_quat_setup_slerp_cache(from: GPtr<u8>, to: GPtr<u8>, cache: GPtr<u8>) -> u32 = cdecl 0x7EC220 => unported;
    /// `CPlaceable::SetMatrix(const CMatrix&)` (0x54F610)
    fn placeable_set_matrix(e: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F610 => unported;
    /// `CPlaceable::SetOrientation(float x, float y, float z)` (0x439A80)
    fn placeable_set_orientation(e: GPtr<u8>, ax: u32, ay: u32, az: u32) = thiscall 0x439A80 => unported;
}

const TIME_MS: usize = 0xB7CB84;
const TIME_STEP: usize = 0xB7CB5C;
const MODEL_INFOS: usize = 0xA9B0C8;
const VEH_ANIM_GROUPS: usize = 0xC1CDC0;
const F_ONE: usize = 0x858624;
const F_ZERO: usize = 0x858B50;
const F_PI: usize = 0x858CB8;
const F_TWO_PI: usize = 0x858CBC;

fn gf(a: usize) -> f32 {
    g::<f32>(a).read()
}

/// The entity's position: its matrix's +0x30 when it has one, else the placeable's +4.
fn pos_of(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

fn anim_group(veh: GPtr<u8>) -> u32 {
    u32::from(rd::<u8>(rd::<GPtr<u8>>(veh, 0x384), 0xDE))
}

fn anim_group_ptr(group: u32) -> GPtr<u8> {
    GPtr::from_addr((group as usize).wrapping_mul(0x94).wrapping_add(VEH_ANIM_GROUPS))
}

/// `assoc->m_fCurrentTime / assoc->m_pHierarchy->m_fTotalLength` (ST0, unrounded).
fn anim_progress(a: GPtr<u8>) -> X87 {
    x(rd::<f32>(a, 0x20)).fdiv_m(rd::<f32>(rd::<GPtr<u8>>(a, 0x14), 0x10))
}

/// `CTaskUtilityLineUpPedWithCar::GetLocalPositionToOpenCarDoor(CVector* out, CVehicle*, float, assoc)`
/// (0x64FC10, thiscall, `ret 0x10`, returns `out`). The door offset scale (handling +0xD4) scales the time
/// argument unless vehicle bit 0x429.0 is set for doors 9 / 11 (then 0). The anim picks the offset kind passed
/// to `ComputeAnimDoorOffsets`: 0 without an anim and for 0x15F..0x174, 0x185, 0x186; 3 for 0x6C, 0x6D, 0x175..0x178,
/// 0x17C..0x17F, 0x183, 0x184; 5 for 0x17A, 0x17B; the other anims leave the variable uninitialised: the
/// original reads the vehicle argument's slot, so the vehicle pointer is passed as the kind. Kinds 0 and 3 become
/// 0 (doors 8, 10), 1 (doors 9, 11) or 2 (door 18); kind 5 becomes 0. Anims 0x185 / 0x186 of a group with flag
/// bit 5 blend the vehicle group's offsets with group 0's by the anim's blend amount. The door then picks the
/// model's seat position (the vehicle struct +0x30, or +0 for model type 5; +0x3C for doors 9 and 11) and the
/// offset is added with the signs of each side; other doors take the seat position as is. Bikes (type 9), except
/// door 18, pass both through `CBike::GetCorrectedWorldDoorPosition` and back into the vehicle's frame.
#[hook(addr = 0x64FC10, conv = thiscall, name = "CTaskUtilityLineUpPedWithCar::GetLocalPositionToOpenCarDoor",
       expect = "83 EC 3C 53 55 8B E9", unverified = "std15-ai-1", route = "L2", hazards = "uninit-locals",
       recs = "car-enter",
       note = "anims outside the kind table pass the vehicle pointer as the offset kind (an uninitialised local in the original)")]
pub fn line_up_get_local_position_to_open_car_door(
    this: GPtr<u8>,
    out: GPtr<u8>,
    veh: GPtr<u8>,
    t: u32,
    assoc: GPtr<u8>,
) -> GPtr<u8> {
    let mi: GPtr<u8> =
        GPtr::from_addr(g::<u32>(MODEL_INFOS + rd::<i16>(veh, 0x22) as isize as usize * 4).read() as usize);
    let door = rd::<i32>(this, 0x18);
    let tf = if rd::<u8>(veh, 0x429) & 1 != 0 && (door == 0xB || door == 9) {
        0.0f32
    } else {
        x(f32::from_bits(t))
            .fmul_m(rd::<f32>(rd::<GPtr<u8>>(veh, 0x384), 0xD4))
            .to_f32()
    };
    let id = if assoc.is_null() {
        None
    } else {
        Some(i32::from(rd::<i16>(assoc, 0x2C)))
    };
    let garbage = veh.addr() as u32;
    let mut kind: u32 = match id {
        None => 0,
        Some(id) => match id {
            0x6C | 0x6D => 3,
            0x15F..=0x174 => 0,
            0x175..=0x178 => 3,
            0x17A | 0x17B => 5,
            0x17C..=0x17F | 0x183 | 0x184 => 3,
            0x185 | 0x186 => 0,
            _ => garbage,
        },
    };
    if kind == 0 || kind == 3 {
        match door {
            0xA | 8 => kind = 0,
            0xB | 9 => kind = 1,
            0x12 => kind = 2,
            _ => {}
        }
    } else if kind == 5 {
        kind = 0;
    }
    // (L1C, ebx, L24): the anim door offset
    let mut buf34 = [0f32; 3];
    let p34: GPtr<u8> = GPtr::from_raw(buf34.as_mut_ptr()).cast();
    let mut buf40 = [0f32; 3];
    let p40: GPtr<u8> = GPtr::from_raw(buf40.as_mut_ptr()).cast();
    let group = anim_group(veh);
    let blended = matches!(id, Some(0x185 | 0x186)) && rd::<u32>(anim_group_ptr(group), 8) >> 5 & 1 != 0;
    let (mut o1c, mut obx, mut o24);
    if blended {
        veh_anim_group_door_offsets(anim_group_ptr(group), p34, kind);
        let b = rd::<f32>(assoc, 0x18);
        let inv = x(gf(F_ONE)).fsub_m(b);
        let l1c = x(rd::<f32>(p34, 0)).fmul(inv).to_f32();
        let l20 = x(rd::<f32>(p34, 4)).fmul(inv).to_f32();
        let l24 = x(rd::<f32>(p34, 8)).fmul(inv).to_f32();
        veh_anim_group_door_offsets(GPtr::from_addr(VEH_ANIM_GROUPS), p34, kind);
        let bx = x(b);
        o1c = x(rd::<f32>(p34, 0)).fmul(bx).fadd_m(l1c).to_f32();
        let l44 = x(rd::<f32>(p34, 4)).fmul(bx).to_f32();
        let l48 = x(rd::<f32>(p34, 8)).fmul(bx).to_f32();
        obx = x(l44).fadd_m(l20).to_f32();
        o24 = x(l48).fadd_m(l24).to_f32();
    } else {
        veh_anim_group_door_offsets(anim_group_ptr(group), p40, kind);
        o1c = rd::<f32>(p40, 0);
        obx = rd::<f32>(p40, 4);
        o24 = rd::<f32>(p40, 8);
    }
    let vs = rd::<GPtr<u8>>(mi, 0x5C);
    let seat_front = || {
        if rd::<i32>(mi, 0x3C) == 5 {
            vs
        } else {
            vs.byte_add(0x30)
        }
    };
    let seat_rear = || vs.byte_add(0x3C);
    // (L10, eax, edi): the seat position as the bike path passes it; (l34, l38, z): the result
    let (l10, py, pz, l34, l38, z): (f32, f32, f32, f32, f32, X87);
    match door {
        10 | 11 => {
            let p = if door == 10 { seat_front() } else { seat_rear() };
            let s = x(rd::<f32>(p, 0)).fadd_m(tf).fchs();
            l10 = s.to_f32();
            py = rd::<f32>(p, 4);
            pz = rd::<f32>(p, 8);
            l34 = s.fsub_m(o1c).to_f32();
            l38 = x(py).fsub_m(obx).to_f32();
            z = x(pz).fsub_m(o24);
        }
        8 | 9 => {
            let p = if door == 8 { seat_front() } else { seat_rear() };
            l10 = x(rd::<f32>(p, 0)).fadd_m(tf).to_f32();
            o1c = x(o1c).fchs().to_f32();
            py = rd::<f32>(p, 4);
            pz = rd::<f32>(p, 8);
            l34 = x(l10).fsub_m(o1c).to_f32();
            l38 = x(py).fsub_m(obx).to_f32();
            z = x(pz).fsub_m(o24);
        }
        0x12 => {
            let p = seat_front();
            l10 = rd::<f32>(p, 0);
            py = rd::<f32>(p, 4);
            pz = rd::<f32>(p, 8);
            l34 = x(o1c).fadd_m(l10).to_f32();
            l38 = x(py).fadd_m(obx).to_f32();
            z = x(pz).fadd_m(o24);
        }
        _ => {
            let p = seat_front();
            l10 = rd::<f32>(p, 0);
            py = rd::<f32>(p, 4);
            pz = rd::<f32>(p, 8);
            o1c = 0.0;
            obx = 0.0;
            o24 = 0.0;
            l34 = l10;
            l38 = py;
            z = x(pz);
        }
    }
    let mut r = [l34.to_bits(), l38.to_bits(), z.to_f32().to_bits()];
    if rd::<i32>(veh, 0x590) == 9 && door != 0x12 {
        let mut l28 = [0f32; 3];
        let p28: GPtr<u8> = GPtr::from_raw(l28.as_mut_ptr()).cast();
        bike_get_corrected_world_door_position(
            veh,
            p28,
            o1c.to_bits(),
            obx.to_bits(),
            o24.to_bits(),
            l10.to_bits(),
            py.to_bits(),
            pz.to_bits(),
        );
        let pos = pos_of(veh);
        for i in 0..3isize {
            let v = x(rd::<f32>(p28, i * 4)).fsub_m(rd::<f32>(pos, i * 4)).to_f32();
            wr::<f32>(p28, i * 4, v);
        }
        let q = multiply3x3_transposed(p40, p28, rd::<GPtr<u8>>(veh, 0x14));
        r = [rd::<u32>(q, 0), rd::<u32>(q, 4), rd::<u32>(q, 8)];
    }
    wr::<u32>(out, 0, r[0]);
    wr::<u32>(out, 4, r[1]);
    wr::<u32>(out, 8, r[2]);
    out
}

/// The vehicle heading from its matrix's forward vector (`atan2(-fwd.x, fwd.y)`), else the placeable's +0x10.
fn vehicle_heading(veh: GPtr<u8>, m: GPtr<u8>) -> X87 {
    if m.is_null() {
        x(rd::<f32>(veh, 0x10))
    } else {
        x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14)))
    }
}

/// The inlined `sin` polynomial of the slerp: `a + a³ (((((a² C7 - C6) a² + C5) a² - C4) a² + C3) a² - C2)`.
fn poly_sin(a: X87) -> X87 {
    let a2 = a.fmul(a);
    let a3 = a2.fmul(a);
    let p = a2
        .fmul_m(gf(0x86D2B0))
        .fsub_m(gf(0x86D2AC))
        .fmul(a2)
        .fadd_m(gf(0x86D2A8))
        .fmul(a2)
        .fsub_m(gf(0x86D2A4))
        .fmul(a2)
        .fadd_m(gf(0x864E0C))
        .fmul(a2)
        .fsub_m(gf(0x85F0A0))
        .fmul(a3);
    a.fadd(p)
}

/// The inlined `RtQuatSlerp` with the cache at `cache` (from +0, to +0x10, omega +0x20, flag +0x24) and the
/// progress `t` (f32): `t <= 0` gives `from`, `t >= 1` gives `to`; else `from * f0 + to * f1` with
/// `f0 = sin((1 - t) omega)`, `f1 = sin(t omega)` by the polynomial, or `1 - t`, `t` with the nearly-zero flag.
fn slerp(from: GPtr<u8>, to: GPtr<u8>, cache: GPtr<u8>, t: f32) -> [f32; 4] {
    let q = |p: GPtr<u8>| {
        [
            rd::<f32>(p, 0),
            rd::<f32>(p, 4),
            rd::<f32>(p, 8),
            rd::<f32>(p, 0xC),
        ]
    };
    if !x(t).fcom(x(gf(F_ZERO))).test_parity(0x41) {
        return q(from);
    }
    if !x(gf(F_ONE)).fcom(x(t)).test_parity(0x41) {
        return q(to);
    }
    let inv = x(gf(F_ONE)).fsub_m(t);
    let (f1, f0) = if rd::<u32>(cache, 0x24) != 0 {
        (x(t), inv)
    } else {
        let s0 = poly_sin(inv.fmul_m(rd::<f32>(cache, 0x20)));
        let s1 = poly_sin(x(rd::<f32>(cache, 0x20)).fmul_m(t));
        (s1, s0)
    };
    let c = |o: isize| rd::<f32>(cache, o);
    let rx = x(c(0)).fmul(f0);
    let ry = x(c(4)).fmul(f0);
    let rz = x(c(8)).fmul(f0).to_f32();
    let qx = x(c(0x10)).fmul(f1).fadd(rx).to_f32();
    let qy = x(c(0x14)).fmul(f1).fadd(ry).to_f32();
    let qz = x(c(0x18)).fmul(f1).fadd_m(rz).to_f32();
    let qw = x(c(0x1C)).fmul(f1).fadd(x(c(0xC)).fmul(f0)).to_f32();
    [qx, qy, qz, qw]
}

/// The inlined quaternion-to-RwMatrix conversion (both bike branches compute the same terms in the same order):
/// right, flags 3, up, at, a zero position.
fn quat_to_rw_matrix(q: [f32; 4], rw: GPtr<u8>) {
    let [qx, qy, qz, qw] = q;
    let n = x(qy)
        .fmul_m(qy)
        .fadd(x(qx).fmul_m(qx))
        .fadd(x(qz).fmul_m(qz))
        .fadd(x(qw).fmul_m(qw));
    let s = n.fdivr_m(gf(0x858CA0));
    let xs = x(qx).fmul(s);
    let ys = s.fmul_m(qy).to_f32();
    let zs = s.fmul_m(qz);
    let wx = xs.fmul_m(qw).to_f32();
    let wy = x(ys).fmul_m(qw).to_f32();
    let wz = x(qw).fmul(zs).to_f32();
    let xx = xs.fmul_m(qx).to_f32();
    let yy = x(ys).fmul_m(qy).to_f32();
    let zz = x(qz).fmul(zs).to_f32();
    let yz = zs.fmul_m(qy).to_f32();
    let xz = xs.fmul_m(qz);
    let xy = x(ys).fmul_m(qx);
    let one = gf(F_ONE);
    let w = |o: isize, v: X87| wr::<f32>(rw, o, v.to_f32());
    w(0, x(zz).fadd_m(yy).fsubr_m(one));
    w(4, x(wz).fadd(xy));
    w(8, xz.fsub_m(wy));
    wr::<u32>(rw, 0xC, 3);
    w(0x10, xy.fsub_m(wz));
    w(0x14, x(zz).fadd_m(xx).fsubr_m(one));
    w(0x18, x(yz).fadd_m(wx));
    w(0x20, xz.fadd_m(wy));
    w(0x24, x(yz).fsub_m(wx));
    w(0x28, x(yy).fadd_m(xx).fsubr_m(one));
    wr::<u32>(rw, 0x30, 0);
    wr::<u32>(rw, 0x34, 0);
    wr::<u32>(rw, 0x38, 0);
}

/// `CTaskUtilityLineUpPedWithCar::ProcessPed(CPed*, CVehicle*, CAnimBlendAssociation*)` (0x6513A0, thiscall,
/// `ret 0xC`, always returns false). Position type 0 stops the ped (move speed 0). The ped's aimed heading
/// (+0x55C) follows the vehicle's (an upside-down vehicle, up.z <= -0.8, turns it by -pi for doors 8 / 9; door 18
/// by +pi; type 2 keeps it). The anim gives the door-position progress L3C and the blend L14 (the anim tables
/// 0x6527B0 / 0x6528BC / 0x6528D0, the critical blend times, model 0x241 forced to 1). The target T is the world
/// door position at L3C (or the ped's own position for type 2), Q the one at 1.0 (non-bikes); Q's z follows the
/// vehicle's right vector and `FindZCoorForPed` unless the vehicle has flag 0x100 (boats upside down get +1).
/// +0xC = Q.z. Types 1 / 2 keep the ped's own z when it stays above Q.z after this frame's gravity. The z
/// blends towards Q.z (bikes by their own anim factors). Before the end time the offset (+0) is faded out over
/// 600 ms and the current heading (+0x558) turned towards the limited aimed heading. Bike anims 0x167..0x16B and
/// 0x175..0x179 slerp the ped's orientation between its own and the vehicle's (`RtQuatSetupSlerpCache` and an
/// inlined slerp and quaternion-to-matrix) and set the matrix with T; otherwise, with L3C <= 0.2 on an upright
/// non-bike of subclass != 2, the ped's matrix becomes the vehicle's moved by the rotated local door position;
/// else the ped is placed at T with the current heading.
#[hook(addr = 0x6513A0, conv = thiscall, name = "CTaskUtilityLineUpPedWithCar::ProcessPed",
       expect = "6A FF 68 F8 32 84 00 64", unverified = "std15-ai-1", route = "L2", hazards = "x87-transcendental",
       recs = "car-enter",
       note = "the vehicle matrix is read without a null check at entry, as the original; locals live in Rust buffers")]
pub fn line_up_ped_with_car_process_ped(this: GPtr<u8>, ped: GPtr<u8>, veh: GPtr<u8>, assoc: GPtr<u8>) -> u8 {
    let pos_type = || rd::<i32>(this, 0x14);
    if pos_type() == 0 {
        wr::<f32>(ped, 0x44, 0.0);
        wr::<f32>(ped, 0x48, 0.0);
        wr::<f32>(ped, 0x4C, 0.0);
    }
    let vm = rd::<GPtr<u8>>(veh, 0x14);
    let door = rd::<i32>(this, 0x18);
    let mut upside_down = false;
    if !x(rd::<f32>(vm, 0x28)).fcom(x(gf(0x858C9C))).test_parity(0x41) {
        upside_down = true;
        let h = if door == 8 || door == 9 {
            vehicle_heading(veh, vm).fsub_m(gf(F_PI))
        } else {
            vehicle_heading(veh, vm)
        };
        wr::<f32>(ped, 0x55C, h.to_f32());
    } else if door == 0x12 {
        wr::<f32>(ped, 0x55C, vehicle_heading(veh, vm).fadd_m(gf(F_PI)).to_f32());
    } else if pos_type() != 2 {
        wr::<f32>(ped, 0x55C, vehicle_heading(veh, vm).to_f32());
    }

    let mut l3c = 0.0f32;
    let mut l14 = 0.0f32;
    let id = if assoc.is_null() {
        -1
    } else {
        i32::from(rd::<i16>(assoc, 0x2C))
    };
    let model_241 = || rd::<i16>(veh, 0x22) == 0x241;
    // the shared progress-against-critical-time tail (0x65158x / 0x6516Bx): r is the critical time
    let ramp = |r: X87, l10: f32, prog: f32| -> f32 {
        let c1 = r.fcom(x(gf(F_ZERO)));
        let c2 = x(prog).fcom(x(l10));
        if !c1.test_any(0x41) {
            if c2.test_parity(0x05) {
                1.0
            } else {
                x(prog).fdiv_m(l10).to_f32()
            }
        } else if c2.test_parity(0x05) {
            x(prog).fsub_m(l10).fdiv(x(gf(F_ONE)).fsub_m(l10)).to_f32()
        } else {
            0.0
        }
    };
    if !assoc.is_null() {
        let group = anim_group(veh);
        match id {
            0x15F..=0x162 => {
                l3c = 1.0;
                let r = X87::from_f64_exact(veh_anim_critical_blend_time(group, id as u32));
                let l10 = r.fabs().to_f32();
                if r.fabs().fcom(x(gf(0x85862C))).test_parity(0x05) {
                    let r2 = r.fsub_m(gf(0x858FBC));
                    let l10b = x(l10).fsub_m(gf(0x858FBC)).to_f32();
                    let prog = anim_progress(assoc).to_f32();
                    l14 = ramp(r2, l10b, prog);
                } else {
                    l14 = 0.0;
                }
                if model_241() {
                    l14 = 1.0;
                }
            }
            0x163..=0x166 => {
                l3c = 1.0;
                let r = X87::from_f64_exact(veh_anim_critical_blend_time(group, id as u32));
                l14 = if r.fabs().fcom(x(gf(0x85862C))).test_parity(0x05) {
                    1.0
                } else {
                    0.0
                };
                if model_241() {
                    l14 = 1.0;
                }
            }
            0x6C | 0x6D => {
                l14 = 0.0;
                l3c = anim_progress(assoc).to_f32();
            }
            0x167..=0x16B => {
                l3c = anim_progress(assoc).fsubr_m(gf(F_ONE)).to_f32();
                let r = X87::from_f64_exact(veh_anim_critical_blend_time(group, id as u32));
                let l10 = r.fabs().to_f32();
                if r.fabs().fcom(x(gf(0x85862C))).test_parity(0x05) {
                    l14 = 1.0;
                } else {
                    let prog = anim_progress(assoc).to_f32();
                    l14 = ramp(r, l10, prog);
                }
                if model_241() {
                    l14 = 1.0;
                }
            }
            0x16C..=0x16E | 0x17C..=0x17F => {
                l3c = 1.0;
                l14 = 0.0;
            }
            0x16F..=0x174 | 0x182 => {
                l3c = 0.0;
                l14 = 1.0;
            }
            0x185 | 0x186 => {
                l3c = 1.0;
                l14 = if rd::<u32>(anim_group_ptr(group), 8) >> 5 & 1 != 0 {
                    1.0
                } else {
                    0.0
                };
            }
            0x175..=0x178 | 0x17A | 0x17B | 0x180 | 0x181 | 0x183 | 0x184 => {
                l3c = anim_progress(assoc).to_f32();
                let gp = anim_group_ptr(group);
                veh_anim_group_get_group(gp, id);
                let r = X87::from_f64_exact(veh_anim_group_critical_blend_time(gp, id));
                let l10 = r.fabs().to_f32();
                let prog = anim_progress(assoc).to_f32();
                let c1 = r.fcom(x(gf(F_ZERO)));
                let c2 = x(prog).fcom(x(l10));
                l14 = if !c1.test_any(0x41) {
                    if c2.test_parity(0x05) {
                        0.0
                    } else {
                        x(prog).fdiv_m(l10).fsubr_m(gf(F_ONE)).to_f32()
                    }
                } else if c2.test_parity(0x05) {
                    x(prog)
                        .fsub_m(l10)
                        .fdiv(x(gf(F_ONE)).fsub_m(l10))
                        .fsubr_m(gf(F_ONE))
                        .to_f32()
                } else {
                    1.0
                };
            }
            _ => {}
        }
    }

    // 0x651885: the target T (L40) and the door position at 1.0 (Q, L4C)
    let mut tmp = [0f32; 3];
    let tp: GPtr<u8> = GPtr::from_raw(tmp.as_mut_ptr()).cast();
    let t = if pos_type() == 2 {
        let p = pos_of(ped);
        [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)]
    } else {
        let r = line_up_get_position_to_open_car_door(this, tp, veh, l3c.to_bits(), assoc);
        [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)]
    };
    let mut p = t;
    let mut qv = t;
    if rd::<i32>(veh, 0x590) != 9 && pos_type() != 2 {
        let r = line_up_get_position_to_open_car_door(this, tp, veh, 1.0f32.to_bits(), assoc);
        qv = [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)];
    }
    let qp: GPtr<u8> = GPtr::from_raw(qv.as_mut_ptr()).cast();
    if rd::<u32>(veh, 0x40) & 0x100 == 0 {
        let m = rd::<GPtr<u8>>(veh, 0x14);
        let (rx, ry, rz): (f32, X87, X87);
        if m.is_null() {
            let h = rd::<f32>(veh, 0x10);
            rx = x(h).fcos().to_f32();
            ry = x(h).fsin();
            rz = x(gf(F_ZERO));
        } else {
            rx = rd::<f32>(m, 0);
            ry = x(rd::<f32>(m, 4));
            rz = x(rd::<f32>(m, 8));
        }
        let pos = pos_of(veh);
        let a = x(qv[2]).fsub_m(rd::<f32>(pos, 8));
        let b = x(qv[1]).fsub_m(rd::<f32>(pos, 4));
        let c = x(qv[0]).fsub_m(rd::<f32>(pos, 0));
        let dot = rz.fmul(a).fadd(ry.fmul(b)).fadd(c.fmul_m(rx)).to_f32();
        // the second read of the right vector: z from the matrix, else 0
        let rz2 = if m.is_null() { 0.0 } else { rd::<f32>(m, 8) };
        let d4 = X87::from_f64_exact(v_vehicle_d4(veh));
        let base = d4.fsubr_m(rd::<f32>(pos, 8));
        let l18 = base.fadd(x(rz2).fmul_m(dot)).fadd_m(gf(F_ONE)).to_f32();
        let old_z = rd::<f32>(qp, 8);
        find_z_coor_for_ped(qp);
        if !x(l18)
            .fsub_m(gf(0x858B8C))
            .fcom(x(rd::<f32>(qp, 8)))
            .test_any(0x41)
        {
            wr::<f32>(qp, 8, old_z);
        }
    } else if rd::<i32>(veh, 0x590) == 5 && vehicle_is_upside_down(veh) != 0 {
        let z = x(rd::<f32>(qp, 8)).fadd_m(gf(F_ONE)).to_f32();
        wr::<f32>(qp, 8, z);
    }
    let qz = rd::<f32>(qp, 8);
    wr::<f32>(this, 0xC, qz);
    if pos_type() == 1 || pos_type() == 2 {
        let pp = pos_of(ped);
        let l18 = x(gf(TIME_STEP))
            .fmul_m(gf(0x863984))
            .fsubr_m(rd::<f32>(ped, 0x4C));
        let sum = l18.fadd_m(rd::<f32>(pp, 8));
        if sum.fcom(x(qz)).test_parity(0x05) {
            wr::<f32>(ped, 0x4C, l18.to_f32());
            p[2] = rd::<f32>(pos_of(ped), 8);
        } else {
            p[2] = qz;
            wr::<f32>(ped, 0x44, 0.0);
            wr::<f32>(ped, 0x48, 0.0);
            wr::<f32>(ped, 0x4C, 0.0);
        }
    }
    let oz = rd::<f32>(this, 0xC);
    let above = x(oz).fcom(x(p[2])).test_any(0x41);
    let blend_z = |p2: f32| x(p2).fsub_m(oz).fmul_m(l14).fadd_m(oz).to_f32();
    if above {
        if pos_type() == 0 {
            p[2] = blend_z(p[2]);
        }
    } else if rd::<i32>(veh, 0x590) != 9 || assoc.is_null() || id == 0x16B {
        p[2] = blend_z(p[2]);
    } else {
        let f = match id {
            0x175..=0x178 => x(gf(F_ONE)).fsub_m(l3c).to_f32(),
            0x167..=0x16A => {
                let p2 = anim_progress(assoc).fdouble();
                if x(gf(F_ONE)).fcom(p2).test_parity(0x05) {
                    p2.to_f32()
                } else {
                    1.0
                }
            }
            _ => 0.0,
        };
        let pos = pos_of(veh);
        let d4 = X87::from_f64_exact(v_vehicle_d4(veh));
        p[2] = d4
            .fsubr_m(rd::<f32>(pos, 8))
            .fadd_m(gf(F_ONE))
            .fsub_m(oz)
            .fmul_m(f)
            .fadd_m(oz)
            .to_f32();
    }

    // 0x651BEA: the offset fade and the heading turn
    let now = g::<u32>(TIME_MS).read();
    if now < rd::<u32>(this, 0x10) {
        let lim = X87::from_f64_exact(limit_radian_angle(rd::<u32>(ped, 0x55C)));
        let rem = rd::<u32>(this, 0x10).wrapping_sub(now);
        let cur = rd::<f32>(ped, 0x558);
        let mut k = xi(rem as i32);
        if (rem as i32) < 0 {
            k = k.fadd_m(gf(0x858C54));
        }
        k = k.fmul_m(gf(0x865060));
        if !k.fcom(x(gf(F_ZERO))).test_parity(0x41) {
            wr::<f32>(this, 0, 0.0);
            wr::<f32>(this, 4, 0.0);
        }
        wr::<f32>(this, 8, 0.0);
        let zk = x(gf(F_ZERO)).fmul(k);
        let l1c = k.fmul_m(rd::<f32>(this, 4)).to_f32();
        let l5c = k.fmul_m(rd::<f32>(this, 0)).to_f32();
        let nz = zk.fsubr_m(p[2]);
        let ny = x(p[1]).fsub_m(l1c);
        p[0] = x(p[0]).fsub_m(l5c).to_f32();
        p[1] = ny.to_f32();
        p[2] = nz.to_f32();
        let mut lim = lim;
        if !x(cur).fadd_m(gf(F_PI)).fcom(lim).test_parity(0x05) {
            lim = lim.fsub_m(gf(F_TWO_PI));
        } else if !x(cur).fsub_m(gf(F_PI)).fcom(lim).test_any(0x41) {
            lim = lim.fadd_m(gf(F_TWO_PI));
        }
        let diff = x(cur).fsub(lim);
        let d = diff.fmul(x(gf(F_ONE)).fsub(k));
        wr::<f32>(ped, 0x558, x(cur).fsub(d).to_f32());
    } else {
        wr::<u32>(ped, 0x558, rd::<u32>(ped, 0x55C));
    }

    // 0x651D15: the bike slerp branches
    let bike1 = matches!(id, 0x167..=0x16B) && !assoc.is_null();
    let bike2 = matches!(id, 0x175..=0x179) && !assoc.is_null();
    if bike1 || bike2 {
        let mut m1 = [0u32; 0x12];
        let m1p: GPtr<u8> = GPtr::from_raw(m1.as_mut_ptr()).cast();
        let mut m3 = [0u32; 0x12];
        let m3p: GPtr<u8> = GPtr::from_raw(m3.as_mut_ptr()).cast();
        let mut m4 = [0u32; 0x12];
        let m4p: GPtr<u8> = GPtr::from_raw(m4.as_mut_ptr()).cast();
        let mut r1 = [0u32; 0x10];
        let r1p: GPtr<u8> = GPtr::from_raw(r1.as_mut_ptr()).cast();
        let mut r2 = [0u32; 0x10];
        let r2p: GPtr<u8> = GPtr::from_raw(r2.as_mut_ptr()).cast();
        let mut r3 = [0u32; 0x10];
        let r3p: GPtr<u8> = GPtr::from_raw(r3.as_mut_ptr()).cast();
        let mut q78 = [0u32; 4];
        let q78p: GPtr<u8> = GPtr::from_raw(q78.as_mut_ptr()).cast();
        let mut q68 = [0u32; 4];
        let q68p: GPtr<u8> = GPtr::from_raw(q68.as_mut_ptr()).cast();
        let mut cache = [0u32; 10];
        let cp: GPtr<u8> = GPtr::from_raw(cache.as_mut_ptr()).cast();
        let q;
        if bike1 {
            matrix_copy_ctor(m1p, rd::<GPtr<u8>>(veh, 0x14));
            if id == 0x16B {
                let mut m2 = [0u32; 0x12];
                let m2p: GPtr<u8> = GPtr::from_raw(m2.as_mut_ptr()).cast();
                matrix_set_rotate_z(m2p, 0x4049_0FDB);
                matrix_mul_assign(m1p, m2p);
                matrix_dtor(m2p);
            }
            matrix_copy_to_rw(m1p, r1p);
            rt_quat_convert_from_matrix(q78p, r1p);
            matrix_copy_ctor(m3p, rd::<GPtr<u8>>(ped, 0x14));
            matrix_copy_to_rw(m3p, r2p);
            rt_quat_convert_from_matrix(q68p, r2p);
            let l10 = anim_progress(assoc).to_f32();
            rt_quat_setup_slerp_cache(q68p, q78p, cp);
            q = slerp(q68p, q78p, cp, l10);
        } else {
            matrix_copy_ctor(m3p, rd::<GPtr<u8>>(veh, 0x14));
            matrix_copy_to_rw(m3p, r2p);
            rt_quat_convert_from_matrix(q78p, r2p);
            matrix_copy_ctor(m1p, rd::<GPtr<u8>>(ped, 0x14));
            matrix_set_rotate_z_only(m1p, rd::<u32>(ped, 0x558));
            matrix_copy_to_rw(m1p, r1p);
            rt_quat_convert_from_matrix(q68p, r1p);
            let l10 = anim_progress(assoc).to_f32();
            rt_quat_setup_slerp_cache(q78p, q68p, cp);
            q = slerp(q78p, q68p, cp, l10);
        }
        quat_to_rw_matrix(q, r3p);
        matrix_update(m4p, r3p);
        wr::<f32>(m4p, 0x30, p[0]);
        wr::<f32>(m4p, 0x34, p[1]);
        wr::<f32>(m4p, 0x38, p[2]);
        placeable_set_matrix(ped, m4p);
        matrix_dtor(m4p);
        if bike1 {
            matrix_dtor(m3p);
            matrix_dtor(m1p);
        } else {
            matrix_dtor(m1p);
            matrix_dtor(m3p);
        }
        return 0;
    }

    // 0x652640
    let near_door = !x(l3c).fcom(x(gf(0x858CC4))).test_parity(0x41);
    if near_door && !upside_down && rd::<i32>(veh, 0x590) != 9 && rd::<i32>(veh, 0x594) != 2 {
        let mut m = [0u32; 0x12];
        let mp: GPtr<u8> = GPtr::from_raw(m.as_mut_ptr()).cast();
        matrix_copy_ctor(mp, rd::<GPtr<u8>>(veh, 0x14));
        let mut l = [0f32; 3];
        let lp: GPtr<u8> = GPtr::from_raw(l.as_mut_ptr()).cast();
        let r = line_up_get_local_position_to_open_car_door(this, tp, veh, 0, assoc);
        for i in 0..3isize {
            wr::<u32>(lp, i * 4, rd::<u32>(r, i * 4));
        }
        let mut o = [0f32; 3];
        let op: GPtr<u8> = GPtr::from_raw(o.as_mut_ptr()).cast();
        let w = multiply3x3(op, mp, lp);
        let wv = [rd::<f32>(w, 0), rd::<f32>(w, 4), rd::<f32>(w, 8)];
        for (i, v) in (0isize..).zip(wv) {
            let s = x(rd::<f32>(mp, 0x30 + i * 4)).fadd_m(v).to_f32();
            wr::<f32>(mp, 0x30 + i * 4, s);
        }
        placeable_set_matrix(ped, mp);
        matrix_dtor(mp);
        return 0;
    }
    let m = rd::<GPtr<u8>>(ped, 0x14);
    if m.is_null() {
        wr::<f32>(ped, 4, p[0]);
        wr::<f32>(ped, 8, p[1]);
        wr::<f32>(ped, 0xC, p[2]);
    } else {
        wr::<f32>(m, 0x30, p[0]);
        wr::<f32>(m, 0x34, p[1]);
        wr::<f32>(m, 0x38, p[2]);
    }
    placeable_set_orientation(ped, 0, 0, rd::<u32>(ped, 0x558));
    0
}

pub const HOOKS: &[HookReg] = &[
    HOOK_LINE_UP_GET_LOCAL_POSITION_TO_OPEN_CAR_DOOR,
    HOOK_LINE_UP_PED_WITH_CAR_PROCESS_PED,
];
