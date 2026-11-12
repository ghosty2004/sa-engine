//! `CCarEnterExit` door geometry (batch `std14-veh-1`): `GetPositionToOpenCarDoor` 0x64E740,
//! `IsRoomForPedToLeaveCar` 0x6504C0 (both unreversed in gta-reversed) and `GetNearestCarDoor` 0x6528F0. Ported from
//! the 1.0 US bytes (gta-reversed as a reading aid; where it differs, the bytes are followed). Unverified
//! (docs/verify/HOWTO.md).
//!
//! Door ids: 8 front right, 9 rear right, 10 front left, 11 rear left, 0x12 the bike / quad side entry.
//! Model info (`CModelInfo::ms_modelInfoPtrs`): +0x3C vehicle type (5 = boat), +0x5C the vehicle structure, whose
//! dummy positions are +0x00 (dummy 0), +0x30 (dummy 4, the front seat) and +0x3C (dummy 5, the rear seat).
//! Handling (+0x384): +0xCC model flags (0x8 one rear seat test, 0x200 tandem seats, 0x8000 force door check),
//! +0xD4 the seat offset distance, +0xDE the animation group (`CVehicleAnimGroupData::m_vehicleAnimGroups`,
//! 0xC1CDC0, 0x94 bytes each).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{jne_mask, jp5, model_info, rd, wr, x};

orig! {
    /// `CVehicleAnimGroup::ComputeAnimDoorOffsets(CVector* ret, int door)` (0x6E3D10, `ret 8`)
    fn anim_door_offsets(group: GPtr<u8>, out: GPtr<u8>, ty: u32) -> GPtr<u8> = thiscall 0x6E3D10 => unported;
    /// `operator*(const CMatrix&, const CVector&)` into `out` (0x59C890)
    fn matrix_mul_vector(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `Multiply3x3(CVector* out, const CMatrix&, const CVector&)` (0x59C790)
    fn multiply3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => unported;
    /// `CBike::GetCorrectedWorldDoorPosition(CVector& out, CVector animOffset, CVector seat)` (0x6BF230, `ret 0x1C`)
    fn bike_corrected_world_door_position(bike: GPtr<u8>, out: GPtr<u8>, ox: u32, oy: u32, oz: u32, sx: u32, sy: u32, sz: u32) -> GPtr<u8> = thiscall 0x6BF230 => unported;
    /// `CVehicle` virtual slot 0xD4 (ST0)
    fn vehicle_vslot_d4(v: GPtr<u8>) -> f64 = thiscall vslot 0xD4 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool x7)` (0x56A490)
    fn world_get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bld: u32, veh: u32, ped: u32, obj: u32, dum: u32, see: u32, cam: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `CWorld::TestSphereAgainstWorld(CVector, float, CEntity*, bool x6)` (0x569E20)
    fn world_test_sphere_against_world(cx: u32, cy: u32, cz: u32, r: u32, ignore: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32, c: u32) -> GPtr<u8> = cdecl 0x569E20 => unported;
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool x7)` (0x5674E0)
    fn world_process_vertical_line(pos: GPtr<u8>, z: u32, cp: GPtr<u8>, ent: GPtr<u8>, b: u32, v: u32, p: u32, o: u32, d: u32, s: u32, l: u32) -> u8 = cdecl 0x5674E0 => unported;
    /// `CTaskManager::GetActiveTask()` (0x681720)
    fn task_manager_get_active_task(tm: GPtr<u8>) -> GPtr<u8> = thiscall 0x681720 => unported;
    /// `CTask::GetTaskType()` (vtable slot 0x10)
    fn task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPedGroups::AreInSameGroup(const CPed*, const CPed*)` (0x5F7F40)
    fn ped_groups_are_in_same_group(a: GPtr<u8>, b: GPtr<u8>) -> u8 = cdecl 0x5F7F40 => unported;
    /// `CCarEnterExit::IsPathToDoorBlockedByVehicleCollisionModel(const CPed*, CVehicle*, const CVector&)` (0x651210)
    fn path_to_door_blocked(ped: GPtr<u8>, veh: GPtr<u8>, pos: GPtr<u8>) -> u8 = cdecl 0x651210 => unported;
    fn get_position_to_open_car_door_o(out: GPtr<u8>, veh: GPtr<u8>, door: i32) -> GPtr<u8> = cdecl 0x64E740 => get_position_to_open_car_door;
    fn is_room_for_ped_to_leave_car_o(veh: GPtr<u8>, door: i32, offset: GPtr<u8>) -> u8 = cdecl 0x6504C0 => is_room_for_ped_to_leave_car;
}

/// `CVehicleAnimGroupData::m_vehicleAnimGroups`
const ANIM_GROUPS: usize = 0xC1CDC0;
/// `CCarEnterExit::ms_vecPedQuickDraggedOutCarAnimOffset`
const QUICK_DRAGGED_OUT_OFFSET: usize = 0xC18C48;

fn kf(a: u32) -> f32 {
    sa_core::mem::GPtr::<f32>::from_addr(a as usize).read()
}

fn vp(v: &mut [f32; 3]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}

fn read3(p: GPtr<u8>) -> [f32; 3] {
    [rd::<f32>(p, 0), rd::<f32>(p, 4), rd::<f32>(p, 8)]
}

fn write3(p: GPtr<u8>, v: [f32; 3]) {
    for (i, c) in v.iter().enumerate() {
        wr::<f32>(p, 4 * i as isize, *c);
    }
}

/// The entity's position (`m_matrix` +0x30, or the placement +4 without a matrix).
fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// The front seat dummy (dummy 0 for a boat model, else dummy 4).
fn front_dummy(mi: GPtr<u8>) -> [f32; 3] {
    let s = rd::<GPtr<u8>>(mi, 0x5C);
    read3(if rd::<u32>(mi, 0x3C) == 5 {
        s
    } else {
        s.byte_add(0x30)
    })
}

/// The rear seat dummy (dummy 5).
fn rear_dummy(mi: GPtr<u8>) -> [f32; 3] {
    read3(rd::<GPtr<u8>>(mi, 0x5C).byte_add(0x3C))
}

/// The animation group's door offset of type `ty` (0 front doors, 1 rear doors, 2 the bike side).
fn door_offset(hd: GPtr<u8>, ty: u32) -> [f32; 3] {
    let g = GPtr::<u8>::from_addr(ANIM_GROUPS + 0x94 * rd::<u8>(hd, 0xDE) as usize);
    let mut d = [0f32; 3];
    anim_door_offsets(g, vp(&mut d), ty);
    d
}

fn f(v: X87) -> f32 {
    v.to_f32()
}

/// `CCarEnterExit::GetPositionToOpenCarDoor(CVector* out, const CVehicle*, int door)` (0x64E740, cdecl, returns
/// `out`).
///
/// Cars (type +0x590 not 9, no tandem seats): seat = 0 for animation group 0x65 and a rear door, else the handling's
/// seat offset; the animation group's door offset C of type 0 (front), 1 (rear), 2 (door 0x12), or the seat offset's
/// bits for any other door (as the original passes them). The seat dummy A (front: doors 8 / 10, rear: 9 / 11)
/// is shifted outwards by the seat offset (left doors: A.x = -(A.x + seat); right doors: A.x + seat, C mirrored);
/// other doors take the front dummy and C = 0. B = A - C; for sub-type 1, or the handling's 0x8 flag with sub-type
/// other than 4, B.z = 0.95 - vtable 0xD4. Out = position + `Multiply3x3(matrix, B)`; the original's last
/// `Multiply3x3(matrix, A)` writes a local nobody reads (not repeated).
///
/// Bikes and tandem seats: door 0x12: C = offset type 2, out = matrix * (front.x - C.x, C.y + front.y, front.z - C.z).
/// Other doors: C = offset type 0, A = rear dummy for 9 / 11, else front; right doors (8 / 9) mirror C.x (* -1.0).
/// Bikes (type 9): `CBike::GetCorrectedWorldDoorPosition(out, C, A)`. Otherwise A.x -/+ seat (left / right), out =
/// matrix * (A - C).
#[hook(addr = 0x64E740, conv = cdecl, name = "CCarEnterExit::GetPositionToOpenCarDoor", expect = "83 EC 30 53 55 56 8B 74 24 44",
       unverified = "std14-veh-1", route = "L2", hazards = "stack-residue", recs = "enter-car",
       note = "other doors pass the seat offset's float bits as the anim offset type, as the original does; a dead Multiply3x3 into a local is skipped")]
pub fn get_position_to_open_car_door(out: GPtr<u8>, veh: GPtr<u8>, door: i32) -> GPtr<u8> {
    let mi = model_info(i32::from(rd::<i16>(veh, 0x22)));
    let hd = rd::<GPtr<u8>>(veh, 0x384);
    let mat = rd::<GPtr<u8>>(veh, 0x14);
    let vtype = rd::<u32>(veh, 0x590);
    if vtype != 9 && (rd::<u32>(hd, 0xCC) >> 9) & 1 == 0 {
        let seat_bits = if rd::<u8>(hd, 0xDE) == 0x65 && (door == 11 || door == 9) {
            0
        } else {
            rd::<u32>(hd, 0xD4)
        };
        let seat = f32::from_bits(seat_bits);
        let ty = match door {
            10 | 8 => 0,
            11 | 9 => 1,
            0x12 => 2,
            _ => seat_bits,
        };
        let c = door_offset(hd, ty);
        let (mut a, c, right) = match door {
            10 => (front_dummy(mi), c, false),
            8 => (front_dummy(mi), c, true),
            11 => (rear_dummy(mi), c, false),
            9 => (rear_dummy(mi), c, true),
            _ => (front_dummy(mi), [0.0; 3], false),
        };
        let bx = if (8..=11).contains(&door) {
            if right {
                a[0] = f(x(a[0]).fadd_m(seat));
                f(x(a[0]).fsub(x(c[0]).fchs()))
            } else {
                a[0] = f(x(a[0]).fadd_m(seat).fchs());
                f(x(a[0]).fsub(x(c[0])))
            }
        } else {
            f(x(a[0]).fsub(x(c[0])))
        };
        let mut b = [bx, f(x(a[1]).fsub_m(c[1])), f(x(a[2]).fsub_m(c[2]))];
        let sub = rd::<u32>(veh, 0x594);
        if sub == 1 || (rd::<u8>(hd, 0xCC) & 8 != 0 && sub != 4) {
            b[2] = f(X87::from_f64_exact(vehicle_vslot_d4(veh)).fsubr_m(kf(0x858EF0)));
        }
        let mut r = [0f32; 3];
        multiply3x3(vp(&mut r), mat, vp(&mut b));
        let p = read3(entity_pos(veh));
        write3(
            out,
            [
                f(x(p[0]).fadd_m(r[0])),
                f(x(p[1]).fadd_m(r[1])),
                f(x(p[2]).fadd_m(r[2])),
            ],
        );
        return out;
    }
    if door == 0x12 {
        let d = door_offset(hd, 2);
        let a = front_dummy(mi);
        let mut b = [
            f(x(a[0]).fadd(x(d[0]).fchs())),
            f(x(d[1]).fadd_m(a[1])),
            f(x(a[2]).fadd(x(d[2]).fchs())),
        ];
        let mut r = [0f32; 3];
        matrix_mul_vector(vp(&mut r), mat, vp(&mut b));
        write3(out, r);
        return out;
    }
    let mut c = door_offset(hd, 0);
    let mut a = if door == 11 || door == 9 {
        rear_dummy(mi)
    } else {
        front_dummy(mi)
    };
    let right = door == 9 || door == 8;
    if vtype == 9 {
        if right {
            c[0] = f(x(c[0]).fmul_m(kf(0x858C1C)));
        }
        let mut b = [0f32; 3];
        let [ox, oy, oz] = c.map(f32::to_bits);
        let [sx, sy, sz] = a.map(f32::to_bits);
        bike_corrected_world_door_position(veh, vp(&mut b), ox, oy, oz, sx, sy, sz);
        write3(out, b);
        return out;
    }
    let seat = x(rd::<f32>(hd, 0xD4));
    if right {
        c[0] = f(x(c[0]).fmul_m(kf(0x858C1C)));
        a[0] = f(x(a[0]).fadd(seat));
    } else {
        a[0] = f(x(a[0]).fsub(seat));
    }
    let mut b = [
        f(x(a[0]).fsub_m(c[0])),
        f(x(a[1]).fsub_m(c[1])),
        f(x(a[2]).fsub_m(c[2])),
    ];
    let mut r = [0f32; 3];
    matrix_mul_vector(vp(&mut r), mat, vp(&mut b));
    write3(out, r);
    out
}

/// `CCarEnterExit::IsRoomForPedToLeaveCar(const CVehicle*, int door, const CVector* offset)` (0x6504C0, cdecl, AL).
///
/// The door's seat dummy Q (cars: doors 8..11 only, else false; bikes and tandem seats: rear dummy for 9 / 11, else
/// the front one), left doors (10 / 11) mirrored, is taken to the world (`matrix * Q`); T =
/// `GetPositionToOpenCarDoor`, plus `Multiply3x3(matrix, offset)` (offset.x mirrored for doors 8 / 9) when an offset
/// is given. Upside down (up.z < 0): Q.z and T.z + 0.5. The line start L: bikes (T.x, T.y, T.z + 0.2) with T.z
/// raised by 0.2 + 0.35; others Q + (T - Q) * (len + 0.35) / len in the plane (L.z = T.z).
/// False when the line Q → L is not clear (buildings, objects), when a sphere of 0.35 at T (buildings unless a
/// train, vehicles, objects; the vehicle ignored) touches an entity other than the vehicle's +0xFC (model 0x260
/// against model 0x241 is allowed), or when the ground under T (vertical line down 1000) is above T but below T.z +
/// 0.6. Then the ground: boats and models 0x1CC / 0x21B / 0x1BF / 0x1A1 take (first ground z) - 1.0, others a
/// vertical line up 1000 (false on no hit) and its z. True when the first ground z is exactly 0.0 or not below
/// that value.
#[hook(addr = 0x6504C0, conv = cdecl, name = "CCarEnterExit::IsRoomForPedToLeaveCar", expect = "81 EC 88 00 00 00 C7 44 24 28",
       unverified = "std14-veh-1", route = "L2", hazards = "stack-residue,ax-al-return", recs = "enter-car",
       note = "the TestSphereAgainstWorld buildings flag is pushed as a dword whose upper bytes are stack residue; the second vertical line overwrites the colpoint the comparison reads first")]
pub fn is_room_for_ped_to_leave_car(veh: GPtr<u8>, door: i32, offset: GPtr<u8>) -> u8 {
    let mi = model_info(i32::from(rd::<i16>(veh, 0x22)));
    let hd = rd::<GPtr<u8>>(veh, 0x384);
    let mat = rd::<GPtr<u8>>(veh, 0x14);
    let vtype = rd::<u32>(veh, 0x590);
    let mut p = if vtype != 9 && (rd::<u32>(hd, 0xCC) >> 9) & 1 == 0 {
        match door {
            10 | 8 => front_dummy(mi),
            11 | 9 => rear_dummy(mi),
            _ => return 0,
        }
    } else if door == 11 || door == 9 {
        rear_dummy(mi)
    } else {
        front_dummy(mi)
    };
    if door == 10 || door == 11 {
        p[0] = f(x(p[0]).fchs());
    }
    let mut q = [0f32; 3];
    matrix_mul_vector(vp(&mut q), mat, vp(&mut p));
    let mut t = [0f32; 3];
    get_position_to_open_car_door_o(vp(&mut t), veh, door);
    if !offset.is_null() {
        let mut o = read3(offset);
        if door == 8 || door == 9 {
            o[0] = f(x(o[0]).fchs());
        }
        let mut r = [0f32; 3];
        multiply3x3(vp(&mut r), mat, vp(&mut o));
        t = [
            f(x(r[0]).fadd_m(t[0])),
            f(x(r[1]).fadd_m(t[1])),
            f(x(r[2]).fadd_m(t[2])),
        ];
    }
    if !jp5(x(rd::<f32>(mat, 0x28)), x(0.0)) {
        q[2] = f(x(q[2]).fadd_m(0.5));
        t[2] = f(x(t[2]).fadd_m(0.5));
    }
    let dx = f(x(t[0]).fsub_m(q[0]));
    let dy = x(t[1]).fsub_m(q[1]);
    let mut l = if vtype == 9 {
        t[2] = f(x(t[2]).fadd_m(kf(0x858CC4)));
        let lz = t[2];
        t[2] = f(x(t[2]).fadd_m(kf(0x858F9C)));
        [t[0], t[1], lz]
    } else {
        let len = dy.fmul(dy).fadd(x(dx).fmul_m(dx)).fsqrt();
        let s = x(kf(0x858F9C)).fadd(len).fdiv(len);
        let lx = f(x(dx).fmul(s).fadd_m(q[0]));
        let ly = f(x(f(dy.fmul(s))).fadd_m(q[1]));
        [lx, ly, t[2]]
    };
    if world_get_is_line_of_sight_clear(vp(&mut q), vp(&mut l), 1, 0, 0, 1, 0, 0, 0) == 0 {
        return 0;
    }
    let bld = u32::from(vtype != 6);
    let [tx, ty, tz] = t.map(f32::to_bits);
    let hit = world_test_sphere_against_world(tx, ty, tz, 0x3EB3_3333, veh, bld, 1, 0, 1, 0, 0);
    if !hit.is_null()
        && !(rd::<u16>(hit, 0x22) == 0x260 && rd::<u16>(veh, 0x22) == 0x241)
        && hit != rd::<GPtr<u8>>(veh, 0xFC)
    {
        return 0;
    }
    // CColPoint (0x2C bytes; position +0..+8), zeroed z as the original's locals
    let mut cp = [0u32; 11];
    let cpp = GPtr::from_raw(cp.as_mut_ptr()).cast::<u8>();
    let mut ent = 0u32;
    let entp = GPtr::from_raw(&raw mut ent).cast::<u8>();
    let down = world_process_vertical_line(vp(&mut t), 0x447A_0000, cpp, entp, 1, 0, 0, 1, 0, 0, 0);
    let ground = f32::from_bits(cp[2]);
    if down != 0
        && !jne_mask(x(ground), x(t[2]), 0x41)
        && !jne_mask(x(t[2]).fadd_m(kf(0x858CC8)), x(ground), 0x41)
    {
        return 0;
    }
    let model = rd::<u16>(veh, 0x22);
    let v = if vtype == 5 || matches!(model, 0x1CC | 0x21B | 0x1BF | 0x1A1) {
        x(f32::from_bits(cp[2])).fsub_m(1.0)
    } else {
        if world_process_vertical_line(vp(&mut t), 0xC47A_0000, cpp, entp, 1, 0, 0, 1, 0, 0, 0) == 0 {
            return 0;
        }
        x(f32::from_bits(cp[2]))
    };
    // fcomp 0.0; test ah, 0x44; jnp: exactly equal to 0.0 → true
    if !x(ground).fcom(x(0.0)).test_parity(0x44) {
        return 1;
    }
    u8::from(jp5(x(ground), v))
}

/// `CCarEnterExit::GetNearestCarDoor(const CPed*, const CVehicle*, CVector& out, int& door)` (0x6528F0, cdecl, AL;
/// the protection fragment 0x4058AC loads the first passenger into EBP).
///
/// Dragged-out offsets: the driver's (front doors) and the first passenger's (door 8 in the general case) are
/// `ms_vecPedQuickDraggedOutCarAnimOffset` when that seat is taken. Bikes other than the BMX (type 9, sub-type not
/// 10) and quads (sub-type 2) drop both and, unless the ped's active task is 0x2BC (enter as passenger), with the
/// vehicle's right.z within 0.1 and the ped's bearing from the vehicle within 30 degrees of its heading, try door
/// 0x12 when *door is 0x12, or 0 with a player whose move blend ratio is above 1.5, or 0 with a non-cop non-player
/// running (move state 6) with temper above 0x41. Buses / low vehicles (+0x429 & 0xA) drop both offsets.
/// With a towed vehicle (+0x4C8): the nearer front door's path is tested against the towed vehicle's collision model
/// (`IsPathToDoorBlockedByVehicleCollisionModel(ped, towed, door)`) and pushed to (999.9, 999.9) when blocked.
/// Force-door-check automobiles take the only front door node present (left first). Then a requested door that has
/// room is taken (8 / 10 only, any other door fails). Then, unless the driver is in the ped's group or flagged
/// (+0x46C & 0x20000000): buses or a nearer left door try left then right; otherwise right (switched to left when the
/// first passenger is in the ped's group, flagged or the car a mission vehicle, without tandem seats and not a bike,
/// or with getting-in flag 4, and left has room) then left.
#[hook(addr = 0x6528F0, conv = cdecl, name = "CCarEnterExit::GetNearestCarDoor", expect = "83 EC 2C 53 55 56 8B 74 24 40 90 E9",
       unverified = "std14-veh-1", route = "L2", hazards = "ax-al-return", recs = "enter-car",
       note = "entry chunk 0x4058AC (`mov ebp, [esi+0x464]`); the towed-vehicle path test passes the towed vehicle, and door 0x12 also enters the bike branch, as the bytes do (gta-reversed differs)")]
pub fn get_nearest_car_door(ped: GPtr<u8>, veh: GPtr<u8>, out: GPtr<u8>, door: GPtr<u8>) -> u8 {
    let off = |taken: bool| {
        if taken {
            GPtr::<u8>::from_addr(QUICK_DRAGGED_OUT_OFFSET)
        } else {
            GPtr::NULL
        }
    };
    let mut drv_off = off(rd::<u32>(veh, 0x460) != 0);
    let mut psg_off = off(rd::<u32>(veh, 0x464) != 0);
    let mut lf = [0f32; 3];
    let mut rf = [0f32; 3];
    get_position_to_open_car_door_o(vp(&mut lf), veh, 10);
    get_position_to_open_car_door_o(vp(&mut rf), veh, 8);
    let vtype = rd::<u32>(veh, 0x590);
    let sub = rd::<u32>(veh, 0x594);
    let take = |d: i32, p: [f32; 3]| {
        wr::<i32>(door, 0, d);
        write3(out, p);
        1
    };
    if (vtype == 9 && sub != 10) || sub == 2 {
        drv_off = GPtr::NULL;
        psg_off = GPtr::NULL;
        if bike_side_entry(ped, veh, door) && is_room_for_ped_to_leave_car_o(veh, 0x12, GPtr::NULL) != 0 {
            wr::<i32>(door, 0, 0x12);
            let mut p = [0f32; 3];
            get_position_to_open_car_door_o(vp(&mut p), veh, 0x12);
            write3(out, p);
            return 1;
        }
    } else if rd::<u8>(veh, 0x429) & 0xA != 0 {
        drv_off = GPtr::NULL;
        psg_off = GPtr::NULL;
    }
    let pp = read3(entity_pos(ped));
    let mut dl = [f(x(lf[0]).fsub_m(pp[0])), f(x(lf[1]).fsub_m(pp[1]))];
    let mut dr = [f(x(rf[0]).fsub_m(pp[0])), f(x(rf[1]).fsub_m(pp[1]))];
    let sq = |d: [f32; 2]| x(d[1]).fmul_m(d[1]).fadd(x(d[0]).fmul_m(d[0]));
    let far = f32::from_bits(0x4479_F99A);
    let towed = rd::<GPtr<u8>>(veh, 0x4C8);
    if !towed.is_null() {
        // fcompp: ST0 = right, ST1 = left; `test ah, 0x41; jne` taken when right <= left
        if !jne_mask(sq(dr), sq(dl), 0x41) {
            if path_to_door_blocked(ped, towed, vp(&mut lf)) != 0 {
                dl = [far, far];
            }
        } else if path_to_door_blocked(ped, towed, vp(&mut rf)) != 0 {
            dr = [far, far];
        }
    }
    let hd = rd::<GPtr<u8>>(veh, 0x384);
    if rd::<u32>(hd, 0xCC) & 0x8000 != 0 && vtype == 0 {
        let node_lf = rd::<u32>(veh, 0x670);
        let node_rf = rd::<u32>(veh, 0x668);
        if node_lf != 0 {
            if node_rf == 0 && is_room_for_ped_to_leave_car_o(veh, 10, drv_off) != 0 {
                return take(10, lf);
            }
            if node_rf == 0 {
                return 0;
            }
        } else {
            if node_rf != 0 && is_room_for_ped_to_leave_car_o(veh, 8, drv_off) != 0 {
                return take(8, rf);
            }
            return 0;
        }
    }
    let want = rd::<i32>(door, 0);
    if want != 0 && is_room_for_ped_to_leave_car_o(veh, want, drv_off) != 0 {
        return match rd::<i32>(door, 0) {
            10 => {
                write3(out, lf);
                1
            }
            8 => {
                write3(out, rf);
                1
            }
            _ => 0,
        };
    }
    let drv = rd::<GPtr<u8>>(veh, 0x460);
    if !drv.is_null()
        && (ped_groups_are_in_same_group(drv, ped) != 0 || rd::<u32>(drv, 0x46C) & 0x2000_0000 != 0)
    {
        return 0;
    }
    let left_first = rd::<u8>(veh, 0x429) & 2 != 0 || !jne_mask(sq(dr), sq(dl), 0x41);
    if left_first {
        if is_room_for_ped_to_leave_car_o(veh, 10, drv_off) != 0 {
            return take(10, lf);
        }
        if is_room_for_ped_to_leave_car_o(veh, 8, psg_off) != 0 {
            return take(8, rf);
        }
        return 0;
    }
    if is_room_for_ped_to_leave_car_o(veh, 8, psg_off) == 0 {
        if is_room_for_ped_to_leave_car_o(veh, 10, drv_off) != 0 {
            return take(10, lf);
        }
        return 0;
    }
    let p0 = rd::<GPtr<u8>>(veh, 0x464);
    if !p0.is_null()
        && vtype != 9
        && (rd::<u32>(hd, 0xCC) >> 9) & 1 == 0
        && (ped_groups_are_in_same_group(p0, ped) != 0
            || rd::<u32>(p0, 0x46C) & 0x2000_0000 != 0
            || rd::<u8>(veh, 0x4A4) == 2)
        && is_room_for_ped_to_leave_car_o(veh, 10, drv_off) != 0
    {
        return take(10, lf);
    }
    if rd::<u8>(veh, 0x486) & 4 != 0 && is_room_for_ped_to_leave_car_o(veh, 10, drv_off) != 0 {
        return take(10, lf);
    }
    take(8, rf)
}

/// The bike / quad side-entry test of [`get_nearest_car_door`] (0x65295D..0x652A7F): whether door 0x12 is tried.
fn bike_side_entry(ped: GPtr<u8>, veh: GPtr<u8>, door: GPtr<u8>) -> bool {
    let task = task_manager_get_active_task(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4));
    if task_get_type(task) == 0x2BC {
        return false;
    }
    let m = rd::<GPtr<u8>>(veh, 0x14);
    if jp5(x(rd::<f32>(m, 8)).fabs(), x(kf(0x858B1C))) {
        return false;
    }
    let vp_ = read3(if m.is_null() {
        veh.byte_add(4)
    } else {
        m.byte_add(0x30)
    });
    let pp = read3(entity_pos(ped));
    let dx = x(pp[0]).fsub_m(vp_[0]);
    let dy = x(pp[1]).fsub_m(vp_[1]);
    let bearing = dx.fchs().fpatan(dy);
    let heading = if m.is_null() {
        x(rd::<f32>(veh, 0x10))
    } else {
        x(rd::<f32>(m, 0x10)).fchs().fpatan(x(rd::<f32>(m, 0x14)))
    };
    let mut d = bearing.fsub(heading);
    let pi = x(kf(0x858CB8));
    if !d.fcom(pi).test_any(0x41) {
        d = d.fsub_m(kf(0x858CBC));
    } else if !d.fcom(x(kf(0x858CC0))).test_parity(0x05) {
        d = d.fadd_m(kf(0x858CBC));
    }
    if jp5(d.fabs(), x(kf(0x858F20))) {
        return false;
    }
    let want = rd::<i32>(door, 0);
    if ped_is_player(ped) != 0 {
        let pd = rd::<GPtr<u8>>(ped, 0x480);
        if !jne_mask(x(rd::<f32>(pd, 0x14)), x(kf(0x858CE8)), 0x41) && want == 0 {
            return true;
        }
    }
    if ped_is_player(ped) == 0
        && rd::<u32>(ped, 0x598) != 6
        && rd::<u32>(ped, 0x534) == 6
        && rd::<i8>(rd::<GPtr<u8>>(ped, 0x59C), 0x25) > 0x41
        && want == 0
    {
        return true;
    }
    want == 0x12
}

pub const HOOKS: &[HookReg] = &[
    HOOK_GET_POSITION_TO_OPEN_CAR_DOOR,
    HOOK_IS_ROOM_FOR_PED_TO_LEAVE_CAR,
    HOOK_GET_NEAREST_CAR_DOOR,
];
