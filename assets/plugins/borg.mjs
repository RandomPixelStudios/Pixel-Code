// BorgBackup (repository + passphrase from the plugin settings).
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
cliPlugin({ bins: ["borg"], name: "BorgBackup", env: { BORG_REPO: settings.repository, BORG_PASSPHRASE: settings.passphrase }, help: `Usage: any borg arguments, e.g.
  'list'   'create ::projects-{now} ~/Dokumente/Projects --exclude "*/target"'   'extract ::archive path'   'prune --keep-daily 7'` });
