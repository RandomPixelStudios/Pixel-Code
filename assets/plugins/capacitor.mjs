// Capacitor / Ionic: turn a web app into Android and iOS apps.
import { spawnSync } from "node:child_process";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const [cmd] = args;
if (cmd === "status" || cmd === "connect") { const r = spawnSync("npx", ["-y", "@capacitor/cli", "--version"], { encoding: "utf8" }); if (r.status !== 0) fail("Capacitor CLI not available (needs Node/npm)."); console.log("Capacitor CLI " + r.stdout.trim()); process.exit(0); }
if (!cmd || cmd === "help") { console.log(`Usage: any cap arguments, e.g.
  'sync'   'add android'   'build android'   'run android --target <device>'   'open android'`); process.exit(0); }
process.exit(spawnSync("npx", ["-y", "@capacitor/cli", ...args], { stdio: "inherit" }).status ?? 1);
