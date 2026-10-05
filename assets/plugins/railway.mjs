// Railway via its CLI (npx) and a project or account token.
import { spawnSync } from "node:child_process";
import { args, settings, fail } from "./common.mjs";

const env = { ...process.env, RAILWAY_API_TOKEN: settings.token || "", ...(settings.project_token ? { RAILWAY_TOKEN: settings.project_token } : {}) };
const cli = (a, opts = { stdio: "inherit" }) => spawnSync("npx", ["-y", "@railway/cli@latest", ...a], { env, ...opts });
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !settings.token && !settings.project_token) fail("A Railway account or project token is required.");

switch (cmd) {
  case "connect": case "status": {
    const r = cli(["whoami"], { encoding: "utf8" });
    if (r.status !== 0) fail((r.stderr || "Railway login failed").trim().split("\n").pop());
    console.log(r.stdout.trim() || "Connected");
    break;
  }
  case undefined: case "help": console.log("Usage: any Railway CLI command, e.g. status | up | logs | variables | redeploy"); break;
  default: process.exit(cli(args).status ?? 1);
}
