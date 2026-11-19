//! What a host sets up before the game starts (raised limits, its own script files) and the main script's global
//! variables.

use std::sync::{Arc, RwLock};

use sa_types::{GameError, GameLimit, GameResult, ScriptFile};

use crate::addr;
use crate::{Game, View};

/// The largest script file a host may provide.
pub const SCRIPT_FILE_MAX: usize = 16 << 20;

/// The script files hosts provide, by [`ScriptFile`] (`None`: the install's).
static SCRIPT_FILES: RwLock<[Option<Arc<Vec<u8>>>; ScriptFile::ALL.len()]> = RwLock::new([None, None]);

/// The file a host provides in place of the install's `file` (the platform's file layer serves it to the game).
pub fn script_file(file: ScriptFile) -> Option<Arc<Vec<u8>>> {
    SCRIPT_FILES.read().unwrap_or_else(|e| e.into_inner())[file as usize].clone()
}

/// The engine's limit for `limit`.
fn core_limit(limit: GameLimit) -> sa_core::options::Limit {
    use sa_core::options::Limit as L;
    match limit {
        GameLimit::PtrNodesSingle => L::PtrNodesSingle,
        GameLimit::PtrNodesDouble => L::PtrNodesDouble,
        GameLimit::EntryInfoNodes => L::EntryInfoNodes,
        GameLimit::Peds => L::Peds,
        GameLimit::Vehicles => L::Vehicles,
        GameLimit::Buildings => L::Buildings,
        GameLimit::Objects => L::Objects,
        GameLimit::Dummies => L::Dummies,
        GameLimit::ColModels => L::ColModels,
        GameLimit::Tasks => L::Tasks,
        GameLimit::Events => L::Events,
        GameLimit::PedIntelligence => L::PedIntelligence,
        GameLimit::Matrices => L::Matrices,
        GameLimit::TextureDictionaries => L::TextureDictionaries,
        GameLimit::StreamingMemory => L::StreamingMemory,
        GameLimit::VehicleStructures => L::VehicleStructures,
    }
}

/// The original's value of `limit` and the highest a host may set.
pub fn limit_range(limit: GameLimit) -> (u32, u32) {
    let l = core_limit(limit);
    (l.original(), l.max())
}

impl<V: View> Game<V> {
    /// Raises `limit` to `value` (from the original's up to its maximum, [`limit_range`]). The game reads its
    /// limits as it starts, so only a value set before then takes effect.
    pub fn set_limit(&mut self, limit: GameLimit, value: u32) -> GameResult<()> {
        if sa_core::options::set_limit(core_limit(limit), value) {
            Ok(())
        } else {
            let (lo, hi) = limit_range(limit);
            Err(GameError::Other(format!("{limit:?} {value}: outside {lo}..={hi}")))
        }
    }

    /// Provides `data` in place of the install's script file `file`, from the next time the game reads it (a new
    /// game). A main script must start with its global variables' chunk (`GOTO`, as the game's compiler writes it).
    pub fn set_script_file(&mut self, file: ScriptFile, data: Vec<u8>) -> GameResult<()> {
        if data.len() > SCRIPT_FILE_MAX {
            return Err(GameError::Other(format!("{file:?}: {} bytes, at most {SCRIPT_FILE_MAX}", data.len())));
        }
        if file == ScriptFile::Main && !data.starts_with(&[0x02, 0x00, 0x01]) {
            return Err(GameError::Other("main script: it does not start with the GOTO over its variables".into()));
        }
        SCRIPT_FILES.write().unwrap_or_else(|e| e.into_inner())[file as usize] = Some(Arc::new(data));
        Ok(())
    }

    /// Sets the main script's global variable `index` (`$index`, at byte `4 * index` of the script space) to
    /// `value`. Only the variables the loaded main script declares (before its first chunk) can be set.
    pub fn set_script_global(&mut self, index: u32, value: i32) -> GameResult<()> {
        let end = self.view.read_u32(addr::SCRIPT_SPACE + 3);
        let at = index.checked_mul(4).filter(|&o| o >= 8 && o.checked_add(4).is_some_and(|e| e <= end));
        match at {
            Some(o) if end <= addr::MAIN_SCRIPT_MAX => {
                self.view.write_u32(addr::SCRIPT_SPACE + o, value as u32);
                Ok(())
            }
            _ => Err(GameError::Other(format!("${index}: not a variable of the loaded main script (it has {end} bytes)"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeView;

    #[test]
    fn script_globals_stay_inside_the_variables() {
        let mut g = Game::new(FakeView::default());
        // no main script loaded
        assert!(g.set_script_global(709, 1).is_err());
        g.view.write(addr::SCRIPT_SPACE, &[0x02, 0x00, 0x01]);
        g.view.write_u32(addr::SCRIPT_SPACE + 3, 46268);
        g.set_script_global(709, 1).unwrap();
        assert_eq!(g.view.read_u32(addr::SCRIPT_SPACE + 0xB14), 1);
        assert!(g.set_script_global(1, 5).is_err());
        assert!(g.set_script_global(46268 / 4, 5).is_err());
        g.set_script_global(46268 / 4 - 1, 5).unwrap();
    }

    #[test]
    fn limits_only_grow() {
        let mut g = Game::new(FakeView::default());
        assert!(g.set_limit(GameLimit::Peds, 100).is_err());
        assert!(g.set_limit(GameLimit::Vehicles, 0x8000).is_err());
        assert_eq!(limit_range(GameLimit::Vehicles), (110, 0x7FFF));
    }
}
