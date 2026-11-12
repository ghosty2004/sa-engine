//! `CPedAttractorPedPlacer::PlacePedAtEffect` (0x5EA390, batch `writerq-ai-1`): puts a ped on a ped-attractor effect,
//! facing its use direction. Ported from the 1.0 US bytes (gta-reversed as a reading aid). Unverified
//! (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::{hook, orig};

use crate::common::*;

orig! {
    /// `CMatrix::SetUnity()` (0x59AE70)
    fn matrix_set_unity(m: GPtr<u8>) = thiscall 0x59AE70 => unported;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0, `ret 4`)
    fn matrix_assign(dst: GPtr<u8>, src: GPtr<u8>) = thiscall 0x59BBC0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    fn placeable_allocate_matrix(p: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0, `ret 4`)
    fn simple_transform_update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CVector::FromMultiply(const CMatrix&, const CVector&)` (0x59C670, `ret 8`): `this = m * v` (with position)
    fn vector_from_multiply(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) = thiscall 0x59C670 => unported;
    /// `Multiply3x3(const CMatrix&, const CVector&)` (0x59C790, cdecl, result through the first argument)
    fn multiply3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => unported;
    /// `CPedPlacement::FindZCoorForPed(CVector&)` (0x616920)
    fn find_z_coor_for_ped(v: GPtr<u8>) -> u8 = cdecl 0x616920 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(float x1, float y1, float x2, float y2)` (0x53CBE0, ST0)
    fn radian_angle_between_points(x1: u32, y1: u32, x2: u32, y2: u32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    fn limit_radian_angle(a: u32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CMatrix::SetRotateZOnly(float)` (0x59B020, `ret 4`)
    fn matrix_set_rotate_z_only(m: GPtr<u8>, a: u32) = thiscall 0x59B020 => unported;
}

fn pa<const N: usize>(v: &mut [u32; N]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}

/// `LimitRadianAngle(f32(GetRadianAngleBetweenPoints(x, y, 0, 0)))` stored as f32.
fn use_angle(dx: u32, dy: u32) -> f32 {
    let a = f32_of(X87::from_f64_exact(radian_angle_between_points(dx, dy, 0, 0)));
    f32_of(X87::from_f64_exact(limit_radian_angle(a.to_bits())))
}

/// `CPedAttractorPedPlacer::PlacePedAtEffect(const C2dEffectPedAttractor& fx, CEntity* entity, CPed* ped, float
/// forward)` (0x5EA390, cdecl, SEH frame for the local `CMatrix`). The local matrix (attached matrix 0, not owned) is
/// `SetUnity` without an entity, else a copy of the entity's matrix (allocated and updated first when it has none).
/// pos = matrix * fx position (`FromMultiply`); use = `Multiply3x3(matrix, fx +0x1C)`; f = `Multiply3x3(matrix, fx
/// +0x28)`; `Multiply3x3(matrix, fx +0x10)` too (result unused). p = (f32(pos.x + f.x*forward), f32(pos.y +
/// f.y*forward), f32(pos.z + f32(f.z*forward))); `FindZCoorForPed(p)`; the ped's position = p (through `fld`/`fstp`
/// for y and z when it has a matrix). angle = `LimitRadianAngle(GetRadianAngleBetweenPoints(use.x, use.y, 0, 0))`:
/// +0x55C and +0x558 = angle, then `SetRotateZOnly(angle)` on the ped's matrix or its heading (+0x10) = angle; the
/// angle is computed a second time and stored to +0x55C and +0x558 again. `~CMatrix` on the local.
#[hook(addr = 0x5EA390, conv = cdecl, name = "CPedAttractorPedPlacer::PlacePedAtEffect", expect = "6A FF 68 78 DA 83 00",
       unverified = "writerq-ai-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "writes the ped's matrix (heap) and its position / rotation fields")]
pub fn place_ped_at_effect(fx: GPtr<u8>, entity: GPtr<u8>, ped: GPtr<u8>, forward: f32) {
    let mut mat = [0u32; 0x48 / 4];
    if entity.is_null() {
        matrix_set_unity(pa(&mut mat));
    } else {
        if rd::<u32>(entity, 0x14) == 0 {
            placeable_allocate_matrix(entity);
            simple_transform_update_matrix(entity.byte_add(4), rd::<GPtr<u8>>(entity, 0x14));
        }
        matrix_assign(pa(&mut mat), rd::<GPtr<u8>>(entity, 0x14));
    }
    let mut pos = [0u32; 3];
    vector_from_multiply(pa(&mut pos), pa(&mut mat), fx);
    let mut v = [rd::<u32>(fx, 0x1C), rd::<u32>(fx, 0x20), rd::<u32>(fx, 0x24)];
    let mut out = [0u32; 3];
    let r = multiply3x3(pa(&mut out), pa(&mut mat), pa(&mut v));
    let use_dir = [rd::<u32>(r, 0), rd::<u32>(r, 4)];
    let mut v = [rd::<u32>(fx, 0x28), rd::<u32>(fx, 0x2C), rd::<u32>(fx, 0x30)];
    let mut out2 = [0u32; 3];
    let r = multiply3x3(pa(&mut out2), pa(&mut mat), pa(&mut v));
    let f = [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)];
    let mut v = [rd::<u32>(fx, 0x10), rd::<u32>(fx, 0x14), rd::<u32>(fx, 0x18)];
    let mut out3 = [0u32; 3];
    multiply3x3(pa(&mut out3), pa(&mut mat), pa(&mut v));
    let fz = x(f[2]).fmul_m(forward).to_f32();
    let pf = |i: usize| f32::from_bits(pos[i]);
    let mut p = [
        x(pf(0)).fadd(x(f[0]).fmul_m(forward)).to_f32().to_bits(),
        x(pf(1)).fadd(x(f[1]).fmul_m(forward)).to_f32().to_bits(),
        x(pf(2)).fadd_m(fz).to_f32().to_bits(),
    ];
    find_z_coor_for_ped(pa(&mut p));
    let m = rd::<GPtr<u8>>(ped, 0x14);
    if m.is_null() {
        wr::<u32>(ped, 4, p[0]);
        wr::<u32>(ped, 8, p[1]);
        wr::<u32>(ped, 0xC, p[2]);
    } else {
        wr::<u32>(m, 0x30, p[0]);
        wr::<f32>(rd::<GPtr<u8>>(ped, 0x14), 0x34, x(f32::from_bits(p[1])).to_f32());
        wr::<f32>(rd::<GPtr<u8>>(ped, 0x14), 0x38, x(f32::from_bits(p[2])).to_f32());
    }
    let a = use_angle(use_dir[0], use_dir[1]);
    wr::<f32>(ped, 0x55C, a);
    wr::<f32>(ped, 0x558, a);
    let m = rd::<GPtr<u8>>(ped, 0x14);
    if m.is_null() {
        wr::<f32>(ped, 0x10, a);
    } else {
        matrix_set_rotate_z_only(m, a.to_bits());
    }
    let a = use_angle(use_dir[0], use_dir[1]);
    wr::<f32>(ped, 0x55C, a);
    wr::<f32>(ped, 0x558, a);
    matrix_dtor(pa(&mut mat));
}

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[HOOK_PLACE_PED_AT_EFFECT];
