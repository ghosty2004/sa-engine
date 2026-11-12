//! Game audio (radio, ambience, one-off sounds) on the native engine: the native backend's `sa_samp_api::audio` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not served by the
//! native engine yet (docs/samp/native-plan.md lists the order).

use sa_samp_api::audio::*;
use sa_samp_api::*;

use sa_samp_api::script::ScriptArg::{Float as F, Int as I};

use sa_core::mem::GPtr;

use crate::script::op;
use crate::{NativeGame, View};

/// `CAEScriptAudioEntity::ProcessMissionAudioEvent` (0x4ECCF0), where 018C ends.
const PROCESS_MISSION_AUDIO_EVENT: usize = 0x4ECCF0;
/// `AudioEngine` (`CAudioEngine`).
const AUDIO_ENGINE: usize = 0xB6BC90;

sa_core::orig! {
    /// `CAudioEngine::ReportFrontendAudioEvent(eAudioEvents, float volumeChange, float speed)` (0x506EA0).
    fn report_frontend_audio_event(this: GPtr<u8>, event: u32, volume_change: f32, speed: f32) = thiscall 0x506EA0 => unported;
}

/// The cases of `ProcessMissionAudioEvent` (0x4ECCF0, the original's switch on `event & 0xFFFF`) whose whole body
/// is `AudioEngine.ReportFrontendAudioEvent(frontend, 0.0, speed)`: (mission event, frontend event, speed). The
/// class-selection arrows 1052 / 1053 are 0x41C / 0x41D.
const FRONTEND_MISSION_EVENTS: [(u16, u32, f32); 17] = [
    (0x41C, 0x28, 1.0),
    (0x41D, 0x0E, 1.0),
    (0x41E, 0x28, 1.0),
    (0x41F, 0x0E, 1.0),
    (0x420, 0x2C, 1.0),
    (0x421, 0x2D, 1.0),
    (0x422, 0x2B, 1.0),
    (0x43B, 0x01, 1.0),
    (0x43C, 0x02, 1.0),
    (0x43D, 0x04, 1.0),
    (0x46D, 0x2E, 1.0),
    (0x46E, 0x10, 1.0),
    (0x471, 0x2B, f32::from_bits(0x3F8F_5C29)),
    (0x472, 0x2B, 1.0),
    (0x473, 0x2B, f32::from_bits(0x3FA1_47AE)),
    (0x47D, 0x2B, 1.0),
    (0x47E, 0x07, 1.0),
];

/// The frontend event 0x4ECCF0 reports for a mission event, if that case does nothing else.
pub(crate) fn frontend_for_mission_event(sound: i32) -> Option<(u32, f32)> {
    let e = (sound as u32 & 0xFFFF) as u16;
    FRONTEND_MISSION_EVENTS
        .iter()
        .find(|&&(m, _, _)| m == e)
        .map(|&(_, f, s)| (f, s))
}

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
        // samp's PlaySound: 018C ADD_ONE_OFF_SOUND (`CAudioEngine::ReportMissionAudioEvent(sound, &pos)` →
        // `CAEScriptAudioEntity::ProcessMissionAudioEvent(sound, pos, null, 0.0, 1.0)`, 0x4ECCF0). While 0x4ECCF0 is
        // unported, the cases whose whole body is a frontend event (the class-selection arrows among them) go to
        // the ported `ReportFrontendAudioEvent` directly, as that case does; the others are reported unsupported
        // (running 018C would stop the game thread).
        use sa_core::orig::registry;
        if registry::is_installed() && registry::name_of(PROCESS_MISSION_AUDIO_EVENT).is_none() {
            if let Some((event, speed)) = frontend_for_mission_event(sound) {
                report_frontend_audio_event(GPtr::from_addr(AUDIO_ENGINE), event, 0.0, speed);
                return Ok(());
            }
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

#[cfg(test)]
mod tests {
    use super::frontend_for_mission_event;

    #[test]
    fn class_selection_arrows_are_frontend_events() {
        assert_eq!(frontend_for_mission_event(1052), Some((0x28, 1.0)));
        assert_eq!(frontend_for_mission_event(1053), Some((0x0E, 1.0)));
        // Only the low 16 bits select the case.
        assert_eq!(frontend_for_mission_event(0x1_041C), Some((0x28, 1.0)));
        // A bank-sound case (0x41B) is not served this way.
        assert_eq!(frontend_for_mission_event(0x41B), None);
    }
}
