//! What a host sets up before the game starts: raised limits.

use sa_types::{GameError, GameLimit, GameResult};

use crate::{Game, View};

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeView;

    #[test]
    fn limits_only_grow() {
        let mut g = Game::new(FakeView::default());
        assert!(g.set_limit(GameLimit::Peds, 100).is_err());
        assert!(g.set_limit(GameLimit::Vehicles, 0x8000).is_err());
        assert_eq!(limit_range(GameLimit::Vehicles), (110, 0x7FFF));
    }
}
