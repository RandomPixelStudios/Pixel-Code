// Gotify - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const [f, rest] = flags(args);
const [cmd, ...a] = rest;
const base = (settings.url || "").replace(/\/$/, "");
if (cmd === "status" || cmd === "connect") { const r = await http("GET", `${base}/version`); console.log(`Gotify ${r.version}`); }
else if (cmd === "send") { await http("POST", `${base}/message?token=${settings.token}`, { body: { title: f.title || "Pixel Code", message: a.join(" "), priority: Number(f.priority || 5) } }); console.log("Sent."); }
else console.log("Usage: send <text> [--title ..] [--priority 5]");
