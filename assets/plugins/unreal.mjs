// Unreal Engine: UnrealEditor-Cmd, UnrealEditor and RunUAT from an engine directory.
import { join, resolve } from "node:path";
import { homedir } from "node:os";
import { args, settings, fail, newest, exists, passthrough } from "./common.mjs";

function engineRoot() {
  if (settings.engine) return settings.engine;
  for (const base of [join(homedir(), "UnrealEngine"), join(homedir(), "Epic Games"), "/opt/UnrealEngine", "/opt/unreal-engine"]) {
    if (exists(join(base, "Engine/Binaries"))) return base;
    const v = newest(base);
    if (v && exists(join(base, v, "Engine/Binaries"))) return join(base, v);
  }
  return null;
}

const root = engineRoot();
const bin = (name) => join(root, "Engine/Binaries/Linux", name);
const [cmd, ...rest] = args;
const missing = "Unreal Engine not found. Set the engine directory (the folder that contains Engine/) in Pixel Code > Settings > Plugins.";

if (cmd === "status" || cmd === "connect") {
  if (!root) fail(missing);
  console.log(`Unreal Engine found: ${root}`);
  process.exit(0);
}
if (!root) fail(missing);

switch (cmd) {
  case "cmd": passthrough(bin("UnrealEditor-Cmd"), [resolve(rest[0]), ...rest.slice(1)]); break;
  case "python": passthrough(bin("UnrealEditor-Cmd"), [resolve(rest[0]), `-ExecutePythonScript=${resolve(rest[1])}`, "-unattended", "-nosplash", "-nullrhi"]); break;
  case "build": passthrough(join(root, "Engine/Build/BatchFiles/RunUAT.sh"), ["BuildCookRun", `-project=${resolve(rest[0])}`, "-platform=" + (rest[1] || "Linux"), "-build", "-cook", "-stage", "-pak", "-archive", `-archivedirectory=${resolve(rest[2] || "Build")}`, "-unattended"]); break;
  case "uat": passthrough(join(root, "Engine/Build/BatchFiles/RunUAT.sh"), rest); break;
  case "open": passthrough(bin("UnrealEditor"), [resolve(rest[0])]); break;
  case undefined:
  case "help":
    console.log(`Usage:
  python <Project.uproject> <script.py>          run a Python script in the editor (headless)
  cmd <Project.uproject> [args]                  UnrealEditor-Cmd, e.g. -run=ResavePackages
  build <Project.uproject> [Linux|Win64] [dir]   BuildCookRun and archive
  uat <args>                                     RunUAT.sh with any arguments
  open <Project.uproject>                        open the editor`);
    break;
  default: passthrough(bin("UnrealEditor-Cmd"), args);
}
