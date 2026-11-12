//! WRITER-MVP-D (helper VEH, batch `mvpd-veh-1`): the per-wheel force functions of `CVehicle`.
//! Ported from the 1.0 US bytes; gta-reversed as a reading aid. Where they differ, the bytes win.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::common::{f32_of, g, rand_at, rd, x, xi};

orig! {
    /// `CVector::CrossProduct(CVector* out, const CVector* a, const CVector* b)` (0x59C730)
    fn cross_product(out: GPtr<CVector>, a: GPtr<CVector>, b: GPtr<CVector>) -> GPtr<CVector>
        = cdecl 0x59C730 => sa_world::collision::data::cross_product_port;
    /// `CVector::Normalise()` (0x59C910)
    fn normalise(v: GPtr<CVector>) = thiscall 0x59C910 => sa_world::collision::data::normalise_port;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0, the vector by value)
    fn apply_move_force(p: GPtr<u8>, force: CVector) = thiscall 0x5429F0 => sa_world::physical::forces::apply_move_force;
    /// `CPhysical::ApplyTurnForce(CVector force, CVector point)` (0x542A50)
    fn apply_turn_force(p: GPtr<u8>, force: CVector, point: CVector) = thiscall 0x542A50 => sa_world::physical::forces::apply_turn_force;
    /// `fabsf` thunk (0x406D10, cdecl, ST0)
    fn fabsf_call(v: f32) -> f64 = cdecl 0x406D10 => unported;
    /// `CPhysical::GetSpeed(CVector point)` (0x542CE0, hidden result pointer)
    fn get_speed(p: GPtr<u8>, out: GPtr<CVector>, point: CVector) -> GPtr<CVector> = thiscall 0x542CE0 => sa_world::physical::forces::get_speed;
    /// `CPhysical::ApplySpringCollision(float, CVector&, CVector&, float, float, float&)` (0x543C90)
    fn apply_spring_collision(p: GPtr<u8>, level: f32, dir: GPtr<u8>, point: GPtr<u8>, len: f32, bias: f32, out: GPtr<f32>) -> u8
        = thiscall 0x543C90 => sa_world::physical::spring::apply_spring_collision;
    /// `CPhysical::ApplySpringCollisionAlt(float, CVector&, CVector&, float, float, CVector&, float&)` (0x543D60)
    fn apply_spring_collision_alt(p: GPtr<u8>, level: f32, dir: GPtr<u8>, point: GPtr<u8>, len: f32, bias: f32, normal: GPtr<u8>, out: GPtr<f32>) -> u8
        = thiscall 0x543D60 => sa_world::physical::spring::apply_spring_collision_alt;
    /// `CPhysical::ApplySpringDampening(float, float, CVector&, CVector&, CVector&)` (0x543E90)
    fn apply_spring_dampening(p: GPtr<u8>, damping: f32, limit: f32, dir: GPtr<u8>, point: GPtr<u8>, speed: GPtr<u8>) -> u8
        = thiscall 0x543E90 => sa_world::physical::spring::apply_spring_dampening;
    /// `Multiply3x3(CVector* out, const CMatrix& m, const CVector& v)` (0x59C790)
    fn multiply3x3(out: GPtr<CVector>, m: GPtr<u8>, v: GPtr<CVector>) -> GPtr<CVector> = cdecl 0x59C790 => sa_world::physical::shared::multiply3x3_port;
    /// `CEntity::GetColModel()` (0x535300)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
}

/// `CTimer::ms_fTimeStep` (0xB7CB5C).
const TIME_STEP: usize = 0xB7_CB5C;
/// `gHandlingDataMgr.fWheelFriction` (0xC2B9CC).
const WHEEL_FRICTION: usize = 0xC2_B9CC;

fn gf(addr: usize) -> f32 {
    g::<f32>(addr).read()
}

fn gb(addr: usize) -> u8 {
    g::<u8>(addr).read()
}

fn sb(addr: usize, v: u8) {
    g::<u8>(addr).write(v)
}

fn vf(p: GPtr<u8>, i: isize) -> f32 {
    rd::<f32>(p, 4 * i)
}

/// `(m_nStatus == STATUS_PLAYER || == STATUS_REMOTE_CONTROLLED)`: `(+0x36 >> 3)` is 0 or 8.
fn player_or_remote(this: GPtr<u8>) -> bool {
    let s = rd::<u8>(this, 0x36) >> 3;
    s == 0 || s == 8
}

/// `1 - |m_GasPedal| * k` (`fld +0x49C; fabs; fmul k; fsubr 1.0`).
fn spin_loss(this: GPtr<u8>, k_addr: usize) -> X87 {
    x(rd::<f32>(this, 0x49C)).fabs().fmul_m(gf(k_addr)).fsubr_m(1.0)
}

/// `CPhysical::GetMass(point, dir)` inlined: `1 / (|point x dir|^2 / m_fTurnMass + 1 / m_fMass)`, the
/// cross product through `CrossProduct` 0x59C730 (squares summed x, y, z).
fn inline_get_mass(this: GPtr<u8>, point: GPtr<u8>, dir: &mut [f32; 3]) -> X87 {
    let mut c = CVector::default();
    cross_product(GPtr::from_raw(&mut c), point.cast(), GPtr::from_raw(dir.as_mut_ptr()).cast());
    let (cx, cy, cz) = (x(c.x), x(c.y), x(c.z));
    cx.fmul(cx)
        .fadd(cy.fmul(cy))
        .fadd(cz.fmul(cz))
        .fdiv_m(rd::<f32>(this, 0x90))
        .fadd(x(1.0).fdiv_m(rd::<f32>(this, 0x8C)))
        .fdivr_m(1.0)
}

/// The common tail of `ProcessWheel`/`ProcessBikeWheel`: `moveVec = fwd * wheelFwd + right *
/// wheelRight`, the anti-dive turn vector, both normalised (length 0 gives x = 1.0 with y/z kept), and
/// the two forces `ApplyMoveForce(moveDir * m_fMass * speed)`, `ApplyTurnForce(turnDir *
/// GetMass(point, turnDir) * turnSpeed, point)`. `anti_dive` is `Some(factor register)` when the
/// handling anti-dive multiplier applies.
fn apply_wheel_forces(this: GPtr<u8>, wf: GPtr<u8>, wrt: GPtr<u8>, point: GPtr<u8>, fwd: f32, right: X87, anti_dive: Option<X87>) {
    let f40 = f32_of(x(fwd).fmul_m(vf(wf, 0)));
    let f44 = f32_of(x(fwd).fmul_m(vf(wf, 1)));
    let f48 = f32_of(x(fwd).fmul_m(vf(wf, 2)));
    let r4c = f32_of(right.fmul_m(vf(wrt, 0)));
    let r50 = f32_of(right.fmul_m(vf(wrt, 1)));
    let rz = right.fmul_m(vf(wrt, 2));
    let mx = f32_of(x(r4c).fadd_m(f40));
    let my = f32_of(x(r50).fadd_m(f44));
    let mz = f32_of(rz.fadd_m(f48));
    let (mut tx, mut ty, mut tz) = (mx, my, mz);
    let sep = anti_dive.is_some();
    if let Some(a) = anti_dive {
        let ax = a.fmul_m(vf(wf, 0));
        let s50 = f32_of(a.fmul_m(vf(wf, 1)));
        let s54 = f32_of(a.fmul_m(vf(wf, 2)));
        let big_x = ax.fmul_m(fwd);
        let big_y = x(s50).fmul_m(fwd);
        let s54 = f32_of(x(s54).fmul_m(fwd));
        tx = f32_of(x(mx).fsub(big_x));
        ty = f32_of(x(my).fsub(big_y));
        tz = f32_of(x(mz).fsub_m(s54));
    }
    let speed = f32_of(x(mz).fmul_m(mz).fadd(x(my).fmul_m(my)).fadd(x(mx).fmul_m(mx)).fsqrt());
    let turn_speed = if sep { f32_of(x(tz).fmul_m(tz).fadd(x(ty).fmul_m(ty)).fadd(x(tx).fmul_m(tx)).fsqrt()) } else { speed };
    // `test ah, 0x41; jp` taken: speed > 0 or unordered
    let md = if x(speed).fcom(x(0.0)).test_parity(0x41) {
        let inv = x(1.0).fdiv_m(speed);
        [f32_of(x(mx).fmul(inv)), f32_of(x(my).fmul(inv)), f32_of(x(mz).fmul(inv))]
    } else {
        [1.0, my, mz]
    };
    let mut td = if !sep {
        md
    } else if x(turn_speed).fcom(x(0.0)).test_parity(0x41) {
        let inv = x(1.0).fdiv_m(turn_speed);
        [f32_of(x(tx).fmul(inv)), f32_of(x(ty).fmul(inv)), f32_of(x(tz).fmul(inv))]
    } else {
        [1.0, ty, tz]
    };
    let mass = rd::<f32>(this, 0x8C);
    let ms = x(mass).fmul_m(speed);
    let tms = inline_get_mass(this, point, &mut td).fmul_m(turn_speed);
    let move_force = CVector::new(f32_of(ms.fmul_m(md[0])), f32_of(x(md[1]).fmul(ms)), f32_of(x(md[2]).fmul(ms)));
    let turn_force = CVector::new(f32_of(x(td[0]).fmul(tms)), f32_of(x(td[1]).fmul(tms)), f32_of(x(td[2]).fmul(tms)));
    apply_move_force(this, move_force);
    let p = CVector::new(vf(point, 0), vf(point, 1), vf(point, 2));
    apply_turn_force(this, turn_force, p);
}

/// `CVehicle::ProcessWheel(CVector& wheelFwd, CVector& wheelRight, CVector& contactSpeed, CVector&
/// contactPoint, int32 wheelsOnGround, float thrust, float brake, float adhesion, int8 wheelId, float&
/// wheelSpeed, tWheelState& state, uint16 wheelStatus)` (0x6D6C00, `ret 0x30`).
///
/// The flags are function statics: `bBraking` 0xC1CDAE = brake != 0, `bDriving` 0xC1CDAD = !braking
/// && thrust != 0, `bAlreadySkidding` 0xC1CDAC is set (never cleared) when the incoming state is not
/// NORMAL. adhesion *= `CTimer::ms_fTimeStep`; a non-NORMAL state multiplies it by the handling
/// traction loss (+0xA4), SPINNING additionally by `1 - |gas| * 0.2` (0x8D34C4) for player/remote
/// status. The state becomes NORMAL. `right = -(contactSpeed . wheelRight) / wheelsOnGround` when that
/// dot is not 0, plus `min(contactSpeedFwd, 0.3 @0x8D34B8) * rand(-0.13, 0.13 @0x8D34B4)` for a burst
/// tyre (status 1; one `rand`). Driving: fwd = thrust, right clamped to ±adhesion. Else, when
/// contactSpeedFwd != 0: fwd = -contactSpeedFwd / wheelsOnGround; not braking and |gas| < 0.01 replaces
/// brake by the wheel friction: bike `fWheelFriction * 0.6 / (mass + 200)`, plane subclass (+0x594 ==
/// 4) 0, else `fWheelFriction / mass` times 0.1 below mass 500 or 0.2 for the RC bandit (0x1B9); brake
/// > adhesion (or NaN) gives FIXED when |contactSpeedFwd| > 0.005, else fwd is clamped to ±brake. When
/// `right^2 + fwd^2 > adhesion^2`: a non-FIXED state becomes SPINNING when driving and `limit *
/// adhesion < |fwd|` (limit 0.3 @0x8D34C0, doubled for wheels 0/2 when contactSpeedFwd > 0.15), else
/// SKIDDING; both are scaled by `trLoss / sqrt(slip) * adhesion`, where (as compiled, unlike
/// gta-reversed) trLoss is 1.0 when bAlreadySkidding is set and the handling traction loss otherwise,
/// times `1 - |gas| * 0.2` for SPINNING with player/remote status. The forces follow when fwd or right
/// is not 0 (anti-dive +0xC4 > 0 while braking (factor 1) or driving (0.5) separates the turn vector).
#[hook(addr = 0x6D6C00, conv = thiscall, name = "CVehicle::ProcessWheel", this = "CVehicle", expect = "83 EC 48 D9 05 50 8B 85 00",
       unverified = "mvpd-veh-1", route = "L2", hazards = "stack-residue,x87-transcendental",
       recs = "driving",
       note = "the original keeps temporaries in its own argument slots (contactSpeedFwd in wheelFwd, contactSpeedRight/min in contactSpeed, adhesion*ts then the mass in adhesion, speed in wheelId, rand in state, slip/turnSpeed in wheelStatus); fsqrt of the slip and the speeds")]
#[allow(clippy::too_many_arguments)]
pub fn vehicle_process_wheel(
    this: GPtr<u8>,
    wheel_fwd: GPtr<u8>,
    wheel_right: GPtr<u8>,
    contact_speed: GPtr<u8>,
    contact_point: GPtr<u8>,
    wheels_on_ground: i32,
    thrust: f32,
    brake: f32,
    adhesion: f32,
    wheel_id: u32,
    _wheel_speed: GPtr<u8>,
    state: GPtr<i32>,
    wheel_status: u32,
) {
    let wf = wheel_fwd;
    let wrt = wheel_right;
    let cs = contact_speed;
    let csf = f32_of(x(vf(wf, 1)).fmul_m(vf(cs, 1)).fadd(x(vf(wf, 0)).fmul_m(vf(cs, 0))).fadd(x(vf(cs, 2)).fmul_m(vf(wf, 2))));
    // `fcom brake, 0.0; test ah, 0x44; jnp` taken: brake == 0
    if x(brake).fcom(x(0.0)).test_parity(0x44) {
        sb(0xC1_CDAE, 1);
        sb(0xC1_CDAD, 0);
    } else {
        sb(0xC1_CDAE, 0);
        sb(0xC1_CDAD, 1);
        if !x(thrust).fcom(x(0.0)).test_parity(0x44) {
            sb(0xC1_CDAD, 0);
        }
    }
    let mut adh = f32_of(x(gf(TIME_STEP)).fmul_m(adhesion));
    let handling = rd::<GPtr<u8>>(this, 0x384);
    let st = state.read();
    if st != 0 {
        adh = f32_of(x(adh).fmul_m(rd::<f32>(handling, 0xA4)));
        sb(0xC1_CDAC, 1);
        if st == 1 && player_or_remote(this) {
            adh = f32_of(spin_loss(this, 0x8D_34C4).fmul_m(adh));
        }
    }
    state.write(0);
    let csr = x(vf(wrt, 1)).fmul_m(vf(cs, 1)).fadd(x(vf(wrt, 2)).fmul_m(vf(cs, 2))).fadd(x(vf(wrt, 0)).fmul_m(vf(cs, 0)));
    let csr_m = f32_of(csr);
    // the 0.0 pushed at entry is `right` until the dot is not 0
    let mut right = x(0.0);
    if csr.fcom(x(0.0)).test_parity(0x44) {
        let r = xi(wheels_on_ground).fdivr_m(csr_m).fchs();
        right = r;
        if wheel_status as u16 == 1 {
            let right_m = f32_of(r);
            let max = if x(csf).fcom(x(gf(0x8D_34B8))).test_parity(0x05) { gf(0x8D_34B8) } else { csf };
            let m = gf(0x8D_34B4);
            let neg = f32_of(x(m).fmul_m(-1.0));
            let rnd = rand_at(0x6D_6D7B);
            right = xi(rnd).fmul_m(f32::from_bits(0x3800_0100)).fmul(x(m).fsub_m(neg)).fadd_m(neg).fmul_m(max).fadd_m(right_m);
        }
    }
    let driving = gb(0xC1_CDAD);
    let mut fwd = 0.0f32;
    if driving != 0 {
        fwd = thrust;
        if !right.fcom(x(0.0)).test_any(0x41) {
            if !right.fcom(x(adh)).test_any(0x41) {
                right = x(adh);
            }
        } else {
            let n = f32_of(x(adh).fchs());
            if !right.fcom(x(n)).test_parity(0x05) {
                right = x(n);
            }
        }
    } else if x(csf).fcom(x(0.0)).test_parity(0x44) {
        let mut brk = x(brake);
        fwd = f32_of(xi(wheels_on_ground).fdivr_m(csf).fchs());
        if gb(0xC1_CDAE) == 0 && !x(rd::<f32>(this, 0x49C)).fabs().fcom(x(0.01)).test_parity(0x05) {
            let mass = rd::<f32>(handling, 4);
            brk = if rd::<u32>(this, 0x590) == 9 {
                x(gf(WHEEL_FRICTION)).fmul_m(0.6).fdiv(x(mass).fadd_m(200.0))
            } else if rd::<u32>(this, 0x594) == 4 {
                x(0.0)
            } else {
                let light = !x(mass).fcom(x(500.0)).test_parity(0x05);
                let q = x(gf(WHEEL_FRICTION)).fdiv_m(mass);
                if light {
                    q.fmul_m(f32::from_bits(0x3DCC_CCCD))
                } else if rd::<u16>(this, 0x22) == 0x1B9 {
                    q.fmul_m(f32::from_bits(0x3E4C_CCCD))
                } else {
                    q
                }
            };
        }
        if brk.fcom(x(adh)).test_parity(0x41) {
            if !x(csf).fabs().fcom(x(f32::from_bits(0x3BA3_D70A))).test_any(0x41) {
                state.write(3);
            }
        } else if !x(fwd).fcom(x(0.0)).test_any(0x41) {
            if !x(fwd).fcom(brk).test_any(0x41) {
                fwd = f32_of(brk);
            }
        } else {
            let nb = brk.fchs();
            if !x(fwd).fcom(nb).test_parity(0x05) {
                fwd = f32_of(nb);
            }
        }
    }
    let slip = right.fmul(right).fadd(x(fwd).fmul_m(fwd));
    let slip_m = f32_of(slip);
    if !slip.fcom(x(adh).fmul_m(adh)).test_any(0x41) {
        if state.read() != 3 {
            let mut lim = x(gf(0x8D_34C0));
            if !x(csf).fcom(x(f32::from_bits(0x3E19_999A))).test_any(0x41) {
                let id = wheel_id as u8;
                if id == 0 || id == 2 {
                    lim = lim.fadd(lim);
                }
            }
            let spinning = driving != 0 && !lim.fmul_m(adh).fcom(x(fwd).fabs()).test_parity(0x05);
            state.write(if spinning { 1 } else { 2 });
        }
        let mut tl = x(rd::<f32>(handling, 0xA4));
        if gb(0xC1_CDAC) != 0 {
            tl = x(1.0);
        } else if state.read() == 1 && player_or_remote(this) {
            tl = tl.fmul(spin_loss(this, 0x8D_34C4));
        }
        let t = tl.fdiv(x(slip_m).fsqrt()).fmul_m(adh);
        fwd = f32_of(x(fwd).fmul(t));
        right = right.fmul(t);
    }
    if !x(fwd).fcom(x(0.0)).test_parity(0x44) && !right.fcom(x(0.0)).test_parity(0x44) {
        return;
    }
    let ad = rd::<f32>(handling, 0xC4);
    let anti_dive = if !x(ad).fcom(x(0.0)).test_any(0x41) {
        if gb(0xC1_CDAE) != 0 {
            Some(x(ad))
        } else if gb(0xC1_CDAD) != 0 {
            Some(x(ad).fmul_m(0.5))
        } else {
            None
        }
    } else {
        None
    };
    apply_wheel_forces(this, wf, wrt, contact_point, fwd, right, anti_dive);
}

/// The driving clamp of `right` to ±adhesion (`fcom 0`, then `fcom adhesion` / `fcom -adhesion`, the
/// negated adhesion stored to f32 first).
fn clamp_right(right: X87, adh: f32) -> X87 {
    if !right.fcom(x(0.0)).test_any(0x41) {
        if !right.fcom(x(adh)).test_any(0x41) {
            return x(adh);
        }
    } else {
        let n = f32_of(x(adh).fchs());
        if !right.fcom(x(n)).test_parity(0x05) {
            return x(n);
        }
    }
    right
}

/// The brake tail: brake > adhesion (or NaN) gives FIXED (3) when |contactSpeedFwd| > 0.005, else the
/// stored fwd is clamped to ±brake (register compares).
fn brake_clamp(brk: X87, adh: f32, csf: f32, fwd: &mut f32, state: GPtr<i32>) {
    if brk.fcom(x(adh)).test_parity(0x41) {
        if !x(csf).fabs().fcom(x(f32::from_bits(0x3BA3_D70A))).test_any(0x41) {
            state.write(3);
        }
    } else if !x(*fwd).fcom(x(0.0)).test_any(0x41) {
        if !x(*fwd).fcom(brk).test_any(0x41) {
            *fwd = f32_of(brk);
        }
    } else {
        let nb = brk.fchs();
        if !x(*fwd).fcom(nb).test_parity(0x05) {
            *fwd = f32_of(nb);
        }
    }
}

/// `CVehicle::ProcessBikeWheel(CVector& wheelFwd, CVector& wheelRight, CVector& contactSpeed,
/// CVector& contactPoint, int32 wheelsOnGround, float thrust, float brake, float adhesion, float
/// destabTraction, int8 wheelId, float& wheelSpeed, tWheelState& state, eBikeWheelSpecial special,
/// uint16 wheelStatus)` (0x6D73B0, `ret 0x38`).
///
/// Statics: `bBraking` 0xC1CDB2 = brake != 0, `bDriving` 0xC1CDB1 = !braking && thrust != 0,
/// `bReversing` 0xC1CDB0 = !braking && thrust < 0, `bAlreadySkidding` 0xC1CDAF set (never cleared)
/// for a non-NORMAL incoming state. adhesion *= the time step, and by the handling traction loss
/// when bAlreadySkidding. Special 2/3 skip the lateral term; otherwise `right = -(contactSpeed .
/// wheelRight) / wheelsOnGround` when the dot is not 0, plus `min(contactSpeedFwd, 0.12 @0x8D34CC) *
/// rand(-0.05, 0.05 @0x8D34C8)` for a burst tyre. Driving: fwd = thrust, right clamped to ±adhesion.
/// Else, when contactSpeedFwd != 0: fwd = -contactSpeedFwd / n; not braking with |gas| < 0.01 uses
/// the wheel friction as brake: BMX (+0x594 == 10) only when -0.05 < fwd < 0.05, `fWheelFriction *
/// 0.5 / (mass + 200)`; bike (+0x590 == 9) `* 0.6 / (mass + 200)`; else `fWheelFriction / mass`, times
/// 0.2 below mass 500 and for the RC bandit (as compiled). Then the ProcessWheel brake clamp. When
/// `right^2 + fwd^2 > (adhesion^2 as f32)`: a non-FIXED state becomes SPINNING when driving and
/// contactSpeedFwd < 0.1, else SKIDDING; both are scaled by `trLoss / sqrt(slip) * adhesion`
/// (trLoss 1.0 when bAlreadySkidding, else the traction loss), right also by destabTraction when it
/// is < 1. Otherwise with destabTraction < 1 (times the traction loss unless bAlreadySkidding) and
/// `adhesion^2 * d * d < slip`, right *= `adhesion * d / sqrt(slip)`. Forces when fwd or right is not
/// 0: the move vector is normalised by `CVector::Normalise`, `ApplyMoveForce(dir * mass * speed)`;
/// unless wheel 1 is braking or reversing, `ApplyTurnForce(2 * (dir * GetMass * speed - rc *
/// matRight), cp - (cp . matRight) * matRight)` with `rc = turnForce . matRight`; then always
/// `ApplyTurnForce(rc * matRight, (cp . matFwd) * matFwd)`.
#[hook(addr = 0x6D73B0, conv = thiscall, name = "CVehicle::ProcessBikeWheel", this = "CVehicle", expect = "83 EC 40 D9 05 50 8B 85 00",
       unverified = "mvpd-veh-1", route = "L2", hazards = "stack-residue,x87-transcendental",
       recs = "driving",
       note = "the original keeps temporaries in its own argument slots (contactSpeedFwd in wheelFwd, right/min in wheelRight/contactSpeed, adhesion^2 in brake, the mass then cp.matRight in adhesion, destab*tl then cp.matFwd in destabTraction, rand in state, the burst mod in special, speed/slip/rc in wheelStatus)")]
#[allow(clippy::too_many_arguments)]
pub fn vehicle_process_bike_wheel(
    this: GPtr<u8>,
    wheel_fwd: GPtr<u8>,
    wheel_right: GPtr<u8>,
    contact_speed: GPtr<u8>,
    contact_point: GPtr<u8>,
    wheels_on_ground: i32,
    thrust: f32,
    brake: f32,
    adhesion: f32,
    destab_traction: f32,
    wheel_id: u32,
    _wheel_speed: GPtr<u8>,
    state: GPtr<i32>,
    special: u32,
    wheel_status: u32,
) {
    let wf = wheel_fwd;
    let wrt = wheel_right;
    let cs = contact_speed;
    let csf = f32_of(x(vf(wf, 1)).fmul_m(vf(cs, 1)).fadd(x(vf(cs, 2)).fmul_m(vf(wf, 2))).fadd(x(vf(wf, 0)).fmul_m(vf(cs, 0))));
    if x(brake).fcom(x(0.0)).test_parity(0x44) {
        sb(0xC1_CDB2, 1);
        sb(0xC1_CDB1, 0);
        sb(0xC1_CDB0, 0);
    } else {
        sb(0xC1_CDB2, 0);
        sb(0xC1_CDB1, 1);
        if !x(thrust).fcom(x(0.0)).test_parity(0x44) {
            sb(0xC1_CDB1, 0);
        }
        sb(0xC1_CDB0, 1);
        if x(thrust).fcom(x(0.0)).test_parity(0x05) {
            sb(0xC1_CDB0, 0);
        }
    }
    let skid = if state.read() != 0 {
        sb(0xC1_CDAF, 1);
        1
    } else {
        gb(0xC1_CDAF)
    };
    state.write(0);
    let handling = rd::<GPtr<u8>>(this, 0x384);
    let mut adh = f32_of(x(gf(TIME_STEP)).fmul_m(adhesion));
    if skid != 0 {
        adh = f32_of(x(adh).fmul_m(rd::<f32>(handling, 0xA4)));
    }
    let mut right = x(0.0);
    if special != 2 && special != 3 {
        let csr = x(vf(wrt, 2)).fmul_m(vf(cs, 2)).fadd(x(vf(wrt, 1)).fmul_m(vf(cs, 1))).fadd(x(vf(wrt, 0)).fmul_m(vf(cs, 0)));
        let csr_m = f32_of(csr);
        if csr.fcom(x(0.0)).test_parity(0x44) {
            let r = xi(wheels_on_ground).fdivr_m(csr_m).fchs();
            right = r;
            if wheel_status as u16 == 1 {
                let right_m = f32_of(r);
                let max = if x(csf).fcom(x(gf(0x8D_34CC))).test_parity(0x05) { gf(0x8D_34CC) } else { csf };
                let m = gf(0x8D_34C8);
                let neg = f32_of(x(m).fmul_m(-1.0));
                let rnd = rand_at(0x6D_7548);
                right = xi(rnd).fmul_m(f32::from_bits(0x3800_0100)).fmul(x(m).fsub_m(neg)).fadd_m(neg).fmul_m(max).fadd_m(right_m);
            }
        }
    }
    let driving = gb(0xC1_CDB1);
    let mut fwd = 0.0f32;
    if driving != 0 {
        fwd = thrust;
        right = clamp_right(right, adh);
    } else if x(csf).fcom(x(0.0)).test_parity(0x44) {
        let mut brk = x(brake);
        fwd = f32_of(xi(wheels_on_ground).fdivr_m(csf).fchs());
        if gb(0xC1_CDB2) == 0 {
            let gas = x(rd::<f32>(this, 0x49C));
            let gas = if !gas.fcom(x(0.0)).test_parity(0x05) { gas.fchs() } else { gas };
            if !gas.fcom(x(0.01)).test_parity(0x05) {
                if rd::<u32>(this, 0x594) == 10 {
                    if !x(fwd).fcom(x(f32::from_bits(0xBD4C_CCCD))).test_any(0x41)
                        && !x(fwd).fcom(x(f32::from_bits(0x3D4C_CCCD))).test_parity(0x05)
                    {
                        brk = x(gf(WHEEL_FRICTION)).fmul_m(0.5).fdiv(x(rd::<f32>(handling, 4)).fadd_m(200.0));
                    }
                } else if rd::<u32>(this, 0x590) == 9 {
                    brk = x(gf(WHEEL_FRICTION)).fmul_m(0.6).fdiv(x(rd::<f32>(handling, 4)).fadd_m(200.0));
                } else {
                    let mass = rd::<f32>(handling, 4);
                    let light = !x(mass).fcom(x(500.0)).test_parity(0x05);
                    let q = x(gf(WHEEL_FRICTION)).fdiv_m(mass);
                    brk = if light || rd::<u16>(this, 0x22) == 0x1B9 { q.fmul_m(f32::from_bits(0x3E4C_CCCD)) } else { q };
                }
            }
        }
        brake_clamp(brk, adh, csf, &mut fwd, state);
    }
    let adh2 = f32_of(x(adh).fmul_m(adh));
    let slip = right.fmul(right).fadd(x(fwd).fmul_m(fwd));
    let slip_m = f32_of(slip);
    if !slip.fcom(x(adh2)).test_any(0x41) {
        if state.read() != 3 {
            let spinning = gb(0xC1_CDB1) != 0 && !x(csf).fcom(x(f32::from_bits(0x3DCC_CCCD))).test_parity(0x05);
            state.write(if spinning { 1 } else { 2 });
        }
        let tl = if skid != 0 { x(1.0) } else { x(rd::<f32>(handling, 0xA4)) };
        let t = tl.fdiv(x(slip_m).fsqrt()).fmul_m(adh);
        fwd = f32_of(x(fwd).fmul(t));
        right = right.fmul(t);
        if !x(destab_traction).fcom(x(1.0)).test_parity(0x05) {
            right = right.fmul_m(destab_traction);
        }
    } else if !x(destab_traction).fcom(x(1.0)).test_parity(0x05) {
        let mut d = destab_traction;
        if skid == 0 {
            d = f32_of(x(d).fmul_m(rd::<f32>(handling, 0xA4)));
        }
        if !x(adh2).fmul_m(d).fmul_m(d).fcom(x(slip_m)).test_parity(0x05) {
            let t = x(adh).fmul_m(d);
            right = right.fmul(t.fdiv(x(slip_m).fsqrt()));
        }
    }
    if !x(fwd).fcom(x(0.0)).test_parity(0x44) && !right.fcom(x(0.0)).test_parity(0x44) {
        return;
    }
    let f20 = f32_of(x(fwd).fmul_m(vf(wf, 0)));
    let f24 = f32_of(x(fwd).fmul_m(vf(wf, 1)));
    let f28 = f32_of(x(fwd).fmul_m(vf(wf, 2)));
    let r2c = f32_of(right.fmul_m(vf(wrt, 0)));
    let r30 = f32_of(right.fmul_m(vf(wrt, 1)));
    let rz = right.fmul_m(vf(wrt, 2));
    let mx = f32_of(x(r2c).fadd_m(f20));
    let my = f32_of(x(r30).fadd_m(f24));
    let mz = f32_of(rz.fadd_m(f28));
    let mut dir = CVector::new(mx, my, mz);
    normalise(GPtr::from_raw(&mut dir));
    let speed = f32_of(x(mz).fmul_m(mz).fadd(x(my).fmul_m(my)).fadd(x(mx).fmul_m(mx)).fsqrt());
    let mass = rd::<f32>(this, 0x8C);
    let mut d3 = [dir.x, dir.y, dir.z];
    let tms = inline_get_mass(this, contact_point, &mut d3).fmul_m(speed);
    let ms = x(mass).fmul_m(speed);
    let move_force = CVector::new(f32_of(x(dir.x).fmul(ms)), f32_of(x(dir.y).fmul(ms)), f32_of(x(dir.z).fmul(ms)));
    let tf = [f32_of(x(dir.x).fmul(tms)), f32_of(x(dir.y).fmul(tms)), f32_of(x(dir.z).fmul(tms))];
    apply_move_force(this, move_force);
    let mat = rd::<GPtr<u8>>(this, 0x14);
    let cp = contact_point;
    let rc = f32_of(x(tf[2]).fmul_m(vf(mat, 2)).fadd(x(tf[1]).fmul_m(vf(mat, 1))).fadd(x(tf[0]).fmul_m(vf(mat, 0))));
    let cp_r = f32_of(x(vf(mat, 1)).fmul_m(vf(cp, 1)).fadd(x(vf(mat, 2)).fmul_m(vf(cp, 2))).fadd(x(vf(cp, 0)).fmul_m(vf(mat, 0))));
    let cp_f = f32_of(x(vf(mat, 5)).fmul_m(vf(cp, 1)).fadd(x(vf(mat, 6)).fmul_m(vf(cp, 2))).fadd(x(vf(cp, 0)).fmul_m(vf(mat, 4))));
    if wheel_id as u8 != 1 || (gb(0xC1_CDB2) == 0 && gb(0xC1_CDB0) == 0) {
        let a = x(cp_r).fmul_m(vf(mat, 0));
        let b = x(cp_r).fmul_m(vf(mat, 1));
        let c = f32_of(x(cp_r).fmul_m(vf(mat, 2)));
        let point = CVector::new(f32_of(x(vf(cp, 0)).fsub(a)), f32_of(x(vf(cp, 1)).fsub(b)), f32_of(x(vf(cp, 2)).fsub_m(c)));
        let p = x(rc).fmul_m(vf(mat, 0));
        let q = x(rc).fmul_m(vf(mat, 1));
        let r = f32_of(x(rc).fmul_m(vf(mat, 2)));
        let fx = f32_of(x(tf[0]).fsub(p));
        let k = gf(0x8D_34D0);
        let force = CVector::new(
            f32_of(x(fx).fmul_m(k)),
            f32_of(x(tf[1]).fsub(q).fmul_m(k)),
            f32_of(x(tf[2]).fsub_m(r).fmul_m(k)),
        );
        apply_turn_force(this, force, point);
    }
    let point = CVector::new(
        f32_of(x(cp_f).fmul_m(vf(mat, 4))),
        f32_of(x(cp_f).fmul_m(vf(mat, 5))),
        f32_of(x(cp_f).fmul_m(vf(mat, 6))),
    );
    let force = CVector::new(
        f32_of(x(rc).fmul_m(vf(mat, 0))),
        f32_of(x(rc).fmul_m(vf(mat, 1))),
        f32_of(x(rc).fmul_m(vf(mat, 2))),
    );
    apply_turn_force(this, force, point);
}

fn vptr(v: &mut CVector) -> GPtr<u8> {
    GPtr::from_raw(v).cast()
}

fn read_vec(p: GPtr<u8>) -> CVector {
    CVector::new(vf(p, 0), vf(p, 1), vf(p, 2))
}

/// `a < b` (`test ah, 5; jp` not taken).
fn lt(a: X87, b: X87) -> bool {
    !a.fcom(b).test_parity(0x05)
}

/// `CAutomobile::ProcessSuspension()` (0x6AFB10).
///
/// `springLength[4]` starts as `m_fWheelsSuspensionCompression` (+0x7D4), the four damping limits as
/// 0, `fwdSpeed = |moveSpeed . GetForward()|` (z, y, x). The Rhino (model 0x1B0) runs three passes
/// unless `CReplay::Mode` (0xA43088) is 1 (playback). Each pass: `dir[i] = up * -1`, `cp[i] =
/// wheelColPoint[i].point (+0x724 + 0x2C*i) - position` (the matrix position, `this+4` without a
/// matrix) for springLength < 1, else 0. Per compressed wheel: bias = handling +0xC0 (1 - bias for
/// the rear wheels 1 and 3; 0.5 for hydraulic geometry + hydraulic install + NPC neutral handling in
/// STATUS_PHYSICS); force level = handling +0xAC, times 1.5 (hydraulic geometry) or 1.0 when the
/// hydraulic install flag is set and `fabsf(fwdSpeed) < 0.15` (the vortex term is then skipped), else
/// for the Vortex (0x21B) times `|+0x994| * 0.25 + 1`; `ApplySpringCollision` when the cars-on-water
/// cheat (0x969152) is on or for the Vortex, else `ApplySpringCollisionAlt` with the col point
/// normal. Then the contact speeds (`GetSpeed(cp)` minus the collision entity's `GetSpeed` at
/// +0x8F8), with `dir = -normal` where the real compression is < 1 and normal.z > 0.35; then per
/// wheel the damping (handling +0xB0, 0.1 when hydraulic, damping > 0.1 and `fabsf(fwdSpeed) <
/// 0.15`) and `ApplySpringDampening` when springLength < 0.99999 (not 1.0, as compiled) and not a
/// monster truck (+0x868 bit 7). The Rhino's extra passes take the spring lengths from the door
/// structs (+0x618.. / +0x61C..) and the col lines 4/7/8/11 or 5/6/9/10, set cp to the transformed
/// line start lowered by `lineLength * springLength`, and rescale `springLength = (s - t) / (1 - t)`
/// with `t = 1 - springLength/lineLength` (+0x878 / +0x888); the next pass recomputes cp from the
/// col points as above. Finally, below a forward speed of 0.02 (0.04 and a doubled roll force
/// 0x8D3170 when not player/remote), a car with both left wheels fully extended (bit compare with
/// 1.0) and a right one compressed rolls with dir 1 (or -1 mirrored) when `|(fwd x Z) . right| < 0.6`:
/// `ApplyTurnForce(up * (roll * turnMass * dir), right * bboxMax.x)` and `ApplyMoveForce((Z x fwd) *
/// (roll * mass * dir))`.
#[hook(addr = 0x6AFB10, conv = thiscall, name = "CAutomobile::ProcessSuspension", this = "CAutomobile", expect = "81 EC 40 01 00 00",
       route = "L2", hazards = "none",
       recs = "driving")]
pub fn automobile_process_suspension(this: GPtr<u8>) {
    let mut spring = [0f32; 4];
    for (i, s) in spring.iter_mut().enumerate() {
        *s = rd::<f32>(this, 0x7D4 + 4 * i as isize);
    }
    let mut damp = [0f32; 4];
    let m0 = rd::<GPtr<u8>>(this, 0x14);
    let fwd_speed = f32_of(
        x(rd::<f32>(this, 0x4C))
            .fmul_m(vf(m0, 6))
            .fadd(x(rd::<f32>(this, 0x48)).fmul_m(vf(m0, 5)))
            .fadd(x(rd::<f32>(this, 0x44)).fmul_m(vf(m0, 4)))
            .fabs(),
    );
    let model = rd::<u16>(this, 0x22);
    let mut loops: i32 = if model == 0x1B0 && gb(0xA4_3088) != 1 { 3 } else { 1 };
    let mut cp = [CVector::default(); 4];
    let mut dir = [CVector::default(); 4];
    let mut cs = [CVector::default(); 4];
    let mut idx: [isize; 4];
    loop {
        let mat = rd::<GPtr<u8>>(this, 0x14);
        for i in 0..4 {
            dir[i] = CVector::new(f32_of(x(vf(mat, 8)).fmul_m(-1.0)), f32_of(x(vf(mat, 9)).fmul_m(-1.0)), f32_of(x(vf(mat, 10)).fmul_m(-1.0)));
            if lt(x(spring[i]), x(1.0)) {
                let pos = if mat.is_null() { this.byte_add(4) } else { mat.byte_add(0x30) };
                let p = this.byte_add(0x724 + 0x2C * i as isize);
                cp[i] = CVector::new(
                    f32_of(x(vf(p, 0)).fsub_m(vf(pos, 0))),
                    f32_of(x(vf(p, 1)).fsub_m(vf(pos, 1))),
                    f32_of(x(vf(p, 2)).fsub_m(vf(pos, 2))),
                );
            } else {
                cp[i] = CVector::default();
            }
        }
        for i in 0..4 {
            if !lt(x(spring[i]), x(1.0)) {
                continue;
            }
            let h = rd::<GPtr<u8>>(this, 0x384);
            let mut bias = rd::<f32>(h, 0xC0);
            if i == 1 || i == 3 {
                bias = f32_of(x(1.0).fsub_m(bias));
            }
            let fl = rd::<u32>(this, 0x38C);
            let geom = fl & 0x1_0000 != 0;
            if geom && fl & 0x2_0000 != 0 && fl & 8 != 0 && rd::<u8>(this, 0x36) & 0xF8 == 0x18 {
                bias = 0.5;
            }
            let mut level = rd::<f32>(h, 0xAC);
            if fl & 0x2_0000 != 0 && lt(X87::from_f64_exact(fabsf_call(fwd_speed)), x(f32::from_bits(0x3E19_999A))) {
                let k = if geom { 1.5 } else { 1.0 };
                level = f32_of(x(k).fmul_m(level));
            } else if model == 0x21B {
                level = f32_of(x(rd::<f32>(this, 0x994)).fabs().fmul_m(0.25).fadd_m(1.0).fmul_m(level));
            }
            let dp = vptr(&mut dir[i]);
            let cpp = vptr(&mut cp[i]);
            let lim = GPtr::from_raw(&mut damp[i]);
            if gb(0x96_9152) != 0 || model == 0x21B {
                apply_spring_collision(this, level, dp, cpp, spring[i], bias, lim);
            } else {
                let normal = this.byte_add(0x734 + 0x2C * i as isize);
                apply_spring_collision_alt(this, level, dp, cpp, spring[i], bias, normal, lim);
            }
        }
        for i in 0..4 {
            let mut out = CVector::default();
            cs[i] = read_vec(get_speed(this, GPtr::from_raw(&mut out), cp[i]).cast());
            let e = rd::<GPtr<u8>>(this, 0x8E8 + 4 * i as isize);
            if !e.is_null() {
                let mut out2 = CVector::default();
                let r = read_vec(get_speed(e, GPtr::from_raw(&mut out2), read_vec(this.byte_add(0x8F8 + 12 * i as isize))).cast());
                cs[i].x = f32_of(x(cs[i].x).fsub_m(r.x));
                cs[i].y = f32_of(x(cs[i].y).fsub_m(r.y));
                cs[i].z = f32_of(x(cs[i].z).fsub_m(r.z));
            }
            if lt(x(rd::<f32>(this, 0x7D4 + 4 * i as isize)), x(1.0)) {
                let n = this.byte_add(0x734 + 0x2C * i as isize);
                if !x(vf(n, 2)).fcom(x(f32::from_bits(0x3EB3_3333))).test_any(0x41) {
                    dir[i] = CVector::new(f32_of(x(vf(n, 0)).fchs()), f32_of(x(vf(n, 1)).fchs()), f32_of(x(vf(n, 2)).fchs()));
                }
            }
        }
        for i in 0..4 {
            let h = rd::<GPtr<u8>>(this, 0x384);
            let mut damping = rd::<f32>(h, 0xB0);
            if rd::<u32>(this, 0x38C) & 0x2_0000 != 0
                && lt(x(f32::from_bits(0x3DCC_CCCD)), x(damping))
                && lt(X87::from_f64_exact(fabsf_call(fwd_speed)), x(f32::from_bits(0x3E19_999A)))
            {
                damping = f32::from_bits(0x3DCC_CCCD);
            }
            if lt(x(spring[i]), x(f32::from_bits(0x3F7F_FF58))) && rd::<i8>(this, 0x868) >= 0 {
                apply_spring_dampening(this, damping, damp[i], vptr(&mut dir[i]), vptr(&mut cp[i]), vptr(&mut cs[i]));
            }
        }
        loops -= 1;
        if model == 0x1B0 {
            if loops <= 0 {
                break;
            }
            let cd = rd::<GPtr<u8>>(get_col_model(this), 0x2C);
            if loops == 2 {
                for (s, off) in spring.iter_mut().zip([0x618, 0x628, 0x630, 0x640]) {
                    *s = rd::<f32>(this, off);
                }
                idx = [4, 7, 8, 11];
            } else if loops == 1 {
                for (s, off) in spring.iter_mut().zip([0x61C, 0x624, 0x634, 0x63C]) {
                    *s = rd::<f32>(this, off);
                }
                idx = [5, 6, 9, 10];
            } else {
                continue;
            }
            for i in 0..4 {
                let line = rd::<GPtr<u8>>(cd, 0x10).byte_add(idx[i] * 32);
                let len = rd::<f32>(this, 0x888 + 4 * i as isize);
                let z = f32_of(x(len).fmul_m(spring[i]).fchs().fadd_m(vf(line, 2)));
                let mut v = CVector::new(vf(line, 0), f32_of(x(vf(line, 1))), z);
                let mut out = CVector::default();
                cp[i] = read_vec(multiply3x3(GPtr::from_raw(&mut out), rd::<GPtr<u8>>(this, 0x14), GPtr::from_raw(&mut v)).cast());
                let t = x(rd::<f32>(this, 0x878 + 4 * i as isize)).fdiv_m(len).fsubr_m(1.0);
                spring[i] = f32_of(x(spring[i]).fsub(t).fdiv(x(1.0).fsub(t)));
            }
        }
        if loops <= 0 {
            break;
        }
    }
    let st = rd::<u8>(this, 0x36) >> 3;
    let mut thr = f32::from_bits(0x3CA3_D70A);
    let mut roll = gf(0x8D_3170);
    if st != 0 && st != 8 {
        thr = f32::from_bits(0x3D23_D70A);
        roll = f32_of(x(roll).fdouble());
    }
    if !lt(x(fwd_speed), x(thr)) {
        return;
    }
    let one = 0x3F80_0000u32;
    let cb = |k: isize| rd::<u32>(this, 0x7D4 + 4 * k);
    let cl = |k: isize| lt(x(rd::<f32>(this, 0x7D4 + 4 * k)), x(1.0));
    let d = if cb(0) == one && cb(1) == one && (cl(2) || cl(3)) {
        1.0f32
    } else if lt(x(fwd_speed), x(thr)) && cb(2) == one && cb(3) == one && (cl(0) || cl(1)) {
        -1.0
    } else {
        return;
    };
    let mut z = CVector::new(0.0, 0.0, 1.0);
    let mut r = CVector::default();
    let mat = rd::<GPtr<u8>>(this, 0x14);
    cross_product(GPtr::from_raw(&mut r), mat.byte_add(0x10).cast(), GPtr::from_raw(&mut z));
    let m = rd::<GPtr<u8>>(this, 0x14);
    let dot = x(r.y).fmul_m(vf(m, 1)).fadd(x(r.z).fmul_m(vf(m, 2))).fadd(x(r.x).fmul_m(vf(m, 0))).fabs();
    if !lt(dot, x(f32::from_bits(0x3F19_999A))) {
        return;
    }
    let colm = rd::<GPtr<u8>>(crate::common::model_info(i32::from(model as i16)), 0x14);
    let k = x(roll).fmul_m(rd::<f32>(this, 0x90)).fmul_m(d);
    let force = CVector::new(f32_of(k.fmul_m(vf(m, 8))), f32_of(k.fmul_m(vf(m, 9))), f32_of(k.fmul_m(vf(m, 10))));
    let b = x(rd::<f32>(colm, 0xC));
    let point = CVector::new(f32_of(b.fmul_m(vf(m, 0))), f32_of(b.fmul_m(vf(m, 1))), f32_of(b.fmul_m(vf(m, 2))));
    apply_turn_force(this, force, point);
    let mut a = CVector::new(0.0, 0.0, 1.0);
    let mut out = CVector::default();
    let r2 = read_vec(cross_product(GPtr::from_raw(&mut out), GPtr::from_raw(&mut a), rd::<GPtr<u8>>(this, 0x14).byte_add(0x10).cast()).cast());
    let k2 = x(roll).fmul_m(rd::<f32>(this, 0x8C)).fmul_m(d);
    apply_move_force(this, CVector::new(f32_of(k2.fmul_m(r2.x)), f32_of(k2.fmul_m(r2.y)), f32_of(k2.fmul_m(r2.z))));
}

pub const HOOKS: &[HookReg] = &[
    HOOK_VEHICLE_PROCESS_WHEEL,
    HOOK_VEHICLE_PROCESS_BIKE_WHEEL,
    HOOK_AUTOMOBILE_PROCESS_SUSPENSION,
];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
