//! WRITER-MVP-D (helper VEH-2, batch `mvpd-veh-7`): random train and plane generation, vehicle
//! lights, the `CAutomobile` constructor and wheel particles. Ported from the 1.0 US bytes;
//! gta-reversed as a reading aid. Where they differ, the bytes win.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;
use sa_world::world::common::{cam_pos, find_player_coors, find_player_vehicle};

use crate::common::{f32_of, g, jp5, rand_at, rd, wr, x, xi};

orig! {
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// Virtual slot 0 (`scalar deleting destructor`)
    fn entity_vdelete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    /// `CStreaming::RequestModel(int32, int32)` (0x4087E0)
    fn request_model(id: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CStreaming::SetModelIsDeletable(int32)` (0x409C10)
    fn set_model_is_deletable(id: i32) = cdecl 0x409C10 => unported;
    /// `CStreaming::SetModelTxdIsDeletable(int32)` (0x409C70)
    fn set_model_txd_is_deletable(id: i32) = cdecl 0x409C70 => unported;
    /// `CTrain::CreateMissionTrain(CVector, bool, uint32, CTrain**, CTrain**, int32, int32, bool)` (0x6F7550)
    fn create_mission_train(px: f32, py: f32, pz: f32, clockwise: u8, kind: u32, out_first: GPtr<GPtr<u8>>, out_last: GPtr<GPtr<u8>>, node: i32, track: i32, mission: u8) = cdecl 0x6F7550 => crate::mvpb_a5::create_mission_train;
    /// `CTrain::FindMaximumSpeedToStopAtStations(float*)` (0x6F5BA0)
    fn find_maximum_speed_to_stop_at_stations(t: GPtr<u8>, out: GPtr<f32>) -> u8 = thiscall 0x6F5BA0 => crate::mvpb_a1::find_maximum_speed_to_stop_at_stations;
    /// `CTrain::SetTrainSpeed(CTrain*, float)` (0x6F5E20)
    fn set_train_speed(t: GPtr<u8>, speed: f32) = cdecl 0x6F5E20 => crate::train::set_train_speed;
    /// `CCullZones::FindTunnelAttributesForCoors(CVector)` (0x72D9F0)
    fn find_tunnel_attributes_for_coors(px: f32, py: f32, pz: f32) -> u32 = cdecl 0x72D9F0 => unported;
    /// `CPlane::CountPlanesAndHelis()` (0x6CCA50)
    fn count_planes_and_helis() -> i32 = cdecl 0x6CCA50 => crate::heli::count_planes_and_helis;
    /// `CPlane::AreWeInNoPlaneZone()` (0x6CCAA0)
    fn are_we_in_no_plane_zone() -> u8 = cdecl 0x6CCAA0 => unported;
    /// `CPlane::AreWeInNoBigPlaneZone()` (0x6CCBB0)
    fn are_we_in_no_big_plane_zone() -> u8 = cdecl 0x6CCBB0 => unported;
    /// `CPlane::FindPlaneCreationCoors(CVector*, CVector*, float*, float*, bool)` (0x6CD090); the bool
    /// is pushed as a whole stack dword
    fn find_plane_creation_coors(coors: GPtr<f32>, target: GPtr<f32>, orientation: GPtr<f32>, height: GPtr<f32>, big: u32) = cdecl 0x6CD090 => unported;
    /// `CVehicle::operator new(uint32)` (0x6E2D50)
    fn vehicle_new(size: u32) -> GPtr<u8> = cdecl 0x6E2D50 => crate::vehicle2::operator_new;
    /// `CHeli::CHeli(int32, eVehicleCreatedBy)` (0x6C4190)
    fn heli_ctor(h: GPtr<u8>, mi: i32, created_by: u32) -> GPtr<u8> = thiscall 0x6C4190 => crate::air2::heli_ctor;
    /// `CPlane::CPlane(int32, eVehicleCreatedBy)` (0x6C8E20)
    fn plane_ctor(p: GPtr<u8>, mi: i32, created_by: u32) -> GPtr<u8> = thiscall 0x6C8E20 => crate::air2::plane_ctor;
    /// `CPlaceable::SetPosn(const CVector&)` (0x4241C0)
    fn set_posn(e: GPtr<u8>, v: GPtr<f32>) = thiscall 0x4241C0 => sa_world::world::entity::set_posn_vector;
    /// `0x6C4130`: the matrix of a heading (`sin`/`cos` of the argument, no roll or pitch)
    fn set_heading_matrix(v: GPtr<u8>, heading: f32) = thiscall 0x6C4130 => unported;
    /// `CCarCtrl::JoinCarWithRoadSystem(CVehicle*)` (0x42F5A0)
    fn join_car_with_road_system(v: GPtr<u8>) = cdecl 0x42F5A0 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => sa_world::world::cworld2::add;
    /// `CPhysical::SetVelocity(CVector)` (0x441130, the vector by value as bits)
    fn set_velocity(p: GPtr<u8>, vx: u32, vy: u32, vz: u32) = thiscall 0x441130 => sa_world::physical::base::set_velocity;
    /// `thunk_FUN_01567620` (0x41BDD0): the engine-on flag setter (+0x428 bit 4)
    fn set_engine_on(v: GPtr<u8>, on: u32) = thiscall 0x41BDD0 => crate::vehicle2::set_flag_428_bit4;
    /// `CPlane::SetGearUp()` (0x6CAC20)
    fn set_gear_up(p: GPtr<u8>) = thiscall 0x6CAC20 => crate::heli::set_gear_up;
    /// `CWorld::FindUnsuspectingTargetPed(CVector, CVector)` (0x566DA0)
    fn find_unsuspecting_target_ped(px: f32, py: f32, pz: f32, qx: f32, qy: f32, qz: f32) -> GPtr<u8> = cdecl 0x566DA0 => unported;
    /// `CWorld::FindUnsuspectingTargetCar(CVector, CVector)` (0x566C90)
    fn find_unsuspecting_target_car(px: f32, py: f32, pz: f32, qx: f32, qy: f32, qz: f32) -> GPtr<u8> = cdecl 0x566C90 => sa_world::world::cworld2::find_unsuspecting_target_car;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CPedGroups::GetPedsGroup(CPed*)` (0x5F7E80)
    fn get_peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CEventDanger::CEventDanger(CEntity*, float)` (0x4B2600)
    fn event_danger_ctor(ev: GPtr<u8>, e: GPtr<u8>, radius: f32) -> GPtr<u8> = thiscall 0x4B2600 => unported;
    /// `CEventDanger` destructor (0x4B26A0)
    fn event_danger_dtor(ev: GPtr<u8>) = thiscall 0x4B26A0 => unported;
    /// `CPedGroupIntelligence::AddEvent(CEvent*)` (0x5F7470)
    fn pgi_add_event(i: GPtr<u8>, ev: GPtr<u8>) -> u8 = thiscall 0x5F7470 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(grp: GPtr<u8>, ev: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => sa_world::world::cworld::find_player_ped_port;
    /// `CPlayerPed::GetWantedLevel()` (0x41BE60)
    fn get_wanted_level(p: GPtr<u8>) -> i32 = thiscall 0x41BE60 => unported;
    /// `CPool<CVehicle>::GetAt(int32)` (0x41CC10)
    fn vehicle_pool_get_at(pool: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x41CC10 => unported;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    fn get_vehicle_appearance(v: GPtr<u8>) -> u32 = thiscall 0x6D1080 => crate::vehicle2::get_vehicle_appearance;
    /// `CTheScripts::IsPlayerOnAMission()` (0x464D50)
    fn is_player_on_a_mission() -> u8 = cdecl 0x464D50 => unported;
    /// `CGameLogic::LaRiotsActiveHere()` (0x441C10)
    fn la_riots_active_here() -> u8 = cdecl 0x441C10 => unported;
    /// `CVehicle::CVehicle(eVehicleCreatedBy)` (0x6D5F10)
    fn vehicle_ctor(v: GPtr<u8>, created_by: u32) -> GPtr<u8> = thiscall 0x6D5F10 => unported;
    /// `CDamageManager::ResetDamageStatus()` (0x6C20E0)
    fn reset_damage_status(dm: GPtr<u8>) = thiscall 0x6C20E0 => crate::damage::reset_damage_status;
    /// `CVehicle::SetModelIndex(uint32)` (0x6D6A40)
    fn vehicle_set_model_index(v: GPtr<u8>, mi: u32) = thiscall 0x6D6A40 => unported;
    /// `CClumpModelInfo::FillFrameArray(RpClump*, RwFrame**)` (0x4C5440)
    fn fill_frame_array(clump: GPtr<u8>, frames: GPtr<u8>) = cdecl 0x4C5440 => unported;
    /// `CVehicle::AddVehicleUpgrade(int32)` (0x6E3290)
    fn add_vehicle_upgrade(v: GPtr<u8>, model: u32) = thiscall 0x6E3290 => unported;
    /// `cHandlingDataMgr::GetFlyingPointer(uint8)` (0x6F52D0)
    fn get_flying_pointer(mgr: GPtr<u8>, id: u32) -> GPtr<u8> = thiscall 0x6F52D0 => crate::handling::get_flying_pointer;
    /// `CVehicleModelInfo::ChooseVehicleColour(uint8&, uint8&, uint8&, uint8&, int32)` (0x4C8500)
    fn choose_vehicle_colour(mi: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>, d: GPtr<u8>, inc: i32) = thiscall 0x4C8500 => unported;
    /// `RwHelper::GetFirstObject(RwFrame*)` (0x7348C0)
    fn get_first_object(frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x7348C0 => unported;
    /// `CDamageManager::SetDoorStatus(eDoors, eDoorStatus)` (0x6C21C0)
    fn set_door_status(dm: GPtr<u8>, door: i32, status: u32) = thiscall 0x6C21C0 => crate::damage::set_door_status;
    /// `CBouncingPanel::SetPanel(int16, int16, float)` (0x6F4920; the float as bits)
    fn bouncing_panel_set_panel(p: GPtr<u8>, frame: u32, axis: u32, limit: u32) = thiscall 0x6F4920 => crate::door::set_panel;
    /// `CAutomobile::SetupSuspensionLines()` (0x6A65D0)
    fn setup_suspension_lines(a: GPtr<u8>) = thiscall 0x6A65D0 => crate::mvpb_a5::setup_suspension_lines;
    /// `CVehicle::IsLawEnforcementVehicle()` (0x6D2370)
    fn is_law_enforcement_vehicle(v: GPtr<u8>) -> u8 = thiscall 0x6D2370 => unported;
    /// `CMatrix::Attach(RwMatrix*, bool)` (0x59BD10)
    fn matrix_attach(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) = thiscall 0x59BD10 => unported;
    /// `CMatrix::CMatrix(RwMatrix*, bool)` (0x59C050)
    fn matrix_ctor_rw(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) -> GPtr<u8> = thiscall 0x59C050 => unported;
    /// `CMatrix::SetTranslate(CVector)` (0x59AF40, the vector by value as bits)
    fn matrix_set_translate(m: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x59AF40 => unported;
    /// `CMatrix::UpdateRW()` (0x59BBB0)
    fn matrix_update_rw(m: GPtr<u8>) = thiscall 0x59BBB0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CAEVehicleAudioEntity::Initialise(CEntity*)` (0x4F7670)
    fn audio_initialise(a: GPtr<u8>, e: GPtr<u8>) = thiscall 0x4F7670 => unported;
}

/// `fcomp` + `test ah, 0x41` + `jne` taken (C0 or C3: `a <= b` or unordered).
fn le(a: X87, b: X87) -> bool {
    a.fcom(b).test_any(0x41)
}

/// `sqrt(a*a + b*b)` in the operand order the original adds them (`a*a` first on the stack).
fn hyp(a: X87, b: X87) -> X87 {
    a.fmul(a).fadd(b.fmul(b)).fsqrt()
}

// `CTrain` generation state (0xC37FFC..0xC38068).
const GEN_TRACK: usize = 0xC3_7FFC;
const GEN_CONFIG: usize = 0xC3_8000;
const GEN_DIRECTION: usize = 0xC3_8004;
const GEN_NODE: usize = 0xC3_8008;
const GEN_STATUS: usize = 0xC3_800C;
const DISABLE_RANDOM_TRAINS: usize = 0xC3_8010;
const NUM_TRACK_NODES: usize = 0xC3_8014;
const TRACK_NODES: usize = 0xC3_8024;
const GEN_CONFIG_COUNTER: usize = 0xC3_8064;
const STATION_NEARBY: usize = 0xC3_8068;
/// `CTrain::aStationCoors[6]` (stride 0xC).
const STATION_COORS: usize = 0x8D_48F8;
/// The train configurations (stride 0x40, zero-terminated model ids).
const TRAIN_CONFIGS: usize = 0x8D_44F8;

fn track_nodes(track: i32) -> GPtr<u8> {
    g::<GPtr<u8>>((TRACK_NODES as i32 + 4 * track) as u32 as usize).read()
}

fn num_track_nodes(track: i32) -> i32 {
    g::<i32>((NUM_TRACK_NODES as i32 + 4 * track) as u32 as usize).read()
}

/// Steps the generation node one place in the direction `dir` (wrapping) and returns the new
/// node's 2D distance from `(cx, cy)` (stored f32 `dx`, x87 `dy`), as the two search loops at
/// 0x6F7EF3 / 0x6F80A6 do.
fn step_node(track: i32, nodes: GPtr<u8>, dir: bool, cx: f32, cy: f32) -> X87 {
    let mut n = g::<i32>(GEN_NODE).read();
    if dir {
        n -= 1;
        g::<i32>(GEN_NODE).write(n);
        if n < 0 {
            n += num_track_nodes(track);
            g::<i32>(GEN_NODE).write(n);
        }
    } else {
        n = (n + 1) % num_track_nodes(track);
        g::<i32>(GEN_NODE).write(n);
    }
    let node = nodes.byte_add(10 * n as isize);
    let px = xi(i32::from(rd::<i16>(node, 0))).fmul_m(0.125);
    let py = xi(i32::from(rd::<i16>(node, 2))).fmul_m(0.125);
    let dx = f32_of(px.fsub_m(cx));
    let dy = py.fsub_m(cy);
    hyp(x(dx), dy)
}

/// `CTrain::DoTrainGenerationAndRemoval()` (0x6F7900).
///
/// Every 3000 ms (`time / 3000` changed, unsigned) the station flag 0xC38068 is recomputed: any of
/// the six station coordinates closer than 60 (2D) to `FindPlayerCoors(-1)`. Then once per
/// `time / 1` (station nearby: generation radius 100) or `time / 950` (radius 70) period:
///
/// * every non-mission front carriage (vehicle type 6, +0x5B8 bit 3 set, bit 5 clear) whose
///   carriages are all at least 220 (2D) from the camera and none of them the player's vehicle
///   (`FindPlayerVehicle(-1, false)`, called for every carriage after its distance operands) is
///   removed and deleted (`CWorld::Remove` + virtual destructor, carriage by carriage); the vehicle
///   pool is walked from the top, its pointer cached;
/// * nothing more when 0xC38010 (`bDisableRandomTrains`) is set; status 0xC3800C:
/// * status 1: the configuration's models are requested (flag 8) until all are loaded; then, when
///   the generation node is more than 60 from the camera, `CreateMissionTrain(0, direction,
///   config, &first, 0, node, track, false)`. Track 0: speed `rand() % 30 + 15`, halved when the
///   node is closer than 300 to (2222, -1750); other tracks `rand() % 7 + 7`; capped by
///   `FindMaximumSpeedToStopAtStations` (`fild` compared with the f32 it wrote), `SetTrainSpeed`,
///   the integer speed's low byte into +0x3D0. Either way the models are made deletable and the
///   status reset to 0;
/// * status 0 (and no front carriage met in the pool walk): track 0 at a random node; within the
///   radius it is taken when its z + 6 is above the player's, or the player's tunnel attributes
///   have 0x880, or the node's lack 0x800. `rand() & 1` is the direction; a node closer than 170 is
///   stepped until it is not (the node before the last step is kept); config = counter =
///   `(counter + 1) % 8`, status 1, and the kept node is written back only when the final distance
///   is above 220. When the first node is already at least 170 away the "kept node" is the stale
///   z integer the original left in that stack slot (kept: original bug). Otherwise, with status
///   0 and weather region 2 (0xC81314): track 1 at a random node within the radius, the same
///   direction/step search (the stepped node stays), and status 1 with config `8 + rand() % 2` when
///   the final distance is below 220.
#[hook(addr = 0x6F7900, conv = cdecl, name = "CTrain::DoTrainGenerationAndRemoval", expect = "B8 F1 19 76 05",
       unverified = "mvpd-veh-7", route = "L2", hazards = "x87-transcendental,heap-writes",
       recs = "driving",
       note = "fsqrt distances; deletes trains and creates them through CreateMissionTrain (pool and heap); writes +0x3D0 of the new train")]
pub fn train_do_generation_and_removal() {
    let prev = g::<u32>(0xB7_CB78).read();
    let now = g::<u32>(0xB7_CB84).read();
    if now / 3000 != prev / 3000 {
        g::<u8>(STATION_NEARBY).write(0);
        for i in 0..6 {
            let st = g::<f32>(STATION_COORS + 12 * i);
            let mut buf = [0f32; 3];
            let p = find_player_coors(GPtr::from_raw(buf.as_mut_ptr()), -1);
            let dx = x(st.read()).fsub_m(p.read());
            let dy = x(st.byte_add(4).read()).fsub_m(p.byte_add(4).read());
            if !jp5(hyp(dx, dy), x(60.0)) {
                g::<u8>(STATION_NEARBY).write(1);
            }
        }
    }
    let (period, radius) = if g::<u8>(STATION_NEARBY).read() != 0 { (1u32, 100.0f32) } else { (950, 70.0) };
    if g::<u32>(0xB7_CB78).read() / period == g::<u32>(0xB7_CB84).read() / period {
        return;
    }
    let pool = g::<GPtr<u8>>(0xB7_4494).read();
    let mut front_seen = false;
    let mut idx = rd::<i32>(pool, 8);
    while idx != 0 {
        idx -= 1;
        if rd::<GPtr<i8>>(pool, 4).byte_add(idx as isize).read() < 0 {
            continue;
        }
        let t = rd::<GPtr<u8>>(pool, 0).byte_add(0xA18 * idx as isize);
        if t.is_null() || rd::<i32>(t, 0x594) != 6 {
            continue;
        }
        let fl = rd::<u8>(t, 0x5B8);
        if fl & 8 == 0 || fl & 0x20 != 0 {
            continue;
        }
        front_seen = true;
        let mut far = true;
        let mut c = t;
        while !c.is_null() {
            let m = rd::<GPtr<u8>>(c, 0x14);
            let pos = if m.is_null() { c.byte_add(4) } else { m.byte_add(0x30) };
            let cam = cam_pos();
            let dx = f32_of(x(cam.read()).fsub_m(rd::<f32>(pos, 0)));
            let dy = f32_of(x(cam.byte_add(4).read()).fsub_m(rd::<f32>(pos, 4)));
            if c == find_player_vehicle(-1, 0) || !jp5(hyp(x(dx), x(dy)), x(220.0)) {
                far = false;
            }
            c = rd::<GPtr<u8>>(c, 0x5D4);
        }
        if far {
            let mut c = t;
            loop {
                let next = rd::<GPtr<u8>>(c, 0x5D4);
                world_remove(c);
                entity_vdelete(c, 1);
                if next.is_null() {
                    break;
                }
                c = next;
            }
        }
    }
    if g::<u8>(DISABLE_RANDOM_TRAINS).read() != 0 {
        return;
    }
    match g::<u32>(GEN_STATUS).read() {
        1 => gen_status_1(),
        0 if !front_seen => gen_status_0(radius),
        _ => {}
    }
}

/// Status 1 of 0x6F7900 (0x6F7B0A).
fn gen_status_1() {
    let models = |cfg: i32| g::<i32>((TRAIN_CONFIGS as i32 + 64 * cfg) as u32 as usize);
    let mut cfg = g::<i32>(GEN_CONFIG).read();
    let mut m = models(cfg);
    if m.read() != 0 {
        let mut loaded = true;
        while m.read() != 0 {
            let id = m.read();
            if g::<u8>((0x8E_4CD0 + 20 * id) as u32 as usize).read() != 1 {
                request_model(id, 8);
                cfg = g::<i32>(GEN_CONFIG).read();
                loaded = false;
            }
            m = m.byte_add(4);
        }
        if !loaded {
            return;
        }
    }
    let cam = cam_pos();
    let (cx, cy) = (cam.read(), cam.byte_add(4).read());
    let track = g::<i32>(GEN_TRACK).read();
    let node_idx = g::<i32>(GEN_NODE).read();
    let node = track_nodes(track).byte_add(10 * node_idx as isize);
    let px = xi(i32::from(rd::<i16>(node, 0))).fmul_m(0.125);
    let py = xi(i32::from(rd::<i16>(node, 2))).fmul_m(0.125);
    let dx = f32_of(px.fsub_m(cx));
    let dy = py.fsub_m(cy);
    if !le(dy.fmul(dy).fadd(x(dx).fmul_m(dx)).fsqrt(), x(60.0)) {
        let mut first = [GPtr::<u8>::NULL];
        let dir = g::<u8>(GEN_DIRECTION).read();
        create_mission_train(0.0, 0.0, 0.0, dir, cfg as u32, GPtr::from_raw(first.as_mut_ptr()), GPtr::NULL, node_idx, track, 0);
        let speed = if g::<i32>(GEN_TRACK).read() == 0 {
            let mut s = rand_at(0x6F_7C46) % 30;
            let n = track_nodes(g::<i32>(GEN_TRACK).read()).byte_add(10 * g::<i32>(GEN_NODE).read() as isize);
            let px = xi(i32::from(rd::<i16>(n, 0))).fmul_m(0.125);
            let py = xi(i32::from(rd::<i16>(n, 2))).fmul_m(0.125);
            s += 15;
            let dx = f32_of(px.fsub_m(2222.0));
            let dy = py.fsub_m(-1750.0);
            if !jp5(dy.fmul(dy).fadd(x(dx).fmul_m(dx)).fsqrt(), x(300.0)) {
                s /= 2;
            }
            s
        } else {
            rand_at(0x6F_7CD0) % 7 + 7
        };
        let t = first[0];
        let mut max = [0f32];
        find_maximum_speed_to_stop_at_stations(t, GPtr::from_raw(max.as_mut_ptr()));
        let fs = xi(speed);
        if !jp5(fs, x(max[0])) {
            max[0] = f32_of(fs);
        }
        set_train_speed(t, max[0]);
        wr::<u8>(t, 0x3D0, speed as u8);
        cfg = g::<i32>(GEN_CONFIG).read();
    }
    let mut m = models(cfg);
    while m.read() != 0 {
        set_model_is_deletable(m.read());
        set_model_txd_is_deletable(m.read());
        m = m.byte_add(4);
    }
    g::<u32>(GEN_STATUS).write(0);
}

/// Status 0 of 0x6F7900 (0x6F7D6D).
fn gen_status_0(radius: f32) {
    let cam = cam_pos();
    let (cx, cy) = (cam.read(), cam.byte_add(4).read());
    g::<i32>(GEN_TRACK).write(0);
    let n = rand_at(0x6F_7DB3) % num_track_nodes(0);
    g::<i32>(GEN_NODE).write(n);
    let node = track_nodes(0).byte_add(10 * n as isize);
    let zi = i32::from(rd::<i16>(node, 4));
    let nx = f32_of(xi(i32::from(rd::<i16>(node, 0))).fmul_m(0.125));
    let ny = f32_of(xi(i32::from(rd::<i16>(node, 2))).fmul_m(0.125));
    let nz = f32_of(xi(zi).fmul_m(0.125));
    let d = hyp(x(nx).fsub_m(cx), x(ny).fsub_m(cy));
    let mut dist = f32_of(d);
    let mut take = false;
    if !jp5(d, x(radius)) {
        let mut buf = [0f32; 3];
        let p = find_player_coors(GPtr::from_raw(buf.as_mut_ptr()), -1);
        take = !le(x(nz).fadd_m(6.0), x(p.byte_add(8).read()));
        if !take {
            let mut buf2 = [0f32; 3];
            let p = find_player_coors(GPtr::from_raw(buf2.as_mut_ptr()), -1);
            let (px, py, pz) = (p.read(), p.byte_add(4).read(), p.byte_add(8).read());
            take = find_tunnel_attributes_for_coors(px, py, pz) & 0x880 != 0
                || find_tunnel_attributes_for_coors(nx, ny, nz) & 0x800 == 0;
        }
    }
    if take {
        let dir = rand_at(0x6F_7EC0) & 1 != 0;
        g::<u8>(GEN_DIRECTION).write(u8::from(dir));
        // the stale `fild` temporary (the node's z integer) unless the search loop runs
        let mut kept = zi;
        if !jp5(x(dist), x(170.0)) {
            let track = g::<i32>(GEN_TRACK).read();
            let nodes = track_nodes(track);
            loop {
                kept = g::<i32>(GEN_NODE).read();
                let d = step_node(track, nodes, dir, cx, cy);
                dist = f32_of(d);
                if jp5(d, x(170.0)) {
                    break;
                }
            }
        }
        let c = (g::<i32>(GEN_CONFIG_COUNTER).read() + 1) % 8;
        g::<u32>(GEN_STATUS).write(1);
        g::<i32>(GEN_CONFIG_COUNTER).write(c);
        g::<i32>(GEN_CONFIG).write(c);
        if !le(x(dist), x(220.0)) {
            g::<i32>(GEN_NODE).write(kept);
        }
        return;
    }
    if g::<u32>(GEN_STATUS).read() != 0 || g::<u16>(0xC8_1314).read() != 2 {
        return;
    }
    g::<i32>(GEN_TRACK).write(1);
    let n = rand_at(0x6F_7FFE) % num_track_nodes(1);
    g::<i32>(GEN_NODE).write(n);
    let node = track_nodes(1).byte_add(10 * n as isize);
    let px = xi(i32::from(rd::<i16>(node, 0))).fmul_m(0.125);
    let py = xi(i32::from(rd::<i16>(node, 2))).fmul_m(0.125);
    let dx = f32_of(px.fsub_m(cx));
    let dy = py.fsub_m(cy);
    let d = hyp(x(dx), dy);
    let mut dist = f32_of(d);
    if jp5(d, x(radius)) {
        return;
    }
    let dir = rand_at(0x6F_8073) & 1 != 0;
    g::<u8>(GEN_DIRECTION).write(u8::from(dir));
    if !jp5(x(dist), x(170.0)) {
        let track = g::<i32>(GEN_TRACK).read();
        let nodes = track_nodes(track);
        loop {
            let d = step_node(track, nodes, dir, cx, cy);
            dist = f32_of(d);
            if jp5(d, x(170.0)) {
                break;
            }
        }
    }
    if jp5(x(dist), x(220.0)) {
        return;
    }
    g::<u32>(GEN_STATUS).write(1);
    g::<i32>(GEN_CONFIG).write(rand_at(0x6F_814C) % 2 + 8);
}

// `CPlane` generation state.
const PLANE_GEN_MODEL: usize = 0xC1_CAD8;
const PLANE_GEN_STATUS: usize = 0xC1_CADC;
const PLANE_GEN_TIME: usize = 0xC1_CAE0;
/// `CTimer::m_snTimeInMilliseconds`.
const TIME_MS: usize = 0xB7_CB84;
/// `CWeather::WeatherRegion` and the two weather type words (19 = sandstorm).
const WEATHER_REGION: usize = 0xC8_1314;
const WEATHER_A: usize = 0xC8_1320;
const WEATHER_B: usize = 0xC8_131C;

fn time_ms() -> u32 {
    g::<u32>(TIME_MS).read()
}

fn plane_gen_set(model: u32) {
    g::<u32>(PLANE_GEN_MODEL).write(model);
    g::<u32>(PLANE_GEN_STATUS).write(1);
}

/// The player's vehicle exists and `GetVehicleAppearance` is 3 or 5 (`FindPlayerVehicle(-1, false)`
/// re-called before every test).
fn player_in_air_appearance() -> bool {
    if find_player_vehicle(-1, 0).is_null() {
        return false;
    }
    get_vehicle_appearance(find_player_vehicle(-1, 0)) == 3 || get_vehicle_appearance(find_player_vehicle(-1, 0)) == 5
}

/// The position the generation code reads: the matrix position when the matrix exists, else +4.
fn ent_pos(e: GPtr<u8>) -> GPtr<u8> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) }
}

/// `CPlane::DoPlaneGenerationAndRemoval()` (0x6CD2F0).
///
/// Runs when `(frameCounter & 0x1F) == 0x1E` (byte compare) and 0x8D33BC is set; `CountPlanesAndHelis`
/// first. Status 0xC1CADC:
///
/// * 1: the model 0xC1CAD8 is requested (flag 8) and nothing else until loaded. Model 497 (police
///   Maverick): `FindPlaneCreationCoors(&pos, &target, &heading, &height, false)`, a `CHeli(model,
///   1)` (operator new 0xA18; a null result is used as is), `SetPosn`, the heading matrix
///   (0x6C4130), `JoinCarWithRoadSystem`, status 3 (+0x36 low bits kept), +0x84C = 0.22,
///   `CWorld::Add`; then +0x9B0 = height, mission 0x26, +0x9B4 = heading, +0x9A8 = height, velocity
///   `(cos, sin, 0) * 2/3`, +0x42C |= 0x40, +0x4A6 = 255, +0x3D0 = 50, engine on; `rand() % 2`:
///   1 = `FindUnsuspectingTargetPed(pos, player)`: target +0x41C (mission 0x27 first),
///   `RegisterReference`, +0x988 |= 2, +0x3DB |= 0xA0, and a `CEventDanger(heli, 200)` (+0xE = 1505)
///   to the ped's group intelligence when it has a group whose +0x2D0 != 1, else (+0xE = 911) to
///   the ped's event group (`Add(ev, false)`) unless the ped is a player, +0x484 == 2 or +0x598 ==
///   6; 0 = `FindUnsuspectingTargetCar` likewise (target written before the mission) without event.
///   Other models (577/592 are big; the bool is pushed as the dword whose low byte replaced the
///   plane count's): a `CPlane` (0xA04), +0x9A4 = heading, the same set-up without +0x84C,
///   +0x9B4/+0x9B8 = heading, +0x9B0/+0x9A8 = height, mission 0x24, velocity `(cos, sin, 0) * 2/3`,
///   +0x42C |= 0x40, +0x4A6 = 150 (small and not 520) else 360 (big: +0x1C bit 0 cleared); 520
///   (Hydra): mission 0x3B, +0x86C = 0; `SetGearUp`, engine on (the original passes the `this` ECX
///   SetGearUp preserved), the model made deletable. Either way the time is stamped into 0xC1CAE0
///   and the status reset.
/// * 0: outside interiors, below 5 planes/helis, not in a no-plane zone and with 0xB5F852 clear
///   (else only the time is stamped): wanted level > 3 in a heli/plane (sub-type 4 or 3, not
///   577/592, 60 s since the stamp) sends a Hydra when no vehicle has mission 0x3B, or one has and
///   the level is > 4. Then, with no plane/heli at all and status still 0: weather region 0 or 4
///   without sandstorm, not flying (appearance 3/5), 120 s: `rand() & 7` picks 476 / 512 / 511
///   (table 0x6CDC0C). Then police Maverick when 0xC0BC5C is set, not on a mission and 350 s
///   passed, or in LA riots; then (region not 0/4, 0xC0BC5C clear, no sandstorm, 200 s, not in a
///   no-big-plane zone, not flying) `rand() & 1` picks 592 / 577.
#[hook(addr = 0x6CD2F0, conv = cdecl, name = "CPlane::DoPlaneGenerationAndRemoval", expect = "64 A1 00 00 00 00",
       unverified = "mvpd-veh-7", route = "L2", hazards = "x87-transcendental,heap-writes",
       recs = "driving",
       note = "fsin/fcos of the heading; creates a heli or plane (pool, heap); a CEventDanger on the stack is copied into the ped/group event queues; the original's SEH frame (operator new + ctor) is not reproduced")]
pub fn plane_do_generation_and_removal() {
    if g::<u32>(0xB7_CB4C).read() & 0x1F != 0x1E || g::<u8>(0x8D_33BC).read() == 0 {
        return;
    }
    let count = count_planes_and_helis();
    match g::<u32>(PLANE_GEN_STATUS).read() {
        0 => plane_gen_status_0(count),
        1 => plane_gen_status_1(count),
        _ => {}
    }
}

/// Status 1 of 0x6CD2F0 (0x6CD34A).
fn plane_gen_status_1(count: i32) {
    let model = g::<i32>(PLANE_GEN_MODEL).read();
    if g::<u8>((0x8E_4CD0 + 20 * model) as u32 as usize).read() != 1 {
        request_model(model, 8);
        return;
    }
    let k = f32::from_bits(0x3F2A_AAAB);
    let mut heading = [0f32];
    let mut height = [0f32];
    let mut pos = [0f32; 3];
    let mut target = [0f32; 3];
    let hp = GPtr::from_raw(heading.as_mut_ptr());
    let fp = GPtr::from_raw(height.as_mut_ptr());
    if model == 0x1F1 {
        find_plane_creation_coors(GPtr::from_raw(pos.as_mut_ptr()), GPtr::from_raw(target.as_mut_ptr()), hp, fp, 0);
        let mem = vehicle_new(0xA18);
        let h = if mem.is_null() { GPtr::NULL } else { heli_ctor(mem, g::<i32>(PLANE_GEN_MODEL).read(), 1) };
        set_posn(h, GPtr::from_raw(pos.as_mut_ptr()));
        set_heading_matrix(h, heading[0]);
        join_car_with_road_system(h);
        let st = (rd::<u8>(h, 0x36) & 7) | 0x18;
        wr::<u32>(h, 0x84C, 0x3E61_47AE);
        wr::<u8>(h, 0x36, st);
        world_add(h);
        let c = x(heading[0]).fcos();
        wr::<f32>(h, 0x9B0, height[0]);
        wr::<u8>(h, 0x3BA, 0x26);
        wr::<f32>(h, 0x9B4, heading[0]);
        wr::<f32>(h, 0x9A8, height[0]);
        let vy = f32_of(x(heading[0]).fsin().fmul_m(k));
        let vx = f32_of(c.fmul_m(k));
        set_velocity(h, vx.to_bits(), vy.to_bits(), 0);
        wr::<u8>(h, 0x42C, rd::<u8>(h, 0x42C) | 0x40);
        wr::<u16>(h, 0x4A6, 0xFF);
        wr::<u8>(h, 0x3D0, 0x32);
        set_engine_on(h, 1);
        let mut pc = [0f32; 3];
        match rand_at(0x6C_D490) % 2 {
            0 => {
                let pos = ent_pos(h);
                let p = find_player_coors(GPtr::from_raw(pc.as_mut_ptr()), -1);
                let (qx, qy, qz) = (p.read(), p.byte_add(4).read(), p.byte_add(8).read());
                let car = find_unsuspecting_target_car(rd(pos, 0), rd(pos, 4), rd(pos, 8), qx, qy, qz);
                if !car.is_null() {
                    let slot = h.byte_add(0x41C);
                    wr::<GPtr<u8>>(slot, 0, car);
                    wr::<u8>(h, 0x3BA, 0x27);
                    register_reference(car, slot);
                    wr::<u8>(h, 0x988, rd::<u8>(h, 0x988) | 2);
                    wr::<u8>(h, 0x3DB, rd::<u8>(h, 0x3DB) | 0xA0);
                }
            }
            1 => {
                let pos = ent_pos(h);
                let p = find_player_coors(GPtr::from_raw(pc.as_mut_ptr()), -1);
                let (qx, qy, qz) = (p.read(), p.byte_add(4).read(), p.byte_add(8).read());
                let ped = find_unsuspecting_target_ped(rd(pos, 0), rd(pos, 4), rd(pos, 8), qx, qy, qz);
                if !ped.is_null() {
                    let slot = h.byte_add(0x41C);
                    wr::<u8>(h, 0x3BA, 0x27);
                    wr::<GPtr<u8>>(slot, 0, ped);
                    register_reference(ped, slot);
                    wr::<u8>(h, 0x988, rd::<u8>(h, 0x988) | 2);
                    wr::<u8>(h, 0x3DB, rd::<u8>(h, 0x3DB) | 0xA0);
                    // the CEventDanger the original builds at esp+0x58 (0x1C bytes below its SEH record)
                    let mut ev = [0u32; 7];
                    let evp = GPtr::<u32>::from_raw(ev.as_mut_ptr()).cast::<u8>();
                    let grp = get_peds_group(ped);
                    if !grp.is_null() && rd::<u8>(grp, 0x2D0) != 1 {
                        event_danger_ctor(evp, h, 200.0);
                        wr::<u16>(evp, 0xE, 0x5E1);
                        pgi_add_event(grp.byte_add(0x30), evp);
                        event_danger_dtor(evp);
                    } else if ped_is_player(ped) == 0 && rd::<u8>(ped, 0x484) != 2 && rd::<i32>(ped, 0x598) != 6 {
                        event_danger_ctor(evp, h, 200.0);
                        let intel = rd::<GPtr<u8>>(ped, 0x47C);
                        wr::<u16>(evp, 0xE, 0x38F);
                        event_group_add(intel.byte_add(0x68), evp, 0);
                        event_danger_dtor(evp);
                    }
                }
            }
            _ => {}
        }
    } else {
        let big = model == 0x241 || model == 0x250;
        let arg = (count as u32 & !0xFF) | u32::from(big);
        find_plane_creation_coors(GPtr::from_raw(pos.as_mut_ptr()), GPtr::from_raw(target.as_mut_ptr()), hp, fp, arg);
        let mem = vehicle_new(0xA04);
        let p = if mem.is_null() { GPtr::NULL } else { plane_ctor(mem, g::<i32>(PLANE_GEN_MODEL).read(), 1) };
        set_posn(p, GPtr::from_raw(pos.as_mut_ptr()));
        wr::<f32>(p, 0x9A4, heading[0]);
        set_heading_matrix(p, heading[0]);
        join_car_with_road_system(p);
        wr::<u8>(p, 0x36, (rd::<u8>(p, 0x36) & 7) | 0x18);
        world_add(p);
        let s = x(heading[0]).fsin();
        wr::<f32>(p, 0x9B4, heading[0]);
        wr::<f32>(p, 0x9B0, height[0]);
        wr::<u8>(p, 0x3BA, 0x24);
        wr::<f32>(p, 0x9B8, heading[0]);
        wr::<f32>(p, 0x9A8, height[0]);
        let vy = f32_of(s.fmul_m(k));
        let vx = f32_of(x(heading[0]).fcos().fmul_m(k));
        set_velocity(p, vx.to_bits(), vy.to_bits(), 0);
        wr::<u8>(p, 0x42C, rd::<u8>(p, 0x42C) | 0x40);
        if !big && g::<u32>(PLANE_GEN_MODEL).read() != 0x208 {
            wr::<u16>(p, 0x4A6, 0x96);
        } else {
            wr::<u16>(p, 0x4A6, 0x168);
            if big {
                wr::<u32>(p, 0x1C, rd::<u32>(p, 0x1C) & !1);
            }
        }
        if g::<u32>(PLANE_GEN_MODEL).read() == 0x208 {
            wr::<u8>(p, 0x3BA, 0x3B);
            wr::<u16>(p, 0x86C, 0);
        }
        set_gear_up(p);
        set_engine_on(p, 1);
        set_model_is_deletable(g::<i32>(PLANE_GEN_MODEL).read());
        set_model_txd_is_deletable(g::<i32>(PLANE_GEN_MODEL).read());
    }
    g::<u32>(PLANE_GEN_TIME).write(time_ms());
    g::<u32>(PLANE_GEN_STATUS).write(0);
}

/// Status 0 of 0x6CD2F0 (0x6CD81D).
fn plane_gen_status_0(count: i32) {
    if g::<u32>(0xB7_2914).read() != 0 || count >= 5 || are_we_in_no_plane_zone() != 0 || g::<u8>(0xB5_F852).read() != 0 {
        g::<u32>(PLANE_GEN_TIME).write(time_ms());
        return;
    }
    if get_wanted_level(find_player_ped(-1)) > 3
        && !find_player_vehicle(-1, 0).is_null()
        && (rd::<i32>(find_player_vehicle(-1, 0), 0x594) == 4 || rd::<i32>(find_player_vehicle(-1, 0), 0x594) == 3)
        && rd::<u16>(find_player_vehicle(-1, 0), 0x22) != 0x241
        && rd::<u16>(find_player_vehicle(-1, 0), 0x22) != 0x250
        && time_ms() > g::<u32>(PLANE_GEN_TIME).read().wrapping_add(60_000)
    {
        let pool = g::<GPtr<u8>>(0xB7_4494).read();
        let mut i = rd::<i32>(pool, 8);
        let mut hydras = 0;
        while i != 0 {
            i -= 1;
            let v = vehicle_pool_get_at(pool, i);
            if !v.is_null() && rd::<u8>(v, 0x3BA) == 0x3B {
                hydras += 1;
            }
        }
        if hydras < 1 || (hydras < 2 && get_wanted_level(find_player_ped(-1)) > 4) {
            plane_gen_set(0x208);
        }
    }
    if count >= 1 || g::<u32>(PLANE_GEN_STATUS).read() != 0 {
        return;
    }
    let sandstorm = || g::<u16>(WEATHER_A).read() == 0x13 || g::<u16>(WEATHER_B).read() == 0x13;
    let region = g::<u16>(WEATHER_REGION).read();
    if (region == 0 || region == 4) && !sandstorm() {
        if !player_in_air_appearance() && time_ms() > g::<u32>(PLANE_GEN_TIME).read().wrapping_add(120_000) {
            let m = match rand_at(0x6C_D9FF) & 7 {
                3 | 4 => 0x200,
                5 | 6 => 0x1FF,
                _ => 0x1DC,
            };
            plane_gen_set(m);
            return;
        }
        if g::<u32>(PLANE_GEN_STATUS).read() != 0 {
            return;
        }
    }
    if g::<u8>(0xC0_BC5C).read() != 0 {
        if is_player_on_a_mission() == 0 && time_ms() > g::<u32>(PLANE_GEN_TIME).read().wrapping_add(350_000) {
            plane_gen_set(0x1F1);
            return;
        }
        if g::<u32>(PLANE_GEN_STATUS).read() != 0 {
            return;
        }
    }
    if la_riots_active_here() != 0 {
        plane_gen_set(0x1F1);
        return;
    }
    if g::<u32>(PLANE_GEN_STATUS).read() != 0 {
        return;
    }
    let region = g::<u16>(WEATHER_REGION).read();
    if region == 0 || region == 4 || g::<u8>(0xC0_BC5C).read() != 0 || sandstorm() {
        return;
    }
    if time_ms() <= g::<u32>(PLANE_GEN_TIME).read().wrapping_add(200_000) || are_we_in_no_big_plane_zone() != 0 || player_in_air_appearance() {
        return;
    }
    plane_gen_set(if rand_at(0x6C_DB9C) & 1 != 0 { 0x250 } else { 0x241 });
}

/// `CAutomobile::CAutomobile(int32 modelIndex, eVehicleCreatedBy, bool setupSuspensionLines)`
/// (0x6B0A90, catalogue `CAutomobile::Constructor`, `ret 0xC`; the SEH prologue through the .HOODLUM
/// fragment 0x407DED).
///
/// `CVehicle(createdBy)`, vtable 0x871120, the damage manager (+0x5A0) reset with wheel damage 0.5,
/// the six doors (+0x5B8, 0x90 bytes) zeroed, the three bouncing panels' frame words (+0x6AC, stride
/// 0x20) = -1, +0x70C..+0x720 zeroed. Types +0x594/+0x590 = 0, +0x8E4 = 0, word +0x86A = 0, +0x868 =
/// bit 0 from 0xC1BFD0 (written once with the old bits) then `| 0x30` alone; `SetModelIndex`, the 25
/// frames (+0x648) cleared and filled. Handling 0xC2B9DC + 0xE0 * (low byte of the model's handling
/// id) (+0x384), flags +0x38C (+0xD0); flag 0x10000: `rand() & 3 == 0` adds upgrade word 0x8CD76C.
/// Flying pointer (+0x388, the full id), +0x804 = 20.0 (0x870FD4), +0x964/+0x808/+0x80C = 0,
/// colours. +0x429 low nibble from model flags (+0xCC bits 0/3/1/2 -> 1/4/2/8). Doors: front
/// (+0x5E8/+0x600 open angles -pi/2,pi/2 with +0x429 bit 1 else -/+0.4*pi, axis words 20), rear
/// (RHINO: all angles 1.0 and the two frames' first objects' flag byte +2 cleared; else by +0x429 bit
/// 0), bonnet (+0x5B8 = -/+0.3*pi by flag 0x10, axis 0x24/0x21), boot (+0x5D0 by flags 0x20/0x40);
/// flag 0x100 sets door status 4 on doors 2..5. The six +0x8CC values are `(c - -c) * rand() *
/// 3.0518509e-05 + -c` with c = 0.15 (the full `rand()`), towtruck/tractor bouncing panels, the
/// hydraulics/firetruck nozzle fields (+0x70C..+0x717), zero vectors +0x8B4/+0x8C0, mass, turn mass,
/// centre of mass, +0x9C = 0.05, elasticity, buoyancy (`percentSubmerged / 1000 * 0.5` when > 0.01,
/// else copied), the zeroed state fields, the four wheels' fields, +0x8A0/+0x8A4 = 1.0, then
/// `SetupSuspensionLines` when asked. Status (+0x36 bits 3..7) = 2, +0x484 = 0, police
/// (`IsLawEnforcementVehicle` with +0x4F8 == 1) -> +0x4F8 = 5, the nitro/gun fields. Model -2: the
/// frame +0x65C's first object's flag byte cleared and the frame +0x650 matrix translated to (x + 0.1,
/// 0, z) of the frame +0x65C matrix (two stack `CMatrix`es; the y the original stores is never used);
/// RHINO: +0x40 |= 0x840000. Audio `Initialise`; +0x984 = 0, +0x981 = 0. Returns `this`.
#[hook(addr = 0x6B0A90, conv = thiscall, name = "CAutomobile::Constructor", this = "CAutomobile", expect = "6A FF E9 56 73 D5 FF",
       unverified = "mvpd-veh-7", route = "L2", hazards = "heap-writes",
       recs = "driving",
       note = "constructs into pool memory (operator new by the caller); SEH frame not reproduced")]
pub fn automobile_constructor(this: GPtr<u8>, mi: u32, created_by: u32, setup_susp: u8) -> GPtr<u8> {
    vehicle_ctor(this, created_by);
    wr::<u32>(this, 0, 0x0087_1120);
    reset_damage_status(this.byte_add(0x5A0));
    wr::<u32>(this, 0x5A0, 0x3F00_0000);
    for k in 0..0x24isize {
        wr::<u32>(this, 0x5B8 + 4 * k, 0);
    }
    for k in 0..3isize {
        wr::<u16>(this, 0x6AC + 0x20 * k, 0xFFFF);
    }
    for k in 0..6isize {
        wr::<u32>(this, 0x70C + 4 * k, 0);
    }
    let mip = crate::common::model_info(mi as i32);
    let hid = i32::from(rd::<i16>(mip, 0x4A));
    wr::<u32>(this, 0x594, 0);
    wr::<u32>(this, 0x590, 0);
    wr::<u32>(this, 0x8E4, 0);
    wr::<u16>(this, 0x86A, 0);
    let mut c = rd::<u8>(this, 0x868);
    c = if g::<u8>(0xC1_BFD0).read() != 0 { c | 1 } else { c & 0xFE };
    wr::<u8>(this, 0x868, c);
    wr::<u8>(this, 0x868, (c & 1) | 0x30);
    vehicle_set_model_index(this, mi);
    for k in 0..0x19isize {
        wr::<u32>(this, 0x648 + 4 * k, 0);
    }
    fill_frame_array(rd::<GPtr<u8>>(this, 0x18), this.byte_add(0x648));
    let hd = GPtr::<u8>::from_addr(0xC2_B9DC + 0xE0 * (hid as u32 & 0xFF) as usize);
    wr::<GPtr<u8>>(this, 0x384, hd);
    let hf = rd::<u32>(hd, 0xD0);
    wr::<u32>(this, 0x38C, hf);
    if hf & 0x1_0000 != 0 && rand_at(0x6B_0BC9) & 3 == 0 {
        add_vehicle_upgrade(this, u32::from(g::<u16>(0x8C_D76C).read()));
    }
    let fp = get_flying_pointer(GPtr::from_addr(0xC2_B9C8), hid as u32);
    wr::<GPtr<u8>>(this, 0x388, fp);
    wr::<u32>(this, 0x804, g::<u32>(0x87_0FD4).read());
    wr::<u32>(this, 0x964, 0);
    wr::<u32>(this, 0x808, 0);
    wr::<u32>(this, 0x80C, 0);
    choose_vehicle_colour(mip, this.byte_add(0x434), this.byte_add(0x435), this.byte_add(0x436), this.byte_add(0x437), 1);
    let f = rd::<u8>(this, 0x429) & 0xF0;
    wr::<u8>(this, 0x429, f);
    let hd = rd::<GPtr<u8>>(this, 0x384);
    if rd::<u8>(hd, 0xCC) & 1 != 0 {
        wr::<u8>(this, 0x429, f | 1);
    }
    if rd::<u8>(hd, 0xCC) & 8 != 0 {
        wr::<u8>(this, 0x429, rd::<u8>(this, 0x429) | 4);
    }
    if rd::<u8>(hd, 0xCC) & 2 != 0 {
        wr::<u8>(this, 0x429, rd::<u8>(this, 0x429) | 2);
    }
    if rd::<u8>(hd, 0xCC) & 4 != 0 {
        wr::<u8>(this, 0x429, rd::<u8>(this, 0x429) | 8);
    }
    const NEG_04PI: u32 = 0xBFA0_D97C;
    const POS_04PI: u32 = 0x3FA0_D97C;
    const NEG_HALF_PI: u32 = 0xBFC9_0FDB;
    const POS_HALF_PI: u32 = 0x3FC9_0FDB;
    let wide = rd::<u8>(this, 0x429) & 2 != 0;
    wr::<u8>(this, 0x5F2, 2);
    wr::<u32>(this, 0x5EC, 0);
    wr::<u16>(this, 0x5F0, 0x14);
    if wide {
        wr::<u32>(this, 0x5E8, NEG_HALF_PI);
        wr::<u32>(this, 0x600, POS_HALF_PI);
    } else {
        wr::<u32>(this, 0x5E8, NEG_04PI);
        wr::<u32>(this, 0x600, POS_04PI);
    }
    wr::<u32>(this, 0x604, 0);
    wr::<u8>(this, 0x60A, 2);
    wr::<u16>(this, 0x608, 0x14);
    if rd::<u16>(this, 0x22) == 0x1B0 {
        for off in [0x618isize, 0x61C, 0x624, 0x628, 0x630, 0x634, 0x63C, 0x640] {
            wr::<u32>(this, off, 0x3F80_0000);
        }
        wr::<u8>(get_first_object(rd::<GPtr<u8>>(this, 0x660)), 2, 0);
        wr::<u8>(get_first_object(rd::<GPtr<u8>>(this, 0x654)), 2, 0);
    } else {
        let wide = rd::<u8>(this, 0x429) & 1 != 0;
        wr::<u8>(this, 0x622, 2);
        wr::<u32>(this, 0x61C, 0);
        if wide {
            wr::<u32>(this, 0x618, NEG_HALF_PI);
            wr::<u16>(this, 0x620, 0x10);
            wr::<u32>(this, 0x630, POS_HALF_PI);
            wr::<u16>(this, 0x638, 0x13);
        } else {
            wr::<u32>(this, 0x618, NEG_04PI);
            wr::<u16>(this, 0x620, 0x14);
            wr::<u32>(this, 0x630, POS_04PI);
            wr::<u16>(this, 0x638, 0x14);
        }
        wr::<u32>(this, 0x634, 0);
        wr::<u8>(this, 0x63A, 2);
    }
    let hd = rd::<GPtr<u8>>(this, 0x384);
    let bonnet_up = rd::<u8>(hd, 0xCC) & 0x10 != 0;
    wr::<u8>(this, 0x5C2, 0);
    wr::<u32>(this, 0x5BC, 0);
    if bonnet_up {
        wr::<u32>(this, 0x5B8, 0xBF71_463B);
        wr::<u16>(this, 0x5C0, 0x24);
    } else {
        wr::<u32>(this, 0x5B8, 0x3F71_463B);
        wr::<u16>(this, 0x5C0, 0x21);
    }
    let mf = rd::<u32>(rd::<GPtr<u8>>(this, 0x384), 0xCC);
    if mf & 0x20 != 0 {
        wr::<u32>(this, 0x5D0, NEG_04PI);
        wr::<u16>(this, 0x5D8, 0x15);
    } else if mf & 0x40 != 0 {
        wr::<u32>(this, 0x5D0, POS_HALF_PI);
        wr::<u16>(this, 0x5D8, 0x12);
    } else {
        wr::<u32>(this, 0x5D0, 0xBF71_463B);
        wr::<u16>(this, 0x5D8, 0x14);
    }
    wr::<u32>(this, 0x5D4, 0);
    wr::<u8>(this, 0x5DA, 0);
    if rd::<u32>(rd::<GPtr<u8>>(this, 0x384), 0xCC) & 0x100 != 0 {
        let dm = this.byte_add(0x5A0);
        for door in 2..6 {
            set_door_status(dm, door, 4);
        }
    }
    for k in 0..6isize {
        let cst = g::<f32>(0x87_0FD8).read();
        let neg = f32_of(x(cst).fchs());
        let r = rand_at(0x6B_0EBB);
        let t = xi(r).fmul_m(f32::from_bits(0x3800_0100));
        let v = x(cst).fsub_m(neg).fmul(t).fadd_m(neg);
        wr::<f32>(this, 0x8CC + 4 * k, f32_of(v));
    }
    let model = rd::<u16>(this, 0x22);
    if model == 0x20D && rd::<u32>(this, 0x69C) != 0 && rd::<i16>(this, 0x6AC) == -1 {
        bouncing_panel_set_panel(this.byte_add(0x6AC), 0x15, 2, 0x3F80_0000);
    } else if model == 0x213 && rd::<u32>(this, 0x68C) != 0 && rd::<i16>(this, 0x6AC) == -1 {
        bouncing_panel_set_panel(this.byte_add(0x6AC), 0x11, 1, 0x3F80_0000);
    }
    if rd::<u32>(this, 0x38C) & 0x1000_0000 != 0 {
        let model = rd::<u16>(this, 0x22);
        wr::<u8>(this, 0x717, 1);
        let v = g::<f32>(match model {
            0x256 | 0x1A3 => 0x85_8B10,
            0x199 => 0x85_8C58,
            _ => 0x85_8B38,
        })
        .read();
        wr::<u8>(this, 0x716, 2);
        let a = x(core::f32::consts::PI).fmul(x(v));
        wr::<u16>(this, 0x714, 0xC4);
        wr::<f32>(this, 0x70C, f32_of(a));
        wr::<f32>(this, 0x710, f32_of(x(v).fmul_m(-core::f32::consts::PI)));
    } else if rd::<u16>(this, 0x22) == 0x220 {
        wr::<u32>(this, 0x70C, 0x3EA0_D97C);
        wr::<u32>(this, 0x710, 0xBEA0_D97C);
        wr::<u8>(this, 0x716, 2);
        wr::<u16>(this, 0x714, 0x184);
        wr::<u8>(this, 0x717, 1);
    }
    for k in 0..3isize {
        wr::<u32>(this, 0x8B4 + 4 * k, 0);
    }
    for k in 0..3isize {
        wr::<u32>(this, 0x8C0 + 4 * k, 0);
    }
    let hd = rd::<GPtr<u8>>(this, 0x384);
    wr::<u32>(this, 0x8C, rd::<u32>(hd, 4));
    wr::<u32>(this, 0x90, rd::<u32>(hd, 0xC));
    for k in 0..3isize {
        wr::<u32>(this, 0xA4 + 4 * k, rd::<u32>(hd, 0x14 + 4 * k));
    }
    wr::<u32>(this, 0x9C, 0x3D4C_CCCD);
    wr::<u32>(this, 0xA0, rd::<u32>(hd, 0x24));
    let sub = rd::<f32>(hd, 0x10);
    if !x(sub).fcom(x(0.01)).test_any(0x41) {
        wr::<f32>(this, 0x98, f32_of(x(sub).fdiv_m(1000.0).fmul_m(0.5)));
    } else {
        wr::<u32>(this, 0x98, rd::<u32>(hd, 0x10));
    }
    for off in [0x8ACisize, 0x8A8, 0x870, 0x874, 0x494, 0x49C, 0x4A0, 0x928, 0x964, 0x8B0] {
        wr::<u32>(this, off, 0);
    }
    wr::<u16>(this, 0x86C, 0);
    wr::<u16>(this, 0x86E, 0);
    for i in 0..4isize {
        for k in 0..3isize {
            wr::<u32>(this, 0x8F8 + 12 * i + 4 * k, 0);
        }
        let w = 4 * i;
        wr::<u32>(this, 0x8E8 + w, 0);
        wr::<u32>(this, 0x7D4 + w, 0x3F80_0000);
        wr::<u32>(this, 0x7E4 + w, 0x3F80_0000);
        wr::<u32>(this, 0x7F4 + w, 0);
        wr::<u32>(this, 0x828 + w, 0);
        wr::<u32>(this, 0x848 + w, 0);
        wr::<u32>(this, 0x858 + w, 0);
        wr::<u32>(this, 0x968 + w, 0);
        wr::<u32>(this, 0x810 + w, 0);
        wr::<u8>(this, 0x820 + i, 0);
    }
    wr::<u8>(this, 0x960, 0);
    wr::<u8>(this, 0x961, 0);
    wr::<u8>(this, 0x962, 0);
    wr::<u32>(this, 0x898, 0);
    wr::<u32>(this, 0x89C, 0);
    wr::<u32>(this, 0x8A0, 0x3F80_0000);
    wr::<u32>(this, 0x8A4, 0x3F80_0000);
    if setup_susp != 0 {
        setup_suspension_lines(this);
    }
    wr::<u8>(this, 0x36, (rd::<u8>(this, 0x36) & 7) | 0x10);
    wr::<u8>(this, 0x484, 0);
    if rd::<u32>(this, 0x4F8) == 1 && is_law_enforcement_vehicle(this) != 0 {
        wr::<u32>(this, 0x4F8, 5);
    }
    let model = rd::<u16>(this, 0x22);
    wr::<u32>(this, 0x94C, 0);
    wr::<u32>(this, 0x950, 0x3D4C_CCCD);
    wr::<u32>(this, 0x958, 0);
    wr::<u32>(this, 0x954, 0xBF80_0000);
    wr::<u8>(this, 0x980, 0);
    wr::<u32>(this, 0x948, 0);
    wr::<u32>(this, 0x944, 0);
    if model == 0xFFFE {
        wr::<u8>(get_first_object(rd::<GPtr<u8>>(this, 0x65C)), 2, 0);
        if rd::<u16>(this, 0x22) == 0xFFFE {
            let mut m1 = [0u32; 18];
            let mut m2 = [0u32; 18];
            let p1 = GPtr::<u32>::from_raw(m1.as_mut_ptr()).cast::<u8>();
            let p2 = GPtr::<u32>::from_raw(m2.as_mut_ptr()).cast::<u8>();
            wr::<u32>(p1, 0x40, 0);
            wr::<u32>(p1, 0x44, 0);
            matrix_attach(p1, rd::<GPtr<u8>>(this, 0x650).byte_add(0x10), 0);
            matrix_ctor_rw(p2, rd::<GPtr<u8>>(this, 0x65C).byte_add(0x10), 0);
            let px = f32_of(x(rd::<f32>(p2, 0x30)).fadd_m(0.1));
            matrix_set_translate(p1, px.to_bits(), 0, rd::<u32>(p2, 0x38));
            matrix_update_rw(p1);
            matrix_dtor(p2);
            matrix_dtor(p1);
        }
    } else if model == 0x1B0 {
        wr::<u32>(this, 0x40, rd::<u32>(this, 0x40) | 0x84_0000);
    }
    wr::<u32>(this, 0x978, 0);
    wr::<u32>(this, 0x97C, 0);
    audio_initialise(this.byte_add(0x138), this);
    wr::<u32>(this, 0x984, 0);
    wr::<u8>(this, 0x981, 0);
    this
}

pub const HOOKS: &[HookReg] = &[HOOK_TRAIN_DO_GENERATION_AND_REMOVAL, HOOK_PLANE_DO_GENERATION_AND_REMOVAL, HOOK_AUTOMOBILE_CONSTRUCTOR];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
