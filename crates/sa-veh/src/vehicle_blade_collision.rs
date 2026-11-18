//! PORT-VEH-4 (batch `veh4-3`): `CVehicle::DoBladeCollision`. Ported from the 1.0 US bytes; gta-reversed
//! as a reading aid (the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, ftol32, g, rd, wr, x};
use crate::mvpd_v5::ped_damage_event;
use sa_core::bindings::{cross_product, entity_get_is_on_screen, general_random_int_in_range, matrix_mul_vec as mat_mul_vec, vector2d_normalise, vector_normalise, vector_normalise_and_mag};
use sa_core::field::{gf, ptr_of as vp};
use crate::mvpd_v1::read_vec as rv;
use sa_math::fpu::fbits as k;
use sa_math::vector::CVector;
use sa_stream::bindings::animate_ped_col_model_skinned;
use sa_world::bindings::{
    camera_get_game_cam_position, entity_get_col_model, fx_add_blood, fx_add_sparks, get_local_direction, localisation_blood,
    multiply3x3, physical_apply_move_force_5429f0 as apply_move_force, physical_apply_turn_force as apply_turn_force,
    physical_set_damaged_piece_record, process_col_models, report_collision, apply_soft_collision_1 as apply_soft_collision,
};
use sa_world::collision::sphere_cast::ensure_matrix;
use sa_world::world::common::entity_pos;

use sa_world::bindings::box_set;
use sa_world::bindings::col_sphere_set;
use sa_world::bindings::sphere_set;
use sa_world::bindings::world_clear_scan_codes as clear_scan_codes;
orig! {
    /// `CVehicle::BladeColSectorList(CPtrListSingleLink&, CColModel&, CMatrix&, int16 rotorType, float
    /// damageMult)` (0x6DAF00, AL)
    fn blade_col_sector_list_call(v: GPtr<u8>, list: GPtr<u8>, col: GPtr<u8>, m: GPtr<u8>, rotor: u32, mult: u32) -> u8 = thiscall 0x6DAF00 => blade_col_sector_list;
}

use sa_math::fpu::floor_f64;
/// `_ftol(floor((c +- r) * 0.02 + 60))` (the sector index the original computes).
fn sector(c: f32, r: f32, plus: bool) -> i32 {
    let v = if plus { x(c).fadd_m(r) } else { x(c).fsub_m(r) };
    let d = v.fmul_m(f32::from_bits(0x3CA3_D70A)).fadd_m(60.0).to_f64();
    ftol32(X87::from_f64_exact(floor_f64(d)))
}

/// The blade test model (0xC1CD38: its bounding box, sphere 0xC1CD50, col data 0xC1CD68 with the one
/// col sphere 0xC1CD98).
const MODEL: usize = 0xC1_CD38;

/// `CVehicle::DoBladeCollision(CVector pos, CMatrix& matrix, int16 rotorType, float radius, float
/// damageMult)` (0x6E2E50): a blade disc of `radius` at `pos` (its box flattened to the 0.05 thickness
/// at 0x8D34EC along x for rotor types +-1, y for +-2, z for +-3) becomes the static test model's box,
/// bounding sphere and single col sphere; every world sector within the disc's world-space bounds
/// (clamped to 0..119; the start index forced to 0 unless positive, computed twice as the original
/// does) is tested with `BladeColSectorList` (buildings, vehicles, peds without damage, objects) under
/// a new scan code. The model's col data is cleared again. True when any list reported a hit.
///
/// Kept as in the original: the hit flag is kept in the low byte of the rotor-type argument and the
/// sector bounds in the radius/matrix argument slots (locals here).
#[hook(addr = 0x6E2E50, conv = thiscall, name = "CVehicle::DoBladeCollision", this = "CVehicle",
       expect = "83 EC 38 8B 44 24 50",
       unverified = "veh4-3", route = "L2", hazards = "heap-writes",
       note = "writes the static blade col model 0xC1CD38..0xC1CDA8 and the scan code; BladeColSectorList damages and pushes what the blades hit")]
#[allow(clippy::too_many_arguments)]
pub fn vehicle_do_blade_collision(this: GPtr<u8>, px: f32, py: f32, pz: f32, matrix: GPtr<u8>, rotor: u32, radius: f32, mult: u32) -> u8 {
    let r = radius;
    let nr = f32_of(x(r).fchs());
    let thick = g::<f32>(0x8D_34EC).read();
    let nthick = f32_of(x(thick).fchs());
    let mut mx = [r, r, r];
    let mut mn = [nr, nr, nr];
    match rotor as i16 {
        -1 | 1 => {
            mx[0] = thick;
            mn[0] = nthick;
        }
        -2 | 2 => {
            mx[1] = thick;
            mn[1] = nthick;
        }
        -3 | 3 => {
            mx[2] = thick;
            mn[2] = nthick;
        }
        _ => {}
    }
    let p = [px, py, pz];
    let mut lo = [0u32; 3];
    let mut hi = [0u32; 3];
    for k in 0..3 {
        lo[k] = f32_of(x(p[k]).fadd_m(mn[k])).to_bits();
        hi[k] = f32_of(x(p[k]).fadd_m(mx[k])).to_bits();
    }
    let buf = |b: &mut [u32; 3]| -> GPtr<u8> { GPtr::from_raw(b as *mut [u32; 3]).cast() };
    box_set(GPtr::from_addr(MODEL), GPtr::<u8>::cast::<u32>(buf(&mut lo)), GPtr::<u8>::cast::<u32>(buf(&mut hi)));
    let mut pos = [px.to_bits(), py.to_bits(), pz.to_bits()];
    sphere_set(GPtr::from_addr(MODEL + 0x18), f32::from_bits(r.to_bits()).to_bits(), GPtr::<u8>::cast::<u32>(buf(&mut pos)));
    g::<u32>(MODEL + 0x2C).write(0xC1_CD68);
    col_sphere_set(GPtr::from_addr(MODEL + 0x60), f32::from_bits(r.to_bits()).to_bits(), GPtr::<u8>::cast::<u32>(buf(&mut pos)), 0, 0, 0xFF);
    g::<u32>(MODEL + 0x38).write(0xC1_CD98);
    g::<u16>(MODEL + 0x30).write(1);
    let mut c = [0u32; 3];
    mat_mul_vec(buf(&mut c), matrix, buf(&mut pos));
    let (cx, cy) = (f32::from_bits(c[0]), f32::from_bits(c[1]));
    let mut hit = false;
    let x0 = if sector(cx, r, false) > 0 { sector(cx, r, false) } else { 0 };
    let y0 = if sector(cy, r, false) > 0 { sector(cy, r, false) } else { 0 };
    let x1 = if sector(cx, r, true) < 0x78 { sector(cx, r, true) } else { 0x78 };
    let y1 = if sector(cy, r, true) < 0x78 { sector(cy, r, true) } else { 0x78 };
    let scan = g::<u16>(0xB7_CD78);
    if scan.read() < 0xFFFF {
        scan.write(scan.read() + 1);
    } else {
        clear_scan_codes();
        scan.write(1);
    }
    let model = GPtr::<u8>::from_addr(MODEL);
    let mut y = y0;
    while y <= y1 {
        let mut xx = x0;
        while xx <= x1 {
            let sy = if y > 0 { y } else { 0 }.min(0x77);
            let sx = if xx > 0 { xx } else { 0 }.min(0x77);
            let sector = GPtr::<u8>::from_addr(0xB7_D0B8 + 8 * (sy * 0x78 + sx) as usize);
            let rep = ((xx & 0xF) + ((y & 0xF) << 4)) as usize;
            let rep = GPtr::<u8>::from_addr(0xB9_92B8 + 12 * rep);
            if blade_col_sector_list_call(this, sector, model, matrix, rotor, mult) != 0 {
                hit = true;
            }
            if blade_col_sector_list_call(this, rep, model, matrix, rotor, mult) != 0 {
                hit = true;
            }
            if blade_col_sector_list_call(this, rep.byte_add(4), model, matrix, rotor, 0) != 0 {
                hit = true;
            }
            if blade_col_sector_list_call(this, rep.byte_add(8), model, matrix, rotor, mult) != 0 {
                hit = true;
            }
            xx += 1;
        }
        y += 1;
    }
    g::<u32>(MODEL + 0x38).write(0);
    g::<u16>(MODEL + 0x30).write(0);
    g::<u32>(MODEL + 0x2C).write(0);
    u8::from(hit)
}

/// `CWorld::m_aTempColPts` (32 col points of 0x2C bytes), `g_fx`, `AudioEngine`, `CWorld::ms_nCurrentScanCode`,
/// `CTimer::m_snTimeInMilliseconds`, the plane rotor damage time `0xC1CC1C`.
const TEMP_COL_PTS: usize = 0xB9_ACD0;
const G_FX: usize = 0xA9_AE00;
const AUDIO_ENGINE: usize = 0xB6_BC90;
const SCAN_CODE: usize = 0xB7_CD78;
const TIME_MS: usize = 0xB7_CB84;
const ROTOR_DMG_TIME: usize = 0xC1_CC1C;


/// The blades against a ped: the 2D direction from the ped to this vehicle (normalised), its local direction; a ped
/// not attached loses +0x46C bit 0 and gets `ApplyMoveForce(dir * -5.0, 5.0)`; the run-over damage event (weapon
/// 0x32, 1000.0) and, with blood on and the ped on screen, `AddBlood(ped + dir * 0.35 (z + 0.6), dir * 0.01 (z 0.01),
/// 16, ped brightness)`.
fn hit_ped(this: GPtr<u8>, ped: GPtr<u8>) {
    let pe = entity_pos(ped);
    let pt = entity_pos(this);
    let dy = f32_of(x(rd::<f32>(pt, 4)).fsub_m(rd::<f32>(pe, 4)));
    let dx = f32_of(x(rd::<f32>(pt, 0)).fsub_m(rd::<f32>(pe, 0)));
    let mut dir = [dx, dy];
    let dp = GPtr::from_raw(dir.as_mut_ptr()).cast::<u8>();
    vector2d_normalise(dp);
    let local = get_local_direction(ped, dp.cast());
    if rd::<u32>(ped, 0xFC) == 0 {
        wr::<u32>(ped, 0x46C, rd::<u32>(ped, 0x46C) & !1);
        let fy = f32_of(x(dir[1]).fmul_m(k(0xC0A0_0000)));
        let fx = f32_of(x(dir[0]).fmul_m(k(0xC0A0_0000)));
        apply_move_force(ped, CVector::new(fx, fy, 5.0));
    }
    ped_damage_event(this, ped, 0x447A_0000, 0x32, local as u32, 0, || {
        if localisation_blood() == 0 {
            return;
        }
        let mut d = CVector::new(
            f32_of(x(dir[0]).fmul_m(k(0x3C23_D70A))),
            f32_of(x(dir[1]).fmul_m(k(0x3C23_D70A))),
            f32::from_bits(0x3C23_D70A),
        );
        let p = rv(entity_pos(ped));
        let mut at = CVector::new(
            f32_of(x(dir[0]).fmul_m(k(0x3EB3_3333)).fadd_m(p.x)),
            f32_of(x(dir[1]).fmul_m(k(0x3EB3_3333)).fadd_m(p.y)),
            f32_of(x(p.z).fadd_m(k(0x3F19_999A))),
        );
        if entity_get_is_on_screen(ped) != 0 {
            fx_add_blood(GPtr::from_addr(G_FX), vp(&mut at), vp(&mut d), 16, f32::from_bits(rd::<u32>(ped, 0x12C)));
        }
    });
}

/// `CVehicle::BladeColSectorList<CPtrListSingleLink<CEntity*>>(list, CColModel& colModel, CMatrix& matrix, int16
/// rotorType, float damageMult)` (0x6DAF00, thiscall, `ret 0x14`, AL always 0; entry `push -1; jmp 0x401589`, the SEH
/// frame not modelled).
///
/// The rotor's up `U` and thickness `T` by `rotorType` (jump table 0x6DBA0C: -3 / 3 the matrix up, -2 / 2 forward,
/// -1 / 1 right, negated for the negative types; T the .data 0.2 at 0x871D9C on that axis, signed; type 0 leaves T zero
/// and U unwritten: zero here, the original's stack is undefined); `R = Multiply3x3(matrix, T)`, `C = matrix * centre`.
/// Every entity of the list but this one, with collision, not yet scanned (it gets the scan code): a ped's skinned col
/// model (else `GetColModel`), temporary objects skipped; `ProcessColModels(matrix, colModel, M(e), cm, temp col
/// points)`. A ped hit gets [`hit_ped`]. Anything else but a missile (345): with elasticity (+0x9C) set to 1.0 (.data
/// 0x871DA4, restored after), each col point with `|d . U| <= 2 * 0.05` or `<= 0.3 * |d . normal|` (d = point - C;
/// d.x through its f32, y / z unrounded in the dot) is moved onto the rotor plane `P -= U * dot`; `F = R x (P - C) +
/// moveSpeed`, `AddSparks(P, normalised F, |F| * 15, 16, 0, 1, 0.2, 1.0)`; an automobile (+0x590 0) with the rotor
/// (+0x84C) above 0.15 gets `ApplySoftCollision`, `ApplyTurnForce((R x (P - C)) * turnMass * -0.0005, P - C)` and the
/// rotor at 0.15; one below 0.075 and above 0 turns round; then `SetDamagedPieceRecord(max(100 * mass / 3000,
/// intensity) * damageMult, e, cp, 1.0)`. After any such point, unless now lies in `[t - 2000, t]` (t = 0xC1CC1C): the
/// player's vehicle (status 0) reports the collision 4 m from the camera towards P, a remote-controlled one (8) at P
/// (`ReportCollision(this, e, 0x40, 0x3F, .., 0.15, 1.0)`), and t = now + `GetRandomNumberInRange(150, 250)` (.data).
#[hook(addr = 0x6DAF00, conv = thiscall, name = "CVehicle::BladeColSectorList<CPtrListSingleLink<CEntity*>>", this = "CVehicle",
       expect = "6A FF E9 82 66 D2 FF", unverified = "frontier2-b26", route = "L2", hazards = "heap-writes",
       note = "damages, pushes and scans what the blades hit; the temp col points; the rotor damage time 0xC1CC1C")]
pub fn blade_col_sector_list(this: GPtr<u8>, list: GPtr<u8>, col: GPtr<u8>, matrix: GPtr<u8>, rotor: u32, mult: u32) -> u8 {
    let mut node = rd::<GPtr<u8>>(list, 0);
    let th = gf(0x87_1D9C);
    let neg = |v: f32| f32_of(x(v).fchs());
    let mut t = CVector::default();
    let mut u = CVector::default();
    let axis = |o: isize| rv(matrix.byte_add(o));
    let negv = |v: CVector| CVector::new(neg(v.x), neg(v.y), neg(v.z));
    match rotor as i16 {
        -3 => {
            t.z = neg(th);
            u = negv(axis(0x20));
        }
        -2 => {
            t.y = neg(th);
            u = negv(axis(0x10));
        }
        -1 => {
            t.x = neg(th);
            u = negv(axis(0));
        }
        1 => {
            t.x = th;
            u = axis(0);
        }
        2 => {
            t.y = th;
            u = axis(0x10);
        }
        3 => {
            t.z = th;
            u = axis(0x20);
        }
        _ => {}
    }
    let mut tmp = CVector::default();
    let r = rv(multiply3x3(vp(&mut tmp), matrix, vp(&mut t)));
    let mut centre = rv(col.byte_add(0x18));
    let c = rv(mat_mul_vec(vp(&mut tmp), matrix, vp(&mut centre)));
    while !node.is_null() {
        let e = rd::<GPtr<u8>>(node, 0);
        node = rd::<GPtr<u8>>(node, 4);
        if e == this || rd::<u8>(e, 0x1C) & 1 == 0 {
            continue;
        }
        let scan = g::<u16>(SCAN_CODE).read();
        if rd::<u16>(e, 0x2C) == scan {
            continue;
        }
        wr::<u16>(e, 0x2C, scan);
        let ty = |e: GPtr<u8>| rd::<u8>(e, 0x36) & 7;
        let cm = if ty(e) == 3 {
            let mi = crate::common::model_info(i32::from(rd::<i16>(e, 0x22)));
            animate_ped_col_model_skinned(mi.cast(), rd::<GPtr<u8>>(e, 0x18))
        } else {
            entity_get_col_model(e)
        };
        if ty(e) == 4 && rd::<u8>(e, 0x13C) == 3 {
            continue;
        }
        if cm.is_null() {
            continue;
        }
        let em = ensure_matrix(e);
        let n = process_col_models(matrix, col, em, cm, GPtr::from_addr(TEMP_COL_PTS), GPtr::NULL, GPtr::NULL, 0);
        if n <= 0 {
            continue;
        }
        if ty(e) == 3 {
            hit_ped(this, e);
            continue;
        }
        if rd::<i16>(e, 0x22) == 0x159 {
            continue;
        }
        blade_hits(this, e, n, &r, &c, &u, mult);
    }
    0
}

/// The col points of one non-ped entity (see [`blade_col_sector_list`]).
fn blade_hits(this: GPtr<u8>, e: GPtr<u8>, n: i32, r: &CVector, c: &CVector, u: &CVector, mult: u32) {
    let saved = rd::<u32>(this, 0x9C);
    wr::<u32>(this, 0x9C, g::<u32>(0x87_1DA4).read());
    let mut any = false;
    let mut intensity = 0f32;
    let mut p = CVector::default();
    let mut i: u32 = 0;
    while (i as i16 as i32) < n {
        let cp = GPtr::<u8>::from_addr(TEMP_COL_PTS + 0x2C * (i as i16 as i32 as usize));
        i = i.wrapping_add(1);
        p = rv(cp);
        let dxf = f32_of(x(p.x).fsub_m(c.x));
        let dyu = x(p.y).fsub_m(c.y);
        let dzu = x(p.z).fsub_m(c.z);
        let dot = f32_of(x(dxf).fmul_m(u.x).fadd(x(u.z).fmul(dzu)).fadd(x(u.y).fmul(dyu)));
        let a = x(dot).fabs();
        let lim = x(gf(0x8D_34EC)).fadd(x(gf(0x8D_34EC)));
        if !a.fcom(lim).test_any(0x41) {
            let (dyf, dzf) = (f32_of(dyu), f32_of(dzu));
            let nn = x(dzf)
                .fmul_m(rd::<f32>(cp, 0x18))
                .fadd(x(dyf).fmul_m(rd::<f32>(cp, 0x14)))
                .fadd(x(dxf).fmul_m(rd::<f32>(cp, 0x10)))
                .fabs()
                .fmul_m(gf(0x87_1DA0));
            if !a.fcom(nn).test_any(0x41) {
                continue;
            }
        }
        any = true;
        let uzd = f32_of(x(u.z).fmul_m(dot));
        p.x = f32_of(x(p.x).fsub(x(u.x).fmul_m(dot)));
        p.y = f32_of(x(p.y).fsub(x(u.y).fmul_m(dot)));
        p.z = f32_of(x(p.z).fsub_m(uzd));
        let mut v = CVector::new(f32_of(x(p.x).fsub_m(c.x)), f32_of(x(p.y).fsub_m(c.y)), f32_of(x(p.z).fsub_m(c.z)));
        let mut rr = *r;
        let mut out = CVector::default();
        let cr = rv(cross_product(vp(&mut out), vp(&mut rr), vp(&mut v)));
        let mut f = CVector::new(
            f32_of(x(cr.x).fadd_m(rd::<f32>(this, 0x44))),
            f32_of(x(cr.y).fadd_m(rd::<f32>(this, 0x48))),
            f32_of(x(cr.z).fadd_m(rd::<f32>(this, 0x4C))),
        );
        let force = f32_of(vector_normalise_and_mag(vp(&mut f)).fmul_m(k(0x4170_0000)));
        fx_add_sparks(GPtr::from_addr(G_FX), vp(&mut p), vp(&mut f), force, 16, CVector::default(), 1, f32::from_bits(0x3E4C_CCCD), 1.0);
        if rd::<u32>(this, 0x590) == 0 {
            let rs = x(rd::<f32>(this, 0x84C));
            if !rs.fcom(x(k(0x3E19_999A))).test_any(0x41) {
                apply_soft_collision(this, e, cp, GPtr::from_raw(&mut intensity as *mut f32));
                let pt = CVector::new(f32_of(x(p.x).fsub_m(c.x)), f32_of(x(p.y).fsub_m(c.y)), f32_of(x(p.z).fsub_m(c.z)));
                let km = x(gf(0x87_1DA8)).fmul_m(rd::<f32>(this, 0x90));
                let fo = CVector::new(f32_of(x(cr.x).fmul(km)), f32_of(x(cr.y).fmul(km)), f32_of(x(cr.z).fmul(km)));
                apply_turn_force(this, fo, pt);
                wr::<u32>(this, 0x84C, 0x3E19_999A);
            } else if !rs.fcom(x(gf(0x87_14A8))).test_parity(5) && !rs.fcom(x(k(0))).test_any(0x41) {
                wr::<f32>(this, 0x84C, f32_of(x(rd::<f32>(this, 0x84C)).fmul_m(k(0xBF80_0000))));
            }
        }
        let mut d = x(gf(0x87_1DAC)).fmul_m(rd::<f32>(this, 0x8C)).fmul_m(k(0x39AE_C33E));
        if d.fcom(x(intensity)).test_any(0x41) {
            d = x(intensity);
        }
        physical_set_damaged_piece_record(this, f32_of(d.fmul_m(f32::from_bits(mult))).to_bits(), e, cp, 1.0);
    }
    if any && rd::<u8>(e, 0x36) & 7 != 3 {
        let now = g::<u32>(TIME_MS).read();
        let t = g::<u32>(ROTOR_DMG_TIME).read();
        if now > t || now < t.wrapping_sub(2000) {
            let status = rd::<u8>(this, 0x36) >> 3;
            let ae = GPtr::<u8>::from_addr(AUDIO_ENGINE);
            if status == 0 {
                let cam = rv(camera_get_game_cam_position(GPtr::from_addr(0xB6_F028)));
                let mut d = CVector::new(f32_of(x(p.x).fsub_m(cam.x)), f32_of(x(p.y).fsub_m(cam.y)), f32_of(x(p.z).fsub_m(cam.z)));
                vector_normalise(vp(&mut d).cast());
                let kk = gf(0x87_1DB8);
                let s = CVector::new(f32_of(x(d.x).fmul_m(kk)), f32_of(x(d.y).fmul_m(kk)), f32_of(x(d.z).fmul_m(kk)));
                let cam = rv(camera_get_game_cam_position(GPtr::from_addr(0xB6_F028)));
                p = CVector::new(f32_of(x(s.x).fadd_m(cam.x)), f32_of(x(s.y).fadd_m(cam.y)), f32_of(x(s.z).fadd_m(cam.z)));
                report_collision(ae, this, e, 0x40, 0x3F, vp(&mut p), GPtr::NULL, gf(0x87_1DB0), 1.0, 0, 0);
            } else if status == 8 {
                report_collision(ae, this, e, 0x40, 0x3F, vp(&mut p), GPtr::NULL, gf(0x87_1DB4), 1.0, 0, 0);
            }
            let now = g::<u32>(TIME_MS).read();
            let r = general_random_int_in_range(g::<i32>(0x8D_34F0).read(), g::<i32>(0x8D_34F4).read());
            g::<u32>(ROTOR_DMG_TIME).write((r as u32).wrapping_add(now));
        }
    }
    wr::<u32>(this, 0x9C, saved);
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_BLADE_COL_SECTOR_LIST,HOOK_VEHICLE_DO_BLADE_COLLISION];
