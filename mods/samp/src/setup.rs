//! What samp.dll changes in the game before it starts, through the mod API's game options, limits and script
//! files (the inventory of its patches: docs/samp/patch-parity.md).

use sa_mod::game::{GameLimit, GameOption, set_limit, set_option};
use sa_mod::script::{ScriptFile, set_file};
use samp_client::patches::{limits, streaming_budget};

/// The game's features samp.dll switches off (RPC 30 TogglePlayerClock turns the clock back on).
const OPTIONS_OFF: [GameOption; 30] = [
    GameOption::StartScreens,
    GameOption::TypedCheats,
    GameOption::Clock,
    GameOption::AmbientPeds,
    GameOption::RandomTraffic,
    GameOption::CarGenerators,
    GameOption::AmbientPlanes,
    GameOption::Cutscenes,
    GameOption::Replay,
    GameOption::Plants,
    GameOption::SunAndMoon,
    GameOption::IdleCamera,
    GameOption::HelpMessages,
    GameOption::ZoneNames,
    GameOption::VehicleNameOnHud,
    GameOption::VitalStatsOnHud,
    GameOption::SinglePlayerPickups,
    GameOption::AmbientGunfire,
    GameOption::RealTimeShadows,
    GameOption::PauseMenuOnFocus,
    GameOption::DeathModelCleanup,
    GameOption::SpeedBlur,
    GameOption::WastedMessage,
    GameOption::VehicleFreebies,
    GameOption::PlayerIdleAnims,
    GameOption::GunAimedAtResponse,
    GameOption::FireFailSounds,
    GameOption::StatChanges,
    GameOption::HandbrakeNeedsControl,
    GameOption::StandingSpeedClamp,
];

/// The physical memory samp.dll's streaming budget is chosen for: the mod sees no machine figures, and this
/// budget (256 MiB) leaves the game heap room on every target.
const STREAMING_BUDGET_RAM_MB: u64 = 1500;

/// The limits samp.dll raises, with its values.
fn raised_limits() -> [(GameLimit, u32); 15] {
    [
        (GameLimit::PtrNodesSingle, limits::PTR_NODES_SINGLE),
        (GameLimit::PtrNodesDouble, limits::PTR_NODES_DOUBLE),
        (GameLimit::EntryInfoNodes, limits::ENTRY_INFO_NODES),
        (GameLimit::Peds, limits::PEDS),
        (GameLimit::PedIntelligence, limits::PEDS),
        (GameLimit::Tasks, limits::TASKS),
        (GameLimit::Events, limits::EVENTS),
        (GameLimit::Matrices, limits::MATRICES),
        (GameLimit::Vehicles, limits::VEHICLES),
        (GameLimit::Buildings, limits::BUILDINGS),
        (GameLimit::Objects, limits::OBJECTS),
        (GameLimit::Dummies, limits::DUMMIES),
        (GameLimit::ColModels, limits::COL_MODELS),
        (GameLimit::TextureDictionaries, limits::TEXTURE_DICTIONARIES),
        (GameLimit::VehicleStructures, limits::VEHICLE_STRUCTURES),
    ]
}

/// The main script the game runs: samp's own (`main.scm` and `script.img` of the user's `samp.saa`, copied into
/// the mod's folder), else samp-rs's minimal one (the player created at Grove Street, densities and the wanted
/// level cap at 0, then idle; no stunt bonus script). Returns which, for the log.
fn main_script() -> Result<&'static str, String> {
    // the signature check hashes the whole archive, too much work for the mod's start-up; a genuine archive decodes
    // the same either way
    let saa = sa_mod::files::read("samp.saa")
        .ok()
        .flatten()
        .and_then(|d| samp_saa::SaaArchive::from_bytes_with(d, samp_saa::SignaturePolicy::AssumeValid).ok());
    let files = saa.and_then(|a| Some((a.read("main.scm")?, a.read("script.img")?)));
    let (what, main, streamed) = match files {
        Some((main, img)) => ("samp.saa's main.scm and script.img", main, Some(img)),
        None => {
            let main = samp_client::scm::build_main_scm(samp_client::game::INITIAL_PLAYER_POS);
            ("samp-rs's minimal main script (no samp.saa in the mod's folder)", main, None)
        }
    };
    set_file(ScriptFile::Main, &main).map_err(|e| format!("main script: {e:?}"))?;
    if let Some(img) = streamed {
        set_file(ScriptFile::Streamed, &img).map_err(|e| format!("streamed scripts: {e:?}"))?;
    }
    Ok(what)
}

/// Applies samp.dll's changes to the game; only in `sa_init` / `sa_settings`, before the game starts.
pub fn apply() -> Result<(), String> {
    for o in OPTIONS_OFF {
        set_option(o, false).map_err(|e| format!("game option {o:?}: {e:?}"))?;
    }
    for (l, v) in raised_limits() {
        set_limit(l, v).map_err(|e| format!("game limit {l:?} {v}: {e:?}"))?;
    }
    let budget = streaming_budget(STREAMING_BUDGET_RAM_MB);
    set_limit(GameLimit::StreamingMemory, budget).map_err(|e| format!("streaming budget {budget}: {e:?}"))?;
    let script = main_script()?;
    sa_mod::info(&format!("samp: the game runs {script}"));
    Ok(())
}
