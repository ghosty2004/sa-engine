//! Helpers shared by the PORT-AUDIO modules: raw field access (sa-world's), x87 shorthands, the audio
//! singletons' addresses and the calls several modules make.

pub use sa_core::mem::GPtr;

pub use sa_math::x87::X87;
pub use sa_world::world::common::{at, f32_of, ftol32, g, rd, wr, x, xi};

/// `AudioEngine` (`CAudioEngine`, 0xB6BC90)
pub const AUDIO_ENGINE: usize = 0xB6BC90;
/// `AEAudioHardware` (`CAEAudioHardware`, 0xB5F8B8)
pub const AE_AUDIO_HARDWARE: usize = 0xB5F8B8;
/// `AESoundManager` (`CAESoundManager`, 0xB62CB0)
pub const AE_SOUND_MANAGER: usize = 0xB62CB0;
/// `AERadioTrackManager` (`CAERadioTrackManager`, 0x8CB6F8)
pub const AE_RADIO_TRACK_MANAGER: usize = 0x8CB6F8;
/// `AEAmbienceTrackManager` (`CAEAmbienceTrackManager`, 0x8AC15C)
pub const AE_AMBIENCE_TRACK_MANAGER: usize = 0x8AC15C;
/// `AECutsceneTrackManager` (`CAECutsceneTrackManager`, 0x8AE554)
pub const AE_CUTSCENE_TRACK_MANAGER: usize = 0x8AE554;

/// The audio engine singleton.
#[inline]
pub fn audio_engine() -> GPtr<u8> {
    g(AUDIO_ENGINE)
}

/// The audio hardware singleton.
#[inline]
pub fn audio_hardware() -> GPtr<u8> {
    g(AE_AUDIO_HARDWARE)
}

/// The sound manager singleton.
#[inline]
pub fn sound_manager() -> GPtr<u8> {
    g(AE_SOUND_MANAGER)
}

pub use crate::bindings::ae_random_int;
pub use crate::bindings::ae_resolve_probability;
