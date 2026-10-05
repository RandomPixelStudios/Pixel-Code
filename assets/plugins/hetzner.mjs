// Hetzner Cloud: servers, actions and more via API token.
import { args, settings, fail, http, print, rawCall } from "./common.mjs";

const base = "https://api.hetzner.cloud/v1";
const headers = { Authorization: `Bearer ${settings.token || ""}` };
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !settings.token) fail("No Hetzner Cloud API token set.");
const get = (p) => http("GET", base + p, { headers });

switch (cmd) {
  case "connect": case "status": { const s = await get("/servers"); console.log(`Connected (${s.servers.length} servers)`); break; }
  case "servers": print((await get("/servers")).servers.map((s) => `${s.id} ${s.name} ${s.status} ${s.public_net.ipv4?.ip ?? ""} ${s.server_type.name} ${s.datacenter.name}`).join("\n")); break;
  case "power": print(await http("POST", `${base}/servers/${rest[0]}/actions/${({ on: "poweron", off: "shutdown", reboot: "reboot", reset: "reset" })[rest[1]] || rest[1]}`, { headers })); break;
  case "create": print(await http("POST", `${base}/servers`, { headers, body: { name: rest[0], server_type: rest[1] || "cx22", image: rest[2] || "ubuntu-24.04", location: rest[3] || "nbg1", ssh_keys: (await get("/ssh_keys")).ssh_keys.map((k) => k.id) } })); break;
  case "delete": print(await http("DELETE", `${base}/servers/${rest[0]}`, { headers })); break;
  case "raw": await rawCall(base, headers, rest); break;
  default: console.log(`Usage:
  servers | power <id> on|off|reboot|reset | create <name> [type cx22] [image ubuntu-24.04] [location nbg1] | delete <id>
  raw <METHOD> </v1 path> [json]   (volumes, firewalls, load_balancers, ...)`);
}
