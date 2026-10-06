// Freesound: search and download sound effects (API key).
import { writeFileSync, mkdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = "https://freesound.org/apiv2";
const tok = `token=${settings.api_key || ""}`;
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && !settings.api_key) fail("No Freesound API key set.");
switch (cmd) {
  case "connect": case "status": { await http("GET", `${base}/search/text/?query=test&page_size=1&${tok}`); console.log("Freesound API key works"); break; }
  case "search": { const r = await http("GET", `${base}/search/text/?query=${encodeURIComponent(w.join(" "))}&page_size=20&fields=id,name,duration,license,username&${f.cc0 ? 'filter=license:"Creative Commons 0"&' : ""}${tok}`); print(r.results.map((s) => `${s.id}  ${s.name}  ${s.duration.toFixed(1)}s  ${s.license.split("/").slice(-3, -2)[0]}  by ${s.username}`).join("\n") || "Nothing found."); break; }
  case "download": { const dir = resolve(f.out || "sounds"); mkdirSync(dir, { recursive: true }); for (const id of w) { const s = await http("GET", `${base}/sounds/${id}/?fields=name,previews&${tok}`); const url = s.previews["preview-hq-ogg"] || s.previews["preview-hq-mp3"]; const file = join(dir, `${id}-${s.name.replace(/[^a-z0-9._-]+/gi, "_")}${url.endsWith(".ogg") ? (s.name.endsWith(".ogg") ? "" : ".ogg") : ".mp3"}`); writeFileSync(file, Buffer.from(await (await fetch(url)).arrayBuffer())); console.log(file); } break; }
  default: console.log(`Usage:
  search <words> [--cc0]           find sounds (check the license before shipping them)
  download <id...> [--out dir]     download high-quality previews (ogg/mp3)`);
}
