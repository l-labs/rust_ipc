//! Error type for l-rs. `K::Error(msg)` is a *value* the server returned;
//! `Err(LError::L(msg))` means the IPC/protocol layer itself failed.

use std::{fmt, io};

/// Everything that can go wrong in the client.
#[derive(Debug)]
pub enum LError {
    Io(io::Error),                                                              // TCP / socket failure
    L(String),                                                                  // server signalled 'type, 'length, …
    AuthFailed,                                                                 // handshake rejected (handle == 0)
    ConnectionFailed(String),                                                   // connect() refused / unreachable
    Deserialize(String),                                                        // bad bytes arriving from the server
    Serialize(String),                                                          // a K value we cannot put on the wire
    Type(String),                                                               // K type mismatch during conversion
}

impl fmt::Display for LError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use LError::*;
        match self {
            // one human line per case
            Io(e) => write!(f, "IO error: {e}"),
            L(s) => write!(f, "l error: '{s}"),
            AuthFailed => write!(f, "authentication failed"),
            ConnectionFailed(s) => write!(f, "connection failed: {s}"),
            Deserialize(s) => write!(f, "deserialize error: {s}"),
            Serialize(s) => write!(f, "serialize error: {s}"),
            Type(s) => write!(f, "type error: {s}"),
        }
    }
}

impl std::error::Error for LError {}
impl From<io::Error> for LError {
    // `?` on any std::io call
    fn from(e: io::Error) -> Self {
        LError::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, LError>;                            // crate-wide alias
