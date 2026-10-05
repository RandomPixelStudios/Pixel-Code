//! Telegram-Bot als Fernbedienung: Nachrichten an den Bot gehen an die Agent-Terminals,
//! Antworten und Benachrichtigungen gehen zurück. Läuft über curl, ohne zusätzliche Abhängigkeiten.

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use crate::plugins;

fn token() -> Option<String> {
    let reg = plugins::load_registry();
    let p = reg.plugins.into_iter().find(|p| p.id == "telegram")?;
    if !p.enabled {
        return None;
    }
    p.settings.get("bot_token").filter(|t| !t.trim().is_empty()).map(|t| t.trim().to_string())
}

fn api(token: &str, method: &str, body: &serde_json::Value, timeout: u64) -> Option<serde_json::Value> {
    let mut child = Command::new("curl")
        .args(["-s", "-m", &timeout.to_string(), "-H", "Content-Type: application/json", "--data-binary", "@-"])
        .arg(format!("https://api.telegram.org/bot{token}/{method}"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    child.stdin.take()?.write_all(body.to_string().as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    serde_json::from_slice(&out.stdout).ok()
}

fn discord() -> Option<(String, String)> {
    let reg = plugins::load_registry();
    let p = reg.plugins.into_iter().find(|p| p.id == "discord" && p.enabled)?;
    let token = p.settings.get("bot_token")?.trim().to_string();
    let channel = p.settings.get("channel_id")?.trim().to_string();
    (!token.is_empty() && !channel.is_empty()).then_some((token, channel))
}

fn discord_api(token: &str, method: &str, path: &str, body: Option<&serde_json::Value>) -> Option<serde_json::Value> {
    let mut cmd = Command::new("curl");
    cmd.args(["-s", "-m", "20", "-X", method, "-H", &format!("Authorization: Bot {token}"), "-H", "Content-Type: application/json"])
        .arg(format!("https://discord.com/api/v10{path}"))
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if let Some(b) = body {
        cmd.args(["--data-binary", &b.to_string()]);
    }
    serde_json::from_slice(&cmd.output().ok()?.stdout).ok()
}

/// Schickt eine Nachricht an alle aktiven Fernsteuerungen (Telegram, Discord).
pub fn send(text: String) {
    let t = text.clone();
    std::thread::spawn(move || {
        let Some((token, channel)) = discord() else { return };
        let chars: Vec<char> = t.chars().collect();
        let t: String = chars[chars.len().saturating_sub(1900)..].iter().collect();
        // Codeblock, damit Terminal-Ausgaben lesbar bleiben
        let content = if t.contains('\n') { format!("```\n{}\n```", t.replace("```", "``")) } else { t };
        discord_api(&token, "POST", &format!("/channels/{channel}/messages"), Some(&serde_json::json!({ "content": content })));
    });
    send_telegram(text);
}

fn send_telegram(text: String) {
    std::thread::spawn(move || {
        let (Some(token), chat) = (token(), plugins::setting("telegram", "chat_id")) else { return };
        if chat.is_empty() {
            return;
        }
        // Telegram erlaubt max. 4096 Zeichen pro Nachricht
        let text: String = if text.chars().count() > 4000 {
            let skip = text.chars().count() - 4000;
            format!("…{}", text.chars().skip(skip).collect::<String>())
        } else {
            text
        };
        api(&token, "sendMessage", &serde_json::json!({ "chat_id": chat, "text": text }), 20);
    });
}

pub fn active() -> bool {
    (token().is_some() && !plugins::setting("telegram", "chat_id").is_empty()) || discord().is_some()
}

pub const HELP: &str = "Pixel Code remote control\n\n\
/list – all terminals\n\
/use <n> – choose the terminal your messages go to\n\
/screen – show the end of that terminal\n\
/new <agent> – start an agent (claude, codex, opencode, omp, …)\n\
/esc /ctrlc /enter – send that key\n\n\
Any other message is typed into the chosen terminal and sent with Enter.";

/// Startet das Long-Polling. Eingehende Texte des gekoppelten Chats landen in `tx`.
/// Der erste Chat, der dem Bot schreibt, wird gekoppelt.
pub fn start(ctx: eframe::egui::Context) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    // Discord: Kanal alle 3 Sekunden abfragen (kein Gateway nötig)
    {
        let tx = tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let mut last: Option<String> = None;
            let mut last_channel = String::new();
            loop {
                std::thread::sleep(Duration::from_secs(3));
                let Some((token, channel)) = discord() else { continue };
                if channel != last_channel {
                    last = None;
                    last_channel = channel.clone();
                }
                let path = match &last {
                    Some(id) => format!("/channels/{channel}/messages?after={id}&limit=50"),
                    None => format!("/channels/{channel}/messages?limit=1"),
                };
                let Some(msgs) = discord_api(&token, "GET", &path, None).and_then(|v| v.as_array().cloned()) else { continue };
                let first = last.is_none();
                // Discord liefert die neueste Nachricht zuerst
                for m in msgs.iter().rev() {
                    if let Some(id) = m["id"].as_str() {
                        last = Some(id.to_string());
                    }
                    if first || m["author"]["bot"].as_bool() == Some(true) {
                        continue;
                    }
                    if let Some(text) = m["content"].as_str().filter(|t| !t.is_empty()) {
                        let _ = tx.send(text.to_string());
                        ctx.request_repaint();
                    }
                }
                if first && last.is_none() {
                    last = Some("0".into());
                }
            }
        });
    }
    std::thread::spawn(move || {
        let mut offset: i64 = 0;
        let mut last_token = String::new();
        loop {
            let Some(token) = token() else {
                std::thread::sleep(Duration::from_secs(3));
                continue;
            };
            if token != last_token {
                offset = 0;
                last_token = token.clone();
            }
            let res = api(&token, "getUpdates", &serde_json::json!({ "offset": offset, "timeout": 25 }), 35);
            let Some(updates) = res.as_ref().and_then(|r| r["result"].as_array()) else {
                std::thread::sleep(Duration::from_secs(5));
                continue;
            };
            for u in updates {
                offset = offset.max(u["update_id"].as_i64().unwrap_or(0) + 1);
                let msg = &u["message"];
                let (Some(chat), Some(text)) = (msg["chat"]["id"].as_i64(), msg["text"].as_str()) else { continue };
                let paired = plugins::setting("telegram", "chat_id");
                if paired.is_empty() {
                    plugins::set_setting("telegram", "chat_id", &chat.to_string());
                    let who = msg["from"]["username"].as_str().or(msg["from"]["first_name"].as_str()).unwrap_or("you");
                    plugins::set_setting("telegram", "paired_user", who);
                    send(format!("✅ Paired with Pixel Code.\n\n{HELP}"));
                    ctx.request_repaint();
                } else if paired == chat.to_string() {
                    let _ = tx.send(text.to_string());
                    ctx.request_repaint();
                }
            }
        }
    });
    rx
}
