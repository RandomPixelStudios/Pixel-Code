// fastlane: automate builds, screenshots, versioning and store uploads.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
cliPlugin({ bins: ["fastlane", "bundle"], name: "fastlane", version: ["--version"], prefix: which(["fastlane"]) ? [] : ["exec", "fastlane"], env: { FASTLANE_SKIP_UPDATE_CHECK: "1", FASTLANE_HIDE_CHANGELOG: "1" }, help: `Usage: any fastlane arguments, e.g.
  'lanes'   'android beta'   'supply --aab app.aab --track internal'   'run increment_version_code'` });
