// TrueNAS: pools, datasets, alerts and apps.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const base = (settings.url || "").replace(/\/$/, "") + "/api/v2.0";
process.env.NODE_TLS_REJECT_UNAUTHORIZED = settings.verify_tls === "yes" ? "1" : "0";
await restPlugin({
  base, headers: { Authorization: `Bearer ${settings.api_key || ""}` }, need: [settings.url, settings.api_key],
  status: async (req) => { const s = await req("GET", "/system/info"); return `Connected to ${s.hostname} (${s.version})`; },
  commands: {
    pools: async (req) => (await req("GET", "/pool")).map((p) => `${p.name}  ${p.status}  ${p.healthy ? "healthy" : "UNHEALTHY"}`).join("\n"),
    datasets: async (req) => (await req("GET", "/pool/dataset?limit=100")).map((d) => `${d.name}  used ${d.used?.value}  avail ${d.available?.value}`).join("\n"),
    alerts: async (req) => (await req("GET", "/alert/list")).map((a) => `${a.level}  ${a.formatted}`).join("\n") || "No alerts.",
    apps: async (req) => (await req("GET", "/app")).map((a) => `${a.name}  ${a.state}`).join("\n"),
    snapshot: async (req, [ds, name]) => req("POST", "/zfs/snapshot", { dataset: ds, name: name || "pixelcode-" + Date.now() }),
  },
  help: "Usage:\n  pools | datasets | alerts | apps | snapshot <dataset> [name]",
});
