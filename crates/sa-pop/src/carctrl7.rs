//! CCarCtrl script car creation: police chase, convoy (WRITER-POP-4, lead). Ported from the 1.0 US bytes;
//! gta-reversed and `docs/re/specs/POP/*.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, g, jne_mask, model_info, rand_at, rand_frac, rd, wr, x, xi};

/// Globals owned by this module: none.
pub const GLOBALS: &[GlobalDesc] = &[];

/// `TheCamera` (0xB6F028).
const THE_CAMERA: usize = 0xB6F028;

orig! {
    /// `CCarCtrl::ChoosePoliceCarModel(uint32)` (0x421980)
    fn choose_police_car_model(ignore: u32) -> i32 = cdecl 0x421980 => crate::carctrl2::choose_police_car_model;
    /// `CCarCtrl::GetNewVehicleDependingOnCarModel(int32, uint8)` (0x421440)
    fn get_new_vehicle_depending_on_car_model(mi: i32, by: u32) -> GPtr<u8> = cdecl 0x421440 => crate::carctrl5::get_new_vehicle_depending_on_car_model;
    /// `CPathNode::GetPosition()` (0x420A10, thiscall, hidden return)
    fn node_get_position(node: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x420A10 => crate::pathfind::get_position;
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool x6, CStoredCollPoly*)` (0x5674E0)
    fn process_vertical_line(o: GPtr<f32>, z: f32, cp: GPtr<f32>, ent: GPtr<u32>, b: u32, v: u32, p: u32, ob: u32, d: u32, see: u32, poly: u32) -> u8 = cdecl 0x5674E0 => unported;
    /// virtual slot 0 of a `CEntity`: the deleting destructor
    fn entity_delete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    /// virtual slot 0xD4 of a `CVehicle`: `GetDistanceFromCentreOfMassToBaseOfModel()` (ST0)
    fn vehicle_base_distance(v: GPtr<u8>) -> f64 = thiscall vslot 0xD4 => unported;
    /// `CEntity::GetIsOnScreen()` (0x534540)
    fn entity_get_is_on_screen(e: GPtr<u8>) -> u8 = thiscall 0x534540 => sa_world::world::entity::get_is_on_screen_port;
    /// `CCamera::IsSphereVisible(const CVector&, float)` (0x420D40)
    fn camera_is_sphere_visible(cam: GPtr<u8>, c: GPtr<f32>, r: f32) -> u8 = thiscall 0x420D40 => unported;
    /// `CWorld::FindObjectsKindaColliding(const CVector&, float, bool, int16*, int16, CEntity**, bool x5)` (0x568B80)
    fn find_objects_kinda_colliding(pos: GPtr<f32>, r: f32, b2d: u32, cnt: GPtr<i16>, max: i32, out: GPtr<u32>, bld: u32, veh: u32, ped: u32, obj: u32, dum: u32) = cdecl 0x568B80 => unported;
    /// `CPlaceable::GetTopDirection()` (0x41CCB0, thiscall, hidden return)
    fn get_top_direction(e: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x41CCB0 => unported;
    /// `FUN_0040FEC0(CVector* out, const CVector*, float)` (0x40FEC0): `*out = v * f`
    fn vec_mul(out: GPtr<f32>, v: GPtr<f32>, f: f32) -> GPtr<f32> = cdecl 0x40FEC0 => unported;
    /// `CCarCtrl::SetUpDriverAndPassengersForVehicle(CVehicle*, int32, int32, bool, bool, int32)` (0x4217C0)
    fn set_up_driver_and_passengers_for_vehicle(v: GPtr<u8>, pt: i32, min: i32, a: u32, b: u32, max: i32) = cdecl 0x4217C0 => crate::carctrl2::set_up_driver_and_passengers_for_vehicle;
    /// `CMatrix::operator=(const CMatrix&)` (0x59BBC0)
    fn matrix_assign(this: GPtr<u8>, o: GPtr<u8>) = thiscall 0x59BBC0 => unported;
    /// `CPlaceable::SetPosn(const CVector&)` (0x4241C0)
    fn set_posn_v(e: GPtr<u8>, p: GPtr<f32>) = thiscall 0x4241C0 => unported;
    /// `CVehicle::ChangeLawEnforcerState(bool)` (0x6D2330)
    fn change_law_enforcer_state(v: GPtr<u8>, b: u8) = thiscall 0x6D2330 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CBike::PlaceOnRoadProperly()` (0x6BEEB0)
    fn bike_place_on_road_properly(v: GPtr<u8>) = thiscall 0x6BEEB0 => unported;
    /// `CAutomobile::PlaceOnRoadProperly()` (0x6AF420)
    fn automobile_place_on_road_properly(v: GPtr<u8>) = thiscall 0x6AF420 => unported;
    /// `CCarAI::AddPoliceCarOccupants(CVehicle*, bool)` (0x41C070)
    fn add_police_car_occupants(v: GPtr<u8>, always: u8) = cdecl 0x41C070 => crate::carctrl4::add_police_car_occupants;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskComplexCarDriveWander::CTaskComplexCarDriveWander(CVehicle*, eCarDrivingStyle, float)` (0x63CB10)
    fn drive_wander_ctor(this: GPtr<u8>, v: GPtr<u8>, style: u32, speed: f32) -> GPtr<u8> = thiscall 0x63CB10 => unported;
    /// `CTaskManager::SetTask(CTask*, int32, bool)` (0x681AF0)
    fn set_task(mgr: GPtr<u8>, task: GPtr<u8>, slot: i32, b: u8) = thiscall 0x681AF0 => unported;
    /// `CEventAcquaintancePed::CEventAcquaintancePed(CPed*)` (0x4AF820)
    fn event_acquaintance_ped_ctor(this: GPtr<u32>, ped: GPtr<u8>) -> GPtr<u32> = thiscall 0x4AF820 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(this: GPtr<u8>, ev: GPtr<u32>, b: u8) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CEventAcquaintancePed::~CEventAcquaintancePed()` (0x4AF890)
    fn event_acquaintance_ped_dtor(this: GPtr<u32>) = thiscall 0x4AF890 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
}

/// `fld; fcom 0.0; test ah, 5; jp skip; fchs`: the absolute value as the bytes take it (NaN unchanged).
fn abs_jp(v: X87) -> X87 {
    if v.fcom(X87::zero()).test_parity(5) {
        v
    } else {
        v.fchs()
    }
}

/// The ground search both functions share: `ProcessVerticalLine` from `pos` up to 1000 (a hit sets the z), then
/// down to -1000 (a hit closer to `pos.z` than the current z wins, `|d|` by `abs_jp`, strict `<` by `jp`); the
/// start value is 1e9.
fn ground_z(pos: GPtr<f32>) -> f32 {
    let mut best = f32::from_bits(0x4E6E_6B28);
    let mut cp = [0f32; 11];
    let mut ent = [0u32];
    let cpp = GPtr::from_raw(cp.as_mut_ptr());
    let ep = GPtr::from_raw(ent.as_mut_ptr());
    if process_vertical_line(pos, 1000.0, cpp, ep, 1, 0, 0, 0, 1, 0, 0) != 0 {
        best = cp[2];
    }
    if process_vertical_line(pos, -1000.0, cpp, ep, 1, 0, 0, 0, 1, 0, 0) != 0 {
        let pz = pos.wrapping_index(2).read();
        let a = abs_jp(x(cp[2]).fsub_m(pz));
        let b = abs_jp(x(best).fsub_m(pz));
        if !a.fcom(b).test_parity(5) {
            best = cp[2];
        }
    }
    best
}

/// `FindObjectsKindaColliding(pos, r, true, &n, 2, NULL, vehicles and peds)`: the count.
fn colliding(pos: GPtr<f32>, r: f32) -> i16 {
    let mut n = [0i16; 2];
    find_objects_kinda_colliding(
        pos,
        r,
        1,
        GPtr::from_raw(n.as_mut_ptr()),
        2,
        GPtr::NULL,
        0,
        1,
        1,
        0,
        0,
    );
    n[0]
}

/// `CBike::` / `CAutomobile::PlaceOnRoadProperly` by +0x590 (`chase`: any non-bike is an automobile; convoy: only
/// type 0).
fn place_on_road(v: GPtr<u8>, chase: bool) {
    let t = rd::<u32>(v, 0x590);
    if t == 9 {
        bike_place_on_road_properly(v);
    } else if chase || t == 0 {
        automobile_place_on_road_properly(v);
    }
}

/// A `CTaskComplexCarDriveWander(v, 0, 10.0)` for the driver of `v` (slot 4; driver used unchecked).
fn drive_wander(v: GPtr<u8>) {
    let t = task_new(0x24);
    let t = if t.is_null() {
        GPtr::NULL
    } else {
        drive_wander_ctor(t, v, 0, 10.0)
    };
    let intel = rd::<GPtr<u8>>(rd::<GPtr<u8>>(v, 0x460), 0x47C);
    set_task(intel.byte_add(4), t, 4, 0);
}

/// `CCarCtrl::CreatePoliceChase(CVehicle* target, int32 pedType, CNodeAddress node)` (0x42C2B0, SEH frame, AL),
/// from the bytes: unless `CPopCycle` 0xC0BC38 equals 0 (`test ah, 0x44; jnp`) or the ped type is 0xE..0x17, a
/// `ftol(rand16 / 32768 * 4)` (rand 0x42C2F8) other than 0 gives 0. A police model (`ChoosePoliceCarModel(1)`,
/// > -1, loaded) and a new vehicle of it (else 0); the node position; the ground z (`ground_z`, a vehicle at 1e9
/// exactly is kept, above is deleted: 0); z = base distance + ground. Deleted (0) when the target is off screen
/// and the spot (model bounding radius) visible, or when anything collides there. Else the target becomes status
/// 3, +0x3B9 = 2, +0x42E |= 8, cruise speed `ftol(speed + 10)`, move speed `top * speed * 0.02`, its occupants
/// (`SetUpDriverAndPassengersForVehicle(target, pedType, 2, 1, 1, 99)`) flagged 0x800 (+0x478); the cop car copies
/// its matrix, takes the position and move speed, status 3, +0x42C |= 1, law enforcer, added, put on the road,
/// gets police occupants, a wander task, an acquaintance event (vtable 0x858E68, type word 0x451) about the
/// target's driver, +0x4A6 = 0xFF, +0x42C |= 0x40; 1.
#[hook(addr = 0x42C2B0, conv = cdecl, name = "CCarCtrl::CreatePoliceChase", expect = "64 A1 00 00 00 00 D9 05",
       unverified = "popw4-pop-1", route = "split", hazards = "heap-writes",
       note = "script command; creates a police car and its crew")]
pub fn create_police_chase(target: GPtr<u8>, ped_type: i32, node: u32) -> u8 {
    let nonzero = x(g::<f32>(0xC0BC38).read()).fcom(X87::zero()).test_parity(0x44);
    if nonzero && !(0xE..=0x17).contains(&ped_type) {
        let r = rand_at(0x42C2F8);
        if ftol32(rand_frac(r).fmul_m(4.0)) != 0 {
            return 0;
        }
    }
    let model = choose_police_car_model(1);
    if model <= -1 || g::<u8>(0x8E4CD0usize.wrapping_add(model.wrapping_mul(20) as usize)).read() != 1 {
        return 0;
    }
    let v = get_new_vehicle_depending_on_car_model(model, 1);
    if v.is_null() {
        return 0;
    }
    let nodes = g::<GPtr<u8>>(0x96F854 + (node & 0xFFFF) as usize * 4).read();
    let mut np = [0f32; 3];
    let r = node_get_position(
        nodes.byte_add(((node >> 16) * 0x1C) as isize),
        GPtr::from_raw(np.as_mut_ptr()),
    );
    let mut pos = [r.read(), r.wrapping_index(1).read(), r.wrapping_index(2).read()];
    let pp = GPtr::from_raw(pos.as_mut_ptr());
    let best = ground_z(pp);
    if !jne_mask(x(best), x(f32::from_bits(0x4E6E_6B28)), 0x41) {
        entity_delete(v, 1);
        return 0;
    }
    pp.wrapping_index(2).write(
        X87::from_f64_exact(vehicle_base_distance(v))
            .fadd_m(best)
            .to_f32(),
    );
    let radius = rd::<f32>(
        rd::<GPtr<u8>>(model_info(i32::from(rd::<i16>(v, 0x22))), 0x14),
        0x24,
    );
    if entity_get_is_on_screen(target) == 0 && camera_is_sphere_visible(g::<u8>(THE_CAMERA), pp, radius) != 0
    {
        entity_delete(v, 1);
        return 0;
    }
    if colliding(pp, radius) != 0 {
        entity_delete(v, 1);
        return 0;
    }
    let speed = rd::<u8>(target, 0x3D0);
    wr::<u8>(target, 0x36, (rd::<u8>(target, 0x36) & 7) | 0x18);
    wr::<u8>(target, 0x3B9, 2);
    let ns = ftol32(xi(i32::from(speed)).fadd_m(10.0)) as u8;
    wr::<u8>(target, 0x42E, rd::<u8>(target, 0x42E) | 8);
    wr::<u8>(target, 0x3D0, ns);
    let mut top = [0f32; 3];
    let t = get_top_direction(target, GPtr::from_raw(top.as_mut_ptr()));
    let mut a = [0f32; 3];
    let a = vec_mul(GPtr::from_raw(a.as_mut_ptr()), t, xi(i32::from(ns)).to_f32());
    let mut b = [0f32; 3];
    let b = vec_mul(GPtr::from_raw(b.as_mut_ptr()), a, f32::from_bits(0x3CA3_D70A));
    for k in 0..3 {
        target
            .byte_add(0x44)
            .cast::<f32>()
            .wrapping_index(k)
            .write(b.wrapping_index(k).read());
    }
    set_up_driver_and_passengers_for_vehicle(target, ped_type, 2, 1, 1, 0x63);
    matrix_assign(rd(v, 0x14), rd(target, 0x14));
    set_posn_v(v, pp);
    for k in 0..3 {
        let s = target.byte_add(0x44).cast::<u32>().wrapping_index(k).read();
        v.byte_add(0x44).cast::<u32>().wrapping_index(k).write(s);
    }
    wr::<u8>(v, 0x36, (rd::<u8>(v, 0x36) & 7) | 0x18);
    wr::<u8>(v, 0x42C, rd::<u8>(v, 0x42C) | 1);
    change_law_enforcer_state(v, 1);
    world_add(v);
    place_on_road(v, true);
    add_police_car_occupants(v, 1);
    let flag = |p: GPtr<u8>| wr::<u32>(p, 0x478, rd::<u32>(p, 0x478) | 0x800);
    flag(rd(target, 0x460));
    let mut k = 0;
    while k < u32::from(rd::<u8>(target, 0x488)) {
        let p = rd::<GPtr<u8>>(target, 0x464 + 4 * k as isize);
        if !p.is_null() {
            flag(p);
        }
        k += 1;
    }
    drive_wander(v);
    let mut ev = [0u32; 8];
    let evp = GPtr::from_raw(ev.as_mut_ptr());
    event_acquaintance_ped_ctor(evp, rd(target, 0x460));
    evp.write(0x0085_8E68);
    wr::<u16>(evp.cast(), 0xE, 0x451);
    let intel = rd::<GPtr<u8>>(rd::<GPtr<u8>>(v, 0x460), 0x47C);
    event_group_add(intel.byte_add(0x68), evp, 0);
    wr::<u16>(v, 0x4A6, 0xFF);
    wr::<u8>(v, 0x42C, rd::<u8>(v, 0x42C) | 0x40);
    evp.write(0x0085_8E68);
    event_acquaintance_ped_dtor(evp);
    1
}

/// `CCarCtrl::CreateConvoy(CVehicle* lead, int32 pedType)` (0x42C740, SEH frame, AL), from the bytes: 2 cars, 3
/// when `ftol(rand16 / 32768 * 100) > 50` (rand 0x42C770); lead +0x45E = its +0x20 word. Car i (1-based k):
/// `GetNewVehicleDependingOnCarModel(lead model, 1)` placed `k * (2 * radius + 2.5)` behind the lead along its
/// forward axis (the z offset kept 80-bit, x / y stored f32), the ground z as in CreatePoliceChase (1e9 exactly
/// kept), then base distance + ground; deleted when anything collides. The first placed car also sets the lead
/// to status 3, +0x3B9 = 2 and move speed `speed * top * 0.02`. Each car copies the lead's matrix, move speed,
/// status, cruise speed, +0x3B9 / +0x3BA / +0x20 / +0x45E, +0x42B |= 8, added, on the road (bike or type 0),
/// `SetUpDriverAndPassengersForVehicle(car, pedType, 0, 1, 0, 99)`, a wander task, speed multiplier `lead - i`
/// (0 when negative, `jp` test), and +0x41C = the previous car (registered reference). Returns whether one was
/// made.
#[hook(addr = 0x42C740, conv = cdecl, name = "CCarCtrl::CreateConvoy", expect = "6A FF 68 9E 74 83 00 64",
       unverified = "popw4-pop-1", route = "split", hazards = "heap-writes",
       note = "script command; creates 2-3 cars with drivers")]
pub fn create_convoy(lead: GPtr<u8>, ped_type: i32) -> u8 {
    let mut made = 0u8;
    let mut prev = lead;
    let r = rand_at(0x42C770);
    let n = ftol32(rand_frac(r).fmul_m(100.0));
    let radius = rd::<f32>(
        rd::<GPtr<u8>>(model_info(i32::from(rd::<i16>(lead, 0x22))), 0x14),
        0x24,
    );
    wr::<u16>(lead, 0x45E, rd::<u16>(lead, 0x20));
    let count = i32::from(n > 0x32) + 2;
    for i in 0..count {
        let v = get_new_vehicle_depending_on_car_model(i32::from(rd::<i16>(lead, 0x22)), 1);
        if v.is_null() {
            continue;
        }
        let m = rd::<GPtr<u8>>(lead, 0x14);
        let k = xi(i + 1);
        let kz = k.fmul_m(rd::<f32>(m, 0x18));
        let ky = k.fmul_m(rd::<f32>(m, 0x14)).to_f32();
        let kx = k.fmul_m(rd::<f32>(m, 0x10)).to_f32();
        let s = x(radius).fadd(x(radius)).fadd_m(2.5).to_f32();
        let oz = kz.fmul_m(s);
        let oy = x(ky).fmul_m(s).to_f32();
        let ox = x(kx).fmul_m(s).to_f32();
        let lp: GPtr<f32> = if m.is_null() {
            lead.byte_add(4).cast()
        } else {
            m.byte_add(0x30).cast()
        };
        let mut pos = [
            x(lp.read()).fsub_m(ox).to_f32(),
            x(lp.wrapping_index(1).read()).fsub_m(oy).to_f32(),
            oz.fsubr_m(lp.wrapping_index(2).read()).to_f32(),
        ];
        let pp = GPtr::from_raw(pos.as_mut_ptr());
        let best = ground_z(pp);
        if !jne_mask(x(best), x(f32::from_bits(0x4E6E_6B28)), 0x41) {
            entity_delete(v, 1);
            continue;
        }
        pp.wrapping_index(2).write(
            X87::from_f64_exact(vehicle_base_distance(v))
                .fadd_m(best)
                .to_f32(),
        );
        if colliding(pp, radius) != 0 {
            entity_delete(v, 1);
            continue;
        }
        if made == 0 {
            wr::<u8>(lead, 0x36, (rd::<u8>(lead, 0x36) & 7) | 0x18);
            let speed = xi(i32::from(rd::<u8>(lead, 0x3D0))).to_f32();
            wr::<u8>(lead, 0x3B9, 2);
            let mut out = [0f32; 3];
            let t = get_top_direction(lead, GPtr::from_raw(out.as_mut_ptr()));
            let c = f32::from_bits(0x3CA3_D70A);
            let mz = x(speed).fmul_m(t.wrapping_index(2).read());
            let my = x(speed).fmul_m(t.wrapping_index(1).read()).to_f32();
            let mx = x(speed).fmul_m(t.read()).to_f32();
            wr::<f32>(lead, 0x44, x(mx).fmul_m(c).to_f32());
            wr::<f32>(lead, 0x48, x(my).fmul_m(c).to_f32());
            wr::<f32>(lead, 0x4C, mz.fmul_m(c).to_f32());
        }
        matrix_assign(rd(v, 0x14), rd(lead, 0x14));
        let vm = rd::<GPtr<u8>>(v, 0x14);
        if vm.is_null() {
            wr::<f32>(v, 8, x(pos[1]).to_f32());
            wr::<f32>(v, 4, pos[0]);
            wr::<f32>(v, 0xC, x(pos[2]).to_f32());
        } else {
            wr::<f32>(vm, 0x30, pos[0]);
            wr::<f32>(rd(v, 0x14), 0x34, x(pos[1]).to_f32());
            wr::<f32>(rd(v, 0x14), 0x38, x(pos[2]).to_f32());
        }
        for k in 0..3 {
            let s = lead.byte_add(0x44).cast::<u32>().wrapping_index(k).read();
            v.byte_add(0x44).cast::<u32>().wrapping_index(k).write(s);
        }
        wr::<u8>(
            v,
            0x36,
            (rd::<u8>(v, 0x36) & 7) | ((rd::<u8>(lead, 0x36) >> 3) << 3),
        );
        wr::<u8>(v, 0x42B, rd::<u8>(v, 0x42B) | 8);
        world_add(v);
        place_on_road(v, false);
        set_up_driver_and_passengers_for_vehicle(v, ped_type, 0, 1, 0, 0x63);
        drive_wander(v);
        wr::<u8>(v, 0x3D0, rd::<u8>(lead, 0x3D0));
        let mult = xi(i).fsubr_m(rd::<f32>(lead, 0x3C8));
        wr::<f32>(v, 0x3C8, mult.to_f32());
        if !mult.fcom(X87::zero()).test_parity(5) {
            wr::<u32>(v, 0x3C8, 0);
        }
        wr::<u8>(v, 0x3B9, rd::<u8>(lead, 0x3B9));
        wr::<u8>(v, 0x3BA, rd::<u8>(lead, 0x3BA));
        wr::<u16>(v, 0x20, rd::<u16>(lead, 0x20));
        wr::<u16>(v, 0x45E, rd::<u16>(lead, 0x45E));
        wr::<GPtr<u8>>(v, 0x41C, prev);
        register_reference(prev, v.byte_add(0x41C));
        prev = v;
        made = 1;
    }
    made
}

/// This module's hooks.
pub const HOOKS: &[HookReg] = &[HOOK_CREATE_POLICE_CHASE, HOOK_CREATE_CONVOY];
