// Heroku: apps, dynos, config and builds via API key.
import { args, settings, fail, http, print, rawCall } from "./common.mjs";

const base = "https://api.heroku.com";
const headers = { Authorization: `Bearer ${settings.api_key || ""}`, Accept: "application/vnd.heroku+json; version=3" };
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !settings.api_key) fail("No Heroku API key set.");
const get = (p) => http("GET", base + p, { headers });

switch (cmd) {
  case "connect": case "status": { const a = await get("/account"); console.log(`Connected as ${a.email}`); break; }
  case "apps": print((await get("/apps")).map((a) => `${a.name} ${a.web_url}`).join("\n")); break;
  case "dynos": print((await get(`/apps/${rest[0]}/dynos`)).map((d) => `${d.name} ${d.state} ${d.size}`).join("\n")); break;
  case "restart": await http("DELETE", `${base}/apps/${rest[0]}/dynos`, { headers }); console.log("Restarted."); break;
  case "config": print(await get(`/apps/${rest[0]}/config-vars`)); break;
  case "config-set": print(await http("PATCH", `${base}/apps/${rest[0]}/config-vars`, { headers, body: Object.fromEntries(rest.slice(1).map((kv) => kv.split(/=(.*)/s).slice(0, 2))) })); break;
  case "logs": { const s = await http("POST", `${base}/apps/${rest[0]}/log-sessions`, { headers, body: { lines: Number(rest[1] || 200) } }); console.log(await (await fetch(s.logplex_url)).text()); break; }
  case "raw": await rawCall(base, headers, rest); break;
  default: console.log(`Usage:
  apps | dynos <app> | restart <app> | config <app> | config-set <app> KEY=value ... | logs <app> [lines] | raw <METHOD> <path> [json]`);
}
