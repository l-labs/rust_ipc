//! CSV read / write for K tables. Cells use interop-friendly spellings (ISO
//! dates, decimal numbers, plain strings) so they open cleanly in Excel /
//! pandas / SQL engines. Reading infers each column in priority order:
//! Long -> Float -> Date -> Time -> Symbol.

use std::path::Path;

use crate::error::{LError, Result};
use crate::io::{table, temporal::*, unwrap_table};
use crate::k::K;

/// Write table `k` to `path` as CSV (header row + one row per record).
pub fn write_csv(path: impl AsRef<Path>, k: &K) -> Result<()> {
    let (cols, data) = unwrap_table(k, "csv write")?;
    let nrows = data.first().map(|c| c.len()).unwrap_or(0);
    let mut w = csv::Writer::from_path(path.as_ref())
        .map_err(|e| LError::Serialize(format!("csv: {e}")))?;
    w.write_record(&cols)                                                       // header
        .map_err(|e| LError::Serialize(format!("csv header: {e}")))?;
    let mut row = Vec::with_capacity(data.len());
    for r in 0..nrows {
        row.clear();
        for c in data { row.push(cell(c, r)); }                                 // format each column
        w.write_record(&row)
            .map_err(|e| LError::Serialize(format!("csv row {r}: {e}")))?;
    }
    w.flush().map_err(|e| LError::Serialize(format!("csv flush: {e}")))?;
    Ok(())
}

/// Read `path` as CSV into a table, inferring each column's K type.
pub fn read_csv(path: impl AsRef<Path>) -> Result<K> {
    let mut rdr = csv::Reader::from_path(path.as_ref())
        .map_err(|e| LError::Deserialize(format!("csv: {e}")))?;
    let names: Vec<String> = rdr.headers()
        .map_err(|e| LError::Deserialize(format!("csv headers: {e}")))?
        .iter().map(String::from).collect();
    let mut cols: Vec<Vec<String>> = vec![Vec::new(); names.len()];
    for rec in rdr.records() {                                                  // gather cells per col
        let rec = rec.map_err(|e| LError::Deserialize(format!("csv record: \
            {e}")))?;
        for (i, field) in rec.iter().enumerate() {
            if i < cols.len() { cols[i].push(field.into()); }
        }
    }
    let kcols: Vec<K> = cols.into_iter().map(infer).collect();
    Ok(table(names, kcols))
}

// ── helpers ─────────────────────────────────────────────────────────────────
/// Render the i-th element of column `col` as a CSV cell string.
fn cell(col: &K, i: usize) -> String {
    use K::*;
    match col {
        BoolVec(v)      => if v[i] { "true".into() } else { "false".into() },
        ByteVec(v)      => v[i].to_string(),
        ShortVec(v)     => v[i].to_string(),
        IntVec(v)       => v[i].to_string(),
        LongVec(v)      => v[i].to_string(),
        RealVec(v)      => v[i].to_string(),
        FloatVec(v)     => v[i].to_string(),
        CharVec(v)      => String::from_utf8_lossy(&[v[i]]).into_owned(),
        SymbolVec(v)    => v[i].clone(),
        DateVec(v)      => fmt_date(v[i]),
        TimeVec(v)      => fmt_time(v[i]),
        DateTimeVec(v)  => fmt_datetime(v[i]),
        TimestampVec(v) => fmt_timestamp(v[i]),
        MinuteVec(v)    => format!("{:02}:{:02}", v[i]/60, v[i]%60),
        SecondVec(v)    => format!("{:02}:{:02}:{:02}",
                                   v[i]/3600, (v[i]/60)%60, v[i]%60),
        other           => format!("{other}"),                                  // last-ditch spelling
    }
}

/// Pick a K column type from string cells (first all-match wins).
fn infer(cells: Vec<String>) -> K {
    let nz: Vec<&str> = cells.iter().map(String::as_str)                        // ignore blanks for
        .filter(|s| !s.is_empty()).collect();                                   // the type vote
    if nz.is_empty() { return K::SymbolVec(cells); }
    let all = |f: &dyn Fn(&str) -> bool| nz.iter().all(|s| f(s));
    if all(&|s| s.parse::<i64>().is_ok()) {
        K::LongVec(cells.iter().map(|s| s.parse().unwrap_or(0)).collect())
    } else if all(&|s| s.parse::<f64>().is_ok()) {
        K::FloatVec(cells.iter().map(|s|
            s.parse().unwrap_or(f64::NAN)).collect())
    } else if all(&|s| parse_date_iso(s).is_some()) {
        K::DateVec(cells.iter().map(|s|
            parse_date_iso(s).unwrap_or(0)).collect())
    } else if all(&|s| parse_time_iso(s).is_some()) {
        K::TimeVec(cells.iter().map(|s|
            parse_time_iso(s).unwrap_or(0)).collect())
    } else {
        K::SymbolVec(cells)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_basic() {
        let t = table(vec!["a".into(), "b".into(), "d".into()], vec![
            K::LongVec(vec![1, 2, 3]), K::FloatVec(vec![1.5, 2.5, 3.5]),
            K::DateVec(vec![0, 1, 365])]);
        let path = std::env::temp_dir().join("l_csv_test.csv");
        write_csv(&path, &t).unwrap();
        let back = read_csv(&path).unwrap();
        let d = match &back { K::Table(d) => d, _ => panic!("not table") };
        let v = match d.as_ref() { K::Dict(_, v) => v.as_ref(), _ => panic!() };
        match v {
            K::List(v) => {
                assert!(matches!(&v[0], K::LongVec(x)  if x == &vec![1i64, 2,
                    3]));
                assert!(matches!(&v[1], K::FloatVec(x) if x == &vec![1.5, 2.5,
                    3.5]));
                assert!(matches!(&v[2], K::DateVec(x)  if x == &vec![0i32, 1,
                    365]));
            }
            _ => panic!("not list"),
        }
        let _ = std::fs::remove_file(&path);
    }
    #[test]
    fn date_roundtrip() {                                                       // ymd <-> days is exact
        for d in [-1000i32, 0, 1, 365, 10000, 100000] {
            let (y, m, dd) = ymd_from_days(d);
            assert_eq!(d, days_from_ymd(y, m, dd), "ymd round-trip at {d}");
        }
    }
}
