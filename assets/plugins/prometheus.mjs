// Prometheus - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = (settings.url || "http://localhost:9090").replace(/\/$/, "") + "/api/v1";
const headers = {};
await restPlugin({ base, headers, need: [], status: async (req) => `Prometheus ${(await req("GET", "/status/buildinfo")).data.version}`,
  commands: {
    query: async (req, a) => (await req("GET", `/query?query=${encodeURIComponent(a.join(" "))}`)).data.result.map((r) => `${JSON.stringify(r.metric)} = ${r.value?.[1]}`).join("\n"),
    alerts: async (req) => (await req("GET", "/alerts")).data.alerts.map((x) => `${x.state}  ${x.labels.alertname}`).join("\n") || "No alerts.",
    targets: async (req) => (await req("GET", "/targets")).data.activeTargets.map((t) => `${t.health}  ${t.scrapeUrl}`).join("\n"),
  },
  help: `Usage:\n  query <promql>\n  alerts\n  targets` });
