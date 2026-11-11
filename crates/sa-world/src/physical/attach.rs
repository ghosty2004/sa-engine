//! `CPhysical` attachment, detachment, placement, collision checks and the per-frame control and
//! friction functions (PORT-PHYS). Ported from the 1.0 US bytes; gta-reversed `Entity/Physical.cpp`
//! was a reading aid.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use super::base::get_bound_rect_v;
use super::shared::*;
use crate::collision::fpu::Fpu;

/// A `CMatrix` local of the original (0x48 bytes; `m_pAttachMatrix` +0x40 and `m_bOwnsAttachedMatrix`
/// +0x44 are 0 as the original's inlined default constructor leaves them).
pub type LocalMatrix = [u32; 18];

orig! {
    /// virtual `CEntity::SetIsStatic(bool)` (vtable +0x10, thiscall; the dword the original pushes)
    pub fn set_is_static_v(e: Ent, v: u32) = thiscall vslot 0x10 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    pub fn world_remove(e: Ent) = cdecl 0x563280 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    pub fn world_add(e: Ent) = cdecl 0x563220 => unported;
    /// `CCollision::ProcessColModels(const CMatrix&, CColModel&, const CMatrix&, CColModel&,
    /// CColPoint*, CColPoint*, float*, bool)` (0x4185C0)
    pub fn process_col_models(m1: Ent, c1: Ent, m2: Ent, c2: Ent, cps: Ent, lcps: Ent, dists: Ent, b: u32) -> i32
        = cdecl 0x4185C0 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    pub fn allocate_matrix(e: Ent) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, thiscall on the placement)
    pub fn simple_transform_update_matrix(t: Ent, m: Ent) = thiscall 0x54F1B0 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40)
    pub fn simple_transform_update_rw_matrix(t: Ent, m: Ent) = thiscall 0x54EF40 => unported;
    /// `CPlaceable::GetMatrix()` (0x411990, returns the matrix)
    pub fn get_matrix(e: Ent) -> Ent = thiscall 0x411990 => unported;
    /// `CMatrix::ResetOrientation()` (0x59AEA0)
    pub fn matrix_reset_orientation(m: GPtr<LocalMatrix>) = thiscall 0x59AEA0 => unported;
    /// `CMatrix::RotateZ(float)` (0x59B390)
    pub fn matrix_rotate_z(m: GPtr<LocalMatrix>, a: u32) = thiscall 0x59B390 => unported;
    /// `CMatrix::RotateX(float)` (0x59B1E0)
    pub fn matrix_rotate_x(m: GPtr<LocalMatrix>, a: u32) = thiscall 0x59B1E0 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0)
    pub fn matrix_assign(dst: Ent, src: Ent) = thiscall 0x59BBC0 => unported;
    /// `CMatrix::operator*=(const CMatrix&)` (0x411A80)
    pub fn matrix_mul_assign(dst: Ent, rhs: Ent) = thiscall 0x411A80 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    pub fn matrix_dtor(m: Ent) = thiscall 0x59ACD0 => unported;
    /// `CMatrix::CMatrix(const CMatrix&)` (0x59BCF0)
    pub fn matrix_copy_ctor(m: Ent, src: Ent) = thiscall 0x59BCF0 => unported;
    /// `CMatrix::UpdateMatrix(RwMatrix*)` (0x59AD20)
    pub fn matrix_update_matrix(m: Ent, rw: Ent) = thiscall 0x59AD20 => unported;
    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70)
    pub fn matrix_update_rw_matrix(m: Ent, rw: Ent) = thiscall 0x59AD70 => unported;
    /// `CMatrix::CopyToRwMatrix(RwMatrix*)` (0x59B8B0)
    pub fn matrix_copy_to_rw(m: Ent, rw: Ent) = thiscall 0x59B8B0 => unported;
    /// `Invert(const CMatrix& src, CMatrix& dst)` (0x59B920, cdecl)
    pub fn matrix_invert(src: Ent, dst: Ent) -> Ent = cdecl 0x59B920 => unported;
    /// `RwFrameGetLTM(RwFrame*)` (0x7F0990)
    pub fn rw_frame_get_ltm(f: Ent) -> Ent = cdecl 0x7F0990 => unported;
    /// `RtQuatConvertFromMatrix(RtQuat*, const RwMatrix*)` (0x7EB5C0)
    pub fn rt_quat_convert_from_matrix(q: Ent, m: Ent) -> Ent = cdecl 0x7EB5C0 => unported;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, hidden return pointer)
    pub fn matrix_mul_point(out: GPtr<CVector>, m: Ent, v: GPtr<CVector>) -> GPtr<CVector> = cdecl 0x59C890 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    pub fn update_rw_frame(e: Ent) = thiscall 0x532B00 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    pub fn ped_is_player(p: Ent) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPedIntelligence::GetTaskClimb()` (0x601180)
    pub fn get_task_climb(i: Ent) -> Ent = thiscall 0x601180 => unported;
    /// `CWorld::ClearScanCodes()` (0x563470)
    pub fn world_clear_scan_codes() = cdecl 0x563470 => unported;
    /// `CPhysical::ProcessCollisionSectorList(int32 x, int32 y)` (0x54BA60)
    pub fn process_collision_sector_list(e: Ent, x: i32, y: i32) -> u8 = thiscall 0x54BA60 => crate::physical::mvpe_phys1::process_collision_sector_list;
    /// `CPhysical::ProcessCollisionSectorList_SimpleCar(CRepeatSector*)` (0x54CFF0)
    pub fn process_collision_sector_list_simple_car(e: Ent, s: Ent) -> u8 = thiscall 0x54CFF0 => crate::physical::mvpe_phys1::process_collision_sector_list_simple_car;
    /// `CPhysical::ApplySpeed()` (0x547B80)
    pub fn apply_speed(e: Ent) = thiscall 0x547B80 => apply_speed_port;
    /// `CPhysical::ApplyFriction(float, CColPoint&)` (0x5454C0)
    pub fn apply_friction_self(e: Ent, friction: f32, cp: Ent) -> u8 = thiscall 0x5454C0 => unported;
    /// `CPhysical::ApplyFriction()` (0x5483D0)
    pub fn apply_friction_void_call(e: Ent) = thiscall 0x5483D0 => apply_friction_void;
    /// `CPhysical::ApplyGravity()` (0x542FE0)
    pub fn apply_gravity_call(e: Ent) = thiscall 0x542FE0 => crate::physical::forces::apply_gravity;
    /// `CPhysical::ApplyAirResistance()` (0x544C40)
    pub fn apply_air_resistance_call(e: Ent) = thiscall 0x544C40 => crate::physical::forces::apply_air_resistance;
    /// `CPhysical::PositionAttachedEntity()` (0x546FF0)
    pub fn position_attached_entity(e: Ent) = thiscall 0x546FF0 => crate::physical::friction_attach::position_attached_entity;
    /// `CPhysical::CheckCollision()` (0x54D920)
    pub fn check_collision_call(e: Ent) -> u8 = thiscall 0x54D920 => check_collision;
    /// `CPhysical::AddCollisionRecord(CEntity*)` (0x543490)
    pub fn add_collision_record_call(e: Ent, other: Ent) = thiscall 0x543490 => crate::physical::base::add_collision_record;
    /// `CPhysical::ApplyCollision(CEntity*, CColPoint&, float&)` (0x5435C0)
    pub fn apply_collision_call(e: Ent, entity: Ent, cp: Ent, damage: GPtr<f32>) -> u8 = thiscall 0x5435C0 => crate::physical::collide::apply_collision;
    /// `CVector::MagnitudeSqr()` (0x406DA0, ST0)
    pub fn magnitude_sqr(v: Ent) -> f64 = thiscall 0x406DA0 => unported;
    /// `CVector::operator*=(float)` (0x40FEF0)
    pub fn vector_scale_assign(v: Ent, s: f32) = thiscall 0x40FEF0 => unported;
    /// `CPhysical::ApplyTurnForce(CVector, CVector)` (0x542A50)
    pub fn apply_turn_force_call(e: Ent, force: CVector, point: CVector) = thiscall 0x542A50 => crate::physical::forces::apply_turn_force;
}

/// `CWorld::m_aTempColPts` (0xB9ACD0)
const TEMP_COL_PTS: usize = 0xB9ACD0;
/// `CWorld::ms_nCurrentScanCode` (0xB7CD78, uint16)
const SCAN_CODE: usize = 0xB7CD78;
/// 99999.9 as the bytes write it (0x47C34FF3)
const MASS_NEARLY_INFINITE: u32 = 0x47C3_4FF3;

/// The raw words of a by-value `CVector` argument.
fn bits(v: CVector) -> [u32; 3] {
    [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()]
}

/// A fresh local `CMatrix` (attach pointer and owns flag 0).
fn local_matrix() -> LocalMatrix {
    [0; 18]
}

/// `if (!e->m_matrix) { e->AllocateMatrix(); e->m_placement.UpdateMatrix(e->m_matrix); }`
fn ensure_matrix(e: Ent) {
    if matrix(e).is_null() {
        allocate_matrix(e);
        simple_transform_update_matrix(e.byte_add(PLACEMENT_POS), matrix(e));
    }
}

/// `CPhysical::DettachAutoAttachedEntity()` (0x5446A0): `SetIsStatic(false)`, clears
/// `bAttachedToEntity` and the fake physics; with `bDisableCollisionForce` it becomes collidable and
/// stops; otherwise an attached vehicle passes its move and turn speed on. Clears the attach offset,
/// the ignored-collision entity, the attached-to pointer and the quaternion; an object takes its
/// object info's elasticity (`fld`/`fstp`).
#[hook(addr = 0x5446A0, conv = thiscall, name = "CPhysical::DettachAutoAttachedEntity", expect = "83 EC 0C 53 56", this = "CPhysical")]
pub fn dettach_auto_attached_entity(this: Ent) {
    set_is_static_v(this, 0);
    let fl = ru(this, PFLAGS) & 0xFDFF_FFFF;
    wu(this, PFLAGS, fl);
    wb(this, FAKE_PHYSICS, 0);
    if fl & 4 != 0 {
        wu(this, PFLAGS, fl | 8);
        wv_bits(this, TURN_SPEED, [0; 3]);
        wv_bits(this, MOVE_SPEED, [0; 3]);
    } else {
        let a = rp(this, ATTACHED_TO);
        if etype(a) == ty::VEHICLE {
            wv_bits(this, MOVE_SPEED, rv_bits(a, MOVE_SPEED));
            wv_bits(this, TURN_SPEED, rv_bits(a, TURN_SPEED));
        }
    }
    wu(this, ATTACH_OFFSET, 0);
    wu(this, ATTACH_OFFSET + 4, 0);
    wu(this, IGNORED_COLLISION, 0);
    wu(this, ATTACHED_TO, 0);
    wu(this, ATTACH_QUAT + 0xC, 0);
    wu(this, ATTACH_QUAT, 0);
    wu(this, ATTACH_QUAT + 4, 0);
    wu(this, ATTACH_QUAT + 8, 0);
    wu(this, ATTACH_OFFSET + 8, 0);
    if etype(this) == ty::OBJECT {
        let info = rp(this, 0x160);
        wf(this, ELASTICITY, X87::from_f32(rf(info, 0xC)).to_f32());
    }
}

/// `CPhysical::DettachEntityFromEntity(float x, float y, float z, bool bApplyTurnForce)` (0x5442F0,
/// `ret 0x10`).
///
/// Two vehicles: the col models are tested (`ProcessColModels` with the lines of this model hidden:
/// colData+6 zeroed and restored) and the attached-to vehicle becomes the ignored-collision entity
/// only when they still touch (cleared if it was it); without col data it is cleared when it was
/// it; any other pair: it becomes the ignored-collision entity. `m = attachedMatrix * (RotateZ(y),
/// RotateX(x))`, `v = m.forward * z` (f32). `CWorld::Remove`, `SetIsStatic(false)`, clear
/// `bAttachedToEntity`, `CWorld::Add`. `bDisableCollisionForce`: an object with an object-info mass
/// `< 99998` gets that mass and turn mass and loses the flag; otherwise it becomes collidable with
/// zero speeds and no turn force. Without `bDisableCollisionForce` the move speed becomes the
/// attached entity's (vehicle/ped/object) plus `v`. `ApplyTurnForce(v, 0.5*v)` when asked. The
/// attach state is cleared.
#[hook(addr = 0x5442F0, conv = thiscall, name = "CPhysical::DettachEntityFromEntity", expect = "6A FF 68 B3 C9 83 00", this = "CPhysical")]
pub fn dettach_entity_from_entity(this: Ent, x: u32, y: u32, z: f32, apply_turn: u8) {
    let att = rp(this, ATTACHED_TO);
    let mut apply_turn = apply_turn;
    if etype(this) == ty::VEHICLE && !att.is_null() && etype(att) == ty::VEHICLE {
        let col = get_col_model(this);
        let cd = rp(col, 0x2C);
        if !cd.is_null() {
            let saved = rb(cd, 6);
            wb(cd, 6, 0);
            let this_matrix = matrix(this);
            let att_col = get_col_model(att);
            let att_matrix = get_matrix(att);
            let n = process_col_models(
                this_matrix,
                col,
                att_matrix,
                att_col,
                GPtr::from_addr(TEMP_COL_PTS),
                GPtr::NULL,
                GPtr::NULL,
                0,
            );
            if n != 0 {
                wp(this, IGNORED_COLLISION, att);
            } else if rp(this, IGNORED_COLLISION) == att {
                wu(this, IGNORED_COLLISION, 0);
            }
            wb(cd, 6, saved);
        } else if rp(this, IGNORED_COLLISION) == att {
            wu(this, IGNORED_COLLISION, 0);
        }
    } else {
        wp(this, IGNORED_COLLISION, att);
    }
    let mut ma = local_matrix();
    let mut mb = local_matrix();
    matrix_reset_orientation(lp(&mut mb));
    matrix_rotate_z(lp(&mut mb), y);
    matrix_rotate_x(lp(&mut mb), x);
    ensure_matrix(att);
    matrix_assign(lp(&mut ma).cast(), matrix(att));
    matrix_mul_assign(lp(&mut ma).cast(), lp(&mut mb).cast());
    let mut f = Fpu::new();
    let mut v = [0f32; 3];
    for (i, v) in v.iter_mut().enumerate() {
        f.fld(f32::from_bits(ma[4 + i]));
        f.fmul_m(z);
        *v = f.fstp();
    }
    world_remove(this);
    set_is_static_v(this, 0);
    wu(this, PFLAGS, ru(this, PFLAGS) & 0xFDFF_FFFF);
    world_add(this);
    let fl = ru(this, PFLAGS);
    if fl & 4 != 0 {
        let info = rp(this, 0x160);
        let light = etype(this) == ty::OBJECT && {
            f.fld(rf(info, 0));
            f.fcomp_m(cf(k::K_863C08));
            !f.jp(5)
        };
        if light {
            wu(this, PFLAGS, fl & 0xFFFF_FFFB);
            wu(this, MASS, ru(info, 0));
            wu(this, TURN_MASS, ru(info, 4));
        } else {
            wv_bits(this, TURN_SPEED, [0; 3]);
            wu(this, PFLAGS, fl | 8);
            wv_bits(this, MOVE_SPEED, [0; 3]);
            apply_turn = 0;
        }
    }
    if pflags(this) & 4 == 0 {
        let a = rp(this, ATTACHED_TO);
        let t = etype(a);
        if t > 1 && t < 5 {
            wv_bits(this, MOVE_SPEED, rv_bits(a, MOVE_SPEED));
            for (i, v) in v.iter().enumerate() {
                let off = MOVE_SPEED + 4 * i as isize;
                f.fld(*v);
                f.fadd_m(rf(this, off));
                wf(this, off, f.fstp());
            }
        }
    }
    if apply_turn != 0 {
        let mut p = [0f32; 3];
        for (i, p) in p.iter_mut().enumerate() {
            f.fld(v[i]);
            f.fmul_m(cf(k::K_858B8C));
            *p = f.fstp();
        }
        apply_turn_force_call(this, cv(v), cv(p));
    }
    wu(this, ATTACHED_TO, 0);
    wu(this, ATTACH_QUAT + 0xC, 0);
    wu(this, ATTACH_QUAT, 0);
    wu(this, ATTACH_QUAT + 4, 0);
    wu(this, ATTACH_QUAT + 8, 0);
    wv_bits(this, ATTACH_OFFSET, [0; 3]);
    matrix_dtor(lp(&mut ma).cast());
    matrix_dtor(lp(&mut mb).cast());
}

/// `CPhysical::AttachEntityToEntity(CPhysical* entity, CVector offset, CVector rotation)` (0x54D570,
/// `ret 0x1C`, returns the entity or 0).
#[hook(addr = 0x54D570, conv = thiscall, name = "CPhysical::AttachEntityToEntity-vec", expect = "55 8B 6C 24 08", this = "CPhysical")]
pub fn attach_entity_to_entity_vec(this: Ent, entity: Ent, offset: CVector, rotation: CVector) -> Ent {
    if entity.is_null() {
        return GPtr::NULL;
    }
    wp(this, ATTACHED_TO, entity);
    register_reference(entity, this.byte_add(ATTACHED_TO).cast());
    wv_bits(this, ATTACH_OFFSET, bits(offset));
    let fl = ru(this, PFLAGS);
    let infinite = (fl >> 6) & 1;
    if infinite != 0 {
        wv_bits(this, ATTACH_ROTATION, rv_bits(pos_ptr(this), 0));
    } else {
        wv_bits(this, ATTACH_ROTATION, bits(rotation));
    }
    let att = rp(this, ATTACHED_TO);
    wu(this, ATTACH_QUAT + 0xC, 0);
    wu(this, ATTACH_QUAT, 0);
    wu(this, ATTACH_QUAT + 4, 0);
    wu(this, ATTACH_QUAT + 8, 0);
    wp(this, IGNORED_COLLISION, att);
    if fl & 4 != 0 {
        wu(this, PFLAGS, fl & 0xFFFF_FFF7);
    } else {
        let t = etype(entity);
        if t > 1 && t < 5 && rb(entity, PFLAGS) & 4 != 0 && etype(this) == ty::OBJECT && infinite == 0 {
            wu(this, PFLAGS, fl | 4);
            wu(this, MASS, MASS_NEARLY_INFINITE);
            wu(this, TURN_MASS, MASS_NEARLY_INFINITE);
        }
    }
    position_attached_entity(this);
    entity
}

/// `CPhysical::AttachEntityToEntity(CPhysical* entity, CVector* offset, CQuaternion* rotation)`
/// (stub 0x54D690, `ret 0xC`, returns the entity or 0). Without an offset it is the position
/// relative to the entity's matrix (transposed rotation of `pos - m.pos`; the forklift's (0x212)
/// fork frame matrix for an object); without a rotation the quaternion of `Invert(m) * matrix`.
#[hook(addr = 0x54D690, conv = thiscall, name = "CPhysical::AttachEntityToEntity-quat", expect = "90 E9 66 4C EB FF", this = "CPhysical")]
pub fn attach_entity_to_entity_quat(this: Ent, entity: Ent, offset: Ent, rotation: Ent) -> Ent {
    if entity.is_null() {
        return GPtr::NULL;
    }
    wp(this, ATTACHED_TO, entity);
    register_reference(entity, this.byte_add(ATTACHED_TO).cast());
    ensure_matrix(entity);
    let mut m = local_matrix();
    matrix_copy_ctor(lp(&mut m).cast(), matrix(entity));
    if etype(this) == ty::OBJECT && rh(entity, MODEL_INDEX) == 0x212 {
        let frame = rp(entity, 0x698);
        if !frame.is_null() {
            let ltm = rw_frame_get_ltm(frame);
            matrix_update_matrix(lp(&mut m).cast(), ltm);
        }
    }
    if !offset.is_null() {
        wv_bits(this, ATTACH_OFFSET, rv_bits(offset, 0));
    } else {
        let p = pos_ptr(this);
        let mf = |i: usize| f32::from_bits(m[i]);
        let mut f = Fpu::new();
        f.fld(rf(p, 8));
        f.fsub_m(mf(14));
        f.fld(rf(p, 4));
        f.fsub_m(mf(13));
        f.fld(rf(p, 0));
        f.fsub_m(mf(12));
        // st0 = dx, st1 = dy, st2 = dz
        f.fld(mf(1));
        f.fmul_st(2);
        f.fld(mf(2));
        f.fmul_st(4);
        f.faddp(1);
        f.fld_st(1);
        f.fmul_m(mf(0));
        f.faddp(1);
        wf(this, ATTACH_OFFSET, f.fstp());
        for (row, off) in [(4usize, 4isize), (8, 8)] {
            f.fld(mf(row));
            f.fmul_st(1);
            f.fld(mf(row + 1));
            f.fmul_st(3);
            f.faddp(1);
            f.fld(mf(row + 2));
            f.fmul_st(4);
            f.faddp(1);
            wf(this, ATTACH_OFFSET + off, f.fstp());
        }
        f.fstp_st(0);
        f.fstp_st(0);
        f.fstp_st(0);
    }
    if !rotation.is_null() {
        for i in 0..4 {
            wu(this, ATTACH_QUAT + 4 * i, ru(rotation, 4 * i));
        }
    } else {
        let mut m2 = local_matrix();
        let mut rw = [0u32; 16];
        matrix_invert(lp(&mut m).cast(), lp(&mut m2).cast());
        matrix_mul_assign(lp(&mut m2).cast(), matrix(this));
        matrix_copy_to_rw(lp(&mut m2).cast(), lp(&mut rw).cast());
        rt_quat_convert_from_matrix(this.byte_add(ATTACH_QUAT), lp(&mut rw).cast());
        matrix_dtor(lp(&mut m2).cast());
    }
    let fl = ru(this, PFLAGS);
    if fl & 4 != 0 {
        wu(this, PFLAGS, fl & 0xFFFF_FFF7);
    } else {
        let t = etype(entity);
        if t > 1 && t < 5 && rb(entity, PFLAGS) & 4 != 0 && etype(this) == ty::OBJECT && fl & 0x40 == 0 {
            wu(this, PFLAGS, fl | 4);
            wu(this, MASS, MASS_NEARLY_INFINITE);
            wu(this, TURN_MASS, MASS_NEARLY_INFINITE);
        }
    }
    if rb(this, PFLAGS) & 0x40 != 0 {
        wv_bits(this, ATTACH_ROTATION, rv_bits(pos_ptr(this), 0));
    }
    wp(this, IGNORED_COLLISION, rp(this, ATTACHED_TO));
    position_attached_entity(this);
    matrix_dtor(lp(&mut m).cast());
    entity
}

/// `CPhysical::PlacePhysicalRelativeToOtherPhysical(CPhysical* rel, CPhysical* toPlace, CVector
/// offset)` (0x546DB0, cdecl): `p = rel.matrix * offset + rel.moveSpeed * (timeStep * 0.9f)`
/// (x: `k*ms.x + p.x`, y/z: `p + stored k*ms`); `CWorld::Remove(toPlace)`, `toPlace.matrix =
/// rel.matrix` with position p (the placement when it has no matrix), its move speed `rel`'s, the
/// RW frame matrix updated, `UpdateRwFrame`, `CWorld::Add`.
#[hook(addr = 0x546DB0, conv = cdecl, name = "CPhysical::PlacePhysicalRelativeToOtherPhysical", expect = "83 EC 18 53 8B 5C 24 20")]
pub fn place_physical_relative_to_other_physical(rel: Ent, to_place: Ent, offset: CVector) {
    let mut off = offset;
    let mut o = CVector::new(0.0, 0.0, 0.0);
    let p = rv(matrix_mul_point(lp(&mut o), matrix(rel), lp(&mut off)).cast(), 0);
    let mut f = Fpu::new();
    f.fld(sa_core::timer::TIME_STEP.get());
    f.fmul_m(cf(k::K_858C20));
    f.fld_st(0);
    f.fmul_m(rf(rel, MOVE_SPEED));
    f.fld_st(1);
    f.fmul_m(rf(rel, MOVE_SPEED + 4));
    let ky = f.fstp();
    f.fxch(1);
    f.fmul_m(rf(rel, MOVE_SPEED + 8));
    let kz = f.fstp();
    f.fadd_m(p[0]);
    let px = f.fstp();
    f.fld(p[1]);
    f.fadd_m(ky);
    let py = f.fstp();
    f.fld(p[2]);
    f.fadd_m(kz);
    let pz = f.fstp();
    world_remove(to_place);
    matrix_assign(matrix(to_place), matrix(rel));
    let m = matrix(to_place);
    if !m.is_null() {
        wf(m, 0x30, px);
        wf(matrix(to_place), 0x34, py);
        wf(matrix(to_place), 0x38, pz);
    } else {
        wv(to_place, PLACEMENT_POS, [px, py, pz]);
    }
    wv_bits(to_place, MOVE_SPEED, rv_bits(rel, MOVE_SPEED));
    let rw = rp(to_place, RW_OBJECT);
    if !rw.is_null() {
        let frame_matrix = rp(rw, 4).byte_add(0x10);
        let m = matrix(to_place);
        if !m.is_null() {
            matrix_update_rw_matrix(m, frame_matrix);
        } else {
            simple_transform_update_rw_matrix(to_place.byte_add(PLACEMENT_POS), frame_matrix);
        }
    }
    update_rw_frame(to_place);
    world_add(to_place);
}

/// `CPhysical::ApplyScriptCollision(CVector normal, float elasticity, float friction, CVector*
/// point)` (0x546ED0, `ret 0x18`, ST0; spec CPhysical.md). The local `CColPoint`'s fields other
/// than point and normal are uninitialised stack in the original; here they are 0 (no original
/// caller: gta-reversed wrapper only).
#[hook(addr = 0x546ED0, conv = thiscall, name = "CPhysical::ApplyScriptCollision", expect = "83 EC 4C 8B 54 24 54", this = "CPhysical")]
pub fn apply_script_collision(this: Ent, normal: CVector, elasticity: u32, friction: u32, point: Ent) -> X87 {
    let normal = bits(normal);
    let saved = ru(this, ELASTICITY);
    let mut impulse: f32 = 0.0;
    let mut cp = [0u32; 11];
    cp[4] = normal[0];
    cp[5] = normal[1];
    cp[6] = normal[2];
    if !point.is_null() {
        let p = rv_bits(point, 0);
        cp[0] = p[0];
        cp[1] = p[1];
        cp[2] = p[2];
    } else {
        let pos = rv_bits(pos_ptr(this), 0);
        let r = rf(get_col_model(this), 0x24);
        let n = |i: usize| f32::from_bits(normal[i]);
        let mut f = Fpu::new();
        f.fld(r);
        f.fld(n(0));
        f.fmul_st(1);
        f.fld(n(1));
        f.fmul_st(2);
        let ty = f.fstp();
        f.fld(n(2));
        f.fmul_st(2);
        let tz = f.fstp();
        f.fld(f32::from_bits(pos[0]));
        f.fsub_st(1);
        cp[0] = f.fstp().to_bits();
        f.fstp_st(0);
        f.fstp_st(0);
        f.fld(f32::from_bits(pos[1]));
        f.fsub_m(ty);
        cp[1] = f.fstp().to_bits();
        f.fld(f32::from_bits(pos[2]));
        f.fsub_m(tz);
        cp[2] = f.fstp().to_bits();
    }
    wu(this, ELASTICITY, elasticity);
    let cpp: Ent = lp(&mut cp).cast();
    if apply_collision_call(this, this, cpp, lp(&mut impulse)) != 0 {
        let mut f = Fpu::new();
        f.fld(f32::from_bits(friction));
        f.fcomp_m(cf(k::K_858B50));
        if !f.jne(0x41) {
            apply_friction_self(this, f32::from_bits(friction), cpp);
        }
    }
    wu(this, ELASTICITY, saved);
    X87::from_f32(impulse)
}

/// `CPhysical::ProcessEntityCollision(CEntity* entity, CColPoint* colPoints)` (0x546D00, `ret 8`,
/// EAX = number of collision points): the entity gets a matrix if needed; `ProcessColModels(this,
/// entity)`; on contact both record the collision (a building does not) and a building, a static
/// entity (flag 4) or one with flag 0x40000 sets this entity's flag 0x1000.
#[hook(addr = 0x546D00, conv = thiscall, name = "CPhysical::ProcessEntityCollision", expect = "53 55 56 8B 74 24 10", this = "CPhysical")]
pub fn process_entity_collision(this: Ent, entity: Ent, cps: Ent) -> i32 {
    ensure_matrix(entity);
    let model = rh(this, MODEL_INDEX) as i16 as i32;
    let mi = sa_core::pools::MODEL_INFO_PTRS.ptr().cast::<GPtr<u8>>().wrapping_index(model).read();
    let this_matrix = matrix(this);
    let ent_matrix = matrix(entity);
    let this_col = rp(mi, 0x14);
    let ent_col = get_col_model(entity);
    let n = process_col_models(this_matrix, this_col, ent_matrix, ent_col, cps, GPtr::NULL, GPtr::NULL, 0);
    if n > 0 {
        add_collision_record_call(this, entity);
        if etype(entity) != ty::BUILDING {
            add_collision_record_call(entity, this);
        }
        let ef = ru(entity, EFLAGS);
        if etype(entity) == ty::BUILDING || ef & 4 != 0 || ef & 0x40000 != 0 {
            wu(this, EFLAGS, ru(this, EFLAGS) | 0x1000);
        }
    }
    n
}

/// `CPhysical::CheckCollision()` (0x54D920, AL): clears entity flag 2; a ped without attachment and
/// without physical flags 0x19000 loses its standing-on entity (+0x568) and moves its +0x46C bit 0
/// to bit 1; a climbing player (task climb state 1, 2, 3 or 5) sets physical flag 0x1000. Next scan
/// code (wraps through `ClearScanCodes`), then `ProcessCollisionSectorList` over the bound rect's
/// sectors (rows outer), true at the first hit.
#[hook(addr = 0x54D920, conv = thiscall, name = "CPhysical::CheckCollision", expect = "83 EC 18 53 55", this = "CPhysical")]
pub fn check_collision(this: Ent) -> u8 {
    wu(this, EFLAGS, ru(this, EFLAGS) & 0xFFFF_FFFD);
    if etype(this) == ty::PED {
        if rp(this, ATTACHED_TO).is_null() && ru(this, PFLAGS) & 0x19000 == 0 {
            let w = ru(this, 0x46C);
            wu(this, 0x568, 0);
            if w & 1 != 0 {
                wu(this, 0x46C, (ru(this, 0x46C) & 0xFFFF_FFFE) | 2);
            }
        }
        if ped_is_player(this) != 0 && !get_task_climb(rp(this, 0x47C)).is_null() {
            let t = get_task_climb(rp(this, 0x47C));
            let s = rb(t, 0xE);
            if s == 1 || s == 2 || s == 3 || s == 5 {
                wu(this, PFLAGS, ru(this, PFLAGS) | 0x1000);
            }
        }
    }
    next_scan_code();
    let mut rect = [0f32; 4];
    get_bound_rect_v(this, lp(&mut rect));
    let x0 = super::base::sector_of(rect[0]);
    let mut y = super::base::sector_of(rect[3]);
    let x1 = super::base::sector_of(rect[2]);
    let y1 = super::base::sector_of(rect[1]);
    while y <= y1 {
        let mut x = x0;
        while x <= x1 {
            if process_collision_sector_list(this, x, y) != 0 {
                return 1;
            }
            x += 1;
        }
        y += 1;
    }
    0
}

/// `if (ms_nCurrentScanCode < 0xFFFF) ms_nCurrentScanCode++; else { ClearScanCodes();
/// ms_nCurrentScanCode = 1; }`
fn next_scan_code() {
    let sc = GPtr::<u16>::from_addr(SCAN_CODE);
    if sc.read() < 0xFFFF {
        sc.write(sc.read().wrapping_add(1));
    } else {
        world_clear_scan_codes();
        sc.write(1);
    }
}

/// `CPhysical::CheckCollision_SimpleCar()` (0x54DAB0, AL): clears entity flag 2, next scan code,
/// `ProcessCollisionSectorList_SimpleCar` on the repeat sector of every collision-list node.
#[hook(addr = 0x54DAB0, conv = thiscall, name = "CPhysical::CheckCollision_SimpleCar", expect = "56 57 8B F9 83 67 1C FD", this = "CPhysical")]
pub fn check_collision_simple_car(this: Ent) -> u8 {
    wu(this, EFLAGS, ru(this, EFLAGS) & 0xFFFF_FFFD);
    next_scan_code();
    let mut node = rp(this, COLLISION_LIST);
    while !node.is_null() {
        if process_collision_sector_list_simple_car(this, rp(node, 8)) != 0 {
            return 1;
        }
        node = rp(node, 0x10);
    }
    0
}

/// `CPhysical::TestCollision(bool applySpeed)` (0x54DEC0, `ret 4`, AL): saves the matrix, sets
/// physical flags 0x11000, clears `bUsesCollision` (and a ped's +0x474 bit 0x8000), optionally
/// `ApplySpeed`, `CheckCollision`, then restores both flags, the matrix and the ped bit.
#[hook(addr = 0x54DEC0, conv = thiscall, name = "CPhysical::TestCollision", expect = "6A FF 68 88 CA 83 00", this = "CPhysical")]
pub fn test_collision(this: Ent, apply: u8) -> u8 {
    let mut m = local_matrix();
    matrix_copy_ctor(lp(&mut m).cast(), matrix(this));
    wu(this, PFLAGS, ru(this, PFLAGS) | 0x11000);
    let uses = ru(this, EFLAGS) & 1;
    wu(this, EFLAGS, ru(this, EFLAGS) & 0xFFFF_FFFE);
    let mut ped_bit = false;
    if etype(this) == ty::PED {
        let w = ru(this, 0x474);
        if w & 0x8000 != 0 {
            ped_bit = true;
            wu(this, 0x474, w & 0xFFFF_7FFF);
        }
    }
    if apply != 0 {
        apply_speed(this);
    }
    let r = check_collision_call(this);
    wu(this, EFLAGS, (ru(this, EFLAGS) & !1) | uses);
    wu(this, PFLAGS, ru(this, PFLAGS) & 0xFFFE_EFFF);
    matrix_assign(matrix(this), lp(&mut m).cast());
    if ped_bit {
        wu(this, 0x474, ru(this, 0x474) | 0x8000);
    }
    matrix_dtor(lp(&mut m).cast());
    r
}

/// `CPhysical::ProcessControl()` (stub-split 0x5485E0): not a ped: clears `bSubmergedInWater`;
/// entity flags `&= 0xFFFFEF97`; unless status 2: clears physical flags 0x40000200, the collided
/// count, piece type, damage intensity and damage entity (reference released), `ApplyFriction()`,
/// and without attachment (or with `bInfiniteMass`) `ApplyGravity()` + `ApplyAirResistance()`.
#[hook(addr = 0x5485E0, conv = thiscall, name = "CPhysical::ProcessControl", expect = "56 8B F1 8A 46 36", this = "CPhysical")]
pub fn process_control(this: Ent) {
    let t = rb(this, TYPE);
    if t & 7 != ty::PED {
        wu(this, PFLAGS, ru(this, PFLAGS) & 0xFFFF_FEFF);
    }
    wu(this, EFLAGS, ru(this, EFLAGS) & 0xFFFF_EF97);
    if t & 0xF8 == 0x10 {
        return;
    }
    wu(this, PFLAGS, ru(this, PFLAGS) & 0xBFFF_FDFF);
    wb(this, NUM_COLLIDED, 0);
    wh(this, PIECE_TYPE, 0);
    wu(this, DAMAGE_INTENSITY, 0);
    let de = rp(this, DAMAGE_ENTITY);
    if !de.is_null() {
        clean_up_old_reference(de, this.byte_add(DAMAGE_ENTITY).cast());
    }
    wu(this, DAMAGE_ENTITY, 0);
    apply_friction_void_call(this);
    if rp(this, ATTACHED_TO).is_null() || rb(this, PFLAGS) & 0x40 != 0 {
        apply_gravity_call(this);
        apply_air_resistance_call(this);
    }
}

/// `CPhysical::UnsetIsInSafePosition()` (0x548320): move and turn speeds negated (`* -1.0f`),
/// `ApplySpeed()`, negated back, entity flag 0x20 cleared.
#[hook(addr = 0x548320, conv = thiscall, name = "CPhysical::UnsetIsInSafePosition", expect = "56 8B F1 D9 46 44", this = "CPhysical")]
pub fn unset_is_in_safe_position(this: Ent) {
    let neg = |this: Ent| {
        for i in 0..6 {
            let off = MOVE_SPEED + 4 * i;
            let mut f = Fpu::new();
            f.fld(rf(this, off));
            f.fmul_m(cf(k::K_858C1C));
            wf(this, off, f.fstp());
        }
    };
    neg(this);
    apply_speed(this);
    neg(this);
    wu(this, EFLAGS, ru(this, EFLAGS) & 0xFFFF_FFDF);
}

/// `CPhysical::ApplyFriction()` (0x5483D0). With `bDisableZ`: `ApplyFriction(timeStep * 0.001f, cp)`
/// on a local `CColPoint` at `pos - (0*r, 0*r, r)` (r = the model's bounding radius) with normal
/// (0, 0, 1) — its other fields are uninitialised stack in the original, 0 here — then `turn.z *=
/// pow(0.98f, timeStep)`. Always: move and turn speeds += the friction speeds (friction first),
/// which are then zeroed. An abandoned bike-type (+0x590 == 9) vehicle not carried by rope, lying on
/// its side (`|up.z| < 0.707f`) and nearly at rest gets its move speed scaled by `pow(G_8CD7EC,
/// timeStep)`.
#[hook(addr = 0x5483D0, conv = thiscall, name = "CPhysical::ApplyFriction-void", expect = "83 EC 44 53 56", this = "CPhysical")]
pub fn apply_friction_void(this: Ent) {
    let mut f = Fpu::new();
    if rb(this, PFLAGS) & 0x80 != 0 {
        let model = rh(this, MODEL_INDEX) as i16 as i32;
        let mi = sa_core::pools::MODEL_INFO_PTRS.ptr().cast::<GPtr<u8>>().wrapping_index(model).read();
        let col = rp(mi, 0x14);
        f.fld(rf(col, 0x24));
        f.fld(cf(k::K_858B50));
        f.fmul_st(1);
        let t = f.fst();
        let t2 = f.fstp();
        let pos = pos_ptr(this);
        f.fld(rf(pos, 0));
        f.fsub_m(t);
        let px = f.fstp();
        f.fld(rf(pos, 4));
        f.fsub_m(t2);
        let py = f.fstp();
        f.fld(rf(pos, 8));
        f.fsub_st(1);
        let pz = f.fstp();
        f.fstp_st(0);
        let mut cp = [0u32; 11];
        cp[0] = px.to_bits();
        cp[1] = py.to_bits();
        cp[2] = pz.to_bits();
        cp[6] = 0x3F80_0000;
        f.fld(sa_core::timer::TIME_STEP.get());
        f.fmul_m(cf(k::K_863B5C));
        let fr = f.fstp();
        apply_friction_self(this, fr, lp(&mut cp).cast());
        let p = ci_pow(X87::from_f32(cf(k::K_863B68)), X87::from_f32(sa_core::timer::TIME_STEP.get()));
        f.push(p);
        f.fmul_m(rf(this, TURN_SPEED + 8));
        wf(this, TURN_SPEED + 8, f.fstp());
    }
    for (src, dst) in [(FRICTION_MOVE, MOVE_SPEED), (FRICTION_TURN, TURN_SPEED)] {
        for i in 0..3 {
            let o = 4 * i;
            f.fld(rf(this, src + o));
            f.fadd_m(rf(this, dst + o));
            wf(this, dst + o, f.fstp());
        }
    }
    wv_bits(this, FRICTION_MOVE, [0; 3]);
    wv_bits(this, FRICTION_TURN, [0; 3]);
    let t = rb(this, TYPE);
    if t & 7 == ty::VEHICLE && ru(this, 0x590) == 9 && (ru(this, PFLAGS) as i32) >= 0 && t & 0xF8 == 0x20 {
        f.fld(rf(matrix(this), 0x28));
        f.fabs();
        f.fcomp_m(cf(k::K_863C1C));
        if f.jp(5) {
            return;
        }
        let g = G_8CD7F4.get();
        f.push(X87::from_f64_exact(magnitude_sqr(this.byte_add(MOVE_SPEED))));
        f.fld(g);
        f.fmul_m(g);
        f.fcompp();
        if f.jne(0x41) {
            return;
        }
        let g = G_8CD7F0.get();
        f.push(X87::from_f64_exact(magnitude_sqr(this.byte_add(TURN_SPEED))));
        f.fld(g);
        f.fmul_m(g);
        f.fcompp();
        if f.jne(0x41) {
            return;
        }
        let p = ci_pow(X87::from_f32(G_8CD7EC.get()), X87::from_f32(sa_core::timer::TIME_STEP.get()));
        vector_scale_assign(this.byte_add(MOVE_SPEED), p.to_f32());
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_DETTACH_AUTO_ATTACHED_ENTITY,
    HOOK_DETTACH_ENTITY_FROM_ENTITY,
    HOOK_ATTACH_ENTITY_TO_ENTITY_VEC,
    HOOK_ATTACH_ENTITY_TO_ENTITY_QUAT,
    HOOK_PLACE_PHYSICAL_RELATIVE_TO_OTHER_PHYSICAL,
    HOOK_APPLY_SCRIPT_COLLISION,
    HOOK_PROCESS_ENTITY_COLLISION,
    HOOK_CHECK_COLLISION,
    HOOK_CHECK_COLLISION_SIMPLE_CAR,
    HOOK_TEST_COLLISION,
    HOOK_PROCESS_CONTROL,
    HOOK_UNSET_IS_IN_SAFE_POSITION,
    HOOK_APPLY_FRICTION_VOID,
];

orig! {
    /// `CMatrix::Reorthogonalise()` (0x59B6A0)
    pub fn matrix_reorthogonalise(m: Ent) = thiscall 0x59B6A0 => unported;
    /// `SetNextScanCode()` (0x4072E0)
    pub fn set_next_scan_code() = cdecl 0x4072E0 => unported;
    /// `CPhysical::ProcessShiftSectorList(int32 x, int32 y)` (0x546670)
    pub fn process_shift_sector_list(e: Ent, x: i32, y: i32) -> u8 = thiscall 0x546670 => crate::physical::mvpe_phys1::process_shift_sector_list;
    /// `CPhysical::RemoveAndAdd()` (0x542560)
    pub fn remove_and_add_call(e: Ent) = thiscall 0x542560 => crate::physical::base::remove_and_add;
}

sa_core::globals! {
    list GLOBALS;
    /// `CWorld::bSecondShift`
    pub static SECOND_SHIFT @ 0xB7CD6C: u8 = 0;
}

/// The sector rect of the bound rect, in the original's conversion order (x0 left, y0 bottom, x1
/// right, y1 top).
fn sector_rect(r: &[f32; 4]) -> (i32, i32, i32, i32) {
    use super::base::sector_of;
    (sector_of(r[0]), sector_of(r[3]), sector_of(r[2]), sector_of(r[1]))
}

/// `CPhysical::ProcessShift()` (0x54DB10, virtual).
///
/// Moving speed 0. Status 2 or flags 0xE0: the turn speed is zeroed when flags 0xE0, entity flags
/// `(& ~0x10) | 0x20`, `RemoveAndAdd`. Otherwise: with entity flag 0x1000 (a ped not standing on a
/// static non-collidable entity, or `CWorld::bSecondShift`) move and turn speed `*= pow(
/// PHYSICAL_SHIFT_SPEED_DAMP, timeStep)`; the matrix is saved, `ApplySpeed`, `Reorthogonalise`,
/// `SetNextScanCode`; vehicles set flag 0x8000 while `ProcessShiftSectorList` runs over the bound
/// rect's sectors. If one shifted (or for a vehicle) a second pass of `ProcessCollisionSectorList`
/// restores the saved matrix on a hit (at once unless `bSecondShift`). Otherwise the entity flags as
/// above, the moving speed `|pos - saved pos|` and `RemoveAndAdd`.
#[hook(addr = 0x54DB10, conv = thiscall, name = "CPhysical::ProcessShift", expect = "55 8B EC 83 E4 F8", this = "CPhysical")]
pub fn process_shift(this: Ent) {
    let mut rect = [0f32; 4];
    get_bound_rect_v(this, lp(&mut rect));
    let t = rb(this, TYPE);
    wu(this, MOVING_SPEED, 0);
    if t & 0xF8 == 0x10 || rb(this, PFLAGS) & 0xE0 != 0 {
        if rb(this, PFLAGS) & 0xE0 != 0 {
            wv_bits(this, TURN_SPEED, [0; 3]);
        }
        wu(this, EFLAGS, (ru(this, EFLAGS) & 0xFFFF_FFEF) | 0x20);
        remove_and_add_call(this);
        return;
    }
    if (ru(this, EFLAGS) >> 12) & 1 != 0 {
        let damp = if t & 7 == ty::PED {
            let s = rp(this, 0x568);
            if s.is_null() {
                true
            } else {
                let sf = ru(s, PFLAGS);
                if sf & 4 == 0 || sf & 8 != 0 { true } else { SECOND_SHIFT.get() != 0 }
            }
        } else {
            SECOND_SHIFT.get() != 0
        };
        if damp {
            for off in [MOVE_SPEED, TURN_SPEED] {
                let p = ci_pow(
                    X87::from_f32(PHYSICAL_SHIFT_SPEED_DAMP.get()),
                    X87::from_f32(sa_core::timer::TIME_STEP.get()),
                );
                vector_scale_assign(this.byte_add(off), p.to_f32());
            }
        }
    }
    let mut saved = local_matrix();
    matrix_copy_ctor(lp(&mut saved).cast(), matrix(this));
    apply_speed(this);
    matrix_reorthogonalise(matrix(this));
    set_next_scan_code();
    let mut hit = false;
    if etype(this) == ty::VEHICLE {
        wu(this, PFLAGS, ru(this, PFLAGS) | 0x8000);
    }
    let (x0, y0, x1, y1) = sector_rect(&rect);
    let mut y = y0;
    while y <= y1 {
        let mut x = x0;
        while x <= x1 {
            if process_shift_sector_list(this, x, y) != 0 {
                hit = true;
            }
            x += 1;
        }
        y += 1;
    }
    wu(this, PFLAGS, ru(this, PFLAGS) & 0xFFFF_7FFF);
    if hit || etype(this) == ty::VEHICLE {
        next_scan_code();
        hit = false;
        let (x0, y0, x1, y1) = sector_rect(&rect);
        let mut y = y0;
        while y <= y1 {
            let mut x = x0;
            while x <= x1 {
                if process_collision_sector_list(this, x, y) != 0 {
                    if SECOND_SHIFT.get() == 0 {
                        matrix_assign(matrix(this), lp(&mut saved).cast());
                        matrix_dtor(lp(&mut saved).cast());
                        return;
                    }
                    hit = true;
                }
                x += 1;
            }
            y += 1;
        }
        if hit {
            matrix_assign(matrix(this), lp(&mut saved).cast());
            matrix_dtor(lp(&mut saved).cast());
            return;
        }
    }
    wu(this, EFLAGS, (ru(this, EFLAGS) & 0xFFFF_FFEF) | 0x20);
    let m = matrix(this);
    let sp = |i: usize| f32::from_bits(saved[12 + i]);
    let mut f = Fpu::new();
    f.fld(rf(m, 0x38));
    f.fsub_m(sp(2));
    f.fld(rf(m, 0x34));
    f.fsub_m(sp(1));
    f.fld(rf(m, 0x30));
    f.fsub_m(sp(0));
    f.fld_st(0);
    f.fmul_st(1);
    f.fld_st(2);
    f.fmul_st(3);
    f.faddp(1);
    f.fld_st(3);
    f.fmul_st(4);
    f.faddp(1);
    f.fstp_st(3);
    f.fstp_st(0);
    f.fstp_st(0);
    f.fsqrt();
    wf(this, MOVING_SPEED, f.fstp());
    remove_and_add_call(this);
    matrix_dtor(lp(&mut saved).cast());
}

pub const HOOKS_2: &[HookReg] = &[HOOK_PROCESS_SHIFT];

sa_core::globals! {
    list GLOBALS_SPEED;
    /// 0x8CDEF4 (2497.7925; ApplySpeed's x upper bound)
    pub static SPEED_X_MAX @ 0x8CDEF4: f32 = f32::from_bits(0x451C1CAE);
    /// 0x8CDEF8 (-1670.4; y upper bound)
    pub static SPEED_Y_MAX @ 0x8CDEF8: f32 = f32::from_bits(0xC4D0CCCC);
    /// 0x8CDF00 (2495.8525; x lower bound)
    pub static SPEED_X_MIN @ 0x8CDF00: f32 = f32::from_bits(0x451BFDA4);
    /// 0x8CDF04 (-1671.41; y lower bound)
    pub static SPEED_Y_MIN @ 0x8CDF04: f32 = f32::from_bits(0xC4D0ED1E);
    /// 0x8CDF08 (12.9; z limit)
    pub static SPEED_Z_LIMIT @ 0x8CDF08: f32 = f32::from_bits(0x414E6666);
}

orig! {
    /// `CTimer::UpdateTimeStep(float)` (0x541F40)
    pub fn update_time_step(ts: u32) = cdecl 0x541F40 => crate::physical::base::update_time_step;
    /// `CPhysical::ApplyMoveSpeed()` (0x542DD0)
    pub fn apply_move_speed_call(e: Ent) = thiscall 0x542DD0 => crate::physical::forces::apply_move_speed;
    /// `CPhysical::ApplyTurnSpeed()` (0x542E20)
    pub fn apply_turn_speed_call(e: Ent) = thiscall 0x542E20 => crate::physical::forces::apply_turn_speed;
    /// `CAudioEngine::ReportMissionAudioEvent(uint16, CObject*)` (0x507350, thiscall on AudioEngine)
    pub fn report_mission_audio_event(ae: Ent, ev: u32, obj: Ent) = thiscall 0x507350 => unported;
    /// `CPlaceable::GetHeading()` (0x441DB0, ST0)
    pub fn get_heading(e: Ent) -> f64 = thiscall 0x441DB0 => unported;
}

/// `TIME_STEP = saved > 1e-5f (or NaN) ? saved : 1e-5f` (`fld; fld; fcomp; jp; fstp`).
fn restore_time_step(saved: f32) {
    let mut f = Fpu::new();
    f.fld(saved);
    f.fld(saved);
    f.fcomp_m(cf(k::K_858C14));
    if !f.jp(0x41) {
        f.fstp_st(0);
        f.fld(cf(k::K_858C14));
    }
    sa_core::timer::TIME_STEP.set(f.fstp());
}

/// One axis of ApplySpeed's bounded-door path (0x547D49 for x, 0x547EA5 for y): `t` the time to the
/// bound; the other axis' position is checked against its bounds (bounce) or the component flips
/// (x: flips only when `bl`; y: when `!bl`). Returns `saved - t` (f32), the time left.
fn speed_axis(this: Ent, axis: isize, t: f32, saved: f32, bl: bool, v: &mut [f32; 3], a: &mut f32) -> f32 {
    let ms_off = MOVE_SPEED + 4 * axis;
    let mut f = Fpu::new();
    v[axis as usize] = -1.0;
    f.fld(rf(this, ms_off));
    f.fabs();
    *a = f.fstp();
    f.fld(rf(this, ms_off));
    f.fcomp_m(cf(k::K_858B50));
    if f.jne(0x41) {
        v[axis as usize] = 1.0;
    }
    update_time_step(t.to_bits());
    apply_move_speed_call(this);
    apply_turn_speed_call(this);
    let (hi, lo, poff) = if axis == 0 {
        (SPEED_Y_MAX.get(), SPEED_Y_MIN.get(), 0x34)
    } else {
        (SPEED_X_MAX.get(), SPEED_X_MIN.get(), 0x30)
    };
    let p = rf(matrix(this), poff);
    let d = cf(k::K_863B64);
    let mut bounce = false;
    f.fld(hi);
    f.fsub_m(d);
    f.fcomp_m(p);
    if !f.jp(5) {
        bounce = true;
    } else {
        f.fld(d);
        f.fadd_m(lo);
        f.fcomp_m(p);
        if !f.jne(0x41) {
            bounce = true;
        } else if (axis == 0 && !bl) || (axis == 1 && bl) {
            f.fld(lo);
            f.fadd_m(hi);
            f.fmul_m(cf(k::K_858B8C));
            f.fld(d);
            f.fsubr_st(1);
            f.fcomp_m(p);
            if !f.jp(5) {
                f.fld(d);
                f.fadd_st(1);
                f.fld(p);
                f.fcompp();
                bounce = !f.jp(5);
            }
            f.fstp_st(0);
        }
    }
    if bounce {
        f.fld(t);
        f.fmul_m(rf(this, ms_off));
        wu(this, PFLAGS, ru(this, PFLAGS) | 2);
        f.fcom_m(cf(k::K_863B6C));
        if !f.jne(0x41) {
            f.fstp_st(0);
            f.fld(cf(k::K_863B6C));
            f.fdiv_m(t);
            wf(this, ms_off, f.fstp());
        } else {
            f.fld(cf(k::K_863B6C));
            f.fchs();
            f.fxch(1);
            f.fcompp();
            if !f.jp(5) {
                f.fld(cf(k::K_863B6C));
                f.fdiv_m(t);
                f.fchs();
                wf(this, ms_off, f.fstp());
            }
        }
    } else {
        f.fld(rf(this, ms_off));
        f.fmul_m(cf(k::K_858C1C));
        wf(this, ms_off, f.fstp());
    }
    f.fld(saved);
    f.fsub_m(t);
    f.fstp()
}

/// The bounded-door path of ApplySpeed (`bDisableZ` without `bApplyGravity`, 0x547C2E..0x5480AD).
fn speed_door(this: Ent, saved: f32) {
    let mut f = Fpu::new();
    let m = matrix(this);
    let ts = sa_core::timer::TIME_STEP.get();
    let mut tt = [1000.0f32; 2];
    for axis in 0..2isize {
        let ms = rf(this, MOVE_SPEED + 4 * axis);
        let p = rf(m, 0x30 + 4 * axis);
        let (hi, lo) = if axis == 0 {
            (SPEED_X_MAX.get(), SPEED_X_MIN.get())
        } else {
            (SPEED_Y_MAX.get(), SPEED_Y_MIN.get())
        };
        f.fld(ts);
        f.fmul_m(ms);
        f.fadd_m(p);
        let mut bound = None;
        f.fcom_m(hi);
        if !f.jne(0x41) {
            f.fld(ms);
            f.fcomp_m(cf(k::K_858B50));
            if !f.jne(0x41) {
                f.fstp_st(0);
                bound = Some(hi);
            }
        }
        if bound.is_none() {
            f.fcomp_m(lo);
            if !f.jp(5) {
                f.fld(ms);
                f.fcomp_m(cf(k::K_858B50));
                if !f.jp(5) {
                    bound = Some(lo);
                }
            }
        }
        if let Some(b) = bound {
            f.fld(b);
            f.fsub_m(p);
            f.fdiv_m(ms);
            tt[axis as usize] = f.fstp();
        }
    }
    let (tx, ty) = (tt[0], tt[1]);
    f.fld(SPEED_Y_MAX.get());
    f.fsub_m(SPEED_Y_MIN.get());
    f.fld(SPEED_X_MAX.get());
    f.fsub_m(SPEED_X_MIN.get());
    f.fcompp();
    let bl = f.jp(5);
    let mut v = [0f32; 3];
    let mut a = 0f32;
    f.fld(tx);
    f.fcomp_m(ty);
    let mut x_first = false;
    if !f.jp(5) {
        f.fld(tx);
        f.fcomp_m(cf(k::K_858C4C));
        x_first = !f.jp(5);
    }
    let rest = if x_first {
        speed_axis(this, 0, tx, saved, bl, &mut v, &mut a)
    } else {
        f.fld(ty);
        f.fcomp_m(cf(k::K_858C4C));
        if f.jp(5) {
            return;
        }
        speed_axis(this, 1, ty, saved, bl, &mut v, &mut a)
    };
    update_time_step(rest.to_bits());
    f.fld(a);
    f.fcomp_m(cf(k::K_858B50));
    if f.jne(0x41) {
        return;
    }
    let model = rh(this, MODEL_INDEX) as i16 as i32;
    let mi = sa_core::pools::MODEL_INFO_PTRS.ptr().cast::<GPtr<u8>>().wrapping_index(model).read();
    let rad = rf(rp(mi, 0x14), 0x24);
    f.fld(rad);
    f.fld(v[0]);
    f.fmul_st(1);
    let ox = f.fstp();
    f.fld(v[1]);
    f.fmul_st(1);
    let oy = f.fstp();
    f.fmul_m(cf(k::K_858B50));
    let pos = pos_ptr(this);
    f.fld(rf(pos, 0));
    f.fsub_m(ox);
    let px = f.fstp();
    f.fld(rf(pos, 4));
    f.fsub_m(oy);
    let py = f.fstp();
    f.fld(rf(pos, 8));
    f.fsub_st(1);
    let pz = f.fstp();
    f.fstp_st(0);
    f.fld(cf(k::K_863B60));
    f.fmul_m(a);
    let fr = f.fstp();
    let mut cp = [0u32; 11];
    cp[0] = px.to_bits();
    cp[1] = py.to_bits();
    cp[2] = pz.to_bits();
    cp[4] = v[0].to_bits();
    cp[5] = v[1].to_bits();
    cp[6] = v[2].to_bits();
    apply_friction_self(this, fr, lp(&mut cp).cast());
    if etype(this) == ty::OBJECT {
        report_mission_audio_event(GPtr::from_addr(crate::physical::collide::AUDIO_ENGINE), 0x3F8, this);
        let b = rb(this, 0x148);
        wb(this, 0x148, if b == 0xFF { 0x36 } else { 0x32 });
    }
}

/// `CPhysical::ApplySpeed()` (0x547B80).
///
/// `bDisableZ` physicals: with `bApplyGravity` a z limit (pos.z clamped to 12.9 with the speeds
/// zeroed when the step would cross it); otherwise the bounded door of the bytes (bounds at
/// 0x8CDEF4..0x8CDF04): the time to the first x/y bound, a partial `ApplyMoveSpeed` +
/// `ApplyTurnSpeed` up to it, then a bounce (speed ±0.03/t) or a flip of that component, and with the
/// rest of the step a friction contact (`ApplyFriction(10*|v|, cp)` at the bound with the axis
/// normal; cp's other fields are uninitialised stack in the original, 0 here) and the object audio
/// event 1016. Then `ApplyMoveSpeed` + `ApplyTurnSpeed` and the time step restored (at least 1e-5).
/// `bDisableMoveForce` objects with a rest heading (+0x158 > -1000) turn only up to `rest ±
/// G_8CD7E8` (a partial step, turn.z damped by `G_8CD7E4`, flag 0x40000000); with object flag 0x4000
/// a turn that crosses the rest heading stops. The heading is `atan2(-fwd.x, fwd.y)` (FPATAN, the f64
/// model of sa-math, stored as f32).
#[hook(addr = 0x547B80, conv = thiscall, name = "CPhysical::ApplySpeed", expect = "83 EC 60 A1 5C CB B7 00", this = "CPhysical")]
pub fn apply_speed_port(this: Ent) {
    let saved_bits = sa_core::timer::TIME_STEP.ptr().cast::<u32>().read();
    let saved = f32::from_bits(saved_bits);
    let fl = ru(this, PFLAGS);
    let mut f = Fpu::new();
    if fl & 0x80 != 0 {
        if fl & 2 != 0 {
            let m = matrix(this);
            f.fld(sa_core::timer::TIME_STEP.get());
            f.fmul_m(rf(this, MOVE_SPEED + 8));
            f.fadd_m(rf(m, 0x38));
            f.fcomp_m(SPEED_Z_LIMIT.get());
            if !f.jp(5) {
                wu(m, 0x38, SPEED_Z_LIMIT.ptr().cast::<u32>().read());
                wv_bits(this, MOVE_SPEED, [0; 3]);
                wv_bits(this, TURN_SPEED, [0; 3]);
            }
        } else {
            speed_door(this, saved);
        }
        apply_move_speed_call(this);
        apply_turn_speed_call(this);
        restore_time_step(saved);
        return;
    }
    if fl & 0x20 == 0 || etype(this) != ty::OBJECT {
        apply_move_speed_call(this);
        apply_turn_speed_call(this);
        restore_time_step(saved);
        return;
    }
    f.fld(rf(this, 0x158));
    f.fcomp_m(cf(k::K_859948));
    if f.jne(0x41) {
        apply_move_speed_call(this);
        apply_turn_speed_call(this);
        restore_time_step(saved);
        return;
    }
    let rest = rf(this, 0x158);
    let m = matrix(this);
    let mut h = if !m.is_null() {
        f.fld(rf(m, 0x10));
        f.fchs();
        f.fld(rf(m, 0x14));
        let x = f.pop();
        let y = f.pop();
        y.fpatan(x).to_f32()
    } else {
        rf(this, 0x10)
    };
    f.fld(rest);
    f.fadd_m(cf(k::K_858CB8));
    f.fcomp_m(h);
    if !f.jp(5) {
        f.fld(h);
        f.fsub_m(cf(k::K_858CBC));
        h = f.fstp();
    } else {
        f.fld(rest);
        f.fsub_m(cf(k::K_858CB8));
        f.fcomp_m(h);
        if !f.jne(0x41) {
            f.fld(h);
            f.fadd_m(cf(k::K_858CBC));
            h = f.fstp();
        }
    }
    let mut ts2 = -1000.0f32;
    let ts = sa_core::timer::TIME_STEP.get();
    let tz = rf(this, TURN_SPEED + 8);
    f.fld(tz);
    f.fcomp_m(cf(k::K_858B50));
    let mut handled = false;
    if !f.jne(0x41) {
        f.fld(G_8CD7E8.get());
        f.fadd_m(rest);
        f.fld(ts);
        f.fmul_m(tz);
        f.fadd_m(h);
        f.fcomp_st(1);
        if !f.jne(0x41) {
            f.fsub_m(h);
            f.fdiv_m(tz);
            ts2 = f.fstp();
            handled = true;
        } else {
            f.fstp_st(0);
        }
    }
    if !handled {
        f.fld(tz);
        f.fcomp_m(cf(k::K_858B50));
        if !f.jp(5) {
            f.fld(rest);
            f.fsub_m(G_8CD7E8.get());
            f.fld(ts);
            f.fmul_m(tz);
            f.fadd_m(h);
            f.fcomp_st(1);
            if !f.jp(5) {
                f.fsub_m(h);
                f.fdiv_m(tz);
                ts2 = f.fstp();
            } else {
                f.fstp_st(0);
            }
        }
    }
    f.fld(ts);
    f.fchs();
    f.fcomp_m(ts2);
    if !f.jp(0x41) {
        update_time_step(ts2.to_bits());
        apply_turn_speed_call(this);
        f.fld(G_8CD7E4.get());
        f.fmul_m(rf(this, TURN_SPEED + 8));
        wf(this, TURN_SPEED + 8, f.fstp());
        f.fld(saved);
        f.fsub_m(ts2);
        update_time_step(f.fstp().to_bits());
        wu(this, PFLAGS, ru(this, PFLAGS) | 0x4000_0000);
    }
    apply_move_speed_call(this);
    apply_turn_speed_call(this);
    update_time_step(saved_bits);
    if ru(this, 0x140) & 0x4000 == 0 {
        return;
    }
    f.push(X87::from_f64_exact(get_heading(this)));
    f.fld(cf(k::K_858CB8));
    f.fadd_st(1);
    f.fcomp_m(rest);
    if !f.jp(5) {
        f.fadd_m(cf(k::K_858CBC));
    } else {
        f.fld(cf(k::K_858CB8));
        f.fsubr_st(1);
        f.fcomp_m(rest);
        if !f.jne(0x41) {
            f.fsub_m(cf(k::K_858CBC));
        }
    }
    f.fld(h);
    f.fsub_m(rest);
    let mut d1 = f.fstp();
    f.fsub_m(rest);
    f.fld(d1);
    f.fabs();
    f.fcomp_m(cf(k::K_858CDC));
    if !f.jp(5) {
        d1 = 0.0;
    }
    f.fld_st(0);
    f.fabs();
    f.fcomp_m(cf(k::K_858CDC));
    if !f.jp(5) {
        f.fstp_st(0);
        f.fld(cf(k::K_858B50));
    }
    f.fld(d1);
    f.fmul_st(1);
    f.fcomp_m(cf(k::K_858B50));
    f.fstp_st(0);
    if !f.jp(5) {
        wu(this, TURN_SPEED + 8, 0);
    }
}

pub const HOOKS_3: &[HookReg] = &[HOOK_APPLY_SPEED_PORT];
