//! WRITER-VEH-BIG (batch `vehbig-1`): `CVehicle::DoVehicleLights`, head / tail light state, the
//! headlight coronas and the light point lights. Ported from the 1.0 US bytes; gta-reversed as a
//! structural guide only.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;
use sa_world::world::common::find_player_vehicle;

use crate::common::{ftol32, g, rd, wr, x};

use crate::bindings::get_light_status;
use crate::bindings::has_valid_45c;
use sa_world::bindings::register_corona;
use sa_world::bindings::add_light;
use sa_core::bindings::matrix_mul_vec as matrix_mul_vector;
use sa_core::bindings::vector_normalise_and_mag as normalise_and_mag;
use sa_world::bindings::max_f;
orig! {
    /// `CVehicle::GetVehicleLightsStatus()` (0x6D55C0)
    fn get_vehicle_lights_status(v: GPtr<u8>) -> u8 = thiscall 0x6D55C0 => crate::vehicle2::get_vehicle_lights_status;
    /// `CVehicle::DoTailLightEffect(int32, CMatrix&, uint8, uint8, uint32, uint8)` (0x6E1780)
    fn do_tail_light_effect(v: GPtr<u8>, id: i32, m: GPtr<u8>, right: u8, forced_off: u8, flags: u32, on: u8) -> u8 = thiscall 0x6E1780 => crate::mvpb_a2::do_tail_light_effect;
    /// `CVehicle::DoHeadLightEffect(int32, CMatrix&, uint8, uint8)` (0x6E0A50)
    fn do_head_light_effect(v: GPtr<u8>, id: i32, m: GPtr<u8>, right: u8, forced_off: u8) -> u8 = thiscall 0x6E0A50 => crate::mvpb_a3::do_head_light_effect;
    /// `CVehicle::DoHeadLightReflection(CMatrix&, uint32, uint8, uint8)` (0x6E1720)
    fn do_head_light_reflection(v: GPtr<u8>, m: GPtr<u8>, flags: u32, left: u32, right: u32) = thiscall 0x6E1720 => crate::vehicle2::do_head_light_reflection;
}

/// Phases B/C side of `operator*(CMatrix, CVector)` 0x59C890.
pub fn matrix_mul_vector_native(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> {
    let mat = sa_math::matrix::CMatrix {
        right: rv(m, 0),
        forward: rv(m, 0x10),
        up: rv(m, 0x20),
        pos: rv(m, 0x30),
        ..Default::default()
    };
    let r = sa_math::vector::matrix_mul_vector(&mat, &rv(v, 0));
    wv(out, r);
    out
}

/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7_CB5C;

fn rv(p: GPtr<u8>, off: isize) -> CVector {
    CVector::new(rd::<f32>(p, off), rd::<f32>(p, off + 4), rd::<f32>(p, off + 8))
}

pub fn wv(p: GPtr<u8>, v: CVector) {
    wr::<f32>(p, 0, v.x);
    wr::<f32>(p, 4, v.y);
    wr::<f32>(p, 8, v.z);
}

use sa_core::field::ldx as ld;

use sa_math::fpu::xc as c;

use sa_math::fpu::le_or_nan as le;

use sa_math::fpu::ge_or_nan as ge;

use sa_world::world::common::entity_pos as pos_ptr;

use sa_world::world::mvpd_w3::cam_position as cam_pos;
/// `CVehicle::DoVehicleLights(CMatrix& matrix, uint32 nLightFlags)` (0x6E1A60, `ret 8`).
///
/// Nothing while the global 0xC1CC18 is set. The lights bit (+0x428 bit 6) follows
/// `GetVehicleLightsStatus`, except: wrecked cars keep it, and abandoned cars (not subclass 6)
/// only switch off, and only when the camera is more than 100 (|dx| + |dy|) away. The bomb state
/// (+0x4A8 bits 3..4) forces lights off (1) or flashing (2); a sounding horn/siren flashes them
/// with the time bit 8. The ZR-350-style pop-up lights (0x1DD) raise +0x958 by `timeStep * 0.01`
/// up to 0.698 (lights stay off until fully up) or lower it to 0. Light status flags +0x584 bits
/// 0..3 are recomputed. Head lights are working unless broken (damage-manager light 1 / 0 for the
/// second, only with flag 1; flag 4 forces working; flags 0x10 / 0x20 switch the front / rear
/// pair off). With the engine bit (+0x428 bit 4): lights off and not flashing draw only the tail
/// lights; else both headlight effects, for each lit headlight with a model position a corona
/// pair (a far 0.1/0.3 corona when the camera faces it within 40 m and the facing exceeds 0.9 /
/// 0.85, and the 0x80-alpha glow sized `(1 - d / 150) * facing * g 0x8D3684`, brightness by the
/// handling flag 22), the tail lights, the headlight reflection, a point light ahead (fog type 1
/// when slow) and, while braking with a driver, the red brake point light behind.
///
/// The original reuses its matrix argument slot for the facing value (`A4`).
#[hook(addr = 0x6e1a60, conv = thiscall, name = "CVehicle::DoVehicleLights", this = "CVehicle",
       expect = "A0 18 CC C1 00",
       route = "L2", hazards = "x87-transcendental,stack-residue",
       recs = "driving,car-ai,grove-walk",
       note = "fsin/fcos of the heading without a matrix via X87; the DoTailLightEffect/DoHeadLightReflection bool dwords carry register / stack residue above the byte in the original (zero-extended here)")]
pub fn vehicle_do_vehicle_lights(this: GPtr<u8>, matrix: GPtr<u8>, flags: u32) {
    if g::<u8>(0xC1_CC18).read() != 0 {
        return;
    }
    let on = get_vehicle_lights_status(this);
    let bl = rd::<u8>(this, 0x428);
    let cur = (bl >> 6) & 1;
    if on != cur {
        let st = rd::<u8>(this, 0x36) >> 3;
        if st == 5 {
        } else if st == 4 && rd::<u32>(this, 0x594) != 6 {
            if cur != 0 {
                let (cam, pos) = (cam_pos(), pos_ptr(this));
                let dx = ld(cam, 0).fsub_m(rd::<f32>(pos, 0));
                let dx = if ge(dx, X87::zero()) { dx } else { dx.fchs() };
                let dy = ld(cam, 4).fsub_m(rd::<f32>(pos, 4));
                let dy = if ge(dy, X87::zero()) { dy } else { dy.fchs() };
                if !le(dy.fadd(dx), c(0x42C8_0000)) {
                    wr::<u8>(this, 0x428, bl & 0xBF);
                }
            }
        } else {
            wr::<u8>(this, 0x428, (((on << 6) ^ bl) & 0x40) ^ bl);
        }
    }
    // L39: flashing, L3a: horn off phase
    let mut flash = false;
    let mut off_phase = false;
    let bomb = (rd::<u8>(this, 0x4A8) >> 3) & 3;
    if bomb == 1 {
        wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xBF);
    }
    if bomb == 2 {
        flash = true;
    }
    let horn = rd::<u16>(this, 0x45C);
    if horn != 0 && horn != 0xFFFF && rd::<u8>(this, 0x36) & 0xF8 != 0x28 {
        if g::<u32>(0xB7_CB84).read() & 0x100 != 0 {
            flash = true;
        } else {
            off_phase = true;
            flash = false;
        }
    }
    if rd::<u16>(this, 0x22) == 0x1DD
        && rd::<u8>(this, 0x36) & 0xF8 != 0x28
        && rd::<u8>(this, 0x428) & 0x10 != 0
    {
        let ts = g::<f32>(TIME_STEP).read();
        if rd::<u8>(this, 0x428) & 0x40 == 0 && !flash && has_valid_45c(this) as u8 == 0 {
            if !le(ld(this, 0x958), X87::zero()) {
                let v = x(ts)
                    .fmul_m(f32::from_bits(0x3C23_D70A))
                    .fsubr_m(rd::<f32>(this, 0x958))
                    .to_f32();
                let r = X87::from_f64_exact(max_f(0.0, v).to_f64());
                wr::<f32>(this, 0x958, r.to_f32());
            }
        } else {
            let mut v = x(ts)
                .fmul_m(f32::from_bits(0x3C23_D70A))
                .fadd_m(rd::<f32>(this, 0x958));
            let k = c(0x3F32_B8C3);
            if !ge(k, v) {
                v = k;
            }
            wr::<f32>(this, 0x958, v.to_f32());
            if !ge(v, k) {
                return;
            }
        }
    }
    wr::<u8>(this, 0x584, rd::<u8>(this, 0x584) & 0xF0);
    let dm_of = if rd::<u32>(this, 0x590) == 0 {
        this
    } else {
        GPtr::from_addr(0)
    };
    let force = flags & 4 != 0;
    let ok = |light: u32| -> bool {
        if force {
            return true;
        }
        if dm_of.is_null() {
            return false;
        }
        get_light_status(dm_of.byte_add(0x5A0), light) == 0
    };
    // L38/L34: front lights, L3c/L3b: rear lights; L30: the second light of each pair exists
    let pair = flags & 1 != 0;
    let mut front_l = ok(1);
    let mut front_r = pair && ok(0);
    let mut rear_l = ok(3);
    let mut rear_r = pair && ok(3);
    if flags & 0x10 != 0 {
        front_r = false;
        front_l = false;
    }
    if flags & 0x20 != 0 {
        rear_r = false;
        rear_l = false;
    }
    if rd::<u8>(this, 0x428) & 0x10 == 0 {
        return;
    }
    let set_bit = |this: GPtr<u8>, bit: u8, al: u8| {
        let cl = rd::<u8>(this, 0x584);
        wr::<u8>(this, 0x584, ((((al << bit) ^ cl) & (1 << bit)) ^ cl) as u8);
        rd::<u8>(this, 0x584) & (1 << bit) != 0
    };
    if rd::<u8>(this, 0x428) & 0x40 == 0 && !flash && !off_phase {
        if set_bit(
            this,
            2,
            do_tail_light_effect(this, 0, matrix, 1, u8::from(!rear_l), flags, 0),
        ) {
            do_tail_light_effect(this, 1, matrix, 1, u8::from(!rear_l), flags, 0);
        }
        if pair
            && set_bit(
                this,
                3,
                do_tail_light_effect(this, 0, matrix, 0, u8::from(!rear_r), flags, 0),
            )
        {
            do_tail_light_effect(this, 1, matrix, 0, u8::from(!rear_r), flags, 0);
        }
        return;
    }
    let l_off = off_phase || !front_l;
    set_bit(this, 0, do_head_light_effect(this, 0, matrix, 1, u8::from(l_off)));
    let r_off = off_phase || !front_r;
    set_bit(this, 1, do_head_light_effect(this, 0, matrix, 0, u8::from(r_off)));
    if rd::<u8>(this, 0x584) & 1 != 0 {
        headlight_coronas(this, matrix, l_off, false);
    }
    if rd::<u8>(this, 0x584) & 2 != 0 {
        headlight_coronas(this, matrix, r_off, true);
    }
    let rl = off_phase || !rear_l;
    if set_bit(
        this,
        2,
        do_tail_light_effect(this, 0, matrix, 1, u8::from(rl), flags, 1),
    ) {
        do_tail_light_effect(this, 1, matrix, 1, u8::from(off_phase || !rear_l), flags, 1);
    }
    if pair {
        let rr = off_phase || !rear_r;
        if set_bit(
            this,
            3,
            do_tail_light_effect(this, 0, matrix, 0, u8::from(rr), flags, 1),
        ) {
            do_tail_light_effect(this, 1, matrix, 0, u8::from(off_phase || !rear_r), flags, 1);
        }
    }
    if off_phase {
        return;
    }
    if rd::<u32>(this, 0x594) != 6 {
        do_head_light_reflection(this, matrix, flags, u32::from(front_r), u32::from(front_l));
    }
    if front_l || front_r {
        let (vx, vy) = (ld(this, 0x44), ld(this, 0x48));
        let slow = !ge(vx.fmul(vx).fadd(vy.fmul(vy)), c(0x3E4F_5C28));
        let p = rv(pos_ptr(this), 0);
        let d = rv(matrix, 0x10);
        add_light(
            1,
            f32::from_bits(p.x.to_bits()),
            f32::from_bits(p.y.to_bits()),
            f32::from_bits(p.z.to_bits()),
            f32::from_bits(d.x.to_bits()),
            f32::from_bits(d.y.to_bits()),
            f32::from_bits(d.z.to_bits()),
            f32::from_bits(0x41A0_0000),
            f32::from_bits(0x3F80_0000),
            f32::from_bits(0x3F80_0000),
            f32::from_bits(0x3F80_0000),
            u32::from(slow),
            0,
            GPtr::NULL,
        );
    }
    // the brake light point behind the car
    let (fx, fy) = (ld(matrix, 0x10).fmul_m(4.0), ld(matrix, 0x14).fmul_m(4.0));
    let fz = ld(matrix, 0x18).fmul_m(4.0).to_f32();
    let pos = pos_ptr(this);
    let behind = CVector::new(
        ld(pos, 0).fsub(fx).to_f32(),
        ld(pos, 4).fsub(fy).to_f32(),
        ld(pos, 8).fsub_m(fz).to_f32(),
    );
    if !rear_l && !rear_r {
        return;
    }
    if le(ld(this, 0x4A0), X87::zero()) || rd::<u8>(this, 0x428) & 0x20 != 0 || rd::<u32>(this, 0x460) == 0 {
        return;
    }
    let dir = CVector::new(
        ld(matrix, 0x10).fchs().to_f32(),
        ld(matrix, 0x14).fchs().to_f32(),
        ld(matrix, 0x18).fchs().to_f32(),
    );
    add_light(
        1,
        f32::from_bits(behind.x.to_bits()),
        f32::from_bits(behind.y.to_bits()),
        f32::from_bits(behind.z.to_bits()),
        f32::from_bits(dir.x.to_bits()),
        f32::from_bits(dir.y.to_bits()),
        f32::from_bits(dir.z.to_bits()),
        f32::from_bits(0x4120_0000),
        f32::from_bits(g::<u32>(0x8D_368C).read()),
        f32::from_bits(g::<u32>(0x8D_3690).read()),
        f32::from_bits(g::<u32>(0x8D_3694).read()),
        0,
        0,
        GPtr::from_addr((this.addr() as u32) as usize),
    );
}

/// 0x6E1F31..0x6E229F (left, ids +7 / +3) and 0x6E22C2..0x6E263B (right, mirrored x, ids +6 /
/// +2): the headlight corona pair.
fn headlight_coronas(this: GPtr<u8>, matrix: GPtr<u8>, off: bool, right: bool) {
    let vs = rd::<GPtr<u8>>(crate::common::model_info(i32::from(rd::<i16>(this, 0x22))), 0x5C);
    let hp = rv(vs, 0x18);
    let z = X87::zero();
    if !x(hp.x).fcom(z).test_parity(0x44)
        && !x(hp.y).fcom(z).test_parity(0x44)
        && !x(hp.z).fcom(z).test_parity(0x44)
    {
        return;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let (fx, fy, fz): (X87, f32, f32) = if m.is_null() {
        let h = ld(this, 0x10);
        (h.fsin().fchs(), h.fcos().to_f32(), 0.0)
    } else {
        (ld(m, 0x10), rd::<f32>(m, 0x14), rd::<f32>(m, 0x18))
    };
    let k = f32::from_bits(0x3D4C_CCCD);
    let oz = x(fz).fmul_m(k).to_f32();
    // the corona's local position (Lc / L18)
    let mut local = CVector::new(
        fx.fmul_m(k).fadd_m(hp.x).to_f32(),
        x(fy).fmul_m(k).fadd_m(hp.y).to_f32(),
        x(oz).fadd_m(hp.z).to_f32(),
    );
    if right {
        local.x = x(hp.x).fadd(x(hp.x)).fsubr_m(local.x).to_f32();
    }
    let lp = GPtr::from_raw(&mut local).cast::<u8>();
    let mut world = CVector::default();
    let w = matrix_mul_vector(GPtr::from_raw(&mut world).cast(), matrix, lp);
    let cam = cam_pos();
    let mut dir = CVector::new(
        ld(cam, 0).fsub_m(rd::<f32>(w, 0)).to_f32(),
        ld(cam, 4).fsub_m(rd::<f32>(w, 4)).to_f32(),
        ld(cam, 8).fsub_m(rd::<f32>(w, 8)).to_f32(),
    );
    let dist = X87::from_f64_exact(normalise_and_mag(GPtr::from_raw(&mut dir).cast()).to_f64()).to_f32();
    let facing = x(dir.z)
        .fmul_m(rd::<f32>(matrix, 0x18))
        .fadd(x(dir.y).fmul_m(rd::<f32>(matrix, 0x14)))
        .fadd(x(dir.x).fmul_m(rd::<f32>(matrix, 0x10)))
        .to_f32();
    if off || le(x(facing), z) {
        return;
    }
    let cam_mode = usize::from(g::<u8>(0xB6_F081).read()) * 0x238;
    if g::<u16>(0xB6_F1A8 + cam_mode).read() == 0x10 && find_player_vehicle(-1, 0) == this {
        return;
    }
    let facing = x(facing).fsqrt().to_f32();
    let sub6 = rd::<u32>(this, 0x594) == 6 && rd::<u16>(this, 0x22) != 0x1C1;
    let th = if sub6 { c(0x3F59_999A) } else { c(0x3F66_6666) };
    let angle = facing.to_bits();
    let bright = rd::<u32>(rd::<GPtr<u8>>(this, 0x384), 0xD0) & 0x40_0000 != 0;
    let range = || x(g::<f32>(0xB6_F118).read()).fmul_m(150.0).to_f32().to_bits();
    let (id_far, id_glow) = if right { (6u32, 2u32) } else { (7, 3) };
    if !le(x(facing), th) && !ge(x(dist), c(0x4220_0000)) {
        let (rg, b) = if bright { (0x96, 0xC3) } else { (0xA0, 0x8C) };
        let size = if sub6 { 0x3E99_999A } else { 0x3D99_999A };
        register_corona(
            (this.addr() as u32).wrapping_add(id_far),
            this,
            rg,
            rg,
            b,
            0xFF,
            lp,
            f32::from_bits(size).to_bits(),
            f32::from_bits(range()).to_bits(),
            4,
            0,
            0,
            0,
            0,
            f32::from_bits(angle).to_bits(),
            0,
            f32::from_bits(0x3E99_999A).to_bits(),
            0,
            f32::from_bits(0x4170_0000).to_bits(),
            0,
            0,
        );
    }
    let mut s = x(facing).fmul_m(0.5).fadd_m(f32::from_bits(0x3E99_999A));
    let mut size = x(dist)
        .fmul_m(f32::from_bits(0x3BDA_740E))
        .fsubr_m(1.0)
        .fmul_m(facing)
        .fmul_m(g::<f32>(0x8D_3684).read())
        .to_f32();
    if sub6 {
        s = s.fadd(s);
        if !ge(c(0x3F80_0000), s) {
            s = c(0x3F80_0000);
        }
        size = x(size).fmul_m(4.0).to_f32();
    }
    let (rg, b) = if bright {
        (ftol32(c(0x433E_0000).fmul(s)), ftol32(s.fmul_m(255.0)))
    } else {
        (ftol32(c(0x4352_0000).fmul(s)), ftol32(s.fmul_m(195.0)))
    };
    register_corona(
        (this.addr() as u32).wrapping_add(id_glow),
        this,
        rg as u32,
        rg as u32,
        b as u32,
        0x80,
        lp,
        f32::from_bits(size.to_bits()).to_bits(),
        f32::from_bits(range()).to_bits(),
        1,
        0,
        1,
        0,
        0,
        f32::from_bits(angle).to_bits(),
        0,
        f32::from_bits(0x3F00_0000).to_bits(),
        0,
        f32::from_bits(0x4170_0000).to_bits(),
        0,
        0,
    );
}

pub const HOOKS: &[HookReg] = &[HOOK_VEHICLE_DO_VEHICLE_LIGHTS];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
