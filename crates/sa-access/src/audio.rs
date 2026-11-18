//! Game sounds: one-off mission sounds and front-end sounds.

use sa_core::bindings::audio_report_frontend_event;
use sa_core::mem::GPtr;
use sa_types::script::ScriptArg::{Float as F, Int as I};
use sa_types::{GameError, GameResult, Vec3};

use crate::script::op;
use crate::{Game, View};

/// `CAEScriptAudioEntity::ProcessMissionAudioEvent` (0x4ECCF0), where 018C ends.
const PROCESS_MISSION_AUDIO_EVENT: usize = 0x4ECCF0;
/// `AudioEngine` (`CAudioEngine`).
const AUDIO_ENGINE: usize = 0xB6BC90;
/// The front-end events `frontend_sound` accepts (`eAudioEvents` 1..=0x2E, the `AE_FRONTEND_*` range).
const FRONTEND_EVENTS: core::ops::RangeInclusive<u32> = 1..=0x2E;

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
pub fn frontend_for_mission_event(sound: i32) -> Option<(u32, f32)> {
    let e = (sound as u32 & 0xFFFF) as u16;
    FRONTEND_MISSION_EVENTS.iter().find(|&&(m, _, _)| m == e).map(|&(_, f, s)| (f, s))
}

impl<V: View> Game<V> {
    /// Plays mission sound `sound` at `pos` (`ADD_ONE_OFF_SOUND` 018C: `CAudioEngine::ReportMissionAudioEvent` →
    /// `CAEScriptAudioEntity::ProcessMissionAudioEvent`, 0x4ECCF0). While 0x4ECCF0 is unported, the cases whose
    /// whole body is a front-end event go to the ported `ReportFrontendAudioEvent` directly, as that case does; the
    /// others are reported unsupported (running 018C would stop the game thread).
    pub fn one_off_sound(&mut self, pos: Vec3, sound: i32) -> GameResult<()> {
        use sa_core::orig::registry;
        if registry::is_installed() && registry::name_of(PROCESS_MISSION_AUDIO_EVENT).is_none() {
            if let Some((event, speed)) = frontend_for_mission_event(sound) {
                audio_report_frontend_event(GPtr::from_addr(AUDIO_ENGINE), event, 0.0, speed);
                return Ok(());
            }
            return Err(GameError::Unsupported(
                "one_off_sound (CAEScriptAudioEntity::ProcessMissionAudioEvent 0x4ECCF0 not ported)",
            ));
        }
        self.run_op(op::ADD_ONE_OFF_SOUND, &[F(pos.x), F(pos.y), F(pos.z), I(sound)]).map(drop)
    }

    /// Plays front-end sound `event` (`CAudioEngine::ReportFrontendAudioEvent(event, 0.0, speed)`: menu clicks,
    /// pick-ups, checkpoint and mission sounds).
    pub fn frontend_sound(&mut self, event: u32, speed: f32) -> GameResult<()> {
        if !FRONTEND_EVENTS.contains(&event) || !speed.is_finite() || speed <= 0.0 {
            return Err(GameError::Other(format!("bad front-end sound {event} at speed {speed}")));
        }
        audio_report_frontend_event(GPtr::from_addr(AUDIO_ENGINE), event, 0.0, speed);
        Ok(())
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
