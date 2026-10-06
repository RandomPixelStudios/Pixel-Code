mod agents;
mod icons;
mod keybinds;
mod layout;
mod plugins;
mod settings;
mod telegram;
mod terminal;
mod theme;
mod update;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use agents::{Agent, AGENTS};
use eframe::egui::{self, pos2, vec2, Color32, Rect, RichText, Sense, Stroke};
use icons::Icon;
use keybinds::{Cmd, Keybinds};
use layout::{Grid, PaneId, Side};
use serde::{Deserialize, Serialize};
use terminal::{Status, Terminal};
use theme::*;

/// Aus dem Startmenü gestartet fehlt der PATH der Login-Shell (z.B. ~/.opencode/bin, ~/.kimi-code/bin).
/// Holt ihn einmal beim Start, damit Agents gefunden werden und Terminals denselben PATH haben.
fn import_login_path() {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let out = std::process::Command::new(shell)
            .args(["-lic", "printf '__PC_PATH__%s__PC_PATH__' \"$PATH\""])
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output();
        let _ = tx.send(out.ok().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()));
    });
    let login = rx.recv_timeout(Duration::from_secs(3)).ok().flatten().and_then(|s| s.split("__PC_PATH__").nth(1).map(str::to_string));
    let mut dirs: Vec<PathBuf> = login.iter().flat_map(|p| std::env::split_paths(p).collect::<Vec<_>>()).collect();
    dirs.extend(std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect::<Vec<_>>()).unwrap_or_default());
    let mut seen = std::collections::HashSet::new();
    dirs.retain(|d| !d.as_os_str().is_empty() && seen.insert(d.clone()));
    if let Ok(joined) = std::env::join_paths(dirs) {
        // SAFETY: läuft vor dem Start aller anderen Threads
        unsafe { std::env::set_var("PATH", joined) };
    }
}

fn main() -> eframe::Result {
    import_login_path();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Pixel Code")
            .with_app_id("pixel-code")
            .with_inner_size([1400.0, 860.0])
            .with_min_inner_size([760.0, 460.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native(
        "Pixel Code",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(App::new(&cc.egui_ctx)))
        }),
    )
}

// ---------------------------------------------------------------- Datenmodell

#[derive(Serialize, Deserialize, Clone)]
enum Location {
    Local(PathBuf),
    Ssh { user: String, host: String, port: u16, path: String },
}

impl Location {
    fn display(&self) -> String {
        match self {
            Location::Local(p) => {
                let s = p.display().to_string();
                match dirs::home_dir().map(|h| h.display().to_string()) {
                    Some(h) if s.starts_with(&h) => s.replacen(&h, "~", 1),
                    _ => s,
                }
            }
            Location::Ssh { user, host, path, .. } => {
                let who = if user.is_empty() { host.clone() } else { format!("{user}@{host}") };
                format!("{who}:{path}")
            }
        }
    }

    /// Arbeitsverzeichnis und optionaler Befehl für ein neues Terminal.
    fn spawn_args(&self) -> (PathBuf, Option<Vec<String>>) {
        match self {
            Location::Local(p) => (p.clone(), None),
            Location::Ssh { user, host, port, path } => {
                let target = if user.is_empty() { host.clone() } else { format!("{user}@{host}") };
                let dir = if path.is_empty() { "~".to_string() } else { path.clone() };
                let remote = format!("cd {} && exec $SHELL -l", if dir.starts_with('~') { dir } else { shell_quote(&dir) });
                let argv = vec!["ssh".into(), "-t".into(), "-p".into(), port.to_string(), target, remote];
                (dirs::home_dir().unwrap_or_default(), Some(argv))
            }
        }
    }

    fn is_remote(&self) -> bool {
        matches!(self, Location::Ssh { .. })
    }
}

#[derive(Serialize, Deserialize)]
struct Session {
    name: String,
    grid: Grid,
    /// Eigene Namen einzelner Terminals.
    #[serde(default)]
    names: HashMap<PaneId, String>,
    /// Mit welchem Agent ein Terminal gestartet wurde (für das Logo).
    #[serde(default)]
    agents: HashMap<PaneId, String>,
    #[serde(default)]
    agent: Option<String>,
}

impl Session {
    fn new(name: String) -> Self {
        Self { name, grid: Grid::default(), names: HashMap::new(), agents: HashMap::new(), agent: None }
    }
}

#[derive(Serialize, Deserialize)]
struct Project {
    name: String,
    location: Location,
    sessions: Vec<Session>,
    #[serde(default = "yes")]
    expanded: bool,
}

fn yes() -> bool {
    true
}

#[derive(Serialize, Deserialize, Default)]
struct Store {
    projects: Vec<Project>,
    project: usize,
    session: usize,
    next_id: PaneId,
    /// Rechte Sidebar mit den Dateien des Projekts (standardmäßig zu)
    #[serde(default)]
    files_visible: bool,
}

fn state_file() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("pixel-code").join("state-v2.json")
}

fn shell_quote(s: &str) -> String {
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "/._-+=:,@".contains(c)) {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

// ---------------------------------------------------------------- UI-Zustand

/// Wird beim Ziehen eines Terminals (Kopfzeile oder Sidebar) mitgeführt.
#[derive(Clone, Copy)]
struct PaneDrag(PaneId);

enum Action {
    Focus(PaneId),
    /// Pane neben eine andere Pane verschieben (auch aus einer anderen Session desselben Projekts)
    Move(PaneId, PaneId, Side),
    /// In eine andere Session des aktuellen Projekts (None = neue Session)
    MoveToSession(PaneId, Option<usize>),
    SplitRight(PaneId),
    SplitDown(PaneId),
    Close(PaneId),
    ToggleMax(PaneId),
    Rename(PaneId),
    New,
}

#[derive(Clone, Copy, PartialEq)]
enum Rename {
    Project(usize),
    Session(usize, usize),
    Pane(PaneId),
}

#[derive(PartialEq, Clone, Copy)]
enum AddKind {
    Local,
    Ssh,
}

struct AddDialog {
    kind: AddKind,
    folder: Option<PathBuf>,
    name: String,
    host: String,
    user: String,
    port: String,
    remote_path: String,
    error: Option<String>,
}

impl AddDialog {
    fn new() -> Self {
        Self {
            kind: AddKind::Local,
            folder: None,
            name: String::new(),
            host: String::new(),
            user: std::env::var("USER").unwrap_or_default(),
            port: "22".into(),
            remote_path: "~".into(),
            error: None,
        }
    }
}

enum Modal {
    Add(AddDialog),
    Rename(Rename, String),
    RemoveProject(usize),
}

/// Was in einem neuen Terminal gestartet wird.
enum Launch {
    Agent(&'static Agent),
    Install(&'static Agent),
}

struct App {
    store: Store,
    terms: HashMap<PaneId, Terminal>,
    /// Befehle, die nach dem Start einer Shell eingetippt werden (claude, codex, ...).
    pending: HashMap<PaneId, String>,
    focused: Option<PaneId>,
    maximized: Option<PaneId>,
    modal: Option<Modal>,
    settings: Option<settings::Settings>,
    browse_rx: Option<mpsc::Receiver<Option<PathBuf>>>,
    pane_rects: Vec<(PaneId, Rect)>,
    installed: HashMap<&'static str, bool>,
    installed_at: Option<Instant>,
    /// Telegram-Fernbedienung
    tg_rx: mpsc::Receiver<String>,
    tg_target: Option<PaneId>,
    tg_list: Vec<PaneId>,
    tg_enter: Vec<(PaneId, Instant)>,
    tg_status: HashMap<PaneId, Status>,
    tg_checked: Option<Instant>,
    keybinds: Keybinds,
    updater: update::Updater,
    /// Vom Plugin/Hook gemeldeter Zustand je Terminal, seit wann
    hook: HashMap<PaneId, (Status, Instant)>,
    /// Geschätzter Zustand (Terminals ohne Hook) und seit wann ein Agent fertig ist
    heur: HashMap<PaneId, Status>,
    heur_done: HashMap<PaneId, Instant>,
    /// Wann der Nutzer ein Terminal zuletzt angesehen hat
    seen: HashMap<PaneId, Instant>,
    status_at: Option<Instant>,
    sidebar_width: f32,
    /// Datei-Sidebar: aufgeklappte Ordner und zwischengespeicherte Ordnerinhalte
    files_expanded: std::collections::HashSet<PathBuf>,
    files_cache: HashMap<PathBuf, (Instant, Vec<(String, bool)>)>,
}

impl App {
    fn new(ctx: &egui::Context) -> Self {
        theme::apply(ctx);
        let saved = std::fs::read_to_string(state_file()).ok();
        let mut store: Store = saved.as_deref().and_then(|s| serde_json::from_str(s).ok()).unwrap_or_default();
        if saved.is_none() {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
            store.projects.push(Project {
                name: "home".into(),
                location: Location::Local(home),
                sessions: vec![Session::new("Session 1".into())],
                expanded: true,
            });
        }
        for p in &mut store.projects {
            for s in &mut p.sessions {
                s.grid.normalize();
            }
        }
        let mut app = Self {
            store,
            terms: HashMap::new(),
            pending: HashMap::new(),
            focused: None,
            maximized: None,
            modal: None,
            settings: None,
            browse_rx: None,
            pane_rects: Vec::new(),
            installed: HashMap::new(),
            installed_at: None,
            tg_rx: telegram::start(ctx.clone()),
            tg_target: None,
            tg_list: Vec::new(),
            tg_enter: Vec::new(),
            tg_status: HashMap::new(),
            tg_checked: None,
            keybinds: Keybinds::load(),
            updater: update::Updater::new(),
            hook: HashMap::new(),
            heur: HashMap::new(),
            heur_done: HashMap::new(),
            seen: HashMap::new(),
            status_at: None,
            sidebar_width: 330.0,
            files_expanded: Default::default(),
            files_cache: HashMap::new(),
        };
        // Alte Zustände vom letzten Start verwerfen
        let _ = std::fs::remove_dir_all(plugins::status_dir());
        app.updater.check(ctx.clone());
        app.select(app.store.project, app.store.session);
        // Plugin-Registry (GitHub-Status) für die Agents aktuell halten
        let _ = plugins::GitHub::new(ctx.clone());
        plugins::refresh_all(ctx.clone());
        // Pixel-Code-Plugin aktuell halten, auch wenn der Agent außerhalb der App gestartet wird
        for id in ["opencode", "omp", "claude", "codex", "gemini", "kimi", "cursor", "qwen", "copilot", "amp"] {
            if agents::get(id).and_then(|a| a.bin).is_some_and(agents::is_installed) {
                plugins::install_agent_plugin(id);
            }
        }
        app
    }

    fn save(&self) {
        let path = state_file();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(s) = serde_json::to_string_pretty(&self.store) {
            let _ = std::fs::write(path, s);
        }
    }

    fn new_id(&mut self) -> PaneId {
        self.store.next_id += 1;
        self.store.next_id
    }

    fn project(&self) -> Option<&Project> {
        self.store.projects.get(self.store.project)
    }

    fn session(&self) -> Option<&Session> {
        self.project()?.sessions.get(self.store.session)
    }

    fn session_mut(&mut self) -> Option<&mut Session> {
        let s = self.store.session;
        self.store.projects.get_mut(self.store.project)?.sessions.get_mut(s)
    }

    fn select(&mut self, project: usize, session: usize) {
        self.store.project = project.min(self.store.projects.len().saturating_sub(1));
        let n = self.project().map_or(0, |p| p.sessions.len());
        self.store.session = session.min(n.saturating_sub(1));
        self.maximized = None;
        self.focused = self.session().and_then(|s| s.grid.panes().first().copied());
        self.save();
    }

    fn refresh_installed(&mut self) {
        if self.installed_at.is_some_and(|t| t.elapsed() < Duration::from_secs(4)) {
            return;
        }
        self.installed = AGENTS.iter().map(|a| (a.id, a.bin.is_none_or(agents::is_installed))).collect();
        self.installed_at = Some(Instant::now());
    }

    fn new_session(&mut self, project: usize) {
        let p = &mut self.store.projects[project];
        let name = format!("Session {}", p.sessions.len() + 1);
        p.sessions.push(Session::new(name));
        p.expanded = true;
        let s = p.sessions.len() - 1;
        self.select(project, s);
    }

    /// Öffnet ein Terminal in der aktuellen Session (neue Session, falls keine existiert).
    fn launch(&mut self, what: Launch) {
        if self.project().is_none() {
            return;
        }
        if self.session().is_none() {
            self.new_session(self.store.project);
        }
        let id = self.new_id();
        let target = self.focused;
        let Some(s) = self.session_mut() else { return };
        let ok = match target.filter(|t| s.grid.contains(*t)) {
            Some(t) => s.grid.split_right(t, id),
            None => s.grid.push(id),
        };
        if !ok {
            return;
        }
        let cmd = match what {
            Launch::Agent(a) => {
                plugins::install_agent_plugin(a.id);
                if a.bin.is_some() {
                    s.agents.insert(id, a.id.to_string());
                    if s.agent.is_none() {
                        s.agent = Some(a.id.to_string());
                    }
                }
                if s.name.starts_with("Session ") && s.grid.len() == 1 {
                    s.name = a.name.to_string();
                }
                a.bin.map(str::to_string)
            }
            Launch::Install(a) => {
                s.names.insert(id, format!("Install {}", a.name));
                self.installed_at = None;
                a.install.map(str::to_string)
            }
        };
        if let Some(cmd) = cmd {
            self.pending.insert(id, cmd);
        }
        self.focused = Some(id);
        self.maximized = None;
        self.save();
    }

    fn close_pane(&mut self, id: PaneId) {
        self.terms.remove(&id);
        let _ = std::fs::remove_file(plugins::status_dir().join(id.to_string()));
        self.pending.remove(&id);
        for p in &mut self.store.projects {
            for s in &mut p.sessions {
                s.grid.remove(id);
                s.names.remove(&id);
                s.agents.remove(&id);
            }
        }
        if self.maximized == Some(id) {
            self.maximized = None;
        }
        if self.focused == Some(id) {
            self.focused = self.session().and_then(|s| s.grid.panes().last().copied());
        }
        self.installed_at = None;
        self.save();
    }

    fn close_session(&mut self, p: usize, s: usize) {
        let session = self.store.projects[p].sessions.remove(s);
        for id in session.grid.panes() {
            self.terms.remove(&id);
        }
        let (cp, cs) = (self.store.project, self.store.session);
        let cs = if cp == p && s < cs { cs - 1 } else { cs };
        self.select(cp, cs);
    }

    fn remove_project(&mut self, i: usize) {
        let p = self.store.projects.remove(i);
        for s in p.sessions {
            for id in s.grid.panes() {
                self.terms.remove(&id);
            }
        }
        let cur = self.store.project;
        let next = if i < cur { cur - 1 } else { cur };
        let ses = if i == cur { 0 } else { self.store.session };
        self.select(next, ses);
    }

    fn apply(&mut self, action: Action) {
        match action {
            Action::Focus(id) => self.focused = Some(id),
            Action::SplitRight(t) | Action::SplitDown(t) => {
                let id = self.new_id();
                let right = matches!(action, Action::SplitRight(_));
                let Some(s) = self.session_mut() else { return };
                let ok = if right { s.grid.split_right(t, id) } else { s.grid.split_down(t, id) };
                if ok {
                    self.focused = Some(id);
                    self.maximized = None;
                    self.save();
                }
            }
            Action::Close(id) => self.close_pane(id),
            Action::Move(id, target, side) => self.move_pane(id, Some((target, side)), None),
            Action::MoveToSession(id, s) => {
                let s = match s {
                    Some(s) => s,
                    None => {
                        let p = &mut self.store.projects[self.store.project];
                        p.sessions.push(Session::new(format!("Session {}", p.sessions.len() + 1)));
                        p.sessions.len() - 1
                    }
                };
                self.move_pane(id, None, Some(s));
            }
            Action::New => self.launch(Launch::Agent(&AGENTS[0])),
            Action::ToggleMax(id) => {
                self.maximized = if self.maximized == Some(id) { None } else { Some(id) };
                self.focused = Some(id);
            }
            Action::Rename(id) => {
                let cur = self.session().and_then(|s| s.names.get(&id).cloned()).unwrap_or_default();
                self.modal = Some(Modal::Rename(Rename::Pane(id), cur));
            }
        }
    }

    /// Verschiebt eine Pane innerhalb des aktuellen Projekts: neben `at` (in dessen Session)
    /// oder ans Ende von Session `session`. Das Terminal läuft dabei einfach weiter.
    fn move_pane(&mut self, id: PaneId, at: Option<(PaneId, Side)>, session: Option<usize>) {
        let pi = self.store.project;
        let Some(p) = self.store.projects.get_mut(pi) else { return };
        let Some(src) = p.sessions.iter().position(|s| s.grid.contains(id)) else { return };
        let dst = match (at, session) {
            (Some((t, _)), _) => p.sessions.iter().position(|s| s.grid.contains(t)),
            (None, Some(s)) => Some(s),
            _ => None,
        };
        let Some(dst) = dst.filter(|d| *d < p.sessions.len()) else { return };
        if src == dst {
            if let Some((t, side)) = at {
                if p.sessions[src].grid.move_pane(id, t, side) {
                    self.focused = Some(id);
                    self.maximized = None;
                    self.save();
                }
            }
            return;
        }
        let ok = match at {
            Some((t, side)) => p.sessions[dst].grid.place(id, t, side),
            None => p.sessions[dst].grid.push(id),
        };
        if !ok {
            return;
        }
        p.sessions[src].grid.remove(id);
        if let Some(n) = p.sessions[src].names.remove(&id) {
            p.sessions[dst].names.insert(id, n);
        }
        if let Some(a) = p.sessions[src].agents.remove(&id) {
            p.sessions[dst].agents.insert(id, a);
        }
        p.expanded = true;
        self.select(pi, dst);
        self.focused = Some(id);
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        for cmd in self.keybinds.consume(ctx) {
            let panes = self.session().map(|s| s.grid.panes()).unwrap_or_default();
            match cmd {
                Cmd::NextPane | Cmd::PrevPane if !panes.is_empty() => {
                    let cur = self.focused.and_then(|f| panes.iter().position(|p| *p == f)).unwrap_or(0);
                    let n = panes.len();
                    let i = if cmd == Cmd::NextPane { (cur + 1) % n } else { (cur + n - 1) % n };
                    self.focused = Some(panes[i]);
                    if self.maximized.is_some() {
                        self.maximized = Some(panes[i]);
                    }
                }
                Cmd::NewTerminal => self.launch(Launch::Agent(&AGENTS[0])),
                Cmd::NewSession if self.project().is_some() => self.new_session(self.store.project),
                Cmd::NextSession | Cmd::PrevSession => {
                    let n = self.project().map_or(0, |p| p.sessions.len());
                    if n > 0 {
                        let cur = self.store.session;
                        let s = if cmd == Cmd::NextSession { (cur + 1) % n } else { (cur + n - 1) % n };
                        self.select(self.store.project, s);
                    }
                }
                Cmd::ToggleFiles => {
                    self.store.files_visible ^= true;
                    self.save();
                }
                Cmd::AddProject => self.modal = Some(Modal::Add(AddDialog::new())),
                Cmd::Settings => self.open_settings(ctx, settings::Page::Plugins),
                _ => {
                    let Some(f) = self.focused else { continue };
                    match cmd {
                        Cmd::SplitRight => self.apply(Action::SplitRight(f)),
                        Cmd::SplitDown => self.apply(Action::SplitDown(f)),
                        Cmd::Close => self.apply(Action::Close(f)),
                        Cmd::Maximize => self.apply(Action::ToggleMax(f)),
                        _ => {}
                    }
                }
            }
        }
    }

    fn open_settings(&mut self, ctx: &egui::Context, page: settings::Page) {
        let mut s = settings::Settings::new(ctx, page, self.keybinds.clone(), self.updater.clone());
        s.nav_width = self.sidebar_width;
        self.settings = Some(s);
    }

    // ---------------------------------------------------------------- Agent-Status

    /// Liest die Zustände der Hooks/Plugins und schätzt sie für alle anderen Terminals.
    fn poll_status(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        // Angesehen: fokussiertes Terminal der sichtbaren Session, solange das Fenster aktiv ist
        if let Some(f) = self.focused.filter(|f| self.session().is_some_and(|s| s.grid.contains(*f))) {
            if ctx.input(|i| i.viewport().focused.unwrap_or(true)) && self.settings.is_none() {
                self.seen.insert(f, now);
            }
        }
        if self.status_at.is_some_and(|t| t.elapsed() < Duration::from_millis(250)) {
            return;
        }
        self.status_at = Some(now);
        let dir = plugins::status_dir();
        for (&id, t) in &self.terms {
            let file = dir.join(id.to_string());
            if t.foreground() == t.shell_name {
                // Agent beendet: gemeldeter Zustand gilt nicht mehr
                if self.hook.remove(&id).is_some() {
                    let _ = std::fs::remove_file(&file);
                }
            } else {
                match std::fs::read_to_string(&file).ok().and_then(|s| Status::parse(&s)) {
                    Some(st) => {
                        if self.hook.get(&id).is_none_or(|(old, _)| *old != st) {
                            self.hook.insert(id, (st, now));
                        }
                    }
                    None => {
                        self.hook.remove(&id);
                    }
                }
            }
            let h = t.status();
            let prev = self.heur.insert(id, h);
            if prev == Some(Status::Working) && h == Status::Idle {
                self.heur_done.insert(id, now);
            } else if h != Status::Idle {
                self.heur_done.remove(&id);
            }
        }
        let terms = &self.terms;
        self.hook.retain(|id, _| terms.contains_key(id));
        self.heur.retain(|id, _| terms.contains_key(id));
        self.heur_done.retain(|id, _| terms.contains_key(id));
        self.seen.retain(|id, _| terms.contains_key(id));
    }

    fn seen_since(&self, id: PaneId, since: Instant) -> bool {
        self.seen.get(&id).is_some_and(|t| *t >= since) || self.terms.get(&id).is_some_and(|t| t.last_input() >= since)
    }

    fn status(&self, id: PaneId) -> Status {
        if !self.terms.get(&id).is_some_and(|t| t.is_alive()) {
            return Status::Idle;
        }
        if let Some(&(st, since)) = self.hook.get(&id) {
            return if st == Status::Done && self.seen_since(id, since) { Status::Idle } else { st };
        }
        let h = self.heur.get(&id).copied().unwrap_or(Status::Idle);
        if h == Status::Idle && self.heur_done.get(&id).is_some_and(|since| !self.seen_since(id, *since)) {
            return Status::Done;
        }
        h
    }

    // ---------------------------------------------------------------- Telegram

    /// Anzeigename einer Pane: eigener Name, sonst Agent, sonst laufendes Programm.
    fn pane_label(&self, id: PaneId) -> String {
        let session = self.store.projects.iter().flat_map(|p| &p.sessions).find(|s| s.grid.contains(id));
        if let Some(n) = session.and_then(|s| s.names.get(&id)) {
            return n.clone();
        }
        let agent = session.map(|s| self.pane_agent(s, id)).filter(|a| a.bin.is_some());
        match (agent, self.terms.get(&id)) {
            (Some(a), _) => a.name.to_string(),
            (None, Some(t)) => t.foreground(),
            (None, None) => "Terminal".into(),
        }
    }

    fn tg_target(&self) -> Option<PaneId> {
        self.tg_target.filter(|t| self.terms.contains_key(t)).or(self.focused.filter(|f| self.terms.contains_key(f)))
    }

    fn telegram(&mut self) {
        // Enter kurz nach dem Text schicken, damit TUIs ihn nicht als Einfügen behandeln
        let now = Instant::now();
        let due: Vec<_> = self.tg_enter.iter().filter(|(_, t)| *t <= now).map(|(id, _)| *id).collect();
        self.tg_enter.retain(|(_, t)| *t > now);
        for id in due {
            if let Some(t) = self.terms.get_mut(&id) {
                t.write(b"\r");
            }
        }

        while let Ok(text) = self.tg_rx.try_recv() {
            let text = text.trim().to_string();
            let (cmd, arg) = text.split_once(' ').map_or((text.as_str(), ""), |(c, a)| (c, a.trim()));
            match cmd {
                "/start" | "/help" => telegram::send(telegram::HELP.into()),
                "/list" => {
                    self.tg_list.clear();
                    let mut lines = Vec::new();
                    for p in &self.store.projects {
                        for s in &p.sessions {
                            for id in s.grid.panes().into_iter().filter(|id| self.terms.contains_key(id)) {
                                self.tg_list.push(id);
                                let mark = if Some(id) == self.tg_target() { " ◀" } else { "" };
                                lines.push(format!("{}. {} / {} / {}{mark}", self.tg_list.len(), p.name, s.name, self.pane_label(id)));
                            }
                        }
                    }
                    telegram::send(if lines.is_empty() { "No terminals are running.".into() } else { lines.join("\n") });
                }
                "/use" => match arg.parse::<usize>().ok().and_then(|n| self.tg_list.get(n.wrapping_sub(1)).copied()) {
                    Some(id) => {
                        self.tg_target = Some(id);
                        telegram::send(format!("Messages now go to {}.", self.pane_label(id)));
                    }
                    None => telegram::send("Unknown number - send /list first.".into()),
                },
                "/screen" => match self.tg_target().and_then(|id| self.terms.get(&id)) {
                    Some(t) => telegram::send(t.screen_text(40)),
                    None => telegram::send("No terminal selected.".into()),
                },
                "/esc" | "/ctrlc" | "/enter" => {
                    let bytes: &[u8] = match cmd {
                        "/esc" => b"\x1b",
                        "/ctrlc" => b"\x03",
                        _ => b"\r",
                    };
                    match self.tg_target().and_then(|id| self.terms.get_mut(&id)) {
                        Some(t) => t.write(bytes),
                        None => telegram::send("No terminal selected.".into()),
                    }
                }
                "/new" => {
                    let agent = agents::AGENTS.iter().find(|a| a.id == arg || a.bin == Some(arg)).unwrap_or(&AGENTS[0]);
                    let before = self.focused;
                    self.launch(Launch::Agent(agent));
                    if self.focused != before {
                        self.tg_target = self.focused;
                        telegram::send(format!("Started {} - your messages go there now.", agent.name));
                    } else {
                        telegram::send("Could not start it (session full or no project).".into());
                    }
                }
                _ => match self.tg_target() {
                    Some(id) => {
                        if let Some(t) = self.terms.get_mut(&id) {
                            t.write(text.as_bytes());
                            self.tg_enter.push((id, now + Duration::from_millis(150)));
                        }
                    }
                    None => telegram::send("No terminal is open. Use /new claude to start one.".into()),
                },
            }
        }

        // Benachrichtigen, wenn ein Agent fertig ist oder eine Frage hat
        if self.tg_checked.is_some_and(|t| t.elapsed() < Duration::from_secs(1)) {
            return;
        }
        self.tg_checked = Some(now);
        let active = telegram::active();
        let ids: Vec<PaneId> = self.terms.keys().copied().collect();
        for id in ids {
            let status = self.status(id);
            let prev = self.tg_status.insert(id, status);
            if !active || prev == Some(status) || prev.is_none() {
                continue;
            }
            let name = self.pane_label(id);
            match status {
                Status::Idle | Status::Done if prev == Some(Status::Working) => telegram::send(format!("✅ {name} is done.\n\n{}", self.terms[&id].screen_text(15))),
                Status::Question => telegram::send(format!("❓ {name} has a question:\n\n{}", self.terms[&id].screen_text(20))),
                Status::Permission => telegram::send(format!("🔐 {name} needs your permission:\n\n{}", self.terms[&id].screen_text(20))),
                Status::Error => telegram::send(format!("❌ {name} reported an error:\n\n{}", self.terms[&id].screen_text(15))),
                _ => {}
            }
        }
        self.tg_status.retain(|id, _| self.terms.contains_key(id));
    }

    // ---------------------------------------------------------------- Sidebar

    /// Agent, der in einer Pane läuft (laufendes Programm vor dem Start-Agent).
    fn pane_agent(&self, s: &Session, id: PaneId) -> &'static Agent {
        let stored = s.agents.get(&id).and_then(|a| agents::get(a));
        match self.terms.get(&id) {
            Some(t) => {
                let fg = t.foreground();
                agents::by_process(&fg).or(if fg == t.shell_name { None } else { stored })
            }
            None => stored,
        }
        .unwrap_or(&AGENTS[0])
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let card = ui.max_rect();
        ui.painter().rect(card, RADIUS, PANE, Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);

        // Kopfzeile wie bei den Terminals: Name links, Aktionen rechts
        let head = Rect::from_min_size(card.min, vec2(card.width(), HEAD));
        ui.painter().line_segment([head.left_bottom() + vec2(1.0, 0.0), head.right_bottom() - vec2(1.0, 0.0)], Stroke::new(1.0, BORDER));
        let mark = Rect::from_center_size(head.left_center() + vec2(22.0, 0.0), vec2(18.0, 18.0));
        egui::Image::from_bytes("bytes://app-icon.svg", APP_ICON_SVG).paint_at(ui, mark);
        ui.painter().text(head.left_center() + vec2(38.0, 0.0), egui::Align2::LEFT_CENTER, "Pixel Code", semibold(13.0), TEXT);

        let update = self.updater.available().is_some();
        let mut x = head.right() - 20.0;
        let mut btn = |ui: &mut egui::Ui, icon: Icon, tip: String, dot: bool| {
            let r = Rect::from_center_size(pos2(x, head.center().y), vec2(26.0, 26.0));
            x -= 28.0;
            let resp = ui.interact(r, ui.id().with(("sidebar_btn", tip.clone())), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(r, 6.0, HOVER);
            }
            icons::draw(ui.painter(), r.center(), icon, if resp.hovered() { TEXT } else { MUTED });
            if dot {
                ui.painter().circle(r.right_top() + vec2(-6.0, 6.0), 3.5, GREEN, Stroke::new(1.5, PANE));
            }
            resp.on_hover_text(tip).clicked()
        };
        let settings_tip = if update { "Settings · update available".to_string() } else { self.keybinds.tip("Settings", Cmd::Settings) };
        if btn(ui, Icon::Gear, settings_tip, update) {
            let page = if update { settings::Page::Update } else { settings::Page::Plugins };
            self.open_settings(ui.ctx(), page);
        }
        let files_tip = self.keybinds.tip(if self.store.files_visible { "Hide files" } else { "Show files" }, Cmd::ToggleFiles);
        if btn(ui, Icon::Files, files_tip, false) {
            self.store.files_visible ^= true;
            self.save();
        }
        if btn(ui, Icon::Plus, self.keybinds.tip("Add project", Cmd::AddProject), false) {
            self.modal = Some(Modal::Add(AddDialog::new()));
        }

        let list = Rect::from_min_max(pos2(card.left() + 8.0, head.bottom() + 10.0), card.max - vec2(8.0, 8.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(list), |ui| self.project_list(ui));
    }

    fn project_list(&mut self, ui: &mut egui::Ui) {
        enum SideCmd {
            Select(usize, usize),
            Focus(usize, usize, PaneId),
            Toggle(usize),
            NewSession(usize),
            CloseSession(usize, usize),
            Rename(Rename, String),
            Remove(usize),
            Move(PaneId, usize),
        }
        let mut cmds = Vec::new();

        let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), 18.0), Sense::hover());
        ui.painter().text(head.left_center() + vec2(8.0, 0.0), egui::Align2::LEFT_CENTER, "PROJECTS", semibold(10.5), FAINT);
        ui.add_space(4.0);

        if self.store.projects.is_empty() {
            ui.add_space(8.0);
            ui.label(RichText::new("  No projects yet").color(MUTED));
        }

        let ctx = ui.ctx().clone();
        let dragging = egui::DragAndDrop::payload::<PaneDrag>(&ctx).map(|d| d.0);
        let now = ui.input(|i| i.time);
        let mut animate = false;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for (pi, p) in self.store.projects.iter().enumerate() {
                let active_p = pi == self.store.project;
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
                let plus_r = Rect::from_center_size(rect.right_center() - vec2(14.0, 0.0), vec2(22.0, 22.0));
                let dots_r = plus_r.translate(vec2(-24.0, 0.0));
                let plus = ui.interact(plus_r, ui.id().with(("newsession", pi)), Sense::click());
                let dots = ui.interact(dots_r, ui.id().with(("dots", pi)), Sense::click());
                let hovered = resp.hovered() || plus.hovered() || dots.hovered();
                let menu_open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&dots));
                if hovered || menu_open {
                    ui.painter().rect_filled(rect, 8.0, HOVER);
                }

                // Pfeil auf/zu, Ordner- bzw. Server-Icon, Name
                let chev = rect.left_center() + vec2(12.0, 0.0);
                let col = if hovered { MUTED } else { FAINT };
                let st = Stroke::new(1.3, col);
                if p.expanded {
                    ui.painter().line_segment([chev + vec2(-3.5, -1.5), chev + vec2(0.0, 2.0)], st);
                    ui.painter().line_segment([chev + vec2(0.0, 2.0), chev + vec2(3.5, -1.5)], st);
                } else {
                    ui.painter().line_segment([chev + vec2(-1.5, -3.5), chev + vec2(2.0, 0.0)], st);
                    ui.painter().line_segment([chev + vec2(2.0, 0.0), chev + vec2(-1.5, 3.5)], st);
                }
                let ir = Rect::from_center_size(rect.left_center() + vec2(32.0, 0.0), vec2(15.0, 15.0));
                if p.location.is_remote() {
                    icons::draw(ui.painter(), ir.center(), Icon::Server, if active_p { MUTED } else { FAINT });
                } else {
                    egui::Image::from_bytes("bytes://folder.svg", FOLDER_SVG)
                        .tint(if active_p { TEXT } else { Color32::from_white_alpha(150) })
                        .paint_at(ui, ir);
                }
                let name_clip = Rect::from_min_max(rect.min, pos2(dots_r.left() - 4.0, rect.bottom()));
                ui.painter().with_clip_rect(name_clip).text(
                    rect.left_center() + vec2(48.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    &p.name,
                    semibold(13.0),
                    if active_p { TEXT } else { MUTED },
                );
                // Eingeklappt: wichtigster Zustand des Projekts als Punkt
                if !p.expanded && !hovered {
                    let st = p.sessions.iter().flat_map(|s| s.grid.panes()).map(|id| self.status(id)).max_by_key(|s| urgency(*s));
                    if let Some(st) = st.filter(|s| *s != Status::Idle) {
                        ui.painter().circle_filled(rect.right_center() - vec2(14.0, 0.0), 3.5, status_color(st));
                    }
                }
                for (r, resp, icon) in [(plus_r, &plus, Icon::Plus), (dots_r, &dots, Icon::Dots)] {
                    if resp.hovered() {
                        ui.painter().rect_filled(r, 6.0, SELECTED);
                    }
                    if hovered || menu_open {
                        icons::draw(ui.painter(), r.center(), icon, if resp.hovered() { TEXT } else { MUTED });
                    }
                }
                if plus.on_hover_text("New session").clicked() {
                    cmds.push(SideCmd::NewSession(pi));
                } else if resp.clicked() {
                    cmds.push(SideCmd::Toggle(pi));
                }
                let mut menu = |ui: &mut egui::Ui| {
                    ui.set_min_width(170.0);
                    if menu_item(ui, "New session") {
                        cmds.push(SideCmd::NewSession(pi));
                    }
                    if menu_item(ui, "Rename…") {
                        cmds.push(SideCmd::Rename(Rename::Project(pi), p.name.clone()));
                    }
                    ui.separator();
                    if menu_item_danger(ui, "Remove project") {
                        cmds.push(SideCmd::Remove(pi));
                    }
                };
                egui::Popup::menu(&dots).show(&mut menu);
                resp.context_menu(&mut menu);
                resp.on_hover_text(p.location.display());

                if !p.expanded {
                    ui.add_space(4.0);
                    continue;
                }
                if p.sessions.is_empty() {
                    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
                    ui.painter().text(r.left_center() + vec2(48.0, 0.0), egui::Align2::LEFT_CENTER, "No sessions", font(12.0), FAINT);
                }
                // Gezogene Pane gehört zu diesem Projekt? Dann sind die Sessions Ablageziele.
                let drag_here = dragging.filter(|d| p.sessions.iter().any(|s| s.grid.contains(*d)));
                for (si, s) in p.sessions.iter().enumerate() {
                    let panes = s.grid.panes();
                    let active = active_p && si == self.store.session;
                    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
                    let r = Rect::from_min_max(r.min + vec2(18.0, 0.0), r.max);
                    let drop = drag_here.filter(|d| !s.grid.contains(*d) && !s.grid.is_full());
                    let drop_hover = drop.is_some() && ui.rect_contains_pointer(r);
                    let fill = if drop_hover { ACCENT.gamma_multiply(0.12) } else if active { SELECTED } else if resp.hovered() { HOVER } else { Color32::TRANSPARENT };
                    ui.painter().rect_filled(r, 8.0, fill);
                    if drop.is_some() {
                        let stroke = if drop_hover { Stroke::new(1.0, ACCENT.gamma_multiply(0.8)) } else { Stroke::new(1.0, BORDER_STRONG) };
                        ui.painter().rect_stroke(r, 8.0, stroke, egui::StrokeKind::Inside);
                    }
                    if active {
                        let bar = Rect::from_min_max(pos2(r.left(), r.top() + 8.0), pos2(r.left() + 2.5, r.bottom() - 8.0));
                        ui.painter().rect_filled(bar, 2.0, ACCENT);
                    }
                    // Wichtigster Zustand der Session
                    let st = panes.iter().map(|id| self.status(*id)).max_by_key(|s| urgency(*s)).unwrap_or(Status::Idle);
                    let dot = r.left_center() + vec2(14.0, 0.0);
                    let alive = panes.iter().any(|id| self.terms.get(id).is_some_and(|t| t.is_alive()));
                    if st == Status::Idle {
                        ui.painter().circle_filled(dot, 3.0, if alive { MUTED } else { FAINT });
                    } else {
                        ui.painter().circle_filled(dot, 3.5, status_color(st));
                    }
                    let clip = Rect::from_min_max(r.min, pos2(r.right() - 34.0, r.bottom()));
                    ui.painter().with_clip_rect(clip).text(
                        r.left_center() + vec2(28.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        &s.name,
                        medium(13.0),
                        if active { TEXT } else { MUTED },
                    );
                    if drop_hover {
                        ui.painter().text(r.right_center() - vec2(12.0, 0.0), egui::Align2::RIGHT_CENTER, "Move here", medium(11.5), ACCENT);
                    } else if !panes.is_empty() {
                        let badge = Rect::from_center_size(r.right_center() - vec2(18.0, 0.0), vec2(22.0, 18.0));
                        ui.painter().rect_filled(badge, 6.0, if active { BORDER_STRONG } else { ELEVATED });
                        ui.painter().text(badge.center(), egui::Align2::CENTER_CENTER, panes.len().to_string(), medium(10.5), MUTED);
                    }
                    if let Some(d) = drop {
                        if resp.dnd_release_payload::<PaneDrag>().is_some() || (drop_hover && ui.input(|i| i.pointer.any_released())) {
                            cmds.push(SideCmd::Move(d, si));
                        }
                    }
                    if resp.clicked() {
                        cmds.push(SideCmd::Select(pi, si));
                    }
                    resp.context_menu(|ui| {
                        ui.set_min_width(160.0);
                        if menu_item(ui, "Rename…") {
                            cmds.push(SideCmd::Rename(Rename::Session(pi, si), s.name.clone()));
                        }
                        ui.separator();
                        if menu_item_danger(ui, "Close session") {
                            cmds.push(SideCmd::CloseSession(pi, si));
                        }
                    });

                    // Ein Eintrag pro Terminal: Logo mit Status-Ring, Name, Zustand
                    for id in &panes {
                        let (row, tresp) = ui.allocate_exact_size(vec2(ui.available_width(), 26.0), Sense::click_and_drag());
                        let row = Rect::from_min_max(row.min + vec2(34.0, 0.0), row.max);
                        let focused = active && self.focused == Some(*id);
                        let being_dragged = dragging == Some(*id);
                        if being_dragged {
                            ui.painter().rect_stroke(row, 7.0, Stroke::new(1.0, ACCENT.gamma_multiply(0.6)), egui::StrokeKind::Inside);
                        } else if focused {
                            ui.painter().rect_filled(row, 7.0, HOVER);
                        } else if tresp.hovered() {
                            ui.painter().rect_filled(row, 7.0, HOVER);
                        }
                        let st = self.status(*id);
                        let c = row.left_center() + vec2(14.0, 0.0);
                        if st != Status::Idle {
                            status_ring(ui.painter(), c, st, now, 9.5);
                            animate |= matches!(st, Status::Working | Status::Question | Status::Permission);
                        }
                        self.pane_agent(s, *id).paint_logo(ui, Rect::from_center_size(c, vec2(13.0, 13.0)), false);
                        let label = self.pane_label(*id);
                        let pill = status_pill(ui, st, row);
                        let clip = Rect::from_min_max(row.min, pos2(pill.map_or(row.right() - 8.0, |p| p.left() - 6.0), row.bottom()));
                        ui.painter().with_clip_rect(clip).text(
                            row.left_center() + vec2(28.0, 0.0),
                            egui::Align2::LEFT_CENTER,
                            &label,
                            font(12.5),
                            if focused || tresp.hovered() { TEXT } else { MUTED },
                        );
                        if tresp.drag_started() {
                            egui::DragAndDrop::set_payload(&ctx, PaneDrag(*id));
                        }
                        if tresp.dragged() {
                            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
                        }
                        if tresp.clicked() {
                            cmds.push(SideCmd::Focus(pi, si, *id));
                        }
                        tresp.on_hover_text("Click to focus · drag onto a terminal or another session to move it");
                    }
                }
                ui.add_space(8.0);
            }
        });

        if animate {
            ui.ctx().request_repaint_after(Duration::from_millis(60));
        }
        for c in cmds {
            match c {
                SideCmd::Select(p, s) => self.select(p, s),
                SideCmd::Focus(p, s, id) => {
                    if (p, s) != (self.store.project, self.store.session) {
                        self.select(p, s);
                    }
                    self.focused = Some(id);
                    if self.maximized.is_some() {
                        self.maximized = Some(id);
                    }
                }
                SideCmd::Toggle(p) => {
                    if p == self.store.project {
                        self.store.projects[p].expanded ^= true;
                    } else {
                        self.store.projects[p].expanded = true;
                        self.select(p, 0);
                    }
                }
                SideCmd::NewSession(p) => self.new_session(p),
                SideCmd::CloseSession(p, s) => self.close_session(p, s),
                SideCmd::Rename(t, n) => self.modal = Some(Modal::Rename(t, n)),
                SideCmd::Remove(p) => self.modal = Some(Modal::RemoveProject(p)),
                SideCmd::Move(id, s) => {
                    // Session gehört zum Projekt der Pane, nicht unbedingt zum aktuellen
                    if let Some(p) = self.store.projects.iter().position(|p| p.sessions.iter().any(|x| x.grid.contains(id))) {
                        if p != self.store.project {
                            self.store.project = p;
                        }
                        self.move_pane(id, None, Some(s));
                    }
                }
            }
        }
    }

    // ---------------------------------------------------------------- Datei-Sidebar

    /// Inhalt eines Ordners (Ordner zuerst), höchstens alle 2 Sekunden neu gelesen.
    fn list_dir(&mut self, dir: &std::path::Path) -> Vec<(String, bool)> {
        if let Some((t, v)) = self.files_cache.get(dir) {
            if t.elapsed() < Duration::from_secs(2) {
                return v.clone();
            }
        }
        let mut v: Vec<(String, bool)> = std::fs::read_dir(dir)
            .map(|rd| {
                rd.flatten()
                    .map(|e| (e.file_name().to_string_lossy().into_owned(), e.file_type().map(|t| t.is_dir()).unwrap_or(false)))
                    .filter(|(n, _)| n != ".git")
                    .take(2000)
                    .collect()
            })
            .unwrap_or_default();
        v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase())));
        self.files_cache.insert(dir.to_path_buf(), (Instant::now(), v.clone()));
        v
    }

    fn file_rows(&mut self, dir: &std::path::Path, depth: usize, out: &mut Vec<(PathBuf, String, bool, usize)>) {
        for (name, is_dir) in self.list_dir(dir) {
            let path = dir.join(&name);
            let open = is_dir && self.files_expanded.contains(&path);
            out.push((path.clone(), name, is_dir, depth));
            if open && out.len() < 5000 {
                self.file_rows(&path, depth + 1, out);
            }
        }
    }

    fn files(&mut self, ui: &mut egui::Ui) {
        let card = ui.max_rect();
        // Ganze Fläche belegen, sonst springt das Panel nach dem Ziehen auf die Inhaltsbreite zurück
        ui.expand_to_include_rect(card);
        ui.painter().rect(card, RADIUS, PANE, Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);
        let head = Rect::from_min_size(card.min, vec2(card.width(), HEAD));
        ui.painter().line_segment([head.left_bottom() + vec2(1.0, 0.0), head.right_bottom() - vec2(1.0, 0.0)], Stroke::new(1.0, BORDER));
        let Some(p) = self.project() else { return };
        let (pname, location) = (p.name.clone(), p.location.clone());
        ui.painter().text(head.left_center() + vec2(14.0, 0.0), egui::Align2::LEFT_CENTER, "Files", semibold(13.0), TEXT);
        ui.painter().with_clip_rect(Rect::from_min_max(head.min, pos2(head.right() - 70.0, head.bottom()))).text(
            head.left_center() + vec2(56.0, 0.0),
            egui::Align2::LEFT_CENTER,
            &pname,
            font(12.0),
            FAINT,
        );
        let mut x = head.right() - 20.0;
        let mut btn = |ui: &mut egui::Ui, icon: Icon, tip: &str| {
            let r = Rect::from_center_size(pos2(x, head.center().y), vec2(26.0, 26.0));
            x -= 28.0;
            let resp = ui.interact(r, ui.id().with(("files_btn", tip)), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(r, 6.0, HOVER);
            }
            icons::draw(ui.painter(), r.center(), icon, if resp.hovered() { TEXT } else { MUTED });
            resp.on_hover_text(tip).clicked()
        };
        if btn(ui, Icon::Close, &self.keybinds.tip("Hide files", Cmd::ToggleFiles)) {
            self.store.files_visible = false;
            self.save();
        }
        if btn(ui, Icon::Reset, "Refresh") {
            self.files_cache.clear();
        }

        let body = Rect::from_min_max(pos2(card.left() + 6.0, head.bottom() + 6.0), card.max - vec2(6.0, 6.0));
        let Location::Local(root) = location else {
            ui.painter().text(body.center_top() + vec2(0.0, 24.0), egui::Align2::CENTER_CENTER, "Only available for local projects", font(12.5), FAINT);
            return;
        };
        let mut rows = Vec::new();
        self.file_rows(&root, 0, &mut rows);
        let mut toggle = None;
        let mut insert = None;
        ui.scope_builder(egui::UiBuilder::new().max_rect(body), |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, 24.0, rows.len(), |ui, range| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for (path, name, is_dir, depth) in &rows[range] {
                    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::click());
                    if resp.hovered() {
                        ui.painter().rect_filled(r, 6.0, HOVER);
                    }
                    let x0 = r.left() + 10.0 + *depth as f32 * 14.0;
                    let c = pos2(x0 + 4.0, r.center().y);
                    let col = if resp.hovered() { TEXT } else { MUTED };
                    if *is_dir {
                        let st = Stroke::new(1.2, FAINT);
                        if self.files_expanded.contains(path) {
                            ui.painter().line_segment([c + vec2(-3.0, -1.5), c + vec2(0.0, 1.5)], st);
                            ui.painter().line_segment([c + vec2(0.0, 1.5), c + vec2(3.0, -1.5)], st);
                        } else {
                            ui.painter().line_segment([c + vec2(-1.5, -3.0), c + vec2(1.5, 0.0)], st);
                            ui.painter().line_segment([c + vec2(1.5, 0.0), c + vec2(-1.5, 3.0)], st);
                        }
                        egui::Image::from_bytes("bytes://folder.svg", FOLDER_SVG)
                            .tint(Color32::from_white_alpha(150))
                            .paint_at(ui, Rect::from_center_size(pos2(x0 + 18.0, r.center().y), vec2(13.0, 13.0)));
                    } else {
                        let f = Rect::from_center_size(pos2(x0 + 18.0, r.center().y), vec2(9.0, 12.0));
                        ui.painter().rect_stroke(f, 1.5, Stroke::new(1.1, FAINT), egui::StrokeKind::Middle);
                    }
                    ui.painter().with_clip_rect(r).text(pos2(x0 + 30.0, r.center().y), egui::Align2::LEFT_CENTER, name, font(12.5), col);
                    if resp.double_clicked() && !*is_dir {
                        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
                    } else if resp.clicked() {
                        if *is_dir {
                            toggle = Some(path.clone());
                        } else {
                            insert = Some(path.clone());
                        }
                    }
                    let tip = if *is_dir { "Click to open" } else { "Click to insert the path into the terminal · double-click to open" };
                    resp.on_hover_text(tip);
                }
            });
        });
        if let Some(t) = toggle {
            if !self.files_expanded.remove(&t) {
                self.files_expanded.insert(t);
            }
        }
        if let Some(path) = insert {
            let rel = path.strip_prefix(&root).unwrap_or(&path).display().to_string();
            if let Some(t) = self.focused.and_then(|f| self.terms.get_mut(&f)) {
                t.write(format!("{} ", shell_quote(&rel)).as_bytes());
            }
        }
    }

    // ---------------------------------------------------------------- Workspace

    fn workspace(&mut self, ui: &mut egui::Ui, input: bool) {
        let rect = ui.available_rect_before_wrap();
        if self.project().is_none() {
            self.welcome(ui, rect);
            return;
        }
        if self.session().is_none_or(|s| s.grid.is_empty()) {
            self.launcher(ui, rect);
            // Terminal aus einer anderen Session hierher ziehen
            if let Some(d) = egui::DragAndDrop::payload::<PaneDrag>(ui.ctx()).map(|d| d.0) {
                if self.project().is_some_and(|p| p.sessions.iter().any(|s| s.grid.contains(d))) && ui.rect_contains_pointer(rect) {
                    let painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("pane_drop")));
                    painter.rect(rect.shrink(4.0), RADIUS, ACCENT.gamma_multiply(0.06), Stroke::new(1.0, ACCENT.gamma_multiply(0.6)), egui::StrokeKind::Inside);
                    painter.text(rect.center(), egui::Align2::CENTER_CENTER, "Move terminal into this session", medium(14.0), ACCENT);
                    if ui.input(|i| i.pointer.any_released()) {
                        let s = self.store.session;
                        self.move_pane(d, None, Some(s));
                    }
                }
            }
            return;
        }

        let ctx = ui.ctx().clone();
        let (cwd, argv) = self.project().unwrap().location.spawn_args();
        let (pi, si) = (self.store.project, self.store.session);
        let mut grid = std::mem::take(&mut self.store.projects[pi].sessions[si].grid);

        for id in grid.panes() {
            if !self.terms.contains_key(&id) {
                match Terminal::spawn(&cwd, argv.as_deref(), id, ctx.clone()) {
                    Ok(mut t) => {
                        if let Some(cmd) = self.pending.remove(&id) {
                            t.write(format!("{cmd}\r").as_bytes());
                        }
                        self.terms.insert(id, t);
                    }
                    Err(e) => eprintln!("failed to start shell: {e}"),
                }
            }
        }

        self.pane_rects.clear();
        let mut actions = Vec::new();
        if let Some(m) = self.maximized.filter(|m| grid.contains(*m)) {
            self.pane(ui, m, rect, &grid, input, &mut actions);
        } else {
            self.grid(ui, &mut grid, rect, input, &mut actions);
        }
        self.store.projects[pi].sessions[si].grid = grid;

        self.pane_drop(ui, &mut actions);
        self.handle_drop(ui);

        // Beendete Shells (z.B. `exit`) schließen ihre Pane
        let dead: Vec<_> = self.terms.iter().filter(|(_, t)| !t.is_alive()).map(|(id, _)| *id).collect();
        actions.extend(dead.into_iter().map(Action::Close));
        for a in actions {
            self.apply(a);
        }
    }

    fn grid(&mut self, ui: &mut egui::Ui, grid: &mut Grid, rect: Rect, input: bool, actions: &mut Vec<Action>) {
        const GAP: f32 = 10.0;
        let mut row_rects = vec![rect];
        if grid.rows.len() == 2 {
            let y = rect.top() + (rect.height() - GAP) * grid.row_ratio;
            row_rects = vec![
                Rect::from_min_max(rect.min, pos2(rect.right(), y)),
                Rect::from_min_max(pos2(rect.left(), y + GAP), rect.max),
            ];
            let handle = Rect::from_min_max(pos2(rect.left(), y), pos2(rect.right(), y + GAP));
            let resp = splitter(ui, handle, ui.id().with("rowsplit"), false);
            if resp.dragged() {
                grid.row_ratio = (grid.row_ratio + resp.drag_delta().y / (rect.height() - GAP)).clamp(0.15, 0.85);
            }
        }

        let snapshot = grid.clone();
        for (r, row_rect) in row_rects.into_iter().enumerate() {
            let ids = grid.rows[r].clone();
            let n = ids.len();
            let total: f32 = grid.widths[r].iter().sum();
            let avail = row_rect.width() - GAP * (n as f32 - 1.0);
            let mut x = row_rect.left();
            for (c, id) in ids.iter().enumerate() {
                let w = avail * grid.widths[r][c] / total;
                let pr = Rect::from_min_max(pos2(x, row_rect.top()), pos2(x + w, row_rect.bottom()));
                self.pane(ui, *id, pr, &snapshot, input, actions);
                x += w;
                if c + 1 < n {
                    let handle = Rect::from_min_max(pos2(x, row_rect.top()), pos2(x + GAP, row_rect.bottom()));
                    let resp = splitter(ui, handle, ui.id().with(("colsplit", r, c)), true);
                    if resp.dragged() {
                        let d = resp.drag_delta().x / avail * total;
                        let min = total * 0.08;
                        let ws = &mut grid.widths[r];
                        let d = d.clamp(min - ws[c], ws[c + 1] - min);
                        ws[c] += d;
                        ws[c + 1] -= d;
                    }
                    x += GAP;
                }
            }
        }
    }

    fn pane(&mut self, ui: &mut egui::Ui, id: PaneId, rect: Rect, grid: &Grid, input: bool, actions: &mut Vec<Action>) {
        let focused = self.focused == Some(id);
        let painter = ui.painter().clone();
        painter.rect_filled(rect, RADIUS, PANE);
        self.pane_rects.push((id, rect));

        let head = Rect::from_min_size(rect.min, vec2(rect.width(), HEAD));
        painter.line_segment([head.left_bottom() + vec2(1.0, 0.0), head.right_bottom() - vec2(1.0, 0.0)], Stroke::new(1.0, BORDER));

        // Header-Fläche zuerst registrieren, damit die Buttons darüber liegen und Klicks bekommen.
        // Über die Kopfzeile lässt sich das Terminal verschieben.
        let head_resp = ui.interact(head, ui.id().with(("head", id)), Sense::click_and_drag());
        if head_resp.drag_started() {
            egui::DragAndDrop::set_payload(ui.ctx(), PaneDrag(id));
        }
        if head_resp.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if head_resp.hovered() && grid.len() > 1 {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        }

        let full = grid.is_full();
        let maxed = self.maximized == Some(id);
        let mut x = head.right() - 20.0;
        let pane_hovered = ui.rect_contains_pointer(rect);
        let mut btn = |ui: &mut egui::Ui, icon: Icon, tip: &str| {
            let r = Rect::from_center_size(pos2(x, head.center().y), vec2(24.0, 24.0));
            x -= 26.0;
            let resp = ui.interact(r, ui.id().with((id, tip)), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(r, 6.0, HOVER);
            }
            let col = if resp.hovered() { TEXT } else if pane_hovered || focused { MUTED } else { FAINT };
            icons::draw(ui.painter(), r.center(), icon, col);
            resp.on_hover_text(tip).clicked()
        };
        let kb = self.keybinds.clone();
        if btn(ui, Icon::Close, &kb.tip("Close", Cmd::Close)) {
            actions.push(Action::Close(id));
        }
        if !full && btn(ui, Icon::Plus, &kb.tip("New terminal", Cmd::NewTerminal)) {
            actions.push(Action::New);
        }
        if grid.len() > 1 {
            let tip = if maxed { kb.tip("Restore", Cmd::Maximize) } else { kb.tip("Maximize", Cmd::Maximize) };
            if btn(ui, if maxed { Icon::Restore } else { Icon::Maximize }, &tip) {
                actions.push(Action::ToggleMax(id));
            }
        }
        if !full && btn(ui, Icon::SplitDown, &kb.tip("Split down", Cmd::SplitDown)) {
            actions.push(Action::SplitDown(id));
        }
        if !full && btn(ui, Icon::SplitRight, &kb.tip("Split right", Cmd::SplitRight)) {
            actions.push(Action::SplitRight(id));
        }
        let buttons_left = x + 12.0;

        let status = self.status(id);
        let others: Vec<(usize, String)> = self.store.projects[self.store.project]
            .sessions
            .iter()
            .enumerate()
            .filter(|(i, s)| *i != self.store.session && !s.grid.is_full())
            .map(|(i, s)| (i, s.name.clone()))
            .collect();
        let session = self.store.projects[self.store.project].sessions.get(self.store.session);
        let custom = session.and_then(|s| s.names.get(&id).cloned());
        let agent = session.and_then(|s| s.agents.get(&id)).and_then(|a| agents::get(a));
        let location = self.store.projects[self.store.project].location.display();
        let Some(term) = self.terms.get_mut(&id) else { return };
        let process = term.foreground();
        // Laufendes Programm hat Vorrang vor dem Start-Agent (wie in der Sidebar).
        let agent = agents::by_process(&process).or(if process == term.shell_name { None } else { agent });

        // Titel: Status-Punkt (+ Agent-Logo) und der aktuell laufende Befehl bzw. eigener Name
        let tp = painter.with_clip_rect(Rect::from_min_max(head.min, pos2(buttons_left, head.max.y)));
        let mut tx = head.left() + 14.0;
        let dot = if !term.is_alive() { FAINT } else if status == Status::Idle { GREEN } else { status_color(status) };
        tp.circle_filled(pos2(tx + 3.0, head.center().y), 3.5, dot);
        tx += 14.0;
        if let Some(a) = agent {
            let r = Rect::from_min_size(pos2(tx, head.center().y - 8.0), vec2(16.0, 16.0));
            a.paint_logo(ui, r.intersect(tp.clip_rect()), false);
            tx += 24.0;
        }
        let mut job = egui::text::LayoutJob::default();
        let fmt = |f: egui::FontId, color: Color32| egui::TextFormat { font_id: f, color, valign: egui::Align::Center, ..Default::default() };
        match &custom {
            Some(n) => {
                job.append(n, 0.0, fmt(medium(12.5), TEXT));
                job.append(&process, 10.0, fmt(mono(12.0), MUTED));
            }
            None => job.append(&process, 0.0, fmt(mono(12.5), TEXT)),
        }
        job.append(&location, 12.0, fmt(mono(11.5), FAINT));
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        // Zustand rechts neben dem Titel, vor den Buttons
        let pill_row = Rect::from_min_max(head.min, pos2(buttons_left, head.max.y));
        let pill = if status == Status::Idle { None } else { status_pill(ui, status, pill_row) };
        let tp = tp.with_clip_rect(Rect::from_min_max(head.min, pos2(pill.map_or(buttons_left, |p| p.left() - 8.0), head.max.y)));
        tp.galley(pos2(tx, head.center().y - galley.size().y / 2.0), galley, TEXT);

        let body = Rect::from_min_max(pos2(rect.left() + 2.0, head.bottom()), rect.max - vec2(2.0, 4.0));
        let resp = term.ui(ui, body, ui.id().with(("term", id)), focused && input);

        if resp.clicked() || resp.secondary_clicked() || head_resp.clicked() || head_resp.secondary_clicked() {
            actions.push(Action::Focus(id));
        }
        if head_resp.double_clicked() {
            actions.push(Action::Rename(id));
        }
        let menu = |ui: &mut egui::Ui, actions: &mut Vec<Action>| {
            ui.set_min_width(190.0);
            if ui.add_enabled(!full, menu_button("Split right", &kb.text(Cmd::SplitRight))).clicked() {
                actions.push(Action::SplitRight(id));
            }
            if ui.add_enabled(!full, menu_button("Split down", &kb.text(Cmd::SplitDown))).clicked() {
                actions.push(Action::SplitDown(id));
            }
            if grid.len() > 1 && ui.add(menu_button(if maxed { "Restore" } else { "Maximize" }, &kb.text(Cmd::Maximize))).clicked() {
                actions.push(Action::ToggleMax(id));
            }
            if menu_item(ui, "Rename…") {
                actions.push(Action::Rename(id));
            }
            ui.menu_button("Move to session", |ui| {
                ui.set_min_width(170.0);
                for (i, name) in &others {
                    if menu_item(ui, name) {
                        actions.push(Action::MoveToSession(id, Some(*i)));
                    }
                }
                if !others.is_empty() {
                    ui.separator();
                }
                if menu_item(ui, "New session") {
                    actions.push(Action::MoveToSession(id, None));
                }
            });
            ui.separator();
            if menu_item_danger(ui, "Close terminal") {
                actions.push(Action::Close(id));
            }
        };
        resp.context_menu(|ui| menu(ui, actions));
        head_resp.context_menu(|ui| menu(ui, actions));

        // Hervorhebung, wenn Dateien über diese Pane gezogen werden
        let dragging = ui.input(|i| !i.raw.hovered_files.is_empty());
        let pointer = ui.input(|i| i.pointer.latest_pos());
        let drop_here = dragging && pointer.map_or(focused, |p| rect.contains(p));
        if drop_here {
            painter.rect_filled(body, 0.0, ACCENT.gamma_multiply(0.08));
            painter.text(body.center(), egui::Align2::CENTER_CENTER, "Drop to insert path", medium(14.0), ACCENT);
        }

        if egui::DragAndDrop::payload::<PaneDrag>(ui.ctx()).is_some_and(|d| d.0 == id) {
            painter.rect_filled(rect, RADIUS, BG.gamma_multiply(0.55));
        }
        let stroke = if drop_here { Stroke::new(1.0, ACCENT.gamma_multiply(0.75)) } else if focused { Stroke::new(1.0, BORDER_STRONG) } else { Stroke::new(1.0, BORDER) };
        painter.rect_stroke(rect, RADIUS, stroke, egui::StrokeKind::Inside);
    }

    /// Zeigt beim Ziehen eines Terminals, wo es landet, und verschiebt es beim Loslassen.
    fn pane_drop(&mut self, ui: &egui::Ui, actions: &mut Vec<Action>) {
        let Some(drag) = egui::DragAndDrop::payload::<PaneDrag>(ui.ctx()).map(|d| d.0) else { return };
        let Some(pos) = ui.input(|i| i.pointer.latest_pos()) else { return };
        // Nur innerhalb eines Projekts (gleicher Ordner bzw. Server)
        if !self.project().is_some_and(|p| p.sessions.iter().any(|s| s.grid.contains(drag))) {
            return;
        }
        let Some(&(target, r)) = self.pane_rects.iter().find(|(id, r)| *id != drag && r.contains(pos)) else { return };
        let fx = (pos.x - r.left()) / r.width();
        let fy = (pos.y - r.top()) / r.height();
        let side = if (0.3..0.7).contains(&fx) && (0.3..0.7).contains(&fy) {
            Side::Center
        } else {
            [(fx, Side::Left), (1.0 - fx, Side::Right), (fy, Side::Top), (1.0 - fy, Side::Bottom)]
                .into_iter()
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(_, s)| s)
                .unwrap()
        };
        let Some(grid) = self.session().map(|s| s.grid.clone()) else { return };
        let mut test = grid.clone();
        let ok = if grid.contains(drag) { test.move_pane(drag, target, side) } else { test.place(drag, target, side) };
        let zone = match side {
            Side::Left => Rect::from_min_max(r.min, pos2(r.center().x, r.bottom())),
            Side::Right => Rect::from_min_max(pos2(r.center().x, r.top()), r.max),
            Side::Top => Rect::from_min_max(r.min, pos2(r.right(), r.center().y)),
            Side::Bottom => Rect::from_min_max(pos2(r.left(), r.center().y), r.max),
            Side::Center => r,
        }
        .shrink(6.0);
        let painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("pane_drop")));
        let col = if ok { ACCENT } else { RED };
        painter.rect(zone, RADIUS, col.gamma_multiply(0.08), Stroke::new(1.0, col.gamma_multiply(0.7)), egui::StrokeKind::Inside);
        let text = match (ok, side) {
            (false, _) => "No space here",
            (true, Side::Center) => "Swap",
            (true, Side::Left) => "Move left",
            (true, Side::Right) => "Move right",
            (true, Side::Top) => "Move up",
            (true, Side::Bottom) => "Move down",
        };
        painter.text(zone.center(), egui::Align2::CENTER_CENTER, text, medium(13.5), col);
        if ok && ui.input(|i| i.pointer.any_released()) {
            actions.push(Action::Move(drag, target, side));
        }
    }

    /// Fügt Pfade von per Drag & Drop abgelegten Dateien in das Terminal unter dem Mauszeiger ein.
    fn handle_drop(&mut self, ui: &egui::Ui) {
        let (files, pointer) = ui.input(|i| (i.raw.dropped_files.clone(), i.pointer.latest_pos()));
        if files.is_empty() {
            return;
        }
        let target = pointer
            .and_then(|p| self.pane_rects.iter().find(|(_, r)| r.contains(p)).map(|(id, _)| *id))
            .or(self.focused);
        let Some(id) = target else { return };
        let text: Vec<String> = files.iter().map(|f| shell_quote(&f.path().display().to_string())).collect();
        if let Some(t) = self.terms.get_mut(&id) {
            if !text.is_empty() {
                t.write(format!("{} ", text.join(" ")).as_bytes());
            }
        }
        self.focused = Some(id);
    }

    fn welcome(&mut self, ui: &mut egui::Ui, rect: Rect) {
        let c = rect.center();
        let p = ui.painter();
        p.text(c - vec2(0.0, 40.0), egui::Align2::CENTER_CENTER, "Welcome to Pixel Code", semibold(24.0), TEXT);
        p.text(c - vec2(0.0, 10.0), egui::Align2::CENTER_CENTER, "Add a project to start coding.", font(14.0), MUTED);
        let b = Rect::from_center_size(c + vec2(0.0, 36.0), vec2(150.0, 36.0));
        let resp = ui.put(b, accent_button("Add project"));
        if resp.clicked() {
            self.modal = Some(Modal::Add(AddDialog::new()));
        }
    }

    fn launcher(&mut self, ui: &mut egui::Ui, rect: Rect) {
        self.refresh_installed();
        let p = self.project().unwrap();
        let title = self.session().map_or(p.name.clone(), |s| s.name.clone());
        let location = p.location.display();
        let remote = p.location.is_remote();

        // Auf entfernten Rechnern lässt sich nicht lokal prüfen, was installiert ist.
        let (installed, missing): (Vec<&'static Agent>, Vec<&'static Agent>) =
            AGENTS.iter().partition(|a| remote || self.installed.get(a.id).copied().unwrap_or(false));

        let mut chosen = None;
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                let width = (rect.width() - 80.0).clamp(300.0, 860.0);
                let margin = ((rect.width() - width) / 2.0).max(0.0);
                ui.horizontal(|ui| {
                    ui.add_space(margin);
                    ui.vertical(|ui| {
                        ui.set_width(width);
                        let content_h = 120.0 + 130.0 * ((installed.len() + 3) / 4) as f32 + 140.0 * ((missing.len() + 3) / 4) as f32;
                        ui.add_space(((rect.height() - content_h) / 2.0 - 30.0).max(40.0));

                        ui.label(RichText::new(title).font(semibold(26.0)));
                        ui.add_space(2.0);
                        ui.label(RichText::new(format!("Start an agent or a terminal in {location}")).font(font(14.0)).color(MUTED));
                        ui.add_space(26.0);

                        section_label(ui, "INSTALLED", installed.len());
                        if let Some(a) = agent_grid(ui, &installed, false) {
                            chosen = Some(Launch::Agent(a));
                        }
                        if !missing.is_empty() {
                            ui.add_space(24.0);
                            section_label(ui, "NOT INSTALLED", missing.len());
                            if let Some(a) = agent_grid(ui, &missing, true) {
                                if a.install.is_some() {
                                    chosen = Some(Launch::Install(a));
                                }
                            }
                        }
                        ui.add_space(26.0);
                        if let Some(b) = self.keybinds.get(Cmd::NewTerminal) {
                            ui.horizontal(|ui| {
                                for k in b.text().split('+').filter(|k| !k.is_empty()) {
                                    keycap(ui, k);
                                }
                                ui.label(RichText::new("new terminal").color(FAINT).font(font(12.0)));
                            });
                        }
                        ui.add_space(40.0);
                    });
                });
            });
        });
        if let Some(l) = chosen {
            self.launch(l);
        }
    }

    // ---------------------------------------------------------------- Dialoge

    fn modals(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.browse_rx {
            match rx.try_recv() {
                Ok(folder) => {
                    if let (Some(f), Some(Modal::Add(d))) = (folder, &mut self.modal) {
                        d.folder = Some(f);
                        d.error = None;
                    }
                    self.browse_rx = None;
                }
                Err(mpsc::TryRecvError::Empty) => ctx.request_repaint_after(Duration::from_millis(100)),
                Err(mpsc::TryRecvError::Disconnected) => self.browse_rx = None,
            }
        }

        let frame = egui::Frame::new()
            .fill(ELEVATED)
            .stroke(Stroke::new(1.0, BORDER_STRONG))
            .corner_radius(16)
            .inner_margin(24)
            .shadow(egui::Shadow { offset: [0, 16], blur: 48, spread: 0, color: Color32::from_black_alpha(160) });

        match &mut self.modal {
            Some(Modal::Add(d)) => {
                let mut submit = false;
                let mut cancel = false;
                let mut browse = false;
                let browsing = self.browse_rx.is_some();
                let resp = egui::Modal::new(egui::Id::new("add_project"))
                    .backdrop_color(Color32::from_black_alpha(150))
                    .frame(frame)
                    .show(ctx, |ui| {
                        ui.set_width(500.0);
                        ui.spacing_mut().item_spacing.y = 6.0;
                        ui.label(RichText::new("Add project").font(semibold(19.0)));
                        ui.label(RichText::new("Open a folder on this computer or connect to a remote machine.").color(MUTED));
                        ui.add_space(16.0);

                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            let w = (ui.available_width() - 10.0) / 2.0;
                            for (kind, icon, title, desc) in [
                                (AddKind::Local, Icon::Folder, "Local folder", "Browse this computer"),
                                (AddKind::Ssh, Icon::Server, "Remote via SSH", "Connect to a server"),
                            ] {
                                let (r, resp) = ui.allocate_exact_size(vec2(w, 68.0), Sense::click());
                                let sel = d.kind == kind;
                                let fill = if sel { SELECTED } else if resp.hovered() { HOVER } else { SURFACE };
                                ui.painter().rect(r, 12.0, fill, Stroke::new(1.0, if sel { BORDER_STRONG.gamma_multiply(1.6) } else { BORDER }), egui::StrokeKind::Inside);
                                let ib = Rect::from_center_size(r.left_center() + vec2(30.0, 0.0), vec2(34.0, 34.0));
                                ui.painter().rect_filled(ib, 9.0, if sel { BORDER_STRONG } else { ELEVATED });
                                icons::draw(ui.painter(), ib.center(), icon, if sel { ACCENT } else { MUTED });
                                ui.painter().text(r.left_center() + vec2(58.0, -9.0), egui::Align2::LEFT_CENTER, title, medium(13.5), TEXT);
                                ui.painter().text(r.left_center() + vec2(58.0, 10.0), egui::Align2::LEFT_CENTER, desc, font(12.0), MUTED);
                                if resp.clicked() {
                                    d.kind = kind;
                                    d.error = None;
                                }
                            }
                        });
                        ui.add_space(16.0);

                        match d.kind {
                            AddKind::Local => {
                                field_label(ui, "Folder");
                                let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 72.0), Sense::click());
                                let hov = resp.hovered() && !browsing;
                                ui.painter().rect(r, 12.0, if hov { HOVER } else { INPUT }, Stroke::new(1.0, if hov { BORDER_STRONG } else { BORDER }), egui::StrokeKind::Inside);
                                let ib = Rect::from_center_size(r.left_center() + vec2(32.0, 0.0), vec2(36.0, 36.0));
                                ui.painter().rect_filled(ib, 9.0, ELEVATED);
                                icons::draw(ui.painter(), ib.center(), Icon::Folder, if d.folder.is_some() { ACCENT } else { MUTED });
                                let (line1, line2) = match &d.folder {
                                    Some(p) => (
                                        p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "/".into()),
                                        p.display().to_string(),
                                    ),
                                    None => ("Choose a folder".to_string(), "Click to browse your files".to_string()),
                                };
                                let clip = ui.painter().with_clip_rect(Rect::from_min_max(r.min, pos2(r.right() - 110.0, r.bottom())));
                                clip.text(r.left_center() + vec2(62.0, -9.0), egui::Align2::LEFT_CENTER, line1, medium(13.5), TEXT);
                                clip.text(r.left_center() + vec2(62.0, 10.0), egui::Align2::LEFT_CENTER, line2, mono(11.5), MUTED);
                                let pill = Rect::from_center_size(r.right_center() - vec2(56.0, 0.0), vec2(88.0, 30.0));
                                ui.painter().rect(pill, 8.0, ELEVATED, Stroke::new(1.0, BORDER_STRONG), egui::StrokeKind::Inside);
                                let label = if browsing { "Opening…" } else if d.folder.is_some() { "Change" } else { "Browse" };
                                ui.painter().text(pill.center(), egui::Align2::CENTER_CENTER, label, medium(12.5), TEXT);
                                if resp.clicked() && !browsing {
                                    browse = true;
                                }
                            }
                            AddKind::Ssh => {
                                ui.horizontal(|ui| {
                                    let w = ui.available_width() - 16.0;
                                    ui.vertical(|ui| {
                                        ui.set_width(w * 0.36);
                                        field_label(ui, "User");
                                        ui.add(input(&mut d.user, "user"));
                                    });
                                    ui.vertical(|ui| {
                                        ui.set_width(w * 0.44);
                                        field_label(ui, "Host");
                                        ui.add(input(&mut d.host, "example.com"));
                                    });
                                    ui.vertical(|ui| {
                                        ui.set_width(w * 0.2);
                                        field_label(ui, "Port");
                                        ui.add(input(&mut d.port, "22"));
                                    });
                                });
                                ui.add_space(6.0);
                                field_label(ui, "Remote path");
                                ui.add(input(&mut d.remote_path, "~/projects/app"));
                                ui.label(RichText::new("Uses your SSH config and keys.").font(font(12.0)).color(FAINT));
                            }
                        }
                        ui.add_space(8.0);
                        field_label(ui, "Name");
                        ui.add(input(&mut d.name, "Defaults to the folder name"));

                        if let Some(e) = &d.error {
                            ui.add_space(4.0);
                            ui.label(RichText::new(e).color(RED));
                        }
                        ui.add_space(18.0);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(accent_button("Add project")).clicked() {
                                submit = true;
                            }
                            if ui.add(ghost_button("Cancel", 84.0)).clicked() {
                                cancel = true;
                            }
                        });
                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            submit = true;
                        }
                    });
                if browse {
                    let (tx, rx) = mpsc::channel();
                    std::thread::spawn(move || {
                        let f = rfd::FileDialog::new()
                            .set_title("Choose project folder")
                            .set_directory(dirs::home_dir().unwrap_or_default())
                            .pick_folder();
                        let _ = tx.send(f);
                    });
                    self.browse_rx = Some(rx);
                }
                if submit {
                    self.submit_add();
                } else if cancel || (resp.should_close() && !browsing) {
                    self.modal = None;
                }
            }
            Some(Modal::Rename(target, name)) => {
                let mut done = false;
                let mut cancel = false;
                let title = match target {
                    Rename::Project(_) => "Rename project",
                    Rename::Session(..) => "Rename session",
                    Rename::Pane(_) => "Rename terminal",
                };
                let is_pane = matches!(target, Rename::Pane(_));
                let resp = egui::Modal::new(egui::Id::new("rename"))
                    .backdrop_color(Color32::from_black_alpha(150))
                    .frame(frame)
                    .show(ctx, |ui| {
                        ui.set_width(380.0);
                        ui.label(RichText::new(title).font(semibold(17.0)));
                        if is_pane {
                            ui.label(RichText::new("Leave empty to show the running command.").color(MUTED));
                        }
                        ui.add_space(12.0);
                        let r = ui.add(input(name, "Name"));
                        r.request_focus();
                        ui.add_space(18.0);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(accent_button("Save")).clicked() {
                                done = true;
                            }
                            if ui.add(ghost_button("Cancel", 84.0)).clicked() {
                                cancel = true;
                            }
                        });
                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            done = true;
                        }
                    });
                if done {
                    let (target, name) = (*target, name.trim().to_string());
                    match target {
                        Rename::Project(p) if !name.is_empty() => self.store.projects[p].name = name,
                        Rename::Session(p, s) if !name.is_empty() => {
                            if let Some(s) = self.store.projects[p].sessions.get_mut(s) {
                                s.name = name;
                            }
                        }
                        Rename::Pane(id) => {
                            for s in self.store.projects.iter_mut().flat_map(|p| &mut p.sessions) {
                                if s.grid.contains(id) {
                                    if name.is_empty() {
                                        s.names.remove(&id);
                                    } else {
                                        s.names.insert(id, name.clone());
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    self.modal = None;
                    self.save();
                } else if cancel || resp.should_close() {
                    self.modal = None;
                }
            }
            Some(Modal::RemoveProject(i)) => {
                let i = *i;
                let Some(p) = self.store.projects.get(i) else {
                    self.modal = None;
                    return;
                };
                let terms: usize = p.sessions.iter().map(|s| s.grid.len()).sum();
                let name = p.name.clone();
                let mut confirm = false;
                let mut cancel = false;
                let resp = egui::Modal::new(egui::Id::new("remove"))
                    .backdrop_color(Color32::from_black_alpha(150))
                    .frame(frame)
                    .show(ctx, |ui| {
                        ui.set_width(400.0);
                        ui.label(RichText::new(format!("Remove “{name}”?")).font(semibold(17.0)));
                        ui.add_space(2.0);
                        let extra = if terms > 0 { format!(" {terms} running terminal(s) will be closed.") } else { String::new() };
                        ui.label(RichText::new(format!("The project is removed from Pixel Code. Files on disk are not touched.{extra}")).color(MUTED));
                        ui.add_space(18.0);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let danger = egui::Button::new(RichText::new("Remove").color(Color32::WHITE).font(medium(13.0)))
                                .fill(Color32::from_rgb(0xdc, 0x3d, 0x3d))
                                .corner_radius(8)
                                .min_size(vec2(96.0, 34.0));
                            if ui.add(danger).clicked() {
                                confirm = true;
                            }
                            if ui.add(ghost_button("Cancel", 84.0)).clicked() {
                                cancel = true;
                            }
                        });
                    });
                if confirm {
                    self.modal = None;
                    self.remove_project(i);
                } else if cancel || resp.should_close() {
                    self.modal = None;
                }
            }
            None => {}
        }
    }

    fn submit_add(&mut self) {
        let Some(Modal::Add(d)) = &mut self.modal else { return };
        let (location, fallback) = match d.kind {
            AddKind::Local => match &d.folder {
                Some(f) if f.is_dir() => {
                    let n = f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "/".into());
                    (Location::Local(f.clone()), n)
                }
                _ => {
                    d.error = Some("Please choose a folder.".into());
                    return;
                }
            },
            AddKind::Ssh => {
                let host = d.host.trim().to_string();
                let Ok(port) = d.port.trim().parse::<u16>() else {
                    d.error = Some("Port must be a number.".into());
                    return;
                };
                if host.is_empty() {
                    d.error = Some("Please enter a host.".into());
                    return;
                }
                let path = d.remote_path.trim().to_string();
                let n = path
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .filter(|s| !s.is_empty() && *s != "~")
                    .unwrap_or(&host)
                    .to_string();
                (Location::Ssh { user: d.user.trim().into(), host, port, path }, n)
            }
        };
        let name = if d.name.trim().is_empty() { fallback } else { d.name.trim().to_string() };
        self.store.projects.push(Project { name, location, sessions: Vec::new(), expanded: true });
        self.modal = None;
        let p = self.store.projects.len() - 1;
        self.new_session(p);
    }
}

// ---------------------------------------------------------------- Widgets

fn agent_grid(ui: &mut egui::Ui, list: &[&'static Agent], missing: bool) -> Option<&'static Agent> {
    const GAP: f32 = 12.0;
    let width = ui.available_width();
    let cols = ((width + GAP) / (190.0 + GAP)).floor().clamp(2.0, 4.0) as usize;
    let w = (width - GAP * (cols as f32 - 1.0)) / cols as f32;
    let mut chosen = None;
    for row in list.chunks(cols) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            for a in row {
                let (r, resp) = ui.allocate_exact_size(vec2(w, 104.0), Sense::click());
                let hov = resp.hovered();
                let fill = if hov { HOVER } else { SURFACE };
                let border = if hov && !missing { BORDER_STRONG.gamma_multiply(1.6) } else if hov { BORDER_STRONG } else { BORDER };
                ui.painter().rect(r, 14.0, fill, Stroke::new(1.0, border), egui::StrokeKind::Inside);

                let logo = Rect::from_min_size(r.min + vec2(16.0, 16.0), vec2(30.0, 30.0));
                a.paint_logo(ui, logo, missing);
                ui.painter().text(
                    r.left_bottom() + vec2(16.0, -36.0),
                    egui::Align2::LEFT_CENTER,
                    a.name,
                    medium(14.0),
                    if missing { MUTED } else { TEXT },
                );
                let sub = a.bin.unwrap_or("shell");
                ui.painter().text(r.left_bottom() + vec2(16.0, -17.0), egui::Align2::LEFT_CENTER, sub, mono(11.5), FAINT);

                let tip = if missing {
                    match a.install {
                        Some(cmd) => {
                            let pill = Rect::from_min_size(pos2(r.right() - 72.0, r.top() + 16.0), vec2(58.0, 24.0));
                            let pf = if hov { ACCENT } else { ELEVATED };
                            ui.painter().rect(pill, 12.0, pf, Stroke::new(1.0, if hov { ACCENT } else { BORDER_STRONG }), egui::StrokeKind::Inside);
                            ui.painter().text(pill.center(), egui::Align2::CENTER_CENTER, "Install", medium(11.5), if hov { BG } else { MUTED });
                            format!("Run: {cmd}")
                        }
                        None => format!("`{sub}` was not found on this computer"),
                    }
                } else {
                    let arrow = r.right_top() + vec2(-26.0, 30.0);
                    if hov {
                        icons::draw(ui.painter(), arrow, Icon::Plus, ACCENT);
                    }
                    format!("Launch {}", a.name)
                };
                if resp.on_hover_text(tip).clicked() {
                    chosen = Some(*a);
                }
            }
        });
        ui.add_space(GAP - ui.spacing().item_spacing.y);
    }
    chosen
}

fn section_label(ui: &mut egui::Ui, text: &str, count: usize) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(text).font(semibold(11.0)).color(FAINT).extra_letter_spacing(1.2));
        ui.label(RichText::new(count.to_string()).font(medium(11.0)).color(FAINT));
    });
    ui.add_space(6.0);
}

fn keycap(ui: &mut egui::Ui, text: &str) {
    let galley = ui.painter().layout_no_wrap(text.to_string(), medium(11.0), MUTED);
    let (r, _) = ui.allocate_exact_size(vec2(galley.size().x + 14.0, 22.0), Sense::hover());
    ui.painter().rect(r, 6.0, SURFACE, Stroke::new(1.0, BORDER_STRONG), egui::StrokeKind::Inside);
    ui.painter().galley(r.center() - galley.size() / 2.0, galley, MUTED);
}

/// Reihenfolge für die Zusammenfassung mehrerer Terminals (höher = wichtiger).
fn urgency(s: Status) -> u8 {
    match s {
        Status::Idle => 0,
        Status::Done => 1,
        Status::Working => 2,
        Status::Error => 3,
        Status::Question => 4,
        Status::Permission => 5,
    }
}

fn status_color(s: Status) -> Color32 {
    match s {
        Status::Idle => FAINT,
        Status::Working | Status::Done => GREEN,
        Status::Question => BLUE,
        Status::Permission => AMBER,
        Status::Error => RED,
    }
}

/// Kleines Etikett rechts in einer Zeile ("Working", "Permission", ...). Gibt seine Fläche zurück.
fn status_pill(ui: &egui::Ui, st: Status, row: Rect) -> Option<Rect> {
    if st == Status::Idle {
        return None;
    }
    let col = status_color(st);
    let galley = ui.painter().layout_no_wrap(st.label().to_string(), medium(10.5), col);
    let w = galley.size().x + if st == Status::Done { 26.0 } else { 14.0 };
    let r = Rect::from_center_size(pos2(row.right() - 6.0 - w / 2.0, row.center().y), vec2(w, 18.0));
    ui.painter().rect_filled(r, 9.0, col.gamma_multiply(0.14));
    let mut x = r.left() + 7.0;
    if st == Status::Done {
        icons::draw(ui.painter(), pos2(x + 5.0, r.center().y), Icon::Check, col);
        x += 12.0;
    }
    ui.painter().galley(pos2(x, r.center().y - galley.size().y / 2.0), galley, col);
    Some(r)
}

/// Kreis um ein Terminal-Icon: grüner drehender Ladebogen = arbeitet, blau = Frage,
/// gelb = wartet auf Freigabe (beide pulsierend), grün = fertig, rot = Fehler.
fn status_ring(p: &egui::Painter, c: egui::Pos2, status: Status, t: f64, r: f32) {
    let pulse = (0.5 + 0.5 * (t * 5.0).sin()) as f32;
    let col = status_color(status);
    match status {
        Status::Idle => {}
        Status::Working => {
            p.circle_stroke(c, r, Stroke::new(1.8, col.gamma_multiply(0.18)));
            let start = (t * 5.0) as f32;
            let pts: Vec<egui::Pos2> = (0..=24).map(|k| c + egui::Vec2::angled(start + k as f32 / 24.0 * 4.2) * r).collect();
            p.add(egui::Shape::line(pts, Stroke::new(1.8, col)));
        }
        Status::Question | Status::Permission => {
            let a = 0.3 + 0.7 * pulse;
            p.circle_filled(c, r, col.gamma_multiply(0.15 * a));
            p.circle_stroke(c, r, Stroke::new(1.8, col.gamma_multiply(a)));
        }
        Status::Done | Status::Error => {
            p.circle_stroke(c, r, Stroke::new(1.8, col.gamma_multiply(0.8)));
        }
    }
}

const APP_ICON_SVG: &[u8] = include_bytes!("../assets/icon.svg");
const FOLDER_SVG: &[u8] = include_bytes!("../assets/logos/folder.svg");

fn menu_item(ui: &mut egui::Ui, text: &str) -> bool {
    ui.add(egui::Button::new(RichText::new(text).font(font(13.0))).frame(false).min_size(vec2(ui.available_width(), 28.0)))
        .clicked()
}

fn menu_item_danger(ui: &mut egui::Ui, text: &str) -> bool {
    ui.add(egui::Button::new(RichText::new(text).font(font(13.0)).color(RED)).frame(false).min_size(vec2(ui.available_width(), 28.0)))
        .clicked()
}

fn menu_button<'a>(text: &'a str, shortcut: &str) -> egui::Button<'a> {
    egui::Button::new(RichText::new(text).font(font(13.0)))
        .shortcut_text(RichText::new(shortcut.to_string()).font(font(11.5)).color(FAINT))
        .frame(false)
        .min_size(vec2(190.0, 28.0))
}

fn splitter(ui: &egui::Ui, handle: Rect, id: egui::Id, vertical: bool) -> egui::Response {
    let resp = ui.interact(handle, id, Sense::drag());
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(if vertical { egui::CursorIcon::ResizeHorizontal } else { egui::CursorIcon::ResizeVertical });
        let line = if vertical {
            Rect::from_center_size(handle.center(), vec2(2.0, (handle.height() - 40.0).max(0.0)))
        } else {
            Rect::from_center_size(handle.center(), vec2((handle.width() - 40.0).max(0.0), 2.0))
        };
        ui.painter().rect_filled(line, 1.0, ACCENT.gamma_multiply(0.6));
    }
    resp
}

fn field_label(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).font(medium(12.0)).color(MUTED));
}

fn input<'a>(s: &'a mut String, hint: &'a str) -> egui::TextEdit<'a> {
    egui::TextEdit::singleline(s)
        .hint_text(RichText::new(hint).color(FAINT))
        .margin(vec2(12.0, 9.0))
        .desired_width(f32::INFINITY)
}

fn accent_button(text: &str) -> egui::Button<'_> {
    egui::Button::new(RichText::new(text).color(BG).font(medium(13.0)))
        .fill(ACCENT)
        .stroke(Stroke::NONE)
        .corner_radius(8)
        .min_size(vec2(116.0, 36.0))
}

fn ghost_button(text: &str, w: f32) -> egui::Button<'_> {
    egui::Button::new(RichText::new(text).font(medium(13.0)))
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, BORDER_STRONG))
        .corner_radius(8)
        .min_size(vec2(w, 36.0))
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_status(&ctx);
        self.telegram();
        // Telegram-Nachrichten und Agent-Status auch ohne Eingaben weiter verarbeiten
        ctx.request_repaint_after(Duration::from_millis(500));

        if let Some(s) = &mut self.settings {
            if !settings::show(ui, s) {
                self.keybinds = s.keybinds.clone();
                self.settings = None;
            }
            return;
        }

        let input = self.modal.is_none();
        if input {
            self.shortcuts(&ctx);
        }

        let side = egui::Panel::left("sidebar")
            .resizable(true)
            .show_separator_line(false)
            .default_size(330.0)
            .size_range(280.0..=520.0)
            .frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin { left: 10, right: 0, top: 10, bottom: 10 }))
            .show(ui, |ui| self.sidebar(ui));
        self.sidebar_width = side.response.rect.width() - 10.0;

        if self.store.files_visible && self.project().is_some() {
            egui::Panel::right("files")
                .resizable(true)
                .show_separator_line(false)
                .default_size(280.0)
                .size_range(200.0..=520.0)
                .frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin { left: 0, right: 10, top: 10, bottom: 10 }))
                .show(ui, |ui| self.files(ui));
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(10))
            .show(ui, |ui| self.workspace(ui, input));

        self.modals(&ctx);

        // Vorschau des gezogenen Terminals am Mauszeiger
        if let (Some(d), Some(pos)) = (egui::DragAndDrop::payload::<PaneDrag>(&ctx), ctx.pointer_latest_pos()) {
            let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("drag_ghost")));
            let label = self.pane_label(d.0);
            let galley = painter.layout_no_wrap(label, medium(12.5), TEXT);
            let r = Rect::from_min_size(pos + vec2(14.0, 12.0), galley.size() + vec2(20.0, 12.0));
            painter.rect(r, 8.0, ELEVATED, Stroke::new(1.0, BORDER_STRONG), egui::StrokeKind::Inside);
            painter.galley(r.min + vec2(10.0, 6.0), galley, TEXT);
        }
    }

    fn on_exit(&mut self) {
        App::save(self);
    }
}
