# l-rs

Rust client for the L database IPC protocol. Pure Rust, no external
dependencies, no `unsafe`. Compressed responses are inflated transparently.

## Quickstart

```toml
[dependencies]
l-rs = { git = "https://github.com/l-labs/rust_ipc" }
```

Start a server (`l -p 5001`), then:

```rust
use l_rs::{Connection, K};

fn main() -> Result<(), l_rs::LError> {
    let mut conn = Connection::connect("localhost", 5001)?;
    let r = conn.query("2+2")?;                              // K::Int(4)
    println!("{r}");
    let v: Vec<i32> = conn.query("til 10")?.try_into()?;     // typed extract
    assert_eq!(v.len(), 10);
    let s = conn.query_with_args("{x+y}", vec![K::Long(2), K::Long(3)])?;
    assert_eq!(s, K::Long(5));
    Ok(())
}
```

`connect_with_auth(host, port, "user:pass")` sends credentials during the
handshake; `send_async(&k)` fires a message without reading a reply.

## Types

Every result is a `K`. Atoms are scalars, vectors homogeneous `Vec<T>`.

| L type           | K variant (atom / vector)   | Rust type       |
|------------------|-----------------------------|-----------------|
| boolean, byte    | `Bool`, `Byte` (+`Vec`)     | `bool`, `u8`    |
| short, int, long | `Short`/`Int`/`Long` (+`Vec`)| `i16`/`i32`/`i64`|
| real, float      | `Real`, `Float` (+`Vec`)    | `f32`, `f64`    |
| char, string     | `Char` / `CharVec`          | `u8` / bytes    |
| symbol           | `Symbol` (+`Vec`)           | `String`        |
| timestamp…time   | `Timestamp`, `Date`, …      | `i64` / `i32`   |
| mixed list       | `List(Vec<K>)`              |                 |
| dict, table      | `Dict(keys, vals)`, `Table` | table wraps dict|

`From` builds a `K` from a Rust value; `TryFrom` extracts one. Null int/long
is `MIN`, null float is `NaN`. `Err(LError::L(msg))` is a server error; other
variants are transport failures. `Connection` is not `Send` — one per thread.

## Build and test

```sh
cargo build          # library + lconn console (cargo run --bin lconn)
l -p 5001            # then, against the running server:
L_TEST_PORT=5001 cargo test
```

Cross-process tests spawn a second server from `l` on `PATH` or `$L_BIN`.
Examples: `connect`, `tables`, `stress`, `benchmark`. MIT license.
