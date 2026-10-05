// Render: services, deploys and logs via API key.
import { args, settings, fail, http, print, rawCall } from "./common.mjs";

const base = "https://api.render.com/v1";
const headers = { Authorization: `Bearer ${settings.api_key || ""}` };
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !settings.api_key) fail("No Render API key set.");
const get = (p) => http("GET", base + p, { headers });

switch (cmd) {
  case "connect": case "status": { const o = await get("/owners"); console.log(`Connected (${o.map((x) => x.owner.name).join(", ")})`); break; }
  case "services": print((await get("/services?limit=50")).map(({ service: s }) => `${s.id} ${s.name} ${s.type} ${s.suspended} ${s.serviceDetails?.url ?? ""}`).join("\n")); break;
  case "deploys": print((await get(`/services/${rest[0]}/deploys?limit=10`)).map(({ deploy: d }) => `${d.id} ${d.status} ${d.createdAt} ${d.commit?.message?.split("\n")[0] ?? ""}`).join("\n")); break;
  case "deploy": print(await http("POST", `${base}/services/${rest[0]}/deploys`, { headers, body: { clearCache: rest[1] === "clear" ? "clear" : "do_not_clear" } })); break;
  case "raw": await rawCall(base, headers, rest); break;
  default: console.log(`Usage:
  services | deploys <service_id> | deploy <service_id> [clear] | raw <METHOD> </v1 path> [json]`);
}
