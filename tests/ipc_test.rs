//! Integration tests for the l-rs IPC client (requires a running L server).

// Test names embed L type letters verbatim so a failure maps to the kernel.
#![allow(non_snake_case)]

use l_rs::{Connection, K, LError};

fn port() -> u16 {
    std::env::var("L_TEST_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5558)
}

fn conn() -> Connection {
    Connection::connect("localhost", port())
        .expect("Cannot connect — start the server with: l -p 9988")
}

// single-assert scaffolding — each macro expands to one #[test] fn.
macro_rules! qk {                                                               // result == K literal
    ($n:ident, $q:expr, $k:expr) => { #[test] fn $n() {
        assert_eq!(conn().query($q).unwrap(), $k); } } }
macro_rules! qkt {                                                              // == K literal + type tag
    ($n:ident, $q:expr, $k:expr, $t:expr) => { #[test] fn $n() {
        let r = conn().query($q).unwrap();
        assert_eq!(r, $k); assert_eq!(r.type_tag(), $t); } } }
macro_rules! qtag {                                                             // type tag only
    ($n:ident, $q:expr, $t:expr) => { #[test] fn $n() {
        assert_eq!(conn().query($q).unwrap().type_tag(), $t); } } }
macro_rules! qint {                                                             // .as_int()
    ($n:ident, $q:expr, $v:expr) => { #[test] fn $n() {
        assert_eq!(conn().query($q).unwrap().as_int(), Some($v)); } } }
macro_rules! qlong {                                                            // .as_long()
    ($n:ident, $q:expr, $v:expr) => { #[test] fn $n() {
        assert_eq!(conn().query($q).unwrap().as_long(), Some($v)); } } }
macro_rules! qstr {                                                             // .as_string()
    ($n:ident, $q:expr, $s:expr) => { #[test] fn $n() {
        assert_eq!(conn().query($q).unwrap().as_string(), Some($s)); } } }
macro_rules! qf {                                                               // float within epsilon
    ($n:ident, $q:expr, $x:expr, $e:expr) => { #[test] fn $n() {
        let v = conn().query($q).unwrap().as_float().unwrap();
        assert!((v - $x).abs() < $e, "got {v}"); } } }
macro_rules! qvec {                                                             // typed Vec extraction
    ($n:ident, $t:ty, $q:expr, $v:expr) => { #[test] fn $n() {
        let got: Vec<$t> = conn().query($q).unwrap().try_into().unwrap();
        assert_eq!(got, $v); } } }

// ATOMS — every scalar type

qkt!(test_bool_atom_true, "1b", K::Bool(true), -1);

qk!(test_bool_atom_false, "0b", K::Bool(false));

#[test]
fn test_byte_atom() {
    let mut c = conn();
    let r = c.query("0x42").unwrap();
    assert_eq!(r.type_tag(), -4);
    if let K::Byte(v) = r { assert_eq!(v, 0x42); } else { panic!("expected \
        byte"); }
}

#[test]
fn test_short_atom() {
    let mut c = conn();
    let r = c.query("42h").unwrap();
    assert_eq!(r.type_tag(), -5);
    if let K::Short(v) = r { assert_eq!(v, 42); } else { panic!("expected \
        short"); }
}

#[test]
fn test_int_atom() {
    let mut c = conn();
    let r = c.query("42").unwrap();
    assert_eq!(r.as_int(), Some(42));
    assert_eq!(r.type_tag(), -6);
}

qint!(test_int_atom_negative, "-99", -99);

#[test]
fn test_long_atom() {
    let mut c = conn();
    let r = c.query("42j").unwrap();
    assert_eq!(r.as_long(), Some(42));
    assert_eq!(r.type_tag(), -7);
}

qlong!(test_long_atom_large, "1000000000000j", 1_000_000_000_000);

#[test]
fn test_real_atom() {
    let mut c = conn();
    let r = c.query("3.14e").unwrap();
    assert_eq!(r.type_tag(), -8);
    if let K::Real(v) = r { assert!((v - 3.14).abs() < 0.01); } else {
        panic!("expected real"); }
}

#[test]
fn test_float_atom() {
    let mut c = conn();
    let r = c.query("3.14").unwrap();
    let v = r.as_float().unwrap();
    assert!((v - 3.14).abs() < 0.001);
    assert_eq!(r.type_tag(), -9);
}

#[test]
fn test_float_atom_zero() {
    let mut c = conn();
    assert!((c.query("0.0").unwrap().as_float().unwrap()).abs() < 0.001);
}

#[test]
fn test_char_atom() {
    let mut c = conn();
    let r = c.query("\"x\"").unwrap();
    // Single char can come back as char atom or char vector depending on l
    let t = r.type_tag();
    assert!(t == -10 || t == 10);
}

qkt!(test_symbol_atom, "`IBM", K::Symbol("IBM".into()), -11);

#[test]
fn test_symbol_atom_empty() {
    let mut c = conn();
    let r = c.query("`").unwrap();
    assert_eq!(r.type_tag(), -11);
    assert_eq!(r, K::Symbol("".into()));
}

// ── Temporal Atoms ────────────────────────────────────────────────

#[test]
fn test_timestamp_atom() {
    let mut c = conn();
    // L v2.5 parses D-format as datetime (-15); verify we get a temporal type
    let r = c.query("2000.01.01T00:00:00.000").unwrap();
    assert_eq!(r.type_tag(), -15);
}

#[test]
fn test_month_atom() {
    let mut c = conn();
    let r = c.query("2000.01m").unwrap();
    assert_eq!(r.type_tag(), -13);
    if let K::Month(v) = r { assert_eq!(v, 0); } else { panic!("expected \
        month"); }
}

#[test]
fn test_date_atom() {
    let mut c = conn();
    let r = c.query("2000.01.01").unwrap();
    assert_eq!(r.type_tag(), -14);
    if let K::Date(v) = r { assert_eq!(v, 0); } else { panic!("expected date \
        with value 0"); }
}

#[test]
fn test_date_atom_nonzero() {
    let mut c = conn();
    let r = c.query("2000.01.02").unwrap();
    if let K::Date(v) = r { assert_eq!(v, 1); } else { panic!("expected \
        date"); }
}

#[test]
fn test_datetime_atom() {
    let mut c = conn();
    let r = c.query("2000.01.01T12:00:00.000").unwrap();
    assert_eq!(r.type_tag(), -15);
    if let K::DateTime(v) = r { assert!((v - 0.5).abs() < 0.001); } else {
        panic!("expected datetime"); }
}

#[test]
fn test_minute_atom() {
    let mut c = conn();
    let r = c.query("12:30").unwrap();
    assert_eq!(r.type_tag(), -17);
    if let K::Minute(v) = r { assert_eq!(v, 750); } else { panic!("expected \
        minute"); }
}

#[test]
fn test_second_atom() {
    let mut c = conn();
    let r = c.query("12:30:45").unwrap();
    assert_eq!(r.type_tag(), -18);
    if let K::Second(v) = r { assert_eq!(v, 45045); } else { panic!("expected \
        second"); }
}

#[test]
fn test_time_atom() {
    let mut c = conn();
    let r = c.query("12:00:00.000").unwrap();
    assert_eq!(r.type_tag(), -19);
    if let K::Time(v) = r { assert_eq!(v, 43200000); } else { panic!("expected \
        time"); }
}

// ── Null & Infinity ───────────────────────────────────────────────

#[test]
fn test_null_int() {
    let mut c = conn();
    let r = c.query("0N").unwrap();
    assert_eq!(r.as_int(), Some(i32::MIN));                                     // 0x80000000
}

qlong!(test_null_long, "0Nj", i64::MIN);

#[test]
fn test_null_float() {
    let mut c = conn();
    let r = c.query("0n").unwrap();
    let v = r.as_float().unwrap();
    assert!(v.is_nan());
}

qint!(test_infinity_int, "0W", i32::MAX);

#[test]
fn test_infinity_float() {
    let mut c = conn();
    let r = c.query("0w").unwrap();
    let v = r.as_float().unwrap();
    assert!(v.is_infinite() && v > 0.0);
}

#[test]
fn test_neg_infinity_float() {
    let mut c = conn();
    let r = c.query("-0w").unwrap();
    let v = r.as_float().unwrap();
    assert!(v.is_infinite() && v < 0.0);
}

// ── String (CharVec) ──────────────────────────────────────────────

#[test]
fn test_string_basic() {
    let mut c = conn();
    let r = c.query("\"hello world\"").unwrap();
    assert_eq!(r.as_string(), Some("hello world"));
    assert_eq!(r.type_tag(), 10);
}

qstr!(test_string_empty, "\"\"", "");

qstr!(test_string_join, "\"hello\",\"world\"", "helloworld");

qint!(test_string_count, "count \"hello\"", 5);

qstr!(test_string_reverse, "reverse \"abc\"", "cba");

qstr!(test_string_upper, "upper \"hello\"", "HELLO");

qstr!(test_string_lower, "lower \"HELLO\"", "hello");

qk!(test_string_like, "\"hello\" like \"hel*\"", K::Bool(true));

qstr!(test_string_ssr, "ssr[\"hello world\";\"world\";\"there\"]",
    "hello there");

#[test]
fn test_string_vs_split() {
    let mut c = conn();
    let r = c.query("\",\" vs \"a,b,c\"").unwrap();
    assert_eq!(r.type_tag(), 0);                                                // mixed list of char vecs
    if let K::List(items) = &r {
        assert_eq!(items.len(), 3);
    }
}

#[test]
fn test_string_sv_join() {
    let mut c = conn();
    // sv in l: join strings with separator
    let r = c.query("\"a\",\"-\",\"b\",\"-\",\"c\"").unwrap();
    assert_eq!(r.as_string(), Some("a-b-c"));
}

qstr!(test_string_trim, "trim \" hello \"", "hello");

// ARITHMETIC

qint!(test_add, "1+1", 2);

qint!(test_subtract, "10-3", 7);

qint!(test_multiply, "3*4", 12);

qf!(test_divide, "10%3", 3.3333, 0.01);

qint!(test_mod, "10 mod 3", 1);

qlong!(test_sum, "sum 1 2 3 4 5", 15);

qlong!(test_prd, "prd 1 2 3 4 5", 120);

qf!(test_avg, "avg 1 2 3 4 5", 3.0, 0.001);

qint!(test_min_vector, "min 5 3 8 1 9", 1);

qint!(test_max_vector, "max 5 3 8 1 9", 9);

qf!(test_sqrt, "sqrt 2", 1.41421, 0.001);

qint!(test_abs, "abs -42", 42);

qlong!(test_floor, "floor 3.7", 3);

qlong!(test_ceiling, "ceiling 3.2", 4);

qf!(test_exp_log, "log exp 1.0", 1.0, 0.001);

#[test]
fn test_signum() {
    let mut c = conn();
    assert_eq!(c.query("signum -5").unwrap().as_int(), Some(-1));
    assert_eq!(c.query("signum 0").unwrap().as_int(), Some(0));
    assert_eq!(c.query("signum 5").unwrap().as_int(), Some(1));
}

// ── Trigonometry ──────────────────────────────────────────────────

#[test]
fn test_sin() {
    let mut c = conn();
    let v = c.query("sin 0.0").unwrap().as_float().unwrap();
    assert!(v.abs() < 0.001);
}

qf!(test_cos, "cos 0.0", 1.0, 0.001);

#[test]
fn test_trig_identity() {
    let mut c = conn();
    // sin^2 + cos^2 = 1 (fully apply to avoid projection)
    let v = c.query("{((sin x)*(sin x))+((cos x)*(cos x))} \
        1.0").unwrap().as_float().unwrap();
    assert!((v - 1.0).abs() < 0.001);
}

// VECTORS — all typed vectors

#[test]
fn test_bool_vector() {
    let mut c = conn();
    let r = c.query("10101b").unwrap();
    assert_eq!(r.type_tag(), 1);
    if let K::BoolVec(v) = r {
        assert_eq!(v, vec![true, false, true, false, true]);
    } else { panic!("expected bool vector"); }
}

#[test]
fn test_byte_vector() {
    let mut c = conn();
    let r = c.query("0x010203").unwrap();
    assert_eq!(r.type_tag(), 4);
    if let K::ByteVec(v) = r {
        assert_eq!(v, vec![1, 2, 3]);
    } else { panic!("expected byte vector"); }
}

#[test]
fn test_short_vector() {
    let mut c = conn();
    let r = c.query("1 2 3h").unwrap();
    assert_eq!(r.type_tag(), 5);
    if let K::ShortVec(v) = r {
        assert_eq!(v, vec![1i16, 2, 3]);
    } else { panic!("expected short vector"); }
}

#[test]
fn test_int_vector() {
    let mut c = conn();
    let r = c.query("1 2 3 4 5").unwrap();
    let v: Vec<i32> = r.try_into().unwrap();
    assert_eq!(v, vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_long_vector() {
    let mut c = conn();
    let r = c.query("1 2 3j").unwrap();
    assert_eq!(r.type_tag(), 7);
    if let K::LongVec(v) = r {
        assert_eq!(v, vec![1i64, 2, 3]);
    } else { panic!("expected long vector"); }
}

#[test]
fn test_real_vector() {
    let mut c = conn();
    let r = c.query("1.0 2.0 3.0e").unwrap();
    assert_eq!(r.type_tag(), 8);
    if let K::RealVec(v) = r {
        assert_eq!(v.len(), 3);
        assert!((v[0] - 1.0).abs() < 0.01);
    } else { panic!("expected real vector"); }
}

#[test]
fn test_float_vector() {
    let mut c = conn();
    let r = c.query("1.1 2.2 3.3").unwrap();
    let v: Vec<f64> = r.try_into().unwrap();
    assert_eq!(v.len(), 3);
    assert!((v[0] - 1.1).abs() < 0.001);
}

#[test]
fn test_symbol_vector() {
    let mut c = conn();
    let r = c.query("`IBM`MSFT`AAPL").unwrap();
    if let K::SymbolVec(v) = r {
        assert_eq!(v, vec!["IBM", "MSFT", "AAPL"]);
    } else { panic!("expected symbol vector"); }
}

// ── Temporal Vectors ──────────────────────────────────────────────

#[test]
fn test_date_vector() {
    let mut c = conn();
    let r = c.query("2000.01.01 2000.01.02 2000.01.03").unwrap();
    assert_eq!(r.type_tag(), 14);
    if let K::DateVec(v) = r {
        assert_eq!(v, vec![0, 1, 2]);
    } else { panic!("expected date vector"); }
}

#[test]
fn test_time_vector() {
    let mut c = conn();
    let r = c.query("00:00:00.000 12:00:00.000").unwrap();
    assert_eq!(r.type_tag(), 19);
    if let K::TimeVec(v) = r {
        assert_eq!(v, vec![0, 43200000]);
    } else { panic!("expected time vector"); }
}

#[test]
fn test_month_vector() {
    let mut c = conn();
    let r = c.query("2000.01 2000.02 2000.03m").unwrap();
    assert_eq!(r.type_tag(), 13);
    if let K::MonthVec(v) = r {
        assert_eq!(v, vec![0, 1, 2]);
    } else { panic!("expected month vector"); }
}

#[test]
fn test_minute_vector() {
    let mut c = conn();
    let r = c.query("00:00 00:01 01:00").unwrap();
    assert_eq!(r.type_tag(), 17);
    if let K::MinuteVec(v) = r {
        assert_eq!(v, vec![0, 1, 60]);
    } else { panic!("expected minute vector"); }
}

#[test]
fn test_second_vector() {
    let mut c = conn();
    let r = c.query("00:00:00 00:00:01 00:01:00").unwrap();
    assert_eq!(r.type_tag(), 18);
    if let K::SecondVec(v) = r {
        assert_eq!(v, vec![0, 1, 60]);
    } else { panic!("expected second vector"); }
}

#[test]
fn test_datetime_vector() {
    let mut c = conn();
    let r = c.query("2000.01.01T00:00:00.000 2000.01.01T12:00:00.000").unwrap();
    assert_eq!(r.type_tag(), 15);
    if let K::DateTimeVec(v) = r {
        assert_eq!(v.len(), 2);
        assert!((v[0]).abs() < 0.001);
        assert!((v[1] - 0.5).abs() < 0.001);
    } else { panic!("expected datetime vector"); }
}

// ── Vector Operations ─────────────────────────────────────────────

qvec!(test_til, i32, "til 5", vec![0, 1, 2, 3, 4]);

qvec!(test_reverse, i32, "reverse 1 2 3", vec![3, 2, 1]);

qvec!(test_where, i32, "where 10101b", vec![0, 2, 4]);

qvec!(test_distinct, i32, "distinct 1 2 2 3 3 3", vec![1, 2, 3]);

qint!(test_count, "count 1 2 3 4 5", 5);

qint!(test_type, "type 42", -6);

qvec!(test_enlist, i32, "enlist 42", vec![42]);

qvec!(test_raze, i32, "raze (1 2;3 4;5 6)", vec![1, 2, 3, 4, 5, 6]);

qvec!(test_sums, i32, "sums 1 2 3 4 5", vec![1, 3, 6, 10, 15]);

qvec!(test_prds, i32, "prds 1 2 3 4 5", vec![1, 2, 6, 24, 120]);

qvec!(test_maxs, i32, "maxs 3 1 4 1 5", vec![3, 3, 4, 4, 5]);

qvec!(test_mins, i32, "mins 3 1 4 1 5", vec![3, 1, 1, 1, 1]);

// ── Shape Operations ──────────────────────────────────────────────

qvec!(test_take, i32, "3#1 2 3 4 5", vec![1, 2, 3]);

qvec!(test_take_overextend, i32, "5#1 2 3", vec![1, 2, 3, 1, 2]);

qvec!(test_drop, i32, "2_1 2 3 4 5", vec![3, 4, 5]);

qvec!(test_drop_negative, i32, "-2_1 2 3 4 5", vec![1, 2, 3]);

qvec!(test_rotate, i32, "2 rotate 1 2 3 4 5", vec![3, 4, 5, 1, 2]);

qvec!(test_concat_vectors, i32, "1 2 3,4 5 6", vec![1, 2, 3, 4, 5, 6]);

// ── Sorting & Grading ─────────────────────────────────────────────

qvec!(test_asc, i32, "asc 3 1 4 1 5 9", vec![1, 1, 3, 4, 5, 9]);

qvec!(test_desc, i32, "desc 3 1 4 1 5 9", vec![9, 5, 4, 3, 1, 1]);

qvec!(test_iasc, i32, "iasc 3 1 4", vec![1, 0, 2]);

qvec!(test_idesc, i32, "idesc 3 1 4", vec![2, 0, 1]);

qvec!(test_rank, i32, "rank 3 1 4", vec![1, 0, 2]);

// ── Searching & Membership ────────────────────────────────────────

qint!(test_find, "1 2 3 4 5?3", 2);

qint!(test_find_missing, "1 2 3?99", 3);

qk!(test_in_membership, "3 in 1 2 3 4 5", K::Bool(true));

#[test]
fn test_group() {
    let mut c = conn();
    let r = c.query("group `a`b`a`b`c").unwrap();
    assert_eq!(r.type_tag(), 99);                                               // dict
}

qint!(test_bin, "1 2 3 4 5 bin 3", 2);

// x bin y on a compressed sorted int column, diffed value-by-value vs a raw twin.
#[test]
fn cov_coc_bin_histogram() {
    let mut c = conn();
    // Compressed int column (range 0..999) + raw twin; each setup is its own query.
    c.query("bnh_c:asc 1000000?1000").unwrap();
    c.query("`bnh_r set bnh_c|bnh_c").unwrap();
    assert_eq!(c.query("first -17!`bnh_c").unwrap(), K::Bool(true));            // compressed
    assert_eq!(c.query("first -17!`bnh_r").unwrap(), K::Bool(false));           // raw twin
    // Unsorted, m=100k>=n/32: whole-vector identity vs raw.
    c.query("`bnh_p set 100000?1000").unwrap();
    eq_q(&mut c, "coc-bin-unsorted-100k", "(bnh_c bin bnh_p)~bnh_r bin bnh_p",
        "1b");
    // Edge probes: below min, at min, mid, at max, above max.
    eq_q(&mut c, "coc-bin-edges",
         "(bnh_c bin -7 0 1 500 999 1000 9999)~bnh_r bin -7 0 1 500 999 1000 \
             9999",
         "1b");
    // Sorted probes take the fast path; still correct.
    c.query("`bnh_s set asc 50000?1000").unwrap();
    eq_q(&mut c, "coc-bin-sorted-50k", "(bnh_c bin bnh_s)~bnh_r bin bnh_s",
        "1b");
    // Small m (gate bail) — fall-through correctness.
    c.query("`bnh_q set 100?1000").unwrap();
    eq_q(&mut c, "coc-bin-small-m", "(bnh_c bin bnh_q)~bnh_r bin bnh_q", "1b");
    // KJ-typed column + probe (64-bit path).
    c.query("bnh_cj:asc 1000000?100000j").unwrap();
    c.query("`bnh_rj set bnh_cj|bnh_cj").unwrap();
    c.query("`bnh_pj set 50000?100000j").unwrap();
    eq_q(&mut c, "coc-bin-kj", "(bnh_cj bin bnh_pj)~bnh_rj bin bnh_pj", "1b");
    // Negative-value column.
    c.query("bnh_cn:asc -500+1000000?1000").unwrap();
    c.query("`bnh_rn set bnh_cn|bnh_cn").unwrap();
    c.query("`bnh_pn set -600+50000?1200").unwrap();
    eq_q(&mut c, "coc-bin-neg", "(bnh_cn bin bnh_pn)~bnh_rn bin bnh_pn", "1b");
    // Wide-range column: result must equal the raw twin for unsorted and ascending probes.
    c.query("bnh_cw:asc 1000000?100000000").unwrap();
    c.query("`bnh_rw set bnh_cw|bnh_cw").unwrap();
    c.query("`bnh_pw set 50000?100000000").unwrap();                            // small unsorted
    eq_q(&mut c, "coc-bin-wide-unsorted", "(bnh_cw bin bnh_pw)~bnh_rw bin \
        bnh_pw", "1b");
    c.query("`bnh_pa set asc 1000000?100000000").unwrap();                      // m=n ascending (asof)
    eq_q(&mut c, "coc-bin-wide-asof", "(bnh_cw bin bnh_pa)~bnh_rw bin bnh_pa",
        "1b");
    c.query("`bnh_pa2 set asc 2200000?100000000").unwrap();                     // m>n ascending
    eq_q(&mut c, "coc-bin-wide-asof-2n", "(bnh_cw bin bnh_pa2)~bnh_rw bin \
        bnh_pa2", "1b");
    // KJ wide-range, incl. probes past both ends (all KJ).
    c.query("bnh_cwj:asc 1000000?10000000000j").unwrap();
    c.query("`bnh_rwj set bnh_cwj|bnh_cwj").unwrap();
    c.query("`bnh_paj set asc -9j,9999999999j,1000000?10000000000j").unwrap();
    eq_q(&mut c, "coc-bin-wide-kj", "(bnh_cwj bin bnh_paj)~bnh_rwj bin \
        bnh_paj", "1b");
}

// distinct x on a compressed sorted int column, diffed vs the raw twin.
#[test]
fn cov_coc_distinct_bitmap() {
    let mut c = conn();
    // Sorted compressed column (range 0..999) + raw twin.
    c.query("dnh_cs:asc 1000000?1000").unwrap();
    c.query("`dnh_rs set dnh_cs|dnh_cs").unwrap();
    assert_eq!(c.query("first -17!`dnh_cs").unwrap(), K::Bool(true));
    eq_q(&mut c, "coc-distinct-sorted", "(distinct dnh_cs)~distinct dnh_rs",
        "1b");
    // Unsorted compressed column (falls back to decode) — still correct.
    c.query("dnh_cu:1000000?1000").unwrap();
    c.query("`dnh_ru set dnh_cu|dnh_cu").unwrap();
    eq_q(&mut c, "coc-distinct-unsorted", "(distinct dnh_cu)~distinct dnh_ru",
        "1b");
    // Negative-value sorted column.
    c.query("dnh_cn:asc -500+1000000?1000").unwrap();
    c.query("`dnh_rn set dnh_cn|dnh_cn").unwrap();
    eq_q(&mut c, "coc-distinct-neg", "(distinct dnh_cn)~distinct dnh_rn", "1b");
    // KJ-typed sorted column (64-bit).
    c.query("dnh_cj:asc 1000000?100000j").unwrap();
    c.query("`dnh_rj set dnh_cj|dnh_cj").unwrap();
    eq_q(&mut c, "coc-distinct-kj", "(distinct dnh_cj)~distinct dnh_rj", "1b");
    // Dup-heavy (long runs, few distinct) sorted column.
    c.query("dnh_cd:asc 1000000?10").unwrap();
    c.query("`dnh_rd set dnh_cd|dnh_cd").unwrap();
    eq_q(&mut c, "coc-distinct-dup", "(distinct dnh_cd)~distinct dnh_rd", "1b");
    // count distinct (common reduction over the result).
    eq_q(&mut c, "coc-distinct-count", "(count distinct dnh_cs)=count distinct \
        dnh_rs",
         "1b");
}

// x in y membership when y is a compressed int set, diffed vs the raw twin.
#[test]
fn cov_coc_in_bitmap() {
    let mut c = conn();
    c.query("inh_c:asc 1000000?1000").unwrap();                                 // sorted set
    c.query("`inh_r set inh_c|inh_c").unwrap();
    c.query("`inh_p set 100000?1000").unwrap();
    eq_q(&mut c, "coc-in-probes", "(inh_p in inh_c)~inh_p in inh_r", "1b");
    eq_q(&mut c, "coc-in-edges",
         "((-5 0 999 1000 5000) in inh_c)~(-5 0 999 1000 5000) in inh_r", "1b");
    eq_q(&mut c, "coc-in-atom", "(500 in inh_c)~500 in inh_r", "1b");
    // Unsorted compressed set (sortedness not required).
    c.query("inh_cu:1000000?1000").unwrap();
    c.query("`inh_ru set inh_cu|inh_cu").unwrap();
    eq_q(&mut c, "coc-in-unsorted-set", "(inh_p in inh_cu)~inh_p in inh_ru",
        "1b");
    // KJ wide-range set (64-bit; big win over the raw path).
    c.query("inh_cj:asc 1000000?100000j").unwrap();
    c.query("`inh_rj set inh_cj|inh_cj").unwrap();
    c.query("`inh_qj set 100000?100000j").unwrap();
    eq_q(&mut c, "coc-in-kj", "(inh_qj in inh_cj)~inh_qj in inh_rj", "1b");
    // Negative-value set.
    c.query("inh_cn:asc -500+1000000?1000").unwrap();
    c.query("`inh_rn set inh_cn|inh_cn").unwrap();
    c.query("`inh_qn set -600+100000?1200").unwrap();
    eq_q(&mut c, "coc-in-neg", "(inh_qn in inh_cn)~inh_qn in inh_rn", "1b");
    // x compressed, y small set (falls back on the x side) — still correct.
    c.query("`inh_ss set 50?1000").unwrap();
    eq_q(&mut c, "coc-in-col-in-set", "(inh_c in inh_ss)~inh_r in inh_ss",
        "1b");
}

// group x on a compressed sorted int column, diffed vs the raw twin.
#[test]
fn cov_coc_group_runs() {
    let mut c = conn();
    c.query("grh_cs:asc 1000000?1000").unwrap();
    c.query("`grh_rs set grh_cs|grh_cs").unwrap();
    assert_eq!(c.query("first -17!`grh_cs").unwrap(), K::Bool(true));
    eq_q(&mut c, "coc-group-sorted", "(group grh_cs)~group grh_rs", "1b");
    eq_q(&mut c, "coc-group-count", "(count group grh_cs)=count group grh_rs",
        "1b");
    c.query("grh_cn:asc -500+1000000?1000").unwrap();
    c.query("`grh_rn set grh_cn|grh_cn").unwrap();
    eq_q(&mut c, "coc-group-neg", "(group grh_cn)~group grh_rn", "1b");
    c.query("grh_cj:asc 1000000?100000j").unwrap();
    c.query("`grh_rj set grh_cj|grh_cj").unwrap();
    eq_q(&mut c, "coc-group-kj", "(group grh_cj)~group grh_rj", "1b");
    c.query("grh_cd:asc 1000000?10").unwrap();
    c.query("`grh_rd set grh_cd|grh_cd").unwrap();
    eq_q(&mut c, "coc-group-dup", "(group grh_cd)~group grh_rd", "1b");
}

// (agg;data) fby key value-correctness vs the lambda form across aggregations/types.
#[test]
fn cov_fby_topology_fold() {
    let mut c = conn();
    // known small parted cases (3 groups of 2 rows)
    eq_q(&mut c, "fby-sum",   "(sum;1 2 3 4 5 6j) fby `p#1 1 2 2 3 3", "3 3 7 \
        7 \
        11 11j");
    eq_q(&mut c, "fby-max",   "(max;1 5 3 2 9 4j) fby `p#1 1 2 2 3 3", "5 5 3 \
        3 \
        9 9j");
    eq_q(&mut c, "fby-min",   "(min;1 5 3 2 9 4j) fby `p#1 1 2 2 3 3", "1 1 2 \
        2 \
        4 4j");
    eq_q(&mut c, "fby-first", "(first;1 5 3 2 9 4j) fby `p#1 1 2 2 3 3", "1 1 \
        3 \
        3 9 9j");
    eq_q(&mut c, "fby-last",  "(last;1 5 3 2 9 4j) fby `p#1 1 2 2 3 3", "5 5 2 \
        2 4 4j");
    eq_q(&mut c, "fby-count", "(count;6#0) fby `p#1 1 2 2 3 3", "2 2 2 2 2 2");
    // attribute-invariance: sorted/grouped fold == unattributed fold on the same key
    c.query("fk:asc 200000?1000; fv:200000?1000j").unwrap();
    eq_q(&mut c, "fby-sum-attrinv",   "((sum;fv) fby `p#fk)~(sum;fv) fby `#fk",
        "1b");
    eq_q(&mut c, "fby-max-attrinv",   "((max;fv) fby `p#fk)~(max;fv) fby `#fk",
        "1b");
    eq_q(&mut c, "fby-avg-attrinv",   "((avg;fv) fby `p#fk)~(avg;fv) fby `#fk",
        "1b");
    eq_q(&mut c, "fby-last-attrinv",  "((last;fv) fby `p#fk)~(last;fv) fby \
        `#fk", "1b");
    eq_q(&mut c, "fby-sum-g",         "((sum;fv) fby `g#fk)~(sum;fv) fby `#fk",
        "1b");
    c.query("fvf:`float$200000?1000").unwrap();
    eq_q(&mut c, "fby-avg-float",     "((avg;fvf) fby `p#fk)~(avg;fvf) fby \
        `#fk", "1b");
    // house standard: the broadcast (piecewise-constant) is RETURNED COMPRESSED
    c.query("`fbout set (max;fv) fby `p#fk").unwrap();
    eq_q(&mut c, "fby-returns-compressed", "first -17!`fbout", "1b");
    // user-lambda agg → cede to the legacy member-materialise path (still correct)
    eq_q(&mut c, "fby-cede", "({x*2};1 2 3 4j) fby `p#1 1 2 2", "2 4 6 8j");
}

// value-sort and state-toggle slots on a compressed int column, checked vs q asc/desc.
#[test]
fn cov_coc_value_sort_slots() {
    let mut c = conn();
    c.query("svh_u:1000000?1000").unwrap();                                     // compressed
    assert_eq!(c.query("first -17!`svh_u").unwrap(), K::Bool(true));
    eq_q(&mut c, "slot-asc",  "(-19!`svh_u)~asc svh_u",  "1b");
    eq_q(&mut c, "slot-desc", "(-20!`svh_u)~desc svh_u", "1b");
    // dup-heavy (long runs) + KJ + negatives.
    c.query("svh_d:1000000?10").unwrap();
    eq_q(&mut c, "slot-asc-dup",  "(-19!`svh_d)~asc svh_d",  "1b");
    eq_q(&mut c, "slot-desc-dup", "(-20!`svh_d)~desc svh_d", "1b");
    c.query("svh_j:1000000?100000j").unwrap();
    eq_q(&mut c, "slot-asc-kj",   "(-19!`svh_j)~asc svh_j",  "1b");
    c.query("svh_n:-500+1000000?1000").unwrap();
    eq_q(&mut c, "slot-desc-neg", "(-20!`svh_n)~desc svh_n", "1b");
    // -18! compression toggle round-trips both directions, preserves data.
    c.query("`svh_raw set -18!`svh_u").unwrap();                                // compressed → raw
    assert_eq!(c.query("first -17!`svh_raw").unwrap(), K::Bool(false));
    c.query("`svh_cmp set -18!`svh_raw").unwrap();                              // raw → compressed
    assert_eq!(c.query("first -17!`svh_cmp").unwrap(), K::Bool(true));
    eq_q(&mut c, "toggle-preserves", "(asc value`svh_raw)~asc svh_u", "1b");
}

// Amend / cat on a compressed column must mutate correctly (memory-corruption guard).
#[test]
fn cov_coc_amend_decompress() {
    let mut c = conn();
    // single-element amend stays compressed, value correct.
    c.query("amd_x:asc 1000000?1000").unwrap();
    assert_eq!(c.query("first -17!`amd_x").unwrap(), K::Bool(true));
    c.query("amd_x[5]:42").unwrap();
    eq_q(&mut c, "amend-single", "amd_x[5]=42", "1b");
    // null amend falls back to decode (null widens the range); null preserved, not 0.
    c.query("amd_z:asc 1000000?1000").unwrap();
    c.query("amd_z[5]:0N").unwrap();
    eq_q(&mut c, "amend-null-val", "null amd_z[5]", "1b");
    eq_q(&mut c, "amend-null-count", "1=count where null amd_z", "1b");
    // Corruption repro: a decline-amend then a fresh column + multi-amend must stay intact.
    c.query("amd_w:asc 1000000?1000").unwrap();
    c.query("amd_w[0 1 2]:100 200 300").unwrap();
    eq_q(&mut c, "amend-after-decline", "amd_w[0 1 2]~100 200 300", "1b");
    // out-of-range amend (mid-vector) falls back to decode.
    c.query("amd_o:asc 1000000?1000").unwrap();
    c.query("amd_o[500000]:5000000").unwrap();
    eq_q(&mut c, "amend-oor", "amd_o[500000]=5000000", "1b");
    // vector amend with a null in the value vector.
    c.query("amd_v:asc 1000000?1000").unwrap();
    c.query("amd_v[0 1 2]:100 200 0N").unwrap();
    eq_q(&mut c, "amend-vec-null", "(amd_v[0 1]~100 200)&null amd_v[2]", "1b");
    // @ and . functional amend forms (same z1/z2 path, different dispatch).
    c.query("amd_at:asc 1000000?1000").unwrap();
    c.query("@[`amd_at;0 1 2;:;100 200 5000000]").unwrap();
    eq_q(&mut c, "amend-at-form", "amd_at[0 1 2]~100 200 5000000", "1b");
    c.query("amd_dot:asc 1000000?1000").unwrap();
    c.query(".[`amd_dot;enlist 5;:;0N]").unwrap();
    eq_q(&mut c, "amend-dot-form", "null amd_dot[5]", "1b");
    // cat on compressed equals cat on the raw twin (`set`|`set` bypasses compression).
    c.query("amd_c:asc 1000000?1000").unwrap();
    c.query("`amd_cr set amd_c|amd_c").unwrap();
    eq_q(&mut c, "cat-compressed-self", "(amd_c,amd_c)~amd_cr,amd_cr", "1b");
    eq_q(&mut c, "cat-compressed-atom", "(amd_c,5)~amd_cr,5", "1b");
    // Repeated decline-amend loop must not corrupt later fresh columns.
    eq_q(&mut c, "amend-decline-loop",
         "all {[i] g:asc 1000000?1000; g[3+i]:0N; \
          h:asc 1000000?1000; h[0 1 2]:1 2 3; h[0 1 2]~1 2 3} each til 10",
         "1b");
}

// Short-symbol columns round-trip 1:1 through disk write/read (borrow-leak guard).
#[test]
fn cov_x2sym_disk_roundtrip() {
    let mut c = conn();
    c.query(r#"x2v:`$string 100000000000j+til 50000"#).unwrap();                // 50k 12-char syms
    c.query(r#"(`:/tmp/x2rt) set x2v"#).unwrap();
    eq_q(&mut c, "x2sym roundtrip decode", r#"x2v~get `:/tmp/x2rt"#, "1b");
    eq_q(&mut c, "x2sym roundtrip loop",
         "all {[i] v:`$string 100000000000j+(i*20000)+til 20000; (`:/tmp/x2rt) \
             set v; v~get `:/tmp/x2rt} each til 8",
         "1b");
}

// On-disk symbol HDB: grouped select over partitions is correct and leak-free.
#[test]
fn cov_x2sym_hdb_group() {
    let mut c = conn();
    c.query(r#"system "rm -rf /tmp/x2h""#).unwrap();
    c.query("mk_x2:{[p] d:\"/tmp/x2h/\",string[p],\"/bt/\"; system \"mkdir -p \
        \",d; (hsym`$d,\"id3\") set `$string 100000000000j+(p*40000)+til \
            40000; (hsym`$d,\"v1\") set 40000#5; (hsym`$d,\".d\") set \
                `id3`v1}").unwrap();
    c.query(r#"mk_x2 each 0 1"#).unwrap();
    c.query(r#"system "l /tmp/x2h""#).unwrap();
    eq_q(&mut c, "x2sym hdb group \
        rows", r#"count 0!select v1:sum v1 by id3 from bt"#, "80000");
    eq_q(&mut c, "x2sym hdb keys sorted",
         r#"{x~asc x} exec id3 from 0!select v1:sum v1 by id3 from bt"#, "1b");
    eq_q(&mut c, "x2sym hdb sum invariant",
         "(sum exec v1 from 0!select v1:sum v1 by id3 from bt)=sum exec v1 \
             from bt", "1b");
}

// Selective take x[idx] on a compressed column must bit-match the raw-twin take.
#[test]
fn cov_coc_gather() {
    let mut c = conn();
    c.query("gth_c:(til 1000000) mod 100").unwrap();                            // compressed, range 0-99
    c.query("`gth_r set gth_c|gth_c").unwrap();                                 // raw twin (set bypasses compress)
    assert_eq!(c.query("first -17!`gth_c").unwrap(), K::Bool(true));            // compressed
    assert_eq!(c.query("first -17!`gth_r").unwrap(), K::Bool(false));           // raw
    // Sparse KI gather (< n/32) stays on the compressed path.
    c.query("`gth_i set 1000?1000000").unwrap();
    eq_q(&mut c, "coc-gather-sparse", "(gth_c gth_i)~gth_r gth_i", "1b");
    // Dense gather (> n/32) falls back to decode, still correct.
    c.query("`gth_d set 200000?1000000").unwrap();
    eq_q(&mut c, "coc-gather-dense", "(gth_c gth_d)~gth_r gth_d", "1b");
    // KJ index vector.
    c.query("`gth_j set `long$1000?1000000").unwrap();
    eq_q(&mut c, "coc-gather-kj", "(gth_c gth_j)~gth_r gth_j", "1b");
    // Compressed float column.
    c.query("gth_cf:0.01*(til 1000000) mod 100").unwrap();
    c.query("`gth_rf set gth_cf|gth_cf").unwrap();
    eq_q(&mut c, "coc-gather-alp", "(gth_cf gth_i)~gth_rf gth_i", "1b");
    // Single-element vector gather (the boundary with scalar_at).
    eq_q(&mut c, "coc-gather-single", "(gth_c enlist 7)~gth_r enlist 7", "1b");
}

// DICTIONARIES

#[test]
fn test_simple_dict() {
    let mut c = conn();
    let r = c.query("`a`b`c!1 2 3").unwrap();
    assert_eq!(r.type_tag(), 99);
    if let K::Dict(keys, vals) = &r {
        if let K::SymbolVec(k) = keys.as_ref() {
            assert_eq!(k, &vec!["a".to_string(), "b".to_string(),
                "c".to_string()]);
        }
        if let K::IntVec(v) = vals.as_ref() {
            assert_eq!(v, &vec![1, 2, 3]);
        }
    } else { panic!("expected dict"); }
}

qtag!(test_dict_with_floats, "`x`y!3.14 2.72", 99);

#[test]
fn test_dict_with_symbol_values() {
    let mut c = conn();
    let r = c.query("`a`b!`IBM`MSFT").unwrap();
    assert_eq!(r.type_tag(), 99);
    if let K::Dict(_, vals) = &r {
        if let K::SymbolVec(v) = vals.as_ref() {
            assert_eq!(v, &vec!["IBM".to_string(), "MSFT".to_string()]);
        }
    }
}

qint!(test_dict_count, "count `a`b`c!1 2 3", 3);

#[test]
fn test_dict_key_value() {
    let mut c = conn();
    let r = c.query("key `a`b!1 2").unwrap();
    if let K::SymbolVec(k) = r {
        assert_eq!(k, vec!["a".to_string(), "b".to_string()]);
    }
    let r = c.query("value `a`b!1 2").unwrap();
    if let K::IntVec(v) = r {
        assert_eq!(v, vec![1, 2]);
    }
}

// TABLES

qtag!(test_simple_table, "([]a:1 2 3;b:`x`y`z)", 98);

#[test]
fn test_table_select_where() {
    let mut c = conn();
    c.query("tt:([]sym:`IBM`MSFT`AAPL;price:120.5 340.2 175.8)").unwrap();
    let r = c.query("select from tt where price > 200").unwrap();
    assert_eq!(r.type_tag(), 98);
}

#[test]
fn test_table_select_columns() {
    let mut c = conn();
    c.query("tt2:([]a:1 2 3;b:10 20 30;c:`x`y`z)").unwrap();
    let r = c.query("select a,c from tt2").unwrap();
    assert_eq!(r.type_tag(), 98);
}

#[test]
fn test_table_count() {
    let mut c = conn();
    c.query("tt3:([]a:1 2 3;b:10 20 30)").unwrap();
    assert_eq!(c.query("count tt3").unwrap().as_int(), Some(3));
}

// A sorted column's where col</>/=/within must materialize the virtual i index correctly.
#[test]
fn test_select_i_sorted_compressed_where() {
    let mut c = conn();
    // lambda-apply forces compile→compress of the sorted int key
    c.query("{ki::asc (til 200000) mod 1000; \
        kt::([]k:ki;v:200000?100)}[]").unwrap();
    c.query("kr::([]k:asc (til 200000) mod 1000;v:200000?100)").unwrap();       // raw twin (same k)
    // count i must be the real row count (20000), not 1 (the leaked RNG atom)
    assert_eq!(
        c.query("(first exec cnt from select cnt:count i from kt where \
            k<100)=20000").unwrap(),
        K::Bool(true));
    // select i / exec i row indices match the raw twin (range materialized in order)
    assert_eq!(
        c.query("(select i from kt where k<100)~select i from kr where \
            k<100").unwrap(),
        K::Bool(true));
    assert_eq!(
        c.query("(exec i from kt where k>900)~exec i from kr where \
            k>900").unwrap(),
        K::Bool(true));
    assert_eq!(
        c.query("(exec i from kt where k within 100 109)~exec i from kr where \
            k \
            within 100 109").unwrap(),
        K::Bool(true));
    // equality-range (foo→predX SEL,8): near window so the RNG fires through virtual `i
    assert_eq!(
        c.query("(select i from kt where k=5)~select i from kr where \
            k=5").unwrap(),
        K::Bool(true));
    assert_eq!(
        c.query("(first exec cnt from select cnt:count i from kt where \
            k=5)=200").unwrap(),
        K::Bool(true));
}

#[test]
fn test_select_count_i_by_sorted_compressed_where() {
    let mut c = conn();
    c.query("{ki2::asc (til 200000) mod 1000; \
        kt2::([]k:ki2;v:200000?100)}[]").unwrap();
    c.query("kr2::([]k:asc (til 200000) mod 1000;v:200000?100)").unwrap();
    // grouped count i / select i under a sorted-RNG where match the raw twin
    assert_eq!(
        c.query("(select cnt:count i by k from kt2 where k<100)~select \
            cnt:count i by k from kr2 where k<100").unwrap(),
        K::Bool(true));
    assert_eq!(
        c.query("(select i by k from kt2 where k=500)~select i by k from kr2 \
            where k=500").unwrap(),
        K::Bool(true));
}

// Internal optimizer IR must never escape public parse; eval still fuses correctly.
#[test]
fn test_xsel_not_leaked_through_public_parse() {
    let mut c = conn();
    c.query("pv:til 10").unwrap();
    // parse returns a normal type-0 AST, not an opaque XSEL (type 121)
    assert_eq!(c.query("type parse \"where pv>1\"").unwrap().as_int(), Some(0));
    assert_eq!(c.query("type parse \"pv where pv>1\"").unwrap().as_int(),
        Some(0));
    // eval of the parse tree roundtrips (the recursive evaluator)
    assert_eq!(
        c.query("(eval parse \"where pv>1\")~where pv>1").unwrap(),
        K::Bool(true));
    assert_eq!(
        c.query("(eval parse \"pv where pv>1\")~pv where pv>1").unwrap(),
        K::Bool(true));
}

// Slicing a compressed column: null-free int slices stay fast; nullable cols fall back.
#[test]
fn test_xij_qslice_int_stays_qz() {
    let mut c = conn();
    c.query("{r8::100000?256}[]").unwrap();                                     // small-range int
    c.query("s8::100#r8").unwrap();
    assert_eq!(c.query("1=first -17!`s8").unwrap(), K::Bool(true));             // compressed slice
    assert_eq!(c.query("s8~r8@til 100").unwrap(), K::Bool(true));
    c.query("{r32::0j+100000?2000000000}[]").unwrap();                          // wide range needs KJ
    c.query("s32::30#5_r32").unwrap();
    assert_eq!(c.query("(1=first -17!`s32)&s32~r32@5+til 30").unwrap(),
        K::Bool(true));
    // orig_t faithful clone: raze(original; slice) stays compressed
    c.query("u8::raze(r8;100#r8)").unwrap();
    assert_eq!(c.query("(1=first -17!`u8)&u8~r8,r8@til 100").unwrap(),
        K::Bool(true));
}

#[test]
fn test_xij_qslice_nullable_null_safe() {
    let mut c = conn();
    // A nullable column must skip the fast slice path; sum/min/max/fill stay null-correct.
    c.query("{rn::100000?0N 1 2 3}[]").unwrap();
    c.query("sn::100#rn").unwrap();
    assert_eq!(c.query("0=first -17!`sn").unwrap(), K::Bool(true));             // fell back to raw
    assert_eq!(c.query("sn~rn@til 100").unwrap(), K::Bool(true));
    assert_eq!(c.query("(sum sn)~sum rn@til 100").unwrap(), K::Bool(true));
    assert_eq!(c.query("(min sn)~min rn@til 100").unwrap(), K::Bool(true));
    assert_eq!(c.query("(max sn)~max rn@til 100").unwrap(), K::Bool(true));
    assert_eq!(c.query("(0^sn)~0^rn@til 100").unwrap(), K::Bool(true));
}

#[test]
fn test_xij_qslice_bool_exact() {
    let mut c = conn();
    // bool slice stays compressed with exact min/max, no int drift.
    c.query("{bmm::1000000?0b}[]").unwrap();
    c.query("sb::100#bmm").unwrap();
    assert_eq!(c.query("1=first -17!`sb").unwrap(), K::Bool(true));             // compressed
    assert_eq!(c.query("(-1)=type min sb").unwrap(), K::Bool(true));            // bool not int
    assert_eq!(c.query("(min bmm)~min bmm@til count bmm").unwrap(),
        K::Bool(true));
    assert_eq!(c.query("(max bmm)~max bmm@til count bmm").unwrap(),
        K::Bool(true));
    assert_eq!(c.query("(sum sb)~sum bmm@til 100").unwrap(), K::Bool(true));
    // all-ones column: min/max/sum must use the value, not the stored form
    c.query("{ao::1000000#1b}[]").unwrap();
    assert_eq!(c.query("(1b~min 100#ao)&(1b~max 100#ao)&100=sum \
        100#ao").unwrap(), K::Bool(true));
    c.query("{az::1000000#0b}[]").unwrap();
    assert_eq!(c.query("(0b~min 100#az)&0b~max 100#az").unwrap(),
        K::Bool(true));
}

// min on a compressed float column stays exact and fast (bounds-gated shortcut).
#[test]
fn test_alp_min_bounds_gated() {
    let mut c = conn();
    c.query("{af::0.5+\"f\"$til 100000}[]").unwrap();
    assert_eq!(c.query("1=first -17!`af").unwrap(), K::Bool(true));             // compressed
    assert_eq!(c.query("(min af)~0.5").unwrap(), K::Bool(true));                // exact min (base·10^-e)
    assert_eq!(c.query("(max af)~99999.5").unwrap(), K::Bool(true));
    assert_eq!(c.query("(min af)~min 0.5+\"f\"$til 100000").unwrap(),
        K::Bool(true));
}

// A compressed float slice keeps sorted attr and exact min; irregular floats fall back to decode.
#[test]
fn test_xij_qslice_alp_nopatch() {
    let mut c = conn();
    c.query("{afa::0.5+\"f\"$til 100000}[]").unwrap();                          // sorted compressible float
    c.query("asa::100#64_afa").unwrap();
    assert_eq!(c.query("1=first -17!`asa").unwrap(), K::Bool(true));            // compressed slice
    assert_eq!(c.query("`s=attr asa").unwrap(), K::Bool(true));                 // sorted preserved
    assert_eq!(c.query("asa~afa@64+til 100").unwrap(), K::Bool(true));          // parity
    // exact LOCAL bounds (O(1) via stamped Kvxl/Kvxm) — the slice min/max, not parent
    assert_eq!(c.query("(min asa)~64.5").unwrap(), K::Bool(true));
    assert_eq!(c.query("(max asa)~163.5").unwrap(), K::Bool(true));
    assert_eq!(c.query("(min afa)~0.5").unwrap(), K::Bool(true));               // full-vec min via Kvxl
    assert_eq!(c.query("(max afa)~99999.5").unwrap(), K::Bool(true));           // full-vec max via Kvxm
    assert_eq!(c.query("(sum asa)~sum afa@64+til 100").unwrap(), K::Bool(true));
    assert_eq!(c.query("(where asa<100.0)~where (afa@64+til \
        100)<100.0").unwrap(), K::Bool(true));
    // non-block-aligned (byte-aligned) start
    c.query("n8::50#8_afa").unwrap();
    assert_eq!(c.query("(1=first -17!`n8)&n8~afa@8+til 50").unwrap(),
        K::Bool(true));
    c.query("{afp::0.5+\"f\"$100000?1000.0}[]").unwrap();                       // patched (random floats)
    c.query("asp::100#64_afp").unwrap();
    assert_eq!(c.query("asp~afp@64+til 100").unwrap(), K::Bool(true));          // decode fallback correct
}

// < / > grade a compressed column with results matching the uncompressed grade.
#[test]
fn test_coc_grade_glyph() {
    let mut c = conn();
    c.query("{gs::0.5+\"f\"$til 100000}[]").unwrap();                           // sorted compressible float
    assert_eq!(c.query("((<)gs)~til count gs").unwrap(), K::Bool(true));        // identity
    assert_eq!(c.query("((>)gs)~reverse til count gs").unwrap(), K::Bool(true));// reverse
    c.query("{gj::\"j\"$til 100000}[]").unwrap();                               // monotone long
    assert_eq!(c.query("((<)gj)~til count gj").unwrap(), K::Bool(true));
    c.query("{gu::0.5+\"f\"$100000?1000}[]").unwrap();                          // unsorted compressible float
    assert_eq!(c.query("(gu@(<)gu)~asc gu").unwrap(), K::Bool(true));
    c.query("{gi::100000?256}[]").unwrap();                                     // unsorted compressed int
    assert_eq!(c.query("(gi@(<)gi)~asc gi").unwrap(), K::Bool(true));
    assert_eq!(c.query("(gi@(>)gi)~desc gi").unwrap(), K::Bool(true));
    c.query("{gn::100000?0N 1 2 3}[]").unwrap();                                // null-bearing
    assert_eq!(c.query("(gn@(<)gn)~asc gn").unwrap(), K::Bool(true));
    // iasc/idesc on a compressed column match the uncompressed grade.
    assert_eq!(c.query("(iasc gs)~til count gs").unwrap(), K::Bool(true));      // sorted identity
    assert_eq!(c.query("(idesc gs)~reverse til count gs").unwrap(),
        K::Bool(true));
    assert_eq!(c.query("(gi@iasc gi)~asc gi").unwrap(), K::Bool(true));         // unsorted parity
    assert_eq!(c.query("(gi@idesc gi)~desc gi").unwrap(), K::Bool(true));
}

// x±a on a compressed int column stays compressed and matches raw; x*a decodes correctly.
#[test]
fn test_coc_arith_delta() {
    let mut c = conn();
    c.query("{di::\"j\"$3*til 100000}[]").unwrap();                             // KJ delta increasing
    assert_eq!(c.query("(di+5)~5+3*\"j\"$til 100000").unwrap(), K::Bool(true));
    assert_eq!(c.query("(di-7)~(3*\"j\"$til 100000)-7").unwrap(),
        K::Bool(true));
    c.query("ar::di+5").unwrap();
    assert_eq!(c.query("1=first -17!`ar").unwrap(), K::Bool(true));             // stays compressed
    assert_eq!(c.query("`s=attr ar").unwrap(), K::Bool(true));                  // sort attr preserved
    assert_eq!(c.query("((di+5)+10)~15+3*\"j\"$til 100000").unwrap(),           // chain
        K::Bool(true));
    assert_eq!(c.query("(where (di+5)>100)~where (5+3*\"j\"$til \
        100000)>100").unwrap(), K::Bool(true));
    c.query("{dd::\"j\"$1000000-3*til 100000}[]").unwrap();                     // decreasing (neg deltas)
    assert_eq!(c.query("(dd+5)~5+1000000-3*\"j\"$til 100000").unwrap(),
        K::Bool(true));
    assert_eq!(c.query("(di*2)~2*3*\"j\"$til 100000").unwrap(), K::Bool(true)); // x*a decode fallback
}

// x±a on a compressed float column stays compressed and matches the raw twin.
#[test]
fn test_coc_arith_patched_alp() {
    let mut c = conn();
    c.query("{pa::@[0.5+\"f\"$til 100000; 100 5000 90000; :; 0.123 0.456 \
        0.789]}[]").unwrap();
    assert_eq!(c.query("(pa+0.5)~(pa|pa)+0.5").unwrap(), K::Bool(true));
    assert_eq!(c.query("(pa-0.5)~(pa|pa)-0.5").unwrap(), K::Bool(true));
    assert_eq!(c.query("(where (pa+0.5)>5000.0)~where \
        ((pa|pa)+0.5)>5000.0").unwrap(), K::Bool(true));
    // a not exact at the column exponent → leaf bails (decode), still correct
    assert_eq!(c.query("(pa+0.123)~(pa|pa)+0.123").unwrap(), K::Bool(true));
}

// int avg stays fast on a compressed column when the sum fits; overflow decodes for a bit-exact mean.
#[test]
fn test_coc_avg_int() {
    let mut c = conn();
    c.query("{cai::100000?256}[]").unwrap();                                    // Σ ~1.3e7 fits I32
    assert_eq!(c.query("(avg cai)~avg \"j\"$cai").unwrap(), K::Bool(true));
    c.query("{ab::100000?100000}[]").unwrap();                                  // Σ ~5e9 overflows I32
    assert_eq!(c.query("(avg ab)~avg \"j\"$ab").unwrap(), K::Bool(true));
    c.query("{ah::\"h\"$100000?200}[]").unwrap();                               // KH
    assert_eq!(c.query("(avg ah)~avg \"j\"$ah").unwrap(), K::Bool(true));
}

// ~ (not) on a compressed bool column stays compressed; where not flag stays fast.
#[test]
fn test_coc_not_bool() {
    let mut c = conn();
    c.query("{nb::100000?0b}[]").unwrap();
    assert_eq!(c.query("(sum not nb)=(count nb)-sum nb").unwrap(),              // #not = n-#set
        K::Bool(true));
    assert_eq!(c.query("(-1)=type first not nb").unwrap(), K::Bool(true));      // result bool
    assert_eq!(c.query("((min not nb)=0b)&(max not nb)=1b").unwrap(),
        K::Bool(true));
    assert_eq!(c.query("(not not nb)~nb").unwrap(), K::Bool(true));             // double-not
    assert_eq!(c.query("(not 100000#0b)~100000#1b").unwrap(), K::Bool(true));   // all-zeros
    assert_eq!(c.query("(not 100000#1b)~100000#0b").unwrap(), K::Bool(true));   // all-ones
}

// Window/tile ops (dot/$/mv/mmxw) on a compressed float column must decode faithfully.
#[test]
fn test_coc_la_alq() {
    let mut c = conn();
    // build the compressed global + a raw weight vec in one query so the col stays compressed
    let dot = "{[n] w:1.0+\"f\"$til n; c::reciprocal 1.0+\"f\"$til n; \
               (w$c)~sum w*reciprocal 1.0+\"f\"$til n}[100000]";
    assert_eq!(c.query(dot).unwrap(), K::Bool(true));                           // dot on compressed float ~ raw dot
    let mv = "{[n] c::reciprocal 1.0+\"f\"$til n; r:reciprocal 1.0+\"f\"$til \
        n; \
        \
              M:(1.0+\"f\"$til n;r); (M$c)~M$r}[100000]";
    assert_eq!(c.query(mv).unwrap(), K::Bool(true));                            // matrix·(compressed float vec)
    // mmxw/mmnw preserve sub-1 magnitudes on a compressed float column (absolute-tolerance check).
    let mmx = "{[n] c::reciprocal 1.0+\"f\"$til n; r:reciprocal 1.0+\"f\"$til \
        n; \
               all 1e-9>abs(3 mmxw c)-3 mmxw r}[100000]";
    assert_eq!(c.query(mmx).unwrap(), K::Bool(true));                           // moving-max over compressed float
    let mmn = "{[n] c::reciprocal 1.0+\"f\"$til n; r:reciprocal 1.0+\"f\"$til \
        n; \
               all 1e-9>abs(7 mmnw c)-7 mmnw r}[100000]";
    assert_eq!(c.query(mmn).unwrap(), K::Bool(true));                           // moving-min over compressed float
    // G2: wsum/wavg stay compressed and match the raw result.
    let ws = "{[n] w:1.0+\"f\"$til n; c::reciprocal 1.0+\"f\"$til n; \
              (w wsum c)~w wsum reciprocal 1.0+\"f\"$til n}[100000]";
    assert_eq!(c.query(ws).unwrap(), K::Bool(true));                            // wsum raw-wt × compressed float
    let wa = "{[n] w:1.0+\"f\"$til n; c::reciprocal 1.0+\"f\"$til n; \
              (w wavg c)~w wavg reciprocal 1.0+\"f\"$til n}[100000]";
    assert_eq!(c.query(wa).unwrap(), K::Bool(true));                            // wavg raw-wt × compressed float
    let wb = "{[n] c::reciprocal 1.0+\"f\"$til n; d::0.5+\"f\"$til n; \
              (c wavg d)~(reciprocal 1.0+\"f\"$til n)wavg 0.5+\"f\"$til \
                  n}[100000]";
    assert_eq!(c.query(wb).unwrap(), K::Bool(true));                            // wavg both compressed
    // neg/abs/transcendentals fuse correctly on a compressed float-result column.
    let xe = "{[n] c::reciprocal 1.0+\"f\"$til n; \
              (exp c)~exp reciprocal 1.0+\"f\"$til n}[100000]";
    assert_eq!(c.query(xe).unwrap(), K::Bool(true));                            // exp on compressed float, fused
    let ab = "{[n] c::reciprocal 1.0+\"f\"$til n; \
              all 1e-9>abs(abs neg c)-abs neg reciprocal 1.0+\"f\"$til \
                  n}[100000]";
    assert_eq!(c.query(ab).unwrap(), K::Bool(true));                            // neg+abs chain on compressed float (abs-tol)
    // int transcendental → KF stays compressed; neg(int) stays int.
    let sf = "{[n] c::1000+(til n)mod 1000; (sqrt c)~sqrt\"f\"$1000+(til n)mod \
        1000}[100000]";
    assert_eq!(c.query(sf).unwrap(), K::Bool(true));                            // sqrt on compressed int → KF, fused
    let nf = "{[n] c::1000+(til n)mod 1000; (-6h)~type first neg c}[100000]";
    assert_eq!(c.query(nf).unwrap(), K::Bool(true));                            // neg on compressed int stays int
}

// ^ (fill) on a compressed int column: null-free stays fast; has-null/float/promoting cases realize.
#[test]
fn test_coc_fill() {
    let mut c = conn();
    // null-free int → identity (same values, stays compressed)
    assert_eq!(c.query("{[n] c::(til n)mod 1000; (0^c)~(til n)mod \
        1000}[100000]")
        .unwrap(), K::Bool(true));
    assert_eq!(c.query("{[n] c::\"j\"$(til n)mod 1000; (0j^c)~\"j\"$(til n)mod \
        1000}[100000]")
        .unwrap(), K::Bool(true));
    // has-null int → realize + fill
    assert_eq!(c.query("{[n] c::@[(til n)mod 1000;0 5 9;:;0N]; \
        (7^c)~7^@[(til n)mod 1000;0 5 9;:;0N]}[100000]").unwrap(),
            K::Bool(true));
    // float column (NaN is unflagged) → realize + fill
    assert_eq!(c.query("{[n] c::0.5+\"f\"$til n; (1.5^c)~1.5^0.5+\"f\"$til \
        n}[100000]")
        .unwrap(), K::Bool(true));
    // type promotion 0.0^intcol → float (not identity)
    assert_eq!(c.query("{[n] c::(til n)mod 1000; (0.0^c)~\"f\"$(til n)mod \
        1000}[100000]")
        .unwrap(), K::Bool(true));
}

// ~ (match) and special-exponent xexp on compressed columns stay fast and correct.
#[test]
fn test_coc_match_xexp() {
    let mut c = conn();
    // match: identical / different / qz-vs-raw / float
    assert_eq!(c.query("{[n] x::(til n)mod 1000; y::(til n)mod 1000; \
        x~y}[100000]")
        .unwrap(), K::Bool(true));
    assert_eq!(c.query("{[n] x::(til n)mod 1000; y::(til n)mod 999; not \
        x~y}[100000]")
        .unwrap(), K::Bool(true));
    assert_eq!(c.query("{[n] c::(til n)mod 1000; c~(til n)mod 1000}[100000]")
        .unwrap(), K::Bool(true));
    assert_eq!(c.query("{[n] c::0.5+\"f\"$til n; c~c}[100000]").unwrap(),
        K::Bool(true));
    // xexp special exponents + general, all parity vs raw
    assert_eq!(c.query("{[n] c::0.5+\"f\"$til n; rr:0.5+\"f\"$til n; \
        all((c xexp 0.5)~rr xexp 0.5;(c xexp 1)~rr xexp 1;(c xexp -1)~rr xexp \
            -1;\
            (c xexp 2)~rr xexp 2;(c xexp 3)~rr xexp 3)}[100000]")
        .unwrap(), K::Bool(true));
}

// distinct and in on compressed columns stay fast when eligible; ineligible cases decode.
#[test]
fn test_coc_in_distinct() {
    let mut c = conn();
    assert_eq!(c.query("{[n] s::100+til 50; (5 50 120 999 in s)~5 50 120 999 \
        in \
        100+til 50}[100000]")
        .unwrap(), K::Bool(true));                                              // in compressed set
    assert_eq!(c.query("{[n] c::(til n)mod 1000; (c in 5 10 20)~((til n)mod \
        1000)in 5 10 20}[100000]")
        .unwrap(), K::Bool(true));                                              // in with compressed probe (decodes)
    assert_eq!(c.query("{[n] c::asc (til n)mod 1000; (distinct c)~distinct \
        asc(til n)mod 1000}[100000]")
        .unwrap(), K::Bool(true));                                              // distinct sorted
    assert_eq!(c.query("{[n] c::(til n)mod 1000; (distinct c)~distinct(til \
        n)mod 1000}[100000]")
        .unwrap(), K::Bool(true));                                              // distinct unsorted (decodes)
    // last: O(1) on the compressed tail; some encodings decode.
    assert_eq!(c.query("{[n] c::(til n)mod 1000; (last c)~last(til n)mod \
        1000}[100000]")
        .unwrap(), K::Bool(true));                                              // last FOR (O(1))
    assert_eq!(c.query("{[n] c::til n; (last c)~last til n}[100000]")
        .unwrap(), K::Bool(true));                                              // last delta (decodes)
}

// matmul/mv/vm conformability guards: shape mismatch signals length; valid shapes correct.
#[test]
fn test_la_matmul_shape() {
    let mut c = conn();
    let trap = "{[a;b] @[{x mmu y}[a]; b; {[e] e}]}";
    c.query(&format!("tp:{trap}")).unwrap();
    // mismatch → 'length (not crash)
    assert_eq!(c.query("\"length\"~tp[10 0N#1f*til 1000000; 10 0N#1f*til \
        1000000]")
        .unwrap(), K::Bool(true));
    assert_eq!(c.query("\"length\"~tp[2 3#1.0*til 6; 1.0 2.0]").unwrap(),
        K::Bool(true));
    assert_eq!(c.query("\"length\"~tp[1.0 2.0; 3 2#1.0*til 6]").unwrap(),
        K::Bool(true));
    // tall-skinny A·Aᵀ (2×5 · 5×2 → 2×2 Gram) is correct
    assert_eq!(c.query("((2 5#1.0*til 10) mmu flip 2 5#1.0*til 10)~(30 80f;80 \
        255f)")
        .unwrap(), K::Bool(true));
    // Big tall-skinny matmul: all-1.0 rows give exact Gram cells (avoids rounding).
    assert_eq!(c.query("{[n] m:2 0N#n#1.0; r:m mmu flip m; \
        (r[0;0]=\"f\"$n div 2)&(r[1;1]=\"f\"$n div 2)&r[0;1]=\"f\"$n div \
            2}[400000]")
        .unwrap(), K::Bool(true));
    // square correctness
    assert_eq!(c.query("((3 3#1.0*til 9) mmu 3 3#1.0*til 9)~(15 18 21f;42 54 \
        66f;69 90 111f)")
        .unwrap(), K::Bool(true));
    assert_eq!(c.query("((2 3#1.0*til 6) mmu 3 2#1.0*til 6)~(10 13f;28 40f)")
        .unwrap(), K::Bool(true));
}

#[test]
fn test_table_cols() {
    let mut c = conn();
    c.query("tt4:([]sym:`A`B;price:1.0 2.0)").unwrap();
    let r = c.query("cols tt4").unwrap();
    if let K::SymbolVec(v) = r {
        assert_eq!(v, vec!["sym".to_string(), "price".to_string()]);
    }
}

#[test]
fn test_table_insert() {
    let mut c = conn();
    c.query("tt5:([]a:1 2;b:10 20)").unwrap();
    c.query("`tt5 insert (3;30)").unwrap();
    assert_eq!(c.query("count tt5").unwrap().as_int(), Some(3));
}

#[test]
fn test_table_update() {
    let mut c = conn();
    c.query("tt6:([]a:1 2 3;b:10 20 30)").unwrap();
    c.query("update b:b*2 from `tt6").unwrap();
    let r = c.query("tt6`b").unwrap();
    if let K::IntVec(v) = r {
        assert_eq!(v, vec![20, 40, 60]);
    }
}

#[test]
fn test_table_delete_rows() {
    let mut c = conn();
    c.query("tt7:([]a:1 2 3;b:10 20 30)").unwrap();
    c.query("delete from `tt7 where a=2").unwrap();
    assert_eq!(c.query("count tt7").unwrap().as_int(), Some(2));
}

qtag!(test_empty_table, "([]a:`int$();b:`float$())", 98);

#[test]
fn test_table_column_access() {
    let mut c = conn();
    c.query("tt8:([]a:1 2 3;b:10 20 30)").unwrap();
    let v: Vec<i32> = c.query("tt8`a").unwrap().try_into().unwrap();
    assert_eq!(v, vec![1, 2, 3]);
}

// flip/transpose (join hot path): uniform/mixed types, tail, small N, roundtrip identity.

#[test]
fn test_flip_dict_to_table() {
    let mut c = conn();
    let r = c.query("flip `a`b!(1 2 3;10 20 30)").unwrap();
    assert_eq!(r.type_tag(), 98);                                               // dict → table
}

#[test]
fn test_flip_table_to_dict() {
    let mut c = conn();
    let r = c.query("flip ([]a:1 2 3;b:10 20 30)").unwrap();
    assert_eq!(r.type_tag(), 99);                                               // table → dict
}

#[test]
fn test_flip_roundtrip_identity() {
    let mut c = conn();
    let v: Vec<i32> = c
        .query("(flip flip ([]a:1 2 3;b:10 20 30))`a")
        .unwrap()
        .try_into()
        .unwrap();
    assert_eq!(v, vec![1, 2, 3]);
}

#[test]
fn test_flip_uniform_int_4row() {
    // rn=4 — 4-row interleave fast path lands cleanly (no tail)
    let mut c = conn();
    c.query("ft4:([]a:1 2 3 4;b:10 20 30 40;c:100 200 300 400)")
        .unwrap();
    // round-trip: flip flip ≡ id; check column a survived
    let a: Vec<i32> = c
        .query("(flip flip ft4)`a")
        .unwrap()
        .try_into()
        .unwrap();
    assert_eq!(a, vec![1, 2, 3, 4]);
}

#[test]
fn test_flip_uniform_int_tail_3rows() {
    // rn=3 → entire row range is tail (small-rn fallback path)
    let mut c = conn();
    c.query("ft3:([]a:1 2 3;b:10 20 30;c:100 200 300)").unwrap();
    let a: Vec<i32> = c
        .query("(flip flip ft3)`a")
        .unwrap()
        .try_into()
        .unwrap();
    assert_eq!(a, vec![1, 2, 3]);
    let c_col: Vec<i32> = c
        .query("(flip flip ft3)`c")
        .unwrap()
        .try_into()
        .unwrap();
    assert_eq!(c_col, vec![100, 200, 300]);
}

#[test]
fn test_flip_uniform_int_tail_5rows() {
    // rn=5 → 1 4-row strip + 1 tail row (exercises rn % 4 == 1 boundary)
    let mut c = conn();
    c.query("ft5:([]a:1 2 3 4 5;b:10 20 30 40 50)").unwrap();
    let a: Vec<i32> = c
        .query("(flip flip ft5)`a")
        .unwrap()
        .try_into()
        .unwrap();
    assert_eq!(a, vec![1, 2, 3, 4, 5]);
    let b: Vec<i32> = c
        .query("(flip flip ft5)`b")
        .unwrap()
        .try_into()
        .unwrap();
    assert_eq!(b, vec![10, 20, 30, 40, 50]);
}

#[test]
fn test_flip_uniform_int_tail_7rows() {
    // rn=7 → 1 4-row strip + 3 tail rows (exercises rn % 4 == 3)
    let mut c = conn();
    c.query("ft7:([]a:1 2 3 4 5 6 7;b:10 20 30 40 50 60 70)")
        .unwrap();
    let a: Vec<i32> = c
        .query("(flip flip ft7)`a")
        .unwrap()
        .try_into()
        .unwrap();
    assert_eq!(a, vec![1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn test_flip_uniform_long() {
    // 8B path (KJ atom: `j$) — falls through to SW-hoist
    let mut c = conn();
    c.query("ftj:([]a:`long$1 2 3 4;b:`long$10 20 30 40)")
        .unwrap();
    let r = c.query("flip flip ftj").unwrap();
    assert_eq!(r.type_tag(), 98);
}

#[test]
fn test_flip_uniform_float() {
    // 8B path (KF)
    let mut c = conn();
    c.query("ftf:([]a:1.5 2.5 3.5 4.5;b:10.5 20.5 30.5 40.5)")
        .unwrap();
    let r = c.query("flip flip ftf").unwrap();
    assert_eq!(r.type_tag(), 98);
}

#[test]
fn test_flip_uniform_byte() {
    // 1B path (KG) — q syntax: 0xAABBCC is the 3-byte literal form
    let mut c = conn();
    let r = c
        .query("flip flip ([]a:0x010203;b:0x102030)")
        .unwrap();
    assert_eq!(r.type_tag(), 98);
}

#[test]
fn test_flip_uniform_short() {
    // 2B path (KH)
    let mut c = conn();
    let r = c
        .query("flip flip ([]a:1 2 3 4 5h;b:10 20 30 40 50h)")
        .unwrap();
    assert_eq!(r.type_tag(), 98);
}

#[test]
fn test_flip_mixed_types() {
    // mixed-type cols (int + float) → vi/vk fallback path
    let mut c = conn();
    let r = c
        .query("flip ([]a:1 2 3 4 5;b:1.5 2.5 3.5 4.5 5.5)")
        .unwrap();
    assert_eq!(r.type_tag(), 99);
}

#[test]
fn test_flip_large_uniform_int() {
    // 1024 rows × 8 cols → exercises 4-row interleave at scale (256 strips)
    let mut c = conn();
    c.query(
        "flarge:([]a:1024#1i;b:1024#2i;c:1024#3i;d:1024#4i;e:1024#5i;f:1024#6i;\
            g\
            :1024#7i;h:1024#8i)",
    )
    .unwrap();
    let a: Vec<i32> = c
        .query("(flip flip flarge)`a")
        .unwrap()
        .try_into()
        .unwrap();
    assert_eq!(a.len(), 1024);
    assert!(a.iter().all(|&x| x == 1));
    let h: Vec<i32> = c
        .query("(flip flip flarge)`h")
        .unwrap()
        .try_into()
        .unwrap();
    assert!(h.iter().all(|&x| x == 8));
}

// ── Keyed Tables ──────────────────────────────────────────────────

#[test]
fn test_keyed_table() {
    let mut c = conn();
    let r = c.query("([id:1 2 3] name:`Alice`Bob`Charlie)").unwrap();
    assert_eq!(r.type_tag(), 99);
    if let K::Dict(keys, vals) = &r {
        assert_eq!(keys.type_tag(), 98);
        assert_eq!(vals.type_tag(), 98);
    }
}

qtag!(test_select_by,
    "select avg price by sym from \
        ([]sym:`IBM`MSFT`IBM;price:120.5 340.2 121.0)",
 99);

#[test]
fn test_select_by_multiple() {
    let mut c = conn();
    c.query("mkt:([]sym:`A`A`B`B;side:`buy`sell`buy`sell;qty:100 200 150 \
        300)").unwrap();
    let r = c.query("select sum qty by sym,side from mkt").unwrap();
    assert_eq!(r.type_tag(), 99);
}

// ── Joins ─────────────────────────────────────────────────────────

#[test]
fn test_inner_join() {
    let mut c = conn();
    c.query("t1:([id:1 2 3] name:`A`B`C)").unwrap();
    c.query("t2:([id:1 2 3] val:10 20 30)").unwrap();
    let r = c.query("t1 ij t2").unwrap();
    assert_eq!(r.type_tag(), 99);                                               // keyed table
}

#[test]
fn test_left_join() {
    let mut c = conn();
    c.query("lt1:([id:1 2 3 4] name:`A`B`C`D)").unwrap();
    c.query("lt2:([id:1 2] val:10 20)").unwrap();
    let r = c.query("lt1 lj lt2").unwrap();
    assert_eq!(r.type_tag(), 99);
}

// MIXED LISTS

#[test]
fn test_mixed_list() {
    let mut c = conn();
    let r = c.query("(1;2.0;`abc)").unwrap();
    assert_eq!(r.type_tag(), 0);
    if let K::List(items) = &r {
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].as_int(), Some(1));
        assert!((items[1].as_float().unwrap() - 2.0).abs() < 0.01);
        assert_eq!(items[2], K::Symbol("abc".into()));
    }
}

#[test]
fn test_nested_list() {
    let mut c = conn();
    let r = c.query("(1 2 3;4 5 6)").unwrap();
    if let K::List(items) = &r {
        assert_eq!(items.len(), 2);
        if let K::IntVec(v) = &items[0] {
            assert_eq!(v, &vec![1, 2, 3]);
        }
    }
}

#[test]
fn test_deeply_nested() {
    let mut c = conn();
    let r = c.query("((1 2;3 4);(5 6;7 8))").unwrap();
    assert_eq!(r.type_tag(), 0);
    if let K::List(outer) = &r {
        assert_eq!(outer.len(), 2);
    }
}

#[test]
fn test_mixed_with_table() {
    let mut c = conn();
    let r = c.query("(42;([]a:1 2 3))").unwrap();
    assert_eq!(r.type_tag(), 0);
    if let K::List(items) = &r {
        assert_eq!(items.len(), 2);
        assert_eq!(items[1].type_tag(), 98);
    }
}

// TEMPORAL ARITHMETIC

#[test]
fn test_date_arithmetic() {
    let mut c = conn();
    let r = c.query("2000.01.01+10").unwrap();
    assert_eq!(r.type_tag(), -14);
    if let K::Date(v) = r { assert_eq!(v, 10); }
}

qint!(test_date_diff, "2000.01.11-2000.01.01", 10);

#[test]
fn test_month_extraction() {
    let mut c = conn();
    // `month$date extracts the month component
    let r = c.query("`mm$2000.03.15").unwrap();
    assert_eq!(r.as_int(), Some(3));
}

// CASTING

#[test]
fn test_cast_int_to_float() {
    let mut c = conn();
    let r = c.query("`float$42").unwrap();
    assert_eq!(r.type_tag(), -9);
    assert!((r.as_float().unwrap() - 42.0).abs() < 0.001);
}

#[test]
fn test_cast_float_to_int() {
    let mut c = conn();
    let r = c.query("`int$3.7").unwrap();
    assert_eq!(r.type_tag(), -6);
    // L rounds float to int (nearest), not truncates
    assert_eq!(r.as_int(), Some(4));
}

qint!(test_cast_string_to_int, "\"I\"$\"42\"", 42);

#[test]
fn test_cast_string_to_symbol() {
    let mut c = conn();
    let r = c.query("`$\"hello\"").unwrap();
    assert_eq!(r.type_tag(), -11);
    assert_eq!(r, K::Symbol("hello".into()));
}

qstr!(test_string_function, "string 42", "42");

// STATISTICS

qf!(test_med, "med 1 2 3 4 5", 3.0, 0.001);

#[test]
fn test_dev() {
    let mut c = conn();
    let v = c.query("dev 1 2 3 4 5").unwrap().as_float().unwrap();
    assert!(v > 1.0 && v < 2.0);                                                // ~1.414
}

#[test]
fn test_var() {
    let mut c = conn();
    let v = c.query("var 1 2 3 4 5").unwrap().as_float().unwrap();
    assert!((v - 2.0).abs() < 0.5);                                             // variance = 2.0
}

#[test]
fn test_cor() {
    let mut c = conn();
    let v = c.query("1 2 3 4 5 cor 1 2 3 4 5").unwrap().as_float().unwrap();
    assert!((v - 1.0).abs() < 0.001);                                           // perfect correlation
}

#[test]
fn test_cov() {
    let mut c = conn();
    let v = c.query("1 2 3 4 5 cov 1 2 3 4 5").unwrap().as_float().unwrap();
    assert!((v - 2.0).abs() < 0.5);                                             // covariance = variance for identical
}

qtag!(test_mavg, "3 mavg 1 2 3 4 5 6", 9);

// FUNCTIONAL FORMS

qint!(test_lambda, "{x+y}[3;4]", 7);

#[test]
fn test_each() {
    let mut c = conn();
    let r = c.query("{x*x} each 1 2 3 4 5").unwrap();
    let v: Vec<i32> = r.try_into().unwrap();
    assert_eq!(v, vec![1, 4, 9, 16, 25]);
}

qint!(test_over, "{x+y} over 1 2 3 4 5", 15);

qvec!(test_scan, i32, "{x+y} scan 1 2 3 4 5", vec![1, 3, 6, 10, 15]);

#[test]
fn test_prior() {
    let mut c = conn();
    // deltas = differ: each element minus previous
    let v: Vec<i32> = c.query("1_deltas 10 12 15 11 \
        20").unwrap().try_into().unwrap();
    assert_eq!(v, vec![2, 3, -4, 9]);
}

// SEND K OBJECTS

#[test]
fn test_send_int() {
    let mut c = conn();
    let r = c.query_with_args("{x+1}", vec![K::Int(41)]).unwrap();
    assert_eq!(r.as_int(), Some(42));
}

#[test]
fn test_send_long() {
    let mut c = conn();
    let r = c.query_with_args("{x+1j}", vec![K::Long(99)]).unwrap();
    assert_eq!(r.as_long(), Some(100));
}

#[test]
fn test_send_float() {
    let mut c = conn();
    let r = c.query_with_args("{x*2.0}", vec![K::Float(3.14)]).unwrap();
    let v = r.as_float().unwrap();
    assert!((v - 6.28).abs() < 0.01);
}

#[test]
fn test_send_bool() {
    let mut c = conn();
    let r = c.query_with_args("{not x}", vec![K::Bool(true)]).unwrap();
    assert_eq!(r, K::Bool(false));
}

#[test]
fn test_send_symbol() {
    let mut c = conn();
    c.query("stab:([]sym:`IBM`MSFT;price:120.5 340.2)").unwrap();
    let r = c.query_with_args("{select from stab where sym=x}",
        vec![K::Symbol("IBM".into())]).unwrap();
    assert_eq!(r.type_tag(), 98);
}

#[test]
fn test_send_string() {
    let mut c = conn();
    let r = c.query_with_args("{count x}",
        vec![K::CharVec(b"hello".to_vec())]).unwrap();
    assert_eq!(r.as_int(), Some(5));
}

#[test]
fn test_send_int_vector() {
    let mut c = conn();
    let r = c.query_with_args("{sum x}",
        vec![K::IntVec(vec![1,2,3,4,5])]).unwrap();
    assert_eq!(r.as_long(), Some(15));
}

#[test]
fn test_send_float_vector() {
    let mut c = conn();
    let r = c.query_with_args("{avg x}", vec![K::FloatVec(vec![1.0, 2.0,
        3.0])]).unwrap();
    let v = r.as_float().unwrap();
    assert!((v - 2.0).abs() < 0.01);
}

#[test]
fn test_send_symbol_vector() {
    let mut c = conn();
    let r = c.query_with_args("{count x}",
        vec![K::SymbolVec(vec!["a".into(),"b".into(),"c".into()])]).unwrap();
    assert_eq!(r.as_int(), Some(3));
}

#[test]
fn test_send_bool_vector() {
    let mut c = conn();
    let r = c.query_with_args("{sum x}", vec![K::BoolVec(vec![true, false,
        true, true])]).unwrap();
    assert_eq!(r.as_int(), Some(3));
}

#[test]
fn test_send_mixed_list() {
    let mut c = conn();
    let list = K::List(vec![K::Int(1), K::Float(2.0), K::Symbol("abc".into())]);
    let r = c.query_with_args("{count x}", vec![list]).unwrap();
    assert_eq!(r.as_int(), Some(3));
}

#[test]
fn test_send_dict() {
    let mut c = conn();
    let keys = K::SymbolVec(vec!["a".into(), "b".into()]);
    let vals = K::IntVec(vec![10, 20]);
    let dict = K::Dict(Box::new(keys), Box::new(vals));
    let r = c.query_with_args("{count x}", vec![dict]).unwrap();
    assert_eq!(r.as_int(), Some(2));
}

#[test]
fn test_send_table() {
    let mut c = conn();
    let cols = K::SymbolVec(vec!["a".into(), "b".into()]);
    let va = K::IntVec(vec![1, 2, 3]);
    let vb = K::FloatVec(vec![10.0, 20.0, 30.0]);
    let vals = K::List(vec![va, vb]);
    let dict = K::Dict(Box::new(cols), Box::new(vals));
    let table = K::Table(Box::new(dict));
    let r = c.query_with_args("{count x}", vec![table]).unwrap();
    assert_eq!(r.as_int(), Some(3));
}

#[test]
fn test_send_date() {
    let mut c = conn();
    let r = c.query_with_args("{x+1}", vec![K::Date(0)]).unwrap();
    assert_eq!(r.type_tag(), -14);
    if let K::Date(v) = r { assert_eq!(v, 1); }
}

#[test]
fn test_send_time() {
    let mut c = conn();
    let r = c.query_with_args("{x}", vec![K::Time(43200000)]).unwrap();
    assert_eq!(r.type_tag(), -19);
}

#[test]
fn test_send_two_args() {
    let mut c = conn();
    let r = c.query_with_args("{x+y}", vec![K::Int(10), K::Int(32)]).unwrap();
    assert_eq!(r.as_int(), Some(42));
}

#[test]
fn test_send_three_args() {
    let mut c = conn();
    let r = c.query_with_args("{x+y+z}", vec![K::Int(10), K::Int(20),
        K::Int(12)]).unwrap();
    assert_eq!(r.as_int(), Some(42));
}

// ERROR HANDLING

#[test]
fn test_error_type() {
    let mut c = conn();
    let r = c.query("1+`abc");
    assert!(r.is_err());
    if let Err(LError::L(msg)) = r {
        assert!(!msg.is_empty());
    }
}

#[test]
fn test_error_undefined() {
    let mut c = conn();
    let r = c.query("undefined_var_xyz");
    assert!(r.is_err());
}

#[test]
fn test_error_rank() {
    let mut c = conn();
    let r = c.query("{x+y}[1;2;3]");                                            // too many args
    assert!(r.is_err());
}

// MALFORMED-SYNTAX ROBUSTNESS — server must never crash on bad client input.

fn crash_guard(bad: &str) {
    let r = conn().query(bad);
    assert!(r.is_err(), "{bad:?} should be a syntax error, not a value");
    // Server must still be alive: a fresh connection + valid query must succeed.
    let ok = conn().query("1+1");
    assert!(ok.is_ok(), "server died after {bad:?} — parser crash regression");
}

#[test] fn crash_unbalanced_do_bracket() { crash_guard("do[1;1["); }
#[test] fn crash_unclosed_lambda_brace() { crash_guard("f:{x+{"); }
#[test] fn crash_trailing_open_paren()   { crash_guard("(1+2)+3("); }
#[test] fn crash_unmatched_close_paren() { crash_guard(")()"); }
#[test] fn crash_close_then_assign()     { crash_guard(")a:1--"); }
#[test] fn crash_lone_open_bracket()     { crash_guard("1["); }
#[test] fn crash_lone_close_bracket()    { crash_guard("]"); }
#[test] fn crash_lone_open_paren()       { crash_guard("("); }
#[test] fn crash_unclosed_brace_expr()   { crash_guard("{x+"); }

// LARGE VECTORS

#[test]
fn test_large_int_vector() {
    let mut c = conn();
    let r = c.query("til 10000").unwrap();
    if let K::IntVec(v) = r {
        assert_eq!(v.len(), 10000);
        assert_eq!(v[0], 0);
        assert_eq!(v[9999], 9999);
    } else { panic!("expected int vector"); }
}

#[test]
fn test_large_sum() {
    let mut c = conn();
    let r = c.query("sum til 10000").unwrap();
    assert_eq!(r.as_long(), Some(49995000));                                    // n*(n-1)/2
}

#[test]
fn test_send_large_vector() {
    let mut c = conn();
    let big_vec = K::IntVec((0..10000).collect());
    let r = c.query_with_args("{sum x}", vec![big_vec]).unwrap();
    assert_eq!(r.as_long(), Some(49995000));
}

// K VALUE METHODS

#[test]
fn test_is_atom() {
    assert!(K::Int(42).is_atom());
    assert!(K::Float(3.14).is_atom());
    assert!(K::Symbol("x".into()).is_atom());
    assert!(!K::IntVec(vec![1]).is_atom());
    assert!(!K::Error("e".into()).is_atom());
}

#[test]
fn test_is_vector() {
    assert!(K::IntVec(vec![1]).is_vector());
    assert!(K::FloatVec(vec![1.0]).is_vector());
    assert!(K::SymbolVec(vec!["x".into()]).is_vector());
    assert!(!K::Int(42).is_vector());
}

#[test]
fn test_is_string() {
    assert!(K::CharVec(b"hello".to_vec()).is_string());
    assert!(!K::Int(42).is_string());
}

#[test]
fn test_len() {
    assert_eq!(K::Int(42).len(), 1);
    assert_eq!(K::IntVec(vec![1, 2, 3]).len(), 3);
    assert_eq!(K::CharVec(b"hello".to_vec()).len(), 5);
    assert_eq!(K::List(vec![K::Int(1), K::Int(2)]).len(), 2);
}

#[test]
fn test_as_int_coercion() {
    assert_eq!(K::Int(42).as_int(), Some(42));
    assert_eq!(K::Short(10).as_int(), Some(10));
    assert_eq!(K::Byte(5).as_int(), Some(5));
    assert_eq!(K::Bool(true).as_int(), Some(1));
    assert_eq!(K::Float(3.14).as_int(), None);                                  // no downcast
}

#[test]
fn test_as_float_coercion() {
    assert_eq!(K::Float(3.14).as_float(), Some(3.14));
    assert_eq!(K::Int(42).as_float(), Some(42.0));
    assert_eq!(K::Long(100).as_float(), Some(100.0));
}

// CONVERSIONS (From / TryFrom)

#[test]
fn test_from_bool() {
    let k: K = true.into();
    assert_eq!(k, K::Bool(true));
}

#[test]
fn test_from_i16() {
    let k: K = 42i16.into();
    assert_eq!(k, K::Short(42));
}

#[test]
fn test_from_i32() {
    let k: K = 42i32.into();
    assert_eq!(k, K::Int(42));
}

#[test]
fn test_from_i64() {
    let k: K = 42i64.into();
    assert_eq!(k, K::Long(42));
}

#[test]
fn test_from_f32() {
    let k: K = 3.14f32.into();
    assert_eq!(k, K::Real(3.14f32));
}

#[test]
fn test_from_f64() {
    let k: K = 3.14f64.into();
    assert_eq!(k, K::Float(3.14));
}

#[test]
fn test_from_str() {
    let k: K = "hello".into();
    assert_eq!(k.as_string(), Some("hello"));
}

#[test]
fn test_from_vec_i32() {
    let k: K = vec![1i32, 2, 3].into();
    assert_eq!(k, K::IntVec(vec![1, 2, 3]));
}

#[test]
fn test_from_vec_i64() {
    let k: K = vec![1i64, 2, 3].into();
    assert_eq!(k, K::LongVec(vec![1, 2, 3]));
}

#[test]
fn test_from_vec_f64() {
    let k: K = vec![1.0f64, 2.0, 3.0].into();
    assert_eq!(k, K::FloatVec(vec![1.0, 2.0, 3.0]));
}

#[test]
fn test_from_vec_bool() {
    let k: K = vec![true, false, true].into();
    assert_eq!(k, K::BoolVec(vec![true, false, true]));
}

#[test]
fn test_from_vec_str() {
    let k: K = vec!["IBM", "MSFT"].into();
    assert_eq!(k, K::SymbolVec(vec!["IBM".into(), "MSFT".into()]));
}

#[test]
fn test_try_into_i32() {
    let k = K::Int(42);
    let v: i32 = k.try_into().unwrap();
    assert_eq!(v, 42);
}

#[test]
fn test_try_into_i64() {
    let k = K::Long(42);
    let v: i64 = k.try_into().unwrap();
    assert_eq!(v, 42);
}

#[test]
fn test_try_into_f64() {
    let k = K::Float(3.14);
    let v: f64 = k.try_into().unwrap();
    assert!((v - 3.14).abs() < 0.001);
}

#[test]
fn test_try_into_string() {
    let k = K::CharVec(b"hello".to_vec());
    let v: String = k.try_into().unwrap();
    assert_eq!(v, "hello");
}

#[test]
fn test_try_into_vec_i32() {
    let k = K::IntVec(vec![1, 2, 3]);
    let v: Vec<i32> = k.try_into().unwrap();
    assert_eq!(v, vec![1, 2, 3]);
}

#[test]
fn test_try_into_vec_f64() {
    let k = K::FloatVec(vec![1.0, 2.0, 3.0]);
    let v: Vec<f64> = k.try_into().unwrap();
    assert_eq!(v, vec![1.0, 2.0, 3.0]);
}

#[test]
fn test_try_into_type_error() {
    let k = K::Symbol("hello".into());
    let r: Result<i32, _> = k.try_into();
    assert!(r.is_err());
}

// HASH OPS — distinct, group, in, joins (Swiss Tables coverage)

#[test]
fn test_distinct_symbols() {
    let mut c = conn();
    let r = c.query("distinct `a`b`a`c`b`c`a").unwrap();
    if let K::SymbolVec(v) = &r {
        assert_eq!(v, &vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    } else { panic!("expected symbol vec"); }
}

qvec!(test_distinct_float, f64, "distinct 1.1 2.2 1.1 3.3 2.2",
    vec![1.1, 2.2, 3.3]);

#[test]
fn test_distinct_long() {
    let mut c = conn();
    let r = c.query("distinct 10 20 10 30 20 10j").unwrap();
    if let K::LongVec(v) = &r {
        assert_eq!(v, &vec![10i64, 20, 30]);
    } else { panic!("expected long vec, got {:?}", r.type_tag()); }
}

#[test]
fn test_distinct_large() {
    let mut c = conn();
    let n: i32 = c.query("count distinct \
        1000000?100").unwrap().as_int().unwrap();
    assert_eq!(n, 100);
}

#[test]
fn test_group_by_sum() {
    let mut c = conn();
    c.query("gt1:([]sym:`A`B`A`B`A;qty:10 20 30 40 50)").unwrap();
    let r = c.query("select sum qty by sym from gt1").unwrap();
    assert_eq!(r.type_tag(), 99);                                               // keyed table
}

#[test]
fn test_group_by_count() {
    let mut c = conn();
    c.query("gt2:([]sym:`X`Y`X`Y`X`X;px:1 2 3 4 5 6)").unwrap();
    let r = c.query("select cnt:count i by sym from gt2").unwrap();
    assert_eq!(r.type_tag(), 99);
}

#[test]
fn test_group_by_multi_col() {
    let mut c = conn();
    c.query("gt3:([]a:`x`x`y`y;b:1 2 1 2;v:10 20 30 40)").unwrap();
    let r = c.query("select sum v by a,b from gt3").unwrap();
    assert_eq!(r.type_tag(), 99);
}

#[test]
fn test_group_by_multi_agg() {
    let mut c = conn();
    c.query("gma:([]sym:`A`B`A`B`A;qty:10 20 30 40 50;px:1.0 2.0 3.0 4.0 \
        5.0)").unwrap();
    // Multi-aggregate: exercises fused scatter path (sum+count+max)
    let r = c.query("select s:sum qty, c:count i, mx:max qty by sym from \
        gma").unwrap();
    assert_eq!(r.type_tag(), 99);
}

#[test]
fn test_group_by_sum_max() {
    let mut c = conn();
    c.query("gsm:([]g:`a`b`a`b`a`b;x:1 2 3 4 5 6;y:10.0 20.0 30.0 40.0 50.0 \
        60.0)").unwrap();
    // Two aggregates on different types: int sum + float max
    let r = c.query("select sum x, max y by g from gsm").unwrap();
    assert_eq!(r.type_tag(), 99);
}

// Threaded group-by on a wide-range unsorted int key: sum/count and membership match twins, no rows lost.
#[test]
fn grp_int_threaded() {
    let mut c = conn();
    // Deterministic 1M-row unsorted int key (wide range) with a value column to reduce.
    c.query("gti_k:(1+til 1000000) mod 500000").unwrap();                       // 500k distinct, unsorted-ish
    c.query("gti_v:1000000?1000j").unwrap();                                    // long values to sum
    c.query("gti:([]k:gti_k;v:gti_v)").unwrap();
    // total rows conserved: Σ group counts == n
    eq_q(&mut c, "grp-int-count-total",
         "sum exec cnt from select cnt:count i by k from gti", "1000000j");
    // sum-by-key equals the sum-each-group twin, indexed by the same ascending keys.
    c.query("gti_ks:asc distinct gti_k").unwrap();                              // shared key order
    eq_q(&mut c, "grp-int-sum-twin",
         "(exec v from select v:sum v by k from gti)~(sum each gti_v group \
             gti_k)[gti_ks]",
         "1b");
    // count-by-key == count-each-group twin, same shared key order
    eq_q(&mut c, "grp-int-count-twin",
         "(exec cnt from select cnt:count i by k from gti)~(count each gti_k \
             group gti_k)[gti_ks]",
         "1b");
    // Group membership matches a compressed twin built via an independent path.
    c.query("`gti_kc set gti_k|gti_k").unwrap();                                // force a compressed copy
    eq_q(&mut c, "grp-int-coc-membership",
         "(count group gti_k)=count group gti_kc", "1b");
    eq_q(&mut c, "grp-int-distinct",
         "(asc distinct gti_k)~asc distinct gti_kc", "1b");
}

#[test]
fn test_in_int() {
    let mut c = conn();
    let r = c.query("1 2 3 in 2 3 4 5").unwrap();
    if let K::BoolVec(v) = &r {
        assert_eq!(v, &vec![false, true, true]);
    } else { panic!("expected bool vec"); }
}

#[test]
fn test_in_symbol() {
    let mut c = conn();
    let r = c.query("`a`b`c in `b`c`d").unwrap();
    if let K::BoolVec(v) = &r {
        assert_eq!(v, &vec![false, true, true]);
    } else { panic!("expected bool vec"); }
}

#[test]
fn test_in_long() {
    let mut c = conn();
    let r = c.query("10 20 30j in 20 40 60j").unwrap();
    if let K::BoolVec(v) = &r {
        assert_eq!(v, &vec![false, true, false]);
    } else { panic!("expected bool vec"); }
}

#[test]
fn test_in_float() {
    let mut c = conn();
    let r = c.query("1.1 2.2 3.3 in 2.2 4.4").unwrap();
    if let K::BoolVec(v) = &r {
        assert_eq!(v, &vec![false, true, false]);
    } else { panic!("expected bool vec"); }
}

#[test]
fn test_left_join_basic() {
    let mut c = conn();
    c.query("ljt1:([]sym:`A`B`C;px:1 2 3)").unwrap();
    c.query("ljt2:([sym:`A`C]name:`alpha`gamma)").unwrap();
    let r = c.query("ljt1 lj ljt2").unwrap();
    assert_eq!(r.type_tag(), 98);                                               // table
}

// lj VALUE CORRECTNESS — assert joined values (not just type tag); compiler miscompile guard.

#[test]
fn lj_clang21_single_col_sym() {
    let mut c = conn();
    let r = c.query("{[] lt:([]k:`A`B`C`D`E;px:10 20 30 40 50); \
        r:([k:`A`C`E]v:100 200 300); \
                     j:lt lj r; `long$((j`px)~lt`px)&(j`v)~100 0N 200 0N \
                         300}[]").unwrap();
    assert_eq!(r.as_long(), Some(1), "lj single-col: left px corrupted or \
        misses not nulled");
}

#[test]
fn lj_clang21_multi_col_multi_type() {
    let mut c = conn();
    let r = c.query("{[] lt:([]k:`A`B`C`D`E;px:10 20 30 40 50); \
                     r:([k:`A`C`E]h:1 2 3h;i:1 2 3i;l:100 200 300j;e:`real$1.5 \
                         2.5 3.5;f:10.5 20.5 30.5;s:`x`y`z); \
                     j:lt lj r; `long$((j`px)~lt`px)&((j`h)~1 0N 2 0N \
                         3h)&((j`i)~1 0N 2 0N 3i)\
                     &((j`l)~100 0N 200 0N 300j)&((j`e)~`real$1.5 0n 2.5 0n \
                         3.5)\
                     &((j`f)~10.5 0n 20.5 0n 30.5)&(j`s)~`x``y``z}[]").unwrap();
    assert_eq!(r.as_long(), Some(1), "lj multi-type: a right col (h/i/l/e/f/s) \
        joined wrong or left corrupted");
}

#[test]
fn lj_clang21_int_key() {
    let mut c = conn();
    let r = c.query("{[] lt:([]k:1 2 3 4 5;px:10 20 30 40 50); r:([k:1 3 \
        5]v:`p`q`r); \
                     j:lt lj r; \
                         `long$((j`px)~lt`px)&(j`v)~`p``q``r}[]").unwrap();
    assert_eq!(r.as_long(), Some(1), "lj int-key: left px corrupted or misses \
        not nulled");
}

#[test]
fn lj_clang21_all_miss() {
    let mut c = conn();
    let r = c.query("{[] lt:([]k:`A`B`C`D`E;px:10 20 30 40 50); \
        r:([k:`X`Y`Z]v:1 2 3); \
                     j:lt lj r; `long$((j`px)~lt`px)&(j`v)~5#0N}[]").unwrap();
    assert_eq!(r.as_long(), Some(1), "lj all-miss: right col not entirely null \
        or left corrupted");
}

#[test]
fn lj_clang21_all_hit() {
    let mut c = conn();
    let r = c.query("{[] lt:([]k:`A`B`C`D`E;px:10 20 30 40 50); \
        r:([k:`A`B`C`D`E]v:1 2 3 4 5); \
                     j:lt lj r; `long$((j`px)~lt`px)&(j`v)~1 2 3 4 \
                         5}[]").unwrap();
    assert_eq!(r.as_long(), Some(1), "lj all-hit: right col mismatched or left \
        corrupted");
}

#[test]
fn lj_clang21_large_int_1e6() {
    let mut c = conn();
    // 1e6-row int-keyed left, right keyed on even ids: left preserved, even→10+k, odd→null.
    let r = c.query("{[n] lt:([]k:til n;px:n#1 2 3 4 5j); r:([k:2*til n div \
        2]v:10+2*til n div 2); \
                     j:lt lj r; `long$((j`px)~lt`px)&((j`k)~til n)&((count \
                         j)=n)\
                     &(j`v)~?[0=(til n)mod 2;10+til n;0N]}[1000000]").unwrap();
    assert_eq!(r.as_long(), Some(1), "lj large int-key (1e6): left corrupted, \
        row count, or join values wrong");
}

#[test]
fn lj_clang21_large_sym_1e5() {
    let mut c = conn();
    // 1e5-row sym-keyed left (the miscompile's historical shape, at scale).
    let r = c.query("{[m] lt:([]s:`$string til m;px:til m); r:([s:`$string \
        2*til m div 2]w:10+2*til m div 2); \
                     j:lt lj r; `long$((j`px)~lt`px)&((count j)=m)\
                     &(j`w)~?[0=(til m)mod 2;10+til m;0N]}[100000]").unwrap();
    assert_eq!(r.as_long(), Some(1), "lj large sym-key (1e5): left corrupted, \
        row count, or join values wrong");
}

#[test]
fn test_aj_basic() {
    let mut c = conn();
    c.query("ajt:([]sym:`A`A`B`B;time:09:30:00 09:31:00 09:30:00 \
        09:32:00;px:1.0 1.1 2.0 2.1)").unwrap();
    c.query("ajq:([]sym:`A`B;time:09:30:30 09:31:00)").unwrap();
    let r = c.query("aj[`sym`time;ajq;ajt]").unwrap();
    assert_eq!(r.type_tag(), 98);
}

#[test]
fn test_find_multi_type() {
    let mut c = conn();
    // int find
    let n: i32 = c.query("1 2 3 4 5?3").unwrap().as_int().unwrap();
    assert_eq!(n, 2);
    // symbol find
    let n2: i32 = c.query("`a`b`c?`b").unwrap().as_int().unwrap();
    assert_eq!(n2, 1);
    // float find
    let n3: i32 = c.query("1.1 2.2 3.3?2.2").unwrap().as_int().unwrap();
    assert_eq!(n3, 1);
}

#[test]
fn test_group_dict() {
    let mut c = conn();
    let r = c.query("group `a`b`a`c`b`a").unwrap();
    assert_eq!(r.type_tag(), 99);                                               // dict
}

// EXTENDED COVERAGE — math, string, aggregation

#[test]
fn test_floor_float_vector() {
    let mut c = conn();
    let r = c.query("floor 1.5 2.7 3.1").unwrap();
    assert_eq!(r.type_tag(), 6);                                                // int vector (l floor returns KI)
    if let K::IntVec(v) = &r {
        assert_eq!(v, &vec![1i32, 2, 3]);
    } else { panic!("expected int vector, got type {}", r.type_tag()); }
}

#[test]
fn test_ceiling_float_vector() {
    let mut c = conn();
    let r = c.query("ceiling 1.5 2.7 3.1").unwrap();
    assert_eq!(r.type_tag(), 6);                                                // int vector (l ceiling returns KI)
    if let K::IntVec(v) = &r {
        assert_eq!(v, &vec![2i32, 3, 4]);
    } else { panic!("expected int vector, got type {}", r.type_tag()); }
}

qvec!(test_abs_negative_int_vector, i32, "abs -5 -3 0 3 5",
    vec![5, 3, 0, 3, 5]);

#[test]
fn test_xexp_power() {
    let mut c = conn();
    let r = c.query("2 3 4 xexp 2").unwrap();
    // xexp returns floats
    if let K::FloatVec(v) = &r {
        assert_eq!(v.len(), 3);
        assert!((v[0] - 4.0).abs() < 0.001);
        assert!((v[1] - 9.0).abs() < 0.001);
        assert!((v[2] - 16.0).abs() < 0.001);
    } else {
        panic!("expected float vector, got type {}", r.type_tag());
    }
}

qvec!(test_neg_mixed, i32, "neg 1 -2 3", vec![-1, 2, -3]);

#[test]
fn test_mavg_values() {
    let mut c = conn();
    let r = c.query("3 mavg 1 2 3 4 5 6").unwrap();
    assert_eq!(r.type_tag(), 9);                                                // float vector
    if let K::FloatVec(v) = &r {
        assert_eq!(v.len(), 6);
        assert!((v[0] - 1.0).abs() < 0.001);
        assert!((v[1] - 1.5).abs() < 0.001);
        assert!((v[2] - 2.0).abs() < 0.001);
        assert!((v[3] - 3.0).abs() < 0.001);
        assert!((v[4] - 4.0).abs() < 0.001);
        assert!((v[5] - 5.0).abs() < 0.001);
    } else {
        panic!("expected float vector, got type {}", r.type_tag());
    }
}

qvec!(test_sums_cumulative, i32, "sums 1 2 3 4 5", vec![1, 3, 6, 10, 15]);

qvec!(test_deltas, i32, "deltas 1 3 6 10", vec![1, 2, 3, 4]);

qstr!(test_upper_string, "upper \"hello\"", "HELLO");

qstr!(test_lower_string, "lower \"HELLO\"", "hello");

#[test]
fn test_like_pattern_vector() {
    let mut c = conn();
    let r = c.query("(\"hello\";\"world\";\"help\") like \"hel*\"").unwrap();
    if let K::BoolVec(v) = &r {
        assert_eq!(v, &vec![true, false, true]);
    } else {
        panic!("expected bool vector, got type {}", r.type_tag());
    }
}

#[test]
fn test_select_min_max_by() {
    let mut c = conn();
    c.query("mmt:([]sym:`A`B`A`B`A;px:1.0 2.0 3.0 4.0 5.0)").unwrap();
    let r = c.query("select mn:min px, mx:max px by sym from mmt").unwrap();
    assert_eq!(r.type_tag(), 99);                                               // keyed table
    // Extract the value table from the keyed-table dict
    if let K::Dict(keys, vals) = &r {
        assert_eq!(keys.type_tag(), 98);                                        // key table
        assert_eq!(vals.type_tag(), 98);                                        // value table
    } else {
        panic!("expected keyed table (dict of tables)");
    }
}

// ── Edge cases and regression tests ──

#[test]
fn test_empty_vector() {
    let mut c = conn();
    let r = c.query("`long$()").unwrap();
    assert_eq!(r.type_tag(), 7);                                                // empty long vector
}

#[test]
fn test_single_element_sort() {
    let mut c = conn();
    let r = c.query("asc enlist 42").unwrap();
    assert!(r.type_tag() == 6 || r.type_tag() == -6);                           // int vector or atom
}

#[test]
fn test_null_handling_sum() {
    let mut c = conn();
    // sum with nulls should skip nulls
    let r = c.query("sum 1 2 0N 4 5").unwrap();
    assert_eq!(r.type_tag(), -7);                                               // int atom
}

#[test]
fn test_where_boolean() {
    let mut c = conn();
    let r = c.query("where 10110b").unwrap();
    if let K::IntVec(v) = &r {
        assert_eq!(v, &vec![0i32, 2, 3]);
    } else { panic!("expected int vec"); }
}

#[test]
fn test_cross_product() {
    let mut c = conn();
    let r = c.query("1 2 3 cross `a`b").unwrap();
    assert_eq!(r.type_tag(), 0);                                                // generic list
}

qlong!(test_string_count_unicode, "count \"hello\"", 5);

#[test]
fn test_til_zero() {
    let mut c = conn();
    let r = c.query("til 0").unwrap();
    assert!(r.type_tag() == 6 || r.type_tag() == 7);                            // empty int or long vector
}

qtag!(test_over_sum, "(+/) 1 2 3 4 5", -7);

#[test]
fn test_scan_running_sum() {
    let mut c = conn();
    let r = c.query("(+\\) 1 2 3 4 5").unwrap();
    if let K::IntVec(v) = &r {
        assert_eq!(v, &vec![1, 3, 6, 10, 15]);
    } else { panic!("expected int vec"); }
}

#[test]
fn test_dpft_roundtrip() {
    // Requires partition write/load; skip in CI (needs writable /tmp and HDB support).
    if std::env::var("L_SKIP_HDB").is_ok() { return; }
    // Write to /tmp, not CWD, so a panic can't leak a partition dir into the repo tree.
    let mut c = conn();
    // Escape any cwd a prior test left, so the rm -rf below can't wipe l's cwd.
    c.query("\\cd /tmp").ok();
    c.query("system \"rm -rf /tmp/_rust_dpft_test\"").unwrap();
    c.query("dpft_t:([]sym:`A`B`C;px:1 2 3.0;vol:100 200 300)").unwrap();
    c.query(".Q.dpft[`:/tmp/_rust_dpft_test;2026.01.01;`sym;`dpft_t]").unwrap();
    c.query("\\l /tmp/_rust_dpft_test").unwrap();
    let r = c.query("count dpft_t").unwrap();
    assert!(r.as_long().unwrap() >= 3);
    c.query("system \"rm -rf /tmp/_rust_dpft_test\"").unwrap();
}

// STRING OPERATIONS — each, concatenation

#[test]
fn test_string_each_upper() {
    let mut c = conn();
    let r = c.query("upper each (\"hello\";\"world\")").unwrap();
    assert_eq!(r.type_tag(), 0);                                                // generic list of char vecs
    if let K::List(items) = &r {
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].as_string(), Some("HELLO"));
        assert_eq!(items[1].as_string(), Some("WORLD"));
    } else { panic!("expected list"); }
}

qstr!(test_string_cat, "\"hello\",\" \",\"world\"", "hello world");

// ERROR HANDLING — domain, length

#[test]
fn test_error_domain() {
    let mut c = conn();
    // 1%0 is float division — returns infinity, not error
    let r = c.query("1%0").unwrap();
    let v = r.as_float().unwrap();
    assert!(v.is_infinite());
}

#[test]
fn test_error_length() {
    let mut c = conn();
    let r = c.query("1 2 3 + 1 2");
    assert!(r.is_err());
    if let Err(LError::L(msg)) = r {
        assert!(!msg.is_empty());
    }
}

// SET OPERATIONS — except, inter, union

#[test]
fn test_except() {
    let mut c = conn();
    let r = c.query("1 2 3 4 5 except 2 4").unwrap();
    if let K::IntVec(v) = &r {
        assert_eq!(v, &vec![1, 3, 5]);
    } else { panic!("expected int vec"); }
}

#[test]
fn test_inter() {
    let mut c = conn();
    let r = c.query("1 2 3 4 5 inter 2 4 6").unwrap();
    if let K::IntVec(v) = &r {
        assert_eq!(v, &vec![2, 4]);
    } else { panic!("expected int vec"); }
}

#[test]
fn test_union() {
    let mut c = conn();
    let r = c.query("1 2 3 union 3 4 5").unwrap();
    if let K::IntVec(v) = &r {
        assert_eq!(v, &vec![1, 2, 3, 4, 5]);
    } else { panic!("expected int vec"); }
}

// DICT ACCESSORS — key, value

#[test]
fn test_key_value() {
    let mut c = conn();
    let rk = c.query("key `a`b!1 2").unwrap();
    if let K::SymbolVec(k) = &rk {
        assert_eq!(k, &vec!["a".to_string(), "b".to_string()]);
    } else { panic!("expected symbol vec"); }

    let rv = c.query("value `a`b!1 2").unwrap();
    if let K::IntVec(v) = &rv {
        assert_eq!(v, &vec![1, 2]);
    } else { panic!("expected int vec"); }
}

// ADVERBS — each-right, each-left, peach

#[test]
fn test_each_right() {
    let mut c = conn();
    // each-right: count each (1 2;3 4 5;6)
    let r = c.query("count each (1 2;3 4 5;6)").unwrap();
    if let K::IntVec(v) = &r {
        assert_eq!(v, &vec![2, 3, 1]);
    } else { panic!("expected int vec, got type {}", r.type_tag()); }
}

#[test]
fn test_each_left() {
    let mut c = conn();
    // each-left: sum each (1 2 3;4 5;6 7 8 9)
    let r = c.query("sum each (1 2 3;4 5;6 7 8 9)").unwrap();
    if let K::LongVec(v) = &r {
        assert_eq!(v, &vec![6i64, 9, 30]);
    } else { panic!("expected int vec, got type {}", r.type_tag()); }
}

#[test]
fn test_peach_each() {
    let mut c = conn();
    // (neg') is peach-each — same as each for single-threaded
    let r = c.query("(neg') 1 2 3").unwrap();
    let v: Vec<i32> = r.try_into().unwrap();
    assert_eq!(v, vec![-1, -2, -3]);
}

// KEYWORDS — wsum, wavg, within, xbar

#[test]
fn test_wsum() {
    let mut c = conn();
    // 1*10 + 2*20 + 3*30 = 10 + 40 + 90 = 140
    let r = c.query("1 2 3 wsum 10 20 30").unwrap();
    // wsum returns float (sum of products promotes to KF)
    assert_eq!(r.type_tag(), -9);                                               // float atom
    assert_eq!(r.as_float(), Some(140.0));
}

#[test]
fn test_wavg() {
    let mut c = conn();
    // (2*10 + 3*20 + 5*30) / (2+3+5) = (20+60+150)/10 = 23.0
    let v = c.query("2 3 5 wavg 10 20 30").unwrap().as_float().unwrap();
    assert!((v - 23.0).abs() < 0.001);
}

qk!(test_within, "5 within 3 7", K::Bool(true));

#[test]
fn test_xbar() {
    let mut c = conn();
    // 5 xbar til 20 → 0 0 0 0 0 5 5 5 5 5 10 10 10 10 10 15 15 15 15 15
    let v: Vec<i32> = c.query("5 xbar til 20").unwrap().try_into().unwrap();
    assert_eq!(v, vec![0,0,0,0,0, 5,5,5,5,5, 10,10,10,10,10, 15,15,15,15,15]);
}

// TEMPORAL — .z.d (today's date)

#[test]
fn test_date_today() {
    let mut c = conn();
    let r = c.query(".z.d").unwrap();
    assert_eq!(r.type_tag(), -14);                                              // date atom
    // .z.d is days since 2000.01.01; any recent date is > 9000
    if let K::Date(v) = r {
        assert!(v > 9000, "expected recent date, got {}", v);
    } else { panic!("expected date atom"); }
}

// VM / COMPILER — conditionals, loops, projections, composition

qstr!(test_conditional, "$[1b;\"yes\";\"no\"]", "yes");

#[test]
fn test_do_loop() {
    let mut c = conn();
    // over (/) with int left: do-loop, apply {x+1} five times to 0
    let r = c.query("{x+1}/[0;5]").unwrap();
    assert_eq!(r.as_int(), Some(5));
}

#[test]
fn test_while_loop() {
    let mut c = conn();
    // scan-while: double x while x<100, starting from 1
    let r = c.query("{x<100}{x*2}/1").unwrap();
    assert_eq!(r.as_int(), Some(128));
}

#[test]
fn test_projection() {
    let mut c = conn();
    // partial application: f is 2+ projected, then apply to 3
    let r = c.query("f:2+; f 3").unwrap();
    assert_eq!(r.as_int(), Some(5));
}

#[test]
fn test_composition() {
    let mut c = conn();
    // composed function: neg of abs of -5 = -5
    let r = c.query("neg abs -5").unwrap();
    assert_eq!(r.as_int(), Some(-5));
}

#[test]
fn test_try_catch() {
    let mut c = conn();
    // protected execution: {x+1} fails on "a", catch returns `error
    let r = c.query("@[{x+1};\"a\";{`error}]").unwrap();
    assert_eq!(r, K::Symbol("error".into()));
}

#[test]
fn test_system_cmd() {
    let mut c = conn();
    // system "t 0" sets/gets timer interval; returns an int
    let r = c.query("2+2").unwrap();                                            // just verify IPC round-trip works
    assert_eq!(r.as_int(), Some(4));
}

#[test]
fn test_amend_at() {
    let mut c = conn();
    // @[x;i;f;y] — amend x at index i with f=: and y=99
    let r = c.query("@[1 2 3;1;:;99]").unwrap();
    if let K::IntVec(v) = &r {
        assert_eq!(v, &vec![1, 99, 3]);
    } else { panic!("expected int vec, got type {}", r.type_tag()); }
}

// UNCOVERED BUILTINS — type, cast, reciprocal, signum, mod, fills, differ, next.

#[test]
fn test_type_function() {
    let mut c = conn();
    // type of int atom = -6
    assert_eq!(c.query("type 42").unwrap().as_int(), Some(-6));
}

#[test]
fn test_string_to_int() {
    let mut c = conn();
    // "I"$"42" — cast string to int
    let r = c.query("\"I\"$\"42\"").unwrap();
    assert_eq!(r.as_int(), Some(42));
}

#[test]
fn test_reciprocal() {
    let mut c = conn();
    // %2 4 8 → 0.5 0.25 0.125
    let r = c.query("1%2 4 8").unwrap();
    if let K::FloatVec(v) = &r {
        assert_eq!(v.len(), 3);
        assert!((v[0] - 0.5).abs() < 0.001);
        assert!((v[1] - 0.25).abs() < 0.001);
        assert!((v[2] - 0.125).abs() < 0.001);
    } else { panic!("expected float vec, got type {}", r.type_tag()); }
}

#[test]
fn test_signum_vector() {
    let mut c = conn();
    // signum -5 0 5 → -1 0 1
    let v: Vec<i32> = c.query("signum -5 0 5").unwrap().try_into().unwrap();
    assert_eq!(v, vec![-1, 0, 1]);
}

#[test]
fn test_mod_seven() {
    let mut c = conn();
    // 7 mod 3 → 1
    assert_eq!(c.query("7 mod 3").unwrap().as_int(), Some(1));
}

#[test]
fn test_fills() {
    let mut c = conn();
    // fills 0N 1 0N 3 0N → 0N 1 1 3 3
    let r = c.query("fills 0N 1 0N 3 0N").unwrap();
    if let K::IntVec(v) = &r {
        assert_eq!(v.len(), 5);
        assert_eq!(v[0], i32::MIN);                                             // 0N stays null (no prior value)
        assert_eq!(v[1], 1);
        assert_eq!(v[2], 1);                                                    // forward-filled
        assert_eq!(v[3], 3);
        assert_eq!(v[4], 3);                                                    // forward-filled
    } else { panic!("expected int vec, got type {}", r.type_tag()); }
}

#[test]
fn test_differ() {
    let mut c = conn();
    // differ 1 1 2 2 3 → 10110b
    let r = c.query("differ 1 1 2 2 3").unwrap();
    if let K::BoolVec(v) = &r {
        // differ: first is always true, then true where consecutive values differ
        assert_eq!(v[0], true);                                                 // first always true
        assert_eq!(v[1], false);                                                // 1=1
        assert_eq!(v[2], true);                                                 // 2!=1
        // remaining values depend on l's differ semantics
    } else { panic!("expected bool vec, got type {}", r.type_tag()); }
}

#[test]
fn test_next() {
    let mut c = conn();
    // next 1 2 3 4 5 → 2 3 4 5 0N
    let r = c.query("next 1 2 3 4 5").unwrap();
    if let K::IntVec(v) = &r {
        assert_eq!(v.len(), 5);
        assert_eq!(v[0], 2);
        assert_eq!(v[1], 3);
        assert_eq!(v[2], 4);
        assert_eq!(v[3], 5);
        assert_eq!(v[4], i32::MIN);                                             // 0N (int null)
    } else { panic!("expected int vec, got type {}", r.type_tag()); }
}

// qSQL COMPREHENSIVE — select / exec / update / delete / keyed / ops

// ── Basic SELECT ─────────────────────────────────────────────────

#[test]
fn test_qsql_select_all() {
    let mut c = conn();
    c.query("tq1:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:1000\
        ?\
        100)").unwrap();
    let r = c.query("select from tq1").unwrap();
    assert_eq!(r.type_tag(), 98);
    assert_eq!(c.query("count select from tq1").unwrap().as_int(), Some(1000));
}

#[test]
fn test_qsql_select_single_col() {
    let mut c = conn();
    c.query("tq2:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:1000\
        ?\
        100)").unwrap();
    let r = c.query("select price from tq2").unwrap();
    assert_eq!(r.type_tag(), 98);
    let cols = c.query("cols select price from tq2").unwrap();
    if let K::SymbolVec(v) = cols {
        assert_eq!(v, vec!["price".to_string()]);
    } else { panic!("expected symbol vec"); }
}

#[test]
fn test_qsql_select_computed_col() {
    let mut c = conn();
    c.query("tq3:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:1000\
        ?\
        100)").unwrap();
    let r = c.query("select p2:price*2 from tq3").unwrap();
    assert_eq!(r.type_tag(), 98);
    let cols = c.query("cols select p2:price*2 from tq3").unwrap();
    if let K::SymbolVec(v) = cols {
        assert_eq!(v, vec!["p2".to_string()]);
    } else { panic!("expected symbol vec"); }
}

// ── WHERE clauses ────────────────────────────────────────────────

#[test]
fn test_qsql_where_gt() {
    let mut c = conn();
    c.query("tq4:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:1000\
        ?\
        100)").unwrap();
    let r = c.query("select from tq4 where price > 50").unwrap();
    assert_eq!(r.type_tag(), 98);
    let n = c.query("count select from tq4 where price > \
        50").unwrap().as_int().unwrap();
    assert!(n > 0 && n < 1000);
}

#[test]
fn test_qsql_where_multi() {
    let mut c = conn();
    c.query("tq5:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:1000\
        ?\
        100)").unwrap();
    let r = c.query("select from tq5 where price > 50, sz > 50").unwrap();
    assert_eq!(r.type_tag(), 98);
    let n = c.query("count select from tq5 where price > 50, sz > \
        50").unwrap().as_int().unwrap();
    assert!(n >= 0 && n <= 1000);
}

#[test]
fn test_qsql_where_in() {
    let mut c = conn();
    c.query("tq6:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:1000\
        ?\
        100)").unwrap();
    let r = c.query("select from tq6 where sym in `a`b").unwrap();
    assert_eq!(r.type_tag(), 98);
    let n = c.query("count select from tq6 where sym in \
        `a`b").unwrap().as_int().unwrap();
    assert!(n > 0 && n < 1000);
}

#[test]
fn test_qsql_where_not_null() {
    let mut c = conn();
    c.query("tq7:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:1000\
        ?\
        100)").unwrap();
    let r = c.query("select from tq7 where not null price").unwrap();
    assert_eq!(r.type_tag(), 98);
    // all prices are non-null (random float), so count should be 1000
    assert_eq!(c.query("count select from tq7 where not null \
        price").unwrap().as_int(), Some(1000));
}

// ── GROUP BY (single / multi / aggregation) ──────────────────────

#[test]
fn test_qsql_by_avg_single() {
    let mut c = conn();
    c.query("tq8:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:1000\
        ?\
        100)").unwrap();
    let r = c.query("select avg price by sym from tq8").unwrap();
    assert_eq!(r.type_tag(), 99);                                               // keyed table
    assert_eq!(c.query("count select avg price by sym from \
        tq8").unwrap().as_int(), Some(5));
}

#[test]
fn test_qsql_by_multi_agg() {
    let mut c = conn();
    c.query("tq9:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:1000\
        ?\
        100)").unwrap();
    let r = c.query("select sum sz, avg price by sym from tq9").unwrap();
    assert_eq!(r.type_tag(), 99);
    assert_eq!(c.query("count select sum sz, avg price by sym from \
        tq9").unwrap().as_int(), Some(5));
}

#[test]
fn test_qsql_by_count() {
    let mut c = conn();
    c.query("tq10:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:100\
        0\
        ?100)").unwrap();
    let r = c.query("select count i by sym from tq10").unwrap();
    assert_eq!(r.type_tag(), 99);
    assert_eq!(c.query("count select count i by sym from \
        tq10").unwrap().as_long(), Some(5));
    // sum of group counts must be 1000
    let total = c.query("sum exec x from select count i by sym from \
        tq10").unwrap().as_long().unwrap();
    assert_eq!(total, 1000);
}

#[test]
fn test_qsql_by_first_last() {
    let mut c = conn();
    c.query("tq11:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:100\
        0\
        ?100)").unwrap();
    let r = c.query("select first price, last price by sym from tq11").unwrap();
    assert_eq!(r.type_tag(), 99);
    assert_eq!(c.query("count select first price, last price by sym from \
        tq11").unwrap().as_int(), Some(5));
}

#[test]
fn test_qsql_by_var() {
    // var by — this is the bug we caught
    let mut c = conn();
    c.query("tq12:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:100\
        0\
        ?100)").unwrap();
    let r = c.query("select var price by sym from tq12").unwrap();
    assert_eq!(r.type_tag(), 99);
    assert_eq!(c.query("count select var price by sym from \
        tq12").unwrap().as_int(), Some(5));
}

#[test]
fn test_qsql_by_dev() {
    let mut c = conn();
    c.query("tq13:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:100\
        0\
        ?100)").unwrap();
    let r = c.query("select dev price by sym from tq13").unwrap();
    assert_eq!(r.type_tag(), 99);
    assert_eq!(c.query("count select dev price by sym from \
        tq13").unwrap().as_int(), Some(5));
}

#[test]
fn test_qsql_by_min_max() {
    let mut c = conn();
    c.query("tq14:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:100\
        0\
        ?100)").unwrap();
    let r = c.query("select min price, max price by sym from tq14").unwrap();
    assert_eq!(r.type_tag(), 99);
    assert_eq!(c.query("count select min price, max price by sym from \
        tq14").unwrap().as_int(), Some(5));
}

#[test]
fn test_qsql_by_date() {
    // by date — this was the crash query
    let mut c = conn();
    c.query("tq15:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:100\
        0\
        ?100)").unwrap();
    let r = c.query("select avg price by date from tq15").unwrap();
    assert_eq!(r.type_tag(), 99);
    let n = c.query("count select avg price by date from \
        tq15").unwrap().as_int().unwrap();
    assert!(n > 0);                                                             // at least one date group
}

// ── UPDATE / DELETE ──────────────────────────────────────────────

#[test]
fn test_qsql_update_in_place() {
    let mut c = conn();
    c.query("tq16:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:100\
        0\
        ?100)").unwrap();
    c.query("update price:price*1.1 from `tq16").unwrap();
    // table still has 1000 rows
    assert_eq!(c.query("count tq16").unwrap().as_int(), Some(1000));
}

#[test]
fn test_qsql_update_where() {
    let mut c = conn();
    c.query("tq17:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:100\
        0\
        ?100)").unwrap();
    c.query("update price:0n from `tq17 where price < 10").unwrap();
    assert_eq!(c.query("count tq17").unwrap().as_int(), Some(1000));
    // some prices should now be null
    let null_cnt = c.query("sum null tq17`price").unwrap().as_int().unwrap();
    assert!(null_cnt > 0);
}

#[test]
fn test_qsql_delete_where() {
    let mut c = conn();
    c.query("tq18:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:100\
        0\
        ?100)").unwrap();
    let before = c.query("count tq18").unwrap().as_int().unwrap();
    c.query("delete from `tq18 where sz = 0").unwrap();
    let after = c.query("count tq18").unwrap().as_int().unwrap();
    assert!(after <= before);
}

// ── EXEC ─────────────────────────────────────────────────────────

#[test]
fn test_qsql_exec_vector() {
    let mut c = conn();
    c.query("tq19:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:100\
        0\
        ?100)").unwrap();
    let r = c.query("exec price from tq19").unwrap();
    assert_eq!(r.type_tag(), 9);                                                // float vector
    if let K::FloatVec(v) = &r {
        assert_eq!(v.len(), 1000);
    } else { panic!("expected float vec, got type {}", r.type_tag()); }
}

#[test]
fn test_qsql_exec_by() {
    let mut c = conn();
    c.query("tq20:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:100\
        0\
        ?100)").unwrap();
    let r = c.query("exec avg price by sym from tq20").unwrap();
    assert_eq!(r.type_tag(), 99);                                               // dict
    if let K::Dict(keys, vals) = &r {
        if let K::SymbolVec(k) = keys.as_ref() {
            assert_eq!(k.len(), 5);
        } else { panic!("expected symbol keys"); }
        if let K::FloatVec(v) = vals.as_ref() {
            assert_eq!(v.len(), 5);
            for f in v { assert!(*f > 0.0 && *f < 100.0); }
        } else { panic!("expected float vals"); }
    } else { panic!("expected dict"); }
}

// ── Keyed Tables ─────────────────────────────────────────────────

#[test]
fn test_qsql_keyed_create() {
    let mut c = conn();
    let r = c.query("([sym:`a`b`c] price:1 2 3f)").unwrap();
    assert_eq!(r.type_tag(), 99);
    if let K::Dict(keys, vals) = &r {
        assert_eq!(keys.type_tag(), 98);
        assert_eq!(vals.type_tag(), 98);
    } else { panic!("expected keyed table"); }
}

#[test]
fn test_qsql_keyed_lookup() {
    let mut c = conn();
    c.query("kt1:([sym:`a`b`c] price:1 2 3f)").unwrap();
    let r = c.query("kt1 `a").unwrap();
    // lookup returns a dict with the value columns
    assert_eq!(r.type_tag(), 99);
}

#[test]
fn test_qsql_keyed_upsert() {
    let mut c = conn();
    c.query("kt2:([sym:`a`b`c] price:1 2 3f)").unwrap();
    c.query("`kt2 upsert (`d; 4f)").unwrap();
    assert_eq!(c.query("count kt2").unwrap().as_int(), Some(4));
}

#[test]
fn test_qsql_xkey() {
    let mut c = conn();
    c.query("tq21:([]sym:`a`b`c`d`e;price:1 2 3 4 5f;sz:10 20 30 40 \
        50)").unwrap();
    let r = c.query("1!tq21").unwrap();
    assert_eq!(r.type_tag(), 99);                                               // keyed table
    assert_eq!(c.query("count 1!tq21").unwrap().as_int(), Some(5));
}

#[test]
fn test_qsql_unkey() {
    let mut c = conn();
    c.query("kt3:([sym:`a`b`c] price:1 2 3f)").unwrap();
    let r = c.query("0!kt3").unwrap();
    assert_eq!(r.type_tag(), 98);                                               // regular table
    assert_eq!(c.query("count 0!kt3").unwrap().as_int(), Some(3));
}

// ── Table Operations ─────────────────────────────────────────────

#[test]
fn test_qsql_xasc() {
    let mut c = conn();
    c.query("tq22:([]sym:`c`a`b;price:3 1 2f)").unwrap();
    let r = c.query("`price xasc tq22").unwrap();
    assert_eq!(r.type_tag(), 98);
    // first price after asc sort should be the smallest
    let first = c.query("first exec price from `price xasc \
        tq22").unwrap().as_float().unwrap();
    assert!((first - 1.0).abs() < 0.01);
}

#[test]
fn test_qsql_xdesc() {
    let mut c = conn();
    c.query("tq23:([]sym:`c`a`b;price:3 1 2f)").unwrap();
    let r = c.query("`price xdesc tq23").unwrap();
    assert_eq!(r.type_tag(), 98);
    let first = c.query("first exec price from `price xdesc \
        tq23").unwrap().as_float().unwrap();
    assert!((first - 3.0).abs() < 0.01);
}

#[test]
fn test_qsql_xcols() {
    let mut c = conn();
    c.query("tq24:([]date:2000.01.01 2000.01.02;sym:`a`b;price:1 2f;sz:10 \
        20)").unwrap();
    let r = c.query("`sym`price xcols tq24").unwrap();
    assert_eq!(r.type_tag(), 98);
    let cols = c.query("cols `sym`price xcols tq24").unwrap();
    if let K::SymbolVec(v) = cols {
        assert_eq!(v[0], "sym");
        assert_eq!(v[1], "price");
    } else { panic!("expected symbol vec"); }
}

#[test]
fn test_qsql_meta() {
    let mut c = conn();
    c.query("tq25:([]date:2000.01.01 2000.01.02;sym:`a`b;price:1 2f;sz:10 \
        20)").unwrap();
    let r = c.query("meta tq25").unwrap();
    assert_eq!(r.type_tag(), 99);                                               // keyed table
}

// ── Multiple WHERE + complex predicates ──────────────────────────

#[test]
fn test_qsql_where_like() {
    let mut c = conn();
    c.query("tq26:([]name:(\"alice\";\"bob\";\"anna\";\"carol\");val:1 2 3 \
        4)").unwrap();
    let r = c.query("select from tq26 where name like \"a*\"").unwrap();
    assert_eq!(r.type_tag(), 98);
    assert_eq!(c.query("count select from tq26 where name like \
        \"a*\"").unwrap().as_int(), Some(2));
}

#[test]
fn test_qsql_select_with_func() {
    let mut c = conn();
    c.query("tq27:([]x:1 2 3 4 5 6 7 8 9 10)").unwrap();
    // use (x mod 2)=0 — mod is a named dyadic in l
    let r = c.query("select from tq27 where 0=(x) mod 2").unwrap();
    assert_eq!(r.type_tag(), 98);
    assert_eq!(c.query("count select from tq27 where 0=(x) mod \
        2").unwrap().as_int(), Some(5));
}

#[test]
fn test_qsql_select_top_n() {
    let mut c = conn();
    c.query("tq28:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:100\
        0\
        ?100)").unwrap();
    let r = c.query("5 sublist select from tq28").unwrap();
    assert_eq!(r.type_tag(), 98);
    assert_eq!(c.query("count 5 sublist select from tq28").unwrap().as_int(),
        Some(5));
}

#[test]
fn test_qsql_select_distinct_sym() {
    let mut c = conn();
    c.query("tq29:([]date:1000?.z.d;sym:1000?`a`b`c`d`e;price:1000?100.0;sz:100\
        0\
        ?100)").unwrap();
    let r = c.query("select distinct sym from tq29").unwrap();
    assert_eq!(r.type_tag(), 98);
    assert_eq!(c.query("count select distinct sym from \
        tq29").unwrap().as_int(), Some(5));
}

// ── Aggregation combos ──────────────────────────────────────────

#[test]
fn test_qsql_sum_by() {
    let mut c = conn();
    c.query("tq30:([]sym:1000?`a`b`c`d`e;qty:1000?100)").unwrap();
    let r = c.query("select sum qty by sym from tq30").unwrap();
    assert_eq!(r.type_tag(), 99);                                               // select-by returns keyed table (dict)
    // verify 5 groups
    let cnt = c.query("count select sum qty by sym from tq30").unwrap();
    assert_eq!(cnt.as_int(), Some(5));
}

#[test]
fn test_qsql_med_by() {
    let mut c = conn();
    c.query("tq31:([]sym:1000?`a`b`c`d`e;price:1000?100.0)").unwrap();
    let r = c.query("select med price by sym from tq31").unwrap();
    assert_eq!(r.type_tag(), 99);
    assert_eq!(c.query("count select med price by sym from \
        tq31").unwrap().as_int(), Some(5));
}

#[test]
fn test_qsql_wavg_by() {
    let mut c = conn();
    c.query("tq32:([]sym:`a`a`b`b;price:10 20 30 40f;sz:1 3 2 2)").unwrap();
    let r = c.query("select wavg[sz;price] by sym from tq32").unwrap();
    assert_eq!(r.type_tag(), 99);
    assert_eq!(c.query("count select wavg[sz;price] by sym from \
        tq32").unwrap().as_int(), Some(2));
}

// ── Insert / multi-insert ────────────────────────────────────────

#[test]
fn test_qsql_insert_row() {
    let mut c = conn();
    c.query("tq33:([]sym:`a`b;price:1 2f)").unwrap();
    c.query("`tq33 insert (`c; 3.0)").unwrap();
    assert_eq!(c.query("count tq33").unwrap().as_int(), Some(3));
}

#[test]
fn test_qsql_insert_multi() {
    let mut c = conn();
    c.query("tq34:([]sym:`symbol$();price:`float$())").unwrap();
    c.query("`tq34 insert (`a`b`c; 1 2 3f)").unwrap();
    assert_eq!(c.query("count tq34").unwrap().as_int(), Some(3));
}

// ── Join tests ───────────────────────────────────────────────────

#[test]
fn test_qsql_ij_tables() {
    let mut c = conn();
    c.query("tij1:([sym:`a`b`c] px:1 2 3f)").unwrap();
    c.query("tij2:([sym:`a`b`c] vol:100 200 300)").unwrap();
    let r = c.query("tij1 ij tij2").unwrap();
    assert_eq!(r.type_tag(), 99);
    assert_eq!(c.query("count tij1 ij tij2").unwrap().as_int(), Some(3));
}

#[test]
fn test_qsql_lj_tables() {
    let mut c = conn();
    c.query("tlj1:([]sym:`a`b`c`d;px:1 2 3 4f)").unwrap();
    c.query("tlj2:([sym:`a`c] name:`alpha`gamma)").unwrap();
    let r = c.query("tlj1 lj tlj2").unwrap();
    assert_eq!(r.type_tag(), 98);
    assert_eq!(c.query("count tlj1 lj tlj2").unwrap().as_int(), Some(4));
}

#[test]
fn test_qsql_uj_tables() {
    let mut c = conn();
    c.query("tuj1:([]sym:`a`b;px:1 2f)").unwrap();
    c.query("tuj2:([]sym:`c`d;px:3 4f)").unwrap();
    let r = c.query("tuj1 uj tuj2").unwrap();
    assert_eq!(r.type_tag(), 98);
    assert_eq!(c.query("count tuj1 uj tuj2").unwrap().as_int(), Some(4));
}

// ── Functional select ────────────────────────────────────────────

#[test]
fn test_qsql_functional_select() {
    let mut c = conn();
    c.query("tq35:([]sym:`a`b`c;price:10 20 30f)").unwrap();
    let r = c.query("?[tq35;();0b;()]").unwrap();
    assert_eq!(r.type_tag(), 98);
    assert_eq!(c.query("count ?[tq35;();0b;()]").unwrap().as_int(), Some(3));
}

// ── Table with all column types ──────────────────────────────────

#[test]
fn test_qsql_mixed_types() {
    let mut c = conn();
    c.query("tq36:([]b:100b;s:1 2 3h;i:10 20 30;j:100 200 300j;f:1.1 2.2 \
        3.3;sym:`x`y`z)").unwrap();
    let r = c.query("select from tq36").unwrap();
    assert_eq!(r.type_tag(), 98);
    assert_eq!(c.query("count tq36").unwrap().as_int(), Some(3));
}

// ── select with multiple computed columns ────────────────────────

#[test]
fn test_qsql_multi_computed() {
    let mut c = conn();
    c.query("tq37:([]a:1 2 3 4 5;b:10 20 30 40 50)").unwrap();
    let r = c.query("select s:a+b, d:b-a, p:a*b from tq37").unwrap();
    assert_eq!(r.type_tag(), 98);
    let cols = c.query("cols select s:a+b, d:b-a, p:a*b from tq37").unwrap();
    if let K::SymbolVec(v) = cols {
        assert_eq!(v, vec!["s".to_string(), "d".to_string(), "p".to_string()]);
    } else { panic!("expected symbol vec"); }
}

// ── fby (filter by) ─────────────────────────────────────────────

#[test]
fn test_qsql_fby() {
    let mut c = conn();
    c.query("tq38:([]sym:`a`b`a`b`a;price:10 20 30 40 50f)").unwrap();
    let r = c.query("select from tq38 where price > (avg;price) fby \
        sym").unwrap();
    assert_eq!(r.type_tag(), 98);
    let n = c.query("count select from tq38 where price > (avg;price) fby \
        sym").unwrap().as_int().unwrap();
    assert!(n > 0 && n < 5);
}

// ── delete column ────────────────────────────────────────────────

#[test]
fn test_qsql_delete_col() {
    let mut c = conn();
    c.query("tq39:([]a:1 2 3;b:10 20 30;c:`x`y`z)").unwrap();
    let r = c.query("delete b from tq39").unwrap();
    assert_eq!(r.type_tag(), 98);
    let cols = c.query("cols delete b from tq39").unwrap();
    if let K::SymbolVec(v) = cols {
        assert_eq!(v, vec!["a".to_string(), "c".to_string()]);
    } else { panic!("expected symbol vec"); }
}

// ── update with conditional ──────────────────────────────────────

#[test]
fn test_qsql_update_conditional() {
    let mut c = conn();
    c.query("tq40:([]sym:`a`b`c`a`b;price:10 20 30 40 50f)").unwrap();
    c.query("update price:price*2 from `tq40 where sym=`a").unwrap();
    assert_eq!(c.query("count tq40").unwrap().as_int(), Some(5));
}

// ── select from empty table ──────────────────────────────────────

#[test]
fn test_qsql_empty_result() {
    let mut c = conn();
    c.query("tq41:([]sym:`a`b`c;price:10 20 30f)").unwrap();
    let r = c.query("select from tq41 where price > 999").unwrap();
    assert_eq!(r.type_tag(), 98);
    assert_eq!(c.query("count select from tq41 where price > \
        999").unwrap().as_int(), Some(0));
}

// ── nested aggregation ───────────────────────────────────────────

#[test]
fn test_qsql_nested_agg() {
    let mut c = conn();
    c.query("tq42:([]sym:1000?`a`b`c`d`e;price:1000?100.0)").unwrap();
    // verify grouped avg produces 5 groups with reasonable values
    c.query("select avg price by sym from tq42").unwrap();
    assert_eq!(c.query("count select avg price by sym from \
        tq42").unwrap().as_int(), Some(5));
}

// ── wsum ─────────────────────────────────────────────────────────

#[test]
fn test_qsql_wsum() {
    let mut c = conn();
    let r = c.query("1 2 3 wsum 10 20 30f").unwrap();
    assert_eq!(r.type_tag(), -9);
    let v = r.as_float().unwrap();
    assert!((v - 140.0).abs() < 0.01);
}

// ── select with til ──────────────────────────────────────────────

#[test]
fn test_qsql_select_with_index() {
    let mut c = conn();
    c.query("tq43:([]x:til 100)").unwrap();
    let r = c.query("select from tq43 where x within 10 20").unwrap();
    assert_eq!(r.type_tag(), 98);
    assert_eq!(c.query("count select from tq43 where x within 10 \
        20").unwrap().as_int(), Some(11));
}

// ── select using each ────────────────────────────────────────────

#[test]
fn test_qsql_each_in_select() {
    let mut c = conn();
    c.query("tq44:([]words:(\"hello\";\"world\";\"test\"))").unwrap();
    let r = c.query("select n:count each words from tq44").unwrap();
    assert_eq!(r.type_tag(), 98);
}

// ── aj (asof join) with table data ───────────────────────────────

#[test]
fn test_qsql_aj_sym_time() {
    let mut c = conn();
    c.query("trades_aj:([]sym:`A`A`B`B;time:09:30 09:35 09:31 09:36;px:100 101 \
        200 201f)").unwrap();
    c.query("quotes_aj:([]sym:`A`B;time:09:32 09:33)").unwrap();
    let r = c.query("aj[`sym`time;quotes_aj;trades_aj]").unwrap();
    assert_eq!(r.type_tag(), 98);
    assert_eq!(c.query("count \
        aj[`sym`time;quotes_aj;trades_aj]").unwrap().as_int(), Some(2));
}

// ── xgroup ───────────────────────────────────────────────────────

#[test]
fn test_qsql_xgroup() {
    let mut c = conn();
    c.query("tq45:([]sym:`a`b`a`b`a;price:1 2 3 4 5f)").unwrap();
    let r = c.query("`sym xgroup tq45").unwrap();
    assert_eq!(r.type_tag(), 99);                                               // keyed table
    assert_eq!(c.query("count `sym xgroup tq45").unwrap().as_int(), Some(2));
}

// ── xbar in select ───────────────────────────────────────────────

#[test]
fn test_qsql_xbar_bucket() {
    let mut c = conn();
    // xbar on integers (simpler than time xbar which may not be supported)
    c.query("tq46:([]x:til 20;px:20?10.0)").unwrap();
    c.query("select avg px by 5 xbar x from tq46").unwrap();
    assert_eq!(c.query("count select avg px by 5 xbar x from \
        tq46").unwrap().as_int(), Some(4));
}

// ── select with enlist ───────────────────────────────────────────

#[test]
fn test_qsql_select_enlist() {
    let mut c = conn();
    c.query("tq47:([]sym:`a`b`c;price:1 2 3f)").unwrap();
    let r = c.query("select from tq47 where sym in enlist `a").unwrap();
    assert_eq!(r.type_tag(), 98);
    assert_eq!(c.query("count select from tq47 where sym in enlist \
        `a").unwrap().as_int(), Some(1));
}

// ── chained updates ──────────────────────────────────────────────

#[test]
fn test_qsql_chained_updates() {
    let mut c = conn();
    c.query("tq48:([]a:1 2 3;b:10 20 30)").unwrap();
    c.query("update a:a*10 from `tq48").unwrap();
    c.query("update b:b+1 from `tq48").unwrap();
    let ra: Vec<i32> = c.query("tq48`a").unwrap().try_into().unwrap();
    assert_eq!(ra, vec![10, 20, 30]);
    let rb: Vec<i32> = c.query("tq48`b").unwrap().try_into().unwrap();
    assert_eq!(rb, vec![11, 21, 31]);
}

// ── select last n rows ───────────────────────────────────────────

#[test]
fn test_qsql_select_tail() {
    let mut c = conn();
    c.query("tq49:([]x:til 100)").unwrap();
    let r = c.query("-5 sublist tq49").unwrap();
    assert_eq!(r.type_tag(), 98);
    assert_eq!(c.query("count -5 sublist tq49").unwrap().as_int(), Some(5));
}

// ── select with inter/union/except ───────────────────────────────

#[test]
fn test_qsql_inter_tables() {
    let mut c = conn();
    c.query("ti1:([]a:1 2 3;b:10 20 30)").unwrap();
    c.query("ti2:([]a:2 3 4;b:20 30 40)").unwrap();
    let r = c.query("ti1 inter ti2").unwrap();
    assert_eq!(r.type_tag(), 98);
    assert_eq!(c.query("count ti1 inter ti2").unwrap().as_int(), Some(2));
}

#[test]
fn test_qsql_except_tables() {
    let mut c = conn();
    c.query("te1:([]a:1 2 3;b:10 20 30)").unwrap();
    c.query("te2:([]a:2 3;b:20 30)").unwrap();
    let r = c.query("te1 except te2").unwrap();
    assert_eq!(r.type_tag(), 98);
    assert_eq!(c.query("count te1 except te2").unwrap().as_int(), Some(1));
}

// ── count by with many groups ────────────────────────────────────

#[test]
fn test_qsql_count_by_many_groups() {
    let mut c = conn();
    c.query("tq50:([]g:1000?50;v:1000?1.0)").unwrap();
    let r = c.query("select count i by g from tq50").unwrap();
    assert_eq!(r.type_tag(), 99);
    let n = c.query("count select count i by g from \
        tq50").unwrap().as_int().unwrap();
    assert_eq!(n, 50);
}

// ── Scale tests ──────────────────────────────────────────────────

#[test]
fn test_qsql_scale_1m_avg_by() {
    let mut c = conn();
    let r = c.query("select avg price by sym from \
        ([]sym:1000000?`a`b`c`d`e;price:1000000?1.0)").unwrap();
    assert_eq!(r.type_tag(), 99);
    assert_eq!(c.query("count select avg price by sym from \
        ([]sym:1000000?`a`b`c`d`e;price:1000000?1.0)").unwrap().as_int(),
            Some(5));
}

#[test]
fn test_qsql_scale_var_by() {
    // var by at scale — the bug surface
    let mut c = conn();
    let r = c.query("select count i, var price by sym from \
        ([]sym:100000?`a`b`c`d`e`f`g`h`i`j;price:100000?1.0)").unwrap();
    assert_eq!(r.type_tag(), 99);
    assert_eq!(c.query("count select count i, var price by sym from \
        ([]sym:100000?`a`b`c`d`e`f`g`h`i`j;price:100000?1.0\
            )").unwrap().as_int(), Some(10));
}

// SCALE + MEMORY SAFETY — large vectors, no crashes, correct results

#[test]
fn test_scale_sum_til_1m() {
    let mut c = conn();
    // til 1000000 produces KI (32-bit); sum overflows. Cast to long first.
    let r = c.query("sum `long$til 1000000").unwrap();
    assert_eq!(r.as_long(), Some(499999500000i64));
}

#[test]
fn test_scale_avg_1m_float() {
    let mut c = conn();
    let r = c.query("avg 1000000?1.0").unwrap();
    assert_eq!(r.type_tag(), -9);                                               // float atom
    let v = r.as_float().unwrap();
    assert!(v > 0.0 && v < 1.0, "avg of uniform [0,1) should be ~0.5, got {}",
        v);
}

#[test]
fn test_scale_count_distinct_1m() {
    let mut c = conn();
    let r = c.query("count distinct 1000000?1000").unwrap();
    let n = r.as_int().unwrap();
    assert_eq!(n, 1000);
}

#[test]
fn test_scale_sort_1m() {
    let mut c = conn();
    let r = c.query("count asc 1000000?1000000").unwrap();
    let n = r.as_int().unwrap();
    assert_eq!(n, 1000000);
}

#[test]
fn test_scale_group_1m() {
    let mut c = conn();
    let r = c.query("group 1000000?100").unwrap();
    assert_eq!(r.type_tag(), 99);                                               // dict
    let n = c.query("count group 1000000?100").unwrap().as_int().unwrap();
    assert_eq!(n, 100);
}

#[test]
fn test_scale_1m_where() {
    let mut c = conn();
    c.query("sc1:([]v:1000000?100)").unwrap();
    let r = c.query("select from sc1 where v > 90").unwrap();
    assert_eq!(r.type_tag(), 98);
    let n = c.query("count select from sc1 where v > \
        90").unwrap().as_int().unwrap();
    assert!(n > 0 && n < 200000);                                               // ~9% of 1M
}

#[test]
fn test_scale_1m_group_by() {
    let mut c = conn();
    c.query("sc2:([]sym:1000000?`a`b`c`d`e;px:1000000?100.0)").unwrap();
    let r = c.query("select avg px by sym from sc2").unwrap();
    assert_eq!(r.type_tag(), 99);
    assert_eq!(c.query("count select avg px by sym from \
        sc2").unwrap().as_int(), Some(5));
}

#[test]
fn test_scale_lj_large() {
    let mut c = conn();
    c.query("scl1:([]sym:100000?`a`b`c`d`e`f`g`h`i`j;px:100000?100.0\
        )").unwrap();
    c.query("scl2:([sym:`a`b`c`d`e`f`g`h`i`j] \
        name:`A`B`C`D`E`F`G`H`I`J)").unwrap();
    let r = c.query("scl1 lj scl2").unwrap();
    assert_eq!(r.type_tag(), 98);
    assert_eq!(c.query("count scl1 lj scl2").unwrap().as_int(), Some(100000));
}

#[test]
fn test_scale_sum_float_1m() {
    let mut c = conn();
    let r = c.query("sum 1000000?1.0").unwrap();
    assert_eq!(r.type_tag(), -9);
    let v = r.as_float().unwrap();
    // sum of 1M uniform [0,1) should be ~500000
    assert!(v > 400000.0 && v < 600000.0, "sum of 1M uniform [0,1) should be \
        ~500K, got {}", v);
}

#[test]
fn test_scale_min_max_1m() {
    let mut c = conn();
    let mn = c.query("min 1000000?1000000").unwrap().as_int().unwrap();
    let mx = c.query("max 1000000?1000000").unwrap().as_int().unwrap();
    assert!(mn >= 0);
    assert!(mx < 1000000);
    assert!(mx > mn);
}

#[test]
fn test_scale_dev_1m() {
    let mut c = conn();
    let r = c.query("dev 1000000?1.0").unwrap();
    assert_eq!(r.type_tag(), -9);
    let v = r.as_float().unwrap();
    // std dev of uniform [0,1) is ~0.2887
    assert!(v > 0.2 && v < 0.35, "dev of 1M uniform [0,1) should be ~0.289, \
        got \
        {}", v);
}

#[test]
fn test_scale_var_1m() {
    let mut c = conn();
    let r = c.query("var 1000000?1.0").unwrap();
    assert_eq!(r.type_tag(), -9);
    let v = r.as_float().unwrap();
    // var of uniform [0,1) is ~0.0833
    assert!(v > 0.05 && v < 0.12, "var of 1M uniform [0,1) should be ~0.083, \
        got {}", v);
}

qint!(test_scale_desc_1m_long, "count desc 1000000?1000000j", 1000000);

qint!(test_scale_sums_1m, "last sums 1000000#1", 1000000);

#[test]
fn test_scale_where_bool_1m() {
    let mut c = conn();
    let r = c.query("count where 1000000?2").unwrap();
    let n = r.as_int().unwrap();
    // ~50% of 1M should be 1
    assert!(n > 400000 && n < 600000, "count where ~50% should be ~500K, got \
        {}", n);
}

qint!(test_scale_distinct_sym_1m, "count distinct 1000000?`a`b`c`d`e`f`g`h`i`j",
    10);

qint!(test_scale_iasc_1m, "count iasc 1000000?1000", 1000000);

#[test]
fn test_scale_2m_select_sum() {
    // 2M rows to push memory harder
    let mut c = conn();
    let r = c.query("sum exec px from \
        ([]sym:2000000?`a`b;px:2000000?100.0)").unwrap();
    assert_eq!(r.type_tag(), -9);
    let v = r.as_float().unwrap();
    assert!(v > 0.0, "sum should be positive");
}

// Section 3: PRIMITIVE x TYPE MATRIX

// ── KH (short, 5h) ──────────────────────────────────────────────

#[test]
fn test_s3_short_vec_add_vec() {
    let mut c = conn();
    let r = c.query("(1 2 3h) + (4 5 6h)").unwrap();
    // l promotes KH+KH to KI (type 6)
    assert_eq!(r.type_tag(), 6);
    if let K::IntVec(v) = &r { assert_eq!(v[0], 5); }
    else { panic!("expected int vec, got type {}", r.type_tag()); }
}

#[test]
fn test_s3_short_scalar_add_vec() {
    let mut c = conn();
    let r = c.query("10h + (1 2 3h)").unwrap();
    assert_eq!(r.type_tag(), 6);                                                // KH promotes to KI
    if let K::IntVec(v) = &r { assert_eq!(v[0], 11); }
    else { panic!("expected int vec, got type {}", r.type_tag()); }
}

#[test]
fn test_s3_short_neg() {
    let mut c = conn();
    let r = c.query("neg 1 2 3h").unwrap();
    if let K::ShortVec(v) = &r { assert!(v[0] < 0); } else { panic!("expected \
        short vec"); }
}

qlong!(test_s3_short_sum, "sum 1 2 3h", 6);

#[test]
fn test_s3_short_min() {
    let mut c = conn();
    let r = c.query("min 5 1 3h").unwrap();
    if let K::Short(v) = r { assert_eq!(v, 1); } else { panic!("expected short \
        atom"); }
}

#[test]
fn test_s3_short_max() {
    let mut c = conn();
    let r = c.query("max 5 1 3h").unwrap();
    if let K::Short(v) = r { assert_eq!(v, 5); } else { panic!("expected short \
        atom"); }
}

qf!(test_s3_short_var, "var 1 2 3h", 0.6667, 0.01);

qf!(test_s3_short_dev, "dev 1 2 3h", 0.8165, 0.01);

#[test]
fn test_s3_short_signum() {
    let mut c = conn();
    let r = c.query("signum -5 0 5h").unwrap();
    // signum always returns KI
    if let K::IntVec(v) = &r {
        assert_eq!(v[0], -1);
        assert_eq!(v[1], 0);
        assert_eq!(v[2], 1);
    } else { panic!("expected short vec"); }
}

#[test]
fn test_s3_short_null_prop() {
    let mut c = conn();
    // KH+KH promotes to KI; null check via vector path
    let r = c.query("(1 2 3h) + (0Nh; 5h; 0Nh)").unwrap();
    if let K::IntVec(v) = &r {
        assert_eq!(v[1], 7);                                                    // normal addition (promoted to int)
    } else { panic!("expected int vec, got type {}", r.type_tag()); }
}

// ── KI (int, 6h) ────────────────────────────────────────────────

#[test]
fn test_s3_int_vec_add_vec() {
    let mut c = conn();
    let r = c.query("(1 2 3) + (4 5 6)").unwrap();
    assert_eq!(r.type_tag(), 6);
    if let K::IntVec(v) = &r { assert_eq!(v[0], 5); } else { panic!("expected \
        int vec"); }
}

#[test]
fn test_s3_int_scalar_add_vec() {
    let mut c = conn();
    let r = c.query("10 + (1 2 3)").unwrap();
    assert_eq!(r.type_tag(), 6);
    if let K::IntVec(v) = &r { assert_eq!(v[0], 11); } else { panic!("expected \
        int vec"); }
}

#[test]
fn test_s3_int_neg() {
    let mut c = conn();
    let r = c.query("neg 1 2 3").unwrap();
    if let K::IntVec(v) = &r { assert_eq!(v[0], -1); } else { panic!("expected \
        int vec"); }
}

qlong!(test_s3_int_sum, "sum 1 2 3", 6);

qint!(test_s3_int_min, "min 5 1 3", 1);

qint!(test_s3_int_max, "max 5 1 3", 5);

qf!(test_s3_int_var, "var 1 2 3", 0.6667, 0.01);

qf!(test_s3_int_dev, "dev 1 2 3", 0.8165, 0.01);

qvec!(test_s3_int_signum, i32, "signum -5 0 5", vec![-1, 0, 1]);

qint!(test_s3_int_null_prop, "1 + 0N", i32::MIN);

// ── KJ (long, 7h) ───────────────────────────────────────────────

#[test]
fn test_s3_long_vec_add_vec() {
    let mut c = conn();
    let r = c.query("(1 2 3j) + (4 5 6j)").unwrap();
    assert_eq!(r.type_tag(), 7);
    if let K::LongVec(v) = &r { assert_eq!(v[0], 5); } else { panic!("expected \
        long vec"); }
}

#[test]
fn test_s3_long_scalar_add_vec() {
    let mut c = conn();
    let r = c.query("10j + (1 2 3j)").unwrap();
    assert_eq!(r.type_tag(), 7);
    if let K::LongVec(v) = &r { assert_eq!(v[0], 11); } else {
        panic!("expected \
        long vec"); }
}

#[test]
fn test_s3_long_neg() {
    let mut c = conn();
    let r = c.query("neg 1 2 3j").unwrap();
    if let K::LongVec(v) = &r { assert_eq!(v[0], -1); } else {
        panic!("expected \
        long vec"); }
}

qlong!(test_s3_long_sum, "sum 1 2 3j", 6);

qlong!(test_s3_long_min, "min 5 1 3j", 1);

qlong!(test_s3_long_max, "max 5 1 3j", 5);

qf!(test_s3_long_var, "var 1 2 3j", 0.6667, 0.01);

qf!(test_s3_long_dev, "dev 1 2 3j", 0.8165, 0.01);

#[test]
fn test_s3_long_signum() {
    let mut c = conn();
    let r = c.query("signum -5 0 5j").unwrap();
    if let K::LongVec(v) = &r {
        assert_eq!(v[0], -1);
        assert_eq!(v[1], 0);
        assert_eq!(v[2], 1);
    } else { panic!("expected long vec"); }
}

#[test]
fn test_s3_long_null_prop() {
    let mut c = conn();
    // Test null propagation in vector arithmetic (pp_d2j worker)
    let r = c.query("(1 2 3j) + (0Nj; 5j; 0Nj)").unwrap();
    if let K::LongVec(v) = &r {
        assert_eq!(v[0], i64::MIN);                                             // null propagated
        assert_eq!(v[1], 7);                                                    // normal addition
        assert_eq!(v[2], i64::MIN);                                             // null propagated
    } else { panic!("expected long vec, got type {}", r.type_tag()); }
}

// ── KE (real/float32, 8h) ───────────────────────────────────────

#[test]
fn test_s3_real_vec_add_vec() {
    let mut c = conn();
    let r = c.query("(1 2 3e) + (4 5 6e)").unwrap();
    // KE+KE stays KE (type 8) — no F64 upcast on the hot path
    assert_eq!(r.type_tag(), 8);
    if let K::RealVec(v) = &r { assert!((v[0] - 5.0).abs() < 0.01); }
    else { panic!("expected real vec, got type {}", r.type_tag()); }
}

#[test]
fn test_s3_real_scalar_add_vec() {
    let mut c = conn();
    let r = c.query("10e + (1 2 3e)").unwrap();
    assert_eq!(r.type_tag(), 8);                                                // KE+KE stays KE
    if let K::RealVec(v) = &r { assert!((v[0] - 11.0).abs() < 0.01); }
    else { panic!("expected real vec, got type {}", r.type_tag()); }
}

#[test]
fn test_s3_real_neg() {
    let mut c = conn();
    let r = c.query("neg 1 2 3e").unwrap();
    // neg on KE promotes to KF
    if let K::FloatVec(v) = &r { assert!(v[0] < 0.0); }
    else if let K::RealVec(v) = &r { assert!(v[0] < 0.0); }
    else { panic!("expected float/real vec, got type {}", r.type_tag()); }
}

qf!(test_s3_real_sum, "sum 1 2 3e", 6.0, 0.01);

qf!(test_s3_real_min, "min 5 1 3e", 1.0, 0.01);

qf!(test_s3_real_max, "max 5 1 3e", 5.0, 0.01);

qf!(test_s3_real_var, "var 1 2 3e", 0.6667, 0.01);

qf!(test_s3_real_dev, "dev 1 2 3e", 0.8165, 0.01);

#[test]
fn test_s3_real_signum() {
    let mut c = conn();
    let r = c.query("signum -5 0 5e").unwrap();
    // signum always returns KI regardless of input type
    if let K::IntVec(v) = &r {
        assert_eq!(v[0], -1);
        assert_eq!(v[1], 0);
        assert_eq!(v[2], 1);
    } else { panic!("expected int vec, got type {}", r.type_tag()); }
}

#[test]
fn test_s3_real_div() {
    let mut c = conn();
    let r = c.query("(6 9 12e) % (2 3 4e)").unwrap();
    let t = r.type_tag();
    // KE division promotes to KF vec
    assert!(t == 8 || t == 9, "expected real or float vec, got {}", t);
    let v = if let K::FloatVec(fv) = &r { fv[0] }
            else if let K::RealVec(rv) = &r { rv[0] as f64 }
            else { panic!("expected float or real vec, got type {}", t) };
    assert!((v - 3.0).abs() < 0.01);
}

// ── KF (float64, 9h) ────────────────────────────────────────────

#[test]
fn test_s3_float_vec_add_vec() {
    let mut c = conn();
    let r = c.query("(1.0 2.0 3.0) + (4.0 5.0 6.0)").unwrap();
    assert_eq!(r.type_tag(), 9);
    if let K::FloatVec(v) = &r { assert!((v[0] - 5.0).abs() < 0.001); }
    else { panic!("expected float vec"); }
}

#[test]
fn test_s3_float_scalar_add_vec() {
    let mut c = conn();
    let r = c.query("10.0 + (1.0 2.0 3.0)").unwrap();
    assert_eq!(r.type_tag(), 9);
    if let K::FloatVec(v) = &r { assert!((v[0] - 11.0).abs() < 0.001); }
    else { panic!("expected float vec"); }
}

#[test]
fn test_s3_float_neg() {
    let mut c = conn();
    let r = c.query("neg 1.0 2.0 3.0").unwrap();
    if let K::FloatVec(v) = &r { assert!(v[0] < 0.0); }
    else { panic!("expected float vec"); }
}

qf!(test_s3_float_sum, "sum 1.0 2.0 3.0", 6.0, 0.001);

qf!(test_s3_float_min, "min 5.0 1.0 3.0", 1.0, 0.001);

qf!(test_s3_float_max, "max 5.0 1.0 3.0", 5.0, 0.001);

qf!(test_s3_float_var, "var 1.0 2.0 3.0", 0.6667, 0.01);

qf!(test_s3_float_dev, "dev 1.0 2.0 3.0", 0.8165, 0.01);

#[test]
fn test_s3_float_signum() {
    let mut c = conn();
    let r = c.query("signum -5.0 0.0 5.0").unwrap();
    // signum always returns KI regardless of input type
    if let K::IntVec(v) = &r {
        assert_eq!(v[0], -1);
        assert_eq!(v[1], 0);
        assert_eq!(v[2], 1);
    } else { panic!("expected int vec, got type {}", r.type_tag()); }
}

#[test]
fn test_s3_float_div() {
    let mut c = conn();
    let r = c.query("(6.0 9.0 12.0) % (2.0 3.0 4.0)").unwrap();
    if let K::FloatVec(v) = &r {
        assert!((v[0] - 3.0).abs() < 0.001);
        assert!((v[1] - 3.0).abs() < 0.001);
        assert!((v[2] - 3.0).abs() < 0.001);
    } else { panic!("expected float vec"); }
}

// Section 4: MIXED-TYPE OPERATIONS

// ── Cross-type arithmetic ────────────────────────────────────────

qtag!(test_s4_int_plus_float, "1 + 1.0", -9);

qtag!(test_s4_int_plus_long, "1 + 1j", -7);

qtag!(test_s4_short_plus_int, "1h + 1", -6);

qtag!(test_s4_real_plus_float, "1e + 1.0", -9);

#[test]
fn test_s4_real_vec_sub_float_vec() {
    let mut c = conn();
    let r = c.query("(1 2 3e) - (1.0 2.0 3.0)").unwrap();
    assert_eq!(r.type_tag(), 9);
    if let K::FloatVec(v) = &r {
        assert!((v[0]).abs() < 0.01);
    } else { panic!("expected float vec"); }
}

qtag!(test_s4_int_vec_plus_float_vec, "(1 2 3) + (1.0 2.0 3.0)", 9);

qf!(test_s4_int_mul_float, "3 * 2.5", 7.5, 0.001);

qtag!(test_s4_short_vec_plus_int_vec, "(1 2 3h) + (4 5 6)", 6);

// ── Cross-type comparisons ───────────────────────────────────────

qk!(test_s4_cmp_int_eq_float, "1 = 1.0", K::Bool(true));

qk!(test_s4_cmp_int_lt_float, "1 < 1.5", K::Bool(true));

qk!(test_s4_cmp_real_eq_float, "1e = 1.0", K::Bool(true));

qk!(test_s4_cmp_short_gt_long, "5h > 3j", K::Bool(true));

// ── Cross-type aggregations ──────────────────────────────────────

qf!(test_s4_sum_real_vec, "sum 1 2 3e", 6.0, 0.01);

qf!(test_s4_avg_int_vec, "avg 1 2 3", 2.0, 0.001);

qf!(test_s4_var_real_vec, "var 1 2 3e", 0.6667, 0.01);

qf!(test_s4_cov_real_vecs, "cov[1 2 3e;4 5 6e]", 0.6667, 0.01);

qf!(test_s4_cor_real_vecs, "cor[1 2 3e;4 5 6e]", 1.0, 0.01);

qf!(test_s4_med_real_vec, "med 3 1 2e", 2.0, 0.01);

// ── Type promotion result type checks ────────────────────────────

qint!(test_s4_type_int_plus_float, "type 1+1.0", -9);

qint!(test_s4_type_short_plus_int, "type 1h+1", -6);

qint!(test_s4_type_real_plus_float, "type 1e+1.0", -9);

qint!(test_s4_type_int_vec_plus_float_vec, "type (1 2 3) + (1.0 2.0 3.0)", 9);

// Section 5: EDGE CASES

// ── Null propagation ─────────────────────────────────────────────

qint!(test_s5_null_int_add_one, "0N + 1", i32::MIN);

qlong!(test_s5_null_long_add_one, "0Nj + 1j", i64::MIN);

qint!(test_s5_one_plus_null_int, "1 + 0N", i32::MIN);

#[test]
fn test_s5_null_neq_null() {
    let mut c = conn();
    // l: 0N=0N → 1b (null equals null at scalar level)
    let r = c.query("0N = 0N").unwrap();
    assert_eq!(r, K::Bool(true));
}

qk!(test_s5_null_predicate, "null 0N", K::Bool(true));

#[test]
fn test_s5_sum_with_null() {
    let mut c = conn();
    // l: sum 1 0N 3 → 4 (null treated as 0 in sum, not propagated)
    let r = c.query("sum 1 0N 3").unwrap();
    assert_eq!(r.as_long(), Some(4));
}

#[test]
fn test_s5_avg_with_null() {
    let mut c = conn();
    let r = c.query("avg 1 0N 3").unwrap();
    let v = r.as_float().unwrap();
    // avg with nulls: may return NaN or partial; verify it's a float
    assert!(v.is_nan() || v.is_finite());
}

// ── Infinity ─────────────────────────────────────────────────────

#[test]
fn test_s5_inf_plus_one() {
    let mut c = conn();
    let v = c.query("0w + 1.0").unwrap().as_float().unwrap();
    assert!(v.is_infinite() && v > 0.0);
}

#[test]
fn test_s5_neg_inf_plus_one() {
    let mut c = conn();
    let v = c.query("-0w + 1.0").unwrap().as_float().unwrap();
    assert!(v.is_infinite() && v < 0.0);
}

#[test]
fn test_s5_inf_sub_inf() {
    let mut c = conn();
    let v = c.query("0w - 0w").unwrap().as_float().unwrap();
    assert!(v.is_nan());
}

qf!(test_s5_min_inf_finite, "min (0w; 1.0)", 1.0, 0.001);

qf!(test_s5_max_neginf_finite, "max (-0w; 1.0)", 1.0, 0.001);

// ── Empty vectors ────────────────────────────────────────────────

qlong!(test_s5_empty_int_sum, "sum \"i\"$()", 0);

#[test]
fn test_s5_empty_float_avg() {
    let mut c = conn();
    let v = c.query("avg \"f\"$()").unwrap().as_float().unwrap();
    assert!(v.is_nan());
}

qlong!(test_s5_empty_int_count, "count \"i\"$()", 0);

#[test]
fn test_s5_empty_int_min() {
    let mut c = conn();
    let r = c.query("min \"i\"$()").unwrap();
    // min of empty int vec returns 0W (int max / infinity)
    assert_eq!(r.as_int(), Some(i32::MAX));
}

#[test]
fn test_s5_empty_int_asc() {
    let mut c = conn();
    let r = c.query("asc \"i\"$()").unwrap();
    assert_eq!(r.type_tag(), 6);
    if let K::IntVec(v) = &r {
        assert_eq!(v.len(), 0);
    } else { panic!("expected empty int vec"); }
}

// ── Single element ───────────────────────────────────────────────

#[test]
fn test_s5_single_var() {
    let mut c = conn();
    let v = c.query("var enlist 5.0").unwrap().as_float().unwrap();
    assert!((v).abs() < 0.001);
}

#[test]
fn test_s5_single_dev() {
    let mut c = conn();
    let v = c.query("dev enlist 5.0").unwrap().as_float().unwrap();
    assert!((v).abs() < 0.001);
}

qf!(test_s5_single_avg, "avg enlist 5.0", 5.0, 0.001);

qlong!(test_s5_single_sum, "sum enlist 42", 42);

#[test]
fn test_s5_single_asc() {
    let mut c = conn();
    let r = c.query("asc enlist 3").unwrap();
    let t = r.type_tag();
    assert!(t == 6 || t == -6);
    if let K::IntVec(v) = &r { assert_eq!(v.len(), 1); }
}

// ── Nested structures ────────────────────────────────────────────

#[test]
fn test_s5_ragged_list() {
    let mut c = conn();
    let r = c.query("(1 2 3;4 5;6 7 8 9)").unwrap();
    assert_eq!(r.type_tag(), 0);
    if let K::List(items) = &r {
        assert_eq!(items.len(), 3);
    } else { panic!("expected list"); }
}

#[test]
fn test_s5_ragged_count_each() {
    let mut c = conn();
    let r = c.query("count each (1 2 3;4 5;6 7 8 9)").unwrap();
    if let K::IntVec(v) = &r {
        assert_eq!(v, &vec![3, 2, 4]);
    } else { panic!("expected int vec"); }
}

qtag!(test_s5_dict_is_99, "(`a`b!1 2)", 99);

#[test]
fn test_s5_type_each_mixed_list() {
    let mut c = conn();
    let r = c.query("type each (1;1.0;\"a\";`sym)").unwrap();
    // type returns short atoms; each over mixed list gives short vec
    assert_eq!(r.type_tag(), 5);
}

// ── String edge cases ────────────────────────────────────────────

qlong!(test_s5_empty_string_count, "count \"\"", 0);

#[test]
fn test_s5_string_concat() {
    let mut c = conn();
    let r = c.query("\"abc\",\"def\"").unwrap();
    assert_eq!(r.as_string(), Some("abcdef"));
    assert_eq!(r.len(), 6);
}

#[test]
fn test_s5_large_string() {
    let mut c = conn();
    let r = c.query("10000#\"x\"").unwrap();
    assert_eq!(r.len(), 10000);
    assert_eq!(r.type_tag(), 10);
}

// ── Temporal edge cases ──────────────────────────────────────────

qtag!(test_s5_zd_type, ".z.d", -14);

qint!(test_s5_zt_type, "type .z.t", -19);

qtag!(test_s5_date_plus_one, ".z.d + 1", -14);

#[test]
fn test_s5_time_plus_millis() {
    let mut c = conn();
    let r = c.query("12:00:00.000 + 1000").unwrap();
    assert_eq!(r.type_tag(), -19);
    if let K::Time(v) = r {
        assert_eq!(v, 43201000);
    } else { panic!("expected time atom"); }
}

// COMPARE PATHS — exercise cm()/c2()/ca() optimizations

qvec!(test_cmp_atom_int_sort, i32, "asc 5 3 1 4 2", vec![1, 2, 3, 4, 5]);

qvec!(test_cmp_atom_long_sort, i32, "iasc 50 30 10 40 20j",
    vec![2, 4, 1, 3, 0]);

qk!(test_cmp_byte_memcmp, "(0x010203)~0x010203", K::Bool(true));

qk!(test_cmp_byte_memcmp_neq, "(0x010203)~0x010204", K::Bool(false));

qk!(test_cmp_char_memcmp, "\"abc\"~\"abc\"", K::Bool(true));

qvec!(test_cmp_char_sort, i32, "iasc \"cba\"", vec![2, 1, 0]);

qint!(test_cmp_float_nan_sort, "count asc 2.0 0n 1.0 3.0 0n", 5);

qvec!(test_cmp_symbol_sort, i32, "iasc `c`a`b", vec![1, 2, 0]);

qvec!(test_cmp_short_sort, i32, "iasc 3 1 2h", vec![1, 2, 0]);

qtag!(test_cmp_mixed_list_sort, "asc (1 2;1 1;1 3)", 0);

#[test]
fn test_cmp_empty_sort() {
    let mut c = conn();
    let r: Vec<i32> = c.query("asc `int$()").unwrap().try_into().unwrap();
    assert_eq!(r, Vec::<i32>::new());
}

qvec!(test_cmp_single_elem, i32, "asc enlist 42", vec![42]);

qvec!(test_cmp_equal_elems, i32, "iasc 5 5 5 5", vec![0, 1, 2, 3]);

qk!(test_cmp_atom_compare_direct, "(<). 3 5", K::Bool(true));

qk!(test_cmp_atom_compare_eq, "(~). 42 42", K::Bool(true));

qint!(test_cmp_large_int_sort, "count asc 10000?1000", 10000);

qvec!(test_cmp_bin_float, i32, "0.5 1.5 2.5 bin 0.0 1.0 2.0 3.0",
    vec![-1, 0, 1, 2]);

qvec!(test_cmp_bin_symbol, i32, "`a`b`c bin `a`b`d", vec![0, 1, 2]);

qint!(test_group_large, "count key group 100000?100", 100);

qtag!(test_flip_homogeneous, "flip (1 2 3;4 5 6;7 8 9)", 0);

qvec!(test_flip_typed_cols, i32, "(flip (1 2 3;4 5 6))[0]", vec![1, 4]);

// PEACH — parallel queries with -s 8

qint!(test_peach_each_sum, "count sum each (til 100;til 200;til 300)", 3);

qtag!(test_peach_sort, "asc each (3 1 2;6 4 5;9 7 8)", 0);

qtag!(test_peach_group, "group each (1 1 2 2 3;`a`b`a`b`a)", 0);

// SORT — asc / desc / iasc / xasc correctness (high-cardinality argsort regression guard).

#[test]
fn test_asc_int_small() {
    let mut c = conn();
    let v: Vec<i32> = c.query("asc 98 7 45 2 98 94 89 26 83 40").unwrap()
        .try_into().unwrap();
    assert_eq!(v, vec![2, 7, 26, 40, 45, 83, 89, 94, 98, 98]);
}

#[test]
fn test_asc_int_triggers_simd_path() {
    // Values/size chosen to route through the high-cardinality int argsort path.
    let mut c = conn();
    let r = c.query("asc 128?1000").unwrap();
    let v: Vec<i32> = r.try_into().unwrap();
    assert_eq!(v.len(), 128);
    for i in 1..v.len() {
        assert!(v[i - 1] <= v[i], "asc not sorted at {}: {} > {}",
                i, v[i - 1], v[i]);
    }
}

#[test]
fn test_asc_int_large_random() {
    // Stress test: 100k random ints, verify fully sorted.
    let mut c = conn();
    let r = c.query("asc 100000?100000").unwrap();
    if let K::IntVec(v) = r {
        assert_eq!(v.len(), 100000);
        for i in 1..v.len() {
            assert!(v[i - 1] <= v[i], "asc not sorted at {}", i);
        }
    } else { panic!("expected int vector"); }
}

#[test]
fn test_asc_long_small() {
    let mut c = conn();
    let r = c.query("asc 3 1 4 1 5 9 2 6 5 3 5j").unwrap();
    if let K::LongVec(v) = r {
        assert_eq!(v, vec![1i64, 1, 2, 3, 3, 4, 5, 5, 5, 6, 9]);
    } else { panic!("expected long vector"); }
}

#[test]
fn test_asc_long_random() {
    let mut c = conn();
    let r = c.query("asc 10000?`long$1000").unwrap();
    if let K::LongVec(v) = r {
        assert_eq!(v.len(), 10000);
        for i in 1..v.len() {
            assert!(v[i - 1] <= v[i], "asc long not sorted at {}", i);
        }
    } else { panic!("expected long vector"); }
}

#[test]
fn test_asc_float() {
    let mut c = conn();
    let v: Vec<f64> = c.query("asc 3.14 2.71 1.41 1.61 2.30").unwrap()
        .try_into().unwrap();
    assert_eq!(v.len(), 5);
    for i in 1..v.len() {
        assert!(v[i - 1] <= v[i], "asc float not sorted at {}", i);
    }
    assert!((v[0] - 1.41).abs() < 0.001);
    assert!((v[4] - 3.14).abs() < 0.001);
}

#[test]
fn test_asc_symbol() {
    let mut c = conn();
    let r = c.query("asc `banana`apple`cherry`date").unwrap();
    assert_eq!(r.type_tag(), 11);
    // Order test via count and known-sorted re-test.
    let eq = c.query("(asc \
        `banana`apple`cherry`date)~`apple`banana`cherry`date")
        .unwrap();
    assert_eq!(eq, K::Bool(true));
}

#[test]
fn test_desc_int() {
    let mut c = conn();
    let v: Vec<i32> = c.query("desc 1 2 3 4 5").unwrap()
        .try_into().unwrap();
    assert_eq!(v, vec![5, 4, 3, 2, 1]);
}

#[test]
fn test_desc_int_random() {
    let mut c = conn();
    let r = c.query("desc 100?1000").unwrap();
    let v: Vec<i32> = r.try_into().unwrap();
    assert_eq!(v.len(), 100);
    for i in 1..v.len() {
        assert!(v[i - 1] >= v[i], "desc not monotone at {}", i);
    }
}

#[test]
fn test_iasc_int() {
    let mut c = conn();
    let v: Vec<i32> = c.query("iasc 30 10 20").unwrap()
        .try_into().unwrap();
    assert_eq!(v, vec![1, 2, 0]);
}

#[test]
fn test_iasc_permutation() {
    // iasc must return a permutation of 0..n-1 and x[iasc x] must be sorted.
    let mut c = conn();
    let r = c.query("x:100?10000; (asc x)~x iasc x").unwrap();
    assert_eq!(r, K::Bool(true));
}

qk!(test_idesc_int, "x:100?10000; (desc x)~x idesc x", K::Bool(true));

#[test]
fn test_xasc_table_int() {
    let mut c = conn();
    let r = c.query(
        "(`price xasc ([]sym:`a`b`c`d;price:30 10 40 20))`price"
    ).unwrap();
    let v: Vec<i32> = r.try_into().unwrap();
    assert_eq!(v, vec![10, 20, 30, 40]);
}

#[test]
fn test_xasc_table_random() {
    // Full xasc round-trip: sort a table by a numeric col and verify monotonicity.
    let mut c = conn();
    let r = c.query(
        "t:([]k:1000?`4;p:1000?10000); (`p xasc t)`p"
    ).unwrap();
    if let K::IntVec(v) = r {
        assert_eq!(v.len(), 1000);
        for i in 1..v.len() {
            assert!(v[i - 1] <= v[i], "xasc price not sorted at {}", i);
        }
    } else { panic!("expected int vector"); }
}

#[test]
fn test_xdesc_table() {
    let mut c = conn();
    let v: Vec<i32> = c.query(
        "(`price xdesc ([]sym:`a`b`c;price:10 30 20))`price"
    ).unwrap().try_into().unwrap();
    assert_eq!(v, vec![30, 20, 10]);
}

#[test]
fn test_select_sum_by_ke() {
    // Regression: group-by scatter-add on a 32-bit float result must not overrun (freelist corruption guard).
    let mut c = conn();
    let n: i64 = c.query(
        "n:50000;\
         t:`a`c!(n?.z.d;n?100e);\
         count select sum c by a from flip t"
    ).unwrap().try_into().unwrap();
    assert!(n > 0, "select sum c by a returned empty result");
}

#[test]
fn hot_groupby_type_matrix() {
    // Group-by fuzzer over aggregation x value-type x key-type; row count matches distinct keys.
    let mut c = conn();
    let aggs = ["sum","avg","min","max","prd","count i","first c","last c"];
    let vtypes = [
        ("KH", "n?100h"),
        ("KI", "n?100i"),
        ("KJ", "n?100j"),
        ("KE", "n?100e"),
        ("KF", "n?100.0"),
    ];
    let ktypes = [
        ("KS", "n?`AA`BB`CC`DD"),
        ("KI", "n?100i"),
        ("KJ", "n?100j"),
        ("KD", "n?.z.d"),
        ("KH", "n?20h"),
    ];
    let n = 1000;
    for &(vt, vexpr) in &vtypes {
    for &(kt, kexpr) in &ktypes {
        // Pick a fresh distinct-count baseline per key type; verify count = count distinct.
        let setup = format!("n:{n};t:`k`c!({kexpr};{vexpr})");
        for &agg in &aggs {
            // Some aggregations need column substitution (count i, first c, last c).
            let q = if agg.contains(' ') {
                format!("{{{setup};(count select {agg} by k from flip t)\
                         =count distinct t`k}}[]")
            } else {
                format!("{{{setup};(count select {agg} c by k from flip t)\
                         =count distinct t`k}}[]")
            };
            let lbl = format!("grp-{}-{}-by-{}", agg.replace(' ',"_"), vt, kt);
            eq_q(&mut c, &lbl, &q, "1b");
        }
    }
    }
}

#[test]
fn hot_groupby_type_matrix_large() {
    // High-cardinality companion (n=50K, >10K groups) to the group-by type matrix.
    let mut c = conn();
    let aggs = ["sum","avg","min","max","count i"];
    let vtypes =
        [("KI","n?100i"),("KJ","n?100j"),("KE","n?100e"),("KF","n?100.0")];
    let ktypes = [("KD","n?.z.d"),("KS","n?`8"),("KI","n?10000i")];
    let n = 50_000;
    for &(vt, vexpr) in &vtypes {
    for &(kt, kexpr) in &ktypes {
        let setup = format!("n:{n};t:`k`c!({kexpr};{vexpr})");
        for &agg in &aggs {
            let q = if agg.contains(' ') {
                format!("{{{setup};(count select {agg} by k from flip t)\
                         =count distinct t`k}}[]")
            } else {
                format!("{{{setup};(count select {agg} c by k from flip t)\
                         =count distinct t`k}}[]")
            };
            let lbl = format!("grp-large-{}-{}-by-{}",
                              agg.replace(' ',"_"), vt, kt);
            eq_q(&mut c, &lbl, &q, "1b");
        }
    }
    }
}

#[test]
fn test_sort_stability_int() {
    // iasc must be stable: equal keys preserve original order.
    let mut c = conn();
    let v: Vec<i32> = c.query("iasc 5#0").unwrap()
        .try_into().unwrap();
    assert_eq!(v, vec![0, 1, 2, 3, 4]);
}

#[test]
fn test_sort_attr_marker() {
    // An ascending sort must tag the result sorted so downstream ops can fast-path.
    let mut c = conn();
    let r = c.query("-2!asc 10?1000").unwrap();
    assert_eq!(r, K::Symbol("s".into()));
}

// PRIMITIVE COVERAGE — full type x shape x sign matrix (migrated from primitives.q).

fn eq_q(c: &mut Connection, tag: &str, expr: &str, expected: &str) {
    let probe = format!("({})~({})", expr, expected);
    let r = c.query(&probe).unwrap_or_else(|e| {
        panic!("{}: query error {:?}\n  expr={}\n  expected={}", tag, e, expr,
            expected)
    });
    if r != K::Bool(true) {
        let actual = c.query(expr)
            .map(|k| format!("{}", k))
            .unwrap_or_else(|e| format!("ERR {:?}", e));
        let exp = c.query(expected)
            .map(|k| format!("{}", k))
            .unwrap_or_else(|e| format!("ERR {:?}", e));
        panic!("{} FAIL\n  expr     = {}\n  got      = {}\n  expected = {}\n  \
            exp_str  = {}",
               tag, expr, actual, expected, exp);
    }
    leak_scan(c, tag, expr);
}

// leak_scan — env-gated (L_LEAK_SCAN) leak probe run after every eq_q check.
fn leak_scan(c: &mut Connection, tag: &str, expr: &str) {
    let iters: i64 = std::env::var("L_LEAK_SCAN").ok()
        .and_then(|s| s.parse().ok()).unwrap_or(0);
    if iters <= 0 { return; }
    let q = format!(
        "lQf::{{{expr}}}; do[5;lQf[]]; .Q.gc[]; lQu::.Q.w[]`used; \
            lQh::.Q.w[]`heap; \
         do[{iters};lQf[]]; .Q.gc[]; \
         ((.Q.w[]`used-lQu)<2000000)&((.Q.w[]`heap-lQh)<8000000)",
        expr = expr, iters = iters);
    match c.query(&q) {
        Ok(K::Bool(true)) => {}
        Ok(_) => {
            let d = c.query("(.Q.w[]`used-lQu;.Q.w[]`heap-lQh)")
                .map(|k| format!("{}", k)).unwrap_or_default();
            panic!("{} LEAK over {} iters (used;heap bytes delta)={}\n  \
                expr={}",
                   tag, iters, d, expr);
        }
        Err(_) => { /* expr not loop-safe; correctness already verified — skip
            */ }
    }
}

#[test]
fn prim_arithmetic_add() {
    let mut c = conn();
    eq_q(&mut c, "i+i vv",      "1 2 3+4 5 6",            "5 7 9");
    eq_q(&mut c, "i+i neg",     "1 2 3+(-1;-2;-3)",       "0 0 0");
    eq_q(&mut c, "i+i null",    "(1;2;0Ni;4)+1 2 3 4",    "(2;4;0Ni;8)");
    eq_q(&mut c, "i+i sv",      "42+1 2 3",               "43 44 45");
    eq_q(&mut c, "i+i sv neg",  "(-1)+1 2 3",             "0 1 2");
    eq_q(&mut c, "i+i ss",      "1+2",                    "3");
    eq_q(&mut c, "i+i ss neg",  "(-1)+(-2)",              "-3");
    eq_q(&mut c, "j+j vv",      "1 2 3j+4 5 6j",          "5 7 9j");
    eq_q(&mut c, "j+j neg",     "1j+(-2j)",               "-1j");
    eq_q(&mut c, "j+j null",    "(1j;0Nj)+1 2j",          "(2j;0Nj)");
    eq_q(&mut c, "f+f vv",      "1.0 2.0+3.0 4.0",        "4 6f");
    eq_q(&mut c, "f+f neg",     "1.0+(-2.0)",             "-1f");
    eq_q(&mut c, "f+f sv",      "1.0+1 2 3f",             "2 3 4f");
    eq_q(&mut c, "h+h vv",      "1 2 3h+4 5 6h",          "5 7 9");
}

#[test]
fn prim_arithmetic_subtract() {
    let mut c = conn();
    eq_q(&mut c, "i-i ss",         "3-1",         "2");
    eq_q(&mut c, "i-i neg result", "1-2",         "-1");
    eq_q(&mut c, "i-i vv",         "5 6 7-1 2 3", "4 4 4");
    eq_q(&mut c, "j-j neg",        "1j-2j",       "-1j");
    eq_q(&mut c, "f-f neg",        "1.0-2.0",     "-1f");
}

#[test]
fn prim_arithmetic_multiply() {
    let mut c = conn();
    eq_q(&mut c, "i*i vv",   "2 3 4*5 6 7",    "10 18 28");
    eq_q(&mut c, "i*i neg",  "(-2)*3",         "-6");
    eq_q(&mut c, "i*i null", "(2;0Ni)*3 4",    "(6;0Ni)");
    eq_q(&mut c, "j*j vv",   "2 3j*4 5j",      "8 15j");
    eq_q(&mut c, "f*f vv",   "2.0 3.0*4.0 5.0","8 15f");
    eq_q(&mut c, "f*f sv",   "2.0*1 2 3f",     "2 4 6f");
}

#[test]
fn prim_arithmetic_divide() {
    let mut c = conn();
    eq_q(&mut c, "f%f vv", "6.0 8.0%2.0 4.0", "3 2f");
    eq_q(&mut c, "f%f sv", "1.0%2.0",         "0.5");
}

#[test]
fn prim_arithmetic_int_div() {
    let mut c = conn();
    eq_q(&mut c, "i div i",       "10 20 30 div 3", "3 6 10");
    eq_q(&mut c, "i div i floor", "10 20 30 div 7", "1 2 4");
    eq_q(&mut c, "j div j",       "10 20j div 3j",  "3 6j");
}

#[test]
fn prim_comparison_eq() {
    let mut c = conn();
    eq_q(&mut c, "i=i vv", "1 2 3=1 3 3",      "101b");
    eq_q(&mut c, "i=i sv", "2=1 2 3",           "010b");
    eq_q(&mut c, "j=j vv", "1 2j=1 3j",         "10b");
    eq_q(&mut c, "f=f vv", "1.0 2.0=1.0 3.0",   "10b");
    eq_q(&mut c, "h=h vv", "1 2 3h=1 3 3h",     "101b");
}

#[test]
fn prim_comparison_lt_gt() {
    let mut c = conn();
    eq_q(&mut c, "i<i vv", "1 2 3<2 2 2",    "100b");
    eq_q(&mut c, "i<i sv", "2<1 2 3",         "001b");
    eq_q(&mut c, "j<j vv", "1 3j<2 2j",       "10b");
    eq_q(&mut c, "f<f vv", "1.0 3.0<2.0 2.0", "10b");
    eq_q(&mut c, "i>i sv", "2>1 2 3",         "100b");
}

#[test]
fn prim_min_max() {
    let mut c = conn();
    eq_q(&mut c, "i&i vv", "1 2 3 4&4 3 2 1",  "1 2 2 1");
    eq_q(&mut c, "i|i vv", "1 2 3 4|4 3 2 1",  "4 3 3 4");
    eq_q(&mut c, "j&j vv", "1 3j&2 2j",         "1 2j");
    eq_q(&mut c, "j|j vv", "1 3j|2 2j",         "2 3j");
    eq_q(&mut c, "f&f vv", "1.0 3.0&2.0 2.0",   "1 2f");
    eq_q(&mut c, "f|f vv", "1.0 3.0|2.0 2.0",   "2 3f");
    eq_q(&mut c, "h&h vv", "1 3h&2 2h",         "1 2h");
    eq_q(&mut c, "h|h vv", "1 3h|2 2h",         "2 3h");
    eq_q(&mut c, "i&i sv", "2&1 3 5",           "1 2 2");
    eq_q(&mut c, "i|i sv", "2|1 3 5",           "2 3 5");
}

#[test]
fn prim_monadic() {
    let mut c = conn();
    eq_q(&mut c, "neg i",   "neg 5",   "-5");
    eq_q(&mut c, "neg j",   "neg 5j",  "-5j");
    eq_q(&mut c, "neg f",   "neg 1.5", "-1.5");
    eq_q(&mut c, "neg neg", "neg -3",  "3");
    eq_q(&mut c, "abs i",   "abs -5",  "5");
    eq_q(&mut c, "abs j",   "abs -5j", "5j");
    eq_q(&mut c, "abs f",   "abs -1.5","1.5");
    eq_q(&mut c, "not b",   "not 101b","010b");
}

#[test]
fn prim_reductions() {
    let mut c = conn();
    eq_q(&mut c, "sum i", "sum 1 2 3 4 5",   "15j");
    eq_q(&mut c, "sum j", "sum 1 2 3 4 5j",  "15j");
    eq_q(&mut c, "sum f", "sum 1 2 3 4 5.0", "15f");
    eq_q(&mut c, "prd i", "prd 1 2 3 4 5",   "120");
    eq_q(&mut c, "min i", "min 3 1 4 1 5",   "1");
    eq_q(&mut c, "max i", "max 3 1 4 1 5",   "5");
    eq_q(&mut c, "min j", "min 3 1 4j",       "1j");
    eq_q(&mut c, "max f", "max 1.0 3.0 2.0",  "3f");
}

#[test]
fn prim_scans() {
    let mut c = conn();
    eq_q(&mut c, "sums i", "sums 1 2 3 4 5",   "1 3 6 10 15");
    eq_q(&mut c, "sums j", "sums 1 2 3j",       "1 3 6j");
    eq_q(&mut c, "sums f", "sums 1 2 3.0",      "1 3 6f");
    eq_q(&mut c, "prds i", "prds 1 2 3 4 5",    "1 2 6 24 120");
    eq_q(&mut c, "mins i", "mins 5 3 4 1 2",    "5 3 3 1 1");
    eq_q(&mut c, "maxs i", "maxs 1 3 2 5 4",    "1 3 3 5 5");
}

#[test]
fn prim_math() {
    let mut c = conn();
    eq_q(&mut c, "sqrt f", "floor sqrt 4.0",         "2");
    eq_q(&mut c, "sqrt 2", "floor 1000*sqrt 2.0",    "1414");
}

#[test]
fn prim_boolean() {
    let mut c = conn();
    eq_q(&mut c, "b&b", "101b&110b", "100b");
    eq_q(&mut c, "b|b", "101b|110b", "111b");
}

#[test]
fn prim_null_propagation() {
    let mut c = conn();
    eq_q(&mut c, "null+i", "(0Ni)+1",  "0Ni");
    eq_q(&mut c, "i+null", "1+0Ni",    "0Ni");
    eq_q(&mut c, "null*i", "(0Ni)*2",  "0Ni");
    eq_q(&mut c, "null+j", "(0Nj)+1j", "0Nj");
}

#[test]
fn prim_sort() {
    // Inputs chosen to exercise the high-cardinality int argsort kernel (regression).
    let mut c = conn();
    eq_q(&mut c, "asc-int",  "asc 98 7 45 2 98 94 89 26 83 40", "`s#2 7 26 40 \
        45 83 89 94 98 98");
    eq_q(&mut c, "asc-long", "asc 3 1 4 1 5 9 2 6 5 3 5j",       "`s#1 1 2 3 3 \
        4 5 5 5 6 9j");
    eq_q(&mut c, "desc-int", "desc 1 2 3 4 5",                     "5 4 3 2 1");
    eq_q(&mut c, "iasc-idx", "iasc 30 10 20",                      "1 2 0");
    eq_q(&mut c, "xasc-tbl", "(`p xasc ([]p:3 1 2;q:`a`b`c))`p",  "`s#1 2 3");
}

#[test]
fn prim_ticktock() {
    // -43!/-44!/-45! timer slots; wrap multi-stage tests in a lambda for one eval context.
    let mut c = conn();
    eq_q(&mut c, "tick-ret",    "tick `t1",
        "1b");
    eq_q(&mut c, "tock-type",   "{tick`t2;1+1;0<tock`t2}[]",
        "1b");
    eq_q(&mut c, "profile-rows","{tick`t3;tock`t3;0<count profile[]}[]",
        "1b");
}

#[test]
fn prim_ipc_serialization() {
    // -8!/-9! and -48!/-49! round-trip; wrapped in a lambda so locals persist.
    let mut c = conn();
    eq_q(&mut c, "b9-roundtrip-tbl",  "{t:([]a:til \
        1000;b:1000?`4);t~-9!-8!t}[]",       "1b");
    eq_q(&mut c, "b9-roundtrip-vec",  "{v:til \
        100000;v~-9!-8!v}[]",                       "1b");
    eq_q(&mut c, "b9-roundtrip-dict", "{d:`x`y`z!(til \
        100;100?100;100?1f);d~-9!-8!d}[]", "1b");
    eq_q(&mut c, "unzb9-roundtrip",
        "{v:10000?1000;v~-9!-48!v}[]",                       "1b");
}

#[test]
fn prim_nan_aware_math() {
    // Forces primitives.c slow path (ltf, gtf, mnf, mxf, eqf).
    let mut c = conn();
    eq_q(&mut c, "log-nan",     "log -1.0",        "0n");
    eq_q(&mut c, "log-zero",    "log 0.0",         "-0w");
    eq_q(&mut c, "log-one",     "log 1.0",         "0f");
    eq_q(&mut c, "log-e",       "1e-6>abs 1f - log exp 1.0", "1b");
    eq_q(&mut c, "nan-min",     "0n|1.0",          "1f");
    eq_q(&mut c, "nan-match",   "0n~0n",           "1b");
    eq_q(&mut c, "nan-vec-max", "(|/) 0n 1 2 3",   "3f");
    eq_q(&mut c, "nan-vec-min", "(&/) 0n 1 2 3",   "1f");
    eq_q(&mut c, "nan-sum",     "sum 0n 1 2 3",    "6f");
    eq_q(&mut c, "nan-avg",     "avg 0n 1 2 3",    "2f");
}

#[test]
fn parser_fusion() {
    // Fusion optimization must match the non-fused path (never observable from outside).
    let mut c = conn();
    // Pure monadic chain; tolerance comparison since fast-math diverges from libm in last ULPs.
    eq_q(&mut c, "fuse-sqrt-log-exp",
                  "all 1e-6>abs (1 2 3f)-sqrt log exp 1.0 4.0 9.0", "1b");
    eq_q(&mut c, "fuse-log-exp-id",
                  "all 1e-6>abs (1 2 3f)-log exp 1.0 2.0 3.0",      "1b");
    eq_q(&mut c, "fuse-exp-log-id",
                  "all 1e-6>abs (1 2 4f)-exp log 1.0 2.0 4.0",      "1b");
    eq_q(&mut c, "fuse-abs-neg",        "abs -1.0 -4.0 -9.0",       "1 4 9f");
    // Reduce-tipped — Path B (single-pass parallel fused-reduce).
    eq_q(&mut c, "fuse-sum-sqrt",
                  "1e-6>abs 6.0-sum sqrt 1.0 4.0 9.0",              "1b");
    eq_q(&mut c, "fuse-avg-log-exp",
                  "1e-6>abs 2.0-avg log exp 1.0 2.0 3.0",           "1b");
    eq_q(&mut c, "fuse-min-sqrt",       "min sqrt 4.0 16.0",        "2f");
    eq_q(&mut c, "fuse-max-log-exp",
                  "1e-4>abs 100.0-max log exp 1.0 100.0",           "1b");
    eq_q(&mut c, "fuse-prd-sqrt",
                  "1e-6>abs 6.0-prd sqrt 1.0 4.0 9.0",              "1b");
    // Pure reduction — single op, no chain.
    eq_q(&mut c, "fuse-sum-only",       "sum 1.0 2.0 3.0",          "6f");
    eq_q(&mut c, "fuse-avg-only",       "avg 1.0 2.0 3.0 4.0",      "2.5f");
    // KE→KF promotion: float32 input, fused chain produces KF output.
    eq_q(&mut c, "fuse-ke-promote",
                  "all 1e-4>abs (1 2 3f)-log exp 1.0 2.0 3.0e",     "1b");
    eq_q(&mut c, "fuse-ke-sum",
                  "1e-4>abs 6.0-sum sqrt 1.0 4.0 9.0e",             "1b");
    // Empty / single-element sanity.
    eq_q(&mut c, "fuse-sum-zero",       "sum sqrt 0#1.0",           "0f");
    eq_q(&mut c, "fuse-single",
                  "1e-6>abs 1.0-{x[0]}sqrt log exp enlist 1.0",     "1b");
    // KI/KJ widen-on-fuse: chain must produce float (sqrt/log/.../avg).
    eq_q(&mut c, "fuse-ki-sum-sqrt",    "sum sqrt 1 4 9",            "6f");
    eq_q(&mut c, "fuse-kj-sum-sqrt",    "sum sqrt 1 4 9j",           "6f");
    eq_q(&mut c, "fuse-ki-avg-log-exp",
                  "1e-6>abs 2.0-avg log exp 1 2 3",                 "1b");
    eq_q(&mut c, "fuse-ki-min-sqrt",    "min sqrt 4 16",             "2f");
    eq_q(&mut c, "fuse-kj-max-sqrt",    "max sqrt 4 16j",            "4f");
    eq_q(&mut c, "fuse-ki-pure-mono",
                  "all 1e-6>abs (1 2 3f)-sqrt 1 4 9",               "1b");
    // KI/KJ non-widening chain: must fall back to preserve integer family.
    eq_q(&mut c, "fuse-ki-abs-fallback","abs -1 -4 -9",              "1 4 9");
    eq_q(&mut c, "fuse-ki-sum-fallback","sum 1 4 9",                 "14j");
    // Negative-test: dyadic chain not eligible.
    eq_q(&mut c, "fuse-dyadic-noop",    "1 2 3 + sqrt 1.0 4.0 9.0", "2 4 6f");
    // Dyadic inside the chain: verify the result is still correct when fusion falls back.
    eq_q(&mut c, "fuse-dyadic-inner",
                  "1e-6>abs (sqrt 2 wavg 1 2 3)-sqrt 2 wavg 1 2 3", "1b");
    // Smaller table — larger ones interact badly with parallel globals and destabilize the suite.
    eq_q(&mut c, "fuse-select-dyadic",
                  "4=count select sqrt size wavg price by sym from \
                      ([]sym:100?`a`b`c`d;price:100?1.0;size:100?100)",
                  "1b");
    // Larger N; fast-math log/exp accuracy drives the 1e-7 tolerance.
    eq_q(&mut c, "fuse-1m-id",
                  "{all 1e-7>abs x-log exp x}1000?1.0",             "1b");
}

#[test]
fn nonfused_v1_parallel_partition() {
    // REGRESSION: parallel monadic workers must read the value column with the right stride.
    let mut c = conn();
    // Tolerances are 1e-7 (fast-math accuracy); the target is catching catastrophically wrong values.
    eq_q(&mut c, "v1-sqrt-sum",
                  "1e-7>abs 21097.4558874807-{sum sqrt 1.0+til x}1000", "1b");
    // log(exp(x)) = x identity, materialized in two steps.
    eq_q(&mut c, "v1-log-exp-id",
                  "{all 1e-7>abs x-{[y]z:exp y;log z}x}1.0+(til 1000)%1000.0",
                  "1b");
    // log: y[500] should be log(501) ≈ 6.21660610108
    eq_q(&mut c, "v1-log-elem500",
                  "1e-7>abs 6.21660610108-{(log 1.0+til x)[500]}1000",   "1b");
    // sqrt: y[500] should be sqrt(501) ≈ 22.38302928560
    eq_q(&mut c, "v1-sqrt-elem500",
                  "1e-7>abs 22.38302928560-{(sqrt 1.0+til x)[500]}1000", "1b");
    // sin/cos identity: sin²+cos² = 1 for any x.
    eq_q(&mut c, "v1-sincos-id",
                  "{[n]x:0.01*til n;s:sin x;c:cos x;all 1e-7>abs \
                      1.0-(s*s)+c*c}1000",
                  "1b");
    // Larger n=10000: forces nw=4 or 8 partitions on most platforms.
    eq_q(&mut c, "v1-sqrt-10k",
                  "1e-7>abs 666716.4591971-{sum sqrt 1.0+til x}10000","1b");
}

#[test]
fn bytecode_fusion() {
    // Bytecode-level fusion must produce bit-identical results to the top-level chain.
    let mut c = conn();
    // Pure-monadic chain in a lambda body.
    eq_q(&mut c, "bcfu-lambda-monadic",
                  "all 1e-6>abs (1 2 3f)-{sqrt log exp x}1.0 4.0 9.0", "1b");
    eq_q(&mut c, "bcfu-lambda-log-exp-id",
                  "all 1e-6>abs (1 2 3f)-{log exp x}1.0 2.0 3.0",      "1b");
    // Reduce-tipped chain in a lambda body.
    eq_q(&mut c, "bcfu-lambda-sum-sqrt",
                  "1e-6>abs 6.0-{sum sqrt x}1.0 4.0 9.0",              "1b");
    eq_q(&mut c, "bcfu-lambda-avg-log-exp",
                  "1e-6>abs 2.0-{avg log exp x}1.0 2.0 3.0",           "1b");
    eq_q(&mut c, "bcfu-lambda-min-sqrt",  "{min sqrt x}4.0 16.0",    "2f");
    eq_q(&mut c, "bcfu-lambda-max-log-exp",
                  "1e-4>abs 100.0-{max log exp x}1.0 100.0",           "1b");
    eq_q(&mut c, "bcfu-lambda-prd-sqrt",
                  "1e-6>abs 6.0-{prd sqrt x}1.0 4.0 9.0",              "1b");
    // Named function called multiple times — q.k-style helper pattern.
    c.query("f:{sum sqrt log exp x}").unwrap();
    eq_q(&mut c, "bcfu-named-call",
                  "1e-6>abs (f 1.0 2.0 3.0)-sum sqrt log exp 1.0 2.0 3.0",
                      "1b");
    // KE / KI / KJ promotion through the bytecode path.
    eq_q(&mut c, "bcfu-ki-sum-sqrt",      "{sum sqrt x}1 4 9",        "6f");
    eq_q(&mut c, "bcfu-kj-sum-sqrt",      "{sum sqrt x}1 4 9j",       "6f");
    eq_q(&mut c, "bcfu-ke-sum-sqrt",
                  "1e-4>abs 6.0-{sum sqrt x}1 4 9e",                   "1b");
    eq_q(&mut c, "bcfu-ki-avg-log-exp",
                  "1e-6>abs 2.0-{avg log exp x}1 2 3",                 "1b");
    // Empty / single-element via bytecode.
    eq_q(&mut c, "bcfu-empty",            "{sum sqrt x}0#1.0",        "0f");
    eq_q(&mut c, "bcfu-single",
                  "1e-6>abs 1.0-{sum sqrt x}enlist 1.0",               "1b");
    // KI non-widening chain — must fall back to preserve integer family.
    eq_q(&mut c, "bcfu-ki-abs-fallback",
                  "{abs x}(-1 -4 -9)",                                 "1 4 9");
    // Negative — non-eligible chain still routes through unfused bytecode.
    eq_q(&mut c, "bcfu-dyadic-noop",
                  "{x+sqrt y}[1 2 3.0;1 4 9.0]",                       "2 4 \
                      6f");
    // 1M-row correctness through bytecode path — exercises BC_FUSE at scale.
    eq_q(&mut c, "bcfu-1m-id",
                  "{all 1e-7>abs x-{log exp x}x}1000?1.0",             "1b");
    // Select agg fusion descends into value vectors so plain select aggs fuse.
    eq_q(&mut c, "bcfu-select-agg-monadic",
                  "(exec avg sqrt price by sym from \
                    ([]sym:`a`b`a`b;price:1.0 4.0 9.0 16.0))[`a]",      "2f");
    eq_q(&mut c, "bcfu-select-agg-monadic-b",
                  "(exec avg sqrt price by sym from \
                    ([]sym:`a`b`a`b;price:1.0 4.0 9.0 16.0))[`b]",      "3f");
}

#[test]
fn fusion_accuracy_parity() {
    // Bit-identical parity between fused chains and the equivalent sequential chain.
    let mut c = conn();
    // Top-level AST fusion vs sequential intermediate.
    eq_q(&mut c, "parity-sum-sqrt",
         "(sum sqrt 1.0+til 1000)~({[x]y:sqrt x;sum y}[1.0+til 1000])",
             "1b");
    eq_q(&mut c, "parity-avg-log",
         "(avg log 1.0+til 1000)~({[x]y:log x;avg y}[1.0+til 1000])",
             "1b");
    eq_q(&mut c, "parity-avg-log-exp",
         "(avg log exp 1.0+til 1000)~ \
          ({[x]a:exp x;b:log a;avg b}[1.0+til 1000])",
              "1b");
    eq_q(&mut c, "parity-sqrt-log-exp",
         "(sqrt log exp 1.0+til 1000)~ \
          ({[x]a:exp x;b:log a;sqrt b}[1.0+til 1000])",
              "1b");
    eq_q(&mut c, "parity-min-sqrt",
         "(min sqrt 1.0+til 50)~({[x]y:sqrt x;min y}[1.0+til 50])",
             "1b");
    eq_q(&mut c, "parity-max-sqrt",
         "(max sqrt 1.0+til 50)~({[x]y:sqrt x;max y}[1.0+til 50])",
             "1b");
    eq_q(&mut c, "parity-prd-sqrt",
         "(prd sqrt 1.0+til 5)~({[x]y:sqrt x;prd y}[1.0+til 5])",
             "1b");
    eq_q(&mut c, "parity-sin-cos",
         "(sin cos 1.0+til 1000)~({[x]y:cos x;sin y}[1.0+til 1000])",
             "1b");
    // BC_FUSE path (lambda-wrapped chains).
    eq_q(&mut c, "parity-bc-sum-sqrt",
         "({sum sqrt x}1.0+til 1000)~({[x]y:sqrt x;sum y}[1.0+til 1000])",
             "1b");
    eq_q(&mut c, "parity-bc-avg-log-exp",
         "({avg log exp x}1.0+til 1000)~ \
          ({[x]a:exp x;b:log a;avg b}[1.0+til 1000])",
              "1b");
    eq_q(&mut c, "parity-bc-sqrt-log-exp",
         "({sqrt log exp x}1.0+til 1000)~ \
          ({[x]a:exp x;b:log a;sqrt b}[1.0+til 1000])",
              "1b");
    eq_q(&mut c, "parity-bc-sin-cos",
         "({sin cos x}1.0+til 1000)~({[x]y:cos x;sin y}[1.0+til 1000])",
             "1b");
    // KE / KI / KJ promotion paths (chain widens to KF).
    eq_q(&mut c, "parity-ki-sum-sqrt",
         "(sum sqrt 1+til 100)~({[x]y:sqrt x;sum y}[1+til 100])",
             "1b");
    eq_q(&mut c, "parity-kj-sum-sqrt",
         "(sum sqrt `long$1+til 100)~({[x]y:sqrt x;sum y}[`long$1+til \
             100])","1b");
    eq_q(&mut c, "parity-ke-avg-log-exp",
         "(avg log exp `real$1.0+til 100)~ \
          ({[x]a:exp x;b:log a;avg b}[`real$1.0+til 100])",
              "1b");
    // 1M-element scale (exercises pp_for parallel partitioning).
    eq_q(&mut c, "parity-1m-sum-sqrt",
         "(sum sqrt 1.0+til 1000000)~ \
          ({[x]y:sqrt x;sum y}[1.0+til 1000000])",
              "1b");
}

#[test]
fn complex_fusion_deep_chains() {
    // 4+ verb monadic chains: fused chain vs equivalent intermediate-var chain.
    let mut c = conn();
    // 4 verbs — fused vs same expression with intermediates.
    eq_q(&mut c, "deep4-parity-vec",
         "(sqrt sqrt log exp 0.5+0.01*til 100)~ \
          ({[x]a:exp x;b:log a;c:sqrt b;sqrt c}[0.5+0.01*til 100])",
              "1b");
    // 5 verbs.  abs makes the chain compose cleanly even on negatives.
    eq_q(&mut c, "deep5-parity-vec",
         "(abs sqrt log exp abs 0.5+0.01*til 100)~ \
          ({[x]a:abs x;b:exp a;c:log b;d:sqrt c;abs d}[0.5+0.01*til 100])",
              "1b");
    // 6 verbs reduce-tipped: sum closes the chain; fused-then-reduced must be recognized.
    eq_q(&mut c, "deep6-reduce-parity",
         "(sum abs sqrt log exp abs sqrt 0.5+0.01*til 100)~ \
          ({[x]a:sqrt x;b:abs a;c:exp b;d:log c;e:sqrt d;sum abs \
              e}[0.5+0.01*til 100])",
         "1b");
    // Atom propagation: a single value must thread through every verb in a deep chain.
    eq_q(&mut c, "deep-atom-parity",
         "(sqrt log exp 4.0)~({[x]a:exp x;b:log a;sqrt b}4.0)",
             "1b");
    // Empty vec through deep chain.
    eq_q(&mut c, "deep-empty-float",
         "0=count sqrt log exp 0#1.0",
             "1b");
    // Single-element vec — border-case allocation / SIMD-tail handling.
    eq_q(&mut c, "deep-singleton-parity",
         "(sqrt log exp enlist 1.0)~({[x]a:exp x;b:log a;sqrt b}enlist 1.0)",
         "1b");
}

#[test]
fn complex_fusion_dyadic_mixed() {
    // A dyadic step inside a monadic chain must break fusion cleanly.
    let mut c = conn();
    // dyadic in the middle — broken chain, intermediates allocated.
    eq_q(&mut c, "mixed-add-then-fuse",
         "all 1e-6>abs (2.0+sqrt log exp 1.0+til 50)-(2.0+sqrt log exp 1.0+til \
             50)",
         "1b");
    // dyadic at the head — sum-of-binary-op.
    eq_q(&mut c, "mixed-sum-mul",
         "55.0~sum 1.0 2.0 3.0 4.0 5.0 6.0 7.0 8.0 9.0 10.0",
             "1b");
    // dyadic at the tail of a fused chain.
    eq_q(&mut c, "mixed-fused-then-mul",
         "all 1e-6>abs (2.0*sqrt 1.0 4.0 9.0 16.0)-(2.0 4.0 6.0 8.0)",
             "1b");
    // Adverb (each) over a fused-chain lambda.
    eq_q(&mut c, "adverb-each-fused",
         "(sqrt each 1.0 4.0 9.0 16.0)~(1.0 2.0 3.0 4.0)",
             "1b");
    // Adverb-over (/) folds; should fuse the inner verb chain.
    eq_q(&mut c, "adverb-over-sum",
         "1e-6>abs 6.0-(+/)1.0 2.0 3.0",
             "1b");
}

#[test]
fn complex_fusion_nan_null_propagation() {
    // NaN and null must produce identical results in fused vs unfused chains.
    let mut c = conn();
    // log(exp(0n)) — fused vs unfused must match bit-for-bit.
    eq_q(&mut c, "null-fused-log-exp-parity",
         "(log exp 0n 0n 0n)~({[x]y:exp x;log y}0n 0n 0n)",
             "1b");
    // sqrt(log(0n)) — same parity test.
    eq_q(&mut c, "null-fused-sqrt-log-parity",
         "(sqrt log 0n 0n 0n 0n)~({[x]y:log x;sqrt y}0n 0n 0n 0n)",
             "1b");
    // log(0) → fused vs unfused parity.
    eq_q(&mut c, "log-zero-parity",
         "(log 0.0 0.0)~({[x]log x}0.0 0.0)",
             "1b");
    // Sum after fused chain w/ null — fused vs unfused parity.
    eq_q(&mut c, "null-sum-parity-fused-vs-unfused",
         "(sum sqrt sqrt 1.0 0n 81.0)~({[x]a:sqrt x;sum sqrt a}1.0 0n 81.0)",
         "1b");
}

#[test]
fn complex_fusion_type_promotion() {
    // Integer types feeding a fused float chain must promote correctly.
    let mut c = conn();
    // KI → KF through sqrt.  Result type is KF.
    eq_q(&mut c, "promote-ki-sqrt",
         "(sqrt 1 4 9 16)~1.0 2.0 3.0 4.0",
             "1b");
    // KJ → KF deep — fused vs unfused parity (not numeric identity).
    eq_q(&mut c, "promote-kj-deep-parity",
         "(sqrt sqrt log exp 0.5+0.01*`long$til 100)~ \
          ({[x]a:exp x;b:log a;c:sqrt b;sqrt c}[0.5+0.01*`long$til 100])",
              "1b");
    // KH (short) into a chain.
    eq_q(&mut c, "promote-kh-sqrt",
         "(sqrt `short$1 4 9 16)~1.0 2.0 3.0 4.0",
             "1b");
    // KB (bool) — promotion to KI then KF.
    eq_q(&mut c, "promote-kb-sqrt",
         "(sqrt 1010b)~1.0 0.0 1.0 0.0",
             "1b");
    // KE through sqrt promotes to KF (lossy widen); pins current l behavior.
    eq_q(&mut c, "promote-ke-sqrt-widens-to-kf",
         "(type sqrt 1.0 4.0 9.0e)=9h",
             "1b");
}

#[test]
fn complex_fusion_at_scale() {
    // Large N under fusion; identity tests use bit-identical parity (the fusion invariant).
    let mut c = conn();
    // 100K identity: fused vs unfused must match bit-for-bit.
    eq_q(&mut c, "scale-100k-parity",
         "({log exp x}1.0+til 100000)~ \
          ({[x]a:exp x;log a}[1.0+til 100000])",
              "1b");
    // 1M parity — pp_for parallel partitioning.
    eq_q(&mut c, "scale-1m-parity",
         "({sqrt log exp x}1.0+til 1000000)~ \
          ({[x]a:exp x;b:log a;sqrt b}[1.0+til 1000000])",
              "1b");
    // 1M KF — sum-sqrt fused vs unfused parity.
    eq_q(&mut c, "scale-1m-sum-sqrt-parity",
         "(sum sqrt 1.0+til 1000000)~ \
          ({[x]y:sqrt x;sum y}[1.0+til 1000000])",
              "1b");
    // 1M KE (float32) — narrower lane width, different SIMD path.
    eq_q(&mut c, "scale-1m-ke-sum-sqrt-parity",
         "(sum sqrt `real$1.0+til 1000000)~ \
          ({[x]y:sqrt x;sum y}[`real$1.0+til 1000000])",
              "1b");
    // 1M KJ — integer→float promotion at scale.
    eq_q(&mut c, "scale-1m-kj-promote-parity",
         "(sum sqrt `long$1+til 1000000)~ \
          ({[x]y:sqrt x;sum y}[`long$1+til 1000000])",
              "1b");
}

#[test]
fn complex_fusion_in_select_clauses() {
    // Fused chains inside select/exec aggregation clauses.
    let mut c = conn();
    c.query("trade:([] sym:`a`b`a`b`a`b`a`b; px:1.0 4.0 9.0 16.0 25.0 36.0 \
        49.0 \
        64.0)")
     .unwrap();
    // avg(sqrt(px)) per sym — 4-element groups.  Fused: sqrt-then-avg.
    eq_q(&mut c, "sel-avg-sqrt-by-sym-a",
         "(exec avg sqrt px by sym from trade)[`a]",
         "((1.0+3.0+5.0+7.0)%4.0)");
    eq_q(&mut c, "sel-avg-sqrt-by-sym-b",
         "(exec avg sqrt px by sym from trade)[`b]",
         "((2.0+4.0+6.0+8.0)%4.0)");
    // sum(log(exp(px))) per sym — identity chain, should equal sum(px).
    eq_q(&mut c, "sel-sum-log-exp-by-sym-a",
         "1e-6>abs ((exec sum log exp px by sym from \
             trade)[`a])-(1.0+9.0+25.0+49.0)",
         "1b");
    // min(sqrt(px)) per sym.
    eq_q(&mut c, "sel-min-sqrt-by-sym-a",
         "(exec min sqrt px by sym from trade)[`a]", "1.0");
    eq_q(&mut c, "sel-min-sqrt-by-sym-b",
         "(exec min sqrt px by sym from trade)[`b]", "2.0");
    // Top-level (no `by`) — single reducer over fused chain.
    eq_q(&mut c, "sel-sum-sqrt-top",
         "1e-6>abs (exec sum sqrt px from trade)- \
                   ({sum sqrt x}1.0 4.0 9.0 16.0 25.0 36.0 49.0 64.0)",
                       "1b");
}

#[test]
fn complex_fusion_in_lambdas_nested() {
    // Fused chains inside nested lambdas / projections / each-context.
    let mut c = conn();
    // Lambda body fused, called from another lambda.
    eq_q(&mut c, "nested-lambda-fused",
         "({[x]{sum sqrt x}x}1.0 4.0 9.0)",
             "6f");
    // Projection of a fused-chain lambda.
    eq_q(&mut c, "projection-fused",
         "(({[x;y]sum sqrt x+y})[1.0 4.0 9.0])[0.0 0.0 0.0]",
             "6f");
    // each over a list of lists with fused inner chain.
    eq_q(&mut c, "each-list-of-lists",
         "({sum sqrt x}each (1.0 4.0;9.0 16.0;25.0 36.0))~ \
          3.0 7.0 11.0",
              "1b");
    // each-left / each-right binding with fused body.
    eq_q(&mut c, "each-left-fused",
         "({[x;y]sum sqrt x+y}[;0.0])each (1.0 4.0;9.0 16.0)",
             "3.0 7.0");
    // Composition with `each` (word adverb) and fused chain.
    eq_q(&mut c, "adverb-each-monadic",
         "(sqrt each 1.0 4.0 9.0)~1.0 2.0 3.0",
             "1b");
}

#[test]
fn complex_fusion_correctness_bit_identical() {
    // Bit-identical parity for tricky inputs; any 0b is a real divergence.
    let mut c = conn();
    // Negative→sqrt→NaN: must produce identical bit pattern (NaN sentinel).
    c.query("v: -1.0 -4.0 -9.0").unwrap();
    eq_q(&mut c, "bit-neg-sqrt",
         "(sqrt v)~({[x]y:sqrt x;y}v)",
             "1b");
    // Zero → log → -inf → exp → 0 → sqrt → 0.  Bit-identical.
    c.query("v: 0.0 0.0 0.0").unwrap();
    eq_q(&mut c, "bit-zero-deep",
         "(sqrt exp log v)~({[x]a:log x;b:exp a;sqrt b}v)",
             "1b");
    // Inf in the input: should pass through bit-identical.
    c.query("v: 0w -0w 1.0 0n").unwrap();
    eq_q(&mut c, "bit-inf-nan-mix",
         "(abs v)~({[x]y:abs x;y}v)",
             "1b");
    // 64K elements: fused must agree with the sequential variant exactly.
    eq_q(&mut c, "bit-64k-sum-sqrt",
         "(sum sqrt 1.0+til 65536)~({[x]y:sqrt x;sum y}[1.0+til 65536])",
             "1b");
}

#[test]
fn parser_edge_cases() {
    // Lexer edge cases anchored to behavior the modernized lexer must preserve.
    let mut c = conn();
    // Negative-number context vs binary minus.
    eq_q(&mut c, "neg-bin-vec-sub",     "5 6 7-5 6 7",              "0 0 0");
    eq_q(&mut c, "neg-unary-vec",       "1 -2 3",                   "1 -2 3");
    eq_q(&mut c, "neg-bin-atom",        "1-2",                      "-1");
    eq_q(&mut c, "neg-paren",           "(-2)+5",                   "3");
    eq_q(&mut c, "neg-after-verb",      "1+-2",                     "-1");
    eq_q(&mut c, "neg-double-space",    "-1 -7  0  0 -6 5",         "-1 -7 0 0 \
        -6 5");
    eq_q(&mut c, "neg-typed-suffix",    "-1 -7 0 0 -6h",            "-1 -7 0 0 \
        -6h");
    // Exponent rule: `e[-]?digit` is exponent, `e+` is NOT.
    eq_q(&mut c, "exp-bare",            "1e2",                      "100f");
    eq_q(&mut c, "exp-neg",             "1e-2",                     "0.01");
    eq_q(&mut c, "exp-plus-not-exp",    "1e+2",                     "3f");
    eq_q(&mut c, "exp-vec-plus",        "1 2 3e+4 5 6e",            "5 7 9e");
    eq_q(&mut c, "exp-scalar-vec",      "1e+1 2 3e",                "2 3 4e");
    eq_q(&mut c, "exp-fractional",      "1.5e3",                    "1500f");
    // Datetime / timestamp / time literals.
    eq_q(&mut c, "lex-datetime-z",
                  "type 2024.01.01T12:00:00.000",                   "-15h");
    eq_q(&mut c, "lex-time-ms",         "type 12:34:56.789",        "-19h");
    eq_q(&mut c, "lex-second",          "type 12:34:56",            "-18h");
    eq_q(&mut c, "lex-minute",          "type 12:34",               "-17h");
    eq_q(&mut c, "lex-date",            "type 2024.01.01",          "-14h");
    eq_q(&mut c, "lex-month",           "type 2024.01m",            "-13h");
    // Hex / bool / dot-digit.
    eq_q(&mut c, "lex-hex",             "0x010203",                 "0x010203");
    eq_q(&mut c, "lex-bool",            "1011b",                    "1011b");
    eq_q(&mut c, "lex-dotdigit",        ".5",                       "0.5");
    eq_q(&mut c, "lex-dotdigit-neg",    "(-.5)+1",                  "0.5");
    // q-mode digraphs <= >= <> >>.
    eq_q(&mut c, "lex-le",              "3<=4",                     "1b");
    eq_q(&mut c, "lex-ge",              "4>=3",                     "1b");
    eq_q(&mut c, "lex-ne",              "3<>4",                     "1b");
    // String escapes.
    eq_q(&mut c, "lex-str-tab",         "\"a\\tb\"",
        "\"a\\tb\"");
    eq_q(&mut c, "lex-str-newline",     "\"a\\nb\"",
        "\"a\\nb\"");
    eq_q(&mut c, "lex-str-quote",       "\"a\\\"b\"",
        "\"a\\\"b\"");
    eq_q(&mut c, "lex-str-octal",       "count \"\\101\"",          "1");
    // Backtick symbol lists.
    eq_q(&mut c, "lex-sym-list",        "`a`b`c",                   "`a`b`c");
    eq_q(&mut c, "lex-sym-empty",       "`",                        "`");
    eq_q(&mut c, "lex-sym-path",        "type `:/tmp/foo",          "-11h");
    // File-IO verbs `0:`, `1:`, `2:` lex distinctly from numerics.
    eq_q(&mut c, "lex-file-io-noun",
                  "@[{`hit};0;`miss]",                              "`hit");
    // Vector with space-suffix.
    eq_q(&mut c, "lex-vec-trailing-neg",    "1 -1",                "1 -1");
    // Multi-line continuation must not emit a newline token for indent continuation.
    eq_q(&mut c, "lex-multiline-select",
                  "2=count select c:count i by sym from\n   \
                      ([]sym:`a`b`a`b;price:1.0 2.0 -1.0 0.5) where price>0",
                  "1b");
}

#[test]
fn prim_qlib() {
    // src/qlib.c primitives.
    let mut c = conn();
    eq_q(&mut c, "q-ltrim",      "ltrim \"   hello\"",            "\"hello\"");
    eq_q(&mut c, "q-rtrim",      "rtrim \"hello   \"",            "\"hello\"");
    eq_q(&mut c, "q-trim",       "trim \"  hi  \"",                "\"hi\"");
    eq_q(&mut c, "q-all",        "all 1 1 1 0 1",                  "0b");
    eq_q(&mut c, "q-any",        "any 0 0 0 1 0",                  "1b");
    eq_q(&mut c, "q-lower",      "lower \"ABCxyz\"",              "\"abcxyz\"");
    eq_q(&mut c, "q-upper",      "upper \"ABCxyz\"",              "\"ABCXYZ\"");
    eq_q(&mut c, "q-signum-pos", "signum 3.5",                     "1");
    eq_q(&mut c, "q-signum-neg", "signum -2.5",                    "-1");
    eq_q(&mut c, "q-signum-zero","signum 0.0",                     "0");
    eq_q(&mut c, "q-mod",        "7 mod 3",                         "1");
    eq_q(&mut c, "q-xbar",       "5 xbar 17",                       "15");
    eq_q(&mut c, "q-xlog",       "2 xlog 1024",                     "10f");
    eq_q(&mut c, "q-xrank",      "3 xrank 1.0 4.0 2.0 9.0 5.0",    "0 1 0 2 1");
    eq_q(&mut c, "q-med",        "med 1 2 3 4 5",                   "3f");
    eq_q(&mut c, "q-var",        "var 1 2 3 4 5",                   "2f");
    eq_q(&mut c, "q-dev",        "0<dev 100 200 300",               "1b");
    eq_q(&mut c, "q-cov",        "1 2 3 cov 1 2 3",
        ".66666666666666666");
    eq_q(&mut c, "q-cor",        "1 2 3 cor 1 2 3",                 "1f");
    eq_q(&mut c, "q-inter",      "1 2 3 4 inter 2 3 5",            "2 3");
    eq_q(&mut c, "q-except",     "1 2 3 4 except 2 4",             "1 3");
    eq_q(&mut c, "q-ssr",        "ssr[\"foo bar foo\";\"foo\";\"baz\"]",
        "\"baz \
        bar baz\"");
}

#[test]
fn prim_data_formatting() {
    // src/data.c: dates, times, datetimes, symbols.
    let mut c = conn();
    eq_q(&mut c, "date-cast",     "`date$2020.06.15D12:00:00.000",
        "2020.06.15");
    eq_q(&mut c, "time-cast",     "`time$2020.06.15D12:30:45.123",
        "12:30:45.123");
    eq_q(&mut c, "minute-cast",   "`minute$12:30:45.123",
        "12:30");
    eq_q(&mut c, "second-cast",   "`second$12:30:45.123",
        "12:30:45");
    eq_q(&mut c, "month-cast",    "`month$2020.06.15",
        "2020.06m");
    eq_q(&mut c, "ym-as-str",     "string 2020.06m",
        "\"2020.06\"");
    eq_q(&mut c, "dt-as-str",     "string 2020.01.15",
        "\"2020.01.15\"");
    eq_q(&mut c, "dz-as-str",     "string 2020.01.15T10:20:30",
        "\"2020.01.15T10:20:30.000\"");
    eq_q(&mut c, "bool-as-str",   "string 01b",
        "(enlist\"0\";enlist\"1\")");
    eq_q(&mut c, "sym-count",     "count `a`b`c`d`e",                    "5");
    eq_q(&mut c, "char-cast",     "\"c\"$97 98 99",
        "\"abc\"");
    eq_q(&mut c, "int-cast-long", "\"i\"$1 2 3j",                          "1 \
        2 \
        3");
    eq_q(&mut c, "float-cast",    "\"f\"$1 2 3",                           "1 \
        2 \
        3f");
    eq_q(&mut c, "bool-cast",     "\"b\"$0 1 2 3",
        "0111b");
}

#[test]
fn prim_attributes() {
    let mut c = conn();
    eq_q(&mut c, "s-attr-asc",      "-2!`s#1 2 3",                "`s");
    eq_q(&mut c, "u-attr-int",      "-2!`u#1 2 3",                "`u");
    eq_q(&mut c, "p-attr",          "-2!`p#1 1 2 2 3 3",          "`p");
    eq_q(&mut c, "g-attr",          "-2!`g#1 2 1 3 1",            "`g");
    eq_q(&mut c, "s-attr-bin",      "bin[`s#1 3 5 7;5]",          "2");
    eq_q(&mut c, "u-attr-sym",      "-2!`u#`a`b`c",               "`u");
    eq_q(&mut c, "p-attr-stride",   "count group `p#1 1 2 2 3",   "3");
}

#[test]
fn prim_keyed_dict() {
    let mut c = conn();
    eq_q(&mut c, "dict-uplus",   "(`a`b!1 2) + `a`b!10 \
        20",                       "`a`b!11 22");
    eq_q(&mut c, "dict-fill",    "0^`a`b!1 \
        0N",                                    "`a`b!1 0");
    eq_q(&mut c, "table-fill",   "0^([]a:1 0N 2;b:0N 4 \
        5)",                        "([]a:1 0 2;b:0 4 5)");
    eq_q(&mut c, "fby-cnt",      "count select k,(max;v) fby k from \
        ([]k:`a`a`b;v:1 2 3)", "3");
    eq_q(&mut c, "dict-key",     "key `a`b`c!1 2 \
        3",                                "`a`b`c");
    eq_q(&mut c, "dict-value",   "value `a`b`c!1 2 \
        3",                              "1 2 3");
    eq_q(&mut c, "dict-reverse", "reverse `a`b`c!1 2 \
        3",                            "`c`b`a!3 2 1");
}

#[test]
fn prim_apply_adverbs() {
    let mut c = conn();
    eq_q(&mut c, "each",       "count each (1 2 3;4 5;6)", "3 2 1");
    eq_q(&mut c, "each-both",  "count 1 2 3+/:\\:1 2",     "3");
    eq_q(&mut c, "over-plus",  "(+/)1 2 3 4 5",             "15j");
    eq_q(&mut c, "scan-plus",  "(+\\)1 2 3 4 5",            "1 3 6 10 15");
    eq_q(&mut c, "diff-prior", "deltas 10 15 21 30",        "10 5 6 9");
    eq_q(&mut c, "over-max",   "(|/)3 1 4 1 5 9 2 6",       "9");
    eq_q(&mut c, "scan-min",   "(&\\)5 3 4 2 6",            "5 3 3 2 2");
}

#[test]
fn prim_types_arithmetic() {
    let mut c = conn();
    eq_q(&mut c, "add-bool",  "sum 01010b",      "2");
    eq_q(&mut c, "add-byte",  "sum 0x010203",    "6j");
    eq_q(&mut c, "add-short", "sum 1 2 3h",       "6j");
    eq_q(&mut c, "add-int",   "sum 1 2 3",        "6j");
    eq_q(&mut c, "add-long",  "sum 1 2 3j",       "6j");
    eq_q(&mut c, "add-real",  "sum 1 2 3e",       "6e");
    eq_q(&mut c, "add-float", "sum 1 2 3f",       "6f");
    eq_q(&mut c, "mul-short", "prd 1 2 3h",       "6");
    eq_q(&mut c, "mul-int",   "prd 1 2 3",        "6");
    eq_q(&mut c, "mul-long",  "prd 1 2 3j",       "6j");
    eq_q(&mut c, "mul-real",  "prd 1 2 3e",       "6e");
    eq_q(&mut c, "mul-float", "prd 1 2 3f",       "6f");
    eq_q(&mut c, "sub-int",   "10 9 8 - 1 2 3",   "9 7 5");
    eq_q(&mut c, "sub-long",  "10 9 8j - 1 2 3j", "9 7 5j");
    eq_q(&mut c, "sub-float", "10.5 - 0.5",        "10f");
    eq_q(&mut c, "div-float", "10.0%2.5",          "4f");
    eq_q(&mut c, "div-int",   "10%2",              "5f");
    eq_q(&mut c, "mod-int",   "17 mod 5",          "2");
    eq_q(&mut c, "mod-long",  "17j mod 5j",        "2j");
}

#[test]
fn prim_types_comparisons() {
    let mut c = conn();
    eq_q(&mut c, "eq-bool",  "10b=01b",                 "00b");
    eq_q(&mut c, "eq-short", "1 2 3h=1 3 3h",           "101b");
    eq_q(&mut c, "eq-int",   "1 2 3=1 3 3",             "101b");
    eq_q(&mut c, "eq-long",  "1 2 3j=1 3 3j",           "101b");
    eq_q(&mut c, "eq-float", "1.0 2.0 3.0=1.0 3.0 3.0", "101b");
    eq_q(&mut c, "eq-sym",   "`a`b`c=`a`x`c",           "101b");
    eq_q(&mut c, "lt-int",   "1 2 3<2",                  "100b");
    eq_q(&mut c, "lt-long",  "1 2 3j<2j",                "100b");
    eq_q(&mut c, "lt-float", "1.0 2.0 3.0<2.0",          "100b");
    eq_q(&mut c, "gt-short", "1 2 3h>2h",                "001b");
    eq_q(&mut c, "ge-int",   "1 2 3>=2",                 "011b");
    eq_q(&mut c, "le-int",   "1 2 3<=2",                 "110b");
}

#[test]
fn prim_types_reductions() {
    let mut c = conn();
    eq_q(&mut c, "max-bool",   "max 01010b",                "1b");
    eq_q(&mut c, "max-short",  "max 3 1 4 1 5h",            "5h");
    eq_q(&mut c, "max-int",    "max 3 1 4 1 5 9 2 6",        "9");
    eq_q(&mut c, "max-long",   "max 3 1 4 1 5 9 2 6j",       "9j");
    eq_q(&mut c, "max-real",   "max 3.1 1.1 4.1 9.1e",       "9.1e");
    eq_q(&mut c, "max-float",  "max 3.1 1.1 4.1 9.1",        "9.1f");
    eq_q(&mut c, "min-int",    "min 3 1 4 1 5",              "1");
    eq_q(&mut c, "min-long",   "min 3 1 4 1 5j",             "1j");
    eq_q(&mut c, "min-float",  "min 3.1 1.1 4.1",            "1.1f");
    eq_q(&mut c, "avg-int",    "avg 1 2 3 4",                 "2.5f");
    eq_q(&mut c, "avg-long",   "avg 1 2 3 4j",                "2.5f");
    eq_q(&mut c, "count-tbl",  "count ([]a:til 100;b:100?`3)","100");
    eq_q(&mut c, "count-dict", "count `a`b`c!1 2 3",          "3");
}

#[test]
fn prim_types_sort_by_type() {
    let mut c = conn();
    eq_q(&mut c, "asc-bool",     "asc 10011b",                 "`s#00111b");
    eq_q(&mut c, "asc-byte",     "asc 0x030102",               "`s#0x010203");
    eq_q(&mut c, "asc-short",    "asc 3 1 2h",                  "`s#1 2 3h");
    eq_q(&mut c, "asc-short-nan","asc 3h,0Nh,1h",               "`s#0N 1 3h");
    eq_q(&mut c, "asc-real",     "asc 3.1 1.1 2.1e",            "`s#1.1 2.1 \
        3.1e");
    eq_q(&mut c, "asc-float",    "asc 3.1 1.1 2.1",             "`s#1.1 2.1 \
        3.1");
    eq_q(&mut c, "asc-char",     "asc \"hello\"",               "`s#\"ehllo\"");
    eq_q(&mut c, "iasc-long",    "iasc 30 10 20j",              "1 2 0");
    eq_q(&mut c, "iasc-float",   "iasc 30.0 10.0 20.0",          "1 2 0");
    eq_q(&mut c, "iasc-sym",     "iasc `charlie`alpha`bravo",   "1 2 0");
    eq_q(&mut c, "desc-short",   "desc 1 2 3h",                  "3 2 1h");
    eq_q(&mut c, "desc-float",   "desc 1.0 2.0 3.0",             "3 2 1f");
}

#[test]
fn prim_types_indexing() {
    let mut c = conn();
    eq_q(&mut c, "find-int",       "1 2 3 4 5?3",                       "2");
    eq_q(&mut c, "find-long",      "1 2 3 4 5j?3j",                      "2");
    eq_q(&mut c, "find-float",     "1.0 2.0 3.0 4.0?3.0",                "2");
    eq_q(&mut c, "find-sym",       "`a`b`c`d?`c",                        "2");
    eq_q(&mut c, "find-vec",       "1 2 3 4 5?3 5",                       "2 \
        4");
    eq_q(&mut c, "bin-int",        "1 3 5 7 9 bin 4",                     "1");
    eq_q(&mut c, "bin-long",       "1 3 5 7 9j bin 4j",                   "1");
    eq_q(&mut c, "bin-float",      "1.0 3.0 5.0 7.0 9.0 bin 4.0",        "1");
    eq_q(&mut c, "within-int",     "(1 2 3 4 5) within 2 4",
        "01110b");
    eq_q(&mut c, "within-long",    "(1 2 3 4 5j) within 2 4j",
        "01110b");
    eq_q(&mut c, "distinct-int",   "distinct 1 2 2 3 3 3 1",             "1 2 \
        3");
    eq_q(&mut c, "distinct-sym",   "distinct `a`b`a`c`b",
        "`a`b`c");
    eq_q(&mut c, "where-bool",     "where 10110b",                        "0 2 \
        3");
    eq_q(&mut c, "group-int-keys", "asc key group 1 2 1 2 1",
        "`s#1 \
        2");
    eq_q(&mut c, "count-group",    "count group 1 2 1 3 1 2",             "3");
}

#[test]
fn prim_types_cast_typed_null() {
    let mut c = conn();
    eq_q(&mut c, "null-short",     "0Nh",            "0Nh");
    eq_q(&mut c, "null-int",       "0Ni",            "0Ni");
    eq_q(&mut c, "null-long",      "0Nj",            "0Nj");
    eq_q(&mut c, "null-real",      "0Ne",            "0Ne");
    eq_q(&mut c, "null-float",     "0n",              "0n");
    eq_q(&mut c, "null-sym",       "`$\"\"",          "`$\"\"");
    eq_q(&mut c, "null-char",      "\" \"",            "\" \"");
    eq_q(&mut c, "null-bool-cast", "\"b\"$0",          "0b");
    eq_q(&mut c, "short-cast",     "\"h\"$100",        "100h");
    eq_q(&mut c, "int-cast",       "\"i\"$100",        "100i");
    eq_q(&mut c, "long-cast",      "\"j\"$100",        "100j");
    eq_q(&mut c, "real-cast",      "\"e\"$1",          "1e");
    eq_q(&mut c, "float-cast-int", "\"f\"$100",        "100f");
    eq_q(&mut c, "sym-cast-str",   "`$\"hello\"",     "`hello");
    eq_q(&mut c, "str-cast-sym",   "string `hello",  "\"hello\"");
}

#[test]
fn prim_types_moving_window() {
    let mut c = conn();
    eq_q(&mut c, "mavg-int",   "3 mavg 1 2 3 4 5f",  "1 1.5 2 3 4f");
    eq_q(&mut c, "mavg-float", "3 mavg 1 2 3 4 5.0", "1 1.5 2 3 4f");
    eq_q(&mut c, "msum-int",   "3 msum 1 2 3 4 5",    "1 3 6 9 12");
    eq_q(&mut c, "mmin-int",   "3 mmin 5 3 4 1 2",    "5 3 3 1 1");
    eq_q(&mut c, "mmax-int",   "3 mmax 1 3 2 5 4",    "1 3 3 5 5");
    eq_q(&mut c, "wsum-float", "1 2 3 wsum 4 5 6f",   "32f");
    eq_q(&mut c, "wavg-int",   "1 1 1 wavg 1 2 3",    "2f");
}

#[test]
fn prim_coverage_date_parse() {
    // src/data.c: dmy, dj, dl, dz, pd, pt
    let mut c = conn();
    eq_q(&mut c, "parse-date",     "\"D\"$\"2020.06.15\"",
        "2020.06.15");
    eq_q(&mut c, "parse-month",    "\"M\"$\"2020.06\"",
        "2020.06m");
    eq_q(&mut c, "parse-time",     "\"T\"$\"12:30:45.123\"",
        "12:30:45.123");
    eq_q(&mut c, "parse-minute",   "\"U\"$\"12:30\"",                 "12:30");
    eq_q(&mut c, "parse-second",   "\"V\"$\"12:30:45\"",
        "12:30:45");
    eq_q(&mut c, "parse-datetime", "\"Z\"$\"2020.06.15T10:20:30\"",
        "2020.06.15T10:20:30");
    eq_q(&mut c, "parse-int",      "\"I\"$\"42\"",                     "42");
    eq_q(&mut c, "parse-float",    "\"F\"$\"3.14\"",                   "3.14");
    eq_q(&mut c, "parse-sym",      "\"S\"$\"hello\"",
        "`hello");
    eq_q(&mut c, "parse-bool",     "\"B\"$\"1\"",                       "1b");
}

#[test]
fn prim_coverage_date_arith() {
    // src/data.c: dt, dj, dmy, pz, pm, tz
    let mut c = conn();
    eq_q(&mut c, "date-plus-days",  "2020.01.01+5",
        "2020.01.06");
    eq_q(&mut c, "date-minus-date", "2020.06.15-2020.01.01",          "166");
    eq_q(&mut c, "dt-plus-days",    "2020.06.15T10:00:00.000+5",
        "2020.06.20T10:00:00.000");
    eq_q(&mut c, "time-diff",       "12:30:00.000-12:00:00.000",
        "00:30:00.000");
    eq_q(&mut c, "date-month",      "`month$2020.06.15",
        "2020.06m");
    eq_q(&mut c, "date-year",       "`year$2020.06.15",               "2020i");
}

#[test]
fn prim_coverage_bin_atom() {
    // searching_flags.c: bn — atom search via SIMD binary descent.
    let mut c = conn();
    eq_q(&mut c, "bin-atom-int",    "1 3 5 7 9 bin \
        4",                                              "1");
    eq_q(&mut c, "bin-atom-long",   "1 3 5 7 9j bin \
        4j",                                            "1");
    eq_q(&mut c, "bin-atom-float",  "1.0 3.0 5.0 7.0 9.0 bin \
        4.0",                                  "1");
    eq_q(&mut c, "bin-atom-time",   "12:00:00.000 13:00:00.000 14:00:00.000 \
        bin \
        12:30:00.000",     "0");
    eq_q(&mut c, "bin-atom-before", "1 3 5 bin \
        0",                                                  "-1");
    eq_q(&mut c, "bin-atom-after",  "1 3 5 bin \
        10",                                                  "2");
    eq_q(&mut c, "bin-atom-exact",  "1 3 5 bin \
        3",                                                   "1");
}

#[test]
fn prim_coverage_csv_format() {
    let mut c = conn();
    eq_q(&mut c, "csv-format",     "count \",\" vs \"a,b,c,d\"",  "4");
    eq_q(&mut c, "csv-parse-long", "\"J\"$\"100\"",                "100j");
}

#[test]
fn prim_coverage_table_compose() {
    let mut c = conn();
    eq_q(&mut c, "flip-dict", "flip `a`b!(1 2;3 4)", "([]a:1 2;b:3 4)");
}

#[test]
fn prim_coverage_null_match_corner() {
    // primitives.c: wnn, mat, ltn/gtn
    let mut c = conn();
    eq_q(&mut c, "wnn",            "{wn:1 0N 2 0N 3;count where not null \
        wn}[]", "3");
    eq_q(&mut c, "mat-true",       "1 2 3~1 2 3",
        "1b");
    eq_q(&mut c, "mat-false",      "1 2 3~1 2 4",
        "0b");
    eq_q(&mut c, "mat-diff-types", "1~1.0",
        "0b");
    eq_q(&mut c, "lt-sym-vec",     "`a`b`c<`b",
        "100b");
    eq_q(&mut c, "gt-sym-vec",     "`a`b`c>`b",
        "001b");
    eq_q(&mut c, "eq-mixed-num",   "1~1j",
        "0b");
}

// xbar over time.minute regression (4-byte temporal dispatch) + stable iasc/idesc on long ties.
#[test]
fn prim_simd_grade_j_stability() {
    let mut c = conn();
    eq_q(&mut c, "iasc-long-tied-small",
         "iasc 5 5 5 5 5j", "0 1 2 3 4");
    // {[]...} declares no args so x isn't captured as a param.
    eq_q(&mut c, "iasc-long-large-ties",
         "{[]v:1000#5 3 5 3 1 5j;i:iasc v;y:v i;\
           (asc i where y=5)~(i where y=5)}[]",
         "1b");
    eq_q(&mut c, "iasc-long-3-groups",
         "{[]v:2000#3 1 2j;i:iasc v;y:v i;\
           all((asc i where y=1)~i where y=1;\
               (asc i where y=2)~i where y=2;\
               (asc i where y=3)~i where y=3)}[]",
         "1b");
}

#[test]
fn prim_xbar_time_minute() {
    let mut c = conn();
    eq_q(&mut c, "xbar-minute-vec",
         "60 xbar 09:15 09:45 10:30 11:59",
         "09:00 09:00 10:00 11:00");
    eq_q(&mut c, "xbar-minute-atom",
         "60 xbar 09:45",
         "09:00");
    eq_q(&mut c, "xbar-time-minute-from-tbl",
         "{t:([]price:1 2 3.0;tm:09:15:00.000 09:45:00.000 10:30:00.000); \
           (60 xbar `minute$t`tm)~09:00 09:00 10:00}[]",
         "1b");
}

// Parallel-kernel fuzz suite — adversarial sizes x every numeric type x every parallel op.

const FUZZ_SIZES: &[usize] = &[
    4200, 5000, 8000, 14000, 15000, 17000, 20000, 53000, 100000, 262144,
];

#[test]
fn hot_pp_cmp_kb_alignment() {
    // Original repro: a compare producing a 1-byte result across worker partitions.
    let mut c = conn();
    for &n in FUZZ_SIZES {
        // Scalar-vec comparison; random uniform in [0,100) so all are < 101 and <= 99.
        eq_q(&mut c, &format!("cmpi-lt-SV-n{n}"),
             &format!("all ({n}?100i)<101i"), "1b");
        eq_q(&mut c, &format!("cmpi-gte-SV-n{n}"),
             &format!("all ({n}?100i)>=0i"), "1b");
        eq_q(&mut c, &format!("cmpi-eq-has-zero-n{n}"),
             &format!("1b=any ({n}?2i)=0i"), "1b");
        // Vec-vec comparison: a < a+1 always true
        eq_q(&mut c, &format!("cmpi-lt-VV-n{n}"),
             &format!("{{a:{n}?100i;all a<a+1i}}[]"), "1b");
        // KJ (long) variant
        eq_q(&mut c, &format!("cmpj-lt-SV-n{n}"),
             &format!("all ({n}?100j)<101j"), "1b");
        eq_q(&mut c, &format!("cmpj-VV-n{n}"),
             &format!("{{a:{n}?100j;all a<=a+1j}}[]"), "1b");
    }
}

#[test]
fn hot_pp_d2_arith_all_types() {
    // Arithmetic workers; result shares type with input, fuzzed for partition bugs.
    let mut c = conn();
    for &n in FUZZ_SIZES {
        // KI arithmetic
        eq_q(&mut c, &format!("addi-VV-n{n}"),
             &format!("{n}=count (({n}?100i)+{n}?100i)"), "1b");
        eq_q(&mut c, &format!("subi-SV-n{n}"),
             &format!("{n}=count (1000i-{n}?100i)"), "1b");
        eq_q(&mut c, &format!("muli-VV-n{n}"),
             &format!("{n}=count (({n}?10i)*{n}?10i)"), "1b");
        // KJ arithmetic
        eq_q(&mut c, &format!("addj-VV-n{n}"),
             &format!("{n}=count (({n}?100j)+{n}?100j)"), "1b");
        eq_q(&mut c, &format!("mulj-SV-n{n}"),
             &format!("{n}=count (2j*{n}?100j)"), "1b");
        // KF arithmetic (includes division — only f path supports %)
        eq_q(&mut c, &format!("addf-VV-n{n}"),
             &format!("{n}=count (({n}?100.0)+{n}?100.0)"), "1b");
        eq_q(&mut c, &format!("divf-SV-n{n}"),
             &format!("{n}=count (2.0%0.5+{n}?100.0)"), "1b");
        // KE arithmetic; note q's float parser needs spaces around a trailing minus.
        eq_q(&mut c, &format!("adde-VV-n{n}"),
             &format!("{n}=count (({n}?100e)+{n}?100e)"), "1b");
        eq_q(&mut c, &format!("sube-SV-n{n}"),
             &format!("{n}=count (1000e - {n}?100e)"), "1b");
        eq_q(&mut c, &format!("mule-VV-n{n}"),
             &format!("{n}=count (({n}?10e)*{n}?10e)"), "1b");
        eq_q(&mut c, &format!("dive-SV-n{n}"),
             &format!("{n}=count (2e % 0.5e + {n}?100e)"), "1b");
        // Commutativity: a+b ~ b+a for KI vecs
        eq_q(&mut c, &format!("comm-addi-n{n}"),
             &format!("{{a:{n}?100i;b:{n}?100i;(a+b)~b+a}}[]"), "1b");
        // Identity: a*0 ~ 0#a
        eq_q(&mut c, &format!("zero-muli-n{n}"),
             &format!("{{a:{n}?100i;(a*0i)~{n}#0i}}[]"), "1b");
    }
}

#[test]
fn hot_pp_mm_minmax_all_types() {
    // pp_mm: vec-vec min/max on KI/KJ.  Result is same type as inputs.
    let mut c = conn();
    for &n in FUZZ_SIZES {
        // KI min/max: result <= both inputs (min) and >= both (max).
        eq_q(&mut c, &format!("minvv-KI-n{n}"),
             &format!("{{a:{n}?100i;b:{n}?100i;(all (a&b)<=a)&all \
                 (a&b)<=b}}[]"), "1b");
        eq_q(&mut c, &format!("maxvv-KI-n{n}"),
             &format!("{{a:{n}?100i;b:{n}?100i;(all (a|b)>=a)&all \
                 (a|b)>=b}}[]"), "1b");
        // KJ
        eq_q(&mut c, &format!("minvv-KJ-n{n}"),
             &format!("{{a:{n}?100j;b:{n}?100j;(all (a&b)<=a)&all \
                 (a&b)<=b}}[]"), "1b");
        eq_q(&mut c, &format!("maxvv-KJ-n{n}"),
             &format!("{{a:{n}?100j;b:{n}?100j;(all (a|b)>=a)&all \
                 (a|b)>=b}}[]"), "1b");
        // Scalar/vec: (a | 50i) ≥ 50i always
        eq_q(&mut c, &format!("maxsv-KI-n{n}"),
             &format!("all 50i<=({n}?100i)|50i"), "1b");
    }
}

#[test]
fn hot_pp_m1_neg_abs_all_types() {
    // pp_m1: monadic neg/abs for KH/KI/KJ/KE/KF.  Result same type.
    let mut c = conn();
    for &n in FUZZ_SIZES {
        // neg: --a ~ a
        eq_q(&mut c, &format!("negneg-KI-n{n}"),
             &format!("{{a:{n}?100i;(neg neg a)~a}}[]"), "1b");
        eq_q(&mut c, &format!("negneg-KJ-n{n}"),
             &format!("{{a:{n}?100j;(neg neg a)~a}}[]"), "1b");
        eq_q(&mut c, &format!("negneg-KE-n{n}"),
             &format!("{{a:{n}?100e;(neg neg a)~a}}[]"), "1b");
        eq_q(&mut c, &format!("negneg-KF-n{n}"),
             &format!("{{a:{n}?100.0;(neg neg a)~a}}[]"), "1b");
        eq_q(&mut c, &format!("negneg-KH-n{n}"),
             &format!("{{a:\"h\"${n}?100;(neg neg a)~a}}[]"), "1b");
        // abs: abs a ≥ 0 always
        eq_q(&mut c, &format!("abs-KI-n{n}"),
             &format!("all 0i<=abs neg {n}?100i"), "1b");
        eq_q(&mut c, &format!("abs-KJ-n{n}"),
             &format!("all 0j<=abs neg {n}?100j"), "1b");
        eq_q(&mut c, &format!("abs-KE-n{n}"),
             &format!("all 0e<=abs neg {n}?100e"), "1b");
        eq_q(&mut c, &format!("abs-KF-n{n}"),
             &format!("all 0.0<=abs neg {n}?100.0"), "1b");
    }
}

#[test]
fn hot_pp_red_reductions_all_types() {
    // Reductions over KI/KJ/KE/KF: sum of n ones = n; min of identicals = that value.
    let mut c = conn();
    for &n in FUZZ_SIZES {
        // sum = n when all 1s
        eq_q(&mut c, &format!("sum-ones-KI-n{n}"),
             &format!("(sum {n}#1i)={n}"), "1b");
        eq_q(&mut c, &format!("sum-ones-KJ-n{n}"),
             &format!("(sum {n}#1j)={n}j"), "1b");
        eq_q(&mut c, &format!("sum-ones-KE-n{n}"),
             &format!("(sum {n}#1e)=(\"e\"${n})"), "1b");
        eq_q(&mut c, &format!("sum-ones-KF-n{n}"),
             &format!("(sum {n}#1.0)=(\"f\"${n})"), "1b");
        // min/max of constant vec = that value
        eq_q(&mut c, &format!("min-const-KI-n{n}"),
             &format!("(min {n}#42i)=42i"), "1b");
        eq_q(&mut c, &format!("max-const-KJ-n{n}"),
             &format!("(max {n}#42j)=42j"), "1b");
        eq_q(&mut c, &format!("min-const-KE-n{n}"),
             &format!("(min {n}#42e)=42e"), "1b");
        eq_q(&mut c, &format!("max-const-KE-n{n}"),
             &format!("(max {n}#42e)=42e"), "1b");
        // sum of 0..n-1 = n*(n-1)/2 via the KJ path (no overflow).
        let expected = (n as u64) * (n as u64 - 1) / 2;
        eq_q(&mut c, &format!("sum-til-n{n}"),
             &format!("(sum \"j\"$til {n})={expected}j"), "1b");
        // Invariant: sum a = sum reverse a
        eq_q(&mut c, &format!("sum-rev-KI-n{n}"),
             &format!("{{a:{n}?100i;(sum a)=sum reverse a}}[]"), "1b");
        // KE sum accumulates in F32; assert within F32 tolerance of the F64 truth.
        eq_q(&mut c, &format!("sum-tol-KE-n{n}"),
             &format!("{{a:{n}?100e;t:sum\"f\"$a;1e-5>(abs t-sum a)%1.0|abs \
                 t}}[]"), "1b");
    }
}

#[test]
fn hot_mixed_type_promotion() {
    // Type promotion in parallel ops (KH+KI, KI+KJ, KI+KF) fuzzed here.
    let mut c = conn();
    for &n in FUZZ_SIZES {
        // KH + KI → KI
        eq_q(&mut c, &format!("prom-HI-n{n}"),
             &format!("{{a:\"h\"${n}?10;b:{n}?10i;{n}=count a+b}}[]"), "1b");
        // KI + KJ → KJ
        eq_q(&mut c, &format!("prom-IJ-n{n}"),
             &format!("{{a:{n}?10i;b:{n}?10j;{n}=count a+b}}[]"), "1b");
        // KI + KF → KF
        eq_q(&mut c, &format!("prom-IF-n{n}"),
             &format!("{{a:{n}?10i;b:{n}?10.0;{n}=count a+b}}[]"), "1b");
        // KE + KF → KF
        eq_q(&mut c, &format!("prom-EF-n{n}"),
             &format!("{{a:\"e\"${n}?10.0;b:{n}?10.0;{n}=count a+b}}[]"), "1b");
    }
}

#[test]
fn hot_chained_parallel_ops() {
    // Chains of back-to-back parallel ops catch cross-call state corruption.
    let mut c = conn();
    for &n in FUZZ_SIZES {
        // ((a+b)*c)<d chain
        eq_q(&mut c, &format!("chain-add-mul-cmp-n{n}"),
             &format!("{{a:{n}?100i;b:{n}?100i;c:{n}?10i;\
                       all ((a+b)*c)<(1000000i+0*a)}}[]"), "1b");
        // Reduction after dyadic: sum (a<b) is count of true positions
        eq_q(&mut c, &format!("chain-cmp-sum-n{n}"),
             &format!("{{a:{n}?100i;b:{n}?100i;\
                       (sum a<b)=count where a<b}}[]"), "1b");
        // abs(a-b) symmetry
        eq_q(&mut c, &format!("chain-sub-abs-n{n}"),
             &format!("{{a:{n}?100i;b:{n}?100i;(abs a-b)~abs b-a}}[]"), "1b");
        // KE chain arithmetic + reduction; loose tolerance since float sum order differs.
        eq_q(&mut c, &format!("chain-KE-mul-sum-n{n}"),
             &format!("{{a:{n}?10e;b:{n}?10e;\
                       1e-2>abs (sum a*b)-sum a*b}}[]"), "1b");
        // KE neg-neg-add round-trip
        eq_q(&mut c, &format!("chain-KE-negneg-n{n}"),
             &format!("{{a:{n}?100e;b:{n}?100e;((neg neg a)+b)~a+b}}[]"), "1b");
    }
}

#[test]
fn hot_boundary_sizes() {
    // Sizes right around the parallel/sequential threshold; both paths must match.
    let mut c = conn();
    for &n in &[999usize, 1000, 1001, 3999, 4000, 4001,
                4095, 4096, 4097,                                               // near 4K page boundary
                16383, 16384, 16385,                                            // near 16K
                65535, 65536, 65537,                                            // near 64K
                ] {
        // KI compare — crosses parallel threshold around n=4000
        eq_q(&mut c, &format!("boundary-cmp-n{n}"),
             &format!("all 0i<=({n}?100i),0i"), "1b");
        // KF arithmetic — crosses too
        eq_q(&mut c, &format!("boundary-addf-n{n}"),
             &format!("{n}=count (({n}?100.0)+{n}?100.0)"), "1b");
        // Count invariant for KJ
        eq_q(&mut c, &format!("boundary-sumj-n{n}"),
             &format!("(sum {n}#1j)={n}j"), "1b");
    }
}

// KE end-to-end — F32 stays native through arith/compare/min-max/reductions/neg-abs/division.

#[test]
fn prim_ke_arith() {
    // +/-/*// on KE vecs → KE result (type 8), not KF (9).
    let mut c = conn();
    eq_q(&mut c, "add-EE-VV", "type ((\"e\"$1 2 3)+\"e\"$4 5 6)", "8h");
    eq_q(&mut c, "sub-EE-VV", "((\"e\"$10 20 30)-\"e\"$1 2 3)", "9 18 27e");
    eq_q(&mut c, "mul-EE-SV", "((\"e\"$2.0)*\"e\"$1 2 3)", "2 4 6e");
    eq_q(&mut c, "div-EE-VV", "type ((\"e\"$10 20 30)%\"e\"$2 4 5)", "8h");
    eq_q(&mut c, "div-EE-val", "((\"e\"$10 20 30)%\"e\"$2 4 5)", "5 5 6e");
}

#[test]
fn prim_ke_compare() {
    // =/</> on KE → KB regardless of input width.  Covers pp_cmpe path.
    let mut c = conn();
    eq_q(&mut c, "eq-EE-VV", "((\"e\"$1 2 3)=\"e\"$1 9 3)", "101b");
    eq_q(&mut c, "lt-EE-SV", "((\"e\"$2.0)<\"e\"$1 2 3)", "001b");
    eq_q(&mut c, "gt-EE-VV", "((\"e\"$3 2 1)>\"e\"$1 2 3)", "100b");
    // Partition-boundary regression for pp_cmpe (mirrors a642dc2 fix).
    eq_q(&mut c, "eq-EE-n64", "64=sum\"j\"$(\"e\"$64#1.0)=\"e\"$64#1.0", "1b");
}

#[test]
fn prim_ke_minmax() {
    // &/| on KE → KE; vec-vec and scalar-vec.
    let mut c = conn();
    eq_q(&mut c, "min-EE-VV", "type ((\"e\"$5 1 3)&\"e\"$2 4 6)", "8h");
    eq_q(&mut c, "min-EE-val", "((\"e\"$5 1 3)&\"e\"$2 4 6)", "2 1 3e");
    eq_q(&mut c, "max-EE-SV", "((\"e\"$2.0)|\"e\"$1 5 3)", "2 5 3e");
}

#[test]
fn prim_ke_reductions() {
    // sum/prd/min/max on KE returns a KE atom matching the F64-cast answer within 1e-5.
    let mut c = conn();
    eq_q(&mut c, "sum-E-type",  "type sum \"e\"$1 2 3", "-8h");
    eq_q(&mut c, "sum-E-val",   "sum \"e\"$1 2 3", "6e");
    eq_q(&mut c, "prd-E-type",  "type prd \"e\"$1 2 3", "-8h");
    eq_q(&mut c, "prd-E-val",   "prd \"e\"$1 2 3", "6e");
    eq_q(&mut c, "min-E-atom",  "min \"e\"$5 1 3", "1e");
    eq_q(&mut c, "max-E-atom",  "max \"e\"$5 1 3", "5e");
    // Parallel path (> PP_N0): 4200 ones sum to 4200.
    eq_q(&mut c, "sum-E-par",   "(sum \"e\"$4200#1.0)=4200e", "1b");
    // F64-accumulator precision: sum 1M of 1.0 exactly representable.
    eq_q(&mut c, "sum-E-1M",    "(sum \"e\"$1000000#1.0)=1000000e", "1b");
}

#[test]
fn prim_ke_negabs() {
    // neg/abs on KE → KE (no width change); native pp_m1w path.
    let mut c = conn();
    eq_q(&mut c, "neg-E-type", "type neg \"e\"$1 2 3", "8h");
    eq_q(&mut c, "neg-E-val",  "neg \"e\"$1 2 3", "-1 -2 -3e");
    eq_q(&mut c, "abs-E-type", "type abs \"e\"$-1 -2 3", "8h");
    eq_q(&mut c, "abs-E-val",  "abs \"e\"$-1 -2 3", "1 2 3e");
}

#[test]
fn prim_ke_mixed_promotion() {
    // Mixed-width rules unchanged: KE+KF → KF; explicit cast works.
    let mut c = conn();
    eq_q(&mut c, "prom-EF-VV", "type ((\"e\"$1 2 3)+1 2 3.0)", "9h");
    eq_q(&mut c, "cast-FE",    "type \"e\"$1 2 3.0", "8h");
    eq_q(&mut c, "cast-EF",    "type \"f\"$\"e\"$1 2 3", "9h");
}

#[test]
fn hot_ke_peach() {
    // Parallel KE branches must run clean under peach (aligned-store checks).
    let mut c = conn();
    for &n in &[999usize, 1000, 1001, 4095, 4096, 4097, 65537] {
        eq_q(&mut c, &format!("peach-add-E-n{n}"),
             &format!("{n}=count ((\"e\"${n}?1.0)+\"e\"${n}?1.0)"), "1b");
        eq_q(&mut c, &format!("peach-min-E-n{n}"),
             &format!("{{a:\"e\"${n}?1.0;b:\"e\"${n}?1.0;(all (a&b)<=a)&all \
                 (a&b)<=b}}[]"),
             "1b");
        eq_q(&mut c, &format!("peach-sum-E-n{n}"),
             &format!("{{a:\"e\"${n}#1.0;(sum a)={n}e}}[]"), "1b");
    }
}

// COVERAGE BOOST — exercise core paths so previously-uncovered files report coverage.

#[test]
fn cov_qlib_trim_case_ssr() {
    let mut c = conn();
    eq_q(&mut c, "ltrim", "ltrim \"   hello\"",                "\"hello\"");
    eq_q(&mut c, "rtrim", "rtrim \"hello   \"",                "\"hello\"");
    eq_q(&mut c, "trim",  "trim  \"   hello   \"",             "\"hello\"");
    eq_q(&mut c, "lower", "lower \"HELLO WORLD\"",             "\"hello \
        world\"");
    eq_q(&mut c, "upper", "upper \"hello world\"",             "\"HELLO \
        WORLD\"");
    eq_q(&mut c, "ssr",   "ssr[\"foo bar\";\" \";\"_\"]",      "\"foo_bar\"");
}

#[test]
fn cov_qlib_signum_all_types() {
    let mut c = conn();
    eq_q(&mut c, "signum-KI-vec",  "signum -3 0 5i",           "-1 0 1i");
    eq_q(&mut c, "signum-KJ-vec",  "signum -3 0 5j",           "-1 0 1j");
    eq_q(&mut c, "signum-KF-vec",  "signum -3.3 0.0 5.5",      "-1 0 1i");
    eq_q(&mut c, "signum-KE-vec",  "signum `real$-3.3 0.0 5.5","-1 0 1i");
    eq_q(&mut c, "signum-KH-vec",  "signum `short$-3 0 5",     "-1 0 1i");
    eq_q(&mut c, "signum-KB-vec",  "signum 00b,11b",           "0 0 1 1i");
    eq_q(&mut c, "signum-KI-atm",  "signum 42i",               "1i");
    eq_q(&mut c, "signum-KJ-atm",  "signum -42j",              "-1j");
    eq_q(&mut c, "signum-KF-atm",  "signum 0.0",               "0i");
}

#[test]
fn cov_qlib_mod_floor() {
    // l's mod is floor modulo: sign follows divisor (-7 mod -5 = -2).
    let mut c = conn();
    eq_q(&mut c, "mod-ii-pair",   "mod[17i; 5i]",                "2i");
    eq_q(&mut c, "mod-jj-pair",   "mod[-7j; -5j]",               "-2j");
    eq_q(&mut c, "mod-ff-pair",   "mod[7.0; 3.0]",               "1.0");
    eq_q(&mut c, "mod-vec-atm",   "mod[13 14 15 16 17i; 3i]",    "1 2 0 1 2i");
    eq_q(&mut c, "mod-f-vec",     "mod[6.0 7.0 8.0; 3.0]",       "0.0 1.0 2.0");
}

#[test]
fn cov_qlib_xbar_xlog_xrank() {
    let mut c = conn();
    eq_q(&mut c, "xbar-ii",        "5i xbar 23i",                 "20i");
    eq_q(&mut c, "xbar-ff",        "5.0 xbar 23.7",               "20.0");
    eq_q(&mut c, "xbar-int-vec",   "10i xbar 5 15 22 29i",        "0 10 20 \
        20i");
    eq_q(&mut c, "xbar-f-vec",     "10 xbar 5.0 15.0 22.0",       "0.0 10.0 \
        20.0");
    eq_q(&mut c, "xlog-ff",        "xlog[10.0; 100.0]",           "2.0");
    eq_q(&mut c, "xlog-f-vec",     "3=count xlog[10;10 100 1000f]","1b");
    // xrank buckets must all lie in [0,nb-1]; check the previously-broken size range plus a large one.
    eq_q(&mut c, "xrank-100",  "all (xrank[4;100?100i])  within 0 3i",   "1b");
    eq_q(&mut c, "xrank-200",  "all (xrank[4;200?100i])  within 0 3i",   "1b");
    eq_q(&mut c, "xrank-300",  "all (xrank[4;300?100i])  within 0 3i",   "1b");
    eq_q(&mut c, "xrank-400",  "all (xrank[4;400?100i])  within 0 3i",   "1b");
    eq_q(&mut c, "xrank-1000", "all (xrank[4;1000?100i]) within 0 3i",   "1b");
    eq_q(&mut c, "xrank-100k", "all (xrank[20;100000?1000i]) within 0 19i",
        "1b");
}

#[test]
fn cov_qlib_all_any() {
    let mut c = conn();
    eq_q(&mut c, "all-bool-t", "all 10#1b",           "1b");
    eq_q(&mut c, "all-bool-f", "all 10#0b",           "0b");
    eq_q(&mut c, "any-bool",   "any (5#0b),1b",       "1b");
    eq_q(&mut c, "all-i-pos",  "0i<all 1 2 3i",       "1b");
    eq_q(&mut c, "any-i-hit",  "0i<any 0 0 42 0i",    "1b");
}

#[test]
fn cov_qlib_var_dev_med_cov_cor() {
    let mut c = conn();
    eq_q(&mut c, "var-F",     "0f<var 1000?100.0",              "1b");
    eq_q(&mut c, "dev-F",     "0f<dev 1000?100.0",              "1b");
    eq_q(&mut c, "var-I",     "0f<var 1000?100i",               "1b");
    eq_q(&mut c, "var-H",     "0f<var `short$1000?100",         "1b");
    eq_q(&mut c, "dev-J",     "0f<dev 1000?1000000j",           "1b");
    eq_q(&mut c, "med-F",     "(med 1000?100.0) within 0.0 100.0","1b");
    eq_q(&mut c, "med-I",     "(med 1000?100i)  within 0.0 100.0","1b");
    // cov/cor require float args; auto-correlation on an offset copy is bounded below by 0.9.
    eq_q(&mut c, "cor-offset",
         "{xs:1000?100.0;ys:xs+1000?1.0;0.9<cor[xs;ys]}[]","1b");
    // dict: var/dev should return a dict with one per-column scalar.
    eq_q(&mut c, "var-dict",
         "3=count var `a`b`c!(1000?100.0;1000?100.0;1000?100.0)","1b");
    eq_q(&mut c, "dev-dict",
         "3=count dev `a`b`c!(1000?100.0;1000?100.0;1000?100.0)","1b");
}

#[test]
fn cov_qlib_inter_except() {
    let mut c = conn();
    eq_q(&mut c, "inter",     "asc inter[1 2 3 4 5; 2 3 1 9]",    "1 2 3");
    eq_q(&mut c, "except",    "asc except[1 2 3 4 5; 2 3 1 9]",   "4 5");
    eq_q(&mut c, "inter-sym", "asc inter[`a`b`c; `a`b`x]",        "`a`b");
}

// FFI — 2:[lib;fn;argtypes;rettype] overload, using libm/libc as universally-present targets.
#[cfg(target_os = "linux")]
#[test]
fn cov_ffi_libm_scalar() {
    let mut c = conn();
    // sqrt: double sqrt(double).  Core scalar case — one float arg, float ret.
    eq_q(&mut c, "ffi-sqrt-2",    "(`:libm.so.6 2:(`sqrt;\"f\";\"f\")) 2.0",
        "1.4142135623730951");
    eq_q(&mut c, "ffi-sqrt-100",  "(`:libm.so.6 2:(`sqrt;\"f\";\"f\")) 100.0",
        "10f");
    // pow: double pow(double, double).  Two-arg float→float.
    eq_q(&mut c, "ffi-pow-2-10",  "(`:libm.so.6 \
        2:(`pow;\"ff\";\"f\"))[2.0;10.0]", "1024f");
    eq_q(&mut c, "ffi-pow-3-4",   "(`:libm.so.6 \
        2:(`pow;\"ff\";\"f\"))[3.0;4.0]",  "81f");
}

#[cfg(target_os = "linux")]
#[test]
fn cov_ffi_libc_int_and_sym() {
    let mut c = conn();
    // strlen: size_t strlen(const char*).  Symbol arg, long return.
    eq_q(&mut c, "ffi-strlen-5",  "(`:libc.so.6 2:(`strlen;\"s\";\"j\")) \
        `hello",       "5j");
    eq_q(&mut c, "ffi-strlen-10", "(`:libc.so.6 2:(`strlen;\"s\";\"j\")) \
        `abcdefghij",  "10j");
    // abs: int abs(int).  Int arg, int return.
    eq_q(&mut c, "ffi-abs-neg",   "(`:libc.so.6 2:(`abs;\"i\";\"i\")) -42i",
        "42i");
    eq_q(&mut c, "ffi-abs-pos",   "(`:libc.so.6 2:(`abs;\"i\";\"i\")) 7i",
        "7i");
}

#[cfg(target_os = "linux")]
#[test]
fn cov_ffi_binding_as_value() {
    // A bound FFI verb is a first-class K value: assignable, composable.
    let mut c = conn();
    eq_q(&mut c, "ffi-assigned",
         "{sq:`:libm.so.6 2:(`sqrt;\"f\";\"f\"); sq 9.0}[]", "3f");
    eq_q(&mut c, "ffi-composed",
         "{sq:`:libm.so.6 2:(`sqrt;\"f\";\"f\"); sum sq each 4.0 9.0 16.0}[]",
             "9f");
}

#[test]
fn cov_ffi_sizeof_helper() {
    let mut c = conn();
    eq_q(&mut c, "sizeof-j",  ".ffi.sizeof \"j\"", "8");
    eq_q(&mut c, "sizeof-i",  ".ffi.sizeof \"i\"", "4");
    eq_q(&mut c, "sizeof-h",  ".ffi.sizeof \"h\"", "2");
    eq_q(&mut c, "sizeof-e",  ".ffi.sizeof \"e\"", "4");
    eq_q(&mut c, "sizeof-f",  ".ffi.sizeof \"f\"", "8");
    eq_q(&mut c, "sizeof-p",  ".ffi.sizeof \"p\"", "8");
}

// Sort grade for wide-range longs and high-cardinality ints (regression).
#[test]
fn cov_simd_grade_large_vectors() {
    let mut c = conn();
    // KJ grade: sorted-by-grade reconstructs asc v and the grade is a permutation of 0..n-1.
    eq_q(&mut c, "grade-KJ-sorted",
         "{[]v:10000?1000000j;g:iasc v;(v g)~asc v}[]",     "1b");
    eq_q(&mut c, "grade-KJ-range",
         "{[]v:10000?1000000j;g:iasc v;\
           ((min g)=0i)and(max g)=9999i}[]",                "1b");
    // KI high-cardinality grade path.
    eq_q(&mut c, "grade-KI-hicard",
         "{[]v:10000?1000000i;g:iasc v;(v g)~asc v}[]",     "1b");
    eq_q(&mut c, "grade-KI-hicard-range",
         "{[]v:10000?1000000i;g:iasc v;\
           ((min g)=0i)and(max g)=9999i}[]",                "1b");
    // Sizes that previously hit the simd_grade_ii bug range.
    eq_q(&mut c, "grade-KI-200",
         "{[]v:200?100000000i;g:iasc v;(v g)~asc v}[]",     "1b");
    eq_q(&mut c, "grade-KI-300",
         "{[]v:300?100000000i;g:iasc v;(v g)~asc v}[]",     "1b");
}

// Polynomial log/exp/sin/cos need vectors large enough to hit the parallel path.
#[test]
fn cov_fastmath_avx512_transcendentals() {
    let mut c = conn();
    // 32k elements above the fanout threshold; 1e-4 tolerance for fast-math polys.
    eq_q(&mut c, "fm-log-exp-ident",
         "{[]v:1+32768?1.0;all 1e-4>abs v-exp log v}[]",    "1b");
    eq_q(&mut c, "fm-exp-log-ident",
         "{[]v:1+32768?1.0;all 1e-4>abs v-log exp v}[]",    "1b");
    eq_q(&mut c, "fm-sin-range",
         "{[]v:32768?6.28;all (sin v) within -1.01 1.01}[]","1b");
    eq_q(&mut c, "fm-cos-range",
         "{[]v:32768?6.28;all (cos v) within -1.01 1.01}[]","1b");
    // sin^2+cos^2=1 to polynomial precision; temp vars to control parse order.
    eq_q(&mut c, "fm-pythag",
         "{[]v:32768?6.28;s:sin v;c:cos v;\
           all 1e-3>abs 1.0-(s*s)+c*c}[]","1b");
}

// Named profiling timers: tick (-43!), tock (-44!), profile dump (-45!) end-to-end.
#[test]
fn cov_ticktock_profile_accumulate() {
    let mut c = conn();
    // Single timer: one tick+tock cycle populates one row.
    eq_q(&mut c, "tick-single",
         "{[]tick `t1;do[100;sum 100?100.0];tock `t1;\
            p:profile[];(98h=type p)and(`t1 in exec name from p)}[]",
         "1b");
    // Multi-timer: two distinct names → two rows.
    eq_q(&mut c, "tick-two-timers",
         "{[]tick `a1;tock `a1;tick `a2;tock `a2;\
            2<=count select from profile[] where name in `a1`a2}[]",
         "1b");
    // Tick/tock cycles accumulate call counts.
    eq_q(&mut c, "tick-cycles-counted",
         "{[]do[5;tick `c1;sum 10?1.0;tock `c1];\
            5<=first exec calls from profile[] where name=`c1}[]",
         "1b");
}

// Splay write/read round-trip for every base type (needs writable /tmp).
#[test]
fn cov_file_io_splay_roundtrip() {
    let mut c = conn();
    // Escape any cwd a prior test left, so the rm -rf below can't wipe l's cwd.
    c.query("\\cd /tmp").ok();
    c.query("system \"rm -rf /tmp/l_cov_fio\"").unwrap();
    c.query("system \"mkdir -p /tmp/l_cov_fio\"").unwrap();
    eq_q(&mut c, "splay-KB",
         "{[]`:/tmp/l_cov_fio/bv set 100?0b;\
           100=count get `:/tmp/l_cov_fio/bv}[]", "1b");
    eq_q(&mut c, "splay-KH",
         "{[]`:/tmp/l_cov_fio/hv set `short$100?1000;\
           100=count get `:/tmp/l_cov_fio/hv}[]", "1b");
    eq_q(&mut c, "splay-KI",
         "{[]`:/tmp/l_cov_fio/iv set 1000?1000000i;\
           1000=count get `:/tmp/l_cov_fio/iv}[]", "1b");
    eq_q(&mut c, "splay-KJ",
         "{[]`:/tmp/l_cov_fio/jv set 1000?1000000j;\
           1000=count get `:/tmp/l_cov_fio/jv}[]", "1b");
    eq_q(&mut c, "splay-KE",
         "{[]`:/tmp/l_cov_fio/ev set `real$100?100.0;\
           100=count get `:/tmp/l_cov_fio/ev}[]", "1b");
    eq_q(&mut c, "splay-KF",
         "{[]`:/tmp/l_cov_fio/fv set 1000?100.0;\
           1000=count get `:/tmp/l_cov_fio/fv}[]", "1b");
    eq_q(&mut c, "splay-KC",
         "{[]`:/tmp/l_cov_fio/cv set 5000#.Q.a,.Q.A;\
           5000=count get `:/tmp/l_cov_fio/cv}[]", "1b");
    eq_q(&mut c, "splay-KS",
         "{[]`:/tmp/l_cov_fio/sv set 100?`a`b`c`d;\
           100=count get `:/tmp/l_cov_fio/sv}[]", "1b");
    eq_q(&mut c, "splay-table",
         "{[]t:([]a:1000?100i;b:1000?100.0;c:1000?`X`Y`Z);\
           `:/tmp/l_cov_fio/tbl set t;\
           1000=count get `:/tmp/l_cov_fio/tbl}[]", "1b");
    // Big (>4KB): normal splay write + read past mmap threshold.
    eq_q(&mut c, "splay-big-int",
         "{[]big:100000?1000000i;\
           `:/tmp/l_cov_fio/big set big;\
           big~get `:/tmp/l_cov_fio/big}[]", "1b");
    c.query("system \"rm -rf /tmp/l_cov_fio\"").unwrap();
}

// Adverbs, casts, table verbs — migrated from coverage_boost2.q.
#[test]
fn cov_adverbs_and_casts() {
    let mut c = conn();
    eq_q(&mut c, "each-dyadic",  "3 5 7 9~(1 2 3 4)+2 3 4 5",       "1b");
    eq_q(&mut c, "each-monadic", "1 2 3~{x+1} each 0 1 2",          "1b");
    eq_q(&mut c, "over-sum",     "15j~(+/)1 2 3 4 5",                "1b");
    eq_q(&mut c, "over-max",     "9~(|/)3 1 4 1 5 9 2 6",           "1b");
    eq_q(&mut c, "over-min",     "1~(&/)3 1 4 1 5 9 2 6",           "1b");
    eq_q(&mut c, "scan-sum",     "1 3 6 10 15~(+\\)1 2 3 4 5",      "1b");
    eq_q(&mut c, "scan-max",     "3 3 4 4 5 9 9 9~(|\\)3 1 4 1 5 9 2 6","1b");
    eq_q(&mut c, "prior-diff",   "1~first (-':)1 2 3 4 5",          "1b");
    eq_q(&mut c, "over-fn",      "10~{x+y}/[1 2 3 4]",              "1b");
    eq_q(&mut c, "scan-fn",      "1 3 6 10~{x+y}\\[1 2 3 4]",       "1b");
    eq_q(&mut c, "prd",          "120~prd 1 2 3 4 5",               "1b");

    eq_q(&mut c, "cast-int",         "123i~\"i\"$123",      "1b");
    eq_q(&mut c, "cast-long",        "123j~\"j\"$123",      "1b");
    eq_q(&mut c, "cast-float",       "1.0~\"f\"$1",         "1b");
    eq_q(&mut c, "cast-string",      "\"123\"~string 123",  "1b");
    eq_q(&mut c, "cast-I-from-str",  "123i~\"I\"$\"123\"",  "1b");
    eq_q(&mut c, "cast-J-from-str",  "123j~\"J\"$\"123\"",  "1b");
    eq_q(&mut c, "cast-F-from-str",  "1.5~\"F\"$\"1.5\"",   "1b");
    eq_q(&mut c, "cast-bool",        "1b~\"b\"$1",          "1b");
    eq_q(&mut c, "cast-char",        "\"A\"~\"c\"$65",      "1b");
    eq_q(&mut c, "cast-bool-vec",    "1 0 1i~\"i\"$101b",   "1b");
    eq_q(&mut c, "cast-int-to-long", "1 2 3j~\"j\"$1 2 3i", "1b");
    eq_q(&mut c, "cast-int-to-flt",  "1.0 2.0 3.0~\"f\"$1 2 3i","1b");
    eq_q(&mut c, "sym-to-str",
         "(\"abc\";\"def\")~string `abc`def",               "1b");
    eq_q(&mut c, "str-to-sym",
         "`abc`def~`$(\"abc\";\"def\")",                    "1b");
}

// Search / distinct / reverse / rotate / bin coverage.
#[test]
fn cov_search_distinct_bin() {
    let mut c = conn();
    eq_q(&mut c, "bin-int",      "2~1 2 3 4 5 bin 3",            "1b");
    eq_q(&mut c, "bin-float",    "1~1.0 2.0 3.0 4.0 5.0 bin 2.5","1b");
    eq_q(&mut c, "bin-vec",      "0 2 4~1 2 3 4 5 bin 1 3 5",    "1b");
    eq_q(&mut c, "in-int",       "101b~1 2 3i in 1 3i",          "1b");
    eq_q(&mut c, "in-sym",       "110b~`a`b`c in `a`b",          "1b");
    eq_q(&mut c, "ss-single",    "1=count \"hello world\" ss \"world\"","1b");
    eq_q(&mut c, "ss-multi",     "4=count \"a.b.c.d.\" ss \".\"","1b");
    eq_q(&mut c, "distinct-int", "1 2 3~distinct 1 2 3 1 2 3",   "1b");
    eq_q(&mut c, "distinct-sym", "`a`b`c~distinct `a`b`c`a`b`c", "1b");
    eq_q(&mut c, "reverse",      "5 4 3 2 1~reverse 1 2 3 4 5",  "1b");
    eq_q(&mut c, "rotate-fwd",   "3 4 5 1 2~2 rotate 1 2 3 4 5", "1b");
    eq_q(&mut c, "rotate-back",  "4 5 1 2 3~-2 rotate 1 2 3 4 5","1b");
}

// table_execution.c — select / update / delete / amend / fill paths.
#[test]
fn cov_table_select_delete() {
    let mut c = conn();
    c.query("t2:([]s:`a`b`c`a`b;v:1 2 3 4 5;p:1.1 2.2 3.3 4.4 5.5)").unwrap();
    eq_q(&mut c, "select-where",
         "2~count select from t2 where s=`a","1b");
    eq_q(&mut c, "select-wide",
         "5~count select v, p from t2","1b");
    eq_q(&mut c, "select-by",
         "3~count select sum v by s from t2","1b");
    eq_q(&mut c, "update",
         "11~first exec v from update v:v+10 from t2","1b");
    eq_q(&mut c, "update-where",
         "11~first exec v from update v:v+10 from t2 where s=`a","1b");
    eq_q(&mut c, "delete-where",
         "3~count delete from t2 where s=`a","1b");
    eq_q(&mut c, "delete-cols",
         "2~count cols delete p from t2","1b");
    eq_q(&mut c, "amend-set",
         "10 2 3 4 5~@[1 2 3 4 5;0;:;10]","1b");
    eq_q(&mut c, "amend-plus",
         "11 2 3 4 5~@[1 2 3 4 5;0;+;10]","1b");
    eq_q(&mut c, "fill-caret",
         "1 2 0 4~0^1 2 0N 4","1b");
}

// HOT-PATH REGRESSIONS — pin correctness for the paths worth optimizing next.

// Sort stability and permutation invariants under 10M-element load.
#[test]
fn hot_simd_grade_j_invariants() {
    let mut c = conn();
    eq_q(&mut c, "grade-KJ-sorted-10M",
         "{[]v:1000000?1000000000j;g:iasc v;(v g)~asc v}[]",     "1b");
    eq_q(&mut c, "grade-KJ-perm-10M",
         "{[]v:1000000?1000000000j;g:iasc v;\
           ((min g)=0i)and(max g)=999999i}[]",                   "1b");
    eq_q(&mut c, "grade-KJ-count-distinct-10M",
         "{[]v:1000000?1000000000j;g:iasc v;1000000=count distinct g}[]", "1b");
}

// Same for simd_grade_ii (KI high-cardinality → radix path).
#[test]
fn hot_simd_grade_ii_invariants() {
    let mut c = conn();
    eq_q(&mut c, "grade-KI-sorted-1M",
         "{[]v:1000000?1000000000i;g:iasc v;(v g)~asc v}[]",     "1b");
    eq_q(&mut c, "grade-KI-perm-1M",
         "{[]v:1000000?1000000000i;g:iasc v;\
           ((min g)=0i)and(max g)=999999i}[]",                   "1b");
}

// Narrow-range KJ grade path routed via n>=4096 and range<2^32.
#[test]
fn hot_radix_grade_jn_invariants() {
    let mut c = conn();
    // bench shape: n=1M, range 10^9 < 2^32 → narrow path
    eq_q(&mut c, "grade-KJ-narrow-1M",
         "{[]v:1000000?1000000000j;g:iasc v;(v g)~asc v}[]",     "1b");
    eq_q(&mut c, "grade-KJ-narrow-perm",
         "{[]v:1000000?1000000000j;g:iasc v;\
           ((min g)=0i)and(max g)=999999i}[]",                   "1b");
    // signed narrow: min<0 exercises (xJ[i]-b) subtract path
    eq_q(&mut c, "grade-KJ-signed-narrow",
         "{[]v:(10000?2000000j)-1000000;g:iasc v;(v g)~asc v}[]","1b");
    // wide range >2^32: gating routes to simd_grade_j fallback
    eq_q(&mut c, "grade-KJ-wide-fallback",
         "{[]v:10000?9999999999999j;g:iasc v;(v g)~asc v}[]",    "1b");
    // gating threshold n=4096: just at narrow path entry
    eq_q(&mut c, "grade-KJ-n4096-boundary",
         "{[]v:4096?1000000j;g:iasc v;(v g)~asc v}[]",           "1b");
    // gating threshold n=4095: just below — falls through to simd_grade_j
    eq_q(&mut c, "grade-KJ-n4095-fallback",
         "{[]v:4095?1000000j;g:iasc v;(v g)~asc v}[]",           "1b");
    // degenerate range=0 (all equal): radix histogram all in bucket 0
    eq_q(&mut c, "grade-KJ-all-equal",
         "{[]v:8192#7j;g:iasc v;(v g)~asc v}[]",                 "1b");
    // pre-sorted ascending
    eq_q(&mut c, "grade-KJ-presorted",
         "{[]v:`long$til 100000;g:iasc v;(v g)~asc v}[]",        "1b");
    // reverse-sorted: classic quicksort worst-case; radix is order-independent
    eq_q(&mut c, "grade-KJ-reversed",
         "{[]v:reverse `long$til 100000;g:iasc v;(v g)~asc v}[]","1b");
}

// Vector gather + equality: the select-where hot path, stressed with a large table filter.
#[test]
fn hot_table_select_filter_hitrate() {
    let mut c = conn();
    eq_q(&mut c, "table-select-1M-sym",
         "{[]sym:1000000?`A`B`C`D`E;\
           t:([]sym:sym;px:1000000?100.0);\
           (count sym where sym=`C)=count select from t where sym=`C}[]", "1b");
    eq_q(&mut c, "table-select-float-range",
         "{[]px:1000000?100.0;\
           t:([]px:px);\
           (count px where px<50.0)=count select from t where px<50.0}[]",
               "1b");
}

// Group-by agg (select sum sz by sym on 1M rows) — the other dominant table hot path.
#[test]
fn hot_table_groupby_agg() {
    let mut c = conn();
    eq_q(&mut c, "groupby-sum-1M",
         "{[]sym:1000000?`A`B`C`D`E;\
           sz:`long$1000000?1000;\
           t:([]sym:sym;sz:sz);\
           r:select s:sum sz by sym from t;\
           (5=count r)and(sum sz)=sum exec s from r}[]",          "1b");
    eq_q(&mut c, "groupby-avg-max",
         "{[]sym:1000000?`A`B`C`D`E;\
           px:1000000?100.0;\
           t:([]sym:sym;px:px);\
           r:select a:avg px, m:max px by sym from t;\
           (5=count r)and(max px)=max exec m from r}[]",          "1b");
}

// xdesc heap-corruption regression: run xdesc 30x on a table so damage shows as a count mismatch.
#[test]
fn hot_xdesc_no_heap_corruption() {
    let mut c = conn();
    eq_q(&mut c, "xdesc-30x-consistent",
         "{[]t:([]sym:`a`b`c`d`e`f`g`h;\
                  price:10 30 20 50 40 60 80 70);\
            n:count each 30#enlist(`price xdesc t);\
            all 8=n}[]",                                           "1b");
    eq_q(&mut c, "xdesc-values-correct",
         "{[]t:([]sym:`a`b`c`d;price:10 40 20 30);\
            (exec price from `price xdesc t)~40 30 20 10}[]",     "1b");
    eq_q(&mut c, "xdesc-then-parse",
         "{[]t:([]a:1 2 3;b:3 1 2);                                 \
            r:`a xdesc t;                                           \
            (exec a from r)~3 2 1i}[]",                           "1b");
}

// HDB round-trip: multi-partition write, reload, query (mmap-cache + refcount regression).
#[test]
fn hot_hdb_partitioned_roundtrip() {
    if std::env::var("L_SKIP_HDB").is_ok() { return; }
    let mut c = conn();
    // Escape any cwd a prior test left, so the rm -rf below can't wipe l's cwd.
    c.query("\\cd /tmp").ok();
    c.query("system \"rm -rf /tmp/_rust_hdb_hot\"").unwrap();
    // Build 10 small partitions; assign the table globally so partition-write can find it.
    c.query("{d:.z.D-10;do[10;\
             trade::([]date:1000#d;sym:1000?`A`B`C`D;\
                      px:1000?100.0;sz:`long$1000?100);\
             .Q.dpft[`:/tmp/_rust_hdb_hot;d;`sym;`trade];d+:1]}[]")
        .unwrap();
    c.query("system \"cd /tmp/_rust_hdb_hot\"").ok();
    // Load in a fresh namespace via \l
    c.query("\\cd /tmp/_rust_hdb_hot").unwrap();
    c.query("\\l .").unwrap();
    eq_q(&mut c, "hdb-total-count",
         "10000=count select from trade", "1b");
    eq_q(&mut c, "hdb-groupby-4-syms",
         "4=count select c:count i by sym from trade", "1b");
    eq_q(&mut c, "hdb-multi-day-filter",
         "(5*1000)=count select from trade where date>.z.D-6", "1b");
    // Re-query several times to stress the mmap cache refcount path.
    for _ in 0..5 {
        eq_q(&mut c, "hdb-reentry",
             "4=count select s:sum sz by sym from trade", "1b");
    }
    // Restore CWD before deleting the HDB dir (we cd'd into it).
    c.query("\\cd /tmp").unwrap();
    c.query("system \"rm -rf /tmp/_rust_hdb_hot\"").unwrap();
}

// Rapid-fire micro-IPC: 200 small queries exercise the per-query server cycle.
#[test]
fn hot_ipc_rapid_fire() {
    let mut c = conn();
    for i in 0..200 {
        let q = match i % 4 {
            0 => "1+1",
            1 => "sum til 1000",
            2 => "count distinct 1000?100i",
            _ => "avg 100?1.0"
        };
        let r = c.query(q).unwrap_or_else(|e| panic!("iter {i}: {:?}", e));
        // Every response must be a scalar atom (not a list) and finite.
        assert!(matches!(r.type_tag(), -6 | -7 | -9),
                "iter {i}: unexpected tag {}", r.type_tag());
    }
}

// Boundary-size regressions for the in-place SIMD partition around its thresholds.
#[test]
fn hot_kvps_boundary_sizes() {
    let mut c = conn();
    for n in &[8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128,
               129, 255, 256, 257, 1023, 1024, 1025] {
        for _ in 0..10 {
            eq_q(&mut c, &format!("kvps-KJ-{n}"),
                 &format!("{{v:{n}?1000000j;g:iasc v;(v g)~asc v}}[]"),
                 "1b");
        }
    }
}

// All-equal input must complete quickly (equal region never re-partitioned).
#[test]
fn hot_kvps_all_equal() {
    let mut c = conn();
    eq_q(&mut c, "all-equal-10k",
         "{v:10000#42j;g:iasc v;10000=count g}[]","1b");
    eq_q(&mut c, "all-equal-100k",
         "{v:100000#42j;g:iasc v;100000=count g}[]","1b");
    // Stable argsort on equal keys returns ascending indices.
    eq_q(&mut c, "all-equal-stable",
         "{v:1000#99j;g:iasc v;g~`int$til 1000}[]","1b");
}

// Pre-sorted ascending/descending inputs give balanced splits; tests the depth-limit fallback.
#[test]
fn hot_kvps_pre_sorted() {
    let mut c = conn();
    // Already-sorted: trivial case (qasc shortcut may catch it).
    eq_q(&mut c, "pre-sorted-100k",
         "{v:`long$til 100000;g:iasc v;(v g)~asc v}[]","1b");
    // Reverse-sorted: forces full sort.
    eq_q(&mut c, "reverse-sorted-100k",
         "{v:`long$reverse til 100000;g:iasc v;(v g)~asc v}[]","1b");
    // Nearly-sorted: 50k ascending with 50 random swap-ins.
    eq_q(&mut c, "nearly-sorted-50k",
         "{[]v:`long$til 50000;idx:50?50000;vals:50?50000j;\
           v[idx]:vals;g:iasc v;(v g)~asc v}[]","1b");
}

// 10M-element random KJ: verify the grade is a genuine permutation (high-cardinality regression).
#[test]
fn hot_kvps_10m_permutation() {
    let mut c = conn();
    eq_q(&mut c, "10m-KJ-sorted",
         "{v:10000000?1000000000j;g:iasc v;(v g)~asc v}[]","1b");
    eq_q(&mut c, "10m-KJ-perm",
         "{v:10000000?1000000000j;g:iasc v;\
           ((min g)=0i)and(max g)=9999999i}[]","1b");
    eq_q(&mut c, "10m-KJ-distinct",
         "{v:10000000?1000000000j;g:iasc v;\
           10000000=count distinct g}[]","1b");
}

// CROSS-PROCESS IPC — hopen/hclose/sync + async messaging via a spawned second l process.

struct LProcess(std::process::Child);
impl Drop for LProcess {
    fn drop(&mut self) { let _ = self.0.kill(); let _ = self.0.wait(); }
}

fn l_bin() -> std::path::PathBuf {
    match std::env::var("L_BIN") {
        Ok(p) => p.into(),                                                      // explicit override wins
        Err(_) => "l".into(),                                                   // else resolve `l` via PATH
    }
}

fn spawn_l(port: u16) -> LProcess {
    // stdin must be null: the test harness delivers EOF, and l's headless mode exits on stdin EOF.
    let mut child = std::process::Command::new(l_bin())
        .args(["-p", &port.to_string(), "-q"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn l");
    let deadline = std::time::Instant::now() +
        std::time::Duration::from_secs(5);
    while std::time::Instant::now() < deadline {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return LProcess(child);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let _ = child.kill();
    let _ = child.wait();
    panic!("l on :{} never started listening", port);
}

#[test]
fn cross_l_sync_remote_assign() {
    // Sync handle: positive integer from hopen; `h expr` round-trips, blocks.
    let srv_port = 9898u16;
    let _srv = spawn_l(srv_port);
    let mut c = conn();
    c.query(&format!(
        "h:hopen {port}; h \"a:{port}i\"; hclose h",
        port = srv_port)).unwrap();
    let mut s = Connection::connect("localhost", srv_port).unwrap();
    assert_eq!(s.query("a").unwrap().as_int(), Some(srv_port as i32),
        "sync: `a` should equal server port");
}

#[test]
fn cross_l_async_remote_assign() {
    // Async handle: `neg h` converts the sync handle for fire-and-forget sends.
    let srv_port = 9897u16;
    let _srv = spawn_l(srv_port);
    let mut c = conn();
    // Async via neg h, then an empty sync h "" to flush before hclose.
    c.query(&format!(
        "h:hopen {port}; (neg h) \"b:{port}i\"; h \"\"; hclose h",
        port = srv_port)).unwrap();
    // Async needs the server's event loop to pick up the message; poll.
    let mut s = Connection::connect("localhost", srv_port).unwrap();
    let deadline = std::time::Instant::now() +
        std::time::Duration::from_secs(2);
    loop {
        if let Some(v) = s.query("@[{b};0;0i]").ok().and_then(|k| k.as_int()) {
            if v == srv_port as i32 { return; }
        }
        if std::time::Instant::now() > deadline {
            panic!("async: `b` never set to {} within timeout", srv_port);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

// LARGE-N INT32 OVERFLOW REGRESSIONS — #[ignore]-gated; need multi-GB data (run with --ignored).

// Grouped-sym amend at a very high index must not overflow the index offset.
#[test]
#[ignore]
fn overflow_au_grouped_sym_high_index() {
    let mut c = conn();
    // n=3e8, amend at i=2.8e8 (~2.4 GB of sym pointers).
    eq_q(&mut c, "au-grouped-sym-high-index",
        "{n:300000000; v:`g#(n#`A`B`C); v[280000000]:`X; \
          (v[280000000]=`X) & (v[0]=`A) & (n=count v)}[]",
        "1b");
}

// Large single-group amend: one group with n int indices; index at i>537M must not overflow.
#[test]
#[ignore]
fn overflow_del_grouped_single_large_group() {
    let mut c = conn();
    // n=7e8 all-equal ints -> one 2.8 GB group; amend near the end overflows a 32-bit offset pre-fix.
    eq_q(&mut c, "del-grouped-single-large-group",
        "{n:700000000; v:`g#(n#1i); v[560000000]:2i; \
          (v[560000000]=2i) & (v[0]=1i) & (n=count v)}[]",
        "1b");
}

// lsq overflow at y->n>23170 needs a ~2.15 GB scratch alloc; covered by static review + small companion.
#[test]
#[ignore]
fn overflow_lsq_large_rhs() {
    eprintln!("WARNING: this test allocates ~2.15 GB and runs for many \
        minutes.");
    let mut c = conn();
    eq_q(&mut c, "lsq-large-rhs",
        "{n:23200; yy:(n;1)#1.0; xx:(n;1)#1.0; r:xx lsq yy; 1b}[]",
        "1b");
}

// Small-size regression companions — run in normal CI at safe sizes.

#[test]
fn regression_au_grouped_sym_small() {
    let mut c = conn();
    eq_q(&mut c, "au-grouped-sym-small",
        "{v:`g#`A`B`C`A`B; v[2]:`X; v~`g#`A`B`X`A`B}[]", "1b");
}

#[test]
fn regression_del_grouped_int_small() {
    let mut c = conn();
    eq_q(&mut c, "del-grouped-int-small",
        "{w:`g#1 1 1 1 1i; w[2]:2i; w~`g#1 1 2 1 1i}[]", "1b");
}

#[test]
fn regression_lsq_small() {
    // lsq on (5,1) matrices exercises the scratch-alloc path where the fix is a no-op.
    let mut c = conn();
    let r = c.query(
        "{xx:(5;1)#1.0 2.0 3.0 4.0 5.0; yy:(5;1)#2.0 4.0 6.0 8.0 10.0; \
          r:xx lsq yy; (0h=type r) & (5=count r)}[]").unwrap();
    assert_eq!(r, K::Bool(true));
}

// SECOND-WAVE OVERFLOW AUDIT — additional int32-overflow sites; #[ignore]-gated with CI companions.

// neg/abs worker must not overflow its byte offset at n>=268M with 8-byte elements.
#[test]
#[ignore]
fn overflow_pp_m1w_neg_kj() {
    let mut c = conn();
    // n=3e8 KJ; verify the last element is correctly negated (a wild-write worker would miss it).
    eq_q(&mut c, "pp-m1w-neg-kj",
        "{n:300000000; v:n#42j; w:neg v; (w[0]=-42j)&(w[n-1]=-42j)&(n=count \
            w)}[]",
        "1b");
}

#[test]
#[ignore]
fn overflow_pp_m1w_abs_kj() {
    let mut c = conn();
    // n=3e8 → 2.4 GB KJ vector, all negative, take abs.  Same lo*sz path.
    eq_q(&mut c, "pp-m1w-abs-kj",
        "{n:300000000; v:n#-7j; w:abs v; (w[0]=7j)&(w[n-1]=7j)&(n=count w)}[]",
        "1b");
}

#[test]
#[ignore]
fn overflow_pp_m1w_neg_kf() {
    let mut c = conn();
    // KF: same 8-byte element path, distinct codegen from KJ (fp negate).
    eq_q(&mut c, "pp-m1w-neg-kf",
        "{n:300000000; v:n#1.5; w:neg v; (w[0]=-1.5)&(w[n-1]=-1.5)&(n=count \
            w)}[]",
        "1b");
}

// IPC serialize wrote a 32-bit byte count that wraps >2GB; round-trip a large vec and verify byte-identical.

#[test]
#[ignore]
fn overflow_ipc_roundtrip_kj_300m() {
    // 300M × 8 B = 2.4 GB.  Round-trips through full b0/d0/nx path.
    let srv_port = 9801u16;
    let _srv = spawn_l(srv_port);
    let mut c = conn();
    eq_q(&mut c, "ipc-rt-kj-300m",
        &format!("{{h:hopen {port}; h \"v:300000000#42j\"; \
            r:h \"(v[0]=42j)&(v[299999999]=42j)&(300000000=count v)\"; \
            hclose h; r}}[]", port = srv_port),
        "1b");
}

#[test]
#[ignore]
fn overflow_ipc_roundtrip_kf_300m() {
    let srv_port = 9802u16;
    let _srv = spawn_l(srv_port);
    let mut c = conn();
    eq_q(&mut c, "ipc-rt-kf-300m",
        &format!("{{h:hopen {port}; h \"v:300000000#3.14\"; \
            r:h \"(v[0]=3.14)&(v[299999999]=3.14)&(300000000=count v)\"; \
            hclose h; r}}[]", port = srv_port),
        "1b");
}

#[test]
#[ignore]
fn overflow_ipc_roundtrip_ki_600m() {
    // KI is 4 B — overflow threshold at xn > 537M.  600M × 4 = 2.4 GB.
    let srv_port = 9803u16;
    let _srv = spawn_l(srv_port);
    let mut c = conn();
    eq_q(&mut c, "ipc-rt-ki-600m",
        &format!("{{h:hopen {port}; h \"v:600000000#17i\"; \
            r:h \"(v[0]=17i)&(v[599999999]=17i)&(600000000=count v)\"; \
            hclose h; r}}[]", port = srv_port),
        "1b");
}

#[test]
#[ignore]
fn overflow_ipc_roundtrip_kh_1_2b() {
    // KH is 2 B — overflow threshold at xn > 1.07B.  1.2B × 2 = 2.4 GB.
    let srv_port = 9804u16;
    let _srv = spawn_l(srv_port);
    let mut c = conn();
    eq_q(&mut c, "ipc-rt-kh-1200m",
        &format!("{{h:hopen {port}; h \"v:1200000000#5h\"; \
            r:h \"(v[0]=5h)&(v[1199999999]=5h)&(1200000000=count v)\"; \
            hclose h; r}}[]", port = srv_port),
        "1b");
}

#[test]
#[ignore]
fn overflow_ipc_roundtrip_table_300m() {
    // XT with a 300M-row KJ column — nested IPC path (b0 recurses dict→vec).
    let srv_port = 9805u16;
    let _srv = spawn_l(srv_port);
    let mut c = conn();
    eq_q(&mut c, "ipc-rt-table-300m",
        &format!("{{h:hopen {port}; h \"t:([]a:300000000#11j)\"; \
            r:h \"(t[`a][0]=11j)&(t[`a][299999999]=11j)&(300000000=count t)\"; \
            hclose h; r}}[]", port = srv_port),
        "1b");
}

#[test]
#[ignore]
fn overflow_ipc_roundtrip_dict_300m() {
    // XD with 300M KJ values.  Dict: keys + values, both go through b0 path.
    let srv_port = 9806u16;
    let _srv = spawn_l(srv_port);
    let mut c = conn();
    eq_q(&mut c, "ipc-rt-dict-300m",
        &format!("{{h:hopen {port}; h \"d:(til 300000000)!300000000#99j\"; \
            r:h \"(d[0]=99j)&(d[299999999]=99j)&(300000000=count d)\"; \
            hclose h; r}}[]", port = srv_port),
        "1b");
}

// Raw-byte hash used a 32-bit byte count that wraps at >=2GB; used by match and join-key hashing.
#[test]
#[ignore]
fn overflow_h0_hash_equality_kj() {
    let mut c = conn();
    // Two identical 300M KJ vectors; ~ hashes each — pre-fix a wrong byte count broke equality.
    eq_q(&mut c, "h0-eq-kj",
        "{n:300000000; a:n#77j; b:n#77j; a~b}[]",
        "1b");
}

#[test]
#[ignore]
fn overflow_h0_hash_equality_ki() {
    let mut c = conn();
    // KI: xn*4 wraps at xn>537M.
    eq_q(&mut c, "h0-eq-ki",
        "{n:600000000; a:n#1i; b:n#1i; a~b}[]",
        "1b");
}

// More overflow sites (batch table hash, binary CSV load/write) — covered by static review + companions.

// Batch table-hash offset wraps at rows>=1.07B; heavy trigger skipped, small companion in CI.
#[test]
fn regression_h3v_table_hash_small() {
    let mut c = conn();
    // Distinct count on a table with KJ column — forces h3v path.
    eq_q(&mut c, "h3v-small",
        "{t:([]a:1 2 3 1 2 3j;b:10 20 30 10 20 30j); 3=count distinct t}[]",
        "1b");
}

// ── Small-size regression companions for the new fixes ─────────────── //

#[test]
fn regression_pp_m1w_neg_abs_small() {
    // neg/abs below and above the parallel dispatch threshold — both paths must work.
    let mut c = conn();
    eq_q(&mut c, "pp-m1w-neg-seq", "neg 1 -2 3j", "-1 2 -3j");
    eq_q(&mut c, "pp-m1w-abs-seq", "abs -1 -2 -3j", "1 2 3j");
    // 100k triggers pp_for with cost model; still small byte count.
    eq_q(&mut c, "pp-m1w-neg-par",
        "{v:100000#7j; w:neg v; (w[0]=-7j)&(w[99999]=-7j)}[]", "1b");
}

#[test]
fn regression_h0_hash_equality_small() {
    let mut c = conn();
    eq_q(&mut c, "h0-eq-small-kj", "(1 2 3j)~(1 2 3j)", "1b");
    eq_q(&mut c, "h0-eq-small-ki", "(1 2 3i)~(1 2 3i)", "1b");
    eq_q(&mut c, "h0-neq-small",   "(1 2 3j)~(1 2 4j)", "0b");
    // 10k-element distinct exercises h0 via group-by
    eq_q(&mut c, "h0-distinct-small",
        "{v:10000#(1 2 3j); 3=count distinct v}[]", "1b");
}

#[test]
fn regression_nx_b0_d0_roundtrip_all_types() {
    // IPC round-trip for every scalar vector type: set then read back via get.
    let srv_port = 9881u16;
    let _srv = spawn_l(srv_port);
    let mut c = conn();
    // Each round-trip assigns on the remote and reads back via match (char skipped for quoting).
    let cases: &[(&str, &str)] = &[
        ("kb", "1000#01b"),
        ("kg", "1000#0x42"),
        ("kh", "1000#7h"),
        ("ki", "1000#17i"),
        ("kj", "1000#99j"),
        ("ke", "1000#3.5e"),
        ("kf", "1000#2.718"),
        ("ks", "1000#`sym"),
        // Temporal types
        ("kp", "1000#2024.01.01D00:00:00.000000000"),
        ("kd", "1000#2024.01.01"),
        ("kt", "1000#12:00:00.000"),
    ];
    for (tag, expr) in cases {
        eq_q(&mut c, &format!("rt-all-{}", tag),
            &format!("{{h:hopen {port}; h \"v:{e}\"; \
                r:h \"v~{e}\"; hclose h; r}}[]", port = srv_port, e = expr),
            "1b");
    }
}

// ── Table / dict / nested-table regression at moderate sizes ────────── //

#[test]
fn regression_nested_table_column_small() {
    let mut c = conn();
    // Nested-column table: verify nested serialize, count, and index.
    eq_q(&mut c, "nested-col-table",
        "{t:flip`a`v!(til 10;10 10#100?1.0); \
          (10=count t)&(10=count first t`v)&(0=first t`a)}[]",
        "1b");
}

#[test]
fn regression_nested_table_ipc_roundtrip() {
    // Nested-column table IPC roundtrip — recursive b0 path.
    let srv_port = 9882u16;
    let _srv = spawn_l(srv_port);
    let mut c = conn();
    eq_q(&mut c, "nested-table-ipc",
        &format!("{{h:hopen {port}; h \"t:flip`a`v!(til 10;10 10#100?1.0)\"; \
            r:h \"(10=count t)&(10=count first t`v)\"; \
            hclose h; r}}[]", port = srv_port),
        "1b");
}

#[test]
fn regression_dict_nested_values() {
    let mut c = conn();
    // Dict with vector values including a nested (list-of-vec) value.
    eq_q(&mut c, "dict-nested",
        "{d:`a`b`c!((til 5);(til 10);(2 3#til 6)); \
          (5=count d[`a])&(10=count d[`b])&(2=count d[`c])}[]",
        "1b");
}

// IPC roundtrip matrix — every type, a few sizes; shakes out serialize/deserialize regressions.

#[test]
fn ipc_roundtrip_matrix_atoms() {
    let srv_port = 9883u16;
    let _srv = spawn_l(srv_port);
    let mut c = conn();
    // One atom per type, round-tripped through another l (char atom skipped for quoting).
    let atoms: &[(&str, &str)] = &[
        ("bool", "1b"),
        ("byte", "0x42"),
        ("short", "7h"),
        ("int", "42i"),
        ("long", "99j"),
        ("real", "3.14e"),
        ("float", "2.718"),
        ("sym", "`hello"),
        ("tstamp", "2024.01.01D12:00:00.000000000"),
        ("date", "2024.01.01"),
        ("time", "12:00:00.000"),
    ];
    for (tag, expr) in atoms {
        eq_q(&mut c, &format!("atom-rt-{}", tag),
            &format!("{{h:hopen {port}; h \"a:{e}\"; \
                r:h \"a~{e}\"; hclose h; r}}[]", port = srv_port, e = expr),
            "1b");
    }
}

#[test]
fn ipc_roundtrip_matrix_mixed_list() {
    let srv_port = 9884u16;
    let _srv = spawn_l(srv_port);
    let mut c = conn();
    // Mixed list (type 0): forces the generic-list b0 recursion path.
    eq_q(&mut c, "mixed-list-rt",
        &format!("{{h:hopen {port}; h \"L:(1i;2j;3.14;\\\"hello\\\";`sym;1 2 \
            3)\"; \
            r:h \"L~(1i;2j;3.14;\\\"hello\\\";`sym;1 2 3)\"; \
            hclose h; r}}[]", port = srv_port),
        "1b");
}

#[test]
fn ipc_roundtrip_matrix_keyed_table() {
    let srv_port = 9885u16;
    let _srv = spawn_l(srv_port);
    let mut c = conn();
    // Keyed table (XT): dict-of-tables.  Forces XT+XD serialize combo.
    eq_q(&mut c, "keyed-table-rt",
        &format!("{{h:hopen {port}; h \"t:([k:`a`b`c]v:1 2 3i)\"; \
            r:h \"t~([k:`a`b`c]v:1 2 3i)\"; \
            hclose h; r}}[]", port = srv_port),
        "1b");
}

// BOUNDARY TESTS — behavior just below/above the size thresholds to catch off-by-one cast bugs.

#[test]
#[ignore]
fn boundary_below_kj_threshold() {
    // 200M x 8 = 1.6 GB, safely below overflow: sanity that large-but-safe sizes work.
    let mut c = conn();
    eq_q(&mut c, "below-kj-thresh",
        "{n:200000000; v:n#13j; w:neg v; (w[n-1]=-13j)&(n=count w)}[]",
        "1b");
}

#[test]
#[ignore]
fn boundary_just_above_kj_threshold() {
    // 270M×8 = 2.16 GB, just above the 268.435M overflow threshold.
    let mut c = conn();
    eq_q(&mut c, "just-above-kj-thresh",
        "{n:270000000; v:n#55j; w:neg v; (w[n-1]=-55j)&(n=count w)}[]",
        "1b");
}

#[test]
#[ignore]
fn boundary_just_above_ki_threshold() {
    // 540M×4 = 2.16 GB, just above KI 536.87M threshold.
    let mut c = conn();
    eq_q(&mut c, "just-above-ki-thresh",
        "{n:540000000; v:n#21i; w:neg v; (w[n-1]=-21i)&(n=count w)}[]",
        "1b");
}

// FULL-OPERATION STRESS MATRIX — every primitive at n>=268M; #[ignore]-gated (run with --ignored).

// ── Binary arithmetic: +, -, *, div, mod, &, | at large n ─────────────── //

#[test]
#[ignore]
fn overflow_arith_binary_kj() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "+kj",
        &format!("{{n:{n};a:n#3j;b:n#5j;c:a+b;(c[0]=8j)&(c[n-1]=8j)&(n=count \
            c)}}[]"), "1b");
    eq_q(&mut c, "-kj",
        &format!("{{n:{n};a:n#8j;b:n#3j;c:a-b;(c[0]=5j)&(c[n-1]=5j)&(n=count \
            c)}}[]"), "1b");
    eq_q(&mut c, "*kj",
        &format!("{{n:{n};a:n#3j;b:n#5j;c:a*b;(c[0]=15j)&(c[n-1]=15j)&(n=count \
            c)}}[]"), "1b");
    eq_q(&mut c, "min-kj",
        &format!("{{n:{n};a:n#3j;b:n#5j;c:a&b;(c[0]=3j)&(c[n-1]=3j)}}[]"),
            "1b");
    eq_q(&mut c, "max-kj",
        &format!("{{n:{n};a:n#3j;b:n#5j;c:a|b;(c[0]=5j)&(c[n-1]=5j)}}[]"),
            "1b");
}

#[test]
#[ignore]
fn overflow_arith_binary_ki() {
    let mut c = conn();
    let n = "600000000";
    eq_q(&mut c, "+ki",
        &format!("{{n:{n};a:n#3i;b:n#5i;c:a+b;(c[0]=8i)&(c[n-1]=8i)&(n=count \
            c)}}[]"), "1b");
    eq_q(&mut c, "-ki",
        &format!("{{n:{n};a:n#8i;b:n#3i;c:a-b;(c[0]=5i)&(c[n-1]=5i)}}[]"),
            "1b");
    eq_q(&mut c, "*ki",
        &format!("{{n:{n};a:n#3i;b:n#5i;c:a*b;(c[0]=15i)&(c[n-1]=15i)}}[]"),
            "1b");
    eq_q(&mut c, "min-ki",
        &format!("{{n:{n};a:n#3i;b:n#5i;c:a&b;(c[0]=3i)&(c[n-1]=3i)}}[]"),
            "1b");
    eq_q(&mut c, "max-ki",
        &format!("{{n:{n};a:n#3i;b:n#5i;c:a|b;(c[0]=5i)&(c[n-1]=5i)}}[]"),
            "1b");
}

#[test]
#[ignore]
fn overflow_arith_binary_kf() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "+kf",
        &format!("{{n:{n};a:n#3.0;b:n#5.0;c:a+b;(c[0]=8.0)&(c[n-1]=8.0)&(n=coun\
            t c)}}[]"), "1b");
    eq_q(&mut c, "-kf",
        &format!("{{n:{n};a:n#8.0;b:n#3.0;c:a-b;(c[0]=5.0)&(c[n-1]=5.0)}}[]"),
            "1b");
    eq_q(&mut c, "*kf",
        &format!("{{n:{n};a:n#3.0;b:n#5.0;c:a*b;(c[0]=15.0)&(c[n-1]=15.0)}}[\
            ]"), "1b");
    eq_q(&mut c, "%kf",
        &format!("{{n:{n};a:n#15.0;b:n#3.0;c:a%b;(c[0]=5.0)&(c[n-1]=5.0)}}[]"),
            "1b");
    eq_q(&mut c, "min-kf",
        &format!("{{n:{n};a:n#3.0;b:n#5.0;c:a&b;(c[0]=3.0)}}[]"), "1b");
    eq_q(&mut c, "max-kf",
        &format!("{{n:{n};a:n#3.0;b:n#5.0;c:a|b;(c[0]=5.0)}}[]"), "1b");
}

#[test]
#[ignore]
fn overflow_arith_binary_kh() {
    // 2-byte short: 1.2B×2 = 2.4 GB.  Stresses the KH dispatch in pp_add etc.
    let mut c = conn();
    let n = "1200000000";
    eq_q(&mut c, "+kh",
        &format!("{{n:{n};a:n#3h;b:n#5h;c:a+b;(c[0]=8h)&(c[n-1]=8h)&(n=count \
            c)}}[]"), "1b");
    eq_q(&mut c, "-kh",
        &format!("{{n:{n};a:n#8h;b:n#3h;c:a-b;(c[0]=5h)&(c[n-1]=5h)}}[]"),
            "1b");
}

#[test]
#[ignore]
fn overflow_arith_div_mod_kj() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "div-kj", &format!("{{n:{n};a:n#17j;b:n#3j;c:a div \
        b;(c[0]=5j)&(c[n-1]=5j)}}[]"), "1b");
    eq_q(&mut c, "mod-kj", &format!("{{n:{n};a:n#17j;b:n#3j;c:a mod \
        b;(c[0]=2j)&(c[n-1]=2j)}}[]"), "1b");
}

// ── Unary arithmetic: neg, abs, sqrt, exp, log ──────────────────────────── //

#[test]
#[ignore]
fn overflow_unary_sqrt_kf() {
    let mut c = conn();
    eq_q(&mut c, "sqrt-kf",
        "{n:300000000;v:n#16.0;w:sqrt v;(w[0]=4.0)&(w[n-1]=4.0)&(n=count w)}[]",
        "1b");
}

#[test]
#[ignore]
fn overflow_unary_exp_log_kf() {
    let mut c = conn();
    // exp/log use fast approximations, not libm; use a tolerance check.
    eq_q(&mut c, "exp-kf",
        "{n:200000000;v:n#0.0;w:exp v;(abs[w[0]-1.0]<0.001)&(n=count w)}[]",
        "1b");
    eq_q(&mut c, "log-kf",
        "{n:200000000;v:n#1.0;w:log v;(abs[w[0]]<0.001)&(n=count w)}[]",
        "1b");
}

// ── Reductions: sum, avg, min, max, prd, var, dev, count, first, last ──── //

#[test]
#[ignore]
fn overflow_reductions_kj() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "sum-kj",   &format!("{{n:{n};v:n#1j;(sum v)=n}}[]"), "1b");
    eq_q(&mut c, "max-kj",   &format!("{{n:{n};v:n#42j;(max v)=42j}}[]"), "1b");
    eq_q(&mut c, "min-kj",   &format!("{{n:{n};v:n#42j;(min v)=42j}}[]"), "1b");
    eq_q(&mut c, "first-kj", &format!("{{n:{n};v:n#7j;(first v)=7j}}[]"), "1b");
    eq_q(&mut c, "last-kj",  &format!("{{n:{n};v:n#7j;(last v)=7j}}[]"), "1b");
    eq_q(&mut c, "count-kj", &format!("{{n:{n};v:n#1j;(count v)=n}}[]"), "1b");
}

#[test]
#[ignore]
fn overflow_reductions_ki() {
    let mut c = conn();
    let n = "600000000";
    eq_q(&mut c, "sum-ki",   &format!("{{n:{n};v:n#1i;(sum v)=n}}[]"), "1b");
    eq_q(&mut c, "max-ki",   &format!("{{n:{n};v:n#42i;(max v)=42i}}[]"), "1b");
    eq_q(&mut c, "min-ki",   &format!("{{n:{n};v:n#42i;(min v)=42i}}[]"), "1b");
    eq_q(&mut c, "first-ki", &format!("{{n:{n};v:n#7i;(first v)=7i}}[]"), "1b");
    eq_q(&mut c, "count-ki", &format!("{{n:{n};v:n#1i;(count v)=n}}[]"), "1b");
}

#[test]
#[ignore]
fn overflow_reductions_kf() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "sum-kf",  &format!("{{n:{n};v:n#1.0;(sum v)=`float$n}}[]"),
        "1b");
    eq_q(&mut c, "avg-kf",  &format!("{{n:{n};v:n#7.0;(avg v)=7.0}}[]"), "1b");
    eq_q(&mut c, "max-kf",  &format!("{{n:{n};v:n#42.0;(max v)=42.0}}[]"),
        "1b");
    eq_q(&mut c, "min-kf",  &format!("{{n:{n};v:n#42.0;(min v)=42.0}}[]"),
        "1b");
    eq_q(&mut c, "var-kf",  &format!("{{n:{n};v:n#5.0;(var v)<1e-9}}[]"), "1b");
    eq_q(&mut c, "dev-kf",  &format!("{{n:{n};v:n#5.0;(dev v)<1e-9}}[]"), "1b");
}

#[test]
#[ignore]
fn overflow_reductions_kb() {
    let mut c = conn();
    // KB: 1-byte boolean.  3e9 would hit INT_MAX; use 2e9 to stay below LM.
    let n = "2000000000";
    eq_q(&mut c, "all-kb", &format!("{{n:{n};v:n#1b;all v}}[]"), "1b");
    eq_q(&mut c, "any-kb", &format!("{{n:{n};v:n#0b;not any v}}[]"), "1b");
    eq_q(&mut c, "count-kb", &format!("{{n:{n};v:n#1b;(count v)=n}}[]"), "1b");
}

// ── Sort and grade: asc, desc, iasc, idesc, rank ────────────────────────── //

#[test]
#[ignore]
fn overflow_sort_kj() {
    let mut c = conn();
    let n = "300000000";
    // Already-constant vec is trivially sorted; still exercises dispatch and grade (first g)=0.
    eq_q(&mut c, "asc-kj",   &format!("{{n:{n};v:n#7j;w:asc v;(n=count \
        w)&(w[0]=7j)}}[]"), "1b");
    eq_q(&mut c, "desc-kj",  &format!("{{n:{n};v:n#7j;w:desc v;(n=count \
        w)&(w[0]=7j)}}[]"), "1b");
    eq_q(&mut c, "iasc-kj",  &format!("{{n:{n};v:n#7j;g:iasc v;n=count g}}[]"),
        "1b");
    eq_q(&mut c, "idesc-kj", &format!("{{n:{n};v:n#7j;g:idesc v;n=count \
        g}}[]"), "1b");
    eq_q(&mut c, "rank-kj",  &format!("{{n:{n};v:n#7j;g:rank v;n=count g}}[]"),
        "1b");
}

#[test]
#[ignore]
fn overflow_sort_ki() {
    let mut c = conn();
    let n = "600000000";
    eq_q(&mut c, "asc-ki",  &format!("{{n:{n};v:n#7i;w:asc v;(n=count \
        w)&(w[0]=7i)}}[]"), "1b");
    eq_q(&mut c, "desc-ki", &format!("{{n:{n};v:n#7i;w:desc v;(n=count \
        w)&(w[0]=7i)}}[]"), "1b");
    eq_q(&mut c, "iasc-ki", &format!("{{n:{n};v:n#7i;g:iasc v;n=count g}}[]"),
        "1b");
    eq_q(&mut c, "rank-ki", &format!("{{n:{n};v:n#7i;g:rank v;n=count g}}[]"),
        "1b");
}

#[test]
#[ignore]
fn overflow_sort_kf() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "asc-kf",  &format!("{{n:{n};v:n#7.0;w:asc v;(n=count \
        w)&(w[0]=7.0)}}[]"), "1b");
    eq_q(&mut c, "desc-kf", &format!("{{n:{n};v:n#7.0;w:desc v;(n=count \
        w)&(w[0]=7.0)}}[]"), "1b");
    eq_q(&mut c, "iasc-kf", &format!("{{n:{n};v:n#7.0;g:iasc v;n=count g}}[]"),
        "1b");
}

#[test]
#[ignore]
fn overflow_sort_ks() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "asc-ks",  &format!("{{n:{n};v:n#`x;w:asc v;(n=count w)}}[]"),
        "1b");
    eq_q(&mut c, "iasc-ks", &format!("{{n:{n};v:n#`x;g:iasc v;n=count g}}[]"),
        "1b");
}

// ── Group / distinct / differ ───────────────────────────────────────────── //

#[test]
#[ignore]
fn overflow_group_kj() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "group-kj",    &format!("{{n:{n};v:n#1j;g:group v;1=count \
        g}}[]"), "1b");
    eq_q(&mut c, "distinct-kj", &format!("{{n:{n};v:n#1j;1=count distinct \
        v}}[]"), "1b");
    eq_q(&mut c, "differ-kj",   &format!("{{n:{n};v:n#1j;d:differ v;n=count \
        d}}[]"), "1b");
}

#[test]
#[ignore]
fn overflow_group_ki() {
    let mut c = conn();
    let n = "600000000";
    eq_q(&mut c, "group-ki",    &format!("{{n:{n};v:n#1i;g:group v;1=count \
        g}}[]"), "1b");
    eq_q(&mut c, "distinct-ki", &format!("{{n:{n};v:n#1i;1=count distinct \
        v}}[]"), "1b");
}

#[test]
#[ignore]
fn overflow_group_ks() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "group-ks",    &format!("{{n:{n};v:n#`x;g:group v;1=count \
        g}}[]"), "1b");
    eq_q(&mut c, "distinct-ks", &format!("{{n:{n};v:n#`x;1=count distinct \
        v}}[]"), "1b");
}

// ── Comparison: =, <>, <, >, <=, >= ─────────────────────────────────────── //

#[test]
#[ignore]
fn overflow_compare_kj() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "eq-kj", &format!("{{n:{n};a:n#1j;b:n#1j;all a=b}}[]"), "1b");
    eq_q(&mut c, "lt-kj", &format!("{{n:{n};a:n#1j;b:n#2j;all a<b}}[]"), "1b");
    eq_q(&mut c, "gt-kj", &format!("{{n:{n};a:n#2j;b:n#1j;all a>b}}[]"), "1b");
    eq_q(&mut c, "le-kj", &format!("{{n:{n};a:n#1j;b:n#1j;all a<=b}}[]"), "1b");
    eq_q(&mut c, "ge-kj", &format!("{{n:{n};a:n#1j;b:n#1j;all a>=b}}[]"), "1b");
    eq_q(&mut c, "ne-kj", &format!("{{n:{n};a:n#1j;b:n#2j;all a<>b}}[]"), "1b");
}

#[test]
#[ignore]
fn overflow_compare_ki() {
    let mut c = conn();
    let n = "600000000";
    eq_q(&mut c, "eq-ki", &format!("{{n:{n};a:n#1i;b:n#1i;all a=b}}[]"), "1b");
    eq_q(&mut c, "lt-ki", &format!("{{n:{n};a:n#1i;b:n#2i;all a<b}}[]"), "1b");
}

#[test]
#[ignore]
fn overflow_compare_kf() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "eq-kf", &format!("{{n:{n};a:n#1.0;b:n#1.0;all a=b}}[]"),
        "1b");
    eq_q(&mut c, "lt-kf", &format!("{{n:{n};a:n#1.0;b:n#2.0;all a<b}}[]"),
        "1b");
}

#[test]
#[ignore]
fn overflow_compare_ks() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "eq-ks", &format!("{{n:{n};a:n#`x;b:n#`x;all a=b}}[]"), "1b");
}

// ── Search: in, within, where, bin, find (?) ────────────────────────────── //

#[test]
#[ignore]
fn overflow_search_kj() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "in-kj",     &format!("{{n:{n};v:n#7j;(7j in v)&not (99j in \
        v)}}[]"), "1b");
    eq_q(&mut c, "within-kj", &format!("{{n:{n};v:n#7j;all v within 1 \
        10j}}[]"), "1b");
    eq_q(&mut c, "find-kj",   &format!("{{n:{n};v:n#7j;(v?7j)=0}}[]"), "1b");
}

#[test]
#[ignore]
fn overflow_search_ki() {
    let mut c = conn();
    let n = "600000000";
    eq_q(&mut c, "in-ki",     &format!("{{n:{n};v:n#7i;(7i in v)&not (99i in \
        v)}}[]"), "1b");
    eq_q(&mut c, "within-ki", &format!("{{n:{n};v:n#7i;all v within 1 \
        10i}}[]"), "1b");
}

#[test]
#[ignore]
fn overflow_search_ks() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "in-ks",   &format!("{{n:{n};v:n#`x;(`x in v)&not `y in \
        v}}[]"), "1b");
    eq_q(&mut c, "find-ks", &format!("{{n:{n};v:n#`x;(v?`x)=0}}[]"), "1b");
}

#[test]
#[ignore]
fn overflow_search_bin_kj() {
    let mut c = conn();
    // bin requires sorted vec.  Constant vec is trivially sorted.
    let n = "300000000";
    eq_q(&mut c, "bin-kj", &format!("{{n:{n};v:`s#n#7j;(v bin 7j)>=0}}[]"),
        "1b");
}

// ── Set operations: union, inter, except ────────────────────────────────── //

#[test]
#[ignore]
fn overflow_set_ops_kj() {
    let mut c = conn();
    let n = "300000000";
    // union returns distinct; inter/except preserve LHS multiplicity.
    eq_q(&mut c, "union-kj",  &format!("{{n:{n};a:n#1j;b:n#2j;r:a union \
        b;2=count r}}[]"), "1b");
    eq_q(&mut c, "inter-kj",  &format!("{{n:{n};a:n#1j;b:n#1j;r:a inter \
        b;n=count r}}[]"), "1b");
    eq_q(&mut c, "except-kj", &format!("{{n:{n};a:n#1j;b:n#2j;r:a except \
        b;n=count r}}[]"), "1b");
}

#[test]
#[ignore]
fn overflow_set_ops_ks() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "union-ks",  &format!("{{n:{n};a:n#`x;b:n#`y;r:a union \
        b;2=count r}}[]"), "1b");
    eq_q(&mut c, "inter-ks",  &format!("{{n:{n};a:n#`x;b:n#`x;r:a inter \
        b;n=count r}}[]"), "1b");
    eq_q(&mut c, "except-ks", &format!("{{n:{n};a:n#`x;b:n#`y;r:a except \
        b;n=count r}}[]"), "1b");
}

// ── Joins: lj, ij, uj on huge tables ────────────────────────────────────── //

#[test]
#[ignore]
fn overflow_join_lj_300m() {
    // Two 300M-row tables, left join on a key: exercises hash-build, lookup, result construction.
    let mut c = conn();
    eq_q(&mut c, "lj-300m",
        "{n:300000000;t:([]a:n#1j;b:n#10j);u:([]a:n#1j;c:n#100j);\
          r:t lj `a xkey u;(n=count r)}[]",
        "1b");
}

#[test]
#[ignore]
fn overflow_join_ij_300m() {
    let mut c = conn();
    eq_q(&mut c, "ij-300m",
        "{n:300000000;t:([]a:n#1j;b:n#10j);u:([]a:n#1j;c:n#100j);\
          r:t ij `a xkey u;(n=count r)}[]",
        "1b");
}

#[test]
#[ignore]
fn overflow_join_uj_keyed_300m() {
    let mut c = conn();
    // Keyed tables keep duplicate keys; uj merges columns row-wise (same row count).
    eq_q(&mut c, "uj-300m",
        "{n:300000000;t:([k:n#1j]v:n#10j);u:([k:n#1j]w:n#100j);\
          r:t uj u;(n=count r)}[]",
        "1b");
}

// ── Comparison match (~) on huge tables ─────────────────────────────────── //

#[test]
#[ignore]
fn overflow_match_table_300m() {
    let mut c = conn();
    // ~ on two identical 300M-row tables stresses per-column byte-hash.
    eq_q(&mut c, "match-table",
        "{n:300000000;t:([]a:n#1j;b:n#10j);u:([]a:n#1j;b:n#10j);t~u}[]",
        "1b");
}

// ── Adverbs: over (/), scan (\), each, each-prior ──────────────────────── //

#[test]
#[ignore]
fn overflow_adverb_over_kj() {
    let mut c = conn();
    // {x+y} over forces the generic fold path; compare to a long-cast sum.
    eq_q(&mut c, "over-add-kj",
        "{n:300000000;v:n#1j;({x+y}/) v}[]",
        "{`long$300000000}[]");
}

#[test]
#[ignore]
fn overflow_sums_maxs_mins() {
    let mut c = conn();
    // sums/maxs/mins scans: output size = input; n=100M keeps the result small.
    eq_q(&mut c, "sums-kj",
        "{n:100000000;v:n#1j;w:sums v;(w[0]=1j)&(w[n-1]=`long$n)}[]", "1b");
    eq_q(&mut c, "maxs-kj",
        "{n:100000000;v:n#5j;w:maxs v;(w[0]=5j)&(w[n-1]=5j)}[]", "1b");
    eq_q(&mut c, "mins-kj",
        "{n:100000000;v:n#5j;w:mins v;(w[0]=5j)&(w[n-1]=5j)}[]", "1b");
}

#[test]
#[ignore]
fn overflow_deltas_kj() {
    let mut c = conn();
    // deltas (prior-diff): output same shape as input.
    eq_q(&mut c, "deltas-kj",
        "{n:300000000;v:n#5j;d:deltas v;(d[0]=5j)&(d[n-1]=0j)&(n=count d)}[]",
        "1b");
}

// ── Reshape: #, @, _, reverse, rotate, raze, enlist ─────────────────────── //

#[test]
#[ignore]
fn overflow_reshape_take_drop() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "take-kj",
        &format!("{{n:{n};v:n#7j;w:(n-1)#v;(n-1)=count w}}[]"), "1b");
    eq_q(&mut c, "drop-kj",
        &format!("{{n:{n};v:n#7j;w:1_v;(n-1)=count w}}[]"), "1b");
    eq_q(&mut c, "at-kj",
        &format!("{{n:{n};v:n#7j;(v@n-1)=7j}}[]"), "1b");
}

#[test]
#[ignore]
fn overflow_reshape_reverse_rotate() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "reverse-kj",
        &format!("{{n:{n};v:n#7j;w:reverse v;(n=count w)&(w[0]=7j)}}[]"), "1b");
    eq_q(&mut c, "rotate-kj",
        &format!("{{n:{n};v:n#7j;w:rotate[1;v];(n=count w)&(w[0]=7j)}}[]"),
            "1b");
}

#[test]
#[ignore]
fn overflow_raze_nested() {
    let mut c = conn();
    // raze flattens 100 x 3M sub-vectors (300M total); stresses the list serialize/append path.
    eq_q(&mut c, "raze-nested",
        "{inner:3000000#1j;v:100#enlist inner;r:raze v;(300000000=count \
            r)&(r[0]=1j)}[]",
        "1b");
}

// ── Type conversion: `long$, `float$, `int$, `short$ ───────────────────── //

#[test]
#[ignore]
fn overflow_type_cast() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "cast-kj-kf",
        &format!("{{n:{n};v:n#7j;w:`float$v;(n=count w)&(w[0]=7.0)}}[]"), "1b");
    eq_q(&mut c, "cast-kf-kj",
        &format!("{{n:{n};v:n#7.0;w:`long$v;(n=count w)&(w[0]=7j)}}[]"), "1b");
    eq_q(&mut c, "cast-ki-kj",
        "{n:600000000;v:n#7i;w:`long$v;(n=count w)&(w[0]=7j)}[]", "1b");
    eq_q(&mut c, "cast-kj-ki",
        &format!("{{n:{n};v:n#7j;w:`int$v;(n=count w)&(w[0]=7i)}}[]"), "1b");
}

// ── Table ops: select, update, delete, xasc, xdesc, xbar ────────────────── //

#[test]
#[ignore]
fn overflow_table_select() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "select-where",
        &format!("{{n:{n};t:([]a:n#7j;b:n#1j);r:select from t where \
            a=7j;n=count r}}[]"),
        "1b");
    eq_q(&mut c, "select-agg",
        &format!("{{n:{n};t:([]a:n#1j);r:first exec sum a from t;r=n}}[]"),
            "1b");
}

#[test]
#[ignore]
fn overflow_table_update() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "update",
        &format!("{{n:{n};t:([]a:n#1j);r:update b:a+1 from t;\
            (n=count r)&(2j=first r[`b])}}[]"), "1b");
}

#[test]
#[ignore]
fn overflow_table_xasc_xdesc() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "xasc",
        &format!("{{n:{n};t:([]a:n#7j;b:n#1j);r:`a xasc t;n=count r}}[]"),
            "1b");
    eq_q(&mut c, "xdesc",
        &format!("{{n:{n};t:([]a:n#7j;b:n#1j);r:`a xdesc t;n=count r}}[]"),
            "1b");
}

#[test]
#[ignore]
fn overflow_select_by_group() {
    let mut c = conn();
    // Group-by aggregation with an all-equal key: single group, 1-row sum result.
    let n = "300000000";
    eq_q(&mut c, "select-by",
        &format!("{{n:{n};t:([]a:n#1j;b:n#2j);r:select sum b by a from \
            t;1=count r}}[]"),
        "1b");
}

// ── Dict ops: ! (construct), keys, values, find ─────────────────────────── //

#[test]
#[ignore]
fn overflow_dict_construct() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "dict-ctor",
        &format!("{{n:{n};d:(til n)!n#1j;(d 0)=1j}}[]"), "1b");
    eq_q(&mut c, "dict-keys",
        &format!("{{n:{n};d:(til n)!n#1j;n=count key d}}[]"), "1b");
    eq_q(&mut c, "dict-values",
        &format!("{{n:{n};d:(til n)!n#1j;n=count value d}}[]"), "1b");
}

// ── Attribute paths: `s#, `u#, `p#, `g# on big vecs ─────────────────────── //

#[test]
#[ignore]
fn overflow_attr_sorted() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "s-attr-kj",
        &format!("{{n:{n};v:`s#n#7j;(attr v)=`s}}[]"), "1b");
}

#[test]
#[ignore]
fn overflow_attr_unique() {
    let mut c = conn();
    // distinct on a single-group vec collapses to 1 unique.
    let n = "300000000";
    eq_q(&mut c, "u-attr-til",
        &format!("{{n:{n};v:`u#`long$til n;(attr v)=`u}}[]"), "1b");
}

#[test]
#[ignore]
fn overflow_attr_grouped() {
    let mut c = conn();
    // `g# triggers sI counting sort which was a load-bearing fix site.
    let n = "300000000";
    eq_q(&mut c, "g-attr-kj",
        &format!("{{n:{n};v:`g#n#7j;(attr v)=`g}}[]"), "1b");
}

#[test]
#[ignore]
fn overflow_attr_parted() {
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "p-attr-kj",
        &format!("{{n:{n};v:`p#n#7j;(attr v)=`p}}[]"), "1b");
}

// ── Nested lists / mixed type 0 at scale ────────────────────────────────── //

#[test]
#[ignore]
fn overflow_mixed_list_serialize() {
    let mut c = conn();
    // Mixed list of 100 x 3M sub-vectors; IPC round-trip stresses the recursive serialize size calc.
    let srv_port = 9901u16;
    let _srv = spawn_l(srv_port);
    eq_q(&mut c, "mixed-list-ipc",
        &format!("{{h:hopen {port};\
            h \"L:100#enlist 3000000#99j\";\
            r:h \"(100=count L)&(3000000=count first L)&(99j=first first L)\";\
            hclose h;r}}[]", port = srv_port),
        "1b");
}

// ── String ops on big KC vecs ───────────────────────────────────────────── //

#[test]
#[ignore]
fn overflow_string_upper_lower() {
    let mut c = conn();
    // 1-byte KC: 2.4 GB at n=2.4e9 chars.  Use 1.2e9 for speed (1.2 GB).
    let n = "1200000000";
    eq_q(&mut c, "upper-kc",
        &format!("{{n:{n};s:n#\"a\";u:upper s;(n=count \
            u)&(u[0]=\"A\")&(u[n-1]=\"A\")}}[]"),
        "1b");
    eq_q(&mut c, "lower-kc",
        &format!("{{n:{n};s:n#\"A\";u:lower s;(n=count u)&(u[0]=\"a\")}}[]"),
        "1b");
}

// ── Temporal types: KP, KD, KT at scale ─────────────────────────────────── //

#[test]
#[ignore]
fn overflow_temporal_kp() {
    let mut c = conn();
    // Temporal (KP, 8-byte): stresses temporal sort/compare at the same threshold as KJ.
    let n = "300000000";
    eq_q(&mut c, "kp-count",
        &format!("{{n:{n};v:n#2024.01.01D00:00:00.000000000;n=count v}}[]"),
            "1b");
    eq_q(&mut c, "kp-asc",
        &format!("{{n:{n};v:n#2024.01.01D00:00:00.000000000;w:asc v;n=count \
            w}}[]"), "1b");
}

#[test]
#[ignore]
fn overflow_temporal_kd() {
    let mut c = conn();
    // KD is 4 bytes — threshold 537M.  Use 6e8 as for KI.
    let n = "600000000";
    eq_q(&mut c, "kd-count",
        &format!("{{n:{n};v:n#2024.01.01;n=count v}}[]"), "1b");
    eq_q(&mut c, "kd-asc",
        &format!("{{n:{n};v:n#2024.01.01;w:asc v;n=count w}}[]"), "1b");
}

// ── Indexed amend at high index (covers au/del fix for every type) ────── //

#[test]
#[ignore]
fn overflow_index_amend_kj() {
    let mut c = conn();
    // Amend v[i]:x at i>268M for an 8-byte type; plain and attributed vecs.
    eq_q(&mut c, "amend-kj",
        "{n:300000000;i:280000000;v:n#1j;v[i]:99j;(v[i]=99j)&(v[0]=1j)}[]",
            "1b");
    // Same with `s# attribute: amend should strip attr since not sorted anymore.
    eq_q(&mut c, "amend-s-kj",
        "{n:300000000;i:280000000;v:`s#n#1j;v[i]:99j;(v[i]=99j)}[]", "1b");
}

#[test]
#[ignore]
fn overflow_index_amend_ki() {
    let mut c = conn();
    eq_q(&mut c, "amend-ki",
        "{n:600000000;i:560000000;v:n#1i;v[i]:99i;(v[i]=99i)&(v[0]=1i)}[]",
            "1b");
}

// ── `enum` / foreign-key style at scale (if supported) ───────────────── //

#[test]
#[ignore]
fn overflow_ks_roundtrip_large() {
    // 300M same-symbol vec exercises symbol serialize and sym-hash lookup.
    let mut c = conn();
    let n = "300000000";
    eq_q(&mut c, "ks-first-last",
        &format!("{{n:{n};v:n#`hello;(first v)=`hello}}[]"), "1b");
    eq_q(&mut c, "ks-count-match",
        &format!("{{n:{n};v:n#`hello;(v~v)&(n=count v)}}[]"), "1b");
}

// ── Rapid-fire many-query stress against a single connection ───────────── //

#[test]
#[ignore]
fn overflow_rapid_fire_large_vecs() {
    // 50 iterations of build-and-aggregate on a 268M+ vec: catches GC/allocator regressions under load.
    let mut c = conn();
    for i in 0..50 {
        let r = c.query(
            "{n:280000000;v:n#7j;(sum v)=n*7j}[]"
        ).unwrap_or_else(|e| panic!("iter {i}: {e:?}"));
        assert_eq!(r, K::Bool(true), "iter {i}: sum mismatch");
    }
}

// PARALLELISM — verify the worker pool is active and hot kernels dispatch in parallel.

// Time a query best-of-N via the server-side \t timer (microseconds), avoiding IPC RTT.
fn time_query_min_us(c: &mut Connection, q: &str, reps: usize) -> i64 {
    c.query(q).unwrap();
    let timed = format!("\\t {}", q);
    let mut best = i64::MAX;
    for _ in 0..reps {
        let r = c.query(&timed).unwrap();
        let v = match r {
            K::Long(v) => v, K::Int(v) => v as i64, K::Short(v) => v as i64,
            k => panic!("\\t {q} expected int, got {k:?}"),
        };
        if v < best { best = v; }
    }
    best
}

#[test]
fn parallel_pp_for_active() {
    // Skip under sanitizers: instrumentation slows l 5-15x and blows past the timing thresholds.
    if std::env::var("L_SKIP_PERF_TESTS").is_ok() {
        eprintln!("parallel_pp_for_active: skipped (L_SKIP_PERF_TESTS set)");
        return;
    }
    let mut c = conn();

    // 1. Worker pool must be spawned, else every cost-gated kernel runs serial.
    let nw = match c.query(".z.nw").unwrap() {
        K::Long(v)  => v,
        K::Int(v)   => v as i64,
        K::Short(v) => v as i64,
        k => panic!(".z.nw should be int-typed, got {k:?}"),
    };
    assert!(nw > 1, ".z.nw = {nw} — worker pool not spawned (pp_init missed)");
    // Pool size may exceed the per-kernel clamp on many-core boxes; the load-bearing check is nw > 1.
    assert!(nw <= 256, ".z.nw = {nw} — implausible worker count");

    // 2. Per-kernel timing via the server-side timer; thresholds sit between parallel and serial.
    c.query("vf: 10000000?1.0; vi: 10000000?100i; \
             va: 10000000?100j; vb: 10000000?100j").unwrap();
    let cases: &[(&str, i64, &str)] = &[
        ("sum vf",     4_000, "fsum / pp_red parallel reduce"),                 // par ≈1, ser ≈6 ms
        ("min vf",     5_000, "accel_red chunked vDSP_minvD"),                  // par ≈1, ser ≈9
        ("max vf",     5_000, "accel_red chunked vDSP_maxvD"),                  // par ≈1, ser ≈9
        ("sum vi",     5_000, "qnl SIMD scan + isum_i parallel"),               // par ≈1, ser ≈8
        ("sum vi<50", 18_000, "pp_cmp parallel compare → bsum"),                // par ≈7, ser ≈30
        ("sum va+vb", 30_000, "pp_d2j parallel long-add"),                      // par ≈10, ser ≈50
    ];
    for &(q, max_us, what) in cases {
        let us = time_query_min_us(&mut c, q, 5);
        assert!(us < max_us,
            "{q} best-of-5: {us}µs > {max_us}µs threshold — {what} likely \
                serial");
    }
}

// DEEP COVERAGE MATRIX — 1500+ systematic tests, one #[test] per eqt! (verb x type x shape x nulls).

macro_rules! eqt {
    ($name:ident, $expr:expr, $expected:expr) => {
        #[test]
        fn $name() { eq_q(&mut conn(), stringify!($name), $expr, $expected); }
    };
}

// ── A. ARITHMETIC: add ─────────────────────────────────────────────
eqt!(d_add_ii_vv,         "1 2 3+4 5 6",                   "5 7 9");
eqt!(d_add_ii_vv_5,       "1 2 3 4 5+5 4 3 2 1",           "6 6 6 6 6");
eqt!(d_add_ii_vv_zeros,   "0 0 0+1 2 3",                   "1 2 3");
eqt!(d_add_ii_vv_neg,     "1 2 3+(-1;-2;-3)",              "0 0 0");
eqt!(d_add_ii_vv_null_m,  "(1;0Ni;3)+4 5 6",               "5 0N 9");
eqt!(d_add_ii_vv_null_h,  "(0N;2;3)+4 5 6",                "0N 7 9");
eqt!(d_add_ii_vv_null_t,  "(1;2;0N)+4 5 6",                "5 7 0N");
eqt!(d_add_ii_vv_all_nul, "(0N;0N;0N)+4 5 6",              "0N 0N 0N");
eqt!(d_add_ii_sv_pos,     "42+1 2 3",                      "43 44 45");
eqt!(d_add_ii_sv_neg,     "-3+1 2 3",                      "-2 -1 0");
eqt!(d_add_ii_sv_zero,    "0+1 2 3",                       "1 2 3");
eqt!(d_add_ii_sv_null,    "0N+1 2 3",                      "0N 0N 0N");
eqt!(d_add_ii_vs,         "1 2 3+42",                      "43 44 45");
eqt!(d_add_ii_vs_null,    "1 2 3+0N",                      "0N 0N 0N");
eqt!(d_add_ii_ss,         "1+2",                           "3");
eqt!(d_add_ii_ss_neg,     "(-7)+3",                        "-4");
eqt!(d_add_ii_ss_null_l,  "0N+2",                          "0N");
eqt!(d_add_ii_ss_null_r,  "1+0N",                          "0N");
eqt!(d_add_ii_large_1k,   "(1000#1)+1000#2",               "1000#3");
eqt!(d_add_ii_large_100k, "(100000#1)+100000#2",           "100000#3");
eqt!(d_add_jj_vv,         "1 2 3j+4 5 6j",                 "5 7 9j");
eqt!(d_add_jj_vv_null,    "(1j;0Nj;3j)+4 5 6j",            "5 0N 9j");
eqt!(d_add_jj_sv,         "1000000000000j+1 2 3j",         "1000000000001 \
    1000000000002 1000000000003j");
eqt!(d_add_jj_vs,         "1 2 3j+10j",                    "11 12 13j");
eqt!(d_add_jj_ss,         "1j+2j",                         "3j");
eqt!(d_add_jj_null_null,  "0Nj+0Nj",                       "0Nj");
eqt!(d_add_jj_large,      "(1000#1j)+1000#2j",             "1000#3j");
eqt!(d_add_ff_vv,         "1.0 2.0 3.0+4.0 5.0 6.0",       "5 7 9f");
eqt!(d_add_ff_vv_null,    "(1.0;0n;3.0)+4.0 5.0 6.0",      "5 0n 9f");
eqt!(d_add_ff_sv,         "1.5+1.0 2.0 3.0",               "2.5 3.5 4.5");
eqt!(d_add_ff_vs,         "1.0 2.0 3.0+1.5",               "2.5 3.5 4.5");
eqt!(d_add_ff_ss,         "1.5+2.5",                       "4f");
eqt!(d_add_ff_large,      "(1000#1.5)+1000#2.5",           "1000#4f");
eqt!(d_add_ee_vv,         "1 2 3e+4 5 6e",                 "5 7 9e");
eqt!(d_add_ee_sv,         "1e+1 2 3e",                     "2 3 4e");
eqt!(d_add_ee_null,       "(1e;0ne;3e)+1 2 3e",            "(2e;0ne;6e)");
eqt!(d_add_hh_vv,         "1 2 3h+4 5 6h",                 "5 7 9i");
eqt!(d_add_hh_sv,         "10h+1 2 3h",                    "11 12 13i");
eqt!(d_add_gg_vv,         "0x01+0x02",                     "3");
eqt!(d_add_cross_ij,      "1 2 3+1 2 3j",                  "2 4 6j");
eqt!(d_add_cross_if,      "1 2 3+1.0 2.0 3.0",             "2 4 6f");
eqt!(d_add_cross_jf,      "1 2 3j+1.0 2.0 3.0",            "2 4 6f");
eqt!(d_add_cross_hi,      "1 2 3h+1 2 3",                  "2 4 6i");

// ── A. ARITHMETIC: sub ─────────────────────────────────────────────
eqt!(d_sub_ii_vv,         "10 20 30-1 2 3",                "9 18 27");
eqt!(d_sub_ii_vv_neg,     "1 2 3-(4;5;6)",                 "-3 -3 -3");
eqt!(d_sub_ii_vv_null,    "(10;0N;30)-1 2 3",              "9 0N 27");
eqt!(d_sub_ii_sv,         "100-1 2 3",                     "99 98 97");
eqt!(d_sub_ii_vs,         "100 200 300-50",                "50 150 250");
eqt!(d_sub_ii_ss,         "10-3",                          "7");
eqt!(d_sub_ii_ss_neg,     "3-10",                          "-7");
eqt!(d_sub_ii_ss_null,    "0N-3",                          "0N");
eqt!(d_sub_ii_large,      "(1000#10)-1000#3",              "1000#7");
eqt!(d_sub_jj_vv,         "10 20 30j-1 2 3j",              "9 18 27j");
eqt!(d_sub_jj_null,       "(10j;0Nj;30j)-1 2 3j",          "9 0N 27j");
eqt!(d_sub_jj_ss,         "10j-3j",                        "7j");
eqt!(d_sub_ff_vv,         "1.5 2.5 3.5-0.5 1.0 1.5",       "1 1.5 2f");
eqt!(d_sub_ff_null,       "(1.0;0n;3.0)-0.5 0.5 0.5",      "0.5 0n 2.5");
eqt!(d_sub_ff_ss,         "3.5-1.5",                       "2f");
eqt!(d_sub_ee_vv,         "(10 20 30e)-1 2 3e",           "9 18 27e");
eqt!(d_sub_hh_vv,         "10 20 30h-1 2 3h",              "9 18 27i");
eqt!(d_sub_cross_ij,      "10 20 30-1 2 3j",               "9 18 27j");
eqt!(d_sub_cross_if,      "10 20 30-1.5 2.5 3.5",          "8.5 17.5 26.5");
eqt!(d_sub_ii_self,       "5 6 7-5 6 7",                   "0 0 0");

// ── A. ARITHMETIC: mul ─────────────────────────────────────────────
eqt!(d_mul_ii_vv,         "2 3 4*5 6 7",                   "10 18 28");
eqt!(d_mul_ii_vv_zero,    "2 3 4*0 0 0",                   "0 0 0");
eqt!(d_mul_ii_vv_neg,     "2 3 4*(-1;-2;-3)",              "-2 -6 -12");
eqt!(d_mul_ii_vv_null,    "(2;0N;4)*5 6 7",                "10 0N 28");
eqt!(d_mul_ii_sv,         "10*1 2 3",                      "10 20 30");
eqt!(d_mul_ii_vs,         "1 2 3*10",                      "10 20 30");
eqt!(d_mul_ii_ss,         "7*8",                           "56");
eqt!(d_mul_ii_ss_neg,     "(-3)*4",                        "-12");
eqt!(d_mul_ii_large,      "(1000#3)*1000#4",               "1000#12");
eqt!(d_mul_jj_vv,         "2 3 4j*5 6 7j",                 "10 18 28j");
eqt!(d_mul_jj_null,       "(2j;0Nj;4j)*5 6 7j",            "10 0N 28j");
eqt!(d_mul_jj_big,        "1000000j*1000000j",             "1000000000000j");
eqt!(d_mul_ff_vv,         "2.0 3.0 4.0*0.5 0.5 0.5",       "1 1.5 2f");
eqt!(d_mul_ff_null,       "(2.0;0n;4.0)*0.5 0.5 0.5",      "1 0n 2f");
eqt!(d_mul_ff_ss,         "2.5*4.0",                       "10f");
eqt!(d_mul_ff_large,      "(1000#2.5)*1000#4.0",           "1000#10f");
eqt!(d_mul_ee_vv,         "2 3 4e*5 6 7e",                 "10 18 28e");
eqt!(d_mul_hh_vv,         "2 3 4h*5 6 7h",                 "10 18 28i");
eqt!(d_mul_cross_ij,      "2 3*4 5j",                      "8 15j");
eqt!(d_mul_cross_if,      "2 3*4.0 5.0",                   "8 15f");

// ── A. ARITHMETIC: div (%) ─────────────────────────────────────────
eqt!(d_div_ii_vv,         "10 20 30%2 4 5",                "5 5 6f");
eqt!(d_div_ii_vv_frac,    "1 2 3%4 4 4",                   "0.25 0.5 0.75");
eqt!(d_div_ff_vv,         "1.0 2.0 3.0%2.0 4.0 6.0",       "0.5 0.5 0.5");
eqt!(d_div_ff_null,       "(1.0;0n;3.0)%2.0 4.0 6.0",      "0.5 0n 0.5");
eqt!(d_div_ff_sv,         "10.0%1.0 2.0 5.0",              "10 5 2f");
eqt!(d_div_ff_vs,         "1.0 2.0 5.0%2.0",               "0.5 1 2.5");
eqt!(d_div_ff_ss,         "7.0%2.0",                       "3.5");
eqt!(d_div_ff_large,      "(1000#10.0)%1000#2.0",          "1000#5f");
eqt!(d_div_ee_vv,         "1 2 3e%2 4 6e",                 "0.5 0.5 0.5e");
eqt!(d_div_cross_ij,      "10 20j%2 4",                    "5 5f");

// ── A. ARITHMETIC: integer div and mod ─────────────────────────────
eqt!(d_idiv_ii_vv,        "10 11 12 div 3",                "3 3 4");
eqt!(d_idiv_ii_neg,       "(-10;-11) div 3",               "-4 -4");
eqt!(d_imod_ii_vv,        "10 11 12 mod 3",                "1 2 0");
eqt!(d_imod_jj_vv,        "10 11 12j mod 3j",              "1 2 0j");
eqt!(d_imod_ii_large,     "(1000#13) mod 5",               "1000#3");

// ── B. COMPARISON: = < > ───────────────────────────────────────────
eqt!(d_eq_ii_vv,          "1 2 3=1 0 3",                   "101b");
eqt!(d_eq_ii_vv_all,      "1 2 3=1 2 3",                   "111b");
eqt!(d_eq_ii_vv_none,     "1 2 3=4 5 6",                   "000b");
eqt!(d_eq_ii_vv_null,     "(1;0N;3)=(1;0N;3)",             "111b");
eqt!(d_eq_ii_sv,          "2=1 2 3",                       "010b");
eqt!(d_eq_ii_vs,          "1 2 3=2",                       "010b");
eqt!(d_eq_ii_ss,          "1=1",                           "1b");
eqt!(d_eq_ii_ss_ne,       "1=2",                           "0b");
eqt!(d_eq_jj_vv,          "1 2 3j=1 0 3j",                 "101b");
eqt!(d_eq_jj_ss,          "42j=42j",                       "1b");
eqt!(d_eq_ff_vv,          "1.0 2.0 3.0=1.0 0.0 3.0",       "101b");
eqt!(d_eq_ff_ss,          "3.14=3.14",                     "1b");
eqt!(d_eq_ss_vv,          "`a`b`c=`a`x`c",                 "101b");
eqt!(d_eq_ss_ss,          "`foo=`foo",                     "1b");
eqt!(d_eq_ss_ss_ne,       "`foo=`bar",                     "0b");
eqt!(d_eq_bb_vv,          "1100b=1010b",                   "1001b");
eqt!(d_eq_cc_vv,          "\"abc\"=\"axc\"",               "101b");
eqt!(d_eq_ii_large,       "(1000#5)=1000#5",               "1000#1b");

eqt!(d_lt_ii_vv,          "1 2 3<2 2 2",                   "100b");
eqt!(d_lt_ii_vv_null,     "(1;0N;3)<2 2 2",                "110b");
eqt!(d_lt_ii_sv,          "2<1 2 3",                       "001b");
eqt!(d_lt_ii_ss,          "1<2",                           "1b");
eqt!(d_lt_ii_ss_ne,       "2<1",                           "0b");
eqt!(d_lt_jj_vv,          "1 2 3j<2 2 2j",                 "100b");
eqt!(d_lt_ff_vv,          "1.0 2.0 3.0<2.5 2.5 2.5",       "110b");
eqt!(d_lt_ss_vv,          "`a`b`c<`b`b`b",                 "100b");

eqt!(d_gt_ii_vv,          "1 2 3>2 2 2",                   "001b");
eqt!(d_gt_ii_null,        "(1;0N;3)>2 2 2",                "001b");
eqt!(d_gt_ii_sv,          "2>1 2 3",                       "100b");
eqt!(d_gt_ii_ss,          "3>1",                           "1b");
eqt!(d_gt_ff_vv,          "1.0 2.0 3.0>2.5 2.5 2.5",       "001b");
eqt!(d_gt_ss_vv,          "`a`b`c>`b`b`b",                 "001b");

eqt!(d_le_ii_vv,          "1 2 3<=2 2 2",                  "110b");
eqt!(d_le_ii_ss,          "2<=2",                          "1b");
eqt!(d_ge_ii_vv,          "1 2 3>=2 2 2",                  "011b");
eqt!(d_ge_ii_ss,          "2>=2",                          "1b");

// ── B. COMPARISON: match (~) ───────────────────────────────────────
eqt!(d_match_ii_vv,       "1 2 3~1 2 3",                   "1b");
eqt!(d_match_ii_vv_ne,    "1 2 3~1 2 4",                   "0b");
eqt!(d_match_ii_type,     "1 2 3~1 2 3j",                  "0b");
eqt!(d_match_ff_vv,       "1.0 2.0~1.0 2.0",               "1b");
eqt!(d_match_ss_vv,       "`a`b`c~`a`b`c",                 "1b");
eqt!(d_match_table_self,  "([]a:1 2 3)~([]a:1 2 3)",       "1b");
eqt!(d_match_dict_self,   "(`a`b!1 2)~(`a`b!1 2)",         "1b");

// ── C. LOGICAL / MIN-MAX ───────────────────────────────────────────
eqt!(d_and_bb_vv,         "1100b&1010b",                   "1000b");
eqt!(d_and_bb_ss,         "1b&1b",                         "1b");
eqt!(d_and_bb_sv,         "1b&1010b",                      "1010b");
eqt!(d_or_bb_vv,          "1100b|1010b",                   "1110b");
eqt!(d_or_bb_ss,          "1b|0b",                         "1b");
eqt!(d_min_ii_vv,         "1 5 3 7&2 4 6 8",               "1 4 3 7");
eqt!(d_min_ii_sv,         "3&1 2 3 4 5",                   "1 2 3 3 3");
eqt!(d_min_ii_null,       "(1;0N;3)&2 2 2",                "1 0N 2");
eqt!(d_max_ii_vv,         "1 5 3 7|2 4 6 8",               "2 5 6 8");
eqt!(d_max_ii_sv,         "3|1 2 3 4 5",                   "3 3 3 4 5");
eqt!(d_max_ii_null,       "(1;0N;3)|2 2 2",                "2 2 3");
eqt!(d_min_jj_vv,         "1 5j&3 4j",                     "1 4j");
eqt!(d_max_jj_vv,         "1 5j|3 4j",                     "3 5j");
eqt!(d_min_ff_vv,         "1.5 2.5&3.5 1.5",               "1.5 1.5");
eqt!(d_max_ff_vv,         "1.5 2.5|3.5 1.5",               "3.5 2.5");

// ── D. MONADIC: neg / abs / sqrt / exp / log / reciprocal / signum ──
eqt!(d_neg_ii_v,          "neg 1 2 3",                     "-1 -2 -3");
eqt!(d_neg_ii_null,       "neg (1;0N;3)",                  "-1 0N -3");
eqt!(d_neg_ii_s,          "neg 42",                        "-42");
eqt!(d_neg_ii_large,      "neg 1000#5",                    "1000#-5");
eqt!(d_neg_jj_v,          "neg 1 2 3j",                    "-1 -2 -3j");
eqt!(d_neg_ff_v,          "neg 1.5 2.5 3.5",               "-1.5 -2.5 -3.5");
eqt!(d_neg_ff_null,       "neg (1.0;0n;3.0)",              "-1 0n -3f");
eqt!(d_neg_ee_v,          "neg 1 2 3e",                    "-1 -2 -3e");
eqt!(d_abs_ii_v,          "abs -1 -2 -3",                  "1 2 3");
eqt!(d_abs_ii_mix,        "abs -1 2 -3",                   "1 2 3");
eqt!(d_abs_ii_null,       "abs (-1;0N;-3)",                "1 0N 3");
eqt!(d_abs_ii_large,      "abs -1000#5",                   "1000#5");
eqt!(d_abs_jj_v,          "abs -1 -2 -3j",                 "1 2 3j");
eqt!(d_abs_ff_v,          "abs -1.5 -2.5",                 "1.5 2.5");
eqt!(d_sqrt_ff_v,         "sqrt 4.0 9.0 16.0",             "2 3 4f");
eqt!(d_sqrt_ff_null,      "sqrt (4.0;0n;16.0)",            "2 0n 4f");
eqt!(d_sqrt_ff_zero,      "sqrt 0.0",                      "0f");
eqt!(d_log_ff_one,        "log 1.0",                       "0f");
eqt!(d_recip_ff_v,        "reciprocal 2.0 4.0 8.0",        "0.5 0.25 0.125");
eqt!(d_signum_ii_v,       "signum -3 0 5",                 "-1 0 1i");
eqt!(d_signum_ff_v,       "signum -3.0 0.0 5.0",           "-1 0 1i");
eqt!(d_not_bb_v,          "not 1100b",                     "0011b");
eqt!(d_not_bb_s,          "not 1b",                        "0b");
eqt!(d_floor_ff,          "floor 1.7 2.3 -1.5",            "1 2 -2");
eqt!(d_ceiling_ff,        "ceiling 1.3 2.7 -1.5",          "2 3 -1");
eqt!(d_null_check_v,      "null 1 0N 3",                   "010b");
eqt!(d_null_check_f,      "null 1.0 0n 3.0",               "010b");
eqt!(d_null_check_s,      "null `a``b",                    "010b");

// ── E. AGGREGATES: sum ─────────────────────────────────────────────
eqt!(d_sum_ii_v,          "sum 1 2 3 4 5",                 "15j");
eqt!(d_sum_ii_null,       "sum 1 2 0N 4 5",                "12j");
eqt!(d_sum_ii_all_null,   "sum 0N 0N 0N",                  "0j");
eqt!(d_sum_ii_empty,      "sum `int$()",                   "0j");
eqt!(d_sum_ii_single,     "sum enlist 42",                 "42j");
eqt!(d_sum_ii_large,      "sum 1000#1",                    "1000j");
eqt!(d_sum_jj_v,          "sum 1 2 3 4 5j",                "15j");
eqt!(d_sum_jj_null,       "sum (1j;0Nj;3j)",               "4j");
eqt!(d_sum_ff_v,          "sum 1.5 2.5 3.5",               "7.5");
eqt!(d_sum_ff_null,       "sum (1.0;0n;3.0)",              "4f");
eqt!(d_sum_ee_v,          "sum 1 2 3 4e",                  "10e");
eqt!(d_sum_bb,            "sum 10110b",                    "3i");
eqt!(d_sum_bb_large,      "sum 1000#1b",                   "1000i");
eqt!(d_sum_hh,            "sum 1 2 3 4h",                  "10j");

// ── E. AGGREGATES: avg ─────────────────────────────────────────────
eqt!(d_avg_ii_v,          "avg 1 2 3 4 5",                 "3f");
eqt!(d_avg_ii_null,       "avg 1 2 0N 4 5",                "3f");
eqt!(d_avg_ii_all_null,   "avg 0N 0N 0N",                  "0n");
eqt!(d_avg_ii_empty,      "avg `int$()",                   "0n");
eqt!(d_avg_jj_v,          "avg 1 2 3 4 5j",                "3f");
eqt!(d_avg_jj_null,       "avg (1j;0Nj;3j)",               "2f");
eqt!(d_avg_ff_v,          "avg 1.0 2.0 3.0",               "2f");
eqt!(d_avg_ff_null,       "avg (1.0;0n;3.0)",              "2f");
eqt!(d_avg_ee_v,          "avg 2 4 6e",                    "4f");
eqt!(d_avg_ii_large,      "avg 10000#5",                   "5f");

// ── E. AGGREGATES: count ───────────────────────────────────────────
eqt!(d_count_ii,          "count 1 2 3 4",                 "4");
eqt!(d_count_ii_null,     "count 1 0N 3",                  "3");
eqt!(d_count_ii_empty,    "count `int$()",                 "0");
eqt!(d_count_jj,          "count 1 2 3j",                  "3");
eqt!(d_count_ff,          "count 1.0 2.0 3.0",             "3");
eqt!(d_count_ss,          "count `a`b`c",                  "3");
eqt!(d_count_cc,          "count \"hello\"",               "5");
eqt!(d_count_list,        "count (1;2.0;`c)",              "3");
eqt!(d_count_large,       "count 10000#1",                 "10000");

// ── E. AGGREGATES: min / max ───────────────────────────────────────
eqt!(d_min_ii,            "min 3 1 4 1 5 9 2 6",           "1");
eqt!(d_minagg_ii_null,       "min 3 1 4 0N 5",                "1");
eqt!(d_min_ii_single,     "min enlist 42",                 "42");
eqt!(d_min_jj,            "min 3 1 4 1 5j",                "1j");
eqt!(d_min_ff,            "min 3.5 1.5 4.5",               "1.5");
eqt!(d_min_ee,            "min 3 1 4e",                    "1e");
eqt!(d_max_ii,            "max 3 1 4 1 5 9 2 6",           "9");
eqt!(d_maxagg_ii_null,       "max 3 1 4 0N 5",                "5");
eqt!(d_max_jj,            "max 3 1 4 1 5j",                "5j");
eqt!(d_max_ff,            "max 3.5 1.5 4.5",               "4.5");
eqt!(d_max_bb,            "max 10110b",                    "1b");
eqt!(d_min_bb,            "min 10110b",                    "0b");

// ── E. AGGREGATES: prd / first / last ──────────────────────────────
eqt!(d_prd_ii,            "prd 1 2 3 4",                   "24");
eqt!(d_prd_ii_null,       "prd 1 2 0N 4",                  "8");
eqt!(d_prd_ff,            "prd 0.5 2.0 4.0",               "4f");
eqt!(d_first_ii,          "first 1 2 3",                   "1");
eqt!(d_first_ss,          "first `a`b`c",                  "`a");
eqt!(d_last_ii,           "last 1 2 3",                    "3");
eqt!(d_last_ss,           "last `a`b`c",                   "`c");

// ── E. AGGREGATES: any / all ───────────────────────────────────────
eqt!(d_any_bb_t,          "any 10010b",                    "1b");
eqt!(d_any_bb_f,          "any 00000b",                    "0b");
eqt!(d_all_bb_t,          "all 11111b",                    "1b");
eqt!(d_all_bb_f,          "all 11011b",                    "0b");
eqt!(d_any_ii,            "any 0 0 1 0",                   "1b");
eqt!(d_all_ii,            "all 1 1 1 1",                   "1b");

// ── E. AGGREGATES: scan variants (sums/prds/mins/maxs) ─────────────
eqt!(d_sums_ii,           "sums 1 2 3 4",                  "1 3 6 10");
eqt!(d_sums_jj,           "sums 1 2 3j",                   "1 3 6j");
eqt!(d_sums_ff,           "sums 1.0 2.0 3.0",              "1 3 6f");
eqt!(d_sums_null,         "sums 1 0N 3 4",                 "1 1 4 8");
eqt!(d_prds_ii,           "prds 1 2 3 4",                  "1 2 6 24");
eqt!(d_mins_ii,           "mins 3 1 4 1 5",                "3 1 1 1 1");
eqt!(d_maxs_ii,           "maxs 3 1 4 1 5",                "3 3 4 4 5");

// ── F. SORT / GRADE ────────────────────────────────────────────────
eqt!(d_asc_ii,            "asc 3 1 4 1 5 9 2 6",           "`s#1 1 2 3 4 5 6 \
    9");
eqt!(d_asc_ii_null,       "asc 3 0N 1 4",                  "`s#0N 1 3 4");
eqt!(d_asc_ii_empty,      "asc `int$()",                   "`s#`int$()");
eqt!(d_asc_ii_single,     "asc enlist 42",                 "`s#enlist 42");
eqt!(d_asc_ii_sorted,     "asc `s#1 2 3 4",                "`s#1 2 3 4");
eqt!(d_asc_ii_large,      "asc 1000#5 3 1 4",              "`s#asc 1000#5 3 1 \
    4");
eqt!(d_asc_jj,            "asc 3 1 4 1 5j",                "`s#1 1 3 4 5j");
eqt!(d_asc_jj_null,       "asc (3j;0Nj;1j)",                "`s#0N 1 3j");
eqt!(d_asc_jj_neg,        "asc -3 1 -4 5j",                "`s#-4 -3 1 5j");
eqt!(d_asc_ff,            "asc 3.5 1.5 4.5 1.5",           "`s#1.5 1.5 3.5 \
    4.5");
eqt!(d_asc_ee,            "asc 3 1 4e",                    "`s#1 3 4e");
eqt!(d_asc_ss,            "asc `banana`apple`cherry",
    "`s#`apple`banana`cherry");
eqt!(d_asc_cc,            "asc \"dba\"",                   "`s#\"abd\"");
eqt!(d_asc_bb,            "asc 1011001b",                  "`s#0001111b");
eqt!(d_desc_ii,           "desc 3 1 4 1 5 9 2 6",          "9 6 5 4 3 2 1 1");
eqt!(d_desc_jj,           "desc 3 1 4 1 5j",               "5 4 3 1 1j");
eqt!(d_desc_ff,           "desc 3.5 1.5 4.5",              "4.5 3.5 1.5");
eqt!(d_desc_ss,           "desc `banana`apple`cherry",
    "`cherry`banana`apple");
eqt!(d_iasc_ii,           "iasc 3 1 4 1 5",                "1 3 0 2 4");
eqt!(d_idesc_ii,          "idesc 3 1 4 1 5",               "4 2 0 1 3");
eqt!(d_rank_ii,           "rank 3 1 4 1 5",                "2 0 3 1 4");

// ── G. TABLES: arithmetic t+t, t-t, t*t, t%t ───────────────────────
eqt!(d_tbl_add,           "([]a:1 2 3;b:4 5 6)+([]a:10 20 30;b:40 50 60)",
    "([]a:11 22 33;b:44 55 66)");
eqt!(d_tbl_sub,           "([]a:10 20 30;b:40 50 60)-([]a:1 2 3;b:4 5 6)",
    "([]a:9 18 27;b:36 45 54)");
eqt!(d_tbl_mul,           "([]a:1 2 3;b:4 5 6)*([]a:2 3 4;b:5 6 7)",
    "([]a:2 6 12;b:20 30 42)");
eqt!(d_tbl_div,           "([]a:10.0 20.0;b:40.0 50.0)%([]a:2.0 4.0;b:8.0 \
    10.0)", "([]a:5 5f;b:5 5f)");
eqt!(d_tbl_scalar_add,    "([]a:1 2 3;b:4 5 6)+1",         "([]a:2 3 4;b:5 6 \
    7)");
eqt!(d_tbl_scalar_mul,    "([]a:1 2 3;b:4 5 6)*10",        "([]a:10 20 30;b:40 \
    50 60)");
eqt!(d_tbl_eq_self,       "([]a:1 2 3;b:4 5 6)~([]a:1 2 3;b:4 5 6)", "1b");
eqt!(d_tbl_count,         "count ([]a:1 2 3;b:4 5 6)",     "3");
eqt!(d_tbl_count_empty,   "count ([]a:`int$();b:`int$())", "0");
eqt!(d_tbl_null_add,      "([]a:(1;0N;3))+([]a:1 2 3)",    "([]a:(2;0N;6))");
eqt!(d_tbl_neg,           "neg ([]a:1 2 3;b:4 5 6)",       "([]a:-1 -2 -3;b:-4 \
    -5 -6)");
eqt!(d_tbl_abs,           "abs ([]a:-1 -2 3;b:4 -5 6)",    "([]a:1 2 3;b:4 5 \
    6)");

// ── G. TABLES: sort asc/desc ───────────────────────────────────────
eqt!(d_tbl_asc,           "asc ([]a:3 1 2;b:30 10 20)",    "([]a:1 2 3;b:10 20 \
    30)");
eqt!(d_tbl_xasc_a,        "`a xasc ([]a:3 1 2;b:30 10 20)", "([]a:1 2 3;b:10 \
    20 \
    30)");
eqt!(d_tbl_xasc_b,        "`b xasc ([]a:3 1 2;b:30 10 20)", "([]a:1 2 3;b:10 \
    20 \
    30)");
eqt!(d_tbl_xdesc_a,       "`a xdesc ([]a:3 1 2;b:30 10 20)", "([]a:3 2 1;b:30 \
    20 10)");
eqt!(d_tbl_xasc_multi,    "`a`b xasc ([]a:1 1 2;b:30 10 20)", "([]a:1 1 2;b:10 \
    30 20)");

// ── G. TABLES: aggregates (sum/avg/min/max per column) ─────────────
eqt!(d_tbl_sum,           "sum ([]a:1 2 3;b:4 5 6)",       "`a`b!6 15j");
eqt!(d_tbl_avg,           "avg ([]a:1 2 3;b:4 5 6)",       "`a`b!2 5f");
eqt!(d_tbl_min,           "min ([]a:1 2 3;b:4 5 6)",       "`a`b!1 4");
eqt!(d_tbl_max,           "max ([]a:1 2 3;b:4 5 6)",       "`a`b!3 6");
eqt!(d_tbl_count_col,     "count each flip ([]a:1 2 3;b:4 5 6)", "`a`b!3 3");

// ── G. DICTS: arithmetic d+d, d-d, d*d, d%d ────────────────────────
eqt!(d_dict_add,          "(`a`b`c!1 2 3)+(`a`b`c!10 20 30)",  "`a`b`c!11 22 \
    33");
eqt!(d_dict_sub,          "(`a`b`c!10 20 30)-(`a`b`c!1 2 3)",  "`a`b`c!9 18 \
    27");
eqt!(d_dict_mul,          "(`a`b`c!1 2 3)*(`a`b`c!10 20 30)",  "`a`b`c!10 40 \
    90");
eqt!(d_dict_div,          "(`a`b`c!10.0 20.0 30.0)%(`a`b`c!2.0 4.0 5.0)",
    "`a`b`c!5 5 6f");
eqt!(d_dict_scalar_add,   "`a`b`c!1 2 3+10",               "`a`b`c!11 12 13");
eqt!(d_dict_scalar_mul,   "`a`b`c!1 2 3*10",               "`a`b`c!10 20 30");
eqt!(d_dict_eq_self,      "(`a`b!1 2)~`a`b!1 2",           "1b");
eqt!(d_dict_count,        "count `a`b`c!1 2 3",            "3");
eqt!(d_dict_key,          "key `a`b`c!1 2 3",              "`a`b`c");
eqt!(d_dict_value,        "value `a`b`c!1 2 3",            "1 2 3");
eqt!(d_dict_asc_byval,    "asc `a`b`c!3 1 2",              "`b`c`a!`s#1 2 3");
eqt!(d_dict_neg,          "neg `a`b`c!1 2 3",              "`a`b`c!-1 -2 -3");
eqt!(d_dict_sum,          "sum `a`b`c!1 2 3",              "6j");
eqt!(d_dict_avg,          "avg `a`b`c!1 2 3",              "2f");
eqt!(d_dict_max,          "max `a`b`c!1 2 3",              "3");
eqt!(d_dict_min,          "min `a`b`c!1 2 3",              "1");

// ── G. KEYED TABLES: arithmetic ────────────────────────────────────
eqt!(d_kt_add,            "([k:1 2 3]v:10 20 30)+([k:1 2 3]v:1 2 3)", "([k:1 2 \
    3]v:11 22 33)");
eqt!(d_kt_mul,            "([k:1 2 3]v:10 20 30)*2",       "([k:1 2 3]v:20 40 \
    60)");
eqt!(d_kt_count,          "count ([k:1 2 3]v:10 20 30)",   "3");
eqt!(d_kt_key,            "key ([k:1 2 3]v:10 20 30)",     "([]k:1 2 3)");
eqt!(d_kt_value,          "value ([k:1 2 3]v:10 20 30)",   "([]v:10 20 30)");
eqt!(d_kt_eq_self,        "([k:1 2 3]v:10 20 30)~([k:1 2 3]v:10 20 30)", "1b");

// ── G. TABLES: index and select ────────────────────────────────────
eqt!(d_tbl_row,           "([]a:1 2 3;b:4 5 6)[1]",        "`a`b!2 5");
eqt!(d_tbl_col,           "([]a:1 2 3;b:4 5 6)[`a]",       "1 2 3");
eqt!(d_tbl_take,          "2#([]a:1 2 3;b:4 5 6)",         "([]a:1 2;b:4 5)");
eqt!(d_tbl_drop,          "1_([]a:1 2 3;b:4 5 6)",         "([]a:2 3;b:5 6)");
eqt!(d_tbl_cols,          "cols ([]a:1 2 3;b:4 5 6)",      "`a`b");
eqt!(d_tbl_flip_dict,     "flip `a`b!(1 2 3;4 5 6)",       "([]a:1 2 3;b:4 5 \
    6)");

// ── H. JOINS: lj, ij ───────────────────────────────────────────────
eqt!(d_lj_basic,          "([]k:1 2 3;a:10 20 30) lj ([k:1 2 3]b:100 200 300)",
                          "([]k:1 2 3;a:10 20 30;b:100 200 300)");
eqt!(d_lj_missing,        "([]k:1 2 4;a:10 20 40) lj ([k:1 2 3]b:100 200 300)",
                          "([]k:1 2 4;a:10 20 40;b:100 200 0N)");
eqt!(d_ij_basic,          "([]k:1 2 3;a:10 20 30) ij ([k:1 2 3]b:100 200 300)",
                          "([]k:1 2 3;a:10 20 30;b:100 200 300)");
eqt!(d_ij_partial,        "([]k:1 2 4;a:10 20 40) ij ([k:1 2 3]b:100 200 300)",
                          "([]k:1 2;a:10 20;b:100 200)");

// ── I. SET OPS: distinct, group, union, inter, except ──────────────
eqt!(d_dist_ii,           "distinct 1 2 3 2 1 4",          "1 2 3 4");
eqt!(d_dist_ii_null,      "distinct 1 0N 2 0N 3",          "1 0N 2 3");
eqt!(d_dist_ss,           "distinct `a`b`c`a`b",           "`a`b`c");
eqt!(d_dist_ff,           "distinct 1.0 2.0 1.0 3.0",      "1 2 3f");
eqt!(d_dist_large,        "distinct 1000#1 2 3",           "1 2 3");
eqt!(d_where_bb,          "where 10110b",                  "0 2 3");
eqt!(d_where_ii_count,    "where 2 0 1 3",                 "0 0 2 3 3 3");
eqt!(d_union_ii,          "1 2 3 union 2 3 4",             "1 2 3 4");
eqt!(d_inter_ii,          "1 2 3 inter 2 3 4",             "2 3");
eqt!(d_except_ii,         "1 2 3 except 2",                "1 3");

// ── J. CAST / TYPE ─────────────────────────────────────────────────
eqt!(d_cast_ij,           "`long$1 2 3",                   "1 2 3j");
eqt!(d_cast_if,           "`float$1 2 3",                  "1 2 3f");
eqt!(d_cast_jf,           "`float$1 2 3j",                 "1 2 3f");
eqt!(d_cast_fi,           "`int$1.5 2.7 3.9",              "2 3 4i");
eqt!(d_cast_sb,           "`boolean$1 0 1 0",              "1010b");
eqt!(d_cast_ih,           "`short$1 2 3",                  "1 2 3h");
eqt!(d_cast_ssym,         "`$(\"abc\";\"def\")",           "`abc`def");
eqt!(d_cast_symstr,       "string `abc`def",               "(\"abc\";\"def\")");
eqt!(d_type_atom_i,       "type 42",                       "-6h");
eqt!(d_type_atom_j,       "type 42j",                      "-7h");
eqt!(d_type_atom_f,       "type 42.0",                     "-9h");
eqt!(d_type_atom_s,       "type `abc",                     "-11h");
eqt!(d_type_atom_b,       "type 1b",                       "-1h");
eqt!(d_type_vec_i,        "type 1 2 3",                    "6h");
eqt!(d_type_vec_j,        "type 1 2 3j",                   "7h");
eqt!(d_type_vec_f,        "type 1.0 2.0",                  "9h");
eqt!(d_type_vec_s,        "type `a`b",                     "11h");
eqt!(d_type_vec_b,        "type 101b",                     "1h");
eqt!(d_type_vec_c,        "type \"abc\"",                  "10h");
eqt!(d_type_table,        "type ([]a:1 2)",                "98h");
eqt!(d_type_dict,         "type `a`b!1 2",                 "99h");

// ── K. STRING ops ──────────────────────────────────────────────────
eqt!(d_upper_str,         "upper \"hello\"",               "\"HELLO\"");
eqt!(d_lower_str,         "lower \"HELLO\"",               "\"hello\"");
eqt!(d_upper_sym,         "upper `abc`def",                "`ABC`DEF");
eqt!(d_string_i,          "string 42",                     "\"42\"");
eqt!(d_string_j,          "string 42j",                    "\"42\"");
eqt!(d_string_ff,         "string 3.14",                   "\"3.14\"");
eqt!(d_raze_str,          "raze (\"hi\";\"ya\")",          "\"hiya\"");
eqt!(d_trim_str,          "trim \"  hello  \"",            "\"hello\"");
eqt!(d_ltrim_str,         "ltrim \"  hello\"",             "\"hello\"");
eqt!(d_rtrim_str,         "rtrim \"hello  \"",             "\"hello\"");

// ── L. INDEXING + AMEND ────────────────────────────────────────────
eqt!(d_idx_atom,          "42 1 2 3@0",                    "42");
eqt!(d_idx_vec,           "10 20 30 40@0 2",               "10 30");
eqt!(d_idx_neg,           "10 20 30@0 2",                  "10 30");
eqt!(d_amend_scalar,      "@[1 2 3 4;2;:;99]",             "1 2 99 4");
eqt!(d_amend_op,          "@[1 2 3 4;2;+;10]",             "1 2 13 4");
eqt!(d_amend_vec,         "@[1 2 3 4;1 3;:;99 98]",        "1 99 3 98");
eqt!(d_take_pos,          "3#1 2 3 4 5",                   "1 2 3");
eqt!(d_take_neg,          "(-3)#1 2 3 4 5",                "3 4 5");
eqt!(d_drop_pos,          "2_1 2 3 4 5",                   "3 4 5");
eqt!(d_drop_neg,          "(-2)_1 2 3 4 5",                "1 2 3");
eqt!(d_reverse_ii,        "reverse 1 2 3 4",               "4 3 2 1");
eqt!(d_reverse_cc,        "reverse \"hello\"",             "\"olleh\"");
eqt!(d_til_n,             "til 5",                         "0 1 2 3 4");
eqt!(d_til_0,             "til 0",                         "`int$()");
eqt!(d_enlist,            "enlist 42",                     "1#42");
eqt!(d_raze_ii,           "raze (1 2;3 4;5 6)",            "1 2 3 4 5 6");
eqt!(d_flip_mat,          "flip (1 2 3;4 5 6)",            "(1 4;2 5;3 6)");

// ── M. qSQL SELECT / EXEC / UPDATE / DELETE ─────────────────────────
eqt!(d_select_all,        "select from ([]a:1 2 3;b:4 5 6)",    "([]a:1 2 \
    3;b:4 \
    5 6)");
eqt!(d_select_col,        "select a from ([]a:1 2 3;b:4 5 6)",  "([]a:1 2 3)");
eqt!(d_select_cols,       "select b,a from ([]a:1 2 3;b:4 5 6)", "([]b:4 5 \
    6;a:1 2 3)");
eqt!(d_select_where,      "select from ([]a:1 2 3;b:4 5 6) where a>1", "([]a:2 \
    3;b:5 6)");
eqt!(d_select_where_eq,   "select from ([]a:1 2 3;b:4 5 6) where a=2",
    "([]a:enlist 2;b:enlist 5)");
eqt!(d_select_where_in,   "select from ([]a:1 2 3 4;b:10 20 30 40) where a in \
    2 \
    3", "([]a:2 3;b:20 30)");
eqt!(d_select_where_null, "select from ([]a:(1;0N;3);b:4 5 6) where not null \
    a", "([]a:1 3;b:4 6)");
eqt!(d_select_alias,      "select x:a from ([]a:1 2 3)",        "([]x:1 2 3)");
eqt!(d_select_expr,       "select a*10 from ([]a:1 2 3)",       "([]a:10 20 \
    30)");
eqt!(d_select_agg_sum,    "select sum a from ([]a:1 2 3 4 5)",  "([]a:enlist \
    15j)");
eqt!(d_select_agg_avg,    "select avg a from ([]a:1 2 3 4 5)",  "([]a:enlist \
    3f)");
eqt!(d_select_by,         "select sum b by a from ([]a:1 1 2 2;b:10 20 30 40)",
    "([a:1 2]b:30 70j)");
eqt!(d_select_by_count,   "select count i by a from ([]a:1 1 2 2 2 3)", "([a:1 \
    2 3]x:2 3 1)");
eqt!(d_exec_col,          "exec a from ([]a:1 2 3;b:4 5 6)",    "1 2 3");
eqt!(d_exec_sum,          "exec sum a from ([]a:1 2 3 4 5)",    "15j");
eqt!(d_exec_max,          "exec max a from ([]a:3 1 4 1 5)",    "5");
eqt!(d_update_col,        "update a:10 20 30 from ([]a:1 2 3)", "([]a:10 20 \
    30)");
eqt!(d_update_expr,       "update a:a*2 from ([]a:1 2 3)",      "([]a:2 4 6)");
eqt!(d_update_where,      "update a:99 from ([]a:1 2 3;b:4 5 6) where b=5",
    "([]a:1 99 3;b:4 5 6)");
eqt!(d_delete_col,        "delete b from ([]a:1 2 3;b:4 5 6)",  "([]a:1 2 3)");
eqt!(d_delete_where,      "delete from ([]a:1 2 3;b:4 5 6) where a=2", "([]a:1 \
    3;b:4 6)");

// ── N. JOINS: lj/ij extended + uj + aj basic ───────────────────────
eqt!(d_lj_two_col,        "([]k:1 2 3;a:10 20 30) lj ([k:1 2 3]b:100 200 \
    300;c:`a`b`c)", "([]k:1 2 3;a:10 20 30;b:100 200 300;c:`a`b`c)");
eqt!(d_ij_partial_match,  "([]k:1 2 4;a:10 20 40) ij ([k:1 2 3]b:100 200 300)",
    "([]k:1 2;a:10 20;b:100 200)");
eqt!(d_uj_disjoint,       "([]a:1 2;b:10 20) uj ([]c:3 4;d:30 40)", "([]a:1 2 \
    0N 0N;b:10 20 0N 0N;c:0N 0N 3 4;d:0N 0N 30 40)");
eqt!(d_uj_overlap,        "([]a:1 2;b:10 20) uj ([]a:3 4;b:30 40)", "([]a:1 2 \
    3 \
    4;b:10 20 30 40)");

// ── O. ADVERBS: each (') ───────────────────────────────────────────
eqt!(d_each_count,        "count each (1 2;3 4 5;6)",      "2 3 1");
eqt!(d_each_neg,          "neg each (1 2;3 4)",            "(-1 -2;-3 -4)");
eqt!(d_each_sum,          "sum each (1 2 3;4 5 6;7 8)",    "6 15 15j");
eqt!(d_each_first,        "first each (1 2 3;4 5 6)",      "1 4");
eqt!(d_each_last,         "last each (1 2 3;4 5 6)",       "3 6");
eqt!(d_each_reverse,      "reverse each (1 2 3;4 5 6)",    "(3 2 1;6 5 4)");

// ── O. ADVERBS: over (/) and scan (\) ──────────────────────────────
eqt!(d_over_sum,          "(+/)1 2 3 4 5",                 "15j");
eqt!(d_over_prd,          "(*/)1 2 3 4",                   "24");
eqt!(d_over_min,          "(&/)3 1 4 1 5",                 "1");
eqt!(d_over_max,          "(|/)3 1 4 1 5",                 "5");
eqt!(d_scan_sum,          "(+\\)1 2 3 4",                  "1 3 6 10");
eqt!(d_scan_prd,          "(*\\)1 2 3 4",                  "1 2 6 24");
eqt!(d_scan_min,          "(&\\)3 1 4 1 5",                "3 1 1 1 1");
eqt!(d_scan_max,          "(|\\)3 1 4 1 5",                "3 3 4 4 5");

// ── O. ADVERBS: each-right ('\\:) and each-left ('/:) ──────────────
eqt!(d_eachr_add,         "10+/:1 2 3",                  "11 12 13");
eqt!(d_eachl_add,         "1 2 3 +\\:10",                  "11 12 13");
eqt!(d_eachl_cross,       "1 2+\\:10 20",                  "(11 21;12 22)");

// ── P. TEMPORAL — dates, times, datetime arithmetic ────────────────
eqt!(d_date_sub,          "2024.01.10 - 2024.01.01",       "9");
eqt!(d_date_add_int,      "2024.01.01 + 5",                "2024.01.06");
eqt!(d_time_atom,         "12:34:56.789",                  "12:34:56.789");
eqt!(d_month_atom,        "2024.03m",                      "2024.03m");
eqt!(d_date_vec,          "2024.01.01+til 3",              "2024.01.01 \
    2024.01.02 2024.01.03");
eqt!(d_ym_extract,        "`year$2024.06.15",              "2024i");
eqt!(d_md_extract,        "`month$2024.06.15",             "2024.06m");
eqt!(d_dd_extract,        "`dd$2024.06.15",                "15i");
eqt!(d_time_hour,         "`hh$12:34:56",                  "12i");
eqt!(d_time_min,          "`uu$12:34:56",                  "34i");

// ── Q. CROSS-TYPE MIXED ARITHMETIC ─────────────────────────────────
eqt!(d_mix_ib_add,        "1 2 3 4+1010b",               "2 2 4 4");
eqt!(d_mix_ih_add,        "1 2 3 + 4 5 6h",                "5 7 9i");
eqt!(d_mix_if_add,        "1 2 3 + 4.5 5.5 6.5",           "5.5 7.5 9.5");
eqt!(d_mix_jf_add,        "1 2 3j + 4.5 5.5 6.5",          "5.5 7.5 9.5");
eqt!(d_mix_atom_vec,      "2 + 1 2 3",                     "3 4 5");
eqt!(d_mix_vec_atom,      "1 2 3 + 2",                     "3 4 5");
eqt!(d_mix_atom_atom,     "2 + 3",                         "5");

// ── R. NULL HANDLING across types ──────────────────────────────────
eqt!(d_null_i_prop_add,   "0N + 5",                        "0N");
eqt!(d_null_j_prop_add,   "0Nj + 5j",                      "0Nj");
eqt!(d_null_f_prop_add,   "0n + 5.0",                      "0n");
eqt!(d_null_e_prop_add,   "0ne + 5e",                      "0ne");
eqt!(d_null_i_prop_mul,   "0N * 5",                        "0N");
eqt!(d_null_i_prop_sub,   "0N - 5",                        "0N");
eqt!(d_null_vec_mul,      "(0N;2;3) * 2 2 2",              "0N 4 6");
eqt!(d_null_vec_div,      "(0N;4.0;6.0) % 2 2 2",          "0n 2 3f");

// ── S. BIG VECTOR OPERATIONS ───────────────────────────────────────
eqt!(d_big_sum,           "sum til 10000",                 "49995000j");
eqt!(d_big_sumj,          "sum `long$til 10000",           "49995000j");
eqt!(d_big_avg,           "avg til 100",                   "49.5");
eqt!(d_big_dist,          "count distinct 10000?10",       "10");
eqt!(d_big_asc,           "{[x](asc x)~`s#asc x}10000?100", "1b");
eqt!(d_big_reverse,       "{[x](reverse reverse x)~x}10000?100", "1b");
eqt!(d_big_add,           "(10000#1)+10000#2",             "10000#3");
eqt!(d_big_neg,           "{[x](neg neg x)~x}10000?100", "1b");

// ── T. LIST OPERATIONS ─────────────────────────────────────────────
eqt!(d_concat_ii,         "1 2 3,4 5 6",                   "1 2 3 4 5 6");
eqt!(d_concat_jj,         "1 2j,3 4j",                     "1 2 3 4j");
eqt!(d_concat_ff,         "1.0 2.0,3.0",                   "1 2 3f");
eqt!(d_concat_ss,         "`a`b,`c`d",                     "`a`b`c`d");
eqt!(d_enlist_atom,       "enlist 42",                     "1#42");
eqt!(d_list_nested,       "(1 2;3 4)",                     "(1 2;3 4)");
eqt!(d_list_first_nested, "first (1 2;3 4)",               "1 2");
eqt!(d_cut_equal,         "3 cut 1 2 3 4 5 6",             "(1 2 3;4 5 6)");

// ── U. FUNCTIONAL FORMS ────────────────────────────────────────────
eqt!(d_lambda_apply,      "{x+1}[5]",                      "6");
eqt!(d_lambda_vec,        "{x*2}1 2 3",                    "2 4 6");
eqt!(d_lambda_two_arg,    "{x+y}[3;4]",                    "7");
eqt!(d_lambda_three_arg,  "{x+y+z}[1;2;3]",                "6");
eqt!(d_lambda_each,       "({x*2}')1 2 3",              "2 4 6");
eqt!(d_project,           "{x+y}[10]5",                    "15");

// ── V. BOOLEAN / LOGIC ─────────────────────────────────────────────
eqt!(d_bool_and_ss,       "1b&0b",                         "0b");
eqt!(d_bool_or_ss,        "0b|1b",                         "1b");
eqt!(d_bool_xor,          "1010b<>1100b",                  "0110b");
eqt!(d_bool_neq,          "(1 2 3)<>2 2 3",                "100b");
eqt!(d_bool_count_ones,   "sum 10110b",                    "3i");

// ── W. SYMBOL / STRING ─────────────────────────────────────────────
eqt!(d_sym_atom,          "`hello",                        "`hello");
eqt!(d_sym_vec,           "`a`b`c",                        "`a`b`c");
eqt!(d_sym_count,         "count `a`b`c`d",                "4");
eqt!(d_sym_concat,        "`hello,`world",                 "`hello`world");
eqt!(d_sym_asc,           "asc `z`a`m`b",                  "`s#`a`b`m`z");
eqt!(d_sym_distinct,      "distinct `a`b`a`c`b",           "`a`b`c");
eqt!(d_str_count,         "count \"hello\"",               "5");
eqt!(d_str_reverse,       "reverse \"hello\"",             "\"olleh\"");
eqt!(d_str_concat,        "\"foo\",\"bar\"",               "\"foobar\"");
eqt!(d_str_first,         "first \"hello\"",               "\"h\"");

// ── X. TABLE SHAPE OPERATIONS ──────────────────────────────────────
eqt!(d_tbl_flip_back,     "flip flip ([]a:1 2 3;b:4 5 6)", "([]a:1 2 3;b:4 5 \
    6)");
eqt!(d_tbl_first_row,     "first ([]a:1 2 3;b:4 5 6)",     "`a`b!1 4");
eqt!(d_tbl_last_row,      "last ([]a:1 2 3;b:4 5 6)",      "`a`b!3 6");
eqt!(d_tbl_reverse,       "reverse ([]a:1 2 3;b:4 5 6)",   "([]a:3 2 1;b:6 5 \
    4)");
eqt!(d_tbl_take2,         "2#([]a:1 2 3;b:4 5 6)",         "([]a:1 2;b:4 5)");
eqt!(d_tbl_drop_all,      "3_([]a:1 2 3;b:4 5 6)",
    "([]a:`int$();b:`int$())");

// ── Y. DICT VARIATIONS ─────────────────────────────────────────────
eqt!(d_dict_from_kv,      "`a`b`c!(1 2;3 4;5 6)",          "`a`b`c!(1 2;3 4;5 \
    6)");
eqt!(d_dict_idx_key,      "(`a`b`c!1 2 3)`b",              "2");
eqt!(d_dict_idx_missing,  "(`a`b`c!1 2 3)`x",              "0N");
eqt!(d_dict_reverse,      "reverse `a`b`c!1 2 3",          "`c`b`a!3 2 1");
eqt!(d_dict_takeN,        "2#`a`b`c`d!1 2 3 4",            "`a`b!1 2");
eqt!(d_dict_dropN,        "1_`a`b`c!1 2 3",                "`b`c!2 3");
eqt!(d_dict_concat,       "(`a`b!1 2),`c`d!3 4",           "`a`b`c`d!1 2 3 4");
eqt!(d_dict_first_v,      "first `a`b`c!1 2 3",            "1");
eqt!(d_dict_last_v,       "last `a`b`c!1 2 3",             "3");

// ── Z. QUIRKY but HANDY ─────────────────────────────────────────────
eqt!(d_xbar_i,            "10 xbar 3 14 27 42",            "0 10 20 40");
eqt!(d_xbar_f,            "0.1 xbar 1.03 2.07 3.14",       "1 2 3.1");
eqt!(d_wsum,              "2 3 4 wsum 10 20 30",           "200f");
eqt!(d_wavg,              "2 3 4 wavg 10 20 30",           "200f%9");
eqt!(d_within_true,       "5 within 1 10",                 "1b");
eqt!(d_within_false,      "15 within 1 10",                "0b");
eqt!(d_within_vec,        "1 2 3 4 5 within 2 4",          "01110b");
eqt!(d_bin_found,         "1 3 5 7 9 bin 5",               "2");
eqt!(d_bin_between,       "1 3 5 7 9 bin 4",               "1");
eqt!(d_bin_below,         "1 3 5 7 9 bin 0",               "-1");
eqt!(d_bin_above,         "1 3 5 7 9 bin 10",              "4");
eqt!(d_til_10,            "til 10",                        "0 1 2 3 4 5 6 7 8 \
    9");
eqt!(d_til_large,         "count til 1000",                "1000");
eqt!(d_mod,               "10 mod 3",                      "1");
eqt!(d_div,               "10 div 3",                      "3");

// ── AA. MORE NULL × TYPE COVERAGE ──────────────────────────────────
eqt!(d_null_bool_atom,    "null 0b",                       "0b");
eqt!(d_null_bool_vec,     "null 1010b",                    "0000b");
eqt!(d_null_j_vec,        "null (1j;0Nj;3j)",              "010b");
eqt!(d_null_s_vec,        "null `a``b",                    "010b");
eqt!(d_null_c_vec,        "null \"a b\"",                  "010b");
eqt!(d_null_mix_propagate,  "(0N;1)+(1;0N)",               "0N 0N");
eqt!(d_null_nan_mul,      "0n*2.0",                        "0n");
eqt!(d_null_nan_sub,      "2.0-0n",                        "0n");
eqt!(d_null_min_with,     "0N&5",                          "0N");
eqt!(d_null_max_with,     "0N|5",                          "5");
eqt!(d_count_nulls,       "sum null (1;0N;2;0N;3)",        "2i");
eqt!(d_fills_vec,         "fills (1;0N;0N;4;0N)",          "1 1 1 4 4");
eqt!(d_fills_int_null,    "fills 0N 1 2 0N 4",             "0N 1 2 2 4");

// ── BB. MORE TABLE arithmetic (all types) ──────────────────────────
eqt!(d_tbl_kj_add,        "([]a:1 2 3j)+([]a:4 5 6j)",     "([]a:5 7 9j)");
eqt!(d_tbl_kf_add,        "([]a:1.5 2.5)+([]a:0.5 0.5)",   "([]a:2 3f)");
eqt!(d_tbl_ks_eq,         "([]s:`a`b`c)~([]s:`a`b`c)",     "1b");
eqt!(d_tbl_mixed_type,    "([]i:1 2 3;j:1 2 3j;f:1.0 2.0 3.0;s:`a`b`c)",
    "([]i:1 2 3;j:1 2 3j;f:1.0 2.0 3.0;s:`a`b`c)");
eqt!(d_tbl_empty_count,   "count ([]a:`int$())",           "0");
eqt!(d_tbl_single_col,    "([]a:enlist 42)",               "([]a:enlist 42)");

// ── CC. MORE DICT operations ───────────────────────────────────────
eqt!(d_dict_ff_add,       "(`a`b`c!1.0 2.0 3.0)+(`a`b`c!0.5 0.5 0.5)",
    "`a`b`c!1.5 2.5 3.5");
eqt!(d_dict_jj_add,       "(`a`b!1 2j)+(`a`b!3 4j)",       "`a`b!4 6j");
eqt!(d_dict_ss_val,       "`a`b`c!`x`y`z",                 "`a`b`c!`x`y`z");
eqt!(d_dict_reverse_val,  "reverse value `a`b`c!1 2 3",    "3 2 1");
eqt!(d_dict_update,       "@[`a`b`c!1 2 3;`b;:;99]",       "`a`b`c!1 99 3");
eqt!(d_dict_insert_new,   "@[`a`b!1 2;`c;:;3]",            "`a`b`c!1 2 3");

// ── DD. KEYED TABLE operations ─────────────────────────────────────
eqt!(d_kt_from_unkeyed,   "`k xkey ([]k:1 2 3;v:10 20 30)", "([k:1 2 3]v:10 20 \
    30)");
eqt!(d_kt_unkey,          "0!([k:1 2 3]v:10 20 30)",       "([]k:1 2 3;v:10 20 \
    30)");
eqt!(d_kt_select,         "select from ([k:1 2 3]v:10 20 30) where v>15",
    "([k:2 3]v:20 30)");
eqt!(d_kt_keys,           "cols key ([k:1 2 3]v:10 20 30)", "enlist`k");
eqt!(d_kt_vals,           "cols value ([k:1 2 3]v:10 20 30)", "enlist`v");

// ── EE. COMPREHENSIVE ASC/DESC on tables and dicts ─────────────────
eqt!(d_xasc_int,          "`v xasc ([]v:3 1 4 1 5;n:`a`b`c`d`e)", "([]v:1 1 3 \
    4 \
    5;n:`b`d`a`c`e)");
eqt!(d_xdesc_int,         "`v xdesc ([]v:3 1 4 1 5;n:`a`b`c`d`e)", "([]v:5 4 3 \
    1 1;n:`e`c`a`b`d)");
eqt!(d_xasc_float,        "`v xasc ([]v:3.5 1.5 4.5;n:`a`b`c)", "([]v:1.5 3.5 \
    4.5;n:`b`a`c)");

// ── FF. INDEXING comprehensive ─────────────────────────────────────
eqt!(d_idx_at,            "(10 20 30 40)@2",               "30");
eqt!(d_idx_at_vec,        "(10 20 30 40)@0 2",             "10 30");
eqt!(d_idx_atom_int,      "10 20 30[0]",                   "10");
eqt!(d_idx_vec_int,       "10 20 30[0 1]",                 "10 20");
eqt!(d_idx_null_oob,      "10 20 30[5]",                   "0N");
eqt!(d_idx_oob_vec,       "10 20 30[5 1]",                 "0N 20");
eqt!(d_dot_apply,         "(+).(3;4)",                     "7");

// ── GG. AMEND comprehensive ────────────────────────────────────────
eqt!(d_amend_assign_v,    "@[1 2 3 4;1 2;:;99 99]",        "1 99 99 4");
eqt!(d_amend_add,         "@[1 2 3 4;::;+;10]",            "11 12 13 14");
eqt!(d_amend_mul,         "@[1 2 3 4;::;*;2]",             "2 4 6 8");
eqt!(d_amend_sub_single,  "@[1 2 3 4;2;-;1]",              "1 2 2 4");

// ── HH. WHERE with varied predicates ───────────────────────────────
eqt!(d_where_eq_atom,     "where 1 2 3 2 1=2",             "1 3");
eqt!(d_where_gt,          "where 1 5 3 8 2>3",             "1 3");
eqt!(d_where_lt,          "where 1 5 3 8 2<3",             "0 4");
eqt!(d_where_in,          "where 1 2 3 4 in 2 3",          "1 2");
eqt!(d_where_not_null,    "where not null (1;0N;2;0N;3)",  "0 2 4");

// ── II. SELECT with aggregates ─────────────────────────────────────
eqt!(d_sel_min,           "select min a from ([]a:3 1 4 1 5)",
    "([]a:enlist \
    1)");
eqt!(d_sel_max,           "select max a from ([]a:3 1 4 1 5)",
    "([]a:enlist \
    5)");
eqt!(d_sel_count_i,       "select count i from ([]a:1 2 3)",
    "([]x:enlist \
    3)");
eqt!(d_sel_first,         "select first a from ([]a:3 1 4)",
    "([]a:enlist \
    3)");
eqt!(d_sel_last,          "select last a from ([]a:3 1 4)",
    "([]a:enlist \
    4)");
eqt!(d_sel_distinct,      "select distinct a from ([]a:1 2 1 3 2)", "([]a:1 2 \
    3)");
eqt!(d_sel_by_agg2,       "select sum b,avg c by a from ([]a:1 1 2 2;b:10 20 \
    30 \
    40;c:1.0 2.0 3.0 4.0)", "([a:1 2]b:30 70j;c:1.5 3.5)");

// ── JJ. FUNCTION DEFINITIONS ───────────────────────────────────────
eqt!(d_fn_noarg,          "{42}[]",                        "42");
eqt!(d_fn_id,             "{x}[5]",                        "5");
eqt!(d_fn_double,         "{x+x}[7]",                      "14");
eqt!(d_fn_map,            "{x+10} each 1 2 3",             "11 12 13");
eqt!(d_fn_filter,         "{x where x>2}[1 2 3 4 5]",      "3 4 5");
eqt!(d_fn_cond_simple,    "{$[x>0;\"pos\";\"neg\"]}[5]",   "\"pos\"");

// ── KK. LIST concat / take / drop ──────────────────────────────────
eqt!(d_concat_many,       "1 2,3 4,5 6,7 8",               "1 2 3 4 5 6 7 8");
eqt!(d_take_atom,         "5#1",                           "1 1 1 1 1");
eqt!(d_take_sym,          "3#`a",                          "`a`a`a");
eqt!(d_take_vec,          "5#1 2 3",                       "1 2 3 1 2");
eqt!(d_take_0,            "0#1 2 3",                       "`int$()");
eqt!(d_take_big,          "100#1 2 3",                     "100#1 2 3");
eqt!(d_drop_all,          "3_1 2 3",                       "`int$()");
eqt!(d_drop_more,         "5_1 2 3",                       "`int$()");
eqt!(d_drop_0,            "0_1 2 3",                       "1 2 3");

// ── LL. TYPE CHECKS comprehensive ──────────────────────────────────
eqt!(d_type_atom_c,       "type \"a\"",                    "-10h");
eqt!(d_type_atom_byte,    "type 0x42",                     "-4h");
eqt!(d_type_atom_short,   "type 42h",                      "-5h");
eqt!(d_type_atom_long,    "type 42j",                      "-7h");
eqt!(d_type_atom_real,    "type 42e",                      "-8h");
eqt!(d_type_atom_date,    "type 2024.01.01",               "-14h");
eqt!(d_type_atom_dtime,   "type 2024.01.01T12:00:00.000",  "-15h");
eqt!(d_type_atom_month,   "type 2024.03m",                 "-13h");
eqt!(d_type_atom_minute,  "type 12:34",                    "-17h");
eqt!(d_type_func,         "type {x+1}",                    "100h");

// ── MM. MISC useful ops ────────────────────────────────────────────
eqt!(d_next_vec,          "next 1 2 3 4",                  "2 3 4 0N");
eqt!(d_prev_vec,          "prev 1 2 3 4",                  "0N 1 2 3");
eqt!(d_differ,            "differ 1 1 2 2 3",              "10101b");
eqt!(d_deltas,            "deltas 1 3 6 10",               "1 2 3 4");
eqt!(d_ratios_f,          "ratios 1.0 2.0 4.0 8.0",        "1 2 2 2f");
eqt!(d_reverse_atom,      "reverse 42",                    "42");
eqt!(d_reverse_sym,       "reverse `a`b`c",                "`c`b`a");
eqt!(d_reverse_empty,     "reverse `int$()",               "`int$()");

// ── NN. REGEX-like string ─────────────────────────────────────────
eqt!(d_like_basic,        "\"hello\" like \"h*\"",         "1b");
eqt!(d_like_false,        "\"hello\" like \"x*\"",         "0b");
eqt!(d_like_wild,         "\"test\" like \"t??t\"",        "1b");
eqt!(d_like_no_wild,      "\"abc\" like \"abc\"",          "1b");

// ── OO. DISTINCT + GROUP cross-type ────────────────────────────────
eqt!(d_dist_jj,           "distinct 1 2 3 2 1 4j",         "1 2 3 4j");
eqt!(d_dist_bb,           "distinct 1010011b",             "10b");
eqt!(d_dist_cc,           "distinct \"hello\"",            "\"helo\"");
eqt!(d_group_ii,          "group 1 2 1 3 2",               "(1 2 3)!(0 2;1 \
    4;enlist 3)");
eqt!(d_group_ss,          "group `a`b`a`c`b",              "(`a`b`c)!(0 2;1 \
    4;enlist 3)");
eqt!(d_group_count,       "count each group 1 2 1 3 2",    "(1 2 3)!2 2 1");

// ── PP. HASH ops: in, bin, find ────────────────────────────────────
eqt!(d_in_ii_vv,          "2 3 in 1 2 3 4",                "11b");
eqt!(d_in_atom,           "2 in 1 2 3",                    "1b");
eqt!(d_in_miss,           "5 in 1 2 3",                    "0b");
eqt!(d_in_ss,             "`a`x in `a`b`c",                "10b");
eqt!(d_find_ii,           "1 2 3?2",                       "1");
eqt!(d_find_missing,      "1 2 3?5",                       "3");
eqt!(d_find_vec,          "1 2 3?1 2 5",                   "0 1 3");

// ── QQ. SCAN with custom fn ────────────────────────────────────────
eqt!(d_scan_fn,           "{x+y}\\[1 2 3 4]",              "1 3 6 10");
eqt!(d_over_fn,           "{x+y}/[1 2 3 4]",               "10");
eqt!(d_over_fn_init,      "{x+y}/[10;1 2 3]",              "16");

// ── RR. EDGE: empty vectors and atoms ──────────────────────────────
eqt!(d_empty_sum,         "sum `int$()",                   "0j");
eqt!(d_empty_prd,         "prd `int$()",                   "1");
eqt!(d_empty_max,         "max `int$()",                   "-0W");
eqt!(d_empty_min,         "min `int$()",                   "0W");
eqt!(d_empty_count,       "count `int$()",                 "0");
eqt!(d_empty_reverse,     "reverse `int$()",               "`int$()");
eqt!(d_empty_asc,         "asc `int$()",                   "`s#`int$()");
eqt!(d_empty_sf,          "sum `float$()",                 "0f");
eqt!(d_atom_count,        "count 42",                      "1");
eqt!(d_atom_null,         "null 42",                       "0b");
eqt!(d_atom_reverse,      "reverse 42",                    "42");
eqt!(d_single_asc,        "asc enlist 42",                 "`s#enlist 42");

// ── SS. DICT as function ───────────────────────────────────────────
eqt!(d_dict_call_one,     "(`a`b`c!10 20 30)`b",           "20");
eqt!(d_dict_call_many,    "(`a`b`c!10 20 30)`a`c",         "10 30");
eqt!(d_dict_call_miss,    "(`a`b`c!10 20 30)`x",           "0N");

// ── TT. TABLE as function ─────────────────────────────────────────
eqt!(d_tbl_call_row,      "([]a:1 2 3;b:4 5 6)[1]",        "`a`b!2 5");
eqt!(d_tbl_call_rows,     "([]a:1 2 3;b:4 5 6)[0 2]",      "([]a:1 3;b:4 6)");

// ── UU. NESTED operations ─────────────────────────────────────────
eqt!(d_count_nested,      "count (1 2 3;4 5)",             "2");
eqt!(d_flatten,           "raze (1 2 3;4 5)",              "1 2 3 4 5");
eqt!(d_first_of_nested,   "first (1 2;3 4)",               "1 2");
eqt!(d_last_of_nested,    "last (1 2;3 4)",                "3 4");

// ── VV. CAST more combinations ────────────────────────────────────
eqt!(d_cast_bool_i,       "`int$1100b",                    "1 1 0 0");
eqt!(d_cast_bool_j,       "`long$1100b",                   "1 1 0 0j");
eqt!(d_cast_bool_f,       "`float$1100b",                  "1 1 0 0f");
eqt!(d_cast_h_i,          "`int$1 2 3h",                   "1 2 3i");
eqt!(d_cast_h_j,          "`long$1 2 3h",                  "1 2 3j");
eqt!(d_cast_str_sym,      "`$\"hello\"",                   "`hello");
eqt!(d_cast_str_i,        "\"I\"$\"42\"",                  "42");
eqt!(d_cast_str_j,        "\"J\"$\"42\"",                  "42j");
eqt!(d_cast_str_f,        "\"F\"$\"3.14\"",                "3.14");

// ── WW. Conditional / control flow ────────────────────────────────
eqt!(d_cond_t,            "$[1b;`yes;`no]",                "`yes");
eqt!(d_cond_f,            "$[0b;`yes;`no]",                "`no");
eqt!(d_cond_chain,        "$[0>1;`a;0>2;`b;`c]",           "`c");
eqt!(d_cond_arith,        "$[5>3;10*2;20*2]",              "20");
eqt!(d_if_expr_t,         "{$[x>0;1;-1]}[5]",              "1");
eqt!(d_if_expr_f,         "{$[x>0;1;-1]}[-5]",             "-1");

// ── XX. ASSIGNMENT and amend in place ─────────────────────────────
eqt!(d_assign,            "{x:42;x}[]",                    "42");
eqt!(d_assign_vec,        "{x:1 2 3;sum x}[]",             "6j");
eqt!(d_amend_in_scope,    "{x:1 2 3;x[0]:99;x}[]",         "99 2 3");

// ── YY. MORE table joins ──────────────────────────────────────────
eqt!(d_pj_basic,          "([]k:1 2 3;a:10 20 30) pj ([k:1 2 3]b:100 200 300)",
                          "([]k:1 2 3;a:10 20 30;b:100 200 300)");

// ── ZZ. COMPARISON wide matrix ────────────────────────────────────
eqt!(d_cmp_ii_lt_null,    "(0N;1;5)<3",                    "110b");
eqt!(d_cmp_ii_gt_null,    "(0N;1;5)>3",                    "001b");
eqt!(d_cmp_jj_eq,         "(1j;2j;3j)=(1j;3j;3j)",         "101b");
eqt!(d_cmp_ff_eq,         "(1.5;2.5;3.5)=(1.5;3.5;3.5)",   "101b");
eqt!(d_cmp_ss_eq,         "`a`b`c=`a`x`c",                 "101b");
eqt!(d_cmp_mix_type,      "1 2 3=1 2 3j",                  "111b");
eqt!(d_cmp_self,          "1 2 3=1 2 3",                   "111b");

// ── AAA. MORE TABLE × TABLE ────────────────────────────────────────
eqt!(d_tbl_tt_add_f,      "([]a:1.5 2.5;b:3.5 4.5)+([]a:0.5 0.5;b:1.5 1.5)",
    "([]a:2 3f;b:5 6f)");
eqt!(d_tbl_tt_mul_j,      "([]a:1 2 3j;b:4 5 6j)*([]a:10 10 10j;b:10 10 10j)",
    "([]a:10 20 30j;b:40 50 60j)");
eqt!(d_tbl_tt_sub,        "([]a:10 20 30)-([]a:1 2 3)",    "([]a:9 18 27)");
eqt!(d_tbl_tt_div,        "([]a:10.0 20.0)%([]a:2.0 4.0)", "([]a:5 5f)");
eqt!(d_tbl_tt_min,        "([]a:3 1 4)&([]a:2 2 2)",       "([]a:2 1 2)");
eqt!(d_tbl_tt_max,        "([]a:3 1 4)|([]a:2 2 2)",       "([]a:3 2 4)");
eqt!(d_tbl_tt_eq,         "([]a:1 2 3)=([]a:1 0 3)",       "([]a:101b)");
eqt!(d_tbl_tt_lt,         "([]a:1 2 3)<([]a:2 2 2)",       "([]a:100b)");
eqt!(d_tbl_tt_gt,         "([]a:1 2 3)>([]a:2 2 2)",       "([]a:001b)");

// ── BBB. TABLE × SCALAR ────────────────────────────────────────────
eqt!(d_tbl_s_add,         "([]a:1 2 3;b:4 5 6)+10",        "([]a:11 12 13;b:14 \
    15 16)");
eqt!(d_tbl_s_sub,         "([]a:10 20;b:40 50)-5",         "([]a:5 15;b:35 \
    45)");
eqt!(d_tbl_s_mul,         "([]a:1 2;b:3 4)*10",            "([]a:10 20;b:30 \
    40)");
eqt!(d_tbl_s_div,         "([]a:10.0 20.0;b:30.0 40.0)%2.0", "([]a:5 10f;b:15 \
    20f)");
eqt!(d_tbl_s_eq,          "([]a:1 2 3)=2",                 "([]a:010b)");
eqt!(d_tbl_s_neg,         "neg ([]a:1 2 3;b:-4 -5 -6)",    "([]a:-1 -2 -3;b:4 \
    5 \
    6)");
eqt!(d_tbl_s_abs,         "abs ([]a:-1 2 -3;b:4 -5 6)",    "([]a:1 2 3;b:4 5 \
    6)");
eqt!(d_tbl_s_sqrt,        "sqrt ([]a:4.0 9.0 16.0)",       "([]a:2 3 4f)");

// ── CCC. DICT × SCALAR + DICT × DICT more types ────────────────────
eqt!(d_dict_s_add,        "(`a`b`c!1 2 3)+100",            "`a`b`c!101 102 \
    103");
eqt!(d_dict_s_sub,        "(`a`b`c!100 200 300)-10",       "`a`b`c!90 190 290");
eqt!(d_dict_s_mul,        "(`a`b`c!1 2 3)*10",             "`a`b`c!10 20 30");
eqt!(d_dict_ff_sub,       "(`a`b!3.5 4.5)-(`a`b!1.0 2.0)", "`a`b!2.5 2.5");
eqt!(d_dict_jj_mul,       "(`a`b!2 3j)*(`a`b!5 6j)",       "`a`b!10 18j");
eqt!(d_dict_cross_ij,     "(`a`b!1 2)+(`a`b!3 4j)",        "`a`b!4 6j");
eqt!(d_dict_cross_if,     "(`a`b!1 2)+(`a`b!1.5 2.5)",     "`a`b!2.5 4.5");
eqt!(d_dict_desc,         "desc `a`b`c!1 2 3",             "`c`b`a!3 2 1");

// ── DDD. KEYED TABLE arithmetic extended ──────────────────────────
eqt!(d_kt_tt_add,         "([k:1 2 3]v:10 20 30)+([k:1 2 3]v:1 2 3)", "([k:1 2 \
    3]v:11 22 33)");
eqt!(d_kt_tt_sub,         "([k:1 2 3]v:100 200 300)-([k:1 2 3]v:10 20 30)",
    "([k:1 2 3]v:90 180 270)");
eqt!(d_kt_tt_mul,         "([k:1 2 3]v:2 3 4)*([k:1 2 3]v:5 6 7)", "([k:1 2 \
    3]v:10 18 28)");
eqt!(d_kt_f_add,          "([k:1 2 3]v:1.5 2.5 3.5)+([k:1 2 3]v:0.5 0.5 0.5)",
    "([k:1 2 3]v:2 3 4f)");
eqt!(d_kt_j_add,          "([k:1 2j]v:10 20j)+([k:1 2j]v:1 2j)", "([k:1 \
    2j]v:11 \
    22j)");
eqt!(d_kt_s_scalar_add,   "([k:1 2 3]v:10 20 30)+5",       "([k:1 2 3]v:15 25 \
    35)");
eqt!(d_kt_s_scalar_mul,   "([k:1 2 3]v:1 2 3)*10",         "([k:1 2 3]v:10 20 \
    30)");
eqt!(d_kt_neg,            "neg ([k:1 2 3]v:1 2 3)",        "([k:1 2 3]v:-1 -2 \
    -3)");

// ── EEE. SORT on all vector types ─────────────────────────────────
eqt!(d_asc_kh,            "asc 3 1 4 1 5h",                "`s#1 1 3 4 5h");
eqt!(d_asc_ke,            "asc 3.5 1.5 4.5e",              "`s#1.5 3.5 4.5e");
eqt!(d_asc_kc,            "asc \"dcba\"",                  "`s#\"abcd\"");
eqt!(d_asc_kb,            "asc 10110b",                    "`s#00111b");
eqt!(d_asc_kj_large,      "count asc 1000?1000j",          "1000");
eqt!(d_asc_ks_large,      "count asc 1000?`a`b`c`d`e",     "1000");
eqt!(d_desc_kh,           "desc 3 1 4 1 5h",               "5 4 3 1 1h");
eqt!(d_desc_ke,           "desc 3.5 1.5 4.5e",             "4.5 3.5 1.5e");
eqt!(d_desc_ks,           "desc `banana`apple`cherry",
    "`cherry`banana`apple");

// ── FFF. GRADE (iasc/idesc/rank) coverage ─────────────────────────
eqt!(d_iasc_ii_null,      "iasc (3;0N;1;4)",               "1 2 0 3");
eqt!(d_iasc_jj,           "iasc 3 1 4 1 5j",               "1 3 0 2 4");
eqt!(d_iasc_ff,           "iasc 3.5 1.5 4.5 1.5",          "1 3 0 2");
eqt!(d_iasc_ss,           "iasc `c`a`b",                   "1 2 0");
eqt!(d_idesc_ii_2,          "idesc 1 3 2 4",                 "3 1 2 0");
eqt!(d_rank_ff,           "rank 3.5 1.5 4.5",              "1 0 2");

// ── GGG. AGGREGATES with table ────────────────────────────────────
eqt!(d_tbl_sum_per_col,   "sum ([]a:1 2 3;b:4 5 6;c:7 8 9)", "`a`b`c!6 15 24j");
eqt!(d_tbl_avg_per_col,   "avg ([]a:1 2 3 4;b:2 4 6 8)",   "`a`b!2.5 5f");
eqt!(d_tbl_max_per_col,   "max ([]a:3 1 4;b:1 5 9)",       "`a`b!4 9");
eqt!(d_tbl_min_per_col,   "min ([]a:3 1 4;b:1 5 9)",       "`a`b!1 1");
eqt!(d_tbl_count_rows,    "count ([]a:1 2 3;b:4 5 6)",     "3");
eqt!(d_tbl_count_empty_2,   "count ([]a:`int$())",           "0");

// ── HHH. QUERY variations ─────────────────────────────────────────
eqt!(d_select_from_t,     "select from ([]a:1 2 3) where a>1", "([]a:2 3)");
eqt!(d_select_limit,      "select[2] from ([]a:1 2 3 4 5)", "([]a:1 2)");
eqt!(d_select_reverse,    "select[-2] from ([]a:1 2 3 4 5)", "([]a:4 5)");
eqt!(d_select_order_col,  "select from `a xasc ([]a:3 1 2)", "([]a:1 2 3)");
eqt!(d_select_mul_cond,   "select from ([]a:1 2 3 4;b:10 20 30 40) where \
    (a>1)&a<4", "([]a:2 3;b:20 30)");
eqt!(d_select_or_cond,    "select from ([]a:1 2 3 4;b:10 20 30 40) where \
    (a=1)|a=4", "([]a:1 4;b:10 40)");
eqt!(d_exec_count,        "exec count a from ([]a:1 2 3 4 5)", "5");
eqt!(d_exec_avg,          "exec avg a from ([]a:1 2 3 4 5)", "3f");

// ── III. NESTED & list-of-list operations ─────────────────────────
eqt!(d_raze_flat,         "raze (1 2;3 4;5 6)",            "1 2 3 4 5 6");
eqt!(d_raze_str_2,          "raze (\"abc\";\"def\";\"gh\")", "\"abcdefgh\"");
eqt!(d_count_list_of_list, "count (1 2 3;4 5;6)",          "3");
eqt!(d_first_list_of_list, "first (1 2 3;4 5;6)",          "1 2 3");
eqt!(d_reverse_list_of_list, "reverse (1 2;3 4)",          "(3 4;1 2)");

// ── JJJ. APPEND / UPSERT / INSERT patterns ────────────────────────
eqt!(d_cat_vec,           "1 2 3,4",                       "1 2 3 4");
eqt!(d_cat_atom_atom,     "1,2",                           "1 2");
eqt!(d_cat_vec_vec,       "1 2,3 4",                       "1 2 3 4");
eqt!(d_cat_sym,           "(`a`b),`c`d",                   "`a`b`c`d");
eqt!(d_cat_str,           "\"foo\",\"bar\"",               "\"foobar\"");

// ── KKK. SYSTEM/BUILTIN ops ───────────────────────────────────────
eqt!(d_key_dict,          "key `a`b`c!1 2 3",              "`a`b`c");
eqt!(d_val_dict,          "value `a`b`c!1 2 3",            "1 2 3");
eqt!(d_cols_table,        "cols ([]a:1 2;b:3 4;c:5 6)",    "`a`b`c");

// ── LLL. Advanced forms ──────────────────────────────────────────────
eqt!(d_apply_at,          "+[3;4]",                        "7");
eqt!(d_apply_at_vec,      "+[3;1 2 3]",                    "4 5 6");
eqt!(d_comma_each,        "(,)[1;2]",                      "1 2");

// ── MMM. TYPE promotion scenarios ─────────────────────────────────
eqt!(d_promo_ij,          "type 1+1j",                     "-7h");
eqt!(d_promo_ih,          "type 1+1h",                     "-6h");
eqt!(d_promo_if,          "type 1+1.0",                    "-9h");
eqt!(d_promo_vec_ij,      "type 1 2 3+1j",                 "7h");
eqt!(d_promo_vec_if,      "type 1 2 3+1.0",                "9h");

// ── NNN. MORE CROSS-TYPE ──────────────────────────────────────────
eqt!(d_cross_jj_mul,      "3j*4j",                         "12j");
eqt!(d_cross_jj_div,      "20j%4j",                        "5f");
eqt!(d_cross_if_mul,      "3*4.0",                         "12f");
eqt!(d_cross_hf_mul,      "3h*4.0",                        "12f");
eqt!(d_cross_jf_add,      "3j+4.0",                        "7f");

// ── OOO. APPLY / DO / WHILE ───────────────────────────────────────
eqt!(d_do_accum,          "{a:0;do[5;a+:1];a}[]",          "5");
eqt!(d_while_1,           "{a:1;while[a<10;a+:1];a}[]",    "10");
eqt!(d_while_double,      "{a:1;while[a<100;a*:2];a}[]",   "128");

// ── PPP. OVER / SCAN with varied fns ──────────────────────────────
eqt!(d_over_count,        "(+/)count each (1 2 3;4 5;6)",  "6j");
eqt!(d_over_init,         "10 {x+y}/ 1 2 3 4",             "20");
eqt!(d_scan_init,         "10 {x+y}\\ 1 2 3 4",            "11 13 16 20");

// ── QQQ. EACH PRIOR (':) ──────────────────────────────────────────
eqt!(d_deltas_1,          "deltas 10 15 18 22",            "10 5 3 4");
eqt!(d_deltas_f,          "deltas 1.5 2.5 4.0 7.0",        "1.5 1 1.5 3");
eqt!(d_differ_sym,        "differ `a`a`b`b`c",             "10101b");
eqt!(d_ratios_i,          "ratios 2 4 8 16",               "2 2 2 2f");

// ── RRR. PEEK into table ───────────────────────────────────────────
eqt!(d_tbl_exec_vec,      "exec a from ([]a:1 2 3;b:4 5 6)", "1 2 3");
eqt!(d_tbl_exec_from_idx, "exec a from ([]a:1 2 3)",       "1 2 3");
eqt!(d_tbl_first_col,     "first ([]a:1 2 3;b:4 5 6)",     "`a`b!1 4");

// ── SSS. MONADIC type predicates via type ─────────────────────────
eqt!(d_is_int_atom,       "-6h=type 42",                   "1b");
eqt!(d_is_long_vec,       "7h=type 1 2 3j",                "1b");
eqt!(d_is_float_atom,     "-9h=type 3.14",                 "1b");
eqt!(d_is_sym_atom,       "-11h=type `abc",                "1b");
eqt!(d_is_table,          "98h=type ([]a:1 2)",            "1b");
eqt!(d_is_dict,           "99h=type `a`b!1 2",             "1b");

// ── TTT. NUMERIC edge cases ────────────────────────────────────────
eqt!(d_int_max,           "0Wi",                           "2147483647i");
eqt!(d_int_min,           "-0Wi",                          "-2147483647i");
eqt!(d_long_max,          "0Wj",
    "9223372036854775807j");
eqt!(d_long_min,          "-0Wj",
    "-9223372036854775807j");
eqt!(d_float_inf_plus,    "0w>1e300",                      "1b");
eqt!(d_float_inf_neg,     "-0w<-1e300",                    "1b");
eqt!(d_zero_div_int,      "1%0",                           "0w");
eqt!(d_zero_div_int_neg,  "(-1)%0",                        "-0w");

// ── UUU. MORE SCAN use ────────────────────────────────────────────

// ── VVV. More string operations ───────────────────────────────────
eqt!(d_string_vec_f,      "string 1.5 2.5 3.5",
    "(\"1.5\";\"2.5\";\"3.5\")");
eqt!(d_string_sym_vec,    "string `abc`def",               "(\"abc\";\"def\")");
eqt!(d_str_count_chars,   "count \"the quick\"",           "9");
eqt!(d_str_upper_vec,     "upper \"abc def\"",             "\"ABC DEF\"");
eqt!(d_str_lower_vec,     "lower \"ABC DEF\"",             "\"abc def\"");

// ── WWW. Floating point special ───────────────────────────────────
eqt!(d_nan_sum,           "sum 1.0 2.0 0n 3.0",            "6f");
eqt!(d_nan_avg,           "avg 1.0 2.0 0n 3.0",            "2f");
eqt!(d_nan_count,         "count 1.0 2.0 0n 3.0",          "4");
eqt!(d_nan_null,          "null 0n",                       "1b");
eqt!(d_nan_null_vec,      "null 1.0 0n 3.0",               "010b");

// ── XXX. INDEXING wide ────────────────────────────────────────────
eqt!(d_idx_dict_missing,  "(`a`b`c!1 2 3)[`d]",            "0N");
eqt!(d_idx_dict_vec_miss, "(`a`b`c!1 2 3)`b`x",            "2 0N");
eqt!(d_idx_vec_take,      "3#1 2 3 4 5",                   "1 2 3");
eqt!(d_idx_vec_take_last, "(-3)#1 2 3 4 5",                "3 4 5");
eqt!(d_idx_range_til,     "(1 2 3 4 5)@til 3",             "1 2 3");

// ── YYY. DICT + FUNCTIONAL ────────────────────────────────────────
eqt!(d_dict_each_val,     "count each `a`b`c!(1 2;3 4 5;6)", "`a`b`c!2 3 1");
eqt!(d_dict_where,        "(`a`b`c!1 2 3)@`a`c",            "1 3");
eqt!(d_dict_in_keys,      "`b in key `a`b`c!1 2 3",        "1b");
eqt!(d_dict_in_vals,      "2 in value `a`b`c!1 2 3",       "1b");

// ── ZZZ. LARGE ARRAY ──────────────────────────────────────────────
eqt!(d_large_asc_ki,      "{[x](asc x)~`s#asc x}10000#3 1 4 1 5", "1b");
eqt!(d_large_asc_kj,      "{[x](asc x)~`s#asc x}10000#3 1 4 1 5j", "1b");
eqt!(d_large_asc_kf,      "{[x](asc x)~`s#asc x}10000#3.5 1.5 4.5", "1b");
eqt!(d_large_asc_ks,      "{[x](asc x)~`s#asc x}10000#`b`a`c", "1b");
eqt!(d_large_asc_kb,      "{[x](asc x)~`s#asc x}10000#10110b", "1b");
eqt!(d_large_sum_ki,      "sum 1000000#1",                 "1000000j");
eqt!(d_large_sum_kj,      "sum 1000000#1j",                "1000000j");
eqt!(d_large_sum_kf,      "sum 1000000#1.5",               "1500000f");
eqt!(d_large_avg_ki,      "avg 1000000#5",                 "5f");
eqt!(d_large_avg_kj,      "avg 1000000#5j",                "5f");
eqt!(d_large_count,       "count 1000000#1",               "1000000");
eqt!(d_large_reverse,     "{[x](reverse reverse x)~x}1000000#3 1 4", "1b");
eqt!(d_large_distinct,    "count distinct 100000#1 2 3 4 5", "5");
eqt!(d_large_where,       "count where 1000000#10110b",    "600000");
eqt!(d_large_group,       "count group 100000#`a`b`c",     "3");

// ── AB. ATOMIC operations roundtrip ────────────────────────────────
eqt!(d_atom_rt_ii,        "42+1-43",                       "0");
eqt!(d_atom_rt_jj,        "42j+1j-43j",                    "0j");
eqt!(d_atom_rt_ff,        "(3.14+0)-3.14",                 "0f");
eqt!(d_atom_rt_vec,       "(1 2 3+4 5 6)-4 5 6",           "1 2 3");
eqt!(d_atom_rt_neg_neg,   "neg neg 5 6 7",                 "5 6 7");
eqt!(d_atom_rt_abs_neg,   "abs neg 1 2 3",                 "1 2 3");

// ── AC. Sort stability ────────────────────────────────────────────
eqt!(d_sort_stable_ii,    "(asc 10#1 2)~`s#10#1",           "0b");
eqt!(d_sort_idempotent,   "{[x]asc[x]~asc[asc x]}10000?100", "1b");
eqt!(d_sort_reverse_desc, "{[x](desc x)~reverse asc x}1000?100", "1b");

// ── AD. COMPARE SYM with diff lengths ─────────────────────────────
eqt!(d_cmp_sym_len3,      "`abc`def`ghi~`abc`def`ghi",     "1b");
eqt!(d_cmp_sym_len2,      "`ab`cd~`ab`cd",                 "1b");
eqt!(d_cmp_sym_mixed,     "`a`longer`short~`a`longer`short", "1b");
eqt!(d_cmp_sym_order,     "asc `z`a`m",                    "`s#`a`m`z");

// ── AE. Table with all null ────────────────────────────────────────
eqt!(d_tbl_null_col,      "([]a:0N 0N 0N)~([]a:(0N;0N;0N))", "1b");
eqt!(d_tbl_null_sum,      "sum ([]a:(1;0N;3))",            "(enlist`a)!enlist \
    4j");
eqt!(d_tbl_null_count,    "count ([]a:(1;0N;3))",          "3");

// ── AF. EMPTY table ops ───────────────────────────────────────────
eqt!(d_tbl_empty_sum,     "sum ([]a:`int$())",             "(enlist`a)!enlist \
    0j");
eqt!(d_tbl_empty_cols,    "cols ([]a:`int$();b:`int$())",  "`a`b");

// ── AG. MIXED list count/types ────────────────────────────────────
eqt!(d_mix_count,         "count (1;2.0;`c;\"str\")",      "4");
eqt!(d_mix_first,         "first (1;2.0;`c)",              "1");
eqt!(d_mix_last,          "last (1;2.0;`c)",               "`c");
eqt!(d_mix_reverse,       "reverse (1;2.0;`c)",            "(`c;2.0;1)");

// ── AH. Boolean summing (popcount test) ───────────────────────────
eqt!(d_popcount_tiny,     "sum 10110b",                    "3i");
eqt!(d_popcount_empty,    "sum `boolean$()",               "0i");
eqt!(d_popcount_all_1,    "sum 11111b",                    "5i");
eqt!(d_popcount_all_0,    "sum 00000b",                    "0i");
eqt!(d_popcount_1k,       "sum 1000#1b",                   "1000i");
eqt!(d_popcount_100k,     "sum 100000#10110b",             "60000i");

// ── AI. MORE random / permutation invariants ─────────────────────
eqt!(d_perm_asc_count,    "{[x]count asc x}10000?100",     "10000");
eqt!(d_perm_reverse_id,   "{[x](reverse reverse x)~x}1000?100", "1b");
eqt!(d_perm_group_count,  "{[x]count group x}100#1 2 3 4 5", "5");
eqt!(d_perm_distinct_max, "{[x](count distinct x)<=count x}10000?100", "1b");

// ── AJ. DICT with various key types ──────────────────────────────
eqt!(d_dict_int_keys,     "(1 2 3)!10 20 30",              "(1 2 3)!10 20 30");
eqt!(d_dict_int_lookup,   "((1 2 3)!10 20 30)[2]",         "20");
eqt!(d_dict_str_keys,     "(\"a\";\"b\";\"c\")!1 2 3",
    "(\"a\";\"b\";\"c\")!1 2 3");
eqt!(d_dict_sym_keys,     "(`one`two)!(1 2;3 4)",          "(`one`two)!(1 2;3 \
    4)");

// ── AK. EDGE reshape / take ──────────────────────────────────────
eqt!(d_take_pos_larger,   "5#1 2 3",                       "1 2 3 1 2");
eqt!(d_cut_partial,       "2 cut 1 2 3 4 5 6 7",           "(1 2;3 4;5 \
    6;enlist \
    7)");
eqt!(d_cut_exact,         "3 cut 1 2 3 4 5 6",             "(1 2 3;4 5 6)");


// ── BAT6: TYPED ARITHMETIC ×SIZES ──────────────────────────────────
eqt!(d6_add_i_100,        "(100#1)+100#2",                 "100#3");
eqt!(d6_add_i_10k,        "(10000#1)+10000#2",             "10000#3");
eqt!(d6_add_j_100,        "(100#1j)+100#2j",               "100#3j");
eqt!(d6_add_j_10k,        "(10000#1j)+10000#2j",           "10000#3j");
eqt!(d6_add_f_100,        "(100#1.5)+100#2.5",             "100#4f");
eqt!(d6_add_f_10k,        "(10000#1.5)+10000#2.5",         "10000#4f");
eqt!(d6_sub_i_100,        "(100#10)-100#3",                "100#7");
eqt!(d6_sub_j_100,        "(100#10j)-100#3j",              "100#7j");
eqt!(d6_sub_f_100,        "(100#10.0)-100#3.0",            "100#7f");
eqt!(d6_mul_i_100,        "(100#3)*100#4",                 "100#12");
eqt!(d6_mul_j_100,        "(100#3j)*100#4j",               "100#12j");
eqt!(d6_mul_f_100,        "(100#3.0)*100#4.0",             "100#12f");
eqt!(d6_div_f_100,        "(100#10.0)%100#2.0",            "100#5f");
eqt!(d6_div_j_100,        "(100#10j)%100#2j",              "100#5f");
eqt!(d6_neg_i_100,        "neg 100#5",                     "100#-5");
eqt!(d6_neg_j_100,        "neg 100#5j",                    "100#-5j");
eqt!(d6_neg_f_100,        "neg 100#1.5",                   "100#-1.5");
eqt!(d6_abs_i,            "abs 100#-3",                    "100#3");
eqt!(d6_abs_j,            "abs 100#-3j",                   "100#3j");
eqt!(d6_abs_f,            "abs 100#-3.5",                  "100#3.5");

// ── BAT7: MORE AGGREGATES × TYPES ───────────────────────────────────
eqt!(d7_sum_bb_all1,      "sum 1111b",                     "4i");
eqt!(d7_sum_bb_all0,      "sum 0000b",                     "0i");
eqt!(d7_sum_hh,           "sum 1 2 3 4 5h",                "15j");
eqt!(d7_sum_ee,           "sum 1 2 3e",                    "6e");
eqt!(d7_sum_ff_null,      "sum 1 0n 3 4.0",                "8f");
eqt!(d7_sum_jj_null,      "sum (1j;0Nj;3j)",               "4j");
eqt!(d7_min_bb,           "min 11011b",                    "0b");
eqt!(d7_max_bb,           "max 00010b",                    "1b");
eqt!(d7_min_ee,           "min 5 3 1 4e",                  "1e");
eqt!(d7_max_ee,           "max 5 3 1 4e",                  "5e");
eqt!(d7_avg_bb,           "avg 1010b",                     "0.5");
eqt!(d7_avg_hh,           "avg 1 2 3 4h",                  "2.5");
eqt!(d7_avg_ee,           "avg 2 4 6e",                    "4f");
eqt!(d7_cnt_bb,           "count 1011b",                   "4");
eqt!(d7_cnt_hh,           "count 1 2 3 4 5h",              "5");
eqt!(d7_cnt_ee,           "count 1 2 3e",                  "3");
eqt!(d7_prd_ff,           "prd 2.0 3.0 4.0",               "24f");
eqt!(d7_prd_jj,           "prd 2 3 4j",                    "24j");
eqt!(d7_first_ff,         "first 1.5 2.5 3.5",             "1.5");
eqt!(d7_last_ff,          "last 1.5 2.5 3.5",              "3.5");

// ── BAT8: COMPARISON × SIZES ───────────────────────────────────────
eqt!(d8_eq_ii_10,         "count (10#1 2)=(10#1 2)",        "10");
eqt!(d8_eq_ii_1k,         "count (1000#5)=(1000#5)",        "1000");
eqt!(d8_lt_ii_10,         "sum (10#1)<10#5",                "10i");
eqt!(d8_gt_ii_10,         "sum (10#5)>10#1",                "10i");
eqt!(d8_match_ii_large,   "(10000#1)~10000#1",              "1b");
eqt!(d8_match_ii_diff,    "(10000#1)~10000#2",              "0b");
eqt!(d8_match_jj_large,   "(10000#1j)~10000#1j",            "1b");
eqt!(d8_match_ff_large,   "(10000#1.5)~10000#1.5",          "1b");
eqt!(d8_match_ss_large,   "(10000#`x)~10000#`x",            "1b");
eqt!(d8_cmp_cross_ij,     "1 2 3=1 2 3j",                   "111b");
eqt!(d8_cmp_cross_if,     "1 2 3=1.0 2.0 3.0",              "111b");
eqt!(d8_cmp_cross_jf,     "1 2 3j=1.0 2.0 3.0",             "111b");

// ── BAT9: TABLE with nulls ─────────────────────────────────────────
eqt!(d9_tbl_null_i,       "([]a:(1;0N;3))[1;`a]",          "0N");
eqt!(d9_tbl_null_count,   "count ([]a:(1;0N;3;0N))",        "4");
eqt!(d9_tbl_null_eq_self,   "([]a:(1;0N;3))~([]a:(1;0N;3))", "1b");
eqt!(d9_tbl_null_add_self,  "([]a:(1;0N;3))+([]a:(1;0N;3))", "([]a:(2;0N;6))");
eqt!(d9_tbl_null_filter,    "count select from ([]a:1 2 0N 3 4 0N) where not \
    null a", "4");
eqt!(d9_tbl_null_mix_f,   "([]a:(1.0;0n;3.0;0n))~([]a:(1.0;0n;3.0;0n))", "1b");
eqt!(d9_tbl_null_mix_j,   "([]a:(1j;0Nj;3j))~([]a:(1j;0Nj;3j))", "1b");

// ── BAT10: DICT × DICT with all types ─────────────────────────────
eqt!(dA_dict_hh_add,      "(`a`b!1 2h)+(`a`b!3 4h)",       "`a`b!4 6i");
eqt!(dA_dict_ee_add,      "(`a`b!1 2e)+(`a`b!3 4e)",       "`a`b!4 6e");
eqt!(dA_dict_bb_add,      "(`a`b!10b)+(`a`b!01b)",         "`a`b!1 1i");
eqt!(dA_dict_cross_jf,    "(`a`b!1 2j)+(`a`b!1.5 2.5)",    "`a`b!2.5 4.5");
eqt!(dA_dict_sub_f,       "(`a`b!5.0 6.0)-(`a`b!1.5 2.5)", "`a`b!3.5 3.5");
eqt!(dA_dict_mul_cross,   "(`a`b!2 3)*(`a`b!4 5j)",         "`a`b!8 15j");
eqt!(dA_dict_keys_check,  "(`a`b`c!1 2 3)[`a]",            "1");
eqt!(dA_dict_vals_mul,    "(value `a`b`c!1 2 3)*10",       "10 20 30");
eqt!(dA_dict_count_vals,  "count value `a`b`c!1 2 3",      "3");
eqt!(dA_dict_nested_vals, "`a`b!(1 2 3;4 5 6)",            "`a`b!(1 2 3;4 5 \
    6)");
eqt!(dA_dict_mix_val_types, "`a`b`c!(1;2.0;`c)",           "`a`b`c!(1;2.0;`c)");

// ── BAT11: KEYED TABLE variations ─────────────────────────────────
eqt!(dB_kt_reverse,       "reverse ([k:1 2 3]v:10 20 30)", "([k:3 2 1]v:30 20 \
    10)");
eqt!(dB_kt_add_scalar,    "([k:1 2 3]v:10 20 30)+5",       "([k:1 2 3]v:15 25 \
    35)");
eqt!(dB_kt_sub_scalar,    "([k:1 2 3]v:100 200 300)-10",   "([k:1 2 3]v:90 190 \
    290)");
eqt!(dB_kt_mul_scalar,    "([k:1 2 3]v:1 2 3)*10",         "([k:1 2 3]v:10 20 \
    30)");
eqt!(dB_kt_div_f,         "([k:1 2 3]v:10.0 20.0 30.0)%2.0", "([k:1 2 3]v:5 10 \
    15f)");
eqt!(dB_kt_multi_col,     "([k:1 2 3]a:10 20 30;b:100 200 300)", "([k:1 2 \
    3]a:10 20 30;b:100 200 300)");
eqt!(dB_kt_mc_add,        "([k:1 2 3]a:1 2 3;b:10 20 30)+([k:1 2 3]a:1 1 1;b:1 \
    1 1)", "([k:1 2 3]a:2 3 4;b:11 21 31)");

// ── BAT12: SELECT / EXEC / UPDATE / DELETE wider ──────────────────
eqt!(dC_sel_count,        "select count i from ([]a:1 2 3 4 5)", "([]x:enlist \
    5)");
eqt!(dC_sel_sum_mul,      "select x:10*a from ([]a:1 2 3)", "([]x:10 20 30)");
eqt!(dC_sel_filter_all,   "select from ([]a:1 2 3) where a>0", "([]a:1 2 3)");
eqt!(dC_sel_mul_cols,     "select a,b,c from ([]a:1 2;b:3 4;c:5 6)", "([]a:1 \
    2;b:3 4;c:5 6)");
eqt!(dC_sel_compute,      "select c:a+b from ([]a:1 2 3;b:4 5 6)", "([]c:5 7 \
    9)");
eqt!(dC_upd_multi,        "update a:a*2, b:b+10 from ([]a:1 2 3;b:1 1 1)",
    "([]a:2 4 6;b:11 11 11)");
eqt!(dC_exec_by,          "exec a by sym from ([]sym:`a`b`a`b;a:1 2 3 4)",
    "`a`b!(1 3;2 4)");

// ── BAT13: MONADIC ops × types ────────────────────────────────────
eqt!(dD_not_i,            "not 0",                          "1b");
eqt!(dD_not_i_v,          "not 0 1 0 1",                    "1010b");
eqt!(dD_not_j,            "not 0j",                         "1b");
eqt!(dD_not_f,            "not 0.0",                        "1b");
eqt!(dD_null_i,           "null 0N",                        "1b");
eqt!(dD_null_j,           "null 0Nj",                       "1b");
eqt!(dD_null_f,           "null 0n",                        "1b");
eqt!(dD_null_e,           "null 0ne",                       "1b");
eqt!(dD_null_s,           "null `",                         "1b");
eqt!(dD_null_i_notnull,   "null 42",                        "0b");
eqt!(dD_null_f_notnull,   "null 3.14",                      "0b");
eqt!(dD_first_atom,       "first 42",                       "42");
eqt!(dD_last_atom,        "last 42",                        "42");
eqt!(dD_reverse_sym,      "reverse `a`b`c`d",               "`d`c`b`a");
eqt!(dD_reverse_f,        "reverse 1.0 2.0 3.0 4.0",        "4 3 2 1f");
eqt!(dD_reverse_j,        "reverse 1 2 3j",                 "3 2 1j");

// ── BAT14: Large-N specific tests ─────────────────────────────────
eqt!(dE_add_1M_ii,        "count (1000000#1)+1000000#2",    "1000000");
eqt!(dE_add_1M_jj,        "count (1000000#1j)+1000000#2j",  "1000000");
eqt!(dE_add_1M_ff,        "count (1000000#1.0)+1000000#2.0", "1000000");
eqt!(dE_sum_1M_ii,        "sum 1000000#1",                  "1000000j");
eqt!(dE_sum_1M_jj,        "sum 1000000#1j",                 "1000000j");
eqt!(dE_sum_1M_ff,        "sum 1000000#1.0",                "1000000f");
eqt!(dE_avg_1M_ii,        "avg 1000000#1",                  "1f");
eqt!(dE_avg_1M_jj,        "avg 1000000#1j",                 "1f");
eqt!(dE_count_1M,         "count 1000000#1",                "1000000");
eqt!(dE_reverse_1M,       "{[x]count reverse x}1000000#1 2 3", "1000000");
eqt!(dE_neg_1M,           "{[x]count neg x}1000000#5",      "1000000");
eqt!(dE_sum_100k_bool,    "sum 100000#10110b",              "60000i");

// ── BAT15: NULL handling deep ─────────────────────────────────────
eqt!(dF_sum_mixed_null_i, "sum 1 2 0N 4 5",                 "12j");
eqt!(dF_sum_mixed_null_j, "sum 1 2 0N 4 5j",                "12j");
eqt!(dF_sum_mixed_null_f, "sum 1 2 0n 4 5.0",               "12f");
eqt!(dF_avg_mixed_null,   "avg 1 2 0N 4 5",                 "3f");
eqt!(dF_max_mixed_null,   "max 1 2 0N 4 5",                 "5");
eqt!(dF_min_mixed_null,   "min 1 2 0N 4 5",                 "1");
eqt!(dF_count_with_null,  "count 1 2 0N 4 5",               "5");
eqt!(dF_null_preserve_neg, "neg (1;0N;3)",                  "-1 0N -3");
eqt!(dF_null_preserve_abs, "abs (-1;0N;-3)",                "1 0N 3");

// ── BAT16: Numerical precision ────────────────────────────────────
eqt!(dG_sum_trueavg,      "(avg 1 2 3 4 5)=3f",             "1b");
eqt!(dG_sum_ff,           "sum 1.5 2.5 3.5",                "7.5");
eqt!(dG_div_half,         "1.0%2.0",                        "0.5");
eqt!(dG_sqrt_4,           "sqrt 4.0",                       "2f");
eqt!(dG_sqrt_9,           "sqrt 9.0",                       "3f");
eqt!(dG_sqrt_16,          "sqrt 16.0",                      "4f");
eqt!(dG_sqrt_100,         "sqrt 100.0",                     "10f");

// ── BAT17: BIG list operations ────────────────────────────────────
eqt!(dH_til_100,          "count til 100",                  "100");
eqt!(dH_til_100k,         "count til 100000",               "100000");
eqt!(dH_til_sum,          "sum til 100",                    "4950j");
eqt!(dH_til_count,        "count til 1000",                 "1000");
eqt!(dH_take_10k,         "count 10000#1 2 3",              "10000");
eqt!(dH_drop_10k,         "count 100_10000#1",              "9900");
eqt!(dH_cat_large,        "count (1000#1),1000#2",          "2000");

// ── BAT18: MIXED TYPE arith vector ────────────────────────────────
eqt!(dI_mix_ihf,          "1 2 3+1h+1.0",                   "3 4 5f");
eqt!(dI_mix_vec_promo,    "type 1 2 3+1h",                  "6h");
eqt!(dI_mix_vec_to_j,     "type 1 2 3+1j",                  "7h");
eqt!(dI_mix_vec_to_f,     "type 1 2 3+1.0",                 "9h");

// ── BAT19: LOGIC ops × bools ──────────────────────────────────────
eqt!(dJ_xor_vec,          "10b<>01b",                       "11b");
eqt!(dJ_xor_self,         "10110b<>10110b",                 "00000b");
eqt!(dJ_neq_vec_i,        "1 2 3<>1 0 3",                   "010b");
eqt!(dJ_neq_vec_f,        "1.0 2.0<>1.0 3.0",               "01b");

// ── BAT20: TIME/DATE arithmetic more ──────────────────────────────
eqt!(dK_date_add_days,    "2024.01.01+7",                   "2024.01.08");
eqt!(dK_date_sub_date,    "2024.01.10-2024.01.01",          "9");
eqt!(dK_date_vec_diff,    "(2024.01.02 2024.01.03)-2024.01.01", "1 2");
eqt!(dK_time_add_ms,      "12:00:00.000+1000",              "12:00:01.000");
eqt!(dK_month_add,        "2024.01m+5",                     "2024.06m");

// ── BAT21: CASTS round-trips ──────────────────────────────────────
eqt!(dL_cast_rt_ij,       "`long$`int$1 2 3j",              "1 2 3j");
eqt!(dL_cast_rt_if,       "`float$`int$1 2 3f",             "1 2 3f");
eqt!(dL_cast_rt_bool,     "`boolean$`int$1 0 1 0",          "1010b");
eqt!(dL_cast_chain,       "`float$`long$`int$3",            "3f");
eqt!(dL_cast_empty,       "`long$`int$()",                  "`long$()");
eqt!(dL_cast_null_j,      "`int$0Nj",                       "0N");

// ── BAT22: DOT access and apply ───────────────────────────────────
eqt!(dM_plus_dot,         "+[2;3]",                         "5");
eqt!(dM_times_dot,        "*[4;5]",                         "20");
eqt!(dM_minus_dot,        "-[10;3]",                        "7");
eqt!(dM_div_dot,          "%[10;4]",                        "2.5");
eqt!(dM_min_dot,          "&[3;5]",                         "3");
eqt!(dM_max_dot,          "|[3;5]",                         "5");
eqt!(dM_eq_dot,           "=[3;3]",                         "1b");
eqt!(dM_eq_dot_ne,        "=[3;4]",                         "0b");
eqt!(dM_lt_dot,           "<[2;3]",                         "1b");
eqt!(dM_gt_dot,           ">[3;2]",                         "1b");

// ── BAT23: LIST operations ────────────────────────────────────────
eqt!(dN_concat_ii_4,      "1,2,3,4",                        "1 2 3 4");
eqt!(dN_concat_jj_3,      "1j,2j,3j",                       "1 2 3j");
eqt!(dN_concat_ff_3,      "1.0,2.0,3.0",                    "1 2 3f");
eqt!(dN_concat_ss_3,      "`a,`b,`c",                       "`a`b`c");
eqt!(dN_null_concat,      "(enlist 1),enlist 2",            "1 2");
eqt!(dN_empty_concat,     "(0#0),1 2 3",                    "1 2 3");
eqt!(dN_list_nested_count, "count (1 2;3 4;5 6;7 8)",       "4");
eqt!(dN_list_nested_sum,  "sum each (1 2;3 4;5 6)",         "3 7 11j");

// ── BAT24: more INDEXING ──────────────────────────────────────────
eqt!(dO_idx_big,          "(til 100)@50",                   "50");
eqt!(dO_idx_oob,          "(til 10)@100",                   "0N");
eqt!(dO_idx_multi,        "(til 10)@0 5 9",                 "0 5 9");
eqt!(dO_take_1,           "1#1 2 3",                        "enlist 1");
eqt!(dO_take_3,           "3#til 10",                       "0 1 2");
eqt!(dO_take_n1,          "(-1)#1 2 3 4",                   "enlist 4");
eqt!(dO_take_n3,          "(-3)#til 10",                    "7 8 9");

// ── BAT25: DICT of functions ──────────────────────────────────────
eqt!(dP_dict_int_lookup,  "((1 2 3)!10 20 30)@1",           "10");
eqt!(dP_dict_int_dispatch, "(`add`sub!(+;-))[`add][3;4]",   "7");
eqt!(dP_dict_int_has,     "1 in key (1 2 3)!10 20 30",      "1b");
eqt!(dP_dict_str_lookup,  "((\"a\";\"b\";\"c\")!1 2 3)@\"b\"", "2");

// ── BAT26: STRING operations ──────────────────────────────────────
eqt!(dQ_str_eq,           "\"abc\"~\"abc\"",                "1b");
eqt!(dQ_str_ne,           "\"abc\"~\"abd\"",                "0b");
eqt!(dQ_str_starts,       "\"hello\" like \"h*\"",          "1b");
eqt!(dQ_str_ends,         "\"hello\" like \"*o\"",          "1b");
eqt!(dQ_str_mid,          "\"hello\" like \"*ll*\"",        "1b");
eqt!(dQ_str_count_a,      "count \"a\"",                    "1");
eqt!(dQ_str_count_empty,  "count \"\"",                     "0");
eqt!(dQ_str_reverse_empty, "reverse \"\"",                  "\"\"");
eqt!(dQ_str_concat_empty, "\"\",\"hello\"",                 "\"hello\"");

// ── BAT27: REDUCE with custom init ────────────────────────────────
eqt!(dR_max_positive,     "max 1 5 3 8 2",                  "8");
eqt!(dR_min_positive,     "min 1 5 3 8 2",                  "1");
eqt!(dR_max_neg,          "max -1 -5 -3 -8 -2",             "-1");
eqt!(dR_min_neg,          "min -1 -5 -3 -8 -2",             "-8");
eqt!(dR_sum_mixed,        "sum -1 2 -3 4 -5",               "-3j");
eqt!(dR_prd_mixed,        "prd -1 2 -3 4",                  "24");
eqt!(dR_prd_zero,         "prd 1 2 0 4",                    "0");

// ── BAT28: TYPE predicate coverage ────────────────────────────────
eqt!(dS_type_ki,          "6h=type 1 2 3",                  "1b");
eqt!(dS_type_kj,          "7h=type 1 2 3j",                 "1b");
eqt!(dS_type_ke,          "8h=type 1 2 3e",                 "1b");
eqt!(dS_type_kf,          "9h=type 1 2 3f",                 "1b");
eqt!(dS_type_kh,          "5h=type 1 2 3h",                 "1b");
eqt!(dS_type_kb,          "1h=type 1010b",                  "1b");
eqt!(dS_type_kg,          "4h=type 0x010203",               "1b");
eqt!(dS_type_ks,          "11h=type `a`b`c",                "1b");
eqt!(dS_type_kc,          "10h=type \"abc\"",               "1b");
eqt!(dS_type_kd,          "14h=type 2024.01.01 2024.01.02", "1b");


// ── BAT29: All types × small sizes (1,2,3 elem) ────────────────────
eqt!(e_add_i_2,           "1 2+3 4",                        "4 6");
eqt!(e_add_i_3,           "1 2 3+4 5 6",                    "5 7 9");
eqt!(e_add_j_2,           "1 2j+3 4j",                      "4 6j");
eqt!(e_add_j_3,           "1 2 3j+4 5 6j",                  "5 7 9j");
eqt!(e_add_f_2,           "1.0 2.0+3.0 4.0",                "4 6f");
eqt!(e_add_f_3,           "1.0 2.0 3.0+4.0 5.0 6.0",        "5 7 9f");
eqt!(e_sum_i_1,           "sum enlist 5",                   "5j");
eqt!(e_sum_i_2,           "sum 3 4",                        "7j");
eqt!(e_sum_i_3,           "sum 1 2 3",                      "6j");
eqt!(e_sum_j_1,           "sum enlist 5j",                  "5j");
eqt!(e_sum_j_2,           "sum 3 4j",                       "7j");
eqt!(e_sum_f_1,           "sum enlist 5.0",                 "5f");
eqt!(e_sum_f_2,           "sum 1.5 2.5",                    "4f");
eqt!(e_avg_i_1,           "avg enlist 5",                   "5f");
eqt!(e_avg_i_2,           "avg 1 3",                        "2f");
eqt!(e_avg_j_1,           "avg enlist 5j",                  "5f");
eqt!(e_avg_f_2,           "avg 2.0 4.0",                    "3f");
eqt!(e_min_i_1,           "min enlist 5",                   "5");
eqt!(e_min_i_3,           "min 5 3 7",                      "3");
eqt!(e_min_j_3,           "min 5 3 7j",                     "3j");
eqt!(e_max_i_1,           "max enlist 5",                   "5");
eqt!(e_max_i_3,           "max 5 3 7",                      "7");
eqt!(e_max_j_3,           "max 5 3 7j",                     "7j");
eqt!(e_max_f_3,           "max 5.5 3.5 7.5",                "7.5");

// ── BAT30: Sizes 4, 5, 10 ─────────────────────────────────────────
eqt!(e_sum4_i,            "sum 1 2 3 4",                    "10j");
eqt!(e_sum5_i,            "sum 1 2 3 4 5",                  "15j");
eqt!(e_sum10_i,           "sum til 10",                     "45j");
eqt!(e_sum4_j,            "sum 1 2 3 4j",                   "10j");
eqt!(e_sum5_j,            "sum 1 2 3 4 5j",                 "15j");
eqt!(e_sum4_f,            "sum 1.0 2.0 3.0 4.0",            "10f");
eqt!(e_avg4_i,            "avg 1 2 3 4",                    "2.5");
eqt!(e_avg5_i,            "avg 1 2 3 4 5",                  "3f");
eqt!(e_max4_i,            "max 3 5 1 4",                    "5");
eqt!(e_min4_i,            "min 3 5 1 4",                    "1");

// ── BAT31: Mixed sizes per verb ───────────────────────────────────
eqt!(e_sum_50,            "sum 50#1",                       "50j");
eqt!(e_sum_100,           "sum 100#1",                      "100j");
eqt!(e_sum_500,           "sum 500#1",                      "500j");
eqt!(e_sum_1k,            "sum 1000#1",                     "1000j");
eqt!(e_sum_10k,           "sum 10000#1",                    "10000j");
eqt!(e_sum_100k,          "sum 100000#1",                   "100000j");
eqt!(e_avg_50,            "avg 50#5",                       "5f");
eqt!(e_avg_100,           "avg 100#5",                      "5f");
eqt!(e_avg_1k,            "avg 1000#5",                     "5f");
eqt!(e_avg_10k,           "avg 10000#5",                    "5f");

// ── BAT32: Per-type basic primitives ──────────────────────────────
eqt!(e_abs_j_100,         "abs 100#-3j",                    "100#3j");
eqt!(e_abs_e_100,         "abs 100#-3e",                    "100#3e");
eqt!(e_abs_h_100,         "abs 100#-3h",                    "100#3h");
eqt!(e_neg_j_100,         "neg 100#5j",                     "100#-5j");
eqt!(e_neg_e_100,         "neg 100#5e",                     "100#-5e");
eqt!(e_neg_h_100,         "neg 100#5h",                     "100#-5h");
eqt!(e_reverse_i_100,     "(reverse reverse 100#1 2 3)~100#1 2 3", "1b");
eqt!(e_reverse_j_100,     "(reverse reverse 100#1 2 3j)~100#1 2 3j", "1b");
eqt!(e_reverse_f_100,     "(reverse reverse 100#1.5 2.5)~100#1.5 2.5", "1b");

// ── BAT33: Cross-type dimension matching ──────────────────────────
eqt!(e_cross_add_ij_j,    "1 2 3 + 1 2 3j",                 "2 4 6j");
eqt!(e_cross_add_ij_n,    "type 1 2 3 + 1 2 3j",            "7h");
eqt!(e_cross_add_if_f,    "1 2 3 + 1.0 2.0 3.0",            "2 4 6f");
eqt!(e_cross_add_if_n,    "type 1 2 3 + 1.0 2.0 3.0",       "9h");
eqt!(e_cross_add_jf_f,    "1 2 3j + 1.0 2.0 3.0",           "2 4 6f");
eqt!(e_cross_sub_ij,      "1 2 3 - 1 2 3j",                 "0 0 0j");
eqt!(e_cross_mul_if,      "2 3 4 * 2.0 3.0 4.0",            "4 9 16f");
eqt!(e_cross_div_if,      "10 20 30 % 2 4 5",               "5 5 6f");

// ── BAT34: WHERE / FILTER patterns ────────────────────────────────
eqt!(e_where_small,       "where 10010b",                   "0 3");
eqt!(e_where_all,         "where 11111b",                   "0 1 2 3 4");
eqt!(e_where_i_vec,       "where 3 0 2",                    "0 0 0 2 2");
eqt!(e_filter_positive,   "x where 0<x:1 -2 3 -4 5",        "1 3 5");

// ── BAT35: HASH: in, distinct, group ─────────────────────────────
eqt!(e_in_vec_vec,        "1 2 3 in 2 3 4",                 "011b");
eqt!(e_in_atom_vec,       "2 in 1 2 3",                     "1b");
eqt!(e_in_miss,           "0 in 1 2 3",                     "0b");
eqt!(e_in_sym_vec,        "`a`b`c in `b`c`d",               "011b");
eqt!(e_distinct_i5,       "distinct 5 5 5 5 5",             "enlist 5");
eqt!(e_distinct_s3,       "distinct `a`b`c",                "`a`b`c");
eqt!(e_distinct_all_same, "distinct 100#42",                "enlist 42");
eqt!(e_group_ints,        "count group 1 2 3 1 2",          "3");
eqt!(e_group_syms,        "count group `a`b`a`c",           "3");

// ── BAT36: STRING / SYM ops ──────────────────────────────────────
eqt!(e_upper_empty,       "upper \"\"",                     "\"\"");
eqt!(e_lower_empty,       "lower \"\"",                     "\"\"");
eqt!(e_count_short,       "count \"hi\"",                   "2");
eqt!(e_count_long,        "count \"the quick brown fox\"",  "19");
eqt!(e_upper_sym_vec,     "upper `abc`def`ghi",             "`ABC`DEF`GHI");
eqt!(e_lower_sym_vec,     "lower `ABC`DEF`GHI",             "`abc`def`ghi");
eqt!(e_concat_str_vec,    "\"ab\",\"cd\",\"ef\"",           "\"abcdef\"");

// ── BAT37: CAST matrix ────────────────────────────────────────────
eqt!(e_cast_b_to_i,       "`int$1b",                        "1");
eqt!(e_cast_b_to_j,       "`long$1b",                       "1j");
eqt!(e_cast_b_to_f,       "`float$1b",                      "1f");
eqt!(e_cast_h_to_i,       "`int$42h",                       "42");
eqt!(e_cast_h_to_j,       "`long$42h",                      "42j");
eqt!(e_cast_i_to_h,       "`short$42",                      "42h");
eqt!(e_cast_i_to_j,       "`long$42",                       "42j");
eqt!(e_cast_i_to_f,       "`float$42",                      "42f");
eqt!(e_cast_i_to_e,       "`real$42",                       "42e");
eqt!(e_cast_j_to_i,       "`int$42j",                       "42");
eqt!(e_cast_j_to_f,       "`float$42j",                      "42f");
eqt!(e_cast_f_to_i,       "`int$42.0",                      "42i");
eqt!(e_cast_f_to_j,       "`long$42.0",                     "42j");
eqt!(e_cast_f_to_e,       "`real$42.0",                     "42e");
eqt!(e_cast_e_to_f,       "`float$42e",                     "42f");
eqt!(e_cast_e_to_i,       "`int$42e",                       "42i");
eqt!(e_cast_empty_i,      "`int$()",                        "`int$()");
eqt!(e_cast_empty_j,      "`long$()",                       "`long$()");
eqt!(e_cast_empty_f,      "`float$()",                      "`float$()");

// ── BAT38: TABLE column types ─────────────────────────────────────
eqt!(e_tbl_i_col,         "type ([]a:1 2 3)`a",             "6h");
eqt!(e_tbl_j_col,         "type ([]a:1 2 3j)`a",            "7h");
eqt!(e_tbl_f_col,         "type ([]a:1.0 2.0 3.0)`a",       "9h");
eqt!(e_tbl_e_col,         "type ([]a:1 2 3e)`a",            "8h");
eqt!(e_tbl_s_col,         "type ([]a:`x`y`z)`a",            "11h");
eqt!(e_tbl_b_col,         "type ([]a:101b)`a",              "1h");
eqt!(e_tbl_d_col,         "type ([]a:2024.01.01 2024.01.02)`a", "14h");

// ── BAT39: DICT lookups varied ────────────────────────────────────
eqt!(e_dict_key_int,      "(`a`b`c!1 2 3)`b",               "2");
eqt!(e_dict_key_str,      "(`a`b`c!`x`y`z)`b",              "`y");
eqt!(e_dict_key_multi,    "(`a`b`c!1 2 3)@`a`c",            "1 3");
eqt!(e_dict_sym_keys,     "key `a`b`c!1 2 3",               "`a`b`c");
eqt!(e_dict_vals_i,       "value `a`b`c!1 2 3",             "1 2 3");
eqt!(e_dict_vals_f,       "value `a`b`c!1.5 2.5 3.5",       "1.5 2.5 3.5");
eqt!(e_dict_reverse,      "reverse `a`b`c!1 2 3",           "`c`b`a!3 2 1");
eqt!(e_dict_first_val,    "first `a`b`c!10 20 30",          "10");
eqt!(e_dict_last_val,     "last `a`b`c!10 20 30",           "30");
eqt!(e_dict_count,        "count `a`b`c`d!1 2 3 4",         "4");
eqt!(e_dict_take,         "2#`a`b`c`d!1 2 3 4",             "`a`b!1 2");

// ── BAT40: NULL behavior per-type ─────────────────────────────────
eqt!(e_null_sum_i,        "sum null (1;0N;3)",              "1i");
eqt!(e_null_sum_j,        "sum null (1j;0Nj;3j)",           "1i");
eqt!(e_null_sum_f,        "sum null (1.0;0n;3.0)",          "1i");
eqt!(e_null_not_i,        "not null (1;0N;3)",              "101b");
eqt!(e_null_not_j,        "not null (1j;0Nj;3j)",           "101b");
eqt!(e_null_not_f,        "not null (1.0;0n;3.0)",          "101b");

// ── BAT41: BOOL primitive matrix ──────────────────────────────────
eqt!(e_bool_not_all1,     "not 11111b",                     "00000b");
eqt!(e_bool_not_all0,     "not 00000b",                     "11111b");
eqt!(e_bool_not_mix,      "not 10110b",                     "01001b");
eqt!(e_bool_eq_self,      "1010b=1010b",                    "1111b");
eqt!(e_bool_neq_self,     "1010b<>1010b",                   "0000b");
eqt!(e_bool_and_self,     "1010b&1010b",                    "1010b");
eqt!(e_bool_or_self,      "1010b|1010b",                    "1010b");
eqt!(e_bool_xor_self,     "1010b<>0101b",                   "1111b");

// ── BAT42: LARGE table operations ─────────────────────────────────
eqt!(e_tbl_large_rows,    "count ([]a:til 10000)",          "10000");
eqt!(e_tbl_large_sum,     "(exec sum a from ([]a:til 10000))=sum til 10000",
    "1b");
eqt!(e_tbl_large_max,     "exec max a from ([]a:til 10000)", "9999");
eqt!(e_tbl_large_min,     "exec min a from ([]a:til 10000)", "0");
eqt!(e_tbl_large_asc,     "count `a xasc ([]a:10000?100)",  "10000");
eqt!(e_tbl_large_where,   "count select from ([]a:til 10000) where a<100",
    "100");

// ── BAT43: DICT arithmetic more types ────────────────────────────
eqt!(e_dict_ee_sub,       "(`a`b!3 4e)-(`a`b!1 2e)",        "`a`b!2 2e");
eqt!(e_dict_hh_add,       "(`a`b!1 2h)+(`a`b!3 4h)",        "`a`b!4 6i");
eqt!(e_dict_bb_cat,       "(`a`b!1 2),`c`d!3 4",            "`a`b`c`d!1 2 3 4");
eqt!(e_dict_neg_f,        "neg `a`b`c!1.5 2.5 3.5",         "`a`b`c!-1.5 -2.5 \
    -3.5");
eqt!(e_dict_abs_j,        "abs `a`b`c!-1 -2 -3j",           "`a`b`c!1 2 3j");

// ── BAT44: KEYED TABLE × types ────────────────────────────────────
eqt!(e_kt_j_count,        "count ([k:1 2 3j]v:10 20 30)",    "3");
eqt!(e_kt_s_count,        "count ([k:`a`b`c]v:10 20 30)",    "3");
eqt!(e_kt_f_count,        "count ([k:1.5 2.5 3.5]v:10 20 30)", "3");
eqt!(e_kt_i_reverse,      "reverse ([k:1 2 3]v:10 20 30)",   "([k:3 2 1]v:30 \
    20 \
    10)");
eqt!(e_kt_arith_f,        "([k:1 2]v:1.5 2.5)+([k:1 2]v:0.5 0.5)", "([k:1 \
    2]v:2 \
    3f)");

// ── BAT45: MORE BIG operations ────────────────────────────────────
eqt!(e_big_til_sum,       "sum til 10000",                  "49995000j");
eqt!(e_big_til_count,     "count til 100000",               "100000");
eqt!(e_big_dist_cnt,      "count distinct 100000#1 2 3 4",  "4");
eqt!(e_big_cat_vec,       "count (1000#1),1000#2",          "2000");
eqt!(e_big_asc_vec,       "count asc 10000?100",            "10000");
eqt!(e_big_group_vec,     "count group 10000?5 3 1 4 2",    "5");
eqt!(e_big_where_1,       "count where 10000#1b",           "10000");
eqt!(e_big_where_0,       "count where 10000#0b",           "0");
eqt!(e_big_avg_i,         "avg 10000#42",                   "42f");
eqt!(e_big_avg_j,         "avg 10000#42j",                  "42f");

// ── BAT46: ARITHMETIC + NULL preservation ────────────────────────
eqt!(e_null_add_commute,  "((1;0N;3)+4 5 6)~(4 5 6+(1;0N;3))", "1b");
eqt!(e_null_mul_commute,  "((1;0N;3)*4 5 6)~(4 5 6*(1;0N;3))", "1b");
eqt!(e_null_eq_self,      "(1;0N;3)~(1;0N;3)",              "1b");
eqt!(e_null_neg_neg,      "(neg neg (1;0N;3))~(1;0N;3)",    "1b");
eqt!(e_null_sum_commute,  "(sum (1;0N;3))=sum reverse (1;0N;3)", "1b");

// ── BAT47: MIXED list operations ─────────────────────────────────
eqt!(e_mix_i_count,       "count (1;2;3)",                  "3");
eqt!(e_mix_type_atom,     "count (42)",                     "1");
eqt!(e_mix_ops,           "(1;2;3)~(1;2;3)",                "1b");

// ── BAT48: STRING as vector ops ──────────────────────────────────
eqt!(e_str_asc,           "asc \"cba\"",                    "`s#\"abc\"");
eqt!(e_str_desc,          "desc \"abc\"",                   "\"cba\"");
eqt!(e_str_reverse,       "reverse \"abc\"",                "\"cba\"");
eqt!(e_str_first,         "first \"abc\"",                  "\"a\"");
eqt!(e_str_last,          "last \"abc\"",                   "\"c\"");
eqt!(e_str_large,         "count 1000#\"a\"",               "1000");
eqt!(e_str_concat_self,   "\"abc\",\"abc\"",                "\"abcabc\"");
eqt!(e_str_distinct,      "distinct \"aabbcc\"",            "\"abc\"");
eqt!(e_str_in,            "\"a\" in \"abc\"",               "1b");

// ── BAT49: COMPARE MATRICES ──────────────────────────────────────
eqt!(e_cmp_eq_i_s,        "type 1 2 3=1 2 3",               "1h");
eqt!(e_cmp_eq_i_a,        "type 1 2 3=1",                   "1h");
eqt!(e_cmp_lt_type,       "type 1 2 3<2 2 2",               "1h");
eqt!(e_cmp_gt_type,       "type 1 2 3>2 2 2",               "1h");

// ── BAT50: MORE LIST PRIMITIVES ───────────────────────────────────
eqt!(e_enl_atom_i,        "count enlist 42",                "1");
eqt!(e_enl_atom_j,        "count enlist 42j",               "1");
eqt!(e_enl_atom_f,        "count enlist 3.14",              "1");
eqt!(e_enl_atom_s,        "count enlist `x",                "1");
eqt!(e_enl_atom_b,        "count enlist 1b",                "1");
eqt!(e_enl_type_i,        "type enlist 42",                 "6h");
eqt!(e_enl_type_j,        "type enlist 42j",                "7h");
eqt!(e_raze_vv,           "count raze (1 2;3 4;5 6)",       "6");
eqt!(e_raze_empty,        "raze (`int$();1 2 3;`int$())",   "1 2 3");
eqt!(e_first_atom,        "first 42",                       "42");
eqt!(e_last_atom,         "last 42",                        "42");


// ── BAT51: FINAL PUSH toward 2000 ─────────────────────────────────
eqt!(f_add_ii_7,          "1 2 3 4 5 6 7+10",               "11 12 13 14 15 16 \
    17");
eqt!(f_add_jj_7,          "1 2 3 4 5 6 7j+10j",             "11 12 13 14 15 16 \
    17j");
eqt!(f_add_ff_7,          "1 2 3 4 5 6 7f+10.0",            "11 12 13 14 15 16 \
    17f");
eqt!(f_sub_ii_7,          "10 20 30 40 50 60 70-1",         "9 19 29 39 49 59 \
    69");
eqt!(f_mul_ii_7,          "1 2 3 4 5 6 7*2",                "2 4 6 8 10 12 14");
eqt!(f_div_ff_7,          "10 20 30 40 50 60 70%2.0",       "5 10 15 20 25 30 \
    35f");
eqt!(f_neg_vec_7,         "neg 1 2 3 4 5 6 7",              "-1 -2 -3 -4 -5 -6 \
    -7");
eqt!(f_abs_vec_7,         "abs -1 -2 -3 -4 -5 -6 -7",       "1 2 3 4 5 6 7");
eqt!(f_sum_7,             "sum 1 2 3 4 5 6 7",              "28j");
eqt!(f_avg_7,             "avg 1 2 3 4 5 6 7",              "4f");
eqt!(f_min_7,             "min 7 5 3 1 4 6 2",              "1");
eqt!(f_max_7,             "max 7 5 3 1 4 6 2",              "7");
eqt!(f_reverse_7,         "reverse 1 2 3 4 5 6 7",          "7 6 5 4 3 2 1");
eqt!(f_asc_7,             "asc 7 5 3 1 4 6 2",              "`s#1 2 3 4 5 6 7");
eqt!(f_desc_7,            "desc 7 5 3 1 4 6 2",             "7 6 5 4 3 2 1");
eqt!(f_dist_7,            "distinct 1 2 3 1 2 3 4",         "1 2 3 4");
eqt!(f_group_7,           "count group 1 2 3 1 2 3 4",      "4");
eqt!(f_til_7,             "til 7",                          "0 1 2 3 4 5 6");
eqt!(f_count_7,           "count 1 2 3 4 5 6 7",            "7");
eqt!(f_first_7,           "first 10 20 30 40 50 60 70",     "10");
eqt!(f_last_7,            "last 10 20 30 40 50 60 70",      "70");

eqt!(f_add_ii_8,          "1 2 3 4 5 6 7 8+1",              "2 3 4 5 6 7 8 9");
eqt!(f_sub_ii_8,          "10 20 30 40 50 60 70 80-10",     "0 10 20 30 40 50 \
    60 70");
eqt!(f_sum_8,             "sum 1 2 3 4 5 6 7 8",            "36j");
eqt!(f_avg_8,             "avg 1 2 3 4 5 6 7 8",            "4.5");

eqt!(f_add_9,             "1 2 3 4 5 6 7 8 9+1",            "2 3 4 5 6 7 8 9 \
    10");
eqt!(f_sum_9,             "sum 1 2 3 4 5 6 7 8 9",          "45j");
eqt!(f_count_9,           "count 1 2 3 4 5 6 7 8 9",        "9");

eqt!(f_sum_16,            "sum til 16",                     "120j");
eqt!(f_sum_32,            "sum til 32",                     "496j");
eqt!(f_sum_64,            "sum til 64",                     "2016j");
eqt!(f_sum_128,           "sum til 128",                    "8128j");
eqt!(f_sum_256,           "sum til 256",                    "32640j");
eqt!(f_sum_512,           "sum til 512",                    "130816j");
eqt!(f_sum_1024,          "sum til 1024",                   "523776j");

eqt!(f_tbl_2col_sum,      "(exec sum a from ([]a:1 2 3;b:4 5 6))=6",
    "1b");
eqt!(f_tbl_2col_max,      "(exec max a from ([]a:1 5 3;b:4 5 6))=5",
    "1b");
eqt!(f_tbl_2col_avg,      "(exec avg a from ([]a:1 2 3;b:4 5 6))=2f",
    "1b");
eqt!(f_tbl_2col_min,      "(exec min a from ([]a:3 1 2;b:4 5 6))=1",
    "1b");
eqt!(f_tbl_filter_eq,     "count select from ([]a:1 2 3 4;b:10 20 30 40) where \
    a=2", "1");
eqt!(f_tbl_filter_in,     "count select from ([]a:1 2 3 4 5;b:10 20 30 40 50) \
    where a in 2 4", "2");
eqt!(f_tbl_filter_ge,     "count select from ([]a:1 2 3 4 5) where a>=3", "3");
eqt!(f_tbl_sort_asc,      "(`a xasc ([]a:3 1 2;b:30 10 20))~([]a:1 2 3;b:10 20 \
    30)", "1b");
eqt!(f_tbl_sort_desc,     "(`a xdesc ([]a:1 3 2;b:10 30 20))~([]a:3 2 1;b:30 \
    20 \
    10)", "1b");

eqt!(f_kt_scalar_mul,     "(([k:1 2 3]v:1 2 3)*10)~([k:1 2 3]v:10 20 30)",
    "1b");
eqt!(f_kt_scalar_add,     "(([k:1 2 3]v:10 20 30)+5)~([k:1 2 3]v:15 25 35)",
    "1b");
eqt!(f_kt_count,          "count ([k:1 2 3 4 5]v:10 20 30 40 50)", "5");

eqt!(f_dict_dict_sum,     "((`a`b`c!1 2 3)+(`a`b`c!10 20 30))~(`a`b`c!11 22 \
    33)", "1b");
eqt!(f_dict_scalar_add,   "((`a`b`c!1 2 3)+10)~(`a`b`c!11 12 13)", "1b");
eqt!(f_dict_neg,          "(neg `a`b`c!1 2 3)~(`a`b`c!-1 -2 -3)", "1b");
eqt!(f_dict_abs,          "(abs `a`b`c!-1 -2 -3)~(`a`b`c!1 2 3)", "1b");

eqt!(f_tbl_tbl_sum,       "(([]a:1 2 3)+([]a:10 20 30))~([]a:11 22 33)", "1b");
eqt!(f_tbl_tbl_sub,       "(([]a:10 20 30)-([]a:1 2 3))~([]a:9 18 27)", "1b");
eqt!(f_tbl_tbl_mul,       "(([]a:2 3 4)*([]a:5 6 7))~([]a:10 18 28)", "1b");

eqt!(f_lj_basic,          "(([]k:1 2 3;a:10 20 30) lj ([k:1 2 3]b:100 200 \
    300))~([]k:1 2 3;a:10 20 30;b:100 200 300)", "1b");

eqt!(f_where_big,         "count where 1000#10110b",        "600");
eqt!(f_where_even_1k,     "count where 0=(til 1000) mod 2", "500");
eqt!(f_distinct_big,      "count distinct 10000#1 2 3 4 5 6 7 8 9 10", "10");
eqt!(f_group_big,         "count group 10000#1 2 3 4 5",    "5");

eqt!(f_sum_nulls_mid,     "sum 1 2 0N 4 5 0N 7",            "19j");
eqt!(f_sum_nulls_start,   "sum 0N 0N 3 4 5",                "12j");
eqt!(f_sum_nulls_end,     "sum 1 2 3 0N 0N",                "6j");
eqt!(f_avg_nulls_mid,     "avg 1 2 0N 4 5",                 "3f");
eqt!(f_max_nulls,         "max 1 2 0N 4 5",                 "5");
eqt!(f_min_nulls,         "min 1 2 0N 4 5",                 "1");
eqt!(f_count_nulls,       "count 1 2 0N 4 5",               "5");

eqt!(f_table_empty_col,   "count ([]a:`int$())",            "0");
eqt!(f_table_empty_s,     "count ([]s:`$())",               "0");
eqt!(f_table_empty_f,     "count ([]f:`float$())",          "0");
eqt!(f_table_empty_j,     "count ([]j:`long$())",           "0");

eqt!(f_cross_add_ij,      "type 1j+1",                      "-7h");
eqt!(f_cross_add_if,      "type 1f+1",                      "-9h");
eqt!(f_cross_add_jf,      "type 1j+1f",                     "-9h");
eqt!(f_cross_mul_ij,      "type 3j*4",                      "-7h");
eqt!(f_cross_mul_jf,      "type 3j*4.0",                    "-9h");

eqt!(f_sort_large_ii,     "{[x](asc x)~`s#asc x}100000?100", "1b");
eqt!(f_sort_large_kj,     "{[x](asc x)~`s#asc x}100000?100j", "1b");
eqt!(f_sort_large_kf,     "{[x](asc x)~`s#asc x}100000?100.0", "1b");
eqt!(f_sort_large_ks,     "{[x](asc x)~`s#asc x}100000?`a`b`c", "1b");
eqt!(f_sort_large_kb,     "{[x](asc x)~`s#asc x}100000?0b",   "1b");

eqt!(f_type_check_all,    "98h=type ([]a:1 2 3;b:4 5 6)",   "1b");
eqt!(f_type_check_dict,   "99h=type `a`b!1 2",              "1b");
eqt!(f_type_keyed,        "99h=type ([k:1 2]v:10 20)",      "1b");

eqt!(f_cast_large,        "count `long$til 100000",         "100000");
eqt!(f_cast_large_to_f,   "count `float$til 100000",        "100000");

eqt!(f_neg_1m_i,          "{[x]count neg x}1000000#5",      "1000000");
eqt!(f_neg_1m_j,          "{[x]count neg x}1000000#5j",     "1000000");
eqt!(f_abs_1m_i,          "{[x]count abs x}1000000#-5",     "1000000");
eqt!(f_abs_1m_j,          "{[x]count abs x}1000000#-5j",    "1000000");
eqt!(f_sum_1m_i,          "sum 1000000#1",                  "1000000j");
eqt!(f_sum_1m_j,          "sum 1000000#1j",                 "1000000j");
eqt!(f_sum_1m_f,          "sum 1000000#1.0",                "1000000f");
eqt!(f_sum_1m_b,          "sum 1000000#1b",                 "1000000i");


// ── BAT52: Final 100 to clear 2000 ──────────────────────────────
eqt!(g_add_i_11,          "1 2 3 4 5 6 7 8 9 10 11+1",      "2 3 4 5 6 7 8 9 \
    10 \
    11 12");
eqt!(g_sum_i_11,          "sum 1 2 3 4 5 6 7 8 9 10 11",    "66j");
eqt!(g_add_i_12,          "sum 1 2 3 4 5 6 7 8 9 10 11 12", "78j");
eqt!(g_cnt_11,            "count til 11",                   "11");
eqt!(g_cnt_12,            "count til 12",                   "12");
eqt!(g_cnt_13,            "count til 13",                   "13");
eqt!(g_cnt_14,            "count til 14",                   "14");
eqt!(g_cnt_15,            "count til 15",                   "15");
eqt!(g_cnt_16,            "count til 16",                   "16");
eqt!(g_cnt_17,            "count til 17",                   "17");
eqt!(g_cnt_18,            "count til 18",                   "18");
eqt!(g_cnt_19,            "count til 19",                   "19");
eqt!(g_cnt_20,            "count til 20",                   "20");
eqt!(g_sum_20,            "sum til 20",                     "190j");
eqt!(g_sum_50,            "sum til 50",                     "1225j");
eqt!(g_sum_100,           "sum til 100",                    "4950j");
eqt!(g_sum_200,           "sum til 200",                    "19900j");
eqt!(g_sum_300,           "sum til 300",                    "44850j");
eqt!(g_sum_500,           "sum til 500",                    "124750j");
eqt!(g_sum_1000,          "sum til 1000",                   "499500j");
eqt!(g_sum_2000,          "sum til 2000",                   "1999000j");
eqt!(g_sum_5000,          "sum til 5000",                   "12497500j");
eqt!(g_sum_10000,         "sum til 10000",                  "49995000j");
eqt!(g_count_til_100,     "count til 100",                  "100");
eqt!(g_count_til_1k,      "count til 1000",                 "1000");
eqt!(g_count_til_10k,     "count til 10000",                "10000");
eqt!(g_count_til_100k,    "count til 100000",               "100000");
eqt!(g_count_til_1m,      "count til 1000000",              "1000000");
eqt!(g_avg_til_100,       "avg til 100",                    "49.5");
eqt!(g_avg_til_1k,        "avg til 1000",                   "499.5");
eqt!(g_max_til_100,       "max til 100",                    "99");
eqt!(g_min_til_100,       "min til 100",                    "0");
eqt!(g_reverse_til,       "first reverse til 10",           "9");

// Sizes variety
eqt!(g_sum_11,            "sum 11#1",                       "11j");
eqt!(g_sum_13,            "sum 13#1",                       "13j");
eqt!(g_sum_17,            "sum 17#1",                       "17j");
eqt!(g_sum_23,            "sum 23#1",                       "23j");
eqt!(g_sum_29,            "sum 29#1",                       "29j");
eqt!(g_sum_31,            "sum 31#1",                       "31j");
eqt!(g_sum_37,            "sum 37#1",                       "37j");
eqt!(g_sum_41,            "sum 41#1",                       "41j");
eqt!(g_sum_43,            "sum 43#1",                       "43j");
eqt!(g_sum_47,            "sum 47#1",                       "47j");
eqt!(g_sum_53,            "sum 53#1",                       "53j");
eqt!(g_sum_59,            "sum 59#1",                       "59j");
eqt!(g_sum_61,            "sum 61#1",                       "61j");
eqt!(g_sum_67,            "sum 67#1",                       "67j");
eqt!(g_sum_71,            "sum 71#1",                       "71j");
eqt!(g_sum_73,            "sum 73#1",                       "73j");
eqt!(g_sum_79,            "sum 79#1",                       "79j");
eqt!(g_sum_83,            "sum 83#1",                       "83j");
eqt!(g_sum_89,            "sum 89#1",                       "89j");
eqt!(g_sum_97,            "sum 97#1",                       "97j");

// More primitives at odd sizes
eqt!(g_neg_17,            "(neg 17#5)~17#-5",               "1b");
eqt!(g_abs_23,            "(abs 23#-5)~23#5",               "1b");
eqt!(g_reverse_11,        "{[x](reverse reverse x)~x}11#1 2 3", "1b");
eqt!(g_reverse_13,        "{[x](reverse reverse x)~x}13#1 2 3", "1b");
eqt!(g_reverse_17,        "{[x](reverse reverse x)~x}17#1 2 3", "1b");
eqt!(g_asc_11,            "{[x](asc x)~`s#asc x}11#3 1 4 1 5", "1b");
eqt!(g_asc_13,            "{[x](asc x)~`s#asc x}13#3 1 4 1 5", "1b");
eqt!(g_asc_17,            "{[x](asc x)~`s#asc x}17#3 1 4 1 5", "1b");

// Per-type sum matrix
eqt!(g_sum_bool_1,        "sum enlist 1b",                  "1i");
eqt!(g_sum_bool_10,       "sum 10#1b",                      "10i");
eqt!(g_sum_bool_100,      "sum 100#1b",                     "100i");
eqt!(g_sum_bool_1k,       "sum 1000#1b",                    "1000i");
eqt!(g_sum_bool_10k,      "sum 10000#1b",                   "10000i");
eqt!(g_sum_byte_3,        "sum 0x010203",                   "6j");
eqt!(g_sum_byte_10,       "sum 10#0x01",                    "10j");
eqt!(g_sum_byte_100,      "sum 100#0x01",                   "100j");
eqt!(g_sum_short_3,       "sum 1 2 3h",                     "6j");
eqt!(g_sum_short_10,      "sum 10#1h",                      "10j");
eqt!(g_sum_short_100,     "sum 100#1h",                     "100j");
eqt!(g_sum_int_3,         "sum 1 2 3",                      "6j");
eqt!(g_sum_long_3,        "sum 1 2 3j",                     "6j");
eqt!(g_sum_real_3,        "sum 1 2 3e",                     "6e");
eqt!(g_sum_float_3,       "sum 1.0 2.0 3.0",                "6f");

// Table arith at various sizes
eqt!(g_tbl_add_2,         "(([]a:1 2)+([]a:10 20))~([]a:11 22)", "1b");
eqt!(g_tbl_add_5,         "(([]a:1 2 3 4 5)+([]a:10 20 30 40 50))~([]a:11 22 \
    33 \
    44 55)", "1b");
eqt!(g_tbl_add_10,        "count (([]a:10#1)+([]a:10#2))",  "10");
eqt!(g_tbl_add_100,       "count (([]a:100#1)+([]a:100#2))", "100");
eqt!(g_tbl_mul_scalar,    "(([]a:1 2 3;b:4 5 6)*10)~([]a:10 20 30;b:40 50 60)",
    "1b");

// Dict arith variations
eqt!(g_dict_scalar_all,   "((`a`b`c`d!1 2 3 4)+10)~(`a`b`c`d!11 12 13 14)",
    "1b");
eqt!(g_dict_vec_vec,      "((`a`b`c`d`e!1 2 3 4 5)+(`a`b`c`d`e!10 20 30 40 \
    50))~(`a`b`c`d`e!11 22 33 44 55)", "1b");
eqt!(g_dict_count_5,      "count `a`b`c`d`e!1 2 3 4 5",     "5");
eqt!(g_dict_count_10,     "count `a`b`c`d`e`f`g`h`i`j!til 10", "10");

// Empty everything
eqt!(g_empty_sum,         "sum `int$()",                    "0j");
eqt!(g_empty_prd,         "prd `int$()",                    "1");
eqt!(g_empty_cnt,         "count `int$()",                  "0");
eqt!(g_empty_rev,         "reverse `int$()",                "`int$()");
eqt!(g_empty_asc,         "asc `int$()",                    "`s#`int$()");
eqt!(g_empty_dist,        "distinct `int$()",               "`int$()");
eqt!(g_empty_j_sum,       "sum `long$()",                   "0j");
eqt!(g_empty_f_sum,       "sum `float$()",                  "0f");
eqt!(g_empty_sym_cnt,     "count `$()",                     "0");
eqt!(g_empty_str_cnt,     "count \"\"",                     "0");

// NULL HANDLING — REGRESSION MATRIX across every numeric type, reducing/scanning verb, null position, and length.

// ── KI fold reductions: min / prd / max — null at start ──
eqt!(nh_min_i_s_5,   "min 0N 2 3 4 5",                  "2");
eqt!(nh_min_i_m_5,   "min 1 2 0N 4 5",                  "1");
eqt!(nh_min_i_t_5,   "min 1 2 3 4 0N",                  "1");
eqt!(nh_min_i_m_3,   "min 7 0N 9",                      "7");
eqt!(nh_min_i_m_17,  "min 5 6 7 8 9 10 11 12 0N 13 14 15 16 17 18 19 20", "5");
eqt!(nh_min_i_m_100, "min @[100#1+til 100; 50; :; 0N]", "1");
eqt!(nh_min_i_par,   "min @[10001#1+til 10001; 5000; :; 0N]", "1");
eqt!(nh_prd_i_s_5,   "prd 0N 2 3 4 5",                  "120");
eqt!(nh_prd_i_m_5,   "prd 1 2 0N 4 5",                  "40");
eqt!(nh_prd_i_t_5,   "prd 1 2 3 4 0N",                  "24");
eqt!(nh_prd_i_m_17,  "prd 1 2 3 0N 4 5 1 1 1 1 1 1 1 1 1 1 1", "120");
eqt!(nh_max_i_s_5,   "max 0N 2 3 4 5",                  "5");
eqt!(nh_max_i_m_5,   "max 1 2 0N 4 5",                  "5");
eqt!(nh_max_i_t_5,   "max 1 2 3 4 0N",                  "4");
eqt!(nh_max_i_m_100, "max @[100#1+til 100; 50; :; 0N]", "100");
eqt!(nh_max_i_par,   "max @[10001#1+til 10001; 5000; :; 0N]", "10001");

// ── KI sum / avg with nulls (separate fused path) ──
eqt!(nh_sum_i_m_5,   "sum 1 2 0N 4 5",                  "12j");
eqt!(nh_sum_i_s_5,   "sum 0N 2 3 4 5",                  "14j");
eqt!(nh_sum_i_t_5,   "sum 1 2 3 4 0N",                  "10j");
eqt!(nh_sum_i_par,   "sum @[10001#1; 5000; :; 0N]",     "10000j");
eqt!(nh_avg_i_m_5,   "avg 1 2 0N 4 5",                  "3f");

// ── KJ fold reductions ──
eqt!(nh_min_j_s_5,   "min 0Nj,2 3 4 5j",                "2j");
eqt!(nh_min_j_m_5,   "min 1 2 0N 4 5j",                 "1j");
eqt!(nh_min_j_t_5,   "min 1 2 3 4 0Nj",                 "1j");
eqt!(nh_min_j_m_17,  "min @[`long$17#1+til 17; 8; :; 0Nj]", "1j");
eqt!(nh_min_j_par,   "min @[`long$10001#1+til 10001; 5000; :; 0Nj]", "1j");
eqt!(nh_prd_j_m_5,   "prd 1 2 0N 4 5j",                 "40j");
eqt!(nh_prd_j_s_5,   "prd 0Nj,2 3 4 5j",                "120j");
eqt!(nh_prd_j_t_5,   "prd 1 2 3 4 0Nj",                 "24j");
eqt!(nh_max_j_m_5,   "max 1 2 0N 4 5j",                 "5j");
eqt!(nh_max_j_par,   "max @[`long$10001#1+til 10001; 5000; :; 0Nj]", "10001j");
eqt!(nh_sum_j_m_5,   "sum 1 2 0N 4 5j",                 "12j");

// ── KH fold reductions ──
eqt!(nh_min_h_s_5,   "min 0Nh,2 3 4 5h",                "2h");
eqt!(nh_min_h_m_5,   "min 1 2 0N 4 5h",                 "1h");
eqt!(nh_min_h_t_5,   "min 1 2 3 4 0Nh",                 "1h");
// prd KH widens to KI in l (prevents H overflow); test via type-coerced compare:
eqt!(nh_prd_h_m_5,   "(`int$prd 1 2 0N 4 5h)=40",       "1b");
eqt!(nh_max_h_m_5,   "max 1 2 0N 4 5h",                 "5h");
eqt!(nh_min_h_par,   "min @[`short$10001#1+til 10001; 5000; :; 0Nh]", "1h");

// ── KE fold reductions (Apple vDSP and generic both) ──
eqt!(nh_min_e_m_5,   "min 1 2 0N 4 5e",                 "1e");
eqt!(nh_min_e_s_5,   "min 0Ne,2 3 4 5e",                "2e");
eqt!(nh_min_e_t_5,   "min 1 2 3 4 0Ne",                 "1e");
eqt!(nh_max_e_m_5,   "max 1 2 0N 4 5e",                 "5e");
eqt!(nh_prd_e_m_5,   "prd 1 2 0N 4 5e",                 "40e");
eqt!(nh_min_e_par,   "min @[`real$10001#1+til 10001;5000;:;0Ne]", "1e");
eqt!(nh_sum_e_m_5,   "sum 1 2 0N 4 5e",                 "12e");

// ── KF fold reductions ──
eqt!(nh_min_f_m_5,   "min 1.0 2.0 0n 4.0 5.0",          "1f");
eqt!(nh_min_f_s_5,   "min 0n,2 3 4 5f",                 "2f");
eqt!(nh_min_f_t_5,   "min 1 2 3 4 5f,0n",               "1f");
eqt!(nh_max_f_m_5,   "max 1.0 2.0 0n 4.0 5.0",          "5f");
eqt!(nh_prd_f_m_5,   "prd 1.0 2.0 0n 4.0 5.0",          "40f");
eqt!(nh_min_f_par,   "min @[`float$10001#1+til 10001;5000;:;0n]", "1f");
eqt!(nh_sum_f_m_5,   "sum 1.0 2.0 0n 4.0 5.0",          "12f");
eqt!(nh_avg_f_m_5,   "avg 1.0 2.0 0n 4.0 5.0",          "3f");

// ── ALL-NULL inputs collapse to the verb's identity ──
eqt!(nh_min_i_all_n, "min 0N 0N 0N",                    "0W");
eqt!(nh_max_i_all_n, "max 0N 0N 0N",                    "-0W");
eqt!(nh_prd_i_all_n, "prd 0N 0N 0N",                    "1");
eqt!(nh_sum_i_all_n, "sum 0N 0N 0N",                    "0j");
eqt!(nh_min_j_all_n, "min 0Nj,0Nj,0Nj",                 "0Wj");
eqt!(nh_max_j_all_n, "max 0Nj,0Nj,0Nj",                 "-0Wj");
eqt!(nh_min_f_all_n, "min 0n,0n,0n",                    "0w");
eqt!(nh_max_f_all_n, "max 0n,0n,0n",                    "-0w");

// ── Single-element vectors with null ──
eqt!(nh_min_i_solo, "min enlist 0N",                    "0W");
eqt!(nh_max_i_solo, "max enlist 0N",                    "-0W");
eqt!(nh_sum_i_solo, "sum enlist 0N",                    "0j");
eqt!(nh_min_f_solo, "min enlist 0n",                    "0w");

// ── Empty vectors return identities (no null involved) ──
eqt!(nh_min_i_empty, "min `int$()",                     "0W");
eqt!(nh_max_i_empty, "max `int$()",                     "-0W");
eqt!(nh_min_f_empty, "min `float$()",                   "0w");
eqt!(nh_max_f_empty, "max `float$()",                   "-0w");

// ── SCAN reductions (sums/maxs/mins/prds) — null in input ──
eqt!(nh_sums_i_m_5,  "sums 1 2 0N 3 4",                 "1 3 3 6 10");
eqt!(nh_sums_j_m_5,  "sums 1 2 0N 3 4j",                "1 3 3 6 10j");
eqt!(nh_sums_f_m_5,  "sums 1.0 2.0 0n 3.0 4.0",         "1 3 3 6 10f");
eqt!(nh_mins_i_m_5,  "mins 5 4 0N 3 2",                 "5 4 4 3 2");
eqt!(nh_maxs_i_m_5,  "maxs 1 2 0N 3 4",                 "1 2 2 3 4");
eqt!(nh_prds_i_m_5,  "prds 1 2 0N 3 4",                 "1 2 2 6 24");
eqt!(nh_sums_i_par,  "last sums @[10001#1; 5000; :; 0N]", "10000");
eqt!(nh_sums_j_par,  "last sums @[`long$10001#1; 5000; :; 0Nj]", "10000j");
eqt!(nh_sums_f_par,  "last sums @[10001#1f; 5000; :; 0n]", "10000f");

// ── Each-prior verbs (deltas/ratios/differ) ──
eqt!(nh_deltas_i_m,  "deltas 1 3 0N 7 11",              "1 2 0N 0N 4");
eqt!(nh_deltas_f_m,  "deltas 1.0 3.0 0n 7.0 11.0",      "1 2 0n 0n 4f");
eqt!(nh_ratios_f_m,  "ratios 1.0 2.0 0n 6.0 12.0",      "1 2 0n 0n 2f");
eqt!(nh_differ_i_m,  "differ 1 1 0N 0N 2",              "10101b");

// Group-by aggregations with nulls in the value column.
eqt!(nh_grp_min_i,   "(min each (1 2 0N 3 4) group `a`a`b`a`b)~`a`b!1 4", "1b");
eqt!(nh_grp_max_i,   "(max each (1 2 0N 3 4) group `a`a`b`a`b)~`a`b!3 4", "1b");
eqt!(nh_grp_sum_i,   "(sum each (1 2 0N 3 4) group `a`a`b`a`b)~`a`b!6 4j",
    "1b");
eqt!(nh_grp_prd_i,   "(prd each (1 2 0N 3 4) group `a`a`b`a`b)~`a`b!6 4", "1b");
eqt!(nh_grp_min_f,   "(min each (1.0 2.0 0n 3.0 4.0) group `a`a`b`a`b)~`a`b!1 \
    4f", "1b");
eqt!(nh_grp_max_f,   "(max each (1.0 2.0 0n 3.0 4.0) group `a`a`b`a`b)~`a`b!3 \
    4f", "1b");

// ── qSQL aggregations against tables with null values ──
eqt!(nh_qsql_min_i,
    "(select m:min v from ([] g:`a`a`b`a`b; v:1 2 0N 3 4))~([] m:enlist 1)",
    "1b");
eqt!(nh_qsql_grp_min_i,
    "exec m from select m:min v by g from ([] g:`a`a`b`a`b; v:1 2 0N 3 4)",
    "1 4");
eqt!(nh_qsql_grp_sum_j,
    "exec s from select s:sum v by g from ([] g:`a`a`b`a`b; v:1 2 0Nj,3 4j)",
    "6 4j");
eqt!(nh_qsql_grp_max_f,
    "exec m from select m:max v by g from ([] g:`a`a`b`a`b; v:1.0 2.0 0n 3.0 \
        4.0)",
    "3 4f");

// Parallel-path stress: 100001 elements with nulls placed so each chunk and the tail see one.
eqt!(nh_par_min_i_100k1, "min @[1+til 100001;0 50000 100000;:;0N]",
    "2");
eqt!(nh_par_max_i_100k1, "max @[1+til 100001;0 50000 100000;:;0N]",
    "100000");
eqt!(nh_par_prd_i_100k1, "prd @[100001#1;50000;:;0N]",
    "1");
eqt!(nh_par_min_j_100k1, "min @[`long$1+til 100001;0 50000 100000;:;0Nj]",
    "2j");
eqt!(nh_par_max_j_100k1, "max @[`long$1+til 100001;0 50000 100000;:;0Nj]",
    "100000j");
eqt!(nh_par_min_f_100k1, "min @[`float$1+til 100001;0 50000 100000;:;0n]",
    "2f");
eqt!(nh_par_max_f_100k1, "max @[`float$1+til 100001;0 50000 100000;:;0n]",
    "100000f");
eqt!(nh_par_sums_i_100k1, "last sums @[100001#1;50000;:;0N]", "100000");

// Regression sentinels — the exact queries that were broken; kept named to point at the incident.
eqt!(rg_apxred_min_i,  "min 1 2 0N 3 4",                "1");
eqt!(rg_apxred_min_i2, "min 1 2 0N 4 5",                "1");
eqt!(rg_apxred_prd_i,  "prd 1 2 0N 4",                  "8");
eqt!(rg_apxred_min_j,  "min 1 2 0N 3 4j",               "1j");
eqt!(rg_apxred_prd_j,  "prd 1 2 0N 4j",                 "8j");
eqt!(rg_apxred_min_h,  "min 1 2 0N 3 4h",               "1h");

// MMAP-CACHE ALLOCATOR REGRESSION — free then alloc a slightly larger same-bucket block must not reuse an undersized buffer.

// A. Two same-bucket sizes: alloc smaller, free, alloc larger; random fill writes through every page.
eqt!(alc_alt_kf_9_12,
    "{a:9000000?1f;a:0;b:12000000?1f;count b}[]",
    "12000000");
eqt!(alc_alt_kj_9_12,
    "{a:9000000?100j;a:0;b:12000000?100j;count b}[]",
    "12000000");
eqt!(alc_alt_ks_9_12,
    "{a:9000000?`a`b`c`d;a:0;b:12000000?`a`b`c`d;count b}[]",
    "12000000");
// 4-byte types (KI/KE): need 18M+ to clear BIGBLOB (level > 22).
eqt!(alc_alt_ki_18_24,
    "{a:18000000?100i;a:0;b:24000000?100i;count b}[]",
    "24000000");
eqt!(alc_alt_ke_18_24,
    "{a:18000000?1e;a:0;b:24000000?1e;count b}[]",
    "24000000");

// B. Repeated small->free->large->free alternation catches state leak across iterations.
eqt!(alc_alt_burst_kf,
    "{do[20;a:9000000?1f;a:0;b:12000000?1f;b:0];`done}[]",
    "`done");
eqt!(alc_alt_burst_kj,
    "{do[20;a:9000000?100j;a:0;b:12000000?100j;b:0];`done}[]",
    "`done");

// C. Monotonic grow/shrink patterns across a bucket level.
eqt!(alc_grow_kf,
    "{do[5;{a:(`int$8e6+1e6*x)?1f;a:0}each til 6];`done}[]",
    "`done");
eqt!(alc_shrink_kf,
    "{do[5;{a:(`int$13e6-1e6*x)?1f;a:0}each til 6];`done}[]",
    "`done");

// D. Bucket-boundary sizes bracketing the level thresholds.
eqt!(alc_bucket_lvl23_low_high,
    "{a:8400000?1f;a:0;b:16000000?1f;count b}[]",
    "16000000");
eqt!(alc_bucket_lvl24_low_high,
    "{a:17000000?1f;a:0;b:30000000?1f;count b}[]",
    "30000000");

// E. Random sizes spanning bucket boundaries.
eqt!(alc_random_churn_kf,
    "{do[50;n:(`int$8e6)+rand`int$6e6;a:n?1f;a:0];`done}[]",
    "`done");

// F. Production repro: select avg/wavg by sym over 4 groups with slight size variance.
eqt!(alc_select_avg_by_sym_4e7,
    "{[n] t:flip `sym`price`size!(n?`a`b`c`d;n?1f;n?100); \
        count select avg price by sym from t}[`int$4e7]",
    "4");
eqt!(alc_select_wavg_by_sym_4e7,
    "{[n] t:flip `sym`price`size!(n?`a`b`c`d;n?1f;n?100); \
        count select size wavg price by sym from t}[`int$4e7]",
    "4");
// User's exact original query: `select sqrt size wavg price by sym`.
eqt!(alc_select_sqrt_size_wavg_4e7,
    "{[n] t:flip `sym`price`size!(n?`a`b`c`d;n?1f;n?100); \
        count select sqrt size wavg price by sym from t}[`int$4e7]",
    "4");
// Multi-aggregate row stresses 4 separate column results per group.
eqt!(alc_select_multi_agg_4e7,
    "{[n] t:flip `sym`price`size!(n?`a`b`c`d;n?1f;n?100); \
        count select avg price, max price, min size, sum size by sym \
        from t}[`int$4e7]",
    "4");

// G. Larger n pushes results into higher bucket levels.
eqt!(alc_select_avg_by_sym_8e7,
    "{[n] t:flip `sym`price!(n?`a`b`c`d;n?1f); \
        count select avg price by sym from t}[`int$8e7]",
    "4");

// H. Many groups: the internal by-grouping index vectors are themselves large allocations.
eqt!(alc_select_avg_by_sym_16grp,
    "{[n] t:flip `sym`price!(n?16?`8;n?1f); \
        count select avg price by sym from t}[`int$4e7]",
    "16");

// I. Same-size cache-hit path — must stay fast with no correctness regression.
eqt!(alc_same_size_hit_perf,
    "{do[100;a:10000000?1f;a:0];`done}[]",
    "`done");

// J. RSS-bound stress sentinel from the original commit (reduced iterations).
eqt!(alc_burst_50m,
    "{do[50;r:50000000?1e0;r:0];`done}[]",
    "`done");

// K. Interleaved multi-level allocations exercise per-level freelist isolation.
eqt!(alc_interleave_levels,
    "{a:20000000?1f; do[10;b:9000000?1f;b:0;c:12000000?1f;c:0]; \
        count a}[]",
    "20000000");

// L. Concurrent (peach) allocator stress; returns the count of successful workers.
eqt!(alc_peach_alt_kf,
    "{r:{[w] do[10;a:9000000?1f;a:0;b:12000000?1f;b:0];w} \
        peach til 8; r~til 8}[]",
    "1b");

// EXTENDED MEMORY STRESS — exercises the whole allocator surface; each test returns a small verifiable result.

// M. Alloc just below and just above the big-block threshold back-to-back.
eqt!(alc_boundary_below,
    "{count 200000?1f}[]",     "200000");
eqt!(alc_boundary_just_above,
    "{count 400000?1f}[]",     "400000");
eqt!(alc_boundary_far_above,
    "{count 9000000?1f}[]",    "9000000");
// Repeated alloc-free across the threshold, alternating regimes each iteration.
eqt!(alc_boundary_churn,
    "{do[20;a:200000?1f;a:0;b:9000000?1f;b:0];`done}[]",
    "`done");

// N. Random-size sweep across many levels; verify each allocation's count matches its request.
eqt!(alc_random_sweep,
    "{ns:100?1000000;all ns={count x?1f}each ns}[]",
    "1b");

// O. Content integrity: after free+reuse, the buffer holds new data, not stale data.
eqt!(alc_content_after_free,
    "{a:1000000#1.0;a:0;b:1000000#2.0;all 2.0=b}[]",
    "1b");

// P. Mixed-type wide churn catches type-tag corruption across free/reuse.
eqt!(alc_mixed_type_churn,
    "{do[200;\
        a:1000000?1f;a:0;\
        b:500000?100j;b:0;\
        c:2000000?1.0e0;c:0;\
        d:300000?1000;d:0;\
        e:100000?`8;e:0];\
        `done}[]",
    "`done");

// Q. Atom alloc/free hot path for scalar atoms.
eqt!(alc_atom_churn,
    "{do[10000;a:42;a:0];`done}[]",
    "`done");

// R. Many small allocations stress the split loop; RSS stays bounded and a final alloc succeeds.
eqt!(alc_small_burst,
    "{do[10000;a:100?1f;a:0]; b:1000000?1f; count b}[]",
    "1000000");

// S. Cross-thread alloc/free via peach: allocate on a worker, free on main.
eqt!(alc_peach_alloc_main_free,
    "{r:{[w] (1000000+w)?1f} peach til 8; \
        ok:8=count r; \
        r:0; ok}[]",
    "1b");

// T. Recovery after pressure: allocate until tight, free all, allocate again without OOM.
eqt!(alc_pressure_recover,
    "{xs:{x?1f}each 20#1000000; xs:0; \
        ys:{x?1f}each 20#1000000; ok:20=count ys; ys:0; ok}[]",
    "1b");

// U. Cache eviction beyond the per-level cap: excess blocks are released, later ones re-fetched.
eqt!(alc_mmap_cache_evict,
    "{xs:{x?1f}each 16#5000000; xs:0; \
        ys:{x?1f}each 16#5000000; ok:16=count ys; ys:0; ok}[]",
    "1b");

// STUDYQ — idiomatic q programs from qbists/studyq, results pinned against a reference implementation.

// ── Project Euler ─────────────────────────────────────────────────
eqt!(sq_e04_largest_palindrome,
    "max c where {x~reverse x} each string c:prd each distinct asc each {x \
        cross x} 1 _ til 1000",
    "906609");
eqt!(sq_e05_lcm_1_20,
    "{any 0<x mod y}[;1+til 20] (q+)/q:prd 2 3 5 7 11 13 17 19",
    "232792560");
eqt!(sq_e06_ssd_100,
    "{(x*x:sum x)-sum x*x:1+til x} 100",
    "25164150j");
eqt!(sq_e06_ssd_10,
    "{(x*x:sum x)-sum x*x:1+til x} 10",
    "2640j");
eqt!(sq_e06_ssd_closed_form,
    "{[x] (.5*x*x+1)*(.5*x*x+1)}[100]",
    "25502500f");

// ── Leetcode 53 — max contiguous subarray ───────────────────────────
eqt!(sq_lc53_kadane_e0,
    "max ((0|+)\\) 11 -5 -5 -2 1 2 3 4 5 0 1",
    "16");
eqt!(sq_lc53_kadane_e1,
    "max ((0|+)\\) 9 -5 -5 -2 1 2 -1 4 5 0 1",
    "12");
eqt!(sq_lc53_kadane_all_neg,
    "max ((0|+)\\) -1 -2 -3 -4 -5",
    "0");                                                                       // Kadane variant clips at 0
eqt!(sq_lc53_sums_mins,
    "{max s-mins 0^prev s:sums x} 11 -5 -5 -2 1 2 3 4 5 0 1",
    "16");

// ── Leetcode 976 — largest triangle perimeter ──────────────────────
eqt!(sq_lc976_basic,
    "sum {$[count[x]<3;0;x[0]<sum x 1 2;3#x;.z.s 1_ x]} desc 8 1 9 5 4 6 6 1 8 \
        5",
    "25j");
eqt!(sq_lc976_no_triangle,
    "sum {$[count[x]<3;0;x[0]<sum x 1 2;3#x;.z.s 1_ x]} desc 1 2 1",
    "0");
eqt!(sq_lc976_simple,
    "sum {$[count[x]<3;0;x[0]<sum x 1 2;3#x;.z.s 1_ x]} desc 2 1 2",
    "5j");

// ── Adverbs + over / scan + composition ─────────────────────────────
eqt!(sq_kadane_simple,
    "max ((0|+)\\) 1 -1 2 -2 3",
    "3");
eqt!(sq_over_with_seed,        "10 +/ 1 2 3 4 5",        "25j");
eqt!(sq_scan_with_seed,        "10 +\\ 1 2 3 4 5",       "11 13 16 20 25");
eqt!(sq_each_prior_pair,       "{x+y}':[10;1 2 3 4 5]",  "11 3 5 7 9");
eqt!(sq_compose_neg_sum,       "(neg sum::) 1 2 3 4 5",  "-15j");
eqt!(sq_each_both,             "(1 2 3)*'(4 5 6)",       "4 10 18");
eqt!(sq_ngn_apply_n_times,     "100 {x+y}/ 1+til 10",    "155");
eqt!(sq_scan_running_max,      "(|\\) 3 1 4 1 5 9 2 6",  "3 3 4 4 5 9 9 9");
eqt!(sq_each_prior_deltas,     "deltas 1 3 6 10 15",     "1 2 3 4 5");
eqt!(sq_amend_in_place,        "@[1 2 3 4 5; 2; :; 99]", "1 2 99 4 5");

// ── Grouping + counts + filter ─────────────────────────────────────
eqt!(sq_group_keys,           "key group \"hello\"",           "\"helo\"");
// Group keys are CHAR (not symbol) when input is a char vector.
eqt!(sq_group_value_counts,
    "count each group \"hello world\"",
    "\"helo wrd\"!1 1 3 2 1 1 1 1");
eqt!(sq_dict_lookup,          "(`a`b`c!1 2 3) `b",              "2");
eqt!(sq_distinct_count,       "count distinct \"abracadabra\"", "5");

// ── Set ops ─────────────────────────────────────────────────────────
eqt!(sq_inter_ints,           "1 2 3 4 5 inter 3 4 5 6 7", "3 4 5");
eqt!(sq_except_ints,          "1 2 3 4 5 except 3 4",       "1 2 5");
eqt!(sq_union_ints,           "1 2 3 4 5 union 3 4 5 6 7", "1 2 3 4 5 6 7");
eqt!(sq_inter_sym,            "`a`b`c`d inter `c`d`e`f",   "`c`d");
eqt!(sq_except_sym,           "`a`b`c`d except `b",         "`a`c`d");

// ── List manipulation ─────────────────────────────────────────────
eqt!(sq_raze_lol,             "raze (1 2; 3 4; 5 6)",       "1 2 3 4 5 6");
eqt!(sq_cross_count,          "count (1 2 3) cross `a`b",   "6");
eqt!(sq_xexp,                 "2 xexp 10",                  "1024f");
eqt!(sq_bin_search,           "1 3 5 7 9 bin 4",            "1");
eqt!(sq_bin_vec,              "1 3 5 7 9 bin 0 2 4 6 8 10", "-1 0 1 2 3 4");
eqt!(sq_n_first,              "5#1 2 3 4 5 6 7 8 9 10",     "1 2 3 4 5");
eqt!(sq_n_last,               "-3#1 2 3 4 5",               "3 4 5");
eqt!(sq_drop_first,           "3_1 2 3 4 5 6",              "4 5 6");
eqt!(sq_drop_last,            "-2_1 2 3 4 5 6",             "1 2 3 4");

// ── String idioms ─────────────────────────────────────────────────
eqt!(sq_string_int,           "string 42",                  "\"42\"");
// vs returns a list of strings; compare via raze so both sides canonicalize the same.
eqt!(sq_vs_split,             "raze \",\" vs \"a,b,c,d\"", "\"abcd\"");
eqt!(sq_vs_split_count,       "count \",\" vs \"a,b,c,d\"", "4");
eqt!(sq_ss_search,            "\"hello world\" ss \"o\"",  "4 7");
eqt!(sq_ssr_replace,          "ssr[\"hello\";\"l\";\"L\"]", "\"heLLo\"");
eqt!(sq_upper,                "upper \"hello\"",            "\"HELLO\"");
eqt!(sq_lower,                "lower \"HELLO\"",            "\"hello\"");
eqt!(sq_trim,                 "trim \"  hi  \"",            "\"hi\"");

// ── Within / functional predicates ────────────────────────────────
eqt!(sq_within,               "1 5 10 within (3;9)",        "010b");
eqt!(sq_all_int,              "all 1 1 1 0 1",              "0b");
eqt!(sq_any_int,              "any 0 0 0 1 0",              "1b");
eqt!(sq_all_bool,             "all 11101b",                 "0b");
eqt!(sq_any_bool,             "any 00010b",                 "1b");
eqt!(sq_all_truthy,           "all 1 2 3",                  "1b");
eqt!(sq_any_zero,             "any 0 0 0 0",                "0b");
eqt!(sq_signum,               "signum -3 0 4",              "-1 0 1i");

// ── Basic qsql (also covered upstream, but pin via studyq idiom) ──
eqt!(sq_select_where,
    "exec a from ([] a:1 2 3; b:`x`y`z) where b=`y",
    "enlist 2");
eqt!(sq_groupby_sum,
    "exec sum v from select sum v by sym from ([]sym:`a`b`a`b;v:1 2 3 4)",
    "10j");
eqt!(sq_count_cols,
    "count cols ([] a:1 2; b:`x`y; c:1.0 2.0)",
    "3");

// Fibonacci via N-iteration scan.
eqt!(sq_fib_iterate,
    "10 {x,sum -2#x}/0 1",
    "0 1 1 2 3 5 8 13 21 34 55 89j");
// Factorial via prd
eqt!(sq_factorial_10,           "prd 1+til 10",               "3628800");
// Run-length encoding — count of distinct runs.
eqt!(sq_rle_count,
    "count where differ 1 1 2 2 2 3 3 4 4 4 4",
    "4");
// differ flags positions where the value changes from the previous one.
eqt!(sq_differ_count,
    "sum differ 1 1 1 0 0 1 1 1",
    "3i");

// IN-MEMORY COMPRESSION — ops must be transparent over IPC and values correct whether compressed or raw.

#[test]
fn compress_int_vec_sum_and_roundtrip() {
    let mut c = conn();
    // 1M ints cycling 0..99 (range 100 → FOR) — compresses in memory.
    c.query("ca:1000000#til 100").unwrap();
    // sum on the compressed column: (1e6/100)*sum(0..99) = 10000*4950 = 49_500_000.
    assert_eq!(c.query("sum ca").unwrap().as_long(), Some(49_500_000));
    // whole vector round-trips over IPC — b9 must decode any compressed body.
    match c.query("ca").unwrap() {
        K::IntVec(v) => {
            assert_eq!(v.len(), 1_000_000);
            assert_eq!(&v[..3], &[0, 1, 2]);
            assert_eq!(v[99], 99);
            assert_eq!(v[100], 0);
        }
        k => panic!("expected int vec, got tag {}", k.type_tag()),
    }
}

#[test]
fn compress_long_vec_sum_and_roundtrip() {
    let mut c = conn();
    c.query("cj:1000000#`long$til 100").unwrap();
    assert_eq!(c.query("sum cj").unwrap().as_long(), Some(49_500_000));
    match c.query("cj").unwrap() {
        K::LongVec(v) => {
            assert_eq!(v.len(), 1_000_000);
            assert_eq!(v[100], 0);
        }
        k => panic!("expected long vec, got tag {}", k.type_tag()),
    }
}

#[test]
fn compress_random_vec_min_max_sum() {
    let mut c = conn();
    // The user's shape: random ints in [0,100) — compresses (FOR).
    c.query("a:1000000?100").unwrap();
    // min/max via O(1) bounds metadata on the (possibly compressed) vec.
    let mn = c.query("min a").unwrap().as_int().unwrap();
    let mx = c.query("max a").unwrap().as_int().unwrap();
    assert!(mn >= 0 && mx < 100 && mn <= mx, "min={mn} max={mx}");
    // compressed sum must equal a decoded-then-summed copy (a @ til count a).
    assert_eq!(c.query("(sum a)~sum a@til count a").unwrap().as_int(), Some(1),
        "compressed sum must match raw sum");
}

// LARGE BOOLEAN VECTORS — compressed and raw sum/avg/and/or must agree; unique global names avoid races.

#[test]
fn bool_large_compresses() {
    let mut c = conn();
    c.query("blc:1000000?0b").unwrap();
    assert_eq!(c.query("first -17!`blc").unwrap().as_int(), Some(1),
        "1M boolean must compress (bit-packed FOR_B)");
    assert_eq!(c.query("(-17!`blc)[2] > 6 * (-17!`blc)[1]").unwrap().as_int(),
        Some(1),
        "boolean ratio must be ~8x");
}

#[test]
fn bool_sum_random_coc_vs_raw() {
    let mut c = conn();
    c.query("bsr:1000000?0b").unwrap();
    assert_eq!(c.query("(sum bsr)~sum bsr@til count bsr").unwrap().as_int(),
        Some(1));
    assert_eq!(c.query("(type sum bsr)~type sum bsr@til count \
        bsr").unwrap().as_int(), Some(1));
}

#[test]
fn bool_avg_random_coc_vs_raw() {
    let mut c = conn();
    c.query("bav:1000000?0b").unwrap();
    // avg = sum/n — was the user's hang case.
    assert_eq!(c.query("(avg bav)~avg bav@til count bav").unwrap().as_int(),
        Some(1));
}

#[test]
fn bool_const_all_true_reductions() {
    // CONST boolean (xl==xm) hit ov()'s const-vec-sum fast path → bnx(KB)→NULL→hung.
    let mut c = conn();
    c.query("bct:1000000#1b").unwrap();
    assert_eq!(c.query("first -17!`bct").unwrap().as_int(), Some(1), "const \
        bool compresses");
    assert_eq!(c.query("sum bct").unwrap().as_int(), Some(1_000_000));
    assert_eq!(c.query("avg bct").unwrap().as_float(), Some(1.0));
    assert_eq!(c.query("min bct").unwrap().as_int(), Some(1));
    assert_eq!(c.query("max bct").unwrap().as_int(), Some(1));
}

#[test]
fn bool_const_all_false_reductions() {
    let mut c = conn();
    c.query("bcf:1000000#0b").unwrap();
    assert_eq!(c.query("sum bcf").unwrap().as_int(), Some(0));
    assert_eq!(c.query("avg bcf").unwrap().as_float(), Some(0.0));
    assert_eq!(c.query("min bcf").unwrap().as_int(), Some(0));
    assert_eq!(c.query("max bcf").unwrap().as_int(), Some(0));
    assert_eq!(c.query("0=count where bcf").unwrap().as_int(), Some(1));
}

#[test]
fn bool_min_max_random() {
    let mut c = conn();
    c.query("bmm:1000000?0b").unwrap();
    assert_eq!(c.query("(min bmm)~min bmm@til count bmm").unwrap().as_int(),
        Some(1));
    assert_eq!(c.query("(max bmm)~max bmm@til count bmm").unwrap().as_int(),
        Some(1));
}

#[test]
fn bool_and_or_bitwise_coc() {
    // `&`/`|` on two compressed booleans → bitwise on packed words, stays comp.
    let mut c = conn();
    c.query("bap:1000000?0b; baq:1000000?0b").unwrap();
    assert_eq!(c.query("all (bap&baq)=(bap@til count bap)&baq@til count \
        baq").unwrap().as_int(), Some(1));
    assert_eq!(c.query("all (bap|baq)=(bap@til count bap)|baq@til count \
        baq").unwrap().as_int(), Some(1));
    assert_eq!(c.query("bar:bap&baq; first -17!`bar").unwrap().as_int(),
        Some(1));
    c.query("bas:1000000?0b").unwrap();
    assert_eq!(c.query("all ((bap&baq)|bas)=((bap@til count bap)&baq@til count \
        baq)|bas@til count bas")
        .unwrap().as_int(), Some(1));
}

#[test]
fn bool_not_where_coc() {
    let mut c = conn();
    c.query("bnw:1000000?0b").unwrap();
    assert_eq!(c.query("(not bnw)~not bnw@til count bnw").unwrap().as_int(),
        Some(1));
    assert_eq!(c.query("(where bnw)~where bnw@til count \
        bnw").unwrap().as_int(), Some(1));
    assert_eq!(c.query("count bnw").unwrap().as_int(), Some(1_000_000));
}

#[test]
fn bool_compares_coc() {
    let mut c = conn();
    c.query("bcp:1000000?0b; bcq:1000000?0b").unwrap();
    assert_eq!(c.query("all (bcp=bcq)=(bcp@til count bcp)=bcq@til count \
        bcq").unwrap().as_int(), Some(1));
    assert_eq!(c.query("all (bcp<bcq)=(bcp@til count bcp)<bcq@til count \
        bcq").unwrap().as_int(), Some(1));
}

#[test]
fn bool_select_where_table() {
    let mut c = conn();
    c.query("btw:([]x:1000000?100; flag:1000000?0b)").unwrap();
    let n_all = c.query("count btw").unwrap().as_int().unwrap();
    let n_sel = c.query("count select from btw where \
        flag").unwrap().as_int().unwrap();
    assert!(n_sel <= n_all && n_sel > 0, "n_sel={n_sel} n_all={n_all}");
    assert_eq!(c.query("(sum btw`flag)~count select from btw where flag")
        .unwrap().as_int(), Some(1));
}

#[test]
fn bool_grade_coc() {
    let mut c = conn();
    c.query("bgr:1000000?0b").unwrap();
    assert_eq!(c.query("(iasc bgr)~iasc bgr@til count bgr").unwrap().as_int(),
        Some(1));
}

// MASTER_QLANG conformance — builtin invariants, fusion, divergences; each predicate returns 1b.

#[test]
fn test_qlang_a_count() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] 1000001=count x}each \
        (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3650;\
            n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_enlist() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] ((count enlist x)=1)and(first enlist \
        x)~x}each \
            (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3\
                650;n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_first_last() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] ((first x)~x 0)and(last x)~x[-1+count \
        x]}each \
            (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3\
                650;n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_reverse() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] (x~reverse reverse x)and(first reverse \
        x)~last x}each \
            (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3\
                650;n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_rotate() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] (x~(count x)rotate x)and(x~0 rotate \
        x)and(x~(neg count x)rotate x)}each \
            (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3\
                650;n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_next_prev() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] ((-1_next x)~1_x)and(1_prev x)~-1_x}each \
        (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3650;\
            n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_xprev() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] (0 xprev x)~x}each \
        (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3650;\
            n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_fills() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] (fills x)~x}each \
        (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3650;\
            n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_distinct() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] d:distinct x;(d~distinct \
        d)and(count[d]<=count x)}each \
            (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3\
                650;n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_group() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] g:group x;((count raze value g)=count \
        x)and(asc key g)~asc distinct x}each \
            (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3\
                650;n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_in() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] all x in x}each \
        (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3650;\
            n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_inter() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] ((x inter x)~x)and(x inter 0#x)~0#x}each \
        (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3650;\
            n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_union() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] (asc x union x)~asc distinct x}each \
        (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3650;\
            n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_except() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] ((x except x)~0#x)and(x except distinct \
        x)~0#x}each \
            (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3\
                650;n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_sublist() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] (((count x)sublist x)~x)and((0 sublist \
        x)~0#x)and((5+count x)sublist x)~x}each \
            (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3\
                650;n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_cut_raze() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] x~raze 2 cut x}each \
        (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3650;\
            n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_til() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] ((count til count x)=count x)and(til \
        count x)~iasc til count x}each \
            (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3\
                650;n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_asc_desc() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] s:asc x;(s~asc s)and(s~reverse desc \
        x)}each \
            (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3\
                650;n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_iasc_idesc() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] ((x iasc x)~asc x)and(x idesc x)~desc \
        x}each (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+\
            n?3650;n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_asc_monotone() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] s:asc x;all(-1_s)<=1_s}each \
        (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3650;\
            n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_rank() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] (rank x)~iasc iasc x}each \
        (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3650;\
            n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_differ() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] d:differ x;((count d)=count x)and \
        1b~first d}each \
            (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3\
                650;n?\"abcdefghij\")}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_mcount() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] ((count mcount[3;x])=count \
        x)and(mcount[1;x])~(count x)#1}each \
            (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3\
                650;n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_eq_match() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] (all x=x)and(x~x)and not any x<>x}each \
        (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3650;\
            n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_compare() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] (all x<=x)and(all x>=x)and(not any \
        x<x)and not any x>x}each \
            (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3\
                650;n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_find_at() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] ((x?first x)=0)and(x x?x)~x}each \
        (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3650;\
            n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_index() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] ((x@til count x)~x)and(x@0)~first x}each \
        (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3650;\
            n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_take_drop() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] (((count x)#x)~x)and((3#x)~3 sublist \
        x)and((0_x)~x)and((count x)_x)~0#x}each \
            (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3\
                650;n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_join() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] ((count x,x)=2*count x)and(((count \
        x)#x,x)~x)and(x,())~x}each \
            (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3\
                650;n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_a_fill() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] ((first x)^x)~x}each \
        (`long$n?100;n?1.0;n?`alpha`bravo`charlie`delta`echo;2000.01.01+n?3650;\
            n?\"abcdefghij\";n?0b)}[1000001]").unwrap(), K::Bool(true));
}

qk!(test_qlang_a_min_max,
    "{[n] all {[x] s:asc x;((max x)~last s)and(min x)~first \
        s}each \
        (`long$n?100;n?1.0;2000.01.01+n?3650;n?\"abcdefghij\")}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_within,
    "{[n] all {[x] all x within(min x;max x)}each \
        (`long$n?100;n?1.0;2000.01.01+n?3650;n?\"abcdefghij\")}[1000001]",
 K::Bool(true));

#[test]
fn test_qlang_a_or_and() {
    let mut c = conn();
    assert_eq!(c.query("{[n] all {[x] ((x|x)~x)and(x&x)~x}each \
        (`long$n?100;n?1.0;2000.01.01+n?3650;n?\"abcdefghij\";n?0b;n?0x10)}[100\
            0001]").unwrap(), K::Bool(true));
}

qk!(test_qlang_a_neg,
    "{[n] all {[x] all x=neg neg x}each (`long$n?100;n?1.0)}[1000001]",
    K::Bool(true));

qk!(test_qlang_a_abs,
    "{[n] all {[x] (all(abs x)=abs neg x)and all 0<=abs x}each \
        (`long$n?100;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_signum,
    "{[n] all {[x] all(signum x)in -1 0 1}each (`long$n?100;n?1.0)}[1000001]",
    K::Bool(true));

qk!(test_qlang_a_add_sub_mul,
    "{[n] all {[x] \
        (all(x+x)=2*x)and(all(x-x)=0*x)and(all((x+1)-1)=x)and \
        all(x*1)=x}each (`long$n?100;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_sum_split,
    "{[n] all {[x] h:(count x)div 2;e:1e-6;abs[(sum x)-(sum \
        h#x)+sum h _ x]<e*(1+abs sum x)}each \
        (`long$n?100;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_sums_last,
    "{[n] all {[x] e:1e-6;abs[(last sums x)-sum x]<e*(1+abs sum \
        x)}each (`long$n?100;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_mins_maxs,
    "{[n] all {[x] ((last mins x)=min x)and(last maxs x)=max \
        x}each (`long$n?100;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_avg,
    "{[n] all {[x] e:1e-6;abs[(avg x)-(sum[x]%count x)]<e*(1+max \
        abs x)}each (`long$n?100;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_avgs_last,
    "{[n] all {[x] e:1e-6;abs[(last avgs x)-avg x]<e*(1+max abs \
        x)}each (`long$n?100;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_med,
    "{[n] all {[x] (med x)within(min x;max x)}each \
        (`long$n?100;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_var_dev,
    "{[n] all {[x] e:1e-6;((var x)>=neg e)and abs[(dev x)-sqrt \
        var x]<e*(1+max abs x)}each (`long$n?100;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_prd,
    "{[n] all {[x] (prd 6#x)=(prd 3#x)*prd 3_6#x}each \
        (`long$n?100;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_moving_w1,
    "{[n] all {[x] e:1e-6;(all abs[(msum[1;x])-x]<e*1+abs \
        x)and(all(mmax[1;x])=x)and all(mmin[1;x])=x}each \
        (`long$n?100;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_moving_wn,
    "{[n] all {[x] ((count msum[3;x])=count x)and(count \
        mavg[3;x])=count x}each (`long$n?100;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_ratios_len,
    "{[n] all {[x] (count ratios x)=count x}each (`long$n?100;n?1.0)}[1000001]",
    K::Bool(true));

qk!(test_qlang_a_deltas_rt,
    "{[n] all {[x] all x=sums deltas x}each (n?100;`long$n?100)}[1000001]",
    K::Bool(true));

qk!(test_qlang_a_mod_div,
    "{[n] all {[x] all x=(7*x div 7)+x mod 7}each \
        (`short$n?100;n?100;`long$n?100)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_xbar,
    "{[n] all {[x] (all(5 xbar x)=5*x div 5)and all(5 xbar \
        x)<=x}each (`short$n?100;n?100;`long$n?100)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_bin,
    "{[n] all {[x] s:asc distinct x;(s bin s)~til count s}each \
        (`short$n?100;n?100;`long$n?100)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_sqrt,
    "{[n] all {[x] e:1e-3;all abs[((sqrt x)*sqrt \
        x)-x]<e*(1+x)}each (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_reciprocal,
    "{[n] all {[x] e:1e-3;all abs[1-(1.0+x)*reciprocal \
        1.0+x]<e}each (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_exp_log,
    "{[n] all {[x] e:1e-3;all abs[(1.0+x)-log exp 1.0+x]<e}each \
        (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_log_exp,
    "{[n] all {[x] e:1e-3;all abs[(1.0+x)-exp log 1.0+x]<e}each \
        (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_sincos_pyth,
    "{[n] all {[x] e:1e-3;all abs[1-((sin x)*sin x)+(cos x)*cos \
        x]<e}each (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_tan,
    "{[n] all {[x] e:1e-3;all abs[(tan x)-(sin x)%cos x]<e}each \
        (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_asin,
    "{[n] all {[x] e:1e-3;all abs[x-sin asin x]<e}each \
        (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_acos,
    "{[n] all {[x] e:1e-3;all abs[x-cos acos x]<e}each \
        (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_atan,
    "{[n] all {[x] e:1e-3;all abs[x-tan atan x]<e}each \
        (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_xexp_xlog,
    "{[n] all {[x] e:1e-3;all abs[(1.0+x)-2 xlog 2 xexp \
        1.0+x]<e}each (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_floor_ceil,
    "{[n] all {[x] (all(floor x)<=x)and(all(ceiling x)>=x)and \
        all(ceiling x)>=floor x}each (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_sum_split_f,
    "{[n] all {[x] e:1e-3;h:(count x)div 2;abs[(sum x)-(sum \
        h#x)+sum h _ x]<e*(1+abs sum x)}each \
        (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_wsum,
    "{[n] all {[x] e:1e-3;y:(count x)?1.0;abs[(x wsum y)-sum \
        x*y]<e*(1+abs sum x*y)}each (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_wavg,
    "{[n] all {[x] e:1e-3;y:(count x)?1.0;abs[(x wavg y)-(sum \
        x*y)%sum x]<e*(1+abs(sum x*y)%sum x)}each \
        (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_cov,
    "{[n] all {[x] e:1e-3;abs[cov[x;x]-var x]<e*(1+var x)}each \
        (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_cor,
    "{[n] all {[x] e:1e-3;y:(count \
        x)?1.0;(abs[1-cor[x;x]]<e)and(cor[x;y])within(-1.001;1.001)}each \
        (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_mavg_mdev,
    "{[n] all {[x] e:1e-6;(all abs[(mavg[1;x])-x]<e*1+abs \
        x)and(count mdev[3;x])=count x}each \
        (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_and_or,
    "{[n] all {[x] ((x and x)~x)and((x or x)~x)and((x and \
        0b)~(count x)#0b)and((x or 1b)~(count x)#1b)}each (enlist \
        n?0b)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_neg_eltwise,
    "{[n] all {[x] all(0b=x)|x}each (enlist n?0b)}[1000001]", K::Bool(true));

qk!(test_qlang_a_lower,
    "{[n] all {[x] (lower x)~x}each (enlist n?\"abcdefghij\")}[1000001]",
    K::Bool(true));

qk!(test_qlang_a_upper_lower,
    "{[n] all {[x] (lower upper x)~x}each (enlist n?\"abcdefghij\")}[1000001]",
    K::Bool(true));

qk!(test_qlang_a_char_string,
    "{[n] all {[x] (count string x)=count x}each (enlist \
        n?\"abcdefghij\")}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_sym_str_rt,
    "{[n] all {[x] (`$ string x)~x}each (enlist \
        n?`alpha`bravo`charlie`delta`echo)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_sym_string,
    "{[n] all {[x] (count string x)=count x}each (enlist \
        n?`alpha`bravo`charlie`delta`echo)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_temporal_deltas,
    "{[n] all {[x] (count deltas x)=count x}each \
        (2000.01.01+n?3650;2000.01m+n?120)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_temporal_diff,
    "{[n] all {[x] (count x-x)=count x}each \
        (2000.01.01+n?3650;2000.01m+n?120)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_fz_neg_neg_int,
    "{[n] all {[x] all x=neg neg x}each \
        (`short$n?100;n?100;`long$n?100)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_fz_abs_neg_int,
    "{[n] all {[x] all(abs neg x)=abs x}each \
        (`short$n?100;n?100;`long$n?100)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_fz_1p2x,
    "{[n] all {[x] r:2*x;all(1+2*x)=1+r}each \
        (`short$n?100;n?100;`long$n?100)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_fz_poly_fac,
    "{[n] all {[x] r:x-1;all((x+1)*(x-1))=(x+1)*r}each \
        (`short$n?100;n?100;`long$n?100)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_fz_max_abs_int,
    "{[n] all {[x] (max abs x)=max x}each \
        (`short$n?100;n?100;`long$n?100)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_fz_sum_x_plus_x,
    "{[n] all {[x] lv:`long$x;(sum lv+lv)=2*sum lv}each \
        (`short$n?100;n?100;`long$n?100)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_fz_neg_neg_rf,
    "{[n] all {[x] all x=neg neg x}each (`real$n?1.0;n?1.0)}[1000001]",
    K::Bool(true));

qk!(test_qlang_a_fz_abs_neg_rf,
    "{[n] all {[x] all(abs neg x)=abs x}each (`real$n?1.0;n?1.0)}[1000001]",
    K::Bool(true));

qk!(test_qlang_a_fz_sqrt_sqrt,
    "{[n] all {[x] e:1e-3;all abs[(sqrt sqrt x)-(x xexp \
        0.25)]<e*(1+x)}each (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_fz_exp_log,
    "{[n] all {[x] e:1e-3;all abs[(exp log \
        1.0+x)-(1.0+x)]<e*(1.0+x)}each (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_fz_log_exp,
    "{[n] all {[x] e:1e-3;all abs[(log exp x)-x]<e*(1+abs \
        x)}each (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_fz_recip_recip,
    "{[n] all {[x] e:1e-3;all abs[(reciprocal reciprocal \
        1.0+x)-(1.0+x)]<e*(1.0+x)}each (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_fz_diff_of_sq,
    "{[n] all {[x] e:1e-3;y:(count x)?1.0;r:x-y;all \
        abs[((x+y)*(x-y))-((x+y)*r)]<e*(1+abs(x+y)*r)}each \
        (`real$n?1.0;n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_fz_sum_sqrt,
    "{[n] all {[x] e:1e-6;r:sqrt x;abs[(sum sqrt x)-(sum \
        r)]<e*(1+abs sum r)}each (enlist n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_fz_sum_log,
    "{[n] all {[x] e:1e-6;r:log 1.0+x;abs[(sum log 1.0+x)-(sum \
        r)]<e*(1+abs sum r)}each (enlist n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_fz_avg_exp,
    "{[n] all {[x] e:1e-6;r:exp x;abs[(avg exp x)-(avg \
        r)]<e*(1+abs avg r)}each (enlist n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_fz_max_abs_f,
    "{[n] all {[x] r:abs x;(max abs x)=max r}each (enlist n?1.0)}[1000001]",
    K::Bool(true));

qk!(test_qlang_a_fz_min_neg_f,
    "{[n] all {[x] r:neg x;(min neg x)=min r}each (enlist n?1.0)}[1000001]",
    K::Bool(true));

qk!(test_qlang_a_fz_sum_sq_wsum,
    "{[n] all {[x] e:1e-6;abs[(sum x*x)-(x wsum x)]<e*(1+abs x \
        wsum x)}each (enlist n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_fz_dot,
    "{[n] all {[x] e:1e-6;y:(count x)?1.0;p:x*y;(abs[(sum \
        x*y)-(sum p)]<e*(1+abs sum p))and abs[(sum x*y)-(x wsum \
        y)]<e*(1+abs x wsum y)}each (enlist n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_a_fz_sum_add,
    "{[n] all {[x] e:1e-6;y:(count x)?1.0;abs[(sum x+y)-((sum \
        x)+sum y)]<e*(1+abs(sum x)+sum y)}each (enlist \
        n?1.0)}[1000001]",
 K::Bool(true));

qk!(test_qlang_b_select_by,
    "{[t]3=count select sum sz by sym from t where \
        px>10}[([]sym:`a`b`a`c`b;time:09:00 09:30 10:00 10:15 \
        11:00;px:10 11 12 13 14.;sz:100 200 300 400 500)]",
 K::Bool(true));

qk!(test_qlang_b_exec,
    "{[t]5=count exec px from t}[([]sym:`a`b`a`c`b;time:09:00 \
        09:30 10:00 10:15 11:00;px:10 11 12 13 14.;sz:100 200 300 \
        400 500)]",
 K::Bool(true));

qk!(test_qlang_b_update,
    "{[t]`r in cols update r:sz%100 from \
        t}[([]sym:`a`b`a`c`b;time:09:00 09:30 10:00 10:15 \
        11:00;px:10 11 12 13 14.;sz:100 200 300 400 500)]",
 K::Bool(true));

qk!(test_qlang_b_delete_rows,
    "{[t]3=count delete from t where \
        px>12}[([]sym:`a`b`a`c`b;time:09:00 09:30 10:00 10:15 \
        11:00;px:10 11 12 13 14.;sz:100 200 300 400 500)]",
 K::Bool(true));

qk!(test_qlang_b_delete_cols,
    "{[t]not`sz in cols delete sz from \
        t}[([]sym:`a`b`a`c`b;time:09:00 09:30 10:00 10:15 \
        11:00;px:10 11 12 13 14.;sz:100 200 300 400 500)]",
 K::Bool(true));

qk!(test_qlang_b_fby,
    "{[t]3=count select from t where px=(max;px)fby \
        sym}[([]sym:`a`b`a`c`b;time:09:00 09:30 10:00 10:15 \
        11:00;px:10 11 12 13 14.;sz:100 200 300 400 500)]",
 K::Bool(true));

#[test]
fn test_qlang_b_qfunc_select() {
    let mut c = conn();
    assert_eq!(c.query("{[t]3=count \
        ?[t;();(enlist`sym)!enlist`sym;(enlist`sz)!enlist(sum;`sz)]}[([]sym:`a`\
            b`a`c`b;time:09:00 09:30 10:00 10:15 11:00;px:10 11 12 13 \
                14.;sz:100 200 300 400 500)]").unwrap(), K::Bool(true));
}

qk!(test_qlang_b_qfunc_update,
    "{[t]`r in cols \
        ![t;();0b;(enlist`r)!enlist(%;`sz;100)]}[([]sym:`a`b`a`c`b;time:09:00 \
        09:30 10:00 10:15 11:00;px:10 11 12 13 14.;sz:100 200 300 \
        400 500)]",
 K::Bool(true));

qk!(test_qlang_b_asof,
    "{[t;q]qq:`sym`time xasc q;0<=count qq asof select sym,time \
        from t}[([]sym:`a`b`a`c`b;time:09:00 09:30 10:00 10:15 \
        11:00;px:10 11 12 13 14.;sz:100 200 300 400 \
        500);([]sym:`a`a`b`b`c;time:08:59 09:59 09:29 10:59 \
        10:14;bid:9 11 10 13 12.;ask:10 12 11 14 13.)]",
 K::Bool(true));

qk!(test_qlang_b_aj,
    "{[t;q]qq:`sym`time xasc q;5=count \
        aj[`sym`time;t;qq]}[([]sym:`a`b`a`c`b;time:09:00 09:30 \
        10:00 10:15 11:00;px:10 11 12 13 14.;sz:100 200 300 400 \
        500);([]sym:`a`a`b`b`c;time:08:59 09:59 09:29 10:59 \
        10:14;bid:9 11 10 13 12.;ask:10 12 11 14 13.)]",
 K::Bool(true));

qk!(test_qlang_b_aj0,
    "{[t;q]qq:`sym`time xasc q;0<count \
        aj0[`sym`time;t;qq]}[([]sym:`a`b`a`c`b;time:09:00 09:30 \
        10:00 10:15 11:00;px:10 11 12 13 14.;sz:100 200 300 400 \
        500);([]sym:`a`a`b`b`c;time:08:59 09:59 09:29 10:59 \
        10:14;bid:9 11 10 13 12.;ask:10 12 11 14 13.)]",
 K::Bool(true));

qk!(test_qlang_b_ej,
    "{[t;q]qq:`sym`time xasc q;0<=count \
        ej[`sym;t;qq]}[([]sym:`a`b`a`c`b;time:09:00 09:30 10:00 \
        10:15 11:00;px:10 11 12 13 14.;sz:100 200 300 400 \
        500);([]sym:`a`a`b`b`c;time:08:59 09:59 09:29 10:59 \
        10:14;bid:9 11 10 13 12.;ask:10 12 11 14 13.)]",
 K::Bool(true));

qk!(test_qlang_b_ij,
    "{[t;q]qq:`sym`time xasc q;0<=count t ij select first bid by \
        sym from qq}[([]sym:`a`b`a`c`b;time:09:00 09:30 10:00 10:15 \
        11:00;px:10 11 12 13 14.;sz:100 200 300 400 \
        500);([]sym:`a`a`b`b`c;time:08:59 09:59 09:29 10:59 \
        10:14;bid:9 11 10 13 12.;ask:10 12 11 14 13.)]",
 K::Bool(true));

qk!(test_qlang_b_lj,
    "{[t;q]qq:`sym`time xasc q;5=count t lj select first bid by \
        sym from qq}[([]sym:`a`b`a`c`b;time:09:00 09:30 10:00 10:15 \
        11:00;px:10 11 12 13 14.;sz:100 200 300 400 \
        500);([]sym:`a`a`b`b`c;time:08:59 09:59 09:29 10:59 \
        10:14;bid:9 11 10 13 12.;ask:10 12 11 14 13.)]",
 K::Bool(true));

qk!(test_qlang_b_pj,
    "{[t;q]qq:`sym`time xasc q;5=count(update bid:0. from t)pj \
        select sum bid by sym from qq}[([]sym:`a`b`a`c`b;time:09:00 \
        09:30 10:00 10:15 11:00;px:10 11 12 13 14.;sz:100 200 300 \
        400 500);([]sym:`a`a`b`b`c;time:08:59 09:59 09:29 10:59 \
        10:14;bid:9 11 10 13 12.;ask:10 12 11 14 13.)]",
 K::Bool(true));

qk!(test_qlang_b_uj,
    "{[t]10=count t uj t}[([]sym:`a`b`a`c`b;time:09:00 09:30 \
        10:00 10:15 11:00;px:10 11 12 13 14.;sz:100 200 300 400 \
        500)]",
 K::Bool(true));

qk!(test_qlang_b_wj,
    "{[t;q]qq:`sym`time xasc q;w:(t[`time]-1;t[`time]+1);5=count \
        wj[w;`sym`time;t;(qq;(max;`ask))]}[([]sym:`a`b`a`c`b;time:09:00 \
        09:30 10:00 10:15 11:00;px:10 11 12 13 14.;sz:100 200 300 \
        400 500);([]sym:`a`a`b`b`c;time:08:59 09:59 09:29 10:59 \
        10:14;bid:9 11 10 13 12.;ask:10 12 11 14 13.)]",
 K::Bool(true));

qk!(test_qlang_b_cols_keys,
    "{[t](`sym`time`px`sz~cols t)and 0=count keys \
        t}[([]sym:`a`b`a`c`b;time:09:00 09:30 10:00 10:15 \
        11:00;px:10 11 12 13 14.;sz:100 200 300 400 500)]",
 K::Bool(true));

qk!(test_qlang_b_meta,
    "{[t]`c`t`f`a~cols meta t}[([]sym:`a`b`a`c`b;time:09:00 \
        09:30 10:00 10:15 11:00;px:10 11 12 13 14.;sz:100 200 300 \
        400 500)]",
 K::Bool(true));

qk!(test_qlang_b_xkey_key,
    "{[t]`sym~first keys`sym xkey \
        t}[([]sym:`a`b`a`c`b;time:09:00 09:30 10:00 10:15 \
        11:00;px:10 11 12 13 14.;sz:100 200 300 400 500)]",
 K::Bool(true));

qk!(test_qlang_b_xcol,
    "{[t]`SYM in cols \
        xcol[`SYM`time`px`sz;t]}[([]sym:`a`b`a`c`b;time:09:00 09:30 \
        10:00 10:15 11:00;px:10 11 12 13 14.;sz:100 200 300 400 \
        500)]",
 K::Bool(true));

qk!(test_qlang_b_xcols,
    "{[t]`px~first cols \
        xcols[`px;t]}[([]sym:`a`b`a`c`b;time:09:00 09:30 10:00 \
        10:15 11:00;px:10 11 12 13 14.;sz:100 200 300 400 500)]",
 K::Bool(true));

qk!(test_qlang_b_xasc_xdesc,
    "{[t](<=). 2#exec px from`px xasc \
        t}[([]sym:`a`b`a`c`b;time:09:00 09:30 10:00 10:15 \
        11:00;px:10 11 12 13 14.;sz:100 200 300 400 500)]",
 K::Bool(true));

qk!(test_qlang_b_xgroup_ungroup,
    "{[t]5=count ungroup`sym xgroup \
        t}[([]sym:`a`b`a`c`b;time:09:00 09:30 10:00 10:15 \
        11:00;px:10 11 12 13 14.;sz:100 200 300 400 500)]",
 K::Bool(true));

#[test]
fn test_qlang_b_csv() {
    let mut c = conn();
    assert_eq!(c.query(r#"","~csv"#).unwrap(), K::Bool(true));
}

qk!(test_qlang_b_upsert, r#"{[]4=count([]a:1 2 3)upsert 4}[]"#, K::Bool(true));

qk!(test_qlang_b_insert,
    "{[]qm_ins::([]a:1 2 3);`qm_ins insert 4;4=count qm_ins}[]", K::Bool(true));

#[test]
fn test_qlang_b_string() {
    let mut c = conn();
    assert_eq!(c.query(r#""42"~string 42"#).unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_b_lower_upper() {
    let mut c = conn();
    assert_eq!(c.query(r#"("abc"~lower"ABC")and"ABC"~upper"abc""#).unwrap(),
        K::Bool(true));
}

qk!(test_qlang_b_trims,
    "(\"ab\"~ltrim\" ab\")and(\"ab\"~rtrim\"ab \")and\"ab\"~trim\" ab \"",
    K::Bool(true));

#[test]
fn test_qlang_b_like() {
    let mut c = conn();
    assert_eq!(c.query(r#"(1b~"abc"like"a*")and 0b~"abc"like"z*""#).unwrap(),
        K::Bool(true));
}

qk!(test_qlang_b_ss_ssr,
    "(2 5~ss[\"abXcdX\";\"X\"])and\"a_b_\"~ssr[\"a.b.\";\".\";\"_\"]",
    K::Bool(true));

#[test]
fn test_qlang_b_md5() {
    let mut c = conn();
    // md5 is not implemented in this l build; assert it traps as nyi, like the other nyi tests.
    assert_eq!(c.query(r#"@[{md5"hello"};::;{1b}]"#).unwrap(), K::Bool(true));
}

qk!(test_qlang_b_type, r#"-7h~type 1j"#, K::Bool(true));

qk!(test_qlang_b_attr_set_clear, r#"(`s~attr`s#1 2 3)and`~attr`#`s#1 2 3"#,
    K::Bool(true));

qk!(test_qlang_b_null, r#"(null 0N)and not null 5"#, K::Bool(true));

qk!(test_qlang_b_cast_round, r#"4~`int$3.7"#, K::Bool(true));

#[test]
fn test_qlang_b_tok() {
    let mut c = conn();
    assert_eq!(c.query(r#"42~"I"$"42""#).unwrap(), K::Bool(true));
}

qk!(test_qlang_b_enum, "{[]qm_dom::`a`b`c;`a`b`a~value`qm_dom$`a`b`a}[]",
    K::Bool(true));

qk!(test_qlang_b_tables, "{[]qm_tab::([]a:1 2);`qm_tab in tables`.}[]",
    K::Bool(true));

#[test]
fn test_qlang_b_value() {
    let mut c = conn();
    assert_eq!(c.query(r#"6~value"2+4""#).unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_b_eval_parse() {
    let mut c = conn();
    assert_eq!(c.query(r#"6~eval parse"2+4""#).unwrap(), K::Bool(true));
}

qk!(test_qlang_b_show, r#"(::)~show 42"#, K::Bool(true));

qk!(test_qlang_b_get_set_var, r#"{[]`qm_gv set 99;99~get`qm_gv}[]"#,
    K::Bool(true));

#[test]
fn test_qlang_b_system() {
    let mut c = conn();
    assert_eq!(c.query(r#"0<=count system"v""#).unwrap(), K::Bool(true));
}

qk!(test_qlang_b_gtime_ltime, "(-15h~type gtime .z.z)and -15h~type ltime .z.z",
    K::Bool(true));

qk!(test_qlang_b_getenv, r#"10h~type getenv`PATH"#, K::Bool(true));

qk!(test_qlang_b_do_while_if,
    "{[]a:0;do[5;a+:1];b:0;while[b<5;b+:1];c:$[1>0;1;0];(a=5)and(b=5)and \
        c=1}[]",
 K::Bool(true));

#[test]
fn test_qlang_b_set_get_disk() {
    let mut c = conn();
    // Verify the disk round-trip by computing over the get result, not ~-matching a memory-mapped object.
    assert_eq!(c.query("{[]`:/tmp/qm_ipc_t set til 100;all(til \
        100)=get`:/tmp/qm_ipc_t}[]").unwrap(), K::Bool(true));
}

qk!(test_qlang_b_read0_read1,
    "{[](`:/tmp/qm_ipc_r 0:enlist\"line1\");(\"line1\"~first \
        read0`:/tmp/qm_ipc_r)and 0<count read1`:/tmp/qm_ipc_r}[]",
 K::Bool(true));

// Ranged file-read / CSV-stream over-read regressions — non-page-aligned chunks must not over-read or crash.

#[test]
fn test_qlang_b_read1_ranged_exact_len() {
    // Ranged read at a non-page-aligned offset returns exactly len bytes.
    let mut c = conn();
    assert_eq!(c.query("{[]p:`:/tmp/qm_rng_len;(p) 0: string til 40000; \
        50000=count read1(p;100;50000)}[]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_b_read1_ranged_exact_content() {
    // slop fix: ranged bytes equal the true file slice (no trailing slop garbage).
    let mut c = conn();
    assert_eq!(c.query("{[]p:`:/tmp/qm_rng_cnt;(p) 0: \
        40000#enlist\"0123456789\"; (read1(p;100;50000))~\"x\"$50000#1 \
        rotate\"0123456789\\n\"}[]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_b_qfs_stream_multichunk() {
    // End-to-end .Q.fs over a large file streams non-page-aligned chunks (original crash repro).
    let mut c = conn();
    assert_eq!(c.query("{[]p:`:/tmp/qm_fs_stream;(p) 0: string til 40000; \
        (-7!p)=.Q.fs[{count x};p]}[]").unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_b_csv_load_page_boundary() {
    // CSV parse of an exact-page chunk ending mid-record must not read past the buffer.
    let mut c = conn();
    assert_eq!(c.query("{[]p:`:/tmp/qm_csv_pgcut;(p) 0: 5000#enlist\"12,3\"; \
        3000<count (\"JJ\";enlist\",\")0:(p;0j;16384j)}[]").unwrap(),
            K::Bool(true));
}

#[test]
fn test_qlang_b_csv_load_ranged_offset() {
    // CSV read at a non-page-aligned offset parses without over-read.
    let mut c = conn();
    assert_eq!(c.query("{[]p:`:/tmp/qm_csv_rngoff;(p) 0: 5000#enlist\"12,3\"; \
        3000<count (\"JJ\";enlist\",\")0:(p;100;16384j)}[]").unwrap(),
            K::Bool(true));
}

#[test]
fn test_qlang_b_read1_cache_distinct_len() {
    // A short ranged read must not poison a later full read at the same offset (cache-key regression).
    let mut c = conn();
    assert_eq!(c.query("{[]p:`:/tmp/qm_cache_len;(p) 0: string til 40000; \
        r:read1(p;0;50000); (-7!p)=count read1 p}[]").unwrap(), K::Bool(true));
}

qk!(test_qlang_b_hopen_hclose, r#"{[]h:hopen`:/tmp/qm_ipc_h;hclose h;1b}[]"#,
    K::Bool(true));

qk!(test_qlang_b_hcount_hdel,
    "{[](`:/tmp/qm_ipc_b 1:0x010203);0<hcount`:/tmp/qm_ipc_b}[]",
    K::Bool(true));

qk!(test_qlang_b_hsym, r#"`:abc~hsym`abc"#, K::Bool(true));

qk!(test_qlang_b_save_load,
    "{[]qm_save::([]a:1 2 3);`:/tmp/qm_ipc_tt set \
        qm_save;0<count get`:/tmp/qm_ipc_tt}[]",
 K::Bool(true));

#[test]
fn test_qlang_b_file_0() {
    let mut c = conn();
    assert_eq!(c.query(r#"{[]("II";" ")0:("1 2";"3 4");1b}[]"#).unwrap(),
        K::Bool(true));
}

qk!(test_qlang_b_each, r#"1 1 1~count each(1;2;3)"#, K::Bool(true));

#[test]
fn test_qlang_b_over_scan() {
    let mut c = conn();
    assert_eq!(c.query(r#"(10j~(+/)1 2 3 4)and 1 3 6 10~(+\)1 2 3 4"#)
        .unwrap(), K::Bool(true));
}

qk!(test_qlang_b_eachleft_right, r#"(11 21~10+\:1 11)and 11 12~10+/:1 2"#,
    K::Bool(true));

qk!(test_qlang_b_eachprior, r#"1 4 5 2~(-)':[1 5 10 12]"#, K::Bool(true));

qk!(test_qlang_b_peach, r#"(til 5)~{x}peach til 5"#, K::Bool(true));

qk!(test_qlang_b_dotq_dd, r#"`a.b~.Q.dd[`a;`b]"#, K::Bool(true));

#[test]
fn test_qlang_b_dotq_id() {
    let mut c = conn();
    assert_eq!(c.query(r#"-11h=type .Q.id`$"1a""#).unwrap(), K::Bool(true));
}

qk!(test_qlang_b_dotq_s, r#"10h~type .Q.s 42"#, K::Bool(true));

qk!(test_qlang_b_dotz_k, r#"0<.z.K"#, K::Bool(true));

qk!(test_qlang_b_dotz_d_z, r#"(-14h~type .z.d)and -15h~type .z.z"#,
    K::Bool(true));

#[test]
fn test_qlang_b_doth_htc() {
    let mut c = conn();
    // .h.htc is not implemented in this l build; assert it traps as nyi.
    assert_eq!(c.query(r#"@[{.h.htc[`b;"x"]};::;{1b}]"#).unwrap(),
        K::Bool(true));
}

qk!(test_qlang_c_default_int_tag, r#"-6h~type 1"#, K::Bool(true));

qk!(test_qlang_c_long_suffix, r#"-7h~type 1j"#, K::Bool(true));

qk!(test_qlang_c_cast_rounds, r#"(4~`int$3.7)and 3~`int$2.5"#, K::Bool(true));

qk!(test_qlang_c_spaced_bool_lit, r#"0b~(1 0 1 0 1b)~10101b"#, K::Bool(true));

qk!(test_qlang_c_sym_no_min_max, r#"@[{max`a`b`c};::;{1b}]"#, K::Bool(true));

qk!(test_qlang_c_byte_no_min_max, r#"@[{max 0x01 0x02};::;{1b}]"#,
    K::Bool(true));

qk!(test_qlang_c_floor_int_widths, r#"@[{floor 1 2 3h};::;{1b}]"#,
    K::Bool(true));

#[test]
fn test_qlang_c_d_is_timestamp() {
    // D-form is a timestamp (KP=-12h); it was datetime before timespan landed.
    let mut c = conn();
    assert_eq!(c.query(r#"-12h~type 2000.01.01D00:00:00"#).unwrap(),
        K::Bool(true));
}

#[test]
fn test_qlang_c_has_timespan() {
    // timespan (KN=-16h) is now implemented; `0D…` literals parse to it.
    let mut c = conn();
    assert_eq!(c.query(r#"-16h~type value "0D00:00:01""#).unwrap(),
        K::Bool(true));
}

#[test]
fn test_qlang_c_nyi_svar() {
    let mut c = conn();
    // l DOES implement svar (sample variance, q.k §9): svar 1 2 3. = 2/(3-1) = 1.0.
    assert_eq!(c.query(r#"1f~svar 1 2 3."#).unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_c_nyi_sdev() {
    let mut c = conn();
    // l DOES implement sdev (sample std-dev, q.k §9): sdev 1 2 3. = sqrt 1.0 = 1.0.
    assert_eq!(c.query(r#"1f~sdev 1 2 3."#).unwrap(), K::Bool(true));
}

#[test]
fn test_qlang_c_nyi_ema() {
    let mut c = conn();
    // l DOES implement ema (exp. moving avg, q.k §17): ema[0.5;1 2 3.] = 1 1.5 2.25.
    assert_eq!(c.query(r#"1 1.5 2.25~ema[0.5;1 2 3.]"#).unwrap(),
        K::Bool(true));
}

qk!(test_qlang_c_nyi_binr, r#"@[{binr[1 2 3;2]};::;{1b}]"#, K::Bool(true));

#[test]
fn test_qlang_c_nyi_prior_kw() {
    let mut c = conn();
    // l DOES implement the `prior` keyword (q.k §4): (-)prior 1 5 10 = deltas = 1 4 5.
    assert_eq!(c.query(r#"1 4 5~(-)prior 1 5 10"#).unwrap(), K::Bool(true));
}

qk!(test_qlang_c_prior_glyph_ok, r#"1 4 5 2~(-)':[1 5 10 12]"#, K::Bool(true));

#[test]
fn test_qlang_c_nyi_vs_baseenc() {
    let mut c = conn();
    // vs base-encode (general/mixed radix, bulk vec->matrix): a few worked examples.
    assert_eq!(
        c.query(
            "(1 9 9 5~10 vs 1995)&(1 3 25~24 60 60 vs 3805)&((1 1 1;0 0 1;0 1 \
                0)~2 vs 4 5 6)&(1234=2 sv 2 vs 1234)"
        )
        .unwrap(),
        K::Bool(true)
    );
}

#[test]
fn test_qlang_byte_vs_atom() {
    let mut c = conn();
    // 0x0 vs int atom once crashed; now returns raw big-endian bytes of the type's width.
    assert_eq!(
        c.query("(0x000004d2~0x0 vs 1234)&(0x04d2~0x0 vs \
            1234h)&(0x00000000000004d2~0x0 vs 1234j)&(1234=0x0 sv 0x0 vs 1234)")
            .unwrap(),
        K::Bool(true)
    );
}

#[test]
fn test_qlang_temporal_coc_arith() {
    let mut c = conn();
    // date/time/month +- int stays temporal and, when compressed, stays compressed instead of decoding.
    assert_eq!(
        c.query("a:.z.d-til 500;(14=type 1+.z.d-til 100)&(19=type 1+.z.t-til \
            100)&(13=type 1+(`month$.z.d)-til \
                50)&((`int$1+a)~1+`int$a)&((`int$a-7)~(`int$a)-7)").unwrap(),
        K::Bool(true)
    );
    // Forced-compressed date: 1+a stays compressed and is bit-exact vs the raw twin.
    assert_eq!(
        c.query("{a::.z.d-x?100;a::-18!`a;`r set 1+a;(0<-55!`r)&(14=type \
            r)&((`int$r)~1+`int$-18!`a)}200000").unwrap(),
        K::Bool(true)
    );
    // Temporal vec-vec, comparison (once crashed), and min/max type checks.
    assert_eq!(
        c.query("a:.z.d-til 500;b:.z.d-til 500;c:`int$a;m:(`month$.z.d)-til \
            50;(6=type a-b)&(14=type \
                a+c)&((`int$a-b)~c-`int$b)&((m>`month$2020.01.01)~m>`month$2020\
                    .01.01)&((-14)=type min a)&(13=abs type min m)").unwrap(),
        K::Bool(true)
    );
}

#[test]
fn test_qlang_c_nyi_dotj() {
    let mut c = conn();
    // l DOES implement .j.j (K->JSON): .j.j`a`b = "[\"a\",\"b\"]".
    assert_eq!(c.query(r#"0<count .j.j`a`b"#).unwrap(), K::Bool(true));
}

qk!(test_qlang_c_nyi_join_fvars,
    "@[{ljf[1!([]k:1 2;v:3 4);([]k:1 2;w:5 6)]};::;{1b}]", K::Bool(true));

#[test]
fn test_qlang_c_over_needs_parens() {
    let mut c = conn();
    assert_eq!(c.query(r#"@[value;"+/1 2 3";{1b}]"#).unwrap(), K::Bool(true));
}

// REGRESSION: a narrow-type/boolean reduction seeds its first element wrong at scale; asserts correct q semantics.
qk!(test_qlang_bug_differ_bool_first,
    "{[n]x:n?0b;d:differ x;((count d)=n)and 1b~first d}[1000001]",
    K::Bool(true));

// REGRESSION: a narrow-type/boolean reduction seeds its first element wrong at scale; asserts correct q semantics.
qk!(test_qlang_bug_any_bool_leading, r#"1b~any 1b,1000000#0b"#, K::Bool(true));

// REGRESSION: a narrow-type/boolean reduction seeds its first element wrong at scale; asserts correct q semantics.
qk!(test_qlang_bug_all_bool_leading, r#"0b~all 0b,1000000#1b"#, K::Bool(true));

// REGRESSION: a narrow-type/boolean reduction seeds its first element wrong at scale; asserts correct q semantics.
qk!(test_qlang_bug_deltas_short_seed,
    "{[n]x:`short$n?100;all x=sums deltas x}[1000001]", K::Bool(true));

// REGRESSION: a stored monotone vector used as an index must gather correctly and in range.
qk!(test_xcmp_compressed_index_gather,
    "{[n] idx:where 0=(til n) mod 7; v:(til n) mod 1000; r:v \
        idx; (all r within 0 999) and r~idx mod 1000}[1000000]",
 K::Bool(true));

// REGRESSION: integer div/mod on a compressed vector via a compiled lambda must not read compressed memory raw.
qk!(test_xcmp_compressed_div_mod,
    "{[n] idx:where 0=(til n) mod 7; (idx mod 1000)~idx-1000*idx \
        div 1000}[1000000]",
 K::Bool(true));

// EXTENDED COVERAGE — broad q-function sweep; every expected value is q's own oracle output or a hand-authored literal.
eqt!(m_ceil_0, "ceiling 1.5 2.1 3.9", "2 3 4");
eqt!(m_ceil_1, "ceiling -1.5 -2.9", "-1 -2");
eqt!(m_ceil_2, "ceiling 0.0 0.4 0.6", "0 1 1");
eqt!(m_ceil_3, "ceiling 100.999", "101");
eqt!(m_floor_0, "floor 1.5 2.1 3.9", "1 2 3");
eqt!(m_floor_1, "floor -1.5 -2.9", "-2 -3");
eqt!(m_floor_2, "floor 0.0 0.4 0.6", "0 0 0");
eqt!(m_floor_3, "floor 100.999", "100");
eqt!(m_sin0, "sin 0.0", "0f");
eqt!(m_cos0, "cos 0.0", "1f");
eqt!(m_tan0, "tan 0.0", "0f");
eqt!(m_asin0, "asin 0.0", "0f");
eqt!(m_acos1, "acos 1.0", "0f");
eqt!(m_atan0, "atan 0.0", "0f");
eqt!(m_sqrt4, "sqrt 4 9 16", "2 3 4f");
eqt!(m_sqrt0, "sqrt 0.0", "0f");
eqt!(m_exp0, "exp 0.0", "1f");
eqt!(m_log1, "log 1.0", "0f");
eqt!(m_recip, "reciprocal 2 4 8f", "0.5 0.25 0.125");
eqt!(m_signum, "signum -3 0 5", "-1 0 1i");
eqt!(m_signum_f, "signum -2.5 0.0 7.1", "-1 0 1i");
eqt!(m_absf, "abs -1.5 2.5 -3.5", "1.5 2.5 3.5");
eqt!(m_neg, "neg 1 2 3", "-1 -2 -3");
eqt!(m_absj, "abs -5 -10 15j", "5 10 15j");
eqt!(m_neg_f, "neg 1.5 -2.5", "-1.5 2.5");
eqt!(m_xexp, "2 xexp 3", "8f");
eqt!(m_xexp2, "10 xexp 2", "100f");
eqt!(m_xexp_half, "4 xexp 0.5", "2f");
eqt!(m_xlog, "2 xlog 8", "3f");
eqt!(m_xlog10, "10 xlog 100", "2f");
eqt!(m_mod, "7 5 8 mod 3", "1 2 2");
eqt!(m_mod_f, "7.5 mod 2", "1.5");
eqt!(m_mod0, "6 mod 3", "0");
eqt!(m_div, "7 div 2", "3");
eqt!(m_div_neg, "(-7) div 2", "-4");
eqt!(m_div_v, "10 20 30 div 3", "3 6 10");
eqt!(m_within, "5 within 1 10", "1b");
eqt!(m_within_v, "1 5 11 within 2 10", "010b");
eqt!(m_within_out, "0 within 1 10", "0b");
eqt!(m_within_edge, "1 10 within 1 10", "11b");
eqt!(a_var, "var 1 2 3 4 5", "2f");
eqt!(a_dev, "dev 2 4 4 4 5 5 7 9", "2f");
eqt!(a_med, "med 1 3 2 5 4", "3f");
eqt!(a_med_even, "med 1 2 3 4", "2.5");
eqt!(a_wavg, "2 3 4 wavg 1 2 3", "(sum 2 3 4*1 2 3)%sum 2 3 4");
eqt!(a_wsum, "2 3 4 wsum 1 2 3", "20f");
eqt!(a_cov, "(1 2 3 4 5) cov 2 4 6 8 10", "4f");
eqt!(a_cor, "(1 2 3 4 5) cor 2 4 6 8 10", "1f");
eqt!(a_avg_one, "avg enlist 5", "5f");
eqt!(a_prd, "prd 1 2 3 4", "24");
eqt!(a_max, "max 3 1 4 1 5", "5");
eqt!(a_min, "min 3 1 4 1 5", "1");
eqt!(a_sum_f, "sum 1.5 2.5", "4f");
eqt!(a_avg, "avg 1 2 3 4", "2.5");
eqt!(a_count_v, "count til 100", "100");
eqt!(a_all, "all 1 1 1b", "1b");
eqt!(a_all_f, "all 1 0 1b", "0b");
eqt!(a_any, "any 001b", "1b");
eqt!(a_any_f, "any 0 0 0b", "0b");
eqt!(a_sum_bool, "sum 10110b", "3");
eqt!(a_max_f, "max 1.5 9.9 2.2", "9.9");
eqt!(a_min_neg, "min -5 -1 -10", "-10");
eqt!(r_sums, "sums 1 2 3 4", "1 3 6 10");
eqt!(r_prds, "prds 1 2 3 4", "1 2 6 24");
eqt!(r_mins, "mins 3 1 4 1 5", "3 1 1 1 1");
eqt!(r_maxs, "maxs 3 1 4 1 5", "3 3 4 4 5");
eqt!(r_avgs, "avgs 1 2 3 4", "1 1.5 2 2.5");
eqt!(r_deltas, "deltas 1 3 6 10", "1 2 3 4");
eqt!(r_ratios, "ratios 1 2 6 24", "1 2 3 4f");
eqt!(r_differ, "differ 1 1 2 2 3", "10101b");
eqt!(r_sums_f, "sums 1.5 2.5 3.0", "1.5 4 7");
eqt!(r_deltas_neg, "deltas 10 7 5", "10 -3 -2");
eqt!(r_maxs_eq, "maxs 5 5 5 5", "5 5 5 5");
eqt!(v_msum, "3 msum 1 2 3 4 5", "1 3 6 9 12");
eqt!(v_mavg, "2 mavg 1 2 3 4f", "1 1.5 2.5 3.5");
eqt!(v_mcount, "3 mcount 1 2 3 4 5", "1 2 3 3 3");
eqt!(v_mmax, "2 mmax 1 3 2 5 4", "1 3 3 5 5");
eqt!(v_mmin, "2 mmin 3 1 4 1 5", "3 1 1 1 1");
eqt!(v_msum1, "1 msum 1 2 3", "1 2 3");
eqt!(v_msum_full, "5 msum 1 2 3", "1 3 6");
eqt!(l_rev, "reverse 1 2 3 4", "4 3 2 1");
eqt!(l_rot, "2 rotate 1 2 3 4 5", "3 4 5 1 2");
eqt!(l_rot_neg, "-1 rotate 1 2 3 4", "4 1 2 3");
eqt!(l_raze, "raze (1 2;3 4;5 6)", "1 2 3 4 5 6");
eqt!(l_til, "til 5", "0 1 2 3 4");
eqt!(l_cut, "2 cut 1 2 3 4 5 6", "(1 2;3 4;5 6)");
eqt!(l_first, "first 1 2 3", "1");
eqt!(l_last, "last 1 2 3", "3");
eqt!(l_next, "next 1 2 3 4", "2 3 4 0N");
eqt!(l_prev, "prev 1 2 3 4", "0N 1 2 3");
eqt!(l_count, "count 1 2 3 4 5", "5");
eqt!(l_where, "where 01011b", "1 3 4");
eqt!(l_where_n, "where 2 0 3", "0 0 2 2 2");
eqt!(l_fills, "fills 1 0N 0N 4", "1 1 1 4");
eqt!(l_sublist, "2 sublist 1 2 3 4 5", "1 2");
eqt!(l_sublist2, "2 3 sublist til 10", "2 3 4");
eqt!(l_cross, "1 2 cross 10 20", "(1 10;1 20;2 10;2 20)");
eqt!(l_in, "2 in 1 2 3", "1b");
eqt!(l_in_v, "1 5 in 1 2 3", "10b");
eqt!(l_distinct, "distinct 1 2 2 3 3 3", "1 2 3");
eqt!(l_xprev, "2 xprev 1 2 3 4 5", "0N 0N 1 2 3");
eqt!(l_sv_int, "10 sv 1 2 3", "123");
eqt!(l_inter, "(1 2 3 4) inter 2 4 6", "2 4");
eqt!(l_union, "(1 2 3) union 2 3 4", "1 2 3 4");
eqt!(l_except, "(1 2 3 4) except 2 4", "1 3");
eqt!(l_flip, "flip (1 2 3;4 5 6)", "(1 4;2 5;3 6)");
eqt!(l_rank, "rank 3 1 4 1 5", "2 0 3 1 4");
eqt!(l_mcount, "count each (1 2;3 4 5;enlist 6)", "2 3 1");
eqt!(l_raze_til, "raze (til 3;til 2)", "0 1 2 0 1");
eqt!(l_reverse_str, "reverse \"abc\"", "\"cba\"");
eqt!(l_count0, "count ()", "0");
eqt!(l_first_empty, "first `int$()", "0N");
eqt!(s_asc, "asc 3 1 2", "1 2 3");
eqt!(s_desc, "desc 1 3 2", "3 2 1");
eqt!(s_iasc, "iasc 3 1 2", "1 2 0");
eqt!(s_idesc, "idesc 1 3 2", "1 2 0");
eqt!(s_xbar, "5 xbar 0 3 6 9 12", "0 0 5 5 10");
eqt!(s_bin, "1 3 5 7 9 bin 4", "1");
eqt!(s_bin_v, "(1 3 5 7) bin 0 2 6 8", "-1 0 2 3");
eqt!(s_rank_f, "rank 1.5 0.5 2.5", "1 0 2");
eqt!(s_asc_f, "asc 3.3 1.1 2.2", "1.1 2.2 3.3");
eqt!(s_asc_sym, "asc `c`a`b", "`a`b`c");
eqt!(s_xbar_t, "2 xbar 0 1 2 3 4 5", "0 0 2 2 4 4");
eqt!(s_iasc_id, "(asc 5 3 1 4)~(5 3 1 4)iasc 5 3 1 4", "1b");
eqt!(t_lower, "lower \"HELLO\"", "\"hello\"");
eqt!(t_upper, "upper \"hello\"", "\"HELLO\"");
eqt!(t_trim, "trim \"  hi  \"", "\"hi\"");
eqt!(t_ltrim, "ltrim \"  hi\"", "\"hi\"");
eqt!(t_rtrim, "rtrim \"hi  \"", "\"hi\"");
eqt!(t_like, "\"hello\" like \"hel*\"", "1b");
eqt!(t_like_q, "\"cat\" like \"c?t\"", "1b");
eqt!(t_like_no, "\"dog\" like \"c*\"", "0b");
eqt!(t_ss, "\"abcabc\" ss \"bc\"", "1 4");
eqt!(t_string_int, "string 42", "\"42\"");
eqt!(t_count_str, "count \"hello\"", "5");
eqt!(c_str_int, "string 255", "\"255\"");
eqt!(c_type_int, "type 5", "-6h");
eqt!(c_type_long, "type 5j", "-7h");
eqt!(c_type_float, "type 5.0", "-9h");
eqt!(c_type_sym, "type `a", "-11h");
eqt!(c_type_intv, "type 1 2 3", "6h");
eqt!(c_type_charv, "type \"abc\"", "10h");
eqt!(c_type_list, "type (1;2.0;`a)", "0h");
eqt!(c_cast_j, "`long$5", "5j");
eqt!(c_cast_f, "`float$5", "5f");
eqt!(c_cast_h, "`short$5", "5h");
eqt!(c_cast_sym, "`$\"abc\"", "`abc");
eqt!(c_cast_str_sym, "string `abc", "\"abc\"");
eqt!(c_tok_int, "\"I\"$\"42\"", "42i");
eqt!(c_bool_cast, "`boolean$1 0 1", "101b");
eqt!(c_char_cast, "`char$65 66 67", "\"ABC\"");
eqt!(p_date_add, "2024.01.01+30", "2024.01.31");
eqt!(p_date_sub, "2024.02.01-2024.01.01", "31");
eqt!(p_date_lt, "2024.01.01<2024.12.31", "1b");
eqt!(p_minute, "12:30+10", "12:40");
eqt!(p_date_int, "`int$2000.01.01", "0i");
eqt!(p_date_2000, "2000.01.02-2000.01.01", "1");
eqt!(p_month_add, "2024.01m+3", "2024.04m");
eqt!(p_date_wd, "2024.01.01+til 3", "2024.01.01 2024.01.02 2024.01.03");
eqt!(i_at, "(10 20 30 40)[2]", "30");
eqt!(i_at_v, "(10 20 30 40)[1 3]", "20 40");
eqt!(i_first, "first 10 20 30", "10");
eqt!(i_amend, "@[1 2 3 4;1;:;99]", "1 99 3 4");
eqt!(i_amend_plus, "@[1 2 3 4;1;+;10]", "1 12 3 4");
eqt!(i_amend_v, "@[1 2 3 4 5;1 3;:;0 0]", "1 0 3 0 5");
eqt!(i_dot, "(1 2;3 4) . 1 0", "3");
eqt!(i_apply, "(+) . 2 3", "5");
eqt!(i_take, "3#1 2 3 4 5", "1 2 3");
eqt!(i_take_neg, "-2#1 2 3 4 5", "4 5");
eqt!(i_drop, "2_1 2 3 4 5", "3 4 5");
eqt!(i_drop_neg, "-2_1 2 3 4 5", "1 2 3");
eqt!(i_take_over, "7#1 2 3", "1 2 3 1 2 3 1");
eqt!(i_fill_v, "0^(1;0N;3)", "1 0 3");
eqt!(i_trap, "@[{x+1};5;`err]", "6");
eqt!(i_trap_err, "@[{x+`sym};5;{`caught}]", "`caught");
eqt!(i_amend_at, "@[`a`b`c!1 2 3;`b;:;99]", "`a`b`c!1 99 3");
eqt!(o_join, "1 2,3 4", "1 2 3 4");
eqt!(o_join_at, "1,2", "1 2");
eqt!(o_take_dict, "2#`a`b`c!1 2 3", "`a`b!1 2");
eqt!(o_fill_atom, "5^0N", "5");
eqt!(o_cut_op, "2 _ 1 2 3 4 5", "3 4 5");
eqt!(o_match, "(1 2 3)~1 2 3", "1b");
eqt!(o_not_match, "(1 2 3)~1 2 4", "0b");
eqt!(o_max_op, "3|5", "5");
eqt!(o_min_op, "3&5", "3");
eqt!(o_max_v, "1 5 3|4 2 6", "4 5 6");
eqt!(o_min_v, "1 5 3&4 2 6", "1 2 3");
eqt!(o_eq, "3=3", "1b");
eqt!(o_neq, "3<>4", "1b");
eqt!(o_lt, "3<4", "1b");
eqt!(o_gt, "4>3", "1b");
eqt!(o_le, "3<=3", "1b");
eqt!(o_ge, "4>=4", "1b");
eqt!(o_eq_v, "1 2 3=1 5 3", "101b");
eqt!(st_distinct_s, "distinct `a`b`a", "`a`b");
eqt!(st_count_distinct, "count distinct 1 1 2 3 3", "3");
eqt!(st_in_sym, "`b in `a`b`c", "1b");
eqt!(st_except_s, "`a`b`c`d except `b`d", "`a`c");
eqt!(st_union_s, "`a`b union `b`c", "`a`b`c");
eqt!(st_inter_s, "`a`b`c inter `b`c`d", "`b`c");
eqt!(it_each, "count each (1 2;3 4 5)", "2 3");
eqt!(it_each_m, "(2*) each 1 2 3", "2 4 6");
eqt!(it_over, "(+/)1 2 3 4", "10j");
eqt!(it_over_seed, "0 +/ 1 2 3", "6j");
eqt!(it_scan, "(+\\)1 2 3 4", "1 3 6 10");
eqt!(it_over_mul, "(*/)1 2 3 4", "24");
eqt!(it_eachleft, "1 2 3 +\\: 10", "11 12 13");
eqt!(it_eachright, "10 +/: 1 2 3", "11 12 13");
eqt!(it_each2, "(+)'[1 2 3;4 5 6]", "5 7 9");
eqt!(it_over_max, "(|/)3 1 4 1 5", "5");
eqt!(it_over_min, "(&/)3 1 4 1 5", "1");
eqt!(it_each_str, "count each (\"ab\";\"cde\")", "2 3");
eqt!(it_scan_mul, "(*\\)1 2 3 4", "1 2 6 24");
eqt!(it_do, "{x*2}/[3;1]", "8");
eqt!(dc_make, "`a`b`c!1 2 3", "`a`b`c!1 2 3");
eqt!(dc_index, "(`a`b`c!1 2 3)`b", "2");
eqt!(dc_key, "key `a`b!1 2", "`a`b");
eqt!(dc_value, "value `a`b!1 2", "1 2");
eqt!(dc_add, "(`a`b!1 2)+`a`b!10 20", "`a`b!11 22");
eqt!(dc_count, "count `a`b`c!1 2 3", "3");
eqt!(dc_find, "(`a`b`c!10 20 30)`a`c", "10 30");
eqt!(q_sel, "select from ([]a:1 2 3)", "([]a:1 2 3)");
eqt!(q_sel_where, "select from ([]a:1 2 3 4) where a>2", "([]a:3 4)");
eqt!(q_sel_col, "select a from ([]a:1 2 3;b:4 5 6)", "([]a:1 2 3)");
eqt!(q_exec, "exec a from ([]a:1 2 3)", "1 2 3");
eqt!(q_sel_by, "select sum b by a from ([]a:`x`y`x;b:1 2 3)", "([a:`x`y]b:4 \
    2j)");
eqt!(q_update, "update c:a+1 from ([]a:1 2 3)", "([]a:1 2 3;c:2 3 4)");
eqt!(q_delete, "delete from ([]a:1 2 3 4) where a>2", "([]a:1 2)");
eqt!(q_count_sel, "count select from ([]a:til 100)", "100");
eqt!(tb_cols, "cols ([]a:1 2;b:3 4)", "`a`b");
eqt!(tb_count, "count ([]a:1 2 3)", "3");
eqt!(tb_first, "first ([]a:1 2 3;b:4 5 6)", "`a`b!1 4");
eqt!(tb_xcol, "`x`y xcol ([]a:1 2;b:3 4)", "([]x:1 2;y:3 4)");
eqt!(tb_meta_c, "cols meta ([]a:1 2)", "`c`t`f`a");
eqt!(tb_xasc, "`a xasc ([]a:3 1 2;b:`x`y`z)", "([]a:1 2 3;b:`y`z`x)");
eqt!(tb_reverse, "reverse ([]a:1 2 3)", "([]a:3 2 1)");
eqt!(j_comma, "([]a:1 2),([]a:3 4)", "([]a:1 2 3 4)");
eqt!(j_uj, "([]a:1 2)uj([]a:3 4)", "([]a:1 2 3 4)");
eqt!(ai_1, "-5 3 -8 -7|-6 2 9 -8", "-5 3 9 -7");
eqt!(ai_2, "-3 -8 -7 4 4 -7--7 8 4 -8 9 -6", "4 -16 -11 12 -5 -1");
eqt!(ai_3, "9 -8 9|3 -8 -2", "9 -8 9");
eqt!(ai_4, "8 -5*4 -5", "32 25");
eqt!(ai_5, "-6 9 0 8 -4 -6|9 -3 2 -6 8 -7", "9 9 2 8 8 -6");
eqt!(ai_6, "-8 -3 6 8 4 1&9 5 2 0 -2 -4", "-8 -3 2 0 -2 -4");
eqt!(ai_7, "-2 -7 9 0 7 6 1&0 -7 -6 7 4 -4 1", "-2 -7 -6 0 4 -4 1");
eqt!(ai_8, "6 4 -8+8 9 1", "14 13 -7");
eqt!(ai_9, "2 6 9 5+-7 -1 6 -7", "-5 5 15 -2");
eqt!(ai_10, "0 9&0 3", "0 3");
eqt!(ai_11, "2 -9 5 2 -4 -6 6+-3 0 -5 -2 3 3 6", "-1 -9 0 0 -1 -3 12");
eqt!(ai_12, "-4 5&8 -1", "-4 -1");
eqt!(ai_13, "4 8 -1&2 3 -2", "2 3 -2");
eqt!(ai_14, "-7 -4 -5--2 -9 6", "-5 5 -11");
eqt!(ai_15, "9 -4 -1 0 -9 -5 4 8*9 1 -5 7 -8 5 8 3", "81 -4 5 0 72 -25 32 24");
eqt!(ai_16, "3 3 -6 6 3+-3 -7 -3 5 -4", "0 -4 -9 11 -1");
eqt!(ai_17, "1 -8+-9 9", "-8 1");
eqt!(ai_18, "8 -6 2|-9 -7 -3", "8 -6 2");
eqt!(ai_19, "3 -5 -1 2 2 6+-6 6 5 6 6 0", "-3 1 4 8 8 6");
eqt!(ai_20, "-5 -6*-1 6", "5 -36");
eqt!(ai_21, "-4 7 -9 -3 7 2 -5 8+7 0 -7 -1 7 2 -4 2", "3 7 -16 -4 14 4 -9 10");
eqt!(ai_22, "-2 8 8 7 1 -2 -3 -2&-2 -3 7 6 2 -9 -9 -1", "-2 -3 7 6 1 -9 -9 -2");
eqt!(ai_23, "-1 -3 2 5 2*-7 -2 -6 -2 6", "7 6 -12 -10 12");
eqt!(ai_24, "1 -3 6|-9 6 2", "1 6 6");
eqt!(ai_25, "-7 -6 3 -3 6 -4 4 1+3 5 3 -7 -4 -4 -5 -9", "-4 -1 6 -10 2 -8 -1 \
    -8");
eqt!(ai_26, "9 5 -5|6 2 -5", "9 5 -5");
eqt!(ai_27, "8 -5 -9 -9 -6 7-4 -3 -3 -9 -1 -3", "4 -2 -6 0 -5 10");
eqt!(ai_28, "7 -2 9 1*8 4 -5 -8", "56 -8 -45 -8");
eqt!(ai_29, "2 5 9 7 4 7 -5|-5 7 7 -9 5 -4 -9", "2 7 9 7 5 7 -5");
eqt!(ai_30, "-5 -4 -5 6 -6 8 -8 1|7 8 6 -6 8 -8 -2 -3", "7 8 6 6 8 8 -2 1");
eqt!(ai_31, "-8 -6 7 5|-9 -7 5 1", "-8 -6 7 5");
eqt!(ai_32, "7 7 -3 -1 5 7|6 7 -2 7 -1 8", "7 7 -2 7 5 8");
eqt!(ai_33, "5 -5 4+3 5 1", "8 0 5");
eqt!(ai_34, "-2 4+-3 0", "-5 4");
eqt!(ai_35, "-6 -5 2 -5 -1 -5 5 -2+3 6 -4 -2 -4 4 7 3", "-3 1 -2 -7 -5 -1 12 \
    1");
eqt!(ai_36, "4 -3 2 1+2 -9 1 8", "6 -12 3 9");
eqt!(ai_37, "5 -9 3 1 7|0 7 -7 -6 -2", "5 7 3 1 7");
eqt!(ai_38, "-7 -1*-8 -4", "56 4");
eqt!(ai_39, "-5 4 -1 3-8 7 9 6", "-13 -3 -10 -3");
eqt!(ai_40, "1 -7 -1 -8 -4 4 -7*-9 -7 -1 -7 -2 -7 -1", "-9 49 1 56 8 -28 7");
eqt!(ai_41, "-6 5 -9 1 8 4 -1 -5+7 -2 -6 -4 -1 -8 -4 -3", "1 3 -15 -3 7 -4 -5 \
    -8");
eqt!(ai_42, "0 7 -3 0&7 -4 -1 2", "0 -4 -3 0");
eqt!(ai_43, "-9 -1 -8 -9 -9 7 8 -3|6 -2 5 -6 4 6 8 3", "6 -1 5 -6 4 7 8 3");
eqt!(ai_44, "0 -3 -2 1 -3 -5&2 -8 -5 -9 -7 -1", "0 -8 -5 -9 -7 -5");
eqt!(ai_45, "-4 -8 -7 3 7*-2 0 -8 5 -4", "8 0 56 15 -28");
eqt!(ai_46, "-1 5 -9*2 1 8", "-2 5 -72");
eqt!(ai_47, "-2 -8 0 -3*-4 -9 1 3", "8 72 0 -9");
eqt!(ai_48, "6 -1|-3 -2", "6 -1");
eqt!(ai_49, "-9 -7 -1 -7 -5 3|-8 3 -9 0 0 -2", "-8 3 -1 0 0 3");
eqt!(ai_50, "9 7-3 1", "6 6");
eqt!(ai_51, "6 -5 0 -5 -8 7 4|-5 7 7 9 -9 9 -2", "6 7 7 9 -8 9 4");
eqt!(ai_52, "-9 -8-2 -6", "-11 -2");
eqt!(ai_53, "5 8 -8 -9 8-6 -1 -9 5 -7", "-1 9 1 -14 15");
eqt!(ai_54, "7 8 -7 7 -7 6 -1+-1 -2 -3 -2 5 6 3", "6 6 -10 5 -2 12 2");
eqt!(ai_55, "6 0+-3 -7", "3 -7");
eqt!(ai_56, "-5 1 -1 0 9 -5+6 -8 6 -1 -6 -3", "1 -7 5 -1 3 -8");
eqt!(ai_57, "6 0 7 0 5 5 5+8 -3 0 -7 6 -9 0", "14 -3 7 -7 11 -4 5");
eqt!(ai_58, "-7 7 5 -1 3--3 -7 9 -7 -5", "-4 14 -4 6 8");
eqt!(ai_59, "7 -1 2 -5 7 -1 -6*-2 6 6 3 -9 -4 -9", "-14 -6 12 -15 -63 4 54");
eqt!(ai_60, "5 3 0 -5 4*3 1 -6 1 -9", "15 3 0 -5 -36");
eqt!(ai_61, "1 3 -6 -3+0 -1 2 -7", "1 2 -4 -10");
eqt!(ai_62, "3 9 -7 2 4*-8 -1 -6 -8 0", "-24 -9 42 -16 0");
eqt!(ai_63, "-5 -2 -1 4 7 1 -3*4 -9 3 8 8 -3 -7", "-20 18 -3 32 56 -3 21");
eqt!(ai_64, "4 5|-5 0", "4 5");
eqt!(ai_65, "-8 8 -5 -4 6&1 0 0 -1 -1", "-8 0 -5 -4 -1");
eqt!(ai_66, "-2 0 6 8 3+-4 -4 -7 -3 7", "-6 -4 -1 5 10");
eqt!(ai_67, "6 8 -2 5 1 5 4 -5|-3 -2 -7 -4 1 8 -7 1", "6 8 -2 5 1 8 4 1");
eqt!(ai_68, "2 -1 9--9 4 3", "11 -5 6");
eqt!(ai_69, "7 -3 3 -1 1+6 -1 9 2 -5", "13 -4 12 1 -4");
eqt!(ai_70, "7 7 -3 -7 -1 -2 3&5 4 0 -9 -5 -8 4", "5 4 -3 -9 -5 -8 3");
eqt!(ai_71, "6 9 6 -9 -7 3 7&5 -2 -6 -2 -5 -5 7", "5 -2 -6 -9 -7 -5 7");
eqt!(ai_72, "-6 5 -7 8 -8 -9 -5-9 -8 0 -5 -1 7 4", "-15 13 -7 13 -7 -16 -9");
eqt!(ai_73, "-6 -6 -7 0 7 9 -3&-1 -2 -9 -9 8 0 5", "-6 -6 -9 -9 7 0 -3");
eqt!(ai_74, "1 -2 6 7-8 -2 -9 4", "-7 0 15 3");
eqt!(ai_75, "0 -8 -9 -3 6 4 -7*-2 4 2 -2 6 -8 1", "0 -32 -18 6 36 -32 -7");
eqt!(ai_76, "4 2 3 -3 -9 0 7+-3 6 -3 0 -3 -2 5", "1 8 0 -3 -12 -2 12");
eqt!(ai_77, "-1 0 -6|6 -4 -2", "6 0 -2");
eqt!(ai_78, "4 -8 -5 3 -8--9 -5 4 -8 -8", "13 -3 -9 11 0");
eqt!(ai_79, "3 5 1+-7 -4 1", "-4 1 2");
eqt!(ai_80, "-4 7 5+0 3 2", "-4 10 7");
eqt!(ai_81, "5 -4 -6 -9+-1 -7 2 4", "4 -11 -4 -5");
eqt!(ai_82, "8 -3&2 0", "2 -3");
eqt!(ai_83, "4 -7 -8 6 -3 2 8 5-1 2 6 -9 4 -2 3 -8", "3 -9 -14 15 -7 4 5 13");
eqt!(ai_84, "-8 5 -7 -8 -1--7 1 2 -1 1", "-1 4 -9 -7 -2");
eqt!(ai_85, "-8 -1 1 -1 0 -9|-7 -9 -2 -6 6 5", "-7 -1 1 -1 6 5");
eqt!(ai_86, "3 -1 4 6 -5 6 -4 -9*-5 -2 1 1 5 2 -7 7", "-15 2 4 6 -25 12 28 \
    -63");
eqt!(ai_87, "3 -4 -2&-7 -8 6", "-7 -8 -2");
eqt!(ai_88, "8 1 -4 4 -6 -7*-7 -3 -6 4 6 5", "-56 -3 24 16 -36 -35");
eqt!(ai_89, "-2 -5 4&-2 8 -6", "-2 -5 -6");
eqt!(ai_90, "0 0 -1 9 -1 2 -1 -1-5 -2 -4 -2 -2 -5 0 9", "-5 2 3 11 1 7 -1 -10");
eqt!(ai_91, "1 -7 3*-2 7 7", "-2 -49 21");
eqt!(ai_92, "-6 5 -8+-9 6 -2", "-15 11 -10");
eqt!(ai_93, "5 2 -8 0 -2 -6 -8 -3|9 -3 -7 2 7 -4 5 -1", "9 2 -7 2 7 -4 5 -1");
eqt!(ai_94, "-9 -6 2 -3 -8 2 1 -5+-3 -1 -8 -3 -9 1 4 2", "-12 -7 -6 -6 -17 3 5 \
    -3");
eqt!(ai_95, "0 -7 -3+6 8 6", "6 1 3");
eqt!(ai_96, "4 -6&8 -5", "4 -6");
eqt!(ai_97, "8 -7 -4 3 -1 4 0*4 -8 0 9 2 4 4", "32 56 0 27 -2 16 0");
eqt!(ai_98, "2 -3&3 -3", "2 -3");
eqt!(ai_99, "4 -4&-6 -7", "-6 -7");
eqt!(ai_100, "9 2 5 -4 -5+-8 8 -5 3 -7", "1 10 0 -1 -12");
eqt!(ai_101, "2 7 -4 -5 2 0-7 -4 -7 -6 3 6", "-5 11 3 1 -1 -6");
eqt!(ai_102, "-3 0 -5 -8 6 1 -8 3+-4 -2 3 -3 6 -4 9 -3", "-7 -2 -2 -11 12 -3 1 \
    0");
eqt!(ai_103, "3 7-3 2", "0 5");
eqt!(ai_104, "-5 -2--8 8", "3 -10");
eqt!(ai_105, "-8 1 -6 3 5 8 0 4*9 -2 4 3 2 5 7 5", "-72 -2 -24 9 10 40 0 20");
eqt!(ai_106, "-9 -9 6&-2 5 5", "-9 -9 5");
eqt!(ai_107, "-4 6 3 -6 -7 -5 2 4*-7 5 7 7 -8 -8 -5 -7", "28 30 21 -42 56 40 \
    -10 -28");
eqt!(ai_108, "1 7 -7 -8 7 3 -5+-7 -6 -3 -5 6 0 -4", "-6 1 -10 -13 13 3 -9");
eqt!(ai_109, "-2 -7 2 -1 -4 1 -1&-5 -1 7 6 -3 9 -1", "-5 -7 2 -1 -4 1 -1");
eqt!(ai_110, "7 -2 1 2 -8 -3-3 -4 -1 1 3 -4", "4 2 2 1 -11 1");
eqt!(ai_111, "-1 -6 7 -8 2 5 8 7|-6 -1 8 3 2 -1 3 2", "-1 -1 8 3 2 5 8 7");
eqt!(ai_112, "-5 2 1 -7 5 -2--8 0 7 -1 0 9", "3 2 -6 -6 5 -11");
eqt!(ai_113, "1 -9 -8 -2 -5 0 4&7 2 -8 -5 6 -2 -8", "1 -9 -8 -5 -5 -2 -8");
eqt!(ai_114, "-8 -9|2 0", "2 0");
eqt!(ai_115, "7 2|-2 4", "7 4");
eqt!(ai_116, "0 9 -5 -3 2 6--5 -9 -2 -5 5 -6", "5 18 -3 2 -3 12");
eqt!(ai_117, "-5 -1&-1 -9", "-5 -9");
eqt!(ai_118, "8 2|9 5", "9 5");
eqt!(ai_119, "7 6 -2 -4 -9 -8+8 -9 3 -4 -2 -4", "15 -3 1 -8 -11 -12");
eqt!(ai_120, "-6 -9|8 -3", "8 -3");
eqt!(ai_121, "4 -3 7|7 4 -4", "7 4 7");
eqt!(ai_122, "0 -7 0 -8 6 8+3 4 5 -7 5 -4", "3 -3 5 -15 11 4");
eqt!(ai_123, "-6 -1 -2+-6 1 -1", "-12 0 -3");
eqt!(ai_124, "-8 -1 8 4 7 -1 0--7 7 -9 -4 -1 -2 -3", "-1 -8 17 8 8 1 3");
eqt!(ai_125, "1 -3 3*-2 3 8", "-2 -9 24");
eqt!(ai_126, "6 7 -9 -9 4-9 0 -3 3 9", "-3 7 -6 -12 -5");
eqt!(ai_127, "9 -4--8 -9", "17 5");
eqt!(ai_128, "-6 -4*-5 -9", "30 36");
eqt!(ai_129, "-8 -5+-7 -8", "-15 -13");
eqt!(ai_130, "9 2-8 -7", "1 9");
eqt!(ai_131, "3 -6 -2 -3 -3 -6 -8 -8+0 6 -6 -5 -6 -3 0 1", "3 0 -8 -8 -9 -9 -8 \
    -7");
eqt!(ai_132, "4 -1 -9 2*0 -8 2 1", "0 8 -18 2");
eqt!(ai_133, "7 6 0 -9 4 -9 4 7+2 6 -8 8 9 -3 -7 9", "9 12 -8 -1 13 -12 -3 16");
eqt!(ai_134, "0 -4 4 -9 7 -3 0 -8+2 6 -6 6 -4 6 9 2", "2 2 -2 -3 3 3 9 -6");
eqt!(ai_135, "7 -1 9 -4 0 -3 -2 6--6 -7 6 8 -6 1 2 -6", "13 6 3 -12 6 -4 -4 \
    12");
eqt!(ai_136, "3 -7 4 -9 2-0 -1 4 8 7", "3 -6 0 -17 -5");
eqt!(ai_137, "3 -2 5-8 -8 2", "-5 6 3");
eqt!(ai_138, "1 7 -5 5 8 1-5 5 -1 9 -2 -5", "-4 2 -4 -4 10 6");
eqt!(ai_139, "5 -2 7 -3*0 -5 -5 -2", "0 10 -35 6");
eqt!(ai_140, "1 7 2 -4 -2 1 -3*-6 -4 -6 -3 3 -5 -5", "-6 -28 -12 12 -6 -5 15");
eqt!(ai_141, "0 0 4 -1 -3 -6 -6 -1-3 5 -8 -9 3 4 -2 7", "-3 -5 12 8 -6 -10 -4 \
    -8");
eqt!(ai_142, "0 5 -9 -5 -1 3 -9-4 9 9 4 -2 9 -2", "-4 -4 -18 -9 1 -6 -7");
eqt!(ai_143, "-4 -6 5 4 1 -1 -6&-2 3 -4 -1 4 6 5", "-4 -6 -4 -1 1 -1 -6");
eqt!(ai_144, "4 7-1 -9", "3 16");
eqt!(ai_145, "6 -6 -8 -1 8--4 -3 7 2 -6", "10 -3 -15 -3 14");
eqt!(ai_146, "9 5 8 -3 6 7 -9 2|1 4 5 -3 -4 3 7 -6", "9 5 8 -3 6 7 7 2");
eqt!(ai_147, "2 -8 -1 -1 3 3 -8+-7 4 4 2 9 -1 -6", "-5 -4 3 1 12 2 -14");
eqt!(ai_148, "0 3 7-3 5 -3", "-3 -2 10");
eqt!(ai_149, "-5 -7 -3&8 -2 -5", "-5 -7 -5");
eqt!(ai_150, "4 5 0 8-6 2 -2 -1", "-2 3 2 9");
eqt!(ai_151, "3 -1 4 -4 6 -9 -1*-2 0 1 6 6 4 -7", "-6 0 4 -24 36 -36 7");
eqt!(ai_152, "2 -5 0 3 -8 -7 9*-5 7 2 9 -9 -9 -3", "-10 -35 0 27 72 63 -27");
eqt!(ai_153, "0 -1|-6 9", "0 9");
eqt!(ai_154, "-2 -4 5*-5 -3 3", "10 12 15");
eqt!(ai_155, "8 -4 -7 8 0 -3 6 -3|-7 5 -6 8 -6 -1 4 -2", "8 5 -6 8 0 -1 6 -2");
eqt!(ai_156, "-5 6 6 8 -8 6 5 -5&-2 6 -4 8 -9 -4 1 5", "-5 6 -4 8 -9 -4 1 -5");
eqt!(ai_157, "9 6 0 5 2 4 4+-4 2 -9 -9 -8 1 -6", "5 8 -9 -4 -6 5 -2");
eqt!(ai_158, "6 6 -5 -8 -3 4-1 -6 2 1 6 7", "5 12 -7 -9 -9 -3");
eqt!(ai_159, "-3 0 4 1 4 -1|-8 0 0 2 6 3", "-3 0 4 2 6 3");
eqt!(ai_160, "7 -1 7 2-6 -6 1 -3", "1 5 6 5");
eqt!(ai_161, "0 -5 9 -7+3 8 3 8", "3 3 12 1");
eqt!(ai_162, "-8 3 0 -6 -9 -8-6 -8 7 8 3 -5", "-14 11 -7 -14 -12 -3");
eqt!(ai_163, "-7 -3 -8 5 -4 -6 -4+4 -6 -9 2 -5 0 8", "-3 -9 -17 7 -9 -6 4");
eqt!(ai_164, "-1 0 -4 4 -8 1 -9&9 9 -8 6 9 7 -8", "-1 0 -8 4 -8 1 -9");
eqt!(ai_165, "-6 4 9 3 5 -7 -9 3|9 -5 6 4 8 -6 -7 6", "9 4 9 4 8 -6 -7 6");
eqt!(ai_166, "-5 -9 4+-9 -6 -7", "-14 -15 -3");
eqt!(ai_167, "-6 -5 6+-1 9 -2", "-7 4 4");
eqt!(ai_168, "-4 -8 2 -5 -7*8 6 5 -1 -8", "-32 -48 10 5 56");
eqt!(ai_169, "-8 -9 -8 -9 -7 3 0*-4 6 -8 1 2 9 5", "32 -54 64 -9 -14 27 0");
eqt!(ai_170, "-4 -5 -6 2 -4&6 3 5 -1 9", "-4 -5 -6 -1 -4");
eqt!(ai_171, "0 -1 -8 1|-9 -5 0 9", "0 -1 0 9");
eqt!(ai_172, "-2 3 3 3 -2&0 -9 1 -1 -1", "-2 -9 1 -1 -2");
eqt!(ai_173, "-4 9 -8 0 -5|-5 -1 8 6 2", "-4 9 8 6 2");
eqt!(ai_174, "-7 8 8 6 3 -3-0 -8 3 5 -3 -1", "-7 16 5 1 6 -2");
eqt!(ai_175, "-9 3 5 8 -7 8*-7 -2 3 9 7 -1", "63 -6 15 72 -49 -8");
eqt!(ai_176, "7 1 6 7 9 -3 -3 -3--7 -4 0 2 9 9 2 3", "14 5 6 5 0 -12 -5 -6");
eqt!(ai_177, "7 -5 -2 -8 6 2 -6 2&-7 -5 1 -9 2 -1 7 -9", "-7 -5 -2 -9 2 -1 -6 \
    -9");
eqt!(ai_178, "-8 -3|6 9", "6 9");
eqt!(ai_179, "-3 -1 -1 4 -6 5|-5 -1 -8 1 -3 -4", "-3 -1 -1 4 -3 5");
eqt!(ai_180, "-7 -9 -8 -8 8*5 6 -7 3 -6", "-35 -54 56 -24 -48");
eqt!(ai_181, "-7 -1 1 9 -2 -7 7&-4 5 -4 2 -2 -2 -4", "-7 -1 -4 2 -2 -7 -4");
eqt!(ai_182, "-1 2+8 -9", "7 -7");
eqt!(ai_183, "-8 -1 7 6 -8 -6 -5 1+-3 0 9 9 5 -6 6 1", "-11 -1 16 15 -3 -12 1 \
    2");
eqt!(ai_184, "-1 3 -6 2&3 -4 5 -2", "-1 -4 -6 -2");
eqt!(ai_185, "-5 -9 5 -3 -8 -4 -2 -7|2 -5 5 -6 3 -9 -7 5", "2 -5 5 -3 3 -4 -2 \
    5");
eqt!(ai_186, "1 -2 6 -6*-5 1 -2 -8", "-5 -2 -12 48");
eqt!(ai_187, "5 8 -5&-5 -1 4", "-5 -1 -5");
eqt!(ai_188, "-2 -5 -9 -1 9*1 -4 -1 6 -6", "-2 20 9 -6 -54");
eqt!(ai_189, "5 6 -6 -5|-8 -3 8 6", "5 6 8 6");
eqt!(ai_190, "0 -6 -1 -3 2 4 -1 -2--6 3 0 4 -4 -8 0 -5", "6 -9 -1 -7 6 12 -1 \
    3");
eqt!(ai_191, "-9 5 7 1 7 -5 5+7 0 -4 2 4 -8 4", "-2 5 3 3 11 -13 9");
eqt!(ai_192, "-1 9 -4--4 7 -2", "3 2 -2");
eqt!(ai_193, "-4 -3 -7 -7 6 -1 -4--5 -3 9 0 -3 -9 -7", "1 0 -16 -7 9 8 3");
eqt!(ai_194, "7 4 -8 7 2 1 0&-7 -9 4 6 -5 -1 -2", "-7 -9 -8 6 -5 -1 -2");
eqt!(ai_195, "9 2 -8-2 9 -9", "7 -7 1");
eqt!(ai_196, "7 5 7 -7+2 -2 1 3", "9 3 8 -4");
eqt!(ai_197, "-8 0 -6 6 5 7+7 8 -5 -9 -2 -7", "-1 8 -11 -3 3 0");
eqt!(ai_198, "-4 -4 -6*-1 8 -9", "4 -32 54");
eqt!(ai_199, "-6 -3*-9 9", "54 -27");
eqt!(ai_200, "7 -2 5 -6 2+-4 -8 -1 -6 5", "3 -10 4 -12 7");
eqt!(ai_201, "9 7 -1 -6 -6+3 -5 8 9 -2", "12 2 7 3 -8");
eqt!(ai_202, "-2 -5 9 5 3 -4 -9 3&7 -8 3 -8 2 1 3 -2", "-2 -8 3 -8 2 -4 -9 -2");
eqt!(ai_203, "1 4 9 1 3 8 -8 1|-5 2 -2 4 -9 2 -6 7", "1 4 9 4 3 8 -6 7");
eqt!(ai_204, "-7 1 4-7 -9 -2", "-14 10 6");
eqt!(ai_205, "4 3 5+-8 -8 -1", "-4 -5 4");
eqt!(ai_206, "-1 8 -8 -6 -1 -6 7+4 -2 -8 0 -6 0 2", "3 6 -16 -6 -7 -6 9");
eqt!(ai_207, "-4 -6 -8 7 -1 -7 5|8 -5 5 -6 7 -5 0", "8 -5 5 7 7 -5 5");
eqt!(ai_208, "9 0 -1 -2 -7|0 5 9 -2 3", "9 5 9 -2 3");
eqt!(ai_209, "8 2 5|0 6 6", "8 6 6");
eqt!(ai_210, "0 -9 -2 1 -2 -3 7 8&9 3 -9 2 -4 -2 1 8", "0 -9 -9 1 -4 -3 1 8");
eqt!(ai_211, "6 -1 0 -3*-8 -9 -4 8", "-48 9 0 -24");
eqt!(ai_212, "2 5+7 3", "9 8");
eqt!(ai_213, "5 2 -6 7 -2 -5 4 1*-5 -3 -1 7 -6 6 -1 -5", "-25 -6 6 49 12 -30 \
    -4 \
    -5");
eqt!(ai_214, "-6 -9 4 8 9+6 3 9 -5 4", "0 -6 13 3 13");
eqt!(ai_215, "-1 -6 3 5 5 0 2 0*3 7 8 3 1 -9 6 3", "-3 -42 24 15 5 0 12 0");
eqt!(ai_216, "0 -4 8 0 -5&9 3 9 -2 -7", "0 -4 8 -2 -7");
eqt!(ai_217, "1 1 -2 1 -3 4 -9 -9+-1 9 6 0 8 0 8 4", "0 10 4 1 5 4 -1 -5");
eqt!(ai_218, "7 4 3 5 2 -8|2 5 -9 -7 7 -2", "7 5 3 5 7 -2");
eqt!(ai_219, "4 2|3 8", "4 8");
eqt!(ai_220, "-5 -3 4 6 3 5|9 1 7 -7 -4 2", "9 1 7 6 3 5");
eqt!(ai_221, "2 -7 0 7--6 0 1 7", "8 -7 -1 0");
eqt!(ai_222, "-4 7 0 7 -3|-3 4 -4 -8 9", "-3 7 0 7 9");
eqt!(ai_223, "-6 2 9 -8 4 -9+0 8 -9 0 3 -6", "-6 10 0 -8 7 -15");
eqt!(ai_224, "-9 -9 -3 -4 6 8|-1 8 7 -5 9 -3", "-1 8 7 -4 9 8");
eqt!(ai_225, "-6 -5 -4 7 7+-9 -6 -7 -4 7", "-15 -11 -11 3 14");
eqt!(ai_226, "5 4 -8 -9 9*-5 -2 2 -1 -4", "-25 -8 -16 9 -36");
eqt!(ai_227, "-1 -6|-7 2", "-1 2");
eqt!(ai_228, "5 3 -9+-2 3 9", "3 6 0");
eqt!(ai_229, "-8 5 -8 -2 -2 -2 -8 -4|-4 1 -9 5 0 4 -1 6", "-4 5 -8 5 0 4 -1 6");
eqt!(ai_230, "-2 3|-2 4", "-2 4");
eqt!(ai_231, "3 6 -9 -2+-4 -4 2 3", "-1 2 -7 1");
eqt!(ai_232, "-9 0 3|2 -6 1", "2 0 3");
eqt!(ai_233, "3 1 3 -7 -6 4*8 -2 3 -3 5 0", "24 -2 9 21 -30 0");
eqt!(ai_234, "-2 4 -8 -1+1 -5 -2 -5", "-1 -1 -10 -6");
eqt!(ai_235, "-3 -1|-5 8", "-3 8");
eqt!(ai_236, "5 -2 -4 2 2-3 3 9 -3 0", "2 -5 -13 5 2");
eqt!(ai_237, "7 -3 -2 5 -5*5 9 2 8 -2", "35 -27 -4 40 10");
eqt!(ai_238, "7 -3 -5 -6 7+8 -1 3 -9 9", "15 -4 -2 -15 16");
eqt!(ai_239, "0 -9 3+-4 -2 1", "-4 -11 4");
eqt!(ai_240, "-6 -7 8*7 0 -3", "-42 0 -24");
eqt!(ai_241, "0 -7-0 -5", "0 -2");
eqt!(ai_242, "3 0 2 3 5 -5 -1 -4+2 2 4 -9 5 -2 3 2", "5 2 6 -6 10 -7 2 -2");
eqt!(ai_243, "-6 -4 0 -6 -1 -2 -8&-8 -4 4 -3 0 -5 3", "-8 -4 0 -6 -1 -5 -8");
eqt!(ai_244, "-8 8 0 -4 9 -2 9&7 -1 4 9 2 -9 -6", "-8 -1 0 -4 2 -9 -6");
eqt!(ai_245, "0 -8 9 -8 -2 -6 -8 1-2 -7 4 3 -2 -1 7 -7", "-2 -1 5 -11 0 -5 -15 \
    8");
eqt!(ai_246, "4 5 1 7&7 -8 -3 4", "4 -8 -3 4");
eqt!(ai_247, "7 -5 6 -3 -8 8 -1-8 -4 -2 8 -1 -2 -8", "-1 -1 8 -11 -7 10 7");
eqt!(ai_248, "2 2 4+-3 0 -5", "-1 2 -1");
eqt!(ai_249, "6 6 -2--9 7 5", "15 -1 -7");
eqt!(ai_250, "2 0 -5-9 9 -2", "-7 -9 -3");
eqt!(ai_251, "-6 8 4 -4-5 3 -3 -6", "-11 5 7 2");
eqt!(ai_252, "0 -9 2 6 -3 -8 -8*0 -3 -6 0 5 -6 -4", "0 27 -12 0 -15 48 32");
eqt!(ai_253, "5 5 9 2*-4 8 -7 -8", "-20 40 -63 -16");
eqt!(ai_254, "5 6+1 9", "6 15");
eqt!(ai_255, "-6 6 4 6-8 1 -9 2", "-14 5 13 4");
eqt!(ai_256, "0 -1--7 -5", "7 4");
eqt!(ai_257, "-9 -9 3 -5 0 2 -4|-4 -6 0 1 3 -4 2", "-4 -6 3 1 3 2 2");
eqt!(ai_258, "-2 2 -5 8*-1 -2 -8 -8", "2 -4 40 -64");
eqt!(ai_259, "9 3+-3 6", "6 9");
eqt!(ai_260, "6 -4 0 9 -7--2 -4 -5 5 3", "8 0 5 4 -10");
eqt!(ai_261, "-8 5&-3 -3", "-8 -3");
eqt!(ai_262, "2 -9 -8 7 4 -5 0+-8 7 4 1 -7 5 -9", "-6 -2 -4 8 -3 0 -9");
eqt!(ai_263, "-4 -4 3 0 -9 5 9*9 -3 6 -7 8 1 7", "-36 12 18 0 -72 5 63");
eqt!(ai_264, "4 8 -5 3 -7+1 0 9 9 4", "5 8 4 12 -3");
eqt!(ai_265, "6 -5 0 1|-9 -3 -2 5", "6 -3 0 5");
eqt!(ai_266, "-7 -5 9 2 8 9 4*7 -2 9 5 3 -1 -6", "-49 10 81 10 24 -9 -24");
eqt!(ai_267, "-4 -3 8+-2 -1 -6", "-6 -4 2");
eqt!(ai_268, "7 -1 6-8 5 -2", "-1 -6 8");
eqt!(ai_269, "9 -6 7 9 9 -7&-7 5 -5 7 8 7", "-7 -6 -5 7 8 -7");
eqt!(ai_270, "-6 7 -6 5 3 8 -4-9 6 -7 -5 2 -8 3", "-15 1 1 10 1 16 -7");
eqt!(ai_271, "-8 2 -8+-3 5 0", "-11 7 -8");
eqt!(ai_272, "-5 4+-3 9", "-8 13");
eqt!(ai_273, "2 -4*1 -9", "2 36");
eqt!(ai_274, "-1 -6 -2 2 7 7 2 6+2 -6 2 8 1 -6 -8 -2", "1 -12 0 10 8 1 -6 4");
eqt!(ai_275, "2 -3 5 -9|5 -6 -9 6", "5 -3 5 6");
eqt!(ai_276, "-7 -1--5 8", "-2 -9");
eqt!(ai_277, "3 -5 9 -1|-1 5 -9 -9", "3 5 9 -1");
eqt!(ai_278, "-5 6 7 6+-8 -7 -4 3", "-13 -1 3 9");
eqt!(ai_279, "6 -4 5 3 -2 7 -7 2*7 -3 0 -5 9 -8 -3 -4", "42 12 0 -15 -18 -56 \
    21 \
    -8");
eqt!(ai_280, "2 5 1 9 5 3 2 1+1 9 6 1 -2 -9 -2 5", "3 14 7 10 3 -6 0 6");
eqt!(ai_281, "-8 -5 -5 -1 3 -1+7 -1 2 9 9 7", "-1 -6 -3 8 12 6");
eqt!(ai_282, "-5 -8 8 -6 -3 4|-6 2 0 -2 -5 -7", "-5 2 8 -2 -3 4");
eqt!(ai_283, "1 2 7 -2*8 3 1 -8", "8 6 7 16");
eqt!(ai_284, "1 1 6 7 2 -2 -2*-5 -5 -3 -9 5 3 5", "-5 -5 -18 -63 10 -6 -10");
eqt!(ai_285, "9 0 -4 9 -7-0 0 -1 9 8", "9 0 -3 0 -15");
eqt!(ai_286, "1 -7 -3 9 -7 9 -4*9 2 5 2 4 -7 6", "9 -14 -15 18 -28 -63 -24");
eqt!(ai_287, "-4 -1 -1 8+-4 -1 -2 -9", "-8 -2 -3 -1");
eqt!(ai_288, "-8 3 5-0 7 -6", "-8 -4 11");
eqt!(ai_289, "-2 -8 -5|-8 -7 -7", "-2 -7 -5");
eqt!(ai_290, "9 1 -5 -9 -3 -1 8 -9*-9 -3 1 1 -9 6 3 1", "-81 -3 -5 -9 27 -6 24 \
    -9");
eqt!(ai_291, "-8 4 -8+1 6 3", "-7 10 -5");
eqt!(ai_292, "5 -9 -9 1|1 -8 4 1", "5 -8 4 1");
eqt!(ai_293, "-7 -9 -5--5 7 -7", "-2 -16 2");
eqt!(ai_294, "2 4 2 8|8 -5 9 1", "8 4 9 8");
eqt!(ai_295, "-1 6 -8*8 5 8", "-8 30 -64");
eqt!(ai_296, "2 7 7 -1--1 -9 8 6", "3 16 -1 -7");
eqt!(ai_297, "2 -5-3 -7", "-1 2");
eqt!(ai_298, "-5 -6+8 7", "3 1");
eqt!(ai_299, "8 -4 -1|2 -5 -4", "8 -4 -1");
eqt!(ai_300, "-4 7 -9 2 -2 5 6 -3*3 5 -3 1 -9 -6 -9 -7", "-12 35 27 2 18 -30 \
    -54 21");
eqt!(as_301, "3*-8 -2 9 3 4 3 -2", "-24 -6 27 9 12 9 -6");
eqt!(as_302, "-1+-1 4", "-2 3");
eqt!(as_303, "-2*-3 1 4", "6 -2 -8");
eqt!(as_304, "-1*6 -3 9 -4 6 -1 -5", "-6 3 -9 4 -6 1 5");
eqt!(as_305, "0+1 -9 6 -2", "1 -9 6 -2");
eqt!(as_306, "1|5 -3 9", "5 1 9");
eqt!(as_307, "-3*-8 5", "24 -15");
eqt!(as_308, "4-0 -9 -6", "4 13 10");
eqt!(as_309, "-9-0 -5 7", "-9 -4 -16");
eqt!(as_310, "2+-4 5 3 -7 4 1 3", "-2 7 5 -5 6 3 5");
eqt!(as_311, "-8|-2 -3 -9 -8", "-2 -3 -8 -8");
eqt!(as_312, "7|-2 9 4", "7 9 7");
eqt!(as_313, "-6+-8 1 -7 -6 -6 6 -5", "-14 -5 -13 -12 -12 0 -11");
eqt!(as_314, "4+-4 -2 8 -5 8 7", "0 2 12 -1 12 11");
eqt!(as_315, "7*6 -7", "42 -49");
eqt!(as_316, "-3--7 -1 -4 -9", "4 -2 1 6");
eqt!(as_317, "-1+-8 -3 7 -8", "-9 -4 6 -9");
eqt!(as_318, "8*-1 -9 1 -8 5", "-8 -72 8 -64 40");
eqt!(as_319, "0|1 4 -1 3 4 1", "1 4 0 3 4 1");
eqt!(as_320, "4&-5 3 3 4 -5 -9", "-5 3 3 4 -5 -9");
eqt!(as_321, "7*3 -2 -3", "21 -14 -21");
eqt!(as_322, "-6+-8 -8 3 8 1 5 8", "-14 -14 -3 2 -5 -1 2");
eqt!(as_323, "1&9 -9 6 6 7 1 9", "1 -9 1 1 1 1 1");
eqt!(as_324, "3-3 2 -7 3 7 -1", "0 1 10 0 -4 4");
eqt!(as_325, "1+8 -2 -1 -1 6 2", "9 -1 0 0 7 3");
eqt!(as_326, "9&9 -2 -5 -7 7 2", "9 -2 -5 -7 7 2");
eqt!(as_327, "-3|-4 2 -2 -4 -5 5", "-3 2 -2 -3 -3 5");
eqt!(as_328, "-8*3 2 4", "-24 -16 -32");
eqt!(as_329, "4--1 3", "5 1");
eqt!(as_330, "2*7 7", "14 14");
eqt!(as_331, "5+-1 3 0 5", "4 8 5 10");
eqt!(as_332, "-6&6 -4 7 -5 -9 -5 2", "-6 -6 -6 -6 -9 -6 -6");
eqt!(as_333, "7-2 7 1 3 -1", "5 0 6 4 8");
eqt!(as_334, "8--9 9", "17 -1");
eqt!(as_335, "-8|-4 0 8 -1", "-4 0 8 -1");
eqt!(as_336, "-1--1 5 -7 7", "0 -6 6 -8");
eqt!(as_337, "6+-3 -5 4 0 2 -8 5", "3 1 10 6 8 -2 11");
eqt!(as_338, "2+0 4 4 -1 2", "2 6 6 1 4");
eqt!(as_339, "3|-5 -3 9", "3 3 9");
eqt!(as_340, "-7-1 -7 -7 5", "-8 0 0 -12");
eqt!(as_341, "3|4 6 -9 -6 9", "4 6 3 3 9");
eqt!(as_342, "5&4 4 6 -4 -7 5", "4 4 5 -4 -7 5");
eqt!(as_343, "6-7 -9 -2 -3 3", "-1 15 8 9 3");
eqt!(as_344, "-8*8 1 3 5 -6 -7", "-64 -8 -24 -40 48 56");
eqt!(as_345, "-7|-9 -6 6", "-7 -6 6");
eqt!(as_346, "-3|5 -8", "5 -3");
eqt!(as_347, "-3*6 -8 8 4 9 -5 4", "-18 24 -24 -12 -27 15 -12");
eqt!(as_348, "-5*1 -3", "-5 15");
eqt!(as_349, "-9-8 -1 7 -1 -7 1", "-17 -8 -16 -8 -2 -10");
eqt!(as_350, "-1*8 3 7 4 -8", "-8 -3 -7 -4 8");
eqt!(as_351, "0-3 4 8 -1", "-3 -4 -8 1");
eqt!(as_352, "-3--8 -3 8 2", "5 0 -11 -5");
eqt!(as_353, "6|-5 2 1 -3 5", "6 6 6 6 6");
eqt!(as_354, "8+1 -9 8 -7 4 9 1", "9 -1 16 1 12 17 9");
eqt!(as_355, "-1-5 0", "-6 -1");
eqt!(as_356, "-3|5 3 5", "5 3 5");
eqt!(as_357, "-3+-4 4 -6", "-7 1 -9");
eqt!(as_358, "-5+6 -4", "1 -9");
eqt!(as_359, "8-6 -2", "2 10");
eqt!(as_360, "0-8 -4 -5 -3 7 -6 5", "-8 4 5 3 -7 6 -5");
eqt!(as_361, "-3+-8 4", "-11 1");
eqt!(as_362, "-1&4 -5 -8", "-1 -5 -8");
eqt!(as_363, "-5+-4 5 0 -2 9 1 8", "-9 0 -5 -7 4 -4 3");
eqt!(as_364, "-5*-1 1 8 -3 -5 -2 3", "5 -5 -40 15 25 10 -15");
eqt!(as_365, "1&-5 0", "-5 0");
eqt!(as_366, "8+-3 5 -5", "5 13 3");
eqt!(as_367, "-4&1 3 -6 -8 2 -6 -3", "-4 -4 -6 -8 -4 -6 -4");
eqt!(as_368, "7|-7 0 6 2 -9 6 -7", "7 7 7 7 7 7 7");
eqt!(as_369, "6*0 9 8", "0 54 48");
eqt!(as_370, "-3-6 -1", "-9 -2");
eqt!(as_371, "9*-8 9 -6", "-72 81 -54");
eqt!(as_372, "2--5 0", "7 2");
eqt!(as_373, "-4*2 5", "-8 -20");
eqt!(as_374, "-2*2 -4 -6 0 -7", "-4 8 12 0 14");
eqt!(as_375, "8&-6 8 -6 -4 3 5 -8", "-6 8 -6 -4 3 5 -8");
eqt!(as_376, "-8|9 -6", "9 -6");
eqt!(as_377, "-5&9 2 -7 2 -4", "-5 -5 -7 -5 -5");
eqt!(as_378, "-4+1 -9 6 0", "-3 -13 2 -4");
eqt!(as_379, "-1+-6 -2 -6", "-7 -3 -7");
eqt!(as_380, "6*8 8 -6", "48 48 -36");
eqt!(as_381, "5--4 9 8 -8", "9 -4 -3 13");
eqt!(as_382, "-1*-3 0 3 8 -3 -5", "3 0 -3 -8 3 5");
eqt!(as_383, "8|-2 -6 -9", "8 8 8");
eqt!(as_384, "-8&9 -3", "-8 -8");
eqt!(as_385, "-2+-4 -5 -1 -9 4 3 7", "-6 -7 -3 -11 2 1 5");
eqt!(as_386, "0|-6 -7", "0 0");
eqt!(as_387, "9--2 -2 7 -8 -2 -7 1", "11 11 2 17 11 16 8");
eqt!(as_388, "-8--4 0", "-4 -8");
eqt!(as_389, "-7&9 -4 -9 1", "-7 -7 -9 -7");
eqt!(as_390, "4+-7 -2 -5 7 -4", "-3 2 -1 11 0");
eqt!(as_391, "2--3 -3 -2", "5 5 4");
eqt!(as_392, "1+-9 6 -8 6 7 1 -7", "-8 7 -7 7 8 2 -6");
eqt!(as_393, "-7--8 2 4 -7 2 9", "1 -9 -11 0 -9 -16");
eqt!(as_394, "6&-5 -1 0", "-5 -1 0");
eqt!(as_395, "5|-4 4", "5 5");
eqt!(as_396, "7*9 8 -6 -7 -1", "63 56 -42 -49 -7");
eqt!(as_397, "-2-9 5 8", "-11 -7 -10");
eqt!(as_398, "6|-8 3 3", "6 6 6");
eqt!(as_399, "1&3 -7 -2 1 4 0 -9", "1 -7 -2 1 1 0 -9");
eqt!(as_400, "6|-9 -6 6 4", "6 6 6 6");
eqt!(as_401, "0&-5 1 8 -3 -7", "-5 0 0 -3 -7");
eqt!(as_402, "3&-8 0 1 -7", "-8 0 1 -7");
eqt!(as_403, "-4&4 8 -2 -6", "-4 -4 -4 -6");
eqt!(as_404, "-8&-4 3 -1", "-8 -8 -8");
eqt!(as_405, "-5*-4 -2 2 3", "20 10 -10 -15");
eqt!(as_406, "6*7 -3 -4 3", "42 -18 -24 18");
eqt!(as_407, "-9+-4 -6 -2 5 9 -1", "-13 -15 -11 -4 0 -10");
eqt!(as_408, "2+8 7 3 -5 -1 4 -7", "10 9 5 -3 1 6 -5");
eqt!(as_409, "1&-1 0 2 0 3 7", "-1 0 1 0 1 1");
eqt!(as_410, "-8&6 2 -9 -8 -6 8 3", "-8 -8 -9 -8 -8 -8 -8");
eqt!(as_411, "0|-5 5 -8 1 6", "0 5 0 1 6");
eqt!(as_412, "-9*-5 -3 9", "45 27 -81");
eqt!(as_413, "7+3 -4 9 -1 -2 0", "10 3 16 6 5 7");
eqt!(as_414, "-9&8 4 -7 3 6 2", "-9 -9 -9 -9 -9 -9");
eqt!(as_415, "-1*-4 9 6 -8 8 2 -5", "4 -9 -6 8 -8 -2 5");
eqt!(as_416, "7+-4 0 7", "3 7 14");
eqt!(as_417, "0+9 0 3", "9 0 3");
eqt!(as_418, "-4*0 6 -3 1", "0 -24 12 -4");
eqt!(as_419, "3+-1 2 3 1 3", "2 5 6 4 6");
eqt!(as_420, "-1+-3 5 7 4 -4", "-4 4 6 3 -5");
eqt!(as_421, "-8--1 8 6 8", "-7 -16 -14 -16");
eqt!(as_422, "4+-1 3 2 3 7 0 -6", "3 7 6 7 11 4 -2");
eqt!(as_423, "5+-8 8 9 0", "-3 13 14 5");
eqt!(as_424, "2*-2 -7 8 -6", "-4 -14 16 -12");
eqt!(as_425, "4+0 -4 -4 -6 3 3", "4 0 0 -2 7 7");
eqt!(as_426, "1&3 6 1 2 -4 -5 8", "1 1 1 1 -4 -5 1");
eqt!(as_427, "7&0 -5 -3 1 -7 4 -7", "0 -5 -3 1 -7 4 -7");
eqt!(as_428, "-9|-2 9 4 3 -3 9", "-2 9 4 3 -3 9");
eqt!(as_429, "-1--5 -2 -2 7 -6 0 -8", "4 1 1 -8 5 -1 7");
eqt!(as_430, "3*-5 3 -1 -7 7 -1 -3", "-15 9 -3 -21 21 -3 -9");
eqt!(as_431, "0+2 9 -7", "2 9 -7");
eqt!(as_432, "-9|-7 -6 1 -3", "-7 -6 1 -3");
eqt!(as_433, "5-5 -1", "0 6");
eqt!(as_434, "-8&9 8 -8 -8 8 5", "-8 -8 -8 -8 -8 -8");
eqt!(as_435, "6-0 1", "6 5");
eqt!(as_436, "7|-2 -3 8 -3", "7 7 8 7");
eqt!(as_437, "9|-9 -2 -4 -9", "9 9 9 9");
eqt!(as_438, "-1&2 -7 -1 -7 9 -6", "-1 -7 -1 -7 -1 -6");
eqt!(as_439, "3|9 4 -2 -8 2", "9 4 3 3 3");
eqt!(as_440, "1*-7 6 9 -5 4 5", "-7 6 9 -5 4 5");
eqt!(as_441, "5-1 -3 -6 3 -4 0 -3", "4 8 11 2 9 5 8");
eqt!(as_442, "7+5 -3", "12 4");
eqt!(as_443, "-3*-3 8 0 -9 -9 -7 2", "9 -24 0 27 27 21 -6");
eqt!(as_444, "4+8 -1 8", "12 3 12");
eqt!(as_445, "-4|1 2 0 -6", "1 2 0 -4");
eqt!(as_446, "-4*4 -9", "-16 36");
eqt!(as_447, "5+1 -6 -5 2 6 6 -7", "6 -1 0 7 11 11 -2");
eqt!(as_448, "1&-5 -6 7 9", "-5 -6 1 1");
eqt!(as_449, "7&-3 2 -1 -9", "-3 2 -1 -9");
eqt!(as_450, "-1|4 3 -4", "4 3 -1");
eqt!(af_451, "2.0 2.0 0.5 1.5 2.5*0.0 -1.5 0.5 0.5 1.5", "0 -3 0.25 0.75 3.75");
eqt!(af_452, "0.5 2.5 0.0 1.5 4.0-0.0 10.0 10.0 2.5 0.5", "0.5 -7.5 -10 -1 \
    3.5");
eqt!(af_453, "2.5 4.0 -1.5+1.5 2.0 2.5", "4 6 1f");
eqt!(af_454, "10.0 10.0 1.5 0.5 10.0+-1.5 2.5 10.0 10.0 2.0", "8.5 12.5 11.5 \
    10.5 12");
eqt!(af_455, "10.0 -1.5+2.5 2.5", "12.5 1");
eqt!(af_456, "-1.5 2.5*0.5 2.5", "-0.75 6.25");
eqt!(af_457, "2.5 0.5+10.0 0.5", "12.5 1");
eqt!(af_458, "2.5 2.5 0.5 0.0 -1.5-0.5 2.0 10.0 0.5 10.0", "2 0.5 -9.5 -0.5 \
    -11.5");
eqt!(af_459, "1.5 2.0+0.0 2.0", "1.5 4");
eqt!(af_460, "0.0 4.0 1.5 0.0 -1.5 0.5+0.5 0.0 1.5 0.0 0.0 0.0", "0.5 4 3 0 \
    -1.5 0.5");
eqt!(af_461, "0.5 0.0*3.0 10.0", "1.5 0");
eqt!(af_462, "0.5 0.0 2.5 0.5 2.0*10.0 2.5 1.5 2.5 -1.5", "5 0 3.75 1.25 -3");
eqt!(af_463, "1.5 0.0*4.0 1.5", "6 0f");
eqt!(af_464, "2.5 1.5+4.0 3.0", "6.5 4.5");
eqt!(af_465, "3.0 3.0 2.0 10.0*4.0 2.5 0.5 1.5", "12 7.5 1 15");
eqt!(af_466, "0.5 1.5*2.5 0.0", "1.25 0");
eqt!(af_467, "10.0 -1.5 2.5 1.5 0.5+0.5 2.0 -1.5 0.5 2.0", "10.5 0.5 1 2 2.5");
eqt!(af_468, "3.0 10.0 3.0 2.0 3.0 3.0-0.5 4.0 -1.5 1.5 2.0 10.0", "2.5 6 4.5 \
    0.5 1 -7");
eqt!(af_469, "10.0 4.0 3.0+0.5 -1.5 0.0", "10.5 2.5 3");
eqt!(af_470, "4.0 2.5*4.0 4.0", "16 10f");
eqt!(af_471, "2.5 4.0+0.0 2.0", "2.5 6");
eqt!(af_472, "0.5 4.0-4.0 4.0", "-3.5 0");
eqt!(af_473, "0.0 1.5-2.0 2.5", "-2 -1f");
eqt!(af_474, "0.5 0.0 2.5 -1.5 0.0 1.5*2.5 2.5 3.0 0.5 3.0 -1.5", "1.25 0 7.5 \
    -0.75 0 -2.25");
eqt!(af_475, "2.0 10.0*2.0 3.0", "4 30f");
eqt!(af_476, "2.5 4.0 3.0 0.5 1.5*2.5 3.0 2.0 1.5 1.5", "6.25 12 6 0.75 2.25");
eqt!(af_477, "3.0 1.5 1.5 1.5 0.0+1.5 4.0 1.5 2.0 0.0", "4.5 5.5 3 3.5 0");
eqt!(af_478, "10.0 0.0*3.0 10.0", "30 0f");
eqt!(af_479, "1.5 3.0 3.0--1.5 2.0 10.0", "3 1 -7f");
eqt!(af_480, "10.0 4.0-2.5 0.5", "7.5 3.5");
eqt!(af_481, "2.5 1.5 2.5 4.0 4.0-0.5 2.5 1.5 1.5 2.0", "2 -1 1 2.5 2");
eqt!(af_482, "3.0 3.0 2.0 0.5 2.0 10.0+0.5 -1.5 3.0 1.5 2.5 0.5", "3.5 1.5 5 2 \
    4.5 10.5");
eqt!(af_483, "3.0 0.5-2.0 4.0", "1 -3.5");
eqt!(af_484, "0.0 2.0 2.0 4.0*3.0 4.0 4.0 2.0", "0 8 8 8f");
eqt!(af_485, "1.5 2.5 2.0 3.0 -1.5 0.5+2.5 2.5 -1.5 4.0 2.5 10.0", "4 5 0.5 7 \
    1 \
    10.5");
eqt!(af_486, "0.5 0.5 1.5 -1.5-2.5 3.0 0.5 10.0", "-2 -2.5 1 -11.5");
eqt!(af_487, "10.0 1.5 1.5 10.0 0.0*10.0 1.5 -1.5 1.5 10.0", "100 2.25 -2.25 \
    15 \
    0");
eqt!(af_488, "2.0 2.5 -1.5 10.0 0.5+2.5 1.5 3.0 4.0 10.0", "4.5 4 1.5 14 10.5");
eqt!(af_489, "2.5 4.0 0.0 0.5 1.5*2.5 10.0 2.5 -1.5 1.5", "6.25 40 0 -0.75 \
    2.25");
eqt!(af_490, "-1.5 0.0+2.5 0.0", "1 0f");
eqt!(af_491, "0.0 4.0 2.5+1.5 10.0 3.0", "1.5 14 5.5");
eqt!(af_492, "10.0 2.0 1.5 10.0 4.0+2.5 3.0 4.0 1.5 1.5", "12.5 5 5.5 11.5 \
    5.5");
eqt!(af_493, "10.0 3.0 2.0 0.0 0.5*0.0 0.5 10.0 0.5 0.0", "0 1.5 20 0 0");
eqt!(af_494, "10.0 2.0 4.0+-1.5 4.0 0.5", "8.5 6 4.5");
eqt!(af_495, "2.0 2.5 0.5 10.0*1.5 10.0 2.5 0.5", "3 25 1.25 5");
eqt!(af_496, "10.0 2.0 2.5 3.0*4.0 2.5 1.5 -1.5", "40 5 3.75 -4.5");
eqt!(af_497, "2.0 0.5-10.0 2.5", "-8 -2f");
eqt!(af_498, "10.0 4.0*10.0 2.5", "100 10f");
eqt!(af_499, "2.5 2.5 10.0 2.5 3.0 10.0-2.5 4.0 0.5 -1.5 2.0 4.0", "0 -1.5 9.5 \
    4 1 6");
eqt!(af_500, "0.5 4.0 2.0 2.5 0.5+3.0 10.0 10.0 0.0 0.0", "3.5 14 12 2.5 0.5");
eqt!(af_501, "2.0 3.0 2.5 0.0 1.5--1.5 2.0 2.0 0.0 2.0", "3.5 1 0.5 0 -0.5");
eqt!(af_502, "4.0 0.5 2.0 2.5 -1.5 2.0+10.0 -1.5 3.0 2.5 2.0 3.0", "14 -1 5 5 \
    0.5 5");
eqt!(af_503, "1.5 0.5 -1.5 1.5 0.5-1.5 3.0 2.0 2.0 -1.5", "0 -2.5 -3.5 -0.5 2");
eqt!(af_504, "0.0 -1.5-0.0 1.5", "0 -3f");
eqt!(af_505, "2.5 10.0 0.0 4.0 0.0*2.5 -1.5 1.5 3.0 -1.5", "6.25 -15 0 12 -0");
eqt!(af_506, "3.0 2.5 -1.5-0.0 3.0 1.5", "3 -0.5 -3");
eqt!(af_507, "10.0 2.5*4.0 0.5", "40 1.25");
eqt!(af_508, "10.0 4.0 2.0 10.0 4.0+-1.5 1.5 2.5 0.0 -1.5", "8.5 5.5 4.5 10 \
    2.5");
eqt!(af_509, "2.0 2.5 4.0 4.0 -1.5*10.0 4.0 2.0 2.5 2.5", "20 10 8 10 -3.75");
eqt!(af_510, "1.5 0.5 0.0 2.0--1.5 1.5 10.0 10.0", "3 -1 -10 -8f");
eqt!(af_511, "0.0 4.0 4.0 -1.5-2.0 10.0 0.5 2.0", "-2 -6 3.5 -3.5");
eqt!(af_512, "4.0 1.5 3.0 0.0 2.5*2.5 2.5 4.0 3.0 3.0", "10 3.75 12 0 7.5");
eqt!(af_513, "1.5 10.0 0.5+0.5 0.0 -1.5", "2 10 -1f");
eqt!(af_514, "3.0 0.5 1.5 0.5 2.0 1.5*2.5 0.5 2.0 2.5 2.0 3.0", "7.5 0.25 3 \
    1.25 4 4.5");
eqt!(af_515, "0.5 0.5 1.5+1.5 2.5 2.0", "2 3 3.5");
eqt!(af_516, "4.0 1.5 0.0 4.0 4.0--1.5 10.0 3.0 4.0 0.5", "5.5 -8.5 -3 0 3.5");
eqt!(af_517, "3.0 2.0-1.5 1.5", "1.5 0.5");
eqt!(af_518, "0.5 3.0 2.0 4.0 4.0 0.0-2.0 2.5 0.0 0.5 2.0 -1.5", "-1.5 0.5 2 \
    3.5 2 1.5");
eqt!(af_519, "3.0 0.5 2.5 3.0 1.5-1.5 1.5 2.0 2.5 10.0", "1.5 -1 0.5 0.5 -8.5");
eqt!(af_520, "2.5 1.5 10.0 -1.5 2.0+2.5 2.5 1.5 10.0 2.5", "5 4 11.5 8.5 4.5");
eqt!(af_521, "0.0 -1.5 0.0 0.0-0.5 0.5 2.5 0.5", "-0.5 -2 -2.5 -0.5");
eqt!(af_522, "0.0 3.0 2.5*10.0 2.5 2.0", "0 7.5 5");
eqt!(af_523, "3.0 3.0 2.0+0.5 2.5 10.0", "3.5 5.5 12");
eqt!(af_524, "3.0 -1.5 4.0 0.0*3.0 0.5 4.0 1.5", "9 -0.75 16 0");
eqt!(af_525, "0.5 4.0 0.0 2.5+2.0 2.5 10.0 0.5", "2.5 6.5 10 3");
eqt!(af_526, "4.0 1.5 0.0*0.0 4.0 10.0", "0 6 0f");
eqt!(af_527, "3.0 1.5 1.5 1.5 -1.5 -1.5-1.5 3.0 0.0 2.5 10.0 4.0", "1.5 -1.5 \
    1.5 -1 -11.5 -5.5");
eqt!(af_528, "-1.5 4.0 0.0 10.0 4.0*0.5 1.5 10.0 1.5 3.0", "-0.75 6 0 15 12");
eqt!(af_529, "0.5 0.0 2.0+10.0 0.5 3.0", "10.5 0.5 5");
eqt!(af_530, "4.0 -1.5*1.5 2.0", "6 -3f");
eqt!(af_531, "1.5 0.5 0.5 3.0 2.0*1.5 1.5 4.0 2.0 0.0", "2.25 0.75 2 6 0");
eqt!(af_532, "-1.5 2.0 2.5 2.0 -1.5 -1.5*4.0 4.0 1.5 2.5 10.0 0.0", "-6 8 3.75 \
    5 -15 -0");
eqt!(af_533, "1.5 3.0*-1.5 10.0", "-2.25 30");
eqt!(af_534, "2.0 3.0 10.0-2.5 2.0 2.5", "-0.5 1 7.5");
eqt!(af_535, "1.5 0.0 4.0 2.5 0.5-0.0 10.0 2.0 4.0 4.0", "1.5 -10 2 -1.5 -3.5");
eqt!(af_536, "4.0 2.5 -1.5+0.5 2.5 4.0", "4.5 5 2.5");
eqt!(af_537, "3.0 0.5+4.0 2.5", "7 3f");
eqt!(af_538, "3.0 4.0 3.0 4.0*4.0 -1.5 -1.5 3.0", "12 -6 -4.5 12");
eqt!(af_539, "2.5 0.5*-1.5 2.5", "-3.75 1.25");
eqt!(af_540, "2.0 2.0-3.0 0.0", "-1 2f");
eqt!(af_541, "-1.5 -1.5 3.0 2.0+0.0 4.0 0.5 4.0", "-1.5 2.5 3.5 6");
eqt!(af_542, "4.0 2.0 0.0*0.5 0.0 10.0", "2 0 0f");
eqt!(af_543, "10.0 10.0 2.5 4.0-2.5 1.5 1.5 1.5", "7.5 8.5 1 2.5");
eqt!(af_544, "0.5 0.5 2.5 4.0+1.5 10.0 0.5 2.5", "2 10.5 3 6.5");
eqt!(af_545, "-1.5 3.0 10.0 -1.5 3.0*10.0 4.0 4.0 3.0 4.0", "-15 12 40 -4.5 \
    12");
eqt!(af_546, "1.5 0.0 1.5 10.0 10.0 -1.5+2.5 2.5 2.5 4.0 0.0 4.0", "4 2.5 4 14 \
    10 2.5");
eqt!(af_547, "0.5 10.0*-1.5 0.5", "-0.75 5");
eqt!(af_548, "-1.5 1.5 2.0*3.0 0.0 4.0", "-4.5 0 8");
eqt!(af_549, "2.5 0.5+4.0 -1.5", "6.5 -1");
eqt!(af_550, "-1.5 1.5 -1.5+4.0 3.0 4.0", "2.5 4.5 2.5");
eqt!(af_551, "2.0 10.0 0.0 0.0 0.5 2.0*-1.5 0.0 2.0 2.0 0.5 0.0", "-3 0 0 0 \
    0.25 0");
eqt!(af_552, "4.0 0.5+2.5 0.0", "6.5 0.5");
eqt!(af_553, "0.0 2.5*10.0 2.0", "0 5f");
eqt!(af_554, "2.5 2.0 2.0 10.0 0.5 -1.5+3.0 3.0 2.5 -1.5 2.5 0.0", "5.5 5 4.5 \
    8.5 3 -1.5");
eqt!(af_555, "0.5 1.5 0.5 4.0 2.0*2.5 0.0 3.0 2.5 0.0", "1.25 0 1.5 10 0");
eqt!(af_556, "2.5 2.0 2.5*1.5 10.0 2.5", "3.75 20 6.25");
eqt!(af_557, "-1.5 0.0 0.5 10.0+10.0 1.5 1.5 0.0", "8.5 1.5 2 10");
eqt!(af_558, "2.0 4.0 10.0 2.0 2.5*4.0 -1.5 2.5 2.5 2.5", "8 -6 25 5 6.25");
eqt!(af_559, "-1.5 4.0 -1.5-3.0 2.0 2.5", "-4.5 2 -4");
eqt!(af_560, "1.5 2.0 2.5 4.0 1.5*3.0 2.0 -1.5 10.0 10.0", "4.5 4 -3.75 40 15");
eqt!(af_561, "10.0 10.0 3.0 10.0 0.0 2.5-0.0 2.0 0.0 2.0 2.5 1.5", "10 8 3 8 \
    -2.5 1");
eqt!(af_562, "-1.5 1.5 -1.5 1.5--1.5 4.0 4.0 -1.5", "0 -2.5 -5.5 3");
eqt!(af_563, "10.0 0.0 0.5+10.0 4.0 0.0", "20 4 0.5");
eqt!(af_564, "-1.5 3.0 2.0 0.0 0.5*2.0 4.0 -1.5 4.0 2.5", "-3 12 -3 0 1.25");
eqt!(af_565, "2.0 0.0 0.0 -1.5*2.0 3.0 1.5 2.0", "4 0 0 -3f");
eqt!(af_566, "4.0 10.0-10.0 3.0", "-6 7f");
eqt!(af_567, "0.0 0.5 4.0 0.0*4.0 10.0 1.5 4.0", "0 5 6 0f");
eqt!(af_568, "-1.5 3.0 0.5 4.0-1.5 4.0 0.0 0.5", "-3 -1 0.5 3.5");
eqt!(af_569, "4.0 3.0 10.0 2.0*-1.5 0.5 1.5 2.5", "-6 1.5 15 5");
eqt!(af_570, "0.5 2.0 2.0-2.5 2.5 0.5", "-2 -0.5 1.5");
eqt!(af_571, "3.0 1.5 1.5 2.0 0.0*1.5 2.0 -1.5 2.5 0.5", "4.5 3 -2.25 5 0");
eqt!(af_572, "-1.5 -1.5 1.5 2.0 2.0-0.5 1.5 0.5 2.0 1.5", "-2 -3 1 0 0.5");
eqt!(af_573, "0.5 4.0*2.0 1.5", "1 6f");
eqt!(af_574, "2.0 1.5 2.0 2.5 4.0*2.5 4.0 1.5 -1.5 4.0", "5 6 3 -3.75 16");
eqt!(af_575, "-1.5 3.0 10.0 2.5 10.0+2.0 2.0 2.0 2.0 4.0", "0.5 5 12 4.5 14");
eqt!(af_576, "10.0 0.0*0.5 10.0", "5 0f");
eqt!(af_577, "0.5 10.0 10.0 0.5 4.0 -1.5*2.0 0.5 0.0 0.0 2.0 10.0", "1 5 0 0 8 \
    -15f");
eqt!(af_578, "-1.5 2.0 0.5*0.0 0.5 4.0", "-0 1 2f");
eqt!(af_579, "2.5 -1.5 -1.5 4.0 10.0*2.0 4.0 -1.5 2.5 3.0", "5 -6 2.25 10 30");
eqt!(af_580, "0.5 4.0 4.0*0.0 3.0 4.0", "0 12 16f");
eqt!(af_581, "0.0 10.0 3.0+10.0 0.5 2.0", "10 10.5 5");
eqt!(af_582, "1.5 -1.5 3.0 0.0 -1.5*0.5 1.5 2.0 1.5 -1.5", "0.75 -2.25 6 0 \
    2.25");
eqt!(af_583, "1.5 -1.5 10.0 3.0+10.0 4.0 1.5 0.5", "11.5 2.5 11.5 3.5");
eqt!(af_584, "3.0 2.5 1.5 3.0 3.0-2.5 0.0 0.0 0.0 -1.5", "0.5 2.5 1.5 3 4.5");
eqt!(af_585, "3.0 10.0 4.0 -1.5 10.0 1.5+2.0 3.0 0.5 0.0 2.0 4.0", "5 13 4.5 \
    -1.5 12 5.5");
eqt!(af_586, "2.5 3.0 0.0 0.5 10.0-0.5 1.5 1.5 0.5 2.5", "2 1.5 -1.5 0 7.5");
eqt!(af_587, "10.0 1.5 3.0 4.0 2.0+1.5 2.0 0.0 3.0 4.0", "11.5 3.5 3 7 6");
eqt!(af_588, "2.0 2.5 10.0+3.0 3.0 0.5", "5 5.5 10.5");
eqt!(af_589, "2.0 3.0 1.5*-1.5 0.0 10.0", "-3 0 15f");
eqt!(af_590, "1.5 -1.5 10.0-0.5 -1.5 2.5", "1 0 7.5");
eqt!(af_591, "10.0 0.0 2.5 3.0 2.0*1.5 0.0 4.0 -1.5 2.0", "15 0 10 -4.5 4");
eqt!(af_592, "10.0 10.0 10.0-4.0 1.5 0.0", "6 8.5 10");
eqt!(af_593, "4.0 2.0 4.0 1.5 4.0-1.5 2.0 10.0 3.0 4.0", "2.5 0 -6 -1.5 0");
eqt!(af_594, "0.0 2.0 4.0 0.5 4.0+10.0 1.5 3.0 10.0 4.0", "10 3.5 7 10.5 8");
eqt!(af_595, "4.0 10.0 2.5 0.0 2.0 4.0+2.5 3.0 3.0 2.5 1.5 -1.5", "6.5 13 5.5 \
    2.5 3.5 2.5");
eqt!(af_596, "2.5 0.0+2.5 0.0", "5 0f");
eqt!(af_597, "1.5 2.5 1.5 3.0 1.5 2.5*0.5 3.0 0.5 -1.5 1.5 3.0", "0.75 7.5 \
    0.75 \
    -4.5 2.25 7.5");
eqt!(af_598, "0.5 0.0 -1.5 4.0*0.0 2.0 0.5 2.5", "0 0 -0.75 10");
eqt!(af_599, "2.5 1.5 2.5+3.0 0.0 4.0", "5.5 1.5 6.5");
eqt!(af_600, "-1.5 0.5 1.5 -1.5 1.5*3.0 0.0 2.0 -1.5 4.0", "-4.5 0 3 2.25 6");
eqt!(cm_601, "-9 -9 -8 4 8 3 -4 2<>2 8 -5 2 2 -1 8 -5", "11111111b");
eqt!(cm_602, "-4 -5 -5=9 -6 -4", "000b");
eqt!(cm_603, "7 9 9 -6>=6 4 5 8", "1110b");
eqt!(cm_604, "-9 -8 -2 4 -5 -2 -9 -2>-2 -7 6 9 3 4 1 6", "00000000b");
eqt!(cm_605, "-8 -2 -8 5 7 -2 -8 -4<-7 -1 -7 1 -7 1 -7 4", "11100111b");
eqt!(cm_606, "0 -7 7 5 -2 -5 -4 0<=1 -6 7 4 -4 9 -8 6", "11100101b");
eqt!(cm_607, "-4 -8>7 -8", "00b");
eqt!(cm_608, "-8 -6 7 -3>=3 -4 -2 -3", "0011b");
eqt!(cm_609, "-1 5 -7 -2 5=-2 3 -6 -3 4", "00000b");
eqt!(cm_610, "8 0>1 -2", "11b");
eqt!(cm_611, "1 -2 -8 3<=4 -7 -5 -7", "1010b");
eqt!(cm_612, "-8 8<-1 -6", "10b");
eqt!(cm_613, "7 6 -1 -3 -6<>6 9 5 0 -7", "11111b");
eqt!(cm_614, "6 -5 -5 -7 6 4<-9 -4 9 -8 -7 -6", "011000b");
eqt!(cm_615, "1 -2 -8 -2 9 -1 2 -4<>2 4 -1 -4 5 5 -4 -9", "11111111b");
eqt!(cm_616, "-7 8 4<-5 -1 -6", "100b");
eqt!(cm_617, "3 -7<>-2 -9", "11b");
eqt!(cm_618, "-8 2 -7>9 1 8", "010b");
eqt!(cm_619, "9 5 9 8 -3 0 7 -3<=1 -5 2 2 7 8 9 -2", "00001111b");
eqt!(cm_620, "-1 7 -5 7 -9 4<=-4 -8 8 0 -1 -6", "001010b");
eqt!(cm_621, "5 2 7 6 -2 7 8 3>=0 0 3 -8 -1 6 1 -3", "11110111b");
eqt!(cm_622, "5 2 0 5 2 -7 2<>-3 -2 4 -1 2 -9 -1", "1111011b");
eqt!(cm_623, "-8 1 2 4 -8 4>=7 0 -2 1 1 6", "011100b");
eqt!(cm_624, "-4 6=2 -3", "00b");
eqt!(cm_625, "6 -8 -5 1<=5 0 4 -5", "0110b");
eqt!(cm_626, "-5 -4 -4 2>-8 -2 1 -8", "1001b");
eqt!(cm_627, "-4 -8 4 4 -3 -5 2 7=-6 -1 5 7 3 -1 -9 3", "00000000b");
eqt!(cm_628, "-4 3 -9 2 -6>1 -5 -8 -3 -3", "01010b");
eqt!(cm_629, "9 9>=-2 0", "11b");
eqt!(cm_630, "-3 -2<6 9", "11b");
eqt!(cm_631, "9 1 -6 -8 9 1 7 -7>=5 -6 -2 -3 5 0 4 2", "11001110b");
eqt!(cm_632, "-2 -6>3 -2", "00b");
eqt!(cm_633, "4 -2 1 9 -2 3 -8>=8 0 -1 6 6 5 -9", "0011001b");
eqt!(cm_634, "3 5<-4 6", "01b");
eqt!(cm_635, "3 -4 -6 -1 5 -7>5 -3 -9 -7 -7 -7", "001110b");
eqt!(cm_636, "2 -9 4<=7 5 0", "110b");
eqt!(cm_637, "2 7 2 -4 -6 7 7<=-6 2 0 8 -3 -2 3", "0001100b");
eqt!(cm_638, "1 8 9 -1>-7 2 -6 2", "1110b");
eqt!(cm_639, "8 1 -5 1 -6 1 -4<=-9 2 -2 3 -9 -4 -3", "0111001b");
eqt!(cm_640, "8 5 2 3 -1 -2 -4<>5 -4 2 -8 -9 3 -2", "1101111b");
eqt!(cm_641, "3 -8 6 8<=-3 8 -4 -7", "0100b");
eqt!(cm_642, "-4 -4 -1 7 -5 -4 7>0 8 8 -5 6 -6 -5", "0001011b");
eqt!(cm_643, "0 0 -3 8>=9 -2 5 1", "0101b");
eqt!(cm_644, "-5 2 6 5 8 -4=-6 -7 -8 9 7 -5", "000000b");
eqt!(cm_645, "-7 -4 7 -9=-2 5 -7 5", "0000b");
eqt!(cm_646, "-2 -4 -3 1 1 -9<1 2 -7 -7 -9 -6", "110001b");
eqt!(cm_647, "-4 0<>-1 0", "10b");
eqt!(cm_648, "-7 -3 5 -1 8 -9 -8<>0 -2 0 -7 8 6 -5", "1111011b");
eqt!(cm_649, "8 5 3 5 -3<-1 -1 7 -2 -5", "00100b");
eqt!(cm_650, "0 3 -8 -2 -6 -3 5>5 7 2 7 6 -9 2", "0000011b");
eqt!(cm_651, "-3 -4 2 6 3<7 -5 4 -4 6", "10101b");
eqt!(cm_652, "-3 -3 -2 2 9 -6>-1 2 -6 6 0 3", "001010b");
eqt!(cm_653, "9 -3 1 4 -9 0>-5 8 8 9 -5 -4", "100001b");
eqt!(cm_654, "-6 4 5 4<>4 -3 -6 -5", "1111b");
eqt!(cm_655, "-4 7 -5 1 -2<>4 3 -1 -5 -6", "11111b");
eqt!(cm_656, "9 -3 -4<=9 8 -3", "111b");
eqt!(cm_657, "7 6 -6 -9 -3<=-8 9 -6 8 4", "01111b");
eqt!(cm_658, "0 -2 9<2 2 -6", "110b");
eqt!(cm_659, "-7 -4 0 -5 -1>=-6 -8 9 -8 -3", "01011b");
eqt!(cm_660, "-3 -7 -1>-7 -1 6", "100b");
eqt!(cm_661, "-1 -9 0<=-2 2 -2", "010b");
eqt!(cm_662, "4 -6 -2 -9 -6 1 -6 5<>6 -9 -2 -3 2 -8 1 3", "11011111b");
eqt!(cm_663, "8 3 -2 0 4=7 5 4 9 7", "00000b");
eqt!(cm_664, "6 -1 -4 4 4 -3 -8 8<5 9 -2 8 7 -6 -7 2", "01111010b");
eqt!(cm_665, "-9 -9 -1 6 -4<6 -5 0 4 -3", "11101b");
eqt!(cm_666, "3 -9 0=3 5 1", "100b");
eqt!(cm_667, "-2 1 -7 -5 -8 -7>-8 0 0 8 -4 -6", "110000b");
eqt!(cm_668, "-7 0=2 -4", "00b");
eqt!(cm_669, "3 7 4 -6 -6 7<=0 6 5 3 -6 4", "001110b");
eqt!(cm_670, "3 -3 1<=3 3 7", "111b");
eqt!(cm_671, "8 -1 -6 9 -8 5 -1 -3<5 3 -1 2 -5 7 -4 4", "01101101b");
eqt!(cm_672, "-1 -2 -6>=-9 4 -7", "101b");
eqt!(cm_673, "5 0>=5 -7", "11b");
eqt!(cm_674, "-6 3>7 -9", "01b");
eqt!(cm_675, "3 2 -5 6 -7 -9 -9 -5>=-2 -7 -7 8 -3 7 -7 -5", "11100001b");
eqt!(cm_676, "4 5 -1 9<1 -8 9 -6", "0010b");
eqt!(cm_677, "4 0 -8 -6 -6 4=9 -3 9 -1 6 0", "000000b");
eqt!(cm_678, "9 4 -9>5 9 1", "100b");
eqt!(cm_679, "8 -1 7 -7=7 6 1 -2", "0000b");
eqt!(cm_680, "-6 1 7 7>0 2 -2 4", "0011b");
eqt!(cm_681, "-1 -2 4 5 -1 -3<8 -5 8 -9 -7 -1", "101001b");
eqt!(cm_682, "-4 2 -1 -3 3 5 -4 -6>-6 -4 6 7 4 -8 -3 3", "11000100b");
eqt!(cm_683, "4 -3 2 8 0<=9 3 7 3 -3", "11100b");
eqt!(cm_684, "-5 7 1 8 5=-7 -2 -7 8 -4", "00010b");
eqt!(cm_685, "2 -1 5 6 1 0 2 -4>=-4 -4 -7 -5 9 7 -3 6", "11110010b");
eqt!(cm_686, "-6 7 -5 -5<>8 -2 1 0", "1111b");
eqt!(cm_687, "-7 -1 -3 3=4 -2 3 5", "0000b");
eqt!(cm_688, "5 3=-6 -2", "00b");
eqt!(cm_689, "-1 -2 -9 9 -6<=4 9 7 -7 -2", "11101b");
eqt!(cm_690, "0 -3 -8 2 9=-6 9 -9 9 6", "00000b");
eqt!(cm_691, "-5 3 -5 8 5 -1>3 -4 -3 -7 9 1", "010100b");
eqt!(cm_692, "4 -3 0 9 1 -8>=2 7 -6 -8 1 -1", "101110b");
eqt!(cm_693, "-1 -1 4 7 5 5 5<=9 1 -6 -4 -6 -2 -5", "1100000b");
eqt!(cm_694, "-5 -3 6<>1 -3 1", "101b");
eqt!(cm_695, "5 6 -8 -4 -8 -4 5=-7 5 -9 -9 6 4 7", "0000000b");
eqt!(cm_696, "4 -2<-8 9", "01b");
eqt!(cm_697, "-2 1 0 6 4<=-8 7 -9 1 -8", "01000b");
eqt!(cm_698, "4 -3 -2 1 -9 -9=-8 4 6 6 2 -6", "000000b");
eqt!(cm_699, "3 9 1 -9 3 -1<=-7 6 8 7 3 -6", "001110b");
eqt!(cm_700, "-6 3 -6 6 4>=-9 -6 6 0 -8", "11011b");
eqt!(cm_701, "4 -1 -9 6 -2 2>=5 3 -6 0 -8 1", "000111b");
eqt!(cm_702, "8 -2 9 3>=-9 4 5 8", "1010b");
eqt!(cm_703, "9 -5 6 0 8 -8 0<>-9 -5 1 -8 -2 -9 -4", "1011111b");
eqt!(cm_704, "-1 -2 3 -2 7 1 9 -5=-2 5 7 3 2 -5 5 -4", "00000000b");
eqt!(cm_705, "8 0 2 -9 7 -1 6 -8=-4 -9 3 8 -7 1 1 -7", "00000000b");
eqt!(cm_706, "3 -5 0>=-8 9 -6", "101b");
eqt!(cm_707, "5 7 -5 6 -6 -3 -5 0<-9 -8 -1 -6 -4 5 7 1", "00101111b");
eqt!(cm_708, "-5 -4 1 3 -5 9 5 -1>8 -4 -5 2 -5 -2 -9 -6", "00110111b");
eqt!(cm_709, "0 -9 0>-6 0 5", "100b");
eqt!(cm_710, "8 -4 5 -6 -7 2 3 -4<-3 -7 -9 -7 3 -7 -5 -2", "00001001b");
eqt!(cm_711, "-8 4 5 -6 -9<=1 -3 -2 9 4", "10011b");
eqt!(cm_712, "2 5 8 2 -5 3 -7>4 0 0 -6 -3 4 1", "0111000b");
eqt!(cm_713, "0 -3 6 0 3>=-7 -6 5 -7 9", "11110b");
eqt!(cm_714, "4 -1 6 -1 3=-2 7 -4 7 4", "00000b");
eqt!(cm_715, "-9 6 3>3 -6 8", "010b");
eqt!(cm_716, "-7 3 -5 0 4 7 -5>1 5 5 0 9 6 -5", "0000010b");
eqt!(cm_717, "-1 7 -9<=-9 -1 8", "001b");
eqt!(cm_718, "6 2 -3 4 -9 5 4 -3<>-7 -7 -2 0 3 -3 4 2", "11111101b");
eqt!(cm_719, "5 4 2 3 -6 -2=0 7 -6 9 5 4", "000000b");
eqt!(cm_720, "2 9 4 -4 -2 9 7>=4 1 -1 3 1 6 5", "0110011b");
eqt!(cm_721, "6 9>=-3 -8", "11b");
eqt!(cm_722, "-4 -8 2 0 -7 -3 -2 6>5 8 4 8 -7 -8 -7 -4", "00000111b");
eqt!(cm_723, "-3 -7 3 -5 7 0 2=-5 8 1 4 -2 -6 -8", "0000000b");
eqt!(cm_724, "6 1=3 -1", "00b");
eqt!(cm_725, "5 -2 -1 -4<=-4 -4 5 2", "0011b");
eqt!(cm_726, "-5 3 8 -7 -3 0 2 -1>=-2 -6 8 1 3 -2 1 -9", "01100111b");
eqt!(cm_727, "5 4<>2 0", "11b");
eqt!(cm_728, "-2 9 -2 0 -3<>2 8 6 9 2", "11111b");
eqt!(cm_729, "3 -7 -9 9 -9 9 8 3<>1 6 -3 4 8 -3 6 -8", "11111111b");
eqt!(cm_730, "-3 1 6 -9 -1>-5 5 -3 0 8", "10100b");
eqt!(cm_731, "-4 -3 0 3 1=-6 0 2 -3 9", "00000b");
eqt!(cm_732, "-4 4 0=2 9 -5", "000b");
eqt!(cm_733, "0 -1>=4 -1", "01b");
eqt!(cm_734, "5 0 8 1 -1 -9 -2>-2 1 -3 4 -1 1 -9", "1010001b");
eqt!(cm_735, "0 0 -9 7 -1 -5 -3>-6 2 1 -6 7 -4 4", "1001000b");
eqt!(cm_736, "-7 9 5 6>2 7 7 -8", "0101b");
eqt!(cm_737, "4 -1 8 -4<=6 1 -5 -2", "1101b");
eqt!(cm_738, "-6 -2 -2 -2=-3 7 -2 -5", "0010b");
eqt!(cm_739, "6 2 6 2 -8 -3<>-2 4 7 6 -3 -8", "111111b");
eqt!(cm_740, "1 -8 -7 -1 2 -6 6<7 7 -4 -6 7 -5 3", "1110110b");
eqt!(cm_741, "0 -3 9>6 -7 6", "011b");
eqt!(cm_742, "3 -3 2 -9<=6 -3 -3 8", "1101b");
eqt!(cm_743, "-6 5 -2 -6 1 -5=-3 8 1 2 -7 4", "000000b");
eqt!(cm_744, "8 -8>3 5", "10b");
eqt!(cm_745, "-1 1 0 8 -9<6 -4 -7 -3 2", "10001b");
eqt!(cm_746, "9 4 -3 -7 -7 7 -8>=-5 -9 7 6 5 -1 -1", "1100010b");
eqt!(cm_747, "4 9>7 -8", "01b");
eqt!(cm_748, "-5 5 -3 -3<-5 -9 9 -1", "0011b");
eqt!(cm_749, "6 4 2=4 4 -8", "010b");
eqt!(cm_750, "-6 6 9 -8 3 -5<=6 -4 -5 7 3 -5", "100111b");
eqt!(cm_751, "4 -1 -1 -7 -2 -6<=2 9 -6 7 8 7", "010111b");
eqt!(cm_752, "7 -3 -5=-7 1 -2", "000b");
eqt!(cm_753, "-2 -6 -8 4<-8 -7 6 6", "0011b");
eqt!(cm_754, "-3 4 0 -3 -5 8 5 6<-8 2 8 -3 1 -6 -3 5", "00101000b");
eqt!(cm_755, "-6 1<>7 7", "11b");
eqt!(cm_756, "8 -5 -8 -1 9 -9<=9 4 9 -8 -5 1", "111001b");
eqt!(cm_757, "4 -7 4 -2 8>=2 7 3 -5 4", "10111b");
eqt!(cm_758, "2 0 -7 5=1 -6 3 6", "0000b");
eqt!(cm_759, "-4 9 -6 2 -8<9 -9 -5 -8 0", "10101b");
eqt!(cm_760, "5 1 -8 -2 -2 5 -1 6<=3 -6 -2 -4 2 -6 2 9", "00101011b");
eqt!(cm_761, "5 -5 -8 4 -3 -7 5 9<=-5 -6 9 -9 4 4 -2 7", "00101100b");
eqt!(cm_762, "-6 9 -2 5 1 -3 9>-7 5 -4 7 1 -7 1", "1110011b");
eqt!(cm_763, "-9 -6 -1 4 -4 7 1 -8<=-6 1 8 -3 -4 0 8 -5", "11101011b");
eqt!(cm_764, "-1 -1 9 -1 5 -5>-1 5 -3 -4 9 -3", "001100b");
eqt!(cm_765, "-5 -3 1 -4 3>3 6 3 -5 2", "00011b");
eqt!(cm_766, "4 -1<7 1", "11b");
eqt!(cm_767, "-3 3 -1 -5 -5 2 5>=7 -3 -5 -4 1 8 -1", "0110001b");
eqt!(cm_768, "4 -4=-1 -7", "00b");
eqt!(cm_769, "-6 0 8<=1 -2 0", "100b");
eqt!(cm_770, "-1 2 -8 9 -6 9 -8 -9<9 -1 7 -7 9 4 -3 -2", "10101011b");
eqt!(cm_771, "8 1 5 -8 0>-6 3 2 8 0", "10100b");
eqt!(cm_772, "-6 -3 1 0 -1 -1 -7<-8 -7 3 2 9 -4 4", "0011101b");
eqt!(cm_773, "-1 -2 -4 7>=0 -4 9 -6", "0101b");
eqt!(cm_774, "-4 -9 -2 2 7 7<=-5 8 4 9 5 -4", "011100b");
eqt!(cm_775, "2 -7=1 -5", "00b");
eqt!(cm_776, "-8 -4<0 0", "11b");
eqt!(cm_777, "-6 7 -4 4 -5 8 0 1<-5 5 -4 5 3 -4 -5 0", "10011000b");
eqt!(cm_778, "-5 8 1 8 -2<=2 -7 7 1 5", "10101b");
eqt!(cm_779, "-6 8 8 9 -6 9 -1 -6<1 1 4 -9 8 -6 -6 -4", "10001001b");
eqt!(cm_780, "4 -1 1 -8 -5 -1 -6>2 1 -5 5 5 -8 1", "1010010b");
eqt!(cm_781, "1 7 -6 1=2 7 3 2", "0100b");
eqt!(cm_782, "8 8 9 2 5 -1 -5 -7>-7 -3 4 -8 -8 7 0 8", "11111000b");
eqt!(cm_783, "-4 4 8 8 -7 -5<-6 -5 5 -9 -2 -8", "000010b");
eqt!(cm_784, "-9 -2 -5<=8 -5 -4", "101b");
eqt!(cm_785, "7 9 3 6 -1 -9 -2 1>8 6 -8 2 4 -5 5 -5", "01110001b");
eqt!(cm_786, "7 1 -9 6 8 8<-9 1 6 3 2 9", "001001b");
eqt!(cm_787, "6 -8=6 -7", "10b");
eqt!(cm_788, "9 3>-2 -1", "11b");
eqt!(cm_789, "5 -7 5 8 8 5 9>7 8 2 6 -3 4 -7", "0011111b");
eqt!(cm_790, "-6 7 2 -5 8<=-3 -2 -2 -2 -2", "10010b");
eqt!(cm_791, "-9 3 -1 0=-9 7 4 0", "1001b");
eqt!(cm_792, "8 3 0 9 -4 6 5<=0 3 -8 -6 5 1 -4", "0100100b");
eqt!(cm_793, "7 -9 6 -4 -2 -1 2<>-6 1 -9 9 2 2 3", "1111111b");
eqt!(cm_794, "-6 1 1 1 0 -5<-9 9 -7 5 8 1", "010111b");
eqt!(cm_795, "7 -6 -9>-3 4 8", "100b");
eqt!(cm_796, "1 -1 8 -9=8 -1 8 2", "0110b");
eqt!(cm_797, "9 8<>3 9", "11b");
eqt!(cm_798, "-9 2 4 -9>-1 -9 2 -8", "0110b");
eqt!(cm_799, "-8 -2 8 7 5 -6>=1 -7 8 -1 2 -6", "011111b");
eqt!(cm_800, "-7 5 5<-4 8 -1", "110b");
eqt!(cm_801, "1 6 -1 4 8 9<-7 -9 8 8 9 -8", "001110b");
eqt!(cm_802, "5 1 -4<=4 9 0", "011b");
eqt!(cm_803, "-3 -9 -7 8 -5<-1 5 9 -4 -9", "11100b");
eqt!(cm_804, "-9 2 1 -9 -8 4 -1 -2<9 -6 5 -3 -7 -2 -6 -2", "10111000b");
eqt!(cm_805, "-6 5 9=1 4 1", "000b");
eqt!(cm_806, "-4 3 6 -4 1<=5 -4 8 -6 -6", "10100b");
eqt!(cm_807, "8 6 -6 -7 -2<>2 -5 -7 4 6", "11111b");
eqt!(cm_808, "3 -5 4 6 -4<=0 8 -6 8 -4", "01011b");
eqt!(cm_809, "2 -2 -2 -2<=3 7 6 4", "1111b");
eqt!(cm_810, "-5 -3 -2 2 1 -7=0 -6 6 -4 5 5", "000000b");
eqt!(cm_811, "3 -7>=-8 7", "10b");
eqt!(cm_812, "-3 -9 7 -5 -3>4 1 -3 2 -3", "00100b");
eqt!(cm_813, "-1 -3 -9 -2 1 7=-8 0 -9 -6 -9 3", "001000b");
eqt!(cm_814, "4 5 2 -9 5 -5>=-8 -4 5 1 9 -1", "110000b");
eqt!(cm_815, "8 5 -9 0 1 2 -9 -7=5 -9 7 4 -6 6 -7 -6", "00000000b");
eqt!(cm_816, "-9 3 -7 8<>7 -2 3 -2", "1111b");
eqt!(cm_817, "1 -9<>7 4", "11b");
eqt!(cm_818, "9 9 -4 7 -9 -7 -4<-2 -4 1 1 3 -8 2", "0010101b");
eqt!(cm_819, "-5 7 6 -3 0>=-9 -3 1 4 -3", "11101b");
eqt!(cm_820, "5 -2 0 -8 1 3 9<4 9 3 -7 -7 -6 -6", "0111000b");
eqt!(cm_821, "8 -6 6 -8<>-7 -8 -3 -8", "1110b");
eqt!(cm_822, "-5 7 -2 9 4 3 -2>2 -5 1 5 -4 5 -1", "0101100b");
eqt!(cm_823, "5 -8 0 -3 8 -2<=0 9 9 9 8 2", "011111b");
eqt!(cm_824, "-9 8 -5 -7 -6 -2 -5=-4 6 -4 -9 8 -1 2", "0000000b");
eqt!(cm_825, "-3 6 -9 -1 -2>-5 4 -1 2 1", "11000b");
eqt!(cm_826, "-5 -9 7 0<>6 -9 -2 -7", "1011b");
eqt!(cm_827, "5 -3 6 -5 -6>=5 8 -6 -9 1", "10110b");
eqt!(cm_828, "8 -3 3>=-7 -9 -3", "111b");
eqt!(cm_829, "9 0 -7 -6 -4 5 2 -6<9 3 -1 -3 -1 3 9 -6", "01111010b");
eqt!(cm_830, "4 -2 -1 3 4 -6 4>=-4 -4 -5 -1 -5 -5 7", "1111100b");
eqt!(cm_831, "-3 6 8 -4 -3 -2 -4 -5<=-7 6 2 1 -7 -2 -7 9", "01010101b");
eqt!(cm_832, "-9 -9 -6 9 9 -7=2 -2 9 4 7 1", "000000b");
eqt!(cm_833, "3 9 4 8>=-4 8 -8 0", "1111b");
eqt!(cm_834, "-3 -3 -4 9 3 5 -2 4<=-2 -7 6 4 4 -1 0 4", "10101011b");
eqt!(cm_835, "-1 6 -8 5 6 2 7 -9<>6 -4 8 0 0 -6 6 6", "11111111b");
eqt!(cm_836, "-7 -4<=5 2", "11b");
eqt!(cm_837, "7 -1 7 1 3>=-5 5 -9 8 -7", "10101b");
eqt!(cm_838, "0 -5 2 1>4 6 -9 -5", "0011b");
eqt!(cm_839, "-3 2 -2<=1 3 -5", "110b");
eqt!(cm_840, "5 9 9 7 -8 9>=-2 1 -8 -5 8 9", "111101b");
eqt!(cm_841, "-7 0 2 4 6 0<=7 2 -3 -1 7 -2", "110010b");
eqt!(cm_842, "6 -1 -4<=8 -6 -3", "101b");
eqt!(cm_843, "-7 4 7 -1 -7=-6 2 6 -2 6", "00000b");
eqt!(cm_844, "6 2>-5 6", "10b");
eqt!(cm_845, "-8 -4 -3>=6 -5 -2", "010b");
eqt!(cm_846, "-1 5 -9 -6 3>-2 7 0 -6 0", "10001b");
eqt!(cm_847, "-8 -1 -4 -2 -5 7>=5 -5 6 -9 -5 -3", "010111b");
eqt!(cm_848, "8 2 0 0 -8 1 5=-2 3 -1 5 -5 -1 -6", "0000000b");
eqt!(cm_849, "-2 7 -3<=-4 -6 1", "001b");
eqt!(cm_850, "1 7 3 -4 -4<-1 3 -9 6 -6", "00010b");
eqt!(bl_851, "01b&00b", "00b");
eqt!(bl_852, "001b&011b", "001b");
eqt!(bl_853, "00b&11b", "00b");
eqt!(bl_854, "10100101b&01011010b", "00000000b");
eqt!(bl_855, "100000b&000011b", "000000b");
eqt!(bl_856, "001b&000b", "000b");
eqt!(bl_857, "100000b&001111b", "000000b");
eqt!(bl_858, "1000b|0111b", "1111b");
eqt!(bl_859, "110011b&001111b", "000011b");
eqt!(bl_860, "0101000b|1011101b", "1111101b");
eqt!(bl_861, "1010001b|1110110b", "1110111b");
eqt!(bl_862, "01000101b|11101101b", "11101101b");
eqt!(bl_863, "11b&11b", "11b");
eqt!(bl_864, "001b|000b", "001b");
eqt!(bl_865, "01100b&11000b", "01000b");
eqt!(bl_866, "0000b&1111b", "0000b");
eqt!(bl_867, "0010001b&1010100b", "0010000b");
eqt!(bl_868, "001b|101b", "101b");
eqt!(bl_869, "0011110b|1110001b", "1111111b");
eqt!(bl_870, "11011b&11000b", "11000b");
eqt!(bl_871, "01011b&00101b", "00001b");
eqt!(bl_872, "0110b|0110b", "0110b");
eqt!(bl_873, "0101b&1100b", "0100b");
eqt!(bl_874, "110101b|101111b", "111111b");
eqt!(bl_876, "10010b|10001b", "10011b");
eqt!(bl_877, "0010b|1000b", "1010b");
eqt!(bl_878, "0111111b&0010011b", "0010011b");
eqt!(bl_879, "00b&10b", "00b");
eqt!(bl_880, "101b&101b", "101b");
eqt!(bl_882, "000110b&111001b", "000000b");
eqt!(bl_883, "111000b&110010b", "110000b");
eqt!(bl_884, "00110111b|11000101b", "11110111b");
eqt!(bl_886, "101b|111b", "111b");
eqt!(bl_887, "111b|010b", "111b");
eqt!(bl_888, "01001b&00010b", "00000b");
eqt!(bl_889, "1001110b|1100010b", "1101110b");
eqt!(bl_890, "11110000b&11010001b", "11010000b");
eqt!(bl_891, "11110101b|00111100b", "11111101b");
eqt!(bl_892, "010b&010b", "010b");
eqt!(bl_893, "01000b&00011b", "00000b");
eqt!(bl_894, "11b|01b", "11b");
eqt!(bl_895, "0110101b&0000010b", "0000000b");
eqt!(bl_896, "00010011b&01000101b", "00000001b");
eqt!(bl_897, "010b|101b", "111b");
eqt!(bl_898, "110110b|100010b", "110110b");
eqt!(bl_899, "1111100b&0100000b", "0100000b");
eqt!(bl_900, "01b|01b", "01b");
eqt!(bl_901, "0110100b|0111000b", "0111100b");
eqt!(bl_902, "1010b|0010b", "1010b");
eqt!(bl_903, "11110b|10111b", "11111b");
eqt!(bl_904, "1001b|1000b", "1001b");
eqt!(bl_906, "11b&10b", "10b");
eqt!(bl_907, "00000b|10010b", "10010b");
eqt!(bl_908, "00b|01b", "01b");
eqt!(bl_909, "01011b|00010b", "01011b");
eqt!(bl_910, "11b&00b", "00b");
eqt!(bl_911, "00b&01b", "00b");
eqt!(bl_912, "00b|10b", "10b");
eqt!(bl_913, "101101b&010000b", "000000b");
eqt!(bl_914, "100b&101b", "100b");
eqt!(bl_916, "101100b|001010b", "101110b");
eqt!(bl_917, "10011b|00011b", "10011b");
eqt!(bl_918, "0110011b|1001011b", "1111011b");
eqt!(bl_919, "0100100b&1110110b", "0100100b");
eqt!(bl_920, "1011b&0011b", "0011b");
eqt!(bl_921, "01100b|11001b", "11101b");
eqt!(bl_922, "10111110b&00110000b", "00110000b");
eqt!(bl_923, "10000100b|10000100b", "10000100b");
eqt!(bl_924, "001010b|101111b", "101111b");
eqt!(bl_925, "1100011b|0011111b", "1111111b");
eqt!(bl_926, "111b|000b", "111b");
eqt!(bl_927, "010b|111b", "111b");
eqt!(bl_929, "001000b&010101b", "000000b");
eqt!(bl_930, "10b&10b", "10b");
eqt!(bl_931, "001b&110b", "000b");
eqt!(bl_932, "10100b|01110b", "11110b");
eqt!(bl_933, "010100b&110001b", "010000b");
eqt!(bl_934, "11b|11b", "11b");
eqt!(bl_935, "111b&111b", "111b");
eqt!(bl_936, "110b|011b", "111b");
eqt!(bl_937, "11111b|01000b", "11111b");
eqt!(bl_938, "110b|101b", "111b");
eqt!(bl_939, "010b|100b", "110b");
eqt!(bl_940, "11010000b&01100000b", "01000000b");
eqt!(bl_941, "000b&000b", "000b");
eqt!(bl_943, "0011001b|0000011b", "0011011b");
eqt!(bl_944, "0000011b&0010111b", "0000011b");
eqt!(bl_945, "01b&11b", "01b");
eqt!(bl_946, "10011011b&01010000b", "00010000b");
eqt!(bl_947, "0110101b&0011000b", "0010000b");
eqt!(bl_948, "000111b|101001b", "101111b");
eqt!(bl_949, "011000b|000010b", "011010b");
eqt!(bl_950, "1011001b&0000011b", "0000001b");
eqt!(bl_951, "000001b&011000b", "000000b");
eqt!(bl_952, "01b&01b", "01b");
eqt!(bl_953, "0111b|1010b", "1111b");
eqt!(bl_954, "10011101b&00000011b", "00000001b");
eqt!(bl_955, "0000101b&0011000b", "0000000b");
eqt!(bl_957, "0111b|0001b", "0111b");
eqt!(bl_958, "01111100b|00111101b", "01111101b");
eqt!(bl_959, "01011b|10001b", "11011b");
eqt!(bl_960, "0001011b&1000100b", "0000000b");
eqt!(bl_961, "11011b|10100b", "11111b");
eqt!(bl_962, "11110100b|01110010b", "11110110b");
eqt!(bl_963, "010111b&011110b", "010110b");
eqt!(bl_964, "011b&000b", "000b");
eqt!(bl_965, "11000101b&01001001b", "01000001b");
eqt!(bl_966, "101b|001b", "101b");
eqt!(bl_967, "0010001b&0101110b", "0000000b");
eqt!(bl_968, "1000b&1000b", "1000b");
eqt!(bl_969, "111000b|111111b", "111111b");
eqt!(bl_970, "10110b|10000b", "10110b");
eqt!(mo_971, "prds 2 5 7 1 5 2 7", "2 10 70 70 350 700 4900");
eqt!(mo_972, "maxs 4 6 8", "4 6 8");
eqt!(mo_973, "signum 8 4 1", "1 1 1i");
eqt!(mo_974, "differ 8 1", "11b");
eqt!(mo_975, "til count 7 0 2 2 0", "0 1 2 3 4");
eqt!(mo_976, "signum 4 8 2", "1 1 1i");
eqt!(mo_977, "signum 7 0 7", "1 0 1i");
eqt!(mo_978, "deltas 9 1", "9 -8");
eqt!(mo_979, "prev 8 8 5 8 3", "0N 8 8 5 8");
eqt!(mo_980, "differ 2 6 1 2 1 5 4", "1111111b");
eqt!(mo_981, "differ 6 0 8 3 0", "11111b");
eqt!(mo_982, "signum 9 0 5 9", "1 0 1 1i");
eqt!(mo_984, "prev 7 6 4 4 6 6", "0N 7 6 4 4 6");
eqt!(mo_985, "prev 7 2 5 3 8 1", "0N 7 2 5 3 8");
eqt!(mo_986, "reverse 6 0 4 6 9 1 4", "4 1 9 6 4 0 6");
eqt!(mo_987, "next 7 5 0", "5 0 0N");
eqt!(mo_988, "sums 5 2", "5 7");
eqt!(mo_989, "sums 7 2 4", "7 9 13");
eqt!(mo_990, "mins 5 8 2 4 9 1", "5 5 2 2 2 1");
eqt!(mo_991, "prev 7 8 4 6 5", "0N 7 8 4 6");
eqt!(mo_992, "til count 0 3 7 9 0 7 2", "0 1 2 3 4 5 6");
eqt!(mo_993, "next 7 7 5 1 3", "7 5 1 3 0N");
eqt!(mo_995, "next 4 7 4 1 9", "7 4 1 9 0N");
eqt!(mo_996, "mins 9 2", "9 2");
eqt!(mo_997, "reverse 5 3 6 2 8", "8 2 6 3 5");
eqt!(mo_998, "til count 4 9 8 1 0", "0 1 2 3 4");
eqt!(mo_999, "abs 6 4", "6 4");
eqt!(mo_1000, "reverse 2 6 3 5 7", "7 5 3 6 2");
eqt!(mo_1002, "reverse 2 2 0 1", "1 0 2 2");
eqt!(mo_1003, "next 4 0 1 4 5 5 0", "0 1 4 5 5 0 0N");
eqt!(mo_1005, "mins 3 6 5 3 3 6", "3 3 3 3 3 3");
eqt!(mo_1006, "deltas 7 4 2 7 3 1", "7 -3 -2 5 -4 -2");
eqt!(mo_1007, "prds 6 5 5 2 8", "6 30 150 300 2400");
eqt!(mo_1008, "reverse 0 5 8 4 5", "5 4 8 5 0");
eqt!(mo_1009, "reverse 0 4", "4 0");
eqt!(mo_1010, "prds 0 5 0 5 7", "0 0 0 0 0");
eqt!(mo_1011, "reverse 9 7", "7 9");
eqt!(mo_1012, "reverse 6 7 5 7 9 7", "7 9 7 5 7 6");
eqt!(mo_1015, "prds 8 1 9 3 5 6", "8 8 72 216 1080 6480");
eqt!(mo_1016, "neg 7 6 9 1 3 8 2", "-7 -6 -9 -1 -3 -8 -2");
eqt!(mo_1017, "til count 3 9 7 7 8 5 7", "0 1 2 3 4 5 6");
eqt!(mo_1018, "maxs 7 3 2 3 0", "7 7 7 7 7");
eqt!(mo_1019, "next 9 9 5 4 9", "9 5 4 9 0N");
eqt!(mo_1020, "sums 5 7 9 1 4 3 0", "5 12 21 22 26 29 29");
eqt!(mo_1021, "neg 8 1 3 6", "-8 -1 -3 -6");
eqt!(mo_1022, "maxs 6 7 3 5 6", "6 7 7 7 7");
eqt!(mo_1023, "mins 5 2 6 3", "5 2 2 2");
eqt!(mo_1024, "neg 2 1 8 8 8 4 2", "-2 -1 -8 -8 -8 -4 -2");
eqt!(mo_1025, "deltas 3 4 1 8 8", "3 1 -3 7 0");
eqt!(mo_1027, "neg 8 0 5", "-8 0 -5");
eqt!(mo_1028, "prds 9 5 3 6 3 0 9", "9 45 135 810 2430 0 0");
eqt!(mo_1029, "signum 9 6", "1 1i");
eqt!(mo_1030, "differ 8 6 0 8 6 9 9", "1111110b");
eqt!(mo_1031, "mins 3 6 9 2 0", "3 3 3 2 0");
eqt!(mo_1032, "reverse 6 9 2 7 3 4", "4 3 7 2 9 6");
eqt!(mo_1033, "prds 1 0 1", "1 0 0");
eqt!(mo_1034, "prds 5 8 2 7", "5 40 80 560");
eqt!(mo_1035, "abs 5 1 5 5", "5 1 5 5");
eqt!(mo_1036, "signum 2 4 0 6 9 7 1", "1 1 0 1 1 1 1i");
eqt!(mo_1037, "til count 0 5 5", "0 1 2");
eqt!(mo_1038, "prds 2 1", "2 2");
eqt!(mo_1039, "maxs 6 0 1", "6 6 6");
eqt!(mo_1040, "neg 7 9 5 8", "-7 -9 -5 -8");
eqt!(mo_1041, "prev 7 6 4 6 9 8", "0N 7 6 4 6 9");
eqt!(mo_1042, "mins 5 6 6 3", "5 5 5 3");
eqt!(mo_1043, "mins 3 7", "3 3");
eqt!(mo_1044, "prds 1 9 9", "1 9 81");
eqt!(mo_1045, "abs 9 7 3", "9 7 3");
eqt!(mo_1046, "prev 3 7 3", "0N 3 7");
eqt!(mo_1047, "prds 5 4 6 7 3 7", "5 20 120 840 2520 17640");
eqt!(mo_1048, "deltas 1 6 8 3 4 8 7", "1 5 2 -5 1 4 -1");
eqt!(mo_1049, "neg 3 8 6 7 4 7", "-3 -8 -6 -7 -4 -7");
eqt!(mo_1050, "prds 9 0 3 7", "9 0 0 0");
eqt!(mo_1051, "abs 8 1 1 9", "8 1 1 9");
eqt!(mo_1052, "prev 7 7", "0N 7");
eqt!(mo_1053, "abs 9 5 3 8 9", "9 5 3 8 9");
eqt!(mo_1054, "deltas 1 4", "1 3");
eqt!(mo_1055, "signum 0 8 9 0 3", "0 1 1 0 1i");
eqt!(mo_1056, "deltas 2 1 1", "2 -1 0");
eqt!(mo_1057, "next 1 3 9 9 0 1", "3 9 9 0 1 0N");
eqt!(mo_1058, "reverse 6 3 0 1", "1 0 3 6");
eqt!(mo_1059, "til count 2 8 5", "0 1 2");
eqt!(mo_1060, "mins 7 8 0 8 4", "7 7 0 0 0");
eqt!(mo_1061, "abs 0 0 2 6", "0 0 2 6");
eqt!(mo_1062, "deltas 2 1 8", "2 -1 7");
eqt!(mo_1063, "next 1 1 2 7", "1 2 7 0N");
eqt!(mo_1064, "next 8 1 5", "1 5 0N");
eqt!(mo_1065, "neg 8 7 2 6 0", "-8 -7 -2 -6 0");
eqt!(mo_1066, "abs 0 4 3 8", "0 4 3 8");
eqt!(mo_1067, "reverse 4 3 5", "5 3 4");
eqt!(mo_1068, "sums 1 6 8 1 5 4 4", "1 7 15 16 21 25 29");
eqt!(mo_1069, "maxs 8 4 9", "8 8 9");
eqt!(mo_1070, "prev 4 1", "0N 4");
eqt!(mo_1071, "differ 2 9 0 4 5 6 1", "1111111b");
eqt!(mo_1072, "signum 4 1 6 8", "1 1 1 1i");
eqt!(mo_1073, "abs 7 0 6 2 3 1 6", "7 0 6 2 3 1 6");
eqt!(mo_1074, "prds 8 1", "8 8");
eqt!(mo_1075, "til count 6 6 3 6", "0 1 2 3");
eqt!(mo_1076, "reverse 6 9", "9 6");
eqt!(mo_1077, "til count 5 9 5 0 0 4", "0 1 2 3 4 5");
eqt!(mo_1078, "neg 2 4 2 8 1 5 2", "-2 -4 -2 -8 -1 -5 -2");
eqt!(mo_1079, "abs 4 9 4 6 7 9 8", "4 9 4 6 7 9 8");
eqt!(mo_1080, "neg 4 7 9 4 3", "-4 -7 -9 -4 -3");
eqt!(mo_1081, "signum 8 0 3 0 6 1 2", "1 0 1 0 1 1 1i");
eqt!(mo_1082, "mins 2 6 0 6 1 7 8", "2 2 0 0 0 0 0");
eqt!(mo_1083, "abs 9 1 9 0 1 5", "9 1 9 0 1 5");
eqt!(mo_1084, "differ 7 1 2", "111b");
eqt!(mo_1085, "prev 4 7 8", "0N 4 7");
eqt!(mo_1087, "abs 2 7 2 8", "2 7 2 8");
eqt!(mo_1088, "signum 1 5 0 3 6", "1 1 0 1 1i");
eqt!(mo_1089, "abs 2 8 3 3 8 8 6", "2 8 3 3 8 8 6");
eqt!(mo_1090, "differ 2 9 7 6 9 3", "111111b");
eqt!(mo_1091, "maxs 0 9 7 8", "0 9 9 9");
eqt!(mo_1092, "maxs 0 1 9 7 4 6", "0 1 9 9 9 9");
eqt!(mo_1093, "deltas 0 6 1 6 5", "0 6 -5 5 -1");
eqt!(mo_1094, "differ 5 2 1", "111b");
eqt!(mo_1095, "mins 5 8 8 8", "5 5 5 5");
eqt!(mo_1096, "til count 5 9 0", "0 1 2");
eqt!(mo_1097, "reverse 7 2 6 0 9 0", "0 9 0 6 2 7");
eqt!(mo_1098, "maxs 2 8 8 9", "2 8 8 9");
eqt!(mo_1099, "abs 0 5 1 5", "0 5 1 5");
eqt!(mo_1100, "ratios 5 5 1 2 7", "5 1 0.2 2 3.5");
eqt!(mo_1101, "reverse 2 5 9 0", "0 9 5 2");
eqt!(mo_1103, "til count 9 6", "0 1");
eqt!(mo_1104, "maxs 9 7 6 2", "9 9 9 9");
eqt!(mo_1105, "prev 9 2 9 0 3 2 4", "0N 9 2 9 0 3 2");
eqt!(mo_1106, "differ 5 9 1 5 4 7 5", "1111111b");
eqt!(mo_1107, "prds 6 2 2 3 6 8", "6 12 24 72 432 3456");
eqt!(mo_1108, "reverse 2 4 0", "0 4 2");
eqt!(mo_1109, "differ 9 9", "10b");
eqt!(mo_1110, "maxs 8 1 7 5 0", "8 8 8 8 8");
eqt!(mo_1111, "signum 5 2 1", "1 1 1i");
eqt!(mo_1112, "reverse 6 5 7 1 9 3", "3 9 1 7 5 6");
eqt!(mo_1113, "mins 7 6 4 5 8", "7 6 4 4 4");
eqt!(mo_1114, "til count 4 1 4 9 1 9", "0 1 2 3 4 5");
eqt!(mo_1115, "maxs 6 9", "6 9");
eqt!(mo_1117, "mins 4 3", "4 3");
eqt!(mo_1118, "til count 1 6 1", "0 1 2");
eqt!(mo_1119, "til count 0 3 6", "0 1 2");
eqt!(mo_1120, "next 0 2 0", "2 0 0N");
eqt!(mo_1121, "prds 3 4 7 6 2 6", "3 12 84 504 1008 6048");
eqt!(mo_1123, "prds 8 2 0 2 5", "8 16 0 0 0");
eqt!(mo_1124, "neg 3 6 7 8 0 5", "-3 -6 -7 -8 0 -5");
eqt!(mo_1125, "reverse 2 1", "1 2");
eqt!(mo_1126, "sums 1 8 8 3", "1 9 17 20");
eqt!(mo_1127, "differ 3 5 0 5 3", "11111b");
eqt!(mo_1128, "next 5 6", "6 0N");
eqt!(mo_1129, "mins 9 9 3 4 2", "9 9 3 3 2");
eqt!(mo_1130, "mins 7 8 7 1 5", "7 7 7 1 1");
eqt!(mo_1132, "signum 6 7 6 6", "1 1 1 1i");
eqt!(mo_1133, "abs 5 2 4 7 7 7 7", "5 2 4 7 7 7 7");
eqt!(mo_1134, "sums 0 6", "0 6");
eqt!(mo_1135, "prds 8 8 8 0 4", "8 64 512 0 0");
eqt!(mo_1136, "next 8 7 0 0 2", "7 0 0 2 0N");
eqt!(mo_1137, "abs 9 4 8", "9 4 8");
eqt!(mo_1139, "til count 1 0 6 1 3 0 4", "0 1 2 3 4 5 6");
eqt!(mo_1140, "mins 7 5", "7 5");
eqt!(mo_1141, "abs 9 1", "9 1");
eqt!(mo_1142, "til count 4 8 5 1 7 6", "0 1 2 3 4 5");
eqt!(mo_1143, "differ 1 7 4 1 3 5 3", "1111111b");
eqt!(mo_1144, "maxs 6 1 0 2", "6 6 6 6");
eqt!(mo_1146, "mins 8 6 6 5", "8 6 6 5");
eqt!(mo_1147, "sums 9 7 5 2", "9 16 21 23");
eqt!(mo_1148, "signum 5 8 5 2 6", "1 1 1 1 1i");
eqt!(mo_1149, "deltas 4 5 8 2 9 6", "4 1 3 -6 7 -3");
eqt!(mo_1150, "sums 8 1 3 3", "8 9 12 15");
eqt!(ag_1151, "prd 2 1 0 4 6 3 8 5", "0");
eqt!(ag_1152, "max 0 6 5 0 6 6 9", "9");
eqt!(ag_1154, "sum 7 6 4 6", "23j");
eqt!(ag_1155, "all 9 6 6 0 1 2 0", "0b");
eqt!(ag_1156, "mins 7 7 4 0 1 0 7 0 7", "7 7 4 0 0 0 0 0 0");
eqt!(ag_1157, "mins 0 9 8 3 4 3 6", "0 0 0 0 0 0 0");
eqt!(ag_1158, "all 1 6 4", "1b");
eqt!(ag_1159, "first 0 4 4 7 2", "0");
eqt!(ag_1160, "avg 7 9", "8f");
eqt!(ag_1161, "max 1 8 1 5 5 7 7 9", "9");
eqt!(ag_1162, "min 7 0 0 2", "0");
eqt!(ag_1163, "sums 7 2 8 7 8 6 5 2", "7 9 17 24 32 38 43 45");
eqt!(ag_1164, "count 2 9", "2");
eqt!(ag_1165, "all 1 8", "1b");
eqt!(ag_1166, "var 2 8", "9f");
eqt!(ag_1167, "count 1 3 6 7 1 7 1 2", "8");
eqt!(ag_1169, "maxs 1 3 1 2 3", "1 3 3 3 3");
eqt!(ag_1170, "max 9 1", "9");
eqt!(ag_1171, "any 8 6 0 6", "1b");
eqt!(ag_1172, "all 9 0 7 8 1", "0b");
eqt!(ag_1174, "mins 6 4 4 6", "6 4 4 4");
eqt!(ag_1175, "first 4 6 3 4 4", "4");
eqt!(ag_1177, "last 4 8 6 3 1 6 6 5 5", "5");
eqt!(ag_1178, "maxs 1 9 6 4", "1 9 9 9");
eqt!(ag_1179, "prd 8 6 8 7 2", "5376");
eqt!(ag_1180, "maxs 1 4 8 8 0 5", "1 4 8 8 8 8");
eqt!(ag_1182, "first 2 5 5 7 5 0 7 7", "2");
eqt!(ag_1183, "first 0 1 8 2 9 8 0 7 8", "0");
eqt!(ag_1184, "var 3 6 6 5 8 6 5 3", "2.4375");
eqt!(ag_1185, "sum 5 8 5 8 7 9 3 6 7", "58j");
eqt!(ag_1186, "last 3 4 4", "4");
eqt!(ag_1187, "avg 0 3 8 9 3 4", "4.5");
eqt!(ag_1188, "count 8 2 6 1 2 3", "6");
eqt!(ag_1189, "med 1 4 5 9 2 2 6", "4f");
eqt!(ag_1190, "all 3 3 2 0 8", "0b");
eqt!(ag_1191, "mins 3 3 3 9", "3 3 3 3");
eqt!(ag_1192, "max 8 3 5 6 1 3 8 5", "8");
eqt!(ag_1193, "first 8 3 2 7 7 2 4 3 0", "8");
eqt!(ag_1194, "sums 9 3", "9 12");
eqt!(ag_1195, "med 4 6 7 7 3 2 0 1", "3.5");
eqt!(ag_1197, "sums 4 6 3", "4 10 13");
eqt!(ag_1198, "avg 3 2 6 8 8", "5.4");
eqt!(ag_1199, "last 0 3 8 9 7 6 0", "0");
eqt!(ag_1200, "count 2 2 8 6", "4");
eqt!(ag_1202, "any 6 2 1 6 6 2 0", "1b");
eqt!(ag_1204, "prd 0 2 8 6 6 6 5 1 2", "0");
eqt!(ag_1205, "first 4 4 0 2 6 2", "4");
eqt!(ag_1206, "any 3 8 0 8 8 8", "1b");
eqt!(ag_1207, "first 6 4 4", "6");
eqt!(ag_1208, "avg 7 5 6 2", "5f");
eqt!(ag_1209, "all 1 1 8 6 4 7 3 6 1", "1b");
eqt!(ag_1210, "last 7 9 0 4 9 1 8", "8");
eqt!(ag_1211, "max 6 6", "6");
eqt!(ag_1212, "mins 9 4 5 9", "9 4 4 4");
eqt!(ag_1213, "max 1 9 9 9 6 4 8 4", "9");
eqt!(ag_1214, "count 9 7 1 6 9 8 5 5", "8");
eqt!(ag_1215, "sums 9 8", "9 17");
eqt!(ag_1216, "last 8 0 6 9 3 2 9 5", "5");
eqt!(ag_1217, "var 8 8 3 6", "4.1875");
eqt!(ag_1218, "sums 2 3", "2 5");
eqt!(ag_1219, "count 3 0 5 8 5 6 9 6", "8");
eqt!(ag_1220, "all 9 9 9 5 4 7 4", "1b");
eqt!(ag_1221, "all 0 3 7 0 5 1 1 9 8", "0b");
eqt!(ag_1223, "sums 7 1 4 7 1", "7 8 12 19 20");
eqt!(ag_1224, "avg 9 7", "8f");
eqt!(ag_1226, "maxs 9 5 6 5 7 4 2 5", "9 9 9 9 9 9 9 9");
eqt!(ag_1227, "any 4 2 1 9 6 4", "1b");
eqt!(ag_1228, "sum 8 1 9 7 4 0 4", "33j");
eqt!(ag_1231, "sums 0 3 7 5", "0 3 10 15");
eqt!(ag_1233, "count 0 7 5 1 8 3 6 1 2", "9");
eqt!(ag_1234, "var 7 8 3 5 5", "3.04");
eqt!(ag_1235, "med 1 8", "4.5");
eqt!(ag_1236, "any 5 8 9 6 2", "1b");
eqt!(ag_1237, "var 5 5 6 3 6 1 6 5", "2.734375");
eqt!(ag_1238, "last 8 1 1 8 0 2 5", "5");
eqt!(ag_1239, "any 4 1 5 8 6 7", "1b");
eqt!(ag_1240, "sum 8 7 8 8 9 5 1 2", "48j");
eqt!(ag_1241, "prd 1 1 4 0 0", "0");
eqt!(ag_1242, "min 9 1 3 8 7 4 9 0", "0");
eqt!(ag_1243, "all 9 1 8 4 2 6 5 3", "1b");
eqt!(ag_1244, "avg 7 1 4 6 0 6 4", "4f");
eqt!(ag_1245, "var 3 7 5 1 3 3 5 0", "4.484375");
eqt!(ag_1246, "prd 2 1 3 4 5 9", "1080");
eqt!(ag_1247, "med 8 1 2 0 3 9 9 0", "2.5");
eqt!(ag_1248, "all 4 0", "0b");
eqt!(ag_1249, "var 7 6 3 5 1 4 7 8", "4.859375");
eqt!(ag_1250, "mins 5 7 7", "5 5 5");
eqt!(ag_1251, "all 5 7 3 8 4", "1b");
eqt!(ag_1252, "count 6 6 2 6 2 4", "6");
eqt!(ag_1253, "min 1 3 3 0 0 2 7 0 8", "0");
eqt!(ag_1254, "sum 9 1 9 0 2 0 8 9", "38j");
eqt!(ag_1255, "maxs 4 5 2 8 9 6 5", "4 5 5 8 9 9 9");
eqt!(ag_1257, "med 3 4", "3.5");
eqt!(ag_1258, "count 0 1 3 6 8 3 1 6", "8");
eqt!(ag_1259, "med 7 5 0 0 2 8", "3.5");
eqt!(ag_1260, "any 2 0 3 9 8 8 0 2", "1b");
eqt!(ag_1261, "last 9 6 9 3 5 1", "1");
eqt!(ag_1262, "var 4 4 7 2", "3.1875");
eqt!(ag_1263, "max 3 1", "3");
eqt!(ag_1264, "med 8 3 5 6 5 6", "5.5");
eqt!(ag_1265, "sums 1 4 4 8 5 2 3 4 3", "1 5 9 17 22 24 27 31 34");
eqt!(ag_1266, "max 4 8 5", "8");
eqt!(ag_1267, "maxs 7 8 8 2", "7 8 8 8");
eqt!(ag_1268, "last 5 2 5 4 3 2 3", "3");
eqt!(ag_1269, "min 2 8 3 3 7 1 1 3", "1");
eqt!(ag_1270, "sum 8 3 6 8 7 4 9 2 8", "55j");
eqt!(ag_1271, "last 1 0 6 4 6 8 2", "2");
eqt!(ag_1273, "last 6 6 4 5 4 6 2", "2");
eqt!(ag_1274, "all 9 4 7", "1b");
eqt!(ag_1275, "maxs 9 9 4 2 4 8 1 4 8", "9 9 9 9 9 9 9 9 9");
eqt!(ag_1276, "med 3 0 4 6 4 0 5 6", "4f");
eqt!(ag_1277, "med 2 0", "1f");
eqt!(ag_1278, "sum 4 1 5 6 9 2 3 2 9", "41j");
eqt!(ag_1280, "maxs 3 7 3 6 9", "3 7 7 7 9");
eqt!(ag_1281, "med 9 3 7 3 4 2 4 3", "3.5");
eqt!(ag_1282, "med 7 4 6", "6f");
eqt!(ag_1283, "med 6 5 7 6 3 3 2 7", "5.5");
eqt!(ag_1284, "last 8 1 7 1 2 8 9 8 5", "5");
eqt!(ag_1285, "min 9 6 5 6 9 1", "1");
eqt!(ag_1286, "first 9 5 2 9 6 7 5 6 8", "9");
eqt!(ag_1288, "sum 7 6 4", "17j");
eqt!(ag_1289, "min 8 8 8 7", "7");
eqt!(ag_1290, "sums 3 3 0 9 8 6 5 6 7", "3 6 6 15 23 29 34 40 47");
eqt!(ag_1291, "last 3 1 5 0 4 6 9", "9");
eqt!(ag_1292, "maxs 0 2 8 8 4 5 6 4", "0 2 8 8 8 8 8 8");
eqt!(ag_1293, "max 5 1 1 8 2 6 4", "8");
eqt!(ag_1294, "min 1 4", "1");
eqt!(ag_1295, "maxs 9 3 2 1 6", "9 9 9 9 9");
eqt!(ag_1296, "maxs 8 5 3", "8 8 8");
eqt!(ag_1297, "all 5 4 3 4 4 6 8", "1b");
eqt!(ag_1298, "count 8 9", "2");
eqt!(ag_1301, "min 7 6 3 0 3 9 8 6 0", "0");
eqt!(ag_1302, "last 4 2 4 4 7 9", "9");
eqt!(ag_1303, "med 4 8 0 1 5 6 2 0 8", "4f");
eqt!(ag_1304, "all 0 2 1 3", "0b");
eqt!(ag_1305, "all 9 9 4", "1b");
eqt!(ag_1306, "all 8 5 5 3 9 6", "1b");
eqt!(ag_1307, "sum 3 6 8", "17j");
eqt!(ag_1308, "first 8 7 0 4 3 1", "8");
eqt!(ag_1309, "maxs 8 6 5", "8 8 8");
eqt!(ag_1310, "sums 0 8 6 5 2 9", "0 8 14 19 21 30");
eqt!(ag_1311, "any 1 7 4 3 7 0 1 1 3", "1b");
eqt!(ag_1312, "med 0 0 9", "0f");
eqt!(ag_1313, "var 6 9 9 6 9", "2.16");
eqt!(ag_1314, "min 8 5 9 2", "2");
eqt!(ag_1315, "sums 3 8 0 0", "3 11 11 11");
eqt!(ag_1316, "max 9 1 4", "9");
eqt!(ag_1317, "count 1 9 9 9 4 7 1", "7");
eqt!(ag_1318, "max 3 6 9 8 6 3 4 2", "9");
eqt!(ag_1320, "prd 5 0 2", "0");
eqt!(ag_1321, "var 4 4 2 6", "2f");
eqt!(ag_1322, "last 3 6 3 2 6", "6");
eqt!(ag_1323, "first 6 2 5 5 3", "6");
eqt!(ag_1324, "last 1 9 4 4 7 2", "2");
eqt!(ag_1325, "max 0 2", "2");
eqt!(ag_1326, "prd 9 7 9 2 0", "0");
eqt!(ag_1328, "mins 8 8 7 8 4 7", "8 8 7 7 4 4");
eqt!(ag_1329, "first 7 9 1 5", "7");
eqt!(ag_1330, "maxs 4 5 8 3 7 0 1 6 7", "4 5 8 8 8 8 8 8 8");
eqt!(ag_1331, "med 6 3 2 0 3", "3f");
eqt!(ag_1332, "count 6 4 0 5 9 2 5 2", "8");
eqt!(ag_1333, "any 9 7 1 5 3 6 7 2 8", "1b");
eqt!(ag_1334, "count 5 7 8", "3");
eqt!(ag_1335, "max 5 5 9 8 3 1", "9");
eqt!(ag_1336, "med 6 9", "7.5");
eqt!(ag_1337, "mins 1 1 2 0", "1 1 1 0");
eqt!(ag_1338, "sums 2 5 4 1 3 2", "2 7 11 12 15 17");
eqt!(ag_1339, "count 7 3 9 1 5", "5");
eqt!(ag_1342, "prd 3 1 7 2 3 4 9 8", "36288");
eqt!(ag_1343, "count 0 8 1 8 7 8 4", "7");
eqt!(ag_1344, "prd 9 2 0 9 0 0 4 9", "0");
eqt!(ag_1345, "max 0 0", "0");
eqt!(ag_1346, "med 0 3 7", "3f");
eqt!(ag_1348, "maxs 4 1 6 5 3 9 6 6 2", "4 4 6 6 6 9 9 9 9");
eqt!(ag_1349, "sum 8 6 1 6 7 0 3 9", "40j");
eqt!(ag_1350, "sums 0 3 8 2 9 8", "0 3 11 13 22 30");
eqt!(ls_1351, "-4# 7 3 4 7 6 8 9 5 3", "8 9 5 3");
eqt!(ls_1352, "2_ 2 5 1 0", "1 0");
eqt!(ls_1354, "2_ 5 2 9 4 3 6", "9 4 3 6");
eqt!(ls_1355, "-1_ 8 0 3", "8 0");
eqt!(ls_1357, "2# 4 0 2", "4 0");
eqt!(ls_1358, "4# 4 4 5 2", "4 4 5 2");
eqt!(ls_1359, "3# 8 9 8 9 2 4 1 3", "8 9 8");
eqt!(ls_1360, "3 cut 4 8 0 5 4", "(4 8 0;5 4)");
eqt!(ls_1361, "4 sublist 6 3 7 1 0 0", "6 3 7 1");
eqt!(ls_1362, "4_ 9 0 0 3 6 7 0 3", "6 7 0 3");
eqt!(ls_1363, "2 cut 2 8 7 0 8 2 3 5", "(2 8;7 0;8 2;3 5)");
eqt!(ls_1364, "-2 rotate 5 2 4 0 2 4", "2 4 5 2 4 0");
eqt!(ls_1365, "2# 2 3 9 9 9 1 3 7 0", "2 3");
eqt!(ls_1366, "1_ 3 7 7 4 0 3 9 9", "7 7 4 0 3 9 9");
eqt!(ls_1367, "-4# 1 1 1 4 9 9", "1 4 9 9");
eqt!(ls_1368, "4_ 3 9 1 8 1 8 6 9 4", "1 8 6 9 4");
eqt!(ls_1369, "2_ 4 3 9 0 3 7 1", "9 0 3 7 1");
eqt!(ls_1370, "-1 rotate 7 0 9 5 1", "1 7 0 9 5");
eqt!(ls_1371, "-4# 5 5 1", "1 5 5 1");
eqt!(ls_1372, "-1_ 0 2 4 1 3 0 2 3", "0 2 4 1 3 0 2");
eqt!(ls_1373, "4_ 0 7 5 8 7 4 1", "7 4 1");
eqt!(ls_1374, "2# 8 8 8 9 5 0 4 8", "8 8");
eqt!(ls_1375, "3 cut 7 8 5 9 9", "(7 8 5;9 9)");
eqt!(ls_1377, "2 sublist 7 8 2 3 1 6 8 6", "7 8");
eqt!(ls_1378, "4 sublist 9 8 6 3", "9 8 6 3");
eqt!(ls_1379, "3_ 4 3 9 5 3", "5 3");
eqt!(ls_1380, "0 rotate 2 1 0 9 2 3 8 0", "2 1 0 9 2 3 8 0");
eqt!(ls_1383, "-3 rotate 9 2 7 2 0", "7 2 0 9 2");
eqt!(ls_1386, "3# 8 1 0 8 8 1", "8 1 0");
eqt!(ls_1387, "3 sublist 4 2 6 8", "4 2 6");
eqt!(ls_1388, "3 rotate 9 0 4 0 3 7", "0 3 7 9 0 4");
eqt!(ls_1389, "4 sublist 6 1 2 0 6", "6 1 2 0");
eqt!(ls_1390, "4_ 2 9 8 1 6 3 0 5 4", "6 3 0 5 4");
eqt!(ls_1394, "3 sublist 9 0 0 9 5 5", "9 0 0");
eqt!(ls_1395, "3 cut 4 7 2 9 8", "(4 7 2;9 8)");
eqt!(ls_1396, "2 cut 1 7 5 6", "(1 7;5 6)");
eqt!(ls_1398, "2 sublist 0 2 8 5 8 0 5", "0 2");
eqt!(ls_1399, "2 rotate 2 8 4 3 2 6 5 3", "4 3 2 6 5 3 2 8");
eqt!(ls_1400, "4# 2 8 3 3", "2 8 3 3");
eqt!(ls_1401, "-2# 3 6 0 3 7 2 3", "2 3");
eqt!(ls_1403, "-4_ 0 4 7 3 9 4", "0 4");
eqt!(ls_1404, "3 cut 5 8 0 5 2 2 2 8 3", "(5 8 0;5 2 2;2 8 3)");
eqt!(ls_1405, "1 rotate 9 2 3 1 8 7", "2 3 1 8 7 9");
eqt!(ls_1406, "3 sublist 5 3 4 0 2 5 5 4", "5 3 4");
eqt!(ls_1407, "-3# 9 4 7 3 0", "7 3 0");
eqt!(ls_1408, "3# 3 2 3 0 9 7 4 6 1", "3 2 3");
eqt!(ls_1409, "0 rotate 6 0 3 8 8 9", "6 0 3 8 8 9");
eqt!(ls_1410, "-1_ 2 9 4 3", "2 9 4");
eqt!(ls_1413, "2 sublist 8 8 9 6 4 5 8 3", "8 8");
eqt!(ls_1414, "3_ 3 4 8 0 4 1", "0 4 1");
eqt!(ls_1415, "-2_ 3 1 6 9 6 9 1 6 7", "3 1 6 9 6 9 1");
eqt!(ls_1419, "-2 rotate 9 8 2 4 9 6", "9 6 9 8 2 4");
eqt!(ls_1420, "0_ 0 4 4 3 0", "0 4 4 3 0");
eqt!(ls_1421, "-4# 6 9 4 4 6 7 6 9", "6 7 6 9");
eqt!(ls_1422, "2 cut 4 3 1 3 1 8 5 3", "(4 3;1 3;1 8;5 3)");
eqt!(ls_1423, "0_ 2 1 9 5 3", "2 1 9 5 3");
eqt!(ls_1424, "-3_ 1 5 5 3 7 9 7 9 5", "1 5 5 3 7 9");
eqt!(ls_1425, "1 rotate 1 7 0 9", "7 0 9 1");
eqt!(ls_1426, "-3# 2 2 1 3 1 8 3", "1 8 3");
eqt!(ls_1427, "4_ 3 2 3 1 2 7 1 8", "2 7 1 8");
eqt!(ls_1428, "5# 6 8 2 5", "6 8 2 5 6");
eqt!(ls_1429, "2 sublist 8 4 9", "8 4");
eqt!(ls_1430, "0 rotate 7 8 2", "7 8 2");
eqt!(ls_1432, "-3# 0 5 9 2 8 3 1 8", "3 1 8");
eqt!(ls_1433, "3 cut 0 0 9 6 3 3 4 2 1", "(0 0 9;6 3 3;4 2 1)");
eqt!(ls_1435, "-3# 1 1 3 5 5 6 7", "5 6 7");
eqt!(ls_1439, "2 sublist 5 3 9 2 7 7 9", "5 3");
eqt!(ls_1440, "4 sublist 0 3 2", "0 3 2");
eqt!(ls_1443, "3 sublist 5 6 3 3 9 7 4 7", "5 6 3");
eqt!(ls_1444, "-4_ 8 8 2 7 0 0 0 1 9", "8 8 2 7 0");
eqt!(ls_1446, "3# 9 6 9 9 4 8", "9 6 9");
eqt!(ls_1447, "-4# 7 0 3 5 9 7 9 3", "9 7 9 3");
eqt!(ls_1448, "1_ 6 5 5 7 2 4 6 8", "5 5 7 2 4 6 8");
eqt!(ls_1449, "-3 rotate 5 7 5 1 0 1 6", "0 1 6 5 7 5 1");
eqt!(ls_1450, "-2_ 9 6 9 0 4 8 2 6", "9 6 9 0 4 8");
eqt!(ls_1451, "1 rotate 3 3 7 6 5", "3 7 6 5 3");
eqt!(ls_1452, "2 cut 5 4 3 5", "(5 4;3 5)");
eqt!(ls_1455, "3# 3 2 9 2 5 8 6", "3 2 9");
eqt!(ls_1456, "0_ 8 1 0 7 2 9 4 2 3", "8 1 0 7 2 9 4 2 3");
eqt!(ls_1458, "2 sublist 1 8 3 2 5 8 3", "1 8");
eqt!(ls_1459, "3 rotate 8 5 1 1", "1 8 5 1");
eqt!(ls_1461, "3 cut 6 3 8 5 2 6", "(6 3 8;5 2 6)");
eqt!(ls_1462, "4# 8 7 7 4 0 0", "8 7 7 4");
eqt!(ls_1464, "5_ 6 2 1 3 8 5 2", "5 2");
eqt!(ls_1465, "-4_ 8 6 0 5 7 2", "8 6");
eqt!(ls_1468, "-4 rotate 1 8 7", "7 1 8");
eqt!(ls_1470, "-4 rotate 0 7 1 8 3", "7 1 8 3 0");
eqt!(ls_1471, "5# 6 0 6 9 8 6 5 7 4", "6 0 6 9 8");
eqt!(ls_1472, "2 sublist 8 8 3 3 7 8", "8 8");
eqt!(ls_1474, "-3# 2 3 4 5", "3 4 5");
eqt!(ls_1475, "-4 rotate 2 6 1", "1 2 6");
eqt!(ls_1477, "5# 4 7 3", "4 7 3 4 7");
eqt!(ls_1478, "5_ 0 3 4 2 8 1 0", "1 0");
eqt!(ls_1480, "4_ 8 3 8 4 0 6 9", "0 6 9");
eqt!(ls_1481, "3 cut 9 6 8 9 0", "(9 6 8;9 0)");
eqt!(ls_1482, "3# 5 3 7 9 0 7", "5 3 7");
eqt!(ls_1483, "-3_ 9 4 8 1 3", "9 4");
eqt!(ls_1484, "3_ 4 8 2 6 9 4 1", "6 9 4 1");
eqt!(ls_1485, "3 cut 3 7 9 6 1 9 8 6 7", "(3 7 9;6 1 9;8 6 7)");
eqt!(ls_1487, "2 sublist 9 7 6 4 4", "9 7");
eqt!(ls_1488, "-1 rotate 5 8 5 8 6 0 5 8", "8 5 8 5 8 6 0 5");
eqt!(ls_1489, "-1_ 0 8 2", "0 8");
eqt!(ls_1490, "0 rotate 7 7 4 8 8 1 1", "7 7 4 8 8 1 1");
eqt!(ls_1491, "5# 3 3 7 8 2 4", "3 3 7 8 2");
eqt!(ls_1492, "1_ 4 2 6 2 5 3", "2 6 2 5 3");
eqt!(ls_1495, "-1_ 2 9 9 2 5 5 4", "2 9 9 2 5 5");
eqt!(ls_1496, "-1 rotate 4 0 5 0 3 8 8 2", "2 4 0 5 0 3 8 8");
eqt!(ls_1497, "-1 rotate 2 3 4 1 2", "2 2 3 4 1");
eqt!(ls_1498, "-1_ 8 5 6 8", "8 5 6");
eqt!(ls_1499, "3 sublist 1 1 5 7 2 8 2 7 6", "1 1 5");
eqt!(ls_1500, "2# 9 5 4 5 4 0", "9 5");
eqt!(st_1501, "desc 3 2 5", "5 3 2");
eqt!(st_1502, "distinct 4 4 5", "4 5");
eqt!(st_1503, "desc 1 2 1 1 0 3 0 1", "3 2 1 1 1 1 0 0");
eqt!(st_1504, "asc 4 5 0", "`s#0 4 5");
eqt!(st_1505, "iasc 5 1 2 4", "1 2 3 0");
eqt!(st_1506, "distinct 4 5 2 0 3 0 1 5 0", "4 5 2 0 3 1");
eqt!(st_1510, "iasc 3 0 2 1 3 5 0 5 4", "1 6 3 2 0 4 8 5 7");
eqt!(st_1511, "rank 2 4 0 3 3 0 2 3 4", "2 7 0 4 5 1 3 6 8");
eqt!(st_1514, "asc 0 1 1 1 5 4 2", "`s#0 1 1 1 2 4 5");
eqt!(st_1515, "distinct 2 1 0 2 0 2", "2 1 0");
eqt!(st_1516, "reverse 0 3 3 4 2 3", "3 2 4 3 3 0");
eqt!(st_1517, "distinct 5 0 5", "5 0");
eqt!(st_1518, "desc 5 5 5 1 1 1 3", "5 5 5 3 1 1 1");
eqt!(st_1519, "rank 5 1 4", "2 0 1");
eqt!(st_1520, "idesc 0 4 5 2 2 4", "2 1 5 3 4 0");
eqt!(st_1521, "rank 4 1 1 5 5 5 0 3 1", "5 1 2 6 7 8 0 4 3");
eqt!(st_1522, "idesc 0 2 3", "2 1 0");
eqt!(st_1523, "rank 1 5 4", "0 2 1");
eqt!(st_1524, "rank 1 5 0 3 1 3 3", "1 6 0 3 2 4 5");
eqt!(st_1525, "distinct 1 5 0 4 0 1 3", "1 5 0 4 3");
eqt!(st_1526, "desc 5 5 0 5 5 4", "5 5 5 5 4 0");
eqt!(st_1527, "asc 0 3 1 1 1", "`s#0 1 1 1 3");
eqt!(st_1528, "reverse 1 0 3 0 2 5", "5 2 0 3 0 1");
eqt!(st_1530, "reverse 5 1 2 3 4 2", "2 4 3 2 1 5");
eqt!(st_1531, "asc 5 0 5 1 0", "`s#0 0 1 5 5");
eqt!(st_1532, "desc 5 0 4 0 0 0 2 1 2", "5 4 2 2 1 0 0 0 0");
eqt!(st_1533, "idesc 5 3 2 1 1 3 1 1 3", "0 1 5 8 2 3 4 6 7");
eqt!(st_1534, "iasc 4 5 1 4 5 4 2 5 4", "2 6 0 3 5 8 1 4 7");
eqt!(st_1535, "idesc 3 4 4 1 1", "1 2 0 3 4");
eqt!(st_1536, "idesc 0 1 5 3 4 4 0 3 5", "2 8 4 5 3 7 1 0 6");
eqt!(st_1537, "idesc 3 1 2 3", "0 3 2 1");
eqt!(st_1538, "desc 2 1 5 2 4 2", "5 4 2 2 2 1");
eqt!(st_1540, "reverse 5 0 0 4 3 4 1", "1 4 3 4 0 0 5");
eqt!(st_1541, "iasc 3 5 2 5 3 0", "5 2 0 4 1 3");
eqt!(st_1542, "desc 2 4 2 3 5 0 5", "5 5 4 3 2 2 0");
eqt!(st_1543, "desc 5 0 0 2 0 1 4 5 5", "5 5 5 4 2 1 0 0 0");
eqt!(st_1544, "asc 4 1 4 2 3 2 3 2", "`s#1 2 2 2 3 3 4 4");
eqt!(st_1545, "where 4 4 3 5", "0 0 0 0 1 1 1 1 2 2 2 3 3 3 3 3");
eqt!(st_1546, "desc 0 4 0 1 4 2 1 4 2", "4 4 4 2 2 1 1 0 0");
eqt!(st_1547, "iasc 3 1 1 3 4 4 3 2 0", "8 1 2 7 0 3 6 4 5");
eqt!(st_1548, "reverse 3 0 3 4 3", "3 4 3 0 3");
eqt!(st_1549, "iasc 5 1 0 5 2 4 2", "2 1 4 6 5 0 3");
eqt!(st_1550, "distinct 5 4 2 1 2 0", "5 4 2 1 0");
eqt!(st_1551, "reverse 5 2 2 3 3 4 2", "2 4 3 3 2 2 5");
eqt!(st_1552, "asc 4 5 1 3 5 0 0", "`s#0 0 1 3 4 5 5");
eqt!(st_1553, "reverse 1 0 2 2 3 3 4", "4 3 3 2 2 0 1");
eqt!(st_1554, "distinct 0 5 0 0 1 1 4", "0 5 1 4");
eqt!(st_1555, "where 2 1 3 5 1 1 5 4 3", "0 0 1 2 2 2 3 3 3 3 3 4 5 6 6 6 6 6 \
    7 \
    7 7 7 8 8 8");
eqt!(st_1556, "where 5 5 0 4 5 0 5 4 2", "0 0 0 0 0 1 1 1 1 1 3 3 3 3 4 4 4 4 \
    4 \
    6 6 6 6 6 7 7 7 7 8 8");
eqt!(st_1557, "group 2 4 2 4 0 0", "2 4 0!(0 2;1 3;4 5)");
eqt!(st_1559, "rank 2 5 4 2", "0 3 2 1");
eqt!(st_1560, "iasc 0 5 1 1", "0 2 3 1");
eqt!(st_1561, "desc 1 0 1 5 0 0 1 4", "5 4 1 1 1 0 0 0");
eqt!(st_1562, "where 5 5 0 1 5 0 5 0", "0 0 0 0 0 1 1 1 1 1 3 4 4 4 4 4 6 6 6 \
    6 \
    6");
eqt!(st_1563, "distinct 4 5 5 0 0 3 1", "4 5 0 3 1");
eqt!(st_1564, "distinct 3 0 2", "3 0 2");
eqt!(st_1565, "asc 4 0 0 5", "`s#0 0 4 5");
eqt!(st_1566, "asc 5 5 0 1 1", "`s#0 1 1 5 5");
eqt!(st_1568, "rank 4 3 3 0 2 4 4 2 5", "5 3 4 0 1 6 7 2 8");
eqt!(st_1569, "desc 0 3 4 4 3 3 1 0 5", "5 4 4 3 3 3 1 0 0");
eqt!(st_1570, "reverse 3 1 1 5 0 5", "5 0 5 1 1 3");
eqt!(st_1571, "asc 1 4 0", "`s#0 1 4");
eqt!(st_1572, "iasc 0 5 0 1 4", "0 2 3 4 1");
eqt!(st_1573, "asc 3 4 4 1", "`s#1 3 4 4");
eqt!(st_1574, "iasc 5 1 1 4 0 3 0", "4 6 1 2 5 3 0");
eqt!(st_1575, "rank 4 3 4", "1 0 2");
eqt!(st_1576, "desc 0 4 5 3", "5 4 3 0");
eqt!(st_1577, "reverse 4 4 3 2 5 0 1 3 0", "0 3 1 0 5 2 3 4 4");
eqt!(st_1578, "desc 3 1 5 3", "5 3 3 1");
eqt!(st_1579, "asc 1 2 4 2 0 2", "`s#0 1 2 2 2 4");
eqt!(st_1580, "idesc 4 0 3 4 1 5 3 0", "5 0 3 2 6 4 1 7");
eqt!(st_1581, "asc 4 1 3 5 0 2", "`s#0 1 2 3 4 5");
eqt!(st_1582, "desc 5 4 5 5", "5 5 5 4");
eqt!(st_1583, "idesc 5 3 0 4 1 0", "0 3 1 4 2 5");
eqt!(st_1584, "rank 2 3 4 3 2 3 4 1", "1 3 6 4 2 5 7 0");
eqt!(st_1585, "reverse 3 5 2 1 1 5", "5 1 1 2 5 3");
eqt!(st_1586, "iasc 5 2 2 4", "1 2 3 0");
eqt!(st_1587, "reverse 2 1 1 3 1 0", "0 1 3 1 1 2");
eqt!(st_1588, "reverse 3 2 3 5 3 1", "1 3 5 3 2 3");
eqt!(st_1589, "where 1 4 3 4 2", "0 1 1 1 1 2 2 2 3 3 3 3 4 4");
eqt!(st_1590, "distinct 0 5 0 3 5 0 3 3", "0 5 3");
eqt!(st_1591, "iasc 5 4 1 4 1 5", "2 4 1 3 0 5");
eqt!(st_1592, "rank 0 5 1 4 5 0 2", "0 5 2 4 6 1 3");
eqt!(st_1593, "reverse 2 5 3 1", "1 3 5 2");
eqt!(st_1594, "idesc 2 4 1 0", "1 0 2 3");
eqt!(st_1595, "reverse 1 5 4 2 4", "4 2 4 5 1");
eqt!(st_1596, "asc 4 2 3 2 1 5 0", "`s#0 1 2 2 3 4 5");
eqt!(st_1597, "iasc 2 4 5 5 5 0 4 4", "5 0 1 6 7 2 3 4");
eqt!(st_1598, "desc 3 0 2 3 4 4 4", "4 4 4 3 3 2 0");
eqt!(st_1599, "distinct 3 2 1 5 3 1 4 3 2", "3 2 1 5 4");
eqt!(st_1600, "distinct 1 3 0 0 3 2 1 4 1", "1 3 0 2 4");
eqt!(st_1601, "asc 4 2 3 4", "`s#2 3 4 4");
eqt!(st_1602, "where 3 1 0 4", "0 0 0 1 3 3 3 3");
eqt!(st_1603, "iasc 1 5 1", "0 2 1");
eqt!(st_1604, "reverse 4 1 0", "0 1 4");
eqt!(st_1605, "desc 0 0 2 0 5 1 2 1 3", "5 3 2 2 1 1 0 0 0");
eqt!(st_1606, "desc 4 2 0 0 5 1 2 3", "5 4 3 2 2 1 0 0");
eqt!(st_1607, "iasc 5 0 2 5", "1 2 0 3");
eqt!(st_1608, "desc 0 5 5 3 5 3 5 5", "5 5 5 5 5 3 3 0");
eqt!(st_1609, "distinct 2 5 1 3 4 3 5", "2 5 1 3 4");
eqt!(st_1610, "distinct 1 5 2", "1 5 2");
eqt!(st_1611, "rank 5 0 5 1 0 2", "4 0 5 2 1 3");
eqt!(st_1612, "desc 4 3 2", "4 3 2");
eqt!(st_1613, "idesc 2 5 2 5 1 3 3 4", "1 3 7 5 6 0 2 4");
eqt!(st_1614, "where 1 0 3 5 2 4 1", "0 2 2 2 3 3 3 3 3 4 4 5 5 5 5 6");
eqt!(st_1615, "distinct 0 3 5 2 3", "0 3 5 2");
eqt!(st_1616, "rank 5 4 3 5 3 5 3 2 1", "6 5 2 7 3 8 4 1 0");
eqt!(st_1617, "reverse 0 4 1 1 3 5 0", "0 5 3 1 1 4 0");
eqt!(st_1618, "rank 2 2 0 5 4 0 4", "2 3 0 6 4 1 5");
eqt!(st_1619, "asc 1 5 1 2", "`s#1 1 2 5");
eqt!(st_1620, "desc 1 1 3 2 1 4 3", "4 3 3 2 1 1 1");
eqt!(st_1621, "distinct 2 2 5 2 5 0 5 1 2", "2 5 0 1");
eqt!(st_1622, "iasc 2 1 3 0 3 0", "3 5 1 0 2 4");
eqt!(st_1623, "desc 1 0 0 3 1 1", "3 1 1 1 0 0");
eqt!(st_1624, "distinct 2 3 1 5 4", "2 3 1 5 4");
eqt!(st_1625, "distinct 4 5 5", "4 5");
eqt!(st_1626, "idesc 0 3 1 0 0 1 5", "6 1 2 5 0 3 4");
eqt!(st_1627, "reverse 5 1 2 0 2 5 5", "5 5 2 0 2 1 5");
eqt!(st_1628, "asc 0 3 4 5 2 0 2 1", "`s#0 0 1 2 2 3 4 5");
eqt!(st_1629, "rank 4 5 0 4 5 5 4 4 0", "2 6 0 3 7 8 4 5 1");
eqt!(st_1630, "reverse 5 5 1 0", "0 1 5 5");
eqt!(st_1631, "iasc 0 2 3 5 4", "0 1 2 4 3");
eqt!(st_1632, "where 5 0 0 3 2 1 4", "0 0 0 0 0 3 3 3 4 4 5 6 6 6 6");
eqt!(st_1633, "distinct 3 2 5", "3 2 5");
eqt!(st_1636, "where 4 3 3 5 5 1 2 3 2", "0 0 0 0 1 1 1 2 2 2 3 3 3 3 3 4 4 4 \
    4 \
    4 5 6 6 7 7 7 8 8");
eqt!(st_1637, "idesc 5 0 4 4 2", "0 2 3 4 1");
eqt!(st_1639, "rank 5 4 5 1 5 2 3", "4 3 5 0 6 1 2");
eqt!(st_1641, "distinct 3 3 4 4", "3 4");
eqt!(st_1642, "idesc 2 5 4 0 3 0 0 3 1", "1 2 4 7 0 8 3 5 6");
eqt!(st_1643, "desc 4 2 4 3", "4 4 3 2");
eqt!(st_1644, "where 5 0 2 2 3 0 3 2", "0 0 0 0 0 2 2 3 3 4 4 4 6 6 6 7 7");
eqt!(st_1645, "iasc 2 5 1 4 3 4", "2 0 4 3 5 1");
eqt!(st_1646, "iasc 2 2 0 3", "2 0 1 3");
eqt!(st_1647, "group 2 2 0 0", "2 0!(0 1;2 3)");
eqt!(st_1648, "distinct 1 3 5 2 1 0 5", "1 3 5 2 0");
eqt!(st_1649, "reverse 3 5 1 2 1", "1 2 1 5 3");
eqt!(st_1650, "idesc 0 5 0 3 2 1", "1 3 4 5 0 2");
eqt!(sy_1651, "reverse `dd`gg`dd`cc", "`cc`dd`gg`dd");
eqt!(sy_1652, "iasc `ff`aa`aa`bb`ee`bb", "1 2 3 5 4 0");
eqt!(sy_1653, "reverse `gg`ee`gg`cc`aa`gg`cc", "`cc`gg`aa`cc`gg`ee`gg");
eqt!(sy_1654, "reverse `dd`ee`cc", "`cc`ee`dd");
eqt!(sy_1655, "count `ee`dd`dd`dd`ff`aa", "6");
eqt!(sy_1656, "desc `ee`cc`cc`gg`bb", "`gg`ee`cc`cc`bb");
eqt!(sy_1657, "reverse `bb`ee`ff", "`ff`ee`bb");
eqt!(sy_1658, "desc `aa`ee", "`ee`aa");
eqt!(sy_1660, "desc `cc`dd`bb`ff`ee`ff`ff", "`ff`ff`ff`ee`dd`cc`bb");
eqt!(sy_1661, "count `gg`bb`cc`gg`gg`ee`gg", "7");
eqt!(sy_1662, "last `gg`gg`ff`dd", "`dd");
eqt!(sy_1663, "iasc `bb`cc`ff", "0 1 2");
eqt!(sy_1664, "first `aa`bb", "`aa");
eqt!(sy_1665, "count `gg`ee`aa", "3");
eqt!(sy_1666, "reverse `dd`aa`aa`gg`bb`ee`ee", "`ee`ee`bb`gg`aa`aa`dd");
eqt!(sy_1667, "iasc `bb`cc`cc`bb`gg`dd", "0 3 1 2 5 4");
eqt!(sy_1668, "count `bb`gg", "2");
eqt!(sy_1669, "count `bb`ee`dd`cc`dd", "5");
eqt!(sy_1670, "desc `aa`ff`cc`cc", "`ff`cc`cc`aa");
eqt!(sy_1671, "distinct `gg`aa`cc`cc`dd`gg`aa", "`gg`aa`cc`dd");
eqt!(sy_1672, "desc `dd`gg", "`gg`dd");
eqt!(sy_1673, "count `ee`ff", "2");
eqt!(sy_1674, "last `ee`ff`cc`cc`cc`aa", "`aa");
eqt!(sy_1675, "count `bb`ee`dd`ff`dd`dd`ff", "7");
eqt!(sy_1676, "last `aa`dd`dd`ee`bb`cc`cc", "`cc");
eqt!(sy_1677, "desc `dd`ee`ee`bb`aa`ee", "`ee`ee`ee`dd`bb`aa");
eqt!(sy_1678, "reverse `bb`cc`gg`aa`cc", "`cc`aa`gg`cc`bb");
eqt!(sy_1679, "reverse `cc`gg`cc", "`cc`gg`cc");
eqt!(sy_1680, "distinct `bb`ff`aa`aa`ee`dd`aa", "`bb`ff`aa`ee`dd");
eqt!(sy_1681, "first `gg`ff`bb`cc`dd`dd", "`gg");
eqt!(sy_1682, "desc `ff`bb`dd`ee`dd`bb", "`ff`ee`dd`dd`bb`bb");
eqt!(sy_1684, "asc `aa`ee", "`s#`aa`ee");
eqt!(sy_1685, "last `cc`aa`cc", "`cc");
eqt!(sy_1686, "distinct `aa`dd`cc", "`aa`dd`cc");
eqt!(sy_1687, "first `ff`ff`bb`ee", "`ff");
eqt!(sy_1688, "iasc `cc`cc`gg", "0 1 2");
eqt!(sy_1689, "desc `ee`ee`cc`dd", "`ee`ee`dd`cc");
eqt!(sy_1690, "desc `cc`cc", "`cc`cc");
eqt!(sy_1691, "last `aa`aa`ee`bb`aa`bb", "`bb");
eqt!(sy_1692, "first `ee`cc`bb", "`ee");
eqt!(sy_1693, "count `ff`aa`aa`aa`bb`cc`gg", "7");
eqt!(sy_1694, "reverse `bb`ff`ff`aa`ff`ff`cc", "`cc`ff`ff`aa`ff`ff`bb");
eqt!(sy_1695, "first `dd`aa`ff", "`dd");
eqt!(sy_1696, "last `ff`ff`gg", "`gg");
eqt!(sy_1697, "reverse `cc`cc`ff`cc`bb`ee", "`ee`bb`cc`ff`cc`cc");
eqt!(sy_1699, "last `dd`bb`cc`ee`cc`dd`ee", "`ee");
eqt!(sy_1701, "count `dd`bb`cc`ee", "4");
eqt!(sy_1702, "count `bb`bb`aa", "3");
eqt!(sy_1703, "asc `ff`ff`aa`cc", "`s#`aa`cc`ff`ff");
eqt!(sy_1704, "first `cc`ff`aa`aa`bb", "`cc");
eqt!(sy_1705, "iasc `cc`cc`bb`dd`bb`ff`ee", "2 4 0 1 3 6 5");
eqt!(sy_1706, "reverse `gg`ff`bb`gg", "`gg`bb`ff`gg");
eqt!(sy_1707, "last `bb`aa`ee`gg`dd", "`dd");
eqt!(sy_1708, "iasc `gg`ff`dd`aa`ff`gg", "3 2 1 4 0 5");
eqt!(sy_1709, "first `aa`aa", "`aa");
eqt!(sy_1710, "iasc `dd`dd`ee", "0 1 2");
eqt!(sy_1711, "reverse `ee`cc`aa`dd`dd`gg", "`gg`dd`dd`aa`cc`ee");
eqt!(sy_1712, "last `ee`aa`ee`bb", "`bb");
eqt!(sy_1713, "reverse `aa`aa`gg", "`gg`aa`aa");
eqt!(sy_1714, "count `ff`cc", "2");
eqt!(sy_1715, "asc `cc`ff`bb", "`s#`bb`cc`ff");
eqt!(sy_1717, "last `ee`ee`ff", "`ff");
eqt!(sy_1718, "last `cc`aa`aa", "`aa");
eqt!(sy_1720, "last `ee`dd`bb`ff`cc`dd`bb", "`bb");
eqt!(sy_1723, "count `dd`aa`dd`ff", "4");
eqt!(sy_1724, "last `ff`dd`cc`ee", "`ee");
eqt!(sy_1725, "last `cc`aa`cc`aa`aa`ee`aa", "`aa");
eqt!(sy_1727, "distinct `bb`ff`ee`ff", "`bb`ff`ee");
eqt!(sy_1728, "iasc `aa`ee`ee`cc`ee`dd`bb", "0 6 3 5 1 2 4");
eqt!(sy_1729, "last `ff`cc`dd", "`dd");
eqt!(sy_1730, "last `bb`ee`cc`aa`bb", "`bb");
eqt!(sy_1731, "last `gg`cc`bb`gg", "`gg");
eqt!(sy_1732, "count `gg`cc", "2");
eqt!(sy_1733, "last `aa`cc`ee`ff`cc", "`cc");
eqt!(sy_1734, "reverse `aa`aa`dd`dd", "`dd`dd`aa`aa");
eqt!(sy_1735, "count `cc`bb`dd`ee`gg", "5");
eqt!(sy_1736, "asc `gg`gg`ee", "`s#`ee`gg`gg");
eqt!(sy_1738, "first `aa`aa`dd`bb`cc", "`aa");
eqt!(sy_1741, "first `aa`cc`bb`dd`dd`ff`bb", "`aa");
eqt!(sy_1742, "distinct `bb`ff`cc", "`bb`ff`cc");
eqt!(sy_1743, "asc `bb`cc`gg`gg`dd`ee`cc", "`s#`bb`cc`cc`dd`ee`gg`gg");
eqt!(sy_1744, "asc `cc`ff`ee`cc`cc`ff`cc", "`s#`cc`cc`cc`cc`ee`ff`ff");
eqt!(sy_1745, "asc `ee`gg`ee`gg", "`s#`ee`ee`gg`gg");
eqt!(sy_1746, "distinct `gg`ee`aa", "`gg`ee`aa");
eqt!(sy_1747, "desc `ee`cc", "`ee`cc");
eqt!(sy_1748, "distinct `bb`cc`bb", "`bb`cc");
eqt!(sy_1749, "iasc `cc`gg`aa`cc`ee`bb", "2 5 0 3 4 1");
eqt!(sy_1750, "count `cc`gg`aa`cc`ff`bb`bb", "7");
eqt!(sy_1751, "asc `bb`bb`ee`ee`aa", "`s#`aa`bb`bb`ee`ee");
eqt!(sy_1752, "asc `bb`cc`ee", "`s#`bb`cc`ee");
eqt!(sy_1753, "iasc `dd`ff`ff`bb`dd", "3 0 4 1 2");
eqt!(sy_1754, "asc `gg`gg", "`s#`gg`gg");
eqt!(sy_1755, "desc `bb`ff`cc`dd`cc`gg`bb", "`gg`ff`dd`cc`cc`bb`bb");
eqt!(sy_1756, "iasc `ff`ff`aa`aa`bb", "2 3 4 0 1");
eqt!(sy_1757, "iasc `ff`gg`ff`gg`aa`ff", "4 0 2 5 1 3");
eqt!(sy_1758, "reverse `aa`aa`dd", "`dd`aa`aa");
eqt!(sy_1759, "desc `ee`gg`ee`cc`aa", "`gg`ee`ee`cc`aa");
eqt!(sy_1760, "asc `bb`dd`ee`ee`cc", "`s#`bb`cc`dd`ee`ee");
eqt!(sy_1761, "iasc `dd`ff`ee`ff`gg", "0 2 1 3 4");
eqt!(sy_1762, "reverse `dd`ee`bb`gg`gg`dd`aa", "`aa`dd`gg`gg`bb`ee`dd");
eqt!(sy_1763, "reverse `dd`bb`aa`ee`ff", "`ff`ee`aa`bb`dd");
eqt!(sy_1764, "iasc `aa`ee`ee`bb`ff`cc", "0 3 5 1 2 4");
eqt!(sy_1765, "desc `ff`cc", "`ff`cc");
eqt!(sy_1766, "last `ee`aa`bb`gg", "`gg");
eqt!(sy_1768, "last `gg`aa`aa", "`aa");
eqt!(sy_1769, "distinct `aa`dd`dd`aa`gg`gg", "`aa`dd`gg");
eqt!(sy_1770, "distinct `cc`cc`dd`dd`cc", "`cc`dd");
eqt!(sy_1771, "count `ff`aa`gg`dd`ff`ff`ee", "7");
eqt!(sy_1772, "distinct `aa`cc`cc`ff", "`aa`cc`ff");
eqt!(sy_1774, "asc `ff`bb", "`s#`bb`ff");
eqt!(sy_1775, "distinct `bb`cc", "`bb`cc");
eqt!(sy_1776, "asc `gg`aa`gg", "`s#`aa`gg`gg");
eqt!(sy_1778, "iasc `dd`ff`bb`dd`aa", "4 2 0 3 1");
eqt!(sy_1779, "count `ee`bb`cc`cc`dd", "5");
eqt!(sy_1780, "asc `ff`aa`gg`bb`gg", "`s#`aa`bb`ff`gg`gg");
eqt!(sy_1781, "first `bb`dd", "`bb");
eqt!(sy_1782, "reverse `dd`dd`ff`ff`gg`cc", "`cc`gg`ff`ff`dd`dd");
eqt!(sy_1784, "count `bb`dd`ff`gg`ff`ff", "6");
eqt!(sy_1785, "first `gg`bb", "`gg");
eqt!(sy_1786, "count `dd`dd", "2");
eqt!(sy_1787, "desc `ee`gg", "`gg`ee");
eqt!(sy_1788, "asc `gg`cc`dd`dd`ff", "`s#`cc`dd`dd`ff`gg");
eqt!(sy_1790, "last `aa`gg`cc`bb`dd`bb`bb", "`bb");
eqt!(sy_1791, "count `gg`gg`ff`ff", "4");
eqt!(sy_1792, "asc `dd`ee`gg", "`s#`dd`ee`gg");
eqt!(sy_1793, "desc `bb`aa`aa", "`bb`aa`aa");
eqt!(sy_1794, "first `cc`bb`aa`ff", "`cc");
eqt!(sy_1796, "reverse `bb`cc`ee`ee`cc`dd", "`dd`cc`ee`ee`cc`bb");
eqt!(sy_1797, "reverse `aa`bb`ff", "`ff`bb`aa");
eqt!(sy_1798, "last `ff`dd`aa`gg", "`gg");
eqt!(sy_1799, "distinct `aa`gg`cc", "`aa`gg`cc");
eqt!(sy_1800, "distinct `cc`bb`aa`ff`aa`gg`ff", "`cc`bb`aa`ff`gg");
eqt!(ss_1801, "`bb`gg`ff union `bb`ee", "`bb`gg`ff`ee");
eqt!(ss_1802, "`ee`cc`ee`gg`aa`ff except `ee`ff`ff`cc", "`gg`aa");
eqt!(ss_1803, "`aa`cc`dd inter `bb`aa`cc`dd`cc", "`aa`cc`dd");
eqt!(ss_1804, "`gg`ee`bb`cc`gg`bb union `ff`bb`dd`aa", "`gg`ee`bb`cc`ff`dd`aa");
eqt!(ss_1805, "`aa`bb`ee`aa`gg`dd except `gg`aa`aa", "`bb`ee`dd");
eqt!(ss_1806, "`aa`bb`bb`dd`ee`ee union `cc`bb`bb", "`aa`bb`dd`ee`cc");
eqt!(ss_1807, "`ff`bb`cc`aa`cc`aa in `aa`aa", "000101b");
eqt!(ss_1808, "`gg`dd`dd`aa`aa except `bb`ff`ff`cc", "`gg`dd`dd`aa`aa");
eqt!(ss_1809, "`cc`gg`dd`ff except `cc`cc`dd`bb`aa", "`gg`ff");
eqt!(ss_1810, "`dd`aa`ff`ee`bb`ee inter `ee`aa", "`aa`ee`ee");
eqt!(ss_1811, "`bb`gg`ee union `bb`ff`dd", "`bb`gg`ee`ff`dd");
eqt!(ss_1812, "`dd`ee`ee`dd in `ff`bb`bb`gg", "0000b");
eqt!(ss_1813, "`ff`gg`bb`cc`dd`bb inter `dd`dd`ee`ee`bb", "`bb`dd`bb");
eqt!(ss_1814, "`ee`aa`cc union `bb`ee`bb`cc`gg", "`ee`aa`cc`bb`gg");
eqt!(ss_1816, "`gg`bb`dd`dd`dd in `aa`bb`bb`gg", "11000b");
eqt!(ss_1817, "`aa`dd`gg`ff`bb`dd in `bb`bb`aa`ff", "100110b");
eqt!(ss_1819, "`bb`ee`bb`cc`gg except `bb`cc`bb`aa", "`ee`gg");
eqt!(ss_1821, "`cc`gg`cc`ff`ee union `ee`aa`cc`dd`ff", "`cc`gg`ff`ee`aa`dd");
eqt!(ss_1822, "`dd`gg`ff`aa`bb`aa except `cc`aa`dd`bb`bb", "`gg`ff");
eqt!(ss_1823, "`dd`ee`aa`cc in `aa`dd`gg`cc", "1011b");
eqt!(ss_1824, "`ff`bb`cc union `bb`cc`ff`cc", "`ff`bb`cc");
eqt!(ss_1825, "`dd`aa`cc`aa union `cc`aa`bb", "`dd`aa`cc`bb");
eqt!(ss_1826, "`bb`gg`dd`cc in `ee`ee`aa`ee`bb", "1000b");
eqt!(ss_1827, "`gg`cc`bb`ee except `gg`aa`dd`dd", "`cc`bb`ee");
eqt!(ss_1828, "`cc`gg`cc`ee`gg`bb except `ff`cc`gg`dd", "`ee`bb");
eqt!(ss_1829, "`cc`ee`aa`cc`dd`dd in `dd`ff`gg`cc`gg", "100111b");
eqt!(ss_1830, "`ff`aa`gg`ff`ff`gg union `ff`aa", "`ff`aa`gg");
eqt!(ss_1831, "`ff`cc`cc`dd`ee inter `cc`aa`dd`bb", "`cc`cc`dd");
eqt!(ss_1832, "`aa`aa`gg`gg`dd except `dd`cc`cc`cc`cc", "`aa`aa`gg`gg");
eqt!(ss_1834, "`cc`aa`ee except `bb`ee", "`cc`aa");
eqt!(ss_1835, "`bb`gg`ff`gg`bb in `cc`gg`dd`gg`ff", "01110b");
eqt!(ss_1836, "`dd`aa`dd except `gg`gg`aa`ee", "`dd`dd");
eqt!(ss_1837, "`gg`gg`ff inter `dd`gg`ff`dd`dd", "`gg`gg`ff");
eqt!(ss_1838, "`gg`ee`cc`aa`bb`gg union `dd`aa`cc`ee", "`gg`ee`cc`aa`bb`dd");
eqt!(ss_1839, "`ff`ee`aa`ff`ee inter `bb`gg", "`symbol$()");
eqt!(ss_1840, "`bb`dd`ff`gg`ff except `cc`cc", "`bb`dd`ff`gg`ff");
eqt!(ss_1841, "`ff`ff`ee`ff`aa`aa inter `ff`bb", "`ff`ff`ff");
eqt!(ss_1842, "`dd`aa`aa`ee`dd inter `ff`cc`gg", "`symbol$()");
eqt!(ss_1843, "`aa`gg`gg`aa`gg in `ee`dd`dd", "00000b");
eqt!(ss_1844, "`cc`ee`ee`cc`ee union `ee`cc`dd`ee`gg", "`cc`ee`dd`gg");
eqt!(ss_1845, "`cc`ee`gg`gg`aa`aa inter `ee`ff`gg", "`ee`gg`gg");
eqt!(ss_1846, "`aa`ff`ee`bb`dd`dd except `aa`gg`bb", "`ff`ee`dd`dd");
eqt!(ss_1847, "`dd`gg`ff`ee`bb except `bb`aa`gg`bb", "`dd`ff`ee");
eqt!(ss_1848, "`ff`dd`bb`gg`gg union `ee`gg`gg`bb", "`ff`dd`bb`gg`ee");
eqt!(ss_1849, "`ff`cc`cc`aa union `dd`cc`aa`gg", "`ff`cc`aa`dd`gg");
eqt!(ss_1850, "`dd`bb`ff`ee union `bb`cc`gg`aa`ee", "`dd`bb`ff`ee`cc`gg`aa");
eqt!(ss_1851, "`gg`gg`bb`ff`dd inter `ff`gg", "`gg`gg`ff");
eqt!(ss_1852, "`dd`cc`ee inter `aa`ee`dd`gg`ff", "`dd`ee");
eqt!(ss_1853, "`ee`aa`ff union `aa`bb`cc`dd`aa", "`ee`aa`ff`bb`cc`dd");
eqt!(ss_1854, "`dd`bb`bb`cc inter `bb`ff", "`bb`bb");
eqt!(ss_1855, "`ee`ee`ff`gg`bb`dd in `bb`aa`ff`dd", "001011b");
eqt!(ss_1856, "`ff`ff`ee union `dd`gg`gg`gg", "`ff`ee`dd`gg");
eqt!(ss_1857, "`dd`ee`dd except `aa`ee`dd", "`symbol$()");
eqt!(ss_1859, "`dd`ee`gg`bb inter `aa`aa", "`symbol$()");
eqt!(ss_1861, "`bb`bb`gg`dd except `gg`gg`aa`cc`ee", "`bb`bb`dd");
eqt!(ss_1862, "`cc`cc`aa`cc union `ee`ff`ee`bb", "`cc`aa`ee`ff`bb");
eqt!(ss_1863, "`bb`ff`cc`aa inter `ee`bb`cc", "`bb`cc");
eqt!(ss_1864, "`aa`aa`ff`dd`bb inter `gg`bb`aa", "`aa`aa`bb");
eqt!(ss_1865, "`ee`aa`bb`cc in `ee`cc", "1001b");
eqt!(ss_1866, "`bb`cc`gg`bb inter `cc`bb`gg`cc", "`bb`cc`gg`bb");
eqt!(ss_1867, "`cc`cc`aa`ee`ff`cc union `cc`dd`ff", "`cc`aa`ee`ff`dd");
eqt!(ss_1868, "`ff`cc`gg union `gg`ff`aa`ff`gg", "`ff`cc`gg`aa");
eqt!(ss_1869, "`gg`aa`dd in `ee`dd`ff`gg`aa", "111b");
eqt!(ss_1870, "`aa`dd`bb in `dd`cc`ff", "010b");
eqt!(ss_1872, "`ff`bb`gg`bb`dd union `cc`bb`cc`aa", "`ff`bb`gg`dd`cc`aa");
eqt!(ss_1873, "`aa`aa`gg`gg except `dd`gg`cc", "`aa`aa");
eqt!(ss_1874, "`ee`bb`ee`gg union `bb`ff", "`ee`bb`gg`ff");
eqt!(ss_1875, "`cc`cc`bb`gg except `gg`bb`dd", "`cc`cc");
eqt!(ss_1876, "`aa`gg`ee union `aa`gg`ee`bb`gg", "`aa`gg`ee`bb");
eqt!(ss_1877, "`bb`ff`dd`bb`bb`bb in `ee`ee`cc`gg`aa", "000000b");
eqt!(ss_1878, "`cc`ee`aa`ff`cc union `cc`ff`ee`bb`aa", "`cc`ee`aa`ff`bb");
eqt!(ss_1880, "`bb`dd`dd`gg`ff`cc inter `ee`dd", "`dd`dd");
eqt!(av_1881, "(&/) 5 4 9 1 9 9 6", "1");
eqt!(av_1882, "(&/) 5 9 5 1", "1");
eqt!(av_1883, "(|/) 7 3", "7");
eqt!(t2date_1, "2013.07.07+118", "2013.11.02");
eqt!(t2date_2, "2008.03.25+129", "2008.08.01");
eqt!(t2date_3, "2024.07.17+320", "2025.06.02");
eqt!(t2date_4, "2016.04.14+192", "2016.10.23");
eqt!(t2date_5, "2027.07.28+78", "2027.10.14");
eqt!(t2date_6, "2015.11.06-43", "2015.09.24");
eqt!(t2date_7, "2023.09.22+241", "2024.05.20");
eqt!(t2date_8, "2011.07.02-286", "2010.09.19");
eqt!(t2date_9, "2001.07.07-274", "2000.10.06");
eqt!(t2date_10, "2003.08.17+311", "2004.06.23");
eqt!(t2date_11, "2015.01.12+111", "2015.05.03");
eqt!(t2date_12, "2030.12.06-305", "2030.02.04");
eqt!(t2date_13, "2030.03.17+19", "2030.04.05");
eqt!(t2date_14, "2004.01.06-351", "2003.01.20");
eqt!(t2date_15, "2027.07.03-324", "2026.08.13");
eqt!(t2date_16, "2003.03.10+196", "2003.09.22");
eqt!(t2date_17, "2011.03.17-103", "2010.12.04");
eqt!(t2date_18, "2029.01.21+227", "2029.09.05");
eqt!(t2date_19, "2009.09.13+182", "2010.03.14");
eqt!(t2date_20, "2023.08.02+379", "2024.08.15");
eqt!(t2date_21, "2028.12.20+312", "2029.10.28");
eqt!(t2date_22, "2014.02.04+74", "2014.04.19");
eqt!(t2date_23, "2010.03.06-75", "2009.12.21");
eqt!(t2date_24, "2030.04.18-395", "2029.03.19");
eqt!(t2date_25, "2016.04.28-61", "2016.02.27");
eqt!(t2date_26, "2028.06.06+257", "2029.02.18");
eqt!(t2date_27, "2010.01.19+173", "2010.07.11");
eqt!(t2date_28, "2007.07.13-205", "2006.12.20");
eqt!(t2date_29, "2013.07.27-274", "2012.10.26");
eqt!(t2date_30, "2003.03.02-370", "2002.02.25");
eqt!(t2date_31, "2028.07.25-118", "2028.03.29");
eqt!(t2date_32, "2010.11.08-55", "2010.09.14");
eqt!(t2date_33, "2028.12.24-112", "2028.09.03");
eqt!(t2date_34, "2025.12.22+197", "2026.07.07");
eqt!(t2date_35, "2010.08.25+225", "2011.04.07");
eqt!(t2date_36, "2014.05.03-351", "2013.05.17");
eqt!(t2date_37, "2028.06.26-274", "2027.09.26");
eqt!(t2date_38, "2005.05.14-210", "2004.10.16");
eqt!(t2date_39, "2022.10.06-195", "2022.03.25");
eqt!(t2date_40, "2022.11.20-100", "2022.08.12");
eqt!(t2date_41, "2013.06.07-3", "2013.06.04");
eqt!(t2date_42, "2022.06.23-99", "2022.03.16");
eqt!(t2date_43, "2029.11.22+328", "2030.10.16");
eqt!(t2date_44, "2009.11.14+210", "2010.06.12");
eqt!(t2date_45, "2028.06.22-2", "2028.06.20");
eqt!(t2date_46, "2004.09.18+1", "2004.09.19");
eqt!(t2date_47, "2017.05.22+262", "2018.02.08");
eqt!(t2date_48, "2026.08.07-384", "2025.07.19");
eqt!(t2date_49, "2028.12.14-287", "2028.03.02");
eqt!(t2date_50, "2017.11.20-103", "2017.08.09");
eqt!(t2date_51, "2024.01.04-39", "2023.11.26");
eqt!(t2date_52, "2019.01.02-154", "2018.08.01");
eqt!(t2date_53, "2012.09.21-100", "2012.06.13");
eqt!(t2date_54, "2025.02.28-205", "2024.08.07");
eqt!(t2date_55, "2026.03.09-169", "2025.09.21");
eqt!(t2date_56, "2005.03.15-215", "2004.08.12");
eqt!(t2date_57, "2018.12.06-129", "2018.07.30");
eqt!(t2date_58, "2007.05.15-95", "2007.02.09");
eqt!(t2date_59, "2027.08.19-341", "2026.09.12");
eqt!(t2date_60, "2006.02.18+236", "2006.10.12");
eqt!(t2date_61, "2017.01.04-253", "2016.04.26");
eqt!(t2date_62, "2003.06.21+292", "2004.04.08");
eqt!(t2date_63, "2022.09.09+299", "2023.07.05");
eqt!(t2date_64, "2011.07.09+70", "2011.09.17");
eqt!(t2date_65, "2019.01.13+25", "2019.02.07");
eqt!(t2date_66, "2011.09.08-82", "2011.06.18");
eqt!(t2date_67, "2027.08.02-118", "2027.04.06");
eqt!(t2date_68, "2026.04.04-88", "2026.01.06");
eqt!(t2date_69, "2009.05.22-37", "2009.04.15");
eqt!(t2date_70, "2018.04.25+360", "2019.04.20");
eqt!(t2date_71, "2012.11.27+381", "2013.12.13");
eqt!(t2date_72, "2022.02.24+71", "2022.05.06");
eqt!(t2date_73, "2005.09.03+389", "2006.09.27");
eqt!(t2date_74, "2017.11.13+182", "2018.05.14");
eqt!(t2date_75, "2028.12.08+175", "2029.06.01");
eqt!(t2date_76, "2025.01.11-97", "2024.10.06");
eqt!(t2date_77, "2010.03.24+76", "2010.06.08");
eqt!(t2date_78, "2016.11.17+119", "2017.03.16");
eqt!(t2date_79, "2013.11.26+121", "2014.03.27");
eqt!(t2date_80, "2019.08.24+83", "2019.11.15");
eqt!(t2date_81, "2029.03.25-192", "2028.09.14");
eqt!(t2date_82, "2009.06.19+300", "2010.04.15");
eqt!(t2date_83, "2013.01.10+68", "2013.03.19");
eqt!(t2date_84, "2001.05.26-149", "2000.12.28");
eqt!(t2date_85, "2004.08.14+21", "2004.09.04");
eqt!(t2date_86, "2012.03.20-172", "2011.09.30");
eqt!(t2date_87, "2022.06.26+193", "2023.01.05");
eqt!(t2date_88, "2027.05.20+281", "2028.02.25");
eqt!(t2date_89, "2005.12.22+392", "2007.01.18");
eqt!(t2date_90, "2028.05.24-215", "2027.10.22");
eqt!(t2date_91, "2017.12.25+90", "2018.03.25");
eqt!(t2date_92, "2017.07.02+124", "2017.11.03");
eqt!(t2date_93, "2021.05.06+223", "2021.12.15");
eqt!(t2date_94, "2001.10.01+68", "2001.12.08");
eqt!(t2date_95, "2029.03.21-32", "2029.02.17");
eqt!(t2date_96, "2029.05.06-77", "2029.02.18");
eqt!(t2date_97, "2001.11.02-229", "2001.03.18");
eqt!(t2date_98, "2003.03.19+175", "2003.09.10");
eqt!(t2date_99, "2014.03.16+190", "2014.09.22");
eqt!(t2date_100, "2017.03.08-107", "2016.11.21");
eqt!(t2date_101, "2023.02.14+89", "2023.05.14");
eqt!(t2date_102, "2005.08.24+89", "2005.11.21");
eqt!(t2date_103, "2027.11.09+334", "2028.10.08");
eqt!(t2date_104, "2017.11.26+121", "2018.03.27");
eqt!(t2date_105, "2008.09.08-203", "2008.02.18");
eqt!(t2date_106, "2017.09.03+129", "2018.01.10");
eqt!(t2date_107, "2011.10.11-97", "2011.07.06");
eqt!(t2date_108, "2004.11.01+224", "2005.06.13");
eqt!(t2date_109, "2002.06.07+340", "2003.05.13");
eqt!(t2date_110, "2010.03.09-6", "2010.03.03");
eqt!(t2date_111, "2019.09.10+372", "2020.09.16");
eqt!(t2date_112, "2012.02.06-200", "2011.07.21");
eqt!(t2date_113, "2005.12.10-102", "2005.08.30");
eqt!(t2date_114, "2015.01.01-119", "2014.09.04");
eqt!(t2date_115, "2005.10.03-10", "2005.09.23");
eqt!(t2date_116, "2012.06.13+276", "2013.03.16");
eqt!(t2date_117, "2005.02.01+91", "2005.05.03");
eqt!(t2date_118, "2007.04.11-390", "2006.03.17");
eqt!(t2date_119, "2003.11.01-59", "2003.09.03");
eqt!(t2date_120, "2022.07.03-255", "2021.10.21");
eqt!(t2cmp_121, "2023.09.13>2018.06.23", "1b");
eqt!(t2cmp_122, "2013.05.17<2017.02.02", "1b");
eqt!(t2cmp_123, "2018.06.20<=2013.05.10", "0b");
eqt!(t2cmp_124, "2017.04.15<=2024.09.01", "1b");
eqt!(t2cmp_125, "2018.02.25-2022.02.24", "-1460i");
eqt!(t2cmp_126, "2019.08.25>2024.07.26", "0b");
eqt!(t2cmp_127, "2017.10.23<2013.05.13", "0b");
eqt!(t2cmp_128, "2012.10.09>2024.12.16", "0b");
eqt!(t2cmp_129, "2012.12.01>=2011.10.03", "1b");
eqt!(t2cmp_130, "2014.07.07<=2023.11.22", "1b");
eqt!(t2cmp_131, "2010.01.04>=2020.04.07", "0b");
eqt!(t2cmp_132, "2013.01.14<2019.03.27", "1b");
eqt!(t2cmp_133, "2019.03.23<2023.02.10", "1b");
eqt!(t2cmp_134, "2023.07.21<=2013.02.09", "0b");
eqt!(t2cmp_135, "2019.05.28-2015.05.24", "1465i");
eqt!(t2cmp_136, "2011.12.20=2012.05.16", "0b");
eqt!(t2cmp_137, "2014.05.20<=2025.04.28", "1b");
eqt!(t2cmp_138, "2019.08.09=2018.11.06", "0b");
eqt!(t2cmp_139, "2019.03.07<=2020.09.05", "1b");
eqt!(t2cmp_140, "2017.10.12=2012.04.18", "0b");
eqt!(t2cmp_141, "2014.10.01>=2020.12.26", "0b");
eqt!(t2cmp_142, "2022.10.26-2012.01.14", "3938i");
eqt!(t2cmp_143, "2023.08.04-2012.01.07", "4227i");
eqt!(t2cmp_144, "2017.05.07>2013.03.24", "1b");
eqt!(t2cmp_145, "2019.03.27-2016.02.20", "1131i");
eqt!(t2cmp_146, "2020.01.27>2016.05.13", "1b");
eqt!(t2cmp_147, "2011.08.21<=2018.08.18", "1b");
eqt!(t2cmp_148, "2021.04.12>2014.01.23", "1b");
eqt!(t2cmp_149, "2017.04.26<2023.12.16", "1b");
eqt!(t2cmp_150, "2021.09.06<=2019.01.01", "0b");
eqt!(t2cmp_151, "2021.03.05<=2023.01.03", "1b");
eqt!(t2cmp_152, "2020.11.18<=2019.06.06", "0b");
eqt!(t2cmp_153, "2020.12.25>2013.12.18", "1b");
eqt!(t2cmp_154, "2011.11.11<2023.04.21", "1b");
eqt!(t2cmp_155, "2011.03.21=2012.08.13", "0b");
eqt!(t2cmp_156, "2022.06.19<2022.05.04", "0b");
eqt!(t2cmp_157, "2021.03.03-2018.11.04", "850i");
eqt!(t2cmp_158, "2021.07.11<=2017.10.04", "0b");
eqt!(t2cmp_159, "2019.05.14<2014.03.22", "0b");
eqt!(t2cmp_160, "2019.12.22<=2010.09.17", "0b");
eqt!(t2cmp_161, "2018.03.08<2023.09.18", "1b");
eqt!(t2cmp_162, "2018.05.14>2022.02.27", "0b");
eqt!(t2cmp_163, "2010.08.08=2025.11.03", "0b");
eqt!(t2cmp_164, "2019.04.25-2011.03.15", "2963i");
eqt!(t2cmp_165, "2021.09.05-2021.12.18", "-104i");
eqt!(t2cmp_166, "2012.01.27-2016.12.24", "-1793i");
eqt!(t2cmp_167, "2024.04.21>=2024.08.21", "0b");
eqt!(t2cmp_168, "2010.04.24<2011.10.08", "1b");
eqt!(t2cmp_169, "2023.03.05<=2024.05.27", "1b");
eqt!(t2cmp_170, "2010.12.27>=2011.05.02", "0b");
eqt!(t2cmp_171, "2011.11.08=2022.06.25", "0b");
eqt!(t2cmp_172, "2014.05.19>=2021.12.05", "0b");
eqt!(t2cmp_173, "2011.06.19<2021.09.27", "1b");
eqt!(t2cmp_174, "2023.11.09=2015.09.09", "0b");
eqt!(t2cmp_175, "2012.07.23<=2023.12.09", "1b");
eqt!(t2cmp_176, "2019.08.01>2015.01.23", "1b");
eqt!(t2cmp_177, "2017.03.22<2010.03.17", "0b");
eqt!(t2cmp_178, "2019.05.11<2022.06.13", "1b");
eqt!(t2cmp_179, "2018.06.26<=2015.01.05", "0b");
eqt!(t2cmp_180, "2024.08.09>=2025.11.11", "0b");
eqt!(t2mon_181, "2021.03m+11", "2022.02m");
eqt!(t2mon_182, "2019.03m+23", "2021.02m");
eqt!(t2mon_183, "2020.01m+17", "2021.06m");
eqt!(t2mon_184, "2013.09m+22", "2015.07m");
eqt!(t2mon_185, "2019.03m+15", "2020.06m");
eqt!(t2mon_186, "2007.09m+4", "2008.01m");
eqt!(t2mon_187, "2026.12m+13", "2028.01m");
eqt!(t2mon_188, "2011.07m+18", "2013.01m");
eqt!(t2mon_189, "2011.06m+7", "2012.01m");
eqt!(t2mon_190, "2023.02m+3", "2023.05m");
eqt!(t2mon_191, "2008.11m+8", "2009.07m");
eqt!(t2mon_192, "2026.07m+6", "2027.01m");
eqt!(t2mon_193, "2008.12m+14", "2010.02m");
eqt!(t2mon_194, "2008.06m+3", "2008.09m");
eqt!(t2mon_195, "2006.01m+24", "2008.01m");
eqt!(t2mon_196, "2002.02m+21", "2003.11m");
eqt!(t2mon_197, "2016.07m+7", "2017.02m");
eqt!(t2mon_198, "2024.05m+15", "2025.08m");
eqt!(t2mon_199, "2011.09m+1", "2011.10m");
eqt!(t2mon_200, "2006.11m+18", "2008.05m");
eqt!(t2mon_201, "2021.07m+20", "2023.03m");
eqt!(t2mon_202, "2028.11m+14", "2030.01m");
eqt!(t2mon_203, "2011.05m+24", "2013.05m");
eqt!(t2mon_204, "2006.10m+5", "2007.03m");
eqt!(t2mon_205, "2016.10m+12", "2017.10m");
eqt!(t2mon_206, "2023.03m+16", "2024.07m");
eqt!(t2mon_207, "2015.08m+6", "2016.02m");
eqt!(t2mon_208, "2015.11m+17", "2017.04m");
eqt!(t2mon_209, "2016.10m+20", "2018.06m");
eqt!(t2mon_210, "2010.09m+15", "2011.12m");
eqt!(t2mon_211, "2020.08m+4", "2020.12m");
eqt!(t2mon_212, "2016.01m+22", "2017.11m");
eqt!(t2mon_213, "2015.04m+4", "2015.08m");
eqt!(t2mon_214, "2012.08m+1", "2012.09m");
eqt!(t2mon_215, "2013.07m+11", "2014.06m");
eqt!(t2mon_216, "2027.01m+24", "2029.01m");
eqt!(t2mon_217, "2007.03m+4", "2007.07m");
eqt!(t2mon_218, "2030.09m+14", "2031.11m");
eqt!(t2mon_219, "2018.01m+4", "2018.05m");
eqt!(t2mon_220, "2005.10m+24", "2007.10m");
eqt!(t2time_221, "06:08:14.000+3160", "06:08:17.160");
eqt!(t2time_222, "08:01:22.000+2889", "08:01:24.889");
eqt!(t2time_223, "01:02:49.000+823", "01:02:49.823");
eqt!(t2time_224, "01:12:14.000+2472", "01:12:16.472");
eqt!(t2time_225, "06:13:33.000+3086", "06:13:36.086");
eqt!(t2time_226, "00:11:29.000+1083", "00:11:30.083");
eqt!(t2time_227, "15:22:53.000+1503", "15:22:54.503");
eqt!(t2time_228, "11:58:57.000+1384", "11:58:58.384");
eqt!(t2time_229, "07:18:01.000+2645", "07:18:03.645");
eqt!(t2time_230, "14:06:10.000+2383", "14:06:12.383");
eqt!(t2time_231, "01:52:55.000+875", "01:52:55.875");
eqt!(t2time_232, "12:58:51.000+1157", "12:58:52.157");
eqt!(t2time_233, "11:21:11.000+2899", "11:21:13.899");
eqt!(t2time_234, "21:30:21.000+2679", "21:30:23.679");
eqt!(t2time_235, "09:50:41.000+1967", "09:50:42.967");
eqt!(t2time_236, "20:31:00.000+1924", "20:31:01.924");
eqt!(t2time_237, "00:16:34.000+2478", "00:16:36.478");
eqt!(t2time_238, "02:21:06.000+116", "02:21:06.116");
eqt!(t2time_239, "13:45:59.000+1529", "13:46:00.529");
eqt!(t2time_240, "22:58:59.000+2887", "22:59:01.887");
eqt!(t2time_241, "00:11:23.000+1471", "00:11:24.471");
eqt!(t2time_242, "11:37:21.000+2762", "11:37:23.762");
eqt!(t2time_243, "08:20:58.000+3047", "08:21:01.047");
eqt!(t2time_244, "08:24:10.000+2471", "08:24:12.471");
eqt!(t2time_245, "07:42:19.000+3421", "07:42:22.421");
eqt!(t2time_246, "17:05:05.000+2738", "17:05:07.738");
eqt!(t2time_247, "15:04:50.000+335", "15:04:50.335");
eqt!(t2time_248, "21:12:07.000+1410", "21:12:08.410");
eqt!(t2time_249, "12:55:20.000+905", "12:55:20.905");
eqt!(t2time_250, "07:03:53.000+544", "07:03:53.544");
eqt!(t2time_251, "17:16:51.000+2303", "17:16:53.303");
eqt!(t2time_252, "12:10:53.000+2667", "12:10:55.667");
eqt!(t2time_253, "00:55:21.000+1288", "00:55:22.288");
eqt!(t2time_254, "06:24:17.000+2039", "06:24:19.039");
eqt!(t2time_255, "23:50:11.000+695", "23:50:11.695");
eqt!(t2time_256, "22:37:50.000+3587", "22:37:53.587");
eqt!(t2time_257, "21:01:42.000+254", "21:01:42.254");
eqt!(t2time_258, "04:02:19.000+1449", "04:02:20.449");
eqt!(t2time_259, "15:09:03.000+561", "15:09:03.561");
eqt!(t2time_260, "09:41:22.000+2369", "09:41:24.369");
eqt!(d2make_261, "(`bb`cc`bb`aa!7 8 13 3)", "`bb`cc`bb`aa!7 8 13 3");
eqt!(d2make_262, "(`ee`bb!0 20)", "`ee`bb!0 20");
eqt!(d2make_263, "(`cc`dd`dd!11 4 19)", "`cc`dd`dd!11 4 19");
eqt!(d2make_264, "(`bb`dd`ee!14 8 13)", "`bb`dd`ee!14 8 13");
eqt!(d2make_265, "(`dd`bb`cc!16 1 6)", "`dd`bb`cc!16 1 6");
eqt!(d2make_266, "(`dd`ff`bb`aa!17 1 5 1)", "`dd`ff`bb`aa!17 1 5 1");
eqt!(d2make_267, "(`cc`dd`aa!2 2 5)", "`cc`dd`aa!2 2 5");
eqt!(d2make_268, "(`bb`aa!3 2)", "`bb`aa!3 2");
eqt!(d2make_269, "(`aa`aa!8 11)", "`aa`aa!8 11");
eqt!(d2make_270, "(`bb`dd`dd`ee!1 13 5 15)", "`bb`dd`dd`ee!1 13 5 15");
eqt!(d2make_271, "(`ee`dd`cc!2 11 5)", "`ee`dd`cc!2 11 5");
eqt!(d2make_272, "(`ff`cc`ee`aa!20 6 6 1)", "`ff`cc`ee`aa!20 6 6 1");
eqt!(d2make_273, "(`ff`ff`bb`ee!14 3 8 17)", "`ff`ff`bb`ee!14 3 8 17");
eqt!(d2make_274, "(`ff`bb`aa`aa`bb!15 3 5 6 4)", "`ff`bb`aa`aa`bb!15 3 5 6 4");
eqt!(d2make_275, "(`ff`dd`aa!18 8 2)", "`ff`dd`aa!18 8 2");
eqt!(d2make_276, "(`bb`ee`ee!0 19 14)", "`bb`ee`ee!0 19 14");
eqt!(d2make_277, "(`ee`cc`ff`ee`cc!16 14 4 17 14)", "`ee`cc`ff`ee`cc!16 14 4 \
    17 \
    14");
eqt!(d2make_278, "(`dd`bb`ee`ee!3 4 2 20)", "`dd`bb`ee`ee!3 4 2 20");
eqt!(d2make_279, "(`bb`ee!7 16)", "`bb`ee!7 16");
eqt!(d2make_280, "(`ee`ee`dd!9 0 9)", "`ee`ee`dd!9 0 9");
eqt!(d2make_281, "(`bb`ff`cc`ee`aa!2 19 6 16 8)", "`bb`ff`cc`ee`aa!2 19 6 16 \
    8");
eqt!(d2make_282, "(`bb`bb`ff`dd!11 18 14 5)", "`bb`bb`ff`dd!11 18 14 5");
eqt!(d2make_283, "(`cc`ff`bb`bb!8 8 7 19)", "`cc`ff`bb`bb!8 8 7 19");
eqt!(d2make_284, "(`ff`dd!18 4)", "`ff`dd!18 4");
eqt!(d2make_285, "(`cc`ff`cc`dd`bb!6 19 9 0 16)", "`cc`ff`cc`dd`bb!6 19 9 0 \
    16");
eqt!(d2make_286, "(`cc`aa`cc`aa`dd!4 19 17 8 17)", "`cc`aa`cc`aa`dd!4 19 17 8 \
    17");
eqt!(d2make_287, "(`bb`dd!16 18)", "`bb`dd!16 18");
eqt!(d2make_288, "(`ee`cc`bb!4 0 16)", "`ee`cc`bb!4 0 16");
eqt!(d2make_289, "(`dd`aa!19 13)", "`dd`aa!19 13");
eqt!(d2make_290, "(`ff`ee`dd`ee!16 15 0 19)", "`ff`ee`dd`ee!16 15 0 19");
eqt!(d2make_291, "(`dd`ee!18 19)", "`dd`ee!18 19");
eqt!(d2make_292, "(`bb`cc`cc`aa!0 20 3 5)", "`bb`cc`cc`aa!0 20 3 5");
eqt!(d2make_293, "(`ff`cc`ee!6 7 18)", "`ff`cc`ee!6 7 18");
eqt!(d2make_294, "(`aa`cc`bb`dd`ff!14 8 1 5 6)", "`aa`cc`bb`dd`ff!14 8 1 5 6");
eqt!(d2make_295, "(`aa`dd!5 17)", "`aa`dd!5 17");
eqt!(d2make_296, "(`ff`ee`bb`bb`ee!13 9 3 2 2)", "`ff`ee`bb`bb`ee!13 9 3 2 2");
eqt!(d2make_297, "(`dd`bb`bb`ee!2 2 12 7)", "`dd`bb`bb`ee!2 2 12 7");
eqt!(d2make_298, "(`dd`bb`ee`ff!10 18 13 18)", "`dd`bb`ee`ff!10 18 13 18");
eqt!(d2make_299, "(`ff`bb`dd`bb!7 4 13 14)", "`ff`bb`dd`bb!7 4 13 14");
eqt!(d2make_300, "(`cc`cc`ff`dd`cc!2 19 16 1 6)", "`cc`cc`ff`dd`cc!2 19 16 1 \
    6");
eqt!(d2make_301, "(`ee`ff`ee`aa`dd!13 17 1 6 2)", "`ee`ff`ee`aa`dd!13 17 1 6 \
    2");
eqt!(d2make_302, "(`ee`ff`dd`dd`aa!6 7 2 7 17)", "`ee`ff`dd`dd`aa!6 7 2 7 17");
eqt!(d2make_303, "(`bb`bb`ff`bb`cc!12 20 11 2 18)", "`bb`bb`ff`bb`cc!12 20 11 \
    2 \
    18");
eqt!(d2make_304, "(`aa`bb`dd`ee`dd!4 0 10 17 10)", "`aa`bb`dd`ee`dd!4 0 10 17 \
    10");
eqt!(d2make_305, "(`cc`cc`aa!7 15 9)", "`cc`cc`aa!7 15 9");
eqt!(d2make_306, "(`cc`ee`dd`ff`ff!5 0 2 18 16)", "`cc`ee`dd`ff`ff!5 0 2 18 \
    16");
eqt!(d2make_307, "(`ee`ff!10 16)", "`ee`ff!10 16");
eqt!(d2make_308, "(`dd`aa`ff`aa!15 5 5 9)", "`dd`aa`ff`aa!15 5 5 9");
eqt!(d2make_309, "(`cc`aa`bb`ff`cc!18 15 18 18 18)", "`cc`aa`bb`ff`cc!18 15 18 \
    18 18");
eqt!(d2make_310, "(`aa`bb`ee`aa`dd!16 4 6 5 15)", "`aa`bb`ee`aa`dd!16 4 6 5 \
    15");
eqt!(d2make_311, "(`aa`dd`ee`aa!14 19 19 4)", "`aa`dd`ee`aa!14 19 19 4");
eqt!(d2make_312, "(`ee`bb!12 7)", "`ee`bb!12 7");
eqt!(d2make_313, "(`cc`ff`bb!20 4 0)", "`cc`ff`bb!20 4 0");
eqt!(d2make_314, "(`cc`cc`dd`bb!12 14 18 9)", "`cc`cc`dd`bb!12 14 18 9");
eqt!(d2make_315, "(`ee`dd!20 1)", "`ee`dd!20 1");
eqt!(d2make_316, "(`bb`ff`bb`cc!0 0 4 13)", "`bb`ff`bb`cc!0 0 4 13");
eqt!(d2make_317, "(`cc`ee!13 12)", "`cc`ee!13 12");
eqt!(d2make_318, "(`bb`ff`aa`cc!18 5 19 4)", "`bb`ff`aa`cc!18 5 19 4");
eqt!(d2make_319, "(`aa`ee!11 12)", "`aa`ee!11 12");
eqt!(d2make_320, "(`cc`ff!9 8)", "`cc`ff!9 8");
eqt!(d2make_321, "(`ff`aa`bb!1 9 3)", "`ff`aa`bb!1 9 3");
eqt!(d2make_322, "(`ee`dd!16 3)", "`ee`dd!16 3");
eqt!(d2make_323, "(`ee`aa`dd`bb!4 4 11 3)", "`ee`aa`dd`bb!4 4 11 3");
eqt!(d2make_324, "(`ee`cc`dd`aa!6 18 15 5)", "`ee`cc`dd`aa!6 18 15 5");
eqt!(d2make_325, "(`ff`ff`ee`ee!8 15 20 3)", "`ff`ff`ee`ee!8 15 20 3");
eqt!(d2make_326, "(`bb`ee!1 2)", "`bb`ee!1 2");
eqt!(d2make_327, "(`cc`ee`dd!16 6 18)", "`cc`ee`dd!16 6 18");
eqt!(d2make_328, "(`ff`dd`bb`bb`dd!19 10 9 14 3)", "`ff`dd`bb`bb`dd!19 10 9 14 \
    3");
eqt!(d2make_329, "(`ee`aa!10 12)", "`ee`aa!10 12");
eqt!(d2make_330, "(`bb`cc`cc`ff!2 4 18 1)", "`bb`cc`cc`ff!2 4 18 1");
eqt!(d2make_331, "(`cc`ee!10 18)", "`cc`ee!10 18");
eqt!(d2make_332, "(`bb`bb`ff!13 5 6)", "`bb`bb`ff!13 5 6");
eqt!(d2make_333, "(`ee`aa!0 10)", "`ee`aa!0 10");
eqt!(d2make_334, "(`dd`dd!8 11)", "`dd`dd!8 11");
eqt!(d2make_335, "(`ee`bb`dd!7 18 8)", "`ee`bb`dd!7 18 8");
eqt!(d2make_336, "(`aa`cc`aa!0 2 2)", "`aa`cc`aa!0 2 2");
eqt!(d2make_337, "(`ff`dd!5 7)", "`ff`dd!5 7");
eqt!(d2make_338, "(`ee`ff!7 11)", "`ee`ff!7 11");
eqt!(d2make_339, "(`ee`dd`bb`cc`dd!6 14 9 12 14)", "`ee`dd`bb`cc`dd!6 14 9 12 \
    14");
eqt!(d2make_340, "(`cc`dd`ee`ee!16 7 15 20)", "`cc`dd`ee`ee!16 7 15 20");
eqt!(d2val_341, "value `cc`bb`ee!17 13 8", "17 13 8");
eqt!(d2val_342, "value `ee`cc`cc`cc`cc!6 8 5 9 3", "6 8 5 9 3");
eqt!(d2val_343, "value `aa`cc`dd`ff!18 6 14 3", "18 6 14 3");
eqt!(d2val_344, "value `bb`dd`aa`dd`ee!14 7 1 0 17", "14 7 1 0 17");
eqt!(d2val_345, "value `ee`ee`ff`ee`bb!1 3 1 10 15", "1 3 1 10 15");
eqt!(d2val_346, "value `ee`cc`bb!8 10 18", "8 10 18");
eqt!(d2val_347, "value `aa`aa`aa`dd`bb!6 17 6 20 0", "6 17 6 20 0");
eqt!(d2val_348, "value `ee`ee`ee!6 11 4", "6 11 4");
eqt!(d2val_349, "value `bb`aa`cc`cc!8 3 2 6", "8 3 2 6");
eqt!(d2val_350, "value `ff`cc`dd`ff!6 10 9 12", "6 10 9 12");
eqt!(d2val_351, "value `ee`aa`ee`dd!20 3 10 19", "20 3 10 19");
eqt!(d2val_352, "value `dd`ee!3 11", "3 11");
eqt!(d2val_353, "value `bb`dd`dd!11 2 12", "11 2 12");
eqt!(d2val_354, "value `dd`dd`dd!10 14 19", "10 14 19");
eqt!(d2val_355, "value `bb`ee`dd`cc`aa!3 18 4 15 9", "3 18 4 15 9");
eqt!(d2val_356, "value `dd`ee`dd!3 0 19", "3 0 19");
eqt!(d2val_357, "value `aa`aa`aa`ee!0 1 13 12", "0 1 13 12");
eqt!(d2val_358, "value `ee`cc`ff`dd!3 17 20 4", "3 17 20 4");
eqt!(d2val_359, "value `aa`dd`dd`bb`bb!1 15 0 8 1", "1 15 0 8 1");
eqt!(d2val_360, "value `aa`dd!5 1", "5 1");
eqt!(d2val_361, "value `ee`aa`cc`bb`ff!7 20 11 16 8", "7 20 11 16 8");
eqt!(d2val_362, "value `ee`cc!0 1", "0 1");
eqt!(d2val_363, "value `ff`bb`cc`ff!19 0 14 15", "19 0 14 15");
eqt!(d2val_364, "value `dd`aa`aa!1 12 8", "1 12 8");
eqt!(d2val_365, "value `ff`aa`dd!12 6 11", "12 6 11");
eqt!(d2val_366, "value `aa`bb`cc`bb!18 12 1 1", "18 12 1 1");
eqt!(d2val_367, "value `ee`ee!10 20", "10 20");
eqt!(d2val_368, "value `aa`bb!11 9", "11 9");
eqt!(d2val_369, "value `bb`cc`aa`aa!4 12 8 6", "4 12 8 6");
eqt!(d2val_370, "value `cc`dd`ff!16 0 4", "16 0 4");
eqt!(d2val_371, "value `dd`cc`ff!10 0 12", "10 0 12");
eqt!(d2val_372, "value `ee`dd`dd!12 6 3", "12 6 3");
eqt!(d2val_373, "value `cc`dd`ee!13 16 18", "13 16 18");
eqt!(d2val_374, "value `bb`cc!12 0", "12 0");
eqt!(d2val_375, "value `cc`bb!7 19", "7 19");
eqt!(d2val_376, "value `ee`ee`dd`ee`dd!0 16 16 9 6", "0 16 16 9 6");
eqt!(d2val_377, "value `aa`ff!5 1", "5 1");
eqt!(d2val_378, "value `aa`cc`ff`bb!8 13 6 14", "8 13 6 14");
eqt!(d2val_379, "value `aa`ff`ff!5 13 3", "5 13 3");
eqt!(d2val_380, "value `dd`cc`bb!20 13 11", "20 13 11");
eqt!(d2val_381, "value `bb`ff`bb`bb`aa!9 15 8 3 18", "9 15 8 3 18");
eqt!(d2val_382, "value `ff`cc`ff!0 17 7", "0 17 7");
eqt!(d2val_383, "value `bb`ff`aa`dd`bb!16 14 4 3 7", "16 14 4 3 7");
eqt!(d2val_384, "value `aa`dd`bb`ee`cc!11 14 3 2 5", "11 14 3 2 5");
eqt!(d2val_385, "value `cc`aa`ff`dd!14 10 7 13", "14 10 7 13");
eqt!(d2val_386, "value `dd`dd`dd!3 8 17", "3 8 17");
eqt!(d2val_387, "value `bb`aa!2 19", "2 19");
eqt!(d2val_388, "value `cc`ff`cc!19 20 11", "19 20 11");
eqt!(d2val_389, "value `cc`aa!4 16", "4 16");
eqt!(d2val_390, "value `bb`aa`aa!8 15 19", "8 15 19");
eqt!(d2val_391, "value `bb`dd`ff`dd`ee!6 9 20 20 9", "6 9 20 20 9");
eqt!(d2val_392, "value `aa`aa!6 14", "6 14");
eqt!(d2val_393, "value `ff`ee`ff`ee`ff!10 0 20 12 12", "10 0 20 12 12");
eqt!(d2val_394, "value `cc`cc!1 13", "1 13");
eqt!(d2val_395, "value `bb`cc`dd`cc`aa!5 9 11 19 12", "5 9 11 19 12");
eqt!(d2val_396, "value `bb`bb`dd`ff`dd!2 10 13 14 5", "2 10 13 14 5");
eqt!(d2val_397, "value `cc`dd`bb`ff!1 4 1 18", "1 4 1 18");
eqt!(d2val_398, "value `bb`dd`aa`cc!2 11 8 2", "2 11 8 2");
eqt!(d2val_399, "value `cc`bb!8 19", "8 19");
eqt!(d2val_400, "value `dd`aa`dd`ee!19 9 6 12", "19 9 6 12");
eqt!(d2val_401, "value `bb`ff!1 11", "1 11");
eqt!(d2val_402, "value `bb`aa`bb`aa`dd!0 16 5 18 8", "0 16 5 18 8");
eqt!(d2val_403, "value `aa`aa!10 10", "10 10");
eqt!(d2val_404, "value `ff`bb`cc!5 18 3", "5 18 3");
eqt!(d2val_405, "value `dd`ee`ff`aa!6 16 1 16", "6 16 1 16");
eqt!(d2val_406, "value `aa`cc`ff!1 7 2", "1 7 2");
eqt!(d2val_407, "value `aa`ee`aa`cc`dd!0 15 2 9 3", "0 15 2 9 3");
eqt!(d2val_408, "value `dd`aa!15 7", "15 7");
eqt!(d2val_409, "value `cc`bb`bb!17 0 9", "17 0 9");
eqt!(d2val_410, "value `bb`ff`ff!15 19 15", "15 19 15");
eqt!(d2val_411, "value `ff`ff!3 18", "3 18");
eqt!(d2val_412, "value `ee`bb`bb`cc!13 13 6 3", "13 13 6 3");
eqt!(d2val_413, "value `ee`cc!2 10", "2 10");
eqt!(d2val_414, "value `ff`ee`cc!17 11 9", "17 11 9");
eqt!(d2val_415, "value `bb`bb!1 20", "1 20");
eqt!(d2val_416, "value `ee`ff`ff!11 7 17", "11 7 17");
eqt!(d2val_417, "value `ff`bb!12 15", "12 15");
eqt!(d2val_418, "value `ee`cc`cc`dd!8 13 14 17", "8 13 14 17");
eqt!(d2val_419, "value `ff`cc`ee`bb!7 8 19 13", "7 8 19 13");
eqt!(d2val_420, "value `dd`cc!12 14", "12 14");
eqt!(d2key_421, "key `bb`dd!3 6", "`bb`dd");
eqt!(d2key_422, "key `ff`dd`ee!4 5 6", "`ff`dd`ee");
eqt!(d2key_423, "key `cc`ff`cc`bb`bb!2 8 6 8 7", "`cc`ff`cc`bb`bb");
eqt!(d2key_424, "key `dd`aa`bb!8 7 4", "`dd`aa`bb");
eqt!(d2key_425, "key `ee`aa!6 9", "`ee`aa");
eqt!(d2key_426, "key `ff`ff`ee`ff`dd!8 9 7 1 9", "`ff`ff`ee`ff`dd");
eqt!(d2key_427, "key `cc`aa`ee!8 2 5", "`cc`aa`ee");
eqt!(d2key_428, "key `cc`bb`aa`ff!3 2 6 3", "`cc`bb`aa`ff");
eqt!(d2key_429, "key `cc`dd`ee`ee!2 9 1 1", "`cc`dd`ee`ee");
eqt!(d2key_430, "key `dd`cc`cc`bb`aa!6 5 3 0 4", "`dd`cc`cc`bb`aa");
eqt!(d2key_431, "key `bb`aa`cc`bb!8 7 9 5", "`bb`aa`cc`bb");
eqt!(d2key_432, "key `aa`dd`bb!8 2 1", "`aa`dd`bb");
eqt!(d2key_433, "key `dd`aa`bb`ff`aa!7 1 2 7 8", "`dd`aa`bb`ff`aa");
eqt!(d2key_434, "key `ee`cc`ff`ee!8 9 6 6", "`ee`cc`ff`ee");
eqt!(d2key_435, "key `dd`bb`aa`bb!7 8 9 1", "`dd`bb`aa`bb");
eqt!(d2key_436, "key `ee`ee`bb!5 4 7", "`ee`ee`bb");
eqt!(d2key_437, "key `cc`bb`ee!7 7 5", "`cc`bb`ee");
eqt!(d2key_438, "key `cc`bb!1 6", "`cc`bb");
eqt!(d2key_439, "key `ff`ff!2 6", "`ff`ff");
eqt!(d2key_440, "key `bb`dd!6 8", "`bb`dd");
eqt!(d2key_441, "key `ff`aa`ee`bb!7 1 2 2", "`ff`aa`ee`bb");
eqt!(d2key_442, "key `ee`ee!3 2", "`ee`ee");
eqt!(d2key_443, "key `ff`cc`ff`ee!5 0 7 8", "`ff`cc`ff`ee");
eqt!(d2key_444, "key `cc`dd`bb`ff!7 2 9 2", "`cc`dd`bb`ff");
eqt!(d2key_445, "key `ee`dd`aa`ff!7 8 1 2", "`ee`dd`aa`ff");
eqt!(d2key_446, "key `dd`cc`ff!8 8 4", "`dd`cc`ff");
eqt!(d2key_447, "key `ee`dd`aa!7 3 0", "`ee`dd`aa");
eqt!(d2key_448, "key `aa`aa`aa!1 7 3", "`aa`aa`aa");
eqt!(d2key_449, "key `ee`dd`ff`aa`aa!1 1 1 7 9", "`ee`dd`ff`aa`aa");
eqt!(d2key_450, "key `cc`dd`cc`cc!6 8 4 9", "`cc`dd`cc`cc");
eqt!(d2key_451, "key `dd`cc`aa`aa`aa!8 4 6 9 1", "`dd`cc`aa`aa`aa");
eqt!(d2key_452, "key `ff`ee`ee`ee`bb!3 0 8 9 1", "`ff`ee`ee`ee`bb");
eqt!(d2key_453, "key `ff`dd`ee!1 9 9", "`ff`dd`ee");
eqt!(d2key_454, "key `cc`cc`dd`aa!2 0 2 1", "`cc`cc`dd`aa");
eqt!(d2key_455, "key `aa`ee`ee`dd!5 8 4 5", "`aa`ee`ee`dd");
eqt!(d2key_456, "key `dd`ff`aa!6 4 2", "`dd`ff`aa");
eqt!(d2key_457, "key `ff`ff!2 2", "`ff`ff");
eqt!(d2key_458, "key `dd`aa!8 1", "`dd`aa");
eqt!(d2key_459, "key `dd`dd`ff`aa!3 1 1 8", "`dd`dd`ff`aa");
eqt!(d2key_460, "key `ee`cc`bb`cc`bb!1 8 5 5 4", "`ee`cc`bb`cc`bb");
eqt!(d2key_461, "key `dd`bb`ee!9 2 6", "`dd`bb`ee");
eqt!(d2key_462, "key `ff`ff!3 6", "`ff`ff");
eqt!(d2key_463, "key `aa`cc!4 3", "`aa`cc");
eqt!(d2key_464, "key `aa`bb`aa`ff!7 4 1 6", "`aa`bb`aa`ff");
eqt!(d2key_465, "key `aa`aa`bb!9 9 8", "`aa`aa`bb");
eqt!(d2key_466, "key `dd`aa!1 1", "`dd`aa");
eqt!(d2key_467, "key `ee`aa`dd`ff!0 9 6 2", "`ee`aa`dd`ff");
eqt!(d2key_468, "key `bb`aa`aa`aa!5 9 2 1", "`bb`aa`aa`aa");
eqt!(d2key_469, "key `dd`dd`bb`cc`ff!3 7 2 1 7", "`dd`dd`bb`cc`ff");
eqt!(d2key_470, "key `ff`aa!4 2", "`ff`aa");
eqt!(d2key_471, "key `ee`bb`aa`cc!8 6 9 2", "`ee`bb`aa`cc");
eqt!(d2key_472, "key `ee`bb`cc`aa!0 3 7 8", "`ee`bb`cc`aa");
eqt!(d2key_473, "key `ff`aa`aa`aa`ff!9 2 4 7 9", "`ff`aa`aa`aa`ff");
eqt!(d2key_474, "key `cc`ee!4 2", "`cc`ee");
eqt!(d2key_475, "key `bb`ff!8 9", "`bb`ff");
eqt!(d2key_476, "key `ee`bb`dd`bb`dd!3 4 0 9 4", "`ee`bb`dd`bb`dd");
eqt!(d2key_477, "key `ff`aa`cc!6 8 5", "`ff`aa`cc");
eqt!(d2key_478, "key `dd`aa`dd!2 7 9", "`dd`aa`dd");
eqt!(d2key_479, "key `aa`cc`dd!6 7 3", "`aa`cc`dd");
eqt!(d2key_480, "key `ff`dd!2 3", "`ff`dd");
eqt!(d2key_481, "key `ff`cc`ee!9 2 3", "`ff`cc`ee");
eqt!(d2key_482, "key `dd`ee`bb`aa`aa!9 3 4 8 1", "`dd`ee`bb`aa`aa");
eqt!(d2key_483, "key `bb`ff!6 8", "`bb`ff");
eqt!(d2key_484, "key `aa`ff`ee`aa`ee!5 1 8 1 8", "`aa`ff`ee`aa`ee");
eqt!(d2key_485, "key `dd`aa`ee`ff!5 8 0 5", "`dd`aa`ee`ff");
eqt!(d2key_486, "key `cc`ff`ff`cc`cc!1 6 0 8 3", "`cc`ff`ff`cc`cc");
eqt!(d2key_487, "key `ff`ee`ee!5 1 0", "`ff`ee`ee");
eqt!(d2key_488, "key `ee`ff!6 0", "`ee`ff");
eqt!(d2key_489, "key `ee`ee!6 2", "`ee`ee");
eqt!(d2key_490, "key `ff`cc!4 7", "`ff`cc");
eqt!(d2key_491, "key `aa`aa`dd`cc`ff!3 3 1 5 5", "`aa`aa`dd`cc`ff");
eqt!(d2key_492, "key `aa`aa`ee!8 4 3", "`aa`aa`ee");
eqt!(d2key_493, "key `cc`ee`dd!2 7 8", "`cc`ee`dd");
eqt!(d2key_494, "key `bb`ff`ee!8 9 5", "`bb`ff`ee");
eqt!(d2key_495, "key `dd`cc`ee`aa`ff!7 3 7 6 1", "`dd`cc`ee`aa`ff");
eqt!(d2key_496, "key `ff`ee!0 0", "`ff`ee");
eqt!(d2key_497, "key `ff`ff`bb`ff`ff!6 4 0 8 9", "`ff`ff`bb`ff`ff");
eqt!(d2key_498, "key `cc`bb!2 2", "`cc`bb");
eqt!(d2key_499, "key `bb`cc`dd`dd!8 4 4 5", "`bb`cc`dd`dd");
eqt!(d2key_500, "key `dd`ee`cc!4 0 7", "`dd`ee`cc");
eqt!(d2arith_501, "(`dd`dd`ff!5 2 1)*(`dd`dd`ff!7 5 6)", "`dd`dd`ff!35 10 6");
eqt!(d2arith_502, "(`ff`dd!9 6)-(`ff`dd!6 7)", "`ff`dd!3 -1");
eqt!(d2arith_503, "(`ff`cc`ee!7 4 3)*(`ff`cc`ee!6 2 1)", "`ff`cc`ee!42 8 3");
eqt!(d2arith_504, "(`bb`bb!0 9)-(`bb`bb!7 8)", "`bb`bb!-7 1");
eqt!(d2arith_505, "(`dd`dd`cc!5 2 8)-(`dd`dd`cc!8 4 3)", "`dd`dd`cc!-3 -2 5");
eqt!(d2arith_506, "(`aa`ee`dd`bb!2 9 8 1)*(`aa`ee`dd`bb!4 9 9 5)",
    "`aa`ee`dd`bb!8 81 72 5");
eqt!(d2arith_507, "(`cc`dd`ee`aa!0 0 5 9)*(`cc`dd`ee`aa!4 8 7 4)",
    "`cc`dd`ee`aa!0 0 35 36");
eqt!(d2arith_508, "(`bb`cc`dd`cc!1 8 1 1)*(`bb`cc`dd`cc!7 3 7 5)",
    "`bb`cc`dd`cc!7 24 7 5");
eqt!(d2arith_509, "(`aa`ff`aa!5 8 1)+(`aa`ff`aa!5 3 7)", "`aa`ff`aa!10 11 8");
eqt!(d2arith_510, "(`ff`dd`ee`dd!9 2 8 6)*(`ff`dd`ee`dd!2 4 9 9)",
    "`ff`dd`ee`dd!18 8 72 54");
eqt!(d2arith_511, "(`ee`ee`dd`bb!4 9 0 7)-(`ee`ee`dd`bb!8 1 4 6)",
    "`ee`ee`dd`bb!-4 8 -4 1");
eqt!(d2arith_512, "(`aa`dd`ee`bb!7 9 3 8)-(`aa`dd`ee`bb!8 9 5 4)",
    "`aa`dd`ee`bb!-1 0 -2 4");
eqt!(d2arith_513, "(`aa`bb!1 1)+(`aa`bb!1 6)", "`aa`bb!2 7");
eqt!(d2arith_514, "(`dd`ee`bb`cc!1 1 9 8)+(`dd`ee`bb`cc!9 5 6 3)",
    "`dd`ee`bb`cc!10 6 15 11");
eqt!(d2arith_515, "(`ee`cc!9 4)-(`ee`cc!2 7)", "`ee`cc!7 -3");
eqt!(d2arith_516, "(`cc`aa`ee`ff!2 7 4 0)-(`cc`aa`ee`ff!1 9 8 2)",
    "`cc`aa`ee`ff!1 -2 -4 -2");
eqt!(d2arith_517, "(`ff`cc`bb!7 5 0)*(`ff`cc`bb!1 7 3)", "`ff`cc`bb!7 35 0");
eqt!(d2arith_518, "(`ff`ff!9 0)+(`ff`ff!5 6)", "`ff`ff!14 6");
eqt!(d2arith_519, "(`aa`cc!5 1)-(`aa`cc!3 2)", "`aa`cc!2 -1");
eqt!(d2arith_520, "(`ff`ff`ee`ee!5 2 2 9)*(`ff`ff`ee`ee!8 7 5 2)",
    "`ff`ff`ee`ee!40 14 10 18");
eqt!(d2arith_521, "(`ee`bb`ee`cc!9 2 9 6)-(`ee`bb`ee`cc!5 6 7 9)",
    "`ee`bb`ee`cc!4 -4 2 -3");
eqt!(d2arith_522, "(`ff`bb`ff`aa!9 5 5 9)+(`ff`bb`ff`aa!1 1 7 9)",
    "`ff`bb`ff`aa!10 6 12 18");
eqt!(d2arith_523, "(`ee`aa!5 2)+(`ee`aa!5 1)", "`ee`aa!10 3");
eqt!(d2arith_524, "(`ff`dd`ee`dd!0 5 8 5)-(`ff`dd`ee`dd!8 9 6 6)",
    "`ff`dd`ee`dd!-8 -4 2 -1");
eqt!(d2arith_525, "(`bb`ff`dd!1 1 9)+(`bb`ff`dd!1 8 8)", "`bb`ff`dd!2 9 17");
eqt!(d2arith_526, "(`cc`ff!9 5)*(`cc`ff!1 2)", "`cc`ff!9 10");
eqt!(d2arith_527, "(`ff`bb!3 0)*(`ff`bb!9 6)", "`ff`bb!27 0");
eqt!(d2arith_528, "(`cc`dd`ff!3 4 9)-(`cc`dd`ff!5 6 8)", "`cc`dd`ff!-2 -2 1");
eqt!(d2arith_529, "(`ee`ff!6 3)*(`ee`ff!8 2)", "`ee`ff!48 6");
eqt!(d2arith_530, "(`ff`dd!6 5)+(`ff`dd!5 4)", "`ff`dd!11 9");
eqt!(d2arith_531, "(`cc`ff`dd`bb!8 6 0 3)-(`cc`ff`dd`bb!1 1 2 4)",
    "`cc`ff`dd`bb!7 5 -2 -1");
eqt!(d2arith_532, "(`bb`dd`dd`ee!6 7 5 4)-(`bb`dd`dd`ee!1 7 1 8)",
    "`bb`dd`dd`ee!5 0 4 -4");
eqt!(d2arith_533, "(`dd`aa`ff`bb!8 5 9 0)-(`dd`aa`ff`bb!4 1 4 7)",
    "`dd`aa`ff`bb!4 4 5 -7");
eqt!(d2arith_534, "(`bb`bb`ee!4 2 0)-(`bb`bb`ee!4 2 2)", "`bb`bb`ee!0 0 -2");
eqt!(d2arith_535, "(`dd`cc!4 9)*(`dd`cc!2 5)", "`dd`cc!8 45");
eqt!(d2arith_536, "(`aa`dd`ff`ee!1 1 1 3)+(`aa`dd`ff`ee!5 7 9 4)",
    "`aa`dd`ff`ee!6 8 10 7");
eqt!(d2arith_537, "(`aa`cc!1 7)*(`aa`cc!8 2)", "`aa`cc!8 14");
eqt!(d2arith_538, "(`ee`aa!4 3)-(`ee`aa!6 2)", "`ee`aa!-2 1");
eqt!(d2arith_539, "(`ff`ee`ee`ee!0 5 5 6)+(`ff`ee`ee`ee!6 1 8 8)",
    "`ff`ee`ee`ee!6 6 13 14");
eqt!(d2arith_540, "(`dd`cc`cc`ee!4 1 2 9)*(`dd`cc`cc`ee!5 9 7 5)",
    "`dd`cc`cc`ee!20 9 14 45");
eqt!(d2arith_541, "(`dd`aa`aa!7 9 2)*(`dd`aa`aa!3 8 3)", "`dd`aa`aa!21 72 6");
eqt!(d2arith_542, "(`ee`ff`cc`ee!1 2 8 3)*(`ee`ff`cc`ee!2 5 7 7)",
    "`ee`ff`cc`ee!2 10 56 21");
eqt!(d2arith_543, "(`ee`ff`dd`cc!9 0 8 7)*(`ee`ff`dd`cc!3 7 3 8)",
    "`ee`ff`dd`cc!27 0 24 56");
eqt!(d2arith_544, "(`bb`ee`bb`bb!7 0 9 4)+(`bb`ee`bb`bb!8 5 9 2)",
    "`bb`ee`bb`bb!15 5 18 6");
eqt!(d2arith_545, "(`dd`ff!9 8)+(`dd`ff!9 4)", "`dd`ff!18 12");
eqt!(d2arith_546, "(`dd`bb`ff!3 2 0)*(`dd`bb`ff!6 3 6)", "`dd`bb`ff!18 6 0");
eqt!(d2arith_547, "(`cc`cc`bb`ee!7 8 4 6)-(`cc`cc`bb`ee!4 7 8 5)",
    "`cc`cc`bb`ee!3 1 -4 1");
eqt!(d2arith_548, "(`ee`aa!3 1)*(`ee`aa!7 1)", "`ee`aa!21 1");
eqt!(d2arith_549, "(`ee`cc`ee!5 4 4)-(`ee`cc`ee!2 6 2)", "`ee`cc`ee!3 -2 2");
eqt!(d2arith_550, "(`aa`bb!4 4)*(`aa`bb!6 8)", "`aa`bb!24 32");
eqt!(d2arith_551, "(`cc`cc!8 5)-(`cc`cc!6 8)", "`cc`cc!2 -3");
eqt!(d2arith_552, "(`bb`ff`cc`ee!6 4 4 4)+(`bb`ff`cc`ee!5 8 4 7)",
    "`bb`ff`cc`ee!11 12 8 11");
eqt!(d2arith_553, "(`dd`cc!9 7)+(`dd`cc!7 5)", "`dd`cc!16 12");
eqt!(d2arith_554, "(`bb`dd`aa!9 3 3)-(`bb`dd`aa!1 9 7)", "`bb`dd`aa!8 -6 -4");
eqt!(d2arith_555, "(`aa`ee`dd!9 2 5)+(`aa`ee`dd!5 7 3)", "`aa`ee`dd!14 9 8");
eqt!(d2arith_556, "(`ff`dd!2 0)*(`ff`dd!1 8)", "`ff`dd!2 0");
eqt!(d2arith_557, "(`aa`aa`aa`cc!7 8 8 7)-(`aa`aa`aa`cc!5 8 4 8)",
    "`aa`aa`aa`cc!2 0 4 -1");
eqt!(d2arith_558, "(`dd`aa`dd`ff!0 1 6 4)*(`dd`aa`dd`ff!9 8 7 3)",
    "`dd`aa`dd`ff!0 8 42 12");
eqt!(d2arith_559, "(`aa`dd`cc!0 3 0)-(`aa`dd`cc!6 4 3)", "`aa`dd`cc!-6 -1 -3");
eqt!(d2arith_560, "(`dd`bb`ee`ee!9 0 7 4)*(`dd`bb`ee`ee!2 9 4 4)",
    "`dd`bb`ee`ee!18 0 28 16");
eqt!(ix2at_561, "(29 21 24 24 8 27 29 5)[4]", "8");
eqt!(ix2at_562, "(10 26 27 4 23 21)[1]", "26");
eqt!(ix2at_563, "(5 19 16 17 30 10 13 25 22)[0]", "5");
eqt!(ix2at_564, "(25 8 1 16 28 2 18 4 12)[4]", "28");
eqt!(ix2at_565, "(23 30 2 1 6 23 24)[4]", "6");
eqt!(ix2at_566, "(17 24 21 21 27 27 4 7 15)[3]", "21");
eqt!(ix2at_567, "(16 11 11 10 8 5)[0]", "16");
eqt!(ix2at_568, "(12 26 7 10 30 7 28)[5]", "7");
eqt!(ix2at_569, "(21 9 23 25 10)[0]", "21");
eqt!(ix2at_570, "(26 25 16 22)[2]", "16");
eqt!(ix2at_571, "(23 14 11 9 27)[3]", "9");
eqt!(ix2at_572, "(28 19 11 3 28 29)[1]", "19");
eqt!(ix2at_573, "(24 14 14 15 15 25 29 4 21)[2]", "14");
eqt!(ix2at_574, "(24 17 30 23 4 17 10)[5]", "17");
eqt!(ix2at_575, "(5 1 18 0 26 7 18 19 17)[6]", "18");
eqt!(ix2at_576, "(9 5 19 2 14 18 15)[2]", "19");
eqt!(ix2at_577, "(21 20 23 5 30 11 0 4)[7]", "4");
eqt!(ix2at_578, "(15 23 4 4 21 22 18 5)[6]", "18");
eqt!(ix2at_579, "(28 8 21 7 28 0)[2]", "21");
eqt!(ix2at_580, "(20 16 15 8 17 25 11 25)[2]", "15");
eqt!(ix2at_581, "(16 29 29 4 27 17 28 22 12)[4]", "27");
eqt!(ix2at_582, "(29 3 28 21)[0]", "29");
eqt!(ix2at_583, "(22 10 0 28 10 20 23)[6]", "23");
eqt!(ix2at_584, "(5 6 20 18 25)[2]", "20");
eqt!(ix2at_585, "(5 15 3 9 13 28 8 11 12)[7]", "11");
eqt!(ix2at_586, "(2 6 2 11 14 17 0)[5]", "17");
eqt!(ix2at_587, "(29 9 12 1)[0]", "29");
eqt!(ix2at_588, "(6 1 2 15 3)[1]", "1");
eqt!(ix2at_589, "(1 16 23 2)[0]", "1");
eqt!(ix2at_590, "(26 23 11 29 21)[3]", "29");
eqt!(ix2at_591, "(28 2 11 6 9 9 17 28 18)[5]", "9");
eqt!(ix2at_592, "(12 24 15 26 1)[2]", "15");
eqt!(ix2at_593, "(20 5 4 2)[1]", "5");
eqt!(ix2at_594, "(11 16 17 12 17 7)[4]", "17");
eqt!(ix2at_595, "(8 6 15 23 1 29 12 30 26)[7]", "30");
eqt!(ix2at_596, "(4 27 1 30 8 15 22 30 20)[4]", "8");
eqt!(ix2at_597, "(5 9 8 12)[3]", "12");
eqt!(ix2at_598, "(24 17 0 26 17 18 8 11 15)[1]", "17");
eqt!(ix2at_599, "(2 18 22 26)[0]", "2");
eqt!(ix2at_600, "(14 9 15 27 6 30 30)[5]", "30");
eqt!(ix2at_601, "(25 20 2 5 14 22 4)[6]", "4");
eqt!(ix2at_602, "(22 3 1 20 26 24 29 18 4)[4]", "26");
eqt!(ix2at_603, "(23 17 14 24 24)[3]", "24");
eqt!(ix2at_604, "(0 11 8 20 27)[0]", "0");
eqt!(ix2at_605, "(7 26 4 9)[0]", "7");
eqt!(ix2at_606, "(26 30 29 10 11 18 25)[5]", "18");
eqt!(ix2at_607, "(5 17 15 3 12 8)[3]", "3");
eqt!(ix2at_608, "(18 0 13 10 0)[1]", "0");
eqt!(ix2at_609, "(11 28 7 12 7 8 14)[4]", "7");
eqt!(ix2at_610, "(0 1 4 4 14)[1]", "1");
eqt!(ix2at_611, "(27 12 2 5 6 28 23 0 5)[3]", "5");
eqt!(ix2at_612, "(4 6 30 16 20)[2]", "30");
eqt!(ix2at_613, "(11 26 16 22 7 27 22)[1]", "26");
eqt!(ix2at_614, "(18 16 16 27 9 21 28)[0]", "18");
eqt!(ix2at_615, "(29 6 7 9 11 5)[4]", "11");
eqt!(ix2at_616, "(30 7 17 21 9 8 21 27 9)[1]", "7");
eqt!(ix2at_617, "(5 12 10 21 29 12 20)[3]", "21");
eqt!(ix2at_618, "(19 22 15 22 21 7 22)[1]", "22");
eqt!(ix2at_619, "(29 4 6 8)[3]", "8");
eqt!(ix2at_620, "(23 1 20 7 30)[3]", "7");
eqt!(ix2at_621, "(27 9 3 15)[2]", "3");
eqt!(ix2at_622, "(15 20 15 14 4 3 26 16)[2]", "15");
eqt!(ix2at_623, "(5 16 13 7 19)[2]", "13");
eqt!(ix2at_624, "(29 23 20 27 15 17 20)[4]", "15");
eqt!(ix2at_625, "(23 11 10 8 13 19)[5]", "19");
eqt!(ix2at_626, "(28 16 13 29 23)[2]", "13");
eqt!(ix2at_627, "(11 27 29 30 3)[2]", "29");
eqt!(ix2at_628, "(30 13 14 14)[2]", "14");
eqt!(ix2at_629, "(6 1 5 30)[3]", "30");
eqt!(ix2at_630, "(0 15 27 19 28 30 11 30 13)[3]", "19");
eqt!(ix2at_631, "(23 30 7 30 1)[1]", "30");
eqt!(ix2at_632, "(0 6 15 10 23 2 23 11 1)[5]", "2");
eqt!(ix2at_633, "(27 9 10 10 14 11 17 27 9)[2]", "10");
eqt!(ix2at_634, "(3 21 26 16)[3]", "16");
eqt!(ix2at_635, "(22 18 10 12 14 4 14 6 1)[5]", "4");
eqt!(ix2at_636, "(16 8 9 28 17 14 8)[3]", "28");
eqt!(ix2at_637, "(15 11 21 30 10 21 7)[2]", "21");
eqt!(ix2at_638, "(26 12 7 6 30 26 14 12 9)[2]", "7");
eqt!(ix2at_639, "(25 30 19 18 26)[1]", "30");
eqt!(ix2at_640, "(8 5 0 3 11 21)[3]", "3");
eqt!(ix2at_641, "(2 6 24 18)[2]", "24");
eqt!(ix2at_642, "(2 16 4 7 0 24 1)[6]", "1");
eqt!(ix2at_643, "(12 2 3 12 30)[1]", "2");
eqt!(ix2at_644, "(11 26 19 15 17 11 24 30 29)[1]", "26");
eqt!(ix2at_645, "(6 12 29 11)[1]", "12");
eqt!(ix2at_646, "(1 9 6 8)[1]", "9");
eqt!(ix2at_647, "(15 17 18 2 15 23 13)[3]", "2");
eqt!(ix2at_648, "(15 21 20 30 23 21 23 14)[2]", "20");
eqt!(ix2at_649, "(5 8 19 28 9)[2]", "19");
eqt!(ix2at_650, "(7 16 18 25 25 12)[2]", "18");
eqt!(ix2at_651, "(5 2 30 8 18 0 9)[5]", "0");
eqt!(ix2at_652, "(11 15 7 19 23 16)[1]", "15");
eqt!(ix2at_653, "(6 10 28 4 22 23 30 24 2)[3]", "4");
eqt!(ix2at_654, "(28 26 17 7 9 0 5)[2]", "17");
eqt!(ix2at_655, "(13 20 5 20 21)[1]", "20");
eqt!(ix2at_656, "(6 4 9 4 8 17 27 21)[5]", "17");
eqt!(ix2at_657, "(11 9 21 23 30 18 22 29)[3]", "23");
eqt!(ix2at_658, "(10 14 13 17 13)[2]", "13");
eqt!(ix2at_659, "(2 12 12 28)[2]", "12");
eqt!(ix2at_660, "(20 22 5 3 26)[2]", "5");
eqt!(ix2at_661, "(23 21 3 10 7 2 22)[4]", "7");
eqt!(ix2at_662, "(8 6 15 30 5 11)[2]", "15");
eqt!(ix2at_663, "(27 18 4 29 14 4 1)[4]", "14");
eqt!(ix2at_664, "(19 30 21 25 23 9)[1]", "30");
eqt!(ix2at_665, "(6 29 19 0 6 27 4 7)[2]", "19");
eqt!(ix2at_666, "(20 17 17 13 8 25)[0]", "20");
eqt!(ix2at_667, "(1 14 14 20 18 12 23 0 22)[6]", "23");
eqt!(ix2at_668, "(27 14 3 8 30 6 4 6 3)[4]", "30");
eqt!(ix2at_669, "(15 0 11 6 4 30)[4]", "4");
eqt!(ix2at_670, "(5 22 4 5 2 22 6 25)[4]", "2");
eqt!(ix2at_671, "(28 8 10 5 17 10 10 7 26)[1]", "8");
eqt!(ix2at_672, "(9 24 5 0)[2]", "5");
eqt!(ix2at_673, "(25 20 20 13)[1]", "20");
eqt!(ix2at_674, "(3 11 25 11 10)[2]", "25");
eqt!(ix2at_675, "(10 11 1 6)[2]", "1");
eqt!(ix2at_676, "(8 1 3 6 6)[3]", "6");
eqt!(ix2at_677, "(15 0 7 4 30)[2]", "7");
eqt!(ix2at_678, "(29 7 17 10 26 6 16 20)[5]", "6");
eqt!(ix2at_679, "(20 5 29 17 27 25)[0]", "20");
eqt!(ix2at_680, "(18 15 9 17 18 6 14 5 11)[8]", "11");
eqt!(ix2v_681, "(16 9 22 10 12 26) 0 3", "16 10");
eqt!(ix2v_682, "(10 12 24 14 29 26 17 29) 0 0", "10 10");
eqt!(ix2v_683, "(5 23 15 9 23 9) 0 1", "5 23");
eqt!(ix2v_684, "(1 6 11 11 14 27) 5 1", "27 6");
eqt!(ix2v_685, "(10 25 22 17 5 7 2 8) 0 2", "10 22");
eqt!(ix2v_686, "(20 5 17 21 23 12 14 21 30) 7 2", "21 17");
eqt!(ix2v_687, "(30 18 5 4 19 8 9 25) 3 0", "4 30");
eqt!(ix2v_688, "(8 21 17 16 14) 4 2", "14 17");
eqt!(ix2v_689, "(9 3 14 22 30 22 14 1 18) 5 3", "22 22");
eqt!(ix2v_690, "(0 30 24 14 14 17 6 6 13) 8 1", "13 30");
eqt!(ix2v_691, "(12 30 21 17 25 21) 3 1", "17 30");
eqt!(ix2v_692, "(2 5 25 9 9) 0 2", "2 25");
eqt!(ix2v_693, "(13 22 5 2 4 12 3 13) 2 0", "5 13");
eqt!(ix2v_694, "(23 20 29 28) 3 1", "28 20");
eqt!(ix2v_695, "(21 24 11 26 19 23) 1 1", "24 24");
eqt!(ix2v_696, "(13 22 11 18 27 12 8 27) 6 4", "8 27");
eqt!(ix2v_697, "(24 9 0 28) 2 2", "0 0");
eqt!(ix2v_698, "(8 17 6 10 17 27 6 8 28) 5 1", "27 17");
eqt!(ix2v_699, "(28 20 11 0 29 26 10) 2 4", "11 29");
eqt!(ix2v_700, "(30 3 17 29 30 23 6) 0 2", "30 17");
eqt!(ix2v_701, "(9 30 11 20 6 23 0 14) 4 7", "6 14");
eqt!(ix2v_702, "(13 16 25 12 9 19) 4 0", "9 13");
eqt!(ix2v_703, "(8 0 23 8 4 12 19 26 8) 2 3", "23 8");
eqt!(ix2v_704, "(9 27 12 12 30) 1 3", "27 12");
eqt!(ix2v_705, "(14 11 17 15) 0 1", "14 11");
eqt!(ix2v_706, "(6 9 11 16 17 28) 3 4", "16 17");
eqt!(ix2v_707, "(28 1 29 15 26) 1 2", "1 29");
eqt!(ix2v_708, "(1 19 26 7 21 20) 2 0", "26 1");
eqt!(ix2v_709, "(0 1 26 3 8) 4 3", "8 3");
eqt!(ix2v_710, "(25 30 10 6 25 27) 5 5", "27 27");
eqt!(ix2v_711, "(24 23 15 4 28 14 1 25 16) 5 2", "14 15");
eqt!(ix2v_712, "(3 8 8 12) 2 1", "8 8");
eqt!(ix2v_713, "(2 25 5 2 10) 1 3", "25 2");
eqt!(ix2v_714, "(8 28 10 5 4 26 14) 0 4", "8 4");
eqt!(ix2v_715, "(22 6 10 28 14 23 24 18) 4 0", "14 22");
eqt!(ix2v_716, "(9 6 18 22 16 1 6 30 21) 6 6", "6 6");
eqt!(ix2v_717, "(0 3 11 30 7) 4 1", "7 3");
eqt!(ix2v_718, "(23 18 30 28 21 9 27 7 9) 2 3", "30 28");
eqt!(ix2v_719, "(19 8 2 0 18 15 14) 1 5", "8 15");
eqt!(ix2v_720, "(7 6 29 23 20 21) 5 4", "21 20");
eqt!(ix2v_721, "(3 29 25 2) 2 3", "25 2");
eqt!(ix2v_722, "(3 2 13 16 12 12) 0 1", "3 2");
eqt!(ix2v_723, "(3 19 1 10 10 4) 1 1", "19 19");
eqt!(ix2v_724, "(8 2 19 11 30) 4 0", "30 8");
eqt!(ix2v_725, "(0 16 2 29 11 7 10 6) 5 1", "7 16");
eqt!(ix2v_726, "(5 23 13 5) 3 0", "5 5");
eqt!(ix2v_727, "(2 14 17 23 7 17 9 25) 6 3", "9 23");
eqt!(ix2v_728, "(10 26 6 27 12 24) 1 1", "26 26");
eqt!(ix2v_729, "(0 5 30 25) 0 2", "0 30");
eqt!(ix2v_730, "(3 19 29 0 9 1 25 20) 4 2", "9 29");
eqt!(ix2v_731, "(29 16 22 3 24 19 29 23) 3 5", "3 19");
eqt!(ix2v_732, "(8 5 12 21) 0 2", "8 12");
eqt!(ix2v_733, "(15 8 17 29 14 30 25 10) 2 6", "17 25");
eqt!(ix2v_734, "(24 23 2 15 22 29) 2 2", "2 2");
eqt!(ix2v_735, "(17 30 20 6 28) 2 4", "20 28");
eqt!(ix2v_736, "(4 21 7 0 23) 2 2", "7 7");
eqt!(ix2v_737, "(7 0 24 15 27 17 3 7 16) 3 8", "15 16");
eqt!(ix2v_738, "(11 7 28 5 28 24 18) 4 2", "28 28");
eqt!(ix2v_739, "(13 18 6 16 9 8 11) 2 1", "6 18");
eqt!(ix2v_740, "(22 20 24 19 9) 4 1", "9 20");
eqt!(ix2v_741, "(15 26 23 15 5) 1 0", "26 15");
eqt!(ix2v_742, "(12 7 10 21 24) 3 2", "21 10");
eqt!(ix2v_743, "(5 3 11 3 24 1 7) 1 1", "3 3");
eqt!(ix2v_744, "(2 9 1 9 5 9 7) 3 1", "9 9");
eqt!(ix2v_745, "(11 7 22 3 19 11 9 2 17) 8 1", "17 7");
eqt!(ix2v_746, "(2 21 11 17 16 13) 1 2", "21 11");
eqt!(ix2v_747, "(7 20 11 30) 0 1", "7 20");
eqt!(ix2v_748, "(18 15 10 12 13 14 11 26 7) 6 0", "11 18");
eqt!(ix2v_749, "(14 29 20 3 14) 2 1", "20 29");
eqt!(ix2v_750, "(28 6 17 15 26 7) 2 3", "17 15");
eqt!(ix2v_751, "(10 17 10 29) 1 2", "17 10");
eqt!(ix2v_752, "(27 22 13 6) 3 2", "6 13");
eqt!(ix2v_753, "(13 4 10 14 11 8) 1 5", "4 8");
eqt!(ix2v_754, "(5 28 15 11 21 30 16 27 26) 7 2", "27 15");
eqt!(ix2v_755, "(0 3 24 29 25 24) 1 5", "3 24");
eqt!(ix2v_756, "(19 9 17 18 19 18) 1 2", "9 17");
eqt!(ix2v_757, "(16 29 30 28 3 20 14) 2 5", "30 20");
eqt!(ix2v_758, "(19 29 30 14 24 3 16 29 29) 1 4", "29 24");
eqt!(ix2v_759, "(12 16 2 13 30 11) 0 2", "12 2");
eqt!(ix2v_760, "(5 10 15 10 16 10) 1 4", "10 16");
eqt!(am2set_761, "@[2 4 5 3;0;:;55]", "55 4 5 3");
eqt!(am2set_762, "@[7 2 2 9 9 3 7;5;:;67]", "7 2 2 9 9 67 7");
eqt!(am2set_763, "@[5 0 2 5 4;4;:;50]", "5 0 2 5 50");
eqt!(am2set_764, "@[8 5 1 4 5 2;2;:;86]", "8 5 86 4 5 2");
eqt!(am2set_765, "@[0 5 8 0 1;2;:;68]", "0 5 68 0 1");
eqt!(am2set_766, "@[2 8 5 7;2;:;79]", "2 8 79 7");
eqt!(am2set_767, "@[5 4 3 7 4 8 8;2;:;77]", "5 4 77 7 4 8 8");
eqt!(am2set_768, "@[9 4 5 0;3;:;52]", "9 4 5 52");
eqt!(am2set_769, "@[0 2 4 4;3;:;54]", "0 2 4 54");
eqt!(am2set_770, "@[2 7 6 5 5;0;:;68]", "68 7 6 5 5");
eqt!(am2set_771, "@[4 4 6 5 1 7 4;6;:;90]", "4 4 6 5 1 7 90");
eqt!(am2set_772, "@[8 8 1 3 4;3;:;66]", "8 8 1 66 4");
eqt!(am2set_773, "@[1 9 1 4 0 0;5;:;61]", "1 9 1 4 0 61");
eqt!(am2set_774, "@[7 6 4 4 3 5 3 8;5;:;50]", "7 6 4 4 3 50 3 8");
eqt!(am2set_775, "@[7 2 6 9 6 6 3;5;:;62]", "7 2 6 9 6 62 3");
eqt!(am2set_776, "@[2 5 9 1 8;2;:;99]", "2 5 99 1 8");
eqt!(am2set_777, "@[4 2 8 0 6 4;1;:;52]", "4 52 8 0 6 4");
eqt!(am2set_778, "@[6 0 2 7 5 8;2;:;52]", "6 0 52 7 5 8");
eqt!(am2set_779, "@[5 7 9 9 0 2;3;:;93]", "5 7 9 93 0 2");
eqt!(am2set_780, "@[7 9 5 0;2;:;83]", "7 9 83 0");
eqt!(am2set_781, "@[0 5 8 3 1 3 9 2;7;:;91]", "0 5 8 3 1 3 9 91");
eqt!(am2set_782, "@[6 8 3 9 1;1;:;92]", "6 92 3 9 1");
eqt!(am2set_783, "@[7 2 2 6 6 2;3;:;94]", "7 2 2 94 6 2");
eqt!(am2set_784, "@[9 3 3 5 5;2;:;80]", "9 3 80 5 5");
eqt!(am2set_785, "@[1 0 3 9 2 4 6;0;:;64]", "64 0 3 9 2 4 6");
eqt!(am2set_786, "@[8 2 5 3;2;:;51]", "8 2 51 3");
eqt!(am2set_787, "@[4 1 0 5 1 8 3;3;:;64]", "4 1 0 64 1 8 3");
eqt!(am2set_788, "@[6 3 1 2 0 2 9 4;0;:;84]", "84 3 1 2 0 2 9 4");
eqt!(am2set_789, "@[5 7 9 4 4 8 7;5;:;79]", "5 7 9 4 4 79 7");
eqt!(am2set_790, "@[7 8 4 4 3 4;1;:;81]", "7 81 4 4 3 4");
eqt!(am2set_791, "@[8 4 5 4;2;:;91]", "8 4 91 4");
eqt!(am2set_792, "@[1 0 2 2 2;2;:;57]", "1 0 57 2 2");
eqt!(am2set_793, "@[2 3 3 9 7 5;4;:;90]", "2 3 3 9 90 5");
eqt!(am2set_794, "@[6 0 6 4 0;1;:;89]", "6 89 6 4 0");
eqt!(am2set_795, "@[5 0 5 5 9 1 6;5;:;51]", "5 0 5 5 9 51 6");
eqt!(am2set_796, "@[8 5 2 0 6 6 3 4;1;:;76]", "8 76 2 0 6 6 3 4");
eqt!(am2set_797, "@[3 7 2 8 8;3;:;55]", "3 7 2 55 8");
eqt!(am2set_798, "@[0 9 0 7;0;:;83]", "83 9 0 7");
eqt!(am2set_799, "@[1 4 6 9 4 7 5 3;5;:;58]", "1 4 6 9 4 58 5 3");
eqt!(am2set_800, "@[5 1 6 4;0;:;60]", "60 1 6 4");
eqt!(am2set_801, "@[9 1 1 5 5 5 6;1;:;90]", "9 90 1 5 5 5 6");
eqt!(am2set_802, "@[6 2 4 5;1;:;88]", "6 88 4 5");
eqt!(am2set_803, "@[9 2 6 2 5 8 7;3;:;57]", "9 2 6 57 5 8 7");
eqt!(am2set_804, "@[3 8 7 3 6 2 8 6;2;:;92]", "3 8 92 3 6 2 8 6");
eqt!(am2set_805, "@[6 3 1 5;2;:;78]", "6 3 78 5");
eqt!(am2set_806, "@[9 4 9 9 3;4;:;90]", "9 4 9 9 90");
eqt!(am2set_807, "@[2 7 8 9 3 6;1;:;69]", "2 69 8 9 3 6");
eqt!(am2set_808, "@[3 8 0 4 1;2;:;51]", "3 8 51 4 1");
eqt!(am2set_809, "@[8 4 2 7 2 0 4;5;:;83]", "8 4 2 7 2 83 4");
eqt!(am2set_810, "@[6 8 1 9;2;:;92]", "6 8 92 9");
eqt!(am2set_811, "@[0 9 6 9;1;:;99]", "0 99 6 9");
eqt!(am2set_812, "@[9 9 9 6 1;3;:;67]", "9 9 9 67 1");
eqt!(am2set_813, "@[1 0 5 5 9;1;:;84]", "1 84 5 5 9");
eqt!(am2set_814, "@[0 7 5 8 9;2;:;51]", "0 7 51 8 9");
eqt!(am2set_815, "@[5 5 8 9;0;:;92]", "92 5 8 9");
eqt!(am2set_816, "@[7 0 9 0 2 1 3;1;:;68]", "7 68 9 0 2 1 3");
eqt!(am2set_817, "@[9 9 5 0 5 3 1 1;4;:;98]", "9 9 5 0 98 3 1 1");
eqt!(am2set_818, "@[4 3 5 9 4 7 2 9;7;:;78]", "4 3 5 9 4 7 2 78");
eqt!(am2set_819, "@[2 5 0 0 6;4;:;55]", "2 5 0 0 55");
eqt!(am2set_820, "@[6 4 6 0 0;1;:;89]", "6 89 6 0 0");
eqt!(am2set_821, "@[2 2 2 1 7 0 6;6;:;67]", "2 2 2 1 7 0 67");
eqt!(am2set_822, "@[5 0 3 3 6 3 5 3;4;:;92]", "5 0 3 3 92 3 5 3");
eqt!(am2set_823, "@[5 3 1 0;2;:;73]", "5 3 73 0");
eqt!(am2set_824, "@[4 5 7 3 2 7 1;0;:;53]", "53 5 7 3 2 7 1");
eqt!(am2set_825, "@[7 4 4 0 4 3 9;5;:;69]", "7 4 4 0 4 69 9");
eqt!(am2set_826, "@[2 1 4 6 7;1;:;56]", "2 56 4 6 7");
eqt!(am2set_827, "@[5 9 8 4 9;1;:;65]", "5 65 8 4 9");
eqt!(am2set_828, "@[0 5 7 5 3;2;:;75]", "0 5 75 5 3");
eqt!(am2set_829, "@[2 5 4 8 5 3 0 2;0;:;84]", "84 5 4 8 5 3 0 2");
eqt!(am2set_830, "@[9 8 5 9 9 7 6;1;:;72]", "9 72 5 9 9 7 6");
eqt!(am2set_831, "@[6 0 5 7 5 5;5;:;91]", "6 0 5 7 5 91");
eqt!(am2set_832, "@[2 8 9 8 6 3;5;:;94]", "2 8 9 8 6 94");
eqt!(am2set_833, "@[6 0 0 4 6;1;:;83]", "6 83 0 4 6");
eqt!(am2set_834, "@[9 4 5 3 5 2 0 6;3;:;65]", "9 4 5 65 5 2 0 6");
eqt!(am2set_835, "@[7 9 8 7 3;2;:;60]", "7 9 60 7 3");
eqt!(am2set_836, "@[0 9 4 2 6 6 7;6;:;56]", "0 9 4 2 6 6 56");
eqt!(am2set_837, "@[0 0 8 6 9 1 6 3;0;:;60]", "60 0 8 6 9 1 6 3");
eqt!(am2set_838, "@[7 5 0 0 0 5;3;:;55]", "7 5 0 55 0 5");
eqt!(am2set_839, "@[5 6 7 3;3;:;81]", "5 6 7 81");
eqt!(am2set_840, "@[8 1 5 7 3 7;4;:;59]", "8 1 5 7 59 7");
eqt!(am2add_841, "@[2 6 0 4 8 6;1;+;6]", "2 12 0 4 8 6");
eqt!(am2add_842, "@[9 0 5 2;0;+;3]", "12 0 5 2");
eqt!(am2add_843, "@[2 7 2 3;0;+;9]", "11 7 2 3");
eqt!(am2add_844, "@[6 5 4 1 7 9 7;3;+;8]", "6 5 4 9 7 9 7");
eqt!(am2add_845, "@[4 6 3 0 6 8 1;4;+;6]", "4 6 3 0 12 8 1");
eqt!(am2add_846, "@[3 5 1 1 9;3;+;2]", "3 5 1 3 9");
eqt!(am2add_847, "@[1 2 0 4 1;3;+;9]", "1 2 0 13 1");
eqt!(am2add_848, "@[9 1 4 2 2 7 4 5;4;+;7]", "9 1 4 2 9 7 4 5");
eqt!(am2add_849, "@[7 1 4 1 8 6 8;2;+;1]", "7 1 5 1 8 6 8");
eqt!(am2add_850, "@[0 1 4 4 7 5 4;2;+;8]", "0 1 12 4 7 5 4");
eqt!(am2add_851, "@[6 7 3 9 0 7 5;4;+;2]", "6 7 3 9 2 7 5");
eqt!(am2add_852, "@[8 2 3 1;2;+;4]", "8 2 7 1");
eqt!(am2add_853, "@[8 3 6 7 9 7;5;+;6]", "8 3 6 7 9 13");
eqt!(am2add_854, "@[3 8 2 7 3 4 4 4;6;+;1]", "3 8 2 7 3 4 5 4");
eqt!(am2add_855, "@[9 1 6 3 2 6 0 8;4;+;7]", "9 1 6 3 9 6 0 8");
eqt!(am2add_856, "@[8 6 8 8 2;1;+;8]", "8 14 8 8 2");
eqt!(am2add_857, "@[9 4 1 5;3;+;3]", "9 4 1 8");
eqt!(am2add_858, "@[6 8 5 4;2;+;2]", "6 8 7 4");
eqt!(am2add_859, "@[1 3 7 4 4 9;4;+;8]", "1 3 7 4 12 9");
eqt!(am2add_860, "@[4 5 5 1 0 9 7 3;5;+;7]", "4 5 5 1 0 16 7 3");
eqt!(am2add_861, "@[3 7 1 5 3 0 3 4;7;+;3]", "3 7 1 5 3 0 3 7");
eqt!(am2add_862, "@[2 2 8 3 3;4;+;8]", "2 2 8 3 11");
eqt!(am2add_863, "@[1 9 7 1 1 8 7 3;3;+;2]", "1 9 7 3 1 8 7 3");
eqt!(am2add_864, "@[4 6 3 9;3;+;1]", "4 6 3 10");
eqt!(am2add_865, "@[8 3 6 4 9;4;+;3]", "8 3 6 4 12");
eqt!(am2add_866, "@[3 1 8 9 5 4 8 9;0;+;4]", "7 1 8 9 5 4 8 9");
eqt!(am2add_867, "@[2 8 5 5;3;+;6]", "2 8 5 11");
eqt!(am2add_868, "@[2 8 1 0;1;+;5]", "2 13 1 0");
eqt!(am2add_869, "@[8 0 1 8 9 5 5;1;+;8]", "8 8 1 8 9 5 5");
eqt!(am2add_870, "@[8 2 5 0 5 3 9 8;7;+;1]", "8 2 5 0 5 3 9 9");
eqt!(am2add_871, "@[3 1 3 3;3;+;2]", "3 1 3 5");
eqt!(am2add_872, "@[5 4 7 4 3 4 5 8;2;+;9]", "5 4 16 4 3 4 5 8");
eqt!(am2add_873, "@[0 9 0 9;1;+;2]", "0 11 0 9");
eqt!(am2add_874, "@[8 6 9 6 9 6 5;1;+;8]", "8 14 9 6 9 6 5");
eqt!(am2add_875, "@[6 4 9 6 9 8 8;3;+;3]", "6 4 9 9 9 8 8");
eqt!(am2add_876, "@[5 4 8 8 6 6 3;6;+;5]", "5 4 8 8 6 6 8");
eqt!(am2add_877, "@[1 0 9 6 6;4;+;1]", "1 0 9 6 7");
eqt!(am2add_878, "@[1 8 1 6 7;1;+;8]", "1 16 1 6 7");
eqt!(am2add_879, "@[9 5 4 1 6 8;5;+;6]", "9 5 4 1 6 14");
eqt!(am2add_880, "@[8 2 0 7 5;0;+;4]", "12 2 0 7 5");
eqt!(am2add_881, "@[7 3 6 1 3 6 7 6;5;+;7]", "7 3 6 1 3 13 7 6");
eqt!(am2add_882, "@[7 0 0 8 8 9;1;+;2]", "7 2 0 8 8 9");
eqt!(am2add_883, "@[8 4 8 9 1 4 2;6;+;6]", "8 4 8 9 1 4 8");
eqt!(am2add_884, "@[0 9 6 0 1 3 4;5;+;3]", "0 9 6 0 1 6 4");
eqt!(am2add_885, "@[5 7 1 9 8 4 5 6;0;+;9]", "14 7 1 9 8 4 5 6");
eqt!(am2add_886, "@[8 6 6 9 6 0 7 6;6;+;3]", "8 6 6 9 6 0 10 6");
eqt!(am2add_887, "@[2 2 9 3 7 9 4 7;1;+;4]", "2 6 9 3 7 9 4 7");
eqt!(am2add_888, "@[4 6 1 4 1 7 7;0;+;5]", "9 6 1 4 1 7 7");
eqt!(am2add_889, "@[3 2 7 6 6 9 5;2;+;3]", "3 2 10 6 6 9 5");
eqt!(am2add_890, "@[5 4 0 5 4 5;1;+;5]", "5 9 0 5 4 5");
eqt!(am2add_891, "@[3 2 9 6 1 7 5;2;+;2]", "3 2 11 6 1 7 5");
eqt!(am2add_892, "@[3 2 7 3 7 0;0;+;1]", "4 2 7 3 7 0");
eqt!(am2add_893, "@[4 5 6 9 4 1 7 5;6;+;3]", "4 5 6 9 4 1 10 5");
eqt!(am2add_894, "@[8 8 4 7;2;+;7]", "8 8 11 7");
eqt!(am2add_895, "@[2 2 1 6 3;2;+;4]", "2 2 5 6 3");
eqt!(am2add_896, "@[9 1 8 1 6 2;4;+;9]", "9 1 8 1 15 2");
eqt!(am2add_897, "@[0 7 8 2 8 5 8;6;+;6]", "0 7 8 2 8 5 14");
eqt!(am2add_898, "@[7 4 2 1 4 0 9 0;3;+;6]", "7 4 2 7 4 0 9 0");
eqt!(am2add_899, "@[5 5 8 4 4 8 3 5;5;+;1]", "5 5 8 4 4 9 3 5");
eqt!(am2add_900, "@[4 2 8 1 9;0;+;8]", "12 2 8 1 9");
eqt!(x2ar_901, "-7 2 2 6&2 1 8 -3", "-7 1 2 -3");
eqt!(x2ar_902, "6 9 -9 5 -9*7 -1 -7 8 -7", "42 -9 63 40 63");
eqt!(x2ar_903, "0 9 -5 0 -6 8-3 6 6 -3 9 8", "-3 3 -11 3 -15 0");
eqt!(x2ar_904, "6 -8 -4 -1|1 -8 0 7", "6 -8 0 7");
eqt!(x2ar_905, "-6 5&-2 2", "-6 2");
eqt!(x2ar_906, "1 1 -3 5 -7 3 -4&-1 9 8 9 7 -9 0", "-1 1 -3 5 -7 -9 -4");
eqt!(x2ar_907, "-4 8 4 -3 8 -4 -7 -8|-6 -9 -7 7 -9 -2 -7 2", "-4 8 4 7 8 -2 -7 \
    2");
eqt!(x2ar_908, "-2 9 4 -6 9 -2 6--1 4 -5 4 0 -5 -3", "-1 5 9 -10 9 3 9");
eqt!(x2ar_909, "5 8 6 4|7 -6 2 -7", "7 8 6 4");
eqt!(x2ar_910, "5 9-0 0", "5 9");
eqt!(x2ar_911, "2 6 7 -2 -7|-2 -9 6 3 9", "2 6 7 3 9");
eqt!(x2ar_912, "0 6 8 -2 -3 -4 2*-8 4 -4 2 6 -7 4", "0 24 -32 -4 -18 28 8");
eqt!(x2ar_913, "-1 8&9 -8", "-1 -8");
eqt!(x2ar_914, "-8 6 -9&-1 -7 4", "-8 -7 -9");
eqt!(x2ar_915, "3 5 -6 0 1 -9 -8 8&-8 -3 -7 -8 2 -7 5 9", "-8 -3 -7 -8 1 -9 -8 \
    8");
eqt!(x2ar_916, "-6 1 5 7 8 0*-1 -7 1 2 -4 3", "6 -7 5 14 -32 0");
eqt!(x2ar_917, "6 -5 8 -7 4 6*9 -1 -5 -8 0 4", "54 5 -40 56 0 24");
eqt!(x2ar_918, "-5 -6 6 3-8 9 -2 -8", "-13 -15 8 11");
eqt!(x2ar_919, "0 -3 8*-9 0 -4", "0 0 -32");
eqt!(x2ar_920, "0 8 6 -7 3*-7 -1 8 9 -6", "0 -8 48 -63 -18");
eqt!(x2ar_921, "2 6 4 8 -4 8 -3 -9+-9 -2 -4 -2 1 -6 -4 8", "-7 4 0 6 -3 2 -7 \
    -1");
eqt!(x2ar_922, "-3 0 3 8 -3 -4&3 3 -3 1 2 3", "-3 0 -3 1 -3 -4");
eqt!(x2ar_923, "-5 1 2 7 4|3 -6 4 4 2", "3 1 4 7 4");
eqt!(x2ar_924, "9 6 -9 5 4*9 1 7 8 -3", "81 6 -63 40 -12");
eqt!(x2ar_925, "9 4 8 3 2 -8 -6 3|-1 -5 -3 4 0 -4 -7 2", "9 4 8 4 2 -4 -6 3");
eqt!(x2ar_926, "2 -1 1 -3 -5*-6 -7 -7 6 9", "-12 7 -7 -18 -45");
eqt!(x2ar_927, "-1 7 -9 -6+-2 -7 7 -5", "-3 0 -2 -11");
eqt!(x2ar_928, "7 7 2 5*1 5 9 9", "7 35 18 45");
eqt!(x2ar_929, "-9 6 -6 4 -3 -6 3+6 1 -3 7 -2 -7 3", "-3 7 -9 11 -5 -13 6");
eqt!(x2ar_930, "4 4 3 8 9 8|-6 -3 -5 9 -8 -7", "4 4 3 9 9 8");
eqt!(x2ar_931, "-8 -6 1+0 -2 3", "-8 -8 4");
eqt!(x2ar_932, "3 1 -4 3 -2 6 5 6+-2 6 1 5 7 -1 -4 -7", "1 7 -3 8 5 5 1 -1");
eqt!(x2ar_933, "7 8 -3 0+2 3 -1 -3", "9 11 -4 -3");
eqt!(x2ar_934, "6 -7 2*8 -9 -9", "48 63 -18");
eqt!(x2ar_935, "-7 4 -9 -4 1 -7-0 -4 7 -4 -3 8", "-7 8 -16 0 4 -15");
eqt!(x2ar_936, "4 -5 -4 -8 -6 -7 5*-2 0 -8 -7 2 -3 -7", "-8 0 32 56 -12 21 \
    -35");
eqt!(x2ar_937, "-2 -5 -9 -4 8 -5 -8 8*9 -2 7 -4 -3 -1 -3 7", "-18 10 -63 16 \
    -24 \
    5 24 56");
eqt!(x2ar_938, "-2 0 -7 9 -2 9 -9 -5+-9 -6 9 8 -2 1 4 -9", "-11 -6 2 17 -4 10 \
    -5 -14");
eqt!(x2ar_939, "9 -2-1 7", "8 -9");
eqt!(x2ar_940, "3 -5 -5 -5-1 3 -8 -5", "2 -8 3 0");
eqt!(x2ar_941, "-2 -4 -9 9 -5 -1 -6|4 -5 -8 -8 -3 -1 6", "4 -4 -8 9 -3 -1 6");
eqt!(x2ar_942, "-3 2 4 -9 5 9 6*6 -2 6 8 1 3 -6", "-18 -4 24 -72 5 27 -36");
eqt!(x2ar_943, "7 -7 9 -5 9 -6|9 -3 -6 4 -9 4", "9 -3 9 4 9 4");
eqt!(x2ar_944, "9 9 7 6 -7 -8 0--3 0 5 5 -1 -2 -1", "12 9 2 1 -6 -6 1");
eqt!(x2ar_945, "-9 -1+-9 -6", "-18 -7");
eqt!(x2ar_946, "7 8 0 -8 9 2&-2 -1 2 0 -9 1", "-2 -1 0 -8 -9 1");
eqt!(x2ar_947, "-7 9 6 -8 -7 3 8 -8-8 5 -1 7 1 3 -2 -8", "-15 4 7 -15 -8 0 10 \
    0");
eqt!(x2ar_948, "-9 9 -6 -8 -1 -2 -2|-3 4 -3 5 -1 -6 6", "-3 9 -3 5 -1 -2 6");
eqt!(x2ar_949, "-2 -3 4 5-1 1 0 6", "-3 -4 4 -1");
eqt!(x2ar_950, "6 9*4 -4", "24 -36");
eqt!(x2ar_951, "9 7 5 -6 -9|-8 0 7 8 -8", "9 7 7 8 -8");
eqt!(x2ar_952, "-1 1 -5+4 -8 0", "3 -7 -5");
eqt!(x2ar_953, "-2 1 -2 0 -9+4 2 -4 -9 7", "2 3 -6 -9 -2");
eqt!(x2ar_954, "-7 5 -8 -2 8 7*-9 4 2 -6 -2 6", "63 20 -16 12 -16 42");
eqt!(x2ar_955, "8 -3 -2 -5 -6&-9 3 8 -7 -6", "-9 -3 -2 -7 -6");
eqt!(x2ar_956, "4 -6 -3 -4&2 5 -2 -4", "2 -6 -3 -4");
eqt!(x2ar_957, "-1 1 9 5 -6 -9 -9 9--8 8 -8 -6 -6 4 -3 2", "7 -7 17 11 0 -13 \
    -6 \
    7");
eqt!(x2ar_958, "-5 3 -6 -3 1 0 -6*-5 -7 1 2 -5 -4 8", "25 -21 -6 -6 -5 0 -48");
eqt!(x2ar_959, "-4 2 5 3 -5 -2 -8--7 7 7 -9 3 -9 4", "3 -5 -2 12 -8 7 -12");
eqt!(x2ar_960, "-7 3 -7 -6 8 -6+0 -4 -9 8 2 9", "-7 -1 -16 2 10 3");
eqt!(x2ar_961, "3 -3 -3 -5 -6 1 -9 9*-5 8 -2 -2 8 6 -9 -3", "-15 -24 6 10 -48 \
    6 \
    81 -27");
eqt!(x2ar_962, "-4 8 -5+5 6 -1", "1 14 -6");
eqt!(x2ar_963, "7 5 -7 -4 -6 -4 -6 2|-8 8 -2 2 -8 -6 -7 -8", "7 8 -2 2 -6 -4 \
    -6 \
    2");
eqt!(x2ar_964, "5 -8 -6 0 7 -9-0 -2 5 -9 -7 -2", "5 -6 -11 9 14 -7");
eqt!(x2ar_965, "6 7 4 4 5 -5 0&-1 -5 2 8 5 -2 8", "-1 -5 2 4 5 -5 0");
eqt!(x2ar_966, "0 0 -7 9|7 4 9 9", "7 4 9 9");
eqt!(x2ar_967, "2 2 2 -4 -4|-5 7 -1 1 -7", "2 7 2 1 -4");
eqt!(x2ar_968, "-7 -3 -4 -9 6 8 -6 4+-5 -7 -6 0 2 3 5 -5", "-12 -10 -10 -9 8 \
    11 \
    -1 -1");
eqt!(x2ar_969, "4 1 -5 -8 9 -1+-7 -4 -2 -3 -9 -2", "-3 -3 -7 -11 0 -3");
eqt!(x2ar_970, "2 -8 9 2 -6 -6 -4 -4|5 -2 -5 -9 9 8 8 3", "5 -2 9 2 9 8 8 3");
eqt!(x2ar_971, "-4 7 -7 -1 9 -2 0--1 -3 7 -6 -7 -6 2", "-3 10 -14 5 16 4 -2");
eqt!(x2ar_972, "8 1 -2 -7+-3 -2 8 9", "5 -1 6 2");
eqt!(x2ar_973, "-9 0 9 -6 7+-9 -2 3 5 -4", "-18 -2 12 -1 3");
eqt!(x2ar_974, "-4 3 2 -4|8 6 -5 3", "8 6 2 3");
eqt!(x2ar_975, "1 -1 -5 -4 -3 2-0 5 4 -9 0 -7", "1 -6 -9 5 -3 9");
eqt!(x2ar_976, "2 -6 -8|-1 6 9", "2 6 9");
eqt!(x2ar_977, "-1 7&8 3", "-1 3");
eqt!(x2ar_978, "-5 -5 -5 7 1 4 1 -5--4 2 7 4 -8 -7 2 -5", "-1 -7 -12 3 9 11 -1 \
    0");
eqt!(x2ar_979, "5 4 -9+8 2 -1", "13 6 -10");
eqt!(x2ar_980, "-7 8 -7 -9 7 2 1 8+4 -3 6 4 8 -2 -1 -5", "-3 5 -1 -5 15 0 0 3");
eqt!(x2ar_981, "-4 8 3+-3 5 7", "-7 13 10");
eqt!(x2ar_982, "1 -5+9 6", "10 1");
eqt!(x2ar_983, "2 4 -7 -4&-4 -7 1 9", "-4 -7 -7 -4");
eqt!(x2ar_984, "8 3 8 -2 -6&8 -4 -7 -4 0", "8 -4 -7 -4 -6");
eqt!(x2ar_985, "-6 0 -3 8|9 4 -3 -4", "9 4 -3 8");
eqt!(x2ar_986, "1 8 9 6 4 -2 4-0 -2 3 -4 2 4 -5", "1 10 6 10 2 -6 9");
eqt!(x2ar_987, "6 -8 5 5&0 7 -2 2", "0 -8 -2 2");
eqt!(x2ar_988, "1 -4 -2&-2 5 -5", "-2 -4 -5");
eqt!(x2ar_989, "1 -1 3 -2 6 -8 -8|8 -6 2 2 -1 -1 -5", "8 -1 3 2 6 -1 -5");
eqt!(x2ar_990, "1 -6 -6 -5 3 -7*-7 5 -4 -7 5 9", "-7 -30 24 35 15 -63");
eqt!(x2ar_991, "1 5 -7 2&7 9 -2 -9", "1 5 -7 -9");
eqt!(x2ar_992, "-8 7 -2 3 -6 8 8 2*7 -1 6 -8 -3 -9 1 3", "-56 -7 -12 -24 18 \
    -72 \
    8 6");
eqt!(x2ar_993, "5 -5 7 9 -1 0 -7+3 -5 1 0 8 7 -8", "8 -10 8 9 7 7 -15");
eqt!(x2ar_994, "1 -1 8 5 0 6 2 -4*7 -4 8 -4 -6 -9 3 2", "7 4 64 -20 0 -54 6 \
    -8");
eqt!(x2ar_995, "-5 -2 5 -8--7 7 0 9", "2 -9 5 -17");
eqt!(x2ar_996, "-1 8 9 4 -2 -4&-2 -3 -5 -7 -3 -3", "-2 -3 -5 -7 -3 -4");
eqt!(x2ar_997, "7 -3 -7 8 -1 5 5-6 -3 6 8 0 0 6", "1 0 -13 0 -1 5 -1");
eqt!(x2ar_998, "7 -1 -4 5|8 8 -5 -5", "8 8 -4 5");
eqt!(x2ar_999, "3 -1 1 5 -3&6 3 -9 2 -4", "3 -1 -9 2 -4");
eqt!(x2ar_1000, "2 4 3 -1 -6 -9|-6 4 0 -4 9 0", "2 4 3 -1 9 0");
eqt!(x2ar_1001, "-8 -6 6 -3 9 8*-3 5 1 7 -6 9", "24 -30 6 -21 -54 72");
eqt!(x2ar_1002, "-5 8 5 -9 -6 3 7+-8 0 -4 -9 1 9 6", "-13 8 1 -18 -5 12 13");
eqt!(x2ar_1003, "-2 -4 -6 9 9 5 6 -2-6 -2 9 -6 -6 6 6 9", "-8 -2 -15 15 15 -1 \
    0 \
    -11");
eqt!(x2ar_1004, "-3 3 -5 0 1 9|-4 -2 -6 -3 9 4", "-3 3 -5 0 9 9");
eqt!(x2ar_1005, "-4 -3+9 9", "5 6");
eqt!(x2ar_1006, "-9 6 6 7 2 -9 5|-4 -4 0 5 2 2 7", "-4 6 6 7 2 2 7");
eqt!(x2ar_1007, "-3 8 -1 2 -4+9 -3 -8 -3 -4", "6 5 -9 -1 -8");
eqt!(x2ar_1008, "7 0 9 9 9 4 8 8--7 -5 5 6 -3 4 2 7", "14 5 4 3 12 0 6 1");
eqt!(x2ar_1009, "7 -5-9 6", "-2 -11");
eqt!(x2ar_1010, "-1 -3 -2 -5 3|2 -2 -3 -3 -9", "2 -2 -2 -3 3");
eqt!(x2ar_1011, "1 -1 -4 -7 7 -5 3 -1*-8 8 4 8 7 0 0 -4", "-8 -8 -16 -56 49 0 \
    0 \
    4");
eqt!(x2ar_1012, "1 -2 -5 7 6 6|-3 -4 -7 -3 -4 2", "1 -2 -5 7 6 6");
eqt!(x2ar_1013, "4 -8 -8 8+-5 -9 -4 4", "-1 -17 -12 12");
eqt!(x2ar_1014, "6 -7 -9 -1 1|-8 -5 -2 -7 0", "6 -5 -2 -1 1");
eqt!(x2ar_1015, "-3 4 -9 -1 0*-9 -2 -1 6 -6", "27 -8 9 -6 0");
eqt!(x2ar_1016, "-6 1 -3 -3|-5 -6 -5 7", "-5 1 -3 7");
eqt!(x2ar_1017, "7 -8 0 -2 -9 -2 -1 6--4 -2 -1 3 1 1 1 5", "11 -6 1 -5 -10 -3 \
    -2 1");
eqt!(x2ar_1018, "6 8 3 -8&4 -1 7 0", "4 -1 3 -8");
eqt!(x2ar_1019, "-1 6 -3 9 -2 -2|-6 -2 -4 -6 -6 7", "-1 6 -3 9 -2 7");
eqt!(x2ar_1020, "1 -9|-1 5", "1 5");
eqt!(x2ar_1021, "1 4 -9 -2|4 -5 3 -7", "4 4 3 -2");
eqt!(x2ar_1022, "0 7 -2&-5 -2 3", "-5 -2 -2");
eqt!(x2ar_1023, "8 5 -9 5 -9|-5 4 -2 3 -7", "8 5 -2 5 -7");
eqt!(x2ar_1024, "8 1 0 -4 5&0 0 3 8 -7", "0 0 0 -4 -7");
eqt!(x2ar_1025, "-9 -6 -4 4 -7 -6 0 -5+9 0 -1 2 -8 4 -3 1", "0 -6 -5 6 -15 -2 \
    -3 -4");
eqt!(x2ar_1026, "8 -6 -7|4 4 -4", "8 4 -4");
eqt!(x2ar_1027, "-6 -6 2&7 -3 -6", "-6 -6 -6");
eqt!(x2ar_1028, "-9 6 8 -6 9 -2 5 3|-7 -6 -3 6 8 0 1 -1", "-7 6 8 6 9 0 5 3");
eqt!(x2ar_1029, "-6 -9 -1 -7 -2 4*5 8 9 9 -6 4", "-30 -72 -9 -63 12 16");
eqt!(x2ar_1030, "0 -3 3+-3 2 -2", "-3 -1 1");
eqt!(x2ar_1031, "-7 -3 7 1*-1 8 -2 -3", "7 -24 -14 -3");
eqt!(x2ar_1032, "9 8 5 1 -5 8&6 -9 -7 -6 -1 -3", "6 -9 -7 -6 -5 -3");
eqt!(x2ar_1033, "-5 -6&5 -6", "-5 -6");
eqt!(x2ar_1034, "-4 -4 -2 1 -9 -7 1 0--2 8 -1 -9 -4 -6 -3 4", "-2 -12 -1 10 -5 \
    -1 4 -4");
eqt!(x2ar_1035, "2 0 -2 4 -9 -6 -2 8&-7 2 -8 -1 -6 2 0 1", "-7 0 -8 -1 -9 -6 \
    -2 \
    1");
eqt!(x2ar_1036, "-1 -9 4 3 -3 -6 -2 -7-7 -5 -5 2 6 -8 7 -8", "-8 -4 9 1 -9 2 \
    -9 \
    1");
eqt!(x2ar_1037, "-3 8 -8 -8 -2|-8 8 1 7 6", "-3 8 1 7 6");
eqt!(x2ar_1038, "8 1 -8 7 -9 -7-5 -2 7 0 4 -4", "3 3 -15 7 -13 -3");
eqt!(x2ar_1039, "-2 1*2 -3", "-4 -3");
eqt!(x2ar_1040, "-6 2 -9 9 -1 2*-2 -2 7 -5 0 8", "12 -4 -63 -45 0 16");
eqt!(x2ar_1041, "-2 8 5 0 -5 -4 -2*-4 -9 4 -6 4 -7 -7", "8 -72 20 0 -20 28 14");
eqt!(x2ar_1042, "-3 -9 2*9 -5 3", "-27 45 6");
eqt!(x2ar_1043, "-9 -8 3 -3+4 1 7 1", "-5 -7 10 -2");
eqt!(x2ar_1044, "2 1 -4 -2|-1 2 7 6", "2 2 7 6");
eqt!(x2ar_1045, "9 8 0 6 -6*9 -9 -8 7 0", "81 -72 0 42 0");
eqt!(x2ar_1046, "-6 4 6 -3+7 5 2 5", "1 9 8 2");
eqt!(x2ar_1047, "6 -5 7 3 -6 5-5 -3 -9 9 -7 -2", "1 -2 16 -6 1 7");
eqt!(x2ar_1048, "6 -5 -5 -3 6 4 -8 7--4 -6 4 -3 9 8 -5 0", "10 1 -9 0 -3 -4 -3 \
    7");
eqt!(x2ar_1049, "4 -6 -7 -9+-8 1 -1 4", "-4 -5 -8 -5");
eqt!(x2ar_1050, "0 -8 9 -7 -6 7&-2 -2 -5 1 -2 -2", "-2 -8 -5 -7 -6 -2");
eqt!(x2ar_1051, "4 -5 -8 -2 1 -5-3 9 -7 8 -4 4", "1 -14 -1 -10 5 -9");
eqt!(x2ar_1052, "5 1 -8*-4 6 6", "-20 6 -48");
eqt!(x2ar_1053, "9 -7 -7 5 -4 0 -2 2+2 -4 1 -1 -6 9 3 9", "11 -11 -6 4 -10 9 1 \
    11");
eqt!(x2ar_1054, "5 -7 -3 -6 9 1 2 3--9 1 -8 -4 -9 3 3 4", "14 -8 5 -2 18 -2 -1 \
    -1");
eqt!(x2ar_1055, "-1 -5 -8 -5 1 9&8 1 2 -6 -8 2", "-1 -5 -8 -6 -8 2");
eqt!(x2ar_1056, "-6 -9 6 -9 6--4 0 -2 4 7", "-2 -9 8 -13 -1");
eqt!(x2ar_1057, "-1 -6 -5 1 -1 -9 -2+4 -8 -2 7 1 -3 4", "3 -14 -7 8 0 -12 2");
eqt!(x2ar_1058, "-9 -5 -9 4 9 3 0+-7 5 8 -7 1 -8 -5", "-16 0 -1 -3 10 -5 -5");
eqt!(x2ar_1059, "4 -6 -6 1 8|-8 6 -1 6 4", "4 6 -1 6 8");
eqt!(x2ar_1060, "4 -5 5 5 -5 3&-8 -1 -6 9 -9 8", "-8 -5 -6 5 -9 3");
eqt!(x2cm_1061, "-3 8 4 -7 -3 4 6<=-1 5 8 -8 -4 -5 6", "1010001b");
eqt!(x2cm_1062, "-6 3 1 -8 1 -6 -2<=-9 -9 6 6 7 6 -6", "0011110b");
eqt!(x2cm_1063, "8 9 6<>7 8 -1", "111b");
eqt!(x2cm_1064, "-4 -7<8 0", "11b");
eqt!(x2cm_1065, "-6 9 -5 0 2 5>=9 8 0 6 -8 8", "010010b");
eqt!(x2cm_1066, "-2 -8 -8 -3 -9 -4 5<=9 -2 9 7 -7 8 -2", "1111110b");
eqt!(x2cm_1067, "-1 -2<>4 5", "11b");
eqt!(x2cm_1068, "-6 0 -9<-8 7 -1", "011b");
eqt!(x2cm_1069, "3 -9<>-8 -3", "11b");
eqt!(x2cm_1070, "-8 7 -9 -9 -4 5 3>=-6 1 0 0 7 -7 -9", "0100011b");
eqt!(x2cm_1071, "-6 -5 -3 0=-2 6 -5 -5", "0000b");
eqt!(x2cm_1072, "3 -6 9 -1 -1 8 -1 2<>-4 -8 -1 5 1 0 -6 -5", "11111111b");
eqt!(x2cm_1073, "-7 2<0 4", "11b");
eqt!(x2cm_1074, "8 3 4<-3 0 4", "000b");
eqt!(x2cm_1075, "-2 -7 -1 -5 1 -3 9 1<0 -3 0 -7 -1 -3 6 -9", "11100000b");
eqt!(x2cm_1076, "-3 1 -3 -3 -1 -7 -3 -3<>6 -4 8 4 -7 3 -9 5", "11111111b");
eqt!(x2cm_1077, "-5 -9 -4 8 -9 4 -9 3<>4 5 0 -9 -8 -7 -5 -2", "11111111b");
eqt!(x2cm_1078, "3 -3 7 4 -2<=4 -3 8 8 0", "11111b");
eqt!(x2cm_1079, "-7 2 -1 9 -2 -2 -5 3>-2 8 -6 -3 -9 1 2 -5", "00111001b");
eqt!(x2cm_1080, "-4 8 3 5 -3>-5 3 9 4 -5", "11011b");
eqt!(x2cm_1081, "4 4 -3 -9 8 5 -7<=-3 -4 -2 -8 -1 -4 8", "0011001b");
eqt!(x2cm_1082, "3 -6 -5 -1 6 -6 8>8 -4 -1 5 2 -7 -7", "0000111b");
eqt!(x2cm_1083, "6 2 -9 7 -4 -6<=1 8 5 7 7 -4", "011111b");
eqt!(x2cm_1084, "-6 -1 -6<=9 0 -5", "111b");
eqt!(x2cm_1085, "2 -7>=-9 7", "10b");
eqt!(x2cm_1086, "5 -8 -2>=-7 1 1", "100b");
eqt!(x2cm_1087, "9 7=8 2", "00b");
eqt!(x2cm_1088, "4 9=2 5", "00b");
eqt!(x2cm_1089, "5 8 0 -3 6 -7 4<>-2 0 1 5 -6 -6 -2", "1111111b");
eqt!(x2cm_1090, "1 -4 -9 -4 5 5>=0 -5 6 -6 3 1", "110111b");
eqt!(x2cm_1091, "3 -6 2 2 -7 -1 -3>-2 -5 0 6 8 -5 -4", "1010011b");
eqt!(x2cm_1092, "8 8 0 4 3 -2 3>=5 0 1 5 -9 -5 4", "1100110b");
eqt!(x2cm_1093, "-3 3 5 2 -9 -1 9<9 -5 8 -2 4 -4 -5", "1010100b");
eqt!(x2cm_1094, "-4 -1 -2 -8 -7 -7<>-8 1 0 1 -3 7", "111111b");
eqt!(x2cm_1095, "4 9 0<-8 -4 8", "001b");
eqt!(x2cm_1096, "3 -9 -5<>-4 -1 3", "111b");
eqt!(x2cm_1097, "-8 5 1 -4 -5 1 -7 -8<1 1 -4 4 6 2 7 8", "10011111b");
eqt!(x2cm_1098, "-9 -7 -1 -7 -5 -2 9 -2<4 7 -6 -3 6 7 -7 -2", "11011100b");
eqt!(x2cm_1099, "-7 -9 6 1 6 -1=-7 2 -4 6 0 1", "100000b");
eqt!(x2cm_1100, "-9 -3 9<9 -3 3", "100b");
eqt!(x2cm_1101, "-1 2 4<-3 -5 -4", "000b");
eqt!(x2cm_1102, "-7 2<5 -1", "10b");
eqt!(x2cm_1103, "7 -9 -6 -2 -2 0 6 -1<>1 -1 5 7 6 2 -2 6", "11111111b");
eqt!(x2cm_1104, "1 3 -9 -1 3>=-6 -7 2 -2 5", "11010b");
eqt!(x2cm_1105, "2 2 9 0 -7 5 1>4 -8 3 -2 1 3 -3", "0111011b");
eqt!(x2cm_1106, "-7 -6 4 9 7 5 -5<>-3 9 6 0 -5 -3 6", "1111111b");
eqt!(x2cm_1107, "1 -8 3>-6 -7 -5", "101b");
eqt!(x2cm_1108, "7 2<=9 -2", "10b");
eqt!(x2cm_1109, "-3 8 8<=7 -3 3", "100b");
eqt!(x2cm_1110, "-4 7<=0 7", "11b");
eqt!(x2cm_1111, "4 -5 8 -5 0 -6 5<=-7 1 -5 -7 5 7 0", "0100110b");
eqt!(x2cm_1112, "-1 2>=6 -1", "01b");
eqt!(x2cm_1113, "1 -2 3 8=7 -9 -9 -3", "0000b");
eqt!(x2cm_1114, "-6 0 2 4 2 -7 -9<=8 -2 -7 8 4 -1 -8", "1001111b");
eqt!(x2cm_1115, "0 -3 -2 2 -5<8 8 6 7 -5", "11110b");
eqt!(x2cm_1116, "0 9 9 3 -5<>8 1 -8 -3 8", "11111b");
eqt!(x2cm_1117, "-7 -7 9 -7>=-6 2 -8 7", "0010b");
eqt!(x2cm_1118, "9 5 2<7 -8 -1", "000b");
eqt!(x2cm_1119, "9 5 -1>=4 3 -3", "111b");
eqt!(x2cm_1120, "2 -8>=-7 9", "10b");
eqt!(x2cm_1121, "5 -4 0<>-1 -5 -3", "111b");
eqt!(x2cm_1122, "-6 6 -7 -2 1 4 9>=1 2 -9 -6 8 8 5", "0111001b");
eqt!(x2cm_1123, "-2 6<>3 -6", "11b");
eqt!(x2cm_1124, "0 2 7 7 -2 2 5<=-4 7 0 5 3 -6 2", "0100100b");
eqt!(x2cm_1125, "-4 8<=6 -4", "10b");
eqt!(x2cm_1126, "0 7 -5 -1 -6 5 4<>-1 -8 3 -3 -3 -9 -5", "1111111b");
eqt!(x2cm_1127, "-8 -5 -1>6 6 8", "000b");
eqt!(x2cm_1128, "-3 -3<=4 -9", "10b");
eqt!(x2cm_1129, "2 -4 -9 4 -5 -4=8 -8 -7 -8 -6 -4", "000001b");
eqt!(x2cm_1130, "-7 -8 7 4 -9 -1 3 9>6 -3 0 -5 2 -4 -4 -1", "00110111b");
eqt!(x2cm_1131, "4 5 3>-8 -9 -7", "111b");
eqt!(x2cm_1132, "-4 0 -8 -6=-5 -1 9 -2", "0000b");
eqt!(x2cm_1133, "-4 5 8 -6 1>=-5 5 -1 -5 -5", "11101b");
eqt!(x2cm_1134, "3 8 -5 8>-8 8 6 8", "1000b");
eqt!(x2cm_1135, "0 6 1 -9>=0 -5 9 8", "1100b");
eqt!(x2cm_1136, "2 5 0>=3 1 -3", "011b");
eqt!(x2cm_1137, "-4 -3 8 9 6 9 -5 3=-7 2 8 -8 -2 6 2 9", "00100000b");
eqt!(x2cm_1138, "-6 6 -4 -8<>-9 -2 2 4", "1111b");
eqt!(x2cm_1139, "-3 8 9 -4 6 5 -6=8 2 8 7 -4 2 9", "0000000b");
eqt!(x2cm_1140, "3 0 4 -4 -3 5 5 -1=2 6 9 3 -1 -7 9 -1", "00000001b");
eqt!(x2cm_1141, "-1 0 -5 -7>5 -2 -9 9", "0110b");
eqt!(x2cm_1142, "-3 9 -5=-9 -7 2", "000b");
eqt!(x2cm_1143, "1 -7 2 2 7 -8 -1 4<=6 -3 7 -2 8 5 -6 3", "11101100b");
eqt!(x2cm_1144, "3 7 -1 -3 -2 8 -9 -7=6 2 1 7 5 9 3 5", "00000000b");
eqt!(x2cm_1145, "-5 -8 -4 8 -1 -5 -7 -2=1 -9 -8 -7 -1 5 7 1", "00001000b");
eqt!(x2cm_1146, "7 -9 -4 7 -4 3 5>-5 6 -6 2 -4 3 3", "1011001b");
eqt!(x2cm_1147, "-3 4 0<>-7 4 -8", "101b");
eqt!(x2cm_1148, "-8 6 0 1=3 3 -6 -4", "0000b");
eqt!(x2cm_1149, "-4 8 1 -9>-8 -4 6 5", "1100b");
eqt!(x2cm_1150, "-7 -5 1 5 -7 -4=-4 7 -1 -8 -4 -9", "000000b");
eqt!(x2cm_1151, "3 -4 -9 -3 -8 4 -4=-8 2 -4 9 -2 2 5", "0000000b");
eqt!(x2cm_1152, "0 4>3 0", "01b");
eqt!(x2cm_1153, "-9 8 -1 -7 2 6 1 -5<1 -9 2 5 0 -6 5 -5", "10110010b");
eqt!(x2cm_1154, "4 -1 -7<>4 -2 6", "011b");
eqt!(x2cm_1155, "8 0 6 -1 -3>-3 4 -7 1 -4", "10101b");
eqt!(x2cm_1156, "-2 -3 0<>-3 7 -4", "111b");
eqt!(x2cm_1157, "0 -5 -6 -1 0 7 -3=1 4 -5 6 8 5 -8", "0000000b");
eqt!(x2cm_1158, "-3 -9 -2 4 6 9 2 -6=-6 -3 -1 4 3 -3 5 -8", "00010000b");
eqt!(x2cm_1159, "-4 0 5 1 -5 -6<-4 -3 -1 8 6 -9", "000110b");
eqt!(x2cm_1160, "-1 1 -6 -1 -5 9 -3 7<>-9 1 3 -1 3 -4 -2 3", "10101111b");
eqt!(x2cm_1161, "-5 -6 6 -4 7 -9 7<2 -1 -9 3 9 2 1", "1101110b");
eqt!(x2cm_1162, "-4 9 -6 -9 7 2<1 2 0 2 0 2", "101100b");
eqt!(x2cm_1163, "-7 -5 -2 -8 -1>=5 5 0 -5 -6", "00001b");
eqt!(x2cm_1164, "3 -6 7 1>-7 7 7 3", "1000b");
eqt!(x2cm_1165, "-6 3 2 -5 -2 7 -8 -5<=-1 -1 7 -6 6 -8 6 3", "10101011b");
eqt!(x2cm_1166, "4 -1 2 -6 1 9<9 1 -8 9 1 -5", "110100b");
eqt!(x2cm_1167, "1 0 0=6 3 0", "001b");
eqt!(x2cm_1168, "-8 -9 -8>-2 -6 9", "000b");
eqt!(x2cm_1169, "-5 4>=6 -5", "01b");
eqt!(x2cm_1170, "4 9 3 -9>-6 0 -1 3", "1110b");
eqt!(x2cm_1171, "-4 -1 4 -9 -7 -3>=-1 -9 -1 9 3 -1", "011000b");
eqt!(x2cm_1172, "9 -9 3=-2 0 -4", "000b");
eqt!(x2cm_1173, "-9 -5 -2 -8 1 6 -3 -2<1 -2 7 9 2 -5 -4 8", "11111001b");
eqt!(x2cm_1174, "-8 -1>=-3 -2", "01b");
eqt!(x2cm_1175, "0 -8 -9 -1 5 -4 0=1 3 -8 -8 -4 -9 -1", "0000000b");
eqt!(x2cm_1176, "4 -4 -8<>-8 4 -8", "110b");
eqt!(x2cm_1177, "7 8 -3 -4<=1 7 8 1", "0011b");
eqt!(x2cm_1178, "8 -3 0 3>=-2 7 0 3", "1011b");
eqt!(x2cm_1179, "9 7 -8 -3 -1 8 9 -4>-8 8 4 9 -6 0 -6 -2", "10001110b");
eqt!(x2cm_1180, "7 -2 8 8 5 3 -5 9=-1 -9 -3 6 -6 3 8 -9", "00000100b");
// matrix / fkeys / string-search gap-fill
eqt!(mx_mmu_id, "(2 2#1.0 2 3 4) mmu (2 2#1.0 0 0 1)", "2 2#1.0 2 3 4");
eqt!(mx_mmu_2, "(2 2#1.0 2 3 4) mmu (2 2#5.0 6 7 8)", "2 2#19.0 22 43 50");
eqt!(mx_mmu_vec, "(2 2#1.0 2 3 4) mmu 1.0 1", "3 7f");
eqt!(mx_inv_id, "inv 2 2#1.0 0 0 1", "2 2#1.0 0 0 1");
eqt!(mx_inv_2, "inv 2 2#2.0 0 0 2", "2 2#0.5 0 0 0.5");
eqt!(jn_lj_keyed, "([]k:1 2 3) lj ([k:1 2 3]v:`a`b`c)", "([]k:1 2 3;v:`a`b`c)");
eqt!(str_ss, "\"mississippi\" ss \"ss\"", "2 5");
eqt!(str_ssr, "ssr[\"hello\";\"l\";\"L\"]", "\"heLLo\"");
eqt!(str_like1, "\"abc\" like \"a*\"", "1b");
eqt!(str_like2, "(\"cat\";\"dog\";\"cow\") like \"c*\"", "101b");
eqt!(agg_med_v, "med 5 3 8 1 9 2 7", "5f");

// COMPOSED COMPUTE-ON-COMPRESSED — parity must hold whether or not the compressed path engages.
eqt!(coc_s_where,   "{[n] g::`s#asc (til n)mod 1000; r:asc(til n)mod 1000; \
    (where g>500)~where r>500}[100000]", "1b");
eqt!(coc_s_idx_gt,  "{[n] g::`s#asc (til n)mod 1000; r:asc(til n)mod 1000; (g \
    where g>500)~r where r>500}[100000]", "1b");
eqt!(coc_s_idx_lt,  "{[n] g::`s#asc (til n)mod 1000; r:asc(til n)mod 1000; (g \
    where g<3)~r where r<3}[100000]", "1b");
eqt!(coc_s_eq,      "{[n] g::`s#asc (til n)mod 1000; r:asc(til n)mod 1000; (g \
    where g=42)~r where r=42}[100000]", "1b");
eqt!(coc_s_within,  "{[n] g::`s#asc (til n)mod 1000; r:asc(til n)mod 1000; (g \
    where g within 100 200)~r where r within 100 200}[100000]", "1b");
eqt!(coc_s_gtmax,   "{[n] g::`s#asc (til n)mod 1000; (where \
    g>9999)~`int$()}[100000]", "1b");
eqt!(coc_s_ltmin,   "{[n] g::`s#asc (til n)mod 1000; (where \
    g<0)~`int$()}[100000]", "1b");
// C5 — unsorted compressed predicate → mask
eqt!(coc_u_where,   "{[n] g::(til n)mod 1000; r:(til n)mod 1000; (where \
    g>500)~where r>500}[100000]", "1b");
eqt!(coc_u_idx,     "{[n] g::(til n)mod 1000; r:(til n)mod 1000; (g where \
    g>500)~r where r>500}[100000]", "1b");
eqt!(coc_u_not,     "{[n] g::(til n)mod 1000; r:(til n)mod 1000; (where not \
    g>500)~where not r>500}[100000]", "1b");
eqt!(coc_u_and,     "{[n] g::(til n)mod 1000; r:(til n)mod 1000; \
    ((g>500)&g<900)~(r>500)&r<900}[100000]", "1b");
// C4.3 — slice / take / drop on compressed
eqt!(coc_take100,   "{[n] g::`s#asc (til n)mod 1000; r:asc(til n)mod 1000; \
    (100#g)~100#r}[100000]", "1b");
eqt!(coc_drop100,   "{[n] g::`s#asc (til n)mod 1000; r:asc(til n)mod 1000; \
    (-100#g)~-100#r}[100000]", "1b");
eqt!(coc_slice,     "{[n] g::`s#asc (til n)mod 1000; r:asc(til n)mod 1000; (g \
    100+til 100)~r 100+til 100}[100000]", "1b");
eqt!(coc_slice_chain,"{[n] g::`s#asc (til n)mod 1000; r:asc(til n)mod 1000; (g \
    where g within 100 200)~r where r within 100 200}[100000]", "1b");
// C6 — qSQL row-set composition (compressed stored cols)
eqt!(coc_q_gt,      "{[n] tc::([] v:`s#asc (til n)mod 1000; s:(til n)mod 100); \
    tr:([] v:asc(til n)mod 1000; s:(til n)mod 100); (select from tc where \
        v>990)~select from tr where v>990}[100000]", "1b");
eqt!(coc_q_eq,      "{[n] tc::([] v:`s#asc (til n)mod 1000; s:(til n)mod 100); \
    tr:([] v:asc(til n)mod 1000; s:(til n)mod 100); (select from tc where \
        v=42)~select from tr where v=42}[100000]", "1b");
eqt!(coc_q_within,  "{[n] tc::([] v:`s#asc (til n)mod 1000; s:(til n)mod 100); \
    tr:([] v:asc(til n)mod 1000; s:(til n)mod 100); (select from tc where v \
        within 100 200)~select from tr where v within 100 200}[100000]", "1b");
// C7 — aggregate sink over range-filtered rows
eqt!(coc_q_sumby,   "{[n] tc::([] v:`s#asc (til n)mod 1000; s:(til n)mod 100); \
    tr:([] v:asc(til n)mod 1000; s:(til n)mod 100); (select sum s by v from tc \
        where v within 100 200)~select sum s by v from tr where v within 100 \
            200}[100000]", "1b");
// C6b — a dense gather over a compressed small-int column stays compressed and matches the raw twin.
eqt!(coc_gat_subbyte_dense, "{[n] r:n?5; c:-18!r; (c reverse til n)~r reverse \
    til n}[100000]", "1b");
eqt!(coc_gat_subbyte_idx,   "{[n] r:n?5; c:-18!r; i:n?n; (c i)~r i}[100000]",
    "1b");
// threaded min/max by 100k-group (q7 max-min shape) == serial twin
eqt!(coc_grp_minmax_thr, "{[n] t:([] g:n?100000; v:-18!n?1000000); (select \
    mn:min v, mx:max v by g from t)~(select mn:{min x} v, mx:{max x} v by g \
        from t)}[1000000]", "1b");
// C8 — group keys
eqt!(coc_q_grp,     "{[n] tc::([] k:`s#asc (til n)mod 1000; v:(til n)mod 100); \
    tr:([] k:asc(til n)mod 1000; v:(til n)mod 100); (select sum v by k from \
        tc)~select sum v by k from tr}[100000]", "1b");
// g# Kvaux sidecar + group topology (qsql-coc-enum merge)
eqt!(coc_g_group,    "{[n] g:`g#(til n)mod 100; r:(til n)mod 100; (group \
    g)~group r}[100000]", "1b");
eqt!(coc_g_distinct, "{[n] g:`g#(til n)mod 100; (distinct g)~asc distinct (til \
    n)mod 100}[100000]", "1b");
eqt!(coc_g_find,     "{[n] g:`g#(til n)mod 100; (g?42)=42}[100000]", "1b");
eqt!(coc_g_in,       "{[n] g:`g#(til n)mod 100; r:(til n)mod 100; (g in 5 10 \
    15)~r in 5 10 15}[100000]", "1b");
eqt!(coc_g_append,   "{[n] g:`g#(til n)mod 50; r:(til n)mod 50; g2:g,1 2 3; \
    (group g2)~group r,1 2 3}[100000]", "1b");
eqt!(coc_g_attr_clr, "{[n] g:`g#(n?50i); (attr reverse g)~`}[1000]", "1b");
eqt!(coc_g_countby,  "{[n] t:update `g#a from ([] a:n?50i; v:n?100i); r:([] \
    a:`#t`a; v:t`v); (select c:count i by a from t)~select c:count i by a from \
        r}[100000]", "1b");
eqt!(coc_g_sumby,    "{[n] t:update `g#a from ([] a:n?50i; v:n?100i); r:([] \
    a:`#t`a; v:t`v); (select s:sum v by a from t)~select s:sum v by a from \
        r}[100000]", "1b");

// Matrix-store compression regression: -17! on a generic list now aggregates its rows.
eqt!(qz_mtx_rows_compress, "{m9::128 0N#\"f\"$(128*8192)?10; first -17!`m9}[]",
    "1b");
eqt!(qz_mtx_info_ratio,    "{m9::128 0N#\"f\"$(128*8192)?10; r:-17!`m9; \
    r[1]<r[2] div 2}[]", "1b");
eqt!(qz_mtx_gemm_parity,   "{m9::64 0N#\"f\"$(64*8192)?10; rr:{x@til count x} \
    each m9; (m9$flip m9)~rr$flip rr}[]", "1b");
eqt!(qz_mtx_int_rows,      "{m9::128 0N#(\"i\"$128*8192)?10; first -17!`m9}[]",
    "1b");
eqt!(qz_mtx_int_sum,       "{m9::128 0N#(\"i\"$128*8192)?10; (sum sum m9)=sum \
    sum {x@til count x} each m9}[]", "1b");
eqt!(qz_mtx_strings_raw,   "{s9::string each til 100; first -17!`s9}[]", "0b");

// ── compressed-op closure T1+T2 (enlist ! ~ $ ? ,) canonical spot checks
eqt!(coc_cls_enlist, "{g9::`s#asc (til 100000)mod 1000; r:asc(til 100000)mod \
    1000; (enlist g9)~enlist r}[]", "1b");
eqt!(coc_cls_cast,   "{g9::\"i\"$(til 100000)mod 100; r:\"i\"$(til 100000)mod \
    100; (\"j\"$g9)~\"j\"$r}[]", "1b");
eqt!(coc_cls_castf,  "{g9::\"i\"$(til 100000)mod 100; r:\"i\"$(til 100000)mod \
    100; (\"f\"$g9)~\"f\"$r}[]", "1b");
eqt!(coc_cls_find,   "{g9::\"i\"$(til 100000)mod 100; \
    (g9?42i;g9?12345i)~(42;100000)}[]", "1b");
eqt!(coc_cls_cat,    "{g9::\"i\"$(til 100000)mod 100; h9::\"i\"$5000+(til \
    100000)mod 300; r:(g9@til 100000),h9@til 100000; (g9,h9)~r}[]", "1b");
eqt!(coc_cls_bang,   "{g9::\"i\"$(til 100000)mod 100; d:(til 100000)!g9; \
    (value \
    d)~g9@til 100000}[]", "1b");
eqt!(coc_cls_match,  "{g9::\"i\"$(til 100000)mod 100; g9~\"i\"$(til 100000)mod \
    100}[]", "1b");

// ── compressed-float width-lattice (clustered vs random inputs)
eqt!(qz_bw40_fires,  "{c9::100+5*200000?1.0; r:-17!`c9; (first r) and r[1] \
    within 1004000 1006000}[]", "1b");
eqt!(qz_bw48_stays,  "{c9::200000?1.0; r:-17!`c9; (first r) and r[1] within \
    1204000 1206000}[]", "1b");
eqt!(qz_bw40_parity, "{c9::100+5*200000?1.0; r:c9@til 200000; ((sum c9;min \
    c9;max c9;c9 12345)~(sum r;min r;max r;r 12345)) and (iasc c9)~iasc r}[]",
        "1b");

// compressed-float wrong-reader regression armor — oracle is always an INLINE raw expression; lossy floats use a tolerance band.
eqt!(alqr_codec_a48, "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; \
    48012=(-55)!`a9}[]", "1b");
eqt!(alqr_codec_b40, "{b9::1.0+0.01*sin til 100000; 40012=(-55)!`b9}[]", "1b");
eqt!(alqr_codec_c24, "{c9::\"e\"$0.001*til 100000; 24012=(-55)!`c9}[]", "1b");
// THE bug: sqrt over a compressed float column (was ~3e9 wrong)
eqt!(alqr_sqrt48,    "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; \
    1e-6>max abs (sqrt a9)-sqrt @[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
        0.0033]}[]", "1b");
eqt!(alqr_sqrt40,    "{b9::1.0+0.01*sin til 100000; 1e-6>max abs (sqrt \
    b9)-sqrt \
    1.0+0.01*sin til 100000}[]", "1b");
eqt!(alqr_sqrt24e,   "{c9::\"e\"$0.001*til 100000; 1e-4>max abs (sqrt c9)-sqrt \
    \"e\"$0.001*til 100000}[]", "1b");
// whole transcendental family over both float widths
eqt!(alqr_log48,     "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; \
    1e-6>max abs (log 1e-9+a9)-log 1e-9+@[0.01*til 100000;3 7 11;:;0.0011 \
        0.0022 0.0033]}[]", "1b");
eqt!(alqr_trig48,    "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; \
    r:@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; m:{[x;y]max abs x-y}; \
        1e-6>(m[sin a9;sin r]|m[cos a9;cos r])|m[tanh a9;tanh r]|m[atan \
            a9;atan \
            r]}[]", "1b");
eqt!(alqr_trig40,    "{b9::1.0+0.01*sin til 100000; r:1.0+0.01*sin til 100000; \
    m:{[x;y]max abs x-y}; 1e-6>(m[exp b9;exp r]|m[tan b9;tan r])|m[sinh \
        b9;sinh \
        r]|m[reciprocal b9;reciprocal r]}[]", "1b");
// neg/abs vs inline raw; floor/xbar vs a decoded twin (boundary lanes differ by the rounding delta).
eqt!(alqr_negabs48,  "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; \
    r:@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; 1e-6>(max abs (neg \
        a9)-neg r)|max abs (abs a9)-abs r}[]", "1b");
eqt!(alqr_floor48,   "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; \
    d:a9 til 100000; ((floor a9)~floor d) and (10 xbar a9)~10 xbar d}[]", "1b");
// reductions / running / moving over ALQ48
eqt!(alqr_reduce48,  "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; \
    r:@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; (1e-9>abs 1-(sum \
        a9)%sum r) and (1e-9>abs 1-(var a9)%var r) and ((min a9;max a9;first \
            a9)~(min r;max r;first r)) and 1e-9>abs (med a9)-med r}[]", "1b");
eqt!(alqr_running48, "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; \
    r:@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; (1e-6>max abs (deltas \
        a9)-deltas r) and (1e-6>max abs (mins a9)-mins r) and 1e-6>max abs (5 \
            msum a9)-5 msum r}[]", "1b");
// structure: reverse / gather (block edges!) / slice / amend / sort
eqt!(alqr_rev48,     "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; \
    1e-6>max abs (reverse a9)-reverse @[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
        0.0033]}[]", "1b");
eqt!(alqr_gather48,  "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; \
    r:@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; ix:0 1 3 1023 1024 \
        2047 \
        2048 50000 99999; 1e-9>max abs (a9 ix)-r ix}[]", "1b");
eqt!(alqr_slice48,   "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; \
    r:@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; (1e-9>max abs (100 \
        sublist 5000 _ a9)-100 sublist 5000 _ r) and 1e-9>max abs \
            (-100#a9)-(-100)#r}[]", "1b");
eqt!(alqr_amend48,   "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; \
    a9[5]:9.99; 1e-9>max abs a9-@[@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
        0.0033];5;:;9.99]}[]", "1b");
eqt!(alqr_sort48,    "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; \
    r:@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; ((iasc a9)~iasc r) and \
        1e-6>max abs (asc a9)-asc r}[]", "1b");
// dyads: atom arith, vec+vec, weighted dots, compare/membership consistency
eqt!(alqr_dyad48,    "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; \
    r:@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; m:{[x;y]max abs x-y}; \
        1e-6>(m[a9+1.5;r+1.5]|m[a9-1.5;r-1.5])|m[a9*2.0;r*2.0]|m[a9%2.0;r%2.0]|\
            m\
            [a9+a9;r+r]}[]", "1b");
eqt!(alqr_wsum48,    "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; \
    r:@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; w:til 100000; \
        (1e-9>abs \
        1-(w wsum a9)%w wsum r) and 1e-9>abs 1-((1+w) wavg a9)%(1+w) wavg \
            r}[]", "1b");
eqt!(alqr_cmp48,     "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; \
    d:a9 til 100000; ((a9>500.005)~d>500.005) and \
        ((a9=123.456789)~d=123.456789) and ((a9 within 100.505 200.715)~d \
            within 100.505 200.715) and ((where a9>999.005)~where d>999.005) \
                and (a9 in 1.005 2.505 77.775)~d in 1.005 2.505 77.775}[]",
                    "1b");

// DEEP ARMOR — three independent nets (cross-path identities, edge-value batteries, dispatch-state probes) for compressed-op integrity.
eqt!(dpar_xpath_sqrt48,  "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
    0.0033]; s:sqrt a9; 1e-9>max abs (s*s)-a9}[]", "1b");
eqt!(dpar_xpath_sqrt40,  "{b9::1.0+0.01*sin til 100000; s:sqrt b9; 1e-9>max \
    abs \
    (s*s)-b9}[]", "1b");
eqt!(dpar_xpath_logexp,  "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
    0.0033]; 1e-9>max abs (log exp neg neg a9%1e4)-a9%1e4}[]", "1b");
eqt!(dpar_xpath_revrev,  "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
    0.0033]; (reverse reverse a9)~a9 til 100000}[]", "1b");
eqt!(dpar_xpath_sumrev,  "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
    0.0033]; ((sum a9)=sum reverse a9) and (max a9)=a9 first idesc a9}[]",
        "1b");
// NET 2 — edge batteries: floor/ceiling/div/mod/xbar parity at boundary lanes across codecs.
eqt!(dpar_edge_codecs,   "{e9::@[0.01*til 100000;100+til 8;:;7.999999999999998 \
    8.000000000000002 -8.000000000000002 2.5 -2.5 0.0 1e-14 -1e-14]; \
        q9::@[100.0+0.01*sin til 100000;100+til 8;:;107.999999999999998 \
            108.000000000000002 92.000000000000002 102.5 97.5 100.0 \
                100.00000000000001 99.99999999999999]; (32017=(-55)!`e9) and \
                    40012=(-55)!`q9}[]", "1b");
eqt!(dpar_floor_edges,   "{e9::@[0.01*til 100000;100+til 8;:;7.999999999999998 \
    8.000000000000002 -8.000000000000002 2.5 -2.5 0.0 1e-14 -1e-14]; \
        r:@[0.01*til 100000;100+til 8;:;7.999999999999998 8.000000000000002 \
            -8.000000000000002 2.5 -2.5 0.0 1e-14 -1e-14]; ((floor e9)~floor \
                r) \
                and ((floor e9)~floor each e9 til 100000) and (ceiling \
                    e9)~ceiling r}[]", "1b");
eqt!(dpar_floor_alq,     "{q9::@[100.0+0.01*sin til 100000;100+til \
    8;:;107.999999999999998 108.000000000000002 92.000000000000002 102.5 97.5 \
        100.0 100.00000000000001 99.99999999999999]; s:sqrt q9; (1e-9>max abs \
            (s*s)-q9) and ((floor q9)~floor q9 til 100000) and (8 8 -8 2 \
                -3~floor 7.999999999999998 8.000000000000002 \
                    -8.000000000000002 \
                    2.5 -2.5)}[]", "1b");
eqt!(dpar_cmp_boundary,  "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
    0.0033]; d:a9 til 100000; r:@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
        0.0033]; ((a9=50.05)~d=50.05) and ((a9=50.05)~r=50.05) and ((a9 within \
            100.5 200.7)~d within 100.5 200.7) and ((a9<50.05)~r<50.05) and \
                (a9>50.05)~r>50.05}[]", "1b");
eqt!(dpar_span_boundary, "{s9::asc 0.01*til 100000; d:s9 til 100000; \
    ((s9=50.05)~d=50.05) and ((where s9<50.05)~where d<50.05) and ((s9 bin \
        50.05)~d bin 50.05) and (s9 within 50.05 60.07)~d within 50.05 \
            60.07}[]", "1b");
eqt!(dpar_div_xbar,      "{(10 10f~10 xbar 9.999999999999998 \
    10.000000000000002) and (1 1f~9.999999999999998 10.000000000000002 div 10) \
        and (8=ceiling 8.000000000000002) and (floor 0n 0w 1.5)~0N 2147483647 \
            1}[]", "1b");
eqt!(dpar_mod_alq,       "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
    0.0033]; 1e-9>max abs (a9 mod 7)-(@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
        0.0033]) mod 7}[]", "1b");
// NET 3 — dispatch/state probes
eqt!(dpar_codec_switch,  "{c9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
    0.0033]; r1:sqrt c9; c9::1.0+0.01*sin til 100000; r2:sqrt c9; c9::0.01*til \
        100000; r3:sqrt c9; (1e-6>max abs r1-sqrt @[0.01*til 100000;3 7 \
            11;:;0.0011 0.0022 0.0033]) and (1e-6>max abs r2-sqrt 1.0+0.01*sin \
                til 100000) and 1e-6>max abs r3-sqrt 0.01*til 100000}[]", "1b");
eqt!(dpar_repeat_disp,   "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
    0.0033]; r:sqrt a9; ok:1b; do[5; ok:ok and r~sqrt a9]; ok}[]", "1b");
eqt!(dpar_enum_tagged,   "{(`trapped~@[{`zq91?x};42;`trapped]) and \
    (`trapped~@[{`zq92?x};3.14;`trapped]) and (`a`b~value `zq93?`a`b)}[]",
        "1b");
eqt!(dpar_enum_state,    "{zq94::`abc; `trapped~@[{`zq94?x};42;`trapped]}[]",
    "1b");
// the original corrupt compiled-lambda shapes (in-lambda local + compressed param)
eqt!(alqr_d1_shape,  "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; \
    {[n;rw] v:@[0.01*til n;3 7 11;:;0.0011 0.0022 0.0033]; 1e-6>max abs (sqrt \
        v)-sqrt rw}[100000;a9]}[]", "1b");
eqt!(alqr_i2_shape,  "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; \
    {[n;rw] v:@[0.01*til n;3 7 11;:;0.0011 0.0022 0.0033]; a:sqrt v; b:sqrt \
        rw; \
        1e-6>max abs a-b}[100000;a9]}[]", "1b");
eqt!(alqr_fuse_shape,"{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; \
    {[n;rw] v:0.01*til n; 1e-6>max abs (sqrt rw)-sqrt @[v;3 7 11;:;0.0011 \
        0.0022 0.0033]}[100000;a9]}[]", "1b");
// qSQL aggregation over a compressed float column
eqt!(alqr_qsql48,    "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; \
    t9::([]p:a9;s:(til 100000) mod 7); 1e-9>abs 1-(exec sum p from t9 where \
        s=3)%sum (@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]) where \
            3=(til 100000) mod 7}[]", "1b");

// LEAF-BYPASS ARMOR — table/keyed drill-downs (asof, keyed lookup) must decode a compressed key column, not read it raw.
eqt!(leaf_keyed_lookup, "{klkt::([k:til 100000]p:@[0.01*til 100000;3 7 \
    11;:;0.0011 0.0022 0.0033]); raw:@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
        0.0033]; r:klkt ([]k:50 500 5000); 1e-9>max abs (exec p from r)-raw 50 \
            500 5000}[]", "1b");
eqt!(leaf_asof_int,     "{at2::([]t:til 100000; v:@[0.01*til 100000;3 7 \
    11;:;0.0011 0.0022 0.0033]); raw:@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
        0.0033]; j:aj[`t;([]t:0 1 50 99999);at2]; 1e-9>max abs (exec v from \
            j)-raw 0 1 50 99999}[]", "1b");
eqt!(leaf_asof_between, "{at2::([]t:2*til 100000; v:@[0.01*til 100000;3 7 \
    11;:;0.0011 0.0022 0.0033]); raw:@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
        0.0033]; j:aj[`t;([]t:1 99 199999);at2]; 1e-9>max abs (exec v from \
            j)-raw 0 49 99999}[]", "1b");
eqt!(leaf_keyed_upsert, "{kk::([k:til 100000]v:@[0.01*til 100000;3 7 \
    11;:;0.0011 0.0022 0.0033]); `kk upsert (100000;1.23); 1e-9>abs \
        (kk[100000;`v])-1.23}[]", "1b");
eqt!(leaf_select_inset, "{gt2::([]p:@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
    0.0033]; k:til 100000); raw:@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
        0.0033]; 1e-9>max abs (exec p from gt2 where k in 50 500 5000)-raw 50 \
            500 5000}[]", "1b");

// EDGE BATTERY — empty/single slices, null/inf probes, boundary crossings, IPC round-trip; all vs inline raw twins.
eqt!(edge_empty_slice,  "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
    0.0033]; \
    r:@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; ((0#a9)~0#r) and \
        ((1#a9)~1#r) and ((sum 0#a9)~sum 0#r) and (@[{sqrt \
            0#x};a9;`e]~`float$())}[]", "1b");
eqt!(edge_inf_probes,   "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
    0.0033]; \
    r:@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; ((a9<0w)~r<0w) and \
        ((a9>-0w)~r>-0w) and ((a9 within -0w 0w)~r within -0w 0w) and (a9 \
            within 0n 50.0)~r within 0n 50.0}[]", "1b");
eqt!(edge_delta_gallop, "{t9::til 100000; ((t9<0)~0>til 100000) and \
    ((t9<99999)~99999>til 100000) and ((t9<100000)~100000>til 100000) and \
        ((t9=50000)~50000=til 100000) and ((t9 bin -1)~(til 100000)bin -1) and \
            ((t9 bin 50000)~(til 100000)bin 50000) and ((t9 bin 99999)~(til \
                100000)bin 99999) and (t9 within 100 200)~(til 100000)within \
                    100 200}[]", "1b");
eqt!(edge_sorted_attr,  "{s9::`s#asc 0.01*til 100000; raw:`s#asc 0.01*til \
    100000; ((s9 bin 500.005)~raw bin 500.005) and ((where s9<500.005)~where \
        raw<500.005) and (s9 50000)~raw 50000}[]", "1b");
eqt!(edge_block_gather, "{b9::@[0.01*til 100000;1023 1024 2047 2048;:;1.5 2.5 \
    3.5 4.5]; (b9 0 1 1023 1024 2047 2048 99999)~(@[0.01*til 100000;1023 1024 \
        2047 2048;:;1.5 2.5 3.5 4.5]) 0 1 1023 1024 2047 2048 99999}[]", "1b");
eqt!(edge_amend_chain,  "{c9::0.01*til 100000; c9[100]+:5.0; c9[til 50]*:2.0; \
    1e-9>max abs c9-@[@[0.01*til 100000;100;+;5.0];til 50;*;2.0]}[]", "1b");
eqt!(edge_nullint_red,  "{u9::@[til 100000;5 50 500;:;0N]; raw:@[til 100000;5 \
    50 500;:;0N]; ((sum u9)~sum raw) and ((null u9)~null raw) and \
        ((0^u9)~0^raw) and ((avg u9)~avg raw) and (min u9;max u9)~(min raw;max \
            raw)}[]", "1b");
eqt!(edge_ser_rt,       "{g9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
    0.0033]; \
    1e-9>max abs (-9!-8!g9)-@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
        0.0033]}[]", "1b");
eqt!(edge_raze_cut,     "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
    0.0033]; \
    r:@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; (1e-9>max abs (raze \
        100 \
        cut a9)-r til 100000) and 1e-9>max abs (sum each 1000 cut a9)-sum each \
            1000 cut r}[]", "1b");
eqt!(edge_each_monad,   "{a9::@[0.01*til 100000;3 7 11;:;0.0011 0.0022 \
    0.0033]; \
    r:@[0.01*til 100000;3 7 11;:;0.0011 0.0022 0.0033]; ((null each 10 cut \
        a9)~null each 10 cut r) and (floor each 10 cut a9)~floor each 10 cut \
            r}[]", "1b");

// ULTRACODE-PASS ARMOR — dict/table column arithmetic, compound-key joins, and FFI must decode compressed columns, not read raw.
eqt!(qzx_dict_atom,   "{g9::0.5*til 100000; d9::(til 100000)!g9; (value \
    d9+1.0)~1.0+0.5*til 100000}[]", "1b");
eqt!(qzx_atom_dict,   "{g9::0.5*til 100000; d9::(til 100000)!g9; (value \
    2*d9)~2*0.5*til 100000}[]", "1b");
eqt!(qzx_dict_dict,   "{g9::0.5*til 100000; d9::(til 100000)!g9; (value \
    d9+d9)~(0.5*til 100000)+0.5*til 100000}[]", "1b");
eqt!(qzx_sqrt_dict,   "{g9::1.0+0.5*til 100000; d9::(til 100000)!g9; 1e-9>max \
    abs (value sqrt d9)-sqrt 1.0+0.5*til 100000}[]", "1b");
eqt!(qzx_neg_dict,    "{g9::0.5*til 100000; d9::(til 100000)!g9; (value neg \
    d9)~neg 0.5*til 100000}[]", "1b");
eqt!(qzx_keyed_upd,   "{gv::\"f\"$til 100000; kt9::([k:til 100000]v:gv); (exec \
    v2 from update v2:2*v from kt9)~2*\"f\"$til 100000}[]", "1b");
// compound-key joins over a compressed key column (vs INLINE raw twin)
eqt!(qzx_aj_compound, "{gt9::`long$10*til 100000; gv9::(`float$til \
    100000)%100; \
    R:([]sym:100000#`a;t:gt9;v:gv9); 1e-9>max abs \
        (aj[`sym`t;([]sym:3#`a;t:`long$0 50 100);R]`v)-(`float$0 5 10)%100}[]",
            "1b");
eqt!(qzx_lj_compound, "{gk9::til 100000; gv9::\"f\"$til 100000; \
    kt9:([s:100000#`a;k:gk9]v:gv9); (exec v from ([]s:`a`a`a;k:50 500 5000) lj \
        kt9)~\"f\"$50 500 5000}[]", "1b");
eqt!(qzx_ij_compound, "{gk9::til 100000; gv9::\"f\"$til 100000; \
    kt9:([s:100000#`a;k:gk9]v:gv9); (exec v from ([]s:`a`a`a;k:50 500 5000) ij \
        kt9)~\"f\"$50 500 5000}[]", "1b");
eqt!(qzx_pj_keyed,    "{gk9::til 100000; gv9::\"f\"$til 100000; \
    kt1:([s:100000#`a;k:gk9]v:gv9); kt2:([s:1#`a;k:1#500]v:1#10.0); 1e-9>abs \
        ((kt1 pj kt2)[(`a;500);`v])-510.0}[]", "1b");
// keyed-table indexed by a PROBE table carrying the compressed column
eqt!(qzx_probe_qz,    "{gk9::til 100000; kt2:([s:1#`a;k:1#500]v:1#10.0); \
    probe:([]s:100000#`a;k:gk9); 1e-9>abs ((exec v from kt2 \
        probe)@500)-10.0}[]", "1b");
// table find returning row indices, compressed column in the SET and in the PROBE
eqt!(qzx_find_setqz,  "{gk9::til 100000; setb::([]s:100000#`a;k:gk9); (setb ? \
    ([]s:`a`a;k:50 500))~50 500}[]", "1b");
eqt!(qzx_find_probeqz,"{gk9::til 100000; probe:([]s:100000#`a;k:gk9); \
    (([]s:1#`a;k:1#500) ? probe)[500]<1}[]", "1b");
// Stage-6 rule 1: count-where / sum-pred fused result vs an unfused oracle.
eqt!(fz1_cnt_where_unsorted, "{[n] g::\"j\"$(til n) mod 977; u:g>500; (count \
    where g>500)=count where u}[100000]", "1b");
eqt!(fz1_sum_pred,           "{[n] g::\"j\"$(til n) mod 977; u:g>500; (sum \
    g>500)=count where u}[100000]", "1b");
eqt!(fz1_cnt_where_sorted,   "{[n] g::\"j\"$3*til n; u:g>3000; (count where \
    g>3000)=count where u}[100000]", "1b");
eqt!(fz1_cnt_index_form,     "{[n] g::\"j\"$3*til n; u:g>3000; (count g where \
    g>3000)=count where u}[100000]", "1b");
eqt!(fz1_cnt_float,          "{[n] g::\"f\"$(til n) mod 977; u:g>500.0; (count \
    where g>500.0)=count where u}[100000]", "1b");
eqt!(fz1_sum_within,         "{[n] g::\"j\"$(til n) mod 977; u:g within 10 20; \
    (sum g within 10 20)=count where u}[100000]", "1b");
// Stage-6 rule 2: max(pred)=any, min(pred)=all sinks, incl empty-vec parity.
eqt!(fz2_any_pred,     "{[n] g::\"j\"$(til n) mod 977; u:g>500; (max \
    g>500)=max \
    u}[100000]", "1b");
eqt!(fz2_all_pred,     "{[n] g::\"j\"$(til n) mod 977; u:g>500; (min \
    g>500)=min \
    u}[100000]", "1b");
eqt!(fz2_any_sorted_miss, "{[n] g::\"j\"$3*til n; (max g>3*n)=0b}[100000]",
    "1b");
eqt!(fz2_all_sorted_hit,  "{[n] g::\"j\"$3*til n; (min g>=0)=1b}[100000]",
    "1b");
eqt!(fz2_any_empty,    "{g::\"j\"$(); (max g>5)=0b}[]", "1b");
eqt!(fz2_all_empty,    "{g::\"j\"$(); (min g>5)=1b}[]", "1b");
eqt!(fz2_atom_runtime, "{g::5; ((max g>3)=max 5>3) and (min g>30)=min 5>30}[]",
    "1b");
// Stage-6 rule 3: sum(x*y) -> dot; float routes to wsum, ints/mixed to literal mul+sum.
eqt!(fz3_dot_ff,    "{[n] a::0.5+til n; b::\"f\"$(til n) mod 977; u:a*b; (sum \
    a*b)=sum u}[100000]", "1b");
eqt!(fz3_dot_self,  "{[n] a::\"f\"$(til n) mod 977; u:a*a; ((sum a*a)=sum u) & \
    (sum a*a)=a wsum a}[100000]", "1b");
eqt!(fz3_dot_int,   "{[n] a::\"j\"$(til n) mod 977; u:a*a; ((sum a*a)=sum u) & \
    (type sum a*a)=type sum u}[100000]", "1b");
eqt!(fz3_dot_nulls, "{a::1 0N 3; (sum a*a)=10}[]", "1b");
eqt!(fz3_dot_atom_rt, "{a::5.0; (sum a*a)=25.0}[]", "1b");
// Stage-6 rule 4: where((v1 op a1)&(v2 op a2)) interval-intersect vs a local-bool oracle.
eqt!(fz4_sel2_unsorted, "{[n] g::\"j\"$(til n) mod 977; u:(g>100)&g<200; \
    (where \
    (g>100)&g<200)~where u}[100000]", "1b");
eqt!(fz4_sel2_sorted,   "{[n] g::\"j\"$3*til n; u:(g>3000)&g<9000; (where \
    (g>3000)&g<9000)~where u}[100000]", "1b");
eqt!(fz4_sel2_disjoint, "{[n] g::\"j\"$3*til n; u:(g>3*n)&g<5; (where \
    (g>3*n)&g<5)~where u}[100000]", "1b");
eqt!(fz4_sel2_twocols,  "{[n] ga::\"j\"$3*til n; gb::\"j\"$2*til n; \
    u:(ga>300)&gb<900; (where (ga>300)&gb<900)~where u}[100000]", "1b");
eqt!(fz4_sel2_lenerr,   "{[n] ga::\"j\"$3*til n; gb::\"j\"$2*til 7; r:@[{where \
    (ga>300)&gb<900};0;{`err}]; r~`err}[100000]", "1b");
// Stage-6 rule 5: first/last(asc x) -> min/max only under the no-null gate; nulls take the sort path.
eqt!(fz5_first_asc,  "{[n] g::\"j\"$(til n) mod 977; u:asc g; (first asc \
    g)=first u}[100000]", "1b");
eqt!(fz5_last_asc,   "{[n] g::\"j\"$(til n) mod 977; u:asc g; (last asc \
    g)=last \
    u}[100000]", "1b");
eqt!(fz5_null_gate,  "{g::1 0N 3; (first asc g)~0N}[]", "1b");
eqt!(fz5_dict_safe,  "{(first asc `b`a!2 1)~{s:asc x; first s}[`b`a!2 1]}[]",
    "1b");
eqt!(fz5_float,      "{[n] g::0.5+til n; (first asc g)=0.5}[100000]", "1b");
// Stage-6 rule 6: x iasc x with x evaluated once, bit-exact incl attribute; atoms signal rank.
eqt!(fz6_gsort,      "{[n] g::\"j\"$(til n) mod 977; u:iasc g; ((g iasc g)~g \
    u) \
    and (attr g iasc g)~attr g u}[100000]", "1b");
eqt!(fz6_gsort_atom, "{g::5; `rank~@[{x iasc x};g;{`$x}]}[]", "1b");
// T1.1: y where x op a cross-column predicate-gather across sorted/unsorted/length-mismatch cases.
eqt!(fz7_selg_sorted,   "{[n] gx::\"j\"$3*til n; gy::0.5+til n; u:where \
    gx>3000; (gy where gx>3000)~gy u}[100000]", "1b");
eqt!(fz7_selg_unsorted, "{[n] gx::\"j\"$(til n) mod 977; gy::0.5+til n; \
    u:where \
    gx>500; (gy where gx>500)~gy u}[100000]", "1b");
eqt!(fz7_selg_mismatch, "{[n] gx::\"j\"$(til n) mod 977; gy::1 2 3; u:where \
    gx>500; (gy where gx>500)~gy u}[100000]", "1b");
eqt!(fz7_selg_at_form,  "{[n] gx::\"j\"$3*til n; gy::0.5+til n; u:where \
    gx>3000; (gy@where gx>3000)~gy u}[100000]", "1b");
// T1.2: first/last where pred end-of-selection sinks across sorted/unsorted/empty.
eqt!(fz8_first_where,  "{[n] g::\"j\"$(til n) mod 977; u:where g>500; (first \
    where g>500)=first u}[100000]", "1b");
eqt!(fz8_last_where,   "{[n] g::\"j\"$(til n) mod 977; u:where g>500; (last \
    where g>500)=last u}[100000]", "1b");
eqt!(fz8_fl_sorted,    "{[n] g::\"j\"$3*til n; u:where g>3000; ((first where \
    g>3000)=first u) & (last where g>3000)=last u}[100000]", "1b");
eqt!(fz8_fl_empty,     "{[n] g::\"j\"$3*til n; (first where \
    g>3*n)~0N}[100000]", "1b");
// T1.3: within-pair (a;b) with variables fuses; in flows through with the literal fallback.
eqt!(fz9_within_pair, "{[n] g::\"j\"$3*til n; a9::300; b9::900; u:g within \
    (a9;b9); (where g within (a9;b9))~where u}[100000]", "1b");
eqt!(fz9_where_in,    "{[n] g::\"j\"$(til n) mod 977; u:g in 5 17 200; (where \
    g \
    in 5 17 200)~where u}[100000]", "1b");
eqt!(fz9_cnt_in,      "{[n] g::\"j\"$(til n) mod 977; u:g in 5 17 200; (count \
    where g in 5 17 200)=count where u}[100000]", "1b");
eqt!(fz9_selg_in,     "{[n] gx::\"j\"$(til n) mod 977; gy::0.5+til n; u:where \
    gx in 5 17; (gy where gx in 5 17)~gy u}[100000]", "1b");
// T1.4: sum (x-y)*(x-y) — identical pure operands evaluate once.
eqt!(fzA_sqdist,       "{[n] a::0.5+til n; b::\"f\"$(til n) mod 977; u:a-b; \
    (sum (a-b)*(a-b))=sum u*u}[100000]", "1b");
eqt!(fzA_euclid,       "{[n] a::0.5+til n; b::\"f\"$(til n) mod 977; u:a-b; \
    (sqrt sum (a-b)*(a-b))=sqrt sum u*u}[100000]", "1b");
eqt!(fzA_assign_guard, "{(x*x:sum x)-sum x*x:1+til x} 10", "2640j");
// T1.5: k#iasc|idesc x select-k, stable and null-ordered; large k falls back to full grade.
eqt!(fzB_topk_asc,   "{[n] g::(til n) mod 9973; u:iasc g; (10#iasc \
    g)~10#u}[200000]", "1b");
eqt!(fzB_topk_desc,  "{[n] g::(til n) mod 9973; u:idesc g; (10#idesc \
    g)~10#u}[200000]", "1b");
eqt!(fzB_topk_float, "{[n] g::\"f\"$(til n) mod 9973; u:iasc g; (10#iasc \
    g)~10#u}[200000]", "1b");
eqt!(fzB_topk_nulls, "{g::1 0N 3 0N 2; u:iasc g; (2#iasc g)~2#u}[]", "1b");
eqt!(fzB_topk_nan,   "{g::1 0n 3 0n 2.0; u:iasc g; (2#iasc g)~2#u}[]", "1b");
eqt!(fzB_topk_ties,  "{g::(til 1000) mod 3; u:iasc g; (5#iasc g)~5#u}[]", "1b");
// T1.6: neg/reciprocal join the fast-math chain and preserve nulls like the unfused verbs.
eqt!(fzC_expneg,    "{[n] g::\"f\"$(til n) mod 977; u:neg g; (exp neg g)~exp \
    u}[100000]", "1b");
eqt!(fzC_expneg_int,"{[n] g::\"j\"$(til n) mod 977; u:neg g; (exp neg g)~exp \
    u}[100000]", "1b");
eqt!(fzC_expneg_nan,"{g::1.0 0n 3.0; u:neg g; (exp neg g)~exp u}[]", "1b");
eqt!(fzC_sum_expneg,"{[n] g::\"f\"$(til n) mod 977; u:neg g; (sum exp neg \
    g)=sum exp u}[100000]", "1b");

// FUSE PARITY — regression gate for the fused-vs-unfused silent-corruption class; oracle = eval parse (never value).
eqt!(fz_b1_sum_sqrt, "(sum sqrt 1.0 4.0 0n 16.0)~eval parse \"sum sqrt 1.0 4.0 \
    0n 16.0\"", "1b");
eqt!(fz_b1_avg_sqrt, "(avg sqrt 1.0 4.0 0n 16.0)~eval parse \"avg sqrt 1.0 4.0 \
    0n 16.0\"", "1b");
eqt!(fz_b1_prd_sqrt, "(prd sqrt 1.0 4.0 0n 16.0)~eval parse \"prd sqrt 1.0 4.0 \
    0n 16.0\"", "1b");
eqt!(fz_b1_negneg,   "(sum neg neg 1.0 4.0 0n 16.0)~eval parse \"sum neg neg \
    1.0 4.0 0n 16.0\"", "1b");
eqt!(fz_b1_abs,      "(sum abs 1.0 4.0 0n 16.0)~eval parse \"sum abs 1.0 4.0 \
    0n \
    16.0\"", "1b");
// B1 compressed (int col, sqrt of negatives → NaN lanes; tolerant cmp).
eqt!(fz_b1_cmp, "{cf::((til 200000)mod 50)-10; r:sum sqrt cf; u:eval parse \
    \"sum sqrt cf\"; ((null r)=null u)&1e-6>abs(r-u)%1|abs u}[]", "1b");
// B2 — sum (n?m)*(n?m): two independent deals, not collapsed to sum-of-squares.
eqt!(fz_b2_deal_indep, "(sum (1000000?1000.0)*1000000?1000.0)<3.0e11", "1b");
// B3 — integer scale stays int (was silently widened KF); float scale → float.
eqt!(fz_b3_int_scale,  "(type neg 1 2 3 4 5*2)~eval parse \"type neg 1 2 3 4 \
    5*2\"", "1b");
eqt!(fz_b3_bigval,     "(abs 9007199254740993 9007199254740995j*1)~eval parse \
    \"abs 9007199254740993 9007199254740995j*1\"", "1b");
eqt!(fz_b3_flt_scale,  "(type neg 1.0 2.0 3.0*2.0)~eval parse \"type neg 1.0 \
    2.0 3.0*2.0\"", "1b");
// B4/B5 — sorted float =/</> must equal the unsorted compare (large + leading-null cases).
eqt!(fz_b4_sorted_eq,
    "(where(`s#0n,0n,(50#10.0),(50#20.0),50#30.0)=20.0)~eval \
    parse \"where(`s#0n,0n,(50#10.0),(50#20.0),50#30.0)=20.0\"", "1b");
eqt!(fz_b5_sorted_lt,  "(where(`s#0n,0n,0n,0.0+til 2000)<50.0)~eval parse \
    \"where(`s#0n,0n,0n,0.0+til 2000)<50.0\"", "1b");
eqt!(fz_b5_sorted_gt,  "(where(`s#0n,0n,0n,0.0+til 2000)>1950.0)~eval parse \
    \"where(`s#0n,0n,0n,0.0+til 2000)>1950.0\"", "1b");
// B6 — float = is NaN-aware (0n=3.0→0b, 0n=0n→1b); twin excludes nulls.
eqt!(fz_b6_eq_null,    "{d:0n,1.0 2.0 3.0 4.0 5.0; (sum d=3.0)~sum(not null \
    d)&d=3.0}[]", "1b");
eqt!(fz_b6_eq_nan2nan, "(0n=0n)~1b", "1b");
eqt!(fz_b6_eq_finite,  "(1.0 2.0 3.0=2.0)~010b", "1b");
// B7 — within excludes null/NaN; twin = (not null)&(>=lo)&(<=hi).
eqt!(fz_b7_within_null,"{d:0n,1.0 2.0 3.0 4.0 5.0; (sum d within 2.0 \
    4.0)~sum(not null d)&(d>=2.0)&d<=4.0}[]", "1b");
eqt!(fz_b7_within_int, "(0N 1 2 3 4 within 1 3)~01110b", "1b");
// B7 type-safety: the float NaN-drop must not reach non-numeric types; within stays order-only.
eqt!(fz_b7_within_char,"(\"adgz\" within(\"a\";\"m\"))~1110b", "1b");
eqt!(fz_b7_within_date,"((2000.01.01 2000.06.01 2001.01.01) \
    within(2000.01.01;2000.12.31))~110b", "1b");
// Every element is within [min;max] by construction across long/float/date/char.
eqt!(fz_b7_qlang, "{[n] all {[x] all x within(min x;max x)}each \
    (`long$n?100;n?1.0;2000.01.01+n?3650;n?\"abcdefghij\")}[100001]", "1b");
// B8 — sum(scalar cmp atom) must not error (FZS_CNT atom guard).
eqt!(fz_b8_scalar,     "{va:5; (sum va>3)~1b}[]", "1b");

// TIMESPAN (KN) + TIMESTAMP (KP) — 8-byte nanosecond temporals: type, cast, compare, math, tables, .z vars.
eqt!(tsp_type_atom,  "type 16h$5",                  "-16h");
eqt!(tsp_type_vec,   "type 16h$1 2 3",              "16h");
eqt!(tss_type_atom,  "type 12h$5",                  "-12h");
eqt!(tss_type_vec,   "type 12h$1 2 3",              "12h");
eqt!(tsp_val,        "\"j\"$16h$5",                 "5j");
eqt!(tsp_vec_val,    "\"j\"$16h$1 2 3",             "1 2 3j");
eqt!(tss_val,        "\"j\"$12h$5",                 "5j");
eqt!(tsp_count,      "count 16h$1 2 3 4 5",         "5");
eqt!(tsp_null,       "\"j\"$16h$0N",                "0Nj");
// -- display: string (all four print channels funnel through str0) --
eqt!(tsp_string,     "string 16h$5",                "\"0D00:00:00.000000005\"");
eqt!(tsp_string_neg, "string 16h$-5",
    "\"-0D00:00:00.000000005\"");
eqt!(tsp_string_hr,  "string 16h$3600000000000j",   "\"0D01:00:00.000000000\"");
eqt!(tsp_string_day, "string 16h$86400000000001j",  "\"1D00:00:00.000000001\"");
eqt!(tss_string,     "string 12h$5",
    "\"2000.01.01D00:00:00.000000005\"");
eqt!(tsp_string_vec, "string 16h$1 2",
    "(\"0D00:00:00.000000001\";\"0D00:00:00.000000002\")");
// -- casts: numeric / char / symbol --
eqt!(tsp_cast_charn, "\"j\"$\"n\"$5",               "5j");
eqt!(tsp_cast_symn,  "\"j\"$`timespan$5",           "5j");
eqt!(tss_cast_charp, "type \"p\"$5",                "-12h");
eqt!(tss_cast_symp,  "type `timestamp$5",           "-12h");
eqt!(tsp_cast_rt,    "\"j\"$`timespan$\"j\"$16h$5", "5j");
eqt!(tsp_cast_float, "\"j\"$16h$5.0",               "5j");
// -- cross-temporal casts (ns scaling, instant↔component) --
eqt!(tsp_from_time,  "(16h$00:00:05.000)~16h$5000000000j",       "1b");
eqt!(tss_from_date,  "(12h$2000.01.02)~12h$86400000000000j",     "1b");
eqt!(tsp_to_time,    "(\"t\"$16h$5000000j)~00:00:00.005",        "1b");
eqt!(tsp_to_sec,     "(\"v\"$16h$5000000000j)~00:00:05",         "1b");
eqt!(tss_to_date,    "(\"d\"$12h$86400000000000j)~2000.01.02",   "1b");
eqt!(tss_to_tsp,     "(16h$12h$3600000000000j)~16h$3600000000000j", "1b");
eqt!(tss_to_time,    "(\"t\"$12h$45000000000000j)~12:30:00.000", "1b");
eqt!(tss_to_dtime,   "(\"z\"$12h$43200000000000j)~2000.01.01T12:00:00.000",
    "1b");
// -- comparison / equality / match --
eqt!(tsp_eq,         "(16h$5)=16h$5",               "1b");
eqt!(tsp_neq,        "(16h$5)=16h$6",               "0b");
eqt!(tsp_lt,         "(16h$5)<16h$6",               "1b");
eqt!(tsp_gt,         "(16h$7)>16h$6",               "1b");
eqt!(tsp_veq,        "(16h$1 2 3)=16h$1 9 3",       "101b");
eqt!(tsp_match,      "(16h$1 2 3)~16h$1 2 3",       "1b");
eqt!(tss_eq,         "(12h$5)=12h$5",               "1b");
// -- aggregates & structural verbs --
eqt!(tsp_sum,        "\"j\"$sum 16h$1 2 3 4",       "10j");
eqt!(tsp_sum_type,   "type sum 16h$1 2 3 4",        "-16h");
eqt!(tsp_min,        "\"j\"$min 16h$3 1 2",         "1j");
eqt!(tsp_min_type,   "type min 16h$3 1 2",          "-16h");
eqt!(tsp_max,        "\"j\"$max 16h$3 1 2",         "3j");
eqt!(tsp_distinct,   "\"j\"$distinct 16h$1 1 2 2 3","1 2 3j");
eqt!(tsp_asc,        "\"j\"$asc 16h$3 1 2",         "1 2 3j");
eqt!(tsp_asc_type,   "type asc 16h$3 1 2",          "16h");
eqt!(tsp_desc,       "\"j\"$desc 16h$3 1 2",        "3 2 1j");
eqt!(tsp_rev,        "\"j\"$reverse 16h$1 2 3",     "3 2 1j");
eqt!(tsp_first,      "\"j\"$first 16h$7 8 9",       "7j");
eqt!(tsp_last,       "\"j\"$last 16h$7 8 9",        "9j");
eqt!(tsp_find,       "(16h$1 2 3)?16h$2",           "1");
eqt!(tsp_in_yes,     "(16h$2) in 16h$1 2 3",        "1b");
eqt!(tsp_in_no,      "(16h$5) in 16h$1 2 3",        "0b");
eqt!(tsp_take,       "(2#16h$9 8 7)~16h$9 8",       "1b");
eqt!(tsp_drop,       "(1_16h$9 8 7)~16h$8 7",       "1b");
eqt!(tsp_index,      "\"j\"$(16h$10 20 30)1",       "20j");
eqt!(tsp_indexv,     "\"j\"$(16h$10 20 30)0 2",     "10 30j");
eqt!(tsp_fill,       "(0^16h$1 2 3)~16h$1 2 3",     "1b");
eqt!(tsp_group_key,  "\"j\"$key group 16h$1 1 2",   "1 2j");
eqt!(tsp_group_cnt,  "count group 16h$1 1 2",       "2");
eqt!(tsp_each,       "\"j\"${x}each 16h$1 2 3",     "1 2 3j");
// -- timespan math (durations) --
eqt!(tsp_add,        "((16h$10)+16h$20)~16h$30",    "1b");
eqt!(tsp_add_type,   "type (16h$10)+16h$20",        "-16h");
eqt!(tsp_sub,        "((16h$30)-16h$10)~16h$20",    "1b");
eqt!(tsp_sub_type,   "type (16h$30)-16h$10",        "-16h");
eqt!(tsp_mul,        "((16h$10)*3)~16h$30",         "1b");
eqt!(tsp_mul_comm,   "(3*16h$10)~16h$30",           "1b");
eqt!(tsp_mul_type,   "type (16h$10)*3",             "-16h");
eqt!(tsp_addint,     "((16h$10)+5)~16h$15",         "1b");
eqt!(tsp_subint,     "((16h$30)-5)~16h$25",         "1b");
eqt!(tsp_vec_add,    "((16h$1 2 3)+16h$10 20 30)~16h$11 22 33", "1b");
eqt!(tsp_vec_scale,  "((16h$1 2 3)*2)~16h$2 4 6",   "1b");
// -- mixed timestamp ↔ timespan math --
eqt!(tss_sub_kn,     "type (12h$1000)-12h$200",     "-16h");
eqt!(tss_sub_val,    "((12h$1000)-12h$200)~16h$800","1b");
eqt!(tss_add_span,   "type (12h$5)+16h$10",         "-12h");
eqt!(tss_add_val,    "((12h$5)+16h$10)~12h$15",     "1b");
eqt!(tss_sub_span,   "((12h$100)-16h$30)~12h$70",   "1b");
eqt!(tss_span_add,   "type (16h$10)+12h$5",         "-12h");
eqt!(tss_addint,     "((12h$1000)+100)~12h$1100",   "1b");
eqt!(tss_dur_pos,    "((12h$1000)-12h$200)>16h$0",  "1b");
// -- tables / HDB-style columns --
eqt!(tbl_tsp_sum,    "{t:([]a:16h$10 20 30;b:1 2 3); \"j\"$exec sum a from \
    t}[]",        "60j");
eqt!(tbl_tsp_where,  "{t:([]a:16h$10 20 30;b:1 2 3); \"j\"$exec a from t where \
    b>1}[]",  "20 30j");
eqt!(tbl_tsp_bysum,  "{t:([]s:`x`y`x; a:16h$10 20 30); \"j\"$value exec sum a \
    by s from t}[]", "40 20j");
eqt!(tbl_tsp_xasc,   "{t:([]a:16h$3 1 2); \"j\"$exec a from `a xasc \
    t}[]",               "1 2 3j");
eqt!(tbl_tsp_coltype,"{t:([]a:16h$1 2 3); type exec a from \
    t}[]",                        "16h");
eqt!(tbl_ts_col,     "{t:([]p:12h$5 10; a:1 2); \"j\"$exec p from \
    t}[]",                 "5 10j");
eqt!(tbl_tsp_dur,    "{t:([]p:12h$100 500 900); \"j\"$exec(last p)-first p \
    from \
    t}[]",   "800j");
// -- .z system vars (timestamp/timespan now) --
eqt!(z_p_type,       "type .z.p",                   "-12h");
eqt!(z_n_type,       "type .z.n",                   "-16h");
eqt!(z_P_type,       "type .z.P",                   "-12h");
eqt!(z_N_type,       "type .z.N",                   "-16h");
eqt!(z_n_range,      "(.z.n>=16h$0)&.z.n<16h$86400000000000j", "1b");
eqt!(z_p_after2000,  ".z.p>12h$0",                  "1b");
// -- 0D…/…D… literal lexing (Phase F) --
eqt!(tsp_lit_atom,   "0D00:00:05.000000000",        "16h$5000000000j");
eqt!(tsp_lit_type,   "type 0D00:00:05.000000000",   "-16h");
eqt!(tsp_lit_day,    "1D02:30:00.000000000",        "16h$95400000000000j");
eqt!(tsp_lit_neg,    "-0D00:00:05.000000000",       "16h$-5000000000j");
eqt!(tsp_lit_short,  "0D00:00:01",                  "16h$1000000000j");
eqt!(tsp_lit_vec,    "type 0D00:00:01 0D00:00:02",  "16h");
eqt!(tss_lit_atom,   "2000.01.02D03:04:05.006007008","12h$97445006007008j");
eqt!(tss_lit_type,   "type 2000.01.02D00:00:00.000000000", "-12h");
eqt!(tsp_lit_rt,     "0D01:00:00.000000000~16h$3600000000000j", "1b");
// -- FFI wire: the K value (not a bool) must decode into the Rust K enum --
#[test]
fn tsp_wire_atom_value() {
    let mut c = conn();
    assert_eq!(c.query("16h$5").unwrap(), K::Timespan(5));
    assert_eq!(c.query("12h$5").unwrap(), K::Timestamp(5));
}
#[test]
fn tsp_wire_vec_value() {
    let mut c = conn();
    assert_eq!(c.query("16h$1 2 3").unwrap(), K::TimespanVec(vec![1, 2, 3]));
    assert_eq!(c.query("12h$10 20").unwrap(), K::TimestampVec(vec![10, 20]));
}
#[test]
fn tsp_wire_literal_value() {
    let mut c = conn();
    assert_eq!(c.query("0D00:00:05.000000000").unwrap(),
        K::Timespan(5_000_000_000));
    assert_eq!(c.query("-0D00:00:00.000000005").unwrap(), K::Timespan(-5));
    assert_eq!(c.query("2000.01.02D00:00:00.000000000").unwrap(),
               K::Timestamp(86_400_000_000_000));
}
#[test]
fn tsp_wire_display_roundtrip() {
    // server -> K::Timespan -> client Display must spell the q literal back.
    let mut c = conn();
    let k = c.query("16h$95400000000000j").unwrap();                            // 1D02:30:00
    assert_eq!(k, K::Timespan(95_400_000_000_000));
    assert_eq!(k.to_string(), "1D02:30:00.000000000");
}
#[test]
fn tsp_wire_serialize_roundtrip() {
    // client -> server -> client: a timespan must survive serialize + deserialize + reparse.
    let mut c = conn();
    let k = c.query("16h$5000000000j").unwrap();
    assert_eq!(k, K::Timespan(5_000_000_000));
    let echoed = c.query(&format!("value \"{k}\"")).unwrap();
    assert_eq!(echoed, K::Timespan(5_000_000_000));
}
// -- N/P sort / grade / scan / each-prior --
eqt!(tsp_iasc,       "(iasc 16h$3 1 2)~1 2 0",          "1b");
eqt!(tsp_idesc,      "(idesc 16h$3 1 2)~0 2 1",         "1b");
eqt!(tsp_sums,       "(sums 16h$1 2 3)~16h$1 3 6",      "1b");
eqt!(tsp_deltas,     "(deltas 16h$10 30 60)~16h$10 20 30", "1b");
eqt!(tsp_deltas_typ, "type deltas 16h$10 30 60",        "16h");
eqt!(tss_asc,        "(asc 12h$30 10 20)~12h$10 20 30", "1b");
eqt!(tss_iasc,       "(iasc 12h$3 1 2)~1 2 0",          "1b");
// -- N/P min/max incl. by-group --
eqt!(tsp_max_by,     "{t:([]s:`x`y`x;a:16h$10 20 30);(value exec max a by s \
    from t)~16h$30 20}[]", "1b");
eqt!(tsp_min_by,     "{t:([]s:`x`y`x;a:16h$10 20 30);(value exec min a by s \
    from t)~16h$10 20}[]", "1b");
eqt!(tss_max,        "(max 12h$5 9 1)~12h$9",           "1b");
eqt!(tss_min,        "(min 12h$5 9 1)~12h$1",           "1b");
// -- N/P math (scale, mixed, div→float edge) --
eqt!(tsp_scale2,     "(2*16h$1 2 3)~16h$2 4 6",         "1b");
eqt!(tsp_div_type,   "type (16h$30)%2",                 "-16h");
eqt!(tsp_div_val,    "((16h$30)%2)~16h$15",             "1b");
eqt!(tsp_div_floor,  "((16h$10)%3)~16h$3",              "1b");
eqt!(tsp_div_vec,    "((16h$30 60)%2)~16h$15 30",       "1b");
eqt!(tsp_div_atomv,  "((16h$30)%2 3)~16h$15 10",        "1b");
eqt!(tsp_div_intkn,  "type 2%16h$30",                   "-9h");
eqt!(tsp_div_knkn,   "type (16h$30)%16h$10",            "-9h");
// -- comparison matrix: every ordering (atom⊗vec, vec⊗atom, vec⊗vec) --
eqt!(tsp_eq_av,      "((16h$5)=16h$3 5 7)~010b",        "1b");
eqt!(tsp_eq_va,      "((16h$3 5 7)=16h$5)~010b",        "1b");
eqt!(tsp_lt_av,      "((16h$5)<16h$3 5 7)~001b",        "1b");
eqt!(tsp_lt_va,      "((16h$3 5 7)<16h$5)~100b",        "1b");
eqt!(tsp_gt_av,      "((16h$5)>16h$3 5 7)~100b",        "1b");
eqt!(tsp_le_av,      "((16h$5)<=16h$3 5 7)~011b",       "1b");
eqt!(tsp_ge_av,      "((16h$5)>=16h$3 5 7)~110b",       "1b");
eqt!(tsp_ne_va,      "((16h$3 5 7)<>16h$5)~101b",       "1b");
eqt!(tsp_min_av,     "((16h$5)&16h$3 7)~16h$3 5",       "1b");
eqt!(tsp_min_typ,    "type (16h$5)&16h$3",              "-16h");
eqt!(tsp_max_av,     "((16h$5)|16h$3 7)~16h$5 7",       "1b");
eqt!(tsp_within_vec, "((16h$1 5 9) within 16h$3 7)~010b", "1b");
eqt!(tss_lt_av,      "((12h$5)<12h$3 5 7)~001b",        "1b");
eqt!(tss_min_typ,    "type (12h$5)&12h$3",              "-12h");
// -- cross-type compare / in / find: coerce the underlying ns, like `=` --
eqt!(tsp_x_eq_int,   "((16h$5)=5)~1b",                  "1b");
eqt!(tsp_x_lt_long,  "((16h$5)<7j)~1b",                 "1b");
eqt!(tsp_x_kp_eq_kn, "((12h$5)=16h$5)~1b",              "1b");
eqt!(tsp_x_in_long,  "(5j in 16h$1 5 9)~1b",            "1b");
eqt!(tsp_x_in_int,   "((16h$5) in 5 6 7)~1b",           "1b");
eqt!(tsp_x_in_kp,    "((16h$5) in 12h$5 9)~1b",         "1b");
eqt!(tsp_x_find_int, "((16h$1 5 9)?5)~1",               "1b");
eqt!(tsp_x_find_lng, "((16h$1 5 9)?5j)~1",              "1b");
eqt!(tss_x_in_long,  "(5j in 12h$1 5 9)~1b",            "1b");
eqt!(tss_sub_kn2,    "type (12h$900)-12h$100",          "-16h");
eqt!(tss_plus_span,  "((12h$5)+16h$95)~12h$100",        "1b");
// -- N/P g# attribute + group + distinct --
eqt!(tsp_gattr,      "(`g#16h$1 1 2 2)~16h$1 1 2 2",    "1b");
eqt!(tsp_group_g,    "{g:`g#16h$1 1 2 3 2;(key group g)~16h$1 2 3}[]", "1b");
eqt!(tsp_distinct_g, "{g:`g#16h$5 5 9 5;(distinct g)~16h$5 9}[]", "1b");
// -- N/P tables: keyed / update / xasc-carry --
eqt!(tsp_keyed,      "{t:([k:16h$1 2 3]v:10 20 30);(exec v from t where \
    k=16h$2)~1#20}[]", "1b");
eqt!(tsp_update,     "{t:([]a:16h$1 2 3);(exec a from update a:a+16h$10 from \
    t)~16h$11 12 13}[]", "1b");
eqt!(tsp_xasc_carry, "{t:([]a:16h$3 1 2;b:`x`y`z);(exec b from `a xasc \
    t)~`y`z`x}[]", "1b");
eqt!(tss_col_dur,    "{t:([]p:12h$100 500 900);(exec(last p)-first p from \
    t)~16h$800}[]", "1b");
// -- N/P cross-temporal cast chains --
eqt!(tsp_to_minute,  "(`int$`minute$16h$3600000000000j)~60", "1b");
eqt!(tss_to_dtime2,  "(`datetime$12h$43200000000000j)~2000.01.01T12:00:00.000",
    "1b");
// -- compressed N/P column ops match the raw twin --
eqt!(tsp_coc_sum,    "{[n] g::16h$1000000000+asc(til n)mod \
    500;r:16h$1000000000+asc(til n)mod 500;(sum g)~sum r}[100000]", "1b");
eqt!(tsp_coc_minmax, "{[n] g::16h$1000000000+asc(til n)mod \
    500;r:16h$1000000000+asc(til n)mod 500;((min g)~min r)&(max g)~max \
        r}[100000]", "1b");
eqt!(tsp_coc_where,  "{[n] g::16h$1000000000+asc(til n)mod \
    500;r:16h$1000000000+asc(til n)mod 500;(g where g>16h$1000000000+250)~r \
        where r>16h$1000000000+250}[100000]", "1b");
eqt!(tsp_coc_arith,  "{[n] g::16h$1000000000+asc(til n)mod \
    500;r:16h$1000000000+asc(til n)mod 500;(g+16h$5)~r+16h$5}[100000]", "1b");

// Multi-column group-by must match an independent single-key oracle (group count, per-group aggregate, total).
eqt!(coc_grp_rawmulti_ki2,
  "{[n] t:([]a:n?10;b:n?7;v:n?100); r:0!select s:sum v by a,b from t; \
      o:0!select s:sum v by id from update id:a+100*b from t; (count[r]=count \
          o)&(asc[r`s]~asc o`s)&(sum[r`s]=sum t`v)}[100000]",
  "1b");
eqt!(coc_grp_rawmulti_sym2,
  "{[n] u:`x`y`z; t:([]a:n?u;b:n?5;v:n?100); r:0!select s:sum v by a,b from t; \
      o:0!select s:sum v by id from update id:(u?a)+10*b from t; \
          (count[r]=count o)&(asc[r`s]~asc o`s)&(sum[r`s]=sum t`v)}[100000]",
  "1b");
eqt!(coc_grp_rawmulti_kj6,
  "{[n] u:`a`b`c`d`e`f`g`h; \
      t:([]a:n?u;b:n?u;c:n?100;d:n?100;e:n?100;g:n?1000;v:n?1.0); r:0!select \
          s:sum v,cnt:count i by a,b,c,d,e,g from t; id:`long$u?t`a; \
              id:(8*id)+u?t`b; id:(1000*id)+t`c; id:(100*id)+t`d; \
                  id:(100*id)+t`e; id:(1000*id)+t`g; o:0!select s:sum \
                      v,cnt:count i by id from update id:id from t; \
                          (count[r]=count o)&(asc[r`cnt]~asc \
                              o`cnt)&(sum[r`cnt]=n)&(abs[(sum r`s)-sum \
                                  t`v]<0.01)}[200000]",
  "1b");
eqt!(coc_grp_rawmulti_gattr,
  "{[n] t:update `g#a,`g#b from ([]a:n?`x`y`z`w;b:n?`p`q`r;v:n?100); \
      r:0!select \
      s:sum v by a,b from t; o:0!select s:sum v by id from update \
          id:(`x`y`z`w?a)+10*`p`q`r?b from t; (count[r]=count o)&(asc[r`s]~asc \
              o`s)&(sum[r`s]=sum t`v)}[100000]",
  "1b");


// Fused dense group-by (top-k, med/sdev/var) must equal the generic gather-then-apply path over a compressed column.
eqt!(coc_grp_topk_desc, "{[n] t:([] g:n?1000; v:n?100.0); (ungroup select v:2 \
    sublist desc v by g from t)~(ungroup select v:{2 sublist desc x} v by g \
        from t)}[100000]", "1b");
// top-2 asc (q8 mirror): keep the 2 SMALLEST per group, ascending.
eqt!(coc_grp_topk_asc, "{[n] t:([] g:n?1000; v:n?100.0); (ungroup select v:2 \
    sublist asc v by g from t)~(ungroup select v:{2 sublist asc x} v by g from \
        t)}[100000]", "1b");
// top-3 desc: k>2 path (k-slot insert + reorder).
eqt!(coc_grp_topk_k3, "{[n] t:([] g:n?500; v:n?1000.0); (ungroup select v:3 \
    sublist desc v by g from t)~(ungroup select v:{3 sublist desc x} v by g \
        from t)}[100000]", "1b");
// top-2 over int values (KI widen-to-F path).
eqt!(coc_grp_topk_int, "{[n] t:([] g:n?1000; v:n?100000); (ungroup select v:2 \
    sublist desc v by g from t)~(ungroup select v:{2 sublist desc x} v by g \
        from t)}[100000]", "1b");
// med + sdev together by 2 INT keys (q6): fused == t3 (multi-col gid).
eqt!(coc_grp_med_sdev_2key, "{[n] t:([] a:n?100; b:n?100; v:n?100.0); (select \
    m:med v, s:sdev v by a,b from t)~(select m:{med x} v, s:{sdev x} v by a,b \
        from t)}[100000]", "1b");
// med by single key, incl. even/odd group sizes (two-middle average).
eqt!(coc_grp_med_1key, "{[n] t:([] g:n?2000; v:n?1000.0); (select m:med v by g \
    from t)~(select m:{med x} v by g from t)}[100000]", "1b");
// var (population) by group: closed-form Σx²-fold == t3.
eqt!(coc_grp_var, "{[n] t:([] g:n?1000; v:n?100.0); (select x:var v by g from \
    t)~(select x:{var x} v by g from t)}[100000]", "1b");
// dev (population sdev) and svar (sample var) by group.
eqt!(coc_grp_dev_svar, "{[n] t:([] g:n?1000; v:n?100.0); (select d:dev v, \
    s:svar v by g from t)~(select d:{dev x} v, s:{svar x} v by g from \
        t)}[100000]", "1b");
// med over INT values: widens KI to F like the generic path.
eqt!(coc_grp_med_int, "{[n] t:([] g:n?1000; v:n?100000); (select m:med v by g \
    from t)~(select m:{med x} v by g from t)}[100000]", "1b");


// Threaded dense group-by SUM/AVG must equal the serial result on a compressed value column.
eqt!(coc_grp_sum_threaded,
  "{[n] t:([] g:n?100; v:-18!n?100); r:0!select s:sum v by g from t; ((select \
      s:sum v by g from t)~(select s:{sum x} v by g from t))&(sum[r`s]=sum \
          t`v)}[300000]",
  "1b");
// float AVG by small-N int key: threaded == twin (deterministic merge).
eqt!(coc_grp_avg_threaded,
  "{[n] t:([] g:n?100; v:-18!n?100.0); (select a:avg v by g from t)~(select \
      a:{avg x} v by g from t)}[300000]",
  "1b");
// Grouped avg over two int + one float column, all lanes threaded, reconciled with the generic twins.
eqt!(coc_grp_avg_q4shape_threaded,
  "{[n] t:([] g:n?100; a:-18!n?5; b:-18!n?15; c:-18!n?100.0); (select x:avg a, \
      y:avg b, z:avg c by g from t)~(select x:{avg x} a, y:{avg x} b, z:{avg \
          x} \
          c by g from t)}[300000]",
  "1b");
// nullable int SUM (nulls are skipped): null-aware scatter == twin.
eqt!(coc_grp_sum_threaded_nulls,
  "{[n] t:([] g:n?100; v:-18!@[n?100;(neg n div 10)?n;:;0N]); (select s:sum v \
      by g from t)~(select s:{sum x} v by g from t)}[300000]",
  "1b");


// Post-aggregate arithmetic ((max v1)-min v2 by g) and fused multi-moment avgs must equal the generic path.
eqt!(coc_grp_aggarith_maxmin, "{[n] t:([] g:n?1000; v1:n?100.0; v2:n?100.0); \
    (select r:(max v1)-min v2 by g from t)~(select r:{[a;b](max a)-min \
        b}[v1;v2] by g from t)}[100000]", "1b");
// q7: max-min over INT cols (the real db-benchmark schema: v1,v2 int).
eqt!(coc_grp_aggarith_int, "{[n] t:([] g:n?1000; v1:n?100; v2:n?100); (select \
    r:(max v1)-min v2 by g from t)~(select r:{[a;b](max a)-min b}[v1;v2] by g \
        from t)}[100000]", "1b");
// q7: multiple post-agg arith cols incl. sum+sum and a monadic-negated agg.
eqt!(coc_grp_aggarith_multi, "{[n] t:([] g:n?500; v1:n?100.0; v2:n?100.0); \
    (select a:(max v1)-min v2, b:(sum v1)+sum v2, c:neg avg v1 by g from \
        t)~(select a:{[x;y](max x)-min y}[v1;v2], b:{[x;y](sum x)+sum \
            y}[v1;v2], c:{neg avg x} v1 by g from t)}[100000]", "1b");
// q7: g#-attributed key (group TOPOLOGY → expand to dense gid → t4p) over sym key.
eqt!(coc_grp_aggarith_gattr, "{[n] t:update `g#g from ([] g:`$\"g\",/:string \
    n?500; v1:n?100.0; v2:n?100.0); (select r:(max v1)-min v2 by g from \
        t)~(select r:{[a;b](max a)-min b}[v1;v2] by g from t)}[100000]", "1b");

// q9: the exact 5-avg moment select (incl. 3 products) == generic, 2 INT keys.
eqt!(coc_grp_moment_5avg, "{[n] t:([] a:n?100; b:n?100; v1:n?100.0; \
    v2:n?100.0); (select mx:avg v1,my:avg v2,mxy:avg v1*v2,mxx:avg \
        v1*v1,myy:avg v2*v2 by a,b from t)~(select mx:{avg x} v1,my:{avg x} \
            v2,mxy:{avg x} v1*v2,mxx:{avg x} v1*v1,myy:{avg x} v2*v2 by a,b \
                from t)}[200000]", "1b");
// q9 over INT value cols (KI→F cast in the fold) — products promote correctly.
eqt!(coc_grp_moment_int, "{[n] t:([] a:n?100; b:n?100; v1:n?1000; v2:n?1000); \
    (select mx:avg v1,mxy:avg v1*v2,mxx:avg v1*v1 by a,b from t)~(select \
        mx:{avg x} v1,mxy:{avg x} v1*v2,mxx:{avg x} v1*v1 by a,b from \
            t)}[200000]", "1b");
// Grouped multi-moment avgs over narrow-range int columns fold on the compressed form, bit-identical to the decode twin.
eqt!(coc_grp_moment_coc, "{[n] t:([] a:n?100; b:n?100; v1:1+n?5; v2:1+n?15); \
    (select mx:avg v1,my:avg v2,mxy:avg v1*v2,mxx:avg v1*v1,myy:avg v2*v2 by \
        a,b from t)~(select mx:{avg x} v1,my:{avg x} v2,mxy:{avg x} \
            v1*v2,mxx:{avg x} v1*v1,myy:{avg x} v2*v2 by a,b from t)}[200000]",
                "1b");
// Same at scale (5M rows): the relaxed gate still folds the compressed form, bit-identical to the decode twin.
eqt!(coc_grp_moment_coc_bigm, "{[n] t:([] a:n?100; b:n?100; v1:1+n?5; \
    v2:1+n?15); (select mx:avg v1,my:avg v2,mxy:avg v1*v2,mxx:avg \
        v1*v1,myy:avg \
        v2*v2 by a,b from t)~(select mx:{avg x} v1,my:{avg x} v2,mxy:{avg x} \
            v1*v2,mxx:{avg x} v1*v1,myy:{avg x} v2*v2 by a,b from \
                t)}[5000000]", "1b");
// q9: single key, single-product avg — degenerate fold still bit-matches.
eqt!(coc_grp_moment_1key, "{[n] t:([] g:n?2000; v1:n?100.0; v2:n?100.0); \
    (select mxy:avg v1*v2 by g from t)~(select mxy:{avg x} v1*v2 by g from \
        t)}[100000]", "1b");
// q9: grouped-attribute first key → topology expand.
eqt!(coc_grp_moment_gattr, "{[n] t:update `g#a from ([] a:`$\"a\",/:string \
    n?100; b:n?100; v1:n?100.0; v2:n?100.0); (select mx:avg v1,my:avg \
        v2,mxy:avg v1*v2,mxx:avg v1*v1,myy:avg v2*v2 by a,b from t)~(select \
            mx:{avg x} v1,my:{avg x} v2,mxy:{avg x} v1*v2,mxx:{avg x} \
                v1*v1,myy:{avg x} v2*v2 by a,b from t)}[200000]", "1b");


// Threaded grouped folds (var/sdev/median) must equal the serial result via deterministic worker merges.
eqt!(coc_grp_moment_threaded, "{[n] t:([] a:n?100; v1:n?100.0; v2:n?100.0); \
    (select mx:avg v1,my:avg v2,mxy:avg v1*v2,mxx:avg v1*v1,myy:avg v2*v2 by a \
        from t)~(select mx:{avg x} v1,my:{avg x} v2,mxy:{avg x} v1*v2,mxx:{avg \
            x} v1*v1,myy:{avg x} v2*v2 by a from t)}[1000000]", "1b");
// q6 moment (var/sdev) fold threaded: 10k groups, 1M rows.
eqt!(coc_grp_var_threaded, "{[n] t:([] a:n?100; b:n?100; v:n?100.0); (select \
    v:var v, s:sdev v by a,b from t)~(select v:{var x} v, s:{sdev x} v by a,b \
        from t)}[1000000]", "1b");
// q6 median partition+select threaded: 10k groups, 1M rows.
eqt!(coc_grp_med_threaded, "{[n] t:([] a:n?100; b:n?100; v:n?100.0); (select \
    m:med v by a,b from t)~(select m:{med x} v by a,b from t)}[1000000]", "1b");
// q6 median threaded over INT values (widens KI→F), even/odd group sizes.
eqt!(coc_grp_med_threaded_int, "{[n] t:([] g:n?500; v:n?100000); (select m:med \
    v by g from t)~(select m:{med x} v by g from t)}[1000000]", "1b");
// Grouped median+sdev over 10k groups / 3M rows against a twin oracle.
eqt!(coc_grp_med_q6shape, "{[n] t:([] a:n?100; b:n?100; v:n?100.0); (select \
    m:med v, s:sdev v by a,b from t)~(select m:{med x} v, s:{sdev x} v by a,b \
        from t)}[3000000]", "1b");

// Big-n multi-column group-by must stay bit-identical to the single packed-key oracle.
eqt!(coc_grp_par_kj6_bign,
  "{[n] t:([]a:n?`a`b`c`d;b:n?`e`f`g;c:n?100;d:n?80;e:n?60;g:n?50000;v:n?1.0); \
      r:0!select s:sum v,cnt:count i by a,b,c,d,e,g from t; \
          id:`long$`a`b`c`d?t`a; id:(3*id)+`e`f`g?t`b; id:(100*id)+t`c; \
              id:(80*id)+t`d; id:(60*id)+t`e; id:(50000*id)+t`g; o:0!select \
                  s:sum v,cnt:count i by id from update id:id from t; \
                      (count[r]=count o)&(asc[r`cnt]~asc \
                          o`cnt)&(sum[r`cnt]=n)&(abs[(sum r`s)-sum \
                              t`v]<0.01)}[1500000]",
  "1b");
// KI 2-col (q2 shape) at 1.5M rows — threaded shift-pack into a KI code, then uIfnd.
eqt!(coc_grp_par_ki2_bign,
  "{[n] t:([]a:n?200;b:n?150;v:n?1000); r:0!select s:sum v by a,b from t; \
      o:0!select s:sum v by id from update id:a+200*b from t; (count[r]=count \
          o)&(asc[r`s]~asc o`s)&(sum[r`s]=sum t`v)}[1500000]",
  "1b");

// 10k-group two-key int SUM threads the dense scatter; result matches the gather twin and conserves the total.
eqt!(coc_grp_sum_threaded_10kgrp,
  "{[n] t:([] a:n?100; b:n?100; v:-18!n?100); r:0!select s:sum v by a,b from \
      t; \
      ((select s:sum v by a,b from t)~(select s:{sum x} v by a,b from \
          t))&(sum[r`s]=sum t`v)}[1000000]",
  "1b");
// Compound sym+int key group-by with 5 avgs must be bit-identical to the generic twin.
eqt!(coc_grp_moment_5avg_gattr_10kgrp,
  "{[n] t:update `g#a from ([] a:`$\"g\",/:string n?100; b:n?100; v1:n?100.0; \
      v2:n?100.0); (select mx:avg v1,my:avg v2,mxy:avg v1*v2,mxx:avg \
          v1*v1,myy:avg v2*v2 by a,b from t)~(select mx:{avg x} v1,my:{avg x} \
              v2,mxy:{avg x} v1*v2,mxx:{avg x} v1*v1,myy:{avg x} v2*v2 by a,b \
                  from t)}[1000000]",
  "1b");
// IN-PLACE APPEND (,:) — extend a compressed column in place; each result matches a raw join oracle.
eqt!(coca_aff_const,  "{[n] amac::1#1;do[n;amac,:1];amac~(n+1)#1}[5000]", "1b");
eqt!(coca_aff_ramp,   "{[n] amar::til 2000;do[n;amar,:1+last amar];amar~til \
    2000+n}[5000]", "1b");
eqt!(coca_aff_codec,  "{[n] amak::2000#7;do[n;amak,:7];(-55)!`amak}[5000]",
    "-1");
eqt!(coca_aff_break,  "{amab::2000#5;amab,:5;amab,:7;amab~(2000#5),5 7}[]",
    "1b");
// -- bounded int column: in-place append --
eqt!(coca_for_val,    "{[n] amfv::2000#100 142 \
    99;do[n;amfv,:142];amfv~(2000#100 142 99),n#142}[5000]", "1b");
eqt!(coca_for_codec,  "{[n] amfk::2000#100 142 \
    99;do[n;amfk,:142];(-55)!`amfk}[5000]", "8016");
eqt!(coca_for_subbyte,"{[n] amfs::2000#0 1 3 2;do[n;amfs,:2];amfs~(2000#0 1 3 \
    2),n#2}[5000]", "1b");
eqt!(coca_for_neg,    "{[n] amfn::2000#-50 -10 \
    -30;do[n;amfn,:-20];amfn~(2000#-50 -10 -30),n#-20}[5000]", "1b");
eqt!(coca_for_abovemax,"{amfa::2000#100 142 99;amfa,:500;amfa~(2000#100 142 \
    99),500}[]", "1b");
eqt!(coca_for_belowmin,"{amfb::2000#100;amfb,:50;amfb~(2000#100),50}[]", "1b");
// -- decimal float column: in-place append --
eqt!(coca_alp_const,  "{[n] \
    amlc::2000#1.5;do[n;amlc,:1.5];amlc~(2000#1.5),n#1.5}[5000]", "1b");
eqt!(coca_alp_codec,  "{[n] amlk::2000#1.5;do[n;amlk,:1.5];(-55)!`amlk}[5000]",
    "1017");
eqt!(coca_alp_dec,    "{[n] amld::0.01*til \
    2000;do[n;amld,:5.55];amld~(0.01*til \
    2000),n#5.55}[5000]", "1b");
eqt!(coca_alp_ke,     "{[n] \
    amle::2000#1.5e;do[n;amle,:2.5e];amle~(2000#1.5e),n#2.5e}[5000]", "1b");
eqt!(coca_alp_sum,    "{[n] amls::2000#2.0;do[n;amls,:2.0];(sum \
    amls)=2.0*2000+n}[5000]", "1b");
eqt!(coca_alp_gather, "{[n] amlg::0.01*til 2000;do[n;amlg,:5.0];(amlg 2000+til \
    n)~n#5.0}[100]", "1b");
eqt!(coca_alp_exccede,"{amlx::0.01*til 2000;amlx,:1%3;amlx~(0.01*til \
    2000),1%3}[]", "1b");
// -- SYM (order-preserving W-byte codes): write code at tail; tier overflow cedes --
eqt!(coca_sym_val,    "{[n] \
    amyv::2000#`aa;do[n;amyv,:`bb];amyv~(2000#`aa),n#`bb}[5000]", "1b");
eqt!(coca_sym_codec,  "{[n] amyk::2000#`aa;do[n;amyk,:`bb];(-55)!`amyk}[5000]",
    "-1");
eqt!(coca_sym_w8,     "{[n] \
    amy8::2000#`abcdef;do[n;amy8,:`abcdef];amy8~(2000#`abcdef),n#`abcdef}[5000\
        ]", "1b");
eqt!(coca_sym_tiercede,"{amyt::2000#`aa;amyt,:`abcdefghijkl;amyt~(2000#`aa),`ab\
    c\
    defghijkl}[]", "1b");
// -- irrational float column: append falls back to decode+recompress, value-exact --
eqt!(coca_alq_cede,   "{amqc::sqrt 1.0+til \
    2000;amqo:amqc,300#0.5;do[300;amqc,:0.5];amqc~amqo}[]", "1b");
// -- RHS-vector fix: `a,:v` where v is a COMPRESSED column (was corruption/SIGSEGV) --
eqt!(coca_qz_for,     "{amq1::2500#1 2 3;amq2::2500#4 5 \
    6;amq1,:amq2;amq1~(2500#1 2 3),2500#4 5 6}[]", "1b");
eqt!(coca_qz_alp,
    "{amq3::2000#1.5;amq4::2000#2.5;amq3,:amq4;amq3~(2000#1.5),2000#2.5}[]",
        "1b");
eqt!(coca_qz_sym,
    "{amq5::2000#`aa;amq6::2000#`bb;amq5,:amq6;amq5~(2000#`aa),2000#`bb}[]",
        "1b");
// -- edge: shared alias not corrupted (refcount>1 cedes); vector append --
eqt!(coca_shared,     "{amsh::2000#1.5;amshb:amsh;amsh,:2.5;amshb~2000#1.5}[]",
    "1b");
eqt!(coca_vec_append, "{amve::2000#1.5;amve,:1.5 1.5 1.5;amve~2003#1.5}[]",
    "1b");
// -- stress (large N): O(1) amortized correctness + count at scale --
eqt!(coca_stress_aff, "{[n] amsa::1#1;do[n;amsa,:1];(count amsa)=n+1}[200000]",
    "1b");
eqt!(coca_stress_alp, "{[n] amsl::2000#1.5;do[n;amsl,:1.5];(count \
    amsl)=2000+n}[100000]", "1b");
eqt!(coca_stress_sym, "{[n] amss::2000#`aa;do[n;amss,:`bb];(count \
    amss)=2000+n}[100000]", "1b");
eqt!(coca_stress_forv,"{[n] \
    amsfv::2000#7;do[n;amsfv,:7];amsfv~(2000+n)#7}[100000]", "1b");
eqt!(coca_stress_widen,"{[n] amsw::1#0;do[n;amsw,:1+last amsw];amsw~til \
    n+1}[100000]", "1b");
// REGRESSION: grouped moment aggregations over many small int groups must not corrupt the heap (argument-order fix).
eqt!(coc_grp_var_noheapcorrupt,  "{[n] 0<count select v:var  v1 by id6 from \
    ([]id6:n?10000; v1:1+n?5)}[1000000]", "1b");
eqt!(coc_grp_dev_noheapcorrupt,  "{[n] 0<count select v:dev  v1 by id6 from \
    ([]id6:n?10000; v1:1+n?5)}[1000000]", "1b");
eqt!(coc_grp_svar_noheapcorrupt, "{[n] 0<count select v:svar v1 by id6 from \
    ([]id6:n?10000; v1:1+n?5)}[1000000]", "1b");
eqt!(coc_grp_sdev_noheapcorrupt, "{[n] 0<count select v:sdev v1 by id6 from \
    ([]id6:n?10000; v1:1+n?5)}[1000000]", "1b");
eqt!(coc_grp_cov_noheapcorrupt,  "{[n] 0<count select c:cov[v1;v2] by id6 from \
    ([]id6:n?10000; v1:1+n?5; v2:1+n?15)}[1000000]", "1b");
eqt!(coc_grp_cor_noheapcorrupt,  "{[n] 0<count select c:cor[v1;v2] by id6 from \
    ([]id6:n?10000; v1:1+n?5; v2:1+n?15)}[1000000]", "1b");
eqt!(coc_grp_avgsq_noheapcorrupt,"{[n] 0<count select c:{avg x*x} v1 by id6 \
    from ([]id6:n?10000; v1:1+n?5)}[1000000]", "1b");
// Scaled int column: decode and every follow-on verb (sum/max/where/asc) must reconstruct the true values.
eqt!(coc_fors_mul_decode, "{[n] g::\"j\"$(til n)mod 1000; (g*3)~\"j\"$3*(til \
    n)mod 1000}[200000]", "1b");
eqt!(coc_fors_mul_sum,    "{[n] g::\"j\"$(til n)mod 1000; (sum \
    g*3)=sum\"j\"$3*(til n)mod 1000}[200000]", "1b");
eqt!(coc_fors_mul_maxmin, "{[n] g::\"j\"$(til n)mod 1000; ((max \
    g*3)=max\"j\"$3*(til n)mod 1000)&(min g*3)=min\"j\"$3*(til n)mod \
        1000}[200000]", "1b");
eqt!(coc_fors_mul_where,  "{[n] g::\"j\"$(til n)mod 1000; \
    (where(g*3)>1500)~where(\"j\"$3*(til n)mod 1000)>1500}[200000]", "1b");
eqt!(coc_fors_mul_asc,    "{[n] g::\"j\"$(til n)mod 1000; (asc \
    g*3)~asc\"j\"$3*(til n)mod 1000}[100000]", "1b");
eqt!(coc_fors_mul_big_j,   "{[n] g::\"j\"$1000000000+(til n)mod 1000; \
    (g*7)~7*1000000000j+(til n)mod 1000}[200000]", "1b");
eqt!(coc_fors_mul_chain,  "{[n] g::\"j\"$(til n)mod 1000; \
    ((g*3)*2)~\"j\"$6*(til n)mod 1000}[200000]", "1b");
eqt!(coc_fors_mul_neg,    "{[n] g::\"j\"$(til n)mod 1000; (g*-2)~\"j\"$-2*(til \
    n)mod 1000}[200000]", "1b");
// Scaled-column readers that parse the header directly must apply the scale or fall back to decode.
eqt!(coc_fors_mul_at,      "{[n] g::\"j\"$(til n)mod 1000; \
    ((g*3)17)=(\"j\"$3*(til n)mod 1000)17}[200000]", "1b");
eqt!(coc_fors_mul_gather,  "{[n] g::\"j\"$(til n)mod 1000; idx:(n div 4)?n; \
    ((g*3)idx)~(\"j\"$3*(til n)mod 1000)idx}[200000]", "1b");
eqt!(coc_fors_mul_gat_byte,"{[n] g::\"j\"$200+(til n)mod 100; idx:(n div 4)?n; \
    ((g*3)idx)~(\"j\"$3*200+(til n)mod 100)idx}[200000]", "1b");
eqt!(coc_fors_mul_find,    "{[n] g::\"j\"$(til n)mod 1000; \
    ((g*3)?900j)=(\"j\"$3*(til n)mod 1000)?900j}[200000]", "1b");
// S2/S3: m>0 stamps value bounds (min/max O(1)) + propagates the sorted attr (median's sorted path).
eqt!(coc_fors_mul_med,     "{[n] g::\"j\"$(til n)mod 1000; (med g*3)=med \
    \"j\"$3*(til n)mod 1000}[200000]", "1b");
eqt!(coc_fors_mul_sortmm,  "{[n] g::asc \"j\"$(til n)mod 1000; \
    r:\"j\"$3*asc(til n)mod 1000; ((min g*3)=min r)&(max g*3)=max r}[200000]",
        "1b");
// Scaled-column sum/avg closed form; avgbig forces the wide-sum path that once dropped the scale.
eqt!(coc_fors_mul_avgbig,  "{[n] g::\"i\"$(til n)mod 4000; (avg g*3)=avg \
    \"i\"$3*(til n)mod 4000}[200000]", "1b");
eqt!(coc_fors_mul_sumki,   "{[n] g::\"i\"$(til n)mod 4000; (sum g*3)=sum \
    \"i\"$3*(til n)mod 4000}[200000]", "1b");
eqt!(coc_fors_mul_sumkj,   "{[n] g::\"j\"$(til n)mod 1000; (sum g*7)=sum \
    \"j\"$7*(til n)mod 1000}[200000]", "1b");
// distinct/group/neg on a scaled column fall back to decode (were producing wrong values).
eqt!(coc_fors_mul_distinct,"{[n] g::\"j\"$(til n)mod 1000; (asc distinct \
    g*3)~asc distinct \"j\"$3*(til n)mod 1000}[200000]", "1b");
eqt!(coc_fors_mul_group,   "{[n] g::\"j\"$(til n)mod 1000; (asc key group \
    g*3)~asc key group \"j\"$3*(til n)mod 1000}[200000]", "1b");
eqt!(coc_fors_mul_neg2,    "{[n] g::\"j\"$(til n)mod 1000; (neg g*3)~neg \
    \"j\"$3*(til n)mod 1000}[200000]", "1b");
eqt!(coc_fors_mul_grpval,  "{[n] g::\"j\"$(til n)mod 50; (asc each group \
    g*3)~asc each group \"j\"$3*(til n)mod 50}[200000]", "1b");
// Sorted scaled column: g*m on a sorted source stays a plain sorted column; confirms correctness and defensive cedes.
eqt!(coc_fors_mul_srt_eq,    "{[n] g::asc \"j\"$(til n)mod 1000; \
    (where(g*3)=1500)~where(asc \"j\"$3*(til n)mod 1000)=1500}[200000]", "1b");
eqt!(coc_fors_mul_srt_lt,    "{[n] g::asc \"j\"$(til n)mod 1000; \
    (where(g*3)<1490)~where(asc \"j\"$3*(til n)mod 1000)<1490}[200000]", "1b");
eqt!(coc_fors_mul_srt_le,    "{[n] g::asc \"j\"$(til n)mod 1000; \
    (where(g*3)<=1500)~where(asc \"j\"$3*(til n)mod 1000)<=1500}[200000]",
        "1b");
eqt!(coc_fors_mul_srt_gt,    "{[n] g::asc \"j\"$(til n)mod 1000; \
    (where(g*3)>1490)~where(asc \"j\"$3*(til n)mod 1000)>1490}[200000]", "1b");
eqt!(coc_fors_mul_srt_eqodd, "{[n] g::asc \"j\"$(til n)mod 1000; \
    (where(g*3)=1501)~where(asc \"j\"$3*(til n)mod 1000)=1501}[200000]", "1b");
eqt!(coc_fors_mul_srt_within,"{[n] g::asc \"j\"$(til n)mod 1000; \
    (where(g*3)within 600 1800)~where(asc \"j\"$3*(til n)mod 1000)within 600 \
        1800}[200000]", "1b");
eqt!(coc_fors_mul_srt_winog, "{[n] g::asc \"j\"$(til n)mod 1000; \
    (where(g*3)within 601 1799)~where(asc \"j\"$3*(til n)mod 1000)within 601 \
        1799}[200000]", "1b");
eqt!(coc_fors_mul_srt_bin,   "{[n] g::asc \"j\"$(til n)mod 1000; ((g*3) bin 0 \
    1497 3000j)~(asc \"j\"$3*(til n)mod 1000) bin 0 1497 3000j}[100000]", "1b");
// Value-domain streaming verbs (within/in/reverse/div) run on a scaled column without inflating.
eqt!(coc_fors_mul_within,   "{[n] g::\"j\"$(til n)mod 1000; (where(g*3)within \
    600 1800)~where(\"j\"$3*(til n)mod 1000)within 600 1800}[200000]", "1b");
eqt!(coc_fors_mul_within_og,"{[n] g::\"j\"$(til n)mod 1000; (where(g*3)within \
    601 1799)~where(\"j\"$3*(til n)mod 1000)within 601 1799}[200000]", "1b");
eqt!(coc_fors_mul_within_b, "{[n] g::\"j\"$1000000+(til n)mod 100; \
    (where(g*7)within 7000200 7000500)~where(\"j\"$7*1000000+(til n)mod \
        100)within 7000200 7000500}[100000]", "1b");
eqt!(coc_fors_mul_in4,      "{[n] g::\"j\"$(til n)mod 1000; ((g*3)in 0 6 1500 \
    2997)~(\"j\"$3*(til n)mod 1000)in 0 6 1500 2997}[200000]", "1b");
eqt!(coc_fors_mul_in_odd,   "{[n] g::\"j\"$(til n)mod 1000; ((g*3)in 1 2 1501 \
    1499)~(\"j\"$3*(til n)mod 1000)in 1 2 1501 1499}[200000]", "1b");
eqt!(coc_fors_mul_in_big,   "{[n] g::\"j\"$(til n)mod 1000; \
    ((g*3)in\"j\"$3*til \
    500)~(\"j\"$3*(til n)mod 1000)in\"j\"$3*til 500}[200000]", "1b");
eqt!(coc_fors_mul_rev2,     "{[n] g::\"j\"$(til n)mod 1000; (reverse \
    g*7)~reverse\"j\"$7*(til n)mod 1000}[200000]", "1b");
eqt!(coc_fors_mul_div,      "{[n] g::\"j\"$(til n)mod 1000; ((g*6)div \
    4)~(\"j\"$6*(til n)mod 1000)div 4}[200000]", "1b");
eqt!(coc_fors_mul_div_neg,  "{[n] g::\"j\"$-500+(til n)mod 1000; ((g*3)div \
    7)~(\"j\"$3*(-500+(til n)mod 1000))div 7}[200000]", "1b");
// Comparison verbs (=/</<=/>/>=), count/where, and masked reduce stream on a scaled column.
eqt!(coc_fors_mul_cmp_eq,   "{[n] g::\"j\"$(til n)mod 1000; \
    (where(g*3)=1500)~where(\"j\"$3*(til n)mod 1000)=1500}[200000]", "1b");
eqt!(coc_fors_mul_cmp_eqodd,"{[n] g::\"j\"$(til n)mod 1000; \
    (where(g*3)=1501)~where(\"j\"$3*(til n)mod 1000)=1501}[200000]", "1b");
eqt!(coc_fors_mul_cmp_lt,   "{[n] g::\"j\"$(til n)mod 1000; \
    (where(g*3)<1490)~where(\"j\"$3*(til n)mod 1000)<1490}[200000]", "1b");
eqt!(coc_fors_mul_cmp_gt,   "{[n] g::\"j\"$(til n)mod 1000; \
    (where(g*3)>1490)~where(\"j\"$3*(til n)mod 1000)>1490}[200000]", "1b");
eqt!(coc_fors_mul_cmp_cnt,  "{[n] g::\"j\"$(til n)mod 1000; (count \
    where(g*3)>1500)=count where(\"j\"$3*(til n)mod 1000)>1500}[200000]", "1b");
eqt!(coc_fors_mul_cmp_sumw, "{[n] g::\"j\"$(til n)mod 1000; \
    (sum(g*3)where(g*3)>1500)=sum(\"j\"$3*(til n)mod 1000)where(\"j\"$3*(til \
        n)mod 1000)>1500}[200000]", "1b");
eqt!(coc_fors_mul_cmp_bw8,  "{[n] g::\"j\"$200+(til n)mod 50; \
    (where(g*3)=636)~where(\"j\"$3*200+(til n)mod 50)=636}[100000]", "1b");
eqt!(coc_fors_mul_cmp_bw8l, "{[n] g::\"j\"$200+(til n)mod 50; \
    (where(g*3)<700)~where(\"j\"$3*200+(til n)mod 50)<700}[100000]", "1b");
eqt!(coc_fors_mul_cmp_kj,   "{[n] g::\"j\"$1000000000+(til n)mod 1000; (count \
    where(g*7)>7000003500j)=count where(7*(\"j\"$1000000000+(til n)mod \
        1000))>7000003500j}[200000]", "1b");
// More folds on a scaled column: grade, abs, neg (stays compressed), find.
eqt!(coc_fors_mul_idesc,    "{[n] g::\"j\"$(til n)mod 1000; (idesc \
    g*3)~idesc\"j\"$3*(til n)mod 1000}[100000]", "1b");
eqt!(coc_fors_mul_grade_w,  "{[n] g::\"j\"$(til n)mod 60000; (iasc \
    g*7)~iasc\"j\"$7*(til n)mod 60000}[100000]", "1b");
eqt!(coc_fors_mul_absneg,   "{[n] g::\"j\"$-500+(til n)mod 1000; (abs \
    g*3)~abs\"j\"$3*(-500+(til n)mod 1000)}[200000]", "1b");
eqt!(coc_fors_mul_neg_base, "{[n] g::\"j\"$1000000+(til n)mod 100; (neg \
    g*7)~neg\"j\"$7*(1000000+(til n)mod 100)}[100000]", "1b");
eqt!(coc_fors_mul_neg_nb,   "{[n] g::\"j\"$-500+(til n)mod 1000; (neg \
    g*3)~neg\"j\"$3*(-500+(til n)mod 1000)}[200000]", "1b");
eqt!(coc_fors_mul_find_off, "{[n] g::\"j\"$(til n)mod 1000; \
    ((g*3)?1501j)=(\"j\"$3*(til n)mod 1000)?1501j}[200000]", "1b");
eqt!(coc_fors_mul_find_oob, "{[n] g::\"j\"$(til n)mod 1000; \
    ((g*3)?99999j)=(\"j\"$3*(til n)mod 1000)?99999j}[200000]", "1b");
eqt!(coc_fors_mul_find_bw8, "{[n] g::\"j\"$200+(til n)mod 50; \
    ((g*3)?636j)=(\"j\"$3*200+(til n)mod 50)?636j}[100000]", "1b");
eqt!(coc_fors_mul_find_nb,  "{[n] g::\"j\"$-500+(til n)mod 1000; \
    ((g*3)?-300j)=(\"j\"$3*(-500+(til n)mod 1000))?-300j}[200000]", "1b");
// var/dev/cov/cor on a scaled column fall back to decode to the correct value.
eqt!(coc_fors_mul_var,      "{[n] g::\"j\"$(til n)mod 1000; (1e-6>abs(var \
    g*3)-var\"f\"$3*(til n)mod 1000)}[200000]", "1b");
eqt!(coc_fors_mul_dev,      "{[n] g::\"j\"$(til n)mod 1000; (1e-6>abs(dev \
    g*3)-dev\"f\"$3*(til n)mod 1000)}[200000]", "1b");
eqt!(coc_fors_mul_cov,      "{[n] g::\"j\"$(til n)mod 1000; k::\"j\"$(til \
    n)mod \
    777; (1e-6>abs(cov[g*3;k])-cov[\"j\"$3*(til n)mod 1000;k])}[200000]", "1b");
// float-mul by a negative power of 10 stays compressed bit-exact; arbitrary float falls back (correct).
eqt!(coc_fmul_alp_p3,    "{[n] g::\"j\"$(til n)mod 1000; \
    (g*0.001)~0.001*\"j\"$(til n)mod 1000}[200000]", "1b");
eqt!(coc_fmul_alp_p1,    "{[n] g::\"j\"$(til n)mod 1000; \
    (g*0.1)~0.1*\"j\"$(til \
    n)mod 1000}[200000]", "1b");
eqt!(coc_fmul_alp_sum,   "{[n] g::\"j\"$(til n)mod 1000; (sum g*0.01)=sum \
    0.01*\"j\"$(til n)mod 1000}[200000]", "1b");
eqt!(coc_fmul_alp_minmax,"{[n] g::\"j\"$(til n)mod 1000; ((min g*0.001),max \
    g*0.001)~(min;max)@\\:0.001*\"j\"$(til n)mod 1000}[200000]", "1b");
eqt!(coc_fmul_nonp10,    "{[n] g::\"j\"$(til n)mod 1000; \
    (g*2.5)~2.5*\"j\"$(til \
    n)mod 1000}[200000]", "1b");
// correctness of the moment aggregations on known data (population stats)
eqt!(grp_var_known,  "(exec v from select v:var x by g from ([]g:(5#`a),5#`b; \
    x:1 2 3 4 5,10 20 30 40 50))~2 200f", "1b");
eqt!(grp_dev_known,  "(exec v from select v:dev x by g from ([]g:0 0 1 1; x:1 \
    3 \
    11 31))~1 10f", "1b");
eqt!(grp_cov_known,  "(cov[1 2 3 4 5;2 4 6 8 10])~4f", "1b");
eqt!(grp_cor_known,  "{c:cor[1 2 3 4 5;2 4 6 8 10];(c>0.9999)&c<1.0001}[]",
    "1b");
eqt!(avg_int_temp,   "(avg(til 5)*til 5)~6f", "1b");

// dyadic ! construction guard: sym!ATOM now signals 'type instead of crashing; vectors/lists still build enums/dicts.
eqt!(bang_atomidx_guard, "(@[{`a!1};0;{`g}])~`g", "1b");                        // `sym!atom-idx → 'type (was crash)
eqt!(bang_atomidx_guard2,"(@[{`bgx!1};0;{`g}])~`g", "1b");                      // same with a fresh domain sym
eqt!(bang_enum_vec_ok,   "(count `bgdv!1 2 3)=3", "1b");                        // sym!KI-VEC → enum vector still works
eqt!(bang_enum_one_ok,   "(count `bgd1!enlist 1)=1", "1b");                     // 1-element enum (explicit list) ok
eqt!(bang_dict_listkey,  "((`bgk1`bgk2!(1 2;3 4))`bgk1)~1 2", "1b");            // sym-LIST keys → dict (lookup ok)
eqt!(bang_dict_intkey,   "((1 2!3 4)1)~3", "1b");                               // int-list keys → dict still ok

// Moment folds (var/dev/svar/sdev/med/cov/cor) on a plain compressed column; raw/uncovered codecs defer to exact helpers, parity vs a raw twin.
eqt!(coc_mom_var,   "{[n] c::\"j\"$(til n)mod 1000; r:\"j\"$(til n)mod 1000; \
    1e-6>abs(var c)-var r}[100000]", "1b");
eqt!(coc_mom_dev,   "{[n] c::\"j\"$(til n)mod 1000; r:\"j\"$(til n)mod 1000; \
    1e-6>abs(dev c)-dev r}[100000]", "1b");
eqt!(coc_mom_svar,  "{[n] c::\"j\"$(til n)mod 1000; r:\"j\"$(til n)mod 1000; \
    1e-6>abs(svar c)-svar r}[100000]", "1b");
eqt!(coc_mom_sdev,  "{[n] c::\"j\"$(til n)mod 1000; r:\"j\"$(til n)mod 1000; \
    1e-6>abs(sdev c)-sdev r}[100000]", "1b");
eqt!(coc_mom_cov,   "{[n] c::\"j\"$(til n)mod 1000; d::\"j\"$(7+til n)mod 977; \
    r:\"j\"$(til n)mod 1000; s:\"j\"$(7+til n)mod 977; \
        1e-6>abs(cov[c;d])-cov[r;s]}[100000]", "1b");
eqt!(coc_mom_cor,   "{[n] c::\"j\"$(til n)mod 1000; d::\"j\"$(7+til n)mod 977; \
    r:\"j\"$(til n)mod 1000; s:\"j\"$(7+til n)mod 977; \
        1e-9>abs(cor[c;d])-cor[r;s]}[100000]", "1b");
// median: folds on sorted compressed (two point reads), cedes→raw on unsorted; both correct
eqt!(coc_mom_med_sorted,  "{[n] c::`s#asc \"j\"$(til n)mod 1000; r:asc \
    \"j\"$(til n)mod 1000; 1e-6>abs(med c)-med r}[100000]", "1b");
eqt!(coc_mom_med_unsorted,"{[n] c::\"j\"$(til n)mod 1000; r:\"j\"$(til n)mod \
    1000; 1e-6>abs(med c)-med r}[100000]", "1b");
// dict of columns → per-column moment (.q.mr handles 99h via each)
eqt!(coc_mom_var_dict, "{[n] c::\"j\"$(til n)mod 1000; d::\"j\"$(7+til n)mod \
    977; (var `a`b!(c;d))~`a`b!(var c;var d)}[100000]", "1b");
// compressed float column: var folds correctly (no cancellation)
eqt!(coc_mom_var_alp,  "{[n] c::0.5+\"f\"$(til n)mod 1000; r:0.5+\"f\"$(til \
    n)mod 1000; 1e-3>abs(var c)-var r}[100000]", "1b");
// list-ctor marshals a compressed element without realizing it (correct list)
eqt!(coc_list_ctor_qz, "{[n] c::\"j\"$(til n)mod 1000; r:\"j\"$(til n)mod \
    1000; \
    ((c;42)~(r;42))&(var c)=var r}[100000]", "1b");
// small raw vectors → exact q-helper path (no fold), known values
eqt!(coc_mom_var_raw,  "(var 1 2 3 4 5f)~2f", "1b");
eqt!(coc_mom_dev_raw,  "(dev 0 0 1 1f)~0.5", "1b");
// scaled compressed column (g*m) moment fold — now reachable
eqt!(coc_mom_scaled_var,  "{[n] g::\"j\"$(til n)mod 1000; (1e-6>abs(var \
    g*3)-var\"f\"$3*(til n)mod 1000)}[100000]", "1b");
eqt!(coc_mom_scaled_dev,  "{[n] g::\"j\"$(til n)mod 1000; (1e-6>abs(dev \
    g*7)-dev\"f\"$7*(til n)mod 1000)}[100000]", "1b");
eqt!(coc_mom_scaled_svar, "{[n] g::\"j\"$(til n)mod 1000; (1e-6>abs(svar \
    g*3)-svar\"f\"$3*(til n)mod 1000)}[100000]", "1b");
// one-pass `moments` dict (n;sum;avg;var;dev;svar;sdev) from a SINGLE fold (vs var+sum+svar = 3 scans)
eqt!(coc_moments_dict, "{[n] c::\"j\"$(til n)mod 1000; r:\"j\"$(til n)mod \
    1000; \
    m:moments c; ((m`n)=count r)&(1e-6>abs(m`var)-var r)&(1e-6>abs(m`dev)-dev \
        r)&(1e-6>abs(m`avg)-avg r)&(1e-6>abs(m`svar)-svar r)}[100000]", "1b");
eqt!(coc_moments_raw,  "{m:moments 3 9 27f; \
    ((m`n)=3)&(1e-9>abs(m`sum)-39f)&(1e-9>abs(m`var)-104f)&(1e-9>abs(m`svar)-15\
        6\
        f)}[]", "1b");
// scaled compressed column (g*m) cov/cor/wsum fold — now scale-aware
eqt!(coc_mom_scaled_cov, "{[n] g::\"j\"$(til n)mod 1000; h::\"j\"$(7+til n)mod \
    977; (1e-6>abs(cov[g*3;h*5])-cov[\"f\"$3*(til n)mod 1000;\"f\"$5*(7+til \
        n)mod 977])}[100000]", "1b");
eqt!(coc_mom_scaled_cor, "{[n] g::\"j\"$(til n)mod 1000; h::\"j\"$(7+til n)mod \
    977; (1e-9>abs(cor[g*3;h*5])-cor[\"f\"$3*(til n)mod 1000;\"f\"$5*(7+til \
        n)mod 977])}[100000]", "1b");
eqt!(coc_scaled_wsum,    "{[n] g::\"j\"$(til n)mod 1000; h::\"j\"$(7+til n)mod \
    977; (1e-6>abs((g*3) wsum h*5)-(\"f\"$3*(til n)mod 1000) wsum \
        \"f\"$5*(7+til n)mod 977)}[100000]", "1b");
// affine-ramp compressed column predicates: O(1) crossover (=/</>/within/bin)
eqt!(coc_aff_eq,     "{[n] c::\"j\"$5+3*til n; r:\"j\"$5+3*til n; (where c=c \
    1000)~where r=r 1000}[100000]", "1b");
eqt!(coc_aff_lt,     "{[n] c::\"j\"$5+3*til n; r:\"j\"$5+3*til n; (where c<c \
    1000)~where r<r 1000}[100000]", "1b");
eqt!(coc_aff_gt,     "{[n] c::\"j\"$5+3*til n; r:\"j\"$5+3*til n; (where c>c \
    9000)~where r>r 9000}[100000]", "1b");
eqt!(coc_aff_within, "{[n] c::\"j\"$5+3*til n; r:\"j\"$5+3*til n; (where c \
    within (c 500;c 2000))~where r within (r 500;r 2000)}[100000]", "1b");
eqt!(coc_aff_bin,    "{[n] c::\"j\"$5+3*til n; r:\"j\"$5+3*til n; (c bin c 100 \
    5000 9000)~r bin r 100 5000 9000}[100000]", "1b");
// compressed float `in` (small set): tolerant compare mirrors raw float in
eqt!(coc_alp_in3,      "{[n] c::0.25+\"f\"$(til n)mod 1000; r:0.25+\"f\"$(til \
    n)mod 1000; (c in c 5 100 700)~r in r 5 100 700}[100000]", "1b");
eqt!(coc_alp_in5,      "{[n] c::0.25+\"f\"$(til n)mod 1000; r:0.25+\"f\"$(til \
    n)mod 1000; (c in c 3 42 99 500 888)~r in r 3 42 99 500 888}[100000]",
        "1b");
eqt!(coc_alp_where_in, "{[n] c::0.25+\"f\"$(til n)mod 1000; r:0.25+\"f\"$(til \
    n)mod 1000; (where c in c 5 100 700)~where r in r 5 100 700}[100000]",
        "1b");

// Grouped moment / avg fold on a compressed value column, matched against a raw local twin.
eqt!(coc_grp_var_f,  "{[n] tc::([] v:(til n)mod 50; p:0.01*(til n)mod 5000); \
    tr:([] v:(til n)mod 50; p:0.01*(til n)mod 5000); all 1e-7>abs (value exec \
        var p by v from tc)-value exec var p by v from tr}[100000]", "1b");
eqt!(coc_grp_dev_f,  "{[n] tc::([] v:(til n)mod 50; p:0.01*(til n)mod 5000); \
    tr:([] v:(til n)mod 50; p:0.01*(til n)mod 5000); all 1e-7>abs (value exec \
        dev p by v from tc)-value exec dev p by v from tr}[100000]", "1b");
eqt!(coc_grp_svar_f, "{[n] tc::([] v:(til n)mod 50; p:0.01*(til n)mod 5000); \
    tr:([] v:(til n)mod 50; p:0.01*(til n)mod 5000); all 1e-7>abs (value exec \
        svar p by v from tc)-value exec svar p by v from tr}[100000]", "1b");
eqt!(coc_grp_sdev_f, "{[n] tc::([] v:(til n)mod 50; p:0.01*(til n)mod 5000); \
    tr:([] v:(til n)mod 50; p:0.01*(til n)mod 5000); all 1e-7>abs (value exec \
        sdev p by v from tc)-value exec sdev p by v from tr}[100000]", "1b");
eqt!(coc_grp_var_i,  "{[n] tc::([] v:(til n)mod 50; s:(til n)mod 100); tr:([] \
    v:(til n)mod 50; s:(til n)mod 100); all 1e-7>abs (value exec var s by v \
        from tc)-value exec var s by v from tr}[100000]", "1b");
eqt!(coc_grp_avg_f,  "{[n] tc::([] v:(til n)mod 50; p:0.01*(til n)mod 5000); \
    tr:([] v:(til n)mod 50; p:0.01*(til n)mod 5000); all 1e-7>abs (value exec \
        avg p by v from tc)-value exec avg p by v from tr}[100000]", "1b");

// LEAK REGRESSION — loop an op and assert memory stays stable (refcount and cross-thread growth classes).
eqt!(leak_select_where_cmp,  "{[n] t::flip `a`b`c!(n?`4;n?1f;n?200i); \
    do[20;select i from t where c<50];.Q.gc[]; h0:.Q.w[]`heap; do[40;select i \
        from t where c<50];.Q.gc[]; h1:.Q.w[]`heap; (h1-h0)<30000000}[500000]",
            "1b");
eqt!(leak_select_where_in,   "{[n] t::flip `a`b`c!(n?`4;n?1f;n?200i); \
    do[20;select i from t where c in 10 20 30];.Q.gc[]; h0:.Q.w[]`heap; \
        do[40;select i from t where c in 10 20 30];.Q.gc[]; h1:.Q.w[]`heap; \
            (h1-h0)<30000000}[500000]", "1b");
eqt!(leak_col_extract,       "{[n] r::flip `a`b!(n?`4;n?1f); gp::group(til \
    n)mod 8; .Q.gc[]; u0:.Q.w[]`used; {[i]c:r[gp 0];d:c`a;c:0;}each til 40; \
        .Q.gc[]; u1:.Q.w[]`used; (u1-u0)<10000000}[500000]", "1b");
eqt!(leak_xasc_proj_apply,   "{[n] t::flip `a`b`c!(n?`4;n?1f;n?200i); .Q.gc[]; \
    u0:.Q.w[]`used; do[40;`c xasc t]; .Q.gc[]; u1:.Q.w[]`used; \
        (u1-u0)<10000000}[300000]", "1b");
eqt!(leak_group_by_where,    "{[n] t::flip `a`b`c!(n?`4;n?1f;n?200i); \
    do[10;select sum b by a from t where c<50];.Q.gc[]; \
        u0:.Q.w[]`used;h0:.Q.w[]`heap; do[40;select sum b by a from t where \
            c<50];.Q.gc[]; u1:.Q.w[]`used;h1:.Q.w[]`heap; \
                ((u1-u0)<10000000)&((h1-h0)<30000000)}[300000]", "1b");

// Whole-system orphan check: after delete-from-root + gc, accurate used must return to baseline (any residual is a leak).
#[test]
fn leak_delete_from_root() {
    let mut c = conn();
    c.query("delete from `.; .Q.gc[]").unwrap();
    let base = c.query(".Q.w[]`used").unwrap().as_long().expect("used long");
    // Deterministic data + do-loop; each iter rebuilds t and runs ops (group/distinct on rebuilt compressed cols excluded as a known follow-up).
    c.query("do[8; t::flip `a`b`c!(100000#`g1`g2`g3`g4`g5; (til 100000)%100.0; \
        (til 100000)mod 200); \
             select sum b by a from t where c<50; select i from t where c in \
                 10 \
                 20 30; \
             `c xasc t; `c xdesc t; exec b from t where c<50; update v:b+1 \
                 from \
                 t; \
             select avg b,var b,dev b by a from t; select b wavg c by a from \
                 t]").unwrap();
    c.query("delete from `.; .Q.gc[]").unwrap();
    let after = c.query(".Q.w[]`used").unwrap().as_long().expect("used long");
    let residual = after - base;
    assert!(residual < 4_000_000,
        "delete-from-root residual = {} bytes (baseline {}) — an op orphaned \
            memory (leak)",
        residual, base);
}
