// Doppler secrets manager.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
cliPlugin({ bins: ["doppler"], name: "Doppler CLI", env: { DOPPLER_TOKEN: settings.token }, help: `Usage: any doppler arguments, e.g.
  'secrets --project app --config dev'   'secrets get API_KEY --plain'   'run --project app --config prd -- npm start'` });
