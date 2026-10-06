// Pexels - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = "https://api.pexels.com";
const headers = { Authorization: settings.api_key || "" };
await restPlugin({ base, headers, need: [settings.api_key], status: async (req) => { await req("GET", "/v1/curated?per_page=1"); return "Connected"; },
  commands: {
    photos: async (req, a) => (await req("GET", `/v1/search?per_page=15&query=${encodeURIComponent(a.join(" "))}`)).photos.map((p) => `${p.src.original}  ${p.alt}`).join("\n"),
    videos: async (req, a) => (await req("GET", `/videos/search?per_page=10&query=${encodeURIComponent(a.join(" "))}`)).videos.map((v) => `${v.video_files[0]?.link}  ${v.duration}s`).join("\n"),
    get: async (_req, a) => { const r = await http("GET", a[0], { raw: true }); const { writeFileSync } = await import("node:fs"); const out = a[1] || a[0].split("/").pop().split("?")[0]; writeFileSync(out, Buffer.from(await r.arrayBuffer())); return `Saved ${out}`; },
  },
  help: `Usage:\n  photos <words>\n  videos <words>\n  get <url> [out]` });
