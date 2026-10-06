// Cloudflare Tunnel (cloudflared): quick tunnels and named tunnels.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
cliPlugin({ bins: ["cloudflared"], name: "cloudflared", env: { TUNNEL_TOKEN: settings.token }, help: `Usage: any cloudflared arguments, e.g.
  'tunnel --url http://localhost:3000'      quick public URL (no account)
  'tunnel run'                              run the named tunnel from the token
  'tunnel list'` });
