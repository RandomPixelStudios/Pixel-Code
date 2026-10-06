// Tailscale: devices in your tailnet (CLI locally, API for the admin view).
import { spawnSync } from "node:child_process";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const [cmd, ...rest] = args;
const bin = which(["tailscale"]);
const api = (p) => http("GET", `https://api.tailscale.com/api/v2/tailnet/-${p}`, { headers: { Authorization: `Bearer ${settings.api_key || ""}` } });
switch (cmd) {
  case "connect": case "status": {
    if (bin) { const r = spawnSync(bin, ["status", "--json"], { encoding: "utf8" }); if (r.status === 0) { const s = JSON.parse(r.stdout); console.log(`Tailscale ${s.BackendState}, ${Object.keys(s.Peer || {}).length} peers`); break; } }
    if (!settings.api_key) fail("Install tailscale or add an API key.");
    console.log(`API ok, ${(await api("/devices")).devices.length} devices`); break;
  }
  case "devices": if (settings.api_key) print((await api("/devices")).devices.map((d) => `${d.hostname}  ${d.addresses[0]}  ${d.os}  last seen ${d.lastSeen}`).join("\n")); else passthrough(bin, ["status"]); break;
  case undefined: case "help": console.log("Usage: devices | any tailscale arguments ('ping host', 'ip', 'file cp x host:', 'serve 3000')"); break;
  default: if (!bin) fail("tailscale CLI not installed."); passthrough(bin, args);
}
