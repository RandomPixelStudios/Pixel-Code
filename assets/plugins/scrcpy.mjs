// scrcpy: mirror and control an Android device on the desktop.
import { spawn } from "node:child_process";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const bin = which(["scrcpy"]);
const [cmd, ...rest] = args;
if (cmd === "status" || cmd === "connect") { if (!bin) fail("scrcpy not installed (sudo apt install scrcpy)."); console.log("scrcpy ready"); process.exit(0); }
if (!bin) fail("scrcpy not installed.");
if (cmd === "record") passthrough(bin, ["--no-playback", "--record", rest[0] || "screen.mp4", "--time-limit", rest[1] || "15"]);
else if (cmd === "start" || !cmd) { spawn(bin, rest, { detached: true, stdio: "ignore" }).unref(); console.log("Mirroring started"); }
else console.log("Usage:\n  start [scrcpy args] | record [file.mp4] [seconds]");
