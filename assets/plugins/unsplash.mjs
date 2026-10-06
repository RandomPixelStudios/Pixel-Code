// Unsplash - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = "https://api.unsplash.com";
const headers = { Authorization: `Client-ID ${settings.access_key || ""}` };
await restPlugin({ base, headers, need: [settings.access_key], status: async (req) => { await req("GET", "/photos?per_page=1"); return "Connected"; },
  commands: {
    search: async (req, a) => (await req("GET", `/search/photos?per_page=15&query=${encodeURIComponent(a.join(" "))}`)).results.map((p) => `${p.id}  ${p.width}x${p.height}  ${p.alt_description || ""}  by ${p.user.name}`).join("\n"),
    get: async (req, a) => { const p = await req("GET", `/photos/${a[0]}`); await req("GET", p.links.download_location.replace("https://api.unsplash.com", "")); const r = await http("GET", p.urls.full, { raw: true }); const { writeFileSync } = await import("node:fs"); const out = a[1] || `unsplash-${a[0]}.jpg`; writeFileSync(out, Buffer.from(await r.arrayBuffer())); return `Saved ${out} (photo by ${p.user.name} on Unsplash)`; },
  },
  help: `Usage:\n  search <words>\n  get <photo_id> [out]` });
