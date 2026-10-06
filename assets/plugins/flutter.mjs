// Flutter SDK.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
cliPlugin({ bins: ["flutter"], name: "Flutter", version: ["--version"], help: `Usage: any flutter arguments, e.g.
  'pub get'  'analyze'  'test'  'build apk --release'  'build appbundle'  'build linux'  'devices'  'run -d <device>'  'doctor'` });
