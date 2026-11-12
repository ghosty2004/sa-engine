//! Game audio (radio, ambience, one-off sounds) on the native engine: the native backend's `sa_samp_api::audio` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not served by the
//! native engine yet (docs/samp/native-plan.md lists the order).

use sa_samp_api::audio::*;
use sa_samp_api::*;

use sa_samp_api::script::ScriptArg::{Float as F, Int as I};

use crate::script::op;
use crate::{NativeGame, View};

/// `CAEScriptAudioEntity::ProcessMissionAudioEvent` (0x4ECCF0), where 018C ends.
const PROCESS_MISSION_AUDIO_EVENT: usize = 0x4ECCF0;

impl<V: View> GameAudio for NativeGame<V> {
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

    fn one_off_sound(&mut self, pos: Vec3, sound: i32) -> GameResult<()> {
        // samp's PlaySound: 018C ADD_ONE_OFF_SOUND (`CAudioEngine::ReportMissionAudioEvent(sound, &pos)`). That
        // ends in `CAEScriptAudioEntity::ProcessMissionAudioEvent` (0x4ECCF0); until the engine serves it the
        // command would stop the game thread, so the sound is reported unsupported instead.
        use sa_core::orig::registry;
        if registry::is_installed() && registry::name_of(PROCESS_MISSION_AUDIO_EVENT).is_none() {
            return Err(GameError::Unsupported(
                "GameAudio::one_off_sound (CAEScriptAudioEntity::ProcessMissionAudioEvent 0x4ECCF0 not ported)",
            ));
        }
        self.run_op(op::ADD_ONE_OFF_SOUND, &[F(pos.x), F(pos.y), F(pos.z), I(sound)])
            .map(drop)
    }

    fn set_weather_sound_muted(&mut self, _muted: bool) -> GameResult<()> {
        Err(GameError::Unsupported("GameAudio::set_weather_sound_muted"))
    }
}
