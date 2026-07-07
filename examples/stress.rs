//! Stress test: N threads × M queries against an l server.
//!
//! Usage:
//!   cargo run --example stress -- [host] [port] [threads] [queries_per_thread]
//!
//! Defaults: localhost 5001 100 50
//!
//! Each thread creates its own Connection, sends M sync queries ("1+1"),
//! verifies each response is 2, then disconnects.

use l_rs::Connection;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let host = args.get(1).map(|s| s.as_str()).unwrap_or("localhost");
    let port: u16 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(5001);
    let n_threads: usize = args.get(3).and_then(|s|
        s.parse().ok()).unwrap_or(100);
    let n_queries: usize = args.get(4).and_then(|s|
        s.parse().ok()).unwrap_or(50);

    println!("Stress test: {} threads × {} queries = {} total",
             n_threads, n_queries, n_threads * n_queries);
    println!("Target: {}:{}", host, port);

    let ok = Arc::new(AtomicU64::new(0));
    let fail = Arc::new(AtomicU64::new(0));
    let conn_fail = Arc::new(AtomicU64::new(0));
    let queries_done = Arc::new(AtomicU64::new(0));

    let start = Instant::now();

    let handles: Vec<_> = (0..n_threads).map(|tid| {
        let ok = ok.clone();
        let fail = fail.clone();
        let conn_fail = conn_fail.clone();
        let queries_done = queries_done.clone();
        let host = host.to_string();

        thread::spawn(move || {
            // Connect
            let mut conn = match Connection::connect(&host, port) {
                Ok(c) => c,
                Err(e) => {
                    conn_fail.fetch_add(1, Ordering::Relaxed);
                    eprintln!("Thread {}: connect failed: {}", tid, e);
                    return;
                }
            };

            // Send queries
            for q in 0..n_queries {
                match conn.query("1+1") {
                    Ok(result) => {
                        if result.as_int() == Some(2) {
                            ok.fetch_add(1, Ordering::Relaxed);
                        } else {
                            fail.fetch_add(1, Ordering::Relaxed);
                            eprintln!("Thread {}: query {}: wrong result {:?}",
                                tid, q, result);
                        }
                    }
                    Err(e) => {
                        fail.fetch_add(1, Ordering::Relaxed);
                        eprintln!("Thread {}: query {}: error {}", tid, q, e);
                        return;                                                 // connection likely broken
                    }
                }
                queries_done.fetch_add(1, Ordering::Relaxed);
            }
        })
    }).collect();

    // Wait for all threads
    for h in handles {
        h.join().unwrap();
    }

    let elapsed = start.elapsed();
    let total_ok = ok.load(Ordering::Relaxed);
    let total_fail = fail.load(Ordering::Relaxed);
    let total_conn_fail = conn_fail.load(Ordering::Relaxed);
    let total_queries = total_ok + total_fail;
    let qps = if elapsed.as_secs_f64() > 0.0 {
        total_queries as f64 / elapsed.as_secs_f64()
    } else { 0.0 };

    println!("\n=== Results ===");
    println!("Time:        {:.2}s", elapsed.as_secs_f64());
    println!("Connections: {} OK, {} failed", n_threads as u64 -
        total_conn_fail, total_conn_fail);
    println!("Queries:     {} OK, {} failed / {} total", total_ok, total_fail,
        total_queries);
    println!("Throughput:  {:.0} queries/sec", qps);

    if total_fail > 0 || total_conn_fail > 0 {
        std::process::exit(1);
    }
}
