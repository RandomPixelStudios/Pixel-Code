// Grafana - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = (settings.url || "http://localhost:3000").replace(/\/$/, "") + "/api";
const headers = { Authorization: `Bearer ${settings.token || ""}` };
await restPlugin({ base, headers, need: [settings.token], status: async (req) => `Grafana ${(await req("GET", "/health")).version}`,
  commands: {
    dashboards: async (req, a) => (await req("GET", `/search?type=dash-db&query=${encodeURIComponent(a.join(" "))}`)).map((d) => `${d.uid}  ${d.title}`).join("\n"),
    alerts: async (req) => (await req("GET", "/prometheus/grafana/api/v1/alerts")).data?.alerts?.map((x) => `${x.state}  ${x.labels?.alertname}`).join("\n") || "No alerts.",
  },
  help: `Usage:\n  dashboards [search]\n  alerts` });
