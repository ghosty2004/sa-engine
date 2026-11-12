//! `--samp HOST[:PORT]`: the skeleton that will host the SA:MP client (samp-rs) in native sa-game
//! (docs/samp/native-plan.md §3).
//!
//! What it does today: builds the native adapter (`sa_samp::engine()`, the `sa_samp_api` traits on the
//! engine's state), installs a stand-in client through `SampHost::install` and registers the frame handler
//! of sa-core's dispatch points (`sa_core::samp`, called by the ported `Game::Idle` / `CGame::Process` at
//! samp's hook sites). The stand-in records which frame phases ran and, once per second of game time, logs
//! what samp's sync would read through the adapter (game state, local player, position, vehicle, clock,
//! weather, area). It opens no socket.
//!
//! What replaces the stand-in: samp-rs's client core as an rlib (`samp-core` after migration step 10, or
//! samp-client's host build until then) whose `GameHooks` implementation is passed to `install` here; its
//! network session runs on its own thread exactly as in the DLL (`netclient.rs`), and the adapter calls stay
//! on the game thread inside the frame callbacks.

use std::cell::RefCell;
use std::fmt::Write as _;

use sa_samp::{EngineView, NativeGame};
use sa_samp_api::hooks::{FramePhase, GameHooks};
use sa_samp_api::{GameAdapter, SampHost as _};

/// Where `--samp` connects (the client will use it; the skeleton only reports it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub host: String,
    pub port: u16,
    pub name: String,
}

/// Parses `HOST[:PORT]` (default port 7777, samp's).
pub fn parse_target(s: &str, name: &str) -> Result<Target, String> {
    let (host, port) = match s.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().map_err(|e| format!("--samp {s}: port: {e}"))?),
        None => (s, 7777),
    };
    if host.is_empty() {
        return Err(format!("--samp {s}: empty host"));
    }
    Ok(Target {
        host: host.into(),
        port,
        name: name.into(),
    })
}

/// Frame phases in `sa_core::samp::FramePhase` / `sa_samp_api::hooks::FramePhase` order.
const PHASES: [FramePhase; 6] = [
    FramePhase::BeforeGameProcess,
    FramePhase::CameraProcess,
    FramePhase::AfterGameProcess,
    FramePhase::Before2d,
    FramePhase::After2d,
    FramePhase::Present,
];

#[derive(Default)]
struct Stats {
    counts: [u64; 6],
    log: Vec<String>,
    last_log_ms: Option<u32>,
}

thread_local! {
    static GAME: RefCell<Option<NativeGame<EngineView>>> = const { RefCell::new(None) };
    static STATS: RefCell<Stats> = RefCell::new(Stats::default());
}

/// The stand-in client: counts phases and samples the adapter's read side.
struct StandIn;

impl GameHooks for StandIn {
    fn frame(&mut self, phase: FramePhase, game: &mut dyn GameAdapter) {
        STATS.with(|s| {
            let mut s = s.borrow_mut();
            if let Some(i) = PHASES.iter().position(|p| *p == phase) {
                s.counts[i] += 1;
            }
            if phase != FramePhase::Before2d {
                return;
            }
            let now = game.game_time_ms();
            if s.last_log_ms.is_some_and(|t| now.wrapping_sub(t) < 1000) {
                return;
            }
            s.last_log_ms = Some(now);
            let line = sample(game);
            println!("samp: {line}");
            s.log.push(line);
        });
    }
}

/// One line of what samp's on-foot / in-car sync would read now.
fn sample(game: &dyn GameAdapter) -> String {
    let mut l = format!("t={} ms state={:?}", game.game_time_ms(), game.game_state());
    match game.local_player() {
        Some(p) => {
            let _ = write!(l, " player={:#x}", p.0);
            if let Ok(s) = game.ped_snapshot(p) {
                let m = s.matrix.pos;
                let _ = write!(
                    l,
                    " pos=({:.2},{:.2},{:.2}) health={:.0} armour={:.0} model={} weapon={} live={}",
                    m.x, m.y, m.z, s.health, s.armour, s.model.0, s.weapon.0, s.in_world
                );
            }
            if let Some(v) = game.local_vehicle() {
                let _ = write!(l, " vehicle={:#x}", v.0);
            }
        }
        None => l.push_str(" player=none"),
    }
    let (h, m) = game.clock();
    let _ = write!(
        l,
        " clock={h:02}:{m:02} weather={:?} area={}",
        game.weather(),
        game.area_raw()
    );
    l
}

/// The UI's input path (`samp_input`): a window event to the adapter.
fn ui_event(e: sa_samp_api::input::WindowEvent) -> sa_samp_api::input::EventResult {
    GAME.with(|g| match g.borrow_mut().as_mut() {
        Some(game) => game.window_event(e),
        None => sa_samp_api::input::EventResult::PassToGame,
    })
}

/// samp's input lock (keyboard, mouse) from the UI.
fn ui_lock() -> (bool, bool) {
    GAME.with(|g| g.borrow().as_ref().map(|g| g.ui_input_lock()).unwrap_or_default())
}


/// sa-core's dispatch handler: runs the client's frame callback on the adapter.
fn on_frame(phase: u8) {
    let Some(&phase) = PHASES.get(usize::from(phase)) else {
        return;
    };
    GAME.with(|g| {
        // A frame callback never re-enters a dispatch point (they are not nested), so the borrow is free.
        if let Some(game) = g.borrow_mut().as_mut() {
            game.dispatch_frame(phase);
        }
    });
}

/// Builds the adapter, installs the stand-in client and registers the dispatch handler.
pub fn install(target: &Target) -> Result<(), String> {
    let mut game = sa_samp::engine();
    game.install(
        &sa_samp_api::config::SampModeConfig::default(),
        Box::new(StandIn),
        None,
    )
    .map_err(|e| format!("samp: install: {e}"))?;
    GAME.with(|g| *g.borrow_mut() = Some(game));
    sa_core::samp::set_frame_handler(on_frame);
    crate::samp_input::set_target(crate::samp_input::Target { event: ui_event, lock: ui_lock });
    println!(
        "sa-game: samp mode (skeleton): server {}:{} as {}; adapter {:?}; stand-in client (no network yet)",
        target.host,
        target.port,
        target.name,
        GAME.with(|g| g.borrow().as_ref().map(|g| g.backend().kind)),
    );
    Ok(())
}

/// The report section of the run.
pub fn report(md: &mut String, target: &Target) {
    sa_core::samp::clear_frame_handler();
    GAME.with(|g| {
        if let Some(mut game) = g.borrow_mut().take() {
            game.shutdown();
        }
    });
    STATS.with(|s| {
        let s = s.borrow();
        let _ = writeln!(
            md,
            "\n## SA:MP host (skeleton, `--samp {}:{}`)\n",
            target.host, target.port
        );
        let _ = writeln!(md, "| frame phase | dispatches |\n|---|---|");
        for (p, n) in PHASES.iter().zip(s.counts) {
            let _ = writeln!(md, "| {p:?} | {n} |");
        }
        let _ = writeln!(md, "\nAdapter samples (one per second of game time):\n");
        for l in &s.log {
            let _ = writeln!(md, "- {l}");
        }
        println!(
            "samp: frame dispatches {:?}",
            PHASES
                .iter()
                .zip(s.counts)
                .map(|(p, n)| format!("{p:?}={n}"))
                .collect::<Vec<_>>()
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_server_address() {
        assert_eq!(
            parse_target("127.0.0.1:7810", "Rust"),
            Ok(Target {
                host: "127.0.0.1".into(),
                port: 7810,
                name: "Rust".into()
            })
        );
        assert_eq!(parse_target("example.org", "N").unwrap().port, 7777);
        assert!(parse_target(":7777", "N").is_err());
        assert!(parse_target("h:x", "N").is_err());
    }
}
