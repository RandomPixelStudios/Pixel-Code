// Docusaurus (runs through npx, nothing to install).
import { spawnSync } from "node:child_process";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const [cmd] = args;
const run = (a, o = { stdio: "inherit" }) => spawnSync("npx", ["-y", "@docusaurus/core@latest", ...a], { env: { ...process.env, GIT_USER: settings.git_user || process.env.GIT_USER }, ...o });
if (cmd === "status" || cmd === "connect") { const r = run(["--version"], { encoding: "utf8" }); if (r.status !== 0) fail("Docusaurus is not available (needs Node.js/npm)."); console.log("Docusaurus " + (r.stdout || "").trim().split("\n").pop()); process.exit(0); }
if (!cmd || cmd === "help") { console.log(`Usage (run inside the docs folder): 'build', 'start', 'serve', 'deploy' (GitHub Pages), 'write-translations'`); process.exit(0); }
process.exit(run(args).status ?? 1);
