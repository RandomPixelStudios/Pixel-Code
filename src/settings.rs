use eframe::egui::{self, pos2, vec2, Color32, Rect, Sense, Stroke};

use crate::icons::{self, Icon};
use crate::keybinds::{Binding, Cmd, Keybinds};
use crate::plugins::{self, GhState, GitHub};
use crate::theme::*;
use crate::update::{self, Updater};

const GITHUB_SVG: &[u8] = include_bytes!("../assets/logos/github.svg");

#[derive(Clone, Copy, PartialEq)]
pub enum Page {
    Plugins,
    Keybinds,
    Update,
}

#[derive(Clone, Copy, PartialEq)]
enum Sort {
    Name,
    Category,
}

pub struct Settings {
    pub page: Page,
    github: GitHub,
    registry: plugins::Registry,
    loaded: Option<std::time::Instant>,
    /// Bearbeitete Felder (plugin, key) -> Text
    edits: std::collections::HashMap<(&'static str, &'static str), String>,
    checking: std::collections::HashSet<&'static str>,
    /// Plugin, dessen Connect-Formular offen ist
    form: Option<&'static str>,
    search: String,
    /// Nur Plugins dieser Kategorie zeigen
    category: Option<&'static str>,
    sort: Sort,
    /// Plugin, dessen Details offen sind ("github" oder eine Def-id)
    detail: Option<&'static str>,
    errors: std::collections::HashMap<&'static str, String>,
    done_rx: std::sync::mpsc::Receiver<(&'static str, bool)>,
    done_tx: std::sync::mpsc::Sender<(&'static str, bool)>,
    pub keybinds: Keybinds,
    /// Aktion, deren neues Kürzel gerade aufgenommen wird
    recording: Option<Cmd>,
    updater: Updater,
}

impl Settings {
    pub fn new(ctx: &egui::Context, page: Page, keybinds: Keybinds, updater: Updater) -> Self {
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        plugins::refresh_all(ctx.clone());
        Self {
            page,
            github: GitHub::new(ctx.clone()),
            registry: plugins::load_registry(),
            loaded: None,
            edits: Default::default(),
            checking: Default::default(),
            form: None,
            search: String::new(),
            category: None,
            sort: Sort::Name,
            detail: None,
            errors: Default::default(),
            done_rx,
            done_tx,
            keybinds,
            recording: None,
            updater,
        }
    }

    /// Verbindet im Hintergrund; das Ergebnis kommt über `done_rx`.
    fn connect(&mut self, ctx: &egui::Context, d: &'static plugins::Def) {
        self.checking.insert(d.id);
        self.errors.remove(d.id);
        let tx = self.done_tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let ok = plugins::connect(d);
            let _ = tx.send((d.id, ok));
            ctx.request_repaint();
        });
    }
}

/// Zeichnet die Einstellungsseite. Gibt `false` zurück, wenn sie geschlossen werden soll.
pub fn show(ui: &mut egui::Ui, s: &mut Settings) -> bool {
    // Esc schließt erst die Plugin-Details bzw. die Aufnahme, dann die Einstellungen
    let mut open = s.detail.is_some() || s.recording.is_some() || !ui.input(|i| i.key_pressed(egui::Key::Escape));

    egui::Panel::left("settings_nav")
        .resizable(false)
        .exact_size(270.0)
        .frame(egui::Frame::new().fill(BG).inner_margin(egui::Margin { left: 10, right: 0, top: 10, bottom: 10 }))
        .show(ui, |ui| {
            let card = ui.max_rect();
            ui.painter().rect(card, RADIUS, PANE, Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);
            // Kopfzeile wie in der Sidebar
            let head = Rect::from_min_size(card.min, vec2(card.width(), HEAD));
            ui.painter().line_segment([head.left_bottom() + vec2(1.0, 0.0), head.right_bottom() - vec2(1.0, 0.0)], Stroke::new(1.0, BORDER));
            let back = Rect::from_min_max(head.min + vec2(6.0, 6.0), head.max - vec2(6.0, 6.0));
            let resp = ui.interact(back, ui.id().with("back"), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(back, 6.0, HOVER);
            }
            icons::draw(ui.painter(), back.left_center() + vec2(14.0, 0.0), Icon::Back, if resp.hovered() { TEXT } else { MUTED });
            ui.painter().text(back.left_center() + vec2(32.0, 0.0), egui::Align2::LEFT_CENTER, "Settings", semibold(13.0), TEXT);
            ui.painter().text(back.right_center() - vec2(8.0, 0.0), egui::Align2::RIGHT_CENTER, "Esc", medium(11.0), FAINT);
            if resp.clicked() {
                open = false;
            }
            ui.scope_builder(egui::UiBuilder::new().max_rect(Rect::from_min_max(pos2(card.left() + 8.0, head.bottom() + 8.0), card.max - vec2(8.0, 8.0))), |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                let update = s.updater.available().is_some();
                for (page, icon, label) in [(Page::Plugins, Icon::Plug, "Plugins"), (Page::Keybinds, Icon::Keyboard, "Keybinds"), (Page::Update, Icon::Download, "Update")] {
                    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
                    let active = s.page == page;
                    if active {
                        ui.painter().rect_filled(r, 8.0, SELECTED);
                    } else if resp.hovered() {
                        ui.painter().rect_filled(r, 8.0, HOVER);
                    }
                    let col = if active || resp.hovered() { TEXT } else { MUTED };
                    icons::draw(ui.painter(), r.left_center() + vec2(16.0, 0.0), icon, col);
                    ui.painter().text(r.left_center() + vec2(34.0, 0.0), egui::Align2::LEFT_CENTER, label, medium(13.0), col);
                    if page == Page::Update && update {
                        ui.painter().circle_filled(r.right_center() - vec2(14.0, 0.0), 3.5, GREEN);
                    }
                    if resp.clicked() {
                        s.page = page;
                        s.recording = None;
                    }
                }
            });
        });

    egui::CentralPanel::default().frame(egui::Frame::new().fill(BG)).show(ui, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let width = (ui.available_width() - 80.0).min(780.0);
            let margin = ((ui.available_width() - width) / 2.0).max(0.0);
            ui.horizontal(|ui| {
                ui.add_space(margin);
                ui.vertical(|ui| {
                    ui.set_width(width);
                    ui.add_space(48.0);
                    match s.page {
                        Page::Plugins => plugins_page(ui, s),
                        Page::Keybinds => keybinds_page(ui, s),
                        Page::Update => update_page(ui, s),
                    }
                    ui.add_space(40.0);
                });
            });
        });
    });

    if let Some(id) = s.detail {
        let modal = egui::Modal::new(egui::Id::new("plugin_detail"))
            .frame(egui::Frame::new().fill(BG).stroke(Stroke::new(1.0, BORDER_STRONG)).corner_radius(16).inner_margin(6))
            .show(ui.ctx(), |ui| {
                ui.set_width(560.0);
                let max_h = ui.ctx().content_rect().height() - 120.0;
                egui::ScrollArea::vertical().max_height(max_h).auto_shrink([false, true]).show(ui, |ui| {
                    if id == "github" {
                        github_card(ui, s);
                    } else if let Some(d) = plugins::DEFS.iter().find(|d| d.id == id) {
                        plugin_card(ui, s, d);
                        ui.add_space(8.0);
                        egui::Frame::new().inner_margin(egui::Margin::symmetric(20, 4)).show(ui, |ui| {
                            ui.label(egui::RichText::new("COMMANDS").font(semibold(10.5)).color(FAINT).extra_letter_spacing(1.2));
                            ui.label(egui::RichText::new(d.usage).font(mono(11.5)).color(MUTED));
                        });
                    }
                });
                ui.add_space(8.0);
                // Close rechts, ohne die volle Höhe zu belegen
                let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::hover());
                let btn = Rect::from_min_size(pos2(row.right() - 90.0, row.top()), vec2(76.0, 32.0));
                if ui.put(btn, plain_button("Close")).clicked() {
                    ui.close();
                }
                ui.add_space(8.0);
            });
        if modal.should_close() {
            s.detail = None;
            s.form = None;
        }
    }
    open
}

fn page_title(ui: &mut egui::Ui, title: &str, sub: &str) {
    ui.label(egui::RichText::new(title).font(semibold(26.0)));
    ui.add_space(2.0);
    ui.label(egui::RichText::new(sub).font(font(14.0)).color(MUTED));
    ui.add_space(28.0);
}

fn plugins_page(ui: &mut egui::Ui, s: &mut Settings) {
    page_title(ui, "Plugins", "Connect services so your agents can use them. Every supported CLI gets access through the Pixel Code plugin.");
    if s.loaded.is_none_or(|t| t.elapsed().as_secs_f32() > 1.0) {
        s.registry = plugins::load_registry();
        s.loaded = Some(std::time::Instant::now());
    }
    while let Ok((id, ok)) = s.done_rx.try_recv() {
        s.checking.remove(id);
        s.registry = plugins::load_registry();
        if ok {
            if s.form == Some(id) {
                s.form = None;
            }
        } else {
            let status = s.registry.plugins.iter().find(|p| p.id == id).map(|p| p.status.clone());
            s.errors.insert(id, status.unwrap_or_else(|| "Could not connect".into()));
            s.form = Some(id);
        }
    }
    ui.horizontal(|ui| {
        let sort_w = 190.0;
        ui.add(
            egui::TextEdit::singleline(&mut s.search)
                .hint_text("Search plugins…")
                .margin(vec2(12.0, 9.0))
                .desired_width(ui.available_width() - sort_w - 8.0),
        );
        // Sortierung: A-Z oder nach Kategorie gruppiert
        ui.spacing_mut().item_spacing.x = 0.0;
        for (i, (sort, label)) in [(Sort::Name, "A–Z"), (Sort::Category, "By category")].into_iter().enumerate() {
            let w = if i == 0 { 70.0 } else { 120.0 };
            let (r, resp) = ui.allocate_exact_size(vec2(w, 34.0), Sense::click());
            let on = s.sort == sort;
            let cr = if i == 0 { egui::CornerRadius { nw: 8, sw: 8, ne: 0, se: 0 } } else { egui::CornerRadius { nw: 0, sw: 0, ne: 8, se: 8 } };
            ui.painter().rect(r, cr, if on { SELECTED } else if resp.hovered() { HOVER } else { INPUT }, Stroke::new(1.0, if on { BORDER_STRONG } else { BORDER }), egui::StrokeKind::Inside);
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, label, medium(12.5), if on { TEXT } else { MUTED });
            if resp.clicked() {
                s.sort = sort;
            }
        }
    });
    ui.add_space(12.0);
    // Kategorie-Filter
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        let all = std::iter::once(None).chain(plugins::CATEGORIES.iter().map(|c| Some(*c)));
        for c in all {
            let on = s.category == c;
            let text = c.unwrap_or("All");
            let galley = ui.painter().layout_no_wrap(text.to_string(), medium(12.0), TEXT);
            let (r, resp) = ui.allocate_exact_size(vec2(galley.size().x + 24.0, 28.0), Sense::click());
            let fill = if on { ACCENT } else if resp.hovered() { HOVER } else { SURFACE };
            ui.painter().rect(r, 14.0, fill, Stroke::new(1.0, if on { ACCENT } else { BORDER }), egui::StrokeKind::Inside);
            ui.painter().text(r.center(), egui::Align2::CENTER_CENTER, text, medium(12.0), if on { BG } else if resp.hovered() { TEXT } else { MUTED });
            if resp.clicked() {
                s.category = if on { None } else { c };
            }
        }
    });
    ui.add_space(24.0);

    let q = s.search.trim().to_lowercase();
    let gh_on = matches!(s.github.get(), GhState::Connected(_));
    let mut tiles: Vec<Tile> = Vec::new();
    if q.is_empty() || "github".contains(&q) {
        tiles.push(Tile {
            id: "github",
            name: "GitHub",
            logo: ("bytes://github.svg", GITHUB_SVG),
            connected: gh_on,
            category: plugins::category("github"),
            status: match s.github.get() {
                GhState::Connected(l) => format!("Connected as {l}"),
                _ => "Repos, PRs, issues".into(),
            },
        });
    }
    for d in plugins::DEFS.iter().filter(|d| q.is_empty() || d.name.to_lowercase().contains(&q) || d.description.to_lowercase().contains(&q)) {
        let e = s.registry.plugins.iter().find(|p| p.id == d.id);
        let on = e.is_some_and(|e| e.enabled && e.connected);
        tiles.push(Tile {
            id: d.id,
            name: d.name,
            logo: d.logo,
            connected: on,
            category: plugins::category(d.id),
            status: if on { e.map(|e| e.status.clone()).unwrap_or_default() } else { short(d.description) },
        });
    }
    if let Some(c) = s.category {
        tiles.retain(|t| t.category == c);
    }
    tiles.sort_by_key(|t| t.name.to_lowercase());
    if tiles.is_empty() {
        ui.label(egui::RichText::new("No plugins match.").color(MUTED));
        return;
    }

    let mut clicked = None;
    let (on, off): (Vec<Tile>, Vec<Tile>) = tiles.into_iter().partition(|t| t.connected);
    if !on.is_empty() {
        section(ui, "CONNECTED", on.len());
        clicked = clicked.or(tile_grid(ui, &on, false));
        ui.add_space(18.0);
    }
    match s.sort {
        Sort::Name => {
            section(ui, "AVAILABLE", off.len());
            clicked = clicked.or(tile_grid(ui, &off, false));
        }
        Sort::Category => {
            for c in plugins::CATEGORIES {
                let group: Vec<Tile> = off.iter().filter(|t| t.category == *c).cloned().collect();
                if group.is_empty() {
                    continue;
                }
                section(ui, &c.to_uppercase(), group.len());
                clicked = clicked.or(tile_grid(ui, &group, false));
                ui.add_space(18.0);
            }
        }
    }
    if let Some(id) = clicked {
        s.detail = Some(id);
        // Formular gleich öffnen, wenn das Plugin Eingaben braucht und noch nicht verbunden ist
        let connected = s.registry.plugins.iter().any(|p| p.id == id && p.enabled && p.connected);
        if !connected && plugins::DEFS.iter().any(|d| d.id == id && !d.fields.is_empty()) {
            s.form = Some(id);
        }
    }
}

fn keybinds_page(ui: &mut egui::Ui, s: &mut Settings) {
    page_title(ui, "Keybinds", "Click a shortcut and press the new key combination. Esc cancels, Backspace removes it.");

    // Aufnahme eines neuen Kürzels
    if let Some(cmd) = s.recording {
        let events = ui.input(|i| i.events.clone());
        for ev in events {
            if let egui::Event::Key { key, pressed: true, modifiers, .. } = ev {
                match key {
                    egui::Key::Escape => s.recording = None,
                    egui::Key::Backspace | egui::Key::Delete if !modifiers.any() => {
                        s.keybinds.map.insert(cmd, None);
                        s.keybinds.save();
                        s.recording = None;
                    }
                    _ => {
                        let mods = egui::Modifiers { ctrl: modifiers.ctrl || modifiers.command, alt: modifiers.alt, shift: modifiers.shift, mac_cmd: false, command: false };
                        let b = Binding { mods, key };
                        // Ohne Ctrl/Alt würde das Kürzel normale Eingaben im Terminal abfangen
                        if !mods.ctrl && !mods.alt && !matches!(key, egui::Key::F1 | egui::Key::F2 | egui::Key::F3 | egui::Key::F4 | egui::Key::F5 | egui::Key::F6 | egui::Key::F7 | egui::Key::F8 | egui::Key::F9 | egui::Key::F10 | egui::Key::F11 | egui::Key::F12) {
                            continue;
                        }
                        if let Some(other) = s.keybinds.conflict(cmd, b) {
                            s.keybinds.map.insert(other, None);
                        }
                        s.keybinds.map.insert(cmd, Some(b));
                        s.keybinds.save();
                        s.recording = None;
                    }
                }
                break;
            }
        }
        ui.ctx().input_mut(|i| i.events.retain(|e| !matches!(e, egui::Event::Key { .. } | egui::Event::Text(_))));
    }

    let mut group = "";
    for cmd in Cmd::ALL {
        if cmd.group() != group {
            if !group.is_empty() {
                ui.add_space(18.0);
            }
            group = cmd.group();
            section(ui, &group.to_uppercase(), Cmd::ALL.iter().filter(|c| c.group() == group).count());
        }
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::hover());
        ui.painter().rect(r, 10.0, SURFACE, Stroke::new(1.0, BORDER), egui::StrokeKind::Inside);
        ui.painter().text(r.left_center() + vec2(16.0, 0.0), egui::Align2::LEFT_CENTER, cmd.label(), medium(13.5), TEXT);

        let recording = s.recording == Some(*cmd);
        let text = if recording { "Press keys…".to_string() } else { s.keybinds.get(*cmd).map_or("Not set".into(), |b| b.text()) };
        let galley = ui.painter().layout_no_wrap(text.clone(), mono(12.0), TEXT);
        let chip = Rect::from_min_max(pos2(r.right() - 16.0 - galley.size().x - 24.0, r.center().y - 14.0), pos2(r.right() - 16.0, r.center().y + 14.0));
        let resp = ui.interact(chip, ui.id().with(("bind", *cmd)), Sense::click());
        let fill = if recording { SELECTED } else if resp.hovered() { HOVER } else { ELEVATED };
        let stroke = if recording { Stroke::new(1.0, ACCENT) } else { Stroke::new(1.0, BORDER_STRONG) };
        ui.painter().rect(chip, 8.0, fill, stroke, egui::StrokeKind::Inside);
        let col = if s.keybinds.get(*cmd).is_none() && !recording { FAINT } else { TEXT };
        ui.painter().text(chip.center(), egui::Align2::CENTER_CENTER, text, mono(12.0), col);
        if resp.on_hover_text("Click to change").clicked() {
            s.recording = if recording { None } else { Some(*cmd) };
        }
        if !s.keybinds.is_default(*cmd) {
            let reset = Rect::from_center_size(pos2(chip.left() - 20.0, r.center().y), vec2(26.0, 26.0));
            let rr = ui.interact(reset, ui.id().with(("reset", *cmd)), Sense::click());
            if rr.hovered() {
                ui.painter().rect_filled(reset, 6.0, HOVER);
            }
            icons::draw(ui.painter(), reset.center(), Icon::Reset, if rr.hovered() { TEXT } else { MUTED });
            if rr.on_hover_text("Reset to default").clicked() {
                s.keybinds.reset(*cmd);
                s.keybinds.save();
            }
        }
        ui.add_space(2.0);
    }
    ui.add_space(20.0);
    if ui.add(plain_button("Reset all to defaults")).clicked() {
        s.keybinds = Keybinds::default();
        s.keybinds.save();
        s.recording = None;
    }
}

fn update_page(ui: &mut egui::Ui, s: &mut Settings) {
    page_title(ui, "Update", "Pixel Code checks GitHub for new releases and installs them for you.");
    let state = s.updater.get();
    card(ui, |ui| {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
            ui.painter().rect(r, 12.0, ELEVATED, Stroke::new(1.0, BORDER_STRONG), egui::StrokeKind::Inside);
            icons::draw(ui.painter(), r.center(), Icon::Download, TEXT);
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.add_space(3.0);
                ui.label(egui::RichText::new(format!("Pixel Code {}", update::VERSION)).font(semibold(15.0)));
                let (dot, line) = match &state {
                    update::State::Idle => (FAINT, "Not checked yet".to_string()),
                    update::State::Checking => (FAINT, "Checking for updates…".to_string()),
                    update::State::UpToDate => (GREEN, "You are on the latest version".to_string()),
                    update::State::Available(r) => (AMBER, format!("Version {} is available", r.version)),
                    update::State::Downloading(r) => (AMBER, format!("Downloading {}…", r.version)),
                    update::State::Installing(r) => (AMBER, format!("Installing {}…", r.version)),
                    update::State::Installed(v) => (GREEN, format!("Version {v} installed - restart to use it")),
                    update::State::Error(e) => (RED, e.clone()),
                };
                ui.horizontal(|ui| {
                    let (d, _) = ui.allocate_exact_size(vec2(8.0, 14.0), Sense::hover());
                    ui.painter().circle_filled(d.center(), 3.5, dot);
                    ui.add(egui::Label::new(egui::RichText::new(line).color(MUTED)).truncate());
                });
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| match &state {
                update::State::Checking | update::State::Downloading(_) | update::State::Installing(_) => {
                    ui.spinner();
                }
                update::State::Available(r) => {
                    let label = if r.deb.is_some() { "Install update" } else { "Open release" };
                    if ui.add(primary_button(label)).clicked() {
                        s.updater.install(ui.ctx().clone(), r.clone());
                    }
                }
                update::State::Installed(_) => {
                    if ui.add(primary_button("Restart now")).clicked() {
                        update::restart(ui.ctx());
                    }
                }
                _ => {
                    if ui.add(primary_button("Check for updates")).clicked() {
                        s.updater.check(ui.ctx().clone());
                    }
                }
            });
        });
        if let update::State::Available(r) = &state {
            if !r.notes.is_empty() {
                ui.add_space(14.0);
                ui.label(egui::RichText::new("WHAT'S NEW").font(semibold(10.5)).color(FAINT).extra_letter_spacing(1.2));
                ui.add_space(4.0);
                ui.label(egui::RichText::new(&r.notes).color(MUTED));
            }
        }
    });
    ui.add_space(14.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Releases:").color(FAINT));
        let url = format!("https://github.com/{}/releases", update::REPO);
        if ui.link(egui::RichText::new(format!("github.com/{}", update::REPO)).color(MUTED)).clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
        }
    });
    if s.updater.busy() {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
    }
}

#[derive(Clone)]
struct Tile {
    id: &'static str,
    name: &'static str,
    logo: (&'static str, &'static [u8]),
    connected: bool,
    category: &'static str,
    status: String,
}

/// Erster Satz einer Beschreibung, gekürzt für die Kachel.
fn short(desc: &str) -> String {
    let first = desc.split(['.', ':']).next().unwrap_or(desc).trim();
    if first.chars().count() > 34 { format!("{}…", first.chars().take(33).collect::<String>()) } else { first.to_string() }
}

fn section(ui: &mut egui::Ui, text: &str, count: usize) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(text).font(semibold(11.0)).color(FAINT).extra_letter_spacing(1.2));
        ui.label(egui::RichText::new(count.to_string()).font(medium(11.0)).color(FAINT));
    });
    ui.add_space(6.0);
}

/// Kachel-Raster wie beim Agent-Start; gibt die angeklickte Plugin-id zurück.
fn tile_grid(ui: &mut egui::Ui, list: &[Tile], _busy: bool) -> Option<&'static str> {
    const GAP: f32 = 12.0;
    let width = ui.available_width();
    let cols = ((width + GAP) / (190.0 + GAP)).floor().clamp(2.0, 4.0) as usize;
    let w = (width - GAP * (cols as f32 - 1.0)) / cols as f32;
    let mut chosen = None;
    for row in list.chunks(cols) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = GAP;
            for t in row {
                let (r, resp) = ui.allocate_exact_size(vec2(w, 104.0), Sense::click());
                let hov = resp.hovered();
                let border = if hov { BORDER_STRONG.gamma_multiply(1.6) } else { BORDER };
                ui.painter().rect(r, 14.0, if hov { HOVER } else { SURFACE }, Stroke::new(1.0, border), egui::StrokeKind::Inside);
                let logo = Rect::from_min_size(r.min + vec2(16.0, 16.0), vec2(30.0, 30.0));
                let tint = if t.connected || hov { Color32::WHITE } else { Color32::from_white_alpha(150) };
                egui::Image::from_bytes(t.logo.0, t.logo.1).tint(tint).paint_at(ui, logo);
                ui.painter().text(r.left_bottom() + vec2(16.0, -36.0), egui::Align2::LEFT_CENTER, t.name, medium(14.0), if t.connected { TEXT } else { MUTED });
                let sub = Rect::from_min_max(r.left_bottom() + vec2(16.0, -26.0), r.right_bottom() - vec2(12.0, 8.0));
                ui.painter().with_clip_rect(sub).text(sub.left_center(), egui::Align2::LEFT_CENTER, &t.status, font(11.5), FAINT);
                if t.connected {
                    ui.painter().circle_filled(r.right_top() + vec2(-22.0, 22.0), 4.0, GREEN);
                } else if hov {
                    let pill = Rect::from_min_size(pos2(r.right() - 80.0, r.top() + 16.0), vec2(66.0, 24.0));
                    ui.painter().rect(pill, 12.0, ACCENT, Stroke::NONE, egui::StrokeKind::Inside);
                    ui.painter().text(pill.center(), egui::Align2::CENTER_CENTER, "Connect", medium(11.5), BG);
                }
                if resp.clicked() {
                    chosen = Some(t.id);
                }
            }
        });
        ui.add_space(GAP - ui.spacing().item_spacing.y);
    }
    chosen
}

fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(RADIUS)
        .inner_margin(20)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
}

fn github_card(ui: &mut egui::Ui, s: &mut Settings) {
    let state = s.github.get();
    card(ui, |ui| {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
            ui.painter().rect(r, 12.0, ELEVATED, Stroke::new(1.0, BORDER_STRONG), egui::StrokeKind::Inside);
            egui::Image::from_bytes("bytes://github.svg", GITHUB_SVG).paint_at(ui, r.shrink(10.0));
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.add_space(3.0);
                ui.label(egui::RichText::new("GitHub").font(semibold(15.0)));
                let (dot, line) = match &state {
                    GhState::Checking => (FAINT, "Checking…".to_string()),
                    GhState::Disconnected => (FAINT, "Not connected".to_string()),
                    GhState::Downloading => (FAINT, "Downloading GitHub CLI…".to_string()),
                    GhState::Code(_) => (AMBER, "Waiting for browser sign-in…".to_string()),
                    GhState::Connected(l) => (GREEN, format!("Connected as {l}")),
                    GhState::Error(e) => (RED, e.clone()),
                };
                ui.horizontal(|ui| {
                    let (d, _) = ui.allocate_exact_size(vec2(8.0, 14.0), Sense::hover());
                    ui.painter().circle_filled(d.center(), 3.5, dot);
                    ui.label(egui::RichText::new(line).color(MUTED));
                });
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| match &state {
                GhState::Connected(_) => {
                    if ui.add(plain_button("Disconnect")).clicked() {
                        plugins::set_enabled("github", false);
                        s.github.disconnect(ui.ctx().clone());
                    }
                }
                GhState::Disconnected | GhState::Error(_) => {
                    if ui.add(primary_button("Connect")).clicked() {
                        s.github.connect(ui.ctx().clone());
                    }
                }
                _ => {
                    ui.spinner();
                }
            });
        });
        ui.add_space(12.0);
        ui.label(
            egui::RichText::new(
                "Sign in with your browser. Agents can then clone, pull and push, and manage pull requests, issues and releases with your account.",
            )
            .color(MUTED),
        );
        if let GhState::Code(code) = &state {
            ui.add_space(14.0);
            egui::Frame::new().fill(ELEVATED).stroke(Stroke::new(1.0, BORDER_STRONG)).corner_radius(10).inner_margin(14).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("Enter this code on github.com/login/device").color(MUTED));
                        ui.label(egui::RichText::new(code).font(mono_bold(24.0)).color(TEXT));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(plain_button("Open browser")).clicked() {
                            ui.ctx().open_url(egui::OpenUrl::new_tab("https://github.com/login/device"));
                        }
                        if ui.add(plain_button("Copy code")).clicked() {
                            ui.ctx().copy_text(code.clone());
                        }
                    });
                });
            });
        }
    });
    if matches!(state, GhState::Checking | GhState::Downloading | GhState::Code(_)) {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(300));
    }
}

fn plugin_card(ui: &mut egui::Ui, s: &mut Settings, d: &'static plugins::Def) {
    let entry = s.registry.plugins.iter().find(|p| p.id == d.id).cloned();
    let busy = s.checking.contains(d.id);
    let connected = entry.as_ref().is_some_and(|e| e.enabled && e.connected);
    let form_open = s.form == Some(d.id) && !connected;
    card(ui, |ui| {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
            ui.painter().rect(r, 12.0, ELEVATED, Stroke::new(1.0, BORDER_STRONG), egui::StrokeKind::Inside);
            egui::Image::from_bytes(d.logo.0, d.logo.1).paint_at(ui, r.shrink(10.0));
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.add_space(3.0);
                ui.label(egui::RichText::new(d.name).font(semibold(15.0)));
                let (dot, line) = if busy {
                    (AMBER, "Connecting…".to_string())
                } else if connected {
                    (GREEN, entry.as_ref().map_or(String::new(), |e| e.status.clone()))
                } else {
                    (FAINT, "Not connected".to_string())
                };
                ui.horizontal(|ui| {
                    let (dr, _) = ui.allocate_exact_size(vec2(8.0, 14.0), Sense::hover());
                    ui.painter().circle_filled(dr.center(), 3.5, dot);
                    ui.add(egui::Label::new(egui::RichText::new(line).color(MUTED)).truncate());
                });
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if busy {
                    ui.spinner();
                } else if connected {
                    if ui.add(plain_button("Disconnect")).clicked() {
                        plugins::disconnect(d);
                        s.edits.retain(|(id, _), _| *id != d.id);
                        s.registry = plugins::load_registry();
                    }
                } else if !form_open {
                    if ui.add(primary_button("Connect")).clicked() {
                        if d.fields.is_empty() {
                            s.connect(ui.ctx(), d);
                        } else {
                            s.form = Some(d.id);
                        }
                    }
                }
            });
        });
        ui.add_space(10.0);
        ui.label(egui::RichText::new(d.description).color(MUTED));

        if d.id == "telegram" && connected {
            let user = entry.as_ref().and_then(|e| e.settings.get("paired_user").cloned());
            ui.add_space(8.0);
            ui.label(match user {
                Some(u) => egui::RichText::new(format!("Paired with {u} - send /help to your bot")).color(TEXT),
                None => egui::RichText::new("Now send /start to your bot on Telegram to pair it.").color(AMBER),
            });
        }

        if !form_open {
            return;
        }
        ui.add_space(14.0);
        egui::Frame::new().fill(ELEVATED).stroke(Stroke::new(1.0, BORDER_STRONG)).corner_radius(10).inner_margin(14).show(ui, |ui| {
            ui.set_width(ui.available_width());
            for f in d.fields {
                let saved = entry.as_ref().and_then(|e| e.settings.get(f.key).cloned()).unwrap_or_default();
                let text = s.edits.entry((d.id, f.key)).or_insert(saved);
                ui.label(egui::RichText::new(f.label).font(medium(12.0)).color(MUTED));
                ui.add(
                    egui::TextEdit::singleline(text)
                        .hint_text(f.hint)
                        .password(f.secret)
                        .margin(vec2(10.0, 7.0))
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(8.0);
            }
            if let Some(err) = s.errors.get(d.id) {
                ui.label(egui::RichText::new(err).color(RED));
                ui.add_space(8.0);
            }
            ui.horizontal(|ui| {
                if ui.add_enabled(!busy, primary_button("Connect")).clicked() {
                    for f in d.fields {
                        let v = s.edits.get(&(d.id, f.key)).cloned().unwrap_or_default();
                        plugins::set_setting(d.id, f.key, v.trim());
                    }
                    s.connect(ui.ctx(), d);
                }
                if ui.add(plain_button("Cancel")).clicked() {
                    s.form = None;
                    s.errors.remove(d.id);
                }
            });
        });
    });
}

fn primary_button(text: &str) -> egui::Button<'_> {
    egui::Button::new(egui::RichText::new(text).color(BG).font(medium(13.0))).fill(ACCENT).corner_radius(8).min_size(vec2(96.0, 32.0))
}

fn plain_button(text: &str) -> egui::Button<'_> {
    egui::Button::new(egui::RichText::new(text).font(medium(13.0)))
        .fill(ELEVATED)
        .stroke(Stroke::new(1.0, BORDER_STRONG))
        .corner_radius(8)
        .min_size(vec2(0.0, 32.0))
}
