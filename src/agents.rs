use std::path::PathBuf;

use eframe::egui::{self, Color32, Rect};

use crate::theme;

pub enum Logo {
    Svg(&'static str, &'static [u8]),
    #[allow(dead_code)]
    /// Fallback ohne offizielles Logo: Monogramm auf farbiger Fläche.
    Mono(&'static str, Color32),
    Shell,
}

pub struct Agent {
    pub id: &'static str,
    pub name: &'static str,
    /// Ausführbare Datei; `None` = einfache Shell.
    pub bin: Option<&'static str>,
    pub install: Option<&'static str>,
    pub logo: Logo,
}

macro_rules! svg {
    ($f:literal) => {
        Logo::Svg(concat!("bytes://", $f), include_bytes!(concat!("../assets/logos/", $f)))
    };
}

pub const AGENTS: &[Agent] = &[
    Agent { id: "shell", name: "Terminal", bin: None, install: None, logo: Logo::Shell },
    Agent {
        id: "claude",
        name: "Claude Code",
        bin: Some("claude"),
        install: Some("npm install -g @anthropic-ai/claude-code"),
        logo: svg!("claude-color.svg"),
    },
    Agent {
        id: "codex",
        name: "Codex",
        bin: Some("codex"),
        install: Some("npm install -g @openai/codex"),
        logo: svg!("openai.svg"),
    },
    Agent {
        id: "opencode",
        name: "OpenCode",
        bin: Some("opencode"),
        install: Some("npm install -g opencode-ai"),
        logo: svg!("opencode.svg"),
    },
    Agent {
        id: "gemini",
        name: "Gemini CLI",
        bin: Some("agy"),
        install: Some("npm install -g @google/gemini-cli"),
        logo: svg!("gemini-color.svg"),
    },
    Agent {
        id: "kimi",
        name: "Kimi Code",
        bin: Some("kimi"),
        install: Some("uv tool install --python 3.13 kimi-cli"),
        logo: svg!("kimi-color.svg"),
    },
    Agent {
        id: "qwen",
        name: "Qwen Code",
        bin: Some("qwen"),
        install: Some("npm install -g @qwen-code/qwen-code"),
        logo: svg!("qwen-color.svg"),
    },
    Agent {
        id: "omp",
        name: "Oh My Pi",
        bin: Some("omp"),
        install: Some("bun install -g @oh-my-pi/pi-coding-agent"),
        logo: svg!("omp.svg"),
    },
    Agent {
        id: "pi",
        name: "Pi",
        bin: Some("pi"),
        install: Some("npm install -g @mariozechner/pi-coding-agent"),
        logo: svg!("pi.svg"),
    },
    Agent {
        id: "copilot",
        name: "Copilot CLI",
        bin: Some("copilot"),
        install: Some("npm install -g @github/copilot"),
        logo: svg!("githubcopilot.svg"),
    },
    Agent {
        id: "cursor",
        name: "Cursor CLI",
        bin: Some("cursor-agent"),
        install: Some("curl https://cursor.com/install -fsS | bash"),
        logo: svg!("cursor.svg"),
    },
    Agent {
        id: "amp",
        name: "Amp",
        bin: Some("amp"),
        install: Some("npm install -g @sourcegraph/amp"),
        logo: svg!("amp.svg"),
    },
    Agent {
        id: "goose",
        name: "Goose",
        bin: Some("goose"),
        install: Some("curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | bash"),
        logo: svg!("goose.svg"),
    },
    Agent { id: "cline", name: "Cline CLI", bin: Some("cline"), install: Some("npm install -g cline"), logo: svg!("cline.svg") },
    Agent { id: "droid", name: "Factory Droid", bin: Some("droid"), install: Some("curl -fsSL https://app.factory.ai/cli | sh"), logo: svg!("factory.svg") },
    Agent { id: "kiro", name: "Kiro CLI", bin: Some("kiro-cli"), install: Some("curl -fsSL https://cli.kiro.dev/install | bash"), logo: svg!("kiro-color.svg") },
    Agent { id: "aider", name: "Aider", bin: Some("aider"), install: Some("curl -LsSf https://aider.chat/install.sh | sh"), logo: svg!("aider.png") },
    Agent { id: "auggie", name: "Auggie", bin: Some("auggie"), install: Some("npm install -g @augmentcode/auggie"), logo: svg!("augment.svg") },
    Agent { id: "continue", name: "Continue CLI", bin: Some("cn"), install: Some("npm install -g @continuedev/cli"), logo: svg!("continue.png") },
    Agent { id: "qoder", name: "Qoder CLI", bin: Some("qodercli"), install: Some("npm install -g @qoder-ai/qodercli"), logo: svg!("qoder-color.svg") },
    Agent { id: "vibe", name: "Mistral Vibe", bin: Some("vibe"), install: Some("uv tool install mistral-vibe"), logo: svg!("mistral-color.svg") },
    Agent { id: "grok", name: "Grok CLI", bin: Some("grok"), install: Some("npm install -g @vibe-kit/grok-cli"), logo: svg!("grok.svg") },
    Agent { id: "codebuddy", name: "CodeBuddy", bin: Some("codebuddy"), install: Some("npm install -g @tencent-ai/codebuddy-code"), logo: svg!("codebuddy-color.svg") },
    Agent { id: "codebuff", name: "Codebuff", bin: Some("codebuff"), install: Some("npm install -g codebuff"), logo: svg!("codebuff.png") },
];

/// Agent zum Namen eines laufenden Prozesses (z.B. `claude`, `.opencode`).
pub fn by_process(name: &str) -> Option<&'static Agent> {
    let n = name.trim_start_matches('.');
    AGENTS.iter().find(|a| a.bin.is_some_and(|b| b == n))
}

pub fn get(id: &str) -> Option<&'static Agent> {
    AGENTS.iter().find(|a| a.id == id)
}

/// Sucht eine ausführbare Datei im PATH und in typischen Installationsorten
/// (Apps aus dem Startmenü bekommen oft nicht den PATH der Shell).
pub fn is_installed(bin: &str) -> bool {
    resolve(bin).is_some()
}

/// Voller Pfad einer ausführbaren Datei (PATH plus typische Installationsorte).
pub fn resolve(bin: &str) -> Option<String> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    if let Some(h) = dirs::home_dir() {
        for d in [".local/bin", ".bun/bin", ".npm-global/bin", ".cargo/bin", ".deno/bin", ".volta/bin", "bin"] {
            dirs.push(h.join(d));
        }
    }
    dirs.iter().map(|d| d.join(bin)).find(|p| {
        p.metadata().map(|m| {
            use std::os::unix::fs::PermissionsExt;
            m.is_file() && m.permissions().mode() & 0o111 != 0
        }).unwrap_or(false)
    }).map(|p| p.display().to_string())
}

impl Agent {
    pub fn paint_logo(&self, ui: &egui::Ui, rect: Rect, faded: bool) {
        let tint = if faded { Color32::from_white_alpha(70) } else { Color32::WHITE };
        match &self.logo {
            Logo::Svg(uri, bytes) => {
                egui::Image::from_bytes(*uri, *bytes).tint(tint).paint_at(ui, rect);
            }
            Logo::Mono(text, color) => {
                let col = if faded { color.gamma_multiply(0.35) } else { *color };
                ui.painter().rect_filled(rect, rect.width() * 0.24, col);
                let size = if text.chars().count() > 2 { rect.height() * 0.38 } else { rect.height() * 0.6 };
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    *text,
                    theme::semibold(size),
                    if faded { Color32::from_white_alpha(120) } else { Color32::WHITE },
                );
            }
            Logo::Shell => {
                let col = if faded { theme::FAINT } else { theme::TEXT };
                let s = rect.width();
                let b = rect.shrink(s * 0.06);
                ui.painter().rect_stroke(b, s * 0.18, egui::Stroke::new(s * 0.07, col), egui::StrokeKind::Middle);
                let st = egui::Stroke::new(s * 0.08, col);
                let o = b.left_top() + egui::vec2(s * 0.24, s * 0.32);
                ui.painter().line_segment([o, o + egui::vec2(s * 0.16, s * 0.15)], st);
                ui.painter().line_segment([o + egui::vec2(s * 0.16, s * 0.15), o + egui::vec2(0.0, s * 0.3)], st);
                ui.painter().line_segment([o + egui::vec2(s * 0.26, s * 0.32), o + egui::vec2(s * 0.5, s * 0.32)], st);
            }
        }
    }
}
