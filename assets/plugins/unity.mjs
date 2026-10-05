// Unity Editor in batch mode (Unity Hub installs under ~/Unity/Hub/Editor/<version>/Editor/Unity).
import { join, resolve } from "node:path";
import { homedir } from "node:os";
import { args, settings, fail, which, newest, exists, passthrough } from "./common.mjs";

function find() {
  if (settings.path) return settings.path;
  for (const hub of [join(homedir(), "Unity/Hub/Editor"), "/opt/unity/Editor", join(homedir(), ".local/share/UnityHub/Editor")]) {
    const v = newest(hub);
    const p = v && join(hub, v, "Editor/Unity");
    if (p && exists(p)) return p;
  }
  return which(["Unity", "unity-editor"]);
}

const bin = find();
const [cmd, ...rest] = args;
const missing = "Unity Editor not found. Install it with Unity Hub or set the editor path in Pixel Code > Settings > Plugins.";

if (cmd === "status" || cmd === "connect") {
  if (!bin) fail(missing);
  console.log(`Unity Editor found: ${bin}`);
  process.exit(0);
}
if (!bin) fail(missing);

const batch = (project) => ["-batchmode", "-nographics", "-quit", "-projectPath", resolve(project || "."), "-logFile", "-"];
switch (cmd) {
  case "method": passthrough(bin, [...batch(rest[1]), "-executeMethod", rest[0]]); break;
  case "build": passthrough(bin, [...batch(rest[2]), `-build${rest[0] || "Linux64"}Player`, resolve(rest[1] || "Build/game")]); break;
  case "test": passthrough(bin, ["-batchmode", "-nographics", "-projectPath", resolve(rest[1] || "."), "-runTests", "-testPlatform", rest[0] || "EditMode", "-logFile", "-"]); break;
  case "open": passthrough(bin, ["-projectPath", resolve(rest[0] || ".")]); break;
  case undefined:
  case "help":
    console.log(`Usage:
  method <Class.StaticMethod> [project_dir]          run an editor method in batch mode
  build <Linux64|Windows64|OSX> <output> [project]   build a player
  test [EditMode|PlayMode] [project_dir]             run tests
  open [project_dir]                                 open the editor
  <any Unity arguments>                              passed to the editor`);
    break;
  default: passthrough(bin, args);
}
