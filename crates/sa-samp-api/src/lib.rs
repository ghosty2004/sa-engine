//! SAMP-API: the engine-side interface the SA:MP client `samp-rs` uses to drive the game.
//!
//! Today samp-rs reaches into `gta_sa.exe` 1.0 US through 622 raw addresses, 157 hook sites and the
//! byte patches of samp.dll (inventory: `docs/samp/README.md`, `catalog/samp-usage.csv`). This crate
//! names every one of those accesses as a typed Rust call, so that the same client code can run on
//! three backends:
//!
//! | backend | where | how it implements the traits |
//! |---|---|---|
//! | **original** (default, [`BackendKind::OriginalExe`]) | samp-rs DLL inside the unmodified `gta_sa.exe` | the current address-based code of samp-rs: memory reads/writes, SCM opcodes through `CRunningScript::ProcessOneCommand`, byte patches and call/vtable hooks |
//! | **engine phase A** ([`BackendKind::EngineInProcess`]) | samp-rs DLL inside `gta_sa.exe` with `sa_hook.dll` replacing functions by Rust ports (Wine) | same memory layout as the original (ADR-001), but hooks are registered with sa-hook's dispatch points and patches become [`config::SampModeConfig`] flags read by the ports |
//! | **engine phase B** ([`BackendKind::EngineNative`]) | samp-rs linked as a Rust library into the native sa-engine executable | no addresses at all: engine-owned state, typed handles, engine config |
//!
//! The crate holds types and traits only. It has no dependencies and no `unsafe`, and builds for the host,
//! `i686-pc-windows-gnu` (samp-rs) and `wasm32-unknown-unknown` (phase C).
//!
//! # Shape
//!
//! * [`GameAdapter`] (implemented by each backend, called by samp-rs) is the union of the feature traits:
//!   [`peds::Peds`], [`vehicles::Vehicles`], [`objects::Objects`], [`world::WorldState`],
//!   [`world::WorldEntities`], [`camera::Camera`], [`hud::Hud`], [`hud::Text2d`], [`hud::Menus`],
//!   [`input::Input`], [`audio::GameAudio`], [`streaming::Streaming`], [`script::ScriptCommands`],
//!   [`render::Overlay`] and [`config::SampMode`].
//! * [`hooks::GameHooks`] (implemented by samp-rs, called by the backend) replaces every hook and every
//!   return-address test with an explicit callback that names its call site.
//! * [`files::FileProvider`] (implemented by samp-rs) replaces the kernel32 file redirection for samp.saa.
//! * [`config::SampModeConfig`] replaces samp.dll's start-up patches; [`config::RuntimeToggle`] its
//!   runtime patches.
//!
//! # Threading and frame contract (summary; full text in `docs/samp/game-adapter.md` §3)
//!
//! * The game is single-threaded. Every [`GameAdapter`] method is called on the game thread, and only
//!   from inside a [`hooks::GameHooks`] callback, which receives the adapter as an argument. samp-rs
//!   never stores the adapter.
//! * Frame callbacks ([`hooks::FramePhase`]) receive `&mut dyn GameAdapter`: everything is allowed.
//! * Mid-frame hooks (inside `CWeapon::Fire`, `CPed::Render`, a vehicle's `ProcessControl`, ...)
//!   receive `&dyn GameAdapter`: reads only. Mutations go through the hook's own typed context or are
//!   queued by samp-rs for the next frame callback (what samp-rs already does with its `SHARED` state
//!   and event queues).
//! * Read methods take `&self`, mutators take `&mut self`, so the borrow checker enforces the rule.
//! * [`files::FileProvider`] may be called from the streaming thread and is `Send + Sync`.

pub mod audio;
pub mod camera;
pub mod config;
pub mod error;
pub mod files;
pub mod hooks;
pub mod hud;
pub mod input;
pub mod objects;
pub mod peds;
pub mod render;
pub mod script;
pub mod streaming;
pub mod types;
pub mod vehicles;
pub mod world;

pub use error::{GameError, GameResult};
pub use types::*;

/// Version of this interface. Bumped on every incompatible change; samp-rs checks it at start-up.
pub const API_VERSION: u32 = 1;

/// Which implementation of the traits is running.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BackendKind {
    /// Unmodified `gta_sa.exe` 1.0 US; the address-based samp-rs code. The default.
    OriginalExe,
    /// sa-engine phase A: `gta_sa.exe` with Rust ports installed by sa-hook (same memory layout).
    EngineInProcess,
    /// sa-engine phase B: native executable, no original code and no fixed addresses.
    EngineNative,
}

/// Static facts about the running backend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackendInfo {
    /// Which backend this is.
    pub kind: BackendKind,
    /// [`API_VERSION`] the backend was built against.
    pub api_version: u32,
    /// Free-form build string (engine version, game exe hash, ...), for logs and crash reports.
    pub build: String,
    /// Whether raw game addresses are meaningful (true for `OriginalExe` and `EngineInProcess`). Code that
    /// still needs an address (crash reports, the samp auth read of 0x4D3AA0) must check this.
    pub has_original_layout: bool,
}

/// Everything samp-rs may ask of the game. Implemented by each backend.
///
/// Object-safe: samp-rs holds it as `&mut dyn GameAdapter` / `&dyn GameAdapter`.
pub trait GameAdapter:
    peds::Peds
    + vehicles::Vehicles
    + objects::Objects
    + world::WorldState
    + world::WorldEntities
    + camera::Camera
    + hud::Hud
    + hud::Text2d
    + hud::Menus
    + input::Input
    + audio::GameAudio
    + streaming::Streaming
    + script::ScriptCommands
    + render::Overlay
    + config::SampMode
{
    /// Which backend runs, and its version.
    fn backend(&self) -> BackendInfo;

    /// Game state machine value (`gGameState` 0xC8D4C0 in the original; 7 = frontend idle,
    /// 8 = loading started, 9 = playing).
    fn game_state(&self) -> GameState;

    /// Real frame time step and the clipped one (`CTimer::ms_fTimeStep` 0xB7CB5C,
    /// `ms_fTimeStepNonClipped` 0xB7CB58).
    fn time_step(&self) -> TimeStep;

    /// Game time in milliseconds (`CTimer::m_snTimeInMilliseconds` 0xB7CB84).
    fn game_time_ms(&self) -> u32;

    /// Only the clipped time step (`CTimer::ms_fTimeStep` 0xB7CB5C), for sites that read just that one.
    fn time_step_clipped(&self) -> f32 {
        self.time_step().clipped
    }

    /// Requests the game to quit at the next frame boundary (`RsGlobal.quit` 0xC17050).
    fn request_quit(&mut self);
}

/// The host side of start-up: samp-rs hands its callbacks and configuration to the backend once.
///
/// Original backend: called by the samp-rs init thread when `gGameState` reaches 7 (frontend idle), the
/// moment samp.dll applies its main patch block. Engine backends: called before the world is created.
pub trait SampHost {
    /// Applies `config` and registers the client's callbacks and file provider. Must be called exactly once,
    /// before the first frame callback. Errors leave the game unmodified where the backend can guarantee it
    /// (the original backend's all-or-nothing relocations).
    fn install(
        &mut self,
        config: &config::SampModeConfig,
        hooks: Box<dyn hooks::GameHooks>,
        files: Option<Box<dyn files::FileProvider>>,
    ) -> GameResult<()>;

    /// Leaves the frontend and starts loading the world (samp's menu-flag writes and `gGameState := 8`).
    fn start_game(&mut self) -> GameResult<()>;
}

// The interface is used through trait objects; these fail to compile if a trait stops being object-safe.
const _: Option<&dyn GameAdapter> = None;
const _: Option<&dyn SampHost> = None;
const _: Option<&dyn hooks::GameHooks> = None;
const _: Option<&dyn files::FileProvider> = None;

#[cfg(test)]
mod tests {
    use crate::config::{Feature, FeatureSet};

    #[test]
    fn feature_bits_are_distinct_and_fit() {
        let mut set = FeatureSet::NONE;
        for (i, f) in Feature::ALL.iter().enumerate() {
            assert_eq!(*f as usize, i, "Feature::ALL must follow declaration order");
            assert!(!set.contains(*f));
            set = set.with(*f);
            assert!(set.contains(*f));
        }
        assert_eq!(set.0.count_ones() as usize, Feature::ALL.len());
    }
}
