// Portainer - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = (settings.url || "").replace(/\/$/, "") + "/api";
const headers = { "X-API-Key": settings.token || "" };
await restPlugin({ base, headers, need: [settings.url, settings.token], status: async (req) => `Portainer ${(await req("GET", "/system/version")).ServerVersion}`,
  commands: {
    endpoints: async (req) => (await req("GET", "/endpoints")).map((e) => `${e.Id}  ${e.Name}`).join("\n"),
    containers: async (req, a) => (await req("GET", `/endpoints/${a[0]}/docker/containers/json?all=1`)).map((c) => `${c.Names[0]}  ${c.State}  ${c.Image}`).join("\n"),
    restart: async (req, a) => { await req("POST", `/endpoints/${a[0]}/docker/containers/${a[1]}/restart`); return "Restarted."; },
  },
  help: `Usage:\n  endpoints\n  containers <endpoint_id>\n  restart <endpoint_id> <container>` });
