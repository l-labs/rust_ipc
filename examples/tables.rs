//! Table operations example for l-rs.
//!
//! Start an L server first: l -p 5001
//! Then run: cargo run --example tables [host] [port]

use l_rs::{Connection, K};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let host = args.get(1).map(|s| s.as_str()).unwrap_or("localhost");
    let port: u16 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(5001);
    let mut conn = Connection::connect(host, port).expect("connect failed");

    // Build a table from Rust data
    let cols = K::SymbolVec(vec!["sym".into(), "price".into(), "qty".into()]);
    let syms = K::SymbolVec(vec!["IBM".into(), "MSFT".into(), "AAPL".into(),
        "GOOG".into()]);
    let prices = K::FloatVec(vec![120.5, 340.2, 175.8, 2800.0]);
    let qtys = K::IntVec(vec![100, 200, 150, 50]);
    let vals = K::List(vec![syms, prices, qtys]);
    let dict = K::Dict(Box::new(cols), Box::new(vals));
    let table = K::Table(Box::new(dict));

    println!("Sending table to L...");
    // Use query_with_args to pass the table as an argument
    match conn.query_with_args("{trade::x}", vec![table]) {
        Ok(_) => println!("Table 'trade' created in L"),
        Err(e) => eprintln!("Error: {}", e),
    }

    // Query the table
    match conn.query("select from trade") {
        Ok(result) => println!("All trades:\n{}", result),
        Err(e) => eprintln!("Error: {}", e),
    }

    match conn.query("select sym, price from trade where price > 200") {
        Ok(result) => println!("\nExpensive (>200):\n{}", result),
        Err(e) => eprintln!("Error: {}", e),
    }

    match conn.query("select avg price by sym from trade") {
        Ok(result) => println!("\nAvg price by sym:\n{}", result),
        Err(e) => eprintln!("Error: {}", e),
    }

    println!("\nDone.");
}
