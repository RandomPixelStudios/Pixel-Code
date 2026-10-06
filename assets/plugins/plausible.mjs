// Plausible - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = (settings.url || "https://plausible.io").replace(/\/$/, "") + "/api/v1/stats";
const headers = { Authorization: `Bearer ${settings.token || ""}` };
await restPlugin({ base, headers, need: [settings.token, settings.site], status: async (req) => { await req("GET", `/realtime/visitors?site_id=${settings.site}`); return `Connected to ${settings.site}`; },
  commands: {
    stats: async (req, a) => (await req("GET", `/aggregate?site_id=${settings.site}&period=${a[0] || "30d"}&metrics=visitors,pageviews,bounce_rate,visit_duration`)).results,
    pages: async (req, a) => (await req("GET", `/breakdown?site_id=${settings.site}&period=${a[0] || "30d"}&property=event:page&limit=20`)).results.map((r) => `${r.visitors}  ${r.page}`).join("\n"),
    sources: async (req, a) => (await req("GET", `/breakdown?site_id=${settings.site}&period=${a[0] || "30d"}&property=visit:source&limit=20`)).results.map((r) => `${r.visitors}  ${r.source}`).join("\n"),
  },
  help: `Usage:\n  stats [day|7d|30d|month|12mo]\n  pages [period]\n  sources [period]` });
