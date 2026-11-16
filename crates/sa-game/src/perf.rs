//! The per-frame measurement of the game (PERF-0, docs/perf/measure.md): the phases and counters of an in-game frame
//! on [`sa_platform::perf`].
//!
//! Off unless asked for (`--perf FILE` / `SA_GAME_PERF=FILE` natively, `?SA_GAME_PERF=N` on the web page): then every
//! entry point below costs one relaxed atomic load. Nothing here runs inside a port's body: the phases are opened
//! around ports by the address registry (the probes of [`PROBES`], installed by [`crate::native::install`] as
//! wrappers of the ports' native entries, [`crate::platform::perf_probe`]), by the D3D9 COM methods
//! ([`crate::d3d9`]) and by the main loop ([`crate::app`]). Game behaviour is unchanged: the probes only time the call
//! they wrap.
//!
//! A row is one in-game frame: the main-loop iterations up to and including one that ran `Game::Idle` (the frame
//! limiter's waiting iterations and message iterations belong to the next frame), so its phases add up to the time
//! between two in-game frames. Iterations outside `gGameState` IDLE (loading) are not recorded.

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};

use sa_platform::perf::{self, Layout, Scope};

pub use sa_platform::perf::enabled;

// ---- phases (exclusive times; their sum is the frame time) -------------------------------------------------------
/// Whatever no other phase covers: the main-loop bookkeeping, the front end (window events, the browser between
/// iterations).
pub const OTHER: usize = 0;
/// The main-loop iteration outside `Game::Idle`: the `gGameState` machine, the frame limiter's wait.
pub const LOOP: usize = 1;
/// Input: the front end's events queued as window messages, `PeekMessageA` / `TranslateMessage` / `DispatchMessageA`.
pub const PUMP: usize = 2;
/// `Game::Idle`'s busy-wait at its start (until 14 ms have passed since the previous frame's), up to `CTimer::Update`.
pub const IDLE_SPIN: usize = 3;
/// `Game::Idle` from `CTimer::Update` up to `CRenderer::ConstructRenderList`: `CGame::Process` (everything its
/// sub-phases below do not cover), the time-of-day lights.
pub const LOGIC: usize = 4;
pub const LOGIC_PAD: usize = 5;
pub const LOGIC_STREAMING: usize = 6;
pub const LOGIC_SCRIPT: usize = 7;
pub const LOGIC_WORLD: usize = 8;
pub const LOGIC_POPULATION: usize = 9;
pub const LOGIC_CAMERA: usize = 10;
pub const LOGIC_FX: usize = 11;
pub const AUDIO: usize = 12;
/// `Game::Idle` from `CRenderer::ConstructRenderList` on: the game's render code (the RenderWare pipelines and the
/// ported RW D3D9 driver, `RenderScene`, effects, 2D) that its sub-phases do not cover, without the D3D9 layer.
pub const RENDER: usize = 13;
pub const RENDER_PRERENDER: usize = 14;
pub const RENDER_LIST: usize = 15;
pub const RENDER_WORLD: usize = 16;
pub const RENDER_SHADOWS: usize = 17;
pub const RENDER_WATER: usize = 18;
pub const RENDER_SKY: usize = 19;
pub const RENDER_EFFECTS: usize = 20;
pub const RENDER_PLANTS: usize = 21;
pub const RENDER_CORONAS: usize = 22;
pub const RENDER_HUD: usize = 23;
/// Our D3D9 COM methods: the translation to the seam and `WgpuDevice` (state, buffer and texture uploads, draws).
pub const D3D9: usize = 24;
/// `IDirect3DDevice9::Present`: the last render pass's submission, the window blit and the surface present.
pub const PRESENT: usize = 25;
/// The measurement's own bookkeeping at the end of a frame.
pub const PERF: usize = 26;

const PHASES: &[&str] = &[
    "other",
    "loop",
    "pump",
    "idle.spin",
    "logic",
    "logic.pad",
    "logic.streaming",
    "logic.script",
    "logic.world",
    "logic.population",
    "logic.camera",
    "logic.fx",
    "audio",
    "render",
    "render.prerender",
    "render.list",
    "render.world",
    "render.shadows",
    "render.water",
    "render.sky",
    "render.effects",
    "render.plants",
    "render.coronas",
    "render.hud",
    "d3d9",
    "present",
    "perf",
];

// ---- counters (per frame) ----------------------------------------------------------------------------------------
pub const C_D3D9_CALLS: usize = 0;
pub const C_D3D9_SETS: usize = 1;
const C_DRAWS: usize = 2;
const C_PASSES: usize = 3;
const C_PIPELINES_NEW: usize = 4;
const C_PIPELINE_CHANGES: usize = 5;
const C_TEXGROUP_CHANGES: usize = 6;
const C_BUF_UPLOADS: usize = 7;
const C_BUF_BYTES: usize = 8;
const C_TEX_UPLOADS: usize = 9;
const C_TEX_BYTES: usize = 10;
const C_STREAM_BYTES: usize = 11;
const C_DISPATCH: usize = 12;
const C_PLATFORM: usize = 13;
pub const C_GAME_ALLOCS: usize = 14;
const C_ITERATIONS: usize = 15;
const C_SUBMITS: usize = 16;

const COUNTERS: &[&str] = &[
    "d3d9_calls",
    "d3d9_set_calls",
    "draws",
    "passes",
    "pipelines_created",
    "pipeline_changes",
    "texture_group_changes",
    "buffer_uploads",
    "buffer_bytes",
    "texture_uploads",
    "texture_bytes",
    "stream_bytes",
    "dispatch_calls",
    "platform_calls",
    "game_allocs",
    "iterations",
    "submits",
];

/// Late column: the GPU time of the frame's render passes (timestamp queries; empty where unsupported).
const LATE: &[&str] = &["gpu_ms"];

/// What a probe does when its function is entered.
#[derive(Clone, Copy, Debug)]
pub enum Act {
    /// opens its phase for the call
    Scope,
    /// `Game::Idle`: opens [`IDLE_SPIN`] and marks the iteration as an in-game frame
    Idle,
    /// `CTimer::Update`: the busy-wait is over ([`IDLE_SPIN`] → [`LOGIC`]), then its phase
    LogicStart,
    /// `CRenderer::ConstructRenderList`: the frame's logic part ends ([`LOGIC`] → [`RENDER`]), then its phase
    RenderStart,
}

/// The original convention and argument words of a probed function (the wrapper's ABI on i686).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sig {
    Cdecl(u8),
    /// `this` included
    Thiscall(u8),
}

/// A port timed through the address registry: its original address, the phase it opens, what its entry does.
#[derive(Clone, Copy, Debug)]
pub struct Probe {
    pub addr: u32,
    pub sig: Sig,
    pub phase: usize,
    pub act: Act,
    pub name: &'static str,
}

const fn p(addr: u32, sig: Sig, phase: usize, act: Act, name: &'static str) -> Probe {
    Probe {
        addr,
        sig,
        phase,
        act,
        name,
    }
}

const C0: Sig = Sig::Cdecl(0);
const C1: Sig = Sig::Cdecl(1);
const T1: Sig = Sig::Thiscall(1);
const T3: Sig = Sig::Thiscall(3);
const S: Act = Act::Scope;

/// The probes. Each is a function its callers reach through the registry (an `=> unported` declaration in a lower
/// crate, catalog/bindings.csv), so the wrapper sees those calls; the signatures are the bindings'.
pub const PROBES: &[Probe] = &[
    p(0x53E920, C1, IDLE_SPIN, Act::Idle, "Game::Idle"),
    p(0x561B10, C0, LOGIC, Act::LogicStart, "CTimer::Update"),
    p(0x541DD0, C0, LOGIC_PAD, S, "CPad::UpdatePads"),
    p(0x40E670, C0, LOGIC_STREAMING, S, "CStreaming::Update"),
    p(
        0x40EA10,
        C1,
        LOGIC_STREAMING,
        S,
        "CStreaming::LoadAllRequestedModels",
    ),
    p(0x46A000, C0, LOGIC_SCRIPT, S, "CTheScripts::Process"),
    p(0x5684A0, C0, LOGIC_WORLD, S, "CWorld::Process"),
    p(0x616650, C1, LOGIC_POPULATION, S, "CPopulation::Update"),
    p(0x4341C0, C0, LOGIC_POPULATION, S, "CCarCtrl::GenerateRandomCars"),
    p(0x52B730, T1, LOGIC_CAMERA, S, "CCamera::Process"),
    p(0x49E640, T3, LOGIC_FX, S, "Fx_c::Update"),
    p(0x507750, T1, AUDIO, S, "CAudioEngine::Service"),
    p(
        0x5556E0,
        C0,
        RENDER_LIST,
        Act::RenderStart,
        "CRenderer::ConstructRenderList",
    ),
    p(0x553910, C0, RENDER_PRERENDER, S, "CRenderer::PreRender"),
    p(
        0x563430,
        C0,
        RENDER_PRERENDER,
        S,
        "CWorld::ProcessPedsAfterPreRender",
    ),
    p(0x706AB0, T1, RENDER_SHADOWS, S, "CRealTimeShadowManager::Update"),
    p(0x553A10, C0, RENDER_WORLD, S, "CRenderer::RenderRoads"),
    p(
        0x553AA0,
        C0,
        RENDER_WORLD,
        S,
        "CRenderer::RenderEverythingBarRoads",
    ),
    p(0x5531E0, C0, RENDER_WORLD, S, "CRenderer::RenderFadingInEntities"),
    p(
        0x553220,
        C0,
        RENDER_WORLD,
        S,
        "CRenderer::RenderFadingInUnderwaterEntities",
    ),
    p(
        0x553D00,
        C0,
        RENDER_WORLD,
        S,
        "CRenderer::RenderFirstPersonVehicle",
    ),
    p(
        0x733800,
        C0,
        RENDER_WORLD,
        S,
        "CVisibilityPlugins::RenderReallyDrawLastObjects",
    ),
    p(0x707F40, C0, RENDER_SHADOWS, S, "CShadows::UpdateStaticShadows"),
    p(0x708300, C0, RENDER_SHADOWS, S, "CShadows::RenderStaticShadows"),
    p(0x70A960, C0, RENDER_SHADOWS, S, "CShadows::RenderStoredShadows"),
    p(
        0x7113B0,
        C0,
        RENDER_SHADOWS,
        S,
        "CStencilShadows::RenderStencilShadows",
    ),
    p(0x6EF650, C0, RENDER_WATER, S, "CWaterLevel::RenderWater"),
    p(0x7178F0, C0, RENDER_SKY, S, "CMovingThings::Render_BeforeClouds"),
    p(0x713950, C0, RENDER_SKY, S, "CClouds::Render"),
    p(0x714650, C0, RENDER_SKY, S, "CClouds::RenderSkyPolys"),
    p(0x7154B0, C0, RENDER_SKY, S, "CClouds::RenderBottomFromHeight"),
    p(0x716380, C0, RENDER_SKY, S, "CClouds::VolumetricCloudsRender"),
    p(0x716C90, C0, RENDER_SKY, S, "CClouds::MovingFogRender"),
    p(0x5DBAE0, C0, RENDER_PLANTS, S, "CPlantMgr::Render"),
    p(0x6FAEC0, C0, RENDER_CORONAS, S, "CCoronas::Render"),
    p(0x6FB630, C0, RENDER_CORONAS, S, "CCoronas::RenderReflections"),
    p(0x6FBAA0, C0, RENDER_CORONAS, S, "CCoronas::RenderSunReflection"),
    p(0x7046E0, C0, RENDER_EFFECTS, S, "CPostEffects::Render"),
    p(0x712810, C0, RENDER_EFFECTS, S, "CBirds::Render"),
    p(0x742CF0, C0, RENDER_EFFECTS, S, "CWeaponEffects::Render"),
    p(0x726AD0, C0, RENDER_EFFECTS, S, "CSpecialFX::Render"),
    p(0x71CE20, C0, RENDER_EFFECTS, S, "CGlass::Render"),
    p(0x720640, C0, RENDER_EFFECTS, S, "CSkidmarks::Render"),
    p(0x72AF70, C0, RENDER_EFFECTS, S, "CWeather::RenderRainStreaks"),
    p(0x58FAE0, C0, RENDER_HUD, S, "CHud::Draw"),
    p(0x71A210, C0, RENDER_HUD, S, "CFont::DrawFonts"),
];

/// Set by the `Game::Idle` probe: the current iteration is an in-game frame.
static IDLE_SEEN: AtomicBool = AtomicBool::new(false);

/// Where the run's results go: natively a CSV file at exit, on the web the console after N frames.
#[derive(Clone, Debug)]
pub enum Output {
    File(std::path::PathBuf),
    Console { frames: usize },
}

struct Run {
    output: Output,
    /// cumulative sources at the last frame boundary: device stats, registry calls, platform calls
    base: [u64; 13],
    gpu: Option<bool>,
}

thread_local! {
    static RUN: RefCell<Option<Run>> = const { RefCell::new(None) };
}

/// Starts the measurement on this thread (the main loop's). Call before the GPU device is created (its timestamp
/// feature is requested only then) and before the registry is installed (the probes).
pub fn enable(output: Output) {
    perf::enable(Layout {
        phases: PHASES,
        counters: COUNTERS,
        late: LATE,
    });
    RUN.with(|r| {
        *r.borrow_mut() = Some(Run {
            output,
            base: [0; 13],
            gpu: None,
        })
    });
    // the `dispatch_calls` counter
    sa_core::orig::registry::set_counting(true);
    println!("sa-game: per-frame measurement on (docs/perf/measure.md)");
}

/// Opens `phase` (no-op when off).
#[inline]
pub fn scope(phase: usize) -> Option<Scope> {
    enabled().then(|| perf::scope(phase))
}

/// Adds to counter `c` (no-op when off).
#[inline]
pub fn add(c: usize, n: u64) {
    if enabled() {
        perf::add(c, n);
    }
}

/// A D3D9 COM method `name` is entered: counted, and [`D3D9`] opened for it.
#[inline]
pub fn d3d9_call(name: &'static str) -> Option<Scope> {
    if !enabled() {
        return None;
    }
    perf::add(C_D3D9_CALLS, 1);
    if name.starts_with("IDirect3DDevice9::Set") {
        perf::add(C_D3D9_SETS, 1);
    }
    Some(perf::scope(D3D9))
}

/// The entry of probe `i` ([`PROBES`]) is called.
#[inline]
pub fn probe_enter(i: usize) -> Option<Scope> {
    if !enabled() {
        return None;
    }
    let pr = PROBES.get(i)?;
    match pr.act {
        Act::Scope => {}
        Act::Idle => IDLE_SEEN.store(true, Ordering::Relaxed),
        Act::LogicStart => perf::switch_top(IDLE_SPIN, LOGIC),
        Act::RenderStart => perf::switch_top(LOGIC, RENDER),
    }
    Some(perf::scope(pr.phase))
}

/// The cumulative sources of the counters: device stats, orig! registry dispatches, platform calls.
fn sources() -> [u64; 13] {
    let s = crate::d3d9::with_device(|d| d.stats).unwrap_or_default();
    let reg: u64 = sa_core::orig::registry::table()
        .iter()
        .map(|e| u64::from(e.calls.load(Ordering::Relaxed)))
        .sum::<u64>()
        + u64::from(sa_core::orig::registry::misses());
    [
        s.draws,
        s.flushes,
        s.pipelines,
        s.pipeline_changes,
        s.texture_group_changes,
        s.buffer_uploads,
        s.buffer_bytes,
        s.texture_uploads,
        s.texture_bytes,
        s.stream_bytes,
        reg,
        crate::bind::calls_total(),
        s.submits,
    ]
}

/// The counter each source feeds.
const SOURCE_COUNTERS: [usize; 13] = [
    C_DRAWS,
    C_PASSES,
    C_PIPELINES_NEW,
    C_PIPELINE_CHANGES,
    C_TEXGROUP_CHANGES,
    C_BUF_UPLOADS,
    C_BUF_BYTES,
    C_TEX_UPLOADS,
    C_TEX_BYTES,
    C_STREAM_BYTES,
    C_DISPATCH,
    C_PLATFORM,
    C_SUBMITS,
];

/// The end of main-loop iteration `n` that began in `gGameState` `state` (both the normal and the blocked path):
/// closes a row after an in-game frame, drops the time of iterations that began outside IDLE (loading).
pub fn end_iteration(n: u32, state: u32) {
    if !enabled() {
        return;
    }
    let _s = perf::scope(PERF);
    perf::add(C_ITERATIONS, 1);
    let idle = IDLE_SEEN.swap(false, Ordering::Relaxed) && state == crate::winmain::STATE_IDLE;
    if !idle && state == crate::winmain::STATE_IDLE {
        // a waiting or message iteration: it belongs to the next frame
        return;
    }
    let row = perf::rows();
    let cur = sources();
    let gpu_on = RUN.with(|r| r.borrow().as_ref().and_then(|r| r.gpu));
    let gpu = crate::d3d9::with_device(|d| {
        let on = gpu_on.unwrap_or_else(|| d.enable_gpu_timing());
        if on {
            d.gpu_frame_end(if idle { row as u64 } else { u64::MAX });
        }
        (on, if on { d.gpu_frame_times(false) } else { Vec::new() })
    });
    let report = RUN.with(|r| {
        let mut r = r.borrow_mut();
        let run = r.as_mut()?;
        if let Some((on, _)) = &gpu
            && run.gpu.is_none()
        {
            run.gpu = Some(*on);
            println!(
                "sa-game: perf: GPU time {}",
                if *on {
                    "from timestamp queries"
                } else {
                    "not available (no TIMESTAMP_QUERY on this device)"
                }
            );
        }
        let deltas: Vec<(usize, u64)> = SOURCE_COUNTERS
            .iter()
            .enumerate()
            .map(|(i, &c)| (c, cur[i].wrapping_sub(run.base[i])))
            .collect();
        run.base = cur;
        if !idle {
            perf::discard();
            return None;
        }
        perf::end_frame(u64::from(n), &deltas);
        match run.output {
            Output::Console { frames } if perf::rows() >= frames => Some(()),
            _ => None,
        }
    });
    for (tag, ms, _) in gpu.map(|g| g.1).unwrap_or_default() {
        if tag != u64::MAX {
            perf::set_late(tag as usize, 0, ms);
        }
    }
    if report.is_some() {
        drop(_s);
        finish();
    }
}

/// Ends the measurement: the remaining GPU times are collected, the summary is printed and the CSV goes to the
/// output (a file natively, the console on the web).
pub fn finish() {
    if !enabled() {
        return;
    }
    let late = crate::d3d9::with_device(|d| d.gpu_frame_times(true)).unwrap_or_default();
    for (tag, ms, _) in late {
        if tag != u64::MAX {
            perf::set_late(tag as usize, 0, ms);
        }
    }
    perf::disable();
    let Some(output) = RUN.with(|r| r.borrow().as_ref().map(|r| r.output.clone())) else {
        return;
    };
    // one console message each (the page's worker keeps only the last 500 lines between two status posts)
    let summary: String = perf::summary()
        .lines()
        .map(|l| format!("sa-game: {l}\n"))
        .collect();
    println!("{}", summary.trim_end());
    let csv = perf::csv();
    match output {
        Output::File(p) => match std::fs::write(&p, &csv) {
            Ok(()) => println!(
                "sa-game: perf: {} frames written to {}",
                perf::rows(),
                p.display()
            ),
            Err(e) => eprintln!("sa-game: perf: {}: {e}", p.display()),
        },
        Output::Console { .. } => {
            println!(
                "sa-game: perf: CSV follows (up to `sa-game: perf: CSV end`)\n{csv}sa-game: perf: CSV end"
            );
        }
    }
}
