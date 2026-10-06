// Maestro: UI tests for Android and iOS in simple YAML flows.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
process.env.PATH += ":" + process.env.HOME + "/.maestro/bin";
cliPlugin({ bins: ["maestro"], name: "Maestro", version: ["--version"], help: `Usage: any maestro arguments, e.g.
  'test .maestro/'   'test flow.yaml --format junit'   'studio'   'record flow.yaml'
  (install: curl -fsSL https://get.maestro.mobile.dev | bash)` });
