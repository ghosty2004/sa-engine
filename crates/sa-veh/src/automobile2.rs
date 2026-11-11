//! PORT-VEH-2 (docs/arch/veh-port.md, second pass): `CAutomobile` (0x988 bytes), its subclasses
//! (`CQuadBike`, `CMonsterTruck`, `CTrailer`) and the static `CDamageManager` node-index helpers.
//! Ported from the 1.0 US bytes; gta-reversed `Automobile.cpp` / `DamageManager.cpp` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{f32_of, g, jne_mask, model_info, rand_at, rd, wr, x, xi};

/// `CAutomobile*` (thiscall `this`).
type Car = GPtr<u8>;

orig! {
    /// `CVehicle::operator delete(void*)` (0x6E2D90, catalogue `delete`)
    pub fn vehicle_delete(p: GPtr<u8>) = cdecl 0x6E2D90 => unported;
    /// `CAutomobile::Destructor()` (0x6A61E0)
    pub fn destructor_call(car: GPtr<u8>) = thiscall 0x6A61E0 => destructor;
    /// `FxSystem_c::Kill()` (0x4AA3F0)
    pub fn fx_kill(fx: GPtr<u8>) = thiscall 0x4AA3F0 => unported;
    /// `CAEVehicleAudioEntity::Terminate()` (0x4FB8C0)
    pub fn audio_terminate(a: GPtr<u8>) = thiscall 0x4FB8C0 => unported;
    /// `CVehicle::Destructor()` (0x6E2B40)
    pub fn vehicle_destructor(v: GPtr<u8>) = thiscall 0x6E2B40 => unported;
    /// `RpMaterialSetTexture(RpMaterial*, RwTexture*)` (0x74DBC0)
    pub fn rp_material_set_texture(m: GPtr<u8>, t: GPtr<u8>) -> GPtr<u8> = cdecl 0x74DBC0 => unported;
    /// `RwTextureDestroy(RwTexture*)` (0x7F3820)
    pub fn rw_texture_destroy(t: GPtr<u8>) -> i32 = cdecl 0x7F3820 => unported;
    /// `RwFrameGetLTM(RwFrame*)` (0x7F0990)
    pub fn rw_frame_get_ltm(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F0990 => unported;
    /// `CDamageManager::GetPanelStatus(ePanels)` (0x6C2180)
    pub fn dm_get_panel_status(dm: GPtr<u8>, panel: u32) -> u32 = thiscall 0x6C2180 => crate::damage::get_panel_status;
    /// `CDamageManager::GetDoorStatus(eDoors)` (0x6C2230)
    pub fn dm_get_door_status(dm: GPtr<u8>, door: i32) -> u32 = thiscall 0x6C2230 => crate::damage::get_door_status;
    /// `CDamageManager::SetDoorStatus(eDoors, eDoorStatus)` (0x6C21C0)
    pub fn dm_set_door_status(dm: GPtr<u8>, door: i32, status: u32) = thiscall 0x6C21C0 => crate::damage::set_door_status;
    /// `CDamageManager::GetCarNodeIndexFromPanel(ePanels)` (0x6C26A0)
    pub fn node_from_panel_call(panel: u32) -> i32 = cdecl 0x6C26A0 => get_car_node_index_from_panel;
    /// `CDamageManager::GetCarNodeIndexFromDoor(eDoors)` (0x6C26F0)
    pub fn node_from_door_call(door: u32) -> i32 = cdecl 0x6C26F0 => get_car_node_index_from_door;
    /// `CBouncingPanel::SetPanel(int16, int16, float)` (0x6F4920), the float as its bits
    pub fn bouncing_panel_set(p: GPtr<u8>, frame: u32, axis: u32, limit: u32) = thiscall 0x6F4920 => crate::door::set_panel;
    /// `CVehicle::SetComponentVisibility(RwFrame*, uint32)` (0x6D2700)
    pub fn set_component_visibility(v: GPtr<u8>, frame: GPtr<u8>, state: u32) = thiscall 0x6D2700 => unported;
    /// `CAutomobile::SpawnFlyingComponent(int32, uint32)` (0x6A8580); returns the `CObject*`
    pub fn spawn_flying_component(car: GPtr<u8>, node: i32, kind: u32) -> GPtr<u8> = thiscall 0x6A8580 => unported;
    /// `CGeneral::GetRandomNumberInRange<float>(float, float)` (0x41BD90, ST0)
    pub fn random_float(min: f32, max: f32) -> f64 = cdecl 0x41BD90 => unported;
    /// `CAEVehicleAudioEntity::AddAudioEvent(eAudioEvents, float)` (0x4F6420)
    pub fn audio_add_event(a: GPtr<u8>, event: u32, volume: f32) = thiscall 0x4F6420 => unported;
    /// `CAEVehicleAudioEntity::AddAudioEvent(eAudioEvents, CVehicle*)` (0x4F7580)
    pub fn audio_add_event_entity(a: GPtr<u8>, event: u32, e: GPtr<u8>) = thiscall 0x4F7580 => unported;
    /// `CGlass::CarWindscreenShatters(CVehicle*, bool)` (0x71C2B0)
    pub fn car_windscreen_shatters(v: GPtr<u8>, explode: u32) = cdecl 0x71C2B0 => unported;
    /// `CVehicle::CanDoorsBeDamaged()` (0x6D1E60)
    pub fn can_doors_be_damaged(v: GPtr<u8>) -> u8 = thiscall 0x6D1E60 => unported;
    /// `CDoor::Open(float)` (0x6F4790)
    pub fn door_open(d: GPtr<u8>, ratio: f32) = thiscall 0x6F4790 => crate::door::open;
    /// `CMatrix::CMatrix(RwMatrix*, bool)` (0x59C050)
    pub fn matrix_ctor_attach(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) = thiscall 0x59C050 => unported;
    /// `CMatrix::SetRotate(float, float, float)` (0x59B120)
    pub fn matrix_set_rotate(m: GPtr<u8>, x: f32, y: f32, z: f32) = thiscall 0x59B120 => unported;
    /// `CMatrix::UpdateRW()` (0x59BBB0)
    pub fn matrix_update_rw(m: GPtr<u8>) = thiscall 0x59BBB0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    pub fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `RwFrameForAllObjects(RwFrame*, callback, void*)` (0x7F1200)
    pub fn rw_frame_for_all_objects(f: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x7F1200 => unported;
    /// `CDoor::IsClosed()` (0x6F4800)
    pub fn door_is_closed(d: GPtr<u8>) -> u8 = thiscall 0x6F4800 => crate::door::is_closed;
    /// `CEventSoundQuiet::CEventSoundQuiet(CEntity*, float, uint32, const CVector&)` (0x5E05B0)
    pub fn event_sound_quiet_ctor(ev: GPtr<u8>, e: GPtr<u8>, volume: f32, start: u32, pos: GPtr<u8>) -> GPtr<u8> = thiscall 0x5E05B0 => unported;
    /// `GetEventGlobalGroup()` (0x4ABA50)
    pub fn get_event_global_group() -> GPtr<u8> = cdecl 0x4ABA50 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    pub fn event_group_add(grp: GPtr<u8>, ev: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// the stack event's destructor (0x5DEA00)
    pub fn event_sound_quiet_dtor(ev: GPtr<u8>) = thiscall 0x5DEA00 => unported;
    /// `CDamageManager::GetWheelStatus(eCarWheel)` (0x6C21B0)
    pub fn dm_get_wheel_status(dm: GPtr<u8>, wheel: i32) -> u32 = thiscall 0x6C21B0 => crate::damage::get_wheel_status;
    /// `SurfaceInfos_c::GetAdhesionGroup(uint32)` (0x55E5C0)
    pub fn get_adhesion_group(si: GPtr<u8>, id: u32) -> u32 = thiscall 0x55E5C0 => unported;
}

/// CRT `floor` (0x8219F0, catalogue-excluded CRT) of the register value passed as a double.
fn x87_floor(v: X87) -> X87 {
    X87::from_f64_exact(sa_stream::modelinfo::vehicle::floor_f64(v.to_f64()))
}

/// `|moveSpeed . forward|` (z, y, x order, `fabs`), stored as f32.
fn forward_speed(v: GPtr<u8>) -> f32 {
    let fwd = rd::<GPtr<u8>>(v, 0x14).byte_add(0x10);
    let s = x(rd::<f32>(v, 0x4C))
        .fmul_m(rd::<f32>(fwd, 8))
        .fadd(x(rd::<f32>(v, 0x48)).fmul_m(rd::<f32>(fwd, 4)))
        .fadd(x(rd::<f32>(v, 0x44)).fmul_m(rd::<f32>(fwd, 0)));
    f32::from_bits(f32_of(s).to_bits() & 0x7FFF_FFFF)
}

/// `CAutomobile::m_doors[door]` (+0x5B8, `CDoor` 0x18 each): `this + (door * 3 + 0xB7) * 8`.
fn door_of(car: Car, d: u32) -> GPtr<u8> {
    car.byte_add((d.wrapping_mul(3).wrapping_add(0xB7)).wrapping_mul(8) as i32 as isize)
}

/// `CAutomobile::scalar deleting destructor(flags)` (0x6B1330): Destructor, `CVehicle::operator
/// delete` when `flags & 1`; returns `this`.
#[hook(addr = 0x6B1330, conv = thiscall, name = "CAutomobile::`scalar deleting destructor'", expect = "56 8B F1 E8 A8 4E FF FF", this = "CAutomobile")]
pub fn scalar_deleting_destructor(car: Car, flags: u32) -> GPtr<u8> {
    destructor_call(car);
    if flags & 1 != 0 {
        vehicle_delete(car);
    }
    car
}

/// `CAutomobile::Destructor()` (0x6A61E0, entry `push -1; jmp` into a protection stub that continues
/// at 0x6A61E7): vtable 0x871120; the three effect systems +0x57C, +0x978, +0x97C killed and cleared
/// when set; audio Terminate (+0x138); `CVehicle::Destructor`.
#[hook(addr = 0x6A61E0, conv = thiscall, name = "CAutomobile::Destructor", expect = "6A FF E9 CC BA D5 FF", this = "CAutomobile")]
pub fn destructor(car: Car) {
    wr::<u32>(car, 0, 0x0087_1120);
    for off in [0x57C, 0x978, 0x97C] {
        let fx = rd::<GPtr<u8>>(car, off);
        if !fx.is_null() {
            fx_kill(fx);
            wr::<u32>(car, off, 0);
        }
    }
    audio_terminate(car.byte_add(0x138));
    vehicle_destructor(car);
}

/// `CAutomobile::CustomCarPlate_BeforeRenderingStart(CVehicleModelInfo*)` (0x6A2F00): with a plate
/// material (model info +0x24): its current texture's refCount (+0x54) += 1, saved in 0xC1BFD8, and
/// the car's plate texture (+0x588) set on the material.
#[hook(addr = 0x6A2F00, conv = thiscall, name = "CAutomobile::CustomCarPlate_BeforeRenderingStart", expect = "8B 44 24 04 8B 40 24", this = "CAutomobile")]
pub fn custom_car_plate_before_rendering_start(car: Car, mi: GPtr<u8>) {
    let mat = rd::<GPtr<u8>>(mi, 0x24);
    if mat.is_null() {
        return;
    }
    let tex = rd::<GPtr<u8>>(mat, 0);
    wr::<u32>(tex, 0x54, rd::<u32>(tex, 0x54).wrapping_add(1));
    let plate = rd::<GPtr<u8>>(car, 0x588);
    g::<u32>(0xC1BFD8).write(tex.addr() as u32);
    rp_material_set_texture(mat, plate);
}

/// `CAutomobile::CustomCarPlate_AfterRenderingStop(CVehicleModelInfo*)` (0x6A2F30): with a plate
/// material: the saved texture (0xC1BFD8) back on it, then destroyed (refCount drop), 0xC1BFD8 = 0.
#[hook(addr = 0x6A2F30, conv = thiscall, name = "CAutomobile::CustomCarPlate_AfterRenderingStop", expect = "8B 44 24 04 8B 40 24", this = "CAutomobile")]
pub fn custom_car_plate_after_rendering_stop(_car: Car, mi: GPtr<u8>) {
    let mat = rd::<GPtr<u8>>(mi, 0x24);
    if mat.is_null() {
        return;
    }
    rp_material_set_texture(mat, g::<GPtr<u8>>(0xC1BFD8).read());
    rw_texture_destroy(g::<GPtr<u8>>(0xC1BFD8).read());
    g::<u32>(0xC1BFD8).write(0);
}

/// `CAutomobile::GetComponentWorldPosition(int32, CVector&)` (0x6A2210): the position row of the
/// node's LTM when the node (+0x648, no range check) exists; else `out` untouched.
#[hook(addr = 0x6A2210, conv = thiscall, name = "CAutomobile::GetComponentWorldPosition", expect = "8B 44 24 04 8B 84 81 48 06 00 00", this = "CAutomobile")]
pub fn get_component_world_position(car: Car, comp: i32, out: GPtr<u32>) {
    let node = rd::<GPtr<u8>>(car, (comp as isize).wrapping_mul(4).wrapping_add(0x648));
    if node.is_null() {
        return;
    }
    let ltm = rw_frame_get_ltm(node);
    for k in 0..3 {
        out.byte_add(4 * k).write(rd::<u32>(ltm, 0x30 + 4 * k));
    }
}

/// `CDamageManager::GetCarNodeIndexFromPanel(ePanels)` (0x6C26A0, static): 0 -> 15, 1 -> 14,
/// 4 -> 18, 5 -> 12, 6 -> 13, others -> -1.
#[hook(addr = 0x6C26A0, conv = cdecl, name = "CDamageManager::GetCarNodeIndexFromPanel", expect = "8B 4C 24 04 83 C8 FF")]
pub fn get_car_node_index_from_panel(panel: u32) -> i32 {
    match panel {
        0 => 0xF,
        1 => 0xE,
        4 => 0x12,
        5 => 0xC,
        6 => 0xD,
        _ => -1,
    }
}

/// `CDamageManager::GetCarNodeIndexFromDoor(eDoors)` (0x6C26F0, static): 0 -> 16, 1 -> 17, 2 -> 10,
/// 3 -> 8, 4 -> 11, 5 -> 9, others -> -1.
#[hook(addr = 0x6C26F0, conv = cdecl, name = "CDamageManager::GetCarNodeIndexFromDoor", expect = "8B 4C 24 04 83 C8 FF")]
pub fn get_car_node_index_from_door(door: u32) -> i32 {
    match door {
        0 => 0x10,
        1 => 0x11,
        2 => 0xA,
        3 => 8,
        4 => 0xB,
        5 => 9,
        _ => -1,
    }
}

/// `CAutomobile::ProcessControlCollisionCheck(bool)` (0x6A29C0): wheel ratios +0x7D4 = 1.0 (0x870FD0).
#[hook(addr = 0x6A29C0, conv = thiscall, name = "CAutomobile::ProcessControlCollisionCheck", expect = "6A FF 68 C8 7E 84 00", this = "CAutomobile")]
pub fn process_control_collision_check(car: Car, apply_speed: u32) {
    crate::bike2::control_collision_check(car, apply_speed, 0x870FD0, 0x7D4);
}

/// `m_aCarNodes[node]` (+0x648, no range check).
fn car_node(car: Car, node: i32) -> GPtr<u8> {
    rd::<GPtr<u8>>(car, (node as isize).wrapping_mul(4).wrapping_add(0x648))
}

/// The model's damageable-component mask (`vehicle struct (+0x5C) -> +0x310`) has bit `node & 31`.
fn node_damageable(car: Car, node: i32) -> bool {
    let mi = model_info(i32::from(rd::<i16>(car, 0x22)));
    let vs = rd::<GPtr<u8>>(mi, 0x5C);
    rd::<u32>(vs, 0x310) & 1u32.wrapping_shl(node as u32) != 0
}

/// The first of the 4 bouncing panels (+0x6AC, 0x20 each) that is free (frame id -1): `Ok(index)`;
/// `Err(())` when one already holds `node` or all 4 hold other frames.
fn free_bouncing_panel(car: Car, node: i32) -> Result<isize, ()> {
    for k in 0..4isize {
        let id = rd::<u16>(car, 0x6AC + 0x20 * k);
        if id == 0xFFFF {
            return Ok(k);
        }
        if i32::from(id as i16) == node {
            return Err(());
        }
    }
    Err(())
}

/// `CAutomobile::SetBumperDamage(int32 panel, bool skipFlying)` (0x6B1350; damage.rs's ApplyDamage
/// calls it for group 0). Panel status 2 (unless handling flag 0x10000000): a free bouncing panel gets
/// `SetPanel(node, 0, rand() * 3.0518e-5 (0x858C7C) * -0.3 - 0.2)` (no visibility change
/// here); status 1: visibility 2; status 3: flying component (type 0) unless skipped, visibility 0.
#[hook(addr = 0x6B1350, conv = thiscall, name = "CAutomobile::SetBumperDamage", expect = "53 55 56 57 8B 7C 24 14", this = "CAutomobile")]
pub fn set_bumper_damage(car: Car, panel: u32, skip_flying: u32) {
    let status = dm_get_panel_status(car.byte_add(0x5A0), panel) as u8;
    let node = node_from_panel_call(panel);
    let frame = car_node(car, node);
    if frame.is_null() || !node_damageable(car, node) {
        return;
    }
    match status {
        2 => {
            if rd::<u32>(rd::<GPtr<u8>>(car, 0x384), 0xCC) & 0x1000_0000 != 0 {
                return;
            }
            let Ok(k) = free_bouncing_panel(car, node) else {
                return;
            };
            let r = rand_at(0x6B13F9);
            let v = xi(r)
                .fmul_m(g::<f32>(0x858C7C).read())
                .fmul_m(g::<f32>(0x858EE4).read())
                .fsub_m(g::<f32>(0x858CC4).read());
            bouncing_panel_set(
                car.byte_add(0x6AC + 0x20 * k),
                node as u32,
                0,
                f32_of(v).to_bits(),
            );
        }
        1 => set_component_visibility(car, frame, 2),
        3 => {
            if skip_flying as u8 == 0 {
                spawn_flying_component(car, node, 0);
            }
            set_component_visibility(car, car_node(car, node), 0);
        }
        _ => {}
    }
}

/// `CAutomobile::SetPanelDamage(int32 panel, bool skipFlying)` (0x6B1480; ApplyDamage's group 5).
/// Status 2 (unless handling flag 0x10000000): a free bouncing panel, for nodes other than 18/15/14,
/// gets `SetPanel(node, 1, GetRandomNumberInRange(-0.2, -0.5))`; then visibility 2 (also when a panel
/// already holds the node or none is free). Status 1: panel 4 (windscreen) plays audio event 0x5B,
/// visibility 2. Status 3: flying component (type 5), or with skipFlying the windscreen shatters
/// (panel 4); visibility 0.
#[hook(addr = 0x6B1480, conv = thiscall, name = "CAutomobile::SetPanelDamage", expect = "53 55 56 57 8B 7C 24 14", this = "CAutomobile")]
pub fn set_panel_damage(car: Car, panel: u32, skip_flying: u32) {
    let status = dm_get_panel_status(car.byte_add(0x5A0), panel) as u8;
    let node = node_from_panel_call(panel);
    let frame = car_node(car, node);
    if frame.is_null() || !node_damageable(car, node) {
        return;
    }
    match status {
        2 => {
            if rd::<u32>(rd::<GPtr<u8>>(car, 0x384), 0xCC) & 0x1000_0000 != 0 {
                return;
            }
            if let Ok(k) = free_bouncing_panel(car, node)
                && node != 0x12
                && node != 0xF
                && node != 0xE
            {
                let r = random_float(f32::from_bits(0xBE4C_CCCD), f32::from_bits(0xBF00_0000));
                let r = f32_of(X87::from_f64_exact(r));
                bouncing_panel_set(car.byte_add(0x6AC + 0x20 * k), node as u32, 1, r.to_bits());
            }
            set_component_visibility(car, car_node(car, node), 2);
        }
        1 => {
            if panel == 4 {
                audio_add_event(car.byte_add(0x138), 0x5B, 0.0);
            }
            set_component_visibility(car, car_node(car, node), 2);
        }
        3 => {
            if skip_flying as u8 == 0 {
                spawn_flying_component(car, node, 5);
            } else if panel == 4 {
                car_windscreen_shatters(car, 0);
            }
            set_component_visibility(car, car_node(car, node), 0);
        }
        _ => {}
    }
}

/// `CAutomobile::SetDoorDamage(eDoors door, bool skipFlying)` (0x6B1600). With a node for the door:
/// - doors that cannot be damaged (`CanDoorsBeDamaged` false) with status > 2: the boot (door 1) goes
///   on below; doors 2.. are opened (`CDoor::Open(0)`) and set to status 2; the bonnet (0) goes on;
/// - boot with handling flag bit 7 of byte +0xCC: status 1 -> 2 (damageable node) or 4, status 3 -> 4,
///   written with SetDoorStatus(1, ...); the new status is used below;
/// - status 2: visibility 2 (damageable node); when the door angle (+0x10) is not 0.0 (`test ah, 44h;
///   jnp`): angles +0xC/+0x10/+0x14 = 0, the node's matrix re-set to its own position with no
///   rotation (CMatrix attach, SetRotate(0,0,0), pos += saved pos, UpdateRW), audio event door + 0x54;
/// - status 4: unless skipped, flying component (bonnet: type 3 plus audio event 0x5D with the
///   object; boot: 4; others: 2); visibility 0;
/// - status 1/3: `RwFrameForAllObjects(node, SetAtomicFlagCB, 0x400)`; the bonnet's door +0x14 = 0.2.
#[hook(addr = 0x6B1600, conv = thiscall, name = "CAutomobile::SetDoorDamage", expect = "6A FF 68 78 81 84 00", this = "CAutomobile")]
pub fn set_door_damage(car: Car, door: u32, skip_flying: u32) {
    let dm = car.byte_add(0x5A0);
    let mut status = dm_get_door_status(dm, door as i32) as u8;
    let node = node_from_door_call(door);
    if car_node(car, node).is_null() {
        return;
    }
    let boot = if can_doors_be_damaged(car) == 0 && status > 2 && door != 0 {
        if door != 1 {
            door_open(door_of(car, door), 0.0);
            dm_set_door_status(dm, door as i32, 2);
            return;
        }
        true
    } else {
        door == 1
    };
    if boot && (rd::<u8>(rd::<GPtr<u8>>(car, 0x384), 0xCC) as i8) < 0 {
        if status == 1 {
            status = if node_damageable(car, node) { 2 } else { 4 };
        } else if status == 3 {
            status = 4;
        }
        dm_set_door_status(dm, 1, u32::from(status));
    }
    match status {
        2 => {
            if node_damageable(car, node) {
                set_component_visibility(car, car_node(car, node), 2);
            }
            let d = door_of(car, door);
            let angle = x(rd::<f32>(d, 0x10));
            if !angle.fcom(x(g::<f32>(0x858B50).read())).test_parity(0x44) {
                return;
            }
            wr::<u32>(d, 0xC, 0);
            wr::<u32>(d, 0x10, 0);
            wr::<u32>(d, 0x14, 0);
            let mut m = crate::bike2::StackMatrix::new();
            let mp = m.ptr();
            matrix_ctor_attach(mp, car_node(car, node).byte_add(0x10), 0);
            let saved = [rd::<f32>(mp, 0x30), rd::<f32>(mp, 0x34), rd::<f32>(mp, 0x38)];
            matrix_set_rotate(mp, 0.0, 0.0, 0.0);
            for (k, s) in saved.into_iter().enumerate() {
                let o = 0x30 + 4 * k as isize;
                wr::<f32>(mp, o, f32_of(x(rd::<f32>(mp, o)).fadd_m(s)));
            }
            matrix_update_rw(mp);
            audio_add_event(car.byte_add(0x138), door.wrapping_add(0x54), 0.0);
            matrix_dtor(mp);
        }
        4 => {
            if skip_flying as u8 == 0 {
                if door == 0 {
                    let obj = spawn_flying_component(car, node, 3);
                    audio_add_event_entity(car.byte_add(0x138), 0x5D, obj);
                } else {
                    spawn_flying_component(car, node, if door == 1 { 4 } else { 2 });
                }
            }
            set_component_visibility(car, car_node(car, node), 0);
        }
        1 | 3 => {
            rw_frame_for_all_objects(car_node(car, node), 0x4C_7B90, 0x400);
            if door == 0 {
                wr::<u32>(car, 0x5CC, 0x3E4C_CCCD);
            }
        }
        _ => {}
    }
}

/// `CEventSoundQuiet(ped, volume, -1, (0,0,0))` built on the stack, added to the global event group,
/// destroyed (OpenDoor's door sound events; the object fits the original's 0x2C-byte slot).
fn add_sound_quiet_event(ped: GPtr<u8>, volume: f32) {
    let mut pos = [0u32; 3];
    let mut ev = [0u32; 11];
    let evp = GPtr::from_raw(ev.as_mut_ptr()).cast::<u8>();
    event_sound_quiet_ctor(
        evp,
        ped,
        volume,
        0xFFFF_FFFF,
        GPtr::from_raw(pos.as_mut_ptr()).cast(),
    );
    let grp = get_event_global_group();
    event_group_add(grp, evp, 0);
    event_sound_quiet_dtor(evp);
}

/// `CAutomobile::OpenDoor(CPed*, int32 node, eDoors, float ratio, bool playSound)` (0x6A6AE0). With the
/// node's frame: a CMatrix attached to its RwMatrix (position saved); a closed door gets
/// `ClearAtomicFlagCB(0x400)` on the node; `CDoor::Open(ratio)`. Was closed and the angle changed
/// (+4 != +0xC, `test ah, 44h; jnp`): `SetAtomicFlagCB(0x400)`, with playSound audio event door + 0x4E
/// and, with a ped, a quiet sound event (60.0). Was open and ratio == 0.0: door status 1 -> 0, 3 -> 2
/// (GetDoorStatus called again for the second test), with playSound audio event door + 0x54 and a
/// ped sound event (120.0). Finally the rotation vector gets the door angle (+0xC) on the door's axis
/// (+0xA, unchecked: the original indexes its own stack; axes 3..23 land on the saved position /
/// the matrix in the same order here, larger ones are not modelled), SetRotate, pos += saved,
/// UpdateRW.
#[hook(addr = 0x6A6AE0, conv = thiscall, name = "CAutomobile::OpenDoor", expect = "64 A1 00 00 00 00", this = "CAutomobile")]
pub fn open_door(car: Car, ped: GPtr<u8>, node: i32, door: u32, ratio: f32, play_sound: u32) {
    let frame = car_node(car, node);
    if frame.is_null() {
        return;
    }
    // the original's stack: rotation (3), saved position (3), CMatrix (0x48 bytes)
    let mut st = [0u32; 24];
    let base = GPtr::from_raw(st.as_mut_ptr()).cast::<u8>();
    let mp = base.byte_add(0x18);
    matrix_ctor_attach(mp, frame.byte_add(0x10), 0);
    for k in 0..3 {
        wr::<u32>(base, 0xC + 4 * k, rd::<u32>(mp, 0x30 + 4 * k));
    }
    let d = door_of(car, door);
    for k in 0..3 {
        wr::<u32>(base, 4 * k, 0);
    }
    let was_closed = door_is_closed(d) != 0;
    if was_closed {
        rw_frame_for_all_objects(car_node(car, node), 0x4C_7BB0, 0x400);
    }
    door_open(d, ratio);
    if was_closed {
        if x(rd::<f32>(d, 4)).fcom(x(rd::<f32>(d, 0xC))).test_parity(0x44) {
            rw_frame_for_all_objects(car_node(car, node), 0x4C_7B90, 0x400);
            if play_sound as u8 != 0 {
                audio_add_event(car.byte_add(0x138), door.wrapping_add(0x4E), 0.0);
                if !ped.is_null() {
                    add_sound_quiet_event(ped, 60.0);
                }
            }
        }
    } else if !x(ratio).fcom(x(g::<f32>(0x858B50).read())).test_parity(0x44) {
        let dm = car.byte_add(0x5A0);
        if dm_get_door_status(dm, door as i32) == 1 {
            dm_set_door_status(dm, door as i32, 0);
        } else if dm_get_door_status(dm, door as i32) == 3 {
            dm_set_door_status(dm, door as i32, 2);
        }
        if play_sound as u8 != 0 {
            audio_add_event(car.byte_add(0x138), door.wrapping_add(0x54), 0.0);
            if !ped.is_null() {
                add_sound_quiet_event(ped, 120.0);
            }
        }
    }
    let axis = rd::<u8>(d, 0xA) as isize;
    let angle = f32_of(x(rd::<f32>(d, 0xC)));
    if axis < 24 {
        wr::<f32>(base, 4 * axis, angle);
    }
    let (rx, ry, rz) = (rd::<f32>(base, 0), rd::<f32>(base, 4), rd::<f32>(base, 8));
    matrix_set_rotate(mp, rx, ry, rz);
    for k in 0..3 {
        let o = 0x30 + 4 * k;
        wr::<f32>(
            mp,
            o,
            f32_of(x(rd::<f32>(mp, o)).fadd_m(rd::<f32>(base, 0xC + 4 * k))),
        );
    }
    matrix_update_rw(mp);
    matrix_dtor(mp);
}

/// `CAutomobile::DoBurstAndSoftGroundRatios()` (0x6A47F0). S = |forward speed|; per wheel i, ratio
/// R = +0x7D4[i], compression c = (+0x888[i] - +0x878[i]) / +0x888[i]:
/// - wheel status 2 (GetWheelStatus): R = 1.0 (0x870FD0);
/// - status 1 (asked again): `rand()` (site 0x6A4875) gates with ftol(frac * (u16(ftol(S * 40)) + 98))
///   < 100: R += c * 0.25 (0x8710B0), clamped to 1.0;
/// - else, R < 1.0 on adhesion group 4 (sand) unless model 432 (Rhino): k = 0x8710B4 (handling flag
///   0x200000: 0x8710BC, 0x100000: 0x8710B8), q = max(0x858EE8, 1 - S / 0x871030 * 0.7 - 0xC81308 *
///   0.7); R += k * (c * q), clamped to 1.0;
/// - else, R < 1.0 on surface 178 (railway track bytes at +0x747 + 0x2C * i): with the wheel scale
///   (front/rear model info +0x40/+0x44), the sleeper phase before/after the wheel's rotation step
///   (floor) decides whether R -= c * 0x8710C0 (floored at 0.2 (0x858CC4)).
/// The x87 stack order, f32 stores and the twice-called GetWheelStatus are kept.
#[hook(addr = 0x6A47F0, conv = thiscall, name = "CAutomobile::DoBurstAndSoftGroundRatios", expect = "55 8B EC 83 E4 F8", this = "CAutomobile")]
pub fn do_burst_and_soft_ground_ratios(car: Car) {
    let mi = model_info(i32::from(rd::<i16>(car, 0x22)));
    let s = forward_speed(car);
    let one = g::<f32>(0x870FD0).read();
    let dm = car.byte_add(0x5A0);
    for i in 0..4isize {
        let r_off = 0x7D4 + 4 * i;
        let comp = || {
            x(rd::<f32>(car, 0x888 + 4 * i))
                .fsub_m(rd::<f32>(car, 0x878 + 4 * i))
                .fdiv_m(rd::<f32>(car, 0x888 + 4 * i))
        };
        let surf = rd::<u8>(car, 0x747 + 0x2C * i);
        if dm_get_wheel_status(dm, i as i32) == 2 {
            wr::<u32>(car, r_off, g::<u32>(0x870FD0).read());
            continue;
        }
        if dm_get_wheel_status(dm, i as i32) == 1 {
            let r = rand_at(0x6A4875);
            let a = xi(r & 0xFFFF).fmul_m(g::<f32>(0x858B14).read());
            let b = x(s).fmul_m(g::<f32>(0x858A10).read());
            let n = (b.ftol() as u32 & 0xFFFF) as i32 + 0x62;
            if (a.fmul(xi(n)).ftol() as u32 as i32) >= 100 {
                continue;
            }
            let v = comp()
                .fmul_m(g::<f32>(0x8710B0).read())
                .fadd_m(rd::<f32>(car, r_off));
            wr::<f32>(car, r_off, f32_of(v));
            if !jne_mask(v, x(one), 0x41) {
                wr::<u32>(car, r_off, g::<u32>(0x870FD0).read());
            }
            continue;
        }
        if !x(rd::<f32>(car, r_off)).fcom(x(one)).test_parity(0x05)
            && get_adhesion_group(
                GPtr::from_addr(0xB7_9538),
                u32::from(rd::<u8>(car, 0x747 + 0x2C * i)),
            ) == 4
            && rd::<u16>(car, 0x22) != 0x1B0
        {
            let hf = rd::<u32>(car, 0x38C);
            let k = if hf & 0x20_0000 != 0 {
                g::<f32>(0x8710BC).read()
            } else if hf & 0x10_0000 != 0 {
                g::<f32>(0x8710B8).read()
            } else {
                g::<f32>(0x8710B4).read()
            };
            let m = g::<f32>(0x858CB0).read();
            let q = x(s)
                .fdiv_m(g::<f32>(0x871030).read())
                .fmul_m(m)
                .fsubr_m(g::<f32>(0x858624).read());
            let q = q.fsub(x(g::<f32>(0xC81308).read()).fmul_m(m));
            let floor_q = g::<f32>(0x858EE8).read();
            let q = if jne_mask(x(floor_q), q, 0x41) {
                q
            } else {
                x(floor_q)
            };
            let pq = comp().fmul(q);
            let v = x(k).fmul(pq).fadd_m(rd::<f32>(car, r_off));
            wr::<f32>(car, r_off, f32_of(v));
            if !jne_mask(v, x(one), 0x41) {
                wr::<u32>(car, r_off, g::<u32>(0x870FD0).read());
            }
            continue;
        }
        if x(rd::<f32>(car, r_off))
            .fcom(x(g::<f32>(0x858624).read()))
            .test_parity(0x05)
            || surf != 0xB2
        {
            continue;
        }
        let w = rd::<f32>(mi, if i == 0 || i == 2 { 0x40 } else { 0x44 });
        let mut w2 = x(w)
            .fmul_m(g::<f32>(0x858B8C).read())
            .fdivr_m(g::<f32>(0x8710C4).read());
        let c8 = g::<f32>(0x8710C8).read();
        if !jne_mask(x(s), x(c8), 0x41) {
            w2 = w2.fmul(x(s).fdiv_m(c8));
        }
        let t = f32_of(x(g::<f32>(0x858624).read()).fdiv(w2));
        let x1 = rd::<f32>(car, 0x828 + 4 * i);
        let a = x(t).fmul_m(x1);
        let sa = f32_of(a);
        let f1 = f32_of(x87_floor(a).fsubr_m(sa));
        let x2 = rd::<f32>(car, 0x848 + 4 * i);
        let b = x(g::<f32>(0xB7CB5C).read()).fmul_m(x2).fadd_m(x1).fmul_m(t);
        let tb = f32_of(b);
        let f2 = x87_floor(b).fsubr_m(tb);
        let zero = x(g::<f32>(0x858B50).read());
        let reduce = if !jne_mask(x(x2), zero, 0x41) && !f2.fcom(x(f1)).test_parity(0x05) {
            true
        } else if x(x2).fcom(zero).test_parity(0x05) {
            false
        } else {
            !jne_mask(f2, x(f1), 0x41)
        };
        if reduce {
            let v = comp()
                .fmul_m(g::<f32>(0x8710C0).read())
                .fsubr_m(rd::<f32>(car, r_off));
            let lo = g::<f32>(0x858CC4).read();
            let v = if v.fcom(x(lo)).test_any(0x41) { x(lo) } else { v };
            wr::<f32>(car, r_off, f32_of(v));
        }
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_SCALAR_DELETING_DESTRUCTOR,
    HOOK_DESTRUCTOR,
    HOOK_CUSTOM_CAR_PLATE_BEFORE_RENDERING_START,
    HOOK_CUSTOM_CAR_PLATE_AFTER_RENDERING_STOP,
    HOOK_GET_COMPONENT_WORLD_POSITION,
    HOOK_GET_CAR_NODE_INDEX_FROM_PANEL,
    HOOK_GET_CAR_NODE_INDEX_FROM_DOOR,
    HOOK_PROCESS_CONTROL_COLLISION_CHECK,
    HOOK_SET_BUMPER_DAMAGE,
    HOOK_SET_PANEL_DAMAGE,
    HOOK_SET_DOOR_DAMAGE,
    HOOK_OPEN_DOOR,
    HOOK_DO_BURST_AND_SOFT_GROUND_RATIOS,
];

/// This module's globals (replace with a `sa_core::globals! { list GLOBALS; ... }` block when it gets some).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];
