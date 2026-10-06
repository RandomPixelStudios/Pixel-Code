// OpenAI - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

import { writeFileSync } from "node:fs";
const [f, rest] = flags(args);
const H = { Authorization: `Bearer ${settings.api_key || ""}` };
const [cmd, ...a] = rest;
if (cmd && cmd !== "help" && !settings.api_key) fail("No OpenAI API key set.");
if (cmd === "status" || cmd === "connect") { const m = await http("GET", "https://api.openai.com/v1/models", { headers: H }); console.log(`Connected, ${m.data.length} models`); }
else if (cmd === "image") { const r = await http("POST", "https://api.openai.com/v1/images/generations", { headers: H, body: { model: f.model || "gpt-image-1", prompt: a.join(" "), size: f.size || "1024x1024" } }); const out = f.out || `image-${Date.now()}.png`; writeFileSync(out, Buffer.from(r.data[0].b64_json, "base64")); console.log(`Saved ${out}`); }
else if (cmd === "speak") { const r = await http("POST", "https://api.openai.com/v1/audio/speech", { headers: H, body: { model: "gpt-4o-mini-tts", voice: f.voice || "alloy", input: a.join(" ") }, raw: true }); const out = f.out || `speech-${Date.now()}.mp3`; writeFileSync(out, Buffer.from(await r.arrayBuffer())); console.log(`Saved ${out}`); }
else if (cmd === "ask") { const r = await http("POST", "https://api.openai.com/v1/chat/completions", { headers: H, body: { model: f.model || settings.model || "gpt-5-mini", messages: [{ role: "user", content: a.join(" ") }] } }); print(r.choices[0].message.content); }
else console.log("Usage:\n  image <prompt> [--size 1024x1024] [--out file]\n  speak <text> [--voice alloy] [--out file]\n  ask <prompt> [--model m]");
