// Discord webhook: post changelogs and release notes into a channel (no bot needed).
import { readFileSync } from "node:fs";
import { resolve, basename } from "node:path";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && !settings.url) fail("No webhook URL set.");
switch (cmd) {
  case "connect": case "status": { const r = await http("GET", settings.url); console.log(`Webhook "${r.name}" ready`); break; }
  case "send": {
    const payload = { content: f.title ? undefined : w.join(" "), username: settings.name || undefined,
      embeds: f.title ? [{ title: f.title, description: (f.file ? readFileSync(resolve(f.file), "utf8") : w.join(" ")).slice(0, 4000), url: f.url, color: 0x34d399 }] : undefined };
    if (f.attach) { const fd = new FormData(); fd.append("payload_json", JSON.stringify(payload)); String(f.attach).split(",").forEach((p, i) => fd.append(`files[${i}]`, new Blob([readFileSync(resolve(p))]), basename(p))); const r = await fetch(settings.url, { method: "POST", body: fd }); if (!r.ok) fail("Discord: " + await r.text()); }
    else await http("POST", settings.url, { body: payload, raw: true });
    console.log("Posted.");
    break;
  }
  default: console.log(`Usage:
  send <text>
  send --title "v1.2.0 released" [--file CHANGELOG.md | <text>] [--url https://modrinth.com/...] [--attach image.png]`);
}
