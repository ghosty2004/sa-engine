//! WRITER-MVP-D (helper VEH, batch `mvpd-veh-6`): hydraulics and vehicle generation. Ported from the
//! 1.0 US bytes; gta-reversed as a reading aid. Where they differ, the bytes win.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, g, model_info, rd, wr, x};

orig! {
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CVehicle::GetSpecialColModel()` (0x6DF3D0)
    fn get_special_col_model(v: GPtr<u8>) -> u8 = thiscall 0x6DF3D0 => unported;
    /// `CVehicleModelInfo::GetWheelPosn(int32, CVector&, bool)` (0x4C7D20)
    fn get_wheel_posn(mi: GPtr<u8>, wheel: i32, out: GPtr<u8>, local: u32) = thiscall 0x4C7D20 => unported;
    /// `CAEVehicleAudioEntity::AddAudioEvent(eAudioEvents, float)` (0x4F6420)
    fn audio_add_event(a: GPtr<u8>, event: u32, volume: f32) = thiscall 0x4F6420 => unported;
    /// `CPlayerPed::GetPadFromPlayer()` (0x609560)
    fn get_pad_from_player(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CPad::HornJustDown()` (0x53FF30)
    fn horn_just_down(p: GPtr<u8>) -> u8 = thiscall 0x53FF30 => unported;
    /// `CPad::GetHydraulicJump()` (0x53FF70)
    fn get_hydraulic_jump(p: GPtr<u8>) -> u8 = thiscall 0x53FF70 => unported;
    /// `CPad::GetCarGunLeftRight()` (0x53FC50), int16 in AX
    fn get_car_gun_left_right(p: GPtr<u8>) -> u32 = thiscall 0x53FC50 => unported;
    /// `CPad::GetCarGunUpDown()` (0x53FC10), int16 in AX
    fn get_car_gun_up_down(p: GPtr<u8>) -> u32 = thiscall 0x53FC10 => unported;
    /// `std::max<float>` (0x420800, ST0)
    fn max_f(a: f32, b: f32) -> f64 = cdecl 0x420800 => unported;
}

/// `CVehicle::m_aSpecialHydraulicData[idx]` (0xC1CB60, stride 0x28).
fn hyd(this: GPtr<u8>) -> GPtr<u8> {
    let idx = i32::from(rd::<i8>(this, 0x48B));
    GPtr::from_addr((0xC1_CB60 + 0x28 * idx) as u32 as usize)
}

fn lt(a: X87, b: X87) -> bool {
    !a.fcom(b).test_parity(0x05)
}

fn gt(a: X87, b: X87) -> bool {
    !a.fcom(b).test_any(0x41)
}

/// One wheel of the hydraulic suspension lines (the shared loop body of the raised and lowered
/// passes, 0x6A0F60 / 0x6A1180): returns the new signed max end delta.
#[allow(clippy::too_many_arguments)]
fn hyd_wheel(this: GPtr<u8>, mi: GPtr<u8>, cd: GPtr<u8>, i: isize, factor: f32, scale: f32, base: f32, zk: f32, zb: f32, half: f32, md: f32, old_z: &mut [f32; 4]) -> f32 {
    let t = f32_of(x(scale).fmul_m(factor).fadd_m(base));
    let line = rd::<GPtr<u8>>(cd, 0x10).byte_add(32 * i);
    old_z[i as usize] = f32_of(x(rd::<f32>(this, 0x888 + 4 * i)).fmul_m(rd::<f32>(this, 0x7D4 + 4 * i)).fsubr_m(rd::<f32>(line, 8)));
    let mut wp = [0f32; 3];
    get_wheel_posn(mi, i as i32, GPtr::from_raw(wp.as_mut_ptr()).cast(), 0);
    wp[2] = f32_of(x(zk).fmul_m(factor).fadd_m(zb).fadd_m(wp[2]));
    let u = x(t).fadd_m(half);
    let line = rd::<GPtr<u8>>(cd, 0x10).byte_add(32 * i);
    wr::<f32>(line, 0, wp[0]);
    wr::<f32>(line, 4, wp[1]);
    wr::<f32>(line, 8, wp[2]);
    wp[2] = f32_of(x(wp[2]).fsub(u));
    let line = rd::<GPtr<u8>>(cd, 0x10).byte_add(32 * i);
    let delta = x(wp[2]).fsub_m(rd::<f32>(line, 0x18));
    let md = if lt(x(md).fabs(), delta.fabs()) { f32_of(delta) } else { md };
    wr::<f32>(line, 0x10, wp[0]);
    wr::<f32>(line, 0x14, wp[1]);
    wr::<f32>(line, 0x18, wp[2]);
    wr::<f32>(this, 0x888 + 4 * i, f32_of(u));
    wr::<f32>(this, 0x878 + 4 * i, t);
    if lt(x(rd::<f32>(this, 0x7D4 + 4 * i)), x(1.0)) {
        let line = rd::<GPtr<u8>>(cd, 0x10).byte_add(32 * i);
        let c = x(rd::<f32>(line, 8)).fsub_m(old_z[i as usize]).fdiv(u);
        wr::<f32>(this, 0x7D4 + 4 * i, f32_of(c));
        if gt(c, x(1.0)) {
            wr::<f32>(this, 0x7D4 + 4 * i, 1.0);
        }
    }
    md
}

/// The horn toggle pass (raise: `+0x86C` 500, lines from the raised extension; lower: 60).
#[allow(clippy::too_many_arguments)]
fn hyd_toggle(this: GPtr<u8>, mi: GPtr<u8>, cd: GPtr<u8>, raise: bool, len: f32, zoff: f32, spring: f32) {
    let mut old_z = [0f32; 4];
    for i in 0..4isize {
        let line = rd::<GPtr<u8>>(cd, 0x10).byte_add(32 * i);
        let ll = rd::<f32>(this, 0x888 + 4 * i);
        let c = rd::<f32>(this, 0x7D4 + 4 * i);
        let p = if raise { x(ll).fmul_m(c) } else { x(c).fmul_m(ll) };
        old_z[i as usize] = f32_of(p.fsubr_m(rd::<f32>(line, 8)));
        let mut wp = [0f32; 3];
        get_wheel_posn(mi, i as i32, GPtr::from_raw(wp.as_mut_ptr()).cast(), 0);
        wp[2] = f32_of(x(zoff).fadd_m(wp[2]));
        let line = rd::<GPtr<u8>>(cd, 0x10).byte_add(32 * i);
        wr::<f32>(line, 0, wp[0]);
        wr::<f32>(line, 4, wp[1]);
        wr::<f32>(line, 8, wp[2]);
        wp[2] = f32_of(x(wp[2]).fsub_m(len));
        let line = rd::<GPtr<u8>>(cd, 0x10).byte_add(32 * i);
        wr::<f32>(line, 0x10, wp[0]);
        wr::<f32>(line, 0x14, wp[1]);
        wr::<f32>(line, 0x18, wp[2]);
        wr::<f32>(this, 0x878 + 4 * i, spring);
        wr::<f32>(this, 0x888 + 4 * i, len);
        if lt(x(rd::<f32>(this, 0x7D4 + 4 * i)), x(1.0)) {
            let line = rd::<GPtr<u8>>(cd, 0x10).byte_add(32 * i);
            let c = x(rd::<f32>(line, 8)).fsub_m(old_z[i as usize]).fdiv_m(len);
            wr::<f32>(this, 0x7D4 + 4 * i, f32_of(c));
            if gt(c, x(1.0)) {
                wr::<f32>(this, 0x7D4 + 4 * i, 1.0);
            }
        }
    }
}

/// `CAutomobile::HydraulicControl()` (0x6A07A0).
///
/// Only for the player status, or the physics status (3) with +0x4A4 == 2 and a hydraulic slot
/// (+0x48B >= 0); never with handling flag 0x40000. A player driver (`IsPlayer`) is the controlling
/// ped; the player status stops while 0x96A8B0 is set. Needs `GetSpecialColModel`. The slot's data
/// (0xC1CB60 + 0x28*slot) is set up once (+8 >= 100): the lowest sphere bottom (from 0), the wheel 0
/// position, upper limit `(upper - lower) * (1.5 with geometry flag 0x10000) + lower`, the raised
/// extension, the normal/raised line limits and the wheel clearance (+0x10/+0x14, moved down by the
/// overlap with `lowest - wheel.z + wheelScale/2 - 0.1`). The pump sound timer +0x86C (< 20 counts
/// up while the speed +0x8B0 > 0.02 with gas, event 0x6E at start; counts down below 0.01, event
/// 0x6F at 0). A player horn press toggles 500 / 60: all four lines are rebuilt from the raised or
/// normal extension (compressions rescaled, clamped to 1), events 0x6D / 0x6B. Otherwise the four
/// wheel factors (1.0; the slot's stored factors +0x18.. for physics; the gun stick's polar
/// direction `max(0, ±cos/sin(atan2(lr, ud) - pi/4)) * |stick| * 1.5/128` for the player unless
/// hydraulic jump) drive the lines between the two states (raised: +0x86C counts to 504; lowered:
/// all-zero factors reload the normal limits, a non-zero factor while 0 zeroes them and sets 60),
/// tracking the largest end delta; its ratio to the extension range decides the sounds (0x6D / 0x6B)
/// and whether the compressions are rescaled (`(c - t) / (1 - t)` into +0x7E4..). The slot's
/// factors are cleared at the end.
#[hook(addr = 0x6A07A0, conv = thiscall, name = "CAutomobile::HydraulicControl", this = "CAutomobile", expect = "83 EC 60 55 8B E9",
       unverified = "mvpd-veh-6", route = "L2", hazards = "x87-transcendental,heap-writes",
       recs = "driving",
       note = "fpatan/fsin/fcos/fsqrt on the gun stick; writes the special col model's lines (heap) and the hydraulic data table 0xC1CB60")]
pub fn automobile_hydraulic_control(this: GPtr<u8>) {
    let mi = model_info(i32::from(rd::<i16>(this, 0x22)));
    let half = f32_of(x(rd::<f32>(mi, 0x40)).fmul_m(0.5));
    let status = rd::<u8>(this, 0x36) >> 3;
    if status != 0 && (status != 3 || rd::<u8>(this, 0x4A4) != 2 || rd::<i8>(this, 0x48B) < 0) {
        return;
    }
    if rd::<u32>(this, 0x38C) & 0x4_0000 != 0 {
        return;
    }
    let mut player = GPtr::<u8>::NULL;
    if status == 0 {
        let d = rd::<GPtr<u8>>(this, 0x460);
        if !d.is_null() && ped_is_player(d) != 0 {
            player = rd::<GPtr<u8>>(this, 0x460);
        }
        if g::<u8>(0x96_A8B0).read() != 0 {
            return;
        }
    }
    if get_special_col_model(this) == 0 {
        return;
    }
    let slot = i32::from(rd::<i8>(this, 0x48B));
    let cd = g::<GPtr<u8>>((0xC1_CCA4 + 0x30 * slot) as u32 as usize).read();
    let h = hyd(this);
    let (mut a, mut b, mut c, mut d, e, low);
    if !lt(x(rd::<f32>(h, 8)), x(100.0)) {
        let mut lowest = 0f32;
        let n = i32::from(rd::<i16>(cd, 0));
        let spheres = rd::<GPtr<u8>>(cd, 8);
        for j in 0..n.max(0) as isize {
            let s = spheres.byte_add(0x14 * j);
            let v = x(rd::<f32>(s, 8)).fsub_m(rd::<f32>(s, 0xC));
            if lt(v, x(lowest)) {
                lowest = f32_of(v);
            }
        }
        let mut wp = [0f32; 3];
        get_wheel_posn(mi, 0, GPtr::from_raw(wp.as_mut_ptr()).cast(), 0);
        let t = x(lowest).fsub_m(wp[2]);
        let hd = rd::<GPtr<u8>>(this, 0x384);
        b = rd::<f32>(hd, 0xBC);
        let geom = rd::<u32>(this, 0x38C) & 0x1_0000 != 0;
        let r = x(rd::<f32>(hd, 0xB8)).fsub_m(b);
        d = f32_of(if geom { r.fmul_m(1.5) } else { r.fmul_m(1.0) });
        a = f32_of(x(d).fadd_m(b));
        wr::<f32>(h, 0, a);
        wr::<f32>(h, 4, b);
        let k = if rd::<u32>(this, 0x38C) & 0x1_0000 != 0 { 0.4 } else { 0.2 };
        c = f32_of(x(a).fsub_m(k));
        let lowx = x(b).fsub_m(k);
        low = f32_of(lowx);
        let tt = x(0.0).fadd(t);
        if lt(x(c), tt) {
            c = f32_of(tt);
        }
        e = f32_of(x(c).fsub(lowx));
        wr::<f32>(h, 8, c);
        wr::<f32>(h, 0xC, f32_of(lowx));
        let q = x(1.0).fsub_m(0.7).fmul_m(d).fmul_m(0.5);
        wr::<f32>(h, 0x10, f32_of(x(a).fsub(q)));
        wr::<f32>(h, 0x14, f32_of(q.fadd_m(b)));
        let m = t.fadd_m(half).fsub_m(0.1);
        if lt(m, x(rd::<f32>(h, 0x14))) {
            let dl = m.fsubr_m(rd::<f32>(h, 0x14));
            wr::<f32>(h, 0x10, f32_of(x(rd::<f32>(h, 0x10)).fsub(dl)));
            wr::<f32>(h, 0x14, f32_of(x(rd::<f32>(h, 0x14)).fsub(dl)));
        }
    } else {
        a = rd::<f32>(h, 0);
        b = rd::<f32>(h, 4);
        d = f32_of(x(a).fsub_m(b));
        c = rd::<f32>(h, 8);
        low = rd::<f32>(h, 0xC);
        e = f32_of(x(c).fsub_m(low));
    }
    let mut sound = false;
    let cx = rd::<u16>(this, 0x86C);
    if cx < 0x14
        && gt(x(rd::<f32>(this, 0x8B0)), x(f32::from_bits(0x3CA3_D70A)))
        && x(rd::<f32>(this, 0x49C)).fcom(x(0.0)).test_parity(0x44)
    {
        if cx == 0 {
            wr::<u16>(this, 0x86C, 0x14);
            audio_add_event(this.byte_add(0x138), 0x6E, 0.0);
            sound = true;
        } else {
            wr::<u16>(this, 0x86C, cx + 1);
        }
    } else if cx > 0 && cx < 0x3D && lt(x(rd::<f32>(this, 0x8B0)), x(f32::from_bits(0x3C23_D70A))) {
        wr::<u16>(this, 0x86C, cx - 1);
        if cx - 1 == 0 {
            audio_add_event(this.byte_add(0x138), 0x6F, 0.0);
        }
    }
    let audio = this.byte_add(0x138);
    if !player.is_null() && horn_just_down(get_pad_from_player(player)) != 0 {
        let ncx: u16 = if rd::<u16>(this, 0x86C) < 0x1F4 { 0x1F4 } else { 0x3C };
        wr::<u16>(this, 0x86C, ncx);
        if ncx >= 0x1F4 {
            let len = f32_of(x(e).fadd_m(half));
            hyd_toggle(this, mi, cd, true, len, c, e);
            audio_add_event(audio, 0x6D, 0.0);
            rescale(this);
        } else {
            let len = f32_of(x(d).fadd_m(half));
            hyd_toggle(this, mi, cd, false, len, a, d);
            audio_add_event(audio, 0x6B, 0.0);
            if sound {
                rescale(this);
            }
        }
        clear_factors(this);
        return;
    }
    let mut f = [1.0f32; 4];
    let mut md = 0f32;
    if rd::<u8>(this, 0x36) & 0xF8 == 0x18 {
        for (k, v) in f.iter_mut().enumerate() {
            *v = rd::<f32>(h, 0x18 + 4 * k as isize);
        }
    } else if !player.is_null() && get_hydraulic_jump(get_pad_from_player(player)) == 0 {
        let lr = f32_of(X87::from_i32(i32::from(get_car_gun_left_right(get_pad_from_player(player)) as u16 as i16)));
        let ud = X87::from_i32(i32::from(get_car_gun_up_down(get_pad_from_player(player)) as u16 as i16));
        let mag = ud.fmul(ud).fadd(x(lr).fmul_m(lr)).fsqrt().fmul_m(1.5).fmul_m(0.007_812_5);
        let ang = x(lr).fpatan(ud).fadd_m(g::<f32>(0x8D_31BC).read());
        let cosv = f32_of(ang.fcos().fmul(mag));
        let sinv = f32_of(ang.fsin().fmul(mag));
        f[0] = f32_of(X87::from_f64_exact(max_f(0.0, f32_of(x(cosv).fchs()))));
        f[1] = f32_of(X87::from_f64_exact(max_f(0.0, f32_of(x(sinv).fchs()))));
        f[2] = f32_of(X87::from_f64_exact(max_f(0.0, sinv)));
        f[3] = f32_of(X87::from_f64_exact(max_f(0.0, cosv)));
    }
    let mut old_z = [0f32; 4];
    let cx = rd::<u16>(this, 0x86C);
    if cx >= 0x1F4 {
        if cx < 0x1F8 {
            wr::<u16>(this, 0x86C, cx + 1);
        }
        let r = f32_of(x(d).fsub_m(e));
        let ac = f32_of(x(a).fsub_m(c));
        for i in 0..4isize {
            let k = i as usize;
            if gt(x(f[k]), x(1.0)) {
                f[k] = 1.0;
            }
            md = hyd_wheel(this, mi, cd, i, f[k], r, e, ac, c, half, md, &mut old_z);
        }
    } else {
        if f.iter().all(|v| !x(*v).fcom(x(0.0)).test_parity(0x44)) {
            if cx == 0 {
                a = rd::<f32>(h, 0x10);
                b = rd::<f32>(h, 0x14);
                d = f32_of(x(a).fsub_m(b));
            }
        } else {
            if cx == 0 {
                f = [0.0; 4];
            }
            wr::<u16>(this, 0x86C, 0x3C);
        }
        let r2 = f32_of(x(e).fsub_m(d));
        let ca = f32_of(x(c).fsub_m(a));
        for i in 0..4isize {
            let k = i as usize;
            if gt(x(f[k]), x(1.0)) {
                f[k] = 1.0;
            }
            md = hyd_wheel(this, mi, cd, i, f[k], r2, d, ca, a, half, md, &mut old_z);
        }
    }
    let span = x(low).fsub_m(b);
    let span_m = f32_of(span);
    if span.fcom(x(0.0)).test_parity(0x44) && gt(x(md).fdiv_m(span_m).fabs(), X87::from_f64_exact(0.01)) {
        let mut v = x(span_m).fadd_m(md).fmul_m(0.5).fdiv_m(span_m);
        if lt(v, x(0.0)) {
            v = x(0.0);
            sound = true;
        } else if gt(v, x(1.0)) {
            v = x(1.0);
            sound = true;
        } else if lt(v, x(0.4)) || gt(v, x(0.6)) {
            sound = true;
        }
        let ts = g::<f32>(0xB7_CB5C).read();
        let lo = x(ts).fmul_m(g::<f32>(0x8D_31B8).read()).fsubr_m(0.5);
        if lt(v, lo) {
            audio_add_event(audio, 0x6B, 0.0);
        } else {
            let hi = x(ts).fmul_m(g::<f32>(0x8D_31B4).read()).fadd_m(0.5);
            if gt(v, hi) {
                audio_add_event(audio, 0x6D, 0.0);
            } else {
                let hi2 = x(ts).fmul_m(g::<f32>(0x8D_31B0).read()).fadd_m(0.5);
                if gt(v, hi2) {
                    audio_add_event(audio, 0x6B, 0.0);
                }
            }
        }
    }
    if sound {
        rescale(this);
    }
    clear_factors(this);
}

/// `+0x7E4[i] = (c - t) / (1 - t)` with `t = 1 - springLength/lineLength` (0x6A13CF).
fn rescale(this: GPtr<u8>) {
    for i in 0..4isize {
        let t = x(rd::<f32>(this, 0x878 + 4 * i)).fdiv_m(rd::<f32>(this, 0x888 + 4 * i)).fsubr_m(1.0);
        let v = x(rd::<f32>(this, 0x7D4 + 4 * i)).fsub(t).fdiv(x(1.0).fsub(t));
        wr::<f32>(this, 0x7E4 + 4 * i, f32_of(v));
    }
}

fn clear_factors(this: GPtr<u8>) {
    for k in 0..4isize {
        wr::<u32>(hyd(this), 0x18 + 4 * k, 0);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_AUTOMOBILE_HYDRAULIC_CONTROL];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
