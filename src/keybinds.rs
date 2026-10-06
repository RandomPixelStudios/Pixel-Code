//! Tastenkürzel der App, änderbar unter Settings > Keybinds (`~/.config/pixel-code/keybinds.json`).

use std::collections::BTreeMap;
use std::path::PathBuf;

use eframe::egui::{self, Key, Modifiers};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Cmd {
    NewTerminal,
    SplitRight,
    SplitDown,
    Close,
    Maximize,
    NextPane,
    PrevPane,
    NewSession,
    NextSession,
    PrevSession,
    AddProject,
    ToggleFiles,
    Settings,
}

impl Cmd {
    pub const ALL: &[Cmd] = &[
        Cmd::NewTerminal,
        Cmd::SplitRight,
        Cmd::SplitDown,
        Cmd::Close,
        Cmd::Maximize,
        Cmd::NextPane,
        Cmd::PrevPane,
        Cmd::NewSession,
        Cmd::NextSession,
        Cmd::PrevSession,
        Cmd::AddProject,
        Cmd::ToggleFiles,
        Cmd::Settings,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Cmd::NewTerminal => "New terminal",
            Cmd::SplitRight => "Split right",
            Cmd::SplitDown => "Split down",
            Cmd::Close => "Close terminal",
            Cmd::Maximize => "Maximize / restore terminal",
            Cmd::NextPane => "Next terminal",
            Cmd::PrevPane => "Previous terminal",
            Cmd::NewSession => "New session",
            Cmd::NextSession => "Next session",
            Cmd::PrevSession => "Previous session",
            Cmd::AddProject => "Add project",
            Cmd::ToggleFiles => "Show / hide files",
            Cmd::Settings => "Open settings",
        }
    }

    pub fn group(self) -> &'static str {
        match self {
            Cmd::NewTerminal | Cmd::SplitRight | Cmd::SplitDown | Cmd::Close | Cmd::Maximize | Cmd::NextPane | Cmd::PrevPane => "Terminals",
            Cmd::NewSession | Cmd::NextSession | Cmd::PrevSession | Cmd::AddProject => "Sessions & projects",
            Cmd::ToggleFiles | Cmd::Settings => "App",
        }
    }

    fn default_binding(self) -> Binding {
        let cs = Modifiers::CTRL | Modifiers::SHIFT;
        let (m, k) = match self {
            Cmd::NewTerminal => (cs, Key::T),
            Cmd::SplitRight => (cs, Key::D),
            Cmd::SplitDown => (cs, Key::E),
            Cmd::Close => (cs, Key::W),
            Cmd::Maximize => (cs, Key::Enter),
            Cmd::NextPane => (Modifiers::CTRL, Key::Tab),
            Cmd::PrevPane => (cs, Key::Tab),
            Cmd::NewSession => (cs, Key::N),
            Cmd::NextSession => (Modifiers::CTRL, Key::PageDown),
            Cmd::PrevSession => (Modifiers::CTRL, Key::PageUp),
            Cmd::AddProject => (cs, Key::O),
            Cmd::ToggleFiles => (cs, Key::B),
            Cmd::Settings => (Modifiers::CTRL, Key::Comma),
        };
        Binding { mods: m, key: k }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Binding {
    pub mods: Modifiers,
    pub key: Key,
}

impl Binding {
    /// z.B. "Ctrl+Shift+D"
    pub fn text(&self) -> String {
        let mut parts = Vec::new();
        if self.mods.ctrl || self.mods.command && !cfg!(target_os = "macos") {
            parts.push("Ctrl");
        }
        if self.mods.alt {
            parts.push("Alt");
        }
        if self.mods.shift {
            parts.push("Shift");
        }
        let key = match self.key {
            Key::Comma => ",",
            Key::Period => ".",
            Key::Minus => "-",
            Key::Plus => "+",
            k => k.name(),
        };
        parts.push(key);
        parts.join("+")
    }

    pub fn parse(s: &str) -> Option<Self> {
        let mut mods = Modifiers::NONE;
        let mut key = None;
        let parts: Vec<&str> = s.split('+').collect();
        // "Ctrl++" endet mit einem leeren Teil
        for (i, p) in parts.iter().enumerate() {
            match *p {
                "Ctrl" => mods = mods | Modifiers::CTRL,
                "Alt" => mods = mods | Modifiers::ALT,
                "Shift" => mods = mods | Modifiers::SHIFT,
                "" if i + 1 == parts.len() => key = Some(Key::Plus),
                "," => key = Some(Key::Comma),
                "." => key = Some(Key::Period),
                "-" => key = Some(Key::Minus),
                k => key = Key::from_name(k),
            }
        }
        Some(Self { mods, key: key? })
    }

    fn strength(&self) -> u8 {
        self.mods.ctrl as u8 + self.mods.shift as u8 + self.mods.alt as u8
    }
}

#[derive(Clone)]
pub struct Keybinds {
    /// `None` = Kürzel entfernt
    pub map: BTreeMap<Cmd, Option<Binding>>,
}

fn path() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("pixel-code").join("keybinds.json")
}

impl Default for Keybinds {
    fn default() -> Self {
        Self { map: Cmd::ALL.iter().map(|c| (*c, Some(c.default_binding()))).collect() }
    }
}

impl Keybinds {
    pub fn load() -> Self {
        let mut k = Self::default();
        let saved: BTreeMap<Cmd, Option<String>> =
            std::fs::read_to_string(path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        for (cmd, b) in saved {
            k.map.insert(cmd, b.as_deref().and_then(Binding::parse));
        }
        k
    }

    pub fn save(&self) {
        let out: BTreeMap<Cmd, Option<String>> = self.map.iter().map(|(c, b)| (*c, b.map(|b| b.text()))).collect();
        let p = path();
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        if let Ok(s) = serde_json::to_string_pretty(&out) {
            let _ = std::fs::write(p, s);
        }
    }

    pub fn get(&self, cmd: Cmd) -> Option<Binding> {
        self.map.get(&cmd).copied().flatten()
    }

    pub fn is_default(&self, cmd: Cmd) -> bool {
        self.get(cmd) == Some(cmd.default_binding())
    }

    pub fn reset(&mut self, cmd: Cmd) {
        self.map.insert(cmd, Some(cmd.default_binding()));
    }

    /// Text für Tooltips, z.B. "Split right  Ctrl+Shift+D".
    pub fn tip(&self, label: &str, cmd: Cmd) -> String {
        match self.get(cmd) {
            Some(b) => format!("{label}  {}", b.text()),
            None => label.to_string(),
        }
    }

    pub fn text(&self, cmd: Cmd) -> String {
        self.get(cmd).map(|b| b.text()).unwrap_or_default()
    }

    /// Andere Aktion, die schon dieses Kürzel hat.
    pub fn conflict(&self, cmd: Cmd, b: Binding) -> Option<Cmd> {
        self.map.iter().find(|(c, x)| **c != cmd && **x == Some(b)).map(|(c, _)| *c)
    }

    /// Verbraucht gedrückte Kürzel und gibt die ausgelösten Aktionen zurück.
    /// Kürzel mit mehr Modifiern zuerst, da egui zusätzliches Shift/Alt ignoriert.
    pub fn consume(&self, ctx: &egui::Context) -> Vec<Cmd> {
        let mut list: Vec<(Cmd, Binding)> = self.map.iter().filter_map(|(c, b)| b.map(|b| (*c, b))).collect();
        list.sort_by_key(|(_, b)| std::cmp::Reverse(b.strength()));
        ctx.input_mut(|i| list.into_iter().filter(|(_, b)| i.consume_key(b.mods, b.key)).map(|(c, _)| c).collect())
    }
}
