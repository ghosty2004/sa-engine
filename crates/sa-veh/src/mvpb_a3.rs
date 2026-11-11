//! WRITER-MVP-B (helper A, batch `mvpb-veh-3`): vehicle functions on the MVP call path. Ported from
//! the 1.0 US bytes; gta-reversed and docs/re/specs/VEH as reading aids. Where they differ, the bytes
//! win.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, g, rd, wr, x, xi};

orig! {
    /// `CVehicle::Render()` (0x6D0E60)
    fn vehicle_render(v: GPtr<u8>) = thiscall 0x6D0E60 => crate::vehicle2::render;
    /// `operator*(const CMatrix&, const CVector&)` (0x59C890, returns the out pointer)
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CVector::NormaliseAndMag()` (0x59C970, ST0)
    fn normalise_and_mag(v: GPtr<u8>) -> f32 = thiscall 0x59C970 => unported;
    /// `CCoronas::RegisterCorona(...)` (0x6FC580, 21 arguments)
    fn register_corona(id: u32, e: u32, r: u32, g: u32, b: u32, a: u32, pos: GPtr<u8>, radius: f32, far: f32, ty: u32, flare: u32, refl: u32, los: u32, unused: u32, angle: f32, long_distance: u32, near_clip: f32, fade_state: u32, fade_speed: f32, below: u32, delay: u32) = cdecl 0x6FC580 => unported;
}

/// `fcom` + `test ah, 0x41` + `jne` not taken: `a > b` (false for NaN).
fn gt(a: X87, b: X87) -> bool {
    !a.fcom(b).test_any(0x41)
}

/// `fcom` + `test ah, 5` + `jp` not taken: `a < b` (false for NaN).
fn lt(a: X87, b: X87) -> bool {
    !a.fcom(b).test_parity(0x05)
}

/// The transmission statics: velocity 0xC1CB40, cheat multiplier 0xC1CB3C, last acceleration 0xC1CB38.
const VEL: usize = 0xC1CB40;
const MULT: usize = 0xC1CB3C;
const LAST: usize = 0xC1CB38;

/// `cTransmission::CalculateDriveAcceleration(const float& gas, uint8& gear, float& gearChangeCount,
/// float& velocity, float* inertia1, float* inertia2, uint8 wheelsOnGround, uint8 cheat)`
/// (0x6D05E0, `ret 0x20`, result in ST0). The velocity goes to 0xC1CB40; below the max reverse
/// velocity (+0x60) or above the max velocity (+0x58): 0. Gear change loop: above the gear's
/// change-up velocity the gear goes up (from 0 only with gas > 0), below its change-down velocity
/// down (not from 0; from 1 only with gas < 0); after a change the original zeroes its inertia2,
/// wheels and cheat argument slots and its EBP copy of inertia1, re-checks the reverse limit and loops.
/// The gear factor (x87): 1 gear -> 1; gear 0 -> 4.5; else `(1 - (g-1)/(n-1))² * (5|4|3 by +0x4C
/// bits 0/1) + 1`; cheat 1 -> multiplier [0x8D3480] (else 1.0 in 0xC1CB3C), cheat 2 -> the "1.0"
/// factor becomes [0x8D3484]. `acc = (B * (mult * +0x50)) * A * 0.4 * gas * timeStep` (stored to
/// 0xC1CB38). With both inertia pointers: airborne (wheels == 0): `inertia1 = min(|gas| / +0x54 *
/// timeStep * [0x8D3478] + inertia1, 1)`, `inertia2 = 0.1`; on the ground: a gear ratio `r` (gear 0/1
/// from `maxVel / n * (1 - 0.667)`), `q = clamp(1 - (r - inertia1) [* 0x8D3488|0x8D348C by cheat] *
/// +0x54, 0.1, 1)`, `n = q * (1 - [0x8D347C]) + [0x8D347C] * inertia2`, `acc *= n`, inertia2 = n,
/// inertia1 = r. Finally the current gear's max velocity (scaled by the multiplier) limits it:
/// `acc *= 1 - min(over / [0x8D3494], 1)` (stored to 0xC1CB38 again).
#[hook(addr = 0x6D05E0, conv = thiscall, name = "cTransmission::CalculateDriveAcceleration", this = "cTransmission", expect = "8B 44 24 10 8B 10 89 15 40 CB C1 00",
       route = "L2", hazards = "none",
       note = "the original reuses its own argument slots as locals (cheat slot gets 1.0f, wheels/inertia2 zeroed after a gear change); the port uses locals")]
#[allow(clippy::too_many_arguments)]
pub fn calculate_drive_acceleration(
    this: GPtr<u8>,
    gas: GPtr<f32>,
    gear: GPtr<u8>,
    _gear_change_count: GPtr<f32>,
    velocity: GPtr<f32>,
    inertia1: GPtr<f32>,
    inertia2: GPtr<f32>,
    wheels: u8,
    cheat: u8,
) -> X87 {
    let vel = || x(g::<f32>(VEL).read());
    g::<u32>(VEL).write(velocity.cast::<u32>().read());
    if lt(vel(), x(rd::<f32>(this, 0x60))) {
        return x(0.0);
    }
    let (mut i1, mut i2, mut wheels, mut cheat) = (inertia1, inertia2, wheels, cheat);
    loop {
        if gt(vel(), x(rd::<f32>(this, 0x58))) {
            return x(0.0);
        }
        wr::<u32>(this, 0x64, g::<u32>(VEL).read());
        let mut dl = gear.read();
        let e = this.byte_add(isize::from(dl) * 0xC);
        if gt(vel(), x(rd::<f32>(e, 4))) {
            if dl == 0 && !gt(x(gas.read()), x(0.0)) {
                break;
            }
            dl = dl.wrapping_add(1);
        } else {
            if !lt(vel(), x(rd::<f32>(e, 8))) || dl == 0 {
                break;
            }
            if dl == 1 && !lt(x(gas.read()), x(0.0)) {
                break;
            }
            dl -= 1;
        }
        gear.write(dl);
        i1 = GPtr::NULL;
        cheat = 0;
        wheels = 0;
        i2 = GPtr::NULL;
        if lt(vel(), x(rd::<f32>(this, 0x60))) {
            return x(0.0);
        }
    }
    let n = rd::<u8>(this, 0x4A);
    let (mut a, b);
    if n == 1 {
        b = x(1.0);
        a = x(1.0);
    } else if gear.read() < 1 {
        b = x(4.5);
        a = x(1.0);
    } else {
        let t = xi(i32::from(gear.read())).fsub_m(1.0).fdiv(xi(i32::from(n)).fsub_m(1.0)).fsubr_m(1.0);
        let t = t.fmul(t);
        let flags = rd::<u32>(this, 0x4C);
        let t = if flags & 1 != 0 {
            t.fmul_m(5.0)
        } else if flags & 2 != 0 {
            t.fmul_m(4.0)
        } else {
            t.fmul_m(3.0)
        };
        b = t.fadd_m(1.0);
        a = x(1.0);
    }
    g::<f32>(MULT).write(1.0);
    if cheat == 1 {
        g::<u32>(MULT).write(g::<u32>(0x8D3480).read());
    } else if cheat == 2 {
        a = x(g::<f32>(0x8D3484).read());
    }
    let k = x(g::<f32>(MULT).read()).fmul_m(rd::<f32>(this, 0x50));
    let mut acc = b
        .fmul(k)
        .fmul(a)
        .fmul_m(f32::from_bits(0x3ECC_CCCD))
        .fmul_m(gas.read())
        .fmul_m(g::<f32>(0xB7CB5C).read());
    g::<f32>(LAST).write(f32_of(acc));
    if !i1.is_null() && !i2.is_null() {
        if wheels == 0 {
            let mut t = x(gas.read())
                .fabs()
                .fdiv_m(rd::<f32>(this, 0x54))
                .fmul_m(g::<f32>(0xB7CB5C).read())
                .fmul_m(g::<f32>(0x8D3478).read())
                .fadd_m(i1.read());
            if lt(x(1.0), t) {
                t = x(1.0);
            }
            i1.write(f32_of(t));
            i2.write(f32::from_bits(0x3DCC_CCCD));
        } else {
            let g0 = gear.read();
            let (num, den) = if g0 <= 1 {
                let u = xi(i32::from(n))
                    .fdivr_m(rd::<f32>(this, 0x58))
                    .fmul(x(1.0).fsub_m(f32::from_bits(0x3F2A_ACDA)));
                if g0 == 0 {
                    (u.fsub(vel()), u.fsub_m(rd::<f32>(this, 8)))
                } else {
                    (vel().fadd(u), u.fadd_m(rd::<f32>(this, 0x10)))
                }
            } else {
                let e = this.byte_add(isize::from(g0) * 0xC);
                (vel().fsub_m(rd::<f32>(e, 8)), x(rd::<f32>(e, 4)).fsub_m(rd::<f32>(e, 8)))
            };
            let r = num.fdiv(den);
            let mut d = r.fsub_m(i1.read());
            if cheat == 1 {
                d = d.fmul_m(g::<f32>(0x8D3488).read());
            } else if cheat == 2 {
                d = d.fmul_m(g::<f32>(0x8D348C).read());
            }
            let mut q = d.fmul_m(rd::<f32>(this, 0x54)).fsubr_m(1.0);
            let lo = x(f32::from_bits(0x3DCC_CCCD));
            if !lt(x(1.0), q) && gt(lo, q) {
                q = lo;
            } else if lt(x(1.0), q) {
                q = x(1.0);
            }
            let kk = g::<f32>(0x8D347C).read();
            let nn = q.fmul(x(1.0).fsub_m(kk)).fadd(x(kk).fmul_m(i2.read()));
            acc = acc.fmul(nn);
            g::<f32>(LAST).write(f32_of(acc));
            i2.write(f32_of(nn));
            i1.write(f32_of(r));
        }
    }
    let e = this.byte_add(isize::from(gear.read()) * 0xC);
    let over = if lt(x(rd::<f32>(e, 0)), x(0.0)) {
        let lim = x(g::<f32>(MULT).read()).fmul_m(rd::<f32>(e, 0));
        if lt(vel(), lim) { Some(lim.fsub_m(g::<f32>(VEL).read())) } else { None }
    } else {
        None
    };
    let over = match over {
        Some(v) => v,
        None => {
            if !gt(x(rd::<f32>(e, 0)), x(0.0)) {
                return acc;
            }
            let lim = x(g::<f32>(MULT).read()).fmul_m(rd::<f32>(e, 0));
            if !gt(vel(), lim) {
                return acc;
            }
            lim.fsubr_m(g::<f32>(VEL).read())
        }
    };
    let mut t = over.fdiv_m(g::<f32>(0x8D3494).read());
    if lt(x(1.0), t) {
        t = x(1.0);
    }
    acc = acc.fmul(x(1.0).fsub(t));
    g::<f32>(LAST).write(f32_of(acc));
    acc
}

/// `CVehicle::DoHeadLightEffect(int32 lightId, CMatrix& m, bool isRight, bool forcedOff)` (0x6E0A50,
/// `ret 0x10`). `P` = vehicle struct + id * 0x18; light 1 at (0,0,0): false. `F` = the matrix forward
/// (no matrix: `(-sin h, cos h, 0)` of the placement heading +0x10, x kept on the x87 stack);
/// `Q = P + F * 0.05` (x87 per component), `Q.x -= P.x + P.x` on the left side; `W = m * Q`;
/// `D = cam - W` normalised (`mag`); `dot = (Dz * fz + Dy * fy) + Dx * fx` with the matrix forward.
/// forcedOff: false. Otherwise the result is true; nothing is registered when `dot <= 0` or the active
/// camera is in mode 0x10 on the player's own vehicle. `dot = sqrt(dot)` (f32). When `dot > 0.9`
/// (0.85 for trailers, +0x594 == 6 and model != 0x1C1) and `mag < 40`: corona `this + isRight + 2 * id
/// + 4`, colour (0x96, 0x96, 0xC3) with handling flag 0x400000 (+0x384 +0xD0) else (0xA0, 0xA0, 0x8C),
/// alpha 0xFF, size 0.075 (trailers 0.3), type 4, angle dot, near clip 0.3. Then `a = dot * 0.5 + 0.3`
/// (trailers `a + a` capped at 1, size * 4), `size = (1 - mag * 0.006666) * [0x8D3684] * dot`, colour
/// `(ftol(190 a), ftol(190 a), ftol(255 a))` with the flag else `(ftol(210 a), .., ftol(195 a))`,
/// corona `this + isRight + 2 * id`, alpha 0x80, type 1, reflection on, near clip 0.5.
#[hook(addr = 0x6E0A50, conv = thiscall, name = "CVehicle::DoHeadLightEffect", this = "CVehicle", expect = "83 EC 24 53 8B 5C 24 2C",
       unverified = "mvpb-veh-3", route = "L2", hazards = "x87-transcendental",
       note = "the original keeps mag/dot/size in its own argument slots (entry esp+4/+8/+0xC); the port uses locals")]
pub fn do_head_light_effect(this: GPtr<u8>, id: i32, mat: GPtr<u8>, right: u8, forced_off: u8) -> u8 {
    let vs = rd::<GPtr<u8>>(crate::common::model_info(rd::<i16>(this, 0x22) as i32), 0x5C);
    let pp = vs.byte_add(id as isize * 0x18);
    let p = [rd::<f32>(pp, 0), rd::<f32>(pp, 4), rd::<f32>(pp, 8)];
    let ne = |v: f32| x(v).fcom(x(0.0)).test_parity(0x44);
    if id == 1 && !ne(p[0]) && !ne(p[1]) && !ne(p[2]) {
        return 0;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let (fx, fy, fz) = if !m.is_null() {
        (x(rd::<f32>(m, 0x10)), rd::<f32>(m, 0x14), rd::<f32>(m, 0x18))
    } else {
        let h = rd::<f32>(this, 0x10);
        let sx = x(h).fsin().fchs();
        (sx, f32_of(x(h).fcos()), 0.0)
    };
    let c = f32::from_bits(0x3D4C_CCCD);
    let cx = fx.fmul_m(c);
    let cy = x(fy).fmul_m(c);
    let tz = f32_of(x(fz).fmul_m(c));
    let mut q = [f32_of(cx.fadd_m(p[0])), f32_of(cy.fadd_m(p[1])), f32_of(x(tz).fadd_m(p[2]))];
    if right == 0 {
        q[0] = f32_of(x(p[0]).fadd(x(p[0])).fsubr_m(q[0]));
    }
    let cam = sa_world::world::common::cam_pos();
    let mut wv = [0f32; 3];
    let qptr = GPtr::from_raw(q.as_mut_ptr()).cast::<u8>();
    let w = matrix_mul_vec(GPtr::from_raw(wv.as_mut_ptr()).cast(), mat, qptr);
    let dz = f32_of(x(cam.wrapping_index(2).read()).fsub_m(rd::<f32>(w, 8)));
    let dy = f32_of(x(cam.wrapping_index(1).read()).fsub_m(rd::<f32>(w, 4)));
    let dx = f32_of(x(cam.read()).fsub_m(rd::<f32>(w, 0)));
    let mut d = [dx, dy, dz];
    let mag = normalise_and_mag(GPtr::from_raw(d.as_mut_ptr()).cast());
    let dot = f32_of(
        x(d[2])
            .fmul_m(rd::<f32>(mat, 0x18))
            .fadd(x(d[1]).fmul_m(rd::<f32>(mat, 0x14)))
            .fadd(x(d[0]).fmul_m(rd::<f32>(mat, 0x10))),
    );
    let base = (this.addr() as u32).wrapping_add(u32::from(right)).wrapping_add((id as u32).wrapping_mul(2));
    if forced_off != 0 {
        return 0;
    }
    if !gt(x(dot), x(0.0)) {
        return 1;
    }
    let active = usize::from(g::<u8>(0xB6F081).read());
    if g::<u16>(0xB6F1A8 + active * 0x238).read() == 0x10 && sa_world::world::common::find_player_vehicle(-1, 0) == this {
        return 1;
    }
    let sub = rd::<u32>(this, 0x594);
    let sdot = f32_of(x(dot).fsqrt());
    let trailer = || sub == 6 && rd::<u16>(this, 0x22) != 0x1C1;
    let thr = if trailer() { f32::from_bits(0x3F59_999A) } else { f32::from_bits(0x3F66_6666) };
    let blue = || rd::<u32>(rd::<GPtr<u8>>(this, 0x384), 0xD0) & 0x40_0000 != 0;
    if gt(x(sdot), x(thr)) && lt(x(mag), x(40.0)) {
        let (rg, b) = if blue() { (0x96, 0xC3) } else { (0xA0, 0x8C) };
        let size = if trailer() { f32::from_bits(0x3E99_999A) } else { f32::from_bits(0x3D99_999A) };
        let far = f32_of(x(g::<f32>(0xB6F118).read()).fmul_m(150.0));
        register_corona(base.wrapping_add(4), this.addr() as u32, rg, rg, b, 0xFF, qptr, size, far, 4, 0, 0, 0, 0, sdot, 0,
            f32::from_bits(0x3E99_999A), 0, 15.0, 0, 0);
    }
    let mut a = x(sdot).fmul_m(0.5).fadd_m(f32::from_bits(0x3E99_999A));
    let mut size = f32_of(
        x(mag)
            .fmul_m(f32::from_bits(0x3BDA_740E))
            .fsubr_m(1.0)
            .fmul_m(g::<f32>(0x8D3684).read())
            .fmul_m(sdot),
    );
    if rd::<u32>(this, 0x594) == 6 && rd::<u16>(this, 0x22) != 0x1C1 {
        a = a.fadd(a);
        if lt(x(1.0), a) {
            a = x(1.0);
        }
        size = f32_of(x(size).fmul_m(4.0));
    }
    let (rg, b) = if blue() {
        (crate::common::ftol32(x(190.0).fmul(a)) as u32, crate::common::ftol32(a.fmul_m(255.0)) as u32)
    } else {
        (crate::common::ftol32(x(210.0).fmul(a)) as u32, crate::common::ftol32(a.fmul_m(195.0)) as u32)
    };
    let far = f32_of(x(g::<f32>(0xB6F118).read()).fmul_m(150.0));
    register_corona(base, this.addr() as u32, rg, rg, b, 0x80, qptr, size, far, 1, 0, 1, 0, 0, sdot, 0, 0.5, 0, 15.0, 0, 0);
    1
}

orig! {
    /// `RwEngineInstance->fpRenderStateGet(state, void*)` (`[0xC97B24] + 0x24`)
    fn rw_render_state_get(state: u32, out: GPtr<u32>) -> u32 = cdecl table 0xC97B24 0x24 => unported;
    /// `RwEngineInstance->fpRenderStateSet(state, value)` (`[0xC97B24] + 0x20`)
    fn rw_render_state_set(state: u32, v: u32) -> u32 = cdecl table 0xC97B24 0x20 => unported;
    /// `RwHelper::GetFirstObject(RwFrame*)` (0x7348C0)
    fn get_first_object(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7348C0 => unported;
    /// `atomic->renderCallBack(atomic)` (the pointer at RpAtomic +0x48)
    fn atomic_render_callback(f: u32, a: GPtr<u8>) -> GPtr<u8> = cdecl dyn => unported;
    /// `CEntity::GetColModel()` (0x535300)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `RwMatrixTranslate(RwMatrix*, const RwV3d*, RwOpCombineType)` (0x7F2450)
    fn rw_matrix_translate(m: GPtr<u8>, v: GPtr<u8>, combine: u32) -> GPtr<u8> = cdecl 0x7F2450 => unported;
    /// `RwFrameUpdateObjects(RwFrame*)` (0x7F0910)
    fn rw_frame_update_objects(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0910 => unported;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    fn get_vehicle_appearance(v: GPtr<u8>) -> u32 = thiscall 0x6D1080 => unported;
    /// `CVehicle::DoHeadLightBeam(int32 lightId, CMatrix&, bool isRight)` (0x6E0E20)
    fn do_head_light_beam(v: GPtr<u8>, id: i32, m: GPtr<u8>, right: u32) = thiscall 0x6E0E20 => unported;
}

/// `GetFirstObject(frame)` twice (callback from the first, argument from the second) and
/// `a->renderCallBack(a)`, as the original's call sequences do.
fn render_first_object(frame: GPtr<u8>) {
    let a = get_first_object(frame);
    let b = get_first_object(frame);
    atomic_render_callback(rd::<u32>(a, 0x48), b);
}

/// `CAutomobile::Render()` (0x6A2B10, virtual +0x48; docs/re/specs/VEH/CAutomobile.md):
/// `m_nTimeTillWeNeedThisCar = now + 3000`; alpha-test reference (30) saved (local preset to 1) and set
/// to 1. With the INVISIBLE_CAR cheat (0x96914B) only the wheel atomics are rendered (frames +0x658,
/// +0x664, +0x650, +0x65C without a null check, then +0x654 and +0x660 when set), else
/// `CVehicle::Render`. The RHINO (0x1B0) draws its 8 extra wheels by moving the rear wheel frames
/// (k < 4: +0x664, else +0x658) to `c * end + f32(t * start)` (x, y), `f32(c * end.z) + t * start.z`
/// of col line 4+k (`c = max(ex[k] - (line0 - spring0) / line0, 0)` with the door-field compressions
/// ex, `t = 1 - c`), translating by `P - pos` (post-concat), updating, rendering, and translating back
/// by `D * -1` (no update). The alpha-test reference is restored; then the two head-light beams
/// (`m_renderLights` +0x584 bits 0/1) unless model 0x1B9/0xFFFE/0x1B0, an RC vehicle (+0x42C bit 7) or
/// a heli appearance (3).
#[hook(addr = 0x6A2B10, conv = thiscall, name = "CAutomobile::Render", this = "CAutomobile", expect = "83 EC 74 A1 84 CB B7 00",
       unverified = "mvpb-veh-3", route = "L2", hazards = "heap-writes")]
pub fn automobile_render(this: GPtr<u8>) {
    wr::<u32>(this, 0x4D0, g::<u32>(0xB7CB84).read().wrapping_add(0xBB8));
    let mut saved = [1u32; 1];
    rw_render_state_get(0x1E, GPtr::from_raw(saved.as_mut_ptr()));
    rw_render_state_set(0x1E, 1);
    if g::<u8>(0x96914B).read() != 0 {
        for off in [0x658, 0x664, 0x650, 0x65C] {
            render_first_object(rd::<GPtr<u8>>(this, off));
        }
        for off in [0x654, 0x660] {
            if !rd::<GPtr<u8>>(this, off).is_null() {
                render_first_object(rd::<GPtr<u8>>(this, off));
            }
        }
    } else {
        vehicle_render(this);
    }
    if rd::<u16>(this, 0x22) == 0x1B0 {
        let cd = rd::<GPtr<u8>>(get_col_model(this), 0x2C);
        let ex = [0x618, 0x61C, 0x624, 0x628, 0x630, 0x634, 0x63C, 0x640].map(|o| rd::<f32>(this, o));
        for (k, exk) in ex.iter().enumerate() {
            let mut c = x(rd::<f32>(this, 0x888))
                .fsub_m(rd::<f32>(this, 0x878))
                .fdiv_m(rd::<f32>(this, 0x888))
                .fsubr_m(*exk);
            if !x(0.0).fcom(c).test_any(0x41) {
                c = x(0.0);
            }
            let t = x(1.0).fsub(c);
            let line = rd::<GPtr<u8>>(cd, 0x10).byte_add(0x80 + 0x20 * k as isize);
            let a = f32_of(t.fmul_m(rd::<f32>(line, 0)));
            let b = f32_of(t.fmul_m(rd::<f32>(line, 4)));
            let tz = t.fmul_m(rd::<f32>(line, 8));
            let cx = c.fmul_m(rd::<f32>(line, 0x10));
            let cy = c.fmul_m(rd::<f32>(line, 0x14));
            let cz = f32_of(c.fmul_m(rd::<f32>(line, 0x18)));
            let px = f32_of(cx.fadd_m(a));
            let py = f32_of(cy.fadd_m(b));
            let pz = f32_of(x(cz).fadd(tz));
            let off = if k < 4 { 0x664 } else { 0x658 };
            let frame = rd::<GPtr<u8>>(this, off);
            let r = [rd::<f32>(frame, 0x40), rd::<f32>(frame, 0x44), rd::<f32>(frame, 0x48)];
            let mut d = [f32_of(x(px).fsub_m(r[0])), f32_of(x(py).fsub_m(r[1])), f32_of(x(pz).fsub_m(r[2]))];
            let dp = GPtr::from_raw(d.as_mut_ptr()).cast::<u8>();
            rw_matrix_translate(frame.byte_add(0x10), dp, 2);
            rw_frame_update_objects(rd::<GPtr<u8>>(this, off));
            render_first_object(rd::<GPtr<u8>>(this, off));
            let frame = rd::<GPtr<u8>>(this, off);
            for v in d.iter_mut() {
                *v = f32_of(x(*v).fmul_m(-1.0));
            }
            rw_matrix_translate(frame.byte_add(0x10), dp, 2);
        }
    }
    rw_render_state_set(0x1E, saved[0]);
    let mi = rd::<u16>(this, 0x22);
    if mi == 0x1B9 || mi == 0xFFFE || mi == 0x1B0 || rd::<u8>(this, 0x42C) & 0x80 != 0 {
        return;
    }
    if get_vehicle_appearance(this) == 3 {
        return;
    }
    if rd::<u8>(this, 0x584) & 1 != 0 {
        do_head_light_beam(this, 0, rd::<GPtr<u8>>(this, 0x14), 1);
    }
    if rd::<u8>(this, 0x584) & 2 != 0 {
        do_head_light_beam(this, 0, rd::<GPtr<u8>>(this, 0x14), 0);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_CALCULATE_DRIVE_ACCELERATION, HOOK_DO_HEAD_LIGHT_EFFECT, HOOK_AUTOMOBILE_RENDER];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
