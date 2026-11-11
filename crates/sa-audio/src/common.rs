//! Helpers shared by the PORT-AUDIO modules: raw field access (sa-world's), x87 shorthands, the audio
//! singletons' addresses and the calls several modules make.

use sa_core::mem::GPtr;
use sa_core::orig;

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

orig! {
    /// `CAEAudioUtility::GetRandomNumberInRange(int, int)` (0x4D9C10): the audio code's own RNG wrapper
    /// (CRT rand, return address 0x4D9C17); several functions use it where gta-reversed calls `CGeneral`.
    pub fn ae_random_int(min: i32, max: i32) -> i32 = cdecl 0x4D9C10 => crate::utility::get_random_number_in_range_int;
    /// `CAEAudioUtility::ResolveProbability(float)` (0x4D9C80)
    pub fn ae_resolve_probability(p: f32) -> u8 = cdecl 0x4D9C80 => crate::utility::resolve_probability;
}
