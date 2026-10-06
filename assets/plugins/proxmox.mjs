// Proxmox VE via its REST API (API token).
import { args, settings, fail, http, print } from "./common.mjs";

process.env.NODE_TLS_REJECT_UNAUTHORIZED = settings.insecure === "yes" ? "0" : process.env.NODE_TLS_REJECT_UNAUTHORIZED;
const base = (settings.url || "https://proxmox.local:8006").replace(/\/$/, "") + "/api2/json";
const headers = { Authorization: `PVEAPIToken=${settings.token_id || ""}=${settings.secret || ""}` };
const [cmd, ...a] = args;
if (cmd && cmd !== "help" && !settings.secret) fail("No Proxmox API token set.");
const get = async (p) => (await http("GET", base + p, { headers })).data;

async function find(vmid) {
  const r = (await get("/cluster/resources?type=vm")).find((x) => String(x.vmid) === String(vmid));
  if (!r) fail(`No VM/CT with id ${vmid}`);
  return r;
}

switch (cmd) {
  case "connect": case "status": { const v = await get("/version"); console.log(`Connected to Proxmox VE ${v.version}`); break; }
  case "nodes": print((await get("/nodes")).map((n) => `${n.node}  ${n.status}  cpu ${(n.cpu * 100).toFixed(0)}%  mem ${(n.mem / n.maxmem * 100).toFixed(0)}%`).join("\n")); break;
  case "vms": print((await get("/cluster/resources?type=vm")).map((v) => `${v.vmid}  ${v.type}  ${v.name}  ${v.status}  node=${v.node}`).join("\n")); break;
  case "power": {
    const r = await find(a[0]);
    const action = { on: "start", off: "shutdown", stop: "stop", reboot: "reboot" }[a[1]] || a[1];
    print(await http("POST", `${base}/nodes/${r.node}/${r.type}/${r.vmid}/status/${action}`, { headers }));
    break;
  }
  case "raw": print(await http(a[0] || "GET", base + a[1], { headers, body: a[2] ? JSON.parse(a[2]) : undefined })); break;
  default: console.log(`Usage:
  nodes
  vms                         VMs and containers
  power <vmid> on|off|stop|reboot
  raw <METHOD> </path> [json] e.g. raw GET /cluster/status`);
}
