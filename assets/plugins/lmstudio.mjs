// LM Studio - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const [f, rest] = flags(args);
const base = (settings.url || "http://127.0.0.1:1234").replace(/\/$/, "") + "/v1";
const [cmd, ...a] = rest;
const models = async () => (await http("GET", `${base}/models`)).data.map((m) => m.id);
if (cmd === "status" || cmd === "connect") console.log(`LM Studio, ${(await models()).length} models`);
else if (cmd === "models") print((await models()).join("\n"));
else if (cmd === "ask") { const model = f.model || (await models())[0]; const r = await http("POST", `${base}/chat/completions`, { body: { model, messages: [{ role: "user", content: a.join(" ") }] } }); print(r.choices[0].message.content); }
else console.log("Usage:\n  models\n  ask <prompt> [--model id]");
