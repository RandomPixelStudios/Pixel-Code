// Microsoft Azure - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

cliPlugin({ bins: ["az"], name: "Microsoft Azure", version: ["version"], prefix: [], env: { AZURE_DEFAULTS_SUBSCRIPTION: settings.subscription }, help: `Usage: any az arguments, e.g. 'group list', 'webapp list', 'vm start -n vm -g rg'` });
