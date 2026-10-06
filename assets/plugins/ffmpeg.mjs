// FFmpeg - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

cliPlugin({ bins: ["ffmpeg"], name: "FFmpeg", version: ["-version"], prefix: [], env: {}, help: `Usage: any ffmpeg arguments, e.g. '-i in.mov -vf scale=1280:-2 out.mp4', '-i clip.mp4 -ss 5 -frames:v 1 thumb.png'` });
