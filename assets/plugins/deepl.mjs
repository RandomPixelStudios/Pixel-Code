// DeepL - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const [f, rest] = flags(args);
const key = settings.api_key || "";
const base = key.endsWith(":fx") ? "https://api-free.deepl.com/v2" : "https://api.deepl.com/v2";
const H = { Authorization: `DeepL-Auth-Key ${key}` };
const [cmd, ...a] = rest;
if (cmd === "status" || cmd === "connect" || cmd === "usage") { const u = await http("GET", `${base}/usage`, { headers: H }); console.log(`${u.character_count} / ${u.character_limit} characters used`); }
else if (cmd === "translate") { const r = await http("POST", `${base}/translate`, { headers: H, body: { text: [a.join(" ")], target_lang: (f.to || "EN").toUpperCase(), ...(f.from ? { source_lang: f.from.toUpperCase() } : {}) } }); print(r.translations[0].text); }
else console.log("Usage:\n  translate <text> --to DE [--from EN]\n  usage");
