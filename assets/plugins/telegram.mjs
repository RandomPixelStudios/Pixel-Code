// Telegram: lets the agent message the user. Incoming messages are handled by the Pixel Code app,
// which forwards them to the agent terminals.
import { args, settings, fail } from "./common.mjs";

const token = settings.bot_token;
const chat = settings.chat_id;
const api = (method, body) =>
  fetch(`https://api.telegram.org/bot${token}/${method}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body || {}),
  }).then((r) => r.json());

const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !token) fail("No Telegram bot token. Add it in Pixel Code > Settings > Plugins.");

switch (cmd) {
  case "connect":
  case "status": {
    const me = await api("getMe");
    if (!me.ok) fail(`Telegram: ${me.description}`);
    console.log(chat ? `@${me.result.username} paired with chat ${chat}` : `@${me.result.username} ready - send /start to the bot to pair`);
    break;
  }
  case "send": {
    if (!chat) fail("Not paired yet: the user has to send /start to the bot first.");
    const r = await api("sendMessage", { chat_id: chat, text: rest.join(" ") });
    if (!r.ok) fail(`Telegram: ${r.description}`);
    console.log("Sent.");
    break;
  }
  default:
    console.log(`Usage:
  send <text>    send a message to the user on Telegram (e.g. when a task is done or you need a decision)`);
}
