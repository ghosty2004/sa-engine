//! FRONTIER-2 (batch `frontier2-1`): `CAutomobile::UpdateMovingCollision` (0x6A1460, user-hit at the level crossing),
//! `CVehicle::GetSpecialColModel` (0x6DF3D0) and the vertex compression it uses (0x536E90). Ported from the 1.0 US
//! bytes; FRONTIER-2 turn 9 (batch `frontier2-b21`): `CAutomobile::AddMovingCollisionSpeed` (0x6A1ED0). gta-reversed
//! differs in the dumper test (the bytes check the vehicle subclass +0x594 == 1) and reads
//! the pad's up/down only once per step; the bytes call `GetCarGunUpDown` again for the clamp and are followed.

use sa_core::abi::HookReg;
use sa_core::bindings::cross_product;
use sa_core::field::ptr_of as vp;
use sa_core::timer::TIME_STEP;
use sa_math::x87::X87;
use sa_world::bindings::multiply3x3;
use sa_core::bindings::{entity_register_reference, matrix_dtor, matrix_mul_vec, matrix_set_rotate_x, matrix_set_translate};
use sa_core::field::gf;
use sa_core::{hook, orig};
use sa_math::fpu::{fabs, gt, le_or_nan, lt};
use sa_pop::bindings::get_car_gun_up_down;
use sa_world::bindings::{get_pad_from_player, get_special_col_model as get_special_col_model_o, max_f, min_f, uncompress_vector};
use sa_world::collision::data::col_triangle_plane_set;

use crate::common::*;
use sa_core::mem::GPtr;
use sa_stream::bindings::ped_is_player;

orig! {
    /// `CCollision::RemoveTrianglePlanes(CColModel*)` (0x4185A0)
    fn remove_triangle_planes(cm: GPtr<u8>) = cdecl 0x4185A0 => unported;
    /// `CColModel::AllocateData()` (0x40F810)
    fn col_model_allocate_data(cm: GPtr<u8>) = thiscall 0x40F810 => unported;
    /// `CColModel::operator=(const CColModel&)` (0x40F7C0)
    fn col_model_assign(cm: GPtr<u8>, src: GPtr<u8>) -> GPtr<u8> = thiscall 0x40F7C0 => unported;
    /// the vertex compression 0x536E90 (`CompressedVector = CVector * 128`, `_ftol` each)
    fn compress_vector_o(dst: GPtr<u8>, src: GPtr<u8>) = cdecl 0x536E90 => compress_vector;
}

/// `CVehicle::m_aSpecialColVehicle[4]`, `m_aSpecialColModel[4]` (0x30 each), the per-slot hydraulic data (0x28 each).
const SPECIAL_VEHICLES: usize = 0xC1_CC08;
const SPECIAL_MODELS: usize = 0xC1_CC78;
const SPECIAL_HYDRAULICS: usize = 0xC1_CB68;

/// 0x536E90 (cdecl): `dst[k] = (int16)ftol(src[k] * [0x858BF4])` for x, y, z.
#[hook(addr = 0x536E90, conv = cdecl, name = "FUN_00536e90", expect = "56 8B 74 24 0C D9 06 57",
       unverified = "frontier2-1", route = "L2", hazards = "none")]
pub fn compress_vector(dst: GPtr<u8>, src: GPtr<u8>) {
    for k in 0..3isize {
        wr(dst, 2 * k, ftol32(x(rd::<f32>(src, 4 * k)).fmul_m(gf(0x85_8BF4))) as u16);
    }
}

/// `CVehicle::GetSpecialColModel()` (0x6DF3D0; its first read runs in the no-CD chunk 0x40A411). The vehicle's slot
/// (+0x48B, signed) when it still holds the vehicle: true. Else the first free of the four slots: the vehicle takes
/// it (flag 0x4000000 at +0x40, a registered reference), the slot's col model loses its triangle planes, gets its data
/// allocated once and becomes a copy of the model's; the slot's hydraulic data is reset (100.0, then four zeros).
/// No free slot: false.
#[hook(addr = 0x6DF3D0, conv = thiscall, name = "CVehicle::GetSpecialColModel", expect = "56 8B F1 90 E9 38 B0 D2 FF",
       unverified = "frontier2-1", route = "L2", hazards = "heap-writes")]
pub fn get_special_col_model(this: GPtr<u8>) -> u8 {
    let idx = rd::<i8>(this, 0x48B);
    if idx > -1 && g::<GPtr<u8>>(SPECIAL_VEHICLES + 4 * idx as usize).read() == this {
        return 1;
    }
    let Some(slot) = (0..4usize).find(|&i| g::<u32>(SPECIAL_VEHICLES + 4 * i).read() == 0) else {
        return 0;
    };
    wr(this, 0x40, rd::<u32>(this, 0x40) | 0x400_0000);
    wr::<u8>(this, 0x48B, slot as u8);
    let cell = g::<GPtr<u8>>(SPECIAL_VEHICLES + 4 * slot);
    cell.write(this);
    entity_register_reference(this, cell.cast());
    let scm = g::<u8>(SPECIAL_MODELS + 0x30 * slot);
    remove_triangle_planes(scm);
    if rd::<u32>(scm, 0x2C) == 0 {
        col_model_allocate_data(scm);
    }
    let mi = g::<GPtr<u8>>(0xA9_B0C8 + 4 * rd::<i16>(this, 0x22) as usize).read();
    col_model_assign(scm, rd::<GPtr<u8>>(mi, 0x14));
    let h = SPECIAL_HYDRAULICS + 0x28 * slot;
    g::<u32>(h).write(0x42C8_0000);
    for k in 0..4 {
        g::<u32>(h + 0x10 + 4 * k).write(0);
    }
    1
}

/// `float(v) * ts * mult * [0x858B88]`, `_ftol`, as int16 (the pad step of the moving part).
fn pad_step(v: i16, ts: f32, mult: f32) -> i16 {
    ftol32(xi(i32::from(v)).fmul_m(ts).fmul_m(mult).fmul_m(gf(0x85_8B88))) as i16
}

/// The models with a moving part: packer 0x1BB, dozer 0x1E6, dumper 0x196, cement truck 0x20C, andromada 0x250,
/// fire-truck ladder 0x220 (status test only), forklift 0x212.
fn has_misc(m: u16) -> bool {
    matches!(m, 0x1BB | 0x1E6 | 0x196 | 0x20C | 0x250 | 0x212)
}

/// `CAutomobile::UpdateMovingCollision(float angle)` (0x6A1460, `ret 4`).
///
/// See the module docs and gta-reversed's reading; the bytes' order: the previous angle (+0x86E) is copied for the
/// moving-part models; a non-player vehicle needs to be a mission vehicle with `angle >= 0` and a moving-part model, a
/// player's needs its driver to be the player outside the game-state hold (its pad is used). The cement truck and the
/// ladder only step their angle (+0x86C) and return false. The others need a special col model (0x6DF3D0); a negative
/// angle without a pad push beyond 10 returns false; the angle comes from `angle * [0x8D314C]` or the pad (clamped to
/// 0 below and to `[0x8D314C]` above); the part's rotation (or the forklift's translation) moves every triangle vertex
/// and sphere of surface 0x41 of the model's col data into the special one, the triangle planes are rebuilt, the
/// special box grows in z to the moved part; true.
#[hook(addr = 0x6A1460, conv = thiscall, name = "CAutomobile::UpdateMovingCollision", expect = "6A FF 68 A8 7E 84 00 64",
       unverified = "frontier2-1", route = "L2", hazards = "heap-writes,uninit-locals",
       note = "the component position of a model without a moving part is the original's uninitialised local (zero here); the SEH frame is not reproduced")]
#[allow(clippy::too_many_lines)]
pub fn update_moving_collision(this: GPtr<u8>, angle_bits: u32) -> u8 {
    let angle = f32::from_bits(angle_bits);
    let zero = x(gf(0x85_8B50));
    let mut pad = GPtr::<u8>::NULL;
    let model = rd::<u16>(this, 0x22);
    if has_misc(model) {
        wr(this, 0x86E, rd::<u16>(this, 0x86C));
    }
    if rd::<u8>(this, 0x36) & 0xF8 != 0 {
        if rd::<u8>(this, 0x4A4) != 2 || lt(x(angle), zero) {
            return 0;
        }
        if !(has_misc(model) || model == 0x220) {
            return 0;
        }
    } else {
        let drv = rd::<GPtr<u8>>(this, 0x460);
        if drv.is_null() || ped_is_player(drv) == 0 || g::<u8>(0x96_A8B0).read() != 0 {
            return 0;
        }
        pad = get_pad_from_player(rd::<GPtr<u8>>(this, 0x460));
    }
    let limit = || g::<i16>(0x8D_314C).read();
    let model = rd::<u16>(this, 0x22);
    if model == 0x20C || model == 0x220 {
        wr(this, 0x86E, rd::<u16>(this, 0x86C));
        if !x(angle).fcom(zero).test_any(0x01) {
            wr(this, 0x86C, ftol32(xi(i32::from(limit())).fmul_m(angle)) as u16);
            return 0;
        }
        if pad.is_null() {
            return 0;
        }
        let v = get_car_gun_up_down(pad);
        if lt(xi(i32::from(v)), x(gf(0x85_9004))) {
            let ts = gf(0xB7_CB5C);
            let v2 = get_car_gun_up_down(pad);
            let t = ftol32(xi(i32::from(v2) * 2).fmul_m(ts).fmul_m(gf(0x87_1008)).fmul_m(gf(0x85_8B88))) as i16;
            let a = i32::from(rd::<u16>(this, 0x86C)) - i32::from(t);
            let r = min_f(xi(i32::from(limit())).to_f32().to_bits(), xi(a).to_f32().to_bits());
            wr(this, 0x86C, ftol32(r) as u16);
        } else {
            if rd::<u16>(this, 0x86C) == 0 {
                return 0;
            }
            let ts = gf(0xB7_CB5C);
            let v2 = get_car_gun_up_down(pad);
            let t = ftol32(xi(i32::from(v2) + 100).fmul_m(ts).fmul_m(gf(0x87_1008)).fmul_m(gf(0x85_8B88))) as i16;
            let a = i32::from(rd::<u16>(this, 0x86C)) - i32::from(t);
            let r = max_f(0.0, xi(a).to_f32());
            wr(this, 0x86C, ftol32(r) as u16);
        }
        return 0;
    }
    let mi = g::<GPtr<u8>>(0xA9_B0C8 + 4 * i32::from(model as i16) as usize).read();
    let cm = rd::<GPtr<u8>>(mi, 0x14);
    let cd = rd::<GPtr<u8>>(cm, 0x2C);
    if get_special_col_model_o(this) == 0 {
        return 0;
    }
    let scm = g::<u8>(SPECIAL_MODELS + 0x30 * i32::from(rd::<i8>(this, 0x48B)) as usize);
    let scd = rd::<GPtr<u8>>(scm, 0x2C);
    if x(angle).fcom(zero).test_any(0x01)
        && (pad.is_null() || !gt(fabs(xi(i32::from(get_car_gun_up_down(pad)))), x(gf(0x85_862C)))) {
        wr(this, 0x86E, rd::<u16>(this, 0x86C));
        return 0;
    }
    wr(this, 0x86E, rd::<u16>(this, 0x86C));
    if !x(angle).fcom(zero).test_any(0x01) {
        wr(this, 0x86C, ftol32(xi(i32::from(limit())).fmul_m(angle)) as u16);
    } else {
        let mut mult = gf(0x87_1008);
        if rd::<u16>(this, 0x22) == 0x212 {
            mult = if !lt(xi(i32::from(get_car_gun_up_down(pad))), zero) { gf(0x87_101C) } else { gf(0x87_1018) };
        }
        let ts = gf(0xB7_CB5C);
        let s = pad_step(get_car_gun_up_down(pad), ts, mult);
        let mut v = i32::from(s) + i32::from(rd::<u16>(this, 0x86C));
        if v < 0 {
            v = 0;
        } else {
            let ts = gf(0xB7_CB5C);
            let s = pad_step(get_car_gun_up_down(pad), ts, mult);
            v = i32::from(s) + i32::from(rd::<u16>(this, 0x86C));
        }
        wr(this, 0x86C, v as u16);
        if i32::from(v as u16) > i32::from(limit()) {
            wr(this, 0x86C, limit() as u16);
        }
    }
    let mut mat = [0u32; 18];
    let mp: GPtr<u8> = GPtr::from_raw(mat.as_mut_ptr()).cast();
    let mut c = [0u32; 3];
    let angle_f = |k: usize| xi(i32::from(rd::<u16>(this, 0x86C))).fmul_m(gf(k)).to_f32();
    let node_pos = |off: isize| {
        let n = rd::<GPtr<u8>>(this, off);
        [rd::<u32>(n, 0x40), rd::<u32>(n, 0x44), rd::<u32>(n, 0x48)]
    };
    match rd::<u16>(this, 0x22) {
        0x1E6 if rd::<u32>(this, 0x698) != 0 => {
            matrix_set_rotate_x(mp, angle_f(0x87_1010));
            c = node_pos(0x698);
        }
        0x196 if rd::<u32>(this, 0x594) == 1 && rd::<u32>(this, 0x6A0) != 0 => {
            matrix_set_rotate_x(mp, angle_f(0x8D_33A8));
            c = node_pos(0x6A0);
        }
        0x250 if rd::<u32>(this, 0x6A8) != 0 => {
            matrix_set_rotate_x(mp, angle_f(0x8D_33C0));
            c = node_pos(0x6A8);
        }
        0x1BB if rd::<u32>(this, 0x698) != 0 => {
            matrix_set_rotate_x(mp, angle_f(0x87_100C));
            c = node_pos(0x698);
        }
        0x212 if rd::<u32>(this, 0x698) != 0 => {
            matrix_set_translate(mp, 0.0, 0.0, angle_f(0x87_1014));
            c = [0; 3];
        }
        _ => {}
    }
    let cf = c.map(f32::from_bits);
    let mut max_z = f32::from_bits(0xC47A_0000);
    let mut min_z = f32::from_bits(0x447A_0000);
    let track = |z: f32, max_z: &mut f32, min_z: &mut f32| {
        if !le_or_nan(x(z), x(*max_z)) {
            *max_z = z;
        } else if lt(x(z), x(*min_z)) {
            *min_z = z;
        }
    };
    // moved = rot * (p - c) + c, stored through the stack (f32)
    let moved = |p: [f32; 3]| {
        let mut d = [
            x(p[0]).fsub_m(cf[0]).to_f32(),
            x(p[1]).fsub_m(cf[1]).to_f32(),
            x(p[2]).fsub_m(cf[2]).to_f32(),
        ];
        let mut out = [0f32; 3];
        let o = matrix_mul_vec(GPtr::from_raw(out.as_mut_ptr()).cast(), mp, GPtr::from_raw(d.as_mut_ptr()).cast());
        [
            x(cf[0]).fadd_m(rd::<f32>(o, 0)).to_f32(),
            x(cf[1]).fadd_m(rd::<f32>(o, 4)).to_f32(),
            x(cf[2]).fadd_m(rd::<f32>(o, 8)).to_f32(),
        ]
    };
    let ntri = rd::<i16>(scd, 4);
    for i in 0..i32::from(ntri) {
        if rd::<u8>(rd::<GPtr<u8>>(scd, 0x18), 8 * i as isize + 6) != 0x41 {
            continue;
        }
        for k in 0..3isize {
            let vi = rd::<u16>(rd::<GPtr<u8>>(cd, 0x18), 8 * i as isize + 2 * k);
            let mut v = [0f32; 3];
            uncompress_vector(GPtr::from_raw(v.as_mut_ptr()).cast(), rd::<GPtr<u8>>(cd, 0x14).byte_add(6 * vi as isize));
            let mut p = moved(v);
            let si = rd::<u16>(rd::<GPtr<u8>>(scd, 0x18), 8 * i as isize + 2 * k);
            compress_vector_o(rd::<GPtr<u8>>(scd, 0x14).byte_add(6 * si as isize), GPtr::from_raw(p.as_mut_ptr()).cast());
            track(p[2], &mut max_z, &mut min_z);
        }
    }
    if rd::<u32>(scd, 0x1C) != 0 {
        for i in 0..i32::from(rd::<i16>(scd, 4)) {
            col_triangle_plane_set(
                rd::<GPtr<u8>>(scd, 0x1C).byte_add(0xA * i as isize).cast(),
                rd::<GPtr<u8>>(scd, 0x14).cast(),
                rd::<GPtr<u8>>(scd, 0x18).byte_add(8 * i as isize).cast(),
            );
        }
    }
    for i in 0..i32::from(rd::<i16>(scd, 0)) {
        let o = 0x14 * i as isize;
        if rd::<u8>(rd::<GPtr<u8>>(scd, 8), o + 0x10) != 0x41 {
            continue;
        }
        let src = rd::<GPtr<u8>>(cd, 8).byte_add(o);
        let p = moved([rd::<f32>(src, 0), rd::<f32>(src, 4), rd::<f32>(src, 8)]);
        let dst = rd::<GPtr<u8>>(scd, 8).byte_add(o);
        wr(dst, 0, p[0]);
        wr(dst, 4, p[1]);
        wr(dst, 8, p[2]);
        let dst = rd::<GPtr<u8>>(scd, 8).byte_add(o);
        let top = x(rd::<f32>(dst, 8)).fadd_m(rd::<f32>(dst, 0xC));
        if !le_or_nan(top, x(max_z)) {
            max_z = top.to_f32();
        } else {
            let bottom = x(rd::<f32>(dst, 8)).fsub_m(rd::<f32>(dst, 0xC));
            if lt(bottom, x(min_z)) {
                min_z = bottom.to_f32();
            }
        }
    }
    if !le_or_nan(x(max_z), x(rd::<f32>(cm, 0x14))) {
        let y = x(rd::<f32>(scm, 0x10)).to_f32();
        wr(scm, 0xC, rd::<u32>(scm, 0xC));
        wr(scm, 0x10, y);
        wr(scm, 0x14, max_z);
    }
    if lt(x(min_z), x(rd::<f32>(cm, 8))) {
        let y = x(rd::<f32>(scm, 4)).to_f32();
        wr(scm, 0, rd::<u32>(scm, 0));
        wr(scm, 4, y);
        wr(scm, 8, min_z);
    }
    matrix_dtor(mp);
    1
}

/// `CAutomobile::AddMovingCollisionSpeed(CVector& point)` (0x6A1ED0, thiscall, `ret 8`, the result
/// in the hidden return slot, EAX = it): the speed a moving part adds at `point`. Zero unless the
/// vehicle is the player's (status 0), status 9, or a mission vehicle (+0x4A4 == 2) whose part angle
/// (u16 +0x86C) or previous angle (+0x86E) is non-zero, and zero when the angle moved more than 100
/// in one frame. The forklift (530) lifts along its up axis at `(a - b) / ts * 0.0006 * 2`; the
/// Dumper (406) / Dozer (486) / Andromada (592) with their part frame turn about the right axis at
/// `(a - b) / ts * k` (k = 0.0002 / 0.0002 / 0.00015) around the frame position, the Packer (443)
/// at -0.0001 around (0, 0, 2): `CrossProduct(M * (w, 0, 0), point - M * centre)`.
#[hook(addr = 0x6A1ED0, conv = thiscall, name = "CAutomobile::AddMovingCollisionSpeed", expect = "83 EC 44 56 8B F1",
       this = "CAutomobile", unverified = "frontier2-b21", route = "L2", hazards = "none")]
pub fn automobile_add_moving_collision_speed(this: GPtr<u8>, out: GPtr<u8>, point: GPtr<u8>) -> GPtr<u8> {
    let zero = |out: GPtr<u8>| {
        for o in [0isize, 4, 8] {
            wr::<u32>(out, o, 0);
        }
        out
    };
    let status = rd::<u8>(this, 0x36) >> 3;
    if status != 0 && status != 9 {
        if rd::<u8>(this, 0x4A4) != 2 {
            return zero(out);
        }
        if rd::<u16>(this, 0x86C) == 0 && rd::<u16>(this, 0x86E) == 0 {
            return zero(out);
        }
    }
    let a = i32::from(rd::<u16>(this, 0x86C));
    let b = i32::from(rd::<u16>(this, 0x86E));
    let d = a - b;
    if !(-100..=100).contains(&d) {
        return zero(out);
    }
    let model = rd::<i16>(this, 0x22);
    let mut speed = 0u32;
    let mut centre = [0u32; 3];
    let frame_pos = |fr: GPtr<u8>| [rd::<u32>(fr, 0x40), rd::<u32>(fr, 0x44), rd::<u32>(fr, 0x48)];
    if model == 0x196 && !rd::<GPtr<u8>>(this, 0x6A0).is_null() {
        speed = 0x3951_B717;
        centre = frame_pos(rd::<GPtr<u8>>(this, 0x6A0));
    } else if model == 0x1BB {
        speed = 0xB8D1_B717;
        centre = [g::<u32>(0x8D3174).read(), g::<u32>(0x8D3178).read(), g::<u32>(0x8D317C).read()];
    } else if model == 0x1E6 && !rd::<GPtr<u8>>(this, 0x698).is_null() {
        speed = 0x3951_B717;
        centre = frame_pos(rd::<GPtr<u8>>(this, 0x698));
    } else if model == 0x250 && !rd::<GPtr<u8>>(this, 0x6A8).is_null() {
        speed = 0x391D_4952;
        centre = frame_pos(rd::<GPtr<u8>>(this, 0x6A8));
    }
    // fild a; fisub b; fdiv ts
    let rate = || X87::from_i32(a).fsub(X87::from_i32(b)).fdiv_m(TIME_STEP.get());
    if model == 0x212 {
        let up = rd::<GPtr<u8>>(this, 0x14).byte_add(0x20);
        let t = rate().fmul_m(f32::from_bits(0x3A1D_4952)).fmul_m(g::<f32>(0x8D31C0).read());
        for o in [0isize, 4, 8] {
            wr::<f32>(out, o, f32_of(t.fmul_m(rd::<f32>(up, o))));
        }
        return out;
    }
    if speed == 0 {
        return zero(out);
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let mut w = [f32_of(rate().fmul_m(f32::from_bits(speed))), 0.0f32, 0.0f32];
    let mut tmp = [0f32; 3];
    let r = multiply3x3(vp(&mut tmp), m, vp(&mut w));
    let rot = [rd::<f32>(r, 0), rd::<f32>(r, 4), rd::<f32>(r, 8)];
    let mut rot = rot;
    let mut c = centre.map(f32::from_bits);
    let mut tmp2 = [0f32; 3];
    let rc = multiply3x3(vp(&mut tmp2), m, vp(&mut c));
    let mut d = [0f32; 3];
    for (i, o) in [0isize, 4, 8].into_iter().enumerate() {
        d[i] = f32_of(x(rd::<f32>(point, o)).fsub_m(rd::<f32>(rc, o)));
    }
    let mut tmp3 = [0f32; 3];
    let res = cross_product(vp(&mut tmp3), vp(&mut rot), vp(&mut d));
    for o in [0isize, 4, 8] {
        wr::<u32>(out, o, rd::<u32>(res, o));
    }
    out
}

pub const HOOKS: &[HookReg] = &[HOOK_COMPRESS_VECTOR, HOOK_GET_SPECIAL_COL_MODEL, HOOK_UPDATE_MOVING_COLLISION,
    HOOK_AUTOMOBILE_ADD_MOVING_COLLISION_SPEED];
