// Brave Search - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const [f, rest] = flags(args);
const H = { "X-Subscription-Token": settings.api_key || "" };
const [cmd, ...a] = rest;
const q = encodeURIComponent(a.join(" "));
if (cmd === "status" || cmd === "connect") { await http("GET", "https://api.search.brave.com/res/v1/web/search?q=test&count=1", { headers: H }); console.log("Connected"); }
else if (cmd === "search") print((await http("GET", `https://api.search.brave.com/res/v1/web/search?q=${q}&count=${f.n || 10}`, { headers: H })).web?.results.map((r) => `${r.title}\n${r.url}\n${r.description}`).join("\n\n"));
else if (cmd === "news") print((await http("GET", `https://api.search.brave.com/res/v1/news/search?q=${q}`, { headers: H })).results.map((r) => `${r.title}\n${r.url}`).join("\n\n"));
else console.log("Usage:\n  search <query> [--n 10]\n  news <query>");
