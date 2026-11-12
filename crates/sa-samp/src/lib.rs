//! `sa-samp`: the native sa-engine backend of the SA:MP game adapter (`docs/samp/game-adapter.md`,
//! `docs/samp/native-plan.md`).
//!
//! samp-rs reaches the game only through the `sa_samp_api` traits. On the unmodified `gta_sa.exe` the
//! *original* backend (in samp-rs) implements them with process-memory reads, script commands and byte
//! patches. This crate implements the same traits on the ported engine, for samp-rs linked into native
//! `sa-game` ([`BackendKind::EngineNative`]).
//!
//! # How the engine's state is read
//!
//! Every port keeps the original's 32-bit layouts (ADR-001): native sa-game is a 32-bit process that maps
//! the legacy address arena at the original addresses (`docs/arch/platform.md` §6), and the wasm32 build
//! keeps the same layouts in linear memory (ROADMAP 2026-11-12). Globals therefore live at their 1.0 US
//! addresses and objects in the engine's pools with the original field offsets, which is exactly how the
//! ports themselves read them (`GPtr::from_addr`). The backend reads through a [`View`]: [`EngineView`]
//! is the engine's memory, tests use a fake. Unlike the original backend, no raw address leaves this
//! crate: handles are generational pool references (below) and reads of objects go through a handle that
//! the pool's byte map validates first, so a stale handle is an error instead of a wild read.
//!
//! # Handles
//!
//! [`PedRef`], [`VehicleRef`] and [`ObjectRef`] hold the pool reference the game itself uses for script
//! handles (`CPools::GetPedRef`: `slot << 8 | reuse counter`). A handle stays valid while the slot holds
//! the same object and becomes invalid when the slot is freed or reused. The script handle of an entity is
//! therefore its adapter handle.
//!
//! # Status
//!
//! The read side is served (game state, timer, local player and players, ped/vehicle/object handles, ped
//! and vehicle snapshots, weapons, money, clock, weather, gravity, area, pads, keys, mouse, camera, menu
//! state, screen size). Every write, typed script command and hook dispatch point still returns
//! [`GameError::Unsupported`]; the order in which they come is `docs/samp/native-plan.md` §5.

mod addr;
mod audio;
mod camera;
mod config;
mod hud;
mod input;
mod objects;
mod peds;
mod render;
pub mod script;
mod streaming;
mod vehicles;
mod view;
mod world;

#[cfg(test)]
mod tests;

pub use view::{EngineView, View};

use sa_samp_api::config::SampModeConfig;
use sa_samp_api::hooks::{FramePhase, GameHooks};
use sa_samp_api::{
    API_VERSION, BackendInfo, BackendKind, GameAdapter, GameError, GameResult, GameState, SampHost, TimeStep,
    files,
};

/// The adapter over the native engine.
pub struct NativeGame<V: View> {
    view: V,
    config: SampModeConfig,
    hooks: Option<Box<dyn GameHooks>>,
    files: Option<Box<dyn files::FileProvider>>,
    installed: bool,
    threads: script::Threads,
    /// The text `CMessages::AddBigMessage` points into (GTA keeps the pointer).
    big_message: Box<[u8; 0x200]>,
}

/// The adapter over this process's engine (native sa-game).
pub fn engine() -> NativeGame<EngineView> {
    NativeGame::new(EngineView)
}

impl<V: View> NativeGame<V> {
    /// An adapter over `view` (the engine in sa-game, a fake memory in tests).
    pub fn new(view: V) -> Self {
        Self::with_executor(view, script::engine_executor)
    }

    /// An adapter whose script commands run through `exec` instead of the engine's interpreter (tests).
    pub fn with_executor(view: V, exec: script::CommandExecutor) -> Self {
        Self {
            view,
            config: SampModeConfig::default(),
            hooks: None,
            files: None,
            installed: false,
            threads: script::Threads::new(exec),
            big_message: Box::new([0; 0x200]),
        }
    }

    /// The memory the adapter reads.
    pub fn view(&self) -> &V {
        &self.view
    }

    /// Whether [`SampHost::install`] ran.
    pub fn installed(&self) -> bool {
        self.installed
    }

    /// The client's file provider, for the engine's file layer (samp.saa redirection).
    pub fn files(&self) -> Option<&dyn files::FileProvider> {
        self.files.as_deref()
    }

    /// Runs the client's frame callback for `phase` (the engine's dispatch point calls this). Does nothing
    /// before [`SampHost::install`].
    pub fn dispatch_frame(&mut self, phase: FramePhase) {
        if let Some(mut hooks) = self.hooks.take() {
            hooks.frame(phase, self);
            // A callback cannot reinstall hooks (it only sees `&mut dyn GameAdapter`), so put them back.
            self.hooks = Some(hooks);
        }
    }

    /// Runs the client's shutdown callback once and drops it.
    pub fn shutdown(&mut self) {
        if let Some(mut hooks) = self.hooks.take() {
            hooks.shutdown(self);
        }
    }
}

impl<V: View> GameAdapter for NativeGame<V> {
    fn backend(&self) -> BackendInfo {
        BackendInfo {
            kind: BackendKind::EngineNative,
            api_version: API_VERSION,
            build: format!("sa-engine {} native", env!("CARGO_PKG_VERSION")),
            // Globals sit at their 1.0 US addresses, but there is no original code, so code reads (samp's
            // auth read of 0x4D3AA0) and crash-report addresses mean nothing here.
            has_original_layout: false,
        }
    }

    fn game_state(&self) -> GameState {
        match self.view.read_i32(addr::GAME_STATE) {
            v @ 0..=6 => GameState::Starting(v as u8),
            7 => GameState::FrontendIdle,
            8 => GameState::LoadingStarted,
            9 => GameState::Playing,
            v => GameState::Other(v),
        }
    }

    fn time_step(&self) -> TimeStep {
        TimeStep {
            clipped: self.view.read_f32(addr::TIME_STEP),
            non_clipped: self.view.read_f32(addr::TIME_STEP_NON_CLIPPED),
        }
    }

    fn game_time_ms(&self) -> u32 {
        self.view.read_u32(addr::TIME_IN_MS)
    }

    fn request_quit(&mut self) {
        self.view.write_u32(addr::RS_QUIT, 1);
    }
}

impl<V: View> SampHost for NativeGame<V> {
    fn install(
        &mut self,
        config: &SampModeConfig,
        hooks: Box<dyn GameHooks>,
        files: Option<Box<dyn files::FileProvider>>,
    ) -> GameResult<()> {
        if self.installed {
            return Err(GameError::Unsupported("SampHost::install called twice"));
        }
        // The engine does not read samp mode yet (SAMP-4): the configuration is stored and reported by
        // `SampMode::config`, the ports keep their single-player behaviour.
        self.config = config.clone();
        self.hooks = Some(hooks);
        self.files = files;
        self.installed = true;
        Ok(())
    }

    fn start_game(&mut self) -> GameResult<()> {
        Err(GameError::Unsupported("SampHost::start_game"))
    }
}
