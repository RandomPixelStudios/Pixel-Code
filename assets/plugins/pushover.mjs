// Pushover - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const [f, rest] = flags(args);
const [cmd, ...a] = rest;
if (cmd === "status" || cmd === "connect") { await http("POST", "https://api.pushover.net/1/users/validate.json", { form: { token: settings.token || "", user: settings.user || "" } }); console.log("Connected"); }
else if (cmd === "send") { await http("POST", "https://api.pushover.net/1/messages.json", { form: { token: settings.token, user: settings.user, message: a.join(" "), title: f.title || "Pixel Code", priority: f.priority || "0" } }); console.log("Sent."); }
else console.log("Usage: send <text> [--title ..] [--priority -1|0|1]");
