// Replicate - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const H = { Authorization: `Bearer ${settings.token || ""}`, Prefer: "wait" };
const [cmd, ...a] = args;
if (cmd === "status" || cmd === "connect") { const r = await http("GET", "https://api.replicate.com/v1/account", { headers: H }); console.log(`Connected as ${r.username}`); }
else if (cmd === "search") print((await http("GET", `https://api.replicate.com/v1/models?query=${encodeURIComponent(a.join(" "))}`, { headers: H })).results?.slice(0, 15).map((m) => `${m.owner}/${m.name}  ${m.description || ""}`).join("\n"));
else if (cmd === "run") { const r = await http("POST", `https://api.replicate.com/v1/models/${a[0]}/predictions`, { headers: H, body: { input: JSON.parse(a.slice(1).join(" ") || "{}") } }); print(r.output ?? r); }
else console.log(`Usage:\n  run <owner/model> <json input>   e.g. run black-forest-labs/flux-schnell {"prompt":"a pixel art castle"}\n  search <words>`);
