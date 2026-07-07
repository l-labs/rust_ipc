//! Hostile-input and robustness tests for the l-rs IPC client.
//!
//! The type-matrix suite in `ipc_test.rs` proves the *happy path*: a real
//! server, well-formed frames, every type echoed. This file attacks the other
//! side — what a corrupt, malicious, or half-dead server can do to the client.
//! Nothing here may panic, abort, or allocate without bound; a broken peer
//! must always surface as a clean `Err`.
//!
//! Four classes:
//!   1. malformed-frame decoding (a fake in-test TCP server feeds crafted
//!      bytes: truncated / oversized / negative lengths, unknown tags, depth
//!      bombs, corrupt LZ4, bad UTF-8, zero-length frames);
//!   2. robustness vs a real server (kill mid-session + reconnect, no fd leak,
//!      blackhole-connect finding);
//!   3. seeded property round-trips (serialize -> deserialize, in-process);
//!   4. serializer bounds (max symbol, count overflow, empty/deep-empty).
//!
//! Server-backed tests need `l` on PATH or `$L_BIN`; the rest are self-hosted.

#![allow(non_snake_case)]

use l_rs::serialize::{deserialize, serialize};
use l_rs::types::{INF_FLOAT, INF_INT, INF_LONG, NULL_INT, NULL_LONG,
                  NULL_SHORT};
use l_rs::{Connection, K, LError};

use std::alloc::{GlobalAlloc, Layout, System};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};
use std::path::PathBuf;
use std::process::Child;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::{env, thread};

// ═══════════════════════════════════════════════════════════════════════════
// Allocation guard — a counting global allocator so a hostile frame that tries
// to size a giant buffer is caught as an *assertion*, not an OOM abort. `alloc`
// AND `alloc_zeroed` are tracked (a `vec![0u8; n]` takes the zeroed path); the
// default `realloc` routes through both, so `Vec` growth is counted too.
// ═══════════════════════════════════════════════════════════════════════════
struct Counting;
static CUR: AtomicUsize = AtomicUsize::new(0);                                  // live bytes
static PEAK: AtomicUsize = AtomicUsize::new(0);                                 // high-water mark

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = System.alloc(l);
        if !p.is_null() { bump(l.size()); }
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        let p = System.alloc_zeroed(l);
        if !p.is_null() { bump(l.size()); }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        CUR.fetch_sub(l.size(), Ordering::Relaxed);
        System.dealloc(p, l);
    }
}
fn bump(n: usize) {                                                            // raise live + peak
    let c = CUR.fetch_add(n, Ordering::Relaxed) + n;
    PEAK.fetch_max(c, Ordering::Relaxed);
}

#[global_allocator]
static ALLOC: Counting = Counting;

// Serialize the measured windows so two probes don't clobber each other's peak
// reset. Concurrent unmeasured tests add only KB-scale noise, far below caps.
static MEASURE: Mutex<()> = Mutex::new(());

/// Run `f`, returning (its result, transient bytes it peaked at) and catching
/// any panic. A stack-overflow abort is uncatchable — but every decode path
/// here is bounded, so a regression that reintroduced one would crash loudly.
fn probe<R>(f: impl FnOnce() -> R)
    -> (std::thread::Result<R>, usize) {
    let _lk = MEASURE.lock().unwrap_or_else(|e| e.into_inner());
    let base = CUR.load(Ordering::Relaxed);
    PEAK.store(base, Ordering::Relaxed);
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    let pk = PEAK.load(Ordering::Relaxed).saturating_sub(base);
    (r, pk)
}

const ALLOC_CAP: usize = 64 << 20;                                            // 64 MiB guard

// ═══════════════════════════════════════════════════════════════════════════
// Fake server — speaks the minimal handshake, then feeds crafted reply bytes.
// ═══════════════════════════════════════════════════════════════════════════
/// Accept one connection, read the client's creds (up to the NUL), optionally
/// send the 1-byte ack, then write `reply` verbatim and lingering-close so the
/// FIN (not an RST) delivers a clean EOF even with the request left unread.
fn serve_once(reply: Vec<u8>, send_ack: bool) -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    let port = l.local_addr().unwrap().port();
    thread::spawn(move || {
        if let Ok((mut s, _)) = l.accept() {
            let _ = s.set_read_timeout(Some(Duration::from_millis(500)));
            let mut one = [0u8; 1];
            loop {                                                            // creds up to NUL
                match s.read(&mut one) {
                    Ok(0) => return,
                    Ok(_) => if one[0] == 0 { break; },
                    Err(_) => break,
                }
            }
            if !send_ack { return; }                                          // reject handshake
            let _ = s.write_all(&[0u8]);                                      // handshake ack
            let _ = s.flush();
            let _ = s.write_all(&reply);                                      // crafted response
            let _ = s.flush();
            let _ = s.shutdown(Shutdown::Write);                              // FIN -> client EOF
            let mut junk = [0u8; 256];                                        // drain to avoid RST
            let end = Instant::now() + Duration::from_millis(500);
            while Instant::now() < end {
                match s.read(&mut junk) { Ok(0) | Err(_) => break, _ => {} }
            }
        }
    });
    port
}

/// Build a well-framed message: 8-byte header (LE, response, no LZ4) + body,
/// with the length field set correctly to `8 + body.len()`.
fn frame(body: &[u8]) -> Vec<u8> {
    let mut v = vec![1u8, 2, 0, 0];
    v.extend_from_slice(&((8 + body.len()) as i32).to_le_bytes());
    v.extend_from_slice(body);
    v
}
/// Frame with an *arbitrary* declared length (to lie about payload size), and
/// a chosen LZ4 flag.
fn frame_claim(total: i32, body: &[u8], lz4: bool) -> Vec<u8> {
    let mut v = vec![1u8, 2, lz4 as u8, 0];
    v.extend_from_slice(&total.to_le_bytes());
    v.extend_from_slice(body);
    v
}
/// A compressed frame: body = `[raw inflated-size i32][LZ4 block]`, LZ4 flag on.
fn lz4_frame(raw: i32, block: &[u8]) -> Vec<u8> {
    let mut body = raw.to_le_bytes().to_vec();
    body.extend_from_slice(block);
    frame_claim(8 + body.len() as i32, &body, true)
}

/// Connect to the fake server on `port` and run one query; the whole thing is
/// probed for panics + peak allocation.
fn talk(port: u16) -> (std::thread::Result<Result<K, LError>>, usize) {
    probe(move || -> Result<K, LError> {
        let mut c = Connection::connect("127.0.0.1", port)?;
        c.query("2+2")
    })
}

/// The core assertion for a hostile frame: no panic, a clean `Err`, bounded
/// allocation.
fn expect_clean_err(reply: Vec<u8>, name: &str) {
    let port = serve_once(reply, true);
    let (res, peak) = talk(port);
    assert!(res.is_ok(), "{name}: client PANICKED (must return Err)");
    assert!(res.unwrap().is_err(), "{name}: expected Err, got Ok");
    assert!(peak < ALLOC_CAP, "{name}: unbounded alloc {peak} bytes");
}

// ── 1a. length-field abuse ──────────────────────────────────────────────────
#[test]
fn mf_truncated_header() {                                                    // 3 of 8 header bytes
    expect_clean_err(vec![1, 2, 0], "truncated_header");
}

#[test]
fn mf_truncated_payload_bounded() {                                           // claims 1KB, sends 16B
    expect_clean_err(frame_claim(8 + 1024, &[0u8; 16], false),
        "truncated_payload");
}

#[test]
fn mf_len_below_minimum() {                                                   // < 8-byte header
    for t in [0i32, 1, 7] {
        expect_clean_err(frame_claim(t, &[], false), "len_below_min");
    }
}

#[test]
fn mf_len_negative() {                                                        // -1 cast -> ~18 EB
    expect_clean_err(frame_claim(-1, &[], false), "len_negative");
    expect_clean_err(frame_claim(i32::MIN, &[], false), "len_i32_min");
}

#[test]
fn mf_len_over_cap() {                                                        // 300 MiB > 256 MiB cap
    expect_clean_err(frame_claim(8 + (300 << 20), &[], false), "len_over_cap");
}

#[test]
fn mf_len_i32_max_no_multi_gb_alloc() {                                       // ~2 GiB: THE guard
    let (res, peak) = { let p = serve_once(
        frame_claim(i32::MAX, &[], false), true); talk(p) };
    assert!(res.is_ok(), "len_i32_max: PANICKED");
    assert!(res.unwrap().is_err(), "len_i32_max: expected Err");
    assert!(peak < ALLOC_CAP,                                                 // must reject pre-alloc
        "len_i32_max: attempted {peak}-byte (multi-GB) allocation");
}

#[test]
fn mf_zero_length_frame() {                                                   // total==8, empty body
    expect_clean_err(frame(&[]), "zero_length_frame");
}

// ── 1b. bad tags & counts ───────────────────────────────────────────────────
#[test]
fn mf_unknown_type_tags() {                                                   // tags we don't model
    for t in [3u8, 20, 50, 97, 100, 127, 200] {
        expect_clean_err(frame(&[t]), "unknown_tag");
    }
}

#[test]
fn mf_negative_vector_count() {                                              // list count = -1
    expect_clean_err(frame(&[0, 0, 0xff, 0xff, 0xff, 0xff]), "neg_count");
}

#[test]
fn mf_huge_vector_count() {                                                  // list count = i32::MAX
    expect_clean_err(frame(&[0, 0, 0xff, 0xff, 0xff, 0x7f]), "huge_count");
}

#[test]
fn mf_huge_symbol_count() {                                                  // sym-vec count = i32::MAX
    expect_clean_err(frame(&[11, 0, 0xff, 0xff, 0xff, 0x7f]), "huge_sym");
}

#[test]
fn mf_nested_huge_count() {                                                  // list[ intvec(i32::MAX) ]
    expect_clean_err(
        frame(&[0, 0, 1, 0, 0, 0, 6, 0, 0xff, 0xff, 0xff, 0x7f]),
        "nested_huge_count");
}

/// Raw wire bytes for `levels` nested single-element lists around an int leaf
/// — a depth bomb built directly, never through the (also-recursive) writer.
fn nested_list_bytes(levels: usize) -> Vec<u8> {
    let mut body = Vec::new();
    for _ in 0..levels { body.extend_from_slice(&[0, 0, 1, 0, 0, 0]); }       // list, count 1
    body.extend_from_slice(&[0xfa, 0, 0, 0, 0]);                             // leaf int 0
    body
}

// ── 1c. depth bomb — 10k-deep list-of-list must NOT overflow the stack ───────
#[test]
fn mf_depth_bomb_10k() {
    expect_clean_err(frame(&nested_list_bytes(10_000)), "depth_bomb");
}

// ── 1d. corrupt / truncated LZ4 ─────────────────────────────────────────────
#[test]
fn mf_lz4_truncated_litlen() {                                              // 0xF0 spills past end
    expect_clean_err(lz4_frame(100, &[0xF0]), "lz4_trunc_litlen");
}

#[test]
fn mf_lz4_truncated_offset() {                                             // offset byte missing
    expect_clean_err(lz4_frame(100, &[0x00, 0x05]), "lz4_trunc_offset");
}

#[test]
fn mf_lz4_truncated_matchlen() {                                           // ml=19 spill past end
    expect_clean_err(lz4_frame(100, &[0x1F, 0x41, 0x01, 0x00]),
        "lz4_trunc_matchlen");
}

#[test]
fn mf_lz4_invalid_offset() {                                               // back-ref before start
    expect_clean_err(lz4_frame(10, &[0x00, 0x01, 0x00]), "lz4_bad_offset");
}

#[test]
fn mf_lz4_raw_size_negative() {                                            // inflated size < 0
    expect_clean_err(lz4_frame(-1, &[0x00]), "lz4_raw_neg");
}

#[test]
fn mf_lz4_raw_size_over_cap() {                                            // inflated size > cap
    expect_clean_err(lz4_frame(300 << 20, &[0x00]), "lz4_raw_cap");
}

// ── 1e. bad UTF-8 in a symbol — DECISION: lossy, not an error ────────────────
#[test]
fn mf_bad_utf8_symbol_is_lossy_not_panic() {
    // Invalid UTF-8 in a symbol is decoded with U+FFFD substitution (see
    // `rd_sym`): the client returns Ok, not Err. That is a deliberate, doc'd
    // choice — a symbol should never be a hard transport failure. We only
    // require: no panic, bounded allocation.
    let port = serve_once(frame(&[0xf5, 0xff, 0xfe, 0x00]), true);           // -11 sym, bad bytes
    let (res, peak) = talk(port);
    assert!(res.is_ok(), "bad_utf8: client PANICKED");
    assert!(peak < ALLOC_CAP, "bad_utf8: unbounded alloc {peak}");
}

// ── 1f. handshake that closes before the ack ─────────────────────────────────
#[test]
fn mf_handshake_closed_no_ack() {
    let port = serve_once(vec![], false);
    let (res, _peak) = talk(port);
    assert!(res.is_ok(), "handshake_closed: PANICKED");
    assert!(res.unwrap().is_err(), "handshake_closed: expected Err");
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. Robustness against a REAL server (needs `l` / $L_BIN).
// ═══════════════════════════════════════════════════════════════════════════
struct Srv(Child);
impl Drop for Srv {
    fn drop(&mut self) { let _ = self.0.kill(); let _ = self.0.wait(); }
}
fn l_bin() -> PathBuf {
    env::var("L_BIN").map(PathBuf::from).unwrap_or_else(|_| "l".into())
}
fn spawn_l(port: u16) -> Srv {
    let child = std::process::Command::new(l_bin())
        .args(["-p", &port.to_string(), "-q"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn().expect("spawn l (set L_BIN or put l on PATH)");
    let mut srv = Srv(child);
    let end = Instant::now() + Duration::from_secs(5);
    while Instant::now() < end {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return srv;
        }
        thread::sleep(Duration::from_millis(50));
    }
    let _ = srv.0.kill();
    panic!("l on :{port} never started listening");
}

#[test]
fn rb_kill_server_midsession_then_reconnect() {
    // A server that dies mid-session must yield a clean Err on the next call,
    // and the client must be able to reconnect to a fresh instance. (The
    // *mid-large-response* cut is covered deterministically by the truncated /
    // oversized-payload frame tests above — a killed server that streamed a
    // partial big frame is exactly that shape.)
    let mut srv = spawn_l(9951);
    let mut c = Connection::connect("127.0.0.1", 9951).expect("connect");
    assert_eq!(c.query("2+2").unwrap().as_int(), Some(4));
    let _ = srv.0.kill();
    let _ = srv.0.wait();
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(||
        c.query("3+3")));
    assert!(res.is_ok(), "query on dead server PANICKED");
    assert!(res.unwrap().is_err(), "query on dead server should Err");
    let _srv2 = spawn_l(9952);                                               // reconnect
    let mut c2 = Connection::connect("127.0.0.1", 9952).expect("reconnect");
    assert_eq!(c2.query("6*7").unwrap().as_int(), Some(42));
}

/// Count this process's open file descriptors (works on macOS & Linux — both
/// expose /dev/fd). Reading it opens one transient fd, measured consistently.
fn open_fds() -> usize {
    std::fs::read_dir("/dev/fd").map(|d| d.count()).unwrap_or(0)
}

#[test]
fn rb_rapid_connect_drop_no_fd_leak() {
    let _srv = spawn_l(9953);
    { let _ = Connection::connect("127.0.0.1", 9953); }                       // warm up
    let before = open_fds();
    for _ in 0..500 {
        let c = Connection::connect("127.0.0.1", 9953).expect("rapid connect");
        drop(c);                                                             // Drop closes the socket
    }
    let after = open_fds();
    assert!(after <= before + 8,
        "fd leak: {before} -> {after} across 500 connect/drop cycles");
}

#[test]
#[ignore]                                                                     // OS-timed; never in CI
fn rb_connect_timeout_blackhole_FINDING() {
    // FINDING: `Connection::connect` / `connect_with_auth` call
    // `TcpStream::connect` with NO timeout, so a blackholed host blocks until
    // the OS TCP timeout (~75 s on macOS). The client exposes no connect-
    // timeout API. Per the task we DOCUMENT this instead of bolting one on;
    // the fix would be a `connect_timeout(host, port, Duration)` constructor
    // over `TcpStream::connect_timeout`. Ignored so it can't stall the suite.
    let t = Instant::now();
    let r = Connection::connect("10.255.255.1", 5591);
    assert!(r.is_err());
    eprintln!("blackhole connect returned Err after {:?} (OS-bounded, \
        not client-bounded)", t.elapsed());
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. Property round-trips — seeded random K, serialize -> deserialize == self.
// A tiny xorshift PRNG keeps the crate dependency-free (as the README boasts).
// ═══════════════════════════════════════════════════════════════════════════
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self { Rng(seed ^ 0x9E37_79B9_7F4A_7C15) }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13; x ^= x >> 7; x ^= x << 17;
        self.0 = x; x
    }
    fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next() % n }
    }
    fn bit(&mut self) -> bool { self.next() & 1 == 1 }
}

fn gen_sym(r: &mut Rng) -> String {                                          // [a-z0-9], 0..7, no NUL
    let n = r.below(8) as usize;
    (0..n).map(|_| {
        b"abcdefghijklmnopqrstuvwxyz0123456789"[r.below(36) as usize] as char
    }).collect()
}
fn nul_i32(r: &mut Rng) -> i32 {                                             // sprinkle int nulls
    if r.below(8) == 0 { NULL_INT } else { r.next() as i32 }
}
fn nul_i64(r: &mut Rng) -> i64 {                                             // sprinkle long nulls
    if r.below(8) == 0 { NULL_LONG } else { r.next() as i64 }
}

fn gen_atom(r: &mut Rng) -> K {
    match r.below(20) {
        0 => K::Bool(r.bit()),
        1 => K::Byte(r.next() as u8),
        2 => K::Short(r.next() as i16),
        3 => K::Int(nul_i32(r)),
        4 => K::Long(nul_i64(r)),
        5 => K::Real(f32::from_bits(r.next() as u32)),                        // any bits incl NaN
        6 => K::Float(f64::from_bits(r.next())),
        7 => K::Char(r.next() as u8),
        8 => K::Symbol(gen_sym(r)),
        9 => K::Timestamp(r.next() as i64),
        10 => K::Month(r.next() as i32),
        11 => K::Date(r.next() as i32),
        12 => K::DateTime(f64::from_bits(r.next())),
        13 => K::Minute(r.next() as i32),
        14 => K::Second(r.next() as i32),
        15 => K::Time(r.next() as i32),
        16 => K::Timespan(r.next() as i64),
        17 => K::Error(gen_sym(r)),
        18 => K::Null,
        _ => K::Int(r.next() as i32),
    }
}

fn gen_vec(r: &mut Rng) -> K {
    let n = r.below(33) as usize;                                            // length 0..32
    macro_rules! v { ($e:expr) => { (0..n).map(|_| $e).collect() } }
    match r.below(17) {
        0 => K::BoolVec(v!(r.bit())),
        1 => K::ByteVec(v!(r.next() as u8)),
        2 => K::ShortVec(v!(r.next() as i16)),
        3 => K::IntVec(v!(nul_i32(r))),
        4 => K::LongVec(v!(nul_i64(r))),
        5 => K::RealVec(v!(f32::from_bits(r.next() as u32))),
        6 => K::FloatVec(v!(f64::from_bits(r.next()))),
        7 => K::CharVec(v!(r.next() as u8)),
        8 => K::SymbolVec(v!(gen_sym(r))),
        9 => K::TimestampVec(v!(r.next() as i64)),
        10 => K::MonthVec(v!(r.next() as i32)),
        11 => K::DateVec(v!(r.next() as i32)),
        12 => K::DateTimeVec(v!(f64::from_bits(r.next()))),
        13 => K::MinuteVec(v!(r.next() as i32)),
        14 => K::SecondVec(v!(r.next() as i32)),
        15 => K::TimeVec(v!(r.next() as i32)),
        _ => K::TimespanVec(v!(r.next() as i64)),
    }
}

fn gen_k(r: &mut Rng, depth: u32) -> K {
    let c = if depth == 0 { r.below(2) } else { r.below(4) };
    match c {
        0 => gen_atom(r),
        1 => gen_vec(r),
        2 => {                                                              // mixed list
            let n = r.below(5) as usize;
            K::List((0..n).map(|_| gen_k(r, depth - 1)).collect())
        }
        _ if r.bit() => {                                                   // dict: syms ! mixed
            let n = r.below(5) as usize;
            let keys = K::SymbolVec((0..n).map(|_| gen_sym(r)).collect());
            let vals = K::List((0..n).map(|_| gen_k(r, depth - 1)).collect());
            K::Dict(Box::new(keys), Box::new(vals))
        }
        _ => {                                                             // table: cols ! vec-list
            let ncol = 1 + r.below(3) as usize;
            let keys = K::SymbolVec((0..ncol).map(|_| gen_sym(r)).collect());
            let vals = K::List((0..ncol).map(|_| gen_vec(r)).collect());
            K::Table(Box::new(K::Dict(Box::new(keys), Box::new(vals))))
        }
    }
}

/// Structural equality that treats floats bit-for-bit, so NaN nulls (and NaN
/// payloads) count as equal iff the codec preserved the exact bits.
fn k_eq(a: &K, b: &K) -> bool {
    use K::*;
    fn fv(x: &[f64], y: &[f64]) -> bool {
        x.len() == y.len()
            && x.iter().zip(y).all(|(p, q)| p.to_bits() == q.to_bits())
    }
    match (a, b) {
        (Real(x), Real(y)) => x.to_bits() == y.to_bits(),
        (Float(x), Float(y)) | (DateTime(x), DateTime(y)) =>
            x.to_bits() == y.to_bits(),
        (RealVec(x), RealVec(y)) => x.len() == y.len()
            && x.iter().zip(y).all(|(p, q)| p.to_bits() == q.to_bits()),
        (FloatVec(x), FloatVec(y)) | (DateTimeVec(x), DateTimeVec(y)) =>
            fv(x, y),
        (List(x), List(y)) => x.len() == y.len()
            && x.iter().zip(y).all(|(p, q)| k_eq(p, q)),
        (Dict(k1, v1), Dict(k2, v2)) => k_eq(k1, k2) && k_eq(v1, v2),
        (Table(d1), Table(d2)) => k_eq(d1, d2),
        _ => a == b,
    }
}

fn rt(k: &K) -> K { deserialize(&serialize(k).unwrap()).unwrap() }

#[test]
fn prop_roundtrip_seeded() {
    let n: u64 = if env::var("L_STRESS").is_ok() { 5000 } else { 200 };
    for seed in 0..n {
        let s = seed.wrapping_mul(0x0100_0000_01b3).wrapping_add(1);          // FNV-ish spread
        let mut r = Rng::new(s);
        let k = gen_k(&mut r, 3);                                            // nested to depth 3
        let back = rt(&k);
        assert!(k_eq(&k, &back),
            "seed {seed}: round-trip mismatch\n in={k:?}\nout={back:?}");
    }
}

#[test]
fn prop_atoms_all_types_incl_null() {
    let cases = [
        K::Bool(true), K::Bool(false), K::Byte(0), K::Byte(255),
        K::Short(NULL_SHORT), K::Short(i16::MAX), K::Int(NULL_INT),
        K::Int(INF_INT), K::Int(0), K::Long(NULL_LONG), K::Long(INF_LONG),
        K::Real(f32::NAN), K::Real(f32::INFINITY), K::Real(-0.0),
        K::Float(f64::NAN), K::Float(INF_FLOAT), K::Float(-0.0), K::Char(0),
        K::Symbol("".into()), K::Symbol("abc".into()),
        K::Timestamp(NULL_LONG), K::Month(NULL_INT), K::Date(0),
        K::DateTime(f64::NAN), K::Minute(0), K::Second(0), K::Time(0),
        K::Timespan(NULL_LONG), K::Error("type".into()), K::Null,
    ];
    for k in &cases {
        assert!(k_eq(k, &rt(k)), "atom mismatch: {k:?}");
    }
}

#[test]
fn prop_vectors_all_types_incl_empty() {
    let cases = [
        K::BoolVec(vec![]), K::BoolVec(vec![true, false, true]),
        K::ByteVec(vec![]), K::ByteVec(vec![0, 255, 128]),
        K::ShortVec(vec![NULL_SHORT, 0, i16::MAX]), K::IntVec(vec![]),
        K::IntVec(vec![NULL_INT, 0, INF_INT]),
        K::LongVec(vec![NULL_LONG, INF_LONG]), K::RealVec(vec![f32::NAN, 0.0]),
        K::FloatVec(vec![]), K::FloatVec(vec![f64::NAN, INF_FLOAT, -0.0]),
        K::CharVec(vec![]), K::CharVec(vec![0, 1, 2, 255]),
        K::SymbolVec(vec![]), K::SymbolVec(vec!["".into(), "x".into()]),
        K::TimestampVec(vec![NULL_LONG]), K::MonthVec(vec![NULL_INT]),
        K::DateVec(vec![]), K::DateTimeVec(vec![f64::NAN]),
        K::MinuteVec(vec![]), K::SecondVec(vec![0]), K::TimeVec(vec![]),
        K::TimespanVec(vec![NULL_LONG]),
    ];
    for k in &cases {
        assert!(k_eq(k, &rt(k)), "vec mismatch: {k:?}");
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. Serializer bounds.
// ═══════════════════════════════════════════════════════════════════════════
#[test]
fn bounds_max_symbol_len() {
    let s: String = std::iter::repeat('z').take(1 << 20).collect();          // 1 MiB symbol
    let k = K::Symbol(s.clone());
    assert!(k_eq(&k, &rt(&k)));
    let kv = K::SymbolVec(vec![s.clone(), "".into(), s]);                     // + empty in the middle
    assert!(k_eq(&kv, &rt(&kv)));
}

#[test]
fn bounds_empty_everything() {
    let cases = [
        K::List(vec![]),
        K::Dict(Box::new(K::SymbolVec(vec![])), Box::new(K::List(vec![]))),
        K::Table(Box::new(K::Dict(Box::new(K::SymbolVec(vec![])),
            Box::new(K::List(vec![]))))),
        K::Symbol("".into()), K::CharVec(vec![]), K::Error("".into()),
        K::IntVec(vec![]), K::SymbolVec(vec![]),
    ];
    for k in &cases {
        assert!(k_eq(k, &rt(k)), "empty mismatch: {k:?}");
    }
}

#[test]
fn bounds_nested_empty_lists_deep() {                                        // depth 64 < MAX_DEPTH
    let mut k = K::List(vec![]);
    for _ in 0..64 { k = K::List(vec![k]); }                                 // safe for the writer too
    assert!(k_eq(&k, &rt(&k)));
}

#[test]
fn bounds_nested_over_maxdepth_rejected() {
    // Serialize is trusted local data (no depth cap); deserialize must refuse
    // a payload nested past MAX_DEPTH rather than overflow the stack. Built as
    // raw bytes so the (also-recursive) writer is never invoked at depth.
    assert!(deserialize(&nested_list_bytes(400)).is_err(),
        "over-depth payload must be rejected");
}

#[test]
fn bounds_count_overflow_rejected() {
    // A wire count is a signed i32, so a serializer asked for >= 2^31 elements
    // would emit a NEGATIVE count. Prove the reader rejects exactly those
    // overflowed encodings instead of trusting them into a giant allocation.
    for bits in [0x8000_0000u32, 0xFFFF_FFFF, 0x7FFF_FFFF] {
        let mut body = vec![6u8, 0];                                         // int-vec tag + attr
        body.extend_from_slice(&bits.to_le_bytes());
        assert!(deserialize(&body).is_err(),
            "count 0x{bits:08x} must be rejected");
    }
}

#[test]
fn bounds_large_feasible_vector() {
    // A real (feasible) large count exercises the header length path at scale.
    let k = K::IntVec((0..1_000_000).collect());
    let bytes = serialize(&k).unwrap();
    let n = i32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
    assert_eq!(n, 1_000_000, "on-wire count field");
    assert!(k_eq(&k, &deserialize(&bytes).unwrap()));
}
