// Expo EAS: cloud builds for iOS/Android, store submission and over-the-air updates.
import { spawnSync } from "node:child_process";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const env = { ...process.env, EXPO_TOKEN: settings.token || process.env.EXPO_TOKEN || "" };
const eas = (a, opts = { stdio: "inherit" }) => spawnSync("npx", ["-y", "eas-cli@latest", ...a], { env, ...opts });
const [cmd] = args;
if (cmd === "status" || cmd === "connect") { const r = eas(["whoami"], { encoding: "utf8" }); if (r.status !== 0) fail("Expo login failed - check the access token."); console.log("Logged in as " + r.stdout.trim().split("\n").pop()); process.exit(0); }
if (!cmd || cmd === "help") { console.log(`Usage: any eas arguments, e.g.
  'build --platform ios --profile preview --non-interactive'   'build --platform android --profile production --non-interactive'
  'submit --platform ios --latest --non-interactive'   'update --branch production --message "Fix crash" --non-interactive'
  'build:list --limit 5'   'device:create'`); process.exit(0); }
process.exit(eas(args).status ?? 1);
