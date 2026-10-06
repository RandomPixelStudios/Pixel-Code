// Mintlify (runs through npx, nothing to install).
import { spawnSync } from "node:child_process";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const [cmd] = args;
const run = (a, o = { stdio: "inherit" }) => spawnSync("npx", ["-y", "mint@latest", ...a], { env: { ...process.env,  }, ...o });
if (cmd === "status" || cmd === "connect") { const r = run(["--version"], { encoding: "utf8" }); if (r.status !== 0) fail("Mintlify is not available (needs Node.js/npm)."); console.log("Mintlify " + (r.stdout || "").trim().split("\n").pop()); process.exit(0); }
if (!cmd || cmd === "help") { console.log(`Usage: any mint arguments, e.g. 'dev', 'broken-links', 'openapi-check openapi.json' (deploys run from the GitHub repo)`); process.exit(0); }
process.exit(run(args).status ?? 1);
