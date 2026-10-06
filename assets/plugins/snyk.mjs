// Snyk: find vulnerabilities in dependencies and code.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
cliPlugin({ bins: ["snyk"], name: "Snyk CLI", env: { SNYK_TOKEN: settings.token }, help: `Usage: any snyk arguments, e.g.
  'test'   'test --all-projects'   'code test'   'container test node:20'   'monitor'` });
