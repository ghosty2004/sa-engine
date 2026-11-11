//! WRITER-VEH-BIG (batch `vehbig-1`): `CAutomobile::ProcessAI`, the per-status driver (player input,
//! simple / physical AI, abandoned, wrecked, remote control, trailers). Ported from the 1.0 US
//! bytes; gta-reversed as a structural guide only.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;
use sa_world::world::common::{find_player_coors, find_player_ped, find_player_vehicle};

use crate::common::{g, rd, wr, x, xi};

orig! {
    /// `CEntity::GetColModel()` (0x535300)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => sa_world::world::entity::get_col_model;
    /// `CWorld::FindPlayerSpeed(int32)` (0x56E090)
    fn find_player_speed(id: i32) -> GPtr<u8> = cdecl 0x56E090 => sa_world::world::cworld::find_player_speed;
    /// `CVector::Magnitude()` (0x4082C0), ST0
    fn vector_magnitude(v: GPtr<u8>) -> f64 = thiscall 0x4082C0 => unported;
    /// `VectorSub(CVector*, const CVector*, const CVector*)` (0x40FE60)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => sa_world::world::mvpa_math::vector_sub;
    /// `CAutomobile::ScanForCrimes()` (0x6ADFF0)
    fn scan_for_crimes(v: GPtr<u8>) = thiscall 0x6ADFF0 => crate::mvpb_a2::scan_for_crimes;
    /// `operator*(const CVector&, float)` (0x40FEC0, cdecl, hidden return pointer)
    fn vector_mul_scalar(out: GPtr<u8>, v: GPtr<u8>, s: f32) -> GPtr<u8> = cdecl 0x40FEC0 => sa_world::world::mvpa_math::vector_mul_scalar;
    /// `VectorAdd(CVector*, const CVector*, const CVector*)` (0x40FE30)
    fn vector_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => sa_world::world::mvpa_math::vector_add;
    /// `CWorld::ProcessLineOfSight(...)` (0x56BA00)
    fn process_line_of_sight(a: GPtr<u8>, b: GPtr<u8>, cp: GPtr<u8>, e: GPtr<u8>, b1: u32, v: u32, p: u32, o: u32, d: u32, s: u32, ic: u32, sc: u32) -> u8 = cdecl 0x56BA00 => unported;
    /// `CCarAI::GetCarToParkAtCoors(CVehicle*, CVector*)` (0x41D350), ST0 (dropped)
    fn get_car_to_park_at_coors(v: GPtr<u8>, p: GPtr<u8>) -> f64 = cdecl 0x41D350 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::CarGunJustDown()` (0x53FFE0): the caller tests AX
    fn car_gun_just_down(pad: GPtr<u8>) -> u32 = thiscall 0x53FFE0 => unported;
    /// `CRemote::TakeRemoteControlledCarFromPlayer(bool)` (0x45AE80)
    fn take_remote_controlled_car_from_player(b: u32) = cdecl 0x45AE80 => unported;
    /// `CAutomobile::RcbanditCheckHitWheels()` (0x6B45E0): the caller tests AX
    fn rcbandit_check_hit_wheels(v: GPtr<u8>) -> u32 = thiscall 0x6B45E0 => unported;
    /// `CPlayerPed::GetPadFromPlayer()` (0x609560)
    fn get_pad_from_player(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CEntity::PruneReferences()` (0x571A90)
    fn prune_references(e: GPtr<u8>) = thiscall 0x571A90 => sa_world::world::entity::prune_references;
    /// `CWorld::FindPlayerSlotWithRemoteVehiclePointer(CEntity*)` (0x563FD0)
    fn find_player_slot_with_remote_vehicle_pointer(v: GPtr<u8>) -> i32 = cdecl 0x563FD0 => sa_world::world::cworld::find_player_slot_with_remote_vehicle_pointer;
    /// `CVehicle::DoDriveByShootings()` (0x741FD0)
    fn do_drive_by_shootings(v: GPtr<u8>) = thiscall 0x741FD0 => unported;
    /// `CPad::GetSteeringLeftRight()` (0x53FB80): int16 in AX
    fn get_steering_left_right(pad: GPtr<u8>) -> u32 = thiscall 0x53FB80 => unported;
    /// `CPad::GetSteeringUpDown()` (0x53FBD0): int16 in AX
    fn get_steering_up_down(pad: GPtr<u8>) -> u32 = thiscall 0x53FBD0 => unported;
    /// `CPad::GetHandBrake()` (0x540040): the caller tests AX
    fn get_hand_brake(pad: GPtr<u8>) -> u32 = thiscall 0x540040 => unported;
    /// `CPad::GetAccelerate()` (0x5403F0): the caller tests AX
    fn get_accelerate(pad: GPtr<u8>) -> u32 = thiscall 0x5403F0 => unported;
    /// `CStats::GetFatAndMuscleModifier(eStatModAbilities)` (0x559AF0), ST0
    fn get_fat_and_muscle_modifier(ability: u32) -> f64 = cdecl 0x559AF0 => unported;
    /// `min(float, float)` (0x404330), ST0
    fn min_f(a: f32, b: f32) -> f64 = cdecl 0x404330 => min_native;
    /// `max(float, float)` (0x420800), ST0
    fn max_f(a: f32, b: f32) -> f64 = cdecl 0x420800 => max_native;
    /// `DotProduct(const RwV3d*, const RwV3d*)` (0x40FDB0), ST0
    fn dot_product(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => dot_native;
    /// `operator*(float, const CVector&)` (0x40FE90, cdecl, hidden return pointer)
    fn scalar_mul_vector(out: GPtr<u8>, s: f32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => sa_world::world::mvpa_math::scalar_mul_vector;
    /// `CPhysical::ApplyTurnForce(CVector, CVector)` (0x542A50)
    fn apply_turn_force(e: GPtr<u8>, force: CVector, point: CVector) = thiscall 0x542A50 => sa_world::physical::forces::apply_turn_force;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0)
    fn apply_move_force(e: GPtr<u8>, force: CVector) = thiscall 0x5429F0 => sa_world::physical::forces::apply_move_force;
    /// `CAutomobile::DoSoftGroundResistance(uint32&)` (0x6A4AF0)
    fn do_soft_ground_resistance(v: GPtr<u8>, flags: GPtr<u32>) = thiscall 0x6A4AF0 => unported;
    /// `CCarAI::UpdateCarAI(CVehicle*)` (0x41DA30)
    fn update_car_ai(v: GPtr<u8>) = cdecl 0x41DA30 => sa_pop::carctrl4::update_car_ai;
    /// `CPhysical::ProcessControl()` (0x5485E0)
    fn physical_process_control(v: GPtr<u8>) = thiscall 0x5485E0 => sa_world::physical::attach::process_control;
    /// `CCarCtrl::UpdateCarOnRails(CVehicle*)` (0x436540)
    fn update_car_on_rails(v: GPtr<u8>) = cdecl 0x436540 => sa_pop::carctrl4::update_car_on_rails;
    /// `cTransmission::CalculateGearForSimpleCar(float, uint8&)` (0x6D0530)
    fn calculate_gear_for_simple_car(t: GPtr<u8>, speed: f32, gear: GPtr<u8>) = thiscall 0x6D0530 => crate::mvpb_a2::calculate_gear_for_simple_car;
    /// `CVehicle::ProcessWheelRotation(tWheelState, const CVector&, const CVector&, float)` (0x6D1230), ST0
    fn process_wheel_rotation(v: GPtr<u8>, state: u32, fwd: GPtr<u8>, speed: GPtr<u8>, radius: f32) -> f64 = thiscall 0x6D1230 => unported;
    /// `CAutomobile::PlayHornIfNecessary()` (0x6A3820)
    fn play_horn_if_necessary(v: GPtr<u8>) = thiscall 0x6A3820 => crate::mvpb_a1::play_horn_if_necessary;
    /// `CCarCtrl::SteerAICarWithPhysics(CVehicle*)` (0x437C20)
    fn steer_ai_car_with_physics(v: GPtr<u8>) = cdecl 0x437C20 => sa_pop::carctrl3::steer_ai_car_with_physics;
    /// `CCarCtrl::ReconsiderRoute(CVehicle*)` (0x42FC40)
    fn reconsider_route(v: GPtr<u8>) = cdecl 0x42FC40 => sa_pop::carctrl4::reconsider_route;
    /// `SurfaceInfos_c::GetAdhesionGroup(uint32)` (0x55E5C0)
    fn get_adhesion_group(si: GPtr<u8>, id: u32) -> u32 = thiscall 0x55E5C0 => unported;
    /// `CVector::SquaredMagnitude()` (0x406DA0), ST0
    fn vector_magnitude_sqr(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740)
    fn find_active_task_by_type(tm: GPtr<u8>, kind: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CVehicle::CanUpdateHornCounter()`-like check (0x4F4CE0)
    fn has_valid_45c(v: GPtr<u8>) -> u32 = thiscall 0x4F4CE0 => crate::vehicle2::has_valid_45c;
    /// `CAutomobile::ProcessControlInputs(uint8)` (vtable +0x60)
    fn v_process_control_inputs(v: GPtr<u8>, player: u32) = thiscall vslot 0x60 => unported;
    /// `CAutomobile::BlowUpCar(CEntity*, bool)` (vtable +0xA4)
    fn v_blow_up_car(v: GPtr<u8>, culprit: GPtr<u8>, in_cutscene: u32) = thiscall vslot 0xA4 => unported;
    /// `CAutomobile::BreakTowLink()` (vtable +0xF8)
    fn v_break_tow_link(v: GPtr<u8>) -> u8 = thiscall vslot 0xF8 => unported;
}

/// Phases B/C side of `min` 0x404330: `a` when `a < b`, else `b`.
pub fn min_native(a: f32, b: f32) -> f64 {
    if x(a).fcom(x(b)).test_parity(0x05) {
        f64::from(b)
    } else {
        f64::from(a)
    }
}

/// Phases B/C side of `max` 0x420800: `a` when `a > b`, else `b` (ties and unordered give `b`).
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
/// `CWorld::Players[CWorld::PlayerInFocus]` field at `base` (+0x190 per player)
fn player_field(base: usize) -> GPtr<u32> {
    g::<u32>(base + usize::from(g::<u8>(0xB7_CD74).read()) * 0x190)
}

fn rv(p: GPtr<u8>, off: isize) -> CVector {
    CVector::new(rd::<f32>(p, off), rd::<f32>(p, off + 4), rd::<f32>(p, off + 8))
}

fn ld(p: GPtr<u8>, off: isize) -> X87 {
    x(rd::<f32>(p, off))
}

fn c(bits: u32) -> X87 {
    X87::constant(bits)
}

fn st0(v: f64) -> X87 {
    X87::from_f64_exact(v)
}

/// `fcomp` + `test ah, 0x41` + `jne` taken: `a <= b` or unordered.
fn le(a: X87, b: X87) -> bool {
    a.fcom(b).test_any(0x41)
}

/// `fcomp` + `test ah, 5` + `jp` taken: `a >= b` or unordered.
fn ge(a: X87, b: X87) -> bool {
    a.fcom(b).test_parity(0x05)
}

fn sq3(p: GPtr<u8>, off: isize) -> X87 {
    let (a, b, cc) = (ld(p, off), ld(p, off + 4), ld(p, off + 8));
    a.fmul(a).fadd(b.fmul(b)).fadd(cc.fmul(cc))
}

fn pos_ptr(this: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(this, 0x14);
    if m.is_null() {
        this.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// `CAutomobile::ProcessAI(uint32& extraHandlingFlags)` (0x6B4800, `ret 4`); returns true when the
/// caller (ProcessControl) must stop.
///
/// Clears the horn bits of +0x3DB; a car whose +0x424 index is not active in the table 0x97D6C0
/// stops at once (false). Police chases: with the player in another vehicle, wanted level > 3, a
/// chase mission (2..5) and the player faster than 0.3: flags |= 1, and |= 2 when the player is
/// faster than 0.4 while this car is slower than 0.3, or farther than 50 m; the RC Bandit (not
/// remote-controlled) gets flag 1. Without flag 1 or the perfect-handling cheat: physics cars with
/// handling flag 16 move the centre of mass y to the middle of the first collision line (flag 3
/// set) or reset it to the handling's (flag 3 cleared), and flag 2 lowers its z to `(bboxMin.z -
/// com.z) * 0.4 + com.z`; other statuses reset the centre of mass; with flag 1 or the cheat, z =
/// `+0x878 * 0.3 - +0x898`. Law enforcers scan for crimes (not abandoned / wrecked / player /
/// remote / disabled). Parking cars probe 10 m ahead and 3 m right every 16 frames and park there.
///
/// Per status: player (and remote-controlled): remote detonation / RC Bandit wheel hits (remote
/// only), drivers in some states (0x3D/0x3A/0x3F) stop, else pad lookup, `PruneReferences`, the
/// virtual `ProcessControlInputs`, drive-by shootings, the centre-of-mass lean while airborne
/// (`(lr/128) * bboxMax.z * g 0x8D3138 * min(t - 500, 1000) * 0.002`, sign from the roll), the
/// in-air rotation control (handbrake: roll around up; no throttle: pitch around forward/right
/// by the steering axes, mouse-steering from 0xB73424/0xB73428 clamped to ±1.5) and
/// `DoSoftGroundResistance`, false. Simple: car AI, `CPhysical::ProcessControl`, rail update, gear,
/// wheel rotation from `ProcessWheelRotation`, horn, true. Physics: car AI, physics steering,
/// route, horn, the wait-for-player handbrake, sand slow-down (`-(1 - g 0xC81308) * mass *
/// timeStep * 0.005 * moveSpeed`), false. Abandoned: brake 1.0/0.5/0.2/0 by wheel contacts and
/// speed; wrecked: brake 0.05; disabled player: handbrake when slow or in some driving tasks;
/// trailer: no inputs and `BreakTowLink` without a tractor.
#[hook(addr = 0x6b4800, conv = thiscall, name = "CAutomobile::ProcessAI", this = "CAutomobile",
       expect = "81 EC A4 00 00 00",
       unverified = "vehbig-1", route = "L2", hazards = "uninit-locals",
       recs = "driving,car-ai,grove-walk",
       note = "the parking probe's CColPoint is uninitialised in the original (zeroed here; ProcessLineOfSight writes it on a hit); the in-air control reuses argument slots for min/max operands")]
pub fn automobile_process_ai(this: GPtr<u8>, flags: GPtr<u32>) -> u8 {
    let col = get_col_model(this);
    wr::<u8>(this, 0x3DB, rd::<u8>(this, 0x3DB) & 0xFC);
    let idx = rd::<i8>(this, 0x424);
    if idx >= 0 && g::<u8>((0x97_D6C0 + idx as i32) as usize).read() == 0 {
        return 0;
    }
    chase_flags(this, flags);
    centre_of_mass(this, col, flags);
    let st = rd::<u8>(this, 0x36) >> 3;
    if !matches!(st, 4 | 5 | 0 | 8 | 9) && rd::<u8>(this, 0x428) & 1 != 0 {
        scan_for_crimes(this);
    }
    park_probe(this);

    let dl = rd::<u8>(this, 0x36);
    match dl >> 3 {
        0 => player(this, col, flags, false),
        8 => {
            if car_gun_just_down(get_pad(0)) & 0xFFFF != 0 && g::<u8>(0xC1_CC00).read() == 0 {
                v_blow_up_car(this, find_player_ped(-1), 0);
                take_remote_controlled_car_from_player(1);
            }
            if rd::<u16>(this, 0x22) == 0x1B9 && g::<u8>(0xC1_CC01).read() == 0 {
                let mut p = [0f32; 3];
                find_player_coors(GPtr::from_raw(p.as_mut_ptr()), -1);
                if rcbandit_check_hit_wheels(this) & 0xFFFF != 0 || rd::<u32>(this, 0x40) & 0x100 != 0 {
                    take_remote_controlled_car_from_player(1);
                    v_blow_up_car(this, find_player_ped(-1), 0);
                }
            }
            let remote = player_field(0xB7_CE48).read() as usize == this.addr();
            player(this, col, flags, remote)
        }
        2 => {
            update_car_ai(this);
            physical_process_control(this);
            update_car_on_rails(this);
            let speed = ld(this, 0x3C8).fmul_m(f32::from_bits(0x3CA3_D70A)).to_f32();
            let dl2 = rd::<u8>(this, 0x961);
            wr::<u8>(this, 0x960, 4);
            wr::<u8>(this, 0x962, dl2);
            wr::<u8>(this, 0x961, 4);
            calculate_gear_for_simple_car(
                rd::<GPtr<u8>>(this, 0x384).byte_add(0x2C),
                speed,
                this.byte_add(0x4B4),
            );
            let m = rd::<GPtr<u8>>(this, 0x14);
            let w = st0(process_wheel_rotation(
                this,
                0,
                m.byte_add(0x10),
                this.byte_add(0x44),
                f32::from_bits(0x3EB3_3333),
            ));
            for off in [0x828isize, 0x82C, 0x830, 0x834] {
                wr::<f32>(this, off, w.fadd_m(rd::<f32>(this, off)).to_f32());
            }
            play_horn_if_necessary(this);
            let h = rd::<u32>(this, 0x514);
            if h > 0 {
                wr::<u32>(this, 0x514, h - 1);
            }
            wr::<u8>(this, 0x42B, rd::<u8>(this, 0x42B) & 0xDE);
            1
        }
        3 | 12 => {
            update_car_ai(this);
            steer_ai_car_with_physics(this);
            reconsider_route(this);
            play_horn_if_necessary(this);
            if rd::<u8>(this, 0x42A) & 8 != 0 {
                let al = rd::<u8>(this, 0x428) | 0x20;
                wr::<u32>(this, 0x49C, 0);
                wr::<u32>(this, 0x4A0, 0x3F80_0000);
                wr::<u8>(this, 0x428, al);
            }
            let si = GPtr::from_addr(0xB7_9538);
            let sand = (0..4isize).any(|i| {
                !ge(ld(this, 0x7D4 + 4 * i), c(0x3F80_0000))
                    && get_adhesion_group(si, u32::from(rd::<u8>(this, 0x747 + 0x2C * i))) == 4
            });
            let model = rd::<u16>(this, 0x22);
            if !sand || model == 0x1B9 || model == 0x1EF || model == 0x1A8 {
                return 0;
            }
            wr::<u8>(this, 0x868, rd::<u8>(this, 0x868) | 0x40);
            let gsand = x(g::<f32>(0xC8_1308).read());
            let s = if !le(gsand, X87::zero()) {
                c(0x3F80_0000)
                    .fsub(gsand)
                    .fmul_m(rd::<f32>(this, 0x8C))
                    .fmul_m(g::<f32>(TIME_STEP).read())
                    .fmul_m(f32::from_bits(0x3BA3_D70A))
                    .fchs()
            } else {
                x(g::<f32>(TIME_STEP).read())
                    .fmul_m(rd::<f32>(this, 0x8C))
                    .fmul_m(f32::from_bits(0x3BA3_D70A))
                    .fchs()
            };
            let mut out = CVector::default();
            let f = rv(
                scalar_mul_vector(GPtr::from_raw(&mut out).cast(), s.to_f32(), this.byte_add(0x44)),
                0,
            );
            apply_move_force(this, f);
            0
        }
        4 => {
            for off in [0x8E8isize, 0x8EC, 0x8F0, 0x8F4] {
                if rd::<u32>(this, off) != 0 {
                    wr::<u8>(this, 0x42C, rd::<u8>(this, 0x42C) | 2);
                }
            }
            let sub = rd::<u32>(this, 0x594);
            if sub == 3 || (sub == 4 && !ge(sq3(this, 0x44), c(0x3DCC_CCCD))) {
                wr::<u32>(this, 0x4A0, 0x3F80_0000);
            }
            let brake = if rd::<u8>(this, 0x42C) & 2 != 0 {
                0x3F00_0000
            } else if !ge(sq3(this, 0x44), c(0x3C23_D70A)) {
                0x3E4C_CCCD
            } else {
                0
            };
            wr::<u32>(this, 0x4A0, brake);
            let al = rd::<u8>(this, 0x428) & 0xDF;
            let horn = rd::<u16>(this, 0x45C);
            wr::<u8>(this, 0x428, al);
            wr::<u32>(this, 0x494, 0);
            wr::<u32>(this, 0x49C, 0);
            if horn == 0 || horn == 0xFFFF || dl & 0xF8 == 0x28 {
                wr::<u32>(this, 0x514, 0);
            }
            if rd::<u8>(this, 0x42A) & 8 != 0 {
                wr::<u8>(this, 0x428, al | 0x20);
                wr::<u32>(this, 0x49C, 0);
                wr::<u32>(this, 0x4A0, 0x3F80_0000);
            }
            0
        }
        5 => {
            wr::<u32>(this, 0x4A0, 0x3D4C_CCCD);
            wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) | 0x20);
            stop_inputs(this)
        }
        9 => {
            let brake_on = if !ge(st0(vector_magnitude_sqr(this.byte_add(0x44))), c(0x3C23_D70A)) {
                true
            } else {
                let d = rd::<GPtr<u8>>(this, 0x460);
                if d.is_null() || ped_is_player(d) == 0 {
                    false
                } else {
                    let d = rd::<GPtr<u8>>(this, 0x460);
                    rd::<u32>(d, 0x530) == 0x3F
                        || [0x337, 0x339, 0x329].iter().any(|&t| {
                            let tm = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x460), 0x47C).byte_add(4);
                            !find_active_task_by_type(tm, t).is_null()
                        })
                }
            };
            if brake_on {
                wr::<u32>(this, 0x4A0, 0x3F80_0000);
                wr::<u32>(this, 0x49C, 0);
                wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) | 0x20);
            } else {
                wr::<u32>(this, 0x4A0, 0);
                wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xDF);
            }
            stop_inputs(this)
        }
        10 => {
            wr::<u32>(this, 0x4A0, 0);
            wr::<u8>(this, 0x428, rd::<u8>(this, 0x428) & 0xDF);
            wr::<u32>(this, 0x494, 0);
            wr::<u32>(this, 0x49C, 0);
            if rd::<u32>(this, 0x4C4) == 0 {
                v_break_tow_link(this);
            }
            0
        }
        _ => 0,
    }
}

/// 0x6B5688: steer and gas to 0, the horn counter cleared when the horn may not sound.
fn stop_inputs(this: GPtr<u8>) -> u8 {
    wr::<u32>(this, 0x494, 0);
    wr::<u32>(this, 0x49C, 0);
    if has_valid_45c(this) as u8 == 0 {
        wr::<u32>(this, 0x514, 0);
    }
    0
}

/// 0x6B4841..0x6B4986: the police-chase extra handling flags.
fn chase_flags(this: GPtr<u8>, flags: GPtr<u32>) {
    let pv = find_player_vehicle(-1, 0);
    let chase = 'c: {
        if pv.is_null() || find_player_vehicle(-1, 0) == this {
            break 'c false;
        }
        let w = rd::<GPtr<u8>>(find_player_ped(-1), 0x480);
        let wanted = if w.is_null() {
            GPtr::<u8>::from_addr(0)
        } else {
            rd::<GPtr<u8>>(w, 0)
        };
        if rd::<i32>(wanted, 0x2C) <= 3 {
            break 'c false;
        }
        matches!(rd::<u8>(this, 0x3BA), 2..=5)
    };
    if chase {
        let sp = find_player_speed(-1);
        if !le(sq3(sp, 0).fsqrt(), c(0x3E99_999A)) {
            flags.write(flags.read() | 1);
            let fast = !le(st0(vector_magnitude(find_player_speed(-1))), c(0x3ECC_CCCD))
                && !ge(st0(vector_magnitude(this.byte_add(0x44))), c(0x3E99_999A));
            if fast {
                flags.write(flags.read() | 2);
            } else {
                let mut pc = [0f32; 3];
                let p = find_player_coors(GPtr::from_raw(pc.as_mut_ptr()), -1);
                let mut d = [0f32; 3];
                let r = vector_sub(GPtr::from_raw(d.as_mut_ptr()).cast(), pos_ptr(this), p.cast());
                if !le(st0(vector_magnitude(r)), c(0x4248_0000)) {
                    flags.write(flags.read() | 2);
                }
            }
            return;
        }
    }
    if rd::<u16>(this, 0x22) == 0x1B9 && rd::<u8>(this, 0x36) & 0xF8 != 0x40 {
        flags.write(flags.read() | 1);
    }
}

/// 0x6B4986..0x6B4A6B: the centre of mass.
fn centre_of_mass(this: GPtr<u8>, col: GPtr<u8>, flags: GPtr<u32>) {
    let copy_com = |this: GPtr<u8>| {
        let h = rd::<GPtr<u8>>(this, 0x384);
        for k in 0..3isize {
            wr::<u32>(this, 0xA4 + 4 * k, rd::<u32>(h, 0x14 + 4 * k));
        }
    };
    if flags.read() & 1 != 0 || g::<u8>(0x96_914C).read() != 0 {
        let z = ld(this, 0x878)
            .fmul_m(f32::from_bits(0x3E99_999A))
            .fsub_m(rd::<f32>(this, 0x898));
        wr::<f32>(this, 0xAC, z.to_f32());
        return;
    }
    if rd::<u8>(this, 0x36) & 0xF8 != 0x18 {
        copy_com(this);
        return;
    }
    let mut f = rd::<u32>(this, 0x38C);
    if f & 0x1_0000 != 0 {
        let cd = rd::<GPtr<u8>>(col, 0x2C);
        if rd::<u8>(this, 0x3BA) != 0 && !cd.is_null() && rd::<i8>(cd, 6) > 0 {
            let lines = rd::<GPtr<u8>>(cd, 0x10);
            f |= 8;
            let y = ld(lines, 0x24).fadd_m(rd::<f32>(lines, 4)).fmul_m(0.5);
            wr::<f32>(this, 0xA8, y.to_f32());
        } else {
            copy_com(this);
            f &= 0xFFFF_FFF7;
        }
        wr::<u32>(this, 0x38C, f);
    }
    if rd::<u8>(this, 0x38C) & 4 != 0 {
        let h = rd::<GPtr<u8>>(this, 0x384);
        let z = ld(col, 8)
            .fsub_m(rd::<f32>(h, 0x1C))
            .fmul_m(f32::from_bits(0x3ECC_CCCD))
            .fadd_m(rd::<f32>(h, 0x1C));
        wr::<f32>(this, 0xAC, z.to_f32());
    }
}

/// 0x6B4A95..0x6B4B95: the parking probe (every 16 frames, offset by the random seed byte +0x20).
fn park_probe(this: GPtr<u8>) {
    let al = rd::<u8>(this, 0x42C);
    if al & 8 == 0 || al & 4 != 0 || rd::<u8>(this, 0x4A4) == 2 || rd::<u8>(this, 0x3BA) != 1 {
        return;
    }
    let fc = g::<u32>(0xB7_CB4C).read();
    if (u32::from(rd::<u8>(this, 0x20)).wrapping_add(fc)) & 0xF != 0 {
        return;
    }
    let model = rd::<u16>(this, 0x22);
    if model == 0x1A4 || model == 0x1B6 {
        return;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let pos = pos_ptr(this);
    let mut a = CVector::default();
    let ap = vector_mul_scalar(GPtr::from_raw(&mut a).cast(), m, 3.0);
    let mut b = CVector::default();
    let bp = vector_mul_scalar(GPtr::from_raw(&mut b).cast(), m.byte_add(0x10), 10.0);
    let mut cc = CVector::default();
    let cp = vector_add(GPtr::from_raw(&mut cc).cast(), pos, bp);
    let mut target = CVector::default();
    let tp = GPtr::from_raw(&mut target).cast::<u8>();
    vector_add(tp, cp, ap);
    // L2c: the colpoint, La4: the hit entity
    let mut colpoint = [0u32; 0x2C / 4];
    let mut hit = GPtr::<u8>::from_addr(0);
    let ok = process_line_of_sight(
        pos,
        tp,
        GPtr::from_raw(colpoint.as_mut_ptr()).cast(),
        GPtr::from_raw(&mut hit).cast(),
        1,
        1,
        1,
        0,
        0,
        0,
        0,
        0,
    );
    if ok != 0 && hit != this {
        return;
    }
    get_car_to_park_at_coors(this, tp);
}

/// 0x6B4C45..0x6B51C9: player / remote-controlled input processing.
fn player(this: GPtr<u8>, col: GPtr<u8>, flags: GPtr<u32>, remote: bool) -> u8 {
    if !remote {
        let d = rd::<GPtr<u8>>(this, 0x460);
        if d.is_null() || matches!(rd::<u32>(d, 0x530), 0x3D | 0x3A | 0x3F) {
            return 0;
        }
    }
    let d = rd::<GPtr<u8>>(this, 0x460);
    let pad = if d.is_null() {
        GPtr::from_addr(0)
    } else {
        get_pad_from_player(d)
    };
    prune_references(this);
    if remote {
        let slot = find_player_slot_with_remote_vehicle_pointer(this);
        if slot >= 0 {
            v_process_control_inputs(this, slot as u32);
        }
    } else {
        let n = rd::<u32>(rd::<GPtr<u8>>(this, 0x460), 0x598);
        if n == 0 || n == 1 {
            v_process_control_inputs(this, n);
        }
    }
    let sub = rd::<u32>(this, 0x594);
    let model = rd::<u16>(this, 0x22);
    if rd::<u8>(this, 0x36) & 0xF8 == 0
        && sub != 3
        && (model == 0x21B || sub != 4)
        && model != 0x259
        && model != 0x1B0
    {
        do_drive_by_shootings(this);
    }

    // the centre-of-mass lean in the air
    let t = player_field(0xB7_CE98).read();
    let h = rd::<GPtr<u8>>(this, 0x384);
    let sub = rd::<u32>(this, 0x594);
    if t > 500 && sub != 3 && sub != 4 {
        let v = (t - 500).min(1000);
        let mut lean = xi(v as i32).fmul_m(f32::from_bits(0x3B03_126F)).to_f32();
        let m = rd::<GPtr<u8>>(this, 0x14);
        if !le(ld(m, 0x28), c(0xBECC_CCCD)) && !pad.is_null() {
            if le(ld(m, 8), X87::zero()) {
                lean = x(lean).fmul_m(-1.0).to_f32();
            }
            let lr = get_steering_left_right(pad) as u16 as i16;
            let h = rd::<GPtr<u8>>(this, 0x384);
            let z = xi(i32::from(lr))
                .fmul_m(f32::from_bits(0x3C00_0000))
                .fmul_m(rd::<f32>(col, 0x14))
                .fmul_m(g::<f32>(0x8D_3138).read())
                .fmul_m(lean)
                .fadd_m(rd::<f32>(h, 0x1C));
            wr::<f32>(this, 0xAC, z.to_f32());
        } else {
            let z = x(g::<f32>(0x8D_3138).read())
                .fmul_m(rd::<f32>(col, 0x14))
                .fmul_m(0.5)
                .fsubr_m(rd::<f32>(h, 0x1C));
            wr::<f32>(this, 0xAC, z.to_f32());
        }
    } else {
        wr::<u32>(this, 0xAC, rd::<u32>(h, 0x1C));
    }

    let sub = rd::<u32>(this, 0x594);
    if rd::<u8>(this, 0x960) == 0 && !pad.is_null() && sub != 3 && sub != 4 {
        air_control(this, pad, sub);
    }
    do_soft_ground_resistance(this, flags);
    0
}

/// 0x6B4E42..0x6B51AE: rotation control while no wheel touches the ground.
fn air_control(this: GPtr<u8>, pad: GPtr<u8>, sub: u32) {
    // La0: the control strength
    let base = if sub == 1 {
        f32::from_bits(0x3B23_D70A)
    } else {
        st0(get_fat_and_muscle_modifier(0xE))
            .fmul_m(f32::from_bits(0x3A37_8034))
            .to_f32()
    };
    let r = c(0x453B_8000).fdiv_m(rd::<f32>(this, 0x90)).to_f32();
    let strength = st0(min_f(1.0, r))
        .fmul_m(base)
        .fmul_m(rd::<f32>(this, 0x90))
        .to_f32();
    // L9c: left/right, La4: up/down
    let lr = get_steering_left_right(pad) as u16 as i16;
    let mut lr = xi(i32::from(lr)).fmul_m(f32::from_bits(0x3C00_0000)).to_f32();
    let ud = get_steering_up_down(pad) as u16 as i16;
    let mut ud = xi(i32::from(ud)).fmul_m(f32::from_bits(0x3C00_0000)).to_f32();
    let small = c(0x3D4C_CCCD);
    if g::<u8>(0xB6_EC2E).read() != 0 && !ge(x(lr).fabs(), small) && !ge(x(ud).fabs(), small) {
        let clamp = |addr: usize| {
            let v = x(g::<f32>(addr).read())
                .fmul_m(f32::from_bits(0x3CA3_D70A))
                .to_f32();
            let v = st0(max_f(-1.5, v)).to_f32();
            st0(min_f(1.5, v)).to_f32()
        };
        lr = clamp(0xB7_3424);
        ud = clamp(0xB7_3428);
    }
    let ts = g::<f32>(0x0B7_CB5C).read();
    let lim = c(0x3CA3_D70A);
    // the shared turn-speed test: (d < 0.02 && axis < 0) || (d > -0.02 && axis > 0)
    let wants = |d: X87, axis: f32| {
        if !ge(d, lim) && !ge(x(axis), X87::zero()) {
            return true;
        }
        !le(d, lim.fchs()) && !le(x(axis), X87::zero())
    };
    let turn = |this: GPtr<u8>, point_axis: isize, force_axis: isize, axis: f32| {
        let m = rd::<GPtr<u8>>(this, 0x14);
        let mut pc = CVector::default();
        let point = rv(
            vector_add(
                GPtr::from_raw(&mut pc).cast(),
                this.byte_add(0xA4),
                m.byte_add(point_axis),
            ),
            0,
        );
        let s = x(ts).fmul_m(axis).fmul_m(strength).to_f32();
        let mut fc = CVector::default();
        let force = rv(
            scalar_mul_vector(GPtr::from_raw(&mut fc).cast(), s, m.byte_add(force_axis)),
            0,
        );
        apply_turn_force(this, force, point);
    };
    if get_hand_brake(pad) & 0xFFFF != 0 {
        let m = rd::<GPtr<u8>>(this, 0x14);
        let d = st0(dot_product(this.byte_add(0x50), m.byte_add(0x20)));
        if wants(d, lr) {
            turn(this, 0x10, 0, lr);
        }
    } else if get_accelerate(pad) & 0xFFFF == 0 {
        let m = rd::<GPtr<u8>>(this, 0x14);
        let d = st0(dot_product(this.byte_add(0x50), m.byte_add(0x10)));
        if wants(d, lr) {
            turn(this, 0x20, 0, lr);
        }
    }
    if get_accelerate(pad) & 0xFFFF != 0 {
        return;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let d = st0(dot_product(this.byte_add(0x50), m));
    if wants(d, ud) {
        turn(this, 0x10, 0x20, ud);
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_AUTOMOBILE_PROCESS_AI];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
