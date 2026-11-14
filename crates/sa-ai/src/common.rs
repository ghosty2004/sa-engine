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

pub use crate::bindings::duck_control_duck_move;
pub use crate::bindings::goto_has_circled_target;
pub use crate::bindings::goto_quit_ik;
pub use crate::bindings::goto_set_up_ik;
pub use crate::bindings::line_up_ped_with_car_ctor;
pub use crate::bindings::ped_set_move_state;
pub use crate::bindings::ped_set_radio_station;
pub use crate::bindings::say_jacked;
pub use crate::bindings::vehicle_is_driver;
pub use sa_core::bindings::assoc_set_current_time;
pub use sa_core::bindings::crt_new;
pub use sa_core::bindings::entity_clean_up_old_reference;
pub use sa_core::bindings::entity_register_reference;
pub use sa_core::bindings::stats_increment_stat;
pub use sa_core::bindings::task_new;
pub use sa_core::bindings::text_get;
pub use sa_stream::bindings::find_player_wanted;
pub use sa_stream::bindings::ped_is_player;
pub use sa_stream::bindings::rand;
pub use sa_world::bindings::add_animation as anim_add_animation;
pub use sa_world::bindings::add_message_q as messages_add_message_q;
pub use sa_world::bindings::event_group_add;
pub use sa_world::bindings::event_new;
pub use sa_world::bindings::intel_get_task_duck;
pub use sa_world::bindings::kob_dtor as event_knock_off_bike_dtor_o;
pub use sa_world::bindings::task_complex_ctor;
pub use sa_world::bindings::vector_magnitude_sqr;
pub use sa_world::bindings::vector_sub;
pub use sa_world::bindings::vehicle_audio_player_about_to_exit_as_driver;
pub use sa_world::bindings::world_get_is_line_of_sight_clear;
orig! {
    /// `CTask::operator delete(void*)` (0x61A5B0): frees a task pool slot.
    pub fn task_delete(p: GPtr<u8>) = cdecl 0x61A5B0 => unported;
    /// `CEvent::operator delete(void*)` (0x4B5630): frees an event pool slot (the scalar deleting
    /// destructors inline the same code).
    pub fn event_delete(p: GPtr<u8>) = cdecl 0x4B5630 => unported;
    /// `CTaskSimple::CTaskSimple()` (0x61A390)
    pub fn task_simple_ctor(t: GPtr<u8>) -> GPtr<u8> = thiscall 0x61A390 => crate::mvpa_ai1::task_simple_ctor;
    /// `CTask::GetTaskType()` (vtable slot 0x10)
    pub fn task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable slot 0x18)
    pub fn task_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskComplex::CreateNextSubTask(CPed*)` (vtable slot 0x20)
    pub fn task_create_next_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x20 => unported;
    /// `CPed::Say(uint16 id, uint32 delay, float probability, bool, bool, bool)` (0x5EFFE0)
    pub fn ped_say(p: GPtr<u8>, id: u32, delay: u32, prob: f32, a: u32, b: u32, c: u32) -> u32 = thiscall 0x5EFFE0 => unported;
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
    /// `CVehicle::GetRideAnimData()` (vtable slot 0xBC)
    pub fn vehicle_get_ride_anim_data(v: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0xBC => unported;
    /// `CAEPedAudioEntity::AddAudioEvent(int32 event, float volume, float speed, CPhysical*, uint8, int32, uint32)` (0x4E2BB0)
    pub fn ped_audio_add_audio_event(a: GPtr<u8>, event: u32, volume: f32, speed: f32, p: GPtr<u8>, b: u32, c: u32, d: u32) = thiscall 0x4E2BB0 => unported;
    /// `SayJacking(CPed* jacker, CPed* jacked, CVehicle*, uint32 offset)` (0x6497F0)
    pub fn say_jacking_o(jacker: GPtr<u8>, jacked: GPtr<u8>, v: GPtr<u8>, offset: u32) -> u8 = cdecl 0x6497F0 => unported;
    /// `CPed::SetMoveAnim()` (vtable slot 0x5C)
    pub fn ped_set_move_anim(p: GPtr<u8>) = thiscall vslot 0x5C => unported;
    /// A virtual destructor called as `delete p` (vtable slot 0: the scalar deleting destructor, flag 1)
    pub fn virtual_delete(p: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
    /// `CGeneral::GetRadianAngleBetweenPoints(float x1, float y1, float x2, float y2)` (0x53CBE0, ST0)
    pub fn general_get_radian_angle_between_points(x1: f32, y1: f32, x2: f32, y2: f32) -> f64 = cdecl 0x53CBE0 => unported;
    /// `CGeneral::LimitRadianAngle(float)` (0x53CB50, ST0)
    pub fn general_limit_radian_angle(a: f32) -> f64 = cdecl 0x53CB50 => unported;
    /// `CEventKnockOffBike::CEventKnockOffBike(CVehicle*, CVector* moveSpeed, CVector* collisionImpactVelocity,
    /// float damageIntensity, float, uint8 knockOffType, uint8 knockOffDirection, int32 time, CPed*, bool isVictimDriver,
    /// bool forceKnockOff)` (0x4AFCF0)
    pub fn event_knock_off_bike_ctor_o(e: GPtr<u8>, veh: GPtr<u8>, speed: GPtr<u8>, impact: GPtr<u8>, f24: u32, f28: u32, b36: u32, b34: u32, time: u32, ped: GPtr<u8>, b1: u32, b2: u32) -> GPtr<u8> = thiscall 0x4AFCF0 => unported;
}
