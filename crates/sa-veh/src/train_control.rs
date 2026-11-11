//! WRITER-VEH-BIG (batch `vehbig-1`): `CTrain::ProcessControl`, the rail driver of every train
//! carriage (and the free physics of a derailed one). Ported from the 1.0 US bytes; gta-reversed
//! as a structural guide only (the node walk, the station flags and the buoyancy force slot differ;
//! the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;
use sa_math::x87::X87;
use sa_world::world::common::find_player_ped;

use crate::common::{g, rand_at, rd, wr, x, xi};

orig! {
    /// `CAEVehicleAudioEntity::Service()` (0x502280)
    fn audio_service(a: GPtr<u8>) = thiscall 0x502280 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPlayerPed::GetPadFromPlayer()` (0x609560)
    fn get_pad_from_player(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x609560 => unported;
    /// `CTrain::FindNumCarriagesPulled()` (0x6F5CD0)
    fn find_num_carriages_pulled(t: GPtr<u8>) -> u32 = thiscall 0x6F5CD0 => crate::train::find_num_carriages_pulled;
    /// `CPad::GetAccelerate()` (0x5403F0): int16 in AX
    fn get_accelerate(pad: GPtr<u8>) -> u32 = thiscall 0x5403F0 => unported;
    /// `CPad::GetBrake()` (0x540080): int16 in AX
    fn get_brake(pad: GPtr<u8>) -> u32 = thiscall 0x540080 => unported;
    /// `CTrain::FindMaximumSpeedToStopAtStations(float*)` (0x6F5BA0)
    fn find_maximum_speed_to_stop_at_stations(t: GPtr<u8>, out: GPtr<f32>) -> u8 = thiscall 0x6F5BA0 => crate::mvpb_a1::find_maximum_speed_to_stop_at_stations;
    /// `CGameLogic::CalcDistanceToForbiddenTrainCrossing(CVector, CVector, bool, CVector&)` (0x4418E0), ST0
    fn calc_distance_to_forbidden_train_crossing(px: f32, py: f32, pz: f32, mx: f32, my: f32, mz: f32, ignore: u8, out: GPtr<u8>) -> f64 = cdecl 0x4418E0 => calc_distance_native;
    /// `CPlaceable::GetTopDirection(CVector*)`-style accessor (0x41CCB0)
    fn get_top_direction(e: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x41CCB0 => sa_world::world::entity::get_top_direction;
    /// `min(float, float)` (0x404330), ST0
    fn min_f(a: f32, b: f32) -> f64 = cdecl 0x404330 => min_native;
    /// `max(float, float)` (0x420800), ST0
    fn max_f(a: f32, b: f32) -> f64 = cdecl 0x420800 => max_native;
    /// `CPad::StartShake(int16, uint8, uint32)` (0x53F920)
    fn start_shake(pad: GPtr<u8>, time: u32, freq: u32, delay: u32) = thiscall 0x53F920 => unported;
    /// `CCamera::CamShake(float, float, float, float)` (0x50A9F0) on `TheCamera` (0xB6F028)
    fn cam_shake(cam: GPtr<u8>, strength: f32, px: f32, py: f32, pz: f32) = thiscall 0x50A9F0 => unported;
    /// `CTrainNode::GetPosn()` (0x6F5440, hidden return pointer)
    fn train_node_get_posn(n: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x6F5440 => node_posn_native;
    /// `VectorSub(CVector*, const CVector*, const CVector*)` (0x40FE60)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => sa_world::world::mvpa_math::vector_sub;
    /// `CVector::Normalise()` (0x59C910)
    fn vector_normalise(v: GPtr<u8>) = thiscall 0x59C910 => normalise_native;
    /// `CTrain::IsInTunnel()` (0x6F6320)
    fn is_in_tunnel(t: GPtr<u8>) -> u8 = thiscall 0x6F6320 => unported;
    /// `CPhysical::ProcessControl()` (0x5485E0)
    fn physical_process_control(v: GPtr<u8>) = thiscall 0x5485E0 => sa_world::physical::attach::process_control;
    /// `CTrain::RemoveRandomPassenger()` (0x6F6850)
    fn remove_random_passenger(t: GPtr<u8>) = thiscall 0x6F6850 => unported;
    /// `CTrain::AddNearbyPedAsRandomPassenger()` (0x6F8170)
    fn add_nearby_ped_as_random_passenger(t: GPtr<u8>) = thiscall 0x6F8170 => unported;
    /// `CAEVehicleAudioEntity::AddAudioEvent(eAudioEvents, float)` (0x4F6420)
    fn add_audio_event(a: GPtr<u8>, event: u32, volume: f32) = thiscall 0x4F6420 => unported;
    /// `CrossProduct(CVector*, const CVector*, const CVector*)` (0x59C730)
    fn cross_product(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C730 => cross_native;
    /// `CTrainNode::GetLightingFromCollision()` (0x6F5F80): the lighting byte in AL
    fn get_lighting_from_collision(n: GPtr<u8>) -> u8 = thiscall 0x6F5F80 => crate::train2::get_lighting_from_collision;
    /// `tColLighting::GetCurrentLighting(float)` (0x59F0C0, by-value lighting byte), ST0
    fn get_current_lighting(l: u8, scale: f32) -> f64 = cdecl 0x59F0C0 => current_lighting_native;
    /// `CPhysical::ApplyMoveSpeed()` (0x542DD0)
    fn apply_move_speed(v: GPtr<u8>) = thiscall 0x542DD0 => sa_world::physical::forces::apply_move_speed;
    /// `CMatrix::UpdateRW(RwMatrix*)` (0x59AD70)
    fn matrix_update_rw(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD70 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40)
    fn simple_transform_update_rw_matrix(t: GPtr<u8>, rw: GPtr<u8>) -> u32 = thiscall 0x54EF40 => sa_world::world::mvpa_math::simple_transform_update_rw_matrix;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    fn update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => sa_world::world::entity::update_rw_frame;
    /// `CPhysical::RemoveAndAdd()` (0x542560)
    fn remove_and_add(e: GPtr<u8>) = thiscall 0x542560 => sa_world::physical::base::remove_and_add;
    /// `CTrain::MarkSurroundingEntitiesForCollisionWithTrain(CVector, float, CEntity*, bool)` (0x6F6640)
    fn mark_surrounding_entities_for_collision_with_train(px: f32, py: f32, pz: f32, radius: f32, train: GPtr<u8>, only: u32) = cdecl 0x6F6640 => crate::train2::mark_surrounding_entities_for_collision_with_train;
    /// `CCarCtrl::ScanForPedDanger(CVehicle*)` (0x42CE40)
    fn scan_for_ped_danger(v: GPtr<u8>) = cdecl 0x42CE40 => sa_pop::carctrl4::scan_for_ped_danger;
    /// `FUN_00424100(float x, float y)` (0x424100): `xor al, al; ret`
    fn area_check_stub(px: u32, py: u32) -> u8 = cdecl 0x424100 => area_check_stub_native;
    /// `CPhysical::SkipPhysics()` (0x5433B0)
    fn skip_physics(v: GPtr<u8>) = thiscall 0x5433B0 => sa_world::physical::base::skip_physics;
    /// `cBuoyancy::ProcessBuoyancy(CPhysical*, float, CVector*, CVector*)` (0x6C3EF0) on `mod_Buoyancy` (0xC1C890)
    fn process_buoyancy(b: GPtr<u8>, e: GPtr<u8>, buoyancy: f32, turn: GPtr<u8>, force: GPtr<u8>) -> u8 = thiscall 0x6C3EF0 => unported;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0)
    fn apply_move_force(e: GPtr<u8>, force: CVector) = thiscall 0x5429F0 => sa_world::physical::forces::apply_move_force;
    /// `CPhysical::ApplyTurnForce(CVector, CVector)` (0x542A50)
    fn apply_turn_force(e: GPtr<u8>, force: CVector, point: CVector) = thiscall 0x542A50 => sa_world::physical::forces::apply_turn_force;
    /// `CEntity::SetIsStatic(bool)` (vtable +0x10)
    fn v_set_is_static(e: GPtr<u8>, s: u32) = thiscall vslot 0x10 => unported;
}

/// Phases B/C side of `CGameLogic::CalcDistanceToForbiddenTrainCrossing` 0x4418E0.
#[allow(clippy::too_many_arguments)]
pub fn calc_distance_native(
    px: f32,
    py: f32,
    pz: f32,
    mx: f32,
    my: f32,
    mz: f32,
    ignore: u8,
    out: GPtr<u8>,
) -> f64 {
    sa_world::world::mvpd_w2::game_logic_calc_distance_to_forbidden_train_crossing(
        px, py, pz, mx, my, mz, ignore, out,
    )
    .to_f64()
}

/// Phases B/C side of `min` 0x404330.
pub fn min_native(a: f32, b: f32) -> f64 {
    if x(a).fcom(x(b)).test_parity(0x05) {
        f64::from(b)
    } else {
        f64::from(a)
    }
}

/// Phases B/C side of `max` 0x420800.
pub fn max_native(a: f32, b: f32) -> f64 {
    if x(a).fcom(x(b)).test_any(0x41) {
        f64::from(b)
    } else {
        f64::from(a)
    }
}

/// Phases B/C side of `CTrainNode::GetPosn` 0x6F5440: the int16 position times 1/8.
pub fn node_posn_native(n: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> {
    for k in 0..3isize {
        wr::<f32>(
            out,
            4 * k,
            xi(i32::from(rd::<i16>(n, 2 * k))).fmul_m(0.125).to_f32(),
        );
    }
    out
}

/// Phases B/C side of `CVector::Normalise` 0x59C910.
pub fn normalise_native(v: GPtr<u8>) {
    let [a, b, c] = sa_math::vector::normalise([rd::<f32>(v, 0), rd::<f32>(v, 4), rd::<f32>(v, 8)]);
    wr::<f32>(v, 0, a);
    wr::<f32>(v, 4, b);
    wr::<f32>(v, 8, c);
}

/// Phases B/C side of `CrossProduct` 0x59C730.
pub fn cross_native(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> {
    let r = sa_math::vector::cross_product(&rv(a, 0), &rv(b, 0));
    wr::<f32>(out, 0, r.x);
    wr::<f32>(out, 4, r.y);
    wr::<f32>(out, 8, r.z);
    out
}

/// Phases B/C side of `tColLighting::GetCurrentLighting` 0x59F0C0.
pub fn current_lighting_native(l: u8, scale: f32) -> f64 {
    sa_world::collision::mvpb_coll::col_lighting_get_current_lighting(l, scale).to_f64()
}

/// Phases B/C side of the stub 0x424100 (`xor al, al; ret`).
pub fn area_check_stub_native(_px: u32, _py: u32) -> u8 {
    0
}

/// `CTimer::ms_fTimeStep`
const TIME_STEP: usize = 0xB7_CB5C;

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

fn ts() -> f32 {
    g::<f32>(TIME_STEP).read()
}

fn pow(base: X87, e: X87) -> X87 {
    st0(sa_core::orig::x87_binary(0x82_2130, base.to_f64(), e.to_f64()))
}

/// `fcomp` + `test ah, 0x41` + `jne` taken: `a <= b` or unordered.
fn le(a: X87, b: X87) -> bool {
    a.fcom(b).test_any(0x41)
}

/// `fcomp` + `test ah, 5` + `jp` taken: `a >= b` or unordered.
fn ge(a: X87, b: X87) -> bool {
    a.fcom(b).test_parity(0x05)
}

/// `|v|` as the `fcomp 0; test ah, 5; jp; fchs` sequences compute it.
fn absx(v: X87) -> X87 {
    if ge(v, X87::zero()) { v } else { v.fchs() }
}

fn pos_ptr(this: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(this, 0x14);
    if m.is_null() {
        this.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
}

/// `GetHeading()` inlined: `atan2(-forward.x, forward.y)` (FPATAN) with a matrix, else +0x10.
fn heading(this: GPtr<u8>) -> X87 {
    let m = rd::<GPtr<u8>>(this, 0x14);
    if m.is_null() {
        ld(this, 0x10)
    } else {
        ld(m, 0x10).fchs().fpatan(ld(m, 0x14))
    }
}

/// A track node (10 bytes: int16 x, y, z in 1/8 units, uint16 distance in 1/3 units, lighting).
fn node(base: GPtr<u8>, i: u32) -> GPtr<u8> {
    base.byte_add(10 * i as isize)
}

/// `fild word [node + 6]; fmul 1/3` (the zero-extended distance).
fn node_dist(n: GPtr<u8>) -> X87 {
    xi(i32::from(rd::<u16>(n, 6))).fmul_m(f32::from_bits(0x3EAA_AAAB))
}

/// `fild word [node + 2k]; fmul 0.125`.
fn node_coord(n: GPtr<u8>, k: isize) -> X87 {
    xi(i32::from(rd::<i16>(n, 2 * k))).fmul_m(0.125)
}

/// Every carriage from `this` along +0x5D4.
fn for_each_carriage(this: GPtr<u8>, mut f: impl FnMut(GPtr<u8>)) {
    let mut t = this;
    loop {
        f(t);
        t = rd::<GPtr<u8>>(t, 0x5D4);
        if t.is_null() {
            break;
        }
    }
}

/// `CTrain::ProcessControl()` (0x6F86A0).
///
/// Clears +0x429 bit 5, services the audio entity; nothing more in the model viewer (0xBA6728).
/// Derailed trains (+0x5B9 bit 0) run the fake-physics counter (status 0: limits 0.003 / 0.0009 /
/// 0.005, else 0.006 / 0.0015 / 0.015; not when stuck), `CPhysical::ProcessControl` and buoyancy
/// (move force from the buoyancy output, the damping `pow(1 - force.z / (max(timeStep, 0.01) *
/// mass * 0.008) * 0.05, timeStep)`, then the move and turn forces). On rails: the front carriage
/// (+0x5B8 bit 3) reads the pad (player) or the station schedule (AI: stop for 10 / 20 s on Streak
/// trains, open the doors (+0x5B8 bits 1/2, passenger state +0x5CA on every carriage), leave after
/// 8 s, 12 s later the next stop from `FindMaximumSpeedToStopAtStations`) and turns the speed
/// difference into gas / brake (x30, at most 1, x255); forbidden crossings within 230 limit the gas
/// by direction and brake 512; speed += `gas / 256 * timeStep * 0.002 / (carriages + 3)`, minus the
/// brake share, times `pow(0.99975, timeStep)`, signs by the direction bit; the player's train
/// shakes the pad and camera above 0.95 and derails every carriage above 1.0 on a sharp curve
/// outside tunnels. Other carriages follow the previous one (`dist = prev + length`) or roll on
/// (`speed * pow(0.99, timeStep)`); Streak carriages at a station run the passenger exchange
/// (`rand() & 3` at 0x6F8ECD / 0x6F8F36). The rail distance wraps to the track length; the node
/// index walks (audio event 0x64 per node, index `% nodes` kept as int16 even when -1); position,
/// orientation (forward between the front and back node points, right = forward x up, up = right x
/// forward), lighting, move speed (`delta / timeStep`, clamped to ±2) and turn speed (heading
/// delta wrapped to ±pi / timeStep, clamped to ±0.1; a train derailed this frame gets
/// `ApplyMoveSpeed` backwards and the physics instead) follow, then the RW matrix, frame, sectors,
/// entity flags, moving speed, the collision marking ahead of the end carriages and the ped danger
/// scan.
#[hook(addr = 0x6f86a0, conv = thiscall, name = "CTrain::ProcessControl", this = "CTrain",
       expect = "83 EC 7C 53 55 56",
       route = "split", hazards = "x87-transcendental,heap-writes",
       recs = "driving",
       note = "FPATAN headings via X87::fpatan; pow via orig::x87_binary; rand sites 0x6F8ECD and 0x6F8F36")]
pub fn train_process_control(this: GPtr<u8>) {
    wr::<u8>(this, 0x429, rd::<u8>(this, 0x429) & 0xDF);
    audio_service(this.byte_add(0x138));
    if g::<u8>(0xBA_6728).read() != 0 {
        return;
    }
    // L48..L40: old position, L68: old heading
    let old = rv(pos_ptr(this), 0);
    let old_heading = heading(this).to_f32();
    let track = usize::from(rd::<u8>(this, 0x5C0)) * 4;
    // L7c: track length, L70: nodes, L6c: node count
    let total = g::<f32>(0xC3_7FEC + track).read();
    let nodes = g::<GPtr<u8>>(0xC3_8024 + track).read();
    let count = i32::from(g::<i16>(0xC3_8014 + track).read());
    if rd::<u8>(this, 0x5B9) & 1 != 0 {
        derailed(this, old);
        return;
    }
    let bl = rd::<u8>(this, 0x5B8);
    if bl & 8 != 0 {
        front_carriage(this, count, nodes);
    } else {
        let prev = rd::<GPtr<u8>>(this, 0x5D0);
        let d = if !prev.is_null() {
            wr::<u32>(this, 0x5A4, rd::<u32>(prev, 0x5A4));
            ld(prev, 0x5A8).fadd_m(rd::<f32>(this, 0x5AC))
        } else {
            let p = pow(
                X87::from_f64_exact(f64::from_bits(0x3FEF_AE14_8000_0000)),
                x(ts()),
            );
            let s = p.fmul_m(rd::<f32>(this, 0x5A4));
            wr::<f32>(this, 0x5A4, s.to_f32());
            s.fmul_m(ts()).fadd_m(rd::<f32>(this, 0x5A8))
        };
        wr::<f32>(this, 0x5A8, d.to_f32());
        if bl & 1 != 0 && bl & 2 != 0 && rd::<u16>(this, 0x22) == 0x23A && bl & 0x20 == 0 {
            passengers(this);
        }
    }

    // wrap the rail distance
    let tot = x(total);
    if !ge(ld(this, 0x5A8), X87::zero()) {
        let mut v = ld(this, 0x5A8);
        loop {
            v = v.fadd_m(total);
            if ge(v, X87::zero()) {
                break;
            }
        }
        wr::<f32>(this, 0x5A8, v.to_f32());
    }
    if !ld(this, 0x5A8).fcom(tot).test_any(1) {
        let mut v = ld(this, 0x5A8);
        loop {
            v = v.fsub_m(total);
            if v.fcom(tot).test_any(1) {
                break;
            }
        }
        wr::<f32>(this, 0x5A8, v.to_f32());
    }
    on_rails(this, old, old_heading, total, nodes, count);
}

/// 0x6F874C..0x6F8E20: the front carriage's speed control.
fn front_carriage(this: GPtr<u8>, count: i32, nodes: GPtr<u8>) {
    let mut pad = get_pad(0);
    let d = rd::<GPtr<u8>>(this, 0x460);
    if !d.is_null() && ped_is_player(d) != 0 {
        pad = get_pad_from_player(rd::<GPtr<u8>>(this, 0x460));
    }
    // L64: carriages pulled
    let n = find_num_carriages_pulled(this);
    let dl = rd::<u8>(this, 0x5B8);
    if dl & 0x40 == 0 {
        wr::<f32>(this, 0x5A4, ld(this, 0x5A4).fchs().to_f32());
    }
    if rd::<u8>(this, 0x36) & 0xF8 == 0 {
        let s = ld(this, 0x5A4);
        if !ge(absx(s), c(0x3A83_126F)) {
            wr::<u32>(this, 0x5B4, 0);
            let a = i32::from(get_accelerate(pad) as u16 as i16);
            let b = i32::from(get_brake(pad) as u16 as i16);
            wr::<f32>(this, 0x5B0, xi(a - b).to_f32());
        } else if !le(ld(this, 0x5A4), X87::zero()) {
            let b = i32::from(get_brake(pad) as u16 as i16);
            wr::<f32>(this, 0x5B4, xi(b).to_f32());
            let a = i32::from(get_accelerate(pad) as u16 as i16);
            wr::<f32>(this, 0x5B0, xi(a).to_f32());
        } else {
            let b = i32::from(get_brake(pad) as u16 as i16);
            wr::<f32>(this, 0x5B0, xi(-b).to_f32());
            let a = i32::from(get_accelerate(pad) as u16 as i16);
            wr::<f32>(this, 0x5B4, xi(a).to_f32());
        }
    } else {
        station_schedule(this, dl);
    }
    if rd::<u8>(this, 0x5B9) & 2 != 0 {
        forbidden_crossings(this);
    }
    let nf = xi(n.wrapping_add(3) as i32);
    let v = ld(this, 0x5B0)
        .fmul_m(f32::from_bits(0x3B80_0000))
        .fmul_m(ts())
        .fmul_m(f32::from_bits(0x3B03_126F))
        .fdiv(nf)
        .fadd_m(rd::<f32>(this, 0x5A4))
        .to_f32();
    wr::<f32>(this, 0x5A4, v);
    if ld(this, 0x5B4).fcom(X87::zero()).test_parity(0x44) {
        let b = ld(this, 0x5B4)
            .fmul_m(f32::from_bits(0x3B80_0000))
            .fmul_m(ts())
            .fmul_m(f32::from_bits(0x3BC4_9BA6))
            .fdiv(nf)
            .to_f32();
        if !ge(absx(x(v)), x(b)) {
            wr::<u32>(this, 0x5A4, 0);
        } else if ge(x(v), X87::zero()) {
            wr::<f32>(this, 0x5A4, x(v).fsub_m(b).to_f32());
        } else {
            wr::<f32>(this, 0x5A4, x(v).fadd_m(b).to_f32());
        }
    }
    let p = pow(
        X87::from_f64_exact(f64::from_bits(0x3FEF_FDF3_C000_0000)),
        x(ts()),
    );
    let s = p.fmul_m(rd::<f32>(this, 0x5A4));
    wr::<f32>(this, 0x5A4, s.to_f32());
    if rd::<u8>(this, 0x5B8) & 0x40 == 0 {
        wr::<f32>(this, 0x5A4, s.fchs().to_f32());
    }
    wr::<f32>(
        this,
        0x5A8,
        x(ts())
            .fmul_m(rd::<f32>(this, 0x5A4))
            .fadd_m(rd::<f32>(this, 0x5A8))
            .to_f32(),
    );
    if rd::<u8>(this, 0x36) & 0xF8 != 0 {
        return;
    }
    let lim = g::<f32>(0x8D_4944).read();
    if !le(absx(ld(this, 0x5A4)), x(lim).fmul_m(f32::from_bits(0x3F73_3333))) {
        start_shake(get_pad(0), 300, 0x46, 0);
        let p = rv(pos_ptr(this), 0);
        cam_shake(
            GPtr::from_addr(0xB6_F028),
            f32::from_bits(0x3DCC_CCCD),
            p.x,
            p.y,
            p.z,
        );
    }
    if le(absx(ld(this, 0x5A4)), x(g::<f32>(0x8D_4944).read())) {
        return;
    }
    // the curve test over the last three nodes
    let cur = u32::from(rd::<u16>(this, 0x5A0));
    let mut p1 = cur as i32 - 1;
    if p1 < 0 {
        p1 = count;
    }
    let mut p2 = p1 - 1;
    if p2 < 0 {
        p2 = count;
    }
    let (n0, n1, n2) = (node(nodes, cur), node(nodes, p1 as u32), node(nodes, p2 as u32));
    let mut t1 = CVector::default();
    let mut t2 = CVector::default();
    let mut a = CVector::default();
    let ap = GPtr::from_raw(&mut a).cast::<u8>();
    let r1 = train_node_get_posn(n1, GPtr::from_raw(&mut t1).cast());
    let r0 = train_node_get_posn(n0, GPtr::from_raw(&mut t2).cast());
    vector_sub(ap, r0, r1);
    let mut b = CVector::default();
    let bp = GPtr::from_raw(&mut b).cast::<u8>();
    let r2 = train_node_get_posn(n2, GPtr::from_raw(&mut t2).cast());
    let r1 = train_node_get_posn(n1, GPtr::from_raw(&mut t1).cast());
    vector_sub(bp, r1, r2);
    vector_normalise(ap);
    vector_normalise(bp);
    let (a, b) = (rv(ap, 0), rv(bp, 0));
    let dot = x(b.z)
        .fmul_m(a.z)
        .fadd(x(b.x).fmul_m(a.x))
        .fadd(x(b.y).fmul_m(a.y));
    if ge(dot, x(g::<f32>(0x8D_4940).read())) {
        return;
    }
    let mut in_tunnel = false;
    let mut t = this;
    while !in_tunnel {
        in_tunnel = is_in_tunnel(t) != 0;
        t = rd::<GPtr<u8>>(t, 0x5D4);
        if t.is_null() {
            break;
        }
    }
    if in_tunnel {
        return;
    }
    for_each_carriage(this, |t| {
        wr::<u32>(t, 0x40, rd::<u32>(t, 0x40) & 0xFFFD_FFFB);
        wr::<u8>(t, 0x5B9, rd::<u8>(t, 0x5B9) | 1);
        v_set_is_static(t, 0);
    });
    physical_process_control(this);
}

/// 0x6F8871..0x6F8A04: the AI front carriage's station schedule and the gas / brake from it.
fn station_schedule(this: GPtr<u8>, dl: u8) {
    let now = g::<u32>(0xB7_CB84).read();
    // L78: the cruise speed as a float
    let cruise = xi(i32::from(rd::<u8>(this, 0x3D0))).to_f32();
    let base: u32 = if rd::<u8>(this, 0x5B9) & 4 != 0 {
        20000
    } else {
        10000
    };
    let dt = now.wrapping_sub(rd::<u32>(this, 0x5BC));
    let target = if dt < base {
        if dl & 2 == 0 {
            for_each_carriage(this, |t| {
                wr::<u8>(t, 0x5B8, rd::<u8>(t, 0x5B8) | 6);
                wr::<u8>(t, 0x5CA, 0);
            });
        }
        X87::zero()
    } else if dt < base + 8000 {
        if dl & 2 != 0 {
            for_each_carriage(this, |t| {
                wr::<u8>(t, 0x5B8, rd::<u8>(t, 0x5B8) & 0xFB);
                wr::<u8>(t, 0x5CA, 4);
            });
        }
        X87::zero()
    } else if dt < base + 12000 {
        if dl & 2 != 0 {
            for_each_carriage(this, |t| {
                wr::<u8>(t, 0x5B8, rd::<u8>(t, 0x5B8) & 0xFD);
                wr::<u8>(t, 0x5CA, 4);
            });
        }
        x(cruise)
    } else if dl & 0x80 != 0 {
        let mut maxs = 0f32;
        if find_maximum_speed_to_stop_at_stations(this, GPtr::from_raw(&mut maxs)) != 0 {
            wr::<u32>(this, 0x5BC, now);
            X87::zero()
        } else if !x(cruise).fcom(x(maxs)).test_parity(0x05) {
            x(cruise)
        } else {
            x(maxs)
        }
    } else {
        x(cruise)
    };
    let v = target
        .fmul_m(f32::from_bits(0x3CA3_D70A))
        .fsub_m(rd::<f32>(this, 0x5A4));
    let one = c(0x3F80_0000);
    if !le(v, X87::zero()) {
        let mut g30 = v.fmul_m(30.0);
        if g30.fcom(one).test_parity(0x05) {
            g30 = one;
        }
        wr::<u32>(this, 0x5B4, 0);
        wr::<f32>(this, 0x5B0, g30.fmul_m(255.0).to_f32());
    } else {
        let mut b = v.fmul_m(-30.0);
        wr::<u32>(this, 0x5B0, 0);
        if b.fcom(one).test_parity(0x05) {
            b = one;
        }
        wr::<f32>(this, 0x5B4, b.fmul_m(255.0).to_f32());
    }
}

/// 0x6F8A17..0x6F8B29: forbidden train crossings (+0x5B9 bit 1).
fn forbidden_crossings(this: GPtr<u8>) {
    let mut out = [0f32; 3];
    let op = GPtr::from_raw(out.as_mut_ptr()).cast::<u8>();
    let p = rv(pos_ptr(this), 0);
    let v = rv(this, 0x44);
    let d = st0(calc_distance_to_forbidden_train_crossing(
        p.x, p.y, p.z, v.x, v.y, v.z, 1, op,
    ));
    if ge(d, c(0x4366_0000)) {
        return;
    }
    let mut up = [0f32; 3];
    let u = get_top_direction(this, GPtr::from_raw(up.as_mut_ptr())).cast::<u8>();
    let dot = x(out[2])
        .fmul_m(rd::<f32>(u, 8))
        .fadd(x(out[1]).fmul_m(rd::<f32>(u, 4)))
        .fadd(x(out[0]).fmul_m(rd::<f32>(u, 0)));
    let gas = rd::<f32>(this, 0x5B0);
    let r = if le(dot, X87::zero()) {
        max_f(0.0, gas)
    } else {
        min_f(0.0, gas)
    };
    wr::<f32>(this, 0x5B0, st0(r).to_f32());
    let p = rv(pos_ptr(this), 0);
    let v = rv(this, 0x44);
    let d = st0(calc_distance_to_forbidden_train_crossing(
        p.x, p.y, p.z, v.x, v.y, v.z, 0, op,
    ));
    if !ge(d, c(0x4366_0000)) {
        wr::<u32>(this, 0x5B4, 0x4400_0000);
    }
}

/// 0x6F8E9F..0x6F8F89: the Streak passenger exchange at a station.
fn passengers(this: GPtr<u8>) {
    let player_in = |this: GPtr<u8>| rd::<GPtr<u8>>(find_player_ped(-1), 0x58C) == this;
    if rd::<u8>(this, 0x5CA) == 0 {
        if player_in(this) {
            wr::<u8>(this, 0x5CB, rd::<u8>(this, 0x5CB) & 0xF0);
        } else {
            let r = rand_at(0x6F_8ECD) as u8;
            let cl = rd::<u8>(this, 0x5CB);
            let al = (r & 3).wrapping_add(1);
            wr::<u8>(this, 0x5CB, cl ^ ((al ^ cl) & 0xF));
        }
        wr::<u8>(this, 0x5CA, 1);
    }
    if rd::<u8>(this, 0x5CA) == 1 {
        remove_random_passenger(this);
        if rd::<u8>(this, 0x5CB) & 0xF == 0 {
            wr::<u8>(this, 0x5CA, 2);
        }
    }
    if rd::<u8>(this, 0x5CA) == 2 {
        let v = if player_in(this) {
            rd::<u8>(this, 0x5CB) & 0xF
        } else {
            let r = rand_at(0x6F_8F36) as u8;
            let dl = rd::<u8>(this, 0x5CB);
            ((((r & 3).wrapping_add(1)) << 4) & 0xF0) | (dl & 0xF)
        };
        wr::<u8>(this, 0x5CB, v);
        wr::<u8>(this, 0x5CA, 3);
    }
    if rd::<u8>(this, 0x5CA) == 3 && rd::<u8>(this, 0x5B8) & 4 != 0 {
        add_nearby_ped_as_random_passenger(this);
        let al = rd::<u8>(this, 0x5CB);
        if al & 0xF == al >> 4 {
            wr::<u8>(this, 0x5CA, 4);
        }
    }
}

/// 0x6F8FE5..0x6F9A30: the node walk, placement and the per-frame bookkeeping on rails.
fn on_rails(this: GPtr<u8>, old: CVector, old_heading: f32, total: f32, nodes: GPtr<u8>, count: i32) {
    let dist = || ld(this, 0x5A8);
    // the next node's distance as the walk compares it (the track length past the last node)
    let next_of = |idx: u32| -> (X87, u32) {
        let nx = (idx + 1) & 0xFFFF;
        if (nx as i32) < count {
            (node_dist(node(nodes, nx)), nx)
        } else {
            (x(total), 0)
        }
    };
    let mut idx = u32::from(rd::<u16>(this, 0x5A0));
    let mut cur = node_dist(node(nodes, idx));
    let (mut nd, mut next) = next_of(idx);
    loop {
        let advance = !ge(dist(), cur) || !ge(nd, dist());
        if !advance {
            break;
        }
        let ni = if !le(cur, dist()) {
            idx as i32 - 1
        } else {
            idx as i32 + 1
        };
        wr::<u16>(this, 0x5A0, (ni % count) as u16);
        add_audio_event(this.byte_add(0x138), 0x64, 0.0);
        idx = u32::from(rd::<u16>(this, 0x5A0));
        cur = node_dist(node(nodes, idx));
        (nd, next) = next_of(idx);
    }
    // the first point
    let ci = u32::from(rd::<u16>(this, 0x5A0));
    let (na, nb) = (node(nodes, ci), node(nodes, next));
    let cur = node_dist(na);
    let mut dd = node_dist(nb).fsub(cur);
    if !ge(dd, X87::zero()) {
        dd = dd.fadd_m(total);
    }
    let t = dist().fsub(cur).fdiv(dd);
    let it = c(0x3F80_0000).fsub(t);
    let p1x = node_coord(na, 0)
        .fmul(it)
        .fadd(node_coord(nb, 0).fmul(t))
        .to_f32();
    let p1y = node_coord(na, 1)
        .fmul(it)
        .fadd(node_coord(nb, 1).fmul(t))
        .to_f32();
    let p1z = node_coord(na, 2).fmul(it).fadd(node_coord(nb, 2).fmul(t));
    // the second point, the carriage length ahead
    let mi = crate::common::model_info(i32::from(rd::<i16>(this, 0x22)));
    let col = rd::<GPtr<u8>>(mi, 0x14);
    let mut d2 = ld(col, 0x10)
        .fsub_m(rd::<f32>(col, 4))
        .fadd_m(rd::<f32>(this, 0x5A8));
    if !le(d2, x(total)) {
        d2 = d2.fsub_m(total);
    }
    let mut idx2 = ci;
    // L6c: the walk's current distance as stored
    let mut cur2 = node_dist(node(nodes, idx2)).to_f32();
    let (mut nd2, mut next2) = next_of(idx2);
    loop {
        let advance = !ge(d2, x(cur2)) || !le(d2, nd2);
        if !advance {
            break;
        }
        idx2 = ((idx2 as i32 + 1) % count) as u32 & 0xFFFF;
        cur2 = node_dist(node(nodes, idx2)).to_f32();
        (nd2, next2) = next_of(idx2);
    }
    let (ma, mb) = (node(nodes, idx2), node(nodes, next2));
    let c2 = node_dist(ma);
    let mut dd2 = node_dist(mb).fsub(c2);
    if !ge(dd2, X87::zero()) {
        dd2 = dd2.fadd_m(total);
    }
    // L78: the second interpolation factor as stored
    let t2 = d2.fsub(c2).fdiv(dd2).to_f32();
    let it2 = c(0x3F80_0000).fsub_m(t2);
    let p2x = node_coord(ma, 0).fmul(it2).fadd(node_coord(mb, 0).fmul_m(t2));
    let p2y = node_coord(mb, 1)
        .fmul_m(t2)
        .fadd(node_coord(ma, 1).fmul(it2))
        .to_f32();
    let p2z = node_coord(mb, 2)
        .fmul_m(t2)
        .fadd(node_coord(ma, 2).fmul(it2))
        .to_f32();
    let m = rd::<GPtr<u8>>(this, 0x14);
    wr::<f32>(m, 0x30, x(p1x).fadd(p2x).fmul_m(0.5).to_f32());
    wr::<f32>(
        rd::<GPtr<u8>>(this, 0x14),
        0x34,
        x(p2y).fadd_m(p1y).fmul_m(0.5).to_f32(),
    );
    wr::<f32>(
        rd::<GPtr<u8>>(this, 0x14),
        0x38,
        x(p2z).fadd(p1z).fmul_m(0.5).to_f32(),
    );
    let col = rd::<GPtr<u8>>(crate::common::model_info(i32::from(rd::<i16>(this, 0x22))), 0x14);
    let h = rd::<GPtr<u8>>(this, 0x384);
    let m = rd::<GPtr<u8>>(this, 0x14);
    wr::<f32>(
        m,
        0x38,
        ld(col, 8)
            .fchs()
            .fadd_m(rd::<f32>(h, 0xBC))
            .fadd_m(rd::<f32>(m, 0x38))
            .to_f32(),
    );
    // L60: forward
    let mut fwd = CVector::new(
        p2x.fsub_m(p1x).to_f32(),
        x(p2y).fsub_m(p1y).to_f32(),
        x(p2z).fsub(p1z).to_f32(),
    );
    let fp = GPtr::from_raw(&mut fwd).cast::<u8>();
    vector_normalise(fp);
    if rd::<u8>(this, 0x5B8) & 0x40 == 0 {
        let f = rv(fp, 0);
        fwd = CVector::new(
            x(f.x).fchs().to_f32(),
            x(f.y).fchs().to_f32(),
            x(f.z).fchs().to_f32(),
        );
    }
    let fp = GPtr::from_raw(&mut fwd).cast::<u8>();
    let mut zup = CVector::new(0.0, 0.0, 1.0);
    let mut tmp = CVector::default();
    let mut right = rv(
        cross_product(
            GPtr::from_raw(&mut tmp).cast(),
            fp,
            GPtr::from_raw(&mut zup).cast(),
        ),
        0,
    );
    let rp = GPtr::from_raw(&mut right).cast::<u8>();
    vector_normalise(rp);
    let mut tmp2 = CVector::default();
    let up = rv(cross_product(GPtr::from_raw(&mut tmp2).cast(), rp, fp), 0);
    let right = rv(rp, 0);
    let fwd = rv(fp, 0);
    let m = rd::<GPtr<u8>>(this, 0x14);
    for (off, v) in [(0isize, right), (0x20, up), (0x10, fwd)] {
        wr::<f32>(rd::<GPtr<u8>>(this, 0x14), off, v.x);
        wr::<f32>(rd::<GPtr<u8>>(this, 0x14), off + 4, v.y);
        wr::<f32>(rd::<GPtr<u8>>(this, 0x14), off + 8, v.z);
    }
    let _ = m;
    let la = st0(get_current_lighting(get_lighting_from_collision(ma), 0.5)).to_f32();
    let lb = st0(get_current_lighting(get_lighting_from_collision(mb), 0.5));
    wr::<f32>(this, 0x12C, lb.fsub_m(la).fmul_m(t2).fadd_m(la).to_f32());
    // move speed from the position change
    let p = pos_ptr(this);
    let dz = ld(p, 8).fsub_m(old.z).to_f32();
    let inv = c(0x3F80_0000).fdiv_m(ts()).to_f32();
    let dx = ld(p, 0).fsub_m(old.x);
    let dy = ld(p, 4).fsub_m(old.y);
    wr::<f32>(this, 0x44, dx.fmul_m(inv).to_f32());
    wr::<f32>(this, 0x48, x(inv).fmul(dy).to_f32());
    wr::<f32>(this, 0x4C, x(inv).fmul_m(dz).to_f32());
    let mut dh = heading(this).fsub_m(old_heading);
    if !le(dh, c(0x4049_0FDB)) {
        dh = dh.fsub_m(f32::from_bits(0x40C9_0FDB));
    } else if !ge(dh, c(0xC049_0FDB)) {
        dh = dh.fadd_m(f32::from_bits(0x40C9_0FDB));
    }
    let tz = dh.fdiv_m(ts()).to_f32();
    wr::<u32>(this, 0x50, 0);
    wr::<u32>(this, 0x54, 0);
    wr::<f32>(this, 0x58, tz);
    if rd::<u8>(this, 0x5B9) & 1 != 0 {
        let neg = |this: GPtr<u8>, off: isize| {
            for k in 0..3isize {
                wr::<f32>(this, off + 4 * k, ld(this, off + 4 * k).fmul_m(-1.0).to_f32());
            }
        };
        neg(this, 0x44);
        neg(this, 0x50);
        apply_move_speed(this);
        neg(this, 0x44);
        neg(this, 0x50);
        physical_process_control(this);
    } else {
        let clamp = |this: GPtr<u8>, off: isize, lo: u32, hi: u32| {
            let mut v = ld(this, off);
            if v.fcom(c(lo)).test_any(0x41) {
                v = c(lo);
            }
            if v.fcom(c(hi)).test_parity(0x05) {
                v = c(hi);
            }
            wr::<f32>(this, off, v.to_f32());
        };
        for k in 0..3isize {
            clamp(this, 0x44 + 4 * k, 0xC000_0000, 0x4000_0000);
        }
        for k in 0..3isize {
            clamp(this, 0x50 + 4 * k, 0xBDCC_CCCD, 0x3DCC_CCCD);
        }
    }
    let rw = rd::<GPtr<u8>>(this, 0x18);
    if !rw.is_null() {
        let frame = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
        let m = rd::<GPtr<u8>>(this, 0x14);
        if m.is_null() {
            simple_transform_update_rw_matrix(this.byte_add(4), frame);
        } else {
            matrix_update_rw(m, frame);
        }
    }
    update_rw_frame(this);
    remove_and_add(this);
    wr::<u32>(this, 0x1C, (rd::<u32>(this, 0x1C) & 0xFFFF_FFAF) | 0x20);
    let p = pos_ptr(this);
    let (ex, ey, ez) = (
        x(old.x).fsub_m(rd::<f32>(p, 0)),
        x(old.y).fsub_m(rd::<f32>(p, 4)),
        x(old.z).fsub_m(rd::<f32>(p, 8)),
    );
    wr::<f32>(
        this,
        0xD4,
        ex.fmul(ex).fadd(ey.fmul(ey)).fadd(ez.fmul(ez)).fsqrt().to_f32(),
    );
    if rd::<u8>(this, 0x5B8) & 0x18 != 0 {
        let col = rd::<GPtr<u8>>(crate::common::model_info(i32::from(rd::<i16>(this, 0x22))), 0x14);
        let m = rd::<GPtr<u8>>(this, 0x14);
        let my = ld(col, 0x10);
        let (ax, ay, az) = (
            my.fmul_m(rd::<f32>(m, 0x10)).to_f32(),
            my.fmul_m(rd::<f32>(m, 0x14)).to_f32(),
            my.fmul_m(rd::<f32>(m, 0x18)).to_f32(),
        );
        let p = pos_ptr(this);
        let (bx, by) = (x(ax).fadd_m(rd::<f32>(p, 0)), x(ay).fadd_m(rd::<f32>(p, 4)));
        let bz = x(az).fadd_m(rd::<f32>(p, 8)).to_f32();
        let t = ts();
        let tx = x(t).fmul_m(rd::<f32>(this, 0x44));
        let ty = x(t).fmul_m(rd::<f32>(this, 0x48)).to_f32();
        let tz = x(t).fmul_m(rd::<f32>(this, 0x4C)).to_f32();
        let px = tx.fadd(bx).to_f32();
        let py = x(ty).fadd(by).to_f32();
        let pz = x(tz).fadd_m(bz).to_f32();
        mark_surrounding_entities_for_collision_with_train(px, py, pz, 3.0, this, 0);
    }
    if rd::<u8>(this, 0x429) & 0x20 == 0 {
        scan_for_ped_danger(this);
    }
}

/// 0x6F9A31..0x6F9DA7: a derailed train: fake physics, physics and buoyancy.
fn derailed(this: GPtr<u8>, old: CVector) {
    let _ = old;
    if (rd::<u32>(this, 0x1C) >> 4) & 1 == 0 && fake_physics(this) {
        return;
    }
    physical_process_control(this);
    // L24: buoyancy turn point, L48: buoyancy force (over the old position slots)
    let mut turn = CVector::default();
    let mut force = CVector::default();
    let ok = process_buoyancy(
        GPtr::from_addr(0xC1_C890),
        this,
        rd::<f32>(this, 0xA0),
        GPtr::from_raw(&mut turn).cast(),
        GPtr::from_raw(&mut force).cast(),
    );
    if ok == 0 {
        return;
    }
    wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) | 0x800_0000);
    let step = if !c(0x3C23_D70A).fcom(x(ts())).test_any(0x41) {
        f32::from_bits(0x3C23_D70A)
    } else {
        ts()
    };
    let k = x(step)
        .fmul_m(rd::<f32>(this, 0x8C))
        .fmul_m(f32::from_bits(0x3C03_126F))
        .fdivr_m(force.z)
        .fmul_m(f32::from_bits(0x3D4C_CCCD))
        .fsubr_m(1.0);
    let p = pow(k, x(ts()));
    for off in (0x44isize..0x5C).step_by(4) {
        wr::<f32>(this, off, p.fmul_m(rd::<f32>(this, off)).to_f32());
    }
    apply_move_force(this, force);
    apply_turn_force(this, force, turn);
}

/// 0x6F9A40..0x6F9C92: the derailed fake-physics counter; true when the physics were skipped.
fn fake_physics(this: GPtr<u8>) -> bool {
    let (fl, tl, sl) = if rd::<u8>(this, 0x36) & 0xF8 != 0 {
        (c(0x3BC4_9BA6), 0x3AC4_9BA6u32, 0x3C75_C28Fu32)
    } else {
        (c(0x3B44_9BA6), 0x3A6B_EDFB, 0x3BA3_D70A)
    };
    let half = 0.5f32;
    let fx = ld(this, 0x74).fadd_m(rd::<f32>(this, 0x44));
    let fy = ld(this, 0x78).fadd_m(rd::<f32>(this, 0x48));
    let fz = ld(this, 0x7C).fadd_m(rd::<f32>(this, 0x4C)).to_f32();
    wr::<f32>(this, 0x74, fx.fmul_m(half).to_f32());
    wr::<f32>(this, 0x78, fy.fmul_m(half).to_f32());
    wr::<f32>(this, 0x7C, x(fz).fmul_m(half).to_f32());
    let tx = ld(this, 0x50).fadd_m(rd::<f32>(this, 0x80));
    let ty = ld(this, 0x54).fadd_m(rd::<f32>(this, 0x84));
    let tz = ld(this, 0x58).fadd_m(rd::<f32>(this, 0x88)).to_f32();
    wr::<f32>(this, 0x80, tx.fmul_m(half).to_f32());
    wr::<f32>(this, 0x84, ty.fmul_m(half).to_f32());
    wr::<f32>(this, 0x88, x(tz).fmul_m(half).to_f32());
    let sq = |off: isize| {
        let (a, b, cc) = (ld(this, off), ld(this, off + 4), ld(this, off + 8));
        a.fmul(a).fadd(b.fmul(b)).fadd(cc.fmul(cc))
    };
    let f = fl.fmul_m(ts());
    let t = x(f32::from_bits(tl)).fmul_m(ts());
    let rest = !f.fmul(f).fcom(sq(0x74)).test_any(1)
        && !t.fmul(t).fcom(sq(0x80)).test_any(1)
        && !ge(ld(this, 0xD4), x(f32::from_bits(sl)))
        && !(!le(ld(this, 0xD8), X87::zero()) && {
            let e = rd::<GPtr<u8>>(this, 0xDC);
            !e.is_null() && rd::<u8>(e, 0x36) & 7 == 3
        });
    if !rest {
        wr::<u8>(this, 0xB8, 0);
        return false;
    }
    let n = rd::<u8>(this, 0xB8).wrapping_add(1);
    wr::<u8>(this, 0xB8, n);
    if n <= 10 {
        return false;
    }
    let p = pos_ptr(this);
    if area_check_stub(rd::<u32>(p, 0), rd::<u32>(p, 4)) != 0 {
        return false;
    }
    if rd::<u8>(this, 0xB8) > 10 {
        wr::<u8>(this, 0xB8, 10);
    }
    for off in [0x44isize, 0x48, 0x50, 0x4C, 0x54, 0x58] {
        wr::<u32>(this, off, 0);
    }
    skip_physics(this);
    true
}

pub const HOOKS: &[HookReg] = &[HOOK_TRAIN_PROCESS_CONTROL];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
