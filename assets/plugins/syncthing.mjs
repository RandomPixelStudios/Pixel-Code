// Syncthing: folder and device status via the local REST API.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
await restPlugin({
  base: (settings.url || "http://127.0.0.1:8384").replace(/\/$/, "") + "/rest", headers: { "X-API-Key": settings.api_key || "" }, need: [settings.api_key],
  status: async (req) => { const s = await req("GET", "/system/status"); return `Syncthing running, uptime ${Math.round(s.uptime / 3600)} h`; },
  commands: {
    folders: async (req) => { const c = await req("GET", "/config/folders"); const out = []; for (const f of c) { const s = await req("GET", `/db/status?folder=${f.id}`); out.push(`${f.label || f.id}  ${s.state}  ${(100 * (s.globalBytes ? s.inSyncBytes / s.globalBytes : 1)).toFixed(0)}% in sync  ${f.path}`); } return out.join("\n"); },
    devices: async (req) => { const c = await req("GET", "/system/connections"); const d = await req("GET", "/config/devices"); return d.map((x) => `${x.name}  ${c.connections[x.deviceID]?.connected ? "connected" : "offline"}`).join("\n"); },
    rescan: async (req, [folder]) => { await http("POST", `${(settings.url || "http://127.0.0.1:8384").replace(/\/$/, "")}/rest/db/scan${folder ? "?folder=" + folder : ""}`, { headers: { "X-API-Key": settings.api_key }, raw: true }); return "Rescan started"; },
  },
  help: "Usage:\n  folders | devices | rescan [folder_id]",
});
