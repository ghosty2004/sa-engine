//! `sa-samp`: the native sa-engine backend of the SA:MP game adapter (`docs/samp/game-adapter.md`,
//! `docs/samp/native-plan.md`).
//!
//! samp-rs reaches the game only through the `sa_samp_api` traits. On the unmodified `gta_sa.exe` the
//! *original* backend (in samp-rs) implements them with process-memory reads, script commands and byte
//! patches. This crate implements the same traits on the ported engine, for samp-rs linked into native
//! `sa-game` ([`BackendKind::EngineNative`]).
//!
//! # How the engine's state is read and changed
//!
//! Through `sa-access`, the engine's own access layer (the mod host uses the same one): [`NativeGame`] holds an
//! `sa_access::Game` and derefs to it, and the trait methods here map samp's calls onto its reads and changes.
//! Handles are its generational pool references (`CPools::GetPedRef`: `slot << 8 | reuse counter`), so the
//! script handle of an entity is its adapter handle.
//!
//! # Status
//!
//! The read side is served (game state, timer, local player and players, ped/vehicle/object handles, ped
//! and vehicle snapshots, weapons, money, clock, weather, gravity, area, pads, keys, mouse, camera, menu
//! state, screen size). Every write, typed script command and hook dispatch point still returns
//! [`GameError::Unsupported`]; the order in which they come is `docs/samp/native-plan.md` §5.

mod audio;
mod camera;
mod commands;
mod config;
mod hud;
mod input;
mod objects;
pub mod overlay;
mod peds;
mod render;
mod streaming;
mod vehicles;

#[cfg(any(test, feature = "fake"))]
pub mod fake;
mod world;

#[cfg(test)]
mod tests;

pub(crate) use sa_access::{addr, script};
pub use sa_access::{EngineView, View};

use sa_samp_api::config::SampModeConfig;
use sa_samp_api::hooks::{FramePhase, GameHooks};
use sa_samp_api::{
    API_VERSION, BackendInfo, BackendKind, GameAdapter, GameError, GameResult, GameState, SampHost, TimeStep,
    files,
};

/// The adapter over the native engine.
pub struct NativeGame<V: View> {
    game: sa_access::Game<V>,
    config: SampModeConfig,
    hooks: Option<Box<dyn GameHooks>>,
    files: Option<Box<dyn files::FileProvider>>,
    installed: bool,
    /// The text `CMessages::AddBigMessage` points into (GTA keeps the pointer).
    big_message: Box<[u8; 0x200]>,
    overlay: overlay::Overlay,
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
        Self::from_game(sa_access::Game::with_executor(view, exec))
    }

    /// An adapter over `game`.
    pub fn from_game(game: sa_access::Game<V>) -> Self {
        Self {
            game,
            config: SampModeConfig::default(),
            hooks: None,
            files: None,
            installed: false,
            big_message: Box::new([0; 0x200]),
            overlay: overlay::Overlay::default(),
        }
    }

    /// The memory the adapter reads.
    pub fn view(&self) -> &V {
        &self.game.view
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
            // An adapter call can reach an unported engine function, which unwinds (the phase B gap report);
            // the client must survive that, so the hooks go back before the unwind continues.
            let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| hooks.frame(phase, self)));
            // A callback cannot reinstall hooks (it only sees `&mut dyn GameAdapter`), so put them back.
            self.hooks = Some(hooks);
            if let Err(e) = r {
                std::panic::resume_unwind(e);
            }
        }
        self.overlay_frame(phase);
    }

    /// Runs the client's shutdown callback once and drops it.
    pub fn shutdown(&mut self) {
        if let Some(mut hooks) = self.hooks.take() {
            hooks.shutdown(self);
        }
        if self.installed {
            sa_core::samp::set_disabled_features(0);
            sa_core::samp::set_toggle(sa_core::samp::toggle::NO_CHEATS, false);
        }
    }
}

impl<V: View> std::ops::Deref for NativeGame<V> {
    type Target = sa_access::Game<V>;

    fn deref(&self) -> &Self::Target {
        &self.game
    }
}

impl<V: View> std::ops::DerefMut for NativeGame<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.game
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
        self.game.game_state()
    }

    fn time_step(&self) -> TimeStep {
        self.game.time_step()
    }

    fn game_time_ms(&self) -> u32 {
        self.game.game_time_ms()
    }

    fn request_quit(&mut self) {
        self.view.write_u32(addr::RS_QUIT, 1);
    }

    fn ui(&mut self) -> Option<&mut dyn sa_samp_api::ui::UiBackend> {
        Some(&mut self.overlay)
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
        // samp mode (SAMP-4): the ports read the disabled features from sa-core (see `config.rs`).
        self.config = config.clone();
        sa_core::samp::set_disabled_features(config.disabled.0);
        self.hooks = Some(hooks);
        self.files = files;
        self.installed = true;
        Ok(())
    }

    fn start_game(&mut self) -> GameResult<()> {
        Err(GameError::Unsupported("SampHost::start_game"))
    }
}
