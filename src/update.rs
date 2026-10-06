//! Sucht auf GitHub nach neuen Versionen und installiert das .deb-Paket.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use eframe::egui;

pub const REPO: &str = "RandomPixelStudios/Pixel-Code";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone)]
pub struct Release {
    pub version: String,
    pub notes: String,
    pub url: String,
    pub deb: Option<String>,
}

#[derive(Clone)]
pub enum State {
    Idle,
    Checking,
    UpToDate,
    Available(Release),
    Downloading(Release),
    Installing(Release),
    /// Installiert, wartet auf Neustart
    Installed(String),
    Error(String),
}

#[derive(Clone)]
pub struct Updater {
    state: Arc<Mutex<State>>,
}

impl Updater {
    pub fn new() -> Self {
        Self { state: Arc::new(Mutex::new(State::Idle)) }
    }

    pub fn get(&self) -> State {
        self.state.lock().unwrap().clone()
    }

    fn set(&self, s: State) {
        *self.state.lock().unwrap() = s;
    }

    pub fn busy(&self) -> bool {
        matches!(self.get(), State::Checking | State::Downloading(_) | State::Installing(_))
    }

    pub fn available(&self) -> Option<Release> {
        match self.get() {
            State::Available(r) => Some(r),
            _ => None,
        }
    }

    pub fn check(&self, ctx: egui::Context) {
        if self.busy() {
            return;
        }
        self.set(State::Checking);
        let me = self.clone();
        std::thread::spawn(move || {
            me.set(match latest() {
                Ok(Some(r)) if newer(&r.version, VERSION) => State::Available(r),
                Ok(_) => State::UpToDate,
                Err(e) => State::Error(e),
            });
            ctx.request_repaint();
        });
    }

    /// Lädt das .deb herunter und installiert es mit pkexec (grafische Passwortabfrage).
    pub fn install(&self, ctx: egui::Context, r: Release) {
        let Some(url) = r.deb.clone() else {
            ctx.open_url(egui::OpenUrl::new_tab(&r.url));
            return;
        };
        self.set(State::Downloading(r.clone()));
        let me = self.clone();
        std::thread::spawn(move || {
            let file = download_dir().join(format!("pixel-code_{}.deb", r.version));
            let res = (|| {
                let _ = std::fs::create_dir_all(download_dir());
                let ok = Command::new("curl").args(["-fsSL", "-o"]).arg(&file).arg(&url).status().is_ok_and(|s| s.success())
                    || Command::new("wget").args(["-q", "-O"]).arg(&file).arg(&url).status().is_ok_and(|s| s.success());
                if !ok {
                    return Err("Download failed. Check your internet connection.".to_string());
                }
                me.set(State::Installing(r.clone()));
                ctx.request_repaint();
                let out = Command::new("pkexec")
                    .args(["apt-get", "install", "-y", "--allow-downgrades"])
                    .arg(&file)
                    .stdin(Stdio::null())
                    .output()
                    .map_err(|e| format!("Could not run pkexec: {e}"))?;
                if !out.status.success() {
                    let err = String::from_utf8_lossy(&out.stderr);
                    let line = err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("Installation was cancelled.");
                    return Err(line.to_string());
                }
                Ok(())
            })();
            me.set(match res {
                Ok(()) => State::Installed(r.version.clone()),
                Err(e) => State::Error(e),
            });
            ctx.request_repaint();
        });
    }
}

fn download_dir() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp")).join("pixel-code/updates")
}

/// GET-Anfrage über curl (sonst wget). `Ok(None)` = nicht gefunden (404).
fn fetch(url: &str) -> Result<Option<String>, String> {
    let ua = format!("pixel-code/{VERSION}");
    let accept = "Accept: application/vnd.github+json";
    if let Ok(out) = Command::new("curl").args(["-sSL", "-H", accept, "-A", &ua, "-w", "\n%{http_code}", url]).output() {
        let text = String::from_utf8_lossy(&out.stdout).into_owned();
        let (body, code) = text.rsplit_once('\n').unwrap_or(("", text.as_str()));
        return match code.trim() {
            "200" => Ok(Some(body.to_string())),
            "404" => Ok(None),
            "403" | "429" => Err("GitHub rate limit reached, try again later.".into()),
            _ => Err("Could not reach GitHub.".into()),
        };
    }
    let out = Command::new("wget")
        .args(["-qO-", "--header", accept, url])
        .output()
        .map_err(|_| "curl or wget is required to check for updates.".to_string())?;
    match out.status.code() {
        Some(0) => Ok(Some(String::from_utf8_lossy(&out.stdout).into_owned())),
        Some(8) => Ok(None),
        _ => Err("Could not reach GitHub.".into()),
    }
}

/// Neueste Version auf GitHub; `None`, wenn noch keine veröffentlicht ist.
fn latest() -> Result<Option<Release>, String> {
    let Some(text) = fetch(&format!("https://api.github.com/repos/{REPO}/releases/latest"))? else { return Ok(None) };
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|_| "Unexpected answer from GitHub.".to_string())?;
    let tag = v["tag_name"].as_str().ok_or("No release found.")?;
    let arch = match std::env::consts::ARCH {
        "aarch64" => "arm64",
        _ => "amd64",
    };
    let urls: Vec<&str> = v["assets"].as_array().into_iter().flatten().filter_map(|a| a["browser_download_url"].as_str()).collect();
    let deb = urls.iter().find(|u| u.ends_with(&format!("_{arch}.deb"))).or(urls.iter().find(|u| u.ends_with(".deb")));
    Ok(Some(Release {
        version: tag.trim_start_matches('v').to_string(),
        notes: v["body"].as_str().unwrap_or("").trim().to_string(),
        url: v["html_url"].as_str().unwrap_or("").to_string(),
        deb: deb.map(|s| s.to_string()),
    }))
}

fn parts(v: &str) -> Vec<u64> {
    v.trim_start_matches('v').split(['.', '-']).map(|p| p.parse().unwrap_or(0)).collect()
}

pub fn newer(a: &str, b: &str) -> bool {
    parts(a) > parts(b)
}

static RESTARTING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// true, wenn die App für ein Update neu startet (Sitzungen sollen weiterlaufen).
pub fn restarting() -> bool {
    RESTARTING.load(std::sync::atomic::Ordering::SeqCst)
}

/// Startet die (neu installierte) App und beendet diese Instanz.
pub fn restart(ctx: &egui::Context) {
    RESTARTING.store(true, std::sync::atomic::Ordering::SeqCst);
    let exe = if std::path::Path::new("/usr/bin/pixel-code").exists() {
        PathBuf::from("/usr/bin/pixel-code")
    } else {
        std::env::current_exe().unwrap_or_default()
    };
    if Command::new(exe).spawn().is_ok() {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}
