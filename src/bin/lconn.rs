//! lconn — interactive remote terminal client for L.
//!
//! Usage:
//!     lconn host:port[:user:pass]
//!     lconn -H host -p 5001 [-u user:pass]
//!     lconn host:port -e "select count i from t"        (one-shot)
//!
//! See the in-REPL `\?` for the client-side command list.

use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use l_rs::io::{csv as csv_io, json as json_io};
use l_rs::serialize;
use l_rs::{Connection, K};
use rustyline::config::Configurer;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

const HELP_TEXT: &str = "\
client-side commands (else passed to server):
  \\\\                  quit
  \\? \\h              this help
  \\spool <path>       start session transcript
  \\spool              stop transcript
  \\load <path>        run a local .q / .k script
  \\load <name> <path> read .csv / .json / binary, assign to <name>
  \\save <path> <expr> write expr result to .csv / .json / binary
  \\cd <dir>           local cwd
  \\!<cmd>             local shell";

/// Wraps each query so the server runs it through `.Q.s` (matches L's local
/// REPL formatter): errors come back as `'msg`, a `::`
/// result comes back as `""` (suppressed).
const FMT_WRAPPER: &str = "{.[{r:value x;$[(::)~r;\"\";.Q.s r]};enlist \
    x;{\"'\",x}]}";

// ── CLI ─────────────────────────────────────────────────────────────────────
#[derive(Parser, Debug)]
#[command(name = "lconn", about = "interactive remote console for L", version)]
struct Args {
    /// Target as host:port[:user:pass].
    target: Option<String>,
    #[arg(short = 'H', long)] host: Option<String>,                             // override target host
    #[arg(short = 'p', long)] port: Option<u16>,                                // override target port
    #[arg(short = 'u', long)] user: Option<String>,                             // override creds (user:pass)
    #[arg(short = 'e', long)] eval: Option<String>,                             // one-shot: send, print, exit
}

struct Target { host: String, port: u16, creds: String }

/// Split `host:port[:user:pass]` into its parts (creds may be "" or "user").
fn parse_target(s: &str) -> Result<Target> {
    let mut it = s.splitn(4, ':');
    let host = it.next().ok_or_else(|| anyhow!("empty target"))?.to_string();
    let port: u16 = it.next().ok_or_else(|| anyhow!("missing port"))?
        .parse().context("port must be a number 1-65535")?;
    let (user, pass) = (it.next().unwrap_or(""), it.next().unwrap_or(""));
    let creds = if user.is_empty() { String::new() }                            // none / user / user:pass
                else if pass.is_empty() { user.into() }
                else { format!("{user}:{pass}") };
    Ok(Target { host, port, creds })
}

/// Build the connection target: positional string first, flags override.
fn resolve_target(args: &Args) -> Result<Target> {
    let mut t = match &args.target {
        Some(s) => parse_target(s)?,
        None    => Target { host: "localhost".into(), port: 5001, creds:
            "".into() },
    };
    if let Some(h) = &args.host { t.host = h.clone(); }
    if let Some(p) = args.port  { t.port = p; }
    if let Some(u) = &args.user { t.creds = u.clone(); }
    Ok(t)
}

fn connect(t: &Target) -> Result<Connection> {
    if t.creds.is_empty() { Connection::connect(&t.host, t.port) }
    else { Connection::connect_with_auth(&t.host, t.port, &t.creds) }
        .with_context(|| format!("connecting to {}:{}", t.host, t.port))
}

// ── terminal width sync ──────────────────────────────────────────────────────
/// TIOCGWINSZ; falls back to 25×80 when stdout isn't a tty.
fn terminal_size() -> (u16, u16) {
    #[repr(C)] struct Winsize { rows: u16, cols: u16, _xp: u16, _yp: u16 }
    let mut ws = Winsize { rows: 0, cols: 0, _xp: 0, _yp: 0 };
    let ok = unsafe {                                                           // ioctl into Winsize
        libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws)
    } == 0;
    if ok && ws.rows > 0 && ws.cols > 0 { (ws.rows, ws.cols) } else { (25, 80) }
}

fn sync_terminal_size(conn: &mut Connection) {                                  // push size as `\c r c`
    let (r, c) = terminal_size();
    let _ = conn.query(&format!("\\c {r} {c}"));
}

static RESIZED: AtomicBool = AtomicBool::new(false);                            // set by SIGWINCH
extern "C" fn on_sigwinch(_: libc::c_int) { RESIZED.store(true,
    Ordering::Relaxed); }
fn install_sigwinch() {
    let h = on_sigwinch as extern "C" fn(libc::c_int);                          // fn item -> fn pointer
    unsafe { libc::signal(libc::SIGWINCH, h as libc::sighandler_t); }
}

/// Does the server have a `.Q.s` formatter we can defer rendering to?
fn has_server_formatter(conn: &mut Connection) -> bool {
    matches!(conn.query(".Q.s 1"), Ok(K::CharVec(_)))
}

// ── send / print / tee ───────────────────────────────────────────────────────
fn tee(log: &mut Option<File>, line: &str) {                                    // mirror to transcript
    if let Some(f) = log.as_mut() { let _ = writeln!(f, "{line}"); }
}

/// Send `expr`, print the result (server-formatted when `fmt`), tee to log.
fn send_and_print(conn: &mut Connection, expr: &str, log: &mut Option<File>,
    fmt: bool) {
    let r = if fmt {                                                            // wrap in .Q.s on server
        conn.query_with_args(FMT_WRAPPER,
            vec![K::CharVec(expr.as_bytes().to_vec())])
    } else { conn.query(expr) };
    let s = match r {
        Ok(k) => match (fmt, k.as_string()) {
            (true, Some(s)) => s.trim_end_matches('\n').to_string(),            // pre-rendered
            _               => format!("{k}"),                                  // client-side Display
        },
        Err(e) => { let s = format!("{e}"); eprintln!("{s}"); tee(log, &s);
            return; }
    };
    if s.is_empty() { return; }                                                 // `::` suppressed
    println!("{s}"); tee(log, &s);
}

// ── format detection by extension ────────────────────────────────────────────
#[derive(Copy, Clone)]
enum Fmt { Script, Csv, Json, Binary }

fn fmt_of(path: &str) -> Fmt {                                                  // by file extension
    match Path::new(path).extension().and_then(|s| s.to_str())
        .unwrap_or("").to_ascii_lowercase().as_str()
    {
        "q" | "k" => Fmt::Script, "csv" => Fmt::Csv, "json" => Fmt::Json,
        _ => Fmt::Binary,
    }
}

fn fmt_label(f: Fmt) -> &'static str {
    match f { Fmt::Script => "script", Fmt::Csv => "csv",
              Fmt::Json => "json", Fmt::Binary => "l" }
}

// ── \load — script (1 arg) or named data file (2 args) ───────────────────────
fn handle_load(conn: &mut Connection, log: &mut Option<File>, line: &str, fmt:
    bool) {
    let rest = line.trim_start_matches("\\load").trim_start();
    if rest.is_empty() { eprintln!("'load: usage: \\load [<name>] <path>");
        return; }
    let mut it = rest.splitn(2, char::is_whitespace);
    let first  = it.next().unwrap();
    let second = it.next().map(str::trim).filter(|s| !s.is_empty());
    let (name, path) = match second { Some(p) => (Some(first), p), None =>
        (None, first) };
    match (name, fmt_of(path)) {
        (None, Fmt::Script) => run_script(conn, log, path, fmt),
        (Some(name), kind @ (Fmt::Csv | Fmt::Json | Fmt::Binary)) =>
            load_data(conn, name, path, kind),
        (None, _) =>
            eprintln!("'load: data file needs a target name: \\load <name> \
                {path}"),
        (Some(_), Fmt::Script) =>
            eprintln!("'load: scripts have no target name: \\load {path}"),
    }
}

/// `\load file.q` — read locally and send each statement (whitespace-prefixed
/// lines continue the previous statement; /-prefixed lines are comments).
fn run_script(conn: &mut Connection, log: &mut Option<File>, path: &str, fmt:
    bool) {
    let buf = match fs::read_to_string(path) {
        Ok(s)  => s,
        Err(e) => { eprintln!("'load: {path}: {e}"); return; }
    };
    let mut stmt = String::new();
    let flush = |stmt: &mut String, conn: &mut Connection, log: &mut
        Option<File>| {
        let s = stmt.trim().to_string(); stmt.clear();
        if s.is_empty() { return; }
        let echo = format!("l>{s}"); println!("{echo}"); tee(log, &echo);
        send_and_print(conn, &s, log, fmt);
    };
    for raw in buf.lines() {
        if raw.starts_with('/') { continue; }                                   // comment line
        if raw.starts_with(|c: char| c.is_whitespace()) {                       // continuation
            stmt.push(' '); stmt.push_str(raw.trim());
        } else { flush(&mut stmt, conn, log); stmt.push_str(raw); }
    }
    flush(&mut stmt, conn, log);
}

/// `\load <name> <path>` — read csv/json/binary locally, `set` it server-side.
fn load_data(conn: &mut Connection, name: &str, path: &str, kind: Fmt) {
    let r = match kind {
        Fmt::Csv    => csv_io::read_csv(path),
        Fmt::Json   => json_io::read_json(path),
        Fmt::Binary => fs::read(path)
            .map_err(|e| l_rs::LError::Deserialize(format!("read: {e}")))
            .and_then(|b| serialize::deserialize(&b)),
        Fmt::Script => unreachable!(),
    };
    let k = match r { Ok(k) => k, Err(e) => { eprintln!("'load: {e}"); return;
        } };
    let n = row_count(&k);
    match conn.query_with_args("set", vec![K::Symbol(name.into()), k]) {
        Ok(_)  => println!("'load: {path} ({}, {n} rows) → `{name}",
            fmt_label(kind)),
        Err(e) => eprintln!("'load: server assign failed: {e}"),
    }
}

// ── \save — write expr result to csv / json / binary (2 args) ────────────────
fn handle_save(conn: &mut Connection, line: &str) {
    let rest = line.trim_start_matches("\\save").trim_start();
    let (path, expr) = match rest.find(char::is_whitespace) {
        Some(i) => (&rest[..i], rest[i..].trim()), None => (rest, ""),
    };
    if path.is_empty() || expr.is_empty() {
        eprintln!("'save: usage: \\save <path> <expr>"); return;
    }
    let k = match conn.query(expr) {
        Ok(k) => k, Err(e) => { eprintln!("'save: {e}"); return; }
    };
    let kind = fmt_of(path);
    let r = match kind {
        Fmt::Csv    => csv_io::write_csv(path, &k),
        Fmt::Json   => json_io::write_json(path, &k),
        Fmt::Binary => serialize::serialize(&k).and_then(|b| fs::write(path, b)
            .map_err(|e| l_rs::LError::Serialize(format!("write: {e}")))),
        Fmt::Script => Err(l_rs::LError::Type("'save: cannot write \
            scripts".into())),
    };
    match r {
        Ok(())  => println!("'save: wrote {} rows to {path} ({})",
                            row_count(&k), fmt_label(kind)),
        Err(e)  => eprintln!("'save: {e}"),
    }
}

// ── \spool — toggle session transcript ───────────────────────────────────────
fn handle_spool(log: &mut Option<File>, line: &str) {
    let path = line.trim_start_matches("\\spool").trim();
    if path.is_empty() {                                                        // bare \spool toggles off
        println!("{}", if log.take().is_some() { "'spool: stopped" }
                       else { "'spool: not active" });
        return;
    }
    match OpenOptions::new().create(true).append(true).open(path) {
        Ok(f)  => { *log = Some(f); println!("'spool: appending to {path}"); }
        Err(e) => eprintln!("'spool: {path}: {e}"),
    }
}

// ── misc helpers ─────────────────────────────────────────────────────────────
fn row_count(k: &K) -> usize {                                                  // rows of a table/dict
    let dict = match k { K::Table(d) => d.as_ref(), other => other };
    match dict {
        K::Dict(_, v) => match v.as_ref() {
            K::List(cols) => cols.first().map(K::len).unwrap_or(0), _ => 0,
        },
        _ => 0,
    }
}

fn run_shell(cmd: &str) {                                                       // \!<cmd> — local shell
    let cmd = cmd.trim();
    if cmd.is_empty() { return; }
    if let Err(e) = Command::new("sh").arg("-c").arg(cmd).status() {
        eprintln!("'shell: {e}");
    }
}

fn change_dir(arg: &str) {                                                      // \cd <dir> (bare = pwd)
    let dir = arg.trim();
    if dir.is_empty() {
        if let Ok(c) = env::current_dir() { println!("{}", c.display()); }
            return;
    }
    if let Err(e) = env::set_current_dir(dir) { eprintln!("'cd: {dir}: {e}"); }
}

fn history_path() -> PathBuf {                                                  // ~/.lconn_history
    env::var("HOME").map(|h| PathBuf::from(h).join(".lconn_history"))
        .unwrap_or_else(|_| PathBuf::from(".lconn_history"))
}

// ── REPL loop ────────────────────────────────────────────────────────────────
fn repl(conn: &mut Connection, fmt: bool) -> Result<()> {
    let mut rl = DefaultEditor::new()?;
    rl.set_auto_add_history(true);
    let hist = history_path();
    let _ = rl.load_history(&hist);
    let mut log: Option<File> = None;
    loop {
        match rl.readline("l>") {
            Ok(line) => {
                let line = line.trim_end_matches(['\r', '\n']).to_string();
                tee(&mut log, &format!("l>{line}"));
                if line == "\\\\" { break; }                                    // \\ quits
                if line.is_empty() { continue; }
                if RESIZED.swap(false, Ordering::Relaxed) {
                    sync_terminal_size(conn); }
                if line == "\\?" || line == "\\h"   { println!("{HELP_TEXT}"); }
                else if line.starts_with("\\spool") { handle_spool(&mut log,
                    &line); }
                else if line.starts_with("\\load")  { handle_load(conn, &mut
                    log, &line, fmt); }
                else if line.starts_with("\\save")  { handle_save(conn,
                    &line); }
                else if let Some(d) = line.strip_prefix("\\cd") {
                    change_dir(d); }
                else if let Some(c) = line.strip_prefix("\\!")  {
                    run_shell(c); }
                else { send_and_print(conn, &line, &mut log, fmt); }
            }
            Err(ReadlineError::Interrupted) => continue,                        // ^C: drop the line
            Err(ReadlineError::Eof)         => break,                           // ^D: quit
            Err(e) => { eprintln!("'readline: {e}"); break; }
        }
    }
    let _ = rl.save_history(&hist);
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let target = resolve_target(&args)?;
    let mut conn = connect(&target)?;
    let fmt = has_server_formatter(&mut conn);                                  // server-side .Q.s?
    sync_terminal_size(&mut conn);
    install_sigwinch();
    match args.eval.as_deref() {
        Some(expr) => { send_and_print(&mut conn, expr, &mut None, fmt);
            Ok(()) }
        None       => repl(&mut conn, fmt),
    }
}
