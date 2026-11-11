//! `CVehicle`, second pass (PORT-VEH-2, docs/arch/veh-port.md): passenger/driver bookkeeping, render
//! helpers (the RenderWare calls stay original through `orig!`), lighting, small state checks, the
//! vehicle pool's operator new/delete. Ported from the 1.0 US bytes; gta-reversed `Vehicle.cpp` and
//! docs/re/specs/VEH as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use sa_math::x87::X87;

use crate::common::{b, ftol32, g, rand_at, rand_frac, rd, wr, x, xi};

/// `CVehicle*` (thiscall `this`).
type Veh = GPtr<u8>;

orig! {
    /// `CPointLights::RemoveLightsAffectingObject()` (0x6FFFE0)
    pub fn point_lights_remove_lights_affecting_object() = cdecl 0x6FFFE0 => unported;
    /// `Light::SetAmbientColours()` (0x735D30)
    pub fn light_set_ambient_colours() = cdecl 0x735D30 => unported;
    /// `Light::DeActivateDirectional()` (0x735C70)
    pub fn light_deactivate_directional() = cdecl 0x735C70 => unported;
    /// `Light::ActivateDirectional()` (0x735C80)
    pub fn light_activate_directional() = cdecl 0x735C80 => unported;
    /// `CRenderer::SetupLightingForEntity(CPhysical*)` (0x553E40); the whole EAX is passed on
    pub fn renderer_setup_lighting_for_entity(e: GPtr<u8>) -> u32 = cdecl 0x553E40 => unported;
    /// `CColModel::CColModel()` (0x40FB60)
    pub fn col_model_ctor(cm: GPtr<u8>) -> GPtr<u8> = thiscall 0x40FB60 => unported;
    /// `CCollisionData::CCollisionData()` (0x40F030)
    pub fn collision_data_ctor(cd: GPtr<u8>) -> GPtr<u8> = thiscall 0x40F030 => unported;
    /// CRT `_atexit` (0x821D1E)
    pub fn atexit(f: u32) -> i32 = cdecl 0x821D1E => unported;
    /// CRT `eh vector constructor iterator(void*, size, count, ctor, dtor)` (0x821D30, stdcall)
    pub fn eh_vector_constructor_iterator(p: u32, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// `CVehicleModelInfo::SetDirtTextures(CVehicleModelInfo*, int32)` (0x5D5DB0)
    pub fn set_dirt_textures(mi: GPtr<u8>, level: i32) = cdecl 0x5D5DB0 => unported;
    /// `CEntity::Render()` (0x534310)
    pub fn entity_render(e: GPtr<u8>) = thiscall 0x534310 => unported;
    /// `RwRenderStateSet` (RwEngineInstance table slot 0x20)
    pub fn rw_render_state_set(state: u32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `CVehicleModelInfo::ResetEditableMaterials(RpClump*)` (0x4C8460)
    pub fn reset_editable_materials(clump: GPtr<u8>) = cdecl 0x4C8460 => unported;
    /// `CAutomobile::CustomCarPlate_AfterRenderingStop(CVehicleModelInfo*)` (0x6A2F30)
    pub fn custom_car_plate_after_rendering_stop(a: GPtr<u8>, mi: GPtr<u8>) = thiscall 0x6A2F30 => unported;
    /// `CVehiclePool::New()` (0x6E2A50)
    pub fn vehicle_pool_new(pool: GPtr<u8>) -> GPtr<u8> = thiscall 0x6E2A50 => unported;
    /// `CVehicle::IsLawEnforcementVehicle()` (0x6D2370)
    pub fn is_law_enforcement_vehicle(v: GPtr<u8>) -> u8 = thiscall 0x6D2370 => crate::vehicle::is_law_enforcement_vehicle;
}

/// `CPools::ms_pVehiclePool`.
const VEHICLE_POOL: usize = 0xB74494;
/// `CModelInfo::ms_modelInfoPtrs`.
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// `CCarCtrl::NumLawEnforcerCars`.
const NUM_LAW_ENFORCER_CARS: usize = 0x969098;

fn mi_of(v: Veh) -> GPtr<u8> {
    g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * (i32::from(rd::<i16>(v, 0x22)) as u32 as usize)).read()
}

/// `FUN_004f4ca0` (0x4F4CA0, a `CVehicle` flag getter): AL = bit 5 of byte `+0x428` (EAX's upper bytes
/// are the caller's).
#[hook(addr = 0x4F4CA0, conv = thiscall, name = "FUN_004f4ca0", expect = "8A 81 28 04 00 00", this = "CVehicle")]
pub fn flag_428_bit5(v: Veh) -> u8 {
    (rd::<u8>(v, 0x428) >> 5) & 1
}

/// `FUN_004f4ce0` (0x4F4CE0): EAX = 1 when the word at `+0x45C` is neither 0 nor 0xFFFF and the
/// entity type/status byte `+0x36 & 0xF8` is not 0x28, else 0.
#[hook(addr = 0x4F4CE0, conv = thiscall, name = "FUN_004f4ce0", expect = "66 8B 81 5C 04 00 00", this = "CVehicle")]
pub fn has_valid_45c(v: Veh) -> u32 {
    let w = rd::<u16>(v, 0x45C);
    if w == 0 || w == 0xFFFF || rd::<u8>(v, 0x36) & 0xF8 == 0x28 {
        0
    } else {
        1
    }
}

/// `CVehicle::RemoveLighting(bool)` (0x5533D0): unless entity flag `0x20000000` (+0x40) is set,
/// `CPointLights::RemoveLightsAffectingObject()`; then `Light::SetAmbientColours()` and
/// `Light::DeActivateDirectional()`. The argument is unused.
#[hook(addr = 0x5533D0, conv = thiscall, name = "CVehicle::RemoveLighting", expect = "F7 41 40 00 00 00 20", this = "CVehicle")]
pub fn remove_lighting(v: Veh, _reset: u32) {
    if rd::<u32>(v, 0x40) & 0x2000_0000 == 0 {
        point_lights_remove_lights_affecting_object();
    }
    light_set_ambient_colours();
    light_deactivate_directional();
}

/// `CVehicle::SetupLighting()` (0x553F20): `Light::ActivateDirectional()`, then the result of
/// `CRenderer::SetupLightingForEntity(this)` (whole EAX).
#[hook(addr = 0x553F20, conv = thiscall, name = "CVehicle::SetupLighting", expect = "56 8B F1 E8 58 1D 1E 00", this = "CVehicle")]
pub fn setup_lighting(v: Veh) -> u32 {
    light_activate_directional();
    renderer_setup_lighting_for_entity(v)
}

/// `GetCurrentAtomicObjectCB(RpAtomic*, void* data)` (0x6A0750): stores the atomic at `*data` when its
/// object flags (`+2`) have bit 2 (rpATOMICRENDER); returns the atomic.
#[hook(addr = 0x6A0750, conv = cdecl, name = "GetCurrentAtomicObjectCB", expect = "8B 44 24 04 F6 40 02 04")]
pub fn get_current_atomic_object_cb(atomic: GPtr<u8>, data: GPtr<GPtr<u8>>) -> GPtr<u8> {
    if rd::<u8>(atomic, 2) & 4 != 0 {
        data.write(atomic);
    }
    atomic
}

/// `__init_s_TestBladeCol` (0x853DF0): `CColModel::CColModel(&s_TestBladeCol)` (0xC1CD38) and
/// `atexit(__dtor_s_TestBladeCol)`.
#[hook(addr = 0x853DF0, conv = cdecl, name = "__init_s_TestBladeCol", expect = "B9 38 CD C1 00")]
pub fn init_s_test_blade_col() {
    col_model_ctor(GPtr::from_addr(0xC1CD38));
    atexit(0x856A50);
}

/// `__init_s_TestBladeColData` (0x853E10): `CCollisionData::CCollisionData(&s_TestBladeColData)`
/// (0xC1CD68, tail call).
#[hook(addr = 0x853E10, conv = cdecl, name = "__init_s_TestBladeColData", expect = "B9 68 CD C1 00")]
pub fn init_s_test_blade_col_data() -> GPtr<u8> {
    collision_data_ctor(GPtr::from_addr(0xC1CD68))
}

/// `__init_CVehicle__m_aSpecialColModel` (0x853DC0): the CRT vector constructor over the four
/// `CColModel`s at 0xC1CC78 (0x30 bytes each), then `atexit` of their destructor.
#[hook(addr = 0x853DC0, conv = cdecl, name = "__init_CVehicle__m_aSpecialColModel", expect = "68 00 F7 40 00")]
pub fn init_special_col_model() {
    eh_vector_constructor_iterator(0xC1CC78, 0x30, 4, 0x40FB60, 0x40F700);
    atexit(0x856A30);
}

/// `CVehicle::UpdatePassengerList()` (0x6D29E0): when the passenger count (+0x484) is non-zero and
/// every one of the 8 passenger slots (+0x464) is empty, the count becomes 0.
#[hook(addr = 0x6D29E0, conv = thiscall, name = "CVehicle::UpdatePassengerList", expect = "8A 81 84 04 00 00", this = "CVehicle")]
pub fn update_passenger_list(v: Veh) {
    if rd::<u8>(v, 0x484) != 0 && (0..8).any(|k| rd::<u32>(v, 0x464 + 4 * k) != 0) {
        return;
    }
    wr::<u8>(v, 0x484, 0);
}

/// `CVehicle::Render()` (0x6D0E60): `SetDirtTextures(modelInfo, ftol(dirtLevel +0x4B0) & 0xF)`, then
/// `CEntity::Render()` (tail call).
#[hook(addr = 0x6D0E60, conv = thiscall, name = "CVehicle::Render", expect = "56 8B F1 D9 86 B0 04 00 00", this = "CVehicle")]
pub fn render(v: Veh) {
    let level = ftol32(x(rd::<f32>(v, 0x4B0))) & 0xF;
    set_dirt_textures(mi_of(v), level);
    entity_render(v);
}

/// `CVehicle::ResetAfterRender()` (0x6D0E20): `RwRenderStateSet(rwRENDERSTATECULLMODE, 2)`,
/// `ResetEditableMaterials(m_pRwClump)`, and for an automobile (subclass +0x590 == 0)
/// `CustomCarPlate_AfterRenderingStop(modelInfo)`.
#[hook(addr = 0x6D0E20, conv = thiscall, name = "CVehicle::ResetAfterRender", expect = "A1 24 7B C9 00", this = "CVehicle")]
pub fn reset_after_render(v: Veh) {
    rw_render_state_set(0x14, 2);
    reset_editable_materials(rd::<GPtr<u8>>(v, 0x18));
    if rd::<u32>(v, 0x590) == 0 {
        custom_car_plate_after_rendering_stop(v, mi_of(v));
    }
}

/// `CVehicle::UsesSiren()` (0x6D8470): models 407 (firetruk), 416 (ambulan), 423 (mrwhoop) -> true,
/// 432 (rhino) -> false, otherwise `IsLawEnforcementVehicle() != 0` (`setne al`).
#[hook(addr = 0x6D8470, conv = thiscall, name = "CVehicle::UsesSiren", expect = "0F BF 41 22 05 69 FE FF FF", this = "CVehicle")]
pub fn uses_siren(v: Veh) -> u8 {
    match i32::from(rd::<i16>(v, 0x22)) {
        407 | 416 | 423 => 1,
        432 => 0,
        _ => b(is_law_enforcement_vehicle(v) != 0),
    }
}

/// `CVehicle::ChangeLawEnforcerState(bool)` (0x6D2330): sets/clears bit 0 of `+0x428` and keeps
/// `CCarCtrl::NumLawEnforcerCars` counting the change.
#[hook(addr = 0x6D2330, conv = thiscall, name = "CVehicle::ChangeLawEnforcerState", expect = "8A 44 24 04 84 C0", this = "CVehicle")]
pub fn change_law_enforcer_state(v: Veh, on: u32) {
    let f = rd::<u8>(v, 0x428);
    let n = g::<i32>(NUM_LAW_ENFORCER_CARS);
    if on as u8 != 0 {
        if f & 1 == 0 {
            wr::<u8>(v, 0x428, f | 1);
            n.write(n.read().wrapping_add(1));
        }
    } else if f & 1 != 0 {
        wr::<u8>(v, 0x428, f & 0xFE);
        n.write(n.read().wrapping_sub(1));
    }
}

/// `CVehicle::IsPassenger(const CPed*)` (0x6D1BD0): null -> false; else whether one of the 8 passenger
/// slots holds it.
#[hook(addr = 0x6D1BD0, conv = thiscall, name = "CVehicle::IsPassenger-Ped", expect = "8B 54 24 04 85 D2", this = "CVehicle")]
pub fn is_passenger_ped(v: Veh, ped: GPtr<u8>) -> u8 {
    if ped.is_null() {
        return 0;
    }
    b((0..8).any(|k| rd::<GPtr<u8>>(v, 0x464 + 4 * k) == ped))
}

/// `CVehicle::operator new(uint32)` (0x6E2D50): `CPools::ms_pVehiclePool->New()` (tail call).
#[hook(addr = 0x6E2D50, conv = cdecl, name = "new", expect = "8B 0D 94 44 B7 00")]
pub fn operator_new(_size: u32) -> GPtr<u8> {
    vehicle_pool_new(g::<GPtr<u8>>(VEHICLE_POOL).read())
}

/// `CVehicle::operator delete(void*)` (0x6E2D90): the vehicle pool's Delete inlined: slot =
/// (p - objects) / 0xA18 (signed), byte map |= 0x80, first free = min(first free, slot).
#[hook(addr = 0x6E2D90, conv = cdecl, name = "delete", expect = "8B 4C 24 04 56")]
pub fn operator_delete(p: GPtr<u8>) {
    let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let d = (p.addr() as u32).wrapping_sub(rd::<u32>(pool, 0)) as i32;
    let slot = d / 0xA18;
    let map = rd::<GPtr<u8>>(pool, 4).byte_add(slot as isize);
    map.write(map.read() | 0x80);
    if slot < rd::<i32>(pool, 0xC) {
        wr::<i32>(pool, 0xC, slot);
    }
}

orig! {
    /// `CVisibilityPlugins::GetAtomicId(RpAtomic*)` (0x732370)
    pub fn vis_get_atomic_id(atomic: GPtr<u8>) -> u32 = cdecl 0x732370 => unported;
    /// `CVisibilityPlugins::GetModelInfo(RpAtomic*)` (0x732260)
    pub fn vis_get_model_info(atomic: GPtr<u8>) -> GPtr<u8> = cdecl 0x732260 => unported;
    /// `RpClumpRemoveAtomic(RpClump*, RpAtomic*)` (0x74A4C0)
    pub fn rp_clump_remove_atomic(clump: GPtr<u8>, atomic: GPtr<u8>) -> GPtr<u8> = cdecl 0x74A4C0 => unported;
    /// `RpAtomicDestroy(RpAtomic*)` (0x749DC0)
    pub fn rp_atomic_destroy(atomic: GPtr<u8>) -> i32 = cdecl 0x749DC0 => unported;
    /// `CBaseModelInfo::RemoveRef()` (0x4C4BB0)
    pub fn base_model_info_remove_ref(mi: GPtr<u8>) = thiscall 0x4C4BB0 => unported;
    /// `CPed::PreRenderAfterTest()` (0x5E65A0)
    pub fn ped_pre_render_after_test(ped: GPtr<u8>) = thiscall 0x5E65A0 => unported;
    /// virtual `CEntity::Render()` (vtable +0x48)
    pub fn entity_render_virtual(e: GPtr<u8>) = thiscall vslot 0x48 => unported;
    /// `RwFrameForAllObjects(RwFrame*, RwObjectCallBack, void*)` (0x7F1200)
    pub fn rw_frame_for_all_objects(frame: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x7F1200 => unported;
    /// `RwFrameForAllChildren(RwFrame*, RwFrameCallBack, void*)` (0x7F0DC0)
    pub fn rw_frame_for_all_children(frame: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x7F0DC0 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    pub fn ped_is_player(ped: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
}

/// `CVehicle::IsUpsideDown()` (0x6D1D90): `matrix.up.z <= -0.9` (fcomp + `test ah, 0x41` + `jp`: NaN
/// gives false).
#[hook(addr = 0x6D1D90, conv = thiscall, name = "CVehicle::IsUpsideDown", expect = "83 EC 0C 8B 49 14", this = "CVehicle")]
pub fn is_upside_down(v: Veh) -> u8 {
    let z = rd::<f32>(rd::<GPtr<u8>>(v, 0x14), 0x28);
    b(!x(z).fcom(x(g::<f32>(0x858CAC).read())).test_parity(0x41))
}

/// `CVehicle::IsOnItsSide()` (0x6D1DD0): `right.z >= 0.8` (not below) or `right.z <= -0.8`; NaN gives
/// false.
#[hook(addr = 0x6D1DD0, conv = thiscall, name = "CVehicle::IsOnItsSide", expect = "83 EC 0C 8B 41 14", this = "CVehicle")]
pub fn is_on_its_side(v: Veh) -> u8 {
    let z = x(rd::<f32>(rd::<GPtr<u8>>(v, 0x14), 8));
    if !z.fcom(x(g::<f32>(0x858C98).read())).test_any(0x01) {
        return 1;
    }
    b(!z.fcom(x(g::<f32>(0x858C9C).read())).test_parity(0x41))
}

/// `CVehicle::SetVehicleAtomicVisibilityCB(RwObject*, void* state)` (0x6D2690): when the atomic id has
/// bit 0 or 1, the atomic's flags byte (`+2`) becomes 4 (rpATOMICRENDER) if `id & 3 == state`, else 0.
/// Returns the atomic.
#[hook(addr = 0x6D2690, conv = cdecl, name = "CVehicle::SetVehicleAtomicVisibilityCB-Object", expect = "56 8B 74 24 08")]
pub fn set_vehicle_atomic_visibility_cb_object(atomic: GPtr<u8>, state: u32) -> GPtr<u8> {
    if vis_get_atomic_id(atomic) & 3 != 0 {
        let id = vis_get_atomic_id(atomic) & 3;
        wr::<u8>(atomic, 2, if id == state { 4 } else { 0 });
    }
    atomic
}

/// `CVehicle::ProcessWheelRotation(tWheelState, const CVector& fwd, const CVector& speed, float radius)`
/// (0x6D1230): state 1 (spinning) -> -1.1 (0x871654), state 3 (fixed) -> 0.0, else
/// `-((fwd.z*speed.z + fwd.y*speed.y) + fwd.x*speed.x) / radius` in ST0.
#[hook(addr = 0x6D1230, conv = thiscall, name = "CVehicle::ProcessWheelRotation", expect = "8B 44 24 04 48", this = "CVehicle")]
pub fn process_wheel_rotation(_v: Veh, state: u32, fwd: GPtr<f32>, speed: GPtr<f32>, radius: f32) -> X87 {
    match state {
        1 => x(g::<f32>(0x871654).read()),
        3 => x(g::<f32>(0x858B50).read()),
        _ => {
            let a = |k: usize| fwd.wrapping_index(k as i32).read();
            let s = |k: usize| speed.wrapping_index(k as i32).read();
            let z = x(a(2)).fmul_m(s(2));
            let y = x(a(1)).fmul_m(s(1));
            let sum = z.fadd(y);
            let xx = x(a(0)).fmul_m(s(0));
            sum.fadd(xx).fdiv_m(radius).fchs()
        }
    }
}

/// `CVehicle::RemoveAllUpgradesCB(RpAtomic*, void*)` (0x6D34D0): an atomic with a model info (an
/// upgrade) is removed from its clump, destroyed, and the model info's reference released. Returns the
/// atomic.
#[hook(addr = 0x6D34D0, conv = cdecl, name = "CVehicle::RemoveAllUpgradesCB", expect = "56 8B 74 24 08")]
pub fn remove_all_upgrades_cb(atomic: GPtr<u8>, _data: u32) -> GPtr<u8> {
    let mi = vis_get_model_info(atomic);
    if !mi.is_null() {
        rp_clump_remove_atomic(rd::<GPtr<u8>>(atomic, 0x3C), atomic);
        rp_atomic_destroy(atomic);
        base_model_info_remove_ref(mi);
    }
    atomic
}

/// The driver (+0x460) and the 8 passenger slots (+0x464) whose ped state (+0x530) is 50
/// (PEDSTATE_DRIVING).
fn for_driving_occupants(v: Veh, f: impl Fn(GPtr<u8>)) {
    let d = rd::<GPtr<u8>>(v, 0x460);
    if !d.is_null() && rd::<u32>(d, 0x530) == 0x32 {
        f(d);
    }
    for k in 0..8 {
        let p = rd::<GPtr<u8>>(v, 0x464 + 4 * k);
        if !p.is_null() && rd::<u32>(p, 0x530) == 0x32 {
            f(p);
        }
    }
}

/// `CVehicle::PreRenderDriverAndPassengers()` (0x6D3DB0): `CPed::PreRenderAfterTest()` on every
/// occupant in state 50.
#[hook(addr = 0x6D3DB0, conv = thiscall, name = "CVehicle::PreRenderDriverAndPassengers", expect = "56 8B F1 8B 8E 60 04 00 00", this = "CVehicle")]
pub fn pre_render_driver_and_passengers(v: Veh) {
    for_driving_occupants(v, ped_pre_render_after_test);
}

/// `CVehicle::RenderDriverAndPassengers()` (0x6D3D60): virtual `Render()` on every occupant in state 50.
#[hook(addr = 0x6D3D60, conv = thiscall, name = "CVehicle::RenderDriverAndPassengers", expect = "56 8B F1 8B 8E 60 04 00 00", this = "CVehicle")]
pub fn render_driver_and_passengers(v: Veh) {
    for_driving_occupants(v, entity_render_virtual);
}

/// `CVehicle::SetComponentVisibility(RwFrame*, uint32 state)` (0x6D2700, a .HOODLUM fragment at
/// 0x401EA2 sets the flag): null frame -> nothing; state 2 sets bit 0 of `+0x42A`; then
/// `RwFrameForAllObjects(frame, SetVehicleAtomicVisibilityCB, state)` and
/// `RwFrameForAllChildren(frame, SetVehicleAtomicVisibilityCB-Frame, state)`.
#[hook(addr = 0x6D2700, conv = thiscall, name = "CVehicle::SetComponentVisibility", expect = "56 8B 74 24 08", this = "CVehicle")]
pub fn set_component_visibility(v: Veh, frame: GPtr<u8>, state: u32) {
    if frame.is_null() {
        return;
    }
    if state == 2 {
        wr::<u8>(v, 0x42A, rd::<u8>(v, 0x42A) | 1);
    }
    rw_frame_for_all_objects(frame, 0x6D2690, state);
    rw_frame_for_all_children(frame, 0x6D26D0, state);
}

/// `CVehicle::GetVehicleAppearance()` (0x6D1080): from handling model flags (`+0x384` -> `+0xCC`) &
/// 0x0F000000: 0 -> 1 (car), 0x01000000 -> 2 (bike), 0x02000000 -> 3 (heli), 0x08000000 -> 4 (boat),
/// 0x04000000 -> 5 (plane), anything else 0.
#[hook(addr = 0x6D1080, conv = thiscall, name = "CVehicle::GetVehicleAppearance", expect = "8B 81 84 03 00 00", preserve = "ecx,edx", this = "CVehicle")]
pub fn get_vehicle_appearance(v: Veh) -> u32 {
    match rd::<u32>(rd::<GPtr<u8>>(v, 0x384), 0xCC) & 0x0F00_0000 {
        0 => 1,
        0x0100_0000 => 2,
        0x0200_0000 => 3,
        0x0800_0000 => 4,
        0x0400_0000 => 5,
        _ => 0,
    }
}

/// `CVehicle::CanPedOpenLocks(const CPed*)` (0x6D1E20, the lock status read sits in a .HOODLUM fragment
/// at 0x401A4D): door lock (+0x4F8) 2, 4, 5, 7 -> false; a player ped with lock 3 -> false; else true.
///
/// Not hooked: protection code reads its entry bytes (0x401B4F, 0x405BD6 read 0x6D1E24).
pub fn can_ped_open_locks(v: Veh, ped: GPtr<u8>) -> u8 {
    let lock = rd::<u32>(v, 0x4F8);
    if matches!(lock, 2 | 4 | 5 | 7) {
        return 0;
    }
    if ped_is_player(ped) != 0 && rd::<u32>(v, 0x4F8) == 3 {
        return 0;
    }
    1
}

orig! {
    /// virtual `CVehicle::BlowUpCar(CEntity* culprit, bool inCutscene)` (vtable +0xA4)
    pub fn blow_up_car_virtual(v: GPtr<u8>, culprit: GPtr<u8>, cutscene: u32) = thiscall vslot 0xA4 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    pub fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CTxdStore::RemoveRef(int32)` (0x731A30)
    pub fn txd_store_remove_ref(slot: i32) = cdecl 0x731A30 => unported;
    /// `RpClumpForAllAtomics(RpClump*, RpAtomicCallBack, void*)` (0x749B70)
    pub fn rp_clump_for_all_atomics(clump: GPtr<u8>, cb: u32, data: u32) -> GPtr<u8> = cdecl 0x749B70 => unported;
    /// `CEntity::DeleteRwObject()` (0x534030)
    pub fn entity_delete_rw_object(e: GPtr<u8>) = thiscall 0x534030 => unported;
    /// `CVehicle::CalculateLightingFromCollision()` (0x6D0CF0)
    pub fn calculate_lighting_from_collision_o(v: GPtr<u8>) = thiscall 0x6D0CF0 => calculate_lighting_from_collision;
    /// `CVehicle::PreRenderDriverAndPassengers()` (0x6D3DB0)
    pub fn pre_render_driver_and_passengers_o(v: GPtr<u8>) = thiscall 0x6D3DB0 => pre_render_driver_and_passengers;
    /// `CEntity::ProcessLightsForEntity()` (0x6FC7A0)
    pub fn entity_process_lights_for_entity(e: GPtr<u8>) = thiscall 0x6FC7A0 => sa_world::world::entity_lights::process_lights_for_entity;
    /// `CPhysical::GetLightingFromCol(bool)` (0x5447B0), float in ST0
    pub fn physical_get_lighting_from_col_st0(p: GPtr<u8>, interior: u32) -> f64 = thiscall 0x5447B0 => unported;
    /// `CVehicleModelInfo::SetEnvMapCoeff(float)` (0x4C9800)
    pub fn vmi_set_env_map_coeff(mi: GPtr<u8>, c: f32) = thiscall 0x4C9800 => unported;
    /// `CVisibilityPlugins::GetClumpAlpha(RpClump*)` (0x732B20)
    pub fn vis_get_clump_alpha(clump: GPtr<u8>) -> i32 = cdecl 0x732B20 => unported;
    /// `CVisibilityPlugins::SetClumpAlpha(RpClump*, int32)` (0x732B00)
    pub fn vis_set_clump_alpha(clump: GPtr<u8>, a: i32) = cdecl 0x732B00 => unported;
}

/// `CTimer::ms_fTimeStep`.
fn time_step() -> f32 {
    g::<f32>(0xB7CB5C).read()
}

/// `CVehicle::ProcessDelayedExplosion()` (0x6D1340): the bomb timer (+0x4DE, u16) counts down by
/// `ftol(timeStep * 16.666666)` (clamped at 0, unsigned compare); when it reaches 0, virtual
/// `BlowUpCar(bomb owner +0x4E0, false)`.
#[hook(addr = 0x6D1340, conv = thiscall, name = "CVehicle::ProcessDelayedExplosion", expect = "56 8B F1 57 66 8B BE DE 04 00 00", this = "CVehicle")]
pub fn process_delayed_explosion(v: Veh) {
    let t = rd::<u16>(v, 0x4DE);
    if t == 0 {
        return;
    }
    let d = ftol32(x(time_step()).fmul_m(g::<f32>(0x858CF0).read())) as u32;
    if u32::from(t) <= d {
        wr::<u16>(v, 0x4DE, 0);
    } else {
        wr::<u16>(v, 0x4DE, t.wrapping_sub(d as u16));
    }
    if rd::<u16>(v, 0x4DE) == 0 {
        blow_up_car_virtual(v, rd::<GPtr<u8>>(v, 0x4E0), 0);
    }
}

/// `CVehicle::ActivateBombWhenEntered()` (0x6D2570): with a driver: unless already armed (bit 5 of
/// `+0x42C`), a bomb type (+0x4A8 & 7) of 5 starts the timer (+0x4DE = 1000) with the bomb owner
/// (+0x4E0 = +0x4CC, reference registered when not null); bit 5 is set. Without a driver bit 5 is
/// cleared.
#[hook(addr = 0x6D2570, conv = thiscall, name = "CVehicle::ActivateBombWhenEntered", expect = "56 8B F1 8B 86 60 04 00 00", this = "CVehicle")]
pub fn activate_bomb_when_entered(v: Veh) {
    let f = rd::<u8>(v, 0x42C);
    if rd::<u32>(v, 0x460) == 0 {
        wr::<u8>(v, 0x42C, f & 0xDF);
        return;
    }
    if f & 0x20 == 0 && rd::<u8>(v, 0x4A8) & 7 == 5 {
        let owner = rd::<GPtr<u8>>(v, 0x4CC);
        wr::<u16>(v, 0x4DE, 1000);
        wr::<GPtr<u8>>(v, 0x4E0, owner);
        if !owner.is_null() {
            entity_register_reference(owner, v.byte_add(0x4E0));
        }
    }
    wr::<u8>(v, 0x42C, rd::<u8>(v, 0x42C) | 0x20);
}

/// `CVehicle::ProcessCarAlarm()` (0x6D21F0): an active alarm (+0x45C, not 0 / 0xFFFF) counts down by
/// `ftol(timeStep * 0.02 * 1000.0)`; when the remaining time is below that step it becomes 0 and
/// `+0x514` (the horn state) is cleared.
#[hook(addr = 0x6D21F0, conv = thiscall, name = "CVehicle::ProcessCarAlarm", expect = "56 57 8B F9 66 8B B7 5C 04 00 00", this = "CVehicle")]
pub fn process_car_alarm(v: Veh) {
    let t = rd::<u16>(v, 0x45C);
    if t == 0 || t == 0xFFFF {
        return;
    }
    let step = x(time_step())
        .fmul_m(g::<f32>(0x858B38).read())
        .fmul_m(g::<f32>(0x858C4C).read());
    let d = ftol32(step) as u32;
    if u32::from(t) < d {
        wr::<u16>(v, 0x45C, 0);
        wr::<u32>(v, 0x514, 0);
    } else {
        wr::<u16>(v, 0x45C, t.wrapping_sub(d as u16));
    }
}

/// `CVehicle::DeleteRwObject()` (0x6D6410): a loaded remap TXD (+0x598 != -1) is released
/// (+0x59C = null first, then `CTxdStore::RemoveRef`, +0x598 / +0x59A = -1); every upgrade atomic is
/// removed (`RemoveAllUpgradesCB`); the 15 upgrade slots (+0x43A, 30 bytes) become -1; then
/// `CEntity::DeleteRwObject()`.
#[hook(addr = 0x6D6410, conv = thiscall, name = "CVehicle::DeleteRwObject", expect = "56 8B F1 66 8B 86 98 05 00 00", this = "CVehicle")]
pub fn delete_rw_object(v: Veh) {
    let txd = rd::<i16>(v, 0x598);
    if txd != -1 {
        wr::<u32>(v, 0x59C, 0);
        txd_store_remove_ref(i32::from(txd));
        wr::<u16>(v, 0x598, 0xFFFF);
        wr::<u16>(v, 0x59A, 0xFFFF);
    }
    rp_clump_for_all_atomics(rd::<GPtr<u8>>(v, 0x18), 0x6D34D0, 0);
    for k in 0..7 {
        wr::<u32>(v, 0x43A + 4 * k, 0xFFFF_FFFF);
    }
    wr::<u16>(v, 0x456, 0xFFFF);
    entity_delete_rw_object(v);
}

/// `CVehicle::PreRender()` (0x6D6480): lighting from collision unless a train (subclass 6), the
/// occupants' PreRender, `ProcessLightsForEntity` when the model has 2D effects (`mi + 0xD`), the low
/// nibble of `+0x584` cleared, and the env-map coefficient `GetLightingFromCol(false) * 0.4`.
#[hook(addr = 0x6D6480, conv = thiscall, name = "CVehicle::PreRender", expect = "51 56 8B F1 83 BE 90 05 00 00 06", this = "CVehicle")]
pub fn pre_render(v: Veh) {
    if rd::<u32>(v, 0x590) != 6 {
        calculate_lighting_from_collision_o(v);
    }
    pre_render_driver_and_passengers_o(v);
    if rd::<u8>(mi_of(v), 0xD) != 0 {
        entity_process_lights_for_entity(v);
    }
    wr::<u8>(v, 0x584, rd::<u8>(v, 0x584) & 0xF0);
    let c = X87::from_f64_exact(physical_get_lighting_from_col_st0(v, 0)).fmul_m(g::<f32>(0x858EE8).read());
    vmi_set_env_map_coeff(mi_of(v), c.to_f32());
}

/// `CVehicle::UpdateClumpAlpha()` (0x6D2980): fading out (bit 2 of `+0x42A`): alpha - 8, below 0 -> 0;
/// else below 255: alpha + 16 capped at 255; then `SetClumpAlpha`.
#[hook(addr = 0x6D2980, conv = thiscall, name = "CVehicle::UpdateClumpAlpha", expect = "56 8B F1 8B 46 18", this = "CVehicle")]
pub fn update_clump_alpha(v: Veh) {
    let mut a = vis_get_clump_alpha(rd::<GPtr<u8>>(v, 0x18));
    if rd::<u8>(v, 0x42A) & 4 != 0 {
        a = a.wrapping_sub(8);
        if a < 0 {
            a = 0;
        }
    } else if a < 0xFF {
        a += 0x10;
        if a > 0xFF {
            a = 0xFF;
        }
    }
    vis_set_clump_alpha(rd::<GPtr<u8>>(v, 0x18), a);
}

/// `CVehicle::PickRandomPassenger()` (0x6D2A10): start = `ftol(rand frac * 8.0)` (rand at 0x6D2A15),
/// then the first non-empty passenger slot `(i + start) % 8` (C remainder, i = 0..7); null if none.
#[hook(addr = 0x6D2A10, conv = thiscall, name = "CVehicle::PickRandomPassenger", expect = "51 56 57 8B F1", this = "CVehicle")]
pub fn pick_random_passenger(v: Veh) -> GPtr<u8> {
    let r = rand_at(0x6D2A1A);
    let start = ftol32(rand_frac(r).fmul_m(g::<f32>(0x859000).read()));
    for i in 0..8i32 {
        let k = i.wrapping_add(start) % 8;
        let p = rd::<GPtr<u8>>(v, 0x464 + 4 * k as isize);
        if !p.is_null() {
            return p;
        }
    }
    GPtr::NULL
}

/// `CVehicle::HeightAboveCeiling(float height, eFlightModel)` (0x6D2600): models 1/2 (heli-like):
/// h < 500 -> -1; h < 950 -> h - 500; h < 1500 -> -1; else `(h - 1000) + 500`. Other models:
/// h < 800 -> -1, else h - 800. ST0; the NaN paths follow the `jp`/`jnp` tests.
#[hook(addr = 0x6D2600, conv = thiscall, name = "CVehicle::HeightAboveCeiling", expect = "8B 44 24 08 83 F8 01", preserve = "edx", this = "CVehicle")]
pub fn height_above_ceiling(_v: Veh, h: f32, model: u32) -> X87 {
    let c = |a: usize| g::<f32>(a).read();
    let minus1 = x(c(0x858C1C));
    if model != 1 && model != 2 {
        if !x(h).fcom(x(c(0x8594DC))).test_parity(0x05) {
            return minus1;
        }
        return x(h).fsub_m(c(0x8594DC));
    }
    if !x(h).fcom(x(c(0x858B58))).test_parity(0x05) {
        return minus1;
    }
    if !x(h).fcom(x(c(0x858F4C))).test_parity(0x05) {
        return x(h).fsub_m(c(0x858B58));
    }
    if !x(h).fcom(x(c(0x86CD80))).test_parity(0x05) {
        return minus1;
    }
    x(h).fsub_m(c(0x858C4C)).fadd_m(c(0x858B58))
}

orig! {
    /// `CVehicleModelInfo::GetCustomCarPlateText()` (0x4C8970)
    pub fn vmi_get_custom_car_plate_text(mi: GPtr<u8>) -> GPtr<u8> = thiscall 0x4C8970 => unported;
    /// `CVehicleModelInfo::SetCustomCarPlateText(char*)` (0x4C8980)
    pub fn vmi_set_custom_car_plate_text(mi: GPtr<u8>, text: u32) = thiscall 0x4C8980 => unported;
    /// `CCustomCarPlateMgr::CreatePlateTexture(char* text, uint8 type)` (0x6FDEA0); the type dword is
    /// passed whole (see CustomCarPlate_TextureCreate)
    pub fn create_plate_texture(text: GPtr<u8>, ty: u32) -> GPtr<u8> = cdecl 0x6FDEA0 => unported;
    /// `CEntity::SetModelIndex(uint32)` (0x532AE0)
    pub fn entity_set_model_index(e: GPtr<u8>, mi: u32) = thiscall 0x532AE0 => unported;
    /// `CVehicle::CustomCarPlate_TextureCreate(CVehicleModelInfo*)` (0x6D10E0)
    pub fn custom_car_plate_texture_create_o(v: GPtr<u8>, mi: GPtr<u8>) -> u8 = thiscall 0x6D10E0 => custom_car_plate_texture_create;
    /// `CVehicleModelInfo::GetMaximumNumberOfPassengersFromNumberOfDoors(int32)` (0x4C89B0)
    pub fn vmi_max_passengers_from_doors(model: u32) -> u8 = cdecl 0x4C89B0 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    pub fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// virtual `CVehicle::SetupSuspensionLines()` (vtable +0xC0)
    pub fn setup_suspension_lines_virtual(v: GPtr<u8>) = thiscall vslot 0xC0 => unported;
    /// `CAudioEngine::SetRadioBassSetting(int8)` (0x506FA0)
    pub fn audio_set_radio_bass_setting(ae: GPtr<u8>, v: u32) = thiscall 0x506FA0 => unported;
    /// `CAutomobile::NitrousControl(int8)` (0x6A3EA0)
    pub fn nitrous_control(a: GPtr<u8>, v: i32) = thiscall 0x6A3EA0 => unported;
    /// `CRopes::FindRope(uint32 id)` (0x556000)
    pub fn ropes_find_rope(id: u32) -> i32 = cdecl 0x556000 => unported;
    /// `operator*(CVector* out, const CMatrix&, const CVector&)` (0x59C890)
    pub fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CRopes::RegisterRope(uint32 id, uint32 type, CVector pos, bool expires, uint8 segments, uint8 flags,
    /// CPhysical* holder, uint32 timeExpire)` (0x556B40)
    pub fn ropes_register_rope(id: u32, ty: u32, px: u32, py: u32, pz: u32, expires: u32, segments: u32, flags: u32, holder: GPtr<u8>, time: u32) -> u8 = cdecl 0x556B40 => unported;
    /// `FindPlayerPed(int32)` (0x56E210)
    pub fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPlayerPed::DoesPlayerWantNewWeapon(eWeaponType, bool)` (0x609710)
    pub fn does_player_want_new_weapon(p: GPtr<u8>, ty: u32, b: u32) -> u8 = thiscall 0x609710 => unported;
    /// `CPed::GiveWeapon(eWeaponType, uint32 ammo, bool)` (0x5E6080)
    pub fn ped_give_weapon(p: GPtr<u8>, ty: u32, ammo: u32, b: u32) -> u32 = thiscall 0x5E6080 => unported;
    /// `CPed::GrantAmmo(eWeaponType, uint32)` (0x5DF220)
    pub fn ped_grant_ammo(p: GPtr<u8>, ty: u32, ammo: u32) = thiscall 0x5DF220 => unported;
    /// `CStreaming::SetModelIsDeletable(int32)` (0x409C10)
    pub fn streaming_set_model_is_deletable(model: i32) = cdecl 0x409C10 => unported;
    /// `CGameLogic::IsCoopGameGoingOn()` (0x441390)
    pub fn is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
    /// `CDarkel::RegisterKillByPlayer(const CPed*, eWeaponType, bool headShot, int32)` (0x43DCD0)
    pub fn darkel_register_kill_by_player(ped: GPtr<u8>, w: u32, head: u32, n: i32) = cdecl 0x43DCD0 => unported;
    /// `CEventVehicleDied::CEventVehicleDied(CVehicle*)` (0x4B0D10)
    pub fn event_vehicle_died_ctor(e: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = thiscall 0x4B0D10 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    pub fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CEventVehicleDied::~CEventVehicleDied()` (0x4B0DA0)
    pub fn event_vehicle_died_dtor(e: GPtr<u8>) = thiscall 0x4B0DA0 => unported;
}

/// `CVehicle::CustomCarPlate_TextureCreate(CVehicleModelInfo*)` (0x6D10E0): plate texture (+0x588) =
/// null; no plate material (`mi + 0x24`) -> false. A custom plate text gives
/// `CreatePlateTexture(text, type)`, clears the text and sets the plate type (`mi + 0x31`) to 0xFF; else
/// the material's texture gets a reference (`+0x54`) and is used. The original passes the type as the
/// dword of its own argument slot after storing the type byte into it: the upper 3 bytes are the
/// model-info pointer's (kept).
#[hook(addr = 0x6D10E0, conv = thiscall, name = "CVehicle::CustomCarPlate_TextureCreate", expect = "56 8B 74 24 08", this = "CVehicle")]
pub fn custom_car_plate_texture_create(v: Veh, mi: GPtr<u8>) -> u8 {
    wr::<u32>(v, 0x588, 0);
    if rd::<u32>(mi, 0x24) == 0 {
        return 0;
    }
    let text = vmi_get_custom_car_plate_text(mi);
    if !text.is_null() {
        let ty = (mi.addr() as u32 & 0xFFFF_FF00) | u32::from(rd::<u8>(mi, 0x31));
        let tex = create_plate_texture(text, ty);
        wr::<GPtr<u8>>(v, 0x588, tex);
        vmi_set_custom_car_plate_text(mi, 0);
        wr::<u8>(mi, 0x31, 0xFF);
        return 1;
    }
    let tex = rd::<GPtr<u8>>(rd::<GPtr<u8>>(mi, 0x24), 0);
    wr::<u32>(tex, 0x54, rd::<u32>(tex, 0x54).wrapping_add(1));
    wr::<GPtr<u8>>(v, 0x588, tex);
    1
}

/// `CVehicle::CanBeDriven()` (0x6D5400): false for vehicle type (+0x594) 11, for type 6 with `+0x5C0`
/// set, and with bit 7 of `+0x42C`; else true when the model's seat offset (vehicle struct `mi + 0x5C`,
/// +0x30 unless the model type `mi + 0x3C` is 5) has `(z*z + y*y) + x*x > 0` (fcomp, `test ah, 0x41`;
/// NaN -> false). AL on the early exits, EAX 0/1 at the end.
#[hook(addr = 0x6D5400, conv = thiscall, name = "CVehicle::CanBeDriven", expect = "8B 81 94 05 00 00", this = "CVehicle")]
pub fn can_be_driven(v: Veh) -> u8 {
    let ty = rd::<u32>(v, 0x594);
    if ty == 0xB || (ty == 6 && rd::<u8>(v, 0x5C0) != 0) || rd::<u8>(v, 0x42C) & 0x80 != 0 {
        return 0;
    }
    let mi = mi_of(v);
    let mut s = rd::<GPtr<u8>>(mi, 0x5C);
    if rd::<u32>(mi, 0x3C) != 5 {
        s = s.byte_add(0x30);
    }
    let (px, py, pz) = (rd::<f32>(s, 0), rd::<f32>(s, 4), rd::<f32>(s, 8));
    let sum = x(pz).fmul_m(pz).fadd(x(py).fmul_m(py)).fadd(x(px).fmul_m(px));
    b(!sum.fcom(x(g::<f32>(0x858B50).read())).test_any(0x41))
}

/// `CVehicle::SetModelIndex(uint32)` (0x6D6A40): `CEntity::SetModelIndex`, the custom plate texture,
/// the colour bytes +0x438/+0x439 from 0xB4E478/9, max passengers from the door count; bit 7 of `+0x42C`
/// set for models 441, 464, 465, 501, 564 (RC vehicles) and cleared otherwise; `+0x513` (plane weapon)
/// = 1 for 476/513, 2 for 511, 3 for 520/576.
#[hook(addr = 0x6D6A40, conv = thiscall, name = "CVehicle::SetModelIndex", expect = "56 57 8B 7C 24 0C", this = "CVehicle")]
pub fn set_model_index(v: Veh, model: u32) {
    entity_set_model_index(v, model);
    custom_car_plate_texture_create_o(v, g::<GPtr<u8>>(MODEL_INFO_PTRS + 4 * model as usize).read());
    wr::<u8>(v, 0x438, g::<u8>(0xB4E478).read());
    wr::<u8>(v, 0x439, g::<u8>(0xB4E479).read());
    let n = vmi_max_passengers_from_doors(model);
    wr::<u8>(v, 0x488, n);
    let m = i32::from(rd::<i16>(v, 0x22));
    let f = rd::<u8>(v, 0x42C);
    wr::<u8>(
        v,
        0x42C,
        if matches!(m, 441 | 464 | 465 | 501 | 564) {
            f | 0x80
        } else {
            f & 0x7F
        },
    );
    match m {
        476 | 513 => wr::<u8>(v, 0x513, 1),
        511 => wr::<u8>(v, 0x513, 2),
        520 | 576 => wr::<u8>(v, 0x513, 3),
        _ => {}
    }
}

/// `CVehicle::RemovePassenger(CPed*)` (0x6D1610, the subclass test sits in a .HOODLUM fragment at
/// 0x4074C3): a train (subclass 6) searches all 8 slots, other vehicles the first `+0x488` (max
/// passengers); the slot found releases its reference, becomes null and the count (+0x484) drops.
#[hook(addr = 0x6D1610, conv = thiscall, name = "CVehicle::RemovePassenger", expect = "8B 54 24 04 85 D2", this = "CVehicle")]
pub fn remove_passenger(v: Veh, ped: GPtr<u8>) {
    if ped.is_null() {
        return;
    }
    let n = if rd::<u32>(v, 0x590) == 6 {
        8
    } else {
        i32::from(rd::<u8>(v, 0x488))
    };
    let mut i = 0;
    while i < n {
        if rd::<GPtr<u8>>(v, 0x464 + 4 * i as isize) == ped {
            let slot = v.byte_add(0x464 + 4 * i as isize);
            let p = slot.cast::<GPtr<u8>>().read();
            if !p.is_null() {
                entity_clean_up_old_reference(p, slot);
            }
            slot.cast::<u32>().write(0);
            wr::<u8>(v, 0x484, rd::<u8>(v, 0x484).wrapping_sub(1));
            return;
        }
        i += 1;
        if rd::<u32>(v, 0x590) != 6 {
            // the loop bound is re-read every iteration (movzx edi, byte [esi + 0x488])
            if i >= i32::from(rd::<u8>(v, 0x488)) {
                return;
            }
        }
    }
}

/// `CVehicle::ClearVehicleUpgradeFlags(int32, int32 compType)` (0x6D3210): 16 (hydraulics): an
/// installed hydraulics slot (`+0x48B` > -1) is freed in 0xC1CC08, `+0x48B` = -1, virtual
/// `SetupSuspensionLines`, `+0xB8` = 0, `+0x4C` = 0.02; the handling flag 0x20000 (+0x38C) cleared;
/// true. 17 (stereo): when `+0x1D3` is 0 and bit 4 of `+0x42E` is set, the bass setting (+0x1BE) 0 -> 2,
/// 1 -> 0, `SetRadioBassSetting`, bit 4 cleared; true. 15 (nitro, automobiles only):
/// `NitrousControl(-1)`, then true only when the model's nitro slot (`[mi + 0x5C] + 0x2B0`) is negative.
/// Anything else false.
#[hook(addr = 0x6D3210, conv = thiscall, name = "CVehicle::ClearVehicleUpgradeFlags", expect = "8B 44 24 08 83 F8 10", this = "CVehicle")]
pub fn clear_vehicle_upgrade_flags(v: Veh, _unused: u32, comp: u32) -> u8 {
    match comp {
        0x10 => {
            if rd::<u32>(v, 0x38C) & 0x20000 != 0 {
                let slot = rd::<i8>(v, 0x48B);
                if slot > -1 {
                    g::<u32>(0xC1CC08 + 4 * slot as usize).write(0);
                    wr::<u8>(v, 0x48B, 0xFF);
                    setup_suspension_lines_virtual(v);
                    wr::<u8>(v, 0xB8, 0);
                    wr::<u32>(v, 0x4C, 0x3CA3_D70A);
                }
            }
            wr::<u32>(v, 0x38C, rd::<u32>(v, 0x38C) & 0xFFFD_FFFF);
            1
        }
        0x11 => {
            if rd::<u8>(v, 0x1D3) == 0 && rd::<u8>(v, 0x42E) & 0x10 != 0 {
                match rd::<u8>(v, 0x1BE) {
                    0 => wr::<u8>(v, 0x1BE, 2),
                    1 => wr::<u8>(v, 0x1BE, 0),
                    _ => {}
                }
                audio_set_radio_bass_setting(GPtr::from_addr(0xB6BC90), u32::from(rd::<u8>(v, 0x1BE)));
                wr::<u8>(v, 0x42E, rd::<u8>(v, 0x42E) & 0xEF);
            }
            1
        }
        0xF if rd::<u32>(v, 0x590) == 0 => {
            nitrous_control(v, -1);
            b(rd::<i32>(rd::<GPtr<u8>>(mi_of(v), 0x5C), 0x2B0) < 0)
        }
        _ => 0,
    }
}

/// `CVehicle::SelectPlaneWeapon(bool, eOrdnanceType)` (0x6D4900): `+0x513` = 1 when choosing, else kept;
/// then by model: 425 (hunter) and 576: ordnance 1 -> 4; 476 (rustler): 1 -> 2; 520 (hydra): 1 -> 4,
/// 2 -> 3; 447/464: nothing more. Other models are untouched.
#[hook(addr = 0x6D4900, conv = thiscall, name = "CVehicle::SelectPlaneWeapon", expect = "0F BF 41 22 05 57 FE FF FF", preserve = "edx", this = "CVehicle")]
pub fn select_plane_weapon(v: Veh, choice: u32, ordnance: u32) {
    let m = i32::from(rd::<i16>(v, 0x22));
    if !matches!(m, 425 | 447 | 464 | 476 | 520 | 576) {
        return;
    }
    let cur = if choice as u8 != 0 { 1 } else { rd::<u8>(v, 0x513) };
    wr::<u8>(v, 0x513, cur);
    match (m, ordnance) {
        (425 | 576, 1) => wr::<u8>(v, 0x513, 4),
        (476, 1) => wr::<u8>(v, 0x513, 2),
        (520, 1) => wr::<u8>(v, 0x513, 4),
        (520, 2) => wr::<u8>(v, 0x513, 3),
        _ => {}
    }
}

/// `CVehicle::UpdateWinch()` (0x6D3B80): for a winch type (`+0x4A8 >> 5 & 3`) other than 0: rope id =
/// `this + 0x1D`; length = -0.2 (type 3) or -0.6; with an existing rope, segments =
/// `ftol(rope+0x31C * 32.0)` and length = `(rope+0x308 * rope+0x31C - segments * rope+0x30C) + length`;
/// the start is `matrix * (0, 0, length)`; `CRopes::RegisterRope(id, type, start, false, segments, 1,
/// this, 20000)`.
#[hook(addr = 0x6D3B80, conv = thiscall, name = "CVehicle::UpdateWinch", expect = "83 EC 20 53 55", this = "CVehicle")]
pub fn update_winch(v: Veh) {
    let ty = (rd::<u8>(v, 0x4A8) >> 5) & 3;
    if ty == 0 {
        return;
    }
    let id = (v.addr() as u32).wrapping_add(0x1D);
    let len = f32::from_bits(if ty == 3 { 0xBE4C_CCCD } else { 0xBF19_999A });
    let mut segs = 0i32;
    let r = ropes_find_rope(id);
    let z = if r >= 0 {
        let rope = g::<u8>(0xB768B8).byte_add(r as isize * 0x328);
        segs = ftol32(x(rd::<f32>(rope, 0x31C)).fmul_m(g::<f32>(0x85950C).read()));
        let a = x(rd::<f32>(rope, 0x308)).fmul_m(rd::<f32>(rope, 0x31C));
        let n = xi(segs).fmul_m(rd::<f32>(rope, 0x30C));
        a.fsub(n).fadd_m(len)
    } else {
        x(len)
    };
    let mut vec = [0f32, 0.0, z.to_f32()];
    let mut out = [0u32; 3];
    let outp = GPtr::from_raw(&raw mut out).cast::<u8>();
    matrix_mul_vec(
        outp,
        rd::<GPtr<u8>>(v, 0x14),
        GPtr::from_raw(&raw mut vec).cast::<u8>(),
    );
    ropes_register_rope(
        id,
        u32::from(ty),
        out[0],
        out[1],
        out[2],
        0,
        segs as u32,
        1,
        v,
        20000,
    );
}

/// `CVehicle::RemoveDriver(bool)` (0x6D1950): status (+0x36 bits 3..7) = 4 (abandoned); unless the
/// argument is set or the driver is a player, bit 4 of `+0x428` is cleared. When the driver is the
/// player: caddy (457) with 0x8E66D4 == 1 hands out a golf club (`GiveWeapon(2, 1, true)` when wanted)
/// and frees model 333; police cars 596..599 with 0x8E6814 == 1 hand out the shotgun once (bit 7 of
/// `+0x428`: `GiveWeapon(25, 5, true)` when wanted, else `GrantAmmo(25, 5)`) and free model 349. Then the
/// driver reference is released and +0x460 = null.
#[hook(addr = 0x6D1950, conv = thiscall, name = "CVehicle::RemoveDriver", expect = "53 56 57 8B F9", this = "CVehicle")]
pub fn remove_driver(v: Veh, keep: u32) {
    wr::<u8>(v, 0x36, (rd::<u8>(v, 0x36) & 7) | 0x20);
    if keep as u8 == 0 {
        let d = rd::<GPtr<u8>>(v, 0x460);
        if d.is_null() || ped_is_player(d) == 0 {
            wr::<u8>(v, 0x428, rd::<u8>(v, 0x428) & 0xEF);
        }
    }
    let d = rd::<GPtr<u8>>(v, 0x460);
    if d == find_player_ped(-1) {
        let m = i32::from(rd::<i16>(v, 0x22));
        if m == 0x1C9 {
            if g::<u8>(0x8E66D4).read() == 1 {
                if does_player_want_new_weapon(d, 2, 1) != 0 {
                    ped_give_weapon(rd::<GPtr<u8>>(v, 0x460), 2, 1, 1);
                }
                streaming_set_model_is_deletable(0x14D);
            }
        } else if (0x254..=0x257).contains(&m) && g::<u8>(0x8E6814).read() == 1 {
            if rd::<u8>(v, 0x428) & 0x80 != 0 {
                let want = does_player_want_new_weapon(d, 0x19, 1);
                let p = rd::<GPtr<u8>>(v, 0x460);
                if want != 0 {
                    ped_give_weapon(p, 0x19, 5, 1);
                } else {
                    ped_grant_ammo(p, 0x19, 5);
                }
                wr::<u8>(v, 0x428, rd::<u8>(v, 0x428) & 0x7F);
            }
            streaming_set_model_is_deletable(0x15D);
        }
    }
    let slot = v.byte_add(0x460);
    let p = rd::<GPtr<u8>>(v, 0x460);
    if !p.is_null() {
        entity_clean_up_old_reference(p, slot);
    }
    wr::<u32>(v, 0x460, 0);
}

/// `CVehicle::KillPedsInVehicle()` (0x6D1C80): `kill_ped_slot` for the driver and for every non-null
/// passenger slot below the max passenger count (+0x488, re-read each iteration). The ped pointer is
/// re-read from the slot after the event constructor, as the original does.
#[hook(addr = 0x6D1C80, conv = thiscall, name = "CVehicle::KillPedsInVehicle", expect = "64 A1 00 00 00 00", this = "CVehicle")]
pub fn kill_peds_in_vehicle(v: Veh) {
    let d = rd::<GPtr<u8>>(v, 0x460);
    if !d.is_null() {
        kill_ped_slot(v, 0x460);
    }
    let mut i = 0isize;
    while i < isize::from(rd::<u8>(v, 0x488)) {
        if rd::<u32>(v, 0x464 + 4 * i) != 0 {
            kill_ped_slot(v, 0x464 + 4 * i);
        }
        i += 1;
    }
}

/// One `CEventVehicleDied(this)` added to the event group (`[ped + 0x47C] + 0x68`) of the ped in the slot at
/// `off`, after `CDarkel::RegisterKillByPlayer(ped, 0x33, false, 0)` unless a co-op game is on; the ped
/// is re-read from the slot after every call, as the original does.
fn kill_ped_slot(v: Veh, off: isize) {
    if is_coop_game_going_on() == 0 {
        darkel_register_kill_by_player(rd::<GPtr<u8>>(v, off), 0x33, 0, 0);
    }
    let mut ev = [0u32; 6];
    let e = GPtr::from_raw(&raw mut ev).cast::<u8>();
    event_vehicle_died_ctor(e, v);
    let ped = rd::<GPtr<u8>>(v, off);
    event_group_add(rd::<GPtr<u8>>(ped, 0x47C).byte_add(0x68), e, 0);
    event_vehicle_died_dtor(e);
}

orig! {
    /// `CCullZones::CamNoRain()` (0x72DDB0)
    pub fn cull_zones_cam_no_rain() -> u8 = cdecl 0x72DDB0 => unported;
    /// `CVehicle::UsesSiren()` (0x6D8470)
    pub fn uses_siren_o(v: GPtr<u8>) -> u8 = thiscall 0x6D8470 => uses_siren;
    /// `CPad::GetHorn()` (0x53FEE0)
    pub fn pad_get_horn(pad: GPtr<u8>) -> u8 = thiscall 0x53FEE0 => unported;
    /// `CAutomobile::CustomCarPlate_BeforeRenderingStart(CVehicleModelInfo*)` (0x6A2F00)
    pub fn custom_car_plate_before_rendering_start(a: GPtr<u8>, mi: GPtr<u8>) = thiscall 0x6A2F00 => unported;
    /// `CTxdStore::AddRef(int32)` (0x731A00)
    pub fn txd_store_add_ref(slot: i32) = cdecl 0x731A00 => unported;
    /// `GetFirstTexture(RwTexDictionary*)` (0x734940)
    pub fn get_first_texture(txd: GPtr<u8>) -> GPtr<u8> = cdecl 0x734940 => unported;
    /// `CStreaming::RequestModel(int32, int32 flags)` (0x4087E0)
    pub fn streaming_request_model(model: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CVehicleModelInfo::SetVehicleColour(uint8, uint8, uint8, uint8)` (0x4C84B0)
    pub fn vmi_set_vehicle_colour(mi: GPtr<u8>, c0: u32, c1: u32, c2: u32, c3: u32) = thiscall 0x4C84B0 => unported;
    /// `CVehicleModelInfo::SetupLightFlags(CVehicle*)` (0x4C8C90)
    pub fn vmi_setup_light_flags(v: GPtr<u8>) = cdecl 0x4C8C90 => unported;
    /// `CVehicleModelInfo::SetEditableMaterials(RpClump*)` (0x4C8430)
    pub fn vmi_set_editable_materials(clump: GPtr<u8>) = cdecl 0x4C8430 => unported;
}

/// `CVehicle::CalculateLightingFromCollision()` (0x6D0CF0): for each of the four wheel lighting bytes
/// (+0x574..+0x577; low nibble = day, high nibble = night) `(lo * K) * (1 - D) + (hi * K) * D` with
/// K = 1/30 (0x858F10) and D = the day/night balance (0x8D12C0), summed in order, times 0.25 ->
/// `+0x12C`. x87 at PC = 24, the original's operation order.
#[hook(addr = 0x6D0CF0, conv = thiscall, name = "CVehicle::CalculateLightingFromCollision", expect = "51 8A 81 74 05 00 00", this = "CVehicle")]
pub fn calculate_lighting_from_collision(v: Veh) {
    let k = g::<f32>(0x858F10).read();
    let one = g::<f32>(0x858624).read();
    let mut sum: Option<X87> = None;
    for i in 0..4 {
        let d = x(g::<f32>(0x8D12C0).read());
        let byte = rd::<u8>(v, 0x574 + i);
        let lo = xi(i32::from(byte & 0xF)).fmul_m(k);
        let a = lo.fmul(x(one).fsub(d));
        let hi = xi(i32::from(byte >> 4)).fmul_m(k).fmul(d);
        let t = a.fadd(hi);
        sum = Some(match sum {
            None => t,
            Some(s) => s.fadd(t),
        });
    }
    let r = sum.unwrap_or(X87::zero()).fmul_m(g::<f32>(0x858C84).read());
    wr::<f32>(v, 0x12C, r.to_f32());
}

/// `CVehicle::ProcessSirenAndHorn(bool)` (0x6E0950, the history index read sits in a .HOODLUM fragment
/// at 0x403940): siren vehicles read the horn history (5 bytes at 0xB73569, index 0xB7356E): a press
/// pattern toggles the siren (bit 7 of `+0x42D`) with the horn (+0x514) off, a held horn sets +0x514 = 1,
/// else 0. Other vehicles with the argument set and no running alarm (`+0x45C` 0/0xFFFF, or status 5)
/// get +0x514 = `CPad::GetHorn() != 0`.
#[hook(addr = 0x6E0950, conv = thiscall, name = "CVehicle::ProcessSirenAndHorn", expect = "56 57 8B F1 E8 17 7B FF FF", this = "CVehicle")]
pub fn process_siren_and_horn(v: Veh, check: u32) {
    if uses_siren_o(v) != 0 {
        let c = i32::from(g::<u8>(0xB7356E).read());
        let hist = |k: i32| g::<u8>(0xB73569 + k as usize).read();
        let h = |k: i32| hist(k % 5);
        if hist(c) != 0 {
            if h(c + 4) != 0 && h(c + 3) != 0 {
                wr::<u32>(v, 0x514, 1);
            } else {
                wr::<u32>(v, 0x514, 0);
            }
            return;
        }
        if h(c + 4) == 0 || h(c + 1) != 0 {
            wr::<u32>(v, 0x514, 0);
            return;
        }
        wr::<u32>(v, 0x514, 0);
        wr::<u8>(v, 0x42D, rd::<u8>(v, 0x42D) ^ 0x80);
        return;
    }
    if check as u8 == 0 {
        return;
    }
    let w = rd::<u16>(v, 0x45C);
    if w == 0 || w == 0xFFFF || rd::<u8>(v, 0x36) & 0xF8 == 0x28 {
        let horn = pad_get_horn(GPtr::from_addr(0xB73458));
        wr::<u32>(v, 0x514, u32::from(horn != 0));
    }
}

/// `CVehicle::GetVehicleLightsStatus()` (0x6D55C0): lights on after 21:00, at 20:mm past the vehicle's
/// random minute (`+0x20 & 0x3F`), before 6:00, at 6:mm before it; else when either weather factor
/// (0xC81300, 0xC81308) exceeds `(random seed word +0x20) * 2e-5`; also when the vehicle is dark
/// (lighting +0x12C < 0.05) in a `CamNoRain` cull zone. Off again for a driver with ped type 7..16 and
/// bit 0 of its `+0x20` while 0xC0BC5C is set. AL only.
#[hook(addr = 0x6D55C0, conv = thiscall, name = "CVehicle::GetVehicleLightsStatus", expect = "51 A0 53 01 B7 00", this = "CVehicle")]
pub fn get_vehicle_lights_status(v: Veh) -> u8 {
    let h = g::<u8>(0xB70153).read();
    let m = u16::from(g::<u8>(0xB70152).read());
    let r = u16::from(rd::<u8>(v, 0x20) & 0x3F);
    let mut on = if h > 20 || (h > 19 && m > r) || h < 6 || (h < 7 && m < r) {
        true
    } else {
        let s = xi(i32::from(rd::<u16>(v, 0x20))).fmul_m(g::<f32>(0x871E78).read());
        !x(g::<f32>(0xC81300).read()).fcom(s).test_any(0x41)
            || !x(g::<f32>(0xC81308).read()).fcom(s).test_any(0x41)
    };
    let mut check_driver = on;
    if !x(rd::<f32>(v, 0x12C))
        .fcom(x(g::<f32>(0x858C28).read()))
        .test_parity(0x05)
        && cull_zones_cam_no_rain() != 0
    {
        on = true;
        check_driver = true;
    }
    if !check_driver {
        return 0;
    }
    let d = rd::<GPtr<u8>>(v, 0x460);
    if !d.is_null() {
        let t = rd::<i32>(d, 0x598);
        if (7..=16).contains(&t) && rd::<u8>(d, 0x20) & 1 != 0 && g::<u8>(0xC0BC5C).read() != 0 {
            return 0;
        }
    }
    b(on)
}

/// `CVehicle::SetupRender()` (0x6D64F0): cull mode 1, the custom plate for
/// automobiles; a pending remap TXD (+0x59A >= 0) is swapped in when loaded (old one released, new
/// one referenced, its first texture -> +0x59C, colour byte +0x434 = 1 unless bit 2 of `+0x42F`) or
/// requested (model 20000 + slot, flags 8); with none bit 2 of `+0x42F` is cleared. Then the model
/// colours (+0x434..+0x437), the light flags, the remap texture global 0xB4E47C and the editable
/// materials.
#[hook(addr = 0x6D64F0, conv = thiscall, name = "CVehicle::SetupRender", expect = "56 8B F1 0F BF 46 22", this = "CVehicle")]
pub fn setup_render(v: Veh) {
    let mi = mi_of(v);
    rw_render_state_set(0x14, 1);
    if rd::<u32>(v, 0x590) == 0 {
        custom_car_plate_before_rendering_start(v, mi);
    }
    let pending = rd::<i16>(v, 0x59A);
    if pending < 0 {
        wr::<u8>(v, 0x42F, rd::<u8>(v, 0x42F) & 0xFB);
    } else if g::<u8>(0x946750 + 20 * pending as usize).read() == 1 {
        if rd::<u32>(v, 0x59C) != 0 {
            let old = i32::from(rd::<i16>(v, 0x598));
            wr::<u32>(v, 0x59C, 0);
            txd_store_remove_ref(old);
        }
        let s = rd::<u16>(v, 0x59A);
        wr::<u16>(v, 0x598, s);
        wr::<u16>(v, 0x59A, 0xFFFF);
        txd_store_add_ref(i32::from(s as i16));
        let idx = i32::from(rd::<i16>(v, 0x598));
        let pool = g::<GPtr<u8>>(0xC8800C).read();
        let entry = if rd::<GPtr<u8>>(pool, 4).byte_add(idx as isize).read() & 0x80 != 0 {
            GPtr::<u8>::NULL
        } else {
            rd::<GPtr<u8>>(pool, 0).byte_add(idx as isize * 12)
        };
        let tex = get_first_texture(rd::<GPtr<u8>>(entry, 0));
        wr::<GPtr<u8>>(v, 0x59C, tex);
        if rd::<u8>(v, 0x42F) & 4 == 0 {
            wr::<u8>(v, 0x434, 1);
        }
    } else {
        streaming_request_model(i32::from(pending) + 20000, 8);
    }
    vmi_set_vehicle_colour(
        mi,
        u32::from(rd::<u8>(v, 0x434)),
        u32::from(rd::<u8>(v, 0x435)),
        u32::from(rd::<u8>(v, 0x436)),
        u32::from(rd::<u8>(v, 0x437)),
    );
    vmi_setup_light_flags(v);
    g::<u32>(0xB4E47C).write(rd::<u32>(v, 0x59C));
    vmi_set_editable_materials(rd::<GPtr<u8>>(v, 0x18));
}

/// `fld dword` + `fabs`.
fn xabs(v: f32) -> X87 {
    x(f32::from_bits(v.to_bits() & 0x7FFF_FFFF))
}

/// `sqrt(x*x + y*y)` of the move speed (+0x44, +0x48) at PC = 24.
fn speed_2d(v: Veh) -> X87 {
    let (sx, sy) = (x(rd::<f32>(v, 0x44)), x(rd::<f32>(v, 0x48)));
    sx.fmul(sx).fadd(sy.fmul(sy)).fsqrt()
}

/// `(x*x + y*y) + z*z` of the turn speed (+0x50..+0x58) compared with 0.0004: true when not above
/// (`test ah, 0x41` + `jne`; NaN -> true).
fn turn_small(v: Veh) -> bool {
    let (tx, ty, tz) = (
        x(rd::<f32>(v, 0x50)),
        x(rd::<f32>(v, 0x54)),
        x(rd::<f32>(v, 0x58)),
    );
    let s = tx.fmul(tx).fadd(ty.fmul(ty)).fadd(tz.fmul(tz));
    s.fcom(x(g::<f32>(0x858FC0).read())).test_any(0x41)
}

/// `CVehicle::CanPedStepOutCar(bool)` (0x6D1F30): upright-ish (`-0.1 <= up.z <= 0.1`): false when
/// `|moveSpeed.z| > 0.05` or the 2D speed > 0.01, else the turn-speed test. Otherwise (tilted, or NaN):
/// boats (subclass 5) -> true; with the argument only the turn-speed test; else false when the 2D speed
/// > 0.01, the turn-speed test when `|moveSpeed.z| <= 0.05`, false otherwise.
#[hook(addr = 0x6D1F30, conv = thiscall, name = "CVehicle::CanPedStepOutCar", expect = "83 EC 0C 8B 41 14", this = "CVehicle")]
pub fn can_ped_step_out_car(v: Veh, ignore_speed: u32) -> u8 {
    let z = x(rd::<f32>(rd::<GPtr<u8>>(v, 0x14), 0x28));
    let c = |a: usize| x(g::<f32>(a).read());
    let tilted = z.fcom(c(0x858B1C)).test_parity(0x41) || z.fcom(c(0x858EF4)).test_any(0x01);
    if !tilted {
        if !xabs(rd::<f32>(v, 0x4C)).fcom(c(0x858C28)).test_any(0x41) {
            return 0;
        }
        if !speed_2d(v).fcom(c(0x858C58)).test_any(0x41) {
            return 0;
        }
        return b(turn_small(v));
    }
    if rd::<u32>(v, 0x590) == 5 {
        return 1;
    }
    if ignore_speed as u8 != 0 {
        return b(turn_small(v));
    }
    if !speed_2d(v).fcom(c(0x858C58)).test_any(0x41) {
        return 0;
    }
    if xabs(rd::<f32>(v, 0x4C)).fcom(c(0x858C28)).test_any(0x41) {
        return b(turn_small(v));
    }
    0
}

orig! {
    /// `CPlayerInfo::AddHealth(int32)` (0x56EAB0)
    pub fn player_info_add_health(pi: GPtr<u8>, n: i32) = thiscall 0x56EAB0 => unported;
    /// `CStats::IncrementStat(eStats, float)` (0x55C180)
    pub fn stats_increment_stat(stat: u32, v: u32) = cdecl 0x55C180 => unported;
    /// `CPhysical::ApplyMoveForce(CVector)` (0x5429F0)
    pub fn physical_apply_move_force(p: GPtr<u8>, fx: u32, fy: u32, fz: u32) = thiscall 0x5429F0 => unported;
    /// `FUN_0065da70` (0x65DA70): `CPhysical::ApplyTurnForce(CVector force, CVector point)` as SetDriver
    /// calls it
    pub fn physical_apply_turn_force_65da70(p: GPtr<u8>, fx: u32, fy: u32, fz: u32, px: u32, py: u32, pz: u32) = thiscall 0x65DA70 => unported;
}

/// The driver's weight effect on the vehicle (`SetDriver`'s tail, `-0.02` at 0x85904C): bikes get
/// `ApplyMoveForce(up * (mass * -0.02))`, others the turn force `(0, 0, mass * -0.02)` at the 2D offset
/// `(ped.pos - vehicle.pos)` through 0x65DA70.
fn apply_occupant_weight(v: Veh, ped: GPtr<u8>) {
    let f = x(rd::<f32>(ped, 0x8C)).fmul_m(g::<f32>(0x85904C).read());
    if rd::<u32>(v, 0x590) == 9 {
        let m = rd::<GPtr<u8>>(v, 0x14);
        let fx = f.fmul_m(rd::<f32>(m, 0x20)).to_f32();
        let fy = f.fmul_m(rd::<f32>(m, 0x24)).to_f32();
        let fz = f.fmul_m(rd::<f32>(m, 0x28)).to_f32();
        physical_apply_move_force(v, fx.to_bits(), fy.to_bits(), fz.to_bits());
    } else {
        let (pm, vm) = (rd::<GPtr<u8>>(ped, 0x14), rd::<GPtr<u8>>(v, 0x14));
        let dy = x(rd::<f32>(pm, 0x34)).fsub_m(rd::<f32>(vm, 0x34)).to_f32();
        let dx = x(rd::<f32>(pm, 0x30)).fsub_m(rd::<f32>(vm, 0x30)).to_f32();
        physical_apply_turn_force_65da70(v, 0, 0, f.to_f32().to_bits(), dx.to_bits(), dy.to_bits(), 0);
    }
}

/// `CVehicle::SetDriver(CPed*)` (0x6D16A0): the driver reference is changed (old released, new
/// registered without a null check). With the freebies flag (bit 7 of `+0x428`) and the player as
/// driver the flag is cleared and: ambulance (416) +20 health; taxi/cabbie (420/438) +$12; enforcer
/// (427) armour raised to the max armour byte 0xB7CEE8; caddy (457) requests the golf club (333) for a
/// non-player or a player who wants it; hotdog (588) stat 0xF5 += 40; police cars 596..599 request the
/// shotgun (349) and set the flag again. Then the driver's weight is applied.
#[hook(addr = 0x6D16A0, conv = thiscall, name = "CVehicle::SetDriver", expect = "83 EC 0C 53 56", this = "CVehicle")]
pub fn set_driver(v: Veh, ped: GPtr<u8>) {
    let slot = v.byte_add(0x460);
    let old = rd::<GPtr<u8>>(v, 0x460);
    if !old.is_null() {
        entity_clean_up_old_reference(old, slot);
    }
    wr::<GPtr<u8>>(v, 0x460, ped);
    entity_register_reference(ped, slot);
    if rd::<u8>(v, 0x428) & 0x80 != 0 && ped == find_player_ped(-1) {
        wr::<u8>(v, 0x428, rd::<u8>(v, 0x428) & 0x7F);
        match i32::from(rd::<i16>(v, 0x22)) {
            416 => player_info_add_health(GPtr::from_addr(0xB7CD98), 0x14),
            420 | 438 => {
                let p = g::<i32>(0xB7CE50 + 0x190 * usize::from(g::<u8>(0xB7CD74).read()));
                p.write(p.read().wrapping_add(12));
            }
            427 => {
                let max = xi(i32::from(g::<u8>(0xB7CEE8).read()));
                let cur = x(rd::<f32>(ped, 0x548));
                let v2 = if !max.fcom(cur).test_any(0x41) { max } else { cur };
                wr::<f32>(ped, 0x548, v2.to_f32());
            }
            457 => {
                if ped_is_player(ped) == 0 || does_player_want_new_weapon(ped, 2, 1) != 0 {
                    streaming_request_model(0x14D, 2);
                }
            }
            588 => stats_increment_stat(0xF5, 0x4220_0000),
            596..=599 => {
                streaming_request_model(0x15D, 2);
                wr::<u8>(v, 0x428, rd::<u8>(v, 0x428) | 0x80);
            }
            _ => {}
        }
    }
    apply_occupant_weight(v, ped);
}

orig! {
    /// CRT `eh vector destructor iterator(void*, size, count, dtor)` (0x821E02, stdcall)
    pub fn eh_vector_destructor_iterator(p: u32, size: u32, n: u32, dtor: u32) = stdcall 0x821E02 => unported;
    /// `FUN_006d5dd0` (0x6D5DD0): the vehicle pool's slot claim for a handle (`CPool::CreateAtRef`)
    pub fn vehicle_pool_create_at_ref(pool: GPtr<u8>, handle: i32) = thiscall 0x6D5DD0 => unported;
    /// `CClumpModelInfo::GetFrameFromId(RpClump*, int32)` (0x4C53C0)
    pub fn clump_get_frame_from_id(clump: GPtr<u8>, id: u32) -> GPtr<u8> = cdecl 0x4C53C0 => unported;
    /// `CRope::Remove()` (0x556780)
    pub fn rope_remove(rope: GPtr<u8>) = thiscall 0x556780 => unported;
    /// `CCarCtrl::UpdateCarCount(CVehicle*, bool remove)` (0x424000)
    pub fn car_ctrl_update_car_count(v: GPtr<u8>, remove: u32) = cdecl 0x424000 => unported;
}

/// `FUN_006d0af0` (0x6D0AF0): AL = the dword at `+0x9EC` equals -1000.0's bits (0xC47A0000).
#[hook(addr = 0x6D0AF0, conv = thiscall, name = "FUN_006d0af0", expect = "81 B9 EC 09 00 00 00 00 7A C4")]
pub fn field_9ec_is_minus_1000(v: Veh) -> u8 {
    b(rd::<u32>(v, 0x9EC) == 0xC47A_0000)
}

/// `__dtor_CVehicle__m_aSpecialColModel` (0x856A30): the CRT vector destructor over the four
/// `CColModel`s at 0xC1CC78 (atexit handler).
#[hook(addr = 0x856A30, conv = cdecl, name = "__dtor_CVehicle__m_aSpecialColModel", expect = "68 00 F7 40 00")]
pub fn dtor_special_col_model() {
    eh_vector_destructor_iterator(0xC1CC78, 0x30, 4, 0x40F700);
}

/// `thunk_FUN_01567620` (0x41BDD0, .HOODLUM body 0x1567620; a `CVehicle` flag setter): with bit 1 of
/// `+0x42D` bit 4 of `+0x428` is cleared, else it becomes bit 0 of the argument.
#[hook(addr = 0x41BDD0, conv = thiscall, name = "thunk_FUN_01567620", expect = "E9 4B B8 14 01")]
pub fn set_flag_428_bit4(v: Veh, on: u32) {
    let f = rd::<u8>(v, 0x428);
    if rd::<u8>(v, 0x42D) & 2 != 0 {
        wr::<u8>(v, 0x428, f & 0xEF);
    } else {
        wr::<u8>(v, 0x428, f ^ (((on as u8) << 4 ^ f) & 0x10));
    }
}

/// `FUN_006e2d60` (0x6E2D60, `CVehicle::operator new(uint32, int32 handle)`): claims the pool slot of
/// the handle (0x6D5DD0) and returns `objects + (handle >> 8) * 0xA18` (address computed first).
#[hook(addr = 0x6E2D60, conv = cdecl, name = "FUN_006e2d60", expect = "8B 44 24 08 8B 0D 94 44 B7 00")]
pub fn operator_new_at(_size: u32, handle: i32) -> GPtr<u8> {
    let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let p = rd::<GPtr<u8>>(pool, 0).byte_add(((handle >> 8).wrapping_mul(0xA18)) as isize);
    vehicle_pool_create_at_ref(pool, handle);
    p
}

/// `FUN_006e2dd0` (0x6E2DD0, `CVehicle::operator delete(void*, int32)`): the same inlined pool delete
/// as 0x6E2D90.
#[hook(addr = 0x6E2DD0, conv = cdecl, name = "FUN_006e2dd0", expect = "8B 4C 24 04 56")]
pub fn operator_delete_at(p: GPtr<u8>, _handle: i32) {
    operator_delete(p);
}

/// `CVehicle::CarHasRoof()` (0x6D25D0): false only with handling model flag 0x800 and both `+0x438`
/// and `+0x439` non-zero.
#[hook(addr = 0x6D25D0, conv = thiscall, name = "CVehicle::CarHasRoof", expect = "8B 81 84 03 00 00", this = "CVehicle")]
pub fn car_has_roof(v: Veh) -> u8 {
    let fl = rd::<u32>(rd::<GPtr<u8>>(v, 0x384), 0xCC);
    b(!(fl & 0x800 != 0 && rd::<u8>(v, 0x438) != 0 && rd::<u8>(v, 0x439) != 0))
}

/// `CVehicle::SetWindowOpenFlag(uint8 nodeId)` (0x6D3080): the node's atomics get flag 0x1000
/// (`SetAtomicFlagCB`).
#[hook(addr = 0x6D3080, conv = thiscall, name = "CVehicle::SetWindowOpenFlag", expect = "0F B6 44 24 04", this = "CVehicle")]
pub fn set_window_open_flag(v: Veh, node: u32) {
    let f = clump_get_frame_from_id(rd::<GPtr<u8>>(v, 0x18), node & 0xFF);
    if !f.is_null() {
        rw_frame_for_all_objects(f, 0x4C7B90, 0x1000);
    }
}

/// `CVehicle::ClearWindowOpenFlag(uint8 nodeId)` (0x6D30B0): flag 0x1000 cleared (`ClearAtomicFlagCB`).
#[hook(addr = 0x6D30B0, conv = thiscall, name = "CVehicle::ClearWindowOpenFlag", expect = "0F B6 44 24 04", this = "CVehicle")]
pub fn clear_window_open_flag(v: Veh, node: u32) {
    let f = clump_get_frame_from_id(rd::<GPtr<u8>>(v, 0x18), node & 0xFF);
    if !f.is_null() {
        rw_frame_for_all_objects(f, 0x4C7BB0, 0x1000);
    }
}

/// `CVehicle::GetPlaneNumGuns()` (0x6D3F30): 425, 447, 464, 487, 497, 548 -> 1; 476 -> 6; 520, 576 -> 2;
/// else 0.
#[hook(addr = 0x6D3F30, conv = thiscall, name = "CVehicle::GetPlaneNumGuns", expect = "0F BF 41 22 05 57 FE FF FF", this = "CVehicle", preserve = "ecx,edx")]
pub fn get_plane_num_guns(v: Veh) -> u32 {
    match i32::from(rd::<i16>(v, 0x22)) {
        425 | 447 | 464 | 487 | 497 | 548 => 1,
        476 => 6,
        520 | 576 => 2,
        _ => 0,
    }
}

/// `CVehicle::RemoveWinch()` (0x6D3C70): the rope with id `this + 0xA320` (as the bytes compute it) is
/// removed; the winch type bits (5, 6 of `+0x4A8`) are cleared.
#[hook(addr = 0x6D3C70, conv = thiscall, name = "CVehicle::RemoveWinch", expect = "56 8B F1 8D 86 20 A3 00 00", this = "CVehicle")]
pub fn remove_winch(v: Veh) {
    let r = ropes_find_rope((v.addr() as u32).wrapping_add(0xA320));
    if r >= 0 {
        rope_remove(g::<u8>(0xB768B8).byte_add(r as isize * 0x328));
    }
    wr::<u8>(v, 0x4A8, rd::<u8>(v, 0x4A8) & 0x9F);
}

/// `CVehicle::IsVehicleNormal()` (0x6D22F0): a driver, no passengers, status not 5, and the model's
/// `+0x4D` byte not 0xFF. AL only.
#[hook(addr = 0x6D22F0, conv = thiscall, name = "CVehicle::IsVehicleNormal", expect = "8B 91 60 04 00 00", this = "CVehicle")]
pub fn is_vehicle_normal(v: Veh) -> u8 {
    b(rd::<u32>(v, 0x460) != 0
        && rd::<u8>(v, 0x484) == 0
        && rd::<u8>(v, 0x36) & 0xF8 != 0x28
        && rd::<u8>(mi_of(v), 0x4D) != 0xFF)
}

/// `CVehicle::SetVehicleAtomicVisibilityCB(RwFrame*, void*)` (0x6D26D0): recurses over the frame's
/// objects and children; returns the frame.
#[hook(addr = 0x6D26D0, conv = cdecl, name = "CVehicle::SetVehicleAtomicVisibilityCB-Frame", expect = "56 8B 74 24 08")]
pub fn set_vehicle_atomic_visibility_cb_frame(frame: GPtr<u8>, data: u32) -> GPtr<u8> {
    rw_frame_for_all_objects(frame, 0x6D2690, data);
    rw_frame_for_all_children(frame, 0x6D26D0, data);
    frame
}

/// `CVehicle::RemoveObjectsCB(RwFrame*, void*)` (0x6D3420): recurses with `RemoveObjectsCB-Object`
/// over the objects and itself over the children; returns the frame.
#[hook(addr = 0x6D3420, conv = cdecl, name = "CVehicle::RemoveObjectsCB-Frame", expect = "56 8B 74 24 08")]
pub fn remove_objects_cb_frame(frame: GPtr<u8>, data: u32) -> GPtr<u8> {
    rw_frame_for_all_objects(frame, 0x6D33B0, data);
    rw_frame_for_all_children(frame, 0x6D3420, data);
    frame
}

/// `CVehicle::RemoveAllUpgrades()` (0x6D3AB0): every upgrade atomic removed (`RemoveAllUpgradesCB`),
/// the 15 upgrade slots (+0x43A) = -1.
#[hook(addr = 0x6D3AB0, conv = thiscall, name = "CVehicle::RemoveAllUpgrades", expect = "56 57 8B F1 8B 46 18", this = "CVehicle")]
pub fn remove_all_upgrades(v: Veh) {
    rp_clump_for_all_atomics(rd::<GPtr<u8>>(v, 0x18), 0x6D34D0, 0);
    for k in 0..7 {
        wr::<u32>(v, 0x43A + 4 * k, 0xFFFF_FFFF);
    }
    wr::<u16>(v, 0x456, 0xFFFF);
}

/// `CVehicle::SetRemapTexDictionary(int32)` (0x6D0BC0): a change to -1 releases the current remap TXD
/// (+0x59C = null, `RemoveRef`, +0x598 = -1); any change sets the pending slot +0x59A.
#[hook(addr = 0x6D0BC0, conv = thiscall, name = "CVehicle::SetRemapTexDictionary", expect = "56 8B F1 0F BF 86 98 05 00 00", this = "CVehicle")]
pub fn set_remap_tex_dictionary(v: Veh, txd: i32) {
    let cur = i32::from(rd::<i16>(v, 0x598));
    if txd == cur {
        return;
    }
    if txd == -1 {
        wr::<u32>(v, 0x59C, 0);
        txd_store_remove_ref(cur);
        wr::<u16>(v, 0x598, txd as u16);
    }
    wr::<u16>(v, 0x59A, txd as u16);
}

/// `CVehicle::SetVehicleCreatedBy(int32)` (0x6D5D70): on a change the car counts are updated around it
/// (`UpdateCarCount(this, true)`, store, `UpdateCarCount(this, false)`).
#[hook(addr = 0x6D5D70, conv = thiscall, name = "CVehicle::SetVehicleCreatedBy", expect = "53 8B 5C 24 08", this = "CVehicle")]
pub fn set_vehicle_created_by(v: Veh, by: u32) {
    if u32::from(rd::<u8>(v, 0x4A4)) == by {
        return;
    }
    car_ctrl_update_car_count(v, 1);
    wr::<u8>(v, 0x4A4, by as u8);
    car_ctrl_update_car_count(v, 0);
}

orig! {
    /// `RpAtomicClone(RpAtomic*)` (0x749E60)
    pub fn rp_atomic_clone(a: GPtr<u8>) -> GPtr<u8> = cdecl 0x749E60 => unported;
    /// `RpClumpAddAtomic(RpClump*, RpAtomic*)` (0x74A490)
    pub fn rp_clump_add_atomic(c: GPtr<u8>, a: GPtr<u8>) -> GPtr<u8> = cdecl 0x74A490 => unported;
    /// `RpAtomicSetFrame(RpAtomic*, RwFrame*)` (0x74BF20)
    pub fn rp_atomic_set_frame(a: GPtr<u8>, f: GPtr<u8>) -> GPtr<u8> = cdecl 0x74BF20 => unported;
    /// `CVisibilityPlugins::GetModelInfoIndex(RpAtomic*)` (0x732250)
    pub fn vis_get_model_info_index(a: GPtr<u8>) -> i32 = cdecl 0x732250 => unported;
    /// `CPopulation::AddPedInCar(CVehicle*, bool driver, int32, int32, bool, bool)` (0x613A00)
    pub fn population_add_ped_in_car(v: GPtr<u8>, driver: u32, a: u32, b: u32, c: u32, d: u32) -> GPtr<u8> = cdecl 0x613A00 => unported;
    /// `CVehicleModelInfo::GetNumRemaps()` (0x4C86B0)
    pub fn vmi_get_num_remaps(mi: GPtr<u8>) -> i32 = thiscall 0x4C86B0 => unported;
}

/// `CVehicle::IsPassenger(int32 modelIndex)` (0x6D1C00): a passenger slot holds a ped of that model.
#[hook(addr = 0x6D1C00, conv = thiscall, name = "CVehicle::IsPassenger-ModelID", expect = "56 8B 74 24 08", this = "CVehicle")]
pub fn is_passenger_model(v: Veh, model: i32) -> u8 {
    b((0..8).any(|k| {
        let p = rd::<GPtr<u8>>(v, 0x464 + 4 * k);
        !p.is_null() && i32::from(rd::<i16>(p, 0x22)) == model
    }))
}

/// `CVehicle::CopyObjectsCB(RwObject*, void* frame)` (0x6D3450): an atomic (type 1) is cloned into the
/// clump at 0xC1CB58 and attached to the frame; returns the object.
#[hook(addr = 0x6D3450, conv = cdecl, name = "CVehicle::CopyObjectsCB", expect = "57 8B 7C 24 08")]
pub fn copy_objects_cb(obj: GPtr<u8>, frame: GPtr<u8>) -> GPtr<u8> {
    if rd::<u8>(obj, 0) == 1 {
        let a = rp_atomic_clone(obj);
        rp_clump_add_atomic(g::<GPtr<u8>>(0xC1CB58).read(), a);
        rp_atomic_set_frame(a, frame);
    }
    obj
}

/// `CVehicle::GetFiringRateMultiplier()` (0x6D4090): vehicle type 4 -> byte `+0x9D8` * 0.0625, type 3
/// -> byte `+0xA10` * 0.0625, else 1.0 (ST0).
#[hook(addr = 0x6D4090, conv = thiscall, name = "CVehicle::GetFiringRateMultiplier", expect = "51 8B 81 94 05 00 00", this = "CVehicle")]
pub fn get_firing_rate_multiplier(v: Veh) -> X87 {
    let k = g::<f32>(0x858620).read();
    match rd::<u32>(v, 0x594) {
        4 => xi(i32::from(rd::<u8>(v, 0x9D8))).fmul_m(k),
        3 => xi(i32::from(rd::<u8>(v, 0xA10))).fmul_m(k),
        _ => x(g::<f32>(0x858624).read()),
    }
}

/// `CVehicle::SetFiringRateMultiplier(float)` (0x6D4010): clamped to 0 (below 0) and 15.9375 (above
/// 0x871E64), times 16.0 (0x8599D0), `ftol` -> byte `+0x9D8` (type 4) or `+0xA10` (type 3).
#[hook(addr = 0x6D4010, conv = thiscall, name = "CVehicle::SetFiringRateMultiplier", expect = "D9 44 24 04 56", this = "CVehicle")]
pub fn set_firing_rate_multiplier(v: Veh, m: f32) {
    let mut m = m;
    if !x(m).fcom(x(g::<f32>(0x858B50).read())).test_parity(0x05) {
        m = 0.0;
    } else if !x(m).fcom(x(g::<f32>(0x871E64).read())).test_any(0x41) {
        m = f32::from_bits(0x417F_0000);
    }
    let off = match rd::<u32>(v, 0x594) {
        4 => 0x9D8,
        3 => 0xA10,
        _ => return,
    };
    let n = ftol32(x(m).fmul_m(g::<f32>(0x8599D0).read()));
    wr::<u8>(v, off, n as u8);
}

/// `CVehicle::FindReplacementUpgradeCB(RpAtomic*, void* data)` (0x6D3490): an atomic (type 1) whose id
/// lacks flag 0x800 and that has a model info index is stored at `data + 4` and stops the iteration
/// (returns null); else returns the atomic.
#[hook(addr = 0x6D3490, conv = cdecl, name = "CVehicle::FindReplacementUpgradeCB", expect = "56 8B 74 24 08")]
pub fn find_replacement_upgrade_cb(a: GPtr<u8>, data: GPtr<u8>) -> GPtr<u8> {
    if rd::<u8>(a, 0) == 1 && vis_get_atomic_id(a) & 0x800 == 0 && vis_get_model_info_index(a) != -1 {
        wr::<GPtr<u8>>(data, 4, a);
        return GPtr::NULL;
    }
    a
}

/// `CVehicle::FindUpgradeCB(RpAtomic*, void* data)` (0x6D3370): an atomic with id flag 0x800 whose model
/// info's upgrade slot (`(+0x12 >> 10) & 0x1F`) equals `*data` is stored at `data + 4` and stops the
/// iteration; else returns the atomic.
#[hook(addr = 0x6D3370, conv = cdecl, name = "CVehicle::FindUpgradeCB", expect = "56 8B 74 24 08")]
pub fn find_upgrade_cb(a: GPtr<u8>, data: GPtr<u8>) -> GPtr<u8> {
    if vis_get_atomic_id(a) & 0x800 != 0 {
        let mi = vis_get_model_info(a);
        let slot = (u32::from(rd::<u16>(mi, 0x12)) >> 10) & 0x1F;
        if rd::<u32>(data, 0) == slot {
            wr::<GPtr<u8>>(data, 4, a);
            return GPtr::NULL;
        }
    }
    a
}

/// `CVehicle::SetUpDriver(int32, bool, bool)` (0x6D1A50): an existing driver is returned; otherwise a
/// random-created vehicle (`+0x4A4` == 1) gets `CPopulation::AddPedInCar(this, true, a, 0, b, c)` and its
/// driver is returned; else null.
#[hook(addr = 0x6D1A50, conv = thiscall, name = "CVehicle::SetUpDriver", expect = "56 8B F1 8B 86 60 04 00 00", this = "CVehicle")]
pub fn set_up_driver(v: Veh, a: u32, b2: u32, c: u32) -> GPtr<u8> {
    let d = rd::<GPtr<u8>>(v, 0x460);
    if !d.is_null() {
        return d;
    }
    if rd::<u8>(v, 0x4A4) != 1 {
        return GPtr::NULL;
    }
    population_add_ped_in_car(v, 1, a, 0, b2, c);
    rd::<GPtr<u8>>(v, 0x460)
}

/// `CVehicle::GetReplacementUpgrade(int32 nodeId)` (0x6D3A50): the model index of the replacement atomic
/// under the node (`FindReplacementUpgradeCB`), or -1.
#[hook(addr = 0x6D3A50, conv = thiscall, name = "CVehicle::GetReplacementUpgrade", expect = "83 EC 08 8B 41 18", this = "CVehicle")]
pub fn get_replacement_upgrade(v: Veh, node: u32) -> i32 {
    let f = clump_get_frame_from_id(rd::<GPtr<u8>>(v, 0x18), node);
    let mut data = [node, 0u32];
    rw_frame_for_all_objects(f, 0x6D3490, GPtr::from_raw(&raw mut data).addr() as u32);
    if data[1] == 0 {
        return -1;
    }
    vis_get_model_info_index(GPtr::from_addr(data[1] as usize))
}

/// `CVehicle::GetRemapIndex()` (0x6D0B70): the index of the current remap TXD (+0x598) in the model's
/// remap list (`mi + 0x2FA`, `GetNumRemaps` re-called every iteration), or -1.
#[hook(addr = 0x6D0B70, conv = thiscall, name = "CVehicle::GetRemapIndex", expect = "53 8B D9 0F BF 43 22", this = "CVehicle")]
pub fn get_remap_index(v: Veh) -> i32 {
    let mi = mi_of(v);
    let mut i = 0i32;
    while i < vmi_get_num_remaps(mi) {
        let k = i32::from(i as i16);
        if rd::<u16>(mi, 0x2FA + 2 * k as isize) == rd::<u16>(v, 0x598) {
            return i;
        }
        i += 1;
    }
    -1
}

/// `CVehicle::GetSpareHasslePosId()` (0x6D3AE0): slots 6 for vehicle type 0, 2 for types 2/9/10, else
/// none; the first slot whose bit in `+0x51B` is clear, else -1.
#[hook(addr = 0x6D3AE0, conv = thiscall, name = "CVehicle::GetSpareHasslePosId", expect = "8B 81 94 05 00 00", this = "CVehicle")]
pub fn get_spare_hassle_pos_id(v: Veh) -> i32 {
    let n = match rd::<u32>(v, 0x594) {
        0 => 6,
        2 | 9 | 10 => 2,
        _ => 0,
    };
    let bits = rd::<u8>(v, 0x51B);
    (0..n).find(|&k| bits & (1u8 << k) == 0).unwrap_or(-1)
}

orig! {
    /// `CGarages::TriggerMessage(const char*, int16, uint16, int16)` (0x447B80)
    pub fn garages_trigger_message(msg: u32, a: i32, time: u32, b: i32) = cdecl 0x447B80 => unported;
    /// `CFire::Extinguish()` (0x5393F0)
    pub fn fire_extinguish(f: GPtr<u8>) = thiscall 0x5393F0 => unported;
    /// `CDamageManager::GetEngineStatus()` (0x6C22C0)
    pub fn dm_get_engine_status(dm: GPtr<u8>) -> u32 = thiscall 0x6C22C0 => unported;
    /// `CDamageManager::SetEngineStatus(uint32)` (0x6C22A0)
    pub fn dm_set_engine_status(dm: GPtr<u8>, s: u32) = thiscall 0x6C22A0 => unported;
    /// `CVisibilityPlugins::GetFrameHierarchyId(RwFrame*)` (0x732A20)
    pub fn vis_get_frame_hierarchy_id(f: GPtr<u8>) -> i32 = cdecl 0x732A20 => unported;
    /// `RwFrameDestroy(RwFrame*)` (0x7F05A0)
    pub fn rw_frame_destroy(f: GPtr<u8>) -> i32 = cdecl 0x7F05A0 => unported;
    /// `CVehicle::AddVehicleUpgrade(int32)` (0x6E3290)
    pub fn add_vehicle_upgrade(v: GPtr<u8>, model: i32) = thiscall 0x6E3290 => unported;
}

/// `CVehicle::GetPlaneGunsAutoAimAngle()` (0x6D3E00): the per-model constant (0x871E00..0x871E24) for
/// 425, 447, 464, 476, 487, 497, 520, 548, 564, 576; else 0.0 (ST0).
#[hook(addr = 0x6D3E00, conv = thiscall, name = "CVehicle::GetPlaneGunsAutoAimAngle", expect = "0F BF 41 22 05 57 FE FF FF", this = "CVehicle")]
pub fn get_plane_guns_auto_aim_angle(v: Veh) -> X87 {
    let a = match i32::from(rd::<i16>(v, 0x22)) {
        425 => 0x871E00,
        447 => 0x871E04,
        464 => 0x871E24,
        476 => 0x871E08,
        487 => 0x871E14,
        497 => 0x871E18,
        520 => 0x871E0C,
        548 => 0x871E1C,
        564 => 0x871E20,
        576 => 0x871E10,
        _ => 0x858B50,
    };
    x(g::<f32>(a).read())
}

/// `CVehicle::ActivateBomb()` (0x6D24F0): bomb type (+0x4A8 & 7) 1 becomes 5 with the timer at 7000 and
/// the player as owner (no reference registered); type 2 becomes 5 at once (`& 0xFD | 5`); both show
/// the garage message at 0x87126C for 3000 ms.
#[hook(addr = 0x6D24F0, conv = thiscall, name = "CVehicle::ActivateBomb", expect = "56 8B F1 8A 86 A8 04 00 00", this = "CVehicle")]
pub fn activate_bomb(v: Veh) {
    let f = rd::<u8>(v, 0x4A8);
    match f & 7 {
        1 => {
            wr::<u8>(v, 0x4A8, (f & 0xFC) | 4);
            wr::<u16>(v, 0x4DE, 7000);
            let p = find_player_ped(-1);
            wr::<GPtr<u8>>(v, 0x4E0, p);
            garages_trigger_message(0x87126C, -1, 3000, -1);
        }
        2 => {
            wr::<u8>(v, 0x4A8, (f & 0xFD) | 5);
            garages_trigger_message(0x87126C, -1, 3000, -1);
        }
        _ => {}
    }
}

/// `CVehicle::ExtinguishCarFire()` (0x6D2460): unless wrecked (status 5) health (+0x4C0) is raised to
/// 300 (kept when above; NaN -> 300); a burning fire (+0x490) loses bit 1 of its flags and is
/// extinguished; automobiles with engine status >= 225 get 215, and `+0x8E4` = 0.
#[hook(addr = 0x6D2460, conv = thiscall, name = "CVehicle::ExtinguishCarFire", expect = "56 8B F1 8A 46 36", this = "CVehicle")]
pub fn extinguish_car_fire(v: Veh) {
    if rd::<u8>(v, 0x36) & 0xF8 != 0x28 {
        let h = x(rd::<f32>(v, 0x4C0));
        let c = x(g::<f32>(0x858FD8).read());
        wr::<f32>(v, 0x4C0, if h.fcom(c).test_any(0x41) { c } else { h }.to_f32());
    }
    let fire = rd::<GPtr<u8>>(v, 0x490);
    if !fire.is_null() {
        fire.write(fire.read() & 0xFD);
        fire_extinguish(rd::<GPtr<u8>>(v, 0x490));
        wr::<u32>(v, 0x490, 0);
    }
    if rd::<u32>(v, 0x590) == 0 {
        let dm = v.byte_add(0x5A0);
        if dm_get_engine_status(dm) >= 0xE1 {
            dm_set_engine_status(dm, 0xD7);
        }
        wr::<u32>(v, 0x8E4, 0);
    }
}

/// `CVehicle::RemoveReplacementUpgrade(int32 nodeId)` (0x6D39E0): the node's objects are removed
/// (`RemoveObjectsCB`), then the model's original atomics of that node are copied back
/// (`CopyObjectsCB`, clump global 0xC1CB58 = this clump). The original passes its own argument slot
/// (zeroed) as the callbacks' data word; the port passes a zeroed local.
#[hook(addr = 0x6D39E0, conv = thiscall, name = "CVehicle::RemoveReplacementUpgrade", expect = "53 8B 5C 24 08", this = "CVehicle")]
pub fn remove_replacement_upgrade(v: Veh, node: u32) {
    let f = clump_get_frame_from_id(rd::<GPtr<u8>>(v, 0x18), node);
    let mut slot = 0u32;
    let sp = GPtr::from_raw(&raw mut slot).addr() as u32;
    rw_frame_for_all_objects(f, 0x6D33B0, sp);
    rw_frame_for_all_children(f, 0x6D3420, sp);
    let f2 = clump_get_frame_from_id(rd::<GPtr<u8>>(mi_of(v), 0x1C), node);
    g::<u32>(0xC1CB58).write(rd::<u32>(v, 0x18));
    rw_frame_for_all_objects(f2, 0x6D3450, f.addr() as u32);
}

/// `CVehicle::SetRemap(int32)` (0x6D0C00, the remap read sits in a .HOODLUM fragment at 0x407645): -1
/// releases a loaded remap TXD (+0x59C = null, `RemoveRef`, +0x598 / +0x59A = -1); otherwise the TXD of
/// the model's remap list entry (`mi + 0x2FA`) becomes pending (+0x59A), and when it is -1 the current
/// one is released first.
#[hook(addr = 0x6D0C00, conv = thiscall, name = "CVehicle::SetRemap", expect = "8B 44 24 04 83 F8 FF", this = "CVehicle")]
pub fn set_remap(v: Veh, idx: i32) {
    if idx == -1 {
        let cur = rd::<i16>(v, 0x598);
        if cur != -1 {
            wr::<u32>(v, 0x59C, 0);
            txd_store_remove_ref(i32::from(cur));
            wr::<u16>(v, 0x598, 0xFFFF);
            wr::<u16>(v, 0x59A, 0xFFFF);
        }
        return;
    }
    let txd = i32::from(rd::<i16>(mi_of(v), 0x2FA + 2 * isize::from(idx as i16)));
    let cur = i32::from(rd::<i16>(v, 0x598));
    if txd == cur {
        return;
    }
    if txd == -1 {
        wr::<u32>(v, 0x59C, 0);
        txd_store_remove_ref(cur);
        wr::<u16>(v, 0x598, txd as u16);
    }
    wr::<u16>(v, 0x59A, txd as u16);
}

/// `CVehicle::RemoveObjectsCB(RwObject*, void* data)` (0x6D33B0): for an atomic: its id is stored at
/// `*data`; a non-upgrade atomic (no id flag 0x800) is removed from its clump and destroyed, its frame
/// destroyed unless it is part of the hierarchy, and its model info's reference released. Returns the
/// object.
#[hook(addr = 0x6D33B0, conv = cdecl, name = "CVehicle::RemoveObjectsCB-Object", expect = "53 56 8B 74 24 0C")]
pub fn remove_objects_cb_object(obj: GPtr<u8>, data: GPtr<u32>) -> GPtr<u8> {
    if rd::<u8>(obj, 0) == 1 {
        let id = vis_get_atomic_id(obj);
        data.write(id);
        if id & 0x800 == 0 {
            let mi = vis_get_model_info(obj);
            let frame = rd::<GPtr<u8>>(obj, 4);
            rp_clump_remove_atomic(rd::<GPtr<u8>>(obj, 0x3C), obj);
            rp_atomic_destroy(obj);
            if vis_get_frame_hierarchy_id(frame) == 0 {
                rw_frame_destroy(frame);
            }
            if !mi.is_null() {
                base_model_info_remove_ref(mi);
            }
        }
    }
    obj
}

/// `CVehicle::CanPedLeanOut(CPed*)` (0x6D5CF0, handling read in a .HOODLUM fragment at 0x4011CD): anim
/// group (handling `+0xDE`) 13: only the driver or the first passenger (EAX 0/1); groups 14..18: false;
/// vehicle types 3, 4, 5, 6: false; else true (AL).
#[hook(addr = 0x6D5CF0, conv = thiscall, name = "CVehicle::CanPedLeanOut", expect = "90 E9 D7 B4 D2 FF", this = "CVehicle")]
pub fn can_ped_lean_out(v: Veh, ped: GPtr<u8>) -> u8 {
    let grp = u32::from(rd::<u8>(rd::<GPtr<u8>>(v, 0x384), 0xDE)) + 0x58;
    if grp == 0x65 {
        return b(ped == rd::<GPtr<u8>>(v, 0x464) || ped == rd::<GPtr<u8>>(v, 0x460));
    }
    if (0x66..=0x6A).contains(&grp) || matches!(rd::<u32>(v, 0x594), 3..=6) {
        return 0;
    }
    1
}

/// `GetTowHitchPos` / `GetTowBarPos` body: `out = matrix * (0, colModel bbox y (+off) + sign * 1.0, 0)`.
fn tow_pos(v: Veh, out: GPtr<u8>, off: isize, add: bool) {
    wr::<u32>(out, 0, 0);
    let cm = rd::<GPtr<u8>>(mi_of(v), 0x14);
    wr::<u32>(out, 8, 0);
    let y = x(rd::<f32>(cm, off));
    let one = g::<f32>(0x858624).read();
    wr::<f32>(out, 4, if add { y.fadd_m(one) } else { y.fsub_m(one) }.to_f32());
    let mut tmp = [0u32; 3];
    let tp = GPtr::from_raw(&raw mut tmp).cast::<u8>();
    let r = matrix_mul_vec(tp, rd::<GPtr<u8>>(v, 0x14), out);
    for k in 0..3 {
        wr::<u32>(out, 4 * k, rd::<u32>(r, 4 * k));
    }
}

/// `CVehicle::GetTowHitchPos(CVector& out, bool, CVehicle*)` (0x6DFB70): with the flag, the point 1.0
/// in front of the bounding box (`max.y + 1`) in world space; AL = the flag.
#[hook(addr = 0x6DFB70, conv = thiscall, name = "CVehicle::GetTowHitchPos", expect = "8A 44 24 08 83 EC 0C", this = "CVehicle")]
pub fn get_tow_hitch_pos(v: Veh, out: GPtr<u8>, on: u32, _other: GPtr<u8>) -> u8 {
    if on as u8 == 0 {
        return 0;
    }
    tow_pos(v, out, 0x10, true);
    1
}

/// `CVehicle::GetTowBarPos(CVector& out, bool, CVehicle*)` (0x6DFBE0): the point 1.0 behind the bounding
/// box (`min.y - 1`).
#[hook(addr = 0x6DFBE0, conv = thiscall, name = "CVehicle::GetTowBarPos", expect = "8A 44 24 08 83 EC 0C", this = "CVehicle")]
pub fn get_tow_bar_pos(v: Veh, out: GPtr<u8>, on: u32, _other: GPtr<u8>) -> u8 {
    if on as u8 == 0 {
        return 0;
    }
    tow_pos(v, out, 4, false);
    1
}

/// `CVehicle::SetupUpgradesAfterLoad()` (0x6E3400): every stored upgrade (+0x43A, 15 words) is cleared
/// and re-added (`AddVehicleUpgrade`).
#[hook(addr = 0x6E3400, conv = thiscall, name = "CVehicle::SetupUpgradesAfterLoad", expect = "53 56 57 8B F9", this = "CVehicle")]
pub fn setup_upgrades_after_load(v: Veh) {
    for k in 0..15 {
        let w = rd::<u16>(v, 0x43A + 2 * k);
        if w != 0xFFFF {
            wr::<u16>(v, 0x43A + 2 * k, 0xFFFF);
            add_vehicle_upgrade(v, i32::from(w as i16));
        }
    }
}

orig! {
    /// `SurfaceInfos_c::IsWater(uint32)` (0x55E710)
    pub fn surface_is_water(s: GPtr<u8>, surf: u32) -> u8 = thiscall 0x55E710 => unported;
    /// `SurfaceInfos_c::MakesCarDirty(uint32)` (0x55E8D0)
    pub fn surface_makes_car_dirty(s: GPtr<u8>, surf: u32) -> u8 = thiscall 0x55E8D0 => unported;
    /// `SurfaceInfos_c::MakesCarClean(uint32)` (0x55E8F0)
    pub fn surface_makes_car_clean(s: GPtr<u8>, surf: u32) -> u8 = thiscall 0x55E8F0 => unported;
    /// virtual `CVehicle::IsUpsideDown`-style check at vtable +0xE4 (one bool argument)
    pub fn vslot_e4(v: GPtr<u8>, a: u32) -> u8 = thiscall vslot 0xE4 => unported;
    /// `CVehicle::AddPassenger(CPed*)` (0x6D13A0)
    pub fn add_passenger_auto(v: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x6D13A0 => unported;
    /// `CPhysical::ApplyTurnForce(CVector force, CVector point)` (0x542A50)
    pub fn physical_apply_turn_force(p: GPtr<u8>, fx: u32, fy: u32, fz: u32, px: u32, py: u32, pz: u32) = thiscall 0x542A50 => unported;
}

/// `g_surfaceInfos`.
const SURFACE_INFOS: usize = 0xB79538;

/// `CVehicle::MakeDirty(CColPoint&)` (0x6D2BF0): on water, or while the roads are wet (0xC81324 not
/// below 0x858CC4), the dirt level (+0x4B0) above 1.0 drops by `timeStep * 0.01`. Otherwise a dirty
/// surface above the speed 0x859934 raises it by `timeStep * k` (capped at 15.0) and a cleaning surface
/// below the speed 0x858CEC lowers it above the minimum by `timeStep * 0.02` (floored). x87 as the
/// original.
#[hook(addr = 0x6D2BF0, conv = thiscall, name = "CVehicle::MakeDirty", expect = "56 57 8B 7C 24 0C", this = "CVehicle")]
pub fn make_dirty(v: Veh, cp: GPtr<u8>) {
    let si = GPtr::<u8>::from_addr(SURFACE_INFOS);
    let c = |a: usize| g::<f32>(a).read();
    let ts = c(0xB7CB5C);
    let wet = surface_is_water(si, u32::from(rd::<u8>(cp, 0x23))) != 0
        || !x(c(0xC81324)).fcom(x(c(0x858CC4))).test_any(0x01);
    if wet {
        let d = rd::<f32>(v, 0x4B0);
        if !x(d).fcom(x(c(0x858624))).test_any(0x41) {
            wr::<f32>(v, 0x4B0, x(ts).fmul_m(c(0x858C58)).fsubr_m(d).to_f32());
        }
        return;
    }
    if surface_makes_car_dirty(si, u32::from(rd::<u8>(cp, 0x23))) != 0 {
        if speed_2d(v).fcom(x(c(0x859934))).test_any(0x41) {
            return;
        }
        let nv = x(ts).fmul_m(c(0x859CD8)).fadd_m(rd::<f32>(v, 0x4B0));
        let max = x(c(0x858B48));
        wr::<f32>(
            v,
            0x4B0,
            if max.fcom(nv).test_parity(0x05) { nv } else { max }.to_f32(),
        );
        return;
    }
    if surface_makes_car_clean(si, u32::from(rd::<u8>(cp, 0x23))) == 0 {
        return;
    }
    if speed_2d(v).fcom(x(c(0x858CEC))).test_any(0x41) {
        return;
    }
    let min = x(c(0x858B90));
    if x(rd::<f32>(v, 0x4B0)).fcom(min).test_any(0x41) {
        return;
    }
    let nv = x(ts).fmul_m(c(0x858B38)).fsubr_m(rd::<f32>(v, 0x4B0));
    wr::<f32>(
        v,
        0x4B0,
        if min.fcom(nv).test_any(0x41) { nv } else { min }.to_f32(),
    );
}

/// `x*x + y*y` of the move speed (no square root), PC = 24.
fn speed_2d_sq(v: Veh) -> X87 {
    let (sx, sy) = (x(rd::<f32>(v, 0x44)), x(rd::<f32>(v, 0x48)));
    sx.fmul(sx).fadd(sy.fmul(sy))
}

/// `CVehicle::CanPedJumpOutCar(CPed*)` (0x6D2030): bikes: only the rider or the first passenger, when
/// the squared 2D speed is at least 0x858CA8. Others: helis/planes (types 3/4) and automobiles tilted
/// (up.z < 0x858C24) and in contact during the last second (`+0x3C` > now - 1000) take the "slow"
/// branch: fast (>= 0.1) -> true; else unless virtual +0xE4(false) the turn speed is damped by
/// 0x858C20 and the move speed is either damped (when `timeStep^2 * k` < `0.01 * |v|^2`) or scaled by
/// `max(0, 1 - timeStep / |v| * k2)`; false. All others: true when `0.1 <= |v2d|^2 <= 0x858B8C`.
#[hook(addr = 0x6D2030, conv = thiscall, name = "CVehicle::CanPedJumpOutCar", expect = "56 8B F1 8B 8E 90 05 00 00", this = "CVehicle")]
pub fn can_ped_jump_out_car(v: Veh, ped: GPtr<u8>) -> u8 {
    let c = |a: usize| g::<f32>(a).read();
    let sub = rd::<u32>(v, 0x590);
    if sub == 9 {
        let p0 = rd::<GPtr<u8>>(v, 0x464);
        if !p0.is_null() && ped != p0 {
            return 0;
        }
        return b(speed_2d_sq(v).fcom(x(c(0x858CA8))).test_parity(0x05));
    }
    let s = speed_2d_sq(v);
    let ty = rd::<u32>(v, 0x594);
    let slow_branch = ty == 4
        || ty == 3
        || (sub == 0
            && !x(rd::<f32>(rd::<GPtr<u8>>(v, 0x14), 0x28))
                .fcom(x(c(0x858C24)))
                .test_parity(0x05)
            && rd::<u32>(v, 0x3C) > g::<u32>(0xB7CB84).read().wrapping_sub(1000));
    if !slow_branch {
        if !s.fcom(x(c(0x858B1C))).test_parity(0x05) {
            return 0;
        }
        return b(s.fcom(x(c(0x858B8C))).test_any(0x41));
    }
    if s.fcom(x(c(0x858B1C))).test_parity(0x05) {
        return 1;
    }
    if vslot_e4(v, 0) != 0 {
        return 0;
    }
    let damp = c(0x858C20);
    for off in [0x50, 0x54, 0x58] {
        wr::<f32>(v, off, x(rd::<f32>(v, off)).fmul_m(damp).to_f32());
    }
    let (mx, my, mz) = (
        x(rd::<f32>(v, 0x44)),
        x(rd::<f32>(v, 0x48)),
        x(rd::<f32>(v, 0x4C)),
    );
    let sum = mx.fmul(mx).fadd(my.fmul(my)).fadd(mz.fmul(mz));
    let a = x(c(0x858C58)).fmul(sum);
    let ts = c(0xB7CB5C);
    let bb = x(ts).fmul_m(ts).fmul_m(c(0x871E38));
    if !bb.fcom(a).test_parity(0x05) {
        for off in [0x44, 0x48, 0x4C] {
            wr::<f32>(v, off, x(rd::<f32>(v, off)).fmul_m(damp).to_f32());
        }
        return 0;
    }
    let mut r = sum.fsqrt().fdivr_m(ts).fmul_m(c(0x863C00)).fsubr_m(c(0x858624));
    let zero = x(c(0x858B50));
    if !zero.fcom(r).test_any(0x41) {
        r = zero;
    }
    for off in [0x44, 0x48, 0x4C] {
        wr::<f32>(v, off, r.fmul_m(rd::<f32>(v, off)).to_f32());
    }
    0
}

/// `CVehicle::AddPassenger(CPed*, uint8 seat)` (0x6D14D0): with bit 1 of `+0x429` the automatic-seat
/// overload decides; otherwise the passenger's weight is applied (bikes: turn force `up * (mass *
/// -0.02)` at `forward * -0.1`; others: 0x65DA70 with `(0, 0, mass * -0.2)` at the 2D offset), then the
/// seat (below the max passenger count, empty) is taken with a registered reference and the count
/// raised. AL.
#[hook(addr = 0x6D14D0, conv = thiscall, name = "CVehicle::AddPassenger-Fixed-Seat", expect = "83 EC 18 56 8B F1", this = "CVehicle")]
pub fn add_passenger_fixed_seat(v: Veh, ped: GPtr<u8>, seat: u32) -> u8 {
    if rd::<u8>(v, 0x429) & 2 != 0 {
        return add_passenger_auto(v, ped);
    }
    let m = rd::<GPtr<u8>>(v, 0x14);
    if rd::<u32>(v, 0x590) == 9 {
        let k = c01();
        let px = x(rd::<f32>(m, 0x10)).fmul_m(k).to_f32();
        let py = x(rd::<f32>(m, 0x14)).fmul_m(k).to_f32();
        let pz = x(rd::<f32>(m, 0x18)).fmul_m(k).to_f32();
        let f = x(rd::<f32>(ped, 0x8C)).fmul_m(g::<f32>(0x85904C).read());
        let fx = f.fmul_m(rd::<f32>(m, 0x20)).to_f32();
        let fy = f.fmul_m(rd::<f32>(m, 0x24)).to_f32();
        let fz = f.fmul_m(rd::<f32>(m, 0x28)).to_f32();
        physical_apply_turn_force(
            v,
            fx.to_bits(),
            fy.to_bits(),
            fz.to_bits(),
            px.to_bits(),
            py.to_bits(),
            pz.to_bits(),
        );
    } else {
        let pm = rd::<GPtr<u8>>(ped, 0x14);
        let dy = x(rd::<f32>(pm, 0x34)).fsub_m(rd::<f32>(m, 0x34)).to_f32();
        let dx = x(rd::<f32>(pm, 0x30)).fsub_m(rd::<f32>(m, 0x30)).to_f32();
        let f = x(rd::<f32>(ped, 0x8C)).fmul_m(g::<f32>(0x858FDC).read()).to_f32();
        physical_apply_turn_force_65da70(v, 0, 0, f.to_bits(), dx.to_bits(), dy.to_bits(), 0);
    }
    let s = seat as u8;
    if s >= rd::<u8>(v, 0x488) {
        return 0;
    }
    let off = 0x464 + 4 * isize::from(s);
    if rd::<u32>(v, off) != 0 {
        return 0;
    }
    wr::<GPtr<u8>>(v, off, ped);
    entity_register_reference(ped, v.byte_add(off));
    wr::<u8>(v, 0x484, rd::<u8>(v, 0x484).wrapping_add(1));
    1
}

/// The -0.1 constant at 0x858EF4.
fn c01() -> f32 {
    g::<f32>(0x858EF4).read()
}

orig! {
    /// `FUN_0059c810` (0x59C810): `out = Multiply3x3(vec, matrix)` (the transposed product; cdecl, returns
    /// `out`)
    pub fn multiply_3x3_vec_mat(out: GPtr<u8>, v: GPtr<u8>, m: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C810 => unported;
    /// `Multiply3x3(CVector* out, const CMatrix&, const CVector&)` (0x59C790)
    pub fn multiply_3x3(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C790 => unported;
}

/// CRT `_CIpow` (0x822130): `st1 ^ st0`.
fn ci_pow(base: X87, exp: f32) -> X87 {
    X87::from_f64_exact(sa_core::orig::x87_binary(0x822130, base.to_f64(), f64::from(exp)))
}

/// `CVehicle::ApplyBoatWaterResistance(tBoatHandlingData*, float depth)` (0x6D2740): `F = depth *
/// handling+0xAC * depth * mass * 0.001` (x30 for the skimmer), `k = 1 / |F * (d*d + 0.05) + 1|` with d
/// the forward speed; `p = pow(k * res, timeStep * 0.5)` per axis (CRT `_CIpow`); the move speed in
/// vehicle space is scaled by p, the turn force `forward * ((p.y - 1) * v.y * mass)` is applied at
/// `-up`, and a falling z speed is damped by `(1 - p.z) * 0.5 + p.z`, a rising one by p.z. The
/// original keeps k and the impulse in its own argument slot (the depth's); the port keeps them in
/// locals.
#[hook(addr = 0x6D2740, conv = thiscall, name = "CVehicle::ApplyBoatWaterResistance", expect = "D9 44 24 08 83 EC 28", this = "CVehicle")]
pub fn apply_boat_water_resistance(v: Veh, bh: GPtr<u8>, depth: f32) {
    let c = |a: usize| g::<f32>(a).read();
    let mut f = x(depth)
        .fmul_m(rd::<f32>(rd::<GPtr<u8>>(v, 0x384), 0xAC))
        .fmul_m(depth)
        .fmul_m(rd::<f32>(v, 0x8C))
        .fmul_m(c(0x858CDC));
    if rd::<u16>(v, 0x22) == 0x1CC {
        f = f.fmul_m(c(0x871DDC));
    }
    let m = rd::<GPtr<u8>>(v, 0x14);
    let d = x(rd::<f32>(v, 0x4C))
        .fmul_m(rd::<f32>(m, 0x18))
        .fadd(x(rd::<f32>(v, 0x48)).fmul_m(rd::<f32>(m, 0x14)))
        .fadd(x(rd::<f32>(v, 0x44)).fmul_m(rd::<f32>(m, 0x10)));
    let mut s = f.fmul(d.fmul(d).fadd_m(c(0x858C28))).fadd_m(c(0x858624));
    if !s.fcom(x(c(0x858B50))).test_parity(0x05) {
        s = s.fchs();
    }
    let k = x(c(0x858624)).fdiv(s).to_f32();
    let t = x(c(0xB7CB5C)).fmul_m(c(0x858B8C)).to_f32();
    let px = ci_pow(x(k).fmul_m(rd::<f32>(bh, 0x24)), t).to_f32();
    let py = ci_pow(x(k).fmul_m(rd::<f32>(bh, 0x28)), t).to_f32();
    let pz = ci_pow(x(k).fmul_m(rd::<f32>(bh, 0x2C)), t).to_f32();
    let ms = v.byte_add(0x44);
    let mut out = [0u32; 3];
    let op = GPtr::from_raw(&raw mut out).cast::<u8>();
    let r = multiply_3x3_vec_mat(op, ms, m);
    for k2 in 0..3 {
        wr::<u32>(ms, 4 * k2, rd::<u32>(r, 4 * k2));
    }
    wr::<f32>(v, 0x44, x(px).fmul_m(rd::<f32>(v, 0x44)).to_f32());
    let ny = x(py).fmul_m(rd::<f32>(v, 0x48));
    wr::<f32>(v, 0x48, ny.to_f32());
    wr::<f32>(v, 0x4C, x(pz).fmul_m(rd::<f32>(v, 0x4C)).to_f32());
    let imp = x(py).fsub_m(c(0x858624)).fmul(ny).to_f32();
    let m2 = rd::<GPtr<u8>>(v, 0x14);
    let r2 = multiply_3x3(op, m2, ms);
    for k2 in 0..3 {
        wr::<u32>(ms, 4 * k2, rd::<u32>(r2, 4 * k2));
    }
    let m3 = rd::<GPtr<u8>>(v, 0x14);
    let neg = c(0x858C1C);
    let dx = x(rd::<f32>(m3, 0x20)).fmul_m(neg).to_f32();
    let dy = x(rd::<f32>(m3, 0x24)).fmul_m(neg).to_f32();
    let dz = x(rd::<f32>(m3, 0x28)).fmul_m(neg).to_f32();
    let fm = x(imp).fmul_m(rd::<f32>(v, 0x8C));
    let fx = fm.fmul_m(rd::<f32>(m3, 0x10)).to_f32();
    let fy = fm.fmul_m(rd::<f32>(m3, 0x14)).to_f32();
    let fz = fm.fmul_m(rd::<f32>(m3, 0x18)).to_f32();
    physical_apply_turn_force(
        v,
        fx.to_bits(),
        fy.to_bits(),
        fz.to_bits(),
        dx.to_bits(),
        dy.to_bits(),
        dz.to_bits(),
    );
    let vz = rd::<f32>(v, 0x4C);
    if !x(vz).fcom(x(c(0x858B50))).test_any(0x41) {
        wr::<f32>(v, 0x4C, x(pz).fmul_m(vz).to_f32());
    } else {
        let half = X87::from_f64_exact(g::<f64>(0x859060).read());
        let damp = x(c(0x858624))
            .fsub_m(pz)
            .fmul(half)
            .fadd_m(pz)
            .fmul_m(rd::<f32>(v, 0x4C));
        wr::<f32>(v, 0x4C, damp.to_f32());
    }
}

orig! {
    /// `CMatrix::Attach(RwMatrix*, bool)` (0x59BD10)
    pub fn matrix_attach(m: GPtr<u8>, rw: GPtr<u8>, owns: u32) = thiscall 0x59BD10 => unported;
    /// `CMatrix::SetRotateX(float)` (0x59B060)
    pub fn matrix_set_rotate_x(m: GPtr<u8>, a: f32) = thiscall 0x59B060 => unported;
    /// `CMatrix::SetRotateY(float)` (0x59B0A0)
    pub fn matrix_set_rotate_y(m: GPtr<u8>, a: f32) = thiscall 0x59B0A0 => unported;
    /// `CMatrix::SetRotateZ(float)` (0x59B0E0)
    pub fn matrix_set_rotate_z(m: GPtr<u8>, a: f32) = thiscall 0x59B0E0 => unported;
    /// `CMatrix::RotateX(float)` (0x59B1E0)
    pub fn matrix_rotate_x(m: GPtr<u8>, a: f32) = thiscall 0x59B1E0 => unported;
    /// `CMatrix::RotateY(float)` (0x59B2C0)
    pub fn matrix_rotate_y(m: GPtr<u8>, a: f32) = thiscall 0x59B2C0 => unported;
    /// `CMatrix::RotateZ(float)` (0x59B390)
    pub fn matrix_rotate_z(m: GPtr<u8>, a: f32) = thiscall 0x59B390 => unported;
    /// `CMatrix::UpdateRW()` (0x59BBB0)
    pub fn matrix_update_rw(m: GPtr<u8>) = thiscall 0x59BBB0 => unported;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    pub fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
}

/// `CVehicle::SetComponentRotation(RwFrame*, eRotationAxis, float angle, bool resetPosition)`
/// (0x6DBA30, the SEH setup sits in a .HOODLUM fragment at 0x403907): a stack `CMatrix` attached to the
/// frame's modelling matrix; with the flag the rotation is set (`SetRotateX/Y/Z` for axis 0/1/2), else
/// the position is first cancelled (`pos += -pos`) and the matrix rotated (`RotateX/Y/Z`); then the
/// saved position is added back, `UpdateRW`, and the matrix destroyed. Null frame: nothing.
///
/// Not hooked: protection code reads its entry bytes (0x156967D `xor ecx, [0x6DBA32]`).
pub fn set_component_rotation(_v: Veh, frame: GPtr<u8>, axis: u32, angle: f32, reset: u32) {
    if frame.is_null() {
        return;
    }
    let mut mat = [0u32; 18];
    let m = GPtr::from_raw(&raw mut mat).cast::<u8>();
    matrix_attach(m, frame.byte_add(0x10), 0);
    let saved = [rd::<f32>(m, 0x30), rd::<f32>(m, 0x34), rd::<f32>(m, 0x38)];
    if reset as u8 != 0 {
        match axis {
            2 => matrix_set_rotate_z(m, angle),
            1 => matrix_set_rotate_y(m, angle),
            0 => matrix_set_rotate_x(m, angle),
            _ => {}
        }
    } else {
        let negs = [
            x(rd::<f32>(m, 0x30)).fchs(),
            x(rd::<f32>(m, 0x34)).fchs(),
            x(rd::<f32>(m, 0x38)).fchs(),
        ];
        for (k, n) in negs.iter().enumerate() {
            let off = 0x30 + 4 * k as isize;
            wr::<f32>(m, off, x(rd::<f32>(m, off)).fadd(*n).to_f32());
        }
        match axis {
            2 => matrix_rotate_z(m, angle),
            1 => matrix_rotate_y(m, angle),
            0 => matrix_rotate_x(m, angle),
            _ => {}
        }
    }
    for (k, s) in saved.iter().enumerate() {
        let off = 0x30 + 4 * k as isize;
        wr::<f32>(m, off, x(rd::<f32>(m, off)).fadd_m(*s).to_f32());
    }
    matrix_update_rw(m);
    matrix_dtor(m);
}

orig! {
    /// CRT `ceil(double)` (0x823820, ST0)
    pub fn crt_ceil(v: f64) -> f64 = cdecl 0x823820 => unported;
    /// `CEntity::GetColModel()` (0x535300)
    pub fn entity_get_col_model(e: GPtr<u8>) -> GPtr<u8> = thiscall 0x535300 => unported;
}

/// `|ms . axis| * timeStep` with the move speed (+0x44) and the matrix row at `row`, summed z, y, x.
fn axis_speed(v: Veh, m: GPtr<u8>, row: isize) -> X87 {
    let d = x(rd::<f32>(v, 0x4C))
        .fmul_m(rd::<f32>(m, row + 8))
        .fadd(x(rd::<f32>(v, 0x48)).fmul_m(rd::<f32>(m, row + 4)))
        .fadd(x(rd::<f32>(v, 0x44)).fmul_m(rd::<f32>(m, row)));
    X87::from_f64_exact(f64::from_bits(d.to_f64().to_bits() & !(1u64 << 63)))
        .fmul_m(g::<f32>(0xB7CB5C).read())
}

/// `CVehicle::SpecialEntityCalcCollisionSteps(bool* processBefore, bool* processAfter)` (0x6D0E90):
/// static entities (bit 2 of `+0x40`) -> 1. `s = |v|^2 * timeStep^2` (f32); below 0.16 -> 1. Steps =
/// `ftol(ceil(sqrt(s) * k))` with k = 6.6667 (player bike) / 5.0 (player) / 3.3333 (others, s > 0.32)
/// / 2.5. The largest per-axis travel relative to the bounding box (forward / y extent, right / x, up
/// / z; x1.5 for bikes) sets `*processBefore` below 1.0, else `*processAfter` below 2.0. Returns the
/// steps (AL).
#[hook(addr = 0x6D0E90, conv = thiscall, name = "CVehicle::SpecialEntityCalcCollisionSteps", expect = "51 53 56 8B F1", this = "CVehicle")]
pub fn special_entity_calc_collision_steps(v: Veh, before: GPtr<u8>, after: GPtr<u8>) -> u8 {
    if rd::<u8>(v, 0x40) & 4 != 0 {
        return 1;
    }
    let c = |a: usize| g::<f32>(a).read();
    let ts = c(0xB7CB5C);
    let (mx, my, mz) = (
        x(rd::<f32>(v, 0x44)),
        x(rd::<f32>(v, 0x48)),
        x(rd::<f32>(v, 0x4C)),
    );
    let s = mx
        .fmul(mx)
        .fadd(my.fmul(my))
        .fadd(mz.fmul(mz))
        .fmul_m(ts)
        .fmul_m(ts)
        .to_f32();
    if x(s).fcom(x(c(0x871E34))).test_any(0x01) {
        return 1;
    }
    let r = x(s).fsqrt();
    let k = if rd::<u8>(v, 0x36) & 0xF8 == 0 {
        if rd::<u32>(v, 0x590) == 9 {
            c(0x866FBC)
        } else {
            c(0x858C80)
        }
    } else if !x(s).fcom(x(c(0x871E30))).test_any(0x41) {
        c(0x859F60)
    } else {
        c(0x858FA0)
    };
    let steps = ftol32(X87::from_f64_exact(crt_ceil(r.fmul_m(k).to_f64()))) as u8;
    let cm = entity_get_col_model(v);
    let m = rd::<GPtr<u8>>(v, 0x14);
    let ext = |hi: isize, lo: isize| x(rd::<f32>(cm, hi)).fsub_m(rd::<f32>(cm, lo));
    let v1 = axis_speed(v, m, 0x10).fdiv(ext(0x10, 4));
    let w = ext(0xC, 0).to_f32();
    let v2 = axis_speed(v, m, 0).fdiv_m(w);
    let mut cur = if !v1.fcom(v2).test_any(0x41) { v1 } else { v2 };
    let h = ext(0x14, 8).to_f32();
    let v3 = axis_speed(v, m, 0x20).fdiv_m(h);
    if cur.fcom(v3).test_any(0x41) {
        cur = v3;
    }
    if rd::<u32>(v, 0x590) == 9 {
        cur = cur.fmul_m(c(0x858CE8));
    }
    if !cur.fcom(x(c(0x858624))).test_parity(0x05) {
        before.write(1);
    } else if !cur.fcom(x(c(0x858CA0))).test_parity(0x05) {
        after.write(1);
    }
    steps
}

orig! {
    /// `CPhysical::CPhysical()` (0x542260)
    pub fn physical_ctor(p: GPtr<u8>) -> GPtr<u8> = thiscall 0x542260 => unported;
    /// `CAEVehicleAudioEntity::CAEVehicleAudioEntity()` (0x4F63E0)
    pub fn vehicle_audio_entity_ctor(a: GPtr<u8>) -> GPtr<u8> = thiscall 0x4F63E0 => unported;
    /// `CAutoPilot::CAutoPilot()` (0x6D5E20)
    pub fn auto_pilot_ctor(a: GPtr<u8>) -> GPtr<u8> = thiscall 0x6D5E20 => unported;
}

/// `CVehicle::CVehicle(eVehicleCreatedBy)` (0x6D5F10): `CPhysical` base, vtable 0x871E80, the audio
/// entity (+0x138) and autopilot (+0x390) members, then the field defaults in the original's store
/// order (incl. two `rand()` calls: the bit 3 of `+0x42C` from `rand * 3.05e-5 < 0`, never true, at
/// 0x6D5FCF; the dirt level `rand() % 15` at 0x6D621F) and `UpdateCarCount(this, false)`. Returns
/// `this`.
#[hook(addr = 0x6D5F10, conv = thiscall, name = "CVehicle::Constructor", expect = "6A FF 68 06 87 84 00", this = "CVehicle")]
pub fn constructor(v: Veh, created_by: u32) -> GPtr<u8> {
    physical_ctor(v);
    wr::<u32>(v, 0, 0x0087_1E80);
    vehicle_audio_entity_ctor(v.byte_add(0x138));
    auto_pilot_ctor(v.byte_add(0x390));
    wr::<u8>(v, 0x36, (rd::<u8>(v, 0x36) & 0xFA) | 2);
    let f42c = rd::<u8>(v, 0x42C) & 0xF8;
    wr::<u32>(v, 0x1C, rd::<u32>(v, 0x1C) | 0x20_0000);
    for off in [0x58C, 0x494, 0x498] {
        wr::<u32>(v, off, 0);
    }
    wr::<u8>(v, 0x4B4, 1);
    wr::<u32>(v, 0x4B8, 0);
    wr::<u32>(v, 0x4BC, 0);
    wr::<u8>(v, 0x4A4, created_by as u8);
    wr::<u16>(v, 0x45E, 0);
    wr::<u8>(v, 0x428, 0xB0);
    wr::<u8>(v, 0x429, 0);
    wr::<u8>(v, 0x42A, 0x20);
    wr::<u8>(v, 0x42B, 0);
    wr::<u8>(v, 0x42C, f42c);
    let r = rand_at(0x6D5FD4);
    let neg = !xi(r)
        .fmul_m(g::<f32>(0x858C7C).read())
        .fcom(x(g::<f32>(0x858B50).read()))
        .test_parity(0x05);
    wr::<u8>(v, 0x42C, (u8::from(neg) << 3) | (rd::<u8>(v, 0x42C) & 7));
    let f42f = rd::<u8>(v, 0x42F) & 0xF0;
    wr::<u8>(v, 0x42D, 0x50);
    wr::<u8>(v, 0x42E, 0x64);
    wr::<u8>(v, 0x42F, f42f);
    car_ctrl_update_car_count(v, 0);
    wr::<u16>(v, 0x4A6, 0);
    wr::<u32>(v, 0x4C0, 0x447A_0000);
    wr::<u32>(v, 0x460, 0);
    for off in 0x484..0x488 {
        wr::<u8>(v, off, 0);
    }
    wr::<u8>(v, 0x488, 8);
    let mut i = 0isize;
    loop {
        wr::<u32>(v, 0x464 + 4 * i, 0);
        i += 1;
        if i >= isize::from(rd::<u8>(v, 0x488)) {
            break;
        }
    }
    wr::<u8>(v, 0x4A9, rd::<u8>(v, 0x4A9) & 0xF0);
    wr::<u8>(v, 0x42C, rd::<u8>(v, 0x42C) & 0x9F);
    wr::<u8>(v, 0x4A8, rd::<u8>(v, 0x4A8) & 0xE0);
    wr::<u8>(v, 0x508, 0xFF);
    wr::<u8>(v, 0x48B, 0xFF);
    wr::<u32>(v, 0x40, rd::<u32>(v, 0x40) | 0x1000_0000);
    wr::<u16>(v, 0x4DE, 0);
    wr::<u32>(v, 0x4E0, 0);
    wr::<u8>(v, 0x4AC, 0);
    wr::<u32>(v, 0x4D8, 0);
    wr::<u32>(v, 0x490, 0);
    wr::<u32>(v, 0x4D4, 0);
    wr::<u16>(v, 0x4DC, 0);
    wr::<u8>(v, 0x4AA, 0);
    wr::<u32>(v, 0x514, 0);
    wr::<u8>(v, 0x518, 0);
    wr::<u8>(v, 0x519, 0);
    wr::<u8>(v, 0x4EC, 0);
    for off in [0x4C4, 0x4C8, 0x4CC, 0x4D0] {
        wr::<u32>(v, off, 0);
    }
    wr::<u16>(v, 0x45C, 0);
    wr::<u32>(v, 0x4F8, 1);
    for off in [0x4FC, 0x500, 0x504, 0x50C, 0x4E8, 0x4E4, 0x48C] {
        wr::<u32>(v, off, 0);
    }
    for off in [0x511, 0x512, 0x51A, 0x540, 0x56C] {
        wr::<u8>(v, off, 0);
    }
    wr::<u32>(v, 0x384, 0);
    wr::<u32>(v, 0x38C, 0);
    wr::<u8>(v, 0x3BA, 0);
    wr::<u8>(v, 0x3BB, 0);
    wr::<u32>(v, 0x3AC, g::<u32>(0xB7CB84).read());
    wr::<u8>(v, 0x3DB, rd::<u8>(v, 0x3DB) & 0xFB);
    wr::<u16>(v, 0x59A, 0xFFFF);
    wr::<u16>(v, 0x598, 0xFFFF);
    for off in [0x59C, 0x578, 0x57C, 0x580, 0x588] {
        wr::<u32>(v, off, 0);
    }
    for k in 0..7 {
        wr::<u32>(v, 0x43A + 4 * k, 0xFFFF_FFFF);
    }
    wr::<u16>(v, 0x456, 0xFFFF);
    let f4a8 = rd::<u8>(v, 0x4A8) & 0x9F;
    wr::<u32>(v, 0x458, 0x3F80_0000);
    for off in [0x489, 0x48A, 0x51B, 0x513] {
        wr::<u8>(v, off, 0);
    }
    wr::<u8>(v, 0x4A8, f4a8);
    let r2 = rand_at(0x6D6224);
    wr::<f32>(v, 0x4B0, xi(r2 % 15).to_f32());
    wr::<u32>(v, 0x430, g::<u32>(0xB7CB84).read());
    wr::<u32>(v, 0x574, 0x4848_4848);
    v
}

orig! {
    /// `CReplay::RecordVehicleDeleted(CVehicle*)` (0x45EBB0)
    pub fn replay_record_vehicle_deleted(v: GPtr<u8>) = cdecl 0x45EBB0 => unported;
    /// `CVehicle::DeleteRwObject()` (0x6D6410)
    pub fn delete_rw_object_o(v: GPtr<u8>) = thiscall 0x6D6410 => delete_rw_object;
    /// `CRadar::ClearBlipForEntity(eBlipType, int32 handle)` (0x587C60)
    pub fn radar_clear_blip_for_entity(ty: u32, handle: i32) = cdecl 0x587C60 => unported;
    /// virtual slot +0x54 of a ped (`FlagToDestroyWhenNextProcessed`)
    pub fn ped_vslot_54(p: GPtr<u8>) = thiscall vslot 0x54 => unported;
    /// `FxManager_c::DestroyFxSystem(FxSystem_c*)` (0x4A9810)
    pub fn fx_destroy_fx_system(mgr: GPtr<u8>, fx: GPtr<u8>) = thiscall 0x4A9810 => unported;
    /// `RwTextureDestroy(RwTexture*)` (0x7F3820)
    pub fn rw_texture_destroy(t: GPtr<u8>) -> i32 = cdecl 0x7F3820 => unported;
    /// `CDarkel::RegisterCarBlownUpByPlayer(CVehicle*, int32)` (0x43DF20)
    pub fn darkel_register_car_blown_up_by_player(v: GPtr<u8>, n: i32) = cdecl 0x43DF20 => unported;
    /// `CAEVehicleAudioEntity::~CAEVehicleAudioEntity()` (0x6D0A10)
    pub fn vehicle_audio_entity_dtor(a: GPtr<u8>) = thiscall 0x6D0A10 => unported;
    /// `CPhysical::~CPhysical()` (0x542450)
    pub fn physical_dtor(p: GPtr<u8>) = thiscall 0x542450 => unported;
}

/// `CVehicle::~CVehicle()` (0x6E2B40, the SEH handler push sits in a protection fragment at 0x401355):
/// vtable 0x871E80, `CReplay::RecordVehicleDeleted`, alarm (+0x45C) = 0, `DeleteRwObject`, the radar
/// blip of the pool handle cleared, occupants flagged for destruction (virtual +0x54; the passenger loop
/// counts in 16 bits against `+0x488`), the fire extinguished, the car counts and the mission /
/// law-enforcer counters (0x9690A8, 0x9690AC) updated, the hydraulics slot freed, the three FX systems
/// destroyed, the plate texture destroyed, the winch rope removed, `RegisterCarBlownUpByPlayer` for a
/// non-mission vehicle with health below 0x859F80, then the audio entity and `CPhysical` destructors.
#[hook(addr = 0x6E2B40, conv = thiscall, name = "CVehicle::Destructor", expect = "6A FF E9 0E E8 D1 FF", this = "CVehicle")]
pub fn destructor(v: Veh) {
    wr::<u32>(v, 0, 0x0087_1E80);
    replay_record_vehicle_deleted(v);
    wr::<u16>(v, 0x45C, 0);
    delete_rw_object_o(v);
    let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let slot = ((v.addr() as u32).wrapping_sub(rd::<u32>(pool, 0)) as i32) / 0xA18;
    let flags = i32::from(rd::<GPtr<u8>>(pool, 4).byte_add(slot as isize).read());
    radar_clear_blip_for_entity(1, (slot << 8).wrapping_add(flags));
    let d = rd::<GPtr<u8>>(v, 0x460);
    if !d.is_null() {
        ped_vslot_54(d);
    }
    if rd::<u8>(v, 0x488) != 0 {
        let mut i = 0u16;
        loop {
            let p = rd::<GPtr<u8>>(v, 0x464 + 4 * i as isize);
            if !p.is_null() {
                ped_vslot_54(p);
            }
            i = i.wrapping_add(1);
            if i >= u16::from(rd::<u8>(v, 0x488)) {
                break;
            }
        }
    }
    let fire = rd::<GPtr<u8>>(v, 0x490);
    if !fire.is_null() {
        fire_extinguish(fire);
        wr::<u32>(v, 0x490, 0);
    }
    car_ctrl_update_car_count(v, 1);
    if rd::<u8>(v, 0x428) & 2 != 0 {
        let c = g::<i32>(0x9690A8);
        c.write(c.read().wrapping_sub(1));
        wr::<u8>(v, 0x428, rd::<u8>(v, 0x428) & 0xFD);
    }
    if rd::<u8>(v, 0x428) & 4 != 0 {
        let c = g::<i32>(0x9690AC);
        c.write(c.read().wrapping_sub(1));
        wr::<u8>(v, 0x428, rd::<u8>(v, 0x428) & 0xFB);
    }
    let hs = rd::<i8>(v, 0x48B);
    if hs > -1 {
        g::<u32>(0xC1CC08 + 4 * hs as usize).write(0);
        wr::<u8>(v, 0x48B, 0xFF);
    }
    for off in [0x578, 0x57C, 0x580] {
        let fx = rd::<GPtr<u8>>(v, off);
        if !fx.is_null() {
            fx_destroy_fx_system(GPtr::from_addr(0xA9AE80), fx);
            wr::<u32>(v, off, 0);
        }
    }
    let tex = rd::<GPtr<u8>>(v, 0x588);
    if !tex.is_null() {
        rw_texture_destroy(tex);
        wr::<u32>(v, 0x588, 0);
    }
    let r = ropes_find_rope((v.addr() as u32).wrapping_add(0x1D));
    if r >= 0 {
        rope_remove(g::<u8>(0xB768B8).byte_add(r as isize * 0x328));
    }
    if rd::<u32>(v, 0x40) & 0x2000_0000 == 0
        && !x(rd::<f32>(v, 0x4C0))
            .fcom(x(g::<f32>(0x859F80).read()))
            .test_parity(0x05)
    {
        darkel_register_car_blown_up_by_player(v, 0);
    }
    vehicle_audio_entity_dtor(v.byte_add(0x138));
    physical_dtor(v);
}

orig! {
    /// `FindPlayerVehicle(int32, bool)` (0x56E0D0)
    pub fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CWeaponEffects::ClearCrossHairImmediately(int32)` (0x742CA0)
    pub fn clear_cross_hair_immediately(n: i32) = cdecl 0x742CA0 => unported;
    /// `CVehicle::GetPlaneWeaponFiringStatus(bool& fire, eOrdnanceType& ordnance)` (0x6E3440)
    pub fn get_plane_weapon_firing_status(v: GPtr<u8>, fire: GPtr<u8>, ord: GPtr<u8>) -> GPtr<u8> = thiscall 0x6E3440 => unported;
    /// `CVehicle::SelectPlaneWeapon(bool, eOrdnanceType)` (0x6D4900)
    pub fn select_plane_weapon_o(v: GPtr<u8>, choice: u32, ord: u32) = thiscall 0x6D4900 => select_plane_weapon;
    /// `CVehicle::FirePlaneGuns()` (0x6D4D30)
    pub fn fire_plane_guns(v: GPtr<u8>) = thiscall 0x6D4D30 => unported;
    /// `CVehicle::PossiblyDropFreeFallBombForPlayer(eOrdnanceType, bool)` (0x6E07E0)
    pub fn possibly_drop_free_fall_bomb(v: GPtr<u8>, ord: u32, b: u32) = thiscall 0x6E07E0 => unported;
    /// `CVehicle::FireHeatSeakingMissile(CEntity*, eOrdnanceType, bool)` (0x6E05C0)
    pub fn fire_heat_seaking_missile(v: GPtr<u8>, target: GPtr<u8>, ord: u32, b: u32) = thiscall 0x6E05C0 => unported;
    /// `CVehicle::FireUnguidedMissile(eOrdnanceType, bool)` (0x6D5110)
    pub fn fire_unguided_missile(v: GPtr<u8>, ord: u32, b: u32) = thiscall 0x6D5110 => unported;
    /// `CVehicle::GetPlaneNumGuns()` (0x6D3F30)
    pub fn get_plane_num_guns_o(v: GPtr<u8>) -> u32 = thiscall 0x6D3F30 => get_plane_num_guns;
    /// `FxSystem_c::SetMatrix(RwMatrix*)` (0x4AA630)
    pub fn fx_system_set_matrix(fx: GPtr<u8>, m: GPtr<u8>) = thiscall 0x4AA630 => unported;
}

/// `CVehicle::ProcessWeapons()` (0x6E3950): the player's plane (type 4) without gun FX clears the cross
/// hair; unless the entity flag 0x20000000: the player / remote status (0 or 8) asks
/// `GetPlaneWeaponFiringStatus` and `SelectPlaneWeapon(fire, ordnance)` (the fire flag is a byte stored
/// into an uninitialised dword local; the dword is passed whole), AI planes fire with bit 4 of `+0x42C`
/// (ordnance 2 for the hydra with weapon 3, else 1). Weapon `+0x513`: 1 guns, 2 free-fall bomb, 3
/// heat-seeker at the target (player: the firing-status result; AI: the player's vehicle for autopilot
/// missions 0x23/0x3B/0x3F, else `+0x41C`), 4 unguided missile. Then the gun FX systems (type 3 +0xA0C,
/// type 4 +0x9D4) follow the clump's frame matrix.
#[hook(addr = 0x6E3950, conv = thiscall, name = "CVehicle::ProcessWeapons", expect = "83 EC 08 53 56", this = "CVehicle")]
pub fn process_weapons(v: Veh) {
    let mut target = GPtr::<u8>::NULL;
    if rd::<u32>(v, 0x594) == 4 && v == find_player_vehicle(-1, 0) && rd::<u32>(v, 0x9DC) == 0 {
        clear_cross_hair_immediately(0);
    }
    if rd::<u32>(v, 0x40) & 0x2000_0000 != 0 {
        return;
    }
    let status = rd::<u8>(v, 0x36) >> 3;
    let (fire, ord) = if status == 0 || status == 8 {
        let mut locals = [0u32; 2];
        let lp = GPtr::from_raw(&raw mut locals).cast::<u8>();
        target = get_plane_weapon_firing_status(v, lp, lp.byte_add(4));
        select_plane_weapon_o(v, locals[0], locals[1]);
        (locals[0] as u8, locals[1])
    } else if rd::<u8>(v, 0x42C) & 0x10 != 0 {
        let ord = if rd::<u16>(v, 0x22) == 0x208 && rd::<u8>(v, 0x513) == 3 {
            2
        } else {
            1
        };
        (1, ord)
    } else {
        (0, 0)
    };
    match rd::<u8>(v, 0x513) {
        1 if fire != 0 => fire_plane_guns(v),
        2 if ord != 0 => possibly_drop_free_fall_bomb(v, ord, 1),
        3 if ord != 0 => {
            let s = rd::<u8>(v, 0x36) >> 3;
            if s != 0 && s != 8 {
                target = if matches!(rd::<u8>(v, 0x3BA), 0x23 | 0x3B | 0x3F) {
                    find_player_vehicle(-1, 0)
                } else {
                    rd::<GPtr<u8>>(v, 0x41C)
                };
            }
            if !target.is_null() {
                fire_heat_seaking_missile(v, target, ord, 1);
            }
        }
        4 if ord != 0 => fire_unguided_missile(v, ord, 1),
        _ => {}
    }
    let arr = match rd::<u32>(v, 0x594) {
        3 => rd::<GPtr<u8>>(v, 0xA0C),
        4 => rd::<GPtr<u8>>(v, 0x9D4),
        _ => return,
    };
    if arr.is_null() {
        return;
    }
    let n = get_plane_num_guns_o(v) as i32;
    for i in 0..n.max(0) {
        let fx = rd::<GPtr<u8>>(arr, 4 * i as isize);
        if !fx.is_null() {
            let clump = rd::<GPtr<u8>>(v, 0x18);
            let m = if clump.is_null() {
                GPtr::NULL
            } else {
                rd::<GPtr<u8>>(clump, 4).byte_add(0x10)
            };
            fx_system_set_matrix(fx, m);
        }
    }
}

orig! {
    /// `FxManager_c::CreateFxSystem(char* name, CVector* pos, RwMatrix*, bool)` (0x4A9BE0)
    pub fn fx_create_fx_system(mgr: GPtr<u8>, name: u32, pos: GPtr<u8>, m: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4A9BE0 => unported;
    /// `FxSystem_c::Play()` (0x4AA2F0)
    pub fn fx_play(fx: GPtr<u8>) = thiscall 0x4AA2F0 => unported;
    /// `FxSystem_c::Kill()` (0x4AA3F0)
    pub fn fx_kill(fx: GPtr<u8>) = thiscall 0x4AA3F0 => unported;
    /// `FxSystem_c::SetConstTime(int32, float)` (0x4AA6C0)
    pub fn fx_set_const_time(fx: GPtr<u8>, on: u32, t: f32) = thiscall 0x4AA6C0 => unported;
    /// `FUN_0040fec0` (0x40FEC0): `operator*(CVector* out, const CVector&, float)`
    pub fn vec_mul_scalar(out: GPtr<u8>, v: GPtr<u8>, s: u32) -> GPtr<u8> = cdecl 0x40FEC0 => unported;
    /// `FxSystem_c::SetVelAdd(CVector*)` (0x4AA730)
    pub fn fx_set_vel_add(fx: GPtr<u8>, v: GPtr<u8>) = thiscall 0x4AA730 => unported;
}

/// `CVehicle::AddDamagedVehicleParticles()` (0x6D2A80): not for planes (type 4). With health in
/// [250, 650), not exploded-flagged (bit 8 of `+0x40`) and a clump, the engine smoke FX (+0x578; name
/// 0x871E4C for handling engine type 'E', else 0x871E3C, at the model's engine position `struct +0x54`)
/// is created and played once, then kept at const time `1 - (health - 250) * 0.0025` with the velocity
/// `moveSpeed * 50`. Otherwise an existing FX is killed and cleared.
#[hook(addr = 0x6D2A80, conv = thiscall, name = "CVehicle::AddDamagedVehicleParticles", expect = "83 EC 18 56 8B F1", this = "CVehicle")]
pub fn add_damaged_vehicle_particles(v: Veh) {
    if rd::<u32>(v, 0x594) == 4 {
        return;
    }
    let c = |a: usize| g::<f32>(a).read();
    let h = x(rd::<f32>(v, 0x4C0));
    let electric = rd::<u8>(rd::<GPtr<u8>>(v, 0x384), 0x75) == 0x45;
    let smoking = !h.fcom(x(c(0x862D20))).test_parity(0x05)
        && x(rd::<f32>(v, 0x4C0)).fcom(x(c(0x859F80))).test_parity(0x05)
        && rd::<u32>(v, 0x40) & 0x100 == 0;
    if !smoking {
        let fx = rd::<GPtr<u8>>(v, 0x578);
        if !fx.is_null() {
            fx_kill(fx);
            wr::<u32>(v, 0x578, 0);
        }
        return;
    }
    let clump = rd::<GPtr<u8>>(v, 0x18);
    let m = if clump.is_null() {
        GPtr::NULL
    } else {
        rd::<GPtr<u8>>(clump, 4).byte_add(0x10)
    };
    if rd::<u32>(v, 0x578) == 0 && !m.is_null() {
        let s = rd::<GPtr<u8>>(mi_of(v), 0x5C).byte_add(0x54);
        let mut pos = [rd::<u32>(s, 0), rd::<u32>(s, 4), rd::<u32>(s, 8)];
        let name = if electric { 0x871E4C } else { 0x871E3C };
        let fx = fx_create_fx_system(
            GPtr::from_addr(0xA9AE80),
            name,
            GPtr::from_raw(&raw mut pos).cast(),
            m,
            0,
        );
        wr::<GPtr<u8>>(v, 0x578, fx);
        if !fx.is_null() {
            fx_play(fx);
        }
    }
    let fx = rd::<GPtr<u8>>(v, 0x578);
    if fx.is_null() {
        return;
    }
    let t = x(rd::<f32>(v, 0x4C0))
        .fsub_m(c(0x859F80))
        .fmul_m(c(0x86C340))
        .fsubr_m(c(0x858624));
    fx_set_const_time(fx, 1, t.to_f32());
    let mut tmp = [0u32; 3];
    let r = vec_mul_scalar(GPtr::from_raw(&raw mut tmp).cast(), v.byte_add(0x44), 0x4248_0000);
    let mut vel = [rd::<u32>(r, 0), rd::<u32>(r, 4), rd::<u32>(r, 8)];
    fx_set_vel_add(rd::<GPtr<u8>>(v, 0x578), GPtr::from_raw(&raw mut vel).cast());
}

orig! {
    /// `SurfaceInfos_c::IsSand(uint32)` (0x55E6F0)
    pub fn surface_is_sand(s: GPtr<u8>, surf: u32) -> u8 = thiscall 0x55E6F0 => unported;
    /// `SurfaceInfos_c::CreatesWheelGrass(uint32)` (0x55E910)
    pub fn surface_creates_wheel_grass(s: GPtr<u8>, surf: u32) -> u8 = thiscall 0x55E910 => unported;
    /// `SurfaceInfos_c::CreatesWheelGravel(uint32)` (0x55E930)
    pub fn surface_creates_wheel_gravel(s: GPtr<u8>, surf: u32) -> u8 = thiscall 0x55E930 => unported;
    /// `SurfaceInfos_c::CreatesWheelMud(uint32)` (0x55E950)
    pub fn surface_creates_wheel_mud(s: GPtr<u8>, surf: u32) -> u8 = thiscall 0x55E950 => unported;
    /// `SurfaceInfos_c::CreatesWheelDust(uint32)` (0x55E970)
    pub fn surface_creates_wheel_dust(s: GPtr<u8>, surf: u32) -> u8 = thiscall 0x55E970 => unported;
    /// `SurfaceInfos_c::CreatesWheelSand(uint32)` (0x55E990)
    pub fn surface_creates_wheel_sand(s: GPtr<u8>, surf: u32) -> u8 = thiscall 0x55E990 => unported;
    /// `SurfaceInfos_c::CreatesWheelSpray(uint32)` (0x55E9B0)
    pub fn surface_creates_wheel_spray(s: GPtr<u8>, surf: u32) -> u8 = thiscall 0x55E9B0 => unported;
    /// `Fx_c::AddWheelSpray(CVehicle*, CVector pos, uint8, uint8, float)` (0x49FB30)
    pub fn fx_add_wheel_spray(fx: GPtr<u8>, v: GPtr<u8>, px: u32, py: u32, pz: u32, a: u32, b: u32, l: u32) = thiscall 0x49FB30 => unported;
    /// `Fx_c::AddWheelGrass(CVehicle*, CVector pos, uint8, float)` (0x49FF20)
    pub fn fx_add_wheel_grass(fx: GPtr<u8>, v: GPtr<u8>, px: u32, py: u32, pz: u32, a: u32, l: u32) = thiscall 0x49FF20 => unported;
    /// `Fx_c::AddWheelGravel(CVehicle*, CVector pos, uint8, float)` (0x4A0170)
    pub fn fx_add_wheel_gravel(fx: GPtr<u8>, v: GPtr<u8>, px: u32, py: u32, pz: u32, a: u32, l: u32) = thiscall 0x4A0170 => unported;
    /// `Fx_c::AddWheelMud(CVehicle*, CVector pos, uint8, float)` (0x4A03C0)
    pub fn fx_add_wheel_mud(fx: GPtr<u8>, v: GPtr<u8>, px: u32, py: u32, pz: u32, a: u32, l: u32) = thiscall 0x4A03C0 => unported;
    /// `Fx_c::AddWheelDust(CVehicle*, CVector pos, uint8, float)` (0x4A09C0)
    pub fn fx_add_wheel_dust(fx: GPtr<u8>, v: GPtr<u8>, px: u32, py: u32, pz: u32, a: u32, l: u32) = thiscall 0x4A09C0 => unported;
    /// `Fx_c::AddWheelSand(CVehicle*, CVector pos, uint8, float)` (0x4A0610)
    pub fn fx_add_wheel_sand(fx: GPtr<u8>, v: GPtr<u8>, px: u32, py: u32, pz: u32, a: u32, l: u32) = thiscall 0x4A0610 => unported;
    /// `CGeneral::GetRandomNumberInRange<float>(float, float)` (0x41BD90)
    pub fn random_float_in_range_st0(min: u32, max: u32) -> f64 = cdecl 0x41BD90 => unported;
}

/// `g_fx` (`Fx_c`).
const G_FX: usize = 0xA9AE00;

/// `CVehicle::AddWheelDirtAndWater(CColPoint&, uint32 belowEffectSpeed, uint8 speedBand, bool inWater)`
/// (0x6D2D50): without the second argument only sand surfaces continue. In water: wheel spray. Else
/// by surface: grass, gravel (returns 1), mud, dust and sand (skipped when it rains and
/// `GetRandomNumberInRange(rain, 1.01) > 0.5`), spray (rain above 0.4 and no `CamNoRain` zone; else
/// returns 1); no effect -> 1. Every effect gets the contact point, the third argument and the
/// vehicle lighting (+0x12C). EAX 0/1.
#[hook(addr = 0x6D2D50, conv = thiscall, name = "CVehicle::AddWheelDirtAndWater", expect = "8B 44 24 08 85 C0", this = "CVehicle")]
pub fn add_wheel_dirt_and_water(v: Veh, cp: GPtr<u8>, below: u32, band: u32, in_water: u32) -> u32 {
    let si = GPtr::<u8>::from_addr(SURFACE_INFOS);
    let fx = GPtr::<u8>::from_addr(G_FX);
    let surf = || u32::from(rd::<u8>(cp, 0x23));
    if below == 0 && surface_is_sand(si, surf()) == 0 {
        return 0;
    }
    let (px, py, pz) = (rd::<u32>(cp, 0), rd::<u32>(cp, 4), rd::<u32>(cp, 8));
    let light = || rd::<u32>(v, 0x12C);
    if in_water as u8 != 0 {
        fx_add_wheel_spray(fx, v, px, py, pz, band, 1, light());
        return 0;
    }
    if surface_creates_wheel_grass(si, surf()) != 0 {
        fx_add_wheel_grass(fx, v, px, py, pz, band, light());
        return 0;
    }
    if surface_creates_wheel_gravel(si, surf()) != 0 {
        fx_add_wheel_gravel(fx, v, px, py, pz, band, light());
        return 1;
    }
    if surface_creates_wheel_mud(si, surf()) != 0 {
        fx_add_wheel_mud(fx, v, px, py, pz, band, light());
        return 0;
    }
    let rain = || g::<f32>(0xC81308).read();
    let dry_enough = || {
        if x(rain()).fcom(x(g::<f32>(0x858B50).read())).test_any(0x41) {
            return true;
        }
        X87::from_f64_exact(random_float_in_range_st0(rain().to_bits(), 0x3F81_47AE))
            .fcom(x(g::<f32>(0x858B8C).read()))
            .test_any(0x41)
    };
    if surface_creates_wheel_dust(si, surf()) != 0 {
        if dry_enough() {
            fx_add_wheel_dust(fx, v, px, py, pz, band, light());
        }
        return 0;
    }
    if surface_creates_wheel_sand(si, surf()) != 0 {
        if dry_enough() {
            fx_add_wheel_sand(fx, v, px, py, pz, band, light());
        }
        return 0;
    }
    if surface_creates_wheel_spray(si, surf()) == 0
        || x(rain()).fcom(x(g::<f32>(0x858EE8).read())).test_any(0x41)
        || cull_zones_cam_no_rain() != 0
    {
        return 1;
    }
    fx_add_wheel_spray(fx, v, px, py, pz, band, 0, light());
    0
}

orig! {
    /// `CVehicle::~CVehicle()` (0x6E2B40)
    pub fn destructor_o(v: GPtr<u8>) = thiscall 0x6E2B40 => destructor;
    /// `CVehicle::DoHeadLightReflectionTwin(CMatrix&)` (0x6E1600)
    pub fn do_head_light_reflection_twin(v: GPtr<u8>, m: GPtr<u8>) = thiscall 0x6E1600 => unported;
    /// `CVehicle::DoHeadLightReflectionSingle(CMatrix&, bool)` (0x6E1440)
    pub fn do_head_light_reflection_single(v: GPtr<u8>, m: GPtr<u8>, right: u32) = thiscall 0x6E1440 => unported;
    /// `CVehicleSaveStructure::Construct(CVehicle*)` (0x5D2330)
    pub fn save_structure_construct(s: GPtr<u8>, v: GPtr<u8>) = thiscall 0x5D2330 => unported;
    /// `CVehicleSaveStructure::Extract(CVehicle*)` (0x5D23B0)
    pub fn save_structure_extract(s: GPtr<u8>, v: GPtr<u8>) = thiscall 0x5D23B0 => unported;
    /// `CGenericGameStorage::SaveDataToWorkBuffer(void*, int32)` (0x5D1270)
    pub fn save_data_to_work_buffer(p: GPtr<u8>, n: i32) -> u8 = cdecl 0x5D1270 => unported;
    /// `CGenericGameStorage::LoadDataFromWorkBuffer(void*, int32)` (0x5D1300)
    pub fn load_data_from_work_buffer(p: GPtr<u8>, n: i32) -> u8 = cdecl 0x5D1300 => unported;
}

/// ``CVehicle::`scalar deleting destructor'(uint8 flags)`` (0x6E38A0): the destructor, then with bit 0
/// the vehicle pool slot is released (the inlined pool delete); returns `this`.
#[hook(addr = 0x6E38A0, conv = thiscall, name = "CVehicle::`scalar deleting destructor'", expect = "56 57 8B F9 E8 97 F2 FF FF", this = "CVehicle")]
pub fn scalar_deleting_destructor(v: Veh, flags: u32) -> GPtr<u8> {
    destructor_o(v);
    if flags & 1 != 0 {
        operator_delete(v);
    }
    v
}

/// `CVehicle::DoHeadLightReflection(CMatrix&, uint32 flags, uint8 left, uint8 right)` (0x6E1720): with
/// flag bit 0: both lights -> twin, left only -> single(false), right only -> single(true). Without it:
/// the combine harvester (532) gets the twin, everything else single(true).
#[hook(addr = 0x6E1720, conv = thiscall, name = "CVehicle::DoHeadLightReflection", expect = "F6 44 24 08 01", this = "CVehicle")]
pub fn do_head_light_reflection(v: Veh, m: GPtr<u8>, flags: u32, left: u32, right: u32) {
    if flags & 1 != 0 {
        match (left as u8 != 0, right as u8 != 0) {
            (true, true) => do_head_light_reflection_twin(v, m),
            (true, false) => do_head_light_reflection_single(v, m, 0),
            (false, true) => do_head_light_reflection_single(v, m, 1),
            (false, false) => {}
        }
    } else if rd::<u16>(v, 0x22) == 0x214 {
        do_head_light_reflection_twin(v, m);
    } else {
        do_head_light_reflection_single(v, m, 1);
    }
}

/// A stack `CVehicleSaveStructure` (0x68 bytes, a `CMatrix` first) preceded by its size dword, as
/// `Save` / `Load` lay them out (matrix attach pointer and owns flag cleared).
fn save_buffer() -> [u32; 0x1B] {
    let mut b = [0u32; 0x1B];
    b[0] = 0x68;
    b
}

/// `CVehicle::Save()` (0x5D4760): the save structure built from the vehicle (`Construct`), then the
/// size dword (4 bytes) and the structure (0x68 bytes) written to the work buffer; the structure's
/// matrix destroyed. AL = 1.
///
/// Not registered (decision needed): the original's structure is an uninitialised stack buffer, so any
/// byte `Construct` leaves unwritten reaches the save file as stack garbage; the port's buffer is zeroed.
pub fn save(v: Veh) -> u8 {
    let mut b = save_buffer();
    let p = GPtr::from_raw(&raw mut b).cast::<u8>();
    save_structure_construct(p.byte_add(4), v);
    save_data_to_work_buffer(p, 4);
    save_data_to_work_buffer(p.byte_add(4), 0x68);
    matrix_dtor(p.byte_add(4));
    1
}

/// `CVehicle::Load()` (0x5D2900): the size dword and the 0x68-byte save structure read from the work
/// buffer, `Extract` into the vehicle, the structure's matrix destroyed. AL = 1.
#[hook(addr = 0x5D2900, conv = thiscall, name = "CVehicle::Load", expect = "6A FF 68 D8 D5 83 00", this = "CVehicle")]
pub fn load(v: Veh) -> u8 {
    let mut b = save_buffer();
    let p = GPtr::from_raw(&raw mut b).cast::<u8>();
    load_data_from_work_buffer(p, 4);
    load_data_from_work_buffer(p.byte_add(4), 0x68);
    save_structure_extract(p.byte_add(4), v);
    matrix_dtor(p.byte_add(4));
    1
}

pub const HOOKS: &[HookReg] = &[
    HOOK_FLAG_428_BIT5,
    HOOK_HAS_VALID_45C,
    HOOK_REMOVE_LIGHTING,
    HOOK_SETUP_LIGHTING,
    HOOK_GET_CURRENT_ATOMIC_OBJECT_CB,
    HOOK_INIT_S_TEST_BLADE_COL,
    HOOK_INIT_S_TEST_BLADE_COL_DATA,
    HOOK_INIT_SPECIAL_COL_MODEL,
    HOOK_UPDATE_PASSENGER_LIST,
    HOOK_RENDER,
    HOOK_RESET_AFTER_RENDER,
    HOOK_USES_SIREN,
    HOOK_CHANGE_LAW_ENFORCER_STATE,
    HOOK_IS_PASSENGER_PED,
    HOOK_OPERATOR_NEW,
    HOOK_OPERATOR_DELETE,
    HOOK_IS_UPSIDE_DOWN,
    HOOK_IS_ON_ITS_SIDE,
    HOOK_SET_VEHICLE_ATOMIC_VISIBILITY_CB_OBJECT,
    HOOK_PROCESS_WHEEL_ROTATION,
    HOOK_REMOVE_ALL_UPGRADES_CB,
    HOOK_PRE_RENDER_DRIVER_AND_PASSENGERS,
    HOOK_RENDER_DRIVER_AND_PASSENGERS,
    HOOK_SET_COMPONENT_VISIBILITY,
    HOOK_GET_VEHICLE_APPEARANCE,
    HOOK_PROCESS_DELAYED_EXPLOSION,
    HOOK_ACTIVATE_BOMB_WHEN_ENTERED,
    HOOK_PROCESS_CAR_ALARM,
    HOOK_DELETE_RW_OBJECT,
    HOOK_PRE_RENDER,
    HOOK_UPDATE_CLUMP_ALPHA,
    HOOK_PICK_RANDOM_PASSENGER,
    HOOK_HEIGHT_ABOVE_CEILING,
    HOOK_CUSTOM_CAR_PLATE_TEXTURE_CREATE,
    HOOK_CAN_BE_DRIVEN,
    HOOK_SET_MODEL_INDEX,
    HOOK_REMOVE_PASSENGER,
    HOOK_CLEAR_VEHICLE_UPGRADE_FLAGS,
    HOOK_SELECT_PLANE_WEAPON,
    HOOK_UPDATE_WINCH,
    HOOK_REMOVE_DRIVER,
    HOOK_KILL_PEDS_IN_VEHICLE,
    HOOK_CALCULATE_LIGHTING_FROM_COLLISION,
    HOOK_PROCESS_SIREN_AND_HORN,
    HOOK_GET_VEHICLE_LIGHTS_STATUS,
    HOOK_SETUP_RENDER,
    HOOK_CAN_PED_STEP_OUT_CAR,
    HOOK_SET_DRIVER,
    HOOK_FIELD_9EC_IS_MINUS_1000,
    HOOK_DTOR_SPECIAL_COL_MODEL,
    HOOK_SET_FLAG_428_BIT4,
    HOOK_OPERATOR_NEW_AT,
    HOOK_OPERATOR_DELETE_AT,
    HOOK_CAR_HAS_ROOF,
    HOOK_SET_WINDOW_OPEN_FLAG,
    HOOK_CLEAR_WINDOW_OPEN_FLAG,
    HOOK_GET_PLANE_NUM_GUNS,
    HOOK_REMOVE_WINCH,
    HOOK_IS_VEHICLE_NORMAL,
    HOOK_SET_VEHICLE_ATOMIC_VISIBILITY_CB_FRAME,
    HOOK_REMOVE_OBJECTS_CB_FRAME,
    HOOK_REMOVE_ALL_UPGRADES,
    HOOK_SET_REMAP_TEX_DICTIONARY,
    HOOK_SET_VEHICLE_CREATED_BY,
    HOOK_IS_PASSENGER_MODEL,
    HOOK_COPY_OBJECTS_CB,
    HOOK_GET_FIRING_RATE_MULTIPLIER,
    HOOK_SET_FIRING_RATE_MULTIPLIER,
    HOOK_FIND_REPLACEMENT_UPGRADE_CB,
    HOOK_FIND_UPGRADE_CB,
    HOOK_SET_UP_DRIVER,
    HOOK_GET_REPLACEMENT_UPGRADE,
    HOOK_GET_REMAP_INDEX,
    HOOK_GET_SPARE_HASSLE_POS_ID,
    HOOK_GET_PLANE_GUNS_AUTO_AIM_ANGLE,
    HOOK_ACTIVATE_BOMB,
    HOOK_EXTINGUISH_CAR_FIRE,
    HOOK_REMOVE_REPLACEMENT_UPGRADE,
    HOOK_SET_REMAP,
    HOOK_REMOVE_OBJECTS_CB_OBJECT,
    HOOK_CAN_PED_LEAN_OUT,
    HOOK_GET_TOW_HITCH_POS,
    HOOK_GET_TOW_BAR_POS,
    HOOK_SETUP_UPGRADES_AFTER_LOAD,
    HOOK_MAKE_DIRTY,
    HOOK_CAN_PED_JUMP_OUT_CAR,
    HOOK_ADD_PASSENGER_FIXED_SEAT,
    HOOK_APPLY_BOAT_WATER_RESISTANCE,
    HOOK_SPECIAL_ENTITY_CALC_COLLISION_STEPS,
    HOOK_CONSTRUCTOR,
    HOOK_DESTRUCTOR,
    HOOK_PROCESS_WEAPONS,
    HOOK_ADD_DAMAGED_VEHICLE_PARTICLES,
    HOOK_ADD_WHEEL_DIRT_AND_WATER,
    HOOK_SCALAR_DELETING_DESTRUCTOR,
    HOOK_DO_HEAD_LIGHT_REFLECTION,
    HOOK_LOAD,
];

/// This module's globals (replace with a `sa_core::globals! { list GLOBALS; ... }` block when it gets some).
pub const GLOBALS: &[sa_core::mem::GlobalDesc] = &[];

/// [`physical_get_lighting_from_col_st0`] as an x87 value (ST0, returned through the f64 ABI).
#[allow(dead_code)]
pub fn physical_get_lighting_from_col(p: GPtr<u8>, interior: u32) -> X87 {
    X87::from_f64_exact(physical_get_lighting_from_col_st0(p, interior))
}

/// [`random_float_in_range_st0`] as an x87 value (ST0, returned through the f64 ABI).
#[allow(dead_code)]
pub fn random_float_in_range(min: u32, max: u32) -> X87 {
    X87::from_f64_exact(random_float_in_range_st0(min, max))
}
