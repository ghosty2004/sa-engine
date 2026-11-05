//! Readers and writers for GTA San Andreas (PC 1.0 US) file formats.
//!
//! * [`img`]: IMG archive v2 directories.
//! * [`rw`]: RenderWare binary streams (DFF, TXD): lossless chunk tree and typed decoders.
//! * [`col`]: collision archives (COLL, COL2, COL3, COL4).
//!
//! The library works on byte slices only (no file system access), so it builds for
//! `wasm32-unknown-unknown`. Every format serialises back to the exact input bytes; the
//! `#[ignore]`d tests in `tests/game_data.rs` check that over the whole install.
//! Format findings are recorded in `docs/re/formats.md`.

mod bytes;
pub mod col;
pub mod error;
pub mod img;
pub mod rw;

pub use error::{Error, Result};
