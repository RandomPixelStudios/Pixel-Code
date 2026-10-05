// DigitalOcean: droplets, apps and more via API token.
import { args, settings, fail, http, print, rawCall } from "./common.mjs";

const base = "https://api.digitalocean.com/v2";
const headers = { Authorization: `Bearer ${settings.token || ""}` };
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !settings.token) fail("No DigitalOcean token set.");
const get = (p) => http("GET", base + p, { headers });

switch (cmd) {
  case "connect": case "status": { const a = await get("/account"); console.log(`Connected as ${a.account.email}`); break; }
  case "droplets": print((await get("/droplets")).droplets.map((d) => `${d.id} ${d.name} ${d.status} ${d.networks.v4.find((n) => n.type === "public")?.ip_address ?? ""} ${d.size_slug} ${d.region.slug}`).join("\n")); break;
  case "power": print(await http("POST", `${base}/droplets/${rest[0]}/actions`, { headers, body: { type: ({ on: "power_on", off: "shutdown", reboot: "reboot" })[rest[1]] || rest[1] } })); break;
  case "apps": print((await get("/apps")).apps?.map((a) => `${a.id} ${a.spec.name} ${a.live_url ?? ""}`).join("\n") || "No apps."); break;
  case "deploy": print(await http("POST", `${base}/apps/${rest[0]}/deployments`, { headers, body: { force_build: true } })); break;
  case "raw": await rawCall(base, headers, rest); break;
  default: console.log(`Usage:
  droplets | power <id> on|off|reboot | apps | deploy <app_id> | raw <METHOD> </v2 path> [json]`);
}
