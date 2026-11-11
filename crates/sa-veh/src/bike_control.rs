//! WRITER-VEH-BIG (batch `vehbig-1`): `CBike::ProcessControl`, the per-frame driver of every
//! two-wheeler (bikes, BMX, quad-style subclasses calling it). Ported from the 1.0 US bytes
//! (gta-reversed has no body for it), block by block with the x87 arithmetic through `X87`.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;
use sa_world::world::common::find_player_ped;

use crate::common::{ftol32, g, rd, wr, x, xi};

orig! {
    /// `CEntity::GetColModel()` (0x535300)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => sa_world::world::entity::get_col_model;
    /// `CAEVehicleAudioEntity::Service()` (0x502280)
    fn audio_service(a: GPtr<u8>) = thiscall 0x502280 => unported;
    /// `CVehicle::ProcessCarAlarm()` (0x6D21F0)
    fn process_car_alarm(v: GPtr<u8>) = thiscall 0x6D21F0 => crate::vehicle2::process_car_alarm;
    /// `CVehicle::ActivateBombWhenEntered()` (0x6D2570)
    fn activate_bomb_when_entered(v: GPtr<u8>) = thiscall 0x6D2570 => crate::vehicle2::activate_bomb_when_entered;
    /// `CVehicle::UpdateClumpAlpha()` (0x6D2980)
    fn update_clump_alpha(v: GPtr<u8>) = thiscall 0x6D2980 => crate::vehicle2::update_clump_alpha;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `Multiply3x3(CVector*, const CVector&, const CMatrix&)` (0x59C810): the transposed product
    fn multiply3x3_transposed(out: GPtr<u8>, v: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C810 => sa_world::world::mvpa_math::multiply3x3_transposed;
    /// `CStats::GetFatAndMuscleModifier(eStatModAbilities)` (0x559AF0), ST0
    fn get_fat_and_muscle_modifier(ability: u32) -> f64 = cdecl 0x559AF0 => unported;
    /// `Multiply3x3(CVector*, const CMatrix*, const CVector*)` (0x59C790)
    fn multiply3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => sa_world::world::mvpa_math::multiply3x3;
    /// `CPhysical::ApplyTurnForce(CVector, CVector)` (0x542A50)
    fn apply_turn_force(e: GPtr<u8>, force: CVector, point: CVector) = thiscall 0x542A50 => sa_world::physical::forces::apply_turn_force;
    /// `VectorAdd(CVector*, const CVector*, const CVector*)` (0x40FE30)
    fn vector_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => sa_world::world::mvpa_math::vector_add;
    /// `operator/(const CVector&, float)` (0x4119D0, hidden return pointer)
    fn vector_div_scalar(out: GPtr<u8>, v: GPtr<u8>, s: f32) -> GPtr<u8> = cdecl 0x4119D0 => sa_world::world::mvpa_math::vector_div_scalar;
    /// `CVector::SquaredMagnitude()` (0x406DA0), ST0
    fn vector_magnitude_sqr(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `FUN_00424100(float x, float y)` (0x424100): `xor al, al; ret`
    fn area_check_stub(px: u32, py: u32) -> u8 = cdecl 0x424100 => crate::train_control::area_check_stub_native;
    /// `CPhysical::SkipPhysics()` (0x5433B0)
    fn skip_physics(v: GPtr<u8>) = thiscall 0x5433B0 => sa_world::physical::base::skip_physics;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0)
    fn get_association(clump: GPtr<u8>, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CCullZones::DoExtraAirResistanceForPlayer()` (0x72DDD0)
    fn do_extra_air_resistance_for_player() -> u8 = cdecl 0x72DDD0 => unported;
    /// `DotProduct(const RwV3d*, const RwV3d*)` (0x40FDB0), ST0
    fn dot_product(a: GPtr<u8>, b: GPtr<u8>) -> f64 = cdecl 0x40FDB0 => crate::automobile_ai::dot_native;
    /// `operator*(float, const CVector&)` (0x40FE90, cdecl, hidden return pointer)
    fn scalar_mul_vector(out: GPtr<u8>, s: f32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => sa_world::world::mvpa_math::scalar_mul_vector;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0)
    fn apply_move_force(e: GPtr<u8>, force: CVector) = thiscall 0x5429F0 => sa_world::physical::forces::apply_move_force;
    /// `CPhysical::ProcessControl()` (0x5485E0)
    fn physical_process_control(v: GPtr<u8>) = thiscall 0x5485E0 => sa_world::physical::attach::process_control;
    /// `CBike::ProcessBuoyancy()` (0x6B5FB0)
    fn bike_process_buoyancy(v: GPtr<u8>) = thiscall 0x6B5FB0 => crate::mvpd_v3::bike_process_buoyancy;
    /// `CPlayerPed::ResetPlayerBreath()` (0x60A8A0)
    fn reset_player_breath(p: GPtr<u8>) = thiscall 0x60A8A0 => unported;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => crate::train_control::normalise_native;
    /// `CPhysical::ApplySpringCollisionAlt(float, CVector&, CVector&, float, float, CVector&, float&)` (0x543D60)
    fn apply_spring_collision_alt(e: GPtr<u8>, level: f32, dir: GPtr<u8>, point: GPtr<u8>, len: f32, bias: f32, normal: GPtr<u8>, out: GPtr<f32>) -> u8 = thiscall 0x543D60 => sa_world::physical::spring::apply_spring_collision_alt;
    /// `CPhysical::ApplySpringCollision(float, CVector&, CVector&, float, float, float&)` (0x543C90)
    fn apply_spring_collision(e: GPtr<u8>, level: f32, dir: GPtr<u8>, point: GPtr<u8>, len: f32, bias: f32, out: GPtr<f32>) -> u8 = thiscall 0x543C90 => sa_world::physical::spring::apply_spring_collision;
    /// `SurfaceInfos_c::GetSkidmarkType(uint32)` (0x55E630)
    fn get_skidmark_type(si: GPtr<u8>, id: u32) -> u32 = thiscall 0x55E630 => unported;
    /// `CPhysical::GetSpeed(CVector)` (0x542CE0)
    fn get_speed(e: GPtr<u8>, out: GPtr<CVector>, point: CVector) -> GPtr<CVector> = thiscall 0x542CE0 => sa_world::physical::forces::get_speed;
    /// `CPhysical::ApplySpringDampening(float, float, CVector&, CVector&, CVector&)` (0x543E90)
    fn apply_spring_dampening(e: GPtr<u8>, damping: f32, force: f32, dir: GPtr<u8>, point: GPtr<u8>, speed: GPtr<u8>) -> u8 = thiscall 0x543E90 => sa_world::physical::spring::apply_spring_dampening;
    /// `cTransmission::CalculateDriveAcceleration(...)` (0x6D05E0), ST0
    fn calculate_drive_acceleration(t: GPtr<u8>, gas: GPtr<u8>, gear: GPtr<u8>, change: GPtr<u8>, velocity: GPtr<u8>, i1: GPtr<u8>, i2: GPtr<u8>, wheels: u32, cheat: u32) -> f64 = thiscall 0x6D05E0 => unported;
    /// `SurfaceInfos_c::GetAdhesiveLimit(CColPoint&)` (0x55EB50), ST0
    fn get_adhesive_limit(si: GPtr<u8>, cp: GPtr<u8>) -> f64 = thiscall 0x55EB50 => unported;
    /// `SurfaceInfos_c::GetAdhesionGroup(uint32)` (0x55E5C0)
    fn get_adhesion_group(si: GPtr<u8>, id: u32) -> u32 = thiscall 0x55E5C0 => unported;
    /// `CrossProduct(CVector*, const CVector*, const CVector*)` (0x59C730)
    fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => crate::train_control::cross_native;
    /// `SurfaceInfos_c::GetWetMultiplier(uint32)` (0x55E600), ST0
    fn get_wet_multiplier(si: GPtr<u8>, id: u32) -> f64 = thiscall 0x55E600 => unported;
    /// `CVehicle::ProcessBikeWheel(...)` (0x6D73B0)
    fn process_bike_wheel(v: GPtr<u8>, fwd: GPtr<u8>, right: GPtr<u8>, speed: GPtr<u8>, point: GPtr<u8>, wheels: i32, thrust: f32, brake: f32, adhesion: f32, destab: f32, wheel: u32, wheel_speed: GPtr<u8>, state: GPtr<i32>, special: u32, status: u32) = thiscall 0x6D73B0 => crate::mvpd_v1::vehicle_process_bike_wheel;
    /// `CVehicle::ProcessSirenAndHorn(bool)` (0x6E0950)
    fn process_siren_and_horn(v: GPtr<u8>, check: u32) = thiscall 0x6E0950 => crate::vehicle2::process_siren_and_horn;
    /// `FxManager_c::CreateFxSystem(const char*, RwV3d*, RwMatrix*, bool)` (0x4A9BE0) on `g_fxMan` (0xA9AE80)
    fn create_fx_system(mgr: GPtr<u8>, name: u32, pos: GPtr<u8>, m: GPtr<u8>, ignore_bbox: u32) -> GPtr<u8> = thiscall 0x4A9BE0 => unported;
    /// `FxSystem_c::Play()` (0x4AA2F0)
    fn fx_play(fx: GPtr<u8>) = thiscall 0x4AA2F0 => unported;
    /// `CEventVehicleOnFire::CEventVehicleOnFire(CVehicle*)` (0x4B10C0)
    fn event_vehicle_on_fire_ctor(ev: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B10C0 => unported;
    /// `GetEventGlobalGroup()` (0x4ABA50)
    fn get_event_global_group() -> GPtr<u8> = cdecl 0x4ABA50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(grp: GPtr<u8>, ev: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CEventVehicleOnFire::~CEventVehicleOnFire()` (0x4B1170)
    fn event_vehicle_on_fire_dtor(ev: GPtr<u8>) = thiscall 0x4B1170 => unported;
    /// `FxSystem_c::Kill()` (0x4AA3F0)
    fn fx_kill(fx: GPtr<u8>) = thiscall 0x4AA3F0 => unported;
    /// `CVehicle::ProcessDelayedExplosion()` (0x6D1340)
    fn process_delayed_explosion(v: GPtr<u8>) = thiscall 0x6D1340 => crate::vehicle2::process_delayed_explosion;
    /// `SurfaceInfos_c::GetRoughness(uint32)` (0x55E810)
    fn get_roughness(si: GPtr<u8>, id: u32) -> u32 = thiscall 0x55E810 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::StartShake(int16, uint8, uint32)` (0x53F920)
    fn start_shake(pad: GPtr<u8>, time: u32, freq: u32, delay: u32) = thiscall 0x53F920 => unported;
    /// `CCarCtrl::ScanForPedDanger(CVehicle*)` (0x42CE40)
    fn scan_for_ped_danger(v: GPtr<u8>) = cdecl 0x42CE40 => sa_pop::carctrl4::scan_for_ped_danger;
    /// `CBike::ProcessAI(uint32&)` (vtable +0x108)
    fn v_process_ai(v: GPtr<u8>, flags: GPtr<u32>) -> u8 = thiscall vslot 0x108 => unported;
    /// `CBike::VehicleDamage(...)` (vtable +0xE0)
    fn v_vehicle_damage(v: GPtr<u8>, dmg: u32, comp: u32, e: u32, p: u32, d: u32, weapon: u32) = thiscall vslot 0xE0 => unported;
    /// `CBike::ProcessControlCollisionCheck(bool)` (vtable +0x5C)
    fn v_process_control_collision_check(v: GPtr<u8>, apply: u32) = thiscall vslot 0x5C => unported;
    /// `CBike::BlowUpCar(CEntity*, bool)` (vtable +0xA4)
    fn v_blow_up_car(v: GPtr<u8>, culprit: GPtr<u8>, in_cutscene: u32) = thiscall vslot 0xA4 => unported;
}

/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7_CB5C;
const SURFACE_INFOS: usize = 0xB7_9538;
const F1: u32 = 0x3F80_0000;
/// The front / rear wheel state statics `ProcessBikeWheel` updates (0xC1C26C, 0xC1C270) and the
/// thrust static (0xC1C27C).
const FRONT_STATE: usize = 0xC1_C26C;
const REAR_STATE: usize = 0xC1_C270;
const THRUST: usize = 0xC1_C27C;

fn rv(p: GPtr<u8>, off: isize) -> CVector {
    CVector::new(rd::<f32>(p, off), rd::<f32>(p, off + 4), rd::<f32>(p, off + 8))
}

fn wv(p: GPtr<u8>, off: isize, v: CVector) {
    wr::<f32>(p, off, v.x);
    wr::<f32>(p, off + 4, v.y);
    wr::<f32>(p, off + 8, v.z);
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

fn ts() -> f32 {
    g::<f32>(TIME_STEP).read()
}

fn gf(addr: usize) -> X87 {
    x(g::<f32>(addr).read())
}

fn pow(base: X87, e: X87) -> X87 {
    st0(sa_core::orig::x87_binary(0x82_2130, base.to_f64(), e.to_f64()))
}

fn asin(v: f32) -> X87 {
    st0(sa_core::orig::x87_unary(0x82_1E70, x(v).to_f64()))
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

fn mat(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0x14)
}

fn gp<T>(v: &mut T) -> GPtr<u8> {
    GPtr::from_raw(v).cast()
}

/// The frame locals shared by the blocks of `ProcessControl` (named after their frame slots).
struct Frame {
    /// L11c: the extra handling flags `ProcessAI` fills
    extra: u32,
    /// L124: the collision data
    col_data: GPtr<u8>,
    /// L138: the model info
    mi: GPtr<u8>,
    /// L139: physics skipped
    skip: bool,
    /// L171: hit from the side while slow (or +0x614 bit 3): ground normal forced up
    upright: bool,
    /// La8..: contact points relative to the position, Ld8..: spring directions, L78..: contact
    /// speeds, L48..: spring forces
    points: [CVector; 4],
    dirs: [CVector; 4],
    speeds: [CVector; 4],
    springs: [f32; 4],
    /// L148..L140: the move speed before the wheels (later the world centre-of-mass offset)
    old_speed: CVector,
}

/// `CBike::ProcessControl()` (0x6B9250).
///
/// Clears the frame flags (+0x429 bit 5, +0x5C8, +0x7A8, +0x614 bit 5, +0x42C bit 1); nothing more
/// while 0xA43088 == 1. Alarm, bomb, clump alpha, the player-on-board counter +0x7B6, the virtual
/// `ProcessAI(&flags)` (its result ignored); simple (status 2) bikes stop here. The leaning lock
/// (+0x614 bit 4) releases when tilted. Unless flag 2 or +0x614 bits 3/4, the centre of mass
/// follows the handling; else the stability torques damp the local turn speed (`pow` of the bike
/// damping, stronger with the wheels in the air for the player's fat/muscle stat). Abandoned /
/// wrecked bikes run the fake-physics counter. Wheel entities in a safe position postpone the bike
/// (only with `CWorld::bForceProcessControl`). `VehicleDamage(0, .., 0x31)`. Then either
/// `SkipPhysics` with the lean decay, or the collision check, the steer drift towards the roll,
/// the pedalling push of a player's BMX-style ride (animation 0xC7), `CPhysical::ProcessControl`,
/// buoyancy, breath reset, the suspension (springs and dampening, two contact-speed passes),
/// skid mark types, drive acceleration, the ground normal +0x618, the steer limit / lean target
/// +0x644, both wheels (`ProcessBikeWheel`, rear first with handling flag 23, wheelie torques with
/// +0x7A8 / +0x7AC), the lean angle +0x64C / +0x648, siren / horn. Finally: fire (the "fire_bike"
/// effect, the on-fire event, blowing up after 5 s), delayed explosion, suspension and roughness
/// pad shake, the danger scan, frozen / resting speeds, the ground alignment torques and the
/// player's wheelie / stoppie control.
///
/// Kept from the bytes: `CalculateDriveAcceleration` gets null inertia pointers; the rear wheel
/// point mixes line 3's y with line 2's z; the front wheel rotation in the air of the
/// front-first order does not scale by the time step; the burnout torque passes the rear contact
/// point as the force and the scaled right vector as the point.
#[hook(addr = 0x6b9250, conv = thiscall, name = "CBike::ProcessControl", this = "CBike",
       expect = "6A FF 68 EB 82 84 00",
       route = "split", hazards = "x87-transcendental,heap-writes,uninit-locals",
       recs = "driving",
       note = "pow/asin through orig::x87_binary/x87_unary; wheel status / bool dwords zero-extended (the original pushes register residue above the byte); the on-fire event lives on the Rust stack (0x30 bytes)")]
pub fn bike_process_control(this: GPtr<u8>) {
    let col = get_col_model(this);
    let mut f = Frame {
        extra: 0,
        col_data: rd::<GPtr<u8>>(col, 0x2C),
        mi: crate::common::model_info(i32::from(rd::<i16>(this, 0x22))),
        skip: false,
        upright: false,
        points: [CVector::default(); 4],
        dirs: [CVector::default(); 4],
        speeds: [CVector::default(); 4],
        springs: [0.0; 4],
        old_speed: CVector::default(),
    };
    audio_service(this.byte_add(0x138));
    wr::<u8>(this, 0x429, rd::<u8>(this, 0x429) & 0xDF);
    wr::<u8>(this, 0x5C8, 0);
    wr::<u16>(this, 0x7A8, 0);
    wr::<u8>(this, 0x614, rd::<u8>(this, 0x614) & 0xDF);
    wr::<u8>(this, 0x42C, rd::<u8>(this, 0x42C) & 0xFD);
    if g::<u8>(0xA4_3088).read() == 1 {
        return;
    }
    process_car_alarm(this);
    activate_bomb_when_entered(this);
    update_clump_alpha(this);
    let player_aboard = {
        let d = rd::<GPtr<u8>>(this, 0x460);
        if d.is_null() {
            false
        } else if ped_is_player(d) != 0 {
            true
        } else {
            let p = rd::<GPtr<u8>>(this, 0x464);
            !p.is_null() && ped_is_player(p) != 0
        }
    };
    if player_aboard {
        match rd::<u8>(this, 0x7B6) {
            1 => wr::<u8>(this, 0x7B6, 2),
            0 => wr::<u8>(this, 0x7B6, 1),
            _ => {}
        }
    } else {
        wr::<u8>(this, 0x7B6, 0);
    }
    v_process_ai(this, GPtr::from_raw(&mut f.extra));
    if rd::<u8>(this, 0x36) >> 3 == 2 {
        return;
    }
    let dl = rd::<u8>(this, 0x614);
    if dl & 0x10 != 0 {
        let m = mat(this);
        if !le(ld(m, 8).fabs(), c(0x3EB3_3333)) || !le(ld(m, 0x18).fabs(), c(0x3F00_0000)) {
            wr::<u8>(this, 0x614, dl & 0xEF);
        }
    }
    if f.extra & 2 == 0 && rd::<u8>(this, 0x614) & 0x18 == 0 {
        let h = rd::<GPtr<u8>>(this, 0x384);
        wr::<u32>(this, 0xA4, rd::<u32>(h, 0x14));
        wr::<u32>(this, 0xA8, rd::<u32>(h, 0x18));
        wr::<u32>(this, 0xAC, rd::<u32>(rd::<GPtr<u8>>(this, 0x63C), 0x28));
    } else {
        stability(this);
        if rd::<u8>(this, 0x36) >> 3 != 0 {
            let h = rd::<GPtr<u8>>(this, 0x384);
            for k in 0..3isize {
                wr::<u32>(this, 0xA4 + 4 * k, rd::<u32>(h, 0x14 + 4 * k));
            }
        }
    }
    let st = rd::<u8>(this, 0x36) >> 3;
    if (rd::<u32>(this, 0x1C) >> 4) & 1 == 0 && (st == 4 || st == 5) && rd::<u8>(this, 0x614) & 8 == 0 {
        fake_physics(this, st, &mut f);
    }
    for i in 0..4isize {
        let e = rd::<GPtr<u8>>(this, 0x7C0 + 4 * i);
        if !e.is_null() {
            wr::<u8>(this, 0x42C, rd::<u8>(this, 0x42C) | 2);
            if g::<u8>(0xB7_CD6E).read() != 0 && (rd::<u32>(e, 0x1C) >> 5) & 1 != 0 {
                wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 0x40);
                return;
            }
        }
    }
    if rd::<u8>(this, 0x42C) & 2 != 0 {
        f.skip = false;
        wr::<u8>(this, 0xB8, 0);
    }
    v_vehicle_damage(this, 0, 0, 0, 0, 0, 0x31);
    f.upright = false;
    let hit_side = 'h: {
        if le(ld(this, 0xD8), X87::zero()) {
            break 'h false;
        }
        let m = mat(this);
        let d = ld(this, 0xE8)
            .fmul_m(rd::<f32>(m, 8))
            .fadd(ld(this, 0xE4).fmul_m(rd::<f32>(m, 4)))
            .fadd(ld(this, 0xE0).fmul_m(rd::<f32>(m, 0)));
        if le(d.fabs(), c(0x3F00_0000)) {
            break 'h false;
        }
        !ge(sq3(this, 0x44), c(0x3DCC_CCCD))
    };
    if hit_side || rd::<u8>(this, 0x614) & 8 != 0 {
        f.upright = true;
    }
    if f.skip {
        skipped(this);
    } else {
        physics(this, &mut f);
    }
    after_physics(this, &mut f);
}

/// 0x6B93DA..0x6B97A6: the stability torques (local turn speed damping around right and forward).
fn stability(this: GPtr<u8>) {
    // L158: forward damping, L15c: roll damping, L164: the speed factor, L154 (unused here)
    let l158 = g::<f32>(0x8D_323C).read();
    let mut l15c = g::<f32>(0x8D_3238).read();
    let mut l164 = 1.0f32;
    let mut local = CVector::default();
    multiply3x3_transposed(gp(&mut local), this.byte_add(0x50), mat(this));
    let damp = if rd::<u8>(this, 0x36) & 0xF8 != 0 {
        x(l15c)
    } else if !x(rd::<f32>(this, 0x710)).fcom(c(F1)).test_any(1)
        && !x(rd::<f32>(this, 0x714)).fcom(c(F1)).test_any(1)
    {
        // both front compressions at rest: the wheels are in the air
        l164 = f32::from_bits(0x42C8_0000);
        let l188 = st0(get_fat_and_muscle_modifier(0xD))
            .fmul_m(f32::from_bits(0x3E4C_CCCD))
            .to_f32();
        let rear_down = !ge(ld(this, 0x718), c(F1)) || !ge(ld(this, 0x71C), c(F1));
        let m = mat(this);
        if rear_down && !le(ld(m, 0x18), X87::zero()) {
            let bh = rd::<GPtr<u8>>(this, 0x63C);
            let t = ld(bh, 0x2C).fsub_m(rd::<f32>(m, 0x18)).fabs().fmul_m(l188);
            if !le(t, c(0x3D4C_CCCD)) {
                c(0x3D4C_CCCD).fsubr_m(l15c)
            } else {
                ld(bh, 0x2C)
                    .fsub_m(rd::<f32>(m, 0x18))
                    .fabs()
                    .fmul_m(l188)
                    .fsubr_m(l15c)
            }
        } else {
            c(0x3F7A_E148)
        }
    } else if ld(this, 0x738).fcom(X87::zero()).test_parity(0x41)
        || ld(this, 0x73C).fcom(X87::zero()).test_parity(0x41)
    {
        // test ah, 0x41; jp: a positive rear wheel counter keeps the plain damping
        x(l15c)
    } else {
        l164 = f32::from_bits(0x42C8_0000);
        let l188 = st0(get_fat_and_muscle_modifier(0xD))
            .fmul_m(f32::from_bits(0x3D99_999A))
            .to_f32();
        let l14c = st0(get_fat_and_muscle_modifier(0xD)).fmul_m(0.25).to_f32();
        let m = mat(this);
        if ge(ld(m, 0x18), X87::zero()) {
            x(l15c)
        } else {
            let bh = rd::<GPtr<u8>>(this, 0x63C);
            let t = ld(bh, 0x30).fsub_m(rd::<f32>(m, 0x18)).fabs().fmul_m(l14c);
            if !le(t, x(l188)) {
                x(l188).fadd_m(f32::from_bits(0x3F66_6666)).fmul_m(l15c)
            } else {
                ld(bh, 0x30)
                    .fsub_m(rd::<f32>(m, 0x18))
                    .fabs()
                    .fmul_m(l14c)
                    .fadd_m(f32::from_bits(0x3F66_6666))
                    .fmul_m(l15c)
            }
        }
    };
    l15c = x(local.x)
        .fmul_m(local.x)
        .fmul_m(l164)
        .fadd_m(1.0)
        .fdivr(damp)
        .to_f32();
    let l158 = c(0x447A_0000)
        .fmul_m(local.y)
        .fmul_m(local.y)
        .fadd_m(1.0)
        .fdivr_m(l158)
        .to_f32();
    let p15c = pow(x(l15c), x(ts())).to_f32();
    let p158 = pow(x(l158), x(ts()));
    // L118: the roll-rate change, s2: the pitch-rate change (register)
    let l118 = x(local.x).fmul_m(p15c).fsub_m(local.x).to_f32();
    let s2 = p158.fmul_m(local.y).fsub_m(local.y);
    let m = mat(this);
    let tm = rd::<f32>(this, 0x90);
    let minus1 = f32::from_bits(0xBF80_0000);
    let (ux, uy, uz) = (
        ld(m, 0x20).fmul_m(minus1),
        ld(m, 0x24).fmul_m(minus1),
        ld(m, 0x28).fmul_m(minus1),
    );
    let ax = ux.to_f32();
    let l134 = uz.fmul(s2).to_f32();
    let l10c = uy.fmul(s2).to_f32();
    let l180 = x(ax).fmul(s2).to_f32();
    let force = CVector::new(
        x(l180).fmul_m(tm).to_f32(),
        x(l10c).fmul_m(tm).to_f32(),
        x(l134).fmul_m(tm).to_f32(),
    );
    let mut out = CVector::default();
    let r = multiply3x3(gp(&mut out), m, this.byte_add(0xA4));
    let point = CVector::new(
        ld(r, 0).fadd_m(rd::<f32>(m, 0)).to_f32(),
        ld(r, 4).fadd_m(rd::<f32>(m, 4)).to_f32(),
        ld(r, 8).fadd_m(rd::<f32>(m, 8)).to_f32(),
    );
    apply_turn_force(this, force, point);
    // around forward: L118 times up
    let m = mat(this);
    let tm = rd::<f32>(this, 0x90);
    let z = x(l118).fmul_m(rd::<f32>(m, 0x28));
    let l134 = x(l118).fmul_m(rd::<f32>(m, 0x24)).to_f32();
    let l180 = x(l118).fmul_m(rd::<f32>(m, 0x20)).to_f32();
    let force = CVector::new(
        x(l180).fmul_m(tm).to_f32(),
        x(l134).fmul_m(tm).to_f32(),
        z.fmul_m(tm).to_f32(),
    );
    let mut out = CVector::default();
    let r = multiply3x3(gp(&mut out), m, this.byte_add(0xA4));
    let point = CVector::new(
        ld(r, 0).fadd_m(rd::<f32>(m, 0x10)).to_f32(),
        ld(r, 4).fadd_m(rd::<f32>(m, 0x14)).to_f32(),
        ld(r, 8).fadd_m(rd::<f32>(m, 0x18)).to_f32(),
    );
    apply_turn_force(this, force, point);
}

/// 0x6B97E1..0x6B9A3D: the abandoned / wrecked fake-physics counter.
fn fake_physics(this: GPtr<u8>, st: u8, f: &mut Frame) {
    let idle = rd::<u8>(this, 0x42B) & 1 == 0
        && !(0..3isize).any(|k| ld(this, 0x44 + 4 * k).fcom(X87::zero()).test_parity(0x44))
        && rd::<u32>(this, 0x72C) != F1;
    let (fl, tl, sl) = if st == 5 {
        (0x3BC4_9BA6u32, 0x3AC4_9BA6u32, 0x3C75_C28Fu32)
    } else {
        (0x3B44_9BA6, 0x3A6B_EDFB, 0x3BA3_D70A)
    };
    for (acc, spd) in [(0x74isize, 0x44isize), (0x80, 0x50)] {
        let mut sum = CVector::default();
        let s = vector_add(gp(&mut sum), this.byte_add(acc), this.byte_add(spd));
        let mut half = CVector::default();
        let h = rv(vector_div_scalar(gp(&mut half), s, 2.0), 0);
        wv(this, acc, h);
    }
    let fl = x(f32::from_bits(fl)).fmul_m(ts()).to_f32();
    let rest = !x(fl)
        .fmul_m(fl)
        .fcom(st0(vector_magnitude_sqr(this.byte_add(0x74))))
        .test_any(1)
        && {
            let tl = x(f32::from_bits(tl)).fmul_m(ts()).to_f32();
            !x(tl)
                .fmul_m(tl)
                .fcom(st0(vector_magnitude_sqr(this.byte_add(0x80))))
                .test_any(1)
                && !ge(ld(this, 0xD4), x(f32::from_bits(sl)))
        };
    if !rest && !idle {
        wr::<u8>(this, 0xB8, 0);
        return;
    }
    let n = rd::<u8>(this, 0xB8).wrapping_add(1);
    wr::<u8>(this, 0xB8, n);
    if n <= 10 && !idle {
        return;
    }
    let p = pos_ptr(this);
    if area_check_stub(rd::<u32>(p, 0), rd::<u32>(p, 4)) != 0 {
        return;
    }
    if !idle || rd::<u8>(this, 0xB8) > 10 {
        wr::<u8>(this, 0xB8, 10);
    }
    for off in [0x44isize, 0x48, 0x4C, 0x50, 0x54, 0x58] {
        wr::<u32>(this, off, 0);
    }
    f.skip = true;
}

/// 0x6B9B39..0x6B9C35: physics skipped; the lean recovers.
fn skipped(this: GPtr<u8>) {
    skip_physics(this);
    wr::<u8>(this, 0x42B, rd::<u8>(this, 0x42B) & 0xDE);
    let locked = (rd::<u8>(this, 0x614) >> 4) & 1 != 0;
    if locked && !ge(ld(this, 0x644), c(0x3EB2_B8C3)) {
        wr::<f32>(
            this,
            0x644,
            x(ts())
                .fmul_m(f32::from_bits(0x3C8E_FA35))
                .fadd_m(rd::<f32>(this, 0x644))
                .to_f32(),
        );
    }
    // 0x6B9B88: `test cl, cl; je 0x6BBC18`: only a locked lean recovers here
    if !locked {
        return;
    }
    let p = pow(
        X87::from_f64_exact(f64::from_bits(0x3FEF_0A3D_8000_0000)),
        x(ts()),
    )
    .to_f32();
    let m = mat(this);
    let rz = ld(m, 8);
    let l164 = if !le(rz, c(F1)) {
        1.0
    } else if !le(c(0xBF80_0000), rz) {
        -1.0
    } else {
        rd::<f32>(m, 8)
    };
    let a = asin(l164);
    let v = x(p)
        .fmul_m(rd::<f32>(this, 0x64C))
        .fsub(c(F1).fsub_m(p).fmul(a.fadd_m(f32::from_bits(0x3E86_0A92))));
    wr::<f32>(this, 0x64C, v.to_f32());
    wr::<f32>(this, 0x648, v.to_f32());
}

/// 0x6B9C3A..0x6BBC18: the physics of a running bike.
fn physics(this: GPtr<u8>, f: &mut Frame) {
    if rd::<u8>(this, 0x42B) & 1 == 0 {
        v_process_control_collision_check(this, 1);
    }
    if f.extra & 2 == 0 && rd::<u8>(this, 0x614) & 0x18 == 0 {
        let rz = ld(mat(this), 8);
        let steer = ld(this, 0x494);
        let k = f32::from_bits(0x3C0E_FA35);
        if ge(rz, X87::zero()) {
            if !ge(steer, c(0x3EDF_66F3)) {
                wr::<f32>(
                    this,
                    0x494,
                    x(ts()).fmul_m(k).fadd_m(rd::<f32>(this, 0x494)).to_f32(),
                );
            }
        } else if !le(steer, c(0xBEDF_66F3)) {
            wr::<f32>(
                this,
                0x494,
                x(ts()).fmul_m(k).fsubr_m(rd::<f32>(this, 0x494)).to_f32(),
            );
        }
    }
    // L134: the air resistance to restore
    let l134 = rd::<u32>(this, 0x98);
    pedal_push(this);
    let was_submerged = (rd::<u32>(this, 0x40) >> 8) & 1 != 0;
    physical_process_control(this);
    wr::<u32>(this, 0x98, l134);
    bike_process_buoyancy(this);
    if !was_submerged && rd::<u32>(this, 0x40) & 0x100 != 0 {
        let d = rd::<GPtr<u8>>(this, 0x460);
        if !d.is_null() && ped_is_player(d) != 0 {
            reset_player_breath(rd::<GPtr<u8>>(this, 0x460));
        } else if rd::<u8>(this, 0x488) > 0 {
            let mut i = 0u32;
            loop {
                let p = rd::<GPtr<u8>>(this, 0x464 + 4 * i as isize);
                if !p.is_null() && ped_is_player(p) != 0 {
                    reset_player_breath(rd::<GPtr<u8>>(this, 0x464 + 4 * i as isize));
                }
                i += 1;
                if i >= u32::from(rd::<u8>(this, 0x488)) {
                    break;
                }
            }
        }
    }
    for i in 0..4isize {
        let r = ld(this, 0x770 + 4 * i)
            .fdiv_m(rd::<f32>(this, 0x780 + 4 * i))
            .fsubr_m(1.0);
        let num = ld(this, 0x710 + 4 * i).fsub(r);
        wr::<f32>(this, 0x710 + 4 * i, num.fdiv(c(F1).fsub(r)).to_f32());
    }
    suspension(this, f);
    wheels(this, f);
}

/// 0x6B9CBB..0x6B9DC7: a player's pedalling push (animation 0xC7 in its push window).
fn pedal_push(this: GPtr<u8>) {
    if rd::<u8>(this, 0x36) & 0xF8 != 0 {
        return;
    }
    let d = rd::<GPtr<u8>>(this, 0x460);
    if d.is_null() {
        return;
    }
    let a = get_association(rd::<GPtr<u8>>(d, 0x18), 0xC7);
    if a.is_null()
        || le(ld(a, 0x18), c(0x3F00_0000))
        || le(ld(a, 0x20), c(0x3D75_C28F))
        || ge(ld(a, 0x20), c(0x3E0F_5C29))
    {
        return;
    }
    let k = if do_extra_air_resistance_for_player() != 0 {
        f32::from_bits(0x3F59_999A)
    } else {
        f32::from_bits(0x3F19_999A)
    };
    wr::<f32>(this, 0x98, ld(this, 0x98).fmul_m(k).to_f32());
    if le(ld(this, 0x49C), c(0x3F00_0000)) {
        return;
    }
    let m = mat(this);
    if le(
        st0(dot_product(this.byte_add(0x44), m.byte_add(0x10))),
        c(0x3E80_0000),
    ) {
        return;
    }
    let s = x(ts())
        .fmul_m(rd::<f32>(this, 0x8C))
        .fmul_m(f32::from_bits(0x3AD1_B718))
        .to_f32();
    let mut out = CVector::default();
    let force = rv(scalar_mul_vector(gp(&mut out), s, m.byte_add(0x10)), 0);
    apply_move_force(this, force);
    wr::<u8>(this, 0x614, rd::<u8>(this, 0x614) | 0x20);
}

/// 0x6B9E86..0x6BA5DB: contact points, springs, skid mark types, the normal overrides, dampening
/// and the contact speeds.
fn suspension(this: GPtr<u8>, f: &mut Frame) {
    let one = c(0x3F80_0000);
    let comp = |i: usize| ld(this, 0x710 + 4 * i as isize);
    for i in 0..4usize {
        if ge(comp(i), one) {
            continue;
        }
        let ii = i as isize;
        let p = pos_ptr(this);
        let cp = this.byte_add(0x660 + 0x2C * ii);
        f.points[i] = CVector::new(
            ld(cp, 0).fsub_m(rd::<f32>(p, 0)).to_f32(),
            ld(cp, 4).fsub_m(rd::<f32>(p, 4)).to_f32(),
            ld(cp, 8).fsub_m(rd::<f32>(p, 8)).to_f32(),
        );
        let line = rd::<GPtr<u8>>(f.col_data, 0x10).byte_add(0x20 * ii);
        let mut d = CVector::new(
            ld(line, 0x10).fsub_m(rd::<f32>(line, 0)).to_f32(),
            ld(line, 0x14).fsub_m(rd::<f32>(line, 4)).to_f32(),
            ld(line, 0x18).fsub_m(rd::<f32>(line, 8)).to_f32(),
        );
        let mut out = CVector::default();
        f.dirs[i] = rv(multiply3x3(gp(&mut out), mat(this), gp(&mut d)), 0);
        vector_normalise(gp(&mut f.dirs[i]));
    }
    wr::<u32>(this, 0x748, 0);
    wr::<u32>(this, 0x744, 0);
    wr::<u8>(this, 0x74F, 0);
    wr::<u8>(this, 0x74E, 0);
    for i in 0..4usize {
        let ii = i as isize;
        if ge(comp(i), one) {
            let line = rd::<GPtr<u8>>(f.col_data, 0x10).byte_add(0x20 * ii + 0x10);
            let mut out = CVector::default();
            f.points[i] = rv(multiply3x3(gp(&mut out), mat(this), line), 0);
            continue;
        }
        let h = rd::<GPtr<u8>>(this, 0x384);
        let mut bias = rd::<f32>(h, 0xC0);
        if i == 2 || i == 3 {
            bias = c(F1).fsub_m(bias).to_f32();
        }
        let normal = this.byte_add(0x670 + 0x2C * ii);
        let len = rd::<f32>(this, 0x710 + 4 * ii);
        let level = rd::<f32>(h, 0xAC);
        let out = GPtr::from_raw(&mut f.springs[i]);
        if !le(ld(normal, 8), c(0x3EB3_3333)) {
            apply_spring_collision_alt(
                this,
                level,
                gp(&mut f.dirs[i]),
                gp(&mut f.points[i]),
                len,
                bias,
                normal,
                out,
            );
        } else {
            apply_spring_collision(
                this,
                level,
                gp(&mut f.dirs[i]),
                gp(&mut f.points[i]),
                len,
                bias,
                out,
            );
        }
        let k = usize::from(i >= 2);
        let t = get_skidmark_type(GPtr::from_addr(SURFACE_INFOS), u32::from(rd::<u8>(normal, 0x13)));
        wr::<u32>(this, 0x744 + 4 * k as isize, t);
        if t == 2 {
            wr::<u8>(this, 0x74E + k as isize, 1);
        }
    }
    contact_speeds(this, f);
    // the steep-ground direction overrides (a pair's normal steeper than 0.35 becomes its direction)
    let pick = |this: GPtr<u8>, a: usize, own: isize, other: isize| -> CVector {
        if !ge(ld(this, 0x710 + 4 * a as isize), c(F1)) {
            rv(this, own)
        } else {
            rv(this, other)
        }
    };
    if !ge(comp(0), one) || !ge(comp(1), one) {
        let n = pick(this, 0, 0x670, 0x69C);
        if !le(x(n.z), c(0x3EB3_3333)) {
            f.dirs[0] = CVector::new(
                x(n.x).fchs().to_f32(),
                x(n.y).fchs().to_f32(),
                x(n.z).fchs().to_f32(),
            );
        }
        let n = pick(this, 1, 0x69C, 0x670);
        if !le(x(n.z), c(0x3EB3_3333)) {
            f.dirs[1] = CVector::new(
                x(n.x).fchs().to_f32(),
                x(n.y).fchs().to_f32(),
                x(n.z).fchs().to_f32(),
            );
        }
    }
    if !ge(comp(2), one) || !ge(comp(3), one) {
        let n = pick(this, 2, 0x6C8, 0x6F4);
        if !le(x(n.z), c(0x3EB3_3333)) {
            f.dirs[2] = CVector::new(
                x(n.x).fchs().to_f32(),
                x(n.y).fchs().to_f32(),
                x(n.z).fchs().to_f32(),
            );
        }
        let n = pick(this, 3, 0x6F4, 0x6C8);
        if !le(x(n.z), c(0x3EB3_3333)) {
            f.dirs[3] = CVector::new(
                x(n.x).fchs().to_f32(),
                x(n.y).fchs().to_f32(),
                x(n.z).fchs().to_f32(),
            );
        }
    }
    for i in 0..4usize {
        if ge(comp(i), one) {
            continue;
        }
        let h = rd::<GPtr<u8>>(this, 0x384);
        apply_spring_dampening(
            this,
            rd::<f32>(h, 0xB0),
            f.springs[i],
            gp(&mut f.dirs[i]),
            gp(&mut f.points[i]),
            gp(&mut f.speeds[i]),
        );
    }
    contact_speeds(this, f);
}

/// The contact speed pass (0x6BA120.. / 0x6BA521..): `GetSpeed` at each point minus the wheel
/// entity's speed at its collision position (the entity pointers are kept).
fn contact_speeds(this: GPtr<u8>, f: &mut Frame) {
    for i in 0..4usize {
        let ii = i as isize;
        let mut tmp = CVector::default();
        f.speeds[i] = rv(get_speed(this, GPtr::from_raw(&mut tmp), f.points[i]).cast(), 0);
        let e = rd::<GPtr<u8>>(this, 0x7C0 + 4 * ii);
        if !e.is_null() {
            let mut t2 = CVector::default();
            let s = rv(
                get_speed(e, GPtr::from_raw(&mut t2), rv(this, 0x7D0 + 0xC * ii)).cast(),
                0,
            );
            f.speeds[i] = CVector::new(
                x(f.speeds[i].x).fsub_m(s.x).to_f32(),
                x(f.speeds[i].y).fsub_m(s.y).to_f32(),
                x(f.speeds[i].z).fsub_m(s.z).to_f32(),
            );
        }
    }
}

/// The per-wheel inputs computed before the wheel blocks.
struct WheelInputs {
    /// L114: drive acceleration (later reused)
    accel: f32,
    /// L150: brake
    brake: f32,
    /// L188 / L134: front / rear brake bias, L164 / L10c: front / rear traction bias
    brake_front: f32,
    brake_rear: f32,
    trac_front: f32,
    trac_rear: f32,
    /// L160: traction
    traction: f32,
    /// L14c: the front wheel index, ebx: the rear wheel index
    front: usize,
    rear: usize,
    /// L108.., Lfc..: the front / rear wheel points in world axes
    front_point: CVector,
    rear_point: CVector,
}

/// 0x6BA5E1..0x6BBC18: drive, ground normal, lean target and both wheels.
fn wheels(this: GPtr<u8>, f: &mut Frame) {
    let m = mat(this);
    // L118: speed along forward
    let mut fwd_speed = ld(this, 0x4C)
        .fmul_m(rd::<f32>(m, 0x18))
        .fadd(ld(this, 0x48).fmul_m(rd::<f32>(m, 0x14)))
        .fadd(ld(this, 0x44).fmul_m(rd::<f32>(m, 0x10)))
        .to_f32();
    let cheat = u32::from(g::<u8>(0x96_914C).read());
    let h = rd::<GPtr<u8>>(this, 0x384);
    let a = calculate_drive_acceleration(
        h.byte_add(0x2C),
        this.byte_add(0x49C),
        this.byte_add(0x4B4),
        this.byte_add(0x4B8),
        gp(&mut fwd_speed),
        GPtr::from_addr(0),
        GPtr::from_addr(0),
        u32::from(rd::<u8>(this, 0x805)),
        cheat,
    );
    let accel = st0(a).fdiv_m(rd::<f32>(this, 0x94)).to_f32();
    let h = rd::<GPtr<u8>>(this, 0x384);
    let brake = ld(h, 0x94).fmul_m(rd::<f32>(this, 0x4A0)).fmul_m(ts()).to_f32();
    let st = rd::<u8>(this, 0x36) >> 3;
    let neutral = st != 0 && st != 8 && rd::<u8>(h, 0xD0) & 8 != 0;
    let (bf, br, tf, tr) = if neutral {
        (1.0, 1.0, 1.0, 1.0)
    } else {
        let tfv = ld(h, 0xA8).fadd(ld(h, 0xA8)).to_f32();
        (
            ld(h, 0x98).fadd(ld(h, 0x98)).to_f32(),
            c(F1)
                .fsub_m(rd::<f32>(h, 0x98))
                .fadd(c(F1).fsub_m(rd::<f32>(h, 0x98)))
                .to_f32(),
            tfv,
            c(0x4000_0000).fsub_m(tfv).to_f32(),
        )
    };
    wr::<u8>(this, 0x806, rd::<u8>(this, 0x805));
    wr::<u8>(this, 0x804, 0);
    wr::<u8>(this, 0x805, 0);
    for i in 0..4isize {
        let wc = 0x730 + 4 * i;
        if ge(ld(this, 0x710 + 4 * i), c(F1)) {
            let mut v = ld(this, wc).fsub_m(ts());
            if le(v, X87::zero()) {
                v = X87::zero();
            }
            wr::<f32>(this, wc, v.to_f32());
            if le(v, X87::zero()) {
                continue;
            }
        } else {
            wr::<u32>(this, wc, 0x4080_0000);
        }
        let n = rd::<u8>(this, 0x804).wrapping_add(1);
        wr::<u8>(this, 0x804, n);
        if i == 2 || i == 3 {
            wr::<u8>(this, 0x805, 1);
        }
        let nrm = this.byte_add(0x670 + 0x2C * i);
        if n == 1 {
            wv(this, 0x618, rv(nrm, 0));
        } else {
            wr::<f32>(this, 0x618, ld(this, 0x618).fadd_m(rd::<f32>(nrm, 0)).to_f32());
            wr::<f32>(this, 0x61C, ld(nrm, 4).fadd_m(rd::<f32>(this, 0x61C)).to_f32());
            wr::<f32>(this, 0x620, ld(nrm, 8).fadd_m(rd::<f32>(this, 0x620)).to_f32());
        }
    }
    let n = rd::<u8>(this, 0x804);
    if n > 0 {
        let inv = c(F1).fdiv(xi(i32::from(n))).to_f32();
        let v = CVector::new(
            x(inv).fmul_m(rd::<f32>(this, 0x618)).to_f32(),
            x(inv).fmul_m(rd::<f32>(this, 0x61C)).to_f32(),
            x(inv).fmul_m(rd::<f32>(this, 0x620)).to_f32(),
        );
        wv(this, 0x618, v);
        let m = mat(this);
        let d = ld(m, 0x28)
            .fmul_m(v.z)
            .fadd(ld(m, 0x24).fmul_m(v.y))
            .fadd(ld(m, 0x20).fmul_m(v.x));
        if !ge(d, c(0xBF00_0000)) {
            for k in 0..3isize {
                wr::<f32>(this, 0x618 + 4 * k, ld(this, 0x618 + 4 * k).fmul_m(-1.0).to_f32());
            }
        }
    } else {
        wv(this, 0x618, CVector::new(0.0, 0.0, 1.0));
    }
    // the wheel points: front on line 0, rear on line 2 (z) / line 3 (y), lowered by the radius
    let front = usize::from(ge(ld(this, 0x710), ld(this, 0x714)));
    let lines = rd::<GPtr<u8>>(f.col_data, 0x10);
    let mut fp = CVector::new(
        0.0,
        rd::<f32>(lines, 4),
        ld(this, 0x710 + 4 * front as isize)
            .fmul_m(rd::<f32>(this, 0x770))
            .fsubr_m(rd::<f32>(lines, 8))
            .fsub(ld(f.mi, 0x40).fmul_m(0.5))
            .to_f32(),
    );
    let mut out = CVector::default();
    let front_point = rv(multiply3x3(gp(&mut out), mat(this), gp(&mut fp)), 0);
    let rear = if !ge(ld(this, 0x718), ld(this, 0x71C)) {
        2
    } else {
        3
    };
    let lines = rd::<GPtr<u8>>(f.col_data, 0x10);
    let mut rp = CVector::new(
        0.0,
        rd::<f32>(lines, 0x64),
        ld(this, 0x710 + 4 * rear as isize)
            .fmul_m(rd::<f32>(this, 0x778))
            .fsubr_m(rd::<f32>(lines, 0x48))
            .fsub(ld(f.mi, 0x44).fmul_m(0.5))
            .to_f32(),
    );
    let mut out = CVector::default();
    let rear_point = rv(multiply3x3(gp(&mut out), mat(this), gp(&mut rp)), 0);
    let h = rd::<GPtr<u8>>(this, 0x384);
    let traction = ld(h, 0x28)
        .fmul_m(rd::<f32>(this, 0x794))
        .fmul_m(g::<f32>(0xC1_C818).read())
        .fmul_m(0.25)
        .to_f32();
    let mut wi = WheelInputs {
        accel,
        brake,
        brake_front: bf,
        brake_rear: br,
        trac_front: tf,
        trac_rear: tr,
        traction,
        front,
        rear,
        front_point,
        rear_point,
    };
    lean_target(this, &wi, fwd_speed);
    f.old_speed = rv(this, 0x44);
    let h = rd::<GPtr<u8>>(this, 0x384);
    let rear_first = (rd::<u32>(h, 0xD0) >> 23) & 1 != 0;
    if !rear_first {
        front_wheel(this, f, &mut wi, false);
    }
    rear_wheel(this, f, &mut wi);
    if rd::<u16>(this, 0x7A8) != 0 && rd::<u32>(this, 0x810) == 1 {
        let v = x(ts())
            .fmul_m(f32::from_bits(0x3B03_126F))
            .fsubr_m(rd::<f32>(this, 0x7AC))
            .to_f32();
        wr::<f32>(this, 0x7AC, v);
        if !ge(x(v), X87::zero()) {
            wr::<u32>(this, 0x7AC, 0);
        }
    } else if !ge(ld(this, 0x7AC), c(F1)) {
        wr::<f32>(
            this,
            0x7AC,
            x(ts())
                .fmul_m(f32::from_bits(0x3BA3_D70A))
                .fadd_m(rd::<f32>(this, 0x7AC))
                .to_f32(),
        );
    }
    if rear_first {
        front_wheel(this, f, &mut wi, true);
    }
    for k in 0..4isize {
        wr::<u32>(this, 0x7C0 + 4 * k, 0);
    }
    lean_angle(this, f);
    wr::<u32>(this, 0x80C, g::<u32>(FRONT_STATE).read());
    wr::<u32>(this, 0x810, g::<u32>(REAR_STATE).read());
    if !ge(ld(this, 0x49C), X87::zero()) && rd::<u32>(this, 0x810) == 1 {
        wr::<u32>(this, 0x810, 0);
    }
    if rd::<u8>(this, 0x36) & 0xF8 == 0 {
        process_siren_and_horn(this, 1);
    } else {
        let hc = rd::<u32>(this, 0x514);
        if hc > 0 {
            wr::<u32>(this, 0x514, hc - 1);
        }
    }
}

/// 0x6BA9E5..0x6BAC1C: the lean target +0x644 (the steer limit of the player's bike).
fn lean_target(this: GPtr<u8>, wi: &WheelInputs, fwd_speed: f32) {
    let st = rd::<u8>(this, 0x36) >> 3;
    let flags = rd::<u8>(this, 0x614);
    if st != 0 && flags & 0x10 != 0 && flags & 8 == 0 {
        if !ge(ld(this, 0x644), c(0x3EB2_B8C3)) {
            wr::<f32>(
                this,
                0x644,
                x(ts())
                    .fmul_m(f32::from_bits(0x3CD6_7750))
                    .fadd_m(rd::<f32>(this, 0x644))
                    .to_f32(),
            );
        }
        return;
    }
    let small = c(0x3C23_D70A);
    if !ge(ld(this, 0x44).fabs(), small)
        && !ge(ld(this, 0x48).fabs(), small)
        && !ld(this, 0x494).fcom(X87::zero()).test_parity(0x44)
    {
        let p = pow(
            X87::from_f64_exact(f64::from_bits(0x3FEE_B851_E000_0000)),
            x(ts()),
        );
        wr::<f32>(this, 0x644, p.fmul_m(rd::<f32>(this, 0x644)).to_f32());
        return;
    }
    let one = c(F1);
    let lim = 'l: {
        if le(x(fwd_speed), small) {
            break 'l one;
        }
        if le(ld(this, 0x730), X87::zero()) && le(ld(this, 0x734), X87::zero()) {
            break 'l one;
        }
        if st != 0 {
            break 'l one;
        }
        let mut cp = [0u8; 0x2C];
        cp[0x20] = 0x3C;
        cp[0x23] = 1;
        let si = GPtr::from_addr(SURFACE_INFOS);
        let bh = rd::<GPtr<u8>>(this, 0x63C);
        let l184 = st0(get_adhesive_limit(si, GPtr::from_raw(cp.as_mut_ptr())))
            .fmul_m(rd::<f32>(bh, 0x20))
            .fmul_m(wi.traction)
            .fmul_m(4.0)
            .to_f32();
        let surf = u32::from(rd::<u8>(this, 0x683 + 0x2C * wi.rear as isize));
        let gr = get_adhesion_group(si, surf);
        let a = if gr == 3 || get_adhesion_group(si, surf) == 4 {
            x(l184).fmul_m(rd::<f32>(rd::<GPtr<u8>>(this, 0x63C), 0x24))
        } else {
            x(l184)
        };
        let q = a.fdiv(x(fwd_speed).fmul_m(fwd_speed));
        let l120 = if ge(one, q) { q.to_f32() } else { 1.0 };
        let h = rd::<GPtr<u8>>(this, 0x384);
        let mut v = asin(l120).fdiv(ld(h, 0xA0).fmul_m(f32::from_bits(0x3C8E_FA35)));
        let steer = ld(this, 0x494);
        let dbl = if !ge(steer, X87::zero()) && !ge(ld(this, 0x648), X87::zero()) {
            true
        } else {
            !le(steer, X87::zero()) && !le(ld(this, 0x648), X87::zero())
        };
        if dbl {
            v = v.fadd(v);
        }
        if ge(one, v) { v } else { one }
    };
    let lim = if rd::<u8>(this, 0x36) & 0xF8 != 0 {
        one
    } else {
        lim
    };
    wr::<f32>(this, 0x644, lim.fmul_m(rd::<f32>(this, 0x494)).to_f32());
}

/// The forward vector projected onto the ground plane of `normal`, normalised, and the right
/// vector `forward x normal`, normalised (0x6BACEB.. and the copies).
fn wheel_axes(this: GPtr<u8>, mut fwd: CVector, normal: GPtr<u8>, upright: bool) -> (CVector, CVector) {
    let d = x(fwd.y)
        .fmul_m(rd::<f32>(normal, 4))
        .fadd(x(fwd.z).fmul_m(rd::<f32>(normal, 8)))
        .fadd(x(fwd.x).fmul_m(rd::<f32>(normal, 0)));
    let (dz, dy, dx) = (
        d.fmul_m(rd::<f32>(normal, 8)),
        d.fmul_m(rd::<f32>(normal, 4)),
        d.fmul_m(rd::<f32>(normal, 0)),
    );
    fwd.x = x(fwd.x).fsub(dx).to_f32();
    fwd.y = x(fwd.y).fsub(dy).to_f32();
    fwd.z = x(fwd.z).fsub(dz).to_f32();
    vector_normalise(gp(&mut fwd));
    let mut out = CVector::default();
    let mut right = rv(cross_product(gp(&mut out), gp(&mut fwd), normal), 0);
    vector_normalise(gp(&mut right));
    if upright {
        right.z = 0.0;
    }
    let _ = this;
    (fwd, right)
}

/// The surface adhesion of a wheel (`GetAdhesiveLimit * mult`, the adhesion-group destabilisation
/// with +0x7B0, wet multiplier for the player, burst-tyre 0.4) and its contact speed.
fn wheel_adhesion(this: GPtr<u8>, idx: usize, mult: f32, burst_off: isize) -> (f32, f32) {
    let cp = this.byte_add(0x660 + 0x2C * idx as isize);
    wr::<u8>(cp, 0x20, 0x3C);
    let si = GPtr::from_addr(SURFACE_INFOS);
    let mut adh = st0(get_adhesive_limit(si, cp)).fmul_m(mult).to_f32();
    let mut destab = 1.0f32;
    if !le(ld(this, 0x7B0), X87::zero()) {
        match get_adhesion_group(si, u32::from(rd::<u8>(cp, 0x23))) {
            1 | 3 => destab = f32::from_bits(0x3F66_6666),
            2 => destab = f32::from_bits(0x3F33_3333),
            _ => {}
        }
    }
    if rd::<u8>(this, 0x36) & 0xF8 == 0 {
        adh = st0(get_wet_multiplier(si, u32::from(rd::<u8>(cp, 0x23))))
            .fmul_m(adh)
            .to_f32();
    }
    if rd::<u8>(this, burst_off) == 1 {
        adh = x(adh).fmul_m(f32::from_bits(0x3ECC_CCCD)).to_f32();
    }
    (adh, destab)
}

/// The contact speed at a wheel point, minus the wheel entity's speed.
fn wheel_speed(this: GPtr<u8>, point: CVector, idx: usize) -> CVector {
    let mut t = CVector::default();
    let mut s = rv(get_speed(this, GPtr::from_raw(&mut t), point).cast(), 0);
    let e = rd::<GPtr<u8>>(this, 0x7C0 + 4 * idx as isize);
    if !e.is_null() {
        let mut t2 = CVector::default();
        let o = rv(
            get_speed(e, GPtr::from_raw(&mut t2), rv(this, 0x7D0 + 0xC * idx as isize)).cast(),
            0,
        );
        s = CVector::new(
            x(s.x).fsub_m(o.x).to_f32(),
            x(s.y).fsub_m(o.y).to_f32(),
            x(s.z).fsub_m(o.z).to_f32(),
        );
    }
    s
}

/// The front wheel (0x6BAC51.. front-first, 0x6BB575.. rear-first).
fn front_wheel(this: GPtr<u8>, f: &mut Frame, wi: &mut WheelInputs, second: bool) {
    if le(ld(this, 0x730), X87::zero()) && le(ld(this, 0x734), X87::zero()) {
        let v = ld(this, 0x758).fmul_m(f32::from_bits(0x3F73_3333)).to_f32();
        wr::<f32>(this, 0x758, v);
        let r = if second { x(v).fmul_m(ts()) } else { x(v) };
        wr::<f32>(this, 0x750, r.fadd_m(rd::<f32>(this, 0x750)).to_f32());
        return;
    }
    let lean = ld(this, 0x644);
    let mut d = CVector::new(lean.fsin().fchs().to_f32(), lean.fcos().to_f32(), 0.0);
    let mut out = CVector::default();
    let fwd0 = rv(multiply3x3(gp(&mut out), mat(this), gp(&mut d)), 0);
    let idx = wi.front;
    let (mut fwd, mut right) = wheel_axes(this, fwd0, this.byte_add(0x670 + 0x2C * idx as isize), f.upright);
    g::<u32>(THRUST).write(0);
    let (adh, destab) = wheel_adhesion(this, idx, wi.traction, 0x65C);
    g::<u32>(FRONT_STATE).write(rd::<u32>(this, 0x80C));
    let mut sp = wheel_speed(this, wi.front_point, idx);
    let mut pt = wi.front_point;
    process_bike_wheel(
        this,
        gp(&mut fwd),
        gp(&mut right),
        gp(&mut sp),
        gp(&mut pt),
        2,
        g::<f32>(THRUST).read(),
        x(wi.brake_front).fmul_m(wi.brake).to_f32(),
        x(adh).fmul_m(wi.trac_front).to_f32(),
        destab,
        0,
        this.byte_add(0x758),
        g::<i32>(FRONT_STATE),
        0,
        u32::from(rd::<u8>(this, 0x65C)),
    );
    if f.extra & 4 != 0 {
        let s = g::<u32>(FRONT_STATE).read();
        if s == 1 || s == 2 {
            g::<u32>(FRONT_STATE).write(0);
        }
    }
}

/// The rear wheel (0x6BAF62..0x6BB4E6).
fn rear_wheel(this: GPtr<u8>, f: &mut Frame, wi: &mut WheelInputs) {
    if le(ld(this, 0x738), X87::zero()) && le(ld(this, 0x73C), X87::zero()) {
        if rd::<u8>(this, 0x428) & 0x20 != 0 {
            wr::<u32>(this, 0x75C, 0);
        } else if x(wi.accel).fcom(X87::zero()).test_parity(0x44) {
            if !le(x(wi.accel), X87::zero()) {
                if !ge(ld(this, 0x75C), c(F1)) {
                    wr::<f32>(
                        this,
                        0x75C,
                        ld(this, 0x75C).fsub_m(f32::from_bits(0x3DCC_CCCD)).to_f32(),
                    );
                }
            } else if !le(ld(this, 0x75C), c(0xBF80_0000)) {
                wr::<f32>(
                    this,
                    0x75C,
                    ld(this, 0x75C).fadd_m(f32::from_bits(0x3D4C_CCCD)).to_f32(),
                );
            }
        }
        wr::<f32>(
            this,
            0x754,
            x(ts())
                .fmul_m(rd::<f32>(this, 0x75C))
                .fadd_m(rd::<f32>(this, 0x754))
                .to_f32(),
        );
        return;
    }
    // L120: rear brake, L124: rear traction multiplier
    let mut rbrake = wi.brake;
    let mut rtrac = wi.traction;
    let m = mat(this);
    let idx = wi.rear;
    let (mut fwd, mut right) = wheel_axes(
        this,
        rv(m, 0x10),
        this.byte_add(0x670 + 0x2C * idx as isize),
        f.upright,
    );
    let tm_k = f32::from_bits(0xBA37_8034);
    let burnout_force = |this: GPtr<u8>, f: &Frame, k: X87| {
        let m = mat(this);
        let (rz, ry) = (
            k.fmul_m(rd::<f32>(m, 8)).to_f32(),
            k.fmul_m(rd::<f32>(m, 4)).to_f32(),
        );
        let rx = k.fmul_m(rd::<f32>(m, 0)).to_f32();
        let t = ts();
        let v = CVector::new(
            x(rx).fmul_m(t).to_f32(),
            x(ry).fmul_m(t).to_f32(),
            x(rz).fmul_m(t).to_f32(),
        );
        apply_turn_force(this, f.points[2], v);
    };
    if rd::<u8>(this, 0x428) & 0x20 != 0 {
        rbrake = f32::from_bits(0x469C_4000);
        wr::<u32>(this, 0x7AC, F1);
    } else if rd::<u16>(this, 0x7A8) != 0 {
        let k = ld(this, 0x494).fmul_m(rd::<f32>(this, 0x90)).fmul_m(tm_k);
        rbrake = 0.0;
        rtrac = 0.0;
        burnout_force(this, f, k);
    } else if !ge(ld(this, 0x7AC), c(F1)) && !le(ld(this, 0x49C), c(0x3F40_0000)) {
        rtrac = x(wi.traction).fmul_m(rd::<f32>(this, 0x7AC)).to_f32();
        let k = c(F1)
            .fsub_m(rd::<f32>(this, 0x7AC))
            .fmul_m(rd::<f32>(this, 0x494))
            .fmul_m(rd::<f32>(this, 0x90))
            .fmul_m(tm_k);
        burnout_force(this, f, k);
    }
    if !le(gf(THRUST), X87::zero()) && !le(x(wi.brake), X87::zero()) {
        wi.brake = 0.0;
    }
    g::<f32>(THRUST).write(wi.accel);
    let (adh, destab) = wheel_adhesion(this, idx, rtrac, 0x65D);
    wi.accel = destab;
    g::<u32>(REAR_STATE).write(rd::<u32>(this, 0x810));
    let mut sp = wheel_speed(this, wi.rear_point, idx);
    let mut pt = wi.rear_point;
    process_bike_wheel(
        this,
        gp(&mut fwd),
        gp(&mut right),
        gp(&mut sp),
        gp(&mut pt),
        2,
        g::<f32>(THRUST).read(),
        x(rbrake).fmul_m(wi.brake_rear).to_f32(),
        x(adh).fmul_m(wi.trac_rear).to_f32(),
        destab,
        1,
        this.byte_add(0x75C),
        g::<i32>(REAR_STATE),
        1,
        u32::from(rd::<u8>(this, 0x65D)),
    );
    if f.extra & 4 != 0 {
        let s = g::<u32>(REAR_STATE).read();
        if s == 1 || s == 2 {
            g::<u32>(REAR_STATE).write(0);
        }
    }
}

/// 0x6BB87A..0x6BBBAD: the rider lean animation, the forced ground normal and the lean angle.
fn lean_angle(this: GPtr<u8>, f: &mut Frame) {
    // L188: the rider's lean from animation 0xC3
    let mut anim = 0.0f32;
    let d = rd::<GPtr<u8>>(this, 0x460);
    if !d.is_null() {
        let a = get_association(rd::<GPtr<u8>>(d, 0x18), 0xC3);
        if !a.is_null() {
            anim = ld(a, 0x18).fmul_m(f32::from_bits(0x3E32_B8C3)).to_f32();
        }
    }
    if f.upright {
        wv(this, 0x618, CVector::new(0.0, 0.0, 1.0));
        let mut t = CVector::default();
        let tp = gp(&mut t);
        cross_product(tp, mat(this).byte_add(0x10), this.byte_add(0x618));
        vector_normalise(tp);
        let mut o = CVector::default();
        let r = rv(cross_product(gp(&mut o), tp, mat(this).byte_add(0x10)), 0);
        wv(this, 0x618, r);
        vector_normalise(this.byte_add(0x618));
    }
    let v = if f.extra & 2 == 0 && rd::<u8>(this, 0x614) & 8 == 0 {
        if rd::<u8>(this, 0x614) & 0x10 != 0 {
            let p = pow(
                X87::from_f64_exact(f64::from_bits(0x3FEF_0A3D_8000_0000)),
                x(ts()),
            )
            .to_f32();
            let a = asin(rd::<f32>(mat(this), 8));
            x(p).fmul_m(rd::<f32>(this, 0x64C)).fsub(
                c(F1)
                    .fsub_m(p)
                    .fmul(a.fadd_m(anim).fadd_m(f32::from_bits(0x3E86_0A92))),
            )
        } else {
            pow(
                X87::from_f64_exact(f64::from_bits(0x3FEE_6666_6000_0000)),
                x(ts()),
            )
            .fmul_m(rd::<f32>(this, 0x64C))
        }
    } else {
        let mut o = CVector::default();
        let r = rv(
            cross_product(gp(&mut o), mat(this).byte_add(0x10), this.byte_add(0x618)),
            0,
        );
        wv(this, 0x624, r);
        vector_normalise(this.byte_add(0x624));
        let side = this.byte_add(0x624);
        let num = if rd::<u32>(this, 0xFC) != 0 {
            X87::zero()
        } else if rd::<u8>(this, 0x804) == 0 {
            let h = rd::<GPtr<u8>>(this, 0x384);
            ld(h, 0xA0)
                .fmul_m(f32::from_bits(0x3C8E_FA35))
                .fdivr_m(rd::<f32>(this, 0x494))
                .fmul_m(ts())
                .fmul_m(f32::from_bits(0xBB83_126F))
        } else {
            let (dx, dy, dz) = if rd::<u32>(this, 0x40) & 4 != 0 {
                let d = (
                    x(f.old_speed.x).fsub_m(rd::<f32>(this, 0x630)),
                    x(f.old_speed.y).fsub_m(rd::<f32>(this, 0x634)),
                    x(f.old_speed.z).fsub_m(rd::<f32>(this, 0x638)),
                );
                wv(this, 0x630, f.old_speed);
                d
            } else {
                (
                    ld(this, 0x44).fsub_m(f.old_speed.x),
                    ld(this, 0x48).fsub_m(f.old_speed.y),
                    ld(this, 0x4C).fsub_m(f.old_speed.z),
                )
            };
            dy.fmul_m(rd::<f32>(side, 4))
                .fadd(dz.fmul_m(rd::<f32>(side, 8)))
                .fadd(dx.fmul_m(rd::<f32>(side, 0)))
        };
        let step = if !c(0x3C23_D70A).fcom(x(ts())).test_any(0x41) {
            c(0x3C23_D70A)
        } else {
            x(ts())
        };
        let mut l184 = num.fdiv(step.fmul_m(f32::from_bits(0x3C03_126F))).to_f32();
        let bh = rd::<GPtr<u8>>(this, 0x63C);
        let lim = if rd::<u8>(this, 0x65C) == 1 {
            c(0x3ECC_CCCD).fmul_m(rd::<f32>(bh, 0x14))
        } else {
            ld(bh, 0x14)
        };
        if !le(x(l184), lim) {
            l184 = lim.to_f32();
        } else if !ge(x(l184), lim.fchs()) {
            l184 = lim.fchs().to_f32();
        }
        let p = pow(ld(bh, 0x1C), x(ts())).to_f32();
        asin(l184)
            .fsub_m(anim)
            .fmul(c(F1).fsub_m(p))
            .fadd(x(p).fmul_m(rd::<f32>(this, 0x64C)))
    };
    wr::<f32>(this, 0x64C, v.to_f32());
    wr::<u32>(this, 0x648, rd::<u32>(this, 0x64C));
}

/// 0x6BBC18..0x6BC914: fire, delayed explosion, pad shake, danger scan, resting speeds and the
/// ground alignment / wheelie torques.
fn after_physics(this: GPtr<u8>, f: &mut Frame) {
    if !ge(ld(this, 0x4C0), c(0x437A_0000)) && rd::<u8>(this, 0x36) & 0xF8 != 0x28 {
        if rd::<u32>(this, 0x594) != 0xA {
            let rw = rd::<GPtr<u8>>(this, 0x18);
            let frame = if rw.is_null() {
                GPtr::from_addr(0)
            } else {
                rd::<GPtr<u8>>(rw, 4).byte_add(0x10)
            };
            if rd::<u32>(this, 0x57C) == 0 && !frame.is_null() {
                let vs = rd::<GPtr<u8>>(crate::common::model_info(i32::from(rd::<i16>(this, 0x22))), 0x5C);
                let mut off = rv(vs, 0x54);
                let fx = create_fx_system(GPtr::from_addr(0xA9_AE80), 0x85_AA5C, gp(&mut off), frame, 0);
                wr::<GPtr<u8>>(this, 0x57C, fx);
                if !fx.is_null() {
                    fx_play(fx);
                    let mut ev = [0u32; 12];
                    let evp = GPtr::from_raw(ev.as_mut_ptr()).cast::<u8>();
                    event_vehicle_on_fire_ctor(evp, this);
                    event_group_add(get_event_global_group(), evp, 0);
                    event_vehicle_on_fire_dtor(evp);
                }
            }
        }
        let ms = ftol32(x(ts()).fmul_m(f32::from_bits(0x3CA3_D70A)).fmul_m(1000.0)) as u32;
        let t = X87::from_f64_exact(f64::from(ms))
            .fadd_m(rd::<f32>(this, 0x7BC))
            .to_f32();
        wr::<f32>(this, 0x7BC, t);
        if !le(x(t), c(0x459C_4000)) {
            v_blow_up_car(this, rd::<GPtr<u8>>(this, 0x800), 0);
        }
    } else {
        wr::<u32>(this, 0x7BC, 0);
        let fx = rd::<GPtr<u8>>(this, 0x57C);
        if !fx.is_null() {
            fx_kill(fx);
            wr::<u32>(this, 0x57C, 0);
        }
    }
    process_delayed_explosion(this);
    pad_shake(this);
    wr::<u8>(this, 0x42B, rd::<u8>(this, 0x42B) & 0xDE);
    if rd::<u8>(this, 0x429) & 0x20 == 0 {
        scan_for_ped_danger(this);
    }
    let pf = rd::<u32>(this, 0x40);
    if pf & 4 != 0 && pf & 8 != 0 {
        for off in (0x44isize..0x74).step_by(4) {
            wr::<u32>(this, off, 0);
        }
    } else if !f.skip
        && (!ld(this, 0x49C).fcom(X87::zero()).test_parity(0x44) || rd::<u8>(this, 0x36) & 0xF8 == 0x28)
        && (0..3isize).all(|k| !ge(ld(this, 0x44 + 4 * k).fabs(), c(0x3BA3_D70A)))
        && (le(ld(this, 0xD8), X87::zero()) || rd::<GPtr<u8>>(this, 0xDC) != find_player_ped(-1))
    {
        wr::<u32>(this, 0x58, 0);
        for off in [0x44isize, 0x48, 0x4C] {
            wr::<u32>(this, off, 0);
        }
    }
    if f.extra & 2 == 0 && rd::<u8>(this, 0x614) & 0x18 == 0 {
        return;
    }
    alignment(this, f);
    if rd::<u8>(this, 0x36) & 0xF8 != 0 {
        return;
    }
    wheelie_control(this, f);
}

/// 0x6BBD7F..0x6BC006: suspension and roughness pad shake.
fn pad_shake(this: GPtr<u8>) {
    let ms2 = sq3(this, 0x44).to_f32();
    let mut susp = 0.0f32;
    let mut rough = 0.0f32;
    for i in 0..4isize {
        let comp = 0x710 + 4 * i;
        let change = ld(this, comp + 0x10).fsub_m(rd::<f32>(this, comp)).to_f32();
        let st = rd::<u8>(this, 0x36) >> 3;
        if !le(x(change), c(0x3E99_999A))
            && (i == 0 || i == 2)
            && !le(x(ms2), c(0x3D23_D70B))
            && (st == 0 || st == 3)
            && !le(x(change), x(susp))
        {
            susp = change;
        }
        if !ge(ld(this, comp), c(0x3F80_0000)) && rd::<u8>(this, 0x36) & 0xF8 == 0 {
            let r = get_roughness(
                GPtr::from_addr(SURFACE_INFOS),
                u32::from(rd::<u8>(this, 0x683 + 0x2C * i)),
            );
            let rr = xi(r as i32).fmul_m(f32::from_bits(0x3DCC_CCCD));
            if le(x(rough), rr) {
                rough = rr.to_f32();
            }
        }
        wr::<u32>(this, comp + 0x10, rd::<u32>(this, comp));
        wr::<u32>(this, comp, F1);
    }
    if g::<u32>(0xB7_CB84).read() & 0x7FF > 0x320 {
        if !x(rough).fcom(c(0x3E94_7AE1)).test_any(1) {
            susp = 0.0;
        }
        rough = 0.0;
    }
    if le(x(susp), X87::zero()) && le(x(rough), X87::zero()) {
        return;
    }
    if rd::<u8>(this, 0x36) & 0xF8 != 0 {
        return;
    }
    let s2 = sq3(this, 0x44);
    if le(s2, c(0x3C23_D70B)) {
        return;
    }
    let sp = s2.fsqrt().fdiv_m(rd::<f32>(this, 0x8C));
    let t = ts();
    let (freq, time) = if !le(x(susp), X87::zero()) {
        let mut fr = sp.fmul_m(susp).fmul_m(f32::from_bits(0x48C3_5000)).fadd_m(100.0);
        if !ge(c(0x437A_0000), fr) {
            fr = c(0x437A_0000);
        }
        let fb = ftol32(fr) as u8;
        (fb, ftol32(x(t).fmul_m(20000.0).fdiv(xi(i32::from(fb)))))
    } else {
        let mut fr = sp.fmul_m(rough).fmul_m(f32::from_bits(0x48C3_5000)).fadd_m(40.0);
        if !ge(c(0x4316_0000), fr) {
            fr = c(0x4316_0000);
        }
        let fb = ftol32(fr) as u8;
        (fb, ftol32(x(t).fmul_m(5000.0).fdiv(xi(i32::from(fb)))))
    };
    start_shake(get_pad(0), time as u32, u32::from(freq), 0);
}

/// 0x6BC1A6..0x6BC348: the torque aligning the bike's up axis with the ground normal +0x618.
fn alignment(this: GPtr<u8>, f: &mut Frame) {
    let m = mat(this);
    // L160: right . groundNormal clamped to [-1, 1]
    let d = ld(this, 0x620)
        .fmul_m(rd::<f32>(m, 8))
        .fadd(ld(this, 0x61C).fmul_m(rd::<f32>(m, 4)))
        .fadd(ld(m, 0).fmul_m(rd::<f32>(this, 0x618)));
    let l160 = if !le(d, c(F1)) {
        1.0
    } else if !ge(x(d.to_f32()), c(0xBF80_0000)) {
        -1.0
    } else {
        d.to_f32()
    };
    let mut out = CVector::default();
    let r = multiply3x3(gp(&mut out), m, this.byte_add(0xA4));
    f.old_speed = rv(r, 0);
    let m = mat(this);
    let point = CVector::new(
        x(f.old_speed.x).fadd_m(rd::<f32>(m, 0)).to_f32(),
        x(f.old_speed.y).fadd_m(rd::<f32>(m, 4)).to_f32(),
        x(f.old_speed.z).fadd_m(rd::<f32>(m, 8)).to_f32(),
    );
    let k = if f.extra & 2 != 0 {
        0xBD8F_5C29
    } else {
        0xBDCC_CCCD
    };
    let s = x(l160).fmul_m(rd::<f32>(this, 0x90)).fmul_m(f32::from_bits(k));
    let ux = s.fmul_m(rd::<f32>(m, 0x20));
    let uy = s.fmul_m(rd::<f32>(m, 0x24)).to_f32();
    let uz = s.fmul_m(rd::<f32>(m, 0x28)).to_f32();
    let t = ts();
    let force = CVector::new(
        ux.fmul_m(t).to_f32(),
        x(uy).fmul_m(t).to_f32(),
        x(uz).fmul_m(t).to_f32(),
    );
    apply_turn_force(this, force, point);
    if f.extra & 2 != 0 {
        wr::<u8>(this, 0x614, rd::<u8>(this, 0x614) & 0xEF);
    }
}

/// `min(|v|, 0.1)` as the wheelie blocks compute it (`fcomp 0.1; test ah, 0x41; jne` recompute).
fn speed_capped(this: GPtr<u8>) -> X87 {
    if !le(sq3(this, 0x44).fsqrt(), c(0x3DCC_CCCD)) {
        c(0x3DCC_CCCD)
    } else {
        sq3(this, 0x44).fsqrt()
    }
}

/// 0x6BC34D..0x6BC914: the player's wheelie (front up) and stoppie (rear up) control.
fn wheelie_control(this: GPtr<u8>, f: &mut Frame) {
    let zero = X87::zero();
    let m = mat(this);
    // `fcomp 0; test ah, 0x41; jp` leaves the block for a positive counter
    let up = |off: isize| ld(this, off).fcom(zero).test_parity(0x41);
    let front_air = !up(0x730) && !up(0x734);
    let rear_air = !up(0x738) && !up(0x73C);
    let half = 0.5f32;
    // the pitch push around right at `com + forward`
    let pitch = |this: GPtr<u8>, f: &Frame, s: X87, bh_off: isize, mv: X87| {
        let bh = rd::<GPtr<u8>>(this, 0x63C);
        let l188 = s.fmul_m(rd::<f32>(bh, bh_off)).fmul(mv).to_f32();
        let modf = st0(get_fat_and_muscle_modifier(0xC)).fmul_m(l188);
        let m = mat(this);
        let point = CVector::new(
            x(f.old_speed.x).fadd_m(rd::<f32>(m, 0x10)).to_f32(),
            x(f.old_speed.y).fadd_m(rd::<f32>(m, 0x14)).to_f32(),
            x(f.old_speed.z).fadd_m(rd::<f32>(m, 0x18)).to_f32(),
        );
        let k = modf.fmul_m(rd::<f32>(this, 0x90)).fmul_m(ts()).fmul_m(half);
        let force = CVector::new(
            k.fmul_m(rd::<f32>(m, 0x20)).to_f32(),
            k.fmul_m(rd::<f32>(m, 0x24)).to_f32(),
            k.fmul_m(rd::<f32>(m, 0x28)).to_f32(),
        );
        apply_turn_force(this, force, point);
    };
    if front_air && !le(ld(m, 0x18), zero) && (!le(ld(this, 0x738), zero) || !le(ld(this, 0x73C), zero)) {
        let bh = rd::<GPtr<u8>>(this, 0x63C);
        let t = ld(bh, 0x2C).fsub_m(rd::<f32>(m, 0x18));
        let s = if !le(t, c(0x3E19_999A)) {
            let v = t.fsubr_m(f32::from_bits(0x3E99_999A));
            if le(zero, v) { v } else { zero }
        } else if ge(t, c(0xBDA3_D70A)) {
            t
        } else {
            let v = t.fsubr_m(f32::from_bits(0xBE0F_5C29));
            if ge(zero, v) { v } else { zero }
        };
        pitch(this, f, s, 0x38, speed_capped(this));
        // the steering roll around right at `com + forward`
        let m = mat(this);
        let point = CVector::new(
            x(f.old_speed.x).fadd_m(rd::<f32>(m, 0x10)).to_f32(),
            x(f.old_speed.y).fadd_m(rd::<f32>(m, 0x14)).to_f32(),
            x(f.old_speed.z).fadd_m(rd::<f32>(m, 0x18)).to_f32(),
        );
        let bh = rd::<GPtr<u8>>(this, 0x63C);
        let k = ld(bh, 0x34)
            .fmul_m(rd::<f32>(this, 0x644))
            .fmul_m(rd::<f32>(this, 0x90))
            .fmul_m(ts())
            .fmul_m(half);
        let force = CVector::new(
            k.fmul_m(rd::<f32>(m, 0)).to_f32(),
            k.fmul_m(rd::<f32>(m, 4)).to_f32(),
            k.fmul_m(rd::<f32>(m, 8)).to_f32(),
        );
        apply_turn_force(this, force, point);
        let bh = rd::<GPtr<u8>>(this, 0x63C);
        let m = mat(this);
        let k = sq3(this, 0x44)
            .fsqrt()
            .fmul_m(rd::<f32>(this, 0x8C))
            .fmul_m(rd::<f32>(bh, 0x34))
            .fmul_m(rd::<f32>(this, 0x644))
            .fmul_m(ts())
            .fmul_m(g::<f32>(0x8D_3288).read());
        let fx = k.fmul_m(rd::<f32>(m, 0));
        let fy = k.fmul_m(rd::<f32>(m, 4)).to_f32();
        let fz = k.fmul_m(rd::<f32>(m, 8)).to_f32();
        let sp = sq3(this, 0x44).fsqrt().to_f32();
        let force = CVector::new(
            fx.fmul_m(sp).to_f32(),
            x(fy).fmul_m(sp).to_f32(),
            x(fz).fmul_m(sp).to_f32(),
        );
        apply_move_force(this, force);
        wr::<f32>(
            this,
            0x648,
            x(ts())
                .fmul_m(rd::<f32>(this, 0x644))
                .fmul_m(g::<f32>(0x8D_3284).read())
                .fadd_m(rd::<f32>(this, 0x648))
                .to_f32(),
        );
        return;
    }
    if !rear_air || ge(ld(m, 0x18), zero) || (le(ld(this, 0x730), zero) && le(ld(this, 0x734), zero)) {
        return;
    }
    let bh = rd::<GPtr<u8>>(this, 0x63C);
    let t = ld(bh, 0x30).fsub_m(rd::<f32>(m, 0x18));
    let s = if !le(t, c(0x3E19_999A)) {
        let v = t.fsubr_m(f32::from_bits(0x3E99_999A));
        if le(zero, v) { v } else { zero }
    } else if ge(t, c(0xBE19_999A)) {
        t
    } else {
        let v = t.fsubr_m(f32::from_bits(0xBE99_999A));
        if ge(zero, v) { v } else { zero }
    };
    let sp = sq3(this, 0x44).fsqrt();
    let mv = if ge(c(0x3DCC_CCCD), sp) {
        sp
    } else {
        c(0x3DCC_CCCD)
    };
    pitch(this, f, s, 0x3C, mv);
    let m = mat(this);
    let l138 = ld(this, 0x4C)
        .fmul_m(rd::<f32>(m, 8))
        .fadd(ld(this, 0x48).fmul_m(rd::<f32>(m, 4)))
        .fadd(ld(m, 0).fmul_m(rd::<f32>(this, 0x44)))
        .fmul_m(rd::<f32>(this, 0x90))
        .fmul_m(ts())
        .fmul_m(g::<f32>(0x8D_3280).read())
        .to_f32();
    let mut zup = CVector::new(0.0, 0.0, 1.0);
    let mut o = CVector::default();
    let op = gp(&mut o);
    cross_product(op, gp(&mut zup), mat(this));
    vector_normalise(op);
    let o = rv(op, 0);
    let point = CVector::new(
        x(o.x).fchs().to_f32(),
        x(o.y).fchs().to_f32(),
        x(o.z).fchs().to_f32(),
    );
    let k = x(l138).fchs();
    let m = mat(this);
    let force = CVector::new(
        k.fmul_m(rd::<f32>(m, 0)).to_f32(),
        k.fmul_m(rd::<f32>(m, 4)).to_f32(),
        k.fmul_m(rd::<f32>(m, 8)).to_f32(),
    );
    apply_turn_force(this, force, point);
}

pub const HOOKS: &[HookReg] = &[HOOK_BIKE_PROCESS_CONTROL];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
