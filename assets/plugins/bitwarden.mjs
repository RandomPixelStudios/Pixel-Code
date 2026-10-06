// Bitwarden CLI: read secrets for the agent (session from 'bw unlock --raw').
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
cliPlugin({ bins: ["bw"], name: "Bitwarden CLI", env: { BW_SESSION: settings.session, BW_CLIENTID: settings.client_id, BW_CLIENTSECRET: settings.client_secret }, help: `Usage: any bw arguments, e.g.
  'get password github.com'   'get item <id>'   'list items --search modrinth'   'generate -ulns --length 24'
  (unlock once in a terminal: bw unlock --raw, and paste the session key into the plugin)` });
