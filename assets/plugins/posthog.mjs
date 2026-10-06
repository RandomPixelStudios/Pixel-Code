// PostHog - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = (settings.url || "https://eu.posthog.com").replace(/\/$/, "") + `/api/projects/${settings.project || ""}`;
const headers = { Authorization: `Bearer ${settings.token || ""}` };
await restPlugin({ base, headers, need: [settings.token, settings.project], status: async (req) => `Connected to project ${(await req("GET", "/")).name}`,
  commands: {
    query: async (req, a) => (await req("POST", "/query/", { query: { kind: "HogQLQuery", query: a.join(" ") } })).results,
    flags: async (req) => (await req("GET", "/feature_flags/")).results.map((f) => `${f.key}  ${f.active ? "on" : "off"}`).join("\n"),
  },
  help: `Usage:\n  query <hogql>   e.g. query select event, count() from events group by event\n  flags` });
