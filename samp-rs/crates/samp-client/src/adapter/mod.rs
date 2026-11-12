//! The game adapter: the client reaches the game through the `sa_samp_api` traits instead of raw addresses, so
//! that the same client can later run on sa-engine. `original` is the backend for the unmodified gta_sa.exe 1.0 US
//! and stays the default. Design and migration plan: sa-engine `docs/samp/game-adapter.md`.

pub mod original;
