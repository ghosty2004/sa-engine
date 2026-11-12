//! WRITER-MVP-D (helper-world) WORLD ports, batch `mvpd-world-2`: `CGameLogic` skip / update logic and other
//! per-frame WORLD functions. Ported from the 1.0 US bytes (the exe wins over gta-reversed).
#![allow(clippy::unreadable_literal)]

use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::X87;

use super::common::{ftol32, g, rd, wr, x};

orig! {
    /// `CWorld::FindPlayerVehicle(int32, bool)` (0x56E0D0, cdecl)
    fn find_player_vehicle(id: i32, remote: u32) -> GPtr<u8> = cdecl 0x56E0D0 => unported;
    /// `CWorld::FindPlayerPed(int32)` (0x56E210, cdecl)
    fn find_player_ped(id: i32) -> GPtr<u8> = cdecl 0x56E210 => unported;
    /// `FindPlayerCoors(CVector* out, int32 playerId)` (0x56E010, cdecl); returns `out`
    fn find_player_coors(out: GPtr<u8>, id: i32) -> GPtr<u8> = cdecl 0x56E010 => unported;
    /// `CPedIntelligence::GetTaskSwim()` (0x601070)
    fn ped_intelligence_get_task_swim(i: GPtr<u8>) -> GPtr<u8> = thiscall 0x601070 => unported;
    /// `CGameLogic::SkipCanBeActivated()` (0x4415C0, cdecl)
    fn skip_can_be_activated_o() -> u8 = cdecl 0x4415C0 => crate::world::mvpd_w2::game_logic_skip_can_be_activated;
    /// `CPad::GetPad(int32)` (0x53FB70, cdecl)
    fn pad_get_pad(id: i32) -> GPtr<u8> = cdecl 0x53FB70 => unported;
    /// `CTaskManager::FindActiveTaskByType(int32)` (0x681740, `ret 4`)
    fn task_manager_find_active_task_by_type(tm: GPtr<u8>, t: i32) -> GPtr<u8> = thiscall 0x681740 => unported;
    /// `CCamera::SetFadeColour(uint8, uint8, uint8)` (0x50BF00, `ret 0xC`)
    fn camera_set_fade_colour(c: GPtr<u8>, r: u32, g: u32, b: u32) = thiscall 0x50BF00 => unported;
    /// `CCamera::Fade(float, int16)` (0x50AC20, `ret 8`)
    fn camera_fade(c: GPtr<u8>, t: f32, dir: u32) = thiscall 0x50AC20 => unported;
    /// `CText::Get(const char*)` (0x6A0050, `ret 4`)
    fn text_get(t: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CMessages::AddBigMessage(const char*, uint32 time, eMessageStyle)` (0x69F2B0, cdecl)
    fn messages_add_big_message(s: GPtr<u8>, time: u32, style: u32) = cdecl 0x69F2B0 => unported;
    /// `CTimer::Suspend()` (0x5619D0, cdecl)
    fn timer_suspend() = cdecl 0x5619D0 => unported;
    /// `CTimer::Resume()` (0x561A00, cdecl)
    fn timer_resume() = cdecl 0x561A00 => unported;
    /// `CStreaming::LoadSceneCollision(const CVector&)` (0x40ED80, cdecl)
    fn streaming_load_scene_collision(p: GPtr<u8>) = cdecl 0x40ED80 => unported;
    /// `CStreaming::LoadScene(const CVector&)` (0x40EB70, cdecl)
    fn streaming_load_scene(p: GPtr<u8>) = cdecl 0x40EB70 => unported;
    /// `CTheScripts::ClearSpaceForMissionEntity(const CVector&, CEntity*)` (0x486B00, cdecl)
    fn clear_space_for_mission_entity(p: GPtr<u8>, e: GPtr<u8>) = cdecl 0x486B00 => unported;
    /// entity virtual +0x38 (`Teleport(CVector, bool)`)
    fn v_teleport(e: GPtr<u8>, x: f32, y: f32, z: f32, b: u32) = thiscall vslot 0x38 => unported;
    /// `CPlaceable::SetHeading(float)` (0x43E0C0, `ret 4`)
    fn placeable_set_heading(e: GPtr<u8>, h: f32) = thiscall 0x43E0C0 => unported;
    /// `CPlaceable::GetHeading()` (0x441DB0, float in ST0)
    fn placeable_get_heading(e: GPtr<u8>) -> f64 = thiscall 0x441DB0 => unported;
    /// `CBike::PlaceOnRoadProperly()` (0x6BEEB0)
    fn bike_place_on_road_properly(v: GPtr<u8>) = thiscall 0x6BEEB0 => unported;
    /// `CAutomobile::PlaceOnRoadProperly()` (0x6AF420)
    fn automobile_place_on_road_properly(v: GPtr<u8>) = thiscall 0x6AF420 => unported;
    /// `operator*(const CVector&, float)` (0x40FEC0, cdecl, hidden result pointer); returns `out`
    fn vector_mul(out: GPtr<u8>, v: GPtr<u8>, k: f32) -> GPtr<u8> = cdecl 0x40FEC0 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660, cdecl, float in ST0)
    fn find_ground_z_for_coord(x: f32, y: f32) -> f64 = cdecl 0x569660 => unported;
    /// `CHud::SetMessage(const char*)` (0x588F60, cdecl)
    fn hud_set_message(s: GPtr<u8>) = cdecl 0x588F60 => unported;
    /// `CGameLogic::PassTime(uint32 minutes)` (0x4414C0, cdecl)
    fn game_logic_pass_time(m: u32) = cdecl 0x4414C0 => unported;
}

/// `CGameLogic::SkipState` (0x96A8A0): 0 none, 1 available, 2 fading out, 3 fading in, 4 available (swim), 5
/// waiting for the script
const SKIP_STATE: usize = 0x96A8A0;
/// `CGameLogic::SkipTimer` (0x96A89C)
const SKIP_TIMER: usize = 0x96A89C;
/// `CGameLogic::SkipPosition` (0x96A8D4)
const SKIP_POSITION: usize = 0x96A8D4;
/// `CTimer::m_snTimeInMilliseconds` (0xB7CB84)
const TIME_MS: usize = 0xB7CB84;
/// `TheCamera` (0xB6F028)
const THE_CAMERA: usize = 0xB6F028;

/// `CGameLogic::SkipCanBeActivated()` (0x4415C0 -> .HOODLUM 0x1564860, cdecl, bool in AL): false while
/// 0xB72914 or `TheCamera` +0x51 (0xB6F079) is set or the skip state is not 1/4. In a car, bike, monster truck
/// or quad (type 0, 1, 2, 9) or a BMX (10) the skip is allowed when no skip vehicle (0x96A898) is set or it is
/// the player's vehicle; otherwise (no vehicle, another type or vehicle) only in state 4 while swimming.
/// The BMX check's `FindPlayerVehicle` is called through the protection's computed-return trick (target
/// `0x2B70680 ror 3` = 0x56E0D0); its second argument is the protection routine's leftover EAX, passed here
/// as `false` (only a set remote vehicle could tell the difference).
#[hook(addr = 0x4415C0, conv = cdecl, name = "CGameLogic::SkipCanBeActivated", expect = "E9 9B 32 12 01",
       unverified = "mvpd-world-2", route = "L2", hazards = "none",
       note = "BMX FindPlayerVehicle(-1, leftover EAX of the protection call) modelled with false")]
pub fn game_logic_skip_can_be_activated() -> u8 {
    if g::<u32>(0xB72914).read() != 0 || g::<u8>(0xB6F079).read() != 0 {
        return 0;
    }
    let st = g::<u32>(SKIP_STATE).read();
    if st != 1 && st != 4 {
        return 0;
    }
    let mut vehicle_ok = false;
    if !find_player_vehicle(-1, 0).is_null() {
        vehicle_ok = rd::<u32>(find_player_vehicle(-1, 0), 0x594) == 9
            || rd::<u32>(find_player_vehicle(-1, 0), 0x594) == 0
            || rd::<u32>(find_player_vehicle(-1, 0), 0x594) == 1
            || rd::<u32>(find_player_vehicle(-1, 0), 0x594) == 2
            || rd::<u32>(find_player_vehicle(-1, 0), 0x594) == 10;
    }
    if vehicle_ok {
        let sv = g::<GPtr<u8>>(0x96A898).read();
        if sv.is_null() || sv == find_player_vehicle(-1, 0) {
            return 1;
        }
    }
    if g::<u32>(SKIP_STATE).read() != 4 {
        return 0;
    }
    let ped = find_player_ped(-1);
    (!ped_intelligence_get_task_swim(rd::<GPtr<u8>>(ped, 0x47C)).is_null()) as u8
}

/// `CGameLogic::UpdateSkip()` (0x442480, cdecl).
///
/// In state 1 the skip is cancelled (state 0) when the player is within 25 units (2D, `fsqrt`) of the skip
/// position; in states 1/4 also 35 s after the skip timer. Then by state: 1/4: when the skip can be activated,
/// pad 0's button +0x16 is just pressed (+0x46 clear) and the player has no active task 0x2C0, state 2, pad
/// +0x10F bit 0 set, black fade out over 2.5 s, timer `now + 3000` and the big message of key 0x859870 (4500 ms,
/// style 1). 2: after the timer, with the timer suspended, the scene at the skip position is loaded; the
/// player's vehicle (or, on foot, the ped) is cleared space for and teleported there: the vehicle gets heading
/// `0x96A8A4 * pi/180`, is placed on the road (bikes, class 9; cars, class 0), gets move speed `forward * 0.4`
/// and the camera's heading slot (`TheCamera.m_aCams[active]` +0x230) `heading - pi/2`; the ped gets `z =
/// ground + 1`. Timer `now + 1500`, `CHud::SetMessage(null)`, timer resumed, `PassTime(dist * 0.05 + 23)`;
/// state 5 (pad bit cleared) when 0x96A894 is set, else a 1.5 s fade in and state 3. 3: after the timer, state 0
/// and the pad bit cleared.
#[hook(addr = 0x442480, conv = cdecl, name = "CGameLogic::UpdateSkip", expect = "A1 A0 A8 96 00",
       unverified = "mvpd-world-2", route = "L2", hazards = "heap-writes,x87-transcendental",
       note = "the fsqrt distance feeds PassTime's minutes")]
pub fn game_logic_update_skip() {
    let state = g::<u32>(SKIP_STATE);
    let now = || g::<u32>(TIME_MS).read();
    let mut tmp = [0f32; 3];
    let tp = GPtr::from_raw(&mut tmp).cast::<u8>();
    let dist2d = |p: GPtr<u8>| {
        let dx = x(rd::<f32>(p, 0)).fsub_m(g::<f32>(SKIP_POSITION).read());
        let dy = x(rd::<f32>(p, 4)).fsub_m(g::<f32>(SKIP_POSITION + 4).read());
        dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt()
    };
    let mut st = state.read();
    let mut cancel = false;
    if st == 1 {
        let p = find_player_coors(tp, -1);
        if !dist2d(p).fcom(x(g::<f32>(0x858FE8).read())).test_parity(5) {
            cancel = true;
        } else {
            st = state.read();
        }
    }
    if !cancel && (st == 4 || st == 1) && now() > g::<u32>(SKIP_TIMER).read().wrapping_add(0x88B8) {
        cancel = true;
    }
    if cancel {
        st = 0;
        state.write(0);
    }
    let cam = GPtr::<u8>::from_addr(THE_CAMERA);
    match st {
        1 | 4 => {
            if skip_can_be_activated_o() == 0 {
                return;
            }
            let pad = pad_get_pad(0);
            if rd::<u16>(pad, 0x16) == 0 || rd::<u16>(pad, 0x46) != 0 {
                return;
            }
            let ped = find_player_ped(-1);
            if !task_manager_find_active_task_by_type(rd::<GPtr<u8>>(ped, 0x47C).byte_add(4), 0x2C0).is_null() {
                return;
            }
            state.write(2);
            let pad = pad_get_pad(0);
            wr::<u8>(pad, 0x10F, rd::<u8>(pad, 0x10F) | 1);
            camera_set_fade_colour(cam, 0, 0, 0);
            camera_fade(cam, f32::from_bits(0x4020_0000), 0);
            g::<u32>(SKIP_TIMER).write(now().wrapping_add(0xBB8));
            let s = text_get(GPtr::from_addr(0xC1B340), GPtr::from_addr(0x859870));
            messages_add_big_message(s, 0x1194, 1);
        }
        2 => {
            if now() <= g::<u32>(SKIP_TIMER).read() {
                return;
            }
            timer_suspend();
            let pos = GPtr::<u8>::from_addr(SKIP_POSITION);
            streaming_load_scene_collision(pos);
            streaming_load_scene(pos);
            let p = find_player_coors(tp, -1);
            let dist = dist2d_rev(p).to_f32();
            let (px, py, pz) = (g::<f32>(SKIP_POSITION).read(), g::<f32>(SKIP_POSITION + 4).read(), g::<f32>(SKIP_POSITION + 8).read());
            if !find_player_vehicle(-1, 0).is_null() {
                clear_space_for_mission_entity(pos, find_player_vehicle(-1, 0));
                let v = find_player_vehicle(-1, 0);
                v_teleport(v, px, py, pz, 0);
                let h = x(g::<f32>(0x96A8A4).read()).fmul_m(g::<f32>(0x8595EC).read()).to_f32();
                placeable_set_heading(find_player_vehicle(-1, 0), h);
                if rd::<u32>(find_player_vehicle(-1, 0), 0x590) == 9 {
                    bike_place_on_road_properly(find_player_vehicle(-1, 0));
                } else if rd::<u32>(find_player_vehicle(-1, 0), 0x590) == 0 {
                    automobile_place_on_road_properly(find_player_vehicle(-1, 0));
                }
                let fwd = rd::<GPtr<u8>>(find_player_vehicle(-1, 0), 0x14).byte_add(0x10);
                let mut out = [0f32; 3];
                let r = vector_mul(GPtr::from_raw(&mut out).cast(), fwd, f32::from_bits(0x3ECC_CCCD));
                let sp = find_player_vehicle(-1, 0).byte_add(0x44);
                wr::<u32>(sp, 0, rd::<u32>(r, 0));
                wr::<u32>(sp, 4, rd::<u32>(r, 4));
                wr::<u32>(sp, 8, rd::<u32>(r, 8));
                let hd = placeable_get_heading(find_player_vehicle(-1, 0));
                let c = X87::from_f64_exact(hd).fsub_m(g::<f32>(0x858FE4).read()).to_f32();
                let active = g::<u8>(0xB6F081).read() as usize;
                g::<f32>(0xB6F258 + 0x238 * active).write(c);
            } else {
                clear_space_for_mission_entity(pos, find_player_ped(-1));
                let ped = find_player_ped(-1);
                v_teleport(ped, px, py, pz, 0);
                let ped = find_player_ped(-1);
                let m = rd::<GPtr<u8>>(ped, 0x14);
                let at = if m.is_null() { ped.byte_add(4) } else { m.byte_add(0x30) };
                let gz = find_ground_z_for_coord(g::<f32>(SKIP_POSITION).read(), g::<f32>(SKIP_POSITION + 4).read());
                wr::<f32>(at, 8, X87::from_f64_exact(gz).fadd_m(g::<f32>(0x858624).read()).to_f32());
            }
            g::<u32>(SKIP_TIMER).write(now().wrapping_add(0x5DC));
            hud_set_message(GPtr::NULL);
            timer_resume();
            let minutes = ftol32(x(dist).fmul_m(g::<f32>(0x858C28).read()).fadd_m(g::<f32>(0x858F84).read()));
            game_logic_pass_time(minutes as u32);
            if g::<u8>(0x96A894).read() != 0 {
                state.write(5);
                let pad = pad_get_pad(0);
                wr::<u8>(pad, 0x10F, rd::<u8>(pad, 0x10F) & 0xFE);
                return;
            }
            camera_set_fade_colour(cam, 0, 0, 0);
            camera_fade(cam, f32::from_bits(0x3FC0_0000), 1);
            state.write(3);
        }
        3 => {
            if now() <= g::<u32>(SKIP_TIMER).read() {
                return;
            }
            state.write(0);
            let pad = pad_get_pad(0);
            wr::<u8>(pad, 0x10F, rd::<u8>(pad, 0x10F) & 0xFE);
        }
        _ => {}
    }
}

/// The state-2 distance: `sqrt(dy*dy + dx*dx)` with `dx = skip.x - p.x`, `dy = skip.y - p.y` (the reverse
/// subtraction of the state-1 check).
fn dist2d_rev(p: GPtr<u8>) -> X87 {
    let dx = x(g::<f32>(SKIP_POSITION).read()).fsub_m(rd::<f32>(p, 0));
    let dy = x(g::<f32>(SKIP_POSITION + 4).read()).fsub_m(rd::<f32>(p, 4));
    dy.fmul(dy).fadd(dx.fmul(dx)).fsqrt()
}

orig! {
    /// `CSetPiece::Update()` (0x499A80)
    fn set_piece_update(p: GPtr<u8>) = thiscall 0x499A80 => unported;
    /// `CObject::ProcessSamSiteBehaviour()` (0x5A07D0)
    fn object_process_sam_site_behaviour(o: GPtr<u8>) = thiscall 0x5A07D0 => unported;
    /// `CObject::ProcessTrainCrossingBehaviour()` (0x5A0B50)
    fn object_process_train_crossing_behaviour(o: GPtr<u8>) = thiscall 0x5A0B50 => unported;
    /// `CRopes::FindRope(uint32 id)` (0x556000, cdecl)
    fn ropes_find_rope(id: GPtr<u8>) -> i32 = cdecl 0x556000 => unported;
    /// `CRopes::RegisterRope(CEntity* id, uint32 type, CVector pos, bool expires, uint8 segments, uint8 flags,
    /// CPhysical* holder, uint32 expireTime)` (0x556B40, cdecl)
    fn ropes_register_rope(id: GPtr<u8>, ty: u32, x: f32, y: f32, z: f32, expires: u32, segs: u32, flags: u32, holder: GPtr<u8>, t: u32) -> u8 = cdecl 0x556B40 => unported;
    /// `CMatrix::operator*(const CMatrix&, const CVector&)` (0x59C890, cdecl, hidden result pointer); returns `out`
    fn matrix_mul_vec(out: GPtr<u8>, m: GPtr<u8>, v: GPtr<u8>) -> GPtr<u8> = cdecl 0x59C890 => unported;
    /// `CGarages::IsModelIndexADoor(int32)` (0x448AF0, cdecl)
    fn garages_is_model_index_a_door(m: i32) -> u8 = cdecl 0x448AF0 => crate::world::garages::is_model_index_a_door;
    /// `CObject::ProcessGarageDoorBehaviour()` (0x44A4D0)
    fn object_process_garage_door_behaviour_o(o: GPtr<u8>) = thiscall 0x44A4D0 => crate::world::mvpd_w2::object_process_garage_door_behaviour;
    /// `CGarages::FindGarageForObject(CObject*)` (0x44A240, cdecl)
    fn garages_find_garage_for_object(o: GPtr<u8>) -> u32 = cdecl 0x44A240 => unported;
    /// `CGarage::BuildRotatedDoorMatrix(CEntity*, float)` (0x4479F0, cdecl)
    fn garage_build_rotated_door_matrix(e: GPtr<u8>, t: f32) = cdecl 0x4479F0 => crate::world::mvpb_world3::garage_build_rotated_door_matrix;
    /// `CMatrix::UpdateRwMatrix(RwMatrix*)` (0x59AD70, `ret 4`)
    fn matrix_update_rw_matrix(m: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x59AD70 => unported;
    /// `CSimpleTransform::UpdateRwMatrix(RwMatrix*)` (0x54EF40, `ret 4`)
    fn simple_transform_update_rw_matrix(t: GPtr<u8>, rw: GPtr<u8>) = thiscall 0x54EF40 => unported;
    /// `CEntity::UpdateRwFrame()` (0x532B00)
    fn entity_update_rw_frame(e: GPtr<u8>) = thiscall 0x532B00 => unported;
    /// `CHud::HelpMessageDisplayed()` (0x588B50, cdecl)
    fn hud_help_message_displayed() -> u8 = cdecl 0x588B50 => unported;
    /// `CStats::IsStatCapped(eStats)` (0x559630, cdecl)
    fn stats_is_stat_capped(s: u32) -> u8 = cdecl 0x559630 => unported;
    /// `CStats::GetStatValue(eStats)` (0x558E40, cdecl, float in ST0)
    fn stats_get_stat_value(s: u32) -> f64 = cdecl 0x558E40 => unported;
    /// `CHud::SetHelpMessageStatUpdate(eStatUpdateState, uint16 stat, float change, float max)` (0x588D40, cdecl)
    fn hud_set_help_message_stat_update(state: u32, stat: u32, change: f32, max: f32) = cdecl 0x588D40 => unported;
    /// `CStats::FindMaxNumberOfGroupMembers()` (0x559A50, cdecl)
    fn stats_find_max_number_of_group_members() -> i32 = cdecl 0x559A50 => unported;
}

/// `CSetPieces::Update()` (0x49AA00 -> .HOODLUM 0x1569C50, cdecl): every frame updates the eighth of the set
/// pieces (`NumSetPieces` 0xA957FC) selected by `frame & 7`: indices `n*f/8 .. n*(f+1)/8` (signed divisions;
/// the end re-read every iteration). The array base comes from the protection routine 0xCB73E0's return value,
/// which is `CSetPieces::aSetPieces` (0xA95818, 0x20-byte entries).
#[hook(addr = 0x49AA00, conv = cdecl, name = "CSetPieces::Update", expect = "E9 4B F2 0C 01",
       route = "L2", hazards = "none",
       note = "array base taken as 0xA95818 (the protection routine's return value)")]
pub fn set_pieces_update() {
    let f = (g::<u32>(0xB7CB4C).read() & 7) as i32;
    let n = || g::<i32>(0xA957FC).read();
    let mut i = n().wrapping_mul(f) / 8;
    while i < n().wrapping_mul(f + 1) / 8 {
        set_piece_update(GPtr::from_addr(0xA95818 + 0x20 * i as usize));
        i += 1;
    }
}

/// `CObject::ProcessControlLogic()` (0x5A29A0): SAM sites (models 0x8CD6AC/0x8CD6B0) and train crossings
/// (0x8CD6B4/0x8CD6B8) run their behaviour; the three crane models (0x8CD6BC/0x8CD6C0/0x8CD6C4) register their
/// rope (`CRopes::RegisterRope(this, type, matrix * offset + (0, 0, slack), false, segments, 1, this, 20000)`):
/// with an existing rope (`FindRope(this)`, 0x328-byte ropes at 0xB76BC0) `segments = _ftol(+0x14 * 32)` and
/// `slack = f32(+0x0 * +0x14 - segments * +0x4)`, else 0; type 4 with offset (0, 36.64, -1.69), type 5/7 (by
/// `0 > pos.x`) with offset 0, type 6 with offset (0, 0, 59). Garage doors then run
/// `ProcessGarageDoorBehaviour`.
#[hook(addr = 0x5A29A0, conv = thiscall, name = "CObject::ProcessControlLogic", this = "CObject", expect = "83 EC 28 53 56 57 8B F9",
       unverified = "mvpd-world-2", route = "L2", hazards = "heap-writes")]
pub fn object_process_control_logic(this: GPtr<u8>) {
    let m = || rd::<i16>(this, 0x22) as i32;
    let mid = |a: usize| g::<u16>(a).read() as i32;
    if m() == mid(0x8CD6AC) || m() == mid(0x8CD6B0) {
        object_process_sam_site_behaviour(this);
    }
    if m() == mid(0x8CD6B4) || m() == mid(0x8CD6B8) {
        object_process_train_crossing_behaviour(this);
    }
    if m() == mid(0x8CD6BC) || m() == mid(0x8CD6C0) || m() == mid(0x8CD6C4) {
        let mut slack = 0f32;
        let mut segs: u32 = 0;
        let idx = ropes_find_rope(this);
        let zero = g::<f32>(0x858B50).read();
        let mut off = [zero, zero, zero];
        if idx >= 0 {
            let r = GPtr::<u8>::from_addr(0xB76BC0 + 0x328 * idx as usize);
            segs = ftol32(x(rd::<f32>(r, 0x14)).fmul_m(g::<f32>(0x85950C).read())) as u32;
            let a = x(rd::<f32>(r, 0)).fmul_m(rd::<f32>(r, 0x14));
            slack = a.fsub(X87::from_i32(segs as i32).fmul_m(rd::<f32>(r, 4))).to_f32();
        }
        let ty = if m() == mid(0x8CD6BC) {
            off = [zero, f32::from_bits(0x4212_8F5C), f32::from_bits(0xBFD8_51EC)];
            4
        } else if m() == mid(0x8CD6C0) {
            let mat = rd::<GPtr<u8>>(this, 0x14);
            let pos = if mat.is_null() { this.byte_add(4) } else { mat.byte_add(0x30) };
            if x(zero).fcom(x(rd::<f32>(pos, 0))).test_any(0x41) { 7 } else { 5 }
        } else {
            off = [zero, zero, f32::from_bits(0x426C_0000)];
            6
        };
        let mut out = [0f32; 3];
        let o = matrix_mul_vec(GPtr::from_raw(&mut out).cast(), rd::<GPtr<u8>>(this, 0x14), GPtr::from_raw(&mut off).cast());
        let (px, py) = (rd::<f32>(o, 0), rd::<f32>(o, 4));
        let pz = x(rd::<f32>(o, 8)).fadd_m(slack).to_f32();
        ropes_register_rope(this, ty, px, py, pz, 0, segs, 1, this, 0x4E20);
    }
    if garages_is_model_index_a_door(rd::<i16>(this, 0x22) as i32) != 0 {
        object_process_garage_door_behaviour_o(this);
    }
}

/// `CObject::ProcessGarageDoorBehaviour()` (0x44A4D0): finds the door's garage once (+0x147, -1 unknown); with
/// one (`CGarages::aGarages` 0x96C048, 0xD8 bytes: door position +0x38, type +0x4C, flags +0x4E) the door is
/// moved relative to the position of the entity at +0x170, by `h = colmodel height - 0x858B1C`: rotating doors
/// (flag 8) rise `h * door * 0x8599A4` and get `BuildRotatedDoorMatrix(door or -door with flag 0x10)`; type
/// 0x2C sinks by `h * door`; type 0x2D slides along x by `door * right.x * 0x859030`; others rise
/// `h * door * 0x8599A0`. Flag 0x40 is copied into the entity's +0x1C bit 0, then the RW matrix and frame are
/// updated (inlined `UpdateRwMatrix`, tail `UpdateRwFrame`).
#[hook(addr = 0x44A4D0, conv = thiscall, name = "CObject::ProcessGarageDoorBehaviour", this = "CObject", expect = "83 EC 10 56 8B F1",
       unverified = "mvpd-world-2", route = "L2", hazards = "heap-writes")]
pub fn object_process_garage_door_behaviour(this: GPtr<u8>) {
    if (rd::<u8>(this, 0x147) as i8) < 0 {
        wr::<u8>(this, 0x147, garages_find_garage_for_object(this) as u8);
    }
    let gi = rd::<u8>(this, 0x147) as i8;
    if gi < 0 {
        return;
    }
    let base = rd::<GPtr<u8>>(this, 0x170);
    let bm = rd::<GPtr<u8>>(base, 0x14);
    let bp = if bm.is_null() { base.byte_add(4) } else { bm.byte_add(0x30) };
    let (bx, bz) = (rd::<f32>(bp, 0), rd::<f32>(bp, 8));
    let mi = g::<GPtr<u8>>(0xA9B0C8 + 4 * rd::<i16>(this, 0x22) as isize as usize).read();
    let col = rd::<GPtr<u8>>(mi, 0x14);
    let h = x(rd::<f32>(col, 0x14)).fsub_m(rd::<f32>(col, 8)).fsub_m(g::<f32>(0x858B1C).read());
    let gar = GPtr::<u8>::from_addr((0x96C048 + 0xD8 * gi as isize) as usize);
    let door = rd::<f32>(gar, 0x38);
    let mat = rd::<GPtr<u8>>(this, 0x14);
    if rd::<u8>(gar, 0x4E) & 8 != 0 {
        wr::<f32>(mat, 0x38, h.fmul_m(door).fmul_m(g::<f32>(0x8599A4).read()).fadd_m(bz).to_f32());
        let gar = GPtr::<u8>::from_addr((0x96C048 + 0xD8 * rd::<u8>(this, 0x147) as i8 as isize) as usize);
        let mut t = rd::<f32>(gar, 0x38);
        if rd::<u8>(gar, 0x4E) & 0x10 != 0 {
            t = x(t).fchs().to_f32();
        }
        garage_build_rotated_door_matrix(this, t);
    } else if rd::<u8>(gar, 0x4C) == 0x2C {
        wr::<f32>(mat, 0x38, x(bz).fsub(h.fmul_m(door)).to_f32());
    } else if rd::<u8>(gar, 0x4C) == 0x2D {
        let v = x(door).fmul_m(rd::<f32>(mat, 0)).fmul_m(g::<f32>(0x859030).read());
        wr::<f32>(mat, 0x30, x(bx).fsub(v).to_f32());
    } else {
        wr::<f32>(mat, 0x38, h.fmul_m(door).fmul_m(g::<f32>(0x8599A0).read()).fadd_m(bz).to_f32());
    }
    let gar = GPtr::<u8>::from_addr((0x96C048 + 0xD8 * rd::<u8>(this, 0x147) as i8 as isize) as usize);
    let bit = ((rd::<u8>(gar, 0x4E) as u32) >> 6) & 1;
    wr::<u32>(this, 0x1C, (rd::<u32>(this, 0x1C) & !1) | bit);
    let rw = rd::<GPtr<u8>>(this, 0x18);
    if !rw.is_null() {
        let m = rd::<GPtr<u8>>(rw, 4).byte_add(0x10);
        let mat = rd::<GPtr<u8>>(this, 0x14);
        if mat.is_null() {
            simple_transform_update_rw_matrix(this.byte_add(4), m);
        } else {
            matrix_update_rw_matrix(mat, m);
        }
    }
    entity_update_rw_frame(this);
}

/// `CStats::DisplayScriptStatUpdateMessage(eStatUpdateState, eStats, float change)` (0x55B980, cdecl): unless
/// pad 0 +0x117, `!0x8CDE56`, 0xB6F065, a displayed help message, 0xB794D4 or 0xBA82E3 (all of which clear
/// 0xB794D4), and unless the stat is capped at 1000 or more, shows `CHud::SetHelpMessageStatUpdate(state, stat,
/// change, max)` with max 10 for stat 0xD2, 1000 for stat 0x40, 1000 for the progress stats (0x15..0x19,
/// 0x45..0x4F, 0x51, 0xA0, 0xA4, 0xA5, 0xDF, 0xE1, 0xE5, 0xE6, 0xE9) only when `change > 1` (or NaN), 100 for
/// 0xFC..0x101, and for 0x150 (gang recruits, with a player ped) the smaller of
/// `FindMaxNumberOfGroupMembers()` and the player data's byte +0x43.
#[hook(addr = 0x55B980, conv = cdecl, name = "CStats::DisplayScriptStatUpdateMessage", expect = "51 56 6A 00 E8 E7 41 FE FF",
       route = "L2", hazards = "none")]
pub fn stats_display_script_stat_update_message(state: u32, stat: u32, change: f32) {
    let flag = g::<u8>(0xB794D4);
    if rd::<u8>(pad_get_pad(0), 0x117) != 0
        || g::<u8>(0x8CDE56).read() == 0
        || g::<u8>(0xB6F065).read() != 0
        || hud_help_message_displayed() != 0
        || flag.read() != 0
        || g::<u8>(0xBA82E3).read() != 0
    {
        flag.write(0);
        return;
    }
    if stats_is_stat_capped(stat) != 0
        && !X87::from_f64_exact(stats_get_stat_value(stat)).fcom(x(g::<f32>(0x858C4C).read())).test_any(1)
    {
        return;
    }
    let progress = |s: u32| {
        matches!(s, 0x15..=0x19 | 0x45..=0x4F | 0x51 | 0xA0 | 0xA4 | 0xA5 | 0xDF | 0xE1 | 0xE5 | 0xE6 | 0xE9)
    };
    let max = match stat {
        0xD2 => f32::from_bits(0x4120_0000),
        0x40 => f32::from_bits(0x447A_0000),
        s if progress(s) => {
            if !x(change).fcom(x(g::<f32>(0x858624).read())).test_parity(0x41) {
                return;
            }
            f32::from_bits(0x447A_0000)
        }
        0xFC..=0x101 => f32::from_bits(0x42C8_0000),
        0x150 => {
            if find_player_ped(-1).is_null() {
                return;
            }
            let ped = find_player_ped(-1);
            let n = rd::<u8>(rd::<GPtr<u8>>(ped, 0x480), 0x43) as i32;
            if stats_find_max_number_of_group_members() < n {
                X87::from_i32(stats_find_max_number_of_group_members()).to_f32()
            } else {
                let ped = find_player_ped(-1);
                X87::from_i32(rd::<u8>(rd::<GPtr<u8>>(ped, 0x480), 0x43) as i32).to_f32()
            }
        }
        _ => return,
    };
    hud_set_help_message_stat_update(state, stat, change, max);
}

/// `CGameLogic::CalcDistanceToForbiddenTrainCrossing(CVector point, CVector moveSpeed, bool ignoreMoveSpeed,
/// CVector& out)` (0x4418E0, cdecl, float in ST0): of the three train crossings (82, -1021) (only while stat 0xB5
/// is 0), (-1568, 537) (stat == 1) and (2770, 277) (stat <= 1), those approached (`(c - p).xy . move.xy -
/// p.z * move.z > 0`, NaN counts as not approached) or all with `ignoreMoveSpeed` are measured 2D (`fsqrt`);
/// a closer one than the best so far (first against the constant 100000, later against the stored f32 minimum)
/// stores `(c.x - p.x, c.y - p.y, -p.z)` into `out`. Returns the minimum (initially 100000); a hit on the
/// last crossing returns its unrounded `fsqrt` value.
#[hook(addr = 0x4418E0, conv = cdecl, name = "CGameLogic::CalcDistanceToForbiddenTrainCrossing", expect = "83 EC 18 53 56",
       unverified = "mvpd-world-2", route = "L2", hazards = "x87-transcendental",
       note = "returns the fsqrt result of the last crossing unrounded")]
#[allow(clippy::too_many_arguments)]
pub fn game_logic_calc_distance_to_forbidden_train_crossing(
    px: f32,
    py: f32,
    pz: f32,
    mx: f32,
    my: f32,
    mz: f32,
    ignore: u8,
    out: GPtr<u8>,
) -> X87 {
    let zero = x(g::<f32>(0x858B50).read());
    let one = x(g::<f32>(0x858624).read());
    let mut min = f32::from_bits(0x47C3_5000);
    // `Some(dist)` when the crossing beats the bound and was stored
    let cross = |cxa: usize, cya: usize, bound: X87, min: &mut f32| -> Option<X87> {
        let (cx, cy) = (g::<f32>(cxa).read(), g::<f32>(cya).read());
        let a = x(cx).fsub_m(px);
        let b = x(cy).fsub_m(py).to_f32();
        let npz = x(pz).fchs();
        let t = x(my).fmul_m(b).fadd(x(mz).fmul(npz)).fadd(a.fmul_m(mx));
        if !t.fcom(zero).test_any(0x41) || ignore != 0 {
            let u = x(px).fsub_m(cx);
            let v = x(py).fsub_m(cy);
            let d = v.fmul(v).fadd(u.fmul(u)).fsqrt();
            if !d.fcom(bound).test_parity(5) {
                *min = d.to_f32();
                wr::<f32>(out, 0, a.to_f32());
                wr::<f32>(out, 4, b);
                wr::<f32>(out, 8, npz.to_f32());
                return Some(d);
            }
        }
        None
    };
    if X87::from_f64_exact(stats_get_stat_value(0xB5)).fcom(zero).test_parity(0x44) {
        // skipped: the stat is not 0
    } else {
        cross(0x859834, 0x859830, x(g::<f32>(0x858EF8).read()), &mut min);
    }
    if !X87::from_f64_exact(stats_get_stat_value(0xB5)).fcom(one).test_parity(0x44) {
        let bound = x(min);
        cross(0x85982C, 0x859828, bound, &mut min);
    }
    if !X87::from_f64_exact(stats_get_stat_value(0xB5)).fcom(one).test_parity(0x41) {
        let bound = x(min);
        if let Some(d) = cross(0x859824, 0x859820, bound, &mut min) {
            return d;
        }
    }
    x(min)
}

orig! {
    /// `FUN_00420800(float a, float b)` (0x420800, cdecl, float in ST0): `a > b ? a : b`
    fn max_f(a: f32, b: f32) -> f64 = cdecl 0x420800 => unported;
    /// `FUN_00404330(float a, float b)` (0x404330, cdecl, float in ST0): `a < b ? a : b`
    fn min_f(a: f32, b: f32) -> f64 = cdecl 0x404330 => unported;
}

/// `CStats::GetFatAndMuscleModifier(eStatModAbilities)` (0x559AF0, cdecl, float in ST0): the ability modifiers
/// built from fat (0xB793D4), muscle (0xB793DC), stamina-like 0xB793D8, the int stats 0xB791B8 / 0xB791A4 /
/// 0xB791B4 / 0xB790A0 and the float 0xB793E0, with the tuning floats at 0x8CDE58..0x8CDEC0 (fat offset
/// 0x8CDEC0, muscle offset 0x8CDEB8, stats scaled by `/1000` as `(s - off) / (1000 - off)` or `* 0x858CDC`).
/// Each case's x87 order and clamps (some through `FUN_00420800` max / `FUN_00404330` min) as in the original;
/// ability values above 15 return 1.
#[hook(addr = 0x559AF0, conv = cdecl, name = "CStats::GetFatAndMuscleModifier", expect = "8B 44 24 04 D9 05 24 86 85 00",
       route = "L2", hazards = "none")]
pub fn stats_get_fat_and_muscle_modifier(ability: u32) -> X87 {
    let f = |a: usize| g::<f32>(a).read();
    let one = f(0x858624);
    let k1000 = f(0x858C4C);
    let kint = f(0x858CDC);
    let fat = f(0xB793D4);
    let muscle = f(0xB793DC);
    // `(fat - off) * k / (1000 - off)` and the same for muscle
    let fat_term = |k: usize| x(fat).fsub_m(f(0x8CDEC0)).fmul_m(f(k)).fdiv(x(k1000).fsub_m(f(0x8CDEC0)));
    let muscle_term = |k: usize| x(muscle).fsub_m(f(0x8CDEB8)).fmul_m(f(k)).fdiv(x(k1000).fsub_m(f(0x8CDEB8)));
    // `max(0, f32((fat - off) / (1000 - off))) * k + 1`
    let fat_max_term = |k: usize| {
        let v = x(fat).fsub_m(f(0x8CDEC0)).fdiv(x(k1000).fsub_m(f(0x8CDEC0))).to_f32();
        X87::from_f64_exact(max_f(0.0, v)).fmul_m(f(k)).fadd_m(one)
    };
    // the value unless it is below `lo` (ordered)
    let at_least = |v: X87, lo: f32| if v.fcom(x(lo)).test_parity(5) { v } else { x(lo) };
    // the value unless it is above `hi` (ordered)
    let at_most = |v: X87, hi: f32| if v.fcom(x(hi)).test_any(0x41) { v } else { x(hi) };
    let int_stat = |a: usize| X87::from_i32(g::<i32>(a).read()).fmul_m(kint);
    // `min(1, f32(int * k)) * m + 1`
    let capped_int = |m: usize| {
        let v = int_stat(0xB791B4).to_f32();
        X87::from_f64_exact(min_f(one, v)).fmul_m(f(m)).fadd_m(one)
    };
    match ability {
        0 => {
            if !x(fat).fcom(x(f(0x8594DC))).test_any(0x41) {
                x(f(0x858CA0))
            } else if !x(fat).fcom(x(f(0x85A700))).test_any(0x41) {
                x(one)
            } else {
                x(f(0x858B50))
            }
        }
        1 => at_least(fat_max_term(0x8CDEBC).fadd(muscle_term(0x8CDEB4)), f(0x858CB0)),
        2 => at_least(fat_max_term(0x8CDEB0).fadd(muscle_term(0x8CDEAC)), f(0x858C98)),
        3 => at_least(fat_term(0x8CDEA8).fadd_m(one).fadd(muscle_term(0x8CDEA4)), f(0x858C98)),
        4 => at_most(fat_term(0x8CDEA0).fadd_m(one).fadd(muscle_term(0x8CDE9C)), f(0x858CA0)),
        5 => {
            let s = fat_term(0x8CDE98).fadd_m(one).fadd(muscle_term(0x8CDE94));
            let s = s.fadd(x(f(0xB793D8)).fmul_m(kint).fmul_m(f(0x8CDE90)));
            let s = s.fadd(int_stat(0xB791B8).fmul_m(f(0x8CDE8C)));
            if !s.fcom(x(f(0x858CA0))).test_any(0x41) { x(f(0x858CA0)) } else { at_least(s, f(0x858C84)) }
        }
        6 => {
            let s = fat_term(0x8CDE88).fadd_m(one).fadd(int_stat(0xB791B8).fmul_m(f(0x8CDE84)));
            if !s.fcom(x(f(0x858CA0))).test_any(0x41) { x(f(0x858CA0)) } else { at_least(s, f(0x858B8C)) }
        }
        7 => x(f(0xB793D8)).fmul_m(kint).fmul_m(f(0x8CDE80)).fadd_m(f(0x858A28)),
        8 => X87::from_i32(g::<i32>(0xB791A4).read()).fadd_m(f(0xB793D8)).fmul_m(f(0x859978)).fmul_m(f(0x8CDE7C)).fadd_m(k1000),
        9 => x(f(0xB793E0)).fmul_m(kint).fmul_m(f(0x8CDE78)),
        10 => x(f(0x8CDE78)),
        11 => capped_int(0x8CDE74),
        12 => capped_int(0x8CDE70),
        13 => capped_int(0x8CDE6C),
        14 => {
            let a = fat_term(0x8CDE68);
            let b = int_stat(0xB790A0).fmul_m(f(0x8CDE64)).fadd_m(one);
            at_most(a.fadd(b), one)
        }
        15 => at_least(fat_term(0x8CDE60).fadd_m(one).fadd(muscle_term(0x8CDE5C)), f(0x8CDE58)),
        _ => x(one),
    }
}

/// Every registration of this module.
pub const HOOKS: &[HookReg] = &[
    HOOK_GAME_LOGIC_SKIP_CAN_BE_ACTIVATED,
    HOOK_GAME_LOGIC_UPDATE_SKIP,
    HOOK_SET_PIECES_UPDATE,
    HOOK_OBJECT_PROCESS_CONTROL_LOGIC,
    HOOK_OBJECT_PROCESS_GARAGE_DOOR_BEHAVIOUR,
    HOOK_STATS_DISPLAY_SCRIPT_STAT_UPDATE_MESSAGE,
    HOOK_GAME_LOGIC_CALC_DISTANCE_TO_FORBIDDEN_TRAIN_CROSSING,
    HOOK_STATS_GET_FAT_AND_MUSCLE_MODIFIER,
];
