//! Pixel-Code-Plugins (GitHub, ioBroker, ...) und das Agent-Plugin "Pixel Code", das allen unterstützten
//! CLIs Zugriff darauf gibt und ihren Zustand (arbeitet, Frage, Freigabe, fertig) an die App meldet.
//! Beide Seiten teilen sich `~/.config/pixel-code/plugins.json`.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------- Registry

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub connected: bool,
    #[serde(default)]
    pub account: Option<String>,
    /// Programm, das der Agent über `pixelcode_plugin_run` aufrufen darf.
    pub command: String,
    /// Feste Argumente vor denen des Agents (z.B. der Pfad des Plugin-Skripts).
    #[serde(default)]
    pub prefix: Vec<String>,
    pub usage: String,
    /// Vom Nutzer eingetragene Werte (API-Keys, Pfade, ...); bekommt das Skript als PC_SETTINGS.
    #[serde(default)]
    pub settings: BTreeMap<String, String>,
    /// Kurze Statuszeile für Einstellungen und Agent.
    #[serde(default)]
    pub status: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Registry {
    pub version: u32,
    pub plugins: Vec<Entry>,
}

fn config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("pixel-code")
}

pub fn registry_path() -> PathBuf {
    config_dir().join("plugins.json")
}

pub fn load_registry() -> Registry {
    std::fs::read_to_string(registry_path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn update_entry(e: Entry) {
    let mut reg = load_registry();
    if reg.version < 2 {
        // Bis v1 wurden Skript-Plugins automatisch eingeschaltet
        for p in reg.plugins.iter_mut().filter(|p| p.id != "github") {
            p.enabled = false;
        }
        reg.version = 2;
    }
    match reg.plugins.iter_mut().find(|p| p.id == e.id) {
        // Schalter und Einstellungen des Nutzers behalten
        Some(p) => *p = Entry { enabled: p.enabled, settings: std::mem::take(&mut p.settings), ..e },
        None => reg.plugins.push(e),
    }
    save_registry(&reg);
}

pub fn save_registry(reg: &Registry) {
    let _ = std::fs::create_dir_all(config_dir());
    if let Ok(s) = serde_json::to_string_pretty(reg) {
        let _ = std::fs::write(registry_path(), s);
    }
}

pub fn set_enabled(id: &str, enabled: bool) {
    let mut reg = load_registry();
    if let Some(p) = reg.plugins.iter_mut().find(|p| p.id == id) {
        p.enabled = enabled;
        save_registry(&reg);
    }
}

// ---------------------------------------------------------------- GitHub

#[derive(Clone, PartialEq)]
pub enum GhState {
    Checking,
    Disconnected,
    Downloading,
    /// Wartet auf die Bestätigung im Browser.
    Code(String),
    Connected(String),
    Error(String),
}

#[derive(Clone)]
pub struct GitHub {
    pub state: Arc<Mutex<GhState>>,
}

impl GitHub {
    pub fn new(ctx: eframe::egui::Context) -> Self {
        let gh = Self { state: Arc::new(Mutex::new(GhState::Checking)) };
        let s = gh.clone();
        std::thread::spawn(move || {
            s.refresh();
            ctx.request_repaint();
        });
        gh
    }

    pub fn get(&self) -> GhState {
        self.state.lock().unwrap().clone()
    }

    fn set(&self, st: GhState) {
        *self.state.lock().unwrap() = st;
    }

    fn refresh(&self) {
        let login = gh_path().and_then(|gh| {
            let out = Command::new(gh).args(["api", "user", "--jq", ".login"]).stdin(Stdio::null()).output().ok()?;
            out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
        });
        let gh = gh_path().map(|p| p.display().to_string()).unwrap_or_else(|| managed_gh().display().to_string());
        update_entry(Entry {
            id: "github".into(),
            name: "GitHub".into(),
            description: "Repositories, pull requests, issues, releases and Actions via the GitHub CLI.".into(),
            enabled: false,
            connected: login.is_some(),
            account: login.clone(),
            command: gh,
            prefix: Vec::new(),
            settings: BTreeMap::new(),
            status: login.as_ref().map_or("Not connected".into(), |l| format!("Connected as {l}")),
            usage: "Arguments for the GitHub CLI (gh), e.g. 'repo view', 'pr create --fill', 'issue list', \
                    'api repos/{owner}/{repo}'. Plain `git push/pull` is also authenticated once GitHub is connected."
                .into(),
        });
        self.set(match login {
            Some(l) => GhState::Connected(l),
            None => GhState::Disconnected,
        });
    }

    /// Lädt bei Bedarf die GitHub CLI herunter und startet die Anmeldung im Browser.
    pub fn connect(&self, ctx: eframe::egui::Context) {
        let s = self.clone();
        std::thread::spawn(move || {
            let res = s.do_connect(&ctx);
            if let Err(e) = res {
                s.set(GhState::Error(e));
            }
            ctx.request_repaint();
        });
    }

    fn do_connect(&self, ctx: &eframe::egui::Context) -> Result<(), String> {
        let gh = match gh_path() {
            Some(p) => p,
            None => {
                self.set(GhState::Downloading);
                ctx.request_repaint();
                download_gh()?
            }
        };
        let mut child = Command::new(&gh)
            .args(["auth", "login", "--web", "-h", "github.com", "-p", "https", "--skip-ssh-key"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Could not start gh: {e}"))?;
        let stderr = child.stderr.take().unwrap();
        let mut log = String::new();
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            log.push_str(&line);
            log.push('\n');
            if let Some(code) = find_code(&line) {
                self.set(GhState::Code(code));
                let _ = Command::new("xdg-open").arg("https://github.com/login/device").spawn();
                ctx.request_repaint();
            }
        }
        let ok = child.wait().map(|s| s.success()).unwrap_or(false);
        if !ok {
            return Err(log.lines().last().unwrap_or("GitHub login failed").to_string());
        }
        // Git selbst (push/pull über https) mit dem gh-Token verbinden
        let _ = Command::new(&gh).args(["auth", "setup-git"]).output();
        self.refresh();
        set_enabled("github", true);
        Ok(())
    }

    pub fn disconnect(&self, ctx: eframe::egui::Context) {
        let s = self.clone();
        std::thread::spawn(move || {
            if let Some(gh) = gh_path() {
                let _ = Command::new(gh).args(["auth", "logout", "-h", "github.com"]).stdin(Stdio::null()).output();
            }
            s.refresh();
            ctx.request_repaint();
        });
    }
}

fn find_code(line: &str) -> Option<String> {
    line.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .find(|w| w.len() == 9 && w.as_bytes()[4] == b'-' && w.chars().all(|c| c == '-' || c.is_ascii_uppercase() || c.is_ascii_digit()))
        .map(str::to_string)
}

fn managed_gh() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("pixel-code/bin/gh")
}

pub fn gh_path() -> Option<PathBuf> {
    let managed = managed_gh();
    if managed.is_file() {
        return Some(managed);
    }
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .map(|d| d.join("gh"))
        .find(|p| p.is_file())
}

/// Holt die aktuelle GitHub CLI von github.com/cli/cli (kein sudo nötig).
fn download_gh() -> Result<PathBuf, String> {
    let arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        a => return Err(format!("Unsupported architecture: {a}")),
    };
    let target = managed_gh();
    let dir = target.parent().unwrap().to_path_buf();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let script = format!(
        "set -e; url=$(curl -fsSL https://api.github.com/repos/cli/cli/releases/latest \
         | grep -o '\"browser_download_url\": \"[^\"]*linux_{arch}.tar.gz\"' | cut -d'\"' -f4); \
         tmp=$(mktemp -d); curl -fsSL \"$url\" | tar xz -C \"$tmp\"; \
         mv \"$tmp\"/gh_*/bin/gh '{}'; rm -rf \"$tmp\"",
        target.display()
    );
    let out = Command::new("sh").args(["-c", &script]).output().map_err(|e| e.to_string())?;
    if !out.status.success() || !target.is_file() {
        return Err(format!("Downloading the GitHub CLI failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(target)
}

// ---------------------------------------------------------------- Skript-Plugins

pub struct Field {
    pub key: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
    pub secret: bool,
}

pub struct Def {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub usage: &'static str,
    pub logo: (&'static str, &'static [u8]),
    pub fields: &'static [Field],
}

const fn field(key: &'static str, label: &'static str, hint: &'static str, secret: bool) -> Field {
    Field { key, label, hint, secret }
}

macro_rules! logo {
    ($f:literal) => {
        (concat!("bytes://", $f), include_bytes!(concat!("../assets/logos/", $f)) as &[u8])
    };
}

/// Plugins, die als Node-Skript laufen (`assets/plugins/<id>.mjs`).
pub const DEFS: &[Def] = &[
    Def {
        id: "websearch",
        name: "Web Search",
        description: "Private web search with a local SearXNG (runs in Docker, starts automatically).",
        usage: "'search <query> [--n 8] [--time day|week|month|year] [--lang de]', 'fetch <url>' (page as text)",
        logo: logo!("searxng.svg"),
        fields: &[field("searxng", "SearXNG URL (optional)", "empty = local SearXNG on 127.0.0.1:8888", false)],
    },
    Def {
        id: "blender",
        name: "Blender",
        description: "Control Blender through the Blender MCP add-on: inspect scenes, run Python, take screenshots.",
        usage: "'scene', 'object <name>', 'exec <python>', 'exec-file <file.py>', 'screenshot [file]', 'raw <mcp command> [json]'",
        logo: logo!("blender.svg"),
        fields: &[field("host", "Host", "localhost", false), field("port", "Port", "9876", false)],
    },
    Def {
        id: "unity",
        name: "Unity",
        description: "Run the Unity Editor in batch mode: editor methods, builds and tests.",
        usage: "'method <Class.Method> [project]', 'build <Linux64|Windows64|OSX> <output> [project]', 'test [EditMode|PlayMode] [project]', 'open [project]'",
        logo: logo!("unity.svg"),
        fields: &[field("path", "Unity Editor path (optional)", "auto: ~/Unity/Hub/Editor/<newest>/Editor/Unity", false)],
    },
    Def {
        id: "unreal",
        name: "Unreal Engine",
        description: "Run Unreal Editor commands, Python scripts and BuildCookRun.",
        usage: "'python <Project.uproject> <script.py>', 'cmd <Project.uproject> [args]', 'build <Project.uproject> [Linux|Win64] [dir]', 'uat <args>', 'open <Project.uproject>'",
        logo: logo!("unrealengine.svg"),
        fields: &[field("engine", "Engine directory", "folder that contains Engine/, e.g. ~/UnrealEngine", false)],
    },
    Def {
        id: "godot",
        name: "Godot",
        description: "Run, script, import and export Godot projects from the command line.",
        usage: "'run [project]', 'script <file.gd> [project]', 'export <preset> <output> [project]', 'import [project]', or any godot arguments",
        logo: logo!("godotengine.svg"),
        fields: &[field("path", "Godot binary (optional)", "auto: godot4 / godot in PATH", false)],
    },
    Def {
        id: "leonardo",
        name: "Leonardo.ai",
        description: "Generate images with Leonardo.ai and save them into the project.",
        usage: "'generate <prompt> [--width 1024 --height 1024 --num 1 --model <id> --negative <text> --out dir]', 'models', 'status'",
        logo: logo!("leonardo.png"),
        fields: &[field("api_key", "API key", "from app.leonardo.ai > API Access", true)],
    },
    Def {
        id: "telegram",
        name: "Telegram",
        description: "Control your agents from Telegram and get notified when they finish or need input.",
        usage: "'send <text>' to message the user on Telegram",
        logo: logo!("telegram.svg"),
        fields: &[field("bot_token", "Bot token", "create a bot with @BotFather and paste its token", true)],
    },
    Def {
        id: "gitlab",
        name: "GitLab",
        description: "Projects, issues, merge requests and pipelines on gitlab.com or your own GitLab.",
        usage: "'projects', 'issues <group/project>', 'issue-create', 'mrs', 'mr-create', 'pipelines', 'raw'",
        logo: logo!("gitlab.svg"),
        fields: &[field("url", "GitLab URL", "https://gitlab.com", false), field("token", "Personal access token", "scope: api", true)],
    },
    Def {
        id: "gitea",
        name: "Gitea / Forgejo",
        description: "Repositories, issues and pull requests on Codeberg, Gitea or Forgejo.",
        usage: "'repos', 'issues <owner/repo>', 'issue-create', 'prs', 'pr-create', 'raw'",
        logo: logo!("gitea.svg"),
        fields: &[field("url", "Server URL", "https://codeberg.org", false), field("token", "Access token", "Settings > Applications", true)],
    },
    Def {
        id: "docker",
        name: "Docker",
        description: "Containers, images, logs and compose on this machine.",
        usage: "'ps', 'logs <name>', or any docker arguments ('compose up -d', 'build -t app .')",
        logo: logo!("docker.svg"),
        fields: &[],
    },
    Def {
        id: "database",
        name: "Database",
        description: "Query PostgreSQL, MySQL/MariaDB or SQLite. Read-only unless you allow writes.",
        usage: "'tables', 'schema <table>', 'query <sql>'",
        logo: logo!("database.svg"),
        fields: &[field("url", "Connection URL", "postgres://user:pass@host/db, mysql://..., or /path/to/file.sqlite", true), field("allow_writes", "Allow writes", "no / yes", false)],
    },
    Def {
        id: "sentry",
        name: "Sentry",
        description: "Read production errors and stack traces so the agent can fix them.",
        usage: "'projects', 'issues [project]', 'issue <id>', 'resolve <id>', 'raw'",
        logo: logo!("sentry.svg"),
        fields: &[field("org", "Organization slug", "the part after sentry.io/organizations/", false), field("token", "Auth token", "User settings > Auth Tokens", true), field("url", "Sentry URL (self-hosted)", "https://sentry.io", false)],
    },
    Def {
        id: "vercel",
        name: "Vercel",
        description: "Deploy and inspect Vercel projects.",
        usage: "'deployments', 'deploy [--prod]', 'logs <url>', 'cli <args>'",
        logo: logo!("vercel.svg"),
        fields: &[field("token", "Access token", "vercel.com/account/tokens", true)],
    },
    Def {
        id: "netlify",
        name: "Netlify",
        description: "Deploy and inspect Netlify sites.",
        usage: "'sites', 'deploys <site_id>', 'deploy [dir] [--prod]', 'cli <args>'",
        logo: logo!("netlify.svg"),
        fields: &[field("token", "Personal access token", "app.netlify.com > User settings > Applications", true)],
    },
    Def {
        id: "cloudflare",
        name: "Cloudflare",
        description: "DNS records, Workers and Pages.",
        usage: "'zones', 'dns <zone_id>', 'dns-add', 'wrangler <args>', 'raw'",
        logo: logo!("cloudflare.svg"),
        fields: &[field("token", "API token", "dash.cloudflare.com/profile/api-tokens", true)],
    },
    Def {
        id: "ssh",
        name: "SSH Servers",
        description: "Run commands and copy files on your servers with your SSH keys.",
        usage: "'hosts', 'run <host> <command>', 'copy <from> <to>'",
        logo: logo!("ssh.svg"),
        fields: &[field("hosts", "Hosts", "user@server, other-host (comma separated, from ~/.ssh/config)", false)],
    },
    Def {
        id: "playwright",
        name: "Browser (Playwright)",
        description: "Open web pages in a real browser: screenshots, visible text, test scripts.",
        usage: "'screenshot <url> [file] [--full]', 'text <url>', 'script <file.mjs>'",
        logo: logo!("playwright.svg"),
        fields: &[],
    },
    Def {
        id: "comfyui",
        name: "ComfyUI",
        description: "Generate images with your local ComfyUI (Stable Diffusion, Flux, ...).",
        usage: "'run <workflow_api.json> [--prompt ..] [--seed ..]', 'models', 'queue'",
        logo: logo!("comfyui.svg"),
        fields: &[field("url", "ComfyUI URL", "http://127.0.0.1:8188", false)],
    },
    Def {
        id: "elevenlabs",
        name: "ElevenLabs",
        description: "Voice-overs and sound effects for games and videos.",
        usage: "'speak <text> [--voice id] [--out file]', 'sfx <description>', 'voices'",
        logo: logo!("elevenlabs.svg"),
        fields: &[field("api_key", "API key", "elevenlabs.io > Profile > API keys", true)],
    },
    Def {
        id: "meshy",
        name: "Meshy",
        description: "Text or image to textured 3D models (GLB/FBX/OBJ).",
        usage: "'text <prompt> [--format glb]', 'image <file-or-url>'",
        logo: logo!("meshy.svg"),
        fields: &[field("api_key", "API key", "meshy.ai > Settings > API", true)],
    },
    Def {
        id: "tripo",
        name: "Tripo3D",
        description: "Text or image to 3D models (GLB).",
        usage: "'text <prompt>', 'image <file>'",
        logo: logo!("tripo.svg"),
        fields: &[field("api_key", "API key", "platform.tripo3d.ai > API keys", true)],
    },
    Def {
        id: "gimp",
        name: "GIMP",
        description: "Convert, resize and batch-edit images with GIMP (no window).",
        usage: "'convert <in> <out>', 'scale <in> <out> <w> <h>', 'batch <script-fu>'",
        logo: logo!("gimp.svg"),
        fields: &[field("path", "GIMP binary (optional)", "auto: gimp-console / gimp", false)],
    },
    Def {
        id: "krita",
        name: "Krita",
        description: "Export Krita documents and animations.",
        usage: "'export <file.kra> <out.png>', 'export-sequence', 'open'",
        logo: logo!("krita.svg"),
        fields: &[field("path", "Krita binary (optional)", "auto: krita", false)],
    },
    Def {
        id: "resolve",
        name: "DaVinci Resolve",
        description: "Import media and render timelines in a running DaVinci Resolve (Studio).",
        usage: "'info', 'import <files>', 'render [preset] [dir]', 'python <code>'",
        logo: logo!("resolve.svg"),
        fields: &[field("path", "Resolve install folder", "/opt/resolve", false)],
    },
    Def {
        id: "itch",
        name: "itch.io",
        description: "Upload game builds to itch.io with butler.",
        usage: "'push <folder> <user/game:channel> [version]', 'status-of <user/game>'",
        logo: logo!("itch.svg"),
        fields: &[field("api_key", "API key", "itch.io/user/settings/api-keys", true)],
    },
    Def {
        id: "steam",
        name: "Steam",
        description: "Upload builds with steamcmd (Steamworks).",
        usage: "'upload <app_build.vdf>', 'cmd <steamcmd commands>'",
        logo: logo!("steam.svg"),
        fields: &[field("username", "Steam build account", "", false), field("path", "steamcmd path (optional)", "auto", false)],
    },
    Def {
        id: "discord",
        name: "Discord",
        description: "Control your agents from a Discord channel and get notified there.",
        usage: "'send <text>', 'read [channel] [count]'",
        logo: logo!("discord.svg"),
        fields: &[field("bot_token", "Bot token", "discord.com/developers > Bot > Reset Token", true), field("channel_id", "Control channel id", "right-click the channel > Copy Channel ID", false)],
    },
    Def {
        id: "slack",
        name: "Slack",
        description: "Post messages, read channels and upload files.",
        usage: "'send [#channel] <text>', 'read [channel]', 'channels', 'upload <file>'",
        logo: logo!("slack.svg"),
        fields: &[field("bot_token", "Bot token", "xoxb-... from api.slack.com/apps > OAuth", true), field("channel", "Default channel id", "C0123...", false)],
    },
    Def {
        id: "whatsapp",
        name: "WhatsApp",
        description: "Get messages from your agents on WhatsApp (Meta Cloud API).",
        usage: "'send <text>'",
        logo: logo!("whatsapp.svg"),
        fields: &[field("token", "Access token", "developers.facebook.com > WhatsApp > API Setup", true), field("phone_number_id", "Phone number id", "shown on the API Setup page", false), field("to", "Your number", "e.g. 4915112345678", false)],
    },
    Def {
        id: "email",
        name: "E-Mail",
        description: "Send e-mails with attachments over SMTP.",
        usage: "'send <text> [--to ..] [--subject ..] [--attach files]'",
        logo: logo!("email.svg"),
        fields: &[field("smtp", "SMTP server", "smtp.gmail.com:465", false), field("user", "User / address", "you@gmail.com", false), field("password", "App password", "Gmail: myaccount.google.com/apppasswords", true), field("to", "Default recipient (optional)", "your own address", false)],
    },
    Def {
        id: "ntfy",
        name: "ntfy",
        description: "Push notifications to your phone, no account needed.",
        usage: "'send <text> [--title ..] [--priority high]'",
        logo: logo!("ntfy.svg"),
        fields: &[field("topic", "Topic", "a long secret name, e.g. pixelcode-x8k2...", false), field("server", "Server (optional)", "https://ntfy.sh", false)],
    },
    Def {
        id: "notion",
        name: "Notion",
        description: "Search, read and write Notion pages.",
        usage: "'search <query>', 'read <page_id>', 'append <page_id> <text>', 'create <parent> <title>', 'raw'",
        logo: logo!("notion.svg"),
        fields: &[field("token", "Integration secret", "notion.so/my-integrations", true)],
    },
    Def {
        id: "obsidian",
        name: "Obsidian",
        description: "Read, search and write notes in your Obsidian vault.",
        usage: "'list', 'search <text>', 'read <note>', 'write <note> <text>', 'append <note> <text>'",
        logo: logo!("obsidian.svg"),
        fields: &[field("vault", "Vault folder", "~/Documents/MyVault", false)],
    },
    Def {
        id: "linear",
        name: "Linear",
        description: "Issues, comments and states in Linear.",
        usage: "'teams', 'issues [TEAM]', 'issue <ABC-1>', 'create <TEAM> <title>', 'comment', 'state', 'gql'",
        logo: logo!("linear.svg"),
        fields: &[field("api_key", "Personal API key", "linear.app > Settings > Security & access", true)],
    },
    Def {
        id: "jira",
        name: "Jira",
        description: "Search, create, comment and move Jira issues.",
        usage: "'search [JQL]', 'issue <KEY-1>', 'create <PROJECT> <summary>', 'comment', 'move', 'raw'",
        logo: logo!("jira.svg"),
        fields: &[field("url", "Site URL", "https://yourteam.atlassian.net", false), field("email", "Account e-mail", "", false), field("token", "API token", "id.atlassian.com/manage-profile/security/api-tokens", true)],
    },
    Def {
        id: "trello",
        name: "Trello",
        description: "Boards, lists and cards.",
        usage: "'boards', 'lists <board>', 'cards <list>', 'add <list> <name>', 'move', 'comment'",
        logo: logo!("trello.svg"),
        fields: &[field("api_key", "API key", "trello.com/power-ups/admin > your Power-Up > API key", false), field("token", "Token", "generated next to the API key", true)],
    },
    Def {
        id: "docs",
        name: "Document Search",
        description: "Search your local documents: Markdown, text, PDF, Word and ODT.",
        usage: "'search <words>', 'read <file>', 'list'",
        logo: logo!("docs.svg"),
        fields: &[field("folders", "Folders", "~/Documents, ~/Notes (comma separated)", false)],
    },
    Def {
        id: "homeassistant",
        name: "Home Assistant",
        description: "Control your smart home (lights, switches, scenes).",
        usage: "'states [domain]', 'state <entity>', 'call <domain.service> [entity|json]'",
        logo: logo!("homeassistant.svg"),
        fields: &[field("url", "Home Assistant URL", "http://homeassistant.local:8123", false), field("token", "Long-lived access token", "Profile > Security > Long-lived access tokens", true)],
    },
    Def {
        id: "modrinth",
        name: "Modrinth",
        description: "Upload Minecraft mod versions and manage your Modrinth projects.",
        usage: "'projects', 'versions <project>', 'game-versions', 'upload <project> <file.jar> --version 1.2.0 --game-versions 1.21.1 --loaders fabric [--changelog ..] [--type release] [--deps id:required]', 'raw'",
        logo: logo!("modrinth.svg"),
        fields: &[field("token", "Personal access token", "modrinth.com/settings/pats (Create versions, Write projects)", true)],
    },
    Def {
        id: "curseforge",
        name: "CurseForge",
        description: "Upload Minecraft mod files to CurseForge.",
        usage: "'projects [username]', 'files <project>', 'info <project>', 'game-versions [filter]', 'upload <project_id> <file.jar> --game-versions 1.21.1 --loaders Fabric [--java 21] [--changelog ..] [--type release]'",
        logo: logo!("curseforge.svg"),
        fields: &[field("token", "Upload API token", "legacy.curseforge.com/account/api-tokens", true), field("api_key", "Core API key (for listing projects)", "console.curseforge.com > API keys", true), field("author", "Your CurseForge username", "", false)],
    },
    Def {
        id: "aws",
        name: "AWS",
        description: "Amazon Web Services through the AWS CLI: S3, EC2, Lambda, CloudWatch logs and everything else.",
        usage: "any AWS CLI command, e.g. 's3 ls', 's3 sync dist s3://bucket', 'ec2 describe-instances', 'logs tail <group>'",
        logo: logo!("aws.svg"),
        fields: &[field("access_key_id", "Access key id", "IAM > Users > Security credentials > Create access key", false), field("secret_access_key", "Secret access key", "", true), field("region", "Region", "eu-central-1", false)],
    },
    Def {
        id: "hetzner",
        name: "Hetzner Cloud",
        description: "Create, list and power Hetzner Cloud servers.",
        usage: "'servers', 'power <id> on|off|reboot', 'create <name> [type] [image] [location]', 'delete <id>', 'raw'",
        logo: logo!("hetzner.svg"),
        fields: &[field("token", "API token", "console.hetzner.cloud > Project > Security > API tokens (Read & Write)", true)],
    },
    Def {
        id: "digitalocean",
        name: "DigitalOcean",
        description: "Droplets and App Platform deployments.",
        usage: "'droplets', 'power <id> on|off|reboot', 'apps', 'deploy <app_id>', 'raw'",
        logo: logo!("digitalocean.svg"),
        fields: &[field("token", "Personal access token", "cloud.digitalocean.com/account/api/tokens", true)],
    },
    Def {
        id: "fly",
        name: "Fly.io",
        description: "Deploy and manage Fly.io apps with flyctl.",
        usage: "any flyctl command, e.g. 'apps list', 'deploy', 'status -a app', 'logs -a app --no-tail'",
        logo: logo!("fly.svg"),
        fields: &[field("token", "Access token", "fly.io/user/personal_access_tokens (or: fly tokens create org)", true)],
    },
    Def {
        id: "railway",
        name: "Railway",
        description: "Deploy and manage Railway projects with the Railway CLI.",
        usage: "any Railway CLI command, e.g. 'status', 'up', 'logs', 'variables', 'redeploy'",
        logo: logo!("railway.svg"),
        fields: &[field("token", "Account token", "railway.com/account/tokens", true), field("project_token", "Project token (optional)", "Project > Settings > Tokens", true)],
    },
    Def {
        id: "render",
        name: "Render",
        description: "Services, deploys and redeploys on Render.",
        usage: "'services', 'deploys <service_id>', 'deploy <service_id> [clear]', 'raw'",
        logo: logo!("render.svg"),
        fields: &[field("api_key", "API key", "dashboard.render.com > Account settings > API keys", true)],
    },
    Def {
        id: "heroku",
        name: "Heroku",
        description: "Apps, dynos, config vars and logs on Heroku.",
        usage: "'apps', 'dynos <app>', 'restart <app>', 'config <app>', 'config-set <app> KEY=value', 'logs <app>', 'raw'",
        logo: logo!("heroku.svg"),
        fields: &[field("api_key", "API key", "dashboard.heroku.com/account > API Key", true)],
    },
    Def {
        id: "linode",
        name: "Akamai / Linode",
        description: "Linode instances.",
        usage: "'instances', 'power <id> on|off|reboot', 'raw'",
        logo: logo!("linode.svg"),
        fields: &[field("token", "Personal access token", "cloud.linode.com/profile/tokens", true)],
    },
    Def {
        id: "vultr",
        name: "Vultr",
        description: "Vultr instances.",
        usage: "'instances', 'power <id> on|off|reboot', 'raw'",
        logo: logo!("vultr.svg"),
        fields: &[field("api_key", "API key", "my.vultr.com/settings/#settingsapi (allow your IP)", true)],
    },
    Def {
        id: "supabase",
        name: "Supabase",
        description: "Supabase projects, SQL queries and edge functions.",
        usage: "'projects', 'sql <project_ref> <query>', 'functions <project_ref>', 'cli <args>', 'raw'",
        logo: logo!("supabase.svg"),
        fields: &[field("token", "Access token", "supabase.com/dashboard/account/tokens", true)],
    },    Def {
        id: "iobroker",
        name: "ioBroker",
        description: "Read and switch ioBroker states (lights, sensors, scripts) through the simple-api adapter.",
        usage: "'states [pattern]', 'get <id>', 'value <id>', 'set <id> <value>', 'toggle <id>', 'objects [pattern]', 'search <text>', 'rooms', 'functions'",
        logo: logo!("iobroker.svg"),
        fields: &[field("url", "simple-api URL", "http://iobroker.local:8087", false), field("user", "User (optional)", "only if authentication is enabled", false), field("password", "Password (optional)", "", true)],
    },
    Def {
        id: "iobroker-vis",
        name: "ioBroker VIS 2",
        description: "Build ioBroker VIS 2 visualizations: views, widgets, CSS, example states and screenshots through the rest-api adapter.",
        usage: "'projects', 'views <project>', 'widgets', 'widget <tpl>', 'add-view <project> <name>', 'add-widget <project> <view> <tpl> --oid <state> --x --y --w --h', 'set-widget', 'del-widget', 'put <project> <file.json> [--view]', 'css <project> [file]', 'restore <project>', 'reload', 'screenshot <project> [view] [file.png] [--dark] [--width]', 'states <pattern>', 'create-state <id> [value]', 'create-states <file.json>', 'set-state <id> <value>'",
        logo: logo!("iobroker.svg"),
        fields: &[
            field("url", "rest-api URL", "http://iobroker.local:8093", false),
            field("instance", "VIS instance (optional)", "vis-2.0", false),
            field("web", "VIS web URL (optional)", "for screenshots, default: same host on port 8082", false),
            field("user", "User (optional)", "only if authentication is enabled", false),
            field("password", "Password (optional)", "", true),
        ],
    },
    Def {
        id: "ollama",
        name: "Ollama",
        description: "Ask local LLMs running in Ollama, list and pull models.",
        usage: "'models', 'running', 'ask <prompt> [--model m]', 'embed <text>', 'pull <model>'",
        logo: logo!("ollama.svg"),
        fields: &[field("url", "Ollama URL", "http://127.0.0.1:11434", false), field("model", "Default model (optional)", "e.g. llama3.2", false)],
    },
    Def {
        id: "proxmox",
        name: "Proxmox VE",
        description: "Nodes, VMs and containers on your Proxmox server.",
        usage: "'nodes', 'vms', 'power <vmid> on|off|stop|reboot', 'raw <METHOD> </path>'",
        logo: logo!("proxmox.svg"),
        fields: &[field("url", "Proxmox URL", "https://proxmox.local:8006", false), field("token_id", "API token id", "user@pam!pixelcode", false), field("secret", "API token secret", "Datacenter > Permissions > API Tokens", true), field("insecure", "Allow self-signed certificate", "no / yes", false)],
    },
    Def {
        id: "kubernetes",
        name: "Kubernetes",
        description: "Pods, deployments and logs in your clusters through kubectl.",
        usage: "any kubectl arguments, e.g. 'get pods -A', 'logs deploy/web', 'rollout restart deploy/web'",
        logo: logo!("kubernetes.svg"),
        fields: &[field("context", "Context (optional)", "default: current kubectl context", false), field("kubeconfig", "Kubeconfig (optional)", "~/.kube/config", false)],
    },
    Def {
        id: "stripe",
        name: "Stripe",
        description: "Payments, customers, products and subscriptions.",
        usage: "'balance', 'payments', 'customers', 'products', 'subscriptions', 'raw <METHOD> </path>'",
        logo: logo!("stripe.svg"),
        fields: &[field("api_key", "API key", "a restricted key from dashboard.stripe.com/apikeys", true)],
    },
    Def {
        id: "todoist",
        name: "Todoist",
        description: "Read, add and complete your Todoist tasks.",
        usage: "'projects', 'tasks [project] [--filter today]', 'add <text> [--due ..]', 'done <id>'",
        logo: logo!("todoist.svg"),
        fields: &[field("token", "API token", "todoist.com > Settings > Integrations > Developer", true)],
    },
    Def {
        id: "npm",
        name: "npm",
        description: "Look up and publish npm packages.",
        usage: "'info <pkg>', 'search <words>', 'downloads <pkg>', or any npm arguments ('publish', 'version patch')",
        logo: logo!("npm.svg"),
        fields: &[field("token", "Access token (optional)", "npmjs.com > Access Tokens (for publish)", true)],
    },
    Def {
        id: "gcloud",
        name: "Google Cloud",
        description: "Google Cloud through the gcloud CLI: Compute, Cloud Run, Storage, logs.",
        usage: "any gcloud command, e.g. 'run deploy', 'compute instances list', 'storage ls'",
        logo: logo!("gcloud.svg"),
        fields: &[field("project", "Project id (optional)", "my-project", false)],
    },
    Def {
        id: "azure",
        name: "Microsoft Azure",
        description: "Azure resources through the az CLI.",
        usage: "any az command, e.g. 'group list', 'webapp up', 'vm list'",
        logo: logo!("azure.svg"),
        fields: &[field("subscription", "Subscription (optional)", "", false)],
    },
    Def {
        id: "firebase",
        name: "Firebase",
        description: "Deploy Hosting, Functions and Firestore rules with the Firebase CLI.",
        usage: "any firebase command, e.g. 'deploy', 'projects:list', 'hosting:channel:deploy preview'",
        logo: logo!("firebase.svg"),
        fields: &[field("token", "CI token (optional)", "firebase login:ci", true)],
    },
    Def {
        id: "terraform",
        name: "Terraform",
        description: "Plan and apply infrastructure with Terraform or OpenTofu.",
        usage: "'init', 'plan', 'apply -auto-approve', 'output', any terraform arguments",
        logo: logo!("terraform.svg"),
        fields: &[],
    },
    Def {
        id: "ansible",
        name: "Ansible",
        description: "Run Ansible playbooks and ad-hoc commands on your servers.",
        usage: "'playbook site.yml -i hosts', or any ansible-playbook arguments",
        logo: logo!("ansible.svg"),
        fields: &[field("inventory", "Default inventory (optional)", "~/ansible/hosts", false)],
    },
    Def {
        id: "redis",
        name: "Redis",
        description: "Query and inspect Redis with redis-cli.",
        usage: "any redis-cli command, e.g. 'KEYS user:*', 'GET key', 'INFO'",
        logo: logo!("redis.svg"),
        fields: &[field("url", "Redis URL", "redis://127.0.0.1:6379", true)],
    },
    Def {
        id: "mongodb",
        name: "MongoDB",
        description: "Run queries against MongoDB with mongosh.",
        usage: "'--eval \"db.users.find().limit(5)\"', any mongosh arguments",
        logo: logo!("mongodb.svg"),
        fields: &[field("url", "Connection string", "mongodb://localhost:27017/app", true)],
    },
    Def {
        id: "ffmpeg",
        name: "FFmpeg",
        description: "Convert, cut and compress video and audio files.",
        usage: "any ffmpeg arguments, e.g. '-i in.mov -c:v libx264 out.mp4'",
        logo: logo!("ffmpeg.svg"),
        fields: &[],
    },
    Def {
        id: "imagemagick",
        name: "ImageMagick",
        description: "Resize, convert and edit images in bulk.",
        usage: "any magick arguments, e.g. 'in.png -resize 50% out.webp'",
        logo: logo!("imagemagick.svg"),
        fields: &[],
    },
    Def {
        id: "ytdlp",
        name: "yt-dlp",
        description: "Download videos and audio from YouTube and many other sites.",
        usage: "any yt-dlp arguments, e.g. '-x --audio-format mp3 <url>'",
        logo: logo!("ytdlp.svg"),
        fields: &[],
    },
    Def {
        id: "mqtt",
        name: "MQTT",
        description: "Publish and subscribe to MQTT topics (Mosquitto clients).",
        usage: "'pub -t topic -m message', 'sub -t topic -C 5'",
        logo: logo!("mqtt.svg"),
        fields: &[field("host", "Broker host", "localhost", false), field("user", "User (optional)", "", false), field("password", "Password (optional)", "", true)],
    },
    Def {
        id: "openai",
        name: "OpenAI",
        description: "Generate images, speech and text with the OpenAI API.",
        usage: "'image <prompt> [--size 1024x1024] [--out file]', 'speak <text> [--out file]', 'ask <prompt>'",
        logo: logo!("openai.svg"),
        fields: &[field("api_key", "API key", "platform.openai.com/api-keys", true), field("model", "Text model (optional)", "gpt-5-mini", false)],
    },
    Def {
        id: "openrouter",
        name: "OpenRouter",
        description: "Ask any model (Claude, GPT, Gemini, Llama, ...) for a second opinion.",
        usage: "'ask <prompt> [--model id]', 'models [filter]'",
        logo: logo!("openrouter.svg"),
        fields: &[field("api_key", "API key", "openrouter.ai/keys", true), field("model", "Default model", "anthropic/claude-sonnet-4.5", false)],
    },
    Def {
        id: "lmstudio",
        name: "LM Studio",
        description: "Ask local models running in LM Studio.",
        usage: "'models', 'ask <prompt> [--model id]'",
        logo: logo!("lmstudio.svg"),
        fields: &[field("url", "Server URL", "http://127.0.0.1:1234", false)],
    },
    Def {
        id: "replicate",
        name: "Replicate",
        description: "Run open-source AI models (images, video, audio, upscaling) on Replicate.",
        usage: "'run <owner/model> <json input>', 'search <words>'",
        logo: logo!("replicate.svg"),
        fields: &[field("token", "API token", "replicate.com/account/api-tokens", true)],
    },
    Def {
        id: "huggingface",
        name: "Hugging Face",
        description: "Search models and datasets, run inference on Hugging Face.",
        usage: "'models <search>', 'datasets <search>', 'run <model> <text>'",
        logo: logo!("huggingface.svg"),
        fields: &[field("token", "Access token", "huggingface.co/settings/tokens", true)],
    },
    Def {
        id: "deepl",
        name: "DeepL",
        description: "Translate texts and UI strings with DeepL.",
        usage: "'translate <text> --to DE [--from EN]', 'usage'",
        logo: logo!("deepl.svg"),
        fields: &[field("api_key", "API key", "deepl.com/account/summary", true)],
    },
    Def {
        id: "brave",
        name: "Brave Search",
        description: "Web and news search through the Brave Search API.",
        usage: "'search <query> [--n 10]', 'news <query>'",
        logo: logo!("brave.svg"),
        fields: &[field("api_key", "API key", "api-dashboard.search.brave.com", true)],
    },
    Def {
        id: "wolfram",
        name: "Wolfram|Alpha",
        description: "Exact answers for math, units, science and data.",
        usage: "'ask <question>'",
        logo: logo!("wolfram.svg"),
        fields: &[field("app_id", "App id", "developer.wolframalpha.com", true)],
    },
    Def {
        id: "wikipedia",
        name: "Wikipedia",
        description: "Search and read Wikipedia articles (no account needed).",
        usage: "'search <words> [--lang de]', 'read <title> [--lang de]'",
        logo: logo!("wikipedia.svg"),
        fields: &[field("lang", "Language", "en", false)],
    },
    Def {
        id: "weather",
        name: "Weather",
        description: "Current weather and forecast from Open-Meteo (no account needed).",
        usage: "'now <city>', 'forecast <city> [--days 7]'",
        logo: logo!("weather.svg"),
        fields: &[field("city", "Default city", "Berlin", false)],
    },
    Def {
        id: "bitbucket",
        name: "Bitbucket",
        description: "Repositories and pull requests on Bitbucket Cloud.",
        usage: "'repos', 'prs <workspace/repo>', 'raw'",
        logo: logo!("bitbucket.svg"),
        fields: &[field("user", "Username", "", false), field("token", "App password / API token", "bitbucket.org > Personal settings > App passwords", true), field("workspace", "Workspace", "", false)],
    },
    Def {
        id: "jenkins",
        name: "Jenkins",
        description: "Trigger and watch Jenkins jobs.",
        usage: "'jobs', 'build <job>', 'log <job> [build]', 'raw'",
        logo: logo!("jenkins.svg"),
        fields: &[field("url", "Jenkins URL", "https://jenkins.example.com", false), field("user", "User", "", false), field("token", "API token", "User > Configure > API Token", true)],
    },
    Def {
        id: "grafana",
        name: "Grafana",
        description: "Dashboards, alerts and data source queries in Grafana.",
        usage: "'dashboards [search]', 'alerts', 'raw'",
        logo: logo!("grafana.svg"),
        fields: &[field("url", "Grafana URL", "http://localhost:3000", false), field("token", "Service account token", "Administration > Service accounts", true)],
    },
    Def {
        id: "prometheus",
        name: "Prometheus",
        description: "Run PromQL queries and check alerts.",
        usage: "'query <promql>', 'alerts', 'targets'",
        logo: logo!("prometheus.svg"),
        fields: &[field("url", "Prometheus URL", "http://localhost:9090", false)],
    },
    Def {
        id: "portainer",
        name: "Portainer",
        description: "Containers and stacks on servers managed by Portainer.",
        usage: "'endpoints', 'containers <endpoint_id>', 'restart <endpoint_id> <container>', 'raw'",
        logo: logo!("portainer.svg"),
        fields: &[field("url", "Portainer URL", "https://portainer.local:9443", false), field("token", "Access token", "My account > Access tokens", true)],
    },
    Def {
        id: "coolify",
        name: "Coolify",
        description: "Deploy and manage apps on your self-hosted Coolify.",
        usage: "'apps', 'deploy <uuid>', 'servers', 'raw'",
        logo: logo!("coolify.svg"),
        fields: &[field("url", "Coolify URL", "https://coolify.example.com", false), field("token", "API token", "Keys & Tokens > API tokens", true)],
    },
    Def {
        id: "elasticsearch",
        name: "Elasticsearch",
        description: "Search and inspect Elasticsearch / OpenSearch indices.",
        usage: "'indices', 'search <index> <query>', 'raw'",
        logo: logo!("elasticsearch.svg"),
        fields: &[field("url", "URL", "http://localhost:9200", false), field("api_key", "API key (optional)", "", true)],
    },
    Def {
        id: "meilisearch",
        name: "Meilisearch",
        description: "Indexes, documents and search in Meilisearch.",
        usage: "'indexes', 'search <index> <query>', 'raw'",
        logo: logo!("meilisearch.svg"),
        fields: &[field("url", "URL", "http://localhost:7700", false), field("api_key", "Master / API key", "", true)],
    },
    Def {
        id: "posthog",
        name: "PostHog",
        description: "Product analytics: run HogQL queries, see events and feature flags.",
        usage: "'query <hogql>', 'flags', 'raw'",
        logo: logo!("posthog.svg"),
        fields: &[field("url", "PostHog URL", "https://eu.posthog.com", false), field("project", "Project id", "", false), field("token", "Personal API key", "Settings > Personal API keys", true)],
    },
    Def {
        id: "plausible",
        name: "Plausible",
        description: "Website traffic stats from Plausible Analytics.",
        usage: "'stats [period]', 'pages [period]', 'sources [period]'",
        logo: logo!("plausible.svg"),
        fields: &[field("url", "Plausible URL", "https://plausible.io", false), field("site", "Site domain", "example.com", false), field("token", "API key", "Account > API keys", true)],
    },
    Def {
        id: "wordpress",
        name: "WordPress",
        description: "Write and publish WordPress posts and pages.",
        usage: "'posts', 'post <title> <html> [--status draft]', 'pages', 'raw'",
        logo: logo!("wordpress.svg"),
        fields: &[field("url", "Site URL", "https://example.com", false), field("user", "User", "", false), field("password", "Application password", "Users > Profile > Application passwords", true)],
    },
    Def {
        id: "shopify",
        name: "Shopify",
        description: "Products, orders and inventory of your Shopify store.",
        usage: "'products', 'orders', 'gql <query>', 'raw'",
        logo: logo!("shopify.svg"),
        fields: &[field("store", "Store domain", "mystore.myshopify.com", false), field("token", "Admin API token", "Apps > Develop apps > Admin API access token", true)],
    },
    Def {
        id: "airtable",
        name: "Airtable",
        description: "Read and write Airtable records.",
        usage: "'bases', 'records <base> <table>', 'add <base> <table> <json>'",
        logo: logo!("airtable.svg"),
        fields: &[field("token", "Personal access token", "airtable.com/create/tokens", true)],
    },
    Def {
        id: "asana",
        name: "Asana",
        description: "Tasks and projects in Asana.",
        usage: "'projects', 'tasks <project>', 'add <project> <name>', 'done <task>'",
        logo: logo!("asana.svg"),
        fields: &[field("token", "Personal access token", "app.asana.com/0/my-apps", true)],
    },
    Def {
        id: "clickup",
        name: "ClickUp",
        description: "Tasks in ClickUp lists.",
        usage: "'teams', 'tasks <list_id>', 'add <list_id> <name>'",
        logo: logo!("clickup.svg"),
        fields: &[field("token", "API token", "Settings > Apps > API token", true)],
    },
    Def {
        id: "confluence",
        name: "Confluence",
        description: "Search and read Confluence pages.",
        usage: "'search <text>', 'page <id>', 'raw'",
        logo: logo!("confluence.svg"),
        fields: &[field("url", "Site URL", "https://yourteam.atlassian.net", false), field("email", "Account e-mail", "", false), field("token", "API token", "id.atlassian.com/manage-profile/security/api-tokens", true)],
    },
    Def {
        id: "dropbox",
        name: "Dropbox",
        description: "List, upload and download Dropbox files.",
        usage: "'ls [path]', 'get <path> [out]', 'put <file> <path>'",
        logo: logo!("dropbox.svg"),
        fields: &[field("token", "Access token", "dropbox.com/developers/apps > Generate token", true)],
    },
    Def {
        id: "nextcloud",
        name: "Nextcloud",
        description: "Files on your Nextcloud (WebDAV): list, upload, download.",
        usage: "'ls [path]', 'get <path> [out]', 'put <file> <path>'",
        logo: logo!("nextcloud.svg"),
        fields: &[field("url", "Nextcloud URL", "https://cloud.example.com", false), field("user", "User", "", false), field("password", "App password", "Settings > Security > Devices & sessions", true)],
    },
    Def {
        id: "figma",
        name: "Figma",
        description: "Read Figma files, export frames as images, read comments.",
        usage: "'file <key>', 'export <key> <node_ids> [--format png]', 'comments <key>'",
        logo: logo!("figma.svg"),
        fields: &[field("token", "Personal access token", "Figma > Settings > Security > Personal access tokens", true)],
    },
    Def {
        id: "unsplash",
        name: "Unsplash",
        description: "Find free high-quality photos and download them into the project.",
        usage: "'search <words>', 'get <photo_id> [out]'",
        logo: logo!("unsplash.svg"),
        fields: &[field("access_key", "Access key", "unsplash.com/oauth/applications", true)],
    },
    Def {
        id: "pexels",
        name: "Pexels",
        description: "Free stock photos and videos from Pexels.",
        usage: "'photos <words>', 'videos <words>', 'get <url> [out]'",
        logo: logo!("pexels.svg"),
        fields: &[field("api_key", "API key", "pexels.com/api", true)],
    },
    Def {
        id: "sketchfab",
        name: "Sketchfab",
        description: "Search and download downloadable 3D models from Sketchfab.",
        usage: "'search <words>', 'download <uid>'",
        logo: logo!("sketchfab.svg"),
        fields: &[field("token", "API token", "sketchfab.com/settings/password", true)],
    },
    Def {
        id: "youtube",
        name: "YouTube",
        description: "Search videos and read channel and video statistics.",
        usage: "'search <words>', 'video <id>', 'channel <id>'",
        logo: logo!("youtube.svg"),
        fields: &[field("api_key", "API key", "console.cloud.google.com > YouTube Data API v3", true)],
    },
    Def {
        id: "matrix",
        name: "Matrix",
        description: "Send messages to a Matrix / Element room.",
        usage: "'send <text> [--room id]', 'read [--room id]'",
        logo: logo!("matrix.svg"),
        fields: &[field("homeserver", "Homeserver", "https://matrix.org", false), field("token", "Access token", "Element > Settings > Help & About > Access token", true), field("room", "Room id", "!abc:matrix.org", false)],
    },
    Def {
        id: "mattermost",
        name: "Mattermost",
        description: "Post to Mattermost channels.",
        usage: "'send <text> [--channel id]', 'channels'",
        logo: logo!("mattermost.svg"),
        fields: &[field("url", "Server URL", "https://chat.example.com", false), field("token", "Personal access token", "Profile > Security > Personal access tokens", true), field("channel", "Default channel id", "", false)],
    },
    Def {
        id: "teams",
        name: "Microsoft Teams",
        description: "Post messages to a Teams channel through a workflow webhook.",
        usage: "'send <text>'",
        logo: logo!("teams.svg"),
        fields: &[field("webhook", "Webhook URL", "Channel > Workflows > Post to a channel when a webhook request is received", true)],
    },
    Def {
        id: "gotify",
        name: "Gotify",
        description: "Push notifications to your self-hosted Gotify server.",
        usage: "'send <text> [--title ..] [--priority 5]'",
        logo: logo!("gotify.svg"),
        fields: &[field("url", "Server URL", "https://gotify.example.com", false), field("token", "App token", "Apps > Create application", true)],
    },
    Def {
        id: "pushover",
        name: "Pushover",
        description: "Push notifications to your phone with Pushover.",
        usage: "'send <text> [--title ..] [--priority 0]'",
        logo: logo!("pushover.svg"),
        fields: &[field("token", "App token", "pushover.net/apps/build", true), field("user", "User key", "shown on pushover.net", true)],
    },
    Def {
        id: "twilio",
        name: "Twilio SMS",
        description: "Send SMS messages through Twilio.",
        usage: "'send <text> [--to +49...]'",
        logo: logo!("twilio.svg"),
        fields: &[field("sid", "Account SID", "", false), field("token", "Auth token", "", true), field("from", "From number", "+1555...", false), field("to", "Default recipient", "+49...", false)],
    },
    Def {
        id: "resend",
        name: "Resend",
        description: "Send transactional e-mails through Resend.",
        usage: "'send <text> --to .. --subject .. [--html]'",
        logo: logo!("resend.svg"),
        fields: &[field("api_key", "API key", "resend.com/api-keys", true), field("from", "From address", "app@yourdomain.com", false)],
    },
    Def {
        id: "hue",
        name: "Philips Hue",
        description: "Switch and dim Philips Hue lights and scenes.",
        usage: "'lights', 'on <id>', 'off <id>', 'bri <id> <0-254>', 'color <id> <hue 0-65535>', 'scenes'",
        logo: logo!("hue.svg"),
        fields: &[field("bridge", "Bridge IP", "192.168.1.20", false), field("username", "API user", "press the bridge button, then POST /api {\"devicetype\":\"pixelcode\"}", true)],
    },
    Def {
        id: "openhab",
        name: "openHAB",
        description: "Read and command openHAB items.",
        usage: "'items [filter]', 'get <item>', 'send <item> <command>'",
        logo: logo!("openhab.svg"),
        fields: &[field("url", "openHAB URL", "http://openhab.local:8080", false), field("token", "API token", "Profile > API tokens", true)],
    },
    Def {
        id: "nodered",
        name: "Node-RED",
        description: "List and deploy Node-RED flows.",
        usage: "'flows', 'export [file]', 'deploy <flows.json>'",
        logo: logo!("nodered.svg"),
        fields: &[field("url", "Node-RED URL", "http://localhost:1880", false), field("token", "Admin token (optional)", "", true)],
    },
    Def {
        id: "tasmota",
        name: "Tasmota",
        description: "Control Tasmota devices (plugs, bulbs, sensors) on your network.",
        usage: "'cmd <device> <command>', 'status <device>' - device = IP or name from the list",
        logo: logo!("tasmota.svg"),
        fields: &[field("devices", "Devices", "plug=192.168.1.50, lamp=192.168.1.51", false), field("password", "Web password (optional)", "", true)],
    },
    Def {
        id: "rcon",
        name: "Minecraft Server (RCON)",
        description: "Send commands to your Minecraft test server and read its log.",
        usage: "'cmd <command>' (reload, give, say ...), 'log [lines]'",
        logo: logo!("rcon.svg"),
        fields: &[field("host", "Host", "127.0.0.1", false), field("port", "RCON port", "25575", false), field("password", "RCON password", "rcon.password in server.properties", true), field("server_dir", "Server folder (optional)", "for reading logs/latest.log", false)],
    },
    Def {
        id: "pterodactyl",
        name: "Pterodactyl",
        description: "Start, stop and command game servers and upload mods via your Pterodactyl panel.",
        usage: "'servers', 'power <id> start|stop|restart', 'cmd <id> <command>', 'resources <id>', 'files <id> [dir]', 'upload <id> <file> [dir]'",
        logo: logo!("pterodactyl.svg"),
        fields: &[field("url", "Panel URL", "https://panel.example.com", false), field("api_key", "Client API key", "Account > API Credentials (ptlc_...)", true)],
    },
    Def {
        id: "crafty",
        name: "Crafty Controller",
        description: "Manage Minecraft servers in Crafty Controller.",
        usage: "'servers', 'action <id> start_server|stop_server|restart_server|backup_server', 'cmd <id> <command>', 'logs <id>'",
        logo: logo!("crafty.svg"),
        fields: &[field("url", "Crafty URL", "https://localhost:8443", false), field("token", "API token", "Panel > Users > API Keys", true)],
    },
    Def {
        id: "hangar",
        name: "Hangar (PaperMC)",
        description: "Upload plugin versions for Paper, Velocity and Waterfall to Hangar.",
        usage: "'projects [owner]', 'versions <project>', 'upload <project> <file.jar> --version 1.2.0 --platform PAPER --mc 1.21.1'",
        logo: logo!("hangar.png"),
        fields: &[field("api_key", "API key", "hangar.papermc.io > Settings > API Keys", true), field("owner", "Your user/organization (optional)", "", false)],
    },
    Def {
        id: "crowdin",
        name: "Crowdin",
        description: "Upload mod/app source strings and download translations.",
        usage: "'progress', 'upload <lang/en_us.json>', 'download [dir]'",
        logo: logo!("crowdin.svg"),
        fields: &[field("token", "Personal access token", "crowdin.com/settings#api-key", true), field("project_id", "Project id", "Project > Tools > API", false)],
    },
    Def {
        id: "mcversions",
        name: "Minecraft Versions",
        description: "Latest Minecraft, Fabric, Quilt, NeoForge, Forge and Parchment versions for gradle.properties.",
        usage: "'minecraft', 'fabric [mc]', 'quilt', 'neoforge [mc]', 'forge [mc]', 'parchment <mc>'",
        logo: logo!("mcversions.png"),
        fields: &[],
    },
    Def {
        id: "prism",
        name: "Prism Launcher",
        description: "Copy freshly built mods into a Prism instance, launch Minecraft and read the game log.",
        usage: "'instances', 'install <instance> <mod.jar>', 'remove <instance> <name>', 'launch <instance> [server]', 'log <instance>'",
        logo: logo!("prism.png"),
        fields: &[field("path", "Prism binary (optional)", "auto: prismlauncher / Flatpak", false), field("data_dir", "Data folder (optional)", "~/.local/share/PrismLauncher", false)],
    },
    Def {
        id: "aseprite",
        name: "Aseprite",
        description: "Export sprites, animations, spritesheets and layers from Aseprite files.",
        usage: "'export <in> <out.png> [--scale 4]', 'sheet <in> <sheet.png> [--tag]', 'layers <in>', 'info <in>', 'script <lua>'",
        logo: logo!("aseprite.svg"),
        fields: &[field("path", "Aseprite binary (optional)", "auto: aseprite in PATH", false)],
    },
    Def {
        id: "blockbench",
        name: "Blockbench",
        description: "Inspect .bbmodel files, extract textures and convert models to Minecraft Java JSON.",
        usage: "'info <model.bbmodel>', 'textures <model> [dir]', 'java <model> [out.json] [modid:block/]'",
        logo: logo!("blockbench.svg"),
        fields: &[],
    },
    Def {
        id: "tiled",
        name: "Tiled",
        description: "Export Tiled maps and tilesets.",
        usage: "'--export-map <map.tmx> <out.json>', '--export-tileset <set.tsx> <out.json>'",
        logo: logo!("tiled.png"),
        fields: &[],
    },
    Def {
        id: "ldtk",
        name: "LDtk",
        description: "Read LDtk projects: levels, layers and entities.",
        usage: "'levels <file.ldtk>', 'entities <file.ldtk> [level]', 'defs <file.ldtk>'",
        logo: logo!("ldtk.png"),
        fields: &[],
    },
    Def {
        id: "polyhaven",
        name: "Poly Haven",
        description: "Free CC0 textures, HDRIs and 3D models.",
        usage: "'search <words> [--type textures|hdris|models]', 'download <id> [--res 2k] [--out dir]'",
        logo: logo!("polyhaven.png"),
        fields: &[],
    },
    Def {
        id: "freesound",
        name: "Freesound",
        description: "Search and download sound effects.",
        usage: "'search <words> [--cc0]', 'download <id...> [--out dir]'",
        logo: logo!("freesound.png"),
        fields: &[field("api_key", "API key", "freesound.org/apiv2/apply", true)],
    },
    Def {
        id: "sox",
        name: "SoX",
        description: "Convert, trim, normalize and add effects to audio.",
        usage: "any sox arguments, e.g. 'in.wav out.ogg', 'in.wav out.wav trim 0 1.5 norm -1'",
        logo: logo!("sox.png"),
        fields: &[],
    },
    Def {
        id: "stability",
        name: "Stability AI",
        description: "Generate images, upscale and remove backgrounds.",
        usage: "'generate <prompt> [--model core|ultra] [--style pixel-art]', 'upscale <img>', 'remove-bg <img>'",
        logo: logo!("stability.svg"),
        fields: &[field("api_key", "API key", "platform.stability.ai/account/keys", true)],
    },
    Def {
        id: "fal",
        name: "fal.ai",
        description: "Run hundreds of image, video, audio and 3D models (Flux, Kling, Hunyuan3D, ...).",
        usage: "'run <model> <prompt> [--out dir]', 'run <model> --json {...}'",
        logo: logo!("fal.svg"),
        fields: &[field("api_key", "API key", "fal.ai/dashboard/keys", true)],
    },
    Def {
        id: "scenario",
        name: "Scenario",
        description: "Game assets in your own trained style.",
        usage: "'models', 'generate --model <id> <prompt> [--num 4]'",
        logo: logo!("scenario.png"),
        fields: &[field("api_key", "API key", "app.scenario.com > Settings > API", false), field("api_secret", "API secret", "", true)],
    },
    Def {
        id: "rodin",
        name: "Hyper3D Rodin",
        description: "High-quality text or image to 3D models.",
        usage: "'text <prompt>', 'image <img...>' [--format glb|fbx|obj] [--quality high]",
        logo: logo!("rodin.png"),
        fields: &[field("api_key", "API key", "hyper3d.ai > API", true)],
    },
    Def {
        id: "reddit",
        name: "Reddit",
        description: "Post devlogs and read subreddits.",
        usage: "'post <subreddit> --title .. [--text ..|--url ..]', 'read <sub>', 'comments <post>', 'reply <id> <text>'",
        logo: logo!("reddit.svg"),
        fields: &[field("client_id", "Client id", "reddit.com/prefs/apps > create 'script' app", false), field("client_secret", "Client secret", "", true), field("username", "Username", "", false), field("password", "Password", "", true)],
    },
    Def {
        id: "bluesky",
        name: "Bluesky",
        description: "Post devlogs with images and read your timeline.",
        usage: "'post <text> [--image a.png]', 'timeline'",
        logo: logo!("bluesky.svg"),
        fields: &[field("handle", "Handle", "you.bsky.social", false), field("app_password", "App password", "bsky.app > Settings > Privacy and security > App passwords", true)],
    },
    Def {
        id: "mastodon",
        name: "Mastodon",
        description: "Post toots with media and read notifications.",
        usage: "'post <text> [--image a.png]', 'notifications'",
        logo: logo!("mastodon.svg"),
        fields: &[field("instance", "Instance", "https://mastodon.social", false), field("token", "Access token", "Preferences > Development > New application", true)],
    },
    Def {
        id: "x",
        name: "X (Twitter)",
        description: "Post updates on X.",
        usage: "'post <text>', 'delete <id>'",
        logo: logo!("x.svg"),
        fields: &[field("api_key", "API key", "developer.x.com > Project > Keys and tokens", false), field("api_secret", "API secret", "", true), field("access_token", "Access token", "(Read and write)", false), field("access_secret", "Access token secret", "", true)],
    },
    Def {
        id: "twitch",
        name: "Twitch",
        description: "Stream title and category, clips and chat.",
        usage: "'stream', 'title [text] [--game ..]', 'clips', 'chat <message>'",
        logo: logo!("twitch.svg"),
        fields: &[field("client_id", "Client id", "dev.twitch.tv/console/apps", false), field("token", "User access token", "scopes channel:manage:broadcast user:write:chat", true)],
    },
    Def {
        id: "patreon",
        name: "Patreon",
        description: "Your campaign, patrons and posts.",
        usage: "'patrons', 'posts'",
        logo: logo!("patreon.svg"),
        fields: &[field("token", "Creator access token", "patreon.com/portal/registration/register-clients", true)],
    },
    Def {
        id: "gumroad",
        name: "Gumroad",
        description: "Products and sales.",
        usage: "'products', 'sales [after]'",
        logo: logo!("gumroad.svg"),
        fields: &[field("token", "Access token", "Settings > Advanced > Applications", true)],
    },
    Def {
        id: "lemonsqueezy",
        name: "Lemon Squeezy",
        description: "Products, orders and license keys.",
        usage: "'products', 'orders', 'licenses'",
        logo: logo!("lemonsqueezy.svg"),
        fields: &[field("api_key", "API key", "app.lemonsqueezy.com/settings/api", true)],
    },
    Def {
        id: "discordwebhook",
        name: "Discord Webhook",
        description: "Post changelogs and release notes into a Discord channel, no bot needed.",
        usage: "'send <text>', 'send --title \"v1.2\" --file CHANGELOG.md [--url ..] [--attach img.png]'",
        logo: logo!("discordwebhook.svg"),
        fields: &[field("url", "Webhook URL", "Channel settings > Integrations > Webhooks", true), field("name", "Display name (optional)", "", false)],
    },
    Def {
        id: "bitwarden",
        name: "Bitwarden",
        description: "Read secrets from your Bitwarden vault.",
        usage: "any bw arguments, e.g. 'get password github.com', 'list items --search x'",
        logo: logo!("bitwarden.svg"),
        fields: &[field("session", "Session key", "run 'bw unlock --raw' in a terminal", true)],
    },
    Def {
        id: "onepassword",
        name: "1Password",
        description: "Read secrets with the 1Password CLI.",
        usage: "any op arguments, e.g. 'read op://Dev/Item/field', 'item list'",
        logo: logo!("onepassword.svg"),
        fields: &[field("token", "Service account token (optional)", "or use the desktop app integration", true)],
    },
    Def {
        id: "doppler",
        name: "Doppler",
        description: "Secrets from Doppler for your projects.",
        usage: "any doppler arguments, e.g. 'secrets --project app --config dev', 'run -- npm start'",
        logo: logo!("doppler.png"),
        fields: &[field("token", "Service token", "dashboard.doppler.com > Access", true)],
    },
    Def {
        id: "tailscale",
        name: "Tailscale",
        description: "Devices and services in your tailnet.",
        usage: "'devices', any tailscale arguments ('ping', 'ip', 'serve 3000')",
        logo: logo!("tailscale.svg"),
        fields: &[field("api_key", "API key (optional)", "login.tailscale.com/admin/settings/keys", true)],
    },
    Def {
        id: "ngrok",
        name: "ngrok",
        description: "Share a local port publicly.",
        usage: "any ngrok arguments, e.g. 'http 3000 --log stdout'",
        logo: logo!("ngrok.svg"),
        fields: &[field("token", "Authtoken", "dashboard.ngrok.com/get-started/your-authtoken", true)],
    },
    Def {
        id: "cloudflared",
        name: "Cloudflare Tunnel",
        description: "Quick public URLs and named tunnels with cloudflared.",
        usage: "'tunnel --url http://localhost:3000', 'tunnel run', 'tunnel list'",
        logo: logo!("cloudflared.svg"),
        fields: &[field("token", "Tunnel token (optional)", "Zero Trust > Networks > Tunnels", true)],
    },
    Def {
        id: "uptimekuma",
        name: "Uptime Kuma",
        description: "Monitor status from your Uptime Kuma and push heartbeats.",
        usage: "'monitors', 'push <token> [up|down] [msg]'",
        logo: logo!("uptimekuma.svg"),
        fields: &[field("url", "Uptime Kuma URL", "http://127.0.0.1:3001", false), field("slug", "Status page slug", "default", false)],
    },
    Def {
        id: "betterstack",
        name: "Better Stack",
        description: "Uptime monitors and incidents.",
        usage: "'monitors', 'incidents', 'pause <id>', 'resume <id>'",
        logo: logo!("betterstack.svg"),
        fields: &[field("token", "API token", "Better Stack > Settings > API tokens", true)],
    },
    Def {
        id: "sonarcloud",
        name: "SonarCloud",
        description: "Quality gate, issues and metrics from SonarCloud or SonarQube.",
        usage: "'gate <project>', 'issues <project> [severity]', 'metrics <project>'",
        logo: logo!("sonarcloud.svg"),
        fields: &[field("token", "Token", "My Account > Security", true), field("url", "Server (SonarQube only)", "https://sonarcloud.io", false)],
    },
    Def {
        id: "snyk",
        name: "Snyk",
        description: "Find vulnerabilities in dependencies, code and containers.",
        usage: "any snyk arguments, e.g. 'test', 'code test', 'container test img'",
        logo: logo!("snyk.svg"),
        fields: &[field("token", "API token", "app.snyk.io/account", true)],
    },
    Def {
        id: "codecov",
        name: "Codecov",
        description: "Test coverage of your repositories.",
        usage: "'repos', 'coverage <repo> [branch]', 'files <repo>'",
        logo: logo!("codecov.svg"),
        fields: &[field("token", "API token", "app.codecov.io > Settings > Access", true), field("owner", "Owner", "GitHub user or organization", false), field("service", "Service", "github", false)],
    },
    Def {
        id: "dockerhub",
        name: "Docker Hub",
        description: "Push images and list repositories and tags on Docker Hub.",
        usage: "'repos', 'tags <repo>', 'push <user/image:tag>'",
        logo: logo!("dockerhub.svg"),
        fields: &[field("username", "Username", "", false), field("token", "Access token", "hub.docker.com > Account settings > Personal access tokens", true)],
    },
    Def {
        id: "ghcr",
        name: "GitHub Packages (GHCR)",
        description: "Push container images to ghcr.io with your GitHub account.",
        usage: "'push ghcr.io/<owner>/<image>:<tag>', 'packages'",
        logo: logo!("ghcr.svg"),
        fields: &[field("token", "Token (optional)", "uses the GitHub plugin login if empty (needs write:packages)", true), field("username", "GitHub user (optional)", "", false)],
    },
    Def {
        id: "neon",
        name: "Neon",
        description: "Serverless Postgres: projects, branches and connection strings.",
        usage: "'projects', 'branches <project>', 'branch <project> <name>', 'uri <project> [branch]'",
        logo: logo!("neon.svg"),
        fields: &[field("api_key", "API key", "console.neon.tech > Account settings > API keys", true)],
    },
    Def {
        id: "turso",
        name: "Turso",
        description: "libSQL databases and auth tokens.",
        usage: "'databases', 'create <name>', 'token <db>'",
        logo: logo!("turso.svg"),
        fields: &[field("org", "Organization", "your Turso org slug", false), field("token", "Platform API token", "turso auth api-tokens mint pixelcode", true)],
    },
    Def {
        id: "pocketbase",
        name: "PocketBase",
        description: "Collections and records in your PocketBase.",
        usage: "'collections', 'list <collection>', 'create <c> <json>', 'update <c> <id> <json>', 'delete <c> <id>'",
        logo: logo!("pocketbase.svg"),
        fields: &[field("url", "PocketBase URL", "http://127.0.0.1:8090", false), field("email", "Superuser e-mail", "", false), field("password", "Superuser password", "", true)],
    },
    Def {
        id: "appwrite",
        name: "Appwrite",
        description: "Appwrite projects, databases, functions and deploys.",
        usage: "any appwrite arguments, e.g. 'push functions', 'databases list'",
        logo: logo!("appwrite.svg"),
        fields: &[],
    },
    Def {
        id: "rclone",
        name: "rclone",
        description: "Copy and sync to almost any cloud storage.",
        usage: "any rclone arguments, e.g. 'listremotes', 'copy build/ remote:releases'",
        logo: logo!("rclone.svg"),
        fields: &[],
    },
    Def {
        id: "restic",
        name: "restic",
        description: "Encrypted backups of your projects.",
        usage: "any restic arguments, e.g. 'snapshots', 'backup ~/Projects', 'restore latest --target /tmp/r'",
        logo: logo!("restic.png"),
        fields: &[field("repository", "Repository", "/mnt/backup/restic or s3:...", false), field("password", "Repository password", "", true)],
    },
    Def {
        id: "borg",
        name: "BorgBackup",
        description: "Deduplicated backups with Borg.",
        usage: "any borg arguments, e.g. 'list', 'create ::{now} ~/Projects'",
        logo: logo!("borg.svg"),
        fields: &[field("repository", "Repository", "/mnt/backup/borg or ssh://...", false), field("passphrase", "Passphrase", "", true)],
    },
    Def {
        id: "syncthing",
        name: "Syncthing",
        description: "Folder and device status of Syncthing.",
        usage: "'folders', 'devices', 'rescan [folder]'",
        logo: logo!("syncthing.svg"),
        fields: &[field("url", "GUI URL", "http://127.0.0.1:8384", false), field("api_key", "API key", "Actions > Settings > General", true)],
    },
    Def {
        id: "spotify",
        name: "Spotify",
        description: "Control music playback while coding.",
        usage: "'now', 'play [song|playlist <name>]', 'pause', 'next', 'previous', 'volume <0-100>'",
        logo: logo!("spotify.svg"),
        fields: &[field("client_id", "Client id", "developer.spotify.com/dashboard (redirect http://127.0.0.1:8899/callback)", false), field("client_secret", "Client secret", "", true)],
    },
    Def {
        id: "jellyfin",
        name: "Jellyfin",
        description: "Libraries, search, sessions and scans on your Jellyfin server.",
        usage: "'libraries', 'search <text>', 'sessions', 'scan'",
        logo: logo!("jellyfin.svg"),
        fields: &[field("url", "Server URL", "http://127.0.0.1:8096", false), field("api_key", "API key", "Dashboard > API Keys", true)],
    },
    Def {
        id: "plex",
        name: "Plex",
        description: "Libraries, search, sessions and scans on your Plex server.",
        usage: "'libraries', 'search <text>', 'sessions', 'scan <key>'",
        logo: logo!("plex.svg"),
        fields: &[field("url", "Server URL", "http://127.0.0.1:32400", false), field("token", "X-Plex-Token", "support.plex.tv: Finding an authentication token", true)],
    },
    Def {
        id: "pihole",
        name: "Pi-hole",
        description: "Stats, pause blocking and block/allow domains.",
        usage: "'stats', 'disable [seconds]', 'enable', 'block <domain>', 'allow <domain>'",
        logo: logo!("pihole.svg"),
        fields: &[field("url", "Pi-hole URL", "http://pi.hole", false), field("password", "Password / app password", "", true)],
    },
    Def {
        id: "truenas",
        name: "TrueNAS",
        description: "Pools, datasets, alerts, apps and snapshots.",
        usage: "'pools', 'datasets', 'alerts', 'apps', 'snapshot <dataset>'",
        logo: logo!("truenas.svg"),
        fields: &[field("url", "TrueNAS URL", "https://truenas.local", false), field("api_key", "API key", "Credentials > API Keys", true)],
    },
    Def {
        id: "unraid",
        name: "Unraid",
        description: "Array, disks, Docker containers and VMs (Unraid 7.2+).",
        usage: "'array', 'docker', 'vms', 'gql <query>'",
        logo: logo!("unraid.svg"),
        fields: &[field("url", "Server URL", "http://tower.local", false), field("api_key", "API key", "Settings > Management Access > API Keys", true)],
    },
    Def {
        id: "wol",
        name: "Wake-on-LAN",
        description: "Switch on PCs and servers over the network.",
        usage: "'list', 'wake <name|MAC>'",
        logo: logo!("wol.svg"),
        fields: &[field("devices", "Devices", "server=AA:BB:CC:DD:EE:FF, pc=11:22:33:44:55:66", false), field("broadcast", "Broadcast address (optional)", "255.255.255.255", false)],
    },
    Def {
        id: "rss",
        name: "RSS Feeds",
        description: "Watch changelogs and blogs (Minecraft, Fabric, mods, ...).",
        usage: "'latest [feed url...]'",
        logo: logo!("rss.svg"),
        fields: &[field("feeds", "Feed URLs", "comma separated, e.g. https://github.com/FabricMC/fabric/releases.atom", false)],
    },
    Def {
        id: "adb",
        name: "Android (adb)",
        description: "Install, start and debug apps on Android phones and emulators: logcat, screenshots, input.",
        usage: "'devices', 'install <apk>', 'start <pkg>', 'stop <pkg>', 'logcat [pkg]', 'screenshot [file]', 'tap x y', 'text ..', 'shell ..'",
        logo: logo!("adb.svg"),
        fields: &[field("sdk", "Android SDK (optional)", "~/Android/Sdk", false), field("device", "Device serial (optional)", "when several are connected", false)],
    },
    Def {
        id: "emulator",
        name: "Android Emulator",
        description: "List, create and start Android virtual devices.",
        usage: "'list', 'start <avd> [--headless]', 'create <name> [image] [device]'",
        logo: logo!("emulator.svg"),
        fields: &[field("sdk", "Android SDK (optional)", "~/Android/Sdk", false)],
    },
    Def {
        id: "androidsdk",
        name: "Android SDK Manager",
        description: "Install platforms, build tools, system images and NDK.",
        usage: "any sdkmanager arguments, e.g. '--list_installed', '\"platforms;android-35\"'",
        logo: logo!("androidsdk.svg"),
        fields: &[field("sdk", "Android SDK (optional)", "~/Android/Sdk", false)],
    },
    Def {
        id: "gradle",
        name: "Gradle",
        description: "Run Gradle tasks: Android, Minecraft mods (runClient), Kotlin Multiplatform, Maven publishing.",
        usage: "any tasks, e.g. 'build', 'runClient', 'assembleRelease', 'bundleRelease', 'publish', 'tasks --all'",
        logo: logo!("gradle.svg"),
        fields: &[],
    },
    Def {
        id: "scrcpy",
        name: "scrcpy",
        description: "Mirror and record an Android screen on the desktop.",
        usage: "'start [args]', 'record [file] [seconds]'",
        logo: logo!("scrcpy.svg"),
        fields: &[],
    },
    Def {
        id: "playconsole",
        name: "Google Play Console",
        description: "Upload app bundles to Play tracks, read and answer reviews.",
        usage: "'tracks', 'upload <app.aab> [--track internal] [--notes ..]', 'reviews', 'reply <id> <text>'",
        logo: logo!("playconsole.svg"),
        fields: &[field("key_file", "Service account JSON", "path to the downloaded key file", false), field("package", "Package name", "com.example.app", false)],
    },
    Def {
        id: "appstoreconnect",
        name: "App Store Connect",
        description: "TestFlight builds, groups and testers, reviews, certificates, profiles and devices.",
        usage: "'apps', 'builds <app>', 'groups <app>', 'add-build', 'add-tester', 'whats-new', 'submit-beta', 'reviews', 'certificates', 'profiles', 'devices', 'raw'",
        logo: logo!("appstoreconnect.svg"),
        fields: &[field("issuer_id", "Issuer ID", "App Store Connect > Users and Access > Integrations", false), field("key_id", "Key ID", "", false), field("key_file", "Private key (.p8)", "path to AuthKey_XXXX.p8", false)],
    },
    Def {
        id: "codemagic",
        name: "Codemagic",
        description: "Cloud builds for iOS and Android on Macs, with artifacts.",
        usage: "'apps', 'build <app> <workflow> [--wait]', 'builds <app>', 'artifacts <build>', 'cancel <build>'",
        logo: logo!("codemagic.svg"),
        fields: &[field("token", "API token", "Codemagic > Teams > Integrations > Codemagic API", true)],
    },
    Def {
        id: "bitrise",
        name: "Bitrise",
        description: "Mobile CI builds, artifacts and logs.",
        usage: "'apps', 'builds <app>', 'build <app> <workflow>', 'artifacts <app> <build>', 'log <app> <build>'",
        logo: logo!("bitrise.svg"),
        fields: &[field("token", "Personal access token", "Account settings > Security", true)],
    },
    Def {
        id: "expo",
        name: "Expo EAS",
        description: "Cloud builds for iOS and Android, store submission and OTA updates.",
        usage: "any eas arguments, e.g. 'build --platform ios --non-interactive', 'submit --platform ios --latest', 'update --branch production'",
        logo: logo!("expo.svg"),
        fields: &[field("token", "Access token", "expo.dev > Account settings > Access tokens", true)],
    },
    Def {
        id: "flutter",
        name: "Flutter",
        description: "Build, test and run Flutter apps.",
        usage: "any flutter arguments, e.g. 'build apk', 'test', 'devices', 'doctor'",
        logo: logo!("flutter.svg"),
        fields: &[],
    },
    Def {
        id: "fastlane",
        name: "fastlane",
        description: "Automate builds, screenshots, versions and store uploads.",
        usage: "any fastlane arguments, e.g. 'lanes', 'android beta'",
        logo: logo!("fastlane.svg"),
        fields: &[],
    },
    Def {
        id: "capacitor",
        name: "Capacitor",
        description: "Turn a web app into Android and iOS apps.",
        usage: "any cap arguments, e.g. 'sync', 'add android', 'run android'",
        logo: logo!("capacitor.svg"),
        fields: &[],
    },
    Def {
        id: "maestro",
        name: "Maestro",
        description: "UI tests for Android and iOS written as simple YAML flows.",
        usage: "any maestro arguments, e.g. 'test .maestro/', 'record flow.yaml'",
        logo: logo!("maestro.png"),
        fields: &[],
    },
    Def {
        id: "revenuecat",
        name: "RevenueCat",
        description: "In-app purchase and subscription metrics and customers.",
        usage: "'metrics', 'customer <id>', 'products', 'entitlements'",
        logo: logo!("revenuecat.svg"),
        fields: &[field("project_id", "Project id", "", false), field("api_key", "Secret API key (v2)", "Project settings > API keys", true)],
    },
    Def {
        id: "onesignal",
        name: "OneSignal",
        description: "Send push notifications to your app users.",
        usage: "'send <text> [--title ..] [--segment ..]', 'history'",
        logo: logo!("onesignal.png"),
        fields: &[field("app_id", "App ID", "Settings > Keys & IDs", false), field("api_key", "REST API key", "", true)],
    },
    Def {
        id: "tauri",
        name: "Tauri",
        description: "Build small desktop (and mobile) apps with Tauri.",
        usage: "any tauri arguments, e.g. 'build', 'dev', 'icon app-icon.png'",
        logo: logo!("tauri.svg"),
        fields: &[],
    },
    Def {
        id: "electron",
        name: "electron-builder",
        description: "Package Electron apps for Linux, Windows and macOS.",
        usage: "any electron-builder arguments, e.g. '--linux AppImage deb'",
        logo: logo!("electron.svg"),
        fields: &[field("github_token", "GitHub token (optional)", "for --publish", true)],
    },
    Def {
        id: "flatpak",
        name: "Flatpak",
        description: "Build and test Flatpak packages for Flathub.",
        usage: "any flatpak-builder arguments, e.g. '--user --install build-dir app.yml'",
        logo: logo!("flatpak.svg"),
        fields: &[],
    },
    Def {
        id: "snapcraft",
        name: "Snapcraft",
        description: "Build snaps and publish them to the Snap Store.",
        usage: "any snapcraft arguments, e.g. 'pack', 'upload --release=edge x.snap'",
        logo: logo!("snapcraft.svg"),
        fields: &[field("credentials", "Store credentials (optional)", "output of snapcraft export-login", true)],
    },
    Def {
        id: "appimage",
        name: "AppImage",
        description: "Package an AppDir into a portable AppImage.",
        usage: "'build <App.AppDir> [out.AppImage]'",
        logo: logo!("appimage.svg"),
        fields: &[],
    },
    Def {
        id: "lighthouse",
        name: "Lighthouse",
        description: "Performance, SEO and accessibility audits of web pages.",
        usage: "any lighthouse arguments, e.g. 'https://site --output=json --quiet'",
        logo: logo!("lighthouse.svg"),
        fields: &[],
    },
    Def {
        id: "algolia",
        name: "Algolia",
        description: "Search indices: query and upload records.",
        usage: "'indices', 'search <index> <q>', 'upload <index> <file.json>', 'clear <index>'",
        logo: logo!("algolia.svg"),
        fields: &[field("app_id", "Application ID", "", false), field("api_key", "Admin API key", "Settings > API Keys", true)],
    },
    Def {
        id: "chromewebstore",
        name: "Chrome Web Store",
        description: "Upload and publish browser extensions.",
        usage: "'upload <extension.zip>', 'publish [testers]'",
        logo: logo!("chromewebstore.svg"),
        fields: &[field("extension_id", "Extension id", "", false), field("client_id", "OAuth client id", "Google Cloud > Credentials", false), field("client_secret", "OAuth client secret", "", true), field("refresh_token", "Refresh token", "from the OAuth playground with chromewebstore scope", true)],
    },
    Def {
        id: "firefoxamo",
        name: "Firefox Add-ons",
        description: "Lint, build and sign Firefox extensions with web-ext.",
        usage: "any web-ext arguments, e.g. 'lint', 'build', 'sign --channel=listed'",
        logo: logo!("firefoxamo.svg"),
        fields: &[field("jwt_issuer", "JWT issuer", "addons.mozilla.org/developers/addon/api/key", false), field("jwt_secret", "JWT secret", "", true)],
    },
    Def {
        id: "cratesio",
        name: "crates.io",
        description: "Publish Rust crates.",
        usage: "'publish [--dry-run]', 'info <crate>'",
        logo: logo!("cratesio.svg"),
        fields: &[field("token", "API token", "crates.io/settings/tokens", true)],
    },
    Def {
        id: "pypi",
        name: "PyPI",
        description: "Build and upload Python packages.",
        usage: "'build', 'upload [files]', 'info <package>'",
        logo: logo!("pypi.svg"),
        fields: &[field("token", "API token", "pypi.org/manage/account/token", true), field("test", "Use TestPyPI", "no / yes", false)],
    },
    Def {
        id: "nuget",
        name: "NuGet",
        description: "Pack and push .NET packages.",
        usage: "'pack', 'push <file.nupkg>', 'info <package>'",
        logo: logo!("nuget.svg"),
        fields: &[field("api_key", "API key", "nuget.org/account/apikeys", true), field("source", "Source (optional)", "https://api.nuget.org/v3/index.json", false)],
    },
    Def {
        id: "jupyter",
        name: "Jupyter",
        description: "Execute and convert Jupyter notebooks.",
        usage: "'run <nb.ipynb>', 'convert <nb> [markdown|html|script]'",
        logo: logo!("jupyter.svg"),
        fields: &[],
    },
    Def {
        id: "kaggle",
        name: "Kaggle",
        description: "Datasets, competitions and notebooks.",
        usage: "any kaggle arguments, e.g. 'datasets list -s x', 'datasets download -d owner/name --unzip'",
        logo: logo!("kaggle.svg"),
        fields: &[field("username", "Username", "", false), field("key", "API key", "kaggle.com/settings > Create New Token", true)],
    },
    Def {
        id: "wandb",
        name: "Weights & Biases",
        description: "Training runs and metrics.",
        usage: "'projects [entity]', 'runs <entity/project>'",
        logo: logo!("wandb.svg"),
        fields: &[field("api_key", "API key", "wandb.ai/authorize", true)],
    },
    Def {
        id: "mintlify",
        name: "Mintlify",
        description: "Preview and check Mintlify documentation.",
        usage: "any mint arguments, e.g. 'dev', 'broken-links'",
        logo: logo!("mintlify.svg"),
        fields: &[],
    },
    Def {
        id: "docusaurus",
        name: "Docusaurus",
        description: "Build, serve and deploy Docusaurus sites.",
        usage: "'build', 'start', 'serve', 'deploy'",
        logo: logo!("docusaurus.svg"),
        fields: &[field("git_user", "GitHub user (for deploy)", "", false)],
    },
    Def {
        id: "gitbook",
        name: "GitBook",
        description: "Spaces, pages and search in GitBook.",
        usage: "'spaces', 'pages <space>', 'search <space> <q>'",
        logo: logo!("gitbook.svg"),
        fields: &[field("token", "API token", "app.gitbook.com/account/developer", true)],
    },
];

/// Kategorie eines Plugins (Filter und Gruppierung in den Einstellungen).
pub fn category(id: &str) -> &'static str {
    match id {
        "github" | "gitlab" | "gitea" | "docker" | "database" | "sentry" | "ssh" | "playwright" | "kubernetes" | "npm" => "Development",
        "vercel" | "netlify" | "cloudflare" | "aws" | "hetzner" | "digitalocean" | "fly" | "railway" | "render" | "heroku" | "linode" | "vultr" | "supabase" | "proxmox" => "Cloud",
        "unity" | "unreal" | "godot" | "itch" | "steam" | "modrinth" | "curseforge" => "Game Dev",
        "blender" | "leonardo" | "comfyui" | "elevenlabs" | "meshy" | "tripo" | "gimp" | "krita" | "resolve" => "Creative",
        "telegram" | "discord" | "slack" | "whatsapp" | "email" | "ntfy" => "Messaging",
        "notion" | "obsidian" | "linear" | "jira" | "trello" | "docs" | "todoist" | "stripe" => "Productivity",
        "homeassistant" | "iobroker" | "iobroker-vis" => "Smart Home",
        "websearch" | "ollama" => "AI & Search",
        "gcloud" | "azure" | "firebase" | "coolify" => "Cloud",
        "terraform" | "ansible" | "redis" | "mongodb" | "bitbucket" | "jenkins" | "grafana" | "prometheus" | "portainer" | "elasticsearch" | "meilisearch" => "Development",
        "ffmpeg" | "imagemagick" | "ytdlp" | "figma" | "unsplash" | "pexels" | "youtube" => "Creative",
        "mqtt" | "hue" | "openhab" | "nodered" | "tasmota" => "Smart Home",
        "openai" | "openrouter" | "lmstudio" | "replicate" | "huggingface" | "deepl" | "brave" | "wolfram" | "wikipedia" | "weather" => "AI & Search",
        "posthog" | "plausible" | "wordpress" | "shopify" | "airtable" | "asana" | "clickup" | "confluence" | "dropbox" | "nextcloud" => "Productivity",
        "sketchfab" => "Game Dev",
        "matrix" | "mattermost" | "teams" | "gotify" | "pushover" | "twilio" | "resend" => "Messaging",
        "rcon" | "pterodactyl" | "crafty" | "hangar" | "crowdin" | "mcversions" | "prism" | "aseprite" | "blockbench" | "tiled" | "ldtk" | "polyhaven" | "freesound" | "scenario" | "rodin" => "Game Dev",
        "sox" | "stability" | "fal" => "Creative",
        "reddit" | "bluesky" | "mastodon" | "x" | "twitch" | "patreon" | "gumroad" | "lemonsqueezy" => "Social",
        "discordwebhook" => "Messaging",
        "bitwarden" | "onepassword" | "doppler" | "tailscale" | "ngrok" | "cloudflared" | "uptimekuma" | "betterstack" | "sonarcloud" | "snyk" | "codecov" | "dockerhub" | "ghcr" | "restic" | "borg" | "syncthing" | "gradle" | "tauri" | "electron" | "flatpak" | "snapcraft" | "appimage" | "lighthouse" | "algolia" | "chromewebstore" | "firefoxamo" | "cratesio" | "pypi" | "nuget" => "Development",
        "neon" | "turso" | "pocketbase" | "appwrite" | "rclone" => "Cloud",
        "spotify" | "jellyfin" | "plex" | "pihole" | "truenas" | "unraid" | "wol" => "Smart Home",
        "rss" | "mintlify" | "docusaurus" | "gitbook" => "Productivity",
        "adb" | "emulator" | "androidsdk" | "scrcpy" | "playconsole" | "appstoreconnect" | "codemagic" | "bitrise" | "expo" | "flutter" | "fastlane" | "capacitor" | "maestro" | "revenuecat" | "onesignal" => "Mobile",
        "jupyter" | "kaggle" | "wandb" => "AI & Search",
        _ => "Other",
    }
}

pub const CATEGORIES: &[&str] = &["Development", "Cloud", "Game Dev", "Mobile", "Creative", "Social", "Messaging", "Productivity", "Smart Home", "AI & Search"];

const SCRIPTS: &[(&str, &str)] = &[
    ("common.mjs", include_str!("../assets/plugins/common.mjs")),
    ("websearch.mjs", include_str!("../assets/plugins/websearch.mjs")),
    ("blender.mjs", include_str!("../assets/plugins/blender.mjs")),
    ("unity.mjs", include_str!("../assets/plugins/unity.mjs")),
    ("unreal.mjs", include_str!("../assets/plugins/unreal.mjs")),
    ("godot.mjs", include_str!("../assets/plugins/godot.mjs")),
    ("leonardo.mjs", include_str!("../assets/plugins/leonardo.mjs")),
    ("telegram.mjs", include_str!("../assets/plugins/telegram.mjs")),
    ("gitlab.mjs", include_str!("../assets/plugins/gitlab.mjs")),
    ("gitea.mjs", include_str!("../assets/plugins/gitea.mjs")),
    ("docker.mjs", include_str!("../assets/plugins/docker.mjs")),
    ("database.mjs", include_str!("../assets/plugins/database.mjs")),
    ("sentry.mjs", include_str!("../assets/plugins/sentry.mjs")),
    ("vercel.mjs", include_str!("../assets/plugins/vercel.mjs")),
    ("netlify.mjs", include_str!("../assets/plugins/netlify.mjs")),
    ("cloudflare.mjs", include_str!("../assets/plugins/cloudflare.mjs")),
    ("ssh.mjs", include_str!("../assets/plugins/ssh.mjs")),
    ("playwright.mjs", include_str!("../assets/plugins/playwright.mjs")),
    ("comfyui.mjs", include_str!("../assets/plugins/comfyui.mjs")),
    ("elevenlabs.mjs", include_str!("../assets/plugins/elevenlabs.mjs")),
    ("meshy.mjs", include_str!("../assets/plugins/meshy.mjs")),
    ("tripo.mjs", include_str!("../assets/plugins/tripo.mjs")),
    ("gimp.mjs", include_str!("../assets/plugins/gimp.mjs")),
    ("krita.mjs", include_str!("../assets/plugins/krita.mjs")),
    ("resolve.mjs", include_str!("../assets/plugins/resolve.mjs")),
    ("itch.mjs", include_str!("../assets/plugins/itch.mjs")),
    ("steam.mjs", include_str!("../assets/plugins/steam.mjs")),
    ("discord.mjs", include_str!("../assets/plugins/discord.mjs")),
    ("slack.mjs", include_str!("../assets/plugins/slack.mjs")),
    ("whatsapp.mjs", include_str!("../assets/plugins/whatsapp.mjs")),
    ("email.mjs", include_str!("../assets/plugins/email.mjs")),
    ("ntfy.mjs", include_str!("../assets/plugins/ntfy.mjs")),
    ("notion.mjs", include_str!("../assets/plugins/notion.mjs")),
    ("obsidian.mjs", include_str!("../assets/plugins/obsidian.mjs")),
    ("linear.mjs", include_str!("../assets/plugins/linear.mjs")),
    ("jira.mjs", include_str!("../assets/plugins/jira.mjs")),
    ("trello.mjs", include_str!("../assets/plugins/trello.mjs")),
    ("docs.mjs", include_str!("../assets/plugins/docs.mjs")),
    ("homeassistant.mjs", include_str!("../assets/plugins/homeassistant.mjs")),
    ("modrinth.mjs", include_str!("../assets/plugins/modrinth.mjs")),
    ("curseforge.mjs", include_str!("../assets/plugins/curseforge.mjs")),
    ("aws.mjs", include_str!("../assets/plugins/aws.mjs")),
    ("hetzner.mjs", include_str!("../assets/plugins/hetzner.mjs")),
    ("digitalocean.mjs", include_str!("../assets/plugins/digitalocean.mjs")),
    ("fly.mjs", include_str!("../assets/plugins/fly.mjs")),
    ("railway.mjs", include_str!("../assets/plugins/railway.mjs")),
    ("render.mjs", include_str!("../assets/plugins/render.mjs")),
    ("heroku.mjs", include_str!("../assets/plugins/heroku.mjs")),
    ("linode.mjs", include_str!("../assets/plugins/linode.mjs")),
    ("vultr.mjs", include_str!("../assets/plugins/vultr.mjs")),
    ("supabase.mjs", include_str!("../assets/plugins/supabase.mjs")),
    ("iobroker.mjs", include_str!("../assets/plugins/iobroker.mjs")),
    ("iobroker-vis.mjs", include_str!("../assets/plugins/iobroker-vis.mjs")),
    ("ollama.mjs", include_str!("../assets/plugins/ollama.mjs")),
    ("proxmox.mjs", include_str!("../assets/plugins/proxmox.mjs")),
    ("kubernetes.mjs", include_str!("../assets/plugins/kubernetes.mjs")),
    ("stripe.mjs", include_str!("../assets/plugins/stripe.mjs")),
    ("todoist.mjs", include_str!("../assets/plugins/todoist.mjs")),
    ("npm.mjs", include_str!("../assets/plugins/npm.mjs")),
    ("gcloud.mjs", include_str!("../assets/plugins/gcloud.mjs")),
    ("azure.mjs", include_str!("../assets/plugins/azure.mjs")),
    ("firebase.mjs", include_str!("../assets/plugins/firebase.mjs")),
    ("terraform.mjs", include_str!("../assets/plugins/terraform.mjs")),
    ("ansible.mjs", include_str!("../assets/plugins/ansible.mjs")),
    ("redis.mjs", include_str!("../assets/plugins/redis.mjs")),
    ("mongodb.mjs", include_str!("../assets/plugins/mongodb.mjs")),
    ("ffmpeg.mjs", include_str!("../assets/plugins/ffmpeg.mjs")),
    ("imagemagick.mjs", include_str!("../assets/plugins/imagemagick.mjs")),
    ("ytdlp.mjs", include_str!("../assets/plugins/ytdlp.mjs")),
    ("mqtt.mjs", include_str!("../assets/plugins/mqtt.mjs")),
    ("openai.mjs", include_str!("../assets/plugins/openai.mjs")),
    ("openrouter.mjs", include_str!("../assets/plugins/openrouter.mjs")),
    ("lmstudio.mjs", include_str!("../assets/plugins/lmstudio.mjs")),
    ("replicate.mjs", include_str!("../assets/plugins/replicate.mjs")),
    ("huggingface.mjs", include_str!("../assets/plugins/huggingface.mjs")),
    ("deepl.mjs", include_str!("../assets/plugins/deepl.mjs")),
    ("brave.mjs", include_str!("../assets/plugins/brave.mjs")),
    ("wolfram.mjs", include_str!("../assets/plugins/wolfram.mjs")),
    ("wikipedia.mjs", include_str!("../assets/plugins/wikipedia.mjs")),
    ("weather.mjs", include_str!("../assets/plugins/weather.mjs")),
    ("bitbucket.mjs", include_str!("../assets/plugins/bitbucket.mjs")),
    ("jenkins.mjs", include_str!("../assets/plugins/jenkins.mjs")),
    ("grafana.mjs", include_str!("../assets/plugins/grafana.mjs")),
    ("prometheus.mjs", include_str!("../assets/plugins/prometheus.mjs")),
    ("portainer.mjs", include_str!("../assets/plugins/portainer.mjs")),
    ("coolify.mjs", include_str!("../assets/plugins/coolify.mjs")),
    ("elasticsearch.mjs", include_str!("../assets/plugins/elasticsearch.mjs")),
    ("meilisearch.mjs", include_str!("../assets/plugins/meilisearch.mjs")),
    ("posthog.mjs", include_str!("../assets/plugins/posthog.mjs")),
    ("plausible.mjs", include_str!("../assets/plugins/plausible.mjs")),
    ("wordpress.mjs", include_str!("../assets/plugins/wordpress.mjs")),
    ("shopify.mjs", include_str!("../assets/plugins/shopify.mjs")),
    ("airtable.mjs", include_str!("../assets/plugins/airtable.mjs")),
    ("asana.mjs", include_str!("../assets/plugins/asana.mjs")),
    ("clickup.mjs", include_str!("../assets/plugins/clickup.mjs")),
    ("confluence.mjs", include_str!("../assets/plugins/confluence.mjs")),
    ("dropbox.mjs", include_str!("../assets/plugins/dropbox.mjs")),
    ("nextcloud.mjs", include_str!("../assets/plugins/nextcloud.mjs")),
    ("figma.mjs", include_str!("../assets/plugins/figma.mjs")),
    ("unsplash.mjs", include_str!("../assets/plugins/unsplash.mjs")),
    ("pexels.mjs", include_str!("../assets/plugins/pexels.mjs")),
    ("sketchfab.mjs", include_str!("../assets/plugins/sketchfab.mjs")),
    ("youtube.mjs", include_str!("../assets/plugins/youtube.mjs")),
    ("matrix.mjs", include_str!("../assets/plugins/matrix.mjs")),
    ("mattermost.mjs", include_str!("../assets/plugins/mattermost.mjs")),
    ("teams.mjs", include_str!("../assets/plugins/teams.mjs")),
    ("gotify.mjs", include_str!("../assets/plugins/gotify.mjs")),
    ("pushover.mjs", include_str!("../assets/plugins/pushover.mjs")),
    ("twilio.mjs", include_str!("../assets/plugins/twilio.mjs")),
    ("resend.mjs", include_str!("../assets/plugins/resend.mjs")),
    ("hue.mjs", include_str!("../assets/plugins/hue.mjs")),
    ("openhab.mjs", include_str!("../assets/plugins/openhab.mjs")),
    ("nodered.mjs", include_str!("../assets/plugins/nodered.mjs")),
    ("tasmota.mjs", include_str!("../assets/plugins/tasmota.mjs")),
    ("rcon.mjs", include_str!("../assets/plugins/rcon.mjs")),
    ("pterodactyl.mjs", include_str!("../assets/plugins/pterodactyl.mjs")),
    ("crafty.mjs", include_str!("../assets/plugins/crafty.mjs")),
    ("hangar.mjs", include_str!("../assets/plugins/hangar.mjs")),
    ("crowdin.mjs", include_str!("../assets/plugins/crowdin.mjs")),
    ("mcversions.mjs", include_str!("../assets/plugins/mcversions.mjs")),
    ("prism.mjs", include_str!("../assets/plugins/prism.mjs")),
    ("aseprite.mjs", include_str!("../assets/plugins/aseprite.mjs")),
    ("blockbench.mjs", include_str!("../assets/plugins/blockbench.mjs")),
    ("tiled.mjs", include_str!("../assets/plugins/tiled.mjs")),
    ("ldtk.mjs", include_str!("../assets/plugins/ldtk.mjs")),
    ("polyhaven.mjs", include_str!("../assets/plugins/polyhaven.mjs")),
    ("freesound.mjs", include_str!("../assets/plugins/freesound.mjs")),
    ("sox.mjs", include_str!("../assets/plugins/sox.mjs")),
    ("stability.mjs", include_str!("../assets/plugins/stability.mjs")),
    ("fal.mjs", include_str!("../assets/plugins/fal.mjs")),
    ("scenario.mjs", include_str!("../assets/plugins/scenario.mjs")),
    ("rodin.mjs", include_str!("../assets/plugins/rodin.mjs")),
    ("reddit.mjs", include_str!("../assets/plugins/reddit.mjs")),
    ("bluesky.mjs", include_str!("../assets/plugins/bluesky.mjs")),
    ("mastodon.mjs", include_str!("../assets/plugins/mastodon.mjs")),
    ("x.mjs", include_str!("../assets/plugins/x.mjs")),
    ("twitch.mjs", include_str!("../assets/plugins/twitch.mjs")),
    ("patreon.mjs", include_str!("../assets/plugins/patreon.mjs")),
    ("gumroad.mjs", include_str!("../assets/plugins/gumroad.mjs")),
    ("lemonsqueezy.mjs", include_str!("../assets/plugins/lemonsqueezy.mjs")),
    ("discordwebhook.mjs", include_str!("../assets/plugins/discordwebhook.mjs")),
    ("bitwarden.mjs", include_str!("../assets/plugins/bitwarden.mjs")),
    ("onepassword.mjs", include_str!("../assets/plugins/onepassword.mjs")),
    ("doppler.mjs", include_str!("../assets/plugins/doppler.mjs")),
    ("tailscale.mjs", include_str!("../assets/plugins/tailscale.mjs")),
    ("ngrok.mjs", include_str!("../assets/plugins/ngrok.mjs")),
    ("cloudflared.mjs", include_str!("../assets/plugins/cloudflared.mjs")),
    ("uptimekuma.mjs", include_str!("../assets/plugins/uptimekuma.mjs")),
    ("betterstack.mjs", include_str!("../assets/plugins/betterstack.mjs")),
    ("sonarcloud.mjs", include_str!("../assets/plugins/sonarcloud.mjs")),
    ("snyk.mjs", include_str!("../assets/plugins/snyk.mjs")),
    ("codecov.mjs", include_str!("../assets/plugins/codecov.mjs")),
    ("dockerhub.mjs", include_str!("../assets/plugins/dockerhub.mjs")),
    ("ghcr.mjs", include_str!("../assets/plugins/ghcr.mjs")),
    ("neon.mjs", include_str!("../assets/plugins/neon.mjs")),
    ("turso.mjs", include_str!("../assets/plugins/turso.mjs")),
    ("pocketbase.mjs", include_str!("../assets/plugins/pocketbase.mjs")),
    ("appwrite.mjs", include_str!("../assets/plugins/appwrite.mjs")),
    ("rclone.mjs", include_str!("../assets/plugins/rclone.mjs")),
    ("restic.mjs", include_str!("../assets/plugins/restic.mjs")),
    ("borg.mjs", include_str!("../assets/plugins/borg.mjs")),
    ("syncthing.mjs", include_str!("../assets/plugins/syncthing.mjs")),
    ("spotify.mjs", include_str!("../assets/plugins/spotify.mjs")),
    ("jellyfin.mjs", include_str!("../assets/plugins/jellyfin.mjs")),
    ("plex.mjs", include_str!("../assets/plugins/plex.mjs")),
    ("pihole.mjs", include_str!("../assets/plugins/pihole.mjs")),
    ("truenas.mjs", include_str!("../assets/plugins/truenas.mjs")),
    ("unraid.mjs", include_str!("../assets/plugins/unraid.mjs")),
    ("wol.mjs", include_str!("../assets/plugins/wol.mjs")),
    ("rss.mjs", include_str!("../assets/plugins/rss.mjs")),
    ("adb.mjs", include_str!("../assets/plugins/adb.mjs")),
    ("emulator.mjs", include_str!("../assets/plugins/emulator.mjs")),
    ("androidsdk.mjs", include_str!("../assets/plugins/androidsdk.mjs")),
    ("gradle.mjs", include_str!("../assets/plugins/gradle.mjs")),
    ("scrcpy.mjs", include_str!("../assets/plugins/scrcpy.mjs")),
    ("playconsole.mjs", include_str!("../assets/plugins/playconsole.mjs")),
    ("appstoreconnect.mjs", include_str!("../assets/plugins/appstoreconnect.mjs")),
    ("codemagic.mjs", include_str!("../assets/plugins/codemagic.mjs")),
    ("bitrise.mjs", include_str!("../assets/plugins/bitrise.mjs")),
    ("expo.mjs", include_str!("../assets/plugins/expo.mjs")),
    ("flutter.mjs", include_str!("../assets/plugins/flutter.mjs")),
    ("fastlane.mjs", include_str!("../assets/plugins/fastlane.mjs")),
    ("capacitor.mjs", include_str!("../assets/plugins/capacitor.mjs")),
    ("maestro.mjs", include_str!("../assets/plugins/maestro.mjs")),
    ("revenuecat.mjs", include_str!("../assets/plugins/revenuecat.mjs")),
    ("onesignal.mjs", include_str!("../assets/plugins/onesignal.mjs")),
    ("tauri.mjs", include_str!("../assets/plugins/tauri.mjs")),
    ("electron.mjs", include_str!("../assets/plugins/electron.mjs")),
    ("flatpak.mjs", include_str!("../assets/plugins/flatpak.mjs")),
    ("snapcraft.mjs", include_str!("../assets/plugins/snapcraft.mjs")),
    ("appimage.mjs", include_str!("../assets/plugins/appimage.mjs")),
    ("lighthouse.mjs", include_str!("../assets/plugins/lighthouse.mjs")),
    ("algolia.mjs", include_str!("../assets/plugins/algolia.mjs")),
    ("chromewebstore.mjs", include_str!("../assets/plugins/chromewebstore.mjs")),
    ("firefoxamo.mjs", include_str!("../assets/plugins/firefoxamo.mjs")),
    ("cratesio.mjs", include_str!("../assets/plugins/cratesio.mjs")),
    ("pypi.mjs", include_str!("../assets/plugins/pypi.mjs")),
    ("nuget.mjs", include_str!("../assets/plugins/nuget.mjs")),
    ("jupyter.mjs", include_str!("../assets/plugins/jupyter.mjs")),
    ("kaggle.mjs", include_str!("../assets/plugins/kaggle.mjs")),
    ("wandb.mjs", include_str!("../assets/plugins/wandb.mjs")),
    ("mintlify.mjs", include_str!("../assets/plugins/mintlify.mjs")),
    ("docusaurus.mjs", include_str!("../assets/plugins/docusaurus.mjs")),
    ("gitbook.mjs", include_str!("../assets/plugins/gitbook.mjs")),
];

fn scripts_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("pixel-code/plugins")
}

fn node() -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    if let Some(h) = dirs::home_dir() {
        dirs.extend([h.join(".local/bin"), h.join(".bun/bin"), h.join(".volta/bin")]);
    }
    ["node", "bun"].iter().flat_map(|n| dirs.iter().map(move |d| d.join(n))).find(|p| p.is_file())
}

/// Prüft ein Skript-Plugin (`<script> status`) und schreibt das Ergebnis in die Registry.
pub fn check(d: &Def) {
    run_check(d, "status");
}

/// Verbindet ein Plugin (`<script> connect`, startet z.B. SearXNG). Gibt zurück, ob es klappt.
pub fn connect(d: &Def) -> bool {
    run_check(d, "connect");
    let ok = load_registry().plugins.iter().any(|p| p.id == d.id && p.connected);
    set_enabled(d.id, ok);
    ok
}

/// Trennt ein Plugin: aus, und geheime Felder (API-Keys, Tokens) werden gelöscht.
pub fn disconnect(d: &Def) {
    let mut reg = load_registry();
    if let Some(p) = reg.plugins.iter_mut().find(|p| p.id == d.id) {
        p.enabled = false;
        for f in d.fields.iter().filter(|f| f.secret) {
            p.settings.remove(f.key);
        }
        if d.id == "telegram" {
            p.settings.remove("chat_id");
            p.settings.remove("paired_user");
        }
        // Gespeicherte Browser-Anmeldungen (Google, Microsoft) löschen
        let _ = std::fs::remove_file(config_dir().join("tokens").join(format!("{}.json", d.id)));
        save_registry(&reg);
    }
}

fn run_check(d: &Def, action: &str) {
    let existing = load_registry().plugins.into_iter().find(|p| p.id == d.id);
    let settings = existing.as_ref().map(|p| p.settings.clone()).unwrap_or_default();
    let script = scripts_dir().join(format!("{}.mjs", d.id));
    // Ausgeschaltete Plugins nicht ausführen (manche laden sonst Pakete über npx), nur anmelden
    if action == "status" && !existing.as_ref().is_some_and(|p| p.enabled) {
        if existing.is_none() {
            update_entry(Entry {
                id: d.id.into(),
                name: d.name.into(),
                description: d.description.into(),
                enabled: false,
                connected: false,
                account: None,
                command: node().map_or("node".into(), |n| n.display().to_string()),
                prefix: vec![script.display().to_string()],
                usage: format!("Commands: {}. Run 'help' for details.", d.usage),
                settings: BTreeMap::new(),
                status: String::new(),
            });
        }
        return;
    }
    let (ok, status, command) = match node() {
        None => (false, "Node.js is required for this plugin".to_string(), "node".to_string()),
        Some(node) => {
            let out = Command::new(&node)
                .arg(&script)
                .arg(action)
                .env("PC_SETTINGS", serde_json::to_string(&settings).unwrap_or_default())
                .stdin(Stdio::null())
                .output();
            let (ok, text) = match out {
                Ok(o) => (o.status.success(), String::from_utf8_lossy(&o.stdout).trim().to_string()),
                Err(e) => (false, e.to_string()),
            };
            (ok, text.lines().next().unwrap_or("").to_string(), node.display().to_string())
        }
    };
    update_entry(Entry {
        id: d.id.into(),
        name: d.name.into(),
        description: d.description.into(),
        // Neue Plugins sind aus, bis der Nutzer sie einschaltet
        enabled: false,
        connected: ok,
        account: None,
        command,
        prefix: vec![script.display().to_string()],
        usage: format!("Commands: {}. Run 'help' for details.", d.usage),
        settings: BTreeMap::new(),
        status,
    });
}

/// Installiert die Plugin-Skripte und prüft alle Plugins im Hintergrund.
pub fn refresh_all(ctx: eframe::egui::Context) {
    std::thread::spawn(move || {
        let dir = scripts_dir();
        for (name, content) in SCRIPTS {
            write_if_changed(&dir.join(name), content);
        }
        for d in DEFS {
            check(d);
            ctx.request_repaint();
        }
    });
}

pub fn set_setting(id: &str, key: &str, value: &str) {
    let mut reg = load_registry();
    if let Some(p) = reg.plugins.iter_mut().find(|p| p.id == id) {
        if value.is_empty() {
            p.settings.remove(key);
        } else {
            p.settings.insert(key.into(), value.into());
        }
        save_registry(&reg);
    }
}

pub fn setting(id: &str, key: &str) -> String {
    load_registry().plugins.into_iter().find(|p| p.id == id).and_then(|p| p.settings.get(key).cloned()).unwrap_or_default()
}

// ---------------------------------------------------------------- Agent-Plugin "Pixel Code"

/// Gemeinsame Logik für OpenCode (JS) und omp (TS); läuft in beiden Fällen unter Bun.
const CORE: &str = r#"
import { readFileSync, writeFileSync, mkdirSync, renameSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { homedir } from "node:os";
import { join } from "node:path";

const REGISTRY = join(process.env.XDG_CONFIG_HOME || join(homedir(), ".config"), "pixel-code", "plugins.json");

function pcLoad() {
  try { return JSON.parse(readFileSync(REGISTRY, "utf8")); } catch { return { version: 1, plugins: [] }; }
}

function pcList() {
  const reg = pcLoad();
  if (!reg.plugins.length) return "No Pixel Code plugins are installed. The user can add them in Pixel Code > Settings > Plugins.";
  return reg.plugins.map((p) => [
    `## ${p.name} (id: ${p.id})`,
    `status: ${p.enabled ? "enabled" : "disabled"}, ${p.connected ? "ready" : "not ready"}${p.status ? " - " + p.status : ""}`,
    p.description,
    `usage: ${p.usage}`,
  ].join("\n")).join("\n\n");
}

// Zerlegt "pr create --title 'a b'" in Argumente (einfache Shell-Quotes).
function pcSplit(s) {
  const out = []; let cur = "", q = null, has = false;
  for (const ch of String(s)) {
    if (q) { if (ch === q) q = null; else cur += ch; }
    else if (ch === "'" || ch === '"') { q = ch; has = true; }
    else if (/\s/.test(ch)) { if (cur || has) out.push(cur); cur = ""; has = false; }
    else cur += ch;
  }
  if (cur || has) out.push(cur);
  return out;
}

function pcRun(id, args, cwd) {
  if (!Array.isArray(args)) args = pcSplit(args ?? "");
  const p = pcLoad().plugins.find((x) => x.id === id);
  if (!p) return `Unknown plugin "${id}". Available: ${pcLoad().plugins.map((x) => x.id).join(", ") || "none"}`;
  if (!p.enabled) return `Plugin "${id}" is disabled. Enable it with pixelcode_plugin_set first (ask the user).`;
  const r = spawnSync(p.command, [...(p.prefix || []), ...(args || [])], {
    cwd: cwd || process.cwd(), encoding: "utf8", maxBuffer: 16 * 1024 * 1024, timeout: 30 * 60 * 1000,
    env: { ...process.env, GH_PROMPT_DISABLED: "1", NO_COLOR: "1", PC_SETTINGS: JSON.stringify(p.settings || {}) },
  });
  if (r.error) return `Failed to run ${p.command}: ${r.error.message}`;
  const out = [r.stdout, r.stderr].filter(Boolean).join("\n").trim();
  return `exit code ${r.status}\n${out}`.slice(0, 60000);
}

function pcSet(id, enabled) {
  const reg = pcLoad();
  const p = reg.plugins.find((x) => x.id === id);
  if (!p) return `Unknown plugin "${id}".`;
  p.enabled = !!enabled;
  writeFileSync(REGISTRY, JSON.stringify(reg, null, 2));
  return `${p.name} is now ${p.enabled ? "enabled" : "disabled"}.`;
}

// Meldet Pixel Code den Zustand des Agents in diesem Terminal (working, question, permission, done, error, idle).
function pcStatus(state) {
  const pane = process.env.PIXEL_CODE_PANE;
  if (!pane) return;
  const dir = process.env.PIXEL_CODE_STATUS_DIR || "/tmp/pixel-code-status";
  try {
    mkdirSync(dir, { recursive: true });
    const tmp = join(dir, `.${pane}.${process.pid}`);
    writeFileSync(tmp, state + "\n");
    renameSync(tmp, join(dir, pane));
  } catch {}
}

// OpenCode-Events -> Zustand
function pcEvent(event) {
  const t = event?.type || "", p = event?.properties || {};
  if (t === "session.status") {
    const s = p.status?.type;
    if (s === "busy" || s === "retry") pcStatus("working");
    else if (s === "idle") pcStatus("done");
  } else if (t === "session.idle") pcStatus("done");
  else if (t === "session.error") pcStatus("error");
  else if (t === "permission.updated" || t === "permission.asked") pcStatus("permission");
  else if (t === "question.asked") pcStatus("question");
  else if (t === "permission.replied" || t === "question.replied" || t === "question.rejected") pcStatus("working");
}

const DESC = {
  list: "List the Pixel Code plugins (GitHub, Blender, Unity, Unreal Engine, Godot, web search, Leonardo.ai, Telegram, ...) with their status and usage. Call this before using pixelcode_plugin_run.",
  run: "Run a Pixel Code plugin command in the project directory, e.g. plugin 'github' with 'pr list', plugin 'websearch' with 'search rust egui', plugin 'blender' with 'exec <python>'. Run a plugin with 'help' to see all of its commands.",
  set: "Enable or disable a Pixel Code plugin. Only do this when the user asks for it.",
};
"#;

fn opencode_plugin() -> String {
    format!(
        r#"// @pixel-code-managed v{v} - generated by Pixel Code, will be overwritten on update.
{CORE}
// OpenCode 1.x: Hooks-Objekt mit Tools.
async function legacy() {{
  const {{ tool }} = await import("@opencode-ai/plugin");
  return {{
    event: async ({{ event }}) => pcEvent(event),
    "tool.execute.before": async (input) => {{ if (input?.tool === "question") pcStatus("question"); }},
    tool: {{
      pixelcode_plugins: tool({{ description: DESC.list, args: {{}}, async execute() {{ return pcList(); }} }}),
      pixelcode_plugin_run: tool({{
        description: DESC.run,
        args: {{
          plugin: tool.schema.string().describe("Plugin id, e.g. github"),
          args: tool.schema.array(tool.schema.string()).describe("Arguments for the plugin command"),
        }},
        async execute(a, ctx) {{ return pcRun(a.plugin, a.args, ctx.directory); }},
      }}),
      pixelcode_plugin_set: tool({{
        description: DESC.set,
        args: {{ plugin: tool.schema.string(), enabled: tool.schema.boolean() }},
        async execute(a) {{ return pcSet(a.plugin, a.enabled); }},
      }}),
    }},
  }};
}}

// OpenCode 2.x: Tools über ctx.tool.transform (Namespace "pixelcode" -> pixelcode_plugins, ...).
// Die Parameter-Schemas werden aus OpenCodes eigenen Tools abgeleitet: Schemas aus einer anderen
// `effect`-Version erkennt OpenCodes Validierung nicht ("Invalid arguments ... Expected object").
async function setup(ctx) {{
  // Zustand für die Pixel-Code-Sidebar
  for (const sub of [ctx?.event?.subscribe, ctx?.bus?.subscribe]) {{
    try {{ if (typeof sub === "function") {{ sub((e) => pcEvent(e?.payload ?? e)); break; }} }} catch {{}}
  }}
  try {{
    const tools = await ctx.tool.list();
    const structs = tools.map((t) => t.input).filter((i) => i && typeof i.mapFields === "function" && i.fields);
    const field = (tag) => {{
      for (const id of ["edit", "grep", "shell", "read"]) {{
        const f = tools.find((t) => t.id === id)?.input?.fields;
        const hit = f && Object.values(f).find((v) => v?.ast?._tag === tag);
        if (hit) return hit;
      }}
      for (const s of structs) {{
        const hit = Object.values(s.fields).find((v) => v?.ast?._tag === tag);
        if (hit) return hit;
      }}
    }};
    const base = structs[0], Str = field("String"), Bool = field("Boolean");
    if (!base || !Str || !Bool) throw new Error("no schema building blocks found");
    const struct = (fields) => base.mapFields(() => fields);
    const done = (text) => ({{ output: text, content: text }});
    const dir = ctx?.location?.directory;
    await ctx.tool.transform((draft) => {{
      draft.add({{
        name: "plugins", description: DESC.list, input: struct({{}}), output: Str,
        options: {{ namespace: "pixelcode" }}, execute: async () => done(pcList()),
      }});
      draft.add({{
        name: "plugin_run", description: DESC.run + " Pass the arguments as one string, e.g. 'pr list --state open'.",
        input: struct({{ plugin: Str, args: Str }}), output: Str,
        options: {{ namespace: "pixelcode" }}, execute: async (a) => done(pcRun(a.plugin, a.args, dir)),
      }});
      draft.add({{
        name: "plugin_set", description: DESC.set,
        input: struct({{ plugin: Str, enabled: Bool }}), output: Str,
        options: {{ namespace: "pixelcode" }}, execute: async (a) => done(pcSet(a.plugin, a.enabled)),
      }});
    }});
  }} catch (e) {{
    console.warn("[pixel-code] could not register tools:", e);
  }}
  return async () => {{}};
}}

export default {{ id: "pixel-code", server: legacy, setup }};
"#,
        v = env!("CARGO_PKG_VERSION")
    )
}

fn package_json(desc: &str) -> String {
    format!(
        r#"{{
  "name": "pixel-code",
  "version": "{}",
  "description": "{desc}",
  "type": "module",
  "main": "index",
  "keywords": ["omp-plugin", "pi-package"],
  "omp": {{ "extensions": ["./index.ts"] }},
  "pi": {{ "extensions": ["./index.ts"] }}
}}
"#,
        env!("CARGO_PKG_VERSION")
    )
}

fn omp_extension() -> String {
    format!(
        r#"// @pixel-code-managed v{v} - generated by Pixel Code, will be overwritten on update.
{CORE}
const text = (t: string) => ({{ content: [{{ type: "text", text: t }}], details: {{}} }});

export default function (pi: any): void {{
  // Zustand für die Pixel-Code-Sidebar
  const on = (ev: string, fn: (e: any) => void) => {{ try {{ pi.on?.(ev, fn); }} catch {{}} }};
  on("agent_start", () => pcStatus("working"));
  on("turn_start", () => pcStatus("working"));
  on("agent_end", () => pcStatus("done"));
  on("tool_call", (e: any) => pcStatus(/^(ask|question|ask_user)/.test(e?.toolName || e?.name || "") ? "question" : "working"));
  on("tool_result", () => pcStatus("working"));
  on("session_shutdown", () => pcStatus("idle"));
  pi.registerTool({{
    name: "pixelcode_plugins",
    label: "Pixel Code plugins",
    description: DESC.list,
    parameters: {{ type: "object", properties: {{}} }},
    async execute() {{ return text(pcList()); }},
  }});
  pi.registerTool({{
    name: "pixelcode_plugin_run",
    label: "Pixel Code plugin",
    description: DESC.run,
    parameters: {{
      type: "object",
      properties: {{
        plugin: {{ type: "string", description: "Plugin id, e.g. github" }},
        args: {{ type: "array", items: {{ type: "string" }}, description: "Arguments for the plugin command" }},
      }},
      required: ["plugin", "args"],
    }},
    async execute(_id: string, p: any, _signal: any, _update: any, ctx: any) {{ return text(pcRun(p.plugin, p.args, ctx?.cwd)); }},
  }});
  pi.registerTool({{
    name: "pixelcode_plugin_set",
    label: "Pixel Code plugin switch",
    description: DESC.set,
    parameters: {{
      type: "object",
      properties: {{ plugin: {{ type: "string" }}, enabled: {{ type: "boolean" }} }},
      required: ["plugin", "enabled"],
    }},
    async execute(_id: string, p: any) {{ return text(pcSet(p.plugin, p.enabled)); }},
  }});
}}
"#,
        v = env!("CARGO_PKG_VERSION")
    )
}

fn write_if_changed(path: &std::path::Path, content: &str) -> bool {
    if std::fs::read_to_string(path).is_ok_and(|c| c == content) {
        return false;
    }
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    std::fs::write(path, content).is_ok()
}

// ---------------------------------------------------------------- MCP-Server für alle anderen CLIs

/// Kleiner MCP-Server (stdio, JSON-RPC) mit denselben Tools wie die OpenCode/omp-Plugins.
fn mcp_server() -> String {
    format!(
        r#"// @pixel-code-managed v{v} - Pixel Code MCP server, generated by Pixel Code, will be overwritten on update.
{CORE}
import {{ createInterface }} from "node:readline";

const TOOLS = [
  {{ name: "pixelcode_plugins", description: DESC.list, inputSchema: {{ type: "object", properties: {{}} }} }},
  {{
    name: "pixelcode_plugin_run",
    description: DESC.run + " Pass the arguments as one string, e.g. 'pr list --state open'.",
    inputSchema: {{
      type: "object",
      properties: {{
        plugin: {{ type: "string", description: "Plugin id, e.g. github" }},
        args: {{ type: "string", description: "Arguments for the plugin command" }},
        cwd: {{ type: "string", description: "Absolute project directory to run in (default: the current project)" }},
      }},
      required: ["plugin", "args"],
    }},
  }},
  {{
    name: "pixelcode_plugin_set",
    description: DESC.set,
    inputSchema: {{
      type: "object",
      properties: {{ plugin: {{ type: "string" }}, enabled: {{ type: "boolean" }} }},
      required: ["plugin", "enabled"],
    }},
  }},
];

const send = (m) => process.stdout.write(JSON.stringify({{ jsonrpc: "2.0", ...m }}) + "\n");

// Projektordner: vom Client über MCP "roots", sonst das Startverzeichnis
let projectDir = process.env.PWD && process.env.PWD !== process.env.KIMI_PLUGIN_ROOT ? process.env.PWD : process.cwd();
let clientRoots = false;
const ROOTS_ID = "pixel-code-roots";
const askRoots = () => clientRoots && send({{ id: ROOTS_ID, method: "roots/list" }});

function call(name, a) {{
  if (name === "pixelcode_plugins") return pcList();
  if (name === "pixelcode_plugin_run") return pcRun(a.plugin, a.args, a.cwd || projectDir);
  if (name === "pixelcode_plugin_set") return pcSet(a.plugin, a.enabled);
  throw new Error(`Unknown tool ${{name}}`);
}}

// Direkter Aufruf ohne MCP (für CLIs mit Skills statt MCP, z.B. Letta Code):
// node pixel-code-mcp.mjs list | run <plugin> <args...>
const [cliCmd, ...cliArgs] = process.argv.slice(2);
if (cliCmd === "list") {{
  console.log(pcList());
  process.exit(0);
}}
if (cliCmd === "run") {{
  console.log(String(pcRun(cliArgs[0], cliArgs.slice(1), process.cwd())));
  process.exit(0);
}}

createInterface({{ input: process.stdin }}).on("line", (line) => {{
  let m;
  try {{ m = JSON.parse(line); }} catch {{ return; }}
  const {{ id, method, params }} = m;
  if (id === ROOTS_ID) {{
    const uri = m.result?.roots?.[0]?.uri;
    if (uri?.startsWith("file://")) projectDir = decodeURIComponent(new URL(uri).pathname);
    return;
  }}
  if (method === "notifications/initialized" || method === "notifications/roots/list_changed") {{
    askRoots();
    return;
  }}
  if (method === "initialize") {{
    clientRoots = !!params?.capabilities?.roots;
    send({{ id, result: {{ protocolVersion: params?.protocolVersion || "2025-06-18", capabilities: {{ tools: {{}} }}, serverInfo: {{ name: "pixel-code", version: "{v}" }} }} }});
  }} else if (method === "tools/list") {{
    send({{ id, result: {{ tools: TOOLS }} }});
  }} else if (method === "tools/call") {{
    try {{
      send({{ id, result: {{ content: [{{ type: "text", text: String(call(params.name, params.arguments || {{}})) }}] }} }});
    }} catch (e) {{
      send({{ id, result: {{ content: [{{ type: "text", text: String(e.message || e) }}], isError: true }} }});
    }}
  }} else if (method === "ping") {{
    send({{ id, result: {{}} }});
  }} else if (id !== undefined) {{
    send({{ id, error: {{ code: -32601, message: `Method not found: ${{method}}` }} }});
  }}
}});
"#,
        v = env!("CARGO_PKG_VERSION")
    )
}

fn mcp_path() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("pixel-code/mcp/pixel-code-mcp.mjs")
}

/// Trägt den Server in einer JSON-Konfiguration unter `key.pixel-code` ein (andere Einträge bleiben).
fn merge_json(path: &std::path::Path, key: &str, server: serde_json::Value) {
    let mut root: serde_json::Value = match std::fs::read_to_string(path) {
        Ok(s) if !s.trim().is_empty() => match serde_json::from_str(&s) {
            Ok(v) => v,
            Err(_) => return, // Kaputte/kommentierte Datei nicht überschreiben
        },
        _ => serde_json::json!({}),
    };
    let Some(obj) = root.as_object_mut() else { return };
    let servers = obj.entry(key).or_insert_with(|| serde_json::json!({}));
    let Some(servers) = servers.as_object_mut() else { return };
    if servers.get("pixel-code") == Some(&server) {
        return;
    }
    servers.insert("pixel-code".into(), server);
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if let Ok(s) = serde_json::to_string_pretty(&root) {
        let _ = std::fs::write(path, s);
    }
}

/// Kimi Code: als echtes Kimi-Plugin (erscheint unter /plugins > Installed).
fn install_kimi_plugin(home: &std::path::Path) {
    let kimi_home = std::env::var_os("KIMI_CODE_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".kimi-code"));
    let root = kimi_home.join("plugins/managed/pixel-code");
    let manifest = serde_json::json!({
        "name": "pixel-code",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Use your Pixel Code plugins (GitHub, web search, Blender, Modrinth, ...) from Kimi Code.",
        "interface": {
            "displayName": "Pixel Code",
            "shortDescription": "Your Pixel Code plugins as tools",
            "developerName": "Pixel Code"
        },
        "mcpServers": { "pixel-code": { "command": "node", "args": ["./pixel-code-mcp.mjs"] } }
    });
    write_if_changed(&root.join("pixel-code-mcp.mjs"), &mcp_server());
    write_if_changed(&root.join("kimi.plugin.json"), &serde_json::to_string_pretty(&manifest).unwrap_or_default());

    let installed_path = kimi_home.join("plugins/installed.json");
    let mut installed: serde_json::Value = match std::fs::read_to_string(&installed_path) {
        Ok(s) => match serde_json::from_str(&s) {
            Ok(v) => v,
            Err(_) => return,
        },
        Err(_) => serde_json::json!({ "version": 1, "plugins": [] }),
    };
    let Some(list) = installed["plugins"].as_array_mut() else { return };
    if !list.iter().any(|p| p["id"] == "pixel-code") {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let iso = Command::new("date").args(["-u", "-d", &format!("@{now}"), "+%Y-%m-%dT%H:%M:%S.000Z"]).output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
        let root_s = std::fs::canonicalize(&root).unwrap_or(root.clone()).display().to_string();
        list.push(serde_json::json!({
            "id": "pixel-code", "root": root_s, "source": "local-path", "enabled": true,
            "installedAt": iso, "updatedAt": iso, "originalSource": root_s
        }));
        if let Ok(s) = serde_json::to_string_pretty(&installed) {
            let _ = std::fs::write(&installed_path, s);
        }
    }
    // Früheren Eintrag in mcp.json entfernen, sonst gäbe es die Tools doppelt
    let mcp = kimi_home.join("mcp.json");
    if let Ok(mut v) = std::fs::read_to_string(&mcp).map(|s| serde_json::from_str::<serde_json::Value>(&s).unwrap_or_default()) {
        if v["mcpServers"].as_object_mut().and_then(|o| o.remove("pixel-code")).is_some() {
            let _ = std::fs::write(&mcp, serde_json::to_string_pretty(&v).unwrap_or_default());
        }
    }
}

/// Hermes Agent: `mcp_servers` in `~/.hermes/config.yaml` (ohne YAML-Bibliothek, als Textzeile).
fn merge_hermes(path: &std::path::Path, node: &str, script: &str) {
    let q = |s: &str| serde_json::to_string(s).unwrap_or_default();
    let entry = format!("pixel-code: {{command: {}, args: [{}]}}", q(node), q(script));
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    // Vorhandenen Eintrag ersetzen (Pfad kann sich geändert haben)
    if let Some(i) = lines.iter().position(|l| l.trim_start().starts_with("pixel-code:")) {
        let indent = lines[i].len() - lines[i].trim_start().len();
        let new = format!("{}{entry}", " ".repeat(indent));
        if lines[i] == new {
            return;
        }
        lines[i] = new;
    } else if let Some(i) = lines.iter().position(|l| l.starts_with("mcp_servers:")) {
        let rest = lines[i]["mcp_servers:".len()..].trim();
        if rest == "{}" {
            lines[i] = "mcp_servers:".into();
        } else if !rest.is_empty() && !rest.starts_with('#') {
            return; // Inline-Schreibweise nicht anfassen
        }
        let indent = lines[i + 1..].iter().find(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
            .map(|l| l.len() - l.trim_start().len()).filter(|n| *n > 0).unwrap_or(2);
        lines.insert(i + 1, format!("{}{entry}", " ".repeat(indent)));
    } else {
        lines.push("mcp_servers:".into());
        lines.push(format!("  {entry}"));
    }
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(path, lines.join("\n") + "\n");
}

/// Strix: Liste von Servern in `~/.strix/mcp-servers.json`.
fn merge_strix(path: &std::path::Path, node: &str, script: &str) {
    let mut list: Vec<serde_json::Value> = match std::fs::read_to_string(path) {
        Ok(s) if !s.trim().is_empty() => match serde_json::from_str(&s) {
            Ok(v) => v,
            Err(_) => return,
        },
        _ => Vec::new(),
    };
    let entry = serde_json::json!({
        "name": "pixel_code", "transport": "stdio", "command": node, "args": [script],
        "notes": "Pixel Code plugins (GitHub, web search, ...). Call pixelcode_plugins first to see what is available."
    });
    if list.contains(&entry) {
        return;
    }
    list.retain(|v| v["name"] != "pixel_code");
    list.push(entry);
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if let Ok(s) = serde_json::to_string_pretty(&list) {
        let _ = std::fs::write(path, s);
    }
}

/// Letta Code: MCP-Server laufen dort auf dem Letta-Server, daher als lokaler Skill,
/// der den Pixel-Code-MCP-Server direkt über die Shell aufruft.
fn install_letta_skill(home: &std::path::Path, node: &str, script: &str) {
    let cmd = format!("'{}' '{}'", node.replace('\'', "'\\''"), script.replace('\'', "'\\''"));
    let skill = format!(
        r#"---
name: pixel-code
description: Use the user's Pixel Code plugins (GitHub, web search, Blender, Docker, Modrinth, ...). Use whenever a task involves one of these services.
---

<!-- @pixel-code-managed v{v} - generated by Pixel Code, will be overwritten on update. -->

# Pixel Code plugins

List the installed plugins, their status and usage:

```sh
{cmd} list
```

Run a plugin command in the current project (one shell argument per plugin argument):

```sh
{cmd} run <plugin> <args...>
# e.g. {cmd} run github pr list --state open
```

The user enables plugins and connects accounts in Pixel Code > Settings > Plugins.
"#,
        v = env!("CARGO_PKG_VERSION")
    );
    write_if_changed(&home.join(".letta/skills/pixel-code/SKILL.md"), &skill);
}

/// Registriert den Pixel-Code-MCP-Server bei einer CLI (über deren `mcp add` oder Konfigurationsdatei).
fn install_mcp(agent: &str, bin: &str) {
    let Some(home) = dirs::home_dir() else { return };
    install_status_hooks(agent, &home);
    let Some(node) = node() else { return };
    let script = mcp_path();
    write_if_changed(&script, &mcp_server());
    let (node_s, script_s) = (node.display().to_string(), script.display().to_string());
    let stdio = serde_json::json!({ "command": node_s, "args": [script_s] });
    let run = |args: &[&str]| {
        Command::new(bin).args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
    };
    match agent {
        "claude" => {
            if !run(&["mcp", "get", "pixel-code"]) {
                run(&["mcp", "add", "-s", "user", "pixel-code", "--", &node_s, &script_s]);
            }
        }
        "codex" => {
            if !run(&["mcp", "get", "pixel-code"]) {
                run(&["mcp", "add", "pixel-code", "--", &node_s, &script_s]);
            }
        }
        // Gemini CLI / Antigravity: "add" aktualisiert auch
        "gemini" => {
            run(&["mcp", "add", "pixel-code", &node_s, &script_s]);
        }
        "kimi" => install_kimi_plugin(&home),
        "cursor" => merge_json(&home.join(".cursor/mcp.json"), "mcpServers", stdio),
        "qwen" => merge_json(&home.join(".qwen/settings.json"), "mcpServers", stdio),
        "copilot" => merge_json(
            &home.join(".copilot/mcp-config.json"),
            "mcpServers",
            serde_json::json!({ "type": "local", "command": node_s, "args": [script_s], "tools": ["*"] }),
        ),
        "amp" => merge_json(&dirs::config_dir().unwrap_or(home.join(".config")).join("amp/settings.json"), "amp.mcpServers", stdio),
        "crush" => merge_json(
            &dirs::config_dir().unwrap_or(home.join(".config")).join("crush/crush.json"),
            "mcp",
            serde_json::json!({ "type": "stdio", "command": node_s, "args": [script_s] }),
        ),
        // Kilo CLI basiert auf OpenCode: "local"-Server mit Befehl als Liste
        "kilo" => merge_json(
            &dirs::config_dir().unwrap_or(home.join(".config")).join("kilo/kilo.json"),
            "mcp",
            serde_json::json!({ "type": "local", "command": [node_s, script_s], "enabled": true }),
        ),
        "iflow" => merge_json(&home.join(".iflow/settings.json"), "mcpServers", stdio),
        "junie" => merge_json(&home.join(".junie/mcp/mcp.json"), "mcpServers", stdio),
        "openhands" => merge_json(&home.join(".openhands/mcp.json"), "mcpServers", stdio),
        "proto" => merge_json(&home.join(".proto/settings.json"), "mcpServers", stdio),
        "cline" => merge_json(&home.join(".cline/data/settings/cline_mcp_settings.json"), "mcpServers", stdio),
        "openclaw" => {
            run(&["mcp", "add", "pixel-code", "--command", &node_s, "--arg", &script_s]);
        }
        "hermes" => merge_hermes(&home.join(".hermes/config.yaml"), &node_s, &script_s),
        "strix" => merge_strix(&home.join(".strix/mcp-servers.json"), &node_s, &script_s),
        "letta" => install_letta_skill(&home, &node_s, &script_s),
        _ => {}
    }
}

// ---------------------------------------------------------------- Agent-Status über Hooks

/// Ordner, in den Hooks und Plugins den Zustand je Terminal schreiben (Datei `<pane id>`).
pub fn status_dir() -> PathBuf {
    dirs::runtime_dir().or_else(dirs::cache_dir).unwrap_or_else(|| PathBuf::from("/tmp")).join("pixel-code/status")
}

fn hook_path() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("pixel-code/bin/pixel-code-hook")
}

/// Wird von den Hooks der CLIs aufgerufen: `pixel-code-hook <state>`, Hook-JSON auf stdin.
const HOOK: &str = r#"#!/bin/sh
# @pixel-code-managed - reports the agent state of a Pixel Code terminal.
input=$(cat 2>/dev/null)
[ -n "$PIXEL_CODE_PANE" ] || exit 0
state="$1"
case "$state" in
  tool)
    case "$input" in
      *'"tool_name":"AskUserQuestion"'*|*'"tool_name": "AskUserQuestion"'*|*'"request_user_input"'*|*'"ask_user"'*) state=question ;;
      *) state=working ;;
    esac ;;
  notify)
    case "$input" in
      *permission_prompt*|*ToolPermission*|*"needs your permission"*) state=permission ;;
      *elicitation*) state=question ;;
      *) exit 0 ;;
    esac ;;
esac
dir="${PIXEL_CODE_STATUS_DIR:-/tmp/pixel-code-status}"
mkdir -p "$dir" 2>/dev/null
printf '%s\n' "$state" > "$dir/.$PIXEL_CODE_PANE.$$" && mv -f "$dir/.$PIXEL_CODE_PANE.$$" "$dir/$PIXEL_CODE_PANE"
exit 0
"#;

fn install_hook_script() -> String {
    let path = hook_path();
    if write_if_changed(&path, HOOK) {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755));
    }
    let p = path.display().to_string().replace('\'', "'\\''");
    p
}

/// Trägt die Status-Hooks in eine Hook-Konfiguration ein. `nested`: Claude/Codex-Format
/// (`[{ hooks: [..] }]`), sonst das flache Format von Gemini/Antigravity.
fn merge_hooks(path: &std::path::Path, events: &[(&str, &str)], nested: bool) {
    let script = install_hook_script();
    let mut root: serde_json::Value = match std::fs::read_to_string(path) {
        Ok(s) if !s.trim().is_empty() => match serde_json::from_str(&s) {
            Ok(v) => v,
            Err(_) => return,
        },
        _ => serde_json::json!({}),
    };
    let before = root.clone();
    let Some(obj) = root.as_object_mut() else { return };
    let hooks = obj.entry("hooks").or_insert_with(|| serde_json::json!({}));
    let Some(hooks) = hooks.as_object_mut() else { return };
    let ours = |v: &serde_json::Value| v.to_string().contains("pixel-code-hook");
    for (event, state) in events {
        let command = format!("test -x '{script}' && '{script}' {state} || cat >/dev/null");
        let entry = if nested {
            serde_json::json!({ "hooks": [{ "type": "command", "command": command, "timeout": 10 }] })
        } else {
            serde_json::json!({ "type": "command", "command": command, "timeout": 10000 })
        };
        let list = hooks.entry(*event).or_insert_with(|| serde_json::json!([]));
        let Some(list) = list.as_array_mut() else { continue };
        if list.iter().any(|v| *v == entry) {
            continue;
        }
        list.retain(|v| !ours(v));
        list.push(entry);
    }
    if root == before {
        return;
    }
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if let Ok(s) = serde_json::to_string_pretty(&root) {
        let _ = std::fs::write(path, s);
    }
}

fn install_status_hooks(agent: &str, home: &std::path::Path) {
    const CLAUDE: &[(&str, &str)] = &[
        ("SessionStart", "idle"),
        ("UserPromptSubmit", "working"),
        ("PreToolUse", "tool"),
        ("PostToolUse", "working"),
        ("PermissionRequest", "permission"),
        ("Notification", "notify"),
        ("Stop", "done"),
        ("StopFailure", "error"),
        ("SessionEnd", "idle"),
    ];
    const CODEX: &[(&str, &str)] = &[
        ("SessionStart", "idle"),
        ("UserPromptSubmit", "working"),
        ("PreToolUse", "tool"),
        ("PostToolUse", "working"),
        ("PermissionRequest", "permission"),
        ("Stop", "done"),
    ];
    const IFLOW: &[(&str, &str)] = &[
        ("SessionStart", "idle"),
        ("UserPromptSubmit", "working"),
        ("PreToolUse", "tool"),
        ("PostToolUse", "working"),
        ("Notification", "notify"),
        ("Stop", "done"),
        ("SessionEnd", "idle"),
    ];
    const GEMINI: &[(&str, &str)] = &[
        ("SessionStart", "idle"),
        ("BeforeAgent", "working"),
        ("BeforeTool", "tool"),
        ("AfterTool", "working"),
        ("Notification", "notify"),
        ("AfterAgent", "done"),
        ("SessionEnd", "idle"),
    ];
    match agent {
        "claude" => merge_hooks(&home.join(".claude/settings.json"), CLAUDE, true),
        "codex" => merge_hooks(&home.join(".codex/hooks.json"), CODEX, true),
        "gemini" => merge_hooks(&home.join(".gemini/settings.json"), GEMINI, false),
        "qwen" => merge_hooks(&home.join(".qwen/settings.json"), GEMINI, false),
        "iflow" => merge_hooks(&home.join(".iflow/settings.json"), IFLOW, true),
        "proto" => merge_hooks(&home.join(".proto/settings.json"), GEMINI, false),
        _ => {}
    }
}

/// Installiert bzw. aktualisiert das "Pixel Code"-Plugin für OpenCode oder omp.
/// Wird bei jedem Start des Agents aufgerufen, damit immer die aktuelle Version liegt.
pub fn install_agent_plugin(agent: &str) {
    let Some(home) = dirs::home_dir() else { return };
    if let Some(bin) = crate::agents::get(agent).and_then(|a| a.bin) {
        if !matches!(agent, "opencode" | "omp" | "pi") {
            let bin = crate::agents::resolve(bin).unwrap_or_else(|| bin.into());
            let agent = agent.to_string();
            std::thread::spawn(move || install_mcp(&agent, &bin));
            return;
        }
    }
    let desc = "Pixel Code: lets the agent use Pixel Code plugins (GitHub, ...)";
    match agent {
        "opencode" => {
            // OpenCode 2 lädt Ordner aus plugins/ (und lädt sie bei Änderungen neu).
            let dir = dirs::config_dir().unwrap_or(home.join(".config")).join("opencode/plugins");
            let _ = std::fs::remove_file(dir.join("pixel-code.js"));
            write_if_changed(&dir.join("pixel-code/index.js"), &opencode_plugin());
            write_if_changed(&dir.join("pixel-code/package.json"), &package_json(desc).replace("./index.ts", "./index.js"));
        }
        "omp" => {
            // Als echtes omp-Plugin verlinkt, damit es in `omp plugin list` erscheint.
            let _ = std::fs::remove_file(home.join(".omp/agent/extensions/pixel-code.ts"));
            let dir = dirs::data_dir().unwrap_or(home.join(".local/share")).join("pixel-code/agent-plugins/omp");
            write_if_changed(&dir.join("index.ts"), &omp_extension());
            let changed = write_if_changed(&dir.join("package.json"), &package_json(desc));
            let lock = std::fs::read_to_string(home.join(".omp/plugins/omp-plugins.lock.json")).unwrap_or_default();
            if changed || !lock.contains("pixel-code") {
                std::thread::spawn(move || {
                    let omp = crate::agents::get("omp").and_then(|a| a.bin).unwrap_or("omp");
                    let _ = Command::new(omp).arg("plugin").arg("link").arg(&dir).stdin(Stdio::null()).output();
                });
            }
        }
        // Pi: omp ist ein Pi-Fork, die Extension-API ist dieselbe
        "pi" => {
            write_if_changed(&home.join(".pi/agent/extensions/pixel-code.ts"), &omp_extension());
        }
        _ => {}
    }
}
