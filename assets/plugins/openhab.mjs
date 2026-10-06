// openHAB - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = (settings.url || "http://openhab.local:8080").replace(/\/$/, "") + "/rest";
const headers = settings.token ? { Authorization: `Bearer ${settings.token}` } : {};
await restPlugin({ base, headers, need: [], status: async (req) => `openHAB ${(await req("GET", "/")).runtimeInfo?.version}`,
  commands: {
    items: async (req, a) => (await req("GET", "/items")).filter((i) => !a[0] || i.name.toLowerCase().includes(a[0].toLowerCase())).map((i) => `${i.name} = ${i.state}  (${i.label || i.type})`).join("\n"),
    get: async (req, a) => req("GET", `/items/${a[0]}`),
    send: async (_req, a) => { await http("POST", `${base}/items/${a[0]}`, { headers: { ...headers, "Content-Type": "text/plain" }, body: a.slice(1).join(" "), raw: true }); return "Sent."; },
  },
  help: `Usage:\n  items [filter]\n  get <item>\n  send <item> <ON|OFF|value>` });
