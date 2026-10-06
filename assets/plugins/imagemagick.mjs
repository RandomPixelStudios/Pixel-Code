// ImageMagick - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

cliPlugin({ bins: ["magick", "convert"], name: "ImageMagick", version: ["--version"], prefix: [], env: {}, help: `Usage: magick arguments, e.g. 'in.png -resize 512x512 out.png', 'mogrify -format webp *.png'` });
