use std::fmt;

/// Error returned by every parser and writer in this crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The input ended before a field could be read.
    Truncated {
        context: &'static str,
        offset: usize,
        needed: usize,
        available: usize,
    },
    /// The input is structurally invalid for the format.
    Invalid { context: &'static str, detail: String },
    /// The input is valid for the format, but uses a variant this crate does not decode
    /// (and that does not occur in the 1.0 US install).
    Unsupported { context: &'static str, detail: String },
}

impl Error {
    pub(crate) fn invalid(context: &'static str, detail: impl Into<String>) -> Self {
        Error::Invalid {
            context,
            detail: detail.into(),
        }
    }

    pub(crate) fn unsupported(context: &'static str, detail: impl Into<String>) -> Self {
        Error::Unsupported {
            context,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Truncated {
                context,
                offset,
                needed,
                available,
            } => write!(
                f,
                "{context}: truncated at offset {offset:#x}: needed {needed} bytes, {available} available"
            ),
            Error::Invalid { context, detail } => write!(f, "{context}: invalid: {detail}"),
            Error::Unsupported { context, detail } => write!(f, "{context}: unsupported: {detail}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
