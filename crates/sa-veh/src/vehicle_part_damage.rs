//! PORT-VEH-3 (batch `veh3-1`): popping and fixing car parts (doors, panels, boot, bonnet) and the
//! plane / quad bike `Fix`. Ported from the 1.0 US bytes; gta-reversed as a reading aid (the bytes
//! win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{rd, wr, x};

orig! {
    /// `CDamageManager::GetPanelStatus(ePanels)` (0x6C2180)
    fn dm_get_panel_status(dm: GPtr<u8>, panel: u32) -> u32 = thiscall 0x6C2180 => crate::damage::get_panel_status;
    /// `CDamageManager::SetPanelStatus(ePanels, uint32)` (0x6C2150)
    fn dm_set_panel_status(dm: GPtr<u8>, panel: u32, status: u32) = thiscall 0x6C2150 => crate::damage::set_panel_status;
    /// `CDamageManager::GetDoorStatus(eDoors)` (0x6C2230)
    fn dm_get_door_status(dm: GPtr<u8>, door: i32) -> u32 = thiscall 0x6C2230 => crate::damage::get_door_status;
    /// `CDamageManager::SetDoorStatus(eDoors, uint32)` (0x6C21C0)
    fn dm_set_door_status(dm: GPtr<u8>, door: i32, status: u32) = thiscall 0x6C21C0 => crate::damage::set_door_status;
    /// `CDamageManager::SetWheelStatus(eCarWheel, uint32)` (0x6C21A0)
    fn dm_set_wheel_status(dm: GPtr<u8>, wheel: i32, status: u32) = thiscall 0x6C21A0 => crate::damage::set_wheel_status;
    /// `CDamageManager::ResetDamageStatus()` (0x6C20E0)
    fn dm_reset_damage_status(dm: GPtr<u8>) = thiscall 0x6C20E0 => crate::damage::reset_damage_status;
    /// `CAutomobile::SpawnFlyingComponent(int32, uint32)` (0x6A8580)
    fn spawn_flying_component(car: GPtr<u8>, node: i32, kind: u32) -> GPtr<u8> = thiscall 0x6A8580 => unported;
    /// `CVehicle::SetComponentVisibility(RwFrame*, uint32)` (0x6D2700)
    fn set_component_visibility(v: GPtr<u8>, frame: GPtr<u8>, state: u32) = thiscall 0x6D2700 => unported;
    /// `CAEVehicleAudioEntity::AddAudioEvent(eAudioEvents, CEntity*)` (0x4F7580)
    fn audio_add_event_entity(a: GPtr<u8>, event: u32, e: GPtr<u8>) = thiscall 0x4F7580 => unported;
    /// `CDoor::Open(float)` (0x6F4790)
    fn door_open(d: GPtr<u8>, ratio: f32) = thiscall 0x6F4790 => crate::door::open;
    /// `CMatrix::CMatrix(RwMatrix*, bool)` (0x59C050)
    fn matrix_ctor_rw(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) -> GPtr<u8> = thiscall 0x59C050 => unported;
    /// `CMatrix::SetTranslate(CVector)` (0x59AF40)
    fn matrix_set_translate(m: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x59AF40 => unported;
    /// `CMatrix::SetRotate(float, float, float)` (0x59B120)
    fn matrix_set_rotate(m: GPtr<u8>, rx: u32, ry: u32, rz: u32) = thiscall 0x59B120 => unported;
    /// `CMatrix::UpdateRW()` (0x59BBB0)
    fn matrix_update_rw(m: GPtr<u8>) = thiscall 0x59BBB0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `RpClumpForAllAtomics(RpClump*, callback, void*)` (0x749B70)
    fn rp_clump_for_all_atomics(clump: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749B70 => unported;
}

const DM: isize = 0x5A0;

/// The node pointer `+0x648[node]` of a car.
fn node(car: GPtr<u8>, n: i32) -> GPtr<u8> {
    rd::<GPtr<u8>>(car, 0x648 + (n as isize) * 4)
}

/// `CAutomobile::PopPanel(eCarNodes, ePanels, bool)` (0x6ADF80): nothing when the panel status is
/// already 3; else, with the effect, a flying component (kind 0 for nodes 5 and 6, else 5), the
/// panel status 3 and the node hidden.
#[hook(addr = 0x6ADF80, conv = thiscall, name = "CAutomobile::PopPanel", this = "CAutomobile",
       expect = "53 55 8B 6C 24 10",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes",
       note = "SpawnFlyingComponent creates an object (pool, world lists)")]
pub fn automobile_pop_panel(this: GPtr<u8>, n: i32, panel: u32, effect: u8) {
    let dm = this.byte_add(DM);
    if dm_get_panel_status(dm, panel) as u8 == 3 {
        return;
    }
    if effect != 0 {
        let kind = if (5..=6).contains(&n) { 0 } else { 5 };
        spawn_flying_component(this, n, kind);
    }
    dm_set_panel_status(dm, panel, 3);
    set_component_visibility(this, node(this, n), 0);
}

/// `CAutomobile::PopDoor(eCarNodes, eDoors, bool)` (0x6ADEF0): nothing when the door status is
/// already 4 (missing); else, with the effect, a flying component (node 0: kind 3 plus the audio
/// event 0x5D with the object; node 1: node 1 kind 4; else the node, kind 2), the door status 4 and
/// the node hidden.
#[hook(addr = 0x6ADEF0, conv = thiscall, name = "CAutomobile::PopDoor", this = "CAutomobile",
       expect = "53 55 8B 6C 24 10",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes",
       note = "SpawnFlyingComponent creates an object (pool, world lists)")]
pub fn automobile_pop_door(this: GPtr<u8>, n: i32, door: i32, effect: u8) {
    let dm = this.byte_add(DM);
    if dm_get_door_status(dm, door) as u8 == 4 {
        return;
    }
    if effect != 0 {
        match n {
            0 => {
                let obj = spawn_flying_component(this, 0, 3);
                audio_add_event_entity(this.byte_add(0x138), 0x5D, obj);
            }
            1 => {
                spawn_flying_component(this, 1, 4);
            }
            _ => {
                spawn_flying_component(this, n, 2);
            }
        }
    }
    dm_set_door_status(dm, door, 4);
    set_component_visibility(this, node(this, n), 0);
}

/// Reset the rotation of a node's modelling matrix (a stack `CMatrix` attached to `frame + 0x10`,
/// `SetTranslate` of its own position, `UpdateRW`).
fn reset_node_rotation(frame: GPtr<u8>) {
    let mut buf = [0u32; 0x12];
    let m: GPtr<u8> = GPtr::from_raw(&mut buf as *mut [u32; 0x12]).cast();
    matrix_ctor_rw(m, frame.byte_add(0x10), 0);
    matrix_set_translate(m, rd::<u32>(m, 0x30), rd::<u32>(m, 0x34), rd::<u32>(m, 0x38));
    matrix_update_rw(m);
    matrix_dtor(m);
}

/// `CAutomobile::FixDoor(eCarNodes, eDoors)` (0x6A35A0): nothing for handling flag 0x100 (the
/// model flag `+0xCC` bit 8); else `CDoor::Open(0)`, door status 0, and when the node exists it is
/// shown and its rotation reset.
#[hook(addr = 0x6A35A0, conv = thiscall, name = "CAutomobile::FixDoor", this = "CAutomobile",
       expect = "64 A1 00 00 00 00",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes",
       note = "writes the node's RwFrame modelling matrix; the SEH frame is not reproduced")]
pub fn automobile_fix_door(this: GPtr<u8>, n: i32, door: i32) {
    if rd::<u32>(rd::<GPtr<u8>>(this, 0x384), 0xCC) & 0x100 != 0 {
        return;
    }
    door_open(this.byte_add(0x5B8 + (door as isize) * 0x18), 0.0);
    dm_set_door_status(this.byte_add(DM), door, 0);
    let f = node(this, n);
    if f.is_null() {
        return;
    }
    set_component_visibility(this, f, 1);
    reset_node_rotation(node(this, n));
}

/// `CAutomobile::FixPanel(eCarNodes, ePanels)` (0x6A3670): panel status 0, every bouncing panel
/// (+0x6AC, 4 x 0x20) on this node gets node id -1, and when the node exists it is shown and its
/// rotation reset.
#[hook(addr = 0x6A3670, conv = thiscall, name = "CAutomobile::FixPanel", this = "CAutomobile",
       expect = "64 A1 00 00 00 00",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes",
       note = "writes the node's RwFrame modelling matrix; the SEH frame is not reproduced")]
pub fn automobile_fix_panel(this: GPtr<u8>, n: i32, panel: u32) {
    dm_set_panel_status(this.byte_add(DM), panel, 0);
    for k in 0..4isize {
        let o = 0x6AC + 0x20 * k;
        if i32::from(rd::<i16>(this, o)) == n {
            wr::<u16>(this, o, 0xFFFF);
        }
    }
    let f = node(this, n);
    if f.is_null() {
        return;
    }
    set_component_visibility(this, f, 1);
    reset_node_rotation(node(this, n));
}

/// Set door 1 (the boot, +0x5D0) to the angle at `this + angle_off` and rotate the boot node
/// (+0x68C, no null check) about the door's axis byte (+0x5DA), keeping its position.
fn set_boot_angle(this: GPtr<u8>, angle_off: isize) {
    wr::<u32>(this, 0x5DC, rd::<u32>(this, angle_off));
    let mut buf = [0u32; 0x12];
    let m: GPtr<u8> = GPtr::from_raw(&mut buf as *mut [u32; 0x12]).cast();
    matrix_ctor_rw(m, rd::<GPtr<u8>>(this, 0x68C).byte_add(0x10), 0);
    // locals [esp+4..]: rot x/y/z (zeroed), the saved position x/y/z; the angle is stored with
    // `fstp [esp + axis*4 + 4]` after x/y were saved and before z is: axis 3/4 overwrite the saved
    // x/y, axis 5 is overwritten by z (axes above 5 hit other stack slots: not modelled, the
    // shipped data uses 0..2)
    let mut loc = [0u32, 0, 0, rd::<u32>(m, 0x30), rd::<u32>(m, 0x34), 0];
    let axis = rd::<u8>(this, 0x5DA) as usize;
    if axis < loc.len() {
        loc[axis] = rd::<u32>(this, 0x5DC);
    }
    loc[5] = rd::<u32>(m, 0x38);
    matrix_set_rotate(m, loc[0], loc[1], loc[2]);
    for k in 0..3usize {
        let o = 0x30 + 4 * k as isize;
        wr::<f32>(m, o, x(rd::<f32>(m, o)).fadd_m(f32::from_bits(loc[3 + k])).to_f32());
    }
    matrix_update_rw(m);
    matrix_dtor(m);
}

/// `CAutomobile::PopBoot()` (0x6AF910): with the boot (door 1) status 0 or 2, the boot angle
/// becomes its open angle (+0x5D0) and the node is rotated to it.
#[hook(addr = 0x6AF910, conv = thiscall, name = "CAutomobile::PopBoot", this = "CAutomobile",
       expect = "64 A1 00 00 00 00",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes",
       note = "writes the boot node's RwFrame modelling matrix; the SEH frame is not reproduced")]
pub fn automobile_pop_boot(this: GPtr<u8>) {
    let s = dm_get_door_status(this.byte_add(DM), 1);
    if s == 0 || s == 2 {
        set_boot_angle(this, 0x5D0);
    }
}

/// `CAutomobile::CloseBoot()` (0x6AFA20): the boot angle becomes its closed angle (+0x5D4) and the
/// node is rotated to it.
#[hook(addr = 0x6AFA20, conv = thiscall, name = "CAutomobile::CloseBoot", this = "CAutomobile",
       expect = "6A FF 68 08 81 84 00",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes",
       note = "writes the boot node's RwFrame modelling matrix; the SEH frame is not reproduced")]
pub fn automobile_close_boot(this: GPtr<u8>) {
    set_boot_angle(this, 0x5D4);
}

/// `CAutomobile::RemoveBonnetInPedCollision()` (0x6AA200): with a bonnet node (+0x688) whose door 0
/// status is 1 or 3 and whose angle (+0x5C4) exceeds 0.4 x the open angle (+0x5B8): a flying
/// bonnet (node 16, kind 2) with audio event 0x5D, the node hidden, door status 4; returns the
/// object, else null.
#[hook(addr = 0x6AA200, conv = thiscall, name = "CAutomobile::RemoveBonnetInPedCollision", this = "CAutomobile",
       expect = "53 56 8B F1 8B 86 88 06 00 00",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes",
       note = "SpawnFlyingComponent creates an object (pool, world lists)")]
pub fn automobile_remove_bonnet_in_ped_collision(this: GPtr<u8>) -> GPtr<u8> {
    if rd::<u32>(this, 0x688) == 0 {
        return GPtr::from_addr(0);
    }
    let dm = this.byte_add(DM);
    if dm_get_door_status(dm, 0) != 1 && dm_get_door_status(dm, 0) != 3 {
        return GPtr::from_addr(0);
    }
    let lim = x(rd::<f32>(this, 0x5B8)).fmul_m(f32::from_bits(0x3ECC_CCCD));
    if lim.fcom(x(rd::<f32>(this, 0x5C4))).test_parity(0x05) {
        return GPtr::from_addr(0);
    }
    let obj = spawn_flying_component(this, 0x10, 2);
    audio_add_event_entity(this.byte_add(0x138), 0x5D, obj);
    set_component_visibility(this, rd::<GPtr<u8>>(this, 0x688), 0);
    dm_set_door_status(dm, 0, 4);
    obj
}

/// `CPlane::Fix()` (0x6CABB0): damage reset; with handling model flag 0x100 the doors 2..5 are set
/// to 4 (missing); +0x42A bit 0 cleared.
#[hook(addr = 0x6CABB0, conv = thiscall, name = "CPlane::Fix", this = "CPlane",
       expect = "56 57 8B F9 8D B7 A0 05 00 00",
       unverified = "veh3-1", route = "L2", hazards = "none")]
pub fn plane_fix(this: GPtr<u8>) {
    let dm = this.byte_add(DM);
    dm_reset_damage_status(dm);
    if rd::<u32>(rd::<GPtr<u8>>(this, 0x384), 0xCC) & 0x100 != 0 {
        for d in 2..6 {
            dm_set_door_status(dm, d, 4);
        }
    }
    wr::<u8>(this, 0x42A, rd::<u8>(this, 0x42A) & 0xFE);
}

/// `CQuadBike::Fix()` (0x6CE2B0): damage reset, doors 2..5 = 4 (missing), +0x42A bit 0 cleared,
/// every atomic through `CVehicleModelInfo::HideAllComponentsAtomicCB` (0x4C7790) with flag 2, the
/// four wheels status 0.
#[hook(addr = 0x6CE2B0, conv = thiscall, name = "CQuadBike::Fix", this = "CQuadBike",
       expect = "56 57 8B F9 8D B7 A0 05 00 00",
       unverified = "veh3-1", route = "L2", hazards = "heap-writes",
       note = "the atomic callback writes the RpAtomic flags")]
pub fn quad_bike_fix(this: GPtr<u8>) {
    let dm = this.byte_add(DM);
    dm_reset_damage_status(dm);
    for d in 2..6 {
        dm_set_door_status(dm, d, 4);
    }
    wr::<u8>(this, 0x42A, rd::<u8>(this, 0x42A) & 0xFE);
    rp_clump_for_all_atomics(rd::<GPtr<u8>>(this, 0x18), 0x004C_7790, 2);
    for w in 0..4 {
        dm_set_wheel_status(dm, w, 0);
    }
}

/// This module's registrations.
pub const HOOKS: &[HookReg] = &[
    HOOK_AUTOMOBILE_POP_PANEL,
    HOOK_AUTOMOBILE_POP_DOOR,
    HOOK_AUTOMOBILE_FIX_DOOR,
    HOOK_AUTOMOBILE_FIX_PANEL,
    HOOK_AUTOMOBILE_POP_BOOT,
    HOOK_AUTOMOBILE_CLOSE_BOOT,
    HOOK_AUTOMOBILE_REMOVE_BONNET_IN_PED_COLLISION,
    HOOK_PLANE_FIX,
    HOOK_QUAD_BIKE_FIX,
];
