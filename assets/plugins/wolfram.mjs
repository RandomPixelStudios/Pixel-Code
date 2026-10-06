// Wolfram|Alpha - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const [cmd, ...a] = args;
const ask = async (q) => (await http("GET", `https://www.wolframalpha.com/api/v1/llm-api?input=${encodeURIComponent(q)}&appid=${settings.app_id || ""}`, { raw: true })).text();
if (cmd === "status" || cmd === "connect") { await ask("1+1"); console.log("Connected"); }
else if (cmd === "ask") print(await ask(a.join(" ")));
else console.log("Usage: ask <question>   e.g. ask integrate x^2 sin x");
