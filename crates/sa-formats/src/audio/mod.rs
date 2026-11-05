//! Audio containers (FMT-3): the `audio/CONFIG/*.dat` index files, the SFX bank packs in
//! `audio/SFX/` and the encrypted stream packs in `audio/streams/`.
//!
//! * [`config`]: `BankLkup.dat`, `BankSlot.dat`, `PakFiles.dat`, `StrmPaks.dat`,
//!   `TrakLkup.dat`, `EventVol.dat` (each read by the exe as a raw memory image).
//! * [`sfx`]: SFX bank packs (`CAEMP3BankLoader`): per bank a 0x12C4-byte header with up to
//!   400 sound entries, followed by the raw sample data.
//! * [`stream`]: stream packs (`CAEDataStream`): every byte XOR'd with a 16-byte key indexed by
//!   the absolute file position; per track a 0x1F84-byte `tTrackInfo` (beat table) followed by
//!   one Ogg Vorbis logical stream. The Ogg page framing is walked and CRC-checked; the Vorbis
//!   audio itself is not decoded.
//!
//! Layouts are taken from the 1.0 US exe (addresses in each module) and cross-checked with
//! gta-reversed. Findings are in `docs/re/formats.md`.

pub mod config;
pub mod sfx;
pub mod stream;

/// Keeps only non-zero counts (for reports).
pub(crate) fn nonzero(v: Vec<(String, u64)>) -> Vec<(String, u64)> {
    v.into_iter().filter(|(_, n)| *n > 0).collect()
}
