//! The mods' game data in the game (feature `mods`, ADR-004 §4.2, [`sa_mod_host::data`]): each mod's declared files
//! appear in the game folder at `mods\<id>\` (a mount of the file layer), the level file `DATA\GTA.DAT` the game
//! reads carries their lines (the file layer's redirect), the model-info stores grow by what their definitions add
//! (before `CModelInfo::Initialise`), the collision-model and texture dictionary pools are at least their original
//! size plus what the data adds (the pool limits), and their texture dictionaries are loaded into slots of their own (before
//! `CGame::Init2`). Without data mods nothing of this happens.

use std::cell::RefCell;
use std::sync::{Arc, OnceLock};

use sa_core::orig::observe::{Observed, Phase};
use sa_mod_host::data::{self, StoreNeeds};
use sa_platform::vfs::GAME_ROOT;
use sa_stream::modelinfo::model_info::{ModelStore, raise_capacity};

use crate::bind;

/// The level file whose lines the mods extend (what `CGame::Initialise` loads after `DATA\DEFAULT.DAT`).
const LEVEL_FILE: &str = "data\\gta.dat";
/// The game's first level file.
const DEFAULT_LEVEL_FILE: &str = "data\\default.dat";

/// `CModelInfo::Initialise`: the stores are raised before it fills them.
pub const MODEL_INFO_INITIALISE: u32 = 0x4C6810;
/// `CGame::Init2`: the mods' texture dictionaries are loaded before it (the level data is loaded then).
pub const GAME_INIT2: u32 = 0x5BA1A0;

/// The level file with the mods' lines.
static LEVEL: OnceLock<Arc<Vec<u8>>> = OnceLock::new();

thread_local! {
    /// What is left to do at the game's start-up.
    static PENDING: RefCell<Pending> = RefCell::default();
}

#[derive(Default)]
struct Pending {
    /// What the mods' definitions add to the model-info stores.
    needs: Option<StoreNeeds>,
    /// The mods' texture dictionaries: (mod id, path in the game folder).
    txds: Vec<(String, String)>,
}

/// Where the mods' folders appear in the game folder (`mods\<id>`).
const MODS_DIR: &str = "mods";

/// The file layer's redirect: the level file with the mods' lines.
fn level_file(path: &str) -> Option<Arc<Vec<u8>>> {
    let (root, rel) = (
        path.get(..GAME_ROOT.len())?,
        path.get(GAME_ROOT.len()..)?.strip_prefix('\\')?,
    );
    if !root.eq_ignore_ascii_case(GAME_ROOT) || !rel.eq_ignore_ascii_case(LEVEL_FILE) {
        return None;
    }
    LEVEL.get().cloned()
}

/// Takes the loaded mods' data from `host` and prepares the game's start-up for it (call after the mods are loaded
/// and before the boot). Returns the game functions to watch ([`observed`]).
pub fn install(host: &mut sa_mod_host::Host) -> Vec<u32> {
    let mods = host.take_data();
    if mods.is_empty() {
        return Vec::new();
    }
    let read = |f: &str| {
        bind::with("mods data", |p| p.vfs.read(&format!("{GAME_ROOT}\\{f}"))).map_err(|e| format!("{f}: {e}"))
    };
    let (default_level, level) = match (read(DEFAULT_LEVEL_FILE), read(LEVEL_FILE)) {
        (Ok(d), Ok(l)) => (d, l),
        (Err(e), _) | (_, Err(e)) => {
            println!("sa-game: mods: no data loaded: {e}");
            return Vec::new();
        }
    };
    let plan = data::plan(mods, &[&default_level, &level], &level, MODS_DIR, &mut |s| {
        println!("sa-game: {s}")
    });
    if plan.mods.is_empty() || !raise_pools(&plan.needs) {
        return Vec::new();
    }
    let mut txds = Vec::new();
    bind::with("mods data", |p| {
        for m in plan.mods {
            txds.extend(
                m.files
                    .txd
                    .iter()
                    .map(|f| (m.id.clone(), format!("{}\\{f}", m.dir))),
            );
            p.vfs.mount(&format!("{GAME_ROOT}\\{}", m.dir), m.fs);
        }
    });
    let _ = LEVEL.set(Arc::new(plan.level_file));
    sa_platform::vfs::set_redirect(Some(level_file));
    PENDING.with_borrow_mut(|p| {
        *p = Pending {
            needs: Some(plan.needs),
            txds,
        }
    });
    vec![MODEL_INFO_INITIALISE, GAME_INIT2]
}

/// The stores' growth: each by what the mods' definitions add.
fn raise_stores(n: StoreNeeds) {
    let all = [
        (ModelStore::Atomic, n.atomic),
        (ModelStore::DamageAtomic, n.damage_atomic),
        (ModelStore::Time, n.time),
        (ModelStore::Weapon, n.weapon),
        (ModelStore::Clump, n.clump),
        (ModelStore::Vehicle, n.vehicle),
        (ModelStore::Ped, n.ped),
    ];
    for (store, extra) in all.into_iter().filter(|s| s.1 > 0) {
        let was = store.capacity();
        if raise_capacity(store, extra) {
            println!(
                "sa-game: mods: {store:?} model-info store {was} -> {}",
                store.capacity()
            );
        } else {
            println!(
                "sa-game: mods: {store:?} model-info store not raised (no memory); the mods' definitions overrun it"
            );
        }
    }
}

/// The game pools the mods' data fills, at least their original size plus what the data adds (on top of the
/// limits a mod set: the stock data stays within the original sizes). False when a pool cannot be that large.
fn raise_pools(n: &StoreNeeds) -> bool {
    use sa_core::options::{Limit, limit, set_limit};
    let all = [
        (Limit::ColModels, n.collision_models, "collision-model"),
        (
            Limit::TextureDictionaries,
            n.texture_dictionaries,
            "texture dictionary",
        ),
    ];
    for (l, extra, name) in all {
        let need = l.original() + extra;
        if need > limit(l) {
            if !set_limit(l, need) {
                println!(
                    "sa-game: mods: no data loaded: the {name} pool cannot hold {need} (at most {})",
                    l.max()
                );
                return false;
            }
            println!("sa-game: mods: {name} pool {need}");
        }
    }
    true
}

/// Loads the mods' texture dictionaries into slots named after their files.
fn load_txds(txds: Vec<(String, String)>) {
    for (id, path) in txds {
        let file = path.rsplit_once('\\').map_or(path.as_str(), |s| s.1);
        let name = file.rsplit_once('.').map_or(file, |s| s.0);
        if sa_access::txd_slot(name.as_bytes()).is_some() {
            println!(
                "sa-game: mod {id}: {file}: the game has a texture dictionary {name:?} already, not loaded"
            );
            continue;
        }
        match sa_access::load_txd_slot(name.as_bytes(), path.as_bytes()) {
            Some(slot) => {
                println!("sa-game: mod {id}: {file} loaded into texture dictionary slot {slot} ({name:?})")
            }
            None => println!("sa-game: mod {id}: {path} could not be opened"),
        }
    }
}

/// The use of the pools the game's data fills (`CPool`: objects, byte map, size; a free slot's byte has bit 7).
fn pool_use() -> String {
    use sa_core::pools::{BUILDING_POOL, COL_MODEL_POOL, DUMMY_POOL};
    let pools = [
        ("buildings", BUILDING_POOL.get().addr()),
        ("dummies", DUMMY_POOL.get().addr()),
        ("collision models", COL_MODEL_POOL.get().addr()),
        ("texture dictionaries", sa_stream::shared::TXD_POOL.get().addr()),
    ];
    let one = |(name, at): (&str, usize)| {
        let pool = sa_core::mem::GPtr::<u8>::from_addr(at);
        if pool.is_null() {
            return format!("{name} -");
        }
        let map = pool.byte_add(4).cast::<sa_core::mem::GPtr<u8>>().read();
        let size = pool.byte_add(8).cast::<i32>().read().max(0);
        let used = (0..size)
            .filter(|&i| (map.byte_add(i as isize).read() as i8) >= 0)
            .count();
        format!("{name} {used}/{size}")
    };
    pools.map(one).join(", ")
}

/// The game's start-up points the mods' data needs ([`install`]'s functions).
pub fn observed(o: &Observed<'_>) {
    match o.entry.addr {
        MODEL_INFO_INITIALISE if matches!(o.phase, Phase::Before) => {
            if let Some(n) = PENDING.with_borrow_mut(|p| p.needs.take()) {
                raise_stores(n);
            }
        }
        GAME_INIT2 if matches!(o.phase, Phase::Before) => {
            let txds = PENDING.with_borrow_mut(|p| std::mem::take(&mut p.txds));
            load_txds(txds);
            let used: Vec<String> = [ModelStore::Atomic, ModelStore::Ped, ModelStore::Clump]
                .into_iter()
                .map(|s| format!("{s:?} {}/{}", s.count(), s.capacity()))
                .collect();
            println!(
                "sa-game: mods: model-info stores after the level data: {}",
                used.join(", ")
            );
            println!("sa-game: mods: pools after the level data: {}", pool_use());
        }
        _ => {}
    }
}
