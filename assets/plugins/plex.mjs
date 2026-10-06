// Plex Media Server.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const base = (settings.url || "http://127.0.0.1:32400").replace(/\/$/, "");
await restPlugin({
  base, headers: { "X-Plex-Token": settings.token || "" }, need: [settings.token],
  status: async (req) => { const s = (await req("GET", "/")).MediaContainer; return `Connected to ${s.friendlyName} (Plex ${s.version})`; },
  commands: {
    libraries: async (req) => (await req("GET", "/library/sections")).MediaContainer.Directory.map((d) => `${d.key}  ${d.title}  (${d.type})`).join("\n"),
    search: async (req, q) => ((await req("GET", `/search?query=${encodeURIComponent(q.join(" "))}`)).MediaContainer.Metadata || []).map((m) => `${m.title} (${m.type}${m.year ? ", " + m.year : ""})`).join("\n") || "Nothing found.",
    sessions: async (req) => ((await req("GET", "/status/sessions")).MediaContainer.Metadata || []).map((m) => `${m.User?.title}: ${m.title} on ${m.Player?.title}`).join("\n") || "Nobody is watching.",
    scan: async (req, [key]) => { await req("GET", `/library/sections/${key}/refresh`); return "Scan started"; },
  },
  help: "Usage:\n  libraries | search <text> | sessions | scan <library key>",
});
