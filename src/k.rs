//! K — l's universal value. A pure-Rust enum: atoms are scalars,
//! vectors are homogeneous `Vec<T>`, and the rest are dict / table / list /
//! error / null. No FFI, no raw pointers; every `K` owns its data.

use std::fmt;

/// A K object. Variant *names* are public API (tests, examples, and
/// downstream bindings match on them) — only the internals are terse.
#[derive(Debug, Clone, PartialEq)]
pub enum K {
    // ── atoms ─────────────────────────────────────────────────────────────
    Bool(bool), Byte(u8), Short(i16), Int(i32), Long(i64), Real(f32),
    Float(f64), Char(u8), Symbol(String),
    Timestamp(i64),                                                             // ns since 2000.01.01
    Month(i32),                                                                 // months since 2000.01
    Date(i32),                                                                  // days since 2000.01.01
    DateTime(f64),                                                              // fractional days since 2000.01.01
    Minute(i32), Second(i32), Time(i32),                                        // minutes / seconds / ms past midnight
    Timespan(i64),                                                              // ns duration (type 16)

    // ── vectors (homogeneous) ───────────────────────────────────────────────
    BoolVec(Vec<bool>), ByteVec(Vec<u8>), ShortVec(Vec<i16>), IntVec(Vec<i32>),
    LongVec(Vec<i64>), RealVec(Vec<f32>), FloatVec(Vec<f64>),
    CharVec(Vec<u8>),                                                           // a string on the wire (type 10)
    SymbolVec(Vec<String>),
    TimestampVec(Vec<i64>), MonthVec(Vec<i32>), DateVec(Vec<i32>),
    DateTimeVec(Vec<f64>), MinuteVec(Vec<i32>), SecondVec(Vec<i32>),
    TimeVec(Vec<i32>),
    TimespanVec(Vec<i64>),                                                      // ns duration vector (type 16)

    // ── compound ────────────────────────────────────────────────────────────
    List(Vec<K>),                                                               // mixed / heterogeneous list (type 0)
    Dict(Box<K>, Box<K>),                                                       // (keys, values) -> type 99
    Table(Box<K>),                                                              // flip of a symbol-keyed column dict -> type 98
    Error(String),                                                              // type -128
    Null,                                                                       // identity / nil (::)
}

/// `type_tag`: atom = `-t`, its vector = `+t`. Symmetric pairs come from one
/// table; the four irregular tags are listed explicitly.
macro_rules! tags { ($($a:ident / $v:ident = $t:literal),* $(,)?) => {
    impl K {
        /// Wire-format type tag for this value (negative = atom).
        pub fn type_tag(&self) -> i16 { match self {
            $( K::$a(..) => -$t, K::$v(..) => $t, )*
            K::List(_) => 0, K::Dict(..) => 99,                                 // generic list / dict
            K::Table(_) => 98, K::Error(_) => -128,                             // table / error
            K::Null => 101,                                                     // identity (::)
        }}
    }
}}
tags! {
    Bool/BoolVec=1, Byte/ByteVec=4, Short/ShortVec=5, Int/IntVec=6,
    Long/LongVec=7, Real/RealVec=8, Float/FloatVec=9, Char/CharVec=10,
    Symbol/SymbolVec=11, Timestamp/TimestampVec=12, Month/MonthVec=13,
    Date/DateVec=14, DateTime/DateTimeVec=15, Minute/MinuteVec=17,
    Second/SecondVec=18, Time/TimeVec=19, Timespan/TimespanVec=16,
}

impl K {
    /// Element count: vectors -> len, dict -> 2, table -> cols, atom -> 1.
    pub fn len(&self) -> usize {
        macro_rules! n { ($($v:ident),*) => { match self {
            $( K::$v(x) => x.len(), )*                                          // every Vec-shaped arm
            K::Dict(..) => 2, K::Table(d) => d.len(), _ => 1,
        }}}
        n!(BoolVec, ByteVec, ShortVec, IntVec, LongVec, RealVec, FloatVec,
           CharVec, SymbolVec, TimestampVec, MonthVec, DateVec, DateTimeVec,
           MinuteVec, SecondVec, TimeVec, TimespanVec, List)
    }
    /// Empty vector / table?
    pub fn is_empty(&self) -> bool { self.len() == 0 }
    /// Scalar (negative tag, but not the -128 error)?
    pub fn is_atom(&self) -> bool { let t = self.type_tag(); t < 0 && t !=
        -128 }
    /// Homogeneous vector (tag 1..19)?
    pub fn is_vector(&self) -> bool { let t = self.type_tag(); t > 0 && t < 20 }
    /// A char vector (L string)?
    pub fn is_string(&self) -> bool { matches!(self, K::CharVec(_)) }

    /// Borrow as `&str` (char vector decoded UTF-8, or a symbol).
    pub fn as_string(&self) -> Option<&str> {
        match self {                                                            // CharVec or Symbol
            K::CharVec(v) => std::str::from_utf8(v).ok(),
            K::Symbol(s)  => Some(s.as_str()),
            _ => None,
        }
    }
    /// Widen any integral atom to i32.
    pub fn as_int(&self) -> Option<i32> {
        match self {                                                            // bool/byte/short/int
            K::Int(v) => Some(*v), K::Short(v) => Some(*v as i32),
            K::Byte(v) => Some(*v as i32), K::Bool(v) => Some(*v as i32),
            _ => None,
        }
    }
    /// Widen any numeric atom to f64.
    pub fn as_float(&self) -> Option<f64> {
        match self {                                                            // float/real/int/long
            K::Float(v) => Some(*v), K::Real(v) => Some(*v as f64),
            K::Int(v) => Some(*v as f64), K::Long(v) => Some(*v as f64),
            _ => None,
        }
    }
    /// Widen int / long atom to i64.
    pub fn as_long(&self) -> Option<i64> {
        match self { K::Long(v) => Some(*v), K::Int(v) => Some(*v as i64),
                     _ => None }
    }
}

// ── temporal core — all L times count from 2000.01.01 ────────────────────
// The calendar math (Howard Hinnant's civil-date algorithm on the 0000-03-01
// era) is shared with `io/`; the literal formatters below stay local so
// `K::Display` round-trips through L's own parser.
pub(crate) const NANOS_PER_DAY: i64 = 86_400_000_000_000;
pub(crate) const MS_PER_DAY:    i64 = 86_400_000;

/// days-since-2000.01.01 -> (year, month, day).
pub(crate) fn ymd_from_days(d: i32) -> (i32, u32, u32) {
    let z = d as i64 + 730_425;                                                 // shift onto 0000-03-01
    let era = z.div_euclid(146_097);                                            // 400-year cycle index
    let doe = z.rem_euclid(146_097) as u64;                                     // day-of-era [0,146096]
    let yoe = (doe - doe/1460 + doe/36524 - doe/146096) / 365;                  // year-of-era
    let y_era = yoe as i64 + era * 400;
    let doy = doe - (365*yoe + yoe/4 - yoe/100);                                // day-of-year [0,365]
    let mp  = (5*doy + 2) / 153;                                                // shifted month [0,11]
    let day = (doy - (153*mp + 2)/5 + 1) as u32;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let year  = (y_era + if month <= 2 { 1 } else { 0 }) as i32;
    (year, month, day)
}

/// Inverse of `ymd_from_days` — (year, month, day) -> days-since-2000.01.01.
#[cfg(any(feature = "csv-io", feature = "json-io"))]                            // only io/ parses dates
pub(crate) fn days_from_ymd(y: i32, m: u32, d: u32) -> i32 {
    let y = if m <= 2 { y - 1 } else { y } as i64;                              // March-rooted year
    let m = if m <= 2 { m + 9 } else { m - 3 } as i64;
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153*m + 2)/5 + d as i64 - 1;
    let doe = yoe*365 + yoe/4 - yoe/100 + doy;
    (era*146_097 + doe - 730_425) as i32
}

fn fmt_date(d: i32) -> String {                                                 // 2000.01.01
    let (y, m, dd) = ymd_from_days(d); format!("{y:04}.{m:02}.{dd:02}")
}
fn fmt_month(m: i32) -> String {                                                // 2000.01m
    let (y, mo) = (2000 + m.div_euclid(12), 1 + m.rem_euclid(12) as u32);
    format!("{y:04}.{mo:02}m")
}
pub(crate) fn fmt_time(ms: i32) -> String {                                     // 00:00:00.000 (io/ reuses)
    let ms = ms as i64;
    let (h, m, s, r) = (ms/3_600_000, (ms/60_000)%60, (ms/1000)%60, ms%1000);
    format!("{h:02}:{m:02}:{s:02}.{r:03}")
}
/// Split nanoseconds-of-day into (hour, minute, second, nanosecond).
pub(crate) fn hmsn(nod: i64) -> (i64, i64, i64, i64) {
    (nod/3_600_000_000_000, (nod/60_000_000_000)%60, (nod/1_000_000_000)%60,
     nod%1_000_000_000)
}
fn fmt_minute(m: i32) -> String { format!("{:02}:{:02}", m/60, m%60) }
fn fmt_second(s: i32) -> String {                                               // 00:00:00
    format!("{:02}:{:02}:{:02}", s/3600, (s/60)%60, s%60)
}
fn fmt_datetime(d: f64) -> String {                                             // 2000.01.01T00:00:00.000
    let whole = d.floor() as i32;
    let ms = ((d - d.floor()) * MS_PER_DAY as f64).round() as i32;
    format!("{}T{}", fmt_date(whole), fmt_time(ms))
}
fn fmt_timestamp(ns: i64) -> String {                                           // 2000.01.01D00:00:00.000000000
    let days = ns.div_euclid(NANOS_PER_DAY) as i32;
    let (h, m, s, n) = hmsn(ns.rem_euclid(NANOS_PER_DAY));                      // split ns-of-day
    format!("{}D{h:02}:{m:02}:{s:02}.{n:09}", fmt_date(days))
}
fn fmt_timespan(ns: i64) -> String {                                            // [-]0D00:00:00.000000000
    let (sign, a) = if ns < 0 { ("-", -ns) } else { ("", ns) };                 // sign + magnitude
    let (h, m, s, n) = hmsn(a % NANOS_PER_DAY);                                 // split ns-of-day
    format!("{sign}{}D{h:02}:{m:02}:{s:02}.{n:09}", a / NANOS_PER_DAY)
}

/// Space-join a slice through a per-element formatter (L vector literal).
fn join<T>(v: &[T], f: impl Fn(&T) -> String) -> String {
    v.iter().map(f).collect::<Vec<_>>().join(" ")
}

impl fmt::Display for K {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use K::*;
        match self {
            // ── atoms ───────────────────────────────────────────────────────
            Bool(v)      => write!(f, "{}b", *v as u8),
            Byte(v)      => write!(f, "0x{v:02x}"),
            Short(v)     => write!(f, "{v}h"),
            Int(v)       => write!(f, "{v}"),
            Long(v)      => write!(f, "{v}"),
            Real(v)      => write!(f, "{v}e"),
            Float(v)     => write!(f, "{v}"),
            Char(v)      => write!(f, "\"{}\"", *v as char),
            Symbol(s)    => write!(f, "`{s}"),
            Timestamp(n) => write!(f, "{}", fmt_timestamp(*n)),
            Month(m)     => write!(f, "{}", fmt_month(*m)),
            Date(d)      => write!(f, "{}", fmt_date(*d)),
            DateTime(d)  => write!(f, "{}", fmt_datetime(*d)),
            Minute(m)    => write!(f, "{}", fmt_minute(*m)),
            Second(s)    => write!(f, "{}", fmt_second(*s)),
            Time(t)      => write!(f, "{}", fmt_time(*t)),
            Timespan(n)  => write!(f, "{}", fmt_timespan(*n)),

            // ── vectors ─────────────────────────────────────────────────────
            BoolVec(v)   => {                                                   // 0/1 digits then `b`
                for b in v { write!(f, "{}", *b as u8)?; } write!(f, "b")
            }
            ByteVec(v)   => {                                                   // `0x` then hex pairs
                write!(f, "0x")?; for b in v { write!(f, "{b:02x}")?; } Ok(())
            }
            ShortVec(v)  => write!(f, "{}h", join(v, |x| x.to_string())),
            IntVec(v)    => write!(f, "{}",  join(v, |x| x.to_string())),
            LongVec(v)   => write!(f, "{}",  join(v, |x| x.to_string())),
            RealVec(v)   => write!(f, "{}e", join(v, |x| x.to_string())),
            FloatVec(v)  => write!(f, "{}",  join(v, |x| x.to_string())),
            CharVec(v)   => write!(f, "\"{}\"", String::from_utf8_lossy(v)),
            SymbolVec(v) => { for s in v { write!(f, "`{s}")?; } Ok(()) }
            TimestampVec(v) => write!(f, "{}", join(v, |x| fmt_timestamp(*x))),
            MonthVec(v)     => write!(f, "{}", join(v, |x| fmt_month(*x))),
            DateVec(v)      => write!(f, "{}", join(v, |x| fmt_date(*x))),
            DateTimeVec(v)  => write!(f, "{}", join(v, |x| fmt_datetime(*x))),
            MinuteVec(v)    => write!(f, "{}", join(v, |x| fmt_minute(*x))),
            SecondVec(v)    => write!(f, "{}", join(v, |x| fmt_second(*x))),
            TimeVec(v)      => write!(f, "{}", join(v, |x| fmt_time(*x))),
            TimespanVec(v)  => write!(f, "{}", join(v, |x| fmt_timespan(*x))),

            // ── compound ────────────────────────────────────────────────────
            List(v)   => write!(f, "({})",                                      // elements split by ;
                v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(";")),
            Dict(k, v) => write!(f, "{k}!{v}"),
            Table(d)  => write!(f, "+{d}"),
            Error(s)  => write!(f, "'{s}"),
            Null      => write!(f, "::"),
        }
    }
}
