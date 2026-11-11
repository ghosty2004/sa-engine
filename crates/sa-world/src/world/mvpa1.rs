//! WRITER-MVP-A batch `mvpa-world-1`: WORLD functions on the native boot/loading path (replay
//! reset, shopping init, interior/furniture/break managers' exit, entry-exit lookups, game-logic skip).
//! Ported from the 1.0 US bytes, SecuROM fragments followed and their obfuscated loads decoded from the
//! image (gta-reversed as a reading aid only). Unverified (docs/verify/HOWTO.md).

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};

use sa_math::x87::X87;

use super::common::{g, rd, wr, x};

/// `CPools::ms_pPedPool`
const PED_POOL: usize = 0xB74490;
/// `CPools::ms_pVehiclePool`
const VEHICLE_POOL: usize = 0xB74494;
/// `CEntryExitManager::mp_poolEntryExits`
const ENTRY_EXIT_POOL: usize = 0x96A7D8;
/// `TheCamera`
const THE_CAMERA: usize = 0xB6F028;
/// `CGameLogic::SkipState`
const SKIP_STATE: usize = 0x96A8A0;

orig! {
    /// `CShopping::LoadStats()` (0x49B6A0)
    fn shopping_load_stats() = cdecl 0x49B6A0 => unported;
    /// `BreakObject_c::Exit()` (0x59DDD0)
    fn break_object_exit(o: GPtr<u8>) = thiscall 0x59DDD0 => unported;
    /// `List_c::RemoveAll()` (0x4A8EB0; no catalogue name: clears a `List_c` head/tail/count)
    fn list_remove_all(l: GPtr<u8>) = thiscall 0x4A8EB0 => unported;
    /// `InteriorGroup_c::Exit()` (0x595290)
    fn interior_group_exit(g: GPtr<u8>) = thiscall 0x595290 => unported;
    /// `FurnitureManager_c::Exit()` (0x5911A0)
    fn furniture_manager_exit(m: GPtr<u8>) = thiscall 0x5911A0 => furniture_manager_exit_port;
    /// CRT `_strnicmp` (0x835AC7)
    fn strnicmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x835AC7 => unported;
    /// `CCamera::SetFadeColour(uint8 r, uint8 g, uint8 b)` (0x50BF00, `ret 0xC`)
    fn camera_set_fade_colour(c: GPtr<u8>, r: u32, gg: u32, b: u32) = thiscall 0x50BF00 => unported;
    /// `CCamera::Fade(float duration, eFadeFlag)` (0x50AC20, `ret 8`)
    fn camera_fade(c: GPtr<u8>, t: u32, dir: u32) = thiscall 0x50AC20 => unported;
    /// `CPad::GetPad(int32)` (0x53FB70)
    fn get_pad(n: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CVehicle::ExtinguishCarFire()` (0x6D2460)
    fn vehicle_extinguish_car_fire(v: GPtr<u8>) = thiscall 0x6D2460 => unported;
    /// `CObject::Create(CDummyObject*)` (0x5A2070)
    fn object_create_from_dummy(d: GPtr<u8>) -> GPtr<u8> = cdecl 0x5A2070 => unported;
    /// `CScriptsForBrains::CheckIfNewEntityNeedsScript(CEntity*, int8 type, void*)` (0x46FF20, `ret 0xC`)
    fn brains_check_if_new_entity_needs_script(b: GPtr<u8>, e: GPtr<u8>, t: u32, u: u32) = thiscall 0x46FF20 => unported;
    /// `CVehicleModelInfo::SetVehicleColour(uint8, uint8, uint8, uint8)` (0x4C84B0, `ret 0x10`)
    fn vmi_set_vehicle_colour(mi: GPtr<u8>, a: u32, b: u32, c: u32, d: u32) = thiscall 0x4C84B0 => unported;
    /// `CEntity::Render()` (0x534310)
    fn entity_render(e: GPtr<u8>) = thiscall 0x534310 => unported;
    /// `CVisibilityPlugins::SetUserValue(RpAtomic*, uint16)` (0x732380)
    fn vis_set_user_value(a: GPtr<u8>, v: u32) = cdecl 0x732380 => unported;
    /// CRT `ceil(double)` (0x823820, ST0)
    fn crt_ceil(v: f64) -> f64 = cdecl 0x823820 => unported;
    /// `List_c::RemoveItem(ListItem_c*)` (0x4A8E30, `ret 4`)
    fn list_remove_item(l: GPtr<u8>, item: GPtr<u8>) = thiscall 0x4A8E30 => unported;
    /// `List_c::AddItem(ListItem_c*)` (0x4A8DF0, `ret 4`: inserts at the head)
    fn list_add_item(l: GPtr<u8>, item: GPtr<u8>) = thiscall 0x4A8DF0 => unported;
    /// `CWorld::Remove(CEntity*)` (0x563280)
    fn world_remove(e: GPtr<u8>) = cdecl 0x563280 => unported;
    /// virtual `CEntity::~CEntity` deleting destructor (vtable +0)
    fn v_delete(e: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    /// virtual `CBaseModelInfo::CreateInstance()` (vtable +0x2C)
    fn v_mi_create_instance(mi: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x2C => unported;
    /// virtual `CPhysical::SetIsStatic(bool)` (vtable +0x10)
    fn v_set_is_static(e: GPtr<u8>, b: u32) = thiscall vslot 0x10 => unported;
    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70, `ret 4`)
    fn matrix_update_rw_matrix(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD70 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40, `ret 4`)
    fn simple_transform_update_rw_matrix(t: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x54EF40 => unported;
    /// `CTagManager::IsTag(const CEntity*)` (0x49CCE0, AL)
    fn tag_manager_is_tag(e: GPtr<u8>) -> u8 = cdecl 0x49CCE0 => unported;
    /// `CTagManager` user-value setter (0x49CD50; no catalogue name)
    fn tag_set_user_value(e: GPtr<u8>) = cdecl 0x49CD50 => tag_manager_setup_atomic;
    /// `CCustomBuildingDNPipeline::PreRenderUpdate(RpAtomic*, bool)` (0x5D7200)
    fn dn_pre_render_update(a: GPtr<u8>, b: u32) = cdecl 0x5D7200 => unported;
    /// `CPhysical::AddToMovingList()` (0x542800)
    fn physical_add_to_moving_list(e: GPtr<u8>) = thiscall 0x542800 => unported;
    /// `operator new` of the 0xC-byte list links (0x5523C0)
    fn link_new(size: u32) -> GPtr<u8> = cdecl 0x5523C0 => unported;
    /// insert of a 0xC-byte link into the list given as argument (0x5329A0, `ret 4`; no catalogue name)
    fn link_insert(link: GPtr<u8>, list: GPtr<u8>) = thiscall 0x5329A0 => unported;
    /// `RpAnimBlendClumpGetFirstAssociation(RpClump*)` (0x4D15E0)
    fn clump_get_first_association(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D15E0 => unported;
    /// `CAnimBlendAssociation::SetCurrentTime(float)` (0x4CEA80, `ret 4`)
    fn assoc_set_current_time(a: GPtr<u8>, t: f32) = thiscall 0x4CEA80 => unported;
    /// `CBaseModelInfo::AddRef()` (0x4C4BA0)
    fn mi_add_ref(mi: GPtr<u8>) = thiscall 0x4C4BA0 => unported;
    /// `CStreaming::AddEntity(CEntity*)` (0x409650)
    fn streaming_add_entity(e: GPtr<u8>) -> GPtr<u8> = cdecl 0x409650 => unported;
    /// `CEntity::CreateEffects()` (0x533790)
    fn entity_create_effects(e: GPtr<u8>) = thiscall 0x533790 => unported;
    /// `GetFirstAtomic(RpClump*)` (0x734820)
    fn get_first_atomic(c: GPtr<u8>) -> GPtr<u8> = cdecl 0x734820 => unported;
    /// `CCustomBuildingRenderer::IsCBPCPipelineAttached(RpAtomic*)` (0x5D7F40, AL)
    fn is_cbpc_pipeline_attached(a: GPtr<u8>) -> u8 = cdecl 0x5D7F40 => unported;
    /// `CReplay::MarkEverythingAsNew()` (0x45D430)
    fn replay_mark_everything_as_new() = cdecl 0x45D430 => mark_everything_as_new;
    /// `CEntryExitManager::GetEntryExitIndex(const char*, uint16, uint16)` (0x43EFD0)
    fn get_entry_exit_index_call(name: GPtr<u8>, en: u32, dis: u32) -> i32 = cdecl 0x43EFD0 => get_entry_exit_index;
    /// `CRadar::ClearBlipForEntity(eBlipType, int32 handle)` (0x587C60)
    fn pickups_clear_blip_for_entity(ty: i32, handle: i32) = cdecl 0x587C60 => unported;
    /// `CFileMgr::SetDir` (0x5387D0)
    fn file_set_dir(path: GPtr<u8>) -> i32 = cdecl 0x5387D0 => sa_core::filemgr::set_dir;
    /// `CFileMgr::OpenFile` (0x538900)
    fn file_open(path: GPtr<u8>, mode: GPtr<u8>) -> GPtr<u8> = cdecl 0x538900 => sa_core::filemgr::open_file;
    /// `CFileMgr::CloseFile` (0x5389D0)
    fn file_close(f: GPtr<u8>) -> i32 = cdecl 0x5389D0 => sa_core::filemgr::close_file;
    /// `CFileLoader::LoadLine(FILE*)` (0x536F80)
    fn load_line(f: GPtr<u8>) -> GPtr<u8> = cdecl 0x536F80 => unported;
    /// CRT `sscanf` (0x8220AD) with three targets
    fn sscanf3(s: GPtr<u8>, fmt: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
    /// CRT `_strupr` (0x836C58)
    fn crt_strupr(s: GPtr<u8>) -> GPtr<u8> = cdecl 0x836C58 => unported;
    /// `CStats::CheckForStatsMessage()` (0x559760)
    fn stats_check_for_stats_message() = cdecl 0x559760 => unported;
    /// `CStats::PopulateFavoriteRadioStationList()` (0x558EC0)
    fn stats_populate_favorite_radio_station_list() -> u8 = cdecl 0x558EC0 => crate::world::misc2::populate_favorite_radio_station_list;
    /// `CStats::LoadActionReactionStats()` (0x5599B0)
    fn stats_load_action_reaction_stats() = cdecl 0x5599B0 => load_action_reaction_stats;
    /// `CStats::LoadStatUpdateConditions()` (0x559860)
    fn stats_load_stat_update_conditions() = cdecl 0x559860 => load_stat_update_conditions;
    /// CRT `sscanf` (0x8220AD) with five targets
    fn sscanf5(s: GPtr<u8>, fmt: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>, c: GPtr<u8>, d: GPtr<u8>, e: GPtr<u8>) -> i32 = cdecl 0x8220AD => unported;
    /// `InteriorManager_c::ReturnInteriorToPool(Interior_c*)` (0x5984B0, `ret 4`)
    fn interior_return_to_pool(m: GPtr<u8>, i: GPtr<u8>) = thiscall 0x5984B0 => unported;
    /// `FurnitureGroup_c::Init()` (0x5910A0)
    fn furniture_group_init(gr: GPtr<u8>) -> u8 = thiscall 0x5910A0 => unported;
    /// `FurnitureManager_c::LoadFurniture()` (0x5C0280)
    fn furniture_load_furniture(m: GPtr<u8>) -> u8 = thiscall 0x5C0280 => unported;
    /// `CQuadTreeNode::DeleteItem(void*)` (0x552A40, `ret 4`)
    fn quad_tree_delete_item(q: GPtr<u8>, item: GPtr<u8>) = thiscall 0x552A40 => unported;
    /// `CEntryExitManager::EnableBurglaryHouses(bool)` (0x43F180)
    fn enable_burglary_houses(b: u32) = cdecl 0x43F180 => unported;
    /// `CEntryExitManager::ResetAreaCodeForVisibleObjects()` (0x43ED80)
    fn reset_area_code_for_visible_objects() = cdecl 0x43ED80 => unported;
    /// `CCutsceneMgr::DeleteCutsceneData()` (0x4D5ED0)
    fn cutscene_delete_cutscene_data() = cdecl 0x4D5ED0 => unported;
    /// `CProjectileInfo::RemoveAllProjectiles()` (0x7399B0)
    fn remove_all_projectiles() = cdecl 0x7399B0 => unported;
    /// `CObject::DeleteAllTempObjects()` (0x5A18B0)
    fn object_delete_all_temp_objects() = cdecl 0x5A18B0 => unported;
    /// `CObject::DeleteAllMissionObjects()` (0x5A1910)
    fn object_delete_all_mission_objects() = cdecl 0x5A1910 => unported;
    /// `CPopulation::ConvertAllObjectsToDummyObjects()` (0x616420)
    fn population_convert_all_objects_to_dummy_objects() = cdecl 0x616420 => unported;
    /// virtual `CEntity::Remove()` (vtable +0xC)
    fn v_entity_remove(e: GPtr<u8>) = thiscall vslot 0xC => unported;
    /// `CPhysical::RemoveFromMovingList()` (0x542860)
    fn physical_remove_from_moving_list(e: GPtr<u8>) = thiscall 0x542860 => unported;
    /// `CPickups::ReInit()` (0x456E60)
    fn pickups_re_init_call() = cdecl 0x456E60 => pickups_re_init;
    /// `CPools::CheckPoolsEmpty()` (0x551950)
    fn pools_check_pools_empty() = cdecl 0x551950 => unported;
}

/// Slot `i` of the `CPool` at `pool` (objects of `size` bytes): the object address, or `None` when the
/// byte map's bit 7 marks it free. The original tests the computed address against null as well; that
/// test is kept by the callers.
fn pool_slot(pool: GPtr<u8>, i: i32, size: i32) -> Option<GPtr<u8>> {
    let map = rd::<GPtr<u8>>(pool, 4);
    if (map.byte_add(i as isize).read() as i8) < 0 {
        return None;
    }
    Some(rd::<GPtr<u8>>(pool, 0).byte_add(i.wrapping_mul(size) as isize))
}

/// `CReplay::MarkEverythingAsNew()` (0x45D430, `nop; jmp` fragment loading the ped pool): walks the ped
/// pool from the last slot down and clears bit 13 of the dword at +0x470 (`bHasBeenRendered`... the
/// replay's "already recorded" flag) of every used, non-null slot, then the vehicle pool (last slot
/// down; the byte-map bit tested with `test byte, 0x80`) clearing bit 2 of byte +0x42B.
#[hook(addr = 0x45D430, conv = cdecl, name = "CReplay::MarkEverythingAsNew", expect = "90 E9 9F 84 FA FF",
       route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn mark_everything_as_new() {
    let peds = g::<GPtr<u8>>(PED_POOL).read();
    let mut i = rd::<i32>(peds, 8);
    while i != 0 {
        i -= 1;
        if let Some(p) = pool_slot(peds, i, 0x7C4)
            && !p.is_null()
        {
            wr::<u32>(p, 0x470, rd::<u32>(p, 0x470) & 0xFFFF_DFFF);
        }
    }
    let vehs = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let mut i = rd::<i32>(vehs, 8);
    while i != 0 {
        i -= 1;
        if let Some(v) = pool_slot(vehs, i, 0xA18)
            && !v.is_null()
        {
            wr::<u8>(v, 0x42B, rd::<u8>(v, 0x42B) & 0xFB);
        }
    }
}

/// `CReplay::Init()` (0x45E220): zeroes the replay state in the original's store order (the first store,
/// `BufferStatus[0]` as a dword, sits in a SecuROM fragment), points `Record.m_pBase` (0x97FB74) at
/// `Buffers[0]` (0x97FB88) with `BufferStatus[0] = 2` (recording), clears `Buffers[0][0]`, sets
/// `bReplayEnabled` (0x8A6160) and tail-calls `MarkEverythingAsNew`.
#[hook(addr = 0x45E220, conv = cdecl, name = "CReplay::Init", expect = "33 C0 33 C9 90",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn replay_init() {
    g::<u32>(0x97FB7C).write(0); // BufferStatus[0..4] (dword)
    g::<u32>(0x97FB70).write(0); // Record.m_nOffset
    g::<u32>(0x97FB64).write(0); // Playback.m_nOffset
    g::<u32>(0x97FB68).write(0); // Playback.m_pBase
    g::<u8>(0x97FB6C).write(0); // Playback.m_bSlot
    g::<u8>(0xA43088).write(0); // Mode
    g::<u32>(0x97FB0C).write(0); // pReferences
    g::<u32>(0x97FB08).write(0); // pPickups
    g::<u32>(0x97FB04).write(0); // pEmptyReferences
    g::<u32>(0x97FB00).write(0); // pWorld1
    g::<u32>(0x97FAF8).write(0); // pStoredCam
    g::<u32>(0x97FAF4).write(0); // pRadarBlips
    g::<u32>(0x97FB80).write(0); // BufferStatus[4..8] (dword)
    g::<u8>(0x97FB78).write(0); // Record.m_bSlot
    g::<u32>(0x97FB74).write(0x97FB88); // Record.m_pBase = Buffers[0]
    g::<u8>(0x97FB7C).write(2); // BufferStatus[0] = REPLAYBUFFER_RECORD
    g::<u8>(0x97FB88).write(0); // Buffers[0][0] = REPLAYPACKET_END
    g::<u8>(0x97FAE1).write(0); // bPlayingBackFromFile
    g::<u8>(0x97FACC).write(0); // bDoLoadSceneWhenDone
    g::<u32>(0x97FAD0).write(0); // FramesActiveLookAroundCam
    g::<u8>(0x8A6160).write(1); // bReplayEnabled
    replay_mark_everything_as_new();
}

/// `CReplay::EmptyReplayBuffer()` (0x45EC90): nothing while `Mode == MODE_PLAYBACK` (1); else
/// `BufferStatus[0..4] = 0` (dword), `Record.m_nOffset = 0`, `BufferStatus[4..8] = 0`,
/// `Record.m_bSlot = 0`, `Record.m_pBase = Buffers[0]`, `BufferStatus[0] = 2`, `Buffers[0][0] = 0` and a
/// tail call of `MarkEverythingAsNew`.
#[hook(addr = 0x45EC90, conv = cdecl, name = "CReplay::EmptyReplayBuffer", expect = "80 3D 88 30 A4 00 01",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn empty_replay_buffer() {
    if g::<u8>(0xA43088).read() == 1 {
        return;
    }
    g::<u32>(0x97FB7C).write(0);
    g::<u32>(0x97FB70).write(0);
    g::<u32>(0x97FB80).write(0);
    g::<u8>(0x97FB78).write(0);
    g::<u32>(0x97FB74).write(0x97FB88);
    g::<u8>(0x97FB7C).write(2);
    g::<u8>(0x97FB88).write(0);
    replay_mark_everything_as_new();
}

/// `CShopping::Init()` (0x49C290): `rep stosd` of 0x230 dwords at 0xA97D90 (`ms_prices`) and 0x8C dwords
/// at 0xA972A0 (`ms_keys`/... the item tables), zero dwords 0xA9A7CC, 0xA9A7D0, 0xA9A310 (`ms_numPrices`,
/// `ms_numPriceModifiers`, `ms_numItemsInShop`) and byte 0xA97298, then a tail call of `LoadStats`.
#[hook(addr = 0x49C290, conv = cdecl, name = "CShopping::Init", expect = "57 33 C0 B9 30 02 00 00",
       route = "split", hazards = "none", recs = "grove-walk",
       note = "tail-calls CShopping::LoadStats (file reads: shopping.dat)")]
pub fn shopping_init() {
    for i in 0..0x230 {
        g::<u32>(0xA97D90 + 4 * i).write(0);
    }
    for i in 0..0x8C {
        g::<u32>(0xA972A0 + 4 * i).write(0);
    }
    g::<u32>(0xA9A7CC).write(0);
    g::<u32>(0xA9A7D0).write(0);
    g::<u32>(0xA9A310).write(0);
    g::<u8>(0xA97298).write(0);
    shopping_load_stats();
}

/// `BreakManager_c::ResetAll()` (0x59E720, thiscall): for each of the 64 `BreakObject_c` (0x20 bytes,
/// from `this`), `Exit()` when its byte +1 (in use) is set.
#[hook(addr = 0x59E720, conv = thiscall, name = "BreakManager_c::ResetAll", expect = "56 57 8B F1 BF 40 00 00 00",
       this = "BreakManager_c", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn break_manager_reset_all(this: GPtr<u8>) {
    let mut o = this;
    for _ in 0..64 {
        if rd::<u8>(o, 1) != 0 {
            break_object_exit(o);
        }
        o = o.byte_add(0x20);
    }
}

/// `FurnitureManager_c::Exit()` (0x5911A0, thiscall): `RemoveAll` on the list at +0x206C, then for each
/// of the 9 groups (0xC bytes from `this`): `RemoveAll` on the list at +0xC of every subgroup in the
/// group's list (head at +0, next at +4), then on the group's list itself.
#[hook(addr = 0x5911A0, conv = thiscall, name = "FurnitureManager_c::Exit", expect = "53 56 57 8B F9",
       this = "FurnitureManager_c", unverified = "mvpa-world-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn furniture_manager_exit_port(this: GPtr<u8>) {
    list_remove_all(this.byte_add(0x206C));
    let mut grp = this;
    for _ in 0..9 {
        let mut s = rd::<GPtr<u8>>(grp, 0);
        while !s.is_null() {
            list_remove_all(s.byte_add(0xC));
            s = rd::<GPtr<u8>>(s, 4);
        }
        list_remove_all(grp);
        grp = grp.byte_add(0xC);
    }
}

/// `InteriorManager_c::Exit()` (0x598010, thiscall): `InteriorGroup_c::Exit` on every group of the list
/// at +0x428C (head, next at +4), `RemoveAll` on the lists at +0x428C, +0x3CA0 and +0x4298, then a tail
/// call of `g_furnitureMan.Exit()` (0xBAB380).
#[hook(addr = 0x598010, conv = thiscall, name = "InteriorManager_c::Exit", expect = "53 56 57 8B F9",
       this = "InteriorManager_c", unverified = "mvpa-world-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn interior_manager_exit(this: GPtr<u8>) {
    let mut s = rd::<GPtr<u8>>(this, 0x428C);
    while !s.is_null() {
        interior_group_exit(s);
        s = rd::<GPtr<u8>>(s, 4);
    }
    list_remove_all(this.byte_add(0x428C));
    list_remove_all(this.byte_add(0x3CA0));
    list_remove_all(this.byte_add(0x4298));
    furniture_manager_exit(g::<u8>(0xBAB380));
}

/// `CEntryExitManager::GetEntryExitIndex(const char* name, uint16 enabledFlags, uint16 disabledFlags)`
/// (0x43EFD0, cdecl): walks the entry-exit pool from the last slot down; returns the index of the first
/// used, non-null `CEntryExit` whose flags (+0x30, u16) contain every bit of the low byte of
/// `enabledFlags` (the original zero-extends only that byte: `movzx cx, byte`), whose flags' low byte
/// shares no bit with the low byte of `disabledFlags`, and whose name matches `name` (`_strnicmp`, 8).
/// -1 when none.
#[hook(addr = 0x43EFD0, conv = cdecl, name = "CEntryExitManager::GetEntryExitIndex", expect = "53 55 56 57 90",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn get_entry_exit_index(name: GPtr<u8>, enabled: u32, disabled: u32) -> i32 {
    let pool = g::<GPtr<u8>>(ENTRY_EXIT_POOL).read();
    let mut i = rd::<i32>(pool, 8);
    let en = enabled as u8 as u16;
    while i != 0 {
        i -= 1;
        let pool = g::<GPtr<u8>>(ENTRY_EXIT_POOL).read();
        let Some(e) = pool_slot(pool, i, 0x3C) else { continue };
        if e.is_null() {
            continue;
        }
        let flags = rd::<u16>(e, 0x30);
        if flags & en != en || (disabled as u8) & (flags as u8) != 0 {
            continue;
        }
        if strnicmp(e, name, 8) == 0 {
            return i;
        }
    }
    -1
}

/// `CGameLogic::ClearSkip(bool afterMission)` (0x441560, `.HOODLUM` stub; body 0x1561270): nothing when
/// `afterMission` and `SkipState == 4`; when `SkipState == 2` the camera fade colour becomes black and a
/// 0.5 s fade in starts (`Fade(0.5f, FADE_IN = 1)`); then `SkipState = 0` and pad 0's byte +0x10F loses bit
/// 0 (`bDisablePlayerEnterCar`... the skip's control lock).
#[hook(addr = 0x441560, conv = cdecl, name = "CGameLogic::ClearSkip", expect = "E9 0B FD 11 01",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn clear_skip(after_mission: u32) {
    let state = g::<u32>(SKIP_STATE).read();
    if after_mission as u8 != 0 && state == 4 {
        return;
    }
    if state == 2 {
        camera_set_fade_colour(g::<u8>(THE_CAMERA), 0, 0, 0);
        camera_fade(g::<u8>(THE_CAMERA), 0x3F00_0000, 1);
    }
    g::<u32>(SKIP_STATE).write(0);
    let pad = get_pad(0);
    wr::<u8>(pad, 0x10F, rd::<u8>(pad, 0x10F) & 0xFE);
}

/// `CWorld::ExtinguishAllCarFiresInArea(CVector point, float radius)` (0x566950, cdecl): for every used,
/// non-null vehicle (pool order 0..size; size and pool pointer read once) whose position (matrix +0x30,
/// else placement +4) satisfies `radius * radius > (z-pz)^2 + (y-py)^2 + (x-px)^2` (x87, that operand
/// order; NaN fails), `ExtinguishCarFire()`.
#[hook(addr = 0x566950, conv = cdecl, name = "CWorld::ExtinguishAllCarFiresInArea", expect = "83 EC 10 E9 95 B6 E9 FF",
       route = "L2", hazards = "none", recs = "grove-walk")]
pub fn extinguish_all_car_fires_in_area(px: f32, py: f32, pz: f32, radius: f32) {
    let pool = g::<GPtr<u8>>(VEHICLE_POOL).read();
    let size = rd::<i32>(pool, 8);
    let mut i = 0;
    while i < size {
        if let Some(v) = pool_slot(pool, i, 0xA18)
            && !v.is_null()
        {
            let m = rd::<GPtr<u8>>(v, 0x14);
            let p = if m.is_null() { v.byte_add(4) } else { m.byte_add(0x30) };
            let dz = x(pz).fsub_m(rd::<f32>(p, 8));
            let dy = x(py).fsub_m(rd::<f32>(p, 4));
            let dx = x(px).fsub_m(rd::<f32>(p, 0));
            let sum = dz.fmul(dz).fadd(dy.fmul(dy)).fadd(dx.fmul(dx));
            if !x(radius).fmul_m(radius).fcom(sum).test_any(0x41) {
                vehicle_extinguish_car_fire(v);
            }
        }
        i += 1;
    }
}

/// `CDummyObject::CreateObject()` (0x59EAC0, thiscall): sets bit 13 of the dummy's flags (+0x1C) around
/// `CObject::Create(this)`; with an object: `g_scriptsForBrains.CheckIfNewEntityNeedsScript(obj, 1, 0)`,
/// the dummy loses flag bits 0 and 7 (`& 0xFFFFFF7E`), its interior/area word +0x30 moves to the object
/// and becomes 0. Returns the object (or null).
#[hook(addr = 0x59EAC0, conv = thiscall, name = "CDummyObject::CreateObject", expect = "56 8B F1 8B 46 1C",
       this = "CDummyObject", unverified = "mvpa-world-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn dummy_create_object(this: GPtr<u8>) -> GPtr<u8> {
    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 0x2000);
    let obj = object_create_from_dummy(this);
    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) & 0xFFFF_DFFF);
    if obj.is_null() {
        return obj;
    }
    brains_check_if_new_entity_needs_script(g::<u8>(0xA90CF0), obj, 1, 0);
    wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) & 0xFFFF_FF7E);
    wr::<u32>(obj, 0x30, rd::<u32>(this, 0x30));
    wr::<u32>(this, 0x30, 0);
    obj
}

/// `CObject::Render()` (0x59F180, thiscall): nothing when flag bit 25 of +0x140 is set; when the remap
/// model (+0x14A, i16) is not -1, the object type (+0x13C) is 3 and bit 7 of +0x140 is set,
/// `CVehicleModelInfo::ms_pRemapTexture` (0xB4E47C) = +0x16C and the remap model's
/// `SetVehicleColour(+0x14C, +0x14D, +0x14E, +0x14F)`; then a tail call of `CEntity::Render`.
#[hook(addr = 0x59F180, conv = thiscall, name = "CObject::Render", expect = "56 8B F1 8B 8E 40 01 00 00",
       this = "CObject", unverified = "mvpa-world-1", route = "L2", hazards = "none", recs = "grove-walk")]
pub fn object_render(this: GPtr<u8>) {
    let flags = rd::<u32>(this, 0x140);
    if flags & 0x0200_0000 != 0 {
        return;
    }
    let model = rd::<i16>(this, 0x14A);
    if model != -1 && rd::<u8>(this, 0x13C) == 3 && (flags as u8 as i8) < 0 {
        g::<u32>(0xB4E47C).write(rd::<u32>(this, 0x16C));
        let mi = model_info(i32::from(model));
        vmi_set_vehicle_colour(
            mi,
            u32::from(rd::<u8>(this, 0x14C)),
            u32::from(rd::<u8>(this, 0x14D)),
            u32::from(rd::<u8>(this, 0x14E)),
            u32::from(rd::<u8>(this, 0x14F)),
        );
    }
    entity_render(this);
}

/// `CModelInfo::ms_modelInfoPtrs[model]` (0xA9B0C8; the index as the original scales it).
fn model_info(model: i32) -> GPtr<u8> {
    GPtr::<GPtr<u8>>::from_addr(0xA9B0C8usize.wrapping_add((model as usize).wrapping_mul(4))).read()
}

/// The tag-atomic user-value setter (0x49CD50, cdecl; called by `CEntity::CreateRwObject` for tags):
/// nothing without an RW object (+0x18). Else the entity is searched in `CTagManager::ms_tagDesc`
/// (0xA9A8C0, 8-byte entries {entity, alpha}; indices `count..1` of the 1-based view at 0xA9A8B8,
/// count at 0xA9AD70); `CVisibilityPlugins::SetUserValue(atomic, alpha)` with the pushed EAX as the
/// original builds it: `movzx ax, byte` keeps the upper half of the entry address. Not found: the
/// original reads the byte at address 4 (`xor eax, eax; movzx ax, [eax+4]`, an access violation),
/// kept.
#[hook(addr = 0x49CD50, conv = cdecl, name = "FUN_0049cd50", expect = "8B 4C 24 04 8B 51 18",
       route = "L2", hazards = "none", recs = "grove-walk",
       note = "no catalogue name; the not-found path faults like the original")]
pub fn tag_manager_setup_atomic(e: GPtr<u8>) {
    let atomic = rd::<GPtr<u8>>(e, 0x18);
    if atomic.is_null() {
        return;
    }
    let mut n = g::<u32>(0xA9AD70).read();
    while n != 0 {
        let ent = g::<GPtr<u8>>(0xA9A8B8usize.wrapping_add(8 * n as usize)).read();
        n -= 1;
        if ent == e {
            let entry = 0xA9A8C0u32.wrapping_add(8 * n);
            let v = (entry & 0xFFFF_0000) | u32::from(g::<u8>(entry as usize + 4).read());
            vis_set_user_value(atomic, v);
            return;
        }
    }
    let v = u32::from(g::<u8>(4).read());
    vis_set_user_value(atomic, v);
}

/// `ceilf`-style helper (0x538AD0, cdecl; called by `CObject::SpecialEntityCalcCollisionSteps`): the
/// float argument widened to a double, CRT `ceil` (0x823820), the double result left in ST0 unrounded.
#[hook(addr = 0x538AD0, conv = cdecl, name = "FUN_00538ad0", expect = "D9 44 24 04 83 EC 08",
       route = "L2", hazards = "none", recs = "grove-walk",
       note = "no catalogue name; ceil of an f32 is exact in a double; pure (L1 spec not written)")]
pub fn ceil_f(v: f32) -> X87 {
    X87::from_f64_exact(crt_ceil(f64::from(v)))
}

/// `WaterCreatureManager_c::Exit()` (0x6E3FD0, thiscall): for every creature of the list at +0x1A0C
/// (head; next read before it is moved): out of the global used list (0xC1F93C) into the free list
/// (0xC1F930), `CWorld::Remove` of its entity (+8, even when null), the entity's deleting destructor
/// (flag 1) when non-null, entity = null and `ms_numCreatures` (0xBB4A70, i16) - 1; then a tail call of
/// `RemoveAll` on the list at +0x1A00.
#[hook(addr = 0x6E3FD0, conv = thiscall, name = "WaterCreatureManager_c::Exit", expect = "55 8B E9 56 8B B5 0C 1A 00 00",
       unverified = "mvpa-world-1", route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn water_creature_manager_exit(this: GPtr<u8>) {
    let mut c = rd::<GPtr<u8>>(this, 0x1A0C);
    while !c.is_null() {
        let next = rd::<GPtr<u8>>(c, 4);
        list_remove_item(g::<u8>(0xC1F93C), c);
        list_add_item(g::<u8>(0xC1F930), c);
        world_remove(rd::<GPtr<u8>>(c, 8));
        let e = rd::<GPtr<u8>>(c, 8);
        if !e.is_null() {
            v_delete(e, 1);
        }
        wr::<u32>(c, 8, 0);
        g::<i16>(0xBB4A70).write(g::<i16>(0xBB4A70).read().wrapping_sub(1));
        c = next;
    }
    list_remove_all(this.byte_add(0x1A00));
}

/// `CEntity::CreateRwObject()` (0x533D30, thiscall virtual). Only for visible entities (bit 7 of +0x1C):
/// the model's `CreateInstance()` (vtable +0x2C) into +0x18, with byte 0xA9B0B0 (render-damaged mode)
/// = 1 around the call when flag bit 9 is set. Without an RW object nothing else. A building (type +0x36
/// & 7 == 1) counts in 0xB71804. The RW frame's modelling matrix (`[rw + 4] + 0x10`) is updated from the
/// matrix (+0x14) or the placement (+4). Atomics (RW type byte 1): tags get their user value
/// (0x49CD50) and `CCustomBuildingDNPipeline::PreRenderUpdate(atomic, true)`. Clumps (type 2) of a model
/// with bit 0 of byte +0x13 (animated): objects (type 4) join the moving list when +0xB4 is null and
/// get `SetIsStatic(false)`; other entities get a 0xC-byte link (`*link = this`; null when the allocation
/// fails, still passed) inserted into the list at 0xB9ACC8. When the LOD (+0x30) has a clump, its first
/// animation's time (+0x20) is copied to ours. Then `AddRef` of the model, +0x28 = `CStreaming::AddEntity`,
/// `CreateEffects`, and flag bit 28 when the first atomic (the atomic itself for atomics) has no
/// CBPC pipeline.
#[hook(addr = 0x533D30, conv = thiscall, name = "CEntity::CreateRwObject", expect = "56 8B F1 8B 46 1C",
       this = "CEntity", unverified = "mvpa-world-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "allocates RW objects (CreateInstance) and a list link: compare structurally")]
pub fn entity_create_rw_object(this: GPtr<u8>) {
    let flags = rd::<u32>(this, 0x1C);
    if (flags as u8 as i8) >= 0 {
        return;
    }
    let mi = model_info(i32::from(rd::<i16>(this, 0x22)));
    if flags & 0x200 != 0 {
        g::<u8>(0xA9B0B0).write(1);
        let rw = v_mi_create_instance(mi);
        wr::<GPtr<u8>>(this, 0x18, rw);
        g::<u8>(0xA9B0B0).write(0);
    } else {
        let rw = v_mi_create_instance(mi);
        wr::<GPtr<u8>>(this, 0x18, rw);
    }
    if rd::<GPtr<u8>>(this, 0x18).is_null() {
        return;
    }
    if rd::<u8>(this, 0x36) & 7 == 1 {
        g::<i32>(0xB71804).write(g::<i32>(0xB71804).read().wrapping_add(1));
    }
    let m = rd::<GPtr<u8>>(this, 0x14);
    if m.is_null() {
        let frame_m = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x18), 4).byte_add(0x10);
        simple_transform_update_rw_matrix(this.byte_add(4), frame_m);
    } else {
        let frame_m = rd::<GPtr<u8>>(rd::<GPtr<u8>>(this, 0x18), 4).byte_add(0x10);
        matrix_update_rw_matrix(m, frame_m);
    }
    let rw_type = rd::<u8>(rd::<GPtr<u8>>(this, 0x18), 0);
    if rw_type == 1 {
        if tag_manager_is_tag(this) != 0 {
            tag_set_user_value(this);
        }
        let rw = rd::<GPtr<u8>>(this, 0x18);
        if !rw.is_null() {
            dn_pre_render_update(rw, 1);
        }
    } else if rw_type == 2 && rd::<u8>(mi, 0x13) & 1 != 0 {
        if rd::<u8>(this, 0x36) & 7 == 4 {
            if rd::<u32>(this, 0xB4) == 0 {
                physical_add_to_moving_list(this);
            }
            v_set_is_static(this, 0);
        } else {
            let link = link_new(0xC);
            if !link.is_null() {
                wr::<GPtr<u8>>(link, 0, this);
            }
            link_insert(link, g::<u8>(0xB9ACC8));
        }
        let lod = rd::<GPtr<u8>>(this, 0x30);
        if !lod.is_null() {
            let lrw = rd::<GPtr<u8>>(lod, 0x18);
            if !lrw.is_null() && rd::<u8>(lrw, 0) == 2 {
                let la = clump_get_first_association(lrw);
                if !la.is_null() {
                    let a = clump_get_first_association(rd::<GPtr<u8>>(this, 0x18));
                    if !a.is_null() {
                        assoc_set_current_time(a, rd::<f32>(la, 0x20));
                    }
                }
            }
        }
    }
    mi_add_ref(mi);
    let r = streaming_add_entity(this);
    wr::<GPtr<u8>>(this, 0x28, r);
    entity_create_effects(this);
    let mut rw = rd::<GPtr<u8>>(this, 0x18);
    if rd::<u8>(rw, 0) != 1 {
        rw = get_first_atomic(rw);
    }
    if is_cbpc_pipeline_attached(rw) == 0 {
        wr::<u32>(this, 0x1C, rd::<u32>(this, 0x1C) | 0x1000_0000);
    }
}

/// `CEntryExitManager::PostEntryExitsCreation()` (0x43F0A0, `.HOODLUM` stub; body 0x15631C0): walks the
/// entry-exit pool from the last slot down; every used, non-null entry-exit with flag bit 2 (+0x30) and
/// no link (+0x38) is linked to the entry-exit `GetEntryExitIndex(its name, 0, 4)` names (when not -1;
/// a free slot gives a null link, then dereferenced like the original); the target gets the reverse link
/// when it has none, time on (+0x34) = 0 and time off (+0x35) = 24.
#[hook(addr = 0x43F0A0, conv = cdecl, name = "CEntryExitManager::PostEntryExitsCreation", expect = "E9 1B 41 12 01",
       route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn post_entry_exits_creation() {
    let pool = g::<GPtr<u8>>(ENTRY_EXIT_POOL).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        i -= 1;
        let pool = g::<GPtr<u8>>(ENTRY_EXIT_POOL).read();
        let Some(e) = pool_slot(pool, i, 0x3C) else { continue };
        if e.is_null() || rd::<u8>(e, 0x30) & 4 == 0 || !rd::<GPtr<u8>>(e, 0x38).is_null() {
            continue;
        }
        let idx = get_entry_exit_index_call(e, 0, 4);
        if idx == -1 {
            continue;
        }
        let pool = g::<GPtr<u8>>(ENTRY_EXIT_POOL).read();
        let t = pool_slot(pool, idx, 0x3C).unwrap_or(GPtr::NULL);
        wr::<GPtr<u8>>(e, 0x38, t);
        if rd::<GPtr<u8>>(t, 0x38).is_null() {
            wr::<GPtr<u8>>(t, 0x38, e);
        }
        wr::<u8>(t, 0x34, 0);
        wr::<u8>(t, 0x35, 0x18);
    }
}

/// `CPickups::ReInit()` (0x456E60): `CollectedPickUpIndex` (0x978678, u16) = 0; every one of the 620
/// pickups (0x9788C4, 0x20 each) with a type (+0x18) has its radar blip cleared (`ClearBlipForEntity(7,
/// (ref index (+0x16) << 16) | slot)`), its object (+0) removed from the world and deleted, object = null,
/// flag bit 0 of +0x19 set and type = 0. Then the index is zeroed again, every pickup gets type 0, ref
/// index 1 and object null, the 0x14 dwords at 0x978628 (`aPickUpsCollected`) are zeroed,
/// `CollectedPickUpIndex`... word 0x978624 = 0 and byte 0x8A5F48 (`DisplayHelpMessage`) = 10.
#[hook(addr = 0x456E60, conv = cdecl, name = "CPickups::ReInit", expect = "53 56 33 DB 57",
       unverified = "mvpa-world-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "deletes pickup objects (pool frees)")]
pub fn pickups_re_init() {
    g::<u16>(0x978678).write(0);
    for slot in 0..620usize {
        let p = g::<u8>(0x9788C4 + slot * 0x20);
        if rd::<u8>(p, 0x18) == 0 {
            continue;
        }
        let handle = (u32::from(rd::<u16>(p, 0x16)) << 16) | slot as u32;
        pickups_clear_blip_for_entity(7, handle as i32);
        let o = rd::<GPtr<u8>>(p, 0);
        if !o.is_null() {
            world_remove(o);
            let o = rd::<GPtr<u8>>(p, 0);
            if !o.is_null() {
                v_delete(o, 1);
            }
            wr::<u32>(p, 0, 0);
        }
        let f = rd::<u8>(p, 0x19) | 1;
        wr::<u8>(p, 0x18, 0);
        wr::<u8>(p, 0x19, f);
    }
    g::<u16>(0x978678).write(0);
    for slot in 0..620usize {
        let p = g::<u8>(0x9788C4 + slot * 0x20);
        wr::<u8>(p, 0x18, 0);
        wr::<u16>(p, 0x16, 1);
        wr::<u32>(p, 0, 0);
    }
    for i in 0..0x14usize {
        g::<u32>(0x978628 + 4 * i).write(0);
    }
    g::<u16>(0x978624).write(0);
    g::<u8>(0x8A5F48).write(10);
}

/// `CGridRef::Init()` (0x71D4E0): reads `DATA\GRIDREF.DAT` (dir "" first). For every line not starting
/// with `#` or NUL: `sscanf(line, "%c%d %s", &letter, &number, name)`, `_strupr(name)` and the name is
/// copied (byte loop) to `GridRefList[(letter * 10 + number - 0x28B)]` (0xC72FB0, 32-byte entries).
/// Letter and number start at 0 and keep their previous values when a line does not set them; the name
/// buffer starts uninitialised in the original (here zeroed).
#[hook(addr = 0x71D4E0, conv = cdecl, name = "CGridRef::Init", expect = "83 EC 28 56 68 54 8B 85 00",
       route = "split", hazards = "uninit-locals", recs = "grove-walk",
       note = "file reads (GRIDREF.DAT)")]
pub fn grid_ref_init() {
    let mut letter = [0u8; 4];
    let mut number = [0i32; 1];
    let mut name = [0u8; 32];
    file_set_dir(g::<u8>(0x858B54));
    let f = file_open(g::<u8>(0x87295C), g::<u8>(0x859C6C));
    loop {
        let line = load_line(f);
        if line.is_null() {
            break;
        }
        let c = line.read();
        if c == b'#' || c == 0 {
            continue;
        }
        sscanf3(
            line,
            g::<u8>(0x872954),
            GPtr::from_raw(letter.as_mut_ptr()),
            GPtr::<i32>::from_raw(number.as_mut_ptr()).cast(),
            GPtr::from_raw(name.as_mut_ptr()),
        );
        let mut s = crt_strupr(GPtr::from_raw(name.as_mut_ptr()));
        let idx = i32::from(letter[0]).wrapping_mul(10).wrapping_add(number[0]).wrapping_sub(0x28B);
        let mut d = g::<u8>(0xC72FB0usize.wrapping_add((idx << 5) as u32 as usize));
        loop {
            let b = s.read();
            d.write(b);
            s = s.byte_add(1);
            d = d.byte_add(1);
            if b == 0 {
                break;
            }
        }
    }
    file_close(f);
}

/// `CStats::Init()` (0x55C0C0): zeroes `StatTypesFloat` (0x52 dwords at 0xB79380) and `StatTypesInt`
/// (0xDF dwords at 0xB79000), `bShowUpdateStats` (0xB794D4) and `CTimer::m_snTimeInMilliseconds`
/// (0xB7CB84; the original really clears it here), sets float stat 0xB793E0 to 570.0 and 0xB793D8 to 100.0,
/// each followed by `CheckForStatsMessage`; zeroes 0x20 dwords at 0xB78E90 and 0x64 at 0xB78CC8, the
/// dwords 0xB78A00/0xB78A04; `PopulateFavoriteRadioStationList`, `LoadActionReactionStats`,
/// `LoadStatUpdateConditions`; then zeroes the nine dwords 0xB794D8, DC, E4, EC, F0, F4, F8, FC, 0xB79500.
#[hook(addr = 0x55C0C0, conv = cdecl, name = "CStats::Init", expect = "53 57 33 C0 B9 52 00 00 00",
       unverified = "mvpa-world-1", route = "split", hazards = "none", recs = "grove-walk",
       note = "one-time init: reads AR_STATS.DAT and STATDISP.DAT")]
pub fn stats_init() {
    for i in 0..0x52usize {
        g::<u32>(0xB79380 + 4 * i).write(0);
    }
    g::<u8>(0xB794D4).write(0);
    g::<u32>(0xB7CB84).write(0);
    for i in 0..0xDFusize {
        g::<u32>(0xB79000 + 4 * i).write(0);
    }
    g::<u32>(0xB793E0).write(0x440E_4000);
    stats_check_for_stats_message();
    g::<u32>(0xB793D8).write(0x42C8_0000);
    stats_check_for_stats_message();
    for i in 0..0x20usize {
        g::<u32>(0xB78E90 + 4 * i).write(0);
    }
    for i in 0..0x64usize {
        g::<u32>(0xB78CC8 + 4 * i).write(0);
    }
    g::<u32>(0xB78A00).write(0);
    g::<u32>(0xB78A04).write(0);
    stats_populate_favorite_radio_station_list();
    stats_load_action_reaction_stats();
    stats_load_stat_update_conditions();
    for a in [0xB794D8usize, 0xB794DC, 0xB794E4, 0xB794EC, 0xB794F0, 0xB794F4, 0xB794F8, 0xB794FC, 0xB79500] {
        g::<u32>(a).write(0);
    }
}

/// `CStats::LoadActionReactionStats()` (0x5599B0): reads `DATA\AR_STATS.DAT` (dir ""); every line not
/// starting with `#`/NUL is `sscanf(line, "%d %s %f", &id, name, &value)` and
/// `fActionReactionStats`... the float table 0xB78F10`[id] = value`. The locals sit in one frame as in the
/// original (value at +0, id at +4, name from +8; id/value start at 0 and keep the previous line's values
/// when a line does not set them).
#[hook(addr = 0x5599B0, conv = cdecl, name = "CStats::LoadActionReactionStats", expect = "83 EC 58 56 68 54 8B 85 00",
       route = "split", hazards = "uninit-locals", recs = "grove-walk",
       note = "file reads (AR_STATS.DAT)")]
pub fn load_action_reaction_stats() {
    let mut frame = [0u8; 0x58];
    let fp = GPtr::<u8>::from_raw(frame.as_mut_ptr());
    file_set_dir(g::<u8>(0x858B54));
    let f = file_open(g::<u8>(0x863FA0), g::<u8>(0x859C6C));
    loop {
        let line = load_line(f);
        if line.is_null() {
            break;
        }
        let c = line.read();
        if c == b'#' || c == 0 {
            continue;
        }
        sscanf3(line, g::<u8>(0x863F94), fp.byte_add(4), fp.byte_add(8), fp);
        let id = rd::<i32>(fp, 4);
        g::<u32>(0xB78F10usize.wrapping_add((id as usize).wrapping_mul(4))).write(rd::<u32>(fp, 0));
    }
    file_close(f);
}

/// `CStats::LoadStatUpdateConditions()` (0x559860): reads `DATA\STATDISP.DAT` (dir ""), count
/// `ms_nNumStatUpdateConditions`... 0xB794D0 = 0 first. Every line not starting with `#`/NUL is
/// `sscanf(line, "%d %s %s %f %s", &stat, unused, cmp, &value, message)` into the frame layout of the
/// original (stat +0, value +4, message +0x10 (8 bytes before cmp, so a long message runs into it like in
/// the original), cmp +0x18, unused +0x24); then the next 0x10-byte entry of 0xB78200 gets: u16 stat,
/// byte +2 = 0, byte +3 = 0 for "lessthan" / 1 for "morethan" (unchanged otherwise; 9-byte compares),
/// dword +4 = value, message copied from +8. The count is stored at the end.
#[hook(addr = 0x559860, conv = cdecl, name = "CStats::LoadStatUpdateConditions", expect = "55 8B EC 83 E4 F8",
       route = "split", hazards = "uninit-locals", recs = "grove-walk",
       note = "file reads (STATDISP.DAT); writes the 0xB78200 table")]
pub fn load_stat_update_conditions() {
    let mut frame = [0u8; 0x64];
    let fp = GPtr::<u8>::from_raw(frame.as_mut_ptr());
    file_set_dir(g::<u8>(0x858B54));
    let f = file_open(g::<u8>(0x863F80), g::<u8>(0x859C6C));
    g::<u32>(0xB794D0).write(0);
    let mut count: u32 = 0;
    let mut e = g::<u8>(0xB78200);
    loop {
        let line = load_line(f);
        if line.is_null() {
            break;
        }
        let c = line.read();
        if c == b'#' || c == 0 {
            continue;
        }
        sscanf5(line, g::<u8>(0x863F70), fp, fp.byte_add(0x24), fp.byte_add(0x18), fp.byte_add(4), fp.byte_add(0x10));
        let cmp9 = |lit: usize| (0..9).all(|k| fp.byte_add(0x18 + k).read() == g::<u8>(lit + k as usize).read());
        wr::<u16>(e, 0, rd::<u16>(fp, 0));
        wr::<u8>(e, 2, 0);
        if cmp9(0x863F64) {
            wr::<u8>(e, 3, 0);
        } else if cmp9(0x863F58) {
            wr::<u8>(e, 3, 1);
        }
        wr::<u32>(e, 4, rd::<u32>(fp, 4));
        let mut k = 0;
        loop {
            let b = fp.byte_add(0x10 + k).read();
            e.byte_add(8 + k).write(b);
            k += 1;
            if b == 0 {
                break;
            }
        }
        count = count.wrapping_add(1);
        e = e.byte_add(0x10);
    }
    g::<u32>(0xB794D0).write(count);
    file_close(f);
}

/// `InteriorManager_c::Init()` (0x5C0500, thiscall): bytes +0x4750 = 0, +0x4751 = 1, +0x4752 = 1; the 8
/// interiors (0x794 each from `this`) go back to `g_interiorMan`'s pool (`ReturnInteriorToPool`, on
/// 0xBAF670); the 8 interior groups (0xBC each from +0x3CAC) get byte +0xC = index and join the list at
/// +0x4298; the words 0xBAB37C / 0xBAB378 = 0; the 512 furniture items (0x10 each, 0xBAB3EC..0xBAD3EC)
/// join the list 0xBAD3EC; the 9 furniture groups of `g_furnitureMan` (0xBAB380, 0xC each) `Init()`;
/// `g_furnitureMan.LoadFurniture()`; dwords +0x473C, +0x42A4, +0x43A8 = 0, +0x472C..+0x4738 =
/// 0x01010101, +0x4754 = 0xFFFD40E0. Returns true with EAX = 0x01010101 (`mov al, 1` over that value).
#[hook(addr = 0x5C0500, conv = thiscall, name = "InteriorManager_c::Init", expect = "53 55 56 8B F1",
       this = "InteriorManager_c", route = "split", hazards = "none", recs = "grove-walk",
       note = "one-time init; LoadFurniture reads furnitur.dat")]
pub fn interior_manager_init(this: GPtr<u8>) -> u32 {
    wr::<u8>(this, 0x4750, 0);
    wr::<u8>(this, 0x4751, 1);
    wr::<u8>(this, 0x4752, 1);
    let mut p = this;
    for _ in 0..8 {
        interior_return_to_pool(g::<u8>(0xBAF670), p);
        p = p.byte_add(0x794);
    }
    let mut p = this.byte_add(0x3CAC);
    for i in 0..8u8 {
        wr::<u8>(p, 0xC, i);
        list_add_item(this.byte_add(0x4298), p);
        p = p.byte_add(0xBC);
    }
    g::<u16>(0xBAB37C).write(0);
    g::<u16>(0xBAB378).write(0);
    let mut a = 0xBAB3ECusize;
    while a < 0xBAD3EC {
        list_add_item(g::<u8>(0xBAD3EC), g::<u8>(a));
        a += 0x10;
    }
    let mut gp = 0xBAB380usize;
    for _ in 0..9 {
        furniture_group_init(g::<u8>(gp));
        gp += 0xC;
    }
    furniture_load_furniture(g::<u8>(0xBAB380));
    wr::<u32>(this, 0x473C, 0);
    wr::<u32>(this, 0x42A4, 0);
    wr::<u32>(this, 0x43A8, 0);
    for o in [0x472C, 0x4730, 0x4734, 0x4738] {
        wr::<u32>(this, o, 0x0101_0101);
    }
    wr::<u32>(this, 0x4754, 0xFFFD_40E0);
    0x0101_0101
}

/// `CEntryExitManager::ShutdownForRestart()` (0x440C40, `.HOODLUM` stub; body 0x1567550): walks the pool
/// from the last slot down; every used, non-null entry-exit gets flag bit 14 (+0x31 |= 0x40); those with
/// flag bit 15 (added by script) that are still used are removed from the quad tree (0x96A7D0) and freed
/// in the pool inline (byte map bit 7 set, first-free index lowered; the index from the signed division
/// by 0x3C as the original computes it). Then `EnableBurglaryHouses(false)`; when `ms_bBurglaryHousesEnabled`...
/// the dword 0x96A7D4 is set: `ResetAreaCodeForVisibleObjects()`, 0x96A7CC = 0 and 0x96A7D4 = 0; finally
/// `ms_entryExitStackPosn` (0x96A7C4) = 0 and byte 0x96A7C8 = 0.
#[hook(addr = 0x440C40, conv = cdecl, name = "CEntryExitManager::ShutdownForRestart", expect = "E9 0B 69 12 01",
       route = "L2", hazards = "heap-writes", recs = "grove-walk")]
pub fn entry_exit_shutdown_for_restart() {
    let pool = g::<GPtr<u8>>(ENTRY_EXIT_POOL).read();
    let mut i = rd::<i32>(pool, 8);
    while i != 0 {
        i -= 1;
        let pool = g::<GPtr<u8>>(ENTRY_EXIT_POOL).read();
        let Some(e) = pool_slot(pool, i, 0x3C) else { continue };
        if e.is_null() {
            continue;
        }
        wr::<u8>(e, 0x31, rd::<u8>(e, 0x31) | 0x40);
        if (rd::<u16>(e, 0x30) as i16) >= 0 {
            continue;
        }
        let Some(obj) = pool_slot(pool, i, 0x3C) else { continue };
        quad_tree_delete_item(g::<GPtr<u8>>(0x96A7D0).read(), obj);
        let pool = g::<GPtr<u8>>(ENTRY_EXIT_POOL).read();
        let idx = (obj.addr() as u32).wrapping_sub(rd::<u32>(pool, 0)) as i32 / 0x3C;
        let m = rd::<GPtr<u8>>(pool, 4).byte_add(idx as isize);
        m.write(m.read() | 0x80);
        if idx < rd::<i32>(pool, 0xC) {
            wr::<i32>(pool, 0xC, idx);
        }
    }
    enable_burglary_houses(0);
    if g::<u32>(0x96A7D4).read() != 0 {
        reset_area_code_for_visible_objects();
        g::<u32>(0x96A7CC).write(0);
        g::<u32>(0x96A7D4).write(0);
    }
    g::<u32>(0x96A7C4).write(0);
    g::<u8>(0x96A7C8).write(0);
}

/// `CWorld::ClearForRestart()` (0x564360): `CCutsceneMgr::DeleteCutsceneData()` when the cutscene state
/// (0xB5F84C) is 2; `CProjectileInfo::RemoveAllProjectiles`, `CObject::DeleteAllTempObjects`,
/// `CObject::DeleteAllMissionObjects`, `CPopulation::ConvertAllObjectsToDummyObjects`; then for each of
/// the 256 repeat sectors (0xB992B8, 0xC each) the list at +4, then the list at +0: every entity is
/// `Remove()`d (vtable +0xC), physicals (type 2..4) leave the moving list, and the entity (re-read from
/// the node) is deleted; `CPickups::ReInit()` and a tail call of `CPools::CheckPoolsEmpty()`.
#[hook(addr = 0x564360, conv = cdecl, name = "CWorld::ClearForRestart", expect = "83 3D 4C F8 B5 00 02",
       unverified = "mvpa-world-1", route = "L2", hazards = "heap-writes", recs = "grove-walk",
       note = "deletes every repeat-sector entity (pool frees); restart path")]
pub fn world_clear_for_restart() {
    if g::<u32>(0xB5F84C).read() == 2 {
        cutscene_delete_cutscene_data();
    }
    remove_all_projectiles();
    object_delete_all_temp_objects();
    object_delete_all_mission_objects();
    population_convert_all_objects_to_dummy_objects();
    let mut sec = 0xB992B8usize;
    while sec < 0xB99EB8 {
        for off in [4isize, 0] {
            let mut node = rd::<GPtr<u8>>(g::<u8>(sec), off);
            while !node.is_null() {
                let e = rd::<GPtr<u8>>(node, 0);
                let next = rd::<GPtr<u8>>(node, 4);
                v_entity_remove(e);
                let t = rd::<u8>(e, 0x36) & 7;
                if t > 1 && t < 5 {
                    physical_remove_from_moving_list(e);
                }
                let e2 = rd::<GPtr<u8>>(node, 0);
                if !e2.is_null() {
                    v_delete(e2, 1);
                }
                node = next;
            }
        }
        sec += 0xC;
    }
    pickups_re_init_call();
    pools_check_pools_empty();
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_MARK_EVERYTHING_AS_NEW,
    HOOK_REPLAY_INIT,
    HOOK_EMPTY_REPLAY_BUFFER,
    HOOK_SHOPPING_INIT,
    HOOK_BREAK_MANAGER_RESET_ALL,
    HOOK_FURNITURE_MANAGER_EXIT_PORT,
    HOOK_INTERIOR_MANAGER_EXIT,
    HOOK_GET_ENTRY_EXIT_INDEX,
    HOOK_CLEAR_SKIP,
    HOOK_EXTINGUISH_ALL_CAR_FIRES_IN_AREA,
    HOOK_DUMMY_CREATE_OBJECT,
    HOOK_OBJECT_RENDER,
    HOOK_TAG_MANAGER_SETUP_ATOMIC,
    HOOK_CEIL_F,
    HOOK_WATER_CREATURE_MANAGER_EXIT,
    HOOK_ENTITY_CREATE_RW_OBJECT,
    HOOK_POST_ENTRY_EXITS_CREATION,
    HOOK_PICKUPS_RE_INIT,
    HOOK_GRID_REF_INIT,
    HOOK_STATS_INIT,
    HOOK_LOAD_ACTION_REACTION_STATS,
    HOOK_LOAD_STAT_UPDATE_CONDITIONS,
    HOOK_INTERIOR_MANAGER_INIT,
    HOOK_ENTRY_EXIT_SHUTDOWN_FOR_RESTART,
    HOOK_WORLD_CLEAR_FOR_RESTART,
];
