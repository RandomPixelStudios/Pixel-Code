// Terraform - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

cliPlugin({ bins: ["terraform", "tofu"], name: "Terraform", version: ["version"], prefix: [], env: {}, help: `Usage: any terraform arguments, e.g. 'init', 'plan', 'apply', 'state list'` });
