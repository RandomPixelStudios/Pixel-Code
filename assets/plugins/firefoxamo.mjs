// web-ext (Firefox add-ons) (runs through npx, nothing to install).
import { spawnSync } from "node:child_process";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const [cmd] = args;
const run = (a, o = { stdio: "inherit" }) => spawnSync("npx", ["-y", "web-ext@latest", ...a], { env: { ...process.env, WEB_EXT_API_KEY: settings.jwt_issuer || '', WEB_EXT_API_SECRET: settings.jwt_secret || '' }, ...o });
if (cmd === "status" || cmd === "connect") { const r = run(["--version"], { encoding: "utf8" }); if (r.status !== 0) fail("web-ext (Firefox add-ons) is not available (needs Node.js/npm)."); console.log("web-ext (Firefox add-ons) " + (r.stdout || "").trim().split("\n").pop()); process.exit(0); }
if (!cmd || cmd === "help") { console.log(`Usage: any web-ext arguments, e.g. 'lint', 'build', 'sign --channel=listed', 'run' (keys from the plugin settings are passed automatically)`); process.exit(0); }
process.exit(run(args).status ?? 1);
