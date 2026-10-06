// Nextcloud - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = (settings.url || "").replace(/\/$/, "") + `/remote.php/dav/files/${settings.user || ""}`;
const headers = { Authorization: "Basic " + Buffer.from(`${settings.user || ""}:${settings.password || ""}`).toString("base64") };
await restPlugin({ base, headers, need: [settings.url, settings.password], status: async () => { await http("PROPFIND", base + "/", { headers: { ...headers, Depth: "0" }, raw: true }); return `Connected as ${settings.user}`; },
  commands: {
    ls: async (_req, a) => { const t = await (await http("PROPFIND", `${base}/${a[0] || ""}`, { headers: { ...headers, Depth: "1" }, raw: true })).text(); return [...t.matchAll(/<d:href>([^<]+)<\/d:href>/g)].map((m) => decodeURIComponent(m[1]).split(`/files/${settings.user}`)[1]).join("\n"); },
    get: async (_req, a) => { const r = await http("GET", `${base}/${a[0]}`, { headers, raw: true }); const { writeFileSync } = await import("node:fs"); const out = a[1] || a[0].split("/").pop(); writeFileSync(out, Buffer.from(await r.arrayBuffer())); return `Saved ${out}`; },
    put: async (_req, a) => { const { readFileSync } = await import("node:fs"); await http("PUT", `${base}/${a[1]}`, { headers, body: readFileSync(a[0]), raw: true }); return "Uploaded."; },
  },
  help: `Usage:\n  ls [path]\n  get <path> [out]\n  put <file> <path>` });
