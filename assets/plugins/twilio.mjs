// Twilio SMS - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const [f, rest] = flags(args);
const [cmd, ...a] = rest;
const H = { Authorization: "Basic " + Buffer.from(`${settings.sid || ""}:${settings.token || ""}`).toString("base64") };
const base = `https://api.twilio.com/2010-04-01/Accounts/${settings.sid}`;
if (cmd === "status" || cmd === "connect") { const r = await http("GET", `${base}.json`, { headers: H }); console.log(`Connected (${r.friendly_name})`); }
else if (cmd === "send") { const r = await http("POST", `${base}/Messages.json`, { headers: H, form: { From: settings.from, To: f.to || settings.to, Body: a.join(" ") } }); console.log(`Sent (${r.sid}).`); }
else console.log("Usage: send <text> [--to +49...]");
