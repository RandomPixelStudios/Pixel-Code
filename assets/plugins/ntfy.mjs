// ntfy.sh push notifications to your phone (no account needed).
import { args, settings, fail, flags } from "./common.mjs";

const server = (settings.server || "https://ntfy.sh").replace(/\/$/, "");
const topic = settings.topic;
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && !topic) fail("Choose a topic name (long and secret, e.g. pixelcode-8f3k2...).");
const push = async (text, title, prio) => {
  const r = await fetch(`${server}/${topic}`, { method: "POST", body: text, headers: { Title: title || "Pixel Code", Priority: prio || "default", ...(settings.token ? { Authorization: `Bearer ${settings.token}` } : {}) } });
  if (!r.ok) fail(`ntfy: HTTP ${r.status}`);
};

switch (cmd) {
  case "connect": await push("Pixel Code is connected.", "Pixel Code"); console.log(`Push sent to ${server}/${topic} - subscribe to this topic in the ntfy app`); break;
  case "status": console.log(`Topic ${server}/${topic}`); break;
  case "send": await push(w.join(" "), f.title, f.priority); console.log("Sent."); break;
  default: console.log(`Usage:
  send <text> [--title "..."] [--priority min|low|default|high|urgent]`);
}
