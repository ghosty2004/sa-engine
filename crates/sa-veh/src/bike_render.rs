//! WRITER-VEH-BIG (batch `vehbig-1`): `CBike::PreRender`, the per-frame visual update of a bike
//! (suspension display, wheel particles, exhaust, the police bike's flashing lights, lights,
//! shadow, wheel rotation and the fork / handlebar / swing arm / wheel / body frames). Ported from
//! the 1.0 US bytes (gta-reversed has no body for it).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::common::{ftol32, g, rd, wr, x, xi};

use crate::bindings::add_damaged_vehicle_particles;
use crate::bindings::add_exhaust_particles;
use crate::bindings::add_single_wheel_particles;
use crate::bindings::do_vehicle_lights;
use crate::bindings::process_wheel_rotation;
use sa_world::bindings::register_corona;
use crate::bindings::store_shadow_for_vehicle;
use sa_core::bindings::matrix_assign;
use sa_core::bindings::matrix_attach;
use sa_world::bindings::matrix_dtor;
use sa_core::bindings::matrix_mul;
use sa_core::bindings::matrix_rotate_y;
use sa_core::bindings::matrix_set_rotate;
use sa_core::bindings::matrix_set_rotate_x;
use sa_core::bindings::matrix_set_unity;
use sa_core::bindings::matrix_update;
use sa_core::bindings::matrix_update_rw;
use sa_core::bindings::quat_get as quaternion_get;
use sa_world::bindings::add_light;
use crate::bindings::vehicle_pre_render;
use sa_core::bindings::matrix_mul_assign;
use sa_core::bindings::matrix_set_translate;
use crate::bindings::bike_calculate_lean_matrix as calculate_lean_matrix;
use sa_world::bindings::entity_get_col_model as get_col_model;
use sa_world::bindings::multiply3x3;
use sa_core::bindings::vector_normalise;
use sa_world::bindings::physical_get_speed as get_speed;
orig! {
    /// `CQuaternion::Set(RwV3d*, float)` (0x59C600)
    pub(crate) fn quaternion_set(q: GPtr<u8>, axis: GPtr<u8>, angle: f32) = thiscall 0x59C600 => unported;
    /// `CBike::DoBurstAndSoftGroundRatios()` (vtable +0xD0)
    fn v_do_burst_and_soft_ground_ratios(v: GPtr<u8>) = thiscall vslot 0xD0 => unported;
}

/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7_CB5C;
const F1: u32 = 0x3F80_0000;

fn rv(p: GPtr<u8>, off: isize) -> CVector {
    CVector::new(rd::<f32>(p, off), rd::<f32>(p, off + 4), rd::<f32>(p, off + 8))
}

fn wv(p: GPtr<u8>, off: isize, v: CVector) {
    wr::<f32>(p, off, v.x);
    wr::<f32>(p, off + 4, v.y);
    wr::<f32>(p, off + 8, v.z);
}

use sa_core::field::ldx as ld;

use sa_math::fpu::xc as c;

fn ts() -> f32 {
    g::<f32>(TIME_STEP).read()
}

use sa_math::fpu::le_or_nan as le;

use sa_math::fpu::ge_or_nan as ge;

fn gp<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v).cast()
}

fn mat(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0x14)
}

use sa_core::field::rdp as frame_at;

/// A stack `CMatrix` (0x48 bytes) with the inline constructor (attach / owns zero).
struct StackMatrix([u32; 0x12]);

impl StackMatrix {
    fn new() -> StackMatrix {
        StackMatrix([0; 0x12])
    }
    fn ptr(&mut self) -> GPtr<u8> {
        GPtr::from_raw(self.0.as_mut_ptr()).cast()
    }
}

/// The displayed suspension height of a pair (line `line` start z minus the rescaled compression
/// of the more compressed wheel of the pair), eased by 0.75 into `out`.
#[allow(clippy::too_many_arguments)]
fn suspension_display(
    this: GPtr<u8>,
    col_data: GPtr<u8>,
    spring: isize,
    line_len: isize,
    ca: isize,
    cb: isize,
    line_z: isize,
    out: isize,
) {
    let r = ld(this, spring).fdiv_m(rd::<f32>(this, line_len)).fsubr_m(1.0);
    let cmin = if ge(ld(this, cb), ld(this, ca)) {
        ld(this, ca)
    } else {
        ld(this, cb)
    };
    let q = cmin.fsub(r).fdiv(c(F1).fsub(r)).to_f32();
    let lines = rd::<GPtr<u8>>(col_data, 0x10);
    let mut v = ld(lines, line_z);
    if !le(x(q), X87::zero()) {
        v = v.fsub(x(q).fmul_m(rd::<f32>(this, spring)));
    }
    let v = v
        .fsub_m(rd::<f32>(this, out))
        .fmul_m(0.75)
        .fadd_m(rd::<f32>(this, out));
    wr::<f32>(this, out, v.to_f32());
}

/// `CBike::PreRender()` (0x6BD090).
///
/// `CVehicle::PreRender`; with +0x42B bit 0 the burst/soft-ground ratios and the eased front /
/// rear suspension heights (+0x760 / +0x764). Driven bikes emit the particles of the more
/// compressed wheel of each pair (from the contact point shifted by `sin(lean) * bboxMin.z * 0.8`
/// along right); the lean matrix; exhaust (engine on, slower than 130); damage particles. The
/// police bike (0x20B) with its siren alternates a red and a blue corona every 256 ms and adds a
/// coloured point light above the front. Lights (with the lean matrix, flags 4), shadow (type 1),
/// the front / rear wheel rotation from `ProcessWheelRotation`, then the frames: the fork
/// (+0x5A8) rotated by the steer lean around the model's fork axis, the handlebars (+0x5BC),
/// the swing arm (+0x5AC, `-asin((front - rear) / armLength)`), the front wheel (+0x5B0; burst:
/// wobble `sin(rot) * 0.05`), the mudguard (+0x5B8), the rear wheel (+0x5B4; burst wobble 0.07)
/// and the body (+0x5A4, rolled by the lean and lowered by `(1 - cos(lean)) * bboxMin.z * 0.9`).
///
/// Kept from the bytes: the rear wheel's contact point is lowered by the front wheel's radius
/// (its rotation uses the rear radius); the light colour bytes come from a timer phase.
#[hook(addr = 0x6bd090, conv = thiscall, name = "CBike::PreRender", this = "CBike",
       expect = "6A FF 68 21 83 84 00",
       route = "L2", hazards = "x87-transcendental,uninit-locals",
       recs = "driving",
       note = "fsin/fcos via X87, _CIasin via orig::x87_unary; stack CMatrix / RwMatrix locals zeroed")]
pub fn bike_pre_render(this: GPtr<u8>) {
    vehicle_pre_render(this);
    // L114: the collision model, L13c: its data
    let col = get_col_model(this);
    let col_data = rd::<GPtr<u8>>(col, 0x2C);
    if rd::<u8>(this, 0x42B) & 1 != 0 {
        v_do_burst_and_soft_ground_ratios(this);
        suspension_display(this, col_data, 0x770, 0x780, 0x710, 0x714, 8, 0x760);
        suspension_display(this, col_data, 0x778, 0x788, 0x718, 0x71C, 0x48, 0x764);
    }
    let st = rd::<u8>(this, 0x36) >> 3;
    if st <= 3 {
        wheel_particles(this, false);
    }
    wr::<u8>(this, 0x5C8, 0);
    calculate_lean_matrix(this);
    let m = mat(this);
    let fwd_speed = ld(m, 0x18)
        .fmul_m(rd::<f32>(this, 0x4C))
        .fadd(ld(m, 0x14).fmul_m(rd::<f32>(this, 0x48)))
        .fadd(ld(m, 0x10).fmul_m(rd::<f32>(this, 0x44)))
        .fdiv_m(g::<f32>(0xC1_C81C).read());
    if rd::<u8>(this, 0x428) & 0x10 != 0
        && (rd::<u32>(rd::<GPtr<u8>>(this, 0x384), 0xCC) >> 8) & 0x10 == 0
        && !ge(fwd_speed, c(0x4302_0000))
        && rd::<u8>(this, 0x42B) & 0x40 == 0
    {
        add_exhaust_particles(this);
    }
    add_damaged_vehicle_particles(this);
    if rd::<u16>(this, 0x22) == 0x20B
        && rd::<u8>(this, 0x42D) & 0x80 != 0
        && rd::<u8>(this, 0x428) & 0x10 != 0
        && rd::<u32>(this, 0x40) & 0x2000_0000 == 0
    {
        police_lights(this);
    }
    do_vehicle_lights(this, this.byte_add(0x5CC), 4);
    store_shadow_for_vehicle(this, 1);

    // L190: the frame matrix
    let mut l190 = StackMatrix::new();
    let mi = crate::common::model_info(i32::from(rd::<i16>(this, 0x22)));
    wheel_rotations(this, col_data, mi);
    frames(this, mi, col, &mut l190);
    matrix_dtor(l190.ptr());
}

/// The front / rear wheel rotation speeds (+0x758 / +0x75C) and angles (+0x750 / +0x754) of the
/// wheels with ground contact (`ProcessWheelRotation` at the contact point's speed).
fn wheel_rotations(this: GPtr<u8>, col_data: GPtr<u8>, mi: GPtr<u8>) {
    let steer = ld(this, 0x494);
    let mut d = CVector::new(steer.fsin().fchs().to_f32(), steer.fcos().to_f32(), 0.0);
    // Lf8: the front wheel direction, L110: the rear (forward)
    let mut front_dir = CVector::default();
    multiply3x3(gp(&mut front_dir), mat(this), gp(&mut d));
    let mut rear_dir = rv(mat(this), 0x10);
    let lines = rd::<GPtr<u8>>(col_data, 0x10);
    if !le(ld(this, 0x730), X87::zero()) || !le(ld(this, 0x734), X87::zero()) {
        let p = wheel_point(this, lines, 0x24, 4, 8, 0x720, 0x724, 0x770, mi);
        let mut tmp = CVector::default();
        let mut sp = rv(get_speed(this, GPtr::from_raw(&mut tmp), p).cast(), 0);
        let r = ld(mi, 0x40).fmul_m(0.5).to_f32();
        // `fst` keeps the unrounded ST0 for the rotation step
        let w = X87::from_f64_exact(process_wheel_rotation(
            this,
            0,
            gp(&mut front_dir),
            gp(&mut sp),
            r,
        ));
        wr::<f32>(this, 0x758, w.to_f32());
        wr::<f32>(
            this,
            0x750,
            w.fmul_m(ts()).fadd_m(rd::<f32>(this, 0x750)).to_f32(),
        );
    }
    if !le(ld(this, 0x738), X87::zero()) || !le(ld(this, 0x73C), X87::zero()) {
        let p = wheel_point(this, lines, 0x64, 0x44, 0x48, 0x728, 0x72C, 0x778, mi);
        let mut tmp = CVector::default();
        let mut sp = rv(get_speed(this, GPtr::from_raw(&mut tmp), p).cast(), 0);
        let r = ld(mi, 0x44).fmul_m(0.5).to_f32();
        let w = X87::from_f64_exact(process_wheel_rotation(
            this,
            rd::<u32>(this, 0x810),
            gp(&mut rear_dir),
            gp(&mut sp),
            r,
        ));
        wr::<f32>(this, 0x75C, w.to_f32());
        wr::<f32>(
            this,
            0x754,
            w.fmul_m(ts()).fadd_m(rd::<f32>(this, 0x754)).to_f32(),
        );
    }
}

/// A wheel's contact point in body axes: y midway between two lines' start y, z the line start z
/// minus the smaller of two previous compressions times the spring length and the front radius.
#[allow(clippy::too_many_arguments)]
fn wheel_point(
    this: GPtr<u8>,
    lines: GPtr<u8>,
    ya: isize,
    yb: isize,
    z: isize,
    pa: isize,
    pb: isize,
    spring: isize,
    mi: GPtr<u8>,
) -> CVector {
    let y = ld(lines, ya).fadd_m(rd::<f32>(lines, yb)).fmul_m(0.5).to_f32();
    let cmin = if ge(ld(this, pa), ld(this, pb)) {
        ld(this, pb)
    } else {
        ld(this, pa)
    };
    let zz = cmin
        .fmul_m(rd::<f32>(this, spring))
        .fsubr_m(rd::<f32>(lines, z))
        .fsub(ld(mi, 0x40).fmul_m(0.5))
        .to_f32();
    CVector::new(0.0, y, zz)
}

/// 0x6BD1FC..0x6BD395: the particles of the leading wheel of each pair.
fn wheel_particles(this: GPtr<u8>, bmx: bool) {
    let spinning = rd::<u32>(this, 0x810) == 2;
    let (vx, vy, vz) = (ld(this, 0x44), ld(this, 0x48), ld(this, 0x4C));
    let speed = vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fsqrt().to_f32();
    for pair in 0..2isize {
        let wheel: isize = if pair == 0 {
            if !x(rd::<f32>(this, 0x720)).fcom(c(F1)).test_any(1) && !ge(ld(this, 0x724), c(F1)) {
                1
            } else {
                0
            }
        } else if !x(rd::<f32>(this, 0x72C)).fcom(c(F1)).test_any(1) && !ge(ld(this, 0x728), c(F1)) {
            2
        } else {
            3
        };
        // the BMX flags the rear pair while +0x810 is 3, the bike the front pair unless spinning
        let mut flags = if bmx {
            if pair == 0 || rd::<u32>(this, 0x810) == 3 {
                4u32
            } else {
                0
            }
        } else if pair == 0 && !spinning {
            4
        } else {
            0
        };
        let col = get_col_model(this);
        let s = ld(this, 0x648)
            .fsin()
            .fmul_m(rd::<f32>(col, 8))
            .fmul_m(f32::from_bits(0x3F4C_CCCD));
        let cp = this.byte_add(0x660 + 0x2C * wheel);
        let m = mat(this);
        let mut from = CVector::new(
            s.fmul_m(rd::<f32>(m, 0)).fadd_m(rd::<f32>(cp, 0)).to_f32(),
            ld(cp, 4).fadd(s.fmul_m(rd::<f32>(m, 4))).to_f32(),
            ld(cp, 8).fadd(s.fmul_m(rd::<f32>(m, 8))).to_f32(),
        );
        if rd::<u8>(this, 0x74C + pair) != 0 {
            flags += 1;
        }
        if rd::<u8>(this, 0x74E + pair) != 0 {
            flags += 2;
        }
        let id = if !le(ld(this, 0x648), X87::zero()) {
            0xBF80_0000u32
        } else {
            F1
        };
        add_single_wheel_particles(
            this,
            rd::<u32>(this, 0x80C + 4 * pair),
            u32::from(rd::<u8>(this, 0x65C + pair)),
            rd::<f32>(this, 0x720 + 4 * wheel),
            speed,
            cp,
            gp(&mut from),
            id as i32,
            pair as i32,
            rd::<u32>(this, 0x744 + 4 * pair),
            this.byte_add(0x74C + pair),
            flags,
        );
    }
}

/// 0x6BD443..0x6BD64D: the police bike's alternating siren coronas and light.
fn police_lights(this: GPtr<u8>) {
    let t = (g::<u32>(0xB7_CB84).read() & 0x1FF) as u8;
    let phase_low = g::<u32>(0xB7_CB84).read() & 0x1FF < 0x100;
    let b = ftol32(x(g::<f32>(0xB7_C4E4).read()).fmul_m(f32::from_bits(0x41CC_0000))) as u32;
    // L148: right lamp, L104: left lamp; L1a5 / bl: the two colour bytes
    let mut right_lamp = CVector::new(
        f32::from_bits(0x3E8F_5C29),
        f32::from_bits(0x3F19_999A),
        f32::from_bits(0x3E99_999A),
    );
    let mut left_lamp = CVector::new(
        f32::from_bits(0xBE8F_5C29),
        f32::from_bits(0x3F19_999A),
        f32::from_bits(0x3E99_999A),
    );
    let (l1a5, bl) = if phase_low {
        register_corona(
            (this.addr() as u32).wrapping_add(0x15),
            this,
            b,
            0,
            0,
            0xFF,
            gp(&mut left_lamp),
            f32::from_bits(0x3ECC_CCCD).to_bits(),
            f32::from_bits(0x4220_0000).to_bits(),
            0,
            0,
            0,
            0,
            0,
            f32::from_bits(0).to_bits(),
            0,
            f32::from_bits(0x3FC0_0000).to_bits(),
            0,
            f32::from_bits(0x41F0_0000).to_bits(),
            0,
            1,
        );
        (t, 0xFFu8.wrapping_sub(t))
    } else {
        let n = t.wrapping_neg();
        register_corona(
            (this.addr() as u32).wrapping_add(0x16),
            this,
            0,
            0,
            b,
            0xFF,
            gp(&mut right_lamp),
            f32::from_bits(0x3ECC_CCCD).to_bits(),
            f32::from_bits(0x4220_0000).to_bits(),
            0,
            0,
            0,
            0,
            0,
            f32::from_bits(0).to_bits(),
            0,
            f32::from_bits(0x3FC0_0000).to_bits(),
            0,
            f32::from_bits(0x41F0_0000).to_bits(),
            0,
            1,
        );
        (n, 0xFFu8.wrapping_sub(n))
    };
    let m = mat(this);
    let uz = ld(m, 0x28).fmul_m(0.5);
    let uy = ld(m, 0x24).fmul_m(0.5).to_f32();
    let ux = ld(m, 0x20).fmul_m(0.5).to_f32();
    let p = if m.is_null() {
        this.byte_add(4)
    } else {
        m.byte_add(0x30)
    };
    let z = ld(m, 0x18).fadd_m(rd::<f32>(p, 8)).fadd(uz).to_f32();
    let y = ld(m, 0x14).fadd_m(rd::<f32>(p, 4)).to_f32();
    let xx = ld(p, 0).fadd_m(rd::<f32>(m, 0x10)).to_f32();
    let k = f32::from_bits(0x3A80_0000);
    add_light(
        0,
        f32::from_bits(x(xx).fadd_m(ux).to_f32().to_bits()),
        f32::from_bits(x(y).fadd_m(uy).to_f32().to_bits()),
        f32::from_bits(z.to_bits()),
        f32::from_bits(0),
        f32::from_bits(0),
        f32::from_bits(0),
        f32::from_bits(0x4120_0000),
        f32::from_bits(xi(i32::from(l1a5)).fmul_m(k).to_f32().to_bits()),
        f32::from_bits(0),
        f32::from_bits(xi(i32::from(bl)).fmul_m(k).to_f32().to_bits()),
        0,
        1,
        GPtr::NULL,
    );
}

/// 0x6BD8EF..0x6BDDEC: the bike's moving frames.
fn frames(this: GPtr<u8>, mi: GPtr<u8>, col: GPtr<u8>, l190: &mut StackMatrix) {
    let mp = l190.ptr();
    let save = |mp: GPtr<u8>| rv(mp, 0x30);
    let restore_add = |mp: GPtr<u8>, s: CVector| {
        wr::<f32>(mp, 0x30, ld(mp, 0x30).fadd_m(s.x).to_f32());
        wr::<f32>(mp, 0x34, ld(mp, 0x34).fadd_m(s.y).to_f32());
        wr::<f32>(mp, 0x38, ld(mp, 0x38).fadd_m(s.z).to_f32());
    };
    let fork = frame_at(this, 0x5A8);
    if !fork.is_null() {
        matrix_attach(mp, fork.byte_add(0x10), 0);
        let s = save(mp);
        // Ldc: a matrix attached to the local RwMatrix L4c
        let mut rw = [0u32; 0x10];
        let mut ldc = StackMatrix::new();
        ldc.0[0x10] = GPtr::from_raw(rw.as_mut_ptr()).addr() as u32;
        let dp = ldc.ptr();
        matrix_set_unity(dp);
        matrix_update_rw(dp);
        let a = ld(mi, 0x58).fmul_m(f32::from_bits(0x3C8E_FA35));
        let mut axis = CVector::new(0.0, a.fsin().to_f32(), a.fcos().fchs().to_f32());
        vector_normalise(GPtr::<u8>::cast::<[f32;3]>(gp(&mut axis)));
        let mut q = [0f32; 4];
        quaternion_set(gp(&mut q), gp(&mut axis), ld(this, 0x644).fchs().to_f32());
        quaternion_get(gp(&mut q), GPtr::from_raw(rw.as_mut_ptr()).cast());
        matrix_update(dp);
        matrix_set_unity(mp);
        let mut l94 = StackMatrix::new();
        let r = matrix_mul(l94.ptr(), mp, dp);
        matrix_assign(mp, r);
        matrix_dtor(l94.ptr());
        restore_add(mp, s);
        matrix_update_rw(mp);
        let bars = frame_at(this, 0x5BC);
        if !bars.is_null() {
            matrix_attach(mp, bars.byte_add(0x10), 0);
            let s = save(mp);
            let st = rd::<u8>(this, 0x36) >> 3;
            if st != 4 && st != 5 {
                matrix_set_translate(mp, s.x, s.y, s.z);
            } else {
                matrix_set_unity(mp);
                matrix_mul_assign(mp, dp);
                restore_add(mp, s);
            }
            matrix_update_rw(mp);
        }
        matrix_dtor(dp);
    }
    let arm = frame_at(this, 0x5AC);
    if !arm.is_null() {
        let v = ld(this, 0x764)
            .fsub_m(rd::<f32>(this, 0x76C))
            .fdiv_m(rd::<f32>(this, 0x798));
        let a = X87::from_f64_exact(sa_core::orig::x87_unary(0x82_1E70, v.to_f64()))
            .fmul_m(-1.0)
            .to_f32();
        matrix_attach(mp, arm.byte_add(0x10), 0);
        let s = save(mp);
        matrix_set_rotate(mp, a, 0.0, 0.0);
        restore_add(mp, s);
        matrix_update_rw(mp);
    }
    matrix_attach(mp, frame_at(this, 0x5B0).byte_add(0x10), 0);
    let lines = rd::<GPtr<u8>>(rd::<GPtr<u8>>(col, 0x2C), 0x10);
    let z = ld(this, 0x760).fsub_m(rd::<f32>(this, 0x7A0)).to_f32();
    let y = ld(lines, 0x24)
        .fadd_m(rd::<f32>(lines, 4))
        .fmul_m(0.5)
        .fsub_m(rd::<f32>(this, 0x79C))
        .fsub(
            ld(this, 0x760)
                .fsub_m(rd::<f32>(this, 0x768))
                .fmul_m(rd::<f32>(this, 0x7A4)),
        )
        .to_f32();
    let s = CVector::new(rd::<f32>(mp, 0x30), y, z);
    if rd::<u8>(this, 0x65C) == 1 {
        let r = rd::<f32>(this, 0x750);
        matrix_set_rotate(
            mp,
            r,
            0.0,
            x(r).fsin().fmul_m(f32::from_bits(0x3D4C_CCCD)).to_f32(),
        );
    } else {
        matrix_set_rotate_x(mp, rd::<f32>(this, 0x750));
    }
    restore_add(mp, s);
    matrix_update_rw(mp);
    matrix_attach(mp, frame_at(this, 0x5B8).byte_add(0x10), 0);
    wv(mp, 0x30, s);
    matrix_update_rw(mp);
    matrix_attach(mp, frame_at(this, 0x5B4).byte_add(0x10), 0);
    let s = save(mp);
    if rd::<u8>(this, 0x65D) == 1 {
        let r = rd::<f32>(this, 0x754);
        matrix_set_rotate(
            mp,
            r,
            0.0,
            x(r).fsin().fmul_m(f32::from_bits(0x3D8F_5C29)).to_f32(),
        );
    } else {
        matrix_set_rotate_x(mp, rd::<f32>(this, 0x754));
    }
    restore_add(mp, s);
    matrix_update_rw(mp);
    let body = frame_at(this, 0x5A4);
    if !body.is_null() {
        matrix_attach(mp, body.byte_add(0x10), 0);
        let mut s = save(mp);
        let cl = ld(this, 0x648).fcos();
        s.z = c(F1)
            .fsub(cl)
            .fmul_m(rd::<f32>(col, 8))
            .fmul_m(f32::from_bits(0x3F66_6666))
            .to_f32();
        matrix_set_rotate_x(
            mp,
            ld(this, 0x648)
                .fabs()
                .fmul_m(f32::from_bits(0xBD4C_CCCD))
                .to_f32(),
        );
        matrix_rotate_y(mp, rd::<f32>(this, 0x648));
        restore_add(mp, s);
        matrix_update_rw(mp);
    }
}

/// `CBmx::PreRender()` (0x6C0810).
///
/// The bike's pre-render without exhaust, damage particles, siren lights or vehicle lights:
/// `CVehicle::PreRender`, the eased suspension heights, the wheel particles (the rear pair flagged
/// only while +0x810 is 3), the lean matrix, the shadow, the wheel rotations, then the frames: the
/// fork (+0x5A8) with the handlebars on +0x5B8, the swing arm (+0x5AC), the front / rear wheels
/// (+0x5B0 / +0x5B4; burst wobble 0.02 / 0.04; their offsets follow the suspension only while
/// 0xC1C83C is set, the rear one only without a swing arm), the body (+0x5A4; with 0xC1C83C clear
/// raised by the suspension blend +0x830 and pitched by `atan2(front - rear, +0x82C)`, yawed by
/// +0x81C) and the crank / pedal frames +0x5BC / +0x5C0 / +0x5C4 rotated by +0x820 / +0x824 /
/// +0x828.
#[hook(addr = 0x6c0810, conv = thiscall, name = "CBmx::PreRender", this = "CBmx",
       expect = "6A FF 68 01 84 84 00",
       route = "L2", hazards = "x87-transcendental,uninit-locals",
       recs = "driving",
       note = "fsin/fcos/fpatan via X87, _CIasin via orig::x87_unary; stack CMatrix / RwMatrix locals zeroed")]
pub fn bmx_pre_render(this: GPtr<u8>) {
    vehicle_pre_render(this);
    // L108: the collision model, L170: its data
    let col = get_col_model(this);
    let col_data = rd::<GPtr<u8>>(col, 0x2C);
    if rd::<u8>(this, 0x42B) & 1 != 0 {
        v_do_burst_and_soft_ground_ratios(this);
        suspension_display(this, col_data, 0x770, 0x780, 0x710, 0x714, 8, 0x760);
        suspension_display(this, col_data, 0x778, 0x788, 0x718, 0x71C, 0x48, 0x764);
    }
    if rd::<u8>(this, 0x36) >> 3 <= 3 {
        wheel_particles(this, true);
    }
    wr::<u8>(this, 0x5C8, 0);
    calculate_lean_matrix(this);
    store_shadow_for_vehicle(this, 1);
    // L16c: the frame matrix
    let mut l16c = StackMatrix::new();
    let mi = crate::common::model_info(i32::from(rd::<i16>(this, 0x22)));
    wheel_rotations(this, col_data, mi);
    bmx_frames(this, mi, col, &mut l16c);
    matrix_dtor(l16c.ptr());
}

/// 0x6C0DA1..0x6C143F: the BMX's moving frames.
fn bmx_frames(this: GPtr<u8>, mi: GPtr<u8>, col: GPtr<u8>, l16c: &mut StackMatrix) {
    let mp = l16c.ptr();
    let save = |mp: GPtr<u8>| rv(mp, 0x30);
    let restore_add = |mp: GPtr<u8>, s: CVector| {
        wr::<f32>(mp, 0x30, ld(mp, 0x30).fadd_m(s.x).to_f32());
        wr::<f32>(mp, 0x34, ld(mp, 0x34).fadd_m(s.y).to_f32());
        wr::<f32>(mp, 0x38, ld(mp, 0x38).fadd_m(s.z).to_f32());
    };
    let follow = g::<u8>(0xC1_C83C).read() != 0;
    let fork = frame_at(this, 0x5A8);
    if !fork.is_null() {
        matrix_attach(mp, fork.byte_add(0x10), 0);
        let s = save(mp);
        let mut rw = [0u32; 0x10];
        let mut ldc = StackMatrix::new();
        ldc.0[0x10] = GPtr::from_raw(rw.as_mut_ptr()).addr() as u32;
        let dp = ldc.ptr();
        matrix_set_unity(dp);
        matrix_update_rw(dp);
        let a = ld(mi, 0x58).fmul_m(f32::from_bits(0x3C8E_FA35));
        let mut axis = CVector::new(0.0, a.fsin().to_f32(), a.fcos().fchs().to_f32());
        vector_normalise(GPtr::<u8>::cast::<[f32;3]>(gp(&mut axis)));
        let mut q = [0f32; 4];
        quaternion_set(gp(&mut q), gp(&mut axis), ld(this, 0x644).fchs().to_f32());
        quaternion_get(gp(&mut q), GPtr::from_raw(rw.as_mut_ptr()).cast());
        matrix_update(dp);
        matrix_set_unity(mp);
        let mut l94 = StackMatrix::new();
        let r = matrix_mul(l94.ptr(), mp, dp);
        matrix_assign(mp, r);
        matrix_dtor(l94.ptr());
        restore_add(mp, s);
        matrix_update_rw(mp);
        let bars = frame_at(this, 0x5B8);
        if !bars.is_null() {
            matrix_attach(mp, bars.byte_add(0x10), 0);
            let s = save(mp);
            let st = rd::<u8>(this, 0x36) >> 3;
            if st != 4 && st != 5 {
                matrix_set_translate(mp, s.x, s.y, s.z);
            } else {
                matrix_set_unity(mp);
                matrix_mul_assign(mp, dp);
                restore_add(mp, s);
            }
            matrix_update_rw(mp);
        }
        matrix_dtor(dp);
    }
    let arm = frame_at(this, 0x5AC);
    if !arm.is_null() {
        let v = ld(this, 0x764)
            .fsub_m(rd::<f32>(this, 0x76C))
            .fdiv_m(rd::<f32>(this, 0x798));
        let a = X87::from_f64_exact(sa_core::orig::x87_unary(0x82_1E70, v.to_f64()))
            .fmul_m(-1.0)
            .to_f32();
        matrix_attach(mp, arm.byte_add(0x10), 0);
        let s = save(mp);
        matrix_set_rotate(mp, a, 0.0, 0.0);
        restore_add(mp, s);
        matrix_update_rw(mp);
    }
    matrix_attach(mp, frame_at(this, 0x5B0).byte_add(0x10), 0);
    let mut s = save(mp);
    if follow {
        let lines = rd::<GPtr<u8>>(rd::<GPtr<u8>>(col, 0x2C), 0x10);
        s.z = ld(this, 0x760).fsub_m(rd::<f32>(this, 0x7A0)).to_f32();
        s.y = ld(lines, 0x24)
            .fadd_m(rd::<f32>(lines, 4))
            .fmul_m(0.5)
            .fsub_m(rd::<f32>(this, 0x79C))
            .fsub(
                ld(this, 0x760)
                    .fsub_m(rd::<f32>(this, 0x768))
                    .fmul_m(rd::<f32>(this, 0x7A4)),
            )
            .to_f32();
    }
    if rd::<u8>(this, 0x65C) == 1 {
        let r = rd::<f32>(this, 0x750);
        matrix_set_rotate(
            mp,
            r,
            0.0,
            x(r).fsin().fmul_m(f32::from_bits(0x3CA3_D70A)).to_f32(),
        );
    } else {
        matrix_set_rotate_x(mp, rd::<f32>(this, 0x750));
    }
    restore_add(mp, s);
    matrix_update_rw(mp);
    matrix_attach(mp, frame_at(this, 0x5B4).byte_add(0x10), 0);
    let mut s = save(mp);
    if follow && frame_at(this, 0x5AC).is_null() {
        s.z = rd::<f32>(this, 0x764);
    }
    if rd::<u8>(this, 0x65D) == 1 {
        let r = rd::<f32>(this, 0x754);
        matrix_set_rotate(
            mp,
            r,
            0.0,
            x(r).fsin().fmul_m(f32::from_bits(0x3D23_D70A)).to_f32(),
        );
    } else {
        matrix_set_rotate_x(mp, rd::<f32>(this, 0x754));
    }
    restore_add(mp, s);
    matrix_update_rw(mp);
    let body = frame_at(this, 0x5A4);
    if !body.is_null() {
        // L174: the body lift, L124: the body pitch
        let mut lift = 0.0f32;
        let mut pitch = 0.0f32;
        if !follow {
            let front = ld(this, 0x760).fsub_m(rd::<f32>(this, 0x768));
            let rear = ld(this, 0x764).fsub_m(rd::<f32>(this, 0x76C));
            lift = c(F1)
                .fsub_m(rd::<f32>(this, 0x830))
                .fmul(front)
                .fadd(rear.fmul_m(rd::<f32>(this, 0x830)))
                .to_f32();
            pitch = front.fsub(rear).fpatan(ld(this, 0x82C)).to_f32();
        }
        matrix_attach(mp, body.byte_add(0x10), 0);
        let mut s = save(mp);
        let cl = ld(this, 0x648).fcos();
        s.z = c(F1)
            .fsub(cl)
            .fmul_m(rd::<f32>(col, 8))
            .fmul_m(f32::from_bits(0x3F66_6666))
            .fadd_m(lift)
            .to_f32();
        matrix_set_rotate_x(
            mp,
            ld(this, 0x648)
                .fabs()
                .fmul_m(f32::from_bits(0xBD4C_CCCD))
                .fadd_m(pitch)
                .to_f32(),
        );
        matrix_rotate_y(mp, ld(this, 0x81C).fadd_m(rd::<f32>(this, 0x648)).to_f32());
        restore_add(mp, s);
        matrix_update_rw(mp);
    }
    for (frame, angle) in [(0x5BCisize, 0x820isize), (0x5C0, 0x824), (0x5C4, 0x828)] {
        let f = frame_at(this, frame);
        if f.is_null() {
            continue;
        }
        matrix_attach(mp, f.byte_add(0x10), 0);
        let s = save(mp);
        matrix_set_rotate(mp, rd::<f32>(this, angle), 0.0, 0.0);
        restore_add(mp, s);
        matrix_update_rw(mp);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_BIKE_PRE_RENDER, HOOK_BMX_PRE_RENDER];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
