//! IPC performance benchmark: latency histograms + throughput for l.
//!
//! Usage:
//!   cargo run --release --example benchmark -- [host] [port]
//!
//! Runs a series of benchmarks against a running l server:
//!   1. Ping latency (1+1) — measures raw IPC round-trip
//!   2. Vector operations — measures serialization + compute + deser
//!   3. Table queries — measures real-world query patterns
//!   4. Throughput — max QPS single-threaded + multi-threaded
//!   5. Payload scaling — latency vs response size
//!
//! Requires: an L server running with `l -p <port>`

use l_rs::Connection;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

fn percentile(sorted: &[u64], p: f64) -> u64 {
    let idx = ((sorted.len() as f64) * p / 100.0) as usize;
    sorted[idx.min(sorted.len() - 1)]
}

/// Print one section title plus the latency-table column header.
fn header(title: &str) {
    println!("{title}");
    println!(
        "  {:<30} {:>6} {:>6} {:>6} {:>6} {:>8}",
        "query", "avg", "p50", "p95", "p99", "qps"
    );
    println!("  {}", "-".repeat(68));
}

fn run_latency(conn: &mut Connection, name: &str, query: &str, n: usize) {
    // Warmup
    for _ in 0..10 {
        let _ = conn.query(query);
    }

    let mut latencies = Vec::with_capacity(n);
    let start = Instant::now();

    for _ in 0..n {
        let t0 = Instant::now();
        match conn.query(query) {
            Ok(_) => latencies.push(t0.elapsed().as_micros() as u64),
            Err(e) => {
                eprintln!("  {} error: {}", name, e);
                return;
            }
        }
    }

    let elapsed = start.elapsed();
    latencies.sort();

    let p50 = percentile(&latencies, 50.0);
    let p95 = percentile(&latencies, 95.0);
    let p99 = percentile(&latencies, 99.0);
    let avg = latencies.iter().sum::<u64>() / latencies.len() as u64;
    let qps = n as f64 / elapsed.as_secs_f64();

    println!(
        "  {:<30} {:>6} {:>6} {:>6} {:>6} {:>8.0}",
        name, avg, p50, p95, p99, qps
    );
}

fn run_throughput(host: &str, port: u16, n_threads: usize, duration_s: u64) {
    let stop = Arc::new(AtomicU64::new(0));
    let total = Arc::new(AtomicU64::new(0));

    let handles: Vec<_> = (0..n_threads)
        .map(|_| {
            let stop = stop.clone();
            let total = total.clone();
            let host = host.to_string();
            thread::spawn(move || {
                let mut conn = match Connection::connect(&host, port) {
                    Ok(c) => c,
                    Err(_) => return,
                };
                let mut count = 0u64;
                while stop.load(Ordering::Relaxed) == 0 {
                    if conn.query("1+1").is_ok() {
                        count += 1;
                    } else {
                        break;
                    }
                }
                total.fetch_add(count, Ordering::Relaxed);
            })
        })
        .collect();

    thread::sleep(Duration::from_secs(duration_s));
    stop.store(1, Ordering::Relaxed);

    for h in handles {
        let _ = h.join();
    }

    let count = total.load(Ordering::Relaxed);
    let qps = count as f64 / duration_s as f64;
    println!(
        "  {:<30} {:>8.0} QPS ({} threads, {}s)",
        "max throughput", qps, n_threads, duration_s
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let host = args.get(1).map(|s| s.as_str()).unwrap_or("localhost");
    let port: u16 = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(5001);

    println!("L IPC Benchmark");
    println!("Target: {}:{}", host, port);
    println!();

    let mut conn = match Connection::connect(host, port) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Connect failed: {}. Start L with: l -p {}", e, port);
            std::process::exit(1);
        }
    };

    // ── 1. Ping latency ──
    header("1. Ping Latency (round-trip, us)");
    run_latency(&mut conn, "1+1 (atom)", "1+1", 10000);
    run_latency(&mut conn, "til 10 (small vec)", "til 10", 10000);
    run_latency(&mut conn, ":: (null)", "::", 10000);
    println!();

    // ── 2. Vector operations ──
    header("2. Vector Operations (1M elements, us)");

    // Setup vectors on server
    let _ = conn.query("BV:1000000?100");
    let _ = conn.query("BW:1000000?100");
    let _ = conn.query("BF:1000000?1.0");

    run_latency(&mut conn, "int add v+v", "BV+BW", 100);
    run_latency(&mut conn, "int mul v+v", "BV*BW", 100);
    run_latency(&mut conn, "int sum", "sum BV", 1000);
    run_latency(&mut conn, "float sum", "sum BF", 1000);
    run_latency(&mut conn, "sort (asc)", "asc BV", 100);
    run_latency(&mut conn, "group", "group BV", 100);
    run_latency(&mut conn, "distinct", "distinct BV", 100);
    println!();

    // ── 3. Payload scaling ──
    header("3. Payload Scaling (response size, us)");
    run_latency(&mut conn, "til 100 (400B)", "til 100", 5000);
    run_latency(&mut conn, "til 10000 (40KB)", "til 10000", 2000);
    run_latency(&mut conn, "til 100000 (400KB)", "til 100000", 500);
    run_latency(&mut conn, "til 1000000 (4MB)", "til 1000000", 100);
    println!();

    // ── 4. Table queries ──
    header("4. Table Queries (us)");

    // Create table on server
    let _ = conn.query("BT:([]s:`a`b`c`d`e`f`g`h;p:8?100.0;v:8?1000)");
    run_latency(&mut conn, "select from BT", "select from BT", 5000);
    run_latency(
        &mut conn,
        "select avg p by s",
        "select avg p by s from BT",
        5000,
    );
    run_latency(
        &mut conn,
        "select where s=`a",
        "select from BT where s=`a",
        5000,
    );

    // Cleanup
    let _ = conn.query("delete BV from `.");
    let _ = conn.query("delete BW from `.");
    let _ = conn.query("delete BF from `.");
    let _ = conn.query("delete BT from `.");
    println!();

    // ── 5. Throughput ──
    println!("5. Throughput");
    drop(conn);
    run_throughput(host, port, 1, 3);
    run_throughput(host, port, 4, 3);
    run_throughput(host, port, 8, 3);

    println!("\nDone.");
}
