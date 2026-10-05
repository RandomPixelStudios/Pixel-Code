// Godot Engine command line.
import { args, settings, fail, which, passthrough } from "./common.mjs";

const bin = settings.path || which(["godot4", "godot", "Godot", "godot-mono", "godot4-mono"]);
const [cmd, ...rest] = args;

if (cmd === "status" || cmd === "connect") {
  if (!bin) fail("Godot not found. Install Godot or set its path in Pixel Code > Settings > Plugins.");
  console.log(`Godot found: ${bin}`);
  process.exit(0);
}
if (!bin) fail("Godot not found. Install Godot or set its path in Pixel Code > Settings > Plugins.");

switch (cmd) {
  case "run": passthrough(bin, ["--path", rest[0] || ".", ...rest.slice(1)]); break;
  case "script": passthrough(bin, ["--headless", "--path", rest[1] || ".", "--script", rest[0]]); break;
  case "export": passthrough(bin, ["--headless", "--path", rest[2] || ".", "--export-release", rest[0], rest[1]]); break;
  case "import": passthrough(bin, ["--headless", "--path", rest[0] || ".", "--import"]); break;
  case undefined:
  case "help":
    console.log(`Usage:
  run [project_dir] [args]                    run the project
  script <file.gd> [project_dir]              run a GDScript headless (extends SceneTree/MainLoop)
  export <preset> <output_file> [project_dir] export a release build
  import [project_dir]                        re-import assets headless
  <any godot arguments>                       passed to the Godot binary, e.g. --version`);
    break;
  default: passthrough(bin, args);
}
