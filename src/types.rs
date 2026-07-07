//! L type tags. Negative tag = atom, positive = vector, 0 = list.
//! In the L IPC protocol the default integer is 32-bit (`Int`=6). One
//! `ktypes!` table drives the enum, the wire-tag decoder (`from_raw`) and
//! the per-element byte size (`size`).

use core::mem::size_of;                                                         // ptr-sized `Symbol` slot

/// Emit `KType` + `from_raw` + `size` from one `Name = wiretag, bytes;` table.
macro_rules! ktypes { ($($n:ident = $t:expr, $z:expr;)*) => {
    /// K type tag. Atoms are negative on the wire, vectors positive, list = 0.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[repr(i16)]
    pub enum KType { $($n = $t),* }                                             // discriminant == positive wire tag

    impl KType {
        /// Decode a raw wire tag (atom or vector — sign is folded away).
        pub fn from_raw(t: i16) -> Option<KType> {
            let a = t.unsigned_abs();                                           // |tag|: atom and its vector share it
            $(if a == ($t as i16).unsigned_abs() { return Some(KType::$n); })*
            None                                                                // 20..97 enum/temporal, 100+ function types: not modeled
        }
        /// Bytes per element; 0 for the symbol-keyed / compound / error types.
        pub fn size(&self) -> usize { match self { $(KType::$n => $z),* } }
    }
}}

ktypes! {
    MixedList = 0,    0;                                                        // generic list of K objects
    Boolean   = 1,    1;                                                        // 1 byte, 0 or 1
    Byte      = 4,    1;                                                        // unsigned 8-bit
    Short     = 5,    2;                                                        // signed 16-bit
    Int       = 6,    4;                                                        // signed 32-bit — l's default int
    Long      = 7,    8;                                                        // signed 64-bit
    Real      = 8,    4;                                                        // IEEE-754 single (f32)
    Float     = 9,    8;                                                        // IEEE-754 double (f64) — "float"=f64
    Char      = 10,   1;                                                        // ASCII byte
    Symbol    = 11,   size_of::<usize>();                                       // nominal slot; NUL-terminated on wire
    Timestamp = 12,   8;                                                        // ns since 2000.01.01
    Month     = 13,   4;                                                        // months since 2000.01
    Date      = 14,   4;                                                        // days since 2000.01.01
    DateTime  = 15,   8;                                                        // fractional days since 2000.01.01
    Timespan  = 16,   8;                                                        // ns duration (type 16)
    Minute    = 17,   4;                                                        // minutes past midnight
    Second    = 18,   4;                                                        // seconds past midnight
    Time      = 19,   4;                                                        // milliseconds past midnight
    Table     = 98,   0;                                                        // flip of a column dict
    Dict      = 99,   0;                                                        // keys ! values
    Error     = -128, 0;                                                        // wire tag -128
}

// ── null / infinity sentinels (L wire values) ──────────────────────────────
pub const NULL_INT:   i32 = i32::MIN;                                           // 0x80000000
pub const NULL_LONG:  i64 = i64::MIN;                                           // 0x8000000000000000
pub const NULL_SHORT: i16 = i16::MIN;                                           // 0x8000
pub const NULL_FLOAT: f64 = f64::NAN;                                           // NaN — compares false to everything
pub const NULL_REAL:  f32 = f32::NAN;                                           // NaN (single)
pub const INF_INT:    i32 = i32::MAX;                                           // 0x7FFFFFFF
pub const INF_LONG:   i64 = i64::MAX;                                           // 0x7FFFFFFFFFFFFFFF
pub const INF_FLOAT:  f64 = f64::INFINITY;
