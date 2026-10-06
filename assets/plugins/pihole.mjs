// Pi-hole v6: stats, blocking on/off, block/allow domains.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const base = (settings.url || "http://pi.hole").replace(/\/$/, "") + "/api";
async function sid() { const r = await http("POST", base + "/auth", { body: { password: settings.password || "" } }); if (!r.session?.valid) fail("Pi-hole login failed"); return { sid: r.session.sid }; }
const [cmd, ...rest] = args;
switch (cmd) {
  case "connect": case "status": { const h = await sid(); const s = await http("GET", base + "/stats/summary", { headers: h }); console.log(`Connected: ${s.queries.total} queries today, ${s.queries.percent_blocked.toFixed(1)}% blocked`); break; }
  case "stats": print(await http("GET", base + "/stats/summary", { headers: await sid() })); break;
  case "disable": print(await http("POST", base + "/dns/blocking", { headers: await sid(), body: { blocking: false, timer: Number(rest[0] || 300) } })); break;
  case "enable": print(await http("POST", base + "/dns/blocking", { headers: await sid(), body: { blocking: true } })); break;
  case "block": print(await http("POST", base + "/domains/deny/exact", { headers: await sid(), body: { domain: rest[0], comment: "Pixel Code" } })); break;
  case "allow": print(await http("POST", base + "/domains/allow/exact", { headers: await sid(), body: { domain: rest[0], comment: "Pixel Code" } })); break;
  default: console.log("Usage:\n  stats | disable [seconds] | enable | block <domain> | allow <domain>");
}
