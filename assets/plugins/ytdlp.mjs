// yt-dlp - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

cliPlugin({ bins: ["yt-dlp"], name: "yt-dlp", version: ["--version"], prefix: [], env: {}, help: `Usage: yt-dlp arguments, e.g. '<url>', '-x --audio-format mp3 <url>', '-F <url>'` });
