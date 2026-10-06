// Redis - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

cliPlugin({ bins: ["redis-cli"], name: "Redis", version: ["--version"], prefix: settings.url ? ["-u", settings.url] : [], env: {}, help: `Usage: redis-cli arguments, e.g. 'GET key', 'KEYS *', 'INFO memory'` });
