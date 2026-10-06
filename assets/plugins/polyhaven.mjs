// Poly Haven: free CC0 textures, HDRIs and 3D models (no account).
import { writeFileSync, mkdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const api = (p) => http("GET", "https://api.polyhaven.com" + p, { headers: { "User-Agent": "PixelCode" } });
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
switch (cmd) {
  case "connect": case "status": { const t = await api("/types"); console.log("Poly Haven available: " + t.join(", ")); break; }
  case "search": { const all = await api(`/assets?type=${f.type || "all"}`); const q = w.join(" ").toLowerCase(); print(Object.entries(all).filter(([id, a]) => !q || id.includes(q) || a.name.toLowerCase().includes(q) || (a.tags || []).some((t) => t.includes(q))).slice(0, 30).map(([id, a]) => `${id}  ${a.name}  [${["hdri", "texture", "model"][a.type]}]`).join("\n") || "Nothing found."); break; }
  case "download": {
    const files = await api(`/files/${w[0]}`); const res = f.res || "2k"; const dir = resolve(f.out || "polyhaven/" + w[0]); mkdirSync(dir, { recursive: true });
    const pick = [];
    if (files.hdri) pick.push(files.hdri[res]?.hdr || files.hdri[res]?.exr);
    for (const [map, v] of Object.entries(files)) if (!["hdri", "blend", "gltf", "fbx", "usd"].includes(map) && v[res]) pick.push(v[res].png || v[res].jpg);
    if (files.gltf?.[res]) { pick.push(files.gltf[res].gltf); for (const inc of Object.values(files.gltf[res].gltf.include || {})) pick.push(inc); }
    for (const p of pick.filter(Boolean)) { const name = p.url.split("/").pop(); writeFileSync(join(dir, name), Buffer.from(await (await fetch(p.url)).arrayBuffer())); console.log(join(dir, name)); }
    break;
  }
  default: console.log(`Usage:
  search <words> [--type hdris|textures|models]
  download <asset_id> [--res 1k|2k|4k] [--out dir]`);
}
