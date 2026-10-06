// Android emulator: list, create and start virtual devices.
import { spawn } from "node:child_process";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const sdk = settings.sdk || process.env.ANDROID_HOME || process.env.ANDROID_SDK_ROOT || (process.env.HOME + "/Android/Sdk");
const emu = which(["emulator"], [sdk + "/emulator"]);
const avd = which(["avdmanager"], [sdk + "/cmdline-tools/latest/bin", sdk + "/tools/bin"]);
const [cmd, ...rest] = args;
if (cmd === "status" || cmd === "connect") { if (!emu) fail("Android emulator not found (install it with the SDK manager or set the SDK path)."); console.log("Emulator ready: " + emu); process.exit(0); }
if (!emu) fail("Android emulator not found.");
switch (cmd) {
  case "list": passthrough(emu, ["-list-avds"]); break;
  case "start": spawn(emu, ["-avd", rest[0], ...(rest.includes("--headless") ? ["-no-window", "-no-audio"] : []), ...(rest.includes("--wipe") ? ["-wipe-data"] : [])], { detached: true, stdio: "ignore" }).unref(); console.log("Starting " + rest[0] + " (wait for 'adb devices')"); break;
  case "create": if (!avd) fail("avdmanager not found (install cmdline-tools)."); passthrough(avd, ["create", "avd", "-n", rest[0], "-k", rest[1] || "system-images;android-35;google_apis;x86_64", "-d", rest[2] || "pixel_7", "--force"]); break;
  default: console.log(`Usage:
  list | start <avd> [--headless] [--wipe] | create <name> [system image] [device]`);
}
