// 1Password CLI (service account token or desktop app integration).
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
cliPlugin({ bins: ["op"], name: "1Password CLI", env: { OP_SERVICE_ACCOUNT_TOKEN: settings.token }, help: `Usage: any op arguments, e.g.
  'read op://Dev/Modrinth/token'   'item list --vault Dev'   'run --env-file .env.op -- ./gradlew publish'` });
