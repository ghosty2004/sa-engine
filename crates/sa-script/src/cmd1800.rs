//! `CRunningScript::ProcessCommands1800To1899` (0x46D050): complete port (PORT-SCRIPT-2 partial, completed in
//! PORT-SCRIPT-4). Every opcode with a jump-table case is in [`PORTED`]; 0753, 0756, 0757, 0768 and 0769 reach
//! the default (`or al,0xFF` at 0x46ED96: -1), as does every command outside the range.
//! Cases follow the 1.0 US bytes and `docs/re/specs/SCRIPT/commands/ProcessCommands1800To1899.md`.
//!
//! Divergence notes (ledger classes of PORT-SCRIPT-2/3): text labels are read into zeroed buffers one byte
//! longer than the label (8-byte labels: 16 bytes); the original's stack buffers are not cleared, so a
//! label that leaves its terminator to the buffer's end makes the callee read stale stack bytes. Bools the
//! original pushes as a dword whose low byte only was written (`mov byte [esp+x],0/1` or `setcc`) are
//! passed as 0/1; the callees read the low byte. The SEH state stores of the task cases are not ported.

#[allow(unused_imports)]
use crate::cmdlib::*;
use crate::vm::Scr;
use sa_core::abi::HookReg;
use sa_core::mem::GPtr;
use sa_core::{hook, orig};
use sa_math::x87::{Cmp, X87};

/// Opcodes with a Rust case, ascending.
pub const PORTED: &[u16] = &[
    0x0708, 0x0709, 0x070A, 0x070B, 0x070C, 0x070D, 0x070E, 0x070F, 0x0710, 0x0711, 0x0712, 0x0713, 0x0714,
    0x0715, 0x0716, 0x0717, 0x0718, 0x0719, 0x071A, 0x071B, 0x071C, 0x071D, 0x071E, 0x071F, 0x0720, 0x0721,
    0x0722, 0x0723, 0x0724, 0x0725, 0x0726, 0x0727, 0x0728, 0x0729, 0x072A, 0x072B, 0x072C, 0x072D, 0x072E,
    0x072F, 0x0730, 0x0731, 0x0732, 0x0733, 0x0734, 0x0735, 0x0736, 0x0737, 0x0738, 0x0739, 0x073A, 0x073B,
    0x073C, 0x073D, 0x073E, 0x073F, 0x0740, 0x0741, 0x0742, 0x0743, 0x0744, 0x0745, 0x0746, 0x0747, 0x0748,
    0x0749, 0x074A, 0x074B, 0x074C, 0x074D, 0x074E, 0x074F, 0x0750, 0x0751, 0x0752, 0x0754, 0x0755, 0x0758,
    0x0759, 0x075A, 0x075B, 0x075C, 0x075D, 0x075E, 0x075F, 0x0760, 0x0761, 0x0762, 0x0763, 0x0764, 0x0765,
    0x0766, 0x0767, 0x076A, 0x076B,
];

/// `.rdata` 0x8595EC: 0.017453292f (degrees to radians)
const K_DEG2RAD: u32 = 0x3C8E_FA35;
/// `.rdata` 0x858B8C: 0.5f; 0x859520: 1/640; 0x859524: 1/448
const K_HALF: u32 = 0x3F00_0000;
const K_INV_640: u32 = 0x3ACC_CCCD;
const K_INV_448: u32 = 0x3B12_4925;
/// `RsGlobal.maximumWidth` / `maximumHeight` (int) 0xC17044 / 0xC17048.
const RS_MAX_WIDTH: usize = 0xC17044;
const RS_MAX_HEIGHT: usize = 0xC17048;
/// `CTheScripts::NumberOfIntroRectanglesThisFrame` (u16) 0xA44B5C and `IntroRectangles[0]` (0x3C bytes) 0xA92D68.
const NUM_SCRIPT_RECTS: usize = 0xA44B5C;
const INTRO_RECTANGLES: u32 = 0xA92D68;
/// The patrol route (`CTheScripts` patrol route, count dword first) 0xC18DB8.
const PATROL_ROUTE: usize = 0xC18DB8;
/// `.rdata` 0x859E38: "NONE"
const NONE_STR: usize = 0x859E38;
/// The 9999.9f immediate (0x461C3F9A) of the random car/char searches.
const K_9999_9: u32 = 0x461C_3F9A;

orig! {
    /// `CRunningScript::ReadTextLabelFromScript(char*, uint8)` (0x463D50)
    fn read_label(s: Scr, buf: GPtr<u8>, len: u32) = thiscall 0x463D50 => crate::vm::read_text_label_from_script;
    /// `CAnimManager::GetAnimationBlock(const char*)` (0x4D3940)
    fn get_animation_block(name: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D3940 => unported;
    /// `CAnimManager::GetAnimation(const char*, const CAnimBlock*)` (0x4D42F0)
    fn get_animation(name: GPtr<u8>, block: GPtr<u8>) -> GPtr<u8> = cdecl 0x4D42F0 => unported;
    /// `RpAnimBlendClumpIsInitialized(RpClump*)` (0x4D6760), AL
    fn rp_anim_blend_clump_is_initialized(clump: GPtr<u8>) -> u8 = cdecl 0x4D6760 => unported;
    /// `RpAnimBlendClumpInit(RpClump*)` (0x4D6720)
    fn rp_anim_blend_clump_init(clump: GPtr<u8>) = cdecl 0x4D6720 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, CAnimBlendHierarchy*, int32 flags, float blendDelta)` (0x4D4410)
    fn blend_animation(clump: GPtr<u8>, anim: GPtr<u8>, flags: u32, delta: u32) -> GPtr<u8> = cdecl 0x4D4410 => unported;
    /// `CRunningScript::ScriptTaskPickUpObject(int32 command)` (0x46AF50; reads its own parameters)
    fn script_task_pick_up_object(s: Scr, command: u32) = thiscall 0x46AF50 => unported;
    /// `CTask::operator new(uint32)` (0x61A5A0)
    fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CRunningScript::GivePedScriptedTask(int32 pedHandle, CTask*, int32 opcode)` (0x465C20)
    fn give_ped_scripted_task(s: Scr, ped: u32, task: GPtr<u8>, command: u32) = thiscall 0x465C20 => unported;
    /// A virtual `CTask` method without arguments returning EAX (`GetTaskType`, vtable +0x10), read at run time
    fn vcall_task_type(f: u32, this: GPtr<u8>) -> u32 = thiscall dyn => unported;
    /// The drive-by task constructor 0x6217D0 (`CTaskSimpleGangDriveBy`: target entity, `const CVector*` target
    /// coords, abort range, frequency, style, seat RHS), returns this
    fn task_drive_by_ctor(t: GPtr<u8>, target: GPtr<u8>, pos: GPtr<u32>, range: u32, freq: u32, style: u32, rhs: u32) -> GPtr<u8> = thiscall 0x6217D0 => unported;
    /// The mobile phone task constructor 0x6348A0 (`CTaskComplexUseMobilePhone(int32 duration)`), returns this
    fn task_use_mobile_phone_ctor(t: GPtr<u8>, duration: u32) -> GPtr<u8> = thiscall 0x6348A0 => unported;
    /// `CTaskManager::FindTaskByType(int32 slot, int32 type)` (0x6817D0)
    fn find_task_by_type(tm: GPtr<u8>, slot: u32, ty: u32) -> GPtr<u8> = thiscall 0x6817D0 => unported;
    /// 0x634A40 (thiscall on the phone task, `CPed*`): ends the mobile phone task
    fn phone_task_stop(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x634A40 => unported;
    /// `CClothes::RebuildPlayer(CPlayerPed*, bool bIgnoreFatAndMuscle)` (0x5A82C0)
    fn rebuild_player(ped: GPtr<u8>, ignore: u32) = cdecl 0x5A82C0 => unported;
    /// `CReplay::Init()` (0x45E220)
    fn replay_init() = cdecl 0x45E220 => unported;
    /// `CTheScripts::GetActualScriptThingIndex(int32 ref, uint8 type)` (0x4839A0)
    fn get_actual_script_thing_index(r: u32, ty: u32) -> i32 = cdecl 0x4839A0 => unported;
    /// `CDecisionMakerTypes::GetInstance()` (0x4684F0)
    fn dm_types_instance() -> GPtr<u8> = cdecl 0x4684F0 => unported;
    /// `CDecisionMakerTypes::FlushDecisionMakerEventResponse(int32 dm, int32 event)` (0x604490)
    fn flush_dm_event_response(t: GPtr<u8>, i: i32, ev: u32) = thiscall 0x604490 => unported;
    /// `CDecisionMakerTypes::AddEventResponse(int32 dm, eEventType, eTaskType, float* chances, int32* flags)`
    /// (0x6044C0; reads 4 floats and 2 ints)
    fn add_dm_event_response(t: GPtr<u8>, i: i32, ev: u32, task: u32, chances: GPtr<u32>, flags: GPtr<u32>) = thiscall 0x6044C0 => unported;
    /// `CPedIntelligence::GetTaskHold(bool)` (0x600FF0)
    fn get_task_hold(intel: GPtr<u8>, b: u32) -> GPtr<u8> = thiscall 0x600FF0 => unported;
    /// `CTaskSimpleHoldEntity::DropEntity(CPed*, bool)` (0x6930F0)
    fn drop_entity(t: GPtr<u8>, ped: GPtr<u8>, b: u32) = thiscall 0x6930F0 => unported;
    /// `CVehicle` virtual slot 0xA4 (`BlowUpCar(CEntity* damager, bool inCutscene)`)
    fn blow_up_car(v: GPtr<u8>, damager: u32, in_cutscene: u32) = thiscall vslot 0xA4 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00)
    fn clean_up_old_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70)
    fn register_reference(e: GPtr<u8>, r: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// 0x41BDD0 (thiscall on the vehicle, bool; a `.HOODLUM` stub, gr `CVehicle::SetEngineOn`)
    fn set_engine_on(v: GPtr<u8>, on: u32) = thiscall 0x41BDD0 => unported;
    /// 0x45AD40 (cdecl, `CVehicle*`; a `.HOODLUM` stub: takes remote control of the car)
    fn take_remote_control(v: GPtr<u8>) = cdecl 0x45AD40 => unported;
    /// `CWorld::FindGroundZForCoord(float, float)` (0x569660), ST0 (read as f64: exact); floats raw
    fn find_ground_z(x: u32, y: u32) -> f64 = cdecl 0x569660 => unported;
    /// `CWorld::FindNearestObjectOfType(int32 model, const CVector&, float radius, bool 2d, bool buildings,
    /// bool vehicles, bool peds, bool objects, bool dummies)` (0x5693F0); radius raw
    fn find_nearest_object_of_type(model: u32, pos: GPtr<u32>, radius: u32, a: u32, b: u32, c: u32, d: u32, e: u32, f: u32) -> GPtr<u8> = cdecl 0x5693F0 => unported;
    /// 0x46A2F0 (thiscall on the object, AL; a `.HOODLUM` stub: the object's damaged-state query)
    fn object_is_damaged(o: GPtr<u8>) -> u8 = thiscall 0x46A2F0 => unported;
    /// `CConversations::StartSettingUpConversation(CPed*)` (0x43A840)
    fn start_setting_up_conversation(p: GPtr<u8>) = cdecl 0x43A840 => unported;
    /// `CConversations::DoneSettingUpConversation(bool)` (0x43ADB0)
    fn done_setting_up_conversation(b: u32) = cdecl 0x43ADB0 => unported;
    /// `CConversations::IsConversationAtNode(char*, CPed*)` (0x43B000), AL
    fn is_conversation_at_node(label: GPtr<u8>, p: GPtr<u8>) -> u8 = cdecl 0x43B000 => unported;
    /// `CGlass::IsObjectGlass(CEntity*)` (0x46A760), AL
    fn is_object_glass(o: GPtr<u8>) -> u8 = cdecl 0x46A760 => unported;
    /// `CGlass::WindowRespondsToCollision(CEntity*, float, CVector speed, CVector point, bool)` (0x71BC40);
    /// the float and both vectors by value as raw dwords
    fn window_responds_to_collision(o: GPtr<u8>, f: u32, sx: u32, sy: u32, sz: u32, px: u32, py: u32, pz: u32, b: u32) = cdecl 0x71BC40 => unported;
    /// `BreakManager_c::Add(CObject*, RwV3d*, float, bool)` (0x59E9B0), ECX = g_breakMan 0xBB4240; float raw
    fn break_manager_add(m: GPtr<u8>, o: GPtr<u8>, v: GPtr<u32>, f: u32, b: u32) = thiscall 0x59E9B0 => unported;
    /// `CEntity::GetIsStatic()` (0x4633E0), AL
    fn get_is_static(o: GPtr<u8>) -> u8 = thiscall 0x4633E0 => unported;
    /// `CPhysical::RemoveFromMovingList()` (0x542860)
    fn remove_from_moving_list(o: GPtr<u8>) = thiscall 0x542860 => unported;
    /// `CEntity` virtual slot 0x20 without arguments (called on the broken object)
    fn entity_vslot_20(o: GPtr<u8>) = thiscall vslot 0x20 => unported;
    /// 0x6470E0 (thiscall; the 0x1C-byte warp-as-driver task constructor `(CVehicle*, CTaskUtilityLineUpPedWithCar*)`)
    fn task_set_ped_in_as_driver_ctor(t: GPtr<u8>, v: GPtr<u8>, u: u32) -> GPtr<u8> = thiscall 0x6470E0 => unported;
    /// `CCarEnterExit::ComputeTargetDoorToEnterAsPassenger(const CVehicle*, int32)` (0x64F190)
    fn compute_target_door_as_passenger(v: GPtr<u8>, seat: u32) -> u32 = cdecl 0x64F190 => unported;
    /// 0x646FE0 (thiscall; the 0x20-byte warp-as-passenger task constructor `(CVehicle*, int32 door,
    /// CTaskUtilityLineUpPedWithCar*)`)
    fn task_set_ped_in_as_passenger_ctor(t: GPtr<u8>, v: GPtr<u8>, door: u32, u: u32) -> GPtr<u8> = thiscall 0x646FE0 => unported;
    /// `CStreaming::DisableCopBikes(bool)` (0x407D10)
    fn disable_cop_bikes(b: u32) = cdecl 0x407D10 => unported;
    /// `CRunningScript::FlameInAngledAreaCheckCommand(int32)` (0x488780)
    fn flame_in_angled_area_check_command(s: Scr, command: u32) = thiscall 0x488780 => unported;
    /// `CStuckCarCheck::AddCarToCheck(int32, float, uint32, bool, bool, bool, bool, int8)` (0x465970), ECX =
    /// `CTheScripts::StuckCars` 0xA90AB0; float raw
    fn add_car_to_check(m: GPtr<u8>, h: u32, dist: u32, time: u32, a: u32, b: u32, c: u32, d: u32, e: u32) = thiscall 0x465970 => unported;
    /// `CDamageManager::ApplyDamage(CAutomobile*, tComponent, float, float)` (0x6C24B0); floats raw
    fn apply_damage(dm: GPtr<u8>, v: GPtr<u8>, comp: u32, intensity: u32, mult: u32) = thiscall 0x6C24B0 => unported;
    /// `CPlaceable::GetHeading()` (0x441DB0), ST0 (an FPATAN result read as f64, the MATH-2 model)
    fn get_heading(e: GPtr<u8>) -> f64 = thiscall 0x441DB0 => unported;
    /// `CMatrix::SetRotateZ(float)` (0x59B0E0) / `CMatrix::RotateY(float)` (0x59B2C0); raw floats
    fn matrix_set_rotate_z(m: GPtr<u8>, a: u32) = thiscall 0x59B0E0 => unported;
    fn matrix_rotate_y(m: GPtr<u8>, a: u32) = thiscall 0x59B2C0 => unported;
    /// 0x46AE00 (thiscall on a `CMatrix`, `const CVector*`; adds the vector to the position +0x30..+0x38)
    fn matrix_add_pos(m: GPtr<u8>, v: GPtr<u32>) = thiscall 0x46AE00 => unported;
    /// 0x46B1A0 (cdecl, model: adds it to `CTheScripts::SuppressedVehicleModels`)
    fn suppress_car_model(m: u32) = cdecl 0x46B1A0 => unported;
    /// 0x46A7E0 (cdecl, model; a `.HOODLUM` stub: removes it from the suppressed models)
    fn dont_suppress_car_model(m: u32) = cdecl 0x46A7E0 => unported;
    /// 0x571980 / 0x571970 (thiscall on 0xB9B9A4, `(uint16 key, int32 1, const char* 0x859E40)`, AL): the
    /// PS2-keyboard key queries
    fn ps2_key_pressed(m: GPtr<u8>, key: u32, one: u32, s: GPtr<u8>) -> u8 = thiscall 0x571980 => unported;
    fn ps2_key_just_pressed(m: GPtr<u8>, key: u32, one: u32, s: GPtr<u8>) -> u8 = thiscall 0x571970 => unported;
    /// `CPool<CPed>` slot accessor 0x4082A0 (thiscall on the pool, slot index; a `.HOODLUM` stub)
    fn ped_pool_get_at(pool: GPtr<u8>, i: u32) -> GPtr<u8> = thiscall 0x4082A0 => unported;
    /// `CTaskSimpleHoldEntity` virtual slot 0x20 `(CPed*)`, AL
    fn task_vslot_20(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall vslot 0x20 => unported;
    /// `CPool<CVehicle>` slot accessor 0x41CC10 (thiscall on the pool, slot index; a `.HOODLUM` stub)
    fn vehicle_pool_get_at(pool: GPtr<u8>, i: u32) -> GPtr<u8> = thiscall 0x41CC10 => unported;
    /// `CVehicle::GetVehicleAppearance()` (0x6D1080)
    fn get_vehicle_appearance(v: GPtr<u8>) -> i32 = thiscall 0x6D1080 => unported;
    /// `CVehicle::CanBeDeleted()` (0x6D1180), AL
    fn can_be_deleted(v: GPtr<u8>) -> u8 = thiscall 0x6D1180 => unported;
    /// `CVector::Magnitude()` (0x4082C0), ST0 (an `fsqrt` result under PC=24: exact as f64)
    fn vector_magnitude(v: GPtr<u32>) -> f64 = thiscall 0x4082C0 => unported;
    /// `CPool<CVehicle>::GetRef(CVehicle*)` (0x424160)
    fn vehicle_get_ref(pool: GPtr<u8>, v: GPtr<u8>) -> i32 = thiscall 0x424160 => unported;
    /// `CRunningScript::IsPedDead(CPed*)` (0x464D70), AL
    fn script_is_ped_dead(s: Scr, p: GPtr<u8>) -> u8 = thiscall 0x464D70 => unported;
    /// `CRunningScript::ThisIsAValidRandomPed(ePedType, bool civ, bool gang, bool criminal)` (0x489490), AL
    fn this_is_a_valid_random_ped(s: Scr, ty: u32, civ: u32, gang: u32, crim: u32) -> u8 = thiscall 0x489490 => unported;
    /// `CPedGroups::GetPedsGroup(const CPed*)` (0x5F7E80)
    fn get_peds_group(p: GPtr<u8>) -> GPtr<u8> = cdecl 0x5F7E80 => unported;
    /// `CPed::SetCharCreatedBy(uint8)` (0x5E47E0)
    fn set_char_created_by(p: GPtr<u8>, by: u32) = thiscall 0x5E47E0 => unported;
    /// `CAutomobile::TellHeliToGoToCoors(float, float, float, float, float)` (0x6A2390); raw floats
    fn tell_heli_to_go_to_coors(v: GPtr<u8>, x: u32, y: u32, z: u32, a: u32, b: u32) = thiscall 0x6A2390 => unported;
    /// `CPlane::IsAlreadyFlying()` (0x6CAB90), AL (ignored)
    fn plane_is_already_flying(v: GPtr<u8>) -> u8 = thiscall 0x6CAB90 => unported;
    /// `CPedType::GetPedFlag(ePedType)` (0x608830)
    fn get_ped_flag(t: u32) -> u32 = cdecl 0x608830 => unported;
    /// `CPedType::SetPedTypeAsAcquaintance(int32, ePedType, int32)` (0x608E20) / `ClearPedTypeAsAcquaintance`
    /// (0x6089F0)
    fn set_ped_type_as_acquaintance(a: u32, t: u32, f: u32) = cdecl 0x608E20 => unported;
    fn clear_ped_type_as_acquaintance(a: u32, t: u32, f: u32) = cdecl 0x6089F0 => unported;
    /// `CRGBA::CRGBA(uint8, uint8, uint8, uint8)` (0x7170C0), returns this
    fn rgba_ctor(c: GPtr<u8>, r: u32, g: u32, b: u32, a: u32) -> GPtr<u8> = thiscall 0x7170C0 => unported;
    /// 0x463370 (thiscall on the rectangle's colour, `const CRGBA*`; a `.HOODLUM` stub): copies the colour
    fn rgba_set_463370(c: GPtr<u8>, from: GPtr<u8>) = thiscall 0x463370 => unported;
    /// `CTaskComplexUseEffect::CTaskComplexUseEffect(C2dEffect*, CEntity*)` (0x6321F0)
    fn task_use_effect_ctor(t: GPtr<u8>, effect: u32, entity: u32) -> GPtr<u8> = thiscall 0x6321F0 => unported;
    /// `CTaskSimpleGunControl::CTaskSimpleGunControl(CEntity*, const CVector*, const CVector*, int8, int16,
    /// int32)` (0x61F3F0)
    fn task_gun_control_ctor(t: GPtr<u8>, target: GPtr<u8>, a: u32, b: u32, c: u32, d: u32, e: u32) -> GPtr<u8> = thiscall 0x61F3F0 => unported;
    /// `CPedIntelligence::IsRespondingToEvent(eEventType)` (0x600DB0), AL
    fn is_responding_to_event(intel: GPtr<u8>, ev: u32) -> u8 = thiscall 0x600DB0 => unported;
    /// `CTaskComplexFleeAnyMeans::CTaskComplexFleeAnyMeans(CEntity*, bool, float, int32, int32, float, float,
    /// float, float)` (0x65CC60); floats raw
    #[allow(clippy::too_many_arguments)]
    fn task_flee_any_means_ctor(t: GPtr<u8>, target: GPtr<u8>, b: u32, a2: u32, a3: u32, a4: u32, a5: u32, a6: u32, a7: u32, a8: u32) -> GPtr<u8> = thiscall 0x65CC60 => unported;
    /// 0x46A340 (thiscall on the 0x28-byte patrol-point anim record, `(const char* anim, const char* block)`;
    /// a `.HOODLUM` stub: strcpy of both names to +0 and +0x18)
    fn patrol_anim_ctor(rec: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) = thiscall 0x46A340 => unported;
    /// 0x46AE80 (thiscall on the patrol route 0xC18DB8, `(const CVector*, const record*)`, AL ignored; a
    /// `.HOODLUM` stub: appends a point)
    fn patrol_route_add(r: GPtr<u8>, pos: GPtr<u32>, rec: GPtr<u8>) -> u8 = thiscall 0x46AE80 => unported;
    /// `CShopping::LoadPrices(const char*)` (0x49B8D0) / `LoadShop(const char*)` (0x49BBE0)
    fn load_prices(s: GPtr<u8>) = cdecl 0x49B8D0 => unported;
    fn load_shop(s: GPtr<u8>) = cdecl 0x49BBE0 => unported;
    /// `CShopping::GetPrice(uint32)` (0x49AD50)
    fn get_price(i: u32) -> u32 = cdecl 0x49AD50 => unported;
    /// `CTaskComplexDie::CTaskComplexDie(eWeaponType, AssocGroupId, AnimationId, float, float, bool, bool,
    /// eFallDir, bool)` (0x630040); floats raw
    #[allow(clippy::too_many_arguments)]
    fn task_die_ctor(t: GPtr<u8>, a: u32, b: u32, c: u32, d: u32, e: u32, f: u32, g: u32, h: u32, i: u32) -> GPtr<u8> = thiscall 0x630040 => unported;
    /// `CVehicle::SetVehicleCreatedBy(int32)` (0x6D5D70)
    fn set_vehicle_created_by(v: GPtr<u8>, by: u32) = thiscall 0x6D5D70 => unported;
    /// `CTheZones::FindZoneByLabel(const char*, eZoneType)` (0x572C40), AX
    fn find_zone_by_label(label: GPtr<u8>, ty: u32) -> u16 = cdecl 0x572C40 => unported;
}

/// `VEH(h)` (`CPool<CVehicle>::GetAtRef`, no null check by the callers).
#[inline]
fn veh(h: u32) -> GPtr<u8> {
    vehicle_at_ref(pool(VEHICLE_POOL), h as i32)
}

/// `PED(h)` (`CPool<CPed>::GetAtRef`).
#[inline]
fn ped(h: u32) -> GPtr<u8> {
    ped_at_ref(pool(PED_POOL), h as i32)
}

/// `OBJ(h)` (`CPool<CObject>::GetAtRef`).
#[inline]
fn obj(h: u32) -> GPtr<u8> {
    object_at_ref(pool(OBJECT_POOL), h as i32)
}

/// The entity position: `m_matrix (+0x14) ? m_matrix + 0x30 : this + 4`.
fn ent_pos(e: GPtr<u8>) -> GPtr<u32> {
    let m = e.byte_add(0x14).cast::<GPtr<u8>>().read();
    if m.is_null() {
        e.byte_add(4)
    } else {
        m.byte_add(0x30)
    }
    .cast::<u32>()
}

/// The mission guard: `m_autoPilot.m_nCarMission` (+0x3BA) is set to `mission` unless it is 0x39 or 0x3A
/// (PLANE/HELI_CRASH_AND_BURN).
fn mission_guard(v: GPtr<u8>, mission: u8) {
    let m = v.byte_add(0x3BA);
    let c = m.read();
    if c != 0x39 && c != 0x3A {
        m.write(mission);
    }
}

/// A model operand: a negative value indexes `CTheScripts::UsedObjectArray` backwards
/// (`imul v,0x1C; mov r,0xA44B88; sub r,v; mov v,[r]`).
#[inline]
fn model_operand(v: u32) -> u32 {
    if (v as i32) < 0 {
        GPtr::<u32>::from_addr(0xA44B88u32.wrapping_sub(v.wrapping_mul(0x1C)) as usize).read()
    } else {
        v
    }
}

/// `task` or null when `operator new` failed (the constructor runs only on a non-null block).
#[inline]
fn new_task(size: u32, ctor: impl FnOnce(GPtr<u8>) -> GPtr<u8>) -> GPtr<u8> {
    let t = task_new(size);
    if t.is_null() {
        GPtr::NULL
    } else {
        ctor(t)
    }
}

/// `CRunningScript::ProcessCommands1800To1899(int32 command)` (0x46D050, thiscall, `ret 4`): AL = 0 continue, 1 wait, -1 unknown.
#[hook(addr = 0x46D050, conv = thiscall, name = "CRunningScript::ProcessCommands1800To1899", expect = "6A FF 64 A1 00 00 00 00", this = "CRunningScript")]
pub fn process_commands_1800_to_1899(s: Scr, command: u32) -> i8 {
    match command {
        // 0711, 0712, 0718, 071B..071D, 0720..0722, 0725, 0728, 0738..073A, 073D, 0740, 0744, 0748, 0758, 0759,
        // 0764..0766: the jump table sends them to 0x46ED92 (`xor al,al`: no operand read, return 0). gr: nop
        // (071C, 0738, 0744, 0758: unimplemented, -1).
        0x0711 | 0x0712 | 0x0718 | 0x071B..=0x071D | 0x0720..=0x0722 | 0x0725 | 0x0728 => {}
        0x0738..=0x073A | 0x073D | 0x0740 | 0x0744 | 0x0748 | 0x0758 | 0x0759 | 0x0764..=0x0766 => {}
        // 0708 CLEAR_CHAR_DECISION_MAKER_EVENT_RESPONSE (0x46D08E) / 0749 CLEAR_GROUP_DECISION_MAKER_EVENT_RESPONSE
        // (0x46E4B4, jumps into 0708's tail): `CollectParameters(2)`; `i = GetActualScriptThingIndex(P0, 7)`; if
        // `0 <= i < 20`: `GetInstance()->FlushDecisionMakerEventResponse(i, P1)`.
        0x0708 | 0x0749 => {
            collect(s, 2);
            let i = get_actual_script_thing_index(sp(0), 7);
            let ev = sp(1);
            if (0..20).contains(&i) {
                flush_dm_event_response(dm_types_instance(), i, ev);
            }
        }
        // 0709 ADD_CHAR_DECISION_MAKER_EVENT_RESPONSE (0x46D0D3) / 074A ADD_GROUP_... (0x46E4CB): see
        // `add_dm_event_response_case`.
        0x0709 | 0x074A => add_dm_event_response_case(s),
        // 070A TASK_PICK_UP_OBJECT (0x46D166): `this->ScriptTaskPickUpObject(command)`
        0x070A => script_task_pick_up_object(s, command),
        // 070B DROP_OBJECT (0x46D175): `CollectParameters(2)`; `p = PED(P0)` (no null check); `b = P1 != 0`; if
        // `p->m_pIntelligence->GetTaskHold(0)`: `GetTaskHold(0)->DropEntity(p, b)` (called twice, as compiled).
        0x070B => {
            collect(s, 2);
            let p = ped(sp(0));
            let b = u32::from(sp(1) != 0);
            if !get_task_hold(p.byte_add(0x47C).cast::<GPtr<u8>>().read(), 0).is_null() {
                let t = get_task_hold(p.byte_add(0x47C).cast::<GPtr<u8>>().read(), 0);
                drop_entity(t, p, b);
            }
        }
        // 070C EXPLODE_CAR_IN_CUTSCENE (0x46D1DB): `v = VEH(P0)`; byte +0x42A |= 0x20; `v->BlowUpCar(nullptr, 1)`
        // (vtable +0xA4).
        0x070C => {
            collect(s, 1);
            let v = veh(sp(0));
            let b = v.byte_add(0x42A);
            b.write(b.read() | 0x20);
            blow_up_car(v, 0, 1);
        }
        // 070D BUILD_PLAYER_MODEL (0x46D21A): `CClothes::RebuildPlayer(CWorld::Players[SP0].m_pPed, false)`;
        // `CReplay::Init()` (player index inlined: `imul SP0,0x190` + 0xB7CD98)
        0x070D => {
            collect(s, 1);
            let ped = player_info(sp(0)).cast::<GPtr<u8>>().read();
            rebuild_player(ped, 0);
            replay_init();
        }
        // 070E PLANE_ATTACK_PLAYER (0x46D24B) / 0724 HELI_ATTACK_PLAYER (0x46D8A7): `CollectParameters(3)`; `v =
        // VEH(P0)`; P1 unused; mission guard -> 0x23 / 0x17; +0x9B0 = P2 (`fld; fstp`); 0724: cruise speed
        // (+0x3D0) = 100.
        0x070E | 0x0724 => {
            collect(s, 3);
            let v = veh(sp(0));
            let p2 = fld_fst(spf(2));
            mission_guard(v, if command == 0x070E { 0x23 } else { 0x17 });
            v.byte_add(0x9B0).cast::<f32>().write(p2);
            if command == 0x0724 {
                v.byte_add(0x3D0).write(100);
            }
        }
        // 070F PLANE_FLY_IN_DIRECTION (0x46D290): `CollectParameters(4)`; `v = VEH(P0)`; `h = P1 * 0.017453292`
        // (x87, kept in a register); mission guard -> 0x24; +0x9B4 = h, +0x9A8 = P3 (raw), +0x9B8 = h, +0x9B0 =
        // P2 (`fld; fstp`).
        0x070F => {
            collect(s, 4);
            let v = veh(sp(0));
            let h = X87::from_f32(spf(1)).fmul_m(f(K_DEG2RAD)).to_f32();
            let p2 = fld_fst(spf(2));
            let p3 = sp(3);
            mission_guard(v, 0x24);
            v.byte_add(0x9B4).cast::<f32>().write(h);
            v.byte_add(0x9A8).cast::<u32>().write(p3);
            v.byte_add(0x9B8).cast::<f32>().write(h);
            v.byte_add(0x9B0).cast::<f32>().write(p2);
        }
        // 0710 PLANE_FOLLOW_ENTITY (0x46D304) / 0726 HELI_FOLLOW_ENTITY (0x46D8F3) / 0727 POLICE_HELI_CHASE_ENTITY
        // (0x46D994): see `follow_entity`.
        0x0710 => follow_entity(s, 0x25, true),
        0x0726 => follow_entity(s, 0x27, false),
        0x0727 => follow_entity(s, 0x28, false),
        // 0713 TASK_DRIVE_BY (0x46D3A7)
        0x0713 => task_drive_by(s, command),
        // 0714 SET_CAR_STAY_IN_SLOW_LANE (0x46D4FA): byte +0x3DB |= 0x10 if P1 != 0, else &= 0xEF.
        // 073B SET_CAR_CAN_GO_AGAINST_TRAFFIC (0x46DF7A): byte +0x3DB &= 0xBF if P1 != 0, else |= 0x40.
        0x0714 | 0x073B => {
            collect(s, 2);
            let b = veh(sp(0)).byte_add(0x3DB);
            let c = b.read();
            let on = sp(1) != 0;
            b.write(match (command == 0x0714, on) {
                (true, true) => c | 0x10,
                (true, false) => c & 0xEF,
                (false, true) => c & 0xBF,
                (false, false) => c | 0x40,
            });
        }
        // 0715 TAKE_REMOTE_CONTROL_OF_CAR (0x46D545): `0x45AD40(VEH(P1))`; P0 (player) unused.
        0x0715 => {
            collect(s, 2);
            take_remote_control(veh(sp(1)));
        }
        // 0716 IS_CLOSEST_OBJECT_OF_TYPE_SMASHED_OR_DAMAGED (0x46D56F)
        0x0716 => closest_object_smashed_or_damaged(s),
        // 0717 START_SETTING_UP_CONVERSATION (0x46D65E): `CConversations::StartSettingUpConversation(PED(P0))`.
        0x0717 => {
            collect(s, 1);
            start_setting_up_conversation(ped(sp(0)));
        }
        // 0719 FINISH_SETTING_UP_CONVERSATION (0x46D689): `CConversations::DoneSettingUpConversation(false)`.
        0x0719 => done_setting_up_conversation(0),
        // 071A IS_CONVERSATION_AT_NODE (0x46D69A): `CollectParameters(1)`; `p = PED(P0)`; 8-byte label (zeroed
        // 16-byte buffer); flag = `IsConversationAtNode(label, p) != 0`.
        0x071A => {
            collect(s, 1);
            let p = ped(sp(0));
            let mut label = [0u8; 16];
            let lp = GPtr::from_raw(label.as_mut_ptr());
            read_label(s, lp, 8);
            set_flag(s, is_conversation_at_node(lp, p) != 0);
        }
        // 071E GET_OBJECT_HEALTH (0x46D6F4): P0 = `_ftol(OBJ(P0)->m_fHealth (+0x154))` (low dword); store 1.
        0x071E => {
            collect(s, 1);
            let h = obj(sp(0)).byte_add(0x154).cast::<f32>().read();
            set_sp(0, X87::from_f32(h).ftol() as u32);
            store(s, 1);
        }
        // 071F SET_OBJECT_HEALTH (0x46D71E): `OBJ(P0)->m_fHealth = (float)P1` (`fild; fstp`).
        0x071F => {
            collect(s, 2);
            let h = X87::from_i32(spi(1)).to_f32();
            obj(sp(0)).byte_add(0x154).cast::<f32>().write(h);
        }
        // 0723 BREAK_OBJECT (0x46D754)
        0x0723 => break_object(s),
        // 0729 TASK_USE_MOBILE_PHONE (0x46DA35)
        0x0729 => task_use_mobile_phone(s, command),
        // 072A TASK_WARP_CHAR_INTO_CAR_AS_DRIVER (0x46DAE8): `v = VEH(P1)`; task (0x1C) 0x6470E0(v, nullptr);
        // `task+0x18 = 1` (not null-checked); `GivePedScriptedTask(P0, task, command)`.
        0x072A => {
            collect(s, 2);
            let p0 = sp(0);
            let v = veh(sp(1));
            let t = new_task(0x1C, |t| task_set_ped_in_as_driver_ctor(t, v, 0));
            t.byte_add(0x18).write(1);
            give_ped_scripted_task(s, p0, t, command);
        }
        // 072B TASK_WARP_CHAR_INTO_CAR_AS_PASSENGER (0x46DB5D): `v = VEH(P1)`; `door = P2 >= 0 ?
        // ComputeTargetDoorToEnterAsPassenger(v, P2) : 0`; task (0x20) 0x646FE0(v, door, nullptr); `task+0x1C = 1`
        // (not null-checked); `GivePedScriptedTask(P0, task, command)`.
        0x072B => {
            collect(s, 3);
            let p0 = sp(0);
            let v = veh(sp(1));
            let door = if spi(2) >= 0 {
                compute_target_door_as_passenger(v, sp(2))
            } else {
                0
            };
            let t = new_task(0x20, |t| task_set_ped_in_as_passenger_ctor(t, v, door, 0));
            t.byte_add(0x1C).write(1);
            give_ped_scripted_task(s, p0, t, command);
        }
        // 072C SWITCH_COPS_ON_BIKES (0x46DBE9): `CStreaming::DisableCopBikes(P0 == 0)`.
        0x072C => {
            collect(s, 1);
            disable_cop_bikes(u32::from(sp(0) == 0));
        }
        // 072D / 072E IS_FLAME_IN_ANGLED_AREA_2D/3D (0x46DC1D): `this->FlameInAngledAreaCheckCommand(command)`.
        0x072D | 0x072E => flame_in_angled_area_check_command(s, command),
        // 072F ADD_STUCK_CAR_CHECK_WITH_WARP (0x46DC2C): `CollectParameters(7)`; `StuckCars.AddCarToCheck(P0, P1,
        // P2, true, P3 != 0, P4 != 0, P5 != 0, (u8)P6)` (the three bools are stack bytes pushed as dwords).
        0x072F => {
            collect(s, 7);
            let (b3, b4, b5) = (
                u32::from(sp(3) != 0),
                u32::from(sp(4) != 0),
                u32::from(sp(5) != 0),
            );
            add_car_to_check(
                GPtr::from_addr(0xA90AB0),
                sp(0),
                sp(1),
                sp(2),
                1,
                b3,
                b4,
                b5,
                sp(6) & 0xFF,
            );
        }
        // 0730 DAMAGE_CAR_PANEL (0x46DCAD) / 073C DAMAGE_CAR_DOOR (0x46DFC5): `v = VEH(P0)`;
        // `v->m_damageManager (+0x5A0).ApplyDamage(v, P1 + 11 / P1 + 5, [0x871640] (150.0, raw), 1.0f)`.
        0x0730 | 0x073C => {
            collect(s, 2);
            let v = veh(sp(0));
            let intensity = GPtr::<u32>::from_addr(0x871640).read();
            let comp = sp(1).wrapping_add(if command == 0x0730 { 11 } else { 5 });
            apply_damage(v.byte_add(0x5A0), v, comp, intensity, 0x3F80_0000);
        }
        // 0731 SET_CAR_ROLL (0x46DCF0): `v = VEH(P0)`; `p` = the entity position (copied); `m_matrix->
        // SetRotateZ((float)GetHeading())`; `m_matrix->RotateY(P1 * 0.017453292)`; `0x46AE00(m_matrix, &p)`.
        0x0731 => {
            collect(s, 2);
            let v = veh(sp(0));
            let p1 = spf(1);
            let pp = ent_pos(v);
            let mut p = [pp.read(), pp.byte_add(4).read(), pp.byte_add(8).read()];
            let h = X87::from_f64_exact(get_heading(v)).to_f32();
            matrix_set_rotate_z(v.byte_add(0x14).cast::<GPtr<u8>>().read(), h.to_bits());
            let a = X87::from_f32(p1).fmul_m(f(K_DEG2RAD)).to_f32();
            matrix_rotate_y(v.byte_add(0x14).cast::<GPtr<u8>>().read(), a.to_bits());
            matrix_add_pos(
                v.byte_add(0x14).cast::<GPtr<u8>>().read(),
                GPtr::from_raw(p.as_mut_ptr()),
            );
        }
        // 0732 SUPPRESS_CAR_MODEL (0x46DD76) / 0733 DONT_SUPPRESS_CAR_MODEL (0x46DD95): `0x46B1A0(P0)` /
        // `0x46A7E0(P0)`.
        0x0732 => {
            collect(s, 1);
            suppress_car_model(sp(0));
        }
        0x0733 => {
            collect(s, 1);
            dont_suppress_car_model(sp(0));
        }
        // 0734 DONT_SUPPRESS_ANY_CAR_MODELS (0x46DDB4): `CTheScripts::SuppressedVehicleModels` (0xA44940, 40
        // dwords) = -1 (`rep stosd`).
        0x0734 => {
            for i in 0..40usize {
                GPtr::<u32>::from_addr(0xA44940 + 4 * i).write(u32::MAX);
            }
        }
        // 0735 IS_PS2_KEYBOARD_KEY_PRESSED (0x46DDCA) / 0736 IS_PS2_KEYBOARD_KEY_JUST_PRESSED (0x46DE0E): the exe
        // has cases (gr: nop): `CollectParameters(1)`; flag = `0x571980 / 0x571970(this = 0xB9B9A4, (u16)P0, 1,
        // 0x859E40) != 0`.
        0x0735 | 0x0736 => {
            collect(s, 1);
            let m = GPtr::from_addr(0xB9B9A4);
            let k = sp(0) & 0xFFFF;
            let r = if command == 0x0735 {
                ps2_key_pressed(m, k, 1, GPtr::from_addr(0x859E40))
            } else {
                ps2_key_just_pressed(m, k, 1, GPtr::from_addr(0x859E40))
            };
            set_flag(s, r != 0);
        }
        // 0737 IS_CHAR_HOLDING_OBJECT (0x46DE3E)
        0x0737 => is_char_holding_object(s),
        // 073E GET_RANDOM_CAR_IN_SPHERE_NO_SAVE (0x46E009)
        0x073E => random_car_in_sphere_no_save(s),
        // 073F GET_RANDOM_CHAR_IN_SPHERE (0x46E172)
        0x073F => random_char_in_sphere(s),
        // 0741 HAS_CHAR_BEEN_ARRESTED (0x46E348): flag = `PED(P0)->m_nPedState (+0x530) == 0x3F` (no null check).
        0x0741 => {
            collect(s, 1);
            set_flag(s, ped(sp(0)).byte_add(0x530).cast::<u32>().read() == 0x3F);
        }
        // 0742 SET_PLANE_THROTTLE (0x46E37A): `VEH(P0)->+0x998 = P1` (raw).
        0x0742 => {
            collect(s, 2);
            let v = veh(sp(0));
            v.byte_add(0x998).cast::<u32>().write(sp(1));
        }
        // 0743 HELI_LAND_AT_COORDS (0x46E3A8): `CAutomobile::TellHeliToGoToCoors(VEH(P0), P1..P5)`; mission guard
        // -> 0x2F.
        0x0743 => {
            collect(s, 6);
            let v = veh(sp(0));
            tell_heli_to_go_to_coors(v, sp(1), sp(2), sp(3), sp(4), sp(5));
            mission_guard(v, 0x2F);
        }
        // 0745 PLANE_STARTS_IN_AIR (0x46E411): `CPlane::IsAlreadyFlying(v)` (result ignored); if `(u16)v->+0x22
        // == 0x208`: `(u16)v->+0x86C = 0`.
        0x0745 => {
            collect(s, 1);
            let v = veh(sp(0));
            plane_is_already_flying(v);
            if v.byte_add(0x22).cast::<u16>().read() == 0x208 {
                v.byte_add(0x86C).cast::<u16>().write(0);
            }
        }
        // 0746 SET_RELATIONSHIP (0x46E450) / 0747 CLEAR_RELATIONSHIP (0x46E482): `CollectParameters(3)`;
        // `Set/ClearPedTypeAsAcquaintance(P0, P1, GetPedFlag(P2))`.
        0x0746 | 0x0747 => {
            collect(s, 3);
            let (a, t) = (sp(0), sp(1));
            let fl = get_ped_flag(sp(2));
            if command == 0x0746 {
                set_ped_type_as_acquaintance(a, t, fl);
            } else {
                clear_ped_type_as_acquaintance(a, t, fl);
            }
        }
        // 074B DRAW_SPRITE_WITH_ROTATION (0x46E55E)
        0x074B => draw_sprite_with_rotation(s),
        // 074C TASK_USE_ATTRACTOR (0x46E675): `i = GetActualScriptThingIndex(P1, 6)`; if `0 <= i < 64`: task
        // (0x24) `CTaskComplexUseEffect(0xC3AB00 + i * 0x40, nullptr)`, `GivePedScriptedTask(P0, ...)`.
        0x074C => {
            collect(s, 2);
            let p0 = sp(0);
            let i = get_actual_script_thing_index(sp(1), 6);
            if (0..64).contains(&i) {
                let e = 0xC3AB00u32.wrapping_add((i as u32) << 6);
                let t = new_task(0x24, |t| task_use_effect_ctor(t, e, 0));
                give_ped_scripted_task(s, p0, t, command);
            }
        }
        // 074D TASK_SHOOT_AT_CHAR (0x46E6FF): `target = PED(P1)`; task (0x3C) `CTaskSimpleGunControl(target,
        // nullptr, nullptr, 3, 1, P2)`; `GivePedScriptedTask(P0, ...)`.
        0x074D => {
            collect(s, 3);
            let (p0, p2) = (sp(0), sp(2));
            let target = ped(sp(1));
            let t = new_task(0x3C, |t| task_gun_control_ctor(t, target, 0, 0, 3, 1, p2));
            give_ped_scripted_task(s, p0, t, command);
        }
        // 074E SET_INFORM_RESPECTED_FRIENDS (0x46E77C): `intel = PED(P0)->m_pIntelligence`; intel+0xC8 = P1,
        // intel+0xC4 = P2 (raw).
        0x074E => {
            collect(s, 3);
            let p2 = sp(2);
            let intel = ped(sp(0)).byte_add(0x47C).cast::<GPtr<u8>>().read();
            intel.byte_add(0xC8).cast::<u32>().write(sp(1));
            intel.byte_add(0xC4).cast::<u32>().write(p2);
        }
        // 074F IS_CHAR_RESPONDING_TO_EVENT (0x46E7BC): flag = the AL of `PED(P0)->m_pIntelligence->
        // IsRespondingToEvent(P1)` (the byte as is).
        0x074F => {
            collect(s, 2);
            let ev = sp(1);
            let intel = ped(sp(0)).byte_add(0x47C).cast::<GPtr<u8>>().read();
            let r = is_responding_to_event(intel, ev);
            flag(s, u32::from(r));
        }
        // 0750 SET_OBJECT_VISIBLE (0x46E802): `OBJ(SP0)` +0x1C bit 7 (bIsVisible) = SP1 != 0 (no null check)
        0x0750 => {
            collect(s, 2);
            let o = obj(sp(0));
            let fl = o.byte_add(0x1C).cast::<u32>();
            let v = fl.read();
            fl.write(if sp(1) != 0 { v | 0x80 } else { v & 0xFFFF_FF7F });
        }
        // 0751 TASK_FLEE_CHAR_ANY_MEANS (0x46E84A)
        0x0751 => task_flee_char_any_means(s, command),
        // 0752 STOP_RECORDING_CAR (0x46E91A): the exe reads one operand (gr: nop).
        0x0752 => collect(s, 1),
        // 0754 FLUSH_PATROL_ROUTE (0x46E92A): the patrol route's point count (0xC18DB8) = 0.
        0x0754 => GPtr::<u32>::from_addr(PATROL_ROUTE).write(0),
        // 0755 EXTEND_PATROL_ROUTE (0x46E93B)
        0x0755 => extend_patrol_route(s),
        // 075A PLAY_OBJECT_ANIM (0x46E9FC)
        0x075A => play_object_anim(s),
        // 075B SET_RADAR_ZOOM (0x46EAE7): `CTheScripts::RadarZoomValue` (0xA444A3) = (u8)P0.
        0x075B => {
            collect(s, 1);
            GPtr::<u8>::from_addr(0xA444A3).write(sp(0) as u8);
        }
        // 075C DOES_BLIP_EXIST (0x46EB03): flag = `GetActualBlipArrayIndex(P0) != -1`.
        0x075C => {
            collect(s, 1);
            set_flag(s, get_actual_blip_array_index(spi(0)) != -1);
        }
        // 075D LOAD_PRICES (0x46EB37) / 075E LOAD_SHOP (0x46EB59): 16-byte label (zeroed 17-byte buffer);
        // `CShopping::LoadPrices / LoadShop(label)`.
        0x075D | 0x075E => {
            let mut label = [0u8; 17];
            let lp = GPtr::from_raw(label.as_mut_ptr());
            read_label(s, lp, 0x10);
            if command == 0x075D {
                load_prices(lp);
            } else {
                load_shop(lp);
            }
        }
        // 075F GET_NUMBER_OF_ITEMS_IN_SHOP (0x46EB7B): P0 = `CShopping::ms_numItemsInShop` (0xA9A7F0); store 1.
        0x075F => {
            set_sp(0, GPtr::<u32>::from_addr(0xA9A7F0).read());
            store(s, 1);
        }
        // 0760 GET_ITEM_IN_SHOP (0x46EB97): P0 = `ms_shopContents[P0]` (0xA9A318, no bounds check); store 1.
        0x0760 => {
            collect(s, 1);
            let v = GPtr::<u32>::from_addr(0xA9A318u32.wrapping_add(sp(0).wrapping_mul(4)) as usize).read();
            set_sp(0, v);
            store(s, 1);
        }
        // 0761 GET_PRICE_OF_ITEM (0x46EBAF): P0 = `CShopping::GetPrice(P0)`; store 1.
        0x0761 => {
            collect(s, 1);
            let v = get_price(sp(0));
            set_sp(0, v);
            store(s, 1);
        }
        // 0762 TASK_DEAD (0x46EBDB): task (0x28) `CTaskComplexDie(0, 0, 0xF, 4.0f, 0.0f, 0, 0, 0, 0)`;
        // `GivePedScriptedTask(P0, ...)` (P0 read after the constructor).
        0x0762 => {
            collect(s, 1);
            let t = new_task(0x28, |t| task_die_ctor(t, 0, 0, 0xF, 0x4080_0000, 0, 0, 0, 0, 0));
            give_ped_scripted_task(s, sp(0), t, command);
        }
        // 0763 SET_CAR_AS_MISSION_CAR (0x46EC43): `v = VEH(P0)`, `h = GetRef(v)`; if the script uses the mission
        // cleanup and `v->m_nCreatedBy (+0x4A4)` is 1 or 3: `SetVehicleCreatedBy(2)`, cleanup add (h, 1).
        0x0763 => {
            collect(s, 1);
            let v = veh(sp(0));
            let h = vehicle_get_ref(pool(VEHICLE_POOL), v);
            if uses_mission_cleanup(s) {
                let by = v.byte_add(0x4A4).read();
                if by == 1 || by == 3 {
                    set_vehicle_created_by(v, 2);
                    mission_cleanup_add(GPtr::from_addr(MISSION_CLEANUP), h, 1);
                }
            }
        }
        // 0767 SET_ZONE_POPULATION_TYPE (0x46ECA2) / 076A SET_ZONE_DEALER_STRENGTH (0x46ED05) / 076B
        // GET_ZONE_DEALER_STRENGTH (0x46ED4F): see `zone_info_case`.
        0x0767 | 0x076A | 0x076B => zone_info_case(s, command),
        _ => return -1,
    }
    0
}

/// 0709 ADD_CHAR_DECISION_MAKER_EVENT_RESPONSE (0x46D0D3) / 074A ADD_GROUP_DECISION_MAKER_EVENT_RESPONSE
/// (0x46E4CB): `CollectParameters(9)`; `i = GetActualScriptThingIndex(P0, 7)`; float[4] = {P6, P5, P3, P4},
/// int[2] = {P8, P7} (the store order of the bytes); if `0 <= i < 20`: `GetInstance()->AddEventResponse(i, P1,
/// P2, floats, ints)` (the callee reads 4 floats and 2 ints).
fn add_dm_event_response_case(s: Scr) {
    collect(s, 9);
    let i = get_actual_script_thing_index(sp(0), 7);
    let (ev, task) = (sp(1), sp(2));
    let mut floats = [sp(6), sp(5), sp(3), sp(4)];
    let mut ints = [sp(8), sp(7)];
    if !(0..20).contains(&i) {
        return;
    }
    add_dm_event_response(
        dm_types_instance(),
        i,
        ev,
        task,
        GPtr::from_raw(floats.as_mut_ptr()),
        GPtr::from_raw(ints.as_mut_ptr()),
    );
}

/// 0710 PLANE_FOLLOW_ENTITY (0x46D304) / 0726 HELI_FOLLOW_ENTITY (0x46D8F3) / 0727 POLICE_HELI_CHASE_ENTITY
/// (0x46D994): `CollectParameters(4)`; `v = VEH(P0)`; `t = 0`; `P1 >= 0` -> `t = PED(P1)`; `P2 >= 0` -> `t =
/// VEH(P2)`; mission guard -> `mission`; `field = v->m_autoPilot.m_pTargetCar (+0x41C)`: old non-null ->
/// `old->CleanUpOldReference(&field)`; `field = t`; `t->RegisterReference(&field)` (not null-checked: both
/// handles < 0 call it with this = 0, kept); 0710: `0x41BDD0(v, 1)` (SetEngineOn); +0x9B0 = P3 (raw); 0726/0727:
/// cruise speed (+0x3D0) = 100.
fn follow_entity(s: Scr, mission: u8, plane: bool) {
    collect(s, 4);
    let v = veh(sp(0));
    let mut t = GPtr::NULL;
    if spi(1) >= 0 {
        t = ped(sp(1));
    }
    if spi(2) >= 0 {
        t = veh(sp(2));
    }
    let p3 = sp(3);
    mission_guard(v, mission);
    let field = v.byte_add(0x41C);
    let old = field.cast::<GPtr<u8>>().read();
    if !old.is_null() {
        clean_up_old_reference(old, field);
    }
    field.cast::<GPtr<u8>>().write(t);
    register_reference(t, field);
    if plane {
        set_engine_on(v, 1);
    }
    v.byte_add(0x9B0).cast::<u32>().write(p3);
    if !plane {
        v.byte_add(0x3D0).write(100);
    }
}

/// 0x0713 TASK_DRIVE_BY (0x46D3A7): `CollectParameters(10)`: SP0 ped, SP1 target ped, SP2 target car, SP3..SP5 coords,
/// SP6 abort range, SP7 style, SP8 seat RHS, SP9 frequency.
/// - `target = 0`, `tPed = 0`; `SP1 >= 0` -> `target = tPed = PED(SP1)`; `SP2 >= 0` -> `target = VEH(SP2)`.
/// - If `SP0 != -1`: `cur = PED(SP0)->m_pIntelligence (+0x47C) +0x10` (no null check of the ped); if `cur` and
///   `cur->GetTaskType() == 0x3FE` and `cur+0x34 == tPed`: nothing is given (AL = 0).
/// - `t = new(0x44)`; ctor 0x6217D0(target, &(SP3..SP5), SP6, SP9, style, SP8 != 0); `t+0xE = 1` (t not null-checked);
///   `GivePedScriptedTask(SP0, t, command)`.
///
/// The style and seat dwords are stack slots whose low byte only is written (`mov [esp+0x20],dl` /
/// `mov [esp+0x14],cl`); the callee takes them as byte arguments, so the port passes the byte zero-extended
/// (`(u8)SP7`, 0/1). The SEH state writes are not ported.
fn task_drive_by(s: Scr, command: u32) {
    collect(s, 10);
    let ped = sp(0);
    let pos = [sp(3), sp(4), sp(5)];
    let (range, style, rhs, freq) = (sp(6), sp(7) & 0xFF, u32::from(sp(8) != 0), sp(9));
    let mut target = GPtr::NULL;
    let mut t_ped = GPtr::NULL;
    if spi(1) >= 0 {
        t_ped = ped_at_ref(pool(PED_POOL), spi(1));
        target = t_ped;
    }
    if spi(2) >= 0 {
        target = vehicle_at_ref(pool(VEHICLE_POOL), spi(2));
    }
    if ped as i32 != -1 {
        let p = ped_at_ref(pool(PED_POOL), ped as i32);
        let cur = p
            .byte_add(0x47C)
            .cast::<GPtr<u8>>()
            .read()
            .byte_add(0x10)
            .cast::<GPtr<u8>>()
            .read();
        if !cur.is_null() {
            let f = cur.cast::<GPtr<u32>>().read().byte_add(0x10).read();
            if vcall_task_type(f, cur) == 0x3FE && cur.byte_add(0x34).cast::<GPtr<u8>>().read() == t_ped {
                return;
            }
        }
    }
    let t = task_new(0x44);
    let t = if t.is_null() {
        GPtr::NULL
    } else {
        task_drive_by_ctor(
            t,
            target,
            GPtr::from_raw(pos.as_ptr().cast_mut()),
            range,
            freq,
            style,
            rhs,
        )
    };
    t.byte_add(0xE).write(1);
    give_ped_scripted_task(s, ped, t, command);
}

/// 0716 IS_CLOSEST_OBJECT_OF_TYPE_SMASHED_OR_DAMAGED (0x46D56F): `CollectParameters(7)`: pos P0..P2, radius P3,
/// model P4, check smashed P5, check damaged P6. z: `fld P2; fcom -100` -> ground z when `z <= -100` (ordered;
/// `fstp st0` then `FindGroundZForCoord(x, y)`), stored with `fstp` (f32). Model < 0 -> used-object model. `o =
/// FindNearestObjectOfType(model, &pos, P3, 0, 0, 0, 0, 1, 1)`; flag = 0; if `o` and `(o+0x36 & 7) == 4`: `o+0x140
/// & 0x400` and P5 -> 1; `0x46A2F0(o) != 0` and P6 -> 1.
fn closest_object_smashed_or_damaged(s: Scr) {
    collect(s, 7);
    let (x, y) = (sp(0), sp(1));
    let z = if X87::from_f32(spf(2))
        .fcom(X87::from_f32(f(K_MINUS_100)))
        .test_parity(0x41)
    {
        fld_fst(spf(2))
    } else {
        X87::from_f64_exact(find_ground_z(x, y)).to_f32()
    };
    let radius = sp(3);
    let model = model_operand(sp(4));
    let mut pos = [x, y, z.to_bits()];
    let o = find_nearest_object_of_type(model, GPtr::from_raw(pos.as_mut_ptr()), radius, 0, 0, 0, 0, 1, 1);
    let mut r = false;
    if !o.is_null() && o.byte_add(0x36).read() & 7 == 4 {
        if o.byte_add(0x140).cast::<u32>().read() & 0x400 != 0 && sp(5) != 0 {
            r = true;
        }
        if object_is_damaged(o) != 0 && sp(6) != 0 {
            r = true;
        }
    }
    set_flag(s, r);
}

/// 0723 BREAK_OBJECT (0x46D754): `CollectParameters(2)`; `o = OBJ(P0)` (no null check).
/// - `CGlass::IsObjectGlass(o)`: `WindowRespondsToCollision(o, 99999.9f (0x47C34FF3), CVector(0, 0, 0),
///   position, false)`; `o+0x140 |= 0x400`.
/// - Else: `info = o->m_pObjectInfo (+0x160)`; `v = info+0x38..+0x40`; `g_breakMan (0xBB4240).Add(o, &v,
///   info+0x44, P1 != 0)`; `o+0x1C &= 0xFFFFFF7E`; `!GetIsStatic(o)` -> `RemoveFromMovingList(o)`; `+0x40 |=
///   0x800000`, `+0x140 |= 0x400`, `+0x1C |= 4`, `+0x44..+0x58 = 0`; `o->vtable[0x20/4]()`.
fn break_object(s: Scr) {
    collect(s, 2);
    let o = obj(sp(0));
    if is_object_glass(o) != 0 {
        let p = ent_pos(o);
        let (px, py, pz) = (p.read(), p.byte_add(4).read(), p.byte_add(8).read());
        window_responds_to_collision(o, 0x47C3_4FF3, 0, 0, 0, px, py, pz, 0);
        let fl = o.byte_add(0x140).cast::<u32>();
        fl.write(fl.read() | 0x400);
        return;
    }
    let info = o.byte_add(0x160).cast::<GPtr<u8>>().read().cast::<u32>();
    let mut v = [
        info.byte_add(0x38).read(),
        info.byte_add(0x3C).read(),
        info.byte_add(0x40).read(),
    ];
    let b = u32::from(sp(1) != 0);
    let f44 = info.byte_add(0x44).read();
    break_manager_add(
        GPtr::from_addr(0xBB4240),
        o,
        GPtr::from_raw(v.as_mut_ptr()),
        f44,
        b,
    );
    let e = o.byte_add(0x1C).cast::<u32>();
    e.write(e.read() & 0xFFFF_FF7E);
    if get_is_static(o) == 0 {
        remove_from_moving_list(o);
    }
    let f40 = o.byte_add(0x40).cast::<u32>();
    let f140 = o.byte_add(0x140).cast::<u32>();
    let (a, b, c) = (f40.read() | 0x80_0000, f140.read() | 0x400, e.read() | 4);
    f40.write(a);
    f140.write(b);
    e.write(c);
    for i in 0..6isize {
        o.byte_add(0x44 + 4 * i).cast::<u32>().write(0);
    }
    entity_vslot_20(o);
}

/// 0x0729 TASK_USE_MOBILE_PHONE (0x46DA35): `CollectParameters(2)`. `SP1 > 0` (signed): `t = new(0x20)` ctor
/// 0x6348A0(-1); `GivePedScriptedTask(SP0, t, command)`. Else `p = PED(SP0)` (no null check); `t =
/// p->m_pIntelligence->m_TaskMgr (+0x47C, +4).FindTaskByType(3, 0x640)`; if `t && t->GetTaskType() == 0x640`:
/// `0x634A40(t, p)`.
fn task_use_mobile_phone(s: Scr, command: u32) {
    collect(s, 2);
    let ped = sp(0);
    if spi(1) > 0 {
        let t = task_new(0x20);
        let t = if t.is_null() {
            GPtr::NULL
        } else {
            task_use_mobile_phone_ctor(t, u32::MAX)
        };
        give_ped_scripted_task(s, ped, t, command);
        return;
    }
    let p = ped_at_ref(pool(PED_POOL), ped as i32);
    let tm = p.byte_add(0x47C).cast::<GPtr<u8>>().read().byte_add(4);
    let t = find_task_by_type(tm, 3, 0x640);
    if t.is_null() {
        return;
    }
    let f = t.cast::<GPtr<u32>>().read().byte_add(0x10).read();
    if vcall_task_type(f, t) == 0x640 {
        phone_task_stop(t, p);
    }
}

/// The held entity of `p` (0737's inner test): `GetTaskHold(0)` non-null and its vtable +0x20 `(p)` non-zero ->
/// `GetTaskHold(0)->+8`; otherwise `held` unchanged.
fn held_entity(p: GPtr<u8>, held: GPtr<u8>) -> GPtr<u8> {
    let intel = |p: GPtr<u8>| p.byte_add(0x47C).cast::<GPtr<u8>>().read();
    if get_task_hold(intel(p), 0).is_null() {
        return held;
    }
    let t = get_task_hold(intel(p), 0);
    if task_vslot_20(t, p) == 0 {
        return held;
    }
    get_task_hold(intel(p), 0).byte_add(8).cast::<GPtr<u8>>().read()
}

/// 0737 IS_CHAR_HOLDING_OBJECT (0x46DE3E): `CollectParameters(2)`; `p = P0 >= 0 ? PED(P0) : 0`; `o = P1 >= 0 ?
/// OBJ(P1) : 0`; `held = 0`; flag = 0.
/// - `p`: `held = held_entity(p)`; flag = `o ? o == held : held != 0`.
/// - No `p`: walks the ped pool from the top slot down (slot accessor 0x4082A0, pool pointer re-read each
///   iteration) while the flag is clear: for a non-null ped `held = 0` then `held_entity`, flag as above; a
///   null slot keeps `held` and skips the test.
fn is_char_holding_object(s: Scr) {
    collect(s, 2);
    let p = if spi(0) >= 0 { ped(sp(0)) } else { GPtr::NULL };
    let o = if spi(1) >= 0 { obj(sp(1)) } else { GPtr::NULL };
    let mut held = GPtr::<u8>::NULL;
    let mut r = false;
    let test = |held: GPtr<u8>| if o.is_null() { !held.is_null() } else { o == held };
    if !p.is_null() {
        held = held_entity(p, held);
        r = test(held);
    } else {
        let mut n = pool(PED_POOL).byte_add(8).cast::<u32>().read();
        while n != 0 && !r {
            n -= 1;
            let q = ped_pool_get_at(pool(PED_POOL), n);
            if q.is_null() {
                continue;
            }
            held = held_entity(q, GPtr::NULL);
            if test(held) {
                r = true;
            }
        }
    }
    set_flag(s, r);
}

/// 073E GET_RANDOM_CAR_IN_SPHERE_NO_SAVE (0x46E009): `CollectParameters(5)`: centre P0..P2, radius P3, model P4.
/// `best = 9999.9f`, `res = -1`; slots from the top down (0x41CC10): non-null, appearance 1 or 2, not
/// `+0x428 & 1`, `(i16)model == P4` or `P4 < 0`, `CanBeDeleted`; `d = Magnitude(pos - centre)` (components
/// `fsub`, stored f32), `fst` to f32; taken when ST0 < radius and `(f32)d < best` (strict, `test ah,5; jp`):
/// `res = GetRef(v)`, `best = d`. P0 = res; store 1.
fn random_car_in_sphere_no_save(s: Scr) {
    collect(s, 5);
    let (cx, cy, cz) = (spf(0), spf(1), spf(2));
    let mut n = pool(VEHICLE_POOL).byte_add(8).cast::<u32>().read();
    let radius = spf(3);
    let mut best = f(K_9999_9);
    let mut res: i32 = -1;
    while n != 0 {
        n -= 1;
        let v = vehicle_pool_get_at(pool(VEHICLE_POOL), n);
        if v.is_null() {
            continue;
        }
        if get_vehicle_appearance(v) != 1 && get_vehicle_appearance(v) != 2 {
            continue;
        }
        if v.byte_add(0x428).read() & 1 != 0 {
            continue;
        }
        let m = i32::from(v.byte_add(0x22).cast::<i16>().read());
        if m != spi(4) && spi(4) >= 0 {
            continue;
        }
        if can_be_deleted(v) == 0 {
            continue;
        }
        let p = ent_pos(v).cast::<f32>();
        let mut d = [
            X87::from_f32(p.read()).fsub_m(cx).to_f32().to_bits(),
            X87::from_f32(p.byte_add(4).read()).fsub_m(cy).to_f32().to_bits(),
            X87::from_f32(p.byte_add(8).read()).fsub_m(cz).to_f32().to_bits(),
        ];
        let mag = X87::from_f64_exact(vector_magnitude(GPtr::from_raw(d.as_mut_ptr())));
        let dist = mag.to_f32();
        if mag.fcom(X87::from_f32(radius)) != Cmp::Less {
            continue;
        }
        if X87::from_f32(dist).fcom(X87::from_f32(best)) != Cmp::Less {
            continue;
        }
        res = vehicle_get_ref(pool(VEHICLE_POOL), v);
        best = dist;
    }
    set_sp(0, res as u32);
    store(s, 1);
}

/// 073F GET_RANDOM_CHAR_IN_SPHERE (0x46E172): `CollectParameters(7)`: centre P0..P2, radius P3, civilian P4,
/// gang P5, criminal P6. `best = 9999.9f`, handle -1; ped slots from the top down (0x4082A0): non-null, random
/// (+0x484 == 1), entity flags +0x1C bit 11 clear, +0x470 bit 3 clear, not `IsPedDead`,
/// `ThisIsAValidRandomPed(+0x598 ped type, P4, P5, P6)`, no group; `d = Magnitude(pos - centre)` as in 073E;
/// taken when ST0 < radius and `(f32)d < best`: handle = GetRef, best = d. A found handle: `SetCharCreatedBy(2)`
/// on GetAtRef(handle), mission ped count (0xC0EC24) + 1, mission cleanup (type 2) when used. P0 = handle;
/// store 1.
fn random_char_in_sphere(s: Scr) {
    collect(s, 7);
    let (cx, cy, cz) = (spf(0), spf(1), spf(2));
    let radius = spf(3);
    let mut n = pool(PED_POOL).byte_add(8).cast::<u32>().read();
    let mut handle: i32 = -1;
    let mut best = f(K_9999_9);
    while n != 0 {
        n -= 1;
        let p = ped_pool_get_at(pool(PED_POOL), n);
        if p.is_null()
            || p.byte_add(0x484).read() != 1
            || p.byte_add(0x1C).cast::<u32>().read() & 0x800 != 0
            || p.byte_add(0x470).read() & 8 != 0
            || script_is_ped_dead(s, p) != 0
        {
            continue;
        }
        let ty = p.byte_add(0x598).cast::<u32>().read();
        if this_is_a_valid_random_ped(s, ty, sp(4), sp(5), sp(6)) == 0 || !get_peds_group(p).is_null() {
            continue;
        }
        let q = ent_pos(p).cast::<f32>();
        let mut d = [
            X87::from_f32(q.read()).fsub_m(cx).to_f32().to_bits(),
            X87::from_f32(q.byte_add(4).read()).fsub_m(cy).to_f32().to_bits(),
            X87::from_f32(q.byte_add(8).read()).fsub_m(cz).to_f32().to_bits(),
        ];
        let mag = X87::from_f64_exact(vector_magnitude(GPtr::from_raw(d.as_mut_ptr())));
        let dist = mag.to_f32();
        if mag.fcom(X87::from_f32(radius)) != Cmp::Less {
            continue;
        }
        if X87::from_f32(dist).fcom(X87::from_f32(best)) != Cmp::Less {
            continue;
        }
        handle = ped_get_ref(pool(PED_POOL), p);
        best = dist;
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

/// 074B DRAW_SPRITE_WITH_ROTATION (0x46E55E): `CollectParameters(10)`: sprite P0, x P1, y P2, w P3, h P4, angle
/// P5, colour P6..P9. `r = IntroRectangles[n]` (`n = u16 [0xA44B5C]`): +0 = 5, +6 (u16) = P0 - 1; x87 (PC=24, the
/// register order of the bytes): `w' = P3 * 0.5` (register), `h' = P4 * 0.5` (`fstp` local), `sx = fild maxWidth *
/// (1/640)` (register), left (+8) = `(x - w') * sx`, `sy = fild maxHeight * (1/448)` (`fstp` local), top (+0xC) =
/// `(y - h') * sy`, right (+0x10) = `(w' + x) * sx`, bottom (+0x14) = `(h' + y) * sy`; +0x18 = `P5 *
/// 0.017453292`; a stack `CRGBA(P6, P7, P8, P9)` copied with 0x463370 into +0x1C; `n += 1`; byte +0x20 = 0.
fn draw_sprite_with_rotation(s: Scr) {
    collect(s, 10);
    let x = X87::from_f32(spf(1));
    let y = X87::from_f32(spf(2));
    let idx = GPtr::<u16>::from_addr(NUM_SCRIPT_RECTS);
    let n = u32::from(idx.read());
    let w2 = X87::from_f32(spf(3)).fmul_m(f(K_HALF));
    let r = GPtr::<u8>::from_addr(INTRO_RECTANGLES.wrapping_add(n * 0x3C) as usize);
    let h2 = X87::from_f32(spf(4)).fmul_m(f(K_HALF)).to_f32();
    let sx = X87::from_i32(GPtr::<i32>::from_addr(RS_MAX_WIDTH).read()).fmul_m(f(K_INV_640));
    let left = x.fsub(w2);
    r.cast::<u32>().write(5);
    r.byte_add(6).cast::<u16>().write(sp(0).wrapping_sub(1) as u16);
    r.byte_add(8).cast::<f32>().write(left.fmul(sx).to_f32());
    let sy = X87::from_i32(GPtr::<i32>::from_addr(RS_MAX_HEIGHT).read())
        .fmul_m(f(K_INV_448))
        .to_f32();
    let top = y.fsub_m(h2).fmul_m(sy).to_f32();
    r.byte_add(0xC).cast::<f32>().write(top);
    let right = w2.fadd(x).fmul(sx).to_f32();
    r.byte_add(0x10).cast::<f32>().write(right);
    let bottom = X87::from_f32(h2).fadd(y).fmul_m(sy).to_f32();
    r.byte_add(0x14).cast::<f32>().write(bottom);
    let angle = X87::from_f32(spf(5)).fmul_m(f(K_DEG2RAD)).to_f32();
    r.byte_add(0x18).cast::<f32>().write(angle);
    let mut col = [0u8; 4];
    let cp = rgba_ctor(
        GPtr::from_raw(col.as_mut_ptr()),
        sp(6) & 0xFF,
        sp(7) & 0xFF,
        sp(8) & 0xFF,
        sp(9) & 0xFF,
    );
    rgba_set_463370(r.byte_add(0x1C), cp);
    idx.write(idx.read().wrapping_add(1));
    r.byte_add(0x20).write(0);
}

/// 0751 TASK_FLEE_CHAR_ANY_MEANS (0x46E84A): `CollectParameters(8)`; `target = PED(P1)`; task (0x54)
/// `CTaskComplexFleeAnyMeans(target, P4 != 0, P2, P3, P5, P6, P7, [0x86F678], [0xC18CF0])` (raw dwords);
/// `GivePedScriptedTask(P0, ...)`. The bool is `setne dl` over EDX = P6 just pushed: the pushed dword is
/// `(P6 & 0xFFFFFF00) | (P4 != 0)`, passed exactly.
fn task_flee_char_any_means(s: Scr, command: u32) {
    collect(s, 8);
    let (p0, p2, p3, p4, p5, p6, p7) = (sp(0), sp(2), sp(3), sp(4), sp(5), sp(6), sp(7));
    let target = ped(sp(1));
    let t = new_task(0x54, |t| {
        let a = GPtr::<u32>::from_addr(0xC18CF0).read();
        let b = GPtr::<u32>::from_addr(0x86F678).read();
        let scream = (p6 & 0xFFFF_FF00) | u32::from(p4 != 0);
        task_flee_any_means_ctor(t, target, scream, p2, p3, p5, p6, p7, b, a)
    });
    give_ped_scripted_task(s, p0, t, command);
}

/// 0755 EXTEND_PATROL_ROUTE (0x46E93B): `CollectParameters(3)` pos; 24-byte anim label, 16-byte block label
/// (zeroed 25/17-byte buffers); if the first 5 bytes of the anim label equal "NONE\0" (0x859E38, `repe cmpsb`): a
/// local 0x28-byte record gets byte 0 = 0 and byte 0x18 = 0; else `0x46A340(&rec, anim, block)`;
/// `0x46AE80(0xC18DB8, &pos, &rec)`. The record starts zeroed (the original's is not cleared; both callees
/// copy only the NUL-terminated names).
fn extend_patrol_route(s: Scr) {
    collect(s, 3);
    let mut pos = [sp(0), sp(1), sp(2)];
    let mut a = [0u8; 25];
    let mut b = [0u8; 17];
    let ap = GPtr::from_raw(a.as_mut_ptr());
    let bp = GPtr::from_raw(b.as_mut_ptr());
    read_label(s, ap, 0x18);
    read_label(s, bp, 0x10);
    let none = GPtr::<u8>::from_addr(NONE_STR);
    let is_none = (0..5isize).all(|i| ap.byte_add(i).read() == none.byte_add(i).read());
    let mut rec = [0u8; 0x28];
    let rp = GPtr::from_raw(rec.as_mut_ptr());
    if is_none {
        rp.write(0);
        rp.byte_add(0x18).write(0);
    } else {
        patrol_anim_ctor(rp, ap, bp);
    }
    patrol_route_add(
        GPtr::from_addr(PATROL_ROUTE),
        GPtr::from_raw(pos.as_mut_ptr()),
        rp,
    );
}

/// 0x075A PLAY_OBJECT_ANIM (0x46E9FC): `CollectParameters(1)`; `obj = OBJ(SP0)` (no null check);
/// `ReadTextLabelFromScript(anim, 24)`; `ReadTextLabelFromScript(block, 16)`; `CollectParameters(3)`: SP0 blend delta
/// (raw f32), SP1 loop, SP2 keep last frame. `a = GetAnimation(anim, GetAnimationBlock(block))`; `flags = (SP1 != 0 ? 2
/// : 0) | (SP2 == 0 ? 8 : 0)`. If `obj->m_pRwObject (+0x18)`: `RpAnimBlendClumpInit` unless initialised;
/// `BlendAnimation(clump, a, flags, delta)`; flag 1. Else flag 0.
///
/// Divergence (not reproducible): the label buffers are stack arrays of 24 and 16 bytes; a name that fills its
/// buffer without a NUL makes the original callees read on into adjacent stack bytes. The port's buffers carry one
/// extra zero byte instead.
fn play_object_anim(s: Scr) {
    collect(s, 1);
    let obj = object_at_ref(pool(OBJECT_POOL), spi(0));
    let mut anim = [0u8; 25];
    let mut block = [0u8; 17];
    let anim_p = GPtr::from_raw(anim.as_mut_ptr());
    let block_p = GPtr::from_raw(block.as_mut_ptr());
    read_label(s, anim_p, 0x18);
    read_label(s, block_p, 0x10);
    collect(s, 3);
    let delta = sp(0);
    let b = get_animation_block(block_p);
    let a = get_animation(anim_p, b);
    let mut flags = if sp(1) != 0 { 2 } else { 0 };
    if sp(2) == 0 {
        flags |= 8;
    }
    let rw = obj.byte_add(0x18).cast::<GPtr<u8>>();
    if rw.read().is_null() {
        set_flag(s, false);
        return;
    }
    if rp_anim_blend_clump_is_initialized(rw.read()) == 0 {
        rp_anim_blend_clump_init(rw.read());
    }
    blend_animation(rw.read(), a, flags, delta);
    set_flag(s, true);
}

/// 0767 SET_ZONE_POPULATION_TYPE (0x46ECA2) / 076A SET_ZONE_DEALER_STRENGTH (0x46ED05): 8-byte zone label
/// (zeroed 16-byte buffer), `CollectParameters(1)`, `z = (i16)FindZoneByLabel(label, 2)`; if `z >= 0`: `info =
/// 0xBA1DEC + 17 * (u16)[0xBA37B4 + 32 * z]` (the zone's `m_nZoneExtraIndexInfo`); 0767: byte info+0x13 = `b ^
/// ((b ^ P0) & 0x1F)` (low 5 bits = P0's); 076A: byte info+0xE = (u8)P0. 076B GET_ZONE_DEALER_STRENGTH
/// (0x46ED4F): label, no operand, no `z >= 0` check (kept); P0 = byte info+0xE; store 1.
fn zone_info_case(s: Scr, command: u32) {
    let mut label = [0u8; 16];
    let lp = GPtr::from_raw(label.as_mut_ptr());
    read_label(s, lp, 8);
    if command != 0x076B {
        collect(s, 1);
    }
    let z = find_zone_by_label(lp, 2) as i16;
    if command != 0x076B && z < 0 {
        return;
    }
    let navi = GPtr::<u16>::from_addr(((i32::from(z) << 5) as u32).wrapping_add(0xBA37B4) as usize).read();
    let info = u32::from(navi) * 0x11;
    match command {
        0x0767 => {
            let b = GPtr::<u8>::from_addr(info.wrapping_add(0xBA1DFF) as usize);
            let c = b.read();
            b.write(c ^ ((c ^ sp(0) as u8) & 0x1F));
        }
        0x076A => GPtr::<u8>::from_addr(info.wrapping_add(0xBA1DFA) as usize).write(sp(0) as u8),
        _ => {
            let v = GPtr::<u8>::from_addr(info.wrapping_add(0xBA1DFA) as usize).read();
            set_sp(0, u32::from(v));
            store(s, 1);
        }
    }
}

pub const HOOKS: &[HookReg] = &[HOOK_PROCESS_COMMANDS_1800_TO_1899];
