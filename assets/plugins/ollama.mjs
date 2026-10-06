// Local LLMs through Ollama.
import { args, settings, fail, http, print, flags } from "./common.mjs";

const base = (settings.url || "http://127.0.0.1:11434").replace(/\/$/, "");
const [f, rest] = flags(args);
const [cmd, ...a] = rest;

switch (cmd) {
  case "connect": case "status": { const r = await http("GET", `${base}/api/version`); const t = await http("GET", `${base}/api/tags`); console.log(`Ollama ${r.version}, ${t.models?.length || 0} models`); break; }
  case "models": print((await http("GET", `${base}/api/tags`)).models.map((m) => `${m.name}  ${(m.size / 1e9).toFixed(1)} GB`).join("\n")); break;
  case "running": print((await http("GET", `${base}/api/ps`)).models.map((m) => m.name).join("\n") || "No models loaded."); break;
  case "ask": {
    const model = f.model || settings.model;
    if (!model) fail("Pass --model <name> or set a default model.");
    const r = await http("POST", `${base}/api/generate`, { body: { model, prompt: a.join(" "), stream: false, ...(f.system ? { system: f.system } : {}) } });
    print(r.response);
    break;
  }
  case "embed": { const r = await http("POST", `${base}/api/embed`, { body: { model: f.model || settings.model, input: a.join(" ") } }); print(JSON.stringify(r.embeddings?.[0])); break; }
  case "pull": { await http("POST", `${base}/api/pull`, { body: { model: a[0], stream: false } }); console.log(`Pulled ${a[0]}.`); break; }
  default: console.log(`Usage:
  models                          installed models
  running                         loaded models
  ask <prompt> [--model m] [--system text]
  embed <text> [--model m]
  pull <model>`);
}
