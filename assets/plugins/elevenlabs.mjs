// ElevenLabs: text to speech and sound effects, saved as mp3.
import { writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { args, settings, fail, http, print, flags } from "./common.mjs";

const base = "https://api.elevenlabs.io/v1";
const headers = { "xi-api-key": settings.api_key || "" };
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && !settings.api_key) fail("No ElevenLabs API key set.");
const save = async (res, file) => { writeFileSync(resolve(file), Buffer.from(await res.arrayBuffer())); console.log(resolve(file)); };

switch (cmd) {
  case "connect": case "status": { const u = await http("GET", `${base}/user/subscription`, { headers }); console.log(`Connected (${u.tier}, ${u.character_count}/${u.character_limit} characters used)`); break; }
  case "voices": print((await http("GET", `${base}/voices`, { headers })).voices.map((v) => `${v.voice_id}  ${v.name} (${v.labels?.accent ?? ""} ${v.labels?.gender ?? ""})`).join("\n")); break;
  case "speak": await save(await http("POST", `${base}/text-to-speech/${f.voice || "JBFqnCBsd6RMkjVDRZzb"}?output_format=mp3_44100_128`, { headers, raw: true, body: { text: w.join(" "), model_id: f.model || "eleven_multilingual_v2" } }), f.out || "speech.mp3"); break;
  case "sfx": await save(await http("POST", `${base}/sound-generation`, { headers, raw: true, body: { text: w.join(" "), ...(f.seconds ? { duration_seconds: Number(f.seconds) } : {}) } }), f.out || "sfx.mp3"); break;
  default: console.log(`Usage:
  speak <text> [--voice <id>] [--out file.mp3] [--model eleven_multilingual_v2]
  sfx <description> [--seconds 3] [--out file.mp3]     sound effect
  voices`);
}
