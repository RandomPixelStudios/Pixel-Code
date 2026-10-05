// Home Assistant via REST API (long-lived access token).
import { args, settings, fail, http, print } from "./common.mjs";

const base = (settings.url || "http://homeassistant.local:8123").replace(/\/$/, "") + "/api";
const headers = { Authorization: `Bearer ${settings.token || ""}` };
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !settings.token) fail("No Home Assistant token set.");

switch (cmd) {
  case "connect": case "status": { const r = await http("GET", `${base}/config`, { headers }); console.log(`Connected to ${r.location_name} (Home Assistant ${r.version})`); break; }
  case "states": print((await http("GET", `${base}/states`, { headers })).filter((s) => !rest[0] || s.entity_id.startsWith(rest[0])).map((s) => `${s.entity_id} = ${s.state}${s.attributes.friendly_name ? "  (" + s.attributes.friendly_name + ")" : ""}`).join("\n")); break;
  case "state": print(await http("GET", `${base}/states/${rest[0]}`, { headers })); break;
  case "call": {
    const [domain, service] = rest[0].split(".");
    const data = rest[1]?.startsWith("{") ? JSON.parse(rest.slice(1).join(" ")) : rest[1] ? { entity_id: rest[1] } : {};
    await http("POST", `${base}/services/${domain}/${service}`, { headers, body: data });
    console.log("Done.");
    break;
  }
  default: console.log(`Usage:
  states [domain]                 e.g. states light.
  state <entity_id>
  call <domain.service> [entity_id | json]   e.g. call light.turn_on light.desk  |  call light.turn_on {"entity_id":"light.desk","rgb_color":[255,0,0]}`);
}
