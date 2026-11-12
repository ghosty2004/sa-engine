//! Game audio (radio, ambience, one-off sounds) on the original executable: the original backend's `sa_samp_api::audio` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not routed through
//! the adapter yet; the client still does that work in its own modules.

use sa_samp_api::audio::*;
use sa_samp_api::*;

use super::OriginalGame;
use crate::patch::Memory;

impl<M: Memory> GameAudio for OriginalGame<M> {
    fn radio_volume(&self) -> u8 {
        0
    }

    fn stop_radio(&mut self) -> GameResult<()> {
        Err(GameError::Unsupported("GameAudio::stop_radio"))
    }

    fn start_radio(&mut self, _station: i8) -> GameResult<()> {
        Err(GameError::Unsupported("GameAudio::start_radio"))
    }

    fn mission_ambience(&mut self, _track: Option<i32>) -> GameResult<()> {
        Err(GameError::Unsupported("GameAudio::mission_ambience"))
    }

    fn one_off_sound(&mut self, _pos: Vec3, _sound: i32) -> GameResult<()> {
        Err(GameError::Unsupported("GameAudio::one_off_sound"))
    }

    fn set_weather_sound_muted(&mut self, _muted: bool) -> GameResult<()> {
        Err(GameError::Unsupported("GameAudio::set_weather_sound_muted"))
    }
}
