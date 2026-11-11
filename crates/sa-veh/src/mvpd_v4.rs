//! WRITER-MVP-D (helper VEH, batch `mvpd-veh-4`): vehicle light and particle effects. Ported from the
//! 1.0 US bytes; gta-reversed as a reading aid. Where they differ, the bytes win.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::common::{f32_of, ftol32, g, model_info, rd, wr, x};

orig! {
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, hidden result pointer)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CVector::NormaliseAndMag()` (0x59C970, ST0)
    fn normalise_and_mag(v: GPtr<u8>) -> f64 = thiscall 0x59C970 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn normalise(v: GPtr<CVector>) = thiscall 0x59C910 => sa_world::collision::data::normalise_port;
    /// `CVector::CrossProduct(CVector* out, const CVector* a, const CVector* b)` (0x59C730)
    fn cross_product(out: GPtr<CVector>, a: GPtr<CVector>, b: GPtr<CVector>) -> GPtr<CVector>
        = cdecl 0x59C730 => sa_world::collision::data::cross_product_port;
    /// `RwRenderStateSet` (RwEngineInstance +0x20)
    fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwIm3DTransform(RxObjSpace3DVertex*, uint32, RwMatrix*, uint32)` (0x7EF450)
    fn rw_im3d_transform(verts: GPtr<u8>, num: u32, m: u32, flags: u32) -> u32 = cdecl 0x7EF450 => unported;
    /// `RwIm3DRenderIndexedPrimitive(RwPrimitiveType, RwImVertexIndex*, int32)` (0x7EF550)
    fn rw_im3d_render_indexed_primitive(prim: u32, indices: GPtr<u8>, num: i32) -> u32 = cdecl 0x7EF550 => unported;
    /// `RwIm3DEnd()` (0x7EF520)
    fn rw_im3d_end() -> u32 = cdecl 0x7EF520 => unported;
}

/// `TempBufferVertices` (0xC4D958, `RxObjSpace3DVertex` stride 0x24) and its counters.
const TEMP_VERTS: usize = 0xC4_D958;
const TEMP_INDICES: usize = 0xC4_B958;
const TEMP_NUM_VERTS: usize = 0xC4_B950;
const TEMP_NUM_INDICES: usize = 0xC4_B954;

fn vf(p: GPtr<u8>, i: isize) -> f32 {
    rd::<f32>(p, 4 * i)
}

fn put(addr: usize, v: u32) {
    g::<u32>(addr).write(v);
}

fn putf(addr: usize, v: X87) {
    g::<f32>(addr).write(f32_of(v));
}

/// `CVehicle::DoHeadLightBeam(int32 lightId, CMatrix& matrix, bool isRight)` (0x6E0E20, `ret 0xC`).
///
/// The light dummy (model info +0x5C, + 24 * lightId); light 1 with a zero dummy stops. `P = matrix
/// * dummy`, for the left light minus `2 * dummy.x * right`. `D = normalised(camera - P)` (TheCamera
/// matrix 0xB6F03C, placement 0xB6F02C, `NormaliseAndMag`); alpha = `ftol((1 - |D . fwd|) * 32)`.
/// Render states (zwrite 0, ztest 1, vertex alpha 1, src blend 5, dst blend 2, shade 2, raster 0,
/// cull 1, alpha func 5, alpha ref 0). `N = normalised(fwd - up * 0.15)` (0.5 for the forklift
/// 0x212), `R = normalised(N x D)`, `L = P - fwd * 0.1`; the 5 temp vertices `L - R*0.05`, `L +
/// R*0.05`, `L + N*3 - R*0.5`, `L + N*3 + R*0.5`, `L + N*0.2` (colours white, alpha a, a, 0, 0, a),
/// 12 indices 0,1,4, 1,3,4, 2,3,4, 0,2,4; `RwIm3DTransform(verts, 5, null, 0x18)`, indexed triangle
/// list and `RwIm3DEnd` when it succeeded; then the render states are restored (raster 0, zwrite 1,
/// ztest 1, src 5, dst 6, vertex alpha 0, cull 2).
#[hook(addr = 0x6E0E20, conv = thiscall, name = "CVehicle::DoHeadLightBeam", this = "CVehicle", expect = "81 EC 94 00 00 00",
       unverified = "mvpd-veh-4", route = "L2", hazards = "x87-transcendental",
       recs = "driving",
       note = "night driving only; writes TempBufferVertices/Indices (0xC4D958/0xC4B958); NormaliseAndMag/Normalise fsqrt in callees")]
pub fn vehicle_do_head_light_beam(this: GPtr<u8>, light_id: u32, mat: GPtr<u8>, is_right: u32) {
    let dummies = rd::<GPtr<u8>>(model_info(i32::from(rd::<i16>(this, 0x22))), 0x5C);
    let dp = dummies.byte_add(24 * light_id as isize);
    let mut dummy = [rd::<u32>(dp, 0), rd::<u32>(dp, 4), rd::<u32>(dp, 8)];
    if light_id == 1
        && !x(f32::from_bits(dummy[0])).fcom(x(0.0)).test_parity(0x44)
        && !x(f32::from_bits(dummy[1])).fcom(x(0.0)).test_parity(0x44)
        && !x(f32::from_bits(dummy[2])).fcom(x(0.0)).test_parity(0x44)
    {
        return;
    }
    let mut p = [0f32; 3];
    matrix_mul_vec(GPtr::from_raw(p.as_mut_ptr()).cast(), mat, GPtr::from_raw(dummy.as_mut_ptr()).cast());
    if is_right as u8 == 0 {
        let k = x(f32::from_bits(dummy[0])).fdouble();
        let a = k.fmul_m(vf(mat, 0));
        let b = f32_of(k.fmul_m(vf(mat, 1)));
        let c = f32_of(k.fmul_m(vf(mat, 2)));
        p[0] = f32_of(x(p[0]).fsub(a));
        p[1] = f32_of(x(p[1]).fsub_m(b));
        p[2] = f32_of(x(p[2]).fsub_m(c));
    }
    let cm = g::<GPtr<u8>>(0xB6_F03C).read();
    let cam = if cm.is_null() { GPtr::<u8>::from_addr(0xB6_F02C) } else { cm.byte_add(0x30) };
    let mut d = CVector::new(
        f32_of(x(vf(cam, 0)).fsub_m(p[0])),
        f32_of(x(vf(cam, 1)).fsub_m(p[1])),
        f32_of(x(vf(cam, 2)).fsub_m(p[2])),
    );
    normalise_and_mag(GPtr::from_raw(&mut d).cast());
    let dot = f32_of(x(d.z).fmul_m(vf(mat, 6)).fadd(x(d.y).fmul_m(vf(mat, 5))).fadd(x(d.x).fmul_m(vf(mat, 4))));
    for (s, v) in [(8, 0), (6, 1), (0xC, 1), (0xA, 5), (0xB, 2), (7, 2), (1, 0), (0x14, 1), (0x1D, 5), (0x1E, 0)] {
        rw_render_state_set(s, v);
    }
    let alpha = ftol32(x(dot).fabs().fsubr_m(1.0).fmul_m(32.0)) as u8;
    let angle = if rd::<u16>(this, 0x22) == 0x212 { x(0.5) } else { x(f32::from_bits(0x3E19_999A)) };
    let ux = f32_of(angle.fmul_m(vf(mat, 8)));
    let uy = f32_of(angle.fmul_m(vf(mat, 9)));
    let uz = angle.fmul_m(vf(mat, 10));
    let mut n = CVector::new(f32_of(x(vf(mat, 4)).fsub_m(ux)), f32_of(x(vf(mat, 5)).fsub_m(uy)), f32_of(x(vf(mat, 6)).fsub(uz)));
    normalise(GPtr::from_raw(&mut n));
    let mut r = CVector::default();
    cross_product(GPtr::from_raw(&mut r), GPtr::from_raw(&mut n), GPtr::from_raw(&mut d));
    normalise(GPtr::from_raw(&mut r));
    let tenth = f32::from_bits(0x3DCC_CCCD);
    let fx = x(vf(mat, 4)).fmul_m(tenth);
    let fy = x(vf(mat, 5)).fmul_m(tenth);
    let fz = f32_of(x(vf(mat, 6)).fmul_m(tenth));
    let l = [f32_of(x(p[0]).fsub(fx)), f32_of(x(p[1]).fsub(fy)), f32_of(x(p[2]).fsub_m(fz))];
    let h = f32::from_bits(0x3D4C_CCCD);
    // R * 0.05
    let rx5 = x(r.x).fmul_m(h);
    let rx5m = f32_of(rx5);
    let ry5 = x(r.y).fmul_m(h);
    let rz5 = x(r.z).fmul_m(h);
    let v0 = [f32_of(x(l[0]).fsub_m(rx5m)), f32_of(x(l[1]).fsub(ry5)), f32_of(x(l[2]).fsub(rz5))];
    let (rx5m, ry5m, rz5m) = (rx5m, f32_of(ry5), f32_of(rz5));
    let v1x = x(rx5m).fadd_m(l[0]);
    let v1y = x(ry5m).fadd_m(l[1]);
    let v1z = f32_of(x(rz5m).fadd_m(l[2]));
    // R * 0.5 and N * 3
    let rx50 = f32_of(x(r.x).fmul_m(0.5));
    let ry50 = x(r.y).fmul_m(0.5);
    let rz50 = f32_of(x(r.z).fmul_m(0.5));
    let nx3 = x(n.x).fmul_m(3.0);
    let nx3m = f32_of(nx3);
    let ny3 = f32_of(x(n.y).fmul_m(3.0));
    let nz3 = f32_of(x(n.z).fmul_m(3.0));
    let ax = f32_of(nx3.fadd_m(l[0]));
    let ay = x(ny3).fadd_m(l[1]);
    let az = f32_of(x(nz3).fadd_m(l[2]));
    let v2 = [f32_of(x(ax).fsub_m(rx50)), f32_of(ay.fsub(ry50)), f32_of(x(az).fsub(x(rz50)))];
    let bx = f32_of(x(nx3m).fadd_m(l[0]));
    let by = x(ny3).fadd_m(l[1]);
    let bz = f32_of(x(nz3).fadd_m(l[2]));
    let v3 = [f32_of(x(bx).fadd_m(rx50)), f32_of(by.fadd(ry50)), f32_of(x(bz).fadd(x(rz50)))];
    let fifth = f32::from_bits(0x3E4C_CCCD);
    let nx2 = x(n.x).fmul_m(fifth);
    let ny2 = x(n.y).fmul_m(fifth);
    let nz2 = f32_of(x(n.z).fmul_m(fifth));
    let v4 = [f32_of(nx2.fadd_m(l[0])), f32_of(ny2.fadd_m(l[1])), f32_of(x(nz2).fadd_m(l[2]))];
    let colour = (u32::from(alpha) << 24) | 0xFF_FFFF;
    let vb = TEMP_VERTS;
    put(vb + 4, v0[1].to_bits());
    put(vb + 0x24 + 8, v1z.to_bits());
    put(vb + 0x48, v2[0].to_bits());
    put(vb + 0x48 + 4, v2[1].to_bits());
    put(vb + 0x48 + 8, v2[2].to_bits());
    put(vb + 8, v0[2].to_bits());
    put(vb, v0[0].to_bits());
    put(vb + 0x6C, v3[0].to_bits());
    put(vb + 0x48 + 0x18, 0xFF_FFFF);
    put(vb + 0x6C + 4, v3[1].to_bits());
    put(vb + 0x6C + 0x18, 0xFF_FFFF);
    put(vb + 0x6C + 8, v3[2].to_bits());
    putf(vb + 0x24, v1x);
    put(vb + 0x90, v4[0].to_bits());
    putf(vb + 0x24 + 4, v1y);
    for (k, idx) in [0u16, 1, 4, 1, 3, 4, 2, 3, 4, 0, 2, 4].into_iter().enumerate() {
        g::<u16>(TEMP_INDICES + 2 * k).write(idx);
    }
    put(TEMP_NUM_INDICES, 12);
    put(vb + 0x18, colour);
    put(vb + 0x24 + 0x18, colour);
    put(vb + 0x90 + 4, v4[1].to_bits());
    put(vb + 0x90 + 8, v4[2].to_bits());
    put(vb + 0x90 + 0x18, colour);
    put(TEMP_NUM_VERTS, 5);
    if rw_im3d_transform(GPtr::from_addr(vb), 5, 0, 0x18) != 0 {
        rw_im3d_render_indexed_primitive(3, GPtr::from_addr(TEMP_INDICES), g::<i32>(TEMP_NUM_INDICES).read());
        rw_im3d_end();
    }
    for (s, v) in [(1, 0), (8, 1), (6, 1), (0xA, 5), (0xB, 6), (0xC, 0), (0x14, 2)] {
        rw_render_state_set(s, v);
    }
}
orig! {
    /// `CMatrix::Attach(RwMatrix*, bool)` (0x59BD10)
    fn matrix_attach(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) = thiscall 0x59BD10 => unported;
    /// `CMatrix::SetScale(float, float, float)` (0x59AF00)
    fn matrix_set_scale(m: GPtr<u8>, sx: f32, sy: f32, sz: f32) = thiscall 0x59AF00 => unported;
    /// `CMatrix::Rotate(float, float, float)` (0x59B460)
    fn matrix_rotate(m: GPtr<u8>, rx: f32, ry: f32, rz: f32) = thiscall 0x59B460 => unported;
    /// `CMatrix::RotateY(float)` (0x59B2C0)
    fn matrix_rotate_y(m: GPtr<u8>, a: f32) = thiscall 0x59B2C0 => unported;
    /// `CMatrix::UpdateRW()` (0x59BBB0)
    fn matrix_update_rw(m: GPtr<u8>) = thiscall 0x59BBB0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CDamageManager::GetWheelStatus(int32)` (0x6C21B0)
    fn dm_get_wheel_status(dm: GPtr<u8>, wheel: i32) -> u32 = thiscall 0x6C21B0 => crate::damage::get_wheel_status;
    /// `CVehicleModelInfo::GetWheelSize(bool front)` (0x6A06F0, ST0)
    fn get_wheel_size(mi: GPtr<u8>, front: u32) -> f64 = thiscall 0x6A06F0 => unported;
    /// `SurfaceInfos_c::IsWater(uint32)` (0x55E710)
    fn surface_is_water(s: GPtr<u8>, surf: u32) -> u8 = thiscall 0x55E710 => unported;
    /// `SurfaceInfos_c::IsSand(uint32)` (0x55E6F0)
    fn surface_is_sand(s: GPtr<u8>, surf: u32) -> u8 = thiscall 0x55E6F0 => unported;
    /// `operator*(float, const CVector&)` (0x40FE90, hidden result pointer)
    fn vector_scale(out: GPtr<u8>, k: f32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => unported;
    /// `operator+(const CVector&, const CVector&)` (0x40FE30, hidden result pointer)
    fn vector_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => unported;
    /// `FxPrtMult_c::FxPrtMult_c(float x7)` (0x4AB290), the floats as bits
    fn fx_prt_mult_ctor(p: GPtr<u8>, r: u32, g: u32, b: u32, a: u32, size: u32, unk: u32, life: u32) -> GPtr<u8> = thiscall 0x4AB290 => unported;
    /// `FxSystem_c::AddParticle(CVector*, CVector*, float, FxPrtMult_c*, float, float, float, int32)` (0x4AA440)
    fn fx_add_particle(fx: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, time: u32, mult: GPtr<u8>, rot: u32, bright: u32, cam: u32, local: u32) = thiscall 0x4AA440 => unported;
}

/// CRT `_CIasin` (0x821E70) of ST0.
fn ci_asin(v: X87) -> X87 {
    X87::from_f64_exact(sa_core::orig::x87_unary(0x82_1E70, v.to_f64()))
}

fn lt0(v: X87) -> bool {
    !v.fcom(x(0.0)).test_parity(0x05)
}

/// `CAutomobile::UpdateWheelMatrix(int32 nodeIndex, int32 flags)` (0x6AA290, `ret 8`; entry `nop;
/// jmp` into the protection fragment 0x401A84, SEH frame).
///
/// Nothing without the node frame (+0x648 + 4*node). Nodes 2..7 select the wheel (rf 2, rear right 3,
/// lf 0, rear left 1), the side (-1 left, +1 right) and the steer angle: front wheels the steer
/// angle +0x494 (none with handling flag 0x20, steer-rear), rear wheels `-steer` with 0x20, +0x498
/// with 0x40, else none; a non-zero angle (flags bit 1 clear) is scaled by 0.6 on the outer side
/// (sign rules per front/rear and side as compiled) and gets +pi on the left; a zero angle is pi on
/// the left, 0 on the right. Other nodes only destroy the (unattached) local matrix. The local
/// `CMatrix` is attached to the frame's modelling matrix; the wheel position = (matrix x, y,
/// wheelPosition +0x838[wheel]); the scale from the model's wheel scale (+0x40 front / +0x44 rear;
/// without a wheel model (+0x48 == -1) relative to +0x458 and an x scale `scale / (+0x458 * 0.7)`),
/// the combine (0x214, nodes 3/6) and the plane (0x201, node 5) specials, the wheel width upgrade
/// factors (handling flags 0x100..0x8000: 0.65 / 0.8 / 1.1 / 1.25; subtypes 1/2 none) and x3 for the
/// kart (0x23B); `SetScale`, then `Rotate(0, 0, angle)` for subtype 3 or `Rotate(side * rotation
/// +0x828, 0, angle (+ 0.3 * sin(rotation) for a burst tyre))`. Unless monster truck / hydraulics /
/// the per-axle handling flags 0x20000 front / 0x200000 rear (those take the tilt below): on water (cheat 0x969152 or the vortex 0x21B) with the previous
/// compression < 1 and not drowning, `RotateY(side * -pi/2)` (flags bit 2 clear) and (flags bit 3
/// clear) a water splash (odd frames, vel.z + 2) / huge smoke or a sand particle at the col point
/// with `0.5 * moveSpeed + side * right`; else (flags bit 0 clear, axle flags clear) the camber
/// `RotateY(asin(clamp(-(pos.z + +0x898 - wheelSize/2) * side, -1, 1)))` (negated for the camber
/// flags 0x80000 front / 0x800000 rear). Monster truck / hydraulics: `RotateY(atan2(-(wheelPos[i] -
/// wheelPos[opposite]) * side, 2|pos.y|))`. The position is added back, `UpdateRW`, `~CMatrix`.
#[hook(addr = 0x6AA290, conv = thiscall, name = "CAutomobile::UpdateWheelMatrix", this = "CAutomobile", expect = "90 E9 EE 77 D5 FF",
       unverified = "mvpd-veh-4", route = "L2", hazards = "x87-transcendental,entry-protection-read",
       recs = "driving",
       note = "fsin of the wheel rotation (burst tyre), fpatan (monster truck/hydraulics), _CIasin via orig::x87_unary(0x821E70); the stack CMatrix lives on the Rust stack")]
pub fn automobile_update_wheel_matrix(this: GPtr<u8>, node: i32, flags: u32) {
    let frame = rd::<GPtr<u8>>(this, 0x648 + 4 * node as isize);
    if frame.is_null() {
        return;
    }
    let mut matb = [0u32; 0x12];
    let mat = GPtr::from_raw(matb.as_mut_ptr()).cast::<u8>();
    let mi = model_info(i32::from(rd::<i16>(this, 0x22)));
    let hf = rd::<u32>(this, 0x38C);
    let steer = rd::<f32>(this, 0x494);
    let (wheel, side, front, rear, mut angle): (isize, f32, bool, bool, Option<f32>);
    match node - 2 {
        0 => {
            (wheel, side, front, rear) = (2, 1.0, true, false);
            angle = if hf & 0x20 != 0 { None } else { Some(steer) };
        }
        1 | 2 => {
            (wheel, side, front, rear) = (3, 1.0, false, true);
            angle = if hf & 0x20 != 0 {
                Some(f32_of(x(steer).fchs()))
            } else if hf & 0x40 != 0 {
                Some(rd::<f32>(this, 0x498))
            } else {
                None
            };
        }
        3 => {
            (wheel, side, front, rear) = (0, -1.0, true, false);
            angle = if hf & 0x20 != 0 { None } else { Some(steer) };
        }
        4 | 5 => {
            (wheel, side, front, rear) = (1, -1.0, false, true);
            angle = if hf & 0x20 != 0 {
                Some(f32_of(x(steer).fchs()))
            } else if hf & 0x40 != 0 {
                Some(rd::<f32>(this, 0x498))
            } else {
                None
            };
        }
        _ => {
            matrix_dtor(mat);
            return;
        }
    }
    // a missing steer (flag paths jumping to 0x6AA474 / 0x6AA47E) is pi on the left, 0 on the right
    let pi = f32::from_bits(0x4049_0FDB);
    let mut a = match angle.take() {
        Some(v) if x(v).fcom(x(0.0)).test_parity(0x44) && flags & 2 == 0 => {
            let left = lt0(x(side));
            let neg = lt0(x(v));
            let pos = !x(v).fcom(x(0.0)).test_any(0x41);
            let k = f32::from_bits(0x3F19_999A);
            let scaled = if left { if rear { pos } else { neg } } else if rear { neg } else { pos };
            let v = if scaled { f32_of(x(k).fmul_m(v)) } else { v };
            if left { f32_of(x(v).fadd_m(pi)) } else { v }
        }
        Some(_) => {
            if lt0(x(side)) {
                pi
            } else {
                0.0
            }
        }
        None => {
            if side < 0.0 {
                pi
            } else {
                0.0
            }
        }
    };
    matrix_attach(mat, frame.byte_add(0x10), 0);
    let mut pos = [rd::<f32>(mat, 0x30), rd::<f32>(mat, 0x34), f32_of(x(rd::<f32>(this, 0x838 + 4 * wheel)))];
    let mut s = rd::<f32>(this, 0x458);
    let mut xs = x(1.0);
    if rd::<i16>(mi, 0x48) == -1 {
        if rear {
            s = f32_of(x(rd::<f32>(mi, 0x44)).fdiv_m(rd::<f32>(mi, 0x40)).fmul_m(s));
        }
        xs = x(rd::<f32>(this, 0x458)).fmul_m(0.7).fdivr_m(rd::<f32>(mi, 0x40));
    } else {
        s = rd::<f32>(mi, if rear { 0x44 } else { 0x40 });
    }
    let model = rd::<u16>(this, 0x22);
    if model == 0x214 && (node == 6 || node == 3) {
        let k = g::<f32>(0x8D_31D4).read();
        s = f32_of(x(k).fdiv_m(rd::<f32>(mi, 0x40)));
        pos[2] = f32_of(x(k).fsub_m(rd::<f32>(mi, 0x44)).fmul_m(g::<f32>(0x8D_31D0).read()).fadd_m(pos[2]));
        let v = if lt0(x(side)) { x(a).fsub_m(pi) } else { x(a) };
        a = f32_of(v.fmul_m(0.5));
        if lt0(x(side)) {
            a = f32_of(x(a).fadd_m(pi));
        }
    } else if model == 0x201 && node == 5 {
        a = 0.0;
    }
    let sub = rd::<u32>(this, 0x594);
    if sub == 1 || sub == 2 {
        xs = x(1.0);
    } else {
        let hf = rd::<u32>(this, 0x38C);
        let ah = (hf >> 8) as u8;
        let k = if rear && ah & 0xF0 != 0 {
            if ah & 0x10 != 0 {
                Some(0x3F26_6666)
            } else if ah & 0x20 != 0 {
                Some(0x3F4C_CCCD)
            } else if ah & 0x40 != 0 {
                Some(0x3F8C_CCCD)
            } else if ah & 0x80 != 0 {
                Some(0x3FA0_0000)
            } else {
                None
            }
        } else if front && ah & 0x0F != 0 {
            if ah & 1 != 0 {
                Some(0x3F26_6666)
            } else if ah & 2 != 0 {
                Some(0x3F4C_CCCD)
            } else if ah & 4 != 0 {
                Some(0x3F8C_CCCD)
            } else if ah & 8 != 0 {
                Some(0x3FA0_0000)
            } else {
                None
            }
        } else {
            None
        };
        if let Some(k) = k {
            xs = xs.fmul_m(f32::from_bits(k));
        }
    }
    if model == 0x23B {
        xs = xs.fmul_m(3.0);
    }
    matrix_set_scale(mat, f32_of(xs.fmul_m(s)), s, s);
    if rd::<u32>(this, 0x594) == 3 {
        matrix_rotate(mat, 0.0, 0.0, a);
    } else {
        let st = dm_get_wheel_status(this.byte_add(0x5A0), wheel as i32);
        let r = x(side).fmul_m(rd::<f32>(this, 0x828 + 4 * wheel));
        if st == 1 {
            let rm = f32_of(r);
            let z = f32_of(r.fsin().fmul_m(f32::from_bits(0x3E99_999A)).fadd_m(a));
            matrix_rotate(mat, rm, 0.0, z);
        } else {
            matrix_rotate(mat, f32_of(r), 0.0, a);
        }
    }
    let mflags = rd::<u32>(rd::<GPtr<u8>>(this, 0x384), 0xCC);
    let tilt_only = rd::<i8>(this, 0x868) < 0
        || rd::<u32>(this, 0x38C) & 0x2_0000 != 0
        || (mflags & 0x2_0000 != 0 && front)
        || (mflags & 0x20_0000 != 0 && rear);
    if tilt_only {
        let opp = if wheel > 1 { wheel - 2 } else { wheel + 2 };
        let yy = x(pos[1]).fabs().fdouble();
        let d = x(rd::<f32>(this, 0x838 + 4 * wheel))
            .fsub_m(rd::<f32>(this, 0x838 + 4 * opp))
            .fmul_m(side)
            .fmul_m(-1.0);
        matrix_rotate_y(mat, f32_of(d.fpatan(yy)));
    } else if (g::<u8>(0x96_9152).read() != 0 || model == 0x21B)
        && lt_one(rd::<f32>(this, 0x7E4 + 4 * wheel))
        && rd::<u8>(this, 0x42B) & 0x40 == 0
    {
        let fl = flags as u8;
        if fl & 4 == 0 {
            matrix_rotate_y(mat, f32_of(x(side).fmul_m(f32::from_bits(0xBFC9_0FDB))));
        }
        if fl & 8 == 0 {
            water_particles(this, wheel, side);
        }
    } else if flags & 1 == 0 && !(front && mflags & 0x1_0000 != 0) && !(rear && mflags & 0x10_0000 != 0) {
        let base = f32_of(x(pos[2]).fadd_m(rd::<f32>(this, 0x898)));
        let h = X87::from_f64_exact(get_wheel_size(mi, u32::from(front))).fmul_m(0.5).fsubr_m(base);
        let flip = (front && mflags & 0x8_0000 != 0) || (rear && mflags & 0x80_0000 != 0);
        let v = h.fmul_m(-1.0).fmul_m(side);
        let c = if !x(-1.0).fcom(v).test_any(0x41) {
            -1.0
        } else if !x(1.0).fcom(v).test_parity(0x05) {
            1.0
        } else if !x(-1.0).fcom(v).test_any(0x41) {
            -1.0
        } else {
            f32_of(v)
        };
        let r = ci_asin(x(c));
        matrix_rotate_y(mat, f32_of(if flip { r.fchs() } else { r }));
    }
    wr::<f32>(mat, 0x30, f32_of(x(rd::<f32>(mat, 0x30)).fadd_m(pos[0])));
    wr::<f32>(mat, 0x34, f32_of(x(rd::<f32>(mat, 0x34)).fadd_m(pos[1])));
    wr::<f32>(mat, 0x38, f32_of(x(rd::<f32>(mat, 0x38)).fadd_m(pos[2])));
    matrix_update_rw(mat);
    matrix_dtor(mat);
}

fn lt_one(v: f32) -> bool {
    !x(v).fcom(x(1.0)).test_parity(0x05)
}

/// The on-water wheel particles of `UpdateWheelMatrix`: at the wheel col point with velocity
/// `0.5 * moveSpeed + side * right`; a water surface gives a water splash (0xA9AE38, odd frames,
/// vel.z + 2) or huge smoke (0xA9AE20), sand a sand particle (0xA9AE18).
fn water_particles(this: GPtr<u8>, wheel: isize, side: f32) {
    let mut mult = [0u32; 8];
    let mp = GPtr::from_raw(mult.as_mut_ptr()).cast::<u8>();
    fx_prt_mult_ctor(mp, 0x3F80_0000, 0x3F80_0000, 0x3F80_0000, 0x3E4C_CCCD, 0x3F00_0000, 0, 0x3D4C_CCCD);
    let cp = this.byte_add(0x724 + 0x2C * wheel);
    let mut pos = [rd::<u32>(cp, 0), rd::<u32>(cp, 4), rd::<u32>(cp, 8)];
    let mut t1 = [0u32; 3];
    let mut t2 = [0u32; 3];
    let mut vel = [0f32; 3];
    let r1 = vector_scale(GPtr::from_raw(t1.as_mut_ptr()).cast(), side, rd::<GPtr<u8>>(this, 0x14));
    let r2 = vector_scale(GPtr::from_raw(t2.as_mut_ptr()).cast(), 0.5, this.byte_add(0x44));
    vector_add(GPtr::from_raw(vel.as_mut_ptr()).cast(), r2, r1);
    let surf = u32::from(rd::<u8>(cp, 0x23));
    let infos = GPtr::<u8>::from_addr(0xB7_9538);
    let pp = GPtr::from_raw(pos.as_mut_ptr()).cast::<u8>();
    let vp = GPtr::from_raw(vel.as_mut_ptr()).cast::<u8>();
    if surface_is_water(infos, surf) != 0 {
        let fc = g::<u32>(0xB7_CB4C).read().wrapping_sub(1);
        let fx = if fc & 1 != 0 {
            wr::<f32>(vp, 8, f32_of(x(rd::<f32>(vp, 8)).fadd_m(2.0)));
            g::<GPtr<u8>>(0xA9_AE38).read()
        } else {
            g::<GPtr<u8>>(0xA9_AE20).read()
        };
        fx_add_particle(fx, pp, vp, 0, mp, 0xBF80_0000, 0x3F99_999A, 0x3F19_999A, 0);
    } else if surface_is_sand(infos, surf) != 0 {
        let mut m2 = [0u32; 8];
        let m2p = GPtr::from_raw(m2.as_mut_ptr()).cast::<u8>();
        fx_prt_mult_ctor(m2p, 0x3F4F_5C29, 0x3F2B_851F, 0x3F11_EB85, 0x3EB3_3333, 0x3F00_0000, 0, 0x3D4C_CCCD);
        fx_add_particle(g::<GPtr<u8>>(0xA9_AE18).read(), pp, vp, 0, m2p, 0xBF80_0000, 0x3F99_999A, 0x3F19_999A, 0);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_VEHICLE_DO_HEAD_LIGHT_BEAM,
    HOOK_AUTOMOBILE_UPDATE_WHEEL_MATRIX,
];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
