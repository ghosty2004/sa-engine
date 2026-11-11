//! WRITER-VEH-BIG (batch `vehbig-1`): `CAutomobile::ProcessControl`, the per-frame driver of every
//! four-wheeled vehicle (and of the heli / plane / quad / monster truck / trailer subclasses, which
//! call it). Ported from the 1.0 US bytes block by block, x87 arithmetic through `sa_math::x87::X87`;
//! gta-reversed served as a structural guide only. Where they differ, the bytes win (noted inline).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;
use sa_world::world::common::{find_player_ped, find_player_vehicle};

use crate::common::{ftol32, g, rand_at, rd, wr, x, xi};

orig! {
    /// `CEntity::GetColModel()` (0x535300)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => sa_world::world::entity::get_col_model;
    /// `CAEVehicleAudioEntity::Service()` (0x502280)
    fn audio_service(a: GPtr<u8>) = thiscall 0x502280 => unported;
    /// `CAutomobile::IsInAir()` (0x6A6140)
    fn is_in_air(v: GPtr<u8>) -> u8 = thiscall 0x6A6140 => unported;
    /// `CAutomobile::ProcessAutoBusDoors()` (0x6A38A0)
    fn process_auto_bus_doors(v: GPtr<u8>) = thiscall 0x6A38A0 => unported;
    /// `CVehicle::ProcessCarAlarm()` (0x6D21F0)
    fn process_car_alarm(v: GPtr<u8>) = thiscall 0x6D21F0 => crate::vehicle2::process_car_alarm;
    /// `CVehicle::UpdateClumpAlpha()` (0x6D2980)
    fn update_clump_alpha(v: GPtr<u8>) = thiscall 0x6D2980 => crate::vehicle2::update_clump_alpha;
    /// `CVehicle::UpdatePassengerList()` (0x6D29E0)
    fn update_passenger_list(v: GPtr<u8>) = thiscall 0x6D29E0 => crate::vehicle2::update_passenger_list;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => sa_world::world::entity::register_reference;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => sa_world::world::entity::clean_up_old_reference;
    /// `CVehicle::IsUpsideDown()` (0x6D1D90)
    fn is_upside_down(v: GPtr<u8>) -> u8 = thiscall 0x6D1D90 => crate::vehicle2::is_upside_down;
    /// `CVehicle::CanPedEnterCar()` (0x6D1E80)
    fn can_ped_enter_car(v: GPtr<u8>) -> u8 = thiscall 0x6D1E80 => unported;
    /// `CAutomobile::ProcessAI(uint32&)` (vtable +0x108; 0x6B4800 for CAutomobile)
    fn v_process_ai(v: GPtr<u8>, flags: GPtr<u32>) -> u8 = thiscall vslot 0x108 => unported;
    /// `CAutomobile::VehicleDamage(float, eVehicleCollisionComponent, CEntity*, CVector*, CVector*, eWeaponType)` (vtable +0xE0)
    fn v_vehicle_damage(v: GPtr<u8>, dmg: u32, comp: u32, e: u32, p: u32, d: u32, weapon: u32) = thiscall vslot 0xE0 => unported;
    /// `CAutomobile::ProcessControlCollisionCheck(bool)` (vtable +0x5C)
    fn v_process_control_collision_check(v: GPtr<u8>, apply: u32) = thiscall vslot 0x5C => unported;
    /// `CAutomobile::ProcessFlyingCarStuff()` (vtable +0x110)
    fn v_process_flying_car_stuff(v: GPtr<u8>) = thiscall vslot 0x110 => unported;
    /// `CAutomobile::ProcessSuspension()` (vtable +0x118)
    fn v_process_suspension(v: GPtr<u8>) = thiscall vslot 0x118 => unported;
    /// `CAutomobile::BreakTowLink()` (vtable +0xF8)
    fn v_break_tow_link(v: GPtr<u8>) -> u8 = thiscall vslot 0xF8 => unported;
    /// `VectorSub(CVector*, const CVector*, const CVector*)` (0x40FE60): `out = a - b`
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => sa_world::world::mvpa_math::vector_sub;
    /// `CVector::SquaredMagnitude()` (0x406DA0), ST0
    fn vector_magnitude_sqr(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `FUN_00424100(float x, float y)` (0x424100): `xor al, al; ret` (a stubbed area check)
    fn area_check_stub(px: u32, py: u32) -> u8 = cdecl 0x424100 => unported;
    /// `CAutomobile::FireTruckControl(CFire*)` (0x729B60)
    fn fire_truck_control(v: GPtr<u8>, fire: u32) = thiscall 0x729B60 => unported;
    /// `CAutomobile::HydraulicControl()` (0x6A07A0)
    fn hydraulic_control(v: GPtr<u8>) = thiscall 0x6A07A0 => crate::mvpd_v6::automobile_hydraulic_control;
    /// `CAutomobile::BoostJumpControl()` (0x6A3A60)
    fn boost_jump_control(v: GPtr<u8>) = thiscall 0x6A3A60 => unported;
    /// `CAutomobile::TowTruckControl()` (0x6A40F0)
    fn tow_truck_control(v: GPtr<u8>) = thiscall 0x6A40F0 => unported;
    /// `CAutomobile::UpdateMovingCollision(float)` (0x6A1460), the float as bits
    fn update_moving_collision(v: GPtr<u8>, angle: u32) -> u8 = thiscall 0x6A1460 => unported;
    /// `CAutomobile::TankControl()` (0x6AE850)
    fn tank_control(v: GPtr<u8>) = thiscall 0x6AE850 => unported;
    /// `CAutomobile::NitrousControl(int8)` (0x6A3EA0)
    fn nitrous_control(v: GPtr<u8>, boosts: u32) = thiscall 0x6A3EA0 => unported;
    /// `FxSystem_c::Kill()` (0x4AA3F0)
    fn fx_kill(fx: GPtr<u8>) = thiscall 0x4AA3F0 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::CarGunJustDown()` (0x53FFE0): the caller tests AX
    fn car_gun_just_down(pad: GPtr<u8>) -> u32 = thiscall 0x53FFE0 => unported;
    /// `CGarages::TriggerMessage(const char*, int16, uint16, int16)` (0x447B80)
    fn garages_trigger_message(text: u32, n1: i32, time: u32, n2: i32) = cdecl 0x447B80 => unported;
    /// `CPhysical::SkipPhysics()` (0x5433B0)
    fn skip_physics(v: GPtr<u8>) = thiscall 0x5433B0 => sa_world::physical::base::skip_physics;
    /// `CPhysical::ProcessControl()` (0x5485E0)
    fn physical_process_control(v: GPtr<u8>) = thiscall 0x5485E0 => sa_world::physical::attach::process_control;
    /// `cHandlingDataMgr::GetBoatPointer(uint8)` (0x6F5300)
    fn get_boat_pointer(mgr: GPtr<u8>, id: u32) -> GPtr<u8> = thiscall 0x6F5300 => crate::handling::get_boat_pointer;
    /// `CVehicle::ProcessBoatControl(tBoatHandlingData*, float*, bool, bool)` (0x6DBCE0)
    fn process_boat_control(v: GPtr<u8>, h: GPtr<u8>, res: GPtr<u8>, hit_wall: u32, post_collision: u32) = thiscall 0x6DBCE0 => unported;
    /// `CAutomobile::ProcessBuoyancy()` (0x6A8C00)
    fn process_buoyancy(v: GPtr<u8>) = thiscall 0x6A8C00 => crate::mvpd_v5::automobile_process_buoyancy;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPlayerPed::ResetPlayerBreath()` (0x60A8A0)
    fn reset_player_breath(p: GPtr<u8>) = thiscall 0x60A8A0 => unported;
    /// `SurfaceInfos_c::GetSkidmarkType(uint32)` (0x55E630) on `g_surfaceInfos` (0xB79538)
    fn get_skidmark_type(si: GPtr<u8>, id: u32) -> u32 = thiscall 0x55E630 => unported;
    /// `SurfaceInfos_c::GetAdhesiveLimit(CColPoint&)` (0x55EB50), ST0
    fn get_adhesive_limit(si: GPtr<u8>, cp: GPtr<u8>) -> f64 = thiscall 0x55EB50 => unported;
    /// `SurfaceInfos_c::GetRoughness(uint32)` (0x55E810)
    fn get_roughness(si: GPtr<u8>, id: u32) -> u32 = thiscall 0x55E810 => unported;
    /// `Multiply3x3(CVector*, const CMatrix*, const CVector*)` (0x59C790)
    fn multiply3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => sa_world::world::mvpa_math::multiply3x3;
    /// `CPhysical::GetSpeed(CVector)` (0x542CE0), result through the hidden pointer
    fn get_speed(e: GPtr<u8>, out: GPtr<CVector>, point: CVector) -> GPtr<CVector> = thiscall 0x542CE0 => sa_world::physical::forces::get_speed;
    /// `CPhysical::ApplyTurnForce(CVector, CVector)` (0x542A50)
    fn apply_turn_force(e: GPtr<u8>, force: CVector, point: CVector) = thiscall 0x542A50 => sa_world::physical::forces::apply_turn_force;
    /// `cTransmission::CalculateDriveAcceleration(...)` (0x6D05E0), ST0
    fn calculate_drive_acceleration(t: GPtr<u8>, gas: GPtr<u8>, gear: GPtr<u8>, change: GPtr<u8>, velocity: GPtr<u8>, i1: GPtr<u8>, i2: GPtr<u8>, wheels: u32, cheat: u32) -> f64 = thiscall 0x6D05E0 => unported;
    /// `CAutomobile::ProcessCarWheelPair(int32, int32, float, CVector*, CVector*, float, float, float, bool)` (0x6A4EC0), floats as bits
    fn process_car_wheel_pair(v: GPtr<u8>, left: u32, right: u32, steer: u32, speeds: GPtr<u8>, points: GPtr<u8>, traction: u32, accel: u32, brake: u32, front: u32) = thiscall 0x6A4EC0 => unported;
    /// `CVehicle::ProcessSirenAndHorn(bool)` (0x6E0950)
    fn process_siren_and_horn(v: GPtr<u8>, check: u32) = thiscall 0x6E0950 => crate::vehicle2::process_siren_and_horn;
    /// `CAutomobile::ProcessCarOnFireAndExplode(bool)` (0x6A7090)
    fn process_car_on_fire_and_explode(v: GPtr<u8>, explode: u32) = thiscall 0x6A7090 => crate::mvpd_v2::automobile_process_car_on_fire_and_explode;
    /// `CVehicle::UsesSiren()` (0x6D8470)
    fn uses_siren(v: GPtr<u8>) -> u8 = thiscall 0x6D8470 => crate::vehicle2::uses_siren;
    /// `CCarAI::MakeWayForCarWithSiren(CVehicle*)` (0x41D660)
    fn make_way_for_car_with_siren(v: GPtr<u8>) = cdecl 0x41D660 => sa_pop::carctrl4::make_way_for_car_with_siren;
    /// `CDamageManager::GetWheelStatus(int32)` (0x6C21B0)
    fn get_wheel_status(dm: GPtr<u8>, wheel: i32) -> u32 = thiscall 0x6C21B0 => crate::damage::get_wheel_status;
    /// `CPad::StartShake(int16, uint8, uint32)` (0x53F920)
    fn start_shake(pad: GPtr<u8>, time: u32, freq: u32, delay: u32) = thiscall 0x53F920 => unported;
    /// `CCarCtrl::ScanForPedDanger(CVehicle*)` (0x42CE40)
    fn scan_for_ped_danger(v: GPtr<u8>) = cdecl 0x42CE40 => sa_pop::carctrl4::scan_for_ped_danger;
    /// `CPlayerPed::GetPadFromPlayer()` (0x609560)
    fn get_pad_from_player(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CPad::GetCarGunLeftRight()` (0x53FC50): int16 in AX
    fn get_car_gun_left_right(pad: GPtr<u8>) -> u32 = thiscall 0x53FC50 => unported;
    /// `CPad::GetCarGunUpDown()` (0x53FC10): int16 in AX
    fn get_car_gun_up_down(pad: GPtr<u8>) -> u32 = thiscall 0x53FC10 => unported;
    /// `CVehicle::UpdateTractorLink(bool, bool)` (0x6E0050)
    fn update_tractor_link(v: GPtr<u8>, a: u32, b: u32) = thiscall 0x6E0050 => unported;
    /// `CVehicle::UpdateTrailerLink(bool, bool)` (0x6DFC50)
    fn update_trailer_link(v: GPtr<u8>, a: u32, b: u32) = thiscall 0x6DFC50 => unported;
    /// `CPhysical::RemoveFromMovingList()` (0x542860)
    fn remove_from_moving_list(v: GPtr<u8>) = thiscall 0x542860 => sa_world::physical::base::remove_from_moving_list;
    /// `CPhysical::AddToMovingList()` (0x542800)
    fn add_to_moving_list(v: GPtr<u8>) = thiscall 0x542800 => sa_world::physical::base::add_to_moving_list;
    /// `CAutomobile::ProcessHarvester()` (0x6A9680)
    fn process_harvester(v: GPtr<u8>) = thiscall 0x6A9680 => unported;
}

/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7_CB5C;
/// `g_surfaceInfos`
const SURFACE_INFOS: usize = 0xB7_9538;
/// `gHandlingDataMgr`
const HANDLING_MGR: usize = 0xC2_B9C8;
/// `CCheat::m_aCheatsActive[]` entries read here
const CHEAT_PERFECT_HANDLING: usize = 0x96_914C;
const CHEAT_CARS_ON_WATER: usize = 0x96_9152;
const CHEAT_ALL_CARS_NITRO: usize = 0x96_9165;
const CHEAT_ALL_TAXIS_NITRO: usize = 0x96_918B;

/// `fld dword [p + off]`
fn ld(p: GPtr<u8>, off: isize) -> X87 {
    x(rd::<f32>(p, off))
}

/// The vector at `p + off`.
fn rv(p: GPtr<u8>, off: isize) -> CVector {
    CVector::new(rd::<f32>(p, off), rd::<f32>(p, off + 4), rd::<f32>(p, off + 8))
}

/// `fstp dword [p + off]`
fn st(p: GPtr<u8>, off: isize, v: X87) {
    wr::<f32>(p, off, v.to_f32());
}

/// A `.rdata` constant.
fn c(bits: u32) -> X87 {
    X87::constant(bits)
}

/// `fld dword [addr]` of a `.data` global.
fn gf(addr: usize) -> X87 {
    x(g::<f32>(addr).read())
}

/// `fcomp` + `test ah, 0x41` + `jne` taken: `a <= b` or unordered.
fn le(a: X87, b: X87) -> bool {
    a.fcom(b).test_any(0x41)
}

/// `fcomp` + `test ah, 5` + `jp` taken: `a >= b` or unordered.
fn ge(a: X87, b: X87) -> bool {
    a.fcom(b).test_parity(0x05)
}

/// `fcomp` + `test ah, 0x44` + `jp` taken: `a != b` or unordered.
fn ne(a: X87, b: X87) -> bool {
    a.fcom(b).test_parity(0x44)
}

/// `fcomp` + `test ah, 1` + `jne` taken: `a < b` or unordered.
fn lt(a: X87, b: X87) -> bool {
    a.fcom(b).test_any(0x01)
}

/// `(x*x + y*y) + z*z` of the vector at `p + off`, register operands (the inlined `SquaredMagnitude`).
fn sq3(p: GPtr<u8>, off: isize) -> X87 {
    let (vx, vy, vz) = (ld(p, off), ld(p, off + 4), ld(p, off + 8));
    vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz))
}

/// `CPlaceable::GetPosition()` inlined: the matrix position when there is a matrix, else the
/// placement's.
fn pos_ptr(this: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(this, 0x14);
    if m.is_null() {
        this.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// The status (`+0x36 >> 3`).
fn status(this: GPtr<u8>) -> u8 {
    rd::<u8>(this, 0x36) >> 3
}

/// Some component of a vector at `p + off` differs from 0.0 (three `test ah, 0x44; jp`).
fn any_nonzero3(p: GPtr<u8>, off: isize) -> bool {
    (0..3).any(|k| ne(ld(p, off + 4 * k), X87::zero()))
}

/// The cheat-and-water wheel test the original inlines twice: some wheel with `comp[i] < 1.0`
/// (compression at `comp_off`) stands on surface 0x27 (shallow water).
fn wheel_on_shallow_water(this: GPtr<u8>, comp_off: isize) -> bool {
    (0..4)
        .any(|i| !ge(ld(this, comp_off + 4 * i), c(0x3F80_0000)) && rd::<u8>(this, 0x747 + 0x2C * i) == 0x27)
}

/// `CAutomobile::ProcessControl()` (0x6B1880).
///
/// In order: the frame flags (+0x429 bit 5, +0x868 bit 6, +0x42C bit 1, +0x86A = 0), the audio
/// entity's `Service`; for planes/helis (+0x594 4 or 3) the "explode immediately" flag (crash
/// missions 0x39/0x3A, or a player's car with health < 250 and fire counter +0x981 == 2; not
/// wrecked; hit from below or not in air; rising or submerged). Extra handling flags (+0x42F bit 1
/// -> 1; the all-taxis-nitro cheat on a player's 0x1A4/0x1B6 -> +0x10), nitro install (+0x38C bit
/// 19, +0x48A = 101), bus doors, alarm, clump alpha, passenger list, the ignition bomb start when a
/// driver appears, `CanPedEnterCar` for upside-down cars with passengers, then the virtual
/// `ProcessAI(&flags)` (returns at once when it is true).
///
/// Abandoned/wrecked cars run the fake-physics counter +0xB8: force and torque are averaged with
/// the speeds (`(f + v) * 0.5`), and when both are inside the class limits (planes 0.003 or 0.009
/// with a vehicle damager / 0.003 / 0.1; wrecked 0.006 / 0.0015 / 0.015; else 0.003 / 0.0009 /
/// 0.005, times the time step), the moving speed is below the limit, no ped damaged it and it is
/// not submerged within 50 m of the camera, or when it rests idle (no speed, rear-right
/// compression bits != 1.0), the counter grows; past 10 (or idle) it is clamped to 10 and both
/// speeds are zeroed unless the stubbed area check 0x424100 returns true. Helis with rotor speed,
/// 0x1BF/0x1A1 with no contact wheel and no damage, the Vortex and (cars-on-water cheat) cars on
/// shallow water keep physics.
///
/// A wheel standing on a physical marks +0x42C bit 1 and, unless `CWorld::bForceProcessControl`,
/// postpones the car (+0x1C bit 6) when that entity is in a safe position. Then `VehicleDamage(0,
/// ..., weapon 0x31)`, the model controls (fire truck for a player's 0x197/0x259, tanks, moving
/// collision, tow trucks, hydraulics or the boost jump), nitro or the nitro effects' `Kill`, and
/// the car-gun bomb arming (7000 ms timer or remote detonation, garage message 0x87126C).
///
/// Physics: `SkipPhysics` (gear-change bits cleared, tyre temperature 1.0), or the collision check,
/// `CPhysical::ProcessControl`, the skimmer's boat control or `ProcessBuoyancy`, breath reset of
/// player occupants entering water, the RC Baron's flying stuff, the compression rescale
/// `(c - r) / (1 - r)` with `r = 1 - spring / line` (not for monster trucks), `ProcessSuspension`,
/// the wheel contact points (skidmark type, muddy flag; disks or line ends transformed by the
/// matrix for wheels in the air), contact speeds relative to the wheel entities, the gear-change
/// nose-down torque, the bus-door timer reuse at +0x870, the wheel counters and drive-wheel
/// counts, drive acceleration / traction (cheats x4, x1.2/x1.4, x1.3/x1.4), the player's steer
/// limit (`asin(min(adhesion * traction * g * 4 / v^2, 1)) / (lock * pi/180)`), the wheel pairs
/// (rear first with handling flag 23), horn / siren, and the flying stuff for other models.
///
/// After: `ProcessCarOnFireAndExplode`, the siren way-making, suspension and roughness shake,
/// the wheel-spin audio filter (`pow(rate, timeStep)`), the pad shake, the danger scan, the
/// hydraulic roll torque, tractor/trailer links, the frozen-car rest checks, the camera-shake
/// jitter of the position (`rand()` at 0x6B367E) and the harvester / Rhino specials.
///
/// Kept from the bytes: the special hydraulic data is indexed with `+0x48B` without a -1 check;
/// the shake frequency is `((speed / mass) * shake) * 400000 + 100` (gta-reversed reorders it);
/// the steer-limit colpoint has only its two surface bytes written (the rest is the Rust zero).
#[hook(addr = 0x6b1880, conv = thiscall, name = "CAutomobile::ProcessControl", this = "CAutomobile",
       expect = "81 EC D8 00 00 00 53 55",
       unverified = "vehbig-1", route = "split", hazards = "x87-transcendental,heap-writes,uninit-locals",
       recs = "driving,car-ai,grove-walk",
       note = "rand site 0x6B367E; _CIasin/_CIpow through orig::x87_unary/x87_binary; the bool dword arguments the original builds from stack residue (ProcessBoatControl's two flags, CalculateDriveAcceleration's cheat, StartShake's frequency, ProcessCarOnFireAndExplode's flag) are passed zero-extended; the steer-limit CColPoint is zeroed apart from its surface bytes")]
pub fn automobile_process_control(this: GPtr<u8>) {
    // La0: the extra handling flags ProcessAI receives by reference
    let mut extra: u32 = u32::from(rd::<u8>(this, 0x42F) & 2 != 0);
    let col_data = rd::<GPtr<u8>>(get_col_model(this), 0x2C);
    wr::<u8>(this, 0x429, rd::<u8>(this, 0x429) & 0xDF);
    wr::<u16>(this, 0x86A, 0);
    wr::<u8>(this, 0x868, rd::<u8>(this, 0x868) & 0xBF);
    wr::<u8>(this, 0x42C, rd::<u8>(this, 0x42C) & 0xFD);
    audio_service(this.byte_add(0x138));

    // Lc8: explode immediately
    let mut explode = false;
    let sub = rd::<u32>(this, 0x594);
    if sub == 4 || sub == 3 {
        let mission = rd::<u8>(this, 0x3BA);
        let crashing = mission == 0x39
            || mission == 0x3A
            || (rd::<u8>(this, 0x36) & 0xF8 == 0
                && !ge(ld(this, 0x4C0), c(0x437A_0000))
                && rd::<u8>(this, 0x981) == 2);
        if crashing && rd::<u8>(this, 0x36) & 0xF8 != 0x28 {
            let hit_below = !le(ld(this, 0xD8), X87::zero()) && !le(ld(this, 0xE8), X87::zero());
            if hit_below || is_in_air(this) == 0 {
                // move speed z >= 0 (C0 clear), or submerged
                if !lt(ld(this, 0x4C), X87::zero()) || rd::<u32>(this, 0x40) & 0x100 != 0 {
                    explode = true;
                }
            }
        }
    }

    if g::<u8>(CHEAT_ALL_TAXIS_NITRO).read() != 0 && rd::<u8>(this, 0x36) & 0xF8 == 0 {
        let mi = rd::<u16>(this, 0x22);
        if mi == 0x1A4 || mi == 0x1B6 {
            extra = extra.wrapping_add(0x10);
        }
    }
    if g::<u8>(CHEAT_ALL_CARS_NITRO).read() != 0 || extra & 0x10 != 0 {
        wr::<u32>(this, 0x38C, rd::<u32>(this, 0x38C) | 0x8_0000);
        wr::<u8>(this, 0x48A, 0x65);
    }
    if rd::<u8>(this, 0x429) & 2 != 0 {
        process_auto_bus_doors(this);
    }
    process_car_alarm(this);
    update_clump_alpha(this);
    update_passenger_list(this);
    let flags_42c = if !rd::<GPtr<u8>>(this, 0x460).is_null() {
        if rd::<u8>(this, 0x42C) & 0x20 == 0 && rd::<u8>(this, 0x4A8) & 7 == 5 {
            let who = rd::<GPtr<u8>>(this, 0x4CC);
            wr::<u16>(this, 0x4DE, 1000);
            wr::<GPtr<u8>>(this, 0x4E0, who);
            if !who.is_null() {
                register_reference(who, this.byte_add(0x4E0));
            }
        }
        rd::<u8>(this, 0x42C) | 0x20
    } else {
        rd::<u8>(this, 0x42C) & 0xDF
    };
    wr::<u8>(this, 0x42C, flags_42c);
    if rd::<u8>(this, 0x484) > 0 && is_upside_down(this) != 0 {
        can_ped_enter_car(this);
    }
    if v_process_ai(this, GPtr::from_raw(&mut extra)) != 0 {
        return;
    }

    // Ld1: skip physics
    let mut skip = false;
    let st_now = status(this);
    if st_now == 4 || st_now == 5 {
        abandoned_fake_physics(this, st_now, &mut skip);
    }

    for i in 0..4isize {
        let e = rd::<GPtr<u8>>(this, 0x8E8 + 4 * i);
        if !e.is_null() {
            wr::<u8>(this, 0x42C, rd::<u8>(this, 0x42C) | 2);
            if g::<u8>(0xB7_CD6E).read() == 0 && (rd::<u32>(e, 0x1C) >> 5) & 1 != 0 {
                wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 0x40);
                return;
            }
        }
    }
    if rd::<u8>(this, 0x42C) & 2 != 0 {
        skip = false;
        wr::<u8>(this, 0xB8, 0);
    }

    v_vehicle_damage(this, 0, 0, 0, 0, 0, 0x31);
    let model = rd::<u16>(this, 0x22);
    if status(this) == 0 && (model == 0x197 || model == 0x259) {
        fire_truck_control(this, 0);
    } else {
        match model {
            0x1B0 | 0x234 => tank_control(this),
            0x1BB | 0x1E6 | 0x196 | 0x20C | 0x250 | 0x212 => {
                if update_moving_collision(this, 0xBF80_0000) != 0 {
                    skip = false;
                }
            }
            0x20D | 0x213 => tow_truck_control(this),
            _ => {
                if rd::<u32>(this, 0x38C) & 0x2_0000 != 0 {
                    hydraulic_control(this);
                } else if (g::<u8>(CHEAT_PERFECT_HANDLING).read() != 0 || extra & 0x10 != 0)
                    && status(this) == 0
                    && !le(
                        X87::from_f64_exact(vector_magnitude_sqr(this.byte_add(0x44))),
                        c(0x3D23_D70B),
                    )
                {
                    boost_jump_control(this);
                }
            }
        }
    }
    if rd::<u32>(this, 0x38C) & 0x8_0000 != 0 {
        nitrous_control(this, 0);
    } else {
        for off in [0x978isize, 0x97C] {
            let fx = rd::<GPtr<u8>>(this, off);
            if !fx.is_null() {
                fx_kill(fx);
                wr::<u32>(this, off, 0);
            }
        }
    }
    if find_player_vehicle(-1, 0) == this && car_gun_just_down(get_pad(0)) & 0xFFFF != 0 {
        let b = rd::<u8>(this, 0x4A8);
        match b & 7 {
            1 => {
                wr::<u8>(this, 0x4A8, (b & 0xFC) | 4);
                wr::<u16>(this, 0x4DE, 7000);
                let p = find_player_ped(-1);
                wr::<GPtr<u8>>(this, 0x4E0, p);
                garages_trigger_message(0x87_126C, -1, 3000, -1);
            }
            2 => {
                wr::<u8>(this, 0x4A8, (b & 0xFD) | 5);
                garages_trigger_message(0x87_126C, -1, 3000, -1);
            }
            _ => {}
        }
    }

    if skip {
        skip_physics(this);
        wr::<u8>(this, 0x42B, rd::<u8>(this, 0x42B) & 0xDE);
        wr::<u32>(this, 0x8A4, 0x3F80_0000);
    } else {
        physics(this, col_data, extra);
    }

    process_car_on_fire_and_explode(this, u32::from(explode));
    if rd::<u8>(this, 0x42D) & 0x80 != 0
        && g::<u32>(0xB7_CB4C).read() & 7 == 5
        && uses_siren(this) != 0
        && rd::<u16>(this, 0x22) != 0x1A7
        && find_player_vehicle(-1, 0) == this
    {
        make_way_for_car_with_siren(this);
    }

    shake_and_wheel_spin(this, extra);

    wr::<u8>(this, 0x42B, rd::<u8>(this, 0x42B) & 0xDE);
    if rd::<u8>(this, 0x429) & 0x20 == 0 {
        scan_for_ped_danger(this);
    }
    hydraulic_roll(this);
    tow_links(this);
    rest_checks(this, skip);

    // camera shake: jitter the position's z
    let shake = gf(0xC8_1340);
    if !le(shake, X87::zero()) {
        let p = pos_ptr(this);
        let (px, py, pz) = (rd::<u32>(p, 0), rd::<u32>(p, 4), rd::<f32>(p, 8));
        let half = shake.fmul_m(f32::from_bits(0x3F00_0000)).to_f32();
        let r = rand_at(0x6B_367E);
        let z = xi(r).fmul_m(f32::from_bits(0x3800_0100)).fmul_m(half).fadd_m(pz);
        let m = rd::<GPtr<u8>>(this, 0x14);
        if m.is_null() {
            st(this, 0xC, z);
            wr::<u32>(this, 4, px);
            wr::<u32>(this, 8, py);
        } else {
            wr::<u32>(m, 0x30, px);
            wr::<u32>(rd::<GPtr<u8>>(this, 0x14), 0x34, py);
            st(rd::<GPtr<u8>>(this, 0x14), 0x38, z);
        }
    }
    if rd::<u16>(this, 0x22) == 0x214 {
        process_harvester(this);
    }
    if rd::<u16>(this, 0x22) == 0x1B0 && (any_nonzero3(this, 0x44) || any_nonzero3(this, 0x50)) {
        for off in [0x618isize, 0x61C, 0x624, 0x628, 0x630, 0x634, 0x63C, 0x640] {
            wr::<u32>(this, off, 0x3F80_0000);
        }
    }
}

/// 0x6B1AA1..0x6B1EE5: the abandoned/wrecked fake-physics counter (see the hook's docs).
fn abandoned_fake_physics(this: GPtr<u8>, st_now: u8, skip: &mut bool) {
    let one = 0x3F80_0000u32;
    // Lbd: idle
    let idle = rd::<u8>(this, 0x42B) & 1 == 0 && !any_nonzero3(this, 0x44) && rd::<u32>(this, 0x7F0) != one;
    let damaged_by = |kind: u8| {
        if le(ld(this, 0xD8), X87::zero()) {
            return false;
        }
        let e = rd::<GPtr<u8>>(this, 0xDC);
        !e.is_null() && rd::<u8>(e, 0x36) & 7 == kind
    };
    // force limit (register), torque limit (Lc4), moving-speed limit (Ld8)
    let (force_lim, torque_lim, speed_lim) = if rd::<u32>(this, 0x594) == 4 {
        let f = if damaged_by(2) {
            c(0x3B44_9BA6)
        } else {
            c(0x3C13_74BC)
        };
        (f, 0x3B44_9BA6u32, 0x3DCC_CCCDu32)
    } else if st_now == 5 {
        (c(0x3BC4_9BA6), 0x3AC4_9BA6, 0x3C75_C28F)
    } else {
        (c(0x3B44_9BA6), 0x3A6B_EDFB, 0x3BA3_D70A)
    };
    // force = (force + moveSpeed) * 0.5, torque = (torque + turnSpeed) * 0.5
    let half = f32::from_bits(0x3F00_0000);
    let fx = ld(this, 0x74).fadd_m(rd::<f32>(this, 0x44));
    let fy = ld(this, 0x78).fadd_m(rd::<f32>(this, 0x48));
    let fz = ld(this, 0x7C).fadd_m(rd::<f32>(this, 0x4C)).to_f32();
    st(this, 0x74, fx.fmul_m(half));
    st(this, 0x78, fy.fmul_m(half));
    st(this, 0x7C, x(fz).fmul_m(half));
    let tx = ld(this, 0x80).fadd_m(rd::<f32>(this, 0x50));
    let ty = ld(this, 0x54).fadd_m(rd::<f32>(this, 0x84));
    let tz = ld(this, 0x58).fadd_m(rd::<f32>(this, 0x88)).to_f32();
    st(this, 0x80, tx.fmul_m(half));
    st(this, 0x84, ty.fmul_m(half));
    st(this, 0x88, x(tz).fmul_m(half));
    let fl = force_lim.fmul_m(g::<f32>(TIME_STEP).read());
    let tl = gf(TIME_STEP).fmul_m(f32::from_bits(torque_lim));

    let reset = 'r: {
        if lt(fl.fmul(fl), sq3(this, 0x74)) {
            break 'r false;
        }
        if lt(tl.fmul(tl), sq3(this, 0x80)) {
            break 'r false;
        }
        if ge(ld(this, 0xD4), x(f32::from_bits(speed_lim))) {
            break 'r false;
        }
        if damaged_by(3) {
            break 'r false;
        }
        if rd::<u32>(this, 0x40) & 0x100 != 0 {
            let cm = g::<GPtr<u8>>(0xB6_F03C).read();
            let cam = if cm.is_null() {
                GPtr::from_addr(0xB6_F02C)
            } else {
                cm.byte_add(0x30)
            };
            let mut d = CVector::default();
            let r = vector_sub(GPtr::from_raw(&mut d).cast(), pos_ptr(this), cam);
            if !ge(X87::from_f64_exact(vector_magnitude_sqr(r)), c(0x451C_4000)) {
                break 'r false;
            }
        }
        true
    };
    if !reset && !idle {
        wr::<u8>(this, 0xB8, 0);
    } else {
        let n = rd::<u8>(this, 0xB8).wrapping_add(1);
        wr::<u8>(this, 0xB8, n);
        if n > 10 || idle {
            let p = pos_ptr(this);
            if area_check_stub(rd::<u32>(p, 0), rd::<u32>(p, 4)) == 0 {
                if !idle || rd::<u8>(this, 0xB8) > 10 {
                    wr::<u8>(this, 0xB8, 10);
                }
                for off in [0x44isize, 0x48, 0x4C, 0x50, 0x54, 0x58] {
                    wr::<u32>(this, off, 0);
                }
                *skip = true;
            }
        }
    }

    // keep physics for: helis with rotor speed, 0x1BF/0x1A1 without contact wheels and damage,
    // the Vortex, cars on shallow water with the cheat
    let h = rd::<GPtr<u8>>(this, 0x384);
    let model = rd::<u16>(this, 0x22);
    let keep = (rd::<u32>(h, 0xCC) & 0x200_0000 != 0 && !le(ld(this, 0x84C), X87::zero()))
        || ((model == 0x1BF || model == 0x1A1)
            && rd::<u8>(this, 0x960) == 0
            && !ld(this, 0xD8).fcom(X87::zero()).test_parity(0x41))
        || model == 0x21B
        || (g::<u8>(CHEAT_CARS_ON_WATER).read() != 0 && wheel_on_shallow_water(this, 0x7D4));
    if keep {
        *skip = false;
        wr::<u8>(this, 0xB8, 0);
    }
}

#[allow(clippy::needless_range_loop)]
/// 0x6B211F..0x6B2BA5: the physics half of ProcessControl (see the hook's docs).
fn physics(this: GPtr<u8>, col_data: GPtr<u8>, extra: u32) {
    if rd::<u8>(this, 0x42B) & 1 == 0 {
        v_process_control_collision_check(this, 1);
    }
    let hit_wall = (rd::<u32>(this, 0x1C) >> 12) & 1;
    let was_submerged = (rd::<u32>(this, 0x40) >> 8) & 1 != 0;
    let post_collision = !le(ld(this, 0xD8), X87::zero()) && !le(ld(this, 0xE8), c(0x3DCC_CCCD));
    physical_process_control(this);
    if rd::<u16>(this, 0x22) == 0x1CC {
        let mi = g::<GPtr<u8>>(0xA9_B7F8).read();
        let id = rd::<i16>(mi, 0x4A) as i32 as u32;
        let h = get_boat_pointer(GPtr::from_addr(HANDLING_MGR), id);
        process_boat_control(this, h, this.byte_add(0x94C), hit_wall, u32::from(post_collision));
        let f = rd::<u32>(this, 0x40);
        wr::<u32>(
            this,
            0x40,
            if f & 0x100 != 0 {
                f | 0x800_0000
            } else {
                f & 0xF7FF_FFFF
            },
        );
    } else {
        process_buoyancy(this);
    }
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
    if rd::<u16>(this, 0x22) == 0x1D0 {
        v_process_flying_car_stuff(this);
    }
    if rd::<u8>(this, 0x868) & 0x80 == 0 {
        for i in 0..4isize {
            let r = ld(this, 0x878 + 4 * i)
                .fdiv_m(rd::<f32>(this, 0x888 + 4 * i))
                .fsubr_m(1.0);
            let num = ld(this, 0x7D4 + 4 * i).fsub(r);
            let den = c(0x3F80_0000).fsub(r);
            st(this, 0x7D4 + 4 * i, num.fdiv(den));
        }
    }
    v_process_suspension(this);

    // L5c: contact points, L8c: contact speeds
    let mut points = [CVector::default(); 4];
    let mut speeds = [CVector::default(); 4];
    for i in 0..4usize {
        let ii = i as isize;
        if !ge(ld(this, 0x7D4 + 4 * ii), c(0x3F80_0000)) {
            wr::<u8>(this, 0x824 + ii, 0);
            let skid = get_skidmark_type(
                GPtr::from_addr(SURFACE_INFOS),
                u32::from(rd::<u8>(this, 0x747 + 0x2C * ii)),
            );
            wr::<u32>(this, 0x810 + 4 * ii, skid);
            if skid == 2 {
                wr::<u8>(this, 0x824 + ii, 1);
            }
            let p = pos_ptr(this);
            let cp = 0x724 + 0x2C * ii;
            points[i] = CVector::new(
                ld(this, cp).fsub_m(rd::<f32>(p, 0)).to_f32(),
                ld(this, cp + 4).fsub_m(rd::<f32>(p, 4)).to_f32(),
                ld(this, cp + 8).fsub_m(rd::<f32>(p, 8)).to_f32(),
            );
        } else {
            let base = rd::<GPtr<u8>>(col_data, 0x10);
            let mut out = CVector::default();
            if rd::<u8>(col_data, 7) & 1 != 0 {
                let disk = base.byte_add(0x24 * ii);
                points[i] = CVector::new(rd::<f32>(disk, 0), rd::<f32>(disk, 4), rd::<f32>(disk, 8));
                points[i].z = x(points[i].z).fsub_m(rd::<f32>(disk, 0xC)).to_f32();
                let pi = GPtr::from_raw(&mut points[i]).cast::<u8>();
                multiply3x3(GPtr::from_raw(&mut out).cast(), rd::<GPtr<u8>>(this, 0x14), pi);
            } else {
                multiply3x3(
                    GPtr::from_raw(&mut out).cast(),
                    rd::<GPtr<u8>>(this, 0x14),
                    base.byte_add(0x20 * ii + 0x10),
                );
            }
            points[i] = out;
        }
    }
    for i in 0..4usize {
        let ii = i as isize;
        let mut tmp = CVector::default();
        let r = get_speed(this, GPtr::from_raw(&mut tmp), points[i]);
        speeds[i] = rv(r.cast(), 0);
        let e = rd::<GPtr<u8>>(this, 0x8E8 + 4 * ii);
        if !e.is_null() {
            let pos = rv(this, 0x8F8 + 0xC * ii);
            let mut t2 = CVector::default();
            let s = rv(get_speed(e, GPtr::from_raw(&mut t2), pos).cast(), 0);
            speeds[i].x = x(speeds[i].x).fsub_m(s.x).to_f32();
            speeds[i].y = x(speeds[i].y).fsub_m(s.y).to_f32();
            wr::<u32>(this, 0x8E8 + 4 * ii, 0);
            speeds[i].z = x(speeds[i].z).fsub_m(s.z).to_f32();
        }
    }

    let m = rd::<GPtr<u8>>(this, 0x14);
    // Lc4: speed along the forward axis
    let mut fwd_speed = ld(this, 0x4C)
        .fmul_m(rd::<f32>(m, 0x18))
        .fadd(ld(this, 0x48).fmul_m(rd::<f32>(m, 0x14)))
        .fadd(ld(this, 0x44).fmul_m(rd::<f32>(m, 0x10)))
        .to_f32();
    if rd::<u8>(this, 0x42B) & 0x20 != 0
        && !le(ld(this, 0x49C), c(0x3ECC_CCCD))
        && !ge(ld(this, 0x4A0), c(0x3DCC_CCCD))
        && !le(x(fwd_speed), c(0x3E19_999A))
        && find_player_vehicle(-1, 0) == this
    {
        let cam = usize::from(g::<u8>(0xB6_F081).read()) * 0x238;
        if g::<u16>(0xB6_F1A8 + cam).read() != 0x10 {
            gear_change_nose_down(this);
        }
    }

    let h = rd::<GPtr<u8>>(this, 0x384);
    // L9c: brake
    let brake = ld(this, 0x4A0)
        .fmul_m(rd::<f32>(h, 0x94))
        .fmul_m(g::<f32>(TIME_STEP).read())
        .to_f32();
    wr::<u8>(this, 0x962, rd::<u8>(this, 0x961));
    wr::<u8>(this, 0x960, 0);
    wr::<u8>(this, 0x961, 0);
    for i in 0..4isize {
        let wc = 0x7F4 + 4 * i;
        if ge(ld(this, 0x7D4 + 4 * i), c(0x3F80_0000)) {
            let mut v = ld(this, wc).fsub_m(g::<f32>(TIME_STEP).read());
            if le(v, X87::zero()) {
                v = X87::zero();
            }
            st(this, wc, v);
            if le(v, X87::zero()) {
                continue;
            }
        } else {
            wr::<u32>(this, wc, 0x4080_0000);
        }
        wr::<u8>(this, 0x960, rd::<u8>(this, 0x960).wrapping_add(1));
        let drive = match rd::<u8>(rd::<GPtr<u8>>(this, 0x384), 0x74) {
            0x34 => true,
            0x46 => i == 0 || i == 2,
            0x52 => i == 1 || i == 3,
            _ => false,
        };
        if drive {
            wr::<u8>(this, 0x961, rd::<u8>(this, 0x961).wrapping_add(1));
        }
    }

    // Lcc: cheat type
    let cheat_type: u32 = if rd::<u32>(this, 0x38C) & 0x8_0000 != 0 && !ge(ld(this, 0x8A4), X87::zero()) {
        2
    } else if extra & 1 != 0 || g::<u8>(CHEAT_PERFECT_HANDLING).read() != 0 {
        1
    } else {
        0
    };
    // Ld8: acceleration
    let mut accel: f32 = 0.0;
    let h = rd::<GPtr<u8>>(this, 0x384);
    if rd::<u8>(this, 0x428) & 0x10 != 0 && rd::<u32>(h, 0xCC) & 0x600_0000 == 0 {
        let a = calculate_drive_acceleration(
            h.byte_add(0x2C),
            this.byte_add(0x49C),
            this.byte_add(0x4B4),
            this.byte_add(0x4B8),
            GPtr::from_raw(&mut fwd_speed).cast(),
            this.byte_add(0x808),
            this.byte_add(0x80C),
            u32::from(rd::<u8>(this, 0x961)),
            cheat_type,
        );
        accel = X87::from_f64_exact(a).fdiv_m(rd::<f32>(this, 0x94)).to_f32();
    }
    // Ld0: traction
    let h = rd::<GPtr<u8>>(this, 0x384);
    let t = if rd::<u8>(this, 0x36) & 0xF8 == 0x18 {
        ld(this, 0x8A0)
            .fmul_m(rd::<f32>(h, 0x28))
            .fmul_m(g::<f32>(0xC1_BFEC).read())
    } else {
        gf(0xC1_BFEC).fmul_m(rd::<f32>(h, 0x28))
    };
    let mut traction = t.fmul_m(0.25).fdiv_m(rd::<f32>(this, 0x94)).to_f32();
    if g::<u8>(CHEAT_PERFECT_HANDLING).read() != 0
        || (rd::<u16>(this, 0x22) == 0x21B && !ld(this, 0x994).fcom(X87::zero()).test_parity(0x44))
    {
        traction = x(traction).fmul_m(4.0).to_f32();
    }
    if find_player_vehicle(-1, 0) != this {
        let cheat = g::<u8>(CHEAT_PERFECT_HANDLING).read() != 0;
        if extra & 1 != 0 || cheat {
            traction = x(traction).fmul_m(f32::from_bits(0x3F99_999A)).to_f32();
            accel = x(accel).fmul_m(f32::from_bits(0x3FB3_3333)).to_f32();
            if extra & 2 != 0 || cheat {
                traction = x(traction).fmul_m(f32::from_bits(0x3FA6_6666)).to_f32();
                accel = x(accel).fmul_m(f32::from_bits(0x3FB3_3333)).to_f32();
            }
        }
    }

    steer_limit(this, fwd_speed, traction);

    let flags = rd::<u32>(this, 0x38C);
    let rear_first = (flags >> 23) & 1 != 0;
    let sp = GPtr::from_raw(speeds.as_mut_ptr()).cast::<u8>();
    let pp = GPtr::from_raw(points.as_mut_ptr()).cast::<u8>();
    let (tb, ab, bb) = (traction.to_bits(), accel.to_bits(), brake.to_bits());
    let front_steer = |this: GPtr<u8>| {
        if rd::<u32>(this, 0x38C) & 0x20 != 0 {
            0xC479_C000
        } else {
            rd::<u32>(this, 0x494)
        }
    };
    if !rear_first {
        process_car_wheel_pair(this, 0, 2, front_steer(this), sp, pp, tb, ab, bb, 1);
    }
    let f = rd::<u32>(this, 0x38C);
    let rear_steer = if f & 0x20 != 0 {
        ld(this, 0x494).fchs().to_f32().to_bits()
    } else if f & 0x40 != 0 {
        rd::<u32>(this, 0x498)
    } else {
        0xC479_C000
    };
    process_car_wheel_pair(this, 1, 3, rear_steer, sp, pp, tb, ab, bb, 0);
    if rear_first {
        process_car_wheel_pair(this, 0, 2, front_steer(this), sp, pp, tb, ab, bb, 1);
    }

    let s = status(this);
    if s == 0 {
        let no_check = rd::<u32>(this, 0x38C) & 0x2_0000 != 0
            || g::<u8>(CHEAT_PERFECT_HANDLING).read() != 0
            || extra & 0x10 != 0;
        process_siren_and_horn(this, u32::from(!no_check));
    } else {
        let horn = rd::<u16>(this, 0x45C);
        if (horn == 0 || horn == 0xFFFF || s == 5) && rd::<u32>(this, 0x514) > 0 {
            wr::<u32>(this, 0x514, rd::<u32>(this, 0x514) - 1);
        }
    }
    if rd::<u16>(this, 0x22) != 0x1D0 {
        v_process_flying_car_stuff(this);
    }
}

/// 0x6B2548..0x6B268E: bus-door timer reuse and the nose-down turn force while changing gear.
fn gear_change_nose_down(this: GPtr<u8>) {
    if rd::<u8>(this, 0x36) & 0xF8 == 0 && rd::<u8>(rd::<GPtr<u8>>(this, 0x384), 0xCC) & 2 == 0 {
        let t = rd::<u32>(this, 0x870);
        if t == 0 {
            wr::<u32>(this, 0x870, 1000);
        } else {
            let dec = ftol32(
                gf(TIME_STEP)
                    .fmul_m(f32::from_bits(0x3CA3_D70A))
                    .fmul_m(f32::from_bits(0x447A_0000)),
            ) as u32;
            wr::<u32>(this, 0x870, t.saturating_sub(dec));
        }
    }
    let one = c(0x3F80_0000);
    let front = !ge(ld(this, 0x7D4), one) || !ge(ld(this, 0x7DC), one);
    let rear = !ge(ld(this, 0x7D8), one) || !ge(ld(this, 0x7E0), one);
    if !(front && rear) {
        return;
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    let minus1 = f32::from_bits(0xBF80_0000);
    let point = CVector::new(
        ld(m, 0x10).fmul_m(minus1).to_f32(),
        ld(m, 0x14).fmul_m(minus1).to_f32(),
        ld(m, 0x18).fmul_m(minus1).to_f32(),
    );
    let mut tm = ld(this, 0x90);
    if ge(tm, c(0x451C_4000)) {
        tm = c(0x451C_4000);
    }
    let k = tm.fmul_m(f32::from_bits(0xBC03_126F));
    let force = CVector::new(
        k.fmul_m(rd::<f32>(m, 0x20)).to_f32(),
        k.fmul_m(rd::<f32>(m, 0x24)).to_f32(),
        k.fmul_m(rd::<f32>(m, 0x28)).to_f32(),
    );
    apply_turn_force(this, force, point);
}

/// 0x6B28D6..0x6B2A34: the player's steer limit from the adhesion (`m_fSteerAngle *= limit`).
fn steer_limit(this: GPtr<u8>, fwd_speed: f32, traction: f32) {
    let gmul = 0x8D_3214;
    if le(gf(gmul), X87::zero()) {
        return;
    }
    let one = c(0x3F80_0000);
    let v = x(fwd_speed);
    let limited = 'l: {
        if le(v, c(0x3C23_D70A)) {
            break 'l None;
        }
        if le(ld(this, 0x7F4), X87::zero()) && le(ld(this, 0x7F8), X87::zero()) {
            break 'l None;
        }
        if rd::<u8>(this, 0x36) & 0xF8 != 0 {
            break 'l None;
        }
        let m = rd::<GPtr<u8>>(this, 0x14);
        // Lcc: speed along the right axis
        let right = ld(this, 0x4C)
            .fmul_m(rd::<f32>(m, 8))
            .fadd(ld(this, 0x48).fmul_m(rd::<f32>(m, 4)))
            .fadd(ld(this, 0x44).fmul_m(rd::<f32>(m, 0)))
            .to_f32();
        // L2c: a CColPoint with only surface A (wheelbase 0x3C) and B (tarmac 1) written
        let mut cp = [0u8; 0x2C];
        cp[0x20] = 0x3C;
        cp[0x23] = 1;
        let adh = X87::from_f64_exact(get_adhesive_limit(
            GPtr::from_addr(SURFACE_INFOS),
            GPtr::from_raw(cp.as_mut_ptr()),
        ));
        let q = adh
            .fmul_m(traction)
            .fmul_m(g::<f32>(gmul).read())
            .fmul_m(4.0)
            .fdiv(v.fmul_m(fwd_speed));
        // La4 = min(q, 1.0) as stored
        let a = if ge(one, q) { q.to_f32() } else { 1.0 };
        let asin = X87::from_f64_exact(sa_core::orig::x87_unary(0x82_1E70, x(a).to_f64()));
        let lim = asin.fdiv(ld(rd::<GPtr<u8>>(this, 0x384), 0xA0).fmul_m(f32::from_bits(0x3C8E_FA35)));
        let steer = ld(this, 0x494);
        let r = x(right);
        if !ge(steer, X87::zero()) {
            if !le(r, c(0x3D4C_CCCD)) {
                break 'l None;
            }
        } else if !le(steer, X87::zero()) && !ge(r, c(0xBD4C_CCCD)) {
            break 'l None;
        }
        if rd::<u8>(this, 0x428) & 0x20 != 0 {
            break 'l None;
        }
        if ge(one, lim) { Some(lim) } else { None }
    };
    let k = limited.unwrap_or(one);
    st(this, 0x494, k.fmul_m(rd::<f32>(this, 0x494)));
}

/// 0x6B2BF5..0x6B2F7D: suspension / roughness shake, the wheel-spin audio filter and the pad shake.
fn shake_and_wheel_spin(this: GPtr<u8>, extra: u32) {
    // Ld0: suspension shake, Lc4: roughness shake, Ld8: wheel-spin rate
    let mut susp = 0.0f32;
    let mut rough = 0.0f32;
    let mut spin = 0.0f32;
    // La4: |moveSpeed|^2 as stored
    let ms2 = sq3(this, 0x44).to_f32();
    for i in 0..4isize {
        let comp = 0x7D4 + 4 * i;
        let change = ld(this, comp + 0x10).fsub_m(rd::<f32>(this, comp)).to_f32();
        if !le(x(change), c(0x3E99_999A)) && extra & 8 == 0 && !le(x(ms2), c(0x3D23_D70B)) {
            get_wheel_status(this.byte_add(0x5A0), i as i32);
            if !le(x(change), x(susp)) {
                susp = change;
            }
        }
        if !ge(ld(this, comp), c(0x3F80_0000)) && rd::<u8>(this, 0x36) & 0xF8 == 0 {
            let r = get_roughness(
                GPtr::from_addr(SURFACE_INFOS),
                u32::from(rd::<u8>(this, 0x747 + 0x2C * i)),
            );
            let rr = xi(r as i32).fmul_m(f32::from_bits(0x3DCC_CCCD));
            if le(x(rough), rr) {
                rough = rr.to_f32();
            }
        }
        let wc = ld(this, comp + 0x20);
        let mut in_air_spin = false;
        if !wc.fcom(X87::zero()).test_parity(0x41) {
            let gas = ld(this, 0x49C);
            if !le(gas, c(0x3F00_0000)) || !ge(gas, c(0xBF00_0000)) {
                let id = usize::from(rd::<u8>(rd::<GPtr<u8>>(this, 0x384), 0));
                let drive = g::<u8>(0xC2_BA50 + id * 0xE0).read();
                in_air_spin = if i == 0 || i == 2 {
                    drive != 0x52
                } else {
                    (i == 1 || i == 3) && drive != 0x46
                };
            }
        }
        if in_air_spin {
            spin = x(spin).fadd_m(g::<f32>(0x8D_349C).read()).to_f32();
        } else if rd::<u32>(this, comp + 0x194) == 1 {
            spin = x(spin).fadd_m(g::<f32>(0x8D_3498).read()).to_f32();
        }
        wr::<u32>(this, comp + 0x10, rd::<u32>(this, comp));
        wr::<u32>(this, comp, 0x3F80_0000);
    }
    let four = rd::<u8>(rd::<GPtr<u8>>(this, 0x384), 0x74) == 0x34;
    let rate = x(spin).fmul_m(if four { 0.25 } else { 0.5 });
    let spin_s = rate.to_f32();
    let rf = if ge(rate, ld(this, 0x4BC)) {
        gf(0x8D_34A0)
    } else {
        gf(0x8D_34A4)
    };
    let p = X87::from_f64_exact(sa_core::orig::x87_binary(
        0x82_2130,
        rf.to_f64(),
        gf(TIME_STEP).to_f64(),
    ));
    let v = c(0x3F80_0000)
        .fsub(p)
        .fmul_m(spin_s)
        .fadd(p.fmul_m(rd::<f32>(this, 0x4BC)));
    st(this, 0x4BC, v);

    if g::<u32>(0xB7_CB84).read() & 0x7FF > 0x320 {
        if !lt(x(rough), c(0x3E94_7AE1)) {
            susp = 0.0;
        }
        rough = 0.0;
    }
    if extra & 8 != 0 {
        return;
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
    let ts = g::<f32>(TIME_STEP).read();
    let (freq, time) = if !le(x(susp), X87::zero()) {
        let mut f = sp.fmul_m(susp).fmul_m(f32::from_bits(0x48C3_5000)).fadd_m(100.0);
        if !ge(c(0x437A_0000), f) {
            f = c(0x437A_0000);
        }
        let fb = ftol32(f) as u8;
        (fb, ftol32(x(ts).fmul_m(20000.0).fdiv(xi(i32::from(fb)))))
    } else {
        let mut f = sp.fmul_m(rough).fmul_m(f32::from_bits(0x48C3_5000)).fadd_m(40.0);
        if !ge(c(0x4316_0000), f) {
            f = c(0x4316_0000);
        }
        let fb = ftol32(f) as u8;
        (fb, ftol32(x(ts).fmul_m(5000.0).fdiv(xi(i32::from(fb)))))
    };
    start_shake(get_pad(0), time as u32, u32::from(freq), 0);
}

/// 0x6B2F9F..0x6B31E0: the hydraulic roll damping torque (slow cars with hydraulics, driven by
/// the special hydraulic data or the player's car-gun stick).
fn hydraulic_roll(this: GPtr<u8>) {
    if rd::<u32>(this, 0x38C) & 0x2_0000 == 0 {
        return;
    }
    if ge(sq3(this, 0x44).fsqrt(), c(0x3E4C_CCCD)) {
        return;
    }
    let s = status(this);
    let half = c(0x3F00_0000);
    let mut go = false;
    if s == 3 {
        let idx = rd::<i8>(this, 0x48B) as i32;
        let rec = (0xC1_CB78i64 + i64::from(idx) * 0x28) as usize;
        let w = |k: usize| x(g::<f32>(rec + 4 * k).read());
        go = (!le(w(0), half) && !le(w(1), half)) || (!le(w(2), half) && !le(w(3), half));
    }
    if !go {
        if s != 0 {
            return;
        }
        let d = rd::<GPtr<u8>>(this, 0x460);
        if d.is_null() || ped_is_player(d) == 0 {
            return;
        }
        let pad = get_pad_from_player(rd::<GPtr<u8>>(this, 0x460));
        let lr = get_car_gun_left_right(pad) as u16 as i16;
        if le(xi(i32::from(lr)).fabs(), c(0x4248_0000)) {
            return;
        }
        let ud = get_car_gun_up_down(pad) as u16 as i16;
        if ge(xi(i32::from(ud)).fabs(), c(0x4248_0000)) {
            return;
        }
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    // Lc4: turn speed along the forward axis
    let tf = ld(this, 0x58)
        .fmul_m(rd::<f32>(m, 0x18))
        .fadd(ld(this, 0x54).fmul_m(rd::<f32>(m, 0x14)))
        .fadd(ld(m, 0x10).fmul_m(rd::<f32>(this, 0x50)))
        .to_f32();
    let ts = gf(TIME_STEP);
    let p1 = X87::from_f64_exact(sa_core::orig::x87_binary(
        0x82_2130,
        gf(0x8D_3210).to_f64(),
        ts.to_f64(),
    ));
    let q = p1.fdiv(x(tf).fmul_m(tf).fmul_m(g::<f32>(0x8D_320C).read()).fadd_m(1.0));
    let p2 = X87::from_f64_exact(sa_core::orig::x87_binary(
        0x82_2130,
        q.to_f64(),
        gf(TIME_STEP).to_f64(),
    ));
    let sp = p2.fmul_m(tf).fsub_m(tf);
    let tm = rd::<f32>(this, 0x90);
    let minus1 = f32::from_bits(0xBF80_0000);
    let (ux, uy, uz) = (
        ld(m, 0x20).fmul_m(minus1),
        ld(m, 0x24).fmul_m(minus1),
        ld(m, 0x28).fmul_m(minus1).to_f32(),
    );
    let fx = sp.fmul(ux).to_f32();
    let fy_reg = sp.fmul(uy);
    let fz = sp.fmul_m(uz).to_f32();
    let force = CVector::new(
        x(fx).fmul_m(tm).to_f32(),
        fy_reg.fmul_m(tm).to_f32(),
        x(fz).fmul_m(tm).to_f32(),
    );
    let mut com = CVector::default();
    let r = multiply3x3(GPtr::from_raw(&mut com).cast(), m, this.byte_add(0xA4));
    let point = CVector::new(
        ld(m, 0).fadd_m(rd::<f32>(r, 0)).to_f32(),
        ld(r, 4).fadd_m(rd::<f32>(m, 4)).to_f32(),
        ld(r, 8).fadd_m(rd::<f32>(m, 8)).to_f32(),
    );
    apply_turn_force(this, force, point);
}

/// 0x6B31E5..0x6B32EC: tractor / trailer link maintenance.
fn tow_links(this: GPtr<u8>) {
    let tractor = rd::<GPtr<u8>>(this, 0x4C4);
    if !tractor.is_null() {
        if rd::<u8>(this, 0x36) & 0xF8 != 0x50 {
            v_break_tow_link(this);
            return;
        }
        let mut moved = false;
        if any_nonzero3(tractor, 0x44) || !le(sq3(this, 0x44), c(0x3C23_D70B)) {
            update_tractor_link(tractor, 0, 0);
            moved = true;
        }
        update_trailer_link(this, 0, 0);
        let tractor = rd::<GPtr<u8>>(this, 0x4C4);
        if !tractor.is_null() && rd::<u8>(this, 0x36) & 0xF8 == 0x50 {
            if moved {
                update_tractor_link(tractor, 0, 1);
            }
            update_trailer_link(this, 0, 1);
        }
        return;
    }
    let trailer = rd::<GPtr<u8>>(this, 0x4C8);
    if trailer.is_null() {
        return;
    }
    if rd::<u8>(trailer, 0x36) & 0xF8 != 0x50 {
        clean_up_old_reference(trailer, this.byte_add(0x4C8));
        wr::<u32>(this, 0x4C8, 0);
    } else if rd::<GPtr<u8>>(trailer, 0x4C4) == this {
        remove_from_moving_list(this);
        add_to_moving_list(this);
    }
}

/// 0x6B32EC..0x6B362F: frozen speeds (+0x40 bits 2 and 3) and the parked-car rest check.
fn rest_checks(this: GPtr<u8>, skip: bool) {
    let f = rd::<u32>(this, 0x40);
    if f & 4 != 0 && f & 8 != 0 {
        for off in (0x44isize..0x74).step_by(4) {
            wr::<u32>(this, off, 0);
        }
        return;
    }
    if skip {
        return;
    }
    if ne(ld(this, 0x49C), X87::zero()) && rd::<u8>(this, 0x36) & 0xF8 != 0x28 {
        return;
    }
    let lim = c(0x3BA3_D70A);
    if (0..3).any(|k| ge(ld(this, 0x44 + 4 * k).fabs(), lim)) {
        return;
    }
    if !le(ld(this, 0xD8), X87::zero()) && rd::<GPtr<u8>>(this, 0xDC) == find_player_ped(-1) {
        return;
    }
    let sub = rd::<u32>(this, 0x594);
    if sub == 4 && ne(ld(this, 0x994), X87::zero()) {
        return;
    }
    if sub == 3 && ne(ld(this, 0x998), X87::zero()) {
        return;
    }
    let w = rd::<u16>(this, 0x86C);
    if ne(xi(i32::from(w)), X87::zero()) && w != rd::<u16>(this, 0x86E) {
        return;
    }
    if rd::<u32>(this, 0x40) & 0x100 != 0 {
        return;
    }
    let tractor = rd::<GPtr<u8>>(this, 0x4C4);
    if !tractor.is_null() && any_nonzero3(tractor, 0x44) {
        return;
    }
    if (0..4).all(|i| ge(ld(this, 0x7E4 + 4 * i), c(0x3F80_0000))) {
        return;
    }
    if rd::<u16>(this, 0x22) == 0x21B
        || (g::<u8>(CHEAT_CARS_ON_WATER).read() != 0 && wheel_on_shallow_water(this, 0x7E4))
    {
        wr::<u8>(this, 0xB8, 0);
        return;
    }
    wr::<u32>(this, 0x44, 0);
    wr::<u32>(this, 0x48, 0);
    wr::<u32>(this, 0x4C, 0);
    wr::<u32>(this, 0x58, 0);
}

pub const HOOKS: &[HookReg] = &[HOOK_AUTOMOBILE_PROCESS_CONTROL];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
