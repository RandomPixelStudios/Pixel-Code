// Tiled map editor: export maps and tilesets.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
cliPlugin({ bins: ["tiled", "org.mapeditor.Tiled"], name: "Tiled", version: ["--version"], help: `Usage:
  --export-map <map.tmx> <out.json|.lua|.tmj>     export a map (format from extension)
  --export-tileset <set.tsx> <out.json>           export a tileset
  --export-formats                                list available formats` });
