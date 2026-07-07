//! # l-rs
//!
//! Pure-Rust client for the **L** database. Speaks the L IPC protocol
//! (8-byte header + serialized K) over TCP with zero external dependencies
//! and zero `unsafe`.
//!
//! ```no_run
//! use l_rs::{Connection, K};
//! let mut conn = Connection::connect("localhost", 5001).unwrap();
//! let result = conn.query("select from trade").unwrap();
//! println!("{result}");
//! ```

pub mod types;                                                                  // KType tags + null/inf sentinels
pub mod k;                                                                      // the K value enum + Display
pub mod serialize;                                                              // wire (de)serialization
pub mod ipc;                                                                    // TCP Connection + LZ4
pub mod error;                                                                  // LError + Result
mod convert;                                                                    // From / TryFrom (Rust <-> K)

#[cfg(any(feature = "csv-io", feature = "json-io"))]
pub mod io;                                                                     // CSV / JSON table I/O (feature-gated)

pub use error::LError;
pub use ipc::Connection;
pub use k::K;
pub use types::KType;
