//! Local file I/O for K tables — CSV and JSON, behind feature flags so the
//! core library stays zero-dependency by default.

#[cfg(feature = "csv-io")]  pub mod csv;
#[cfg(feature = "json-io")] pub mod json;

// ── shared table destructure (csv + json writers) ──────────────────────────
#[cfg(any(feature = "csv-io", feature = "json-io"))]
use crate::{error::{LError, Result}, k::K};

/// Pull `(column-names, columns)` out of a table (or bare column dict). `ctx`
/// tags any error (e.g. "csv write"). Borrows the column list in place.
#[cfg(any(feature = "csv-io", feature = "json-io"))]
pub(crate) fn unwrap_table<'a>(k: &'a K, ctx: &str)
    -> Result<(Vec<String>, &'a Vec<K>)> {
    let dict = match k {                                                        // table -> its col dict
        K::Table(d)     => d.as_ref(),
        d @ K::Dict(..) => d,
        _ => return Err(LError::Type(
            format!("{ctx}: expected table, got type {}", k.type_tag()))),
    };
    let (keys, vals) = match dict {                                             // dict is (keys, vals)
        K::Dict(k, v) => (k.as_ref(), v.as_ref()), _ => unreachable!(),
    };
    let names = match keys {                                                    // keys must be symbols
        K::SymbolVec(s) => s.clone(),
        _ => return Err(LError::Type(format!("{ctx}: names must be symbols"))),
    };
    let cols = match vals {                                                     // vals must be a list
        K::List(v) => v,
        _ => return Err(LError::Type(format!("{ctx}: columns must be a list"))),
    };
    if names.len() != cols.len() {                                              // shapes must agree
        return Err(LError::Type(
            format!("{ctx}: {} names but {} columns", names.len(),
                cols.len())));
    }
    Ok((names, cols))
}

/// Inverse of `unwrap_table` — build a table K from names + columns.
#[cfg(any(feature = "csv-io", feature = "json-io"))]
pub(crate) fn table(names: Vec<String>, cols: Vec<K>) -> K {                     // (names!cols) flipped
    K::Table(Box::new(K::Dict(Box::new(K::SymbolVec(names)),
                              Box::new(K::List(cols)))))
}

// ── temporal: ISO-8601 spellings (csv + json) ──────────────────────────────
// L's temporal types count from 2000-01-01. The civil-date *math* lives in
// `crate::k` (shared with `K::Display`); here we only add the ISO `-`/`T`
// formatters and their inverse parsers, round-trippable with each other.
#[cfg(any(feature = "csv-io", feature = "json-io"))]
pub(crate) mod temporal {
    pub(crate) use crate::k::{days_from_ymd, fmt_time, ymd_from_days};          // shared math + HH:MM:SS.mmm
    use crate::k::{hmsn, MS_PER_DAY, NANOS_PER_DAY};

    pub fn fmt_date(d: i32) -> String {                                         // YYYY-MM-DD
        let (y, m, dd) = ymd_from_days(d); format!("{y:04}-{m:02}-{dd:02}")
    }
    pub fn fmt_datetime(d: f64) -> String {                                     // date T time
        let whole = d.floor() as i32;
        let ms = ((d - d.floor()) * MS_PER_DAY as f64).round() as i32;
        format!("{}T{}", fmt_date(whole), fmt_time(ms))
    }
    pub fn fmt_timestamp(ns: i64) -> String {                                   // date T time.nanos
        let days = ns.div_euclid(NANOS_PER_DAY) as i32;
        let (h, m, s, n) = hmsn(ns.rem_euclid(NANOS_PER_DAY));                  // split ns-of-day
        format!("{}T{h:02}:{m:02}:{s:02}.{n:09}", fmt_date(days))
    }

    pub fn parse_date_iso(s: &str) -> Option<i32> {                             // YYYY-MM-DD or .
        let s = s.trim();
        if s.len() != 10 { return None; }
        let b = s.as_bytes();
        if (b[4] != b'-' && b[4] != b'.') || (b[7] != b'-' && b[7] != b'.') {
            return None;                                                        // separators must match
        }
        let (y, m, d): (i32, u32, u32) =
            (s[0..4].parse().ok()?, s[5..7].parse().ok()?,
                s[8..10].parse().ok()?);
        if !(1..=12).contains(&m) || !(1..=31).contains(&d) { return None; }
        Some(days_from_ymd(y, m, d))
    }

    pub fn parse_time_iso(s: &str) -> Option<i32> {                             // HH:MM:SS[.sss]
        let mut parts = s.trim().split(':');
        let h: i32 = parts.next()?.parse().ok()?;
        let m: i32 = parts.next()?.parse().ok()?;
        let sec_part = parts.next()?;
        if parts.next().is_some() { return None; }                              // too many ':' groups
        let (sec, ms) = match sec_part.split_once('.') {
            Some((s, frac)) => {                                                // fractional seconds
                let s: i32 = s.parse().ok()?;
                let take = frac.len().min(3);                                   // keep ms precision
                let mut ms: i32 = frac[..take].parse().ok()?;
                if take < 3 { ms *= 10_i32.pow((3 - take) as u32); }
                (s, ms)
            }
            None => (sec_part.parse().ok()?, 0),
        };
        if !(0..24).contains(&h) || !(0..60).contains(&m) ||
            !(0..60).contains(&sec) {
            return None;                                                        // out-of-range field
        }
        Some((h*3600 + m*60 + sec)*1000 + ms)
    }
}
