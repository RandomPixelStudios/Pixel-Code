// Vultr: instances via API key.
import { args, settings, fail, http, print, rawCall } from "./common.mjs";

const base = "https://api.vultr.com/v2";
const headers = { Authorization: `Bearer ${settings.api_key || ""}` };
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !settings.api_key) fail("No Vultr API key set.");
const get = (p) => http("GET", base + p, { headers });

switch (cmd) {
  case "connect": case "status": { const a = await get("/account"); console.log(`Connected as ${a.account.email} (balance ${a.account.balance})`); break; }
  case "instances": print((await get("/instances")).instances.map((i) => `${i.id} ${i.label} ${i.status}/${i.power_status} ${i.main_ip} ${i.plan} ${i.region}`).join("\n")); break;
  case "power": print(await http("POST", `${base}/instances/${rest[0]}/${({ on: "start", off: "halt", reboot: "reboot" })[rest[1]] || rest[1]}`, { headers })); break;
  case "raw": await rawCall(base, headers, rest); break;
  default: console.log(`Usage:
  instances | power <id> on|off|reboot | raw <METHOD> </v2 path> [json]`);
}
