// Wikipedia - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const [f, rest] = flags(args);
const lang = f.lang || settings.lang || "en";
const [cmd, ...a] = rest;
const H = { "User-Agent": "pixel-code" };
if (cmd === "status" || cmd === "connect") console.log(`Wikipedia (${lang})`);
else if (cmd === "search") print((await http("GET", `https://${lang}.wikipedia.org/w/api.php?action=query&list=search&format=json&srlimit=10&srsearch=${encodeURIComponent(a.join(" "))}`, { headers: H })).query.search.map((s) => s.title).join("\n"));
else if (cmd === "read") { const r = await http("GET", `https://${lang}.wikipedia.org/w/api.php?action=query&prop=extracts&explaintext=1&format=json&redirects=1&titles=${encodeURIComponent(a.join(" "))}`, { headers: H }); print(Object.values(r.query.pages)[0].extract?.slice(0, 20000) || "Not found."); }
else console.log("Usage:\n  search <words> [--lang de]\n  read <title> [--lang de]");
