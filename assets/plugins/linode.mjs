// Akamai / Linode: instances via API token.
import { args, settings, fail, http, print, rawCall } from "./common.mjs";

const base = "https://api.linode.com/v4";
const headers = { Authorization: `Bearer ${settings.token || ""}` };
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !settings.token) fail("No Linode token set.");
const get = (p) => http("GET", base + p, { headers });

switch (cmd) {
  case "connect": case "status": { const p = await get("/profile"); console.log(`Connected as ${p.username}`); break; }
  case "instances": print((await get("/linode/instances")).data.map((l) => `${l.id} ${l.label} ${l.status} ${l.ipv4[0]} ${l.type} ${l.region}`).join("\n")); break;
  case "power": print(await http("POST", `${base}/linode/instances/${rest[0]}/${({ on: "boot", off: "shutdown", reboot: "reboot" })[rest[1]] || rest[1]}`, { headers })); break;
  case "raw": await rawCall(base, headers, rest); break;
  default: console.log(`Usage:
  instances | power <id> on|off|reboot | raw <METHOD> </v4 path> [json]`);
}
