//! AUDIO subsystem of the 1:1 rebuild of `gta_sa.exe` 1.0 US (PLAN §2 crate `sa-audio`, PORT-AUDIO,
//! `docs/arch/audio-port.md`): the game side of the CAE* audio system — `CAudioEngine`, the
//! `CAEAudioHardware` wrappers, `CAESound` / `CAESoundManager`, the `CAE*AudioEntity` classes, the
//! `CAERadioTrackManager` / ambience / cutscene track logic, `CAEAudioUtility`, `CAEAudioEnvironment`
//! and `CAudioZones`.
//!
//! Same rules as the other engine crates (ADR-001): game state through `sa_core::globals!`/`GPtr`,
//! calls to other catalogued functions through `sa_core::orig!` with the original address, every port
//! registered with `#[hook]` and listed in its module's `HOOKS` ([`hooks::ALL`]). Sound output
//! (DirectSound buffers, the stream thread's decoders) stays original in phase A and is reached through
//! `orig!`. Ported from the 1.0 US bytes; gta-reversed and `docs/re/specs/AUDIO/*.md` are reading aids.
//! Where they differ, the bytes win (e.g. `CAEAudioUtility::GetRandomNumberInRange` 0x4D9C10 is used
//! where gta-reversed calls `CGeneral`, so the rand caller stream matches).
#![no_std]
#![deny(unsafe_code)]
#![recursion_limit = "1024"]

#[cfg(test)]
extern crate std;

pub use sa_core::hook;

pub mod common;
pub mod hooks;

pub mod engine;
pub mod global_weapon_ambient;
pub mod entities;
#[rustfmt::skip] // generated (tools/gen/audio_patterns.py --check compares the files byte for byte)
pub mod gen_bodies;
mod gen_check;
#[rustfmt::skip] // generated (tools/gen/audio_patterns.py --check compares the files byte for byte)
pub mod gen_ctors;
#[rustfmt::skip] // generated (tools/gen/audio_patterns.py --check compares the files byte for byte)
pub mod gen_dtors;
#[rustfmt::skip] // generated (tools/gen/audio_patterns.py --check compares the files byte for byte)
pub mod gen_leaves;
#[rustfmt::skip] // generated (tools/gen/audio_patterns.py --check compares the files byte for byte)
pub mod gen_virtuals;
pub mod hardware;
pub mod ped;
pub mod ped_events;
pub mod port2_a;
pub mod port2_b;
pub mod port3_ent;
pub mod port3_lead;
pub mod port3_veh;
pub mod port4_hw;
pub mod port4_init;
pub mod port4_service;
pub mod radio;
pub mod sound;
pub mod statics;
pub mod utility;
pub mod vehicle;
pub mod audio_data_stream;

pub mod std6_natives;
pub mod generated;
pub mod weather_update_parameters;
pub mod vehicle_add_audio_event;
pub mod player_vehicle_engine;
pub mod bicycle_tyre_volume;
pub mod radio_queue_tracks_native;
pub mod weapon_gun_sounds;
