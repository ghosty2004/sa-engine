//! The small POP-catalogued managers (PORT-POP): `CDarkel`, `CTagManager`, `CTrafficLights`,
//! `CRoadBlocks`, `CAccidentManager`, the leaf part. Ported from the 1.0 US bytes; gta-reversed
//! `Darkel.cpp`, `TagManager.cpp`, `TrafficLights.cpp`, `RoadBlocks.cpp`, `AccidentManager.cpp` as
//! reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::timer::TIME_MS;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use crate::common::{ftol32, jne_mask, jp5, model_info, rd, wr, x};
use crate::population::ALL_RANDOM_PEDS_THIS_TYPE;

sa_core::globals! {
    list GLOBALS;
    /// `CDarkel::TimeOfFrenzyStart`
    pub static TIME_OF_FRENZY_START @ 0x96A6E0: u32 = 0;
    /// `CDarkel::ModelToKill[4]`
    pub static MODEL_TO_KILL @ 0x96A6F0: [i32; 4] = [0; 4];
    /// `CDarkel::Status`
    pub static DARKEL_STATUS @ 0x96A704: u16 = 0;
    /// `CTagManager::ms_tagDesc` (150 x { CEntity*, uint8 alpha })
    pub static TAG_DESC @ 0xA9A8C0: [u8; 150 * 8] = [0; 150 * 8];
    /// `CTagManager::ms_numTags`
    pub static NUM_TAGS @ 0xA9AD70: i32 = 0;
    /// `CTagManager::ms_numTagged`
    pub static NUM_TAGGED @ 0xA9AD74: i32 = 0;
    /// `CTagManager::ms_pPipeline`
    pub static TAG_PIPELINE @ 0xA9AD78: GPtr<u8> = GPtr::NULL;
    /// `CRoadBlocks::aScriptRoadBlocks` (16 x 0x1C, +0 active)
    pub static SCRIPT_ROAD_BLOCKS @ 0xA43AD0: [u8; 16 * 0x1C] = [0; 16 * 0x1C];
}

/// `CCheat::m_aCheatsActive` entry 0x1E ("all traffic lights green"), 0x96914E (sa-world's cheat table)
const CHEAT_GREEN_LIGHTS: usize = 0x96914E;

orig! {
    /// `CHud::SetHelpMessage(const char*, bool, bool, bool)` (0x588BE0)
    pub fn hud_set_help_message(text: GPtr<u8>, quick: u8, permanent: u8, add_to_brief: u8) = cdecl 0x588BE0 => unported;
    /// `CDarkel::DealWithWeaponChangeAtEndOfFrenzy()` (0x43D7A0)
    pub fn deal_with_weapon_change_at_end_of_frenzy() = cdecl 0x43D7A0 => unported;
    /// `CDarkel::FrenzyOnGoing()` (0x43D1F0)
    pub fn frenzy_on_going_call() -> u8 = cdecl 0x43D1F0 => frenzy_on_going;
    /// `CVisibilityPlugins::SetUserValue(RpAtomic*, uint16)` (0x732380)
    pub fn set_user_value(atomic: GPtr<u8>, v: u32) = cdecl 0x732380 => unported;
    /// `CGameLogic::LaRiotsActiveHere()` (0x441C10)
    pub fn la_riots_active_here() -> u8 = cdecl 0x441C10 => unported;
    /// virtual slot 9 (`GetModelType`) of a `CBaseModelInfo`
    pub fn model_info_get_type(mi: GPtr<u8>) -> i32 = thiscall vslot 0x24 => unported;
    /// virtual slot 1 (`AsAtomicModelInfoPtr`) of a `CBaseModelInfo`
    pub fn model_info_as_atomic(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x4 => unported;
    /// `CPlaceable::AllocateMatrix()` (0x54F560)
    pub fn allocate_matrix(e: GPtr<u8>) = thiscall 0x54F560 => unported;
    /// `CSimpleTransform::UpdateMatrix(CMatrix*)` (0x54F1B0)
    pub fn update_matrix(t: GPtr<u8>, m: GPtr<u8>) = thiscall 0x54F1B0 => unported;
    /// `CGeneral::GetATanOfXY(float, float)` (0x53CC70)
    pub fn get_atan_of_xy(x: f32, y: f32) -> f64 = cdecl 0x53CC70 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    pub fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
}

fn frenzy_active() -> bool {
    let s = DARKEL_STATUS.get();
    s == 1 || s == 4
}

/// `CDarkel::Init()` (stub 0x43CEB0): status 0.
#[hook(addr = 0x43CEB0, conv = cdecl, name = "CDarkel::Init", expect = "E9 FB 78 12 01")]
pub fn darkel_init() {
    DARKEL_STATUS.set(0);
}

/// `CDarkel::FrenzyOnGoing()` (stub 0x43D1F0): status 1 or 4 (EAX 0/1).
#[hook(addr = 0x43D1F0, conv = cdecl, name = "CDarkel::FrenzyOnGoing", expect = "E9 CB 63 12 01")]
pub fn frenzy_on_going() -> u8 {
    u8::from(frenzy_active())
}

/// `CDarkel::ResetOnPlayerDeath()` (0x43DC10).
#[hook(addr = 0x43DC10, conv = cdecl, name = "CDarkel::ResetOnPlayerDeath", expect = "6A 00 6A 00 6A 01")]
pub fn reset_on_player_death() {
    hud_set_help_message(GPtr::NULL, 1, 0, 0);
    if !frenzy_active() {
        return;
    }
    DARKEL_STATUS.set(3);
    ALL_RANDOM_PEDS_THIS_TYPE.set(-1);
    TIME_OF_FRENZY_START.set(TIME_MS.get());
    deal_with_weapon_change_at_end_of_frenzy();
}

/// `CDarkel::ThisVehicleShouldBeKilledForFrenzy(const CVehicle&)` (stub 0x43D350).
#[hook(addr = 0x43D350, conv = cdecl, name = "CDarkel::ThisVehicleShouldBeKilledForFrenzy", expect = "E9 7B FB 12 01")]
pub fn this_vehicle_should_be_killed_for_frenzy(veh: GPtr<u8>) -> u8 {
    if !frenzy_active() {
        return 0;
    }
    let mk = MODEL_TO_KILL.get();
    if mk[3] == -2 {
        return 1;
    }
    let m = i32::from(rd::<i16>(veh, 0x22));
    if m == mk[3] || m == mk[2] || m == mk[1] || m == mk[0] {
        return 1;
    }
    u8::from(mk[3] == -3 && rd::<i32>(veh, 0x594) == 9)
}

/// `CTagManager::Init()` (stub 0x49CC50).
#[hook(addr = 0x49CC50, conv = cdecl, name = "CTagManager::Init", expect = "E9 0B E0 0C 01")]
pub fn tag_init() {
    NUM_TAGS.set(0);
    NUM_TAGGED.set(0);
}

fn tag(i: i32) -> GPtr<u8> {
    TAG_DESC.ptr().cast::<u8>().byte_add(i.wrapping_mul(8) as isize)
}

/// `CTagManager::AddTag(CEntity*)` (stub 0x49CC90).
#[hook(addr = 0x49CC90, conv = cdecl, name = "CTagManager::AddTag", expect = "E9 7B 54 0C 01")]
pub fn add_tag(e: GPtr<u8>) {
    let n = NUM_TAGS.get();
    wr(tag(n), 0, e);
    wr::<u8>(tag(n), 4, 0);
    NUM_TAGS.set(n.wrapping_add(1));
}

/// `CTagManager::ShutdownForRestart()` (stub 0x49CC60): every tag's alpha 0 (downwards), tagged count 0.
#[hook(addr = 0x49CC60, conv = cdecl, name = "CTagManager::ShutdownForRestart", expect = "E9 9B 1C 0D 01")]
pub fn shutdown_for_restart() {
    let mut n = NUM_TAGS.get();
    while n != 0 {
        n = n.wrapping_sub(1);
        wr::<u8>(tag(n), 4, 0);
    }
    NUM_TAGGED.set(0);
}

/// `fild` of an unsigned dword (signed load, + 2^32 when negative).
fn fild_u32(v: i32) -> X87 {
    let r = crate::common::xi(v);
    if v < 0 { r.fadd_m(4_294_967_296.0) } else { r }
}

/// `CTagManager::GetPercentageTagged()` (0x49CDA0): `_ftol(numTagged / numTags * 100)` (unsigned loads).
#[hook(addr = 0x49CDA0, conv = cdecl, name = "CTagManager::GetPercentageTagged", expect = "51 A1 74 AD A9 00")]
pub fn get_percentage_tagged() -> i32 {
    let a = fild_u32(NUM_TAGGED.get());
    let b = fild_u32(NUM_TAGS.get());
    ftol32(a.fdiv(b).fmul_m(100.0))
}

/// `CTagManager::IsTag(const CEntity*)` (0x49CCE0): an atomic model info (type 1) whose flags +0x12 have
/// special type 6 (`& 0x7800 == 0x3000`) and whose atomic info +0x13 bit 7 is clear.
#[hook(addr = 0x49CCE0, conv = cdecl, name = "CTagManager::IsTag", expect = "8B 44 24 04 0F BF 48 22")]
pub fn is_tag(e: GPtr<u8>) -> u8 {
    let mi = model_info(i32::from(rd::<i16>(e, 0x22)));
    if model_info_get_type(mi) != 1 {
        return 0;
    }
    if u32::from(rd::<u16>(mi, 0x12)) & 0x7800 != 0x3000 {
        return 0;
    }
    let a = model_info_as_atomic(mi);
    if rd::<u8>(a, 0x13) & 0x80 != 0 { 0 } else { 1 }
}

/// `CTagManager::SetupAtomic(RpAtomic*)` (stub 0x49CE10).
#[hook(addr = 0x49CE10, conv = cdecl, name = "CTagManager::SetupAtomic", expect = "E9 5B 24 0D 01")]
pub fn setup_atomic(atomic: GPtr<u8>) {
    let geom: GPtr<u8> = rd(atomic, 0x18);
    let list: GPtr<u8> = rd(geom, 0x20);
    let mat: GPtr<u8> = rd(list, 4);
    wr(mat, 8, TAG_PIPELINE.get());
    wr(geom, 8, rd::<u32>(geom, 8) | 0x40);
    set_user_value(atomic, 0);
}

fn light_phase() -> u32 {
    (TIME_MS.get() >> 1) & 0x3FFF
}

fn lights_blocked() -> bool {
    la_riots_active_here() != 0 || crate::common::g::<u8>(CHEAT_GREEN_LIGHTS).read() != 0
}

fn light1() -> u8 {
    if lights_blocked() {
        return 0;
    }
    let t = light_phase();
    if t < 5000 {
        return 0;
    }
    u8::from(t >= 6000) + 1
}

fn light2() -> u8 {
    if lights_blocked() {
        return 0;
    }
    let t = light_phase();
    if t < 6000 {
        return 2;
    }
    if t < 11000 {
        return 0;
    }
    u8::from(t >= 12000) + 1
}

/// `CTrafficLights::LightForCars1()` (stub 0x49D2D0).
#[hook(addr = 0x49D2D0, conv = cdecl, name = "CTrafficLights::LightForCars1", expect = "E9 DB 12 0D 01")]
pub fn light_for_cars1() -> u8 {
    light1()
}

/// `CTrafficLights::LightForCars2()` (stub 0x49D310).
#[hook(addr = 0x49D310, conv = cdecl, name = "CTrafficLights::LightForCars2", expect = "E9 BB 4D 0C 01")]
pub fn light_for_cars2() -> u8 {
    light2()
}

/// `CTrafficLights::LightForCars1_Visual()` (0x49D350): flashing amber/red during the riots, else as
/// `LightForCars1` (which calls `LaRiotsActiveHere` a second time).
#[hook(addr = 0x49D350, conv = cdecl, name = "CTrafficLights::LightForCars1_Visual", expect = "E8 BB 48 FA FF")]
pub fn light_for_cars1_visual() -> u8 {
    if la_riots_active_here() != 0 {
        return ((TIME_MS.get() >> 9) as u8 & 2) | 1;
    }
    light1()
}

/// `CTrafficLights::LightForCars2_Visual()` (0x49D3A0).
#[hook(addr = 0x49D3A0, conv = cdecl, name = "CTrafficLights::LightForCars2_Visual", expect = "E8 6B 48 FA FF")]
pub fn light_for_cars2_visual() -> u8 {
    if la_riots_active_here() != 0 {
        return ((TIME_MS.get() >> 9) as u8 & 2) | 1;
    }
    light2()
}

/// `CTrafficLights::FindTrafficLightTypeFromOrientation(float)` (0x49D4D0): 1 in (60, 150) and (240, 330),
/// else 2 (x87 compares, NaN gives 2).
#[hook(addr = 0x49D4D0, conv = cdecl, name = "CTrafficLights::FindTrafficLightTypeFromOrientation", expect = "D9 44 24 04 D8 1D 34 8B 85 00")]
pub fn find_traffic_light_type_from_orientation(a: f32) -> i32 {
    let v = x(a);
    if !jne_mask(v, x(60.0), 0x41) && !jp5(v, x(150.0)) {
        return 1;
    }
    if jne_mask(v, x(240.0), 0x41) || jp5(v, x(330.0)) {
        return 2;
    }
    1
}

/// `CTrafficLights::IsMITrafficLight(int32)` (0x49D5A0): one of the nine traffic-light model indices
/// (`ModelIndices` 0x8CD4F4..0x8CD514).
#[hook(addr = 0x49D5A0, conv = cdecl, name = "CTrafficLights::IsMITrafficLight", expect = "0F B7 0D F4 D4 8C 00")]
pub fn is_mi_traffic_light(mi: i32) -> u8 {
    let t = sa_stream::modelinfo::model_indices::MODEL_INDICES_A
        .ptr()
        .cast::<u16>();
    for k in [0, 2, 4, 6, 8, 10, 12, 16, 14] {
        if mi == i32::from(t.wrapping_index(k).read()) {
            return 1;
        }
    }
    0
}

/// `CTrafficLights::FindOrientationForTrafficLightType(CEntity*)` (0x49D520): `GetATanOfXY(m.up.x? +0x10,
/// +0x14) * 57.295776` of the entity matrix (allocated and updated from the simple transform when absent,
/// checked twice as the bytes).
#[hook(addr = 0x49D520, conv = cdecl, name = "CTrafficLights::FindOrientationForTrafficLightType", expect = "56 8B 74 24 08")]
pub fn find_orientation_for_traffic_light_type(e: GPtr<u8>) -> X87 {
    for _ in 0..2 {
        if rd::<GPtr<u8>>(e, 0x14).is_null() {
            allocate_matrix(e);
            update_matrix(e.byte_add(4), rd(e, 0x14));
        }
    }
    let m: GPtr<u8> = rd(e, 0x14);
    let r = get_atan_of_xy(rd(m, 0x10), rd(m, 0x14));
    X87::from_f64_exact(r).fmul_m(f32::from_bits(0x4265_2EE0))
}

/// `CRoadBlocks::ClearScriptRoadBlocks()` (stub 0x460EC0).
#[hook(addr = 0x460EC0, conv = cdecl, name = "CRoadBlocks::ClearScriptRoadBlocks", expect = "E9 AB E0 10 01")]
pub fn clear_script_road_blocks() {
    let b = SCRIPT_ROAD_BLOCKS.ptr().cast::<u8>();
    for k in 0..16 {
        wr::<u8>(b, k * 0x1C, 0);
    }
}

/// `CAccidentManager::GetNumberOfFreeAccidents()` (0x56CEE0, thiscall): entries with a ped, neither flag
/// set, and the ped's +0x40 bit 8 clear.
#[hook(addr = 0x56CEE0, conv = thiscall, name = "CAccidentManager::GetNumberOfFreeAccidents", expect = "56 57 33 C0 8B F1", this = "CAccidentManager")]
pub fn get_number_of_free_accidents(this: GPtr<u8>) -> i32 {
    let mut n = 0;
    for i in 0..16 {
        let a = this.byte_add(i * 8);
        let ped: GPtr<u8> = rd(a, 0);
        if !ped.is_null() && rd::<u8>(a, 4) == 0 && rd::<u8>(a, 5) == 0 && rd::<u32>(ped, 0x40) & 0x100 == 0 {
            n += 1;
        }
    }
    n
}

/// `CAccidentManager::ReportAccident(CPed*)` (0x56CE80, thiscall, `ret 4`): a ped not in state 2 (+0x484)
/// with +0x46C bit 31 set goes into the first free entry (unless listed), then `ped->RegisterReference`.
#[hook(addr = 0x56CE80, conv = thiscall, name = "CAccidentManager::ReportAccident", expect = "57 8B F9 8B 4C 24 08", this = "CAccidentManager")]
pub fn report_accident(this: GPtr<u8>, ped: GPtr<u8>) {
    if rd::<u8>(ped, 0x484) == 2 || rd::<i32>(ped, 0x46C) >= 0 {
        return;
    }
    let mut free = -1i32;
    for i in 0..16 {
        let p: GPtr<u8> = rd(this, i * 8);
        if p == ped {
            return;
        }
        if p.is_null() && free == -1 {
            free = i as i32;
        }
    }
    if free == -1 {
        return;
    }
    let a = this.byte_add(free as isize * 8);
    wr(a, 0, ped);
    wr::<u8>(a, 4, 0);
    wr::<u8>(a, 5, 0);
    register_reference(ped, a);
}

pub const HOOKS: &[HookReg] = &[
    HOOK_DARKEL_INIT,
    HOOK_FRENZY_ON_GOING,
    HOOK_RESET_ON_PLAYER_DEATH,
    HOOK_THIS_VEHICLE_SHOULD_BE_KILLED_FOR_FRENZY,
    HOOK_TAG_INIT,
    HOOK_ADD_TAG,
    HOOK_SHUTDOWN_FOR_RESTART,
    HOOK_GET_PERCENTAGE_TAGGED,
    HOOK_IS_TAG,
    HOOK_SETUP_ATOMIC,
    HOOK_LIGHT_FOR_CARS1,
    HOOK_LIGHT_FOR_CARS2,
    HOOK_LIGHT_FOR_CARS1_VISUAL,
    HOOK_LIGHT_FOR_CARS2_VISUAL,
    HOOK_FIND_TRAFFIC_LIGHT_TYPE_FROM_ORIENTATION,
    HOOK_IS_MI_TRAFFIC_LIGHT,
    HOOK_FIND_ORIENTATION_FOR_TRAFFIC_LIGHT_TYPE,
    HOOK_CLEAR_SCRIPT_ROAD_BLOCKS,
    HOOK_GET_NUMBER_OF_FREE_ACCIDENTS,
    HOOK_REPORT_ACCIDENT,
];
