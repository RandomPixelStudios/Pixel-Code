// Resend - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const [f, rest] = flags(args);
const [cmd, ...a] = rest;
const H = { Authorization: `Bearer ${settings.api_key || ""}` };
if (cmd === "status" || cmd === "connect") { const r = await http("GET", "https://api.resend.com/domains", { headers: H }); console.log(`Connected, ${r.data.length} domains`); }
else if (cmd === "send") { const body = { from: settings.from, to: [f.to], subject: f.subject || "Pixel Code", ...(f.html ? { html: a.join(" ") } : { text: a.join(" ") }) }; const r = await http("POST", "https://api.resend.com/emails", { headers: H, body }); console.log(`Sent (${r.id}).`); }
else console.log("Usage: send <text> --to address --subject .. [--html]");
