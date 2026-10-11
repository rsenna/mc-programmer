//! Shared error type for the library.
//!
//! Every fallible API in `mc_programmer` returns [`Error`]. The messages
//! mirror the Python / Swift libraries so error text stays comparable across
//! the three ports.

use std::fmt;

#[derive(Debug)]
pub enum Error {
    /// A DT1 Sys-Ex message was shorter than the 14-byte framing minimum.
    MessageTooShort(usize),
    /// The message was not framed by `0xF0` … `0xF7`.
    MissingFraming,
    /// The manufacturer ID byte was not Roland's `0x41`.
    NotRoland(u8),
    /// The command byte was not DT1 (`0x12`).
    WrongCommand(u8),
    /// An index argument was out of range (track, clip, step, partial, …).
    Index(String),
    /// A value argument was out of range or of the wrong type.
    Value(String),
    /// No reply to a universal device inquiry.
    NoDeviceResponse,
    /// Invalid UTF-8 encountered while reading a file.
    InvalidUtf8,
    /// The JSON payload was not a valid preset / project file.
    Json(String),
    /// Underlying I/O error from `save` / `load`.
    Io(std::io::Error),
    /// MIDI backend initialization, connection, port, or send error.
    Midi(String),
}

impl fmt::Display for Error {
    /// Format a readable diagnostic, preserving messages from underlying I/O errors.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MessageTooShort(n) => write!(f, "DT1 message too short: {n} bytes"),
            Error::MissingFraming => write!(f, "Missing Sys-Ex framing bytes"),
            Error::NotRoland(id) => write!(f, "Not a Roland message (id 0x{id:02X})"),
            Error::WrongCommand(cmd) => write!(f, "Not a DT1 message (cmd 0x{cmd:02X})"),
            Error::Index(s) | Error::Value(s) | Error::Json(s) | Error::Midi(s) => write!(f, "{s}"),
            Error::NoDeviceResponse => write!(f, "No response to device inquiry"),
            Error::InvalidUtf8 => write!(f, "Invalid UTF-8 in file"),
            Error::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {
    /// Expose the underlying I/O error when this error wraps one.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    /// Wrap a filesystem I/O failure in the shared library error type.
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}
impl From<std::string::FromUtf8Error> for Error {
    fn from(_: std::string::FromUtf8Error) -> Self {
        Error::InvalidUtf8
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
