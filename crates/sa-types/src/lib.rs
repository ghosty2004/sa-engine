//! Plain value types of the game's state, shared by everything that reads or drives the game from outside the
//! ports: the engine's state access layer (`sa-access`), the mod API and the SA:MP game adapter.

pub mod camera;
pub mod error;
pub mod input;
pub mod ped;
pub mod script;
pub mod vehicle;
mod values;

pub use error::{GameError, GameResult};
pub use values::*;
