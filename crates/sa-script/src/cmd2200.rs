//! `CRunningScript::ProcessCommands2200To2299` (0x474900): complete port (PORT-SCRIPT-2 partial, completed in
//! PORT-SCRIPT-3). Every opcode of the range has a jump-table case and is in [`PORTED`]; the default (-1,
//! `or al,0xFF` at 0x476120) is unreachable for 2200..2299 and kept for other command values.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands2200To2299.md`.
//!
//! Divergence note (08B3, 08D4, 08DB): the 8-byte labels are read into a zeroed 16-byte buffer and copied with the
//! original's byte loop up to the terminator; the original's label buffer is not cleared, so an
//! 8-character label (no terminator) makes the loop copy stale stack bytes after it. Differs only for
//! 8-character labels.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x0898, 0x0899, 0x089A, 0x089B, 0x089C, 0x089D, 0x089E, 0x089F, 0x08A0, 0x08A1, 0x08A2, 0x08A3, 0x08A4,
    0x08A5, 0x08A6, 0x08A7, 0x08A8, 0x08A9, 0x08AA, 0x08AB, 0x08AC, 0x08AD, 0x08AE, 0x08AF, 0x08B0, 0x08B1,
    0x08B2, 0x08B3, 0x08B4, 0x08B5, 0x08B6, 0x08B7, 0x08B8, 0x08B9, 0x08BA, 0x08BB, 0x08BC, 0x08BD, 0x08BE,
    0x08BF, 0x08C0, 0x08C1, 0x08C2, 0x08C3, 0x08C4, 0x08C5, 0x08C6, 0x08C7, 0x08C8, 0x08C9, 0x08CA, 0x08CB,
    0x08CC, 0x08CD, 0x08CE, 0x08CF, 0x08D0, 0x08D1, 0x08D2, 0x08D3, 0x08D4, 0x08D5, 0x08D6, 0x08D7, 0x08D8,
    0x08D9, 0x08DA, 0x08DB, 0x08DC, 0x08DD, 0x08DE, 0x08DF, 0x08E0, 0x08E1, 0x08E2, 0x08E3, 0x08E4, 0x08E5,
    0x08E6, 0x08E7, 0x08E8, 0x08E9, 0x08EA, 0x08EB, 0x08EC, 0x08ED, 0x08EE, 0x08EF, 0x08F0, 0x08F1, 0x08F2,
    0x08F3, 0x08F4, 0x08F5, 0x08F6, 0x08F7, 0x08F8, 0x08F9, 0x08FA, 0x08FB,
];

/// `CTheScripts::StreamedScripts` (`CStreamedScripts`) 0xA47B60.
const STREAMED_SCRIPTS: usize = 0xA47B60;
/// The load-state byte the case reads for streamed script `i`: `0x964E08 + i*20` (CStreaming::ms_aInfoForModel,
/// 20-byte entries, model `i + 26230`, the first SCM model index 0x6676).
const SCM_LOAD_STATE: usize = 0x964E08;

/// `RsGlobal.maximumWidth` / `maximumHeight` (int32) 0xC17044 / 0xC17048.
const MAXIMUM_WIDTH: usize = 0xC17044;
const MAXIMUM_HEIGHT: usize = 0xC17048;
/// `.rdata` 0x859520: 0.0015625f (1/640)
const K_1_640: u32 = 0x3ACC_CCCD;
/// `.rdata` 0x859524: 0.002232143f (1/448)
const K_1_448: u32 = 0x3B12_4925;
/// `.rdata` 0x859EE0: "DUMMY"
const DUMMY: usize = 0x859EE0;

orig! {
    /// `CMenuSystem::CheckForSelected(uint8)` (0x5807E0), AL (the full ScriptParams dword is pushed)
    fn check_for_selected(id: u32) -> i8 = cdecl 0x5807E0 => unported;
    /// `CMenuSystem::ActivateOneItem(uint8, uint8, bool)` (0x581B30)
    fn activate_one_item(id: u32, row: u32, on: u32) = cdecl 0x581B30 => unported;
    /// `CMenuSystem::SwitchOffMenu(uint8)` (0x580750)
    fn switch_off_menu(id: u32) = cdecl 0x580750 => unported;
    /// `CStreamedScripts::GetProperIndexFromIndexUsedByScript(int16)` (0x470810), AX
    fn get_proper_index_from_index_used_by_script(m: GPtr<u8>, i: u32) -> u16 = thiscall 0x470810 => unported;
    /// `CStreaming::RequestModel(int32, int32 flags)` (0x4087E0)
    fn request_model(id: i32, flags: i32) = cdecl 0x4087E0 => unported;
    /// `CMenuSystem::SetColumnOrientation(uint8, uint8, eFontAlignment)` (0x582080)
    fn set_column_orientation(menu: u32, column: u32, align: u32) = cdecl 0x582080 => unported;
    /// `CMenuSystem::CheckForAccept(uint8)` (0x5807C0), AL (the full ScriptParams dword is pushed)
    fn check_for_accept(id: u32) -> i8 = cdecl 0x5807C0 => unported;
    /// `C3dMarkers::ForceRender(bool)` (0x722870)
    fn markers_force_render(on: u32) = cdecl 0x722870 => unported;
    /// `CPostEffects::ScriptNightVisionSwitch(bool)` (0x701120)
    fn script_night_vision_switch(on: u32) = cdecl 0x701120 => unported;
    /// `CPostEffects::ScriptInfraredVisionSwitch(bool)` (0x701140)
    fn script_infrared_vision_switch(on: u32) = cdecl 0x701140 => unported;
    /// `CTheZones::FindZoneByLabel(const char*, eZoneType)` (0x572C40), AX
    fn find_zone_by_label(label: GPtr<u8>, ty: u32) -> u16 = cdecl 0x572C40 => unported;
    /// ADD_PRICE_MODIFIER's callee (0x49BDD0, cdecl (P0, P1); unnamed in the catalogue, gr
    /// `CShopping::AddPriceModifier`)
    fn add_price_modifier(a: u32, b: u32) = cdecl 0x49BDD0 => unported;
    /// REMOVE_PRICE_MODIFIER's callee (0x49ACD0, cdecl (P0); a thunk, gr `CShopping::RemovePriceModifier`)
    fn remove_price_modifier(a: u32) = cdecl 0x49ACD0 => unported;
    /// `CCutsceneMgr::IsCutsceneSkipButtonBeingPressed()` (0x4D5D10), AL
    fn is_cutscene_skip_button_being_pressed() -> u8 = cdecl 0x4D5D10 => unported;
    /// `CPlane::SetGearUp()` (0x6CAC20) / `SetGearDown()` (0x6CAC70)
    fn plane_set_gear_up(v: GPtr<u8>) = thiscall 0x6CAC20 => unported;
    fn plane_set_gear_down(v: GPtr<u8>) = thiscall 0x6CAC70 => unported;
    /// `CConversations::RemoveConversationForPed(CPed*)` (0x43A960)
    fn remove_conversation_for_ped(p: GPtr<u8>) = cdecl 0x43A960 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `CPlayerPed::MakePlayerGroupDisappear()` (0x60A440) / `MakePlayerGroupReappear()` (0x60A4B0)
    fn make_player_group_disappear(p: GPtr<u8>) = thiscall 0x60A440 => unported;
    fn make_player_group_reappear(p: GPtr<u8>) = thiscall 0x60A4B0 => unported;
    /// CRT `strncmp` (0x8214D0)
    fn strncmp(a: GPtr<u8>, b: GPtr<u8>, n: u32) -> i32 = cdecl 0x8214D0 => unported;
    /// `CRadar::SetCoordBlipAppearance(int32, uint8)` (0x583E50)
    fn set_coord_blip_appearance(h: u32, app: u32) = cdecl 0x583E50 => unported;
    /// `CConversations::IsPlayerInPositionForConversation(CPed*, bool)` (0x43B0B0), AL
    fn is_player_in_position_for_conversation(p: GPtr<u8>, b: u32) -> u8 = cdecl 0x43B0B0 => unported;
    /// `CConversations::EnableConversation(CPed*, bool)` (0x43AA40)
    fn enable_conversation(p: GPtr<u8>, on: u32) = cdecl 0x43AA40 => unported;
    /// `CAutomobile::UpdateMovingCollision(float)` (0x6A1460); the float as a raw dword
    fn update_moving_collision(v: GPtr<u8>, a: u32) = thiscall 0x6A1460 => unported;
    /// `CDamageManager::GetCarNodeIndexFromDoor(eDoors)` (0x6C26F0)
    fn get_car_node_index_from_door(door: u32) -> u32 = cdecl 0x6C26F0 => unported;
    /// `CVehicle` virtual slot 0x98 (`IsDoorMissing(eDoors)`), AL
    fn vehicle_is_door_missing(v: GPtr<u8>, door: u32) -> u8 = thiscall vslot 0x98 => unported;
    /// `CVehicle` virtual slot 0x6C (`OpenDoor(CPed*, int32 node, eDoors, float ratio, bool sound)`); ratio raw
    fn vehicle_open_door(v: GPtr<u8>, ped: GPtr<u8>, node: u32, door: u32, ratio: u32, sound: u32) = thiscall vslot 0x6C => unported;
    /// `CVehicle` virtual slot 0x84 (`IsDoorFullyOpen(..)`), AL; the argument is the node index the case pushes
    fn vehicle_is_door_fully_open(v: GPtr<u8>, node: u32) -> u8 = thiscall vslot 0x84 => unported;
    /// `CRunningScript::SetCharCoordinates(CPed*, float x, y, z, bool warp, bool offset)` (0x464DC0); raw floats
    fn set_char_coordinates(s: Scr, p: GPtr<u8>, x: u32, y: u32, z: u32, warp: u32, offset: u32) = thiscall 0x464DC0 => unported;
    /// `CTheZones::InitZonesPopulationSettings()` (0x5720D0)
    fn init_zones_population_settings() = cdecl 0x5720D0 => unported;
    /// `CVehicle` virtual slot 0xA8 (`BlowUpCarCutSceneNoExtras(bool, bool, bool, bool)`)
    fn vehicle_blow_up_cutscene(v: GPtr<u8>, a: u32, b: u32, c: u32, d: u32) = thiscall vslot 0xA8 => unported;
    /// `CRunningScript::ObjectInAngledAreaCheckCommand(int32 command)` (0x4883F0)
    fn object_in_angled_area_check_command(s: Scr, command: u32) = thiscall 0x4883F0 => unported;
    /// `CTheScripts::ScriptAttachAnimGroupToCharModel(int32 model, char* ifp)` (0x474800)
    fn script_attach_anim_group_to_char_model(model: i32, ifp: GPtr<u8>) -> u8 = cdecl 0x474800 => unported;
    /// `CTheScripts::AddToListOfSpecialAnimGroupsAttachedToCharModels(int32 model, char* ifp)` (0x474750)
    fn add_to_list_of_special_anim_groups(model: i32, ifp: GPtr<u8>) = cdecl 0x474750 => unported;
    /// `CMenuSystem::InsertOneMenuItemWithNumber(uint8, uint8, uint8, char*, int32, int32)` (0x581D70)
    fn insert_one_menu_item_with_number(menu: u32, col: u32, row: u32, key: GPtr<u8>, n1: u32, n2: u32) = cdecl 0x581D70 => unported;
    /// `CCutsceneMgr::AppendToNextCutscene(const char*, const char*)` (0x4D5DB0)
    fn append_to_next_cutscene(a: GPtr<u8>, b: GPtr<u8>) = cdecl 0x4D5DB0 => unported;
    /// `CTheZones::FindSmallestZoneForPosition(const CVector&, bool)` (0x572360)
    fn find_smallest_zone_for_position(pos: GPtr<u32>, b: u32) -> GPtr<u8> = cdecl 0x572360 => unported;
    /// CRT `strncpy` (0x821F40)
    fn strncpy_(dst: GPtr<u8>, src: GPtr<u8>, n: u32) -> GPtr<u8> = cdecl 0x821F40 => unported;
    /// `CPedGroupMembership::CountMembersExcludingLeader()` (0x5F6AA0)
    fn count_members_excluding_leader(m: GPtr<u8>) -> i32 = thiscall 0x5F6AA0 => unported;
    /// `CPedGroupMembership::RemoveNFollowers(int32)` (0x5FB1D0)
    fn remove_n_followers(m: GPtr<u8>, n: i32) = thiscall 0x5FB1D0 => unported;
    /// `CClothes::GetTextureDependency(int32)` (0x5A7EA0)
    fn get_texture_dependency(i: u32) -> i32 = cdecl 0x5A7EA0 => unported;
    /// The ped pool's slot accessor at 0x4082A0 (thiscall on the pool, slot index; a `.HOODLUM` thunk,
    /// `CPool<CPed>::GetAt(int32)` in gr)
    fn ped_pool_get_at(pool: GPtr<u8>, i: i32) -> GPtr<u8> = thiscall 0x4082A0 => unported;
    /// `CVector::Magnitude()` (0x4082C0), ST0 (read as f64: the callee's PC-24 result is exact)
    fn vector_magnitude(v: GPtr<u32>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CRunningScript::IsPedDead(CPed*)` (0x464D70), AL
    fn script_is_ped_dead(s: Scr, p: GPtr<u8>) -> u8 = thiscall 0x464D70 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80)
    fn get_peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPed::SetCharCreatedBy(uint8)` (0x5E47E0)
    fn set_char_created_by(p: GPtr<u8>, by: u32) = thiscall 0x5E47E0 => unported;
    /// `CEntryExitManager::FindNearestEntryExit(const CVector2D&, float range, int32 ignore)` (0x43F4B0)
    fn find_nearest_entry_exit(pos: GPtr<u32>, range: u32, ignore: i32) -> i32 = cdecl 0x43F4B0 => unported;
    /// The entry-exit slot accessor at 0x43EF00 (cdecl (index) -> CEntryExit*; a `.HOODLUM` thunk)
    fn entry_exit_at(i: i32) -> GPtr<u8> = cdecl 0x43EF00 => unported;
    /// `CEntryExitManager::AddEntryExitToStack(CEntryExit*)` (0x43E410)
    fn add_entry_exit_to_stack(e: GPtr<u8>) = cdecl 0x43E410 => unported;
    /// `CTimeCycle::StartExtraColour(int32, bool)` (0x55FEC0) / `StopExtraColour(bool)` (0x55FF20)
    fn start_extra_colour(c: u32, b: u32) = cdecl 0x55FEC0 => unported;
    fn stop_extra_colour(b: u32) = cdecl 0x55FF20 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0), AL
    fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CRadar::SetBlipEntryExit(int32, CEntryExit*)` (0x583F00)
    fn set_blip_entry_exit(h: u32, e: GPtr<u8>) = cdecl 0x583F00 => unported;
    /// `CVector::NormaliseAndMag()` (0x59C970), ST0 (read as f64: the callee's PC-24 result is exact)
    fn normalise_and_mag(v: GPtr<u32>) -> f64 = thiscall 0x59C970 => unported;
    /// `Fx_c::AddSparks(CVector& pos, CVector& dir, float, int32 count, CVector across, uint8, float, float)`
    /// (0x49F040); floats and the by-value vector as raw dwords
    fn fx_add_sparks(
        fx: GPtr<u8>, pos: GPtr<u32>, dir: GPtr<u32>, force: u32, count: u32, ax: u32, ay: u32, az: u32,
        blur: u32, spread: u32, life: u32
    ) = thiscall 0x49F040 => unported;
    /// `CPedIntelligence::GetEffectInUse()` (0x6018D0)
    fn get_effect_in_use(i: GPtr<u8>) -> u32 = thiscall 0x6018D0 => unported;
    /// `CPedIntelligence::SetEffectInUse(C2dEffect*)` (0x6018E0)
    fn set_effect_in_use(i: GPtr<u8>, e: u32) = thiscall 0x6018E0 => unported;
    /// The closest-attractor search at 0x6002F0 (cdecl, 9 dwords: `(CVector* pos, float range, int32 model,
    /// int32 5, char* name, bool 1, C2dEffect* inUse, C2dEffect** outEffect, CEntity** outEntity)`; unnamed in
    /// the catalogue)
    fn find_closest_attractor(
        pos: GPtr<u32>, range: u32, model: i32, five: u32, name: GPtr<u8>, one: u32, in_use: u32,
        out_effect: GPtr<u32>, out_entity: GPtr<u32>
    ) = cdecl 0x6002F0 => unported;
    /// `CRunningScript::RemoveScriptFromList(CRunningScript**)` (0x464BD0) / `AddScriptToList` (0x464C00)
    fn remove_script_from_list(s: Scr, list: GPtr<u8>) = thiscall 0x464BD0 => unported;
    fn add_script_to_list(s: Scr, list: GPtr<u8>) = thiscall 0x464C00 => unported;
    /// `CRunningScript::ShutdownThisScript()` (0x465AA0)
    fn shutdown_this_script(s: Scr) = thiscall 0x465AA0 => unported;
    /// `CEventAttractor::CEventAttractor(C2dEffect*, CEntity*, bool)` (0x4AF350)
    fn event_attractor_ctor(e: GPtr<u8>, effect: u32, entity: u32, b: u32) -> GPtr<u8> = thiscall 0x4AF350 => unported;
    /// The `CEventAttractor` destructor at 0x4AF400
    fn event_attractor_dtor(e: GPtr<u8>) = thiscall 0x4AF400 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    fn event_group_add(g: GPtr<u8>, e: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTaskComplexUseEffect::CTaskComplexUseEffect(C2dEffect*, CEntity*)` (0x6321F0)
    fn task_use_effect_ctor(t: GPtr<u8>, effect: u32, entity: u32) -> GPtr<u8> = thiscall 0x6321F0 => unported;
    /// `CRunningScript::GivePedScriptedTask(int32 ped, CTask*, int32 command)` (0x465C20)
    fn give_ped_scripted_task(s: Scr, ped: u32, task: GPtr<u8>, command: u32) = thiscall 0x465C20 => unported;
    /// CRT `_stricmp` (0x8229B6)
    fn stricmp(a: GPtr<u8>, b: GPtr<u8>) -> i32 = cdecl 0x8229B6 => unported;
    /// `CMenuSystem::CreateNewMenu(eMenuType, const char*, float x, float y, float width, int8 columns, bool
    /// interactive, bool background, eFontAlignment)` (0x582300), AL; floats as raw dwords
    fn create_new_menu(
        ty: u32, title: GPtr<u8>, x: u32, y: u32, w: u32, columns: u32, interactive: u32, background: u32, align: u32
    ) -> u8 = cdecl 0x582300 => unported;
    /// `CMenuSystem::InsertMenu(uint8 menu, uint8 column, char* title, char* rows[12])` (0x581E00), 15 dwords
    fn insert_menu(
        menu: u32, column: u32, a: GPtr<u8>, b: GPtr<u8>, r0: GPtr<u8>, r1: GPtr<u8>, r2: GPtr<u8>,
        r3: GPtr<u8>, r4: GPtr<u8>, r5: GPtr<u8>, r6: GPtr<u8>, r7: GPtr<u8>, r8: GPtr<u8>, r9: GPtr<u8>,
        r10: GPtr<u8>
    ) = cdecl 0x581E00 => unported;
}

/// Reads an 8-byte label (zeroed 16-byte buffer) and, unless `_stricmp(label, "DUMMY") == 0`, copies it into
/// `dst` byte by byte up to and including the terminator (the original's inline strcpy loop). `dst[0]` is
/// cleared by the caller as in the original.
fn label_unless_dummy(s: Scr, dst: GPtr<u8>) {
    let mut tmp = [0u8; 16];
    let t = GPtr::from_raw(tmp.as_mut_ptr());
    read_text_label(s, t, 8);
    if stricmp(t, GPtr::from_addr(DUMMY)) != 0 {
        let mut i = 0isize;
        loop {
            let c = t.byte_add(i).read();
            dst.byte_add(i).write(c);
            i += 1;
            if c == 0 {
                break;
            }
        }
    }
}

/// The 8-byte label read (zeroed 16-byte buffer) and the original's inline strcpy loop into `dst`
/// (08EE/08EF: no DUMMY test).
fn label_unless_dummy_raw(s: Scr, dst: GPtr<u8>) {
    let mut tmp = [0u8; 16];
    let t = GPtr::from_raw(tmp.as_mut_ptr());
    read_text_label(s, t, 8);
    let mut i = 0isize;
    loop {
        let c = t.byte_add(i).read();
        dst.byte_add(i).write(c);
        i += 1;
        if c == 0 {
            break;
        }
    }
}

/// 089E / 08E5: centre (P0, P1, P2), radius P3; walks the ped pool from the top slot down (slot accessor
/// 0x4082A0); a candidate is non-null, random (`m_nCreatedBy` +0x484 == 1), entity flags +0x1C bit 11 clear,
/// +0x470 bit 3 clear, not `IsPedDead`, then 089E: model info byte +0x32 bit 0 set (drugs buyer) and no
/// group; 08E5: no group and +0x474 bit 23 clear. d = `Magnitude(pos - centre)` (components `fsub`, stored
/// f32; the magnitude `fst` to f32); taken when `d < radius` and `(float)best > d` (`fild`), then handle =
/// GetRef and best = `_ftol2(d)`. A found handle: `SetCharCreatedBy(2)` on GetAtRef(handle), mission ped
/// count + 1, mission cleanup (type 2). P0 = handle (-1 when none); StoreParameters(1).
fn random_char_in_sphere(s: Scr, drugs_buyers: bool) {
    collect(s, 4);
    let (cx, cy, cz, radius) = (spf(0), spf(1), spf(2), spf(3));
    let mut handle: i32 = -1;
    let mut best: i32 = 9999;
    let mut n = pool(PED_POOL).byte_add(8).cast::<i32>().read();
    while n != 0 {
        n -= 1;
        let pp = pool(PED_POOL);
        let ped = ped_pool_get_at(pp, n);
        if ped.is_null()
            || ped.byte_add(0x484).read() != 1
            || ped.byte_add(0x1C).cast::<u32>().read() & 0x800 != 0
            || ped.byte_add(0x470).read() & 8 != 0
            || script_is_ped_dead(s, ped) != 0
        {
            continue;
        }
        if drugs_buyers {
            let model = i32::from(ped.byte_add(0x22).cast::<i16>().read());
            let mi = GPtr::<GPtr<u8>>::from_addr(0xA9B0C8usize.wrapping_add((model as u32).wrapping_mul(4) as usize)).read();
            if mi.byte_add(0x32).read() & 1 == 0 || !get_peds_group(ped).is_null() {
                continue;
            }
        } else if !get_peds_group(ped).is_null() || ped.byte_add(0x474).cast::<u32>().read() & 0x80_0000 != 0 {
            continue;
        }
        let m = ped.byte_add(0x14).cast::<GPtr<u8>>().read();
        let pos = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) }.cast::<f32>();
        let mut v = [
            X87::from_f32(pos.read()).fsub_m(cx).to_f32().to_bits(),
            X87::from_f32(pos.byte_add(4).read()).fsub_m(cy).to_f32().to_bits(),
            X87::from_f32(pos.byte_add(8).read()).fsub_m(cz).to_f32().to_bits(),
        ];
        let d = X87::from_f64_exact(vector_magnitude(GPtr::from_raw(v.as_mut_ptr()))).to_f32();
        if X87::from_f32(d).fcom(X87::from_f32(radius)) != sa_math::x87::Cmp::Less {
            continue;
        }
        if X87::from_i32(best).fcom(X87::from_f32(d)) != sa_math::x87::Cmp::Greater {
            continue;
        }
        handle = ped_get_ref(pool(PED_POOL), ped);
        best = X87::from_f32(d).ftol() as u32 as i32;
    }
    if handle >= 0 {
        set_char_created_by(ped_at_ref(pool(PED_POOL), handle), 2);
        let c = GPtr::<u32>::from_addr(0xC0EC24);
        c.write(c.read().wrapping_add(1));
        if uses_mission_cleanup(s) {
            mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), handle, 2);
        }
    }
    set_sp(0, handle as u32);
    store(s, 1);
}

/// 08A0 TASK_USE_CLOSEST_MAP_ATTRACTOR (0x474C0A): `CollectParameters(6)` (ped, range, model, x, y, z), 8-byte
/// attractor name (zeroed 16-byte buffer). Ped -1: position (P3, P4, P5), no ped; else the ped's position.
/// inUse = ped ? `m_pIntelligence->GetEffectInUse()` : 0; model: 0 → -1, < 0 → used-object model. Search
/// 0x6002F0(&pos, P1, model, 5, name, 1, inUse, &effect, &entity) with both outputs zeroed first. No effect →
/// flag 0, return 0. A script with `m_ExternalType` (+0xC9) != -1 (a brain): moved from the active to the idle
/// list, `ShutdownThisScript()`, a stack `CEventAttractor(effect, entity, true)` (0x20 bytes) with word +0xE =
/// 0xE9, `ped->m_pIntelligence->m_eventGroup.Add(&event, true)`, `SetEffectInUse(effect)`, the destructor;
/// returns 1 without touching the flag. Otherwise `new CTaskComplexUseEffect(effect, entity)` (null when `new`
/// fails), `GivePedScriptedTask(P0, task, command)`, flag 1, return 0. The SEH state stores are not ported.
fn task_use_closest_map_attractor(s: Scr, command: u32) -> i8 {
    collect(s, 6);
    let ped_h = spi(0);
    let range = sp(1);
    let mut model = spi(2);
    let (x, y, z) = (sp(3), sp(4), sp(5));
    let mut name = [0u8; 16];
    let np = GPtr::from_raw(name.as_mut_ptr());
    read_text_label(s, np, 8);
    let (ped, mut pos) = if ped_h == -1 {
        (GPtr::<u8>::NULL, [x, y, z])
    } else {
        let p = ped_at_ref(pool(PED_POOL), ped_h);
        let m = p.byte_add(0x14).cast::<GPtr<u8>>().read();
        let q = if m.is_null() { p.byte_add(4) } else { m.byte_add(0x30) }.cast::<u32>();
        (p, [q.read(), q.byte_add(4).read(), q.byte_add(8).read()])
    };
    let in_use = if ped.is_null() {
        0
    } else {
        get_effect_in_use(ped.byte_add(0x47C).cast::<GPtr<u8>>().read())
    };
    if model == 0 {
        model = -1;
    } else if model < 0 {
        model = GPtr::<i32>::from_addr(0xA44B88usize.wrapping_add(model.wrapping_neg().wrapping_mul(0x1C) as u32 as usize))
            .read();
    }
    let mut effect = [0u32; 1];
    let mut entity = [0u32; 1];
    find_closest_attractor(
        GPtr::from_raw(pos.as_mut_ptr()),
        range,
        model,
        5,
        np,
        1,
        in_use,
        GPtr::from_raw(effect.as_mut_ptr()),
        GPtr::from_raw(entity.as_mut_ptr()),
    );
    if effect[0] == 0 {
        set_flag(s, false);
        return 0;
    }
    if s.byte_add(0xC9).read() != 0xFF {
        remove_script_from_list(s, GPtr::from_addr(0xA8B42C));
        add_script_to_list(s, GPtr::from_addr(0xA8B428));
        shutdown_this_script(s);
        let mut ev = [0u32; 8];
        let e = GPtr::from_raw(ev.as_mut_ptr()).cast::<u8>();
        event_attractor_ctor(e, effect[0], entity[0], 1);
        let intel = ped.byte_add(0x47C).cast::<GPtr<u8>>().read();
        e.byte_add(0xE).cast::<u16>().write(0xE9);
        event_group_add(intel.byte_add(0x68), e, 1);
        set_effect_in_use(ped.byte_add(0x47C).cast::<GPtr<u8>>().read(), effect[0]);
        event_attractor_dtor(e);
        return 1;
    }
    let t = task_new(0x24);
    let t = if t.is_null() { GPtr::NULL } else { task_use_effect_ctor(t, effect[0], entity[0]) };
    give_ped_scripted_task(s, ped_h as u32, t, command);
    set_flag(s, true);
    0
}

/// 08D4 CREATE_MENU (0x475500): title (label unless "DUMMY", else ""), `CollectParameters(7)`; sx =
/// `fild maximumWidth; fmul 1/640`; x = P0 * sx, w = sx * P2, y = (`fild maximumHeight; fmul 1/448`) * P1
/// (x87, stored f32); `P0 = (u8)CreateNewMenu(0, title, x, y, w, P3, P4, P5, P6)`; StoreParameters(1).
fn create_menu(s: Scr) {
    let mut title = [0u8; 16];
    let tp = GPtr::from_raw(title.as_mut_ptr());
    label_unless_dummy(s, tp);
    collect(s, 7);
    let sx = X87::from_i32(GPtr::<i32>::from_addr(MAXIMUM_WIDTH).read()).fmul_m(f(K_1_640));
    let x = X87::from_f32(spf(0)).fmul(sx).to_f32().to_bits();
    let w = sx.fmul_m(spf(2)).to_f32().to_bits();
    let y = X87::from_i32(GPtr::<i32>::from_addr(MAXIMUM_HEIGHT).read())
        .fmul_m(f(K_1_448))
        .fmul_m(spf(1))
        .to_f32()
        .to_bits();
    let id = create_new_menu(0, tp, x, y, w, sp(3), sp(4), sp(5), sp(6));
    set_sp(0, u32::from(id));
    store(s, 1);
}

/// 08DB SET_MENU_COLUMN (0x4756B4): `CollectParameters(2)` (menu, column, kept); 13 labels: the column
/// title A, B and 11 rows (10-byte buffers, first bytes cleared before the row loop), each copied unless
/// "DUMMY"; `InsertMenu(P0, P1, A, B, row0..row10)`.
fn set_menu_column(s: Scr) {
    collect(s, 2);
    let (menu, column) = (sp(0), sp(1));
    let mut a = [0u8; 16];
    let ap = GPtr::from_raw(a.as_mut_ptr());
    label_unless_dummy(s, ap);
    let mut b = [0u8; 16];
    let bp = GPtr::from_raw(b.as_mut_ptr());
    label_unless_dummy(s, bp);
    let mut rows = [0u8; 112];
    let r = GPtr::from_raw(rows.as_mut_ptr());
    for i in 0..11 {
        label_unless_dummy(s, r.byte_add(10 * i));
    }
    let row = |i: isize| r.byte_add(10 * i);
    insert_menu(
        menu,
        column,
        ap,
        bp,
        row(0),
        row(1),
        row(2),
        row(3),
        row(4),
        row(5),
        row(6),
        row(7),
        row(8),
        row(9),
        row(10),
    );
}

/// `CRunningScript::ProcessCommands2200To2299(int32 command)` (0x474900, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x474900, conv = thiscall, name = "CRunningScript::ProcessCommands2200To2299", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript")]
pub fn process_commands_2200_to_2299(s: Scr, command: u32) -> i8 {
    match command {
        // 08D2 SET_OBJECT_SCALE (0x4754B8): `OBJ(SP0)->m_fScale (+0x15C) = SP1` (raw dword, no null check)
        0x08D2 => {
            collect(s, 2);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            o.byte_add(0x15C).cast::<u32>().write(sp(1));
        }
        // 0899, 089A, 089D, 08A1, 08AA, 08AE, 08B0, 08CC..08CF, 08D5: the jump table sends them to 0x47611C
        // (`xor al,al`: no operand read, return 0; gr nop / unimplemented).
        0x0899 | 0x089A | 0x089D | 0x08A1 | 0x08AA | 0x08AE | 0x08B0 | 0x08CC..=0x08CF | 0x08D5 => {}
        // 08A9 STREAM_SCRIPT (0x47500D) / 08AB HAS_STREAMED_SCRIPT_LOADED (0x475049): `CollectParameters(1)`;
        // P0 = (int16)StreamedScripts.GetProperIndexFromIndexUsedByScript((u16)P0) (written back); 08A9:
        // `RequestModel(P0 + 26230, 4)`; 08AB: flag = load state byte (0x964E08 + 20 * P0) == 1 (tail 0x4760E5).
        0x08A9 | 0x08AB => {
            collect(s, 1);
            let i = i32::from(
                get_proper_index_from_index_used_by_script(GPtr::from_addr(STREAMED_SCRIPTS), sp(0) & 0xFFFF) as i16,
            );
            set_sp(0, i as u32);
            if command == 0x08A9 {
                request_model(i.wrapping_add(0x6676), 4);
            } else {
                let st = GPtr::<u8>::from_addr(SCM_LOAD_STATE.wrapping_add(i.wrapping_mul(20) as usize)).read();
                set_flag(s, st == 1);
            }
        }
        // 08B4..08B9 IS_{GLOBAL,LOCAL}_VAR_BIT_SET_{CONST,VAR,LVAR} (0x475244): `CollectParameters(2)`;
        // flag = (P0 & (1 << (P1 & 31))) != 0 (`shl edx,cl`).
        0x08B4..=0x08B9 => {
            collect(s, 2);
            set_flag(s, sp(0) & (1u32 << (sp(1) & 31)) != 0);
        }
        // 08BA..08BC SET_GLOBAL_VAR_BIT (0x475274) / 08BD..08BF SET_LOCAL_VAR_BIT (0x4752A2): `p =
        // GetPointerToScriptVariable(2 / 1)`, `CollectParameters(1)`, `*p |= 1 << (P0 & 31)`.
        0x08BA..=0x08BF => {
            let p = var_ptr(s, if command <= 0x08BC { 2 } else { 1 });
            collect(s, 1);
            p.write(p.read() | (1u32 << (sp(0) & 31)));
        }
        // 08C0..08C2 CLEAR_GLOBAL_VAR_BIT (0x4752CC) / 08C3..08C5 CLEAR_LOCAL_VAR_BIT (0x4752FC): as above with
        // `*p &= ~(1 << (P0 & 31))`.
        0x08C0..=0x08C5 => {
            let p = var_ptr(s, if command <= 0x08C2 { 2 } else { 1 });
            collect(s, 1);
            p.write(p.read() & !(1u32 << (sp(0) & 31)));
        }
        // 0898 ENABLE_CRANE_CONTROLS (0x47493E): bytes 0xA44496 / 0xA44495 / 0xA44494 (bEnableCraneRaise /
        // Lower / Release) = P0 / P1 / P2 != 0.
        0x0898 => {
            collect(s, 3);
            GPtr::<u8>::from_addr(0xA44496).write(u8::from(sp(0) != 0));
            GPtr::<u8>::from_addr(0xA44495).write(u8::from(sp(1) != 0));
            GPtr::<u8>::from_addr(0xA44494).write(u8::from(sp(2) != 0));
        }
        // 08A3 CAN_TRIGGER_GANG_WAR_WHEN_ON_A_MISSION (0x474E88): CGangWars::bCanTriggerGangWarWhenOnAMission
        // (0x96AB93) = P0 != 0.
        0x08A3 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0x96AB93).write(u8::from(sp(0) != 0));
        }
        // 08A5 WINCH_CAN_PICK_VEHICLE_UP (0x474ED7): no null check; byte +0x42D bit 4 = P1 != 0.
        0x08A5 => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let b = v.byte_add(0x42D);
            let c = b.read();
            b.write(if sp(1) != 0 { c | 0x10 } else { c & 0xEF });
        }
        // 08A8 SET_ALWAYS_DRAW_3D_MARKERS (0x474FD9): C3dMarkers::ForceRender(P0 != 0).
        0x08A8 => {
            collect(s, 1);
            markers_force_render(u32::from(sp(0) != 0));
        }
        // 08AC SET_GANG_WARS_TRAINING_MISSION (0x475085): CGangWars::bTrainingMission (0x96AB91) = P0 != 0.
        0x08AC => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0x96AB91).write(u8::from(sp(0) != 0));
        }
        // 08B1 SET_NIGHT_VISION (0x4751A5) / 08B2 SET_INFRARED_VISION (0x4751D9): the switch with P0 != 0.
        0x08B1 => {
            collect(s, 1);
            script_night_vision_switch(u32::from(sp(0) != 0));
        }
        0x08B2 => {
            collect(s, 1);
            script_infrared_vision_switch(u32::from(sp(0) != 0));
        }
        // 08B3 SET_ZONE_FOR_GANG_WARS_TRAINING (0x47520D): 8-byte label (zeroed 16-byte buffer, see the module
        // notes); `i = (int16)FindZoneByLabel(label, 2)`; CGangWars::ZoneInfoForTraining (0x8A5F40) = the word at
        // 0xBA37B4 + 32 * i (zero-extended).
        0x08B3 => {
            let mut buf = [0u8; 16];
            let b = GPtr::from_raw(buf.as_mut_ptr());
            read_text_label(s, b, 8);
            let i = i32::from(find_zone_by_label(b, 2) as i16);
            let w = GPtr::<u16>::from_addr(0xBA37B4usize.wrapping_add((i << 5) as usize)).read();
            GPtr::<u32>::from_addr(0x8A5F40).write(u32::from(w));
        }
        // 08C8 ADD_PRICE_MODIFIER (0x4753B0) / 08C9 REMOVE_PRICE_MODIFIER (0x4753D6).
        0x08C8 => {
            collect(s, 2);
            add_price_modifier(sp(0), sp(1));
        }
        0x08C9 => {
            collect(s, 1);
            remove_price_modifier(sp(0));
        }
        // 08D0 IS_SKIP_CUTSCENE_BUTTON_PRESSED (0x475459): no operand; flag = IsCutsceneSkipButtonBeingPressed().
        0x08D0 => {
            let b = is_cutscene_skip_button_being_pressed() != 0;
            set_flag(s, b);
        }
        // 08DD SWITCH_DEATH_PENALTIES (0x475891) / 08DE SWITCH_ARREST_PENALTIES (0x4758B1): bytes 0x8A5E48 /
        // 0x8A5E49 = P0 != 0.
        0x08DD | 0x08DE => {
            collect(s, 1);
            let a = if command == 0x08DD { 0x8A5E48 } else { 0x8A5E49 };
            GPtr::<u8>::from_addr(a).write(u8::from(sp(0) != 0));
        }
        // 08DF SET_EXTRA_HOSPITAL_RESTART_POINT (0x4758D0): (P0, P1, P2) → 0xA43414.., P3 → 0xA43258 (heading),
        // P4 → 0xA43254 (raw dwords, in this order).
        0x08DF => {
            collect(s, 5);
            GPtr::<u32>::from_addr(0xA43414).write(sp(0));
            GPtr::<u32>::from_addr(0xA43418).write(sp(1));
            GPtr::<u32>::from_addr(0xA4341C).write(sp(2));
            GPtr::<u32>::from_addr(0xA43258).write(sp(3));
            GPtr::<u32>::from_addr(0xA43254).write(sp(4));
        }
        // 08E0 SET_EXTRA_POLICE_STATION_RESTART_POINT (0x475926): P0 → 0xA43420, P1 → 0xA43424, P3 → 0xA43250,
        // P2 → 0xA43428, P4 → 0xA4324C (raw dwords, in this order).
        0x08E0 => {
            collect(s, 5);
            GPtr::<u32>::from_addr(0xA43420).write(sp(0));
            GPtr::<u32>::from_addr(0xA43424).write(sp(1));
            GPtr::<u32>::from_addr(0xA43250).write(sp(3));
            GPtr::<u32>::from_addr(0xA43428).write(sp(2));
            GPtr::<u32>::from_addr(0xA4324C).write(sp(4));
        }
        // 08E2 GET_TERRITORY_UNDER_CONTROL_PERCENTAGE (0x475996): no operand read before the store;
        // P0 = _ftol2(float at 0x96AB9C * 100.0) (low dword); StoreParameters(1).
        0x08E2 => {
            let v = GPtr::<f32>::from_addr(0x96AB9C).read();
            let n = X87::from_f32(v).fmul_m(f(0x42C8_0000)).ftol() as u32;
            set_sp(0, n);
            store(s, 1);
        }
        // 08E6 SET_PLANE_UNDERCARRIAGE_UP (0x475B96): no null check; P1 != 0 → SetGearUp, else SetGearDown.
        0x08E6 => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            if sp(1) != 0 {
                plane_set_gear_up(v);
            } else {
                plane_set_gear_down(v);
            }
        }
        // 08E7 DISABLE_ALL_ENTRY_EXITS (0x475BD4): CEntryExitManager::ms_bDisabled (0x96A7C8) = P0 != 0.
        0x08E7 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0x96A7C8).write(u8::from(sp(0) != 0));
        }
        // 08E9 SET_OBJECT_AS_STEALABLE (0x475C40): no null check; dword +0x140 bit 0x2000 = P1 != 0.
        0x08E9 => {
            collect(s, 2);
            let o = object_at_ref(pool(OBJECT_POOL), spi(0));
            let f1 = o.byte_add(0x140).cast::<u32>();
            let v = f1.read();
            f1.write(if sp(1) != 0 { v | 0x2000 } else { v & 0xFFFF_DFFF });
        }
        // 08EA SET_CREATE_RANDOM_GANG_MEMBERS (0x475C91): byte 0xC0FCB2 (m_bDontCreateRandomGangMembers) = P0 == 0.
        0x08EA => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0xC0FCB2).write(u8::from(sp(0) == 0));
        }
        // 08EC GET_VEHICLE_CLASS (0x475D73): no null check; P0 = (int8)ms_modelInfoPtrs[(int16)+0x22]->+0x4D
        // (movsx); StoreParameters(1).
        0x08EC => {
            collect(s, 1);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let model = i32::from(v.byte_add(0x22).cast::<i16>().read());
            let mi = GPtr::<GPtr<u8>>::from_addr(0xA9B0C8usize.wrapping_add((model as u32).wrapping_mul(4) as usize)).read();
            set_sp(0, i32::from(mi.byte_add(0x4D).cast::<i8>().read()) as u32);
            store(s, 1);
        }
        // 08ED CLEAR_CONVERSATION_FOR_CHAR (0x475DB1): a null ped does nothing; else
        // RemoveConversationForPed(ped).
        0x08ED => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            if !ped.is_null() {
                remove_conversation_for_ped(ped);
            }
        }
        // 08F2 VEHICLE_CAN_BE_TARGETTED_BY_HS_MISSILE (0x475F0A) / 08F3 SET_FREEBIES_IN_VEHICLE (0x475F55): no
        // null check; byte +0x42D bit 6 / +0x428 bit 7 = P1 != 0.
        0x08F2 | 0x08F3 => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let (off, mask) = if command == 0x08F2 { (0x42D, 0x40u8) } else { (0x428, 0x80u8) };
            let b = v.byte_add(off);
            let c = b.read();
            b.write(if sp(1) != 0 { c | mask } else { c & !mask });
        }
        // 08F5 MAKE_PLAYER_GANG_DISAPPEAR (0x47600F) / 08F6 MAKE_PLAYER_GANG_REAPPEAR (0x476027): no operand;
        // on FindPlayerPed(-1).
        0x08F5 => make_player_group_disappear(find_player_ped(-1)),
        0x08F6 => make_player_group_reappear(find_player_ped(-1)),
        // 08F8 SHOW_UPDATE_STATS (0x47608E): byte 0x8CDE56 = P0 != 0.
        0x08F8 => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0x8CDE56).write(u8::from(sp(0) != 0));
        }
        // 08F9 IS_VAR_TEXT_LABEL16_EQUAL_TO_TEXT_LABEL (0x4760AB) / 08FA IS_LVAR_... (0x4760AF): `p =
        // GetPointerToScriptVariable(2 / 1)`; 16-byte label (zeroed 17-byte buffer: the original's is not
        // cleared, which matters only when the label leaves bytes unwritten before a terminator);
        // flag = strncmp(p, label, 16) == 0.
        0x08F9 | 0x08FA => {
            let v = var_ptr(s, if command == 0x08F9 { 2 } else { 1 });
            let mut buf = [0u8; 17];
            let b = GPtr::from_raw(buf.as_mut_ptr());
            read_text_label(s, b, 16);
            let eq = strncmp(v.cast(), b, 16) == 0;
            set_flag(s, eq);
        }
        // 08FB SET_COORD_BLIP_APPEARANCE (0x4760FC): SetCoordBlipAppearance(P0, (u8)P1).
        0x08FB => {
            collect(s, 2);
            set_coord_blip_appearance(sp(0), sp(1) & 0xFF);
        }
        // 089B IS_PLAYER_IN_POSITION_FOR_CONVERSATION (0x47497F): flag = IsPlayerInPositionForConversation(ped,
        // false) (no null check).
        0x089B => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let b = is_player_in_position_for_conversation(ped, 0) != 0;
            set_flag(s, b);
        }
        // 089C ENABLE_CONVERSATION (0x4749BB): EnableConversation(ped, P1 != 0).
        0x089C => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            enable_conversation(ped, u32::from(sp(1) != 0));
        }
        // 089F GET_PED_TYPE (0x474BD4): no null check; P0 = dword +0x598; StoreParameters(1).
        0x089F => {
            collect(s, 1);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            set_sp(0, ped.byte_add(0x598).cast::<u32>().read());
            store(s, 1);
        }
        // 08A2 PLANE_ATTACK_PLAYER_USING_DOG_FIGHT (0x474E43): no null check; `fld P2`; mission byte +0x3BA = 0x3F
        // unless it is 0x39/0x3A; +0x9B0 = P2 (`fstp`). P1 is collected but unused.
        0x08A2 => {
            collect(s, 3);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let z = fld_fst(spf(2));
            let m = v.byte_add(0x3BA);
            let c = m.read();
            if c != 0x39 && c != 0x3A {
                m.write(0x3F);
            }
            v.byte_add(0x9B0).cast::<f32>().write(z);
        }
        // 08A4 CONTROL_MOVABLE_VEHICLE_PART (0x474EA8): no null check; UpdateMovingCollision(P1).
        0x08A4 => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            update_moving_collision(v, sp(1));
        }
        // 08A6 OPEN_CAR_DOOR_A_BIT (0x474F22): no null check; node = GetCarNodeIndexFromDoor(P1); nothing when
        // IsDoorMissing(P1) or the node frame (+0x648 + 4 * node) is null; else OpenDoor(nullptr, node, P1, P2, true).
        0x08A6 => {
            collect(s, 3);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let door = sp(1);
            let node = get_car_node_index_from_door(door);
            if vehicle_is_door_missing(v, door) == 0
                && v.byte_add(0x648 + 4 * node as isize).cast::<u32>().read() != 0
            {
                vehicle_open_door(v, GPtr::NULL, node, door, sp(2), 1);
            }
        }
        // 08A7 IS_CAR_DOOR_FULLY_OPEN (0x474F8C): no null check; flag = vslot 0x84(GetCarNodeIndexFromDoor(P1)).
        0x08A7 => {
            collect(s, 2);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let node = get_car_node_index_from_door(sp(1));
            let b = vehicle_is_door_fully_open(v, node) != 0;
            set_flag(s, b);
        }
        // 08AF SET_CHAR_MAX_HEALTH (0x475170): no null check; +0x544 = (float)P1 (`fild; fstp`).
        0x08AF => {
            collect(s, 2);
            let h = X87::from_i32(spi(1)).to_f32();
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            ped.byte_add(0x544).cast::<f32>().write(h);
        }
        // 08C6 SET_CHAR_CAN_BE_KNOCKED_OFF_BIKE (0x47532C): no null check; dword +0x474 bits 27..28 = bits 0..1
        // of P1 (`(old ^ ((P1 << 27 ^ old) & 0x18000000))`).
        0x08C6 => {
            collect(s, 2);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let fl = ped.byte_add(0x474).cast::<u32>();
            let old = fl.read();
            fl.write(old ^ (((sp(1) << 27) ^ old) & 0x1800_0000));
        }
        // 08C7 SET_CHAR_COORDINATES_DONT_WARP_GANG (0x47536F): SetCharCoordinates(ped, P1, P2, P3, false, true).
        0x08C7 => {
            collect(s, 4);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            set_char_coordinates(s, ped, sp(1), sp(2), sp(3), 0, 1);
        }
        // 08CA INIT_ZONE_POPULATION_SETTINGS (0x4753F4): no operand.
        0x08CA => init_zones_population_settings(),
        // 08CB EXPLODE_CAR_IN_CUTSCENE_SHAKE_AND_BITS (0x475400): no null check; byte +0x42A |= 0x20; vslot
        // 0xA8(P1 == 0, P2 == 0, false, P3 != 0) (`setcc cl` bools passed as 0/1).
        0x08CB => {
            collect(s, 4);
            let v = vehicle_at_ref(pool(VEHICLE_POOL), spi(0));
            let b = v.byte_add(0x42A);
            b.write(b.read() | 0x20);
            vehicle_blow_up_cutscene(v, u32::from(sp(1) == 0), u32::from(sp(2) == 0), 0, u32::from(sp(3) != 0));
        }
        // 08D1 GET_CUTSCENE_OFFSET (0x47547A): P0..P2 = CCutsceneMgr::ms_cutsceneOffset (0xBC4034..0xBC403C);
        // StoreParameters(3).
        0x08D1 => {
            set_sp(1, GPtr::<u32>::from_addr(0xBC4038).read());
            set_sp(0, GPtr::<u32>::from_addr(0xBC4034).read());
            set_sp(2, GPtr::<u32>::from_addr(0xBC403C).read());
            store(s, 3);
        }
        // 08D3 GET_CURRENT_POPULATION_ZONE_TYPE (0x4754E6): P0 = dword 0xC0BC6C; StoreParameters(1).
        0x08D3 => {
            set_sp(0, GPtr::<u32>::from_addr(0xC0BC6C).read());
            store(s, 1);
        }
        // 08E1 FIND_NUMBER_TAGS_TAGGED (0x47597A): P0 = dword 0xA9AD74; StoreParameters(1).
        0x08E1 => {
            set_sp(0, GPtr::<u32>::from_addr(0xA9AD74).read());
            store(s, 1);
        }
        // 08E3 / 08E4 IS_OBJECT_IN_ANGLED_AREA_2D/3D (0x4759BC): ObjectInAngledAreaCheckCommand(command).
        0x08E3 | 0x08E4 => object_in_angled_area_check_command(s, command),
        // 08E8 ATTACH_ANIMS_TO_MODEL (0x475BF3): model P0 (< 0: used-object model, resolved after the label);
        // 8-byte IFP name (zeroed 16-byte buffer); ScriptAttachAnimGroupToCharModel(model, name) (result unused),
        // then AddToListOfSpecialAnimGroupsAttachedToCharModels(model, name).
        0x08E8 => {
            collect(s, 1);
            let mut model = spi(0);
            let mut buf = [0u8; 16];
            let b = GPtr::from_raw(buf.as_mut_ptr());
            read_text_label(s, b, 8);
            if model < 0 {
                model = GPtr::<i32>::from_addr(0xA44B88usize.wrapping_sub(model.wrapping_mul(0x1C) as u32 as usize)).read();
            }
            script_attach_anim_group_to_char_model(model, b);
            add_to_list_of_special_anim_groups(model, b);
        }
        // 08EE SET_MENU_ITEM_WITH_NUMBER / 08EF SET_MENU_ITEM_WITH_2_NUMBERS (0x475DE4): `CollectParameters(3)`
        // (menu, column, row kept); 8-byte key (zeroed buffer) copied with the byte loop; 08EE:
        // `CollectParameters(1)`, n2 = -1; 08EF: `CollectParameters(2)`, n2 = P1;
        // InsertOneMenuItemWithNumber(menu, column, row, key, P0, n2).
        0x08EE | 0x08EF => {
            collect(s, 3);
            let (menu, col, row) = (sp(0), sp(1), sp(2));
            let mut key = [0u8; 16];
            label_unless_dummy_raw(s, GPtr::from_raw(key.as_mut_ptr()));
            let n2 = if command == 0x08EE {
                collect(s, 1);
                u32::MAX
            } else {
                collect(s, 2);
                sp(1)
            };
            insert_one_menu_item_with_number(menu, col, row, GPtr::from_raw(key.as_mut_ptr()), sp(0), n2);
        }
        // 08F0 APPEND_TO_NEXT_CUTSCENE (0x475E76): two 8-byte labels (zeroed buffers);
        // AppendToNextCutscene(first, second).
        0x08F0 => {
            let mut a = [0u8; 16];
            let mut b = [0u8; 16];
            let ap = GPtr::from_raw(a.as_mut_ptr());
            let bp = GPtr::from_raw(b.as_mut_ptr());
            read_text_label(s, ap, 8);
            read_text_label(s, bp, 8);
            append_to_next_cutscene(ap, bp);
        }
        // 08F1 GET_NAME_OF_INFO_ZONE (0x475EB7): zone = FindSmallestZoneForPosition(&(P0, P1, P2), true);
        // `strncpy(GetPointerToScriptVariable(2), zone, 8)` (the zone's label is its first field; no null check).
        0x08F1 => {
            collect(s, 3);
            let mut pos = [sp(0), sp(1), sp(2)];
            let z = find_smallest_zone_for_position(GPtr::from_raw(pos.as_mut_ptr()), 1);
            let v = var_ptr(s, 2);
            strncpy_(v.cast(), z, 8);
        }
        // 08F4 SET_SCRIPT_LIMIT_TO_GANG_SIZE (0x475FA0): FindPlayerPed(-1)->m_pPlayerData (+0x480) byte +0x43 =
        // (u8)P0; group membership = 0xC09928 + 0x2D4 * playerData->+0x38 (FindPlayerPed called again for each
        // read); n = CountMembersExcludingLeader() - that byte; n > 0 → RemoveNFollowers(n).
        0x08F4 => {
            collect(s, 1);
            let pd = |p: GPtr<u8>| p.byte_add(0x480).cast::<GPtr<u8>>().read();
            pd(find_player_ped(-1)).byte_add(0x43).write(sp(0) as u8);
            let g = pd(find_player_ped(-1)).byte_add(0x38).cast::<u32>().read();
            let d = pd(find_player_ped(-1));
            let m = GPtr::<u8>::from_addr(0xC09928usize.wrapping_add(g.wrapping_mul(0x2D4) as usize));
            let n = count_members_excluding_leader(m).wrapping_sub(i32::from(d.byte_add(0x43).read()));
            if n > 0 {
                remove_n_followers(m, n);
            }
        }
        // 08F7 GET_CLOTHES_ITEM (0x47603F): info = Players[P0] (0x190 stride); t = GetTextureDependency(P1);
        // desc = dword info +8; P0 = desc[P1 + 10] (+0x28 + 4*P1), P1 = desc[t]; StoreParameters(2).
        0x08F7 => {
            collect(s, 2);
            let info = player_info(sp(0));
            let i = sp(1);
            let t = get_texture_dependency(i);
            let desc = info.byte_add(8).cast::<GPtr<u8>>().read();
            set_sp(0, desc.byte_add(0x28 + 4 * i as i32 as isize).cast::<u32>().read());
            set_sp(1, desc.byte_add(4 * t as isize).cast::<u32>().read());
            store(s, 2);
        }
        // 089E GET_RANDOM_CHAR_IN_SPHERE_ONLY_DRUGS_BUYERS (0x474A04) / 08E5 GET_RANDOM_CHAR_IN_SPHERE_NO_BRAIN
        // (0x4759CB): see [`random_char_in_sphere`].
        0x089E => random_char_in_sphere(s, true),
        0x08E5 => random_char_in_sphere(s, false),
        // 08AD SET_CHAR_HAS_USED_ENTRY_EXIT (0x4750A5): no null check. e = entry-exit slot
        // `FindNearestEntryExit(&(P1, P2), P3, -1)`; linked = e->+0x38 when non-null (and then linked->+0x38 = e),
        // else e; ped area (+0x2F) = linked->+0x32; ped +0x78C = linked->+0x32 != 0 ? e : null. For a player:
        // byte 0x96A7C4 = 0, AddEntryExitToStack(e), then linked->+0x33 (u8) > 0 ? StartExtraColour(that - 1,
        // false) : StopExtraColour(false).
        0x08AD => {
            collect(s, 4);
            let ped = ped_at_ref(pool(PED_POOL), spi(0));
            let mut pos = [sp(1), sp(2)];
            let i = find_nearest_entry_exit(GPtr::from_raw(pos.as_mut_ptr()), sp(3), -1);
            let e = entry_exit_at(i);
            let link = e.byte_add(0x38).cast::<GPtr<u8>>().read();
            let l = if link.is_null() {
                e
            } else {
                link.byte_add(0x38).cast::<GPtr<u8>>().write(e);
                link
            };
            ped.byte_add(0x2F).write(l.byte_add(0x32).read());
            let area = l.byte_add(0x32).read();
            ped.byte_add(0x78C).cast::<u32>().write(if area != 0 { e.addr() as u32 } else { 0 });
            if ped_is_player(ped) != 0 {
                GPtr::<u32>::from_addr(0x96A7C4).write(0);
                add_entry_exit_to_stack(e);
                let c = l.byte_add(0x33).read();
                if c > 0 {
                    start_extra_colour(u32::from(c) - 1, 0);
                } else {
                    stop_extra_colour(0);
                }
            }
        }
        // 08DC SET_BLIP_ENTRY_EXIT (0x475838): SetBlipEntryExit(P0, slot(FindNearestEntryExit(&(P1, P2), P3, 0))).
        0x08DC => {
            collect(s, 4);
            let mut pos = [sp(1), sp(2)];
            let i = find_nearest_entry_exit(GPtr::from_raw(pos.as_mut_ptr()), sp(3), 0);
            set_blip_entry_exit(sp(0), entry_exit_at(i));
        }
        // 08EB ADD_SPARKS (0x475CB0): pos (P0, P1, P2), dir (P3, P4, P5) normalised in place, force = the
        // magnitude `NormaliseAndMag` returns (`fstp` f32); `g_fx.AddSparks(&pos, &dir, force, P6, (0, 0, 0), 1,
        // 0.3, 1.0)`.
        0x08EB => {
            collect(s, 7);
            let mut pos = [sp(0), sp(1), sp(2)];
            let mut dir = [sp(3), sp(4), sp(5)];
            let dp = GPtr::from_raw(dir.as_mut_ptr());
            let force = X87::from_f64_exact(normalise_and_mag(dp)).to_f32().to_bits();
            fx_add_sparks(
                GPtr::from_addr(0xA9AE00),
                GPtr::from_raw(pos.as_mut_ptr()),
                dp,
                force,
                sp(6),
                0,
                0,
                0,
                1,
                0x3ECC_CCCD,
                0x3F80_0000,
            );
        }
        // 08A0 TASK_USE_CLOSEST_MAP_ATTRACTOR (0x474C0A): see [`task_use_closest_map_attractor`].
        0x08A0 => return task_use_closest_map_attractor(s, command),
        // 08D4 CREATE_MENU (0x475500)
        0x08D4 => create_menu(s),
        // 08D6 SET_MENU_COLUMN_ORIENTATION (0x4755D0): `SetColumnOrientation((u8)P0, (u8)P1, (u8)P2)`
        // (zero-extended pushes).
        0x08D6 => {
            collect(s, 3);
            set_column_orientation(sp(0) & 0xFF, sp(1) & 0xFF, sp(2) & 0xFF);
        }
        // 08D8 GET_MENU_ITEM_ACCEPTED (0x475632): `P0 = (int8)CheckForAccept(P0)` (movsx); StoreParameters(1).
        0x08D8 => {
            collect(s, 1);
            let r = check_for_accept(sp(0));
            set_sp(0, i32::from(r) as u32);
            store(s, 1);
        }
        // 08D7 GET_MENU_ITEM_SELECTED (0x475602): `SP0 = (int8)CheckForSelected(SP0)` (movsx); `StoreParameters(1)`
        0x08D7 => {
            collect(s, 1);
            let r = check_for_selected(sp(0));
            set_sp(0, i32::from(r) as u32);
            store(s, 1);
        }
        // 08D9 ACTIVATE_MENU_ITEM (0x475661): `ActivateOneItem((u8)SP0, (u8)SP1, (u8)SP2)` (zero-extended pushes)
        0x08D9 => {
            collect(s, 3);
            activate_one_item(sp(0) & 0xFF, sp(1) & 0xFF, sp(2) & 0xFF);
        }
        // 08DA DELETE_MENU (0x475693): `SwitchOffMenu((u8)SP0)`
        0x08DA => {
            collect(s, 1);
            switch_off_menu(sp(0) & 0xFF);
        }
        // 08DB SET_MENU_COLUMN (0x4756B4)
        0x08DB => set_menu_column(s),
        _ => return -1,
    }
    0
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_2200_TO_2299];
