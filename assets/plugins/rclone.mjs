// rclone: copy and sync to almost any cloud storage (configured remotes from 'rclone config').
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
cliPlugin({ bins: ["rclone"], name: "rclone", help: `Usage: any rclone arguments, e.g.
  'listremotes'   'ls remote:bucket'   'copy build/ remote:releases/1.2.0 --progress'   'sync ~/Projects b2:backup/projects'
  (set up remotes once in a terminal: rclone config)` });
