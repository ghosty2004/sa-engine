//! Readers and writers for GTA San Andreas (PC 1.0 US) file formats.
//!
//! * [`img`]: IMG archive v2 directories.
//! * [`rw`]: RenderWare binary streams (DFF, TXD): lossless chunk tree and typed decoders.
//! * [`col`]: collision archives (COLL, COL2, COL3, COL4).
//! * [`data`]: data definition files (IDE, IPL text/binary, `gta.dat`, the `data/*.dat` tables,
//!   path nodes), parsed with the exe's own line reader ([`lines`]) and `sscanf` ([`scanf`]).
//! * [`ifp`]: animation packages (ANP3, ANPK).
//! * [`gxt`]: text tables (TABL/TKEY/TDAT) and the exe's key hash.
//! * [`scm`]: `main.scm` header segments, streamed scripts and the instruction decoder.
//! * [`carrec`]: vehicle recordings (`carrec.img` entries).
//! * [`audio`]: SFX bank and stream pak containers and the audio configuration tables.
//! * [`save`]: save file block structure.
//!
//! The library works on byte slices only (no file system access), so it builds for
//! `wasm32-unknown-unknown`. Every format serialises back to the exact input bytes; the
//! `#[ignore]`d tests in `tests/game_data.rs` check that over the whole install.
//! Format findings are recorded in `docs/re/formats.md`.

pub mod audio;
mod bytes;
pub mod carrec;
pub mod col;
pub mod data;
pub mod error;
pub mod gxt;
pub mod ifp;
pub mod img;
pub mod lines;
pub mod rw;
pub mod save;
pub mod scanf;
pub mod scm;

pub use error::{Error, Result};
