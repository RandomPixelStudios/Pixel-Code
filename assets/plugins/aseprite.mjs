// Aseprite command line: export sprites, spritesheets and layers.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const bin = settings.path || which(["aseprite", "Aseprite"]);
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd === "status" || cmd === "connect") { if (!bin) fail("Aseprite not found (set its path, e.g. ~/.steam/steam/steamapps/common/Aseprite/aseprite)."); console.log("Aseprite found: " + bin); process.exit(0); }
if (!bin) fail("Aseprite not found.");
switch (cmd) {
  case "export": passthrough(bin, ["-b", w[0], ...(f.scale ? ["--scale", f.scale] : []), "--save-as", w[1]]); break;
  case "sheet": passthrough(bin, ["-b", w[0], "--sheet", w[1], "--data", w[1].replace(/\.[^.]+$/, ".json"), "--sheet-type", f.type || "packed", "--format", "json-array", ...(f.tag ? ["--split-tags"] : []), ...(f.scale ? ["--scale", f.scale] : [])]); break;
  case "layers": passthrough(bin, ["-b", "--split-layers", w[0], "--save-as", w[1] || "{layer}.png"]); break;
  case "script": passthrough(bin, ["-b", ...(w[1] ? [w[1]] : []), "--script", w[0]]); break;
  case "info": passthrough(bin, ["-b", "--list-layers", "--list-tags", w[0]]); break;
  default: console.log(`Usage:
  export <in.aseprite> <out.png|gif> [--scale 4]       frames are numbered automatically for animations
  sheet <in.aseprite> <sheet.png> [--type packed|horizontal] [--tag] [--scale 2]   + JSON with frame data
  layers <in.aseprite> [out pattern {layer}.png] | info <file> | script <file.lua> [sprite]`);
}
