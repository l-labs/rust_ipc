//! JSON read / write for K tables. Format is row-oriented — an array of
//! objects `[{"col": v, …}, …]`. Common scalars (long / float / bool /
//! string) round-trip; temporal columns are written as ISO-8601 strings and
//! read back as SymbolVec (cast server-side with `"D"$` etc. if needed).

use std::fs;
use std::path::Path;

use serde_json::{Map, Number, Value};

use crate::error::{LError, Result};
use crate::io::{table, temporal::*, unwrap_table};
use crate::k::K;

/// Write table `k` to `path` as a pretty JSON array of row objects.
pub fn write_json(path: impl AsRef<Path>, k: &K) -> Result<()> {
    let (names, cols) = unwrap_table(k, "json write")?;
    let nrows = cols.first().map(|c| c.len()).unwrap_or(0);
    let mut rows = Vec::with_capacity(nrows);
    for r in 0..nrows {                                                         // one object per row
        let mut obj = Map::with_capacity(names.len());
        for (n, c) in names.iter().zip(cols.iter()) {                           // fill the row object
            obj.insert(n.clone(), cell(c, r));
        }
        rows.push(Value::Object(obj));
    }
    let bytes = serde_json::to_vec_pretty(&Value::Array(rows))
        .map_err(|e| LError::Serialize(format!("json: {e}")))?;
    fs::write(path.as_ref(), bytes)
        .map_err(|e| LError::Serialize(format!("json write: {e}")))
}

/// Read `path` (array of row objects) into a table, inferring column types.
pub fn read_json(path: impl AsRef<Path>) -> Result<K> {
    let bytes = fs::read(path.as_ref())
        .map_err(|e| LError::Deserialize(format!("json read: {e}")))?;
    let v: Value = serde_json::from_slice(&bytes)
        .map_err(|e| LError::Deserialize(format!("json parse: {e}")))?;
    let rows = match v {
        Value::Array(r) => r,
        _ => return Err(LError::Deserialize("json: expected top-level \
            array".into())),
    };
    let names: Vec<String> = match rows.first() {                               // schema from first row
        Some(Value::Object(m)) => m.keys().cloned().collect(),
        Some(_) => return Err(LError::Deserialize("json: rows must be \
            objects".into())),
        None    => return Ok(table(Vec::new(), Vec::new())),
    };
    let mut cols: Vec<Vec<Value>> = vec![Vec::with_capacity(rows.len());
        names.len()];
    for row in rows {                                                           // pivot rows -> columns
        let mut obj = match row {
            Value::Object(m) => m,
            _ => return Err(LError::Deserialize("json: rows must be \
                objects".into())),
        };
        for (i, n) in names.iter().enumerate() {
            cols[i].push(obj.remove(n).unwrap_or(Value::Null));
        }
    }
    Ok(table(names, cols.into_iter().map(infer).collect()))
}

// ── helpers ─────────────────────────────────────────────────────────────────
/// Render the i-th element of column `col` as a JSON value.
fn cell(col: &K, i: usize) -> Value {
    use K::*;
    match col {
        BoolVec(v)      => Value::Bool(v[i]),
        ByteVec(v)      => Value::Number(v[i].into()),
        ShortVec(v)     => Value::Number(v[i].into()),
        IntVec(v)       => Value::Number(v[i].into()),
        LongVec(v)      => Value::Number(v[i].into()),
        RealVec(v)      => num_or_null(v[i] as f64),
        FloatVec(v)     => num_or_null(v[i]),
        SymbolVec(v)    => Value::String(v[i].clone()),
        CharVec(v)      =>
            Value::String(String::from_utf8_lossy(&[v[i]]).into_owned()),
        DateVec(v)      => Value::String(fmt_date(v[i])),
        TimeVec(v)      => Value::String(fmt_time(v[i])),
        DateTimeVec(v)  => Value::String(fmt_datetime(v[i])),
        TimestampVec(v) => Value::String(fmt_timestamp(v[i])),
        _               => Value::String(format!("{col}")),                     // fallback spelling
    }
}

fn num_or_null(f: f64) -> Value {                                               // NaN / Inf -> null
    Number::from_f64(f).map(Value::Number).unwrap_or(Value::Null)
}

/// Infer a K column type from JSON values. All-null -> "" symbols; any mixed
/// shape falls back to SymbolVec.
fn infer(cells: Vec<Value>) -> K {
    let (mut int, mut flt, mut boolean, mut null) = (true, true, true, true);
    for v in &cells {
        match v {
            Value::Null    => continue,                                         // nulls don't vote
            Value::Bool(_) => { null = false; int = false; flt = false; }
            Value::Number(n) => {
                null = false; boolean = false;
                if !n.is_i64() && !n.is_u64() { int = false; }                  // not whole
                if n.as_f64().is_none()       { flt = false; }
            }
            _ => { null = false; int = false; flt = false; boolean = false; }
        }
    }
    if null    { return K::SymbolVec(vec![String::new(); cells.len()]); }
    if boolean { return K::BoolVec(                                             // all bools
        cells.iter().map(|v| v.as_bool().unwrap_or(false)).collect()); }
    if int     { return K::LongVec(                                             // all whole numbers
        cells.iter().map(|v| v.as_i64().unwrap_or(0)).collect()); }
    if flt     { return K::FloatVec(                                            // all numbers
        cells.iter().map(|v| v.as_f64().unwrap_or(f64::NAN)).collect()); }
    K::SymbolVec(cells.into_iter().map(|v| match v {                            // strings (or stringify)
        Value::String(s) => s, Value::Null => String::new(), o => o.to_string(),
    }).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_basic() {
        let t = table(vec!["a".into(), "b".into(), "c".into()], vec![
            K::LongVec(vec![1, 2, 3]), K::FloatVec(vec![1.5, 2.5, 3.5]),
            K::SymbolVec(vec!["x".into(), "y".into(), "z".into()])]);
        let path = std::env::temp_dir().join("l_json_test.json");
        write_json(&path, &t).unwrap();
        let back = read_json(&path).unwrap();
        let d = match &back { K::Table(d) => d, _ => panic!("not table") };
        let v = match d.as_ref() { K::Dict(_, v) => v.as_ref(), _ => panic!() };
        match v {
            K::List(v) => {
                assert!(matches!(&v[0], K::LongVec(x)  if x == &vec![1i64, 2,
                    3]));
                assert!(matches!(&v[1], K::FloatVec(x) if x == &vec![1.5, 2.5,
                    3.5]));
                assert!(matches!(&v[2], K::SymbolVec(x)
                    if x == &vec!["x".to_string(), "y".into(), "z".into()]));
            }
            _ => panic!("not list"),
        }
        let _ = std::fs::remove_file(&path);
    }
}
