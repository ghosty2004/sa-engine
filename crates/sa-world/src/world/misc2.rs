//! Garages, entry-exits, pickups, game logic, stats, shopping, interiors, water creatures, furniture, break objects, set pieces (PORT-WORLD-2, helper B).
//!
//! Ported from the 1.0 US bytes (gta-reversed as reading aid; where it differs the bytes win).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{find_player_coors, ftol32, g, rd, wr, x, xi};
use super::entryexit::POOL_ENTRY_EXITS;
use super::garages::{self, NUM_GARAGES};
use super::pickups;

sa_core::globals! {
    list GLOBALS;
    /// `CSetPieces::NumSetPieces`
    pub static NUM_SET_PIECES @ 0xA957FC: u32 = 0;
    /// `CSetPieces::bDebug`
    pub static SET_PIECES_DEBUG @ 0xA957F8: u8 = 0;
    /// `CEntryExitManager::ms_numVisibleEntities`
    pub static NUM_VISIBLE_ENTITIES @ 0x96A7DC: u32 = 0;
    /// `CEntryExitManager::ms_oldAreaCode` (written as a dword, read as a byte)
    pub static OLD_AREA_CODE @ 0x96A734: u32 = 0;
}

/// `CEntryExitManager::ms_visibleEntityList` (`CEntity*[32]`)
const VISIBLE_ENTITY_LIST: usize = 0x96A738;
/// `CGame::currArea`
const CURR_AREA: usize = 0xB72914;
/// end of `CGarages::aGarages`
const GARAGES_END: usize = 0x96EA78;

orig! {
    /// `CAudioEngine::GetRadioStationListenTimes` (0x507020) on `AudioEngine` (0xB6BC90)
    pub fn get_radio_station_listen_times(ae: GPtr<u8>) -> GPtr<u32> = thiscall 0x507020 => unported;
    /// `CQuadTreeNode::DeleteItem(void*)` (0x552A40)
    pub fn quad_tree_delete_item(node: GPtr<u8>, item: GPtr<u8>) = thiscall 0x552A40 => unported;
    /// `CRenderer::GetObjectsInFrustum(CEntity** out, float farPlane, RwMatrix*)` (0x554C60)
    pub fn get_objects_in_frustum(out: GPtr<GPtr<u8>>, far: f32, m: GPtr<u8>) -> i32 = cdecl 0x554C60 => unported;
    /// `CAnimManager::GetAnimationBlockIndex(const char*)` (0x4D3990)
    pub fn get_animation_block_index(name: GPtr<u8>) -> i32 = cdecl 0x4D3990 => unported;
    /// `CStreaming::SetModelIsDeletable(int32)` (0x409C10)
    pub fn set_model_is_deletable(id: i32) = cdecl 0x409C10 => unported;
    /// `CStreaming::StreamPedsForInterior(int32)` (0x40BBB0)
    pub fn stream_peds_for_interior(ty: i32) = cdecl 0x40BBB0 => unported;
    /// `CStreaming::LoadAllRequestedModels(bool)` (0x40EA10)
    pub fn load_all_requested_models(prio: u32) = cdecl 0x40EA10 => unported;
    /// `CVehicle::IsLawEnforcementVehicle` (0x6D2370)
    pub fn is_law_enforcement_vehicle(veh: GPtr<u8>) -> u8 = thiscall 0x6D2370 => unported;
    /// `CDamageManager::GetEngineStatus` (0x6C22C0)
    pub fn get_engine_status(dm: GPtr<u8>) -> u32 = thiscall 0x6C22C0 => unported;
    /// `CDamageManager::SetEngineStatus(uint32)` (0x6C22A0)
    pub fn set_engine_status(dm: GPtr<u8>, s: u32) = thiscall 0x6C22A0 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    pub fn get_pad(i: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CPopulation::RemovePed(CPed*)` (0x610F20)
    pub fn population_remove_ped(ped: GPtr<u8>) = cdecl 0x610F20 => unported;
    /// `CPedIntelligence::GetTaskSwim` (0x601070)
    pub fn get_task_swim(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x601070 => unported;
    /// `CPlayerPed::SetWantedLevel(int32)` (0x609F10)
    pub fn set_wanted_level(ped: GPtr<u8>, level: i32) = thiscall 0x609F10 => unported;
    /// `CAnimManager::RemoveAnimBlockRef(int32)` (0x4D3FD0)
    pub fn remove_anim_block_ref(i: i32) = cdecl 0x4D3FD0 => unported;
    /// `CWeaponInfo::GetWeaponInfo(eWeaponType, eWeaponSkill)` (0x743C60)
    pub fn get_weapon_info(w: i32, skill: i32) -> GPtr<u8> = cdecl 0x743C60 => unported;
    /// `CPedIntelligence::GetTaskJetPack` (0x601110)
    pub fn get_task_jet_pack(intel: GPtr<u8>) -> GPtr<u8> = thiscall 0x601110 => unported;
    /// `RwV3dTransformPoints(out, in, n, matrix)` (0x7EDD90)
    pub fn rw_v3d_transform_points(out: GPtr<f32>, inp: GPtr<f32>, n: i32, m: GPtr<u8>) -> GPtr<f32> = cdecl 0x7EDD90 => unported;
    /// `CBaseModelInfo::Get2dEffect(int32)` (0x4C4C70)
    pub fn get_2d_effect(mi: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x4C4C70 => unported;
    /// `CStreaming::RequestModel(int32, int32 flags)` (0x4087E0)
    pub fn request_model(id: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CAnimManager::AddAnimBlockRef(int32)` (0x4D3FB0)
    pub fn add_anim_block_ref(i: i32) = cdecl 0x4D3FB0 => unported;
    /// `CStats::IncrementStat(uint16, float)` (0x55C180)
    pub fn increment_stat(id: u32, v: u32) = cdecl 0x55C180 => unported;
    /// `CStats::DisplayScriptStatUpdateMessage(uint8, uint32 stat, float)` (0x55B980)
    pub fn display_script_stat_update_message(state: u32, id: u32, v: u32) = cdecl 0x55B980 => unported;
    /// `Interior_c::IsVisible` (0x5929F0)
    pub fn interior_is_visible(int: GPtr<u8>) -> u8 = thiscall 0x5929F0 => unported;
    /// `RwEngineInstance->dOpenDevice.fpRenderStateSet` (RwEngineInstance 0xC97B24, +0x20)
    pub fn rw_render_state_set(state: i32, value: u32) -> i32 = cdecl table 0xC97B24 0x20 => unported;
    /// `BreakObject_c::Render(bool)` (0x59E480)
    pub fn break_object_render(o: GPtr<u8>, flag: u32) = thiscall 0x59E480 => unported;
    /// `CPed::GetEntityThatThisPedIsHolding` (0x5E02E0)
    pub fn get_entity_that_this_ped_is_holding(ped: GPtr<u8>) -> GPtr<u8> = thiscall 0x5E02E0 => unported;
    /// `CRenderer::RequestObjectsInDirection(const CVector&, float, int32)` (0x555CB0)
    pub fn request_objects_in_direction(v: GPtr<f32>, angle: u32, flags: i32) = cdecl 0x555CB0 => unported;
    /// `List_c::~List_c` (0x4A8DE0)
    pub fn list_dtor(l: GPtr<u8>) = thiscall 0x4A8DE0 => unported;
    /// `ListItem_c::~ListItem_c` (0x4A8DC0)
    pub fn list_item_dtor(i: GPtr<u8>) = thiscall 0x4A8DC0 => unported;
    /// `FurnitureSubGroup_c::GetRandomId(uint8 rating)` (0x590FD0)
    pub fn sub_group_get_random_id(sg: GPtr<u8>, rating: u32) -> i32 = thiscall 0x590FD0 => unported;
    /// `FurnitureSubGroup_c::GetFurniture(int16 id, uint8 rating)` (0x590EE0)
    pub fn sub_group_get_furniture(sg: GPtr<u8>, id: u32, rating: u32) -> GPtr<u8> = thiscall 0x590EE0 => unported;
    /// `List_c::List_c` (0x4A8DD0)
    pub fn list_ctor(l: GPtr<u8>) = thiscall 0x4A8DD0 => unported;
    /// `List_c::AddItem(ListItem_c*)` (0x4A8DF0); EAX passed through
    pub fn list_add_item(l: GPtr<u8>, item: GPtr<u8>) -> u32 = thiscall 0x4A8DF0 => unported;
    /// `List_c::RemoveHead` (0x4A8E70)
    pub fn list_remove_head(l: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A8E70 => unported;
    /// `ListItem_c::ListItem_c` (0x4A8DB0)
    pub fn list_item_ctor(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x4A8DB0 => unported;
    /// CRT `eh vector constructor iterator` (0x821D30, stdcall)
    pub fn eh_vector_ctor(arr: GPtr<u8>, size: u32, n: u32, ctor: u32, dtor: u32) = stdcall 0x821D30 => unported;
    /// `BreakObject_c::Update(float)` (0x59E220)
    pub fn break_object_update(o: GPtr<u8>, step: u32) = thiscall 0x59E220 => unported;
    /// `BreakObject_c::Init(CObject*, ...)` (0x59E750)
    pub fn break_object_init(o: GPtr<u8>, obj: GPtr<u8>, a: u32, b: u32, c: u32) -> u8 = thiscall 0x59E750 => unported;
}

// ---- CGarages ----

/// `CGarages::IsPointInAGarageCameraZone(CVector)` (0x449BA0, by value): any of the 50 garages with a
/// type (the jump table sends every non-zero type up to 22, and all above, to the same test) that
/// contains the point with margin 0.5.
#[hook(addr = 0x449BA0, conv = cdecl, name = "CGarages::IsPointInAGarageCameraZone", expect = "8B 54 24 0C 56 8B 74 24")]
pub fn is_point_in_a_garage_camera_zone(px: f32, py: f32, pz: f32) -> u8 {
    for i in 0..garages::MAX_GARAGES as i32 {
        let gp = garages::garage(i);
        if rd::<u8>(gp, 0x4C) != 0 && garages::is_point_inside_garage_margin(gp, px, py, pz, 0.5) != 0 {
            return 1;
        }
    }
    0
}

/// `CGarages::IsPointWithinAnyGarage(const CVector&)` (stub 0x448990 → 0x156EF90): any of the 50
/// garages with a type that contains the point.
#[hook(addr = 0x448990, conv = cdecl, name = "CGarages::IsPointWithinAnyGarage", expect = "E9 FB 65 12 01")]
pub fn is_point_within_any_garage(p: GPtr<f32>) -> u8 {
    let mut a = garages::GARAGES;
    while a < GARAGES_END {
        let gp = g::<u8>(a);
        if rd::<u8>(gp, 0x4C) != 0
            && garages::is_point_inside_garage(gp, p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read()) != 0
        {
            return 1;
        }
        a += garages::GARAGE_SIZE;
    }
    0
}

/// `CGarages::Init_AfterRestart` (0x448B60): every used garage gets its original type back (+0x4F →
/// +0x4C), `InitDoorsAtStart` and a door audio `Reset`; then `NoResprays`... the byte 0x96C008 = 0 and
/// the model index of every stored car (as `CGarages::Init`).
#[hook(addr = 0x448B60, conv = cdecl, name = "CGarages::Init_AfterRestart", expect = "A1 24 C0 96 00 57 33 FF")]
pub fn init_after_restart() {
    let mut i = 0u32;
    while i < NUM_GARAGES.get() {
        let gp = garages::garage(i as i32);
        wr::<u8>(gp, 0x4C, rd::<u8>(gp, 0x4F));
        garages::init_doors_at_start(gp);
        garages::door_audio_reset(gp.byte_add(0x50));
        i += 1;
    }
    g::<u8>(0x96C008).write(0);
    let mut e = 0x96ABEA;
    while e < 0x96ACEA {
        for k in 0..20 {
            g::<u16>(e + 0x100 * k).write(0);
        }
        e += 0x40;
    }
}

/// `CGarages::PlayerArrestedOrDied` (stub 0x449E60 → 0x1569150): `CGarage::PlayerArrestedOrDied` of
/// every garage with a type, then the two words 0x96C01C / 0x96C020 = 0.
#[hook(addr = 0x449E60, conv = cdecl, name = "CGarages::PlayerArrestedOrDied", expect = "E9 EB F2 11 01")]
pub fn garages_player_arrested_or_died() {
    let mut a = garages::GARAGES;
    while a < GARAGES_END {
        let gp = g::<u8>(a);
        if rd::<u8>(gp, 0x4C) != 0 {
            garages::player_arrested_or_died(gp);
        }
        a += garages::GARAGE_SIZE;
    }
    g::<u32>(0x96C01C).write(0);
    g::<u32>(0x96C020).write(0);
}

/// `CGarages::FindGarageForObject(CObject*)` (0x44A240): of the garages (`NumGarages`) containing the
/// object with margin 7, the one whose centre (base + A·width/2 + B·depth/2, z + 0·w/2 + 0·d/2, with
/// the original's f32 roundings) is nearest in 3D (strictly nearer than 99999.9 and than the previous
/// best); -1 when none.
#[hook(addr = 0x44A240, conv = cdecl, name = "CGarages::FindGarageForObject", expect = "83 EC 30 8B 0D 24 C0 96 00")]
pub fn find_garage_for_object(obj: GPtr<u8>) -> i32 {
    let mut best = f32::from_bits(0x47C3_4FF3);
    let mut best_i = -1;
    let m = rd::<GPtr<u8>>(obj, 0x14);
    let pos = if m.is_null() { obj.byte_add(4) } else { m.byte_add(0x30) };
    let mut i = 0u32;
    while i < NUM_GARAGES.get() {
        let gp = garages::garage(i as i32);
        let (px, py, pz) = (rd::<f32>(pos, 0), rd::<f32>(pos, 4), rd::<f32>(pos, 8));
        if garages::is_point_inside_garage_margin(gp, px, py, pz, 7.0) != 0 {
            let f = |o: isize| rd::<f32>(gp, o);
            let hw = x(f(0x20)).fmul_m(0.5).to_f32();
            let axhw = x(f(0xC)).fmul_m(hw).to_f32();
            let ayhw = x(f(0x10)).fmul_m(hw);
            let hw0 = x(hw).fmul_m(0.0).to_f32();
            let cx1 = x(axhw).fadd(x(f(0))).to_f32();
            let y1 = ayhw.fadd(x(f(4)));
            let z1 = x(hw0).fadd_m(f(8));
            let hd = x(f(0x24)).fmul_m(0.5).to_f32();
            let bxhd = x(f(0x14)).fmul_m(hd).to_f32();
            let byhd = x(f(0x18)).fmul_m(hd);
            let hd0 = x(hd).fmul_m(0.0).to_f32();
            let cx = x(bxhd).fadd_m(cx1).to_f32();
            let cy = byhd.fadd(y1).to_f32();
            let cz = x(hd0).fadd(z1).to_f32();
            let dx = x(rd::<f32>(pos, 0)).fsub_m(cx);
            let dy = x(rd::<f32>(pos, 4)).fsub_m(cy);
            let dz = x(rd::<f32>(pos, 8)).fsub_m(cz);
            let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx)).fsqrt();
            // test ah,5 / jp: kept only when Less
            if !d.fcom(x(best)).test_parity(0x05) {
                best = d.to_f32();
                best_i = i as i32;
            }
        }
        i += 1;
    }
    best_i
}

// ---- CEntryExitManager ----

/// `CEntryExitManager::ResetAreaCodeForVisibleObjects` (stub 0x43ED80 → 0x15678A0): area code (+0x2F)
/// of every visible entity, last first, = `ms_oldAreaCode`; the count ends at -1 (original bug: one more
/// decrement after the loop, also when the count was 0).
#[hook(addr = 0x43ED80, conv = cdecl, name = "CEntryExitManager::ResetAreaCodeForVisibleObjects", expect = "E9 1B 8B 12 01")]
pub fn reset_area_code_for_visible_objects() {
    if NUM_VISIBLE_ENTITIES.get() != 0 {
        loop {
            let n = NUM_VISIBLE_ENTITIES.get().wrapping_sub(1);
            let area = OLD_AREA_CODE.get() as u8;
            NUM_VISIBLE_ENTITIES.set(n);
            let e = g::<GPtr<u8>>(VISIBLE_ENTITY_LIST).wrapping_index(n as i32).read();
            wr::<u8>(e, 0x2F, area);
            if n == 0 {
                break;
            }
        }
    }
    NUM_VISIBLE_ENTITIES.set(NUM_VISIBLE_ENTITIES.get().wrapping_sub(1));
}

/// `CEntryExitManager::DeleteOne(int32)` (stub 0x43FD50 → 0x1560ED0): for a used slot, the entry-exit
/// leaves the quad tree (`mp_QuadTree` 0x96A7D0) and its pool slot is freed (flag bit 7, first free
/// lowered when the index is below it).
#[hook(addr = 0x43FD50, conv = cdecl, name = "CEntryExitManager::DeleteOne", expect = "E9 7B 11 12 01")]
pub fn entry_exit_delete_one(i: i32) {
    let pool = POOL_ENTRY_EXITS.get();
    if (rd::<GPtr<u8>>(pool, 4).byte_add(i as isize).read() as i8) < 0 {
        return;
    }
    let e = rd::<GPtr<u8>>(pool, 0).byte_add(i.wrapping_mul(0x3C) as isize);
    quad_tree_delete_item(g::<GPtr<u8>>(0x96A7D0).read(), e);
    let pool = POOL_ENTRY_EXITS.get();
    let idx = ((e.addr() as i32).wrapping_sub(rd::<GPtr<u8>>(pool, 0).addr() as i32)) / 0x3C;
    let f = rd::<GPtr<u8>>(pool, 4).byte_add(idx as isize);
    f.write(f.read() | 0x80);
    if idx < rd::<i32>(pool, 0xC) {
        wr::<i32>(pool, 0xC, idx);
    }
}

/// `CEntryExitManager::SetAreaCodeForVisibleObjects` (stub 0x43ECF0 → 0x1560930): the entities in the
/// frustum (far plane 6, into 0xC8E1C8) whose area (+0x2F) is `CGame::currArea`, other than the player
/// ped, up to 32, are listed and moved to area 13; `ms_oldAreaCode` = `currArea`.
#[hook(addr = 0x43ECF0, conv = cdecl, name = "CEntryExitManager::SetAreaCodeForVisibleObjects", expect = "E9 3B 1C 12 01")]
pub fn set_area_code_for_visible_objects() {
    let arr = g::<GPtr<u8>>(0xC8E1C8);
    let mut n = get_objects_in_frustum(arr, 6.0, GPtr::NULL) as u32;
    let player = super::common::find_player_ped(-1);
    NUM_VISIBLE_ENTITIES.set(0);
    let mut area = g::<u32>(CURR_AREA).read();
    let mut k = 0;
    let mut cnt = 0u32;
    while n != 0 {
        n -= 1;
        if cnt as i32 >= 0x20 {
            break;
        }
        let e = arr.wrapping_index(k).read();
        if u32::from(rd::<u8>(e, 0x2F)) == area && e != player {
            g::<GPtr<u8>>(VISIBLE_ENTITY_LIST).wrapping_index(cnt as i32).write(e);
            NUM_VISIBLE_ENTITIES.set(cnt + 1);
            wr::<u8>(e, 0x2F, 13);
            cnt = NUM_VISIBLE_ENTITIES.get();
            area = g::<u32>(CURR_AREA).read();
        }
        k += 1;
    }
    OLD_AREA_CODE.set(area);
}

/// `InteriorManager_c::SetEntryExitPtr(CEntryExit*)` (0x598180, thiscall, `ret 4`): only for an
/// entry-exit whose area (its link's when linked, +0x38 → +0x32) is not 0. When its rectangle (+8..+0x14)
/// differs from the stored one (+0x4740..: compared y0, y1, x0, x1; NaN counts as different) the two
/// counters +0x43A8 / +0x42A4 = 0 and the 16 bytes +0x472C = 1; then the pointer (+0x473C) and the
/// rectangle are stored.
#[hook(addr = 0x598180, conv = thiscall, name = "InteriorManager_c::SetEntryExitPtr", expect = "8B 54 24 04 8B 42 38 85", this = "InteriorManager_c")]
pub fn interior_set_entry_exit_ptr(this: GPtr<u8>, ee: GPtr<u8>) {
    let link = rd::<GPtr<u8>>(ee, 0x38);
    let area = if link.is_null() { rd::<u8>(ee, 0x32) } else { rd::<u8>(link, 0x32) };
    if area == 0 {
        return;
    }
    // test ah,0x44 / jp: taken unless Equal
    let differs = |eo: isize, to: isize| x(rd::<f32>(ee, eo)).fcom(x(rd::<f32>(this, to))).test_parity(0x44);
    if differs(0xC, 0x4744) || differs(0x14, 0x474C) || differs(0x8, 0x4740) || differs(0x10, 0x4748) {
        wr::<u32>(this, 0x43A8, 0);
        wr::<u32>(this, 0x42A4, 0);
        for o in [0x472C, 0x4730, 0x4734, 0x4738] {
            wr::<u32>(this, o, 0x0101_0101);
        }
    }
    wr::<GPtr<u8>>(this, 0x473C, ee);
    for o in [0, 4, 8, 0xC] {
        wr::<u32>(this, 0x4740 + o, rd::<u32>(ee, 8 + o));
    }
}

// ---- CPickups ----

/// `CPickups::RemoveUnnecessaryPickups(const CVector&, float)` (0x4563A0): every pickup of type 4 or 8
/// whose position (packed int16 / 8) is within `radius` (3D, `fsqrt`) of `pos` is removed.
#[hook(addr = 0x4563A0, conv = cdecl, name = "CPickups::RemoveUnnecessaryPickups", expect = "83 EC 1C 56 57 8B 7C 24")]
pub fn remove_unnecessary_pickups(pos: GPtr<f32>, radius: f32) {
    let k = f32::from_bits(0x3E00_0000);
    for i in 0..pickups::NUM_PICKUPS as u32 {
        let p = pickups::pickup(i);
        let ty = rd::<u8>(p, 0x1C);
        if ty != 4 && ty != 8 {
            continue;
        }
        let px = xi(i32::from(rd::<i16>(p, 0x10))).fmul_m(k);
        let py = xi(i32::from(rd::<i16>(p, 0x12))).fmul_m(k);
        let pz = xi(i32::from(rd::<i16>(p, 0x14))).fmul_m(k).to_f32();
        let dx = px.fsub_m(pos.read()).to_f32();
        let dy = py.fsub_m(pos.wrapping_index(1).read());
        let dz = x(pz).fsub_m(pos.wrapping_index(2).read());
        let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(x(dx).fmul_m(dx)).fsqrt();
        if d.fcom(x(radius)).test_parity(0x05) {
            continue;
        }
        pickups::remove(p);
    }
}

// ---- CShopping ----

/// `CShopping::ms_shopLoaded` (index into the section names at 0x8A61D8)
const SHOP_LOADED: usize = 0xA9A7C8;
/// `CShopping::ms_priceModifiers` (key, modifier; 8 bytes each) and their count
const PRICE_MODIFIERS: usize = 0xA98650;
const NUM_PRICE_MODIFIERS: usize = 0xA9A7D0;

/// `CShopping::RemoveLoadedPrices` (0x49AC90): the animation block named by the loaded shop's section
/// becomes deletable (`+0x63E7`, the IFP ids), then `ms_shopLoaded` = 0.
#[hook(addr = 0x49AC90, conv = cdecl, name = "CShopping::RemoveLoadedPrices", expect = "A1 C8 A7 A9 00 8B 04 85")]
pub fn remove_loaded_prices() {
    let name = g::<GPtr<u8>>(0x8A61D8).wrapping_index(g::<i32>(SHOP_LOADED).read()).read();
    let b = get_animation_block_index(name);
    if b != -1 {
        set_model_is_deletable(b.wrapping_add(0x63E7));
    }
    g::<u32>(SHOP_LOADED).write(0);
}

/// `CShopping` price-modifier removal (stub 0x49ACD0 → 0x156AC70, `RemovePriceModifier(uint32 key)`):
/// the entry with the key is replaced by the last one; the count is decremented (no copy when it drops
/// to 0 or below).
#[hook(addr = 0x49ACD0, conv = cdecl, name = "thunk_FUN_0156ac70", expect = "E9 9B FF 0C 01")]
pub fn remove_price_modifier(key: u32) {
    let mut n = g::<i32>(NUM_PRICE_MODIFIERS).read();
    let m = g::<u32>(PRICE_MODIFIERS);
    let mut i = 0;
    if n > 0 {
        while key != m.wrapping_index(i * 2).read() {
            i += 1;
            if i >= n {
                break;
            }
        }
    }
    if i == n {
        return;
    }
    n -= 1;
    g::<i32>(NUM_PRICE_MODIFIERS).write(n);
    if n <= 0 {
        return;
    }
    m.wrapping_index(i * 2).write(m.wrapping_index(n * 2).read());
    m.wrapping_index(i * 2 + 1).write(m.wrapping_index(n * 2 + 1).read());
}

/// `CShopping::ShutdownForRestart` (stub 0x49B640 → 0x15624D0): the byte 0xA9A7D8 = 0 (after the section
/// name is read), the loaded shop's animation block released, all bought flags (0x230 bytes at
/// 0xA972A0) cleared, `ms_shopLoaded` and the price-modifier count = 0.
#[hook(addr = 0x49B640, conv = cdecl, name = "CShopping::ShutdownForRestart", expect = "E9 8B 6E 0C 01")]
pub fn shopping_shutdown_for_restart() {
    let name = g::<GPtr<u8>>(0x8A61D8).wrapping_index(g::<i32>(SHOP_LOADED).read()).read();
    g::<u8>(0xA9A7D8).write(0);
    let b = get_animation_block_index(name);
    if b != -1 {
        set_model_is_deletable(b.wrapping_add(0x63E7));
    }
    for i in 0..0x8C {
        g::<u32>(0xA972A0).wrapping_index(i).write(0);
    }
    g::<u32>(SHOP_LOADED).write(0);
    g::<u32>(NUM_PRICE_MODIFIERS).write(0);
}

/// `CShopping::IncrementStat2(int32 id, int32 value)` (0x49AFD0): nothing for -1; ×10 for stats 0x42 and
/// 0x50; then `CStats::ModifyStat(id, (float)value)`.
#[hook(addr = 0x49AFD0, conv = cdecl, name = "CShopping::IncrementStat2", expect = "8B 44 24 04 83 F8 FF 74")]
pub fn increment_stat2(id: i32, value: i32) {
    if id == -1 {
        return;
    }
    let v = if id == 0x42 || id == 0x50 { value.wrapping_mul(10) } else { value };
    super::stats::modify_stat(id as u32, xi(v).to_f32().to_bits());
}

// ---- CSetPiece ----

/// `(int16 at +ox, int16 at +ox+2) / 4` into `out` (`CVector2D`, hidden return pointer; y is loaded
/// first); EAX = out.
fn set_piece_coord(this: GPtr<u8>, out: GPtr<f32>, ox: isize) -> GPtr<f32> {
    let k = f32::from_bits(0x3E80_0000);
    let y = xi(i32::from(rd::<i16>(this, ox + 2))).fmul_m(k);
    let xx = xi(i32::from(rd::<i16>(this, ox))).fmul_m(k);
    out.write(xx.to_f32());
    out.wrapping_index(1).write(y.to_f32());
    out
}

/// `CSetPiece::GetSpawnCoord1` (0x4997A0, thiscall, `CVector2D` by hidden pointer): +0xC/+0xE / 4.
#[hook(addr = 0x4997A0, conv = thiscall, name = "CSetPiece::GetSpawnCoord1", expect = "51 0F BF 41 0E 0F BF 49", this = "CSetPiece")]
pub fn set_piece_get_spawn_coord1(this: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> {
    set_piece_coord(this, out, 0xC)
}

/// `CSetPiece::GetSpawnCoord2` (0x4997E0): +0x10/+0x12 / 4.
#[hook(addr = 0x4997E0, conv = thiscall, name = "CSetPiece::GetSpawnCoord2", expect = "51 0F BF 41 12 0F BF 49", this = "CSetPiece")]
pub fn set_piece_get_spawn_coord2(this: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> {
    set_piece_coord(this, out, 0x10)
}

/// `CSetPiece::GetTargetCoord1` (0x499820): +0x14/+0x16 / 4.
#[hook(addr = 0x499820, conv = thiscall, name = "CSetPiece::GetTargetCoord1", expect = "51 0F BF 41 16 0F BF 49", this = "CSetPiece")]
pub fn set_piece_get_target_coord1(this: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> {
    set_piece_coord(this, out, 0x14)
}

/// `CSetPiece::GetTargetCoord2` (0x499860): +0x18/+0x1A / 4.
#[hook(addr = 0x499860, conv = thiscall, name = "CSetPiece::GetTargetCoord2", expect = "51 0F BF 41 1A 0F BF 49", this = "CSetPiece")]
pub fn set_piece_get_target_coord2(this: GPtr<u8>, out: GPtr<f32>) -> GPtr<f32> {
    set_piece_coord(this, out, 0x18)
}

// ---- interiors ----

/// `InteriorGroup_c::GetEntity` (0x594BD0, thiscall): +8 (4 bytes, not hookable).
#[sa_core::native(addr = 0x594BD0, conv = thiscall, name = "InteriorGroup_c::GetEntity")]
pub fn interior_group_get_entity(this: GPtr<u8>) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 8)
}

/// `InteriorGroup_c::GetPed(int32)` (0x594B90, thiscall): `+0x38[i]`.
#[hook(addr = 0x594B90, conv = thiscall, name = "InteriorGroup_c::GetPed", expect = "8B 44 24 04 8B 44 81 38", this = "InteriorGroup_c")]
pub fn interior_group_get_ped(this: GPtr<u8>, i: i32) -> GPtr<u8> {
    rd::<GPtr<u8>>(this, 0x38 + i as isize * 4)
}

/// `InteriorGroup_c::AddInterior(Interior_c*)` (0x594840, thiscall): stored in the first null slot of
/// `+0x10[8]`, the count byte +0xF incremented; EAX = the slot (8 when full, nothing stored).
#[hook(addr = 0x594840, conv = thiscall, name = "InteriorGroup_c::AddInterior", expect = "33 C0 8D 51 10 83 3A 00", this = "InteriorGroup_c")]
pub fn interior_group_add_interior(this: GPtr<u8>, interior: GPtr<u8>) -> i32 {
    for i in 0..8 {
        if rd::<u32>(this, 0x10 + i as isize * 4) == 0 {
            wr::<GPtr<u8>>(this, 0x10 + i as isize * 4, interior);
            wr::<u8>(this, 0xF, rd::<u8>(this, 0xF).wrapping_add(1));
            return i;
        }
    }
    8
}

/// `InteriorGroup_c::SetupOfficePeds` (0x594BF0, thiscall): `StreamPedsForInterior(2)`,
/// `LoadAllRequestedModels(false)`, +0x36 = 0.
#[hook(addr = 0x594BF0, conv = thiscall, name = "InteriorGroup_c::SetupOfficePeds", expect = "56 6A 02 8B F1 E8 B6 6F", this = "InteriorGroup_c")]
pub fn interior_group_setup_office_peds(this: GPtr<u8>) {
    stream_peds_for_interior(2);
    load_all_requested_models(0);
    wr::<u8>(this, 0x36, 0);
}

/// `Interior_c::GetFurnitureEntity(Furniture_c*)` (0x5913B0, thiscall): the node of the list at +0x5C
/// (+4 next) whose +8 is the argument, or null.
#[hook(addr = 0x5913B0, conv = thiscall, name = "Interior_c::GetFurnitureEntity", expect = "8B 41 5C 85 C0 74 15 8B", this = "Interior_c")]
pub fn interior_get_furniture_entity(this: GPtr<u8>, f: GPtr<u8>) -> GPtr<u8> {
    let mut n = rd::<GPtr<u8>>(this, 0x5C);
    while !n.is_null() {
        if rd::<GPtr<u8>>(n, 8) == f {
            return n;
        }
        n = rd::<GPtr<u8>>(n, 4);
    }
    GPtr::NULL
}

/// `InteriorManager_c::ActivatePeds(bool)` (0x598080, thiscall): +0x4752 = the flag.
#[hook(addr = 0x598080, conv = thiscall, name = "InteriorManager_c::ActivatePeds", expect = "8A 44 24 04 88 81 52 47 00 00", this = "InteriorManager_c")]
pub fn interior_manager_activate_peds(this: GPtr<u8>, on: u8) {
    wr::<u8>(this, 0x4752, on);
}

/// `InteriorManager_c::IsGroupActive(int32 type)` (0x598280, thiscall): any group of the list at
/// +0x428C (+4 next) whose type byte +0xE (sign-extended) is `type`.
#[hook(addr = 0x598280, conv = thiscall, name = "InteriorManager_c::IsGroupActive", expect = "8B 81 8C 42 00 00 85 C0", this = "InteriorManager_c")]
pub fn interior_manager_is_group_active(this: GPtr<u8>, ty: i32) -> u8 {
    let mut n = rd::<GPtr<u8>>(this, 0x428C);
    while !n.is_null() {
        if i32::from(rd::<i8>(n, 0xE)) == ty {
            return 1;
        }
        n = rd::<GPtr<u8>>(n, 4);
    }
    0
}

/// `InteriorManager_c::FindStealableObjectId(CEntity*)` (0x598360, thiscall): the index of the entity
/// among the `+0x43A8` stealable objects (0x1C each from +0x43AC), -1 when absent.
#[hook(addr = 0x598360, conv = thiscall, name = "InteriorManager_c::FindStealableObjectId-1", expect = "8B 91 A8 43 00 00 56 33", this = "InteriorManager_c")]
pub fn interior_manager_find_stealable_object_id(this: GPtr<u8>, e: GPtr<u8>) -> i32 {
    let n = rd::<i32>(this, 0x43A8);
    let mut i = 0;
    while i < n {
        if rd::<GPtr<u8>>(this, 0x43AC + i as isize * 0x1C) == e {
            return i;
        }
        i += 1;
    }
    -1
}

/// `InteriorManager_c::SetStealableObjectStolen(CEntity*, bool)` (0x598390, thiscall, `ret 8`): the
/// stolen byte (+0x18 of the record) of the entity's record = the flag. EAX: `index * 0x1C` when found,
/// else the count (0 when not positive).
#[hook(addr = 0x598390, conv = thiscall, name = "InteriorManager_c::SetStealableObjectStolen", expect = "56 8B B1 A8 43 00 00 33", this = "InteriorManager_c")]
pub fn interior_manager_set_stealable_object_stolen(this: GPtr<u8>, e: GPtr<u8>, stolen: u8) -> i32 {
    let n = rd::<i32>(this, 0x43A8);
    if n <= 0 {
        return 0;
    }
    let mut i = 0;
    while i < n {
        if rd::<GPtr<u8>>(this, 0x43AC + i as isize * 0x1C) == e {
            let o = i.wrapping_mul(0x1C);
            wr::<u8>(this, 0x43C4 + o as isize, stolen);
            return o;
        }
        i += 1;
    }
    n
}

// ---- batch 4: garages, pickups, interiors ----

/// `CGarages::IsCarSprayable(CVehicle*)` (stub 0x4479A0 → 0x156B1C0): not for law enforcement
/// vehicles, vehicle class 10 (+0x594) or the models 407, 416, 431, 437 (and, through the unsigned
/// compare, model -2).
#[hook(addr = 0x4479A0, conv = cdecl, name = "CGarages::IsCarSprayable", expect = "E9 1B 38 12 01")]
pub fn is_car_sprayable(veh: GPtr<u8>) -> u8 {
    if is_law_enforcement_vehicle(veh) != 0 || rd::<u32>(veh, 0x594) == 10 {
        return 0;
    }
    let m = i32::from(rd::<i16>(veh, 0x22)) as u32;
    let excluded = if m > 0x1AF { m == 0x1B5 || m == 0xFFFF_FFFE } else { m == 0x1AF || m == 0x197 || m == 0x1A0 };
    (!excluded) as u8
}

/// `CGarages::StopCarFromBlowingUp(CAutomobile*)` (0x448890): +0x8E4 (`m_dwBurnTimer`) = 0, health
/// (+0x4C0) raised to 300 (300 also for NaN), engine status (+0x5A0 damage manager) set to
/// `max(status, 0x113)` (always 0x113, clamped to 250 by the setter).
#[hook(addr = 0x448890, conv = cdecl, name = "CGarages::StopCarFromBlowingUp", expect = "8B 4C 24 04 D9 81 C0 04 00 00")]
pub fn stop_car_from_blowing_up(veh: GPtr<u8>) {
    let h = rd::<f32>(veh, 0x4C0);
    wr::<u32>(veh, 0x8E4, 0);
    // test ah,0x41 / jne: 300 unless Greater
    let v = if x(h).fcom(x(300.0)).test_any(0x41) { 300.0 } else { h };
    wr::<f32>(veh, 0x4C0, x(v).to_f32());
    let dm = veh.byte_add(0x5A0);
    let s = get_engine_status(dm);
    if s < 0x113 {
        set_engine_status(dm, 0x113);
    } else {
        set_engine_status(dm, get_engine_status(dm));
    }
}

/// `CGarages::IsPointWithinHideOutGarage(const CVector&)` (0x448900): any garage of a hideout type
/// (16..18, 24..32, 39..42, 45: the byte table at 0x44896C) that contains the point.
#[hook(addr = 0x448900, conv = cdecl, name = "CGarages::IsPointWithinHideOutGarage", expect = "56 8B 74 24 08")]
pub fn is_point_within_hide_out_garage(p: GPtr<f32>) -> u8 {
    let mut a = garages::GARAGES;
    while a < GARAGES_END {
        let gp = g::<u8>(a);
        let t = u32::from(rd::<u8>(gp, 0x4C)).wrapping_sub(16);
        if matches!(t, 0..=2 | 8..=16 | 23..=26 | 29)
            && garages::is_point_inside_garage(gp, p.read(), p.wrapping_index(1).read(), p.wrapping_index(2).read()) != 0
        {
            return 1;
        }
        a += garages::GARAGE_SIZE;
    }
    0
}

/// `CPickups::ModifyStringLabelForControlSetting(char*)` (0x454B70): a label longer than 2 ending in
/// `_?` gets its last character from pad 0's controller mode (+0x10A): 0, 1 → 'L', 2 → 'T', 3 → 'C',
/// other modes unchanged.
#[hook(addr = 0x454B70, conv = cdecl, name = "CPickups::ModifyStringLabelForControlSetting", expect = "56 57 8B 7C 24 0C")]
pub fn modify_string_label_for_control_setting(label: GPtr<u8>) {
    let mut n = 0i32;
    while label.byte_add(n as isize).read() != 0 {
        n += 1;
    }
    if n <= 2 || label.byte_add(n as isize - 2).read() != b'_' {
        return;
    }
    let c = match rd::<u16>(get_pad(0), 0x10A) {
        0 | 1 => b'L',
        2 => b'T',
        3 => b'C',
        _ => return,
    };
    label.byte_add(n as isize - 1).write(c);
}

/// `InteriorManager_c::GetPedsInteriorGroup(CPed*)` (0x598240, thiscall): the group of the list at
/// +0x428C (+4 next) whose ped slots (+0x38, count byte +0x36 sign-extended) hold the ped, or null.
#[hook(addr = 0x598240, conv = thiscall, name = "InteriorManager_c::GetPedsInteriorGroup", expect = "8B 81 8C 42 00 00", this = "InteriorManager_c")]
pub fn interior_manager_get_peds_interior_group(this: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> {
    let mut gp = rd::<GPtr<u8>>(this, 0x428C);
    while !gp.is_null() {
        let n = i32::from(rd::<i8>(gp, 0x36));
        for i in 0..n.max(0) {
            if rd::<GPtr<u8>>(gp, 0x38 + i as isize * 4) == ped {
                return gp;
            }
        }
        gp = rd::<GPtr<u8>>(gp, 4);
    }
    GPtr::NULL
}

/// `InteriorGroup_c::GetNumInteriorInfos(int32 type)` (0x594920, thiscall): over the 8 interior slots
/// (+0x10), the count of infos (+0x590, 0x20 each, count byte +0x40D) whose type byte is `type`.
#[hook(addr = 0x594920, conv = thiscall, name = "InteriorGroup_c::GetNumInteriorInfos", expect = "53 8B 5C 24 08", this = "InteriorGroup_c")]
pub fn interior_group_get_num_interior_infos(this: GPtr<u8>, ty: i32) -> i32 {
    let mut c = 0i32;
    for s in 0..8 {
        let int = rd::<GPtr<u8>>(this, 0x10 + s * 4);
        if int.is_null() {
            continue;
        }
        let n = i32::from(rd::<i8>(int, 0x40D));
        for k in 0..n.max(0) {
            if i32::from(rd::<i8>(int, 0x590 + k as isize * 0x20)) == ty {
                c += 1;
            }
        }
    }
    c
}

/// `InteriorGroup_c::RemovePed(CPed*)` (0x594E30, thiscall): the first of the 16 ped slots (+0x38)
/// holding the ped: `CPopulation::RemovePed`, slot cleared, count byte +0x36 decremented. (EAX, unused
/// by the void callers, is not reproduced.)
#[hook(addr = 0x594E30, conv = thiscall, name = "InteriorGroup_c::RemovePed", expect = "8B 54 24 04 56 57", this = "InteriorGroup_c")]
pub fn interior_group_remove_ped(this: GPtr<u8>, ped: GPtr<u8>) {
    for i in 0..16 {
        let s = rd::<GPtr<u8>>(this, 0x38 + i * 4);
        if !s.is_null() && s == ped {
            population_remove_ped(ped);
            wr::<u32>(this, 0x38 + i * 4, 0);
            wr::<u8>(this, 0x36, rd::<u8>(this, 0x36).wrapping_sub(1));
            return;
        }
    }
}

// ---- batch 5: interiors, water creatures ----

/// `InteriorGroup_c::Init(CEntity*, int32 type)` (0x5947E0, thiscall, `ret 8`): interior slots
/// (+0x10[8]), ped slots (+0x38[16]) and +0x78[16] cleared, the bytes +0xF, +0xB8, +0xB9, +0x34, +0x35,
/// +0xBA = 0, +8 = the entity, +0xD = the type byte.
#[hook(addr = 0x5947E0, conv = thiscall, name = "InteriorGroup_c::Init", expect = "8B D1 53 57 33 C0", this = "InteriorGroup_c")]
pub fn interior_group_init(this: GPtr<u8>, e: GPtr<u8>, ty: u32) {
    for i in 0..8 {
        wr::<u32>(this, 0x10 + i * 4, 0);
    }
    for i in 0..16 {
        wr::<u32>(this, 0x38 + i * 4, 0);
        wr::<u32>(this, 0x78 + i * 4, 0);
    }
    for o in [0xF, 0xB8, 0xB9, 0x34, 0x35, 0xBA] {
        wr::<u8>(this, o, 0);
    }
    wr::<GPtr<u8>>(this, 8, e);
    wr::<u8>(this, 0xD, ty as u8);
}

/// `InteriorGroup_c::ContainsInteriorType(int32)` (0x595250, thiscall): any of the 8 interiors whose
/// info (+0x14) type byte (+0) is the type.
#[hook(addr = 0x595250, conv = thiscall, name = "InteriorGroup_c::ContainsInteriorType", expect = "56 8B 74 24 08", this = "InteriorGroup_c")]
pub fn interior_group_contains_interior_type(this: GPtr<u8>, ty: i32) -> u8 {
    for s in 0..8 {
        let int = rd::<GPtr<u8>>(this, 0x10 + s * 4);
        if !int.is_null() && i32::from(rd::<u8>(rd::<GPtr<u8>>(int, 0x14), 0)) == ty {
            return 1;
        }
    }
    0
}

/// `Interior_c::GetTileStatus(int32 x, int32 y)` (0x5918E0, thiscall, `ret 8`): inside the info's
/// width/depth (+0x14 → bytes +2/+3, signed compares) the tile byte `+0x68[x * 30 + y]`, else 1.
#[hook(addr = 0x5918E0, conv = thiscall, name = "Interior_c::GetTileStatus", expect = "8B 51 14 8B 44 24 04", this = "Interior_c")]
pub fn interior_get_tile_status(this: GPtr<u8>, tx: i32, ty: i32) -> u32 {
    let info = rd::<GPtr<u8>>(this, 0x14);
    if tx >= i32::from(rd::<u8>(info, 2)) || ty >= i32::from(rd::<u8>(info, 3)) || tx < 0 || ty < 0 {
        return 1;
    }
    u32::from(rd::<u8>(this, 0x68 + (tx * 0x1E + ty) as isize))
}

/// `InteriorManager_c::HasInteriorHadStealDataSetup(Interior_c*)` (0x5982B0, thiscall): the interior's
/// +8 among the `+0x42A4` ids at +0x42A8.
#[hook(addr = 0x5982B0, conv = thiscall, name = "InteriorManager_c::HasInteriorHadStealDataSetup", expect = "8B 91 A4 42 00 00", this = "InteriorManager_c")]
pub fn interior_manager_has_interior_had_steal_data_setup(this: GPtr<u8>, int: GPtr<u8>) -> u8 {
    let n = rd::<i32>(this, 0x42A4);
    if n <= 0 {
        return 0;
    }
    let id = rd::<u32>(int, 8);
    for i in 0..n {
        if rd::<u32>(this, 0x42A8 + i as isize * 4) == id {
            return 1;
        }
    }
    0
}

/// `WaterCreatureManager_c::CanAddWaterCreatureAtPos(int32 type, CVector pos)` (0x6E4510, thiscall,
/// `ret 0x10`): false when a group leader (+0x1C null) of the type (+0xC) in the creature list
/// (+0x1A0C, +4 next) is nearer in 2D than the type's minimum distance (`tWaterCreatureInfo` 0x8D3698,
/// 0x2C each, +0xC; squared in registers, `(dy² + dx²)` compared below it).
#[hook(addr = 0x6E4510, conv = thiscall, name = "WaterCreatureManager_c::CanAddWaterCreatureAtPos", expect = "83 EC 2C 53 8B 5C 24 34")]
pub fn can_add_water_creature_at_pos(this: GPtr<u8>, ty: i32, px: f32, py: f32, _pz: f32) -> u8 {
    let md = g::<f32>(0x8D3698 + 0xC).byte_add(ty.wrapping_mul(0x2C) as isize).read();
    let mut n = rd::<GPtr<u8>>(this, 0x1A0C);
    while !n.is_null() {
        let next = rd::<GPtr<u8>>(n, 4);
        if i32::from(rd::<u8>(n, 0xC)) == ty && rd::<u32>(n, 0x1C) == 0 {
            let e = rd::<GPtr<u8>>(n, 8);
            let m = rd::<GPtr<u8>>(e, 0x14);
            let pos = if m.is_null() { e.byte_add(4) } else { m.byte_add(0x30) };
            let dx = x(px).fsub_m(rd::<f32>(pos, 0));
            let dy = x(py).fsub_m(rd::<f32>(pos, 4));
            let d = dy.fmul(dy).fadd(dx.fmul(dx));
            // fcompp (md² vs d), test ah,0x41 / je: false when Greater
            if !x(md).fmul_m(md).fcom(d).test_any(0x41) {
                return 0;
            }
        }
        n = next;
    }
    1
}

// ---- batch 6: game logic ----

/// One forbidden-area test of `SetPlayerWantedLevelForForbiddenTerritories`: inside the line area the
/// player gets wanted level 4; with `immediately` the wanted record's +0xC (`m_nLastTimeWantedDecreased`...
/// gr: the chaos/time field) = 0.
fn forbidden_area(pts: usize, n: i32, immediately: u8) {
    let mut c = [0f32; 3];
    let p = find_player_coors(GPtr::from_raw(c.as_mut_ptr()), -1);
    let (px, py) = (p.read().to_bits(), p.wrapping_index(1).read().to_bits());
    if super::gamelogic::is_point_within_line_area(g(pts), n, px, py) as u8 == 0 {
        return;
    }
    set_wanted_level(super::common::find_player_ped(-1), 4);
    if immediately != 0 {
        wr::<u32>(super::cworld::find_player_wanted(-1), 0xC, 0);
    }
}

/// `CGameLogic::SetPlayerWantedLevelForForbiddenTerritories(bool immediately)` (0x441770): unless
/// immediately, only when `(CTimer::m_FrameCounter & 0x1F) == 0x12`; not above z 950, not while
/// swimming, not at wanted level 4 or more (the player data's wanted record is dereferenced unchecked).
/// While stat 0xB5 (cities unlocked) <= 1 the area at 0x8A5EC8 (10 points), while <= 0 the one at
/// 0x8A5E58 (9 points) give wanted level 4 (the stat is read again for the second test).
#[hook(addr = 0x441770, conv = cdecl, name = "CGameLogic::SetPlayerWantedLevelForForbiddenTerritories", expect = "83 EC 0C 53 8A 5C 24 14")]
pub fn set_player_wanted_level_for_forbidden_territories(immediately: u8) {
    if immediately == 0 && g::<u32>(0xB7CB4C).read() as u8 & 0x1F != 0x12 {
        return;
    }
    let mut c = [0f32; 3];
    let p = find_player_coors(GPtr::from_raw(c.as_mut_ptr()), -1);
    if above(p.wrapping_index(2).read(), 950.0) {
        return;
    }
    let ped = super::common::find_player_ped(-1);
    if !get_task_swim(rd::<GPtr<u8>>(ped, 0x47C)).is_null() {
        return;
    }
    let pd = rd::<GPtr<u8>>(super::common::find_player_ped(-1), 0x480);
    let wanted = if pd.is_null() { GPtr::NULL } else { rd::<GPtr<u8>>(pd, 0) };
    if rd::<i32>(wanted, 0x2C) >= 4 {
        return;
    }
    // test ah,0x41 / jp: the area test runs for Less or Equal
    if !super::stats::get_stat_value(0xB5).fcom(x(1.0)).test_parity(0x41) {
        forbidden_area(0x8A5EC8, 10, immediately);
    }
    if !super::stats::get_stat_value(0xB5).fcom(x(0.0)).test_parity(0x41) {
        forbidden_area(0x8A5E58, 9, immediately);
    }
}

// ---- batch 8: garages, entry-exits, shopping, interiors ----

/// `CGarages::FindSafeHouseIndexForGarageType(int32)` (0x4489F0): the jump table at 0x448A78 (types
/// 16..45): 17 → 1, 18 → 2, 24..35 → 3..14, 39..42 → 15..18, 45 → 19; 0 otherwise.
#[hook(addr = 0x4489F0, conv = cdecl, name = "CGarages::FindSafeHouseIndexForGarageType", preserve = "ecx,edx", expect = "8B 44 24 04 83 C0 F0")]
pub fn find_safe_house_index_for_garage_type(ty: i32) -> i32 {
    match ty {
        17 => 1,
        18 => 2,
        24..=35 => ty - 21,
        39..=42 => ty - 24,
        45 => 19,
        _ => 0,
    }
}

/// `CEntryExitManager::EnableBurglaryHouses(bool)` (stub 0x43F180 → 0x156C6D0): the flag
/// (`ms_bBurglaryHousesEnabled` 0x96A730), then for every used entry-exit (last first) with flag 0x1000
/// (+0x30, burglary house) flag 0x4000 (enabled) = the argument.
#[hook(addr = 0x43F180, conv = cdecl, name = "CEntryExitManager::EnableBurglaryHouses", expect = "E9 4B D5 12 01")]
pub fn enable_burglary_houses(on: u8) {
    g::<u8>(0x96A730).write(on);
    let mut i = rd::<u32>(POOL_ENTRY_EXITS.get(), 8);
    while i != 0 {
        let pool = POOL_ENTRY_EXITS.get();
        let used = (rd::<GPtr<u8>>(pool, 4).byte_add(i as isize - 1).read() as i8) >= 0;
        i -= 1;
        let e = rd::<GPtr<u8>>(pool, 0).byte_add(i.wrapping_mul(0x3C) as isize);
        if !used || e.is_null() {
            continue;
        }
        let f = rd::<u16>(e, 0x30);
        if f & 0x1000 == 0 {
            continue;
        }
        wr::<u16>(e, 0x30, if on != 0 { f | 0x4000 } else { f & 0xBFFF });
    }
}

/// `CShopping` price-modifier set (0x49BDD0, `AddPriceModifier(uint32 key, int32 modifier)`): the
/// modifier of the key's entry (appended when absent and the count is not negative), then the price
/// (+4) of the key's price item = the modifier. EAX: `index * 3` of the price item, else the price count
/// (0 when not positive).
#[hook(addr = 0x49BDD0, conv = cdecl, name = "FUN_0049bdd0", expect = "8B 0D D0 A7 A9 00")]
pub fn add_price_modifier(key: u32, modifier: u32) -> i32 {
    let m = g::<u32>(PRICE_MODIFIERS);
    let n = g::<i32>(NUM_PRICE_MODIFIERS).read();
    let mut i = 0;
    if n > 0 {
        loop {
            if key == m.wrapping_index(i * 2).read() {
                m.wrapping_index(i * 2 + 1).write(modifier);
                break;
            }
            i += 1;
            if i >= n {
                break;
            }
        }
    }
    if i == n {
        m.wrapping_index(n * 2).write(key);
        m.wrapping_index(n * 2 + 1).write(modifier);
        g::<i32>(NUM_PRICE_MODIFIERS).write(n + 1);
    }
    let np = g::<i32>(0xA9A7CC).read();
    if np <= 0 {
        return 0;
    }
    for j in 0..np {
        if g::<u32>(0xA986F0 + j as usize * 0x18).read() == key {
            g::<u32>(0xA986F4 + j as usize * 0x18).write(modifier);
            return j * 3;
        }
    }
    np
}

/// `CShopping::IncrementStat(int32 index, int32 value)` (0x49BE70): nothing for -1; the stat of the
/// local table {0x15, 0x42, 0x50, 0x14F, 0x16, 0xF5}[index] (other indices read the original's stack:
/// not reproduced, nothing done), ×10 for 0x42 / 0x50, `CStats::ModifyStat(stat, (float)value)`.
#[hook(addr = 0x49BE70, conv = cdecl, name = "CShopping::IncrementStat", expect = "8B 44 24 04 83 EC 18")]
pub fn shopping_increment_stat(index: i32, value: i32) {
    const STATS: [i32; 6] = [0x15, 0x42, 0x50, 0x14F, 0x16, 0xF5];
    if index == -1 {
        return;
    }
    let Some(&stat) = STATS.get(index as usize) else { return };
    let v = if stat == 0x42 || stat == 0x50 { value.wrapping_mul(10) } else { value };
    super::stats::modify_stat(stat as u32, xi(v).to_f32().to_bits());
}

/// `InteriorManager_c::FindStealableObjectId(int32, int32 model, CVector pos)` (0x5982F0, thiscall,
/// `ret 0x14`): the index of the stealable record (0x1C each from +0x43AC) with +4 = the first argument,
/// +0 = the model and the position (+8..+0x10) equal; -1 when absent.
#[hook(addr = 0x5982F0, conv = thiscall, name = "InteriorManager_c::FindStealableObjectId-2", expect = "53 56 8B B1 A8 43 00 00", this = "InteriorManager_c")]
pub fn interior_manager_find_stealable_object_id2(this: GPtr<u8>, a: u32, model: u32, px: f32, py: f32, pz: f32) -> i32 {
    let n = rd::<i32>(this, 0x43A8);
    // test ah,0x44 / jp: taken unless Equal
    let eq = |v: f32, w: f32| !x(v).fcom(x(w)).test_parity(0x44);
    for k in 0..n.max(0) {
        let r = this.byte_add(0x43AC + k as isize * 0x1C);
        if rd::<u32>(r, 8) == a
            && rd::<u32>(r, 4) == model
            && eq(rd::<f32>(r, 0xC), px)
            && eq(rd::<f32>(r, 0x10), py)
            && eq(rd::<f32>(r, 0x14), pz)
        {
            return k;
        }
    }
    -1
}

// ---- batch 9: list-based constructors and managers ----

/// `InteriorManager_c::ReturnInteriorToPool(Interior_c*)` (0x5984B0, thiscall): tail jump to
/// `List_c::AddItem` on the pool list (+0x3CA0); EAX is the callee's.
#[hook(addr = 0x5984B0, conv = thiscall, name = "InteriorManager_c::ReturnInteriorToPool", expect = "81 C1 A0 3C 00 00", this = "InteriorManager_c")]
pub fn interior_manager_return_interior_to_pool(this: GPtr<u8>, int: GPtr<u8>) -> u32 {
    list_add_item(this.byte_add(0x3CA0), int)
}

/// `InteriorManager_c::GetInteriorFromPool` (0x5984A0, thiscall): tail jump to `List_c::RemoveHead` on
/// the pool list (+0x3CA0).
#[hook(addr = 0x5984A0, conv = thiscall, name = "InteriorManager_c::GetInteriorFromPool", expect = "81 C1 A0 3C 00 00", this = "InteriorManager_c")]
pub fn interior_manager_get_interior_from_pool(this: GPtr<u8>) -> GPtr<u8> {
    list_remove_head(this.byte_add(0x3CA0))
}

/// `FurnitureGroup_c::FurnitureGroup_c` (0x590DE0, thiscall): `List_c::List_c`; EAX = this.
#[hook(addr = 0x590DE0, conv = thiscall, name = "FurnitureGroup_c::FurnitureGroup_c", expect = "56 8B F1 E8 E8 7F F1 FF", this = "FurnitureGroup_c")]
pub fn furniture_group_ctor(this: GPtr<u8>) -> GPtr<u8> {
    list_ctor(this);
    this
}

/// `FurnitureItem::FurnitureItem` (0x590E00, thiscall): `ListItem_c::ListItem_c`; EAX = this.
#[hook(addr = 0x590E00, conv = thiscall, name = "FUN_00590e00", expect = "56 8B F1 E8 A8 7F F1 FF")]
pub fn furniture_item_ctor(this: GPtr<u8>) -> GPtr<u8> {
    list_item_ctor(this);
    this
}

/// `Furniture_c::Furniture_c` (0x590EC0, thiscall): `ListItem_c::ListItem_c`; EAX = this.
#[hook(addr = 0x590EC0, conv = thiscall, name = "Furniture_c::Furniture_c", expect = "56 8B F1 E8 E8 7E F1 FF")]
pub fn furniture_ctor(this: GPtr<u8>) -> GPtr<u8> {
    list_item_ctor(this);
    this
}

/// `InteriorGroup_c::InteriorGroup_c` (0x597FE0, thiscall): `ListItem_c::ListItem_c`; EAX = this.
#[hook(addr = 0x597FE0, conv = thiscall, name = "InteriorGroup_c::Constructor", expect = "56 8B F1 E8 C8 0D F1 FF", this = "InteriorGroup_c")]
pub fn interior_group_ctor(this: GPtr<u8>) -> GPtr<u8> {
    list_item_ctor(this);
    this
}

/// `WaterCreature_c::WaterCreature_c` (0x6E3E30, thiscall): `ListItem_c::ListItem_c`, +8 = 0; EAX = this.
#[hook(addr = 0x6E3E30, conv = thiscall, name = "FUN_006e3e30", expect = "56 8B F1 E8 78 4F DC FF", this = "WaterCreature_c")]
pub fn water_creature_ctor(this: GPtr<u8>) -> GPtr<u8> {
    list_item_ctor(this);
    wr::<u32>(this, 8, 0);
    this
}

/// `Interior_c::Interior_c` (0x5921D0, thiscall): `ListItem_c::ListItem_c`, `List_c::List_c` of +0x5C,
/// the words +0x3EC / +0x3F0 = 0xFFFF; EAX = this. (The SEH frame around the list constructor is not
/// built.)
#[hook(addr = 0x5921D0, conv = thiscall, name = "Interior_c::Constructor", expect = "6A FF 68 68 CF 83 00", this = "Interior_c")]
pub fn interior_ctor(this: GPtr<u8>) -> GPtr<u8> {
    list_item_ctor(this);
    list_ctor(this.byte_add(0x5C));
    wr::<u16>(this, 0x3EC, 0xFFFF);
    wr::<u16>(this, 0x3F0, 0xFFFF);
    this
}

/// `BreakManager_c::BreakManager_c` (0x59E620, thiscall): `eh vector constructor iterator` over the 64
/// `BreakObject_c` (0x20 each); EAX = this.
#[hook(addr = 0x59E620, conv = thiscall, name = "FUN_0059e620", expect = "56 68 80 D1 59 00")]
pub fn break_manager_ctor(this: GPtr<u8>) -> GPtr<u8> {
    eh_vector_ctor(this, 0x20, 0x40, 0x59D170, 0x59D180);
    this
}

/// `BreakManager_c::Update(float)` (0x59E670, thiscall, `ret 4`): `BreakObject_c::Update` of every
/// active (+1) object of the 64.
#[hook(addr = 0x59E670, conv = thiscall, name = "BreakManager_c::Update", expect = "53 8B 5C 24 08")]
pub fn break_manager_update(this: GPtr<u8>, step: u32) {
    for i in 0..0x40 {
        let o = this.byte_add(i * 0x20);
        if rd::<u8>(o, 1) != 0 {
            break_object_update(o, step);
        }
    }
}

/// `BreakManager_c::Add(CObject*, a, b, c)` (0x59E9B0, thiscall, `ret 0x10`): `BreakObject_c::Init` of the
/// first inactive (+1) object of the 64 with the four arguments; false when none is free or the init
/// fails.
#[hook(addr = 0x59E9B0, conv = thiscall, name = "BreakManager_c::Add", expect = "33 C0 8D 51 01")]
pub fn break_manager_add(this: GPtr<u8>, obj: GPtr<u8>, a: u32, b: u32, c: u32) -> u8 {
    for i in 0..0x40 {
        let o = this.byte_add(i * 0x20);
        if rd::<u8>(o, 1) == 0 {
            if o.is_null() || break_object_init(o, obj, a, b, c) == 0 {
                return 0;
            }
            return 1;
        }
    }
    0
}

/// `WaterCreatureManager_c::Init` (0x6E3F90, thiscall): the 128 creatures (0x34 each from this) are
/// added to the free list (+0x1A00), +0x1A18 = 0; returns true.
#[hook(addr = 0x6E3F90, conv = thiscall, name = "WaterCreatureManager_c::Init", expect = "53 55 56 8B E9")]
pub fn water_creature_manager_init(this: GPtr<u8>) -> u8 {
    for i in 0..0x80 {
        list_add_item(this.byte_add(0x1A00), this.byte_add(i * 0x34));
    }
    wr::<u32>(this, 0x1A18, 0);
    1
}

/// `FurnitureGroup_c::AddSubGroup(int32 id, a2..a5 unused, uint8, uint8, uint8)` (0x5910E0, thiscall,
/// `ret 0x20`): takes the next of the 128 sub-groups of the store (count word 0xBAB37C, 0x1C each at
/// 0xBAD3F8): +8 = id, +0x18..+0x1A = the three bytes, then `List_c::AddItem` on this; false when full.
#[hook(addr = 0x5910E0, conv = thiscall, name = "FurnitureGroup_c::AddSubGroup", expect = "66 8B 15 7C B3 BA 00", this = "FurnitureGroup_c")]
#[allow(clippy::too_many_arguments)]
pub fn furniture_group_add_sub_group(this: GPtr<u8>, id: u32, _a2: u32, _a3: u32, _a4: u32, _a5: u32, b6: u8, b7: u8, b8: u8) -> u8 {
    let n = g::<u16>(0xBAB37C).read();
    if n >= 0x80 {
        return 0;
    }
    let sg = g::<u8>(0xBAD3F8).byte_add(n as isize * 0x1C);
    g::<u16>(0xBAB37C).write(n + 1);
    wr::<u32>(sg, 8, id);
    wr::<u8>(sg, 0x18, b6);
    wr::<u8>(sg, 0x19, b7);
    wr::<u8>(sg, 0x1A, b8);
    list_add_item(this, sg);
    1
}

/// `CGarages::CountCarsInHideoutGarage(uint8 type)` (stub 0x44A210 → 0x1567330): the stored cars (model
/// word != 0, 4 per hideout) of the type's safe house.
#[hook(addr = 0x44A210, conv = cdecl, name = "CGarages::CountCarsInHideoutGarage", expect = "E9 1B D1 11 01")]
pub fn count_cars_in_hideout_garage(ty: u8) -> i32 {
    let idx = find_safe_house_index_for_garage_type(i32::from(ty));
    let base = 0x96ABEA + (idx << 8) as usize;
    (0..4).filter(|j| g::<u16>(base + j * 0x40).read() != 0).count() as i32
}

// ---- batch 10: furniture lookups, list destructors ----

/// `FurnitureGroup_c::~FurnitureGroup_c` (0x590DF0, thiscall): `jmp List_c::~List_c`.
#[hook(addr = 0x590DF0, conv = thiscall, name = "FurnitureGroup_c::~FurnitureGroup_c", expect = "E9 EB 7F F1 FF 90", this = "FurnitureGroup_c")]
pub fn furniture_group_dtor(this: GPtr<u8>) {
    list_dtor(this);
}

/// `FurnitureItem::~FurnitureItem` (0x590E10, thiscall): `jmp ListItem_c::~ListItem_c`.
#[hook(addr = 0x590E10, conv = thiscall, name = "(missed by ghidra: data-code-immediate)", expect = "E9 AB 7F F1 FF 90")]
pub fn furniture_item_dtor(this: GPtr<u8>) {
    list_item_dtor(this);
}

/// `Furniture_c::~Furniture_c` (0x590ED0, thiscall): `jmp ListItem_c::~ListItem_c`.
#[hook(addr = 0x590ED0, conv = thiscall, name = "Furniture_c::~Furniture_c", expect = "E9 EB 7E F1 FF 90")]
pub fn furniture_dtor(this: GPtr<u8>) {
    list_item_dtor(this);
}

/// `InteriorGroup_c::~InteriorGroup_c` (0x597FF0, thiscall): `jmp ListItem_c::~ListItem_c`.
#[hook(addr = 0x597FF0, conv = thiscall, name = "InteriorGroup_c::Destructor", expect = "E9 CB 0D F1 FF 90", this = "InteriorGroup_c")]
pub fn interior_group_dtor(this: GPtr<u8>) {
    list_item_dtor(this);
}

/// `WaterCreature_c::~WaterCreature_c` (0x6E3E50, thiscall): `jmp ListItem_c::~ListItem_c`.
#[hook(addr = 0x6E3E50, conv = thiscall, name = "(missed by ghidra: data-code-immediate)", expect = "E9 6B 4F DC FF 90", this = "WaterCreature_c")]
pub fn water_creature_dtor(this: GPtr<u8>) {
    list_item_dtor(this);
}

/// The sub-group with id `id` in the list starting at `head` (+4 next, +8 id), or null.
fn find_sub_group(head: GPtr<u8>, id: u32) -> GPtr<u8> {
    let mut n = head;
    while !n.is_null() {
        if rd::<u32>(n, 8) == id {
            return n;
        }
        n = rd::<GPtr<u8>>(n, 4);
    }
    GPtr::NULL
}

/// `FurnitureGroup_c::GetRandomId(int32 subGroup, uint8 rating)` (0x591170, thiscall, `ret 8`): the
/// sub-group's `GetRandomId(rating)`, -1 when absent.
#[hook(addr = 0x591170, conv = thiscall, name = "FurnitureGroup_c::GetRandomId", expect = "8B 09 85 C9 74 16", this = "FurnitureGroup_c")]
pub fn furniture_group_get_random_id(this: GPtr<u8>, sub: u32, rating: u32) -> i32 {
    let sg = find_sub_group(rd::<GPtr<u8>>(this, 0), sub);
    if sg.is_null() { -1 } else { sub_group_get_random_id(sg, rating) }
}

/// `FurnitureManager_c::GetRandomId(int32 group, int32 subGroup, uint8 rating)` (0x591220, thiscall,
/// `ret 0xC`): as `FurnitureGroup_c::GetRandomId` on group `group` (0xC each, list head at +0).
#[hook(addr = 0x591220, conv = thiscall, name = "FurnitureManager_c::GetRandomId", expect = "8B 44 24 04 8D 04 40", this = "FurnitureManager_c")]
pub fn furniture_manager_get_random_id(this: GPtr<u8>, group: i32, sub: u32, rating: u32) -> i32 {
    let sg = find_sub_group(rd::<GPtr<u8>>(this, group.wrapping_mul(0xC) as isize), sub);
    if sg.is_null() { -1 } else { sub_group_get_random_id(sg, rating) }
}

/// `FurnitureGroup_c::GetFurniture(int32 subGroup, int16 id, uint8 rating)` (0x591130, thiscall,
/// `ret 0xC`): the sub-group's `GetFurniture(id, rating)`, null when absent.
#[hook(addr = 0x591130, conv = thiscall, name = "FurnitureGroup_c::GetFurniture", expect = "8B 09 85 C9 74 16", this = "FurnitureGroup_c")]
pub fn furniture_group_get_furniture(this: GPtr<u8>, sub: u32, id: u32, rating: u32) -> GPtr<u8> {
    let sg = find_sub_group(rd::<GPtr<u8>>(this, 0), sub);
    if sg.is_null() { GPtr::NULL } else { sub_group_get_furniture(sg, id, rating) }
}

/// `FurnitureManager_c::GetFurniture(int32 group, int32 subGroup, int16 id, uint8 rating)` (0x5911E0,
/// thiscall, `ret 0x10`).
#[hook(addr = 0x5911E0, conv = thiscall, name = "FurnitureManager_c::GetFurniture", expect = "8B 44 24 04 8D 04 40", this = "FurnitureManager_c")]
pub fn furniture_manager_get_furniture(this: GPtr<u8>, group: i32, sub: u32, id: u32, rating: u32) -> GPtr<u8> {
    let sg = find_sub_group(rd::<GPtr<u8>>(this, group.wrapping_mul(0xC) as isize), sub);
    if sg.is_null() { GPtr::NULL } else { sub_group_get_furniture(sg, id, rating) }
}

// ---- batch 12 ----

/// `CShopping::RemoveLoadedShop` (stub 0x49AE30 → 0x15623F0): as `RemoveLoadedPrices`, with the byte
/// 0xA9A7D8 = 0 after the section name is read.
#[hook(addr = 0x49AE30, conv = cdecl, name = "CShopping::RemoveLoadedShop", expect = "E9 BB 75 0C 01")]
pub fn remove_loaded_shop() {
    let name = g::<GPtr<u8>>(0x8A61D8).wrapping_index(g::<i32>(SHOP_LOADED).read()).read();
    g::<u8>(0xA9A7D8).write(0);
    let b = get_animation_block_index(name);
    if b != -1 {
        set_model_is_deletable(b.wrapping_add(0x63E7));
    }
    g::<u32>(SHOP_LOADED).write(0);
}

/// `CEntryExit::RequestObjectsInFrustum` (0x43E690, thiscall): `CRenderer::RequestObjectsInDirection`
/// with a copy of the vector at +0x20 of `[0x96A7C0]`, this +0x2C and 0x20.
#[hook(addr = 0x43E690, conv = thiscall, name = "CEntryExit::RequestObjectsInFrustum", expect = "83 EC 0C A1 C0 A7 96 00", this = "CEntryExit")]
pub fn entry_exit_request_objects_in_frustum(this: GPtr<u8>) {
    let src = g::<GPtr<u8>>(0x96A7C0).read();
    let mut v = [0f32; 3];
    v[0] = rd::<f32>(src, 0x20);
    v[1] = x(rd::<f32>(src, 0x24)).to_f32();
    v[2] = x(rd::<f32>(src, 0x28)).to_f32();
    request_objects_in_direction(GPtr::from_raw(v.as_mut_ptr()), rd::<u32>(this, 0x2C), 0x20);
}

/// `FurnitureSubGroup_c::FurnitureSubGroup_c` (0x590E20, thiscall): `ListItem_c::ListItem_c`,
/// `List_c::List_c` of +0xC; EAX = this (the SEH frame is not built).
#[hook(addr = 0x590E20, conv = thiscall, name = "FurnitureSubGroup_c::FurnitureSubGroup_c", expect = "6A FF 68 88 CE 83 00", this = "FurnitureSubGroup_c")]
pub fn furniture_sub_group_ctor(this: GPtr<u8>) -> GPtr<u8> {
    list_item_ctor(this);
    list_ctor(this.byte_add(0xC));
    this
}

/// `WaterCreatureManager_c::WaterCreatureManager_c` (0x6E3EB0, thiscall): the 128 creatures (`eh vector
/// constructor iterator`, 0x34 each), `List_c::List_c` of +0x1A00 and +0x1A0C; EAX = this (the SEH
/// frame is not built).
#[hook(addr = 0x6E3EB0, conv = thiscall, name = "FUN_006e3eb0", expect = "6A FF 68 54 88 84 00")]
pub fn water_creature_manager_ctor(this: GPtr<u8>) -> GPtr<u8> {
    eh_vector_ctor(this, 0x34, 0x80, 0x6E3E30, 0x6E3E50);
    list_ctor(this.byte_add(0x1A00));
    list_ctor(this.byte_add(0x1A0C));
    this
}

// ---- batch 13 ----

/// `CStats::UpdateStatsOnRespawn` (0x55CFC0): the respawn counter at 0xB79508 (as unsigned: + 2^32 when
/// negative, PC=24 rounding) not above the float stat at 0xB78FF4 is incremented; else it is reset to 0,
/// and when the stat at 0xB793E0 is above 400 stat 0x18 is first incremented by the float at 0xB78F74
/// and the update message shown.
#[hook(addr = 0x55CFC0, conv = cdecl, name = "CStats::UpdateStatsOnRespawn", expect = "51 A1 08 95 B7 00")]
pub fn update_stats_on_respawn() {
    let t = g::<i32>(0xB79508).read();
    let mut v = xi(t);
    if t < 0 {
        v = v.fadd_m(f32::from_bits(0x4F80_0000));
    }
    // test ah,0x41 / jne: unless Greater the counter is incremented instead
    if v.fcom(x(g::<f32>(0xB78FF4).read())).test_any(0x41) {
        g::<i32>(0xB79508).write(t.wrapping_add(1));
        return;
    }
    if !x(g::<f32>(0xB793E0).read()).fcom(x(400.0)).test_any(0x41) {
        increment_stat(0x18, g::<u32>(0xB78F74).read());
        display_script_stat_update_message(0, 0x18, g::<u32>(0xB78F74).read());
    }
    g::<u32>(0xB79508).write(0);
}

/// `InteriorGroup_c::CalcIsVisible` (0x595200, thiscall): +0x35 = +0x34; +0x34 = 1 when any of the
/// `+0xF` (signed) interiors `IsVisible`, else 0.
#[hook(addr = 0x595200, conv = thiscall, name = "InteriorGroup_c::CalcIsVisible", expect = "56 8B F1 8A 46 34", this = "InteriorGroup_c")]
pub fn interior_group_calc_is_visible(this: GPtr<u8>) {
    wr::<u8>(this, 0x35, rd::<u8>(this, 0x34));
    wr::<u8>(this, 0x34, 0);
    let mut i = 0;
    while i < i32::from(rd::<i8>(this, 0xF)) {
        if interior_is_visible(rd::<GPtr<u8>>(this, 0x10 + i as isize * 4)) != 0 {
            wr::<u8>(this, 0x34, 1);
            return;
        }
        i += 1;
    }
}

/// `FurnitureGroup_c::AddFurniture(int32 subGroup, a2..a6)` (0x5C0230, thiscall, `ret 0x18`):
/// `FurnitureSubGroup_c::AddFurniture(a2..a6)` on every sub-group with the id; returns true.
#[hook(addr = 0x5C0230, conv = thiscall, name = "FurnitureGroup_c::AddFurniture", expect = "56 8B 31 85 F6 74 36", this = "FurnitureGroup_c")]
#[allow(clippy::too_many_arguments)]
pub fn furniture_group_add_furniture(this: GPtr<u8>, sub: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32) -> u8 {
    let mut n = rd::<GPtr<u8>>(this, 0);
    while !n.is_null() {
        if rd::<u32>(n, 8) == sub {
            sub_group_add_furniture(n, a2, a3, a4, a5, a6);
        }
        n = rd::<GPtr<u8>>(n, 4);
    }
    1
}

/// `BreakManager_c::Render(bool)` (0x59E6A0, thiscall, `ret 4`): render states 0xC=1, 0xA=5, 0xB=6,
/// 0xE=1, then `BreakObject_c::Render` of every active (+1) object of the 64.
#[hook(addr = 0x59E6A0, conv = thiscall, name = "BreakManager_c::Render", expect = "A1 24 7B C9 00")]
pub fn break_manager_render(this: GPtr<u8>, flag: u32) {
    rw_render_state_set(0xC, 1);
    rw_render_state_set(0xA, 5);
    rw_render_state_set(0xB, 6);
    rw_render_state_set(0xE, 1);
    for i in 0..0x40 {
        let o = this.byte_add(i * 0x20);
        if rd::<u8>(o, 1) != 0 {
            break_object_render(o, flag);
        }
    }
}

/// `CEntryExit::ProcessStealableObjects(CPed*)` (0x43E990, thiscall, `ret 4`; this unused): for an
/// object (type 4) held by the ped with flag 0x2000 at +0x140 and object type (+0x13C) neither 2 nor 6,
/// outside an interior (`[0x96A7C0]` area +0x32 = 0) it becomes stolen in `g_interiorMan` (0xBAF670),
/// inside one it is un-stolen when known; then its area (+0x2F) = that area (also for types 2 / 6).
#[hook(addr = 0x43E990, conv = thiscall, name = "CEntryExit::ProcessStealableObjects", expect = "8B 4C 24 04 56", this = "CEntryExit")]
pub fn entry_exit_process_stealable_objects(_this: GPtr<u8>, ped: GPtr<u8>) {
    let o = get_entity_that_this_ped_is_holding(ped);
    if o.is_null() || rd::<u8>(o, 0x36) & 7 != 4 || rd::<u32>(o, 0x140) & 0x2000 == 0 {
        return;
    }
    let t = rd::<u8>(o, 0x13C);
    if t != 2 && t != 6 {
        let im = g::<u8>(0xBAF670);
        if rd::<u8>(g::<GPtr<u8>>(0x96A7C0).read(), 0x32) == 0 {
            interior_manager_set_stealable_object_stolen(im, o, 1);
        } else if interior_manager_find_stealable_object_id(im, o) != -1 {
            interior_manager_set_stealable_object_stolen(im, o, 0);
        }
    }
    wr::<u8>(o, 0x2F, rd::<u8>(g::<GPtr<u8>>(0x96A7C0).read(), 0x32));
}

// ---- batch 15: interiors ----

/// `Interior_c::GetTileCentre(float x, float y, CVector* out)` (0x591BD0, thiscall, `ret 0xC`): the
/// local point `(-w/2 + x + 0.5, -d/2 + y + 0.5, -h/2)` (info +0x14: bytes +2/+3/+4; PC=24 additions),
/// transformed in place by the interior's matrix (+0x18, `RwV3dTransformPoints`).
#[hook(addr = 0x591BD0, conv = thiscall, name = "Interior_c::GetTileCentre", expect = "51 8B 41 14 0F B6 50 02", this = "Interior_c")]
pub fn interior_get_tile_centre(this: GPtr<u8>, tx: f32, ty: f32, out: GPtr<f32>) {
    let info = rd::<GPtr<u8>>(this, 0x14);
    let half = |b: isize| xi(-i32::from(rd::<u8>(info, b))).fmul_m(0.5);
    out.write(half(2).fadd_m(tx).fadd_m(0.5).to_f32());
    out.wrapping_index(1).write(half(3).fadd_m(ty).fadd_m(0.5).to_f32());
    out.wrapping_index(2).write(half(4).to_f32());
    rw_v3d_transform_points(out, out, 1, this.byte_add(0x18));
}

/// `InteriorGroup_c::GetRandomInterior` (0x5948C0, thiscall): `k = _ftol(rand/32768 * count)` (count
/// +0xF sign-extended; `fimul`), the k-th non-null of the 8 interior slots, else null.
#[hook(addr = 0x5948C0, conv = thiscall, name = "InteriorGroup_c::GetRandomInterior", expect = "83 EC 08 56 8B F1", this = "InteriorGroup_c")]
pub fn interior_group_get_random_interior(this: GPtr<u8>) -> GPtr<u8> {
    let count = i32::from(rd::<i8>(this, 0xF));
    let r = sa_core::orig::at_site(0x5948D4, sa_core::crt::rand) & 0xFFFF;
    let k = ftol32(xi(r).fmul_m(f32::from_bits(0x3800_0000)).fmul(xi(count)));
    let mut seen = 0;
    for s in 0..8 {
        let int = rd::<GPtr<u8>>(this, 0x10 + s * 4);
        if int.is_null() {
            continue;
        }
        if seen == k {
            return int;
        }
        seen += 1;
    }
    GPtr::NULL
}

/// `InteriorManager_c::AddSameGroupEffectInfos(ff_t* info, int32)` (0x598430, thiscall, `ret 8`): for the
/// entity's (`info+0`) model's 2D effects (count byte +0xD, re-read each step) other than the first listed
/// one (`info+0x28`): an interior effect (type +0xC = 5) of the same group (+0x11) as `info+8`'s is
/// appended (`info+8[]`, its index to `info+0x28[]`, count `info+4`) while fewer than 8.
#[hook(addr = 0x598430, conv = thiscall, name = "InteriorManager_c::AddSameGroupEffectInfos", expect = "55 56 8B 74 24 0C", this = "InteriorManager_c")]
pub fn interior_manager_add_same_group_effect_infos(_this: GPtr<u8>, info: GPtr<u8>, _unused: u32) {
    let model = i32::from(rd::<i16>(rd::<GPtr<u8>>(info, 0), 0x22));
    let mi = g::<GPtr<u8>>(0xA9B0C8).wrapping_index(model).read();
    let mut i = 0i32;
    while i < i32::from(rd::<u8>(mi, 0xD)) {
        if i != rd::<i32>(info, 0x28) {
            let fx = get_2d_effect(mi, i);
            if rd::<u8>(fx, 0xC) == 5 && rd::<u8>(rd::<GPtr<u8>>(info, 8), 0x11) == rd::<u8>(fx, 0x11) {
                let n = rd::<i32>(info, 4);
                if n < 8 {
                    wr::<GPtr<u8>>(info, 8 + n as isize * 4, fx);
                    let n = rd::<i32>(info, 4);
                    wr::<i32>(info, 0x28 + n as isize * 4, i);
                    wr::<i32>(info, 4, n.wrapping_add(1));
                }
            }
        }
        i += 1;
    }
}

/// `InteriorGroup_c::ReferenceAnims` (0x5950D0, thiscall): once (+0xBA): the animation block of the
/// group type (+0xE: 0 "int_house", 1 "int_shop", 2 "int_office"; other types use the saved ECX, i.e.
/// `this`, as the index: original bug) gets a reference when loaded (and +0xBA = 1), else is requested
/// (`RequestModel(index + 0x63E7, 8)`).
#[hook(addr = 0x5950D0, conv = thiscall, name = "InteriorGroup_c::ReferenceAnims", expect = "51 56 8B F1 8A 86 BA 00 00 00", this = "InteriorGroup_c")]
pub fn interior_group_reference_anims(this: GPtr<u8>) {
    if rd::<u8>(this, 0xBA) != 0 {
        return;
    }
    let idx = match rd::<u8>(this, 0xE) {
        0 => get_animation_block_index(g(0x866D40)),
        1 => get_animation_block_index(g(0x866D34)),
        2 => get_animation_block_index(g(0x866D28)),
        _ => this.addr() as i32,
    };
    if g::<u8>(0xB5D4B0).byte_add((idx << 5) as isize).read() == 1 {
        add_anim_block_ref(idx);
        wr::<u8>(this, 0xBA, 1);
    } else {
        request_model(idx.wrapping_add(0x63E7), 8);
    }
}

// ---- batch 17 ----

/// `CStats::FindMaxNumberOfGroupMembers` (0x559A50): by the respect stat (0xB79480): below 10 → 0, 60
/// → 2, 160 → 3, 330 → 4, 540 → 5, 800 → 6, else 7 (each test: `test ah,5 / jp`, true only for Less).
#[hook(addr = 0x559A50, conv = cdecl, name = "CStats::FindMaxNumberOfGroupMembers", preserve = "edx", expect = "51 A1 80 94 B7 00")]
pub fn find_max_number_of_group_members() -> i32 {
    let v = g::<f32>(0xB79480).read();
    for (k, n) in [(10.0, 0), (60.0, 2), (160.0, 3), (330.0, 4), (540.0, 5), (800.0, 6)] {
        if !x(v).fcom(x(k)).test_parity(0x05) {
            return n;
        }
    }
    7
}

/// `CPickups::RemovePickUpsInArea(float x1, float x2, float y1, float y2, float z1, float z2)` (0x456D30):
/// every pickup with a type whose position (packed int16 / 8) is inside the box (bounds included) is
/// removed.
#[hook(addr = 0x456D30, conv = cdecl, name = "CPickups::RemovePickUpsInArea", expect = "51 56 BE D0 88 97 00")]
pub fn remove_pick_ups_in_area(x1: f32, x2: f32, y1: f32, y2: f32, z1: f32, z2: f32) {
    let k = f32::from_bits(0x3E00_0000);
    // `test ah,1 / jne` skips Less / Unordered against the low bound; `test ah,0x41 / jp` skips Greater /
    // Unordered against the high bound.
    let inside = |c: i16, lo: f32, hi: f32| {
        let v = xi(i32::from(c)).fmul_m(k);
        !v.fcom(x(lo)).test_any(0x01) && !v.fcom(x(hi)).test_parity(0x41)
    };
    for i in 0..pickups::NUM_PICKUPS as u32 {
        let p = pickups::pickup(i);
        if rd::<u8>(p, 0x1C) == 0 {
            continue;
        }
        if inside(rd::<i16>(p, 0x10), x1, x2) && inside(rd::<i16>(p, 0x12), y1, y2) && inside(rd::<i16>(p, 0x14), z1, z2) {
            pickups::remove(p);
        }
    }
}

/// `strcpy` as the inlined byte loop does it.
fn copy_str(dst: GPtr<u8>, src: GPtr<u8>) {
    let mut i = 0;
    loop {
        let c = src.byte_add(i).read();
        dst.byte_add(i).write(c);
        if c == 0 {
            break;
        }
        i += 1;
    }
}

/// `CSaveGarage::CopyGarageIntoSaveGarage(const CGarage&)` (0x5D3020, thiscall, `ret 4`): type, door
/// state, flags (+0x4C..+0x4E) → +0..+2; the 16 dwords +0..+0x3C → +4..+0x40; name (+0x44) copied;
/// original type +0x4F → +0x4C.
#[hook(addr = 0x5D3020, conv = thiscall, name = "CSaveGarage::CopyGarageIntoSaveGarage", expect = "8B 44 24 04 8A 50 4C", this = "CSaveGarage")]
pub fn copy_garage_into_save_garage(this: GPtr<u8>, gp: GPtr<u8>) {
    for k in 0..3 {
        wr::<u8>(this, k, rd::<u8>(gp, 0x4C + k));
    }
    for k in 0..16 {
        wr::<u32>(this, 4 + k * 4, rd::<u32>(gp, k * 4));
    }
    copy_str(this.byte_add(0x44), gp.byte_add(0x44));
    wr::<u8>(this, 0x4C, rd::<u8>(gp, 0x4F));
}

/// `CSaveGarage::CopyGarageOutOfSaveGarage(CGarage&)` (0x5D30C0, thiscall, `ret 4`): the reverse, with
/// +0x40 of the garage = 0 (only 0x3C+4 bytes restored: +0x40 of the save goes to +0x3C).
#[hook(addr = 0x5D30C0, conv = thiscall, name = "CSaveGarage::CopyGarageOutOfSaveGarage", expect = "8A 11 8B 44 24 04", this = "CSaveGarage")]
pub fn copy_garage_out_of_save_garage(this: GPtr<u8>, gp: GPtr<u8>) {
    for k in 0..3 {
        wr::<u8>(gp, 0x4C + k, rd::<u8>(this, k));
    }
    for k in 0..16 {
        wr::<u32>(gp, k * 4, rd::<u32>(this, 4 + k * 4));
    }
    wr::<u32>(gp, 0x40, 0);
    copy_str(gp.byte_add(0x44), this.byte_add(0x44));
    wr::<u8>(gp, 0x4F, rd::<u8>(this, 0x4C));
}

// ---- batch 20 ----

/// `CPickups::TryToMerge_WeaponType(CVector pos, eWeaponType, ePickupType, uint32 ammo, ...)` (0x4555A0):
/// the first pickup of the type whose model (+0x18) is the weapon's model (`GetWeaponInfo(w, 1)` +0xC)
/// within 5.5 (3D, the `RemoveUnnecessaryPickups` x87 order) gets the ammo added (+8); true then.
#[hook(addr = 0x4555A0, conv = cdecl, name = "CPickups::TryToMerge_WeaponType", expect = "8B 44 24 10 83 EC 1C")]
pub fn try_to_merge_weapon_type(px: f32, py: f32, pz: f32, weapon: i32, ty: u8, ammo: u32) -> u8 {
    let model = rd::<u32>(get_weapon_info(weapon, 1), 0xC);
    let k = f32::from_bits(0x3E00_0000);
    for i in 0..pickups::NUM_PICKUPS as u32 {
        let p = pickups::pickup(i);
        if rd::<u8>(p, 0x1C) != ty || u32::from(rd::<u16>(p, 0x18)) != model {
            continue;
        }
        let qx = xi(i32::from(rd::<i16>(p, 0x10))).fmul_m(k);
        let qy = xi(i32::from(rd::<i16>(p, 0x12))).fmul_m(k);
        let qz = xi(i32::from(rd::<i16>(p, 0x14))).fmul_m(k).to_f32();
        let dx = qx.fsub_m(px).to_f32();
        let dy = qy.fsub_m(py);
        let dz = x(qz).fsub_m(pz);
        let d = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(x(dx).fmul_m(dx)).fsqrt();
        // test ah,5 / jnp: merged when Less
        if !d.fcom(x(5.5)).test_parity(0x05) {
            wr::<u32>(p, 8, rd::<u32>(p, 8).wrapping_add(ammo));
            return 1;
        }
    }
    0
}

/// `CGameLogic::StopPlayerMovingFromDirection(int32 id, CVector dir)` (0x441290): the moving entity is the
/// player's vehicle when in one, else the ped when +0x46C bit 0 or a jetpack task is set (else nothing).
/// A negative `dir.z` is taken as 0; when `dot = (dir.y·v.y + dir.x·v.x) + v.z·dir.z` is negative the
/// move speed (+0x44) loses `dir·dot` (each component stored as f32); the speed is written back in every
/// case.
#[hook(addr = 0x441290, conv = cdecl, name = "CGameLogic::StopPlayerMovingFromDirection", expect = "8B 44 24 04 69 C0 90 01 00 00")]
pub fn stop_player_moving_from_direction(id: i32, dx: f32, dy: f32, dz: f32) {
    let ped = rd::<GPtr<u8>>(g::<u8>(super::cworld::PLAYERS).byte_add(id.wrapping_mul(0x190) as isize), 0);
    let flags = rd::<u32>(ped, 0x46C);
    let veh = rd::<GPtr<u8>>(ped, 0x58C);
    let e = if flags & 0x100 != 0 && !veh.is_null() {
        veh
    } else if flags & 1 != 0 || !get_task_jet_pack(rd::<GPtr<u8>>(ped, 0x47C)).is_null() {
        ped
    } else {
        return;
    };
    // test ah,5 / jp: zeroed only when Less
    let dz = if !x(dz).fcom(X87::zero()).test_parity(0x05) { 0.0 } else { dz };
    let mut s = [rd::<u32>(e, 0x44), rd::<u32>(e, 0x48), rd::<u32>(e, 0x4C)];
    let f = f32::from_bits;
    let dot = x(dy).fmul_m(f(s[1])).fadd(x(dx).fmul_m(f(s[0]))).fadd(x(f(s[2])).fmul_m(dz));
    if !dot.fcom(X87::zero()).test_parity(0x05) {
        let ax = x(dx).fmul(dot);
        let ay = x(dy).fmul(dot);
        let az = dot.fmul_m(dz).to_f32();
        s[0] = x(f(s[0])).fsub(ax).to_f32().to_bits();
        s[1] = x(f(s[1])).fsub(ay).to_f32().to_bits();
        s[2] = x(f(s[2])).fsub_m(az).to_f32().to_bits();
    }
    wr::<u32>(e, 0x44, s[0]);
    wr::<u32>(e, 0x48, s[1]);
    wr::<u32>(e, 0x4C, s[2]);
}

// ---- batch 21: interior tiles ----

/// `Interior_c::CheckTilesEmpty(int32 x, int32 y, int32 w, int32 d, bool allow9)` (0x591680, thiscall,
/// `ret 0x14`): false for a negative origin or a rectangle beyond the info's width/depth (signed); else
/// true unless a tile (`+0x68[(x + i) * 30 + y + j]`) is non-zero and (not `allow9` or not 9).
#[hook(addr = 0x591680, conv = thiscall, name = "Interior_c::CheckTilesEmpty", expect = "8B 44 24 04 85 C0", this = "Interior_c")]
pub fn interior_check_tiles_empty(this: GPtr<u8>, tx: i32, ty: i32, w: i32, d: i32, allow9: u8) -> u8 {
    if tx < 0 || ty < 0 {
        return 0;
    }
    let info = rd::<GPtr<u8>>(this, 0x14);
    if tx.wrapping_add(w) > i32::from(rd::<u8>(info, 2)) || ty.wrapping_add(d) > i32::from(rd::<u8>(info, 3)) {
        return 0;
    }
    let base = this.byte_add(0x68 + tx.wrapping_mul(0x1E).wrapping_add(ty) as isize);
    for i in 0..w.max(0) {
        for j in 0..d.max(0) {
            let t = base.byte_add((i * 0x1E + j) as isize).read();
            if t != 0 && (allow9 == 0 || t != 9) {
                return 0;
            }
        }
    }
    1
}

/// `Interior_c::SetTilesStatus(int32 x, int32 y, int32 w, int32 d, int32 status, bool force)` (0x591700,
/// thiscall, `ret 0x18`): with the same bounds checks (nothing done when failing), every tile: 9 with
/// status 5 → 10; else with `force` all but 5, 7, 8 take the status; without, 0 takes it, 3 takes status
/// 4, and a 3 with status 3 stops the whole function there.
#[hook(addr = 0x591700, conv = thiscall, name = "Interior_c::SetTilesStatus", expect = "8B 44 24 04 85 C0", this = "Interior_c")]
#[allow(clippy::too_many_arguments)]
pub fn interior_set_tiles_status(this: GPtr<u8>, tx: i32, ty: i32, w: i32, d: i32, status: i32, force: u8) {
    if tx < 0 || ty < 0 {
        return;
    }
    let info = rd::<GPtr<u8>>(this, 0x14);
    if tx.wrapping_add(w) > i32::from(rd::<u8>(info, 2)) || ty.wrapping_add(d) > i32::from(rd::<u8>(info, 3)) {
        return;
    }
    let base = this.byte_add(0x68 + tx.wrapping_mul(0x1E).wrapping_add(ty) as isize);
    for i in 0..w.max(0) {
        for j in 0..d.max(0) {
            let p = base.byte_add((i * 0x1E + j) as isize);
            let t = p.read();
            if t == 9 && status == 5 {
                p.write(10);
            } else if force != 0 {
                if !matches!(t, 5 | 7 | 8) {
                    p.write(status as u8);
                }
            } else if t == 3 {
                if status == 3 {
                    return;
                }
                if status == 4 {
                    p.write(4);
                }
            } else if t == 0 {
                p.write(status as u8);
            }
        }
    }
}

/// `InteriorGroup_c::DereferenceAnims` (0x595160, thiscall): when referenced (+0xBA), the block's
/// reference is removed (`RemoveAnimBlockRef`; same index choice and `this` bug as `ReferenceAnims`) and
/// +0xBA = 0.
#[hook(addr = 0x595160, conv = thiscall, name = "InteriorGroup_c::DereferenceAnims", expect = "51 56 8B F1 8A 86 BA 00 00 00", this = "InteriorGroup_c")]
pub fn interior_group_dereference_anims(this: GPtr<u8>) {
    if rd::<u8>(this, 0xBA) == 0 {
        return;
    }
    let idx = match rd::<u8>(this, 0xE) {
        0 => get_animation_block_index(g(0x866D40)),
        1 => get_animation_block_index(g(0x866D34)),
        2 => get_animation_block_index(g(0x866D28)),
        _ => this.addr() as i32,
    };
    remove_anim_block_ref(idx);
    wr::<u8>(this, 0xBA, 0);
}

// ---- batch 22: interior tiles (random, point test) ----

/// `Interior_c::GetRandomTile(int32 status, int32* outX, int32* outY)` (0x591B20, thiscall, `ret 0xC`):
/// draws `x = _ftol(rand/32768 * width)`, `y = _ftol(rand/32768 * depth)` (`fimul`) until the tile status
/// (`GetTileStatus` inlined: 1 outside) is `status`; then the coordinates are stored.
#[hook(addr = 0x591B20, conv = thiscall, name = "Interior_c::GetRandomTile", expect = "51 53 8B 5C 24 0C", this = "Interior_c")]
pub fn interior_get_random_tile(this: GPtr<u8>, status: i32, out_x: GPtr<i32>, out_y: GPtr<i32>) {
    let k = f32::from_bits(0x3800_0000);
    // The two `rand` calls return to 0x591B3A and 0x591B68 (randcalls trace key).
    let draw = |site: u32, n: i32| ftol32(xi(sa_core::orig::at_site(site, sa_core::crt::rand) & 0xFFFF).fmul_m(k).fmul(xi(n)));
    loop {
        let tx = draw(0x591B3A, i32::from(rd::<u8>(rd::<GPtr<u8>>(this, 0x14), 2)));
        let ty = draw(0x591B68, i32::from(rd::<u8>(rd::<GPtr<u8>>(this, 0x14), 3)));
        let info = rd::<GPtr<u8>>(this, 0x14);
        let t = if tx >= i32::from(rd::<u8>(info, 2)) || ty >= i32::from(rd::<u8>(info, 3)) || tx < 0 || ty < 0 {
            1
        } else {
            i32::from(rd::<u8>(this, 0x68 + (tx * 0x1E + ty) as isize))
        };
        if t == status {
            out_x.write(tx);
            out_y.write(ty);
            return;
        }
    }
}

/// x87 `fabs` (clears the sign bit, NaN included).
fn fabs(a: X87) -> X87 {
    X87::from_f64_exact(a.to_f64().abs())
}

/// `Interior_c::IsPtInside(const CVector* pt, CVector margin)` (0x5913E0, thiscall, `ret 0x10`): the point
/// relative to the centre (+0x48, each component stored as f32) projected on the three matrix axes
/// (+0x18 / +0x28 / +0x38: `|(a.y·d.y + a.x·d.x) + a.z·d.z|`) must not exceed half the info's
/// width / depth / height plus the margin (false only when that bound is Less).
#[hook(addr = 0x5913E0, conv = thiscall, name = "Interior_c::IsPtInside", expect = "83 EC 0C D9 41 48", this = "Interior_c")]
pub fn interior_is_pt_inside(this: GPtr<u8>, pt: GPtr<f32>, mx: f32, my: f32, mz: f32) -> u8 {
    let d = [
        x(rd::<f32>(this, 0x48)).fsubr_m(pt.read()).to_f32(),
        x(pt.wrapping_index(1).read()).fsub_m(rd::<f32>(this, 0x4C)).to_f32(),
        x(pt.wrapping_index(2).read()).fsub_m(rd::<f32>(this, 0x50)).to_f32(),
    ];
    let info = rd::<GPtr<u8>>(this, 0x14);
    for (axis, size, m) in [(0x18isize, 2isize, mx), (0x28, 3, my), (0x38, 4, mz)] {
        let a = fabs(
            x(rd::<f32>(this, axis + 4))
                .fmul_m(d[1])
                .fadd(x(rd::<f32>(this, axis)).fmul_m(d[0]))
                .fadd(x(rd::<f32>(this, axis + 8)).fmul_m(d[2])),
        );
        let b = xi(i32::from(rd::<u8>(info, size))).fmul_m(0.5).fadd_m(m);
        // fcompp (bound vs projection), test ah,5 / jnp: false when Less
        if !b.fcom(a).test_parity(0x05) {
            return 0;
        }
    }
    1
}

/// `Interior_c::GetNumEmptyTiles(int32 x, int32 y, int32 dir, int32 len)` (0x591920, thiscall, `ret 0x10`):
/// the number of whole empty strips of `len` tiles stepping away from (x, y): directions 0 / 2 scan rows
/// along x and step y by -1 / +1, directions 1 / 3 scan columns along y and step x by +1 / -1. A strip
/// leaving the info's width / depth or holding a non-zero tile ends the count. Original bug kept: with
/// `len <= 0` it never returns.
#[hook(addr = 0x591920, conv = thiscall, name = "Interior_c::GetNumEmptyTiles", expect = "83 EC 0C 8B 54 24 18", this = "Interior_c")]
pub fn interior_get_num_empty_tiles(this: GPtr<u8>, tx: i32, ty: i32, dir: i32, len: i32) -> i32 {
    let step = if dir == 3 || dir == 0 { -1 } else { 1 };
    let along_y = dir == 3 || dir == 1;
    let mut count = 0i32;
    let (mut ox, mut oy) = (tx, ty);
    loop {
        for k in 0..len.max(0) {
            let (cx, cy) = if along_y { (ox, ty.wrapping_add(k)) } else { (tx.wrapping_add(k), oy) };
            let info = rd::<GPtr<u8>>(this, 0x14);
            if cx >= i32::from(rd::<u8>(info, 2)) || cy >= i32::from(rd::<u8>(info, 3)) || cx < 0 || cy < 0 {
                return count;
            }
            if rd::<u8>(this, 0x68 + cx.wrapping_mul(0x1E).wrapping_add(cy) as isize) != 0 {
                return count;
            }
        }
        count = count.wrapping_add(1);
        if along_y {
            ox = ox.wrapping_add(step);
        } else {
            oy = oy.wrapping_add(step);
        }
    }
}

// ---- CSetPieces, break objects, furniture ----

/// `CSetPieces::Init` (stub 0x4994F0 → 0x1562350): `NumSetPieces` = 0, `bDebug` = 0.
#[hook(addr = 0x4994F0, conv = cdecl, name = "CSetPieces::Init", expect = "E9 5B 8E 0C 01")]
pub fn set_pieces_init() {
    NUM_SET_PIECES.set(0);
    SET_PIECES_DEBUG.set(0);
}

/// `FurnitureGroup_c::Init` (0x5910A0, thiscall): returns 1.
#[hook(addr = 0x5910A0, conv = thiscall, name = "FurnitureGroup_c::Init", expect = "B8 01 00 00 00 C3", this = "FurnitureGroup_c")]
pub fn furniture_group_init(_this: GPtr<u8>) -> i32 {
    1
}

/// `BreakManager_c::Init` (0x59E650, thiscall): `mov al,1; ret` (3 bytes, not hookable).
pub fn break_manager_init(_this: GPtr<u8>) -> u8 {
    1
}

/// `BreakObject_c::BreakObject_c` (0x59D170, thiscall): +0, +1 = 0, +8 = 0; EAX = this.
#[hook(addr = 0x59D170, conv = thiscall, name = "BreakObject_c::BreakObject_c", expect = "8B C1 33 C9 88 08 88 48", this = "BreakObject_c")]
pub fn break_object_ctor(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u8>(this, 0, 0);
    wr::<u8>(this, 1, 0);
    wr::<u32>(this, 8, 0);
    this
}

/// `BreakObject_c::~BreakObject_c` (0x59D180, thiscall): `ret` (1 byte, not hookable).
#[sa_core::native(addr = 0x59D180, conv = thiscall, name = "(missed by ghidra: data-code-immediate)")]
pub fn break_object_dtor(_this: GPtr<u8>) {}

/// `memcmp(a, b, n) == 0` as `repe cmpsb` evaluates it (stops at the first difference).
fn bytes_eq(a: GPtr<u8>, lit: &[u8]) -> bool {
    lit.iter().enumerate().all(|(i, &c)| a.byte_add(i as isize).read() == c)
}

/// `FurnitureManager_c::GetGroupId(const char*)` (0x5BFB40, thiscall, `ret 4`): the index of the group
/// name (compared with its terminator), -1 when unknown.
#[hook(addr = 0x5BFB40, conv = thiscall, name = "FurnitureManager_c::GetGroupId", expect = "8B 44 24 04 56 57 BF CC", this = "FurnitureManager_c")]
pub fn furniture_get_group_id(_this: GPtr<u8>, name: GPtr<u8>) -> i32 {
    const NAMES: [(&[u8], i32); 6] = [
        (b"IT_SHOP\0", 0),
        (b"IT_OFFICE\0", 1),
        (b"IT_LOUNGE\0", 2),
        (b"IT_BEDROOM\0", 3),
        (b"IT_KITCHEN\0", 4),
        (b"IT_MISC\0", 8),
    ];
    for (lit, id) in NAMES {
        if bytes_eq(name, lit) {
            return id;
        }
    }
    -1
}

/// `FurnitureManager_c::GetSubGroupId(const char*)` (0x5BFBF0, thiscall, `ret 4`): the sub-group index of
/// the name within its group (51 names, each compared with its terminator, `repe cmpsb`), -1 when unknown
/// (STD-6).
#[hook(addr = 0x5BFBF0, conv = thiscall, name = "FurnitureManager_c::GetSubGroupId", expect = "8B 44 24 04 56 57 BF 0C AF 86 00", this = "FurnitureManager_c",
       unverified = "std6-world-1", route = "L1-pure", hazards = "none", recs = "new-game")]
pub fn furniture_get_sub_group_id(_this: GPtr<u8>, name: GPtr<u8>) -> i32 {
    const NAMES: [(&[u8], i32); 51] = [
        (b"SHOP_UNIT1_L\0", 0),
        (b"SHOP_UNIT1_R\0", 1),
        (b"SHOP_UNIT1_M\0", 2),
        (b"SHOP_UNIT2_L\0", 3),
        (b"SHOP_UNIT2_R\0", 4),
        (b"SHOP_UNIT2_M\0", 5),
        (b"SHOP_UNIT3_L\0", 6),
        (b"SHOP_UNIT3_R\0", 7),
        (b"SHOP_UNIT3_M\0", 8),
        (b"SHOP_UNIT4_L\0", 9),
        (b"SHOP_UNIT4_R\0", 10),
        (b"SHOP_UNIT4_M\0", 11),
        (b"SHOP_CHECKOUTS\0", 12),
        (b"SHOP_BASKETS\0", 13),
        (b"SHOP_LIGHTS\0", 14),
        (b"OFFICE_DESKS\0", 0),
        (b"OFFICE_CHAIRS\0", 1),
        (b"OFFICE_WATERCOOLERS\0", 2),
        (b"OFFICE_FILINGCABS\0", 3),
        (b"LOUNGE_SOFAS\0", 0),
        (b"LOUNGE_CHAIRS\0", 1),
        (b"LOUNGE_TVSTANDS\0", 2),
        (b"LOUNGE_TVS\0", 3),
        (b"LOUNGE_TABLES\0", 4),
        (b"LOUNGE_LAMPS\0", 5),
        (b"LOUNGE_HIFI_TABLES\0", 6),
        (b"LOUNGE_VIDEOS\0", 7),
        (b"LOUNGE_HIFIS\0", 8),
        (b"LOUNGE_CONSOLES\0", 9),
        (b"BEDROOM_BEDS\0", 0),
        (b"BEDROOM_TABLES\0", 1),
        (b"BEDROOM_DRAWERS\0", 2),
        (b"BEDROOM_WARDROBES\0", 3),
        (b"KITCHEN_UNIT_L\0", 0),
        (b"KITCHEN_UNIT_M\0", 1),
        (b"KITCHEN_UNIT_R\0", 2),
        (b"KITCHEN_UNIT_SINKS\0", 3),
        (b"KITCHEN_UNIT_FRIDGES\0", 4),
        (b"KITCHEN_UNIT_OVENS\0", 5),
        (b"KITCHEN_UNIT_WASHERS\0", 6),
        (b"KITCHEN_UNIT_CORNERS\0", 7),
        (b"KITCHEN_MICROWAVES\0", 8),
        (b"KITCHEN_TOASTERS\0", 9),
        (b"KITCHEN_TVS\0", 10),
        (b"MISC_PLANTS\0", 0),
        (b"MISC_LIGHTS\0", 1),
        (b"MISC_RUGS\0", 2),
        (b"MISC_PLATES\0", 3),
        (b"MISC_FOOD\0", 4),
        (b"MISC_MAGAZINES\0", 5),
        (b"MISC_CLOTHES\0", 6),
    ];
    for (lit, id) in NAMES {
        if bytes_eq(name, lit) {
            return id;
        }
    }
    -1
}

/// `FurnitureSubGroup_c::AddFurniture(uint16 model, int16 id, uint8 wealthMin, uint8 wealthMax, uint8
/// maxAng)` (0x5C00C0, thiscall, `ret 0x14`, bool in AL): below 256 records (count u16 at 0xBAB378), the
/// next `Furniture_c` (0x14 each from 0xBAE1F8): +8 model, +0xA id; the tile size from the model's
/// collision box (`this+0x1A` set: `max - min`, else `max + 0.5`, x then y), each `_ftol`'d into a byte
/// plus one unless the fraction is below 0.02 (`test ah,5; jnp`; the y size goes through an f32 store,
/// the x size stays in ST); +0xE / +0xF wealth, +0x13 maxAng, +0x10..+0x12 the sub-group's bytes
/// +0x18..+0x1A; then `List_c::AddItem(this+0xC, rec)` (STD-6).
#[hook(addr = 0x5C00C0, conv = thiscall, name = "FurnitureSubGroup_c::AddFurniture", expect = "83 EC 18 57 8B F9", this = "FurnitureSubGroup_c",
       unverified = "std6-world-1", route = "L2", hazards = "none", recs = "new-game")]
pub fn sub_group_add_furniture(this: GPtr<u8>, model: u32, id: u32, wealth_min: u32, wealth_max: u32, max_ang: u32) -> u8 {
    let count = g::<u16>(0xBAB378);
    let n = count.read();
    if n >= 0x100 {
        return 0;
    }
    count.write(n + 1);
    let rec = GPtr::<u8>::from_addr(0xBAE1F8 + usize::from(n) * 0x14);
    let model = model as u16;
    wr(rec, 8, model);
    wr(rec, 0xA, id as u16);
    let mi = g::<GPtr<u8>>(0xA9B0C8).wrapping_index(i32::from(model)).read();
    let col: GPtr<u8> = rd(mi, 0x14);
    let (min_x, min_y): (f32, f32) = (rd(col, 0), rd(col, 4));
    let (max_x, max_y): (f32, f32) = (rd(col, 0xC), rd(col, 0x10));
    let (sx, sy) = if rd::<u8>(this, 0x1A) != 0 {
        (x(max_x).fsub_m(min_x), x(max_y).fsub_m(min_y))
    } else {
        let half = f32::from_bits(0x3F00_0000);
        (x(max_x).fadd_m(half), x(max_y).fadd_m(half))
    };
    let k = f32::from_bits(0x3CA3_D70A);
    let size = |v: X87| {
        let t = ftol32(v);
        let mut b = t as u8;
        if v.fsub(xi(t)).fcom(x(k)).test_parity(0x05) {
            b = b.wrapping_add(1);
        }
        b
    };
    wr(rec, 0xC, size(sx));
    wr(rec, 0xD, size(x(sy.to_f32())));
    wr(rec, 0xE, wealth_min as u8);
    wr(rec, 0xF, wealth_max as u8);
    wr(rec, 0x13, max_ang as u8);
    wr(rec, 0x10, rd::<u8>(this, 0x18));
    wr(rec, 0x11, rd::<u8>(this, 0x19));
    wr(rec, 0x12, rd::<u8>(this, 0x1A));
    list_add_item(this.wrapping_index(0xC), rec);
    1
}

/// `CSetPieces::AddOne(uint8 type, CVector2D a, CVector2D b, CVector2D spawn1, CVector2D target1, CVector2D
/// spawn2, CVector2D target2)` (0x499500): below 210 set pieces, the next record (0x20 each from
/// 0xA95818): +0x1C type byte; the area's min / max corner (+4..+0xA: min x, min y, max x, max y; `test
/// ah,5` picks the first only when Less, `test ah,0x41` the first only when Greater) and the four points
/// (+0xC spawn1, +0x14 target1, +0x10 spawn2, +0x18 target2) as int16 `_ftol(v * 4)`; the count is
/// incremented.
#[hook(addr = 0x499500, conv = cdecl, name = "CSetPieces::AddOne", expect = "57 8B 3D FC 57 A9 00")]
#[allow(clippy::too_many_arguments)]
pub fn set_pieces_add_one(
    ty: u32,
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
    s1x: f32,
    s1y: f32,
    t1x: f32,
    t1y: f32,
    s2x: f32,
    s2y: f32,
    t2x: f32,
    t2y: f32,
) {
    let n = NUM_SET_PIECES.get() as i32;
    if n >= 0xD2 {
        return;
    }
    let r = g::<u8>(0xA95818).byte_add(n as isize * 0x20);
    wr::<u8>(r, 0x1C, ty as u8);
    let min = |a: f32, b: f32| if !x(a).fcom(x(b)).test_parity(0x05) { a } else { b };
    let max = |a: f32, b: f32| if !x(a).fcom(x(b)).test_any(0x41) { a } else { b };
    let q = |v: f32| ftol32(x(v).fmul_m(4.0)) as u16;
    wr::<u16>(r, 4, q(min(ax, bx)));
    wr::<u16>(r, 6, q(min(ay, by)));
    wr::<u16>(r, 8, q(max(ax, bx)));
    wr::<u16>(r, 0xA, q(max(ay, by)));
    wr::<u16>(r, 0xC, q(s1x));
    wr::<u16>(r, 0xE, q(s1y));
    wr::<u16>(r, 0x14, q(t1x));
    wr::<u16>(r, 0x16, q(t1y));
    wr::<u16>(r, 0x10, q(s2x));
    wr::<u16>(r, 0x12, q(s2y));
    wr::<u16>(r, 0x18, q(t2x));
    wr::<u16>(r, 0x1A, q(t2y));
    NUM_SET_PIECES.set((n + 1) as u32);
}

// ---- CGameLogic, CStats, water creatures ----

fn above(v: f32, k: f32) -> bool {
    // test ah,0x41 / je
    !x(v).fcom(x(k)).test_any(0x41)
}

fn below(v: f32, k: f32) -> bool {
    // test ah,5 / jnp
    !x(v).fcom(x(k)).test_parity(0x05)
}

/// `CGameLogic::LaRiotsActiveHere` (0x441C10): false above z 950; true with the riot cheat (0x969175);
/// else, with `CGameLogic::bLaRiotsActive`... (0xB72958), true inside either of the two boxes
/// (1620 < x < 2824, -2178 < y < -1067) / (157 < x < 1630, -1950 < y < -1192). `FindPlayerCoors` is
/// called twice.
#[hook(addr = 0x441C10, conv = cdecl, name = "CGameLogic::LaRiotsActiveHere", expect = "83 EC 18 8D 44 24 0C 6A")]
pub fn la_riots_active_here() -> u8 {
    let mut a = [0f32; 3];
    let mut b = [0f32; 3];
    let r = find_player_coors(GPtr::from_raw(a.as_mut_ptr()), -1);
    if above(r.wrapping_index(2).read(), 950.0) {
        return 0;
    }
    if g::<u8>(0x969175).read() != 0 {
        return 1;
    }
    if g::<u8>(0xB72958).read() == 0 {
        return 0;
    }
    find_player_coors(GPtr::from_raw(b.as_mut_ptr()), -1);
    let (px, py) = (b[0], b[1]);
    if above(px, 1620.0) && below(px, 2824.0) && above(py, -2178.0) && below(py, -1067.0) {
        return 1;
    }
    (above(px, 157.0) && below(px, 1630.0) && above(py, -1950.0) && below(py, -1192.0)) as u8
}

/// `CStats::PopulateFavoriteRadioStationList` (0x558EC0): copies the 14 listen times of the audio
/// engine into `FavoriteRadioStationList` (0xB78E58); true when all are 0.
#[hook(addr = 0x558EC0, conv = cdecl, name = "CStats::PopulateFavoriteRadioStationList", expect = "83 EC 0C 53 55 56 57 B9")]
pub fn populate_favorite_radio_station_list() -> u8 {
    let src = get_radio_station_listen_times(g(0xB6BC90));
    let mut all_zero = 1;
    for i in 0..14 {
        let v = src.wrapping_index(i).read();
        g::<u32>(0xB78E58).wrapping_index(i).write(v);
        if v != 0 {
            all_zero = 0;
        }
    }
    all_zero
}

/// `(rand() & 0xFFFF) * (1/32768) * k`, truncated by `_ftol`; `site` is the original return address of the
/// `rand` call (sa-hook's rand hook keys the randcalls trace on it).
fn rand_times(site: u32, k: f32) -> i32 {
    let r = sa_core::orig::at_site(site, sa_core::crt::rand) & 0xFFFF;
    ftol32(xi(r).fmul_m(f32::from_bits(0x3800_0000)).fmul_m(k))
}

/// `WaterCreatureManager_c::GetRandomWaterCreatureId` (0x6E4040, thiscall): `p = rand * 100`; below 80
/// `rand * 3`, below 90 `3 - rand * -2`, else 5 + (p >= 97). Each draw is `(rand() & 0xFFFF) / 32768`.
#[hook(addr = 0x6E4040, conv = thiscall, name = "WaterCreatureManager_c::GetRandomWaterCreatureId", expect = "51 E8 D8 DA 13 00")]
pub fn get_random_water_creature_id(_this: GPtr<u8>) -> i32 {
    let p = rand_times(0x6E4046, 100.0);
    if p < 0x50 {
        return rand_times(0x6E406E, 3.0);
    }
    if p < 0x5A {
        return 3i32.wrapping_sub(rand_times(0x6E4099, -2.0));
    }
    5 + (p >= 0x61) as i32
}

/// Registrations of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_IS_POINT_IN_A_GARAGE_CAMERA_ZONE,
    HOOK_IS_POINT_WITHIN_ANY_GARAGE,
    HOOK_INIT_AFTER_RESTART,
    HOOK_GARAGES_PLAYER_ARRESTED_OR_DIED,
    HOOK_RESET_AREA_CODE_FOR_VISIBLE_OBJECTS,
    HOOK_ENTRY_EXIT_DELETE_ONE,
    HOOK_SET_AREA_CODE_FOR_VISIBLE_OBJECTS,
    HOOK_INTERIOR_SET_ENTRY_EXIT_PTR,
    HOOK_REMOVE_UNNECESSARY_PICKUPS,
    HOOK_REMOVE_LOADED_PRICES,
    HOOK_REMOVE_PRICE_MODIFIER,
    HOOK_SHOPPING_SHUTDOWN_FOR_RESTART,
    HOOK_INCREMENT_STAT2,
    HOOK_SET_PIECE_GET_SPAWN_COORD1,
    HOOK_SET_PIECE_GET_SPAWN_COORD2,
    HOOK_SET_PIECE_GET_TARGET_COORD1,
    HOOK_SET_PIECE_GET_TARGET_COORD2,
    HOOK_INTERIOR_GROUP_GET_PED,
    HOOK_INTERIOR_GROUP_ADD_INTERIOR,
    HOOK_INTERIOR_GROUP_SETUP_OFFICE_PEDS,
    HOOK_INTERIOR_GET_FURNITURE_ENTITY,
    HOOK_INTERIOR_MANAGER_ACTIVATE_PEDS,
    HOOK_INTERIOR_MANAGER_IS_GROUP_ACTIVE,
    HOOK_INTERIOR_MANAGER_FIND_STEALABLE_OBJECT_ID,
    HOOK_INTERIOR_MANAGER_SET_STEALABLE_OBJECT_STOLEN,
    HOOK_IS_CAR_SPRAYABLE,
    HOOK_STOP_CAR_FROM_BLOWING_UP,
    HOOK_IS_POINT_WITHIN_HIDE_OUT_GARAGE,
    HOOK_MODIFY_STRING_LABEL_FOR_CONTROL_SETTING,
    HOOK_INTERIOR_MANAGER_GET_PEDS_INTERIOR_GROUP,
    HOOK_INTERIOR_GROUP_GET_NUM_INTERIOR_INFOS,
    HOOK_INTERIOR_GROUP_REMOVE_PED,
    HOOK_INTERIOR_GROUP_INIT,
    HOOK_INTERIOR_GROUP_CONTAINS_INTERIOR_TYPE,
    HOOK_INTERIOR_GET_TILE_STATUS,
    HOOK_INTERIOR_MANAGER_HAS_INTERIOR_HAD_STEAL_DATA_SETUP,
    HOOK_CAN_ADD_WATER_CREATURE_AT_POS,
    HOOK_SET_PLAYER_WANTED_LEVEL_FOR_FORBIDDEN_TERRITORIES,
    HOOK_FIND_GARAGE_FOR_OBJECT,
    HOOK_FIND_SAFE_HOUSE_INDEX_FOR_GARAGE_TYPE,
    HOOK_ENABLE_BURGLARY_HOUSES,
    HOOK_ADD_PRICE_MODIFIER,
    HOOK_SHOPPING_INCREMENT_STAT,
    HOOK_INTERIOR_MANAGER_FIND_STEALABLE_OBJECT_ID2,
    HOOK_INTERIOR_MANAGER_RETURN_INTERIOR_TO_POOL,
    HOOK_INTERIOR_MANAGER_GET_INTERIOR_FROM_POOL,
    HOOK_FURNITURE_GROUP_CTOR,
    HOOK_FURNITURE_ITEM_CTOR,
    HOOK_FURNITURE_CTOR,
    HOOK_INTERIOR_GROUP_CTOR,
    HOOK_WATER_CREATURE_CTOR,
    HOOK_INTERIOR_CTOR,
    HOOK_BREAK_MANAGER_CTOR,
    HOOK_BREAK_MANAGER_UPDATE,
    HOOK_BREAK_MANAGER_ADD,
    HOOK_WATER_CREATURE_MANAGER_INIT,
    HOOK_FURNITURE_GROUP_ADD_SUB_GROUP,
    HOOK_COUNT_CARS_IN_HIDEOUT_GARAGE,
    HOOK_FURNITURE_GROUP_DTOR,
    HOOK_FURNITURE_ITEM_DTOR,
    HOOK_FURNITURE_DTOR,
    HOOK_INTERIOR_GROUP_DTOR,
    HOOK_WATER_CREATURE_DTOR,
    HOOK_FURNITURE_GROUP_GET_RANDOM_ID,
    HOOK_FURNITURE_MANAGER_GET_RANDOM_ID,
    HOOK_FURNITURE_GROUP_GET_FURNITURE,
    HOOK_FURNITURE_MANAGER_GET_FURNITURE,
    HOOK_REMOVE_LOADED_SHOP,
    HOOK_ENTRY_EXIT_REQUEST_OBJECTS_IN_FRUSTUM,
    HOOK_FURNITURE_SUB_GROUP_CTOR,
    HOOK_WATER_CREATURE_MANAGER_CTOR,
    HOOK_UPDATE_STATS_ON_RESPAWN,
    HOOK_INTERIOR_GROUP_CALC_IS_VISIBLE,
    HOOK_FURNITURE_GROUP_ADD_FURNITURE,
    HOOK_BREAK_MANAGER_RENDER,
    HOOK_ENTRY_EXIT_PROCESS_STEALABLE_OBJECTS,
    HOOK_INTERIOR_GET_TILE_CENTRE,
    HOOK_INTERIOR_GROUP_GET_RANDOM_INTERIOR,
    HOOK_INTERIOR_MANAGER_ADD_SAME_GROUP_EFFECT_INFOS,
    HOOK_INTERIOR_GROUP_REFERENCE_ANIMS,
    HOOK_FIND_MAX_NUMBER_OF_GROUP_MEMBERS,
    HOOK_REMOVE_PICK_UPS_IN_AREA,
    HOOK_COPY_GARAGE_INTO_SAVE_GARAGE,
    HOOK_COPY_GARAGE_OUT_OF_SAVE_GARAGE,
    HOOK_TRY_TO_MERGE_WEAPON_TYPE,
    HOOK_STOP_PLAYER_MOVING_FROM_DIRECTION,
    HOOK_INTERIOR_CHECK_TILES_EMPTY,
    HOOK_INTERIOR_SET_TILES_STATUS,
    HOOK_INTERIOR_GROUP_DEREFERENCE_ANIMS,
    HOOK_INTERIOR_GET_RANDOM_TILE,
    HOOK_INTERIOR_IS_PT_INSIDE,
    HOOK_INTERIOR_GET_NUM_EMPTY_TILES,
    HOOK_SET_PIECES_ADD_ONE,
    HOOK_SET_PIECES_INIT,
    HOOK_FURNITURE_GROUP_INIT,
    HOOK_BREAK_OBJECT_CTOR,
    HOOK_FURNITURE_GET_GROUP_ID,
    HOOK_FURNITURE_GET_SUB_GROUP_ID,
    HOOK_SUB_GROUP_ADD_FURNITURE,
    HOOK_LA_RIOTS_ACTIVE_HERE,
    HOOK_POPULATE_FAVORITE_RADIO_STATION_LIST,
    HOOK_GET_RANDOM_WATER_CREATURE_ID,
];
