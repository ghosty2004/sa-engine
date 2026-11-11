//! CGangWars, CRoadBlocks, CTrafficLights, CDarkel, car generators (PORT-POP-3, helper B). Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/POP/*.md` as reading aids.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use sa_math::x87::X87;

use crate::common::{g, jne_mask, jp5, model_info, rd, wr, x, xi};

/// Globals owned by this module: none.
pub const GLOBALS: &[GlobalDesc] = &[];

orig! {
    /// `CTheScripts::IsPlayerOnAMission()` (0x464D50)
    fn is_player_on_a_mission() -> u8 = cdecl 0x464D50 => unported;
    /// `CGangWars::EndGangWar(bool)` (0x4464C0; ported unhookable in misc2)
    fn end_gang_war(b: u32) = cdecl 0x4464C0 => unported;
    /// `CGangWars::UpdateTerritoryUnderControlPercentage()` (0x443DE0)
    fn update_territory_under_control_percentage() = cdecl 0x443DE0 => crate::misc2::update_territory_under_control_percentage;
    /// `CWorld::FindPlayerWanted(int32)` (0x56E230)
    fn find_player_wanted(id: i32) -> GPtr<u8> = cdecl 0x56E230 => unported;
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0)
    fn find_player_vehicle(id: i32, remote: u8) -> GPtr<u8> = cdecl 0x56E0D0 => sa_world::world::cworld::find_player_vehicle_port;
    /// `CMatrix::~CMatrix()` (0x59ACD0)
    fn matrix_dtor(m: GPtr<u8>) = thiscall 0x59ACD0 => unported;
    /// `CWorld::FindObjectsKindaColliding(const CVector&, float, bool, int16*, int16, CEntity**, bool x5)` (0x568B80)
    /// `CStats::IncrementStat(uint16, float)` (0x55C180)
    fn increment_stat(id: u32, v: f32) = cdecl 0x55C180 => unported;
    /// `CStats::DecrementStat(uint16, float)` (0x559FA0; float as raw bits)
    fn decrement_stat(id: u16, v: u32) = cdecl 0x559FA0 => sa_world::world::stats::decrement_stat;
    /// `CStats::DisplayScriptStatUpdateMessage(uint8, uint32, float)` (0x55B980)
    fn display_script_stat_update_message(state: u32, stat: u32, v: f32) = cdecl 0x55B980 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => sa_world::world::cworld::find_player_ped_port;
    /// `CPedGroupMembership::IsMember(CPed*)` (0x5F6A10)
    fn membership_is_member(this: GPtr<u8>, ped: GPtr<u8>) -> u32 = thiscall 0x5F6A10 => unported;
    /// `CDarkel::CheckDamagedWeaponType(int32, int32)` (0x43D9E0)
    fn check_damaged_weapon_type(weapon: i32, ty: i32) -> u8 = cdecl 0x43D9E0 => crate::misc2::check_damaged_weapon_type;
    /// `CDarkel::ThisPedShouldBeKilledForFrenzy(const CPed&)` (0x43D2F0)
    fn this_ped_should_be_killed_for_frenzy(ped: GPtr<u8>) -> u8 = cdecl 0x43D2F0 => crate::misc2::this_ped_should_be_killed_for_frenzy;
    /// `CAudioEngine::ReportFrontendAudioEvent(eAudioEvents, float, float)` (0x506EA0, `ret 0xC`)
    fn report_frontend_audio_event(this: GPtr<u8>, event: u32, volume: f32, speed: f32) = thiscall 0x506EA0 => unported;
    /// `CGangWars::AddKillToProvocation(ePedType)` (0x443950)
    fn add_kill_to_provocation(pt: i32) = cdecl 0x443950 => crate::misc2::add_kill_to_provocation;
    /// `CPopCycle::PlayerKilledADealer()` (0x610490)
    fn player_killed_a_dealer() = cdecl 0x610490 => crate::population2::player_killed_a_dealer;
    /// `CPed::SetCharCreatedBy(uint8)` (0x5E47E0, `ret 4`)
    fn ped_set_char_created_by(ped: GPtr<u8>, by: u32) = thiscall 0x5E47E0 => unported;
    fn find_objects_kinda_colliding(pos: GPtr<f32>, r: f32, b2d: u32, cnt: GPtr<i16>, max: i32, out: GPtr<u32>, bld: u32, veh: u32, ped: u32, obj: u32, dum: u32) = cdecl 0x568B80 => unported;
}

/// `m_matrix ? &m_matrix->pos : &placement.pos`.
fn entity_pos(e: GPtr<u8>) -> GPtr<f32> {
    let m = rd::<GPtr<u8>>(e, 0x14);
    if m.is_null() {
        e.byte_add(4).cast()
    } else {
        m.byte_add(0x30).cast()
    }
}

/// `(float)(int16)p[off] * 0.125f` (`fild` + `fmul [0x858C48]`).
fn ncoord(p: GPtr<u8>, off: isize) -> X87 {
    xi(i32::from(rd::<i16>(p, off))).fmul_m(0.125)
}

/// `CModelInfo::ms_modelInfoPtrs[mi]->m_pColModel` (+0x14).
fn col_model(mi: i32) -> GPtr<u8> {
    rd::<GPtr<u8>>(model_info(mi), 0x14)
}

/// `CTimer::m_FrameCounter` (0xB7CB4C).
fn frame_counter() -> u32 {
    g::<u32>(0xB7CB4C).read()
}

/// `CGangWars::Update()` (0x446610), partial port (ADR-002 fallback). PORTED: the prologue and the
/// "gang wars inactive" exit: `EndGangWar(true)` when the player is on a mission, the previous-frame mission
/// flag 0x96ABA0 is clear and `State` (0x96AB94) is 0; `0x96ABA0 = IsPlayerOnAMission()`; return when
/// 0xB5F852 is set; `UpdateTerritoryUnderControlPercentage()` when the frame counter byte 0xB7CB4C is 0x38;
/// return when `bGangWarsActive` (0x96AB90) is 0. NOT PORTED (fallback to the whole original body, decided at
/// entry): `bGangWarsActive != 0` (co-op test, the war timer, the state machine, attack waves). None of the
/// prologue's callees writes 0x96AB90, so the entry test selects the same path the original takes at 0x446662.
#[hook(addr = 0x446610, conv = cdecl, name = "CGangWars::Update", expect = "83 EC 34 53 E8 37 E7 01 00", fallback)]
pub fn gang_wars_update() {
    if g::<u8>(0x96AB90).read() != 0 {
        return gang_wars_update_original();
    }
    if is_player_on_a_mission() != 0 && g::<u8>(0x96ABA0).read() == 0 && g::<u32>(0x96AB94).read() == 0 {
        end_gang_war(1);
    }
    let m = is_player_on_a_mission() != 0;
    g::<u8>(0x96ABA0).write(u8::from(m));
    if g::<u8>(0xB5F852).read() != 0 {
        return;
    }
    if g::<u8>(0xB7CB4C).read() == 0x38 {
        update_territory_under_control_percentage();
    }
}

/// `CRoadBlocks::GenerateRoadBlocks()` (0x4629E0, SEH frame for two stack `CMatrix` locals), partial port
/// (ADR-002 fallback). PORTED: the quiet frame: no wanted level (`FindPlayerWanted(-1)` word +0x1C == 0) or not
/// in a vehicle (`FindPlayerVehicle(-1, false)` null), and the script road block of this frame
/// (`ScriptRoadBlocks[m_FrameCounter & 15]`, 0xA43AB8 stride 0x1C, active byte +0x18) inactive:
/// `bGenerateRoadBlocks`-style byte 0xA43584 = 0 and the two `CMatrix` locals destroyed. NOT PORTED (fallback to
/// the whole original, decided at entry from the same reads, which have no side effects): the wanted-level
/// road blocks and the script road blocks.
#[hook(addr = 0x4629E0, conv = cdecl, name = "CRoadBlocks::GenerateRoadBlocks", expect = "6A FF 68 73 7C 83 00", fallback)]
pub fn generate_road_blocks() {
    let wanted = rd::<u16>(find_player_wanted(-1), 0x1C) != 0 && !find_player_vehicle(-1, 0).is_null();
    let slot = ((frame_counter() & 0xF) * 0x1C) as usize;
    if wanted || g::<u8>(0xA43AD0 + slot).read() != 0 {
        return generate_road_blocks_original();
    }
    // the two stack CMatrix locals (m_pAttachMatrix and m_bOwnsAttachedMatrix cleared by the prologue)
    let mut m1 = [0u32; 0x12];
    let mut m2 = [0u32; 0x12];
    g::<u8>(0xA43584).write(0);
    matrix_dtor(GPtr::from_raw(m1.as_mut_ptr()).cast());
    matrix_dtor(GPtr::from_raw(m2.as_mut_ptr()).cast());
}

/// `CCarGenerator::CheckForBlockage(int32 model)` (0x6F32E0, thiscall, `ret 4`, AL): up to 8 vehicles / peds
/// within the model's bounding radius (2.0 without a col model) of the generator position (2D test) whose
/// vertical extent overlaps the model's (`entZ + ent.max.z + 1 > genZ + min.z` and `entZ + ent.min.z - 1 <
/// genZ + max.z`; ±1.0 around the generator z without a col model) block it: bit 0 of +0xD is set and 1
/// returned. x87 compares as the bytes (`test ah, 0x41; jne` / `test ah, 5; jp`).
#[hook(addr = 0x6F32E0, conv = thiscall, name = "CCarGenerator::CheckForBlockage", expect = "8B 44 24 04 83 EC 38", this = "CCarGenerator")]
pub fn check_for_blockage(this: GPtr<u8>, model: i32) -> u8 {
    let gen_col = col_model(model);
    let radius = if gen_col.is_null() {
        2.0
    } else {
        rd::<f32>(gen_col, 0x24)
    };
    let mut pos = [
        ncoord(this, 4).to_f32(),
        ncoord(this, 6).to_f32(),
        ncoord(this, 8).to_f32(),
    ];
    let mut count = [0i16; 2];
    let mut out = [0u32; 8];
    find_objects_kinda_colliding(
        GPtr::from_raw(pos.as_mut_ptr()),
        radius,
        1,
        GPtr::from_raw(count.as_mut_ptr()),
        8,
        GPtr::from_raw(out.as_mut_ptr()),
        0,
        1,
        1,
        0,
        0,
    );
    for &e in out.iter().take(count[0].max(0) as usize) {
        let e = GPtr::<u8>::from_addr(e as usize);
        let ecol = col_model(i32::from(rd::<i16>(e, 0x22)));
        let z = ncoord(this, 8);
        let ez = || x(entity_pos(e).wrapping_index(2).read());
        let (lo, hi_add) = if gen_col.is_null() {
            (z.fsub_m(1.0), 1.0)
        } else {
            let c = col_model(model);
            (z.fadd_m(rd::<f32>(c, 8)), rd::<f32>(c, 0x14))
        };
        if jne_mask(ez().fadd_m(rd::<f32>(ecol, 0x14)).fadd_m(1.0), lo, 0x41) {
            continue;
        }
        let hi = z.fadd_m(hi_add);
        if !jp5(ez().fadd_m(rd::<f32>(ecol, 8)).fsub_m(1.0), hi) {
            wr::<u8>(this, 0xD, rd::<u8>(this, 0xD) | 1);
            return 1;
        }
    }
    0
}

/// `CDarkel::DrawMessages()` (0x43CEC0), partial port (ADR-002 fallback). PORTED: the exits drawing nothing:
/// byte 0xA43088 == 1 or frenzy `Status` (0x96A704, u16) not in 1..=4.
/// NOT PORTED (fallback to the whole original, decided at entry from the same two reads): statuses 1..4 (the
/// big messages and the timer / kill counter HUD).
#[hook(addr = 0x43CEC0, conv = cdecl, name = "CDarkel::DrawMessages", expect = "A0 88 30 A4 00", fallback)]
pub fn darkel_draw_messages() {
    if g::<u8>(0xA43088).read() == 1 {
        return;
    }
    if (1..=4).contains(&g::<u16>(0x96A704).read()) {
        darkel_draw_messages_original();
    }
}

/// `CDarkel::RegisterKillByPlayer(const CPed* ped, eWeaponType weapon, bool headShot, int32 playerId)` (0x43DCD0):
/// respect / kill stats by ped type 6..17 (jump tables 0x43DF10 / 0x43DF04: cop and dealer +1.5 respect, gang
/// members +0.75 and stat 0xEF, the player's gang (type 8) -5 respect, stat 0xF0 and, when the ped is in the
/// player's group, stat 0xA3 with its update message); during a frenzy (status 1 or 4) a kill with the frenzy
/// weapon (or the paired weapons 0x33, 0x34/0x1C, 0x31/0x32, 0x25/0x12, `CheckDamagedWeaponType` for 0x38/0x39)
/// of a ped that counts lowers `KillsNeeded` (unless 0x969A49 requires head shots) with frontend sound 0x2B,
/// and stat 0xCE (cops) / 0xCD; then the per-model kill counter (0x969A50, u16, index `playerId + model*2`),
/// stat 0x79, the per-type counters (0xB78E90, or 0xB78EE0 for +0x470 bit 10), head shot stat 0x82, stat 0xB0,
/// `AddKillToProvocation` for player 0 unless the player's +0x46C bit 8, and `PlayerKilledADealer` (tail jump)
/// for types >= 17.
#[hook(addr = 0x43DCD0, conv = cdecl, name = "CDarkel::RegisterKillByPlayer", expect = "57 8B 7C 24 08")]
pub fn register_kill_by_player(ped: GPtr<u8>, weapon: i32, head_shot: u8, player_id: i32) {
    let ty = rd::<u32>(ped, 0x598);
    match ty.wrapping_sub(6) {
        0 | 11 => increment_stat(0x44, 1.5),
        2 => {
            decrement_stat(0x44, 5.0f32.to_bits());
            increment_stat(0xF0, 1.0);
            let grp = rd::<i32>(rd::<GPtr<u8>>(find_player_ped(-1), 0x480), 0x38);
            let membership = g::<u8>((0xC09928 + grp * 0x2D4) as usize);
            if membership_is_member(membership, ped) & 0xFF != 0 {
                increment_stat(0xA3, 1.0);
                display_script_stat_update_message(0, 0x150, 1.0);
            }
        }
        1 | 3..=10 => {
            increment_stat(0x44, 0.75);
            increment_stat(0xEF, 1.0);
        }
        _ => {}
    }
    let status = g::<u16>(0x96A704).read();
    if status == 1 || status == 4 {
        let fw = g::<i32>(0x96A700).read();
        let counts = ((fw == 0x38 || fw == 0x39) && check_damaged_weapon_type(weapon, fw) != 0)
            || weapon == fw
            || weapon == 0x33
            || (weapon == 0x34 && fw == 0x1C)
            || (weapon == 0x31 && fw == 0x32)
            || (weapon == 0x32 && fw == 0x31)
            || (weapon == 0x25 && fw == 0x12);
        if counts {
            if this_ped_should_be_killed_for_frenzy(ped) != 0
                && (g::<u8>(0x969A49).read() == 0 || head_shot != 0)
            {
                let k = g::<i32>(0x96A6EC);
                k.write(k.read().wrapping_sub(1));
                report_frontend_audio_event(g(0xB6BC90), 0x2B, 0.0, 1.0);
            }
            increment_stat(if rd::<u32>(ped, 0x598) == 6 { 0xCE } else { 0xCD }, 1.0);
        }
    }
    let model = i32::from(rd::<i16>(ped, 0x22));
    let kc = g::<u16>((0x969A50 + (player_id + model * 2) * 2) as usize);
    kc.write(kc.read().wrapping_add(1));
    increment_stat(0x79, 1.0);
    let c = if rd::<u32>(ped, 0x470) & 0x400 != 0 {
        g::<u32>(0xB78EE0)
    } else {
        g::<u32>((0xB78E90 + rd::<u32>(ped, 0x598) * 4) as usize)
    };
    c.write(c.read().wrapping_add(1));
    if head_shot != 0 {
        increment_stat(0x82, 1.0);
    }
    increment_stat(0xB0, 1.0);
    if player_id == 0 && rd::<u32>(g::<GPtr<u8>>(0xB7CD98).read(), 0x46C) & 0x100 == 0 {
        add_kill_to_provocation(rd::<i32>(ped, 0x598));
    }
    if rd::<i32>(ped, 0x598) >= 0x11 {
        player_killed_a_dealer();
    }
}

orig! {
    /// `CWorld::FindPlayerCoors(CVector* out, int32)` (0x56E010)
    fn find_player_coors_out(out: GPtr<f32>, id: i32) -> GPtr<f32> = cdecl 0x56E010 => unported;
    /// `CTheZones::GetZoneInfo(const CVector&, CZone**)` (0x572400)
    fn get_zone_info(p: GPtr<f32>, out: GPtr<GPtr<u8>>) -> GPtr<u8> = cdecl 0x572400 => sa_world::world::zones::get_zone_info;
    /// `CLoadedCarGroup::PickRandomCar(bool, bool)` (0x611C50, `ret 8`)
    fn pick_random_car(this: GPtr<u8>, a: u32, b: u32) -> i32 = thiscall 0x611C50 => unported;
    /// `CCarGenerator::CheckForBlockage(int32)` (0x6F32E0)
    fn check_for_blockage_call(this: GPtr<u8>, model: i32) -> u8 = thiscall 0x6F32E0 => check_for_blockage;
    /// `CStreaming::RequestModel(int32, int32)` (0x4087E0)
    fn request_model(m: i32, flags: i32) = cdecl 0x4087E0 => sa_stream::streaming::request_model;
    /// `CModelInfo::IsBoatModel(int32)` (0x4C5A70)
    fn is_boat_model(id: i32) -> bool = cdecl 0x4C5A70 => sa_stream::modelinfo::model_info::is_boat_model;
    /// `CWorld::ProcessVerticalLine(const CVector&, float, CColPoint&, CEntity*&, bool x6, CStoredCollPoly*)` (0x5674E0)
    fn process_vertical_line(o: GPtr<f32>, z: f32, cp: GPtr<u8>, ent: GPtr<u32>, b: u32, v: u32, p: u32, ob: u32, d: u32, see: u32, poly: u32) -> u8 = cdecl 0x5674E0 => unported;
    /// `CSpecialPlateHandler::Find(int32 index, char* out)` (0x6F2D30, `ret 8`)
    fn special_plate_find(this: GPtr<u8>, index: u32, out: GPtr<u8>) -> i32 = thiscall 0x6F2D30 => unported;
    /// `CVehicleModelInfo::SetCustomCarPlateText(char*)` (0x4C8980, `ret 4`)
    fn set_custom_car_plate_text(mi: GPtr<u8>, text: GPtr<u8>) = thiscall 0x4C8980 => unported;
    /// `CVehicle::operator new(uint32)` (0x6E2D50)
    fn vehicle_new(size: u32) -> GPtr<u8> = cdecl 0x6E2D50 => unported;
    /// vehicle constructors `(int32 model, uint8 createdBy)` (`ret 8`)
    fn monster_truck_ctor(this: GPtr<u8>, mi: i32, by: u32) -> GPtr<u8> = thiscall 0x6C8D60 => unported;
    fn quad_bike_ctor(this: GPtr<u8>, mi: i32, by: u32) -> GPtr<u8> = thiscall 0x6CE370 => unported;
    fn heli_ctor(this: GPtr<u8>, mi: i32, by: u32) -> GPtr<u8> = thiscall 0x6C4190 => unported;
    fn plane_ctor(this: GPtr<u8>, mi: i32, by: u32) -> GPtr<u8> = thiscall 0x6C8E20 => unported;
    fn boat_ctor(this: GPtr<u8>, mi: i32, by: u32) -> GPtr<u8> = thiscall 0x6F2940 => unported;
    fn train_ctor(this: GPtr<u8>, mi: i32, by: u32) -> GPtr<u8> = thiscall 0x6F6030 => unported;
    fn bike_ctor(this: GPtr<u8>, mi: i32, by: u32) -> GPtr<u8> = thiscall 0x6BF430 => unported;
    fn bmx_ctor(this: GPtr<u8>, mi: i32, by: u32) -> GPtr<u8> = thiscall 0x6BF820 => unported;
    fn trailer_ctor(this: GPtr<u8>, mi: i32, by: u32) -> GPtr<u8> = thiscall 0x6D03A0 => unported;
    /// `CAutomobile::CAutomobile(int32, uint8, bool)` (0x6B0A90, `ret 0xC`)
    fn automobile_ctor(this: GPtr<u8>, mi: i32, by: u32, setup: u32) -> GPtr<u8> = thiscall 0x6B0A90 => unported;
    /// `CEntity::GetDistanceFromCentreOfMassToBaseOfModel()` (0x536BE0, ST0)
    fn get_distance_from_centre_of_mass_to_base_of_model(e: GPtr<u8>) -> f64 = thiscall 0x536BE0 => unported;
    /// `CVehicle::SetCollisionLighting(tColLighting)` (0x6D0CA0, `ret 4`)
    fn vehicle_set_collision_lighting(v: GPtr<u8>, l: u32) = thiscall 0x6D0CA0 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660, ST0)
    fn find_ground_z_for_coord(x: f32, y: f32) -> f64 = cdecl 0x569660 => unported;
    /// `CEntity` vtable slot 0x10 with one argument (0)
    fn entity_vslot_10(e: GPtr<u8>, a: u32) = thiscall vslot 0x10 => unported;
    /// `CPlaceable::SetPosn(float, float, float)` (0x420B80, `ret 0xC`)
    fn placeable_set_posn(e: GPtr<u8>, x: f32, y: f32, z: f32) = thiscall 0x420B80 => unported;
    /// `CPlaceable::SetOrientation(float, float, float)` (0x439A80, `ret 0xC`)
    fn placeable_set_orientation(e: GPtr<u8>, xr: f32, yr: f32, zr: f32) = thiscall 0x439A80 => unported;
    /// `CPathFind::FindNodeClosestToCoors(CNodeAddress*, CVector, ...)` (0x44F460, `ret 0x2C`)
    fn find_node_closest_to_coors(this: GPtr<u8>, out: GPtr<u32>, px: f32, py: f32, pz: f32, node_type: u32, max_dist: f32, low_traffic: u32, unused: u32, unkn: u32, boats: u32, ignore_interior: u32) -> GPtr<u32> = thiscall 0x44F460 => crate::pathfind::find_node_closest_to_coors;
    /// `CVehicle::SetVehicleCreatedBy(int32)` (0x6D5D70, `ret 4`)
    fn vehicle_set_created_by(veh: GPtr<u8>, by: i32) = thiscall 0x6D5D70 => unported;
    /// `CWorld::Add(CEntity*)` (0x563220)
    fn world_add(e: GPtr<u8>) = cdecl 0x563220 => unported;
    /// `CCarCtrl::SetUpDriverAndPassengersForVehicle(CVehicle*, int32, int32, bool, bool, int32)` (0x4217C0)
    fn set_up_driver_and_passengers_for_vehicle(veh: GPtr<u8>, pt: i32, min: i32, a: u32, b: u32, max: i32) = cdecl 0x4217C0 => crate::carctrl2::set_up_driver_and_passengers_for_vehicle;
    /// `CGeneral::GetRandomNumberInRange(int32, int32)` (0x407180)
    fn random_int_in_range(min: i32, max: i32) -> i32 = cdecl 0x407180 => unported;
    /// `CVisibilityPlugins::SetClumpAlpha(RpClump*, int32)` (0x732B00)
    fn set_clump_alpha(clump: GPtr<u8>, alpha: i32) = cdecl 0x732B00 => unported;
}

/// `CStreaming::ms_aInfoForModel[mi].m_nLoadState == LOADSTATE_LOADED` (0x8E4CC0 + 0x10, stride 0x14).
fn model_loaded(mi: i32) -> bool {
    g::<u8>((0x8E4CD0 + mi * 20) as usize).read() == 1
}

/// `CCarGenerator::DoInternalProcessing()` (0x6F34D0, thiscall, SEH frame), partial port (ADR-002 fallback):
/// the hotdog van generators (`m_nModelId` == 0x24C: the vendor ped of model 0xA8 is created through
/// `AddPed(4, 0xA8, pos)` whose fourth argument is the caller's saved EDI, which the port cannot reproduce) run the
/// whole original. PORTED: everything else. Parked car limit `NumParkedCars` (0x9690A0) 10 at night (hour > 21
/// or < 7), else 5, unless the generator ignores it (+0xD bit 4); a negative model is a random-car generator
/// (`-model` if loaded, else `m_LoadedAppropriateCars.PickRandomCar(true, true)` after `GetZoneInfo` of the
/// player position, rejecting model-info +0x4D == 5 and col-model lengths > 8.0, stored back as `-model` with
/// colours 0xFF); `CheckForBlockage` pushes the next try 4 ms later; models are requested (priority 8) and
/// must be loaded. Land vehicles: `ProcessVerticalLine` from 1.0 above the generator z (1000.0 when z <= -100)
/// down to -1000.0, special plate text, the constructor by vehicle type with created-by 3 (bikes / BMXs set
/// +0x614 bit 4; a failed allocation writes it through null as the original), the result of
/// `GetDistanceFromCentreOfMassToBaseOfModel` discarded, +0x4A8 &= 0xE7, +0x428 &= 0xBF and
/// `SetCollisionLighting` with the dword at colpoint +0x25. Boats / helis / planes (IsBoatModel or models 0x1CC,
/// 0x1BF, 0x1A1): constructor by type 3 / 4 / 5 (other types take a stale stack slot in the original: null
/// here, unreachable with the shipped models), `FindGroundZForCoord` above -100, +0x4A6 = 0xFF. Then vtable
/// slot 0x10 (0), +0x428 &= 0xEF, position (ground + base distance), heading `+0xA * 2pi/256`, status 4,
/// unlocked, +0x42A bit 1 = generator +0xD bit 3; by day tractors / combines (0x213 / 0x214) near a road node
/// (`FindNodeClosestToCoors` 20.0) become driving cars with a farmer (ped type 4); `CWorld::Add`; alarm and
/// door-lock chances (`GetRandomNumberInRange(0, 100)` x2); colours copied or saved back; clump alpha 0; the
/// vehicle handle; the generate count is never decremented (signed compare with -1, original bug); next
/// time = now + 4.
#[hook(addr = 0x6F34D0, conv = thiscall, name = "CCarGenerator::DoInternalProcessing", expect = "6A FF 68 14 8A 84 00", this = "CCarGenerator", fallback)]
pub fn do_internal_processing(this: GPtr<u8>) {
    if rd::<i16>(this, 0) == 0x24C {
        return do_internal_processing_original(this);
    }
    let hours = g::<u8>(0xB70153).read();
    let night = !(7..=0x15).contains(&hours);
    if rd::<u8>(this, 0xD) & 0x10 == 0 && g::<i32>(0x9690A0).read() >= if night { 10 } else { 5 } {
        return;
    }
    let next_try = |this: GPtr<u8>| wr::<u32>(this, 0x14, rd::<u32>(this, 0x14).wrapping_add(4));
    let gm = rd::<i16>(this, 0);
    let model: i32;
    if gm < 0 {
        if gm != -1 && model_loaded(-i32::from(gm)) {
            model = -i32::from(gm);
        } else {
            let mut pc = [0f32; 3];
            let pcp = GPtr::from_raw(pc.as_mut_ptr());
            find_player_coors_out(pcp, -1);
            get_zone_info(pcp, GPtr::NULL);
            model = pick_random_car(g(0xC0E9F8), 1, 1);
            if model < 0 {
                return;
            }
            let mi = model_info(model);
            if rd::<u8>(mi, 0x4D) == 5 {
                return;
            }
            let col = rd::<GPtr<u8>>(mi, 0x14);
            if !col.is_null() && !jne_mask(x(rd::<f32>(col, 0x10)).fsub_m(rd::<f32>(col, 4)), x(8.0), 0x41) {
                return;
            }
            wr::<i16>(this, 0, -(model as i16));
            wr::<u8>(this, 2, 0xFF);
            wr::<u8>(this, 3, 0xFF);
        }
        if check_for_blockage_call(this, model) != 0 {
            next_try(this);
            return;
        }
    } else {
        if check_for_blockage_call(this, i32::from(gm)) != 0 {
            next_try(this);
            return;
        }
        request_model(i32::from(rd::<i16>(this, 0)), 8);
        model = i32::from(rd::<i16>(this, 0));
        if model == 0x24C {
            request_model(0xA8, 8);
        }
    }
    if !model_loaded(model) {
        return;
    }
    // (the 0x24C / 0xA8 load test only matters for the hotdog generators, which took the fallback)
    let mi = model_info(model);
    let veh: GPtr<u8>;
    let mut ground: f32;
    if is_boat_model(model) || model == 0x1CC || model == 0x1BF || model == 0x1A1 {
        let v = match rd::<u32>(mi, 0x3C) {
            3 => (0xA18, heli_ctor as fn(GPtr<u8>, i32, u32) -> GPtr<u8>),
            4 => (0xA04, plane_ctor as fn(GPtr<u8>, i32, u32) -> GPtr<u8>),
            5 => (0x7E8, boat_ctor as fn(GPtr<u8>, i32, u32) -> GPtr<u8>),
            _ => (0, heli_ctor as fn(GPtr<u8>, i32, u32) -> GPtr<u8>),
        };
        veh = if v.0 == 0 {
            GPtr::NULL
        } else {
            let m = vehicle_new(v.0);
            if m.is_null() { m } else { (v.1)(m, model, 3) }
        };
        let z = xi(i32::from(rd::<i16>(this, 8))).fmul_m(0.125);
        ground = z.to_f32();
        if !z.fcom(x(-100.0)).test_parity(0x41) {
            let gy = xi(i32::from(rd::<i16>(this, 6))).fmul_m(0.125).to_f32();
            let gx = xi(i32::from(rd::<i16>(this, 4))).fmul_m(0.125).to_f32();
            ground = X87::from_f64_exact(find_ground_z_for_coord(gx, gy)).to_f32();
        }
        wr::<u16>(veh, 0x4A6, 0xFF);
    } else {
        let z = xi(i32::from(rd::<i16>(this, 8))).fmul_m(0.125);
        let px = xi(i32::from(rd::<i16>(this, 4))).fmul_m(0.125).to_f32();
        let py = xi(i32::from(rd::<i16>(this, 6))).fmul_m(0.125).to_f32();
        let top = if z.fcom(x(-100.0)).test_parity(0x41) {
            z.fadd_m(1.0).to_f32()
        } else {
            1000.0
        };
        let mut pos = [px, py, top];
        let mut cp = [0u32; 12];
        let mut ent = [0u32; 1];
        let cpp = GPtr::<u8>::from_raw(cp.as_mut_ptr().cast());
        if process_vertical_line(
            GPtr::from_raw(pos.as_mut_ptr()),
            -1000.0,
            cpp,
            GPtr::from_raw(ent.as_mut_ptr()),
            1,
            0,
            0,
            0,
            0,
            0,
            0,
        ) == 0
        {
            return;
        }
        ground = rd::<f32>(cpp, 8);
        let mut plate = [0u8; 16];
        let pp = GPtr::<u8>::from_raw(plate.as_mut_ptr());
        let index = (this.addr() as u32).wrapping_sub(0xC27AD0) >> 5;
        special_plate_find(g(0xC279D8), index, pp);
        if plate[0] != 0 && rd::<u32>(model_info(model), 0x24) != 0 {
            set_custom_car_plate_text(model_info(model), pp);
        }
        type Ctor = fn(GPtr<u8>, i32, u32) -> GPtr<u8>;
        let (size, ctor, bike): (u32, Option<Ctor>, bool) = match rd::<u32>(model_info(model), 0x3C) {
            1 => (0x99C, Some(monster_truck_ctor), false),
            2 => (0x9BC, Some(quad_bike_ctor), false),
            3 => (0xA18, Some(heli_ctor), false),
            4 => (0xA04, Some(plane_ctor), false),
            6 => (0x6AC, Some(train_ctor), false),
            9 => (0x814, Some(bike_ctor), true),
            10 => (0x838, Some(bmx_ctor), true),
            11 => (0x9F4, Some(trailer_ctor), false),
            _ => (0x988, None, false),
        };
        let m = vehicle_new(size);
        veh = if m.is_null() {
            if bike {
                let p = GPtr::<u8>::from_addr(0x614);
                p.write(p.read() | 0x10);
            }
            m
        } else {
            match ctor {
                Some(c) => {
                    let v = c(m, model, 3);
                    if bike {
                        wr::<u8>(v, 0x614, rd::<u8>(v, 0x614) | 0x10);
                    }
                    v
                }
                None => automobile_ctor(m, model, 3, 1),
            }
        };
        get_distance_from_centre_of_mass_to_base_of_model(veh);
        wr::<u8>(veh, 0x4A8, rd::<u8>(veh, 0x4A8) & 0xE7);
        wr::<u8>(veh, 0x428, rd::<u8>(veh, 0x428) & 0xBF);
        vehicle_set_collision_lighting(veh, rd::<u32>(cpp, 0x25));
    }
    // 0x6F3ADA
    entity_vslot_10(veh, 0);
    wr::<u8>(veh, 0x428, rd::<u8>(veh, 0x428) & 0xEF);
    let z = X87::from_f64_exact(get_distance_from_centre_of_mass_to_base_of_model(veh))
        .fadd_m(ground)
        .to_f32();
    let py = xi(i32::from(rd::<i16>(this, 6))).fmul_m(0.125).to_f32();
    let px = xi(i32::from(rd::<i16>(this, 4))).fmul_m(0.125).to_f32();
    placeable_set_posn(veh, px, py, z);
    let heading = xi(i32::from(rd::<u8>(this, 0xA)))
        .fmul_m(f32::from_bits(0x3CC9_0FDB))
        .to_f32();
    placeable_set_orientation(veh, 0.0, 0.0, heading);
    wr::<u8>(veh, 0x36, (rd::<u8>(veh, 0x36) & 7) | 0x20);
    wr::<u32>(veh, 0x4F8, 1);
    let old = rd::<u8>(veh, 0x42A);
    wr::<u8>(veh, 0x42A, old ^ (((rd::<u8>(this, 0xD) >> 2) ^ old) & 2));
    let mut driver = -1i32;
    let vm = rd::<u16>(veh, 0x22);
    if !night && (vm == 0x213 || vm == 0x214) {
        let p = entity_pos(veh);
        let mut node = [0u32; 1];
        let np = GPtr::from_raw(node.as_mut_ptr());
        find_node_closest_to_coors(
            g(0x96F050),
            np,
            p.read(),
            p.wrapping_index(1).read(),
            p.wrapping_index(2).read(),
            0,
            20.0,
            0,
            0,
            0,
            0,
            1,
        );
        let a = node[0];
        if a & 0xFFFF != 0xFFFF {
            let nodes = g::<GPtr<u8>>((0x96F854 + (a & 0xFFFF) * 4) as usize).read();
            let n = nodes.byte_add(((a >> 16) * 0x1C) as isize);
            if rd::<u8>(n, 0x18) & 0xF != 0 {
                let links = g::<GPtr<u32>>((0x96FA94 + (a & 0xFFFF) * 4) as usize).read();
                let l0 = links.wrapping_index(i32::from(rd::<i16>(n, 0x10))).read();
                if !g::<GPtr<u8>>((0x96F854 + (l0 & 0xFFFF) * 4) as usize)
                    .read()
                    .is_null()
                {
                    vehicle_set_created_by(veh, 1);
                    wr::<u32>(veh, 0x390, node[0]);
                    wr::<u8>(veh, 0x3D0, 7);
                    wr::<u8>(veh, 0x3BA, 1);
                    wr::<u8>(veh, 0x36, (rd::<u8>(veh, 0x36) & 7) | 0x18);
                    wr::<u32>(veh, 0x394, l0);
                    wr::<u8>(veh, 0x42C, rd::<u8>(veh, 0x42C) | 0x40);
                    wr::<u16>(veh, 0x4A6, 0xFF);
                    wr::<u8>(this, 0xD, rd::<u8>(this, 0xD) | 1);
                    driver = 4;
                }
            }
        }
    }
    world_add(veh);
    if driver != -1 {
        set_up_driver_and_passengers_for_vehicle(veh, driver, 0, 0, 0, 0x63);
    }
    if random_int_in_range(0, 100) < i32::from(rd::<u8>(this, 0xB)) {
        wr::<u16>(veh, 0x45C, 0xFFFF);
    }
    if random_int_in_range(0, 100) < i32::from(rd::<u8>(this, 0xC)) {
        wr::<u32>(veh, 0x4F8, 2);
    }
    let c1 = rd::<u8>(this, 2);
    if c1 != 0xFF && rd::<u8>(this, 3) != 0xFF {
        wr::<u8>(veh, 0x434, c1);
        wr::<u8>(veh, 0x435, rd::<u8>(this, 3));
    } else if rd::<i16>(this, 0) < -1 {
        wr::<u8>(this, 2, rd::<u8>(veh, 0x434));
        wr::<u8>(this, 3, rd::<u8>(veh, 0x435));
    }
    set_clump_alpha(rd::<GPtr<u8>>(veh, 0x18), 0);
    let pool = g::<GPtr<u8>>(0xB74494).read();
    let index = ((veh.addr() as i32).wrapping_sub(rd::<i32>(pool, 0))) / 0xA18;
    let flag = rd::<GPtr<u8>>(pool, 4).wrapping_index(index).read();
    wr::<u16>(this, 0x18, ((index << 8) as u16).wrapping_add(u16::from(flag)));
    // `movzx ecx, m_nGenerateCount; cmp ecx, -1; jge`: never decremented
    wr::<u32>(this, 0x14, g::<u32>(0xB7CB84).read().wrapping_add(4));
}

/// `CGangWars::StrengthenPlayerInfluenceInZone(int32 amount)` (stub 0x445F50 -> 0x1565090): on the zone being
/// fought over (`*(CZoneInfo**)0x96AB8C`), the Grove strength byte (+1) grows by `amount` up to 0x37 (only when
/// below 0x37); when Grove was not already strongest (non-zero and >= +0 + +2) but now is, stat 0xEA += 1.0. The
/// float is pushed through an obfuscated pair: `[0x15604FC] - [0x665271]` = 0x1950B2E9 - 0xD9D0B2E9 = 0x3F800000
/// (decoded from the static image).
#[hook(addr = 0x445F50, conv = cdecl, name = "CGangWars::StrengthenPlayerInfluenceInZone", expect = "E9 3B F1 11 01")]
pub fn strengthen_player_influence_in_zone(amount: i32) {
    let zi = g::<GPtr<u8>>(0x96AB8C).read();
    let strongest = |zi: GPtr<u8>| {
        let v = i32::from(rd::<u8>(zi, 1));
        v != 0 && v >= i32::from(rd::<u8>(zi, 2)) + i32::from(rd::<u8>(zi, 0))
    };
    let was = strongest(zi);
    let cur = rd::<u8>(zi, 1);
    let zi = if cur < 0x37 {
        let v = (i32::from(cur) + amount).min(0x37);
        wr::<u8>(zi, 1, v as u8);
        g::<GPtr<u8>>(0x96AB8C).read()
    } else {
        zi
    };
    if !was && strongest(zi) {
        increment_stat(0xEA, f32::from_bits(0x3F80_0000));
    }
}

/// `CGangWars::AttackWaveOvercome()` (0x445B30, AL): walks the ped pool downwards; peds of the attack wave
/// (+0x474 bit 12) that are dead or dying (+0x530 == 0x37 / 0x36) leave the wave (bit cleared,
/// `SetCharCreatedBy(1)`); the others count, and those within 45.0 (2D) of the player count as near. Overcome
/// when at most one is left and none is near (also when the pool is empty). The pool pointer is read through an
/// obfuscated pointer: `rol([0x15606E8], [0x4B5588] & 31)` = `rol(0x48005BA2, 9)` = 0xB74490 (CPools::ms_pPedPool,
/// decoded from the static image; the key byte 0x4B5588 is code).
#[hook(addr = 0x445B30, conv = cdecl, name = "CGangWars::AttackWaveOvercome", expect = "83 EC 14 53 90")]
pub fn attack_wave_overcome() -> u8 {
    let pool = g::<GPtr<u8>>(0xB74490).read();
    let n = rd::<i32>(pool, 8);
    if n == 0 {
        return 1;
    }
    let (mut count, mut near) = (0i32, 0i32);
    for i in (0..n).rev() {
        if rd::<GPtr<u8>>(pool, 4).wrapping_index(i).read() & 0x80 != 0 {
            continue;
        }
        let ped = rd::<GPtr<u8>>(pool, 0).byte_add(i as isize * 0x7C4);
        if ped.is_null() {
            continue;
        }
        let flags = rd::<u32>(ped, 0x474);
        if flags & 0x1000 == 0 {
            continue;
        }
        let state = rd::<u32>(ped, 0x530);
        if state == 0x37 || state == 0x36 {
            wr::<u32>(ped, 0x474, flags & 0xFFFF_EFFF);
            ped_set_char_created_by(ped, 1);
            continue;
        }
        count += 1;
        let p = entity_pos(ped);
        let mut pc = [0f32; 3];
        let c = find_player_coors_out(GPtr::from_raw(pc.as_mut_ptr()), -1);
        let dx = x(p.read()).fsub_m(c.read());
        let dy = x(p.wrapping_index(1).read()).fsub_m(c.wrapping_index(1).read());
        let d = dx.fmul(dx).fadd(dy.fmul(dy)).fsqrt();
        if !jp5(d, x(45.0)) {
            near += 1;
        }
    }
    u8::from(count <= 1 && near == 0)
}

pub const HOOKS: &[HookReg] = &[
    HOOK_GANG_WARS_UPDATE,
    HOOK_GENERATE_ROAD_BLOCKS,
    HOOK_CHECK_FOR_BLOCKAGE,
    HOOK_DARKEL_DRAW_MESSAGES,
    HOOK_REGISTER_KILL_BY_PLAYER,
    HOOK_DO_INTERNAL_PROCESSING,
    HOOK_STRENGTHEN_PLAYER_INFLUENCE_IN_ZONE,
    HOOK_ATTACK_WAVE_OVERCOME,
];
