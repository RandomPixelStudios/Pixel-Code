// Uptime Kuma: monitor states via a public status page and push monitors.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const base = (settings.url || "").replace(/\/$/, "");
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !base) fail("Uptime Kuma URL is required.");
switch (cmd) {
  case "connect": case "status": { const r = await http("GET", `${base}/api/status-page/${settings.slug || "default"}`); console.log(`Connected: ${r.config?.title ?? "status page"}`); break; }
  case "monitors": {
    const p = await http("GET", `${base}/api/status-page/${settings.slug || "default"}`);
    const h = await http("GET", `${base}/api/status-page/heartbeat/${settings.slug || "default"}`);
    for (const g of p.publicGroupList) for (const m of g.monitorList) { const last = (h.heartbeatList[m.id] || []).at(-1); console.log(`${last?.status === 1 ? "UP  " : "DOWN"}  ${m.name}  ${last ? last.ping + " ms" : ""}  ${last?.msg || ""}`); }
    break;
  }
  case "push": { await http("GET", `${base}/api/push/${rest[0]}?status=${rest[1] || "up"}&msg=${encodeURIComponent(rest.slice(2).join(" ") || "OK")}`); console.log("Pushed."); break; }
  default: console.log(`Usage:
  monitors                          all monitors of the status page (slug from settings)
  push <push_token> [up|down] [msg] report to a push monitor (e.g. after a backup)`);
}
