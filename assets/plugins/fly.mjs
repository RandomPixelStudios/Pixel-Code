// Fly.io via flyctl (downloaded on connect) and an access token.
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";
import { args, settings, fail, which, passthrough } from "./common.mjs";

const local = join(homedir(), ".fly/bin/flyctl");
const bin = which(["flyctl", "fly"]) || (existsSync(local) ? local : null);
process.env.FLY_API_TOKEN = settings.token || "";
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !settings.token) fail("No Fly.io token set.");

switch (cmd) {
  case "connect": case "status": {
    if (!bin) { if (cmd === "status") fail("flyctl not installed - press Connect"); spawnSync("sh", ["-c", "curl -fsSL https://fly.io/install.sh | sh"], { stdio: "ignore" }); }
    const r = spawnSync(bin || local, ["auth", "whoami"], { encoding: "utf8" });
    if (r.status !== 0) fail((r.stderr || "Fly.io login failed").trim().split("\n").pop());
    console.log(`Connected as ${r.stdout.trim()}`);
    break;
  }
  case undefined: case "help": console.log("Usage: any flyctl command, e.g. apps list | deploy | status -a myapp | logs -a myapp --no-tail | secrets set KEY=value"); break;
  default: if (!bin) fail("flyctl not installed - press Connect."); passthrough(bin, args);
}
