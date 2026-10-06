// YouTube - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const base = "https://www.googleapis.com/youtube/v3";
const headers = {};
await restPlugin({ base, headers, need: [settings.api_key], status: async (req) => { await req("GET", `/videos?part=id&chart=mostPopular&maxResults=1&key=${settings.api_key}`); return "Connected"; },
  commands: {
    search: async (req, a) => (await req("GET", `/search?part=snippet&type=video&maxResults=15&q=${encodeURIComponent(a.join(" "))}&key=${settings.api_key}`)).items.map((v) => `${v.id.videoId}  ${v.snippet.title}  (${v.snippet.channelTitle})`).join("\n"),
    video: async (req, a) => (await req("GET", `/videos?part=snippet,statistics&id=${a[0]}&key=${settings.api_key}`)).items[0],
    channel: async (req, a) => (await req("GET", `/channels?part=snippet,statistics&id=${a[0]}&key=${settings.api_key}`)).items[0],
  },
  help: `Usage:\n  search <words>\n  video <id>\n  channel <id>` });
