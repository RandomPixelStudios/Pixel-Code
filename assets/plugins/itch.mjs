// itch.io uploads with butler (downloaded on first connect).
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";
import { args, settings, fail, passthrough } from "./common.mjs";

const dir = join(homedir(), ".local/share/pixel-code/butler");
const bin = join(dir, "butler");
const env = { ...process.env, BUTLER_API_KEY: settings.api_key || "" };
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !settings.api_key) fail("No itch.io API key set.");

function install() {
  mkdirSync(dir, { recursive: true });
  const r = spawnSync("sh", ["-c", `curl -fsSL -o /tmp/butler.zip https://broth.itch.zone/butler/linux-amd64/LATEST/archive/default && unzip -o -q /tmp/butler.zip -d '${dir}' && chmod +x '${bin}'`], { stdio: "ignore" });
  if (r.status !== 0 || !existsSync(bin)) fail("Downloading butler failed (is unzip installed?)");
}

switch (cmd) {
  case "connect": case "status": {
    if (!existsSync(bin)) { if (cmd === "status") fail("butler not installed - press Connect"); install(); }
    const r = spawnSync(bin, ["status", "--json", "x/y"], { encoding: "utf8", env });
    if (/invalid key|401/i.test(r.stdout + r.stderr)) fail("itch.io rejected the API key");
    console.log("butler ready");
    break;
  }
  case "push": process.env.BUTLER_API_KEY = settings.api_key; passthrough(bin, ["push", rest[0], rest[1], ...(rest[2] ? ["--userversion", rest[2]] : [])]); break;
  case "status-of": passthrough(bin, ["status", rest[0]]); break;
  default: console.log(`Usage:
  push <folder-or-zip> <user/game:channel> [version]    upload a build, e.g. push build/linux me/mygame:linux 1.0.2
  status-of <user/game>                                 channels and versions`);
}
