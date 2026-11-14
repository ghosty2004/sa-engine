//! WRITER-VEH-BIG (batch `vehbig-1`): `CBmx::ProcessAI`, the BMX's per-status driver (rider input,
//! pedalling and sprint energy, simple / physical AI, abandoned, wrecked, disabled). Ported from
//! the 1.0 US bytes (gta-reversed has no code for it).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;

use crate::common::{g, rd, wr, x, xi};

use crate::bindings::calculate_gear_for_simple_car;
use crate::bindings::do_drive_by_shootings;
use crate::bindings::do_soft_ground_resistance;
use crate::bindings::process_wheel_rotation;
use sa_core::bindings::vector_magnitude;
use sa_pop::bindings::car_gun_just_down;
use sa_pop::bindings::steer_ai_car_with_physics;
use sa_pop::bindings::update_car_ai;
use sa_pop::bindings::update_car_on_rails;
use sa_stream::bindings::ped_is_player;
use sa_world::bindings::get_button_sprint_results;
use sa_world::bindings::get_pad_from_player;
use sa_world::bindings::physical_process_control;
use crate::bindings::activate_bomb;
use sa_core::bindings::clump_get_association as get_association;
use sa_core::bindings::prune_references;
use sa_world::bindings::handle_sprint_energy;
use sa_world::bindings::pad_get_accelerate as get_accelerate;
use sa_world::bindings::pad_get_steering_left_right as get_steering_left_right;
use sa_world::bindings::player_control_button_sprint as control_button_sprint;
use sa_world::bindings::stats_get_fat_and_muscle_modifier as get_fat_and_muscle_modifier;
use sa_world::bindings::vector_add;
use sa_world::bindings::vector_scale as scalar_mul_vector;
orig! {
    /// `CPhysical::ApplyTurnForce(CVector, CVector)` (0x542A50)
    fn apply_turn_force(e: GPtr<u8>, force: CVector, point: CVector) = thiscall 0x542A50 => sa_world::physical::forces::apply_turn_force;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0)
    fn apply_move_force(e: GPtr<u8>, force: CVector) = thiscall 0x5429F0 => sa_world::physical::forces::apply_move_force;
    /// `DotProduct(const RwV3d*, const RwV3d*)` (0x40FDB0), ST0
    fn dot_product(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => dot_native;
    /// `CBmx::ProcessBunnyHop()` (0x6C0590)
    fn process_bunny_hop(v: GPtr<u8>) = thiscall 0x6C0590 => crate::bike2::bmx_process_bunny_hop;
    /// `max(float, float)` (0x420800), ST0
    fn max_f(a: f32, b: f32) -> f64 = cdecl 0x420800 => max_native;
    /// `CPad::GetAccelerateJustDown()` (0x540440): bool in AL
    fn get_accelerate_just_down(pad: GPtr<u8>) -> u32 = thiscall 0x540440 => unported;
    /// `CStats::UpdateStatsWhenCycling(bool, CBmx*)` (0x55C780)
    fn update_stats_when_cycling(sprint: u32, v: GPtr<u8>) = cdecl 0x55C780 => unported;
    /// `CBike::PlayHornIfNecessary()` (0x6B7130)
    fn play_horn_if_necessary(v: GPtr<u8>) = thiscall 0x6B7130 => crate::bike::play_horn_if_necessary;
    /// `CBike::ReduceHornCounter()` (0x6B5F90)
    fn reduce_horn_counter(v: GPtr<u8>) = thiscall 0x6B5F90 => crate::bike::reduce_horn_counter;
    /// `CBike::ProcessControlInputs(uint8)` (vtable +0x60)
    fn v_process_control_inputs(v: GPtr<u8>, player: u32) = thiscall vslot 0x60 => unported;
}

/// Phases B/C side of `max` 0x420800: `a` when `a > b`, else `b`.
pub fn max_native(a: f32, b: f32) -> f64 {
    if x(a).fcom(x(b)).test_any(0x41) {
        f64::from(b)
    } else {
        f64::from(a)
    }
}

/// Phases B/C side of `DotProduct` 0x40FDB0.
pub fn dot_native(a: GPtr<u8>, b: GPtr<u8>) -> f64 {
    sa_math::vector::dot_product_x87(&rv(a, 0), &rv(b, 0)).to_f64()
}

/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7_CB5C;
const F1: u32 = 0x3F80_0000;

fn rv(p: GPtr<u8>, off: isize) -> CVector {
    CVector::new(rd::<f32>(p, off), rd::<f32>(p, off + 4), rd::<f32>(p, off + 8))
}

use sa_core::field::ldx as ld;

use sa_math::fpu::xc as c;

use sa_math::fpu::st0;

fn ts() -> f32 {
    g::<f32>(TIME_STEP).read()
}

use sa_math::fpu::le_or_nan as le;

use sa_math::fpu::ge_or_nan as ge;

fn sq3(p: GPtr<u8>, off: isize) -> X87 {
    let (a, b, cc) = (ld(p, off), ld(p, off + 4), ld(p, off + 8));
    a.fmul(a).fadd(b.fmul(b)).fadd(cc.fmul(cc))
}

fn add_flags(flags: GPtr<u32>) {
    flags.write(flags.read().wrapping_add(2));
}

/// `CBmx::ProcessAI(uint32& extraHandlingFlags)` (0x6C1470, `ret 4`); always returns true.
///
/// Clears the horn bits of +0x3DB and the free-wheel flag +0x834. Per status: player: flags += 2,
/// +0x614 bit 3 cleared; a player rider gets `ProcessControlInputs`, the centre-of-mass y from the
/// lean +0x650 (`bmxHandling[+0xC or +4] * lean + com.y`), the pedal / brake lean torque around up
/// at `com + forward` (`-(timeStep * k)`, k from the gas (0x1D4: `gas * 0.7 + 0.3`, else `(gas + 1)
/// / 2`), the handling, turn mass, lean and `min(|v|, 0.1)`), references pruning, drive-bys, soft
/// ground, the in-air steering roll (all four compressions exactly 1.0: `(lr / 128) * turnMass *
/// timeStep * 0.002` along right at forward, when the up turn speed allows it), the bunny hop, and
/// the sprint: a strong sprint (`ControlButtonSprint > 1.2`, or a sprint result > 1 with the
/// pedalling association 0xD0 blended in) sets +0x614 bit 5 and copies the player's sprint value
/// into +0x818; else `HandleSprintEnergy(false, max(0.5, 1 - accel / 255 / 2))` and +0x818 = 4.9
/// while sprinting (gas forced to 1 when idle), or decays by 0.4 (4.9 on a fresh accelerate) while
/// the player data +0x18 is negative, else 0; then the cycling stats and the bomb. Simple: car AI,
/// physics, rails, gear, both wheel rotations (`ProcessWheelRotation`, radius = model wheel scale /
/// 2), horn. Physics: car AI, steering, horn, flags += 2, the wait handbrake, +0x818 decay by
/// `timeStep * 0.02` (above 5) or `* 0.01` (doubled for +0x42F bit 1). Abandoned / wrecked /
/// disabled: brakes and inputs reset. Then, with a rider whose pedalling animations (0xC7/0xC6
/// or 0xCA/0xCB/0xCC) are blended above 0.5: gas 0 and the free-wheel flag when a wheel touches;
/// else above +0x818 = 5 with a rear wheel down the pedal push `((ts * mass * 0.3) * k) * 0.008 *
/// forward`, k = clamp(2.4 - 1.5 * (v . forward) / handling+0x88, 0, 2), times the player's
/// fat/muscle modifier or 1.25 for +0x42F bit 1.
///
/// The original reuses its argument slot (the flags pointer) as a float local once done with it.
#[hook(addr = 0x6c1470, conv = thiscall, name = "CBmx::ProcessAI", this = "CBmx",
       expect = "83 EC 44 53 55 56 8B F1",
       route = "L2", hazards = "stack-residue",
       recs = "driving",
       note = "the original continues through the protection fragment 0x401797 (`mov eax, [esi+0x460]; jmp 0x6C1A04`); own argument slot rewritten")]
pub fn bmx_process_ai(this: GPtr<u8>, flags: GPtr<u32>) -> u8 {
    let mi = crate::common::model_info(i32::from(rd::<i16>(this, 0x22)));
    wr::<u8>(this, 0x3DB, rd::<u8>(this, 0x3DB) & 0xFC);
    wr::<u8>(this, 0x834, 0);
    match rd::<u8>(this, 0x36) >> 3 {
        0 => {
            add_flags(flags);
            wr::<u8>(this, 0x614, rd::<u8>(this, 0x614) & 0xF7);
            let d = rd::<GPtr<u8>>(this, 0x460);
            if !d.is_null() && ped_is_player(d) != 0 {
                player(this, flags);
            }
        }
        1 => add_flags(flags),
        2 => {
            update_car_ai(this);
            physical_process_control(this);
            update_car_on_rails(this);
            let speed = ld(this, 0x3C8).fmul_m(f32::from_bits(0x3CA3_D70A)).to_f32();
            let dl = rd::<u8>(this, 0x805);
            wr::<u8>(this, 0x804, 2);
            wr::<u8>(this, 0x806, dl);
            wr::<u8>(this, 0x805, 2);
            calculate_gear_for_simple_car(
                rd::<GPtr<u8>>(this, 0x384).byte_add(0x2C),
                speed,
                this.byte_add(0x4B4),
            );
            for (k, off) in [(0x40isize, 0x750isize), (0x44, 0x754)] {
                let r = ld(mi, k).fmul_m(0.5).to_f32();
                let fwd = rd::<GPtr<u8>>(this, 0x14).byte_add(0x10);
                let w = st0(process_wheel_rotation(this, 0, fwd, this.byte_add(0x44), r));
                wr::<f32>(this, off, w.fadd_m(rd::<f32>(this, off)).to_f32());
            }
            play_horn_if_necessary(this);
            reduce_horn_counter(this);
            wr::<u8>(this, 0x42B, rd::<u8>(this, 0x42B) & 0xDE);
            wr::<u8>(this, 0x614, rd::<u8>(this, 0x614) & 0x7F);
            wr::<u32>(this, 0x818, 0);
        }
        3 => {
            update_car_ai(this);
            steer_ai_car_with_physics(this);
            play_horn_if_necessary(this);
            add_flags(flags);
            let dl = rd::<u8>(this, 0x614) & 0x7F;
            wr::<u8>(this, 0x614, dl);
            if rd::<u8>(this, 0x42A) & 8 != 0 {
                let al = rd::<u8>(this, 0x428) | 0x20;
                wr::<u32>(this, 0x49C, 0);
                wr::<u32>(this, 0x4A0, F1);
                wr::<u8>(this, 0x428, al);
            } else {
                wr::<u8>(this, 0x614, dl & 0xF7);
            }
            if !le(ld(this, 0x818), X87::zero()) {
                let d = if !le(ld(this, 0x818), c(0x40A0_0000)) {
                    x(ts()).fmul_m(f32::from_bits(0x3CA3_D70A))
                } else {
                    let d = x(ts()).fmul_m(f32::from_bits(0x3C23_D70A));
                    if rd::<u8>(this, 0x42F) & 2 != 0 {
                        d.fadd(d)
                    } else {
                        d
                    }
                };
                wr::<f32>(this, 0x818, d.fsubr_m(rd::<f32>(this, 0x818)).to_f32());
                if !ge(ld(this, 0x818), X87::zero()) {
                    wr::<u32>(this, 0x818, 0);
                }
            }
        }
        4 => {
            wr::<u32>(this, 0x4A0, 0);
            let slow = !ge(sq3(this, 0x44), c(0x3C23_D70A));
            let on = slow || rd::<u8>(this, 0x614) & 0x10 != 0;
            let al = rd::<u8>(this, 0x428);
            wr::<u8>(this, 0x428, if on { al | 0x20 } else { al & 0xDF });
            wr::<u32>(this, 0x49C, 0);
            wr::<u32>(this, 0x514, 0);
            let occupied =
                rd::<u32>(this, 0x460) != 0 || rd::<u32>(this, 0x464) != 0 || rd::<u8>(this, 0x42A) & 8 != 0;
            if occupied && rd::<u8>(this, 0x614) & 0x10 == 0 {
                add_flags(flags);
            }
            let al = rd::<u8>(this, 0x42A);
            wr::<u32>(this, 0x654, 0);
            wr::<u32>(this, 0x658, 0);
            wr::<u8>(this, 0x614, rd::<u8>(this, 0x614) & 0x7F);
            wr::<u32>(this, 0x818, 0);
            if al & 8 != 0 {
                let a = rd::<u8>(this, 0x428) | 0x20;
                wr::<u32>(this, 0x49C, 0);
                wr::<u32>(this, 0x4A0, F1);
                wr::<u8>(this, 0x428, a);
            }
        }
        5 => {
            let cl = rd::<u8>(this, 0x428) | 0x20;
            let al = rd::<u8>(this, 0x614) & 0x7F;
            wr::<u32>(this, 0x4A0, 0x3D4C_CCCD);
            wr::<u8>(this, 0x428, cl);
            wr::<u32>(this, 0x494, 0);
            wr::<u32>(this, 0x49C, 0);
            wr::<u32>(this, 0x514, 0);
            wr::<u8>(this, 0x614, al);
            wr::<u32>(this, 0x818, 0);
            wr::<u32>(this, 0x654, 0);
            wr::<u32>(this, 0x658, 0);
        }
        9 => {
            let al = rd::<u8>(this, 0x428);
            if !ge(sq3(this, 0x44), c(0x3C23_D70A)) {
                wr::<u32>(this, 0x4A0, F1);
                wr::<u8>(this, 0x428, al | 0x20);
            } else {
                wr::<u32>(this, 0x4A0, 0);
                wr::<u8>(this, 0x428, al & 0xDF);
            }
            wr::<u32>(this, 0x494, 0);
            wr::<u32>(this, 0x49C, 0);
            wr::<u32>(this, 0x514, 0);
            add_flags(flags);
            wr::<u8>(this, 0x614, rd::<u8>(this, 0x614) & 0x7F);
            wr::<u32>(this, 0x818, 0);
        }
        _ => {}
    }
    pedalling(this);
    1
}

/// 0x6C14DB..0x6C19FC: a player rider.
fn player(this: GPtr<u8>, flags: GPtr<u32>) {
    let driver = rd::<GPtr<u8>>(this, 0x460);
    v_process_control_inputs(this, u32::from(rd::<u8>(driver, 0x598)));
    let pad = get_pad_from_player(rd::<GPtr<u8>>(this, 0x460));
    let ped = rd::<GPtr<u8>>(this, 0x460);
    let bh = rd::<GPtr<u8>>(this, 0x63C);
    let h = rd::<GPtr<u8>>(this, 0x384);
    let lean = rd::<f32>(this, 0x650);
    let min_speed = |this: GPtr<u8>| {
        if !le(st0(vector_magnitude(GPtr::<u8>::cast::<u32>(this.byte_add(0x44)))), c(0x3DCC_CCCD)) {
            c(0x3DCC_CCCD)
        } else {
            st0(vector_magnitude(GPtr::<u8>::cast::<u32>(this.byte_add(0x44))))
        }
    };
    // the pedal / brake lean torque
    let torque = if !ge(x(lean), X87::zero()) {
        wr::<f32>(
            this,
            0xA8,
            ld(bh, 0xC).fmul_m(lean).fadd_m(rd::<f32>(h, 0x18)).to_f32(),
        );
        let braking = x(rd::<f32>(this, 0x4A0)).fcom(X87::zero()).test_parity(0x44)
            || rd::<u8>(this, 0x428) & 0x20 != 0;
        if braking && rd::<u8>(this, 0x804) != 0 {
            None
        } else {
            let mv = min_speed(this);
            let f = if rd::<u16>(this, 0x22) == 0x1D4 {
                ld(this, 0x49C)
                    .fmul_m(f32::from_bits(0x3F33_3333))
                    .fadd_m(f32::from_bits(0x3E99_999A))
            } else {
                ld(this, 0x49C).fadd_m(1.0).fmul_m(0.5)
            };
            let bh = rd::<GPtr<u8>>(this, 0x63C);
            Some(
                f.fmul_m(rd::<f32>(bh, 0x10))
                    .fmul_m(rd::<f32>(this, 0x90))
                    .fmul_m(rd::<f32>(this, 0x650))
                    .fmul(mv)
                    .to_f32(),
            )
        }
    } else {
        wr::<f32>(
            this,
            0xA8,
            ld(bh, 4).fmul_m(lean).fadd_m(rd::<f32>(h, 0x18)).to_f32(),
        );
        if ge(ld(this, 0x4A0), X87::zero()) && rd::<u8>(this, 0x804) != 0 {
            None
        } else {
            let mv = min_speed(this);
            let bh = rd::<GPtr<u8>>(this, 0x63C);
            Some(
                ld(bh, 8)
                    .fmul_m(rd::<f32>(this, 0x90))
                    .fmul(mv)
                    .fmul_m(rd::<f32>(this, 0x650))
                    .to_f32(),
            )
        }
    };
    if let Some(k) = torque {
        let m = rd::<GPtr<u8>>(this, 0x14);
        let mut pc = CVector::default();
        let point = rv(
            vector_add(
                GPtr::from_raw(&mut pc).cast(),
                this.byte_add(0xA4),
                m.byte_add(0x10),
            ),
            0,
        );
        let s = x(ts()).fmul_m(k).fchs().to_f32();
        let mut fc = CVector::default();
        let force = rv(
            scalar_mul_vector(GPtr::from_raw(&mut fc).cast(), s, m.byte_add(0x20)),
            0,
        );
        apply_turn_force(this, force, point);
    }
    prune_references(this);
    if rd::<u8>(this, 0x36) & 0xF8 == 0 {
        do_drive_by_shootings(this);
    }
    do_soft_ground_resistance(this, flags);

    // in-air steering roll (all compressions exactly 1.0)
    if (0..4isize).all(|i| rd::<u32>(this, 0x710 + 4 * i) == F1) {
        let m = rd::<GPtr<u8>>(this, 0x14);
        let d = st0(dot_product(this.byte_add(0x50), m.byte_add(0x20))).to_f32();
        let lim = c(0x3D23_D70A);
        let lr0 = || i32::from((get_steering_left_right(pad) as u32) as u16 as i16);
        let wants = if !ge(x(d), lim) && !ge(xi(lr0()), X87::zero()) {
            true
        } else {
            !ge(lim.fchs(), x(d)) && !le(xi(lr0()), X87::zero())
        };
        if wants {
            let m = rd::<GPtr<u8>>(this, 0x14);
            let t = ts();
            let lr = lr0();
            let s = xi(lr)
                .fmul_m(f32::from_bits(0x3C00_0000))
                .fmul_m(rd::<f32>(this, 0x90))
                .fmul_m(t)
                .fmul_m(f32::from_bits(0x3B03_126F))
                .to_f32();
            let mut fc = CVector::default();
            let force = rv(scalar_mul_vector(GPtr::from_raw(&mut fc).cast(), s, m), 0);
            let point = rv(m, 0x10);
            apply_turn_force(this, force, point);
        }
    }
    process_bunny_hop(this);
    let assoc = get_association(rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x460), 0x18), 0xD0);
    let strong = !le(st0(control_button_sprint(ped, 1)), c(0x3F99_999A))
        || (!le(st0(get_button_sprint_results(ped, 1)), c(F1))
            && !assoc.is_null()
            && !le(ld(assoc, 0x18), c(0x3F00_0000)));
    if strong {
        wr::<u8>(this, 0x614, rd::<u8>(this, 0x614) | 0x20);
        wr::<u32>(this, 0x818, rd::<u32>(rd::<GPtr<u8>>(ped, 0x480), 0x1C));
    } else {
        let acc = get_accelerate(pad) as u16 as i16;
        let r = xi(i32::from(acc))
            .fmul_m(f32::from_bits(0x3B80_8081))
            .fmul_m(0.5)
            .fsubr_m(1.0)
            .to_f32();
        let rate = st0(max_f(0.5, r)).to_f32();
        handle_sprint_energy(ped, 0, rate);
        if !le(st0(get_button_sprint_results(ped, 1)), X87::zero()) {
            wr::<u32>(this, 0x818, 0x409C_CCCD);
            if !ld(this, 0x49C).fcom(X87::zero()).test_parity(0x44)
                && !ld(this, 0x4A0).fcom(X87::zero()).test_parity(0x44)
            {
                wr::<u32>(this, 0x49C, F1);
            }
        } else if ge(ld(rd::<GPtr<u8>>(ped, 0x480), 0x18), X87::zero()) {
            wr::<u32>(this, 0x818, 0);
        } else if get_accelerate_just_down(pad) as u8 != 0 {
            wr::<u32>(this, 0x818, 0x409C_CCCD);
        } else {
            let v = ld(this, 0x818).fsub_m(f32::from_bits(0x3ECC_CCCD)).to_f32();
            wr::<f32>(this, 0x818, st0(max_f(0.0, v)).to_f32());
        }
    }
    update_stats_when_cycling(u32::from((rd::<u8>(this, 0x614) >> 5) & 1), this);
    if car_gun_just_down(pad) as u32 & 0xFFFF != 0 {
        activate_bomb(this);
    }
}

/// 0x6C1A04..0x6C2007 (reached through the fragment 0x401797): pedalling animations and the push.
fn pedalling(this: GPtr<u8>) {
    let d = rd::<GPtr<u8>>(this, 0x460);
    let half = c(0x3F00_0000);
    if !d.is_null() {
        let clump = |this: GPtr<u8>| rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x460), 0x18);
        let mut a = get_association(rd::<GPtr<u8>>(d, 0x18), 0xC7);
        if a.is_null() || !ge(ld(a, 0x18), half) {
            a = get_association(clump(this), 0xC6);
        }
        let mut b = get_association(clump(this), 0xCA);
        if b.is_null() || !ge(ld(b, 0x18), half) {
            b = get_association(clump(this), 0xCB);
            if b.is_null() || !ge(ld(b, 0x18), half) {
                b = get_association(clump(this), 0xCC);
            }
        }
        let a_on = !a.is_null() && !le(ld(a, 0x18), half);
        let b_on = !b.is_null() && !le(ld(b, 0x18), half);
        if a_on || b_on {
            wr::<u32>(this, 0x49C, 0);
            if rd::<u8>(this, 0x428) & 0x20 != 0 {
                return;
            }
            if (0..4isize).any(|i| !ge(ld(this, 0x710 + 4 * i), c(F1))) {
                wr::<u8>(this, 0x834, 1);
            }
            return;
        }
    }
    if le(ld(this, 0x818), c(0x40A0_0000)) {
        return;
    }
    if ge(ld(this, 0x718), c(F1)) && ge(ld(this, 0x71C), c(F1)) {
        return;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let h = rd::<GPtr<u8>>(this, 0x384);
    let v = ld(this, 0x4C)
        .fmul_m(rd::<f32>(m, 0x18))
        .fadd(ld(this, 0x48).fmul_m(rd::<f32>(m, 0x14)))
        .fadd(ld(this, 0x44).fmul_m(rd::<f32>(m, 0x10)))
        .fdiv_m(rd::<f32>(h, 0x88))
        .fmul_m(1.5)
        .fsubr_m(f32::from_bits(0x4019_999A));
    let two = c(0x4000_0000);
    // A4: the clamped push factor as stored
    let k = if !le(v, two) {
        two
    } else if !ge(v, X87::zero()) {
        X87::zero()
    } else {
        v
    };
    let k = if rd::<u8>(this, 0x36) & 0xF8 == 0 {
        let a4 = k.to_f32();
        st0(get_fat_and_muscle_modifier(5)).fmul_m(a4)
    } else if rd::<u8>(this, 0x42F) & 2 != 0 {
        k.fmul_m(1.25)
    } else {
        k
    };
    let f = x(ts())
        .fmul_m(rd::<f32>(this, 0x8C))
        .fmul_m(f32::from_bits(0x3E99_999A))
        .fmul(k)
        .fmul_m(f32::from_bits(0x3C03_126F));
    let m = rd::<GPtr<u8>>(this, 0x14);
    let force = CVector::new(
        f.fmul_m(rd::<f32>(m, 0x10)).to_f32(),
        f.fmul_m(rd::<f32>(m, 0x14)).to_f32(),
        f.fmul_m(rd::<f32>(m, 0x18)).to_f32(),
    );
    apply_move_force(this, force);
}

pub const HOOKS: &[HookReg] = &[HOOK_BMX_PROCESS_AI];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
