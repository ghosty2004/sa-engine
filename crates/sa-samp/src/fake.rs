//! A fake engine memory for host tests (`sa-access`'s, with the adapter around it).

pub use sa_access::fake::*;

use crate::NativeGame;

/// An adapter over [`sa_access::fake::game`].
pub fn game() -> NativeGame<FakeView> {
    NativeGame::from_game(sa_access::fake::game())
}
