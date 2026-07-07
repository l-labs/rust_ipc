//! Basic connection example for l-rs.
//!
//! Start an L server first: l -p 5001
//! Then run: cargo run --example connect [host] [port]

use l_rs::Connection;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let host = args.get(1).map(|s| s.as_str()).unwrap_or("localhost");
    let port: u16 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(5001);
    let mut conn = match Connection::connect(host, port) {
        Ok(c) => { println!("Connected to L on port {port}"); c }
        Err(e) => { eprintln!("Failed to connect: {}", e); return; }
    };

    // Simple arithmetic
    match conn.query("1+1") {
        Ok(result) => println!("1+1 = {}", result),
        Err(e) => eprintln!("Query error: {}", e),
    }

    // Vector operations
    match conn.query("til 10") {
        Ok(result) => println!("til 10 = {}", result),
        Err(e) => eprintln!("Query error: {}", e),
    }

    // Create and query a table
    match conn.query("t:([]sym:`IBM`MSFT`AAPL;price:120.5 340.2 175.8;qty:100 \
        200 150)") {
        Ok(_) => println!("Created table t"),
        Err(e) => eprintln!("Error: {}", e),
    }

    match conn.query("select from t where price>150") {
        Ok(result) => println!("select from t where price>150:\n{}", result),
        Err(e) => eprintln!("Query error: {}", e),
    }

    // String operations
    match conn.query("count \"hello world\"") {
        Ok(result) => println!("count \"hello world\" = {}", result),
        Err(e) => eprintln!("Query error: {}", e),
    }

    // Math
    match conn.query("sqrt 2") {
        Ok(result) => println!("sqrt 2 = {}", result),
        Err(e) => eprintln!("Query error: {}", e),
    }

    // Type conversions
    let result = conn.query("1+1").unwrap();
    if let Some(v) = result.as_int() {
        println!("Got int: {}", v);
    }

    let result = conn.query("til 5").unwrap();
    let v: Vec<i32> = result.try_into().unwrap();
    println!("Got vec: {:?}", v);

    println!("\nDone.");
}
