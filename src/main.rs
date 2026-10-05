mod agents;
mod icons;
mod layout;
mod plugins;
mod settings;
mod telegram;
mod terminal;
mod theme;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use agents::{Agent, AGENTS};
use eframe::egui::{self, pos2, vec2, Color32, Rect, RichText, Sense, Stroke};
use icons::Icon;
use layout::{Grid, PaneId};
use serde::{Deserialize, Serialize};
use terminal::{Status, Terminal};
use theme::*;

fn main() -> eframe::Result {
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

enum Action {
    Focus(PaneId),
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
        };
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

    fn shortcuts(&mut self, ctx: &egui::Context) {
        let cs = egui::Modifiers::CTRL | egui::Modifiers::SHIFT;
        let c = egui::Modifiers::CTRL;
        let multi = self.session().is_some_and(|s| s.grid.len() > 1);
        let (d, e, w, t, m, next, prev) = ctx.input_mut(|i| {
            (
                i.consume_key(cs, egui::Key::D),
                i.consume_key(cs, egui::Key::E),
                i.consume_key(c, egui::Key::W) || i.consume_key(cs, egui::Key::W),
                i.consume_key(c, egui::Key::T) || i.consume_key(cs, egui::Key::T),
                i.consume_key(cs, egui::Key::Enter),
                i.consume_key(c, egui::Key::Tab) || (multi && i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)),
                i.consume_key(cs, egui::Key::Tab),
            )
        });
        if next || prev {
            let panes = self.session().map(|s| s.grid.panes()).unwrap_or_default();
            if !panes.is_empty() {
                let cur = self.focused.and_then(|f| panes.iter().position(|p| *p == f)).unwrap_or(0);
                let n = panes.len();
                let i = if next { (cur + 1) % n } else { (cur + n - 1) % n };
                self.focused = Some(panes[i]);
                if self.maximized.is_some() {
                    self.maximized = Some(panes[i]);
                }
            }
        }
        if t {
            self.launch(Launch::Agent(&AGENTS[0]));
        }
        if let Some(f) = self.focused {
            if d {
                self.apply(Action::SplitRight(f));
            }
            if e {
                self.apply(Action::SplitDown(f));
            }
            if w {
                self.apply(Action::Close(f));
            }
            if m {
                self.apply(Action::ToggleMax(f));
            }
        }
    }

    // ---------------------------------------------------------------- Telegram

    /// Anzeigename einer Pane: eigener Name, sonst Agent, sonst laufendes Programm.
    fn pane_label(&self, id: PaneId) -> String {
        let session = self.store.projects.iter().flat_map(|p| &p.sessions).find(|s| s.grid.contains(id));
        if let Some(n) = session.and_then(|s| s.names.get(&id)) {
            return n.clone();
        }
        let fg = self.terms.get(&id).map(|t| t.foreground()).unwrap_or_default();
        agents::by_process(&fg)
            .or_else(|| session.and_then(|s| s.agents.get(&id)).and_then(|a| agents::get(a)))
            .map_or(fg, |a| a.name.to_string())
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
            let status = self.terms[&id].status();
            let prev = self.tg_status.insert(id, status);
            if !active || prev == Some(status) || prev.is_none() {
                continue;
            }
            let name = self.pane_label(id);
            match status {
                Status::Idle if prev == Some(Status::Working) => telegram::send(format!("✅ {name} is done.\n\n{}", self.terms[&id].screen_text(15))),
                Status::Question => telegram::send(format!("❓ {name} needs your input:\n\n{}", self.terms[&id].screen_text(20))),
                Status::Error => telegram::send(format!("❌ {name} reported an error:\n\n{}", self.terms[&id].screen_text(15))),
                _ => {}
            }
        }
        self.tg_status.retain(|id, _| self.terms.contains_key(id));
    }

    // ---------------------------------------------------------------- Sidebar

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let card = ui.max_rect();
        ui.painter().rect(card, RADIUS, Color32::BLACK, Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);
        let inner = card.shrink(10.0);

        // Footer mit Settings
        let footer = Rect::from_min_max(pos2(inner.left(), inner.bottom() - 34.0), inner.max);
        ui.painter().line_segment(
            [pos2(card.left() + 1.0, footer.top() - 9.0), pos2(card.right() - 1.0, footer.top() - 9.0)],
            Stroke::new(1.0, BORDER),
        );
        let resp = ui.interact(footer, ui.id().with("settings"), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(footer, 8.0, HOVER);
        }
        let col = if resp.hovered() { TEXT } else { MUTED };
        icons::draw(ui.painter(), footer.left_center() + vec2(16.0, 0.0), Icon::Gear, col);
        ui.painter().text(footer.left_center() + vec2(34.0, 0.0), egui::Align2::LEFT_CENTER, "Settings", medium(13.0), col);
        if resp.clicked() {
            self.settings = Some(settings::Settings::new(ui.ctx()));
        }

        let list = Rect::from_min_max(pos2(inner.left(), inner.top() + 4.0), pos2(inner.right(), footer.top() - 18.0));
        ui.scope_builder(egui::UiBuilder::new().max_rect(list), |ui| self.project_list(ui));
    }

    fn project_list(&mut self, ui: &mut egui::Ui) {
        let (btn, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::click());
        ui.painter().rect(
            btn,
            10.0,
            if resp.hovered() { SELECTED } else { ELEVATED },
            Stroke::new(1.0, if resp.hovered() { BORDER_STRONG.gamma_multiply(1.6) } else { BORDER_STRONG }),
            egui::StrokeKind::Inside,
        );
        let label = ui.painter().layout_no_wrap("Add project".into(), medium(13.5), TEXT);
        let w = 12.0 + 8.0 + label.size().x;
        let x0 = btn.center().x - w / 2.0;
        icons::draw(ui.painter(), pos2(x0 + 6.0, btn.center().y), Icon::Plus, TEXT);
        ui.painter().galley(pos2(x0 + 20.0, btn.center().y - label.size().y / 2.0), label, TEXT);
        if resp.clicked() {
            self.modal = Some(Modal::Add(AddDialog::new()));
        }
        ui.add_space(14.0);
        let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
        ui.painter().text(head.left_center() + vec2(8.0, 0.0), egui::Align2::LEFT_CENTER, "PROJECTS", semibold(10.5), FAINT);
        ui.add_space(4.0);

        enum Cmd {
            Select(usize, usize),
            Toggle(usize),
            NewSession(usize),
            CloseSession(usize, usize),
            Rename(Rename, String),
            Remove(usize),
        }
        let mut cmds = Vec::new();

        if self.store.projects.is_empty() {
            ui.add_space(8.0);
            ui.label(RichText::new("No projects yet").color(MUTED));
        }

        let now = ui.input(|i| i.time);
        let mut animate = false;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for (pi, p) in self.store.projects.iter().enumerate() {
                let active_p = pi == self.store.project;
                let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
                let plus_r = Rect::from_center_size(rect.right_center() - vec2(14.0, 0.0), vec2(22.0, 22.0));
                let dots_r = plus_r.translate(vec2(-24.0, 0.0));
                let plus = ui.interact(plus_r, ui.id().with(("newsession", pi)), Sense::click());
                let dots = ui.interact(dots_r, ui.id().with(("dots", pi)), Sense::click());
                let hovered = resp.hovered() || plus.hovered() || dots.hovered();
                let menu_open = egui::Popup::is_id_open(ui.ctx(), egui::Popup::default_response_id(&dots));

                if hovered || menu_open {
                    ui.painter().rect_filled(rect, 8.0, HOVER);
                }
                let ir = Rect::from_center_size(rect.left_center() + vec2(15.0, 0.0), vec2(16.0, 16.0));
                if p.location.is_remote() {
                    draw_small(ui.painter(), ir.center(), Icon::Server, if active_p { MUTED } else { FAINT });
                } else {
                    egui::Image::from_bytes("bytes://folder.svg", FOLDER_SVG)
                        .tint(if active_p { TEXT } else { Color32::from_white_alpha(170) })
                        .paint_at(ui, ir);
                }
                let name_clip = Rect::from_min_max(rect.min, pos2(dots_r.left() - 4.0, rect.bottom()));
                ui.painter().with_clip_rect(name_clip).text(
                    rect.left_center() + vec2(32.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    &p.name,
                    medium(13.5),
                    if active_p { TEXT } else { MUTED },
                );

                for (r, resp, icon) in [(plus_r, &plus, Icon::Plus), (dots_r, &dots, Icon::Dots)] {
                    if resp.hovered() {
                        ui.painter().rect_filled(r, 6.0, SELECTED);
                    }
                    let visible = matches!(icon, Icon::Plus) || hovered || menu_open;
                    if visible {
                        let col = if resp.hovered() { TEXT } else if hovered { MUTED } else { FAINT };
                        icons::draw(ui.painter(), r.center(), icon, col);
                    }
                }
                let plus = plus.on_hover_text("New session");
                if plus.clicked() {
                    cmds.push(Cmd::NewSession(pi));
                } else if resp.clicked() {
                    cmds.push(Cmd::Toggle(pi));
                }

                let mut menu = |ui: &mut egui::Ui| {
                    ui.set_min_width(170.0);
                    if menu_item(ui, "New session") {
                        cmds.push(Cmd::NewSession(pi));
                    }
                    if menu_item(ui, "Rename…") {
                        cmds.push(Cmd::Rename(Rename::Project(pi), p.name.clone()));
                    }
                    ui.separator();
                    if menu_item_danger(ui, "Remove project") {
                        cmds.push(Cmd::Remove(pi));
                    }
                };
                egui::Popup::menu(&dots).show(&mut menu);
                resp.context_menu(&mut menu);
                resp.on_hover_text(p.location.display());

                if p.expanded {
                    for (si, s) in p.sessions.iter().enumerate() {
                        let panes = s.grid.panes();
                        let h = if panes.is_empty() { 30.0 } else { 54.0 };
                        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::click());
                        let r = Rect::from_min_max(r.min + vec2(14.0, 0.0), r.max);
                        let active = active_p && si == self.store.session;
                        let fill = if active { SELECTED } else if resp.hovered() { HOVER } else { Color32::TRANSPARENT };
                        ui.painter().rect_filled(r, 8.0, fill);
                        if active {
                            let bar = Rect::from_min_max(pos2(r.left(), r.top() + 8.0), pos2(r.left() + 2.5, r.bottom() - 8.0));
                            ui.painter().rect_filled(bar, 2.0, ACCENT);
                        }
                        let line = r.top() + 15.0;
                        let alive = panes.iter().any(|id| self.terms.get(id).is_some_and(|t| t.is_alive()));
                        ui.painter().circle_filled(pos2(r.left() + 14.0, line), 3.0, if alive { GREEN } else { FAINT });
                        ui.painter().with_clip_rect(Rect::from_min_max(r.min, pos2(r.right() - 30.0, r.bottom()))).text(
                            pos2(r.left() + 26.0, line),
                            egui::Align2::LEFT_CENTER,
                            &s.name,
                            medium(13.0),
                            if active { TEXT } else { MUTED },
                        );
                        if !panes.is_empty() {
                            let badge = Rect::from_center_size(pos2(r.right() - 16.0, line), vec2(20.0, 17.0));
                            ui.painter().rect_filled(badge, 6.0, if active { BORDER_STRONG } else { ELEVATED });
                            ui.painter().text(badge.center(), egui::Align2::CENTER_CENTER, panes.len().to_string(), medium(10.5), MUTED);
                            // Ein Icon pro Terminal, mit Status-Ring
                            let mut x = r.left() + 26.0 + 9.0;
                            let y = r.top() + 39.0;
                            for id in &panes {
                                let c = pos2(x, y);
                                let status = self.terms.get(id).map_or(Status::Idle, |t| t.status());
                                status_ring(ui.painter(), c, status, now);
                                if status != Status::Idle {
                                    animate = true;
                                }
                                let stored = s.agents.get(id).and_then(|a| agents::get(a));
                                // Was gerade wirklich läuft, hat Vorrang vor dem Agent, mit dem die Pane gestartet wurde.
                                let agent = match self.terms.get(id) {
                                    Some(t) => {
                                        let fg = t.foreground();
                                        agents::by_process(&fg).or(if fg == t.shell_name { None } else { stored })
                                    }
                                    None => stored,
                                }
                                .unwrap_or(&AGENTS[0]);
                                agent.paint_logo(ui, Rect::from_center_size(c, vec2(14.0, 14.0)), false);
                                x += 26.0;
                            }
                        }
                        if resp.clicked() {
                            cmds.push(Cmd::Select(pi, si));
                        }
                        resp.context_menu(|ui| {
                            ui.set_min_width(160.0);
                            if menu_item(ui, "Rename…") {
                                cmds.push(Cmd::Rename(Rename::Session(pi, si), s.name.clone()));
                            }
                            ui.separator();
                            if menu_item_danger(ui, "Close session") {
                                cmds.push(Cmd::CloseSession(pi, si));
                            }
                        });
                    }
                }
                ui.add_space(6.0);
            }
        });

        if animate {
            ui.ctx().request_repaint_after(Duration::from_millis(60));
        }
        for c in cmds {
            match c {
                Cmd::Select(p, s) => self.select(p, s),
                Cmd::Toggle(p) => {
                    if p == self.store.project {
                        self.store.projects[p].expanded ^= true;
                    } else {
                        self.store.projects[p].expanded = true;
                        self.select(p, 0);
                    }
                }
                Cmd::NewSession(p) => self.new_session(p),
                Cmd::CloseSession(p, s) => self.close_session(p, s),
                Cmd::Rename(t, n) => self.modal = Some(Modal::Rename(t, n)),
                Cmd::Remove(p) => self.modal = Some(Modal::RemoveProject(p)),
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
            return;
        }

        let ctx = ui.ctx().clone();
        let (cwd, argv) = self.project().unwrap().location.spawn_args();
        let (pi, si) = (self.store.project, self.store.session);
        let mut grid = std::mem::take(&mut self.store.projects[pi].sessions[si].grid);

        for id in grid.panes() {
            if !self.terms.contains_key(&id) {
                match Terminal::spawn(&cwd, argv.as_deref(), ctx.clone()) {
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
        const HEAD: f32 = 36.0;
        let focused = self.focused == Some(id);
        let painter = ui.painter().clone();
        painter.rect_filled(rect, RADIUS, PANE);
        self.pane_rects.push((id, rect));

        let head = Rect::from_min_size(rect.min, vec2(rect.width(), HEAD));
        painter.line_segment([head.left_bottom() + vec2(1.0, 0.0), head.right_bottom() - vec2(1.0, 0.0)], Stroke::new(1.0, BORDER));

        // Header-Fläche zuerst registrieren, damit die Buttons darüber liegen und Klicks bekommen.
        let head_resp = ui.interact(head, ui.id().with(("head", id)), Sense::click());

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
        if btn(ui, Icon::Close, "Close  Ctrl+W") {
            actions.push(Action::Close(id));
        }
        if !full && btn(ui, Icon::Plus, "New terminal  Ctrl+T") {
            actions.push(Action::New);
        }
        if grid.len() > 1 {
            let (icon, tip) = if maxed { (Icon::Restore, "Restore") } else { (Icon::Maximize, "Maximize  Ctrl+Shift+Enter") };
            if btn(ui, icon, tip) {
                actions.push(Action::ToggleMax(id));
            }
        }
        if !full && btn(ui, Icon::SplitDown, "Split down  Ctrl+Shift+E") {
            actions.push(Action::SplitDown(id));
        }
        if !full && btn(ui, Icon::SplitRight, "Split right  Ctrl+Shift+D") {
            actions.push(Action::SplitRight(id));
        }
        let buttons_left = x + 12.0;

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
        tp.circle_filled(pos2(tx + 3.0, head.center().y), 3.5, if term.is_alive() { GREEN } else { FAINT });
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
            if ui.add_enabled(!full, menu_button("Split right", "Ctrl+Shift+D")).clicked() {
                actions.push(Action::SplitRight(id));
            }
            if ui.add_enabled(!full, menu_button("Split down", "Ctrl+Shift+E")).clicked() {
                actions.push(Action::SplitDown(id));
            }
            if grid.len() > 1 && ui.add(menu_button(if maxed { "Restore" } else { "Maximize" }, "Ctrl+Shift+Enter")).clicked() {
                actions.push(Action::ToggleMax(id));
            }
            if menu_item(ui, "Rename…") {
                actions.push(Action::Rename(id));
            }
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

        let stroke = if drop_here { Stroke::new(1.0, ACCENT.gamma_multiply(0.75)) } else if focused { Stroke::new(1.0, BORDER_STRONG) } else { Stroke::new(1.0, BORDER) };
        painter.rect_stroke(rect, RADIUS, stroke, egui::StrokeKind::Inside);
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
                        ui.horizontal(|ui| {
                            keycap(ui, "Ctrl");
                            keycap(ui, "Shift");
                            keycap(ui, "T");
                            ui.label(RichText::new("new terminal").color(FAINT).font(font(12.0)));
                        });
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

/// Kreis um ein Terminal-Icon: grüner drehender Ladebogen = arbeitet,
/// gelb blinkend = Frage/Freigabe, rot blinkend = Fehler, nichts = untätig.
fn status_ring(p: &egui::Painter, c: egui::Pos2, status: Status, t: f64) {
    const R: f32 = 11.0;
    let blink = (0.5 + 0.5 * (t * 6.0).sin()) as f32;
    match status {
        Status::Idle => {}
        Status::Working => {
            p.circle_stroke(c, R, Stroke::new(2.0, GREEN.gamma_multiply(0.18)));
            let start = (t * 5.0) as f32;
            let pts: Vec<egui::Pos2> =
                (0..=24).map(|k| c + egui::Vec2::angled(start + k as f32 / 24.0 * 4.2) * R).collect();
            p.add(egui::Shape::line(pts, Stroke::new(2.0, GREEN)));
        }
        Status::Question | Status::Error => {
            let col = if status == Status::Error { RED } else { Color32::from_rgb(0xfa, 0xcc, 0x15) };
            let a = 0.2 + 0.8 * blink;
            p.circle_filled(c, R, col.gamma_multiply(0.15 * a));
            p.circle_stroke(c, R, Stroke::new(2.0, col.gamma_multiply(a)));
        }
    }
}

const FOLDER_SVG: &[u8] = include_bytes!("../assets/logos/folder.svg");

fn draw_small(p: &egui::Painter, c: egui::Pos2, icon: Icon, col: Color32) {
    icons::draw(p, c, icon, col);
}

fn menu_item(ui: &mut egui::Ui, text: &str) -> bool {
    ui.add(egui::Button::new(RichText::new(text).font(font(13.0))).frame(false).min_size(vec2(ui.available_width(), 28.0)))
        .clicked()
}

fn menu_item_danger(ui: &mut egui::Ui, text: &str) -> bool {
    ui.add(egui::Button::new(RichText::new(text).font(font(13.0)).color(RED)).frame(false).min_size(vec2(ui.available_width(), 28.0)))
        .clicked()
}

fn menu_button<'a>(text: &'a str, shortcut: &'a str) -> egui::Button<'a> {
    egui::Button::new(RichText::new(text).font(font(13.0)))
        .shortcut_text(RichText::new(shortcut).font(font(11.5)).color(FAINT))
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
        self.telegram();
        // Telegram-Nachrichten und Agent-Status auch ohne Eingaben weiter verarbeiten
        ctx.request_repaint_after(Duration::from_secs(1));

        if let Some(s) = &mut self.settings {
            if !settings::show(ui, s) {
                self.settings = None;
            }
            return;
        }

        let input = self.modal.is_none();
        if input {
            self.shortcuts(&ctx);
        }

        egui::Panel::left("sidebar")
            .resizable(true)
            .show_separator_line(false)
            .default_size(330.0)
            .size_range(280.0..=520.0)
            .frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin { left: 10, right: 0, top: 10, bottom: 10 }))
            .show(ui, |ui| self.sidebar(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(10))
            .show(ui, |ui| self.workspace(ui, input));

        self.modals(&ctx);
    }

    fn on_exit(&mut self) {
        App::save(self);
    }
}
