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
pub use sa_world::bindings::ped_set_move_state;
pub use crate::bindings::ped_set_radio_station;
pub use crate::bindings::say_jacked;
pub use crate::bindings::vehicle_is_driver;
pub use sa_anim::bindings::assoc_set_current_time;
pub use sa_core::bindings::crt_new;
pub use sa_world::bindings::entity_clean_up_old_reference;
pub use sa_core::bindings::entity_register_reference;
pub use sa_world::bindings::stats_increment_stat;
pub use crate::bindings::task_new;
pub use sa_core::bindings::text_get;
pub use sa_world::bindings::find_player_wanted;
pub use sa_stream::bindings::ped_is_player;
pub use sa_stream::bindings::rand;
pub use sa_world::bindings::add_animation as anim_add_animation;
pub use sa_world::bindings::add_message_q as messages_add_message_q;
pub use crate::bindings::event_group_add;
pub use sa_world::bindings::event_new;
pub use sa_world::bindings::intel_get_task_duck;
pub use sa_world::bindings::kob_dtor as event_knock_off_bike_dtor_o;
pub use crate::bindings::task_complex_ctor;
pub use sa_world::bindings::vector_magnitude_sqr;
pub use sa_world::bindings::vector_sub;
pub use sa_world::bindings::vehicle_audio_player_about_to_exit_as_driver;
pub use sa_core::bindings::world_get_is_line_of_sight_clear;
pub use crate::bindings::say_jacking_o;
pub use sa_anim::bindings::assoc_set_finish_callback;
pub use sa_core::bindings::clump_get_association;
pub use sa_core::bindings::general_get_radian_angle_between_points;
pub use sa_core::bindings::general_limit_radian_angle;
pub use sa_world::bindings::anim_blend_animation;
pub use sa_world::bindings::kob_ctor as event_knock_off_bike_ctor_o;
pub use sa_world::bindings::ped_audio_add_audio_event;
pub use sa_world::bindings::ped_say;
pub use crate::bindings::task_simple_ctor;
pub use sa_world::bindings::vector_magnitude_sqr as vector_magnitude_sqr_x;
pub use sa_core::bindings::random_float_in_range as general_random_float_in_range_x;
orig! {
    /// `CTask::operator delete(void*)` (0x61A5B0): frees a task pool slot.
    pub fn task_delete(p: GPtr<u8>) = cdecl 0x61A5B0 => unported;
    /// `CEvent::operator delete(void*)` (0x4B5630): frees an event pool slot (the scalar deleting
    /// destructors inline the same code).
    pub fn event_delete(p: GPtr<u8>) = cdecl 0x4B5630 => unported;
    /// `CTask::GetTaskType()` (vtable slot 0x10)
    pub fn task_get_type(t: GPtr<u8>) -> i32 = thiscall vslot 0x10 => unported;
    /// `CTask::MakeAbortable(CPed*, int32 priority, const CEvent*)` (vtable slot 0x18)
    pub fn task_make_abortable(t: GPtr<u8>, ped: GPtr<u8>, priority: i32, event: GPtr<u8>) -> u8 = thiscall vslot 0x18 => unported;
    /// `CTaskComplex::CreateNextSubTask(CPed*)` (vtable slot 0x20)
    pub fn task_create_next_sub_task(t: GPtr<u8>, ped: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0x20 => unported;
    /// `CVehicle::GetRideAnimData()` (vtable slot 0xBC)
    pub fn vehicle_get_ride_anim_data(v: GPtr<u8>) -> GPtr<u8> = thiscall vslot 0xBC => unported;
    /// `CPed::SetMoveAnim()` (vtable slot 0x5C)
    pub fn ped_set_move_anim(p: GPtr<u8>) = thiscall vslot 0x5C => unported;
    /// A virtual destructor called as `delete p` (vtable slot 0: the scalar deleting destructor, flag 1)
    pub fn virtual_delete(p: GPtr<u8>, flags: u32) = thiscall vslot 0x0 => unported;
}
