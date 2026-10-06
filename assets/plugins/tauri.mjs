// Tauri (runs through npx, nothing to install).
import { spawnSync } from "node:child_process";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const [cmd] = args;
const run = (a, o = { stdio: "inherit" }) => spawnSync("npx", ["-y", "@tauri-apps/cli@latest", ...a], { env: { ...process.env,  }, ...o });
if (cmd === "status" || cmd === "connect") { const r = run(["--version"], { encoding: "utf8" }); if (r.status !== 0) fail("Tauri is not available (needs Node.js/npm)."); console.log("Tauri " + (r.stdout || "").trim().split("\n").pop()); process.exit(0); }
if (!cmd || cmd === "help") { console.log(`Usage: any tauri arguments, e.g. 'build', 'build --bundles appimage,deb', 'dev', 'icon app-icon.png', 'android build', 'ios build' (iOS needs a Mac)`); process.exit(0); }
process.exit(run(args).status ?? 1);
