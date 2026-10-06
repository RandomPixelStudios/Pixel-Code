// MongoDB - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

cliPlugin({ bins: ["mongosh"], name: "MongoDB", version: ["--version"], prefix: settings.url ? [settings.url, "--quiet"] : ["--quiet"], env: {}, help: `Usage: mongosh arguments, e.g. '--eval "db.getCollectionNames()"'` });
