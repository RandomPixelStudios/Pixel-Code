// Blockbench models (.bbmodel): inspect, extract textures, convert to Minecraft Java block/item JSON.
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { resolve, join, basename } from "node:path";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const load = (f) => { try { return JSON.parse(readFileSync(resolve(f), "utf8")); } catch (e) { fail("Cannot read model: " + e.message); } };
const [cmd, ...rest] = args;
switch (cmd) {
  case "connect": case "status": console.log("Blockbench model tools ready (works on .bbmodel files, no app needed)"); break;
  case "info": { const m = load(rest[0]); print(`${m.name} (format ${m.meta?.model_format}, ${m.resolution?.width}x${m.resolution?.height})\nelements: ${m.elements?.length ?? 0}\ngroups: ${(m.outliner || []).filter((o) => typeof o === "object").map((o) => o.name).join(", ")}\ntextures: ${(m.textures || []).map((t) => t.name).join(", ")}\nanimations: ${(m.animations || []).map((a) => a.name).join(", ") || "-"}`); break; }
  case "textures": { const m = load(rest[0]); const dir = resolve(rest[1] || "."); mkdirSync(dir, { recursive: true }); for (const t of m.textures || []) { const data = (t.source || "").split(",")[1]; if (!data) continue; const file = join(dir, t.name.endsWith(".png") ? t.name : t.name + ".png"); writeFileSync(file, Buffer.from(data, "base64")); console.log(file); } break; }
  case "java": {
    const m = load(rest[0]);
    const tex = Object.fromEntries((m.textures || []).map((t, i) => [String(i), (rest[2] || "modid:block/") + basename(t.name, ".png")]));
    const out = { credit: "Made with Blockbench", texture_size: [m.resolution?.width || 16, m.resolution?.height || 16], textures: { ...tex, particle: tex["0"] },
      elements: (m.elements || []).filter((e) => e.type !== "mesh").map((e) => ({ name: e.name, from: e.from, to: e.to, ...(e.rotation?.some((r) => r) ? { rotation: { angle: e.rotation.find((r) => r), axis: "xyz"[e.rotation.findIndex((r) => r)], origin: e.origin } } : {}),
        faces: Object.fromEntries(Object.entries(e.faces || {}).filter(([, f]) => f.texture !== null && f.texture !== undefined).map(([k, f]) => [k, { uv: f.uv, texture: "#" + f.texture, ...(f.rotation ? { rotation: f.rotation } : {}) }])) })) };
    writeFileSync(resolve(rest[1] || basename(rest[0], ".bbmodel") + ".json"), JSON.stringify(out, null, 2));
    console.log("Written " + (rest[1] || basename(rest[0], ".bbmodel") + ".json"));
    break;
  }
  default: console.log(`Usage:
  info <model.bbmodel>                          elements, groups, textures, animations
  textures <model.bbmodel> [dir]                extract embedded textures as PNG
  java <model.bbmodel> [out.json] [modid:block/]  convert cubes to a Minecraft Java block/item model`);
}
