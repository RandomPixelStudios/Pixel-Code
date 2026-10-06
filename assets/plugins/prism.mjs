// Prism Launcher: list instances, copy a freshly built mod in and launch the game.
import { readdirSync, copyFileSync, existsSync, mkdirSync, readFileSync } from "node:fs";
import { join, basename, resolve } from "node:path";
import { homedir } from "node:os";
import { spawn } from "node:child_process";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const data = settings.data_dir || [join(homedir(), ".local/share/PrismLauncher"), join(homedir(), ".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher")].find(existsSync);
const bin = settings.path || which(["prismlauncher", "PrismLauncher"]) || (existsSync("/var/lib/flatpak/exports/bin/org.prismlauncher.PrismLauncher") ? "/var/lib/flatpak/exports/bin/org.prismlauncher.PrismLauncher" : null);
const inst = (n) => join(data, "instances", n);
const mods = (n) => [join(inst(n), "minecraft/mods"), join(inst(n), ".minecraft/mods")].find(existsSync) || join(inst(n), "minecraft/mods");
const [cmd, ...rest] = args;
switch (cmd) {
  case "connect": case "status": if (!bin || !data) fail("Prism Launcher not found (install it or set its path)."); console.log(`Prism Launcher found, ${readdirSync(join(data, "instances")).filter((x) => !x.startsWith(".") && x !== "instgroups.json").length} instances`); break;
  case "instances": print(readdirSync(join(data, "instances")).filter((x) => existsSync(join(inst(x), "instance.cfg"))).map((x) => { const name = (readFileSync(join(inst(x), "instance.cfg"), "utf8").match(/^name=(.*)$/m) || [])[1]; return `${x}  ${name ?? ""}`; }).join("\n")); break;
  case "install": { const d = mods(rest[0]); mkdirSync(d, { recursive: true }); for (const f of rest.slice(1)) { copyFileSync(resolve(f), join(d, basename(f))); console.log("Copied " + basename(f)); } break; }
  case "remove": { const { rmSync } = await import("node:fs"); for (const f of readdirSync(mods(rest[0])).filter((x) => x.includes(rest[1]))) { rmSync(join(mods(rest[0]), f)); console.log("Removed " + f); } break; }
  case "launch": spawn(bin, ["--launch", rest[0], ...(rest[1] ? ["--server", rest[1]] : [])], { detached: true, stdio: "ignore" }).unref(); console.log("Launching " + rest[0]); break;
  case "log": { const f = [join(inst(rest[0]), "minecraft/logs/latest.log"), join(inst(rest[0]), ".minecraft/logs/latest.log")].find(existsSync); if (!f) fail("No log yet."); console.log(readFileSync(f, "utf8").split("\n").slice(-Number(rest[1] || 80)).join("\n")); break; }
  default: console.log(`Usage:
  instances                              list instances (folder names)
  install <instance> <mod.jar...>        copy mod jars into the instance
  remove <instance> <name-part>          remove matching jars
  launch <instance> [server:port]        start Minecraft
  log <instance> [lines]                 end of the game log (crashes, mixin errors)`);
}
