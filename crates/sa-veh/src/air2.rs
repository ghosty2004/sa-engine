//! PORT-VEH-2 (docs/arch/veh-port.md, second pass): `CHeli` and `CPlane`. Ported from the 1.0 US bytes;
//! gta-reversed and docs/re/specs/VEH as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use crate::common::{g, rd, wr, x, xi};

/// `CHeli*` / `CPlane*`.
type Ptr = GPtr<u8>;

/// `CTimer::m_snTimeInMilliseconds`.
fn now() -> u32 {
    g::<u32>(0xB7CB84).read()
}

sa_core::globals! {
    list GLOBALS;
    /// `CHeli::TestForNewRandomHelisTimer`
    pub static TEST_FOR_NEW_RANDOM_HELIS_TIMER @ 0xC1C960: u32 = 0;
}

orig! {
    /// `FindPlayerWanted(int32)` (0x56E230)
    pub fn find_player_wanted(id: i32) -> GPtr<u8> = cdecl 0x56E230 => unported;
    /// `CWanted::NumOfHelisRequired()` (0x561FA0)
    pub fn num_of_helis_required(w: GPtr<u8>) -> i32 = thiscall 0x561FA0 => unported;
    /// `CCullZones::PlayerNoRain()` (0x72DDC0)
    pub fn player_no_rain() -> u8 = cdecl 0x72DDC0 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CHeli::GenerateHeli(CPed*, bool)` (0x6C6520)
    pub fn generate_heli(ped: GPtr<u8>, police: u32) -> GPtr<u8> = cdecl 0x6C6520 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    pub fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `FindPlayerCoors(CVector*, int32)` (0x56E010)
    pub fn find_player_coors(out: GPtr<u8>, id: i32) -> GPtr<u8> = cdecl 0x56E010 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    pub fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// Virtual slot 0 (`scalar deleting destructor`)
    pub fn entity_vdelete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    /// `CVehicle::Render()` (0x6D0E60)
    pub fn vehicle_render(v: GPtr<u8>) = thiscall 0x6D0E60 => crate::vehicle2::render;
    /// `CVehicle::operator delete(void*)` (0x6E2D90)
    pub fn vehicle_delete(p: GPtr<u8>) = cdecl 0x6E2D90 => crate::vehicle2::operator_delete;
    /// `CAEVehicleAudioEntity::Terminate()` (0x4FB8C0)
    pub fn audio_terminate(a: GPtr<u8>) = thiscall 0x4FB8C0 => unported;
    /// `CAutomobile::~CAutomobile` body (0x6A61E0)
    pub fn automobile_destructor(v: GPtr<u8>) = thiscall 0x6A61E0 => unported;
    /// `CAutomobile::CAutomobile(int32, eVehicleCreatedBy, bool)` (0x6B0A90)
    pub fn automobile_ctor(v: GPtr<u8>, mi: i32, created_by: u32, setup_susp: u32) -> GPtr<u8> = thiscall 0x6B0A90 => unported;
    /// `CAutomobile::OpenDoor(CPed*, int32, eDoors, float, bool)` (0x6A6AE0)
    pub fn automobile_open_door(v: GPtr<u8>, ped: GPtr<u8>, comp: i32, door: i32, ratio: f32, sound: u32) = thiscall 0x6A6AE0 => unported;
    /// `CVehicle::GetPlaneNumGuns()` (0x6D3F30)
    pub fn get_plane_num_guns(v: GPtr<u8>) -> i32 = thiscall 0x6D3F30 => unported;
    /// `FxSystem_c::Kill()` (0x4AA3F0)
    pub fn fx_kill(fx: GPtr<u8>) = thiscall 0x4AA3F0 => unported;
    /// `FxManager_c::DestroyFxSystem(FxSystem_c*)` (0x4A9810)
    pub fn fx_destroy(mgr: GPtr<u8>, fx: GPtr<u8>) = thiscall 0x4A9810 => unported;
    /// CRT `free` (0x8213AE)
    pub fn free(p: GPtr<u8>) = cdecl 0x8213AE => unported;
    /// `CDamageManager::SetDoorStatus(eDoors, eDoorStatus)` (0x6C21C0)
    pub fn set_door_status(dm: GPtr<u8>, door: i32, status: u32) = thiscall 0x6C21C0 => crate::damage::set_door_status;
    /// `CVehicleModelInfo::SetVehicleColour(uint8, uint8, uint8, uint8)` (0x4C84B0)
    pub fn set_vehicle_colour(mi: GPtr<u8>, a: u32, b: u32, c: u32, d: u32) = thiscall 0x4C84B0 => unported;
    /// `RwFrameForAllObjects` (0x7F1200)
    pub fn rw_frame_for_all_objects(frame: GPtr<u8>, cb: u32, data: GPtr<u8>) -> GPtr<u8> = cdecl 0x7F1200 => unported;
    /// `CVehicle::SetComponentAtomicAlpha(RpAtomic*, int32)` (0x6D2960)
    pub fn set_component_atomic_alpha(a: GPtr<u8>, alpha: u32) = cdecl 0x6D2960 => crate::vehicle::set_component_atomic_alpha;
    /// `CEntity::Render()` (0x534310)
    pub fn entity_render(e: GPtr<u8>) = thiscall 0x534310 => unported;
    /// `CMatrix::CMatrix(RwMatrix*, bool)` (0x59C050)
    pub fn matrix_ctor_rw(m: GPtr<u8>, rw: GPtr<u8>, temp: u32) -> GPtr<u8> = thiscall 0x59C050 => unported;
    /// `CMatrix::SetTranslate(CVector)` (0x59AF40)
    pub fn matrix_set_translate(m: GPtr<u8>, x: u32, y: u32, z: u32) = thiscall 0x59AF40 => unported;
    /// `CMatrix::UpdateRW()` (0x59BBB0)
    pub fn matrix_update_rw(m: GPtr<u8>) = thiscall 0x59BBB0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    pub fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `RwHelper::GetFirstObject(RwFrame*)` (0x7348C0)
    pub fn get_first_object(frame: GPtr<u8>) -> GPtr<u8> = cdecl 0x7348C0 => unported;
    /// `CBouncingPanel::SetPanel(int16, int16, float)` (0x6F4920)
    pub fn bouncing_panel_set_panel(p: GPtr<u8>, frame: u32, axis: u32, limit: u32) = thiscall 0x6F4920 => crate::door::set_panel;
    /// `CVehicleModelInfo::GetWheelPosn(int32, CVector&, bool)` (0x4C7D20)
    pub fn get_wheel_posn(mi: GPtr<u8>, wheel: i32, out: GPtr<u8>, local: u32) = thiscall 0x4C7D20 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CDamageManager::SetWheelStatus(int32, uint32)` (0x6C21A0)
    pub fn set_wheel_status(dm: GPtr<u8>, wheel: i32, status: u32) = thiscall 0x6C21A0 => crate::damage::set_wheel_status;
    /// `CAutomobile::ProcessControl()` (0x6B1880)
    pub fn automobile_process_control(a: GPtr<u8>) = thiscall 0x6B1880 => unported;
    /// `CVehicle::ProcessWeapons()` (0x6E3950)
    pub fn process_weapons(v: GPtr<u8>) = thiscall 0x6E3950 => unported;
    /// `FxSystem_c::GetCompositeMatrix(RwMatrix*)` (0x4AA8C0)
    pub fn fx_get_composite_matrix(fx: GPtr<u8>, out: GPtr<u8>) -> GPtr<u8> = thiscall 0x4AA8C0 => unported;
    /// `FxPrtMult_c::FxPrtMult_c(float r, g, b, a, size, unk, life)` (0x4AB290)
    pub fn fx_prt_mult_ctor(p: GPtr<u8>, r: u32, g: u32, b: u32, a: u32, size: u32, unk: u32, life: u32) -> GPtr<u8> = thiscall 0x4AB290 => unported;
    /// `FxSystem_c::AddParticle(RwV3d*, RwV3d*, float, FxPrtMult_c*, float, float, float, uint8)` (0x4AA440)
    pub fn fx_add_particle(fx: GPtr<u8>, pos: GPtr<u8>, vel: GPtr<u8>, time: u32, mult: GPtr<u8>, rot: u32, bright: u32, cam: u32, local: u32) = thiscall 0x4AA440 => unported;
    /// `CHeli::SearchLightCone(...)` (0x6C58E0), 19 argument words
    pub fn search_light_cone(corona: u32, ox: u32, oy: u32, oz: u32, tx: u32, ty: u32, tz: u32, radius: u32, power: u32, unk: u32, shadow: u32, a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>, d: u32, e: u32, f: u32, h: u32, i: u32) = cdecl 0x6C58E0 => unported;
}

/// `GetCurrentAtomicObjectCB` (0x6A0750), passed to `RwFrameForAllObjects` by address.
const GET_CURRENT_ATOMIC_OBJECT_CB: u32 = 0x6A_0750;
/// `g_fxMan`.
const FX_MAN: usize = 0xA9AE80;

/// `CPlane::Render()` (0x6CAB70): `+0x4D0 = now + 3000`, then (tail jump) `CVehicle::Render`.
#[hook(addr = 0x6CAB70, conv = thiscall, name = "CPlane::Render", expect = "A1 84 CB B7 00", this = "CPlane")]
pub fn plane_render(p: Ptr) {
    wr::<u32>(p, 0x4D0, now().wrapping_add(3000));
    vehicle_render(p);
}

/// Kills (and destroys through `g_fxMan`) the `GetPlaneNumGuns()` gun-flash systems of the array at
/// `off`, frees the array and clears the pointer (shared by `~CHeli` and `~CPlane`).
fn kill_gun_fx(v: Ptr, off: isize) {
    if rd::<u32>(v, off) == 0 {
        return;
    }
    let mut i = 0i32;
    if get_plane_num_guns(v) > 0 {
        loop {
            let fx = rd::<GPtr<u8>>(rd::<GPtr<u8>>(v, off), 4 * i as isize);
            if !fx.is_null() {
                fx_kill(fx);
                let fx = rd::<GPtr<u8>>(rd::<GPtr<u8>>(v, off), 4 * i as isize);
                fx_destroy(GPtr::from_addr(FX_MAN), fx);
            }
            i += 1;
            if i >= get_plane_num_guns(v) {
                break;
            }
        }
    }
    free(rd::<GPtr<u8>>(v, off));
    wr::<u32>(v, off, 0);
}

/// `FxSystem_c::Kill` of the pointer at `off` when not null, then the pointer is cleared.
fn kill_fx_at(v: Ptr, off: isize) {
    let fx = rd::<GPtr<u8>>(v, off);
    if !fx.is_null() {
        fx_kill(fx);
        wr::<u32>(v, off, 0);
    }
}

/// `CHeli::~CHeli()` (0x6C4340): vtable 0x871680; the gun-flash array `+0xA0C` (see `kill_gun_fx`);
/// `CAEVehicleAudioEntity::Terminate(+0x138)`; `CAutomobile::~CAutomobile`.
#[hook(addr = 0x6C4340, conv = thiscall, name = "CHeli::~CHeli", expect = "6A FF 68 38 84 84 00", this = "CHeli")]
pub fn heli_dtor(h: Ptr) {
    wr::<u32>(h, 0, 0x0087_1680);
    kill_gun_fx(h, 0xA0C);
    audio_terminate(h.byte_add(0x138));
    automobile_destructor(h);
}

/// `CPlane::~CPlane()` (0x6C9160): vtable 0x871948; the gun-flash array `+0x9D4`; the four fx systems
/// `+0x9E8..+0x9F4` and `+0x9F8` killed and cleared; audio Terminate; `CAutomobile::~CAutomobile`.
#[hook(addr = 0x6C9160, conv = thiscall, name = "CPlane::~CPlane", expect = "6A FF 68 68 85 84 00", this = "CPlane")]
pub fn plane_dtor(p: Ptr) {
    wr::<u32>(p, 0, 0x0087_1948);
    kill_gun_fx(p, 0x9D4);
    for k in 0..4 {
        kill_fx_at(p, 0x9E8 + 4 * k);
    }
    kill_fx_at(p, 0x9F8);
    audio_terminate(p.byte_add(0x138));
    automobile_destructor(p);
}

orig! {
    /// `CHeli::~CHeli()` (0x6C4340)
    pub fn heli_dtor_o(h: GPtr<u8>) = thiscall 0x6C4340 => crate::air2::heli_dtor;
    /// `CPlane::~CPlane()` (0x6C9160)
    pub fn plane_dtor_o(p: GPtr<u8>) = thiscall 0x6C9160 => crate::air2::plane_dtor;
}

/// `CHeli::scalar deleting destructor(uint8)` (0x6C4810).
#[hook(addr = 0x6C4810, conv = thiscall, name = "CHeli::`scalar deleting destructor'", expect = "56 8B F1 E8 28 FB FF FF", this = "CHeli")]
pub fn heli_scalar_deleting_dtor(h: Ptr, flags: u32) -> Ptr {
    heli_dtor_o(h);
    if flags & 1 != 0 {
        vehicle_delete(h);
    }
    h
}

/// `CPlane::scalar deleting destructor(uint8)` (0x6CADB0).
#[hook(addr = 0x6CADB0, conv = thiscall, name = "CPlane::`scalar deleting destructor'", expect = "56 8B F1 E8 A8 E3 FF FF", this = "CPlane")]
pub fn plane_scalar_deleting_dtor(p: Ptr, flags: u32) -> Ptr {
    plane_dtor_o(p);
    if flags & 1 != 0 {
        vehicle_delete(p);
    }
    p
}

/// `CHeli::RenderAllHeliSearchLights()` (0x6C7C50): `SearchLightCone` for every registered light (8-bit
/// counter compared unsigned with `NumberOfSearchLights`): corona, origin and target by value, radius,
/// power, the two flag bytes zero-extended, pointers to the entry's `+0x28/+0x34/+0x40`, then 0, 0.05, 0,
/// 0, 1.0.
#[hook(addr = 0x6C7C50, conv = cdecl, name = "CHeli::RenderAllHeliSearchLights", expect = "A1 6C C9 C1 00")]
pub fn render_all_heli_search_lights() {
    let n = crate::heli::NUMBER_OF_SEARCH_LIGHTS.get();
    if n == 0 {
        return;
    }
    let base = crate::heli::HELI_SEARCH_LIGHTS.ptr().cast::<u8>();
    let mut i: u8 = 0;
    loop {
        let e = base.byte_add(isize::from(i) * 0x4C);
        search_light_cone(
            rd::<u32>(e, 0x20),
            rd::<u32>(e, 0),
            rd::<u32>(e, 4),
            rd::<u32>(e, 8),
            rd::<u32>(e, 0xC),
            rd::<u32>(e, 0x10),
            rd::<u32>(e, 0x14),
            rd::<u32>(e, 0x18),
            rd::<u32>(e, 0x1C),
            u32::from(rd::<u8>(e, 0x24)),
            u32::from(rd::<u8>(e, 0x25)),
            e.byte_add(0x28),
            e.byte_add(0x34),
            e.byte_add(0x40),
            0,
            0x3D4C_CCCD,
            0,
            0,
            0x3F80_0000,
        );
        i = i.wrapping_add(1);
        if u32::from(i) >= crate::heli::NUMBER_OF_SEARCH_LIGHTS.get() {
            return;
        }
    }
}

/// `CPlane::OpenDoor(CPed*, int32 component, eDoors door, float ratio, bool playSound)` (0x6CACB0):
/// `CAutomobile::OpenDoor`; then for model 0x201 with the byte 0xC1CAFC set, the component frame's
/// matrix (`+0x648 + 4 * component`, RwFrame `+0x10`) is wrapped in a temporary CMatrix whose position y
/// becomes `(door angle - previous angle) + y` (door `+0x5B8 + 0x18 * door`), and written back
/// (`UpdateRW`).
#[hook(addr = 0x6CACB0, conv = thiscall, name = "CPlane::OpenDoor", expect = "64 A1 00 00 00 00", this = "CPlane")]
pub fn plane_open_door(p: Ptr, ped: GPtr<u8>, comp: i32, door: i32, ratio: f32, sound: u32) {
    automobile_open_door(p, ped, comp, door, ratio, sound);
    if rd::<u16>(p, 0x22) != 0x201 || g::<u8>(0xC1CAFC).read() == 0 {
        return;
    }
    let frame = rd::<GPtr<u8>>(p, 0x648 + 4 * comp as isize);
    let mut m = [0u32; 0x12];
    let mp = GPtr::from_raw(&raw mut m).cast::<u8>();
    matrix_ctor_rw(mp, frame.byte_add(0x10), 0);
    let d = 0x5B8 + 0x18 * door as isize;
    let y = x(rd::<f32>(p, d + 0xC))
        .fsub_m(rd::<f32>(p, d + 0x10))
        .fadd_m(rd::<f32>(mp, 0x34))
        .to_f32();
    matrix_set_translate(mp, rd::<u32>(mp, 0x30), y.to_bits(), rd::<u32>(mp, 0x38));
    matrix_update_rw(mp);
    matrix_dtor(mp);
}

/// `CHeli::Render()` (0x6C4400): `+0x4D0 = now + 3000`; the model's `SetVehicleColour` with the four
/// colour bytes `+0x434..+0x437`; the first atomic of the frames `+0x678` and `+0x680` gets alpha 255,
/// of `+0x67C` and `+0x684` alpha 0 (frames that are null or without atomic are skipped);
/// `CEntity::Render`.
#[hook(addr = 0x6C4400, conv = thiscall, name = "CHeli::Render", expect = "51 8B 15 84 CB B7 00", this = "CHeli")]
pub fn heli_render(h: Ptr) {
    let t = now();
    let mi = crate::common::model_info(i32::from(rd::<i16>(h, 0x22)));
    wr::<u32>(h, 0x4D0, t.wrapping_add(3000));
    set_vehicle_colour(
        mi,
        u32::from(rd::<u8>(h, 0x434)),
        u32::from(rd::<u8>(h, 0x435)),
        u32::from(rd::<u8>(h, 0x436)),
        u32::from(rd::<u8>(h, 0x437)),
    );
    for (off, alpha) in [(0x678, 0xFF), (0x680, 0xFF), (0x67C, 0), (0x684, 0)] {
        let frame = rd::<GPtr<u8>>(h, off);
        let mut atomic = GPtr::<u8>::NULL;
        if frame.is_null() {
            continue;
        }
        rw_frame_for_all_objects(
            frame,
            GET_CURRENT_ATOMIC_OBJECT_CB,
            GPtr::from_raw(&raw mut atomic).cast(),
        );
        if !atomic.is_null() {
            set_component_atomic_alpha(atomic, alpha);
        }
    }
    entity_render(h);
}

/// `CHeli::CHeli(int32 modelIndex, eVehicleCreatedBy)` (0x6C4190, the SEH prologue through the .HOODLUM
/// fragment 0x403C98): `CAutomobile(mi, createdBy, true)`, then the heli fields (vtable 0x871680,
/// subtype 3, rotor speeds, search light state, ...). For the Hunter (0x1A9) door 2 is set to status 0
/// and the door `+0x5E0` gets open angle 0.9425 (0x3F71463B), closed 0, axis 1 / dirn 0x13. The rotor
/// phase `+0xA14 = rand() * [0x858C7C] * [0x858B44] + [0x858CA0]`. Returns `this`.
#[hook(addr = 0x6C4190, conv = thiscall, name = "CHeli", expect = "6A FF E9 01 FB D3 FF", this = "CHeli")]
pub fn heli_ctor(h: Ptr, mi: i32, created_by: u32) -> Ptr {
    automobile_ctor(h, mi, created_by, 1);
    let b988 = rd::<u8>(h, 0x988) & 0xFC;
    let f40 = rd::<u32>(h, 0x40) | 0x0100_0000;
    wr::<u32>(h, 0, 0x0087_1680);
    wr::<u32>(h, 0x594, 3);
    for k in 0..7 {
        wr::<u32>(h, 0x98C + 4 * k, 0);
    }
    wr::<u32>(h, 0x9B0, 0x4120_0000);
    wr::<u32>(h, 0x9A8, 0x4120_0000);
    wr::<u32>(h, 0x9AC, 0x4120_0000);
    wr::<u32>(h, 0x9B4, 0);
    wr::<u8>(h, 0x988, b988);
    wr::<u32>(h, 0xA00, 0);
    wr::<u32>(h, 0x40, f40);
    if mi == 0x1A9 {
        set_door_status(h.byte_add(0x5A0), 2, 0);
        wr::<u32>(h, 0x5E8, 0x3F71_463B);
        wr::<u32>(h, 0x5EC, 0);
        wr::<u8>(h, 0x5F2, 1);
        wr::<u16>(h, 0x5F0, 0x13);
    }
    wr::<u8>(h, 0x9B9, 4);
    wr::<u32>(h, 0x9BA, 0);
    wr::<u32>(h, 0x9F0, now());
    for k in 0..6 {
        wr::<u32>(h, 0x9C0 + 4 * k, 0);
        wr::<u32>(h, 0x9D8 + 4 * k, 0);
    }
    let b42c = rd::<u8>(h, 0x42C) | 0x40;
    wr::<u32>(h, 0xA04, 0);
    wr::<u32>(h, 0xA08, now());
    wr::<u8>(h, 0x42C, b42c);
    wr::<u8>(h, 0x3DF, 0xA);
    wr::<u32>(h, 0xA0C, 0);
    wr::<u8>(h, 0xA10, 0x10);
    wr::<u8>(h, 0x9B8, 0);
    wr::<u8>(h, 0xA11, 0);
    let r = crate::common::rand_at(0x6C42E8);
    let v = xi(r)
        .fmul_m(g::<f32>(0x858C7C).read())
        .fmul_m(g::<f32>(0x858B44).read())
        .fadd_m(g::<f32>(0x858CA0).read());
    wr::<f32>(h, 0xA14, v.to_f32());
    h
}

/// `CPlane::CPlane(int32 modelIndex, eVehicleCreatedBy)` (0x6C8E20, catalogue `CPlane::Constructor`):
/// `CAutomobile(mi, createdBy, true)`, vtable 0x871948, subtype 4, the plane fields; flag 0x1000000 of
/// `+0x40` except for model 0x21B; per model the door 2 setup (0x208 / 0x1DC / 0x200 / 0x201 / 0x207 /
/// 0x229; 0x207 and 0x201 also clear the flags byte of the first object of frames `+0x65C` resp.
/// `+0x664`, `+0x658`) or, for 0x21B with an unset panel, `SetPanel(0x15, 1, -0.25)` on `+0x6AC`; the
/// four suspension values `+0x838 = (v - wheel.z) + localWheel.z` (`GetWheelPosn` world / local); model
/// 0x208 copies the word 0x8D33C4 to `+0x86C`. Returns `this`.
#[hook(addr = 0x6C8E20, conv = thiscall, name = "CPlane::Constructor", expect = "6A FF 68 48 85 84 00", this = "CPlane")]
pub fn plane_ctor(p: Ptr, mi: i32, created_by: u32) -> Ptr {
    automobile_ctor(p, mi, created_by, 1);
    let mip = crate::common::model_info(mi);
    wr::<u32>(p, 0, 0x0087_1948);
    wr::<u32>(p, 0x594, 4);
    for off in [0x988, 0x98C, 0x990, 0x994] {
        wr::<u32>(p, off, 0);
    }
    wr::<u32>(p, 0x998, 0x3F80_0000);
    for off in [0x9C4, 0x9C8, 0x9CC, 0x9A0, 0x9A4, 0x9B4, 0x9B8] {
        wr::<u32>(p, off, 0);
    }
    wr::<u32>(p, 0x9A8, 0x4170_0000);
    wr::<u32>(p, 0x9AC, 0x41C8_0000);
    wr::<u32>(p, 0x9B0, 0x41A0_0000);
    for off in [0x9BC, 0x9C0, 0x99C] {
        wr::<u32>(p, off, 0);
    }
    if rd::<u16>(p, 0x22) != 0x21B {
        wr::<u32>(p, 0x40, rd::<u32>(p, 0x40) | 0x0100_0000);
    }
    let b42c = rd::<u8>(p, 0x42C) | 0x40;
    let b429 = rd::<u8>(p, 0x429) | 4;
    wr::<u16>(p, 0x4A6, 0xFF);
    wr::<u8>(p, 0x42C, b42c);
    wr::<u8>(p, 0x429, b429);
    let door = |angle: u32, axis: u8, dirn: u16| {
        set_door_status(p.byte_add(0x5A0), 2, 0);
        wr::<u32>(p, 0x5E8, angle);
        wr::<u8>(p, 0x5F2, axis);
        wr::<u16>(p, 0x5F0, dirn);
        wr::<u32>(p, 0x5EC, 0);
    };
    match mi {
        0x208 | 0x1DC | 0x200 => door(0x3FF1_463B, 1, 0x13),
        0x207 => {
            door(0xC016_CBE4, 1, 0x12);
            wr::<u8>(get_first_object(rd::<GPtr<u8>>(p, 0x65C)), 2, 0);
        }
        0x229 => door(0xBFA0_D97C, 2, 0x14),
        0x21B => {
            if rd::<u16>(p, 0x6AC) == 0xFFFF {
                bouncing_panel_set_panel(p.byte_add(0x6AC), 0x15, 1, 0xBE80_0000);
            }
        }
        0x201 => {
            door(0x3FF1_463B, 1, 0x13);
            wr::<u8>(get_first_object(rd::<GPtr<u8>>(p, 0x664)), 2, 0);
            wr::<u8>(get_first_object(rd::<GPtr<u8>>(p, 0x658)), 2, 0);
        }
        _ => {}
    }
    for i in 0..4i32 {
        let mut a = [0f32; 3];
        let mut b = [0f32; 3];
        get_wheel_posn(mip, i, GPtr::from_raw(&raw mut a).cast(), 0);
        get_wheel_posn(mip, i, GPtr::from_raw(&raw mut b).cast(), 1);
        let off = 0x838 + 4 * i as isize;
        wr::<f32>(p, off, x(rd::<f32>(p, off)).fsub_m(a[2]).fadd_m(b[2]).to_f32());
    }
    for off in [0x9D0, 0x9D4] {
        wr::<u32>(p, off, 0);
    }
    wr::<u8>(p, 0x9D8, 0x10);
    for off in [0x9DC, 0x9E0, 0x9E8, 0x9EC, 0x9F0, 0x9F4, 0x9F8] {
        wr::<u32>(p, off, 0);
    }
    if rd::<u16>(p, 0x22) == 0x208 {
        wr::<u16>(p, 0x86C, g::<u16>(0x8D33C4).read());
    }
    wr::<u8>(p, 0xA00, 0);
    p
}

/// `CPlane::ProcessControl()` (0x6C9260): for a player-controlled (status 0) model 0x200/0x201 the
/// smoke toggle `+0xA00` flips when the pad (`GetPad(driver ped type +0x598 == 1)`) has `+0x26` set and
/// `+0x56` clear; the toggle is cleared unless the engine is on (`+0x428 & 0x10`), flag `+0x42B & 0x40`
/// is clear and there is a driver. Model 0x1CC bursts its four wheels (status 2). Then
/// `CAutomobile::ProcessControl`; `+0x1B4` (u16) = `+0x9A0`, which is cleared; `ProcessWeapons`; model
/// 0x21B clears `+0x968..+0x974`. With a smoke fx `+0x9F8`: `+0x9FC += _ftol(timeStep * [0x858B38] *
/// [0x859948])`, two particles of `[0xA9AE20]` at the fx's composite matrix position with velocity
/// `-moveSpeed * [0x858C80]` (z through an f32 store), times 0 and 0.05; the fx is killed and cleared
/// when `+0x9FC <= 0` or `+0x42B & 0x40`.
#[hook(addr = 0x6C9260, conv = thiscall, name = "CPlane::ProcessControl", expect = "83 EC 74 56 8B F1", this = "CPlane")]
pub fn plane_process_control(p: Ptr) {
    if rd::<u8>(p, 0x36) & 0xF8 == 0 && matches!(rd::<u16>(p, 0x22), 0x200 | 0x201) {
        let drv = rd::<GPtr<u8>>(p, 0x460);
        let n = i32::from(!drv.is_null() && rd::<u32>(drv, 0x598) == 1);
        let pad = get_pad(n);
        if rd::<u16>(pad, 0x26) != 0 && rd::<u16>(pad, 0x56) == 0 {
            wr::<u8>(p, 0xA00, u8::from(rd::<u8>(p, 0xA00) == 0));
        }
    }
    if rd::<u8>(p, 0xA00) != 0
        && !(rd::<u8>(p, 0x428) & 0x10 != 0 && rd::<u8>(p, 0x42B) & 0x40 == 0 && rd::<u32>(p, 0x460) != 0)
    {
        wr::<u8>(p, 0xA00, 0);
    }
    if rd::<u16>(p, 0x22) == 0x1CC {
        for w in 0..4 {
            set_wheel_status(p.byte_add(0x5A0), w, 2);
        }
    }
    automobile_process_control(p);
    let v = rd::<u32>(p, 0x9A0);
    wr::<u16>(p, 0x1B4, v as u16);
    if v != 0 {
        wr::<u32>(p, 0x9A0, 0);
    }
    process_weapons(p);
    if rd::<u16>(p, 0x22) == 0x21B {
        for off in [0x968, 0x96C, 0x970, 0x974] {
            wr::<u32>(p, off, 0);
        }
    }
    let fx = rd::<GPtr<u8>>(p, 0x9F8);
    if fx.is_null() {
        return;
    }
    let add = crate::common::ftol32(
        x(g::<f32>(0xB7CB5C).read())
            .fmul_m(g::<f32>(0x858B38).read())
            .fmul_m(g::<f32>(0x859948).read()),
    );
    wr::<i32>(p, 0x9FC, rd::<i32>(p, 0x9FC).wrapping_add(add));
    let mut m = [0u32; 0x10];
    let mp = GPtr::from_raw(&raw mut m).cast::<u8>();
    fx_get_composite_matrix(fx, mp);
    let k = g::<f32>(0x858C80).read();
    let nz = x(rd::<f32>(p, 0x4C)).fchs().to_f32();
    let mut vel = [
        x(rd::<f32>(p, 0x44)).fchs().fmul_m(k).to_f32(),
        x(rd::<f32>(p, 0x48)).fchs().fmul_m(k).to_f32(),
        x(nz).fmul_m(k).to_f32(),
    ];
    let vp = GPtr::from_raw(&raw mut vel).cast::<u8>();
    let mut mult = [0u32; 8];
    let mlt = GPtr::from_raw(&raw mut mult).cast::<u8>();
    fx_prt_mult_ctor(mlt, 0, 0, 0, 0x3E4C_CCCD, 0x3F80_0000, 0x3F80_0000, 0x3DCC_CCCD);
    for time in [0, 0x3D4C_CCCD] {
        fx_add_particle(
            g::<GPtr<u8>>(0xA9AE20).read(),
            mp.byte_add(0x30),
            vp,
            time,
            mlt,
            0xBF80_0000,
            0x3F99_999A,
            0x3F19_999A,
            0,
        );
    }
    if rd::<i32>(p, 0x9FC) <= 0 || rd::<u8>(p, 0x42B) & 0x40 != 0 {
        fx_kill(rd::<GPtr<u8>>(p, 0x9F8));
        wr::<u32>(p, 0x9F8, 0);
    }
}

/// `CHeli::UpdateHelis()` (0x6C79A0): search lights reset; `n = NumOfHelisRequired()` of the player's
/// wanted, 0 under a no-rain zone or interior (`currArea != 0`), when police helis are off or when one
/// of the words 0xC81320 / 0xC8131C is 0x13. A police Maverick (0x1F1) slot that is not wrecked
/// (`+0x40` bit 29, `+0x42B & 0x40`) makes the new heli a police one, a news chopper (0x1E8) undoes
/// that; the flag is also cleared unless the byte 0xB7CB8C is set (then the 0x8E73A4 test is skipped
/// for a police heli when 0x8E72F0 == 1). When generation is allowed (0x8E73A4 == 1), `now >` the
/// timer (then timer = now + 15000) and fewer helis exist than required, `GenerateHeli(player ped,
/// flag)` fills the first free slot (`RegisterReference`). Then each slot: a wrecked heli gets mission
/// 0x2A and the slot is cleared (not removed); one already on mission 0x2A farther than [0x858F98]
/// from the player is removed from the world and deleted. Finally helis above the required count are
/// sent away (mission 0x2A, `+0x9B0 = +0x9A8 = 100.0`).
#[hook(addr = 0x6C79A0, conv = cdecl, name = "CHeli::UpdateHelis", expect = "83 EC 10 53 55 56")]
pub fn update_helis() {
    use crate::heli::{HELIS, NUMBER_OF_SEARCH_LIGHTS, POLICE_HELIS_ALLOWED};
    let slots = HELIS.ptr().cast::<GPtr<u8>>();
    NUMBER_OF_SEARCH_LIGHTS.set(0);
    let mut required = num_of_helis_required(find_player_wanted(-1));
    let mut count = 0i32;
    let mut police = false;
    let usable = |h: GPtr<u8>| rd::<u32>(h, 0x40) & 0x2000_0000 == 0 && rd::<u8>(h, 0x42B) & 0x40 == 0;
    for k in 0..2 {
        let h = slots.wrapping_index(k).read();
        if !h.is_null() {
            count += 1;
            if rd::<u16>(h, 0x22) == 0x1F1 && usable(h) {
                police = true;
            }
        }
    }
    if player_no_rain() != 0 || g::<i32>(0xB72914).read() != 0 {
        required = 0;
    }
    if POLICE_HELIS_ALLOWED.get() == 0 {
        required = 0;
    }
    if g::<u16>(0xC81320).read() == 0x13 || g::<u16>(0xC8131C).read() == 0x13 {
        required = 0;
    }
    for k in 0..2 {
        let h = slots.wrapping_index(k).read();
        if !h.is_null() && rd::<u16>(h, 0x22) == 0x1E8 {
            police = false;
        }
    }
    // 0: test 0x8E73A4 first, 1: skip that test, 2: no generation
    let path = if g::<u8>(0xB7CB8C).read() == 0 {
        police = false;
        0
    } else if !police {
        0
    } else if g::<u8>(0x8E72F0).read() == 1 {
        1
    } else {
        2
    };
    if path == 0 && g::<u8>(0x8E73A4).read() != 1 {
        // no generation
    } else if path != 2 {
        let now = now();
        if now > TEST_FOR_NEW_RANDOM_HELIS_TIMER.get() {
            TEST_FOR_NEW_RANDOM_HELIS_TIMER.set(now.wrapping_add(15000));
            if count < required {
                let h = generate_heli(find_player_ped(-1), u32::from(police));
                for k in 0..2 {
                    if slots.wrapping_index(k).read().is_null() {
                        slots.wrapping_index(k).write(h);
                        entity_register_reference(h, slots.wrapping_index(k).cast());
                        break;
                    }
                }
            }
        }
    }
    for k in 0..2 {
        let slot = slots.wrapping_index(k);
        let h = slot.read();
        if h.is_null() {
            continue;
        }
        if !usable(h) {
            wr::<u8>(h, 0x3BA, 0x2A);
            slot.write(GPtr::NULL);
            continue;
        }
        if rd::<u8>(h, 0x3BA) != 0x2A {
            continue;
        }
        let m = rd::<GPtr<u8>>(h, 0x14);
        let hp = if m.is_null() {
            h.byte_add(4)
        } else {
            m.byte_add(0x30)
        };
        let mut pc = [0f32; 3];
        let pp = find_player_coors(GPtr::from_raw(&raw mut pc).cast(), -1);
        let dx = x(rd::<f32>(pp, 0)).fsub_m(rd::<f32>(hp, 0));
        let dy = x(rd::<f32>(pp, 4)).fsub_m(rd::<f32>(hp, 4));
        let dz = x(rd::<f32>(pp, 8)).fsub_m(rd::<f32>(hp, 8));
        let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
        let d = sa_math::x87::X87::from_f64_exact(f64::from_bits(d.to_f64().to_bits() & !(1 << 63)));
        if crate::common::jne_mask(d, x(g::<f32>(0x858F98).read()), 0x41) {
            continue;
        }
        world_remove(slot.read());
        let h = slot.read();
        if !h.is_null() {
            entity_vdelete(h, 1);
        }
        slot.write(GPtr::NULL);
    }
    let mut left = required;
    let h0 = slots.wrapping_index(0).read();
    if !h0.is_null() && rd::<u8>(h0, 0x3BA) != 0x2A {
        if required > 0 {
            left = required - 1;
        } else {
            wr::<u8>(h0, 0x3BA, 0x2A);
            wr::<u32>(h0, 0x9B0, 0x42C8_0000);
            wr::<u32>(h0, 0x9A8, 0x42C8_0000);
        }
    }
    let h1 = slots.wrapping_index(1).read();
    if !h1.is_null() && rd::<u8>(h1, 0x3BA) != 0x2A && left <= 0 {
        wr::<u8>(h1, 0x3BA, 0x2A);
        wr::<u32>(h1, 0x9B0, 0x42C8_0000);
        wr::<u32>(h1, 0x9A8, 0x42C8_0000);
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_UPDATE_HELIS,
    HOOK_PLANE_PROCESS_CONTROL,
    HOOK_PLANE_CTOR,
    HOOK_PLANE_RENDER,
    HOOK_HELI_DTOR,
    HOOK_PLANE_DTOR,
    HOOK_HELI_SCALAR_DELETING_DTOR,
    HOOK_PLANE_SCALAR_DELETING_DTOR,
    HOOK_RENDER_ALL_HELI_SEARCH_LIGHTS,
    HOOK_PLANE_OPEN_DOOR,
    HOOK_HELI_RENDER,
    HOOK_HELI_CTOR,
];
