// electron-builder (runs through npx, nothing to install).
import { spawnSync } from "node:child_process";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const [cmd] = args;
const run = (a, o = { stdio: "inherit" }) => spawnSync("npx", ["-y", "electron-builder@latest", ...a], { env: { ...process.env, GH_TOKEN: settings.github_token || process.env.GH_TOKEN }, ...o });
if (cmd === "status" || cmd === "connect") { const r = run(["--version"], { encoding: "utf8" }); if (r.status !== 0) fail("electron-builder is not available (needs Node.js/npm)."); console.log("electron-builder " + (r.stdout || "").trim().split("\n").pop()); process.exit(0); }
if (!cmd || cmd === "help") { console.log(`Usage: any electron-builder arguments, e.g. '--linux AppImage deb', '--win nsis', '--publish always'`); process.exit(0); }
process.exit(run(args).status ?? 1);
