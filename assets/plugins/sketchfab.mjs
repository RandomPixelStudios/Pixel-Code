// Sketchfab - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = "https://api.sketchfab.com/v3";
const headers = { Authorization: `Token ${settings.token || ""}` };
await restPlugin({ base, headers, need: [settings.token], status: async (req) => `Connected as ${(await req("GET", "/me")).username}`,
  commands: {
    search: async (req, a) => (await req("GET", `/search?type=models&downloadable=true&q=${encodeURIComponent(a.join(" "))}`)).results.map((m) => `${m.uid}  ${m.name}  (${m.license?.label || "?"})`).join("\n"),
    download: async (req, a) => { const d = await req("GET", `/models/${a[0]}/download`); const url = (d.glb || d.gltf).url; const r = await http("GET", url, { raw: true }); const { writeFileSync } = await import("node:fs"); const out = `${a[0]}.${d.glb ? "glb" : "zip"}`; writeFileSync(out, Buffer.from(await r.arrayBuffer())); return `Saved ${out}`; },
  },
  help: `Usage:\n  search <words>\n  download <uid>` });
