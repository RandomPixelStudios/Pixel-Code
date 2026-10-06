// Hugging Face - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const H = settings.token ? { Authorization: `Bearer ${settings.token}` } : {};
const [cmd, ...a] = args;
if (cmd === "status" || cmd === "connect") { if (!settings.token) { console.log("Public access"); } else { const r = await http("GET", "https://huggingface.co/api/whoami-v2", { headers: H }); console.log(`Connected as ${r.name}`); } }
else if (cmd === "models" || cmd === "datasets") print((await http("GET", `https://huggingface.co/api/${cmd}?search=${encodeURIComponent(a.join(" "))}&limit=20&sort=downloads`, { headers: H })).map((m) => `${m.id}  (${m.downloads ?? 0} downloads)`).join("\n"));
else if (cmd === "run") print(await http("POST", `https://router.huggingface.co/hf-inference/models/${a[0]}`, { headers: H, body: { inputs: a.slice(1).join(" ") } }));
else console.log("Usage:\n  models <search>\n  datasets <search>\n  run <model> <text>");
