// OpenRouter - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const [f, rest] = flags(args);
const H = { Authorization: `Bearer ${settings.api_key || ""}` };
const [cmd, ...a] = rest;
if (cmd === "status" || cmd === "connect") { const r = await http("GET", "https://openrouter.ai/api/v1/key", { headers: H }); console.log(`Connected (${r.data?.label || "key"})`); }
else if (cmd === "models") print((await http("GET", "https://openrouter.ai/api/v1/models")).data.map((m) => m.id).filter((i) => !a[0] || i.includes(a[0])).join("\n"));
else if (cmd === "ask") { const r = await http("POST", "https://openrouter.ai/api/v1/chat/completions", { headers: H, body: { model: f.model || settings.model || "openrouter/auto", messages: [{ role: "user", content: a.join(" ") }] } }); print(r.choices[0].message.content); }
else console.log("Usage:\n  ask <prompt> [--model provider/model]\n  models [filter]");
