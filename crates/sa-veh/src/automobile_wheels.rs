//! WRITER-VEH-BIG (batch `vehbig-1`): `CAutomobile::ProcessCarWheelPair`. The original is an
//! unoptimised (`/Od`-style, EBP frame) build: every intermediate goes through an f32 local and the
//! small accessors are real calls. Ported from the 1.0 US bytes; gta-reversed matches its structure.
//!
//! Inlined (pure accessors without side effects, never hooked): `GetMatrix` 0x41BD80, the identity
//! casts 0x41BD00 / 0x404350, `GetForward` 0x41BD10, `CColPoint` normal 0x411BA0 and surface-B
//! 0x47E050 getters, the surface-A setter 0x411BF0, `GetStatus` 0x4043E0, `GetModelIndex` 0x404420,
//! the time-step getter 0x41BD50, `min` 0x404330 and the `fsin`/`fcos` wrappers 0x41BCE0/0x41BCF0
//! (modelled with `X87::fsin`/`fcos`). The vector helpers are `orig!` calls.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::common::{g, rd, wr, x};

orig! {
    /// `cHandlingDataMgr::HasFrontWheelDrive(uint8)` (0x6A0480)
    fn has_front_wheel_drive(mgr: GPtr<u8>, id: u32) -> u32 = thiscall 0x6A0480 => crate::handling::has_front_wheel_drive;
    /// `cHandlingDataMgr::HasRearWheelDrive(uint8)` (0x6A04B0)
    fn has_rear_wheel_drive(mgr: GPtr<u8>, id: u32) -> u32 = thiscall 0x6A04B0 => crate::handling::has_rear_wheel_drive;
    /// `operator*(float, const CVector&)` (0x40FE90, cdecl, hidden return pointer)
    fn scalar_mul_vector(out: GPtr<u8>, s: f32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => sa_world::world::mvpa_math::scalar_mul_vector;
    /// `CPhysical::ApplyTurnForce(CVector, CVector)` (0x542A50)
    fn apply_turn_force(e: GPtr<u8>, force: CVector, point: CVector) = thiscall 0x542A50 => sa_world::physical::forces::apply_turn_force;
    /// `DotProduct(const RwV3d*, const RwV3d*)` (0x40FDB0), ST0
    fn dot_product(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => dot_product_native;
    /// `CVector::operator-=(const CVector&)` (0x406D70)
    fn vector_sub_assign(v: GPtr<u8>, o: GPtr<u8>) -> GPtr<u8> = thiscall 0x406D70 => sa_world::world::mvpa_math::vector_sub_assign;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => normalise_native;
    /// `CrossProduct(CVector*, const CVector*, const CVector*)` (0x59C730)
    fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => cross_product_native;
    /// `VectorSub(CVector*, const CVector*, const CVector*)` (0x40FE60)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => sa_world::world::mvpa_math::vector_sub;
    /// `VectorAdd(CVector*, const CVector*, const CVector*)` (0x40FE30)
    fn vector_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => sa_world::world::mvpa_math::vector_add;
    /// `SurfaceInfos_c::GetAdhesiveLimit(CColPoint&)` (0x55EB50), ST0
    fn get_adhesive_limit(si: GPtr<u8>, cp: GPtr<u8>) -> f64 = thiscall 0x55EB50 => unported;
    /// `CEntity::TreatAsPlayerForCollisions()` (0x541F70)
    fn treat_as_player_for_collisions(e: GPtr<u8>) -> u8 = thiscall 0x541F70 => sa_world::world::entity::treat_as_player_for_collisions;
    /// `SurfaceInfos_c::GetWetMultiplier(uint32)` (0x55E600), ST0
    fn get_wet_multiplier(si: GPtr<u8>, id: u32) -> f64 = thiscall 0x55E600 => unported;
    /// `SurfaceInfos_c::GetAdhesionGroup(uint32)` (0x55E5C0)
    fn get_adhesion_group(si: GPtr<u8>, id: u32) -> u32 = thiscall 0x55E5C0 => unported;
    /// `CDamageManager::GetWheelStatus(int32)` (0x6C21B0)
    fn get_wheel_status(dm: GPtr<u8>, wheel: i32) -> u32 = thiscall 0x6C21B0 => crate::damage::get_wheel_status;
    /// `CVehicle::ProcessWheel(...)` (0x6D6C00)
    fn process_wheel(v: GPtr<u8>, fwd: GPtr<u8>, right: GPtr<u8>, speed: GPtr<u8>, point: GPtr<u8>, wheels: i32, thrust: f32, brake: f32, adhesion: f32, wheel: u32, wheel_speed: GPtr<u8>, state: GPtr<i32>, status: u32) = thiscall 0x6D6C00 => crate::mvpd_v1::vehicle_process_wheel;
}

/// Phases B/C side of `DotProduct` 0x40FDB0 (sa-math's port of the same sum order).
pub fn dot_product_native(a: GPtr<u8>, b: GPtr<u8>) -> f64 {
    sa_math::vector::dot_product_x87(&rv(a, 0), &rv(b, 0)).to_f64()
}

/// Phases B/C side of `CVector::Normalise` 0x59C910.
pub fn normalise_native(v: GPtr<u8>) {
    let [a, b, c] = sa_math::vector::normalise([rd::<f32>(v, 0), rd::<f32>(v, 4), rd::<f32>(v, 8)]);
    wv(v, CVector::new(a, b, c));
}

/// Phases B/C side of `CrossProduct` 0x59C730.
pub fn cross_product_native(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> {
    let r = sa_math::vector::cross_product(&rv(a, 0), &rv(b, 0));
    wv(out, r);
    out
}

/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7_CB5C;
/// `g_surfaceInfos`
const SURFACE_INFOS: usize = 0xB7_9538;
/// `gHandlingDataMgr`
const HANDLING_MGR: usize = 0xC2_B9C8;
const F1: u32 = 0x3F80_0000;
const F2: u32 = 0x4000_0000;

fn rv(p: GPtr<u8>, off: isize) -> CVector {
    CVector::new(rd::<f32>(p, off), rd::<f32>(p, off + 4), rd::<f32>(p, off + 8))
}

pub fn wv(p: GPtr<u8>, v: CVector) {
    wr::<f32>(p, 0, v.x);
    wr::<f32>(p, 4, v.y);
    wr::<f32>(p, 8, v.z);
}

fn ld(p: GPtr<u8>, off: isize) -> X87 {
    x(rd::<f32>(p, off))
}

fn c(bits: u32) -> X87 {
    X87::constant(bits)
}

fn ts() -> f32 {
    g::<f32>(TIME_STEP).read()
}

/// `fcomp` + `test ah, 0x41` + `jne` taken: `a <= b` or unordered.
fn le(a: X87, b: X87) -> bool {
    a.fcom(b).test_any(0x41)
}

/// `fcomp` + `test ah, 5` + `jp` taken: `a >= b` or unordered.
fn ge(a: X87, b: X87) -> bool {
    a.fcom(b).test_parity(0x05)
}

/// `min(a, b)` (0x404330): `a` when `a < b`, else `b` (ties and unordered give `b`).
fn min_f(a: f32, b: f32) -> f32 {
    if ge(x(a), x(b)) { b } else { a }
}

/// `a * v` through 0x40FE90.
fn smul(s: f32, v: &mut CVector) -> CVector {
    let mut out = CVector::default();
    rv(
        scalar_mul_vector(GPtr::from_raw(&mut out).cast(), s, GPtr::from_raw(v).cast()),
        0,
    )
}

/// One wheel's contact processing (0x6A5100.. for the left wheel, 0x6A573E.. for the right one).
#[allow(clippy::too_many_arguments)]
fn wheel(
    this: GPtr<u8>,
    i: i32,
    steer: Option<(f32, f32)>,
    hydra_check: bool,
    drive: bool,
    accel: f32,
    traction: f32,
    brake: f32,
    bias: f32,
    brake_bias: f32,
    traction_bias: f32,
    speeds: GPtr<u8>,
    points: GPtr<u8>,
) {
    let ii = i as isize;
    let thrust = if drive { accel } else { 0.0 };
    let m = rd::<GPtr<u8>>(this, 0x14);
    let cp = this.byte_add(0x724 + 0x2C * ii);
    let normal = cp.byte_add(0x10);
    // V20: wheel forward, V4c: wheel right
    let mut fwd = rv(m, 0x10);
    let fp = GPtr::from_raw(&mut fwd).cast::<u8>();
    let d = X87::from_f64_exact(dot_product(fp, normal)).to_f32();
    let mut tmp = CVector::default();
    let t = scalar_mul_vector(GPtr::from_raw(&mut tmp).cast(), d, normal);
    vector_sub_assign(fp, t);
    vector_normalise(fp);
    let mut out = CVector::default();
    let mut right = rv(cross_product(GPtr::from_raw(&mut out).cast(), fp, normal), 0);
    vector_normalise(GPtr::from_raw(&mut right).cast());
    if let Some((sin, cos)) = steer
        && (!hydra_check || rd::<u16>(this, 0x22) != 0x208)
    {
        {
            let mut a = smul(sin, &mut right);
            let mut b = smul(cos, &mut fwd);
            let mut o1 = CVector::default();
            let new_fwd = rv(
                vector_sub(
                    GPtr::from_raw(&mut o1).cast(),
                    GPtr::from_raw(&mut b).cast(),
                    GPtr::from_raw(&mut a).cast(),
                ),
                0,
            );
            let mut a2 = smul(cos, &mut right);
            let mut b2 = smul(sin, &mut fwd);
            let mut o2 = CVector::default();
            right = rv(
                vector_add(
                    GPtr::from_raw(&mut o2).cast(),
                    GPtr::from_raw(&mut b2).cast(),
                    GPtr::from_raw(&mut a2).cast(),
                ),
                0,
            );
            fwd = new_fwd;
        }
    }
    wr::<u8>(cp, 0x20, 0x3C);
    let si = GPtr::from_addr(SURFACE_INFOS);
    // V50: adhesion
    let mut adh = X87::from_f64_exact(get_adhesive_limit(si, cp))
        .fmul_m(traction)
        .to_f32();
    if treat_as_player_for_collisions(this) != 0 {
        let surf = u32::from(rd::<u8>(cp, 0x23));
        adh = X87::from_f64_exact(get_wet_multiplier(si, surf))
            .fmul_m(adh)
            .to_f32();
        let h = rd::<GPtr<u8>>(this, 0x384);
        let k = c(F1)
            .fsub_m(rd::<f32>(this, 0x7D4 + 4 * ii))
            .fmul_m(4.0)
            .fmul_m(rd::<f32>(h, 0xAC))
            .fmul_m(bias)
            .to_f32();
        adh = x(min_f(2.0, k)).fmul_m(adh).to_f32();
        let flags = rd::<u32>(this, 0x38C);
        if flags & 0x20_0000 != 0 && get_adhesion_group(si, u32::from(rd::<u8>(cp, 0x23))) as i32 > 2 {
            adh = x(adh).fmul_m(f32::from_bits(0x3FB3_3333)).to_f32();
        } else if rd::<u32>(this, 0x38C) & 0x10_0000 != 0
            && get_adhesion_group(si, u32::from(rd::<u8>(cp, 0x23))) as i32 > 2
        {
            adh = x(adh).fmul_m(f32::from_bits(0x3F93_3333)).to_f32();
        }
    }
    // V34: the wheel state ProcessWheel updates
    let mut state = rd::<i32>(this, 0x968 + 4 * ii);
    let burst = get_wheel_status(this.byte_add(0x5A0), i) == 1;
    let adhesion = if burst {
        x(adh)
            .fmul_m(rd::<f32>(this, 0x5A0))
            .fmul_m(traction_bias)
            .to_f32()
    } else {
        x(adh).fmul_m(traction_bias).to_f32()
    };
    let brk = x(brake).fmul_m(brake_bias).to_f32();
    process_wheel(
        this,
        GPtr::from_raw(&mut fwd).cast(),
        GPtr::from_raw(&mut right).cast(),
        speeds.byte_add(0xC * ii),
        points.byte_add(0xC * ii),
        i32::from(rd::<u8>(this, 0x960)),
        thrust,
        brk,
        adhesion,
        i as u32 & 0xFF,
        this.byte_add(0x858 + 4 * ii),
        GPtr::from_raw(&mut state),
        u32::from(burst),
    );
    let st = if drive && !ge(ld(this, 0x49C), X87::zero()) && state == 1 {
        0
    } else {
        state
    };
    wr::<i32>(this, 0x968 + 4 * ii, st);
}

/// The airborne wheel spin (0x6A5D34.. / 0x6A5E9F..).
fn air_spin(this: GPtr<u8>, i: i32, drive: bool, accel: f32) {
    let ii = i as isize;
    let sp = 0x848 + 4 * ii;
    if !ld(this, 0x7F4 + 4 * ii).fcom(X87::zero()).test_parity(0x41) {
        if drive && x(accel).fcom(X87::zero()).test_parity(0x44) {
            if !le(x(accel), X87::zero()) {
                if !ge(ld(this, sp), c(F1)) {
                    wr::<f32>(
                        this,
                        sp,
                        ld(this, sp).fsub_m(f32::from_bits(0x3DCC_CCCD)).to_f32(),
                    );
                }
            } else if !le(ld(this, sp), c(0xBF80_0000)) {
                wr::<f32>(
                    this,
                    sp,
                    ld(this, sp).fadd_m(f32::from_bits(0x3D4C_CCCD)).to_f32(),
                );
            }
        } else {
            wr::<f32>(
                this,
                sp,
                ld(this, sp).fmul_m(f32::from_bits(0x3F73_3333)).to_f32(),
            );
        }
        let rot = 0x828 + 4 * ii;
        wr::<f32>(
            this,
            rot,
            x(ts())
                .fmul_m(rd::<f32>(this, sp))
                .fadd_m(rd::<f32>(this, rot))
                .to_f32(),
        );
    }
}

/// `CAutomobile::ProcessCarWheelPair(eCarWheel left, eCarWheel right, float steerAngle, CVector*
/// contactSpeeds, CVector* contactPoints, float traction, float acceleration, float brake, bool
/// bFront)` (0x6A4EC0, `ret 0x24`).
///
/// Drive wheels: front pair with `HasFrontWheelDrive`, rear pair with `HasRearWheelDrive`. The
/// suspension bias is `2 * handling+0xC0` (front) or `2 - that` (rear). Rear pair: the handbrake
/// (+0x428 bit 5) without flag 0x40 sets brake = 20000; else a burnout (+0x86A) on drive wheels zeroes
/// brake and traction and applies the turn force `(min(1, 3000 / turnMass) * (-0.002 * turnMass)) *
/// steer * right` at the left contact point; else without nitro (flag 19) a rear-drive car scales
/// traction by the tyre temperature +0x8A4. When either wheel has a contact counter > 0: steering
/// (when steer > -100) rotates each wheel's ground-projected forward/right frame by `sin`/`cos`
/// (the left wheel not for the Hydra 0x208); the colpoint's surface A becomes 0x3C; adhesion =
/// `GetAdhesiveLimit * traction`, for player-like entities times the wet multiplier, `min(2, ((1 -
/// compression) * 4) * suspensionForce * bias)` and 1.4 (off-road flag 21) / 1.15 (flag 20) on
/// adhesion groups above 2; brake/traction biases (`2 * bias` front, `2 - 2 * bias` rear, 1 for
/// neutral NPC handling, flag 3); `ProcessWheel` (burst tyres: adhesion also times the damage
/// manager's wheel effect); a spinning drive wheel with negative gas becomes NORMAL. Then the rear
/// pair updates the tyre temperature (burnout heats by `timeStep * 0.001` up to 3, else it decays
/// `pow(0.995, timeStep) * (t - 1) + 1`), and (not for helis) airborne wheels spin up (-0.1 below
/// 1, +0.05 above -1) or slow down (* 0.95) and rotate by `timeStep * speed`.
///
/// The original writes `brake` and `traction` into its own argument slots (kept as locals here).
#[hook(addr = 0x6a4ec0, conv = thiscall, name = "CAutomobile::ProcessCarWheelPair", this = "CAutomobile",
       expect = "55 8B EC 90 E9 A8 E9 D5 FF",
       route = "L2", hazards = "x87-transcendental,stack-residue",
       recs = "driving,car-ai",
       note = "entry is `push ebp; mov ebp, esp; nop; jmp 0x403871` (frame setup in a protection fragment); fsin/fcos via X87, pow via orig::x87_binary(0x822130); brake/traction argument slots are rewritten by the original; ProcessWheel's wheel-id dword carries ECX garbage above the byte in the original (zero-extended here)")]
#[allow(clippy::too_many_arguments)]
pub fn automobile_process_car_wheel_pair(
    this: GPtr<u8>,
    left: i32,
    right: i32,
    steer: f32,
    speeds: GPtr<u8>,
    points: GPtr<u8>,
    traction: f32,
    accel: f32,
    brake: f32,
    front: u32,
) {
    let front = front as u8 != 0;
    let mgr = GPtr::from_addr(HANDLING_MGR);
    let id = |this: GPtr<u8>| u32::from(rd::<u8>(rd::<GPtr<u8>>(this, 0x384), 0));
    let drive = if front {
        has_front_wheel_drive(mgr, id(this)) as u8 != 0
    } else {
        has_rear_wheel_drive(mgr, id(this)) as u8 != 0
    };
    let (mut traction, mut brake) = (traction, brake);
    let mut bias = c(F2)
        .fmul_m(rd::<f32>(rd::<GPtr<u8>>(this, 0x384), 0xC0))
        .to_f32();
    if !front {
        bias = c(F2).fsub_m(bias).to_f32();
        if (rd::<u8>(this, 0x428) >> 5) & 1 != 0 && rd::<u32>(this, 0x38C) & 0x40 == 0 {
            brake = f32::from_bits(0x469C_4000);
        } else if drive && rd::<u16>(this, 0x86A) != 0 {
            brake = 0.0;
            traction = 0.0;
            let tm = rd::<f32>(this, 0x90);
            let k = c(0xBB03_126F).fmul_m(tm).to_f32();
            let r = c(0x453B_8000).fdiv_m(tm).to_f32();
            let s1 = x(min_f(1.0, r)).fmul_m(k).to_f32();
            let s = x(s1).fmul_m(rd::<f32>(this, 0x494)).to_f32();
            let mut out = CVector::default();
            let force = rv(
                scalar_mul_vector(GPtr::from_raw(&mut out).cast(), s, rd::<GPtr<u8>>(this, 0x14)),
                0,
            );
            let point = rv(points, 0xC * left as isize);
            apply_turn_force(this, point, force);
        } else if rd::<u32>(this, 0x38C) & 0x8_0000 == 0 && has_rear_wheel_drive(mgr, id(this)) as u8 != 0 {
            traction = x(traction).fmul_m(rd::<f32>(this, 0x8A4)).to_f32();
        }
    }

    let wc = |i: i32| ld(this, 0x7F4 + 4 * i as isize);
    if !le(wc(left), X87::zero()) || !le(wc(right), X87::zero()) {
        let can_steer = !le(x(steer), c(0xC2C8_0000));
        let trig = if can_steer {
            let sin = x(steer).fsin().to_f32();
            let cos = x(steer).fcos().to_f32();
            Some((sin, cos))
        } else {
            None
        };
        let st = u32::from(rd::<u8>(this, 0x36)) >> 3;
        let neutral = st != 0 && st != 8 && rd::<u32>(this, 0x38C) & 8 != 0;
        let h = rd::<GPtr<u8>>(this, 0x384);
        let bias_of = |off: isize| -> f32 {
            if neutral {
                1.0
            } else if front {
                c(F2).fmul_m(rd::<f32>(h, off)).to_f32()
            } else {
                c(F2).fmul_m(rd::<f32>(h, off)).fsubr_m(2.0).to_f32()
            }
        };
        let brake_bias = bias_of(0x98);
        let traction_bias = bias_of(0xA8);
        if !le(wc(left), X87::zero()) {
            wheel(
                this,
                left,
                trig,
                true,
                drive,
                accel,
                traction,
                brake,
                bias,
                brake_bias,
                traction_bias,
                speeds,
                points,
            );
        }
        if !le(wc(right), X87::zero()) {
            wheel(
                this,
                right,
                trig,
                false,
                drive,
                accel,
                traction,
                brake,
                bias,
                brake_bias,
                traction_bias,
                speeds,
                points,
            );
        }
    }

    if !front && rd::<u32>(this, 0x38C) & 0x8_0000 == 0 {
        if rd::<u16>(this, 0x86A) != 0
            && drive
            && (rd::<u32>(this, 0x96C) == 1 || rd::<u32>(this, 0x974) == 1)
        {
            let t = x(ts())
                .fmul_m(f32::from_bits(0x3A83_126F))
                .fadd_m(rd::<f32>(this, 0x8A4))
                .to_f32();
            wr::<f32>(this, 0x8A4, t);
            if !le(x(t), c(0x4040_0000)) {
                wr::<u32>(this, 0x8A4, 0x4040_0000);
            }
        } else if !le(ld(this, 0x8A4), c(F1)) {
            let d = ld(this, 0x8A4).fsub_m(1.0).to_f32();
            let p = X87::from_f64_exact(sa_core::orig::x87_binary(
                0x82_2130,
                c(0x3F7E_B852).to_f64(),
                x(ts()).to_f64(),
            ));
            wr::<f32>(this, 0x8A4, p.fmul_m(d).fadd_m(1.0).to_f32());
        }
    }
    if rd::<u32>(rd::<GPtr<u8>>(this, 0x384), 0xCC) & 0x200_0000 != 0 {
        return;
    }
    air_spin(this, left, drive, accel);
    air_spin(this, right, drive, accel);
}

pub const HOOKS: &[HookReg] = &[HOOK_AUTOMOBILE_PROCESS_CAR_WHEEL_PAIR];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
