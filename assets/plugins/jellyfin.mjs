// Jellyfin media server.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const base = (settings.url || "http://127.0.0.1:8096").replace(/\/$/, "");
await restPlugin({
  base, headers: { Authorization: `MediaBrowser Token="${settings.api_key || ""}"` }, need: [settings.api_key],
  status: async (req) => { const s = await req("GET", "/System/Info"); return `Connected to ${s.ServerName} (Jellyfin ${s.Version})`; },
  commands: {
    libraries: async (req) => (await req("GET", "/Library/VirtualFolders")).map((l) => `${l.Name}  ${l.CollectionType ?? ""}  ${l.Locations.join(", ")}`).join("\n"),
    search: async (req, q) => (await req("GET", `/Items?searchTerm=${encodeURIComponent(q.join(" "))}&Recursive=true&Limit=20`)).Items.map((i) => `${i.Id}  ${i.Name} (${i.Type}${i.ProductionYear ? ", " + i.ProductionYear : ""})`).join("\n"),
    sessions: async (req) => (await req("GET", "/Sessions")).filter((s) => s.NowPlayingItem).map((s) => `${s.UserName}: ${s.NowPlayingItem.Name} on ${s.DeviceName}`).join("\n") || "Nobody is watching.",
    scan: async (req) => { await http("POST", base + "/Library/Refresh", { headers: { Authorization: `MediaBrowser Token="${settings.api_key}"` }, raw: true }); return "Library scan started"; },
  },
  help: "Usage:\n  libraries | search <text> | sessions | scan",
});
