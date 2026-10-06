// ngrok tunnels: share a local port publicly.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
cliPlugin({ bins: ["ngrok"], name: "ngrok", env: { NGROK_AUTHTOKEN: settings.token }, help: `Usage: any ngrok arguments, e.g.
  'http 3000 --log stdout'      (runs until stopped; better start it in its own terminal)
  'api tunnels list'            (with an API key in NGROK_API_KEY)` });
