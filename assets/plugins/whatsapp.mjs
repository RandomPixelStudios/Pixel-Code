// WhatsApp Cloud API (Meta): send messages to your number.
import { args, settings, fail, http } from "./common.mjs";

const base = `https://graph.facebook.com/v21.0/${settings.phone_number_id || ""}`;
const headers = { Authorization: `Bearer ${settings.token || ""}` };
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && (!settings.token || !settings.phone_number_id || !settings.to)) fail("Token, phone number id and your number are required.");

switch (cmd) {
  case "connect": case "status": { const r = await http("GET", base, { headers }); console.log(`Connected (${r.display_phone_number ?? r.verified_name ?? "ok"}) -> ${settings.to}`); break; }
  case "send": {
    await http("POST", `${base}/messages`, { headers, body: { messaging_product: "whatsapp", to: settings.to, type: "text", text: { body: rest.join(" ") } } });
    console.log("Sent.");
    break;
  }
  default: console.log(`Usage:
  send <text>    message your WhatsApp number (the number must have written to the business number in the last 24h)`);
}
