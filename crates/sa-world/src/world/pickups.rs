//! `CPickup` / `CPickups` leaves (PORT-WORLD). Ported from the 1.0 US bytes (gta-reversed
//! `Pickup.cpp`, `Pickups.cpp` and `docs/re/specs/WORLD/CPickup*.md` as reading aids).
//!
//! `CPickup` (0x20): +0 revenue value (f32), +4 `m_pObject`, +8 ammo, +0xC regeneration time, +0x10/+0x12/
//! +0x14 position (int16, 1/8 units), +0x16 money per day (u16), +0x18 model index (i16), +0x1A reference
//! index (u16), +0x1C type (u8), +0x1D flags (bit 0 disabled, bit 3 visible...). `aPickUps[620]` at
//! 0x9788C0; a handle is `(reference << 16) | index`.

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{find_player_ped, find_player_vehicle, ftol32, g, rd, wr, x, xi};

pub const NUM_PICKUPS: usize = 620;
pub const PICKUP_SIZE: usize = 0x20;
pub const NUM_COLLECTED: usize = 20;

const _: () = assert!(PICKUP_SIZE == sa_core::layouts::size::CPickup);

sa_core::globals! {
    list GLOBALS;
    /// `CPickups::DisplayHelpMessage`
    pub static DISPLAY_HELP_MESSAGE @ 0x8A5F48: u8 = 10;
    /// `CPickups::CollectedPickUpIndex`
    pub static COLLECTED_PICKUP_INDEX @ 0x978624: u16 = 0;
    /// `CPickups::aPickUpsCollected`
    pub static PICKUPS_COLLECTED @ 0x978628: [i32; NUM_COLLECTED] = [0; NUM_COLLECTED];
    /// `CPickups::NumMessages`
    pub static NUM_MESSAGES @ 0x978678: u16 = 0;
    /// `CPickups::aPickUps`
    pub static PICKUPS @ 0x9788C0: [u8; NUM_PICKUPS * PICKUP_SIZE] = [0; NUM_PICKUPS * PICKUP_SIZE];
}

/// 0x859000: 8.0; 0x858C48: 0.125
const F8: f32 = 8.0;
const F1_8: f32 = 0.125;
/// `CCutsceneMgr::ms_running`
const CUTSCENE_RUNNING: usize = 0xB5F851;
/// `TheCamera` byte +0x? at 0xB6F065 tested by `PickUpShouldBeInvisible`
const CAM_BYTE_B6F065: usize = 0xB6F065;
/// `CModelInfo::ms_modelInfoPtrs`
const MODEL_INFO_PTRS: usize = 0xA9B0C8;
/// model indices tested (`ModelIndices`, set at load): kill frenzy, the German-only pickup, the three of
/// `WeaponForModel`
const MI_KILLFRENZY: usize = 0x8CD5D4;
const MI_GERMAN_PICKUP: usize = 0x8CD5F4;
const MI_WFM_A: usize = 0x8CD5C0;
const MI_WFM_B: usize = 0x8CD5C8;
const MI_WFM_C: usize = 0x8CD5BC;

orig! {
    /// `CWorld::Remove(CEntity*)` (0x563280)
    pub fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// `CEntity` scalar deleting destructor (vtable slot 0, `delete` with flag 1)
    pub fn entity_vdelete(e: GPtr<u8>, flag: i32) = thiscall vslot 0x0 => unported;
    /// `CRadar::ClearBlipForEntity(eBlipType, int32 handle)` (0x587C60)
    pub fn clear_blip_for_entity(ty: i32, handle: i32) = cdecl 0x587C60 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    pub fn get_weapon_info(ty: i32, skill: i32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CBaseModelInfo::GetModelType` (vtable slot 0x10)
    pub fn model_info_get_type(mi: GPtr<u8>) -> u8 = thiscall vslot 0x10 => unported;
    /// `CPedIntelligence::GetTaskJetPack` (0x601110)
    pub fn get_task_jet_pack(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x601110 => unported;
    /// `CTheScripts::IsPlayerOnAMission` (0x464D50)
    pub fn is_player_on_a_mission() -> u8 = cdecl 0x464D50 => unported;
    /// `CDarkel::FrenzyOnGoing` (0x43D1F0)
    pub fn frenzy_on_going() -> u8 = cdecl 0x43D1F0 => unported;
    /// `CLocalisation::KillFrenzy` (0x56D2A0)
    pub fn localisation_kill_frenzy() -> u8 = cdecl 0x56D2A0 => unported;
    /// `CLocalisation::GermanGame` (0x56D200)
    pub fn german_game() -> u8 = cdecl 0x56D200 => unported;
    /// `CGameLogic::IsCoopGameGoingOn` (0x441390)
    pub fn is_coop_game_going_on() -> u8 = cdecl 0x441390 => unported;
    /// `CWeapon::CanBeUsedFor2Player(eWeaponType)` (0x73B240)
    pub fn can_be_used_for_2_player(w: i32) -> u8 = cdecl 0x73B240 => unported;
    /// `CPickup::GetPosn(CVector* out)` (0x4549A0, thiscall)
    pub fn get_posn_call(this: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> = thiscall 0x4549A0 => get_posn;
    /// `CPickup::IsVisible` (0x454C70)
    pub fn is_visible_call(this: GPtr<u8>) -> u32 = thiscall 0x454C70 => is_visible;
    /// `CPickups::WeaponForModel(int32)` (0x454AE0)
    pub fn weapon_for_model_call(model: i32) -> i32 = cdecl 0x454AE0 => weapon_for_model;
    /// `CPickup::Remove` (0x4556C0, thiscall)
    pub fn remove_call(this: GPtr<u8>) = thiscall 0x4556C0 => remove;
    /// `CPickup::Update(CPlayerPed*, CVehicle*, int32 playerId)` (0x457410)
    pub fn pickup_update(this: GPtr<u8>, ped: GPtr<u8>, veh: GPtr<u8>, id: i32) -> u8 = thiscall 0x457410 => crate::world::mvpd_w5::pickup_update;
    /// `CPickup::GiveUsAPickUpObject(CObject**, int32 slot)` (0x4567E0)
    pub fn give_us_a_pick_up_object(this: GPtr<u8>, out: GPtr<GPtr<u8>>, slot: i32) = thiscall 0x4567E0 => give_us_a_pick_up_object_port;
    /// `CPools::MakeSureSlotInObjectPoolIsEmpty(int32)` (0x550080)
    pub fn make_sure_slot_in_object_pool_is_empty(slot: i32) = cdecl 0x550080 => unported;
    /// `CObject::operator new(uint32 size, int32 handle)` (0x5A1EF0)
    pub fn object_new_at(size: u32, handle: i32) -> GPtr<u8> = cdecl 0x5A1EF0 => unported;
    /// `CObject::CObject(int32 model, bool createRW)` (0x5A1D70): returns `this`
    pub fn object_ctor(o: GPtr<u8>, model: u32, rw: u32) -> GPtr<u8> = thiscall 0x5A1D70 => unported;
    /// `CObject::Create(int32 model, bool)` (0x5A1F60)
    pub fn object_create(model: u32, b: u32) -> GPtr<u8> = cdecl 0x5A1F60 => unported;
    /// `CPlaceable::SetOrientation(float x, float y, float z)` (0x439A80)
    pub fn placeable_set_orientation(e: GPtr<u8>, ox: u32, oy: u32, oz: u32) = thiscall 0x439A80 => unported;
    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70)
    pub fn matrix_update_rw_matrix(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD70 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40)
    pub fn simple_transform_update_rw_matrix(t: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x54EF40 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    pub fn entity_update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    pub fn entity_register_reference(e: GPtr<u8>, r: GPtr<GPtr<u8>>) = thiscall 0x571B70 => unported;
    /// `CPickup::PickUpShouldBeInvisible()` (0x454D20, AL)
    pub fn pick_up_should_be_invisible_call(this: GPtr<u8>) -> u8 = thiscall 0x454D20 => pick_up_should_be_invisible;
    /// `CWorld::Add(CEntity*)` (0x563220)
    pub fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CPickup::FindTextIndexForString(const char*)` (0x455500, AL used)
    pub fn find_text_index_for_string_call(s: GPtr<u8>) -> i32 = cdecl 0x455500 => find_text_index_for_string;
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPad::CollectPickupJustDown` (0x540A70)
    pub fn pad_collect_pickup_just_down(pad: GPtr<u8>) -> u8 = thiscall 0x540A70 => unported;
    /// `CPad::GetTarget` (0x540670)
    pub fn pad_get_target(pad: GPtr<u8>) -> u8 = thiscall 0x540670 => unported;
    /// `CPedIntelligence::FindTaskByType(int32)` (0x600EE0)
    pub fn find_task_by_type(intel: GPtr<u8>, ty: i32) -> GPtr<u8> = thiscall 0x600EE0 => unported;
    /// `CPickups::AddToCollectedPickupsArray(int32)` (0x455240)
    pub fn add_to_collected_pickups_array_call(i: i32) = cdecl 0x455240 => add_to_collected_pickups_array;
    /// CRT `_stricmp` (0x8229B6)
    pub fn stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
}

pub fn pickup(i: u32) -> GPtr<u8> {
    PICKUPS.ptr().cast::<u8>().byte_add((i as usize * PICKUP_SIZE) as isize)
}

fn reference(i: u32) -> u16 {
    rd::<u16>(pickup(i), 0x1A)
}

/// `(reference(i) << 16) | i`
fn handle_of(i: u32) -> i32 {
    ((u32::from(reference(i)) << 16) | i) as i32
}

/// The validity test of a handle (`-1` → none; reference must match).
fn valid_index(h: i32) -> Option<u32> {
    if h == -1 {
        return None;
    }
    let i = (h as u32) & 0xFFFF;
    if (h as u32 >> 16) as u16 != reference(i) {
        return None;
    }
    Some(i)
}

/// `CPickup` object removal: `CWorld::Remove`, `delete` (re-read pointer), clear.
fn get_rid_of_object(p: GPtr<u8>) {
    let o = rd::<GPtr<u8>>(p, 4);
    if o.is_null() {
        return;
    }
    world_remove(o);
    let o = rd::<GPtr<u8>>(p, 4);
    if !o.is_null() {
        entity_vdelete(o, 1);
    }
    wr::<GPtr<u8>>(p, 4, GPtr::NULL);
}

/// `thunk_FUN_0156f730` (stub 0x454920 → 0x156F730, called from `CPickup::Update`): true for 3, 4, 5.
#[hook(addr = 0x454920, conv = cdecl, name = "thunk_FUN_0156f730", expect = "E9 0B AE 11 01")]
pub fn is_type_3_4_5(v: i32) -> u32 {
    matches!(v, 3..=5) as u32
}

/// `CPickup::SetPosn(float x, float y, float z)` (0x454960, thiscall, `ret 0xC`): int16 of `_ftol(v * 8)`.
#[hook(addr = 0x454960, conv = thiscall, name = "CPickup::SetPosn", expect = "D9 44 24 04 56", this = "CPickup")]
pub fn set_posn(this: GPtr<u8>, px: f32, py: f32, pz: f32) {
    wr::<i16>(this, 0x10, ftol32(x(px).fmul_m(F8)) as i16);
    wr::<i16>(this, 0x12, ftol32(x(py).fmul_m(F8)) as i16);
    wr::<i16>(this, 0x14, ftol32(x(pz).fmul_m(F8)) as i16);
}

/// `CPickup::GetPosn(CVector* out)` (0x4549A0, thiscall, `ret 4`): `out = pos * 0.125`; returns `out`.
#[hook(addr = 0x4549A0, conv = thiscall, name = "CPickup::GetPosn", expect = "51 0F BF 41 14", this = "CPickup")]
pub fn get_posn(this: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> {
    let c = |o: isize| xi(i32::from(rd::<i16>(this, o))).fmul_m(F1_8).to_f32();
    out.write(c(0x10));
    out.wrapping_index(1).write(c(0x12));
    out.wrapping_index(2).write(c(0x14));
    out
}

/// `CPickups::Init` (stub 0x454A70 → 0x1564650): `NumMessages` = 0; every pickup: type 0, reference 1,
/// object null; the collected array and its index 0; `DisplayHelpMessage` = 10.
#[hook(addr = 0x454A70, conv = cdecl, name = "CPickups::Init", expect = "E9 DB FB 10 01")]
pub fn init() {
    NUM_MESSAGES.set(0);
    for i in 0..NUM_PICKUPS as u32 {
        let p = pickup(i);
        wr::<u8>(p, 0x1C, 0);
        wr::<u16>(p, 0x1A, 1);
        wr::<u32>(p, 4, 0);
    }
    PICKUPS_COLLECTED.set([0; NUM_COLLECTED]);
    COLLECTED_PICKUP_INDEX.set(0);
    DISPLAY_HELP_MESSAGE.set(10);
}

/// `CPickups::ModelForWeapon(eWeaponType)` (stub 0x454AC0 → 0x1568BD0): `GetWeaponInfo(w, 1)->m_nModelId1`.
#[hook(addr = 0x454AC0, conv = cdecl, name = "CPickups::ModelForWeapon", expect = "E9 0B 41 11 01")]
pub fn model_for_weapon(w: i32) -> i32 {
    rd::<i32>(get_weapon_info(w, 1), 0xC)
}

/// `CPickups::WeaponForModel(int32 model)` (0x454AE0): 0x30 for the models at 0x8CD5C0 / 0x8CD5BC, 0x2F
/// for the one at 0x8CD5C8 and 0x172; else (model != -1) the weapon type (+0x24) of a weapon model info
/// (type 4); else 0.
#[hook(addr = 0x454AE0, conv = cdecl, name = "CPickups::WeaponForModel", expect = "90 90 E9 40 CA FA FF")]
pub fn weapon_for_model(model: i32) -> i32 {
    let mi = |a: usize| i32::from(g::<u16>(a).read());
    if model == mi(MI_WFM_A) {
        return 0x30;
    }
    if model == mi(MI_WFM_B) {
        return 0x2F;
    }
    if model == mi(MI_WFM_C) {
        return 0x30;
    }
    if model == 0x172 {
        return 0x2F;
    }
    if model == -1 {
        return 0;
    }
    let info = g::<GPtr<u8>>(MODEL_INFO_PTRS).wrapping_index(model).read();
    if model_info_get_type(info) == 4 {
        return rd::<i32>(info, 0x24);
    }
    0
}

/// `CPickups::IsPickUpPickedUp(int32 handle)` (stub 0x454B40 → 0x156DA60): finds and clears the handle
/// in the collected array.
#[hook(addr = 0x454B40, conv = cdecl, name = "CPickups::IsPickUpPickedUp", expect = "E9 1B 8F 11 01")]
pub fn is_pick_up_picked_up(h: i32) -> u8 {
    let a = PICKUPS_COLLECTED.ptr().cast::<i32>();
    for i in 0..NUM_COLLECTED as i32 {
        if a.wrapping_index(i).read() == h {
            a.wrapping_index(i).write(0);
            return 1;
        }
    }
    0
}

/// The camera position `IsVisible` uses: `[0xB6F03C] + 0x30` or the vector at 0xB6F02C.
fn cam_pos() -> GPtr<f32> {
    super::common::cam_pos()
}

/// `CPickup::IsVisible` (0x454C70, thiscall): 2D distance camera → pickup below 100 (EAX 0/1).
#[hook(addr = 0x454C70, conv = thiscall, name = "CPickup::IsVisible", preserve = "edx", expect = "83 EC 10 0F BF 41 10", this = "CPickup")]
pub fn is_visible(this: GPtr<u8>) -> u32 {
    let px = xi(i32::from(rd::<i16>(this, 0x10))).fmul_m(F1_8);
    let py = xi(i32::from(rd::<i16>(this, 0x12))).fmul_m(F1_8).to_f32();
    let c = cam_pos();
    let a = x(c.read()).fsub(px);
    let b = x(c.wrapping_index(1).read()).fsub_m(py);
    let d = b.fmul(b).fadd(a.fmul(a)).fsqrt();
    // fcomp 100; test ah,5; jp: false unless Less
    (!d.fcom(x(100.0)).test_parity(0x05)) as u32
}

/// `CPickup::GetRidOfObjects` (0x454CF0, thiscall).
#[hook(addr = 0x454CF0, conv = thiscall, name = "CPickup::GetRidOfObjects", expect = "56 8B F1 8B 46 04", this = "CPickup")]
pub fn get_rid_of_objects(this: GPtr<u8>) {
    get_rid_of_object(this);
}

/// 2D length of `(a.x - c0, a.y - c1)`, summed y² + x² (both orders occur; the sum commutes).
fn dist2d(dx: X87, dy: X87) -> X87 {
    dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt()
}

/// `CPickup::PickUpShouldBeInvisible` (0x454D20, thiscall), see the module docs / spec.
#[hook(addr = 0x454D20, conv = thiscall, name = "CPickup::PickUpShouldBeInvisible", expect = "A0 51 F8 B5 00", this = "CPickup")]
pub fn pick_up_should_be_invisible(this: GPtr<u8>) -> u8 {
    if g::<u8>(CUTSCENE_RUNNING).read() != 0 {
        return 1;
    }
    let model = rd::<u16>(this, 0x18);
    if model == g::<u16>(MI_KILLFRENZY).read()
        && (is_player_on_a_mission() != 0 || frenzy_on_going() != 0 || localisation_kill_frenzy() == 0)
    {
        return 1;
    }
    if rd::<u8>(this, 0x1C) == 0x15 && !find_player_ped(1).is_null() {
        return 1;
    }
    if rd::<u16>(this, 0x18) == g::<u16>(MI_GERMAN_PICKUP).read() && german_game() != 0 {
        let mut p = [0f32; 3];
        let pp = get_posn_call(this, GPtr::from_raw(p.as_mut_ptr()));
        // 0x859B08 = -1658.0 (bits 0xC4CF4000 at 0x859B04 is y? see below), 0x859B04
        let c0 = f32::from_bits(0x44B8_E000); // [0x859B08] = 1479.0
        let c1 = f32::from_bits(0xC4CF_4000); // [0x859B04] = -1658.0
        let a = x(pp.read()).fsub_m(c0);
        let b = x(pp.wrapping_index(1).read()).fsub_m(c1);
        if !dist2d(a, b).fcom(x(10.0)).test_parity(0x05) {
            return 1;
        }
        let v = [2511.0f32, 1208.0, 0.0];
        let mut q = [0f32; 3];
        let qp = get_posn_call(this, GPtr::from_raw(q.as_mut_ptr()));
        // VectorSub (0x40FE60, inline helper): f32 components
        let dx = x(qp.read()).fsub_m(v[0]).to_f32();
        let dy = x(qp.wrapping_index(1).read()).fsub_m(v[1]).to_f32();
        if !dist2d(x(dx), x(dy)).fcom(x(10.0)).test_parity(0x05) {
            return 1;
        }
    }
    if g::<u8>(CAM_BYTE_B6F065).read() != 0 && rd::<u8>(this, 0x1C) != 0x10 && rd::<u16>(this, 0x18) != 0x16E {
        return 1;
    }
    if is_coop_game_going_on() != 0 {
        let w = weapon_for_model_call(i32::from(rd::<u16>(this, 0x18)));
        if can_be_used_for_2_player(w) == 0 {
            return 1;
        }
    }
    0
}

/// `CPickups::FindPickUpForThisObject(CObject*)` (stub 0x4551C0 → 0x156D870): the first pickup in use
/// whose object is `obj`, else pickup 0.
#[hook(addr = 0x4551C0, conv = cdecl, name = "CPickups::FindPickUpForThisObject", expect = "E9 AB 86 11 01")]
pub fn find_pick_up_for_this_object(obj: GPtr<u8>) -> GPtr<u8> {
    for i in 0..NUM_PICKUPS as u32 {
        let p = pickup(i);
        if rd::<u8>(p, 0x1C) != 0 && rd::<GPtr<u8>>(p, 4) == obj {
            return p;
        }
    }
    pickup(0)
}

/// `CPickups::PassTime(uint32)` (0x455200): regeneration times of pickups whose type is neither 0 nor
/// 0x10 decrease by `t`, floored at 0.
#[hook(addr = 0x455200, conv = cdecl, name = "CPickups::PassTime", expect = "8B 54 24 04 B8 CC 88 97 00")]
pub fn pass_time(t: u32) {
    for i in 0..NUM_PICKUPS as u32 {
        let p = pickup(i);
        let ty = rd::<u8>(p, 0x1C);
        if ty == 0 || ty == 0x10 {
            continue;
        }
        let r = rd::<u32>(p, 0xC);
        wr::<u32>(p, 0xC, r.saturating_sub(t));
    }
}

/// `CPickups::AddToCollectedPickupsArray(int32 index)` (stub 0x455240 → 0x1561580).
#[hook(addr = 0x455240, conv = cdecl, name = "CPickups::AddToCollectedPickupsArray", expect = "E9 3B C3 10 01")]
pub fn add_to_collected_pickups_array(i: i32) {
    let v = ((u32::from(rd::<u16>(PICKUPS.ptr().cast::<u8>(), ((i as u32) << 5) as i32 as isize + 0x1A)) << 16) | i as u32) as i32;
    let idx = COLLECTED_PICKUP_INDEX.get();
    PICKUPS_COLLECTED.ptr().cast::<i32>().wrapping_index(i32::from(idx)).write(v);
    let n = idx.wrapping_add(1);
    COLLECTED_PICKUP_INDEX.set(n);
    if n >= 0x14 {
        COLLECTED_PICKUP_INDEX.set(0);
    }
}

/// `CPickups::GetUniquePickupIndex(int32 index)` (0x455280).
#[hook(addr = 0x455280, conv = cdecl, name = "CPickups::GetUniquePickupIndex", expect = "8B 4C 24 04 8B C1")]
pub fn get_unique_pickup_index(i: i32) -> i32 {
    let r = rd::<u16>(PICKUPS.ptr().cast::<u8>(), ((i as u32) << 5) as i32 as isize + 0x1A);
    ((u32::from(r) << 16) | i as u32) as i32
}

/// `CPickups::GetActualPickupIndex(int32 handle)` (stub 0x4552A0 → 0x1564BB0): the index, or -1.
#[hook(addr = 0x4552A0, conv = cdecl, name = "CPickups::GetActualPickupIndex", expect = "E9 0B F9 10 01")]
pub fn get_actual_pickup_index(h: i32) -> i32 {
    match valid_index(h) {
        Some(i) => i as i32,
        None => -1,
    }
}

/// `CPickups::RemovePickupObjects` (0x455470): every pickup in use with an object: object removed and
/// deleted, flag bit 3 cleared.
#[hook(addr = 0x455470, conv = cdecl, name = "CPickups::RemovePickupObjects", expect = "56 BE C4 88 97 00")]
pub fn remove_pickup_objects() {
    for i in 0..NUM_PICKUPS as u32 {
        let p = pickup(i);
        if rd::<u8>(p, 0x1C) == 0 || rd::<GPtr<u8>>(p, 4).is_null() {
            continue;
        }
        get_rid_of_object(p);
        wr::<u8>(p, 0x1D, rd::<u8>(p, 0x1D) & 0xF7);
    }
}

/// `CPickups::PlayerCanPickUpThisWeaponTypeAtThisMoment(eWeaponType)` (stub 0x4554C0 → 0x156A950):
/// true when weapon flag bit 1 (+0x18) is set or the player has no jetpack task.
#[hook(addr = 0x4554C0, conv = cdecl, name = "CPickups::PlayerCanPickUpThisWeaponTypeAtThisMoment", expect = "E9 8B 54 11 01")]
pub fn player_can_pick_up_this_weapon_type_at_this_moment(w: i32) -> u8 {
    let info = get_weapon_info(w, 1);
    if (rd::<u32>(info, 0x18) >> 1) & 1 != 0 {
        return 1;
    }
    let ped = find_player_ped(-1);
    if !get_task_jet_pack(rd::<GPtr<u8>>(ped, 0x47C)).is_null() {
        return 0;
    }
    1
}

/// `CPickup::FindTextIndexForString(const char*)` (stub 0x455500 → 0x156EC50): null → 0, "PROP_3" → 1,
/// "PROP_4" → 2 (case-insensitive), else 0.
#[hook(addr = 0x455500, conv = cdecl, name = "CPickup::FindTextIndexForString", expect = "E9 4B 97 11 01")]
pub fn find_text_index_for_string(s: GPtr<u8>) -> i32 {
    if s.is_null() {
        return 0;
    }
    if stricmp(g(0x859B3C), s) == 0 {
        return 1;
    }
    if stricmp(g(0x859B34), s) == 0 { 2 } else { 0 }
}

/// `CPickup::FindStringForTextIndex(int32)` (stub 0x455540 → 0x1565750): 1 → "PROP_3", 2 → "PROP_4",
/// else "FESZ_CA".
#[hook(addr = 0x455540, conv = cdecl, name = "CPickup::FindStringForTextIndex", expect = "E9 0B 02 11 01")]
pub fn find_string_for_text_index(i: i32) -> GPtr<u8> {
    match i {
        1 => g(0x859B3C),
        2 => g(0x859B34),
        _ => g(0x859B44),
    }
}

/// `CPickups::UpdateMoneyPerDay(int32 handle, uint16)` (stub 0x455680 → 0x156B360).
#[hook(addr = 0x455680, conv = cdecl, name = "CPickups::UpdateMoneyPerDay", expect = "E9 DB 5C 11 01")]
pub fn update_money_per_day(h: i32, money: u16) {
    if let Some(i) = valid_index(h) {
        wr::<u16>(pickup(i), 0x16, money);
    }
}

/// `CPickup::Remove` (0x4556C0, thiscall): radar blip of its handle cleared, object removed and
/// deleted, type 0, flag bit 0 set.
#[hook(addr = 0x4556C0, conv = thiscall, name = "CPickup::Remove", expect = "56 8B F1 8B C6", this = "CPickup")]
pub fn remove(this: GPtr<u8>) {
    let i = ((this.addr() as i32).wrapping_sub(PICKUPS.ptr().addr() as i32)) >> 5;
    clear_blip_for_entity(7, handle_of(i as u32));
    get_rid_of_object(this);
    let f = rd::<u8>(this, 0x1D) | 1;
    wr::<u8>(this, 0x1C, 0);
    wr::<u8>(this, 0x1D, f);
}

/// `CPickups::TestForPickupsInBubble(CVector pos, float radius)` (0x456450): any pickup (in use or not)
/// whose 3D distance is below `radius`.
#[hook(addr = 0x456450, conv = cdecl, name = "CPickups::TestForPickupsInBubble", expect = "83 EC 1C B9 D2 88 97 00")]
pub fn test_for_pickups_in_bubble(px: f32, py: f32, pz: f32, radius: f32) -> u8 {
    for i in 0..NUM_PICKUPS as u32 {
        let p = pickup(i);
        let c = |o: isize| xi(i32::from(rd::<i16>(p, o))).fmul_m(F1_8);
        let (cx, cy) = (c(0x10), c(0x12));
        let cz = c(0x14).to_f32();
        let dxf = cx.fsub_m(px).to_f32();
        let b = cy.fsub_m(py);
        let cc = x(cz).fsub_m(pz);
        let d = cc.fmul(cc).fadd(b.fmul(b)).fadd(x(dxf).fmul_m(dxf)).fsqrt();
        if !d.fcom(x(radius)).test_parity(0x05) {
            return 1;
        }
    }
    0
}

/// `CPickups::GetNewUniquePickupIndex(int32 index)` (0x456A30): reference + 1 (wrapping to 1 from
/// 0xFFFE), the new handle.
#[hook(addr = 0x456A30, conv = cdecl, name = "CPickups::GetNewUniquePickupIndex", expect = "8B 54 24 04 8B C2")]
pub fn get_new_unique_pickup_index(i: i32) -> i32 {
    let p = PICKUPS.ptr().cast::<u8>().byte_add(((i as u32) << 5) as i32 as isize + 0x1A).cast::<u16>();
    let r = p.read();
    let n = if r < 0xFFFE { r + 1 } else { 1 };
    p.write(n);
    ((u32::from(n) << 16) | i as u32) as i32
}

/// `CPickups::RemoveMissionPickUps` (0x456DE0): pickups of type 0x16 removed as `CPickup::Remove` does
/// (inlined).
#[hook(addr = 0x456DE0, conv = cdecl, name = "CPickups::RemoveMissionPickUps", expect = "56 57 BE C4 88 97 00")]
pub fn remove_mission_pick_ups() {
    for i in 0..NUM_PICKUPS as u32 {
        let p = pickup(i);
        if rd::<u8>(p, 0x1C) != 0x16 {
            continue;
        }
        clear_blip_for_entity(7, handle_of(i));
        get_rid_of_object(p);
        let f = rd::<u8>(p, 0x1D) | 1;
        wr::<u8>(p, 0x1C, 0);
        wr::<u8>(p, 0x1D, f);
    }
}

/// `CPickups::RemovePickUp(int32 handle)` (0x4573D0): `CPickup::Remove` of a valid handle.
#[hook(addr = 0x4573D0, conv = cdecl, name = "CPickups::RemovePickUp", expect = "8B 4C 24 04 83 F9 FF")]
pub fn remove_pick_up(h: i32) {
    if let Some(i) = valid_index(h) {
        remove_call(pickup(i));
    }
}

/// `CPickups::CollectPickupBuffer` / `PlayerOnWeaponPickup`
const COLLECT_PICKUP_BUFFER: usize = 0x97D644;
const PLAYER_ON_WEAPON_PICKUP: usize = 0x97D640;

/// `max(v - 1, 0)` as the original's `dec; setl; dec; and` sequence.
fn dec_floor0(v: i32) -> i32 {
    let d = v.wrapping_sub(1);
    if d < 0 { 0 } else { d }
}

/// Neither the 0x2BD nor the 0x640 task (both looked up on player 0's intelligence).
fn player0_free() -> bool {
    let ped = g::<GPtr<u8>>(0xB7CD98).read();
    if !find_task_by_type(rd::<GPtr<u8>>(ped, 0x47C), 0x2BD).is_null() {
        return false;
    }
    let ped = g::<GPtr<u8>>(0xB7CD98).read();
    find_task_by_type(rd::<GPtr<u8>>(ped, 0x47C), 0x640).is_null()
}

/// `CPickups::Update` (0x458DE0): not during replay playback. A 1/32 slice of the pickups (by frame
/// counter) updates visibility (flag bit 3; creates the object (`GiveUsAPickUpObject` + `CWorld::Add`)
/// when visible and not disabled, removes it when not visible); the collect-button buffer and the
/// weapon-pickup counter count down; then a 1/6 slice runs `CPickup::Update` for player 1 (when free
/// of the two tasks) and for player 2 (when present and player 1's intelligence is free of them, as
/// the original tests), recording collected pickups.
#[hook(addr = 0x458DE0, conv = cdecl, name = "CPickups::Update", expect = "51 80 3D 88 30 A4 00 01")]
pub fn update() {
    if g::<u8>(0xA43088).read() == 1 {
        return;
    }
    let fc = g::<u32>(0xB7CB4C).read();
    let c = ((fc & 0x1F) * 0x26C) as i32;
    let (start, end) = (c / 32, (c + 0x26C) / 32);
    for i in start..end {
        let p = pickup(i as u32);
        if rd::<u8>(p, 0x1C) == 0 {
            continue;
        }
        let vis = is_visible_call(p);
        let f = rd::<u8>(p, 0x1D);
        if vis as u8 != 0 {
            let f = f | 8;
            wr::<u8>(p, 0x1D, f);
            if f & 1 == 0 && rd::<GPtr<u8>>(p, 4).is_null() {
                give_us_a_pick_up_object(p, p.byte_add(4).cast(), -1);
                let o = rd::<GPtr<u8>>(p, 4);
                if !o.is_null() {
                    world_add(o);
                }
            }
        } else {
            wr::<u8>(p, 0x1D, f & 0xF7);
            get_rid_of_object(p);
        }
    }
    let buf = g::<i32>(COLLECT_PICKUP_BUFFER);
    if pad_collect_pickup_just_down(get_pad(0)) != 0 {
        buf.write(6);
    } else {
        buf.write(dec_floor0(buf.read()));
    }
    let w = g::<i32>(PLAYER_ON_WEAPON_PICKUP);
    if w.read() != 0 {
        w.write(dec_floor0(w.read()));
    }
    if pad_get_target(get_pad(0)) != 0 {
        buf.write(0);
    }
    let p1 = player0_free();
    let p2 = !g::<GPtr<u8>>(0xB7CF28).read().is_null() && player0_free();
    let r = (fc % 6) as i32 * 0x26C;
    let (start, end) = (r / 6, (r + 0x26C) / 6);
    for i in start..end {
        let q = pickup(i as u32);
        if rd::<u8>(q, 0x1C) == 0 || rd::<u8>(q, 0x1D) & 8 == 0 {
            continue;
        }
        if p1 {
            let focus = i32::from(super::cworld::PLAYER_IN_FOCUS.get());
            let veh = find_player_vehicle(-1, 0);
            let ped = find_player_ped(-1);
            if pickup_update(q, ped, veh, focus) != 0 {
                let h = handle_of(i as u32);
                let idx = COLLECTED_PICKUP_INDEX.get();
                PICKUPS_COLLECTED.ptr().cast::<i32>().wrapping_index(i32::from(idx)).write(h);
                let n = idx.wrapping_add(1);
                COLLECTED_PICKUP_INDEX.set(n);
                if n >= 0x14 {
                    COLLECTED_PICKUP_INDEX.set(0);
                }
                continue;
            }
        }
        if p2 && rd::<u8>(q, 0x1C) != 0 && rd::<u8>(q, 0x1D) & 8 != 0 {
            let veh = find_player_vehicle(1, 0);
            let ped = find_player_ped(1);
            if pickup_update(q, ped, veh, 1) != 0 {
                add_to_collected_pickups_array_call(i);
            }
        }
    }
}

/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84)
const TIME_MS: usize = 0xB7CB84;

/// The first pickup index in `range` whose type byte (+0x1C) satisfies `f`.
fn find_slot(mut range: impl Iterator<Item = usize>, f: impl Fn(u8) -> bool) -> Option<usize> {
    range.find(|&i| f(g::<u8>(0x9788DC + i * PICKUP_SIZE).read()))
}

/// `CPickups::GenerateNewOne(CVector pos, uint32 model, uint8 type, uint32 ammo, uint32 moneyPerDay, bool
/// isEmpty, const char* message)` (0x456F20, cdecl). The slot: for type 0xB / 0xD or `isEmpty` the last
/// free one (type 0, from 619 down), else (or when none) the first free one; then the first of type 8,
/// then the first of type 4 or 5 (none → -1); a reused slot loses its object (`CWorld::Remove`, the
/// re-read pointer deleted through vtable slot 0 with flag 1 when non-null, +4 = 0). Fields in the
/// exe's order: ammo +8, money per day +0x16, flags +0x1D = (flags & 0xF8) | isEmpty << 1, type +0x1C,
/// revenue +0 = 0, regeneration time +0xC = now (+20000 for type 4, +120000 for 5, +30000 for 8; types
/// 9 / 10 become 9 with now + 1500; 0xB / 0xC become 0xB with now + 1500), model +0x18 (int16), flags
/// bits 4-6 = `FindTextIndexForString(message)`, position +0x10 / +0x12 / +0x14 = `_ftol(c * 8.0)` (x87),
/// flags bit 3 = `IsVisible()`, object +4 = 0; a visible pickup gets `GiveUsAPickUpObject(&object, -1)`
/// and `CWorld::Add(object)` when one was made. The reference +0x1A goes up by one (restarts at 1 from
/// 0xFFFE); returns `reference << 16 | index`.
#[hook(addr = 0x456f20, conv = cdecl, name = "CPickups::GenerateNewOne", expect = "8A 4C 24 14 80 F9 0D",
       unverified = "mvpi-world-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "creates the pickup's CObject (object pool) when visible")]
#[allow(clippy::too_many_arguments)]
pub fn generate_new_one(
    px: f32,
    py: f32,
    pz: f32,
    model: u32,
    ty: u32,
    ammo: u32,
    money_per_day: u32,
    is_empty: u32,
    message: GPtr<u8>,
) -> i32 {
    let ty = ty as u8;
    let empty = is_empty as u8;
    let mut slot = None;
    if ty == 0xD || ty == 0xB || empty != 0 {
        slot = find_slot((0..NUM_PICKUPS).rev(), |t| t == 0);
    }
    if slot.is_none() {
        slot = find_slot(0..NUM_PICKUPS, |t| t == 0);
    }
    if slot.is_none() {
        let reuse = find_slot(0..NUM_PICKUPS, |t| t == 8).or_else(|| find_slot(0..NUM_PICKUPS, |t| t == 4 || t == 5));
        let Some(i) = reuse else {
            return -1;
        };
        let p = PICKUPS.ptr().cast::<u8>().byte_add((i * PICKUP_SIZE) as isize);
        let obj = rd::<GPtr<u8>>(p, 4);
        if !obj.is_null() {
            world_remove(obj);
            let obj = rd::<GPtr<u8>>(p, 4);
            if !obj.is_null() {
                entity_vdelete(obj, 1);
            }
            wr::<u32>(p, 4, 0);
        }
        slot = Some(i);
    }
    let i = slot.unwrap_or_default();
    let p = PICKUPS.ptr().cast::<u8>().byte_add((i * PICKUP_SIZE) as isize);
    wr::<u32>(p, 8, ammo);
    wr::<u16>(p, 0x16, money_per_day as u16);
    let fl = ((empty & 1) << 1) | (rd::<u8>(p, 0x1D) & 0xF8);
    let now = g::<u32>(TIME_MS).read();
    wr::<u8>(p, 0x1C, ty);
    wr::<u32>(p, 0, 0);
    wr::<u32>(p, 0xC, now);
    wr::<u8>(p, 0x1D, fl);
    match ty {
        4 => wr::<u32>(p, 0xC, now.wrapping_add(20000)),
        5 => wr::<u32>(p, 0xC, now.wrapping_add(120000)),
        8 => wr::<u32>(p, 0xC, now.wrapping_add(30000)),
        9 | 10 => {
            wr::<u8>(p, 0x1C, 9);
            wr::<u32>(p, 0xC, now.wrapping_add(1500));
        }
        0xB | 0xC => {
            wr::<u8>(p, 0x1C, 0xB);
            wr::<u32>(p, 0xC, now.wrapping_add(1500));
        }
        _ => {}
    }
    wr::<u16>(p, 0x18, model as u16);
    let text = find_text_index_for_string_call(message) as u8;
    let fl = rd::<u8>(p, 0x1D);
    wr::<u8>(p, 0x1D, (fl & !0x70) | ((text << 4) & 0x70));
    wr::<u16>(p, 0x10, ftol32(x(px).fmul_m(F8)) as u16);
    wr::<u16>(p, 0x12, ftol32(x(py).fmul_m(F8)) as u16);
    wr::<u16>(p, 0x14, ftol32(x(pz).fmul_m(F8)) as u16);
    let vis = is_visible_call(p) as u8;
    let fl = rd::<u8>(p, 0x1D);
    let fl = (fl & !8) | ((vis << 3) & 8);
    wr::<u8>(p, 0x1D, fl);
    wr::<u32>(p, 4, 0);
    if fl & 8 != 0 {
        give_us_a_pick_up_object(p, p.byte_add(4).cast(), -1);
        let obj = rd::<GPtr<u8>>(p, 4);
        if !obj.is_null() {
            world_add(obj);
        }
    }
    let r = rd::<u16>(p, 0x1A);
    let r = if r < 0xFFFE { r + 1 } else { 1 };
    wr::<u16>(p, 0x1A, r);
    ((u32::from(r) << 16) | i as u32) as i32
}

/// `CCutsceneMgr::ms_cutsceneLoadStatus` (int32)
const CUTSCENE_LOAD_STATUS: usize = 0xB5F84C;
/// `ModelIndices` entries 0x8CD750 (gets entity flag 0x100000), 0x8CD5CC and 0x8CD5F0 (carry the ammo
/// byte into +0x13D)
const MI_8CD750: usize = 0x8CD750;
const MI_8CD5CC: usize = 0x8CD5CC;
const MI_8CD5F0: usize = 0x8CD5F0;

/// `CPickup::GiveUsAPickUpObject(CObject** obj, int32 slot)` (0x4567E0, thiscall, `ret 8`): `*obj = 0`;
/// nothing more while the cutscene load status is 2. A weapon model (model type 4, vtable slot 0x10)
/// calls `GetWeaponInfo(its weapon type +0x24, 1)` (result unused). With `slot` >= 0:
/// `MakeSureSlotInObjectPoolIsEmpty(slot)`, then, unless `*obj` is set, `operator new(0x17C, slot << 8)`
/// and `CObject(model, false)` (null when the allocation fails); else `CObject::Create(model, false)`.
/// Null → done. The object: type +0x13C = 5; the pickup position (`GetPosn`, x87 loads) into the matrix
/// (+0x30..+0x38) or the placement (+4..+0xC); model `[0x8CD750]` sets entity flag 0x100000;
/// `SetOrientation(0, 0, -pi/2)`; with an RW object the RW matrix updated (`CMatrix::UpdateRwMatrix` or
/// `CSimpleTransform::UpdateRwMatrix`) and `UpdateRwFrame()`; physical flags +0x40 bit 1 cleared and bit
/// 23 set, entity flags +0x1C bit 0 cleared, object flags +0x140 bit 0 set and bit 1 = the pickup's flag
/// bit 1, entity bit 21 set, object bit 25 = `PickUpShouldBeInvisible()`, entity bit 31 set,
/// `RegisterReference(obj)`; +0x13D = the ammo's low byte for models `[0x8CD5CC]` / `[0x8CD5F0]`, else 0.
/// Pickup type 7: object bit 3 and physical bit 29; type 0x12: object bit 2 and +0x13E = ammo / 5
/// (signed). Every object access re-reads `*obj`.
#[hook(addr = 0x4567e0, conv = thiscall, name = "CPickup::GiveUsAPickUpObject", expect = "83 EC 0C 56 8B 74 24 14",
       unverified = "mvpi-world-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "creates a CObject (object pool, RW clump)")]
pub fn give_us_a_pick_up_object_port(this: GPtr<u8>, obj: GPtr<GPtr<u8>>, slot: i32) {
    obj.write(GPtr::NULL);
    if g::<i32>(CUTSCENE_LOAD_STATUS).read() == 2 {
        return;
    }
    let model = u32::from(rd::<u16>(this, 0x18));
    let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + model as usize * 4).read();
    if model_info_get_type(mi) == 4 {
        let mi = g::<GPtr<u8>>(MODEL_INFO_PTRS + model as usize * 4).read();
        get_weapon_info(rd::<i32>(mi, 0x24), 1);
    }
    let o = if slot >= 0 {
        make_sure_slot_in_object_pool_is_empty(slot);
        let cur = obj.read();
        if !cur.is_null() {
            cur
        } else {
            let mem = object_new_at(0x17C, slot << 8);
            let o = if mem.is_null() { GPtr::NULL } else { object_ctor(mem, u32::from(rd::<u16>(this, 0x18)), 0) };
            obj.write(o);
            o
        }
    } else {
        let o = object_create(u32::from(rd::<u16>(this, 0x18)), 0);
        obj.write(o);
        o
    };
    if o.is_null() {
        return;
    }
    wr::<u8>(o, 0x13C, 5);
    let mut tmp = [0f32; 3];
    let p = get_posn_call(this, GPtr::from_raw(tmp.as_mut_ptr()));
    let (pz, py, px) = (
        x(p.wrapping_index(2).read()).to_f32(),
        x(p.wrapping_index(1).read()).to_f32(),
        x(p.read()).to_f32(),
    );
    let o = obj.read();
    if !rd::<GPtr<u8>>(o, 0x14).is_null() {
        wr::<f32>(rd::<GPtr<u8>>(o, 0x14), 0x30, px);
        wr::<f32>(rd::<GPtr<u8>>(o, 0x14), 0x34, py);
        wr::<f32>(rd::<GPtr<u8>>(o, 0x14), 0x38, pz);
    } else {
        wr::<f32>(o, 4, px);
        wr::<f32>(o, 8, py);
        wr::<f32>(o, 0xC, pz);
    }
    let o = obj.read();
    if i32::from(rd::<i16>(o, 0x22)) == i32::from(g::<u16>(MI_8CD750).read()) {
        wr::<u32>(o, 0x1C, rd::<u32>(o, 0x1C) | 0x10_0000);
    }
    placeable_set_orientation(obj.read(), 0, 0, 0xBFC9_0FDB);
    let o = obj.read();
    let rw = rd::<GPtr<u8>>(o, 0x18);
    if !rw.is_null() {
        let rwm = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
        let m = rd::<GPtr<u8>>(o, 0x14);
        if m.is_null() {
            simple_transform_update_rw_matrix(o.byte_add(4), rwm);
        } else {
            matrix_update_rw_matrix(m, rwm);
        }
    }
    entity_update_rw_frame(obj.read());
    let upd = |off: isize, f: &dyn Fn(u32) -> u32| {
        let o = obj.read();
        wr::<u32>(o, off, f(rd::<u32>(o, off)));
    };
    upd(0x40, &|v| v & 0xFFFF_FFFD);
    upd(0x40, &|v| v | 0x80_0000);
    upd(0x1C, &|v| v & 0xFFFF_FFFE);
    upd(0x140, &|v| v | 1);
    let pf = u32::from(rd::<u8>(this, 0x1D));
    upd(0x140, &|v| v ^ ((pf ^ v) & 2));
    upd(0x1C, &|v| v | 0x20_0000);
    let inv = u32::from(pick_up_should_be_invisible_call(this));
    upd(0x140, &|v| v ^ (((inv << 25) ^ v) & 0x200_0000));
    upd(0x1C, &|v| v | 0x8000_0000);
    entity_register_reference(obj.read(), obj);
    let m = rd::<u16>(this, 0x18);
    if m == g::<u16>(MI_8CD5CC).read() || m == g::<u16>(MI_8CD5F0).read() {
        wr::<u8>(obj.read(), 0x13D, rd::<u8>(this, 8));
    } else {
        wr::<u8>(obj.read(), 0x13D, 0);
    }
    match rd::<u8>(this, 0x1C) {
        7 => {
            upd(0x140, &|v| v | 8);
            upd(0x40, &|v| v | 0x2000_0000);
        }
        0x12 => {
            upd(0x140, &|v| v | 4);
            let ammo = rd::<i32>(this, 8);
            wr::<u16>(obj.read(), 0x13E, (ammo / 5) as u16);
        }
        _ => {}
    }
}

pub const HOOKS: &[HookReg] = &[
    HOOK_GIVE_US_A_PICK_UP_OBJECT_PORT,
    HOOK_GENERATE_NEW_ONE,
    HOOK_UPDATE,
    HOOK_IS_TYPE_3_4_5,
    HOOK_SET_POSN,
    HOOK_GET_POSN,
    HOOK_INIT,
    HOOK_MODEL_FOR_WEAPON,
    HOOK_WEAPON_FOR_MODEL,
    HOOK_IS_PICK_UP_PICKED_UP,
    HOOK_IS_VISIBLE,
    HOOK_GET_RID_OF_OBJECTS,
    HOOK_PICK_UP_SHOULD_BE_INVISIBLE,
    HOOK_FIND_PICK_UP_FOR_THIS_OBJECT,
    HOOK_PASS_TIME,
    HOOK_ADD_TO_COLLECTED_PICKUPS_ARRAY,
    HOOK_GET_UNIQUE_PICKUP_INDEX,
    HOOK_GET_ACTUAL_PICKUP_INDEX,
    HOOK_REMOVE_PICKUP_OBJECTS,
    HOOK_PLAYER_CAN_PICK_UP_THIS_WEAPON_TYPE_AT_THIS_MOMENT,
    HOOK_FIND_TEXT_INDEX_FOR_STRING,
    HOOK_FIND_STRING_FOR_TEXT_INDEX,
    HOOK_UPDATE_MONEY_PER_DAY,
    HOOK_REMOVE,
    HOOK_TEST_FOR_PICKUPS_IN_BUBBLE,
    HOOK_GET_NEW_UNIQUE_PICKUP_INDEX,
    HOOK_REMOVE_MISSION_PICK_UPS,
    HOOK_REMOVE_PICK_UP,
];
