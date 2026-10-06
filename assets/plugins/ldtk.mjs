// LDtk level files: list levels, layers and entities (reads the .ldtk JSON).
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const [cmd, file, ...rest] = args;
const load = () => { try { return JSON.parse(readFileSync(resolve(file), "utf8")); } catch (e) { fail("Cannot read .ldtk: " + e.message); } };
switch (cmd) {
  case "connect": case "status": console.log("LDtk tools ready (works on .ldtk files)"); break;
  case "levels": print(load().levels.map((l) => `${l.identifier}  ${l.pxWid}x${l.pxHei} at (${l.worldX},${l.worldY})`).join("\n")); break;
  case "entities": { const p = load(); for (const l of p.levels) for (const li of l.layerInstances || []) for (const e of li.entityInstances || []) if (!rest[0] || l.identifier === rest[0]) console.log(`${l.identifier}  ${e.__identifier} at ${e.px.join(",")}  ${e.fieldInstances.map((f) => f.__identifier + "=" + JSON.stringify(f.__value)).join(" ")}`); break; }
  case "defs": { const d = load().defs; print({ layers: d.layers.map((l) => `${l.identifier} (${l.__type})`), entities: d.entities.map((e) => e.identifier), tilesets: d.tilesets.map((t) => t.relPath) }); break; }
  default: console.log(`Usage:
  levels <file.ldtk> | entities <file.ldtk> [level] | defs <file.ldtk>`);
}
