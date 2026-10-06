// Android SDK manager: install platforms, build tools, system images and NDK.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const sdk = settings.sdk || process.env.ANDROID_HOME || process.env.ANDROID_SDK_ROOT || (process.env.HOME + "/Android/Sdk");
const bin = which(["sdkmanager"], [sdk + "/cmdline-tools/latest/bin", sdk + "/tools/bin"]);
if (!bin && args[0] !== "help") { if (args[0] === "status" || args[0] === "connect") fail("sdkmanager not found (install Android cmdline-tools or set the SDK path)."); fail("sdkmanager not found."); }
const [cmd] = args;
if (cmd === "status" || cmd === "connect") { console.log("Android SDK at " + sdk); process.exit(0); }
if (!cmd || cmd === "help") { console.log(`Usage: any sdkmanager arguments, e.g.
  '--list_installed'   '"platforms;android-35" "build-tools;35.0.0"'   '--update'   '--licenses' (accept in a terminal)`); process.exit(0); }
passthrough(bin, ["--sdk_root=" + sdk, ...args]);
