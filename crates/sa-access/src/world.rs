//! The game state machine, time, clock, weather, area, gravity, the camera and the keyboard.

use sa_types::camera::CameraState;
use sa_types::script::ScriptArg::Int as I;
use sa_types::{GameResult, GameState, TimeStep};

use crate::addr;
use crate::script::op;
use crate::{Game, View};

/// The `int16` of `CKeyboardState` (`CPad::NewKeyState`) that holds the key with Windows virtual-key code `vk`:
/// `FKeys[12]`, `standardKeys[256]` (indexed by the upper-case character), then the named keys from `esc`.
fn key_state_index(vk: u16) -> Option<u32> {
    const NAMED: u32 = 12 + 256;
    let vk32 = u32::from(vk);
    Some(match vk {
        0x70..=0x7B => vk32 - 0x70,
        0x20 | 0x30..=0x39 | 0x41..=0x5A => 12 + vk32,
        0x1B => NAMED,
        0x2D => NAMED + 1,
        0x2E => NAMED + 2,
        0x24 => NAMED + 3,
        0x23 => NAMED + 4,
        0x21 => NAMED + 5,
        0x22 => NAMED + 6,
        0x26 => NAMED + 7,
        0x28 => NAMED + 8,
        0x25 => NAMED + 9,
        0x27 => NAMED + 10,
        0x0D => NAMED + 18,
        0x61..=0x69 => NAMED + 20 + (vk32 - 0x61),
        0x60 => NAMED + 29,
        0x08 => NAMED + 30,
        0x09 => NAMED + 31,
        0x10 => NAMED + 36,
        0xA0 => NAMED + 34,
        0xA1 => NAMED + 35,
        0xA2 => NAMED + 37,
        0xA3 => NAMED + 38,
        0xA4 => NAMED + 39,
        0xA5 => NAMED + 40,
        _ => return None,
    })
}

impl<V: View> Game<V> {
    /// `gGameState`.
    pub fn game_state(&self) -> GameState {
        GameState::from_raw(self.view.read_i32(addr::GAME_STATE))
    }

    /// The frame's time steps.
    pub fn time_step(&self) -> TimeStep {
        TimeStep {
            clipped: self.view.read_f32(addr::TIME_STEP),
            non_clipped: self.view.read_f32(addr::TIME_STEP_NON_CLIPPED),
        }
    }

    /// Game time in milliseconds (`CTimer::m_snTimeInMilliseconds`).
    pub fn game_time_ms(&self) -> u32 {
        self.view.read_u32(addr::TIME_IN_MS)
    }

    /// The game clock `(hours, minutes)`.
    pub fn clock(&self) -> (u8, u8) {
        (self.view.read_u8(addr::CLOCK_HOURS), self.view.read_u8(addr::CLOCK_MINUTES))
    }

    /// Sets the game clock: the clock bytes, then `SET_TIME_OF_DAY` 00C0 (`CClock::SetGameClock`).
    pub fn set_clock(&mut self, hour: u8, minute: u8) -> GameResult<()> {
        self.view.write(addr::CLOCK_HOURS, &[hour]);
        self.view.write(addr::CLOCK_MINUTES, &[minute]);
        self.run_op(op::SET_TIME_OF_DAY, &[I(i32::from(hour)), I(i32::from(minute))]).map(drop)
    }

    /// The weather `(forced, new, old)` (`CWeather::ForcedWeatherType`, `NewWeatherType`, `OldWeatherType`).
    pub fn weather(&self) -> (i16, i16, i16) {
        let rd = |a| self.view.read_i16(a);
        (rd(addr::WEATHER_FORCED), rd(addr::WEATHER_NEW), rd(addr::WEATHER_OLD))
    }

    /// Sets the gravity (`GAME_GRAVITY`, per step; the original's 0.008).
    pub fn set_gravity(&mut self, g: f32) -> GameResult<()> {
        self.view.write(addr::GAME_GRAVITY, &g.to_le_bytes());
        Ok(())
    }

    /// Sets the game's millisecond timer (`CTimer::m_snTimeInMilliseconds`), the time the game's timed events and
    /// its clock (while it runs) count in.
    pub fn set_game_time(&mut self, ms: u32) -> GameResult<()> {
        self.view.write(addr::TIME_IN_MS, &ms.to_le_bytes());
        Ok(())
    }

    /// Sets the weather at once: forced, new and old type.
    pub fn set_weather(&mut self, weather: u8) -> GameResult<()> {
        let v = u32::from(weather).to_le_bytes();
        for a in [addr::WEATHER_FORCED, addr::WEATHER_NEW, addr::WEATHER_OLD] {
            self.view.write(a, &v);
        }
        Ok(())
    }

    /// Forces the weather the clock blends to.
    pub fn force_weather(&mut self, weather: u8) -> GameResult<()> {
        self.view.write(addr::WEATHER_FORCED, &u32::from(weather).to_le_bytes());
        Ok(())
    }

    /// `GAME_GRAVITY`.
    pub fn gravity(&self) -> f32 {
        self.view.read_f32(addr::GAME_GRAVITY)
    }

    /// `CGame::currArea`.
    pub fn area_raw(&self) -> i32 {
        self.view.read_i32(addr::CURR_AREA)
    }

    /// The camera (`TheCamera`, the active cam `Cams[0]`, `CDraw::ms_fAspectRatio`).
    pub fn camera(&self) -> CameraState {
        use addr::camera as c;
        let v = &self.view;
        CameraState {
            matrix: v.read_matrix(c::MATRIX),
            front: v.read_vec3(c::FRONT),
            source: v.read_vec3(c::SOURCE),
            mode: v.read_u16(c::MODE),
            fov: v.read_f32(c::FOV),
            looking_left: v.read_u8(c::LOOKING_LEFT) != 0,
            looking_right: v.read_u8(c::LOOKING_RIGHT) != 0,
            aspect_ratio: v.read_f32(c::ASPECT_RATIO),
            weapon_mode: v.read_u16(c::WEAPON_MODE),
        }
    }

    /// Whether the key with Windows virtual-key code `vk` is held in the keyboard state the game itself reads this
    /// frame (`CPad::NewKeyState`), not the OS's.
    pub fn key_held(&self, vk: u16) -> bool {
        key_state_index(vk).is_some_and(|i| self.view.read_i16(addr::NEW_KEY_STATE + 2 * i) != 0)
    }
}
