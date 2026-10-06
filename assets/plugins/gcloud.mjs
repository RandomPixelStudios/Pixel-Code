// Google Cloud - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

cliPlugin({ bins: ["gcloud"], name: "Google Cloud", version: ["--version"], prefix: [], env: { CLOUDSDK_CORE_PROJECT: settings.project }, help: `Usage: any gcloud arguments, e.g. 'run services list', 'compute instances list', 'logging read'` });
