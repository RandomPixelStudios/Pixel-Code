// Appwrite CLI (projects, databases, functions, deploys).
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
cliPlugin({ bins: ["appwrite"], name: "Appwrite CLI", help: `Usage: any appwrite arguments, e.g.
  'push functions'   'databases list'   'projects list'   (log in once in a terminal: appwrite login)` });
