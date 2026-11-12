//! samp mode (SAMP-4) on the native engine: the native backend's `sa_samp_api::config` traits. `install` stores
//! the configuration and hands the disabled features to the ports (`sa_core::samp::set_disabled_features`); the
//! ports test their bit at the call samp NOPs (sa_core::samp::feature lists the wired ones). Runtime toggles:
//! typed cheats so far.

use sa_samp_api::config::*;
use sa_samp_api::*;

use crate::{NativeGame, View};

impl<V: View> SampMode for NativeGame<V> {
    fn config(&self) -> &SampModeConfig {
        &self.config
    }

    fn set_toggle(&mut self, toggle: RuntimeToggle) -> GameResult<()> {
        match toggle {
            RuntimeToggle::TypedCheats(on) => {
                sa_core::samp::set_toggle(sa_core::samp::toggle::NO_CHEATS, !on);
                Ok(())
            }
            _ => Err(GameError::Unsupported("SampMode::set_toggle")),
        }
    }
}
