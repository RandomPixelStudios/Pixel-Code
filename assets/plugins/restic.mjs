// restic backups (repository + password from the plugin settings).
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
cliPlugin({ bins: ["restic"], name: "restic", env: { RESTIC_REPOSITORY: settings.repository, RESTIC_PASSWORD: settings.password }, help: `Usage: any restic arguments, e.g.
  'snapshots'   'backup ~/Dokumente/Projects --exclude target'   'restore latest --target /tmp/restore'   'check'   'init'` });
