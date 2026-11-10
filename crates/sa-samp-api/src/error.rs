//! Errors returned by adapter calls.

use core::fmt;

/// Why an adapter call did not do what was asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameError {
    /// The handle does not name a live entity (deleted, wrong generation, or never created).
    InvalidHandle,
    /// The model, collision, animation block or texture dictionary is not loaded yet.
    NotLoaded,
    /// The pool is full (a samp limit from [`crate::config::Limits`] was reached).
    PoolFull,
    /// The call is not allowed in the current phase (for example entity creation from a mid-frame hook, or
    /// a frame call before [`crate::SampHost::install`]).
    WrongPhase,
    /// This backend does not implement the call (yet). The string names the method.
    Unsupported(&'static str),
    /// The original code faulted inside the call (original/phase A backends; samp's SEH guard). `code` is
    /// the exception code, `address` where it happened.
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
            Self::Other(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for GameError {}

/// Result of an adapter call.
pub type GameResult<T> = Result<T, GameError>;
