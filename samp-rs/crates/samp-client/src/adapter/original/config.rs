//! samp-mode configuration and runtime toggles on the original executable: the original backend's `sa_samp_api::config` traits.
//! Methods that return `Unsupported` (or a neutral value, for reads that cannot fail) are not routed through
//! the adapter yet; the client still does that work in its own modules.

use sa_samp_api::config::*;
use sa_samp_api::*;

use super::OriginalGame;
use crate::patch::Memory;

impl<M: Memory> SampMode for OriginalGame<M> {
    fn config(&self) -> &SampModeConfig {
        &super::DEFAULT_CONFIG
    }

    fn set_toggle(&mut self, _toggle: RuntimeToggle) -> GameResult<()> {
        Err(GameError::Unsupported("SampMode::set_toggle"))
    }
}
