// Slack bot: post messages, read channels, upload files.
import { readFileSync } from "node:fs";
import { basename, resolve } from "node:path";
import { args, settings, fail, http, print } from "./common.mjs";

const headers = { Authorization: `Bearer ${settings.bot_token || ""}` };
const api = async (method, body) => {
  const r = await http("POST", `https://slack.com/api/${method}`, { headers: { ...headers, "Content-Type": "application/json; charset=utf-8" }, body });
  if (!r.ok) fail(`Slack: ${r.error}`);
  return r;
};
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !settings.bot_token) fail("No Slack bot token set.");
const ch = (x) => x || settings.channel;

switch (cmd) {
  case "connect": case "status": { const r = await api("auth.test"); console.log(`Connected as ${r.user} in ${r.team}`); break; }
  case "send": await api("chat.postMessage", { channel: ch(rest[0]?.startsWith("#") || rest[0]?.startsWith("C") ? rest.shift() : null), text: rest.join(" ") }); console.log("Sent."); break;
  case "read": print((await api("conversations.history", { channel: ch(rest[0]), limit: Number(rest[1] || 20) })).messages.reverse().map((m) => `${m.user ?? m.bot_id}: ${m.text}`).join("\n")); break;
  case "channels": print((await api("conversations.list", { limit: 200 })).channels.map((c) => `${c.id} #${c.name}`).join("\n")); break;
  case "upload": {
    const file = resolve(rest[0]);
    const data = readFileSync(file);
    const up = await http("GET", `https://slack.com/api/files.getUploadURLExternal?filename=${encodeURIComponent(basename(file))}&length=${data.length}`, { headers });
    if (!up.ok) fail(`Slack: ${up.error}`);
    await fetch(up.upload_url, { method: "POST", body: data });
    await api("files.completeUploadExternal", { files: [{ id: up.file_id }], channel_id: ch(rest[1]) });
    console.log("Uploaded.");
    break;
  }
  default: console.log(`Usage:
  send [#channel|C123] <text>   post a message (default channel from settings)
  read [channel] [count]        recent messages
  channels                      list channels
  upload <file> [channel]       upload a file`);
}
