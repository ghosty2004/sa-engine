//! samp mode (SAMP-4) on the native engine: the native backend's `sa_samp_api::config` traits. The
//! configuration given to `SampHost::install` is stored and reported; the ports do not read it yet, and
//! runtime toggles are not served.

use sa_samp_api::config::*;
use sa_samp_api::*;

use crate::{NativeGame, View};

impl<V: View> SampMode for NativeGame<V> {
    fn config(&self) -> &SampModeConfig {
        &self.config
    }

    fn set_toggle(&mut self, _toggle: RuntimeToggle) -> GameResult<()> {
        Err(GameError::Unsupported("SampMode::set_toggle"))
    }
}
