//! L IPC wire (de)serialization. Layout per value:
//!   atom   : tag(1) + little-endian payload
//!   vector : tag(1) + attr(1) + count(i32 LE) + count*elem little-endian
//!   symbol : NUL-terminated UTF-8 bytes
//!   dict   : keys-K then values-K (no header); table: tag(1)+attr(1)+dict-K
//! Everything is little-endian (the only endianness l speaks), so each
//! POD vector is one bounds-checked slice fed through `chunks_exact` — a
//! shape LLVM autovectorizes into NEON/AVX byte moves, no scalar tail loop.

use crate::error::{LError, Result};
use crate::k::K;
use core::mem::size_of;

// ── write helpers (LE, bulk) ───────────────────────────────────────────────
macro_rules! pa { ($b:expr, $t:expr, $v:expr) => {{                             // atom: tag + LE bytes
    $b.push($t); $b.extend_from_slice(&$v.to_le_bytes());
}}}
macro_rules! hdr { ($b:expr, $t:expr, $n:expr) => {{                            // vector header
    $b.push($t); $b.push(0); $b.extend_from_slice(&($n as i32).to_le_bytes());
}}}
macro_rules! pb { ($b:expr, $t:expr, $v:expr) => {{                             // raw-byte vector
    hdr!($b, $t, $v.len()); $b.extend_from_slice($v);
}}}
macro_rules! pv { ($b:expr, $t:expr, $v:expr) => {{                             // POD vector, LE per elt
    hdr!($b, $t, $v.len());
    for x in $v { $b.extend_from_slice(&x.to_le_bytes()); }                     // autovectorizes
}}}

/// Serialize a K value into freshly-allocated l wire bytes.
pub fn serialize(k: &K) -> Result<Vec<u8>> {
    let mut buf = Vec::new(); write_k(&mut buf, k)?; Ok(buf)
}

fn write_k(buf: &mut Vec<u8>, k: &K) -> Result<()> {
    match k {
        // ── atoms (negative tags) ───────────────────────────────────────────
        K::Bool(v)      => pa!(buf, 0xff, (*v as u8)),                          // -1
        K::Byte(v)      => pa!(buf, 0xfc, v),                                   // -4
        K::Short(v)     => pa!(buf, 0xfb, v),                                   // -5
        K::Int(v)       => pa!(buf, 0xfa, v),                                   // -6
        K::Long(v)      => pa!(buf, 0xf9, v),                                   // -7
        K::Real(v)      => pa!(buf, 0xf8, v),                                   // -8
        K::Float(v)     => pa!(buf, 0xf7, v),                                   // -9
        K::Char(v)      => pa!(buf, 0xf6, v),                                   // -10
        K::Symbol(s)    => wr_sym(buf, 0xf5, s),                                // -11 (NUL-terminated)
        K::Timestamp(v) => pa!(buf, 0xf4, v),                                   // -12
        K::Month(v)     => pa!(buf, 0xf3, v),                                   // -13
        K::Date(v)      => pa!(buf, 0xf2, v),                                   // -14
        K::DateTime(v)  => pa!(buf, 0xf1, v),                                   // -15
        K::Timespan(v)  => pa!(buf, 0xf0, v),                                   // -16
        K::Minute(v)    => pa!(buf, 0xef, v),                                   // -17
        K::Second(v)    => pa!(buf, 0xee, v),                                   // -18
        K::Time(v)      => pa!(buf, 0xed, v),                                   // -19

        // ── vectors (positive tags) ─────────────────────────────────────────
        K::BoolVec(v)   => { let t: Vec<u8> =                                   // bools -> 0/1 bytes
                             v.iter().map(|b| *b as u8).collect(); pb!(buf, 1,
                                 &t); }
        K::ByteVec(v)   => pb!(buf, 4, v),
        K::ShortVec(v)  => pv!(buf, 5, v),
        K::IntVec(v)    => pv!(buf, 6, v),
        K::LongVec(v)   => pv!(buf, 7, v),
        K::RealVec(v)   => pv!(buf, 8, v),
        K::FloatVec(v)  => pv!(buf, 9, v),
        K::CharVec(v)   => pb!(buf, 10, v),                                     // string bytes
        K::SymbolVec(v) => { hdr!(buf, 11, v.len());                            // n NUL-terminated syms
                             for s in v { wr_bytes(buf, s); } }
        K::TimestampVec(v) => pv!(buf, 12, v),
        K::MonthVec(v)     => pv!(buf, 13, v),
        K::DateVec(v)      => pv!(buf, 14, v),
        K::DateTimeVec(v)  => pv!(buf, 15, v),
        K::TimespanVec(v)  => pv!(buf, 16, v),
        K::MinuteVec(v)    => pv!(buf, 17, v),
        K::SecondVec(v)    => pv!(buf, 18, v),
        K::TimeVec(v)      => pv!(buf, 19, v),

        // ── compound ────────────────────────────────────────────────────────
        K::List(xs)     => { hdr!(buf, 0, xs.len());                            // type-0 mixed list
                             for x in xs { write_k(buf, x)?; } }
        K::Dict(k, v)   => { buf.push(99); write_k(buf, k)?; write_k(buf, v)?; }
        K::Table(d)     => { buf.push(98); buf.push(0); write_k(buf, d)?; }
        K::Error(s)     => wr_sym(buf, 0x80, s),                                // -128 + NUL string
        K::Null         => { buf.push(0x65); buf.push(0); }                     // 101 identity
    }
    Ok(())
}

fn wr_bytes(buf: &mut Vec<u8>, s: &str) {                                       // UTF-8 + NUL
    buf.extend_from_slice(s.as_bytes()); buf.push(0);
}
fn wr_sym(buf: &mut Vec<u8>, tag: u8, s: &str) {                                // tag then NUL string
    buf.push(tag); wr_bytes(buf, s);
}

// ── read helpers (bounds-checked, bulk) ─────────────────────────────────────
/// Borrow `n` bytes at `*p`, advancing past them; errors before any OOB read.
fn take<'a>(d: &'a [u8], p: &mut usize, n: usize) -> Result<&'a [u8]> {
    let end = p.checked_add(n).filter(|&e| e <= d.len()).ok_or_else(|| {
        LError::Deserialize(format!("need {n} bytes at {}, have {}", *p,
            d.len()))
    })?;
    let s = &d[*p..end]; *p = end; Ok(s)                                        // advance the cursor
}
/// Read a vector header (skip attr byte, take i32 count). A count is rejected
/// when negative or larger than the bytes that remain: every element costs at
/// least one wire byte, so a count past `remaining` is by definition corrupt.
/// This stops a hostile length from sizing a giant `Vec` before any data is
/// even read (a negative count would cast to a near-`usize::MAX` request and
/// abort the process in the allocator).
fn rd_n(d: &[u8], p: &mut usize) -> Result<usize> {
    take(d, p, 1)?;                                                             // attr — unused here
    let n = i32::from_le_bytes(take(d, p, 4)?.try_into().unwrap());
    if n < 0 || n as usize > d.len() - *p {                                    // *p is past the header
        return Err(LError::Deserialize(format!("bad vector count: {n}")));
    }
    Ok(n as usize)
}

/// A safe pre-allocation hint. A wire count is never trusted to size a `Vec`
/// directly — a large one would abort in the allocator before a single byte
/// is validated. Clamp to what could actually fit (>=1 wire byte per element)
/// and to a fixed ceiling, letting the `Vec` grow for the rare huge value.
fn cap_hint(n: usize, remaining: usize) -> usize {
    n.min(remaining).min(1 << 16)                                              // ceiling ~64k elements
}
/// Read a NUL-terminated symbol (lossy UTF-8), consuming the terminator.
fn rd_sym(d: &[u8], p: &mut usize) -> Result<String> {
    let start = *p;
    while *p < d.len() && d[*p] != 0 { *p += 1; }                               // scan to NUL / EOF
    let s = String::from_utf8_lossy(&d[start..*p]).into_owned();
    if *p < d.len() { *p += 1; } Ok(s)                                          // step over the NUL
}

macro_rules! ga { ($d:expr, $p:expr, $ty:ty) => {                               // atom: one POD, LE
    <$ty>::from_le_bytes(take($d, $p, size_of::<$ty>())?.try_into().unwrap())
}}
macro_rules! gv { ($d:expr, $p:expr, $ty:ty) => {{                              // POD vector, bulk LE
    let w = size_of::<$ty>(); let n = rd_n($d, $p)?;
    let len = n.checked_mul(w)                                                  // guard count*width
        .ok_or_else(|| LError::Deserialize("vector too large".into()))?;
    take($d, $p, len)?.chunks_exact(w)                                          // one OOB check, then
        .map(|c| <$ty>::from_le_bytes(c.try_into().unwrap()))                   // autovectorized
        .collect::<Vec<$ty>>()
}}}

/// Deserialize one K value from the front of `data`.
pub fn deserialize(data: &[u8]) -> Result<K> { read_k(data, &mut 0, 0) }

/// Cap on `List`/`Dict`/`Table` nesting. A hostile stream of list-of-list
/// headers would otherwise recurse once per level and overflow the stack —
/// an uncatchable abort; past this depth we bail with an ordinary error.
/// Kept well below what a 2 MiB worker-thread stack tolerates (recursion is
/// fat in debug builds), yet far past any real reply, which nests only a few
/// levels deep (table = dict = (symbols; list of columns)).
const MAX_DEPTH: usize = 128;

fn read_k(d: &[u8], p: &mut usize, depth: usize) -> Result<K> {
    if depth > MAX_DEPTH {
        return Err(LError::Deserialize("nesting too deep".into()));
    }
    let t = take(d, p, 1)?[0] as i8;                                            // signed wire tag
    Ok(match t {
        // ── atoms ───────────────────────────────────────────────────────────
        -1   => K::Bool(ga!(d, p, u8) != 0),
        -4   => K::Byte(ga!(d, p, u8)),
        -5   => K::Short(ga!(d, p, i16)),
        -6   => K::Int(ga!(d, p, i32)),
        -7   => K::Long(ga!(d, p, i64)),
        -8   => K::Real(ga!(d, p, f32)),
        -9   => K::Float(ga!(d, p, f64)),
        -10  => K::Char(ga!(d, p, u8)),
        -11  => K::Symbol(rd_sym(d, p)?),
        -12  => K::Timestamp(ga!(d, p, i64)),
        -13  => K::Month(ga!(d, p, i32)),
        -14  => K::Date(ga!(d, p, i32)),
        -15  => K::DateTime(ga!(d, p, f64)),
        -16  => K::Timespan(ga!(d, p, i64)),
        -17  => K::Minute(ga!(d, p, i32)),
        -18  => K::Second(ga!(d, p, i32)),
        -19  => K::Time(ga!(d, p, i32)),
        -128 => K::Error(rd_sym(d, p)?),

        // ── vectors ─────────────────────────────────────────────────────────
        0  => { let n = rd_n(d, p)?;                                            // capacity is a hint,
                let mut xs = Vec::with_capacity(cap_hint(n, d.len() - *p));     // not the trusted count
                for _ in 0..n { xs.push(read_k(d, p, depth + 1)?); }
                K::List(xs) }
        1  => { let n = rd_n(d, p)?;                                            // bytes -> bools
                K::BoolVec(take(d, p, n)?.iter().map(|b| *b != 0).collect()) }
        4  => { let n = rd_n(d, p)?; K::ByteVec(take(d, p, n)?.to_vec()) }
        5  => K::ShortVec(gv!(d, p, i16)),
        6  => K::IntVec(gv!(d, p, i32)),
        7  => K::LongVec(gv!(d, p, i64)),
        8  => K::RealVec(gv!(d, p, f32)),
        9  => K::FloatVec(gv!(d, p, f64)),
        10 => { let n = rd_n(d, p)?; K::CharVec(take(d, p, n)?.to_vec()) }
        11 => { let n = rd_n(d, p)?;                                            // n NUL-terminated syms
                let mut v = Vec::with_capacity(cap_hint(n, d.len() - *p));
                for _ in 0..n { v.push(rd_sym(d, p)?); } K::SymbolVec(v) }
        12 => K::TimestampVec(gv!(d, p, i64)),
        13 => K::MonthVec(gv!(d, p, i32)),
        14 => K::DateVec(gv!(d, p, i32)),
        15 => K::DateTimeVec(gv!(d, p, f64)),
        16 => K::TimespanVec(gv!(d, p, i64)),
        17 => K::MinuteVec(gv!(d, p, i32)),
        18 => K::SecondVec(gv!(d, p, i32)),
        19 => K::TimeVec(gv!(d, p, i32)),

        // ── compound ────────────────────────────────────────────────────────
        98  => { take(d, p, 1)?;                                                // skip attr
                 K::Table(Box::new(read_k(d, p, depth + 1)?)) }
        99  => { let k = read_k(d, p, depth + 1)?;
                 let v = read_k(d, p, depth + 1)?;
                 K::Dict(Box::new(k), Box::new(v)) }
        101 => { let _ = take(d, p, 1); K::Null }                               // identity + attr byte
        _   => return Err(LError::Deserialize(
                   format!("unknown type tag: {t}"))),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rt(k: K) { assert_eq!(k,
        deserialize(&serialize(&k).unwrap()).unwrap()); }

    #[test] fn roundtrip_int()      { rt(K::Int(42)); }
    #[test] fn roundtrip_float()    { rt(K::Float(3.14)); }
    #[test] fn roundtrip_string()   { rt(K::CharVec(b"hello world".to_vec())); }
    #[test] fn roundtrip_symbol()   { rt(K::Symbol("IBM".into())); }
    #[test] fn roundtrip_int_vec()  { rt(K::IntVec(vec![1, 2, 3, 4, 5])); }
    #[test] fn roundtrip_sym_vec()  {
        rt(K::SymbolVec(vec!["IBM".into(), "MSFT".into(), "AAPL".into()])); }
    #[test] fn roundtrip_mixed()    {
        rt(K::List(vec![K::Int(1), K::Float(2.0), K::Symbol("abc".into())])); }
    #[test] fn roundtrip_dict()     {
        rt(K::Dict(Box::new(K::SymbolVec(vec!["a".into(), "b".into()])),
                   Box::new(K::IntVec(vec![10, 20])))); }
    #[test] fn roundtrip_table()    {
        let cols = K::SymbolVec(vec!["sym".into(), "price".into()]);
        let vals = K::List(vec![K::SymbolVec(vec!["IBM".into(), "MSFT".into()]),
                                K::FloatVec(vec![120.5, 340.2])]);
        rt(K::Table(Box::new(K::Dict(Box::new(cols), Box::new(vals))))); }
}
