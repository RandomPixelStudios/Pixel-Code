// Steam uploads with steamcmd (needs a Steamworks partner account and app build scripts).
import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import { args, settings, fail, which, passthrough } from "./common.mjs";

const bin = settings.path || which(["steamcmd", "steamcmd.sh"]);
const [cmd, ...rest] = args;
if (cmd === "status" || cmd === "connect") {
  if (!bin) fail("steamcmd not found (sudo apt install steamcmd, or set its path).");
  if (!settings.username) fail("Steam username missing.");
  console.log(`steamcmd ready for ${settings.username}`);
  process.exit(0);
}
if (!bin) fail("steamcmd not found.");
switch (cmd) {
  case "upload": passthrough(bin, ["+login", settings.username, "+run_app_build", resolve(rest[0]), "+quit"]); break;
  case "cmd": passthrough(bin, ["+login", settings.username, ...rest, "+quit"]); break;
  default: console.log(`Usage:
  upload <app_build.vdf>     upload a build (Steam Guard: log in once in a terminal with 'steamcmd +login <user>')
  cmd <steamcmd commands>    e.g. cmd +app_status 480`);
}
