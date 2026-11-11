//! WRITER-MVP-B (helperB) WORLD ports, part 3: garage containment tests and `CGarage::BuildRotatedDoorMatrix`.
//! Ported from the 1.0 US bytes (the exe wins over gta-reversed).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

use super::common::{rd, wr, x};

orig! {
    /// `CEntity::GetColModel` (0x535300)
    fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CPlaceable::GetMatrix` (0x411990)
    fn placeable_get_matrix(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x411990 => unported;
    /// `operator*(CVector* out, const CMatrix*, const CVector*)` (0x59C890, cdecl)
    fn matrix_mul_vec(out: GPtr<f32>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<f32> = cdecl 0x59C890 => unported;
    /// `CGarage::IsPointInsideGarage(CVector, float)` (0x4487D0, `ret 0x10`)
    fn garage_is_point_inside_r(g: GPtr<u8>, x: f32, y: f32, z: f32, r: f32) -> u8 = thiscall 0x4487D0 => unported;
    /// `CGarage::IsEntityEntirelyInside3D(CEntity*, float)` (0x448BE0, `ret 8`)
    fn garage_is_entity_entirely_inside_3d_o(g: GPtr<u8>, e: GPtr<u8>, m: f32) -> u8 = thiscall 0x448BE0 => crate::world::mvpb_world3::garage_is_entity_entirely_inside_3d;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0, cdecl)
    fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210, cdecl)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740, `ret 4`)
    fn task_manager_find_active_task_by_type(tm: GPtr<u8>, ty: u32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CPlaceable::AllocateMatrix` (0x54F560)
    fn allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, `ret 4`)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CMatrix::SetRotateZ(float)` (0x59B0E0, `ret 4`)
    fn matrix_set_rotate_z(m: GPtr<u8>, a: f32) = thiscall 0x59B0E0 => unported;
    /// `CMatrix::~CMatrix` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CrossProduct(CVector* out, const CVector*, const CVector*)` (0x59C730, cdecl)
    fn cross_product(out: GPtr<f32>, a: GPtr<u8>, b: GPtr<f32>) -> GPtr<f32> = cdecl 0x59C730 => unported;
}

fn entity_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `CGarage::IsEntityEntirelyInside3D(CEntity*, float margin)` (0x448BE0, `ret 8`): AL = 0 when the entity's
/// position is outside the box shrunk by the margin on any axis (x87: `min - m > p` or `max + m < p`, x/y/z with
/// z range +8..+0x1C; unordered compares count as inside), or when any collision sphere (centre through
/// `GetMatrix`, radius `margin - r`) is not inside (`IsPointInsideGarage(c, r)`); else 1.
#[hook(addr = 0x448BE0, conv = thiscall, name = "CGarage::IsEntityEntirelyInside3D", this = "CGarage",
       expect = "83 EC 10 53 55", route = "L2", hazards = "heap-writes")]
pub fn garage_is_entity_entirely_inside_3d(this: GPtr<u8>, e: GPtr<u8>, m: f32) -> u8 {
    for (lo, hi, k) in [(0x28isize, 0x2Cisize, 0isize), (0x30, 0x34, 4), (8, 0x1C, 8)] {
        let p = x(rd::<f32>(entity_pos(e), k));
        if x(rd::<f32>(this, lo)).fsub_m(m).fcom(p) == Cmp::Greater {
            return 0;
        }
        let p = x(rd::<f32>(entity_pos(e), k));
        if x(m).fadd_m(rd::<f32>(this, hi)).fcom(p) == Cmp::Less {
            return 0;
        }
    }
    let cd = rd::<GPtr<u8>>(entity_get_col_model(e), 0x2C);
    if cd.is_null() {
        return 1;
    }
    let mut i: i16 = 0;
    while i < rd::<i16>(cd, 0) {
        let sphere = rd::<GPtr<u8>>(cd, 8).byte_add(0x14 * i as isize);
        let mat = placeable_get_matrix(e);
        let mut out = [0.0f32; 3];
        let c = matrix_mul_vec(GPtr::from_raw(out.as_mut_ptr()), mat, sphere);
        let r = x(m).fsub_m(rd::<f32>(rd::<GPtr<u8>>(cd, 8), 0x14 * i as isize + 0xC)).to_f32();
        if garage_is_point_inside_r(this, c.read(), c.wrapping_index(1).read(), c.wrapping_index(2).read(), r) == 0 {
            return 0;
        }
        i += 1;
    }
    1
}

/// `|v|` as the original computes it: negated only when `v < 0` (x87 compare with 0).
fn abs_x87(v: f32) -> X87 {
    let a = x(v);
    if a.fcom(X87::zero()) == Cmp::Less { a.fchs() } else { a }
}

/// `CGarage::IsStaticPlayerCarEntirelyInside` (0x44A830): AL = 0 unless the player is in an automobile or bike
/// (+0x590 = 0 / 9) and not leaving it (task 0x2C0 active), the car's position is inside the garage's x/y range
/// (`min <= p <= max`, x87), each move-speed component is at most 0.01 in magnitude and the squared speed at most
/// 0.0001; then `IsEntityEntirelyInside3D(car, 0)`.
#[hook(addr = 0x44A830, conv = thiscall, name = "CGarage::IsStaticPlayerCarEntirelyInside", this = "CGarage",
       expect = "56 6A 00 6A FF 8B F1", route = "L2", hazards = "none")]
pub fn garage_is_static_player_car_entirely_inside(this: GPtr<u8>) -> u8 {
    if find_player_vehicle(-1, 0).is_null() {
        return 0;
    }
    if rd::<u32>(find_player_vehicle(-1, 0), 0x590) != 0 && rd::<u32>(find_player_vehicle(-1, 0), 0x590) != 9 {
        return 0;
    }
    let ped = find_player_ped(-1);
    if !task_manager_find_active_task_by_type(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 0x2C0).is_null() {
        return 0;
    }
    let v = find_player_vehicle(-1, 0);
    let p = entity_pos(v);
    if x(rd::<f32>(p, 0)).fcom(x(rd::<f32>(this, 0x28))) == Cmp::Less
        || x(rd::<f32>(p, 0)).fcom(x(rd::<f32>(this, 0x2C))) == Cmp::Greater
        || x(rd::<f32>(p, 4)).fcom(x(rd::<f32>(this, 0x30))) == Cmp::Less
        || x(rd::<f32>(p, 4)).fcom(x(rd::<f32>(this, 0x34))) == Cmp::Greater
    {
        return 0;
    }
    let lim = x(f32::from_bits(0x3C23_D70A));
    let sx = abs_x87(rd::<f32>(v, 0x44));
    if sx.fcom(lim) == Cmp::Greater {
        return 0;
    }
    let sy = abs_x87(rd::<f32>(v, 0x48));
    if sy.fcom(lim) == Cmp::Greater {
        return 0;
    }
    let sz = abs_x87(rd::<f32>(v, 0x4C));
    if sz.fcom(lim) == Cmp::Greater {
        return 0;
    }
    let s = sz.fmul(sz).fadd(sy.fmul(sy)).fadd(sx.fmul(sx));
    if s.fcom(x(f32::from_bits(0x38D1_B717))) == Cmp::Greater {
        return 0;
    }
    garage_is_entity_entirely_inside_3d_o(this, v, 0.0)
}

fn ensure_matrix(e: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(e, 0x14) == 0 {
        allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(4), rd::<GPtr<u8>>(e, 0x14));
    }
    rd::<GPtr<u8>>(e, 0x14)
}

/// `CGarage::BuildRotatedDoorMatrix(CEntity* door, float t)` (0x4479F0, cdecl): with `a = t * -pi/2`,
/// `s = sin a`, `c = cos a` (stored as f32): up = (-s*fwd.y, s*fwd.x, c), right = fwd x up, both written into the
/// door's matrix (allocated/updated whenever missing, before every write as the original). Kept original detail:
/// `s` is stored into the caller's `t` argument slot (not modelled).
#[hook(addr = 0x4479F0, conv = cdecl, name = "CGarage::BuildRotatedDoorMatrix", expect = "83 EC 1C D9 44 24 24",
       route = "L2", hazards = "x87-transcendental,heap-writes",
       note = "FSIN/FCOS results stored as f32; writes its own t argument slot")]
pub fn garage_build_rotated_door_matrix(door: GPtr<u8>, t: f32) {
    let a = x(t).fmul_m(f32::from_bits(0xBFC9_0FDB));
    let s = a.fsin().to_f32();
    let c = a.fcos().to_f32();
    ensure_matrix(door);
    let m = ensure_matrix(door);
    let fx = x(s).fmul_m(rd::<f32>(m, 0x10));
    let fy = x(s).fmul_m(rd::<f32>(rd::<GPtr<u8>>(door, 0x14), 0x14));
    let mut up = [fy.fchs().to_f32(), fx.to_f32(), c];
    let m = ensure_matrix(door);
    let mut right = [0.0f32; 3];
    cross_product(GPtr::from_raw(right.as_mut_ptr()), m.byte_add(0x10), GPtr::from_raw(up.as_mut_ptr()));
    for (off, v) in [(0isize, right[0]), (4, right[1]), (8, right[2]), (0x20, up[0]), (0x24, up[1]), (0x28, up[2])] {
        let m = ensure_matrix(door);
        wr::<f32>(m, off, v);
    }
}

/// The axis-aligned entry-exit box test: `x` in [+8, +0x10], `y` in [+0x14, +0xC] (Less / Greater fail, x87,
/// unordered passes) and `|pos.z - +0x18| < 1`.
fn entry_exit_box(this: GPtr<u8>, px: X87, py: X87, pz: f32) -> bool {
    px.fcom(x(rd::<f32>(this, 8))) != Cmp::Less
        && px.fcom(x(rd::<f32>(this, 0x10))) != Cmp::Greater
        && py.fcom(x(rd::<f32>(this, 0x14))) != Cmp::Less
        && py.fcom(x(rd::<f32>(this, 0xC))) != Cmp::Greater
        && x(pz).fsub_m(rd::<f32>(this, 0x18)).fabs().fcom(x(1.0)) == Cmp::Less
}

/// `CEntryExit::IsInArea(const CVector&)` (0x43E460, `ret 4`): with no rotation (+0x1C == 0) the box test on the
/// position; otherwise the position is rotated about the box centre by a stack `CMatrix::SetRotateZ(+0x1C)`
/// (`centre + M * (pos.xy - centre, pos.z)`, results stored as f32), tested, and the matrix destroyed. AL = result.
#[hook(addr = 0x43E460, conv = thiscall, name = "CEntryExit::IsInArea", expect = "64 A1 00 00 00 00",
       route = "L2", hazards = "none")]
pub fn entry_exit_is_in_area(this: GPtr<u8>, pos: GPtr<u8>) -> u8 {
    if !x(rd::<f32>(this, 0x1C)).fcom(X87::zero()).test_parity(0x44) {
        return u8::from(entry_exit_box(this, x(rd::<f32>(pos, 0)), x(rd::<f32>(pos, 4)), rd::<f32>(pos, 8)));
    }
    let cy = x(rd::<f32>(this, 0xC)).fsub_m(rd::<f32>(this, 0x14)).fmul_m(0.5).fadd_m(rd::<f32>(this, 0x14)).to_f32();
    let cx = x(rd::<f32>(this, 0x10)).fsub_m(rd::<f32>(this, 8)).fmul_m(0.5).fadd_m(rd::<f32>(this, 8)).to_f32();
    let mut d = [
        x(rd::<f32>(pos, 0)).fsub_m(cx).to_f32(),
        x(rd::<f32>(pos, 4)).fsub_m(cy).to_f32(),
        x(rd::<f32>(pos, 8)).to_f32(),
    ];
    let mut mat = [0u32; 18];
    let m: GPtr<u8> = GPtr::from_raw(mat.as_mut_ptr().cast());
    matrix_set_rotate_z(m, rd::<f32>(this, 0x1C));
    let mut out = [0.0f32; 3];
    let r = matrix_mul_vec(GPtr::from_raw(out.as_mut_ptr()), m, GPtr::from_raw(d.as_mut_ptr()).cast());
    let rx = x(r.read()).fadd_m(cx).to_f32();
    let ry = x(r.wrapping_index(1).read()).fadd_m(cy).to_f32();
    let ok = entry_exit_box(this, x(rx), x(ry), rd::<f32>(pos, 8));
    matrix_dtor(m);
    u8::from(ok)
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_ENTRY_EXIT_IS_IN_AREA,
    HOOK_GARAGE_IS_ENTITY_ENTIRELY_INSIDE_3D,
    HOOK_GARAGE_IS_STATIC_PLAYER_CAR_ENTIRELY_INSIDE,
    HOOK_GARAGE_BUILD_ROTATED_DOOR_MATRIX,
];
