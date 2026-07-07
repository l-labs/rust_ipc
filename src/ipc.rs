//! IPC client over TCP. Each message is an 8-byte header then a serialized K:
//!   [0] endianness (1=LE)  [1] msg-type  [2] LZ4 flag  [3] reserved
//!   [4..8] total length (i32 LE, header included)
//! A compressed payload is `[uncompressed-size i32 LE][LZ4 block]`; we inflate
//! it transparently before deserializing.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};

use crate::error::{LError, Result};
use crate::serialize;
use crate::k::K;

/// Defensive ceiling on one framed message — and on a compressed payload's
/// declared inflated size. The L IPC length is a signed 32-bit field, so 2 GiB
/// is the protocol's own hard limit; we refuse anything past this as a corrupt
/// or hostile frame rather than hand a wild length to the allocator (a huge or
/// negative-cast length would abort the process before a byte is validated).
const MAX_MSG: usize = 256 << 20;                                              // 256 MiB per reply

/// Message kind written into header byte 1.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum MsgType { Async = 0, Sync = 1, Response = 2 }                          // fire / request / reply

/// A live TCP connection to an l instance. Not `Send` — one per thread.
pub struct Connection { stream: TcpStream }

impl Connection {
    /// Connect with no credentials.
    pub fn connect(host: &str, port: u16) -> Result<Self> {
        Self::connect_with_auth(host, port, "")
    }

    /// Connect, sending `user:pass` (or "") then reading l's 1-byte ack.
    pub fn connect_with_auth(host: &str, port: u16, creds: &str) ->
        Result<Self> {
        let addr = format!("{host}:{port}");
        let mut stream = TcpStream::connect(&addr)                              // refused -> our error
            .map_err(|e| LError::ConnectionFailed(format!("{addr}: {e}")))?;
        let mut auth = creds.as_bytes().to_vec();                               // handshake = creds+NUL
        auth.push(0);
        stream.write_all(&auth)?;
        let mut ack = [0u8; 1];                                                 // 1 byte ok; close=err
        stream.read_exact(&mut ack)?;
        Ok(Connection { stream })
    }

    /// Run `expr` synchronously and return the result K.
    pub fn query(&mut self, expr: &str) -> Result<K> {
        self.send(MsgType::Sync, &str_k(expr))?; self.receive()
    }

    /// Run `expr` with positional args, sent as `(expr; a1; a2; …)`.
    pub fn query_with_args(&mut self, expr: &str, args: Vec<K>) -> Result<K> {
        let mut xs = vec![str_k(expr)]; xs.extend(args);                        // head is the expr
        self.send(MsgType::Sync, &K::List(xs))?; self.receive()
    }

    /// Fire-and-forget a K value (no reply read).
    pub fn send_async(&mut self, k: &K) -> Result<()> {
        self.send(MsgType::Async, k) }

    /// Like `query` but hand back the raw (decompressed) payload bytes,
    /// before deserialization — for byte-exact comparison in test runners.
    pub fn query_raw(&mut self, expr: &str) -> Result<Vec<u8>> {
        self.send(MsgType::Sync, &str_k(expr))?; self.recv_payload()
    }

    /// Close the socket explicitly (also happens on drop).
    pub fn close(self) { let _ = self.stream.shutdown(Shutdown::Both); }

    // ── framing ─────────────────────────────────────────────────────────────
    fn send(&mut self, msg: MsgType, k: &K) -> Result<()> {
        let body = serialize::serialize(k)?;
        let mut h = [0u8; 8];                                                   // header: LE, type, len
        h[0] = 1; h[1] = msg as u8;
        h[4..8].copy_from_slice(&((8 + body.len()) as i32).to_le_bytes());
        self.stream.write_all(&h)?; self.stream.write_all(&body)?;
        self.stream.flush()?; Ok(())
    }

    /// Read one framed message, returning its decompressed payload bytes.
    fn recv_payload(&mut self) -> Result<Vec<u8>> {
        let mut h = [0u8; 8];
        self.stream.read_exact(&mut h)?;                                        // header
        let compressed = h[2] == 1;                                             // byte 2: LZ4 flag
        let total = i32::from_le_bytes([h[4], h[5], h[6], h[7]]);               // signed on the wire
        if total < 8 {                                                          // < 8 (and any negative)
            return Err(LError::Deserialize("invalid message length".into()));
        }
        let total = total as usize;
        if total - 8 > MAX_MSG {                                                // reject BEFORE alloc
            return Err(LError::Deserialize(format!(
                "message length {total} exceeds {MAX_MSG}-byte cap")));
        }
        let mut body = vec![0u8; total - 8];                                    // payload after header
        self.stream.read_exact(&mut body)?;
        if compressed && body.len() > 4 {                                       // [raw size][LZ4 block]
            let raw = i32::from_le_bytes([body[0], body[1], body[2], body[3]]);
            if raw < 0 || raw as usize > MAX_MSG {                              // corrupt inflate size
                return Err(LError::Deserialize(format!(
                    "compressed size {raw} out of range")));
            }
            lz4(&body[4..], raw as usize).map_err(LError::Deserialize)
        } else { Ok(body) }
    }

    /// Read + deserialize a reply; a server `K::Error` surfaces as `L`.
    fn receive(&mut self) -> Result<K> {
        let k = serialize::deserialize(&self.recv_payload()?)?;
        match k { K::Error(m) => Err(LError::L(m)), other => Ok(other) }
    }
}

impl Drop for Connection {                                                      // best-effort close
    fn drop(&mut self) { let _ = self.stream.shutdown(Shutdown::Both); }
}

fn str_k(expr: &str) -> K { K::CharVec(expr.as_bytes().to_vec()) }              // L char vector

/// Minimal LZ4 block decompressor (standard format, zero deps). A token is
/// `<literal-len:4><match-len:4>`; lengths ≥15 spill across 0xFF bytes; a
/// match copies `match-len+4` bytes from `offset` behind the cursor.
fn lz4(src: &[u8], dst_len: usize) -> std::result::Result<Vec<u8>, String> {
    let mut dst = vec![0u8; dst_len];
    let (mut si, mut di) = (0usize, 0usize);
    // Every `src[..]` index below is length-guarded: a truncated or corrupt
    // block must fail with an Err, never panic on an out-of-bounds read.
    while si < src.len() {
        let tok = src[si] as usize; si += 1;
        let mut ll = tok >> 4;                                                  // literal run length
        if ll == 15 { loop {                                                    // spills across 0xFF
            if si >= src.len() { return Err("lz4: truncated lit len".into()); }
            let b = src[si] as usize; si += 1;
            ll += b; if b != 255 { break; } } }
        if di + ll > dst_len || si + ll > src.len() {
            return Err("lz4: literal overflow".into());
        }
        dst[di..di + ll].copy_from_slice(&src[si..si + ll]);                    // bulk literal copy
        di += ll; si += ll;
        if si >= src.len() { break; }                                           // last sequence: no match
        if si + 2 > src.len() { return Err("lz4: truncated offset".into()); }
        let off = (src[si] as usize) | ((src[si + 1] as usize) << 8); si += 2;
        if off == 0 || di < off { return Err("lz4: invalid offset".into()); }
        let mut ml = (tok & 0x0f) + 4;                                          // match length (min 4)
        if ml == 19 { loop {                                                    // spills across 0xFF
            if si >= src.len() { return Err("lz4: truncated match len".into());}
            let b = src[si] as usize; si += 1;
            ml += b; if b != 255 { break; } } }
        if di + ml > dst_len { return Err("lz4: match overflow".into()); }
        let s = di - off;
        if off >= ml { dst.copy_within(s..s + ml, di); }                        // non-overlap: bulk move
        else { for i in 0..ml { dst[di + i] = dst[s + i]; } }                   // overlap: replicate
        di += ml;
    }
    dst.truncate(di); Ok(dst)
}
