// AppImage: package an AppDir into a single portable file (appimagetool, downloaded on connect).
import { existsSync, chmodSync, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const tool = join(process.env.HOME, ".local/share/pixel-code/bin/appimagetool");
const bin = which(["appimagetool"]) || (existsSync(tool) ? tool : null);
const [cmd, ...rest] = args;
switch (cmd) {
  case "connect": if (!bin) { mkdirSync(join(process.env.HOME, ".local/share/pixel-code/bin"), { recursive: true }); writeFileSync(tool, Buffer.from(await (await fetch("https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage")).arrayBuffer())); chmodSync(tool, 0o755); } console.log("appimagetool ready"); break;
  case "status": if (!bin) fail("appimagetool not installed - press Connect"); console.log("appimagetool ready"); break;
  case "build": process.env.ARCH = process.env.ARCH || "x86_64"; passthrough(bin, [rest[0], ...(rest[1] ? [rest[1]] : [])]); break;
  default: console.log("Usage:\n  build <MyApp.AppDir> [MyApp-x86_64.AppImage]   (AppDir needs AppRun, a .desktop file and an icon)");
}
