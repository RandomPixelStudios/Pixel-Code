// RevenueCat: in-app purchases and subscriptions.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const pid = settings.project_id;
await restPlugin({
  base: `https://api.revenuecat.com/v2/projects/${pid}`, headers: { Authorization: `Bearer ${settings.api_key || ""}` }, need: [settings.api_key, pid],
  status: async (req) => { const m = await req("GET", "/metrics/overview"); return "Connected: " + m.metrics.map((x) => `${x.name} ${x.value}`).slice(0, 3).join(", "); },
  commands: {
    metrics: async (req) => (await req("GET", "/metrics/overview")).metrics.map((m) => `${m.name}: ${m.value}${m.unit === "$" ? " $" : ""}`).join("\n"),
    customer: async (req, [id]) => req("GET", `/customers/${encodeURIComponent(id)}`),
    products: async (req) => (await req("GET", "/products")).items.map((p) => `${p.store_identifier}  ${p.type}  ${p.app_id}`).join("\n"),
    entitlements: async (req) => (await req("GET", "/entitlements")).items.map((e) => `${e.lookup_key}  ${e.display_name}`).join("\n"),
  },
  help: "Usage:\n  metrics | customer <app_user_id> | products | entitlements",
});
