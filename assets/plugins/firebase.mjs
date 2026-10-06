// Firebase - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

cliPlugin({ bins: ["firebase"], name: "Firebase", version: ["--version"], prefix: [], env: { FIREBASE_TOKEN: settings.token }, help: `Usage: any firebase arguments, e.g. 'deploy --only hosting', 'projects:list'` });
