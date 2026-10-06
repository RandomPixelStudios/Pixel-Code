//! PTY-Host: ein Hintergrundprozess (`pixel-code --pty-host`), der alle Shells und Agents besitzt.
//! Die App verbindet sich pro Terminal über einen Unix-Socket. Beendet sich die App (Neustart nach
//! einem Update, Absturz), laufen die Sitzungen weiter, und die neue App verbindet sich wieder.
//!
//! Protokoll (beide Richtungen): Frames `[Typ u8][Länge u32 BE][Daten]`.
//! App -> Host: `A` attach (JSON), `I` Eingabe, `R` Größe (JSON), `K` beenden, `L` Liste, `S` alles beenden.
//! Host -> App: `H` hello (JSON), `O` Ausgabe, `F` Vordergrundprozess, `X` beendet, `L` Liste (JSON).
//! Das Protokoll muss abwärtskompatibel bleiben: eine neue App spricht mit einem alten, noch laufenden Host.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};

pub const PROTOCOL: u32 = 1;

pub fn socket_path() -> PathBuf {
    let base = dirs::runtime_dir().unwrap_or_else(|| {
        let uid = std::fs::read_to_string("/proc/self/loginuid").unwrap_or_default();
        PathBuf::from(format!("/tmp/pixel-code-{}", uid.trim()))
    });
    base.join("pixel-code").join("pty.sock")
}

// ---------------------------------------------------------------- Frames

pub fn write_frame(w: &mut impl Write, kind: u8, data: &[u8]) -> std::io::Result<()> {
    let mut buf = Vec::with_capacity(5 + data.len());
    buf.push(kind);
    buf.extend_from_slice(&(data.len() as u32).to_be_bytes());
    buf.extend_from_slice(data);
    w.write_all(&buf)?;
    w.flush()
}

pub fn read_frame(r: &mut impl Read) -> std::io::Result<(u8, Vec<u8>)> {
    let mut head = [0u8; 5];
    r.read_exact(&mut head)?;
    let len = u32::from_be_bytes([head[1], head[2], head[3], head[4]]) as usize;
    if len > 64 * 1024 * 1024 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "frame too large"));
    }
    let mut data = vec![0u8; len];
    r.read_exact(&mut data)?;
    Ok((head[0], data))
}

#[derive(Serialize, Deserialize)]
pub struct Attach {
    pub pane: u64,
    pub cwd: PathBuf,
    pub argv: Option<Vec<String>>,
    pub env: Vec<(String, String)>,
    pub rows: u16,
    pub cols: u16,
    #[serde(default)]
    pub protocol: u32,
}

#[derive(Serialize, Deserialize)]
pub struct Hello {
    pub protocol: u32,
    /// true = an eine laufende Sitzung angehängt, false = neu gestartet
    pub resumed: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Size {
    pub rows: u16,
    pub cols: u16,
}

// ---------------------------------------------------------------- Host

struct Session {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    parser: vt100::Parser,
    /// Verbindung der App, die gerade angehängt ist (mit Verbindungsnummer)
    client: Option<(u64, UnixStream)>,
    fg: String,
}

type Sessions = Arc<Mutex<HashMap<u64, Session>>>;

fn send(s: &mut Session, kind: u8, data: &[u8]) {
    if let Some((_, c)) = s.client.as_mut() {
        if write_frame(c, kind, data).is_err() {
            s.client = None;
        }
    }
}

fn foreground(master: &dyn MasterPty) -> String {
    master
        .process_group_leader()
        .and_then(|pid| std::fs::read_to_string(format!("/proc/{pid}/comm")).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Läuft im Prozess `pixel-code --pty-host`, bis keine Sitzung und keine App mehr da ist.
pub fn run_host() {
    let path = socket_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }
    // Läuft schon ein Host? Dann nichts tun.
    if UnixStream::connect(&path).is_ok() {
        return;
    }
    let _ = std::fs::remove_file(&path);
    let Ok(listener) = UnixListener::bind(&path) else { return };
    let sessions: Sessions = Arc::new(Mutex::new(HashMap::new()));
    let clients = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    // Vordergrundprozesse beobachten und ohne Sitzungen nach einer Weile beenden
    {
        let sessions = sessions.clone();
        let clients = clients.clone();
        let path = path.clone();
        std::thread::spawn(move || {
            let mut idle_since: Option<Instant> = None;
            loop {
                std::thread::sleep(Duration::from_millis(500));
                let mut map = sessions.lock().unwrap();
                for s in map.values_mut() {
                    let fg = foreground(s.master.as_ref());
                    if fg != s.fg {
                        s.fg = fg.clone();
                        send(s, b'F', fg.as_bytes());
                    }
                }
                let empty = map.is_empty() && clients.load(std::sync::atomic::Ordering::SeqCst) == 0;
                drop(map);
                match (empty, idle_since) {
                    (true, None) => idle_since = Some(Instant::now()),
                    (true, Some(t)) if t.elapsed() > Duration::from_secs(20) => {
                        let _ = std::fs::remove_file(&path);
                        std::process::exit(0);
                    }
                    (false, _) => idle_since = None,
                    _ => {}
                }
            }
        });
    }

    for conn in listener.incoming().flatten() {
        let sessions = sessions.clone();
        let clients = clients.clone();
        std::thread::spawn(move || {
            clients.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            handle(conn, sessions);
            clients.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        });
    }
}

fn spawn_session(a: &Attach, sessions: &Sessions) -> Result<(), String> {
    let pair = native_pty_system()
        .openpty(PtySize { rows: a.rows.max(1), cols: a.cols.max(2), pixel_width: 0, pixel_height: 0 })
        .map_err(|e| e.to_string())?;
    let shell = a.env.iter().find(|(k, _)| k == "SHELL").map(|(_, v)| v.clone()).unwrap_or_else(|| "/bin/bash".into());
    let mut cmd = match &a.argv {
        Some(v) if !v.is_empty() => {
            let mut c = CommandBuilder::new(&v[0]);
            c.args(&v[1..]);
            c
        }
        _ => CommandBuilder::new(&shell),
    };
    cmd.env_clear();
    for (k, v) in &a.env {
        cmd.env(k, v);
    }
    cmd.cwd(if a.cwd.is_dir() { a.cwd.clone() } else { dirs::home_dir().unwrap_or_default() });
    let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
    let pane = a.pane;
    sessions.lock().unwrap().insert(
        pane,
        Session {
            master: pair.master,
            writer,
            child,
            parser: vt100::Parser::new(a.rows.max(1), a.cols.max(2), 0),
            client: None,
            fg: String::new(),
        },
    );
    let sessions = sessions.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 16 * 1024];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let mut map = sessions.lock().unwrap();
                    let Some(s) = map.get_mut(&pane) else { return };
                    s.parser.process(&buf[..n]);
                    send(s, b'O', &buf[..n]);
                }
            }
        }
        let mut map = sessions.lock().unwrap();
        if let Some(mut s) = map.remove(&pane) {
            let _ = s.child.wait();
            send(&mut s, b'X', b"");
        }
    });
    Ok(())
}

static NEXT_CONN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn handle(mut conn: UnixStream, sessions: Sessions) {
    let me = NEXT_CONN.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let mut pane: Option<u64> = None;
    loop {
        let Ok((kind, data)) = read_frame(&mut conn) else { break };
        match kind {
            b'A' => {
                let Ok(a) = serde_json::from_slice::<Attach>(&data) else { break };
                let resumed = sessions.lock().unwrap().contains_key(&a.pane);
                if !resumed && spawn_session(&a, &sessions).is_err() {
                    let _ = write_frame(&mut conn, b'X', b"");
                    break;
                }
                let Ok(out) = conn.try_clone() else { break };
                // Eine hängende App darf den Host nicht blockieren
                let _ = out.set_write_timeout(Some(Duration::from_secs(3)));
                let mut map = sessions.lock().unwrap();
                let Some(s) = map.get_mut(&a.pane) else { break };
                let hello = serde_json::to_vec(&Hello { protocol: PROTOCOL, resumed }).unwrap_or_default();
                // Unter der Sperre: Bildschirmzustand und Live-Ausgabe kommen in der richtigen Reihenfolge an
                s.client = Some((me, out));
                send(s, b'H', &hello);
                if resumed {
                    let _ = s.master.resize(PtySize { rows: a.rows.max(1), cols: a.cols.max(2), pixel_width: 0, pixel_height: 0 });
                    let screen = s.parser.screen();
                    let mut state = Vec::new();
                    if screen.alternate_screen() {
                        state.extend_from_slice(b"\x1b[?1049h");
                    }
                    state.extend_from_slice(b"\x1b[H\x1b[2J");
                    state.extend_from_slice(&screen.state_formatted());
                    state.extend_from_slice(&screen.cursor_state_formatted());
                    send(s, b'O', &state);
                    s.parser.screen_mut().set_size(a.rows.max(1), a.cols.max(2));
                    // Programm soll neu zeichnen (z.B. Claude Code nach Größenänderung)
                    if let Some(pid) = s.master.process_group_leader() {
                        unsafe { libc_kill(-pid, 28) };
                    }
                }
                let fg = s.fg.clone();
                send(s, b'F', fg.as_bytes());
                pane = Some(a.pane);
            }
            b'I' => {
                let Some(p) = pane else { continue };
                if let Some(s) = sessions.lock().unwrap().get_mut(&p) {
                    let _ = s.writer.write_all(&data);
                    let _ = s.writer.flush();
                }
            }
            b'R' => {
                let (Some(p), Ok(sz)) = (pane, serde_json::from_slice::<Size>(&data)) else { continue };
                if let Some(s) = sessions.lock().unwrap().get_mut(&p) {
                    let _ = s.master.resize(PtySize { rows: sz.rows.max(1), cols: sz.cols.max(2), pixel_width: 0, pixel_height: 0 });
                    s.parser.screen_mut().set_size(sz.rows.max(1), sz.cols.max(2));
                }
            }
            b'K' => {
                let Some(p) = pane else { continue };
                if let Some(s) = sessions.lock().unwrap().get_mut(&p) {
                    s.client = None;
                    let _ = s.child.kill();
                }
                break;
            }
            b'L' => {
                let ids: Vec<u64> = sessions.lock().unwrap().keys().copied().collect();
                let _ = write_frame(&mut conn, b'L', &serde_json::to_vec(&ids).unwrap_or_default());
            }
            b'S' => {
                for s in sessions.lock().unwrap().values_mut() {
                    s.client = None;
                    let _ = s.child.kill();
                }
            }
            _ => {}
        }
    }
    // Verbindung weg: Sitzung läuft weiter, nur nicht mehr angehängt
    if let Some(p) = pane {
        if let Some(s) = sessions.lock().unwrap().get_mut(&p) {
            if s.client.as_ref().is_some_and(|(id, _)| *id == me) {
                s.client = None;
            }
        }
    }
}

unsafe extern "C" {
    #[link_name = "kill"]
    fn libc_kill(pid: i32, sig: i32) -> i32;
}

// ---------------------------------------------------------------- App-Seite

/// Verbindet sich mit dem Host und startet ihn bei Bedarf.
pub fn connect() -> Result<UnixStream, String> {
    let path = socket_path();
    if let Ok(s) = UnixStream::connect(&path) {
        return Ok(s);
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    // Eigene Sitzung (setsid), damit der Host das Schließen der App überlebt
    std::process::Command::new("setsid")
        .arg("-f")
        .arg(&exe)
        .arg("--pty-host")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .or_else(|_| {
            std::process::Command::new(&exe)
                .arg("--pty-host")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
        })
        .map_err(|e| format!("could not start the terminal host: {e}"))?;
    for _ in 0..100 {
        std::thread::sleep(Duration::from_millis(30));
        if let Ok(s) = UnixStream::connect(&path) {
            return Ok(s);
        }
    }
    Err("terminal host did not start".into())
}

/// Laufende Sitzungen des Hosts (leer, wenn keiner läuft).
pub fn list() -> Vec<u64> {
    let Ok(mut s) = UnixStream::connect(socket_path()) else { return Vec::new() };
    let _ = s.set_read_timeout(Some(Duration::from_secs(2)));
    if write_frame(&mut s, b'L', b"").is_err() {
        return Vec::new();
    }
    read_frame(&mut s).ok().and_then(|(_, d)| serde_json::from_slice(&d).ok()).unwrap_or_default()
}

/// Beendet eine Sitzung, an die gerade keine App angehängt ist.
pub fn kill(pane: u64) {
    let Ok(mut s) = UnixStream::connect(socket_path()) else { return };
    let attach = Attach { pane, cwd: PathBuf::from("/"), argv: Some(vec!["true".into()]), env: Vec::new(), rows: 1, cols: 2, protocol: PROTOCOL };
    let _ = write_frame(&mut s, b'A', &serde_json::to_vec(&attach).unwrap_or_default());
    let _ = write_frame(&mut s, b'K', b"");
}
