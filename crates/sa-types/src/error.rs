//! Errors of calls into the game.

use core::fmt;

/// Why a call into the game did not do what was asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameError {
    /// The handle does not name a live entity (deleted, wrong generation, or never created).
    InvalidHandle,
    /// The model, collision, animation block or texture dictionary is not loaded yet.
    NotLoaded,
    /// The pool is full.
    PoolFull,
    /// The call is not allowed in the current phase (for example entity creation from a mid-frame hook).
    WrongPhase,
    /// Game memory the call needed could not be read (null or freed pointer).
    Unreadable {
        /// Address that could not be read.
        address: u32,
    },
    /// This backend does not implement the call (yet). The string names the method.
    Unsupported(&'static str),
    /// Code faulted inside the call. `code` is the exception code, `address` where it happened.
    Fault {
        /// Exception code (for example 0xC0000005).
        code: u32,
        /// Faulting instruction address.
        address: u32,
    },
    /// The game memory did not hold the expected original bytes, so a patch or hook was not applied.
    Mismatch {
        /// Address that was checked.
        address: u32,
    },
    /// Any other failure, with a short reason.
    Other(String),
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHandle => f.write_str("invalid entity handle"),
            Self::NotLoaded => f.write_str("resource not loaded"),
            Self::PoolFull => f.write_str("pool full"),
            Self::WrongPhase => f.write_str("call not allowed in this frame phase"),
            Self::Unsupported(m) => write!(f, "not supported by this backend: {m}"),
            Self::Fault { code, address } => write!(f, "fault {code:#x} at {address:#x}"),
            Self::Mismatch { address } => write!(f, "unexpected original bytes at {address:#x}"),
            Self::Unreadable { address } => write!(f, "game memory at {address:#x} is not readable"),
            Self::Other(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for GameError {}

/// Result of a call into the game.
pub type GameResult<T> = Result<T, GameError>;
