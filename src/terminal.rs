use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use eframe::egui;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};

/// Eine laufende Shell in einem Pseudo-Terminal plus der zugehörige Bildschirmzustand.
pub struct Terminal {
    parser: Arc<Mutex<vt100::Parser>>,
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    alive: Arc<AtomicBool>,
    size: (u16, u16),
    scroll: usize,
    scroll_acc: f32,
    pub shell_name: String,
    #[allow(dead_code)]
    last_output: Arc<Mutex<Instant>>,
    last_input: Instant,
}

/// Grober Zustand eines Agents, abgeleitet aus Ausgabe und Bildschirminhalt.
#[derive(Clone, Copy, PartialEq)]
pub enum Status {
    Idle,
    Working,
    Question,
    Error,
}

impl Terminal {
    /// Startet die Login-Shell in `cwd`, oder `argv` (z.B. ssh), falls angegeben.
    pub fn spawn(cwd: &Path, argv: Option<&[String]>, ctx: egui::Context) -> anyhow_lite::Result<Self> {
        let size = (24u16, 80u16);
        let pty = native_pty_system();
        let pair = pty
            .openpty(PtySize { rows: size.0, cols: size.1, pixel_width: 0, pixel_height: 0 })
            .map_err(|e| e.to_string())?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".into());
        let mut cmd = match argv {
            Some(a) if !a.is_empty() => {
                let mut c = CommandBuilder::new(&a[0]);
                c.args(&a[1..]);
                c
            }
            _ => CommandBuilder::new(&shell),
        };
        cmd.cwd(if cwd.is_dir() { cwd.to_path_buf() } else { dirs::home_dir().unwrap_or_default() });
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
        drop(pair.slave);

        let parser = Arc::new(Mutex::new(vt100::Parser::new(size.0, size.1, 5000)));
        let alive = Arc::new(AtomicBool::new(true));
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

        let last_output = Arc::new(Mutex::new(Instant::now()));
        {
            let parser = parser.clone();
            let alive = alive.clone();
            let last_output = last_output.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 16 * 1024];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            parser.lock().unwrap().process(&buf[..n]);
                            *last_output.lock().unwrap() = Instant::now();
                            ctx.request_repaint();
                        }
                    }
                }
                alive.store(false, Ordering::SeqCst);
                ctx.request_repaint();
            });
        }

        let shell_name = Path::new(&shell)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "shell".into());

        Ok(Self { parser, writer, master: pair.master, child, alive, size, scroll: 0, scroll_acc: 0.0, shell_name, last_output, last_input: Instant::now() })
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    /// Name des Prozesses, der gerade im Vordergrund des Terminals läuft (z.B. `vim`, `claude`).
    pub fn foreground(&self) -> String {
        self.master
            .process_group_leader()
            .and_then(|pid| std::fs::read_to_string(format!("/proc/{pid}/comm")).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| self.shell_name.clone())
    }

    pub fn write(&mut self, bytes: &[u8]) {
        if self.scroll != 0 {
            self.scroll = 0;
            self.parser.lock().unwrap().screen_mut().set_scrollback(0);
        }
        self.last_input = Instant::now();
        let _ = self.writer.write_all(bytes);
        let _ = self.writer.flush();
    }

    /// Heuristik: Fragen/Freigaben und Fehler werden am Bildschirmtext erkannt,
    /// Arbeit an laufender Ausgabe, die nicht nur das Echo eigener Eingaben ist.
    pub fn status(&self) -> Status {
        if !self.is_alive() {
            return Status::Idle;
        }
        let text = self.parser.lock().unwrap().screen().contents();
        let lines: Vec<String> = text.lines().map(|l| l.trim().to_lowercase()).filter(|l| !l.is_empty()).collect();
        let tail = |n: usize| lines[lines.len().saturating_sub(n)..].to_vec();
        const ASK: &[&str] = &[
            "do you want to", "would you like to", "(y/n)", "[y/n]", "allow command", "allow this",
            "approve", "❯ 1. yes", "› 1. yes", "yes, and don't ask", "press enter to confirm",
        ];
        const ERR: &[&str] = &["api error", "error:", "fatal:", "panicked", "✗ "];
        let t = tail(14);
        // Claude, Codex & Co. zeigen beim Arbeiten "esc to interrupt" an.
        let interrupt = t.iter().any(|l| l.contains("to interrupt") || l.contains("esc interrupt") || l.contains("esc to cancel"));
        if !interrupt && t.iter().any(|l| ASK.iter().any(|k| l.contains(k))) {
            return Status::Question;
        }
        if interrupt {
            return Status::Working;
        }
        if tail(6).iter().any(|l| ERR.iter().any(|k| l.contains(k))) {
            return Status::Error;
        }
        Status::Idle
    }

    /// Die letzten `n` Zeilen des sichtbaren Bildschirms als Text (für Telegram).
    pub fn screen_text(&self, n: usize) -> String {
        let text = self.parser.lock().unwrap().screen().contents();
        let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
        let end = lines.iter().rposition(|l| !l.is_empty()).map_or(0, |i| i + 1);
        lines[end.saturating_sub(n)..end].join("\n")
    }

    fn resize(&mut self, rows: u16, cols: u16) {
        if (rows, cols) == self.size || rows == 0 || cols == 0 {
            return;
        }
        self.size = (rows, cols);
        let _ = self.master.resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 });
        self.parser.lock().unwrap().screen_mut().set_size(rows, cols);
    }

    /// Zeichnet das Terminal in `rect` und verarbeitet Eingaben, wenn es fokussiert ist.
    pub fn ui(&mut self, ui: &mut egui::Ui, rect: egui::Rect, id: egui::Id, focused: bool) -> egui::Response {
        let font = egui::FontId::monospace(13.0);
        let (cw, ch) = ui.fonts_mut(|f| (f.glyph_width(&font, 'M'), f.row_height(&font)));
        let inner = rect.shrink2(egui::vec2(8.0, 6.0));
        let cols = ((inner.width() / cw).floor() as u16).max(2);
        let rows = ((inner.height() / ch).floor() as u16).max(1);
        self.resize(rows, cols);

        let response = ui.interact(rect, id, egui::Sense::click());
        if focused {
            ui.memory_mut(|m| {
                m.request_focus(id);
                m.set_focus_lock_filter(
                    id,
                    egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: true },
                );
            });
            self.handle_input(ui);
        }
        if response.hovered() {
            let dy = ui.input(|i| i.smooth_scroll_delta.y);
            self.scroll_acc += dy;
            let lines = (self.scroll_acc / ch).trunc() as i64;
            if lines != 0 {
                self.scroll_acc -= lines as f32 * ch;
                let mut p = self.parser.lock().unwrap();
                let new = (self.scroll as i64 + lines).max(0) as usize;
                p.screen_mut().set_scrollback(new);
                self.scroll = p.screen().scrollback();
            }
        }

        let painter = ui.painter_at(rect);
        let parser = self.parser.lock().unwrap();
        let screen = parser.screen();
        let (srows, scols) = screen.size();
        let default_fg = egui::Color32::from_rgb(0xd4, 0xd4, 0xd4);

        for r in 0..srows {
            let y = inner.top() + r as f32 * ch;
            let mut job = egui::text::LayoutJob::default();
            for c in 0..scols {
                let Some(cell) = screen.cell(r, c) else { continue };
                if cell.is_wide_continuation() {
                    continue;
                }
                let mut fg = to_color(cell.fgcolor(), default_fg, cell.bold());
                let mut bg = to_color(cell.bgcolor(), egui::Color32::TRANSPARENT, false);
                if cell.inverse() {
                    let fg_old = fg;
                    fg = if bg == egui::Color32::TRANSPARENT { crate::theme::TERM_BG } else { bg };
                    bg = fg_old;
                }
                if cell.dim() {
                    fg = fg.gamma_multiply(0.6);
                }
                if bg != egui::Color32::TRANSPARENT {
                    let x = inner.left() + c as f32 * cw;
                    let w = if cell.is_wide() { cw * 2.0 } else { cw };
                    painter.rect_filled(egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, ch)), 0.0, bg);
                }
                let text = cell.contents();
                let text = if text.is_empty() { " " } else { text };
                job.append(
                    text,
                    0.0,
                    egui::TextFormat {
                        font_id: if cell.bold() { crate::theme::mono_bold(13.0) } else { font.clone() },
                        color: fg,
                        italics: cell.italic(),
                        underline: if cell.underline() { egui::Stroke::new(1.0, fg) } else { egui::Stroke::NONE },
                        ..Default::default()
                    },
                );
            }
            let galley = ui.fonts_mut(|f| f.layout_job(job));
            painter.galley(egui::pos2(inner.left(), y), galley, default_fg);
        }

        if self.scroll == 0 && !screen.hide_cursor() {
            let (cr, cc) = screen.cursor_position();
            let pos = egui::pos2(inner.left() + cc as f32 * cw, inner.top() + cr as f32 * ch);
            let r = egui::Rect::from_min_size(pos, egui::vec2(cw, ch));
            if focused {
                painter.rect_filled(r, 0.0, egui::Color32::from_rgba_unmultiplied(200, 200, 200, 170));
            } else {
                painter.rect_stroke(r, 0.0, egui::Stroke::new(1.0, egui::Color32::GRAY), egui::StrokeKind::Inside);
            }
        }
        if self.scroll > 0 {
            painter.text(
                rect.right_top() + egui::vec2(-10.0, 8.0),
                egui::Align2::RIGHT_TOP,
                format!("↑ {} Zeilen", self.scroll),
                egui::FontId::proportional(11.0),
                crate::theme::MUTED,
            );
        }
        response
    }

    fn handle_input(&mut self, ui: &egui::Ui) {
        let (app_cursor, bracketed) = {
            let p = self.parser.lock().unwrap();
            (p.screen().application_cursor(), p.screen().bracketed_paste())
        };
        let events = ui.input(|i| i.events.clone());
        let mut out: Vec<u8> = Vec::new();
        for ev in events {
            match ev {
                egui::Event::Text(t) => out.extend_from_slice(t.as_bytes()),
                egui::Event::Paste(t) => {
                    if bracketed {
                        out.extend_from_slice(b"\x1b[200~");
                        out.extend_from_slice(t.as_bytes());
                        out.extend_from_slice(b"\x1b[201~");
                    } else {
                        out.extend_from_slice(t.as_bytes());
                    }
                }
                egui::Event::Copy => out.push(0x03),
                egui::Event::Cut => out.push(0x18),
                egui::Event::Key { key, pressed: true, modifiers, .. } => {
                    if let Some(b) = key_bytes(key, modifiers, app_cursor) {
                        out.extend_from_slice(&b);
                    }
                }
                _ => {}
            }
        }
        if !out.is_empty() {
            self.write(&out);
        }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn key_bytes(key: egui::Key, m: egui::Modifiers, app_cursor: bool) -> Option<Vec<u8>> {
    use egui::Key::*;
    let arrow = |c: u8| -> Vec<u8> {
        if m.ctrl {
            vec![0x1b, b'[', b'1', b';', b'5', c]
        } else if m.alt {
            vec![0x1b, b'[', b'1', b';', b'3', c]
        } else if app_cursor {
            vec![0x1b, b'O', c]
        } else {
            vec![0x1b, b'[', c]
        }
    };
    let s = |x: &str| Some(x.as_bytes().to_vec());
    match key {
        Enter => s("\r"),
        Backspace => if m.ctrl || m.alt { s("\x1b\x7f") } else { s("\x7f") },
        Tab => if m.shift { s("\x1b[Z") } else { s("\t") },
        Escape => s("\x1b"),
        ArrowUp => Some(arrow(b'A')),
        ArrowDown => Some(arrow(b'B')),
        ArrowRight => Some(arrow(b'C')),
        ArrowLeft => Some(arrow(b'D')),
        Home => s("\x1b[H"),
        End => s("\x1b[F"),
        PageUp => s("\x1b[5~"),
        PageDown => s("\x1b[6~"),
        Delete => s("\x1b[3~"),
        Insert => s("\x1b[2~"),
        F1 => s("\x1bOP"),
        F2 => s("\x1bOQ"),
        F3 => s("\x1bOR"),
        F4 => s("\x1bOS"),
        F5 => s("\x1b[15~"),
        F6 => s("\x1b[17~"),
        F7 => s("\x1b[18~"),
        F8 => s("\x1b[19~"),
        F9 => s("\x1b[20~"),
        F10 => s("\x1b[21~"),
        F11 => s("\x1b[23~"),
        F12 => s("\x1b[24~"),
        _ if m.ctrl && !m.shift => {
            // Ctrl+Buchstabe -> Steuerzeichen (Ctrl+C kommt als Event::Copy)
            let name = key.name();
            let mut chars = name.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if c.is_ascii_alphabetic() => {
                    let b = (c.to_ascii_lowercase() as u8) & 0x1f;
                    Some(if m.alt { vec![0x1b, b] } else { vec![b] })
                }
                _ => match key {
                    OpenBracket => s("\x1b"),
                    Backslash => s("\x1c"),
                    CloseBracket => s("\x1d"),
                    Space => Some(vec![0]),
                    _ => None,
                },
            }
        }
        _ if m.alt && !m.ctrl => {
            let name = key.name();
            let mut chars = name.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if c.is_ascii_alphanumeric() => {
                    Some(vec![0x1b, c.to_ascii_lowercase() as u8])
                }
                _ => None,
            }
        }
        _ => None,
    }
}

fn to_color(c: vt100::Color, default: egui::Color32, bold: bool) -> egui::Color32 {
    match c {
        vt100::Color::Default => default,
        vt100::Color::Idx(i) => {
            let i = if bold && i < 8 { i + 8 } else { i };
            palette(i)
        }
        vt100::Color::Rgb(r, g, b) => egui::Color32::from_rgb(r, g, b),
    }
}

fn palette(i: u8) -> egui::Color32 {
    const BASE: [(u8, u8, u8); 16] = [
        (0x1e, 0x1e, 0x1e), (0xe0, 0x6c, 0x75), (0x98, 0xc3, 0x79), (0xe5, 0xc0, 0x7b),
        (0x61, 0xaf, 0xef), (0xc6, 0x78, 0xdd), (0x56, 0xb6, 0xc2), (0xd4, 0xd4, 0xd4),
        (0x5c, 0x63, 0x70), (0xff, 0x7b, 0x86), (0xb5, 0xe8, 0x90), (0xff, 0xd6, 0x8a),
        (0x7d, 0xc4, 0xff), (0xde, 0x9b, 0xf2), (0x7f, 0xd8, 0xe3), (0xff, 0xff, 0xff),
    ];
    match i {
        0..=15 => {
            let (r, g, b) = BASE[i as usize];
            egui::Color32::from_rgb(r, g, b)
        }
        16..=231 => {
            let i = i - 16;
            let v = |x: u8| if x == 0 { 0 } else { 55 + x * 40 };
            egui::Color32::from_rgb(v(i / 36), v((i / 6) % 6), v(i % 6))
        }
        _ => {
            let g = 8 + (i - 232) * 10;
            egui::Color32::from_rgb(g, g, g)
        }
    }
}

pub mod anyhow_lite {
    pub type Result<T> = std::result::Result<T, String>;
}
