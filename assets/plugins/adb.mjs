// Android Debug Bridge: install, start and debug apps on phones and emulators.
import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const sdk = settings.sdk || process.env.ANDROID_HOME || process.env.ANDROID_SDK_ROOT || (process.env.HOME + "/Android/Sdk");
const bin = which(["adb"], [sdk + "/platform-tools"]);
const [cmd, ...rest] = args;
const dev = settings.device ? ["-s", settings.device] : [];
if (cmd === "status" || cmd === "connect") { if (!bin) fail("adb not found (install the Android SDK platform-tools or set the SDK path)."); const r = spawnSync(bin, ["devices"], { encoding: "utf8" }); const n = r.stdout.split("\n").slice(1).filter((l) => /\tdevice$/.test(l)).length; console.log(`adb ready, ${n} device(s) connected`); process.exit(0); }
if (!bin) fail("adb not found.");
switch (cmd) {
  case "devices": passthrough(bin, ["devices", "-l"]); break;
  case "install": passthrough(bin, [...dev, "install", "-r", "-d", resolve(rest[0])]); break;
  case "start": passthrough(bin, [...dev, "shell", "monkey", "-p", rest[0], "-c", "android.intent.category.LAUNCHER", "1"]); break;
  case "stop": passthrough(bin, [...dev, "shell", "am", "force-stop", rest[0]]); break;
  case "logcat": { const pid = rest[0] ? spawnSync(bin, [...dev, "shell", "pidof", rest[0]], { encoding: "utf8" }).stdout.trim() : ""; passthrough(bin, [...dev, "logcat", "-d", "-t", rest[1] || "300", ...(pid ? ["--pid", pid] : []), ...(rest[0] && !pid ? ["*:E"] : [])]); break; }
  case "screenshot": { const out = resolve(rest[0] || "device.png"); const r = spawnSync(bin, [...dev, "exec-out", "screencap", "-p"], { maxBuffer: 64e6 }); (await import("node:fs")).writeFileSync(out, r.stdout); console.log(out); break; }
  case "record": passthrough(bin, [...dev, "shell", "screenrecord", "--time-limit", rest[0] || "10", "/sdcard/pixelcode.mp4"]); break;
  case "tap": passthrough(bin, [...dev, "shell", "input", "tap", rest[0], rest[1]]); break;
  case "text": passthrough(bin, [...dev, "shell", "input", "text", rest.join(" ").replace(/ /g, "%s")]); break;
  case "shell": passthrough(bin, [...dev, "shell", ...rest]); break;
  case "push": case "pull": passthrough(bin, [...dev, cmd, rest[0], rest[1]]); break;
  default: console.log(`Usage:
  devices | install <app.apk> | start <package> | stop <package> | logcat [package] [lines]
  screenshot [file.png] | record [seconds] | tap <x> <y> | text <text> | shell <cmd> | push/pull <from> <to>`);
}
