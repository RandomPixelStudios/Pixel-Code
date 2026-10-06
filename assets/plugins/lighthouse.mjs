// Lighthouse (runs through npx, nothing to install).
import { spawnSync } from "node:child_process";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const [cmd] = args;
const run = (a, o = { stdio: "inherit" }) => spawnSync("npx", ["-y", "lighthouse@latest", ...a], { env: { ...process.env,  }, ...o });
if (cmd === "status" || cmd === "connect") { const r = run(["--version"], { encoding: "utf8" }); if (r.status !== 0) fail("Lighthouse is not available (needs Node.js/npm)."); console.log("Lighthouse " + (r.stdout || "").trim().split("\n").pop()); process.exit(0); }
if (!cmd || cmd === "help") { console.log(`Usage: any lighthouse arguments, e.g. 'https://example.com --only-categories=performance,seo,accessibility --output=json --output-path=report.json --chrome-flags="--headless" --quiet'`); process.exit(0); }
process.exit(run(args).status ?? 1);
