// SoX: convert, trim, normalize and mix audio.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
cliPlugin({ bins: ["sox"], name: "SoX", version: ["--version"], help: `Usage: any sox arguments, e.g.
  'in.wav out.ogg'                         convert
  'in.wav out.wav trim 0 1.5 norm -1'      cut to 1.5 s and normalize
  'in.wav out.wav pitch 300 reverb'        effects
  'in.wav -n stat'                         statistics` });
