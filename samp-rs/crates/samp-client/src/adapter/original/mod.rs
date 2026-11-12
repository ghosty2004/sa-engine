//! The original backend: `sa_samp_api::GameAdapter` and `SampHost` on the unmodified gta_sa.exe 1.0 US, built
//! from the client's existing address-based code (memory reads and writes through [`Memory`], script commands,
//! byte patches and hooks). Each feature trait has its own file; methods are moved in one feature at a time and
//! must keep the exact reads, writes, calls and order of the code they replace.

mod audio;
mod camera;
/// Camera calls for the client (main thread).
#[cfg(windows)]
pub use camera::calls as cam;
mod config;
mod hud;
mod input;
mod objects;
/// Object calls for the client (main thread).
#[cfg(windows)]
pub use objects::calls as object_calls;
mod peds;
/// Ped calls for the client (main thread).
#[cfg(windows)]
pub use peds::calls as ped_calls;
mod render;
mod script;
mod streaming;
/// Streaming calls for the client (main thread).
#[cfg(windows)]
pub use streaming::calls as stream;
mod vehicles;
/// Vehicle calls for the client (main thread).
#[cfg(windows)]
pub use vehicles::calls as veh_calls;
mod world;
/// World-entity calls for the client (main thread).
#[cfg(windows)]
pub use world::calls as world_calls;

use std::sync::LazyLock;

use sa_samp_api::config::SampModeConfig;
use sa_samp_api::{
    API_VERSION, BackendInfo, BackendKind, GameAdapter, GameError, GameResult, GameState, SampHost, TimeStep, files,
    hooks,
};

use crate::patch::{Memory, PatchError};
use crate::{frametimer, game};

/// A failed read of game memory as an adapter error. Reads fail only for address 0, i.e. a missing object.
fn read_error(e: PatchError) -> GameError {
    match e {
        PatchError::Access { .. } => GameError::InvalidHandle,
        PatchError::Mismatch { addr, .. } | PatchError::NotABranch { addr, .. } => {
            GameError::Mismatch { address: addr }
        }
    }
}

/// A failed write of game memory as an adapter error.
fn write_error(e: PatchError) -> GameError {
    GameError::Other(format!("write failed: {e}"))
}

/// The adapter over the original executable's memory. Over `ProcessMemory` it holds no data, so [`process`] is free
/// and inlines down to the same reads, writes and calls as the code it replaces.
pub struct OriginalGame<M: Memory> {
    mem: M,
}

/// The samp-mode configuration the adapter reports until `SampHost::install` stores the client's.
static DEFAULT_CONFIG: LazyLock<SampModeConfig> = LazyLock::new(SampModeConfig::default);

/// The adapter over this process (the DLL inside gta_sa.exe).
#[cfg(windows)]
#[inline]
pub fn process() -> OriginalGame<crate::patch::ProcessMemory> {
    OriginalGame::new(crate::patch::ProcessMemory)
}

// `process` must stay free: the adapter over the live process carries no state.
#[cfg(windows)]
const _: () = assert!(std::mem::size_of::<OriginalGame<crate::patch::ProcessMemory>>() == 0);

impl<M: Memory> OriginalGame<M> {
    /// An adapter over `mem` (the process memory in the DLL, a fake memory in tests).
    pub fn new(mem: M) -> Self {
        Self { mem }
    }

    /// The memory the adapter reads and writes.
    pub fn memory(&self) -> &M {
        &self.mem
    }
}

impl<M: Memory> GameAdapter for OriginalGame<M> {
    fn backend(&self) -> BackendInfo {
        BackendInfo {
            kind: BackendKind::OriginalExe,
            api_version: API_VERSION,
            build: format!("samp-client {} on gta_sa.exe 1.0 US", env!("CARGO_PKG_VERSION")),
            has_original_layout: true,
        }
    }

    fn game_state(&self) -> GameState {
        match self.mem.read_i32(game::G_GAME_STATE) {
            Ok(v @ 0..=6) => GameState::Starting(v as u8),
            Ok(7) => GameState::FrontendIdle,
            Ok(8) => GameState::LoadingStarted,
            Ok(9) => GameState::Playing,
            Ok(v) => GameState::Other(v),
            Err(_) => GameState::Other(-1),
        }
    }

    fn time_step(&self) -> TimeStep {
        TimeStep {
            clipped: self.mem.read_f32(frametimer::TIME_STEP).unwrap_or(0.0),
            non_clipped: self.mem.read_f32(frametimer::TIME_STEP_NON_CLIPPED).unwrap_or(0.0),
        }
    }

    fn game_time_ms(&self) -> u32 {
        self.mem.read_u32(crate::world::TIMER_MS).unwrap_or(0)
    }

    fn request_quit(&mut self) {}
}

impl<M: Memory> SampHost for OriginalGame<M> {
    fn install(
        &mut self,
        _config: &SampModeConfig,
        _hooks: Box<dyn hooks::GameHooks>,
        _files: Option<Box<dyn files::FileProvider>>,
    ) -> GameResult<()> {
        Err(GameError::Unsupported("SampHost::install"))
    }

    fn start_game(&mut self) -> GameResult<()> {
        Err(GameError::Unsupported("SampHost::start_game"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patch::fake::FakeMemory;

    #[test]
    fn reports_the_original_backend() {
        let game = OriginalGame::new(FakeMemory::new(0x0040_0000, 0x0100_0000));
        let info = game.backend();
        assert_eq!(info.kind, BackendKind::OriginalExe);
        assert_eq!(info.api_version, API_VERSION);
        assert!(info.has_original_layout);
    }

    fn fake() -> OriginalGame<FakeMemory> {
        OriginalGame::new(FakeMemory::new(0x0040_0000, 0x0100_0000))
    }

    #[test]
    fn reads_the_game_state() {
        let mut game = fake();
        for (raw, state) in [
            (5, GameState::Starting(5)),
            (7, GameState::FrontendIdle),
            (8, GameState::LoadingStarted),
            (9, GameState::Playing),
            (42, GameState::Other(42)),
        ] {
            game.mem.set(game::G_GAME_STATE, &i32::to_le_bytes(raw));
            assert_eq!(game.game_state(), state);
        }
    }

    #[test]
    fn reads_the_local_player_focus_and_money() {
        use sa_samp_api::peds::Peds;
        let mut game = fake();
        assert_eq!(game.local_player(), None);
        game.mem.set(game::CWORLD_PLAYERS, &0x0100_0000u32.to_le_bytes());
        game.mem.set(game::PLAYER_IN_FOCUS, &[3]);
        game.mem.set(game::PLAYER_MONEY, &(-25i32).to_le_bytes());
        assert_eq!(game.local_player(), Some(sa_samp_api::PedRef(0x0100_0000)));
        assert_eq!(game.player_in_focus(), sa_samp_api::PlayerSlot(3));
        assert_eq!(game.player_money(), -25);
    }

    #[test]
    fn ped_script_handle_matches_the_pool_lookup() {
        use sa_samp_api::peds::Peds;
        use sa_samp_api::{PedRef, ScriptHandle};
        let mut game = fake();
        let (pool, objects, slots) = (0x0100_0000u32, 0x0100_1000u32, 0x0100_0800u32);
        game.mem.set(game::PED_POOL, &pool.to_le_bytes());
        game.mem.set(pool, &objects.to_le_bytes());
        game.mem.set(pool + 4, &slots.to_le_bytes());
        game.mem.set(pool + 8, &140i32.to_le_bytes());
        game.mem.set(slots + 2, &[0x05]);
        let ped = objects + 2 * game::PED_POOL_ELEMENT;
        assert_eq!(game.ped_script_handle(PedRef(ped)), Ok(ScriptHandle(0x205)));
        assert_eq!(game.ped_script_handle(PedRef(ped + 4)), Err(GameError::InvalidHandle));
        assert_eq!(game.ped_script_handle(PedRef(0)), Err(GameError::InvalidHandle));
    }

    #[test]
    fn reads_ped_models_and_keys() {
        use sa_samp_api::ModelId;
        use sa_samp_api::input::Input;
        use sa_samp_api::peds::Peds;
        let mut game = fake();
        let info = 0x0100_0000u32;
        game.mem.set(game::MODEL_INFO_PTRS + 4 * 7, &info.to_le_bytes());
        game.mem.set(info, &game::VTBL_PED_MODEL_INFO.to_le_bytes());
        assert!(game.is_ped_model(ModelId(7)));
        assert!(!game.is_ped_model(ModelId(8)));
        assert!(!game.is_ped_model(ModelId(-1)));
        game.mem.set(game::NEW_KEY_STATE + game::key::LEFT, &255u16.to_le_bytes());
        assert!(game.game_key_down(game::key::LEFT as u16));
        assert!(!game.game_key_down(game::key::RIGHT as u16));
    }

    #[test]
    fn reads_clock_weather_gravity_and_timer() {
        use sa_samp_api::world::WorldState;
        let mut game = fake();
        game.mem.set(crate::world::CLOCK_HOURS, &[21]);
        game.mem.set(crate::world::CLOCK_MINUTES, &[7]);
        game.mem.set(crate::world::WEATHER_FORCED, &(-1i16).to_le_bytes());
        game.mem.set(crate::world::WEATHER_NEW, &10u16.to_le_bytes());
        game.mem.set(crate::world::WEATHER_OLD, &3u16.to_le_bytes());
        game.mem.set(game::GAME_GRAVITY, &0.008f32.to_le_bytes());
        game.mem.set(frametimer::TIME_STEP, &1.5f32.to_le_bytes());
        game.mem.set(frametimer::TIME_STEP_NON_CLIPPED, &2.5f32.to_le_bytes());
        game.mem.set(crate::world::TIMER_MS, &123_456u32.to_le_bytes());
        assert_eq!(game.clock(), (21, 7));
        assert_eq!(game.weather(), (-1, 10, 3));
        assert_eq!(game.gravity(), 0.008);
        assert_eq!(game.time_step(), TimeStep { clipped: 1.5, non_clipped: 2.5 });
        assert_eq!(game.game_time_ms(), 123_456);
    }

    #[test]
    fn writes_weather_and_gravity_like_samp() {
        use crate::world::{World, Write};
        use sa_samp_api::world::WorldState;
        // The adapter's writes are the ones `World::weather_writes` lists for each clock state.
        for clock_enabled in [false, true] {
            let mut game = fake();
            for a in [crate::world::WEATHER_FORCED, crate::world::WEATHER_NEW, crate::world::WEATHER_OLD] {
                game.mem.set(a, &[0xEE; 4]);
            }
            let w = World { clock_enabled, ..World::default() };
            let r = if clock_enabled { game.force_weather(17) } else { game.set_weather(17) };
            assert_eq!(r, Ok(()));
            let mut expect = fake();
            for a in [crate::world::WEATHER_FORCED, crate::world::WEATHER_NEW, crate::world::WEATHER_OLD] {
                expect.mem.set(a, &[0xEE; 4]);
            }
            for wr in w.weather_writes(17) {
                let Write::Data(a, b) = wr else { panic!("weather writes are data") };
                expect.mem.set(a, &b);
            }
            for a in [crate::world::WEATHER_FORCED, crate::world::WEATHER_NEW, crate::world::WEATHER_OLD] {
                assert_eq!(game.mem.get(a, 4), expect.mem.get(a, 4), "clock {clock_enabled} at {a:#x}");
            }
        }
        let mut game = fake();
        game.set_gravity(0.005).unwrap();
        assert_eq!(game.gravity(), 0.005);
    }

    #[test]
    fn hud_switches_and_menu_flag() {
        use sa_samp_api::hud::Hud;
        let mut game = fake();
        game.set_hud_visible(false).unwrap();
        assert_eq!(game.mem.get(game::MENU_HUD_ON, 1), [0]);
        assert_eq!(game.mem.get(game::HUD_SCRIPT_DONT_DISPLAY_RADAR, 1), [1]);
        game.set_hud_visible(true).unwrap();
        assert_eq!(game.mem.get(game::MENU_HUD_ON, 1), [1]);
        assert_eq!(game.mem.get(game::HUD_SCRIPT_DONT_DISPLAY_RADAR, 1), [0]);
        game.mem.set(0xBA67A4, &[0, 1, 0, 0]);
        assert!(!game.menu_active());
        assert_eq!(game.menu_active_dword(), 0x100);
    }

    #[test]
    fn is_usable_as_a_trait_object() {
        let mut game = OriginalGame::new(FakeMemory::new(0x0040_0000, 0x0100_0000));
        let adapter: &mut dyn GameAdapter = &mut game;
        assert!(matches!(adapter.stop_radio(), Err(GameError::Unsupported(_))));
    }
}
