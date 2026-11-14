//! Helpers shared by the AI modules: byte-offset field access and x87 shorthands (re-exported from
//! sa-world's `world::common`), the task and event pools' `operator new`/`operator delete`, and the
//! `orig!` declarations several modules call.

pub use sa_core::mem::GPtr;
pub use sa_math::x87::X87;
pub use sa_world::world::common::{at, f32_of, ftol32, g, rd, wr, x, xi};

use sa_core::orig;

/// `CPools::ms_pTaskPool` (CPool<CTask>, slot 0x80)
pub const TASK_POOL: usize = 0xB744A8;
/// `CPools::ms_pEventPool` (CPool<CEvent>, slot 0x44)
pub const EVENT_POOL: usize = 0xB744AC;

orig! {
    /// `CTask::operator new(uint32 size)` (0x61A5A0): a slot of the task pool (0x80 bytes).
    pub fn task_new(size: u32) -> GPtr<u8> = cdecl 0x61A5A0 => unported;
    /// `CTask::operator delete(void*)` (0x61A5B0): frees a task pool slot.
    pub fn task_delete(p: GPtr<u8>) = cdecl 0x61A5B0 => unported;
    /// `CEvent::operator new(uint32 size)` (0x4B5620): a slot of the event pool (0x44 bytes).
    pub fn event_new(size: u32) -> GPtr<u8> = cdecl 0x4B5620 => unported;
    /// `CEvent::operator delete(void*)` (0x4B5630): frees an event pool slot (the scalar deleting
    /// destructors inline the same code).
    pub fn event_delete(p: GPtr<u8>) = cdecl 0x4B5630 => unported;
    /// CRT `rand()` (0x821B1E); name the original return address with `sa_core::orig::at_site`.
    pub fn rand() -> i32 = cdecl 0x821B1E => sa_core::crt::rand_owned;
    /// `CTaskSimple::CTaskSimple()` (0x61A390)
    pub fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => crate::mvpa_ai1::task_simple_ctor;
    /// `CTaskComplex::CTaskComplex()` (0x61A3B0)
    pub fn task_complex_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A3B0 => unported;
    /// `CEntity::RegisterReference(CEntity**)` (0x571B70, thiscall, `ret 4`)
    pub fn entity_register_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571B70 => unported;
    /// `CEntity::CleanUpOldReference(CEntity**)` (0x571A00, thiscall, `ret 4`)
    pub fn entity_clean_up_old_reference(e: GPtr<u8>, slot: GPtr<u8>) = thiscall 0x571A00 => unported;
    /// `CTask::GetTaskType()` (vtable slot 0x10)
    pub fn task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable slot 0x18)
    pub fn task_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskComplex::CreateNextSubTask(CPed*)` (vtable slot 0x20)
    pub fn task_create_next_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x20 => unported;
    /// `CPed::Say(uint16 id, uint32 delay, float probability, bool, bool, bool)` (0x5EFFE0)
    pub fn ped_say(p: GPtr<u8>, id: u32, delay: u32, prob: f32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
    /// `CText::Get(const char*)` (0x6A0050, `TheText` 0xC1B340)
    pub fn text_get(text: GPtr<u8>, key: GPtr<u8>) -> GPtr<u8> = thiscall 0x6A0050 => unported;
    /// `CMessages::AddMessageQ(const char*, uint32 time, uint16 flags, bool previousBrief)` (0x69F0B0)
    pub fn messages_add_message_q(text: GPtr<u8>, time: u32, flags: u32, prev: u32) = cdecl 0x69F0B0 => unported;
    /// `CStats::IncrementStat(uint16, float)` (0x55C180)
    pub fn stats_increment_stat(id: u32, value: f32) = cdecl 0x55C180 => unported;
    /// `FindPlayerWanted(int32)` (0x56E230)
    pub fn find_player_wanted(id: i32) -> GPtr<u8> = cdecl 0x56E230 => unported;
    /// `CWorld::GetIsLineOfSightClear(const CVector&, const CVector&, bool buildings, vehicles, peds, objects,
    /// dummies, seeThrough, ignoreSomeObjects)` (0x56A490, AL)
    pub fn world_get_is_line_of_sight_clear(a: GPtr<u8>, b: GPtr<u8>, bld: u32, veh: u32, ped: u32, obj: u32, dum: u32, see: u32, cam: u32) -> u8 = cdecl 0x56A490 => unported;
    /// `operator-(CVector* out, const CVector&, const CVector&)` (0x40FE60, cdecl, returns out)
    pub fn vector_sub(out: GPtr<u8>, a: GPtr<u8>, b: GPtr<u8>) -> GPtr<u8> = cdecl 0x40FE60 => unported;
    /// `CVector::SquaredMagnitude()` (0x406DA0, ST0 read as f64)
    pub fn vector_magnitude_sqr(v: GPtr<u8>) -> f64 = thiscall 0x406DA0 => unported;
    /// `CVector::SquaredMagnitude()` (0x406DA0, the full 80-bit ST0)
    pub fn vector_magnitude_sqr_x(v: GPtr<u8>) -> X87 = reg("args = ecx") 0x406DA0 => unported;
    /// `CGeneral::GetRandomNumberInRange(float, float)` (0x41BD90, the full 80-bit ST0; draws its own rand)
    pub fn general_random_float_in_range_x(min: f32, max: f32) -> X87 = reg("args = stack") 0x41BD90 => unported;
    /// `CAnimManager::BlendAnimation(RpClump*, AssocGroupId, AnimationId, float delta)` (0x4D4610)
    pub fn anim_blend_animation(clump: GPtr<u8>, group: u32, id: u32, delta: f32) -> GPtr<u8> = cdecl 0x4D4610 => unported;
    /// `RpAnimBlendClumpGetAssociation(RpClump*, AnimationId)` (0x4D68B0)
    pub fn clump_get_association(clump: GPtr<u8>, id: u32) -> GPtr<u8> = cdecl 0x4D68B0 => unported;
    /// `CAnimBlendAssociation::SetFinishCallback(void (*)(CAnimBlendAssociation*, void*), void*)` (0x4CEBE0)
    pub fn assoc_set_finish_callback(a: GPtr<u8>, cb: u32, data: GPtr<u8>) = thiscall 0x4CEBE0 => unported;
    /// `CPed::IsPlayer()` (0x5DF8F0)
    pub fn ped_is_player(p: GPtr<u8>) -> u8 = thiscall 0x5DF8F0 => unported;
    /// `CPed::SetRadioStation()` (0x5DFD90)
    pub fn ped_set_radio_station(p: GPtr<u8>) = thiscall 0x5DFD90 => unported;
    /// `CVehicle::IsDriver(CPed*)` (0x6D1C40)
    pub fn vehicle_is_driver(v: GPtr<u8>, p: GPtr<u8>) -> u8 = thiscall 0x6D1C40 => unported;
    /// `CVehicle::GetRideAnimData()` (vtable slot 0xBC)
    pub fn vehicle_get_ride_anim_data(v: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0xBC => unported;
    /// `CAEVehicleAudioEntity::PlayerAboutToExitVehicleAsDriver()` (0x4F5BA0)
    pub fn vehicle_audio_player_about_to_exit_as_driver(a: GPtr<u8>) = thiscall 0x4F5BA0 => unported;
    /// `CAEPedAudioEntity::AddAudioEvent(int32 event, float volume, float speed, CPhysical*, uint8, int32, uint32)` (0x4E2BB0)
    pub fn ped_audio_add_audio_event(a: GPtr<u8>, event: u32, volume: f32, speed: f32, p: GPtr<u8>, b: u32, c: u32, d: u32) = thiscall 0x4E2BB0 => unported;
    /// `CEventGroup::Add(CEvent*, bool)` (0x4AB420)
    pub fn event_group_add(group: GPtr<u8>, e: GPtr<u8>, valid: u32) -> GPtr<u8> = thiscall 0x4AB420 => unported;
    /// CRT `operator new(uint32)` (0x82119A)
    pub fn crt_new(size: u32) -> GPtr<u8> = cdecl 0x82119A => unported;
    /// `CTaskUtilityLineUpPedWithCar::CTaskUtilityLineUpPedWithCar(const CVector& offset, int32 time, int32, int32 door)` (0x64FBB0)
    pub fn line_up_ped_with_car_ctor(u: GPtr<u8>, offset: GPtr<u8>, time: u32, a: u32, door: u32) -> GPtr<u8> = thiscall 0x64FBB0 => unported;
    /// `SayJacked(CPed* jacked, CVehicle*, uint32 offset)` (0x6497A0)
    pub fn say_jacked(jacked: GPtr<u8>, v: GPtr<u8>, offset: u32) -> u8 = cdecl 0x6497A0 => unported;
    /// `SayJacking(CPed* jacker, CPed* jacked, CVehicle*, uint32 offset)` (0x6497F0)
    pub fn say_jacking_o(jacker: GPtr<u8>, jacked: GPtr<u8>, v: GPtr<u8>, offset: u32) -> u8 = cdecl 0x6497F0 => unported;
    /// `CPed::SetMoveState(eMoveState)` (0x5DEC00)
    pub fn ped_set_move_state(p: GPtr<u8>, s: u32) = thiscall 0x5DEC00 => unported;
    /// `CPed::SetMoveAnim()` (vtable slot 0x5C)
    pub fn ped_set_move_anim(p: GPtr<u8>) = thiscall vslot 0x5C => unported;
    /// `CPedIntelligence::GetTaskDuck(bool ignoreCheckingForSimplestActiveTask)` (0x6010A0)
    pub fn intel_get_task_duck(i: GPtr<u8>, ignore: u32) -> GPtr<u8> = thiscall 0x6010A0 => unported;
    /// `CTaskSimpleDuck::ControlDuckMove(CVector2D moveSpeed)` (0x6923F0, by value)
    pub fn duck_control_duck_move(d: GPtr<u8>, mx: f32, my: f32) = thiscall 0x6923F0 => unported;
    /// `CAnimBlendAssociation::SetCurrentTime(float)` (0x4CEA80)
    pub fn assoc_set_current_time(a: GPtr<u8>, t: f32) = thiscall 0x4CEA80 => unported;
    /// `CAnimManager::AddAnimation(RpClump*, AssocGroupId, AnimationId)` (0x4D3AA0)
    pub fn anim_add_animation(clump: GPtr<u8>, group: u32, id: u32) -> GPtr<u8> = cdecl 0x4D3AA0 => unported;
    /// A virtual destructor called as `delete p` (vtable slot 0: the scalar deleting destructor, flag 1)
    pub fn virtual_delete(p: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(float x1, float y1, float x2, float y2)` (0x53CBE0, ST0)
    pub fn general_get_radian_angle_between_points(x1: f32, y1: f32, x2: f32, y2: f32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    pub fn general_limit_radian_angle(a: f32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CTaskSimpleGoTo::HasCircledTarget(CPed*)` (0x667A10)
    pub fn goto_has_circled_target(t: GPtr<u8>, ped: GPtr<u8>) -> u8 = thiscall 0x667A10 => unported;
    /// `CTaskSimpleGoTo::QuitIK(CPed*)` (0x667CA0)
    pub fn goto_quit_ik(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x667CA0 => unported;
    /// `CTaskSimpleGoTo::SetUpIK(CPed*)` (0x667AD0)
    pub fn goto_set_up_ik(t: GPtr<u8>, ped: GPtr<u8>) = thiscall 0x667AD0 => unported;
    /// `CEventKnockOffBike::CEventKnockOffBike(CVehicle*, CVector* moveSpeed, CVector* collisionImpactVelocity,
    /// float damageIntensity, float, uint8 knockOffType, uint8 knockOffDirection, int32 time, CPed*, bool isVictimDriver,
    /// bool forceKnockOff)` (0x4AFCF0)
    pub fn event_knock_off_bike_ctor_o(e: GPtr<u8>, veh: GPtr<u8>, speed: GPtr<u8>, impact: GPtr<u8>, f24: u32, f28: u32, b36: u32, b34: u32, time: u32, ped: GPtr<u8>, b1: u32, b2: u32) -> GPtr<u8> = thiscall 0x4AFCF0 => unported;
    /// `CEventKnockOffBike::~CEventKnockOffBike()` (0x4AFE70)
    pub fn event_knock_off_bike_dtor_o(e: GPtr<u8>) = thiscall 0x4AFE70 => unported;
}
