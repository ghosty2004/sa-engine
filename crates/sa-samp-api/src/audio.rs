//! Game audio samp drives. The internet audio stream (RPC 41/42) is played by samp-rs itself (BASS on
//! Windows); the game only lends its radio volume and stops its radio. samp-rs today: `audio.rs`,
//! `script.rs` (`ADD_ONE_OFF_SOUND`), `vehhooks.rs` (siren/horn). Mapping: `docs/samp/game-adapter.md` §4.8.

use crate::{GameResult, Vec3};

/// Game audio operations.
pub trait GameAudio {
    /// Radio volume setting, 0..=64 (0xB5FCC8), so samp's stream follows it.
    fn radio_volume(&self) -> u8;
    /// Stops the radio (`CAudioEngine::StopRadio` 0x506F70 on `AudioEngine` 0xB6BC90).
    fn stop_radio(&mut self) -> GameResult<()>;
    /// Starts the radio station (`CAudioEngine::StartRadio` 0x507DC0).
    fn start_radio(&mut self, station: i8) -> GameResult<()>;
    /// PlayerPlaySound with a mission-ambience id (`CAEAmbienceTrackManager::PlaySpecialMissionAmbienceTrack`
    /// 0x4D6D50 on 0x8AC15C), or stops it (`StopSpecialMissionAmbienceTrack` 0x4D6D60) when `None`.
    fn mission_ambience(&mut self, track: Option<i32>) -> GameResult<()>;
    /// `ADD_ONE_OFF_SOUND` (018C).
    fn one_off_sound(&mut self, pos: Vec3, sound: i32) -> GameResult<()>;
    /// Mutes or restores the weather sounds while a samp sound plays (samp-rs `install_weather_hook`).
    fn set_weather_sound_muted(&mut self, muted: bool) -> GameResult<()>;
}
