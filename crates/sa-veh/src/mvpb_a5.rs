//! WRITER-MVP-B (helper A, batch `mvpb-veh-5`): vehicle functions on the MVP call path. Ported from
//! the 1.0 US bytes; gta-reversed and docs/re/specs/VEH as reading aids. Where they differ, the bytes
//! win.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::vector::CVector;

use crate::common::{f32_of, g, rd, wr, x};

orig! {
    /// `CDamageManager::GetDoorStatus(eDoors)` (0x6C2230)
    fn dm_get_door_status(dm: GPtr<u8>, door: i32) -> u32 = thiscall 0x6C2230 => crate::damage::get_door_status;
    /// `CDamageManager::SetDoorStatus(eDoors, eDoorStatus)` (0x6C21C0)
    fn dm_set_door_status(dm: GPtr<u8>, door: i32, status: u32) = thiscall 0x6C21C0 => crate::damage::set_door_status;
    /// `CMatrix::CMatrix(RwMatrix*, bool)` (0x59C050)
    fn matrix_ctor_attach(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) = thiscall 0x59C050 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CMatrix::SetRotate(float, float, float)` (0x59B120)
    fn matrix_set_rotate(m: GPtr<u8>, rx: f32, ry: f32, rz: f32) = thiscall 0x59B120 => unported;
    /// `CMatrix::UpdateRW()` (0x59BBB0)
    fn matrix_update_rw(m: GPtr<u8>) = thiscall 0x59BBB0 => unported;
    /// `Multiply3x3(CVector* out, const CMatrix&, const CVector&)` (0x59C790)
    fn multiply_3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => unported;
    /// `CVehicle::CanDoorsBeDamaged()` (0x6D1E60)
    fn can_doors_be_damaged(v: GPtr<u8>) -> u8 = thiscall 0x6D1E60 => crate::vehicle::can_doors_be_damaged;
    /// `CDoor::ProcessImpact(CVehicle*, CVector&, CVector&, CVector&)` (0x6F4540)
    fn door_process_impact(d: GPtr<u8>, v: GPtr<u8>, mf: GPtr<u8>, tf: GPtr<u8>, off: GPtr<u8>) -> u8 = thiscall 0x6F4540 => crate::door2::process_impact;
    /// `CDoor::Process(CVehicle*, CVector&, CVector&, CVector&)` (0x6F4040)
    fn door_process(d: GPtr<u8>, v: GPtr<u8>, mf: GPtr<u8>, tf: GPtr<u8>, off: GPtr<u8>) -> u8 = thiscall 0x6F4040 => unported;
    /// `CAEVehicleAudioEntity::AddAudioEvent(eAudioEvents, float)` (0x4F6420)
    fn audio_add_event(a: GPtr<u8>, event: u32, volume: f32) = thiscall 0x4F6420 => unported;
    /// `CAEVehicleAudioEntity::AddAudioEvent(eAudioEvents, CVehicle*)` (0x4F7580)
    fn audio_add_event_entity(a: GPtr<u8>, event: u32, e: GPtr<u8>) = thiscall 0x4F7580 => unported;
    /// `CAutomobile::SpawnFlyingComponent(int32, uint32)` (0x6A8580)
    fn spawn_flying_component(car: GPtr<u8>, node: i32, kind: u32) -> GPtr<u8> = thiscall 0x6A8580 => unported;
    /// `CVehicle::SetComponentVisibility(RwFrame*, uint32)` (0x6D2700)
    fn set_component_visibility(v: GPtr<u8>, frame: GPtr<u8>, state: u32) = thiscall 0x6D2700 => unported;
    /// `operator*(float, const CVector&)` into `out` (0x40FE90)
    fn vector_scale(out: GPtr<u8>, k: f32, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE90 => unported;
    /// `operator+(const CVector&, const CVector&)` into `out` (0x40FE30)
    fn vector_add(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE30 => unported;
    /// `VectorSub` into `out` (0x40FE60)
    fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CPhysical::ApplyTurnForce(CVector force, CVector point)` (0x542A50)
    fn apply_turn_force(p: GPtr<u8>, force: CVector, point: CVector) = thiscall 0x542A50 => sa_world::physical::forces::apply_turn_force;
}

/// A local `[f32; 3]` as a `GPtr<u8>`.
fn vp(v: &mut [f32; 3]) -> GPtr<u8> {
    GPtr::from_raw(v.as_mut_ptr()).cast()
}

fn vec_at(p: GPtr<u8>) -> CVector {
    CVector { x: rd::<f32>(p, 0), y: rd::<f32>(p, 4), z: rd::<f32>(p, 8) }
}

/// `CAutomobile::ProcessSwingingDoor(eCarNodes node, eDoors door)` (0x6A9D70, `ret 8`, SEH frame).
/// `st = GetDoorStatus(door)`; no node frame: return. A closed door (status 0 or 2; > 3 returns)
/// only goes on when `m_fDamageIntensity > 100`, door >= 1 and status (+0x36 >> 3) 0 or 3. A stack
/// CMatrix attached to the frame (+0x10): while closed and `CanDoorsBeDamaged()`, `ProcessImpact(this,
/// moveForce +0x8B4, turnForce +0x8C0, Multiply3x3(m_matrix, pos))` opens it (status + 1). Only an
/// open door (1 or 3) continues: a second attached CMatrix; the bonnet (door 0, axis nibble +0x5C0 ==
/// 1) gains `angVel += ((s fz mz + s fy my) + s fx mx)` with `s = (sin(angle) + 0.1) * [0x8D31CC]`
/// (the original adds 0.1 after the sine; gta-reversed adds it to the angle); `CDoor::Process` closing
/// it sets status - 1 and audio event `door + 0x54`; the door angle goes into rotation axis +0x5C2 of
/// a zero vector, `SetRotate`, position += the saved one, `UpdateRW`. An open bonnet (+0x5C3 == 1)
/// moving forward (`(mz fz + my fy) + mx fx > 0.4`) flies off: `SpawnFlyingComponent(16, 2)`, audio
/// 0x5D, node +0x688 hidden, status 4; the object's speed = `(0.4 ms ± 0.1 right) + 0.5 up` (sign by
/// `rand() & 1`, site 0x6AA09B) and `ApplyTurnForce(10 up, forward)`.
#[hook(addr = 0x6A9D70, conv = thiscall, name = "CAutomobile::ProcessSwingingDoor", this = "CAutomobile", expect = "6A FF 64 A1 00 00 00 00",
       route = "L2", hazards = "heap-writes,x87-transcendental",
       note = "the door axis byte indexes the original's stack rotation vector; 3..5 hit the saved position (modelled), larger values write elsewhere in its frame (ignored)")]
pub fn process_swinging_door(this: GPtr<u8>, node: i32, door: i32) {
    let dm = this.byte_add(0x5A0);
    let mut st = dm_get_door_status(dm, door) as u8;
    let frame = rd::<GPtr<u8>>(this, 0x648 + 4 * node as isize);
    if frame.is_null() {
        return;
    }
    if st != 1 && st != 3 {
        if st >= 3 {
            return;
        }
        if x(rd::<f32>(this, 0xD8)).fcom(x(100.0)).test_any(0x41) || door < 1 {
            return;
        }
        let s = rd::<u8>(this, 0x36) >> 3;
        if s != 0 && s != 3 {
            return;
        }
    }
    let door_obj = this.byte_add((door.wrapping_mul(3).wrapping_add(0xB7) as isize) * 8);
    let mut m1 = [0u32; 18];
    let m1p = GPtr::from_raw(m1.as_mut_ptr()).cast::<u8>();
    matrix_ctor_attach(m1p, frame.byte_add(0x10), 0);
    let mut pos = [rd::<f32>(m1p, 0x30), rd::<f32>(m1p, 0x34), rd::<f32>(m1p, 0x38)];
    let mut t = [0f32; 3];
    if (st == 0 || st == 2) && can_doors_be_damaged(this) != 0 {
        multiply_3x3(vp(&mut t), rd::<GPtr<u8>>(this, 0x14), vp(&mut pos));
        if door_process_impact(door_obj, this, this.byte_add(0x8B4), this.byte_add(0x8C0), vp(&mut t)) != 0 {
            st = st.wrapping_add(1);
            dm_set_door_status(dm, door, u32::from(st));
        }
    }
    if st == 1 || st == 3 {
        let mut m2 = [0u32; 18];
        let m2p = GPtr::from_raw(m2.as_mut_ptr()).cast::<u8>();
        matrix_ctor_attach(m2p, rd::<GPtr<u8>>(this, 0x648 + 4 * node as isize).byte_add(0x10), 0);
        // the original's frame: rotation vector [0..3] followed by the saved position [3..6]
        let mut rq = [0.0, 0.0, 0.0, rd::<f32>(m2p, 0x30), rd::<f32>(m2p, 0x34), rd::<f32>(m2p, 0x38)];
        if door == 0 && rd::<u8>(this, 0x5C0) & 0xF == 1 {
            let mat = rd::<GPtr<u8>>(this, 0x14);
            let s = x(rd::<f32>(this, 0x5C4))
                .fsin()
                .fadd_m(f32::from_bits(0x3DCC_CCCD))
                .fmul_m(g::<f32>(0x8D31CC).read());
            let sz = s.fmul_m(rd::<f32>(mat, 0x18)).fmul_m(rd::<f32>(this, 0x4C));
            let sy = s.fmul_m(rd::<f32>(mat, 0x14)).fmul_m(rd::<f32>(this, 0x48));
            let sx = s.fmul_m(rd::<f32>(mat, 0x10)).fmul_m(rd::<f32>(this, 0x44));
            wr::<f32>(this, 0x5CC, f32_of(sz.fadd(sy).fadd(sx).fadd_m(rd::<f32>(this, 0x5CC))));
        }
        let mut t2 = [0f32; 3];
        let qp = GPtr::from_raw(rq.as_mut_ptr()).cast::<u8>().byte_add(12);
        multiply_3x3(vp(&mut t2), rd::<GPtr<u8>>(this, 0x14), qp);
        if door_process(door_obj, this, this.byte_add(0x8B4), this.byte_add(0x8C0), vp(&mut t2)) != 0 {
            st = st.wrapping_sub(1);
            dm_set_door_status(dm, door, u32::from(st));
            audio_add_event(this.byte_add(0x138), (door + 0x54) as u32, 0.0);
        }
        let axis = usize::from(rd::<u8>(door_obj, 0xA));
        if axis < 6 {
            rq[axis] = rd::<f32>(door_obj, 0xC);
        }
        matrix_set_rotate(m2p, rq[0], rq[1], rq[2]);
        for (k, qk) in rq[3..].iter().enumerate() {
            let off = 0x30 + 4 * k as isize;
            wr::<f32>(m2p, off, f32_of(x(rd::<f32>(m2p, off)).fadd_m(*qk)));
        }
        matrix_update_rw(m2p);
        if door == 0 && rd::<u8>(this, 0x5C3) == 1 {
            let mat = rd::<GPtr<u8>>(this, 0x14);
            let d = x(rd::<f32>(this, 0x4C))
                .fmul_m(rd::<f32>(mat, 0x18))
                .fadd(x(rd::<f32>(this, 0x48)).fmul_m(rd::<f32>(mat, 0x14)))
                .fadd(x(rd::<f32>(this, 0x44)).fmul_m(rd::<f32>(mat, 0x10)));
            if !d.fcom(x(f32::from_bits(0x3ECC_CCCD))).test_any(0x41) {
                let obj = spawn_flying_component(this, 0x10, 2);
                audio_add_event_entity(this.byte_add(0x138), 0x5D, obj);
                set_component_visibility(this, rd::<GPtr<u8>>(this, 0x688), 0);
                dm_set_door_status(dm, 0, 4);
                if !obj.is_null() {
                    let r = crate::common::rand_at(0x6AA09B);
                    let mat = rd::<GPtr<u8>>(this, 0x14);
                    let (mut u, mut rt, mut m, mut a, mut out) = ([0f32; 3], [0f32; 3], [0f32; 3], [0f32; 3], [0f32; 3]);
                    vector_scale(vp(&mut u), 0.5, mat.byte_add(0x20));
                    vector_scale(vp(&mut rt), f32::from_bits(0x3DCC_CCCD), mat);
                    vector_scale(vp(&mut m), f32::from_bits(0x3ECC_CCCD), this.byte_add(0x44));
                    if r & 1 != 0 {
                        vector_add(vp(&mut a), vp(&mut m), vp(&mut rt));
                    } else {
                        vector_sub(vp(&mut a), vp(&mut m), vp(&mut rt));
                    }
                    vector_add(vp(&mut out), vp(&mut a), vp(&mut u));
                    for (k, v) in out.iter().enumerate() {
                        wr::<f32>(obj, 0x44 + 4 * k as isize, *v);
                    }
                    let mat = rd::<GPtr<u8>>(this, 0x14);
                    let mut f = [0f32; 3];
                    vector_scale(vp(&mut f), 10.0, mat.byte_add(0x20));
                    let point = vec_at(mat.byte_add(0x10));
                    apply_turn_force(obj, CVector { x: f[0], y: f[1], z: f[2] }, point);
                }
            }
        }
        matrix_dtor(m2p);
    }
    matrix_dtor(m1p);
}

orig! {
    /// `CTrain::FindClosestTrackNode(CVector, int32* track)` (0x6F6BD0)
    fn find_closest_track_node(x: f32, y: f32, z: f32, track: GPtr<i32>) -> i32 = cdecl 0x6F6BD0 => crate::train2::find_closest_track_node;
    /// `CVehicle::operator new(uint32)` (0x6E2D50)
    fn vehicle_new(size: u32) -> GPtr<u8> = cdecl 0x6E2D50 => crate::vehicle2::operator_new;
    /// `CTrain::CTrain(int32 model, eVehicleCreatedBy)` (0x6F6030)
    fn train_ctor(t: GPtr<u8>, mi: u32, created_by: u32) -> GPtr<u8> = thiscall 0x6F6030 => crate::train2::train_ctor;
    /// `CMatrix::SetTranslate(float, float, float)` (0x59AF40), the floats as bits
    fn matrix_set_translate(m: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x59AF40 => unported;
    /// `CTrain::FindPositionOnTrackFromCoors()` (0x6F6CC0)
    fn train_find_position_on_track_from_coors(t: GPtr<u8>) = thiscall 0x6F6CC0 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn entity_register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CVehicle::ProcessControl()` (virtual, vtable +0x28)
    fn process_control_virtual(v: GPtr<u8>) = thiscall vslot 0x28 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => sa_world::world::cworld2::remove;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => sa_world::world::cworld2::add;
    /// `CPopulation::AddPedInCar(CVehicle*, bool driver, int32 gang, int32 seat, bool male, bool criminal)` (0x613A00)
    fn add_ped_in_car(veh: GPtr<u8>, driver: u8, gang: i32, seat: i32, male: u8, criminal: u8) -> GPtr<u8> = cdecl 0x613A00 => sa_pop::population2::add_ped_in_car;
    /// `CPedIntelligence::SetPedDecisionMakerType(int32)` (0x600B50)
    fn intel_set_ped_decision_maker_type(i: GPtr<u8>, t: i32) = thiscall 0x600B50 => unported;
}

/// `colModel.bbox.max.y - min.y` of the model of `t` (model info +0x14, +0x10 / +4), x87.
fn carriage_length(t: GPtr<u8>) -> sa_math::x87::X87 {
    let cm = rd::<GPtr<u8>>(crate::common::model_info(rd::<i16>(t, 0x22) as i32), 0x14);
    x(rd::<f32>(cm, 0x10)).fsub_m(rd::<f32>(cm, 4))
}

/// `CTrain::CreateMissionTrain(CVector posn, bool clockwise, uint32 type, CTrain** first, CTrain**
/// last, int32 node, int32 track, bool isMission)` (0x6F7550, cdecl, SEH frame; docs/re/specs/VEH/
/// CTrain.md): a negative node is found with `FindClosestTrackNode(posn, &track)`; `rd = node dist
/// (u16) * 0.33333334` (f32). For each model of config 0x8D44F8 + type * 0x40 (until 0): `new
/// CTrain(m, 4)` (null when the pool is full; then the matrix use crashes as in the original),
/// `SetTranslate(0, 0, 0)`, status 4, node index (u16), `bIsLocked`, mission/clockwise bits (5/6 of
/// +0x5B8), rail distance = rd, track id. Mission trains lose bStopsAtStations; the first carriage of
/// a mission train is placed at posn and `FindPositionOnTrackFromCoors()`; the first carriage gets
/// length 0; later ones `length = -len(this)`, `rd -= len` (clockwise) or `length = len(prev)`, `rd +=
/// len`. Front/last carriage bits, `*first`/`*last`, +0x42A bit 1 for followers, the prev/next links
/// with references, `ProcessControl()` (virtual +0x28). Then every carriage `CWorld::Remove` +
/// `CWorld::Add` (last to first); unless the first is model 449 a driver (`AddPedInCar(first, true,
/// -1, 0, false, false)`) with decision maker 6; finally bIsStreakModel (+0x5B9 bit 2) on every
/// carriage = any carriage is model 538.
#[hook(addr = 0x6F7550, conv = cdecl, name = "CTrain::CreateMissionTrain", expect = "6A FF 68 3B 8B 84 00",
       unverified = "mvpb-veh-5", route = "L2", hazards = "heap-writes",
       note = "the original keeps node/track/clockwise-bits in its own argument slots; the port uses locals")]
#[allow(clippy::too_many_arguments)]
pub fn create_mission_train(
    px: f32,
    py: f32,
    pz: f32,
    clockwise: u8,
    kind: u32,
    out_first: GPtr<GPtr<u8>>,
    out_last: GPtr<GPtr<u8>>,
    node: i32,
    track: i32,
    is_mission: u8,
) {
    let mut node = node;
    let mut track_cell = [track];
    if node < 0 {
        node = find_closest_track_node(px, py, pz, GPtr::from_raw(track_cell.as_mut_ptr()));
    }
    let track = track_cell[0];
    let nodes = g::<u32>(0xC38024 + 4 * track as usize).read();
    let dist = g::<u16>(nodes.wrapping_add((node as u32).wrapping_mul(10)) as usize + 6).read();
    let mut rd_ = f32_of(crate::common::xi(i32::from(dist)).fmul_m(f32::from_bits(0x3EAA_AAAB)));
    let mut cfg = 0x8D44F8usize.wrapping_add((kind as usize).wrapping_mul(0x40));
    let mut arr = [GPtr::<u8>::NULL; 16];
    let mut count = 0usize;
    let mut prev = GPtr::<u8>::NULL;
    let mission_bit = (is_mission & 1) << 5;
    let cw_bit = (clockwise & 1) << 6;
    while g::<u32>(cfg).read() != 0 {
        let mem = vehicle_new(0x6AC);
        let t = if mem.is_null() { GPtr::NULL } else { train_ctor(mem, g::<u32>(cfg).read(), 4) };
        matrix_set_translate(rd::<GPtr<u8>>(t, 0x14), 0, 0, 0);
        let st = (rd::<u8>(t, 0x36) & 7) | 0x20;
        wr::<u16>(t, 0x5A0, node as u16);
        let mut f = rd::<u8>(t, 0x5B8);
        wr::<u8>(t, 0x36, st);
        let l = rd::<u8>(t, 0x428) | 8;
        f = (f & 0xDF) | mission_bit;
        wr::<u8>(t, 0x428, l);
        f = (f & 0xBF) | cw_bit;
        wr::<f32>(t, 0x5A8, rd_);
        wr::<u8>(t, 0x5B8, f);
        wr::<u8>(t, 0x5C0, track as u8);
        if is_mission != 0 {
            wr::<u8>(t, 0x5B8, f & 0x7F);
        }
        if prev.is_null() {
            if is_mission != 0 {
                let m = rd::<GPtr<u8>>(t, 0x14);
                if !m.is_null() {
                    wr::<f32>(m, 0x30, px);
                    wr::<f32>(rd::<GPtr<u8>>(t, 0x14), 0x34, py);
                    wr::<f32>(rd::<GPtr<u8>>(t, 0x14), 0x38, pz);
                } else {
                    wr::<f32>(t, 8, py);
                    wr::<f32>(t, 4, px);
                    wr::<f32>(t, 0xC, pz);
                }
                train_find_position_on_track_from_coors(t);
            }
            wr::<u32>(t, 0x5AC, 0);
        } else if clockwise != 0 {
            let len = carriage_length(t);
            wr::<f32>(t, 0x5AC, f32_of(len.fchs()));
            rd_ = f32_of(x(rd_).fsub(len));
        } else {
            let len = carriage_length(prev);
            wr::<f32>(t, 0x5AC, f32_of(len));
            rd_ = f32_of(len.fadd_m(rd_));
        }
        arr[count & 0xF] = t;
        count += 1;
        let dl = rd::<u8>(t, 0x5B8);
        if prev.is_null() {
            wr::<u8>(t, 0x5B8, dl | 8);
            if !out_first.is_null() {
                out_first.write(t);
            }
        } else {
            let cl = rd::<u8>(t, 0x42A) | 2;
            wr::<u8>(t, 0x5B8, dl & 0xF7);
            wr::<u8>(t, 0x42A, cl);
            wr::<u8>(prev, 0x5B8, rd::<u8>(prev, 0x5B8) & 0xEF);
        }
        wr::<u8>(t, 0x5B8, rd::<u8>(t, 0x5B8) | 0x10);
        if !out_last.is_null() {
            out_last.write(t);
        }
        wr::<GPtr<u8>>(t, 0x5D0, prev);
        if !prev.is_null() {
            entity_register_reference(prev, t.byte_add(0x5D0));
        }
        wr::<u32>(t, 0x5D4, 0);
        if !prev.is_null() {
            wr::<GPtr<u8>>(prev, 0x5D4, t);
            entity_register_reference(t, prev.byte_add(0x5D4));
        }
        process_control_virtual(t);
        prev = t;
        cfg += 4;
    }
    for i in (0..count.min(16)).rev() {
        world_remove(arr[i]);
        world_add(arr[i]);
    }
    let first = out_first.read();
    if rd::<u16>(first, 0x22) != 0x1C1 {
        add_ped_in_car(first, 1, -1, 0, 0, 0);
        let first = out_first.read();
        if !first.is_null() {
            let driver = rd::<GPtr<u8>>(first, 0x460);
            if !driver.is_null() {
                intel_set_ped_decision_maker_type(rd::<GPtr<u8>>(driver, 0x47C), 6);
            }
        }
    }
    let first = out_first.read();
    let mut streak = 0u8;
    let mut c = first;
    while !c.is_null() {
        if rd::<u16>(c, 0x22) == 0x21A {
            streak = 1;
            break;
        }
        c = rd::<GPtr<u8>>(c, 0x5D4);
    }
    let mut c = first;
    while !c.is_null() {
        wr::<u8>(c, 0x5B9, (rd::<u8>(c, 0x5B9) & 0xFB) | (streak << 2));
        c = rd::<GPtr<u8>>(c, 0x5D4);
    }
}

orig! {
    /// `CEntity::GetColModel()` (0x535300)
    fn get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
    /// `CMemoryMgr::Malloc(uint32, uint32)` (0x72F420)
    fn memory_malloc(size: u32, hint: u32) -> GPtr<u8> = cdecl 0x72F420 => unported;
    /// `CVehicleModelInfo::GetWheelPosn(int32, CVector&, bool)` (0x4C7D20)
    fn get_wheel_posn(mi: GPtr<u8>, wheel: i32, out: GPtr<u8>, local: u8) = thiscall 0x4C7D20 => sa_stream::modelinfo::vehicle::get_wheel_posn;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    fn get_vehicle_appearance(v: GPtr<u8>) -> u32 = thiscall 0x6D1080 => unported;
}

/// `sqrt((x*x + y*y) + z*z)` of the vector at `p` (x87).
fn mag3(p: GPtr<u8>) -> sa_math::x87::X87 {
    let (vx, vy, vz) = (x(rd::<f32>(p, 0)), x(rd::<f32>(p, 4)), x(rd::<f32>(p, 8)));
    vx.fmul(vx).fadd(vy.fmul(vy)).fadd(vz.fmul(vz)).fsqrt()
}

/// `CAutomobile::SetupSuspensionLines()` (0x6A65D0, virtual; docs/re/specs/VEH/CAutomobile.deviations.md).
/// Lines are allocated when the col data (+0x2C) has none (+0x10): 12 for the RHINO (0x1B0) else 4
/// (`m_nNumLines` byte +6), `CMemoryMgr::Malloc(n * 0x20, 0)`, remembered. Per wheel i: `GetWheelPosn(i,
/// &p, false)`; quads (+0x594 == 2) push x out by 0.15; `m_wheelPosition[i] = p.z` (before the change);
/// `start = (x, y, f32(z + upper))` (handling +0xB8); `end.z = ((lower - upper) - wheelSize * 0.5) +
/// start.z` (front size for wheels 0/2); spring length (+0x878) `= upper - lower`, line length (+0x888)
/// `= start.z - end.z` (from the stored line). `GetVehicleAppearance()` is called once more unless it
/// returns 5 (results unused). Front/rear height above road (+0x898/+0x89C) `= (1 - 1 / (+0xAC * 4)) *
/// spring[0|3] + (size * 0.5 - line[0|3].start.z)`; every `m_wheelPosition` = `size * 0.5 - front
/// height`. bbox min.z lowered to line 0's end; radius raised to the larger corner magnitude; RC Bandit
/// (0x1B9) radius 2 and sphere radii 0.3; with model flag 0x40000000 (+0xCC) on fresh lines, spheres
/// below `d = (0.25 | 0.12 for 0x23B) - frontHeight` are lifted (radius max(cz - d, 0.4) when above
/// 0.4, centre z = d + radius). The RHINO interpolates lines 4..7 / 8..11 between lines 0/1 and 2/3 at
/// t = 0.2..0.8 with the original's mix of f32 and x87 intermediates.
#[hook(addr = 0x6A65D0, conv = thiscall, name = "CAutomobile::SetupSuspensionLines", this = "CAutomobile", expect = "83 EC 5C 53 55 56 57",
       route = "L2", hazards = "heap-writes,x87-transcendental")]
pub fn setup_suspension_lines(this: GPtr<u8>) {
    let mi = crate::common::model_info(rd::<i16>(this, 0x22) as i32);
    let cm = get_col_model(this);
    let cd = rd::<GPtr<u8>>(cm, 0x2C);
    let mut fresh = false;
    if rd::<u32>(cd, 0x10) == 0 {
        wr::<u8>(cd, 6, if rd::<u16>(this, 0x22) == 0x1B0 { 0xC } else { 4 });
        let n = rd::<i8>(cd, 6) as i32;
        let p = memory_malloc((n << 5) as u32, 0);
        wr::<GPtr<u8>>(cd, 0x10, p);
        fresh = true;
    }
    let lines = || rd::<GPtr<u8>>(cd, 0x10);
    let hd = || rd::<GPtr<u8>>(this, 0x384);
    for i in 0..4isize {
        let mut p = [0f32; 3];
        get_wheel_posn(mi, i as i32, vp(&mut p), 0);
        if rd::<u32>(this, 0x594) == 2 {
            let c = f32::from_bits(0x3E19_999A);
            p[0] = if !x(p[0]).fcom(x(0.0)).test_any(0x41) { f32_of(x(p[0]).fadd_m(c)) } else { f32_of(x(p[0]).fsub_m(c)) };
        }
        wr::<f32>(this, 0x838 + 4 * i, p[2]);
        p[2] = f32_of(x(p[2]).fadd_m(rd::<f32>(hd(), 0xB8)));
        let l = lines().byte_add(i * 0x20);
        wr::<f32>(l, 0, p[0]);
        wr::<f32>(l, 4, p[1]);
        wr::<f32>(l, 8, p[2]);
        let size = rd::<f32>(mi, if i == 0 || i == 2 { 0x40 } else { 0x44 });
        let h = hd();
        let ez = x(rd::<f32>(h, 0xBC)).fsub_m(rd::<f32>(h, 0xB8)).fsub(x(size).fmul_m(0.5)).fadd_m(p[2]);
        p[2] = f32_of(ez);
        let e = lines().byte_add(i * 0x20 + 0x10);
        wr::<f32>(e, 0, p[0]);
        wr::<f32>(e, 4, p[1]);
        wr::<f32>(e, 8, p[2]);
        let h = hd();
        wr::<f32>(this, 0x878 + 4 * i, f32_of(x(rd::<f32>(h, 0xB8)).fsub_m(rd::<f32>(h, 0xBC))));
        let l = lines().byte_add(i * 0x20);
        wr::<f32>(this, 0x888 + 4 * i, f32_of(x(rd::<f32>(l, 8)).fsub_m(rd::<f32>(l, 0x18))));
    }
    if get_vehicle_appearance(this) != 5 {
        get_vehicle_appearance(this);
    }
    let h = hd();
    let k = || x(rd::<f32>(h, 0xAC)).fmul_m(4.0).fdivr_m(1.0).fsubr_m(1.0);
    let front = k()
        .fmul_m(rd::<f32>(this, 0x878))
        .fadd(x(rd::<f32>(mi, 0x40)).fmul_m(0.5).fsub_m(rd::<f32>(lines(), 8)));
    wr::<f32>(this, 0x898, f32_of(front));
    let rear = k()
        .fmul_m(rd::<f32>(this, 0x884))
        .fadd(x(rd::<f32>(mi, 0x44)).fmul_m(0.5).fsub_m(rd::<f32>(lines(), 0x68)));
    wr::<f32>(this, 0x89C, f32_of(rear));
    for i in 0..4isize {
        let size = rd::<f32>(mi, if i == 0 || i == 2 { 0x40 } else { 0x44 });
        wr::<f32>(this, 0x838 + 4 * i, f32_of(x(size).fmul_m(0.5).fsub_m(rd::<f32>(this, 0x898))));
    }
    let l0 = lines();
    if !x(rd::<f32>(l0, 0x18)).fcom(x(rd::<f32>(cm, 8))).test_parity(0x05) {
        wr::<u32>(cm, 8, rd::<u32>(l0, 0x18));
    }
    let mn = mag3(cm);
    let mx = mag3(cm.byte_add(0xC));
    let r = if !mn.fcom(mx).test_any(0x41) { mn } else { mag3(cm.byte_add(0xC)) };
    if !r.fcom(x(rd::<f32>(cm, 0x24))).test_any(0x41) {
        wr::<f32>(cm, 0x24, f32_of(r));
    }
    if rd::<u16>(this, 0x22) == 0x1B9 {
        wr::<u32>(cm, 0x24, 0x4000_0000);
        let mut j: i16 = 0;
        while j < rd::<i16>(cd, 0) {
            wr::<u32>(rd::<GPtr<u8>>(cd, 8), j as isize * 0x14 + 0xC, 0x3E99_999A);
            j += 1;
        }
    }
    if rd::<u32>(hd(), 0xCC) & 0x4000_0000 != 0 && fresh {
        let base = if rd::<u16>(this, 0x22) == 0x23B { f32::from_bits(0x3DF5_C28F) } else { 0.25 };
        let d = f32_of(x(base).fsub_m(rd::<f32>(this, 0x898)));
        let c04 = f32::from_bits(0x3ECC_CCCD);
        let mut j = 0i32;
        while j < i32::from(rd::<i16>(cd, 0)) {
            let s = rd::<GPtr<u8>>(cd, 8).byte_add(j as isize * 0x14);
            if !x(rd::<f32>(s, 8)).fsub_m(rd::<f32>(s, 0xC)).fcom(x(d)).test_parity(0x05) {
                if !x(rd::<f32>(s, 0xC)).fcom(x(c04)).test_any(0x41) {
                    let v = x(rd::<f32>(s, 8)).fsub_m(d);
                    let v = if !x(c04).fcom(v).test_any(0x41) { x(c04) } else { v };
                    wr::<f32>(s, 0xC, f32_of(v));
                }
                let s = rd::<GPtr<u8>>(cd, 8).byte_add(j as isize * 0x14);
                wr::<f32>(s, 8, f32_of(x(d).fadd_m(rd::<f32>(s, 0xC))));
            }
            j += 1;
        }
    }
    if rd::<u16>(this, 0x22) != 0x1B0 {
        return;
    }
    let mut dst = 0isize;
    for src in [0isize, 0x40] {
        for k in 1..=4 {
            let t = crate::common::xi(k).fmul_m(f32::from_bits(0x3E4C_CCCD));
            let u = x(1.0).fsub(t);
            let l = lines().byte_add(src);
            let tx = t.fmul_m(rd::<f32>(l, 0x20));
            let ty = t.fmul_m(rd::<f32>(l, 0x24));
            let az = f32_of(t.fmul_m(rd::<f32>(l, 0x28)));
            let ux = u.fmul_m(rd::<f32>(l, 0));
            let cy = f32_of(u.fmul_m(rd::<f32>(l, 4)));
            let cz = f32_of(u.fmul_m(rd::<f32>(l, 8)));
            let out = lines().byte_add(dst + 0x80);
            wr::<f32>(out, 0, f32_of(ux.fadd(tx)));
            wr::<f32>(out, 4, f32_of(x(cy).fadd(ty)));
            wr::<f32>(out, 8, f32_of(x(cz).fadd_m(az)));
            let l = lines().byte_add(src);
            let bx = f32_of(t.fmul_m(rd::<f32>(l, 0x30)));
            let by = f32_of(t.fmul_m(rd::<f32>(l, 0x34)));
            let bz = f32_of(t.fmul_m(rd::<f32>(l, 0x38)));
            let ex = f32_of(u.fmul_m(rd::<f32>(l, 0x10)));
            let ey = f32_of(u.fmul_m(rd::<f32>(l, 0x14)));
            let ez = u.fmul_m(rd::<f32>(l, 0x18));
            let out = lines().byte_add(dst + 0x90);
            wr::<f32>(out, 0, f32_of(x(ex).fadd_m(bx)));
            wr::<f32>(out, 4, f32_of(x(ey).fadd_m(by)));
            wr::<f32>(out, 8, f32_of(ez.fadd_m(bz)));
            dst += 0x20;
        }
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_SWINGING_DOOR, HOOK_CREATE_MISSION_TRAIN, HOOK_SETUP_SUSPENSION_LINES];

pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
