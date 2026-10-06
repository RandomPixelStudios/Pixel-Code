// Snapcraft: build snaps and publish them to the Snap Store.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
cliPlugin({ bins: ["snapcraft"], name: "Snapcraft", env: { SNAPCRAFT_STORE_CREDENTIALS: settings.credentials }, help: `Usage: any snapcraft arguments, e.g.
  'pack'   'upload --release=edge my-app_1.0_amd64.snap'   'status my-app'   'list-revisions my-app'
  (credentials: snapcraft export-login --snaps my-app - and paste the output into the plugin)` });
