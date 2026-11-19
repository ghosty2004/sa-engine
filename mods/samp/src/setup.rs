//! The limits samp.dll raises in the game before it starts (its pool sizes, matrices, texture dictionaries and
//! streaming budget), through the mod API's game limits.

use sa_mod::game::{GameLimit, set_limit};
use samp_client::patches::{limits, streaming_budget};

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

/// Raises the game's limits to samp.dll's; only in `sa_init` / `sa_settings`, before the game sizes them.
pub fn raise_limits() -> Result<(), String> {
    for (l, v) in raised_limits() {
        set_limit(l, v).map_err(|e| format!("game limit {l:?} {v}: {e:?}"))?;
    }
    let budget = streaming_budget(STREAMING_BUDGET_RAM_MB);
    set_limit(GameLimit::StreamingMemory, budget).map_err(|e| format!("streaming budget {budget}: {e:?}"))
}
