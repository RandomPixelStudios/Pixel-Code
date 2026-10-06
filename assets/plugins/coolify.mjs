// Coolify - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = (settings.url || "").replace(/\/$/, "") + "/api/v1";
const headers = { Authorization: `Bearer ${settings.token || ""}` };
await restPlugin({ base, headers, need: [settings.url, settings.token], status: async (req) => `Coolify ${await req("GET", "/version")}`,
  commands: {
    apps: async (req) => (await req("GET", "/applications")).map((x) => `${x.uuid}  ${x.name}  ${x.status}`).join("\n"),
    servers: async (req) => (await req("GET", "/servers")).map((x) => `${x.uuid}  ${x.name}  ${x.ip}`).join("\n"),
    deploy: async (req, a) => req("GET", `/deploy?uuid=${a[0]}`),
  },
  help: `Usage:\n  apps\n  servers\n  deploy <uuid>` });
