//! Small script helpers the ported command switches still reached through the original (FRONTIER-2 lane C, batch
//! `frontier2-c2`). Ported from the 1.0 US bytes (the `.HOODLUM` bodies included). Unverified (docs/verify/HOWTO.md).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::field::at;
use sa_core::hook;
use sa_core::mem::GPtr;

/// `CRunningScript::ThisIsAValidRandomPed(ePedType type, bool civilian, bool gang, bool criminal)` (0x489490,
/// thiscall, `ret 0x10`; byte table 0x4894E8 over `type - 4`, dword table 0x4894D8): types 4 / 5 → `civilian`, 7..15
/// → `gang`, 20 / 22 → `criminal` (each as a whole dword, non-zero → 1); every other type, and anything outside 4..22
/// → 0. AL.
#[hook(addr = 0x489490, conv = thiscall, name = "CRunningScript::ThisIsAValidRandomPed", expect = "8B 44 24 04 83 C0 FC",
       unverified = "frontier2-c2", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn this_is_a_valid_random_ped(_this: GPtr<u8>, ty: u32, civilian: u32, gang: u32, criminal: u32) -> u8 {
    let t = ty.wrapping_sub(4);
    if t > 0x12 {
        return 0;
    }
    const CASE: [u8; 0x13] = [0, 0, 3, 1, 1, 1, 1, 1, 1, 1, 1, 1, 3, 3, 3, 3, 2, 3, 2];
    let v = match CASE[t as usize] {
        0 => civilian,
        1 => gang,
        2 => criminal,
        _ => 0,
    };
    u8::from(v != 0)
}

/// `CScriptsForBrains::SwitchAllObjectBrainsWithThisID(int8 id, bool on)` (stub 0x46A900 → .HOODLUM 0x156FA50,
/// thiscall, `ret 8`): for a non-negative id, every one of the 0x46 brains whose id byte (+3) equals it gets its
/// active byte (+4) = `on`.
#[hook(addr = 0x46A900, conv = thiscall, name = "CScriptsForBrains::SwitchAllObjectBrainsWithThisID",
       expect = "E9 4B 51 10 01", unverified = "frontier2-c2", route = "L2", hazards = "none")]
pub fn switch_all_object_brains_with_this_id(this: GPtr<u8>, id: u32, on: u32) {
    let id = id as u8;
    if (id as i8) < 0 {
        return;
    }
    for i in 0..0x46isize {
        let e = this.byte_add(i * 0x14);
        if at::<u8>(e, 3).read() == id {
            at::<u8>(e, 4).write(on as u8);
        }
    }
}

/// The script attractor record's ped-type slot claim (stub 0x492CB0 → .HOODLUM 0x156E990, thiscall, `ret 4`; unnamed in
/// gta-reversed): `+0x20 = 1`; the first of the four handle slots (+0) holding -1 becomes -2 and its ped type (+0x10
/// + 4i) the argument; the others are left alone.
#[hook(addr = 0x492CB0, conv = thiscall, name = "thunk_FUN_0156e990", expect = "E9 DB BC 0D 01",
       unverified = "frontier2-c2", route = "L2", hazards = "none")]
pub fn script_attractor_add_ped_type(this: GPtr<u8>, ped_type: u32) {
    at::<u8>(this, 0x20).write(1);
    for i in 0..4isize {
        if at::<i32>(this, 4 * i).read() == -1 {
            at::<i32>(this, 4 * i).write(-2);
            at::<u32>(this, 0x10 + 4 * i).write(ped_type);
            return;
        }
    }
}

/// `CStreamedScripts::FindStreamedScriptQuiet(const char* name)` (0x4706F0, thiscall, `ret 4`): the index of the first
/// of the `+0xA44` (word) streamed scripts whose name (`+8 + 0x20 i`) `_stricmp`s equal, else -1.
#[hook(addr = 0x4706F0, conv = thiscall, name = "CStreamedScripts::FindStreamedScriptQuiet", expect = "53 55 56 8B D9 33 F6",
       unverified = "frontier2-c3", route = "L2", hazards = "none")]
pub fn find_streamed_script_quiet(this: GPtr<u8>, name: GPtr<u8>) -> i32 {
    let mut i = 0i32;
    while i < i32::from(at::<u16>(this, 0xA44).read()) {
        if stricmp(this.byte_add(8 + 0x20 * i as isize), name) == 0 {
            return i;
        }
        i += 1;
    }
    -1
}

/// `CScripted2dEffects::AddScripted2DEffect(float radius)` (0x6FA7C0, cdecl): the first free of the 64 slots (used byte
/// 0xC3A1A0 + i) is claimed: used = 1, `ms_userLists`-style dword 0xC3A020 + 4i = -1, the 36-byte user list at 0xC3A200
/// + 36i gets its eight dwords = -1 and +0x20 = 0, the effect pair 0xC3BB00 + 0x94 i is reset (0x6F9EB0), the byte
/// 0xC39FE0 + i = 0 and the radius (fld / fstp) at 0xC39EE0 + 4i; returns the slot. No free slot → -1.
#[hook(addr = 0x6FA7C0, conv = cdecl, name = "CScripted2dEffects::AddScripted2DEffect", expect = "83 CA FF 56 8B C2",
       unverified = "frontier2-c3", route = "L2", hazards = "none")]
pub fn add_scripted_2d_effect(radius: u32) -> i32 {
    for i in 0..0x40usize {
        let used = GPtr::<u8>::from_addr(0xC3A1A0 + i);
        if used.read() != 0 {
            continue;
        }
        used.write(1);
        GPtr::<i32>::from_addr(0xC3A020 + 4 * i).write(-1);
        let list = GPtr::<u8>::from_addr(0xC3A200 + 36 * i);
        for k in 0..4isize {
            at::<i32>(list, 4 * k).write(-1);
            at::<i32>(list, 0x10 + 4 * k).write(-1);
        }
        at::<u8>(list, 0x20).write(0);
        effect_2d_init(GPtr::from_addr(0xC3BB00 + 0x94 * i));
        GPtr::<u8>::from_addr(0xC39FE0 + i).write(0);
        GPtr::<u32>::from_addr(0xC39EE0 + 4 * i).write(sa_math::fpu::fld_fst(f32::from_bits(radius)).to_bits());
        return i as i32;
    }
    -1
}

/// `CScripted2dEffects::GetEffectPairs(const C2dEffect* effect)` (0x6FA840, cdecl): the effect's index among the 64
/// 0x40-byte effects at 0xC3AB00 selects the pair `0xC3BB00 + 0x94 i`; an unknown effect gives index -1 (0xC3BA6C).
#[hook(addr = 0x6FA840, conv = cdecl, name = "CScripted2dEffects::GetEffectPairs", expect = "56 8B 74 24 08 83 C8 FF",
       unverified = "frontier2-c3", route = "L2", hazards = "none")]
pub fn get_effect_pairs(effect: GPtr<u8>) -> GPtr<u8> {
    let i = (0..0x40u32).find(|i| effect.addr() as u32 == 0xC3AB00 + 0x40 * i).map_or(-1, |i| i as i32);
    GPtr::from_addr((i as u32).wrapping_mul(0x94).wrapping_add(0xC3BB00) as usize)
}

/// `CTheScripts::AddScriptSphere(uint32 id, CVector pos, float radius)` (stub 0x483B30 → .HOODLUM 0x156E050, cdecl): the
/// first free of the 0x18-byte spheres at 0xA91268 (slot 0 tested first, then slots 1..16 by their used byte; a full
/// array ends on index 16, as the original) gets used = 1, id (+4) = `id + index`, the position (+8) and radius
/// (+0x14) as raw dwords; returns `GetNewUniqueScriptThingIndex(index, 0)`.
#[hook(addr = 0x483B30, conv = cdecl, name = "CTheScripts::AddScriptSphere", expect = "E9 1B A5 0E 01",
       unverified = "frontier2-c3", route = "L2", hazards = "none")]
pub fn add_script_sphere(id: u32, x: u32, y: u32, z: u32, radius: u32) -> i32 {
    const SPHERES: usize = 0xA91268;
    let mut index = 0u32;
    if GPtr::<u8>::from_addr(SPHERES).read() != 0 {
        let mut off = 0u32;
        loop {
            if off >= 0x180 {
                break;
            }
            let used = GPtr::<u8>::from_addr(SPHERES + 0x18 + off as usize).read();
            off += 0x18;
            index += 1;
            if used == 0 {
                break;
            }
        }
    }
    let e = GPtr::<u8>::from_addr(SPHERES + 0x18 * index as usize);
    at::<u32>(e, 4).write(id.wrapping_add(index));
    at::<u32>(e, 8).write(x);
    at::<u32>(e, 0xC).write(y);
    at::<u32>(e, 0x10).write(z);
    at::<u8>(e, 0).write(1);
    at::<u32>(e, 0x14).write(radius);
    get_new_unique_script_thing_index(index as i32, 0)
}

/// `CTheScripts::IsEntityWithinSearchLight(int32 handle, CEntity* entity)` (stub 0x493900 → .HOODLUM 0x156C000, cdecl):
/// the search light's index (`GetActualScriptThingIndex(handle, 2)`); a valid one → `IsPointWithinSearchLight(&pos,
/// index)` on the entity's position (a copy on the stack). AL.
#[hook(addr = 0x493900, conv = cdecl, name = "CTheScripts::IsEntityWithinSearchLight", expect = "E9 FB 86 0D 01",
       unverified = "frontier2-c3", route = "L2", hazards = "none")]
pub fn is_entity_within_search_light(handle: u32, entity: GPtr<u8>) -> u8 {
    let index = get_actual_script_thing_index(handle, 2);
    if index < 0 {
        return 0;
    }
    let p = sa_world::world::common::entity_pos(entity);
    let mut pos = [at::<u32>(p, 0).read(), at::<u32>(p, 4).read(), at::<u32>(p, 8).read()];
    u8::from(is_point_within_search_light(GPtr::from_raw(pos.as_mut_ptr()).cast(), index) != 0)
}

use crate::bindings::get_new_unique_script_thing_index;
use sa_core::bindings::stricmp;
use sa_render::bindings::effect_2d_init;
use sa_world::bindings::get_actual_script_thing_index;
sa_core::orig! {
    /// `CTheScripts::IsPointWithinSearchLight(const CVector& point, int32 index)` (0x493280, cdecl, AL)
    fn is_point_within_search_light(point: GPtr<u8>, index: i32) -> u8 = cdecl 0x493280 => unported;
}

/// The patrol-point anim record constructor (stub 0x46A340 → .HOODLUM 0x156E1D0, thiscall, `ret 8`; unnamed): `strcpy`
/// of the anim name to +0 and of the block name to +0x18 (byte loops, the terminator included).
#[hook(addr = 0x46A340, conv = thiscall, name = "thunk_FUN_0156e1d0", expect = "E9 8B 3E 10 01",
       unverified = "frontier2-c3", route = "L2", hazards = "none")]
pub fn patrol_anim_record_ctor(this: GPtr<u8>, anim: GPtr<u8>, block: GPtr<u8>) {
    for (dst, src) in [(this, anim), (this.byte_add(0x18), block)] {
        let mut k = 0isize;
        loop {
            let b = at::<u8>(src, k).read();
            at::<u8>(dst, k).write(b);
            k += 1;
            if b == 0 {
                break;
            }
        }
    }
}

/// `CPlayerInfo` of the player in focus (`CWorld::Players` 0xB7CD98, 0x190 each; `CWorld::PlayerInFocus` byte 0xB7CD74),
/// re-read at every use as the original does.
fn focus_player() -> GPtr<u8> {
    GPtr::from_addr(0xB7CD98 + usize::from(GPtr::<u8>::from_addr(0xB7CD74).read()) * 0x190)
}

/// `CRemote::TakeRemoteControlledCarFromPlayer(bool blowUp)` (stub 0x45AE80 → .HOODLUM 0x1562160, cdecl): a mission
/// remote car (created-by +0x4A4 == 2) of the focus player (`m_pRemoteVehicle` +0xB0) becomes random (`SetVehicleCreatedBy
/// (1)`) and leaves the mission cleanup list (`RemoveEntityFromList(handle, 1)`, the handle from the vehicle pool inline);
/// then the car's flag bit 3 of +0x428 is cleared and the player's +0xE0 = now, +0xDD = 1, +0xDE = `blowUp`, +0xDF = 1.
#[hook(addr = 0x45AE80, conv = cdecl, name = "CRemote::TakeRemoteControlledCarFromPlayer", expect = "E9 DB 72 10 01",
       unverified = "frontier2-c4", route = "L2", hazards = "heap-writes")]
pub fn take_remote_controlled_car_from_player(blow_up: u32) {
    let veh = at::<GPtr<u8>>(focus_player(), 0xB0).read();
    if at::<u8>(veh, 0x4A4).read() == 2 {
        vehicle_set_created_by(veh, 1);
        let veh = at::<GPtr<u8>>(focus_player(), 0xB0).read();
        let pool = GPtr::<GPtr<u8>>::from_addr(0xB74494).read();
        let idx = (veh.addr() as u32).wrapping_sub(at::<u32>(pool, 0).read()) as i32 / 0xA18;
        let flags = at::<GPtr<u8>>(pool, 4).read().byte_add(idx as isize).read();
        cleanup_remove_entity_from_list(GPtr::from_addr(0xA90850), (idx << 8).wrapping_add(i32::from(flags)), 1);
    }
    let veh = at::<GPtr<u8>>(focus_player(), 0xB0).read();
    at::<u8>(veh, 0x428).write(at::<u8>(veh, 0x428).read() & 0xF7);
    let p = focus_player();
    at::<u32>(p, 0xE0).write(GPtr::<u32>::from_addr(0xB7CB84).read());
    at::<u8>(p, 0xDD).write(1);
    at::<u8>(p, 0xDE).write(blow_up as u8);
    at::<u8>(p, 0xDF).write(1);
}

/// `CRemote::GivePlayerRemoteControlledCar(float x, float y, float z, float angle, uint16 model)` (0x45AB10, cdecl;
/// FRONTIER-2 lane C turn 5): a new vehicle (`operator new` 0x6E2D50) built as a `CHeli` (0xA18 bytes), `CPlane`
/// (0xA04) or `CAutomobile` (0x988, setup 1), all created-by 2 (an allocation failure leaves NULL, which the original
/// dereferences). `z = FindGroundZFor3DCoord(x, y, z + 2, &model-argument, NULL) + GetDistanceFromCentreOfMassToBase
/// OfModel()`, `SetRotateZOnly(angle)` on its matrix, the position into the matrix (or the placement); status 8 (byte
/// +0x36 high bits 0x40), +0x428 |= 8, `JoinCarWithRoadSystem`, +0x3BA / +0x3BB / +0x3B9 = 0, +0x3C8 = 9.0, +0x3D0 = 9,
/// +0x3B7 / +0x3B8 = 0, +0x428 bit 0x10 = !(+0x42D & 2); `CWorld::Add`; the player's vehicle (if any) gets status 9;
/// the focus player's `m_pRemoteVehicle` (+0xB0) = the car and registers the reference; the camera takes control
/// (mode 0x12, jump cut, script) and `SetZoomValueCamStringScript(1)`.
#[hook(addr = 0x45AB10, conv = cdecl, name = "CRemote::GivePlayerRemoteControlledCar", expect = "6A FF 68 11 7A 83 00",
       unverified = "frontier2-c11", route = "L2", hazards = "heap-writes")]
pub fn give_player_remote_controlled_car(x: f32, y: f32, z: f32, angle: f32, model: u32) {
    let mut model_slot = model;
    let mi = (model & 0xFFFF) as i32;
    let veh = if crate::bindings::is_heli_model(mi) {
        let v = sa_world::bindings::vehicle_new(0xA18);
        if v.is_null() { v } else { sa_core::bindings::heli_ctor(v, mi, 2) }
    } else if crate::bindings::is_plane_model(mi) {
        let v = sa_world::bindings::vehicle_new(0xA04);
        if v.is_null() { v } else { sa_core::bindings::plane_ctor(v, mi, 2) }
    } else {
        let v = sa_world::bindings::vehicle_new(0x988);
        if v.is_null() { v } else { sa_core::bindings::automobile_ctor(v, mi as u32, 2, 1) }
    };
    let zg = sa_math::x87::X87::from_f32(z).fadd_m(GPtr::<f32>::from_addr(0x858CA0).read()).to_f32();
    let flag = GPtr::<u8>::from_raw(core::ptr::addr_of_mut!(model_slot).cast::<u8>());
    let z = sa_math::x87::X87::from_f64_exact(sa_world::bindings::find_ground_z_for_3d_coord(x, y, zg, flag, GPtr::NULL))
        .to_f32();
    let z = sa_math::x87::X87::from_f64_exact(sa_world::bindings::get_distance_from_centre_of_mass_to_base_of_model(veh))
        .fadd_m(z)
        .to_f32();
    sa_core::bindings::matrix_set_rotate_z_only(at::<GPtr<u8>>(veh, 0x14).read(), angle);
    let m = at::<GPtr<u8>>(veh, 0x14).read();
    if !m.is_null() {
        at::<f32>(m, 0x30).write(x);
        at::<f32>(at::<GPtr<u8>>(veh, 0x14).read(), 0x34).write(y);
        at::<f32>(at::<GPtr<u8>>(veh, 0x14).read(), 0x38).write(z);
    } else {
        at::<f32>(veh, 4).write(x);
        at::<f32>(veh, 8).write(y);
        at::<f32>(veh, 0xC).write(z);
    }
    at::<u8>(veh, 0x36).write(at::<u8>(veh, 0x36).read() & 7 | 0x40);
    at::<u8>(veh, 0x428).write(at::<u8>(veh, 0x428).read() | 8);
    sa_pop::bindings::join_car_with_road_system(veh);
    remote_car_state(veh);
    sa_core::bindings::world_add(veh);
    disable_player_vehicle();
    let slot = focus_player().byte_add(0xB0);
    slot.cast::<GPtr<u8>>().write(veh);
    sa_core::bindings::entity_register_reference(veh, slot);
    remote_car_camera(veh, 1);
}

/// The remote car's AI state shared by `GivePlayerRemoteControlledCar` / `TakeRemoteControlOfCar`: +0x3BA / +0x3BB /
/// +0x3B9 = 0, +0x3C8 = 9.0, +0x3D0 = 9, +0x3B7 / +0x3B8 = 0, then +0x428 bit 0x10 = !(+0x42D & 2) (both read first).
fn remote_car_state(veh: GPtr<u8>) {
    let f = at::<u8>(veh, 0x428).read();
    let no_engine = at::<u8>(veh, 0x42D).read() & 2 != 0;
    at::<u8>(veh, 0x3BA).write(0);
    at::<u8>(veh, 0x3BB).write(0);
    at::<u8>(veh, 0x3B9).write(0);
    at::<u32>(veh, 0x3C8).write(0x4110_0000);
    at::<u8>(veh, 0x3D0).write(9);
    at::<u8>(veh, 0x3B7).write(0);
    at::<u8>(veh, 0x3B8).write(0);
    at::<u8>(veh, 0x428).write(if no_engine { f & 0xEF } else { f | 0x10 });
}

/// The player's own vehicle (`FindPlayerVehicle(-1, false)`, called again when there is one) gets status 9.
fn disable_player_vehicle() {
    if !sa_core::bindings::find_player_vehicle(-1, 0).is_null() {
        let pv = sa_core::bindings::find_player_vehicle(-1, 0);
        at::<u8>(pv, 0x36).write(at::<u8>(pv, 0x36).read() & 7 | 0x48);
    }
}

/// `TheCamera.TakeControl(veh, 0x12, switch, 1)` and `SetZoomValueCamStringScript(1)`.
fn remote_car_camera(veh: GPtr<u8>, switch: u32) {
    sa_world::bindings::camera_take_control(GPtr::from_addr(0xB6F028), veh, 0x12, switch, 1);
    crate::bindings::set_zoom_value_cam_string_script(GPtr::from_addr(0xB6F028), 1);
}

/// `CRemote::TakeRemoteControlOfCar(CVehicle* veh)` (stub 0x45AD40 → .HOODLUM 0x156E620, cdecl; FRONTIER-2 lane C turn
/// 5): a current remote car of the focus player (+0xB0) gets status 3, and the switch becomes 2 when it is not `veh`;
/// `veh` gets status 8, +0x428 |= 8 and the remote AI state ([`remote_car_state`]); the player's vehicle status 9; the
/// old reference is cleaned up (`CleanUpOldReference`), +0xB0 = `veh` registered; the camera takes control with the
/// switch (else 1: the value the no-CD VM 0xCB73E0 pushes for descriptor 0x1560094, evaluated on the exe image).
#[hook(addr = 0x45AD40, conv = cdecl, name = "thunk_FUN_0156e620", expect = "E9 DB 38 11 01",
       unverified = "frontier2-c11", route = "L2", hazards = "heap-writes")]
pub fn take_remote_control_of_car(veh: GPtr<u8>) {
    let mut other = false;
    let old = at::<GPtr<u8>>(focus_player(), 0xB0).read();
    if !old.is_null() {
        at::<u8>(old, 0x36).write(at::<u8>(old, 0x36).read() & 7 | 0x18);
        if at::<GPtr<u8>>(focus_player(), 0xB0).read() != veh {
            other = true;
        }
    }
    at::<u8>(veh, 0x36).write(at::<u8>(veh, 0x36).read() & 7 | 0x40);
    at::<u8>(veh, 0x428).write(at::<u8>(veh, 0x428).read() | 8);
    remote_car_state(veh);
    disable_player_vehicle();
    let slot = focus_player().byte_add(0xB0);
    let cur = slot.cast::<GPtr<u8>>().read();
    if !cur.is_null() {
        sa_core::bindings::entity_clean_up_old_reference(cur, slot);
    }
    let slot = focus_player().byte_add(0xB0);
    slot.cast::<GPtr<u8>>().write(veh);
    sa_core::bindings::entity_register_reference(veh, slot);
    remote_car_camera(veh, if other { 2 } else { 1 });
}

/// `CPatrolRoute::AddRoutePoint(const CVector& pos, const record& anim)` (stub 0x46AE80 → .HOODLUM 0x15622E0, thiscall,
/// `ret 8`; unnamed): with fewer than 8 points (+0), the point is copied to `+0x144 + 12n`, the record's anim name (+0)
/// and block name (+0x18) `strcpy`d into the 0x28-byte record at `+4 + 0x28n`, the count incremented; AL = 1. A full
/// route → AL = 0.
#[hook(addr = 0x46AE80, conv = thiscall, name = "thunk_FUN_015622e0", expect = "E9 5B 74 0F 01",
       unverified = "frontier2-c4", route = "L2", hazards = "none")]
pub fn patrol_route_add_point(this: GPtr<u8>, pos: GPtr<u8>, rec: GPtr<u8>) -> u8 {
    let n = at::<i32>(this, 0).read();
    if n >= 8 {
        return 0;
    }
    let dst = this.byte_add(0x144 + 12 * n as isize);
    for k in 0..3isize {
        at::<u32>(dst, 4 * k).write(at::<u32>(pos, 4 * k).read());
    }
    let r = this.byte_add(4 + 0x28 * at::<i32>(this, 0).read() as isize);
    for (d, s) in [(r, rec), (r.byte_add(0x18), rec.byte_add(0x18))] {
        let mut k = 0isize;
        loop {
            let b = at::<u8>(s, k).read();
            at::<u8>(d, k).write(b);
            k += 1;
            if b == 0 {
                break;
            }
        }
    }
    at::<i32>(this, 0).write(at::<i32>(this, 0).read() + 1);
    1
}

use sa_ai::bindings::cleanup_remove_entity_from_list;
use sa_world::bindings::vehicle_set_created_by;

/// `CTheScripts::IsEntityWithinAnySearchLight(CEntity* entity, int32* out)` (0x493960, cdecl): `*out = -1`; the first
/// of the 8 search lights (0x7C bytes from 0xA94D68) in use (byte +0) whose cone holds the entity's position (a stack
/// copy; `IsPointWithinSearchLight(&pos, i)`) gives `*out = (id word +4 << 16) | i` and AL = 1; none → AL = 0.
#[hook(addr = 0x493960, conv = cdecl, name = "CTheScripts::IsEntityWithinAnySearchLight", expect = "8B 4C 24 04 83 EC 0C",
       unverified = "frontier2-c4", route = "L2", hazards = "none")]
pub fn is_entity_within_any_search_light(entity: GPtr<u8>, out: GPtr<u32>) -> u8 {
    out.write(0xFFFF_FFFF);
    let p = sa_world::world::common::entity_pos(entity);
    let mut pos = [at::<u32>(p, 0).read(), at::<u32>(p, 4).read(), at::<u32>(p, 8).read()];
    for i in 0..8u32 {
        let light = GPtr::<u8>::from_addr(0xA94D68 + 0x7C * i as usize);
        if at::<u8>(light, 0).read() != 0
            && is_point_within_search_light(GPtr::from_raw(pos.as_mut_ptr()).cast(), i as i32) != 0
        {
            out.write((u32::from(at::<u16>(light, 4).read()) << 16) | i);
            return 1;
        }
    }
    0
}

/// `CTheScripts::StartNewScript(uint8* ip, uint16 index)` (stub 0x464C90 → .HOODLUM 0x156E210, cdecl): the script
/// `ScriptsArray[index]` (0xA8B430 + 0xE0 index) is looked for in the idle list (`pIdleScripts` 0xA8B428, the head
/// address decoded in line); absent → NULL. Found: unlinked (its previous script's next, or the idle head, takes its
/// next; the next's previous takes its previous), `Init()`ed, its IP (+0x14) set, linked in front of the active list
/// (`pActiveScripts` 0xA8B42C) and marked active (+0xC4 = 1). Returns it.
#[hook(addr = 0x464C90, conv = cdecl, name = "CTheScripts::StartNewScript-indexed", expect = "E9 7B 95 10 01",
       unverified = "frontier2-c4", route = "L2", hazards = "heap-writes", recs = "save-load")]
pub fn start_new_script_indexed(ip: u32, index: u32) -> GPtr<u8> {
    let script = GPtr::<u8>::from_addr(0xA8B430 + 0xE0 * (index & 0xFFFF) as usize);
    let idle = GPtr::<GPtr<u8>>::from_addr(0xA8B428);
    let mut s = idle.read();
    while !s.is_null() && s != script {
        s = at::<GPtr<u8>>(s, 0).read();
    }
    if s.is_null() {
        return GPtr::NULL;
    }
    let prev = at::<GPtr<u8>>(s, 4).read();
    if prev.is_null() {
        idle.write(at::<GPtr<u8>>(s, 0).read());
    } else {
        at::<GPtr<u8>>(prev, 0).write(at::<GPtr<u8>>(s, 0).read());
    }
    let next = at::<GPtr<u8>>(s, 0).read();
    if !next.is_null() {
        at::<GPtr<u8>>(next, 4).write(at::<GPtr<u8>>(s, 4).read());
    }
    running_script_init(s);
    at::<u32>(s, 0x14).write(ip);
    let active = GPtr::<GPtr<u8>>::from_addr(0xA8B42C);
    at::<GPtr<u8>>(s, 0).write(active.read());
    at::<u32>(s, 4).write(0);
    let head = active.read();
    if !head.is_null() {
        at::<GPtr<u8>>(head, 4).write(s);
    }
    active.write(s);
    at::<u8>(s, 0xC4).write(1);
    s
}

use crate::bindings::running_script_init;

/// The streamed-script release (0x40C1C0, cdecl; unnamed, script command 0910 REMOVE_STREAMED_SCRIPT's callee):
/// `CStreaming::RemoveModel(index + 0x6676)` (the streamed scripts' model ids start at 26230). The original adds into
/// its own argument slot and tail-jumps (FRONTIER-2 turn 2).
#[hook(addr = 0x40C1C0, conv = cdecl, name = "FUN_0040c1c0", expect = "8B 44 24 04 05 76 66 00 00",
       unverified = "frontier2-c5", route = "L2", hazards = "heap-writes")]
pub fn remove_streamed_script(index: i32) {
    sa_core::bindings::streaming_remove_model(index.wrapping_add(0x6676));
}

/// `CTheScripts::IsPointWithinSearchLight(const CVector& point, int32 index)` (0x493280, cdecl, AL; FRONTIER-2 turn 2).
/// The search light `index` (0x7C each from 0xA94DC0: position +0, two direction vectors +0xC and +0x18): local copies
/// of both directions are normalised (`CVector::NormaliseAndMag`; the first magnitude kept as f32, the second on the x87
/// stack); with `d = point - position` (x87), `a = (d.z*A.z + d.y*A.y + d.x*A.x) / |A|` (stored f32) and `b = (d.z*B.z +
/// d.y*B.y + d.x*B.x) / |B|`; AL = 1 when `b*b + a*a <= 1.0` (`test ah, 0x41` / `jnp`), else 0 (NaN → 0).
#[hook(addr = 0x493280, conv = cdecl, name = "CTheScripts::IsPointWithinSearchLight", expect = "83 EC 18 53 56",
       unverified = "frontier2-c5", route = "L2", hazards = "none")]
pub fn is_point_within_search_light_port(point: GPtr<u8>, index: i32) -> u8 {
    let sl = GPtr::<u8>::from_addr(0xA94DC0).byte_add(index.wrapping_mul(0x7C) as isize);
    let mut a: [u32; 3] = core::array::from_fn(|k| at::<u32>(sl, 0xC + 4 * k as isize).read());
    let mut b: [u32; 3] = core::array::from_fn(|k| at::<u32>(sl, 0x18 + 4 * k as isize).read());
    let ap: GPtr<u8> = GPtr::from_raw(&raw mut a).cast();
    let bp: GPtr<u8> = GPtr::from_raw(&raw mut b).cast();
    let mag_a = vector_normalise_and_mag(ap).to_f32();
    let mag_b = vector_normalise_and_mag(bp);
    let f = |p: GPtr<u8>, k: isize| at::<f32>(p, 4 * k).read();
    let d: [X87; 3] = core::array::from_fn(|k| X87::from_f32(f(point, k as isize)).fsub_m(f(sl, k as isize)));
    let dot = |v: GPtr<u8>| X87::from_f32(f(v, 2)).fmul(d[2]).fadd(X87::from_f32(f(v, 1)).fmul(d[1])).fadd(X87::from_f32(f(v, 0)).fmul(d[0]));
    let pa = dot(ap).fdiv_m(mag_a).to_f32();
    let pb = dot(bp).fdiv(mag_b);
    let r = pb.fmul(pb).fadd(X87::from_f32(pa).fmul_m(pa));
    u8::from(!r.fcom(X87::from_f32(1.0)).test_parity(0x41))
}

use sa_core::bindings::vector_normalise_and_mag;
use sa_math::x87::X87;

/// Every `#[hook]` of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_IS_ENTITY_WITHIN_ANY_SEARCH_LIGHT,
    HOOK_START_NEW_SCRIPT_INDEXED,
    HOOK_TAKE_REMOTE_CONTROLLED_CAR_FROM_PLAYER,
    HOOK_GIVE_PLAYER_REMOTE_CONTROLLED_CAR,
    HOOK_TAKE_REMOTE_CONTROL_OF_CAR,
    HOOK_PATROL_ROUTE_ADD_POINT,
    HOOK_PATROL_ANIM_RECORD_CTOR,
    HOOK_FIND_STREAMED_SCRIPT_QUIET,
    HOOK_ADD_SCRIPTED_2D_EFFECT,
    HOOK_GET_EFFECT_PAIRS,
    HOOK_ADD_SCRIPT_SPHERE,
    HOOK_IS_ENTITY_WITHIN_SEARCH_LIGHT,
    HOOK_THIS_IS_A_VALID_RANDOM_PED,
    HOOK_SWITCH_ALL_OBJECT_BRAINS_WITH_THIS_ID,
    HOOK_SCRIPT_ATTRACTOR_ADD_PED_TYPE,
    HOOK_REMOVE_STREAMED_SCRIPT,
    HOOK_IS_POINT_WITHIN_SEARCH_LIGHT_PORT,
];
